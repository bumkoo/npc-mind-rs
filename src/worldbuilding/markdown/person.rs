//! Person 마크다운 → `Person` 애그리거트 변환.
//!
//! 입력 frontmatter 스키마는 `docs/tasks/task-phase2-person-vertical-slice.md` §6.1을 따름.
//!
//! HEXACO 6 dim은 frontmatter `hexaco:` 매핑에서 직접 추출. `Score` VO가 -1.0~+1.0
//! 범위를 deserialize 시점에 검증하므로 유효성은 자동.
//!
//! 24 facet은 `extras.hexaco_facets` 정형 JSON으로 보존 (선택). 없으면 본문
//! `## HEXACO 분석` 섹션의 산문이 보충 자료.

use std::collections::BTreeMap;

use serde_json::{Map, Value as JsonValue};
use serde_yaml::Value as YamlValue;

use crate::domain::personality::Score;
use crate::domain::world::{
    GroupId, HexacoSix, Person, PersonId, PersonStatus, PersonTemporal,
};

use super::frontmatter::{FrontmatterError, parse_frontmatter, parse_h2_sections};

#[derive(Debug, thiserror::Error)]
pub enum PersonMarkdownError {
    #[error(transparent)]
    Frontmatter(#[from] FrontmatterError),
    #[error("필수 필드 누락: {0}")]
    MissingField(&'static str),
    #[error("필드 '{field}' 타입 불일치 (기대: {expected})")]
    TypeMismatch {
        field: &'static str,
        expected: &'static str,
    },
    #[error("HEXACO '{field}' 값 범위 초과({value}): -1.0 ~ +1.0")]
    HexacoOutOfRange { field: &'static str, value: f32 },
}

/// 마크다운 텍스트를 Person으로 변환. `source_path`는 호출자가 setter로 주입.
pub fn person_from_markdown(md: &str) -> Result<Person, PersonMarkdownError> {
    let fm = parse_frontmatter(md)?;
    let map = fm
        .value
        .as_mapping()
        .ok_or(PersonMarkdownError::MissingField("frontmatter (mapping)"))?;

    let id = get_str(map, "id")
        .ok_or(PersonMarkdownError::MissingField("id"))?
        .to_string();
    let kind = get_str(map, "kind")
        .ok_or(PersonMarkdownError::MissingField("kind"))?
        .to_string();
    let name = get_str(map, "name")
        .ok_or(PersonMarkdownError::MissingField("name"))?
        .to_string();

    let aliases = get_string_array(map, "aliases").unwrap_or_default();
    let status = match get_str(map, "status") {
        Some(s) => PersonStatus::from_str_loose(s).ok_or(PersonMarkdownError::TypeMismatch {
            field: "status",
            expected: "alive|dead|missing|unknown",
        })?,
        None => PersonStatus::default(),
    };
    let hexaco = parse_hexaco(map.get(&YamlValue::from("hexaco")))?;
    let temporal = parse_temporal(map.get(&YamlValue::from("temporal")))?;
    let affiliation = get_string_array(map, "affiliation")
        .unwrap_or_default()
        .into_iter()
        .map(GroupId::new)
        .collect();
    let birthplace = get_str(map, "birthplace").map(|s| s.to_string());
    let current_location = get_str(map, "current_location").map(|s| s.to_string());
    let summary = get_str(map, "summary")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let tags = get_string_array(map, "tags").unwrap_or_default();
    let extras = parse_extras_map(map.get(&YamlValue::from("extras")));
    let body_sections: BTreeMap<String, String> = parse_h2_sections(&fm.body);

    Ok(Person {
        id: PersonId::new(id),
        kind,
        name,
        aliases,
        status,
        hexaco,
        temporal,
        affiliation,
        birthplace,
        current_location,
        summary,
        tags,
        extras,
        body_sections,
        source_path: None,
    })
}

// ---------------------------------------------------------------------------
// YAML 추출 헬퍼 (Group 파서와 동일 패턴 — 추후 frontmatter 모듈로 합치는 것 검토)
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

fn parse_hexaco(v: Option<&YamlValue>) -> Result<HexacoSix, PersonMarkdownError> {
    let Some(v) = v else { return Ok(HexacoSix::neutral()); };
    if v.is_null() {
        return Ok(HexacoSix::neutral());
    }
    let map = v
        .as_mapping()
        .ok_or(PersonMarkdownError::TypeMismatch {
            field: "hexaco",
            expected: "mapping",
        })?;

    let read = |field: &'static str| -> Result<Score, PersonMarkdownError> {
        match map.get(YamlValue::from(field)) {
            Some(YamlValue::Number(n)) => {
                let raw = n
                    .as_f64()
                    .ok_or(PersonMarkdownError::TypeMismatch {
                        field,
                        expected: "number",
                    })? as f32;
                Score::new(raw, field)
                    .map_err(|_| PersonMarkdownError::HexacoOutOfRange { field, value: raw })
            }
            None | Some(YamlValue::Null) => Ok(Score::neutral()),
            _ => Err(PersonMarkdownError::TypeMismatch {
                field,
                expected: "number",
            }),
        }
    };

    Ok(HexacoSix {
        honesty_humility: read("honesty_humility")?,
        emotionality: read("emotionality")?,
        extraversion: read("extraversion")?,
        agreeableness: read("agreeableness")?,
        conscientiousness: read("conscientiousness")?,
        openness: read("openness")?,
    })
}

fn parse_temporal(v: Option<&YamlValue>) -> Result<PersonTemporal, PersonMarkdownError> {
    let Some(v) = v else { return Ok(PersonTemporal::default()); };
    if v.is_null() {
        return Ok(PersonTemporal::default());
    }
    let map = v
        .as_mapping()
        .ok_or(PersonMarkdownError::TypeMismatch {
            field: "temporal",
            expected: "mapping",
        })?;
    let birth_year = get_str(map, "birth_year").map(|s| s.trim().to_string());
    let death_year = get_str(map, "death_year").map(|s| s.trim().to_string());
    let age_at_game_start = match map.get(YamlValue::from("age_at_game_start")) {
        Some(YamlValue::Number(n)) => n.as_u64().map(|x| x as u32),
        _ => None,
    };
    let notes = get_str(map, "notes").map(|s| s.trim().to_string());
    Ok(PersonTemporal {
        birth_year,
        death_year,
        age_at_game_start,
        notes,
    })
}

fn parse_extras_map(v: Option<&YamlValue>) -> Map<String, JsonValue> {
    let Some(v) = v else { return Map::new(); };
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

    // 장르 중립 fixture — wuxia 어휘 없이 frontmatter 모든 필드를 시연.
    // 실제 인물 변환은 `tests/world_chilguk_chunchu_e2e.rs` 참조.
    const NEUTRAL_FIXTURE: &str = r#"---
id: person-alpha
kind: active
name: Alice Alpha
aliases: [The Architect, A.A.]
status: alive
hexaco:
  honesty_humility: 0.5
  emotionality: -0.2
  extraversion: 0.3
  agreeableness: 0.6
  conscientiousness: 0.8
  openness: 0.7
temporal:
  birth_year: Year 0
  age_at_game_start: 35
  notes: Founding member of Alpha Council.
affiliation:
  - group-alpha
  - group-beta
birthplace: place-alpha-hq
current_location: place-alpha-hq
summary: |
  Multi-line summary describing the architect's role.
tags: [test, person, founder]
extras:
  signature_skill: charter writing
  priority: A
  hexaco_facets:
    H_sincerity: 0.7
    H_fairness: 0.6
---

## 개요
Prose paragraph describing the person.

## 동기
What they want, what they fear.

## HEXACO 분석
Justification prose for the 6-dim values above.
"#;

    #[test]
    fn parse_neutral_fixture_full_roundtrip() {
        let p = person_from_markdown(NEUTRAL_FIXTURE).expect("파싱 성공");
        assert_eq!(p.id.as_str(), "person-alpha");
        assert_eq!(p.kind, "active");
        assert_eq!(p.name, "Alice Alpha");
        assert_eq!(p.aliases, vec!["The Architect", "A.A."]);
        assert_eq!(p.status, PersonStatus::Alive);

        // hexaco 6 dim
        assert!((p.hexaco.honesty_humility.value() - 0.5).abs() < 1e-6);
        assert!((p.hexaco.emotionality.value() - -0.2).abs() < 1e-6);
        assert!((p.hexaco.openness.value() - 0.7).abs() < 1e-6);

        // temporal
        assert_eq!(p.temporal.birth_year.as_deref(), Some("Year 0"));
        assert_eq!(p.temporal.age_at_game_start, Some(35));

        // affiliation
        assert_eq!(p.affiliation.len(), 2);
        assert_eq!(p.affiliation[0], GroupId::new("group-alpha"));

        // 외래키 텍스트
        assert_eq!(p.birthplace.as_deref(), Some("place-alpha-hq"));
        assert_eq!(p.current_location.as_deref(), Some("place-alpha-hq"));

        // summary + tags
        assert!(p.summary.contains("architect"));
        assert!(p.tags.contains(&"founder".to_string()));

        // extras
        assert_eq!(
            p.extras.get("signature_skill").and_then(|v| v.as_str()),
            Some("charter writing")
        );
        assert!(p.extras.contains_key("hexaco_facets"));

        // body sections
        assert!(p.body_sections.contains_key("개요"));
        assert!(p.body_sections.contains_key("HEXACO 분석"));
    }

    #[test]
    fn missing_required_field_errs() {
        let md = "---\nname: x\nkind: active\n---\n";
        assert!(matches!(
            person_from_markdown(md),
            Err(PersonMarkdownError::MissingField("id"))
        ));
    }

    #[test]
    fn invalid_status_errs() {
        let md = "---\nid: npc-x\nkind: active\nname: x\nstatus: ghost\n---\n";
        assert!(matches!(
            person_from_markdown(md),
            Err(PersonMarkdownError::TypeMismatch { field: "status", .. })
        ));
    }

    #[test]
    fn hexaco_out_of_range_errs() {
        let md = "---\nid: npc-x\nkind: active\nname: x\nhexaco:\n  honesty_humility: -1.5\n---\n";
        assert!(matches!(
            person_from_markdown(md),
            Err(PersonMarkdownError::HexacoOutOfRange { field: "honesty_humility", .. })
        ));
    }

    #[test]
    fn missing_hexaco_yields_neutral() {
        let md = "---\nid: npc-x\nkind: active\nname: x\n---\n";
        let p = person_from_markdown(md).unwrap();
        assert_eq!(p.hexaco, HexacoSix::neutral());
    }

    #[test]
    fn partial_hexaco_uses_neutral_for_missing_dim() {
        // 3 dim만 명시 — 나머지는 0.0(neutral)으로.
        let md = "---\nid: npc-x\nkind: active\nname: x\nhexaco:\n  honesty_humility: 0.5\n  conscientiousness: 0.8\n  openness: 0.7\n---\n";
        let p = person_from_markdown(md).unwrap();
        assert!((p.hexaco.honesty_humility.value() - 0.5).abs() < 1e-6);
        assert_eq!(p.hexaco.emotionality.value(), 0.0);
        assert_eq!(p.hexaco.extraversion.value(), 0.0);
        assert_eq!(p.hexaco.agreeableness.value(), 0.0);
        assert!((p.hexaco.conscientiousness.value() - 0.8).abs() < 1e-6);
        assert!((p.hexaco.openness.value() - 0.7).abs() < 1e-6);
    }

    #[test]
    fn affiliation_parses_as_groupids() {
        let md = "---\nid: npc-x\nkind: active\nname: x\naffiliation: [group-a, group-b]\n---\n";
        let p = person_from_markdown(md).unwrap();
        assert_eq!(
            p.affiliation,
            vec![GroupId::new("group-a"), GroupId::new("group-b")]
        );
    }

    #[test]
    fn empty_affiliation_yields_empty_vec() {
        let md = "---\nid: npc-x\nkind: active\nname: x\naffiliation: []\n---\n";
        let p = person_from_markdown(md).unwrap();
        assert!(p.affiliation.is_empty());
    }

    #[test]
    fn invalid_hexaco_type_errs() {
        let md = "---\nid: npc-x\nkind: active\nname: x\nhexaco:\n  honesty_humility: \"high\"\n---\n";
        assert!(matches!(
            person_from_markdown(md),
            Err(PersonMarkdownError::TypeMismatch { field: "honesty_humility", .. })
        ));
    }

    #[test]
    fn historical_kind_parses() {
        let md = "---\nid: H01\nkind: historical\nname: 진천명\nstatus: dead\ntemporal:\n  death_year: 270년 전\n---\n";
        let p = person_from_markdown(md).unwrap();
        assert_eq!(p.kind, "historical");
        assert_eq!(p.status, PersonStatus::Dead);
        assert_eq!(p.temporal.death_year.as_deref(), Some("270년 전"));
        assert!(!p.is_mind_eligible());
    }
}
