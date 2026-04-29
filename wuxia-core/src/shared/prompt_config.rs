// wuxia-core/src/shared/prompt_config.rs
//
// 프롬프트 설정 데이터 타입.
//
// prompt_config.toml의 구조를 Rust 타입으로 정의한다.
// 이 타입은 wuxia-data(로딩)와 wuxia-llm(프롬프트 조립) 모두에서 사용한다.
//
// 의존성 방향:
//   wuxia-core (타입 정의) ← wuxia-data (로딩) + wuxia-llm (사용)
//   → wuxia-data ↔ wuxia-llm 직접 참조 없음.
//
// 비유: 강호 서신의 "양식 규격"
//   문장 템플릿, 기억 포맷을 코드가 아닌 설정으로 관리한다.
//   새 언어 추가 시 TOML만 수정하면 된다.
//
// 아키텍처 원칙:
//   - 번역이 필요한 텍스트만 TOML로 관리한다.
//   - XML 태그명은 코드(template.rs)에서 상수로 관리한다 (단일 원본).
//   → 실제 값은 prompt_config.toml 참조.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// PromptTemplates — 문장 템플릿 (언어별)
// ---------------------------------------------------------------------------

/// 프롬프트 문장 템플릿.
///
/// 하나의 언어에 대한 문장 패턴을 정의한다.
/// `{name}`, `{age}` 등의 플레이스홀더는 빌드 시 `.replace()`로 치환한다.
/// `{tag_*}` 플레이스홀더는 template.rs의 XML 태그 상수로 치환한다.
///
/// 실제 값은 prompt_config.toml `[templates.ko]`, `[templates.en]` 참조.
///
/// # Example (prompt_config.toml)
/// ```toml
/// [templates.ko]
/// identity = "너는 {name}({hanja})이다. 별호는 {alias}({alias_hanja})."
/// directive_1 = "너는 AI가 아니라 무협 세계의 '{name}'이다. ..."
/// # ...
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptTemplates {
    /// 정체성 한 줄. 플레이스홀더: {name}, {hanja}, {alias}, {alias_hanja}
    pub identity: String,
    /// 기본 정보 한 줄. 플레이스홀더: {age}, {gender}, {affiliation}, {role_desc}
    pub basic_info_line: String,
    /// 금기어 접두사. 플레이스홀더: 없음 (뒤에 단어 목록이 붙음)
    pub forbidden_words_prefix: String,
    /// 관계 헤더. 플레이스홀더: {level}
    pub relationship_header_line: String,
    /// 최종 요약 시 남은 대화 라벨. 플레이스홀더: 없음
    pub remaining_dialogue: String,
    /// 지시 1: AI 아님 선언. 플레이스홀더: {name}
    pub directive_1: String,
    /// 지시 2: Persona 준수. 플레이스홀더: {tag_persona}
    pub directive_2: String,
    /// 지시 3: Memory_Bank 반영. 플레이스홀더: {tag_memory}
    pub directive_3: String,
    /// 지시 4: Relationship+Summary 맥락. 플레이스홀더: {tag_relationship}, {tag_summary}
    pub directive_4: String,
    /// 지시 5: JSON Reasoning(CoT) 강제. 지능 향상을 위해 항상 포함.
    pub directive_json: String,
    /// 출력 포맷 예시. 플레이스홀더: {name}
    pub directive_output_example: String,
}

// ---------------------------------------------------------------------------
// MemoryFormat — 기억 포맷 (언어별)
// ---------------------------------------------------------------------------

/// 기억 문자열 포맷.
///
/// 실제 값은 prompt_config.toml `[memory_format.ko]`, `[memory_format.en]` 참조.
///
/// # Example (prompt_config.toml)
/// ```toml
/// [memory_format.ko]
/// today = "오늘"
/// days_ago = "{n}일 전"
/// entry = "({time}) {content}\n  [{label}]"
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryFormat {
    /// 당일 라벨. 플레이스홀더: 없음
    pub today: String,
    /// N일 전 패턴. 플레이스홀더: {n}
    pub days_ago: String,
    /// 기억 한 줄 포맷. 플레이스홀더: {time}, {content}, {label}
    pub entry: String,
}

