// wuxia-core/src/application/time_character.rs
//
// TimeCharacterService — "시간이 흐르면 캐릭터가 나이를 먹는다"
//
// [리팩터링 v2] DomainEvent 매칭 패턴이 변경되었다:
//   이전: DomainEvent::YearPassed { .. }
//   이후: DomainEvent::Time(TimeEvent::YearPassed { .. })
//
// 나머지 구조는 동일:
//   - Application Service는 얇다 (thin)
//   - 이벤트를 받아서 적절한 도메인 메서드를 호출
//   - 결과 이벤트를 수집하여 반환

use crate::character::Character;
use crate::shared::event::DomainEvent;
use crate::time::TimeEvent;

// ---------------------------------------------------------------------------
// TimeCharacterService
// ---------------------------------------------------------------------------

/// Application Service: 시간 이벤트 → 캐릭터 변화 조율
///
/// # Example
/// ```
/// use wuxia_core::shared::{CharacterId, GameTime, DomainEvent};
/// use wuxia_core::character::{Character, Gender, CharacterRole};
/// use wuxia_core::time::GameClock;
/// use wuxia_core::application::TimeCharacterService;
///
/// let service = TimeCharacterService::new();
/// let mut characters = vec![
///     Character::new(
///         CharacterId::new(1), "령호충".into(), None,
///         Gender::Male, 1175, 25, CharacterRole::Npc,
///     ),
///     Character::new(
///         CharacterId::new(2), "임영영".into(), None,
///         Gender::Female, 1178, 22, CharacterRole::Npc,
///     ),
/// ];
///
/// let mut clock = GameClock::new(GameTime::new(1200, 1, 1));
/// let time_events = clock.tick_days(360);
/// let char_events = service.process_time_events(&time_events, &mut characters);
///
/// assert_eq!(characters[0].age(), 26);
/// assert_eq!(characters[1].age(), 23);
/// ```
pub struct TimeCharacterService;

impl TimeCharacterService {
    pub fn new() -> Self {
        Self
    }

    /// 범용 시간 이벤트 처리기.
    ///
    /// 시간 도메인에서 발생한 모든 이벤트를 받아서
    /// 캐릭터 관련 처리가 필요한 이벤트만 골라서 처리한다.
    ///
    /// 현재 처리하는 이벤트:
    ///   - `TimeEvent::YearPassed` → 모든 살아있는 캐릭터 나이 +1
    ///
    /// 현재 무시하는 이벤트 (향후 확장 지점):
    ///   - `TimeEvent::DayPassed`     → (Phase 3: 일일 행동 스케줄링)
    ///   - `TimeEvent::SeasonChanged` → (Phase 3: 계절 이벤트 트리거)
    ///   - `DomainEvent::Character`   → 캐릭터 이벤트는 여기서 처리 안 함
    pub fn process_time_events(
        &self,
        events: &[DomainEvent],
        characters: &mut [Character],
    ) -> Vec<DomainEvent> {
        let mut result_events = Vec::new();

        for event in events {
            match event {
                DomainEvent::Time(TimeEvent::YearPassed { .. }) => {
                    let aging_events = self.handle_year_passed(characters);
                    result_events.extend(aging_events);
                }
                // 향후 확장 지점:
                // DomainEvent::Time(TimeEvent::DayPassed { .. }) => { /* Phase 3 */ }
                // DomainEvent::Time(TimeEvent::SeasonChanged { .. }) => { /* Phase 3 */ }
                _ => {
                    // 캐릭터와 무관한 이벤트는 무시
                }
            }
        }

        result_events
    }

    /// YearPassed 이벤트 전용 핸들러.
    ///
    /// 모든 살아있는 캐릭터를 순회하며 `age_one_year()`를 호출한다.
    /// 각 캐릭터에서 발생한 이벤트를 모두 수집하여 반환한다.
    ///
    /// # Example
    /// ```
    /// use wuxia_core::shared::CharacterId;
    /// use wuxia_core::character::{Character, Gender, CharacterRole};
    /// use wuxia_core::application::TimeCharacterService;
    ///
    /// let service = TimeCharacterService::new();
    /// let mut chars = vec![
    ///     Character::new(
    ///         CharacterId::new(1), "A".into(), None,
    ///         Gender::Male, 1170, 30, CharacterRole::Npc,
    ///     ),
    /// ];
    ///
    /// let events = service.handle_year_passed(&mut chars);
    /// assert_eq!(chars[0].age(), 31);
    /// assert!(events.len() >= 1); // Aged (+ maybe LifeStageChanged)
    /// ```
    pub fn handle_year_passed(
        &self,
        characters: &mut [Character],
    ) -> Vec<DomainEvent> {
        let mut all_events = Vec::new();

        for character in characters.iter_mut() {
            if character.is_alive() {
                let events = character.age_one_year();
                all_events.extend(events);
            }
        }

        all_events
    }
}

