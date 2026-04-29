// wuxia-core/src/shared/event.rs
//
// Domain Events — 도메인 간 통신을 위한 래퍼 이벤트.
//
// [리팩터링 v2] 도메인별 enum을 DomainEvent wrapper로 감싼다.
//
// 이전 구조 (v1): 모든 이벤트가 하나의 enum에
//   DomainEvent::DayPassed { .. }
//   DomainEvent::CharacterAged { .. }
//   → 도메인이 늘수록 enum이 비대해지는 문제
//
// 새 구조 (v2): 도메인별 enum + wrapper
//   TimeEvent::DayPassed { .. }           ← 시간 도메인 소유
//   CharacterEvent::Aged { .. }           ← 캐릭터 도메인 소유
//   DomainEvent::Time(TimeEvent::...)     ← 래퍼가 감싸서 전달
//   DomainEvent::Character(CharacterEvent::...) ← 래퍼가 감싸서 전달
//
// 장점:
//   - 각 도메인이 자기 이벤트를 독립적으로 관리
//   - 새 도메인 추가 시 wrapper에 variant 하나만 추가
//   - Application Service에서 도메인별 필터링이 자연스러움
//
// 비유: 강호 소식통 (江湖消息)
//   시간 소식: "새해가 밝았다!" → TimeEvent::YearPassed
//   인물 소식: "령호충이 서른이 되었다!" → CharacterEvent::Aged
//   소식통이 모아서 전달: DomainEvent::Time(...), DomainEvent::Character(...)

use serde::{Deserialize, Serialize};

use crate::character::CharacterEvent;
use crate::growth::GrowthEvent;
use crate::memory::MemoryEvent;
use crate::psychology::PsychologyEvent;
use crate::relationship::RelationshipEvent;
use crate::time::TimeEvent;

/// 모든 도메인 이벤트를 감싸는 래퍼 enum.
///
/// 각 도메인이 자기 이벤트 enum을 소유하고,
/// DomainEvent는 이들을 하나로 묶어 Application Service에 전달한다.
///
/// # 새 도메인 추가 시
/// 1. 해당 도메인에 `XxxEvent` enum 생성
/// 2. 여기에 `Xxx(XxxEvent)` variant 추가
/// 3. `From<XxxEvent> for DomainEvent` 구현
///
/// # Example
/// ```
/// use wuxia_core::shared::DomainEvent;
/// use wuxia_core::time::TimeEvent;
/// use wuxia_core::character::CharacterEvent;
/// use wuxia_core::shared::CharacterId;
///
/// // TimeEvent → DomainEvent 자동 변환
/// let time_event = TimeEvent::YearPassed { new_year: 1201 };
/// let domain_event: DomainEvent = time_event.into();
///
/// // 매칭 시 두 단계
/// match &domain_event {
///     DomainEvent::Time(TimeEvent::YearPassed { new_year }) => {
///         assert_eq!(*new_year, 1201);
///     }
///     _ => panic!("Expected YearPassed"),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DomainEvent {
    /// 시간 도메인 이벤트 (DayPassed, SeasonChanged, YearPassed)
    Time(TimeEvent),

    /// 캐릭터 도메인 이벤트 (Aged, LifeStageChanged)
    Character(CharacterEvent),

    /// 성장 도메인 이벤트 (Trained, YearlyAgingApplied)
    Growth(GrowthEvent),

    /// 기억 도메인 이벤트 (MemoryStored, MemoryRecalled, ImportanceUpdated)
    Memory(MemoryEvent),

    /// 관계 도메인 이벤트 (AffinityChanged, TrustChanged, LevelChanged, ...)
    Relationship(RelationshipEvent),

    /// 심리 도메인 이벤트 (PersonalityChanged, EmotionGenerated, MoodChanged, ...)
    Psychology(PsychologyEvent),
}

