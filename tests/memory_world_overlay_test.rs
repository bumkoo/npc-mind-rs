//! Step D — World 오버레이 통합 테스트.
//!
//! `Command::ApplyWorldEvent` → `ApplyWorldEventRequested` → `WorldEventOccurred` →
//! Canonical `MemoryEntry(World, Seeded)` 생성 + 기존 Topic Canonical supersede.
//!
//! 커버리지:
//! - 신규 World Canonical 엔트리 생성
//! - 기존 같은 topic 엔트리 supersede
//! - `get_canonical_by_topic`이 최신 Canonical을 반환
//! - topic 없으면 supersede 없이 새 엔트리만 추가
//! - `Provenance::is_canonical` 검증

mod common;

use common::in_memory_store::InMemoryMemoryStore;
use npc_mind::application::command::{Command, CommandDispatcher};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::event::{EventKind, EventPayload};
use npc_mind::domain::memory::{
    MemoryEntry, MemoryLayer, MemoryScope, MemorySource, MemoryType, Provenance,
};
use npc_mind::ports::MemoryStore;
use npc_mind::{ApplyWorldEventRequest, EventStore, InMemoryRepository};
use std::sync::Arc;

fn build_dispatcher(
    store: Arc<InMemoryMemoryStore>,
) -> (
    CommandDispatcher<InMemoryRepository>,
    Arc<InMemoryEventStore>,
) {
    let repo = InMemoryRepository::new();
    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(repo, event_store.clone(), bus)
        .with_default_handlers()
        .with_memory_full(store as Arc<dyn MemoryStore>);
    (dispatcher, event_store)
}

fn seed_canonical(store: &dyn MemoryStore, id: &str, topic: &str, content: &str, seq: u64) {
    #[allow(deprecated)]
    let e = MemoryEntry {
        id: id.into(),
        created_seq: seq,
        event_id: seq,
        scope: MemoryScope::World {
            world_id: "jianghu".into(),
        },
        source: MemorySource::Experienced,
        provenance: Provenance::Seeded,
        memory_type: MemoryType::WorldEvent,
        layer: MemoryLayer::A,
        content: content.into(),
        topic: Some(topic.into()),
        emotional_context: None,
        timestamp_ms: seq,
        last_recalled_at: None,
        recall_count: 0,
        origin_chain: vec![],
        confidence: 1.0,
        acquired_by: None,
        superseded_by: None,
        consolidated_into: None,
        npc_id: "jianghu".into(),
    };
    store.index(e, None).unwrap();
}

#[tokio::test]
async fn apply_world_event_emits_requested_and_occurred() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, event_store) = build_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::ApplyWorldEvent(ApplyWorldEventRequest {
            world_id: "jianghu".into(),
            topic: Some("leader".into()),
            fact: "새 맹주 등장".into(),
            significance: 0.8,
            witnesses: vec!["sage".into()],
        }))
        .await
        .expect("dispatch");

    let all = event_store.get_all_events();
    let req_count = all
        .iter()
        .filter(|e| e.kind() == EventKind::ApplyWorldEventRequested)
        .count();
    let occ_count = all
        .iter()
        .filter(|e| e.kind() == EventKind::WorldEventOccurred)
        .count();
    assert_eq!(req_count, 1);
    assert_eq!(occ_count, 1);
    // 이벤트 aggregate_id가 world_id로 라우팅되는지 확인
    let req = all
        .iter()
        .find(|e| e.kind() == EventKind::ApplyWorldEventRequested)
        .unwrap();
    assert_eq!(req.aggregate_id, "jianghu");
}

#[tokio::test]
async fn creates_canonical_entry_with_world_scope_seeded_provenance() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, _) = build_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::ApplyWorldEvent(ApplyWorldEventRequest {
            world_id: "jianghu".into(),
            topic: Some("leader".into()),
            fact: "새 맹주 등장".into(),
            significance: 0.5,
            witnesses: vec![],
        }))
        .await
        .unwrap();

    let canon = store
        .get_canonical_by_topic("leader")
        .unwrap()
        .expect("canonical 엔트리 발견되어야");
    assert_eq!(canon.content, "새 맹주 등장");
    assert_eq!(canon.memory_type, MemoryType::WorldEvent);
    assert_eq!(canon.provenance, Provenance::Seeded);
    assert!(matches!(
        canon.scope,
        MemoryScope::World { ref world_id } if world_id == "jianghu"
    ));
    assert!(canon.provenance.is_canonical(&canon.scope));
}

