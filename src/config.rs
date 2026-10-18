//! `times-up.toml`: the portable settings file that lives next to the exe.
//!
//! Reading never fails hard: every bad value is reported in [`Loaded::problems`] and replaced by
//! its default, so a typo in the file cannot stop the timer. Writing edits the file in place
//! (via `toml_edit`) so the teacher's comments and layout survive.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use toml_edit::{DocumentMut, Item, Table, TableLike, Value, value};

use crate::schedule::{
    Break, Class, DEFAULT_BREAK_LEN, DEFAULT_FINAL, DEFAULT_LEAD_IN, DEFAULT_LINGER,
    DEFAULT_WARNING, MINUTE, Schedule, Secs, Warnings, Weekday,
};

pub const FILE_NAME: &str = "times-up.toml";

const DAYS: [(&str, Weekday); 7] = [
    ("monday", Weekday::Mon),
    ("tuesday", Weekday::Tue),
    ("wednesday", Weekday::Wed),
    ("thursday", Weekday::Thu),
    ("friday", Weekday::Fri),
    ("saturday", Weekday::Sat),
    ("sunday", Weekday::Sun),
];

/// The commented file written when none exists yet.
pub const DEFAULT_CONFIG: &str = include_str!("../assets/times-up.sample.toml");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Parse `#RRGGBB` or `#RRGGBBAA`.
    pub fn parse(s: &str) -> Option<Self> {
        let hex = s.trim().strip_prefix('#')?;
        if !hex.is_ascii() || !matches!(hex.len(), 6 | 8) {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        Some(Self {
            r: byte(0)?,
            g: byte(2)?,
            b: byte(4)?,
            a: if hex.len() == 8 { byte(6)? } else { 255 },
        })
    }

    /// `#rrggbb`, or `#rrggbbaa` when not fully opaque; what [`Rgba::parse`] reads back.
    pub fn to_hex(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colors {
    pub lesson: Rgba,
    pub warning: Rgba,
    pub final_: Rgba,
    pub break_gap: Rgba,
    pub track: Rgba,
    pub text: Rgba,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            lesson: Rgba::rgb(0x2e, 0xcc, 0x71),
            warning: Rgba::rgb(0xf3, 0x9c, 0x12),
            final_: Rgba::rgb(0xe7, 0x4c, 0x3c),
            break_gap: Rgba::rgb(0xbd, 0xc3, 0xc7),
            track: Rgba {
                r: 0x33,
                g: 0x33,
                b: 0x33,
                a: 0x99,
            },
            text: Rgba::rgb(0xff, 0xff, 0xff),
        }
    }
}

/// Saved window rectangle; every part is optional until the user first moves the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WindowRect {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub schedule: Schedule,
    pub colors: Colors,
    pub window: WindowRect,
    /// Where the Settings window was last.
    pub settings_window: WindowRect,
    /// Length of the break that classes of 100 minutes or more get by default.
    pub default_break: Secs,
    /// Warning thresholds used by classes that do not set their own.
    pub warnings: Warnings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schedule: Schedule::new(),
            colors: Colors::default(),
            window: WindowRect::default(),
            settings_window: WindowRect::default(),
            default_break: DEFAULT_BREAK_LEN,
            warnings: Warnings::default(),
        }
    }
}

/// The `[general]` times: when the bar appears and goes, and the defaults classes follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct General {
    /// The bar appears this long before a class starts.
    pub lead_in: Secs,
    /// ...and stays this long after the last class ends.
    pub linger: Secs,
    /// Length of the break that classes of 100 minutes or more get by default.
    pub default_break: Secs,
    /// Warning thresholds used by classes that do not set their own.
    pub warnings: Warnings,
}

impl General {
    pub const MAX_LEAD_IN: Secs = 60 * MINUTE;
    pub const MAX_LINGER: Secs = 300;
    pub const MIN_BREAK: Secs = MINUTE;
    pub const MAX_BREAK: Secs = 30 * MINUTE;
    pub const MAX_WARNING: Secs = 60 * MINUTE;

    /// Why these values cannot be used, if they cannot. What the Settings window offers always
    /// passes; the file itself is read more leniently.
    pub fn validate(&self) -> Result<(), String> {
        if self.lead_in > Self::MAX_LEAD_IN {
            return Err("The lead-in can be at most 60 minutes".to_string());
        }
        if self.linger > Self::MAX_LINGER {
            return Err("The bar can stay at most 300 seconds".to_string());
        }
        if !(Self::MIN_BREAK..=Self::MAX_BREAK).contains(&self.default_break) {
            return Err("The default break must be 1 to 30 minutes".to_string());
        }
        let Warnings { warning, final_ } = self.warnings;
        if warning > Self::MAX_WARNING || final_ > Self::MAX_WARNING {
            return Err("The warning and final times can be at most 60 minutes".to_string());
        }
        Warnings::new(warning, final_)
            .map_err(|_| "Final minutes must not be longer than warning".to_string())?;
        Ok(())
    }
}

impl Config {
    pub fn general(&self) -> General {
        General {
            lead_in: self.schedule.lead_in,
            linger: self.schedule.linger,
            default_break: self.default_break,
            warnings: self.warnings,
        }
    }
}

/// A parsed config plus human-readable messages about everything that was ignored.
#[derive(Clone, Debug)]
pub struct Loaded {
    pub config: Config,
    pub problems: Vec<String>,
}

#[derive(Debug)]
pub enum ConfigError {
    Io(String),
    Parse(String),
    /// The class to update is not in the file (e.g. it was edited meanwhile).
    ClassNotFound,
    /// The edit would produce a timetable that is not valid; nothing was written.
    Invalid(String),
    /// There is no file to write to (it could not be created at startup).
    NoFile,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "cannot write {FILE_NAME}: {e}"),
            Self::Parse(e) => write!(f, "{FILE_NAME} is not valid TOML: {e}"),
            Self::ClassNotFound => write!(f, "that class is not in {FILE_NAME} (any more)"),
            Self::Invalid(e) => write!(f, "{e}"),
            Self::NoFile => write!(f, "there is no settings file to save to"),
        }
    }
}

impl std::error::Error for ConfigError {}

// ---- parsing ----

/// Parse the text of a config file. Never fails: see [`Loaded`].
pub fn parse(text: &str) -> Loaded {
    let doc = match text.parse::<DocumentMut>() {
        Ok(doc) => doc,
        Err(e) => {
            return Loaded {
                config: Config::default(),
                problems: vec![format!(
                    "{FILE_NAME} is not valid TOML, so the defaults are used: {}",
                    e.to_string().trim()
                )],
            };
        }
    };
    let mut p = Problems::default();
    let mut config = Config::default();
    let mut lead_in = DEFAULT_LEAD_IN;
    let mut linger = DEFAULT_LINGER;

    if let Some(t) = p.section(&doc, "general") {
        lead_in = p.secs(t, "general", "lead_in_minutes", MINUTE, lead_in);
        linger = p.secs(t, "general", "linger_seconds", 1, linger);
        config.default_break = p.secs(
            t,
            "general",
            "default_break_minutes",
            MINUTE,
            config.default_break,
        );
        let warning_key = key_of(t, "warning_minutes", "orange_minutes");
        let final_key = key_of(t, "final_minutes", "red_minutes");
        let warning = p.secs(t, "general", warning_key, MINUTE, DEFAULT_WARNING);
        let final_ = p.secs(t, "general", final_key, MINUTE, DEFAULT_FINAL);
        match Warnings::new(warning, final_) {
            Ok(w) => config.warnings = w,
            Err(e) => p.add(format!(
                "[general] {warning_key}/{final_key}: {e}; using defaults"
            )),
        }
    }

    if let Some(t) = p.section(&doc, "colors") {
        let c = &mut config.colors;
        c.lesson = p.color(t, key_of(t, "lesson", "green"), c.lesson);
        c.warning = p.color(t, key_of(t, "warning", "orange"), c.warning);
        c.final_ = p.color(t, key_of(t, "final", "red"), c.final_);
        c.break_gap = p.color(t, "break", c.break_gap);
        c.track = p.color(t, "track", c.track);
        c.text = p.color(t, "text", c.text);
    }

    config.window = p.rect(&doc, "window");
    config.settings_window = p.rect(&doc, "settings_window");

    let mut schedule = Schedule::new();
    schedule.lead_in = lead_in;
    schedule.linger = linger;
    for (name, day) in DAYS {
        let Some(t) = p.section(&doc, name) else {
            continue;
        };
        let Some(classes) = t.get("classes") else {
            continue;
        };
        let mut list = Vec::new();
        for (i, ct) in class_tables(classes).into_iter().enumerate() {
            let at = format!("[{name}] class {}", i + 1);
            if let Some(class) = p.class(ct, &at, &config) {
                list.push(class);
            }
        }
        if let Err(e) = schedule.set_day(day, list) {
            p.add(format!("[{name}]: {e}; no classes are used for this day"));
        }
    }
    config.schedule = schedule;

    Loaded {
        config,
        problems: p.0,
    }
}

