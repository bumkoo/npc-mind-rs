// wuxia-core/src/time/event.rs
//
// Time Domain Events — 시간 도메인에서 발생하는 이벤트들.
//
// [v1.1] WatchChanged 추가. tick() 단위가 1일→1시간대로 변경됨.
//
// TimeEvent는 GameClock::tick()에서만 생성된다.
// Application Service가 이것을 DomainEvent::Time(...)으로 감싸서 전달한다.
//
// 이벤트 종류 (작은 단위 → 큰 단위):
//   WatchChanged  → 매 tick마다 (가장 빈번) [v1.1 신설]
//   DayPassed     → Night→Dawn 전환 시 (6 tick마다)
//   SeasonChanged → 계절이 바뀔 때 (연 4회)
//   YearPassed    → 연말에 (연 1회)

use serde::{Deserialize, Serialize};

use crate::shared::time::{GameTime, Season, Watch};

/// 시간 도메인에서 발생하는 이벤트들.
///
/// 작은 단위 → 큰 단위 순서로 정의한다.
///
/// # Example
/// ```
/// use wuxia_core::time::TimeEvent;
/// use wuxia_core::shared::GameTime;
/// use wuxia_core::shared::time::Watch;
///
/// let event = TimeEvent::WatchChanged {
///     new_watch: Watch::Morning,
///     date: GameTime::with_watch(1200, 3, 15, Watch::Morning),
/// };
/// assert_eq!(event.name(), "WatchChanged");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimeEvent {
    /// 시간대가 바뀌었다. GameClock::tick()마다 발생. [v1.1 신설]
    ///
    /// 구독자:
    ///   - (향후) 시간대별 수련 보너스 적용
    ///   - (향후) NPC 행동 패턴 변경
    WatchChanged { new_watch: Watch, date: GameTime },

    /// 하루가 지났다. Night→Dawn 전환 시 발생 (6 tick마다).
    ///
    /// 구독자:
    ///   - 캐릭터: 피로 자연 회복
    ///   - 캐릭터: 부상 회복 진행
    ///   - 성장: 일일 수련 결과 적용
    DayPassed { date: GameTime },

    /// 계절이 바뀌었다. 연 4회 발생.
    ///
    /// 구독자:
    ///   - 전투: 지형/날씨 보정 업데이트
    ///   - 서사: 계절 이벤트 (예: 겨울 매화검회)
    SeasonChanged { new_season: Season },

    /// 새해가 시작되었다. 연 1회 발생.
    ///
    /// 구독자:
    ///   - Application Service: 캐릭터 나이 +1 트리거
    ///   - 성장: 연간 쇠퇴/성장 처리
    ///   - 심리: 성격 미세 변화
    YearPassed { new_year: u32 },
}

use crate::shared::event_macros::impl_event_name;

impl_event_name!(TimeEvent {
    WatchChanged => "WatchChanged",
    DayPassed => "DayPassed",
    SeasonChanged => "SeasonChanged",
    YearPassed => "YearPassed",
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_changed_event() {
        let event = TimeEvent::WatchChanged {
            new_watch: Watch::Morning,
            date: GameTime::with_watch(1200, 3, 15, Watch::Morning),
        };
        assert_eq!(event.name(), "WatchChanged");
    }

    #[test]
    fn day_passed_event() {
        let event = TimeEvent::DayPassed {
            date: GameTime::new(1200, 3, 1),
        };
        assert_eq!(event.name(), "DayPassed");
    }

    #[test]
    fn season_changed_event() {
        let event = TimeEvent::SeasonChanged {
            new_season: Season::Spring,
        };
        assert_eq!(event.name(), "SeasonChanged");
    }

    #[test]
    fn year_passed_event() {
        let event = TimeEvent::YearPassed { new_year: 1201 };
        assert_eq!(event.name(), "YearPassed");
    }

    #[test]
    fn clone_and_eq() {
        let a = TimeEvent::YearPassed { new_year: 1200 };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn watch_changed_clone_and_eq() {
        let a = TimeEvent::WatchChanged {
            new_watch: Watch::Night,
            date: GameTime::with_watch(1200, 6, 15, Watch::Night),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn serialization_roundtrip() {
        let events = vec![
            TimeEvent::WatchChanged {
                new_watch: Watch::Midday,
                date: GameTime::with_watch(1200, 6, 15, Watch::Midday),
            },
            TimeEvent::DayPassed {
                date: GameTime::new(1200, 6, 15),
            },
            TimeEvent::SeasonChanged {
                new_season: Season::Autumn,
            },
            TimeEvent::YearPassed { new_year: 1201 },
        ];
        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let restored: TimeEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(event, restored);
        }
    }
}
