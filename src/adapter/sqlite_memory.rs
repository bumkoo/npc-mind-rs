//! SqliteMemoryStore — SQLite FTS5 + 벡터 BLOB 기반 기억 저장소
//!
//! 메타데이터 + 전문 검색(FTS5) + 벡터 저장을 하나의 SQLite DB로 처리.
//! 10K 미만 규모에서 brute-force cosine이 sub-ms 성능.

use crate::domain::memory::{MemoryEntry, MemoryResult, MemoryType};
use crate::ports::{MemoryError, MemoryStore};
use rusqlite::{params, Connection};
use std::sync::Mutex;

/// SQLite 기반 기억 저장소
pub struct SqliteMemoryStore {
    conn: Mutex<Connection>,
}

impl SqliteMemoryStore {
    /// 파일 기반 저장소 생성
    pub fn new(path: &str) -> Result<Self, MemoryError> {
        let conn = Connection::open(path)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let store = Self { conn: Mutex::new(conn) };
        store.init_tables()?;
        Ok(store)
    }

    /// 인메모리 저장소 생성 (테스트용)
    pub fn in_memory() -> Result<Self, MemoryError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let store = Self { conn: Mutex::new(conn) };
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
                USING fts5(id, content);

            CREATE TABLE IF NOT EXISTS vectors (
                id TEXT PRIMARY KEY,
                embedding BLOB NOT NULL
            );"
        ).map_err(|e| MemoryError::StorageError(e.to_string()))?;
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

        // FTS5 인덱스에 직접 삽입
        conn.execute(
            "INSERT OR REPLACE INTO memories_fts (id, content) VALUES (?1, ?2)",
            params![entry.id, entry.content],
        ).map_err(|e| MemoryError::StorageError(e.to_string()))?;

        if let Some(emb) = embedding {
            let blob = floats_to_blob(&emb);
            conn.execute(
                "INSERT OR REPLACE INTO vectors (id, embedding) VALUES (?1, ?2)",
                params![entry.id, blob],
            ).map_err(|e| MemoryError::StorageError(e.to_string()))?;
        }

        Ok(())
    }

    fn search_by_meaning(
        &self,
        query_embedding: &[f32],
        npc_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryResult>, MemoryError> {
        let conn = self.conn.lock().unwrap();

        // 모든 벡터 로드 + cosine 계산 (brute-force, 10K 이하에서 충분)
        let mut stmt = conn
            .prepare("SELECT v.id, v.embedding FROM vectors v JOIN memories m ON v.id = m.id WHERE (?1 IS NULL OR m.npc_id = ?1)")
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        let mut scored: Vec<(String, f32)> = stmt
            .query_map(params![npc_id], |row| {
                let id: String = row.get(0)?;
                let blob: Vec<u8> = row.get(1)?;
                let emb = blob_to_floats(&blob);
                let score = cosine_sim(query_embedding, &emb);
                Ok((id, score))
            })
            .map_err(|e| MemoryError::StorageError(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored.truncate(limit);

        let results = scored
            .into_iter()
            .filter_map(|(id, score)| {
                let entry = load_entry(&conn, &id).ok()?;
                Some(MemoryResult {
                    entry,
                    relevance_score: score,
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
            // LIKE fallback (CJK/유니코드 키워드)
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

        // npc_id 필터
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

fn floats_to_blob(floats: &[f32]) -> Vec<u8> {
    floats
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}

fn blob_to_floats(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}
