//! Simulation time, the calendar and pulses (SPR-005 T-01).
//!
//! - [`SimTime`]: `i64` minutes from the world epoch. Negative values are before the epoch, so
//!   founders can be born before year 0 (ADR-0026).
//! - [`Calendar`]: 12 months, a 360-day year (twelve 30-day months) or a 365-day year (31, 28, 31,
//!   30, 31, 30, 31, 31, 30, 31, 30, 31), and 2 or 4 seasons.
//! - [`Pulse`]: `Year`, `Season`, `Month`, `Day`, `Hour`. A pulse fires at the first minute of
//!   each of its periods. The bubble's presentation-only `Realtime` pulse is plane B and does not
//!   exist here.
//!
//! The epoch, `SimTime(0)`, is year 0, month 1, day 1, 00:00. Years are proleptic integers: year
//! −1 ends one minute before the epoch. There are no leap years (`05.1` §4.1).
//!
//! DECISION: seasons start at month 1 and divide the year into equal runs of whole months: with
//! four seasons they start at months 1, 4, 7 and 10; with two, at months 1 and 7. No contract
//! fixes the season starts; a world that wants another alignment needs a contract change, not a
//! parameter, because the Season pulse is part of the phase schedule.
//!
//! Every operation is integer arithmetic with Euclidean division (floor toward −∞), so times
//! before the epoch behave exactly like times after it. Operations that could leave the `i64`
//! range return `None`; nothing wraps or saturates silently.
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §4.1; ADR-0005 (and amendment v1.1 items 1–2);
//! ADR-0026; `CODEX/05_PROJECT/SPR-005_CoreTimeAndScheduler.md` T-01.

use core::fmt;

/// Minutes in an hour.
pub const MINUTES_PER_HOUR: i64 = 60;
/// Minutes in a day.
pub const MINUTES_PER_DAY: i64 = 24 * MINUTES_PER_HOUR;
/// Months in a year, in every calendar (`05.1` §4.1).
pub const MONTHS_PER_YEAR: u8 = 12;

/// A simulation instant: minutes from the world epoch, signed (ADR-0005 v1.1 item 1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimTime(i64);

impl SimTime {
    /// The world epoch: year 0, month 1, day 1, 00:00.
    pub const EPOCH: Self = Self(0);
    /// The earliest representable instant.
    pub const MIN: Self = Self(i64::MIN);
    /// The latest representable instant.
    pub const MAX: Self = Self(i64::MAX);

    /// The instant `minutes` after the epoch (before it when negative).
    #[must_use]
    pub const fn from_minutes(minutes: i64) -> Self {
        Self(minutes)
    }

    /// Minutes from the epoch.
    #[must_use]
    pub const fn minutes(self) -> i64 {
        self.0
    }

    /// `self + minutes`, or `None` outside the `i64` range.
    #[must_use]
    pub const fn checked_add_minutes(self, minutes: i64) -> Option<Self> {
        match self.0.checked_add(minutes) {
            Some(m) => Some(Self(m)),
            None => None,
        }
    }
}

impl fmt::Display for SimTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t={}min", self.0)
    }
}

/// The length of the calendar year (`05.1` §4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DaysPerYear {
    /// Twelve 30-day months.
    D360,
    /// 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31.
    D365,
}

/// The number of seasons in a year (`05.1` §4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SeasonsPerYear {
    /// Two seasons of six months, starting at months 1 and 7.
    Two,
    /// Four seasons of three months, starting at months 1, 4, 7 and 10.
    Four,
}

impl SeasonsPerYear {
    /// The number of seasons.
    #[must_use]
    pub const fn count(self) -> u8 {
        match self {
            Self::Two => 2,
            Self::Four => 4,
        }
    }

    /// Months in each season.
    #[must_use]
    pub const fn months_per_season(self) -> u8 {
        MONTHS_PER_YEAR / self.count()
    }
}

/// A pulse: the cadence a system binds to (`05.1` §4.1), coarsest first.
///
/// The derived order is coarse to fine. A boundary of a coarser pulse is always a boundary of
/// every finer one (a year starts on a season, month, day and hour boundary).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Pulse {
    /// The first minute of each year.
    Year,
    /// The first minute of each season.
    Season,
    /// The first minute of each month.
    Month,
    /// The first minute of each day.
    Day,
    /// The first minute of each hour.
    Hour,
}

