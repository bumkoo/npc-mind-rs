//! WorldOverlayHandler — 세계 오버레이 사건 → Canonical MemoryEntry + supersede (Step D)
//!
//! `WorldEventOccurred` 이벤트를 Inline phase에서 소비해:
//! 1. 같은 `topic`의 기존 유효 Canonical 엔트리를 `mark_superseded`로 대체 표시.
//! 2. 새 `MemoryEntry(scope=World, provenance=Seeded, type=WorldEvent)`를 생성해
//!    `MemoryStore`에 저장.
//!
//! Canonical 정의: `Provenance::Seeded && scope=World` → τ=∞ (영구 사실, §2.4).
//! Consolidation 대상이 아니며 (Type×Layer 표 — WorldEvent는 초기 Layer=A이지만
//! Consolidation 제외) Ranker 1단계 Source 필터에서 Experienced 상위에 위치.
//!
//! **supersede 정책** (§8.4, 리뷰 B1 수정):
//! - topic이 None이면 supersede 없이 새 엔트리만 추가 (독립 이벤트로 취급).
//! - topic이 Some이면 **같은 topic의 Canonical 1건만** `get_canonical_by_topic`으로 조회해
//!   supersede한다. 다른 NPC의 Personal `Heard`/`Rumor` 엔트리는 건드리지 않는다 —
//!   게임이 확정한 "사실"이 바뀌어도 개별 NPC의 주관적 해석(헤아린 기억)은 그 NPC의
//!   다음 상호작용에서 자연스럽게 갱신되어야 하며, 엔진이 일괄 무효화하지 않는다.
//!   (초기 구현은 topic 전수 supersede였으나 리뷰에서 플레이어 기억 파괴 부작용 지적)
//!
//! **Inline 계약**: MemoryStore 호출 실패는 `tracing::warn!`만 남기고 커맨드는 계속.

use std::sync::Arc;

use crate::application::command::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::event::{DomainEvent, EventKind, EventPayload};
use crate::domain::memory::{
    MemoryEntry, MemoryLayer, MemoryScope, MemorySource, MemoryType, Provenance,
};
use crate::ports::MemoryStore;

pub struct WorldOverlayHandler {
    store: Arc<dyn MemoryStore>,
}

impl WorldOverlayHandler {
    pub fn new(store: Arc<dyn MemoryStore>) -> Self {
        Self { store }
    }

    /// 결정적 엔트리 id — `(event.id, world_id)` 쌍이 유일하므로 충돌 없음 (리뷰 M3).
    /// 같은 커맨드가 replay되면 같은 id가 산출되고, `MemoryStore::index` 구현은
    /// 같은 id를 overwrite-in-place하는 것이 일반 규칙이라 **idempotent**하다.
    fn derive_entry_id(event_id: u64, world_id: &str) -> String {
        format!("world-{event_id:012}-{world_id}")
    }
}

