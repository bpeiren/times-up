//! The week overview of the Settings window: one row per weekday on a shared time axis, classes
//! that are selected, moved, resized, added and removed with the mouse. Plain Rust without Win32:
//! the window feeds it pointer positions and gives it a [`Store`] to save to, and draws the text
//! through a callback, so everything here is unit tested. All sizes are device pixels.
//!
//! A drag changes a preview only; the day is saved through the store when the button is released.

use crate::canvas::{Align, Canvas, RectF};
use crate::config::{Colors, Rgba};
use crate::interaction::{CursorShape, Rect};
use crate::schedule::{Class, MINUTE, Secs, Weekday, format_clock, hm};
use crate::store::Store;
use crate::strings;

/// Classes snap to this, and may not get shorter.
pub const SNAP: Secs = 5 * MINUTE;
const DAY_END: Secs = 24 * 3600;
const AXIS_STEP: Secs = 30 * MINUTE;
const AXIS_MIN_START: Secs = hm(8, 0);
const AXIS_MIN_END: Secs = hm(17, 0);
/// A class added with the plus button: this long, this long after the last class.
const NEW_LEN: Secs = 50 * MINUTE;
const NEW_GAP: Secs = 10 * MINUTE;
const FIRST_START: Secs = hm(8, 30);

// Logical sizes at 96 DPI.
const LABEL_W: f32 = 132.0;
const HOURS_H: f32 = 26.0;
const FOOTER_H: f32 = 52.0;
const SIDE: f32 = 12.0;
const PLUS: f32 = 30.0;
const ROW_PAD: f32 = 5.0;
const GRIP: f32 = 6.0;
const REMOVE_W: f32 = 130.0;
const REMOVE_H: f32 = 32.0;

const GRID: Rgba = Rgba {
    r: 255,
    g: 255,
    b: 255,
    a: 28,
};
const DIM: Rgba = Rgba {
    r: 255,
    g: 255,
    b: 255,
    a: 90,
};
const BUTTON: Rgba = Rgba::rgb(0x3a, 0x3c, 0x42);
const ERROR: Rgba = Rgba::rgb(0xff, 0x8a, 0x80);

// ---- the time axis ----

/// The span of time all rows show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Axis {
    pub start: Secs,
    pub end: Secs,
}

fn floor_to(t: Secs, step: Secs) -> Secs {
    t / step * step
}

fn ceil_to(t: Secs, step: Secs) -> Secs {
    t.div_ceil(step) * step
}

impl Axis {
    /// At least 08:00 to 17:00, and wide enough for every class with 30 minutes to spare.
    pub fn fitted<'a>(classes: impl IntoIterator<Item = &'a Class>) -> Self {
        let mut axis = Self {
            start: AXIS_MIN_START,
            end: AXIS_MIN_END,
        };
        for c in classes {
            axis.start = axis
                .start
                .min(floor_to(c.start.saturating_sub(AXIS_STEP), AXIS_STEP));
            axis.end = axis
                .end
                .max(ceil_to(c.end + AXIS_STEP, AXIS_STEP).min(DAY_END));
        }
        axis
    }

    /// This axis, grown in 30 minute steps (never beyond the day) to contain `from..to`.
    pub fn grown(self, from: i64, to: i64) -> Self {
        let mut axis = self;
        if from < i64::from(axis.start) {
            axis.start = floor_to(from.max(0) as Secs, AXIS_STEP);
        }
        if to > i64::from(axis.end) {
            axis.end = ceil_to(to.min(i64::from(DAY_END)) as Secs, AXIS_STEP);
        }
        axis
    }

    const fn span(self) -> Secs {
        self.end - self.start
    }
}

// ---- geometry ----

/// Where everything sits for a given week and axis.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub axis: Axis,
    pub days: Vec<Weekday>,
    pub rows: Vec<Rect>,
    /// The part of every row the axis maps onto.
    pub track: Rect,
    pub hours: Rect,
    pub plus: Vec<Rect>,
    pub remove: Rect,
    pub grip: i32,
    scale: f32,
}

fn px(v: f32, scale: f32) -> i32 {
    (v * scale).round() as i32
}

/// Lay out `days` in `area`.
pub fn layout(area: Rect, scale: f32, days: &[Weekday], axis: Axis) -> Layout {
    let label = px(LABEL_W, scale);
    let side = px(SIDE, scale);
    let plus_w = px(PLUS, scale);
    let hours_h = px(HOURS_H, scale);
    let footer_h = px(FOOTER_H, scale);

    let track_x = area.x + label;
    let track_w = (area.w - label - plus_w - 2 * side).max(1);
    let rows_top = area.y + hours_h;
    let rows_h = (area.h - hours_h - footer_h).max(0);
    let count = days.len().max(1) as i32;
    let row_h = rows_h / count;

    let rows: Vec<Rect> = (0..days.len() as i32)
        .map(|i| Rect::new(area.x, rows_top + i * row_h, area.w, row_h))
        .collect();
    let plus = rows
        .iter()
        .map(|r| {
            let size = plus_w.min(r.h - px(ROW_PAD, scale));
            Rect::new(
                track_x + track_w + side,
                r.y + (r.h - size) / 2,
                size.max(0),
                size.max(0),
            )
        })
        .collect();
    let button_h = px(REMOVE_H, scale);
    Layout {
        axis,
        days: days.to_vec(),
        rows,
        track: Rect::new(track_x, rows_top, track_w, rows_h),
        hours: Rect::new(track_x, area.y, track_w, hours_h),
        plus,
        remove: Rect::new(
            area.right() - side - px(REMOVE_W, scale),
            area.bottom() - footer_h + (footer_h - button_h) / 2,
            px(REMOVE_W, scale),
            button_h,
        ),
        grip: px(GRIP, scale).max(2),
        scale,
    }
}

