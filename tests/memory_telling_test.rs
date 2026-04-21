//! Step C2 — TellInformation 커맨드 경로 end-to-end 통합 테스트.
//!
//! 커버리지:
//! - 청자당 1 `InformationTold` 팬아웃 (B5)
//! - `origin_chain_in` 길이 → Heard/Rumor 분기 (§7.1)
//! - `stated_confidence × normalized_trust` 신뢰도 계산
//! - `ListenerRole::Overhearer`가 동일 처리 경로를 탐
//! - `with_memory()` 미부착 시 MemoryStore 저장은 건너뛰되 이벤트는 여전히 발행됨
//!
//! `chat` feature 불필요, `embed` feature 불필요 — 테스트 전용 InMemoryMemoryStore 사용.

mod common;

use common::in_memory_store::InMemoryMemoryStore;
use npc_mind::application::command::{Command, CommandDispatcher};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::EventStore;
use npc_mind::domain::event::{EventKind, EventPayload, ListenerRole};
use npc_mind::domain::memory::{MemoryScope, MemorySource};
use npc_mind::domain::personality::{NpcBuilder, Score};
use npc_mind::domain::relationship::Relationship;
use npc_mind::ports::{MemoryQuery, MemoryScopeFilter, MemoryStore};
use npc_mind::{InMemoryRepository, TellInformationRequest};
use std::sync::Arc;

fn make_npc(id: &str) -> npc_mind::domain::personality::Npc {
    NpcBuilder::new(id, id).build()
}

fn trust_rel(owner: &str, target: &str, trust: f32) -> Relationship {
    Relationship::new(
        owner,
        target,
        Score::new(0.0, "closeness").unwrap(),
        Score::new(trust, "trust").unwrap(),
        Score::new(0.0, "power").unwrap(),
    )
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
    let dispatcher = CommandDispatcher::new(repo, event_store.clone(), bus)
        .with_default_handlers()
        .with_memory(store.clone() as Arc<dyn MemoryStore>);

    (dispatcher, event_store)
}

fn collect_told(store: &InMemoryEventStore) -> Vec<EventPayload> {
    store
        .get_all_events()
        .into_iter()
        .filter(|e| e.kind() == EventKind::InformationTold)
        .map(|e| e.payload)
        .collect()
}

fn listener_entries(store: &dyn MemoryStore, listener: &str) -> Vec<npc_mind::MemoryEntry> {
    let results = store
        .search(MemoryQuery {
            scope_filter: Some(MemoryScopeFilter::Exact(MemoryScope::Personal {
                npc_id: listener.into(),
            })),
            limit: 100,
            ..Default::default()
        })
        .unwrap();
    results.into_iter().map(|r| r.entry).collect()
}

#[tokio::test]
async fn tell_information_emits_one_told_event_per_listener() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, event_store) = build_dispatcher(store.clone());

    let _ = dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into(), "wanderer".into()],
            overhearers: vec![],
            claim: "장문인이 바뀐다".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        }))
        .await
        .expect("dispatch must succeed");

    let tolds = collect_told(&event_store);
    assert_eq!(tolds.len(), 2, "B5: one InformationTold per listener");
    for p in tolds {
        let EventPayload::InformationTold { listener_role, .. } = p else {
            unreachable!()
        };
        assert_eq!(listener_role, ListenerRole::Direct);
    }
}

#[tokio::test]
async fn direct_speaker_creates_heard_memory_in_listener_scope() {
    // 화자가 직접 경험을 말함 (origin_chain_in = []) → 청자 chain = [speaker], len=1 → Heard
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, _) = build_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into()],
            overhearers: vec![],
            claim: "직접 목격한 사건".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        }))
        .await
        .unwrap();

    let entries = listener_entries(&*store, "pupil");
    assert_eq!(entries.len(), 1);
    let e = &entries[0];
    assert_eq!(e.source, MemorySource::Heard);
    assert_eq!(e.content, "직접 목격한 사건");
    assert_eq!(e.origin_chain, vec!["sage".to_string()]);
    assert!(matches!(
        e.scope,
        MemoryScope::Personal { ref npc_id } if npc_id == "pupil"
    ));
}

