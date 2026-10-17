//! Startup and the per-tick work: load the config, create the window, and each second decide
//! what the bar shows from the local time and the schedule.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use windows::Win32::System::SystemInformation::GetLocalTime;

use super::autostart::Autostart;
use super::menu::{Command, MenuState};
use super::monitors;
use super::settings_window;
use super::text::render_mask;
use super::window::{self, DEFAULT_HEIGHT, DEFAULT_WIDTH, Handler, Placement};
use crate::bar;
use crate::canvas::Canvas;
use crate::config::{self, Config, ConfigError, ConfigFile, WindowRect};
use crate::editing::{self, Handle};
use crate::interaction::{CursorShape, Rect, default_rect, fit_on_screen};
use crate::schedule::{Class, Secs, State, Weekday};
use crate::store::Store;

/// Aim this many milliseconds past a second boundary so the clock has surely ticked over.
const TICK_MARGIN_MS: u32 = 25;
/// Label height as a share of the bar height.
const LABEL_SCALE: f32 = 0.5;
/// How close (as a share of the bar height, at least 4 px) the pointer must be to a break edge or
/// zone boundary to pick it up.
const HANDLE_GRIP_SCALE: f32 = 0.15;
/// The time shown while a handle is dragged: its font and the strip at the bottom of the bar it
/// is drawn in, as shares of the bar height.
const DRAG_LABEL_SCALE: f32 = 0.22;
const DRAG_STRIP_SCALE: f32 = 0.32;
/// Keep the drag label this far (as a share of the bar width) from either end.
const DRAG_LABEL_MARGIN: f32 = 0.08;

/// Opens the Settings window on the store, or brings the open one to the front.
type SettingsOpener = Box<dyn Fn(&Rc<RefCell<Store>>) -> Result<(), String>>;

/// What the menu commands act on, so tests can swap in fakes instead of touching the real
/// registry or opening a window.
struct Services {
    autostart: Autostart,
    open_settings: SettingsOpener,
}

impl Services {
    fn production() -> Self {
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("times-up.exe"));
        Self {
            autostart: Autostart::current_user(exe),
            open_settings: Box::new(|store| {
                settings_window::open(store.clone())
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }),
        }
    }
}

/// A class the teacher changed on the bar, with the version to go back to. The change lives only
/// in memory: it is undone when another class (or none) becomes current, and it is lost on
/// restart, unless it is saved as the default.
struct Edited {
    day: Weekday,
    index: usize,
    original: Class,
}

/// A handle being dragged.
struct Grab {
    handle: Handle,
    /// The class as it was when the drag started; every move is computed from this.
    start: Class,
    day: Weekday,
    index: usize,
    /// Width of the bar in pixels, to turn pixels into time.
    width: i32,
}

struct App {
    /// The config and the file it lives in; polled each tick so edits to the file apply immediately.
    store: Rc<RefCell<Store>>,
    /// Milliseconds past the second at the last tick, used to time the next one.
    last_ms: u32,
    services: Services,
    edited: Option<Edited>,
    grab: Option<Grab>,
}

impl App {
    fn new(config: Config, file: Option<ConfigFile>) -> Self {
        Self::with_services(config, file, Services::production())
    }

    fn with_services(config: Config, file: Option<ConfigFile>, services: Services) -> Self {
        Self {
            store: Rc::new(RefCell::new(Store::new(config, file))),
            last_ms: 0,
            services,
            edited: None,
            grab: None,
        }
    }

    /// The class the bar is showing at `now` (counting from the lead-in until it has just ended).
    fn current_class(&self, day: Weekday, now: Secs) -> Option<usize> {
        match self.store.borrow().schedule().state(day, now) {
            State::LeadIn { class, .. }
            | State::Running { class, .. }
            | State::Break { class, .. }
            | State::Ended { class } => Some(class),
            State::Idle { .. } => None,
        }
    }

    /// Undo the teacher's changes to a class once it is no longer the current one.
    fn revert_stale(&mut self, day: Weekday, now: Secs) {
        let stale = self
            .edited
            .as_ref()
            .is_some_and(|e| e.day != day || self.current_class(day, now) != Some(e.index));
        if stale {
            self.undo_edit();
            self.grab = None;
        }
    }

    /// Put the edited class back as it was.
    fn undo_edit(&mut self) {
        if let Some(edited) = self.edited.take()
            && let Some(class) = self
                .store
                .borrow_mut()
                .schedule_mut()
                .class_mut(edited.day, edited.index)
        {
            *class = edited.original;
        }
    }

    /// Store the break and thresholds of the edited class in the config file as its defaults.
    /// Returns an error text when that is not possible; the edit stays as it was then.
    fn save_default(&mut self) -> Option<String> {
        let edited = self.edited.as_ref()?;
        let (day, index) = (edited.day, edited.index);
        let class = self
            .store
            .borrow()
            .schedule()
            .classes(day)
            .get(index)?
            .clone();
        match self.store.borrow_mut().save_class(day, &class) {
            Err(ConfigError::NoFile) => Some("There is no settings file to save to.".to_string()),
            Ok(()) => {
                // What is on the bar is now what the file says; nothing left to undo.
                self.edited = None;
                None
            }
            Err(e) => Some(format!("Could not save the defaults: {e}")),
        }
    }

