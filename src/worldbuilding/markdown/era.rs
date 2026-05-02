//! Era 마크다운 → `Era` 애그리거트 변환.
//!
//! 입력 frontmatter 스키마는 `docs/tasks/task-phase5b-era-timeline-vertical-slice.md` §6.1을 따름.
//!
//! 변환 정책:
//! - `kind`/`name`/`id`는 frontmatter 필수
//! - `temporal.start_year_relative`/`end_year_relative`는 정수 (음수 허용)
//! - `key_events`는 문자열 배열 — Phase 5b 외래키 활성 (Era→Event 단방향, world-load 검증)
//! - **R4 strict typing 패턴 (Phase 5a)**: key_events 시퀀스의 non-String 항목은 hard error

use std::collections::BTreeMap;

use serde_json::{Map, Value as JsonValue};
use serde_yaml::Value as YamlValue;

use crate::domain::world::{Era, EraId, EraTemporal, EventId};

use super::frontmatter::{FrontmatterError, parse_frontmatter, parse_h2_sections};

#[derive(Debug, thiserror::Error)]
pub enum EraMarkdownError {
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

/// 마크다운 텍스트를 Era로 변환. `source_path`는 호출자가 setter로 주입.
pub fn era_from_markdown(md: &str) -> Result<Era, EraMarkdownError> {
    let fm = parse_frontmatter(md)?;
    let map = fm
        .value
        .as_mapping()
        .ok_or(EraMarkdownError::MissingField("frontmatter (mapping)"))?;

    let id = get_str(map, "id")
        .ok_or(EraMarkdownError::MissingField("id"))?
        .to_string();
    let kind = get_str(map, "kind")
        .ok_or(EraMarkdownError::MissingField("kind"))?
        .to_string();
    let name = get_str(map, "name")
        .ok_or(EraMarkdownError::MissingField("name"))?
        .to_string();

    let aliases = get_string_array(map, "aliases").unwrap_or_default();
    let summary = get_str(map, "summary")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let tags = get_string_array(map, "tags").unwrap_or_default();
    let extras = parse_extras_map(map.get(&YamlValue::from("extras")));
    let temporal = parse_temporal(map.get(&YamlValue::from("temporal")))?;
    // R4 strict (Phase 5a 패턴): key_events는 외래키 활성 ID 시퀀스 — silent skip 차단.
    let key_events = get_string_array_strict(map, "key_events", "key_events")?
        .into_iter()
        .map(EventId::new)
        .collect();
    let body_sections: BTreeMap<String, String> = parse_h2_sections(&fm.body);

    Ok(Era {
        id: EraId::new(id),
        kind,
        name,
        aliases,
        summary,
        tags,
        extras,
        temporal,
        key_events,
        body_sections,
        source_path: None,
    })
}

// ---------------------------------------------------------------------------
// YAML 추출 헬퍼 — group/person/place/atlas/event 파서와 동일 패턴.
// ---------------------------------------------------------------------------

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

/// R4 strict — Phase 5a `event.rs` 패턴 동일. key_events 같은 외래키 활성 ID 시퀀스에서
/// non-String 항목 silent skip 차단.
fn get_string_array_strict(
    map: &serde_yaml::Mapping,
    key: &str,
    field: &'static str,
) -> Result<Vec<String>, EraMarkdownError> {
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
                        return Err(EraMarkdownError::TypeMismatch {
                            field,
                            expected: "string item (외래키 ID)",
                        });
                    }
                }
            }
            Ok(out)
        }
        YamlValue::Null => Ok(Vec::new()),
        _ => Err(EraMarkdownError::TypeMismatch {
            field,
            expected: "sequence",
        }),
    }
}

fn parse_temporal(v: Option<&YamlValue>) -> Result<EraTemporal, EraMarkdownError> {
    let Some(v) = v else {
        return Ok(EraTemporal::default());
    };
    if v.is_null() {
        return Ok(EraTemporal::default());
    }
    let map = v.as_mapping().ok_or(EraMarkdownError::TypeMismatch {
        field: "temporal",
        expected: "mapping",
    })?;
    let start_year_relative = get_i32(map, "start_year_relative");
    let end_year_relative = get_i32(map, "end_year_relative");
    let notes = get_str(map, "notes").map(|s| s.to_string());
    Ok(EraTemporal {
        start_year_relative,
        end_year_relative,
        notes,
    })
}

