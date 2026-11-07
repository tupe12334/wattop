//! Energy flow panel: charger, system and battery nodes joined by animated lines.

use crate::{panel, telemetry::Sample};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph},
};

/// Battery watts within this band count as holding.
pub const EPS_W: f64 = 0.05;
const ORANGE: Color = Color::Rgb(255, 140, 0);
const NODE_H: i32 = 5;
const MID: i32 = NODE_H / 2; // row where lines meet a node

/// Which energy lines carry power right now.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flows {
    pub charger_to_system: bool,
    pub charger_to_battery: bool,
    pub battery_to_system: bool,
}

/// Decides which lines are live. Each line has one fixed direction.
pub fn flows(s: &Sample) -> Flows {
    Flows {
        charger_to_system: s.plugged_in,
        charger_to_battery: s.battery_w > EPS_W,
        battery_to_system: s.battery_w < -EPS_W,
    }
}

/// Signed watts, with a real minus sign.
pub fn signed_w(v: f64) -> String {
    format!("{v:+.1} W").replace('-', "−")
}

pub fn draw(f: &mut Frame, area: Rect, s: &Sample, frame: u64) {
    let block = panel("energy flow");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let live = flows(s);
    let (w, h) = (inner.width as i32, inner.height as i32);
    let nw = (w / 3).clamp(14, 28);
    let sx = w - nw; // system node left edge
    let bx = (w - nw) / 2; // battery node left edge
    let by = h - NODE_H; // battery node top edge
    let cx = nw / 2; // charger node centre
    let sxc = sx + nw / 2; // system node centre
    let bottom = by + MID; // row where battery lines run

    let top_line = [(nw, MID), (sx - 1, MID)];
    let charge_line = [(cx, NODE_H), (cx, bottom), (bx - 1, bottom)];
    let drain_line = [(bx + nw, bottom), (sxc, bottom), (sxc, NODE_H)];

    let buf = f.buffer_mut();
    let on = |on: bool, c: Color| on.then_some(c);
    route(buf, inner, &top_line, on(live.charger_to_system, Color::Green), frame);
    route(buf, inner, &charge_line, on(live.charger_to_battery, Color::Green), frame);
    route(buf, inner, &drain_line, on(live.battery_to_system, ORANGE), frame);

    let (charger_border, charger_lines) = if s.plugged_in {
        let ports = if s.power_ports.is_empty() { "no port".to_string() } else { s.power_ports.join(" + ") };
        let source = if s.power_sources > 1 {
            format!("{} sources", s.power_sources)
        } else {
            s.charger_name.clone().unwrap_or_else(|| "unknown adapter".into())
        };
        (
            Color::Green,
            vec![
                Line::from(ports.green().bold()),
                Line::from(source),
                Line::from(format!("{:.1} V × {:.1} A", s.charger_volts, s.charger_amps).dark_gray()),
            ],
        )
    } else {
        (Color::DarkGray, vec![Line::from("UNPLUGGED".dark_gray().dim())])
    };
    node(f, inner, (0, 0), nw, charger_border, "⚡ CHARGER", charger_lines);

    let system = vec![Line::from(format!("load {:.1} W", s.load_w).yellow().bold())];
    node(f, inner, (sx, 0), nw, Color::Cyan, "◆ SYSTEM", system);

    let (batt_border, batt_val) = if s.battery_w > EPS_W {
        (Color::Green, signed_w(s.battery_w).green().bold())
    } else if s.battery_w < -EPS_W {
        (ORANGE, signed_w(s.battery_w).fg(ORANGE).bold())
    } else {
        (Color::DarkGray, "0.0 W".dark_gray())
    };
    node(f, inner, (bx, by), nw, batt_border, "▮ BATTERY", vec![Line::from(batt_val)]);
}

/// Draws one rounded node box: a title on the border, then body lines.
fn node(f: &mut Frame, inner: Rect, at: (i32, i32), nw: i32, border: Color, title: &str, lines: Vec<Line>) {
    let rect = clip(inner, at.0, at.1, nw, NODE_H);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(border))
        .title(Span::styled(format!(" {title} "), Style::new().fg(border).bold()));
    f.render_widget(Paragraph::new(lines).centered().block(block), rect);
}

