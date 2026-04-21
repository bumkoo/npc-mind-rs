//! TellingIngestionHandler — 정보 수신 → `MemoryEntry(Heard/Rumor)` 생성 (Step C2)
//!
//! 설계 문서: `docs/memory/03-implementation-design.md` §6.3, §8.5
//!
//! **책임**: `InformationTold` 이벤트 (청자당 1개, §3.1 B5) 를 구독해 청자의
//! 관점에서 `MemoryEntry`를 만들고 주입받은 `MemoryStore`로 영속화한다.
//!
//! **왜 Inline인가**: Transactional EventHandler 체인이 `InformationTold` follow-up을
//! commit한 뒤, Inline phase에서 projection/저장을 수행한다. Inline 핸들러의 에러는
//! 치명적이지 않고 로그만 남기므로, 저장 실패로 커맨드 전체가 롤백되지 않는다.
//!
//! **Source 결정 (§7.1 MemoryClassifier)**: 청자의 `origin_chain` = `[speaker, ...in]`.
//! - len = 1 → `Heard` (화자가 직접 경험·목격)
//! - len ≥ 2 → `Rumor` (체인이 둘 이상 — 중간 hop 존재)
//!
//! **Confidence (§8.5)**: `entry.confidence = stated_confidence × normalized_trust`.
//! `normalized_trust = (trust.value() + 1) / 2` 로 [-1, 1] → [0, 1] 매핑.
//! 관계가 없으면 0.5 (중립)로 fallback.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::application::command::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest, HandlerResult,
};
use crate::domain::event::{DomainEvent, EventKind, EventPayload};
use crate::domain::memory::{
    MemoryEntry, MemoryLayer, MemoryScope, MemorySource, MemoryType, Provenance,
};
use crate::ports::MemoryStore;

/// 청자의 관점에서 정보 전달 내용을 `MemoryEntry`로 저장하는 Inline 핸들러.
pub struct TellingIngestionHandler {
    store: Arc<dyn MemoryStore>,
    /// `mem-{NNNNNN}` 형식 엔트리 id 생성용 카운터. 프로세스 내에서 단조 증가.
    /// 분산 환경·replay는 Step F 이후 §15 결정 유보 항목.
    counter: Arc<AtomicU64>,
}

impl TellingIngestionHandler {
    pub fn new(store: Arc<dyn MemoryStore>) -> Self {
        // 기존 store에 저장된 개수로 카운터 초기화 — 같은 프로세스 내 재시작 시 id 충돌 방지.
        let start = store.count() as u64;
        Self {
            store,
            counter: Arc::new(AtomicU64::new(start)),
        }
    }

    fn next_id(&self) -> String {
        let n = self.counter.fetch_add(1, Ordering::SeqCst);
        format!("mem-{n:06}")
    }
}