impl Layout {
    /// The pixel column of a time.
    pub fn x_of(&self, t: Secs) -> i32 {
        let along = i64::from(t) - i64::from(self.axis.start);
        self.track.x + (along * i64::from(self.track.w) / i64::from(self.axis.span())) as i32
    }

    /// The time under a pixel column; not limited to the axis.
    pub fn time_at(&self, x: i32) -> i64 {
        let along = i64::from(x - self.track.x) * i64::from(self.axis.span());
        i64::from(self.axis.start) + along.div_euclid(i64::from(self.track.w))
    }

    /// The rectangle of a class inside its row.
    pub fn class_rect(&self, row: usize, class: &Class) -> Rect {
        let r = self.rows[row];
        let pad = px(ROW_PAD, self.scale);
        let (x0, x1) = (self.x_of(class.start), self.x_of(class.end));
        Rect::new(x0, r.y + pad, (x1 - x0).max(1), (r.h - 2 * pad).max(1))
    }
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.x && x < r.right() && y >= r.y && y < r.bottom()
}

/// Which part of a class the pointer is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Body,
    Start,
    End,
}

/// What is under the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Nothing,
    Plus(Weekday),
    Remove,
    Class {
        day: Weekday,
        index: usize,
        part: Part,
    },
}

// ---- adding, and the shape of a dragged class ----

/// Start and end of the class the plus button adds: 50 minutes, 10 minutes after the last class
/// (08:30 on an empty day). `None` when it would not fit in the day.
pub fn new_class_span(classes: &[Class]) -> Option<(Secs, Secs)> {
    let start = classes
        .last()
        .map_or(FIRST_START, |c| ceil_to(c.end + NEW_GAP, SNAP));
    let end = start + NEW_LEN;
    (end <= DAY_END).then_some((start, end))
}

fn snap(t: i64) -> i64 {
    let step = i64::from(SNAP);
    (t + step / 2).div_euclid(step) * step
}

/// `original` stretched or moved to `start..end`. A moved class keeps its break and zones; a
/// resized one keeps its zones and its break when that still fits, and a class that had the
/// default break gets the default break for its new length.
fn with_span(original: &Class, start: Secs, end: Secs, default_break: Secs) -> Class {
    let mut class = original.clone();
    class.start = start;
    class.end = end;
    if end - start == original.duration() {
        return class;
    }
    let had_default = Class::new(original.start, original.end, default_break)
        .is_ok_and(|c| c.brk == original.brk);
    let Ok(mut resized) = Class::new(start, end, default_break) else {
        return class;
    };
    resized.warnings = original.warnings;
    if !had_default {
        resized = match original.brk {
            Some(b) => resized.clone().with_break(Some(b)).unwrap_or(resized),
            None => Class {
                brk: None,
                ..resized
            },
        };
    }
    resized
}

// ---- the editor ----

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Selected {
    day: Weekday,
    start: Secs,
    end: Secs,
}

struct Drag {
    day: Weekday,
    index: usize,
    part: Part,
    /// Pointer time minus class start where a move began.
    offset: i64,
    original: Class,
    prev_end: Secs,
    next_start: Secs,
    axis: Axis,
}

struct Preview {
    day: Weekday,
    classes: Vec<Class>,
}

#[derive(Default)]
pub struct Timetable {
    selected: Option<Selected>,
    drag: Option<Drag>,
    preview: Option<Preview>,
    /// Why the last save failed, shown until the next change.
    error: Option<String>,
    /// A save changed the running class that has temporary changes on the bar.
    live_touched: bool,
}

impl Timetable {
    fn classes<'a>(&'a self, store: &'a Store, day: Weekday) -> &'a [Class] {
        match &self.preview {
            Some(p) if p.day == day => &p.classes,
            _ => store.schedule().classes(day),
        }
    }

