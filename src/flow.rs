//! Energy flow panel: charger, battery and system boxes in one row, joined by animated connectors.

use crate::panel;
use wattop::telemetry::Sample;
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
pub const ORANGE: Color = Color::Rgb(255, 140, 0);
const BOX_H: i32 = 5; // box height, borders included
const BOX_MIN_W: i32 = 14; // narrowest box that fits its text
const GAP_MIN_W: i32 = 5; // narrowest connector gap
const GAP_MAX_W: i32 = 12;

/// A column range inside the panel: (start x, width).
pub type Cols = (i32, i32);

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

/// Columns of the one-line layout: charger, gap 1, battery, gap 2, system.
/// Boxes share one width and the two gaps share one width. Spare cells go to the left margin.
pub fn line_layout(width: i32) -> (Cols, Cols, Cols, Cols, Cols) {
    let width = width.max(0);
    let gap = (width / 8).clamp(GAP_MIN_W, GAP_MAX_W).min(width / 2);
    let box_w = ((width - 2 * gap) / 3).max(0);
    let left = (width - (3 * box_w + 2 * gap)) / 2;
    let charger = left;
    let gap1 = charger + box_w;
    let battery = gap1 + gap;
    let gap2 = battery + box_w;
    let system = gap2 + gap;
    ((charger, box_w), (gap1, gap), (battery, box_w), (gap2, gap), (system, box_w))
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
    let (w, h) = (inner.width as i32, inner.height as i32);
    let (charger, gap1, battery, gap2, system) = line_layout(w);
    if charger.1 < BOX_MIN_W || h < BOX_H {
        compact(f, inner, s, live);
        return;
    }

    let left_live = (live.charger_to_system || live.charger_to_battery).then_some(Color::Green);
    let right_live = if live.battery_to_system {
        Some(ORANGE)
    } else if live.charger_to_system {
        Some(Color::Green)
    } else {
        None
    };
    connector(f.buffer_mut(), inner, gap1, left_live, frame, Some(&format!("{:.1} W", s.input_w)));
    connector(f.buffer_mut(), inner, gap2, right_live, frame, None);

    let (charger_border, charger_lines) = if s.plugged_in {
        let port = if s.power_ports.is_empty() { "no port".to_string() } else { s.power_ports.join(" + ") };
        (Color::Green, vec![Line::from(port.green().bold()), Line::from(format!("{} W", s.adapter_watts))])
    } else {
        (Color::DarkGray, vec![Line::from("UNPLUGGED".dark_gray().dim())])
    };
    node(f, inner, charger, charger_border, "⚡ CHARGER", charger_lines);

    let (batt_color, state, value) = battery_state(s);
    let batt_lines = vec![Line::from(value.fg(batt_color).bold()), Line::from(state.fg(batt_color))];
    node(f, inner, battery, batt_color, "▮ BATTERY", batt_lines);

    let system_lines = vec![Line::from(format!("load {:.1} W", s.load_w).yellow().bold())];
    node(f, inner, system, Color::Cyan, "◆ SYSTEM", system_lines);
}

/// One-line text fallback for panels too narrow or short for boxes. No borders.
fn compact(f: &mut Frame, inner: Rect, s: &Sample, live: Flows) {
    let (charger, charger_style) = if s.plugged_in {
        (format!("⚡ {:.1} W", s.input_w), Style::new().fg(Color::Green).bold())
    } else {
        ("⚡ unplugged".to_string(), Style::new().fg(Color::DarkGray))
    };
    let (batt_color, _, value) = battery_state(s);
    let link = |on: bool, c: Color| Style::new().fg(if on { c } else { Color::DarkGray });
    let left_on = live.charger_to_system || live.charger_to_battery;
    let right_color = if live.battery_to_system { ORANGE } else { Color::Green };
    let right_on = live.battery_to_system || live.charger_to_system;
    let line = Line::from(vec![
        Span::styled(charger, charger_style),
        Span::styled(" ─▸ ", link(left_on, Color::Green)),
        Span::styled(format!("▮ {value}"), Style::new().fg(batt_color).bold()),
        Span::styled(" ─▸ ", link(right_on, right_color)),
        Span::styled(format!("◆ {:.1} W", s.load_w), Style::new().fg(Color::Yellow).bold()),
    ]);
    // Middle row only. Paragraph clips anything wider than the panel.
    let row = Rect { y: inner.y + inner.height / 2, height: inner.height.min(1), ..inner };
    f.render_widget(Paragraph::new(line).centered(), row);
}