impl DomainEvent {
    /// 로깅/디버깅용 이벤트 이름.
    ///
    /// 내부 이벤트의 name()을 위임한다.
    pub fn name(&self) -> &'static str {
        match self {
            DomainEvent::Time(e) => e.name(),
            DomainEvent::Character(e) => e.name(),
            DomainEvent::Growth(e) => e.name(),
            DomainEvent::Memory(e) => e.name(),
            DomainEvent::Relationship(e) => e.name(),
            DomainEvent::Psychology(e) => e.name(),
        }
    }
}

// ---------------------------------------------------------------------------
// From 구현 — 도메인 이벤트를 DomainEvent로 자연스럽게 변환
// ---------------------------------------------------------------------------

impl From<TimeEvent> for DomainEvent {
    fn from(event: TimeEvent) -> Self {
        DomainEvent::Time(event)
    }
}

impl From<CharacterEvent> for DomainEvent {
    fn from(event: CharacterEvent) -> Self {
        DomainEvent::Character(event)
    }
}

impl From<GrowthEvent> for DomainEvent {
    fn from(event: GrowthEvent) -> Self {
        DomainEvent::Growth(event)
    }
}

impl From<MemoryEvent> for DomainEvent {
    fn from(event: MemoryEvent) -> Self {
        DomainEvent::Memory(event)
    }
}

impl From<RelationshipEvent> for DomainEvent {
    fn from(event: RelationshipEvent) -> Self {
        DomainEvent::Relationship(event)
    }
}

