//! Listener-perspective 정규식 프리필터 (Phase 3)
//!
//! BGE-M3 임베딩이 표면 어휘 편향으로 오분류하는 케이스를 규칙 기반으로 덮는다.
//!
//! ## 파이프라인
//!
//! ```text
//! utterance → Prefilter::classify()
//!   Some(hit) → (sign, magnitude, p_s_default) 직접 반환
//!   None      → 임베딩 경로로 fallback
//! ```
//!
//! ## 설계 철학
//!
//! - **어미 결합형** — `아니` 단독 매칭 금지, `아니었(으면|더라면)`처럼 결합
//! - **첫 매칭 반환** — 카테고리 등록 순서가 우선순위
//! - **외부화** — 패턴은 TOML 에서 로드, Rust 재컴파일 불필요
//!
//! ## 참고
//!
//! - 설계: `docs/emotion/sign-classifier-design.md` §3.5
//! - 기본 패턴: `data/listener_perspective/prefilter/patterns.toml`

use super::types::{ListenerPerspectiveError, Magnitude, PrefilterHit, Sign};
use regex::Regex;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::str::FromStr;

// ============================================================
// TOML 스키마 (외부 I/O 경계, private)
// ============================================================

#[derive(Debug, Deserialize)]
struct PatternFile {
    #[allow(dead_code)]
    meta: PatternMeta,
    #[serde(rename = "category")]
    categories: Vec<CategoryDef>,
}

#[derive(Debug, Deserialize)]
struct PatternMeta {
    #[allow(dead_code)]
    version: String,
}

#[derive(Debug, Deserialize)]
struct CategoryDef {
    name: String,
    sign: String,
    magnitude: String,
    p_s_default: f32,
    #[allow(dead_code)]
    description: String,
    patterns: Vec<String>,
}

// ============================================================
// 컴파일된 카테고리 (런타임 전용)
// ============================================================

struct CompiledCategory {
    name: String,
    sign: Sign,
    magnitude: Magnitude,
    p_s_default: f32,
    /// (원본 소스 문자열, 컴파일된 정규식)
    patterns: Vec<(String, Regex)>,
}

impl std::fmt::Debug for CompiledCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledCategory")
            .field("name", &self.name)
            .field("sign", &self.sign)
            .field("magnitude", &self.magnitude)
            .field("p_s_default", &self.p_s_default)
            .field("pattern_count", &self.patterns.len())
            .finish()
    }
}

// ============================================================
// 공개 엔진
// ============================================================

/// 정규식 기반 Listener-perspective 프리필터
///
/// TOML 설정에서 카테고리(이름·sign·magnitude·p_s_default·patterns) 목록을 로드하고,
/// `classify(utterance)` 호출 시 첫 매칭 카테고리의 `PrefilterHit` 를 반환한다.
#[derive(Debug)]
pub struct Prefilter {
    categories: Vec<CompiledCategory>,
}

impl Prefilter {
    /// TOML 파일 경로에서 로드
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, ListenerPerspectiveError> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).map_err(|e| {
            ListenerPerspectiveError::PatternIo(format!("{}: {}", path.display(), e))
        })?;
        Self::from_toml(&content)
    }

    /// TOML 문자열에서 직접 로드
    pub fn from_toml(content: &str) -> Result<Self, ListenerPerspectiveError> {
        let parsed: PatternFile = toml::from_str(content)
            .map_err(|e| ListenerPerspectiveError::PatternParse(e.to_string()))?;

        let mut categories = Vec::with_capacity(parsed.categories.len());
        for cat in parsed.categories {
            let sign = Sign::from_str(&cat.sign)?;
            let magnitude = Magnitude::from_str(&cat.magnitude)?;
            let mut patterns = Vec::with_capacity(cat.patterns.len());
            for p in &cat.patterns {
                let re = Regex::new(p).map_err(|e| {
                    ListenerPerspectiveError::PatternCompile {
                        category: cat.name.clone(),
                        pattern: p.clone(),
                        reason: e.to_string(),
                    }
                })?;
                patterns.push((p.clone(), re));
            }
            categories.push(CompiledCategory {
                name: cat.name,
                sign,
                magnitude,
                p_s_default: cat.p_s_default,
                patterns,
            });
        }
        Ok(Self { categories })
    }

    /// 발화 분류. 첫 매칭되는 카테고리 반환 (우선순위 = 등록 순서).
    ///
    /// 매칭 실패 시 `None` — 임베딩 경로로 fallback 하는 것이 호출 측 책임.
    pub fn classify(&self, utterance: &str) -> Option<PrefilterHit> {
        for cat in &self.categories {
            for (src, re) in &cat.patterns {
                if re.is_match(utterance) {
                    return Some(PrefilterHit {
                        sign: cat.sign,
                        magnitude: cat.magnitude,
                        p_s_default: cat.p_s_default,
                        matched_category: cat.name.clone(),
                        matched_pattern: src.clone(),
                    });
                }
            }
        }
        None
    }

    /// 등록된 카테고리 이름 목록 (디버깅·로그용)
    pub fn category_names(&self) -> Vec<&str> {
        self.categories.iter().map(|c| c.name.as_str()).collect()
    }

    /// 등록된 카테고리 수
    pub fn category_count(&self) -> usize {
        self.categories.len()
    }
}