#[derive(Default)]
struct Problems(Vec<String>);

impl Problems {
    fn add(&mut self, msg: String) {
        self.0.push(msg);
    }

    fn section<'a>(&mut self, doc: &'a DocumentMut, name: &str) -> Option<&'a dyn TableLike> {
        let item = doc.get(name)?;
        let table = item.as_table_like();
        if table.is_none() {
            self.add(format!("[{name}] must be a table; ignored"));
        }
        table
    }

    /// A number of `unit` seconds (integer or decimal), or `default` when absent or invalid.
    fn secs(
        &mut self,
        t: &dyn TableLike,
        section: &str,
        key: &str,
        unit: Secs,
        default: Secs,
    ) -> Secs {
        match t.get(key) {
            None => default,
            Some(item) => number_secs(item, unit).unwrap_or_else(|| {
                self.add(format!(
                    "[{section}] {key} must be a number of at least 0; using the default"
                ));
                default
            }),
        }
    }

    /// The saved rectangle of a window section; empty when the section is absent.
    fn rect(&mut self, doc: &DocumentMut, name: &str) -> WindowRect {
        let Some(t) = self.section(doc, name) else {
            return WindowRect::default();
        };
        let size =
            |p: &mut Self, key: &str| p.int(t, name, key).and_then(|v| u32::try_from(v).ok());
        WindowRect {
            x: self.int(t, name, "x"),
            y: self.int(t, name, "y"),
            width: size(self, "width"),
            height: size(self, "height"),
        }
    }

    fn int(&mut self, t: &dyn TableLike, section: &str, key: &str) -> Option<i32> {
        let item = t.get(key)?;
        let v = item.as_integer().and_then(|v| i32::try_from(v).ok());
        if v.is_none() {
            self.add(format!("[{section}] {key} must be a whole number; ignored"));
        }
        v
    }

    fn color(&mut self, t: &dyn TableLike, key: &str, default: Rgba) -> Rgba {
        match t.get(key) {
            None => default,
            Some(item) => item.as_str().and_then(Rgba::parse).unwrap_or_else(|| {
                self.add(format!(
                    "[colors] {key} must look like \"#RRGGBB\" or \"#RRGGBBAA\"; using the default"
                ));
                default
            }),
        }
    }

    fn class(&mut self, t: &dyn TableLike, at: &str, config: &Config) -> Option<Class> {
        let time = |key: &str, p: &mut Self| match t.get(key).and_then(Item::as_str) {
            Some(s) => parse_time(s).or_else(|| {
                p.add(format!(
                    "{at}: {key} \"{s}\" is not a time like \"08:30\"; class skipped"
                ));
                None
            }),
            None => {
                p.add(format!("{at}: {key} is missing; class skipped"));
                None
            }
        };
        let start = time("start", self);
        let end = time("end", self);
        let mut class = match Class::new(start?, end?, config.default_break) {
            Ok(c) => c,
            Err(e) => {
                self.add(format!("{at}: {e}; class skipped"));
                return None;
            }
        };
        // Breaks: `breaks = [{ start, len }, ...]` in minutes (`[]` for none). The older
        // break_start / break_len (len = 0 for none) still work. Without any of them the class
        // keeps the default break rule.
        let breaks_item = t.get("breaks");
        let old_start = t.get("break_start");
        let old_len = t.get("break_len");
        let chosen = if breaks_item.is_some() && (old_start.is_some() || old_len.is_some()) {
            self.add(format!(
                "{at}: breaks and break_start/break_len are both set; using the default breaks"
            ));
            None
        } else if let Some(item) = breaks_item {
            match break_list(item) {
                Some(list) => Some(list),
                None => {
                    self.add(format!(
                        "{at}: breaks must be a list like [{{ start = 45, len = 10 }}] in minutes; using the default breaks"
                    ));
                    None
                }
            }
        } else if old_start.is_some() || old_len.is_some() {
            let len = old_len
                .and_then(|i| number_secs(i, MINUTE))
                .unwrap_or(config.default_break);
            Some(if len == 0 {
                Vec::new()
            } else {
                let start = old_start
                    .and_then(|i| number_secs(i, MINUTE))
                    .unwrap_or_else(|| class.duration().saturating_sub(len) / 2);
                vec![Break { start, len }]
            })
        } else {
            None
        };
        if let Some(list) = chosen {
            match class.clone().with_breaks(list) {
                Ok(c) => class = c,
                Err(e) => self.add(format!("{at}: breaks: {e}; using the default breaks")),
            }
        }
        let blocks = class.block_count();
        class.warnings = vec![config.warnings; blocks];

        // Thresholds: one number for every block, or one per block.
        let warning_item = t.get(key_of(t, "warning", "orange"));
        let final_item = t.get(key_of(t, "final", "red"));
        let warning = warning_item.and_then(|i| per_block(i, MINUTE, blocks));
        let final_ = final_item.and_then(|i| per_block(i, MINUTE, blocks));
        if warning_item.is_some() && warning.is_none() || final_item.is_some() && final_.is_none() {
            self.add(format!(
                "{at}: warning/final must be a number or a list with a number for each of the {blocks} block(s), at least 0; using the defaults"
            ));
        }
        for b in 0..blocks {
            let w = Warnings {
                warning: warning.as_ref().map_or(class.warnings[b].warning, |o| o[b]),
                final_: final_.as_ref().map_or(class.warnings[b].final_, |r| r[b]),
            };
            match Warnings::new(w.warning, w.final_) {
                Ok(w) => class.warnings[b] = w,
                Err(e) => self.add(format!(
                    "{at}: warning/final (block {}): {e}; using the defaults",
                    b + 1
                )),
            }
        }
        Some(class)
    }
}

/// Minutes or seconds as `unit`-second counts; integers and decimals, at least 0.
fn number_secs(item: &Item, unit: Secs) -> Option<Secs> {
    let v = item
        .as_integer()
        .map(|i| i as f64)
        .or_else(|| item.as_float())?;
    (v.is_finite() && v >= 0.0 && v * (unit as f64) < f64::from(u32::MAX))
        .then(|| (v * (unit as f64)).round() as Secs)
}

/// A threshold for each of `blocks` blocks, converted to seconds: `5` and `[5]` give the same
/// value for every block, a list gives one value per block. A list of two also fits a class with
/// one block (files written before classes could have several breaks always held two values, and
/// the first is the one that counts).
fn per_block(item: &Item, unit: Secs, blocks: usize) -> Option<Vec<Secs>> {
    if let Some(v) = number_secs(item, unit) {
        return Some(vec![v; blocks]);
    }
    let values = item
        .as_array()?
        .iter()
        .map(|v| number_secs(&Item::Value(v.clone()), unit))
        .collect::<Option<Vec<_>>>()?;
    match values.len() {
        1 => Some(vec![values[0]; blocks]),
        2 if blocks == 1 => Some(vec![values[0]]),
        n if n == blocks => Some(values),
        _ => None,
    }
}

