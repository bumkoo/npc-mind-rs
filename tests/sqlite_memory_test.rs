//! SqliteMemoryStore 테스트 — embed feature 필요
//!
//! `cargo test --features embed --test sqlite_memory_test`

#![cfg(feature = "embed")]

use npc_mind::adapter::sqlite_memory::SqliteMemoryStore;
use npc_mind::domain::memory::{MemoryEntry, MemoryType};
use npc_mind::MemoryStore;

fn sample_entry(id: &str, npc_id: &str, content: &str, ts: u64) -> MemoryEntry {
    MemoryEntry {
        id: id.to_string(),
        npc_id: npc_id.to_string(),
        content: content.to_string(),
        emotional_context: Some((0.5, -0.3, 0.1)),
        timestamp_ms: ts,
        event_id: 1,
        memory_type: MemoryType::Dialogue,
    }
}

#[test]
fn sqlite_create_and_index() {
    let store = SqliteMemoryStore::in_memory().unwrap();
    assert_eq!(store.count(), 0);

    store.index(sample_entry("m1", "npc1", "첫 번째 기억", 100), None).unwrap();
    store.index(sample_entry("m2", "npc1", "두 번째 기억", 200), None).unwrap();
    assert_eq!(store.count(), 2);
}

#[test]
fn sqlite_fts5_keyword_search() {
    let store = SqliteMemoryStore::in_memory().unwrap();

    store.index(sample_entry("m1", "npc1", "무림맹주가 배신을 암시했다", 100), None).unwrap();
    store.index(sample_entry("m2", "npc1", "화산파의 검법은 정교하다", 200), None).unwrap();

    let results = store.search_by_keyword("배신", None, 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entry.id, "m1");
}

#[test]
fn sqlite_vector_search() {
    let store = SqliteMemoryStore::in_memory().unwrap();

    let emb1 = vec![1.0, 0.0, 0.0];
    let emb2 = vec![0.0, 1.0, 0.0];

    store.index(sample_entry("m1", "npc1", "배신당했다", 100), Some(emb1)).unwrap();
    store.index(sample_entry("m2", "npc1", "친구가 되었다", 200), Some(emb2)).unwrap();

    let query = vec![0.9, 0.1, 0.0];
    let results = store.search_by_meaning(&query, None, 2).unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].entry.id, "m1"); // 더 유사
    assert!(results[0].relevance_score > results[1].relevance_score);
}

#[test]
fn sqlite_get_recent_sorted() {
    let store = SqliteMemoryStore::in_memory().unwrap();

    store.index(sample_entry("m1", "npc1", "오래된", 100), None).unwrap();
    store.index(sample_entry("m2", "npc1", "최근", 300), None).unwrap();
    store.index(sample_entry("m3", "npc1", "중간", 200), None).unwrap();

    let recent = store.get_recent("npc1", 2).unwrap();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].id, "m2");
    assert_eq!(recent[1].id, "m3");
}

#[test]
fn sqlite_emotional_context_preserved() {
    let store = SqliteMemoryStore::in_memory().unwrap();

    store.index(sample_entry("m1", "npc1", "감정 기억", 100), None).unwrap();

    let recent = store.get_recent("npc1", 1).unwrap();
    assert_eq!(recent.len(), 1);
    let ctx = recent[0].emotional_context.unwrap();
    assert!((ctx.0 - 0.5).abs() < 0.01);
    assert!((ctx.1 - (-0.3)).abs() < 0.01);
    assert!((ctx.2 - 0.1).abs() < 0.01);
}
