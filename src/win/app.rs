//! Startup and the per-tick work: load the config, create the window, and each second decide
//! what the bar shows from the local time and the schedule.

use std::path::{Path, PathBuf};
use std::process::Command as Process;

use windows::Win32::System::SystemInformation::GetLocalTime;

use super::autostart::Autostart;
use super::menu::Command;
use super::monitors;
use super::text::render_mask;
use super::window::{self, DEFAULT_HEIGHT, DEFAULT_WIDTH, Handler, Placement};
use crate::bar;
use crate::canvas::Canvas;
use crate::config::{self, Config, ConfigFile, WindowRect};
use crate::interaction::{Rect, default_rect, fit_on_screen};
use crate::schedule::{Secs, Weekday};

/// Aim this many milliseconds past a second boundary so the clock has surely ticked over.
const TICK_MARGIN_MS: u32 = 25;
/// Label height as a share of the bar height.
const LABEL_SCALE: f32 = 0.5;

/// Opens a file for editing.
type Opener = Box<dyn Fn(&Path) -> std::io::Result<()>>;

/// What the menu commands act on, so tests can swap in fakes instead of touching the real
/// registry or starting Notepad.
struct Services {
    autostart: Autostart,
    /// Open the config file for editing.
    open_config: Opener,
}

impl Services {
    fn production() -> Self {
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("times-up.exe"));
        Self {
            autostart: Autostart::current_user(exe),
            open_config: Box::new(|path| Process::new("notepad.exe").arg(path).spawn().map(|_| ())),
        }
    }
}

struct App {
    config: Config,
    /// Where the config came from; polled each tick so edits to the file apply immediately.
    file: Option<ConfigFile>,
    /// Milliseconds past the second at the last tick, used to time the next one.
    last_ms: u32,
    services: Services,
}

impl App {
    fn new(config: Config, file: Option<ConfigFile>) -> Self {
        Self::with_services(config, file, Services::production())
    }

