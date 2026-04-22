//! Step C3 — SeedRumor / SpreadRumor 커맨드 경로 end-to-end 통합 테스트.
//!
//! 커버리지:
//! - `Command::SeedRumor` → `RumorSeeded` → RumorStore 저장
//! - `Command::SpreadRumor` → `RumorSpread` → 수신자별 `MemoryEntry(Rumor)` 저장
//! - 홉 증가에 따른 confidence 기하 감쇠
//! - 같은 커맨드 내 중복 수신자 dedup
//! - 존재하지 않는 rumor_id 확산 시 실패
//! - MAX_EVENTS_PER_COMMAND 경계 (20 이내는 통과)
//!
//! `chat`·`embed` feature 불필요.

mod common;

use common::in_memory_rumor::InMemoryRumorStore;
use common::in_memory_store::InMemoryMemoryStore;
use npc_mind::application::command::{Command, CommandDispatcher};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::event::EventKind;
use npc_mind::domain::memory::{MemoryScope, MemorySource};
use npc_mind::domain::personality::NpcBuilder;
use npc_mind::domain::tuning::{RUMOR_HOP_CONFIDENCE_DECAY, RUMOR_MIN_CONFIDENCE};
use npc_mind::ports::{MemoryQuery, MemoryScopeFilter, MemoryStore, RumorStore};
use npc_mind::{
    EventStore, InMemoryRepository, RumorOriginInput, RumorReachInput, SeedRumorRequest,
    SpreadRumorRequest,
};
use std::sync::Arc;

fn build_dispatcher() -> (
    CommandDispatcher<InMemoryRepository>,
    Arc<InMemoryEventStore>,
    Arc<InMemoryMemoryStore>,
    Arc<InMemoryRumorStore>,
) {
    let mut repo = InMemoryRepository::new();
    // 수신자 NPC들을 넉넉히 등록 (Relationship은 Rumor 처리에 필요 없음)
    for id in ["a", "b", "c", "d", "sage", "pupil"] {
        repo.add_npc(NpcBuilder::new(id, id).build());
    }
    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let memory_store = Arc::new(InMemoryMemoryStore::new());
    let rumor_store = Arc::new(InMemoryRumorStore::new());
    let dispatcher = CommandDispatcher::new(repo, event_store.clone(), bus)
        .with_default_handlers()
        .with_memory(memory_store.clone() as Arc<dyn MemoryStore>)
        .with_rumor(
            memory_store.clone() as Arc<dyn MemoryStore>,
            rumor_store.clone() as Arc<dyn RumorStore>,
        );
    (dispatcher, event_store, memory_store, rumor_store)
}

fn recipient_entries(store: &dyn MemoryStore, npc: &str) -> Vec<npc_mind::MemoryEntry> {
    store
        .search(MemoryQuery {
            scope_filter: Some(MemoryScopeFilter::Exact(MemoryScope::Personal {
                npc_id: npc.into(),
            })),
            limit: 100,
            ..Default::default()
        })
        .unwrap()
        .into_iter()
        .map(|r| r.entry)
        .collect()
}

#[tokio::test]
async fn seed_rumor_persists_aggregate_and_emits_rumor_seeded() {
    let (dispatcher, event_store, _, rumor_store) = build_dispatcher();

    dispatcher
        .dispatch_v2(Command::SeedRumor(SeedRumorRequest {
            topic: Some("moorim-leader-change".into()),
            seed_content: None,
            reach: RumorReachInput::default(),
            origin: RumorOriginInput::Seeded,
        }))
        .await
        .expect("seed must succeed");

    // Rumor aggregate 저장됨
    let all: Vec<npc_mind::Rumor> = rumor_store
        .find_by_topic("moorim-leader-change")
        .unwrap();
    assert_eq!(all.len(), 1);
    assert!(!all[0].is_orphan());

    // RumorSeeded 이벤트 발행됨 (aggregate_id = rumor_id로 저장)
    let rumor_id = &all[0].id;
    let rumor_events = event_store.get_events(rumor_id);
    assert!(rumor_events
        .iter()
        .any(|e| e.kind() == EventKind::RumorSeeded));
}

#[tokio::test]
async fn seed_orphan_without_content_fails_with_invalid_situation() {
    let (dispatcher, _, _, _) = build_dispatcher();
    let err = dispatcher
        .dispatch_v2(Command::SeedRumor(SeedRumorRequest {
            topic: None,
            seed_content: None,
            reach: RumorReachInput::default(),
            origin: RumorOriginInput::Seeded,
        }))
        .await
        .expect_err("orphan without seed_content must fail");
    let msg = format!("{err:?}");
    assert!(msg.contains("InvalidSituation") || msg.contains("seed_content"), "{msg}");
}

