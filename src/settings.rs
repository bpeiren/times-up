//! The Settings window as plain geometry and painting: which sections there are, where their tabs
//! and the content area sit, and the frame drawn around them. The window itself, with the text and
//! the Win32 messages, lives in `win::settings_window`. All sizes are device pixels.

use crate::canvas::{Canvas, RectF};
use crate::config::{Rgba, WindowRect};
use crate::interaction::{Rect, fit_on_screen};
use crate::strings;

/// Logical size (96 DPI) a Settings window starts with, and the smallest it may get.
pub const DEFAULT_SIZE: (i32, i32) = (900, 660);
pub const MIN_SIZE: (i32, i32) = (640, 500);

const TAB_BAR_HEIGHT: f32 = 40.0;
const TAB_WIDTH: f32 = 130.0;
const UNDERLINE_HEIGHT: f32 = 3.0;

const BACKGROUND: Rgba = Rgba::rgb(0x20, 0x21, 0x24);
const TAB_BAR: Rgba = Rgba::rgb(0x2a, 0x2b, 0x2f);

/// A page of the Settings window; the tabs along the top switch between them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Timetable,
    General,
}

impl Section {
    /// In tab order.
    pub const ALL: [Section; 2] = [Section::General, Section::Timetable];

    pub const fn label(self) -> &'static str {
        match self {
            Section::Timetable => strings::SECTION_TIMETABLE,
            Section::General => strings::SECTION_GENERAL,
        }
    }
}

/// Where everything sits in a window of a given client size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub tab_bar: Rect,
    pub tabs: Vec<(Section, Rect)>,
    /// What is left for the selected section.
    pub content: Rect,
}

pub fn layout(width: i32, height: i32, scale: f32) -> Layout {
    let bar_h = (TAB_BAR_HEIGHT * scale).round() as i32;
    let tab_w = (TAB_WIDTH * scale).round() as i32;
    let tabs = Section::ALL
        .iter()
        .zip(0..)
        .map(|(&section, i)| (section, Rect::new(i * tab_w, 0, tab_w, bar_h)))
        .collect();
    Layout {
        tab_bar: Rect::new(0, 0, width, bar_h),
        tabs,
        content: Rect::new(0, bar_h, width, (height - bar_h).max(0)),
    }
}

impl Layout {
    /// The tab under a point of the window, if any.
    pub fn tab_at(&self, x: i32, y: i32) -> Option<Section> {
        self.tabs
            .iter()
            .find(|(_, r)| x >= r.x && x < r.right() && y >= r.y && y < r.bottom())
            .map(|&(section, _)| section)
    }
}

/// The window's size for a display `scale`.
pub fn default_size(scale: f32) -> (i32, i32) {
    (
        (DEFAULT_SIZE.0 as f32 * scale) as i32,
        (DEFAULT_SIZE.1 as f32 * scale) as i32,
    )
}

pub fn min_size(scale: f32) -> (i32, i32) {
    (
        (MIN_SIZE.0 as f32 * scale) as i32,
        (MIN_SIZE.1 as f32 * scale) as i32,
    )
}

/// Where the window goes: the saved rectangle brought onto a connected monitor, or the middle of
/// the primary monitor when none was saved. Sizes that were never saved use the defaults for the
/// display's `scale`; nothing goes below the minimum. `areas` are the monitors' usable areas,
/// primary first.
pub fn placement(saved: &WindowRect, scale: f32, areas: &[Rect]) -> Rect {
    let (min_w, min_h) = min_size(scale);
    let (default_w, default_h) = default_size(scale);
    let width = saved.width.map_or(default_w, |w| w as i32).max(min_w);
    let height = saved.height.map_or(default_h, |h| h as i32).max(min_h);
    match (saved.x, saved.y, areas.first()) {
        (Some(x), Some(y), _) => fit_on_screen(Rect::new(x, y, width, height), areas, 0),
        (_, _, Some(primary)) => {
            let (w, h) = (width.min(primary.w), height.min(primary.h));
            Rect::new(
                primary.x + (primary.w - w) / 2,
                primary.y + (primary.h - h) / 2,
                w,
                h,
            )
        }
        _ => Rect::new(0, 0, width, height),
    }
}

