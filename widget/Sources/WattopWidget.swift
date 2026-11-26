// The Power Flow widget: its timeline and its configuration.

import SwiftUI
import WidgetKit

/// One reading on the timeline. `sample` is nil on a Mac without a battery.
struct PowerEntry: TimelineEntry {
    let date: Date
    let sample: PowerSample?
    /// How far the flow animation has run, in steps, and the run it belongs to.
    /// See `Flow` in PowerPresentation.swift.
    var frame = 0.0
    var batch = 0
}

struct PowerProvider: TimelineProvider {
    /// Seconds until the widget asks for a reading by itself. This is only a fallback for when
    /// the menu-bar app does not run. The app reloads the widget while the user can see it.
    /// See app/src/widget.rs.
    static let refresh: TimeInterval = 15 * 60

    func placeholder(in context: Context) -> PowerEntry {
        PowerEntry(date: .now, sample: .preview)
    }

    func getSnapshot(in context: Context, completion: @escaping (PowerEntry) -> Void) {
        // The gallery shows a snapshot. A Mac without a battery gets the sample reading there.
        let sample = PowerSample.read() ?? (context.isPreview ? .preview : nil)
        completion(PowerEntry(date: .now, sample: sample))
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<PowerEntry>) -> Void) {
        // One reading, drawn as several steps of the flow animation.
        let sample = PowerSample.read()
        let now = Date.now
        var dates: [Date]
        var frames: [Double]
        let run: Date
        if let running = FlowRun.running(at: now) {
            // The last timeline still runs. Go on from it: the first entry is one step ahead
            // of what the widget shows now, and macOS animates to it. So a reload makes no
            // stop in the motion.
            run = running
            let shown = now.timeIntervalSince(run) / Flow.step
            dates = (0..<Flow.steps).map { now + Flow.step * Double($0) }
            frames = (0..<Flow.steps).map { shown + Double($0) + 1 }
        } else {
            // The pulses stood still: the widget was hidden, or it is new. Start a new run.
            // The first entry is the still picture. The motion starts a moment later.
            run = now + Flow.lead
            dates = [now] + (0..<Flow.steps).map { run + Flow.step * Double($0) }
            frames = [0] + (0..<Flow.steps).map { Double($0) + 1 }
        }
        FlowRun.save(run: run, end: (dates.last ?? now) + Flow.step)
        let batch = Int(run.timeIntervalSinceReferenceDate) % 1_000_000
        let entries = zip(dates, frames).map {
            PowerEntry(date: $0, sample: sample, frame: $1, batch: batch)
        }
        completion(Timeline(entries: entries, policy: .after(.now + Self.refresh)))
    }
}

/// The run of the flow animation, kept between reloads in the defaults of the widget.
/// A run is one unbroken motion. It ends when a timeline plays out before the next reload.
private enum FlowRun {
    private static let runKey = "flowRun"
    private static let endKey = "flowEnd"

    /// The start of the run, when its last timeline still has steps to play.
    static func running(at now: Date) -> Date? {
        let store = UserDefaults.standard
        let (run, end) = (store.double(forKey: runKey), store.double(forKey: endKey))
        guard run > 0, now.timeIntervalSinceReferenceDate <= end else { return nil }
        return Date(timeIntervalSinceReferenceDate: run)
    }

    static func save(run: Date, end: Date) {
        let store = UserDefaults.standard
        store.set(run.timeIntervalSinceReferenceDate, forKey: runKey)
        store.set(end.timeIntervalSinceReferenceDate, forKey: endKey)
    }
}

@main
struct WattopWidget: Widget {
    var body: some WidgetConfiguration {
        StaticConfiguration(kind: "power-flow", provider: PowerProvider()) { entry in
            PowerWidgetView(entry: entry)
        }
        .configurationDisplayName("Power Flow")
        .description("Where your power flows, and whether the battery gains or loses.")
        .supportedFamilies([.systemSmall, .systemMedium])
    }
}

extension PowerSample {
    /// A plausible reading for the placeholder and the widget gallery.
    static let preview = PowerSample(
        percent: 86,
        pluggedIn: true,
        fullyCharged: false,
        adapterWatts: 65,
        inputW: 61.5,
        loadW: 9.1,
        batteryW: 52.4,
        minutesLeft: 80,
        ports: ["USB-C port 2"],
        portOut: []
    )
}
