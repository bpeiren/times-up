//! The General section of the Settings window: the six colours of the bar with a preview, and the
//! lead-in, linger and the default break and warning minutes. Plain geometry, painting and the
//! rules for changing a value, like [`crate::detail`]; the window supplies the text and the
//! colour picker. Every change is validated and saved on the click, so a value that does not
//! make sense is refused with a message and the file stays as it was.

use crate::canvas::{Align, Canvas, RectF};
use crate::config::{Colors, General, Rgba};
use crate::interaction::Rect;
use crate::schedule::{MINUTE, Secs};
use crate::store::Store;
use crate::strings;

const PAD: f32 = 20.0;
const GAP: f32 = 32.0;
const PREVIEW_H: f32 = 44.0;
const HEAD_H: f32 = 28.0;
const ROW_H: f32 = 32.0;
const STEP_W: f32 = 32.0;
const STEP_H: f32 = 26.0;
const VALUE_W: f32 = 92.0;
const SWATCH_W: f32 = 56.0;
const SWATCH_H: f32 = 24.0;
const BUTTON_W: f32 = 140.0;
const BUTTON_H: f32 = 30.0;
/// Share of a column taken by the labels.
const LABEL_SHARE: f32 = 0.42;

const HEADING: Rgba = Rgba::rgb(0xb8, 0xba, 0xc0);
const LABEL: Rgba = Rgba::rgb(0xf2, 0xf2, 0xf2);
const BUTTON: Rgba = Rgba::rgb(0x3a, 0x3c, 0x42);
const BUTTON_TEXT: Rgba = Rgba::rgb(0xf2, 0xf2, 0xf2);
const DISABLED_TEXT: Rgba = Rgba::rgb(0x70, 0x72, 0x78);
const OUTLINE: Rgba = Rgba::rgb(0x70, 0x72, 0x78);
const ERROR: Rgba = Rgba::rgb(0xff, 0x8a, 0x80);

/// One of the six colours of the bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Green,
    Orange,
    Red,
    Break,
    Track,
    Text,
}

impl Slot {
    pub const ALL: [Slot; 6] = [
        Slot::Green,
        Slot::Orange,
        Slot::Red,
        Slot::Break,
        Slot::Track,
        Slot::Text,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Slot::Green => strings::COLOR_GREEN,
            Slot::Orange => strings::COLOR_ORANGE,
            Slot::Red => strings::COLOR_RED,
            Slot::Break => strings::COLOR_BREAK,
            Slot::Track => strings::COLOR_TRACK,
            Slot::Text => strings::COLOR_TEXT,
        }
    }

    pub fn get(self, colors: &Colors) -> Rgba {
        match self {
            Slot::Green => colors.green,
            Slot::Orange => colors.orange,
            Slot::Red => colors.red,
            Slot::Break => colors.break_gap,
            Slot::Track => colors.track,
            Slot::Text => colors.text,
        }
    }

    pub fn set(self, colors: &mut Colors, color: Rgba) {
        match self {
            Slot::Green => colors.green = color,
            Slot::Orange => colors.orange = color,
            Slot::Red => colors.red = color,
            Slot::Break => colors.break_gap = color,
            Slot::Track => colors.track = color,
            Slot::Text => colors.text = color,
        }
    }
}

/// One of the numbers that can be stepped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    LeadIn,
    Linger,
    DefaultBreak,
    Orange,
    Red,
}

