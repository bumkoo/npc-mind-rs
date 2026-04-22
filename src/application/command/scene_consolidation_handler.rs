//! SceneConsolidationHandler — Scene 종료 시 Layer A → Layer B 흡수 (Step D, Inline)
//!
//! 설계 문서: `docs/memory/03-implementation-design.md` §6.3, §8.2
//!
//! **책임**: `SceneEnded` 이벤트를 구독해 해당 Scene에서 생성된 Layer A
//! (DialogueTurn / BeatTransition) 엔트리들을 하나의 Layer B `SceneSummary` 엔트리로
//! 흡수한다. 각 Layer A 엔트리의 `consolidated_into`에 새 Layer B 엔트리 id를 기록.
//!
//! **후보 선정** (Scene 범위 탐지):
//! - Personal scope가 `npc_id` 또는 `partner_id`인 Layer A 엔트리.
//! - Relationship scope `{a, b} == {npc_id, partner_id}` 인 Layer A 엔트리.
//! - `consolidated_into`가 이미 있는 엔트리는 제외 (중복 흡수 방지).
//! - `memory_type ∈ {DialogueTurn, BeatTransition}` 만 대상 (I-ME-8,
//!   RelationshipChange/WorldEvent/FactionKnowledge/FamilyFact는 제외).
//!
//! **요약 생성** (휴리스틱, §14):
//! - 후보가 0개면 아무 것도 안 함 (no-op).
//! - 후보가 1개 이상이면 `"{turn 개수}턴 간 대화 요약: {첫 content} ... {끝 content}"` 포맷.
//!   LLM 기반 요약은 후속 Phase 과제.
//!
//! **산출**:
//! - 새 `MemoryEntry(scope=Personal{npc_id}, memory_type=SceneSummary, layer=B)` 생성.
//!   요약 엔트리는 Scene 주체 NPC의 Personal Scope로 저장 (관점 분리는 향후 과제).
//! - 각 흡수된 entry에 `consolidated_into = new_id` 마킹.
//!
//! **Inline 계약**: MemoryStore 호출 실패는 로그만. 커맨드 전체는 중단되지 않음.

use std::sync::Arc;

use crate::application::command::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::event::{DomainEvent, EventKind, EventPayload};
use crate::domain::memory::{
    MemoryEntry, MemoryLayer, MemoryScope, MemorySource, MemoryType, Provenance,
};
use crate::domain::scene_id::SceneId;
use crate::ports::{MemoryQuery, MemoryScopeFilter, MemoryStore};

pub struct SceneConsolidationHandler {
    store: Arc<dyn MemoryStore>,
}

impl SceneConsolidationHandler {
    pub fn new(store: Arc<dyn MemoryStore>) -> Self {
        Self { store }
    }

    fn derive_summary_id(event_id: u64, npc_id: &str) -> String {
        format!("summary-{event_id:012}-{npc_id}")
    }

    /// Scene 범위의 Layer A 대화·Beat 엔트리를 모은다.
    ///
    /// NpcAllowed 필터로 두 NPC의 관점을 각각 긁어와 id로 dedup한다.
    /// Personal/Relationship scope 모두 자동 포함되며, World/Faction/Family는 관점이
    /// NPC-specific하지 않으므로 요약 대상에서 제외된다(§8.2).
    fn collect_scene_entries(&self, scene: &SceneId) -> Vec<MemoryEntry> {
        let mut collected: Vec<MemoryEntry> = Vec::new();
        for npc in [scene.npc_id.as_str(), scene.partner_id.as_str()] {
            let q = MemoryQuery {
                scope_filter: Some(MemoryScopeFilter::NpcAllowed(npc.into())),
                layer_filter: Some(MemoryLayer::A),
                exclude_superseded: true,
                exclude_consolidated_source: true,
                limit: 1000,
                ..Default::default()
            };
            match self.store.search(q) {
                Ok(rs) => {
                    for r in rs {
                        // Consolidation 대상 타입만 (§8.2)
                        if !matches!(
                            r.entry.memory_type,
                            MemoryType::DialogueTurn | MemoryType::BeatTransition
                        ) {
                            continue;
                        }
                        // Scene 범위 scope 확인
                        if !Self::scope_matches_scene(&r.entry.scope, scene) {
                            continue;
                        }
                        if collected.iter().any(|e| e.id == r.entry.id) {
                            continue;
                        }
                        collected.push(r.entry);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        scene = %scene,
                        npc,
                        error = %e,
                        "SceneConsolidationHandler: search failed"
                    );
                }
            }
        }
        // 결정적 순서: timestamp 오름차순 → created_seq 오름차순
        collected.sort_by(|a, b| {
            a.timestamp_ms
                .cmp(&b.timestamp_ms)
                .then_with(|| a.created_seq.cmp(&b.created_seq))
        });
        collected
    }

