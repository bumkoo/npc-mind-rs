//! Step C2 — TellInformation 커맨드 경로 end-to-end 통합 테스트.
//!
//! 커버리지:
//! - `Command::TellInformation` → `InformationTold` 발행 (팬아웃)
//! - `InformationTold` → `TellingIngestionHandler` → `MemoryStore.index`
//! - `origin_chain` 확장 및 `MemorySource` 분류 (Heard vs Rumor)
//! - `stated_confidence`와 `trust` 결합에 의한 `confidence` 계산
//! - 다중 청자(listeners + overhearers) 동시 처리 및 dedup
//!
//! `chat`·`embed` feature 불필요.

mod common;

use common::in_memory_store::InMemoryMemoryStore;
use npc_mind::application::command::{Command, CommandDispatcher};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::event::{EventPayload, ListenerRole};
use npc_mind::domain::memory::MemorySource;
use npc_mind::domain::personality::NpcBuilder;
use npc_mind::domain::relationship::Relationship;
use npc_mind::ports::{MemoryStore, RumorStore};
use npc_mind::{
    EventStore, InMemoryRepository, SeedRumorRequest, TellInformationRequest,
};
use std::sync::{Arc, Mutex};

fn make_npc(id: &str) -> npc_mind::domain::personality::Npc {
    NpcBuilder::new(id, id).build()
}

fn build_dispatcher(
    store: Arc<InMemoryMemoryStore>,
) -> (
    CommandDispatcher<InMemoryRepository>,
    Arc<InMemoryEventStore>,
) {
    let mut repo = InMemoryRepository::new();
    repo.add_npc(make_npc("sage"));
    repo.add_npc(make_npc("pupil"));
    repo.add_npc(make_npc("wanderer"));
    repo.add_npc(make_npc("relay"));
    repo.add_npc(make_npc("final_listener"));
    // 기본 중립 관계 (청자 → 화자)
    repo.add_relationship(Relationship::neutral("pupil", "sage"));
    repo.add_relationship(Relationship::neutral("wanderer", "sage"));
    repo.add_relationship(Relationship::neutral("final_listener", "relay"));

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let repo_arc = Arc::new(Mutex::new(repo));
    let dispatcher = CommandDispatcher::new(repo_arc, event_store.clone(), bus)
        .with_default_handlers()
        .with_memory(store.clone() as Arc<dyn MemoryStore>);

    (dispatcher, event_store)
}

fn collect_told(store: &InMemoryEventStore) -> Vec<EventPayload> {
    store
        .get_all_events()
        .into_iter()
        .filter_map(|e| match e.payload {
            EventPayload::InformationTold { .. } => Some(e.payload),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn tell_information_requested_emits_one_information_told_per_listener() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, event_store) = build_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::TellInformation(Box::new(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into()],
            overhearers: vec!["wanderer".into()],
            claim: "장문인이 바뀐다".into(),
            stated_confidence: 0.8,
            origin_chain_in: vec![],
            topic: None,
        })))
        .await
        .unwrap();

    let told = collect_told(&*event_store);
    assert_eq!(told.len(), 2, "listeners + overhearers = 2 events");

    // 각 청자에게 메모리 저장됨
    assert_eq!(store.count(), 2);
}

#[tokio::test]
async fn tell_information_deduplicates_listeners_and_overhearers() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, event_store) = build_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::TellInformation(Box::new(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into(), "pupil".into()], // 중복
            overhearers: vec!["pupil".into(), "wanderer".into()], // 중복
            claim: "x".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        })))
        .await
        .unwrap();

    let told = collect_told(&*event_store);
    assert_eq!(told.len(), 2, "pupil(Direct) + wanderer(Overhearer) = 2");
    assert_eq!(store.count(), 2);
}

#[tokio::test]
async fn telling_ingestion_adds_one_hop_for_direct_listener() {
    let store = Arc::new(InMemoryMemoryStore::new());
    
    let mut repo = InMemoryRepository::new();
    repo.add_npc(make_npc("sage"));
    repo.add_npc(make_npc("pupil"));
    repo.add_relationship(Relationship::neutral("pupil", "sage"));

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let repo_arc = Arc::new(Mutex::new(repo));
    let dispatcher = CommandDispatcher::new(repo_arc, event_store.clone(), bus)
        .with_default_handlers()
        .with_memory(store.clone() as Arc<dyn MemoryStore>);

    dispatcher
        .dispatch_v2(Command::TellInformation(Box::new(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into()],
            overhearers: vec![],
            claim: "claim".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![], // 원천 정보
            topic: None,
        })))
        .await
        .unwrap();

    let entries = store.get_recent("pupil", 1).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, MemorySource::Heard, "직접 들음");
    assert_eq!(entries[0].origin_chain, vec!["sage".to_string()]);
}