fn get_i32(map: &serde_yaml::Mapping, key: &str) -> Option<i32> {
    map.get(YamlValue::from(key)).and_then(|v| match v {
        YamlValue::Number(n) => n.as_i64().map(|x| x as i32),
        _ => None,
    })
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

// ---------------------------------------------------------------------------
// 단위 테스트
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// 장르 중립 fixture — 무협 어휘 X.
    const NEUTRAL_ERA: &str = "---
id: era-test-fall
kind: fall
name: Test Fall Era
aliases: [End Era, Last 30 Years]
summary: |
  Multi-line era summary covering the final 30 years.
tags: [test, era, historical]
temporal:
  start_year_relative: -30
  end_year_relative: 0
  notes: Boundary inclusive-exclusive policy applied (§3.3).
key_events:
  - event-alpha
  - event-bravo
  - event-charlie
extras:
  game_role: Final phase trigger
  player_relevance: 5
---

## Overview
Era overview prose.

## Key Triggers
Cause and conditions transitioning into this era.

## Outcome
Long-term consequences of this era's events.
";

    #[test]
    fn parse_neutral_era_full_roundtrip() {
        let e = era_from_markdown(NEUTRAL_ERA).expect("파싱 성공");
        assert_eq!(e.id.as_str(), "era-test-fall");
        assert_eq!(e.kind, "fall");
        assert_eq!(e.name, "Test Fall Era");
        assert_eq!(e.aliases, vec!["End Era", "Last 30 Years"]);
        assert!(e.summary.contains("Multi-line"));
        assert!(e.tags.contains(&"historical".to_string()));

        // temporal
        assert_eq!(e.temporal.start_year_relative, Some(-30));
        assert_eq!(e.temporal.end_year_relative, Some(0));
        assert!(e.temporal.notes.is_some());

        // key_events 외래키 — 텍스트 보존, 작성 순서 유지
        assert_eq!(
            e.key_events,
            vec![
                EventId::new("event-alpha"),
                EventId::new("event-bravo"),
                EventId::new("event-charlie"),
            ]
        );

        // body sections
        assert!(e.body_sections.contains_key("Overview"));
        assert!(e.body_sections.contains_key("Key Triggers"));
        assert!(e.body_sections.contains_key("Outcome"));

        // extras
        assert_eq!(
            e.extras.get("game_role").and_then(|v| v.as_str()),
            Some("Final phase trigger")
        );
        assert_eq!(
            e.extras.get("player_relevance").and_then(|v| v.as_i64()),
            Some(5)
        );
    }

    #[test]
    fn missing_required_id_errs() {
        let md = "---\nkind: fall\nname: x\n---\n";
        assert!(matches!(
            era_from_markdown(md),
            Err(EraMarkdownError::MissingField("id"))
        ));
    }

    #[test]
    fn missing_required_kind_errs() {
        let md = "---\nid: era-x\nname: x\n---\n";
        assert!(matches!(
            era_from_markdown(md),
            Err(EraMarkdownError::MissingField("kind"))
        ));
    }

    #[test]
    fn missing_required_name_errs() {
        let md = "---\nid: era-x\nkind: fall\n---\n";
        assert!(matches!(
            era_from_markdown(md),
            Err(EraMarkdownError::MissingField("name"))
        ));
    }

    #[test]
    fn empty_temporal_yields_default() {
        let md = "---\nid: era-x\nkind: fall\nname: X\n---\n";
        let e = era_from_markdown(md).unwrap();
        assert_eq!(e.temporal, EraTemporal::default());
    }

    #[test]
    fn null_temporal_yields_default() {
        let md = "---\nid: era-x\nkind: fall\nname: X\ntemporal: ~\n---\n";
        let e = era_from_markdown(md).unwrap();
        assert_eq!(e.temporal, EraTemporal::default());
    }

    #[test]
    fn negative_year_relative_parses() {
        let md = "---\nid: era-x\nkind: founding\nname: X\ntemporal:\n  start_year_relative: -270\n  end_year_relative: -220\n---\n";
        let e = era_from_markdown(md).unwrap();
        assert_eq!(e.temporal.start_year_relative, Some(-270));
        assert_eq!(e.temporal.end_year_relative, Some(-220));
    }

    #[test]
    fn empty_key_events_yields_empty() {
        let md = "---\nid: era-x\nkind: fall\nname: X\n---\n";
        let e = era_from_markdown(md).unwrap();
        assert!(e.key_events.is_empty());
    }

    #[test]
    fn null_key_events_yields_empty() {
        let md = "---\nid: era-x\nkind: fall\nname: X\nkey_events: ~\n---\n";
        let e = era_from_markdown(md).unwrap();
        assert!(e.key_events.is_empty());
    }

    #[test]
    fn key_events_preserves_input_order() {
        // 시간순 권장이라 작성 순서 보존이 핵심.
        let md = "---\nid: era-x\nkind: fall\nname: X\nkey_events:\n  - event-c\n  - event-a\n  - event-b\n---\n";
        let e = era_from_markdown(md).unwrap();
        assert_eq!(
            e.key_events,
            vec![
                EventId::new("event-c"),
                EventId::new("event-a"),
                EventId::new("event-b"),
            ]
        );
    }

    // R4 strict typing 회귀 가드 — Phase 5a 패턴 그대로.
    #[test]
    fn key_events_with_non_string_item_errs() {
        let md = "---\nid: era-x\nkind: fall\nname: X\nkey_events:\n  - event-a\n  - 42\n  - event-b\n---\n";
        let res = era_from_markdown(md);
        assert!(
            matches!(
                res,
                Err(EraMarkdownError::TypeMismatch {
                    field: "key_events",
                    ..
                })
            ),
            "got {res:?}"
        );
    }

    #[test]
    fn key_events_non_sequence_errs() {
        let md = "---\nid: era-x\nkind: fall\nname: X\nkey_events:\n  a: b\n---\n";
        let res = era_from_markdown(md);
        assert!(
            matches!(
                res,
                Err(EraMarkdownError::TypeMismatch {
                    field: "key_events",
                    ..
                })
            ),
            "got {res:?}"
        );
    }

    #[test]
    fn aliases_with_non_string_remains_permissive() {
        // 자유 메타는 silent skip — Phase 5a permissive 정책 일관 적용.
        let md = "---\nid: era-x\nkind: fall\nname: X\naliases:\n  - alias-a\n  - 42\n  - alias-b\n---\n";
        let e = era_from_markdown(md).expect("aliases는 permissive");
        assert_eq!(e.aliases, vec!["alias-a", "alias-b"]);
    }

    #[test]
    fn body_sections_parsed() {
        let md = "---\nid: era-x\nkind: fall\nname: X\n---\n\n## A\nfirst body\n\n## B\nsecond body\n";
        let e = era_from_markdown(md).unwrap();
        assert!(e.body_sections.contains_key("A"));
        assert!(e.body_sections.contains_key("B"));
    }

    #[test]
    fn extras_map_preserved() {
        let md = "---\nid: era-x\nkind: fall\nname: X\nextras:\n  game_role: trigger\n  player_relevance: 5\n---\n";
        let e = era_from_markdown(md).unwrap();
        assert_eq!(
            e.extras.get("game_role").and_then(|v| v.as_str()),
            Some("trigger")
        );
        assert_eq!(
            e.extras.get("player_relevance").and_then(|v| v.as_i64()),
            Some(5)
        );
    }

    /// 한국어 wuxia era — frontmatter 안의 한국어 + body H2 정상 보존.
    const WUXIA_ERA: &str = "---
id: era-fall-of-empire
kind: fall
name: 붕괴기
aliases:
  - 6국 분열기
  - 240-270년차
summary: |
  240~270년차의 30년. 통일제국 대진의 영토 와해와 칠국 형성이 일어난 시기.
tags: [wuxia, era, historical, fall-of-empire]
temporal:
  start_year_relative: -30
  end_year_relative: 0
  notes: |
    boundary 정책 §3.3 — start inclusive · end exclusive.
key_events:
  - event-bloody-cult-rebellion-2nd
  - event-blood-disappearance
  - event-bloody-night
  - event-hwasan-fall
  - event-six-states-independence
extras:
  game_role: 게임 시작 시점의 정치 지도가 본 시대에서 형성됨
  player_relevance: 5
---

## 개요
산문 — 본 시대의 핵심 흐름.

## 핵심 트리거
산문 — 직전 시대(쇠퇴기)에서 본 시대로 넘어가는 트리거.
";

    #[test]
    fn parse_wuxia_era_preserves_korean() {
        let e = era_from_markdown(WUXIA_ERA).expect("한국어 wuxia era 파싱 성공");
        assert_eq!(e.name, "붕괴기");
        assert_eq!(e.aliases, vec!["6국 분열기", "240-270년차"]);
        assert_eq!(e.temporal.start_year_relative, Some(-30));
        assert_eq!(e.temporal.end_year_relative, Some(0));
        assert_eq!(e.key_events.len(), 5);
        assert_eq!(
            e.key_events[0],
            EventId::new("event-bloody-cult-rebellion-2nd")
        );
        assert!(e.body_sections.contains_key("개요"));
        assert!(e.body_sections.contains_key("핵심 트리거"));
    }
}
