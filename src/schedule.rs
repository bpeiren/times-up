//! Pure schedule logic: classes, breaks, warning zones and what to show at a given moment.
//!
//! Times are whole seconds. A time of day is seconds since midnight; offsets inside a class are
//! seconds since the class start. Classes are half-open intervals `[start, end)`, so a class that
//! ends exactly when the next one starts hands over without a gap.

use std::fmt;

pub type Secs = u32;

pub const MINUTE: Secs = 60;
pub const DEFAULT_WARNING: Secs = 10 * MINUTE;
pub const DEFAULT_FINAL: Secs = 5 * MINUTE;
pub const DEFAULT_BREAK_LEN: Secs = 10 * MINUTE;
pub const DEFAULT_LEAD_IN: Secs = 5 * MINUTE;
pub const DEFAULT_LINGER: Secs = 10;
/// Classes at least this long get a default break in the middle.
pub const TWO_HOUR_MIN_LEN: Secs = 100 * MINUTE;

/// Seconds since midnight for `h:m`.
pub const fn hm(h: u32, m: u32) -> Secs {
    h * 3600 + m * MINUTE
}

/// A time of day as `HH:MM` (seconds are dropped).
pub fn format_clock(secs: Secs) -> String {
    format!("{:02}:{:02}", secs / 3600, secs % 3600 / 60)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Weekday {
    pub const ALL: [Weekday; 7] = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];

    pub const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Lesson,
    Warning,
    Final,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScheduleError {
    /// A class must end after it starts.
    EmptyClass,
    /// A break must leave a non-empty block on both sides.
    BreakOutsideClass,
    /// The final zone must not be longer than the warning zone.
    FinalLongerThanWarning,
    /// Two classes overlap (back-to-back is fine).
    Overlap,
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EmptyClass => "a class must end after it starts",
            Self::BreakOutsideClass => "a break must lie strictly inside the class",
            Self::FinalLongerThanWarning => {
                "the final zone must not be longer than the warning zone"
            }
            Self::Overlap => "classes must not overlap",
        })
    }
}

impl std::error::Error for ScheduleError {}

/// How long before the end of a block it changes to the warning and final zones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Warnings {
    pub warning: Secs,
    pub final_: Secs,
}

impl Default for Warnings {
    fn default() -> Self {
        Self {
            warning: DEFAULT_WARNING,
            final_: DEFAULT_FINAL,
        }
    }
}

impl Warnings {
    pub fn new(warning: Secs, final_: Secs) -> Result<Self, ScheduleError> {
        if final_ > warning {
            return Err(ScheduleError::FinalLongerThanWarning);
        }
        Ok(Self { warning, final_ })
    }

    /// Zone for `left` seconds remaining in a block. The thresholds are inclusive: with exactly
    /// ten minutes left the block is already in the warning zone.
    pub fn zone(&self, left: Secs) -> Zone {
        if left <= self.final_ {
            Zone::Final
        } else if left <= self.warning {
            Zone::Warning
        } else {
            Zone::Lesson
        }
    }
}

/// A break inside a class, as an offset from the class start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Break {
    pub start: Secs,
    pub len: Secs,
}

impl Break {
    pub const fn end(&self) -> Secs {
        self.start + self.len
    }
}

/// A teaching block, as offsets from the class start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Block {
    pub start: Secs,
    pub end: Secs,
}

impl Block {
    pub const fn len(&self) -> Secs {
        self.end - self.start
    }

