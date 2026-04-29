// wuxia-core/src/memory/event.rs
//
// Memory Domain Events — 기억 도메인에서 발생하는 이벤트들.
//
// TimeEvent, CharacterEvent, GrowthEvent와 동일한 패턴.
// DomainEvent::Memory(MemoryEvent)로 감싸져 Application Service에 전달된다.
//
// 비유: 강호의 기억 소식
//   "소연이 새로운 것을 기억했다!" → MemoryStored
//   "소연이 옛 기억을 떠올렸다!"   → MemoryRecalled
//   "소연의 기억이 재평가되었다!"   → ImportanceUpdated

use serde::{Deserialize, Serialize};

use crate::shared::id::{CharacterId, MemoryId};

use super::types::MemoryType;

/// 기억 도메인에서 발생하는 이벤트들.
///
/// # Example
/// ```
/// use wuxia_core::memory::{MemoryEvent, MemoryType};
/// use wuxia_core::shared::id::{CharacterId, MemoryId};
///
/// let event = MemoryEvent::MemoryStored {
///     memory_id: MemoryId::new(1),
///     character_id: CharacterId::new(5),
///     memory_type: MemoryType::Observation,
///     importance: 7.0,
/// };
/// assert_eq!(event.name(), "MemoryStored");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MemoryEvent {
    /// 새 기억이 저장되었다.
    ///
    /// 구독자:
    ///   - (향후) 심리 도메인: 중요도 임계치 초과 시 즉시 성찰 트리거
    ///   - (향후) 서사 도메인: 중요 기억 기반 이벤트 생성
    MemoryStored {
        memory_id: MemoryId,
        character_id: CharacterId,
        memory_type: MemoryType,
        importance: f32,
    },

    /// 기억이 검색/회상되었다.
    ///
    /// recalled_ids는 순위 순 정렬: [0] = 최고 점수 기억.
    /// recalled_ids.len() = 회상된 기억 수.
    ///
    /// 구독자:
    ///   - (향후) 심리 도메인: 자주 회상되는 기억 → 중요도 상향 조정 후보
    ///   - (향후) 로깅: 어떤 기억이 자주 회상되는지 추적
    MemoryRecalled {
        character_id: CharacterId,
        recalled_ids: Vec<MemoryId>,
    },

    /// 기억의 중요도가 재평가되었다.
    ///
    /// Tier 2 일상 성찰에서 발생.
    /// 예: 사소해 보였던 대화가 나중에 중요한 단서였음을 깨달음.
    ///
    /// 구독자:
    ///   - (향후) 심리 도메인: 3축 가치관 형성기억 업데이트
    ImportanceUpdated {
        memory_id: MemoryId,
        character_id: CharacterId,
        old_importance: f32,
        new_importance: f32,
    },
}

use crate::shared::event_macros::impl_event_name;

impl_event_name!(MemoryEvent {
    MemoryStored => "MemoryStored",
    MemoryRecalled => "MemoryRecalled",
    ImportanceUpdated => "MemoryImportanceUpdated",
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_stored_event() {
        let event = MemoryEvent::MemoryStored {
            memory_id: MemoryId::new(1),
            character_id: CharacterId::new(5),
            memory_type: MemoryType::Observation,
            importance: 7.0,
        };
        assert_eq!(event.name(), "MemoryStored");
    }

    #[test]
    fn memory_recalled_event() {
        let event = MemoryEvent::MemoryRecalled {
            character_id: CharacterId::new(5),
            recalled_ids: vec![MemoryId::new(42), MemoryId::new(7)],
        };
        assert_eq!(event.name(), "MemoryRecalled");
        // [0] = 최고 점수 기억
        if let MemoryEvent::MemoryRecalled { recalled_ids, .. } = &event {
            assert_eq!(recalled_ids.len(), 2);
            assert_eq!(recalled_ids[0], MemoryId::new(42));
        }
    }

    #[test]
    fn importance_updated_event() {
        let event = MemoryEvent::ImportanceUpdated {
            memory_id: MemoryId::new(1),
            character_id: CharacterId::new(5),
            old_importance: 3.0,
            new_importance: 8.0,
        };
        assert_eq!(event.name(), "MemoryImportanceUpdated");
    }

    #[test]
    fn clone_and_eq() {
        let a = MemoryEvent::MemoryStored {
            memory_id: MemoryId::new(1),
            character_id: CharacterId::new(5),
            memory_type: MemoryType::Reflection,
            importance: 9.0,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn serialization_roundtrip() {
        let events = vec![
            MemoryEvent::MemoryStored {
                memory_id: MemoryId::new(1),
                character_id: CharacterId::new(5),
                memory_type: MemoryType::Observation,
                importance: 7.0,
            },
            MemoryEvent::MemoryRecalled {
                character_id: CharacterId::new(5),
                recalled_ids: vec![MemoryId::new(42), MemoryId::new(7)],
            },
            MemoryEvent::ImportanceUpdated {
                memory_id: MemoryId::new(1),
                character_id: CharacterId::new(5),
                old_importance: 3.0,
                new_importance: 8.0,
            },
        ];
        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let restored: MemoryEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(event, restored);
        }
    }
}
