//! Lore 저장소 — `LoreStore` trait + `SqliteLoreStore` 구현 (embed feature).
//!
//! 스키마는 `SqliteMemoryStore` 패턴을 그대로 미러링한다:
//! - `lore_chunks`      : 일반 테이블 (메타 + 원문 TEXT)
//! - `lore_chunks_fts`  : FTS5 가상 테이블 (trigram 토크나이저)
//! - `lore_chunks_vec`  : sqlite-vec vec0 (FLOAT[1024], 코사인)
//! 세 테이블은 `chunk_id`로 조인.

#[cfg(feature = "embed")]
use super::query::Chunk;
use super::query::{ChunkContext, SearchHit, SearchQuery};

/// 저장된 청크 레코드 — `LoreStore::upsert_batch`에 들어가는 입력 단위.
#[derive(Debug, Clone)]
pub struct ChunkRecord {
    pub chunk_id: String,
    pub corpus_id: String,
    pub edition_id: String,
    pub language: String,
    pub text: String,
    pub chapter_index: Option<u32>,
    pub chapter_title: Option<String>,
    pub char_offset_in_edition: u64,
    pub char_offset_in_chapter: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum LoreError {
    #[error("저장소 오류: {0}")]
    Storage(String),
    #[error("임베딩 차원 불일치: 기대 {expected}, 입력 {actual}")]
    DimMismatch { expected: usize, actual: usize },
    #[error("기능 미활성: {0}")]
    FeatureDisabled(&'static str),
}

/// Lore 저장소 포트 — RAG 인덱싱·검색 추상화.
///
/// `embed` feature가 꺼져 있어도 trait 자체는 컴파일된다. 구현체는 embed gate.
pub trait LoreStore: Send + Sync {
    /// 청크 + 임베딩을 batch로 upsert. 같은 `chunk_id`는 덮어쓴다.
    /// 임베딩 길이가 0이면 vec0 등록 생략(텍스트·FTS만).
    fn upsert_batch(
        &self,
        chunks: &[ChunkRecord],
        embeddings: &[Vec<f32>],
    ) -> Result<(), LoreError>;

    /// 의미 기반 검색 — query 임베딩으로 vec0 ANN.
    fn search(
        &self,
        query_embedding: &[f32],
        params: &SearchQuery,
    ) -> Result<Vec<SearchHit>, LoreError>;

    /// 단일 청크 + 같은 edition 내 인접 청크 조회 (`chapter_index` 우선).
    fn get_chunk(
        &self,
        chunk_id: &str,
        before: u32,
        after: u32,
    ) -> Result<Option<ChunkContext>, LoreError>;

    /// edition별 인덱싱된 청크 수 — 진행률·상태 확인용.
    fn count_chunks(&self, edition_id: &str) -> Result<u64, LoreError>;

    /// `list_corpora` 보조 — DB에 인덱싱된 corpus_id별 청크 수 카운트.
    /// (corpus_id, total_chunks)
    fn corpus_chunk_counts(&self) -> Result<Vec<(String, u64)>, LoreError>;
}

#[cfg(feature = "embed")]
pub use sqlite_impl::SqliteLoreStore;

#[cfg(feature = "embed")]
mod sqlite_impl {
    use super::*;
    use rusqlite::ffi::{
        sqlite3, sqlite3_api_routines, sqlite3_auto_extension,
    };
    use rusqlite::{params, Connection};
    use sqlite_vec::sqlite3_vec_init;
    use std::sync::{Mutex, Once};
    use zerocopy::AsBytes;

    /// bge-m3 dense 차원.
    pub const DEFAULT_LORE_EMBEDDING_DIM: usize = 1024;

    const SCHEMA_VERSION: i64 = 1;

    static VEC_INIT: Once = Once::new();

    type SqliteExtensionInit = unsafe extern "C" fn(
        *mut sqlite3,
        *mut *mut i8,
        *const sqlite3_api_routines,
    ) -> i32;

    fn ensure_vec_extension_loaded() {
        VEC_INIT.call_once(|| unsafe {
            let init: SqliteExtensionInit =
                std::mem::transmute(sqlite3_vec_init as *const ());
            sqlite3_auto_extension(Some(init));
        });
    }

    /// SQLite 기반 Lore 저장소.
    pub struct SqliteLoreStore {
        conn: Mutex<Connection>,
        dim: usize,
    }

    impl SqliteLoreStore {
        pub fn new(path: &str) -> Result<Self, LoreError> {
            Self::with_dim(path, DEFAULT_LORE_EMBEDDING_DIM)
        }

        pub fn with_dim(path: &str, dim: usize) -> Result<Self, LoreError> {
            ensure_vec_extension_loaded();
            let conn = Connection::open(path)
                .map_err(|e| LoreError::Storage(e.to_string()))?;
            let store = Self {
                conn: Mutex::new(conn),
                dim,
            };
            store.init_tables()?;
            Ok(store)
        }

        pub fn in_memory() -> Result<Self, LoreError> {
            Self::in_memory_with_dim(DEFAULT_LORE_EMBEDDING_DIM)
        }

        pub fn in_memory_with_dim(dim: usize) -> Result<Self, LoreError> {
            ensure_vec_extension_loaded();
            let conn = Connection::open_in_memory()
                .map_err(|e| LoreError::Storage(e.to_string()))?;
            let store = Self {
                conn: Mutex::new(conn),
                dim,
            };
            store.init_tables()?;
            Ok(store)
        }

        pub fn dim(&self) -> usize {
            self.dim
        }

        fn init_tables(&self) -> Result<(), LoreError> {
            let conn = self.conn.lock().unwrap();
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS lore_schema_meta (version INTEGER PRIMARY KEY)",
            )
            .map_err(|e| LoreError::Storage(e.to_string()))?;

            let current: i64 = conn
                .query_row(
                    "SELECT COALESCE(MAX(version), 0) FROM lore_schema_meta",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);

            if current < 1 {
                Self::migrate_v1(&conn, self.dim)?;
            }

            conn.execute(
                "INSERT OR REPLACE INTO lore_schema_meta(version) VALUES (?)",
                [SCHEMA_VERSION],
            )
            .map_err(|e| LoreError::Storage(e.to_string()))?;
            Ok(())
        }

        fn migrate_v1(conn: &Connection, dim: usize) -> Result<(), LoreError> {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS lore_chunks (
                    chunk_id TEXT PRIMARY KEY,
                    corpus_id TEXT NOT NULL,
                    edition_id TEXT NOT NULL,
                    language TEXT NOT NULL,
                    text TEXT NOT NULL,
                    chapter_index INTEGER,
                    chapter_title TEXT,
                    char_offset_in_edition INTEGER NOT NULL,
                    char_offset_in_chapter INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_lore_edition ON lore_chunks(edition_id, char_offset_in_edition);
                CREATE INDEX IF NOT EXISTS idx_lore_corpus ON lore_chunks(corpus_id);
                CREATE INDEX IF NOT EXISTS idx_lore_chapter ON lore_chunks(edition_id, chapter_index, char_offset_in_chapter);
                CREATE VIRTUAL TABLE IF NOT EXISTS lore_chunks_fts
                    USING fts5(chunk_id, text, tokenize='trigram');",
            )
            .map_err(|e| LoreError::Storage(e.to_string()))?;

            let vec_ddl = format!(
                "CREATE VIRTUAL TABLE IF NOT EXISTS lore_chunks_vec USING vec0(
                    chunk_id TEXT PRIMARY KEY,
                    edition_id TEXT partition key,
                    embedding FLOAT[{dim}] distance_metric=cosine
                );"
            );
            conn.execute_batch(&vec_ddl)
                .map_err(|e| LoreError::Storage(e.to_string()))?;
            Ok(())
        }
    }

