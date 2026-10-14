//! Mouse interaction with the bar as plain geometry: what is under the pointer, how a drag changes
//! the window rectangle, and where the window may live on the available monitors. All sizes are
//! device pixels. No Win32 here, so every rule is unit tested.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub const fn right(&self) -> i32 {
        self.x + self.w
    }

    pub const fn bottom(&self) -> i32 {
        self.y + self.h
    }

    /// Area of the overlap with `other` (0 when they do not touch).
    pub fn overlap_area(&self, other: &Rect) -> i64 {
        let w = (self.right().min(other.right()) - self.x.max(other.x)).max(0);
        let h = (self.bottom().min(other.bottom()) - self.y.max(other.y)).max(0);
        i64::from(w) * i64::from(h)
    }

    pub fn area(&self) -> i64 {
        i64::from(self.w.max(0)) * i64::from(self.h.max(0))
    }
}

/// Which edges a resize drag moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Edges {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

/// What is under the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    /// The body of the bar: dragging moves the window.
    Move,
    /// An edge or corner: dragging resizes the window.
    Resize(Edges),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorShape {
    Arrow,
    SizeWestEast,
    SizeNorthSouth,
    SizeNorthwestSoutheast,
    SizeNortheastSouthwest,
}

impl Hit {
    pub fn cursor(self) -> CursorShape {
        let Hit::Resize(e) = self else {
            return CursorShape::Arrow;
        };
        match (e.left || e.right, e.top || e.bottom) {
            (true, true) if e.left == e.top => CursorShape::SizeNorthwestSoutheast,
            (true, true) => CursorShape::SizeNortheastSouthwest,
            (true, false) => CursorShape::SizeWestEast,
            (false, true) => CursorShape::SizeNorthSouth,
            (false, false) => CursorShape::Arrow,
        }
    }
}

/// What is at `(x, y)` (client coordinates) of a `w` x `h` window, with `grip` pixels of edge.
/// The grip never takes more than a third of the window, so tiny bars can still be moved.
pub fn hit_test(x: i32, y: i32, w: i32, h: i32, grip: i32) -> Hit {
    let (gx, gy) = (grip.min(w / 3).max(1), grip.min(h / 3).max(1));
    let edges = Edges {
        left: x < gx,
        right: x >= w - gx,
        top: y < gy,
        bottom: y >= h - gy,
    };
    if edges == Edges::default() {
        Hit::Move
    } else {
        Hit::Resize(edges)
    }
}

/// The window rectangle after dragging `hit` by `(dx, dy)` from `start`. A resize keeps the
/// opposite edge in place and never goes below `min` (width, height).
pub fn drag_rect(start: Rect, hit: Hit, dx: i32, dy: i32, min: (i32, i32)) -> Rect {
    let Hit::Resize(e) = hit else {
        return Rect::new(start.x + dx, start.y + dy, start.w, start.h);
    };
    let (mut left, mut top, mut right, mut bottom) =
        (start.x, start.y, start.right(), start.bottom());
    if e.left {
        left = (left + dx).min(right - min.0);
    }
    if e.right {
        right = (right + dx).max(left + min.0);
    }
    if e.top {
        top = (top + dy).min(bottom - min.1);
    }
    if e.bottom {
        bottom = (bottom + dy).max(top + min.1);
    }
    Rect::new(left, top, right - left, bottom - top)
}

/// Top centre of `area`, `margin` pixels below its top edge: where the bar starts by default.
pub fn default_rect(width: i32, height: i32, area: Rect, margin: i32) -> Rect {
    Rect::new(
        area.x + (area.w - width) / 2,
        area.y + margin,
        width,
        height,
    )
}