#[tokio::test]
async fn spread_emits_rumor_spread_and_creates_recipient_memories() {
    let (dispatcher, event_store, memory_store, rumor_store) = build_dispatcher();

    // 먼저 seed
    dispatcher
        .dispatch_v2(Command::SeedRumor(SeedRumorRequest {
            topic: Some("topic-a".into()),
            seed_content: Some("초기 본문".into()),
            reach: RumorReachInput::default(),
            origin: RumorOriginInput::Seeded,
        }))
        .await
        .unwrap();
    let rumor_id = rumor_store.find_by_topic("topic-a").unwrap().pop().unwrap().id;

    // 그 다음 spread
    dispatcher
        .dispatch_v2(Command::SpreadRumor(SpreadRumorRequest {
            rumor_id: rumor_id.clone(),
            recipients: vec!["a".into(), "b".into(), "c".into()],
            content_version: None,
        }))
        .await
        .expect("spread must succeed");

    // 이벤트 검증 — rumor_id aggregate에 RumorSpread 있음
    let rumor_events = event_store.get_events(&rumor_id);
    let spreads: Vec<_> = rumor_events
        .iter()
        .filter(|e| e.kind() == EventKind::RumorSpread)
        .collect();
    assert_eq!(spreads.len(), 1);

    // 각 수신자에게 MemoryEntry(Rumor) 저장됨
    for npc in ["a", "b", "c"] {
        let entries = recipient_entries(&*memory_store, npc);
        assert_eq!(entries.len(), 1, "{npc} entry count");
        assert_eq!(entries[0].source, MemorySource::Rumor);
        assert_eq!(entries[0].content, "초기 본문");
        assert_eq!(entries[0].topic.as_deref(), Some("topic-a"));
    }

    // Rumor 애그리거트에 홉 기록됨
    let reloaded = rumor_store.load(&rumor_id).unwrap().unwrap();
    assert_eq!(reloaded.hops().len(), 1);
    assert_eq!(reloaded.hops()[0].recipients.len(), 3);
}

#[tokio::test]
async fn successive_spreads_increment_hop_and_decay_confidence() {
    let (dispatcher, _, memory_store, rumor_store) = build_dispatcher();

    dispatcher
        .dispatch_v2(Command::SeedRumor(SeedRumorRequest {
            topic: Some("topic-b".into()),
            seed_content: Some("본문".into()),
            reach: RumorReachInput::default(),
            origin: RumorOriginInput::Seeded,
        }))
        .await
        .unwrap();
    let rumor_id = rumor_store.find_by_topic("topic-b").unwrap().pop().unwrap().id;

    // hop 0
    dispatcher
        .dispatch_v2(Command::SpreadRumor(SpreadRumorRequest {
            rumor_id: rumor_id.clone(),
            recipients: vec!["a".into()],
            content_version: None,
        }))
        .await
        .unwrap();
    // hop 1
    dispatcher
        .dispatch_v2(Command::SpreadRumor(SpreadRumorRequest {
            rumor_id: rumor_id.clone(),
            recipients: vec!["b".into()],
            content_version: None,
        }))
        .await
        .unwrap();
    // hop 2
    dispatcher
        .dispatch_v2(Command::SpreadRumor(SpreadRumorRequest {
            rumor_id: rumor_id.clone(),
            recipients: vec!["c".into()],
            content_version: None,
        }))
        .await
        .unwrap();

    let a = recipient_entries(&*memory_store, "a").pop().unwrap();
    let b = recipient_entries(&*memory_store, "b").pop().unwrap();
    let c = recipient_entries(&*memory_store, "c").pop().unwrap();

    assert!((a.confidence - 1.0).abs() < 1e-6); // decay^0
    assert!(
        (b.confidence - RUMOR_HOP_CONFIDENCE_DECAY).abs() < 1e-6,
        "expected decay^1 = {}, got {}",
        RUMOR_HOP_CONFIDENCE_DECAY,
        b.confidence
    );
    let expected_c = (RUMOR_HOP_CONFIDENCE_DECAY * RUMOR_HOP_CONFIDENCE_DECAY).max(RUMOR_MIN_CONFIDENCE);
    assert!((c.confidence - expected_c).abs() < 1e-5);
    assert!(c.confidence < b.confidence && b.confidence < a.confidence);
}

#[tokio::test]
async fn spread_unknown_rumor_fails() {
    use npc_mind::application::command::dispatcher::DispatchV2Error;
    use npc_mind::application::command::handler_v2::HandlerError;

    let (dispatcher, _, _, _) = build_dispatcher();
    let err = dispatcher
        .dispatch_v2(Command::SpreadRumor(SpreadRumorRequest {
            rumor_id: "ghost".into(),
            recipients: vec!["a".into()],
            content_version: None,
        }))
        .await
        .expect_err("unknown rumor_id must fail");

    // Variant match로 승격 (C3 리뷰 n2) — 문자열 매칭 리팩터 내성 강화.
    match err {
        DispatchV2Error::HandlerFailed {
            handler,
            source: HandlerError::InvalidInput(msg),
        } => {
            assert_eq!(handler, "RumorAgent");
            assert!(
                msg.contains("ghost"),
                "error should mention missing rumor_id: {msg}"
            );
        }
        other => panic!("expected HandlerFailed/InvalidInput, got {other:?}"),
    }
}

