//! YAML frontmatter 추출 + H2 섹션 파싱.
//!
//! 입력 마크다운은 다음 구조를 가정:
//! ```text
//! ---
//! id: group-x
//! kind: alliance
//! ...
//! ---
//!
//! ## 개요
//! 산문 ...
//!
//! ## 권력 구조
//! 산문 ...
//! ```

use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum FrontmatterError {
    #[error("frontmatter `---` 펜스 누락")]
    MissingFence,
    #[error("frontmatter 닫는 `---` 누락")]
    UnterminatedFence,
    #[error("YAML 파싱 실패: {0}")]
    Yaml(String),
}

/// 파싱된 frontmatter — 본문(`body`)과 별도로 보관.
#[derive(Debug, Clone)]
pub struct Frontmatter {
    /// `---`와 `---` 사이의 raw YAML 본문 (디버깅·재직렬화용).
    pub raw_yaml: String,
    /// `serde_yaml::Value` 트리.
    pub value: serde_yaml::Value,
    /// frontmatter 뒤에 오는 마크다운 본문 (선행 빈 줄 제거 후).
    pub body: String,
}

/// 마크다운 텍스트에서 frontmatter + body 분리.
///
/// 지원 형태:
/// - `---\n<yaml>\n---\n<body>` (LF/CRLF 모두)
/// - 첫 `---`은 파일 첫 줄에 있어야 함 (선행 BOM/공백은 허용).
pub fn parse_frontmatter(md: &str) -> Result<Frontmatter, FrontmatterError> {
    let trimmed = md.trim_start_matches('\u{FEFF}');
    let original_lines: Vec<&str> = trimmed.lines().collect();
    let first = original_lines.first().ok_or(FrontmatterError::MissingFence)?;
    if first.trim() != "---" {
        return Err(FrontmatterError::MissingFence);
    }

    let mut yaml_buf = String::new();
    let mut found_close = false;
    let mut body_start_line: Option<usize> = None;
    for (i, line) in original_lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            found_close = true;
            body_start_line = Some(i + 1);
            break;
        }
        yaml_buf.push_str(line);
        yaml_buf.push('\n');
    }
    if !found_close {
        return Err(FrontmatterError::UnterminatedFence);
    }

    let value: serde_yaml::Value = serde_yaml::from_str(&yaml_buf)
        .map_err(|e| FrontmatterError::Yaml(e.to_string()))?;

    let body = if let Some(start) = body_start_line {
        original_lines[start..].join("\n").trim_start().to_string()
    } else {
        String::new()
    };

    Ok(Frontmatter {
        raw_yaml: yaml_buf,
        value,
        body,
    })
}

