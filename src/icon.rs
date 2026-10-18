//! The notification-area icon: a tiny version of the bar itself. It is drawn at run time on the
//! software canvas, at whatever size Windows asks for, so no icon resource is needed.

use crate::canvas::{Canvas, RectF};
use crate::config::{Colors, Rgba};

/// The bar as a glyph: a dark rounded track, filled left to right in the three zone colours.
/// `size` is the width and height in pixels (16 at 96 DPI, larger on high-DPI displays).
pub fn tray_icon(size: u32, colors: &Colors) -> Canvas {
    let mut canvas = Canvas::new(size, size);
    if size == 0 {
        return canvas;
    }
    let s = size as f32;
    // A bar across the middle: a third of the height, inset a little from the sides.
    let inset = (s * 0.06).max(0.5);
    let height = (s * 0.5).max(2.0);
    let bar = RectF::new(inset, (s - height) / 2.0, s - 2.0 * inset, height);
    let radius = height / 3.0;
    // Opaque, so the icon reads on both light and dark taskbars.
    let track = Rgba::rgb(0x2b, 0x31, 0x38);
    canvas.fill_rounded_rect(bar, radius, track, None);

    // The lesson zone for most of the hour, then the warning and final minutes, like a class nearly over.
    let zones = [
        (0.0, 0.6, colors.lesson),
        (0.6, 0.82, colors.warning),
        (0.82, 1.0, colors.final_),
    ];
    for (from, to, color) in zones {
        let clip = RectF::new(bar.x + bar.w * from, bar.y, bar.w * (to - from), bar.h);
        canvas.fill_rounded_rect(bar, radius, opaque(color), Some(clip));
    }
    canvas
}

fn opaque(c: Rgba) -> Rgba {
    Rgba { a: 255, ..c }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha(c: &Canvas, x: u32, y: u32) -> u32 {
        c.pixel(x, y) >> 24
    }

    fn solid(c: Rgba) -> u32 {
        0xff00_0000 | u32::from(c.r) << 16 | u32::from(c.g) << 8 | u32::from(c.b)
    }

    #[test]
    fn the_icon_has_the_requested_size() {
        for size in [16, 20, 24, 32, 48] {
            let icon = tray_icon(size, &Colors::default());
            assert_eq!((icon.width(), icon.height()), (size, size));
        }
        assert_eq!(tray_icon(0, &Colors::default()).pixels().len(), 0);
    }

    #[test]
    fn it_shows_the_three_zone_colours_from_left_to_right() {
        let c = Colors::default();
        let icon = tray_icon(64, &c);
        let y = 32;
        assert_eq!(icon.pixel(12, y), solid(c.lesson), "lesson on the left");
        assert_eq!(icon.pixel(45, y), solid(c.warning), "warning next");
        assert_eq!(icon.pixel(58, y), solid(c.final_), "final at the end");
    }

    #[test]
    fn the_glyph_is_a_bar_in_the_middle_with_transparent_surroundings() {
        let icon = tray_icon(32, &Colors::default());
        assert_eq!(alpha(&icon, 0, 0), 0, "corner");
        assert_eq!(alpha(&icon, 16, 2), 0, "above the bar");
        assert_eq!(alpha(&icon, 16, 29), 0, "below the bar");
        assert_eq!(alpha(&icon, 16, 16), 255, "the bar itself is opaque");
    }

    #[test]
    fn it_is_still_drawn_at_the_smallest_size() {
        let icon = tray_icon(16, &Colors::default());
        let opaque = icon.pixels().iter().filter(|&&p| p >> 24 == 255).count();
        assert!(opaque > 40, "only {opaque} solid pixels");
        let c = Colors::default();
        let row = (0..16).map(|x| icon.pixel(x, 8)).collect::<Vec<_>>();
        assert!(
            row.contains(&solid(c.lesson))
                && row.contains(&solid(c.warning))
                && row.contains(&solid(c.final_))
        );
    }

    #[test]
    fn the_icon_follows_the_configured_colours() {
        let c = Colors {
            lesson: Rgba::rgb(1, 2, 3),
            // Transparency is ignored: an icon must stay solid.
            final_: Rgba {
                r: 9,
                g: 8,
                b: 7,
                a: 10,
            },
            ..Colors::default()
        };
        let icon = tray_icon(64, &c);
        assert_eq!(icon.pixel(12, 32), solid(c.lesson));
        assert_eq!(icon.pixel(58, 32), solid(Rgba::rgb(9, 8, 7)));
    }
}