/// `[{ start = 45, len = 10 }, ...]` (minutes from the class start) as breaks.
fn break_list(item: &Item) -> Option<Vec<Break>> {
    let secs = |t: &dyn TableLike, key: &str| number_secs(t.get(key)?, MINUTE);
    let tables: Vec<&dyn TableLike> = match item {
        Item::ArrayOfTables(a) => a.iter().map(|t| t as &dyn TableLike).collect(),
        Item::Value(Value::Array(a)) => a
            .iter()
            .map(|v| v.as_inline_table().map(|t| t as &dyn TableLike))
            .collect::<Option<_>>()?,
        _ => return None,
    };
    tables
        .into_iter()
        .map(|t| {
            Some(Break {
                start: secs(t, "start")?,
                len: secs(t, "len")?,
            })
        })
        .collect()
}

/// `"08:30"` -> seconds since midnight; `"24:00"` is the end of the day.
fn parse_time(s: &str) -> Option<Secs> {
    let (h, m) = s.trim().split_once(':')?;
    let (h, m) = (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?);
    (m < 60 && (h < 24 || (h == 24 && m == 0))).then_some(h * 3600 + m * 60)
}

/// The class entries of a `classes` item, written either as `classes = [{...}]` or `[[day.classes]]`.
fn class_tables(item: &Item) -> Vec<&dyn TableLike> {
    match item {
        Item::ArrayOfTables(a) => a.iter().map(|t| t as &dyn TableLike).collect(),
        Item::Value(Value::Array(a)) => a
            .iter()
            .filter_map(|v| v.as_inline_table().map(|t| t as &dyn TableLike))
            .collect(),
        _ => Vec::new(),
    }
}

fn class_tables_mut(item: &mut Item) -> Vec<&mut dyn TableLike> {
    match item {
        Item::ArrayOfTables(a) => a.iter_mut().map(|t| t as &mut dyn TableLike).collect(),
        Item::Value(Value::Array(a)) => a
            .iter_mut()
            .filter_map(|v| v.as_inline_table_mut().map(|t| t as &mut dyn TableLike))
            .collect(),
        _ => Vec::new(),
    }
}

// ---- editing ----

/// A number of minutes as a TOML value: whole minutes as integers, anything else as a decimal.
fn minutes_value(secs: Secs) -> Value {
    if secs.is_multiple_of(MINUTE) {
        Value::from(i64::from(secs / MINUTE))
    } else {
        Value::from(f64::from(secs) / f64::from(MINUTE))
    }
}

/// The thresholds of the blocks as a list of minutes.
fn list_value(secs: impl Iterator<Item = Secs>) -> Value {
    let mut arr = toml_edit::Array::new();
    for s in secs {
        arr.push(minutes_value(s));
    }
    Value::Array(arr)
}

/// The breaks as `[{ start = 45, len = 10 }, ...]` in minutes.
fn breaks_value(breaks: &[Break]) -> Value {
    let mut arr = toml_edit::Array::new();
    for b in breaks {
        let mut t = toml_edit::InlineTable::new();
        t.insert("start", minutes_value(b.start));
        t.insert("len", minutes_value(b.len));
        arr.push(Value::InlineTable(t));
    }
    Value::Array(arr)
}

fn set_window(doc: &mut DocumentMut, name: &str, rect: &WindowRect) {
    let section = doc.entry(name).or_insert_with(|| Item::Table(Table::new()));
    let Some(t) = section.as_table_like_mut() else {
        *section = Item::Table(Table::new());
        return set_window(doc, name, rect);
    };
    let mut set = |key: &str, v: Option<i64>| {
        if let Some(v) = v {
            t.insert(key, value(v));
        }
    };
    set("x", rect.x.map(i64::from));
    set("y", rect.y.map(i64::from));
    set("width", rect.width.map(i64::from));
    set("height", rect.height.map(i64::from));
}

/// Write the breaks and warning thresholds of `class` into the entry with the same start and end.
fn set_class(doc: &mut DocumentMut, day: Weekday, class: &Class) -> Result<(), ConfigError> {
    let name = DAYS[day.index()].0;
    let classes = doc
        .get_mut(name)
        .and_then(|d| d.as_table_like_mut())
        .and_then(|d| d.get_mut("classes"))
        .ok_or(ConfigError::ClassNotFound)?;
    let entry = class_tables_mut(classes)
        .into_iter()
        .find(|t| {
            let time = |k: &str| t.get(k).and_then(Item::as_str).and_then(parse_time);
            time("start") == Some(class.start) && time("end") == Some(class.end)
        })
        .ok_or(ConfigError::ClassNotFound)?;

    // A file from before classes could have several breaks: its old keys give way to the new one.
    entry.remove("break_start");
    entry.remove("break_len");
    entry.insert("breaks", Item::Value(breaks_value(&class.breaks)));
    // A file from before the zones were renamed: its old keys give way to the new ones.
    entry.remove("orange");
    entry.remove("red");
    let warning = list_value(class.warnings.iter().map(|w| w.warning));
    let final_ = list_value(class.warnings.iter().map(|w| w.final_));
    entry.insert("warning", Item::Value(warning));
    entry.insert("final", Item::Value(final_));
    Ok(())
}

/// Replace the value of `key`, keeping the comment on its line; add it when it is missing.
fn set_value(t: &mut dyn TableLike, key: &str, new: Value) {
    if let Some(Item::Value(old)) = t.get_mut(key) {
        let mut new = new;
        *new.decor_mut() = old.decor().clone();
        *old = new;
    } else {
        t.insert(key, Item::Value(new));
    }
}

/// The key a setting is read from: `key`, or `old_key` in a file from before it was renamed.
/// When both are there the new one wins.
fn key_of<'a>(t: &dyn TableLike, key: &'a str, old_key: &'a str) -> &'a str {
    if t.get(key).is_none() && t.get(old_key).is_some() {
        old_key
    } else {
        key
    }
}

/// Like [`set_value`] under the new name `key`; the old name `old_key` (empty: none) is removed
/// and its value's comment moves to the new key.
fn set_value_renamed(t: &mut dyn TableLike, key: &str, old_key: &str, new: Value) {
    let old = if old_key.is_empty() {
        None
    } else {
        t.remove(old_key)
    };
    let mut new = new;
    if t.get(key).is_none()
        && let Some(Item::Value(old)) = old
    {
        *new.decor_mut() = old.decor().clone();
    }
    set_value(t, key, new);
}

/// The table `name`, made when it is missing (or not a table).
fn section_mut<'a>(doc: &'a mut DocumentMut, name: &str) -> &'a mut dyn TableLike {
    let item = doc.entry(name).or_insert_with(|| Item::Table(Table::new()));
    if item.as_table_like().is_none() {
        *item = Item::Table(Table::new());
    }
    item.as_table_like_mut().expect("a table was just made")
}

fn set_general(doc: &mut DocumentMut, general: &General) {
    let t = section_mut(doc, "general");
    set_value(t, "lead_in_minutes", minutes_value(general.lead_in));
    set_value(t, "linger_seconds", Value::from(i64::from(general.linger)));
    set_value(
        t,
        "default_break_minutes",
        minutes_value(general.default_break),
    );
    set_value_renamed(
        t,
        "warning_minutes",
        "orange_minutes",
        minutes_value(general.warnings.warning),
    );
    set_value_renamed(
        t,
        "final_minutes",
        "red_minutes",
        minutes_value(general.warnings.final_),
    );
}

fn set_colors(doc: &mut DocumentMut, colors: &Colors) {
    let t = section_mut(doc, "colors");
    for (key, old_key, color) in [
        ("lesson", "green", colors.lesson),
        ("warning", "orange", colors.warning),
        ("final", "red", colors.final_),
        ("break", "", colors.break_gap),
        ("track", "", colors.track),
        ("text", "", colors.text),
    ] {
        set_value_renamed(t, key, old_key, Value::from(color.to_hex()));
    }
}

