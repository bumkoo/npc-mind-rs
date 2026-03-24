//! 한국어 연기 가이드 포맷터 — LocaleFormatter + ko.toml 기반
//!
//! `KoreanFormatter`는 `locales/ko.toml`을 내장(include_str!)하여
//! 별도 파일 로드 없이 한국어 포맷을 제공하는 편의 래퍼다.
//!
//! 다른 언어는 `LocaleFormatter::from_toml()`로 직접 생성하면 된다.

use crate::domain::guide::ActingGuide;
use crate::ports::GuideFormatter;
use super::formatter::LocaleFormatter;

/// 내장 한국어 로케일 TOML
const KO_TOML: &str = include_str!("../../locales/ko.toml");

/// 한국어 연기 가이드 포맷터 — ko.toml 내장 래퍼
pub struct KoreanFormatter {
    inner: LocaleFormatter,
}

impl KoreanFormatter {
    /// 새 한국어 포맷터 생성 (ko.toml 내장 로드)
    pub fn new() -> Self {
        let inner = LocaleFormatter::from_toml(KO_TOML)
            .expect("내장 ko.toml 파싱 실패 — TOML 구조 확인 필요");
        Self { inner }
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
