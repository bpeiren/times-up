//! Startup and the per-tick work: load the config, create the window, and each second decide
//! what the bar shows from the local time and the schedule.

use windows::Win32::System::SystemInformation::GetLocalTime;

use super::text::render_mask;
use super::window::{self, DEFAULT_HEIGHT, DEFAULT_WIDTH, Handler, Placement};
use crate::bar;
use crate::canvas::Canvas;
use crate::config::{self, Config, ConfigFile};
use crate::schedule::{Secs, Weekday};

/// Aim this many milliseconds past a second boundary so the clock has surely ticked over.
const TICK_MARGIN_MS: u32 = 25;
/// Label height as a share of the bar height.
const LABEL_SCALE: f32 = 0.5;

struct App {
    config: Config,
    /// Where the config came from; polled each tick so edits to the file apply immediately.
    file: Option<ConfigFile>,
    /// Milliseconds past the second at the last tick, used to time the next one.
    last_ms: u32,
}

impl App {
    fn new(config: Config, file: Option<ConfigFile>) -> Self {
        Self {
            config,
            file,
            last_ms: 0,
        }
    }

    /// Paint the bar for `day` at `now` seconds after midnight. `false` means nothing to show.
    fn draw_at(&mut self, canvas: &mut Canvas, day: Weekday, now: Secs) -> bool {
        let Some(view) = bar::view(&self.config.schedule, day, now) else {
            return false;
        };
        bar::paint(canvas, &view, &self.config.colors);
        let (w, h) = (canvas.width(), canvas.height());
        let font_px = (h as f32 * LABEL_SCALE) as i32;
        if let Some(mask) = render_mask(&view.label, w, h, font_px) {
            canvas.blend_mask(&mask, w, h, 0, 0, self.config.colors.text);
        }
        true
    }
}

impl Handler for App {
    fn draw(&mut self, canvas: &mut Canvas) -> bool {
        if let Some(file) = &mut self.file
            && let Some(loaded) = file.poll()
        {
            self.config = loaded.config;
        }
        let now = local_now();
        self.last_ms = now.ms;
        self.draw_at(canvas, now.day, now.secs)
    }

    fn next_tick_ms(&self) -> u32 {
        next_tick_delay(self.last_ms)
    }
}

struct Now {
    day: Weekday,
    secs: Secs,
    ms: u32,
}

fn local_now() -> Now {
    // SAFETY: plain query of the local time.
    let t = unsafe { GetLocalTime() };
    Now {
        day: weekday_from_windows(t.wDayOfWeek),
        secs: u32::from(t.wHour) * 3600 + u32::from(t.wMinute) * 60 + u32::from(t.wSecond),
        ms: u32::from(t.wMilliseconds),
    }
}

/// Windows counts days from Sunday = 0.
fn weekday_from_windows(day_of_week: u16) -> Weekday {
    match day_of_week {
        1 => Weekday::Mon,
        2 => Weekday::Tue,
        3 => Weekday::Wed,
        4 => Weekday::Thu,
        5 => Weekday::Fri,
        6 => Weekday::Sat,
        _ => Weekday::Sun,
    }
}

/// Delay that lands just after the next second boundary, given `ms` past the current second.
fn next_tick_delay(ms: u32) -> u32 {
    1000 - ms.min(999) + TICK_MARGIN_MS
}