#[tokio::test]
async fn telling_ingestion_adds_two_hops_for_overhearer() {
    let store = Arc::new(InMemoryMemoryStore::new());
    
    let mut repo = InMemoryRepository::new();
    repo.add_npc(make_npc("relay"));
    repo.add_npc(make_npc("final_listener"));
    repo.add_relationship(Relationship::neutral("final_listener", "relay"));

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let repo_arc = Arc::new(Mutex::new(repo));
    let dispatcher = CommandDispatcher::new(repo_arc, event_store.clone(), bus)
        .with_default_handlers()
        .with_memory(store.clone() as Arc<dyn MemoryStore>);

    dispatcher
        .dispatch_v2(Command::TellInformation(Box::new(TellInformationRequest {
            speaker: "relay".into(),
            listeners: vec!["final_listener".into()],
            overhearers: vec![],
            claim: "claim".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec!["original-source".into()], // 전해 들은 정보
            topic: None,
        })))
        .await
        .unwrap();

    let entries = store.get_recent("final_listener", 1).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, MemorySource::Rumor, "체인 2개 이상 → 소문");
    assert_eq!(
        entries[0].origin_chain,
        vec!["relay".to_string(), "original-source".to_string()]
    );
}

#[tokio::test]
async fn confidence_is_scaled_by_trust() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let mut repo = InMemoryRepository::new();
    repo.add_npc(make_npc("sage"));
    repo.add_npc(make_npc("pupil"));

    // 신뢰도 0.6 설정 → normalized trust = (0.6 + 1) / 2 = 0.8
    // Stage 1 4축 swap: trust ±100, 그 외 NEUTRAL.
    use npc_mind::domain::relationship::{AxisScore, WarinessScore};
    let rel = Relationship::new(
        "pupil",
        "sage",
        AxisScore::new(60.0),   // trust
        AxisScore::NEUTRAL,     // affinity (구 closeness)
        AxisScore::NEUTRAL,     // respect
        WarinessScore::NEUTRAL,
    );
    repo.add_relationship(rel);

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let repo_arc = Arc::new(Mutex::new(repo));
    let dispatcher = CommandDispatcher::new(repo_arc, event_store.clone(), bus)
        .with_default_handlers()
        .with_memory(store.clone() as Arc<dyn MemoryStore>);

    dispatcher
        .dispatch_v2(Command::TellInformation(Box::new(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into()],
            overhearers: vec![],
            claim: "claim".into(),
            stated_confidence: 0.5, // 화자가 0.5 확신으로 말함
            origin_chain_in: vec![],
            topic: None,
        })))
        .await
        .unwrap();

    let entries = store.get_recent("pupil", 1).unwrap();
    // 기대: 0.5(stated) * 0.8(trust) = 0.4
    assert!((entries[0].confidence - 0.4).abs() < 1e-6);
}

#[tokio::test]
async fn budget_exhaustion_test_21_listeners_succeeds_but_22_fails() {
    // Phase 1 Mind Architecture (Stage 0 Findings F3 (아) / spec §11.7):
    // MAX_EVENTS_PER_COMMAND 21 → 22로 인상. TellInformation은 청자당 1 이벤트 +
    // 초기 TellInformationRequested 1개. 22명까지 한 커맨드 안에서 처리 가능,
    // 23명부터 EventBudgetExceeded.
    let store = Arc::new(InMemoryMemoryStore::new());

    let mut repo = InMemoryRepository::new();
    // 23명의 NPC 등록
    for i in 0..23 {
        repo.add_npc(make_npc(&format!("pupil-{i}")));
    }
    let repo_arc = Arc::new(Mutex::new(repo));
    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(repo_arc, event_store.clone(), bus)
        .with_default_handlers()
        .with_memory(store.clone() as Arc<dyn MemoryStore>);

    let mut listeners_21 = Vec::new();
    for i in 0..21 {
        listeners_21.push(format!("pupil-{i}"));
    }

    let ok = dispatcher
        .dispatch_v2(Command::TellInformation(Box::new(TellInformationRequest {
            speaker: "sage".into(),
            listeners: listeners_21,
            overhearers: vec![],
            claim: "claim".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        })))
        .await;

    assert!(ok.is_ok(), "21명까지는 한 커맨드 예산(22) 내");

    let mut listeners_22 = Vec::new();
    for i in 0..22 {
        listeners_22.push(format!("pupil-{i}"));
    }

    let err = dispatcher
        .dispatch_v2(Command::TellInformation(Box::new(TellInformationRequest {
            speaker: "sage".into(),
            listeners: listeners_22,
            overhearers: vec![],
            claim: "claim".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        })))
        .await;

    assert!(err.is_err(), "22명은 이벤트 예산(22개) 초과로 실패해야 함 (TellInformationRequested 1 + 22 fanout = 23)");
}
