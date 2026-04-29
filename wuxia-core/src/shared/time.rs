// wuxia-core/src/shared/time.rs
//
// Game world calendar — a simplified time system for the wuxia world.
//
// Design decisions:
//   - 1 year  = 12 months
//   - 1 month = 30 days
//   - 1 year  = 360 days (simplified — no leap years, no irregular months)
//   - 1 day   = 6 watches (시간대/經) [v1.1]
//   - 1 year  = 2160 watches (360 × 6)
//   - Seasons follow the wuxia convention:
//       Winter: months 12, 1-2 (겨울)
//       Spring: months 3-5    (봄)
//       Summer: months 6-8    (여름)
//       Autumn: months 9-11   (가을)
//
// WHY simplify?
// Real calendars have 28/30/31-day months, leap years, etc.
// For a game, consistent 30-day months make time math trivial:
//   total_days = (year * 360) + ((month - 1) * 30) + day
// No edge cases. No bugs. Easy to test.
//
// [v1.1] Watch system — 십이시진(十二時辰) 2개씩 묶음
//
// A day is divided into 6 watches (경/更), each representing
// a pair of traditional Chinese double-hours (시진/時辰):
//
//   Dawn      (黎明/새벽)   인묘시(寅卯)  03~07시   내공 수련 보너스
//   Morning   (午前/오전)   진사시(辰巳)  07~11시   탐험/이동 최적
//   Midday    (正午/한낮)   오미시(午未)  11~15시   외공/무력 보너스
//   Afternoon (午後/오후)   신유시(申酉)  15~19시   독서와 연마에 적합
//   Evening   (黃昏/저녁)   술해시(戌亥)  19~23시   휴식과 관계 형성
//   Night     (深夜/심야)   자축시(子丑)  23~03시   수면, 사파 활동
//
// A day starts at Dawn and ends after Night.
// "묘시까지 수련한다" = tick_until(Watch::Morning)

use serde::{Deserialize, Serialize};
use std::fmt;

use super::i18n::Translatable;

/// Days in each month (all months are 30 days in our simplified calendar).
pub const DAYS_PER_MONTH: u32 = 30;

/// Months in a year.
pub const MONTHS_PER_YEAR: u32 = 12;

/// Days in a year.
pub const DAYS_PER_YEAR: u32 = DAYS_PER_MONTH * MONTHS_PER_YEAR; // 360

/// Watches (시간대) in a day. [v1.1]
pub const WATCHES_PER_DAY: u32 = 6;

/// Watches in a year. [v1.1]
pub const WATCHES_PER_YEAR: u32 = DAYS_PER_YEAR * WATCHES_PER_DAY; // 2160

// ---------------------------------------------------------------------------
// Watch (시간대/經) [v1.1]
// ---------------------------------------------------------------------------

/// The six watches of a day — 십이시진(十二時辰) paired into game time units.
///
/// Each watch represents ~4 hours of in-world time.
/// The day starts at Dawn and ends after Night.
///
/// In wuxia novels, time is expressed as "자시에 만나자" (meet at the hour of
/// the rat). This system lets us say "진시(Morning)까지 수련한다" naturally.
///
/// ```
/// use wuxia_core::shared::time::Watch;
///
/// let dawn = Watch::Dawn;
/// assert_eq!(dawn.next(), Watch::Morning);
/// assert_eq!(dawn.index(), 0);
/// assert!(!dawn.is_last());
///
/// let night = Watch::Night;
/// assert_eq!(night.next(), Watch::Dawn);
/// assert!(night.is_last());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Watch {
    /// 黎明 (새벽) — 인시~묘시 (寅卯) 03~07시. 내공 수련에 좋다.
    Dawn = 0,
    /// 午前 (오전) — 진시~사시 (辰巳) 07~11시. 탐험과 이동에 적합.
    Morning = 1,
    /// 正午 (한낮) — 오시~미시 (午未) 11~15시. 양기 절정, 외공 수련.
    Midday = 2,
    /// 午後 (오후) — 신시~유시 (申酉) 15~19시. 독서와 연마에 적합.
    Afternoon = 3,
    /// 黃昏 (저녁) — 술시~해시 (戌亥) 19~23시. 휴식과 관계 형성.
    Evening = 4,
    /// 深夜 (심야) — 자시~축시 (子丑) 23~03시. 수면, 사파 활동.
    Night = 5,
}

