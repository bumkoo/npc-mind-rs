//! 분류기 프로토타입 — 공통 로딩·관리
//!
//! Sign/Magnitude 양 분류기가 공유하는 프로토타입 구조.
//! TOML 스키마:
//!
//! ```toml
//! [meta]
//! language = "ko"
//! register = "wuxia"
//! version = "2"
//! group = "sign_keep"  # 또는 magnitude_weak 등
//! last_updated = "2026-04-19"
//!
//! [prototypes]
//! items = [
//!     { text = "...", subtype = "gratitude", source = "created_by_bekay" },
//! ]
//! ```
//!
//! 설계: `docs/emotion/sign-classifier-design.md` §3.3, §4.3
//!       `docs/emotion/phase7-converter-integration.md` §3

use super::types::ListenerPerspectiveError;
use serde::Deserialize;
use std::fs;
use std::path::Path;

// ============================================================
// TOML 스키마 (I/O 경계, private)
// ============================================================

#[derive(Debug, Deserialize)]
struct PrototypeFile {
    meta: PrototypeMetaDto,
    prototypes: PrototypeSection,
}

#[derive(Debug, Deserialize)]
struct PrototypeMetaDto {
    version: String,
    group: String,
    #[serde(default)]
    #[allow(dead_code)]
    language: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    register: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    last_updated: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PrototypeSection {
    items: Vec<PrototypeDto>,
}

#[derive(Debug, Deserialize)]
struct PrototypeDto {
    text: String,
    subtype: String,
    #[serde(default)]
    source: Option<String>,
}

// ============================================================
// 공개 도메인 타입
// ============================================================

/// 단일 프로토타입 항목
#[derive(Debug, Clone)]
pub struct Prototype {
    pub text: String,
    pub subtype: String,
    pub source: Option<String>,
}

/// 단일 그룹 프로토타입 세트 (예: sign_keep, magnitude_strong)
#[derive(Debug, Clone)]
pub struct PrototypeSet {
    pub group: String,
    pub version: String,
    pub items: Vec<Prototype>,
}

impl PrototypeSet {
    /// 프로토타입 개수
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// 비어있는지
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 텍스트만 추출 (임베딩 계산용)
    pub fn texts(&self) -> Vec<&str> {
        self.items.iter().map(|p| p.text.as_str()).collect()
    }
}

// ============================================================
// 로딩 API
// ============================================================

/// 프로토타입 파일 로드 (group 검증 포함)
///
/// `expected_group`: TOML meta.group 이 일치해야 함 (e.g. "sign_keep")
pub fn load_prototypes_from_path<P: AsRef<Path>>(
    path: P,
    expected_group: &str,
) -> Result<PrototypeSet, ListenerPerspectiveError> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).map_err(|e| {
        ListenerPerspectiveError::PrototypeIo(format!("{}: {}", path.display(), e))
    })?;
    load_prototypes_from_toml(&content, expected_group)
}

/// TOML 문자열에서 로드
pub fn load_prototypes_from_toml(
    content: &str,
    expected_group: &str,
) -> Result<PrototypeSet, ListenerPerspectiveError> {
    let parsed: PrototypeFile = toml::from_str(content)
        .map_err(|e| ListenerPerspectiveError::PrototypeParse(e.to_string()))?;

    if parsed.meta.group != expected_group {
        return Err(ListenerPerspectiveError::PrototypeGroupMismatch {
            expected: expected_group.to_string(),
            actual: parsed.meta.group,
        });
    }

    if parsed.prototypes.items.is_empty() {
        return Err(ListenerPerspectiveError::EmptyPrototypes(
            parsed.meta.group,
        ));
    }

    let items = parsed
        .prototypes
        .items
        .into_iter()
        .map(|p| Prototype {
            text: p.text,
            subtype: p.subtype,
            source: p.source,
        })
        .collect();

    Ok(PrototypeSet {
        group: parsed.meta.group,
        version: parsed.meta.version,
        items,
    })
}

// ============================================================
// 단위 테스트
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[meta]
language = "ko"
register = "wuxia"
version = "2"
group = "sign_keep"
last_updated = "2026-04-18"

[prototypes]
items = [
    { text = "이 신세, 언제고 갚겠소이다.", subtype = "gratitude", source = "created_by_bekay" },
    { text = "과연 뛰어나시오.", subtype = "praise" },
]
"#;

    #[test]
    fn load_valid_prototypes() {
        let set = load_prototypes_from_toml(SAMPLE, "sign_keep").unwrap();
        assert_eq!(set.group, "sign_keep");
        assert_eq!(set.version, "2");
        assert_eq!(set.len(), 2);
        assert_eq!(set.items[0].subtype, "gratitude");
        assert_eq!(set.items[0].source.as_deref(), Some("created_by_bekay"));
        assert_eq!(set.items[1].source, None);
    }

    #[test]
    fn texts_extraction() {
        let set = load_prototypes_from_toml(SAMPLE, "sign_keep").unwrap();
        let texts = set.texts();
        assert_eq!(texts.len(), 2);
        assert_eq!(texts[0], "이 신세, 언제고 갚겠소이다.");
    }

    #[test]
    fn rejects_group_mismatch() {
        let err = load_prototypes_from_toml(SAMPLE, "sign_invert").unwrap_err();
        assert!(matches!(
            err,
            ListenerPerspectiveError::PrototypeGroupMismatch { .. }
        ));
    }

    #[test]
    fn rejects_empty_items() {
        let empty = r#"
[meta]
version = "1"
group = "sign_keep"
[prototypes]
items = []
"#;
        let err = load_prototypes_from_toml(empty, "sign_keep").unwrap_err();
        assert!(matches!(err, ListenerPerspectiveError::EmptyPrototypes(_)));
    }

    #[test]
    fn rejects_malformed_toml() {
        let bad = r#"
[meta
version = "1"
"#;
        let err = load_prototypes_from_toml(bad, "sign_keep").unwrap_err();
        assert!(matches!(err, ListenerPerspectiveError::PrototypeParse(_)));
    }
}
