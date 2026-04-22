//! SceneConsolidationHandler — Scene 종료 시 Layer A → Layer B 흡수 (Step D, Inline)
//!
//! 설계 문서: `docs/memory/03-implementation-design.md` §6.3, §8.2
//!
//! **책임**: `SceneEnded` 이벤트를 구독해 해당 Scene에서 생성된 Layer A
//! (DialogueTurn / BeatTransition) 엔트리들을 각 참여 NPC 관점의 Layer B
//! `SceneSummary` 엔트리로 흡수한다.
//!
//! **관점 분리** (리뷰 B3): 이전 구현은 한쪽 NPC의 Personal Scope로 단일 summary를 만들고
//! 양쪽 Layer A를 모두 그쪽으로 `consolidated_into` 링크해서 partner 관점에서는 자신의
//! Layer A가 "상대 NPC의 Personal 요약"을 가리키는 비대칭이 생겼다. 이제:
//! - 각 참여 NPC마다 **자기 Layer A만 흡수하는 Personal Scope 요약 1건**을 생성한다.
//! - Layer A가 없는 NPC는 summary를 만들지 않는다.
//! - 결과: Scene당 최대 2개의 Layer B `SceneSummary`가 만들어지며, 각 NPC의
//!   `consolidated_into` 체인은 항상 자기 Personal Scope 안에서 닫힌다.
//!
//! **후보 선정** (per-NPC):
//! - 해당 NPC의 Personal Scope Layer A + 해당 NPC가 참여한 Relationship Scope Layer A.
//!   (`MemoryScopeFilter::NpcAllowed(npc)`는 이 둘과 World를 모두 반환하므로, World는
//!   사후 scope 체크로 제외한다.)
//! - `consolidated_into`가 이미 있는 엔트리는 제외 (중복 흡수 방지).
//! - `memory_type ∈ {DialogueTurn, BeatTransition}` 만 대상 (I-ME-8,
//!   RelationshipChange/WorldEvent/FactionKnowledge/FamilyFact는 제외).
//! - Scene 범위 scope 확인 (다른 Scene의 엔트리가 NpcAllowed로 딸려오는 경우 배제).
//!
//! **요약 생성** (휴리스틱, §14):
//! - 후보가 0개면 해당 NPC는 skip.
//! - 후보가 1개 이상이면 `"{count}턴 간 대화 요약: {첫 content} ... {끝 content}"`.
//!   긴 content는 `SUMMARY_ENTRY_SNIPPET_CAP`으로 잘라낸다 (리뷰 L3).
//!   LLM 기반 요약은 후속 Phase 과제.
//!
//! **산출**:
//! - NPC별 `MemoryEntry(scope=Personal{npc}, memory_type=SceneSummary, layer=B)`
//!   + `topic=Some("scene:{npc}:{partner}")` (리뷰 M7 — 후속 검색 편의).
//! - 자기 Layer A 엔트리들에 `consolidated_into = self_summary_id` 마킹.
//!
//! **Inline 계약**: MemoryStore 호출 실패는 로그만. 커맨드 전체는 중단되지 않음.
//! 한 NPC 수집이 실패해도 다른 NPC의 요약은 여전히 생성된다 (리뷰 H2는 per-NPC 단위에서
//! 반쪽 데이터 방지로 해석: 한 NPC의 search가 실패하면 그 NPC의 summary는 skip).

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

/// 요약 content에 포함하는 첫/끝 content 스니펫 최대 문자 수 (리뷰 L3).
const SUMMARY_ENTRY_SNIPPET_CAP: usize = 120;

pub struct SceneConsolidationHandler {
    store: Arc<dyn MemoryStore>,
}

impl SceneConsolidationHandler {
    pub fn new(store: Arc<dyn MemoryStore>) -> Self {
        Self { store }
    }

    /// 요약 엔트리 id — NPC별 고유 (리뷰 B3, M3).
    /// `(event.id, npc_id)` 쌍이 유일하므로 결정적이며, replay 시 overwrite-in-place.
    fn derive_summary_id(event_id: u64, npc_id: &str) -> String {
        format!("summary-{event_id:012}-{npc_id}")
    }

    /// Scene summary 전용 topic — 후속 `get_by_topic_latest` 편의 (리뷰 M7).
    /// `SceneId`는 (npc, partner) 방향이 유의미하므로 양방향 쌍을 정규화해
    /// 두 NPC 요약이 같은 topic 아래에서 조회 가능하게 한다.
    fn derive_scene_topic(scene: &SceneId) -> String {
        let (a, b) = if scene.npc_id <= scene.partner_id {
            (scene.npc_id.as_str(), scene.partner_id.as_str())
        } else {
            (scene.partner_id.as_str(), scene.npc_id.as_str())
        };
        format!("scene:{a}:{b}")
    }

