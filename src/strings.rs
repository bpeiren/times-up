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
pub const BUTTON_ADD_BREAK: &str = "Add break";
pub const BUTTON_REMOVE_BREAK: &str = "Remove break";

pub const DETAIL_HINT: &str = "Select a class to edit its break and warning times";
pub const TIP_IDLE: &str = "Drag the lines to change the break and the orange and red times";
pub const TIP_ADDING: &str = "Click where the break should be; Esc cancels";
pub const TIP_CLASS: &str = "Class";
pub const TIP_BLOCK: &str = "Block";
pub const TIP_BREAK: &str = "Break";
pub const TIP_ORANGE: &str = "Orange";
pub const TIP_RED: &str = "Red";
pub const TIP_FROM: &str = "from";