// ============================================================
// 단위 테스트 (도메인 자체 건전성)
//
// 통합 벤치는 tests/prefilter_unit.rs + tests/magnitude_bench.rs 에서 병행 유지.
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TOML: &str = r#"
[meta]
version = "test"

[[category]]
name = "counterfactual_gratitude"
sign = "keep"
magnitude = "strong"
p_s_default = 0.7
description = "반사실 감사"
patterns = [
    "아니었(으면|더라면)",
]

[[category]]
name = "sarcasm_interjection"
sign = "invert"
magnitude = "strong"
p_s_default = 0.6
description = "감탄사 빈정"
patterns = [
    "^(허허|아이고|아이구)",
]
"#;

    #[test]
    fn from_toml_parses_categories() {
        let pf = Prefilter::from_toml(SAMPLE_TOML).unwrap();
        assert_eq!(pf.category_count(), 2);
        assert_eq!(
            pf.category_names(),
            vec!["counterfactual_gratitude", "sarcasm_interjection"]
        );
    }

    #[test]
    fn classify_hits_counterfactual_gratitude() {
        let pf = Prefilter::from_toml(SAMPLE_TOML).unwrap();
        let hit = pf.classify("그대 아니었으면 이 몸은 이미 이 세상 사람이 아니었으리").unwrap();
        assert_eq!(hit.matched_category, "counterfactual_gratitude");
        assert_eq!(hit.sign, Sign::Keep);
        assert_eq!(hit.magnitude, Magnitude::Strong);
        assert!((hit.p_s_default - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn classify_hits_sarcasm_interjection() {
        let pf = Prefilter::from_toml(SAMPLE_TOML).unwrap();
        let hit = pf.classify("아이고, 훌륭하기도 하셔라.").unwrap();
        assert_eq!(hit.matched_category, "sarcasm_interjection");
        assert_eq!(hit.sign, Sign::Invert);
    }

    #[test]
    fn classify_misses_plain_greeting() {
        let pf = Prefilter::from_toml(SAMPLE_TOML).unwrap();
        assert!(pf.classify("오늘 날이 좋구려.").is_none());
    }

    #[test]
    fn from_toml_rejects_invalid_sign() {
        let bad = r#"
[meta]
version = "test"

[[category]]
name = "x"
sign = "flip"
magnitude = "strong"
p_s_default = 0.5
description = "bad"
patterns = ["foo"]
"#;
        let err = Prefilter::from_toml(bad).unwrap_err();
        assert!(matches!(err, ListenerPerspectiveError::InvalidSign(_)));
    }

    #[test]
    fn from_toml_rejects_bad_regex() {
        let bad = r#"
[meta]
version = "test"

[[category]]
name = "x"
sign = "keep"
magnitude = "strong"
p_s_default = 0.5
description = "bad"
patterns = ["[unclosed"]
"#;
        let err = Prefilter::from_toml(bad).unwrap_err();
        assert!(matches!(
            err,
            ListenerPerspectiveError::PatternCompile { .. }
        ));
    }
}