/// `32400` -> `"09:00"`; the end of the day is `"24:00"`.
fn format_time(secs: Secs) -> String {
    let minutes = secs.div_ceil(MINUTE);
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// One class as a one-line inline table. The break and the thresholds are only written when they
/// differ from what the file's defaults would give, so the file stays as short as it was.
fn class_value(class: &Class, default_break: Secs, defaults: Warnings) -> Value {
    let mut t = toml_edit::InlineTable::new();
    t.insert("start", format_time(class.start).into());
    t.insert("end", format_time(class.end).into());
    let standard = Class::new(class.start, class.end, default_break).ok();
    if standard.as_ref().map(|c| &c.breaks) != Some(&class.breaks) {
        t.insert("breaks", breaks_value(&class.breaks));
    }
    if class.warnings.iter().any(|w| *w != defaults) {
        t.insert(
            "warning",
            list_value(class.warnings.iter().map(|w| w.warning)),
        );
        t.insert("final", list_value(class.warnings.iter().map(|w| w.final_)));
    }
    Value::InlineTable(t)
}

/// Replace the `classes` array of `day` by `classes`, one class per line.
fn set_day(
    doc: &mut DocumentMut,
    day: Weekday,
    classes: &[Class],
    default_break: Secs,
    defaults: Warnings,
) {
    let name = DAYS[day.index()].0;
    let mut array = toml_edit::Array::new();
    for class in classes {
        let mut v = class_value(class, default_break, defaults);
        v.decor_mut().set_prefix(
            "
  ",
        );
        array.push_formatted(v);
    }
    if !classes.is_empty() {
        array.set_trailing(
            "
",
        );
    }
    array.set_trailing_comma(!classes.is_empty());

    let section = doc.entry(name).or_insert_with(|| Item::Table(Table::new()));
    if section.as_table_like().is_none() {
        *section = Item::Table(Table::new());
    }
    if let Some(t) = section.as_table_like_mut() {
        t.insert("classes", Item::Value(Value::Array(array)));
    }
}

// ---- the file on disk ----

/// Where the config lives: `times-up.toml` next to the running exe.
pub fn default_path() -> PathBuf {
    std::env::current_exe()
        .map(|exe| exe.with_file_name(FILE_NAME))
        .unwrap_or_else(|_| PathBuf::from(FILE_NAME))
}

/// The config file, with change detection and in-place saving.
pub struct ConfigFile {
    path: PathBuf,
    stamp: Option<(SystemTime, u64)>,
}

impl ConfigFile {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            stamp: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load the file, first creating the commented default when it does not exist. If it cannot
    /// be created or read (read-only folder, no access), the defaults are used and the reason is
    /// in `problems`.
    pub fn load_or_create(&mut self) -> Loaded {
        if !self.path.exists()
            && let Err(e) = fs::write(&self.path, DEFAULT_CONFIG)
        {
            let mut loaded = parse(DEFAULT_CONFIG);
            loaded.problems.push(format!(
                "cannot create {}: {e}; using the built-in defaults and nothing will be saved",
                self.path.display()
            ));
            return loaded;
        }
        self.stamp = self.stat();
        match fs::read_to_string(&self.path) {
            Ok(text) => parse(&text),
            Err(e) => {
                let mut loaded = parse(DEFAULT_CONFIG);
                loaded.problems.push(format!(
                    "cannot read {}: {e}; using the built-in defaults",
                    self.path.display()
                ));
                loaded
            }
        }
    }

    /// Cheap check meant to be called about once a second: returns the new config when the file
    /// changed since it was last loaded or saved, `None` otherwise (also while it is missing).
    pub fn poll(&mut self) -> Option<Loaded> {
        let stamp = self.stat()?;
        if self.stamp == Some(stamp) {
            return None;
        }
        self.stamp = Some(stamp);
        let text = fs::read_to_string(&self.path).ok()?;
        Some(parse(&text))
    }

    pub fn save_window(&mut self, rect: &WindowRect) -> Result<(), ConfigError> {
        self.edit(|doc| {
            set_window(doc, "window", rect);
            Ok(())
        })
    }

    pub fn save_settings_window(&mut self, rect: &WindowRect) -> Result<(), ConfigError> {
        self.edit(|doc| {
            set_window(doc, "settings_window", rect);
            Ok(())
        })
    }

    /// Write the `[general]` times, keeping the comments and everything else in the file.
    pub fn save_general(&mut self, general: &General) -> Result<(), ConfigError> {
        general.validate().map_err(ConfigError::Invalid)?;
        self.edit(|doc| {
            set_general(doc, general);
            Ok(())
        })
    }

    /// Write all six `[colors]`, keeping the comments and everything else in the file.
    pub fn save_colors(&mut self, colors: &Colors) -> Result<(), ConfigError> {
        self.edit(|doc| {
            set_colors(doc, colors);
            Ok(())
        })
    }

    /// Read the file as it is now; it counts as seen, so [`ConfigFile::poll`] stays quiet.
    pub fn reload(&mut self) -> Result<Loaded, ConfigError> {
        let text = fs::read_to_string(&self.path).map_err(|e| ConfigError::Io(e.to_string()))?;
        self.stamp = self.stat();
        Ok(parse(&text))
    }

    /// Store the breaks and warning thresholds of `class` as its defaults (the file entry with the
    /// same start and end), leaving the rest of the file as it was.
    pub fn save_class(&mut self, day: Weekday, class: &Class) -> Result<(), ConfigError> {
        self.edit(|doc| set_class(doc, day, class))
    }

    /// Rewrite the classes of `day` (one per line) and nothing else in the file. `default_break`
    /// and `defaults` are the file's defaults, so only deviations from them are written.
    pub fn save_day(
        &mut self,
        day: Weekday,
        classes: &[Class],
        default_break: Secs,
        defaults: Warnings,
    ) -> Result<(), ConfigError> {
        self.edit(|doc| {
            set_day(doc, day, classes, default_break, defaults);
            Ok(())
        })
    }

    fn stat(&self) -> Option<(SystemTime, u64)> {
        let meta = fs::metadata(&self.path).ok()?;
        Some((meta.modified().ok()?, meta.len()))
    }

    fn edit(
        &mut self,
        f: impl FnOnce(&mut DocumentMut) -> Result<(), ConfigError>,
    ) -> Result<(), ConfigError> {
        let text = fs::read_to_string(&self.path).map_err(|e| ConfigError::Io(e.to_string()))?;
        let mut doc = text
            .parse::<DocumentMut>()
            .map_err(|e| ConfigError::Parse(e.to_string().trim().to_string()))?;
        f(&mut doc)?;
        // Write a sibling file and rename it over the original so a crash cannot leave half a file.
        let tmp = self.path.with_extension("toml.tmp");
        let result = fs::write(&tmp, doc.to_string()).and_then(|()| fs::rename(&tmp, &self.path));
        if let Err(e) = result {
            let _ = fs::remove_file(&tmp);
            return Err(ConfigError::Io(e.to_string()));
        }
        // Our own write must not look like an external edit.
        self.stamp = self.stat();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::{State, Zone, hm};

    /// A scratch directory removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("times-up-test-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn file(&self) -> PathBuf {
            self.0.join(FILE_NAME)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            if let Ok(entries) = fs::read_dir(&self.0) {
                for e in entries.flatten() {
                    if let Ok(meta) = e.metadata() {
                        let mut perm = meta.permissions();
                        #[allow(clippy::permissions_set_readonly_false)]
                        perm.set_readonly(false);
                        let _ = fs::set_permissions(e.path(), perm);
                    }
                }
            }
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn mins(m: u32) -> Secs {
        m * MINUTE
    }

    // ---- defaults and the sample file ----

    #[test]
    fn default_file_parses_without_problems() {
        let loaded = parse(DEFAULT_CONFIG);
        assert_eq!(loaded.problems, Vec::<String>::new());
        let c = loaded.config;
        assert_eq!(c.colors, Colors::default());
        assert_eq!(c.window, WindowRect::default());
        assert_eq!(c.default_break, mins(10));
        assert_eq!(c.warnings, Warnings::default());
        assert_eq!(c.schedule.lead_in, mins(5));
        assert_eq!(c.schedule.linger, 10);
        for day in [
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
        ] {
            assert_eq!(c.schedule.classes(day).len(), 3, "{day:?}");
        }
        assert!(c.schedule.classes(Weekday::Sat).is_empty());
        // The 10:25-12:05 sample is 100 minutes and so has a break.
        assert!(!c.schedule.classes(Weekday::Mon)[2].breaks.is_empty());
        assert!(c.schedule.classes(Weekday::Mon)[0].breaks.is_empty());
    }

    #[test]
    fn empty_file_gives_defaults() {
        let loaded = parse("");
        assert!(loaded.problems.is_empty());
        assert_eq!(loaded.config.colors, Colors::default());
        assert!(loaded.config.schedule.classes(Weekday::Mon).is_empty());
    }

    #[test]
    fn parsed_schedule_drives_the_state() {
        let c = parse(DEFAULT_CONFIG).config;
        assert!(matches!(
            c.schedule.state(Weekday::Mon, hm(9, 0)),
            State::Running {
                class: 0,
                zone: Zone::Lesson,
                ..
            }
        ));
        assert!(matches!(
            c.schedule.state(Weekday::Sat, hm(9, 0)),
            State::Idle { .. }
        ));
    }

    // ---- values ----

    #[test]
    fn settings_are_read() {
        let loaded = parse(
            r##"
[general]
lead_in_minutes = 3
linger_seconds = 20
default_break_minutes = 15
orange_minutes = 8
red_minutes = 2.5

[colors]
lesson = "#00ff00"
break = "#112233cc"

[window]
x = -1200
y = 40
width = 640
height = 72
"##,
        );
        assert_eq!(loaded.problems, Vec::<String>::new());
        let c = loaded.config;
        assert_eq!(c.schedule.lead_in, mins(3));
        assert_eq!(c.schedule.linger, 20);
        assert_eq!(c.default_break, mins(15));
        assert_eq!(
            c.warnings,
            Warnings {
                warning: mins(8),
                final_: 150
            }
        );
        assert_eq!(c.colors.lesson, Rgba::rgb(0, 255, 0));
        assert_eq!(
            c.colors.break_gap,
            Rgba {
                r: 0x11,
                g: 0x22,
                b: 0x33,
                a: 0xcc
            }
        );
        assert_eq!(c.colors.final_, Colors::default().final_);
        assert_eq!(
            c.window,
            WindowRect {
                x: Some(-1200),
                y: Some(40),
                width: Some(640),
                height: Some(72)
            }
        );
    }

    #[test]
    fn colors_parse_strictly() {
        assert_eq!(Rgba::parse("#ff8000"), Some(Rgba::rgb(255, 128, 0)));
        assert_eq!(
            Rgba::parse(" #ff800080 "),
            Some(Rgba {
                r: 255,
                g: 128,
                b: 0,
                a: 128
            })
        );
        for bad in ["ff8000", "#ff80", "#gg0000", "#ff8000f", "", "#é12345"] {
            assert_eq!(Rgba::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn times_parse_strictly() {
        assert_eq!(parse_time("08:30"), Some(hm(8, 30)));
        assert_eq!(parse_time("8:05"), Some(hm(8, 5)));
        assert_eq!(parse_time("24:00"), Some(hm(24, 0)));
        for bad in ["24:01", "12:60", "1230", "ab:cd", "", "-1:00"] {
            assert_eq!(parse_time(bad), None, "{bad}");
        }
    }

    // ---- classes ----

    #[test]
    fn class_options_are_read() {
        let loaded = parse(
            r#"
[monday]
classes = [
  { start = "09:00", end = "10:40", break_start = 40, break_len = 20, warning = [10, 6], final = [5, 3] },
  { start = "11:00", end = "12:40", break_len = 0 },
  { start = "13:00", end = "14:40", warning = 12 },
  { start = "15:00", end = "15:50", final = [4] },
]
"#,
        );
        assert_eq!(loaded.problems, Vec::<String>::new());
        let classes = loaded.config.schedule.classes(Weekday::Mon).to_vec();
        assert_eq!(
            classes[0].breaks,
            vec![Break {
                start: mins(40),
                len: mins(20)
            }]
        );
        assert_eq!(
            classes[0].warnings[0],
            Warnings {
                warning: mins(10),
                final_: mins(5)
            }
        );
        assert_eq!(
            classes[0].warnings[1],
            Warnings {
                warning: mins(6),
                final_: mins(3)
            }
        );
        assert!(
            classes[1].breaks.is_empty(),
            "break_len = 0 removes the default break"
        );
        assert_eq!(
            classes[2].breaks,
            vec![Break {
                start: mins(45),
                len: mins(10)
            }]
        );
        assert_eq!(
            classes[2].warnings,
            [Warnings {
                warning: mins(12),
                final_: mins(5)
            }; 2]
        );
        assert_eq!(
            classes[3].warnings,
            vec![Warnings {
                warning: mins(10),
                final_: mins(4)
            }]
        );
    }

    #[test]
    fn classes_as_array_of_tables_work_too() {
        let loaded = parse(
            r#"
[[tuesday.classes]]
start = "09:00"
end = "09:50"

[[tuesday.classes]]
start = "10:00"
end = "11:40"
"#,
        );
        assert_eq!(loaded.problems, Vec::<String>::new());
        let classes = loaded.config.schedule.classes(Weekday::Tue);
        assert_eq!(classes.len(), 2);
        assert!(!classes[1].breaks.is_empty());
    }

    #[test]
    fn general_thresholds_apply_to_classes_without_their_own() {
        let loaded = parse(
            r#"
[general]
orange_minutes = 7
red_minutes = 3
[monday]
classes = [{ start = "09:00", end = "09:50" }]
"#,
        );
        let class = &loaded.config.schedule.classes(Weekday::Mon)[0];
        assert_eq!(
            class.warnings,
            vec![Warnings {
                warning: mins(7),
                final_: mins(3)
            }]
        );
    }

    // ---- bad input falls back with a message ----

    #[test]
    fn invalid_toml_falls_back_to_defaults() {
        let loaded = parse("[general\nlead_in_minutes = ");
        assert_eq!(loaded.problems.len(), 1);
        assert!(
            loaded.problems[0].contains("not valid TOML"),
            "{:?}",
            loaded.problems
        );
        assert!(loaded.config.schedule.classes(Weekday::Mon).is_empty());
    }

    #[test]
    fn bad_values_are_reported_and_defaulted() {
        let loaded = parse(
            r##"
[general]
lead_in_minutes = "soon"
linger_seconds = -4
orange_minutes = 3
red_minutes = 9

[colors]
lesson = "lime"
final = 5

[window]
x = "left"
width = -5
"##,
        );
        let text = loaded.problems.join("\n");
        for needle in [
            "lead_in_minutes",
            "linger_seconds",
            "orange_minutes/red_minutes",
            "[colors] lesson",
            "[colors] final",
            "[window] x",
        ] {
            assert!(text.contains(needle), "missing {needle:?} in:\n{text}");
        }
        let c = loaded.config;
        assert_eq!(c.schedule.lead_in, DEFAULT_LEAD_IN);
        assert_eq!(c.schedule.linger, DEFAULT_LINGER);
        assert_eq!(c.warnings, Warnings::default());
        assert_eq!(c.colors, Colors::default());
        assert_eq!(c.window, WindowRect::default());
    }

    #[test]
    fn bad_classes_are_skipped_and_the_rest_is_kept() {
        let loaded = parse(
            r#"
[monday]
classes = [
  { start = "09:00", end = "09:50" },
  { start = "9am", end = "10:50" },
  { start = "11:00" },
  { start = "12:00", end = "11:00" },
  { start = "13:00", end = "14:40", break_start = 99, break_len = 10 },
  { start = "15:00", end = "15:50", warning = "lots" },
  { start = "16:00", end = "16:50", warning = 1, final = 2 },
]
"#,
        );
        let text = loaded.problems.join("\n");
        assert!(text.contains("class 2") && text.contains("9am"), "{text}");
        assert!(
            text.contains("class 3") && text.contains("end is missing"),
            "{text}"
        );
        assert!(text.contains("class 4"), "{text}");
        assert!(text.contains("class 5") && text.contains("break"), "{text}");
        assert!(
            text.contains("class 6") && text.contains("warning/final"),
            "{text}"
        );
        assert!(text.contains("class 7"), "{text}");
        let classes = loaded.config.schedule.classes(Weekday::Mon);
        // Classes 1, 5 (with the default break), 6 and 7 (with default thresholds) survive.
        assert_eq!(classes.len(), 4);
        assert_eq!(
            classes[1].breaks,
            vec![Break {
                start: mins(45),
                len: mins(10)
            }]
        );
        assert_eq!(classes[3].warnings, vec![Warnings::default()]);
    }

    #[test]
    fn overlapping_day_is_dropped_with_a_message() {
        let loaded = parse(
            r#"
[friday]
classes = [{ start = "09:00", end = "09:50" }, { start = "09:30", end = "10:20" }]
[monday]
classes = [{ start = "09:00", end = "09:50" }]
"#,
        );
        assert!(
            loaded
                .problems
                .iter()
                .any(|p| p.contains("[friday]") && p.contains("overlap"))
        );
        assert!(loaded.config.schedule.classes(Weekday::Fri).is_empty());
        assert_eq!(loaded.config.schedule.classes(Weekday::Mon).len(), 1);
    }

    #[test]
    fn section_of_the_wrong_type_is_reported() {
        let loaded = parse("colors = 3\ngeneral = \"x\"");
        assert_eq!(loaded.problems.len(), 2);
        assert_eq!(loaded.config.colors, Colors::default());
    }

    // ---- the file on disk ----

    #[test]
    fn missing_file_is_created_with_the_commented_default() {
        let dir = TempDir::new("create");
        let mut file = ConfigFile::new(dir.file());
        let loaded = file.load_or_create();
        assert!(loaded.problems.is_empty());
        assert_eq!(fs::read_to_string(dir.file()).unwrap(), DEFAULT_CONFIG);
        assert_eq!(loaded.config.schedule.classes(Weekday::Mon).len(), 3);
        // An existing file is left alone.
        fs::write(dir.file(), "[general]\nlead_in_minutes = 1\n").unwrap();
        let loaded = ConfigFile::new(dir.file()).load_or_create();
        assert_eq!(loaded.config.schedule.lead_in, mins(1));
        assert_eq!(
            fs::read_to_string(dir.file()).unwrap(),
            "[general]\nlead_in_minutes = 1\n"
        );
    }

    #[test]
    fn unwritable_location_uses_defaults_and_does_not_panic() {
        let dir = TempDir::new("nodir");
        let mut file = ConfigFile::new(dir.0.join("no-such-folder").join(FILE_NAME));
        let loaded = file.load_or_create();
        assert!(
            loaded.problems.iter().any(|p| p.contains("cannot create")),
            "{:?}",
            loaded.problems
        );
        assert_eq!(loaded.config.schedule.classes(Weekday::Mon).len(), 3);
        assert!(file.poll().is_none());
        assert!(matches!(
            file.save_window(&WindowRect::default()),
            Err(ConfigError::Io(_))
        ));
    }

    #[test]
    fn read_only_file_loads_but_save_fails_cleanly() {
        let dir = TempDir::new("readonly");
        fs::write(dir.file(), DEFAULT_CONFIG).unwrap();
        let mut perm = fs::metadata(dir.file()).unwrap().permissions();
        perm.set_readonly(true);
        fs::set_permissions(dir.file(), perm).unwrap();

        let mut file = ConfigFile::new(dir.file());
        let loaded = file.load_or_create();
        assert!(loaded.problems.is_empty());
        let rect = WindowRect {
            x: Some(1),
            y: Some(2),
            width: Some(300),
            height: Some(40),
        };
        let err = file.save_window(&rect).unwrap_err();
        assert!(err.to_string().contains("cannot write"), "{err}");
        assert_eq!(fs::read_to_string(dir.file()).unwrap(), DEFAULT_CONFIG);
        assert!(
            !dir.0.join("times-up.toml.tmp").exists(),
            "temp file left behind"
        );
    }

    #[test]
    fn poll_notices_external_edits_only() {
        let dir = TempDir::new("poll");
        let mut file = ConfigFile::new(dir.file());
        file.load_or_create();
        assert!(file.poll().is_none(), "nothing changed yet");

        fs::write(
            dir.file(),
            "[general]\nlead_in_minutes = 2\n# edited by hand\n",
        )
        .unwrap();
        let loaded = file.poll().expect("edit should be noticed");
        assert_eq!(loaded.config.schedule.lead_in, mins(2));
        assert!(file.poll().is_none(), "the same edit is reported once");

        // A broken edit is reported as problems, not as a failure.
        fs::write(dir.file(), "[general\n").unwrap();
        let loaded = file.poll().expect("edit should be noticed");
        assert_eq!(loaded.problems.len(), 1);

        // A vanished file is ignored; the caller keeps the config it has.
        fs::remove_file(dir.file()).unwrap();
        assert!(file.poll().is_none());
    }

    #[test]
    fn saving_window_keeps_comments_and_does_not_look_like_an_edit() {
        let dir = TempDir::new("window");
        let mut file = ConfigFile::new(dir.file());
        file.load_or_create();
        let rect = WindowRect {
            x: Some(-30),
            y: Some(15),
            width: Some(700),
            height: Some(80),
        };
        file.save_window(&rect).unwrap();
        assert!(
            file.poll().is_none(),
            "our own write must not trigger a reload"
        );

        let text = fs::read_to_string(dir.file()).unwrap();
        assert!(
            text.contains("# Time's up settings."),
            "header comment lost"
        );
        assert!(
            text.contains("# the bar appears this long before a class starts"),
            "inline comment lost"
        );
        assert!(text.contains("# One section per weekday"), "comment lost");
        let reloaded = parse(&text);
        assert_eq!(reloaded.problems, Vec::<String>::new());
        assert_eq!(reloaded.config.window, rect);
        assert_eq!(reloaded.config.schedule.classes(Weekday::Mon).len(), 3);

        // Saving again updates the same keys.
        let rect2 = WindowRect {
            width: Some(900),
            ..rect
        };
        file.save_window(&rect2).unwrap();
        let text = fs::read_to_string(dir.file()).unwrap();
        assert_eq!(
            text.matches("width = ").count(),
            3,
            "one active key and the commented hints of both windows:\n{text}"
        );
        assert_eq!(parse(&text).config.window, rect2);
    }

    #[test]
    fn the_settings_window_has_its_own_section() {
        let dir = TempDir::new("settings-window");
        let mut file = ConfigFile::new(dir.file());
        file.load_or_create();
        let bar = WindowRect {
            x: Some(1),
            y: Some(2),
            width: Some(600),
            height: Some(60),
        };
        let settings = WindowRect {
            x: Some(-20),
            y: Some(30),
            width: Some(900),
            height: Some(500),
        };
        file.save_window(&bar).unwrap();
        file.save_settings_window(&settings).unwrap();
        assert!(file.poll().is_none(), "our own write is not an edit");
        let loaded = parse(&fs::read_to_string(dir.file()).unwrap());
        assert_eq!(loaded.problems, Vec::<String>::new());
        assert_eq!(loaded.config.window, bar);
        assert_eq!(loaded.config.settings_window, settings);
        assert_eq!(
            parse(
                "[settings_window]
width = -5
"
            )
            .config
            .settings_window,
            WindowRect::default()
        );
    }

    #[test]
    fn saving_window_creates_the_section_when_missing() {
        let dir = TempDir::new("window-new");
        fs::write(dir.file(), "# mine\n[general]\nlead_in_minutes = 4\n").unwrap();
        let mut file = ConfigFile::new(dir.file());
        file.load_or_create();
        let rect = WindowRect {
            x: Some(5),
            y: Some(6),
            width: Some(7),
            height: Some(8),
        };
        file.save_window(&rect).unwrap();
        let text = fs::read_to_string(dir.file()).unwrap();
        assert!(text.starts_with("# mine"));
        let c = parse(&text).config;
        assert_eq!(c.window, rect);
        assert_eq!(c.schedule.lead_in, mins(4));
    }

    #[test]
    fn saving_a_class_default_round_trips_and_keeps_the_rest() {
        let dir = TempDir::new("class");
        let mut file = ConfigFile::new(dir.file());
        let mut c = file.load_or_create().config;

        // Monday's 10:25-12:05 class: move the break and change the second block's warnings.
        let mut class = c.schedule.classes(Weekday::Mon)[2].clone();
        class.breaks = vec![Break {
            start: mins(30),
            len: mins(15),
        }];
        class.warnings[1] = Warnings {
            warning: mins(7),
            final_: mins(2),
        };
        file.save_class(Weekday::Mon, &class).unwrap();
        assert!(file.poll().is_none());

        let text = fs::read_to_string(dir.file()).unwrap();
        assert!(text.contains("# Time's up settings."));
        assert!(text.contains("# One section per weekday"));
        c = parse(&text).config;
        assert_eq!(c.schedule.classes(Weekday::Mon)[2], class);
        // Other classes and other days are untouched.
        assert!(c.schedule.classes(Weekday::Mon)[0].breaks.is_empty());
        assert_eq!(
            c.schedule.classes(Weekday::Tue)[2].breaks,
            parse(DEFAULT_CONFIG).config.schedule.classes(Weekday::Tue)[2].breaks
        );

        // Removing the break writes break_len = 0 and drops break_start.
        class.breaks.clear();
        class.warnings.truncate(1);
        file.save_class(Weekday::Mon, &class).unwrap();
        let text = fs::read_to_string(dir.file()).unwrap();
        let again = parse(&text);
        assert_eq!(again.problems, Vec::<String>::new());
        assert!(
            again.config.schedule.classes(Weekday::Mon)[2]
                .breaks
                .is_empty()
        );
    }

    // ---- files from before the zones were renamed ----

    const OLD_KEYS: &str = r##"[general]
orange_minutes = 8
red_minutes = 2

[colors]
green = "#010203"
orange = "#040506"
red = "#070809"

[monday]
classes = [
  { start = "09:00", end = "09:50", orange = 7, red = 3 },
]
"##;

    #[test]
    fn the_old_keys_are_still_read() {
        let loaded = parse(OLD_KEYS);
        assert_eq!(loaded.problems, Vec::<String>::new());
        let c = loaded.config;
        assert_eq!(c.colors.lesson, Rgba::rgb(1, 2, 3));
        assert_eq!(c.colors.warning, Rgba::rgb(4, 5, 6));
        assert_eq!(c.colors.final_, Rgba::rgb(7, 8, 9));
        assert_eq!((c.warnings.warning, c.warnings.final_), (mins(8), mins(2)));
        let w = c.schedule.classes(Weekday::Mon)[0].warnings[0];
        assert_eq!((w.warning, w.final_), (mins(7), mins(3)));
    }

    #[test]
    fn a_new_key_wins_over_the_old_one() {
        let loaded = parse(
            "[general]\norange_minutes = 8\nwarning_minutes = 9\n\n[colors]\ngreen = \"#010203\"\nlesson = \"#0a0b0c\"\n\n[monday]\nclasses = [{ start = \"09:00\", end = \"09:50\", orange = 7, warning = 6 }]\n",
        );
        assert_eq!(loaded.problems, Vec::<String>::new());
        assert_eq!(loaded.config.warnings.warning, mins(9));
        assert_eq!(loaded.config.colors.lesson, Rgba::rgb(0x0a, 0x0b, 0x0c));
        let w = loaded.config.schedule.classes(Weekday::Mon)[0].warnings[0];
        assert_eq!(w.warning, mins(6));
    }

    #[test]
    fn saving_a_class_into_an_old_file_writes_the_new_keys() {
        let dir = TempDir::new("old-class");
        fs::write(dir.file(), OLD_KEYS).unwrap();
        let mut file = ConfigFile::new(dir.file());
        let c = file.load_or_create().config;
        let mut class = c.schedule.classes(Weekday::Mon)[0].clone();
        class.warnings[0] = Warnings {
            warning: mins(6),
            final_: mins(1),
        };
        file.save_class(Weekday::Mon, &class).unwrap();

        let text = fs::read_to_string(dir.file()).unwrap();
        assert!(
            text.contains("warning = [6]") && text.contains("final = [1]"),
            "{text}"
        );
        assert!(
            !text.contains("orange = 7") && !text.contains("red = 3"),
            "{text}"
        );
        // What the app did not write stays as it was.
        assert!(text.contains("green = \"#010203\""));
        assert!(text.contains("orange_minutes = 8"));
        assert_eq!(parse(&text).config.schedule.classes(Weekday::Mon)[0], class);
    }

    #[test]
    fn saving_a_class_works_for_array_of_tables_and_decimal_minutes() {
        let dir = TempDir::new("class-aot");
        fs::write(
            dir.file(),
            "[[tuesday.classes]]\n# my first class\nstart = \"09:00\"\nend = \"10:40\"\n",
        )
        .unwrap();
        let mut file = ConfigFile::new(dir.file());
        let c = file.load_or_create().config;
        let mut class = c.schedule.classes(Weekday::Tue)[0].clone();
        class.breaks = vec![Break {
            start: 2700 + 30,
            len: 630,
        }];
        file.save_class(Weekday::Tue, &class).unwrap();
        let text = fs::read_to_string(dir.file()).unwrap();
        assert!(text.contains("# my first class"));
        assert_eq!(parse(&text).config.schedule.classes(Weekday::Tue)[0], class);
    }

    #[test]
    fn saving_an_unknown_class_is_an_error() {
        let dir = TempDir::new("class-missing");
        let mut file = ConfigFile::new(dir.file());
        let c = file.load_or_create().config;
        let mut class = c.schedule.classes(Weekday::Mon)[0].clone();
        class.start += MINUTE;
        assert!(matches!(
            file.save_class(Weekday::Mon, &class),
            Err(ConfigError::ClassNotFound)
        ));
        assert!(matches!(
            file.save_class(Weekday::Sat, &class),
            Err(ConfigError::ClassNotFound)
        ));
    }

    #[test]
    fn saving_into_a_broken_file_reports_the_parse_error() {
        let dir = TempDir::new("broken");
        fs::write(dir.file(), "[general\n").unwrap();
        let mut file = ConfigFile::new(dir.file());
        file.load_or_create();
        let err = file.save_window(&WindowRect::default()).unwrap_err();
        assert!(matches!(err, ConfigError::Parse(_)), "{err}");
        assert_eq!(fs::read_to_string(dir.file()).unwrap(), "[general\n");
    }

    #[test]
    fn default_path_is_next_to_the_exe() {
        let p = default_path();
        assert_eq!(p.file_name().unwrap(), FILE_NAME);
        assert_eq!(p.parent(), std::env::current_exe().unwrap().parent());
    }

    // ---- several breaks ----

    fn lab_class(extra: &str) -> Loaded {
        parse(&format!(
            "[monday]\nclasses = [{{ start = \"09:00\", end = \"12:00\"{extra} }}]\n"
        ))
    }

    #[test]
    fn a_breaks_list_gives_a_class_any_number_of_breaks() {
        let loaded = lab_class(", breaks = [{ start = 50, len = 10 }, { start = 110, len = 10 }]");
        assert_eq!(loaded.problems, Vec::<String>::new());
        let class = &loaded.config.schedule.classes(Weekday::Mon)[0];
        assert_eq!(
            class.breaks,
            vec![
                Break {
                    start: mins(50),
                    len: mins(10)
                },
                Break {
                    start: mins(110),
                    len: mins(10)
                }
            ]
        );
        assert_eq!(class.block_count(), 3);
        assert_eq!(class.warnings, vec![Warnings::default(); 3]);
    }

    #[test]
    fn an_empty_breaks_list_means_no_breaks_and_a_missing_one_the_default() {
        let none = lab_class(", breaks = []");
        assert_eq!(none.problems, Vec::<String>::new());
        assert!(
            none.config.schedule.classes(Weekday::Mon)[0]
                .breaks
                .is_empty()
        );
        let default = lab_class("");
        assert_eq!(
            default.config.schedule.classes(Weekday::Mon)[0]
                .breaks
                .len(),
            1
        );
    }

    #[test]
    fn old_and_new_break_keys_together_give_the_default_breaks_and_a_message() {
        let loaded = lab_class(", breaks = [], break_len = 5");
        assert_eq!(loaded.problems.len(), 1, "{:?}", loaded.problems);
        assert!(
            loaded.problems[0].contains("both set"),
            "{:?}",
            loaded.problems
        );
        let class = &loaded.config.schedule.classes(Weekday::Mon)[0];
        assert_eq!(class.breaks.len(), 1, "the default break");
    }

    #[test]
    fn breaks_that_do_not_fit_or_are_not_a_list_give_the_default_breaks_and_a_message() {
        for extra in [
            ", breaks = [{ start = 50, len = 10 }, { start = 55, len = 10 }]",
            ", breaks = [{ start = 175, len = 10 }]",
            ", breaks = 5",
            ", breaks = [{ start = 50 }]",
            ", breaks = [3]",
        ] {
            let loaded = lab_class(extra);
            assert_eq!(loaded.problems.len(), 1, "{extra}: {:?}", loaded.problems);
            let class = &loaded.config.schedule.classes(Weekday::Mon)[0];
            assert_eq!(class.breaks.len(), 1, "{extra}: the default break");
        }
    }

    #[test]
    fn thresholds_are_given_per_block() {
        let loaded = lab_class(
            ", breaks = [{ start = 50, len = 10 }, { start = 110, len = 10 }], warning = [10, 8, 6], final = [5, 4, 3]",
        );
        assert_eq!(loaded.problems, Vec::<String>::new());
        let w = &loaded.config.schedule.classes(Weekday::Mon)[0].warnings;
        let pair = |a, b| Warnings {
            warning: mins(a),
            final_: mins(b),
        };
        assert_eq!(*w, vec![pair(10, 5), pair(8, 4), pair(6, 3)]);

        // One number, or a list of one, goes for every block.
        let loaded = lab_class(
            ", breaks = [{ start = 50, len = 10 }, { start = 110, len = 10 }], warning = 7, final = [2]",
        );
        assert_eq!(loaded.problems, Vec::<String>::new());
        let w = &loaded.config.schedule.classes(Weekday::Mon)[0].warnings;
        assert_eq!(*w, vec![pair(7, 2); 3]);
    }

    #[test]
    fn a_threshold_list_of_the_wrong_length_gives_the_defaults_and_a_message() {
        let loaded = lab_class(
            ", breaks = [{ start = 50, len = 10 }, { start = 110, len = 10 }], warning = [10, 8], final = [5, 4]",
        );
        assert_eq!(loaded.problems.len(), 1, "{:?}", loaded.problems);
        let w = &loaded.config.schedule.classes(Weekday::Mon)[0].warnings;
        assert_eq!(*w, vec![Warnings::default(); 3]);
    }

    #[test]
    fn a_pair_of_thresholds_from_before_several_breaks_still_loads() {
        // Two values on a class with the default break (two blocks) and on one without a break.
        let loaded = parse(
            "[monday]\nclasses = [{ start = \"09:00\", end = \"10:40\", warning = [10, 8], final = [5, 4] }, { start = \"11:00\", end = \"11:50\", warning = [9, 8], final = [3, 2] }]\n",
        );
        assert_eq!(loaded.problems, Vec::<String>::new());
        let classes = loaded.config.schedule.classes(Weekday::Mon);
        assert_eq!(classes[0].warnings[1].warning, mins(8));
        assert_eq!(classes[1].warnings.len(), 1);
        assert_eq!(classes[1].warnings[0].warning, mins(9));
    }

    #[test]
    fn saving_a_class_with_several_breaks_writes_the_list_and_removes_the_old_keys() {
        let dir = TempDir::new("several-breaks");
        fs::write(
            dir.file(),
            "[monday]\nclasses = [{ start = \"09:00\", end = \"12:00\", break_start = 40, break_len = 20 }]\n",
        )
        .unwrap();
        let mut file = ConfigFile::new(dir.file());
        let mut class = file.load_or_create().config.schedule.classes(Weekday::Mon)[0].clone();
        class = class
            .with_breaks(vec![
                Break {
                    start: mins(50),
                    len: mins(10),
                },
                Break {
                    start: mins(110),
                    len: mins(10),
                },
            ])
            .unwrap();
        class.warnings[2] = Warnings {
            warning: mins(4),
            final_: mins(1),
        };
        file.save_class(Weekday::Mon, &class).unwrap();

        let text = fs::read_to_string(dir.file()).unwrap();
        assert!(
            text.contains("breaks = [{ start = 50, len = 10 }, { start = 110, len = 10 }]"),
            "{text}"
        );
        assert!(
            !text.contains("break_start") && !text.contains("break_len"),
            "{text}"
        );
        assert!(
            text.contains("warning = [10, 10, 4]") && text.contains("final = [5, 5, 1]"),
            "{text}"
        );
        let loaded = parse(&text);
        assert_eq!(loaded.problems, Vec::<String>::new());
        assert_eq!(loaded.config.schedule.classes(Weekday::Mon)[0], class);
    }

    #[test]
    fn saving_a_day_writes_breaks_only_when_they_differ_from_the_default_rule() {
        let dir = TempDir::new("day-breaks");
        fs::write(dir.file(), "[monday]\nclasses = []\n").unwrap();
        let mut file = ConfigFile::new(dir.file());
        let standard = Class::new(hm(9, 0), hm(10, 40), DEFAULT_BREAK_LEN).unwrap();
        let three = Class::new(hm(11, 0), hm(14, 0), DEFAULT_BREAK_LEN)
            .unwrap()
            .with_breaks(vec![
                Break {
                    start: mins(50),
                    len: mins(10),
                },
                Break {
                    start: mins(110),
                    len: mins(10),
                },
            ])
            .unwrap();
        let none = Class::new(hm(15, 0), hm(17, 0), DEFAULT_BREAK_LEN)
            .unwrap()
            .with_breaks(Vec::new())
            .unwrap();
        let classes = vec![standard, three, none];
        file.save_day(
            Weekday::Mon,
            &classes,
            DEFAULT_BREAK_LEN,
            Warnings::default(),
        )
        .unwrap();
        let text = fs::read_to_string(dir.file()).unwrap();
        assert_eq!(text.matches("breaks =").count(), 2, "{text}");
        assert!(text.contains("breaks = []"), "{text}");
        let loaded = parse(&text);
        assert_eq!(loaded.problems, Vec::<String>::new());
        assert_eq!(loaded.config.schedule.classes(Weekday::Mon), &classes[..]);
    }
}
