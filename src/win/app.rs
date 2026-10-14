//! Startup: load the config, create the window, run until it closes.

use windows::Win32::System::SystemInformation::GetLocalTime;

use super::text::render_mask;
use super::window::{self, DEFAULT_HEIGHT, DEFAULT_WIDTH, Handler, Placement};
use crate::canvas::{Canvas, RectF};
use crate::config::{self, Config, ConfigFile};

struct App {
    config: Config,
}

impl Handler for App {
    /// Placeholder drawing until the real bar exists: the track, a fill that grows with the
    /// seconds of the current minute, and the clock. It shows that the redraw loop works.
    fn draw(&mut self, canvas: &mut Canvas) {
        let (w, h) = (canvas.width() as f32, canvas.height() as f32);
        let colors = self.config.colors;
        let full = RectF::new(0.0, 0.0, w, h);
        let radius = h / 4.0;
        canvas.fill_rounded_rect(full, radius, colors.track, None);

        // SAFETY: plain query of the local time.
        let now = unsafe { GetLocalTime() };
        let fill = w * f32::from(now.wSecond) / 60.0;
        canvas.fill_rounded_rect(
            full,
            radius,
            colors.green,
            Some(RectF::new(0.0, 0.0, fill, h)),
        );

        let clock = format!("{:02}:{:02}:{:02}", now.wHour, now.wMinute, now.wSecond);
        if let Some(mask) = render_mask(&clock, canvas.width(), canvas.height(), (h * 0.5) as i32) {
            canvas.blend_mask(&mask, canvas.width(), canvas.height(), 0, 0, colors.text);
        }
    }
}

/// Run the application; returns the process exit code.
pub fn run() -> i32 {
    window::enable_dpi_awareness();
    let scale = window::dpi_scale();

    // Problems in the file (typos, read-only folder) fall back to defaults; the app will show
    // them once there is somewhere to show them.
    let loaded = ConfigFile::new(config::default_path()).load_or_create();
    let config = loaded.config;

    let placement = Placement {
        x: config.window.x,
        y: config.window.y,
        width: config
            .window
            .width
            .unwrap_or((DEFAULT_WIDTH as f32 * scale) as u32),
        height: config
            .window
            .height
            .unwrap_or((DEFAULT_HEIGHT as f32 * scale) as u32),
    };
    match window::create(placement, Box::new(App { config })) {
        Ok(_) => window::run_message_loop(),
        Err(e) => {
            eprintln!("cannot create the window: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Rgba;

    #[test]
    fn placeholder_draws_track_fill_and_clock() {
        let mut canvas = Canvas::new(300, 40);
        let mut app = App {
            config: Config::default(),
        };
        app.draw(&mut canvas);

        // Rounded corners stay transparent; the middle of the bar is painted.
        assert_eq!(canvas.pixel(0, 0) >> 24, 0);
        assert_eq!(canvas.pixel(299, 39) >> 24, 0);
        assert!(canvas.pixel(150, 2) >> 24 > 0, "track");

        // The clock text is drawn in the text colour somewhere in the middle of the bar.
        let white = Rgba::rgb(255, 255, 255);
        let text = (0..40).flat_map(|y| (100..200).map(move |x| (x, y)));
        let inked = text
            .filter(|&(x, y)| {
                canvas.pixel(x, y)
                    == 0xff00_0000
                        | u32::from(white.r) << 16
                        | u32::from(white.g) << 8
                        | u32::from(white.b)
            })
            .count();
        assert!(inked > 50, "only {inked} text pixels");
    }

    #[test]
    fn redrawing_after_clear_gives_the_same_picture() {
        let mut app = App {
            config: Config::default(),
        };
        let mut a = Canvas::new(200, 30);
        app.draw(&mut a);
        // Two draws inside one second look identical; across a second boundary only the clock
        // and the fill change, never the transparent corners.
        let mut b = Canvas::new(200, 30);
        app.draw(&mut b);
        assert_eq!(a.pixel(0, 0), b.pixel(0, 0));
        assert_eq!(a.pixel(199, 29), b.pixel(199, 29));
    }
}
