//! 한국어 연기 가이드 포맷터 — LocaleFormatter + ko.toml 기반
//!
//! `KoreanFormatter`는 빌트인 한국어 로케일을 사용하는 편의 래퍼다.
//! 다른 언어나 커스텀 로케일은 `FormattedMindService`를 통해 사용하세요.

use super::builtin_toml;
use super::formatter::LocaleFormatter;
use crate::domain::guide::ActingGuide;
use crate::ports::GuideFormatter;

/// 한국어 연기 가이드 포맷터 — ko.toml 내장 래퍼
pub struct KoreanFormatter {
    inner: LocaleFormatter,
}

impl KoreanFormatter {
    /// 새 한국어 포맷터 생성 (빌트인 ko.toml 로드)
    ///
    /// 리소스 로드에 실패하면 빈 포맷터를 반환하고 표준 에러에 경고를 출력합니다.
    /// 명시적 에러 처리가 필요하면 [`KoreanFormatter::try_new`]를 사용하세요.
    pub fn new() -> Self {
        Self::try_new().unwrap_or_else(|e| {
            eprintln!("Warning: KoreanFormatter initialization failed: {}", e);
            // 최소한의 기본 구조로 생성 (빈 문자열 파싱은 항상 성공함)
            let inner = LocaleFormatter::from_toml("").unwrap();
            Self { inner }
        })
    }

    /// 한국어 포맷터 생성을 시도합니다.
    pub fn try_new() -> Result<Self, String> {
        let ko_toml = builtin_toml("ko").ok_or_else(|| "Built-in ko.toml not found".to_string())?;
        let inner = LocaleFormatter::from_toml(ko_toml).map_err(|e| e.to_string())?;
        Ok(Self { inner })
    }
}

impl Default for KoreanFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl GuideFormatter for KoreanFormatter {
    fn format_prompt(&self, guide: &ActingGuide) -> String {
        self.inner.format_prompt(guide)
    }

    fn format_json(&self, guide: &ActingGuide) -> Result<String, serde_json::Error> {
        self.inner.format_json(guide)
    }
}