/// Paint the background, the tab bar and the underline of the selected tab (in `accent`). The
/// tab labels are drawn on top by the caller.
pub fn paint_frame(canvas: &mut Canvas, layout: &Layout, selected: Section, accent: Rgba) {
    let (w, h) = (canvas.width() as f32, canvas.height() as f32);
    canvas.fill_rounded_rect(RectF::new(0.0, 0.0, w, h), 0.0, BACKGROUND, None);
    let bar = layout.tab_bar;
    canvas.fill_rounded_rect(
        RectF::new(bar.x as f32, bar.y as f32, bar.w as f32, bar.h as f32),
        0.0,
        TAB_BAR,
        None,
    );
    if let Some((_, tab)) = layout.tabs.iter().find(|(s, _)| *s == selected) {
        let line = (UNDERLINE_HEIGHT * bar.h as f32 / TAB_BAR_HEIGHT).max(1.0);
        canvas.fill_rounded_rect(
            RectF::new(
                tab.x as f32,
                (tab.bottom() as f32) - line,
                tab.w as f32,
                line,
            ),
            0.0,
            accent,
            None,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_are_two_sections_the_timetable_first() {
        assert_eq!(Section::ALL, [Section::General, Section::Timetable]);
        assert_eq!(Section::Timetable.label(), "Timetable");
        assert_eq!(Section::General.label(), "General");
    }

    #[test]
    fn the_tab_bar_spans_the_top_and_the_content_fills_the_rest() {
        let l = layout(800, 500, 1.0);
        assert_eq!(l.tab_bar, Rect::new(0, 0, 800, 40));
        assert_eq!(l.content, Rect::new(0, 40, 800, 460));
        assert_eq!(
            l.tabs,
            vec![
                (Section::General, Rect::new(0, 0, 130, 40)),
                (Section::Timetable, Rect::new(130, 0, 130, 40)),
            ]
        );
    }

    #[test]
    fn the_layout_scales_with_the_display() {
        let l = layout(1600, 1000, 2.0);
        assert_eq!(l.tab_bar.h, 80);
        assert_eq!(l.content, Rect::new(0, 80, 1600, 920));
        assert_eq!(l.tabs[0].1, Rect::new(0, 0, 260, 80));
    }

    #[test]
    fn a_window_shorter_than_the_tab_bar_has_no_content() {
        assert_eq!(layout(300, 10, 1.0).content.h, 0);
    }

    #[test]
    fn tabs_are_hit_inside_their_rectangle_only() {
        let l = layout(800, 500, 1.0);
        assert_eq!(l.tab_at(0, 0), Some(Section::General));
        assert_eq!(l.tab_at(129, 39), Some(Section::General));
        assert_eq!(l.tab_at(130, 10), Some(Section::Timetable));
        assert_eq!(l.tab_at(259, 39), Some(Section::Timetable));
        assert_eq!(l.tab_at(260, 10), None);
        assert_eq!(l.tab_at(10, 40), None);
        assert_eq!(l.tab_at(-1, 10), None);
    }

    #[test]
    fn sizes_follow_the_display_scale() {
        assert_eq!(default_size(1.0), DEFAULT_SIZE);
        assert_eq!(default_size(1.5), (1350, 990));
        assert_eq!(min_size(2.0), (1280, 1000));
    }

    const SCREEN: Rect = Rect::new(0, 0, 1920, 1040);

    fn saved(x: Option<i32>, y: Option<i32>, w: Option<u32>, h: Option<u32>) -> WindowRect {
        WindowRect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    #[test]
    fn without_a_saved_rectangle_the_window_is_centred_on_the_primary_monitor() {
        let r = placement(&WindowRect::default(), 1.0, &[SCREEN]);
        assert_eq!(r, Rect::new(510, 190, 900, 660));
        let r = placement(&WindowRect::default(), 2.0, &[SCREEN]);
        assert_eq!((r.w, r.h), (1800, 1040), "as big as the monitor allows");
        assert_eq!((r.x, r.y), (60, 0));
    }

    #[test]
    fn a_saved_rectangle_is_kept_when_it_is_visible() {
        let w = saved(Some(100), Some(50), Some(700), Some(520));
        assert_eq!(placement(&w, 1.0, &[SCREEN]), Rect::new(100, 50, 700, 520));
    }

    #[test]
    fn a_saved_rectangle_on_a_missing_monitor_is_pulled_back() {
        let w = saved(Some(5000), Some(5000), Some(700), Some(520));
        let r = placement(&w, 1.0, &[SCREEN]);
        assert!(
            r.overlap_area(&SCREEN) == r.area(),
            "{r:?} is fully visible"
        );
        let off = saved(Some(1800), Some(50), Some(700), Some(520));
        let r = placement(&off, 1.0, &[SCREEN]);
        assert_eq!(r, Rect::new(1220, 50, 700, 520));
    }

    #[test]
    fn a_saved_size_below_the_minimum_is_raised_and_missing_parts_default() {
        let w = saved(Some(10), Some(10), Some(100), None);
        let r = placement(&w, 1.0, &[SCREEN]);
        assert_eq!((r.w, r.h), (MIN_SIZE.0, DEFAULT_SIZE.1));
        let no_monitor = placement(&WindowRect::default(), 1.0, &[]);
        assert_eq!(no_monitor, Rect::new(0, 0, 900, 660));
    }

    #[test]
    fn the_frame_is_opaque_with_an_accent_under_the_selected_tab() {
        let accent = Rgba::rgb(10, 200, 30);
        let l = layout(400, 200, 1.0);
        let mut canvas = Canvas::new(400, 200);
        paint_frame(&mut canvas, &l, Section::General, accent);
        assert!(canvas.pixels().iter().all(|p| p >> 24 == 0xff));
        assert_eq!(canvas.pixel(50, 38), 0xff0a_c81e, "underline of the tab");
        assert_ne!(canvas.pixel(200, 38), canvas.pixel(50, 38), "only that tab");
        assert_ne!(
            canvas.pixel(200, 10),
            canvas.pixel(200, 100),
            "bar vs content"
        );
    }
}
