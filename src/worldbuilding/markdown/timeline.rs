//! Timeline 마크다운 → `Timeline` 애그리거트 변환.
//!
//! 입력 frontmatter 스키마는 `docs/tasks/task-phase5b-era-timeline-vertical-slice.md` §6를 따름.
//!
//! 변환 정책:
//! - `kind`/`name`/`id`는 frontmatter 필수
//! - `references`는 EraId 배열 — Phase 5b 외래키 활성 (world-load 검증)
//! - **R4 strict typing 패턴 (Phase 5a·5b 일관)**: references 시퀀스의 non-String 항목은 hard error

use std::collections::BTreeMap;

use serde_json::{Map, Value as JsonValue};
use serde_yaml::Value as YamlValue;

use crate::domain::world::{EraId, Timeline, TimelineId};

use super::frontmatter::{FrontmatterError, parse_frontmatter, parse_h2_sections};

#[derive(Debug, thiserror::Error)]
pub enum TimelineMarkdownError {
    #[error(transparent)]
    Frontmatter(#[from] FrontmatterError),
    #[error("필수 필드 누락: {0}")]
    MissingField(&'static str),
    #[error("필드 '{field}' 타입 불일치 (기대: {expected})")]
    TypeMismatch {
        field: &'static str,
        expected: &'static str,
    },
}

pub fn timeline_from_markdown(md: &str) -> Result<Timeline, TimelineMarkdownError> {
    let fm = parse_frontmatter(md)?;
    let map = fm
        .value
        .as_mapping()
        .ok_or(TimelineMarkdownError::MissingField("frontmatter (mapping)"))?;

    let id = get_str(map, "id")
        .ok_or(TimelineMarkdownError::MissingField("id"))?
        .to_string();
    let kind = get_str(map, "kind")
        .ok_or(TimelineMarkdownError::MissingField("kind"))?
        .to_string();
    let name = get_str(map, "name")
        .ok_or(TimelineMarkdownError::MissingField("name"))?
        .to_string();

    let aliases = get_string_array(map, "aliases").unwrap_or_default();
    let summary = get_str(map, "summary")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let tags = get_string_array(map, "tags").unwrap_or_default();
    let extras = parse_extras_map(map.get("extras"));
    // R4 strict — references는 외래키 활성 EraId 시퀀스.
    let references = get_string_array_strict(map, "references", "references")?
        .into_iter()
        .map(EraId::new)
        .collect();
    let body_sections: BTreeMap<String, String> = parse_h2_sections(&fm.body);

    Ok(Timeline {
        id: TimelineId::new(id),
        kind,
        name,
        aliases,
        summary,
        tags,
        extras,
        references,
        body_sections,
        source_path: None,
    })
}

fn get_str<'a>(map: &'a serde_yaml::Mapping, key: &str) -> Option<&'a str> {
    map.get(YamlValue::from(key)).and_then(|v| match v {
        YamlValue::String(s) => Some(s.as_str()),
        YamlValue::Null => None,
        _ => None,
    })
}

