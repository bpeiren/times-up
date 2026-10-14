//! A tiny software canvas: premultiplied BGRA pixels, anti-aliased rounded rectangles and text
//! masks. It is what the window shows (via a layered window), so no GPU or graphics library is
//! needed and memory use stays at a few bytes per pixel.

use crate::config::Rgba;

/// A rectangle in pixel coordinates; fractional edges are anti-aliased.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RectF {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl RectF {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
}

pub struct Canvas {
    width: u32,
    height: u32,
    /// `0xAARRGGBB` with the colour channels premultiplied by alpha (what Windows expects).
    px: Vec<u32>,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            px: vec![0; (width as usize) * (height as usize)],
        }
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u32] {
        &self.px
    }

    /// The pixel at `(x, y)`, or fully transparent outside the canvas.
    pub fn pixel(&self, x: u32, y: u32) -> u32 {
        if x < self.width && y < self.height {
            self.px[(y * self.width + x) as usize]
        } else {
            0
        }
    }

    /// Make every pixel fully transparent.
    pub fn clear(&mut self) {
        self.px.fill(0);
    }

    /// Source-over a rounded rectangle of `color`, optionally limited to `clip`.
    pub fn fill_rounded_rect(
        &mut self,
        rect: RectF,
        radius: f32,
        color: Rgba,
        clip: Option<RectF>,
    ) {
        let clip = clip.unwrap_or(RectF::new(0.0, 0.0, self.width as f32, self.height as f32));
        let x0 = rect.x.max(clip.x).max(0.0).floor() as u32;
        let y0 = rect.y.max(clip.y).max(0.0).floor() as u32;
        let x1 = (rect.x + rect.w)
            .min(clip.x + clip.w)
            .min(self.width as f32)
            .ceil()
            .max(0.0) as u32;
        let y1 = (rect.y + rect.h)
            .min(clip.y + clip.h)
            .min(self.height as f32)
            .ceil()
            .max(0.0) as u32;
        let radius = radius.clamp(0.0, rect.w.min(rect.h) / 2.0);
        let (cx, cy) = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
        let (hx, hy) = (rect.w / 2.0 - radius, rect.h / 2.0 - radius);

        for y in y0..y1 {
            for x in x0..x1 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                // Signed distance to the rounded rectangle: negative inside.
                let (qx, qy) = ((px - cx).abs() - hx, (py - cy).abs() - hy);
                let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
                let d = outside + qx.max(qy).min(0.0) - radius;
                let shape = (0.5 - d).clamp(0.0, 1.0);
                if shape <= 0.0 {
                    continue;
                }
                let coverage =
                    shape * overlap(x as f32, clip.x, clip.w) * overlap(y as f32, clip.y, clip.h);
                self.blend(x, y, color, coverage);
            }
        }
    }

    /// Source-over `color` through a coverage mask of `mw` x `mh` bytes placed at `(x, y)`.
    pub fn blend_mask(&mut self, mask: &[u8], mw: u32, mh: u32, x: i32, y: i32, color: Rgba) {
        for my in 0..mh {
            for mx in 0..mw {
                let Some(&m) = mask.get((my * mw + mx) as usize) else {
                    return;
                };
                let (cx, cy) = (x + mx as i32, y + my as i32);
                if m > 0 && cx >= 0 && cy >= 0 {
                    self.blend(cx as u32, cy as u32, color, f32::from(m) / 255.0);
                }
            }
        }
    }

    fn blend(&mut self, x: u32, y: u32, color: Rgba, coverage: f32) {
        if x >= self.width || y >= self.height {
            return;
        }
        let alpha = (f32::from(color.a) * coverage.clamp(0.0, 1.0)).round() as u32;
        let src = premultiply(color, alpha);
        let dst = &mut self.px[(y * self.width + x) as usize];
        *dst = over(*dst, src);
    }
}

/// How much of the unit pixel starting at `p` lies inside `[start, start + len)`.
fn overlap(p: f32, start: f32, len: f32) -> f32 {
    ((p + 1.0).min(start + len) - p.max(start)).clamp(0.0, 1.0)
}

fn premultiply(c: Rgba, alpha: u32) -> u32 {
    let ch = |v: u8| (u32::from(v) * alpha + 127) / 255;
    (alpha << 24) | (ch(c.r) << 16) | (ch(c.g) << 8) | ch(c.b)
}