// ---------------------------------------------------------------------------
// MemoryLabels — 기억 중요도 라벨 (5단계)
// ---------------------------------------------------------------------------

/// 기억 중요도 라벨 5단계 문자열.
///
/// 하나의 언어에 대한 5단계 라벨을 정의한다.
/// TOML에서 `[memory_labels.ko]`, `[memory_labels.en]` 형태로 로딩된다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryLabelSet {
    pub level_1: String,
    pub level_2: String,
    pub level_3: String,
    pub level_4: String,
    pub level_5: String,
}

/// 기억 중요도 라벨 경계값.
///
/// importance 값을 5단계 라벨로 변환하는 기준.
/// TOML에서 `[memory_labels.thresholds]` 형태로 로딩된다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryLabelThresholds {
    pub level_1: f32,
    pub level_2: f32,
    pub level_3: f32,
    pub level_4: f32,
    pub level_5: f32,
}

/// 기억 중요도 라벨 전체 설정.
///
/// 경계값 + 언어별 라벨 문자열.
/// TOML `[memory_labels]` 섹션 전체.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryLabelsConfig {
    pub thresholds: MemoryLabelThresholds,
    #[serde(flatten)]
    pub labels: HashMap<String, MemoryLabelSet>,
}

impl MemoryLabelsConfig {
    /// importance 값을 언어별 라벨 문자열로 변환한다.
    ///
    /// # 반환
    /// - 해당 locale의 라벨 문자열 ("en" fallback)
    /// - locale별 라벨이 없으면 fallback 문자열
    pub fn importance_to_label(&self, importance: f32, locale_code: &str) -> &str {
        let labels = self.labels.get(locale_code)
            .or_else(|| self.labels.get("en"));

        let Some(l) = labels else {
            return "memory";
        };

        let t = &self.thresholds;
        if importance >= t.level_5 {
            &l.level_5
        } else if importance >= t.level_4 {
            &l.level_4
        } else if importance >= t.level_3 {
            &l.level_3
        } else if importance >= t.level_2 {
            &l.level_2
        } else {
            &l.level_1
        }
    }
}

// ---------------------------------------------------------------------------
// PromptConfig — 전체 프롬프트 설정
// ---------------------------------------------------------------------------

/// prompt_config.toml의 전체 구조.
///
/// HashMap 키는 locale code ("ko", "en")와 일치한다.
/// XML 태그명은 template.rs에서 상수로 관리하므로 여기에 포함하지 않는다.
///
/// # 사용 흐름
/// ```text
///   Locale::Ko.code()               → "ko"
///   config.language_directive["ko"]  → "한국어로 대답한다."
///   config.templates["ko"]           → PromptTemplates { ... }
/// ```
///
/// # Example
/// ```
/// use wuxia_core::shared::prompt_config::{
///     PromptConfig, PromptTemplates, MemoryFormat,
///     MemoryLabelsConfig, MemoryLabelThresholds, MemoryLabelSet,
/// };
/// use std::collections::HashMap;
///
/// let mut lang = HashMap::new();
/// lang.insert("ko".to_string(), "d".to_string());
/// let mut templates = HashMap::new();
/// templates.insert("ko".to_string(), PromptTemplates {
///     identity: "t".to_string(), basic_info_line: "t".to_string(),
///     forbidden_words_prefix: "t".to_string(),
///     relationship_header_line: "t".to_string(),
///     remaining_dialogue: "t".to_string(),
///     directive_1: "t".to_string(), directive_2: "t".to_string(),
///     directive_3: "t".to_string(), directive_4: "t".to_string(),
///     directive_json: "t".to_string(), directive_output_example: "t".to_string(),
/// });
/// let mut mf = HashMap::new();
/// mf.insert("ko".to_string(), MemoryFormat {
///     today: "m".to_string(), days_ago: "m".to_string(), entry: "m".to_string(),
/// });
/// let memory_labels = MemoryLabelsConfig {
///     thresholds: MemoryLabelThresholds {
///         level_1: 1.0, level_2: 3.0, level_3: 5.0, level_4: 7.0, level_5: 9.0,
///     },
///     labels: HashMap::new(),
/// };
///
/// let config = PromptConfig {
///     language_directive: lang, templates, memory_format: mf, memory_labels,
///     world_lore: HashMap::new(),
/// };
/// assert!(config.templates_for("ko").is_some());
/// assert!(config.memory_format_for("ko").is_some());
/// ```
/// 세계관 공유 지식 데이터 (언어별).
/// [v4.5] 모든 NPC가 공유하는 배경 설정.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldLore {
    /// 시대 및 거시적 역사 설정.
    pub era_history: Vec<String>,
    /// 국가별 지리 및 특징.
    pub geography: Vec<String>,
    /// 국가 간 역학 관계 매트릭스.
    pub geopolitics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptConfig {
    /// locale code → 언어 지시문. "ko" → "한국어로 대답한다."
    pub language_directive: HashMap<String, String>,
    /// locale code → 문장 템플릿. "ko" → PromptTemplates { ... }
    pub templates: HashMap<String, PromptTemplates>,
    /// locale code → 기억 포맷. "ko" → MemoryFormat { ... }
    pub memory_format: HashMap<String, MemoryFormat>,
    /// 기억 중요도 라벨 설정 (경계값 + 언어별 5단계).
    pub memory_labels: MemoryLabelsConfig,
    /// [v4.5] 세계관 공유 지식 (시대, 지리, 정치).
    #[serde(default)]
    pub world_lore: HashMap<String, WorldLore>,
}