    pub const fn is_empty(&self) -> bool {
        self.end == self.start
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Class {
    pub start: Secs,
    pub end: Secs,
    pub brk: Option<Break>,
    /// Warning thresholds per block (the second entry is only used when there is a break).
    pub warnings: [Warnings; 2],
}

impl Class {
    /// A class with the default break rule: classes of 100 minutes or more get a centered break
    /// of `default_break_len`, shorter classes get none.
    pub fn new(start: Secs, end: Secs, default_break_len: Secs) -> Result<Self, ScheduleError> {
        let class = Self {
            start,
            end,
            brk: None,
            warnings: [Warnings::default(); 2],
        };
        let duration = class.duration();
        if end <= start || duration < TWO_HOUR_MIN_LEN {
            return class.with_break(None);
        }
        let brk = Break {
            start: (duration.saturating_sub(default_break_len)) / 2,
            len: default_break_len,
        };
        class.with_break(Some(brk))
    }

    /// Replace the break (`None` removes it).
    pub fn with_break(mut self, brk: Option<Break>) -> Result<Self, ScheduleError> {
        self.brk = brk;
        self.validate()?;
        Ok(self)
    }

    /// Replace the warning thresholds of one block.
    pub fn with_warnings(
        mut self,
        block: usize,
        warnings: Warnings,
    ) -> Result<Self, ScheduleError> {
        self.warnings[block.min(1)] = Warnings::new(warnings.warning, warnings.final_)?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), ScheduleError> {
        if self.end <= self.start {
            return Err(ScheduleError::EmptyClass);
        }
        if let Some(b) = self.brk
            && (b.len == 0 || b.start == 0 || b.end() >= self.duration())
        {
            return Err(ScheduleError::BreakOutsideClass);
        }
        Ok(())
    }

    /// Length of the class; zero for an (invalid) class that ends before it starts.
    pub const fn duration(&self) -> Secs {
        self.end.saturating_sub(self.start)
    }

    pub const fn block_count(&self) -> usize {
        if self.brk.is_some() { 2 } else { 1 }
    }

    /// Block `index` (0 or 1) as offsets from the class start.
    pub fn block(&self, index: usize) -> Block {
        match (self.brk, index) {
            (None, _) => Block {
                start: 0,
                end: self.duration(),
            },
            (Some(b), 0) => Block {
                start: 0,
                end: b.start,
            },
            (Some(b), _) => Block {
                start: b.end(),
                end: self.duration(),
            },
        }
    }
}

/// What the window should show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Nothing to show. `next_start_in` is the time until the next class today, if any.
    Idle { next_start_in: Option<Secs> },
    /// Shortly before class `class`.
    LeadIn { class: usize, starts_in: Secs },
    /// Inside a teaching block: `elapsed` since the class start, `left` until the block ends.
    Running {
        class: usize,
        elapsed: Secs,
        block: usize,
        left: Secs,
        zone: Zone,
    },
    /// Inside the break: `left` until it ends.
    Break {
        class: usize,
        elapsed: Secs,
        left: Secs,
    },
    /// Class `class` has just finished.
    Ended { class: usize },
}

impl State {
    pub const fn is_visible(&self) -> bool {
        !matches!(self, Self::Idle { .. })
    }
}

#[derive(Clone, Debug)]
pub struct Schedule {
    days: [Vec<Class>; 7],
    /// How long before a class the window appears.
    pub lead_in: Secs,
    /// How long the window stays after the last class ends.
    pub linger: Secs,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            days: Default::default(),
            lead_in: DEFAULT_LEAD_IN,
            linger: DEFAULT_LINGER,
        }
    }
}

impl Schedule {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the classes of a day. They are sorted by start time and must not overlap.
    pub fn set_day(&mut self, day: Weekday, mut classes: Vec<Class>) -> Result<(), ScheduleError> {
        for c in &classes {
            c.validate()?;
        }
        classes.sort_by_key(|c| c.start);
        if classes.windows(2).any(|w| w[1].start < w[0].end) {
            return Err(ScheduleError::Overlap);
        }
        self.days[day.index()] = classes;
        Ok(())
    }

    pub fn classes(&self, day: Weekday) -> &[Class] {
        &self.days[day.index()]
    }

    /// Mutable access for per-class edits; the caller keeps the classes valid and ordered.
    pub fn class_mut(&mut self, day: Weekday, index: usize) -> Option<&mut Class> {
        self.days[day.index()].get_mut(index)
    }