/// 마크다운 본문에서 H2 섹션을 추출.
///
/// 규칙:
/// - `## <title>` 라인이 섹션 시작.
/// - `# <title>` (H1)은 무시 (섹션 키 미생성, 본문도 미수집).
/// - H2 다음 라인부터 다음 H2 또는 EOF까지 본문.
/// - 본문은 양끝 빈 줄 trim, 내부 줄바꿈은 보존.
/// - 동일 제목 H2가 여럿이면 마지막 본문이 우선 (덮어쓰기).
/// - **Fenced code block** (``` 또는 ~~~로 둘러싸인 영역) 안의 `## `는 헤더로 인식하지 않음.
///   같은 종류의 펜스(``` ↔ ~~~)에서만 토글되며, 들여쓰기는 무시(0-3 공백 허용).
pub fn parse_h2_sections(body: &str) -> BTreeMap<String, String> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    let mut cur_title: Option<String> = None;
    let mut cur_buf: String = String::new();
    let mut fence: Option<char> = None; // Some('`') | Some('~') | None
    for line in body.lines() {
        let trimmed = line.trim_start();
        // 펜스 토글 검사 — 같은 종류 펜스(``` 또는 ~~~)만 매치.
        // CommonMark는 정확한 길이 매치까지 요구하지만, 본 파서는 단순화: 3+ 연속 백틱/틸드.
        if let Some(c) = fence_char(trimmed) {
            match fence {
                None => fence = Some(c),
                Some(prev) if prev == c => fence = None,
                _ => {} // 다른 종류 펜스는 무시 (열려 있으면 그대로 유지)
            }
            // 펜스 라인 자체는 본문에 포함 (현 섹션이 있을 때만).
            if cur_title.is_some() {
                cur_buf.push_str(line);
                cur_buf.push('\n');
            }
            continue;
        }
        // 펜스 안에서는 헤더 매칭 비활성 — 본문으로 누적.
        if fence.is_some() {
            if cur_title.is_some() {
                cur_buf.push_str(line);
                cur_buf.push('\n');
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("## ") {
            if let Some(title) = cur_title.take() {
                out.insert(title, cur_buf.trim().to_string());
            }
            cur_title = Some(rest.trim().to_string());
            cur_buf = String::new();
        } else if trimmed.starts_with("# ") && !trimmed.starts_with("## ") {
            // H1 — 섹션 분리에 사용하지 않음. 진행 중 섹션이 있으면 닫고 무시.
            if let Some(title) = cur_title.take() {
                out.insert(title, cur_buf.trim().to_string());
            }
            cur_buf = String::new();
        } else if cur_title.is_some() {
            cur_buf.push_str(line);
            cur_buf.push('\n');
        }
    }
    if let Some(title) = cur_title.take() {
        out.insert(title, cur_buf.trim().to_string());
    }
    out
}

/// 라인이 fenced code block 펜스인지(``` / ~~~ 3+ 연속) 검사하고 펜스 문자를 반환.
fn fence_char(trimmed: &str) -> Option<char> {
    for c in ['`', '~'] {
        let n = trimmed.chars().take_while(|&ch| ch == c).count();
        if n >= 3 {
            return Some(c);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// 단위 테스트
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "---\nid: group-test\nkind: alliance\nname: 테스트\n---\n\n## 개요\n첫 단락.\n\n둘째 단락.\n\n## 시간 변화\n메모.\n";

    #[test]
    fn parse_frontmatter_extracts_yaml_and_body() {
        let fm = parse_frontmatter(SAMPLE).unwrap();
        assert_eq!(fm.value["id"].as_str(), Some("group-test"));
        assert_eq!(fm.value["kind"].as_str(), Some("alliance"));
        assert!(fm.body.starts_with("## 개요"));
    }

    #[test]
    fn parse_frontmatter_missing_fence_errs() {
        let md = "id: group-test\nname: x\n";
        assert!(matches!(
            parse_frontmatter(md),
            Err(FrontmatterError::MissingFence)
        ));
    }

    #[test]
    fn parse_frontmatter_unterminated_errs() {
        let md = "---\nid: group-test\nkind: alliance\n";
        assert!(matches!(
            parse_frontmatter(md),
            Err(FrontmatterError::UnterminatedFence)
        ));
    }

    #[test]
    fn parse_frontmatter_invalid_yaml_errs() {
        let md = "---\nid: group-test\n  bad: : :\n---\n";
        assert!(matches!(
            parse_frontmatter(md),
            Err(FrontmatterError::Yaml(_))
        ));
    }

    #[test]
    fn parse_h2_sections_collects_in_order() {
        let fm = parse_frontmatter(SAMPLE).unwrap();
        let secs = parse_h2_sections(&fm.body);
        assert_eq!(secs.len(), 2);
        assert!(secs.contains_key("개요"));
        assert!(secs.contains_key("시간 변화"));
        assert!(secs["개요"].contains("첫 단락"));
        assert!(secs["개요"].contains("둘째 단락"));
        assert_eq!(secs["시간 변화"], "메모.");
    }

    #[test]
    fn parse_h2_sections_ignores_h1() {
        let body = "# 제목\n무시.\n\n## 개요\n본문.";
        let secs = parse_h2_sections(body);
        assert_eq!(secs.len(), 1);
        assert!(secs.contains_key("개요"));
    }

    #[test]
    fn parse_frontmatter_handles_crlf() {
        // CRLF 입력에서 펜스 인식·body 추출이 정상 작동.
        let md = "---\r\nid: group-x\r\nkind: clan\r\nname: test\r\n---\r\n\r\n## Heading\r\nBody.";
        let fm = parse_frontmatter(md).expect("CRLF 파싱");
        assert_eq!(fm.value["id"].as_str(), Some("group-x"));
        assert!(fm.body.starts_with("## Heading"));
    }

    #[test]
    fn parse_h2_sections_ignores_hash_inside_fenced_code_block() {
        // ``` 안의 `## fake` 는 헤더로 인식되지 않아야 함.
        let body = "## Real\nbefore\n```\n## fake header\nstill code\n```\nafter\n\n## Real2\ndone";
        let secs = parse_h2_sections(body);
        assert_eq!(secs.len(), 2);
        assert!(secs.contains_key("Real"));
        assert!(secs.contains_key("Real2"));
        assert!(!secs.contains_key("fake header"));
        assert!(secs["Real"].contains("## fake header"));
        assert!(secs["Real"].contains("```"));
    }

    #[test]
    fn parse_h2_sections_handles_tilde_fence() {
        // ~~~ 펜스도 토글되어야 함.
        let body = "## A\n~~~\n## not a header\n~~~\n## B\nbody";
        let secs = parse_h2_sections(body);
        assert_eq!(secs.len(), 2);
        assert!(secs.contains_key("A"));
        assert!(secs.contains_key("B"));
    }

    #[test]
    fn parse_h2_sections_duplicate_title_keeps_last() {
        let body = "## Same\nfirst\n\n## Same\nsecond";
        let secs = parse_h2_sections(body);
        assert_eq!(secs.len(), 1);
        assert_eq!(secs["Same"], "second");
    }

    #[test]
    fn parse_frontmatter_empty_body_yields_empty_sections() {
        let md = "---\nid: g\nkind: clan\nname: x\n---\n";
        let fm = parse_frontmatter(md).unwrap();
        let secs = parse_h2_sections(&fm.body);
        assert!(secs.is_empty());
    }
}
