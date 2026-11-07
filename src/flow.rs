//! Energy flow panel: a power bus junction. CHARGER and SYSTEM sit on one row, BATTERY sits under the junction.

use crate::panel;
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph},
};
use wattop::telemetry::Sample;

/// Battery watts within this band count as holding.
pub const EPS_W: f64 = 0.05;
pub const ORANGE: Color = Color::Rgb(255, 140, 0);
const BOX_H: u16 = 5; // box height, borders included
const BOX_W: u16 = 15; // odd, so BATTERY centres exactly under the junction
const BUS_ROW: u16 = 2; // middle row of the top boxes, where the bus runs
const GAP_MIN: u16 = 6; // narrowest bus gap that still fits a label
const GAP_MAX: u16 = 14;
const INNER_H: u16 = 2 * BOX_H; // top boxes, then BATTERY
/// Panel rows the bus layout needs, borders included.
pub const PANEL_H: u16 = INNER_H + 2;

/// Which energy lines carry power right now. The bus to SYSTEM is always live.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flows {
    /// The charger feeds the bus.
    pub charger_to_bus: bool,
    /// The battery feeds the bus, which happens when it drains.
    pub battery_to_bus: bool,
    /// The bus feeds the battery, which happens when it charges.
    pub bus_to_battery: bool,
}

/// Decides which lines are live. Each line has one fixed direction.
pub fn flows(s: &Sample) -> Flows {
    Flows {
        charger_to_bus: s.plugged_in,
        battery_to_bus: s.battery_w < -EPS_W,
        bus_to_battery: s.battery_w > EPS_W,
    }
}

/// Signed watts, with a real minus sign.
pub fn signed_w(v: f64) -> String {
    format!("{v:+.1} W").replace('-', "−")
}

/// Box and junction positions, relative to the panel's inner area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BusRects {
    charger: Rect,
    system: Rect,
    battery: Rect,
    /// Column of the junction ●. The vertical link runs down this column.
    junction: u16,
    /// Width of each bus gap, left and right of the junction.
    gap: u16,
}

/// Places the three equal boxes around the junction. None when the area is too small.
fn bus_layout(width: u16, height: u16) -> Option<BusRects> {
    let span = 2 * BOX_W + 1; // charger, gap, junction, gap, system
    if width < span + 2 * GAP_MIN || height < INNER_H {
        return None;
    }
    let gap = ((width - span) / 2).min(GAP_MAX);
    let left = (width - span - 2 * gap) / 2;
    let junction = left + BOX_W + gap;
    Some(BusRects {
        charger: Rect::new(left, 0, BOX_W, BOX_H),
        system: Rect::new(junction + 1 + gap, 0, BOX_W, BOX_H),
        battery: Rect::new(junction - BOX_W / 2, BOX_H, BOX_W, BOX_H),
        junction,
        gap,
    })
}

/// Battery colour, state word and value text.
fn battery_state(s: &Sample) -> (Color, &'static str, String) {
    if s.battery_w > EPS_W {
        (Color::Green, "▲ charging", signed_w(s.battery_w))
    } else if s.battery_w < -EPS_W {
        (ORANGE, "▼ draining", signed_w(s.battery_w))
    } else {
        (Color::Cyan, "■ holding", "0.0 W".to_string())
    }
}

pub fn draw(f: &mut Frame, area: Rect, s: &Sample, frame: u64) {
    let block = panel("energy flow");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let live = flows(s);
    let Some(b) = bus_layout(inner.width, inner.height) else {
        compact(f, inner, s, live);
        return;
    };
    {
        let buf = f.buffer_mut();
        bus_row(buf, inner, &b, s, live, frame);
        vertical_link(buf, inner, &b, s, live, frame);
    }

    let (charger_border, charger_lines) = if s.plugged_in {
        let port = if s.power_ports.is_empty() {
            "no port".to_string()
        } else {
            s.power_ports.join(" + ")
        };
        (
            Color::Green,
            vec![
                Line::from(port.green().bold()),
                Line::from(format!("{} W", s.adapter_watts)),
            ],
        )
    } else {
        (
            Color::DarkGray,
            vec![Line::from("UNPLUGGED".dark_gray().dim())],
        )
    };
    node(
        f,
        inner,
        b.charger,
        charger_border,
        "⚡ CHARGER",
        charger_lines,
    );

    let (batt_color, state, value) = battery_state(s);
    let batt_lines = vec![
        Line::from(value.fg(batt_color).bold()),
        Line::from(state.fg(batt_color)),
    ];
    node(f, inner, b.battery, batt_color, "▮ BATTERY", batt_lines);

    let system_lines = vec![Line::from(
        format!("load {:.1} W", s.load_w).yellow().bold(),
    )];
    node(f, inner, b.system, Color::Cyan, "◆ SYSTEM", system_lines);
}