/// Draws one box at a column range, with its title on the top border and body lines centred.
fn node(f: &mut Frame, inner: Rect, cols: Cols, border: Color, title: &str, lines: Vec<Line>) {
    let rect = clip(inner, cols.0, 0, cols.1, BOX_H);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(border))
        .title(Span::styled(format!(" {title} "), Style::new().fg(border).bold()));
    let body = block.inner(rect);
    f.render_widget(block, rect);
    let top = body.height.saturating_sub(lines.len() as u16) / 2;
    let text = Rect { y: body.y + top, height: body.height.min(lines.len() as u16), ..body };
    f.render_widget(Paragraph::new(lines).centered(), text);
}

/// Draws one connector gap on the middle row. `live` gives the colour of a moving line,
/// None draws a dim idle line. `label` shows above a live line when it fits the gap.
fn connector(buf: &mut Buffer, inner: Rect, gap: Cols, live: Option<Color>, frame: u64, label: Option<&str>) {
    let row = BOX_H / 2;
    for i in 0..gap.1 {
        let (sym, style) = match live {
            // Chevrons march right: the phase shifts one cell per frame.
            Some(c) if (i as u64 + frame) % 2 == 1 => ("▸", Style::new().fg(c).bold()),
            Some(c) => ("─", Style::new().fg(c)),
            None => ("─", Style::new().fg(Color::DarkGray).dim()),
        };
        put(buf, inner, gap.0 + i, row, sym, style);
    }
    if let (Some(text), Some(c)) = (label, live) {
        let n = text.chars().count() as i32;
        if n <= gap.1 {
            put_str(buf, inner, gap.0 + (gap.1 - n) / 2, row - 1, text, Style::new().fg(c).bold());
        }
    }
}

/// Clamps a box to the inner area so nothing draws outside the panel.
fn clip(inner: Rect, x: i32, y: i32, w: i32, h: i32) -> Rect {
    let x = x.clamp(0, inner.width as i32);
    let y = y.clamp(0, inner.height as i32);
    let w = w.min(inner.width as i32 - x).max(0);
    let h = h.min(inner.height as i32 - y).max(0);
    Rect::new(inner.x + x as u16, inner.y + y as u16, w as u16, h as u16)
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
        let charging = Sample { plugged_in: true, battery_w: 3.0, ..Default::default() };
        let draining = Sample { plugged_in: true, battery_w: -1.5, ..Default::default() };
        let unplugged = Sample { battery_w: -4.0, ..Default::default() };
        let idle = Sample::default();
        assert_eq!(flows(&charging), Flows { charger_to_system: true, charger_to_battery: true, battery_to_system: false });
        assert_eq!(flows(&draining), Flows { charger_to_system: true, charger_to_battery: false, battery_to_system: true });
        assert_eq!(flows(&unplugged), Flows { charger_to_system: false, charger_to_battery: false, battery_to_system: true });
        assert_eq!(flows(&idle), Flows::default());
    }

    #[test]
    fn line_layout_boxes_are_equal_and_disjoint() {
        for width in [0, 1, 7, 40, 52, 72, 100, 120, 200] {
            let (c, g1, b, g2, s) = line_layout(width);
            assert_eq!(c.1, b.1, "width {width}");
            assert_eq!(b.1, s.1, "width {width}");
            assert_eq!(g1.1, g2.1, "width {width}");
            let parts = [c, g1, b, g2, s];
            let mut end = 0;
            for (start, len) in parts {
                assert!(len >= 0, "width {width}");
                assert!(start >= end, "overlap at width {width}");
                end = start + len;
            }
            assert!(end <= width, "overflow at width {width}");
        }
    }

    #[test]
    fn line_layout_fits_common_widths() {
        for width in [40, 72, 120] {
            let (c, g1, b, g2, s) = line_layout(width);
            assert!(g1.1 >= GAP_MIN_W && g2.1 >= GAP_MIN_W, "gaps at width {width}");
            assert_eq!(c.0, 0, "left margin at width {width}");
            assert!(s.0 + s.1 <= width && b.0 == g1.0 + g1.1, "layout at width {width}");
        }
    }
}
