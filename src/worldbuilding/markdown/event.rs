//! Event 마크다운 → `Event` 애그리거트 변환.
//!
//! 입력 frontmatter 스키마는 `docs/tasks/task-phase5a-event-vertical-slice.md` §6.1을 따름.
//!
//! 변환 정책:
//! - `kind`/`category`/`name`/`id`는 frontmatter 필수
//! - `temporal.year_relative`는 정수 (음수 허용 — 270년차 기준 절대 연도)
//! - `participants.{people,groups,places}`는 문자열 배열 (외래키 검증은 world-load 책임)
//! - `era_id`는 텍스트만 보존 (Phase 5b 외래키 활성)

use std::collections::BTreeMap;

use serde_json::{Map, Value as JsonValue};
use serde_yaml::Value as YamlValue;

use crate::domain::world::{
    Event, EventCategory, EventId, EventTemporal, ParticipantsRefs,
};

use super::frontmatter::{FrontmatterError, parse_frontmatter, parse_h2_sections};

#[derive(Debug, thiserror::Error)]
pub enum EventMarkdownError {
    #[error(transparent)]
    Frontmatter(#[from] FrontmatterError),
    #[error("필수 필드 누락: {0}")]
    MissingField(&'static str),
    #[error("필드 '{field}' 타입 불일치 (기대: {expected})")]
    TypeMismatch {
        field: &'static str,
        expected: &'static str,
    },
    #[error("필드 '{field}'에 알 수 없는 값 '{value}' (허용: {allowed})")]
    UnknownValue {
        field: &'static str,
        value: String,
        allowed: &'static str,
    },
}

/// 마크다운 텍스트를 Event로 변환. `source_path`는 호출자가 setter로 주입.
pub fn event_from_markdown(md: &str) -> Result<Event, EventMarkdownError> {
    let fm = parse_frontmatter(md)?;
    let map = fm
        .value
        .as_mapping()
        .ok_or(EventMarkdownError::MissingField("frontmatter (mapping)"))?;

    let id = get_str(map, "id")
        .ok_or(EventMarkdownError::MissingField("id"))?
        .to_string();
    let kind = get_str(map, "kind")
        .ok_or(EventMarkdownError::MissingField("kind"))?
        .to_string();
    let name = get_str(map, "name")
        .ok_or(EventMarkdownError::MissingField("name"))?
        .to_string();

    // category — 누락 시 default(historical), 알 수 없는 값은 hard error
    // (조용한 default fallback이 silent regression을 부르므로).
    let category = match get_str(map, "category") {
        None => EventCategory::default(),
        Some(s) => EventCategory::from_str_loose(s).ok_or_else(|| {
            EventMarkdownError::UnknownValue {
                field: "category",
                value: s.to_string(),
                allowed: "historical | scheduled | legendary",
            }
        })?,
    };

    let aliases = get_string_array(map, "aliases").unwrap_or_default();
    let summary = get_str(map, "summary")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let tags = get_string_array(map, "tags").unwrap_or_default();
    let extras = parse_extras_map(map.get(&YamlValue::from("extras")));
    let temporal = parse_temporal(map.get(&YamlValue::from("temporal")))?;
    let era_id = get_optional_str(map, "era_id").map(|s| s.to_string());
    let participants = parse_participants(map.get(&YamlValue::from("participants")))?;
    let related_events = get_string_array(map, "related_events")
        .unwrap_or_default()
        .into_iter()
        .map(EventId::new)
        .collect();
    let body_sections: BTreeMap<String, String> = parse_h2_sections(&fm.body);

    Ok(Event {
        id: EventId::new(id),
        kind,
        category,
        name,
        aliases,
        summary,
        tags,
        extras,
        temporal,
        era_id,
        participants,
        body_sections,
        related_events,
        source_path: None,
    })
}

// ---------------------------------------------------------------------------
// YAML 추출 헬퍼 — group/person/place/atlas 파서와 동일 패턴.
// ---------------------------------------------------------------------------

fn get_str<'a>(map: &'a serde_yaml::Mapping, key: &str) -> Option<&'a str> {
    map.get(YamlValue::from(key)).and_then(|v| match v {
        YamlValue::String(s) => Some(s.as_str()),
        YamlValue::Null => None,
        _ => None,
    })
}

