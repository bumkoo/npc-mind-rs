//! RelationshipMemoryHandler — 관계 변화 → MemoryEntry, cause variant별 분기 (Step D, Inline)
//!
//! 설계 문서: `docs/memory/03-implementation-design.md` §6.3, §8.3
//!
//! **책임**: `RelationshipUpdated` 이벤트를 구독해 양 당사자(owner/target) 관점에서
//! `MemoryEntry(memory_type=RelationshipChange, layer=A)`를 생성한다.
//! `RelationshipChangeCause` variant에 따라 content/source/topic을 분기한다 (§8.3, A8):
//!
//! | cause | source | topic | content |
//! |---|---|---|---|
//! | `SceneInteraction { scene_id }` | Experienced | `None` | "장면에서 {target}과(와)의 관계 변화" |
//! | `InformationTold { origin_chain }` | Heard or Rumor (체인 길이 기준) | `None` | "정보 전달로 {target} 관련 감정 변화" |
//! | `WorldEventOverlay { topic }` | Experienced | topic 계승 | "세계 사건({topic})으로 {target} 관련 변화" |
//! | `Rumor { rumor_id }` | Rumor | `None` | "소문({rumor_id}) 여파로 {target} 관련 변화" |
//! | `Unspecified` | Experienced | `None` | 일반 cause 미표기 변화 |
//!
//! **threshold 필터**: MEMORY_RELATIONSHIP_DELTA_THRESHOLD(0.05)보다 변화량 작으면 no-op.
//! 한 이벤트로 3축(closeness/trust/power) 모두의 Δ 중 최대값이 threshold 미만이면 의미
//! 없는 미세 변동으로 간주하고 기억을 남기지 않는다.
//!
//! **관점 분리**: owner → target 관점의 엔트리 하나만 생성한다. target 관점은 해당
//! RelationshipAgent가 따로 발행하는 (owner 반전) 이벤트가 존재한다면 그 이벤트에서
//! 같은 handler가 또 한 번 실행되어 생성된다. 현재 RelationshipAgent는 owner 관점만
//! 발행하므로 이 handler도 owner 쪽 엔트리만 만든다.
//!
//! **Inline 계약**: MemoryStore 에러는 로그만. 커맨드는 계속.

use std::sync::Arc;

use crate::application::command::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::event::{DomainEvent, EventKind, EventPayload, RelationshipChangeCause};
use crate::domain::memory::{
    MemoryEntry, MemoryLayer, MemoryScope, MemorySource, MemoryType, Provenance,
};
use crate::domain::tuning::MEMORY_RELATIONSHIP_DELTA_THRESHOLD;
use crate::ports::MemoryStore;

pub struct RelationshipMemoryHandler {
    store: Arc<dyn MemoryStore>,
}

impl RelationshipMemoryHandler {
    pub fn new(store: Arc<dyn MemoryStore>) -> Self {
        Self { store }
    }

    fn derive_entry_id(event_id: u64, owner: &str) -> String {
        format!("rel-{event_id:012}-{owner}")
    }

    fn max_delta(
        bc: f32,
        bt: f32,
        bp: f32,
        ac: f32,
        at: f32,
        ap: f32,
    ) -> f32 {
        [(ac - bc).abs(), (at - bt).abs(), (ap - bp).abs()]
            .into_iter()
            .fold(0.0_f32, f32::max)
    }

    /// cause variant에 따른 (source, topic, content) 결정.
    fn derive_from_cause(
        cause: &RelationshipChangeCause,
        target: &str,
    ) -> (MemorySource, Option<String>, String) {
        match cause {
            RelationshipChangeCause::SceneInteraction { scene_id: _ } => (
                MemorySource::Experienced,
                None,
                format!("장면에서 {target}과(와)의 관계 변화"),
            ),
            RelationshipChangeCause::InformationTold { origin_chain } => {
                // 체인 길이 0/1 → Heard, 2+ → Rumor (§2.2 MemorySource::from_origin_chain)
                let source = MemorySource::from_origin_chain(origin_chain.len(), None);
                (
                    source,
                    None,
                    format!("정보 전달로 {target} 관련 감정 변화"),
                )
            }
            RelationshipChangeCause::WorldEventOverlay { topic } => (
                MemorySource::Experienced,
                topic.clone(),
                match topic {
                    Some(t) => format!("세계 사건({t})으로 {target} 관련 변화"),
                    None => format!("세계 사건 여파로 {target} 관련 변화"),
                },
            ),
            RelationshipChangeCause::Rumor { rumor_id } => (
                MemorySource::Rumor,
                None,
                format!("소문({rumor_id}) 여파로 {target} 관련 변화"),
            ),
            RelationshipChangeCause::Unspecified => (
                MemorySource::Experienced,
                None,
                format!("{target}과(와)의 관계 변화"),
            ),
        }
    }
}

