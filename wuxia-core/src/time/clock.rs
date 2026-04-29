// wuxia-core/src/time/clock.rs
//
// Game Clock — the heartbeat of the wuxia world.
//
// The clock is the only thing that makes time move forward.
// Every game loop iteration calls tick(), and the rest of the world
// reacts to the events that come out.
//
// [v1.1] tick() now advances by one Watch (시간대), not one day.
//
//   tick()           → 1 시간대 전진 (WatchChanged 항상)
//   tick_day()       → 다음 Dawn까지 전진 (기존 tick() 호환)
//   tick_days(n)     → n일 전진 (기존 tick_days() 호환)
//   tick_until(w)    → 목표 시간대까지 전진 ("묘시까지 수련")
//   tick_watches(n)  → n 시간대 전진
//
// Event ordering per tick (작은 단위 먼저):
//   1. WatchChanged  — 매 tick
//   2. DayPassed     — Night→Dawn 전환 시
//   3. SeasonChanged — 계절 변경 시
//   4. YearPassed    — 연말 전환 시
//
// Think of it as the drummer in a marching army (鼓手):
//   tick() → WatchChanged    (every beat)
//   tick() → DayPassed       (every 6 beats, at dawn)
//   tick() → SeasonChanged   (when season shifts)
//   tick() → YearPassed      (when year ends)
//
// Other domains never advance time themselves.
// They only listen and react.

use serde::{Deserialize, Serialize};

use crate::shared::event::DomainEvent;
use crate::shared::time::{GameTime, Watch};

use super::event::TimeEvent;

// ---------------------------------------------------------------------------
// GameClock
// ---------------------------------------------------------------------------

/// The master clock of the game world.
///
/// Holds the current date+watch and advances it one watch at a time.
/// Each tick may produce multiple events (ordered small → large unit):
///   1. WatchChanged (always)
///   2. DayPassed (when Night→Dawn, i.e. new day starts)
///   3. SeasonChanged (if season differs after day change)
///   4. YearPassed (if year boundary was crossed)
///
/// # Example
/// ```
/// use wuxia_core::time::GameClock;
/// use wuxia_core::shared::{GameTime, DomainEvent};
/// use wuxia_core::shared::time::Watch;
/// use wuxia_core::time::TimeEvent;
///
/// let mut clock = GameClock::new(GameTime::new(1200, 1, 1));
///
/// // Single tick = 1 watch forward
/// let events = clock.tick();
/// assert_eq!(clock.current_time(), GameTime::with_watch(1200, 1, 1, Watch::Morning));
/// assert_eq!(events[0].name(), "WatchChanged");
///
/// // tick_day = advance to next Dawn
/// let events = clock.tick_day();
/// assert_eq!(clock.current_time(), GameTime::new(1200, 1, 2));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameClock {
    current_time: GameTime,
}

impl GameClock {
    /// Create a new clock starting at the given date+watch.
    pub fn new(start: GameTime) -> Self {
        Self {
            current_time: start,
        }
    }

    /// The current date+watch in the game world.
    pub fn current_time(&self) -> GameTime {
        self.current_time
    }

    /// Advance the world by one watch (시간대). [v1.1]
    ///
    /// Returns events in order: WatchChanged → DayPassed → SeasonChanged → YearPassed.
    /// Events are wrapped in DomainEvent::Time(...).
    ///
    /// This is the atomic time unit. All other methods build on this.
    pub fn tick(&mut self) -> Vec<DomainEvent> {
        let old_season = self.current_time.season();
        let was_last_day = self.current_time.is_last_day_of_year();
        let was_night = self.current_time.watch() == Watch::Night;

        // Advance one watch
        self.current_time = self.current_time.next_watch();

        let mut events = Vec::new();

        // 1. WatchChanged — always
        events.push(
            TimeEvent::WatchChanged {
                new_watch: self.current_time.watch(),
                date: self.current_time,
            }
            .into(),
        );

        // 2~4: Only on day boundary (Night→Dawn)
        if was_night {
            // 2. DayPassed
            events.push(
                TimeEvent::DayPassed {
                    date: self.current_time,
                }
                .into(),
            );

            // 3. SeasonChanged — only when season actually changes
            let new_season = self.current_time.season();
            if old_season != new_season {
                events.push(TimeEvent::SeasonChanged { new_season }.into());
            }

            // 4. YearPassed — when we crossed the year boundary
            if was_last_day {
                events.push(
                    TimeEvent::YearPassed {
                        new_year: self.current_time.year(),
                    }
                    .into(),
                );
            }
        }

        events
    }