impl EventHandler for WorldOverlayHandler {
    fn name(&self) -> &'static str {
        "WorldOverlayHandler"
    }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![EventKind::WorldEventOccurred])
    }

    fn mode(&self) -> DeliveryMode {
        DeliveryMode::Inline {
            priority: priority::inline::WORLD_OVERLAY_INGESTION,
        }
    }

    fn handle(
        &self,
        event: &DomainEvent,
        _ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        let EventPayload::WorldEventOccurred {
            world_id,
            topic,
            fact,
            significance: _, // 현 단계에서는 기록 없이 이벤트 필드로만 유지
            witnesses: _,
        } = &event.payload
        else {
            return Ok(HandlerResult::default());
        };

        let new_id = Self::derive_entry_id(event.id, world_id);

        // 1) topic 있으면 같은 topic의 **Canonical 1건만** supersede.
        //    다른 NPC의 Personal Heard/Rumor 등 개별 주관 기억은 보존한다 (리뷰 B1).
        if let Some(topic_str) = topic.as_ref() {
            match self.store.get_canonical_by_topic(topic_str) {
                Ok(Some(canon)) => {
                    // 자기 자신을 덮어쓰지 않음 (재진입/replay 방어).
                    if canon.id != new_id {
                        if let Err(e) = self.store.mark_superseded(&canon.id, &new_id) {
                            tracing::warn!(
                                event_id = event.id,
                                world_id,
                                old_id = %canon.id,
                                error = %e,
                                "WorldOverlayHandler: mark_superseded failed"
                            );
                        }
                    }
                }
                Ok(None) => {
                    // 이 topic에 Canonical이 아직 없음 — 본 커맨드가 **첫** Canonical 시딩.
                }
                Err(e) => {
                    tracing::warn!(
                        event_id = event.id,
                        world_id,
                        topic = %topic_str,
                        error = %e,
                        "WorldOverlayHandler: get_canonical_by_topic failed"
                    );
                }
            }
        }

        // 2) 새 Canonical 엔트리 생성.
        //
        // Layer::A 선택 근거 (리뷰 H6): 스펙 §2.3 표에 따라 WorldEvent의 초기 Layer는 A지만
        // Consolidation **제외** 대상이다. 이 제외는 `SceneConsolidationHandler`가
        // `memory_type ∈ {DialogueTurn, BeatTransition}` 필터로 강제하며, 다른 경로에서
        // Layer-only consolidation이 추가되면 WorldEvent가 잘못 흡수될 수 있으므로
        // Consolidation 확장 시 반드시 WorldEvent 제외를 재확인해야 한다.
        #[allow(deprecated)] // Personal 투영 grand-father (§2.5 H10) — scope.owner_a()와 일치
        let entry = MemoryEntry {
            id: new_id.clone(),
            created_seq: event.id,
            event_id: event.id,
            scope: MemoryScope::World {
                world_id: world_id.clone(),
            },
            source: MemorySource::Experienced,
            provenance: Provenance::Seeded,
            memory_type: MemoryType::WorldEvent,
            layer: MemoryLayer::A,
            content: fact.clone(),
            topic: topic.clone(),
            emotional_context: None,
            timestamp_ms: event.timestamp_ms,
            last_recalled_at: None,
            recall_count: 0,
            origin_chain: vec![],
            confidence: 1.0,
            acquired_by: None,
            superseded_by: None,
            consolidated_into: None,
            npc_id: world_id.clone(),
        };

        if let Err(e) = self.store.index(entry, None) {
            tracing::warn!(
                event_id = event.id,
                world_id,
                error = %e,
                "WorldOverlayHandler: MemoryStore.index failed"
            );
        }

        Ok(HandlerResult::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::command::handler_v2::test_support::HandlerTestHarness;
    use crate::ports::{MemoryQuery, MemoryScopeFilter};
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
            let mut g = self.entries.lock().unwrap();
            if let Some(pos) = g.iter().position(|e| e.id == entry.id) {
                g[pos] = entry;
            } else {
                g.push(entry);
            }
            Ok(())
        }
        fn search_by_meaning(
            &self,
            _q: &[f32],
            _npc_id: Option<&str>,
            _limit: usize,
        ) -> Result<Vec<crate::domain::memory::MemoryResult>, crate::ports::MemoryError> {
            Ok(vec![])
        }
        fn search_by_keyword(
            &self,
            _kw: &str,
            _npc_id: Option<&str>,
            _limit: usize,
        ) -> Result<Vec<crate::domain::memory::MemoryResult>, crate::ports::MemoryError> {
            Ok(vec![])
        }
        fn get_recent(
            &self,
            _npc_id: &str,
            _limit: usize,
        ) -> Result<Vec<MemoryEntry>, crate::ports::MemoryError> {
            Ok(vec![])
        }
        fn count(&self) -> usize {
            self.entries.lock().unwrap().len()
        }
        fn search(
            &self,
            q: MemoryQuery,
        ) -> Result<Vec<crate::domain::memory::MemoryResult>, crate::ports::MemoryError> {
            // 리뷰 H3: 프로덕션 store 의미와 맞춰 scope_filter/layer_filter 전수 준수.
            let g = self.entries.lock().unwrap();
            let results = g
                .iter()
                .filter(|e| match &q.scope_filter {
                    None | Some(MemoryScopeFilter::Any) => true,
                    Some(MemoryScopeFilter::Exact(s)) => &e.scope == s,
                    Some(MemoryScopeFilter::NpcAllowed(npc)) => match &e.scope {
                        MemoryScope::Personal { npc_id } => npc_id == npc,
                        MemoryScope::World { .. } => true,
                        MemoryScope::Relationship { a, b } => a == npc || b == npc,
                        _ => false,
                    },
                })
                .filter(|e| {
                    q.topic
                        .as_ref()
                        .map(|t| e.topic.as_deref() == Some(t.as_str()))
                        .unwrap_or(true)
                })
                .filter(|e| q.layer_filter.map(|l| e.layer == l).unwrap_or(true))
                .filter(|e| !q.exclude_superseded || e.superseded_by.is_none())
                .filter(|e| !q.exclude_consolidated_source || e.consolidated_into.is_none())
                .map(|e| crate::domain::memory::MemoryResult {
                    entry: e.clone(),
                    relevance_score: 1.0,
                })
                .collect();
            Ok(results)
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
            topic: &str,
        ) -> Result<Option<MemoryEntry>, crate::ports::MemoryError> {
            let g = self.entries.lock().unwrap();
            let mut candidates: Vec<&MemoryEntry> = g
                .iter()
                .filter(|e| {
                    e.topic.as_deref() == Some(topic) && e.superseded_by.is_none()
                })
                .collect();
            candidates.sort_by(|a, b| b.created_seq.cmp(&a.created_seq));
            Ok(candidates.first().map(|e| (*e).clone()))
        }
        fn get_canonical_by_topic(
            &self,
            topic: &str,
        ) -> Result<Option<MemoryEntry>, crate::ports::MemoryError> {
            let g = self.entries.lock().unwrap();
            let mut candidates: Vec<&MemoryEntry> = g
                .iter()
                .filter(|e| {
                    e.topic.as_deref() == Some(topic)
                        && e.provenance.is_canonical(&e.scope)
                        && e.superseded_by.is_none()
                })
                .collect();
            candidates.sort_by(|a, b| b.created_seq.cmp(&a.created_seq));
            Ok(candidates.first().map(|e| (*e).clone()))
        }
        fn mark_superseded(
            &self,
            old_id: &str,
            new_id: &str,
        ) -> Result<(), crate::ports::MemoryError> {
            let mut g = self.entries.lock().unwrap();
            if let Some(e) = g.iter_mut().find(|e| e.id == old_id) {
                e.superseded_by = Some(new_id.into());
            }
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

    fn occurred(event_id: u64, world_id: &str, topic: Option<&str>, fact: &str) -> DomainEvent {
        DomainEvent::new(
            event_id,
            world_id.into(),
            1,
            EventPayload::WorldEventOccurred {
                world_id: world_id.into(),
                topic: topic.map(String::from),
                fact: fact.into(),
                significance: 0.5,
                witnesses: vec![],
            },
        )
    }

    #[test]
    fn creates_canonical_entry_with_world_scope_and_seeded_provenance() {
        let store = Arc::new(SpyStore::default());
        let handler = WorldOverlayHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();

        harness
            .dispatch(&handler, occurred(10, "jianghu", Some("leader"), "새 맹주"))
            .expect("must succeed");

        let entries = store.entries.lock().unwrap().clone();
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.source, MemorySource::Experienced);
        assert_eq!(e.provenance, Provenance::Seeded);
        assert_eq!(e.memory_type, MemoryType::WorldEvent);
        assert!(matches!(&e.scope, MemoryScope::World { world_id } if world_id == "jianghu"));
        assert_eq!(e.topic.as_deref(), Some("leader"));
        assert!(e.provenance.is_canonical(&e.scope), "Canonical 조건 충족");
    }

    #[test]
    fn supersedes_previous_same_topic_entry() {
        let store = Arc::new(SpyStore::default());
        // 기존 Canonical seed
        #[allow(deprecated)]
        let old = MemoryEntry {
            id: "old-canon".into(),
            created_seq: 1,
            event_id: 1,
            scope: MemoryScope::World {
                world_id: "jianghu".into(),
            },
            source: MemorySource::Experienced,
            provenance: Provenance::Seeded,
            memory_type: MemoryType::WorldEvent,
            layer: MemoryLayer::A,
            content: "옛 맹주".into(),
            topic: Some("leader".into()),
            emotional_context: None,
            timestamp_ms: 1,
            last_recalled_at: None,
            recall_count: 0,
            origin_chain: vec![],
            confidence: 1.0,
            acquired_by: None,
            superseded_by: None,
            consolidated_into: None,
            npc_id: "jianghu".into(),
        };
        store.index(old, None).unwrap();

        let handler = WorldOverlayHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();
        harness
            .dispatch(&handler, occurred(20, "jianghu", Some("leader"), "새 맹주"))
            .unwrap();

        let entries = store.entries.lock().unwrap().clone();
        let old_e = entries.iter().find(|e| e.id == "old-canon").unwrap();
        assert!(old_e.superseded_by.is_some(), "기존 Canonical이 supersede되어야");
        // get_canonical_by_topic은 superseded 제외 → 새 엔트리 반환
        let canon = store.get_canonical_by_topic("leader").unwrap().unwrap();
        assert_eq!(canon.content, "새 맹주");
    }

    #[test]
    fn topic_none_does_not_supersede() {
        let store = Arc::new(SpyStore::default());
        #[allow(deprecated)]
        let old = MemoryEntry {
            id: "old".into(),
            created_seq: 1,
            event_id: 1,
            scope: MemoryScope::World {
                world_id: "jianghu".into(),
            },
            source: MemorySource::Experienced,
            provenance: Provenance::Seeded,
            memory_type: MemoryType::WorldEvent,
            layer: MemoryLayer::A,
            content: "some topic fact".into(),
            topic: Some("leader".into()),
            emotional_context: None,
            timestamp_ms: 1,
            last_recalled_at: None,
            recall_count: 0,
            origin_chain: vec![],
            confidence: 1.0,
            acquired_by: None,
            superseded_by: None,
            consolidated_into: None,
            npc_id: "jianghu".into(),
        };
        store.index(old, None).unwrap();

        let handler = WorldOverlayHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();
        // topic = None → supersede 없음, 새 엔트리만 추가
        harness
            .dispatch(&handler, occurred(30, "jianghu", None, "독립 사건"))
            .unwrap();

        let entries = store.entries.lock().unwrap().clone();
        assert_eq!(entries.len(), 2);
        let old_e = entries.iter().find(|e| e.id == "old").unwrap();
        assert!(old_e.superseded_by.is_none());
    }

    #[test]
    fn non_canonical_entries_on_same_topic_are_preserved() {
        // 리뷰 B1 회귀 가드: 같은 topic의 개인 주관 기억(Heard/Rumor)은 supersede 대상 아님.
        let store = Arc::new(SpyStore::default());
        #[allow(deprecated)]
        let personal_heard = MemoryEntry {
            id: "pupil-heard".into(),
            created_seq: 1,
            event_id: 1,
            scope: MemoryScope::Personal {
                npc_id: "pupil".into(),
            },
            source: MemorySource::Heard,
            provenance: Provenance::Runtime,
            memory_type: MemoryType::DialogueTurn,
            layer: MemoryLayer::A,
            content: "나한테 전해준 이야기".into(),
            topic: Some("leader".into()),
            emotional_context: None,
            timestamp_ms: 1,
            last_recalled_at: None,
            recall_count: 0,
            origin_chain: vec!["sage".into()],
            confidence: 0.8,
            acquired_by: None,
            superseded_by: None,
            consolidated_into: None,
            npc_id: "pupil".into(),
        };
        store.index(personal_heard, None).unwrap();

        let handler = WorldOverlayHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();
        harness
            .dispatch(&handler, occurred(50, "jianghu", Some("leader"), "새 맹주 확정"))
            .unwrap();

        let entries = store.entries.lock().unwrap().clone();
        let heard = entries.iter().find(|e| e.id == "pupil-heard").unwrap();
        assert!(
            heard.superseded_by.is_none(),
            "Personal Heard 엔트리는 World 오버레이로 supersede되지 않아야 함"
        );
        // 새 World 엔트리가 추가되긴 한다
        assert_eq!(entries.len(), 2);
    }
}
