//! SqliteMemoryStore 테스트 — embed feature 필요
//!
//! `cargo test --features embed --test sqlite_memory_test`

#![cfg(feature = "embed")]

use npc_mind::adapter::sqlite_memory::SqliteMemoryStore;
use npc_mind::domain::memory::{MemoryEntry, MemoryType};
use npc_mind::MemoryStore;

/// 테스트 차원 — vec0 가상 테이블의 임베딩 크기와 일치해야 한다.
const TEST_DIM: usize = 3;

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
    let store = SqliteMemoryStore::in_memory_with_dim(TEST_DIM).unwrap();
    assert_eq!(store.count(), 0);

    store.index(sample_entry("m1", "npc1", "첫 번째 기억", 100), None).unwrap();
    store.index(sample_entry("m2", "npc1", "두 번째 기억", 200), None).unwrap();
    assert_eq!(store.count(), 2);
}

#[test]
fn sqlite_fts5_keyword_search() {
    let store = SqliteMemoryStore::in_memory_with_dim(TEST_DIM).unwrap();

    store.index(sample_entry("m1", "npc1", "무림맹주가 배신을 암시했다", 100), None).unwrap();
    store.index(sample_entry("m2", "npc1", "화산파의 검법은 정교하다", 200), None).unwrap();

    let results = store.search_by_keyword("배신", None, 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entry.id, "m1");
}

#[test]
fn sqlite_keyword_search_matches_korean_multichar() {
    // 한글 다글자 쿼리가 부분 문자열을 포함한 항목을 반환하는 regression test.
    // 주의: trigram FTS5 또는 LIKE fallback 어느 경로를 통해서도 통과한다
    // (둘 다 "무림맹주를 ..."에서 "무림맹주"를 매치).
    // trigram 적용 여부의 구조적 검증은 sqlite_fts5_uses_trigram_tokenizer 참조.
    let store = SqliteMemoryStore::in_memory_with_dim(TEST_DIM).unwrap();

    store.index(sample_entry("m1", "npc1", "무림맹주를 칭송한다", 100), None).unwrap();
    store.index(sample_entry("m2", "npc1", "화산파 검법의 정수", 200), None).unwrap();
    store.index(sample_entry("m3", "npc1", "무림맹주가 등장했다", 300), None).unwrap();

    let results = store.search_by_keyword("무림맹주", None, 10).unwrap();
    assert_eq!(results.len(), 2);
    let ids: Vec<_> = results.iter().map(|r| r.entry.id.clone()).collect();
    assert!(ids.contains(&"m1".to_string()));
    assert!(ids.contains(&"m3".to_string()));
}

#[test]
fn sqlite_fts5_uses_trigram_tokenizer() {
    // memories_fts 가상 테이블이 실제로 trigram 토크나이저로 생성되었는지
    // sqlite_master의 CREATE 문을 직접 조회해 검증한다.
    // 이 테스트만이 trigram 적용 여부를 구조적으로 보장한다.
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("mem.db");

    {
        let _store = SqliteMemoryStore::with_dim(db_path.to_str().unwrap(), TEST_DIM).unwrap();
    }

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='memories_fts'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(
        sql.to_lowercase().contains("trigram"),
        "memories_fts는 trigram 토크나이저로 생성되어야 합니다. 실제 SQL: {sql}"
    );
}

#[test]
fn sqlite_vec0_cosine_search() {
    let store = SqliteMemoryStore::in_memory_with_dim(TEST_DIM).unwrap();

    // 단위 벡터로 구성해 cosine 거리가 결정적.
    let emb1 = vec![1.0, 0.0, 0.0]; // "배신" 방향
    let emb2 = vec![0.0, 1.0, 0.0]; // 다른 방향
    let emb3 = vec![0.95, 0.05, 0.0]; // "배신"에 매우 가까움

    store.index(sample_entry("m1", "npc1", "배신당했다", 100), Some(emb1)).unwrap();
    store.index(sample_entry("m2", "npc1", "친구가 되었다", 200), Some(emb2)).unwrap();
    store.index(sample_entry("m3", "npc1", "약속을 어겼다", 300), Some(emb3)).unwrap();

    let query = vec![1.0, 0.0, 0.0];
    let results = store.search_by_meaning(&query, None, 3).unwrap();

    assert_eq!(results.len(), 3);
    // cosine distance: m1=0.0, m3≈0.0013, m2=1.0 → 유사도 내림차순
    assert_eq!(results[0].entry.id, "m1");
    assert_eq!(results[1].entry.id, "m3");
    assert_eq!(results[2].entry.id, "m2");
    assert!(results[0].relevance_score > results[1].relevance_score);
    assert!(results[1].relevance_score > results[2].relevance_score);
}

#[test]
fn sqlite_vec0_npc_filter() {
    let store = SqliteMemoryStore::in_memory_with_dim(TEST_DIM).unwrap();

    store.index(sample_entry("m1", "alice", "alice 기억", 100), Some(vec![1.0, 0.0, 0.0])).unwrap();
    store.index(sample_entry("m2", "bob", "bob 기억", 200), Some(vec![1.0, 0.0, 0.0])).unwrap();

    let query = vec![1.0, 0.0, 0.0];
    let alice_only = store.search_by_meaning(&query, Some("alice"), 10).unwrap();
    assert_eq!(alice_only.len(), 1);
    assert_eq!(alice_only[0].entry.npc_id, "alice");
}

#[test]
fn sqlite_vec0_dimension_mismatch_errors() {
    let store = SqliteMemoryStore::in_memory_with_dim(TEST_DIM).unwrap();

    // 잘못된 차원 → EmbeddingError
    let entry = sample_entry("m1", "npc1", "차원 불일치", 100);
    let err = store.index(entry, Some(vec![0.1, 0.2])).unwrap_err();
    assert!(format!("{err}").contains("dim"));
}

#[test]
fn sqlite_get_recent_sorted() {
    let store = SqliteMemoryStore::in_memory_with_dim(TEST_DIM).unwrap();

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
    let store = SqliteMemoryStore::in_memory_with_dim(TEST_DIM).unwrap();

    store.index(sample_entry("m1", "npc1", "감정 기억", 100), None).unwrap();

    let recent = store.get_recent("npc1", 1).unwrap();
    assert_eq!(recent.len(), 1);
    let ctx = recent[0].emotional_context.unwrap();
    assert!((ctx.0 - 0.5).abs() < 0.01);
    assert!((ctx.1 - (-0.3)).abs() < 0.01);
    assert!((ctx.2 - 0.1).abs() < 0.01);
}

#[test]
fn sqlite_file_backed_roundtrip() {
    // TempDir로 실제 파일 기반 저장소의 index/search 동작 확인.
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("mem.db");
    let store = SqliteMemoryStore::with_dim(db_path.to_str().unwrap(), TEST_DIM).unwrap();

    store.index(sample_entry("m1", "npc1", "파일 기반", 100), Some(vec![1.0, 0.0, 0.0])).unwrap();

    let query = vec![1.0, 0.0, 0.0];
    let results = store.search_by_meaning(&query, None, 1).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entry.id, "m1");
}
