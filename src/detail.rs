//! The detail bar of the Settings window: the selected class drawn large, like the live bar, with
//! its break and its orange and red zones draggable (the rules are those of [`editing`]), plus
//! the Add break and Remove break buttons. Plain Rust without Win32, like [`crate::timetable`]:
//! the window feeds it pointer positions and draws the text through a callback.
//!
//! A drag changes a preview only; the day is saved through the store when the button is released.

use crate::bar;
use crate::canvas::{Align, Canvas, RectF};
use crate::config::{Colors, Rgba};
use crate::editing::{self, Handle, MIN_BLOCK, ZoneEdge};
use crate::interaction::{CursorShape, Rect};
use crate::schedule::{Break, Class, MINUTE, Secs, Weekday, format_clock};
use crate::store::Store;
use crate::strings;

// Logical sizes at 96 DPI.
const HEIGHT: f32 = 172.0;
const SIDE: f32 = 12.0;
const TITLE_H: f32 = 36.0;
const BAR_H: f32 = 56.0;
const INFO_H: f32 = 32.0;
const BUTTON_W: f32 = 116.0;
const BUTTON_H: f32 = 30.0;
const GAP: f32 = 8.0;
/// How close, as a share of the bar height (at least 4 px), the pointer must be to a handle.
const GRIP_SCALE: f32 = 0.15;

const BUTTON: Rgba = Rgba::rgb(0x3a, 0x3c, 0x42);
const DIM: Rgba = Rgba {
    r: 255,
    g: 255,
    b: 255,
    a: 90,
};
const TICK: Rgba = Rgba {
    r: 255,
    g: 255,
    b: 255,
    a: 150,
};
const SHADE: Rgba = Rgba {
    r: 255,
    g: 255,
    b: 255,
    a: 110,
};
const NO_FIT: Rgba = Rgba {
    r: 255,
    g: 60,
    b: 60,
    a: 170,
};
const ERROR: Rgba = Rgba::rgb(0xff, 0x8a, 0x80);

fn px(v: f32, scale: f32) -> i32 {
    (v * scale).round() as i32
}

/// Split the content of the Settings window into the area for the week and the area for the
/// detail bar below it.
pub fn split(content: Rect, scale: f32) -> (Rect, Rect) {
    let detail_h = px(HEIGHT, scale).min(content.h / 2);
    (
        Rect::new(content.x, content.y, content.w, content.h - detail_h),
        Rect::new(content.x, content.bottom() - detail_h, content.w, detail_h),
    )
}

/// Where everything sits in the detail area.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub area: Rect,
    pub title: Rect,
    pub add: Rect,
    pub remove: Rect,
    pub bar: Rect,
    pub info: Rect,
}

pub fn layout(area: Rect, scale: f32) -> Layout {
    let side = px(SIDE, scale);
    let (title_h, bar_h, info_h) = (px(TITLE_H, scale), px(BAR_H, scale), px(INFO_H, scale));
    let (button_w, button_h, gap) = (px(BUTTON_W, scale), px(BUTTON_H, scale), px(GAP, scale));
    let inner = (area.w - 2 * side).max(1);
    let button_y = area.y + (title_h - button_h) / 2;
    let remove = Rect::new(area.right() - side - button_w, button_y, button_w, button_h);
    let add = Rect::new(remove.x - gap - button_w, button_y, button_w, button_h);
    Layout {
        area,
        title: Rect::new(
            area.x + side,
            area.y,
            (add.x - gap - area.x - side).max(1),
            title_h,
        ),
        add,
        remove,
        bar: Rect::new(area.x + side, area.y + title_h, inner, bar_h),
        info: Rect::new(area.x + side, area.y + title_h + bar_h, inner, info_h),
    }
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.x && x < r.right() && y >= r.y && y < r.bottom()
}

/// What is under the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Nothing,
    AddBreak,
    RemoveBreak,
    Bar,
}

/// Whether a break fits in `class`: both blocks keep at least the minimum length.
pub fn break_fits(class: &Class, brk: &Break) -> bool {
    brk.len > 0 && brk.start >= MIN_BLOCK && brk.end() + MIN_BLOCK <= class.duration()
}

/// The break the Add break mode shows when the pointer is `offset` seconds into the class: its
/// middle at the pointer, to the whole minute.
pub fn proposed_break(offset: i64, len: Secs) -> Break {
    let minute = i64::from(MINUTE);
    let left = offset - i64::from(len) / 2;
    let start = ((left + minute / 2).div_euclid(minute) * minute).clamp(0, i64::from(u32::MAX));
    Break {
        start: start as Secs,
        len,
    }
}

/// `45 min`, or `1 min 30 s` for a time that is not a whole number of minutes.
fn format_length(secs: Secs) -> String {
    match (secs / MINUTE, secs % MINUTE) {
        (m, 0) => format!("{m} min"),
        (0, s) => format!("{s} s"),
        (m, s) => format!("{m} min {s} s"),
    }
}

fn span_text(class: &Class, from: Secs, to: Secs) -> String {
    format!(
        "{}\u{2013}{}  ({})",
        format_clock(class.start + from),
        format_clock(class.start + to),
        format_length(to - from)
    )
}

