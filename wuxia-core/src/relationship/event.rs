// wuxia-core/src/relationship/event.rs
//
// Relationship Domain Events — 관계 도메인에서 발생하는 이벤트들.
//
// TimeEvent, CharacterEvent, GrowthEvent, MemoryEvent와 동일한 패턴.
// DomainEvent::Relationship(RelationshipEvent)로 감싸져
// Application Service에 전달된다.
//
// 비유: 강호 인맥 소식 (江湖人脈消息)
//   "소연이 플레이어를 좋게 봤다!" → AffinityChanged
//   "소연이 과거를 털어놓았다!"     → TrustChanged
//   "소연이 등을 돌렸다!"           → BondBroken

use serde::{Deserialize, Serialize};

use crate::shared::id::{CharacterId, RelationshipId};

use super::level::RelationshipLevel;
use super::relationship_type::RelationshipType;

/// 관계 도메인에서 발생하는 이벤트들.
///
/// # Example
/// ```
/// use wuxia_core::relationship::RelationshipEvent;
/// use wuxia_core::shared::id::{CharacterId, RelationshipId};
///
/// let event = RelationshipEvent::AffinityChanged {
///     relationship_id: RelationshipId::new(1),
///     source: CharacterId::new(1),
///     target: CharacterId::new(2),
///     old_value: 30.0,
///     new_value: 45.0,
/// };
/// assert_eq!(event.name(), "RelAffinityChanged");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RelationshipEvent {
    /// 호감도가 변했다.
    ///
    /// "소연이 정보를 무료로 줬다" → affinity +10
    ///
    /// 구독자:
    ///   - (향후) 서사 도메인: 퀘스트 트리거 (호감도 30/50/60/70/80+)
    ///   - (향후) 심리 도메인: NPC 기분 변화
    AffinityChanged {
        relationship_id: RelationshipId,
        source: CharacterId,
        target: CharacterId,
        old_value: f32,
        new_value: f32,
    },

    /// 신뢰도가 변했다.
    ///
    /// "소연이 개방 소속임을 밝혔다" → trust +15
    ///
    /// 구독자:
    ///   - (향후) 서사 도메인: 비밀 공유 이벤트 해금
    TrustChanged {
        relationship_id: RelationshipId,
        source: CharacterId,
        target: CharacterId,
        old_value: f32,
        new_value: f32,
    },

    /// 관계 유형이 변했다.
    ///
    /// "소연과 동맹을 맺었다" → None → Some(Ally)
    TypeChanged {
        relationship_id: RelationshipId,
        source: CharacterId,
        target: CharacterId,
        old_type: Option<RelationshipType>,
        new_type: Option<RelationshipType>,
    },

    /// 관계 깊이(레벨)가 변했다.
    ///
    /// "소연과의 관계가 '친근'에서 '가까운 사이'로 깊어졌다"
    ///
    /// 구독자:
    ///   - (향후) 서사 도메인: 퀘스트 해금 판정
    ///   - (향후) UI: 관계 변화 알림
    LevelChanged {
        relationship_id: RelationshipId,
        source: CharacterId,
        target: CharacterId,
        old_level: RelationshipLevel,
        new_level: RelationshipLevel,
    },

    /// 관계가 파기되었다.
    ///
    /// "소연이 등을 돌렸다. 개방의 적으로 대하겠어."
    /// 서사적으로 '결별'을 의미한다.
    ///
    /// 구독자:
    ///   - (향후) 서사 도메인: 퀘스트라인 차단
    ///   - (향후) 심리 도메인: Tier 3 전환점 성찰
    BondBroken {
        relationship_id: RelationshipId,
        source: CharacterId,
        target: CharacterId,
        reason: String,
    },

    /// 상호작용이 기록되었다.
    ///
    /// 구독자:
    ///   - (향후) 기억 도메인: 상호작용을 기억으로 저장
    InteractionRecorded {
        relationship_id: RelationshipId,
        source: CharacterId,
        target: CharacterId,
        interaction_count: u32,
    },
}

use crate::shared::event_macros::impl_event_name;

