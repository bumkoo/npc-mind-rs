use crate::domain::guide::ActingGuide;
use crate::domain::pad::{CachedPadEmbeddings, PadAnchorSet};

/// 연기 가이드 포맷터 포트 — 가이드를 특정 형식으로 변환
pub trait GuideFormatter: Send + Sync {
    fn format_prompt(&self, guide: &ActingGuide) -> String;
    fn format_json(&self, guide: &ActingGuide) -> Result<String, serde_json::Error>;
}

/// PAD 앵커 로딩 포트 — 포맷 무관 앵커 소스
pub trait PadAnchorSource {
    fn load_anchors(&self) -> Result<PadAnchorSet, AnchorLoadError>;
    fn load_cached_embeddings(&self) -> Result<Option<CachedPadEmbeddings>, AnchorLoadError>;
    fn save_cached_embeddings(
        &self,
        embeddings: &CachedPadEmbeddings,
    ) -> Result<(), AnchorLoadError>;
}

/// 앵커 로딩 오류
#[derive(Debug, thiserror::Error)]
pub enum AnchorLoadError {
    #[error("앵커 파싱 실패: {0}")]
    ParseError(String),
    #[error("앵커 I/O 실패: {0}")]
    IoError(String),
    #[error("앵커 데이터 검증 실패: {0}")]
    ValidationError(String),
}
