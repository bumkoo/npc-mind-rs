//! RumorDistributionHandler — 소문 확산 → 수신자별 MemoryEntry 생성 (Step C3, Inline)
//!
//! 설계 문서: `docs/memory/03-implementation-design.md` §6.3, §8.6
//!
//! **책임**: `RumorSpread` 이벤트를 구독해 각 수신자의 관점에서
//! `MemoryEntry(source=Rumor, scope=Personal)`를 `MemoryStore`에 저장한다.
//!
//! **콘텐츠 해소**:
//! 1. `content_version`이 있으면 해당 Distortion 본문
//! 2. 없으면 `rumor.topic`의 Canonical MemoryEntry 본문 (`MemoryStore::get_canonical_by_topic`)
//! 3. Canonical 없으면 `rumor.seed_content`
//! 4. 모두 없으면 `"[내용 없음]"` 플레이스홀더 (현실 시나리오에서는 도달하지 않아야 함)
//!
//! **Confidence 감쇠**: `RUMOR_HOP_CONFIDENCE_DECAY ^ hop_index`. 홉이 깊어질수록
//! 수신자의 저장 confidence가 기하 감소. `RUMOR_MIN_CONFIDENCE` 하한.
//!
//! **저장 실패**: Inline 계약대로 `tracing::warn!`만. 커맨드 전체는 중단되지 않는다.
//! I-RU-5 "트랜잭션 일관성" 직접 구현은 장기 과제 — 현재는 best-effort.

use std::sync::Arc;

use crate::application::command::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest, HandlerResult,
};
use crate::domain::event::{DomainEvent, EventKind, EventPayload};
use crate::domain::memory::{
    MemoryEntry, MemoryLayer, MemoryScope, MemorySource, MemoryType, Provenance,
};
use crate::domain::tuning::{RUMOR_HOP_CONFIDENCE_DECAY, RUMOR_MIN_CONFIDENCE};
use crate::ports::{MemoryStore, RumorStore};

pub struct RumorDistributionHandler {
    memory_store: Arc<dyn MemoryStore>,
    rumor_store: Arc<dyn RumorStore>,
}

impl RumorDistributionHandler {
    pub fn new(memory_store: Arc<dyn MemoryStore>, rumor_store: Arc<dyn RumorStore>) -> Self {
        Self {
            memory_store,
            rumor_store,
        }
    }

    fn derive_entry_id(event_id: u64, recipient: &str) -> String {
        format!("mem-{event_id:012}-{recipient}")
    }

    /// 수신자에게 노출될 콘텐츠 확정.
    fn resolve_content(&self, rumor_id: &str, content_version: &Option<String>) -> String {
        // 1) content_version — 해당 Distortion
        if let Some(cv) = content_version {
            if let Ok(Some(rumor)) = self.rumor_store.load(rumor_id) {
                if let Some(d) = rumor.distortions().iter().find(|d| &d.id == cv) {
                    return d.content.clone();
                }
            }
        }
        // 2)(3) rumor의 topic이 있으면 Canonical 조회, 없으면 seed_content fallback
        if let Ok(Some(rumor)) = self.rumor_store.load(rumor_id) {
            if let Some(topic) = &rumor.topic {
                if let Ok(Some(canon)) = self.memory_store.get_canonical_by_topic(topic) {
                    return canon.content;
                }
            }
            if let Some(seed) = &rumor.seed_content {
                return seed.clone();
            }
        }
        "[내용 없음]".into()
    }

    fn confidence_for_hop(hop_index: u32) -> f32 {
        // 1.0 × decay^hop_index, 하한 clamp.
        let raw = RUMOR_HOP_CONFIDENCE_DECAY.powi(hop_index as i32);
        raw.max(RUMOR_MIN_CONFIDENCE)
    }
}

