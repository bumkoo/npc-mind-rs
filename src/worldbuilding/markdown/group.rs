//! Group 마크다운 → `Group` 애그리거트 변환.
//!
//! 입력 frontmatter 스키마는 `docs/tasks/task-phase1-group-vertical-slice.md` §6.1을 따름.

use crate::domain::world::{Group, GroupId, GroupStatus, MemberRef, Temporal};
use serde_json::{Map, Value as JsonValue};
use serde_yaml::Value as YamlValue;

use super::frontmatter::{FrontmatterError, parse_frontmatter, parse_h2_sections};

#[derive(Debug, thiserror::Error)]
pub enum GroupMarkdownError {
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

/// 마크다운 텍스트를 Group으로 변환. `source_path`는 호출자가 setter로 주입.
pub fn group_from_markdown(md: &str) -> Result<Group, GroupMarkdownError> {
    let fm = parse_frontmatter(md)?;
    let map = fm
        .value
        .as_mapping()
        .ok_or(GroupMarkdownError::MissingField("frontmatter (mapping)"))?;

    let id = get_str(map, "id")
        .ok_or(GroupMarkdownError::MissingField("id"))?
        .to_string();
    let kind = get_str(map, "kind")
        .ok_or(GroupMarkdownError::MissingField("kind"))?
        .to_string();
    let name = get_str(map, "name")
        .ok_or(GroupMarkdownError::MissingField("name"))?
        .to_string();

    let aliases = get_string_array(map, "aliases").unwrap_or_default();
    let summary = get_str(map, "summary").map(|s| s.trim().to_string()).unwrap_or_default();
    let tags = get_string_array(map, "tags").unwrap_or_default();

    let temporal = parse_temporal(map.get("temporal"))?;
    let members = parse_members(map.get("members"))?;
    let headquarters = get_str(map, "headquarters").map(|s| s.to_string());
    let parent_group =
        get_str(map, "parent_group").map(|s| GroupId::new(s.to_string()));
    let allied_groups = get_string_array(map, "allied_groups")
        .unwrap_or_default()
        .into_iter()
        .map(GroupId::new)
        .collect();
    let rival_groups = get_string_array(map, "rival_groups")
        .unwrap_or_default()
        .into_iter()
        .map(GroupId::new)
        .collect();
    let extras = parse_extras_map(map.get("extras"));
    let body_sections = parse_h2_sections(&fm.body);

    Ok(Group {
        id: GroupId::new(id),
        kind,
        name,
        aliases,
        summary,
        tags,
        extras,
        body_sections,
        temporal,
        members,
        headquarters,
        parent_group,
        allied_groups,
        rival_groups,
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

fn parse_temporal(v: Option<&YamlValue>) -> Result<Temporal, GroupMarkdownError> {
    let Some(v) = v else { return Ok(Temporal::default()); };
    if v.is_null() {
        return Ok(Temporal::default());
    }
    let map = v
        .as_mapping()
        .ok_or(GroupMarkdownError::TypeMismatch {
            field: "temporal",
            expected: "mapping",
        })?;
    let founded_at = get_str(map, "founded_at").map(|s| s.trim().to_string());
    let dissolved_at = get_str(map, "dissolved_at").map(|s| s.trim().to_string());
    let status = match get_str(map, "status") {
        Some(s) => GroupStatus::from_str_loose(s).ok_or(
            GroupMarkdownError::TypeMismatch {
                field: "temporal.status",
                expected: "active|declining|dissolved|dormant",
            },
        )?,
        None => GroupStatus::default(),
    };
    let notes = get_str(map, "notes").map(|s| s.trim().to_string());
    Ok(Temporal {
        founded_at,
        dissolved_at,
        status,
        notes,
    })
}

fn parse_members(v: Option<&YamlValue>) -> Result<Vec<MemberRef>, GroupMarkdownError> {
    let Some(v) = v else { return Ok(Vec::new()); };
    if v.is_null() {
        return Ok(Vec::new());
    }
    let seq = v
        .as_sequence()
        .ok_or(GroupMarkdownError::TypeMismatch {
            field: "members",
            expected: "sequence",
        })?;
    let mut out = Vec::with_capacity(seq.len());
    for item in seq {
        let map = item
            .as_mapping()
            .ok_or(GroupMarkdownError::TypeMismatch {
                field: "members[*]",
                expected: "mapping",
            })?;
        let person_id = get_str(map, "person_id").map(|s| s.to_string());
        let display_name = get_str(map, "display_name").map(|s| s.to_string());
        let role = get_str(map, "role")
            .ok_or(GroupMarkdownError::MissingField("members[*].role"))?
            .to_string();
        let note = get_str(map, "note").map(|s| s.to_string());
        out.push(MemberRef {
            person_id,
            display_name,
            role,
            note,
        });
    }
    Ok(out)
}

fn parse_extras_map(v: Option<&YamlValue>) -> Map<String, JsonValue> {
    let Some(v) = v else { return Map::new(); };
    if v.is_null() {
        return Map::new();
    }
    // YAML → JSON 변환 후 Object만 채택.
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

    // 장르 중립 fixture — wuxia·판타지·SF 어휘 없이 frontmatter 모든 필드를 시연.
    // 도메인 시나리오는 `tests/world_chilguk_chunchu_e2e.rs`에서 실제 SoT 파일로 검증.
    const NEUTRAL_FIXTURE: &str = r#"---
id: group-alpha
kind: alliance
name: Alpha Council
aliases: [The Council, Alpha Pact]
summary: |
  Multi-line summary describing the alliance's purpose and scope across two
  short sentences. Used to verify block-scalar handling.
tags: [test, alliance, sample]
temporal:
  founded_at: Year 0 (origin)
  dissolved_at: ~
  status: declining
  notes: |
    Founded at origin; experienced two periods of internal strife.
members:
  - person_id: person-a01
    display_name: Alice
    role: Leader (chair)
  - person_id: person-a02
    display_name: Bob
    role: Officer
    note: '"trusted" — operations lead'
headquarters: place-alpha-hq
parent_group: ~
allied_groups: []
rival_groups: []
extras:
  alignment: orthodox
  founder: Alice
  charter_year: 0
---

## Overview
Prose paragraph describing the alliance.

## Power Structure
Alice chairs; Bob leads operations.
"#;

    #[test]
    fn parse_neutral_fixture_full_roundtrip() {
        let g = group_from_markdown(NEUTRAL_FIXTURE).expect("파싱 성공");
        assert_eq!(g.id.as_str(), "group-alpha");
        assert_eq!(g.kind, "alliance");
        assert_eq!(g.name, "Alpha Council");
        assert_eq!(
            g.aliases,
            vec!["The Council".to_string(), "Alpha Pact".to_string()]
        );
        assert!(g.summary.contains("Multi-line"));
        assert_eq!(g.tags, vec!["test", "alliance", "sample"]);

        // temporal
        assert_eq!(g.temporal.founded_at.as_deref(), Some("Year 0 (origin)"));
        assert!(g.temporal.dissolved_at.is_none());
        assert_eq!(g.temporal.status, GroupStatus::Declining);
        assert!(g.temporal.notes.as_deref().unwrap().contains("Founded"));

        // members
        assert_eq!(g.members.len(), 2);
        assert_eq!(g.members[0].person_id.as_deref(), Some("person-a01"));
        assert_eq!(g.members[0].role, "Leader (chair)");
        assert_eq!(g.members[1].note.as_deref(), Some("\"trusted\" — operations lead"));

        // 외래키 텍스트
        assert_eq!(g.headquarters.as_deref(), Some("place-alpha-hq"));
        assert!(g.parent_group.is_none());
        assert!(g.allied_groups.is_empty());

        // extras
        assert_eq!(g.alignment(), Some("orthodox"));
        assert_eq!(
            g.extras.get("founder").and_then(|v| v.as_str()),
            Some("Alice")
        );

        // body sections
        assert!(g.body_sections.contains_key("Overview"));
        assert!(g.body_sections.contains_key("Power Structure"));
        assert!(g.body_sections["Overview"].contains("Prose"));
    }

    #[test]
    fn missing_required_field_errs() {
        let md = "---\nname: 테스트\nkind: alliance\n---\n";
        assert!(matches!(
            group_from_markdown(md),
            Err(GroupMarkdownError::MissingField("id"))
        ));
    }

    #[test]
    fn invalid_status_errs() {
        let md = "---\nid: group-x\nkind: alliance\nname: x\ntemporal:\n  status: forever\n---\n";
        assert!(matches!(
            group_from_markdown(md),
            Err(GroupMarkdownError::TypeMismatch { field: "temporal.status", .. })
        ));
    }

    #[test]
    fn parent_group_parses_as_groupid() {
        let md = "---\nid: group-x\nkind: alliance\nname: x\nparent_group: group-parent\n---\n";
        let g = group_from_markdown(md).unwrap();
        assert_eq!(g.parent_group, Some(GroupId::new("group-parent")));
    }

    #[test]
    fn rival_and_allied_arrays_become_groupids() {
        let md = "---\nid: group-x\nkind: alliance\nname: x\nallied_groups: [group-a, group-b]\nrival_groups: [group-c]\n---\n";
        let g = group_from_markdown(md).unwrap();
        assert_eq!(
            g.allied_groups,
            vec![GroupId::new("group-a"), GroupId::new("group-b")]
        );
        assert_eq!(g.rival_groups, vec![GroupId::new("group-c")]);
    }
}
