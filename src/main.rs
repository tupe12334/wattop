mod flow;
mod smc;
mod telemetry;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::Marker,
    text::{Line, Span},
    widgets::{Axis, Block, BorderType, Cell, Chart, Dataset, GraphType, Paragraph, Row, Table},
};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use sysinfo::{ProcessesToUpdate, System};
use telemetry::Sample;

const HISTORY: usize = 300; // 5 minutes at 1 Hz
const TICK: Duration = Duration::from_secs(1); // data refresh
const FRAME: Duration = Duration::from_millis(200); // redraw, drives flow animation

struct App {
    history: VecDeque<Sample>,
    last: Result<Sample, String>,
    sys: System,
    /// None when the SMC cannot be opened; ioreg values are used then.
    smc: Option<smc::Smc>,
}

impl App {
    fn tick(&mut self) {
        let mut read = telemetry::read();
        if let (Ok(s), Some(smc)) = (&mut read, &self.smc) {
            telemetry::apply_live_power(s, smc.read_f32("PDTR").ok(), smc.read_f32("PSTR").ok());
        }
        self.last = read.map_err(|e| format!("{e:#}"));
        if let Ok(s) = &self.last {
            if self.history.len() == HISTORY {
                self.history.pop_front();
            }
            self.history.push_back(s.clone());
        }
        self.sys.refresh_processes(ProcessesToUpdate::All, true);
    }
}

fn main() -> Result<()> {
    let mut app = App {
        history: VecDeque::new(),
        last: Err("loading".into()),
        sys: System::new()?,
        smc: smc::Smc::open().ok(),
    };
    app.tick();
    let mut terminal = ratatui::init();
    let mut next = Instant::now() + TICK;
    let mut frame: u64 = 0;
    let res = loop {
        if Instant::now() >= next {
            app.tick();
            next = Instant::now() + TICK;
        }
        if let Err(e) = terminal.draw(|f| draw(f, &app, frame)) {
            break Err(e.into());
        }
        frame = frame.wrapping_add(1);
        if event::poll(FRAME)?
            && let Event::Key(k) = event::read()?
            && matches!(k.code, KeyCode::Char('q') | KeyCode::Esc)
        {
            break Ok(());
        }
    };
    ratatui::restore();
    res
}

fn panel(title: &str) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(Span::styled(format!(" {title} "), Style::new().fg(Color::Cyan).bold()))
}

/// Row heights for flow, status, chart, procs. Flow shrinks first (to 9 rows),
/// then the chart keeps at least 6 rows. Procs (0 rows) hide when there is no room.
fn heights(total: u16) -> [u16; 4] {
    const STATUS: u16 = 9;
    const CHART_MIN: u16 = 6;
    const PROCS_MIN: u16 = 5;
    let flow = (total * 45 / 100).clamp(9, 12).min(total.saturating_sub(STATUS + CHART_MIN));
    let below = total.saturating_sub(flow + STATUS);
    let procs = if below >= CHART_MIN + PROCS_MIN { (below * 2 / 5).max(PROCS_MIN) } else { 0 };
    [flow, STATUS, below - procs, procs]
}

fn draw(f: &mut Frame, app: &App, frame: u64) {
    let [flow_h, status_h, chart_h, procs_h] = heights(f.area().height);
    let [top, status, chart, procs] = Layout::vertical([
        Constraint::Length(flow_h),
        Constraint::Length(status_h),
        Constraint::Length(chart_h),
        Constraint::Length(procs_h),
    ])
    .areas(f.area());

    match &app.last {
        Ok(s) => {
            flow::draw(f, top, s, frame);
            draw_status(f, status, s);
        }
        Err(e) => f.render_widget(Paragraph::new(e.as_str()).red().block(panel("wattop")), top),
    }
    draw_chart(f, chart, &app.history);
    if procs_h > 0 {
        draw_procs(f, procs, &app.sys);
    }
}

fn level_color(percent: i64) -> Color {
    match percent {
        ..=15 => Color::Red,
        16..=40 => Color::Yellow,
        _ => Color::Green,
    }
}