fn get_string_array(map: &serde_yaml::Mapping, key: &str) -> Option<Vec<String>> {
    let v = map.get(YamlValue::from(key))?;
    match v {
        YamlValue::Sequence(seq) => Some(
            seq.iter()
                .filter_map(|item| match item {
                    YamlValue::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect(),
        ),
        YamlValue::Null => Some(Vec::new()),
        _ => None,
    }
}

fn get_string_array_strict(
    map: &serde_yaml::Mapping,
    key: &str,
    field: &'static str,
) -> Result<Vec<String>, TimelineMarkdownError> {
    let Some(v) = map.get(YamlValue::from(key)) else {
        return Ok(Vec::new());
    };
    match v {
        YamlValue::Sequence(seq) => {
            let mut out = Vec::with_capacity(seq.len());
            for item in seq {
                match item {
                    YamlValue::String(s) => out.push(s.clone()),
                    _ => {
                        return Err(TimelineMarkdownError::TypeMismatch {
                            field,
                            expected: "string item (외래키 ID)",
                        });
                    }
                }
            }
            Ok(out)
        }
        YamlValue::Null => Ok(Vec::new()),
        _ => Err(TimelineMarkdownError::TypeMismatch {
            field,
            expected: "sequence",
        }),
    }
}

fn parse_extras_map(v: Option<&YamlValue>) -> Map<String, JsonValue> {
    let Some(v) = v else {
        return Map::new();
    };
    if v.is_null() {
        return Map::new();
    }
    let json: JsonValue = serde_json::to_value(v).unwrap_or(JsonValue::Null);
    if let JsonValue::Object(map) = json {
        map
    } else {
        Map::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NEUTRAL_TIMELINE: &str = "---
id: timeline-test-history
kind: history
name: Test History
aliases: [Test Era List, sample-history]
summary: |
  Multi-line history timeline summary.
tags: [test, timeline, history]
references:
  - era-alpha
  - era-bravo
  - era-charlie
extras:
  game_role: Main timeline
  player_relevance: 5
---

## Overview
Timeline overview prose.

## Era Transitions
Inter-era transition narrative.
";

    #[test]
    fn parse_neutral_timeline_full_roundtrip() {
        let t = timeline_from_markdown(NEUTRAL_TIMELINE).expect("파싱 성공");
        assert_eq!(t.id.as_str(), "timeline-test-history");
        assert_eq!(t.kind, "history");
        assert_eq!(t.name, "Test History");
        assert_eq!(t.aliases, vec!["Test Era List", "sample-history"]);
        assert!(t.summary.contains("Multi-line"));
        assert!(t.tags.contains(&"history".to_string()));
        assert_eq!(
            t.references,
            vec![
                EraId::new("era-alpha"),
                EraId::new("era-bravo"),
                EraId::new("era-charlie"),
            ]
        );
        assert!(t.body_sections.contains_key("Overview"));
        assert!(t.body_sections.contains_key("Era Transitions"));
    }

    #[test]
    fn missing_required_id_errs() {
        let md = "---\nkind: history\nname: x\n---\n";
        assert!(matches!(
            timeline_from_markdown(md),
            Err(TimelineMarkdownError::MissingField("id"))
        ));
    }

    #[test]
    fn missing_required_kind_errs() {
        let md = "---\nid: timeline-x\nname: x\n---\n";
        assert!(matches!(
            timeline_from_markdown(md),
            Err(TimelineMarkdownError::MissingField("kind"))
        ));
    }

    #[test]
    fn missing_required_name_errs() {
        let md = "---\nid: timeline-x\nkind: history\n---\n";
        assert!(matches!(
            timeline_from_markdown(md),
            Err(TimelineMarkdownError::MissingField("name"))
        ));
    }

    #[test]
    fn empty_references_yields_empty() {
        let md = "---\nid: timeline-x\nkind: history\nname: X\n---\n";
        let t = timeline_from_markdown(md).unwrap();
        assert!(t.references.is_empty());
    }

    #[test]
    fn null_references_yields_empty() {
        let md = "---\nid: timeline-x\nkind: history\nname: X\nreferences: ~\n---\n";
        let t = timeline_from_markdown(md).unwrap();
        assert!(t.references.is_empty());
    }

    #[test]
    fn references_preserves_input_order() {
        let md = "---\nid: timeline-x\nkind: history\nname: X\nreferences:\n  - era-c\n  - era-a\n  - era-b\n---\n";
        let t = timeline_from_markdown(md).unwrap();
        assert_eq!(
            t.references,
            vec![EraId::new("era-c"), EraId::new("era-a"), EraId::new("era-b"),]
        );
    }

    // R4 strict typing — references는 외래키 활성, non-String 항목 hard error.
    #[test]
    fn references_with_non_string_item_errs() {
        let md = "---\nid: timeline-x\nkind: history\nname: X\nreferences:\n  - era-a\n  - 42\n  - era-b\n---\n";
        let res = timeline_from_markdown(md);
        assert!(
            matches!(
                res,
                Err(TimelineMarkdownError::TypeMismatch {
                    field: "references",
                    ..
                })
            ),
            "got {res:?}"
        );
    }

    #[test]
    fn references_non_sequence_errs() {
        let md = "---\nid: timeline-x\nkind: history\nname: X\nreferences:\n  a: b\n---\n";
        let res = timeline_from_markdown(md);
        assert!(
            matches!(
                res,
                Err(TimelineMarkdownError::TypeMismatch {
                    field: "references",
                    ..
                })
            ),
            "got {res:?}"
        );
    }

    #[test]
    fn aliases_with_non_string_remains_permissive() {
        // 자유 메타 — Phase 5a·5b permissive 정책 일관.
        let md = "---\nid: timeline-x\nkind: history\nname: X\naliases:\n  - alias-a\n  - 42\n  - alias-b\n---\n";
        let t = timeline_from_markdown(md).expect("aliases는 permissive");
        assert_eq!(t.aliases, vec!["alias-a", "alias-b"]);
    }

    /// 한국어 wuxia timeline.
    const WUXIA_TIMELINE: &str = "---
id: timeline-jungwon-history
kind: history
name: 칠국춘추 270년사
aliases:
  - 중원사
  - main-history
  - 270년 연표
summary: |
  원년부터 현재(270년차)까지의 핵심 분기점을 묶은 메인 시간선.
tags: [wuxia, timeline, history, main]
references:
  - era-founding
  - era-prosperity
  - era-turning
  - era-decline
  - era-fall-of-empire
extras:
  game_role: 메인 시간선 — 모든 NPC 대사가 본 timeline의 era에서 인과를 끌어옴
  player_relevance: 5
---

## 개요
원년부터 현재까지 270년의 핵심 분기점.

## Era 변천
건국기 → 전성기 → 변곡기 → 쇠퇴기 → 붕괴기.

## 핵심 인과 사슬
empire-founding → bloody-cult-rebellion-2nd → blood-disappearance → bloody-night ↔ hwasan-fall → six-states-independence.

## 게임 시점에서의 활용
NPC 대사·메인 퀘스트 단서·서사 분기점 모두 본 timeline의 사건들을 인과로 활용.
";

    #[test]
    fn parse_wuxia_timeline_preserves_korean() {
        let t = timeline_from_markdown(WUXIA_TIMELINE).expect("한국어 wuxia timeline 파싱 성공");
        assert_eq!(t.name, "칠국춘추 270년사");
        assert_eq!(t.aliases, vec!["중원사", "main-history", "270년 연표"]);
        assert_eq!(t.references.len(), 5);
        assert_eq!(t.references[0], EraId::new("era-founding"));
        assert_eq!(t.references[4], EraId::new("era-fall-of-empire"));
        assert!(t.body_sections.contains_key("개요"));
        assert!(t.body_sections.contains_key("Era 변천"));
        assert!(t.body_sections.contains_key("핵심 인과 사슬"));
        assert!(t.body_sections.contains_key("게임 시점에서의 활용"));
    }
}
