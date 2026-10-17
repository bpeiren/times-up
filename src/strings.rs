//! Every piece of text the app shows, in one place so a translation only has to touch this file.

pub const APP_NAME: &str = "Time's up";

pub const LABEL_SAVE_DEFAULT: &str = "Save as default for this slot";
pub const LABEL_SETTINGS: &str = "Settings...";
pub const LABEL_AUTOSTART: &str = "Start with Windows";
pub const LABEL_QUIT: &str = "Quit";

pub const SETTINGS_TITLE: &str = "Time's up - Settings";
pub const SECTION_TIMETABLE: &str = "Timetable";

/// Indexed by `Weekday::index`.
pub const DAY_NAMES: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
pub const BUTTON_REMOVE_CLASS: &str = "Remove class";