/// All watches in order, for iteration.
const ALL_WATCHES: [Watch; 6] = [
    Watch::Dawn,
    Watch::Morning,
    Watch::Midday,
    Watch::Afternoon,
    Watch::Evening,
    Watch::Night,
];

impl Watch {
    /// Numeric index (0=Dawn, 5=Night). Used for total_watches math.
    pub fn index(self) -> u32 {
        self as u32
    }

    /// Create Watch from index (0~5). Panics if out of range.
    pub fn from_index(index: u32) -> Self {
        ALL_WATCHES[index as usize]
    }

    /// Next watch in the cycle. Night wraps to Dawn.
    pub fn next(self) -> Self {
        Self::from_index((self.index() + 1) % WATCHES_PER_DAY)
    }

    /// Is this the last watch of the day? (Night)
    pub fn is_last(self) -> bool {
        self == Watch::Night
    }

    /// Number of watches from self to target (going forward).
    /// Dawn→Morning = 1, Dawn→Dawn = 6 (full cycle), Night→Dawn = 1.
    pub fn watches_until(self, target: Watch) -> u32 {
        let diff = (target.index() as i32) - (self.index() as i32);
        if diff <= 0 {
            (diff + WATCHES_PER_DAY as i32) as u32
        } else {
            diff as u32
        }
    }

    /// All six watches in order.
    pub fn all() -> &'static [Watch; 6] {
        &ALL_WATCHES
    }
}

impl Default for Watch {
    /// Default watch is Dawn — the start of a new day.
    fn default() -> Self {
        Watch::Dawn
    }
}

impl PartialOrd for Watch {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Watch {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index().cmp(&other.index())
    }
}

impl fmt::Display for Watch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Watch::Dawn => "Dawn",
            Watch::Morning => "Morning",
            Watch::Midday => "Midday",
            Watch::Afternoon => "Afternoon",
            Watch::Evening => "Evening",
            Watch::Night => "Night",
        };
        write!(f, "{}", name)
    }
}

impl Translatable for Watch {
    fn translation_key(&self) -> &'static str {
        match self {
            Watch::Dawn => "watch.dawn",
            Watch::Morning => "watch.morning",
            Watch::Midday => "watch.midday",
            Watch::Afternoon => "watch.afternoon",
            Watch::Evening => "watch.evening",
            Watch::Night => "watch.night",
        }
    }
}

// ---------------------------------------------------------------------------
// Season
// ---------------------------------------------------------------------------

/// The four seasons of the wuxia world.
///
/// Seasons affect travel, combat modifiers, and narrative events.
/// For example, the "Plum Blossom Sword Meeting" only happens in winter.
///
/// Season mapping (natural seasons):
///   Winter: 12, 1, 2
///   Spring:  3, 4, 5
///   Summer:  6, 7, 8
///   Autumn:  9, 10, 11
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Season {
    Spring, // 봄: months 3-5
    Summer, // 여름: months 6-8
    Autumn, // 가을: months 9-11
    Winter, // 겨울: months 12, 1-2
}

impl Season {
    /// Determine the season from a month number (1-12).
    ///
    /// Panics if month is outside 1..=12 (enforced by GameTime constructor).
    pub fn from_month(month: u32) -> Self {
        match month {
            3..=5 => Season::Spring,
            6..=8 => Season::Summer,
            9..=11 => Season::Autumn,
            12 | 1..=2 => Season::Winter,
            _ => unreachable!("month must be 1..=12, got {}", month),
        }
    }
}

impl fmt::Display for Season {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Translatable for Season {
    fn translation_key(&self) -> &'static str {
        match self {
            Season::Spring => "season.spring",
            Season::Summer => "season.summer",
            Season::Autumn => "season.autumn",
            Season::Winter => "season.winter",
        }
    }
}

