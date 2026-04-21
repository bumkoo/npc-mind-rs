//! InMemoryMemoryStore + MemoryAgent 통합 테스트 (feature flag 없이 실행 가능)

mod common;

use common::in_memory_store::InMemoryMemoryStore;
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::event::{DomainEvent, EventPayload};
use npc_mind::domain::memory::{MemoryEntry, MemoryType};
use npc_mind::{EventStore, MemoryStore};

use std::sync::Arc;

fn sample_entry(id: &str, npc_id: &str, content: &str, memory_type: MemoryType) -> MemoryEntry {
    MemoryEntry::personal(id, npc_id, content, None, 1000, 1, memory_type)
}

#[test]
fn in_memory_store_index_and_count() {
    let store = InMemoryMemoryStore::new();
    assert_eq!(store.count(), 0);

    store.index(sample_entry("m1", "npc1", "안녕하세요", MemoryType::DialogueTurn), None).unwrap();
    store.index(sample_entry("m2", "npc1", "반갑습니다", MemoryType::DialogueTurn), None).unwrap();
    assert_eq!(store.count(), 2);
}

#[test]
fn keyword_search_filters_by_content() {
    let store = InMemoryMemoryStore::new();
    store.index(sample_entry("m1", "npc1", "무림맹주가 배신을 암시했다", MemoryType::DialogueTurn), None).unwrap();
    store.index(sample_entry("m2", "npc1", "화산파의 검법은 정교하다", MemoryType::DialogueTurn), None).unwrap();
    store.index(sample_entry("m3", "npc2", "무림맹주를 만났다", MemoryType::DialogueTurn), None).unwrap();

    // 전체 검색
    let results = store.search_by_keyword("무림맹주", None, 10).unwrap();
    assert_eq!(results.len(), 2);

    // NPC 필터
    let results = store.search_by_keyword("무림맹주", Some("npc1"), 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entry.id, "m1");
}

#[test]
fn meaning_search_with_mock_embeddings() {
    let store = InMemoryMemoryStore::new();

    // Mock embeddings (3차원)
    let emb1 = vec![1.0, 0.0, 0.0]; // "배신" 방향
    let emb2 = vec![0.0, 1.0, 0.0]; // "우정" 방향
    let emb3 = vec![0.8, 0.2, 0.0]; // "배신" 유사

    store.index(sample_entry("m1", "npc1", "배신당했다", MemoryType::DialogueTurn), Some(emb1)).unwrap();
    store.index(sample_entry("m2", "npc1", "친구가 되었다", MemoryType::DialogueTurn), Some(emb2)).unwrap();
    store.index(sample_entry("m3", "npc1", "약속을 어겼다", MemoryType::DialogueTurn), Some(emb3)).unwrap();

    // "배신" 방향으로 검색
    let query = vec![0.9, 0.1, 0.0];
    let results = store.search_by_meaning(&query, None, 2).unwrap();

    assert_eq!(results.len(), 2);
    // 가장 유사한 것이 먼저
    assert_eq!(results[0].entry.id, "m1");
    assert!(results[0].relevance_score > results[1].relevance_score);
}

#[test]
fn get_recent_sorted_by_time() {
    let store = InMemoryMemoryStore::new();

    let mut e1 = sample_entry("m1", "npc1", "오래된 기억", MemoryType::DialogueTurn);
    e1.timestamp_ms = 100;
    let mut e2 = sample_entry("m2", "npc1", "최근 기억", MemoryType::DialogueTurn);
    e2.timestamp_ms = 300;
    let mut e3 = sample_entry("m3", "npc1", "중간 기억", MemoryType::DialogueTurn);
    e3.timestamp_ms = 200;

    store.index(e1, None).unwrap();
    store.index(e2, None).unwrap();
    store.index(e3, None).unwrap();

    let recent = store.get_recent("npc1", 2).unwrap();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].id, "m2"); // 최근
    assert_eq!(recent[1].id, "m3"); // 중간
}

#[test]
fn memory_entry_serialization() {
    let entry = sample_entry("m1", "npc1", "테스트", MemoryType::RelationshipChange);
    let json = serde_json::to_string(&entry).unwrap();
    let deserialized: MemoryEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, "m1");
    assert_eq!(deserialized.memory_type, MemoryType::RelationshipChange);
}

#[test]
fn dialogue_turn_completed_event_works() {
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());

    let id = store.next_id();
    let event = DomainEvent::new(
        id,
        "mu_baek".to_string(),
        1,
        EventPayload::DialogueTurnCompleted {
            npc_id: "mu_baek".to_string(),
            partner_id: "gyo_ryong".to_string(),
            speaker: "user".to_string(),
            utterance: "너를 믿어도 되겠느냐?".to_string(),
            emotion_snapshot: vec![("Fear".to_string(), 0.6)],
        },
    );

    store.append(&[event.clone()]);
    bus.publish(&event);

    let events = store.get_all_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].payload_type(), "DialogueTurnCompleted");
}

#[test]
fn npc_id_filter_isolates_memories() {
    let store = InMemoryMemoryStore::new();
    store.index(sample_entry("m1", "alice", "alice의 기억", MemoryType::DialogueTurn), None).unwrap();
    store.index(sample_entry("m2", "bob", "bob의 기억", MemoryType::DialogueTurn), None).unwrap();

    let alice = store.get_recent("alice", 10).unwrap();
    assert_eq!(alice.len(), 1);
    assert_eq!(alice[0].legacy_npc_id(), "alice");

    let bob = store.get_recent("bob", 10).unwrap();
    assert_eq!(bob.len(), 1);
    assert_eq!(bob[0].legacy_npc_id(), "bob");
}
