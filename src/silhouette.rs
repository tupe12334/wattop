//! Top-down laptop outline that shows which physical port the charger uses.

use crate::{flow, panel};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
};
use wattop::telemetry::Sample;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// One physical port. `slot` counts from the back (1) to the front (`slots`).
#[derive(Debug, PartialEq, Eq)]
pub struct PortPos {
    /// Name as `Sample.power_ports` spells it, e.g. "USB-C port 2". Never-powered ports use their own name.
    pub name: &'static str,
    /// Short label drawn on the outline, e.g. "USB-C 2".
    pub short: &'static str,
    pub side: Side,
    pub slot: u8,
    pub slots: u8,
}

/// Power-capable ports per model. Only verified layouts are listed. Never guess.
/// Non-power ports (HDMI, SD, audio) are left out on purpose.
const MAC16_1: &[PortPos] = &[
    PortPos {
        name: "MagSafe",
        short: "MagSafe",
        side: Side::Left,
        slot: 1,
        slots: 4,
    },
    PortPos {
        name: "USB-C port 2",
        short: "USB-C 2",
        side: Side::Left,
        slot: 2,
        slots: 4,
    },
    PortPos {
        name: "USB-C port 1",
        short: "USB-C 1",
        side: Side::Left,
        slot: 3,
        slots: 4,
    },
    PortPos {
        name: "USB-C port 4",
        short: "USB-C 4",
        side: Side::Right,
        slot: 2,
        slots: 3,
    },
];

/// Port table for a `hw.model` id. None when the layout is not verified.
pub fn layout(model: &str) -> Option<&'static [PortPos]> {
    match model {
        "Mac16,1" => Some(MAC16_1),
        _ => None,
    }
}

/// Reads `hw.model`, e.g. "Mac16,1". Empty when sysctl fails.
pub fn model() -> String {
    std::process::Command::new("sysctl")
        .args(["-n", "hw.model"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .unwrap_or_default()
}

const BODY_W: i32 = 12;
const BODY_H: i32 = 7;
/// Width that leaves room for labels and cables on both sides.
const LABEL_W: i32 = 36;

/// Body row for a port. Slots spread over the four interior rows.
fn row(p: &PortPos) -> i32 {
    1 + (p.slot as i32 - 1) * 4 / (p.slots as i32 - 1).max(1)
}

pub fn draw(f: &mut Frame, area: Rect, s: &Sample, ports: Option<&'static [PortPos]>) {
    let watts = if s.plugged_in {
        format!(" in {:.1} W ", s.input_w)
    } else {
        " on battery ".into()
    };
    let block = panel("ports").title_bottom(Span::styled(
        watts,
        Style::new().fg(Color::LightGreen).bold(),
    ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let fits = inner.width as i32 >= BODY_W && inner.height as i32 >= BODY_H;
    match ports {
        Some(ports) if fits => outline(f, inner, s, ports),
        _ => list(f, inner, s),
    }
}

/// Fallback when the model is unknown or the panel is too small.
fn list(f: &mut Frame, inner: Rect, s: &Sample) {
    let lines: Vec<Line> = if s.power_ports.is_empty() {
        vec![Line::from("no power port".dark_gray())]
    } else {
        s.power_ports
            .iter()
            .map(|p| Line::from(p.clone()).light_green().bold())
            .collect()
    };
    f.render_widget(Paragraph::new(lines), inner);
}

fn outline(f: &mut Frame, inner: Rect, s: &Sample, ports: &[PortPos]) {
    let w = inner.width as i32;
    let bx = (w - BODY_W) / 2; // body left edge
    let rb = bx + BODY_W - 1; // body right edge
    let labels = w >= LABEL_W;
    let dim = Style::new().fg(Color::DarkGray);
    let buf = f.buffer_mut();

    let body = [
        "╭──────────╮".to_string(),
        "│▪▪▪▪▪▪▪▪▪▪│".to_string(),
        "│▪▪▪▪▪▪▪▪▪▪│".to_string(),
        "│   ┌──┐   │".to_string(),
        "│   └──┘   │".to_string(),
        "│          │".to_string(),
        "╰──────────╯".to_string(),
    ];
    for (y, line) in body.iter().enumerate() {
        for (x, ch) in line.chars().enumerate() {
            flow::put(buf, inner, bx + x as i32, y as i32, &ch.to_string(), dim);
        }
    }

    for p in ports {
        let y = row(p);
        let on = s.power_ports.iter().any(|n| n == p.name);
        let style = if on {
            Style::new().fg(Color::LightGreen).bold()
        } else {
            dim
        };
        let notch = if p.side == Side::Left { "▐" } else { "▌" };
        let edge = if p.side == Side::Left { bx } else { rb };
        flow::put(buf, inner, edge, y, notch, style);
        if !labels {
            continue;
        }
        let text_style = if on { style } else { dim };
        match p.side {
            Side::Left => {
                if on {
                    for x in (bx - 4)..bx {
                        flow::put(buf, inner, x, y, if x == bx - 1 { "▶" } else { "═" }, style);
                    }
                }
                let start = bx - 5 - p.short.chars().count() as i32;
                flow::put_str(buf, inner, start, y, p.short, text_style);
            }
            Side::Right => {
                if on {
                    for x in (rb + 1)..(rb + 5) {
                        flow::put(buf, inner, x, y, if x == rb + 1 { "◀" } else { "═" }, style);
                    }
                }
                flow::put_str(buf, inner, rb + 5, y, p.short, text_style);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usb_c_2_is_left_slot_2_of_4_on_mac16_1() {
        let ports = layout("Mac16,1").unwrap();
        let p = ports.iter().find(|p| p.name == "USB-C port 2").unwrap();
        assert_eq!((p.side, p.slot, p.slots), (Side::Left, 2, 4));
    }

    #[test]
    fn mac16_1_lists_only_power_ports() {
        let ports = layout("Mac16,1").unwrap();
        assert_eq!(ports.len(), 4);
        assert!(
            ports
                .iter()
                .all(|p| p.name == "MagSafe" || p.name.starts_with("USB-C")),
            "non-power port in layout: {ports:?}"
        );
    }

    #[test]
    fn unknown_model_has_no_layout() {
        assert!(layout("MacBookPro99,1").is_none());
    }
}
