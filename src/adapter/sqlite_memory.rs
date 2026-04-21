//! SqliteMemoryStore — SQLite FTS5(텍스트/키워드) + sqlite-vec vec0(임베딩) 기반 기억 저장소
//!
//! 하나의 SQLite 파일 안에서 세 레이어가 `id`로 조인된다:
//! - `memories`      : 일반 테이블 (메타데이터 + 원문 TEXT)
//! - `memories_fts`  : FTS5 가상 테이블 (키워드 전문 검색)
//! - `memories_vec`  : sqlite-vec `vec0` 가상 테이블 (코사인 ANN)
//!
//! sqlite-vec는 순수 C 확장이라 tokio 런타임을 요구하지 않는다.
//! `embed` feature가 활성화되어도 라이브러리 코어의 runtime-agnostic 원칙은 유지된다.

use crate::domain::memory::{
    MemoryEntry, MemoryLayer, MemoryResult, MemoryScope, MemorySource, MemoryType, Provenance,
};
use crate::ports::{MemoryError, MemoryQuery, MemoryScopeFilter, MemoryStore};
use rusqlite::{ffi::sqlite3_auto_extension, params, Connection};
use sqlite_vec::sqlite3_vec_init;
use std::sync::{Mutex, Once};
use zerocopy::AsBytes;

/// bge-m3 dense 임베딩 차원 (기본값).
pub const DEFAULT_EMBEDDING_DIM: usize = 1024;

/// sqlite-vec auto-extension 등록은 프로세스 전역 1회만 수행.
static VEC_INIT: Once = Once::new();

fn ensure_vec_extension_loaded() {
    VEC_INIT.call_once(|| {
        // sqlite3_vec_init을 auto-extension으로 등록하면
        // 이후 모든 Connection::open() 호출에서 vec0 모듈이 로드된다.
        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite3_vec_init as *const (),
            )));
        }
    });
}

/// SQLite 기반 기억 저장소 (FTS5 + sqlite-vec).
pub struct SqliteMemoryStore {
    conn: Mutex<Connection>,
    dim: usize,
}

impl SqliteMemoryStore {
    /// 파일 기반 저장소 생성. 임베딩 차원은 `DEFAULT_EMBEDDING_DIM`(1024).
    pub fn new(path: &str) -> Result<Self, MemoryError> {
        Self::with_dim(path, DEFAULT_EMBEDDING_DIM)
    }

    /// 파일 기반 저장소 + 임베딩 차원 지정.
    pub fn with_dim(path: &str, dim: usize) -> Result<Self, MemoryError> {
        ensure_vec_extension_loaded();
        let conn = Connection::open(path)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let store = Self {
            conn: Mutex::new(conn),
            dim,
        };
        store.init_tables()?;
        Ok(store)
    }

    /// 인메모리 저장소 생성 (테스트용). 기본 차원.
    pub fn in_memory() -> Result<Self, MemoryError> {
        Self::in_memory_with_dim(DEFAULT_EMBEDDING_DIM)
    }

    /// 인메모리 저장소 + 임베딩 차원 지정 (테스트용).
    pub fn in_memory_with_dim(dim: usize) -> Result<Self, MemoryError> {
        ensure_vec_extension_loaded();
        let conn = Connection::open_in_memory()
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let store = Self {
            conn: Mutex::new(conn),
            dim,
        };
        store.init_tables()?;
        Ok(store)
    }

    fn init_tables(&self) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();

