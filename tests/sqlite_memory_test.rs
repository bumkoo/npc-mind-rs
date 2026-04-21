//! SqliteMemoryStore 테스트 — embed feature 필요
//!
//! `cargo test --features embed --test sqlite_memory_test`

#![cfg(feature = "embed")]
// Step B deprecated된 레거시 MemoryStore 메서드 호환성 검증을 계속 수행.
#![allow(deprecated)]

use npc_mind::adapter::sqlite_memory::SqliteMemoryStore;
use npc_mind::domain::memory::{MemoryEntry, MemoryType};
use npc_mind::MemoryStore;

/// 테스트 차원 — vec0 가상 테이블의 임베딩 크기와 일치해야 한다.
const TEST_DIM: usize = 3;

fn sample_entry(id: &str, npc_id: &str, content: &str, ts: u64) -> MemoryEntry {
    MemoryEntry::personal(
        id,
        npc_id,
        content,
        Some((0.5, -0.3, 0.1)),
        ts,
        1,
        MemoryType::DialogueTurn,
    )
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
fn sqlite_reindex_same_id_does_not_duplicate_fts_rows() {
    // FTS5 가상 테이블은 id 컬럼에 UNIQUE 제약이 없어, INSERT OR REPLACE로는
    // 기존 행을 덮어쓰지 못하고 새 rowid를 추가한다. 구현은 DELETE + INSERT로
    // 이를 회피해야 하며, 같은 id 재인덱싱 후 keyword 검색 결과가 1건이어야 한다.
    let store = SqliteMemoryStore::in_memory_with_dim(TEST_DIM).unwrap();

    store.index(sample_entry("m1", "npc1", "무림맹주가 배신했다", 100), None).unwrap();
    // 같은 id로 여러 번 덮어쓰기 — content도 모두 동일 키워드 포함
    store.index(sample_entry("m1", "npc1", "무림맹주가 배신했다 (v2)", 200), None).unwrap();
    store.index(sample_entry("m1", "npc1", "무림맹주가 배신했다 (v3)", 300), None).unwrap();

    let results = store.search_by_keyword("배신", None, 10).unwrap();
    assert_eq!(
        results.len(),
        1,
        "같은 id 재인덱싱이 FTS에 중복 행을 누적하면 안 됨"
    );
    assert_eq!(results[0].entry.id, "m1");
    // 최신 content가 살아있어야 한다 (INSERT OR REPLACE on memories 동작 확인).
    assert!(results[0].entry.content.contains("(v3)"));
}

#[test]
fn sqlite_memory_type_persistence_roundtrip() {
    // MemoryType의 5개 변종이 모두 as_persisted / from_persisted 왕복 후 보존되는지.
    let store = SqliteMemoryStore::in_memory_with_dim(TEST_DIM).unwrap();
    let types = [
        ("m1", MemoryType::DialogueTurn),
        ("m2", MemoryType::RelationshipChange),
        ("m3", MemoryType::BeatTransition),
        ("m4", MemoryType::SceneSummary),
        ("m5", MemoryType::GameEvent),
    ];
    for (id, ty) in &types {
        let entry = MemoryEntry::personal(
            *id,
            "npc1",
            format!("{id} content"),
            None,
            100,
            1,
            ty.clone(),
        );
        store.index(entry, None).unwrap();
    }

    let recent = store.get_recent("npc1", 10).unwrap();
    assert_eq!(recent.len(), 5);
    for (id, expected) in &types {
        let got = recent
            .iter()
            .find(|e| e.id == *id)
            .expect("entry not found")
            .memory_type
            .clone();
        assert_eq!(got, *expected, "{id} memory_type roundtrip");
    }
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
    assert_eq!(alice_only[0].entry.legacy_npc_id(), "alice");
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

// ---------------------------------------------------------------------------
// Step A (Foundation) — v2 schema + 신규 메서드
// ---------------------------------------------------------------------------

#[test]
fn sqlite_schema_meta_recorded_as_current_version() {
    // 신규 DB 생성 직후 schema_meta에 최신 버전이 기록되어야 한다.
    // v3 = rumor_distortions composite PK 이후 (Step C1 사후 리뷰 대응).
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("mem.db");
    {
        let _store =
            SqliteMemoryStore::with_dim(db_path.to_str().unwrap(), TEST_DIM).unwrap();
    }
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let version: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_meta", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 3);
}

#[test]
fn sqlite_v3_rumor_distortions_has_composite_primary_key() {
    // rumor_distortions는 `PRIMARY KEY (rumor_id, id)` — 서로 다른 rumor에 같은
    // distortion id가 충돌 없이 공존해야 Step C3 이후 안전.
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("mem.db");
    {
        let _store =
            SqliteMemoryStore::with_dim(db_path.to_str().unwrap(), TEST_DIM).unwrap();
    }
    let conn = rusqlite::Connection::open(&db_path).unwrap();

    conn.execute(
        "INSERT INTO rumors (id, origin_kind, reach_min_significance, created_at) \
         VALUES ('r1', 'seeded', 0.0, 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO rumors (id, origin_kind, reach_min_significance, created_at) \
         VALUES ('r2', 'seeded', 0.0, 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO rumor_distortions (id, rumor_id, parent, content, created_at) \
         VALUES ('d1', 'r1', NULL, 'v1', 0)",
        [],
    )
    .unwrap();
    // 같은 id "d1"이지만 rumor_id가 달라 composite PK 기준 별개 — 통과해야 함.
    conn.execute(
        "INSERT INTO rumor_distortions (id, rumor_id, parent, content, created_at) \
         VALUES ('d1', 'r2', NULL, 'v2', 0)",
        [],
    )
    .expect("composite PK must allow same distortion id across different rumors");

    // 같은 (rumor_id, id)는 거부되어야 함.
    let dup = conn.execute(
        "INSERT INTO rumor_distortions (id, rumor_id, parent, content, created_at) \
         VALUES ('d1', 'r1', NULL, 'dup', 0)",
        [],
    );
    assert!(
        dup.is_err(),
        "composite PK must reject same (rumor_id, id) duplicate"
    );
}

#[test]
fn sqlite_v2_canonical_index_exists() {
    // Canonical 조회 최적화 partial index가 설치되어 있어야 한다.
    // Step D의 WorldOverlayHandler·get_canonical_by_topic()이 사용할 지점.
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("mem.db");
    {
        let _store =
            SqliteMemoryStore::with_dim(db_path.to_str().unwrap(), TEST_DIM).unwrap();
    }
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'memories'")
        .unwrap();
    let indexes: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    for expected in [
        "idx_memories_topic",
        "idx_memories_topic_latest",
        "idx_memories_scope",
        "idx_memories_superseded",
        "idx_memories_source_layer",
        "idx_memories_provenance",
        "idx_memories_canonical",
    ] {
        assert!(
            indexes.iter().any(|i| i == expected),
            "expected index {expected} not found; have {indexes:?}"
        );
    }
}

#[test]
fn sqlite_v2_columns_exist_on_memories() {
    // 신규 13개 컬럼이 실제로 memories 테이블에 존재하는지.
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("mem.db");
    {
        let _store =
            SqliteMemoryStore::with_dim(db_path.to_str().unwrap(), TEST_DIM).unwrap();
    }
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let mut stmt = conn.prepare("PRAGMA table_info(memories)").unwrap();
    let cols: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    for expected in [
        "scope_kind",
        "owner_a",
        "owner_b",
        "source",
        "provenance",
        "layer",
        "topic",
        "origin_chain",
        "confidence",
        "acquired_by",
        "created_seq",
        "last_recalled_at",
        "recall_count",
        "superseded_by",
        "consolidated_into",
    ] {
        assert!(cols.contains(&expected.to_string()), "missing column: {expected}");
    }
}

#[test]
fn sqlite_v1_to_v2_migration_backfills_existing_rows() {
    // v1 스키마로 DB를 먼저 만든 뒤, SqliteMemoryStore::with_dim이 v2로 마이그레이션하면
    // 기존 행의 scope_kind='personal' / owner_a=npc_id / created_seq=event_id 기본값이
    // 채워져야 한다.
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("mem.db");

    // v1 스키마 수동 생성
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE memories (
                id TEXT PRIMARY KEY,
                npc_id TEXT NOT NULL,
                content TEXT NOT NULL,
                emotional_p REAL,
                emotional_a REAL,
                emotional_d REAL,
                timestamp_ms INTEGER NOT NULL,
                event_id INTEGER NOT NULL,
                memory_type TEXT NOT NULL
            );
            INSERT INTO memories (id, npc_id, content, timestamp_ms, event_id, memory_type)
            VALUES ('legacy-1', 'npc1', 'v1 legacy content', 100, 42, 'Dialogue');",
        )
        .unwrap();
    }

    // v2 adapter 오픈 → 마이그레이션 수행
    let store = SqliteMemoryStore::with_dim(db_path.to_str().unwrap(), TEST_DIM).unwrap();

    // 마이그레이션 후 get_by_id로 읽어 기본값 확인
    let entry = store.get_by_id("legacy-1").unwrap().expect("legacy row present");
    assert_eq!(entry.id, "legacy-1");
    assert_eq!(entry.legacy_npc_id(), "npc1");
    assert_eq!(entry.created_seq, 42); // event_id로 백필
    assert_eq!(entry.memory_type, MemoryType::DialogueTurn); // "Dialogue" alias 해석
    match &entry.scope {
        npc_mind::domain::memory::MemoryScope::Personal { npc_id } => {
            assert_eq!(npc_id, "npc1");
        }
        _ => panic!("expected Personal scope"),
    }
}