    /// Advance to the next Dawn (start of a new day). [v1.1]
    ///
    /// From Dawn: advances 6 watches (full day).
    /// From Midday: advances 4 watches (rest of the day).
    /// Equivalent to the old tick() behavior — always ends at Dawn.
    ///
    /// This is the primary backward-compatible method.
    pub fn tick_day(&mut self) -> Vec<DomainEvent> {
        let mut all_events = Vec::new();
        loop {
            let was_night = self.current_time.watch() == Watch::Night;
            all_events.extend(self.tick());
            // Stop after processing Night→Dawn transition
            if was_night {
                break;
            }
        }
        all_events
    }

    /// Advance multiple days, collecting all events. [v1.1 updated]
    ///
    /// Each "day" calls tick_day() which ends at Dawn.
    /// Backward-compatible with old tick_days().
    pub fn tick_days(&mut self, days: u32) -> Vec<DomainEvent> {
        let mut all_events = Vec::new();
        for _ in 0..days {
            all_events.extend(self.tick_day());
        }
        all_events
    }

    /// Advance to the target watch. [v1.1]
    ///
    /// "묘시(Morning)까지 수련한다" = tick_until(Watch::Morning)
    ///
    /// If already at the target watch, advances a full cycle (6 watches)
    /// to reach the same watch on the next day.
    pub fn tick_until(&mut self, target: Watch) -> Vec<DomainEvent> {
        let n = self.current_time.watch().watches_until(target);
        self.tick_watches(n)
    }

    /// Advance by exactly n watches. [v1.1]
    pub fn tick_watches(&mut self, n: u32) -> Vec<DomainEvent> {
        let mut all_events = Vec::new();
        for _ in 0..n {
            all_events.extend(self.tick());
        }
        all_events
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::time::{Season, Watch, WATCHES_PER_DAY, WATCHES_PER_YEAR};
    use crate::time::TimeEvent;

    // =======================================================================
    // Basic tick (1 watch)
    // =======================================================================

    #[test]
    fn tick_advances_one_watch() {
        let mut clock = GameClock::new(GameTime::new(1200, 1, 1)); // Dawn
        clock.tick();
        assert_eq!(
            clock.current_time(),
            GameTime::with_watch(1200, 1, 1, Watch::Morning)
        );
    }

    #[test]
    fn tick_always_emits_watch_changed() {
        let mut clock = GameClock::new(GameTime::new(1200, 6, 15));
        let events = clock.tick();

        assert_eq!(events.len(), 1); // Only WatchChanged
        match &events[0] {
            DomainEvent::Time(TimeEvent::WatchChanged { new_watch, date }) => {
                assert_eq!(*new_watch, Watch::Morning);
                assert_eq!(*date, GameTime::with_watch(1200, 6, 15, Watch::Morning));
            }
            other => panic!("Expected WatchChanged, got {:?}", other),
        }
    }

    #[test]
    fn tick_mid_day_no_day_passed() {
        // Dawn→Morning: only WatchChanged, no DayPassed
        let mut clock = GameClock::new(GameTime::new(1200, 4, 15));
        let events = clock.tick();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name(), "WatchChanged");
    }

