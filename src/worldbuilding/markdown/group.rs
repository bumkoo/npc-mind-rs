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

    let temporal = parse_temporal(map.get(&YamlValue::from("temporal")))?;
    let members = parse_members(map.get(&YamlValue::from("members")))?;
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
    let extras = parse_extras_map(map.get(&YamlValue::from("extras")));
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

    const DAEJIN_FIXTURE: &str = r#"---
id: group-daejin-court
kind: dynasty-court
name: 대진 황실
aliases: [낙양 조정, 중원 황실]
summary: |
  270년 전 통일제국의 후예. 천순제는 꼭두각시이며 실권은 조고가 잡고 있다.
tags: [wuxia, group, dynasty]
temporal:
  founded_at: 원년 (270년 전)
  dissolved_at: ~
  status: declining
  notes: |
    270년 전 통일제국으로 출발.
members:
  - person_id: npc-07
    display_name: 천순제
    role: 황제 (꼭두각시)
  - person_id: npc-02
    display_name: 조고
    role: 실권자
    note: 환관, 십상시 수장
headquarters: place-daejin-luoyang
parent_group: ~
allied_groups: []
rival_groups: []
extras:
  alignment: imperial
  shadow_ruler: 조고
  capital: 낙양(洛陽)
---

## 개요
산문 — 대진 황실은 270년 전 통일제국의 후예다.

## 권력 구조
천순제 ↔ 조고 ↔ 십상시.
"#;

    #[test]
    fn parse_daejin_fixture_full_roundtrip() {
        let g = group_from_markdown(DAEJIN_FIXTURE).expect("파싱 성공");
        assert_eq!(g.id.as_str(), "group-daejin-court");
        assert_eq!(g.kind, "dynasty-court");
        assert_eq!(g.name, "대진 황실");
        assert_eq!(g.aliases, vec!["낙양 조정".to_string(), "중원 황실".to_string()]);
        assert!(g.summary.contains("꼭두각시"));
        assert_eq!(g.tags, vec!["wuxia", "group", "dynasty"]);

        // temporal
        assert_eq!(g.temporal.founded_at.as_deref(), Some("원년 (270년 전)"));
        assert!(g.temporal.dissolved_at.is_none());
        assert_eq!(g.temporal.status, GroupStatus::Declining);
        assert!(g.temporal.notes.as_deref().unwrap().contains("270년 전"));

        // members
        assert_eq!(g.members.len(), 2);
        assert_eq!(g.members[0].person_id.as_deref(), Some("npc-07"));
        assert_eq!(g.members[0].role, "황제 (꼭두각시)");
        assert_eq!(g.members[1].note.as_deref(), Some("환관, 십상시 수장"));

        // 외래키 텍스트
        assert_eq!(g.headquarters.as_deref(), Some("place-daejin-luoyang"));
        assert!(g.parent_group.is_none());
        assert!(g.allied_groups.is_empty());

        // extras
        assert_eq!(g.alignment(), Some("imperial"));
        assert_eq!(
            g.extras.get("shadow_ruler").and_then(|v| v.as_str()),
            Some("조고")
        );

        // body sections
        assert!(g.body_sections.contains_key("개요"));
        assert!(g.body_sections.contains_key("권력 구조"));
        assert!(g.body_sections["개요"].contains("270년 전"));
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
