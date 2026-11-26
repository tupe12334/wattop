// Renders every state of the Power Flow widget into one PNG for each appearance, and the
// README picture, docs/media/widget-{dark,light}.png.
// It compiles the real views from ../Sources, without WidgetKit's timeline code.
// Run it with ../preview.sh. Add a case here when the widget gains a state.

import AppKit
import SwiftUI
import WidgetKit

/// Stands in for the timeline entry in WattopWidget.swift, which this tool does not compile.
struct PowerEntry {
    let date: Date
    let sample: PowerSample?
    var frame = 0.0
    var batch = 0
}

/// One state to draw. A nil sample is a Mac without a battery.
struct StateCase {
    let name: String
    let detail: String
    let sample: PowerSample?
}

private func sample(
    percent: Int, plugged: Bool, full: Bool = false, adapter: Int = 0, input: Double = 0,
    load: Double, battery: Double, minutes: Int? = nil, ports: [String] = [],
    out: [PowerSample.PortOut] = []
) -> PowerSample {
    PowerSample(
        percent: percent, pluggedIn: plugged, fullyCharged: full, adapterWatts: adapter,
        inputW: input, loadW: load, batteryW: battery, minutesLeft: minutes, ports: ports,
        portOut: out)
}

let cases: [StateCase] = [
    StateCase(
        name: "Charging, fast", detail: "A wide leg: many watts into the battery.",
        sample: sample(
            percent: 65, plugged: true, adapter: 60, input: 50.6, load: 10.4, battery: 40.2,
            minutes: 70, ports: ["USB-C port 1"])),
    StateCase(
        name: "Charging, gentle", detail: "A thin leg: few watts into the battery.",
        sample: sample(
            percent: 92, plugged: true, adapter: 60, input: 16.9, load: 9.4, battery: 7.5,
            minutes: 38, ports: ["USB-C port 1"])),
    StateCase(
        name: "Full, on charger", detail: "Charger feeds the system. The battery holds.",
        sample: sample(
            percent: 100, plugged: true, full: true, adapter: 60, input: 18.2, load: 18.2,
            battery: 0, ports: ["USB-C port 1"])),
    StateCase(
        name: "Charger too weak", detail: "A net loss: only the battery legs light.",
        sample: sample(
            percent: 63, plugged: true, adapter: 20, input: 18.7, load: 104.6, battery: -85.9,
            minutes: 41, ports: ["MagSafe"])),
    StateCase(
        name: "On battery, gentle", detail: "Battery feeds the system, in orange.",
        sample: sample(percent: 41, plugged: false, load: 12.3, battery: -12.3, minutes: 215)),
    StateCase(
        name: "On battery, low and heavy", detail: "A wide leg: many watts out of the battery.",
        sample: sample(percent: 8, plugged: false, load: 48.0, battery: -48.0, minutes: 5)),
    StateCase(
        name: "Powering a device, on battery", detail: "The battery feeds the device and the system.",
        sample: sample(
            percent: 58, plugged: false, load: 21.0, battery: -21.0, minutes: 150,
            out: [PowerSample.PortOut(name: "USB-C port 2", watts: 11.5)])),
    StateCase(
        name: "Charging and powering a device",
        detail: "The charger feeds system, device and battery.",
        sample: sample(
            percent: 99, plugged: true, adapter: 60, input: 31.2, load: 16.0, battery: 15.2,
            minutes: 12, ports: ["USB-C port 1"],
            out: [PowerSample.PortOut(name: "USB-C port 2", watts: 1.1)])),
    StateCase(
        name: "Charger gives nothing, device on",
        detail: "0 W in. The battery feeds system and device.",
        sample: sample(
            percent: 99, plugged: true, adapter: 60, input: 0, load: 7.7, battery: -7.7,
            ports: ["USB-C port 1"],
            out: [PowerSample.PortOut(name: "USB-C port 2", watts: 1.95)])),
    StateCase(
        name: "Charger too weak, device on",
        detail: "A net loss: the battery feeds device and system.",
        sample: sample(
            percent: 99, plugged: true, adapter: 60, input: 7.0, load: 10.2, battery: -3.2,
            ports: ["USB-C port 1"],
            out: [PowerSample.PortOut(name: "USB-C port 2", watts: 1.7)])),
    StateCase(
        name: "Just unplugged", detail: "Old charge watts for a few seconds. Still a drain.",
        sample: sample(percent: 100, plugged: false, full: true, load: 9.8, battery: 6.4)),
    StateCase(
        name: "Unplugged, no reading", detail: "The battery is the only source. Its leg is lit.",
        sample: sample(percent: 80, plugged: false, load: 0, battery: 0)),
    StateCase(
        name: "Long texts", detail: "Two ports in, two ports out, over 100 W.",
        sample: sample(
            percent: 27, plugged: true, adapter: 140, input: 138.4, load: 36.1, battery: 102.3,
            minutes: 48, ports: ["MagSafe", "USB-C port 2"],
            out: [
                PowerSample.PortOut(name: "USB-C port 1", watts: 11.5),
                PowerSample.PortOut(name: "USB-C port 3", watts: 4.4),
            ])),
    StateCase(
        name: "Charger with no port name", detail: "The adapter rating stands in for the port.",
        sample: sample(
            percent: 52, plugged: true, adapter: 96, input: 71.0, load: 14.8, battery: 56.2,
            minutes: 55,
            out: [PowerSample.PortOut(name: "USB-C port 2", watts: 7.5)])),
    StateCase(name: "No battery", detail: "A desktop Mac.", sample: nil),
]

