//! Lore 검색 DTO — `LoreStore` 트레잇과 MCP 도구가 공유하는 타입.

use serde::{Deserialize, Serialize};

/// `search_lore` 쿼리 파라미터.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    #[serde(default = "default_top_k")]
    pub top_k: u32,
    #[serde(default)]
    pub corpus_filter: Option<Vec<String>>,
    #[serde(default)]
    pub edition_filter: Option<Vec<String>>,
}

fn default_top_k() -> u32 {
    5
}

/// 한 검색 결과.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub corpus_id: String,
    pub edition_id: String,
    pub chunk_id: String,
    pub text: String,
    pub score: f32,
    pub language: String,
    #[serde(default)]
    pub chapter_index: Option<u32>,
    #[serde(default)]
    pub chapter_title: Option<String>,
}

/// `get_chunk` 결과 — 포커스 청크 + 앞뒤 문맥.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkContext {
    pub focus: Chunk,
    pub before: Vec<Chunk>,
    pub after: Vec<Chunk>,
}

/// 직렬화용 청크 — 메타 + 본문.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub chunk_id: String,
    pub corpus_id: String,
    pub edition_id: String,
    pub language: String,
    pub text: String,
    #[serde(default)]
    pub chapter_index: Option<u32>,
    #[serde(default)]
    pub chapter_title: Option<String>,
    pub char_offset_in_edition: u64,
    pub char_offset_in_chapter: u64,
}

/// `list_corpora` 한 항목.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusSummary {
    pub corpus_id: String,
    pub title: String,
    pub author_name: Option<String>,
    pub genre_tags: Vec<String>,
    pub license: Option<String>,
    pub editions: Vec<EditionSummary>,
    /// 인덱싱된 청크 수 (없으면 None — DB가 비었거나 미인덱싱).
    pub indexed_chunks: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditionSummary {
    pub edition_id: String,
    pub language: String,
    pub edition: Option<String>,
}
