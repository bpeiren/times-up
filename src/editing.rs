//! Editing a class on the bar itself: the break and the orange and red zone boundaries can be
//! dragged. Pure rules (no Win32): which handle is under the pointer, and what a drag does to the
//! class, snapped to whole minutes and clamped so the result is always a sensible class.

use crate::schedule::{Break, Class, MINUTE, Secs, Warnings};

/// A block between the start (or the break) and the end of the class needs at least this much.
pub const MIN_BLOCK: Secs = 5 * MINUTE;
pub const MIN_BREAK: Secs = MINUTE;
/// The red zone is at least this long, and orange starts at least this much before red.
pub const MIN_RED: Secs = MINUTE;
pub const MIN_ZONE_GAP: Secs = MINUTE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneEdge {
    Orange,
    Red,
}

/// Something on the bar that can be dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    /// The start of the break: moves it, so the first block gets longer or shorter.
    BreakStart,
    /// The end of the break.
    BreakEnd,
    /// The break as a whole: it keeps its length.
    BreakBody,
    /// Where the orange or red zone of a block begins.
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
        ZoneEdge::Orange => w.orange,
        ZoneEdge::Red => w.red,
    };
    b.end - before_end.min(b.len())
}

/// The handle under `x` (pixels from the left edge of a bar `width` pixels wide), if any.
/// Boundaries within `grip` pixels count; the nearest one wins, with the break edges ahead of the
/// zone boundaries when equally near. Failing that, the inside of the break is its body.
pub fn handle_at(class: &Class, x: f32, width: f32, grip: f32) -> Option<Handle> {
    if width <= 0.0 || class.duration() == 0 {
        return None;
    }
    let px = |secs: Secs| secs as f32 / class.duration() as f32 * width;
    let mut candidates: Vec<(f32, Handle)> = Vec::with_capacity(6);
    if let Some(b) = class.brk {
        candidates.push((px(b.start), Handle::BreakStart));
        candidates.push((px(b.end()), Handle::BreakEnd));
    }
    for block in 0..class.block_count() {
        for edge in [ZoneEdge::Orange, ZoneEdge::Red] {
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
    let b = class.brk?;
    (x > px(b.start) && x < px(b.end())).then_some(Handle::BreakBody)
}

/// `class` after dragging `handle` by `delta` seconds (positive = later). Everything is snapped
/// to whole minutes and kept within the limits; a handle the class does not have changes nothing.
pub fn drag(class: &Class, handle: Handle, delta: i64) -> Class {
    let duration = i64::from(class.duration());
    let (min_block, min_break) = (i64::from(MIN_BLOCK), i64::from(MIN_BREAK));
    let mut out = class.clone();

    match handle {
        Handle::BreakStart | Handle::BreakEnd | Handle::BreakBody => {
            let Some(b) = class.brk else {
                return out;
            };
            let (start, end) = (i64::from(b.start), i64::from(b.end()));
            let len = end - start;
            let (new_start, new_end) = match handle {
                Handle::BreakStart => {
                    let s = snap(start + delta).clamp(min_block, (end - min_break).max(min_block));
                    (s, end)
                }
                Handle::BreakEnd => {
                    let e = snap(end + delta).clamp(
                        start + min_break,
                        (duration - min_block).max(start + min_break),
                    );
                    (start, e)
                }
                _ => {
                    let s = snap(start + delta)
                        .clamp(min_block, (duration - min_block - len).max(min_block));
                    (s, s + len)
                }
            };
            out.brk = Some(Break {
                start: new_start as Secs,
                len: (new_end - new_start) as Secs,
            });
        }
        Handle::Zone { block, edge } => {
            if block >= class.block_count() {
                return out;
            }
            let len = i64::from(class.block(block).len());
            let w = class.warnings[block];
            // The zones as they are drawn: never longer than the block itself.
            let orange = i64::from(w.orange).min(len);
            let red = i64::from(w.red).min(len);
            let (gap, min_red) = (i64::from(MIN_ZONE_GAP), i64::from(MIN_RED));
            // Dragging later means less time before the end of the block.
            let (new_orange, new_red) = match edge {
                ZoneEdge::Orange => {
                    let lo = (red + gap).min(len);
                    (snap(orange - delta).clamp(lo, len), red)
                }
                ZoneEdge::Red => {
                    let hi = (orange - gap).max(min_red);
                    (orange, snap(red - delta).clamp(min_red, hi))
                }
            };
            out.warnings[block] = Warnings {
                orange: new_orange as Secs,
                red: new_red.min(new_orange) as Secs,
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
        assert_eq!(at(&c, 450.0), Some(Handle::BreakStart));
        assert_eq!(at(&c, 453.0), Some(Handle::BreakStart), "within the grip");
        assert_eq!(at(&c, 550.0), Some(Handle::BreakEnd));
        assert_eq!(at(&c, 500.0), Some(Handle::BreakBody));
        assert_eq!(at(&c, 460.0), Some(Handle::BreakBody));
        assert_eq!(
            at(&c, 545.0),
            Some(Handle::BreakBody),
            "beyond the end grip is body"
        );
    }

    #[test]
    fn every_zone_boundary_has_a_handle() {
        let c = double();
        // Block 1: orange at 35 min, red at 40 min. Block 2: orange at 90, red at 95.
        let zone = |block, edge| Some(Handle::Zone { block, edge });
        assert_eq!(at(&c, 350.0), zone(0, ZoneEdge::Orange));
        assert_eq!(at(&c, 400.0), zone(0, ZoneEdge::Red));
        assert_eq!(at(&c, 900.0), zone(1, ZoneEdge::Orange));
        assert_eq!(at(&c, 950.0), zone(1, ZoneEdge::Red));
        assert_eq!(at(&c, 902.0), zone(1, ZoneEdge::Orange));
    }

    #[test]
    fn a_single_block_class_has_only_zone_handles() {
        let c = single(); // 50 minutes: orange at 40, red at 45
        let w = 500.0;
        assert_eq!(
            handle_at(&c, 400.0, w, GRIP),
            Some(Handle::Zone {
                block: 0,
                edge: ZoneEdge::Orange
            })
        );
        assert_eq!(
            handle_at(&c, 450.0, w, GRIP),
            Some(Handle::Zone {
                block: 0,
                edge: ZoneEdge::Red
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
        // Orange at 6 minutes before the block end and red 1 minute before it, in a bar where a
        // minute is only 6 px: the pointer picks the closer boundary.
        let mut c = double();
        c.warnings[0] = Warnings {
            orange: 2 * 60,
            red: 60,
        };
        let w = 600.0; // 6 px per minute
        let orange_x = (45.0 - 2.0) * 6.0;
        let red_x = (45.0 - 1.0) * 6.0;
        assert_eq!(
            handle_at(&c, orange_x + 1.0, w, 5.0),
            Some(Handle::Zone {
                block: 0,
                edge: ZoneEdge::Orange
            })
        );
        assert_eq!(
            handle_at(&c, red_x - 1.0, w, 5.0),
            Some(Handle::Zone {
                block: 0,
                edge: ZoneEdge::Red
            })
        );
    }

    #[test]
    fn break_edges_win_over_zone_boundaries_at_the_same_spot() {
        // Red zone of 5 minutes ends exactly where a break could start: a block-1 red marker
        // sitting on the break start.
        let mut c = double();
        c.warnings[0] = Warnings {
            orange: 10 * 60,
            red: 0,
        };
        // red_at == break start (45 min).
        assert_eq!(at(&c, 450.0), Some(Handle::BreakStart));
    }

    #[test]
    fn a_class_without_a_break_has_no_break_handles() {
        let c = double().with_break(None).unwrap();
        for x in [450.0, 500.0, 550.0] {
            assert!(!matches!(
                at(&c, x),
                Some(Handle::BreakStart | Handle::BreakEnd | Handle::BreakBody)
            ));
        }
    }

    // ---- dragging the break ----

    fn brk(c: &Class) -> (i64, i64) {
        let b = c.brk.unwrap();
        (i64::from(b.start), i64::from(b.end()))
    }

    #[test]
    fn dragging_the_break_body_moves_it_and_keeps_its_length() {
        let c = drag(&double(), Handle::BreakBody, mins(10));
        assert_eq!(brk(&c), (mins(55), mins(65)));
        let c = drag(&double(), Handle::BreakBody, -mins(20));
        assert_eq!(brk(&c), (mins(25), mins(35)));
    }

    #[test]
    fn dragging_the_break_start_changes_its_length() {
        let c = drag(&double(), Handle::BreakStart, -mins(5));
        assert_eq!(
            brk(&c),
            (mins(40), mins(55)),
            "starts earlier, ends where it did"
        );
        let c = drag(&double(), Handle::BreakStart, mins(3));
        assert_eq!(brk(&c), (mins(48), mins(55)));
    }

    #[test]
    fn dragging_the_break_end_changes_its_length() {
        let c = drag(&double(), Handle::BreakEnd, mins(7));
        assert_eq!(brk(&c), (mins(45), mins(62)));
        let c = drag(&double(), Handle::BreakEnd, -mins(4));
        assert_eq!(brk(&c), (mins(45), mins(51)));
    }

    #[test]
    fn break_drags_snap_to_whole_minutes() {
        let c = drag(&double(), Handle::BreakBody, 94); // 1.57 minutes -> 2
        assert_eq!(brk(&c), (mins(47), mins(57)));
        let c = drag(&double(), Handle::BreakBody, 29); // under half a minute -> 0
        assert_eq!(brk(&c), (mins(45), mins(55)));
        let c = drag(&double(), Handle::BreakBody, -31); // -0.52 -> -1
        assert_eq!(brk(&c), (mins(44), mins(54)));
    }

    #[test]
    fn blocks_keep_at_least_five_minutes() {
        // Far to the left: the first block stays 5 minutes; the break keeps its 10.
        let c = drag(&double(), Handle::BreakBody, -mins(500));
        assert_eq!(brk(&c), (mins(5), mins(15)));
        // Far to the right: the second block stays 5 minutes.
        let c = drag(&double(), Handle::BreakBody, mins(500));
        assert_eq!(brk(&c), (mins(85), mins(95)));
        // Start edge dragged to the very start.
        let c = drag(&double(), Handle::BreakStart, -mins(500));
        assert_eq!(brk(&c), (mins(5), mins(55)));
        // End edge dragged to the very end.
        let c = drag(&double(), Handle::BreakEnd, mins(500));
        assert_eq!(brk(&c), (mins(45), mins(95)));
    }

    #[test]
    fn the_break_keeps_at_least_one_minute() {
        let c = drag(&double(), Handle::BreakStart, mins(500));
        assert_eq!(brk(&c), (mins(54), mins(55)));
        let c = drag(&double(), Handle::BreakEnd, -mins(500));
        assert_eq!(brk(&c), (mins(45), mins(46)));
    }

    #[test]
    fn a_dragged_break_always_gives_a_valid_class() {
        for handle in [Handle::BreakStart, Handle::BreakEnd, Handle::BreakBody] {
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
        for handle in [Handle::BreakStart, Handle::BreakEnd, Handle::BreakBody] {
            assert_eq!(drag(&c, handle, mins(5)), c);
        }
    }

    // ---- dragging zone boundaries ----

    fn zones(c: &Class, block: usize) -> (i64, i64) {
        let w = c.warnings[block];
        (i64::from(w.orange), i64::from(w.red))
    }

    fn orange(block: usize) -> Handle {
        Handle::Zone {
            block,
            edge: ZoneEdge::Orange,
        }
    }

    fn red(block: usize) -> Handle {
        Handle::Zone {
            block,
            edge: ZoneEdge::Red,
        }
    }

    #[test]
    fn dragging_a_zone_boundary_later_shortens_the_zone() {
        // Orange starts 10 minutes before the end; 3 minutes later leaves 7.
        let c = drag(&double(), orange(0), mins(3));
        assert_eq!(zones(&c, 0), (mins(7), mins(5)));
        // Earlier lengthens it.
        let c = drag(&double(), orange(0), -mins(4));
        assert_eq!(zones(&c, 0), (mins(14), mins(5)));
        let c = drag(&double(), red(0), -mins(2));
        assert_eq!(zones(&c, 0), (mins(10), mins(7)));
    }

    #[test]
    fn each_block_has_its_own_zones() {
        let c = drag(&double(), red(1), mins(2));
        assert_eq!(zones(&c, 1), (mins(10), mins(3)));
        assert_eq!(
            zones(&c, 0),
            (mins(10), mins(5)),
            "the other block is untouched"
        );
    }

    #[test]
    fn orange_stays_before_red_with_a_minute_between() {
        // Orange dragged past red: stops one minute before it.
        let c = drag(&double(), orange(0), mins(30));
        assert_eq!(zones(&c, 0), (mins(6), mins(5)));
        // Red dragged earlier than orange: stops one minute after it.
        let c = drag(&double(), red(0), -mins(30));
        assert_eq!(zones(&c, 0), (mins(10), mins(9)));
    }

    #[test]
    fn red_keeps_at_least_a_minute() {
        let c = drag(&double(), red(0), mins(30));
        assert_eq!(zones(&c, 0), (mins(10), mins(1)));
    }

    #[test]
    fn zones_stay_inside_their_block_so_they_never_reach_the_break() {
        // Orange dragged far to the left stops at the start of the block (45 minutes long).
        let c = drag(&double(), orange(0), -mins(500));
        assert_eq!(zones(&c, 0), (mins(45), mins(5)));
        // The second block is also 45 minutes, so it cannot reach into the break either.
        let c = drag(&double(), orange(1), -mins(500));
        assert_eq!(zones(&c, 1), (mins(45), mins(5)));
        // Red follows the same limit through orange.
        let c = drag(&double(), red(1), -mins(500));
        assert_eq!(zones(&c, 1), (mins(10), mins(9)));
    }

    #[test]
    fn zone_drags_snap_to_whole_minutes() {
        let c = drag(&double(), orange(0), 100); // 1.67 min -> 2
        assert_eq!(zones(&c, 0), (mins(8), mins(5)));
        let c = drag(&double(), orange(0), 20); // less than half a minute -> no change
        assert_eq!(zones(&c, 0), (mins(10), mins(5)));
    }

    #[test]
    fn a_zone_longer_than_its_block_is_dragged_from_where_it_is_drawn() {
        // Orange is configured as 60 minutes but a 45 minute block only shows 45.
        let mut c = double();
        c.warnings[0] = Warnings {
            orange: 60 * 60,
            red: 5 * 60,
        };
        let moved = drag(&c, orange(0), mins(5));
        assert_eq!(zones(&moved, 0), (mins(40), mins(5)));
    }

    #[test]
    fn a_dragged_zone_always_gives_a_valid_class() {
        let handles = [orange(0), red(0), orange(1), red(1)];
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
                    assert!(r <= o, "{handle:?} {delta}: red {r} orange {o}");
                    assert!(r >= 60, "{handle:?} {delta}");
                    assert!(o <= mins(45), "{handle:?} {delta}");
                }
            }
        }
    }

    #[test]
    fn zone_handles_for_a_missing_block_do_nothing() {
        let c = single();
        assert_eq!(drag(&c, orange(1), mins(5)), c);
        assert_eq!(drag(&c, red(1), mins(5)), c);
    }

    #[test]
    fn dragging_a_zone_does_not_touch_the_break() {
        let c = drag(&double(), orange(0), mins(2));
        assert_eq!(c.brk, double().brk);
        let c = drag(&double(), Handle::BreakBody, mins(2));
        assert_eq!(c.warnings, double().warnings);
    }

    #[test]
    fn moving_the_break_changes_which_block_a_boundary_belongs_to() {
        // A 20 minute first block (break at 20..30) with red 5 and orange 10 before its end.
        let c = drag(&double(), Handle::BreakBody, -mins(25));
        assert_eq!(brk(&c), (mins(20), mins(30)));
        assert_eq!(c.block(0).len(), mins(20) as u32);
        // Its zones are still valid and inside the shorter block.
        let (o, r) = zones(&c, 0);
        assert!(o <= mins(20) && r <= o);
        // A zone drag on the shortened block uses the new length.
        let c = drag(&c, orange(0), -mins(500));
        assert_eq!(zones(&c, 0).0, mins(20));
    }
}
