//! JSON 기반 PAD 앵커 소스 어댑터
//!
//! `PadAnchorSource` 포트의 JSON 구현.
//! TOML과 동일한 구조를 JSON으로 제공.

use std::path::{Path, PathBuf};

use crate::domain::pad::CachedPadEmbeddings;
use crate::ports::{AnchorLoadError, PadAnchorSource};

use super::anchor_common::{self, AnchorRaw, AnchorSourceBase};

// ---------------------------------------------------------------------------
// JsonAnchorSource
// ---------------------------------------------------------------------------

/// JSON 기반 앵커 소스
pub struct JsonAnchorSource {
    base: AnchorSourceBase,
}

impl JsonAnchorSource {
    /// 문자열에서 생성
    pub fn from_content(content: &str) -> Self {
        Self {
            base: AnchorSourceBase::from_content(content),
        }
    }

    /// 런타임 파일 경로에서 로드
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, AnchorLoadError> {
        Ok(Self {
            base: AnchorSourceBase::from_file(path)?,
        })
    }

    /// 임베딩 캐시 파일 경로 설정
    pub fn with_cache_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.base = self.base.with_cache_path(path);
        self
    }
}

impl PadAnchorSource for JsonAnchorSource {
    fn load_anchors(&self) -> Result<crate::domain::pad::PadAnchorSet, AnchorLoadError> {
        let parsed: AnchorRaw = serde_json::from_str(&self.base.content)
            .map_err(|e| AnchorLoadError::ParseError(e.to_string()))?;
        Ok(parsed.into_anchor_set())
    }

    fn load_cached_embeddings(&self) -> Result<Option<CachedPadEmbeddings>, AnchorLoadError> {
        anchor_common::load_cached_embeddings(&self.base.cache_path)
    }

    fn save_cached_embeddings(
        &self,
        embeddings: &CachedPadEmbeddings,
    ) -> Result<(), AnchorLoadError> {
        anchor_common::save_cached_embeddings(&self.base.cache_path, embeddings)
    }
}