/// Draws the bus row: charger into the junction, then junction out to the system.
fn bus_row(buf: &mut Buffer, inner: Rect, b: &BusRects, s: &Sample, live: Flows, frame: u64) {
    let row = BUS_ROW as i32;
    let gap = b.gap as i32;
    let left_x = b.charger.right() as i32;
    let right_x = b.junction as i32 + 1;
    let left_color = s.plugged_in.then_some(Color::Green);
    let right_color = if live.battery_to_bus {
        ORANGE
    } else {
        Color::Green
    };

    chevrons(buf, inner, left_x, gap, row, left_color, frame);
    chevrons(buf, inner, right_x, gap, row, Some(right_color), frame);
    put(
        buf,
        inner,
        b.junction as i32,
        row,
        "●",
        Style::new().fg(Color::White).bold(),
    );
    if s.plugged_in {
        gap_label(
            buf,
            inner,
            left_x,
            gap,
            &format!("{:.1} W", s.input_w),
            Color::Green,
        );
    }
    gap_label(
        buf,
        inner,
        right_x,
        gap,
        &format!("{:.1} W", s.load_w),
        right_color,
    );
}

/// Draws the vertical link under the junction, then the battery draw label beside it.
fn vertical_link(buf: &mut Buffer, inner: Rect, b: &BusRects, s: &Sample, live: Flows, frame: u64) {
    let x = b.junction as i32;
    let (color, down) = (Color::Green, live.bus_to_battery);
    let up_color = ORANGE;
    let link_color = if down {
        color
    } else if live.battery_to_bus {
        up_color
    } else {
        Color::DarkGray
    };
    for y in (BUS_ROW as i32 + 1)..(BOX_H as i32) {
        let (sym, style) = if down {
            // Chevrons march down: the phase shifts one row per frame.
            let sym = if (y as u64 + frame) % 2 == 1 {
                "▾"
            } else {
                "│"
            };
            (sym, Style::new().fg(link_color).bold())
        } else if live.battery_to_bus {
            // Chevrons march up.
            let sym = if (y as u64 + frame).is_multiple_of(2) {
                "▴"
            } else {
                "│"
            };
            (sym, Style::new().fg(link_color).bold())
        } else {
            ("│", Style::new().fg(link_color).dim())
        };
        put(buf, inner, x, y, sym, style);
    }
    // Label beside the link. It must stay inside the gap, so it never touches the SYSTEM box.
    let text = format!("{:.1} W", s.battery_w.abs());
    if (text.chars().count() as i32) < b.gap as i32 {
        put_str(
            buf,
            inner,
            x + 2,
            BUS_ROW as i32 + 1,
            &text,
            Style::new().fg(link_color).bold(),
        );
    }
}

/// One-line label centred above a bus gap, shown only when it fits the gap.
fn gap_label(buf: &mut Buffer, inner: Rect, x: i32, width: i32, text: &str, color: Color) {
    let n = text.chars().count() as i32;
    if n <= width {
        put_str(
            buf,
            inner,
            x + (width - n) / 2,
            BUS_ROW as i32 - 1,
            text,
            Style::new().fg(color).bold(),
        );
    }
}

/// Draws one run of bus cells on a row. A live run shows chevrons that march right.
fn chevrons(
    buf: &mut Buffer,
    inner: Rect,
    x: i32,
    width: i32,
    y: i32,
    live: Option<Color>,
    frame: u64,
) {
    for i in 0..width {
        let (sym, style) = match live {
            Some(c) if (i as u64 + frame) % 2 == 1 => ("▸", Style::new().fg(c).bold()),
            Some(c) => ("─", Style::new().fg(c)),
            None => ("─", Style::new().fg(Color::DarkGray).dim()),
        };
        put(buf, inner, x + i, y, sym, style);
    }
}

/// Two-line text fallback for panels too small for boxes. No borders.
/// Line 1 is the bus. Line 2 hangs the battery below the junction.
fn compact(f: &mut Frame, inner: Rect, s: &Sample, live: Flows) {
    let (charger, charger_style) = if s.plugged_in {
        (
            format!("⚡ {:.1} W", s.input_w),
            Style::new().fg(Color::Green).bold(),
        )
    } else {
        ("⚡ unplugged".to_string(), Style::new().fg(Color::DarkGray))
    };
    let (batt_color, state, value) = battery_state(s);
    let link_in = Style::new().fg(if s.plugged_in {
        Color::Green
    } else {
        Color::DarkGray
    });
    let link_out = Style::new().fg(if live.battery_to_bus {
        ORANGE
    } else {
        Color::Green
    });
    let line = [
        (charger, charger_style),
        (" ─▸ ".to_string(), link_in),
        ("●".to_string(), Style::new().fg(Color::White).bold()),
        (" ─▸ ".to_string(), link_out),
        (
            format!("◆ {:.1} W", s.load_w),
            Style::new().fg(Color::Yellow).bold(),
        ),
    ];
    let junction = line[0].0.chars().count() as i32 + 4; // column of ● in line 1
    let width: i32 = line.iter().map(|(t, _)| t.chars().count() as i32).sum();
    let x0 = (inner.width as i32 - width) / 2;
    let y0 = (inner.height as i32 - 2).max(0) / 2;
    let buf = f.buffer_mut();
    let mut x = x0;
    for (text, style) in &line {
        put_str(buf, inner, x, y0, text, *style);
        x += text.chars().count() as i32;
    }
    let branch = format!("▾ ▮ {value} {state}");
    put_str(
        buf,
        inner,
        x0 + junction,
        y0 + 1,
        &branch,
        Style::new().fg(batt_color).bold(),
    );
}

