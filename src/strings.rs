//! Every piece of text the app shows, in one place so a translation only has to touch this file.

pub const APP_NAME: &str = "Time's up";

pub const LABEL_SAVE_DEFAULT: &str = "Save as default for this slot";
pub const LABEL_SETTINGS: &str = "Settings...";
pub const LABEL_AUTOSTART: &str = "Start with Windows";
pub const LABEL_QUIT: &str = "Quit";

pub const SETTINGS_TITLE: &str = "Time's up - Settings";
pub const SECTION_TIMETABLE: &str = "Timetable";
pub const SECTION_GENERAL: &str = "General";

pub const GENERAL_COLORS: &str = "Colours";
pub const GENERAL_TIMES: &str = "Times";
pub const COLOR_GREEN: &str = "Green";
pub const COLOR_ORANGE: &str = "Orange";
pub const COLOR_RED: &str = "Red";
pub const COLOR_BREAK: &str = "Break";
pub const COLOR_TRACK: &str = "Track";
pub const COLOR_TEXT: &str = "Text";
pub const FIELD_TRACK_OPACITY: &str = "Track opacity";
pub const BUTTON_RESET_COLORS: &str = "Reset colours";
pub const PREVIEW_TEXT: &str = "12:34";
pub const FIELD_LEAD_IN: &str = "Lead-in";
pub const FIELD_LINGER: &str = "Stays after last class";
pub const FIELD_BREAK: &str = "Default break";
pub const FIELD_ORANGE: &str = "Orange before end";
pub const FIELD_RED: &str = "Red before end";
pub const AT_LEAST: &str = "cannot go below";
pub const AT_MOST: &str = "cannot go above";

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

pub const BANNER_TEXT: &str = "The running class has temporary changes from the bar.";
pub const BANNER_REPLACE: &str = "Replace (Enter)";
pub const BANNER_SAVE_ONLY: &str = "Save only (Esc)";

pub const DETAIL_HINT: &str = "Select a class to edit its break and warning times";
pub const TIP_IDLE: &str = "Drag the lines to change the break and the orange and red times";
pub const TIP_ADDING: &str = "Click where the break should be; Esc cancels";
pub const TIP_CLASS: &str = "Class";
pub const TIP_BLOCK: &str = "Block";
pub const TIP_BREAK: &str = "Break";
pub const TIP_ORANGE: &str = "Orange";
pub const TIP_RED: &str = "Red";
pub const TIP_FROM: &str = "from";