impl From<PsychologyEvent> for DomainEvent {
    fn from(event: PsychologyEvent) -> Self {
        DomainEvent::Psychology(event)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::LifeStage;
    use crate::growth::martial_art::MasteryLevel;
    use crate::growth::{ChangeSource, GrowthEvent, StatChange, StatType};
    use crate::memory::{MemoryEvent, MemoryType};
    use crate::psychology::{
        AxisType, EmotionType, HexacoFactor, PracticalValueType, PsychologyEvent, ReflectionTier,
    };
    use crate::relationship::RelationshipEvent;
    use crate::shared::id::{CharacterId, MartialArtId, MemoryId, RelationshipId};
    use crate::shared::time::{GameTime, Season};

    // -- From 변환 --

    #[test]
    fn from_time_event() {
        let time_event = TimeEvent::YearPassed { new_year: 1201 };
        let domain_event: DomainEvent = time_event.into();
        assert_eq!(domain_event.name(), "YearPassed");
    }

    #[test]
    fn from_character_event() {
        let char_event = CharacterEvent::Aged {
            character_id: CharacterId::new(1),
            new_age: 26,
        };
        let domain_event: DomainEvent = char_event.into();
        assert_eq!(domain_event.name(), "CharacterAged");
    }

    #[test]
    fn from_growth_event() {
        let growth_event = GrowthEvent::StatTrained {
            character_id: CharacterId::new(1),
            changes: vec![StatChange::new(StatType::Strength, 5, ChangeSource::StatTraining)],
        };
        let domain_event: DomainEvent = growth_event.into();
        assert_eq!(domain_event.name(), "GrowthStatTrained");
    }

    #[test]
    fn from_memory_event() {
        let memory_event = MemoryEvent::MemoryStored {
            memory_id: MemoryId::new(1),
            character_id: CharacterId::new(5),
            memory_type: MemoryType::Observation,
            importance: 7.0,
        };
        let domain_event: DomainEvent = memory_event.into();
        assert_eq!(domain_event.name(), "MemoryStored");
    }

    #[test]
    fn from_relationship_event() {
        let rel_event = RelationshipEvent::AffinityChanged {
            relationship_id: RelationshipId::new(1),
            source: CharacterId::new(1),
            target: CharacterId::new(2),
            old_value: 0.0,
            new_value: 30.0,
        };
        let domain_event: DomainEvent = rel_event.into();
        assert_eq!(domain_event.name(), "RelAffinityChanged");
    }

    #[test]
    fn from_psychology_event() {
        let psy_event = PsychologyEvent::EmotionGenerated {
            character_id: CharacterId::new(1),
            emotion_type: EmotionType::Anger,
            intensity: 72.5,
        };
        let domain_event: DomainEvent = psy_event.into();
        assert_eq!(domain_event.name(), "PsyEmotionGenerated");
    }

    // -- 매칭 --

    #[test]
    fn match_time_event() {
        let event = DomainEvent::Time(TimeEvent::DayPassed {
            date: GameTime::new(1200, 3, 15),
        });
        match &event {
            DomainEvent::Time(TimeEvent::DayPassed { date }) => {
                assert_eq!(date.year(), 1200);
            }
            _ => panic!("Expected DayPassed"),
        }
    }

    #[test]
    fn match_character_event() {
        let event = DomainEvent::Character(CharacterEvent::LifeStageChanged {
            character_id: CharacterId::new(1),
            from: LifeStage::Youth,
            to: LifeStage::Prime,
        });
        match &event {
            DomainEvent::Character(CharacterEvent::LifeStageChanged { from, to, .. }) => {
                assert_eq!(*from, LifeStage::Youth);
                assert_eq!(*to, LifeStage::Prime);
            }
            _ => panic!("Expected LifeStageChanged"),
        }
    }

    #[test]
    fn match_growth_event() {
        let event = DomainEvent::Growth(GrowthEvent::YearlyAgingApplied {
            character_id: CharacterId::new(1),
            life_stage: LifeStage::Elder,
            changes: vec![
                StatChange::new(StatType::Vitality, -2, ChangeSource::YearlyAging),
            ],
        });
        match &event {
            DomainEvent::Growth(GrowthEvent::YearlyAgingApplied { life_stage, changes, .. }) => {
                assert_eq!(*life_stage, LifeStage::Elder);
                assert_eq!(changes[0].delta(), -2);
            }
            _ => panic!("Expected YearlyAgingApplied"),
        }
    }

    // -- name() 위임 --

    #[test]
    fn name_delegates_to_inner() {
        let events = vec![
            DomainEvent::Time(TimeEvent::WatchChanged {
                new_watch: crate::shared::time::Watch::Morning,
                date: GameTime::with_watch(1200, 1, 1, crate::shared::time::Watch::Morning),
            }),
            DomainEvent::Time(TimeEvent::DayPassed {
                date: GameTime::new(1200, 1, 1),
            }),
            DomainEvent::Time(TimeEvent::SeasonChanged {
                new_season: Season::Spring,
            }),
            DomainEvent::Time(TimeEvent::YearPassed { new_year: 1201 }),
            DomainEvent::Character(CharacterEvent::Aged {
                character_id: CharacterId::new(1),
                new_age: 26,
            }),
            DomainEvent::Character(CharacterEvent::LifeStageChanged {
                character_id: CharacterId::new(1),
                from: LifeStage::Youth,
                to: LifeStage::Prime,
            }),
            DomainEvent::Growth(GrowthEvent::StatTrained {
                character_id: CharacterId::new(1),
                changes: vec![],
            }),
            DomainEvent::Growth(GrowthEvent::YearlyAgingApplied {
                character_id: CharacterId::new(1),
                life_stage: LifeStage::Elder,
                changes: vec![],
            }),
            DomainEvent::Growth(GrowthEvent::ArtPracticed {
                character_id: CharacterId::new(1),
                martial_art_id: MartialArtId::new(1),
                proficiency_gain: 3,
                new_proficiency: 33,
                old_mastery: MasteryLevel::Beginner,
                new_mastery: MasteryLevel::Proficient,
                stat_changes: vec![],
            }),
            DomainEvent::Memory(MemoryEvent::MemoryStored {
                memory_id: MemoryId::new(1),
                character_id: CharacterId::new(5),
                memory_type: MemoryType::Observation,
                importance: 7.0,
            }),
            DomainEvent::Memory(MemoryEvent::MemoryRecalled {
                character_id: CharacterId::new(5),
                recalled_ids: vec![MemoryId::new(42)],
            }),
            DomainEvent::Memory(MemoryEvent::ImportanceUpdated {
                memory_id: MemoryId::new(1),
                character_id: CharacterId::new(5),
                old_importance: 3.0,
                new_importance: 8.0,
            }),
            DomainEvent::Relationship(RelationshipEvent::AffinityChanged {
                relationship_id: RelationshipId::new(1),
                source: CharacterId::new(1),
                target: CharacterId::new(2),
                old_value: 0.0,
                new_value: 30.0,
            }),
            DomainEvent::Relationship(RelationshipEvent::BondBroken {
                relationship_id: RelationshipId::new(1),
                source: CharacterId::new(2),
                target: CharacterId::new(1),
                reason: "배신".to_string(),
            }),
            DomainEvent::Psychology(PsychologyEvent::EmotionGenerated {
                character_id: CharacterId::new(1),
                emotion_type: EmotionType::Anger,
                intensity: 72.5,
            }),
            DomainEvent::Psychology(PsychologyEvent::PersonalityChanged {
                character_id: CharacterId::new(5),
                factor: HexacoFactor::Extraversion,
                old_value: 30,
                new_value: 35,
            }),
            DomainEvent::Psychology(PsychologyEvent::AxisIntensityChanged {
                character_id: CharacterId::new(1),
                axis: AxisType::Rightness,
                old_value: 90.0,
                new_value: 85.0,
                tier: ReflectionTier::TurningPoint,
            }),
            DomainEvent::Psychology(PsychologyEvent::PracticalValueChanged {
                character_id: CharacterId::new(1),
                value_type: PracticalValueType::Vengeance,
                old_value: 60.0,
                new_value: 75.0,
                tier: ReflectionTier::TurningPoint,
            }),
        ];

        let expected_names = vec![
            "WatchChanged",
            "DayPassed",
            "SeasonChanged",
            "YearPassed",
            "CharacterAged",
            "CharacterLifeStageChanged",
            "GrowthStatTrained",
            "GrowthYearlyAgingApplied",
            "GrowthArtPracticed",
            "MemoryStored",
            "MemoryRecalled",
            "MemoryImportanceUpdated",
            "RelAffinityChanged",
            "RelBondBroken",
            "PsyEmotionGenerated",
            "PsyPersonalityChanged",
            "PsyAxisIntensityChanged",
            "PsyValueChanged",
        ];

        for (event, expected) in events.iter().zip(expected_names.iter()) {
            assert_eq!(event.name(), *expected);
        }
    }

    // -- Serialization --

    #[test]
    fn serialization_roundtrip() {
        let event = DomainEvent::Time(TimeEvent::YearPassed { new_year: 1201 });
        let json = serde_json::to_string(&event).unwrap();
        let restored: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, restored);
    }

    #[test]
    fn character_event_serialization() {
        let event = DomainEvent::Character(CharacterEvent::Aged {
            character_id: CharacterId::new(42),
            new_age: 55,
        });
        let json = serde_json::to_string(&event).unwrap();
        let restored: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, restored);
    }

    #[test]
    fn psychology_event_serialization() {
        let event = DomainEvent::Psychology(PsychologyEvent::EmotionGenerated {
            character_id: CharacterId::new(1),
            emotion_type: EmotionType::Fear,
            intensity: 55.0,
        });
        let json = serde_json::to_string(&event).unwrap();
        let restored: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, restored);
    }

    // -- Clone & Eq --

    #[test]
    fn clone_and_eq() {
        let a = DomainEvent::Character(CharacterEvent::Aged {
            character_id: CharacterId::new(1),
            new_age: 30,
        });
        let b = a.clone();
        assert_eq!(a, b);
    }
}