    /// **한 NPC의 관점**에서 Scene 범위 Layer A 대화·Beat 엔트리를 모은다.
    ///
    /// 반환 규칙:
    /// - `Ok(entries)`: 검색 성공. entries는 비어 있을 수 있다.
    /// - `Err(())`: 저장소 search 실패. 호출자는 이 NPC의 summary를 skip해야 한다
    ///   (리뷰 H2 — 반쪽짜리 summary 생성 방지).
    #[allow(clippy::result_unit_err)]
    fn collect_entries_for(
        &self,
        scene: &SceneId,
        npc: &str,
    ) -> Result<Vec<MemoryEntry>, ()> {
        let q = MemoryQuery {
            scope_filter: Some(MemoryScopeFilter::NpcAllowed(npc.into())),
            layer_filter: Some(MemoryLayer::A),
            exclude_superseded: true,
            exclude_consolidated_source: true,
            limit: 1000,
            ..Default::default()
        };
        let rs = match self.store.search(q) {
            Ok(rs) => rs,
            Err(e) => {
                tracing::warn!(
                    scene = %scene,
                    npc,
                    error = %e,
                    "SceneConsolidationHandler: search failed"
                );
                return Err(());
            }
        };
        let mut out: Vec<MemoryEntry> = Vec::new();
        for r in rs {
            if !matches!(
                r.entry.memory_type,
                MemoryType::DialogueTurn | MemoryType::BeatTransition
            ) {
                continue;
            }
            if !Self::scope_belongs_to_npc_in_scene(&r.entry.scope, scene, npc) {
                continue;
            }
            if out.iter().any(|e| e.id == r.entry.id) {
                continue;
            }
            out.push(r.entry);
        }
        // 결정적 순서: timestamp 오름차순 → created_seq 오름차순.
        out.sort_by(|a, b| {
            a.timestamp_ms
                .cmp(&b.timestamp_ms)
                .then_with(|| a.created_seq.cmp(&b.created_seq))
        });
        Ok(out)
    }

    /// 특정 NPC 관점에서 이 scope 엔트리가 이 Scene의 자기 몫인지 판정.
    ///
    /// - Personal{X}: X == 이 NPC
    /// - Relationship{a, b}: {a, b} == {npc, scene의 상대}
    /// - World/Faction/Family: 관점 비특정 → 요약 대상 아님 (§8.2)
    ///
    /// Relationship 대칭 정규화(a ≤ b)는 `MemoryScope::relationship` 생성자에서 강제되지만,
    /// 방어적으로 양방향 모두 체크한다 (리뷰 M5).
    fn scope_belongs_to_npc_in_scene(
        scope: &MemoryScope,
        scene: &SceneId,
        npc: &str,
    ) -> bool {
        let other = if npc == scene.npc_id {
            scene.partner_id.as_str()
        } else if npc == scene.partner_id {
            scene.npc_id.as_str()
        } else {
            return false;
        };
        match scope {
            MemoryScope::Personal { npc_id } => npc_id == npc,
            MemoryScope::Relationship { a, b } => {
                (a == npc && b == other) || (a == other && b == npc)
            }
            _ => false,
        }
    }

    /// 휴리스틱 요약 — 첫 · 마지막 엔트리 content를 조합 (긴 content는 cap).
    fn summarize(entries: &[MemoryEntry]) -> String {
        if entries.is_empty() {
            return String::new();
        }
        if entries.len() == 1 {
            return format!("1턴 요약: {}", truncate(&entries[0].content));
        }
        let first = truncate(&entries.first().unwrap().content);
        let last = truncate(&entries.last().unwrap().content);
        format!("{}턴 간 대화 요약: {} ... {}", entries.len(), first, last)
    }
}

