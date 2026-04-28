//! Lore RAG — 장르 원전 임베딩·검색 컴포넌트 (Phase 0).
//!
//! - `corpus`  : `data/corpus/manifest.toml` 파싱 (PD 원전 목록).
//! - `query`   : `SearchQuery` / `SearchHit` / `ChunkContext` DTO.
//! - `store`   : `LoreStore` trait + `SqliteLoreStore` (embed feature).
//! - `ingest`  : EPUB → 청킹 → 임베딩 → 저장 파이프라인 (embed feature).
//!
//! 같은 RAG가 두 곳에서 호출됨:
//!   1. 도구 설계 단계의 worldbuilding 결정 검증 (Cowork 세션)
//!   2. 완성된 도구의 AI 협업 기능 (Mind Studio MCP — `search_lore` 등)

pub mod corpus;
pub mod ingest;
pub mod query;
pub mod store;

pub use corpus::{CorpusMeta, Edition, Manifest, ManifestError};
pub use ingest::{ChunkConfig, ChapterText, EpubReader, IngestStats, chunk_chapter, chunk_edition};
pub use query::{Chunk, ChunkContext, CorpusSummary, EditionSummary, SearchHit, SearchQuery};
pub use store::{ChunkRecord, LoreError, LoreStore};

#[cfg(feature = "embed")]
pub use ingest::EpubFileReader;
#[cfg(feature = "embed")]
pub use store::SqliteLoreStore;