/// What the tooltip under the bar says for the point `offset` seconds into the class.
fn tip(class: &Class, offset: Secs, handle: Option<Handle>) -> Option<String> {
    if let Some(Handle::Zone { edge, .. }) = handle
        && let Some(label) = editing::describe(class, handle?)
    {
        let name = match edge {
            ZoneEdge::Orange => strings::TIP_ORANGE,
            ZoneEdge::Red => strings::TIP_RED,
        };
        return Some(format!("{name} {} {}", strings::TIP_FROM, label.text));
    }
    if let Some(b) = class.brk
        && offset >= b.start
        && offset < b.end()
    {
        return Some(format!(
            "{}  {}",
            strings::TIP_BREAK,
            span_text(class, b.start, b.end())
        ));
    }
    let blocks = class.block_count();
    let index = (0..blocks).find(|&i| {
        let b = class.block(i);
        offset >= b.start && offset < b.end
    })?;
    let b = class.block(index);
    let name = if blocks == 1 {
        strings::TIP_CLASS.to_string()
    } else {
        format!("{} {}", strings::TIP_BLOCK, index + 1)
    };
    Some(format!("{name}  {}", span_text(class, b.start, b.end)))
}

type Key = (Weekday, Secs, Secs);

/// A handle being dragged.
struct Grab {
    day: Weekday,
    index: usize,
    handle: Handle,
    /// The class as it was when the drag started; every move is computed from this.
    start: Class,
    press_x: i32,
}

#[derive(Default)]
pub struct Detail {
    grab: Option<Grab>,
    preview: Option<Class>,
    /// The Add break mode is on for this class.
    adding: Option<Key>,
    pointer: Option<(i32, i32)>,
    error: Option<String>,
}

type Selected<'a> = Option<(Weekday, usize, &'a Class)>;

/// The selected class with its day and index, from the store.
pub fn selected_class(store: &Store, selection: Option<(Weekday, usize)>) -> Selected<'_> {
    let (day, index) = selection?;
    let class = store.schedule().classes(day).get(index)?;
    Some((day, index, class))
}

impl Detail {
    pub fn is_dragging(&self) -> bool {
        self.grab.is_some()
    }

    /// Whether the Add break mode is on.
    pub fn is_adding(&self) -> bool {
        self.adding.is_some()
    }

    fn key(day: Weekday, class: &Class) -> Key {
        (day, class.start, class.end)
    }

    fn adding_for(&self, day: Weekday, class: &Class) -> bool {
        self.adding == Some(Self::key(day, class))
    }

    fn grip(layout: &Layout) -> f32 {
        (layout.bar.h as f32 * GRIP_SCALE).max(4.0)
    }

    /// Seconds into `class` for a pixel column of the bar (not limited to the bar).
    fn offset_at(layout: &Layout, class: &Class, x: i32) -> i64 {
        let along = i64::from(x - layout.bar.x) * i64::from(class.duration());
        along.div_euclid(i64::from(layout.bar.w.max(1)))
    }

    pub fn target(&self, layout: &Layout, x: i32, y: i32) -> Target {
        if inside(layout.add, x, y) {
            Target::AddBreak
        } else if inside(layout.remove, x, y) {
            Target::RemoveBreak
        } else if inside(layout.bar, x, y) {
            Target::Bar
        } else {
            Target::Nothing
        }
    }

    fn handle_at(layout: &Layout, class: &Class, x: i32) -> Option<Handle> {
        editing::handle_at(
            class,
            (x - layout.bar.x) as f32,
            layout.bar.w as f32,
            Self::grip(layout),
        )
    }

    pub fn cursor(
        &self,
        store: &Store,
        selection: Option<(Weekday, usize)>,
        layout: &Layout,
        x: i32,
        y: i32,
    ) -> CursorShape {
        let Some((day, _, class)) = selected_class(store, selection) else {
            return CursorShape::Arrow;
        };
        if let Some(grab) = &self.grab {
            return handle_cursor(grab.handle);
        }
        if self.target(layout, x, y) != Target::Bar {
            return CursorShape::Arrow;
        }
        if self.adding_for(day, class) {
            return CursorShape::SizeWestEast;
        }
        Self::handle_at(layout, class, x).map_or(CursorShape::Arrow, handle_cursor)
    }

    /// The button went down at `(x, y)`: press a button, pick up a handle or place the break.
    pub fn press(
        &mut self,
        store: &mut Store,
        selection: Option<(Weekday, usize)>,
        layout: &Layout,
        x: i32,
        y: i32,
    ) {
        let Some((day, index, class)) = selected_class(store, selection) else {
            return;
        };
        let class = class.clone();
        match self.target(layout, x, y) {
            Target::AddBreak if class.brk.is_none() => {
                let key = Self::key(day, &class);
                self.adding = (self.adding != Some(key)).then_some(key);
            }
            Target::RemoveBreak if class.brk.is_some() => {
                self.adding = None;
                let mut without = class;
                without.brk = None;
                self.save(store, day, index, without);
            }
            Target::Bar if self.adding_for(day, &class) => {
                let len = store.config().default_break;
                let brk = proposed_break(Self::offset_at(layout, &class, x), len);
                if break_fits(&class, &brk)
                    && let Ok(with) = class.clone().with_break(Some(brk))
                {
                    self.adding = None;
                    self.save(store, day, index, with);
                }
            }
            Target::Bar => {
                if let Some(handle) = Self::handle_at(layout, &class, x) {
                    self.grab = Some(Grab {
                        day,
                        index,
                        handle,
                        start: class,
                        press_x: x,
                    });
                }
            }
            _ => {}
        }
    }