    /// The class the bar is about at `now` (counting from the lead-in until it has just ended).
    pub fn current_class(&self, day: Weekday, now: Secs) -> Option<usize> {
        match self.state(day, now) {
            State::LeadIn { class, .. }
            | State::Running { class, .. }
            | State::Break { class, .. }
            | State::Ended { class } => Some(class),
            State::Idle { .. } => None,
        }
    }

    /// The state at `now` (seconds since midnight) on `day`.
    ///
    /// Priority: a running class or break, then a class that just ended, then the lead-in of the
    /// next class, then idle.
    pub fn state(&self, day: Weekday, now: Secs) -> State {
        let classes = self.classes(day);

        if let Some((i, c)) = classes
            .iter()
            .enumerate()
            .find(|(_, c)| c.start <= now && now < c.end)
        {
            return Self::inside(i, c, now - c.start);
        }

        if let Some(i) = classes
            .iter()
            .position(|c| c.end <= now && now < c.end + self.linger)
        {
            return State::Ended { class: i };
        }

        match classes.iter().enumerate().find(|(_, c)| c.start > now) {
            Some((i, c)) if c.start - now <= self.lead_in => State::LeadIn {
                class: i,
                starts_in: c.start - now,
            },
            Some((_, c)) => State::Idle {
                next_start_in: Some(c.start - now),
            },
            None => State::Idle {
                next_start_in: None,
            },
        }
    }