impl EventHandler for RumorDistributionHandler {
    fn name(&self) -> &'static str {
        "RumorDistributionHandler"
    }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![EventKind::RumorSpread])
    }

    fn mode(&self) -> DeliveryMode {
        // TellingIngestionHandler(40)와 동일 우선순위 — 서로 다른 이벤트 kind이므로 실행 순서
        // 의존성 없음. 별도 상수 도입은 리뷰 N1과 함께 추후 정리.
        DeliveryMode::Inline { priority: 40 }
    }

    fn handle(
        &self,
        event: &DomainEvent,
        _ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        let EventPayload::RumorSpread {
            rumor_id,
            hop_index,
            recipients,
            content_version,
        } = &event.payload
        else {
            return Ok(HandlerResult::default());
        };

        let content = self.resolve_content(rumor_id, content_version);
        let confidence = Self::confidence_for_hop(*hop_index);
        // topic은 저장 시 필요하므로 한 번 조회.
        let rumor = self.rumor_store.load(rumor_id).ok().flatten();
        let topic = rumor.as_ref().and_then(|r| r.topic.clone());

        for recipient in recipients {
            let id = Self::derive_entry_id(event.id, recipient);
            // Rumor 수신 시 청자의 origin_chain은 rumor_id 자체를 마커로 사용(추후 Source 확장 시 세분화).
            let chain = vec![format!("rumor:{rumor_id}")];
            #[allow(deprecated)] // Personal 투영 grand-father (§2.5 H10)
            let entry = MemoryEntry {
                id: id.clone(),
                created_seq: event.id,
                event_id: event.id,
                scope: MemoryScope::Personal {
                    npc_id: recipient.clone(),
                },
                source: MemorySource::Rumor,
                provenance: Provenance::Runtime,
                memory_type: MemoryType::DialogueTurn,
                layer: MemoryLayer::A,
                content: content.clone(),
                topic: topic.clone(),
                emotional_context: None,
                timestamp_ms: event.timestamp_ms,
                last_recalled_at: None,
                recall_count: 0,
                origin_chain: chain,
                confidence,
                acquired_by: None,
                superseded_by: None,
                consolidated_into: None,
                npc_id: recipient.clone(),
            };

            if let Err(e) = self.memory_store.index(entry, None) {
                tracing::warn!(
                    event_id = event.id,
                    rumor_id,
                    recipient,
                    error = %e,
                    "RumorDistributionHandler: MemoryStore.index failed"
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
    use crate::domain::rumor::{ReachPolicy, Rumor, RumorOrigin};
    use crate::ports::MemoryQuery;
    use std::sync::Mutex;

    #[derive(Default)]
    struct SpyMemoryStore {
        inner: Mutex<Vec<MemoryEntry>>,
        canonical: Mutex<Option<MemoryEntry>>,
    }

    impl MemoryStore for SpyMemoryStore {
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
            _q: MemoryQuery,
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
            Ok(self.canonical.lock().unwrap().clone())
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
            _a: &[String],
            _b: &str,
        ) -> Result<(), crate::ports::MemoryError> {
            Ok(())
        }
        fn record_recall(&self, _id: &str, _now_ms: u64) -> Result<(), crate::ports::MemoryError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct SpyRumorStore {
        inner: Mutex<Vec<Rumor>>,
    }

    impl RumorStore for SpyRumorStore {
        fn save(&self, rumor: &Rumor) -> Result<(), crate::ports::MemoryError> {
            let mut g = self.inner.lock().unwrap();
            if let Some(pos) = g.iter().position(|r| r.id == rumor.id) {
                g[pos] = rumor.clone();
            } else {
                g.push(rumor.clone());
            }
            Ok(())
        }
        fn load(&self, id: &str) -> Result<Option<Rumor>, crate::ports::MemoryError> {
            Ok(self.inner.lock().unwrap().iter().find(|r| r.id == id).cloned())
        }
        fn find_by_topic(&self, _topic: &str) -> Result<Vec<Rumor>, crate::ports::MemoryError> {
            Ok(vec![])
        }
        fn find_active_in_reach(
            &self,
            _reach: &ReachPolicy,
        ) -> Result<Vec<Rumor>, crate::ports::MemoryError> {
            Ok(vec![])
        }
    }

    fn spread_event(event_id: u64, rumor_id: &str, hop: u32, recipients: &[&str]) -> DomainEvent {
        let mut ev = DomainEvent::new(
            event_id,
            rumor_id.into(),
            1,
            EventPayload::RumorSpread {
                rumor_id: rumor_id.into(),
                hop_index: hop,
                recipients: recipients.iter().map(|s| s.to_string()).collect(),
                content_version: None,
            },
        );
        ev.timestamp_ms = 5000;
        ev
    }

    #[test]
    fn spread_creates_personal_rumor_entry_per_recipient() {
        let mem = Arc::new(SpyMemoryStore::default());
        let rum = Arc::new(SpyRumorStore::default());
        rum.save(&Rumor::with_forecast_content(
            "r1",
            "topic",
            "원본",
            RumorOrigin::Seeded,
            ReachPolicy::default(),
            0,
        ))
        .unwrap();

        let handler = RumorDistributionHandler::new(mem.clone(), rum);
        let mut harness = HandlerTestHarness::new();
        harness
            .dispatch(&handler, spread_event(7, "r1", 0, &["a", "b", "c"]))
            .unwrap();

        let entries = mem.inner.lock().unwrap().clone();
        assert_eq!(entries.len(), 3);
        for (e, expected) in entries.iter().zip(&["a", "b", "c"]) {
            assert_eq!(e.source, MemorySource::Rumor);
            assert!(matches!(
                &e.scope,
                MemoryScope::Personal { npc_id } if npc_id == expected
            ));
            assert_eq!(e.topic.as_deref(), Some("topic"));
            assert_eq!(e.origin_chain, vec!["rumor:r1".to_string()]);
        }
    }

    #[test]
    fn confidence_decays_geometrically_with_hop_index() {
        let mem = Arc::new(SpyMemoryStore::default());
        let rum = Arc::new(SpyRumorStore::default());
        rum.save(&Rumor::orphan(
            "r",
            "seed",
            RumorOrigin::Seeded,
            ReachPolicy::default(),
            0,
        ))
        .unwrap();
        let handler = RumorDistributionHandler::new(mem.clone(), rum);
        let mut harness = HandlerTestHarness::new();

        harness.dispatch(&handler, spread_event(1, "r", 0, &["a"])).unwrap();
        harness.dispatch(&handler, spread_event(2, "r", 2, &["b"])).unwrap();

        let entries = mem.inner.lock().unwrap().clone();
        let a = &entries[0];
        let b = &entries[1];
        assert!((a.confidence - 1.0).abs() < 1e-6, "hop 0 → decay^0 = 1.0");
        // decay=0.8, hop=2 → 0.64
        assert!((b.confidence - 0.64).abs() < 1e-5, "hop 2 → 0.64 (got {})", b.confidence);
        assert!(b.confidence < a.confidence);
    }

    #[test]
    fn content_resolution_uses_canonical_when_available() {
        let mem = Arc::new(SpyMemoryStore::default());
        let rum = Arc::new(SpyRumorStore::default());
        rum.save(&Rumor::new(
            "r",
            "t",
            RumorOrigin::Seeded,
            ReachPolicy::default(),
            0,
        ))
        .unwrap();
        // Canonical 등록
        #[allow(deprecated)]
        let canon = MemoryEntry {
            id: "canon-1".into(),
            created_seq: 0,
            event_id: 0,
            scope: MemoryScope::World {
                world_id: "jianghu".into(),
            },
            source: MemorySource::Experienced,
            provenance: Provenance::Seeded,
            memory_type: MemoryType::WorldEvent,
            layer: MemoryLayer::A,
            content: "정확한 사실 본문".into(),
            topic: Some("t".into()),
            emotional_context: None,
            timestamp_ms: 0,
            last_recalled_at: None,
            recall_count: 0,
            origin_chain: vec![],
            confidence: 1.0,
            acquired_by: None,
            superseded_by: None,
            consolidated_into: None,
            npc_id: "jianghu".into(),
        };
        *mem.canonical.lock().unwrap() = Some(canon);

        let handler = RumorDistributionHandler::new(mem.clone(), rum);
        let mut harness = HandlerTestHarness::new();
        harness
            .dispatch(&handler, spread_event(1, "r", 0, &["a"]))
            .unwrap();

        let e = mem.inner.lock().unwrap()[0].clone();
        assert_eq!(e.content, "정확한 사실 본문");
    }

    #[test]
    fn content_resolution_falls_back_to_seed_when_no_canonical() {
        let mem = Arc::new(SpyMemoryStore::default());
        let rum = Arc::new(SpyRumorStore::default());
        rum.save(&Rumor::orphan(
            "r-orphan",
            "고아 소문 본문",
            RumorOrigin::Authored { by: None },
            ReachPolicy::default(),
            0,
        ))
        .unwrap();
        // 고아 Rumor → topic 없음 → canonical 조회 생략, seed_content 사용
        let handler = RumorDistributionHandler::new(mem.clone(), rum);
        let mut harness = HandlerTestHarness::new();
        harness
            .dispatch(&handler, spread_event(1, "r-orphan", 0, &["a"]))
            .unwrap();

        let e = mem.inner.lock().unwrap()[0].clone();
        assert_eq!(e.content, "고아 소문 본문");
    }

    #[test]
    fn content_resolution_uses_distortion_when_content_version_set() {
        let mem = Arc::new(SpyMemoryStore::default());
        let rum = Arc::new(SpyRumorStore::default());
        let mut r = Rumor::with_forecast_content(
            "r",
            "t",
            "원본",
            RumorOrigin::Seeded,
            ReachPolicy::default(),
            0,
        );
        r.add_distortion(crate::domain::rumor::RumorDistortion {
            id: "d1".into(),
            parent: None,
            content: "변형된 버전".into(),
            created_at: 0,
        })
        .unwrap();
        rum.save(&r).unwrap();

        let handler = RumorDistributionHandler::new(mem.clone(), rum);
        let mut harness = HandlerTestHarness::new();
        // content_version = "d1"인 spread 이벤트 수동 생성
        let ev = DomainEvent::new(
            1,
            "r".into(),
            1,
            EventPayload::RumorSpread {
                rumor_id: "r".into(),
                hop_index: 0,
                recipients: vec!["a".into()],
                content_version: Some("d1".into()),
            },
        );
        harness.dispatch(&handler, ev).unwrap();

        let e = mem.inner.lock().unwrap()[0].clone();
        assert_eq!(e.content, "변형된 버전");
    }

    #[test]
    fn ignores_unrelated_event_kind() {
        let mem = Arc::new(SpyMemoryStore::default());
        let rum = Arc::new(SpyRumorStore::default());
        let handler = RumorDistributionHandler::new(mem.clone(), rum);
        let mut harness = HandlerTestHarness::new();
        let ev = DomainEvent::new(
            0,
            "x".into(),
            0,
            EventPayload::EmotionCleared { npc_id: "x".into() },
        );
        harness.dispatch(&handler, ev).unwrap();
        assert_eq!(mem.count(), 0);
    }
}