#[tokio::test]
async fn relayed_information_classified_as_rumor_when_chain_length_two_or_more() {
    // 'relay'가 이미 들은 정보를 'final_listener'에게 전달.
    // 청자 chain = [relay, original_witness], len=2 → Rumor.
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, _) = build_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "relay".into(),
            listeners: vec!["final_listener".into()],
            overhearers: vec![],
            claim: "건너건너 들은 이야기".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec!["original_witness".into()],
            topic: None,
        }))
        .await
        .unwrap();

    let entries = listener_entries(&*store, "final_listener");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, MemorySource::Rumor);
    assert_eq!(
        entries[0].origin_chain,
        vec!["relay".to_string(), "original_witness".to_string()]
    );
}

#[tokio::test]
async fn confidence_multiplies_stated_by_normalized_trust() {
    // pupil의 sage에 대한 trust = 0.6 → normalized = 0.8
    // stated = 0.5 → confidence = 0.4
    let store = Arc::new(InMemoryMemoryStore::new());

    let mut repo = InMemoryRepository::new();
    repo.add_npc(make_npc("sage"));
    repo.add_npc(make_npc("pupil"));
    repo.add_relationship(trust_rel("pupil", "sage", 0.6));

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(repo, event_store.clone(), bus)
        .with_default_handlers()
        .with_memory(store.clone() as Arc<dyn MemoryStore>);

    dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into()],
            overhearers: vec![],
            claim: "반만 믿는 이야기".into(),
            stated_confidence: 0.5,
            origin_chain_in: vec![],
            topic: None,
        }))
        .await
        .unwrap();

    let entries = listener_entries(&*store, "pupil");
    assert_eq!(entries.len(), 1);
    assert!(
        (entries[0].confidence - 0.4).abs() < 1e-6,
        "expected ~0.4 (0.5 × 0.8), got {}",
        entries[0].confidence
    );
}

#[tokio::test]
async fn overhearers_receive_distinct_told_events_with_overhearer_role() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, event_store) = build_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into()],
            overhearers: vec!["wanderer".into()],
            claim: "엿들을 수도 있는 이야기".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        }))
        .await
        .unwrap();

    let tolds = collect_told(&event_store);
    assert_eq!(tolds.len(), 2);
    let roles: Vec<ListenerRole> = tolds
        .iter()
        .map(|p| match p {
            EventPayload::InformationTold { listener_role, .. } => *listener_role,
            _ => unreachable!(),
        })
        .collect();
    assert!(roles.contains(&ListenerRole::Direct));
    assert!(roles.contains(&ListenerRole::Overhearer));

    // 두 청자 모두 각자의 scope에 MemoryEntry가 만들어져야 한다.
    assert_eq!(listener_entries(&*store, "pupil").len(), 1);
    assert_eq!(listener_entries(&*store, "wanderer").len(), 1);
}

#[tokio::test]
async fn without_memory_builder_events_still_fire_but_no_memory_stored() {
    // with_memory() 미부착 — 이벤트 스트림만 흐르고 MemoryStore 저장은 없어야 함.
    let mut repo = InMemoryRepository::new();
    repo.add_npc(make_npc("sage"));
    repo.add_npc(make_npc("pupil"));
    repo.add_relationship(Relationship::neutral("pupil", "sage"));

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(repo, event_store.clone(), bus)
        .with_default_handlers();
    // 주의: with_memory 호출 없음.

    dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into()],
            overhearers: vec![],
            claim: "저장 안 되는 이야기".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        }))
        .await
        .unwrap();

    // InformationTold는 발행된다
    let tolds = collect_told(&event_store);
    assert_eq!(tolds.len(), 1);

    // 그러나 MemoryStore에 저장되지 않는다 (외부 소비자가 없음)
    let external_store = InMemoryMemoryStore::new();
    assert_eq!(external_store.count(), 0);
}

#[tokio::test]
async fn empty_listeners_no_events_no_memory() {
    // 청자 0 → follow-up 0, 커맨드 자체는 성공 (초기 *Requested는 여전히 commit).
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, event_store) = build_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec![],
            overhearers: vec![],
            claim: "아무도 안 듣는 이야기".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        }))
        .await
        .unwrap();

    let tolds = collect_told(&event_store);
    assert!(tolds.is_empty());
    assert_eq!(store.count(), 0);
}