        // Schema version 관리
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_meta (version INTEGER PRIMARY KEY)",
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        let current: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_meta",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        if current < 1 {
            Self::migrate_v1(&conn, self.dim)?;
        }
        if current < 2 {
            Self::migrate_v2(&conn, self.dim)?;
        }

        conn.execute(
            "INSERT OR REPLACE INTO schema_meta(version) VALUES (2)",
            [],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        Ok(())
    }

    /// v1 — 초기 스키마. 기존 파일이 v1 이상이면 `CREATE TABLE IF NOT EXISTS`로 no-op.
    fn migrate_v1(conn: &Connection, dim: usize) -> Result<(), MemoryError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memories (
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
            CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts
                USING fts5(id, content, tokenize='trigram');",
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        let vec_ddl = format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memories_vec USING vec0(
                id TEXT PRIMARY KEY,
                npc_id TEXT partition key,
                embedding FLOAT[{dim}] distance_metric=cosine
            );"
        );
        conn.execute_batch(&vec_ddl)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(())
    }

    /// v2 — Step A 확장. ALTER TABLE로 13개 컬럼 + 6개 인덱스 + vec0 재생성.
    ///
    /// `memories_vec`은 dim·partition key 스키마가 고정이라 ALTER 불가 → 기존 테이블을
    /// 드랍 후 재생성하고, 기존 행은 새 partition_key 포맷으로 재인덱싱한다.
    /// (v1 데이터의 npc_id → `"personal:<npc_id>"`)
    fn migrate_v2(conn: &Connection, dim: usize) -> Result<(), MemoryError> {
        // ALTER TABLE — 13 신규 컬럼
        let alters = [
            "ALTER TABLE memories ADD COLUMN scope_kind TEXT NOT NULL DEFAULT 'personal'",
            "ALTER TABLE memories ADD COLUMN owner_a TEXT",
            "ALTER TABLE memories ADD COLUMN owner_b TEXT",
            "ALTER TABLE memories ADD COLUMN source TEXT NOT NULL DEFAULT 'experienced'",
            "ALTER TABLE memories ADD COLUMN provenance TEXT NOT NULL DEFAULT 'runtime'",
            "ALTER TABLE memories ADD COLUMN layer TEXT NOT NULL DEFAULT 'a'",
            "ALTER TABLE memories ADD COLUMN topic TEXT",
            "ALTER TABLE memories ADD COLUMN origin_chain TEXT",
            "ALTER TABLE memories ADD COLUMN confidence REAL NOT NULL DEFAULT 1.0",
            "ALTER TABLE memories ADD COLUMN acquired_by TEXT",
            "ALTER TABLE memories ADD COLUMN created_seq INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE memories ADD COLUMN last_recalled_at INTEGER",
            "ALTER TABLE memories ADD COLUMN recall_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE memories ADD COLUMN superseded_by TEXT REFERENCES memories(id)",
            "ALTER TABLE memories ADD COLUMN consolidated_into TEXT REFERENCES memories(id)",
        ];
        for stmt in &alters {
            // 이미 존재하면 SQLite가 "duplicate column name" 에러를 내지만 migration 재실행
            // 시나리오에서만 발생 — 무시한다.
            let _ = conn.execute(stmt, []);
        }

        // 기존 v1 행의 신규 컬럼 기본값 백필
        conn.execute(
            "UPDATE memories SET owner_a = npc_id WHERE owner_a IS NULL",
            [],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        conn.execute(
            "UPDATE memories SET created_seq = event_id WHERE created_seq = 0",
            [],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        // 인덱스
        let indexes = [
            "CREATE INDEX IF NOT EXISTS idx_memories_topic ON memories(topic) WHERE topic IS NOT NULL",
            "CREATE INDEX IF NOT EXISTS idx_memories_topic_latest ON memories(topic, created_seq DESC) WHERE topic IS NOT NULL AND superseded_by IS NULL",
            "CREATE INDEX IF NOT EXISTS idx_memories_scope ON memories(scope_kind, owner_a, owner_b)",
            "CREATE INDEX IF NOT EXISTS idx_memories_superseded ON memories(superseded_by)",
            "CREATE INDEX IF NOT EXISTS idx_memories_source_layer ON memories(source, layer)",
            "CREATE INDEX IF NOT EXISTS idx_memories_provenance ON memories(provenance, scope_kind)",
        ];
        for stmt in &indexes {
            conn.execute(stmt, [])
                .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        }

        // vec0 재생성 — partition key를 npc_id → partition_key로 변경
        // 기존 벡터 데이터를 새 테이블로 옮기기
        let has_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM memories_vec", [], |r| r.get(0))
            .unwrap_or(0);

        // 기존 벡터 (있으면) 수집
        let mut migrated_vecs: Vec<(String, String, Vec<u8>)> = Vec::new();
        if has_rows > 0 {
            let sql = "SELECT v.id, m.npc_id, v.embedding FROM memories_vec v JOIN memories m ON v.id = m.id";
            let mut stmt = conn
                .prepare(sql)
                .map_err(|e| MemoryError::StorageError(e.to_string()))?;
            let rows = stmt
                .query_map([], |row| {
                    let id: String = row.get(0)?;
                    let npc_id: String = row.get(1)?;
                    let emb: Vec<u8> = row.get(2)?;
                    Ok((id, format!("personal:{npc_id}"), emb))
                })
                .map_err(|e| MemoryError::StorageError(e.to_string()))?;
            for r in rows {
                if let Ok(v) = r {
                    migrated_vecs.push(v);
                }
            }
        }

        conn.execute("DROP TABLE IF EXISTS memories_vec", [])
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let vec_ddl = format!(
            "CREATE VIRTUAL TABLE memories_vec USING vec0(
                id TEXT PRIMARY KEY,
                partition_key TEXT partition key,
                embedding FLOAT[{dim}] distance_metric=cosine
            );"
        );
        conn.execute_batch(&vec_ddl)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        for (id, pkey, emb) in migrated_vecs {
            conn.execute(
                "INSERT INTO memories_vec (id, partition_key, embedding) VALUES (?1, ?2, ?3)",
                params![id, pkey, emb],
            )
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        }

        // Rumor 테이블 (Step C에서 사용 시작, v2에서 테이블만 선제 생성)
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS rumors (
                id TEXT PRIMARY KEY,
                topic TEXT,
                seed_content TEXT,
                origin_kind TEXT NOT NULL,
                origin_ref TEXT,
                reach_regions TEXT,
                reach_factions TEXT,
                reach_npc_ids TEXT,
                reach_min_significance REAL,
                status TEXT NOT NULL DEFAULT 'active',
                created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS rumor_hops (
                rumor_id TEXT NOT NULL REFERENCES rumors(id),
                hop_index INTEGER NOT NULL,
                content_version TEXT,
                recipients TEXT NOT NULL,
                spread_at INTEGER NOT NULL,
                PRIMARY KEY (rumor_id, hop_index)
            );
            CREATE TABLE IF NOT EXISTS rumor_distortions (
                id TEXT PRIMARY KEY,
                rumor_id TEXT NOT NULL REFERENCES rumors(id),
                parent TEXT REFERENCES rumor_distortions(id),
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_rumors_topic ON rumors(topic) WHERE topic IS NOT NULL;
            CREATE INDEX IF NOT EXISTS idx_rumors_status ON rumors(status);",
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        Ok(())
    }
}

impl MemoryStore for SqliteMemoryStore {
    fn index(&self, entry: MemoryEntry, embedding: Option<Vec<f32>>) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();

        let (ep, ea, ed) = match entry.emotional_context {
            Some((p, a, d)) => (Some(p), Some(a), Some(d)),
            None => (None, None, None),
        };

        let origin_chain_json = serde_json::to_string(&entry.origin_chain).unwrap_or_default();
        let partition_key = entry.scope.partition_key();
        let legacy_npc_id = entry.scope.owner_a().to_string();

        conn.execute(
            "INSERT OR REPLACE INTO memories (
                id, npc_id, content, emotional_p, emotional_a, emotional_d,
                timestamp_ms, event_id, memory_type,
                scope_kind, owner_a, owner_b, source, provenance, layer, topic,
                origin_chain, confidence, acquired_by, created_seq,
                last_recalled_at, recall_count, superseded_by, consolidated_into
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                     ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                     ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)",
            params![
                entry.id,
                legacy_npc_id,
                entry.content,
                ep,
                ea,
                ed,
                entry.timestamp_ms as i64,
                entry.event_id as i64,
                entry.memory_type.as_persisted(),
                entry.scope.kind(),
                entry.scope.owner_a(),
                entry.scope.owner_b(),
                source_as_persisted(entry.source),
                provenance_as_persisted(entry.provenance),
                layer_as_persisted(entry.layer),
                entry.topic,
                origin_chain_json,
                entry.confidence as f64,
                entry.acquired_by,
                entry.created_seq as i64,
                entry.last_recalled_at.map(|v| v as i64),
                entry.recall_count as i64,
                entry.superseded_by,
                entry.consolidated_into,
            ],
        ).map_err(|e| MemoryError::StorageError(e.to_string()))?;

        // FTS5 중복 방지 (DELETE 후 INSERT)
        conn.execute(
            "DELETE FROM memories_fts WHERE id = ?1",
            params![entry.id],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        conn.execute(
            "INSERT INTO memories_fts (id, content) VALUES (?1, ?2)",
            params![entry.id, entry.content],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        if let Some(emb) = embedding {
            if emb.len() != self.dim {
                return Err(MemoryError::EmbeddingError(format!(
                    "embedding dim {} != expected {}",
                    emb.len(),
                    self.dim
                )));
            }
            conn.execute(
                "DELETE FROM memories_vec WHERE id = ?1",
                params![entry.id],
            )
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
            conn.execute(
                "INSERT INTO memories_vec (id, partition_key, embedding) VALUES (?1, ?2, ?3)",
                params![entry.id, partition_key, emb.as_bytes()],
            )
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        }

        Ok(())
    }

    fn search_by_meaning(
        &self,
        query_embedding: &[f32],
        npc_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryResult>, MemoryError> {
        if query_embedding.len() != self.dim {
            return Err(MemoryError::EmbeddingError(format!(
                "query dim {} != expected {}",
                query_embedding.len(),
                self.dim
            )));
        }

        let conn = self.conn.lock().unwrap();

        // v2: partition_key 포맷으로 필터 (`"personal:<npc_id>"`)
        let partition_key_filter = npc_id.map(|id| format!("personal:{id}"));

        let sql = "SELECT v.id, v.distance
                   FROM memories_vec v
                   WHERE v.embedding MATCH ?1
                     AND k = ?2
                     AND (?3 IS NULL OR v.partition_key = ?3)
                   ORDER BY v.distance";

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        let rows = stmt
            .query_map(
                params![
                    query_embedding.as_bytes(),
                    limit as i64,
                    partition_key_filter
                ],
                |row| {
                    let id: String = row.get(0)?;
                    let distance: f64 = row.get(1)?;
                    Ok((id, distance as f32))
                },
            )
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        let scored: Vec<(String, f32)> = rows.filter_map(|r| r.ok()).collect();

        let results = scored
            .into_iter()
            .filter_map(|(id, distance)| {
                let entry = load_entry(&conn, &id).ok()?;
                // cosine distance(0=동일, 2=반대) → similarity [−1, 1] 범위로 변환.
                Some(MemoryResult {
                    entry,
                    relevance_score: 1.0 - distance,
                })
            })
            .collect();

        Ok(results)
    }

    fn search_by_keyword(
        &self,
        keyword: &str,
        npc_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryResult>, MemoryError> {
        let conn = self.conn.lock().unwrap();

        // FTS5(trigram)로 검색 — trigram 토크나이저가 한글/CJK를 3-gram으로 분해해
        // 기본 토크나이저의 언어별 단어 경계 문제를 우회한다.
        // 잘못된 FTS5 구문이나 기타 예외 시 LIKE fallback으로 방어.
        let fts_results: Vec<MemoryEntry> = conn
            .prepare(
                "SELECT m.* FROM memories m
                 JOIN memories_fts f ON m.id = f.id
                 WHERE memories_fts MATCH ?1
                 LIMIT ?2",
            )
            .and_then(|mut stmt| {
                stmt.query_map(params![keyword, (limit * 10) as i64], row_to_entry)
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
            })
            .unwrap_or_default();

        let results: Vec<MemoryEntry> = if fts_results.is_empty() {
            let pattern = format!("%{}%", keyword);
            conn.prepare(
                "SELECT * FROM memories WHERE content LIKE ?1 LIMIT ?2",
            )
            .and_then(|mut stmt| {
                stmt.query_map(params![pattern, (limit * 10) as i64], row_to_entry)
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
            })
            .unwrap_or_default()
        } else {
            fts_results
        };

        let results = results
            .into_iter()
            .filter(|e| npc_id.map_or(true, |id| e.legacy_npc_id() == id))
            .take(limit)
            .map(|entry| MemoryResult {
                entry,
                relevance_score: 1.0,
            })
            .collect();

        Ok(results)
    }

    fn get_recent(
        &self,
        npc_id: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT * FROM memories WHERE npc_id = ?1 ORDER BY timestamp_ms DESC LIMIT ?2",
            )
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        let results = stmt
            .query_map(params![npc_id, limit as i64], row_to_entry)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    fn count(&self) -> usize {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))
            .unwrap_or(0)
    }

    // -----------------------------------------------------------------------
    // Step A 신규 메서드
    // -----------------------------------------------------------------------

    fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryResult>, MemoryError> {
        let conn = self.conn.lock().unwrap();

        // WHERE 절 조립 — 선택적 필터를 bind 파라미터로.
        let mut where_parts: Vec<String> = Vec::new();
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        match &query.scope_filter {
            None | Some(MemoryScopeFilter::Any) => {}
            Some(MemoryScopeFilter::Exact(scope)) => {
                where_parts.push(format!(
                    "scope_kind = ?{} AND owner_a = ?{}",
                    binds.len() + 1,
                    binds.len() + 2
                ));
                binds.push(Box::new(scope.kind().to_string()));
                binds.push(Box::new(scope.owner_a().to_string()));
                if let Some(ob) = scope.owner_b() {
                    where_parts
                        .push(format!("owner_b = ?{}", binds.len() + 1));
                    binds.push(Box::new(ob.to_string()));
                } else {
                    where_parts.push("owner_b IS NULL".into());
                }
            }
            Some(MemoryScopeFilter::NpcAllowed(npc)) => {
                // Personal Scope with matching npc_id OR World scope OR Relationship touching npc.
                // Faction/Family은 Step C에서 NpcWorld join 도입 예정.
                where_parts.push(format!(
                    "(\
                        (scope_kind = 'personal' AND owner_a = ?{n}) \
                        OR scope_kind = 'world' \
                        OR (scope_kind = 'relationship' AND (owner_a = ?{n} OR owner_b = ?{n}))\
                    )",
                    n = binds.len() + 1
                ));
                binds.push(Box::new(npc.clone()));
            }
        }

        if let Some(ref sources) = query.source_filter {
            if !sources.is_empty() {
                let placeholders: Vec<String> = sources
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("?{}", binds.len() + 1 + i))
                    .collect();
                where_parts.push(format!("source IN ({})", placeholders.join(",")));
                for s in sources {
                    binds.push(Box::new(source_as_persisted(*s).to_string()));
                }
            }
        }

        if let Some(layer) = query.layer_filter {
            where_parts.push(format!("layer = ?{}", binds.len() + 1));
            binds.push(Box::new(layer_as_persisted(layer).to_string()));
        }

        if let Some(ref topic) = query.topic {
            where_parts.push(format!("topic = ?{}", binds.len() + 1));
            binds.push(Box::new(topic.clone()));
        }

        if query.exclude_superseded {
            where_parts.push("superseded_by IS NULL".into());
        }
        if query.exclude_consolidated_source {
            where_parts.push("consolidated_into IS NULL".into());
        }

        let where_clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };
        let limit_clause = if query.limit > 0 {
            format!("LIMIT {}", query.limit)
        } else {
            String::new()
        };
        let sql = format!(
            "SELECT * FROM memories {where_clause} ORDER BY created_seq DESC {limit_clause}"
        );

        let param_refs: Vec<&dyn rusqlite::ToSql> =
            binds.iter().map(|b| b.as_ref() as &dyn rusqlite::ToSql).collect();

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let entries: Vec<MemoryEntry> = stmt
            .query_map(rusqlite::params_from_iter(param_refs), row_to_entry)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries
            .into_iter()
            .map(|entry| MemoryResult {
                entry,
                relevance_score: 1.0,
            })
            .collect())
    }

    fn get_by_id(&self, id: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let res = conn.query_row(
            "SELECT * FROM memories WHERE id = ?1",
            params![id],
            row_to_entry,
        );
        match res {
            Ok(e) => Ok(Some(e)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(MemoryError::StorageError(e.to_string())),
        }
    }

    fn get_by_topic_latest(&self, topic: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let res = conn.query_row(
            "SELECT * FROM memories
             WHERE topic = ?1 AND superseded_by IS NULL
             ORDER BY created_seq DESC LIMIT 1",
            params![topic],
            row_to_entry,
        );
        match res {
            Ok(e) => Ok(Some(e)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(MemoryError::StorageError(e.to_string())),
        }
    }

    fn get_canonical_by_topic(&self, topic: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let res = conn.query_row(
            "SELECT * FROM memories
             WHERE topic = ?1
               AND provenance = 'seeded'
               AND scope_kind = 'world'
               AND superseded_by IS NULL
             ORDER BY created_seq DESC LIMIT 1",
            params![topic],
            row_to_entry,
        );
        match res {
            Ok(e) => Ok(Some(e)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(MemoryError::StorageError(e.to_string())),
        }
    }

    fn mark_superseded(&self, old_id: &str, new_id: &str) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE memories SET superseded_by = ?1 WHERE id = ?2",
            params![new_id, old_id],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(())
    }

    fn mark_consolidated(&self, a_ids: &[String], b_id: &str) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        for a in a_ids {
            tx.execute(
                "UPDATE memories SET consolidated_into = ?1 WHERE id = ?2",
                params![b_id, a],
            )
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        }
        tx.commit()
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(())
    }

    fn record_recall(&self, id: &str, now_ms: u64) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE memories
             SET last_recalled_at = ?1,
                 recall_count = recall_count + 1
             WHERE id = ?2",
            params![now_ms as i64, id],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 헬퍼
// ---------------------------------------------------------------------------

fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<MemoryEntry> {
    let ep: Option<f64> = row.get(3)?;
    let ea: Option<f64> = row.get(4)?;
    let ed: Option<f64> = row.get(5)?;
    let emotional_context = match (ep, ea, ed) {
        (Some(p), Some(a), Some(d)) => Some((p as f32, a as f32, d as f32)),
        _ => None,
    };

    let type_str: String = row.get(8)?;
    let memory_type =
        MemoryType::from_persisted(&type_str).unwrap_or(MemoryType::DialogueTurn);

    let ts: i64 = row.get(6)?;
    let eid: i64 = row.get(7)?;

    // v2 컬럼 — column index 9부터 (0-based)
    let id: String = row.get(0)?;
    let npc_id: String = row.get(1)?;
    let content: String = row.get(2)?;
    let scope_kind: String = row.get(9).unwrap_or_else(|_| "personal".into());
    let owner_a: Option<String> = row.get(10).ok();
    let owner_b: Option<String> = row.get(11).ok();
    let source_str: String = row.get(12).unwrap_or_else(|_| "experienced".into());
    let provenance_str: String = row.get(13).unwrap_or_else(|_| "runtime".into());
    let layer_str: String = row.get(14).unwrap_or_else(|_| "a".into());
    let topic: Option<String> = row.get(15).ok();
    let origin_chain_json: Option<String> = row.get(16).ok();
    let confidence: f64 = row.get(17).unwrap_or(1.0);
    let acquired_by: Option<String> = row.get(18).ok();
    let created_seq: i64 = row.get(19).unwrap_or(0);
    let last_recalled_at: Option<i64> = row.get(20).ok();
    let recall_count: i64 = row.get(21).unwrap_or(0);
    let superseded_by: Option<String> = row.get(22).ok();
    let consolidated_into: Option<String> = row.get(23).ok();

    let scope = scope_from_columns(
        &scope_kind,
        owner_a.as_deref(),
        owner_b.as_deref(),
        &npc_id,
    );
    let source = source_from_persisted(&source_str);
    let provenance = provenance_from_persisted(&provenance_str);
    let layer = layer_from_persisted(&layer_str);
    let origin_chain: Vec<String> = origin_chain_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    #[allow(deprecated)]
    Ok(MemoryEntry {
        id,
        created_seq: created_seq as u64,
        event_id: eid as u64,
        scope,
        source,
        provenance,
        memory_type,
        layer,
        content,
        topic,
        emotional_context,
        timestamp_ms: ts as u64,
        last_recalled_at: last_recalled_at.map(|v| v as u64),
        recall_count: recall_count as u32,
        origin_chain,
        confidence: confidence as f32,
        acquired_by,
        superseded_by,
        consolidated_into,
        npc_id,
    })
}

// ---------------------------------------------------------------------------
// VO ↔ SQL 직렬화 helper
// ---------------------------------------------------------------------------

fn source_as_persisted(s: MemorySource) -> &'static str {
    match s {
        MemorySource::Experienced => "experienced",
        MemorySource::Witnessed => "witnessed",
        MemorySource::Heard => "heard",
        MemorySource::Rumor => "rumor",
    }
}