impl Pulse {
    /// Every pulse, coarsest first.
    pub const ALL: [Self; 5] = [Self::Year, Self::Season, Self::Month, Self::Day, Self::Hour];

    /// The name used in the system specs (`[[schedule]] pulse`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Year => "Year",
            Self::Season => "Season",
            Self::Month => "Month",
            Self::Day => "Day",
            Self::Hour => "Hour",
        }
    }

    /// The pulse with this spec name, if any.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.name() == name)
    }
}

impl fmt::Display for Pulse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A calendar date and time of day. Months and days count from 1.
///
/// The derived order (year, month, day, hour, minute) is chronological.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    /// The year; negative before the epoch.
    pub year: i64,
    /// The month, `1..=12`.
    pub month: u8,
    /// The day of the month, from 1 to the month's length.
    pub day: u8,
    /// The hour, `0..=23`.
    pub hour: u8,
    /// The minute, `0..=59`.
    pub minute: u8,
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}-{:02}-{:02} {:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute
        )
    }
}

const MONTHS_360: [u8; 12] = [30; 12];
const MONTHS_365: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// A world calendar: year length and season count (`05.1` §4.1; the manifest chooses).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Calendar {
    /// The year length.
    pub days_per_year: DaysPerYear,
    /// The season count.
    pub seasons: SeasonsPerYear,
}

impl Calendar {
    /// A calendar.
    #[must_use]
    pub const fn new(days_per_year: DaysPerYear, seasons: SeasonsPerYear) -> Self {
        Self {
            days_per_year,
            seasons,
        }
    }

