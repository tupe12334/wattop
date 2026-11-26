// Views of the Power Flow widget.
// Both sizes draw the energy loop, after the energy monitor of a hybrid car: CHARGER, SYSTEM
// and BATTERY on a ring.
// One cue per state: a lit leg of the ring says that power moves, and where. No arrow, sign
// or state word repeats it. The colour of a leg names its source: green from the charger,
// orange from the battery. The width of a leg follows the watts on it.
// The widget shows no watts: a number is only right for a moment.
// Small: the loop around the charge level. Medium: the loop beside one row per component.

import AppKit
import CoreText
import SwiftUI
import WidgetKit

struct PowerWidgetView: View {
    @Environment(\.widgetFamily) private var family
    let entry: PowerEntry

    var body: some View {
        PowerContent(
            sample: entry.sample, family: family, frame: entry.frame, batch: entry.batch)
            // macOS drops this background when the desktop dims widgets or the user picks
            // a clear or tinted widget style. The legs still carry their colours then.
            .containerBackground(for: .widget) {
                NetBackground(sample: entry.sample)
            }
    }
}

/// The widget's background: a system fill, washed from the bottom with the colour of the
/// battery's net motion, like the sky of the Weather widget. Green is a net gain, orange a
/// net loss. A battery that holds, or a Mac without one, has no wash. It is the one cue for
/// gain or loss: no word, sign or arrow says it again.
struct NetBackground: View {
    let sample: PowerSample?

    var body: some View {
        ZStack {
            Rectangle().fill(.fill.tertiary)
            if let tint {
                LinearGradient(
                    colors: [tint.opacity(0.42), tint.opacity(0.08)],
                    startPoint: .bottom, endPoint: .top)
            }
        }
    }

    private var tint: Color? {
        switch sample?.motion {
        case .charging: .green
        case .draining: .orange
        default: nil
        }
    }
}

/// The content of the widget in one size. A nil sample is a Mac without a battery.
/// The size is a parameter so that Preview/States.swift can draw it outside WidgetKit.
struct PowerContent: View {
    let sample: PowerSample?
    let family: WidgetFamily
    /// How far the flow animation has run, in steps, and the run it belongs to. See `Flow`.
    var frame = 0.0
    var batch = 0

    var body: some View {
        if let sample {
            Group {
                if family == .systemMedium {
                    MediumView(sample: sample, frame: frame, batch: batch)
                } else {
                    LoopView(sample: sample, showsTime: true, frame: frame, batch: batch)
                }
            }
            .accessibilityElement(children: .ignore)
            .accessibilityLabel(sample.spokenSummary)
        } else {
            NoBatteryView()
        }
    }
}

struct MediumView: View {
    let sample: PowerSample
    let frame: Double
    let batch: Int

    var body: some View {
        HStack(spacing: 14) {
            LoopView(sample: sample, showsTime: false, frame: frame, batch: batch)
                .aspectRatio(1, contentMode: .fit)
            // The rows name the stations of the loop. They hold no watts: see the loop.
            VStack(spacing: 0) {
                DetailRow(
                    symbol: PowerSample.portSymbol, title: sample.portTitle, value: sample.portNote,
                    dimmed: !sample.portActive)
                Divider()
                DetailRow(
                    symbol: sample.levelSymbol, title: "Battery", value: sample.timeLeft,
                    dimmed: false)
                Divider()
                DetailRow(
                    symbol: "laptopcomputer", title: "System", value: "This Mac",
                    dimmed: false)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

/// One component beside the loop: its symbol and name on the left, a note on the right.
private struct DetailRow: View {
    let symbol: String
    let title: String
    let value: String
    let dimmed: Bool

    var body: some View {
        HStack(spacing: 6) {
            Image(systemName: symbol)
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(.secondary)
                .frame(width: 18)
            Text(title)
                .font(.system(size: 13, weight: .semibold))
                .layoutPriority(1)
            Spacer(minLength: 4)
            Text(value)
                .font(.system(size: 12))
                .foregroundStyle(.secondary)
                .opacity(dimmed ? 0.6 : 1)
        }
        .lineLimit(1)
        .minimumScaleFactor(0.8)
        .frame(maxHeight: .infinity)
    }
}

// MARK: - Shared pieces

/// Shown on a Mac without a battery.
struct NoBatteryView: View {
    var body: some View {
        VStack(spacing: 6) {
            Image(systemName: "desktopcomputer")
                .font(.system(size: 24))
            Text("No battery")
                .font(.system(size: 13, weight: .semibold))
        }
        .foregroundStyle(.secondary)
    }
}