impl EventHandler for TellingIngestionHandler {
    fn name(&self) -> &'static str {
        "TellingIngestionHandler"
    }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![EventKind::InformationTold])
    }

    fn mode(&self) -> DeliveryMode {
        // Inline priority는 projection 이후여도 무방 — MemoryStore 인덱싱이 projection
        // 쿼리 일관성에 영향을 주지 않으므로 가장 뒤(Scene projection 30보다 뒤)에 배치.
        DeliveryMode::Inline { priority: 40 }
    }

    fn handle(
        &self,
        event: &DomainEvent,
        ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        let EventPayload::InformationTold {
            speaker,
            listener,
            listener_role: _,
            claim,
            stated_confidence,
            origin_chain_in,
        } = &event.payload
        else {
            return Ok(HandlerResult::default());
        };

        // 청자의 origin_chain: [speaker, ...inherited]
        let mut chain = Vec::with_capacity(origin_chain_in.len() + 1);
        chain.push(speaker.clone());
        chain.extend(origin_chain_in.iter().cloned());

        let source = MemorySource::from_origin_chain(chain.len(), None);

        // confidence = stated × normalized_trust
        // normalized_trust = (trust.value() + 1) / 2, 관계 없으면 0.5
        let normalized_trust = ctx
            .repo
            .get_relationship(listener, speaker)
            .map(|r| (r.trust().value() + 1.0) / 2.0)
            .unwrap_or(0.5);
        let confidence = (stated_confidence * normalized_trust).clamp(0.0, 1.0);

        let id = self.next_id();
        #[allow(deprecated)] // Grand-fathered Personal 투영 (§2.5 H10) — scope.owner_a()와 일치.
        let entry = MemoryEntry {
            id: id.clone(),
            created_seq: event.id,
            event_id: event.id,
            scope: MemoryScope::Personal {
                npc_id: listener.clone(),
            },
            source,
            provenance: Provenance::Runtime,
            memory_type: MemoryType::DialogueTurn,
            layer: MemoryLayer::A,
            content: claim.clone(),
            topic: None,
            emotional_context: None,
            timestamp_ms: event.timestamp_ms,
            last_recalled_at: None,
            recall_count: 0,
            origin_chain: chain,
            confidence,
            acquired_by: None,
            superseded_by: None,
            consolidated_into: None,
            npc_id: listener.clone(),
        };

        if let Err(e) = self.store.index(entry, None) {
            // Inline 계약: 에러는 로그만, 커맨드 전체는 계속.
            tracing::warn!(
                event_id = event.id,
                listener,
                error = %e,
                "TellingIngestionHandler: MemoryStore.index failed"
            );
        }

        Ok(HandlerResult::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::command::handler_v2::test_support::HandlerTestHarness;
    use crate::domain::event::ListenerRole;
    use crate::domain::personality::{NpcBuilder, Score};
    use crate::domain::relationship::Relationship;
    use crate::ports::MemoryQuery;

    /// 테스트용 인메모리 MemoryStore. 실제 SqliteMemoryStore는 embed feature 전용이라
    /// 여기서는 test_support의 InMemoryMemoryStore와 동등한 간이 스토어를 쓴다.
    /// (기존 tests/common/in_memory_store.rs는 통합 테스트 전용이므로, 단위 테스트에서는
    /// 카운트와 최근 추가 엔트리만 검증할 수 있는 최소 스파이를 사용.)
    #[derive(Default)]
    struct SpyStore {
        inner: std::sync::Mutex<Vec<MemoryEntry>>,
    }

    impl MemoryStore for SpyStore {
        fn index(
            &self,
            entry: MemoryEntry,
            _embedding: Option<Vec<f32>>,
        ) -> Result<(), crate::ports::MemoryError> {
            self.inner.lock().unwrap().push(entry);
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
            self.inner.lock().unwrap().len()
        }

        fn search(
            &self,
            _query: MemoryQuery,
        ) -> Result<Vec<crate::domain::memory::MemoryResult>, crate::ports::MemoryError> {
            Ok(vec![])
        }

        fn get_by_id(
            &self,
            id: &str,
        ) -> Result<Option<MemoryEntry>, crate::ports::MemoryError> {
            Ok(self
                .inner
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
            _old_id: &str,
            _new_id: &str,
        ) -> Result<(), crate::ports::MemoryError> {
            Ok(())
        }

        fn mark_consolidated(
            &self,
            _a_ids: &[String],
            _b_id: &str,
        ) -> Result<(), crate::ports::MemoryError> {
            Ok(())
        }

        fn record_recall(
            &self,
            _id: &str,
            _now_ms: u64,
        ) -> Result<(), crate::ports::MemoryError> {
            Ok(())
        }
    }

    fn told_event(speaker: &str, listener: &str, chain_in: &[&str], stated: f32) -> DomainEvent {
        DomainEvent::new(
            42,
            listener.into(),
            1,
            EventPayload::InformationTold {
                speaker: speaker.into(),
                listener: listener.into(),
                listener_role: ListenerRole::Direct,
                claim: "장문인이 바뀐다".into(),
                stated_confidence: stated,
                origin_chain_in: chain_in.iter().map(|s| s.to_string()).collect(),
            },
        )
    }

    #[test]
    fn chain_length_one_classifies_as_heard() {
        let store = Arc::new(SpyStore::default());
        let handler = TellingIngestionHandler::new(store.clone());

        let mut harness = HandlerTestHarness::new()
            .with_npc(NpcBuilder::new("sage", "Sage").build())
            .with_npc(NpcBuilder::new("pupil", "Pupil").build())
            .with_relationship(Relationship::neutral("pupil", "sage"));

        harness
            .dispatch(&handler, told_event("sage", "pupil", &[], 1.0))
            .expect("handler must succeed");

        let saved = store.inner.lock().unwrap().clone();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].source, MemorySource::Heard);
        // 체인은 [speaker] + inherited → 화자만 있음
        assert_eq!(saved[0].origin_chain, vec!["sage".to_string()]);
    }

    #[test]
    fn chain_length_two_plus_classifies_as_rumor() {
        let store = Arc::new(SpyStore::default());
        let handler = TellingIngestionHandler::new(store.clone());

        let mut harness = HandlerTestHarness::new()
            .with_npc(NpcBuilder::new("relay", "Relay").build())
            .with_npc(NpcBuilder::new("final", "Final").build())
            .with_relationship(Relationship::neutral("final", "relay"));

        harness
            .dispatch(
                &handler,
                told_event("relay", "final", &["original-source"], 1.0),
            )
            .expect("handler must succeed");

        let saved = store.inner.lock().unwrap().clone();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].source, MemorySource::Rumor);
        assert_eq!(
            saved[0].origin_chain,
            vec!["relay".to_string(), "original-source".to_string()]
        );
    }

    #[test]
    fn confidence_applies_trust_multiplier() {
        let store = Arc::new(SpyStore::default());
        let handler = TellingIngestionHandler::new(store.clone());

        // trust = 0.6 → normalized = 0.8, stated = 0.5 → confidence = 0.4
        let rel = Relationship::new(
            "pupil",
            "sage",
            Score::new(0.0, "closeness").unwrap(),
            Score::new(0.6, "trust").unwrap(),
            Score::new(0.0, "power").unwrap(),
        );

        let mut harness = HandlerTestHarness::new()
            .with_npc(NpcBuilder::new("sage", "Sage").build())
            .with_npc(NpcBuilder::new("pupil", "Pupil").build())
            .with_relationship(rel);

        harness
            .dispatch(&handler, told_event("sage", "pupil", &[], 0.5))
            .expect("handler must succeed");

        let saved = store.inner.lock().unwrap().clone();
        assert_eq!(saved.len(), 1);
        assert!(
            (saved[0].confidence - 0.4).abs() < 1e-6,
            "confidence expected ~0.4, got {}",
            saved[0].confidence
        );
    }

    #[test]
    fn missing_relationship_falls_back_to_neutral_trust() {
        // 관계 없음 → normalized_trust = 0.5, stated = 0.8 → confidence = 0.4
        let store = Arc::new(SpyStore::default());
        let handler = TellingIngestionHandler::new(store.clone());

        let mut harness = HandlerTestHarness::new()
            .with_npc(NpcBuilder::new("stranger", "Stranger").build())
            .with_npc(NpcBuilder::new("pupil", "Pupil").build());

        harness
            .dispatch(
                &handler,
                told_event("stranger", "pupil", &[], 0.8),
            )
            .expect("handler must succeed");

        let saved = store.inner.lock().unwrap().clone();
        assert!((saved[0].confidence - 0.4).abs() < 1e-6);
    }

    #[test]
    fn entry_scope_is_personal_listener() {
        let store = Arc::new(SpyStore::default());
        let handler = TellingIngestionHandler::new(store.clone());

        let mut harness = HandlerTestHarness::new()
            .with_npc(NpcBuilder::new("sage", "Sage").build())
            .with_npc(NpcBuilder::new("pupil", "Pupil").build())
            .with_relationship(Relationship::neutral("pupil", "sage"));

        harness
            .dispatch(&handler, told_event("sage", "pupil", &[], 1.0))
            .expect("handler must succeed");

        let entry = store.inner.lock().unwrap()[0].clone();
        assert_eq!(
            entry.scope,
            MemoryScope::Personal {
                npc_id: "pupil".into()
            }
        );
        assert_eq!(entry.layer, MemoryLayer::A);
        assert_eq!(entry.provenance, Provenance::Runtime);
    }

    #[test]
    fn ignores_unrelated_event_kind() {
        let store = Arc::new(SpyStore::default());
        let handler = TellingIngestionHandler::new(store.clone());
        let mut harness = HandlerTestHarness::new();
        let event = DomainEvent::new(
            0,
            "x".into(),
            0,
            EventPayload::EmotionCleared { npc_id: "x".into() },
        );
        harness.dispatch(&handler, event).unwrap();
        assert_eq!(store.count(), 0);
    }
}