#[tokio::test]
async fn spread_dedupes_repeated_recipients() {
    let (dispatcher, _, memory_store, rumor_store) = build_dispatcher();
    dispatcher
        .dispatch_v2(Command::SeedRumor(SeedRumorRequest {
            topic: Some("t".into()),
            seed_content: Some("x".into()),
            reach: RumorReachInput::default(),
            origin: RumorOriginInput::Seeded,
        }))
        .await
        .unwrap();
    let rumor_id = rumor_store.find_by_topic("t").unwrap().pop().unwrap().id;

    dispatcher
        .dispatch_v2(Command::SpreadRumor(SpreadRumorRequest {
            rumor_id,
            recipients: vec!["a".into(), "b".into(), "a".into()],
            content_version: None,
        }))
        .await
        .unwrap();

    // 같은 커맨드 내 중복은 제거되어 "a" 엔트리는 1개만 (이벤트 1개 → 수신자 2명).
    assert_eq!(recipient_entries(&*memory_store, "a").len(), 1);
    assert_eq!(recipient_entries(&*memory_store, "b").len(), 1);
}

#[tokio::test]
async fn successive_seeds_produce_distinct_rumor_ids_in_same_dispatcher() {
    // RumorAgent가 event.id(=0)를 쓰던 버그의 회귀 가드 — 같은 dispatcher로 두 번 시드하면
    // 두 Rumor가 서로 다른 id로 저장되어야 한다.
    let (dispatcher, _, _, rumor_store) = build_dispatcher();

    dispatcher
        .dispatch_v2(Command::SeedRumor(SeedRumorRequest {
            topic: Some("t1".into()),
            seed_content: None,
            reach: RumorReachInput::default(),
            origin: RumorOriginInput::Seeded,
        }))
        .await
        .unwrap();
    dispatcher
        .dispatch_v2(Command::SeedRumor(SeedRumorRequest {
            topic: Some("t2".into()),
            seed_content: None,
            reach: RumorReachInput::default(),
            origin: RumorOriginInput::Seeded,
        }))
        .await
        .unwrap();

    let r1 = rumor_store.find_by_topic("t1").unwrap().pop().unwrap();
    let r2 = rumor_store.find_by_topic("t2").unwrap().pop().unwrap();
    assert_ne!(r1.id, r2.id, "두 Rumor는 서로 다른 id를 가져야 함");
}

#[tokio::test]
async fn seed_rumor_is_stored_under_rumor_id_aggregate() {
    // C1 회귀 가드 확장 — RumorSeeded.aggregate_id = rumor_id여서
    // EventStore.get_events(rumor_id) 로 조회 가능해야 한다 (§3.3).
    let (dispatcher, event_store, _, rumor_store) = build_dispatcher();

    dispatcher
        .dispatch_v2(Command::SeedRumor(SeedRumorRequest {
            topic: Some("topic-c".into()),
            seed_content: None,
            reach: RumorReachInput::default(),
            origin: RumorOriginInput::Seeded,
        }))
        .await
        .unwrap();

    let rumor = rumor_store.find_by_topic("topic-c").unwrap().pop().unwrap();
    let events = event_store.get_events(&rumor.id);
    assert!(events.iter().any(|e| e.kind() == EventKind::RumorSeeded));
}

#[tokio::test]
async fn deep_hop_confidence_floors_at_min() {
    // C3 리뷰 n3 — hop_index가 깊어지면 `decay^n`이 RUMOR_MIN_CONFIDENCE 하한에 걸린다.
    // decay=0.8, min=0.1 → 0.8^N < 0.1이 되는 N은 11 이상 (0.8^10 ≈ 0.107, 0.8^11 ≈ 0.086).
    let (dispatcher, _, memory_store, rumor_store) = build_dispatcher();

    dispatcher
        .dispatch_v2(Command::SeedRumor(SeedRumorRequest {
            topic: Some("deep-topic".into()),
            seed_content: Some("본문".into()),
            reach: RumorReachInput::default(),
            origin: RumorOriginInput::Seeded,
        }))
        .await
        .unwrap();
    let rumor_id = rumor_store
        .find_by_topic("deep-topic")
        .unwrap()
        .pop()
        .unwrap()
        .id;

    // hop 0..=20까지 연속 spread. 각 홉에 유일 수신자 하나씩.
    for hop in 0..=20_u32 {
        let npc_id = format!("listener-{hop}");
        dispatcher
            .dispatch_v2(Command::SpreadRumor(SpreadRumorRequest {
                rumor_id: rumor_id.clone(),
                recipients: vec![npc_id],
                content_version: None,
            }))
            .await
            .unwrap();
    }

    // hop 0: 1.0, hop 20: clamp(0.8^20 ≈ 0.0115, 0.1) = 0.1
    let hop0 = recipient_entries(&*memory_store, "listener-0").pop().unwrap();
    let hop20 = recipient_entries(&*memory_store, "listener-20").pop().unwrap();
    assert!((hop0.confidence - 1.0).abs() < 1e-6);
    assert!(
        (hop20.confidence - RUMOR_MIN_CONFIDENCE).abs() < 1e-6,
        "hop 20 should floor at RUMOR_MIN_CONFIDENCE ({}), got {}",
        RUMOR_MIN_CONFIDENCE,
        hop20.confidence
    );
}
