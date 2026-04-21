//! RumorAgent — 소문 애그리거트 관리 (Step C3, Memory 컨텍스트, Transactional)
//!
//! 설계 문서: `docs/memory/03-implementation-design.md` §6.2
//!
//! **책임**:
//! - `SeedRumorRequested` → 새 `Rumor` 생성 + `RumorStore.save` + `RumorSeeded` follow-up
//! - `SpreadRumorRequested` → 기존 `Rumor` 로드 → 새 홉 추가 → `RumorStore.save` →
//!   `RumorSpread` follow-up (이후 Inline `RumorDistributionHandler`가 수신자 MemoryEntry를
//!   생성)
//!
//! **Rumor id 생성**: `rumor-{event.id:012}` — 결정적(Event Sourcing replay 안전).
//!
//! **불변식**: Rumor 자체의 I-RU-1~6은 `Rumor::add_hop`/`add_distortion`/`transition_to`가
//! 방어한다. 본 에이전트는 저장소 오류를 `HandlerError::Infrastructure`로 전파한다.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::application::command::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::event::{DomainEvent, EventKind, EventPayload};
use crate::domain::rumor::{ReachPolicy, Rumor, RumorHop};
use crate::ports::RumorStore;

pub struct RumorAgent {
    store: Arc<dyn RumorStore>,
    /// Rumor id 생성용 단조 카운터. `EventStore::next_id()`와 독립 관리되어
    /// **event log에 id gap을 유발하지 않는다** (Step C3 사후 리뷰 M1).
    /// 프로세스 수명 동안만 유일 — replay 시 재생성 필요성은 설계 §15 결정 유보.
    counter: Arc<AtomicU64>,
}