impl_event_name!(RelationshipEvent {
    AffinityChanged => "RelAffinityChanged",
    TrustChanged => "RelTrustChanged",
    TypeChanged => "RelTypeChanged",
    LevelChanged => "RelLevelChanged",
    BondBroken => "RelBondBroken",
    InteractionRecorded => "RelInteractionRecorded",
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relationship::RelationshipLevel;

    fn ids() -> (RelationshipId, CharacterId, CharacterId) {
        (RelationshipId::new(1), CharacterId::new(1), CharacterId::new(2))
    }

    #[test]
    fn affinity_changed_event() {
        let (rid, src, tgt) = ids();
        let event = RelationshipEvent::AffinityChanged {
            relationship_id: rid,
            source: src,
            target: tgt,
            old_value: 30.0,
            new_value: 45.0,
        };
        assert_eq!(event.name(), "RelAffinityChanged");
    }

    #[test]
    fn trust_changed_event() {
        let (rid, src, tgt) = ids();
        let event = RelationshipEvent::TrustChanged {
            relationship_id: rid,
            source: src,
            target: tgt,
            old_value: 10.0,
            new_value: 35.0,
        };
        assert_eq!(event.name(), "RelTrustChanged");
    }

    #[test]
    fn type_changed_event() {
        let (rid, src, tgt) = ids();
        let event = RelationshipEvent::TypeChanged {
            relationship_id: rid,
            source: src,
            target: tgt,
            old_type: None,
            new_type: Some(RelationshipType::Friend),
        };
        assert_eq!(event.name(), "RelTypeChanged");
    }

    #[test]
    fn level_changed_event() {
        let (rid, src, tgt) = ids();
        let event = RelationshipEvent::LevelChanged {
            relationship_id: rid,
            source: src,
            target: tgt,
            old_level: RelationshipLevel::Stranger,
            new_level: RelationshipLevel::Acquaintance,
        };
        assert_eq!(event.name(), "RelLevelChanged");
    }

    #[test]
    fn bond_broken_event() {
        let (rid, src, tgt) = ids();
        let event = RelationshipEvent::BondBroken {
            relationship_id: rid,
            source: src,
            target: tgt,
            reason: "플레이어가 조고 편에 섰다".to_string(),
        };
        assert_eq!(event.name(), "RelBondBroken");
    }

    #[test]
    fn interaction_recorded_event() {
        let (rid, src, tgt) = ids();
        let event = RelationshipEvent::InteractionRecorded {
            relationship_id: rid,
            source: src,
            target: tgt,
            interaction_count: 5,
        };
        assert_eq!(event.name(), "RelInteractionRecorded");
    }

    #[test]
    fn clone_and_eq() {
        let (rid, src, tgt) = ids();
        let a = RelationshipEvent::AffinityChanged {
            relationship_id: rid,
            source: src,
            target: tgt,
            old_value: 0.0,
            new_value: 30.0,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn serialization_roundtrip() {
        let (rid, src, tgt) = ids();
        let events = vec![
            RelationshipEvent::AffinityChanged {
                relationship_id: rid, source: src, target: tgt,
                old_value: 0.0, new_value: 30.0,
            },
            RelationshipEvent::TrustChanged {
                relationship_id: rid, source: src, target: tgt,
                old_value: 10.0, new_value: 50.0,
            },
            RelationshipEvent::TypeChanged {
                relationship_id: rid, source: src, target: tgt,
                old_type: None, new_type: Some(RelationshipType::Enemy),
            },
            RelationshipEvent::LevelChanged {
                relationship_id: rid, source: src, target: tgt,
                old_level: RelationshipLevel::Friendly,
                new_level: RelationshipLevel::Enemy,
            },
            RelationshipEvent::BondBroken {
                relationship_id: rid, source: src, target: tgt,
                reason: "배신".to_string(),
            },
            RelationshipEvent::InteractionRecorded {
                relationship_id: rid, source: src, target: tgt,
                interaction_count: 3,
            },
        ];
        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let restored: RelationshipEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(event, restored);
        }
    }
}
