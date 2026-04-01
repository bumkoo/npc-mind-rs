//! JSON 기반 PAD 앵커 소스 어댑터
//!
//! `PadAnchorSource` 포트의 JSON 구현.
//! TOML과 동일한 구조를 JSON으로 제공.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::domain::pad::{PadAnchorSet, PadAxisAnchorsOwned, CachedPadEmbeddings};
use crate::ports::{PadAnchorSource, AnchorLoadError};

// ---------------------------------------------------------------------------
// JSON 역직렬화 구조체 (어댑터 내부용)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct AnchorJson {
    #[allow(dead_code)]
    meta: AnchorMeta,
    pleasure: AxisAnchorsJson,
    arousal: AxisAnchorsJson,
    dominance: AxisAnchorsJson,
}

#[derive(Deserialize)]
struct AnchorMeta {
    #[allow(dead_code)]
    language: String,
    #[allow(dead_code)]
    version: String,
}

#[derive(Deserialize)]
struct AxisAnchorsJson {
    positive: Vec<String>,
    negative: Vec<String>,
}

// ---------------------------------------------------------------------------
// JsonAnchorSource
// ---------------------------------------------------------------------------

/// JSON 기반 앵커 소스
pub struct JsonAnchorSource {
    json_content: String,
    cache_path: Option<PathBuf>,
}

impl JsonAnchorSource {
    /// 문자열에서 생성
    pub fn from_content(content: &str) -> Self {
        Self {
            json_content: content.to_owned(),
            cache_path: None,
        }
    }

    /// 런타임 파일 경로에서 로드
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, AnchorLoadError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| AnchorLoadError::IoError(e.to_string()))?;
        Ok(Self {
            json_content: content,
            cache_path: None,
        })
    }

    /// 임베딩 캐시 파일 경로 설정
    pub fn with_cache_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.cache_path = Some(path.into());
        self
    }
}

impl PadAnchorSource for JsonAnchorSource {
    fn load_anchors(&self) -> Result<PadAnchorSet, AnchorLoadError> {
        let parsed: AnchorJson = serde_json::from_str(&self.json_content)
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
        let content = std::fs::read_to_string(path)
            .map_err(|e| AnchorLoadError::IoError(e.to_string()))?;
        let cached: CachedPadEmbeddings = serde_json::from_str(&content)
            .map_err(|e| AnchorLoadError::ParseError(e.to_string()))?;
        Ok(Some(cached))
    }

    fn save_cached_embeddings(&self, embeddings: &CachedPadEmbeddings) -> Result<(), AnchorLoadError> {
        let Some(ref path) = self.cache_path else {
            return Ok(());
        };
        let json = serde_json::to_string_pretty(embeddings)
            .map_err(|e| AnchorLoadError::ParseError(e.to_string()))?;
        std::fs::write(path, json)
            .map_err(|e| AnchorLoadError::IoError(e.to_string()))?;
        Ok(())
    }
}