    fn inside(index: usize, c: &Class, elapsed: Secs) -> State {
        if let Some(b) = c.brk
            && b.start <= elapsed
            && elapsed < b.end()
        {
            return State::Break {
                class: index,
                elapsed,
                left: b.end() - elapsed,
            };
        }
        let block = match c.brk {
            Some(b) if elapsed >= b.end() => 1,
            _ => 0,
        };
        let left = c.block(block).end - elapsed;
        State::Running {
            class: index,
            elapsed,
            block,
            left,
            zone: c.warnings[block].zone(left),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MON: Weekday = Weekday::Mon;

    fn class(start: Secs, end: Secs) -> Class {
        Class::new(start, end, DEFAULT_BREAK_LEN).unwrap()
    }

    /// 09:00-09:50 (single hour).
    fn single() -> Schedule {
        let mut s = Schedule::new();
        s.set_day(MON, vec![class(hm(9, 0), hm(9, 50))]).unwrap();
        s
    }

    /// 09:00-10:40 (two hours with the default break).
    fn double() -> Schedule {
        let mut s = Schedule::new();
        s.set_day(MON, vec![class(hm(9, 0), hm(10, 40))]).unwrap();
        s
    }

    fn running(class: usize, elapsed: Secs, block: usize, left: Secs, zone: Zone) -> State {
        State::Running {
            class,
            elapsed,
            block,
            left,
            zone,
        }
    }

    #[test]
    fn clock_times_are_formatted_as_hh_mm() {
        assert_eq!(format_clock(hm(9, 5)), "09:05");
        assert_eq!(format_clock(hm(14, 40)), "14:40");
        assert_eq!(format_clock(hm(0, 0)), "00:00");
        assert_eq!(format_clock(hm(9, 5) + 59), "09:05", "seconds are dropped");
        assert_eq!(format_clock(hm(24, 0)), "24:00", "the end of the day");
    }

    // ---- classes and default break ----

    #[test]
    fn fifty_minute_class_has_no_break() {
        let c = class(hm(9, 0), hm(9, 50));
        assert_eq!(c.brk, None);
        assert_eq!(c.block_count(), 1);
        assert_eq!(
            c.block(0),
            Block {
                start: 0,
                end: 50 * MINUTE
            }
        );
    }

    #[test]
    fn hundred_minute_class_gets_centered_ten_minute_break() {
        let c = class(hm(9, 0), hm(10, 40));
        assert_eq!(
            c.brk,
            Some(Break {
                start: 45 * MINUTE,
                len: 10 * MINUTE
            })
        );
        assert_eq!(c.block_count(), 2);
        assert_eq!(
            c.block(0),
            Block {
                start: 0,
                end: 45 * MINUTE
            }
        );
        assert_eq!(
            c.block(1),
            Block {
                start: 55 * MINUTE,
                end: 100 * MINUTE
            }
        );
    }

    #[test]
    fn default_break_length_is_configurable() {
        let c = Class::new(hm(9, 0), hm(10, 40), 20 * MINUTE).unwrap();
        assert_eq!(
            c.brk,
            Some(Break {
                start: 40 * MINUTE,
                len: 20 * MINUTE
            })
        );
    }

    #[test]
    fn classes_just_under_two_hours_get_no_break() {
        assert_eq!(class(hm(9, 0), hm(10, 39)).brk, None);
    }

    #[test]
    fn explicit_break_replaces_or_removes_default() {
        let custom = Break {
            start: 30 * MINUTE,
            len: 5 * MINUTE,
        };
        let c = class(hm(9, 0), hm(10, 40))
            .with_break(Some(custom))
            .unwrap();
        assert_eq!(c.brk, Some(custom));
        assert_eq!(
            c.block(1),
            Block {
                start: 35 * MINUTE,
                end: 100 * MINUTE
            }
        );
        assert_eq!(c.with_break(None).unwrap().block_count(), 1);
    }

    #[test]
    fn invalid_classes_and_breaks_are_rejected() {
        assert_eq!(
            Class::new(hm(9, 0), hm(9, 0), DEFAULT_BREAK_LEN),
            Err(ScheduleError::EmptyClass)
        );
        assert_eq!(
            Class::new(hm(10, 0), hm(9, 0), DEFAULT_BREAK_LEN),
            Err(ScheduleError::EmptyClass)
        );
        let c = class(hm(9, 0), hm(10, 40));
        for b in [
            Break {
                start: 0,
                len: 10 * MINUTE,
            },
            Break {
                start: 10 * MINUTE,
                len: 0,
            },
            Break {
                start: 95 * MINUTE,
                len: 5 * MINUTE,
            },
            Break {
                start: 95 * MINUTE,
                len: 10 * MINUTE,
            },
        ] {
            assert_eq!(
                c.clone().with_break(Some(b)),
                Err(ScheduleError::BreakOutsideClass),
                "{b:?}"
            );
        }
        // A break longer than the class cannot be the default either.
        assert_eq!(
            Class::new(hm(9, 0), hm(10, 40), 100 * MINUTE),
            Err(ScheduleError::BreakOutsideClass)
        );
    }

    // ---- warnings ----

    #[test]
    fn zone_thresholds_are_inclusive() {
        let w = Warnings::default();
        assert_eq!(w.zone(10 * MINUTE + 1), Zone::Lesson);
        assert_eq!(w.zone(10 * MINUTE), Zone::Warning);
        assert_eq!(w.zone(5 * MINUTE + 1), Zone::Warning);
        assert_eq!(w.zone(5 * MINUTE), Zone::Final);
        assert_eq!(w.zone(0), Zone::Final);
    }

    #[test]
    fn red_may_not_exceed_orange() {
        assert!(Warnings::new(5 * MINUTE, 5 * MINUTE).is_ok());
        assert_eq!(
            Warnings::new(5 * MINUTE, 6 * MINUTE),
            Err(ScheduleError::FinalLongerThanWarning)
        );
        assert_eq!(
            class(hm(9, 0), hm(9, 50)).with_warnings(
                0,
                Warnings {
                    warning: 1,
                    final_: 2
                }
            ),
            Err(ScheduleError::FinalLongerThanWarning)
        );
    }

    #[test]
    fn warnings_longer_than_the_block_start_orange_immediately() {
        let mut s = Schedule::new();
        let c = class(hm(9, 0), hm(9, 50))
            .with_warnings(0, Warnings::new(60 * MINUTE, 5 * MINUTE).unwrap())
            .unwrap();
        s.set_day(MON, vec![c]).unwrap();
        assert!(matches!(
            s.state(MON, hm(9, 0)),
            State::Running {
                zone: Zone::Warning,
                ..
            }
        ));
    }

    // ---- state: single class ----

    #[test]
    fn single_class_walks_through_the_zones() {
        let s = single();
        let at = |m: u32, sec: u32| s.state(MON, hm(9, m) + sec);
        assert_eq!(at(0, 0), running(0, 0, 0, 50 * MINUTE, Zone::Lesson));
        // 10:01 left -> lesson, 10:00 left -> warning.
        assert_eq!(
            at(39, 59),
            running(0, 39 * MINUTE + 59, 0, 10 * MINUTE + 1, Zone::Lesson)
        );
        assert_eq!(
            at(40, 0),
            running(0, 40 * MINUTE, 0, 10 * MINUTE, Zone::Warning)
        );
        // 5:01 left -> warning, 5:00 left -> final.
        assert_eq!(
            at(44, 59),
            running(0, 44 * MINUTE + 59, 0, 5 * MINUTE + 1, Zone::Warning)
        );
        assert_eq!(
            at(45, 0),
            running(0, 45 * MINUTE, 0, 5 * MINUTE, Zone::Final)
        );
        assert_eq!(at(49, 59), running(0, 49 * MINUTE + 59, 0, 1, Zone::Final));
    }

    #[test]
    fn lead_in_linger_and_idle_around_a_class() {
        let s = single();
        let before = |secs_before: Secs| s.state(MON, hm(9, 0) - secs_before);
        assert_eq!(
            before(10 * MINUTE),
            State::Idle {
                next_start_in: Some(10 * MINUTE)
            }
        );
        assert_eq!(
            before(5 * MINUTE + 1),
            State::Idle {
                next_start_in: Some(5 * MINUTE + 1)
            }
        );
        assert_eq!(
            before(5 * MINUTE),
            State::LeadIn {
                class: 0,
                starts_in: 5 * MINUTE
            }
        );
        assert_eq!(
            before(1),
            State::LeadIn {
                class: 0,
                starts_in: 1
            }
        );

        let end = hm(9, 50);
        assert_eq!(s.state(MON, end), State::Ended { class: 0 });
        assert_eq!(s.state(MON, end + 9), State::Ended { class: 0 });
        assert_eq!(
            s.state(MON, end + 10),
            State::Idle {
                next_start_in: None
            }
        );
        assert_eq!(
            s.state(MON, hm(23, 59)),
            State::Idle {
                next_start_in: None
            }
        );
    }

    #[test]
    fn visibility() {
        let s = single();
        assert!(!s.state(MON, hm(8, 0)).is_visible());
        assert!(s.state(MON, hm(8, 56)).is_visible());
        assert!(s.state(MON, hm(9, 10)).is_visible());
        assert!(s.state(MON, hm(9, 50)).is_visible());
        assert!(!s.state(MON, hm(10, 0)).is_visible());
    }

    #[test]
    fn lead_in_and_linger_are_configurable() {
        let mut s = single();
        s.lead_in = 60;
        s.linger = 0;
        assert_eq!(
            s.state(MON, hm(9, 0) - 61),
            State::Idle {
                next_start_in: Some(61)
            }
        );
        assert_eq!(
            s.state(MON, hm(9, 0) - 60),
            State::LeadIn {
                class: 0,
                starts_in: 60
            }
        );
        assert_eq!(
            s.state(MON, hm(9, 50)),
            State::Idle {
                next_start_in: None
            }
        );
    }

    // ---- state: class with break ----

    #[test]
    fn two_hour_class_has_two_blocks_and_a_break() {
        let s = double();
        let at = |m: u32, sec: u32| s.state(MON, hm(9, 0) + m * MINUTE + sec);
        // Block 1 (45 min): warning in its last 10 minutes, final in its last 5.
        assert_eq!(at(0, 0), running(0, 0, 0, 45 * MINUTE, Zone::Lesson));
        assert_eq!(
            at(34, 59),
            running(0, 34 * MINUTE + 59, 0, 10 * MINUTE + 1, Zone::Lesson)
        );
        assert_eq!(
            at(35, 0),
            running(0, 35 * MINUTE, 0, 10 * MINUTE, Zone::Warning)
        );
        assert_eq!(
            at(40, 0),
            running(0, 40 * MINUTE, 0, 5 * MINUTE, Zone::Final)
        );
        assert_eq!(at(44, 59), running(0, 44 * MINUTE + 59, 0, 1, Zone::Final));
        // Break (10 min).
        assert_eq!(
            at(45, 0),
            State::Break {
                class: 0,
                elapsed: 45 * MINUTE,
                left: 10 * MINUTE
            }
        );
        assert_eq!(
            at(54, 59),
            State::Break {
                class: 0,
                elapsed: 54 * MINUTE + 59,
                left: 1
            }
        );
        // Block 2 starts in the lesson zone again with its own zones.
        assert_eq!(
            at(55, 0),
            running(0, 55 * MINUTE, 1, 45 * MINUTE, Zone::Lesson)
        );
        assert_eq!(
            at(90, 0),
            running(0, 90 * MINUTE, 1, 10 * MINUTE, Zone::Warning)
        );
        assert_eq!(
            at(95, 0),
            running(0, 95 * MINUTE, 1, 5 * MINUTE, Zone::Final)
        );
        assert_eq!(at(99, 59), running(0, 99 * MINUTE + 59, 1, 1, Zone::Final));
        assert_eq!(at(100, 0), State::Ended { class: 0 });
    }

    #[test]
    fn blocks_have_independent_warnings() {
        let mut s = Schedule::new();
        let c = class(hm(9, 0), hm(10, 40))
            .with_warnings(0, Warnings::new(2 * MINUTE, MINUTE).unwrap())
            .unwrap();
        s.set_day(MON, vec![c]).unwrap();
        // Block 1: warning only in the last 2 minutes.
        assert!(matches!(
            s.state(MON, hm(9, 40)),
            State::Running {
                block: 0,
                zone: Zone::Lesson,
                ..
            }
        ));
        assert!(matches!(
            s.state(MON, hm(9, 43)),
            State::Running {
                block: 0,
                zone: Zone::Warning,
                ..
            }
        ));
        // Block 2 keeps the defaults.
        assert!(matches!(
            s.state(MON, hm(10, 30)),
            State::Running {
                block: 1,
                zone: Zone::Warning,
                ..
            }
        ));
    }

    #[test]
    fn moved_break_changes_the_blocks() {
        let mut s = Schedule::new();
        let brk = Break {
            start: 30 * MINUTE,
            len: 20 * MINUTE,
        };
        let c = class(hm(9, 0), hm(10, 40)).with_break(Some(brk)).unwrap();
        s.set_day(MON, vec![c]).unwrap();
        assert_eq!(
            s.state(MON, hm(9, 30)),
            State::Break {
                class: 0,
                elapsed: 30 * MINUTE,
                left: 20 * MINUTE
            }
        );
        assert_eq!(
            s.state(MON, hm(9, 50)),
            running(0, 50 * MINUTE, 1, 50 * MINUTE, Zone::Lesson)
        );
    }

    // ---- several classes ----

    #[test]
    fn back_to_back_classes_stay_visible() {
        let mut s = Schedule::new();
        s.set_day(
            MON,
            vec![class(hm(9, 0), hm(9, 50)), class(hm(9, 50), hm(10, 40))],
        )
        .unwrap();
        // The second class takes over at the exact end of the first; no Ended or Idle in between.
        assert!(matches!(
            s.state(MON, hm(9, 49) + 59),
            State::Running { class: 0, .. }
        ));
        assert_eq!(
            s.state(MON, hm(9, 50)),
            running(1, 0, 0, 50 * MINUTE, Zone::Lesson)
        );
        // During the first class, nothing hides the window either.
        assert!(s.state(MON, hm(9, 30)).is_visible());
    }

    #[test]
    fn short_gap_shows_ended_then_lead_in() {
        let mut s = Schedule::new();
        s.set_day(
            MON,
            vec![class(hm(9, 0), hm(9, 50)), class(hm(9, 55), hm(10, 45))],
        )
        .unwrap();
        let end = hm(9, 50);
        assert_eq!(s.state(MON, end), State::Ended { class: 0 });
        assert_eq!(
            s.state(MON, end + 10),
            State::LeadIn {
                class: 1,
                starts_in: 5 * MINUTE - 10
            }
        );
        assert_eq!(
            s.state(MON, hm(9, 55)),
            running(1, 0, 0, 50 * MINUTE, Zone::Lesson)
        );
    }

    #[test]
    fn long_gap_is_idle_with_time_to_next_class() {
        let mut s = Schedule::new();
        s.set_day(
            MON,
            vec![class(hm(9, 0), hm(9, 50)), class(hm(11, 0), hm(11, 50))],
        )
        .unwrap();
        assert_eq!(
            s.state(MON, hm(10, 0)),
            State::Idle {
                next_start_in: Some(hm(1, 0))
            }
        );
        assert_eq!(
            s.state(MON, hm(10, 56)),
            State::LeadIn {
                class: 1,
                starts_in: 4 * MINUTE
            }
        );
    }

    #[test]
    fn classes_are_sorted_and_overlap_is_rejected() {
        let mut s = Schedule::new();
        s.set_day(
            MON,
            vec![class(hm(11, 0), hm(11, 50)), class(hm(9, 0), hm(9, 50))],
        )
        .unwrap();
        assert_eq!(s.classes(MON)[0].start, hm(9, 0));
        assert_eq!(
            s.set_day(
                MON,
                vec![class(hm(9, 0), hm(9, 50)), class(hm(9, 49), hm(10, 40))]
            ),
            Err(ScheduleError::Overlap)
        );
        // A failed update leaves the day untouched.
        assert_eq!(s.classes(MON).len(), 2);
    }

    #[test]
    fn invalid_class_in_set_day_is_rejected() {
        let mut s = Schedule::new();
        let mut bad = class(hm(9, 0), hm(9, 50));
        bad.end = bad.start;
        assert_eq!(s.set_day(MON, vec![bad]), Err(ScheduleError::EmptyClass));
    }

    // ---- days ----

    #[test]
    fn each_weekday_has_its_own_classes() {
        let mut s = Schedule::new();
        s.set_day(Weekday::Mon, vec![class(hm(9, 0), hm(9, 50))])
            .unwrap();
        s.set_day(Weekday::Tue, vec![class(hm(10, 0), hm(11, 40))])
            .unwrap();
        assert!(matches!(
            s.state(Weekday::Mon, hm(9, 10)),
            State::Running { .. }
        ));
        assert_eq!(
            s.state(Weekday::Wed, hm(9, 10)),
            State::Idle {
                next_start_in: None
            }
        );
        assert_eq!(
            s.state(Weekday::Tue, hm(9, 10)),
            State::Idle {
                next_start_in: Some(50 * MINUTE)
            }
        );
        assert_eq!(s.classes(Weekday::Tue)[0].block_count(), 2);
    }

    #[test]
    fn weekends_have_no_classes() {
        let s = single();
        for day in [Weekday::Sat, Weekday::Sun] {
            assert!(s.classes(day).is_empty());
            assert_eq!(
                s.state(day, hm(9, 10)),
                State::Idle {
                    next_start_in: None
                }
            );
        }
    }

    #[test]
    fn day_change_resets_state() {
        let s = single();
        // 23:59 on Monday is idle; 09:10 on Tuesday (no classes set) is idle too.
        assert!(!s.state(MON, hm(23, 59)).is_visible());
        assert!(!s.state(Weekday::Tue, hm(9, 10)).is_visible());
        assert!(s.state(MON, hm(9, 10)).is_visible());
    }

    #[test]
    fn class_mut_allows_per_class_edits() {
        let mut s = double();
        let c = s.class_mut(MON, 0).unwrap();
        c.brk = None;
        assert!(matches!(
            s.state(MON, hm(9, 50)),
            State::Running { block: 0, .. }
        ));
        assert!(s.class_mut(MON, 1).is_none());
    }
}
