// wuxia-core/src/character/event.rs
//
// Character Domain Events — 캐릭터 도메인에서 발생하는 이벤트들.
//
// 캐릭터 도메인은 "이 존재는 누구인가?"에 대한 변화를 알린다.
// 현재 이벤트:
//   Aged             → 나이를 한 살 먹었다
//   LifeStageChanged → 생애 단계가 전환되었다 (청년→장년 등)
//
// 향후 추가 가능:
//   Died             → 사망 (Phase 3+)
//   NameChanged      → 개명 (서사 이벤트)

use serde::{Deserialize, Serialize};

use crate::character::fatigue::FatigueLevel;
use crate::character::injury::{InjuryType, InjurySeverity};
use crate::character::LifeStage;
use crate::shared::id::CharacterId;

/// 캐릭터 도메인에서 발생하는 이벤트들.
///
/// # Example
/// ```
/// use wuxia_core::character::CharacterEvent;
/// use wuxia_core::shared::CharacterId;
///
/// let event = CharacterEvent::Aged {
///     character_id: CharacterId::new(1),
///     new_age: 26,
/// };
/// assert_eq!(event.name(), "CharacterAged");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CharacterEvent {
    /// 캐릭터가 한 살 먹었다.
    /// Character::age_one_year()에서 항상 발생.
    Aged {
        character_id: CharacterId,
        new_age: u32,
    },

    /// 생애 단계가 전환되었다.
    /// Character::age_one_year()에서 단계 경계를 넘을 때 발생.
    LifeStageChanged {
        character_id: CharacterId,
        from: LifeStage,
        to: LifeStage,
    },

    /// 피로도가 변했다. [v2.3A]
    /// add_fatigue(), recover_fatigue(), daily_rest_recovery()에서 발생.
    ///
    /// 구독자:
    ///   - 성장 도메인: 수련 효율 조정
    ///   - 서사 도메인: 탈진 이벤트 트리거
    FatigueChanged {
        character_id: CharacterId,
        old_fatigue: u32,
        new_fatigue: u32,
        fatigue_level: FatigueLevel,
    },

    /// 부상을 입었다. [v2.3A]
    /// Character::injure()에서 발생.
    ///
    /// 구독자:
    ///   - 성장 도메인: 수련 제한 적용
    ///   - 서사 도메인: 부상 이벤트 트리거
    ///   - 관계 도메인: 간호 기회 생성
    Injured {
        character_id: CharacterId,
        injury_type: InjuryType,
        severity: InjurySeverity,
    },

    /// 부상이 완치되었다. [v2.3A]
    /// Character::heal_daily() 또는 treat_injury()에서 남은 일수가 0이 되면 발생.
    InjuryHealed {
        character_id: CharacterId,
        injury_type: InjuryType,
    },
}

use crate::shared::event_macros::impl_event_name;

impl_event_name!(CharacterEvent {
    Aged => "CharacterAged",
    LifeStageChanged => "CharacterLifeStageChanged",
    FatigueChanged => "CharacterFatigueChanged",
    Injured => "CharacterInjured",
    InjuryHealed => "CharacterInjuryHealed",
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aged_event() {
        let event = CharacterEvent::Aged {
            character_id: CharacterId::new(1),
            new_age: 26,
        };
        assert_eq!(event.name(), "CharacterAged");
    }

    #[test]
    fn life_stage_changed_event() {
        let event = CharacterEvent::LifeStageChanged {
            character_id: CharacterId::new(1),
            from: LifeStage::Youth,
            to: LifeStage::Prime,
        };
        assert_eq!(event.name(), "CharacterLifeStageChanged");
    }

    #[test]
    fn clone_and_eq() {
        let a = CharacterEvent::Aged {
            character_id: CharacterId::new(1),
            new_age: 30,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn fatigue_changed_event() {
        let event = CharacterEvent::FatigueChanged {
            character_id: CharacterId::new(1),
            old_fatigue: 20,
            new_fatigue: 35,
            fatigue_level: FatigueLevel::Mild,
        };
        assert_eq!(event.name(), "CharacterFatigueChanged");
    }

    #[test]
    fn injured_event() {
        let event = CharacterEvent::Injured {
            character_id: CharacterId::new(1),
            injury_type: InjuryType::QiDeviation,
            severity: InjurySeverity::Critical,
        };
        assert_eq!(event.name(), "CharacterInjured");
    }

    #[test]
    fn injury_healed_event() {
        let event = CharacterEvent::InjuryHealed {
            character_id: CharacterId::new(1),
            injury_type: InjuryType::Bruise,
        };
        assert_eq!(event.name(), "CharacterInjuryHealed");
    }

    #[test]
    fn serialization_roundtrip() {
        let events: Vec<CharacterEvent> = vec![
            CharacterEvent::Aged {
                character_id: CharacterId::new(42),
                new_age: 55,
            },
            CharacterEvent::LifeStageChanged {
                character_id: CharacterId::new(7),
                from: LifeStage::Middle,
                to: LifeStage::Elder,
            },
            CharacterEvent::FatigueChanged {
                character_id: CharacterId::new(3),
                old_fatigue: 50,
                new_fatigue: 45,
                fatigue_level: FatigueLevel::Moderate,
            },
            CharacterEvent::Injured {
                character_id: CharacterId::new(5),
                injury_type: InjuryType::Fracture,
                severity: InjurySeverity::Major,
            },
            CharacterEvent::InjuryHealed {
                character_id: CharacterId::new(5),
                injury_type: InjuryType::Fracture,
            },
        ];
        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let restored: CharacterEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(event, restored);
        }
    }
}