    impl LoreStore for SqliteLoreStore {
        fn upsert_batch(
            &self,
            chunks: &[ChunkRecord],
            embeddings: &[Vec<f32>],
        ) -> Result<(), LoreError> {
            if !embeddings.is_empty() && embeddings.len() != chunks.len() {
                return Err(LoreError::Storage(format!(
                    "embeddings 개수 {} != chunks {}",
                    embeddings.len(),
                    chunks.len()
                )));
            }
            for (i, e) in embeddings.iter().enumerate() {
                if e.len() != self.dim {
                    return Err(LoreError::DimMismatch {
                        expected: self.dim,
                        actual: e.len(),
                    });
                }
                let _ = i;
            }

            let mut conn = self.conn.lock().unwrap();
            let tx = conn
                .transaction()
                .map_err(|e| LoreError::Storage(e.to_string()))?;

            for (i, c) in chunks.iter().enumerate() {
                tx.execute(
                    "INSERT OR REPLACE INTO lore_chunks
                     (chunk_id, corpus_id, edition_id, language, text,
                      chapter_index, chapter_title,
                      char_offset_in_edition, char_offset_in_chapter)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        c.chunk_id,
                        c.corpus_id,
                        c.edition_id,
                        c.language,
                        c.text,
                        c.chapter_index.map(|v| v as i64),
                        c.chapter_title,
                        c.char_offset_in_edition as i64,
                        c.char_offset_in_chapter as i64,
                    ],
                )
                .map_err(|e| LoreError::Storage(e.to_string()))?;

                tx.execute(
                    "DELETE FROM lore_chunks_fts WHERE chunk_id = ?1",
                    params![c.chunk_id],
                )
                .map_err(|e| LoreError::Storage(e.to_string()))?;
                tx.execute(
                    "INSERT INTO lore_chunks_fts (chunk_id, text) VALUES (?1, ?2)",
                    params![c.chunk_id, c.text],
                )
                .map_err(|e| LoreError::Storage(e.to_string()))?;

                if let Some(emb) = embeddings.get(i) {
                    tx.execute(
                        "DELETE FROM lore_chunks_vec WHERE chunk_id = ?1",
                        params![c.chunk_id],
                    )
                    .map_err(|e| LoreError::Storage(e.to_string()))?;
                    tx.execute(
                        "INSERT INTO lore_chunks_vec (chunk_id, edition_id, embedding)
                         VALUES (?1, ?2, ?3)",
                        params![c.chunk_id, c.edition_id, emb.as_bytes()],
                    )
                    .map_err(|e| LoreError::Storage(e.to_string()))?;
                }
            }