    /// The pointer moved. Returns whether the detail bar has to be repainted.
    pub fn moved(&mut self, layout: &Layout, x: i32, y: i32) -> bool {
        let before = self.pointer;
        self.pointer = Some((x, y));
        let Some(grab) = &self.grab else {
            let near = |p: Option<(i32, i32)>| p.is_some_and(|(px, py)| inside(layout.bar, px, py));
            return near(before) || near(self.pointer);
        };
        let width = f64::from(layout.bar.w.max(1));
        let delta = f64::from(x - grab.press_x) / width * f64::from(grab.start.duration());
        self.preview = Some(editing::drag(
            &grab.start,
            grab.handle,
            delta.round() as i64,
        ));
        true
    }

    /// The button went up: save what the drag made, if it changed anything.
    pub fn release(&mut self, store: &mut Store) {
        let (grab, preview) = (self.grab.take(), self.preview.take());
        let (Some(grab), Some(preview)) = (grab, preview) else {
            return;
        };
        if preview != grab.start {
            self.save(store, grab.day, grab.index, preview);
        }
    }

    /// Give up a drag without saving (the mouse capture was lost).
    pub fn cancel(&mut self) {
        self.grab = None;
        self.preview = None;
    }

    /// Keep the Add break mode only while the class it was started for is still the selected one:
    /// selecting something else ends it, and selecting the class again does not bring it back.
    pub fn sync(&mut self, store: &Store, selection: Option<(Weekday, usize)>) {
        let key = selected_class(store, selection).map(|(day, _, class)| Self::key(day, class));
        if self.adding != key {
            self.adding = None;
        }
    }

    /// Escape: leave the Add break mode, or give up a drag. Returns whether it did anything.
    pub fn escape(&mut self) -> bool {
        let used = self.adding.is_some() || self.grab.is_some();
        self.adding = None;
        self.cancel();
        used
    }

    fn save(&mut self, store: &mut Store, day: Weekday, index: usize, class: Class) {
        let mut classes = store.schedule().classes(day).to_vec();
        classes[index] = class;
        self.error = store.set_day(day, classes).err().map(|e| e.to_string());
    }

    /// Paint the detail bar. `text` draws a string in an area with an alignment and a colour.
    pub fn paint(
        &self,
        canvas: &mut Canvas,
        store: &Store,
        selection: Option<(Weekday, usize)>,
        layout: &Layout,
        text: &mut dyn FnMut(&mut Canvas, &str, Rect, Rgba, Align),
    ) {
        let colors = store.config().colors;
        let Some((day, _, stored)) = selected_class(store, selection) else {
            let hint = Rect::new(layout.bar.x, layout.bar.y, layout.bar.w, layout.bar.h);
            text(canvas, strings::DETAIL_HINT, hint, DIM, Align::Center);
            fill(canvas, layout.add, BUTTON, 6.0);
            fill(canvas, layout.remove, BUTTON, 6.0);
            text(
                canvas,
                strings::BUTTON_ADD_BREAK,
                layout.add,
                DIM,
                Align::Center,
            );
            text(
                canvas,
                strings::BUTTON_REMOVE_BREAK,
                layout.remove,
                DIM,
                Align::Center,
            );
            return;
        };
        let class = self.preview.as_ref().unwrap_or(stored);
        let adding = self.adding_for(day, class);

        let title = format!(
            "{}  {}\u{2013}{}  ({})",
            strings::DAY_NAMES[day.index()],
            format_clock(class.start),
            format_clock(class.end),
            format_length(class.duration())
        );
        text(canvas, &title, layout.title, colors.text, Align::Left);
        let (can_add, can_remove) = (class.brk.is_none(), class.brk.is_some());
        let on = |enabled: bool| if enabled { colors.text } else { DIM };
        fill(canvas, layout.add, BUTTON, 6.0);
        text(
            canvas,
            strings::BUTTON_ADD_BREAK,
            layout.add,
            on(can_add),
            Align::Center,
        );
        fill(canvas, layout.remove, BUTTON, 6.0);
        text(
            canvas,
            strings::BUTTON_REMOVE_BREAK,
            layout.remove,
            on(can_remove),
            Align::Center,
        );

        self.paint_bar(canvas, store, class, adding, layout, &colors);
        let (info, color) = self.info(store, class, adding, layout, &colors);
        text(canvas, &info, layout.info, color, Align::Left);
    }

