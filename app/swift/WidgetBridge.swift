// Bridge from the Rust menu-bar app to WidgetKit and the window list.
// WidgetKit has no C API, so app/build.rs compiles this file and links it into the app.
// See app/src/widget.rs for the caller.

import AppKit
import CoreGraphics
import WidgetKit

// Rust calls both functions from its sampling thread, which has no autorelease pool that
// drains. Each one runs in its own pool, or the objects AppKit and the window list
// autorelease would pile up on every sample.

/// Asks WidgetKit to reload the Power Flow widget. The kind is the one in
/// widget/Sources/WattopWidget.swift.
@_cdecl("wattop_widget_reload")
public func wattopWidgetReload() {
    autoreleasepool {
        if #available(macOS 11.0, *) {
            WidgetCenter.shared.reloadTimelines(ofKind: "power-flow")
        }
    }
}

/// True when the user can see a desktop widget: one is on the current Space and no window
/// covers all of it. False when the display sleeps or the screen is locked.
/// The window list gives no widget names without the Screen Recording permission, so any
/// desktop widget counts, not only Power Flow.
@_cdecl("wattop_widgets_on_screen")
public func wattopWidgetsOnScreen() -> Bool {
    autoreleasepool { widgetsOnScreen() }
}

private func widgetsOnScreen() -> Bool {
    let session = CGSessionCopyCurrentDictionary() as? [String: Any]
    if session?["CGSSessionScreenIsLocked"] as? Bool == true { return false }
    if CGDisplayIsAsleep(CGMainDisplayID()) != 0 { return false }
    let onScreen = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID)
    guard let windows = onScreen as? [[String: Any]] else { return false }

    // The list runs front to back. App windows seen so far can cover what comes after them.
    var covers: [CGRect] = []
    for window in windows {
        guard let layer = window[kCGWindowLayer as String] as? Int,
            let box = window[kCGWindowBounds as String] as? NSDictionary,
            let bounds = CGRect(dictionaryRepresentation: box)
        else { continue }
        if layer == 0 {
            if window[kCGWindowAlpha as String] as? Double ?? 1 > 0 { covers.append(bounds) }
        } else if layer < 0, isWidgetHost(window[kCGWindowOwnerPID as String] as? pid_t) {
            // Desktop widgets sit below every app window, just above the desktop icons.
            let seen = probes(bounds).contains { point in !covers.contains { $0.contains(point) } }
            if seen { return true }
        }
    }
    return false
}

/// Notification Center draws the desktop widgets. Its process name is localized, so match
/// the bundle id.
private func isWidgetHost(_ pid: pid_t?) -> Bool {
    guard let pid else { return false }
    return NSRunningApplication(processIdentifier: pid)?.bundleIdentifier
        == "com.apple.notificationcenterui"
}

/// The centre of a widget and four points near its corners. A widget counts as seen when
/// one of them is clear.
private func probes(_ r: CGRect) -> [CGPoint] {
    let inset = r.insetBy(dx: r.width / 4, dy: r.height / 4)
    return [
        CGPoint(x: r.midX, y: r.midY),
        CGPoint(x: inset.minX, y: inset.minY),
        CGPoint(x: inset.maxX, y: inset.minY),
        CGPoint(x: inset.minX, y: inset.maxY),
        CGPoint(x: inset.maxX, y: inset.maxY),
    ]
}