fn draw_status(f: &mut Frame, area: Rect, s: &Sample) {
    let [cell, readouts] = Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)]).areas(area);

    let percent = s.percent.clamp(0, 100);
    let color = level_color(percent);
    let filled = (percent as usize).div_ceil(10);
    let bar: String = (0..10).map(|i| if i < filled { '▰' } else { '▱' }).collect();
    let (word, word_color) = if s.plugged_in && s.battery_w.abs() <= flow::EPS_W {
        ("HOLDING", Color::Cyan)
    } else if s.battery_w > flow::EPS_W {
        ("CHARGING", Color::Green)
    } else {
        ("DISCHARGING", Color::Red)
    };
    let cell_lines = vec![
        Line::from(Span::styled(bar, Style::new().fg(color))),
        Line::from(Span::styled(format!("{percent}%"), Style::new().fg(color).bold())),
        Line::from(Span::styled(word, Style::new().fg(word_color).bold())),
    ];
    f.render_widget(Paragraph::new(cell_lines).block(panel("battery")), cell);

    let net_color = if s.battery_w >= 0.0 { Color::Green } else { Color::Red };
    let eta = s.minutes_left.map_or("—".into(), |m| format!("{}h {:02}m", m / 60, m % 60));
    let rating = if s.plugged_in { format!("{} W", s.adapter_watts) } else { "—".into() };
    let kv = |k: &str, v: String, c: Color| {
        Line::from(vec![
            Span::styled(format!("{k:<15}"), Style::new().fg(Color::DarkGray)),
            Span::styled(v, Style::new().fg(c).bold()),
        ])
    };
    let source = if !s.plugged_in {
        "—".to_string()
    } else {
        let port = if s.power_ports.is_empty() { "no port".to_string() } else { s.power_ports.join(" + ") };
        format!("{port} · {}", s.charger_name.as_deref().unwrap_or("unknown adapter"))
    };
    let lines = vec![
        kv("Input", format!("{:.1} W", s.input_w), Color::Green),
        kv("Load", format!("{:.1} W", s.load_w), Color::Yellow),
        kv("Battery net", flow::signed_w(s.battery_w), net_color),
        kv("Source", source, Color::White),
        kv("Charger rating", rating, Color::White),
        kv("Temperature", format!("{:.1} °C", s.temp_c), Color::White),
        kv("ETA", eta, Color::White),
    ];

    let verdict = if s.plugged_in && s.input_w < s.load_w {
        Span::styled(
            format!(" Charger too weak: −{:.1} W from battery ", s.load_w - s.input_w),
            Style::new().fg(Color::Red).bold(),
        )
    } else if s.battery_w > flow::EPS_W {
        Span::styled(format!(" Charging at {:.1} W ", s.battery_w), Style::new().fg(Color::Green).bold())
    } else if !s.plugged_in {
        Span::styled(" On battery ", Style::new().fg(Color::Yellow).bold())
    } else {
        Span::raw("")
    };
    f.render_widget(Paragraph::new(lines).block(panel("power").title_bottom(verdict)), readouts);
}

fn draw_chart(f: &mut Frame, area: Rect, h: &VecDeque<Sample>) {
    // Right-align: the newest sample sits at the right edge.
    let x0 = HISTORY.saturating_sub(h.len());
    let series = |pick: fn(&Sample) -> f64| -> Vec<(f64, f64)> {
        h.iter().enumerate().map(|(i, s)| ((x0 + i) as f64, pick(s))).collect()
    };
    let input = series(|s| s.input_w);
    let load = series(|s| s.load_w);
    let batt = series(|s| s.battery_w);
    let zero = vec![(0.0, 0.0), ((HISTORY - 1) as f64, 0.0)];

    let all = input.iter().chain(&load).chain(&batt).map(|p| p.1);
    let (lo, hi) = all.fold((0.0f64, 1.0f64), |(lo, hi), v| (lo.min(v), hi.max(v)));
    let (lo, hi) = ((lo - 1.0).floor(), (hi + 1.0).ceil());

    let mut datasets = Vec::new();
    if lo < 0.0 {
        datasets.push(Dataset::default().name("0 W").marker(Marker::Braille).graph_type(GraphType::Line).fg(Color::DarkGray).data(&zero));
    }
    datasets.extend([
        Dataset::default().name("in").marker(Marker::Braille).graph_type(GraphType::Line).fg(Color::Green).data(&input),
        Dataset::default().name("load").marker(Marker::Braille).graph_type(GraphType::Line).fg(Color::Yellow).data(&load),
        Dataset::default().name("battery").marker(Marker::Braille).graph_type(GraphType::Line).fg(Color::Magenta).data(&batt),
    ]);

    let label = |v: f64| Span::styled(format!("{v:.0}W"), Style::new().fg(Color::DarkGray));
    let span_s = h.len().max(2) - 1;
    f.render_widget(
        Chart::new(datasets)
            .block(panel(&format!("watts · last {span_s}s · q to quit")))
            .x_axis(Axis::default().bounds([0.0, (HISTORY - 1) as f64]))
            .y_axis(Axis::default().bounds([lo, hi]).labels(vec![label(lo), label((lo + hi) / 2.0), label(hi)])),
        area,
    );
}

fn draw_procs(f: &mut Frame, area: Rect, sys: &System) {
    let mut procs: Vec<_> = sys.processes().values().collect();
    procs.sort_by(|a, b| b.cpu_usage().total_cmp(&a.cpu_usage()));
    let rows = procs.iter().take(area.height.saturating_sub(3).min(5) as usize).map(|p| {
        let cpu = p.cpu_usage();
        let c = if cpu > 50.0 { Color::Red } else if cpu > 15.0 { Color::Yellow } else { Color::White };
        Row::new(vec![
            Cell::from(p.pid().to_string()).fg(Color::DarkGray),
            Cell::from(p.name().to_string_lossy().into_owned()),
            Cell::from(format!("{cpu:6.1}%")).fg(c),
            Cell::from(format!("{} MB", p.memory() / 1_048_576)).fg(Color::DarkGray),
        ])
    });
    f.render_widget(
        Table::new(rows, [Constraint::Length(8), Constraint::Min(20), Constraint::Length(9), Constraint::Length(10)])
            .header(Row::new(["pid", "process", "cpu", "mem"]).fg(Color::Cyan).bold())
            .block(panel("top power users (cpu)")),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::heights;

    #[test]
    fn chart_keeps_six_rows_on_small_terminals() {
        let [flow, status, chart, procs] = heights(28);
        assert_eq!((flow, status, chart, procs), (12, 9, 7, 0));
        assert!(heights(24)[2] >= 6);
        assert_eq!(heights(60).iter().sum::<u16>(), 60);
    }
}
