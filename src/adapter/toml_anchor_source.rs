//! TOML 기반 PAD 앵커 소스 어댑터
//!
//! `PadAnchorSource` 포트의 기본 구현.
//! 빌트인(`include_str!`) 또는 런타임 파일 로드를 모두 지원한다.
//! 임베딩 캐시는 별도 JSON 파일(`.embeddings.json`)로 관리.

use std::path::{Path, PathBuf};

use crate::domain::pad::CachedPadEmbeddings;
use crate::ports::{AnchorLoadError, PadAnchorSource};

use super::anchor_common::{self, AnchorRaw, AnchorSourceBase};

// ---------------------------------------------------------------------------
// TomlAnchorSource
// ---------------------------------------------------------------------------

/// TOML 기반 앵커 소스
///
/// - `from_str()`: `include_str!` 등 컴파일 타임 문자열에서 생성
/// - `from_file()`: 런타임 파일 경로에서 로드
/// - `with_cache_path()`: 임베딩 캐시 파일 경로 설정
pub struct TomlAnchorSource {
    base: AnchorSourceBase,
}

impl TomlAnchorSource {
    /// 컴파일 타임 문자열에서 생성 (`include_str!` 용)
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

impl PadAnchorSource for TomlAnchorSource {
    fn load_anchors(&self) -> Result<crate::domain::pad::PadAnchorSet, AnchorLoadError> {
        let parsed: AnchorRaw = toml::from_str(&self.base.content)
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