impl Field {
    pub const ALL: [Field; 5] = [
        Field::LeadIn,
        Field::Linger,
        Field::DefaultBreak,
        Field::Orange,
        Field::Red,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Field::LeadIn => strings::FIELD_LEAD_IN,
            Field::Linger => strings::FIELD_LINGER,
            Field::DefaultBreak => strings::FIELD_BREAK,
            Field::Orange => strings::FIELD_ORANGE,
            Field::Red => strings::FIELD_RED,
        }
    }

    /// The value, in seconds.
    pub fn get(self, g: &General) -> Secs {
        match self {
            Field::LeadIn => g.lead_in,
            Field::Linger => g.linger,
            Field::DefaultBreak => g.default_break,
            Field::Orange => g.warnings.orange,
            Field::Red => g.warnings.red,
        }
    }

    /// What one click changes, in seconds.
    const fn step(self) -> Secs {
        match self {
            Field::Linger => 5,
            _ => MINUTE,
        }
    }

    /// The smallest and largest value, in seconds.
    const fn range(self) -> (Secs, Secs) {
        match self {
            Field::LeadIn => (0, General::MAX_LEAD_IN),
            Field::Linger => (0, General::MAX_LINGER),
            Field::DefaultBreak => (General::MIN_BREAK, General::MAX_BREAK),
            Field::Orange | Field::Red => (0, General::MAX_WARNING),
        }
    }

    /// `5 min` or `10 s`.
    pub fn display(self, g: &General) -> String {
        self.show(self.get(g))
    }

    fn show(self, secs: Secs) -> String {
        match self {
            Field::Linger => format!("{secs} s"),
            _ => format!("{} min", secs / MINUTE),
        }
    }

    /// Whether a click in direction `dir` (-1 or 1) stays within the range of the field.
    pub fn can_step(self, g: &General, dir: i32) -> bool {
        let (min, max) = self.range();
        let v = self.get(g);
        if dir < 0 { v > min } else { v < max }
    }

    /// The values after a click in direction `dir`, or why not.
    pub fn stepped(self, g: &General, dir: i32) -> Result<General, String> {
        let (min, max) = self.range();
        if !self.can_step(g, dir) {
            return Err(if dir < 0 {
                format!("{} {} {}", self.label(), strings::AT_LEAST, self.show(min))
            } else {
                format!("{} {} {}", self.label(), strings::AT_MOST, self.show(max))
            });
        }
        let step = self.step();
        let v = self.get(g);
        let v = if dir < 0 {
            v.saturating_sub(step).max(min)
        } else {
            (v + step).min(max)
        };
        let mut out = *g;
        match self {
            Field::LeadIn => out.lead_in = v,
            Field::Linger => out.linger = v,
            Field::DefaultBreak => out.default_break = v,
            Field::Orange => out.warnings.orange = v,
            Field::Red => out.warnings.red = v,
        }
        out.validate()?;
        Ok(out)
    }
}

/// Opacity of a colour as a whole percentage.
fn percent(alpha: u8) -> u32 {
    (u32::from(alpha) * 100 + 127) / 255
}

fn alpha_of(percent: u32) -> u8 {
    ((percent.min(100) * 255 + 50) / 100) as u8
}

