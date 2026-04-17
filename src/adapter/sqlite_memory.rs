//! SqliteMemoryStore — SQLite FTS5(텍스트/키워드) + sqlite-vec vec0(임베딩) 기반 기억 저장소
//!
//! 하나의 SQLite 파일 안에서 세 레이어가 `id`로 조인된다:
//! - `memories`      : 일반 테이블 (메타데이터 + 원문 TEXT)
//! - `memories_fts`  : FTS5 가상 테이블 (키워드 전문 검색)
//! - `memories_vec`  : sqlite-vec `vec0` 가상 테이블 (코사인 ANN)
//!
//! sqlite-vec는 순수 C 확장이라 tokio 런타임을 요구하지 않는다.
//! `embed` feature가 활성화되어도 라이브러리 코어의 runtime-agnostic 원칙은 유지된다.

use crate::domain::memory::{MemoryEntry, MemoryResult, MemoryType};
use crate::ports::{MemoryError, MemoryStore};
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
                USING fts5(id, content);",
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        // vec0는 스키마에 차원을 하드코딩해야 해서 동적 SQL로 생성.
        // id를 TEXT PRIMARY KEY로, npc_id를 partition key로 두어 npc별 검색을 가속.
        let vec_ddl = format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memories_vec USING vec0(
                id TEXT PRIMARY KEY,
                npc_id TEXT partition key,
                embedding FLOAT[{dim}] distance_metric=cosine
            );",
            dim = self.dim,
        );
        conn.execute_batch(&vec_ddl)
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

        conn.execute(
            "INSERT OR REPLACE INTO memories (id, npc_id, content, emotional_p, emotional_a, emotional_d, timestamp_ms, event_id, memory_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                entry.id,
                entry.npc_id,
                entry.content,
                ep,
                ea,
                ed,
                entry.timestamp_ms as i64,
                entry.event_id as i64,
                format!("{:?}", entry.memory_type),
            ],
        ).map_err(|e| MemoryError::StorageError(e.to_string()))?;

        conn.execute(
            "INSERT OR REPLACE INTO memories_fts (id, content) VALUES (?1, ?2)",
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
            // vec0는 INSERT OR REPLACE를 지원하지 않는 경우가 있어 DELETE 후 INSERT.
            conn.execute(
                "DELETE FROM memories_vec WHERE id = ?1",
                params![entry.id],
            )
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
            conn.execute(
                "INSERT INTO memories_vec (id, npc_id, embedding) VALUES (?1, ?2, ?3)",
                params![entry.id, entry.npc_id, emb.as_bytes()],
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

        // sqlite-vec vec0의 KNN 구문:
        //   WHERE embedding MATCH ? AND k = ?
        //   ORDER BY distance
        // npc_id 필터는 partition key로 적용 가능하지만 NULL-or-match 패턴을 그대로 쓴다.
        let sql = "SELECT v.id, v.distance
                   FROM memories_vec v
                   WHERE v.embedding MATCH ?1
                     AND k = ?2
                     AND (?3 IS NULL OR v.npc_id = ?3)
                   ORDER BY v.distance";

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        let rows = stmt
            .query_map(
                params![query_embedding.as_bytes(), limit as i64, npc_id],
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

        // FTS5 검색 시도, 실패 시 LIKE fallback (CJK 토크나이저 부재 대응)
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
            .filter(|e| npc_id.map_or(true, |id| e.npc_id == id))
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
    let memory_type = match type_str.as_str() {
        "Dialogue" => MemoryType::Dialogue,
        "Relationship" => MemoryType::Relationship,
        "BeatTransition" => MemoryType::BeatTransition,
        "SceneEnd" => MemoryType::SceneEnd,
        "GameEvent" => MemoryType::GameEvent,
        _ => MemoryType::Dialogue,
    };

    let ts: i64 = row.get(6)?;
    let eid: i64 = row.get(7)?;

    Ok(MemoryEntry {
        id: row.get(0)?,
        npc_id: row.get(1)?,
        content: row.get(2)?,
        emotional_context,
        timestamp_ms: ts as u64,
        event_id: eid as u64,
        memory_type,
    })
}

fn load_entry(conn: &Connection, id: &str) -> Result<MemoryEntry, MemoryError> {
    conn.query_row(
        "SELECT * FROM memories WHERE id = ?1",
        params![id],
        row_to_entry,
    )
    .map_err(|e| MemoryError::StorageError(e.to_string()))
}