#[test]
fn sqlite_mark_superseded_and_topic_latest() {
    let store = SqliteMemoryStore::in_memory_with_dim(TEST_DIM).unwrap();

    let mut e1 = sample_entry("m1", "npc1", "v1", 100);
    e1.topic = Some("aquatica".into());
    e1.created_seq = 10;
    let mut e2 = sample_entry("m2", "npc1", "v2", 200);
    e2.topic = Some("aquatica".into());
    e2.created_seq = 20;
    store.index(e1, None).unwrap();
    store.index(e2, None).unwrap();

    // 최신은 m2
    let latest = store.get_by_topic_latest("aquatica").unwrap().unwrap();
    assert_eq!(latest.id, "m2");

    // m2를 supersede 처리하면 latest는 m1
    store.mark_superseded("m2", "m3-future").unwrap();
    let latest = store.get_by_topic_latest("aquatica").unwrap().unwrap();
    assert_eq!(latest.id, "m1");
}

#[test]
fn sqlite_record_recall_increments_counter() {
    let store = SqliteMemoryStore::in_memory_with_dim(TEST_DIM).unwrap();
    store.index(sample_entry("m1", "npc1", "회상 대상", 100), None).unwrap();

    let before = store.get_by_id("m1").unwrap().unwrap();
    assert_eq!(before.recall_count, 0);
    assert!(before.last_recalled_at.is_none());

    store.record_recall("m1", 500).unwrap();
    store.record_recall("m1", 600).unwrap();

    let after = store.get_by_id("m1").unwrap().unwrap();
    assert_eq!(after.recall_count, 2);
    assert_eq!(after.last_recalled_at, Some(600));
}