/// Clamps a box to the inner area so nothing draws outside the panel.
fn clip(inner: Rect, x: i32, y: i32, w: i32, h: i32) -> Rect {
    let x = x.clamp(0, inner.width as i32);
    let y = y.clamp(0, inner.height as i32);
    let w = w.min(inner.width as i32 - x).max(0);
    let h = h.min(inner.height as i32 - y).max(0);
    Rect::new(inner.x + x as u16, inner.y + y as u16, w as u16, h as u16)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    L,
    R,
    U,
    D,
}

impl Side {
    fn opposite(self) -> Self {
        match self {
            Side::L => Side::R,
            Side::R => Side::L,
            Side::U => Side::D,
            Side::D => Side::U,
        }
    }
}

/// Line glyph that joins two sides of one cell.
fn glyph(a: Side, b: Side) -> &'static str {
    use Side::*;
    match (a, b) {
        (L, R) | (R, L) => "─",
        (U, D) | (D, U) => "│",
        (R, D) | (D, R) => "┌",
        (L, D) | (D, L) => "┐",
        (R, U) | (U, R) => "└",
        _ => "┘",
    }
}

fn arrow(d: Side) -> &'static str {
    match d {
        Side::R => "▸",
        Side::L => "◂",
        Side::D => "▾",
        Side::U => "▴",
    }
}

/// Direction of one step between two neighbouring cells.
fn step(a: (i32, i32), b: (i32, i32)) -> Side {
    if b.0 > a.0 {
        Side::R
    } else if b.0 < a.0 {
        Side::L
    } else if b.1 > a.1 {
        Side::D
    } else {
        Side::U
    }
}

/// Expands an axis-aligned polyline into every cell it touches.
fn walk(points: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let mut out = vec![points[0]];
    for pair in points.windows(2) {
        let (mut x, mut y) = pair[0];
        let (tx, ty) = pair[1];
        while (x, y) != (tx, ty) {
            x += (tx - x).signum();
            y += (ty - y).signum();
            out.push((x, y));
        }
    }
    out
}

/// Draws a line. `live` gives the colour of a moving line, None draws a dim idle line.
fn route(buf: &mut Buffer, inner: Rect, points: &[(i32, i32)], live: Option<Color>, frame: u64) {
    let cells = walk(points);
    let n = cells.len();
    if n < 2 {
        return;
    }
    for i in 0..n {
        let d_in = if i == 0 { step(cells[0], cells[1]) } else { step(cells[i - 1], cells[i]) };
        let d_out = if i + 1 < n { step(cells[i], cells[i + 1]) } else { d_in };
        let (x, y) = cells[i];
        let straight = d_in == d_out;
        match live {
            Some(c) if straight && i > 0 && i + 1 < n && (i as u64 % 4) == frame % 4 => {
                put(buf, inner, x, y, arrow(d_out), Style::new().fg(c).bold());
            }
            Some(c) => put(buf, inner, x, y, glyph(d_in.opposite(), d_out), Style::new().fg(c)),
            None => put(buf, inner, x, y, glyph(d_in.opposite(), d_out), Style::new().fg(Color::DarkGray).dim()),
        }
    }
}

/// Writes one symbol at a cell relative to `inner`, skipping cells outside it.
fn put(buf: &mut Buffer, inner: Rect, x: i32, y: i32, sym: &str, style: Style) {
    if x < 0 || y < 0 || x >= inner.width as i32 || y >= inner.height as i32 {
        return;
    }
    if let Some(cell) = buf.cell_mut((inner.x + x as u16, inner.y + y as u16)) {
        cell.set_symbol(sym).set_style(style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flows_follow_power_direction() {
        let charging = Sample { plugged_in: true, battery_w: 3.0, ..Default::default() };
        let draining = Sample { plugged_in: true, battery_w: -1.5, ..Default::default() };
        let unplugged = Sample { battery_w: -4.0, ..Default::default() };
        let idle = Sample::default();
        assert_eq!(flows(&charging), Flows { charger_to_system: true, charger_to_battery: true, battery_to_system: false });
        assert_eq!(flows(&draining), Flows { charger_to_system: true, charger_to_battery: false, battery_to_system: true });
        assert_eq!(flows(&unplugged), Flows { charger_to_system: false, charger_to_battery: false, battery_to_system: true });
        assert_eq!(flows(&idle), Flows::default());
    }
}
