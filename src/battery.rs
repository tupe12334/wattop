//! Battery segment model: fill level plus a pulse that runs while charging or draining.

/// Battery motion, from the battery watts sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Charging,
    Draining,
    Holding,
}

/// What one battery segment shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellKind {
    Filled,
    Empty,
    /// Bright charge pulse, in the empty part.
    ChargePulse,
    /// Dim trail just behind the charge pulse.
    ChargeTrail,
    /// Drain pulse, in the filled part.
    DrainPulse,
}

/// Segments for a panel inner width. Leaves one column for the nub. Max 10.
pub fn segment_count(inner_width: usize) -> usize {
    inner_width.saturating_sub(1).clamp(1, 10)
}

/// Filled segments for a percent. Rounds up, so 1% still shows one segment.
pub fn filled_cells(width: usize, percent: i64) -> usize {
    (percent.clamp(0, 100) as usize * width).div_ceil(100)
}

/// Segment kinds for one frame. The pulse moves one segment per frame and loops.
/// Charging: starts at the fill edge and sweeps right, toward the nub.
/// Draining: starts at the fill edge and sweeps left, toward the start.
pub fn battery_cells(width: usize, percent: i64, motion: Motion, frame: u64) -> Vec<CellKind> {
    let filled = filled_cells(width, percent);
    let mut cells: Vec<CellKind> = (0..width)
        .map(|i| {
            if i < filled {
                CellKind::Filled
            } else {
                CellKind::Empty
            }
        })
        .collect();
    match motion {
        Motion::Charging if filled < width => {
            let span = (width - filled) as u64;
            let head = filled + (frame % span) as usize;
            cells[head] = CellKind::ChargePulse;
            if head > filled {
                cells[head - 1] = CellKind::ChargeTrail;
            }
        }
        Motion::Draining if filled > 0 => {
            let span = filled as u64;
            cells[filled - 1 - (frame % span) as usize] = CellKind::DrainPulse;
        }
        _ => {}
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The single pulse segment, if any: its index and kind.
    fn pulse(cells: &[CellKind]) -> Option<(usize, CellKind)> {
        cells.iter().enumerate().find_map(|(i, c)| {
            matches!(c, CellKind::ChargePulse | CellKind::DrainPulse).then_some((i, *c))
        })
    }

    #[test]
    fn charging_pulse_moves_right_inside_empty_part() {
        let (w, p) = (10, 40); // 4 filled, 6 empty
        let filled = filled_cells(w, p);
        for n in 0..24u64 {
            let (a, kind) = pulse(&battery_cells(w, p, Motion::Charging, n)).unwrap();
            let (b, _) = pulse(&battery_cells(w, p, Motion::Charging, n + 1)).unwrap();
            assert_eq!(kind, CellKind::ChargePulse);
            assert!(a >= filled && a < w, "pulse {a} left the empty part");
            assert!(
                b == a + 1 || (a == w - 1 && b == filled),
                "pulse jumped {a} -> {b}"
            );
        }
    }

    #[test]
    fn charging_trail_sits_behind_pulse() {
        let (w, p) = (10, 40);
        let filled = filled_cells(w, p);
        // Frame 0: pulse on the fill edge, no trail (the cell behind is filled).
        assert_eq!(
            battery_cells(w, p, Motion::Charging, 0)[filled],
            CellKind::ChargePulse
        );
        // Frame 2: pulse two past the edge, trail one behind it.
        let cells = battery_cells(w, p, Motion::Charging, 2);
        assert_eq!(cells[filled + 2], CellKind::ChargePulse);
        assert_eq!(cells[filled + 1], CellKind::ChargeTrail);
    }

    #[test]
    fn draining_pulse_moves_left_inside_filled_part() {
        let (w, p) = (10, 40); // 4 filled, 6 empty
        let filled = filled_cells(w, p);
        for n in 0..24u64 {
            let (a, kind) = pulse(&battery_cells(w, p, Motion::Draining, n)).unwrap();
            let (b, _) = pulse(&battery_cells(w, p, Motion::Draining, n + 1)).unwrap();
            assert_eq!(kind, CellKind::DrainPulse);
            assert!(a < filled, "pulse {a} left the filled part");
            assert!(
                b + 1 == a || (a == 0 && b == filled - 1),
                "pulse jumped {a} -> {b}"
            );
        }
    }

    #[test]
    fn segments_fit_inner_width_with_nub() {
        // Drawn row = segments + nub column.
        for inner in [8, 12, 24, 40] {
            let drawn = segment_count(inner) + 1;
            assert!(drawn <= inner, "row {drawn} wider than inner {inner}");
        }
        assert_eq!(segment_count(40), 10);
    }

    #[test]
    fn holding_has_no_pulse() {
        for n in 0..10 {
            assert!(pulse(&battery_cells(10, 40, Motion::Holding, n)).is_none());
        }
    }

    #[test]
    fn edge_levels_do_not_panic() {
        for motion in [Motion::Charging, Motion::Draining, Motion::Holding] {
            for percent in [-5, 0, 1, 100, 150] {
                for width in [0, 1, 10] {
                    for frame in [0, 1, 999] {
                        assert_eq!(battery_cells(width, percent, motion, frame).len(), width);
                    }
                }
            }
        }
        // Full battery: nothing to sweep into. Empty battery: nothing to drain.
        assert!(pulse(&battery_cells(10, 100, Motion::Charging, 3)).is_none());
        assert!(pulse(&battery_cells(10, 0, Motion::Draining, 3)).is_none());
        // Empty battery charging: the pulse starts at the first segment.
        assert_eq!(
            pulse(&battery_cells(10, 0, Motion::Charging, 0)),
            Some((0, CellKind::ChargePulse))
        );
    }
}
