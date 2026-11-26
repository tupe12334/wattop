//! Reloads the desktop widget, only while the user can see a desktop widget.
//! WidgetKit has no "now visible" event for a widget, so the app watches the window list.
//! The Swift side is in app/swift/WidgetBridge.swift.

use std::time::{Duration, Instant};

/// Time between reloads while a widget stays on screen.
// ponytail: the widget reads the IORegistry watts, which change about every 20 s.
// A shorter interval reloads the same numbers. macOS grants this pace while a widget is visible.
const INTERVAL: Duration = Duration::from_secs(20);

unsafe extern "C" {
    fn wattop_widget_reload();
    fn wattop_widgets_on_screen() -> bool;
}

/// Samples in a row that must see a widget before the first reload. A swipe between
/// Spaces shows the desktop for a moment, and that is not the user looking at it.
const SETTLE: u32 = 2;

/// Decides when to reload. Call `tick` once per sample.
#[derive(Default)]
pub struct Reloader {
    last: Option<Instant>,
    /// Samples in a row that saw a desktop widget.
    seen: u32,
    /// What the port did at the last sample.
    port: Option<Port>,
}

/// What the power port does: a charger is plugged in, and a port powers a device.
/// A change of either turns a station or a leg of the widget on or off.
pub type Port = (bool, bool);

impl Reloader {
    /// `port` is None when the sample failed.
    pub fn tick(&mut self, port: Option<Port>) {
        // SAFETY: both functions take no arguments, keep no pointers and are thread-safe.
        let visible = unsafe { wattop_widgets_on_screen() };
        self.seen = if visible {
            self.seen.saturating_add(1)
        } else {
            0
        };
        let changed = port.is_some() && self.port.is_some() && port != self.port;
        if port.is_some() {
            self.port = port;
        }
        if due(self.seen, self.last.map(|t| t.elapsed()), changed) {
            // SAFETY: as above.
            unsafe { wattop_widget_reload() };
            self.last = Some(Instant::now());
        }
    }
}

/// Reload when a widget has settled on screen, then every `INTERVAL` while it stays there.
/// Reload at once when the port changes, so that a plug or an unplug shows without a wait.
/// Never reload a hidden widget: it reloads when it comes back.
fn due(seen: u32, since_last: Option<Duration>, port_changed: bool) -> bool {
    seen == SETTLE || (seen > SETTLE && (port_changed || since_last.is_none_or(|d| d >= INTERVAL)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_widget_never_reloads() {
        assert!(!due(0, None, false));
        assert!(!due(0, Some(Duration::from_secs(3600)), true));
    }

    #[test]
    fn a_glimpse_of_the_desktop_does_not_reload() {
        assert!(!due(SETTLE - 1, None, false));
        assert!(!due(SETTLE - 1, Some(Duration::from_secs(3600)), true));
    }

    #[test]
    fn widget_reloads_when_it_settles_on_screen() {
        assert!(due(SETTLE, None, false));
        // Coming back counts, however short the time since the last reload.
        assert!(due(SETTLE, Some(Duration::from_secs(1)), false));
    }

    #[test]
    fn visible_widget_reloads_once_per_interval() {
        assert!(!due(
            SETTLE + 5,
            Some(INTERVAL - Duration::from_secs(1)),
            false
        ));
        assert!(due(SETTLE + 5, Some(INTERVAL), false));
    }

    #[test]
    fn visible_widget_reloads_at_once_on_a_plug_or_an_unplug() {
        assert!(due(SETTLE + 5, Some(Duration::from_secs(1)), true));
    }
}