    fn paint_bar(
        &self,
        canvas: &mut Canvas,
        store: &Store,
        class: &Class,
        adding: bool,
        layout: &Layout,
        colors: &Colors,
    ) {
        let bar_rect = layout.bar;
        let (w, h) = (bar_rect.w.max(1) as u32, bar_rect.h.max(1) as u32);
        let mut bar_canvas = Canvas::new(w, h);
        let view = bar::class_view(class);
        bar::paint(&mut bar_canvas, &view, colors);

        // Where the handles are, so it is clear what can be dragged.
        let tick = (h as f32 / 28.0).max(1.0);
        for seg in &view.segments {
            let marks = match seg.kind {
                bar::SegmentKind::Break => vec![seg.start, seg.end],
                bar::SegmentKind::Block { orange_at, red_at } => vec![orange_at, red_at],
            };
            for at in marks.into_iter().filter(|&at| at > 0.0 && at < 1.0) {
                let x = at * w as f32 - tick / 2.0;
                let rect = RectF::new(x, h as f32 * 0.25, tick, h as f32 * 0.5);
                bar_canvas.fill_rounded_rect(rect, 0.0, TICK, None);
            }
        }

        if adding
            && let Some((px, py)) = self.pointer
            && inside(bar_rect, px, py)
        {
            let brk = proposed_break(
                Self::offset_at(layout, class, px),
                store.config().default_break,
            );
            let per_px = w as f32 / class.duration().max(1) as f32;
            let rect = RectF::new(
                brk.start as f32 * per_px,
                0.0,
                brk.len as f32 * per_px,
                h as f32,
            );
            let color = if break_fits(class, &brk) {
                SHADE
            } else {
                NO_FIT
            };
            bar_canvas.fill_rounded_rect(rect, 0.0, color, None);
        }
        canvas.composite(&bar_canvas, bar_rect.x, bar_rect.y);
    }

    /// The line under the bar and its colour.
    fn info(
        &self,
        store: &Store,
        class: &Class,
        adding: bool,
        layout: &Layout,
        colors: &Colors,
    ) -> (String, Rgba) {
        if let Some(error) = &self.error {
            return (error.clone(), ERROR);
        }
        if let (Some(grab), Some(_)) = (&self.grab, &self.preview) {
            let label = editing::describe(class, grab.handle);
            return (label.map_or_else(String::new, |l| l.text), colors.text);
        }
        let Some((x, _)) = self.pointer.filter(|&(x, y)| inside(layout.bar, x, y)) else {
            let idle = if adding {
                strings::TIP_ADDING
            } else {
                strings::TIP_IDLE
            };
            return (idle.to_string(), DIM);
        };
        let offset = Self::offset_at(layout, class, x);
        if adding {
            let brk = proposed_break(offset, store.config().default_break);
            let fits = break_fits(class, &brk);
            let text = format!(
                "{}\u{2013}{}",
                format_clock(class.start + brk.start),
                format_clock(class.start + brk.end())
            );
            return (text, if fits { colors.text } else { colors.red });
        }
        let handle = Self::handle_at(layout, class, x);
        let secs = offset.clamp(0, i64::from(class.duration())) as Secs;
        (tip(class, secs, handle).unwrap_or_default(), colors.text)
    }
}

fn handle_cursor(handle: Handle) -> CursorShape {
    match handle {
        Handle::BreakBody => CursorShape::SizeAll,
        _ => CursorShape::SizeWestEast,
    }
}

