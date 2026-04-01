//! TOML 기반 PAD 앵커 소스 어댑터
//!
//! `PadAnchorSource` 포트의 기본 구현.
//! 빌트인(`include_str!`) 또는 런타임 파일 로드를 모두 지원한다.
//! 임베딩 캐시는 별도 JSON 파일(`.embeddings.json`)로 관리.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::domain::pad::{CachedPadEmbeddings, PadAnchorSet, PadAxisAnchorsOwned};
use crate::ports::{AnchorLoadError, PadAnchorSource};

// ---------------------------------------------------------------------------
// TOML 역직렬화 구조체 (어댑터 내부용)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct AnchorToml {
    #[allow(dead_code)]
    meta: AnchorMeta,
    pleasure: AxisAnchorsToml,
    arousal: AxisAnchorsToml,
    dominance: AxisAnchorsToml,
}

#[derive(Deserialize)]
struct AnchorMeta {
    #[allow(dead_code)]
    language: String,
    #[allow(dead_code)]
    version: String,
}

#[derive(Deserialize)]
struct AxisAnchorsToml {
    positive: Vec<String>,
    negative: Vec<String>,
}

// ---------------------------------------------------------------------------
// TomlAnchorSource
// ---------------------------------------------------------------------------

/// TOML 기반 앵커 소스
///
/// - `from_str()`: `include_str!` 등 컴파일 타임 문자열에서 생성
/// - `from_file()`: 런타임 파일 경로에서 로드
/// - `with_cache_path()`: 임베딩 캐시 파일 경로 설정
pub struct TomlAnchorSource {
    toml_content: String,
    cache_path: Option<PathBuf>,
}

impl TomlAnchorSource {
    /// 컴파일 타임 문자열에서 생성 (`include_str!` 용)
    pub fn from_content(content: &str) -> Self {
        Self {
            toml_content: content.to_owned(),
            cache_path: None,
        }
    }

    /// 런타임 파일 경로에서 로드
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, AnchorLoadError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| AnchorLoadError::IoError(e.to_string()))?;
        Ok(Self {
            toml_content: content,
            cache_path: None,
        })
    }

    /// 임베딩 캐시 파일 경로 설정
    pub fn with_cache_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.cache_path = Some(path.into());
        self
    }
}

impl PadAnchorSource for TomlAnchorSource {
    fn load_anchors(&self) -> Result<PadAnchorSet, AnchorLoadError> {
        let parsed: AnchorToml = toml::from_str(&self.toml_content)
            .map_err(|e| AnchorLoadError::ParseError(e.to_string()))?;

        Ok(PadAnchorSet {
            pleasure: PadAxisAnchorsOwned {
                positive: parsed.pleasure.positive,
                negative: parsed.pleasure.negative,
            },
            arousal: PadAxisAnchorsOwned {
                positive: parsed.arousal.positive,
                negative: parsed.arousal.negative,
            },
            dominance: PadAxisAnchorsOwned {
                positive: parsed.dominance.positive,
                negative: parsed.dominance.negative,
            },
        })
    }

    fn load_cached_embeddings(&self) -> Result<Option<CachedPadEmbeddings>, AnchorLoadError> {
        let Some(ref path) = self.cache_path else {
            return Ok(None);
        };
        if !path.exists() {
            return Ok(None);
        }
        let content =
            std::fs::read_to_string(path).map_err(|e| AnchorLoadError::IoError(e.to_string()))?;
        let cached: CachedPadEmbeddings = serde_json::from_str(&content)
            .map_err(|e| AnchorLoadError::ParseError(e.to_string()))?;
        Ok(Some(cached))
    }

    fn save_cached_embeddings(
        &self,
        embeddings: &CachedPadEmbeddings,
    ) -> Result<(), AnchorLoadError> {
        let Some(ref path) = self.cache_path else {
            return Ok(());
        };
        let json = serde_json::to_string_pretty(embeddings)
            .map_err(|e| AnchorLoadError::ParseError(e.to_string()))?;
        std::fs::write(path, json).map_err(|e| AnchorLoadError::IoError(e.to_string()))?;
        Ok(())
    }
}
