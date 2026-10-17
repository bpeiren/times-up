//! The single source of truth for the timetable and settings: file <-> `Store` <-> views.
//!
//! The live bar (and later the Settings window) read the [`Config`] from here. The file poll and
//! every edit go through the store, and each change that is applied bumps the revision, so a view
//! only has to compare revisions to know that it must redraw. Plain Rust without Win32.

use std::path::Path;

use crate::config::{Config, ConfigError, ConfigFile, WindowRect};
use crate::schedule::{Class, Schedule, Weekday};

pub struct Store {
    config: Config,
    /// Where the config came from; `None` when there is no file to read or write.
    file: Option<ConfigFile>,
    revision: u64,
}

impl Store {
    pub fn new(config: Config, file: Option<ConfigFile>) -> Self {
        Self {
            config,
            file,
            revision: 0,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    #[cfg(test)]
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    pub fn schedule(&self) -> &Schedule {
        &self.config.schedule
    }

    /// Changes whenever the config changed through the file or an edit.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn path(&self) -> Option<&Path> {
        self.file.as_ref().map(ConfigFile::path)
    }

    /// Mutable access for changes that live only on the bar and are never saved (the teacher
    /// stretching the running class). They are not edits, so the revision stays as it is.
    pub fn schedule_mut(&mut self) -> &mut Schedule {
        &mut self.config.schedule
    }

    /// Take over what the file says when it changed since it was last read or written by us.
    /// Returns whether it did; unsaved live changes are dropped then, because the file is the truth.
    pub fn poll(&mut self) -> bool {
        let Some(loaded) = self.file.as_mut().and_then(ConfigFile::poll) else {
            return false;
        };
        self.config = loaded.config;
        self.revision += 1;
        true
    }

    /// Replace the classes of `day`. They are validated first, then only that day's `classes`
    /// array is rewritten in the file, starting from the file as it is now. Nothing changes when
    /// the classes are invalid or the file cannot be written.
    pub fn set_day(&mut self, day: Weekday, classes: Vec<Class>) -> Result<(), ConfigError> {
        let mut schedule = self.config.schedule.clone();
        schedule
            .set_day(day, classes)
            .map_err(|e| ConfigError::Invalid(e.to_string()))?;
        let file = self.file.as_mut().ok_or(ConfigError::NoFile)?;
        file.save_day(
            day,
            schedule.classes(day),
            self.config.default_break,
            self.config.warnings,
        )?;
        self.config.schedule = schedule;
        self.revision += 1;
        Ok(())
    }

    /// Store the break and thresholds of a class as its defaults in the file.
    pub fn save_class(&mut self, day: Weekday, class: &Class) -> Result<(), ConfigError> {
        self.file
            .as_mut()
            .ok_or(ConfigError::NoFile)?
            .save_class(day, class)
    }

    /// Remember where the bar is. The position is not part of what views draw, so the revision
    /// stays as it is.
    pub fn save_window(&mut self, rect: WindowRect) -> Result<(), ConfigError> {
        self.config.window = rect;
        self.file
            .as_mut()
            .ok_or(ConfigError::NoFile)?
            .save_window(&rect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::{Break, DEFAULT_BREAK_LEN, MINUTE, hm};
    use std::fs;
    use std::path::PathBuf;

    const SAMPLE: &str = "# my schedule\n[monday]\nclasses = [\n  { start = \"09:00\", end = \"10:40\" },\n]\n\n[friday]\n# keep me\nclasses = [{ start = \"13:00\", end = \"13:50\" }] # and me\n";

    /// A scratch directory with a config file, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str, text: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("times-up-store-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("times-up.toml"), text).unwrap();
            Self(dir)
        }

        fn file(&self) -> PathBuf {
            self.0.join("times-up.toml")
        }

        fn store(&self) -> Store {
            let mut file = ConfigFile::new(self.file());
            let config = file.load_or_create().config;
            Store::new(config, Some(file))
        }

        fn text(&self) -> String {
            fs::read_to_string(self.file()).unwrap()
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn class(start: (u32, u32), end: (u32, u32)) -> Class {
        Class::new(hm(start.0, start.1), hm(end.0, end.1), DEFAULT_BREAK_LEN).unwrap()
    }

    #[test]
    fn set_day_rewrites_only_that_day_and_bumps_the_revision() {
        let dir = Scratch::new("set-day", SAMPLE);
        let mut store = dir.store();
        assert_eq!(store.revision(), 0);

        let classes = vec![class((9, 0), (9, 50)), class((10, 0), (10, 50))];
        store.set_day(Weekday::Mon, classes.clone()).unwrap();

        assert_eq!(store.revision(), 1);
        assert_eq!(store.schedule().classes(Weekday::Mon), classes);
        let text = dir.text();
        let monday = "# my schedule\n[monday]\nclasses = [\n  { start = \"09:00\", end = \"09:50\" },\n  { start = \"10:00\", end = \"10:50\" },\n]\n";
        assert!(text.starts_with(monday), "{text}");
        // Friday, with its comments, is byte for byte what it was.
        assert!(
            text.ends_with(&SAMPLE[SAMPLE.find("\n[friday]").unwrap()..]),
            "{text}"
        );
        // Reading the file back gives the same timetable.
        let parsed = crate::config::parse(&text).config;
        assert_eq!(parsed.schedule.classes(Weekday::Mon), classes);
    }

    #[test]
    fn set_day_creates_a_missing_day_and_empties_a_day() {
        let dir = Scratch::new("create", SAMPLE);
        let mut store = dir.store();
        store
            .set_day(Weekday::Wed, vec![class((8, 30), (9, 20))])
            .unwrap();
        store.set_day(Weekday::Mon, Vec::new()).unwrap();
        let parsed = crate::config::parse(&dir.text()).config;
        assert_eq!(parsed.schedule.classes(Weekday::Wed).len(), 1);
        assert!(parsed.schedule.classes(Weekday::Mon).is_empty());
        assert_eq!(store.revision(), 2);
    }

    #[test]
    fn breaks_and_thresholds_survive_a_round_trip() {
        let dir = Scratch::new("break", SAMPLE);
        let mut store = dir.store();
        let long = class((9, 0), (10, 40));
        assert!(long.brk.is_some());
        let mut moved = class((11, 0), (12, 40));
        moved.brk = Some(Break {
            start: 30 * MINUTE,
            len: 5 * MINUTE,
        });
        let mut gone = class((13, 0), (14, 40));
        gone.brk = None;
        let mut warned = class((15, 0), (15, 50));
        warned.warnings[0] = crate::schedule::Warnings::new(8 * MINUTE, 3 * MINUTE).unwrap();
        let classes = vec![long, moved, gone, warned];
        store.set_day(Weekday::Tue, classes.clone()).unwrap();
        let parsed = crate::config::parse(&dir.text()).config;
        assert_eq!(parsed.schedule.classes(Weekday::Tue), classes);
    }

    #[test]
    fn invalid_classes_change_nothing() {
        let dir = Scratch::new("invalid", SAMPLE);
        let mut store = dir.store();
        let before = dir.text();
        let overlapping = vec![class((9, 0), (10, 0)), class((9, 30), (10, 30))];
        assert!(matches!(
            store.set_day(Weekday::Mon, overlapping),
            Err(ConfigError::Invalid(_))
        ));
        assert_eq!(dir.text(), before);
        assert_eq!(store.revision(), 0);
        assert_eq!(store.schedule().classes(Weekday::Mon).len(), 1);
    }

    #[test]
    fn an_edit_starts_from_the_freshly_read_file() {
        let dir = Scratch::new("fresh", SAMPLE);
        let mut store = dir.store();
        // Someone edits another part of the file behind our back.
        fs::write(
            dir.file(),
            format!("{SAMPLE}\n[colors]\ngreen = \"#010203\"\n"),
        )
        .unwrap();
        store
            .set_day(Weekday::Mon, vec![class((9, 0), (9, 50))])
            .unwrap();
        assert!(dir.text().contains("green = \"#010203\""));
    }

    #[test]
    fn our_own_write_is_not_seen_as_an_external_change() {
        let dir = Scratch::new("own", SAMPLE);
        let mut store = dir.store();
        store
            .set_day(Weekday::Mon, vec![class((9, 0), (9, 50))])
            .unwrap();
        assert!(!store.poll());
        assert_eq!(store.revision(), 1);
    }

    #[test]
    fn an_external_edit_reloads_and_bumps_the_revision() {
        let dir = Scratch::new("external", SAMPLE);
        let mut store = dir.store();
        assert!(!store.poll());
        let other = "[tuesday]\nclasses = [{ start = \"10:00\", end = \"10:50\" }]\n\n";
        fs::write(dir.file(), other).unwrap();
        assert!(store.poll());
        assert_eq!(store.revision(), 1);
        assert!(store.schedule().classes(Weekday::Mon).is_empty());
        assert_eq!(store.schedule().classes(Weekday::Tue).len(), 1);
    }

    #[test]
    fn live_changes_and_window_moves_do_not_bump_the_revision() {
        let dir = Scratch::new("live", SAMPLE);
        let mut store = dir.store();
        store.schedule_mut().class_mut(Weekday::Mon, 0).unwrap().end += MINUTE;
        let rect = WindowRect {
            x: Some(5),
            y: Some(6),
            width: Some(700),
            height: Some(40),
        };
        store.save_window(rect).unwrap();
        assert_eq!(store.revision(), 0);
        assert_eq!(store.config().window, rect);
    }

    #[test]
    fn without_a_file_edits_are_refused() {
        let mut store = Store::new(Config::default(), None);
        assert!(matches!(
            store.set_day(Weekday::Mon, vec![class((9, 0), (9, 50))]),
            Err(ConfigError::NoFile)
        ));
        assert_eq!(store.revision(), 0);
        assert!(store.schedule().classes(Weekday::Mon).is_empty());
        assert!(!store.poll());
    }
}