// ---------------------------------------------------------------------------
// GameTime
// ---------------------------------------------------------------------------

/// A point in time within the game world.
///
/// Represents a specific date and watch (year/month/day/watch) in the wuxia
/// world calendar. GameTime is the universal clock that all domains reference.
///
/// [v1.1] Now includes Watch (시간대) for sub-day granularity.
/// Backward compatibility: `new()` defaults to Dawn.
///
/// Example:
/// ```
/// use wuxia_core::shared::GameTime;
/// use wuxia_core::shared::time::Watch;
///
/// // Backward-compatible: defaults to Dawn
/// let date = GameTime::new(1200, 3, 15);
/// assert_eq!(date.year(), 1200);
/// assert_eq!(date.month(), 3);
/// assert_eq!(date.day(), 15);
/// assert_eq!(date.watch(), Watch::Dawn);
///
/// // Precise: specify watch
/// let precise = GameTime::with_watch(1200, 3, 15, Watch::Midday);
/// assert_eq!(precise.watch(), Watch::Midday);
/// assert!(precise > date);  // Midday is later than Dawn on the same day
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GameTime {
    year: u32,
    month: u32,  // 1..=12
    day: u32,    // 1..=30
    #[serde(default)]
    watch: Watch, // [v1.1] Dawn..Night, defaults to Dawn for backward compat
}

impl GameTime {
    /// Create a new GameTime at Dawn. Month must be 1-12, day must be 1-30.
    ///
    /// Backward-compatible: watch defaults to Dawn.
    /// Panics if values are out of range (use `try_new` for fallible creation).
    pub fn new(year: u32, month: u32, day: u32) -> Self {
        Self::with_watch(year, month, day, Watch::Dawn)
    }

    /// Create a new GameTime with a specific watch. [v1.1]
    ///
    /// Panics if month or day are out of range.
    pub fn with_watch(year: u32, month: u32, day: u32, watch: Watch) -> Self {
        Self::try_new_with_watch(year, month, day, watch).unwrap_or_else(|| {
            panic!(
                "invalid date: month={} (1..={}), day={} (1..={})",
                month, MONTHS_PER_YEAR, day, DAYS_PER_MONTH
            )
        })
    }

    /// Fallible creation at Dawn — returns None if values are out of range.
    pub fn try_new(year: u32, month: u32, day: u32) -> Option<Self> {
        Self::try_new_with_watch(year, month, day, Watch::Dawn)
    }

    /// Fallible creation with a specific watch. [v1.1]
    pub fn try_new_with_watch(
        year: u32,
        month: u32,
        day: u32,
        watch: Watch,
    ) -> Option<Self> {
        if (1..=MONTHS_PER_YEAR).contains(&month) && (1..=DAYS_PER_MONTH).contains(&day) {
            Some(Self {
                year,
                month,
                day,
                watch,
            })
        } else {
            None
        }
    }

    // --- Getters ---

    pub fn year(&self) -> u32 {
        self.year
    }

    pub fn month(&self) -> u32 {
        self.month
    }

    pub fn day(&self) -> u32 {
        self.day
    }

    /// Current watch (시간대). [v1.1]
    pub fn watch(&self) -> Watch {
        self.watch
    }

    /// Current season based on the month.
    pub fn season(&self) -> Season {
        Season::from_month(self.month)
    }

    /// Is this the last day of the year? (month=12, day=30)
    pub fn is_last_day_of_year(&self) -> bool {
        self.month == MONTHS_PER_YEAR && self.day == DAYS_PER_MONTH
    }

    /// Is this the last day of the current month? (day=30)
    pub fn is_last_day_of_month(&self) -> bool {
        self.day == DAYS_PER_MONTH
    }

    // --- Day-level arithmetic (backward compatible) ---