    fn handle_grip(height: i32) -> f32 {
        (height as f32 * HANDLE_GRIP_SCALE).max(4.0)
    }

    /// The handle of the current class under the pointer at `x` in a bar `w` x `h`.
    fn handle_under(
        &self,
        day: Weekday,
        now: Secs,
        x: i32,
        w: i32,
        h: i32,
    ) -> Option<(usize, Handle)> {
        let index = self.current_class(day, now)?;
        let store = self.store.borrow();
        let class = store.schedule().classes(day).get(index)?;
        let handle = editing::handle_at(class, x as f32, w as f32, Self::handle_grip(h))?;
        Some((index, handle))
    }

    /// Pick up a handle of the current class, if there is one at `x`.
    fn grab_at(&mut self, day: Weekday, now: Secs, x: i32, w: i32, h: i32) -> bool {
        let Some((index, handle)) = self.handle_under(day, now, x, w, h) else {
            return false;
        };
        let start = self.store.borrow().schedule().classes(day)[index].clone();
        self.grab = Some(Grab {
            handle,
            start,
            day,
            index,
            width: w,
        });
        true
    }

    /// The handle follows the pointer, `dx` pixels from where it was picked up. The class is
    /// changed in the schedule right away, so the bar shows it on the next paint.
    fn move_grab(&mut self, dx: i32) {
        let Some(grab) = &self.grab else {
            return;
        };
        let width = f64::from(grab.width.max(1));
        let seconds = (f64::from(dx) / width * f64::from(grab.start.duration())).round() as i64;
        let changed = editing::drag(&grab.start, grab.handle, seconds);
        let (day, index, start) = (grab.day, grab.index, grab.start.clone());

        // An edit of some other class cannot be kept alongside this one.
        if self
            .edited
            .as_ref()
            .is_some_and(|e| e.day != day || e.index != index)
        {
            self.undo_edit();
        }
        // Remember the first version of the class, however many drags it takes.
        if self.edited.is_none() && changed != start {
            self.edited = Some(Edited {
                day,
                index,
                original: start,
            });
        }
        let mut store = self.store.borrow_mut();
        let Some(class) = store.schedule_mut().class_mut(day, index) else {
            return;
        };
        *class = changed;
        // Dragged back to where it began: nothing is left to undo or save.
        if self.edited.as_ref().is_some_and(|e| e.original == *class) {
            self.edited = None;
        }
    }

    /// Paint the bar for `day` at `now` seconds after midnight. `false` means nothing to show.
    fn draw_at(&mut self, canvas: &mut Canvas, day: Weekday, now: Secs) -> bool {
        self.revert_stale(day, now);
        let Some(view) = bar::view(self.store.borrow().schedule(), day, now) else {
            return false;
        };
        bar::paint(canvas, &view, &self.store.borrow().config().colors);
        let (w, h) = (canvas.width(), canvas.height());
        let font_px = (h as f32 * LABEL_SCALE) as i32;
        if let Some(mask) = render_mask(&view.label, w, h, font_px) {
            // The mask has the text in the middle of the whole bar; shift it to the middle of the
            // block that the countdown is about.
            let shift = (view.label_center * w as f32 - w as f32 / 2.0).round() as i32;
            canvas.blend_mask(
                &mask,
                w,
                h,
                shift,
                0,
                self.store.borrow().config().colors.text,
            );
        }
        self.draw_drag_label(canvas, day);
        true
    }

    /// While a handle is dragged: its time, small, at the bottom of the bar under the handle.
    fn draw_drag_label(&self, canvas: &mut Canvas, day: Weekday) {
        let Some(grab) = self.grab.as_ref().filter(|g| g.day == day) else {
            return;
        };
        let store = self.store.borrow();
        let Some(class) = store.schedule().classes(day).get(grab.index) else {
            return;
        };
        let Some(label) = editing::describe(class, grab.handle) else {
            return;
        };
        let (w, h) = (canvas.width(), canvas.height());
        let strip = ((h as f32 * DRAG_STRIP_SCALE) as u32).max(1);
        let font_px = ((h as f32 * DRAG_LABEL_SCALE) as i32).max(9);
        if let Some(mask) = render_mask(&label.text, w, strip, font_px) {
            let center = label
                .position
                .clamp(DRAG_LABEL_MARGIN, 1.0 - DRAG_LABEL_MARGIN);
            let shift = (center * w as f32 - w as f32 / 2.0).round() as i32;
            canvas.blend_mask(
                &mask,
                w,
                strip,
                shift,
                (h - strip) as i32,
                self.store.borrow().config().colors.text,
            );
        }
    }
}