    fn with_services(config: Config, file: Option<ConfigFile>, services: Services) -> Self {
        Self {
            config,
            file,
            last_ms: 0,
            services,
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

    /// Remember where the teacher put the bar. A config that cannot be written (read-only
    /// folder) just means the position is not kept for the next start.
    fn window_changed(&mut self, rect: Rect) {
        self.config.window = WindowRect {
            x: Some(rect.x),
            y: Some(rect.y),
            width: u32::try_from(rect.w).ok(),
            height: u32::try_from(rect.h).ok(),
        };
        if let Some(file) = &mut self.file {
            let _ = file.save_window(&self.config.window);
        }
    }

    fn autostart_enabled(&self) -> bool {
        self.services.autostart.is_enabled()
    }

    /// Open the config in Notepad, or turn "Start with Windows" on or off. A failure comes back
    /// as text for the user and leaves things as they were.
    fn run_command(&mut self, command: Command) -> Option<String> {
        match command {
            Command::OpenConfig => {
                let path = self
                    .file
                    .as_ref()
                    .map_or_else(config::default_path, |f| f.path().to_path_buf());
                (self.services.open_config)(&path)
                    .err()
                    .map(|e| format!("Could not open {}: {e}", path.display()))
            }
            Command::ToggleAutostart => self
                .services
                .autostart
                .toggle()
                .err()
                .map(|e| format!("Could not change \"Start with Windows\": {e}")),
            // The window itself handles Quit.
            Command::Quit => None,
        }
    }
}

/// Where the window starts: the saved rectangle moved onto a connected monitor if need be, or the
/// default spot (top centre of the primary monitor) when nothing is saved. Sizes that were
/// never saved use the defaults for the display's `scale`, and nothing goes below `min`.
fn startup_rect(
    saved: &WindowRect,
    scale: f32,
    areas: &[Rect],
    min: (i32, i32),
    margin: i32,
) -> Rect {
    let width = saved
        .width
        .map_or((DEFAULT_WIDTH as f32 * scale) as i32, |w| w as i32)
        .max(min.0);
    let height = saved
        .height
        .map_or((DEFAULT_HEIGHT as f32 * scale) as i32, |h| h as i32)
        .max(min.1);
    match (saved.x, saved.y, areas.first()) {
        (Some(x), Some(y), _) => fit_on_screen(Rect::new(x, y, width, height), areas, margin),
        (_, _, Some(primary)) => default_rect(width, height, *primary, margin),
        _ => Rect::new(0, margin, width, height),
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

    let rect = startup_rect(
        &config.window,
        scale,
        &monitors::work_areas(),
        window::min_size(),
        window::top_margin(),
    );
    let placement = Placement {
        x: Some(rect.x),
        y: Some(rect.y),
        width: rect.w as u32,
        height: rect.h as u32,
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

    // ---- window placement ----

    const PRIMARY: Rect = Rect::new(0, 0, 1920, 1040);
    const SECOND: Rect = Rect::new(1920, 0, 1280, 1024);
    const MIN: (i32, i32) = (160, 24);

    fn saved(x: Option<i32>, y: Option<i32>, w: Option<u32>, h: Option<u32>) -> WindowRect {
        WindowRect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    #[test]
    fn nothing_saved_starts_top_centre_of_the_primary_monitor_at_the_default_size() {
        let rect = startup_rect(&WindowRect::default(), 1.0, &[PRIMARY, SECOND], MIN, 20);
        assert_eq!(rect, Rect::new(660, 20, 600, 60));
        // The default size follows the display scale.
        let rect = startup_rect(&WindowRect::default(), 1.5, &[PRIMARY], MIN, 30);
        assert_eq!((rect.w, rect.h), (900, 90));
        assert_eq!(rect.x, (1920 - 900) / 2);
        assert_eq!(rect.y, 30);
    }

    #[test]
    fn a_saved_rectangle_is_restored_as_it_was() {
        let s = saved(Some(2000), Some(300), Some(700), Some(80));
        assert_eq!(
            startup_rect(&s, 1.0, &[PRIMARY, SECOND], MIN, 20),
            Rect::new(2000, 300, 700, 80)
        );
    }

    #[test]
    fn a_saved_size_without_a_position_keeps_the_size_and_uses_the_default_spot() {
        let s = saved(None, None, Some(800), Some(50));
        assert_eq!(
            startup_rect(&s, 1.0, &[PRIMARY], MIN, 20),
            Rect::new(560, 20, 800, 50)
        );
        // A position needs both coordinates.
        let s = saved(Some(100), None, Some(800), Some(50));
        assert_eq!(startup_rect(&s, 1.0, &[PRIMARY], MIN, 20).y, 20);
    }

    #[test]
    fn a_missing_monitor_brings_the_window_back_to_the_primary_one() {
        // Saved on the second monitor, which is not connected now.
        let s = saved(Some(2400), Some(300), Some(600), Some(60));
        assert_eq!(
            startup_rect(&s, 1.0, &[PRIMARY], MIN, 20),
            Rect::new(660, 20, 600, 60)
        );
        // Barely visible at the edge: pulled fully onto the screen.
        let s = saved(Some(1850), Some(300), Some(600), Some(60));
        assert_eq!(
            startup_rect(&s, 1.0, &[PRIMARY], MIN, 20),
            Rect::new(1320, 300, 600, 60)
        );
    }

    #[test]
    fn a_saved_size_below_the_minimum_is_raised() {
        let s = saved(Some(100), Some(100), Some(10), Some(5));
        let rect = startup_rect(&s, 1.0, &[PRIMARY], MIN, 20);
        assert_eq!((rect.w, rect.h), (160, 24));
    }

    #[test]
    fn without_monitor_information_the_saved_position_is_used() {
        let s = saved(Some(50), Some(60), Some(300), Some(40));
        assert_eq!(
            startup_rect(&s, 1.0, &[], MIN, 20),
            Rect::new(50, 60, 300, 40)
        );
        assert_eq!(
            startup_rect(&WindowRect::default(), 1.0, &[], MIN, 20),
            Rect::new(0, 20, 600, 60)
        );
    }

    // ---- saving the window rectangle ----

    #[test]
    fn moving_or_resizing_the_window_is_saved_in_the_config_file() {
        let dir: PathBuf =
            std::env::temp_dir().join(format!("times-up-app-save-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(config::FILE_NAME);
        let mut file = ConfigFile::new(&path);
        let config = file.load_or_create().config;
        let mut app = App::new(config, Some(file));

        app.window_changed(Rect::new(500, 40, 640, 72));
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("# Times Up settings."), "comments are kept");
        let reloaded = config::parse(&text).config;
        assert_eq!(
            reloaded.window,
            saved(Some(500), Some(40), Some(640), Some(72))
        );
        assert_eq!(app.config.window, reloaded.window);

        // Our own save is not mistaken for an outside edit, so it does not reload mid-drag.
        let mut canvas = Canvas::new(100, 20);
        let _ = Handler::draw(&mut app, &mut canvas);
        assert_eq!(app.config.window, reloaded.window);

        // The next start comes back at the same place.
        assert_eq!(
            startup_rect(&reloaded.window, 1.0, &[PRIMARY, SECOND], MIN, 20),
            Rect::new(500, 40, 640, 72)
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unwritable_config_does_not_stop_the_window_from_moving() {
        // No file at all: the change is kept in memory only.
        let mut app = app_with(vec![]);
        app.window_changed(Rect::new(10, 20, 300, 40));
        assert_eq!(
            app.config.window,
            saved(Some(10), Some(20), Some(300), Some(40))
        );

        // A file that cannot be written (read-only): saving fails quietly.
        let dir: PathBuf =
            std::env::temp_dir().join(format!("times-up-app-readonly-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(config::FILE_NAME);
        fs::write(&path, config::DEFAULT_CONFIG).unwrap();
        let mut perm = fs::metadata(&path).unwrap().permissions();
        perm.set_readonly(true);
        fs::set_permissions(&path, perm).unwrap();
        let mut file = ConfigFile::new(&path);
        let config = file.load_or_create().config;
        let mut app = App::new(config, Some(file));
        app.window_changed(Rect::new(1, 2, 300, 40));
        assert_eq!(fs::read_to_string(&path).unwrap(), config::DEFAULT_CONFIG);
        assert_eq!(
            app.config.window.x,
            Some(1),
            "still remembered while running"
        );

        let mut perm = fs::metadata(&path).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perm.set_readonly(false);
        let _ = fs::set_permissions(&path, perm);
        let _ = fs::remove_dir_all(&dir);
    }

    // ---- menu commands ----

    use std::cell::RefCell;
    use std::rc::Rc;

    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RegDeleteKeyW};
    use windows::core::PCWSTR;

    /// Services with a throwaway registry key and an opener that records instead of starting
    /// Notepad. The key is deleted when this is dropped.
    struct Fake {
        subkey: String,
        opened: Rc<RefCell<Vec<PathBuf>>>,
    }

    impl Fake {
        fn new(label: &str) -> Self {
            Self {
                subkey: format!(r"Software\TimesUpTest\{}-app-{label}", std::process::id()),
                opened: Rc::new(RefCell::new(Vec::new())),
            }
        }

        fn services(&self, open_fails: bool) -> Services {
            let opened = self.opened.clone();
            Services {
                autostart: Autostart::with_key(
                    HKEY_CURRENT_USER,
                    &self.subkey,
                    "TimesUp",
                    r"C:\Tools\times-up.exe",
                ),
                open_config: Box::new(move |path| {
                    opened.borrow_mut().push(path.to_path_buf());
                    if open_fails {
                        Err(std::io::Error::other("notepad is missing"))
                    } else {
                        Ok(())
                    }
                }),
            }
        }
    }

    impl Drop for Fake {
        fn drop(&mut self) {
            let wide = |s: &str| {
                s.encode_utf16()
                    .chain(std::iter::once(0))
                    .collect::<Vec<u16>>()
            };
            let (sub, parent) = (wide(&self.subkey), wide(r"Software\TimesUpTest"));
            // SAFETY: deleting keys this test created.
            unsafe {
                let _ = RegDeleteKeyW(HKEY_CURRENT_USER, PCWSTR(sub.as_ptr()));
                let _ = RegDeleteKeyW(HKEY_CURRENT_USER, PCWSTR(parent.as_ptr()));
            }
        }
    }

    #[test]
    fn open_config_opens_the_config_file() {
        let fake = Fake::new("open");
        let dir: PathBuf =
            std::env::temp_dir().join(format!("times-up-app-open-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let mut file = ConfigFile::new(dir.join(config::FILE_NAME));
        let config = file.load_or_create().config;
        let mut app = App::with_services(config, Some(file), fake.services(false));

        assert_eq!(app.run_command(Command::OpenConfig), None);
        assert_eq!(*fake.opened.borrow(), vec![dir.join(config::FILE_NAME)]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn open_config_failure_is_reported_with_the_path_and_the_reason() {
        let fake = Fake::new("open-fails");
        let mut app = App::with_services(Config::default(), None, fake.services(true));
        let message = app.run_command(Command::OpenConfig).expect("a message");
        assert!(message.contains("Could not open"), "{message}");
        assert!(message.contains(config::FILE_NAME), "{message}");
        assert!(message.contains("notepad is missing"), "{message}");
        // Without a loaded file the standard place next to the exe is opened.
        assert_eq!(*fake.opened.borrow(), vec![config::default_path()]);
    }

    #[test]
    fn start_with_windows_toggles_the_run_entry() {
        let fake = Fake::new("toggle");
        let mut app = App::with_services(Config::default(), None, fake.services(false));
        assert!(!app.autostart_enabled());
        assert_eq!(app.run_command(Command::ToggleAutostart), None);
        assert!(
            app.autostart_enabled(),
            "the menu would now show a check mark"
        );
        assert_eq!(app.run_command(Command::ToggleAutostart), None);
        assert!(!app.autostart_enabled());
    }

    #[test]
    fn a_failing_start_with_windows_toggle_is_reported_and_changes_nothing() {
        let long = "x".repeat(300);
        let services = Services {
            autostart: Autostart::with_key(
                HKEY_CURRENT_USER,
                &format!(r"Software\TimesUpTest\{long}"),
                "TimesUp",
                r"C:\Tools\times-up.exe",
            ),
            open_config: Box::new(|_| Ok(())),
        };
        let mut app = App::with_services(Config::default(), None, services);
        let message = app
            .run_command(Command::ToggleAutostart)
            .expect("a message");
        assert!(message.contains("Start with Windows"), "{message}");
        assert!(!app.autostart_enabled(), "still off");
    }

    #[test]
    fn quit_is_left_to_the_window() {
        let fake = Fake::new("quit");
        let mut app = App::with_services(Config::default(), None, fake.services(false));
        assert_eq!(app.run_command(Command::Quit), None);
        assert!(fake.opened.borrow().is_empty());
        assert!(!app.autostart_enabled());
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