/// What a click on the page can mean.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Pick(Slot),
    /// The opacity of the track, one step down (-1) or up (1).
    Opacity(i32),
    ResetColors,
    Step(Field, i32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorRow {
    pub slot: Slot,
    pub label: Rect,
    pub swatch: Rect,
    pub hex: Rect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stepper {
    pub label: Rect,
    pub minus: Rect,
    pub value: Rect,
    pub plus: Rect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRow {
    pub field: Field,
    pub stepper: Stepper,
}

/// Where everything sits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub preview: Rect,
    pub colors_heading: Rect,
    pub times_heading: Rect,
    pub colors: Vec<ColorRow>,
    pub opacity: Stepper,
    pub reset: Rect,
    pub fields: Vec<FieldRow>,
    pub message: Rect,
}

pub fn layout(area: Rect, scale: f32) -> Layout {
    let px = |v: f32| (v * scale).round() as i32;
    let (pad, gap, row_h) = (px(PAD), px(GAP), px(ROW_H));
    let width = (area.w - 2 * pad).max(0);
    let col_w = ((width - gap) / 2).max(0);
    let left = area.x + pad;
    let right = left + col_w + gap;
    let preview = Rect::new(left, area.y + pad, width, px(PREVIEW_H));
    let top = preview.bottom() + pad / 2 * 2;
    let head_h = px(HEAD_H);
    let rows_top = top + head_h;
    let label_w = (col_w as f32 * LABEL_SHARE) as i32;

    let centred = |y: i32, h: i32| y + (row_h - h) / 2;
    let stepper = |x: i32, y: i32| {
        let (sw, sh, vw) = (px(STEP_W), px(STEP_H), px(VALUE_W));
        let sy = centred(y, sh);
        let minus = Rect::new(x + label_w, sy, sw, sh);
        let value = Rect::new(minus.right(), sy, vw, sh);
        Stepper {
            label: Rect::new(x, y, label_w, row_h),
            minus,
            value,
            plus: Rect::new(value.right(), sy, sw, sh),
        }
    };

    let colors = Slot::ALL
        .iter()
        .zip(0..)
        .map(|(&slot, i)| {
            let y = rows_top + i * row_h;
            let (sw, sh) = (px(SWATCH_W), px(SWATCH_H));
            let swatch = Rect::new(left + label_w, centred(y, sh), sw, sh);
            ColorRow {
                slot,
                label: Rect::new(left, y, label_w, row_h),
                swatch,
                hex: Rect::new(
                    swatch.right() + pad / 2,
                    y,
                    (left + col_w - swatch.right() - pad / 2).max(0),
                    row_h,
                ),
            }
        })
        .collect();
    let opacity_y = rows_top + Slot::ALL.len() as i32 * row_h;
    let opacity = stepper(left, opacity_y);
    let (bw, bh) = (px(BUTTON_W), px(BUTTON_H));
    let reset = Rect::new(left, opacity_y + row_h + pad / 2, bw.min(col_w.max(1)), bh);

    let fields: Vec<FieldRow> = Field::ALL
        .iter()
        .zip(0..)
        .map(|(&field, i)| FieldRow {
            field,
            stepper: stepper(right, rows_top + i * row_h),
        })
        .collect();
    let message_y = rows_top + Field::ALL.len() as i32 * row_h + pad / 2;
    Layout {
        preview,
        colors_heading: Rect::new(left, top, col_w, head_h),
        times_heading: Rect::new(right, top, col_w, head_h),
        colors,
        opacity,
        reset,
        fields,
        message: Rect::new(right, message_y, col_w, row_h * 2),
    }
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.x && x < r.right() && y >= r.y && y < r.bottom()
}

impl Layout {
    pub fn target_at(&self, x: i32, y: i32) -> Option<Target> {
        for row in &self.colors {
            if inside(row.swatch, x, y) || inside(row.hex, x, y) {
                return Some(Target::Pick(row.slot));
            }
        }
        if inside(self.opacity.minus, x, y) {
            return Some(Target::Opacity(-1));
        }
        if inside(self.opacity.plus, x, y) {
            return Some(Target::Opacity(1));
        }
        if inside(self.reset, x, y) {
            return Some(Target::ResetColors);
        }
        for row in &self.fields {
            if inside(row.stepper.minus, x, y) {
                return Some(Target::Step(row.field, -1));
            }
            if inside(row.stepper.plus, x, y) {
                return Some(Target::Step(row.field, 1));
            }
        }
        None
    }
}

/// The state of the page: only the message about the last click that was refused.
#[derive(Default)]
pub struct Page {
    message: Option<String>,
}

impl Page {
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// A click. Returns the colour the teacher wants to change, for the window to ask for with
    /// the colour picker; everything else is handled here.
    pub fn press(
        &mut self,
        store: &mut Store,
        layout: &Layout,
        x: i32,
        y: i32,
    ) -> Option<(Slot, Rgba)> {
        let target = layout.target_at(x, y)?;
        self.message = None;
        match target {
            Target::Pick(slot) => return Some((slot, slot.get(&store.config().colors))),
            Target::Opacity(dir) => {
                let mut colors = store.config().colors;
                let now = percent(colors.track.a) as i32;
                let next = (now + dir * 10).clamp(0, 100) as u32;
                colors.track.a = alpha_of(next);
                self.save_colors(store, colors);
            }
            Target::ResetColors => self.save_colors(store, Colors::default()),
            Target::Step(field, dir) => match field.stepped(&store.config().general(), dir) {
                Ok(general) => {
                    if let Err(e) = store.set_general(general) {
                        self.message = Some(e.to_string());
                    }
                }
                Err(e) => self.message = Some(e),
            },
        }
        None
    }

    /// The colour picker returned `color` for `slot`.
    pub fn set_color(&mut self, store: &mut Store, slot: Slot, color: Rgba) {
        let mut colors = store.config().colors;
        if slot.get(&colors) == color {
            return;
        }
        slot.set(&mut colors, color);
        self.message = None;
        self.save_colors(store, colors);
    }

    fn save_colors(&mut self, store: &mut Store, colors: Colors) {
        if colors == store.config().colors {
            return;
        }
        if let Err(e) = store.set_colors(colors) {
            self.message = Some(e.to_string());
        }
    }

    /// Paint the page. `text` draws a string in an area with a colour and an alignment.
    pub fn paint(
        &self,
        canvas: &mut Canvas,
        store: &Store,
        layout: &Layout,
        text: &mut dyn FnMut(&mut Canvas, &str, Rect, Rgba, Align),
    ) {
        let config = store.config();
        let colors = &config.colors;
        paint_preview(canvas, layout.preview, colors, text);
        text(
            canvas,
            strings::GENERAL_COLORS,
            layout.colors_heading,
            HEADING,
            Align::Left,
        );
        text(
            canvas,
            strings::GENERAL_TIMES,
            layout.times_heading,
            HEADING,
            Align::Left,
        );
        for row in &layout.colors {
            let color = row.slot.get(colors);
            text(canvas, row.slot.label(), row.label, LABEL, Align::Left);
            fill(canvas, grow(row.swatch, 1), OUTLINE, 5.0);
            fill(canvas, row.swatch, Rgba::rgb(0x20, 0x21, 0x24), 4.0);
            fill(canvas, row.swatch, color, 4.0);
            text(canvas, &color.to_hex(), row.hex, HEADING, Align::Left);
        }
        let opacity = format!("{}%", percent(colors.track.a));
        paint_stepper(
            canvas,
            &layout.opacity,
            strings::FIELD_TRACK_OPACITY,
            &opacity,
            (colors.track.a > 0, percent(colors.track.a) < 100),
            text,
        );
        fill(canvas, layout.reset, BUTTON, 6.0);
        text(
            canvas,
            strings::BUTTON_RESET_COLORS,
            layout.reset,
            BUTTON_TEXT,
            Align::Center,
        );
        let general = config.general();
        for row in &layout.fields {
            paint_stepper(
                canvas,
                &row.stepper,
                row.field.label(),
                &row.field.display(&general),
                (
                    row.field.can_step(&general, -1),
                    row.field.can_step(&general, 1),
                ),
                text,
            );
        }
        if let Some(message) = &self.message {
            text(canvas, message, layout.message, ERROR, Align::Left);
        }
    }
}

fn grow(r: Rect, by: i32) -> Rect {
    Rect::new(r.x - by, r.y - by, r.w + 2 * by, r.h + 2 * by)
}

fn fill(canvas: &mut Canvas, r: Rect, color: Rgba, radius: f32) {
    canvas.fill_rounded_rect(
        RectF::new(r.x as f32, r.y as f32, r.w as f32, r.h as f32),
        radius,
        color,
        None,
    );
}

fn paint_stepper(
    canvas: &mut Canvas,
    s: &Stepper,
    label: &str,
    value: &str,
    (can_minus, can_plus): (bool, bool),
    text: &mut dyn FnMut(&mut Canvas, &str, Rect, Rgba, Align),
) {
    text(canvas, label, s.label, LABEL, Align::Left);
    for (r, glyph, enabled) in [(s.minus, "-", can_minus), (s.plus, "+", can_plus)] {
        fill(canvas, r, BUTTON, 6.0);
        let color = if enabled { BUTTON_TEXT } else { DISABLED_TEXT };
        text(canvas, glyph, r, color, Align::Center);
    }
    text(canvas, value, s.value, LABEL, Align::Center);
}

/// A small bar in the chosen colours: green, orange and red, a break, more green, then the
/// part that is not filled yet, with the text colour on top.
fn paint_preview(
    canvas: &mut Canvas,
    area: Rect,
    colors: &Colors,
    text: &mut dyn FnMut(&mut Canvas, &str, Rect, Rgba, Align),
) {
    fill(canvas, area, colors.track, 0.0);
    let at = |share: f32| area.x + (area.w as f32 * share).round() as i32;
    let part = |from: f32, to: f32, color: Rgba, canvas: &mut Canvas| {
        fill(
            canvas,
            Rect::new(at(from), area.y, at(to) - at(from), area.h),
            color,
            0.0,
        );
    };
    part(0.0, 0.30, colors.green, canvas);
    part(0.30, 0.40, colors.orange, canvas);
    part(0.40, 0.48, colors.red, canvas);
    part(0.48, 0.56, colors.break_gap, canvas);
    part(0.56, 0.72, colors.green, canvas);
    text(
        canvas,
        strings::PREVIEW_TEXT,
        area,
        colors.text,
        Align::Center,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::Warnings;

    const AREA: Rect = Rect::new(0, 40, 860, 500);

    #[test]
    fn every_colour_and_number_has_a_row_and_nothing_overlaps() {
        let l = layout(AREA, 1.0);
        assert_eq!(l.colors.len(), 6);
        assert_eq!(l.fields.len(), 5);
        let mut rects = vec![l.preview, l.reset, l.opacity.minus, l.opacity.plus];
        rects.extend(l.colors.iter().flat_map(|r| [r.swatch]));
        for f in &l.fields {
            rects.extend([f.stepper.minus, f.stepper.value, f.stepper.plus]);
        }
        for (i, a) in rects.iter().enumerate() {
            assert!(a.w > 0 && a.h > 0);
            for b in &rects[i + 1..] {
                assert_eq!(a.overlap_area(b), 0, "{a:?} {b:?}");
            }
        }
        assert!(l.reset.bottom() <= AREA.bottom());
        assert!(
            l.colors[0].swatch.x < l.fields[0].stepper.minus.x,
            "two columns"
        );
    }

    #[test]
    fn it_fits_the_smallest_window() {
        let l = layout(Rect::new(0, 40, 640 - 16, 500 - 79 - 40), 1.0);
        assert!(l.reset.bottom() <= 40 + 500 - 79 - 40, "{:?}", l.reset);
        let last = &l.fields[4].stepper;
        assert!(last.plus.right() <= 640 - 16, "{last:?}");
        assert!(l.message.bottom() <= 40 + 500 - 79 - 40 + 60);
    }

    #[test]
    fn the_layout_scales_with_the_display() {
        let one = layout(AREA, 1.0);
        let two = layout(Rect::new(0, 80, 1720, 1000), 2.0);
        assert_eq!(two.colors[0].label.h, one.colors[0].label.h * 2);
        assert_eq!(two.opacity.minus.w, one.opacity.minus.w * 2);
    }

    #[test]
    fn clicks_pick_the_target_under_them() {
        let l = layout(AREA, 1.0);
        let c = |r: Rect| (r.x + r.w / 2, r.y + r.h / 2);
        let at = |r: Rect| {
            let (x, y) = c(r);
            l.target_at(x, y)
        };
        assert_eq!(at(l.colors[4].swatch), Some(Target::Pick(Slot::Track)));
        assert_eq!(at(l.colors[0].hex), Some(Target::Pick(Slot::Green)));
        assert_eq!(at(l.opacity.minus), Some(Target::Opacity(-1)));
        assert_eq!(at(l.opacity.plus), Some(Target::Opacity(1)));
        assert_eq!(at(l.reset), Some(Target::ResetColors));
        assert_eq!(
            at(l.fields[3].stepper.plus),
            Some(Target::Step(Field::Orange, 1))
        );
        assert_eq!(
            at(l.fields[0].stepper.minus),
            Some(Target::Step(Field::LeadIn, -1))
        );
        assert_eq!(
            at(l.fields[0].stepper.value),
            None,
            "the value is no button"
        );
        assert_eq!(at(l.preview), None);
    }

    fn general() -> General {
        General {
            lead_in: 5 * MINUTE,
            linger: 10,
            default_break: 10 * MINUTE,
            warnings: Warnings::default(),
        }
    }

    #[test]
    fn stepping_changes_one_value_by_its_step() {
        let g = general();
        assert_eq!(Field::LeadIn.stepped(&g, 1).unwrap().lead_in, 6 * MINUTE);
        assert_eq!(Field::LeadIn.stepped(&g, -1).unwrap().lead_in, 4 * MINUTE);
        assert_eq!(Field::Linger.stepped(&g, 1).unwrap().linger, 15);
        assert_eq!(
            Field::DefaultBreak.stepped(&g, -1).unwrap().default_break,
            9 * MINUTE
        );
        assert_eq!(
            Field::Orange.stepped(&g, 1).unwrap().warnings.orange,
            11 * MINUTE
        );
        assert_eq!(Field::Red.stepped(&g, -1).unwrap().warnings.red, 4 * MINUTE);
        assert_eq!(
            Field::Red.stepped(&g, 1).unwrap().default_break,
            g.default_break
        );
    }

    #[test]
    fn values_stop_at_their_limits_with_a_reason() {
        let g = General {
            lead_in: 0,
            ..general()
        };
        assert!(!Field::LeadIn.can_step(&g, -1) && Field::LeadIn.can_step(&g, 1));
        assert_eq!(
            Field::LeadIn.stepped(&g, -1).unwrap_err(),
            "Lead-in cannot go below 0 min"
        );
        let g = General {
            default_break: General::MAX_BREAK,
            ..general()
        };
        assert_eq!(
            Field::DefaultBreak.stepped(&g, 1).unwrap_err(),
            "Default break cannot go above 30 min"
        );
        let g = General {
            default_break: General::MIN_BREAK,
            ..general()
        };
        assert!(Field::DefaultBreak.stepped(&g, -1).is_err());
        let g = General {
            linger: 300,
            ..general()
        };
        assert!(Field::Linger.stepped(&g, 1).is_err());
        let g = General {
            linger: 3,
            ..general()
        };
        assert_eq!(
            Field::Linger.stepped(&g, -1).unwrap().linger,
            0,
            "never below 0"
        );
    }

    #[test]
    fn red_cannot_pass_orange() {
        let g = general(); // orange 10, red 5
        let up = General {
            warnings: Warnings {
                orange: 10 * MINUTE,
                red: 10 * MINUTE,
            },
            ..g
        };
        assert!(Field::Red.stepped(&up, 1).is_err());
        assert_eq!(
            Field::Orange.stepped(&up, -1).unwrap_err(),
            "Red must not be longer than orange"
        );
        assert!(Field::Red.stepped(&g, 1).is_ok());
    }

    #[test]
    fn opacity_is_a_whole_percentage() {
        assert_eq!(percent(0x99), 60);
        assert_eq!(alpha_of(60), 0x99);
        assert_eq!((percent(0), percent(255)), (0, 100));
        assert_eq!((alpha_of(0), alpha_of(100), alpha_of(150)), (0, 255, 255));
        for p in 0..=100 {
            assert_eq!(percent(alpha_of(p)), p);
        }
    }

    #[test]
    fn slots_read_and_write_their_own_colour() {
        let mut colors = Colors::default();
        for (i, slot) in Slot::ALL.iter().enumerate() {
            slot.set(&mut colors, Rgba::rgb(i as u8, 9, 9));
        }
        for (i, slot) in Slot::ALL.iter().enumerate() {
            assert_eq!(slot.get(&colors), Rgba::rgb(i as u8, 9, 9));
        }
        assert_eq!(Slot::Break.label(), "Break");
    }

    #[test]
    fn painting_shows_every_text_and_the_colours() {
        let t = tests_store("paint");
        let store = &t.0;
        let l = layout(AREA, 1.0);
        let mut canvas = Canvas::new(860, 540);
        let mut drawn: Vec<String> = Vec::new();
        Page::default().paint(&mut canvas, store, &l, &mut |_, t, _, _, _| {
            drawn.push(t.to_string());
        });
        for expected in [
            "Colours",
            "Times",
            "Green",
            "#2ecc71",
            "#33333399",
            "60%",
            "5 min",
            "10 s",
            "Reset colours",
            "Lead-in",
        ] {
            assert!(
                drawn.iter().any(|t| t == expected),
                "{expected} in {drawn:?}"
            );
        }
        let colors = store.config().colors;
        let solid =
            |c: Rgba| 0xff00_0000 | u32::from(c.r) << 16 | u32::from(c.g) << 8 | u32::from(c.b);
        let row = &l.colors[0];
        assert_eq!(
            canvas.pixel((row.swatch.x + 5) as u32, (row.swatch.y + 12) as u32),
            solid(colors.green)
        );
        let p = l.preview;
        assert_eq!(
            canvas.pixel((p.x + 20) as u32, (p.y + 5) as u32),
            solid(colors.green)
        );
        assert_eq!(
            canvas.pixel((p.x + p.w * 35 / 100) as u32, (p.y + 5) as u32),
            solid(colors.orange)
        );
    }

    /// A store on a scratch file with the default settings.
    struct TestStore(Store, std::path::PathBuf);

    impl Drop for TestStore {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.1);
        }
    }

    fn tests_store(name: &str) -> TestStore {
        use crate::config::{ConfigFile, FILE_NAME};
        let dir =
            std::env::temp_dir().join(format!("times-up-general-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(FILE_NAME), crate::config::DEFAULT_CONFIG).unwrap();
        let mut file = ConfigFile::new(dir.join(FILE_NAME));
        let config = file.load_or_create().config;
        TestStore(Store::new(config, Some(file)), dir)
    }

    fn press_at(page: &mut Page, store: &mut Store, r: Rect) -> Option<(Slot, Rgba)> {
        page.press(store, &layout(AREA, 1.0), r.x + r.w / 2, r.y + r.h / 2)
    }

    #[test]
    fn clicking_steppers_saves_and_a_refused_step_shows_a_message() {
        let mut t = tests_store("clicks");
        let l = layout(AREA, 1.0);
        let mut page = Page::default();
        press_at(&mut page, &mut t.0, l.fields[0].stepper.plus);
        assert_eq!(t.0.config().general().lead_in, 6 * MINUTE);
        assert_eq!(
            t.0.schedule().lead_in,
            6 * MINUTE,
            "applied without a restart"
        );
        assert!(
            std::fs::read_to_string(t.0.path().unwrap())
                .unwrap()
                .contains("lead_in_minutes = 6")
        );
        assert_eq!(page.message(), None);

        // Orange 10 -> 5 is fine, one more would put it below red.
        for _ in 0..5 {
            press_at(&mut page, &mut t.0, l.fields[3].stepper.minus);
        }
        assert_eq!(t.0.config().general().warnings.orange, 5 * MINUTE);
        assert_eq!(page.message(), None);
        let revision = t.0.revision();
        press_at(&mut page, &mut t.0, l.fields[3].stepper.minus);
        assert_eq!(page.message(), Some("Red must not be longer than orange"));
        assert_eq!(t.0.config().general().warnings.orange, 5 * MINUTE);
        assert_eq!(t.0.revision(), revision, "refused: nothing was saved");
        press_at(&mut page, &mut t.0, l.fields[0].stepper.plus);
        assert_eq!(page.message(), None, "the next click clears it");
    }

    #[test]
    fn opacity_and_reset_change_the_colours() {
        let mut t = tests_store("colors");
        let l = layout(AREA, 1.0);
        let mut page = Page::default();
        press_at(&mut page, &mut t.0, l.opacity.plus);
        assert_eq!(percent(t.0.config().colors.track.a), 70);
        press_at(&mut page, &mut t.0, l.opacity.minus);
        press_at(&mut page, &mut t.0, l.opacity.minus);
        assert_eq!(percent(t.0.config().colors.track.a), 50);

        let pick = press_at(&mut page, &mut t.0, l.colors[2].swatch);
        assert_eq!(pick, Some((Slot::Red, Colors::default().red)));
        page.set_color(&mut t.0, Slot::Red, Rgba::rgb(1, 2, 3));
        assert_eq!(t.0.config().colors.red, Rgba::rgb(1, 2, 3));
        assert!(
            std::fs::read_to_string(t.0.path().unwrap())
                .unwrap()
                .contains("red = \"#010203\"")
        );

        press_at(&mut page, &mut t.0, l.reset);
        assert_eq!(t.0.config().colors, Colors::default());
    }

    #[test]
    fn without_a_settings_file_changes_are_refused_with_a_message() {
        let mut store = Store::new(crate::config::Config::default(), None);
        let l = layout(AREA, 1.0);
        let mut page = Page::default();
        press_at(&mut page, &mut store, l.fields[0].stepper.plus);
        assert!(page.message().is_some());
        assert_eq!(store.config().general().lead_in, 5 * MINUTE);
        page.set_color(&mut store, Slot::Red, Rgba::rgb(1, 2, 3));
        assert_eq!(store.config().colors, Colors::default());
    }
}