/// Run the application; returns the process exit code.
pub fn run() -> i32 {
    window::enable_dpi_awareness();
    let scale = window::dpi_scale();

    // Problems in the file (typos, read-only folder) fall back to defaults; the app will show
    // them once there is somewhere to show them.
    let mut file = ConfigFile::new(config::default_path());
    let config = file.load_or_create().config;

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
    match window::create(placement, Box::new(App::new(config, Some(file)))) {
        Ok(_) => window::run_message_loop(),
        Err(e) => {
            eprintln!("cannot create the window: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::config::{self, Rgba};
    use crate::schedule::{Class, DEFAULT_BREAK_LEN, MINUTE, hm};

    const MON: Weekday = Weekday::Mon;

    fn app_with(classes: Vec<Class>) -> App {
        let mut config = Config::default();
        config.schedule.set_day(MON, classes).unwrap();
        App::new(config, None)
    }

    fn class(start: Secs, end: Secs) -> Class {
        Class::new(start, end, DEFAULT_BREAK_LEN).unwrap()
    }

    fn is_clear(canvas: &Canvas) -> bool {
        canvas.pixels().iter().all(|&p| p == 0)
    }

    fn white_pixels(canvas: &Canvas) -> usize {
        let white = 0xffff_ffff;
        canvas.pixels().iter().filter(|&&p| p == white).count()
    }

    #[test]
    fn nothing_is_drawn_when_idle() {
        let mut app = app_with(vec![class(hm(9, 0), hm(9, 50))]);
        let mut canvas = Canvas::new(300, 40);
        assert!(!app.draw_at(&mut canvas, MON, hm(8, 0)));
        assert!(is_clear(&canvas));
        assert!(!app.draw_at(&mut canvas, Weekday::Sat, hm(9, 10)));
        assert!(!app.draw_at(&mut Canvas::new(300, 40), MON, hm(9, 50) + 10));
    }

    #[test]
    fn running_class_draws_the_bar_and_the_countdown() {
        let mut app = app_with(vec![class(hm(9, 0), hm(9, 50))]);
        let mut canvas = Canvas::new(500, 50);
        assert!(app.draw_at(&mut canvas, MON, hm(9, 0) + 25 * MINUTE));

        let green = config::Colors::default().green;
        let solid_green =
            0xff00_0000 | u32::from(green.r) << 16 | u32::from(green.g) << 8 | u32::from(green.b);
        assert_eq!(canvas.pixel(20, 25), solid_green, "left half is filled");
        assert_ne!(canvas.pixel(480, 25), solid_green, "right half is not");
        assert_eq!(canvas.pixel(0, 0) >> 24, 0, "rounded corner");
        assert!(white_pixels(&canvas) > 50, "the countdown text is drawn");
    }

    #[test]
    fn label_changes_with_the_time_and_is_drawn_in_the_text_colour() {
        let mut app = app_with(vec![class(hm(9, 0), hm(10, 40))]);
        app.config.colors.text = Rgba::rgb(255, 0, 255);
        let mut running = Canvas::new(500, 50);
        let mut brk = Canvas::new(500, 50);
        assert!(app.draw_at(&mut running, MON, hm(9, 10)));
        assert!(app.draw_at(&mut brk, MON, hm(9, 50)));
        let magenta = 0xffff_00ff;
        let count = |c: &Canvas| c.pixels().iter().filter(|&&p| p == magenta).count();
        assert!(count(&running) > 50 && count(&brk) > 50);
        assert_ne!(running.pixels(), brk.pixels());
        // "Break 05:00" is longer than "35:00", so it inks more of the bar.
        assert!(count(&brk) > count(&running));
    }

    #[test]
    fn lead_in_and_ended_are_shown() {
        let mut app = app_with(vec![class(hm(9, 0), hm(9, 50))]);
        let mut canvas = Canvas::new(500, 50);
        assert!(
            app.draw_at(&mut canvas, MON, hm(9, 0) - 3 * MINUTE),
            "lead-in"
        );
        assert!(white_pixels(&canvas) > 50);
        let mut canvas = Canvas::new(500, 50);
        assert!(app.draw_at(&mut canvas, MON, hm(9, 50) + 2), "just ended");
        let mut canvas = Canvas::new(500, 50);
        assert!(
            !app.draw_at(&mut canvas, MON, hm(9, 50) + 10),
            "gone after the linger time"
        );
    }

    #[test]
    fn text_scales_with_the_height_of_the_bar() {
        let mut app = app_with(vec![class(hm(9, 0), hm(9, 50))]);
        let (mut small, mut large) = (Canvas::new(400, 30), Canvas::new(800, 60));
        app.draw_at(&mut small, MON, hm(9, 10));
        app.draw_at(&mut large, MON, hm(9, 10));
        assert!(
            white_pixels(&large) > white_pixels(&small) * 2,
            "{} vs {}",
            white_pixels(&large),
            white_pixels(&small)
        );
    }

    #[test]
    fn colours_come_from_the_config() {
        let mut app = app_with(vec![class(hm(9, 0), hm(9, 50))]);
        app.config.colors.green = Rgba::rgb(10, 20, 30);
        let mut canvas = Canvas::new(500, 50);
        app.draw_at(&mut canvas, MON, hm(9, 25));
        assert_eq!(canvas.pixel(20, 25), 0xff0a_141e);
    }

    #[test]
    fn config_file_edits_apply_on_the_next_draw() {
        let dir: PathBuf =
            std::env::temp_dir().join(format!("times-up-app-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(config::FILE_NAME);
        fs::write(
            &path,
            "[monday]\nclasses = [{ start = \"00:00\", end = \"23:59\" }]\n",
        )
        .unwrap();

        let mut file = ConfigFile::new(&path);
        let config = file.load_or_create().config;
        let mut app = App::new(config, Some(file));
        // The class lasts all day, so whatever the time is the bar shows on Monday...
        assert_eq!(app.config.schedule.classes(MON).len(), 1);
        // ...until the file is edited: a new schedule and colour take effect via poll.
        fs::write(&path, "[colors]\ngreen = \"#010203\"\n[tuesday]\nclasses = [{ start = \"08:00\", end = \"08:50\" }]\n").unwrap();
        let mut canvas = Canvas::new(100, 20);
        let _ = Handler::draw(&mut app, &mut canvas); // polls the file, then draws "now"
        assert_eq!(app.config.colors.green, Rgba::rgb(1, 2, 3));
        assert!(app.config.schedule.classes(MON).is_empty());
        assert_eq!(app.config.schedule.classes(Weekday::Tue).len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn windows_weekdays_map_from_sunday() {
        let days = [
            (0, Weekday::Sun),
            (1, Weekday::Mon),
            (2, Weekday::Tue),
            (3, Weekday::Wed),
            (4, Weekday::Thu),
            (5, Weekday::Fri),
            (6, Weekday::Sat),
        ];
        for (n, day) in days {
            assert_eq!(weekday_from_windows(n), day);
        }
    }

    #[test]
    fn next_tick_lands_just_after_the_second_boundary() {
        assert_eq!(next_tick_delay(0), 1000 + TICK_MARGIN_MS);
        assert_eq!(next_tick_delay(400), 600 + TICK_MARGIN_MS);
        assert_eq!(next_tick_delay(999), 1 + TICK_MARGIN_MS);
        assert_eq!(
            next_tick_delay(5000),
            1 + TICK_MARGIN_MS,
            "never underflows"
        );
        let mut app = app_with(vec![]);
        app.last_ms = 250;
        assert_eq!(Handler::next_tick_ms(&app), 775);
    }

    #[test]
    fn local_time_is_plausible() {
        let now = local_now();
        assert!(now.secs < 24 * 3600 && now.ms < 1000);
    }
}