    /// Convert to a total number of days (for easy comparison/arithmetic).
    /// Day 1 of year 0, month 1 = total_days 1.
    ///
    /// NOTE: This ignores the watch field. Use `to_total_watches()` for
    /// sub-day precision.
    pub fn to_total_days(&self) -> u64 {
        (self.year as u64) * (DAYS_PER_YEAR as u64)
            + ((self.month - 1) as u64) * (DAYS_PER_MONTH as u64)
            + (self.day as u64)
    }

    /// Create GameTime from total days. Watch is set to Dawn.
    pub fn from_total_days(total: u64) -> Self {
        // total_days is 1-based, so day 1 = year 0, month 1, day 1
        let total_zero = total - 1; // shift to 0-based for division

        let year = (total_zero / DAYS_PER_YEAR as u64) as u32;
        let remaining = (total_zero % DAYS_PER_YEAR as u64) as u32;

        let month = (remaining / DAYS_PER_MONTH) + 1;
        let day = (remaining % DAYS_PER_MONTH) + 1;

        Self {
            year,
            month,
            day,
            watch: Watch::Dawn,
        }
    }

    /// Advance by one day, returning the new GameTime at Dawn.
    pub fn next_day(&self) -> Self {
        Self::from_total_days(self.to_total_days() + 1)
    }

    /// Calculate the number of days between two GameTimes.
    /// Returns positive if `other` is later, negative if earlier.
    ///
    /// NOTE: This ignores the watch field and compares at day level only.
    pub fn days_between(&self, other: &GameTime) -> i64 {
        other.to_total_days() as i64 - self.to_total_days() as i64
    }

    /// Advance by a given number of days. Watch resets to Dawn.
    pub fn advance_days(&self, days: u32) -> Self {
        Self::from_total_days(self.to_total_days() + days as u64)
    }

    // --- Watch-level arithmetic [v1.1] ---

    /// Convert to total watches (0-based). [v1.1]
    ///
    /// GameTime(0, 1, 1, Dawn) = 0
    /// GameTime(0, 1, 1, Night) = 5
    /// GameTime(0, 1, 2, Dawn) = 6
    pub fn to_total_watches(&self) -> u64 {
        let day_zero = self.to_total_days() - 1; // 0-based day count
        day_zero * (WATCHES_PER_DAY as u64) + (self.watch.index() as u64)
    }

    /// Create GameTime from total watches (0-based). [v1.1]
    pub fn from_total_watches(total: u64) -> Self {
        let watch_index = (total % WATCHES_PER_DAY as u64) as u32;
        let day_zero = total / WATCHES_PER_DAY as u64;

        // Convert day_zero back to year/month/day (1-based)
        let total_days_1based = day_zero + 1;
        let mut result = Self::from_total_days(total_days_1based);
        result.watch = Watch::from_index(watch_index);
        result
    }

    /// Advance by one watch. Night→Dawn also advances the day. [v1.1]
    pub fn next_watch(&self) -> Self {
        Self::from_total_watches(self.to_total_watches() + 1)
    }

    /// Number of watches between self and other. [v1.1]
    /// Positive if other is later, negative if earlier.
    pub fn watches_between(&self, other: &GameTime) -> i64 {
        other.to_total_watches() as i64 - self.to_total_watches() as i64
    }
}

// --- Equality: all 4 fields must match ---

impl PartialEq for GameTime {
    fn eq(&self, other: &Self) -> bool {
        self.year == other.year
            && self.month == other.month
            && self.day == other.day
            && self.watch == other.watch
    }
}

impl Eq for GameTime {}

impl std::hash::Hash for GameTime {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_total_watches().hash(state);
    }
}

// --- Ordering: includes watch for sub-day precision ---

impl PartialOrd for GameTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GameTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_total_watches().cmp(&other.to_total_watches())
    }
}

impl fmt::Display for GameTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // e.g., "Y1200-M03-D15 Dawn (Spring)"
        write!(
            f,
            "Y{}-M{:02}-D{:02} {} ({})",
            self.year,
            self.month,
            self.day,
            self.watch,
            self.season()
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "time_tests.rs"]
mod tests;