    /// The month lengths in days, January first.
    #[must_use]
    pub const fn month_lengths(&self) -> &'static [u8; 12] {
        match self.days_per_year {
            DaysPerYear::D360 => &MONTHS_360,
            DaysPerYear::D365 => &MONTHS_365,
        }
    }

    /// Days in a year: 360 or 365.
    #[must_use]
    pub const fn days_in_year(&self) -> i64 {
        match self.days_per_year {
            DaysPerYear::D360 => 360,
            DaysPerYear::D365 => 365,
        }
    }

    /// Minutes in a year.
    #[must_use]
    pub const fn minutes_per_year(&self) -> i64 {
        self.days_in_year() * MINUTES_PER_DAY
    }

    /// Days in a month (`1..=12`), or `None` for another month number.
    #[must_use]
    pub fn days_in_month(&self, month: u8) -> Option<u8> {
        let index = usize::from(month.checked_sub(1)?);
        self.month_lengths().get(index).copied()
    }

    /// The day of the year (0-based) on which a month (`1..=12`) starts.
    fn month_start_day(self, month: u8) -> i64 {
        self.month_lengths()
            .iter()
            .take(usize::from(month.saturating_sub(1)))
            .map(|&d| i64::from(d))
            .sum()
    }

    /// The season (0-based) a month (`1..=12`) belongs to.
    #[must_use]
    pub const fn season_of_month(&self, month: u8) -> u8 {
        (month.saturating_sub(1)) / self.seasons.months_per_season()
    }

    /// The calendar date of an instant. Total: every `i64` minute has a date.
    #[must_use]
    pub fn to_date(&self, t: SimTime) -> Date {
        let year = t.0.div_euclid(self.minutes_per_year());
        let in_year = t.0.rem_euclid(self.minutes_per_year());
        let day_of_year = in_year / MINUTES_PER_DAY;
        let in_day = in_year % MINUTES_PER_DAY;
        let mut month: u8 = 1;
        let mut day_left = day_of_year;
        for &len in self.month_lengths() {
            if day_left < i64::from(len) {
                break;
            }
            day_left -= i64::from(len);
            month += 1;
        }
        Date {
            year,
            month,
            // day_left < the month's length <= 31; hour < 24; minute < 60.
            day: narrow(day_left + 1),
            hour: narrow(in_day / MINUTES_PER_HOUR),
            minute: narrow(in_day % MINUTES_PER_HOUR),
        }
    }

    /// The instant of a date, or `None` if a field is out of range (month 13, day 31 of a
    /// 30-day month, hour 24) or the instant is outside the `i64` range.
    #[must_use]
    pub fn from_date(&self, date: Date) -> Option<SimTime> {
        let len = self.days_in_month(date.month)?;
        if date.day == 0 || date.day > len || date.hour > 23 || date.minute > 59 {
            return None;
        }
        let day_of_year = self.month_start_day(date.month) + i64::from(date.day) - 1;
        let in_year = day_of_year * MINUTES_PER_DAY
            + i64::from(date.hour) * MINUTES_PER_HOUR
            + i64::from(date.minute);
        let start = date.year.checked_mul(self.minutes_per_year())?;
        start.checked_add(in_year).map(SimTime)
    }

    /// The start of the `pulse` period that contains `t`: the latest boundary at or before `t`.
    /// `None` only when that boundary is before `SimTime::MIN`.
    #[must_use]
    pub fn floor(&self, pulse: Pulse, t: SimTime) -> Option<SimTime> {
        let d = self.to_date(t);
        let date = match pulse {
            Pulse::Year => Date {
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                ..d
            },
            Pulse::Season => Date {
                month: self.season_of_month(d.month) * self.seasons.months_per_season() + 1,
                day: 1,
                hour: 0,
                minute: 0,
                ..d
            },
            Pulse::Month => Date {
                day: 1,
                hour: 0,
                minute: 0,
                ..d
            },
            Pulse::Day => Date {
                hour: 0,
                minute: 0,
                ..d
            },
            Pulse::Hour => Date { minute: 0, ..d },
        };
        self.from_date(date)
    }

    /// Whether `t` is a boundary of `pulse` (the pulse fires at `t`).
    #[must_use]
    pub fn is_boundary(&self, pulse: Pulse, t: SimTime) -> bool {
        self.floor(pulse, t) == Some(t)
    }

    /// The first boundary of `pulse` strictly after `t`, or `None` past `SimTime::MAX`.
    #[must_use]
    pub fn next_boundary(&self, pulse: Pulse, t: SimTime) -> Option<SimTime> {
        let d = self.to_date(t);
        let next = match pulse {
            Pulse::Hour => {
                let start = t.0.checked_sub(i64::from(d.minute))?;
                return start.checked_add(MINUTES_PER_HOUR).map(SimTime);
            }
            Pulse::Day => {
                let start = t.0.checked_sub(t.0.rem_euclid(MINUTES_PER_DAY))?;
                return start.checked_add(MINUTES_PER_DAY).map(SimTime);
            }
            Pulse::Month => {
                if d.month == MONTHS_PER_YEAR {
                    first_of(d.year.checked_add(1)?, 1)
                } else {
                    first_of(d.year, d.month + 1)
                }
            }
            Pulse::Season => {
                let per = self.seasons.months_per_season();
                let next_season = self.season_of_month(d.month) + 1;
                if next_season == self.seasons.count() {
                    first_of(d.year.checked_add(1)?, 1)
                } else {
                    first_of(d.year, next_season * per + 1)
                }
            }
            Pulse::Year => first_of(d.year.checked_add(1)?, 1),
        };
        self.from_date(next)
    }

    /// The pulses that fire at `t`, coarsest first. A boundary of a coarse pulse is a boundary of
    /// every finer one, so this is always a suffix of [`Pulse::ALL`].
    #[must_use]
    pub fn pulses_at(&self, t: SimTime) -> Vec<Pulse> {
        Pulse::ALL
            .into_iter()
            .filter(|&p| self.is_boundary(p, t))
            .collect()
    }

    /// The first instant strictly after `t` at which any of `pulses` fires, or `None` if `pulses`
    /// is empty or every next boundary is past `SimTime::MAX`.
    #[must_use]
    pub fn next_instant(&self, t: SimTime, pulses: &[Pulse]) -> Option<SimTime> {
        pulses
            .iter()
            .filter_map(|&p| self.next_boundary(p, t))
            .min()
    }
}

/// The first minute of a month.
const fn first_of(year: i64, month: u8) -> Date {
    Date {
        year,
        month,
        day: 1,
        hour: 0,
        minute: 0,
    }
}