            tx.commit()
                .map_err(|e| LoreError::Storage(e.to_string()))?;
            Ok(())
        }

        fn search(
            &self,
            query_embedding: &[f32],
            params_in: &SearchQuery,
        ) -> Result<Vec<SearchHit>, LoreError> {
            if query_embedding.len() != self.dim {
                return Err(LoreError::DimMismatch {
                    expected: self.dim,
                    actual: query_embedding.len(),
                });
            }
            let conn = self.conn.lock().unwrap();
            let top_k = params_in.top_k.max(1) as i64;

            // edition_filter가 있으면 vec0 partition_key로 한정 — vec0는 IN을 지원하지
            // 않으므로 edition마다 별도 쿼리 후 merge. corpus_filter는 후처리.
            let edition_filters: Vec<String> = if let Some(eds) = &params_in.edition_filter {
                eds.clone()
            } else {
                vec![String::new()] // 전체
            };

            let oversample = top_k * 4;
            let mut all_hits: Vec<(String, f32)> = Vec::new();
            for ed in &edition_filters {
                let sql = if ed.is_empty() {
                    "SELECT chunk_id, distance FROM lore_chunks_vec
                     WHERE embedding MATCH ?1 AND k = ?2
                     ORDER BY distance"
                } else {
                    "SELECT chunk_id, distance FROM lore_chunks_vec
                     WHERE embedding MATCH ?1 AND k = ?2 AND edition_id = ?3
                     ORDER BY distance"
                };
                let mut stmt = conn
                    .prepare(sql)
                    .map_err(|e| LoreError::Storage(e.to_string()))?;
                let rows: Vec<(String, f32)> = if ed.is_empty() {
                    stmt.query_map(params![query_embedding.as_bytes(), oversample], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)? as f32))
                    })
                    .map_err(|e| LoreError::Storage(e.to_string()))?
                    .filter_map(|r| r.ok())
                    .collect()
                } else {
                    stmt.query_map(
                        params![query_embedding.as_bytes(), oversample, ed],
                        |row| {
                            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)? as f32))
                        },
                    )
                    .map_err(|e| LoreError::Storage(e.to_string()))?
                    .filter_map(|r| r.ok())
                    .collect()
                };
                all_hits.extend(rows);
            }

            // distance 오름차순 정렬 + dedup (같은 chunk_id 중복 제거)
            all_hits.sort_by(|a, b| {
                a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
            });
            let mut seen = std::collections::HashSet::new();
            all_hits.retain(|(id, _)| seen.insert(id.clone()));

            // chunk_id로 메타 조회 + corpus_filter 후처리
            let corpus_set: Option<std::collections::HashSet<String>> =
                params_in.corpus_filter.as_ref().map(|v| v.iter().cloned().collect());
            let mut results: Vec<SearchHit> = Vec::new();
            for (chunk_id, distance) in all_hits {
                if results.len() >= top_k as usize {
                    break;
                }
                let row = conn
                    .query_row(
                        "SELECT corpus_id, edition_id, language, text, chapter_index, chapter_title
                         FROM lore_chunks WHERE chunk_id = ?1",
                        params![chunk_id],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, String>(2)?,
                                row.get::<_, String>(3)?,
                                row.get::<_, Option<i64>>(4)?,
                                row.get::<_, Option<String>>(5)?,
                            ))
                        },
                    )
                    .ok();
                if let Some((corpus_id, edition_id, language, text, ch_idx, ch_title)) = row {
                    if let Some(set) = &corpus_set
                        && !set.contains(&corpus_id)
                    {
                        continue;
                    }
                    // SearchHit.score 정규화 (Phase 0 cleanup 검증):
                    //   sqlite-vec `distance_metric=cosine`은 cosine distance = 1 - cos_sim을
                    //   반환하며 (입력 벡터 정규화 여부와 무관하게 sqlite-vec 내부에서 norm
                    //   분모로 나눠 계산), 범위는 [0, 2]이다. 따라서 `1.0 - distance`는
                    //   cosine similarity 그 자체이며 [-1, 1] 범위.
                    //   체크포인트 2 정성 평가: cross-lingual KO↔ZH 매칭 score 0.45~0.62 관측.
                    //   bge-m3 cross-lingual의 정상 범위(같은 의미 다른 언어 0.4~0.7)와 일치.
                    results.push(SearchHit {
                        corpus_id,
                        edition_id,
                        chunk_id,
                        text,
                        score: 1.0 - distance,
                        language,
                        chapter_index: ch_idx.map(|v| v as u32),
                        chapter_title: ch_title,
                    });
                }
            }
            Ok(results)
        }

        fn get_chunk(
            &self,
            chunk_id: &str,
            before: u32,
            after: u32,
        ) -> Result<Option<ChunkContext>, LoreError> {
            let conn = self.conn.lock().unwrap();
            let focus = match load_chunk(&conn, chunk_id)? {
                Some(c) => c,
                None => return Ok(None),
            };

            // 같은 edition 내 char_offset_in_edition 기준으로 앞/뒤 N개.
            let before_chunks: Vec<Chunk> = conn
                .prepare(
                    "SELECT chunk_id, corpus_id, edition_id, language, text,
                            chapter_index, chapter_title,
                            char_offset_in_edition, char_offset_in_chapter
                     FROM lore_chunks
                     WHERE edition_id = ?1 AND char_offset_in_edition < ?2
                     ORDER BY char_offset_in_edition DESC
                     LIMIT ?3",
                )
                .and_then(|mut stmt| {
                    stmt.query_map(
                        params![
                            focus.edition_id,
                            focus.char_offset_in_edition as i64,
                            before as i64
                        ],
                        row_to_chunk,
                    )
                    .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
                })
                .unwrap_or_default()
                .into_iter()
                .rev()
                .collect();

            let after_chunks: Vec<Chunk> = conn
                .prepare(
                    "SELECT chunk_id, corpus_id, edition_id, language, text,
                            chapter_index, chapter_title,
                            char_offset_in_edition, char_offset_in_chapter
                     FROM lore_chunks
                     WHERE edition_id = ?1 AND char_offset_in_edition > ?2
                     ORDER BY char_offset_in_edition ASC
                     LIMIT ?3",
                )
                .and_then(|mut stmt| {
                    stmt.query_map(
                        params![
                            focus.edition_id,
                            focus.char_offset_in_edition as i64,
                            after as i64
                        ],
                        row_to_chunk,
                    )
                    .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
                })
                .unwrap_or_default();

            Ok(Some(ChunkContext {
                focus,
                before: before_chunks,
                after: after_chunks,
            }))
        }

        fn count_chunks(&self, edition_id: &str) -> Result<u64, LoreError> {
            let conn = self.conn.lock().unwrap();
            let n: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM lore_chunks WHERE edition_id = ?1",
                    params![edition_id],
                    |r| r.get(0),
                )
                .map_err(|e| LoreError::Storage(e.to_string()))?;
            Ok(n.max(0) as u64)
        }

        fn corpus_chunk_counts(&self) -> Result<Vec<(String, u64)>, LoreError> {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT corpus_id, COUNT(*) FROM lore_chunks GROUP BY corpus_id")
                .map_err(|e| LoreError::Storage(e.to_string()))?;
            let rows = stmt
                .query_map([], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?.max(0) as u64))
                })
                .map_err(|e| LoreError::Storage(e.to_string()))?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        }
    }

    fn row_to_chunk(row: &rusqlite::Row) -> rusqlite::Result<Chunk> {
        Ok(Chunk {
            chunk_id: row.get(0)?,
            corpus_id: row.get(1)?,
            edition_id: row.get(2)?,
            language: row.get(3)?,
            text: row.get(4)?,
            chapter_index: row.get::<_, Option<i64>>(5)?.map(|v| v as u32),
            chapter_title: row.get(6)?,
            char_offset_in_edition: row.get::<_, i64>(7)?.max(0) as u64,
            char_offset_in_chapter: row.get::<_, i64>(8)?.max(0) as u64,
        })
    }

    fn load_chunk(conn: &Connection, chunk_id: &str) -> Result<Option<Chunk>, LoreError> {
        let res = conn.query_row(
            "SELECT chunk_id, corpus_id, edition_id, language, text,
                    chapter_index, chapter_title,
                    char_offset_in_edition, char_offset_in_chapter
             FROM lore_chunks WHERE chunk_id = ?1",
            params![chunk_id],
            row_to_chunk,
        );
        match res {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(LoreError::Storage(e.to_string())),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn fake_emb(seed: u8, dim: usize) -> Vec<f32> {
            (0..dim).map(|i| ((seed as usize + i) % 13) as f32 * 0.1).collect()
        }

        #[test]
        fn upsert_search_get_chunk_roundtrip() {
            let dim = 8;
            let store = SqliteLoreStore::in_memory_with_dim(dim).unwrap();
            let chunks = vec![
                ChunkRecord {
                    chunk_id: "c1".into(),
                    corpus_id: "shuihu".into(),
                    edition_id: "ed1".into(),
                    language: "zh".into(),
                    text: "강호의 의리".into(),
                    chapter_index: Some(1),
                    chapter_title: Some("第一回".into()),
                    char_offset_in_edition: 0,
                    char_offset_in_chapter: 0,
                },
                ChunkRecord {
                    chunk_id: "c2".into(),
                    corpus_id: "shuihu".into(),
                    edition_id: "ed1".into(),
                    language: "zh".into(),
                    text: "梁山泊".into(),
                    chapter_index: Some(1),
                    chapter_title: Some("第一回".into()),
                    char_offset_in_edition: 500,
                    char_offset_in_chapter: 500,
                },
            ];
            let embs = vec![fake_emb(1, dim), fake_emb(2, dim)];
            store.upsert_batch(&chunks, &embs).unwrap();
            assert_eq!(store.count_chunks("ed1").unwrap(), 2);

            let q = SearchQuery {
                query: "x".into(),
                top_k: 5,
                corpus_filter: None,
                edition_filter: None,
            };
            let hits = store.search(&fake_emb(1, dim), &q).unwrap();
            assert!(!hits.is_empty());
            assert_eq!(hits[0].chunk_id, "c1");

            let ctx = store.get_chunk("c2", 1, 1).unwrap().unwrap();
            assert_eq!(ctx.focus.chunk_id, "c2");
            assert_eq!(ctx.before.len(), 1);
            assert_eq!(ctx.before[0].chunk_id, "c1");
        }
    }
}
