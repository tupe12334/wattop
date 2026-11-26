// The ring of the energy loop: its geometry, its legs and the pulses that run on them.

import SwiftUI

/// The ring. Angles are degrees, counted counterclockwise from 3 o'clock.
struct Loop {
    static let port = 150.0
    static let system = 30.0
    static let battery = 270.0

    let centre: CGPoint
    let radius: CGFloat
    /// Distance along the ring that a leg keeps clear around each station.
    let gap: CGFloat

    func point(_ degrees: Double) -> CGPoint {
        let angle = degrees * .pi / 180
        return CGPoint(x: centre.x + radius * cos(angle), y: centre.y - radius * sin(angle))
    }

    /// The arc of a leg between its two stations, as a path. It stops short of each station
    /// by the gap and by half its own width, so that its round ends stay clear of the rims.
    func arc(_ leg: Leg) -> Path {
        let short = Double((gap + leg.lineWidth / 2) / radius) * 180 / .pi
        let clear = short * (leg.to > leg.from ? 1 : -1)
        let (from, to) = (leg.from + clear, leg.to - clear)
        var path = Path()
        let steps = 40
        for step in 0...steps {
            let p = point(from + (to - from) * Double(step) / Double(steps))
            if step == 0 { path.move(to: p) } else { path.addLine(to: p) }
        }
        return path
    }
}

extension Loop {
    /// Bright pulses spaced evenly round the whole ring. A leg shows the part that crosses it.
    /// A pulse is as wide as the leg at its head and thin at its tail, like a comet.
    /// `forward` is true when the pulses travel counterclockwise.
    func pulses(width: CGFloat, forward: Bool) -> Path {
        var all = Path()
        let steps = 14
        for start in stride(from: 0.0, to: 360.0, by: Flow.spacing) {
            // The two edges of the pulse, from its low angle to its high angle.
            var outer: [CGPoint] = []
            var inner: [CGPoint] = []
            for step in 0...steps {
                let share = Double(step) / Double(steps)
                // 0 at the tail, 1 at the head.
                let along = forward ? share : 1 - share
                let half = width * (Flow.tail + (1 - Flow.tail) * along) / 2
                let angle = (start + Flow.pulse * share) * .pi / 180
                let (x, y) = (cos(angle), -sin(angle))
                outer.append(CGPoint(x: centre.x + (radius + half) * x, y: centre.y + (radius + half) * y))
                inner.append(CGPoint(x: centre.x + (radius - half) * x, y: centre.y + (radius - half) * y))
            }
            var body = Path()
            body.addLines(outer + inner.reversed())
            body.closeSubpath()
            // A round head, joined to the body so that the fill has no hole.
            let head = point(start + (forward ? Flow.pulse : 0))
            let cap = Path(
                ellipseIn: CGRect(
                    x: head.x - width / 2, y: head.y - width / 2, width: width, height: width))
            all.addPath(Path(body.cgPath.union(cap.cgPath)))
        }
        return all
    }

    /// The colour of the pulses: full at each head, then fading out to nothing at the end
    /// of each tail, just before the head behind it. Gradient angles run clockwise, so a
    /// pulse lies at the end of each spacing.
    func pulseFill(_ color: Color, forward: Bool, hub: UnitPoint) -> AngularGradient {
        let space = Flow.spacing - Flow.pulse
        var stops: [Gradient.Stop] = []
        for base in stride(from: 0.0, to: 360.0, by: Flow.spacing) {
            let at = { (degrees: Double) in CGFloat((base + degrees) / 360) }
            if forward {
                // Head at the start of the pulse, tail at its end.
                stops += [
                    .init(color: .clear, location: at(0)),
                    .init(color: color, location: at(space / 2)),
                    .init(color: color, location: at(space + Flow.pulse * Flow.solid)),
                    .init(color: .clear, location: at(Flow.spacing)),
                ]
            } else {
                // Tail at the start of the pulse, head at its end.
                stops += [
                    .init(color: color, location: at(0)),
                    .init(color: .clear, location: at(space)),
                    .init(color: color, location: at(space + Flow.pulse * (1 - Flow.solid))),
                    .init(color: color, location: at(Flow.spacing)),
                ]
            }
        }
        return AngularGradient(gradient: Gradient(stops: stops), center: hub)
    }
}

/// One leg of the ring. Power moves from `from` to `to`. Nil colour is an idle leg.
struct Leg {
    let from: Double
    let to: Double
    let color: Color?
    /// Watts on the leg. The sign is ignored.
    let watts: Double
    /// Degrees that the pulses of this leg run ahead of the others at the start of a run.
    var phase = 0.0
    /// Speed of the pulses of this leg, as a share of `Flow.turn`.
    var pace = 1.0

    /// Width of the neutral line: an idle leg, and the spaces between pulses on a live one.
    static let track: CGFloat = 2.5

    /// Width of a pulse at its head. It grows in proportion to the watts, from a thin line
    /// that stays visible at 0 W to the widest line at 100 W and more.
    var width: CGFloat { 3 + 3.5 * min(abs(watts), 100) / 100 }

    /// The widest thing the leg draws.
    var lineWidth: CGFloat { color == nil ? Self.track : max(width, Self.track) }
}

struct LegView: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    let loop: Loop
    let leg: Leg
    let frame: Double
    let batch: Int
    /// The centre of the ring, as a share of the view. The pulses turn about it.
    let hub: UnitPoint

    var body: some View {
        if let color = leg.color {
            // A leg needs no arrow: the pulses run the way the power moves.
            let line = StrokeStyle(lineWidth: leg.lineWidth, lineCap: .round)
            let forward = leg.to > leg.from
            ZStack {
                if reduceMotion {
                    loop.arc(leg).stroke(color, style: line)
                } else {
                    // A neutral line, and coloured pulses that run along it. Only the pulses
                    // carry colour. Each one has a long tail that thins and fades out.
                    loop.arc(leg).stroke(idle, style: StrokeStyle(lineWidth: Leg.track, lineCap: .round))
                    loop.pulses(width: leg.width, forward: forward)
                        .fill(loop.pulseFill(color, forward: forward, hub: hub))
                        // The accent of a tinted widget style goes to the power that moves.
                        .widgetAccentable()
                        .rotationEffect(.degrees(turn), anchor: hub)
                        .mask(loop.arc(leg).stroke(style: line))
                        .animation(.linear(duration: Flow.step), value: frame)
                        // A new run is a new view. macOS then fades to it. Without this, a
                        // widget that was hidden turns the pulses through every step it
                        // missed, and the turn grows with the time it was hidden.
                        .id(batch)
                        .transition(.opacity)
                }
            }
        } else {
            loop.arc(leg).stroke(idle, style: StrokeStyle(lineWidth: Leg.track, lineCap: .round))
        }
    }

    /// How far the pulses have turned. A leg that runs counterclockwise turns them that way.
    private var turn: Double {
        (frame * Flow.turn * leg.pace + leg.phase) * (leg.to > leg.from ? -1 : 1)
    }
}