fn source_from_persisted(s: &str) -> MemorySource {
    match s {
        "experienced" => MemorySource::Experienced,
        "witnessed" => MemorySource::Witnessed,
        "heard" => MemorySource::Heard,
        "rumor" => MemorySource::Rumor,
        _ => MemorySource::Experienced,
    }
}

fn provenance_as_persisted(p: Provenance) -> &'static str {
    match p {
        Provenance::Seeded => "seeded",
        Provenance::Runtime => "runtime",
    }
}

fn provenance_from_persisted(s: &str) -> Provenance {
    match s {
        "seeded" => Provenance::Seeded,
        _ => Provenance::Runtime,
    }
}

fn layer_as_persisted(l: MemoryLayer) -> &'static str {
    match l {
        MemoryLayer::A => "a",
        MemoryLayer::B => "b",
    }
}

fn layer_from_persisted(s: &str) -> MemoryLayer {
    match s {
        "b" => MemoryLayer::B,
        _ => MemoryLayer::A,
    }
}

fn scope_from_columns(
    kind: &str,
    owner_a: Option<&str>,
    owner_b: Option<&str>,
    fallback_npc_id: &str,
) -> MemoryScope {
    match kind {
        "personal" => MemoryScope::Personal {
            npc_id: owner_a
                .unwrap_or(fallback_npc_id)
                .to_string(),
        },
        "relationship" => MemoryScope::Relationship {
            a: owner_a.unwrap_or("").to_string(),
            b: owner_b.unwrap_or("").to_string(),
        },
        "faction" => MemoryScope::Faction {
            faction_id: owner_a.unwrap_or("").to_string(),
        },
        "family" => MemoryScope::Family {
            family_id: owner_a.unwrap_or("").to_string(),
        },
        "world" => MemoryScope::World {
            world_id: owner_a.unwrap_or("").to_string(),
        },
        _ => MemoryScope::Personal {
            npc_id: fallback_npc_id.to_string(),
        },
    }
}

fn load_entry(conn: &Connection, id: &str) -> Result<MemoryEntry, MemoryError> {
    conn.query_row(
        "SELECT * FROM memories WHERE id = ?1",
        params![id],
        row_to_entry,
    )
    .map_err(|e| MemoryError::StorageError(e.to_string()))
}
