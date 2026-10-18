//! The bar: what the schedule state looks like on screen. [`view`] turns the state into plain
//! numbers and a label; [`paint`] draws them. Both are free of Win32 so they are easy to test.

use crate::canvas::{Canvas, RectF};
use crate::config::{Colors, Rgba};
use crate::schedule::{Class, Schedule, Secs, State, Weekday};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SegmentKind {
    /// A teaching block. `orange_at` and `red_at` are where those zones begin (shares of the bar).
    Block {
        orange_at: f32,
        red_at: f32,
    },
    Break,
}

/// A stretch of the bar, as shares (0.0..=1.0) of the whole class.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub start: f32,
    pub end: f32,
    pub kind: SegmentKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BarView {
    /// Share of the whole class that has passed; the bar is filled up to here.
    pub progress: f32,
    pub segments: Vec<Segment>,
    /// The countdown text.
    pub label: String,
    /// Where the label is centred, as a share of the bar width: in the block it counts down.
    pub label_center: f32,
}

/// What the label is about, and so what it is centred on.
#[derive(Clone, Copy)]
enum Focus {
    Block(usize),
    Break,
    /// The whole class, not started yet.
    Whole,
    LastBlock,
}

/// `mm:ss`, or `h:mm:ss` from one hour up.
pub fn format_countdown(secs: Secs) -> String {
    let (h, m, s) = (secs / 3600, secs % 3600 / 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// What to show at `now` on `day`, or `None` when the window should be hidden.
pub fn view(schedule: &Schedule, day: Weekday, now: Secs) -> Option<BarView> {
    let state = schedule.state(day, now);
    let (class, progress_secs, label, focus) = match state {
        State::Idle { .. } => return None,
        State::LeadIn { class, starts_in } => (
            class,
            0,
            format!("Starts in {}", format_countdown(starts_in)),
            Focus::Whole,
        ),
        State::Running {
            class,
            elapsed,
            left,
            block,
            ..
        } => (class, elapsed, format_countdown(left), Focus::Block(block)),
        State::Break {
            class,
            elapsed,
            left,
        } => (
            class,
            elapsed,
            format!("Break {}", format_countdown(left)),
            Focus::Break,
        ),
        State::Ended { class } => (class, u32::MAX, format_countdown(0), Focus::LastBlock),
    };
    let class = schedule.classes(day).get(class)?;
    let total = class.duration().max(1) as f32;
    let share = |secs: Secs| secs as f32 / total;

    let segments = segments(class);
    let middle = |start: Secs, end: Secs| (share(start) + share(end)) / 2.0;
    let label_center = match focus {
        Focus::Block(i) => {
            let b = class.block(i);
            middle(b.start, b.end)
        }
        Focus::LastBlock => {
            let b = class.block(class.block_count() - 1);
            middle(b.start, b.end)
        }
        Focus::Break => class
            .breaks
            .iter()
            .find(|b| b.start <= progress_secs && progress_secs < b.end())
            .map_or(0.5, |b| middle(b.start, b.end())),
        Focus::Whole => 0.5,
    };
    Some(BarView {
        label_center,
        progress: share(progress_secs.min(class.duration())),
        segments,
        label,
    })
}

/// The blocks and the breaks of `class` as shares of the whole class.
fn segments(class: &Class) -> Vec<Segment> {
    let total = class.duration().max(1) as f32;
    let share = |secs: Secs| secs as f32 / total;
    let mut segments = Vec::with_capacity(class.block_count() * 2);
    for index in 0..class.block_count() {
        let block = class.block(index);
        let w = class.warnings[index];
        segments.push(Segment {
            start: share(block.start),
            end: share(block.end),
            kind: SegmentKind::Block {
                orange_at: share(block.end - w.warning.min(block.len())),
                red_at: share(block.end - w.final_.min(block.len())),
            },
        });
        if let Some(b) = class.breaks.get(index) {
            segments.push(Segment {
                start: share(b.start),
                end: share(b.end()),
                kind: SegmentKind::Break,
            });
        }
    }
    segments
}

/// `class` as a bar that is completely filled in, so that every zone and the break show in their
/// colour (what the Settings window edits). It has no label.
pub fn class_view(class: &Class) -> BarView {
    BarView {
        progress: 1.0,
        segments: segments(class),
        label: String::new(),
        label_center: 0.5,
    }
}

/// How much of the break colour shows before the break has been reached.
const UNREACHED_BREAK_ALPHA: f32 = 0.45;

fn with_alpha(c: Rgba, factor: f32) -> Rgba {
    Rgba {
        a: (f32::from(c.a) * factor).round() as u8,
        ..c
    }
}

/// Draw `view` over the whole canvas: a dim track, the elapsed part filled from the left in the
/// colour of each zone, the break in grey, and markers where the warning and final zones begin.
/// The label is not drawn here (text needs the Windows layer).
pub fn paint(canvas: &mut Canvas, view: &BarView, colors: &Colors) {
    let (w, h) = (canvas.width() as f32, canvas.height() as f32);
    let full = RectF::new(0.0, 0.0, w, h);
    let radius = h / 4.0;
    let progress_x = view.progress * w;

    canvas.fill_rounded_rect(full, radius, colors.track, None);

    let fill = |canvas: &mut Canvas, from: f32, to: f32, color: Rgba| {
        if to > from {
            canvas.fill_rounded_rect(
                full,
                radius,
                color,
                Some(RectF::new(from, 0.0, to - from, h)),
            );
        }
    };
    for seg in &view.segments {
        let (x0, x1) = (seg.start * w, seg.end * w);
        match seg.kind {
            SegmentKind::Break => {
                fill(
                    canvas,
                    x0,
                    x1,
                    with_alpha(colors.break_gap, UNREACHED_BREAK_ALPHA),
                );
                fill(canvas, x0, x1.min(progress_x), colors.break_gap);
            }
            SegmentKind::Block { orange_at, red_at } => {
                let (xo, xr) = (orange_at * w, red_at * w);
                fill(canvas, x0, xo.min(progress_x), colors.lesson);
                fill(canvas, xo, xr.min(progress_x), colors.warning);
                fill(canvas, xr, x1.min(progress_x), colors.final_);
            }
        }
    }

    // Markers on the part that is not filled yet, so the teacher sees when the colours change.
    let marker_width = (h / 20.0).max(1.0);
    for seg in &view.segments {
        if let SegmentKind::Block { orange_at, red_at } = seg.kind {
            for (at, color) in [(orange_at, colors.warning), (red_at, colors.final_)] {
                let x = at * w;
                if x > progress_x && at > seg.start && at < seg.end {
                    let rect = RectF::new(x - marker_width / 2.0, h * 0.2, marker_width, h * 0.6);
                    canvas.fill_rounded_rect(rect, 0.0, with_alpha(color, 0.8), None);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::{Class, DEFAULT_BREAK_LEN, MINUTE, Warnings, hm};

    const MON: Weekday = Weekday::Mon;

    fn schedule(classes: Vec<Class>) -> Schedule {
        let mut s = Schedule::new();
        s.set_day(MON, classes).unwrap();
        s
    }

    fn class(start: Secs, end: Secs) -> Class {
        Class::new(start, end, DEFAULT_BREAK_LEN).unwrap()
    }

    /// 09:00-09:50.
    fn single() -> Schedule {
        schedule(vec![class(hm(9, 0), hm(9, 50))])
    }

    /// 09:00-10:40 with the default break.
    fn double() -> Schedule {
        schedule(vec![class(hm(9, 0), hm(10, 40))])
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    // ---- countdown text ----

    #[test]
    fn countdown_formatting() {
        assert_eq!(format_countdown(0), "00:00");
        assert_eq!(format_countdown(59), "00:59");
        assert_eq!(format_countdown(5 * MINUTE + 7), "05:07");
        assert_eq!(format_countdown(50 * MINUTE), "50:00");
        assert_eq!(format_countdown(3600), "1:00:00");
        assert_eq!(format_countdown(3600 + 61), "1:01:01");
    }

    // ---- view ----

    #[test]
    fn hidden_when_idle() {
        let s = single();
        assert_eq!(view(&s, MON, hm(8, 0)), None);
        assert_eq!(view(&s, MON, hm(9, 51)), None);
        assert_eq!(view(&s, Weekday::Sat, hm(9, 10)), None);
    }

    #[test]
    fn running_shows_time_left_and_progress() {
        let v = view(&single(), MON, hm(9, 0) + 15 * MINUTE).unwrap();
        assert_eq!(v.label, "35:00");
        assert!(close(v.progress, 0.3), "{}", v.progress);
        assert_eq!(v.segments.len(), 1);
        let seg = v.segments[0];
        assert!(close(seg.start, 0.0) && close(seg.end, 1.0));
        // 50 minute block: warning for the last 10 minutes, final for the last 5.
        assert_eq!(
            seg.kind,
            SegmentKind::Block {
                orange_at: 0.8,
                red_at: 0.9
            }
        );
    }

    #[test]
    fn start_and_end_of_a_class() {
        let s = single();
        let v = view(&s, MON, hm(9, 0)).unwrap();
        assert_eq!((v.label.as_str(), v.progress), ("50:00", 0.0));
        let v = view(&s, MON, hm(9, 49) + 59).unwrap();
        assert_eq!(v.label, "00:01");
        assert!(v.progress > 0.99);
    }

    #[test]
    fn lead_in_shows_an_empty_bar_with_the_time_to_start() {
        let v = view(&single(), MON, hm(9, 0) - 4 * MINUTE - 12).unwrap();
        assert_eq!(v.label, "Starts in 04:12");
        assert_eq!(v.progress, 0.0);
        assert_eq!(v.segments.len(), 1);
    }

    #[test]
    fn ended_shows_a_full_bar_for_a_moment() {
        let s = single();
        let v = view(&s, MON, hm(9, 50) + 3).unwrap();
        assert_eq!((v.label.as_str(), v.progress), ("00:00", 1.0));
        assert_eq!(
            view(&s, MON, hm(9, 50) + 10),
            None,
            "hidden after the linger time"
        );
    }

    #[test]
    fn two_hour_class_has_two_blocks_and_a_break_segment() {
        let v = view(&double(), MON, hm(9, 0) + MINUTE).unwrap();
        assert_eq!(v.segments.len(), 3);
        let [b1, brk, b2] = [v.segments[0], v.segments[1], v.segments[2]];
        assert!(close(b1.start, 0.0) && close(b1.end, 0.45));
        assert_eq!(brk.kind, SegmentKind::Break);
        assert!(close(brk.start, 0.45) && close(brk.end, 0.55));
        assert!(close(b2.start, 0.55) && close(b2.end, 1.0));
        // Each block has its own zones: 10 and 5 minutes before its own end.
        let SegmentKind::Block { orange_at, red_at } = b1.kind else {
            panic!()
        };
        assert!(
            close(orange_at, 0.35) && close(red_at, 0.40),
            "{orange_at} {red_at}"
        );
        let SegmentKind::Block { orange_at, red_at } = b2.kind else {
            panic!()
        };
        assert!(
            close(orange_at, 0.90) && close(red_at, 0.95),
            "{orange_at} {red_at}"
        );
    }

    #[test]
    fn break_has_its_own_countdown() {
        let v = view(&double(), MON, hm(9, 45) + 30).unwrap();
        assert_eq!(v.label, "Break 09:30");
        assert!(close(v.progress, 2730.0 / 6000.0));
        let v = view(&double(), MON, hm(9, 55)).unwrap();
        assert_eq!(v.label, "45:00", "back to counting the second block");
    }

    #[test]
    fn the_label_is_centred_in_the_block_it_counts_down() {
        // A single block: the middle of the bar.
        let single = view(&single(), MON, hm(9, 10)).unwrap();
        assert!(close(single.label_center, 0.5));
        // A double class: block 1 is 0..45%, the break 45..55%, block 2 55..100%.
        let at = |m: u32| {
            view(&double(), MON, hm(9, 0) + m * MINUTE)
                .unwrap()
                .label_center
        };
        assert!(close(at(10), 0.225), "first block: {}", at(10));
        assert!(close(at(44), 0.225));
        assert!(close(at(60), 0.775), "second block: {}", at(60));
        assert!(close(at(99), 0.775));
        assert!(close(at(50), 0.5), "the break text sits on the break");
    }

    #[test]
    fn the_label_before_and_after_class_is_centred_sensibly() {
        // Lead-in: the class has not started, so the whole bar.
        let lead = view(&double(), MON, hm(9, 0) - 60).unwrap();
        assert!(lead.label.starts_with("Starts in"));
        assert!(close(lead.label_center, 0.5));
        // Just ended: the last block, as the last countdown was in it.
        let ended = view(&double(), MON, hm(10, 40) + 2).unwrap();
        assert!(close(ended.label_center, 0.775));
        let ended = view(&single(), MON, hm(9, 50) + 2).unwrap();
        assert!(close(ended.label_center, 0.5));
    }

    #[test]
    fn the_label_follows_a_moved_break() {
        let brk = crate::schedule::Break {
            start: 30 * MINUTE,
            len: 20 * MINUTE,
        };
        let c = class(hm(9, 0), hm(10, 40)).with_breaks(vec![brk]).unwrap();
        let s = schedule(vec![c]);
        let at = |m: u32| view(&s, MON, hm(9, 0) + m * MINUTE).unwrap().label_center;
        assert!(close(at(10), 0.15), "block 1 is now 0..30%: {}", at(10));
        assert!(close(at(40), 0.40), "the break is 30..50%");
        assert!(close(at(70), 0.75), "block 2 is 50..100%");
    }

    #[test]
    fn warnings_longer_than_a_block_are_limited_to_it() {
        let c = class(hm(9, 0), hm(9, 50))
            .with_warnings(0, Warnings::new(90 * MINUTE, 5 * MINUTE).unwrap())
            .unwrap();
        let v = view(&schedule(vec![c]), MON, hm(9, 0)).unwrap();
        assert_eq!(
            v.segments[0].kind,
            SegmentKind::Block {
                orange_at: 0.0,
                red_at: 0.9
            }
        );
    }

    #[test]
    fn view_follows_a_moved_break_and_custom_warnings() {
        let brk = crate::schedule::Break {
            start: 30 * MINUTE,
            len: 20 * MINUTE,
        };
        let c = class(hm(9, 0), hm(10, 40))
            .with_breaks(vec![brk])
            .unwrap()
            .with_warnings(1, Warnings::new(20 * MINUTE, 8 * MINUTE).unwrap())
            .unwrap();
        let v = view(&schedule(vec![c]), MON, hm(9, 0)).unwrap();
        assert!(close(v.segments[1].start, 0.3) && close(v.segments[1].end, 0.5));
        let SegmentKind::Block { orange_at, red_at } = v.segments[2].kind else {
            panic!()
        };
        assert!(close(orange_at, 80.0 / 100.0) && close(red_at, 92.0 / 100.0));
    }

    // ---- paint ----

    const W: u32 = 1000;
    const H: u32 = 20;

    fn colors() -> Colors {
        Colors::default()
    }

    fn solid(c: Rgba) -> u32 {
        0xff00_0000 | u32::from(c.r) << 16 | u32::from(c.g) << 8 | u32::from(c.b)
    }

    fn painted(s: &Schedule, now: Secs) -> Canvas {
        let v = view(s, MON, now).expect("visible");
        let mut canvas = Canvas::new(W, H);
        paint(&mut canvas, &v, &colors());
        canvas
    }

    fn alpha(canvas: &Canvas, x: u32) -> u32 {
        canvas.pixel(x, H / 2) >> 24
    }

    #[test]
    fn unfilled_bar_is_only_the_dim_track_with_markers() {
        let c = painted(&single(), hm(9, 0));
        let track = colors().track;
        assert_eq!(alpha(&c, 400), u32::from(track.a), "track alpha");
        assert_ne!(c.pixel(400, 10), solid(colors().lesson));
        // Markers where warning (80%) and final (90%) begin, in those colours.
        let warning = c.pixel(800, H / 2);
        let final_ = c.pixel(900, H / 2);
        assert!(warning >> 24 > u32::from(track.a), "warning marker visible");
        assert!(
            (warning >> 16 & 0xff) > (warning & 0xff),
            "warning marker is warm"
        );
        assert!(final_ >> 24 > u32::from(track.a), "final marker visible");
        assert_ne!(warning, final_);
        // Rounded outer corners.
        assert_eq!(c.pixel(0, 0) >> 24, 0);
        assert_eq!(c.pixel(W - 1, H - 1) >> 24, 0);
    }

    #[test]
    fn bar_fills_from_the_left_in_green() {
        let c = painted(&single(), hm(9, 0) + 25 * MINUTE); // 50%
        assert_eq!(c.pixel(250, 10), solid(colors().lesson));
        assert_eq!(c.pixel(498, 10), solid(colors().lesson));
        assert_ne!(c.pixel(502, 10), solid(colors().lesson));
        assert_eq!(alpha(&c, 502), u32::from(colors().track.a));
    }

    #[test]
    fn zones_follow_elapsed_time() {
        // 40:00 elapsed of 50:00 = 80%: the warning zone has just started.
        let c = painted(&single(), hm(9, 0) + 40 * MINUTE + 90); // 83%
        assert_eq!(c.pixel(500, 10), solid(colors().lesson));
        assert_eq!(c.pixel(810, 10), solid(colors().warning));
        assert_ne!(c.pixel(850, 10), solid(colors().warning), "not filled yet");

        let c = painted(&single(), hm(9, 0) + 46 * MINUTE); // 92%
        assert_eq!(c.pixel(500, 10), solid(colors().lesson));
        assert_eq!(c.pixel(850, 10), solid(colors().warning));
        assert_eq!(c.pixel(910, 10), solid(colors().final_));
        assert_ne!(c.pixel(950, 10), solid(colors().final_));
    }

    #[test]
    fn a_full_bar_is_fully_coloured_without_markers_showing() {
        let c = painted(&single(), hm(9, 50) + 1);
        for (x, color) in [
            (100, colors().lesson),
            (850, colors().warning),
            (950, colors().final_),
        ] {
            assert_eq!(c.pixel(x, 10), solid(color), "x={x}");
        }
        assert_eq!(c.pixel(0, 0) >> 24, 0, "corners stay rounded");
    }

    #[test]
    fn colours_come_from_the_config() {
        let v = view(&single(), MON, hm(9, 0) + 25 * MINUTE).unwrap();
        let mut custom = colors();
        custom.lesson = Rgba::rgb(1, 2, 3);
        custom.track = Rgba {
            r: 9,
            g: 9,
            b: 9,
            a: 0x40,
        };
        let mut canvas = Canvas::new(W, H);
        paint(&mut canvas, &v, &custom);
        assert_eq!(canvas.pixel(100, 10), solid(custom.lesson));
        assert_eq!(alpha(&canvas, 700), 0x40);
    }

    #[test]
    fn break_is_a_grey_gap_between_two_blocks() {
        // During the break at 50% through it: the break segment spans 45%..55%.
        let c = painted(&double(), hm(9, 0) + 50 * MINUTE);
        let grey = colors().break_gap;
        assert_eq!(c.pixel(100, 10), solid(colors().lesson));
        assert_eq!(c.pixel(460, 10), solid(grey), "elapsed part of the break");
        let ahead = c.pixel(540, 10);
        assert!(
            ahead >> 24 > u32::from(colors().track.a) && ahead >> 24 < 255,
            "dim grey: {ahead:#x}"
        );
        assert_eq!(
            c.pixel(700, 10) >> 24,
            u32::from(colors().track.a),
            "second block untouched"
        );
    }

    #[test]
    fn each_block_of_a_double_class_has_its_own_zones() {
        // End of block 1: warning (35%..40%) and final (40%..45%) just before the break.
        let c = painted(&double(), hm(9, 0) + 44 * MINUTE);
        assert_eq!(c.pixel(300, 10), solid(colors().lesson));
        assert_eq!(c.pixel(375, 10), solid(colors().warning));
        assert_eq!(c.pixel(420, 10), solid(colors().final_));
        // Block 2 starts in the lesson zone again.
        let c = painted(&double(), hm(9, 0) + 70 * MINUTE);
        assert_eq!(c.pixel(600, 10), solid(colors().lesson));
        assert_eq!(
            c.pixel(420, 10),
            solid(colors().final_),
            "earlier colours stay"
        );
        assert_eq!(c.pixel(500, 10), solid(colors().break_gap));
    }

    #[test]
    fn moving_edge_is_anti_aliased() {
        // 9:00:03 of a 3000 s class = 0.1% = 1 px: the edge pixel exists but 1 s later it moves on.
        let a = painted(&single(), hm(9, 0) + 1);
        let b = painted(&single(), hm(9, 0) + 2);
        let edge = |c: &Canvas| (0..W).find(|&x| c.pixel(x, 10) != solid(colors().lesson));
        assert!(edge(&b) >= edge(&a));
        // A fractional position gives a partially covered pixel.
        let partial = (0..5).any(|x| {
            let p = b.pixel(x, 10);
            p != solid(colors().lesson) && p >> 24 > u32::from(colors().track.a)
        });
        assert!(partial, "no blended edge pixel");
    }

    #[test]
    fn painting_adapts_to_any_canvas_size() {
        let v = view(&double(), MON, hm(9, 30)).unwrap();
        for (w, h) in [(1, 1), (40, 6), (3000, 200)] {
            let mut canvas = Canvas::new(w, h);
            paint(&mut canvas, &v, &colors());
        }
    }

    // ---- several breaks ----

    /// 09:00-12:00 with breaks at 50..60 and 110..120.
    fn lab() -> Class {
        let brk = |start: u32, len: u32| crate::schedule::Break {
            start: start * MINUTE,
            len: len * MINUTE,
        };
        class(hm(9, 0), hm(12, 0))
            .with_breaks(vec![brk(50, 10), brk(110, 10)])
            .unwrap()
    }

    #[test]
    fn a_class_with_two_breaks_draws_three_blocks_and_two_breaks() {
        let v = class_view(&lab());
        let kinds: Vec<bool> = v
            .segments
            .iter()
            .map(|s| matches!(s.kind, SegmentKind::Break))
            .collect();
        assert_eq!(kinds, [false, true, false, true, false]);
        let edge = |i: usize| (v.segments[i].start * 180.0, v.segments[i].end * 180.0);
        let near = |(a, b): (f32, f32), (c, d): (f32, f32)| close(a, c) && close(b, d);
        assert!(near(edge(0), (0.0, 50.0)), "{:?}", edge(0));
        assert!(near(edge(1), (50.0, 60.0)));
        assert!(near(edge(2), (60.0, 110.0)));
        assert!(near(edge(3), (110.0, 120.0)));
        assert!(near(edge(4), (120.0, 180.0)));
    }

    #[test]
    fn the_label_sits_on_the_break_or_block_that_is_counted_down() {
        let s = schedule(vec![lab()]);
        let at = |m: u32| view(&s, MON, hm(9, 0) + m * MINUTE).unwrap();
        assert!(close(at(10).label_center, 25.0 / 180.0));
        assert!(close(at(55).label_center, 55.0 / 180.0), "first break");
        assert!(close(at(80).label_center, 85.0 / 180.0), "second block");
        assert!(close(at(115).label_center, 115.0 / 180.0), "second break");
        assert_eq!(at(115).label, "Break 05:00");
        assert!(close(at(150).label_center, 150.0 / 180.0), "third block");
    }

    #[test]
    fn every_block_has_its_own_zones() {
        let c = lab()
            .with_warnings(1, Warnings::new(20 * MINUTE, 8 * MINUTE).unwrap())
            .unwrap();
        let v = class_view(&c);
        let SegmentKind::Block { orange_at, red_at } = v.segments[2].kind else {
            panic!()
        };
        // Block 2 ends at 110 minutes: 20 and 8 minutes before that.
        assert!(close(orange_at * 180.0, 90.0) && close(red_at * 180.0, 102.0));
        let SegmentKind::Block { orange_at, .. } = v.segments[4].kind else {
            panic!()
        };
        assert!(
            close(orange_at * 180.0, 170.0),
            "block 3 keeps the defaults"
        );
    }
}