/// Draws one box at a relative rectangle, with its title centred on the top border and body lines centred.
fn node(f: &mut Frame, inner: Rect, rect: Rect, border: Color, title: &str, lines: Vec<Line>) {
    let rect = Rect {
        x: inner.x + rect.x,
        y: inner.y + rect.y,
        ..rect
    };
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(border))
        .title(Span::styled(
            format!(" {title} "),
            Style::new().fg(border).bold(),
        ))
        .title_alignment(Alignment::Center);
    let body = block.inner(rect);
    f.render_widget(block, rect);
    let top = body.height.saturating_sub(lines.len() as u16) / 2;
    let text = Rect {
        y: body.y + top,
        height: body.height.min(lines.len() as u16),
        ..body
    };
    f.render_widget(Paragraph::new(lines).centered(), text);
}

/// Writes one symbol at a cell relative to `inner`, skipping cells outside it.
pub fn put(buf: &mut Buffer, inner: Rect, x: i32, y: i32, sym: &str, style: Style) {
    if x < 0 || y < 0 || x >= inner.width as i32 || y >= inner.height as i32 {
        return;
    }
    if let Some(cell) = buf.cell_mut((inner.x + x as u16, inner.y + y as u16)) {
        cell.set_symbol(sym).set_style(style);
    }
}

/// Writes text one cell per char, starting at a cell relative to `inner`.
pub fn put_str(buf: &mut Buffer, inner: Rect, x: i32, y: i32, text: &str, style: Style) {
    for (i, ch) in text.chars().enumerate() {
        put(buf, inner, x + i as i32, y, &ch.to_string(), style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flows_follow_power_direction() {
        let charging = Sample {
            plugged_in: true,
            battery_w: 3.0,
            ..Default::default()
        };
        let draining = Sample {
            plugged_in: true,
            battery_w: -1.5,
            ..Default::default()
        };
        let unplugged = Sample {
            battery_w: -4.0,
            ..Default::default()
        };
        let idle = Sample::default();
        let want = |charger_to_bus, battery_to_bus, bus_to_battery| Flows {
            charger_to_bus,
            battery_to_bus,
            bus_to_battery,
        };
        assert_eq!(flows(&charging), want(true, false, true));
        assert_eq!(flows(&draining), want(true, true, false));
        assert_eq!(flows(&unplugged), want(false, true, false));
        assert_eq!(flows(&idle), Flows::default());
    }

    #[test]
    fn bus_boxes_are_equal_disjoint_and_inside() {
        for width in [43, 72, 80, 120, 200] {
            let b = bus_layout(width, INNER_H).expect("fits");
            assert_eq!(b.charger.width, b.system.width, "width {width}");
            assert_eq!(b.charger.width, b.battery.width, "width {width}");
            assert_eq!(b.charger.height, b.battery.height, "width {width}");
            assert_eq!(b.system.height, b.battery.height, "width {width}");
            for r in [b.charger, b.system, b.battery] {
                assert!(
                    r.right() <= width && r.bottom() <= INNER_H,
                    "outside at width {width}"
                );
            }
            assert!(
                !b.charger.intersects(b.system),
                "charger/system at width {width}"
            );
            assert!(
                !b.charger.intersects(b.battery),
                "charger/battery at width {width}"
            );
            assert!(
                !b.system.intersects(b.battery),
                "system/battery at width {width}"
            );
            // The junction and its link sit in the gaps, not in a box.
            assert!(
                b.charger.right() <= b.junction && b.junction < b.system.x,
                "junction at width {width}"
            );
        }
    }

    #[test]
    fn battery_sits_under_junction() {
        for width in [72, 80, 120] {
            let b = bus_layout(width, INNER_H).expect("fits");
            let centre = b.battery.x + b.battery.width / 2;
            assert!(centre.abs_diff(b.junction) <= 1, "width {width}");
        }
    }

    #[test]
    fn bus_layout_needs_room() {
        assert!(bus_layout(42, INNER_H).is_none());
        assert!(bus_layout(43, INNER_H - 1).is_none());
        assert!(bus_layout(43, INNER_H).is_some());
    }
}