/// `era_id: ~` 같은 명시적 null과 미지정·빈 문자열을 모두 None으로.
/// 비-null 문자열만 Some으로 반환하므로 `era_id` 텍스트 보존에 적합.
fn get_optional_str<'a>(map: &'a serde_yaml::Mapping, key: &str) -> Option<&'a str> {
    map.get(YamlValue::from(key)).and_then(|v| match v {
        YamlValue::String(s) if !s.is_empty() => Some(s.as_str()),
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

fn parse_temporal(v: Option<&YamlValue>) -> Result<EventTemporal, EventMarkdownError> {
    let Some(v) = v else {
        return Ok(EventTemporal::default());
    };
    if v.is_null() {
        return Ok(EventTemporal::default());
    }
    let map = v
        .as_mapping()
        .ok_or(EventMarkdownError::TypeMismatch {
            field: "temporal",
            expected: "mapping",
        })?;
    let year = get_str(map, "year").map(|s| s.to_string());
    // year_relative는 정수 (음수 허용). YAML i64 → i32 변환 시 범위 위반은 silent clamp 대신
    // 그대로 변환 — 270년차 스케일(±300)에선 i32로 충분.
    let year_relative = map
        .get(YamlValue::from("year_relative"))
        .and_then(|v| match v {
            YamlValue::Number(n) => n.as_i64().map(|x| x as i32),
            _ => None,
        });
    let duration = get_str(map, "duration").map(|s| s.to_string());
    let notes = get_str(map, "notes").map(|s| s.to_string());
    Ok(EventTemporal {
        year,
        year_relative,
        duration,
        notes,
    })
}

fn parse_participants(
    v: Option<&YamlValue>,
) -> Result<ParticipantsRefs, EventMarkdownError> {
    let Some(v) = v else {
        return Ok(ParticipantsRefs::default());
    };
    if v.is_null() {
        return Ok(ParticipantsRefs::default());
    }
    let map = v
        .as_mapping()
        .ok_or(EventMarkdownError::TypeMismatch {
            field: "participants",
            expected: "mapping",
        })?;
    let people = get_string_array(map, "people").unwrap_or_default();
    let groups = get_string_array(map, "groups").unwrap_or_default();
    let places = get_string_array(map, "places").unwrap_or_default();
    Ok(ParticipantsRefs {
        people,
        groups,
        places,
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
    const NEUTRAL_EVENT: &str = "---
id: event-test-betrayal
kind: betrayal
category: historical
name: Test Betrayal
aliases: [The Night, Test Aliases]
summary: |
  Multi-line event summary.
tags: [test, event, historical]
temporal:
  year: 10 years ago (year 260)
  year_relative: -10
  duration: three nights
  notes: Migration to Era domain pending Phase 5b.
era_id: era-fall-of-empire
participants:
  people:
    - npc-alpha
    - npc-bravo
  groups:
    - group-court
  places:
    - place-capital
related_events:
  - event-aftermath
extras:
  trigger: Power vacuum after coronation
  outcome: Six states declare independence
  player_relevance: 5
---

## Overview
Event overview prose.

## Triggers
Cause and conditions.

## Aftermath
Long-term consequences.
";

    #[test]
    fn parse_neutral_event_full_roundtrip() {
        let e = event_from_markdown(NEUTRAL_EVENT).expect("파싱 성공");
        assert_eq!(e.id.as_str(), "event-test-betrayal");
        assert_eq!(e.kind, "betrayal");
        assert_eq!(e.category, EventCategory::Historical);
        assert_eq!(e.name, "Test Betrayal");
        assert_eq!(e.aliases, vec!["The Night", "Test Aliases"]);
        assert!(e.summary.contains("Multi-line"));
        assert!(e.tags.contains(&"historical".to_string()));

        // temporal
        assert_eq!(e.temporal.year.as_deref(), Some("10 years ago (year 260)"));
        assert_eq!(e.temporal.year_relative, Some(-10));
        assert_eq!(e.temporal.duration.as_deref(), Some("three nights"));
        assert!(e.temporal.notes.is_some());

        // era_id 텍스트 보존
        assert_eq!(e.era_id.as_deref(), Some("era-fall-of-empire"));

        // participants 외래키 텍스트 보존
        assert_eq!(
            e.participants.people,
            vec!["npc-alpha".to_string(), "npc-bravo".to_string()]
        );
        assert_eq!(e.participants.groups, vec!["group-court".to_string()]);
        assert_eq!(e.participants.places, vec!["place-capital".to_string()]);

        // related_events
        assert_eq!(
            e.related_events,
            vec![EventId::new("event-aftermath")]
        );

        // body sections
        assert!(e.body_sections.contains_key("Overview"));
        assert!(e.body_sections.contains_key("Triggers"));
        assert!(e.body_sections.contains_key("Aftermath"));

        // extras
        assert_eq!(
            e.extras.get("trigger").and_then(|v| v.as_str()),
            Some("Power vacuum after coronation")
        );
        assert_eq!(
            e.extras.get("outcome").and_then(|v| v.as_str()),
            Some("Six states declare independence")
        );
    }

    #[test]
    fn missing_required_id_errs() {
        let md = "---\nkind: betrayal\nname: x\n---\n";
        assert!(matches!(
            event_from_markdown(md),
            Err(EventMarkdownError::MissingField("id"))
        ));
    }

    #[test]
    fn missing_required_kind_errs() {
        let md = "---\nid: event-x\nname: x\n---\n";
        assert!(matches!(
            event_from_markdown(md),
            Err(EventMarkdownError::MissingField("kind"))
        ));
    }

    #[test]
    fn missing_required_name_errs() {
        let md = "---\nid: event-x\nkind: war\n---\n";
        assert!(matches!(
            event_from_markdown(md),
            Err(EventMarkdownError::MissingField("name"))
        ));
    }

    #[test]
    fn missing_category_defaults_to_historical() {
        let md = "---\nid: event-x\nkind: war\nname: X\n---\n";
        let e = event_from_markdown(md).unwrap();
        assert_eq!(e.category, EventCategory::Historical);
    }

    #[test]
    fn unknown_category_errs() {
        let md = "---\nid: event-x\nkind: war\nname: X\ncategory: invalid\n---\n";
        let res = event_from_markdown(md);
        assert!(matches!(
            res,
            Err(EventMarkdownError::UnknownValue {
                field: "category",
                ..
            })
        ));
    }

    #[test]
    fn known_categories_parse_correctly() {
        for (cat, expected) in [
            ("historical", EventCategory::Historical),
            ("scheduled", EventCategory::Scheduled),
            ("legendary", EventCategory::Legendary),
        ] {
            let md = format!("---\nid: event-x\nkind: war\nname: X\ncategory: {cat}\n---\n");
            let e = event_from_markdown(&md).unwrap();
            assert_eq!(e.category, expected);
        }
    }

    #[test]
    fn empty_temporal_yields_default() {
        let md = "---\nid: event-x\nkind: war\nname: X\n---\n";
        let e = event_from_markdown(md).unwrap();
        assert_eq!(e.temporal, EventTemporal::default());
    }

    #[test]
    fn null_temporal_yields_default() {
        let md = "---\nid: event-x\nkind: war\nname: X\ntemporal: ~\n---\n";
        let e = event_from_markdown(md).unwrap();
        assert_eq!(e.temporal, EventTemporal::default());
    }

    #[test]
    fn negative_year_relative_parses() {
        // 270년차 기준 음수 — i32로 보존.
        let md = "---\nid: event-x\nkind: war\nname: X\ntemporal:\n  year_relative: -270\n---\n";
        let e = event_from_markdown(md).unwrap();
        assert_eq!(e.temporal.year_relative, Some(-270));
    }

    #[test]
    fn zero_year_relative_parses() {
        // 270년차 = 현재 = year_relative = 0.
        let md = "---\nid: event-x\nkind: war\nname: X\ntemporal:\n  year_relative: 0\n---\n";
        let e = event_from_markdown(md).unwrap();
        assert_eq!(e.temporal.year_relative, Some(0));
    }

    #[test]
    fn empty_participants_yields_empty() {
        let md = "---\nid: event-x\nkind: war\nname: X\n---\n";
        let e = event_from_markdown(md).unwrap();
        assert!(e.participants.is_empty());
    }

    #[test]
    fn null_participants_yields_empty() {
        let md = "---\nid: event-x\nkind: war\nname: X\nparticipants: ~\n---\n";
        let e = event_from_markdown(md).unwrap();
        assert!(e.participants.is_empty());
    }

    #[test]
    fn participants_partial_categories() {
        // 일부 카테고리만 있어도 다른 카테고리는 빈 Vec로 자연스럽게 채워짐.
        let md = "---\nid: event-x\nkind: war\nname: X\nparticipants:\n  people:\n    - npc-01\n---\n";
        let e = event_from_markdown(md).unwrap();
        assert_eq!(e.participants.people, vec!["npc-01".to_string()]);
        assert!(e.participants.groups.is_empty());
        assert!(e.participants.places.is_empty());
    }

    #[test]
    fn participants_preserves_input_order() {
        // 인물 순서가 사건의 비중과 시각적 의미를 가지므로 작성 순서 보존.
        let md = "---\nid: event-x\nkind: war\nname: X\nparticipants:\n  people:\n    - npc-c\n    - npc-a\n    - npc-b\n---\n";
        let e = event_from_markdown(md).unwrap();
        assert_eq!(
            e.participants.people,
            vec!["npc-c".to_string(), "npc-a".to_string(), "npc-b".to_string()]
        );
    }

    #[test]
    fn era_id_null_yields_none() {
        let md = "---\nid: event-x\nkind: war\nname: X\nera_id: ~\n---\n";
        let e = event_from_markdown(md).unwrap();
        assert!(e.era_id.is_none());
    }

    #[test]
    fn era_id_empty_string_yields_none() {
        // 빈 문자열은 None 취급 — Phase 5b 외래키 검증 시 의미 있는 값만 유효.
        let md = "---\nid: event-x\nkind: war\nname: X\nera_id: \"\"\n---\n";
        let e = event_from_markdown(md).unwrap();
        assert!(e.era_id.is_none());
    }

    #[test]
    fn era_id_text_preserved() {
        // Phase 5a엔 era_id 텍스트만 보존. Phase 5b 외래키 활성 예정.
        let md = "---\nid: event-x\nkind: war\nname: X\nera_id: era-fall-of-empire\n---\n";
        let e = event_from_markdown(md).unwrap();
        assert_eq!(e.era_id.as_deref(), Some("era-fall-of-empire"));
    }

    #[test]
    fn related_events_yields_event_ids() {
        let md = "---\nid: event-x\nkind: war\nname: X\nrelated_events:\n  - event-a\n  - event-b\n---\n";
        let e = event_from_markdown(md).unwrap();
        assert_eq!(
            e.related_events,
            vec![EventId::new("event-a"), EventId::new("event-b")]
        );
    }

    #[test]
    fn related_events_empty_when_omitted() {
        let md = "---\nid: event-x\nkind: war\nname: X\n---\n";
        let e = event_from_markdown(md).unwrap();
        assert!(e.related_events.is_empty());
    }

    #[test]
    fn body_sections_parsed_in_order_independent_btreemap() {
        let md = "---\nid: event-x\nkind: war\nname: X\n---\n\n## A 섹션\n첫 본문\n\n## B 섹션\n둘째 본문\n";
        let e = event_from_markdown(md).unwrap();
        assert!(e.body_sections.contains_key("A 섹션"));
        assert!(e.body_sections.contains_key("B 섹션"));
        assert!(e.body_sections.get("A 섹션").unwrap().contains("첫 본문"));
    }

    #[test]
    fn extras_map_preserved() {
        let md = "---\nid: event-x\nkind: war\nname: X\nextras:\n  trigger: foo\n  outcome: bar\n  player_relevance: 5\n---\n";
        let e = event_from_markdown(md).unwrap();
        assert_eq!(
            e.extras.get("trigger").and_then(|v| v.as_str()),
            Some("foo")
        );
        assert_eq!(
            e.extras.get("outcome").and_then(|v| v.as_str()),
            Some("bar")
        );
        // 정수도 보존 (str-only 강제 X).
        assert_eq!(
            e.extras.get("player_relevance").and_then(|v| v.as_i64()),
            Some(5)
        );
    }

    /// 한국어 wuxia 어휘 — 도메인 자체엔 무협 어휘가 없지만 파서는 한국어 콘텐츠를
    /// 그대로 보존해야 함 (frontmatter 안의 한국어 + body 한국어 H2).
    const WUXIA_EVENT: &str = "---
id: event-bloody-night
kind: betrayal
category: historical
name: 붉은 밤의 변
aliases:
  - 붉은 밤
  - 10년 전 변란
summary: |
  10년 전(260년차) 통일제국 대진의 영토 와해를 가져온 결정적 사건.
tags: [wuxia, event, historical, fall-of-empire]
temporal:
  year: 10년 전 (260년차)
  year_relative: -10
  duration: 사흘 밤
  notes: Phase 5b Era 결합 시 era-fall-of-empire로 정형 시간 승급 예정.
era_id: ~
participants:
  people:
    - npc-02
    - npc-07
    - npc-01
  groups:
    - group-daejin-court
    - group-shipsangsi
  places:
    - place-daejin
    - place-namgung
related_events: []
extras:
  trigger: 천순제 즉위 직후 권력 공백
  outcome: 6 지역 독립 → 칠국춘추 시대 시작
  player_relevance: 5
---

## 개요
산문 — 사건 핵심 묘사.

## 발단
산문 — 직전 상황 + 트리거.
";

    #[test]
    fn parse_wuxia_event_preserves_korean() {
        let e = event_from_markdown(WUXIA_EVENT).expect("한국어 wuxia 사건 파싱 성공");
        assert_eq!(e.name, "붉은 밤의 변");
        assert_eq!(e.aliases, vec!["붉은 밤", "10년 전 변란"]);
        assert_eq!(e.temporal.year.as_deref(), Some("10년 전 (260년차)"));
        assert_eq!(e.temporal.year_relative, Some(-10));
        assert_eq!(e.temporal.duration.as_deref(), Some("사흘 밤"));
        // era_id: ~ → None
        assert!(e.era_id.is_none());
        assert_eq!(e.participants.people.len(), 3);
        assert_eq!(e.participants.groups.len(), 2);
        assert_eq!(e.participants.places.len(), 2);
        assert!(e.body_sections.contains_key("개요"));
        assert!(e.body_sections.contains_key("발단"));
    }
}
