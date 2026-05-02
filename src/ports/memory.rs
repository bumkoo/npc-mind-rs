use crate::domain::memory::{MemoryEntry, MemoryLayer, MemoryResult, MemoryScope, MemorySource};
use crate::domain::rumor::{ReachPolicy, Rumor};

/// Scope 기반 검색 필터
#[derive(Debug, Clone)]
pub enum MemoryScopeFilter {
    Any,
    Exact(MemoryScope),
    /// 이 NPC가 접근 가능한 모든 scope.
    NpcAllowed(String),
}

/// Ranker 이전 단계에서 `MemoryStore`에 넘길 질의 DTO.
#[derive(Debug, Clone, Default)]
pub struct MemoryQuery {
    pub text: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub scope_filter: Option<MemoryScopeFilter>,
    pub source_filter: Option<Vec<MemorySource>>,
    pub layer_filter: Option<MemoryLayer>,
    pub topic: Option<String>,
    pub exclude_superseded: bool,
    pub exclude_consolidated_source: bool,
    pub min_retention: Option<f32>,
    pub current_pad: Option<(f32, f32, f32)>,
    pub limit: usize,
}

/// 기억 저장/검색 포트 — RAG 인덱스 추상화
pub trait MemoryStore: Send + Sync {
    fn index(&self, entry: MemoryEntry, embedding: Option<Vec<f32>>) -> Result<(), MemoryError>;

    #[deprecated(since = "0.4.0", note = "Use MemoryStore::search(MemoryQuery { embedding: Some(..), .. })")]
    fn search_by_meaning(
        &self,
        query_embedding: &[f32],
        npc_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryResult>, MemoryError>;

    #[deprecated(since = "0.4.0", note = "Use MemoryStore::search(MemoryQuery { text: Some(..), .. })")]
    fn search_by_keyword(
        &self,
        keyword: &str,
        npc_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryResult>, MemoryError>;

    #[deprecated(since = "0.4.0", note = "Use MemoryStore::search(MemoryQuery { scope_filter: Some(NpcAllowed(..)), .. })")]
    fn get_recent(
        &self,
        npc_id: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, MemoryError>;

    fn count(&self) -> usize;

    fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryResult>, MemoryError>;
    fn get_by_id(&self, id: &str) -> Result<Option<MemoryEntry>, MemoryError>;
    fn get_by_topic_latest(&self, topic: &str) -> Result<Option<MemoryEntry>, MemoryError>;
    fn get_canonical_by_topic(&self, topic: &str) -> Result<Option<MemoryEntry>, MemoryError>;
    fn mark_superseded(&self, old_id: &str, new_id: &str) -> Result<(), MemoryError>;
    fn mark_consolidated(&self, a_ids: &[String], b_id: &str) -> Result<(), MemoryError>;
    fn record_recall(&self, id: &str, now_ms: u64) -> Result<(), MemoryError>;
    fn clear_all(&self) -> Result<(), MemoryError>;
}

/// 기억 저장소 오류
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("기억 저장소 오류: {0}")]
    StorageError(String),
    #[error("임베딩 오류: {0}")]
    EmbeddingError(String),
}

/// 소문 애그리거트 저장/검색 포트.
pub trait RumorStore: Send + Sync {
    fn save(&self, rumor: &Rumor) -> Result<(), MemoryError>;
    fn load(&self, id: &str) -> Result<Option<Rumor>, MemoryError>;
    fn find_by_topic(&self, topic: &str) -> Result<Vec<Rumor>, MemoryError>;
    fn find_active_in_reach(&self, reach: &ReachPolicy) -> Result<Vec<Rumor>, MemoryError>;
    fn list_all(&self) -> Result<Vec<Rumor>, MemoryError>;
    fn clear_all(&self) -> Result<(), MemoryError>;
}

/// 기억 프레이밍 포트 (LLM 프롬프트 주입용).
pub trait MemoryFramer: Send + Sync {
    fn frame(&self, entry: &MemoryEntry, locale: &str) -> String;
    fn frame_block(&self, entries: &[MemoryEntry], locale: &str) -> String;
}