impl EventHandler for RelationshipMemoryHandler {
    fn name(&self) -> &'static str {
        "RelationshipMemoryHandler"
    }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![EventKind::RelationshipUpdated])
    }

    fn mode(&self) -> DeliveryMode {
        DeliveryMode::Inline {
            priority: priority::inline::RELATIONSHIP_MEMORY,
        }
    }

    fn handle(
        &self,
        event: &DomainEvent,
        _ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        let EventPayload::RelationshipUpdated {
            owner_id,
            target_id,
            before_closeness,
            before_trust,
            before_power,
            after_closeness,
            after_trust,
            after_power,
            cause,
        } = &event.payload
        else {
            return Ok(HandlerResult::default());
        };

        // 미세 변동은 기록하지 않음
        let delta = Self::max_delta(
            *before_closeness,
            *before_trust,
            *before_power,
            *after_closeness,
            *after_trust,
            *after_power,
        );
        if delta < MEMORY_RELATIONSHIP_DELTA_THRESHOLD {
            return Ok(HandlerResult::default());
        }

        let (source, topic, content) = Self::derive_from_cause(cause, target_id);

        let id = Self::derive_entry_id(event.id, owner_id);
        #[allow(deprecated)] // Personal 투영 grand-father (§2.5 H10)
        let entry = MemoryEntry {
            id: id.clone(),
            created_seq: event.id,
            event_id: event.id,
            scope: MemoryScope::Personal {
                npc_id: owner_id.clone(),
            },
            source,
            provenance: Provenance::Runtime,
            memory_type: MemoryType::RelationshipChange,
            layer: MemoryLayer::A,
            content,
            topic,
            emotional_context: None,
            timestamp_ms: event.timestamp_ms,
            last_recalled_at: None,
            recall_count: 0,
            origin_chain: match cause {
                RelationshipChangeCause::InformationTold { origin_chain } => origin_chain.clone(),
                RelationshipChangeCause::Rumor { rumor_id } => vec![format!("rumor:{rumor_id}")],
                _ => vec![],
            },
            confidence: 1.0,
            acquired_by: None,
            superseded_by: None,
            consolidated_into: None,
            npc_id: owner_id.clone(),
        };

        if let Err(e) = self.store.index(entry, None) {
            tracing::warn!(
                event_id = event.id,
                owner_id,
                target_id,
                error = %e,
                "RelationshipMemoryHandler: MemoryStore.index failed"
            );
        }

        Ok(HandlerResult::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::command::handler_v2::test_support::HandlerTestHarness;
    use crate::domain::scene_id::SceneId;
    use std::sync::Mutex;

    #[derive(Default)]
    struct SpyStore {
        entries: Mutex<Vec<MemoryEntry>>,
    }

    impl MemoryStore for SpyStore {
        fn index(
            &self,
            entry: MemoryEntry,
            _embedding: Option<Vec<f32>>,
        ) -> Result<(), crate::ports::MemoryError> {
            self.entries.lock().unwrap().push(entry);
            Ok(())
        }
        fn search_by_meaning(
            &self,
            _q: &[f32],
            _npc: Option<&str>,
            _limit: usize,
        ) -> Result<Vec<crate::domain::memory::MemoryResult>, crate::ports::MemoryError> {
            Ok(vec![])
        }
        fn search_by_keyword(
            &self,
            _kw: &str,
            _npc: Option<&str>,
            _limit: usize,
        ) -> Result<Vec<crate::domain::memory::MemoryResult>, crate::ports::MemoryError> {
            Ok(vec![])
        }
        fn get_recent(
            &self,
            _npc: &str,
            _limit: usize,
        ) -> Result<Vec<MemoryEntry>, crate::ports::MemoryError> {
            Ok(vec![])
        }
        fn count(&self) -> usize {
            self.entries.lock().unwrap().len()
        }
        fn search(
            &self,
            _q: crate::ports::MemoryQuery,
        ) -> Result<Vec<crate::domain::memory::MemoryResult>, crate::ports::MemoryError> {
            Ok(vec![])
        }
        fn get_by_id(
            &self,
            id: &str,
        ) -> Result<Option<MemoryEntry>, crate::ports::MemoryError> {
            Ok(self
                .entries
                .lock()
                .unwrap()
                .iter()
                .find(|e| e.id == id)
                .cloned())
        }
        fn get_by_topic_latest(
            &self,
            _t: &str,
        ) -> Result<Option<MemoryEntry>, crate::ports::MemoryError> {
            Ok(None)
        }
        fn get_canonical_by_topic(
            &self,
            _t: &str,
        ) -> Result<Option<MemoryEntry>, crate::ports::MemoryError> {
            Ok(None)
        }
        fn mark_superseded(
            &self,
            _o: &str,
            _n: &str,
        ) -> Result<(), crate::ports::MemoryError> {
            Ok(())
        }
        fn mark_consolidated(
            &self,
            _a: &[String],
            _b: &str,
        ) -> Result<(), crate::ports::MemoryError> {
            Ok(())
        }
        fn record_recall(&self, _id: &str, _now_ms: u64) -> Result<(), crate::ports::MemoryError> {
            Ok(())
        }
    }

    fn rel_updated_event(
        event_id: u64,
        owner: &str,
        target: &str,
        delta_close: f32,
        cause: RelationshipChangeCause,
    ) -> DomainEvent {
        DomainEvent::new(
            event_id,
            owner.into(),
            1,
            EventPayload::RelationshipUpdated {
                owner_id: owner.into(),
                target_id: target.into(),
                before_closeness: 0.0,
                before_trust: 0.0,
                before_power: 0.0,
                after_closeness: delta_close,
                after_trust: 0.0,
                after_power: 0.0,
                cause,
            },
        )
    }

    #[test]
    fn scene_interaction_cause_creates_experienced_entry() {
        let store = Arc::new(SpyStore::default());
        let handler = RelationshipMemoryHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();

        harness
            .dispatch(
                &handler,
                rel_updated_event(
                    10,
                    "alice",
                    "bob",
                    0.3,
                    RelationshipChangeCause::SceneInteraction {
                        scene_id: SceneId::new("alice", "bob"),
                    },
                ),
            )
            .unwrap();

        let entries = store.entries.lock().unwrap().clone();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, MemorySource::Experienced);
        assert_eq!(entries[0].memory_type, MemoryType::RelationshipChange);
        assert_eq!(entries[0].topic, None);
        assert!(entries[0].content.contains("bob"));
    }

    #[test]
    fn information_told_cause_branches_on_chain_length() {
        let store = Arc::new(SpyStore::default());
        let handler = RelationshipMemoryHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();

        harness
            .dispatch(
                &handler,
                rel_updated_event(
                    11,
                    "alice",
                    "bob",
                    0.3,
                    RelationshipChangeCause::InformationTold {
                        origin_chain: vec!["sage".into()],
                    },
                ),
            )
            .unwrap();
        harness
            .dispatch(
                &handler,
                rel_updated_event(
                    12,
                    "alice",
                    "bob",
                    0.3,
                    RelationshipChangeCause::InformationTold {
                        origin_chain: vec!["relay".into(), "witness".into()],
                    },
                ),
            )
            .unwrap();

        let entries = store.entries.lock().unwrap().clone();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].source, MemorySource::Heard, "체인 길이 1 → Heard");
        assert_eq!(entries[1].source, MemorySource::Rumor, "체인 길이 2 → Rumor");
        assert_eq!(entries[0].origin_chain, vec!["sage".to_string()]);
    }

    #[test]
    fn world_event_overlay_cause_sets_topic() {
        let store = Arc::new(SpyStore::default());
        let handler = RelationshipMemoryHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();

        harness
            .dispatch(
                &handler,
                rel_updated_event(
                    13,
                    "alice",
                    "bob",
                    0.3,
                    RelationshipChangeCause::WorldEventOverlay {
                        topic: Some("leader-change".into()),
                    },
                ),
            )
            .unwrap();

        let entries = store.entries.lock().unwrap().clone();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, MemorySource::Experienced);
        assert_eq!(entries[0].topic.as_deref(), Some("leader-change"));
        assert!(entries[0].content.contains("leader-change"));
    }

    #[test]
    fn rumor_cause_sets_rumor_source_and_chain_marker() {
        let store = Arc::new(SpyStore::default());
        let handler = RelationshipMemoryHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();

        harness
            .dispatch(
                &handler,
                rel_updated_event(
                    14,
                    "alice",
                    "bob",
                    0.3,
                    RelationshipChangeCause::Rumor {
                        rumor_id: "r-007".into(),
                    },
                ),
            )
            .unwrap();

        let entries = store.entries.lock().unwrap().clone();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, MemorySource::Rumor);
        assert_eq!(entries[0].origin_chain, vec!["rumor:r-007".to_string()]);
        assert!(entries[0].content.contains("r-007"));
    }

    #[test]
    fn unspecified_cause_uses_generic_content() {
        let store = Arc::new(SpyStore::default());
        let handler = RelationshipMemoryHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();

        harness
            .dispatch(
                &handler,
                rel_updated_event(
                    15,
                    "alice",
                    "bob",
                    0.3,
                    RelationshipChangeCause::Unspecified,
                ),
            )
            .unwrap();

        let entries = store.entries.lock().unwrap().clone();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, MemorySource::Experienced);
        assert_eq!(entries[0].topic, None);
    }

    #[test]
    fn small_deltas_below_threshold_are_skipped() {
        let store = Arc::new(SpyStore::default());
        let handler = RelationshipMemoryHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();

        // 0.01 변화 → MEMORY_RELATIONSHIP_DELTA_THRESHOLD=0.05 미만 → skip
        harness
            .dispatch(
                &handler,
                rel_updated_event(
                    16,
                    "alice",
                    "bob",
                    0.01,
                    RelationshipChangeCause::Unspecified,
                ),
            )
            .unwrap();
        assert_eq!(store.count(), 0, "미세 변동은 기록 스킵");
    }
}
