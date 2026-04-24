//! Memory framing formatter — Source별 라벨과 header/footer로 "떠오르는 기억" 블록 생성.
//!
//! Step B — `DialogueOrchestrator.inject_memory_push`가 이 formatter를 사용해 MemoryRanker 결과를
//! 시스템 프롬프트 prepend용 텍스트 블록으로 변환한다.

use std::collections::HashMap;

use serde::Deserialize;

use crate::domain::memory::{MemoryEntry, MemorySource};
use crate::ports::MemoryFramer;
use crate::presentation::builtin_toml;

// ---------------------------------------------------------------------------
// TOML 스키마 — `[memory.framing]` 섹션만 파싱
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct FramingRoot {
    memory: FramingMemory,
}

#[derive(Debug, Deserialize)]
struct FramingMemory {
    framing: FramingLabels,
}

#[derive(Debug, Deserialize)]
struct FramingLabels {
    experienced: String,
    witnessed: String,
    heard: String,
    rumor: String,
    block: FramingBlock,
}

#[derive(Debug, Deserialize)]
struct FramingBlock {
    header: String,
    footer: String,
}

// ---------------------------------------------------------------------------
// LocaleMemoryFramer — 다국어 memory framing 구현
// ---------------------------------------------------------------------------

/// locale(ko/en) 별로 `[memory.framing]` TOML 섹션을 미리 로드해 두고
/// `MemoryEntry → prompt block` 포맷팅을 수행.
///
/// 지원 locale은 `builtin_toml`이 아는 것들 + 외부에서 `with_locale`로 추가 주입한 것.
pub struct LocaleMemoryFramer {
    labels: HashMap<String, FramingLabels>,
    /// locale 조회 실패 시 사용할 기본 locale (초기값 "ko").
    default_locale: String,
}

impl LocaleMemoryFramer {
    /// 내장 locale(ko/en) 기반 인스턴스 생성. 내장 TOML에 `[memory.framing]` 섹션이
    /// 없으면 해당 locale은 등록하지 않고 fallback(raw content) 경로로 동작.
    pub fn new() -> Self {
        let mut labels: HashMap<String, FramingLabels> = HashMap::new();
        for code in ["ko", "en"] {
            if let Some(toml_str) = builtin_toml(code) {
                match toml::from_str::<FramingRoot>(toml_str) {
                    Ok(root) => {
                        labels.insert(code.to_string(), root.memory.framing);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "LocaleMemoryFramer: locale '{}' TOML에 [memory.framing] 파싱 실패 ({}) — raw content fallback",
                            code,
                            e
                        );
                    }
                }
            }
        }
        Self {
            labels,
            default_locale: "ko".to_string(),
        }
    }

    /// 외부 TOML 문자열로 locale 추가 (테스트·커스텀 로케일용).
    pub fn with_locale_toml(mut self, locale: impl Into<String>, toml_str: &str) -> Self {
        if let Ok(root) = toml::from_str::<FramingRoot>(toml_str) {
            self.labels.insert(locale.into(), root.memory.framing);
        }
        self
    }

    /// fallback locale 변경 (조회 실패 시 사용).
    pub fn with_default_locale(mut self, locale: impl Into<String>) -> Self {
        self.default_locale = locale.into();
        self
    }

    /// locale 조회 — 없으면 default로, default도 없으면 None.
    fn lookup_labels(&self, locale: &str) -> Option<&FramingLabels> {
        self.labels
            .get(locale)
            .or_else(|| self.labels.get(&self.default_locale))
    }
}

impl Default for LocaleMemoryFramer {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryFramer for LocaleMemoryFramer {
    fn frame(&self, entry: &MemoryEntry, locale: &str) -> String {
        let Some(labels) = self.lookup_labels(locale) else {
            return entry.content.clone();
        };
        let template = match entry.source {
            MemorySource::Experienced => &labels.experienced,
            MemorySource::Witnessed => &labels.witnessed,
            MemorySource::Heard => &labels.heard,
            MemorySource::Rumor => &labels.rumor,
        };
        template.replace("{content}", &entry.content)
    }

