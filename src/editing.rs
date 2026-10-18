//! Editing a class on the bar itself: the breaks and the warning and final zone boundaries can be
//! dragged. Pure rules (no Win32): which handle is under the pointer, and what a drag does to the
//! class, snapped to whole minutes and clamped so the result is always a sensible class.

use crate::schedule::{Break, Class, MINUTE, Secs, Warnings, format_clock};

/// A block between the start, the breaks and the end of the class needs at least this much.
pub const MIN_BLOCK: Secs = 5 * MINUTE;
pub const MIN_BREAK: Secs = MINUTE;
/// The final zone is at least this long, and warning starts at least this much before final.
pub const MIN_RED: Secs = MINUTE;
pub const MIN_ZONE_GAP: Secs = MINUTE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneEdge {
    Warning,
    Final,
}

/// Something on the bar that can be dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    /// The start of break `n`: moves it, so the block before it gets longer or shorter.
    BreakStart(usize),
    /// The end of break `n`.
    BreakEnd(usize),
    /// Break `n` as a whole: it keeps its length.
    BreakBody(usize),
    /// Where the warning or final zone of a block begins.
    Zone { block: usize, edge: ZoneEdge },
}

/// Nearest whole minute.
fn snap(secs: i64) -> i64 {
    let m = i64::from(MINUTE);
    (secs + m / 2).div_euclid(m) * m
}

/// Where the zone boundaries and the break lie, in seconds from the class start.
fn zone_at(class: &Class, block: usize, edge: ZoneEdge) -> Secs {
    let b = class.block(block);
    let w = class.warnings[block];
    let before_end = match edge {
        ZoneEdge::Warning => w.warning,
        ZoneEdge::Final => w.final_,
    };
    b.end - before_end.min(b.len())
}

/// What to tell the teacher while a handle is dragged: the clock time it stands for and where on
/// the bar to show it.
#[derive(Clone, Debug, PartialEq)]
pub struct HandleLabel {
    /// `09:35` for a zone boundary (when the zone starts), `09:45–09:55` for the break.
    pub text: String,
    /// Where the label belongs, as a share of the bar width.
    pub position: f32,
}

/// The label for `handle` of `class` as it is now; `None` for a handle of a break the class
/// does not have.
pub fn describe(class: &Class, handle: Handle) -> Option<HandleLabel> {
    let total = class.duration().max(1) as f32;
    let at = |offset: Secs| format_clock(class.start + offset);
    match handle {
        Handle::Zone { block, edge } => {
            if block >= class.block_count() {
                return None;
            }
            let offset = zone_at(class, block, edge);
            Some(HandleLabel {
                text: at(offset),
                position: offset as f32 / total,
            })
        }
        Handle::BreakStart(n) | Handle::BreakEnd(n) | Handle::BreakBody(n) => {
            let b = *class.breaks.get(n)?;
            let position = match handle {
                Handle::BreakStart(_) => b.start as f32,
                Handle::BreakEnd(_) => b.end() as f32,
                _ => (b.start + b.end()) as f32 / 2.0,
            } / total;
            Some(HandleLabel {
                text: format!("{}\u{2013}{}", at(b.start), at(b.end())),
                position,
            })
        }
    }
}

/// The handle under `x` (pixels from the left edge of a bar `width` pixels wide), if any.
/// Boundaries within `grip` pixels count; the nearest one wins, with the break edges ahead of the
/// zone boundaries when equally near. Failing that, the inside of a break is its body.
pub fn handle_at(class: &Class, x: f32, width: f32, grip: f32) -> Option<Handle> {
    if width <= 0.0 || class.duration() == 0 {
        return None;
    }
    let px = |secs: Secs| secs as f32 / class.duration() as f32 * width;
    let mut candidates: Vec<(f32, Handle)> = Vec::with_capacity(2 * class.block_count() * 2);
    for (n, b) in class.breaks.iter().enumerate() {
        candidates.push((px(b.start), Handle::BreakStart(n)));
        candidates.push((px(b.end()), Handle::BreakEnd(n)));
    }
    for block in 0..class.block_count() {
        for edge in [ZoneEdge::Warning, ZoneEdge::Final] {
            candidates.push((
                px(zone_at(class, block, edge)),
                Handle::Zone { block, edge },
            ));
        }
    }
    let nearest = candidates
        .iter()
        .map(|&(at, handle)| ((at - x).abs(), handle))
        .filter(|&(distance, _)| distance <= grip)
        .min_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((_, handle)) = nearest {
        return Some(handle);
    }
    class
        .breaks
        .iter()
        .position(|b| x > px(b.start) && x < px(b.end()))
        .map(Handle::BreakBody)
}

