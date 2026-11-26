// Presentation rules of a power sample, and the flow animation constants.

import Foundation

extension PowerSample {
    /// Charge level, e.g. "86%".
    var level: String { "\(min(max(percent, 0), 100))%" }

    /// Time to full or empty, e.g. "1h 20m" or "45 min". Empty when macOS has no estimate.
    var timeLeft: String {
        guard let minutes = minutesLeft else { return "" }
        if minutes < 60 { return "\(minutes) min" }
        return "\(minutes / 60)h \(String(format: "%02d", minutes % 60))m"
    }

    /// SF Symbol of the power port station. It is a plug for power in and for power out:
    /// the lit leg and its pulses show which way the power moves.
    static let portSymbol = "powerplug.fill"

    /// Power moves through the port station, in or out.
    var portActive: Bool { pluggedIn || powersDevice }

    /// Name of the port station in the rows.
    var portTitle: String {
        supplies || !powersDevice ? "Charger" : "Device"
    }

    /// The port in use, e.g. "USB-C port 2". Two ports are too long for a row, so they
    /// become a count. A charger that delivers power while a device is on another port
    /// names the device's port, e.g. "Port 2 out": the charger's own port matters less.
    var portNote: String {
        guard portActive else { return "Not connected" }
        if supplies && powersDevice {
            if let only = portOut.first, portOut.count == 1 {
                return "\(only.name.replacingOccurrences(of: "USB-C p", with: "P")) out"
            }
            return "\(portOut.count) ports out"
        }
        let names = powersDevice ? portOut.map(\.name) : ports
        switch names.count {
        case 0: return "\(adapterWatts) W adapter"
        case 1: return names[0]
        default: return "\(names.count) ports"
        }
    }

    /// SF Symbol for the charge level. It has no bolt: the lit leg already says "charging".
    var levelSymbol: String {
        switch percent {
        case ..<13: "battery.0percent"
        case ..<38: "battery.25percent"
        case ..<63: "battery.50percent"
        case ..<88: "battery.75percent"
        default: "battery.100percent"
        }
    }

    /// The whole reading as one sentence, for VoiceOver. The loop itself is not spoken.
    /// It has no watts, like the widget.
    var spokenSummary: String {
        // The same rules as the legs of the loop.
        let battery =
            if charges {
                "Battery charging"
            } else if drains {
                "Battery draining"
            } else {
                "Battery holding"
            }
        let time = timeLeft.isEmpty ? "" : ", \(timeLeft) left"
        let charger =
            if supplies {
                "Charger connected"
            } else if pluggedIn {
                "Charger connected, delivering no power"
            } else {
                "Charger not connected"
            }
        let device = powersDevice ? " A port powers a device." : ""
        return "\(battery), \(level)\(time). \(charger).\(device)"
    }
}

/// The flow animation. A widget cannot run an animation of its own. The timeline holds one
/// entry for each step, and macOS animates from one entry to the next. A run is one unbroken
/// motion over several timelines. Its entries share a batch number.
enum Flow {
    /// Seconds from one step to the next.
    static let step: TimeInterval = 2
    /// Degrees the pulses move along the ring in one step.
    static let turn = 45.0
    /// Degrees from the start of one pulse to the start of the next.
    static let spacing = 90.0
    /// Degrees of one pulse, head and tail. The tail ends just before the next head.
    static let pulse = 84.0
    /// Width of a pulse at the end of its tail, as a share of its width at its head.
    static let tail = 0.15
    /// Share of a pulse, from its head, that has its full colour. The rest fades out.
    static let solid = 0.4
    /// Steps in one timeline. They cover the time to the next reload from the app.
    static let steps = 12
    /// Seconds a new run shows its still picture before the motion starts.
    static let lead: TimeInterval = 0.3
    /// Pace of each leg, in the order port to system, port and battery, battery to system.
    /// The small differences make two legs pass through "in step" about every half minute.
    static let paces = [1.0, 1.14, 0.88]
}