impl Handler for App {
    fn draw(&mut self, canvas: &mut Canvas) -> bool {
        if self.store.borrow_mut().poll() {
            // The file is the truth: changes made on the bar and not saved are dropped.
            self.edited = None;
            self.grab = None;
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
        let _ = self.store.borrow_mut().save_window(WindowRect {
            x: Some(rect.x),
            y: Some(rect.y),
            width: u32::try_from(rect.w).ok(),
            height: u32::try_from(rect.h).ok(),
        });
    }

    fn menu_state(&self) -> MenuState {
        MenuState {
            autostart: self.services.autostart.is_enabled(),
            can_save_default: self.edited.is_some(),
        }
    }

    fn grab(&mut self, x: i32, _y: i32, w: i32, h: i32) -> bool {
        let now = local_now();
        self.grab_at(now.day, now.secs, x, w, h)
    }

    fn grab_move(&mut self, dx: i32) {
        self.move_grab(dx);
    }

    fn grab_end(&mut self) {
        self.grab = None;
    }

    fn hover(&self, x: i32, _y: i32, w: i32, h: i32) -> Option<CursorShape> {
        let now = local_now();
        self.handle_under(now.day, now.secs, x, w, h)
            .map(|_| CursorShape::SizeWestEast)
    }

    /// Save the class defaults, open the Settings window, or turn "Start with Windows" on or
    /// off. A failure comes back as text for the user and leaves things as they were.
    fn run_command(&mut self, command: Command) -> Option<String> {
        match command {
            Command::SaveDefault => self.save_default(),
            Command::Settings => (self.services.open_settings)(&self.store)
                .err()
                .map(|e| format!("Could not open the settings window: {e}")),
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
    match window::create_with_tray(placement, Box::new(App::new(config, Some(file)))) {
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
        app.store.borrow_mut().config_mut().colors.text = Rgba::rgb(255, 0, 255);
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
        app.store.borrow_mut().config_mut().colors.green = Rgba::rgb(10, 20, 30);
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
        assert_eq!(app.store.borrow().schedule().classes(MON).len(), 1);
        // ...until the file is edited: a new schedule and colour take effect via poll.
        fs::write(&path, "[colors]\ngreen = \"#010203\"\n[tuesday]\nclasses = [{ start = \"08:00\", end = \"08:50\" }]\n").unwrap();
        let mut canvas = Canvas::new(100, 20);
        let _ = Handler::draw(&mut app, &mut canvas); // polls the file, then draws "now"
        assert_eq!(app.store.borrow().config().colors.green, Rgba::rgb(1, 2, 3));
        assert!(app.store.borrow().schedule().classes(MON).is_empty());
        assert_eq!(app.store.borrow().schedule().classes(Weekday::Tue).len(), 1);
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
        assert!(text.contains("# Time's up settings."), "comments are kept");
        let reloaded = config::parse(&text).config;
        assert_eq!(
            reloaded.window,
            saved(Some(500), Some(40), Some(640), Some(72))
        );
        assert_eq!(app.store.borrow().config().window, reloaded.window);

        // Our own save is not mistaken for an outside edit, so it does not reload mid-drag.
        let mut canvas = Canvas::new(100, 20);
        let _ = Handler::draw(&mut app, &mut canvas);
        assert_eq!(app.store.borrow().config().window, reloaded.window);

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
            app.store.borrow().config().window,
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
            app.store.borrow().config().window.x,
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

    /// Services with a throwaway registry key and an opener that records instead of opening a
    /// window. The key is deleted when this is dropped.
    struct Fake {
        subkey: String,
        /// The stores the Settings window was asked to open on.
        opened: Rc<RefCell<Vec<Rc<RefCell<Store>>>>>,
    }

    impl Fake {
        fn new(label: &str) -> Self {
            Self {
                subkey: format!(r"Software\TimesUpTest-{}-app-{label}", std::process::id()),
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
                open_settings: Box::new(move |store| {
                    opened.borrow_mut().push(store.clone());
                    if open_fails {
                        Err("no window for you".to_string())
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
            let sub = wide(&self.subkey);
            // SAFETY: deleting the key this test created.
            unsafe {
                let _ = RegDeleteKeyW(HKEY_CURRENT_USER, PCWSTR(sub.as_ptr()));
            }
        }
    }

    #[test]
    fn settings_opens_the_settings_window_on_the_apps_store() {
        let fake = Fake::new("open");
        let mut app = App::with_services(Config::default(), None, fake.services(false));

        assert_eq!(app.run_command(Command::Settings), None);
        assert_eq!(app.run_command(Command::Settings), None);
        let opened = fake.opened.borrow();
        assert_eq!(opened.len(), 2);
        assert!(opened.iter().all(|s| Rc::ptr_eq(s, &app.store)));
    }

    #[test]
    fn settings_failure_is_reported_with_the_reason() {
        let fake = Fake::new("open-fails");
        let mut app = App::with_services(Config::default(), None, fake.services(true));
        let message = app.run_command(Command::Settings).expect("a message");
        assert!(message.contains("settings window"), "{message}");
        assert!(message.contains("no window for you"), "{message}");
    }

    #[test]
    fn start_with_windows_toggles_the_run_entry() {
        let fake = Fake::new("toggle");
        let mut app = App::with_services(Config::default(), None, fake.services(false));
        assert!(!app.menu_state().autostart);
        assert_eq!(app.run_command(Command::ToggleAutostart), None);
        assert!(
            app.menu_state().autostart,
            "the menu would now show a check mark"
        );
        assert_eq!(app.run_command(Command::ToggleAutostart), None);
        assert!(!app.menu_state().autostart);
    }

    #[test]
    fn a_failing_start_with_windows_toggle_is_reported_and_changes_nothing() {
        let long = "x".repeat(300);
        let services = Services {
            autostart: Autostart::with_key(
                HKEY_CURRENT_USER,
                &format!(r"Software\{long}"),
                "TimesUp",
                r"C:\Tools\times-up.exe",
            ),
            open_settings: Box::new(|_| Ok(())),
        };
        let mut app = App::with_services(Config::default(), None, services);
        let message = app
            .run_command(Command::ToggleAutostart)
            .expect("a message");
        assert!(message.contains("Start with Windows"), "{message}");
        assert!(!app.menu_state().autostart, "still off");
    }

    #[test]
    fn quit_is_left_to_the_window() {
        let fake = Fake::new("quit");
        let mut app = App::with_services(Config::default(), None, fake.services(false));
        assert_eq!(app.run_command(Command::Quit), None);
        assert!(fake.opened.borrow().is_empty());
        assert!(!app.menu_state().autostart);
    }

    // ---- editing the break and the zones on the bar ----

    use crate::schedule::Break;

    /// Two double classes on Monday: 09:00-10:40 and 11:00-12:40, both with the default break.
    fn two_double_classes() -> App {
        app_with(vec![
            class(hm(9, 0), hm(10, 40)),
            class(hm(11, 0), hm(12, 40)),
        ])
    }

    /// 09:30, in the middle of the first block of the first class.
    const NOW: Secs = hm(9, 30);
    /// A bar 1000 px wide: 10 px per minute of the 100 minute classes.
    const W: i32 = 1000;
    const H: i32 = 40;

    fn brk_of(app: &App, index: usize) -> Break {
        app.store.borrow().schedule().classes(MON)[index]
            .brk
            .unwrap()
    }

    fn minutes(b: Break) -> (u32, u32) {
        (b.start / MINUTE, b.end() / MINUTE)
    }

    #[test]
    fn dragging_the_break_changes_the_current_class_only() {
        let mut app = two_double_classes();
        assert!(
            app.grab_at(MON, NOW, 500, W, H),
            "the middle of the break is its body"
        );
        app.move_grab(100); // 100 px = 10 minutes later
        assert_eq!(minutes(brk_of(&app, 0)), (55, 65));
        assert_eq!(
            minutes(brk_of(&app, 1)),
            (45, 55),
            "the other class is untouched"
        );
        assert!(app.menu_state().can_save_default);
        app.grab_end();
        assert!(app.grab.is_none());
    }

    #[test]
    fn the_break_edges_resize_it() {
        let mut app = two_double_classes();
        assert!(app.grab_at(MON, NOW, 450, W, H)); // its start edge
        app.move_grab(-50);
        assert_eq!(minutes(brk_of(&app, 0)), (40, 55));
        app.grab_end();
        assert!(app.grab_at(MON, NOW, 550, W, H)); // its end edge
        app.move_grab(30);
        assert_eq!(minutes(brk_of(&app, 0)), (40, 58));
    }

    #[test]
    fn zone_boundaries_can_be_dragged() {
        let mut app = two_double_classes();
        // Orange of block 1 begins 10 minutes before its end (at 35 minutes = 350 px).
        assert!(app.grab_at(MON, NOW, 350, W, H));
        app.move_grab(20); // 2 minutes later: 8 minutes of orange left
        let w = app.store.borrow().schedule().classes(MON)[0].warnings[0];
        assert_eq!((w.orange / MINUTE, w.red / MINUTE), (8, 5));
        // And the red boundary of the second block (at 95 minutes = 950 px).
        app.grab_end();
        assert!(app.grab_at(MON, NOW, 950, W, H));
        app.move_grab(-20);
        let w = app.store.borrow().schedule().classes(MON)[0].warnings[1];
        assert_eq!((w.orange / MINUTE, w.red / MINUTE), (10, 7));
    }

    #[test]
    fn dragging_is_clamped() {
        let mut app = two_double_classes();
        assert!(app.grab_at(MON, NOW, 500, W, H));
        app.move_grab(-100_000);
        assert_eq!(minutes(brk_of(&app, 0)), (5, 15), "blocks keep 5 minutes");
        app.grab_end();
        assert!(app.grab_at(MON, NOW, 100, W, H)); // the break is at 5..15 now: its body
        app.move_grab(100_000);
        assert_eq!(minutes(brk_of(&app, 0)), (85, 95));
    }

    #[test]
    fn every_move_is_computed_from_where_the_drag_started() {
        let mut app = two_double_classes();
        assert!(app.grab_at(MON, NOW, 500, W, H));
        app.move_grab(100);
        app.move_grab(30);
        assert_eq!(minutes(brk_of(&app, 0)), (48, 58), "not 45 + 10 + 3");
        app.move_grab(-20);
        assert_eq!(minutes(brk_of(&app, 0)), (43, 53));
    }

    #[test]
    fn dragging_back_to_the_start_leaves_nothing_to_save() {
        let mut app = two_double_classes();
        assert!(app.grab_at(MON, NOW, 500, W, H));
        app.move_grab(100);
        assert!(app.menu_state().can_save_default);
        app.move_grab(0);
        assert_eq!(minutes(brk_of(&app, 0)), (45, 55));
        assert!(!app.menu_state().can_save_default, "no net change");
        // A movement too small to change anything does not count as an edit either.
        app.move_grab(3);
        assert!(!app.menu_state().can_save_default);
    }

    #[test]
    fn the_first_version_is_kept_across_several_drags() {
        let mut app = two_double_classes();
        assert!(app.grab_at(MON, NOW, 500, W, H));
        app.move_grab(100);
        app.grab_end();
        assert!(app.grab_at(MON, NOW, 600, W, H)); // the break is now at 55..65
        app.move_grab(-50);
        app.grab_end();
        assert_eq!(minutes(brk_of(&app, 0)), (50, 60));
        // Undoing goes back to the original, not to the state before the last drag.
        app.undo_edit();
        assert_eq!(minutes(brk_of(&app, 0)), (45, 55));
    }

    #[test]
    fn nothing_is_grabbed_where_there_is_no_handle() {
        let mut app = two_double_classes();
        assert!(
            !app.grab_at(MON, NOW, 100, W, H),
            "empty part of the first block"
        );
        assert!(
            !app.grab_at(MON, NOW, 700, W, H),
            "empty part of the second block"
        );
        assert!(app.grab.is_none());
        // Without a current class there is nothing to edit.
        assert!(!app.grab_at(MON, hm(8, 0), 500, W, H));
        assert!(!app.grab_at(Weekday::Sat, NOW, 500, W, H));
        // A class without a break only has zone handles.
        let mut single = app_with(vec![class(hm(9, 0), hm(9, 50))]);
        assert!(!single.grab_at(MON, NOW, 250, 500, H));
        assert!(single.grab_at(MON, NOW, 400, 500, H));
        // Moving without a grab does nothing.
        app.move_grab(50);
        assert!(!app.menu_state().can_save_default);
    }

    #[test]
    fn hover_finds_the_same_handles() {
        let app = two_double_classes();
        assert!(app.handle_under(MON, NOW, 500, W, H).is_some());
        assert!(app.handle_under(MON, NOW, 350, W, H).is_some());
        assert!(app.handle_under(MON, NOW, 100, W, H).is_none());
        assert!(app.handle_under(MON, hm(8, 0), 500, W, H).is_none());
    }

    #[test]
    fn handles_can_be_picked_up_during_the_lead_in_and_after_the_end() {
        let mut app = two_double_classes();
        assert!(app.grab_at(MON, hm(9, 0) - 120, 500, W, H), "lead-in");
        app.grab_end();
        assert!(app.grab_at(MON, hm(10, 40) + 3, 500, W, H), "just ended");
    }

    // ---- changes are undone when the class is over ----

    fn edited_first_class() -> App {
        let mut app = two_double_classes();
        assert!(app.grab_at(MON, NOW, 500, W, H));
        app.move_grab(100);
        app.grab_end();
        assert_eq!(minutes(brk_of(&app, 0)), (55, 65));
        app
    }

    #[test]
    fn the_change_stays_while_the_class_is_current_and_is_drawn() {
        let mut app = edited_first_class();
        let mut canvas = Canvas::new(1000, 40);
        // 9:50 is 50 minutes in: inside the moved break (55..65)? no, still before it.
        assert!(app.draw_at(&mut canvas, MON, hm(9, 50)));
        assert_eq!(minutes(brk_of(&app, 0)), (55, 65));
        // 10:00 is 60 minutes in: now inside the moved break.
        let mut canvas = Canvas::new(1000, 40);
        assert!(app.draw_at(&mut canvas, MON, hm(10, 0)));
        let state = app.store.borrow().schedule().state(MON, hm(10, 0));
        assert!(matches!(state, State::Break { .. }), "{state:?}");
        assert!(app.menu_state().can_save_default);
    }

    #[test]
    fn the_change_is_undone_once_the_class_is_over() {
        let mut app = edited_first_class();
        let mut canvas = Canvas::new(1000, 40);
        // Just after the end the class is still shown, so the change is still there.
        assert!(app.draw_at(&mut canvas, MON, hm(10, 40) + 5));
        assert_eq!(minutes(brk_of(&app, 0)), (55, 65));
        // Once the bar is hidden between classes the default is back.
        assert!(!app.draw_at(&mut canvas, MON, hm(10, 50)));
        assert_eq!(minutes(brk_of(&app, 0)), (45, 55));
        assert!(!app.menu_state().can_save_default);
    }

    #[test]
    fn the_change_is_undone_when_the_next_class_starts() {
        let mut app = edited_first_class();
        let mut canvas = Canvas::new(1000, 40);
        assert!(app.draw_at(&mut canvas, MON, hm(11, 10)));
        assert_eq!(minutes(brk_of(&app, 0)), (45, 55));
        assert_eq!(minutes(brk_of(&app, 1)), (45, 55));
        assert!(!app.menu_state().can_save_default);
    }

    #[test]
    fn back_to_back_classes_do_not_inherit_each_others_changes() {
        let mut app = app_with(vec![
            class(hm(9, 0), hm(10, 40)),
            class(hm(10, 40), hm(12, 20)),
        ]);
        assert!(app.grab_at(MON, hm(10, 30), 500, W, H));
        app.move_grab(100);
        app.grab_end();
        let mut canvas = Canvas::new(1000, 40);
        assert!(app.draw_at(&mut canvas, MON, hm(10, 40)));
        assert_eq!(minutes(brk_of(&app, 0)), (45, 55), "class 1 went back");
        assert_eq!(minutes(brk_of(&app, 1)), (45, 55), "class 2 never changed");
    }

    #[test]
    fn the_change_is_undone_when_the_day_changes() {
        let mut app = edited_first_class();
        let mut canvas = Canvas::new(1000, 40);
        assert!(!app.draw_at(&mut canvas, Weekday::Tue, NOW));
        assert_eq!(minutes(brk_of(&app, 0)), (45, 55));
    }

    #[test]
    fn a_drag_in_progress_is_dropped_with_its_class() {
        let mut app = two_double_classes();
        assert!(app.grab_at(MON, NOW, 500, W, H));
        app.move_grab(100);
        let mut canvas = Canvas::new(1000, 40);
        assert!(!app.draw_at(&mut canvas, MON, hm(13, 0)));
        assert!(app.grab.is_none());
        app.move_grab(50); // the pointer is still moving: harmless
        assert_eq!(minutes(brk_of(&app, 0)), (45, 55));
    }

    // ---- saving as the default for the slot ----

    const SAMPLE: &str = "# my schedule\n[monday]\nclasses = [\n  { start = \"09:00\", end = \"10:40\" },\n  { start = \"11:00\", end = \"12:40\" },\n]\n";

    /// An app whose config comes from a real file in a scratch folder.
    fn app_with_file(label: &str) -> (App, PathBuf, PathBuf) {
        let dir: PathBuf =
            std::env::temp_dir().join(format!("times-up-app-edit-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(config::FILE_NAME);
        fs::write(&path, SAMPLE).unwrap();
        let mut file = ConfigFile::new(&path);
        let config = file.load_or_create().config;
        (App::new(config, Some(file)), path, dir)
    }

    #[test]
    fn save_as_default_writes_the_class_to_the_file() {
        let (mut app, path, dir) = app_with_file("save");
        assert!(app.grab_at(MON, NOW, 500, W, H));
        app.move_grab(100); // break 55..65
        app.grab_end();
        // The first block now ends at 55 minutes, so its orange zone begins at 45 minutes.
        assert!(app.grab_at(MON, NOW, 450, W, H));
        app.move_grab(20); // 2 minutes later: 8 minutes of orange
        app.grab_end();
        let edited = app.store.borrow().schedule().classes(MON)[0].clone();

        assert_eq!(app.run_command(Command::SaveDefault), None);
        assert!(!app.menu_state().can_save_default, "nothing left to save");
        assert_eq!(
            app.store.borrow().schedule().classes(MON)[0],
            edited,
            "the bar keeps showing it"
        );

        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.starts_with("# my schedule"),
            "comments are kept:\n{text}"
        );
        let loaded = config::parse(&text);
        assert!(loaded.problems.is_empty(), "{:?}", loaded.problems);
        assert_eq!(loaded.config.schedule.classes(MON)[0], edited);
        assert_eq!(
            minutes(loaded.config.schedule.classes(MON)[0].brk.unwrap()),
            (55, 65)
        );
        // The other class in the file was not touched.
        assert_eq!(
            minutes(loaded.config.schedule.classes(MON)[1].brk.unwrap()),
            (45, 55)
        );

        // Our own save is not mistaken for an outside edit: the saved class stays as it is.
        let mut canvas = Canvas::new(100, 20);
        let _ = Handler::draw(&mut app, &mut canvas);
        assert_eq!(app.store.borrow().schedule().classes(MON)[0], edited);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_saved_default_survives_the_class_ending() {
        let (mut app, _path, dir) = app_with_file("survives");
        assert!(app.grab_at(MON, NOW, 500, W, H));
        app.move_grab(100);
        app.grab_end();
        assert_eq!(app.run_command(Command::SaveDefault), None);
        let mut canvas = Canvas::new(1000, 40);
        assert!(!app.draw_at(&mut canvas, MON, hm(10, 50)), "class over");
        assert_eq!(
            minutes(brk_of(&app, 0)),
            (55, 65),
            "it is the default now, so no undo"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn saving_without_changes_does_nothing() {
        let (mut app, path, dir) = app_with_file("nochange");
        assert_eq!(app.run_command(Command::SaveDefault), None);
        assert_eq!(fs::read_to_string(&path).unwrap(), SAMPLE);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn saving_without_a_config_file_is_reported() {
        let mut app = two_double_classes();
        assert!(app.grab_at(MON, NOW, 500, W, H));
        app.move_grab(100);
        let message = app.run_command(Command::SaveDefault).expect("a message");
        assert!(message.contains("no settings file"), "{message}");
        assert!(
            app.menu_state().can_save_default,
            "the change is still there"
        );
    }

    #[test]
    fn a_failed_save_is_reported_and_keeps_the_change() {
        let (mut app, path, dir) = app_with_file("readonly-save");
        assert!(app.grab_at(MON, NOW, 500, W, H));
        app.move_grab(100);
        let mut perm = fs::metadata(&path).unwrap().permissions();
        perm.set_readonly(true);
        fs::set_permissions(&path, perm).unwrap();

        let message = app.run_command(Command::SaveDefault).expect("a message");
        assert!(message.contains("Could not save the defaults"), "{message}");
        assert_eq!(fs::read_to_string(&path).unwrap(), SAMPLE);
        assert!(
            app.menu_state().can_save_default,
            "still unsaved, so it can be retried"
        );
        assert_eq!(minutes(brk_of(&app, 0)), (55, 65));

        let mut perm = fs::metadata(&path).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perm.set_readonly(false);
        fs::set_permissions(&path, perm).unwrap();
        assert_eq!(
            app.run_command(Command::SaveDefault),
            None,
            "works once the file can be written"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_class_that_is_gone_from_the_file_cannot_be_saved() {
        let (mut app, path, dir) = app_with_file("gone");
        assert!(app.grab_at(MON, NOW, 500, W, H));
        app.move_grab(100);
        // The teacher edits the file by hand so that this class no longer exists there.
        fs::write(
            &path,
            "[monday]\nclasses = [{ start = \"13:00\", end = \"14:40\" }]\n",
        )
        .unwrap();
        let message = app.run_command(Command::SaveDefault).expect("a message");
        assert!(message.contains("Could not save the defaults"), "{message}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn editing_the_file_by_hand_drops_unsaved_changes() {
        let (mut app, path, dir) = app_with_file("reload");
        assert!(app.grab_at(MON, NOW, 500, W, H));
        app.move_grab(100);
        assert!(app.menu_state().can_save_default);
        fs::write(&path, format!("{SAMPLE}# edited\n")).unwrap();
        let mut canvas = Canvas::new(100, 20);
        let _ = Handler::draw(&mut app, &mut canvas); // notices the new file
        assert!(!app.menu_state().can_save_default);
        assert!(app.grab.is_none());
        assert_eq!(minutes(brk_of(&app, 0)), (45, 55), "the file wins");
        let _ = fs::remove_dir_all(&dir);
    }

    // ---- where the text is drawn ----

    const WHITE: u32 = 0xffff_ffff;

    /// The box around the text-coloured pixels in `x0..x1` and `y0..y1`: (min x, max x, min y, max y).
    fn ink_box(
        canvas: &Canvas,
        x0: u32,
        x1: u32,
        y0: u32,
        y1: u32,
    ) -> Option<(u32, u32, u32, u32)> {
        let mut found: Option<(u32, u32, u32, u32)> = None;
        for y in y0..y1 {
            for x in x0..x1 {
                if canvas.pixel(x, y) == WHITE {
                    found = Some(match found {
                        None => (x, x, y, y),
                        Some((a, b, c, d)) => (a.min(x), b.max(x), c.min(y), d.max(y)),
                    });
                }
            }
        }
        found
    }

    fn centre(b: (u32, u32, u32, u32)) -> f32 {
        (b.0 + b.1) as f32 / 2.0
    }

    /// A 1000 x 60 bar drawn for the two double classes at `now`.
    fn drawn(app: &mut App, now: Secs) -> Canvas {
        let mut canvas = Canvas::new(1000, 60);
        assert!(app.draw_at(&mut canvas, MON, now));
        canvas
    }

    #[test]
    fn the_countdown_is_centred_in_the_block_it_counts() {
        let mut app = two_double_classes();
        // The first block spans 0..450 px, the break 450..550, the second block 550..1000.
        let first = ink_box(&drawn(&mut app, hm(9, 10)), 0, 1000, 0, 60).expect("text");
        assert!(
            (centre(first) - 225.0).abs() <= 8.0,
            "first block: {first:?}"
        );
        let second = ink_box(&drawn(&mut app, hm(10, 0)), 0, 1000, 0, 60).expect("text");
        assert!(
            (centre(second) - 775.0).abs() <= 8.0,
            "second block: {second:?}"
        );
        let brk = ink_box(&drawn(&mut app, hm(9, 50)), 0, 1000, 0, 60).expect("text");
        assert!((centre(brk) - 500.0).abs() <= 8.0, "break: {brk:?}");
    }

    #[test]
    fn the_countdown_of_a_single_block_class_stays_in_the_middle() {
        let mut app = app_with(vec![class(hm(9, 0), hm(9, 50))]);
        let ink = ink_box(&drawn(&mut app, hm(9, 10)), 0, 1000, 0, 60).expect("text");
        assert!((centre(ink) - 500.0).abs() <= 8.0, "{ink:?}");
    }

    #[test]
    fn the_lead_in_text_is_centred_on_the_whole_bar() {
        let mut app = two_double_classes();
        let ink = ink_box(&drawn(&mut app, hm(9, 0) - 120), 0, 1000, 0, 60).expect("text");
        assert!((centre(ink) - 500.0).abs() <= 8.0, "{ink:?}");
    }

    #[test]
    fn the_countdown_follows_a_moved_break() {
        let mut app = two_double_classes();
        // Break from 45..55 minutes to 25..35: block 1 is now 0..25%, the middle of it is at 125 px.
        assert!(app.grab_at(MON, NOW, 500, 1000, 60));
        app.move_grab(-200);
        app.grab_end();
        let ink = ink_box(&drawn(&mut app, hm(9, 10)), 0, 1000, 0, 40).expect("text");
        assert!((centre(ink) - 125.0).abs() <= 8.0, "{ink:?}");
    }

    // ---- the time shown while dragging ----

    /// Where the small label is drawn: the strip at the bottom of a 60 px bar.
    const STRIP: (u32, u32) = (42, 60);

    #[test]
    fn no_time_label_is_drawn_when_nothing_is_dragged() {
        let mut app = two_double_classes();
        let canvas = drawn(&mut app, NOW);
        assert_eq!(ink_box(&canvas, 300, 400, STRIP.0, STRIP.1), None);
    }

    #[test]
    fn dragging_a_zone_boundary_shows_its_start_time_small_at_the_bottom() {
        let mut app = two_double_classes();
        // The orange boundary of the first block is at 35 minutes = 350 px (09:35).
        assert!(app.grab_at(MON, NOW, 350, 1000, 60));
        let canvas = drawn(&mut app, NOW);
        let label =
            ink_box(&canvas, 250, 450, STRIP.0, STRIP.1).expect("a time label under the handle");
        assert!((centre(label) - 350.0).abs() <= 12.0, "{label:?}");
        // It is smaller than the countdown.
        let countdown = ink_box(&canvas, 0, 1000, 0, STRIP.0).expect("countdown");
        let (small, large) = (label.3 - label.2 + 1, countdown.3 - countdown.2 + 1);
        assert!(
            small * 3 <= large * 2,
            "label {small} px high, countdown {large} px"
        );
        // The countdown is still there and is not disturbed.
        assert!((centre(countdown) - 225.0).abs() <= 8.0, "{countdown:?}");

        // It shows the new time while the boundary moves (3 minutes later is 09:38), and
        // different digits mean different pixels.
        let before = canvas.pixels().to_vec();
        app.move_grab(30);
        let after = drawn(&mut app, NOW);
        assert_ne!(before, after.pixels(), "the label changed with the drag");
        let moved = ink_box(&after, 250, 450, STRIP.0, STRIP.1).expect("label");
        assert!(
            (centre(moved) - 380.0).abs() <= 12.0,
            "it follows the handle: {moved:?}"
        );
    }

    #[test]
    fn dragging_the_break_shows_when_it_starts_and_ends() {
        let mut app = two_double_classes();
        assert!(app.grab_at(MON, NOW, 500, 1000, 60));
        let canvas = drawn(&mut app, NOW);
        let label = ink_box(&canvas, 400, 600, STRIP.0, STRIP.1).expect("a label");
        // "09:45-09:55" is wider than a single time and centred under the break.
        assert!((centre(label) - 500.0).abs() <= 12.0, "{label:?}");
        let zone = {
            let mut app = two_double_classes();
            assert!(app.grab_at(MON, NOW, 350, 1000, 60));
            ink_box(&drawn(&mut app, NOW), 250, 450, STRIP.0, STRIP.1).unwrap()
        };
        assert!(
            label.1 - label.0 > (zone.1 - zone.0) * 3 / 2,
            "break {label:?} vs time {zone:?}"
        );
    }

    #[test]
    fn the_time_label_goes_away_when_the_drag_ends() {
        let mut app = two_double_classes();
        assert!(app.grab_at(MON, NOW, 350, 1000, 60));
        app.move_grab(20);
        assert!(ink_box(&drawn(&mut app, NOW), 250, 450, STRIP.0, STRIP.1).is_some());
        app.grab_end();
        assert_eq!(
            ink_box(&drawn(&mut app, NOW), 250, 450, STRIP.0, STRIP.1),
            None
        );
    }

    #[test]
    fn the_time_label_stays_on_the_bar_near_its_ends() {
        let mut app = two_double_classes();
        // The red boundary of the second block is at 95 minutes = 950 px, near the right end.
        assert!(app.grab_at(MON, NOW, 950, 1000, 60));
        let canvas = drawn(&mut app, NOW);
        let label = ink_box(&canvas, 800, 1000, STRIP.0, STRIP.1).expect("label");
        assert!(label.1 < 1000, "{label:?}");
        assert!(label.0 > 850, "{label:?}");
    }

    #[test]
    fn no_time_label_is_drawn_for_another_days_drag() {
        let mut app = two_double_classes();
        assert!(app.grab_at(MON, NOW, 350, 1000, 60));
        let mut canvas = Canvas::new(1000, 60);
        // Tuesday has no classes, so nothing is drawn at all, and certainly not the label.
        assert!(!app.draw_at(&mut canvas, Weekday::Tue, NOW));
        assert_eq!(ink_box(&canvas, 0, 1000, 0, 60), None);
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