/// `src` over `dst`, both premultiplied.
fn over(dst: u32, src: u32) -> u32 {
    let inv = 255 - (src >> 24);
    let ch = |shift: u32| {
        let s = (src >> shift) & 0xff;
        let d = (dst >> shift) & 0xff;
        (s + (d * inv + 127) / 255).min(255) << shift
    };
    ch(24) | ch(16) | ch(8) | ch(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Rgba = Rgba::rgb(255, 0, 0);
    const BLUE: Rgba = Rgba::rgb(0, 0, 255);

    fn argb(a: u32, r: u32, g: u32, b: u32) -> u32 {
        a << 24 | r << 16 | g << 8 | b
    }

    #[test]
    fn new_canvas_is_transparent() {
        let c = Canvas::new(4, 3);
        assert_eq!((c.width(), c.height()), (4, 3));
        assert_eq!(c.pixels().len(), 12);
        assert!(c.pixels().iter().all(|&p| p == 0));
        assert_eq!(
            c.pixel(99, 99),
            0,
            "outside the canvas reads as transparent"
        );
    }

    #[test]
    fn solid_rect_fills_exactly_its_pixels() {
        let mut c = Canvas::new(10, 6);
        c.fill_rounded_rect(RectF::new(2.0, 1.0, 5.0, 4.0), 0.0, RED, None);
        for y in 0..6 {
            for x in 0..10 {
                let inside = (2..7).contains(&x) && (1..5).contains(&y);
                assert_eq!(
                    c.pixel(x, y),
                    if inside { argb(255, 255, 0, 0) } else { 0 },
                    "({x},{y})"
                );
            }
        }
    }

    #[test]
    fn colour_is_premultiplied_by_alpha() {
        let mut c = Canvas::new(2, 1);
        c.fill_rounded_rect(
            RectF::new(0.0, 0.0, 2.0, 1.0),
            0.0,
            Rgba {
                r: 200,
                g: 100,
                b: 0,
                a: 128,
            },
            None,
        );
        assert_eq!(c.pixel(0, 0), argb(128, 100, 50, 0));
    }

    #[test]
    fn rounded_corners_are_cut_but_the_middle_is_solid() {
        let mut c = Canvas::new(40, 20);
        c.fill_rounded_rect(RectF::new(0.0, 0.0, 40.0, 20.0), 10.0, RED, None);
        assert_eq!(c.pixel(0, 0) >> 24, 0, "corner pixel is transparent");
        assert_eq!(c.pixel(39, 19) >> 24, 0);
        assert_eq!(c.pixel(20, 10), argb(255, 255, 0, 0));
        assert_eq!(
            c.pixel(20, 0),
            argb(255, 255, 0, 0),
            "straight edge stays solid"
        );
        // The curve is anti-aliased: some pixel along it is partially covered.
        let partial = (0..10).any(|y| (0..10).any(|x| (1..255).contains(&(c.pixel(x, y) >> 24))));
        assert!(partial, "no anti-aliased pixels found on the corner");
    }

    #[test]
    fn radius_is_limited_to_half_the_short_side() {
        let mut c = Canvas::new(20, 10);
        c.fill_rounded_rect(RectF::new(0.0, 0.0, 20.0, 10.0), 500.0, RED, None);
        assert_eq!(c.pixel(10, 5), argb(255, 255, 0, 0));
        assert_eq!(c.pixel(0, 0) >> 24, 0);
    }

    #[test]
    fn fractional_edges_are_anti_aliased() {
        let mut c = Canvas::new(4, 1);
        c.fill_rounded_rect(RectF::new(0.0, 0.0, 1.5, 1.0), 0.0, RED, None);
        assert_eq!(c.pixel(0, 0), argb(255, 255, 0, 0));
        let half = c.pixel(1, 0);
        assert!((126..=129).contains(&(half >> 24)), "alpha {}", half >> 24);
        assert_eq!(c.pixel(2, 0), 0);
    }

    #[test]
    fn clip_limits_the_fill() {
        let mut c = Canvas::new(10, 4);
        let clip = RectF::new(0.0, 0.0, 4.0, 4.0);
        c.fill_rounded_rect(RectF::new(0.0, 0.0, 10.0, 4.0), 0.0, RED, Some(clip));
        assert_eq!(c.pixel(3, 2), argb(255, 255, 0, 0));
        assert_eq!(c.pixel(4, 2), 0);
        // A fractional clip edge blends.
        let mut c = Canvas::new(10, 4);
        c.fill_rounded_rect(
            RectF::new(0.0, 0.0, 10.0, 4.0),
            0.0,
            RED,
            Some(RectF::new(0.0, 0.0, 3.25, 4.0)),
        );
        assert_eq!(c.pixel(2, 0) >> 24, 255);
        assert!((63..=65).contains(&(c.pixel(3, 0) >> 24)));
    }

    #[test]
    fn shapes_outside_or_partly_outside_the_canvas_do_not_panic() {
        let mut c = Canvas::new(5, 5);
        c.fill_rounded_rect(RectF::new(-10.0, -10.0, 8.0, 8.0), 2.0, RED, None);
        c.fill_rounded_rect(RectF::new(3.0, 3.0, 50.0, 50.0), 4.0, RED, None);
        c.fill_rounded_rect(RectF::new(100.0, 100.0, 5.0, 5.0), 0.0, RED, None);
        c.fill_rounded_rect(RectF::new(0.0, 0.0, 0.0, 0.0), 0.0, RED, None);
        assert_eq!(c.pixel(0, 0), 0);
        // (4, 4) lies inside the rounded corner of the second rectangle (almost fully covered).
        assert!(c.pixel(4, 4) >> 24 >= 240);
    }

    #[test]
    fn later_shapes_blend_over_earlier_ones() {
        let mut c = Canvas::new(2, 1);
        c.fill_rounded_rect(RectF::new(0.0, 0.0, 2.0, 1.0), 0.0, RED, None);
        c.fill_rounded_rect(
            RectF::new(0.0, 0.0, 1.0, 1.0),
            0.0,
            Rgba {
                r: 0,
                g: 0,
                b: 255,
                a: 128,
            },
            None,
        );
        // 50% blue over opaque red: stays opaque, half red and half blue.
        assert_eq!(c.pixel(0, 0), argb(255, 127, 0, 128));
        assert_eq!(c.pixel(1, 0), argb(255, 255, 0, 0));
        c.fill_rounded_rect(RectF::new(0.0, 0.0, 1.0, 1.0), 0.0, BLUE, None);
        assert_eq!(c.pixel(0, 0), argb(255, 0, 0, 255));
    }

    #[test]
    fn semi_transparent_over_transparent_keeps_alpha() {
        let mut c = Canvas::new(1, 1);
        c.fill_rounded_rect(
            RectF::new(0.0, 0.0, 1.0, 1.0),
            0.0,
            Rgba {
                r: 255,
                g: 255,
                b: 255,
                a: 0x99,
            },
            None,
        );
        assert_eq!(c.pixel(0, 0), argb(0x99, 0x99, 0x99, 0x99));
    }

    #[test]
    fn clear_makes_everything_transparent() {
        let mut c = Canvas::new(3, 3);
        c.fill_rounded_rect(RectF::new(0.0, 0.0, 3.0, 3.0), 0.0, RED, None);
        c.clear();
        assert!(c.pixels().iter().all(|&p| p == 0));
    }

    #[test]
    fn mask_blends_the_colour_by_coverage() {
        let mut c = Canvas::new(4, 2);
        let mask = [255, 128, 0, 255];
        c.blend_mask(&mask, 2, 2, 1, 0, BLUE);
        assert_eq!(c.pixel(1, 0), argb(255, 0, 0, 255));
        assert!((127..=129).contains(&(c.pixel(2, 0) >> 24)));
        assert_eq!(c.pixel(1, 1), 0, "zero coverage leaves the pixel alone");
        assert_eq!(c.pixel(2, 1), argb(255, 0, 0, 255));
        assert_eq!(c.pixel(0, 0), 0);
    }

    #[test]
    fn mask_is_clipped_to_the_canvas_and_tolerates_short_data() {
        let mut c = Canvas::new(3, 3);
        c.blend_mask(&[255; 9], 3, 3, -1, -1, RED);
        assert_eq!(c.pixel(0, 0), argb(255, 255, 0, 0));
        assert_eq!(c.pixel(1, 1), argb(255, 255, 0, 0));
        assert_eq!(c.pixel(2, 2), 0);
        c.clear();
        c.blend_mask(&[255; 2], 3, 3, 0, 0, RED);
        assert_eq!(c.pixel(1, 0), argb(255, 255, 0, 0));
        assert_eq!(c.pixel(2, 0), 0);
    }
}