/// UTF-8 안전한 char 기준 truncate (리뷰 L3).
fn truncate(s: &str) -> String {
    if s.chars().count() <= SUMMARY_ENTRY_SNIPPET_CAP {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(SUMMARY_ENTRY_SNIPPET_CAP).collect();
        format!("{truncated}…")
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
        let topic = Self::derive_scene_topic(&scene_id);

        // self-Scene(npc == partner) 가드 (리뷰 H1) — dedup된 participant 목록.
        let mut participants: Vec<&str> = vec![npc_id.as_str()];
        if partner_id != npc_id {
            participants.push(partner_id.as_str());
        }

        for npc in participants {
            // 한 NPC의 수집 실패는 해당 NPC summary만 skip — 다른 NPC는 여전히 처리됨.
            let entries = match self.collect_entries_for(&scene_id, npc) {
                Ok(es) => es,
                Err(()) => continue,
            };
            if entries.is_empty() {
                continue;
            }

            let summary_id = Self::derive_summary_id(event.id, npc);
            let summary_content = Self::summarize(&entries);

            #[allow(deprecated)] // Personal grand-father — scope.owner_a()와 일치
            let summary_entry = MemoryEntry {
                id: summary_id.clone(),
                created_seq: event.id,
                event_id: event.id,
                scope: MemoryScope::Personal {
                    npc_id: npc.into(),
                },
                source: MemorySource::Experienced,
                provenance: Provenance::Runtime,
                memory_type: MemoryType::SceneSummary,
                layer: MemoryLayer::B,
                content: summary_content,
                topic: Some(topic.clone()),
                emotional_context: None,
                timestamp_ms: event.timestamp_ms,
                last_recalled_at: None,
                recall_count: 0,
                origin_chain: vec![],
                confidence: 1.0,
                acquired_by: None,
                superseded_by: None,
                consolidated_into: None,
                npc_id: npc.into(),
            };

            if let Err(e) = self.store.index(summary_entry, None) {
                tracing::warn!(
                    event_id = event.id,
                    npc,
                    error = %e,
                    "SceneConsolidationHandler: summary index failed"
                );
                continue;
            }

            let a_ids: Vec<String> = entries.iter().map(|e| e.id.clone()).collect();
            if let Err(e) = self.store.mark_consolidated(&a_ids, &summary_id) {
                tracing::warn!(
                    event_id = event.id,
                    npc,
                    error = %e,
                    "SceneConsolidationHandler: mark_consolidated failed"
                );
            }
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
    fn per_participant_summary_splits_layer_a_by_owner() {
        // 리뷰 B3: 각 NPC의 Personal Scope 안에서만 consolidated_into 링크가 닫혀야 한다.
        let store = Arc::new(SpyStore::default());
        store
            .index(layer_a_turn("t1", "alice", "alice 인사", 1), None)
            .unwrap();
        store
            .index(layer_a_turn("t2", "bob", "bob 답인사", 2), None)
            .unwrap();
        store
            .index(layer_a_turn("t3", "alice", "alice 작별", 3), None)
            .unwrap();

        let handler = SceneConsolidationHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();
        harness
            .dispatch(&handler, scene_ended(100, "alice", "bob"))
            .expect("must succeed");

        let all = store.entries.lock().unwrap().clone();
        // 3 Layer A + 2 Layer B summary (per-NPC)
        assert_eq!(all.len(), 5);
        let summaries: Vec<&MemoryEntry> = all
            .iter()
            .filter(|e| e.memory_type == MemoryType::SceneSummary)
            .collect();
        assert_eq!(summaries.len(), 2, "NPC당 1개 summary (총 2개)");

        // alice summary는 alice Personal Scope, bob summary는 bob Personal Scope
        let alice_summary = summaries
            .iter()
            .find(|e| matches!(&e.scope, MemoryScope::Personal { npc_id } if npc_id == "alice"))
            .expect("alice의 summary");
        let bob_summary = summaries
            .iter()
            .find(|e| matches!(&e.scope, MemoryScope::Personal { npc_id } if npc_id == "bob"))
            .expect("bob의 summary");

        // topic이 "scene:alice:bob"로 정규화 (a ≤ b)
        assert_eq!(alice_summary.topic.as_deref(), Some("scene:alice:bob"));
        assert_eq!(bob_summary.topic.as_deref(), Some("scene:alice:bob"));

        // alice 소속 엔트리(t1, t3)는 alice summary를 가리키고, bob 소속(t2)은 bob summary를 가리킴
        let t1 = all.iter().find(|e| e.id == "t1").unwrap();
        let t2 = all.iter().find(|e| e.id == "t2").unwrap();
        let t3 = all.iter().find(|e| e.id == "t3").unwrap();
        assert_eq!(t1.consolidated_into.as_deref(), Some(alice_summary.id.as_str()));
        assert_eq!(t3.consolidated_into.as_deref(), Some(alice_summary.id.as_str()));
        assert_eq!(t2.consolidated_into.as_deref(), Some(bob_summary.id.as_str()));
    }

    #[test]
    fn no_layer_a_means_no_summary_created() {
        let store = Arc::new(SpyStore::default());
        let handler = SceneConsolidationHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();
        harness
            .dispatch(&handler, scene_ended(1, "alice", "bob"))
            .expect("must succeed");
        assert_eq!(store.entries.lock().unwrap().len(), 0);
    }

    #[test]
    fn one_sided_layer_a_creates_only_that_npcs_summary() {
        // alice만 Layer A 엔트리가 있으면 alice summary만 생성되고 bob summary는 없다.
        let store = Arc::new(SpyStore::default());
        store
            .index(layer_a_turn("t1", "alice", "일방적 독백", 1), None)
            .unwrap();
        let handler = SceneConsolidationHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();
        harness
            .dispatch(&handler, scene_ended(101, "alice", "bob"))
            .expect("must succeed");
        let summary_count = store
            .entries
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.memory_type == MemoryType::SceneSummary)
            .count();
        assert_eq!(summary_count, 1);
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

    #[test]
    fn self_scene_deduplicates_participant() {
        // 리뷰 H1: npc_id == partner_id인 self-scene도 무한루프/이중 summary 없이 처리.
        let store = Arc::new(SpyStore::default());
        store
            .index(layer_a_turn("t1", "alice", "독백", 1), None)
            .unwrap();
        let handler = SceneConsolidationHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();
        harness
            .dispatch(&handler, scene_ended(300, "alice", "alice"))
            .expect("must succeed");
        let summary_count = store
            .entries
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.memory_type == MemoryType::SceneSummary)
            .count();
        assert_eq!(summary_count, 1, "self-scene은 summary 1개만");
    }
}