    #[test]
    fn tick_night_to_dawn_emits_day_passed() {
        let mut clock =
            GameClock::new(GameTime::with_watch(1200, 4, 15, Watch::Night));
        let events = clock.tick();

        assert_eq!(clock.current_time(), GameTime::new(1200, 4, 16)); // Dawn

        // WatchChanged + DayPassed
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].name(), "WatchChanged");
        assert_eq!(events[1].name(), "DayPassed");

        match &events[1] {
            DomainEvent::Time(TimeEvent::DayPassed { date }) => {
                assert_eq!(*date, GameTime::new(1200, 4, 16));
            }
            other => panic!("Expected DayPassed, got {:?}", other),
        }
    }

    #[test]
    fn six_ticks_equal_one_day() {
        let mut clock = GameClock::new(GameTime::new(1200, 3, 15)); // Dawn
        let mut all_events = Vec::new();
        for _ in 0..WATCHES_PER_DAY {
            all_events.extend(clock.tick());
        }
        assert_eq!(clock.current_time(), GameTime::new(1200, 3, 16)); // Dawn of next day

        let watch_count = all_events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::WatchChanged { .. })))
            .count();
        let day_count = all_events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::DayPassed { .. })))
            .count();

        assert_eq!(watch_count, 6);
        assert_eq!(day_count, 1);
    }

    // =======================================================================
    // Season changes
    // =======================================================================

    #[test]
    fn season_change_on_night_to_dawn_boundary() {
        // Feb 30, Night → Mar 1, Dawn: Winter → Spring
        let mut clock =
            GameClock::new(GameTime::with_watch(1200, 2, 30, Watch::Night));
        let events = clock.tick();

        // WatchChanged + DayPassed + SeasonChanged
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].name(), "WatchChanged");
        assert_eq!(events[1].name(), "DayPassed");
        assert_eq!(events[2].name(), "SeasonChanged");

        match &events[2] {
            DomainEvent::Time(TimeEvent::SeasonChanged { new_season }) => {
                assert_eq!(*new_season, Season::Spring);
            }
            other => panic!("Expected SeasonChanged, got {:?}", other),
        }
    }

    #[test]
    fn no_season_change_within_same_season() {
        // Dec 30, Night → Jan 1: Winter → Winter (no season change)
        let mut clock =
            GameClock::new(GameTime::with_watch(1200, 12, 30, Watch::Night));
        let events = clock.tick();

        let season_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::SeasonChanged { .. })))
            .collect();
        assert_eq!(season_events.len(), 0);
    }

    // =======================================================================
    // Year boundary
    // =======================================================================

    #[test]
    fn year_passed_on_night_to_dawn() {
        // Dec 30, Night → Jan 1 of next year
        let mut clock =
            GameClock::new(GameTime::with_watch(1200, 12, 30, Watch::Night));
        let events = clock.tick();

        assert_eq!(clock.current_time(), GameTime::new(1201, 1, 1));

        // WatchChanged + DayPassed + YearPassed (no SeasonChanged: winter→winter)
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].name(), "WatchChanged");
        assert_eq!(events[1].name(), "DayPassed");
        assert_eq!(events[2].name(), "YearPassed");

        match &events[2] {
            DomainEvent::Time(TimeEvent::YearPassed { new_year }) => {
                assert_eq!(*new_year, 1201);
            }
            other => panic!("Expected YearPassed, got {:?}", other),
        }
    }

    #[test]
    fn no_year_passed_mid_year() {
        let mut clock =
            GameClock::new(GameTime::with_watch(1200, 6, 15, Watch::Night));
        let events = clock.tick();

        let year_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::YearPassed { .. })))
            .collect();
        assert_eq!(year_events.len(), 0);
    }

    // =======================================================================
    // Event ordering
    // =======================================================================

    #[test]
    fn event_order_watch_then_day_then_season() {
        // Feb 30, Night → Mar 1: season change day
        let mut clock =
            GameClock::new(GameTime::with_watch(1200, 2, 30, Watch::Night));
        let events = clock.tick();

        assert_eq!(events[0].name(), "WatchChanged");
        assert_eq!(events[1].name(), "DayPassed");
        assert_eq!(events[2].name(), "SeasonChanged");
    }

    #[test]
    fn event_order_watch_then_day_then_year() {
        // Dec 30, Night → Jan 1 (winter→winter)
        let mut clock =
            GameClock::new(GameTime::with_watch(1200, 12, 30, Watch::Night));
        let events = clock.tick();

        assert_eq!(events[0].name(), "WatchChanged");
        assert_eq!(events[1].name(), "DayPassed");
        assert_eq!(events[2].name(), "YearPassed");
    }

    // =======================================================================
    // tick_day — backward compatible "advance one full day"
    // =======================================================================

    #[test]
    fn tick_day_from_dawn() {
        let mut clock = GameClock::new(GameTime::new(1200, 1, 1));
        let events = clock.tick_day();
        assert_eq!(clock.current_time(), GameTime::new(1200, 1, 2));

        // 6 WatchChanged + 1 DayPassed
        let watch_count = events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::WatchChanged { .. })))
            .count();
        let day_count = events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::DayPassed { .. })))
            .count();
        assert_eq!(watch_count, 6);
        assert_eq!(day_count, 1);
    }

    #[test]
    fn tick_day_from_midday() {
        // Midday → next Dawn (4 ticks: Afternoon, Evening, Night→Dawn)
        let mut clock =
            GameClock::new(GameTime::with_watch(1200, 3, 15, Watch::Midday));
        let events = clock.tick_day();
        assert_eq!(clock.current_time(), GameTime::new(1200, 3, 16));

        let watch_count = events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::WatchChanged { .. })))
            .count();
        assert_eq!(watch_count, 4); // Afternoon, Evening, Night, Dawn(next day)
    }

    #[test]
    fn tick_day_from_night() {
        // Night → Dawn (1 tick)
        let mut clock =
            GameClock::new(GameTime::with_watch(1200, 3, 15, Watch::Night));
        let events = clock.tick_day();
        assert_eq!(clock.current_time(), GameTime::new(1200, 3, 16));

        let watch_count = events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::WatchChanged { .. })))
            .count();
        assert_eq!(watch_count, 1);
    }

    #[test]
    fn tick_day_year_boundary() {
        let mut clock = GameClock::new(GameTime::new(1200, 12, 30));
        let events = clock.tick_day();
        assert_eq!(clock.current_time(), GameTime::new(1201, 1, 1));

        let has_year = events
            .iter()
            .any(|e| matches!(e, DomainEvent::Time(TimeEvent::YearPassed { .. })));
        assert!(has_year);
    }

    // =======================================================================
    // tick_days — backward compatible multi-day
    // =======================================================================

    #[test]
    fn tick_days_zero() {
        let mut clock = GameClock::new(GameTime::new(1200, 1, 1));
        let events = clock.tick_days(0);
        assert!(events.is_empty());
        assert_eq!(clock.current_time(), GameTime::new(1200, 1, 1));
    }

    #[test]
    fn tick_days_five() {
        let mut clock = GameClock::new(GameTime::new(1200, 1, 1));
        let events = clock.tick_days(5);
        assert_eq!(clock.current_time(), GameTime::new(1200, 1, 6));

        let day_count = events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::DayPassed { .. })))
            .count();
        assert_eq!(day_count, 5);
    }

    #[test]
    fn tick_days_360_full_year() {
        let mut clock = GameClock::new(GameTime::new(1200, 1, 1));
        let all_events = clock.tick_days(360);

        assert_eq!(clock.current_time(), GameTime::new(1201, 1, 1));

        let watch_count = all_events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::WatchChanged { .. })))
            .count();
        let day_count = all_events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::DayPassed { .. })))
            .count();
        let season_count = all_events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::SeasonChanged { .. })))
            .count();
        let year_count = all_events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::YearPassed { .. })))
            .count();

        assert_eq!(watch_count, WATCHES_PER_YEAR as usize); // 2160
        assert_eq!(day_count, 360);
        assert_eq!(season_count, 4);
        assert_eq!(year_count, 1);
    }

    #[test]
    fn season_changes_at_correct_months() {
        let mut clock = GameClock::new(GameTime::new(1200, 1, 1));

        let mut season_transitions = Vec::new();
        for _ in 0..360 {
            let events = clock.tick_day();
            for event in &events {
                if let DomainEvent::Time(TimeEvent::SeasonChanged { new_season }) = event {
                    season_transitions.push(*new_season);
                }
            }
        }

        // Starting in January (Winter):
        // Feb→Mar: Winter→Spring
        // May→Jun: Spring→Summer
        // Aug→Sep: Summer→Autumn
        // Nov→Dec: Autumn→Winter
        assert_eq!(
            season_transitions,
            vec![Season::Spring, Season::Summer, Season::Autumn, Season::Winter]
        );
    }

    // =======================================================================
    // tick_until — "묘시까지 수련한다"
    // =======================================================================

    #[test]
    fn tick_until_next_watch() {
        // Dawn → tick_until(Morning) = 1 tick
        let mut clock = GameClock::new(GameTime::new(1200, 3, 15));
        let events = clock.tick_until(Watch::Morning);
        assert_eq!(
            clock.current_time(),
            GameTime::with_watch(1200, 3, 15, Watch::Morning)
        );

        let watch_count = events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::WatchChanged { .. })))
            .count();
        assert_eq!(watch_count, 1);
    }

    #[test]
    fn tick_until_wraps_around() {
        // Afternoon → tick_until(Morning) = 4 ticks (Evening, Night, Dawn(+day), Morning)
        let mut clock =
            GameClock::new(GameTime::with_watch(1200, 3, 15, Watch::Afternoon));
        let events = clock.tick_until(Watch::Morning);
        assert_eq!(
            clock.current_time(),
            GameTime::with_watch(1200, 3, 16, Watch::Morning)
        );

        let watch_count = events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::WatchChanged { .. })))
            .count();
        assert_eq!(watch_count, 4);
    }

    #[test]
    fn tick_until_same_watch_advances_full_cycle() {
        // Dawn → tick_until(Dawn) = 6 ticks (full day cycle)
        let mut clock = GameClock::new(GameTime::new(1200, 3, 15));
        let events = clock.tick_until(Watch::Dawn);
        assert_eq!(clock.current_time(), GameTime::new(1200, 3, 16));

        let watch_count = events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::WatchChanged { .. })))
            .count();
        assert_eq!(watch_count, 6);
    }

    #[test]
    fn tick_until_night_from_evening() {
        // Evening → tick_until(Night) = 1 tick
        let mut clock =
            GameClock::new(GameTime::with_watch(1200, 3, 15, Watch::Evening));
        clock.tick_until(Watch::Night);
        assert_eq!(
            clock.current_time(),
            GameTime::with_watch(1200, 3, 15, Watch::Night)
        );
    }

    // =======================================================================
    // tick_watches — advance by n watches
    // =======================================================================

    #[test]
    fn tick_watches_three() {
        let mut clock = GameClock::new(GameTime::new(1200, 3, 15)); // Dawn
        let events = clock.tick_watches(3);
        assert_eq!(
            clock.current_time(),
            GameTime::with_watch(1200, 3, 15, Watch::Afternoon)
        );

        let watch_count = events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::WatchChanged { .. })))
            .count();
        assert_eq!(watch_count, 3);
    }

    #[test]
    fn tick_watches_zero() {
        let mut clock = GameClock::new(GameTime::new(1200, 1, 1));
        let events = clock.tick_watches(0);
        assert!(events.is_empty());
        assert_eq!(clock.current_time(), GameTime::new(1200, 1, 1));
    }

    #[test]
    fn tick_watches_twelve_is_two_days() {
        let mut clock = GameClock::new(GameTime::new(1200, 3, 15));
        clock.tick_watches(12);
        assert_eq!(clock.current_time(), GameTime::new(1200, 3, 17));
    }

    // =======================================================================
    // Serialization
    // =======================================================================

    #[test]
    fn serialization_roundtrip() {
        let original = GameClock::new(GameTime::with_watch(1200, 7, 22, Watch::Evening));
        let json = serde_json::to_string(&original).unwrap();
        let restored: GameClock = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.current_time(), original.current_time());
    }
}
