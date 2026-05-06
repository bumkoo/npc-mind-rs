//! Step D — ApplyWorldEvent 커맨드 경로 end-to-end 통합 테스트.
//!
//! 커버리지:
//! - `Command::ApplyWorldEvent` → `WorldEventOccurred` 발행
//! - `WorldEventOccurred` → `WorldOverlayHandler` → `MemoryStore.index`
//! - 같은 topic의 이전 Canonical 엔트리 supersede 확인
//! - Personal Heard 엔트리는 supersede되지 않음 확인 (B6 원칙)
//! - world_id 유효성 검증

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
use std::sync::{Arc, Mutex};

fn setup_dispatcher(
    store: Arc<InMemoryMemoryStore>,
) -> (
    CommandDispatcher<InMemoryRepository>,
    Arc<InMemoryEventStore>,
) {
    let repo = InMemoryRepository::new();
    let repo_arc = Arc::new(Mutex::new(repo));
    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(repo_arc, event_store.clone(), bus)
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
        timestamp_ms: 100 * seq,
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
async fn world_overlay_creates_canonical_entry_and_emits_occurred() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, event_store) = setup_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::ApplyWorldEvent(Box::new(ApplyWorldEventRequest {
            world_id: "jianghu".into(),
            topic: Some("leader".into()),
            fact: "새 맹주 등극".into(),
            significance: 0.8,
            witnesses: vec![],
        })))
        .await
        .expect("must succeed");

    let all = event_store.get_all_events();
    // 1. 초기 Requested 발행됨
    assert!(all
        .iter()
        .any(|e| e.kind() == EventKind::ApplyWorldEventRequested));
    // 2. 후속 Occurred 발행됨
    assert!(all
        .iter()
        .any(|e| e.kind() == EventKind::WorldEventOccurred));

    // 3. 메모리에 저장됨
    let occurred = all
        .iter()
        .find(|e| e.kind() == EventKind::WorldEventOccurred)
        .unwrap();
    let entry = store.get_by_id(&format!("world-{:012}-jianghu", occurred.id)).unwrap().unwrap();
    assert_eq!(entry.content, "새 맹주 등극");
    assert_eq!(entry.provenance, Provenance::Seeded);
}

#[tokio::test]
async fn world_overlay_supersedes_existing_canonical_entry() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, _) = setup_dispatcher(store.clone());

    seed_canonical(&*store, "old-canon", "leader", "옛 맹주", 1);

    dispatcher
        .dispatch_v2(Command::ApplyWorldEvent(Box::new(ApplyWorldEventRequest {
            world_id: "jianghu".into(),
            topic: Some("leader".into()),
            fact: "새 맹주".into(),
            significance: 0.8,
            witnesses: vec![],
        })))
        .await
        .unwrap();

    let old = store.get_by_id("old-canon").unwrap().unwrap();
    assert!(old.superseded_by.is_some(), "기존 Canonical은 supersede되어야 함");
    
    let latest = store.get_canonical_by_topic("leader").unwrap().unwrap();
    assert_eq!(latest.content, "새 맹주");
}

#[tokio::test]
async fn world_overlay_does_not_supersede_personal_heard_entry() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, _) = setup_dispatcher(store.clone());

    // Personal Heard 엔트리 시드
    #[allow(deprecated)]
    let heard = MemoryEntry {
        id: "personal-heard".into(),
        created_seq: 1,
        event_id: 1,
        scope: MemoryScope::Personal {
            npc_id: "pupil".into(),
        },
        source: MemorySource::Heard,
        provenance: Provenance::Runtime,
        memory_type: MemoryType::DialogueTurn,
        layer: MemoryLayer::A,
        content: "내가 들은 소문".into(),
        topic: Some("leader".into()),
        emotional_context: None,
        timestamp_ms: 100,
        last_recalled_at: None,
        recall_count: 0,
        origin_chain: vec!["sage".into()],
        confidence: 0.8,
        acquired_by: None,
        superseded_by: None,
        consolidated_into: None,
        npc_id: "pupil".into(),
    };
    store.index(heard, None).unwrap();

    dispatcher
        .dispatch_v2(Command::ApplyWorldEvent(Box::new(ApplyWorldEventRequest {
            world_id: "jianghu".into(),
            topic: Some("leader".into()),
            fact: "정식 공표된 새 맹주".into(),
            significance: 0.8,
            witnesses: vec![],
        })))
        .await
        .unwrap();

    let old = store.get_by_id("personal-heard").unwrap().unwrap();
    assert!(old.superseded_by.is_none(), "Personal 엔트리는 보호되어야 함");
}

#[tokio::test]
async fn world_overlay_without_topic_succeeds_but_does_not_supersede() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, _) = setup_dispatcher(store.clone());

    seed_canonical(&*store, "c1", "leader", "맹주 A", 1);

    dispatcher
        .dispatch_v2(Command::ApplyWorldEvent(Box::new(ApplyWorldEventRequest {
            world_id: "jianghu".into(),
            topic: None, // topic 없음
            fact: "독립적 세계 사건".into(),
            significance: 0.5,
            witnesses: vec![],
        })))
        .await
        .unwrap();

    let old = store.get_by_id("c1").unwrap().unwrap();
    assert!(old.superseded_by.is_none());
}

#[tokio::test]
async fn apply_world_event_fails_with_invalid_input() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, _) = setup_dispatcher(store.clone());

    // 1. world_id 누락
    let err1 = dispatcher
        .dispatch_v2(Command::ApplyWorldEvent(Box::new(ApplyWorldEventRequest {
            world_id: "".into(),
            topic: None,
            fact: "fact".into(),
            significance: 0.5,
            witnesses: vec![],
        })))
        .await;
    assert!(err1.is_err());

    // 2. fact 누락
    let err2 = dispatcher
        .dispatch_v2(Command::ApplyWorldEvent(Box::new(ApplyWorldEventRequest {
            world_id: "w".into(),
            topic: None,
            fact: "  ".into(),
            significance: 0.5,
            witnesses: vec![],
        })))
        .await;
    assert!(err2.is_err());
}

#[tokio::test]
async fn world_overlay_records_requested_event_with_cascade_0() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, event_store) = setup_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::ApplyWorldEvent(Box::new(ApplyWorldEventRequest {
            world_id: "jianghu".into(),
            topic: None,
            fact: "fact".into(),
            significance: 0.5,
            witnesses: vec![],
        })))
        .await
        .unwrap();

    let req = event_store
        .get_all_events()
        .into_iter()
        .find(|e| e.kind() == EventKind::ApplyWorldEventRequested)
        .unwrap();
    
    assert_eq!(req.metadata.cascade_depth, 0);
    assert_eq!(req.metadata.parent_event_id, None);
}