impl RumorAgent {
    pub fn new(store: Arc<dyn RumorStore>) -> Self {
        Self {
            store,
            counter: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Rumor id 포맷: `rumor-{counter:012}`. RumorAgent 인스턴스별 카운터 기반.
    fn next_rumor_id(&self) -> String {
        let n = self.counter.fetch_add(1, Ordering::SeqCst);
        format!("rumor-{n:012}")
    }

    fn handle_seed(
        &self,
        event: &DomainEvent,
        topic: &Option<String>,
        seed_content: &Option<String>,
        reach: &ReachPolicy,
        origin: &crate::domain::rumor::RumorOrigin,
    ) -> Result<HandlerResult, HandlerError> {
        let rumor_id = self.next_rumor_id();

        // Rumor 생성 — topic/seed_content 조합에 따른 생성자 분기 (§2.6 Canonical 해소표).
        let rumor = match (topic, seed_content) {
            (Some(t), Some(sc)) => Rumor::with_forecast_content(
                &rumor_id,
                t,
                sc,
                origin.clone(),
                reach.clone(),
                event.timestamp_ms,
            ),
            (Some(t), None) => {
                Rumor::new(&rumor_id, t, origin.clone(), reach.clone(), event.timestamp_ms)
            }
            (None, Some(sc)) => Rumor::orphan(
                &rumor_id,
                sc,
                origin.clone(),
                reach.clone(),
                event.timestamp_ms,
            ),
            (None, None) => {
                return Err(HandlerError::InvalidInput(
                    "SeedRumor: topic 없으면 seed_content 필수 (§2.6 orphan invariant)".into(),
                ));
            }
        };

        self.store.save(&rumor).map_err(|e| {
            tracing::error!(error = %e, "RumorAgent: rumor_store.save failed");
            HandlerError::Infrastructure("rumor_store.save")
        })?;

        let follow_up = DomainEvent::new(
            0,
            rumor_id.clone(),
            0,
            EventPayload::RumorSeeded {
                rumor_id,
                topic: topic.clone(),
                origin: origin.clone(),
                seed_content: seed_content.clone(),
                reach_policy: reach.clone(),
            },
        );
        Ok(HandlerResult {
            follow_up_events: vec![follow_up],
        })
    }

    fn handle_spread(
        &self,
        event: &DomainEvent,
        rumor_id: &str,
        extra_recipients: &[String],
    ) -> Result<HandlerResult, HandlerError> {
        let mut rumor = self
            .store
            .load(rumor_id)
            .map_err(|e| {
                tracing::error!(rumor_id, error = %e, "RumorAgent: rumor_store.load failed");
                HandlerError::Infrastructure("rumor_store.load")
            })?
            .ok_or_else(|| {
                HandlerError::InvalidInput(format!("SpreadRumor: rumor_id '{rumor_id}' 없음"))
            })?;

        // TODO(step-f): Fading/Faded status 전이가 도입되면 여기서 `Faded` rumor의 spread를
        // 거부해야 한다. Step C3 시점에는 status 전이 트리거(백그라운드 틱)가 없어 항상
        // Active라 가드 불필요. 리뷰 M4 참조.

        // 동일 수신자 중복 제거 — 같은 홉에서 같은 사람에게 두 번 저장되지 않도록.
        //
        // **빈 recipients 정책**: 수신자 0명이면 홉은 여전히 추가되고 `RumorSpread`
        // 이벤트도 발행된다 (Inline `RumorDistributionHandler`는 반복 대상이 없어 no-op).
        // "유령 홉" 형태이지만 `hop_index` 단조성을 유지하므로 허용 — caller가 원하지
        // 않으면 dispatch 전에 검증하라.
        let mut seen = std::collections::HashSet::new();
        let recipients: Vec<String> = extra_recipients
            .iter()
            .filter(|r| seen.insert(r.as_str()))
            .cloned()
            .collect();

        let hop_index = rumor.next_hop_index();
        let hop = RumorHop {
            hop_index,
            content_version: None, // Step C3 초기: 원본 content, distortion chain은 후속 작업.
            recipients: recipients.clone(),
            spread_at: event.timestamp_ms,
        };
        rumor
            .add_hop(hop)
            .map_err(|e| HandlerError::InvalidInput(format!("add_hop: {e}")))?;

        self.store.save(&rumor).map_err(|e| {
            tracing::error!(error = %e, "RumorAgent: rumor_store.save failed");
            HandlerError::Infrastructure("rumor_store.save")
        })?;

        let follow_up = DomainEvent::new(
            0,
            rumor_id.to_string(),
            0,
            EventPayload::RumorSpread {
                rumor_id: rumor_id.to_string(),
                hop_index,
                recipients,
                content_version: None,
            },
        );
        Ok(HandlerResult {
            follow_up_events: vec![follow_up],
        })
    }
}

impl EventHandler for RumorAgent {
    fn name(&self) -> &'static str {
        "RumorAgent"
    }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![
            EventKind::SeedRumorRequested,
            EventKind::SpreadRumorRequested,
        ])
    }

    fn mode(&self) -> DeliveryMode {
        DeliveryMode::Transactional {
            priority: priority::transactional::RUMOR_SPREAD,
            can_emit_follow_up: true,
        }
    }

    fn handle(
        &self,
        event: &DomainEvent,
        // 의도적 미사용: RumorAgent는 자체 AtomicU64 카운터로 rumor_id를 생성하고
        // RumorStore 외 repo/shared state를 참조하지 않는다. `prior_events`·
        // `aggregate_key`도 현재 분기에 쓸 일 없음.
        _ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        match &event.payload {
            EventPayload::SeedRumorRequested {
                pending_id: _,
                topic,
                seed_content,
                reach,
                origin,
            } => self.handle_seed(event, topic, seed_content, reach, origin),
            EventPayload::SpreadRumorRequested {
                rumor_id,
                extra_recipients,
            } => self.handle_spread(event, rumor_id, extra_recipients),
            _ => Ok(HandlerResult::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::command::handler_v2::test_support::HandlerTestHarness;
    use crate::domain::rumor::{RumorOrigin, RumorStatus};
    use std::sync::Mutex;

    /// 테스트용 인메모리 RumorStore.
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
        fn find_by_topic(&self, topic: &str) -> Result<Vec<Rumor>, crate::ports::MemoryError> {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .iter()
                .filter(|r| r.topic.as_deref() == Some(topic))
                .cloned()
                .collect())
        }
        fn find_active_in_reach(
            &self,
            _reach: &ReachPolicy,
        ) -> Result<Vec<Rumor>, crate::ports::MemoryError> {
            Ok(self.inner.lock().unwrap().clone())
        }
    }

    fn seed_req_event(
        event_id: u64,
        topic: Option<&str>,
        seed: Option<&str>,
        origin: RumorOrigin,
    ) -> DomainEvent {
        let mut ev = DomainEvent::new(
            event_id,
            topic.map(|t| t.to_string()).unwrap_or_else(|| "orphan".into()),
            1,
            EventPayload::SeedRumorRequested {
                pending_id: format!("{event_id:012}"),
                topic: topic.map(|t| t.into()),
                seed_content: seed.map(|s| s.into()),
                reach: ReachPolicy::default(),
                origin,
            },
        );
        ev.timestamp_ms = 1000;
        ev
    }

    fn spread_req_event(event_id: u64, rumor_id: &str, recipients: &[&str]) -> DomainEvent {
        let mut ev = DomainEvent::new(
            event_id,
            rumor_id.into(),
            1,
            EventPayload::SpreadRumorRequested {
                rumor_id: rumor_id.into(),
                extra_recipients: recipients.iter().map(|s| s.to_string()).collect(),
            },
        );
        ev.timestamp_ms = 2000;
        ev
    }

    #[test]
    fn seed_with_topic_creates_canonical_linked_rumor() {
        let store = Arc::new(SpyRumorStore::default());
        let agent = RumorAgent::new(store.clone());
        let mut harness = HandlerTestHarness::new();

        let result = harness
            .dispatch(
                &agent,
                seed_req_event(42, Some("moorim-leader-change"), None, RumorOrigin::Seeded),
            )
            .expect("seed must succeed");

        assert_eq!(result.follow_up_events.len(), 1);
        let EventPayload::RumorSeeded {
            rumor_id,
            topic,
            seed_content,
            ..
        } = &result.follow_up_events[0].payload
        else {
            panic!("expected RumorSeeded");
        };
        assert!(rumor_id.starts_with("rumor-"), "rumor_id format: {rumor_id}");
        assert_eq!(topic.as_deref(), Some("moorim-leader-change"));
        assert!(seed_content.is_none());

        // follow-up의 rumor_id로 저장소에서 조회 가능해야 한다 (round-trip)
        let saved = store.load(rumor_id).unwrap().unwrap();
        assert_eq!(saved.topic.as_deref(), Some("moorim-leader-change"));
        assert!(!saved.is_orphan());
        assert_eq!(saved.status(), RumorStatus::Active);
    }

    #[test]
    fn seed_orphan_requires_seed_content() {
        let store = Arc::new(SpyRumorStore::default());
        let agent = RumorAgent::new(store);
        let mut harness = HandlerTestHarness::new();

        let err = harness
            .dispatch(&agent, seed_req_event(1, None, None, RumorOrigin::Seeded))
            .expect_err("orphan without seed_content must fail");
        assert!(matches!(err, HandlerError::InvalidInput(_)));
    }

    fn rumor_id_of(result: &HandlerResult) -> String {
        let EventPayload::RumorSeeded { rumor_id, .. } = &result.follow_up_events[0].payload
        else {
            panic!("expected RumorSeeded");
        };
        rumor_id.clone()
    }

    #[test]
    fn seed_orphan_with_seed_content_succeeds() {
        let store = Arc::new(SpyRumorStore::default());
        let agent = RumorAgent::new(store.clone());
        let mut harness = HandlerTestHarness::new();

        let result = harness
            .dispatch(
                &agent,
                seed_req_event(7, None, Some("떠도는 얘기"), RumorOrigin::Authored { by: None }),
            )
            .expect("must succeed");

        let saved = store.load(&rumor_id_of(&result)).unwrap().unwrap();
        assert!(saved.is_orphan());
        assert_eq!(saved.seed_content.as_deref(), Some("떠도는 얘기"));
    }

    #[test]
    fn seed_forecast_has_both_topic_and_seed() {
        let store = Arc::new(SpyRumorStore::default());
        let agent = RumorAgent::new(store.clone());
        let mut harness = HandlerTestHarness::new();

        let result = harness
            .dispatch(
                &agent,
                seed_req_event(
                    10,
                    Some("master-change"),
                    Some("조만간 바뀐다더라"),
                    RumorOrigin::Authored {
                        by: Some("informant".into()),
                    },
                ),
            )
            .unwrap();

        let saved = store.load(&rumor_id_of(&result)).unwrap().unwrap();
        assert_eq!(saved.topic.as_deref(), Some("master-change"));
        assert_eq!(saved.seed_content.as_deref(), Some("조만간 바뀐다더라"));
    }

    #[test]
    fn successive_seeds_get_distinct_rumor_ids() {
        // 핵심 회귀 가드 — `event.id=0`을 쓰던 버그를 방지. 두 번 시드하면 서로 다른
        // rumor_id가 나와야 한다.
        let store = Arc::new(SpyRumorStore::default());
        let agent = RumorAgent::new(store.clone());
        let mut harness = HandlerTestHarness::new();

        let r1 = harness
            .dispatch(&agent, seed_req_event(0, Some("t1"), None, RumorOrigin::Seeded))
            .unwrap();
        let r2 = harness
            .dispatch(&agent, seed_req_event(0, Some("t2"), None, RumorOrigin::Seeded))
            .unwrap();

        let id1 = rumor_id_of(&r1);
        let id2 = rumor_id_of(&r2);
        assert_ne!(id1, id2, "두 시드는 서로 다른 rumor_id를 받아야 함");
        assert!(store.load(&id1).unwrap().is_some());
        assert!(store.load(&id2).unwrap().is_some());
    }

    #[test]
    fn spread_unknown_rumor_returns_invalid_input() {
        let store = Arc::new(SpyRumorStore::default());
        let agent = RumorAgent::new(store);
        let mut harness = HandlerTestHarness::new();

        let err = harness
            .dispatch(&agent, spread_req_event(1, "ghost", &["a"]))
            .expect_err("must fail");
        assert!(matches!(err, HandlerError::InvalidInput(_)));
    }

    #[test]
    fn spread_appends_hop_and_emits_rumor_spread() {
        let store = Arc::new(SpyRumorStore::default());
        // 먼저 seed
        let seeded = Rumor::new(
            "rumor-seed",
            "t",
            RumorOrigin::Seeded,
            ReachPolicy::default(),
            100,
        );
        store.save(&seeded).unwrap();

        let agent = RumorAgent::new(store.clone());
        let mut harness = HandlerTestHarness::new();

        let result = harness
            .dispatch(
                &agent,
                spread_req_event(99, "rumor-seed", &["npc-a", "npc-b"]),
            )
            .expect("must succeed");

        let EventPayload::RumorSpread {
            rumor_id,
            hop_index,
            recipients,
            ..
        } = &result.follow_up_events[0].payload
        else {
            panic!("expected RumorSpread");
        };
        assert_eq!(rumor_id, "rumor-seed");
        assert_eq!(*hop_index, 0);
        assert_eq!(recipients, &vec!["npc-a".to_string(), "npc-b".to_string()]);

        // 저장소에도 홉이 추가됨
        let reloaded = store.load("rumor-seed").unwrap().unwrap();
        assert_eq!(reloaded.hops().len(), 1);
        assert_eq!(reloaded.hops()[0].hop_index, 0);
    }

    #[test]
    fn spread_dedupes_recipients() {
        let store = Arc::new(SpyRumorStore::default());
        store
            .save(&Rumor::new(
                "r",
                "t",
                RumorOrigin::Seeded,
                ReachPolicy::default(),
                0,
            ))
            .unwrap();
        let agent = RumorAgent::new(store.clone());
        let mut harness = HandlerTestHarness::new();

        let result = harness
            .dispatch(
                &agent,
                spread_req_event(1, "r", &["a", "b", "a"]),
            )
            .unwrap();

        let EventPayload::RumorSpread { recipients, .. } = &result.follow_up_events[0].payload
        else {
            unreachable!()
        };
        assert_eq!(recipients, &vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn spread_twice_monotonic_hop_index() {
        let store = Arc::new(SpyRumorStore::default());
        store
            .save(&Rumor::new(
                "r",
                "t",
                RumorOrigin::Seeded,
                ReachPolicy::default(),
                0,
            ))
            .unwrap();
        let agent = RumorAgent::new(store.clone());
        let mut harness = HandlerTestHarness::new();

        let r1 = harness
            .dispatch(&agent, spread_req_event(1, "r", &["a"]))
            .unwrap();
        let r2 = harness
            .dispatch(&agent, spread_req_event(2, "r", &["b"]))
            .unwrap();

        let h1 = match &r1.follow_up_events[0].payload {
            EventPayload::RumorSpread { hop_index, .. } => *hop_index,
            _ => unreachable!(),
        };
        let h2 = match &r2.follow_up_events[0].payload {
            EventPayload::RumorSpread { hop_index, .. } => *hop_index,
            _ => unreachable!(),
        };
        assert_eq!(h1, 0);
        assert_eq!(h2, 1);
    }
}
