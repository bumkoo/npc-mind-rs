//! Listener-Perspective 정규식 프리필터 (Phase 3)
//!
//! 임베딩(BGE-M3)이 표면 어휘 편향으로 오분류하는 케이스를
//! 카테고리적 규칙으로 덮는다.
//!
//! 파이프라인:
//!   utterance → Prefilter.classify()
//!     Some(hit)  → (sign, magnitude, P_S_default) 직접 반환
//!     None       → 기존 임베딩 경로로 fallback
//!
//! 패턴: data/listener_perspective/prefilter/patterns.toml
//! 설계: docs/emotion/sign-classifier-design.md §3.5 (Phase 3)
//!
//! ## Phase 7 이관 상태 (Step 1 완료)
//!
//! 도메인 정식 구현은 `src/domain/listener_perspective/prefilter.rs` 로 이관됨.
//! 이 파일은 **회귀 감시용 이중 구현**으로 유지된다 —
//! `magnitude_bench.rs`, `sign_classifier_bench.rs` 등 기존 벤치가 feature flag
//! 와 무관하게 계속 통과해야 하기 때문.
//!
//! Phase 7 Step 4 (MindService 통합) 완료 후 이 파일은 제거 예정.

use regex::Regex;
use serde::Deserialize;
use std::fs;

// ============================================================
// 도메인 타입
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sign {
    Keep,
    Invert,
}

impl Sign {
    pub fn from_str(s: &str) -> Sign {
        match s {
            "keep" => Sign::Keep,
            "invert" => Sign::Invert,
            _ => panic!("알 수 없는 sign: {}", s),
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Sign::Keep => "keep",
            Sign::Invert => "invert",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Magnitude {
    Weak,
    Normal,
    Strong,
}

impl Magnitude {
    pub fn from_str(s: &str) -> Magnitude {
        match s {
            "weak" => Magnitude::Weak,
            "normal" => Magnitude::Normal,
            "strong" => Magnitude::Strong,
            _ => panic!("알 수 없는 magnitude: {}", s),
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Magnitude::Weak => "weak",
            Magnitude::Normal => "normal",
            Magnitude::Strong => "strong",
        }
    }
}

/// 프리필터 매칭 결과
#[derive(Debug, Clone)]
pub struct PrefilterHit {
    pub sign: Sign,
    pub magnitude: Magnitude,
    pub p_s_default: f32,
    pub matched_category: String,
    pub matched_pattern: String,
}

// ============================================================
// TOML 스키마
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
// Prefilter 엔진
// ============================================================

/// 컴파일된 카테고리 (런타임용)
struct CompiledCategory {
    name: String,
    sign: Sign,
    magnitude: Magnitude,
    p_s_default: f32,
    patterns: Vec<(String, Regex)>, // (원본 소스, 컴파일)
}

pub struct Prefilter {
    categories: Vec<CompiledCategory>,
}

impl Prefilter {
    /// TOML 파일 경로에서 로드
    pub fn from_path(path: &str) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("패턴 파일 로드 실패 {}: {}", path, e))?;
        Self::from_toml(&content)
    }

    /// TOML 문자열에서 직접 로드
    pub fn from_toml(content: &str) -> Result<Self, String> {
        let parsed: PatternFile = toml::from_str(content)
            .map_err(|e| format!("패턴 파싱 실패: {}", e))?;

        let mut categories = Vec::with_capacity(parsed.categories.len());
        for cat in parsed.categories {
            let mut patterns = Vec::with_capacity(cat.patterns.len());
            for p in &cat.patterns {
                let re = Regex::new(p)
                    .map_err(|e| format!("카테고리 {} 패턴 컴파일 실패 '{}': {}", cat.name, p, e))?;
                patterns.push((p.clone(), re));
            }
            categories.push(CompiledCategory {
                name: cat.name,
                sign: Sign::from_str(&cat.sign),
                magnitude: Magnitude::from_str(&cat.magnitude),
                p_s_default: cat.p_s_default,
                patterns,
            });
        }
        Ok(Self { categories })
    }

    /// 발화 분류. 첫 매칭되는 카테고리 반환 (우선순위 = 등록 순서)
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

    /// 등록된 카테고리 이름 목록 (디버깅용)
    pub fn category_names(&self) -> Vec<&str> {
        self.categories.iter().map(|c| c.name.as_str()).collect()
    }
}