/// A widget-shaped tile. macOS draws the real surface, so this fill is only a stand-in.
private struct Tile: View {
    @Environment(\.colorScheme) private var scheme
    let family: WidgetFamily
    let sample: PowerSample?
    var frame = 0.0

    var body: some View {
        PowerContent(sample: sample, family: family, frame: frame)
            .padding(16)
            .frame(width: family == .systemMedium ? 344 : 164, height: 164)
            // A stand-in for the desktop under the widget, then the widget's own background.
            .background {
                ZStack {
                    scheme == .dark ? Color(white: 0.11) : Color(white: 0.97)
                    NetBackground(sample: sample)
                }
            }
            .clipShape(RoundedRectangle(cornerRadius: 22, style: .continuous))
    }
}

/// One widget with its state written above it.
private struct Labelled: View {
    let item: StateCase
    let family: WidgetFamily

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            VStack(alignment: .leading, spacing: 2) {
                Text(item.name).font(.system(size: 15, weight: .semibold))
                // The wide tile has room for the explanation. The narrow one names its size.
                Text(family == .systemMedium ? item.detail : "Small size")
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
            }
            .lineLimit(1)
            Tile(family: family, sample: item.sample)
        }
    }
}

private struct Sheet: View {
    @Environment(\.colorScheme) private var scheme

    var body: some View {
        VStack(alignment: .leading, spacing: 28) {
            Text("Power Flow widget: every state")
                .font(.system(size: 22, weight: .semibold))
            ForEach(cases, id: \.name) { item in
                HStack(alignment: .bottom, spacing: 24) {
                    Labelled(item: item, family: .systemMedium)
                    Labelled(item: item, family: .systemSmall)
                }
            }
            // Four steps of the flow animation. macOS moves the pulses from one to the next.
            VStack(alignment: .leading, spacing: 8) {
                Text("Flow animation").font(.system(size: 15, weight: .semibold))
                Text("Four steps, two seconds apart. The pulses run from the source.")
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
                HStack(spacing: 12) {
                    ForEach(0..<4, id: \.self) { step in
                        Tile(family: .systemSmall, sample: cases[0].sample, frame: Double(step))
                            .scaleEffect(0.8)
                            .frame(width: 131, height: 131)
                    }
                }
            }
            Text("Drawn from the widget's own views. macOS supplies the real background.")
                .font(.system(size: 11))
                .foregroundStyle(.secondary)
        }
        .padding(32)
        .foregroundStyle(.primary)
        .background(scheme == .dark ? Color(white: 0.2) : Color(white: 0.86))
    }
}

/// The picture for the README: the widget in three states, on a clear background so it
/// sits on the page of either GitHub theme.
private struct Showcase: View {
    /// A step of the flow animation with the pulses well inside their legs.
    static let frame = 0.6

    private func state(_ name: String) -> PowerSample? {
        guard let item = cases.first(where: { $0.name == name }) else {
            fatalError("no state named \(name)")
        }
        return item.sample
    }

    var body: some View {
        HStack(spacing: 20) {
            Tile(family: .systemMedium, sample: state("Charging, fast"), frame: Self.frame)
            Tile(family: .systemSmall, sample: state("On battery, gentle"), frame: Self.frame)
            Tile(
                family: .systemSmall, sample: state("Powering a device, on battery"),
                frame: Self.frame)
        }
        .padding(8)
    }
}

/// Writes a view as a PNG at twice its size.
@MainActor
private func write<Content: View>(_ view: Content, to file: URL) {
    let renderer = ImageRenderer(content: view)
    renderer.scale = 2
    guard let tiff = renderer.nsImage?.tiffRepresentation,
        let png = NSBitmapImageRep(data: tiff)?.representation(using: .png, properties: [:])
    else { fatalError("could not render \(file.lastPathComponent)") }
    do { try png.write(to: file) } catch { fatalError("\(error)") }
    print("Wrote \(file.path)")
}

@main
enum States {
    @MainActor
    static func main() {
        let folder = CommandLine.arguments.dropFirst().first ?? "."
        let base = URL(fileURLWithPath: folder)
        for (scheme, name) in [(ColorScheme.dark, "dark"), (ColorScheme.light, "light")] {
            write(
                Sheet().environment(\.colorScheme, scheme),
                to: base.appendingPathComponent("states-\(name).png"))
            write(
                Showcase().environment(\.colorScheme, scheme),
                to: base.appendingPathComponent("widget-\(name).png"))
        }
    }
}
