//! Atlas 마크다운 → `Atlas` 애그리거트 변환.
//!
//! 입력 frontmatter 스키마는 `docs/tasks/task-phase4-atlas-vertical-slice.md` §6.1을 따름.
//!
//! **핵심**: `## 배치 다이어그램` 섹션 안의 ASCII art 코드블록(```...```)이 byte-exact
//! 보존되어야 한다. `parse_h2_sections`가 펜스 안에서 `## ` 헤더를 무시하므로 깨지지 않는다.

use std::collections::BTreeMap;

use serde_json::{Map, Value as JsonValue};
use serde_yaml::Value as YamlValue;

use crate::domain::world::{Atlas, AtlasExtent, AtlasId, PlaceId};

use super::frontmatter::{FrontmatterError, parse_frontmatter, parse_h2_sections};

#[derive(Debug, thiserror::Error)]
pub enum AtlasMarkdownError {
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

/// 마크다운 텍스트를 Atlas로 변환. `source_path`는 호출자가 setter로 주입.
pub fn atlas_from_markdown(md: &str) -> Result<Atlas, AtlasMarkdownError> {
    let fm = parse_frontmatter(md)?;
    let map = fm
        .value
        .as_mapping()
        .ok_or(AtlasMarkdownError::MissingField("frontmatter (mapping)"))?;

    let id = get_str(map, "id")
        .ok_or(AtlasMarkdownError::MissingField("id"))?
        .to_string();
    let kind = get_str(map, "kind")
        .ok_or(AtlasMarkdownError::MissingField("kind"))?
        .to_string();
    let name = get_str(map, "name")
        .ok_or(AtlasMarkdownError::MissingField("name"))?
        .to_string();

    let aliases = get_string_array(map, "aliases").unwrap_or_default();
    let summary = get_str(map, "summary")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let tags = get_string_array(map, "tags").unwrap_or_default();
    let extras = parse_extras_map(map.get(&YamlValue::from("extras")));
    let extent = parse_extent(map.get(&YamlValue::from("extent")))?;
    let references = get_string_array(map, "references")
        .unwrap_or_default()
        .into_iter()
        .map(PlaceId::new)
        .collect();
    let body_sections: BTreeMap<String, String> = parse_h2_sections(&fm.body);

    Ok(Atlas {
        id: AtlasId::new(id),
        kind,
        name,
        aliases,
        summary,
        tags,
        extras,
        extent,
        references,
        body_sections,
        source_path: None,
    })
}

// ---------------------------------------------------------------------------
// YAML 추출 헬퍼 — group/person/place 파서와 동일 패턴.
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

fn parse_extent(v: Option<&YamlValue>) -> Result<AtlasExtent, AtlasMarkdownError> {
    let Some(v) = v else { return Ok(AtlasExtent::default()); };
    if v.is_null() {
        return Ok(AtlasExtent::default());
    }
    let map = v
        .as_mapping()
        .ok_or(AtlasMarkdownError::TypeMismatch {
            field: "extent",
            expected: "mapping",
        })?;
    let projection = get_str(map, "projection")
        .map(str::to_string)
        .unwrap_or_else(|| "schematic".to_string());
    let unit = get_str(map, "unit")
        .map(str::to_string)
        .unwrap_or_else(|| "schematic".to_string());
    let width_units = map
        .get(YamlValue::from("width_units"))
        .and_then(|v| v.as_u64())
        .map(|n| n as u32);
    let height_units = map
        .get(YamlValue::from("height_units"))
        .and_then(|v| v.as_u64())
        .map(|n| n as u32);
    Ok(AtlasExtent {
        projection,
        width_units,
        height_units,
        unit,
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

    // 장르 중립 fixture — 무협 어휘 없음. ASCII 다이어그램 byte-exact 보존 검증.
    const NEUTRAL_ATLAS: &str = "---
id: atlas-test-continent
kind: continent
name: Test Continent
aliases: [The Realm Map, Schematic Test]
summary: |
  Multi-line atlas summary.
tags: [test, atlas, continent]
extras:
  era: Current Era (year 270)
  era_id: ~
extent:
  projection: schematic
  width_units: 7
  height_units: 7
  unit: schematic
references:
  - place-alpha
  - place-beta
  - place-gamma
---

## Overview
Continent overview prose.

## 배치 다이어그램
```
┌──────┐
│ NORTH│
└──┬───┘
   │
┌──┴───┐
│ SOUTH│
└──────┘
```

## Notes
Trailing prose.
";

    #[test]
    fn parse_neutral_atlas_full_roundtrip() {
        let a = atlas_from_markdown(NEUTRAL_ATLAS).expect("파싱 성공");
        assert_eq!(a.id.as_str(), "atlas-test-continent");
        assert_eq!(a.kind, "continent");
        assert_eq!(a.name, "Test Continent");
        assert_eq!(a.aliases, vec!["The Realm Map", "Schematic Test"]);
        assert!(a.summary.contains("Multi-line"));
        assert!(a.tags.contains(&"continent".to_string()));

        // extras
        assert_eq!(a.era(), Some("Current Era (year 270)"));
        assert!(a.era_id().is_none());

        // extent
        assert_eq!(a.extent.projection, "schematic");
        assert_eq!(a.extent.width_units, Some(7));
        assert_eq!(a.extent.height_units, Some(7));
        assert_eq!(a.extent.unit, "schematic");

        // references — 작성 순서 보존.
        assert_eq!(
            a.references,
            vec![
                PlaceId::new("place-alpha"),
                PlaceId::new("place-beta"),
                PlaceId::new("place-gamma"),
            ]
        );

        // body sections
        assert!(a.body_sections.contains_key("Overview"));
        assert!(a.body_sections.contains_key("배치 다이어그램"));
        assert!(a.body_sections.contains_key("Notes"));
    }

    #[test]
    fn ascii_diagram_preserved_byte_exact() {
        // box-drawing 문자, 빈 줄, 들여쓰기 — 코드블록 안에 들어 있어야 byte-exact.
        // 마크다운 펜스(```) 자체는 본문에 포함되며, 그 사이의 모든 라인은 변형 없이 보존.
        let a = atlas_from_markdown(NEUTRAL_ATLAS).expect("파싱 성공");
        let diagram = a
            .body_sections
            .get("배치 다이어그램")
            .expect("배치 다이어그램 섹션 필요");
        // 펜스 보존.
        assert!(diagram.starts_with("```"));
        assert!(diagram.ends_with("```"));
        // box-drawing 보존.
        assert!(diagram.contains("┌──────┐"));
        assert!(diagram.contains("│ NORTH│"));
        assert!(diagram.contains("└──────┘"));
        // 본문 안의 ## 가짜 헤더 같은 게 빠지지 않았는지 (NEUTRAL_ATLAS엔 없으나 회귀 의식).
    }

    #[test]
    fn ascii_diagram_with_inner_hash_lines_not_split_into_h2() {
        // 코드블록 안에 `## fake header`가 있어도 새 H2로 분리되지 않아야 함.
        let md = r#"---
id: atlas-x
kind: continent
name: X
---

## 배치 다이어그램
```
## not a real header
just diagram art
```

## Real Section
real prose
"#;
        let a = atlas_from_markdown(md).unwrap();
        assert!(a.body_sections.contains_key("배치 다이어그램"));
        assert!(a.body_sections.contains_key("Real Section"));
        assert!(!a.body_sections.contains_key("not a real header"));
        assert!(
            a.body_sections["배치 다이어그램"].contains("## not a real header"),
            "코드블록 안의 ## 가짜 헤더는 본문으로 보존되어야 함"
        );
    }

    #[test]
    fn missing_required_id_errs() {
        let md = "---\nkind: continent\nname: x\n---\n";
        assert!(matches!(
            atlas_from_markdown(md),
            Err(AtlasMarkdownError::MissingField("id"))
        ));
    }

    #[test]
    fn missing_required_kind_errs() {
        let md = "---\nid: atlas-x\nname: x\n---\n";
        assert!(matches!(
            atlas_from_markdown(md),
            Err(AtlasMarkdownError::MissingField("kind"))
        ));
    }

    #[test]
    fn missing_required_name_errs() {
        let md = "---\nid: atlas-x\nkind: continent\n---\n";
        assert!(matches!(
            atlas_from_markdown(md),
            Err(AtlasMarkdownError::MissingField("name"))
        ));
    }

    #[test]
    fn empty_extent_yields_default_schematic() {
        let md = "---\nid: atlas-x\nkind: continent\nname: x\n---\n";
        let a = atlas_from_markdown(md).unwrap();
        assert_eq!(a.extent, AtlasExtent::default());
    }

    #[test]
    fn null_extent_yields_default() {
        let md = "---\nid: atlas-x\nkind: continent\nname: x\nextent: ~\n---\n";
        let a = atlas_from_markdown(md).unwrap();
        assert_eq!(a.extent, AtlasExtent::default());
    }

    #[test]
    fn empty_references_yields_empty_vec() {
        let md = "---\nid: atlas-x\nkind: continent\nname: x\nreferences: []\n---\n";
        let a = atlas_from_markdown(md).unwrap();
        assert!(a.references.is_empty());
    }

    #[test]
    fn references_preserve_input_order() {
        let md = "---\nid: atlas-x\nkind: continent\nname: x\nreferences:\n  - place-c\n  - place-a\n  - place-b\n---\n";
        let a = atlas_from_markdown(md).unwrap();
        assert_eq!(
            a.references,
            vec![
                PlaceId::new("place-c"),
                PlaceId::new("place-a"),
                PlaceId::new("place-b"),
            ],
            "references는 작성 순서 그대로 보존되어야 함 (좌상→우하 같은 의도된 순서)"
        );
    }

    #[test]
    fn era_id_text_preserved_for_phase5() {
        // Phase 4엔 era_id 텍스트만 보존 (검증 비활성). Phase 5에서 외래키 활성.
        let md = "---\nid: atlas-x\nkind: continent\nname: x\nextras:\n  era: 현재\n  era_id: era-current\n---\n";
        let a = atlas_from_markdown(md).unwrap();
        assert_eq!(a.era(), Some("현재"));
        assert_eq!(a.era_id(), Some("era-current"));
    }
}
