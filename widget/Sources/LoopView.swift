// The energy loop: three stations on a ring, with the legs that light between them.

import AppKit
import CoreText
import SwiftUI

/// Rims and idle legs: the text colour, faded, so they follow light and dark.
let rim = Color.primary.opacity(0.4)
let idle = Color.primary.opacity(0.16)

// MARK: - The loop

/// The energy loop. CHARGER, SYSTEM and BATTERY are three stations on a ring. A leg of the
/// ring lights up when power moves along it. Its width follows the watts.
struct LoopView: View {
    let sample: PowerSample
    /// Show the time left under the charge level. The medium widget has it in its rows.
    let showsTime: Bool
    let frame: Double
    let batch: Int

    private static let station: CGFloat = 26

    var body: some View {
        GeometryReader { geo in
            let size = geo.size
            // The ring and the battery station under it fill the height.
            let radius = (size.height - Self.station - 2) / 2
            let centre = CGPoint(x: size.width / 2, y: size.height - Self.station / 2 - 1 - radius)
            // Legs stop short of the stations, so a station needs no opaque disc to hide them.
            let loop = Loop(centre: centre, radius: radius, gap: Self.station / 2 + 3)
            ZStack {
                ForEach(Array(legs.enumerated()), id: \.offset) { _, leg in
                    LegView(
                        loop: loop, leg: leg, frame: frame, batch: batch,
                        hub: UnitPoint(x: centre.x / size.width, y: centre.y / size.height))
                }
                // The inked pixels sit on the ring's centre, not the text boxes.
                middle
                    .alignmentGuide(HorizontalAlignment.center) { InkLine.inkMidX(middleLines, $0) }
                    .alignmentGuide(VerticalAlignment.center) { InkLine.inkMidY(middleLines, $0) }
                    .frame(width: radius * 1.5, height: radius * 1.5)
                    .position(centre)
                Station(symbol: PowerSample.portSymbol)
                    .opacity(sample.portActive ? 1 : 0.4)
                    .position(loop.point(Loop.port))
                Station(symbol: "laptopcomputer")
                    .position(loop.point(Loop.system))
                Station(symbol: sample.levelSymbol)
                    .position(loop.point(Loop.battery))
            }
        }
    }

    /// The three legs of the ring. Each one starts its pulses a third of a spacing after the
    /// one before, and each one has its own pace. So two lit legs drift: one leads, then they
    /// run in step, then the other leads. With one pace they would move like one mirrored
    /// picture.
    private var legs: [Leg] {
        [portToSystem, portAndBattery, batteryToSystem].enumerated().map { index, leg in
            var leg = leg
            leg.phase = Flow.spacing * Double(index) / 3
            leg.pace = Flow.paces[index]
            return leg
        }
    }

    /// A charger feeds the system. On a net loss the leg stays dark, even when the charger
    /// covers part of the load: green beside orange reads as two answers to one question.
    private var portToSystem: Leg {
        // The share of the input that the system takes. The rest goes to the battery.
        let watts = min(sample.inputW, sample.loadW)
        let lit = sample.supplies && !sample.drains
        return Leg(from: Loop.port, to: Loop.system, color: lit ? .green : nil, watts: watts)
    }

    /// The leg between the power port and the battery. It is the one leg with two directions:
    /// a charger fills the battery, or the battery feeds a device while the battery drains.
    private var portAndBattery: Leg {
        if sample.charges {
            return Leg(from: Loop.port, to: Loop.battery, color: .green, watts: sample.batteryW)
        }
        if sample.feedsDevice {
            // The power for the device comes out of the battery, not out of the system.
            return Leg(
                from: Loop.battery, to: Loop.port, color: .orange, watts: sample.deviceFromBatteryW)
        }
        return Leg(from: Loop.port, to: Loop.battery, color: nil, watts: 0)
    }

    /// The battery feeds the system. Orange marks power that leaves the battery. It is the
    /// drain colour of the terminal dashboard too.
    private var batteryToSystem: Leg {
        // What the battery gives, less the part that goes to a device.
        let watts = max(sample.drainW - sample.deviceFromBatteryW, 0)
        return Leg(
            from: Loop.battery, to: Loop.system + 360, color: sample.drains ? .orange : nil,
            watts: watts)
    }

    /// The lines in the middle of the ring, top to bottom.
    private var middleLines: [InkLine] {
        // No watts here: the legs carry the rate. The level and the time left change slowly.
        var lines = [InkLine(text: sample.level, size: 26, weight: .light)]
        if showsTime && !sample.timeLeft.isEmpty {
            lines.append(InkLine(text: sample.timeLeft, size: 11, weight: .semibold))
        }
        return lines
    }

    private var middle: some View {
        VStack(spacing: 0) {
            ForEach(Array(middleLines.enumerated()), id: \.offset) { index, line in
                Text(line.text)
                    .font(.system(size: line.size, weight: line.weight))
                    .foregroundStyle(index == 0 ? HierarchicalShapeStyle.primary : .secondary)
            }
        }
        .lineLimit(1)
        .minimumScaleFactor(0.6)
    }
}

/// One line of text, measured by the pixels its glyphs ink. A text box is taller than its
/// digits: it keeps room for ascenders and descenders that "99%" does not have, so a
/// centred box draws the digits off centre. The ink is measured from the baseline, which
/// SwiftUI reports exactly; its line box does not follow from the font metrics.
struct InkLine {
    let text: String
    let size: CGFloat
    let weight: Font.Weight

    private var font: NSFont {
        let weights: [Font.Weight: NSFont.Weight] = [
            .light: .light, .regular: .regular, .semibold: .semibold, .bold: .bold,
        ]
        return NSFont.systemFont(ofSize: size, weight: weights[weight] ?? .regular)
    }

    /// The line's advance width and its glyphs' ink. Core Text counts x from the line's
    /// start and y up from the baseline.
    private var measure: (width: CGFloat, ink: CGRect) {
        let line = CTLineCreateWithAttributedString(
            NSAttributedString(string: text, attributes: [.font: font]))
        let width = CTLineGetTypographicBounds(line, nil, nil, nil)
        return (width, CTLineGetBoundsWithOptions(line, .useGlyphPathBounds))
    }

    /// The x of the ink's centre in a VStack of the lines, each centred across its width.
    static func inkMidX(_ lines: [InkLine], _ stack: ViewDimensions) -> CGFloat {
        let ink = lines.map(\.measure).reduce(CGRect.null) { all, line in
            all.union(line.ink.offsetBy(dx: (stack.width - line.width) / 2, dy: 0))
        }
        return ink.isNull ? stack.width / 2 : ink.midX
    }

    /// The y of the ink's centre in a VStack of the lines: from the top of the first line's
    /// ink to the bottom of the last line's ink, both measured from their baselines.
    static func inkMidY(_ lines: [InkLine], _ stack: ViewDimensions) -> CGFloat {
        guard let first = lines.first?.measure.ink, let last = lines.last?.measure.ink else {
            return stack.height / 2
        }
        let top = stack[VerticalAlignment.firstTextBaseline] - first.maxY
        let bottom = stack[VerticalAlignment.lastTextBaseline] - last.minY
        return (top + bottom) / 2
    }
}

/// A station on the ring: a rim and a symbol. It has no state colour.
struct Station: View {
    let symbol: String

    var body: some View {
        ZStack {
            Circle().strokeBorder(rim, lineWidth: 1.5)
            Image(systemName: symbol)
                .font(.system(size: 11, weight: .semibold))
        }
        .frame(width: 26, height: 26)
    }
}