#[tokio::test]
async fn new_world_event_supersedes_existing_same_topic_canonical() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, _) = build_dispatcher(store.clone());

    // 기존 Canonical 시드
    seed_canonical(&*store, "old-canon", "leader", "옛 맹주", 1);
    // 사전 확인: Canonical이 옛 맹주
    let pre = store.get_canonical_by_topic("leader").unwrap().unwrap();
    assert_eq!(pre.content, "옛 맹주");

    dispatcher
        .dispatch_v2(Command::ApplyWorldEvent(ApplyWorldEventRequest {
            world_id: "jianghu".into(),
            topic: Some("leader".into()),
            fact: "새 맹주 등장".into(),
            significance: 0.7,
            witnesses: vec![],
        }))
        .await
        .unwrap();

    // 새 Canonical 반환
    let canon = store.get_canonical_by_topic("leader").unwrap().unwrap();
    assert_eq!(canon.content, "새 맹주 등장", "Canonical이 새 엔트리로 교체");

    // 기존 엔트리는 superseded_by가 채워져 있어야
    let old = store.get_by_id("old-canon").unwrap().unwrap();
    assert!(
        old.superseded_by.is_some(),
        "기존 Canonical은 supersede 마킹"
    );
}

#[tokio::test]
async fn non_canonical_personal_entries_on_same_topic_preserved() {
    // 리뷰 B1 회귀 가드 — Personal Heard/Rumor는 World 오버레이로 supersede되지 않아야.
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, _) = build_dispatcher(store.clone());

    // 기존 Canonical 시드
    seed_canonical(&*store, "old-canon", "leader", "옛 맹주", 1);
    // 어떤 NPC의 Personal Heard 엔트리도 시드
    let heard = MemoryEntry::personal(
        "pupil-heard",
        "pupil",
        "맹주가 바뀐다는 소문을 들었다",
        None,
        2,
        2,
        MemoryType::DialogueTurn,
    );
    // topic 명시 후 재저장
    let heard = MemoryEntry {
        topic: Some("leader".into()),
        source: MemorySource::Heard,
        ..heard
    };
    store.index(heard, None).unwrap();

    dispatcher
        .dispatch_v2(Command::ApplyWorldEvent(ApplyWorldEventRequest {
            world_id: "jianghu".into(),
            topic: Some("leader".into()),
            fact: "새 맹주 등장".into(),
            significance: 0.7,
            witnesses: vec![],
        }))
        .await
        .unwrap();

    // Canonical은 supersede됨
    let old_canon = store.get_by_id("old-canon").unwrap().unwrap();
    assert!(old_canon.superseded_by.is_some());

    // Personal Heard는 보존되어야 함
    let heard = store.get_by_id("pupil-heard").unwrap().unwrap();
    assert!(
        heard.superseded_by.is_none(),
        "Personal Heard는 World 오버레이로 supersede되지 않아야"
    );
}

#[tokio::test]
async fn topic_none_creates_entry_but_does_not_supersede() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, _) = build_dispatcher(store.clone());

    // 기존 topic 엔트리 시드
    seed_canonical(&*store, "old", "leader", "옛 사실", 1);

    dispatcher
        .dispatch_v2(Command::ApplyWorldEvent(ApplyWorldEventRequest {
            world_id: "jianghu".into(),
            topic: None,
            fact: "독립 사건".into(),
            significance: 0.5,
            witnesses: vec![],
        }))
        .await
        .unwrap();

    // 기존 엔트리는 supersede 되지 않아야 함
    let old = store.get_by_id("old").unwrap().unwrap();
    assert!(old.superseded_by.is_none(), "topic=None은 supersede 안 함");
    // 그래도 새 엔트리는 생성됨 (world:jianghu scope에 2건)
    assert_eq!(store.count(), 2);
}

#[tokio::test]
async fn invalid_world_event_rejected_early() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, _) = build_dispatcher(store.clone());

    // 빈 world_id → InvalidSituation
    let err = dispatcher
        .dispatch_v2(Command::ApplyWorldEvent(ApplyWorldEventRequest {
            world_id: "".into(),
            topic: None,
            fact: "something".into(),
            significance: 0.5,
            witnesses: vec![],
        }))
        .await
        .expect_err("should fail");
    assert!(format!("{err:?}").contains("world_id"));

    // 빈 fact → InvalidSituation
    let err = dispatcher
        .dispatch_v2(Command::ApplyWorldEvent(ApplyWorldEventRequest {
            world_id: "jianghu".into(),
            topic: None,
            fact: "   ".into(),
            significance: 0.5,
            witnesses: vec![],
        }))
        .await
        .expect_err("should fail");
    assert!(format!("{err:?}").contains("fact"));
}

#[tokio::test]
async fn significance_clamped_to_unit_range() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, event_store) = build_dispatcher(store.clone());

    // 범위 밖 significance → dispatcher가 [0, 1] clamp
    dispatcher
        .dispatch_v2(Command::ApplyWorldEvent(ApplyWorldEventRequest {
            world_id: "jianghu".into(),
            topic: None,
            fact: "극단 중요도".into(),
            significance: 999.0,
            witnesses: vec![],
        }))
        .await
        .unwrap();

    let req = event_store
        .get_all_events()
        .into_iter()
        .find(|e| e.kind() == EventKind::ApplyWorldEventRequested)
        .unwrap();
    let EventPayload::ApplyWorldEventRequested { significance, .. } = req.payload else {
        panic!("unexpected payload");
    };
    assert!(significance <= 1.0 && significance >= 0.0);
}