impl PromptConfig {
    /// locale code로 언어 지시문을 조회한다.
    ///
    /// 없으면 "en" fallback을 시도한다.
    pub fn directive_for(&self, locale_code: &str) -> Option<&str> {
        self.language_directive
            .get(locale_code)
            .or_else(|| self.language_directive.get("en"))
            .map(|s| s.as_str())
    }

    /// locale code로 문장 템플릿을 조회한다.
    ///
    /// 없으면 "en" fallback을 시도한다.
    pub fn templates_for(&self, locale_code: &str) -> Option<&PromptTemplates> {
        self.templates
            .get(locale_code)
            .or_else(|| self.templates.get("en"))
    }

    /// locale code로 기억 포맷을 조회한다.
    ///
    /// 없으면 "en" fallback을 시도한다.
    pub fn memory_format_for(&self, locale_code: &str) -> Option<&MemoryFormat> {
        self.memory_format
            .get(locale_code)
            .or_else(|| self.memory_format.get("en"))
    }

    /// locale code로 세계관 설정을 조회한다.
    pub fn world_lore_for(&self, locale_code: &str) -> Option<&WorldLore> {
        self.world_lore
            .get(locale_code)
            .or_else(|| self.world_lore.get("en"))
    }

    /// 기억 중요도 라벨 설정을 반환한다.
    pub fn memory_labels(&self) -> &MemoryLabelsConfig {
        &self.memory_labels
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_templates(prefix: &str) -> PromptTemplates {
        PromptTemplates {
            identity: format!("T-{}", prefix),
            basic_info_line: format!("T-{}", prefix),
            forbidden_words_prefix: format!("T-{}", prefix),
            relationship_header_line: format!("T-{}", prefix),
            remaining_dialogue: format!("T-{}", prefix),
            directive_1: format!("T-{}", prefix),
            directive_2: format!("T-{}", prefix),
            directive_3: format!("T-{}", prefix),
            directive_4: format!("T-{}", prefix),
            directive_json: format!("T-{}", prefix),
            directive_output_example: format!("T-{}", prefix),
        }
    }

    fn make_config() -> PromptConfig {
        let mut lang = HashMap::new();
        lang.insert("ko".to_string(), "D-ko".to_string());
        lang.insert("en".to_string(), "D-en".to_string());

        let mut templates = HashMap::new();
        templates.insert("ko".to_string(), make_templates("ko"));
        templates.insert("en".to_string(), make_templates("en"));

        let mut memory_format = HashMap::new();
        memory_format.insert("ko".to_string(), MemoryFormat {
            today: "M-ko".to_string(), days_ago: "M-ko".to_string(), entry: "M-ko".to_string(),
        });
        memory_format.insert("en".to_string(), MemoryFormat {
            today: "M-en".to_string(), days_ago: "M-en".to_string(), entry: "M-en".to_string(),
        });

        let memory_labels = MemoryLabelsConfig {
            thresholds: MemoryLabelThresholds {
                level_1: 1.0, level_2: 3.0, level_3: 5.0, level_4: 7.0, level_5: 9.0,
            },
            labels: {
                let mut m = HashMap::new();
                m.insert("ko".to_string(), MemoryLabelSet {
                    level_1: "L1-ko".to_string(), level_2: "L2-ko".to_string(),
                    level_3: "L3-ko".to_string(), level_4: "L4-ko".to_string(),
                    level_5: "L5-ko".to_string(),
                });
                m.insert("en".to_string(), MemoryLabelSet {
                    level_1: "L1-en".to_string(), level_2: "L2-en".to_string(),
                    level_3: "L3-en".to_string(), level_4: "L4-en".to_string(),
                    level_5: "L5-en".to_string(),
                });
                m
            },
        };

        PromptConfig { language_directive: lang, templates, memory_format, memory_labels, world_lore: HashMap::new() }
    }

    #[test]
    fn directive_for_ko() {
        let config = make_config();
        assert_eq!(config.directive_for("ko").unwrap(), "D-ko");
    }

    #[test]
    fn directive_for_fallback() {
        let config = make_config();
        assert_eq!(config.directive_for("ja").unwrap(), "D-en");
    }

    #[test]
    fn templates_for_ko() {
        let config = make_config();
        let t = config.templates_for("ko").unwrap();
        assert_eq!(t.identity, "T-ko");
    }

    #[test]
    fn templates_for_fallback() {
        let config = make_config();
        let t = config.templates_for("ja").unwrap();
        assert_eq!(t.identity, "T-en");
    }

    #[test]
    fn memory_format_for_ko() {
        let config = make_config();
        let m = config.memory_format_for("ko").unwrap();
        assert_eq!(m.today, "M-ko");
    }

    #[test]
    fn memory_format_for_fallback() {
        let config = make_config();
        let m = config.memory_format_for("ja").unwrap();
        assert_eq!(m.today, "M-en");
    }

    #[test]
    fn serde_roundtrip() {
        let config = make_config();
        let json = serde_json::to_string(&config).expect("serialize");
        let restored: PromptConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(config, restored);
    }

    // -- memory_labels --

    #[test]
    fn importance_to_label_level_boundaries() {
        let config = make_config();
        let ml = config.memory_labels();
        assert_eq!(ml.importance_to_label(1.0, "ko"), "L1-ko");
        assert_eq!(ml.importance_to_label(2.9, "ko"), "L1-ko");
        assert_eq!(ml.importance_to_label(3.0, "ko"), "L2-ko");
        assert_eq!(ml.importance_to_label(4.9, "ko"), "L2-ko");
        assert_eq!(ml.importance_to_label(5.0, "ko"), "L3-ko");
        assert_eq!(ml.importance_to_label(6.9, "ko"), "L3-ko");
        assert_eq!(ml.importance_to_label(7.0, "ko"), "L4-ko");
        assert_eq!(ml.importance_to_label(8.9, "ko"), "L4-ko");
        assert_eq!(ml.importance_to_label(9.0, "ko"), "L5-ko");
        assert_eq!(ml.importance_to_label(10.0, "ko"), "L5-ko");
    }

    #[test]
    fn importance_to_label_en() {
        let config = make_config();
        let ml = config.memory_labels();
        assert_eq!(ml.importance_to_label(7.5, "en"), "L4-en");
    }

    #[test]
    fn importance_to_label_locale_fallback() {
        let config = make_config();
        let ml = config.memory_labels();
        assert_eq!(ml.importance_to_label(9.5, "ja"), "L5-en");
    }

    #[test]
    fn importance_to_label_below_minimum() {
        let config = make_config();
        let ml = config.memory_labels();
        assert_eq!(ml.importance_to_label(0.5, "ko"), "L1-ko");
    }
}
