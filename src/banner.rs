//! The banner of the Settings window that asks what to do with the temporary changes of the
//! running class when it is edited in Settings: Replace them with the saved settings, or Save
//! only (keep them on the bar until the class ends). Plain geometry and painting, like
//! [`crate::detail`]; the question itself is handled by the window.

use crate::canvas::{Align, Canvas, RectF};
use crate::config::Rgba;
use crate::interaction::Rect;
use crate::strings;

const HEIGHT: f32 = 52.0;
const SIDE: f32 = 12.0;
const BUTTON_W: f32 = 140.0;
const BUTTON_H: f32 = 32.0;
const GAP: f32 = 8.0;

const BACKGROUND: Rgba = Rgba::rgb(0x5a, 0x45, 0x12);
const BUTTON: Rgba = Rgba::rgb(0x7a, 0x62, 0x24);
/// The default answer (Enter) stands out.
const DEFAULT_BUTTON: Rgba = Rgba::rgb(0xe0, 0xa8, 0x20);
const TEXT: Rgba = Rgba::rgb(0xf5, 0xf0, 0xe0);
const DEFAULT_TEXT: Rgba = Rgba::rgb(0x20, 0x16, 0x00);

/// What to do with the temporary changes of the running class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// The bar shows the saved class again.
    Replace,
    /// The temporary changes stay on the bar until the class ends.
    SaveOnly,
}

/// Where the banner sits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub area: Rect,
    pub text: Rect,
    pub replace: Rect,
    pub save_only: Rect,
}

/// Take the banner off the top of `content`: its area and what is left.
pub fn split(content: Rect, scale: f32) -> (Rect, Rect) {
    let h = ((HEIGHT * scale).round() as i32).min(content.h);
    (
        Rect::new(content.x, content.y, content.w, h),
        Rect::new(content.x, content.y + h, content.w, content.h - h),
    )
}

pub fn layout(area: Rect, scale: f32) -> Layout {
    let px = |v: f32| (v * scale).round() as i32;
    let (side, w, h, gap) = (px(SIDE), px(BUTTON_W), px(BUTTON_H), px(GAP));
    let y = area.y + (area.h - h) / 2;
    let save_only = Rect::new(area.right() - side - w, y, w, h);
    let replace = Rect::new(save_only.x - gap - w, y, w, h);
    Layout {
        area,
        text: Rect::new(
            area.x + side,
            area.y,
            (replace.x - gap - area.x - side).max(1),
            area.h,
        ),
        replace,
        save_only,
    }
}

impl Layout {
    /// The answer whose button is at `(x, y)`.
    pub fn answer_at(&self, x: i32, y: i32) -> Option<Answer> {
        let inside = |r: Rect| x >= r.x && x < r.right() && y >= r.y && y < r.bottom();
        if inside(self.replace) {
            Some(Answer::Replace)
        } else if inside(self.save_only) {
            Some(Answer::SaveOnly)
        } else {
            None
        }
    }
}

/// Paint the banner. `text` draws a string in an area with an alignment and a colour.
pub fn paint(
    canvas: &mut Canvas,
    layout: &Layout,
    text: &mut dyn FnMut(&mut Canvas, &str, Rect, Rgba, Align),
) {
    fill(canvas, layout.area, BACKGROUND, 0.0);
    text(canvas, strings::BANNER_TEXT, layout.text, TEXT, Align::Left);
    fill(canvas, layout.replace, DEFAULT_BUTTON, 6.0);
    text(
        canvas,
        strings::BANNER_REPLACE,
        layout.replace,
        DEFAULT_TEXT,
        Align::Center,
    );
    fill(canvas, layout.save_only, BUTTON, 6.0);
    text(
        canvas,
        strings::BANNER_SAVE_ONLY,
        layout.save_only,
        TEXT,
        Align::Center,
    );
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
    use super::*;

    const CONTENT: Rect = Rect::new(0, 40, 860, 500);

    #[test]
    fn the_banner_is_the_top_of_the_content() {
        let (banner, rest) = split(CONTENT, 1.0);
        assert_eq!(banner, Rect::new(0, 40, 860, 52));
        assert_eq!(rest, Rect::new(0, 92, 860, 448));
        let (banner, rest) = split(Rect::new(0, 0, 100, 20), 1.0);
        assert_eq!((banner.h, rest.h), (20, 0), "never more than there is");
    }

    #[test]
    fn the_buttons_sit_at_the_right_and_the_text_gets_the_rest() {
        let (area, _) = split(CONTENT, 1.0);
        let l = layout(area, 1.0);
        assert!(l.text.right() <= l.replace.x && l.replace.right() < l.save_only.x);
        assert!(l.save_only.right() <= area.right());
        assert!(l.replace.y >= area.y && l.replace.bottom() <= area.bottom());
        assert_eq!(l.replace.w, 140);
    }

    #[test]
    fn a_click_picks_the_answer_of_the_button_under_it() {
        let (area, _) = split(CONTENT, 1.0);
        let l = layout(area, 1.0);
        let at = |r: Rect| l.answer_at(r.x + 2, r.y + 2);
        assert_eq!(at(l.replace), Some(Answer::Replace));
        assert_eq!(at(l.save_only), Some(Answer::SaveOnly));
        assert_eq!(l.answer_at(l.text.x, l.text.y), None);
    }

    #[test]
    fn painting_shows_the_question_and_both_answers() {
        let (area, _) = split(CONTENT, 1.0);
        let l = layout(area, 1.0);
        let mut canvas = Canvas::new(860, 600);
        let mut drawn: Vec<(String, Align)> = Vec::new();
        paint(&mut canvas, &l, &mut |_, t, _, _, a| {
            drawn.push((t.to_string(), a));
        });
        assert_eq!(
            drawn,
            vec![
                (strings::BANNER_TEXT.to_string(), Align::Left),
                (strings::BANNER_REPLACE.to_string(), Align::Center),
                (strings::BANNER_SAVE_ONLY.to_string(), Align::Center),
            ]
        );
        assert_eq!(canvas.pixel(3, 45), 0xff5a_4512, "the strip is filled");
        let default = canvas.pixel(l.replace.x as u32 + 70, l.replace.y as u32 + 16);
        assert_eq!(default, 0xffe0_a820, "Replace stands out as the default");
    }
}
