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
    pub fn new() -> Self {
        let ko_toml = builtin_toml("ko").expect("빌트인 ko.toml이 등록되어 있어야 합니다");
        let inner = LocaleFormatter::from_toml(ko_toml)
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
