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
//! **threshold 필터**: `profile().memory_relationship_delta_threshold`(default 5.0)보다 변화량 작으면 no-op.
//! 한 이벤트로 4축(trust/affinity/respect/wariness) 모두의 Δ 중 **최대값**이 threshold 미만이면 의미
//! 없는 미세 변동으로 간주하고 기억을 남기지 않는다. 어떤 축이 주도한 변화인지
//! content에 추적용 라벨("[affinity Δ=34.0]" 등)로 포함한다 (리뷰 H4).
//!
//! **관점 분리 (TODO step-f)**: 현재 owner → target 관점의 엔트리만 만든다. target 관점
//! 엔트리는 target의 RelationshipPolicy가 따로 `RelationshipUpdated`를 발행하는 경우에만
//! 생기는데, 현재 RelationshipPolicy는 owner 관점 이벤트 1개만 내보낸다. 스펙 §6.3 line
//! 579("당사자 a, b 각각 별 엔트리")를 완전히 만족하려면 Step F에서 ①target 경로를
//! 추가로 발행하거나 ②이 handler가 target 관점 엔트리도 미러 생성하도록 확장해야 한다.
//! 후자는 "target이 이 변화를 실제로 느꼈는가"라는 도메인 판단이 필요하므로 Step F로 연기.
//!
//! **Inline 계약**: MemoryStore 에러는 로그만. 커맨드는 계속.

use std::sync::Arc;

use crate::application::command::handler_v2::{
    DeliveryMode, DynamicHandlerContext, EventHandler, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::event::{DomainEvent, EventKind, EventPayload, RelationshipChangeCause};
use crate::domain::memory::{
    MemoryEntry, MemoryLayer, MemoryScope, MemorySource, MemoryType, Provenance,
};
use crate::domain::tuning::profile;
use crate::ports::MemoryStore;

pub struct RelationshipMemoryHandler {
    store: Arc<dyn MemoryStore>,
}

impl RelationshipMemoryHandler {
    pub fn new(store: Arc<dyn MemoryStore>) -> Self {
        Self { store }
    }

    /// 결정적 엔트리 id — `(event.id, owner)` 쌍이 유일. 같은 `RelationshipUpdated` 이벤트
    /// 가 replay되면 같은 id가 산출되며 MemoryStore가 overwrite-in-place (리뷰 M3).
    fn derive_entry_id(event_id: u64, owner: &str) -> String {
        format!("rel-{event_id:012}-{owner}")
    }

    /// 네 축 중 가장 큰 변화량 + 그 축 이름 반환 (리뷰 H4, Stage 3 — 4축 확장).
    ///
    /// 모두 동률이면 trust → affinity → respect → wariness 순으로 선점 (안정 정렬).
    /// 인자 순서는 `RelationshipUpdatedPayload` 필드 순서와 정합.
    fn dominant_delta(
        bt: f32, ba: f32, br: f32, bw: f32,
        at: f32, aa: f32, ar: f32, aw: f32,
    ) -> (f32, &'static str) {
        let deltas = [
            ((at - bt).abs(), "trust"),
            ((aa - ba).abs(), "affinity"),
            ((ar - br).abs(), "respect"),
            ((aw - bw).abs(), "wariness"),
        ];
        deltas
            .into_iter()
            .fold((0.0_f32, "trust"), |acc, cur| {
                if cur.0 > acc.0 {
                    cur
                } else {
                    acc
                }
            })
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
                // `MemorySource::from_origin_chain` 계약 (§2.2):
                //   len=0 → Rumor (출처 불명), len=1 → Heard (직접 전해 들음),
                //   len≥2 → Rumor (재전파). 여기는 관계 변화 원인으로 정보 전달이
                //   명시된 경우이므로 정상 경로는 len≥1이지만, 만약 호출자가 빈
                //   체인을 넘기면 아래 from_origin_chain이 Rumor로 수렴한다.
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

    fn handle_v2(
        &self,
        event: &DomainEvent,
        _ctx: &mut dyn DynamicHandlerContext,
    ) -> Result<HandlerResult, HandlerError> {
        let EventPayload::RelationshipUpdated(p) = &event.payload
        else {
            return Ok(HandlerResult::default());
        };

        // 미세 변동은 기록하지 않음. 또한 주도 축 라벨을 content에 포함해 추적성 확보.
        let (delta, axis) = Self::dominant_delta(
            p.before_trust,
            p.before_affinity,
            p.before_respect,
            p.before_wariness,
            p.after_trust,
            p.after_affinity,
            p.after_respect,
            p.after_wariness,
        );
        if delta < profile().memory_relationship_delta_threshold {
            return Ok(HandlerResult::default());
        }

        let (source, topic, base_content) = Self::derive_from_cause(&p.cause, &p.target_id);
        let content = format!("{base_content} [{axis} Δ={delta:.2}]");

        let id = Self::derive_entry_id(event.id, &p.owner_id);
        #[allow(deprecated)] // Personal 투영 grand-father (§2.5 H10)
        let entry = MemoryEntry {
            id: id.clone(),
            created_seq: event.id,
            event_id: event.id,
            scope: MemoryScope::Personal {
                npc_id: p.owner_id.clone(),
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
            origin_chain: match &p.cause {
                RelationshipChangeCause::InformationTold { origin_chain } => origin_chain.clone(),
                RelationshipChangeCause::Rumor { rumor_id } => vec![format!("rumor:{rumor_id}")],
                _ => vec![],
            },
            confidence: 1.0,
            acquired_by: None,
            superseded_by: None,
            consolidated_into: None,
            npc_id: p.owner_id.clone(),
        };

        if let Err(e) = self.store.index(entry, None) {
            tracing::warn!(
                event_id = event.id,
                owner_id = %p.owner_id,
                target_id = %p.target_id,
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
            q: crate::ports::MemoryQuery,
        ) -> Result<Vec<crate::domain::memory::MemoryResult>, crate::ports::MemoryError> {
            use crate::ports::MemoryScopeFilter;
            let g = self.entries.lock().unwrap();
            let out: Vec<_> = g
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
        fn clear_all(&self) -> Result<(), crate::ports::MemoryError> {
            self.entries.lock().unwrap().clear();
            Ok(())
        }
    }

    fn rel_updated_event(
        event_id: u64,
        owner: &str,
        target: &str,
        delta_affinity: f32,
        cause: RelationshipChangeCause,
    ) -> DomainEvent {
        DomainEvent::new(
            event_id,
            owner.into(),
            1,
            EventPayload::RelationshipUpdated(Box::new(
                crate::domain::event::RelationshipUpdatedPayload {
                    owner_id: owner.into(),
                    target_id: target.into(),
                    before_trust: 0.0,
                    before_affinity: 0.0,
                    before_respect: 0.0,
                    before_wariness: 0.0,
                    after_trust: 0.0,
                    after_affinity: delta_affinity,
                    after_respect: 0.0,
                    after_wariness: 0.0,
                    cause,
                },
            )),
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
                    30.0,
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
                    30.0,
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
                    30.0,
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
                    30.0,
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
                    30.0,
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
                    30.0,
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

        // 0.01 변화 → MEMORY_RELATIONSHIP_DELTA_THRESHOLD=5.0 (±100 scale) 미만 → skip
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