/// Narrows a calendar field already known to be in `0..=255`.
fn narrow(v: i64) -> u8 {
    // INVARIANT: callers pass a day (1..=31), an hour (0..=23) or a minute (0..=59).
    u8::try_from(v).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::{Calendar, Date, DaysPerYear, Pulse, SeasonsPerYear, SimTime};

    const C365: Calendar = Calendar::new(DaysPerYear::D365, SeasonsPerYear::Four);
    const C360: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Two);

    fn date(year: i64, month: u8, day: u8, hour: u8, minute: u8) -> Date {
        Date {
            year,
            month,
            day,
            hour,
            minute,
        }
    }

    #[test]
    fn the_epoch_and_the_minute_before_it() {
        assert_eq!(C365.to_date(SimTime::EPOCH), date(0, 1, 1, 0, 0));
        assert_eq!(
            C365.to_date(SimTime::from_minutes(-1)),
            date(-1, 12, 31, 23, 59)
        );
        assert_eq!(
            C360.to_date(SimTime::from_minutes(-1)),
            date(-1, 12, 30, 23, 59)
        );
    }

    #[test]
    fn month_lengths_sum_to_the_year() {
        for c in [C365, C360] {
            let sum: i64 = c.month_lengths().iter().map(|&d| i64::from(d)).sum();
            assert_eq!(sum, c.days_in_year());
        }
        assert_eq!(C365.days_in_month(2), Some(28));
        assert_eq!(C365.days_in_month(0), None);
        assert_eq!(C365.days_in_month(13), None);
    }

    #[test]
    fn invalid_dates_are_refused() {
        assert_eq!(C365.from_date(date(0, 2, 29, 0, 0)), None);
        assert_eq!(C360.from_date(date(0, 1, 31, 0, 0)), None);
        assert_eq!(C365.from_date(date(0, 13, 1, 0, 0)), None);
        assert_eq!(C365.from_date(date(0, 1, 0, 0, 0)), None);
        assert_eq!(C365.from_date(date(0, 1, 1, 24, 0)), None);
        assert_eq!(C365.from_date(date(0, 1, 1, 0, 60)), None);
        assert_eq!(C365.from_date(date(i64::MAX, 1, 1, 0, 0)), None);
    }

    #[test]
    fn seasons_start_on_the_documented_months() {
        let four: Vec<u8> = (1..=12).map(|m| C365.season_of_month(m)).collect();
        assert_eq!(four, [0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3]);
        let two: Vec<u8> = (1..=12).map(|m| C360.season_of_month(m)).collect();
        assert_eq!(two, [0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1]);
    }

    #[test]
    fn pulses_at_a_year_start_are_all_of_them() {
        let t = C365.from_date(date(-3, 1, 1, 0, 0)).unwrap();
        assert_eq!(C365.pulses_at(t), Pulse::ALL.to_vec());
        let april = C365.from_date(date(5, 4, 1, 0, 0)).unwrap();
        assert_eq!(
            C365.pulses_at(april),
            [Pulse::Season, Pulse::Month, Pulse::Day, Pulse::Hour]
        );
        let april_360 = C360.from_date(date(5, 4, 1, 0, 0)).unwrap();
        assert_eq!(
            C360.pulses_at(april_360),
            [Pulse::Month, Pulse::Day, Pulse::Hour]
        );
        let mid = SimTime::from_minutes(17);
        assert_eq!(C365.pulses_at(mid), []);
    }

    #[test]
    fn pulse_names_round_trip() {
        for p in Pulse::ALL {
            assert_eq!(Pulse::from_name(p.name()), Some(p));
        }
        assert_eq!(Pulse::from_name("Realtime"), None);
    }

    #[test]
    fn the_edges_of_the_range_do_not_wrap() {
        assert_eq!(C365.next_boundary(Pulse::Hour, SimTime::MAX), None);
        assert_eq!(C365.next_boundary(Pulse::Year, SimTime::MAX), None);
        assert_eq!(C365.floor(Pulse::Year, SimTime::MIN), None);
        assert_eq!(SimTime::MAX.checked_add_minutes(1), None);
        let _ = C365.to_date(SimTime::MIN);
        let _ = C365.to_date(SimTime::MAX);
    }
}