/// `class` after dragging `handle` by `delta` seconds (positive = later). Everything is snapped
/// to whole minutes and kept within the limits; a handle the class does not have changes nothing.
pub fn drag(class: &Class, handle: Handle, delta: i64) -> Class {
    let duration = i64::from(class.duration());
    let (min_block, min_break) = (i64::from(MIN_BLOCK), i64::from(MIN_BREAK));
    let mut out = class.clone();

    match handle {
        Handle::BreakStart(n) | Handle::BreakEnd(n) | Handle::BreakBody(n) => {
            let Some(b) = class.breaks.get(n) else {
                return out;
            };
            let (start, end) = (i64::from(b.start), i64::from(b.end()));
            let len = end - start;
            // A break stays between its neighbours, each block keeping its minimum length.
            let earliest = n
                .checked_sub(1)
                .map_or(0, |p| i64::from(class.breaks[p].end()))
                + min_block;
            let latest = class
                .breaks
                .get(n + 1)
                .map_or(duration, |next| i64::from(next.start))
                - min_block;
            let (new_start, new_end) = match handle {
                Handle::BreakStart(_) => {
                    let s = snap(start + delta).clamp(earliest, (end - min_break).max(earliest));
                    (s, end)
                }
                Handle::BreakEnd(_) => {
                    let e =
                        snap(end + delta).clamp(start + min_break, latest.max(start + min_break));
                    (start, e)
                }
                _ => {
                    let s = snap(start + delta).clamp(earliest, (latest - len).max(earliest));
                    (s, s + len)
                }
            };
            out.breaks[n] = Break {
                start: new_start as Secs,
                len: (new_end - new_start) as Secs,
            };
        }
        Handle::Zone { block, edge } => {
            if block >= class.block_count() {
                return out;
            }
            let len = i64::from(class.block(block).len());
            let w = class.warnings[block];
            // The zones as they are drawn: never longer than the block itself.
            let warning = i64::from(w.warning).min(len);
            let final_ = i64::from(w.final_).min(len);
            let (gap, min_final) = (i64::from(MIN_ZONE_GAP), i64::from(MIN_RED));
            // Dragging later means less time before the end of the block.
            let (new_warning, new_final) = match edge {
                ZoneEdge::Warning => {
                    let lo = (final_ + gap).min(len);
                    (snap(warning - delta).clamp(lo, len), final_)
                }
                ZoneEdge::Final => {
                    let hi = (warning - gap).max(min_final);
                    (warning, snap(final_ - delta).clamp(min_final, hi))
                }
            };
            out.warnings[block] = Warnings {
                warning: new_warning as Secs,
                final_: new_final.min(new_warning) as Secs,
            };
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::{DEFAULT_BREAK_LEN, hm};

    /// 09:00-10:40: blocks of 45 minutes around a 10 minute break (45..55 min).
    fn double() -> Class {
        Class::new(hm(9, 0), hm(10, 40), DEFAULT_BREAK_LEN).unwrap()
    }

    /// 09:00-09:50: one block of 50 minutes.
    fn single() -> Class {
        Class::new(hm(9, 0), hm(9, 50), DEFAULT_BREAK_LEN).unwrap()
    }

    const fn mins(m: i64) -> i64 {
        m * 60
    }

    // ---- handle_at ----

    /// A bar of 1000 px for the 100 minute class: 10 px per minute.
    const W: f32 = 1000.0;
    const GRIP: f32 = 4.0;

    fn at(class: &Class, x: f32) -> Option<Handle> {
        handle_at(class, x, W, GRIP)
    }

    #[test]
    fn break_edges_and_body_are_found() {
        let c = double();
        assert_eq!(at(&c, 450.0), Some(Handle::BreakStart(0)));
        assert_eq!(
            at(&c, 453.0),
            Some(Handle::BreakStart(0)),
            "within the grip"
        );
        assert_eq!(at(&c, 550.0), Some(Handle::BreakEnd(0)));
        assert_eq!(at(&c, 500.0), Some(Handle::BreakBody(0)));
        assert_eq!(at(&c, 460.0), Some(Handle::BreakBody(0)));
        assert_eq!(
            at(&c, 545.0),
            Some(Handle::BreakBody(0)),
            "beyond the end grip is body"
        );
    }

    #[test]
    fn every_zone_boundary_has_a_handle() {
        let c = double();
        // Block 1: warning at 35 min, final at 40 min. Block 2: warning at 90, final at 95.
        let zone = |block, edge| Some(Handle::Zone { block, edge });
        assert_eq!(at(&c, 350.0), zone(0, ZoneEdge::Warning));
        assert_eq!(at(&c, 400.0), zone(0, ZoneEdge::Final));
        assert_eq!(at(&c, 900.0), zone(1, ZoneEdge::Warning));
        assert_eq!(at(&c, 950.0), zone(1, ZoneEdge::Final));
        assert_eq!(at(&c, 902.0), zone(1, ZoneEdge::Warning));
    }

    #[test]
    fn a_single_block_class_has_only_zone_handles() {
        let c = single(); // 50 minutes: warning at 40, final at 45
        let w = 500.0;
        assert_eq!(
            handle_at(&c, 400.0, w, GRIP),
            Some(Handle::Zone {
                block: 0,
                edge: ZoneEdge::Warning
            })
        );
        assert_eq!(
            handle_at(&c, 450.0, w, GRIP),
            Some(Handle::Zone {
                block: 0,
                edge: ZoneEdge::Final
            })
        );
        assert_eq!(handle_at(&c, 100.0, w, GRIP), None);
    }

    #[test]
    fn empty_places_have_no_handle() {
        let c = double();
        for x in [0.0, 100.0, 200.0, 300.0, 600.0, 700.0, 800.0, 999.0] {
            assert_eq!(at(&c, x), None, "x = {x}");
        }
        assert_eq!(handle_at(&c, 10.0, 0.0, GRIP), None, "no width");
    }

    #[test]
    fn the_nearest_handle_wins_when_two_are_in_reach() {
        // Warning at 6 minutes before the block end and final 1 minute before it, in a bar where a
        // minute is only 6 px: the pointer picks the closer boundary.
        let mut c = double();
        c.warnings[0] = Warnings {
            warning: 2 * 60,
            final_: 60,
        };
        let w = 600.0; // 6 px per minute
        let orange_x = (45.0 - 2.0) * 6.0;
        let red_x = (45.0 - 1.0) * 6.0;
        assert_eq!(
            handle_at(&c, orange_x + 1.0, w, 5.0),
            Some(Handle::Zone {
                block: 0,
                edge: ZoneEdge::Warning
            })
        );
        assert_eq!(
            handle_at(&c, red_x - 1.0, w, 5.0),
            Some(Handle::Zone {
                block: 0,
                edge: ZoneEdge::Final
            })
        );
    }

    #[test]
    fn break_edges_win_over_zone_boundaries_at_the_same_spot() {
        // Final zone of 5 minutes ends exactly where a break could start: a block-1 final marker
        // sitting on the break start.
        let mut c = double();
        c.warnings[0] = Warnings {
            warning: 10 * 60,
            final_: 0,
        };
        // red_at == break start (45 min).
        assert_eq!(at(&c, 450.0), Some(Handle::BreakStart(0)));
    }

    #[test]
    fn a_class_without_a_break_has_no_break_handles() {
        let c = double().with_breaks(Vec::new()).unwrap();
        for x in [450.0, 500.0, 550.0] {
            assert!(!matches!(
                at(&c, x),
                Some(Handle::BreakStart(_) | Handle::BreakEnd(_) | Handle::BreakBody(_))
            ));
        }
    }

    // ---- describing a handle ----

    #[test]
    fn a_zone_boundary_is_described_by_the_time_the_zone_starts() {
        let c = double(); // 09:00-10:40, blocks 09:00-09:45 and 09:55-10:40
        let label = |block, edge| describe(&c, Handle::Zone { block, edge }).unwrap();
        let warning1 = label(0, ZoneEdge::Warning);
        assert_eq!(warning1.text, "09:35");
        assert!(
            (warning1.position - 0.35).abs() < 1e-4,
            "{}",
            warning1.position
        );
        assert_eq!(label(0, ZoneEdge::Final).text, "09:40");
        assert_eq!(label(1, ZoneEdge::Warning).text, "10:30");
        let final2 = label(1, ZoneEdge::Final);
        assert_eq!(final2.text, "10:35");
        assert!((final2.position - 0.95).abs() < 1e-4);
    }

    #[test]
    fn the_description_follows_the_dragged_class() {
        let moved = drag(
            &double(),
            Handle::Zone {
                block: 0,
                edge: ZoneEdge::Warning,
            },
            mins(3),
        );
        assert_eq!(
            describe(
                &moved,
                Handle::Zone {
                    block: 0,
                    edge: ZoneEdge::Warning
                }
            )
            .unwrap()
            .text,
            "09:38"
        );
    }

    #[test]
    fn a_zone_longer_than_its_block_is_described_where_it_is_drawn() {
        let mut c = double();
        c.warnings[0] = Warnings {
            warning: 60 * 60,
            final_: 5 * 60,
        };
        let label = describe(
            &c,
            Handle::Zone {
                block: 0,
                edge: ZoneEdge::Warning,
            },
        )
        .unwrap();
        assert_eq!(label.text, "09:00", "it starts at the start of its block");
    }

    #[test]
    fn the_break_is_described_by_its_start_and_end() {
        let c = double(); // break 09:45-09:55
        for (handle, position) in [
            (Handle::BreakStart(0), 0.45),
            (Handle::BreakEnd(0), 0.55),
            (Handle::BreakBody(0), 0.50),
        ] {
            let label = describe(&c, handle).unwrap();
            assert_eq!(label.text, "09:45\u{2013}09:55", "{handle:?}");
            assert!(
                (label.position - position).abs() < 1e-4,
                "{handle:?}: {}",
                label.position
            );
        }
        let moved = drag(&c, Handle::BreakBody(0), mins(10));
        assert_eq!(
            describe(&moved, Handle::BreakBody(0)).unwrap().text,
            "09:55\u{2013}10:05"
        );
    }

    #[test]
    fn nothing_is_described_for_a_handle_the_class_does_not_have() {
        let c = single();
        assert_eq!(describe(&c, Handle::BreakStart(0)), None);
        assert_eq!(describe(&c, Handle::BreakBody(0)), None);
        assert_eq!(
            describe(
                &c,
                Handle::Zone {
                    block: 1,
                    edge: ZoneEdge::Final
                }
            ),
            None
        );
        assert!(
            describe(
                &c,
                Handle::Zone {
                    block: 0,
                    edge: ZoneEdge::Final
                }
            )
            .is_some()
        );
    }

    // ---- dragging the break ----

    fn brk(c: &Class) -> (i64, i64) {
        let b = c.breaks[0];
        (i64::from(b.start), i64::from(b.end()))
    }

    #[test]
    fn dragging_the_break_body_moves_it_and_keeps_its_length() {
        let c = drag(&double(), Handle::BreakBody(0), mins(10));
        assert_eq!(brk(&c), (mins(55), mins(65)));
        let c = drag(&double(), Handle::BreakBody(0), -mins(20));
        assert_eq!(brk(&c), (mins(25), mins(35)));
    }

    #[test]
    fn dragging_the_break_start_changes_its_length() {
        let c = drag(&double(), Handle::BreakStart(0), -mins(5));
        assert_eq!(
            brk(&c),
            (mins(40), mins(55)),
            "starts earlier, ends where it did"
        );
        let c = drag(&double(), Handle::BreakStart(0), mins(3));
        assert_eq!(brk(&c), (mins(48), mins(55)));
    }

    #[test]
    fn dragging_the_break_end_changes_its_length() {
        let c = drag(&double(), Handle::BreakEnd(0), mins(7));
        assert_eq!(brk(&c), (mins(45), mins(62)));
        let c = drag(&double(), Handle::BreakEnd(0), -mins(4));
        assert_eq!(brk(&c), (mins(45), mins(51)));
    }

    #[test]
    fn break_drags_snap_to_whole_minutes() {
        let c = drag(&double(), Handle::BreakBody(0), 94); // 1.57 minutes -> 2
        assert_eq!(brk(&c), (mins(47), mins(57)));
        let c = drag(&double(), Handle::BreakBody(0), 29); // under half a minute -> 0
        assert_eq!(brk(&c), (mins(45), mins(55)));
        let c = drag(&double(), Handle::BreakBody(0), -31); // -0.52 -> -1
        assert_eq!(brk(&c), (mins(44), mins(54)));
    }

    #[test]
    fn blocks_keep_at_least_five_minutes() {
        // Far to the left: the first block stays 5 minutes; the break keeps its 10.
        let c = drag(&double(), Handle::BreakBody(0), -mins(500));
        assert_eq!(brk(&c), (mins(5), mins(15)));
        // Far to the right: the second block stays 5 minutes.
        let c = drag(&double(), Handle::BreakBody(0), mins(500));
        assert_eq!(brk(&c), (mins(85), mins(95)));
        // Start edge dragged to the very start.
        let c = drag(&double(), Handle::BreakStart(0), -mins(500));
        assert_eq!(brk(&c), (mins(5), mins(55)));
        // End edge dragged to the very end.
        let c = drag(&double(), Handle::BreakEnd(0), mins(500));
        assert_eq!(brk(&c), (mins(45), mins(95)));
    }

    #[test]
    fn the_break_keeps_at_least_one_minute() {
        let c = drag(&double(), Handle::BreakStart(0), mins(500));
        assert_eq!(brk(&c), (mins(54), mins(55)));
        let c = drag(&double(), Handle::BreakEnd(0), -mins(500));
        assert_eq!(brk(&c), (mins(45), mins(46)));
    }

    #[test]
    fn a_dragged_break_always_gives_a_valid_class() {
        for handle in [
            Handle::BreakStart(0),
            Handle::BreakEnd(0),
            Handle::BreakBody(0),
        ] {
            for delta in [
                -mins(1000),
                -mins(33),
                -61,
                -1,
                0,
                1,
                61,
                mins(33),
                mins(1000),
            ] {
                let c = drag(&double(), handle, delta);
                assert!(c.validate().is_ok(), "{handle:?} {delta}");
                let (s, e) = brk(&c);
                assert!(
                    s >= mins(5) && e <= mins(95) && e - s >= 60,
                    "{handle:?} {delta}: {s} {e}"
                );
            }
        }
    }

    #[test]
    fn break_handles_do_nothing_without_a_break() {
        let c = single();
        for handle in [
            Handle::BreakStart(0),
            Handle::BreakEnd(0),
            Handle::BreakBody(0),
        ] {
            assert_eq!(drag(&c, handle, mins(5)), c);
        }
    }

    // ---- dragging zone boundaries ----

    fn zones(c: &Class, block: usize) -> (i64, i64) {
        let w = c.warnings[block];
        (i64::from(w.warning), i64::from(w.final_))
    }

    fn warning(block: usize) -> Handle {
        Handle::Zone {
            block,
            edge: ZoneEdge::Warning,
        }
    }

    fn final_(block: usize) -> Handle {
        Handle::Zone {
            block,
            edge: ZoneEdge::Final,
        }
    }

    #[test]
    fn dragging_a_zone_boundary_later_shortens_the_zone() {
        // Warning starts 10 minutes before the end; 3 minutes later leaves 7.
        let c = drag(&double(), warning(0), mins(3));
        assert_eq!(zones(&c, 0), (mins(7), mins(5)));
        // Earlier lengthens it.
        let c = drag(&double(), warning(0), -mins(4));
        assert_eq!(zones(&c, 0), (mins(14), mins(5)));
        let c = drag(&double(), final_(0), -mins(2));
        assert_eq!(zones(&c, 0), (mins(10), mins(7)));
    }

    #[test]
    fn each_block_has_its_own_zones() {
        let c = drag(&double(), final_(1), mins(2));
        assert_eq!(zones(&c, 1), (mins(10), mins(3)));
        assert_eq!(
            zones(&c, 0),
            (mins(10), mins(5)),
            "the other block is untouched"
        );
    }

    #[test]
    fn orange_stays_before_red_with_a_minute_between() {
        // Warning dragged past final: stops one minute before it.
        let c = drag(&double(), warning(0), mins(30));
        assert_eq!(zones(&c, 0), (mins(6), mins(5)));
        // Final dragged earlier than warning: stops one minute after it.
        let c = drag(&double(), final_(0), -mins(30));
        assert_eq!(zones(&c, 0), (mins(10), mins(9)));
    }

    #[test]
    fn red_keeps_at_least_a_minute() {
        let c = drag(&double(), final_(0), mins(30));
        assert_eq!(zones(&c, 0), (mins(10), mins(1)));
    }

    #[test]
    fn zones_stay_inside_their_block_so_they_never_reach_the_break() {
        // Warning dragged far to the left stops at the start of the block (45 minutes long).
        let c = drag(&double(), warning(0), -mins(500));
        assert_eq!(zones(&c, 0), (mins(45), mins(5)));
        // The second block is also 45 minutes, so it cannot reach into the break either.
        let c = drag(&double(), warning(1), -mins(500));
        assert_eq!(zones(&c, 1), (mins(45), mins(5)));
        // Final follows the same limit through warning.
        let c = drag(&double(), final_(1), -mins(500));
        assert_eq!(zones(&c, 1), (mins(10), mins(9)));
    }

    #[test]
    fn zone_drags_snap_to_whole_minutes() {
        let c = drag(&double(), warning(0), 100); // 1.67 min -> 2
        assert_eq!(zones(&c, 0), (mins(8), mins(5)));
        let c = drag(&double(), warning(0), 20); // less than half a minute -> no change
        assert_eq!(zones(&c, 0), (mins(10), mins(5)));
    }

    #[test]
    fn a_zone_longer_than_its_block_is_dragged_from_where_it_is_drawn() {
        // Warning is configured as 60 minutes but a 45 minute block only shows 45.
        let mut c = double();
        c.warnings[0] = Warnings {
            warning: 60 * 60,
            final_: 5 * 60,
        };
        let moved = drag(&c, warning(0), mins(5));
        assert_eq!(zones(&moved, 0), (mins(40), mins(5)));
    }

    #[test]
    fn a_dragged_zone_always_gives_a_valid_class() {
        let handles = [warning(0), final_(0), warning(1), final_(1)];
        for handle in handles {
            for delta in [
                -mins(1000),
                -mins(40),
                -61,
                -1,
                0,
                1,
                61,
                mins(40),
                mins(1000),
            ] {
                let c = drag(&double(), handle, delta);
                assert!(c.validate().is_ok());
                for block in 0..2 {
                    let (o, r) = zones(&c, block);
                    assert!(r <= o, "{handle:?} {delta}: final {r} warning {o}");
                    assert!(r >= 60, "{handle:?} {delta}");
                    assert!(o <= mins(45), "{handle:?} {delta}");
                }
            }
        }
    }

    #[test]
    fn zone_handles_for_a_missing_block_do_nothing() {
        let c = single();
        assert_eq!(drag(&c, warning(1), mins(5)), c);
        assert_eq!(drag(&c, final_(1), mins(5)), c);
    }

    #[test]
    fn dragging_a_zone_does_not_touch_the_break() {
        let c = drag(&double(), warning(0), mins(2));
        assert_eq!(c.breaks, double().breaks);
        let c = drag(&double(), Handle::BreakBody(0), mins(2));
        assert_eq!(c.warnings, double().warnings);
    }

    #[test]
    fn moving_the_break_changes_which_block_a_boundary_belongs_to() {
        // A 20 minute first block (break at 20..30) with final 5 and warning 10 before its end.
        let c = drag(&double(), Handle::BreakBody(0), -mins(25));
        assert_eq!(brk(&c), (mins(20), mins(30)));
        assert_eq!(c.block(0).len(), mins(20) as u32);
        // Its zones are still valid and inside the shorter block.
        let (o, r) = zones(&c, 0);
        assert!(o <= mins(20) && r <= o);
        // A zone drag on the shortened block uses the new length.
        let c = drag(&c, warning(0), -mins(500));
        assert_eq!(zones(&c, 0).0, mins(20));
    }

    // ---- several breaks ----

    fn brk_at(start_min: i64, len_min: i64) -> Break {
        Break {
            start: (start_min * 60) as Secs,
            len: (len_min * 60) as Secs,
        }
    }

    /// 09:00-12:00 with breaks at 50..60 and 110..120.
    fn lab() -> Class {
        Class::new(hm(9, 0), hm(12, 0), DEFAULT_BREAK_LEN)
            .unwrap()
            .with_breaks(vec![brk_at(50, 10), brk_at(110, 10)])
            .unwrap()
    }

    #[test]
    fn every_break_has_its_own_handles() {
        // A bar of 1800 px for 180 minutes: 10 px per minute.
        let c = lab();
        let find = |x: f32| handle_at(&c, x, 1800.0, 4.0);
        assert_eq!(find(500.0), Some(Handle::BreakStart(0)));
        assert_eq!(find(600.0), Some(Handle::BreakEnd(0)));
        assert_eq!(find(1100.0), Some(Handle::BreakStart(1)));
        assert_eq!(find(1150.0), Some(Handle::BreakBody(1)));
        assert_eq!(find(550.0), Some(Handle::BreakBody(0)));
    }

    #[test]
    fn describe_names_the_break_by_its_position() {
        let c = lab();
        let label = describe(&c, Handle::BreakBody(1)).unwrap();
        assert_eq!(label.text, "10:50\u{2013}11:00");
        assert!((label.position - 115.0 / 180.0).abs() < 1e-4);
        assert_eq!(describe(&c, Handle::BreakStart(2)), None);
    }

    #[test]
    fn a_break_stops_at_its_neighbours_and_never_crosses_them() {
        let c = lab();
        // Later: the block before the second break keeps its five minutes.
        let moved = drag(&c, Handle::BreakBody(0), mins(200));
        assert_eq!(moved.breaks, vec![brk_at(95, 10), brk_at(110, 10)]);
        // Earlier: the block after the first break keeps its five minutes.
        let moved = drag(&c, Handle::BreakBody(1), -mins(200));
        assert_eq!(moved.breaks, vec![brk_at(50, 10), brk_at(65, 10)]);
        // The edges meet the same limits.
        let moved = drag(&c, Handle::BreakEnd(0), mins(100));
        assert_eq!(moved.breaks, vec![brk_at(50, 55), brk_at(110, 10)]);
        let moved = drag(&c, Handle::BreakStart(1), -mins(100));
        assert_eq!(moved.breaks, vec![brk_at(50, 10), brk_at(65, 55)]);
        // The last break stops five minutes from the end of the class.
        let moved = drag(&c, Handle::BreakBody(1), mins(200));
        assert_eq!(moved.breaks, vec![brk_at(50, 10), brk_at(165, 10)]);
        // Dragging a break the class does not have changes nothing.
        assert_eq!(drag(&c, Handle::BreakBody(2), mins(5)), c);
    }

    #[test]
    fn dragging_a_break_keeps_the_thresholds_of_every_block() {
        let custom = Warnings::new(mins(3) as Secs, mins(1) as Secs).unwrap();
        let c = lab().with_warnings(1, custom).unwrap();
        let moved = drag(&c, Handle::BreakBody(0), mins(5));
        assert_eq!(moved.warnings, c.warnings);
    }

    #[test]
    fn the_zones_of_every_block_are_found_and_dragged_on_their_own() {
        let custom = Warnings::new(mins(20) as Secs, mins(8) as Secs).unwrap();
        let c = lab().with_warnings(2, custom).unwrap();
        // Block 3 is 120..180: its warning zone starts 20 minutes before the end, at 160.
        assert_eq!(
            handle_at(&c, 1600.0, 1800.0, 4.0),
            Some(Handle::Zone {
                block: 2,
                edge: ZoneEdge::Warning
            })
        );
        let moved = drag(
            &c,
            Handle::Zone {
                block: 2,
                edge: ZoneEdge::Warning,
            },
            mins(5),
        );
        assert_eq!(moved.warnings[2].warning, mins(15) as Secs);
        assert_eq!(moved.warnings[0], c.warnings[0]);
        assert_eq!(moved.warnings[1], c.warnings[1]);
    }
}
