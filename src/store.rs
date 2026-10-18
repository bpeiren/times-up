//! The single source of truth for the timetable and settings: file <-> `Store` <-> views.
//!
//! The live bar and the Settings window read the [`Config`] from here. The file poll and every
//! edit go through the store, and each change that is applied bumps the revision, so a view only
//! has to compare revisions to know that it must redraw. Plain Rust without Win32.
//!
//! The schedule held here is the saved one, what the file says. The teacher can change the class
//! that is running on the bar itself; that temporary change is kept apart as the [`Live`] class
//! and only the bar looks at it ([`Store::live_schedule`]). So an edit of the saved timetable
//! can never write a temporary change into the file by accident.

use std::borrow::Cow;
use std::path::Path;

use crate::config::{Colors, Config, ConfigError, ConfigFile, General, WindowRect};
use crate::schedule::{Class, Schedule, Weekday};

/// A temporary version of a class that is running, changed on the bar and not saved. It belongs
/// to the saved class with the same day, start and end.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Live {
    pub day: Weekday,
    pub class: Class,
}

impl Live {
    fn is_version_of(&self, day: Weekday, class: &Class) -> bool {
        self.day == day && (self.class.start, self.class.end) == (class.start, class.end)
    }
}

pub struct Store {
    config: Config,
    live: Option<Live>,
    /// Where the config came from; `None` when there is no file to read or write.
    file: Option<ConfigFile>,
    revision: u64,
}