    fn scope_matches_scene(scope: &MemoryScope, scene: &SceneId) -> bool {
        match scope {
            MemoryScope::Personal { npc_id } => {
                npc_id == &scene.npc_id || npc_id == &scene.partner_id
            }
            MemoryScope::Relationship { a, b } => {
                (a == &scene.npc_id && b == &scene.partner_id)
                    || (a == &scene.partner_id && b == &scene.npc_id)
            }
            _ => false,
        }
    }

    /// 휴리스틱 요약 — 첫 · 마지막 엔트리 content를 조합.
    fn summarize(entries: &[MemoryEntry]) -> String {
        if entries.is_empty() {
            return String::new();
        }
        if entries.len() == 1 {
            return format!("1턴 요약: {}", entries[0].content);
        }
        let first = &entries.first().unwrap().content;
        let last = &entries.last().unwrap().content;
        format!(
            "{}턴 간 대화 요약: {} ... {}",
            entries.len(),
            first,
            last
        )
    }
}

impl EventHandler for SceneConsolidationHandler {
    fn name(&self) -> &'static str {
        "SceneConsolidationHandler"
    }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![EventKind::SceneEnded])
    }

    fn mode(&self) -> DeliveryMode {
        DeliveryMode::Inline {
            priority: priority::inline::SCENE_CONSOLIDATION,
        }
    }

    fn handle(
        &self,
        event: &DomainEvent,
        _ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        let EventPayload::SceneEnded { npc_id, partner_id } = &event.payload else {
            return Ok(HandlerResult::default());
        };

        let scene_id = SceneId::new(npc_id.clone(), partner_id.clone());
        let entries = self.collect_scene_entries(&scene_id);
        if entries.is_empty() {
            return Ok(HandlerResult::default());
        }

        let summary_id = Self::derive_summary_id(event.id, npc_id);
        let summary_content = Self::summarize(&entries);

        // timestamp는 마지막 턴 이후 = event.timestamp_ms 사용
        #[allow(deprecated)]
        let summary_entry = MemoryEntry {
            id: summary_id.clone(),
            created_seq: event.id,
            event_id: event.id,
            scope: MemoryScope::Personal {
                npc_id: npc_id.clone(),
            },
            source: MemorySource::Experienced,
            provenance: Provenance::Runtime,
            memory_type: MemoryType::SceneSummary,
            layer: MemoryLayer::B,
            content: summary_content,
            topic: None,
            emotional_context: None,
            timestamp_ms: event.timestamp_ms,
            last_recalled_at: None,
            recall_count: 0,
            origin_chain: vec![],
            confidence: 1.0,
            acquired_by: None,
            superseded_by: None,
            consolidated_into: None,
            npc_id: npc_id.clone(),
        };

        if let Err(e) = self.store.index(summary_entry, None) {
            tracing::warn!(
                event_id = event.id,
                npc_id,
                partner_id,
                error = %e,
                "SceneConsolidationHandler: summary index failed"
            );
            return Ok(HandlerResult::default());
        }

        let a_ids: Vec<String> = entries.iter().map(|e| e.id.clone()).collect();
        if let Err(e) = self.store.mark_consolidated(&a_ids, &summary_id) {
            tracing::warn!(
                event_id = event.id,
                npc_id,
                partner_id,
                error = %e,
                "SceneConsolidationHandler: mark_consolidated failed"
            );
        }

        Ok(HandlerResult::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::command::handler_v2::test_support::HandlerTestHarness;
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
            q: MemoryQuery,
        ) -> Result<Vec<crate::domain::memory::MemoryResult>, crate::ports::MemoryError> {
            let g = self.entries.lock().unwrap();
            let out = g
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
                .filter(|e| q.layer_filter.map(|l| e.layer == l).unwrap_or(true))
                .filter(|e| !q.exclude_superseded || e.superseded_by.is_none())
                .filter(|e| !q.exclude_consolidated_source || e.consolidated_into.is_none())
                .map(|e| crate::domain::memory::MemoryResult {
                    entry: e.clone(),
                    relevance_score: 1.0,
                })
                .collect();
            Ok(out)
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
            _topic: &str,
        ) -> Result<Option<MemoryEntry>, crate::ports::MemoryError> {
            Ok(None)
        }
        fn get_canonical_by_topic(
            &self,
            _topic: &str,
        ) -> Result<Option<MemoryEntry>, crate::ports::MemoryError> {
            Ok(None)
        }
        fn mark_superseded(
            &self,
            _old: &str,
            _new: &str,
        ) -> Result<(), crate::ports::MemoryError> {
            Ok(())
        }
        fn mark_consolidated(
            &self,
            a: &[String],
            b: &str,
        ) -> Result<(), crate::ports::MemoryError> {
            let mut g = self.entries.lock().unwrap();
            for e in g.iter_mut() {
                if a.iter().any(|id| id == &e.id) {
                    e.consolidated_into = Some(b.into());
                }
            }
            Ok(())
        }
        fn record_recall(&self, _id: &str, _now_ms: u64) -> Result<(), crate::ports::MemoryError> {
            Ok(())
        }
    }

    fn layer_a_turn(id: &str, npc: &str, content: &str, ts: u64) -> MemoryEntry {
        MemoryEntry::personal(
            id,
            npc,
            content,
            None,
            ts,
            ts,
            MemoryType::DialogueTurn,
        )
    }

    fn scene_ended(event_id: u64, npc: &str, partner: &str) -> DomainEvent {
        let mut ev = DomainEvent::new(
            event_id,
            npc.into(),
            1,
            EventPayload::SceneEnded {
                npc_id: npc.into(),
                partner_id: partner.into(),
            },
        );
        ev.timestamp_ms = 9999;
        ev
    }

    #[test]
    fn consolidates_layer_a_entries_into_layer_b_summary() {
        let store = Arc::new(SpyStore::default());
        // 두 NPC의 대화 turn 3개 시드
        store
            .index(layer_a_turn("t1", "alice", "인사 나눔", 1), None)
            .unwrap();
        store
            .index(layer_a_turn("t2", "bob", "답인사", 2), None)
            .unwrap();
        store
            .index(layer_a_turn("t3", "alice", "작별", 3), None)
            .unwrap();

        let handler = SceneConsolidationHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();
        harness
            .dispatch(&handler, scene_ended(100, "alice", "bob"))
            .expect("must succeed");

        let all = store.entries.lock().unwrap().clone();
        // 3 Layer A + 1 Layer B summary = 4
        assert_eq!(all.len(), 4);
        let summary = all
            .iter()
            .find(|e| e.memory_type == MemoryType::SceneSummary)
            .unwrap();
        assert_eq!(summary.layer, MemoryLayer::B);
        assert!(summary.content.contains("요약"));

        // Layer A 엔트리 3개 모두 consolidated_into 마킹
        let consolidated_count = all
            .iter()
            .filter(|e| e.memory_type == MemoryType::DialogueTurn && e.consolidated_into.is_some())
            .count();
        assert_eq!(consolidated_count, 3, "3개 Layer A가 모두 consolidated_into 마킹");
    }

    #[test]
    fn no_layer_a_means_no_summary_created() {
        let store = Arc::new(SpyStore::default());
        let handler = SceneConsolidationHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();
        harness
            .dispatch(&handler, scene_ended(1, "alice", "bob"))
            .expect("must succeed");
        // 아무 엔트리도 없었으므로 요약도 생성되지 않음
        assert_eq!(store.entries.lock().unwrap().len(), 0);
    }

    #[test]
    fn relationship_change_entries_are_not_consolidated() {
        // RelationshipChange 타입 엔트리는 Consolidation 대상이 아님 (§8.2)
        let store = Arc::new(SpyStore::default());
        store
            .index(layer_a_turn("t1", "alice", "일반 턴", 1), None)
            .unwrap();
        let rel_entry = MemoryEntry::personal(
            "r1",
            "alice",
            "관계 변화",
            None,
            1,
            1,
            MemoryType::RelationshipChange,
        );
        store.index(rel_entry, None).unwrap();

        let handler = SceneConsolidationHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();
        harness
            .dispatch(&handler, scene_ended(200, "alice", "bob"))
            .expect("must succeed");

        let all = store.entries.lock().unwrap().clone();
        let rel_e = all.iter().find(|e| e.id == "r1").unwrap();
        assert!(rel_e.consolidated_into.is_none(), "RelationshipChange는 제외");
        let turn_e = all.iter().find(|e| e.id == "t1").unwrap();
        assert!(turn_e.consolidated_into.is_some(), "DialogueTurn은 흡수");
    }
}