impl Default for TimeCharacterService {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::{CharacterEvent, CharacterRole, Gender, LifeStage};
    use crate::shared::id::CharacterId;
    use crate::shared::time::GameTime;
    use crate::time::{GameClock, TimeEvent};

    /// Helper: 테스트용 캐릭터 생성
    fn make_char(id: u64, name: &str, age: u32, role: CharacterRole) -> Character {
        Character::new(
            CharacterId::new(id),
            name.to_string(),
            None,
            Gender::Male,
            1200 - age as u32,
            age,
            role,
        )
    }

    // =======================================================================
    // handle_year_passed 테스트
    // =======================================================================

    #[test]
    fn handle_year_passed_ages_all_characters() {
        let service = TimeCharacterService::new();
        let mut characters = vec![
            make_char(1, "령호충", 25, CharacterRole::Npc),
            make_char(2, "임영영", 22, CharacterRole::Npc),
            make_char(3, "영호", 30, CharacterRole::Player),
        ];

        let events = service.handle_year_passed(&mut characters);

        assert_eq!(characters[0].age(), 26);
        assert_eq!(characters[1].age(), 23);
        assert_eq!(characters[2].age(), 31);

        let aged_count = events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Character(CharacterEvent::Aged { .. })))
            .count();
        assert_eq!(aged_count, 3);
    }

    #[test]
    fn handle_year_passed_detects_life_stage_change() {
        let service = TimeCharacterService::new();
        let mut characters = vec![
            make_char(1, "영호", 32, CharacterRole::Player),
        ];

        let events = service.handle_year_passed(&mut characters);

        let stage_changes: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Character(CharacterEvent::LifeStageChanged { .. })))
            .collect();

        assert_eq!(stage_changes.len(), 1);
        match &stage_changes[0] {
            DomainEvent::Character(CharacterEvent::LifeStageChanged { from, to, .. }) => {
                assert_eq!(*from, LifeStage::Youth);
                assert_eq!(*to, LifeStage::Prime);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn handle_year_passed_empty_characters() {
        let service = TimeCharacterService::new();
        let mut characters: Vec<Character> = vec![];

        let events = service.handle_year_passed(&mut characters);
        assert!(events.is_empty());
    }

    #[test]
    fn handle_year_passed_includes_companions() {
        let service = TimeCharacterService::new();
        let mut characters = vec![
            make_char(1, "양과", 25, CharacterRole::Npc),
            Character::new(
                CharacterId::new(99),
                "신조".to_string(),
                None,
                Gender::Male,
                1140,
                60,
                CharacterRole::Companion,
            ),
        ];

        let events = service.handle_year_passed(&mut characters);

        assert_eq!(characters[0].age(), 26);
        assert_eq!(characters[1].age(), 61);

        let aged_count = events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Character(CharacterEvent::Aged { .. })))
            .count();
        assert_eq!(aged_count, 2);
    }

    // =======================================================================
    // process_time_events 테스트
    // =======================================================================

    #[test]
    fn process_time_events_handles_year_passed() {
        let service = TimeCharacterService::new();
        let mut characters = vec![
            make_char(1, "령호충", 25, CharacterRole::Npc),
        ];

        let time_events = vec![
            DomainEvent::Time(TimeEvent::YearPassed { new_year: 1201 }),
        ];
        let char_events = service.process_time_events(&time_events, &mut characters);

        assert_eq!(characters[0].age(), 26);
        assert!(!char_events.is_empty());
    }

    #[test]
    fn process_time_events_ignores_non_year_events() {
        let service = TimeCharacterService::new();
        let mut characters = vec![
            make_char(1, "령호충", 25, CharacterRole::Npc),
        ];

        let time_events = vec![
            DomainEvent::Time(TimeEvent::DayPassed {
                date: GameTime::new(1200, 3, 15),
            }),
            DomainEvent::Time(TimeEvent::SeasonChanged {
                new_season: crate::shared::time::Season::Spring,
            }),
        ];

        let char_events = service.process_time_events(&time_events, &mut characters);

        assert_eq!(characters[0].age(), 25);
        assert!(char_events.is_empty());
    }

    #[test]
    fn process_time_events_handles_mixed_events() {
        let service = TimeCharacterService::new();
        let mut characters = vec![
            make_char(1, "령호충", 25, CharacterRole::Npc),
            make_char(2, "임영영", 22, CharacterRole::Npc),
        ];

        let time_events = vec![
            DomainEvent::Time(TimeEvent::DayPassed {
                date: GameTime::new(1200, 12, 29),
            }),
            DomainEvent::Time(TimeEvent::DayPassed {
                date: GameTime::new(1200, 12, 30),
            }),
            DomainEvent::Time(TimeEvent::YearPassed { new_year: 1201 }),
            DomainEvent::Time(TimeEvent::DayPassed {
                date: GameTime::new(1201, 1, 1),
            }),
        ];

        let char_events = service.process_time_events(&time_events, &mut characters);

        assert_eq!(characters[0].age(), 26);
        assert_eq!(characters[1].age(), 23);

        let aged_count = char_events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Character(CharacterEvent::Aged { .. })))
            .count();
        assert_eq!(aged_count, 2);
    }

    // =======================================================================
    // GameClock 통합 테스트 (clock → events → service → characters)
    // =======================================================================

    #[test]
    fn integration_clock_tick_360_ages_characters() {
        let service = TimeCharacterService::new();

        let mut characters = vec![
            make_char(1, "령호충", 25, CharacterRole::Npc),
            make_char(2, "임영영", 22, CharacterRole::Npc),
            make_char(3, "악불군", 54, CharacterRole::Npc),
        ];

        let mut clock = GameClock::new(GameTime::new(1200, 1, 1));
        let time_events = clock.tick_days(360);
        let char_events = service.process_time_events(&time_events, &mut characters);

        assert_eq!(characters[0].age(), 26, "령호충: 25→26");
        assert_eq!(characters[1].age(), 23, "임영영: 22→23");
        assert_eq!(characters[2].age(), 55, "악불군: 54→55");

        let aged_count = char_events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Character(CharacterEvent::Aged { .. })))
            .count();
        assert_eq!(aged_count, 3);

        let stage_changes: Vec<_> = char_events
            .iter()
            .filter_map(|e| match e {
                DomainEvent::Character(CharacterEvent::LifeStageChanged {
                    character_id,
                    from,
                    to,
                }) => Some((*character_id, *from, *to)),
                _ => None,
            })
            .collect();

        assert_eq!(stage_changes.len(), 1);
        assert_eq!(stage_changes[0].0, CharacterId::new(3));
        assert_eq!(stage_changes[0].1, LifeStage::Prime);
        assert_eq!(stage_changes[0].2, LifeStage::Middle);
    }

    #[test]
    fn integration_multi_year_aging() {
        let service = TimeCharacterService::new();

        let mut characters = vec![
            make_char(1, "장삼봉", 67, CharacterRole::Npc),
        ];

        let mut clock = GameClock::new(GameTime::new(1200, 1, 1));

        let mut all_char_events = Vec::new();
        for _ in 0..3 {
            let time_events = clock.tick_days(360);
            let char_events =
                service.process_time_events(&time_events, &mut characters);
            all_char_events.extend(char_events);
        }

        assert_eq!(characters[0].age(), 70);
        assert_eq!(characters[0].life_stage(), LifeStage::Elder);

        let aged_count = all_char_events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Character(CharacterEvent::Aged { .. })))
            .count();
        assert_eq!(aged_count, 3);

        let stage_count = all_char_events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Character(CharacterEvent::LifeStageChanged { .. })))
            .count();
        assert_eq!(stage_count, 1);
    }

    #[test]
    fn integration_no_double_aging_per_year() {
        let service = TimeCharacterService::new();
        let mut characters = vec![
            make_char(1, "테스트", 20, CharacterRole::Npc),
        ];

        let mut clock = GameClock::new(GameTime::new(1200, 1, 1));
        let time_events = clock.tick_days(360);

        let year_count = time_events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Time(TimeEvent::YearPassed { .. })))
            .count();
        assert_eq!(year_count, 1, "360일에 YearPassed는 정확히 1회");

        let char_events = service.process_time_events(&time_events, &mut characters);

        assert_eq!(characters[0].age(), 21, "나이는 정확히 +1");

        let aged_count = char_events
            .iter()
            .filter(|e| matches!(e, DomainEvent::Character(CharacterEvent::Aged { .. })))
            .count();
        assert_eq!(aged_count, 1, "CharacterAged도 정확히 1회");
    }

    // =======================================================================
    // Default trait
    // =======================================================================

    #[test]
    fn default_creates_service() {
        let service = TimeCharacterService::default();
        let mut characters = vec![
            make_char(1, "테스트", 20, CharacterRole::Npc),
        ];
        let events = service.handle_year_passed(&mut characters);
        assert_eq!(characters[0].age(), 21);
        assert_eq!(events.len(), 1);
    }
}
