// wuxia-core/src/experience/handlers/character_handler.rs
//
// ① 캐릭터 핸들러 — 피로/부상 계산.
//
// "몸이 먼저" — 물리적 제약이 가장 먼저 적용된다.
//
// 구독하는 경험:
//   Training → 피로 추가 (intensity × duration / 3)
//   Combat → 피로 추가 (고정 20)
//   Travel → 피로 추가 (duration × 3)
//   Rest → 피로 회복 (recovery × 100)
//   TimePassage → 일일 자연 회복 (DAILY_REST_RECOVERY)
//   기타 → no-op

use std::collections::HashMap;

use crate::character::Character;
use crate::character::fatigue::DAILY_REST_RECOVERY;
use crate::shared::event::DomainEvent;
use crate::shared::id::CharacterId;

use super::super::event::ExperienceEvent;
use super::super::handler::{EventHandler, HandlerResult, ProcessingContext};

// ---------------------------------------------------------------------------
// CharacterHandler
// ---------------------------------------------------------------------------

/// ① 캐릭터 핸들러 — 피로/부상 계산.
///
/// `HashMap<CharacterId, Character>`를 소유하고,
/// ExperienceEvent에 따라 `add_fatigue()` / `recover_fatigue()`를 호출한다.
pub struct CharacterHandler {
    characters: HashMap<CharacterId, Character>,
}

impl CharacterHandler {
    /// 캐릭터 맵으로 핸들러 생성.
    pub fn new(characters: HashMap<CharacterId, Character>) -> Self {
        Self { characters }
    }

    /// 특정 캐릭터 참조 (읽기).
    pub fn get(&self, id: &CharacterId) -> Option<&Character> {
        self.characters.get(id)
    }

    /// 모든 캐릭터를 소비하여 반환.
    pub fn into_characters(self) -> HashMap<CharacterId, Character> {
        self.characters
    }

    /// 수련 피로 = intensity × duration / 3 (MVP 공식)
    fn training_fatigue(intensity: u32, duration: u32) -> u32 {
        (intensity * duration) / 3
    }

    /// 전투 피로 = 고정 20 (MVP)
    const COMBAT_FATIGUE: u32 = 20;

    /// 여행 피로 = duration × 3
    fn travel_fatigue(duration: u32) -> u32 {
        duration * 3
    }

    /// 휴식 회복 = recovery × 100 (recovery는 0.0~1.0)
    fn rest_recovery(recovery: f32) -> u32 {
        (recovery.clamp(0.0, 1.0) * 100.0).round() as u32
    }
}