    /// The days shown and the axis they share (fixed while a class is being dragged).
    pub fn days_and_axis(&self, store: &Store) -> (Vec<Weekday>, Axis) {
        let days = Weekday::ALL.to_vec();
        let axis = self.drag.as_ref().map_or_else(
            || Axis::fitted(days.iter().flat_map(|&d| self.classes(store, d))),
            |d| d.axis,
        );
        (days, axis)
    }

    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Whether a save touched the running class that has temporary changes on the bar since the
    /// last time this was asked.
    pub fn take_live_touched(&mut self) -> bool {
        std::mem::take(&mut self.live_touched)
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The selected class, as its day and index in that day, if it still exists.
    pub fn selected(&self, store: &Store) -> Option<(Weekday, usize)> {
        let s = self.selected?;
        self.classes(store, s.day)
            .iter()
            .position(|c| c.start == s.start && c.end == s.end)
            .map(|i| (s.day, i))
    }

    fn select(&mut self, day: Weekday, class: &Class) {
        self.selected = Some(Selected {
            day,
            start: class.start,
            end: class.end,
        });
    }

    /// What is under `(x, y)`.
    pub fn target(&self, store: &Store, layout: &Layout, x: i32, y: i32) -> Target {
        if inside(layout.remove, x, y) {
            return Target::Remove;
        }
        for (row, &day) in layout.days.iter().enumerate() {
            if inside(layout.plus[row], x, y) {
                return Target::Plus(day);
            }
            for (index, class) in self.classes(store, day).iter().enumerate() {
                let r = layout.class_rect(row, class);
                if !inside(r, x, y) {
                    continue;
                }
                let grip = layout.grip.min(r.w / 3);
                let part = if x < r.x + grip {
                    Part::Start
                } else if x >= r.right() - grip {
                    Part::End
                } else {
                    Part::Body
                };
                return Target::Class { day, index, part };
            }
        }
        Target::Nothing
    }

    pub fn cursor(&self, store: &Store, layout: &Layout, x: i32, y: i32) -> CursorShape {
        let part = match &self.drag {
            Some(d) => Some(d.part),
            None => match self.target(store, layout, x, y) {
                Target::Class { part, .. } => Some(part),
                _ => None,
            },
        };
        match part {
            Some(Part::Body) => CursorShape::SizeAll,
            Some(Part::Start | Part::End) => CursorShape::SizeWestEast,
            None => CursorShape::Arrow,
        }
    }

    /// The button went down at `(x, y)`: select, start a drag, or press a button. Adding and
    /// removing are saved right away.
    pub fn press(&mut self, store: &mut Store, layout: &Layout, x: i32, y: i32) {
        match self.target(store, layout, x, y) {
            Target::Plus(day) => self.add_class(store, day),
            Target::Remove => self.remove_selected(store),
            Target::Nothing => self.selected = None,
            Target::Class { day, index, part } => {
                let classes = store.schedule().classes(day);
                let original = classes[index].clone();
                self.select(day, &original);
                self.drag = Some(Drag {
                    day,
                    index,
                    part,
                    offset: layout.time_at(x) - i64::from(original.start),
                    prev_end: if index == 0 {
                        0
                    } else {
                        classes[index - 1].end
                    },
                    next_start: classes.get(index + 1).map_or(DAY_END, |c| c.start),
                    axis: layout.axis,
                    original,
                });
            }
        }
    }

    /// The pointer moved to column `x` while the button is down. The axis grows when the class is
    /// pushed against its end; the class stays between its neighbours.
    pub fn drag_to(&mut self, store: &Store, layout: &Layout, x: i32, _y: i32) {
        let Some(drag) = &mut self.drag else {
            return;
        };
        let t = snap(layout.time_at(x));
        let original = &drag.original;
        let duration = i64::from(original.duration());
        let (start, end) = match drag.part {
            Part::Body => {
                let start = snap(layout.time_at(x) - drag.offset);
                (start, start + duration)
            }
            Part::Start => (t, i64::from(original.end)),
            Part::End => (i64::from(original.start), t),
        };
        drag.axis = drag.axis.grown(start, end);

        let (lo, hi) = (i64::from(drag.prev_end), i64::from(drag.next_start));
        let min = i64::from(SNAP);
        let (start, end) = match drag.part {
            Part::Body => {
                let s = start.clamp(lo, hi - duration);
                (s, s + duration)
            }
            Part::Start => (
                start.clamp(lo, i64::from(original.end) - min),
                i64::from(original.end),
            ),
            Part::End => (
                i64::from(original.start),
                end.clamp(i64::from(original.start) + min, hi),
            ),
        };
        let class = with_span(
            original,
            start as Secs,
            end as Secs,
            store.config().default_break,
        );
        let mut classes = store.schedule().classes(drag.day).to_vec();
        classes[drag.index] = class.clone();
        let day = drag.day;
        self.preview = Some(Preview { day, classes });
        self.select(day, &class);
    }

    /// The button went up: save what the drag made, if it changed anything.
    pub fn release(&mut self, store: &mut Store) {
        let (drag, preview) = (self.drag.take(), self.preview.take());
        let (Some(drag), Some(preview)) = (drag, preview) else {
            return;
        };
        if store.schedule().classes(drag.day) == preview.classes {
            return;
        }
        self.save(store, drag.day, preview.classes);
    }

    /// Give up a drag without saving (the mouse capture was lost).
    pub fn cancel(&mut self) {
        self.drag = None;
        self.preview = None;
    }

    fn save(&mut self, store: &mut Store, day: Weekday, classes: Vec<Class>) {
        match store.set_day(day, classes) {
            Ok(touched_live) => {
                self.error = None;
                self.live_touched |= touched_live;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    /// Add a class at the end of `day` and select it. Does nothing when it does not fit.
    pub fn add_class(&mut self, store: &mut Store, day: Weekday) {
        let mut classes = store.schedule().classes(day).to_vec();
        let Some((start, end)) = new_class_span(&classes) else {
            return;
        };
        let config = store.config();
        let (default_break, warnings) = (config.default_break, config.warnings);
        let Ok(mut class) = Class::new(start, end, default_break) else {
            return;
        };
        class.warnings = [warnings; 2];
        classes.push(class.clone());
        self.save(store, day, classes);
        if self.error.is_none() {
            self.select(day, &class);
        }
    }

    /// Remove the selected class (the Remove button and the Delete key).
    pub fn remove_selected(&mut self, store: &mut Store) {
        let Some((day, index)) = self.selected(store) else {
            return;
        };
        let mut classes = store.schedule().classes(day).to_vec();
        classes.remove(index);
        self.save(store, day, classes);
        if self.error.is_none() {
            self.selected = None;
        }
    }

    /// Whether the plus button of `day` has room for another class.
    pub fn can_add(&self, store: &Store, day: Weekday) -> bool {
        new_class_span(self.classes(store, day)).is_some()
    }

    /// Paint the week. `text` draws a string centred in a rectangle in a colour.
    pub fn paint(
        &self,
        canvas: &mut Canvas,
        store: &Store,
        layout: &Layout,
        text: &mut dyn FnMut(&mut Canvas, &str, Rect, Rgba, Align),
    ) {
        let colors = store.config().colors;
        self.paint_axis(canvas, layout, &colors, text);
        let selected = self.selected(store);
        for (row, &day) in layout.days.iter().enumerate() {
            let r = layout.rows[row];
            let name = Rect::new(r.x, r.y, layout.track.x - r.x, r.h);
            text(
                canvas,
                strings::DAY_NAMES[day.index()],
                name,
                colors.text,
                Align::Center,
            );
            fill(canvas, layout.class_track(row), colors.track, 0.0);
            for (index, class) in self.classes(store, day).iter().enumerate() {
                let rect = layout.class_rect(row, class);
                if selected == Some((day, index)) {
                    let outline = Rect::new(rect.x - 2, rect.y - 2, rect.w + 4, rect.h + 4);
                    fill(canvas, outline, colors.text, 5.0);
                }
                fill(canvas, rect, colors.lesson, 4.0);
            }
            let can_add = self.can_add(store, day);
            let plus = layout.plus[row];
            fill(canvas, plus, BUTTON, 6.0);
            text(
                canvas,
                "+",
                plus,
                if can_add { colors.text } else { DIM },
                Align::Center,
            );
        }
        let remove_color = if selected.is_some() { colors.text } else { DIM };
        fill(canvas, layout.remove, BUTTON, 6.0);
        text(
            canvas,
            strings::BUTTON_REMOVE_CLASS,
            layout.remove,
            remove_color,
            Align::Center,
        );
        if let Some(error) = &self.error {
            let area = Rect::new(
                layout.hours.x,
                layout.remove.y,
                layout.remove.x - layout.hours.x,
                layout.remove.h,
            );
            text(canvas, error, area, ERROR, Align::Center);
        }
        self.paint_drag_label(canvas, store, layout, &colors, text);
    }

    fn paint_axis(
        &self,
        canvas: &mut Canvas,
        layout: &Layout,
        colors: &Colors,
        text: &mut dyn FnMut(&mut Canvas, &str, Rect, Rgba, Align),
    ) {
        let bottom = layout.track.bottom();
        let label_w = layout.hours.h * 3;
        for hour in layout.axis.start.div_ceil(3600)..=layout.axis.end / 3600 {
            let x = layout.x_of(hour * 3600);
            fill(
                canvas,
                Rect::new(x, layout.track.y, 1, bottom - layout.track.y),
                GRID,
                0.0,
            );
            let area = Rect::new(x - label_w / 2, layout.hours.y, label_w, layout.hours.h);
            text(
                canvas,
                &format_clock(hour * 3600),
                area,
                colors.text,
                Align::Center,
            );
        }
    }

    fn paint_drag_label(
        &self,
        canvas: &mut Canvas,
        store: &Store,
        layout: &Layout,
        colors: &Colors,
        text: &mut dyn FnMut(&mut Canvas, &str, Rect, Rgba, Align),
    ) {
        let (Some(drag), Some(_)) = (&self.drag, &self.preview) else {
            return;
        };
        let Some(row) = layout.days.iter().position(|&d| d == drag.day) else {
            return;
        };
        let Some(class) = self.classes(store, drag.day).get(drag.index) else {
            return;
        };
        let rect = layout.class_rect(row, class);
        let label = format!("{}-{}", format_clock(class.start), format_clock(class.end));
        let w = rect.h * 4;
        let area = Rect::new(rect.x + rect.w / 2 - w / 2, rect.y, w, rect.h);
        fill(
            canvas,
            area,
            Rgba {
                a: 200,
                ..colors.track
            },
            5.0,
        );
        text(canvas, &label, area, colors.text, Align::Center);
    }
}

impl Layout {
    /// The strip of a row that the axis covers.
    fn class_track(&self, row: usize) -> Rect {
        let r = self.rows[row];
        let pad = px(ROW_PAD, self.scale);
        Rect::new(
            self.track.x,
            r.y + pad,
            self.track.w,
            (r.h - 2 * pad).max(1),
        )
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
    use crate::schedule::{Break, DEFAULT_BREAK_LEN};

    const MON: Weekday = Weekday::Mon;
    const AREA: Rect = Rect::new(0, 40, 860, 480);

    /// A scratch config with a store on it, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str, text: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("times-up-timetable-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
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
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn class(start: Secs, end: Secs) -> Class {
        Class::new(start, end, DEFAULT_BREAK_LEN).unwrap()
    }

    fn day_text(classes: &[(&str, &str)]) -> String {
        let list: Vec<String> = classes
            .iter()
            .map(|(s, e)| format!("{{ start = \"{s}\", end = \"{e}\" }}"))
            .collect();
        format!("[monday]\nclasses = [{}]\n", list.join(", "))
    }

    /// The layout for the current state of `view`.
    fn lay(view: &Timetable, store: &Store) -> Layout {
        let (days, axis) = view.days_and_axis(store);
        layout(AREA, 1.0, &days, axis)
    }

    /// Where the middle of `class` on Monday is.
    fn middle(view: &Timetable, store: &Store, class: &Class) -> (i32, i32) {
        let l = lay(view, store);
        let r = l.class_rect(0, class);
        (r.x + r.w / 2, r.y + r.h / 2)
    }

    fn press_at(view: &mut Timetable, store: &mut Store, x: i32, y: i32) {
        let l = lay(view, store);
        view.press(store, &l, x, y);
    }

    fn drag_at(view: &mut Timetable, store: &Store, x: i32) {
        let l = lay(view, store);
        view.drag_to(store, &l, x, 0);
    }

    fn monday(store: &Store) -> Vec<(Secs, Secs)> {
        store
            .schedule()
            .classes(MON)
            .iter()
            .map(|c| (c.start, c.end))
            .collect()
    }

    // ---- the axis ----

    #[test]
    fn the_axis_is_at_least_eight_to_five() {
        assert_eq!(
            Axis::fitted([]),
            Axis {
                start: hm(8, 0),
                end: hm(17, 0)
            }
        );
        let inside = [class(hm(9, 0), hm(10, 0))];
        assert_eq!(
            Axis::fitted(&inside),
            Axis {
                start: hm(8, 0),
                end: hm(17, 0)
            }
        );
    }

    #[test]
    fn the_axis_fits_the_week_with_half_an_hour_to_spare() {
        let early = class(hm(7, 10), hm(8, 0));
        let late = class(hm(16, 30), hm(18, 15));
        let axis = Axis::fitted(&[early, late]);
        assert_eq!(
            axis,
            Axis {
                start: hm(6, 30),
                end: hm(19, 0)
            }
        );
        // A class ending at 17:00 sharp leaves the padding after it.
        let axis = Axis::fitted(&[class(hm(16, 0), hm(17, 0))]);
        assert_eq!(axis.end, hm(17, 30));
        let axis = Axis::fitted(&[class(hm(0, 0), hm(0, 50)), class(hm(23, 0), hm(24, 0))]);
        assert_eq!(
            axis,
            Axis {
                start: 0,
                end: hm(24, 0)
            }
        );
    }

    #[test]
    fn the_axis_grows_in_half_hour_steps_up_to_the_day() {
        let axis = Axis {
            start: hm(8, 0),
            end: hm(17, 0),
        };
        assert_eq!(axis.grown(hm(9, 0).into(), hm(17, 0).into()), axis);
        assert_eq!(
            axis.grown(hm(9, 0).into(), hm(17, 5).into()).end,
            hm(17, 30)
        );
        assert_eq!(
            axis.grown(hm(7, 55).into(), hm(9, 0).into()).start,
            hm(7, 30)
        );
        assert_eq!(axis.grown(-500, hm(9, 0).into()).start, 0);
        assert_eq!(axis.grown(0, 100_000).end, hm(24, 0));
    }

    // ---- adding ----

    #[test]
    fn a_new_class_is_fifty_minutes_ten_minutes_after_the_last() {
        assert_eq!(new_class_span(&[]), Some((hm(8, 30), hm(9, 20))));
        let last = [class(hm(9, 0), hm(9, 50))];
        assert_eq!(new_class_span(&last), Some((hm(10, 0), hm(10, 50))));
        let last = [class(hm(9, 0), hm(9, 53))];
        assert_eq!(
            new_class_span(&last),
            Some((hm(10, 5), hm(10, 55))),
            "snapped up"
        );
    }

    #[test]
    fn there_is_no_room_for_a_class_that_would_end_after_midnight() {
        assert!(new_class_span(&[class(hm(22, 0), hm(22, 50))]).is_some());
        assert_eq!(new_class_span(&[class(hm(22, 30), hm(23, 15))]), None);
    }

    // ---- geometry ----

    #[test]
    fn all_seven_days_have_a_row() {
        let dir = Scratch::new("days", "");
        let store = dir.store();
        let (days, _) = Timetable::default().days_and_axis(&store);
        assert_eq!(days, Weekday::ALL.to_vec());
    }

    #[test]
    fn times_and_pixels_map_both_ways() {
        let axis = Axis {
            start: hm(8, 0),
            end: hm(17, 0),
        };
        let l = layout(AREA, 1.0, &[MON], axis);
        assert_eq!(l.x_of(hm(8, 0)), l.track.x);
        assert_eq!(l.x_of(hm(17, 0)), l.track.right());
        for t in [hm(8, 0), hm(9, 5), hm(12, 30), hm(16, 55)] {
            assert!((l.time_at(l.x_of(t)) - i64::from(t)).abs() < 90, "{t}");
        }
        assert!(
            l.time_at(l.track.x - 50) < i64::from(hm(8, 0)),
            "not limited to the axis"
        );
    }

    #[test]
    fn rows_share_the_height_and_the_buttons_sit_beside_them() {
        let days = [MON, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri];
        let l = layout(
            AREA,
            1.0,
            &days,
            Axis {
                start: hm(8, 0),
                end: hm(17, 0),
            },
        );
        assert_eq!(l.rows.len(), 5);
        assert!(l.rows.iter().all(|r| r.h == l.rows[0].h));
        assert!(l.rows[4].bottom() <= l.remove.y);
        assert!(
            l.plus
                .iter()
                .all(|p| p.x >= l.track.right() && p.right() <= AREA.right())
        );
        assert!(l.remove.right() <= AREA.right() && l.remove.bottom() <= AREA.bottom());
    }

    #[test]
    fn pointer_targets_are_found() {
        let dir = Scratch::new("target", &day_text(&[("09:00", "09:50")]));
        let store = dir.store();
        let view = Timetable::default();
        let l = lay(&view, &store);
        let c = &store.schedule().classes(MON)[0];
        let r = l.class_rect(0, c);
        let y = r.y + r.h / 2;
        let at = |x, y| view.target(&store, &l, x, y);
        let part = |part| Target::Class {
            day: MON,
            index: 0,
            part,
        };
        assert_eq!(at(r.x + r.w / 2, y), part(Part::Body));
        assert_eq!(at(r.x + 1, y), part(Part::Start));
        assert_eq!(at(r.right() - 2, y), part(Part::End));
        assert_eq!(at(r.right() + 5, y), Target::Nothing);
        assert_eq!(at(l.plus[0].x + 2, l.plus[0].y + 2), Target::Plus(MON));
        assert_eq!(at(l.remove.x + 2, l.remove.y + 2), Target::Remove);
        assert_eq!(
            view.cursor(&store, &l, r.x + r.w / 2, y),
            CursorShape::SizeAll
        );
        assert_eq!(
            view.cursor(&store, &l, r.x + 1, y),
            CursorShape::SizeWestEast
        );
        assert_eq!(view.cursor(&store, &l, 1, 1), CursorShape::Arrow);
    }

    // ---- selecting and dragging ----

    #[test]
    fn clicking_selects_and_clicking_elsewhere_deselects() {
        let dir = Scratch::new(
            "select",
            &day_text(&[("09:00", "09:50"), ("10:00", "10:50")]),
        );
        let mut store = dir.store();
        let mut view = Timetable::default();
        assert_eq!(view.selected(&store), None);
        let second = store.schedule().classes(MON)[1].clone();
        let (x, y) = middle(&view, &store, &second);
        press_at(&mut view, &mut store, x, y);
        view.release(&mut store);
        assert_eq!(view.selected(&store), Some((MON, 1)));
        assert_eq!(store.revision(), 0, "a click saves nothing");
        press_at(&mut view, &mut store, 5, 5);
        assert_eq!(view.selected(&store), None);
    }

    #[test]
    fn dragging_the_body_moves_the_class_in_five_minute_steps_and_saves_on_release() {
        let dir = Scratch::new("move", &day_text(&[("09:00", "09:50")]));
        let mut store = dir.store();
        let mut view = Timetable::default();
        let c = store.schedule().classes(MON)[0].clone();
        let (x, y) = middle(&view, &store, &c);
        press_at(&mut view, &mut store, x, y);
        // An hour later on a 9 hour axis over ~700 pixels.
        let l = lay(&view, &store);
        drag_at(&mut view, &store, x + l.x_of(hm(10, 0)) - l.x_of(hm(9, 0)));
        assert!(view.is_dragging());
        assert_eq!(
            monday(&store),
            vec![(hm(9, 0), hm(9, 50))],
            "nothing saved yet"
        );
        let l = lay(&view, &store);
        let (days, _) = view.days_and_axis(&store);
        assert_eq!(days.len(), 7);
        let preview = &view.classes(&store, MON)[0];
        assert_eq!(preview.start % SNAP, 0);
        assert_eq!(preview.duration(), 50 * 60);
        assert!(preview.start == hm(10, 0), "{}", preview.start);
        let _ = l;
        view.release(&mut store);
        assert!(!view.is_dragging());
        assert_eq!(store.revision(), 1);
        let start = store.schedule().classes(MON)[0].start;
        assert!(
            dir.text()
                .contains(&format!("start = \"{}\"", format_clock(start)))
        );
        assert_eq!(
            view.selected(&store),
            Some((MON, 0)),
            "still selected after the move"
        );
    }

    #[test]
    fn a_class_stops_at_its_neighbours() {
        let dir = Scratch::new(
            "neighbours",
            &day_text(&[("09:00", "09:50"), ("10:00", "10:50"), ("11:00", "11:50")]),
        );
        let mut store = dir.store();
        let mut view = Timetable::default();
        let mid = store.schedule().classes(MON)[1].clone();
        let (x, y) = middle(&view, &store, &mid);
        press_at(&mut view, &mut store, x, y);
        let l = lay(&view, &store);
        drag_at(&mut view, &store, l.x_of(hm(15, 0)));
        let p = view.classes(&store, MON)[1].clone();
        assert_eq!(
            (p.start, p.end),
            (hm(10, 10), hm(11, 0)),
            "against the next class"
        );
        drag_at(&mut view, &store, l.x_of(hm(8, 0)));
        let p = view.classes(&store, MON)[1].clone();
        assert_eq!(
            (p.start, p.end),
            (hm(9, 50), hm(10, 40)),
            "against the previous class"
        );
        view.release(&mut store);
        assert_eq!(
            monday(&store),
            vec![
                (hm(9, 0), hm(9, 50)),
                (hm(9, 50), hm(10, 40)),
                (hm(11, 0), hm(11, 50))
            ]
        );
    }

    #[test]
    fn resizing_an_edge_keeps_the_other_edge_and_at_least_five_minutes() {
        let dir = Scratch::new(
            "resize",
            &day_text(&[("09:00", "09:50"), ("10:30", "11:20")]),
        );
        let mut store = dir.store();
        let mut view = Timetable::default();
        let c = store.schedule().classes(MON)[0].clone();
        let l = lay(&view, &store);
        let r = l.class_rect(0, &c);
        press_at(&mut view, &mut store, r.right() - 2, r.y + r.h / 2);
        drag_at(&mut view, &store, l.x_of(hm(10, 15)));
        let p = view.classes(&store, MON)[0].clone();
        assert_eq!((p.start, p.end), (hm(9, 0), hm(10, 15)));
        drag_at(&mut view, &store, l.x_of(hm(10, 50)));
        assert_eq!(
            view.classes(&store, MON)[0].end,
            hm(10, 30),
            "stops at the next class"
        );
        drag_at(&mut view, &store, l.x_of(hm(8, 30)));
        assert_eq!(
            view.classes(&store, MON)[0].end,
            hm(9, 5),
            "five minutes at least"
        );
        view.release(&mut store);
        assert_eq!(monday(&store)[0], (hm(9, 0), hm(9, 5)));

        let c = store.schedule().classes(MON)[0].clone();
        let l = lay(&view, &store);
        let r = l.class_rect(0, &c);
        press_at(&mut view, &mut store, r.x, r.y + r.h / 2);
        drag_at(&mut view, &store, l.x_of(hm(7, 0)));
        assert_eq!(view.classes(&store, MON)[0].start, hm(7, 0));
        assert_eq!(
            view.days_and_axis(&store).1.start,
            hm(7, 0),
            "the axis grew"
        );
        let l = lay(&view, &store);
        drag_at(&mut view, &store, l.x_of(hm(9, 30)));
        assert_eq!(
            view.classes(&store, MON)[0].start,
            hm(9, 0),
            "never past the end"
        );
        view.release(&mut store);
    }

    #[test]
    fn the_axis_stays_while_dragging_grows_at_the_end_and_fits_again_afterwards() {
        let dir = Scratch::new("axis", &day_text(&[("15:00", "15:50")]));
        let mut store = dir.store();
        let mut view = Timetable::default();
        let c = store.schedule().classes(MON)[0].clone();
        let (x, y) = middle(&view, &store, &c);
        press_at(&mut view, &mut store, x, y);
        let (_, before) = view.days_and_axis(&store);
        assert_eq!(
            before,
            Axis {
                start: hm(8, 0),
                end: hm(17, 0)
            }
        );
        let l = lay(&view, &store);
        drag_at(&mut view, &store, l.x_of(hm(16, 0)));
        assert_eq!(view.days_and_axis(&store).1, before, "inside: fixed");
        drag_at(&mut view, &store, l.track.right() + 60);
        let (_, during) = view.days_and_axis(&store);
        assert!(
            during.end > before.end && during.end % (30 * 60) == 0,
            "{during:?}"
        );
        assert_eq!(during.start, before.start);
        // Pull it back: the axis does not shrink while the button is down.
        let l2 = lay(&view, &store);
        drag_at(&mut view, &store, l2.x_of(hm(9, 0)));
        assert_eq!(view.days_and_axis(&store).1, during);
        view.release(&mut store);
        let (_, after) = view.days_and_axis(&store);
        assert_eq!(
            after,
            Axis {
                start: hm(8, 0),
                end: hm(17, 0)
            },
            "fits the data again"
        );
    }

    #[test]
    fn dragging_back_to_where_it_began_saves_nothing() {
        let dir = Scratch::new("unchanged", &day_text(&[("09:00", "09:50")]));
        let mut store = dir.store();
        let mut view = Timetable::default();
        let c = store.schedule().classes(MON)[0].clone();
        let (x, y) = middle(&view, &store, &c);
        press_at(&mut view, &mut store, x, y);
        drag_at(&mut view, &store, x + 80);
        drag_at(&mut view, &store, x);
        view.release(&mut store);
        assert_eq!(store.revision(), 0);
    }

    #[test]
    fn cancelling_a_drag_keeps_the_saved_class() {
        let dir = Scratch::new("cancel", &day_text(&[("09:00", "09:50")]));
        let mut store = dir.store();
        let mut view = Timetable::default();
        let c = store.schedule().classes(MON)[0].clone();
        let (x, y) = middle(&view, &store, &c);
        press_at(&mut view, &mut store, x, y);
        drag_at(&mut view, &store, x + 80);
        view.cancel();
        assert!(!view.is_dragging());
        assert_eq!(monday(&store), vec![(hm(9, 0), hm(9, 50))]);
        assert_eq!(view.classes(&store, MON)[0].start, hm(9, 0));
    }

    #[test]
    fn a_moved_class_keeps_its_break_and_a_default_break_follows_a_resize() {
        let custom = class(hm(9, 0), hm(10, 40))
            .with_break(Some(Break {
                start: 30 * 60,
                len: 5 * 60,
            }))
            .unwrap();
        let moved = with_span(&custom, hm(11, 0), hm(12, 40), DEFAULT_BREAK_LEN);
        assert_eq!(moved.brk, custom.brk);
        let resized = with_span(&custom, hm(9, 0), hm(11, 0), DEFAULT_BREAK_LEN);
        assert_eq!(resized.brk, custom.brk, "a custom break stays if it fits");
        let tiny = with_span(&custom, hm(9, 0), hm(9, 20), DEFAULT_BREAK_LEN);
        assert_eq!(tiny.brk, None, "and goes if it cannot");

        let plain = class(hm(9, 0), hm(9, 50));
        assert_eq!(plain.brk, None);
        let long = with_span(&plain, hm(9, 0), hm(11, 0), DEFAULT_BREAK_LEN);
        assert_eq!(
            long.brk,
            class(hm(9, 0), hm(11, 0)).brk,
            "gets the default break"
        );
        let no_break = class(hm(9, 0), hm(10, 40)).with_break(None).unwrap();
        let wider = with_span(&no_break, hm(9, 0), hm(11, 0), DEFAULT_BREAK_LEN);
        assert_eq!(wider.brk, None, "a deliberate no-break stays");
        let mut warned = plain.clone();
        warned.warnings[0] = crate::schedule::Warnings::new(8 * 60, 2 * 60).unwrap();
        assert_eq!(
            with_span(&warned, hm(9, 0), hm(9, 30), DEFAULT_BREAK_LEN).warnings,
            warned.warnings
        );
    }

    // ---- adding and removing ----

    #[test]
    fn the_plus_button_adds_a_class_and_selects_it() {
        let dir = Scratch::new("add", &day_text(&[("09:00", "09:50")]));
        let mut store = dir.store();
        let mut view = Timetable::default();
        let l = lay(&view, &store);
        let p = l.plus[0];
        press_at(&mut view, &mut store, p.x + 2, p.y + 2);
        assert_eq!(
            monday(&store),
            vec![(hm(9, 0), hm(9, 50)), (hm(10, 0), hm(10, 50))]
        );
        assert_eq!(view.selected(&store), Some((MON, 1)));
        assert!(dir.text().contains("10:50"));
        assert_eq!(store.revision(), 1);
    }

    #[test]
    fn an_empty_day_gets_its_first_class_at_half_past_eight() {
        let dir = Scratch::new("first", "");
        let mut store = dir.store();
        let mut view = Timetable::default();
        view.add_class(&mut store, Weekday::Wed);
        let c = &store.schedule().classes(Weekday::Wed)[0];
        assert_eq!((c.start, c.end), (hm(8, 30), hm(9, 20)));
        assert_eq!(c.warnings, [store.config().warnings; 2]);
    }

    #[test]
    fn the_plus_button_does_nothing_without_room() {
        let dir = Scratch::new("full", &day_text(&[("22:30", "23:30")]));
        let mut store = dir.store();
        let mut view = Timetable::default();
        assert!(!view.can_add(&store, MON));
        assert!(view.can_add(&store, Weekday::Tue));
        view.add_class(&mut store, MON);
        assert_eq!(store.revision(), 0);
        assert_eq!(view.error(), None);
    }

    #[test]
    fn remove_class_and_delete_remove_the_selected_class() {
        let dir = Scratch::new(
            "remove",
            &day_text(&[("09:00", "09:50"), ("10:00", "10:50")]),
        );
        let mut store = dir.store();
        let mut view = Timetable::default();
        view.remove_selected(&mut store);
        assert_eq!(store.revision(), 0, "nothing selected, nothing removed");

        let first = store.schedule().classes(MON)[0].clone();
        let (x, y) = middle(&view, &store, &first);
        press_at(&mut view, &mut store, x, y);
        view.release(&mut store);
        let l = lay(&view, &store);
        press_at(&mut view, &mut store, l.remove.x + 3, l.remove.y + 3);
        assert_eq!(monday(&store), vec![(hm(10, 0), hm(10, 50))]);
        assert_eq!(view.selected(&store), None);

        let last = store.schedule().classes(MON)[0].clone();
        let (x, y) = middle(&view, &store, &last);
        press_at(&mut view, &mut store, x, y);
        view.release(&mut store);
        view.remove_selected(&mut store); // what the Delete key does
        assert!(monday(&store).is_empty());
        assert!(dir.text().contains("classes = []"), "{}", dir.text());
    }

    #[test]
    fn a_selection_that_disappears_from_the_file_is_forgotten() {
        let dir = Scratch::new(
            "external",
            &day_text(&[("09:00", "09:50"), ("10:00", "10:50")]),
        );
        let mut store = dir.store();
        let mut view = Timetable::default();
        let second = store.schedule().classes(MON)[1].clone();
        let (x, y) = middle(&view, &store, &second);
        press_at(&mut view, &mut store, x, y);
        view.release(&mut store);
        // The file changes behind our back: the class is still there at another index...
        fs::write(
            dir.0.join(config::FILE_NAME),
            day_text(&[("10:00", "10:50")]),
        )
        .unwrap();
        assert!(store.poll());
        assert_eq!(view.selected(&store), Some((MON, 0)));
        // ...and then it is gone.
        fs::write(
            dir.0.join(config::FILE_NAME),
            // A different length too: the file is told apart by its modified time and length, and a
            // coarse file system clock may give two quick writes the same time.
            day_text(&[("11:00", "11:50"), ("12:00", "12:50")]),
        )
        .unwrap();
        assert!(store.poll());
        assert_eq!(view.selected(&store), None);
    }

    #[test]
    fn a_failed_save_is_shown_and_changes_nothing() {
        let dir = Scratch::new("fail", &day_text(&[("09:00", "09:50")]));
        let mut store = dir.store();
        let mut view = Timetable::default();
        fs::write(dir.0.join(config::FILE_NAME), "this is [not toml").unwrap();
        view.add_class(&mut store, MON);
        assert!(view.error().is_some());
        assert_eq!(monday(&store), vec![(hm(9, 0), hm(9, 50))]);
        assert_eq!(view.selected(&store), None);
    }

    // ---- painting ----

    #[test]
    fn painting_labels_the_days_hours_and_buttons_and_marks_the_selection() {
        let dir = Scratch::new("paint", &day_text(&[("09:00", "09:50")]));
        let mut store = dir.store();
        let mut view = Timetable::default();
        let c = store.schedule().classes(MON)[0].clone();
        let (x, y) = middle(&view, &store, &c);
        let mut canvas = Canvas::new(860, 520);
        let mut drawn: Vec<String> = Vec::new();
        let l = lay(&view, &store);
        view.paint(&mut canvas, &store, &l, &mut |_, t, _, _, _| {
            drawn.push(t.to_string())
        });
        for expected in [
            "Monday",
            "Friday",
            "08:00",
            "17:00",
            "+",
            strings::BUTTON_REMOVE_CLASS,
        ] {
            assert!(
                drawn.iter().any(|t| t == expected),
                "{expected} in {drawn:?}"
            );
        }
        assert!(drawn.iter().any(|t| t == "Wednesday" || t == "Sunday"));
        let lesson = store.config().colors.lesson;
        let argb = |c: Rgba| u32::from_be_bytes([c.a, c.r, c.g, c.b]);
        assert_eq!(canvas.pixel(x as u32, y as u32), argb(lesson));
        let r = l.class_rect(0, &c);
        let outline = canvas.pixel((r.x - 1) as u32, y as u32);
        assert_ne!(outline, argb(lesson));

        press_at(&mut view, &mut store, x, y);
        let l = lay(&view, &store);
        let mut canvas = Canvas::new(860, 520);
        let mut drawn = Vec::new();
        view.paint(&mut canvas, &store, &l, &mut |_, t, _, _, _| {
            drawn.push(t.to_string())
        });
        let text = store.config().colors.text;
        assert_eq!(
            canvas.pixel((r.x - 1) as u32, y as u32),
            argb(text),
            "outline"
        );
        assert!(
            !drawn.iter().any(|t| t.contains('-')),
            "no label before the class moves: {drawn:?}"
        );

        drag_at(&mut view, &store, x + 60);
        let l = lay(&view, &store);
        let mut drawn = Vec::new();
        view.paint(&mut canvas, &store, &l, &mut |_, t, _, _, _| {
            drawn.push(t.to_string())
        });
        let label = view.classes(&store, MON)[0].clone();
        let expected = format!("{}-{}", format_clock(label.start), format_clock(label.end));
        assert_ne!(label.start, hm(9, 0));
        assert!(
            drawn.contains(&expected),
            "the drag label {expected}: {drawn:?}"
        );
    }
}
