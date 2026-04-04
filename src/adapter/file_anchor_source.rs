//! 파일 기반 PAD 앵커 소스 어댑터 (JSON/TOML 지원)
//!
//! `PadAnchorSource` 포트의 구현체.
//! 빌트인(`include_str!`) 또는 런타임 파일 로드를 모두 지원하며,
//! 파일 확장자나 명시적 포맷 지정을 통해 JSON/TOML을 처리한다.

use std::path::{Path, PathBuf};

use crate::domain::pad::CachedPadEmbeddings;
use crate::ports::{AnchorLoadError, PadAnchorSource};

use super::anchor_common::{self, AnchorRaw, AnchorSourceBase};

/// 지원하는 앵커 파일 포맷
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorFormat {
    Json,
    Toml,
}

/// 파일 기반 앵커 소스
pub struct FileAnchorSource {
    base: AnchorSourceBase,
    format: AnchorFormat,
}

impl FileAnchorSource {
    /// 문자열과 포맷에서 생성
    pub fn from_content(content: &str, format: AnchorFormat) -> Self {
        Self {
            base: AnchorSourceBase::from_content(content),
            format,
        }
    }

    /// 파일 경로에서 생성 (확장자로 포맷 자동 판별)
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, AnchorLoadError> {
        let path_ref = path.as_ref();
        let format = match path_ref.extension().and_then(|s| s.to_str()) {
            Some("json") => AnchorFormat::Json,
            Some("toml") => AnchorFormat::Toml,
            _ => return Err(AnchorLoadError::ValidationError(format!("지원하지 않는 앵커 파일 확장자: {:?}", path_ref))),
        };

        Ok(Self {
            base: AnchorSourceBase::from_file(path_ref)?,
            format,
        })
    }

    /// 임베딩 캐시 파일 경로 설정
    pub fn with_cache_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.base = self.base.with_cache_path(path);
        self
    }
}

impl PadAnchorSource for FileAnchorSource {
    fn load_anchors(&self) -> Result<crate::domain::pad::PadAnchorSet, AnchorLoadError> {
        let parsed: AnchorRaw = match self.format {
            AnchorFormat::Json => serde_json::from_str(&self.base.content)
                .map_err(|e| AnchorLoadError::ParseError(e.to_string()))?,
            AnchorFormat::Toml => toml::from_str(&self.base.content)
                .map_err(|e| AnchorLoadError::ParseError(e.to_string()))?,
        };
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