#[test]
fn sqlite_search_by_memory_query_filters_source_and_layer() {
    use npc_mind::domain::memory::{MemoryLayer, MemorySource};
    use npc_mind::ports::{MemoryQuery, MemoryScopeFilter};

    let store = SqliteMemoryStore::in_memory_with_dim(TEST_DIM).unwrap();

    let mut e1 = sample_entry("m1", "npc1", "experienced layer A", 100);
    e1.source = MemorySource::Experienced;
    let mut e2 = sample_entry("m2", "npc1", "heard layer A", 200);
    e2.source = MemorySource::Heard;
    let mut e3 = sample_entry("m3", "npc1", "experienced layer B", 300);
    e3.source = MemorySource::Experienced;
    e3.layer = MemoryLayer::B;
    e3.memory_type = MemoryType::SceneSummary;

    store.index(e1, None).unwrap();
    store.index(e2, None).unwrap();
    store.index(e3, None).unwrap();

    // Source 필터 — Experienced만
    let q = MemoryQuery {
        scope_filter: Some(MemoryScopeFilter::NpcAllowed("npc1".into())),
        source_filter: Some(vec![MemorySource::Experienced]),
        exclude_superseded: true,
        limit: 10,
        ..Default::default()
    };
    let out = store.search(q).unwrap();
    let ids: Vec<_> = out.iter().map(|r| r.entry.id.clone()).collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"m1".to_string()));
    assert!(ids.contains(&"m3".to_string()));

    // Layer 필터 — B만
    let q = MemoryQuery {
        scope_filter: Some(MemoryScopeFilter::NpcAllowed("npc1".into())),
        layer_filter: Some(MemoryLayer::B),
        exclude_superseded: true,
        limit: 10,
        ..Default::default()
    };
    let out = store.search(q).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].entry.id, "m3");
}