/// Make sure a saved rectangle can actually be seen. `areas` are the usable areas of the
/// monitors, primary first.
///
/// * At least half of it on one monitor: kept as it is.
/// * Hardly on any monitor (e.g. the monitor was disconnected, or moved far): pulled fully onto
///   the monitor it overlaps most.
/// * On no monitor at all: back to the default place on the primary monitor.
pub fn fit_on_screen(rect: Rect, areas: &[Rect], margin: i32) -> Rect {
    let Some(primary) = areas.first() else {
        return rect;
    };
    let (best, overlap) = areas
        .iter()
        .map(|a| (a, rect.overlap_area(a)))
        .max_by_key(|&(_, o)| o)
        .expect("areas is not empty");
    if overlap == 0 {
        let (w, h) = (rect.w.min(primary.w), rect.h.min(primary.h));
        return default_rect(w, h, *primary, margin);
    }
    if overlap * 2 >= rect.area() {
        return rect;
    }
    let (w, h) = (rect.w.min(best.w), rect.h.min(best.h));
    Rect::new(
        rect.x.clamp(best.x, best.right() - w),
        rect.y.clamp(best.y, best.bottom() - h),
        w,
        h,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn edges(left: bool, right: bool, top: bool, bottom: bool) -> Edges {
        Edges {
            left,
            right,
            top,
            bottom,
        }
    }

    // ---- rect ----

    #[test]
    fn overlap_and_area() {
        let a = Rect::new(0, 0, 100, 50);
        assert_eq!(a.area(), 5000);
        assert_eq!(a.overlap_area(&Rect::new(50, 25, 100, 100)), 50 * 25);
        assert_eq!(
            a.overlap_area(&Rect::new(100, 0, 10, 10)),
            0,
            "touching is not overlapping"
        );
        assert_eq!(a.overlap_area(&Rect::new(-500, -500, 10, 10)), 0);
        assert_eq!(a.overlap_area(&a), 5000);
        assert_eq!(Rect::new(0, 0, -5, 10).area(), 0);
    }

    // ---- hit testing ----

    #[test]
    fn body_moves_and_edges_resize() {
        let hit = |x, y| hit_test(x, y, 600, 60, 6);
        assert_eq!(hit(300, 30), Hit::Move);
        assert_eq!(hit(6, 30), Hit::Move, "just inside the grip");
        assert_eq!(hit(5, 30), Hit::Resize(edges(true, false, false, false)));
        assert_eq!(hit(594, 30), Hit::Resize(edges(false, true, false, false)));
        assert_eq!(hit(300, 5), Hit::Resize(edges(false, false, true, false)));
        assert_eq!(hit(300, 54), Hit::Resize(edges(false, false, false, true)));
        assert_eq!(hit(0, 0), Hit::Resize(edges(true, false, true, false)));
        assert_eq!(hit(599, 0), Hit::Resize(edges(false, true, true, false)));
        assert_eq!(hit(0, 59), Hit::Resize(edges(true, false, false, true)));
        assert_eq!(hit(599, 59), Hit::Resize(edges(false, true, false, true)));
    }

    #[test]
    fn grip_never_swallows_a_tiny_bar() {
        // 30 px high with a 12 px grip: only 10 px per edge, so the middle can still be dragged.
        assert_eq!(hit_test(100, 15, 200, 30, 12), Hit::Move);
        assert_eq!(
            hit_test(100, 9, 200, 30, 12),
            Hit::Resize(edges(false, false, true, false))
        );
        // Degenerate sizes do not panic.
        let _ = hit_test(0, 0, 0, 0, 6);
        let _ = hit_test(0, 0, 1, 1, 6);
    }

    #[test]
    fn cursor_shapes() {
        let c = |l, r, t, b| Hit::Resize(edges(l, r, t, b)).cursor();
        assert_eq!(Hit::Move.cursor(), CursorShape::Arrow);
        assert_eq!(c(true, false, false, false), CursorShape::SizeWestEast);
        assert_eq!(c(false, true, false, false), CursorShape::SizeWestEast);
        assert_eq!(c(false, false, true, false), CursorShape::SizeNorthSouth);
        assert_eq!(
            c(true, false, true, false),
            CursorShape::SizeNorthwestSoutheast
        );
        assert_eq!(
            c(false, true, false, true),
            CursorShape::SizeNorthwestSoutheast
        );
        assert_eq!(
            c(false, true, true, false),
            CursorShape::SizeNortheastSouthwest
        );
        assert_eq!(
            c(true, false, false, true),
            CursorShape::SizeNortheastSouthwest
        );
    }

    // ---- dragging ----

    const START: Rect = Rect::new(100, 100, 600, 60);
    const MIN: (i32, i32) = (160, 24);

    #[test]
    fn moving_keeps_the_size() {
        assert_eq!(
            drag_rect(START, Hit::Move, 30, -20, MIN),
            Rect::new(130, 80, 600, 60)
        );
        assert_eq!(
            drag_rect(START, Hit::Move, -500, 0, MIN),
            Rect::new(-400, 100, 600, 60),
            "may leave the screen"
        );
    }

    #[test]
    fn resizing_moves_only_the_dragged_edges() {
        let r = |l, rt, t, b| Hit::Resize(edges(l, rt, t, b));
        assert_eq!(
            drag_rect(START, r(false, true, false, false), 50, 99, MIN),
            Rect::new(100, 100, 650, 60)
        );
        assert_eq!(
            drag_rect(START, r(true, false, false, false), -50, 99, MIN),
            Rect::new(50, 100, 650, 60)
        );
        assert_eq!(
            drag_rect(START, r(false, false, false, true), 99, 20, MIN),
            Rect::new(100, 100, 600, 80)
        );
        assert_eq!(
            drag_rect(START, r(false, false, true, false), 99, -20, MIN),
            Rect::new(100, 80, 600, 80)
        );
        // A corner moves two edges at once.
        assert_eq!(
            drag_rect(START, r(true, false, true, false), 10, 10, MIN),
            Rect::new(110, 110, 590, 50)
        );
        assert_eq!(
            drag_rect(START, r(false, true, false, true), 10, 10, MIN),
            Rect::new(100, 100, 610, 70)
        );
    }

    #[test]
    fn resizing_stops_at_the_minimum_with_the_opposite_edge_fixed() {
        let r = |l, rt, t, b| Hit::Resize(edges(l, rt, t, b));
        // Shrinking from the right.
        assert_eq!(
            drag_rect(START, r(false, true, false, false), -1000, 0, MIN),
            Rect::new(100, 100, 160, 60)
        );
        // Shrinking from the left keeps the right edge (700) in place.
        let shrunk = drag_rect(START, r(true, false, false, false), 1000, 0, MIN);
        assert_eq!((shrunk.right(), shrunk.w), (700, 160));
        // Same for the height, from the top (bottom edge 160 stays) and from the bottom.
        let shrunk = drag_rect(START, r(false, false, true, false), 0, 1000, MIN);
        assert_eq!((shrunk.bottom(), shrunk.h), (160, 24));
        assert_eq!(
            drag_rect(START, r(false, false, false, true), 0, -1000, MIN).h,
            24
        );
    }

    #[test]
    fn a_zero_drag_changes_nothing() {
        for hit in [
            Hit::Move,
            Hit::Resize(edges(true, true, true, true)),
            Hit::Resize(edges(false, true, false, false)),
        ] {
            assert_eq!(drag_rect(START, hit, 0, 0, MIN), START);
        }
    }

    // ---- placement on monitors ----

    const PRIMARY: Rect = Rect::new(0, 0, 1920, 1040);
    const SECOND: Rect = Rect::new(1920, 0, 1280, 1024);

    #[test]
    fn default_place_is_top_centre() {
        assert_eq!(
            default_rect(600, 60, PRIMARY, 20),
            Rect::new(660, 20, 600, 60)
        );
        assert_eq!(
            default_rect(600, 60, SECOND, 20),
            Rect::new(2260, 20, 600, 60)
        );
    }

    #[test]
    fn visible_rects_are_left_alone() {
        let inside = Rect::new(300, 200, 600, 60);
        assert_eq!(fit_on_screen(inside, &[PRIMARY], 20), inside);
        // On the second monitor.
        let on_second = Rect::new(2000, 100, 600, 60);
        assert_eq!(fit_on_screen(on_second, &[PRIMARY, SECOND], 20), on_second);
        // Half of it off the edge is still fine.
        let half = Rect::new(-300, 100, 600, 60);
        assert_eq!(fit_on_screen(half, &[PRIMARY], 20), half);
        // Spanning two monitors with most of it on one.
        let span = Rect::new(1700, 100, 600, 60);
        assert_eq!(fit_on_screen(span, &[PRIMARY, SECOND], 20), span);
    }

    #[test]
    fn a_missing_monitor_sends_the_window_to_the_primary() {
        // Saved on the second monitor, which is no longer connected.
        let saved = Rect::new(2400, 300, 600, 60);
        let fitted = fit_on_screen(saved, &[PRIMARY], 20);
        assert_eq!(fitted, Rect::new(660, 20, 600, 60));
        // Far away in negative space (a monitor to the left that is gone).
        assert_eq!(
            fit_on_screen(Rect::new(-3000, 100, 600, 60), &[PRIMARY], 20),
            fitted
        );
    }

    #[test]
    fn barely_visible_rects_are_pulled_onto_the_best_monitor() {
        // Only 100 of 600 px still on the primary monitor.
        let fitted = fit_on_screen(Rect::new(1820, 200, 600, 60), &[PRIMARY], 20);
        assert_eq!(fitted, Rect::new(1320, 200, 600, 60));
        // Mostly below the screen.
        let fitted = fit_on_screen(Rect::new(300, 1030, 600, 60), &[PRIMARY], 20);
        assert_eq!(fitted, Rect::new(300, 980, 600, 60));
        // Above and to the left.
        let fitted = fit_on_screen(Rect::new(-590, -50, 600, 60), &[PRIMARY], 20);
        assert_eq!(fitted, Rect::new(0, 0, 600, 60));
    }

    #[test]
    fn a_rect_larger_than_the_monitor_is_shrunk_to_fit() {
        let fitted = fit_on_screen(Rect::new(-100, -100, 3000, 2000), &[PRIMARY], 20);
        assert_eq!(fitted, Rect::new(0, 0, 1920, 1040));
        let fitted = fit_on_screen(Rect::new(5000, 5000, 3000, 2000), &[PRIMARY], 20);
        assert_eq!(fitted.w, 1920);
        assert!(fitted.h <= 1040);
    }

    #[test]
    fn no_monitor_information_leaves_the_rect_alone() {
        let r = Rect::new(5, 6, 7, 8);
        assert_eq!(fit_on_screen(r, &[], 20), r);
    }
}