    fn frame_block(&self, entries: &[MemoryEntry], locale: &str) -> String {
        if entries.is_empty() {
            return String::new();
        }
        let Some(labels) = self.lookup_labels(locale) else {
            // fallback: entries만 줄바꿈으로 이어붙임
            return entries
                .iter()
                .map(|e| e.content.clone())
                .collect::<Vec<_>>()
                .join("\n");
        };
        let lines: Vec<String> = entries.iter().map(|e| self.frame(e, locale)).collect();
        format!("{}{}{}", labels.block.header, lines.join("\n"), labels.block.footer)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::memory::{MemoryEntry, MemorySource, MemoryType};

    fn entry_with(source: MemorySource, content: &str) -> MemoryEntry {
        let mut e = MemoryEntry::personal(
            "m1",
            "npc1",
            content,
            None,
            1000,
            1,
            MemoryType::DialogueTurn,
        );
        e.source = source;
        e
    }

    #[test]
    fn framer_experienced_ko_label() {
        let f = LocaleMemoryFramer::new();
        let out = f.frame(&entry_with(MemorySource::Experienced, "사부와 약속했다"), "ko");
        assert_eq!(out, "[겪음] 사부와 약속했다");
    }

    #[test]
    fn framer_source_variants_ko() {
        let f = LocaleMemoryFramer::new();
        for (src, prefix) in [
            (MemorySource::Experienced, "[겪음]"),
            (MemorySource::Witnessed, "[목격]"),
            (MemorySource::Heard, "[전해 들음]"),
            (MemorySource::Rumor, "[강호에 떠도는 소문]"),
        ] {
            let out = f.frame(&entry_with(src, "x"), "ko");
            assert!(
                out.starts_with(prefix),
                "source {:?} 라벨 누락: {}",
                src,
                out
            );
        }
    }

    #[test]
    fn framer_source_variants_en() {
        let f = LocaleMemoryFramer::new();
        for (src, prefix) in [
            (MemorySource::Experienced, "[Experienced]"),
            (MemorySource::Witnessed, "[Witnessed]"),
            (MemorySource::Heard, "[Heard]"),
            (MemorySource::Rumor, "[Rumor]"),
        ] {
            let out = f.frame(&entry_with(src, "x"), "en");
            assert!(
                out.starts_with(prefix),
                "en source {:?} 라벨 누락: {}",
                src,
                out
            );
        }
    }

    #[test]
    fn framer_unknown_locale_falls_back_to_default() {
        let f = LocaleMemoryFramer::new();
        // 미지원 locale ("fr") → 기본값(ko)로 fallback
        let out = f.frame(&entry_with(MemorySource::Experienced, "테스트"), "fr");
        assert_eq!(out, "[겪음] 테스트");
    }

    #[test]
    fn framer_block_empty_returns_empty() {
        let f = LocaleMemoryFramer::new();
        assert_eq!(f.frame_block(&[], "ko"), "");
    }

    #[test]
    fn framer_block_assembles_header_entries_footer() {
        let f = LocaleMemoryFramer::new();
        let entries = vec![
            entry_with(MemorySource::Experienced, "첫 기억"),
            entry_with(MemorySource::Heard, "둘째 기억"),
        ];
        let out = f.frame_block(&entries, "ko");
        // header(ko): "\n# 떠오르는 기억\n"
        assert!(out.contains("# 떠오르는 기억"));
        assert!(out.contains("[겪음] 첫 기억"));
        assert!(out.contains("[전해 들음] 둘째 기억"));
    }

    #[test]
    fn framer_no_locale_loaded_falls_back_to_raw_content() {
        let f = LocaleMemoryFramer {
            labels: HashMap::new(),
            default_locale: "xx".into(),
        };
        let e = entry_with(MemorySource::Experienced, "raw");
        assert_eq!(f.frame(&e, "ko"), "raw");
        assert_eq!(
            f.frame_block(&[e.clone(), e.clone()], "ko"),
            "raw\nraw"
        );
    }
}
