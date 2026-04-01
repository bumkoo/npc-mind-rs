//! PAD 앵커 소스 공통 모듈
//!
//! `TomlAnchorSource`와 `JsonAnchorSource`가 공유하는
//! 역직렬화 구조체, 변환 로직, 임베딩 캐시 헬퍼를 제공한다.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::domain::pad::{CachedPadEmbeddings, PadAnchorSet, PadAxisAnchorsOwned};
use crate::ports::AnchorLoadError;

// ---------------------------------------------------------------------------
// 공통 역직렬화 구조체
// ---------------------------------------------------------------------------

/// 앵커 파일 최상위 구조 (TOML/JSON 공용)
#[derive(Deserialize)]
pub(crate) struct AnchorRaw {
    /// 스키마 호환용 — 파싱 시 필드 존재 확인만 하고 값은 사용하지 않음
    #[allow(dead_code)]
    pub meta: AnchorMeta,
    pub pleasure: AxisAnchorsRaw,
    pub arousal: AxisAnchorsRaw,
    pub dominance: AxisAnchorsRaw,
}

/// 앵커 메타 정보 (언어, 버전)
#[derive(Deserialize)]
pub(crate) struct AnchorMeta {
    /// 앵커 언어 코드 (예: "ko") — 파싱 검증용, 런타임에서는 미사용
    #[allow(dead_code)]
    pub language: String,
    /// 앵커 데이터 버전 — 하위 호환성 검증용, 런타임에서는 미사용
    #[allow(dead_code)]
    pub version: String,
}

/// 축별 앵커 텍스트 (positive/negative)
#[derive(Deserialize)]
pub(crate) struct AxisAnchorsRaw {
    pub positive: Vec<String>,
    pub negative: Vec<String>,
}

// ---------------------------------------------------------------------------
// 변환
// ---------------------------------------------------------------------------

impl AnchorRaw {
    /// 파싱된 원시 데이터를 도메인 `PadAnchorSet`으로 변환
    pub fn into_anchor_set(self) -> PadAnchorSet {
        PadAnchorSet {
            pleasure: PadAxisAnchorsOwned {
                positive: self.pleasure.positive,
                negative: self.pleasure.negative,
            },
            arousal: PadAxisAnchorsOwned {
                positive: self.arousal.positive,
                negative: self.arousal.negative,
            },
            dominance: PadAxisAnchorsOwned {
                positive: self.dominance.positive,
                negative: self.dominance.negative,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// 임베딩 캐시 헬퍼
// ---------------------------------------------------------------------------

/// 캐시 파일에서 임베딩 로드 (경로 없으면 None)
pub(crate) fn load_cached_embeddings(
    cache_path: &Option<PathBuf>,
) -> Result<Option<CachedPadEmbeddings>, AnchorLoadError> {
    let Some(path) = cache_path else {
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

/// 임베딩을 캐시 파일에 저장 (경로 없으면 no-op)
pub(crate) fn save_cached_embeddings(
    cache_path: &Option<PathBuf>,
    embeddings: &CachedPadEmbeddings,
) -> Result<(), AnchorLoadError> {
    let Some(path) = cache_path else {
        return Ok(());
    };
    let json = serde_json::to_string_pretty(embeddings)
        .map_err(|e| AnchorLoadError::ParseError(e.to_string()))?;
    std::fs::write(path, json).map_err(|e| AnchorLoadError::IoError(e.to_string()))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 공통 빌더 패턴
// ---------------------------------------------------------------------------

/// 앵커 소스 공통 필드 + 유틸리티
pub(crate) struct AnchorSourceBase {
    pub content: String,
    pub cache_path: Option<PathBuf>,
}

impl AnchorSourceBase {
    pub fn from_content(content: &str) -> Self {
        Self {
            content: content.to_owned(),
            cache_path: None,
        }
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, AnchorLoadError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| AnchorLoadError::IoError(e.to_string()))?;
        Ok(Self {
            content,
            cache_path: None,
        })
    }

    pub fn with_cache_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.cache_path = Some(path.into());
        self
    }
}