impl EventHandler for CharacterHandler {
    fn handle_event(
        &mut self,
        event: &ExperienceEvent,
        _ctx: &ProcessingContext,
    ) -> HandlerResult {
        let subject = event.header().subject;

        let character = match self.characters.get_mut(&subject) {
            Some(c) => c,
            None => return HandlerResult::empty(),
        };

        let side_effects: Vec<DomainEvent> = match event {
            ExperienceEvent::Training { intensity, duration, .. } => {
                character.add_fatigue(Self::training_fatigue(*intensity, *duration))
            }
            ExperienceEvent::Combat { .. } => {
                character.add_fatigue(Self::COMBAT_FATIGUE)
            }
            ExperienceEvent::Travel { duration, .. } => {
                character.add_fatigue(Self::travel_fatigue(*duration))
            }
            ExperienceEvent::Rest { recovery, .. } => {
                character.recover_fatigue(Self::rest_recovery(*recovery))
            }
            ExperienceEvent::TimePassage { .. } => {
                character.recover_fatigue(DAILY_REST_RECOVERY)
            }
            _ => Vec::new(),
        };

        HandlerResult::with_effects(side_effects)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::CharacterEvent;
    use crate::experience::event::ExperienceHeader;
    use crate::shared::id::{ExperienceId, LocationId, MartialArtId};
    use crate::shared::time::GameTime;
    use crate::test_fixtures::make_default_character;

    fn make_header(subject_id: u64) -> ExperienceHeader {
        ExperienceHeader::new(
            ExperienceId::new(1),
            CharacterId::new(subject_id),
            GameTime::new(1200, 3, 15),
            LocationId::new(10),
            5.0,
        )
    }

    fn make_handler_with_char(id: u64) -> CharacterHandler {
        let mut map = HashMap::new();
        map.insert(CharacterId::new(id), make_default_character(id));
        CharacterHandler::new(map)
    }

    fn ctx() -> ProcessingContext {
        ProcessingContext::new()
    }

    // --- 수련 피로 ---

    #[test]
    fn training_adds_fatigue() {
        let mut handler = make_handler_with_char(1);
        let event = ExperienceEvent::Training {
            header: make_header(1),
            skill: MartialArtId::new(1),
            method: "대련".to_string(),
            mentor: None,
            companion: None,
            duration: 3,
            intensity: 6,
        };

        let result = handler.handle_event(&event, &ctx());

        // intensity(6) × duration(3) / 3 = 6 피로
        assert_eq!(result.side_effects.len(), 1);
        assert!(result.side_effects.iter().any(|e| {
            matches!(e, DomainEvent::Character(CharacterEvent::FatigueChanged {
                new_fatigue: 6, ..
            }))
        }));
        assert_eq!(handler.get(&CharacterId::new(1)).unwrap().fatigue(), 6);
    }

    #[test]
    fn training_fatigue_formula_values() {
        // intensity=10, duration=6 → 10×6/3 = 20
        assert_eq!(CharacterHandler::training_fatigue(10, 6), 20);
        // intensity=1, duration=1 → 1×1/3 = 0 (정수 나눗셈)
        assert_eq!(CharacterHandler::training_fatigue(1, 1), 0);
        // intensity=5, duration=3 → 5×3/3 = 5
        assert_eq!(CharacterHandler::training_fatigue(5, 3), 5);
    }

    #[test]
    fn zero_intensity_training_no_op() {
        let mut handler = make_handler_with_char(1);
        let event = ExperienceEvent::Training {
            header: make_header(1),
            skill: MartialArtId::new(1),
            method: String::new(),
            mentor: None,
            companion: None,
            duration: 0,
            intensity: 0,
        };

        let result = handler.handle_event(&event, &ctx());

        // 0 피로 추가 → no-op
        assert!(result.side_effects.is_empty());
    }

    // --- 전투 피로 ---

    #[test]
    fn combat_adds_fixed_fatigue() {
        let mut handler = make_handler_with_char(1);
        let event = ExperienceEvent::Combat {
            header: make_header(1),
            opponent: CharacterId::new(2),
            result: crate::experience::CombatResult::Victory,
            technique_used: None,
            technique_faced: None,
        };

        let result = handler.handle_event(&event, &ctx());

        assert_eq!(result.side_effects.len(), 1);
        assert!(result.side_effects.iter().any(|e| {
            matches!(e, DomainEvent::Character(CharacterEvent::FatigueChanged {
                new_fatigue: 20, ..
            }))
        }));
    }

    // --- 여행 피로 ---

    #[test]
    fn travel_adds_duration_fatigue() {
        let mut handler = make_handler_with_char(1);
        let event = ExperienceEvent::Travel {
            header: make_header(1),
            destination: LocationId::new(20),
            companion: None,
            duration: 4,
        };

        let result = handler.handle_event(&event, &ctx());

        // duration(4) × 3 = 12 피로
        assert_eq!(result.side_effects.len(), 1);
        assert!(result.side_effects.iter().any(|e| {
            matches!(e, DomainEvent::Character(CharacterEvent::FatigueChanged {
                new_fatigue: 12, ..
            }))
        }));
    }

    // --- 휴식 회복 ---

    #[test]
    fn rest_recovers_fatigue() {
        let mut handler = make_handler_with_char(1);

        // 먼저 피로를 50으로 올림
        let combat = ExperienceEvent::Combat {
            header: make_header(1),
            opponent: CharacterId::new(2),
            result: crate::experience::CombatResult::Draw,
            technique_used: None,
            technique_faced: None,
        };
        handler.handle_event(&combat, &ctx()); // +20
        handler.handle_event(&combat, &ctx()); // +20 → 40

        let rest = ExperienceEvent::Rest {
            header: make_header(1),
            method: "명상".to_string(),
            recovery: 0.3,
        };

        let result = handler.handle_event(&rest, &ctx());

        // 0.3 × 100 = 30 회복 → 40 - 30 = 10
        assert_eq!(result.side_effects.len(), 1);
        assert_eq!(handler.get(&CharacterId::new(1)).unwrap().fatigue(), 10);
    }

    // --- 시간 경과 ---

    #[test]
    fn time_passage_daily_recovery() {
        let mut handler = make_handler_with_char(1);

        // 피로 20으로 올림
        let combat = ExperienceEvent::Combat {
            header: make_header(1),
            opponent: CharacterId::new(2),
            result: crate::experience::CombatResult::Victory,
            technique_used: None,
            technique_faced: None,
        };
        handler.handle_event(&combat, &ctx()); // +20

        let time = ExperienceEvent::TimePassage {
            header: make_header(1),
            duration: 6,
            without_contact: true,
        };

        let result = handler.handle_event(&time, &ctx());

        // DAILY_REST_RECOVERY = 5 → 20 - 5 = 15
        assert_eq!(result.side_effects.len(), 1);
        assert_eq!(handler.get(&CharacterId::new(1)).unwrap().fatigue(), 15);
    }

    // --- 무관한 이벤트 ---

    #[test]
    fn unknown_event_no_op() {
        let mut handler = make_handler_with_char(1);
        let event = ExperienceEvent::Gift {
            header: make_header(1),
            giver: CharacterId::new(5),
            receiver: CharacterId::new(1),
            item: crate::shared::id::ItemId::new(42),
        };

        let result = handler.handle_event(&event, &ctx());
        assert!(result.side_effects.is_empty());
    }

    #[test]
    fn unknown_character_no_op() {
        let mut handler = make_handler_with_char(1);
        // subject가 CharacterId(99)인데, 핸들러에는 1만 있음
        let event = ExperienceEvent::Training {
            header: ExperienceHeader::new(
                ExperienceId::new(1),
                CharacterId::new(99),
                GameTime::new(1200, 1, 1),
                LocationId::new(1),
                5.0,
            ),
            skill: MartialArtId::new(1),
            method: String::new(),
            mentor: None,
            companion: None,
            duration: 3,
            intensity: 10,
        };

        let result = handler.handle_event(&event, &ctx());
        assert!(result.side_effects.is_empty());
    }

    // --- 피로 상한 ---

    #[test]
    fn fatigue_clamps_at_max() {
        let mut handler = make_handler_with_char(1);
        // intensity=10, duration=6 → 20 피로, 5번 반복 = 100
        let event = ExperienceEvent::Training {
            header: make_header(1),
            skill: MartialArtId::new(1),
            method: String::new(),
            mentor: None,
            companion: None,
            duration: 6,
            intensity: 10,
        };

        for _ in 0..6 {
            handler.handle_event(&event, &ctx());
        }

        // 100 이상으로는 올라가지 않음
        assert_eq!(handler.get(&CharacterId::new(1)).unwrap().fatigue(), 100);
    }

    // --- 소유권 ---

    #[test]
    fn into_characters_returns_map() {
        let handler = make_handler_with_char(1);
        let chars = handler.into_characters();
        assert_eq!(chars.len(), 1);
        assert!(chars.contains_key(&CharacterId::new(1)));
    }
}