fn fill(canvas: &mut Canvas, r: Rect, color: Rgba, radius: f32) {
    canvas.fill_rounded_rect(
        RectF::new(r.x as f32, r.y as f32, r.w as f32, r.h as f32),
        radius,
        color,
        None,
    );
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::config::{self, ConfigFile};
    use crate::schedule::{DEFAULT_BREAK_LEN, hm};

    const MON: Weekday = Weekday::Mon;
    const SELECTED: Option<(Weekday, usize)> = Some((MON, 0));
    const AREA: Rect = Rect::new(0, 400, 860, 172);

    /// A scratch config with a store on it, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str, start: &str, end: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("times-up-detail-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            let text =
                format!("[monday]\nclasses = [{{ start = \"{start}\", end = \"{end}\" }}]\n");
            fs::write(dir.join(config::FILE_NAME), text).unwrap();
            Self(dir)
        }

        fn store(&self) -> Store {
            let mut file = ConfigFile::new(self.0.join(config::FILE_NAME));
            let config = file.load_or_create().config;
            Store::new(config, Some(file))
        }

        fn text(&self) -> String {
            fs::read_to_string(self.0.join(config::FILE_NAME)).unwrap()
        }

        fn reloaded(&self) -> Class {
            config::parse(&self.text()).config.schedule.classes(MON)[0].clone()
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn lay() -> Layout {
        layout(AREA, 1.0)
    }

    /// The pixel column of `offset` seconds into the class.
    fn col(l: &Layout, class: &Class, offset: Secs) -> i32 {
        l.bar.x + (i64::from(offset) * i64::from(l.bar.w) / i64::from(class.duration())) as i32
    }

    fn mid_y(l: &Layout) -> i32 {
        l.bar.y + l.bar.h / 2
    }

    fn class_of(store: &Store) -> Class {
        store.schedule().classes(MON)[0].clone()
    }

    // ---- geometry and rules ----

    #[test]
    fn the_detail_area_is_the_bottom_of_the_content() {
        let content = Rect::new(0, 40, 860, 520);
        let (week, detail) = split(content, 1.0);
        assert_eq!(detail, Rect::new(0, 388, 860, 172));
        assert_eq!(week, Rect::new(0, 40, 860, 348));
        let (week, detail) = split(Rect::new(0, 0, 800, 100), 1.0);
        assert_eq!((week.h, detail.h), (50, 50), "never more than half");
    }

    #[test]
    fn the_bar_sits_under_the_title_and_above_the_tooltip() {
        let l = lay();
        assert!(l.title.bottom() <= l.bar.y && l.bar.bottom() <= l.info.y);
        assert!(l.info.bottom() <= AREA.bottom());
        assert!(l.add.right() < l.remove.x && l.remove.right() <= AREA.right());
        assert!(l.title.right() <= l.add.x);
        assert_eq!(l.bar.w, AREA.w - 24);
    }

    #[test]
    fn a_break_has_to_leave_five_minutes_on_both_sides() {
        let class = Class::new(hm(9, 0), hm(10, 40), DEFAULT_BREAK_LEN).unwrap();
        let min = 5 * 60;
        let ten = DEFAULT_BREAK_LEN;
        let at = |start| break_fits(&class, &Break { start, len: ten });
        assert!(at(min) && at(class.duration() - ten - min));
        assert!(!at(min - 60) && !at(class.duration() - ten - min + 60));
        assert!(!break_fits(&class, &Break { start: 600, len: 0 }));
    }

    #[test]
    fn the_proposed_break_is_centred_on_the_pointer_to_the_whole_minute() {
        assert_eq!(
            proposed_break(45 * 60 + 20, 600),
            Break {
                start: 40 * 60,
                len: 600
            }
        );
        assert_eq!(
            proposed_break(45 * 60 + 40, 600),
            Break {
                start: 41 * 60,
                len: 600
            }
        );
        assert_eq!(proposed_break(-500, 600).start, 0);
        assert_eq!(
            proposed_break(2 * 60, 600).start,
            0,
            "never before the class"
        );
    }

    #[test]
    fn tooltips_name_the_block_the_break_or_the_zone() {
        let class = Class::new(hm(9, 0), hm(10, 40), DEFAULT_BREAK_LEN).unwrap();
        let min = |m: Secs| m * 60;
        assert_eq!(
            tip(&class, min(10), None).unwrap(),
            "Block 1  09:00\u{2013}09:45  (45 min)"
        );
        assert_eq!(
            tip(&class, min(50), None).unwrap(),
            "Break  09:45\u{2013}09:55  (10 min)"
        );
        assert_eq!(
            tip(&class, min(70), None).unwrap(),
            "Block 2  09:55\u{2013}10:40  (45 min)"
        );
        let single = Class::new(hm(9, 0), hm(9, 50), DEFAULT_BREAK_LEN).unwrap();
        assert_eq!(
            tip(&single, min(10), None).unwrap(),
            "Class  09:00\u{2013}09:50  (50 min)"
        );
        let zone = Handle::Zone {
            block: 0,
            edge: ZoneEdge::Orange,
        };
        assert_eq!(
            tip(&class, min(35), Some(zone)).unwrap(),
            "Orange from 09:35"
        );
        assert_eq!(format_length(90), "1 min 30 s");
        assert_eq!(format_length(30), "30 s");
    }

    // ---- dragging handles ----

    #[test]
    fn dragging_a_break_edge_changes_the_break_and_saves_on_release() {
        let dir = Scratch::new("break-edge", "09:00", "10:40");
        let mut store = dir.store();
        let mut view = Detail::default();
        let l = lay();
        let class = class_of(&store);
        let y = mid_y(&l);
        let x = col(&l, &class, 45 * 60);
        view.press(&mut store, SELECTED, &l, x, y);
        assert!(view.is_dragging());
        let later = col(&l, &class, 50 * 60);
        assert!(view.moved(&l, later, y));
        assert_eq!(store.revision(), 0, "nothing is saved mid-drag");
        view.release(&mut store);

        assert!(!view.is_dragging());
        assert_eq!(store.revision(), 1);
        let brk = class_of(&store).brk.unwrap();
        assert!(brk.start > 49 * 60 && brk.start < 51 * 60, "{brk:?}");
        assert_eq!(brk.end(), 55 * 60, "the other edge stays");
        assert_eq!(dir.reloaded().brk, Some(brk), "and it is in the file");
    }

    #[test]
    fn dragging_a_zone_line_changes_the_warning_time() {
        let dir = Scratch::new("zone", "09:00", "09:50");
        let mut store = dir.store();
        let mut view = Detail::default();
        let l = lay();
        let class = class_of(&store);
        let y = mid_y(&l);
        // The orange zone starts 10 minutes before the end of the block.
        view.press(&mut store, SELECTED, &l, col(&l, &class, 40 * 60), y);
        assert!(view.is_dragging());
        view.moved(&l, col(&l, &class, 35 * 60), y);
        view.release(&mut store);
        let w = class_of(&store).warnings[0];
        assert!(w.orange > 14 * 60 && w.orange < 16 * 60, "{w:?}");
        assert_eq!(dir.reloaded().warnings[0], w);
    }

    #[test]
    fn a_drag_that_ends_where_it_began_saves_nothing() {
        let dir = Scratch::new("same", "09:00", "10:40");
        let mut store = dir.store();
        let mut view = Detail::default();
        let l = lay();
        let class = class_of(&store);
        let (x, y) = (col(&l, &class, 45 * 60), mid_y(&l));
        view.press(&mut store, SELECTED, &l, x, y);
        view.moved(&l, x + 40, y);
        view.moved(&l, x, y);
        view.release(&mut store);
        assert_eq!(store.revision(), 0);
    }

    #[test]
    fn losing_the_capture_or_pressing_escape_gives_a_drag_up() {
        let dir = Scratch::new("cancel", "09:00", "10:40");
        let mut store = dir.store();
        let mut view = Detail::default();
        let l = lay();
        let class = class_of(&store);
        let (x, y) = (col(&l, &class, 45 * 60), mid_y(&l));
        view.press(&mut store, SELECTED, &l, x, y);
        view.moved(&l, x + 60, y);
        view.cancel();
        view.release(&mut store);
        assert_eq!(store.revision(), 0);
        view.press(&mut store, SELECTED, &l, x, y);
        assert!(view.escape());
        assert!(!view.is_dragging());
        assert!(!view.escape(), "nothing left to cancel");
    }

    #[test]
    fn pressing_the_bar_away_from_any_handle_or_without_a_class_does_nothing() {
        let dir = Scratch::new("nothing", "09:00", "09:50");
        let mut store = dir.store();
        let mut view = Detail::default();
        let l = lay();
        let class = class_of(&store);
        view.press(&mut store, SELECTED, &l, col(&l, &class, 5 * 60), mid_y(&l));
        assert!(!view.is_dragging());
        view.press(&mut store, None, &l, l.bar.x + 3, mid_y(&l));
        view.press(&mut store, SELECTED, &l, 2, 2);
        assert_eq!(store.revision(), 0);
    }

    // ---- adding and removing a break ----

    #[test]
    fn add_break_places_the_break_where_you_click_and_ends_the_mode() {
        let dir = Scratch::new("add", "09:00", "09:50");
        let mut store = dir.store();
        let mut view = Detail::default();
        let l = lay();
        let class = class_of(&store);
        assert_eq!(class.brk, None);
        view.press(&mut store, SELECTED, &l, l.add.x + 3, l.add.y + 3);
        assert!(view.is_adding());
        let x = col(&l, &class, 20 * 60);
        let y = mid_y(&l);
        assert_eq!(
            view.cursor(&store, SELECTED, &l, x, y),
            CursorShape::SizeWestEast
        );
        view.moved(&l, x, y);
        view.press(&mut store, SELECTED, &l, x, y);

        assert!(!view.is_adding(), "one click ends the mode");
        let brk = class_of(&store).brk.unwrap();
        assert_eq!(brk.len, store.config().default_break);
        assert!(
            brk.start > 14 * 60 && brk.start < 16 * 60,
            "centred on the click: {brk:?}"
        );
        assert_eq!(dir.reloaded().brk, Some(brk));
        assert_eq!(store.revision(), 1);
    }

    #[test]
    fn a_break_that_does_not_fit_is_not_placed_and_escape_cancels_the_mode() {
        let dir = Scratch::new("nofit", "09:00", "09:50");
        let mut store = dir.store();
        let mut view = Detail::default();
        let l = lay();
        let class = class_of(&store);
        view.press(&mut store, SELECTED, &l, l.add.x + 3, l.add.y + 3);
        let y = mid_y(&l);
        view.press(&mut store, SELECTED, &l, col(&l, &class, 2 * 60), y);
        view.press(&mut store, SELECTED, &l, col(&l, &class, 48 * 60), y);
        assert!(view.is_adding(), "still looking for a place");
        assert_eq!(class_of(&store).brk, None);
        assert_eq!(store.revision(), 0);
        assert!(view.escape());
        assert!(!view.is_adding());
        assert_eq!(
            view.cursor(&store, SELECTED, &l, col(&l, &class, 20 * 60), y),
            CursorShape::Arrow
        );
    }

    #[test]
    fn the_mode_ends_when_another_class_or_none_is_selected_and_does_not_come_back() {
        let dir = Scratch::new("sync", "09:00", "09:50");
        let mut store = dir.store();
        let mut view = Detail::default();
        let l = lay();
        view.press(&mut store, SELECTED, &l, l.add.x + 3, l.add.y + 3);
        assert!(view.is_adding());

        view.sync(&store, SELECTED);
        assert!(view.is_adding(), "same class: nothing changes");
        view.sync(&store, None);
        assert!(!view.is_adding(), "deselected");
        view.sync(&store, SELECTED);
        assert!(
            !view.is_adding(),
            "and selecting it again does not turn it back on"
        );
        let class = class_of(&store);
        assert_eq!(
            view.cursor(&store, SELECTED, &l, col(&l, &class, 20 * 60), mid_y(&l)),
            CursorShape::Arrow
        );
    }

    #[test]
    fn the_add_break_button_toggles_the_mode_and_needs_a_class_without_a_break() {
        let dir = Scratch::new("toggle", "09:00", "09:50");
        let mut store = dir.store();
        let mut view = Detail::default();
        let l = lay();
        view.press(&mut store, SELECTED, &l, l.add.x + 3, l.add.y + 3);
        assert!(view.is_adding());
        view.press(&mut store, SELECTED, &l, l.add.x + 3, l.add.y + 3);
        assert!(!view.is_adding());

        let long = Scratch::new("toggle-long", "09:00", "10:40");
        let mut store = long.store();
        view.press(&mut store, SELECTED, &l, l.add.x + 3, l.add.y + 3);
        assert!(!view.is_adding(), "this class has a break already");
    }

    #[test]
    fn remove_break_stores_a_zero_length_break() {
        let dir = Scratch::new("remove", "09:00", "10:40");
        let mut store = dir.store();
        let mut view = Detail::default();
        let l = lay();
        assert!(class_of(&store).brk.is_some());
        view.press(&mut store, SELECTED, &l, l.remove.x + 3, l.remove.y + 3);
        assert_eq!(class_of(&store).brk, None);
        assert_eq!(store.revision(), 1);
        assert!(dir.text().contains("break_len = 0"), "{}", dir.text());
        assert_eq!(dir.reloaded().brk, None);

        // Nothing to remove the second time.
        view.press(&mut store, SELECTED, &l, l.remove.x + 3, l.remove.y + 3);
        assert_eq!(store.revision(), 1);
    }

    #[test]
    fn a_break_that_is_removed_and_added_again_round_trips() {
        let dir = Scratch::new("again", "09:00", "10:40");
        let mut store = dir.store();
        let mut view = Detail::default();
        let l = lay();
        view.press(&mut store, SELECTED, &l, l.remove.x + 3, l.remove.y + 3);
        view.press(&mut store, SELECTED, &l, l.add.x + 3, l.add.y + 3);
        let class = class_of(&store);
        view.press(
            &mut store,
            SELECTED,
            &l,
            col(&l, &class, 30 * 60),
            mid_y(&l),
        );
        assert_eq!(dir.reloaded(), class_of(&store));
    }

    #[test]
    fn a_failed_save_is_shown_and_changes_nothing() {
        let dir = Scratch::new("fail", "09:00", "10:40");
        let mut store = dir.store();
        let mut view = Detail::default();
        let l = lay();
        fs::write(dir.0.join(config::FILE_NAME), "this is [not toml").unwrap();
        view.press(&mut store, SELECTED, &l, l.remove.x + 3, l.remove.y + 3);
        assert!(class_of(&store).brk.is_some());
        let mut drawn = Vec::new();
        let mut canvas = Canvas::new(860, 600);
        view.paint(&mut canvas, &store, SELECTED, &l, &mut |_, t, _, c, _| {
            drawn.push((t.to_string(), c));
        });
        assert!(
            drawn
                .iter()
                .any(|(t, c)| t.contains("not valid") && *c == ERROR),
            "{drawn:?}"
        );
    }

    // ---- the cursor ----

    #[test]
    fn the_cursor_shows_what_can_be_dragged() {
        let dir = Scratch::new("cursor", "09:00", "10:40");
        let store = dir.store();
        let view = Detail::default();
        let l = lay();
        let class = class_of(&store);
        let y = mid_y(&l);
        let at = |offset| view.cursor(&store, SELECTED, &l, col(&l, &class, offset), y);
        assert_eq!(at(45 * 60), CursorShape::SizeWestEast, "break start");
        assert_eq!(at(50 * 60), CursorShape::SizeAll, "break body");
        assert_eq!(at(10 * 60), CursorShape::Arrow, "plain block");
        assert_eq!(
            view.cursor(&store, None, &l, l.bar.x + 3, y),
            CursorShape::Arrow
        );
        assert_eq!(view.cursor(&store, SELECTED, &l, 2, 2), CursorShape::Arrow);
    }

    // ---- painting ----

    fn paint(
        view: &Detail,
        store: &Store,
        selection: Option<(Weekday, usize)>,
    ) -> (Canvas, Vec<(String, Rgba, Align)>) {
        let mut canvas = Canvas::new(860, 600);
        let mut drawn = Vec::new();
        view.paint(
            &mut canvas,
            store,
            selection,
            &lay(),
            &mut |_, t, _, c, a| {
                drawn.push((t.to_string(), c, a));
            },
        );
        (canvas, drawn)
    }

    fn argb(c: Rgba) -> u32 {
        u32::from_be_bytes([c.a, c.r, c.g, c.b])
    }

    #[test]
    fn the_selected_class_is_drawn_like_the_live_bar() {
        let dir = Scratch::new("paint", "09:00", "10:40");
        let store = dir.store();
        let view = Detail::default();
        let l = lay();
        let class = class_of(&store);
        let (canvas, drawn) = paint(&view, &store, SELECTED);
        let colors = store.config().colors;
        let pixel = |offset| canvas.pixel(col(&l, &class, offset) as u32, mid_y(&l) as u32);
        assert_eq!(pixel(10 * 60), argb(colors.green), "first block");
        assert_eq!(pixel(50 * 60), argb(colors.break_gap), "the break");
        assert_eq!(
            pixel(43 * 60),
            argb(colors.red),
            "the last minutes of block 1"
        );
        let texts: Vec<&str> = drawn.iter().map(|(t, _, _)| t.as_str()).collect();
        assert!(
            texts.contains(&"Monday  09:00\u{2013}10:40  (100 min)"),
            "{texts:?}"
        );
        assert!(
            texts.contains(&strings::BUTTON_ADD_BREAK)
                && texts.contains(&strings::BUTTON_REMOVE_BREAK)
        );
        assert!(texts.contains(&strings::TIP_IDLE));
        let color_of = |text: &str| drawn.iter().find(|(t, _, _)| t == text).unwrap().1;
        assert_eq!(color_of(strings::BUTTON_REMOVE_BREAK), colors.text);
        assert_eq!(
            color_of(strings::BUTTON_ADD_BREAK),
            DIM,
            "it has a break already"
        );
    }

    #[test]
    fn hovering_the_bar_tells_the_times_and_the_duration() {
        let dir = Scratch::new("hover", "09:00", "10:40");
        let store = dir.store();
        let mut view = Detail::default();
        let l = lay();
        let class = class_of(&store);
        view.moved(&l, col(&l, &class, 50 * 60), mid_y(&l));
        let (_, drawn) = paint(&view, &store, SELECTED);
        assert!(
            drawn
                .iter()
                .any(|(t, _, _)| t == "Break  09:45\u{2013}09:55  (10 min)"),
            "{drawn:?}"
        );
        // Moving off the bar clears it again.
        assert!(view.moved(&l, 3, 3), "the tooltip has to go");
        let (_, drawn) = paint(&view, &store, SELECTED);
        assert!(drawn.iter().all(|(t, _, _)| !t.starts_with("Break ")));
    }

    #[test]
    fn dragging_shows_the_time_of_the_handle_and_the_new_position() {
        let dir = Scratch::new("drag-label", "09:00", "10:40");
        let mut store = dir.store();
        let mut view = Detail::default();
        let l = lay();
        let class = class_of(&store);
        let y = mid_y(&l);
        view.press(&mut store, SELECTED, &l, col(&l, &class, 45 * 60), y);
        view.moved(&l, col(&l, &class, 40 * 60), y);
        let (canvas, drawn) = paint(&view, &store, SELECTED);
        assert!(
            drawn.iter().any(|(t, _, _)| t.starts_with("09:40")),
            "{drawn:?}"
        );
        let colors = store.config().colors;
        let pixel = |offset| canvas.pixel(col(&l, &class, offset) as u32, y as u32);
        assert_eq!(
            pixel(42 * 60),
            argb(colors.break_gap),
            "the preview already shows the new break"
        );
        assert_eq!(store.revision(), 0);
    }

    #[test]
    fn add_break_shades_the_span_and_turns_red_where_it_does_not_fit() {
        let dir = Scratch::new("shade", "09:00", "09:50");
        let mut store = dir.store();
        let mut view = Detail::default();
        let l = lay();
        let class = class_of(&store);
        let y = mid_y(&l);
        view.press(&mut store, SELECTED, &l, l.add.x + 3, l.add.y + 3);

        let (_, drawn) = paint(&view, &store, SELECTED);
        assert!(
            drawn.iter().any(|(t, _, _)| t == strings::TIP_ADDING),
            "{drawn:?}"
        );

        view.moved(&l, col(&l, &class, 25 * 60), y);
        let (canvas, drawn) = paint(&view, &store, SELECTED);
        let colors = store.config().colors;
        let (text, color, _) = drawn
            .iter()
            .find(|(t, _, _)| t.starts_with("09:20"))
            .unwrap();
        assert_eq!(text, "09:20\u{2013}09:30");
        assert_eq!(*color, colors.text);
        let inside = canvas.pixel(col(&l, &class, 25 * 60) as u32, y as u32);
        let outside = canvas.pixel(col(&l, &class, 10 * 60) as u32, y as u32);
        assert_ne!(inside, outside, "the span is shaded");
        assert_eq!(outside, argb(colors.green));

        view.moved(&l, col(&l, &class, 46 * 60), y);
        let (_, drawn) = paint(&view, &store, SELECTED);
        let (_, color, _) = drawn
            .iter()
            .find(|(t, _, _)| t.starts_with("09:41"))
            .unwrap();
        assert_eq!(*color, colors.red, "it does not fit before the end");
    }

    #[test]
    fn without_a_selection_there_is_a_hint_and_greyed_out_buttons() {
        let dir = Scratch::new("hint", "09:00", "09:50");
        let store = dir.store();
        let view = Detail::default();
        let (_, drawn) = paint(&view, &store, None);
        let texts: Vec<&str> = drawn.iter().map(|(t, _, _)| t.as_str()).collect();
        assert!(texts.contains(&strings::DETAIL_HINT));
        assert!(
            drawn
                .iter()
                .all(|(t, c, _)| t == strings::DETAIL_HINT || *c == DIM)
        );
    }

    #[test]
    fn the_title_is_left_aligned_and_the_buttons_centred() {
        let dir = Scratch::new("align", "09:00", "09:50");
        let store = dir.store();
        let (_, drawn) = paint(&Detail::default(), &store, SELECTED);
        let align = |text: &str| {
            drawn
                .iter()
                .find(|(t, _, _)| t.starts_with(text))
                .unwrap()
                .2
        };
        assert_eq!(align("Monday"), Align::Left);
        assert_eq!(align(strings::BUTTON_ADD_BREAK), Align::Center);
    }
}