impl Store {
    pub fn new(config: Config, file: Option<ConfigFile>) -> Self {
        Self {
            config,
            live: None,
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

    pub fn live(&self) -> Option<&Live> {
        self.live.as_ref()
    }

    /// The saved class that `live` is a temporary version of; `None` when there is no such class
    /// any more (the file changed), in which case the live version does not apply.
    pub fn live_base(&self) -> Option<(usize, &Class)> {
        let live = self.live.as_ref()?;
        self.config
            .schedule
            .classes(live.day)
            .iter()
            .enumerate()
            .find(|(_, c)| live.is_version_of(live.day, c))
    }

    /// Change the running class on the bar only. The class must keep the start and end of the
    /// saved one. Not an edit of the timetable, so the revision stays as it is.
    pub fn set_live(&mut self, day: Weekday, class: Class) {
        self.live = Some(Live { day, class });
    }

    /// Forget the temporary changes: the bar shows the saved class again.
    pub fn clear_live(&mut self) {
        self.live = None;
    }

    /// The schedule as the bar shows it: the saved one with the temporary version of the running
    /// class in place of the saved version.
    pub fn live_schedule(&self) -> Cow<'_, Schedule> {
        let (Some(live), Some((index, _))) = (self.live.as_ref(), self.live_base()) else {
            return Cow::Borrowed(&self.config.schedule);
        };
        let mut schedule = self.config.schedule.clone();
        if let Some(class) = schedule.class_mut(live.day, index) {
            *class = live.class.clone();
        }
        Cow::Owned(schedule)
    }

    /// Make the temporary version the saved one in memory (after it was written to the file as
    /// the default of its slot).
    ///
    /// The saved timetable changed, so the revision moves and the Settings window redraws.
    pub fn commit_live(&mut self) {
        let index = self.live_base().map(|(index, _)| index);
        if let (Some(live), Some(index)) = (self.live.take(), index)
            && let Some(class) = self.config.schedule.class_mut(live.day, index)
        {
            *class = live.class;
            self.revision += 1;
        }
    }

    /// Take over what the file says when it changed since it was last read or written by us.
    /// Returns whether it did; unsaved live changes are dropped then, because the file is the truth.
    pub fn poll(&mut self) -> bool {
        let Some(loaded) = self.file.as_mut().and_then(ConfigFile::poll) else {
            return false;
        };
        self.config = loaded.config;
        // The file is the truth: changes made on the bar and not saved are dropped.
        self.live = None;
        self.revision += 1;
        true
    }

    /// Replace the classes of `day`. They are validated first, then only that day's `classes`
    /// array is rewritten in the file, starting from the file as it is now. Nothing changes when
    /// the classes are invalid or the file cannot be written.
    ///
    /// Returns whether the edit changed the settings (break, warning times) of the saved class
    /// that has temporary changes on the bar. Those temporary changes are left as they are for
    /// the caller to decide about. A class that was moved, resized or removed is no longer the
    /// class they were made to: they stop applying and the bar drops them.
    pub fn set_day(&mut self, day: Weekday, classes: Vec<Class>) -> Result<bool, ConfigError> {
        let before = self.live_base().map(|(_, c)| c.clone());
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
        let after = self.live_base().map(|(_, c)| c);
        Ok(before.is_some() && after.is_some() && before.as_ref() != after)
    }

    /// Change the lead-in, linger and the defaults of the classes. The values are validated, the
    /// `[general]` section of the file is rewritten and the file is read again, because classes
    /// without their own break or thresholds follow these defaults. Nothing changes when they
    /// are invalid or the file cannot be written.
    pub fn set_general(&mut self, general: General) -> Result<(), ConfigError> {
        general.validate().map_err(ConfigError::Invalid)?;
        let file = self.file.as_mut().ok_or(ConfigError::NoFile)?;
        file.save_general(&general)?;
        self.config = file.reload()?.config;
        self.revision += 1;
        Ok(())
    }

    /// Change the colours of the bar. Nothing changes when the file cannot be written.
    pub fn set_colors(&mut self, colors: Colors) -> Result<(), ConfigError> {
        self.file
            .as_mut()
            .ok_or(ConfigError::NoFile)?
            .save_colors(&colors)?;
        self.config.colors = colors;
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

    /// Remember where the Settings window is; like the bar's position this is not an edit.
    pub fn save_settings_window(&mut self, rect: WindowRect) -> Result<(), ConfigError> {
        self.config.settings_window = rect;
        self.file
            .as_mut()
            .ok_or(ConfigError::NoFile)?
            .save_settings_window(&rect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Rgba;
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
            format!("{SAMPLE}\n[colors]\nlesson = \"#010203\"\n"),
        )
        .unwrap();
        store
            .set_day(Weekday::Mon, vec![class((9, 0), (9, 50))])
            .unwrap();
        assert!(dir.text().contains("lesson = \"#010203\""));
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
        let mut live = store.schedule().classes(Weekday::Mon)[0].clone();
        live.brk = None;
        store.set_live(Weekday::Mon, live);
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

    /// 09:00-10:40 on Monday, with its default break, and a live version without the break.
    fn with_live(name: &str) -> (Scratch, Store, Class) {
        let dir = Scratch::new(name, SAMPLE);
        let mut store = dir.store();
        let mut live = store.schedule().classes(Weekday::Mon)[0].clone();
        assert!(live.brk.is_some());
        live.brk = None;
        store.set_live(Weekday::Mon, live.clone());
        (dir, store, live)
    }

    #[test]
    fn the_bar_sees_the_live_class_and_the_saved_schedule_does_not() {
        let (_dir, store, live) = with_live("live-view");
        assert_eq!(store.live_schedule().classes(Weekday::Mon)[0], live);
        assert!(store.schedule().classes(Weekday::Mon)[0].brk.is_some());
        assert_eq!(store.live_base().map(|(i, _)| i), Some(0));
        assert_eq!(store.live().map(|l| l.day), Some(Weekday::Mon));
    }

    #[test]
    fn clearing_the_live_class_shows_the_saved_one_again() {
        let (_dir, mut store, _) = with_live("live-clear");
        store.clear_live();
        assert!(store.live().is_none());
        assert_eq!(
            store.live_schedule().classes(Weekday::Mon),
            store.schedule().classes(Weekday::Mon)
        );
    }

    #[test]
    fn committing_the_live_class_makes_it_the_saved_one_in_memory() {
        let (_dir, mut store, live) = with_live("live-commit");
        store.commit_live();
        assert!(store.live().is_none());
        assert_eq!(store.schedule().classes(Weekday::Mon)[0], live);
    }

    #[test]
    fn committing_moves_the_revision_so_the_settings_window_redraws() {
        let (_dir, mut store, _) = with_live("live-commit-revision");
        let before = store.revision();
        store.commit_live();
        assert_eq!(store.revision(), before + 1);
        store.commit_live();
        assert_eq!(store.revision(), before + 1, "nothing left to commit");
    }

    #[test]
    fn an_external_edit_drops_the_live_class() {
        let (dir, mut store, _) = with_live("live-external");
        fs::write(dir.file(), SAMPLE.replace("# my schedule", "# changed")).unwrap();
        assert!(store.poll());
        assert!(store.live().is_none());
    }

    #[test]
    fn saving_another_class_never_writes_the_live_changes_to_the_file() {
        let (dir, mut store, live) = with_live("live-other");
        let mut classes = store.schedule().classes(Weekday::Mon).to_vec();
        classes.push(class((11, 0), (11, 50)));
        let touched = store.set_day(Weekday::Mon, classes).unwrap();
        assert!(!touched, "the running class was not edited");
        let saved = crate::config::parse(&dir.text()).config;
        assert!(saved.schedule.classes(Weekday::Mon)[0].brk.is_some());
        // The bar still shows the live version.
        assert_eq!(store.live_schedule().classes(Weekday::Mon)[0], live);
        assert_eq!(store.live_schedule().classes(Weekday::Mon).len(), 2);
    }

    #[test]
    fn editing_the_running_class_is_reported_and_leaves_the_live_class_alone() {
        let (_dir, mut store, live) = with_live("live-touched");
        let mut changed = store.schedule().classes(Weekday::Mon)[0].clone();
        changed.warnings[0] = crate::schedule::Warnings::new(7 * MINUTE, 2 * MINUTE).unwrap();
        assert!(store.set_day(Weekday::Mon, vec![changed.clone()]).unwrap());
        assert_eq!(
            store.live().map(|l| &l.class),
            Some(&live),
            "left for the caller"
        );
        assert_eq!(store.live_schedule().classes(Weekday::Mon)[0], live);
        assert_eq!(store.schedule().classes(Weekday::Mon)[0], changed);
    }

    #[test]
    fn moving_or_removing_the_running_class_is_no_conflict_and_the_live_class_stops_applying() {
        let (_dir, mut store, _) = with_live("live-moved");
        let moved = class((9, 5), (10, 45));
        assert!(!store.set_day(Weekday::Mon, vec![moved.clone()]).unwrap());
        assert_eq!(store.live_base(), None);
        assert_eq!(store.live_schedule().classes(Weekday::Mon), [moved]);

        let (_dir, mut store, _) = with_live("live-removed");
        assert!(!store.set_day(Weekday::Mon, Vec::new()).unwrap());
        assert!(store.live_schedule().classes(Weekday::Mon).is_empty());
    }

    #[test]
    fn an_edit_of_another_day_is_not_a_conflict() {
        let (_dir, mut store, _) = with_live("live-day");
        let touched = store
            .set_day(Weekday::Tue, vec![class((9, 0), (9, 50))])
            .unwrap();
        assert!(!touched);
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

    // ---- general settings and colours ----

    const GENERAL: &str = "# top\n[general]\nlead_in_minutes = 5   # before\nlinger_seconds = 10\n\n[colors]\ngreen = \"#2ecc71\" # nice\n\n[monday]\nclasses = [\n  { start = \"09:00\", end = \"10:40\" },\n  { start = \"11:00\", end = \"11:50\", warning = [7, 7], final = [3, 3] },\n]\n";

    fn general(store: &Store) -> General {
        store.config().general()
    }

    #[test]
    fn set_general_rewrites_the_values_and_keeps_comments_and_the_rest() {
        let dir = Scratch::new("general", GENERAL);
        let mut store = dir.store();
        let mut g = general(&store);
        g.lead_in = 8 * MINUTE;
        g.linger = 25;
        g.default_break = 15 * MINUTE;
        g.warnings = crate::schedule::Warnings::new(12 * MINUTE, 4 * MINUTE).unwrap();
        store.set_general(g).unwrap();

        let text = dir.text();
        assert!(text.contains("lead_in_minutes = 8   # before"), "{text}");
        assert!(text.contains("linger_seconds = 25"));
        assert!(text.contains("default_break_minutes = 15"));
        assert!(text.contains("warning_minutes = 12") && text.contains("final_minutes = 4"));
        assert!(text.starts_with("# top\n"));
        assert!(text.contains("green = \"#2ecc71\" # nice"));
        assert!(
            text.contains(
                "{ start = \"11:00\", end = \"11:50\", warning = [7, 7], final = [3, 3] }"
            )
        );
        assert_eq!(general(&store), g);
        assert_eq!(store.revision(), 1);
        assert!(!store.poll(), "our own write is not an external edit");
    }

    #[test]
    fn classes_that_follow_the_defaults_follow_the_new_ones() {
        let dir = Scratch::new("follow", GENERAL);
        let mut store = dir.store();
        let mut g = general(&store);
        g.default_break = 20 * MINUTE;
        g.warnings = crate::schedule::Warnings::new(15 * MINUTE, 6 * MINUTE).unwrap();
        store.set_general(g).unwrap();

        let classes = store.schedule().classes(Weekday::Mon);
        assert_eq!(
            classes[0].brk.unwrap().len,
            20 * MINUTE,
            "no own break: the default"
        );
        assert_eq!(classes[0].warnings[0].warning, 15 * MINUTE);
        assert_eq!(
            classes[1].warnings[0].warning,
            7 * MINUTE,
            "its own thresholds stay"
        );
        assert_eq!(store.schedule().lead_in, g.lead_in);
    }

    #[test]
    fn invalid_general_values_are_refused_and_nothing_is_written() {
        let dir = Scratch::new("general-invalid", GENERAL);
        let mut store = dir.store();
        let before = dir.text();
        let good = general(&store);
        let bad = [
            General {
                lead_in: 61 * MINUTE,
                ..good
            },
            General {
                linger: 301,
                ..good
            },
            General {
                default_break: 0,
                ..good
            },
            General {
                default_break: 31 * MINUTE,
                ..good
            },
            General {
                warnings: crate::schedule::Warnings {
                    warning: 3 * MINUTE,
                    final_: 5 * MINUTE,
                },
                ..good
            },
            General {
                warnings: crate::schedule::Warnings {
                    warning: 61 * MINUTE,
                    final_: 1,
                },
                ..good
            },
        ];
        for g in bad {
            assert!(
                matches!(store.set_general(g), Err(ConfigError::Invalid(_))),
                "{g:?}"
            );
        }
        assert_eq!(dir.text(), before);
        assert_eq!(general(&store), good);
        assert_eq!(store.revision(), 0);
    }

    #[test]
    fn general_without_a_section_adds_it_and_without_a_file_is_refused() {
        let dir = Scratch::new("general-new", SAMPLE);
        let mut store = dir.store();
        let mut g = general(&store);
        g.lead_in = 2 * MINUTE;
        store.set_general(g).unwrap();
        let text = dir.text();
        assert!(
            text.contains("[general]") && text.contains("lead_in_minutes = 2"),
            "{text}"
        );
        assert!(text.contains("# my schedule") && text.contains("# keep me"));

        let mut none = Store::new(Config::default(), None);
        assert!(matches!(none.set_general(g), Err(ConfigError::NoFile)));
        assert!(matches!(
            none.set_colors(Colors::default()),
            Err(ConfigError::NoFile)
        ));
    }

    const OLD_KEYS: &str = "[general]\norange_minutes = 8   # warm\nred_minutes = 2\n\n[colors]\ngreen = \"#2ecc71\" # nice\norange = \"#f39c12\"\nred = \"#e74c3c\"\n\n[monday]\nclasses = [\n  { start = \"09:00\", end = \"09:50\", orange = 7, red = 3 },\n]\n";

    #[test]
    fn writing_the_general_values_into_an_old_file_uses_the_new_keys() {
        let dir = Scratch::new("old-general", OLD_KEYS);
        let mut store = dir.store();
        let mut g = general(&store);
        g.warnings = crate::schedule::Warnings::new(12 * MINUTE, 4 * MINUTE).unwrap();
        store.set_general(g).unwrap();
        let text = dir.text();
        assert!(text.contains("warning_minutes = 12   # warm"), "{text}");
        assert!(text.contains("final_minutes = 4"), "{text}");
        assert!(
            !text.contains("orange_minutes") && !text.contains("red_minutes"),
            "{text}"
        );
        assert_eq!(general(&dir.store()), g);
    }

    #[test]
    fn writing_the_colours_into_an_old_file_uses_the_new_keys() {
        let dir = Scratch::new("old-colors", OLD_KEYS);
        let mut store = dir.store();
        let mut c = store.config().colors;
        c.warning = Rgba::rgb(1, 2, 3);
        store.set_colors(c).unwrap();
        let text = dir.text();
        for key in ["lesson", "warning", "final"] {
            assert!(text.contains(&format!("{key} = \"#")), "{key} in {text}");
        }
        assert!(text.contains("lesson = \"#2ecc71\" # nice"), "{text}");
        let colors = text.split("[monday]").next().unwrap();
        for old in ["green", "orange =", "red ="] {
            assert!(!colors.contains(old), "{old} in {text}");
        }
        assert_eq!(dir.store().config().colors, c);
    }

    #[test]
    fn editing_a_day_of_an_old_file_writes_the_new_class_keys() {
        let dir = Scratch::new("old-day", OLD_KEYS);
        let mut store = dir.store();
        let mut c = store.schedule().classes(Weekday::Mon)[0].clone();
        c.warnings[0] = crate::schedule::Warnings::new(6 * MINUTE, MINUTE).unwrap();
        store.set_day(Weekday::Mon, vec![c.clone()]).unwrap();
        let text = dir.text();
        assert!(
            text.contains("warning = [6, 7]") && text.contains("final = [1, 3]"),
            "{text}"
        );
        assert!(
            !text.contains("orange = 7") && !text.contains("red = 3"),
            "{text}"
        );
        assert_eq!(dir.store().schedule().classes(Weekday::Mon)[0], c);
    }

    #[test]
    fn set_colors_writes_all_six_and_keeps_comments() {
        let dir = Scratch::new("colors", GENERAL);
        let mut store = dir.store();
        let mut c = store.config().colors;
        c.lesson = Rgba::rgb(1, 2, 3);
        c.track.a = 0x40;
        store.set_colors(c).unwrap();

        let text = dir.text();
        assert!(text.contains("lesson = \"#010203\" # nice"), "{text}");
        assert!(text.contains("track = \"#33333340\""));
        for key in ["warning", "final", "break", "text"] {
            assert!(text.contains(&format!("{key} = \"#")), "{key} in {text}");
        }
        assert_eq!(store.config().colors, c);
        assert_eq!(store.revision(), 1);
        // And what was written reads back the same.
        let mut again = dir.store();
        assert_eq!(again.config().colors, c);
        assert!(!again.poll());

        store.set_colors(Colors::default()).unwrap();
        assert_eq!(dir.store().config().colors, Colors::default());
    }
}
