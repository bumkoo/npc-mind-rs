//! Place 마크다운 → `Place` 애그리거트 변환.
//!
//! 입력 frontmatter 스키마는 `docs/tasks/task-phase3-place-vertical-slice.md` §6.1을 따름.
//!
//! 두 layer (settlement·geography)를 같은 파서로 처리. layer별 권장 H2 섹션은 다르나
//! 파서는 layer를 보지 않고 frontmatter+H2를 그대로 보존한다 — layer 분기 검증은
//! 호출자(world-load)가 책임.

use std::collections::BTreeMap;

use serde_json::{Map, Value as JsonValue};
use serde_yaml::Value as YamlValue;

use crate::domain::world::{Place, PlaceId, PlaceLayer, Spatial};

use super::frontmatter::{FrontmatterError, parse_frontmatter, parse_h2_sections};

#[derive(Debug, thiserror::Error)]
pub enum PlaceMarkdownError {
    #[error(transparent)]
    Frontmatter(#[from] FrontmatterError),
    #[error("필수 필드 누락: {0}")]
    MissingField(&'static str),
    #[error("필드 '{field}' 타입 불일치 (기대: {expected})")]
    TypeMismatch {
        field: &'static str,
        expected: &'static str,
    },
    #[error("layer '{value}' 알 수 없음 (기대: settlement|geography)")]
    InvalidLayer { value: String },
}

/// 마크다운 텍스트를 Place로 변환. `source_path`는 호출자가 setter로 주입.
pub fn place_from_markdown(md: &str) -> Result<Place, PlaceMarkdownError> {
    let fm = parse_frontmatter(md)?;
    let map = fm
        .value
        .as_mapping()
        .ok_or(PlaceMarkdownError::MissingField("frontmatter (mapping)"))?;

    let id = get_str(map, "id")
        .ok_or(PlaceMarkdownError::MissingField("id"))?
        .to_string();
    let kind = get_str(map, "kind")
        .ok_or(PlaceMarkdownError::MissingField("kind"))?
        .to_string();
    let name = get_str(map, "name")
        .ok_or(PlaceMarkdownError::MissingField("name"))?
        .to_string();
    let layer_str = get_str(map, "layer")
        .ok_or(PlaceMarkdownError::MissingField("layer"))?;
    let layer = PlaceLayer::from_str_loose(layer_str).ok_or(PlaceMarkdownError::InvalidLayer {
        value: layer_str.to_string(),
    })?;

    let aliases = get_string_array(map, "aliases").unwrap_or_default();
    let summary = get_str(map, "summary")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let tags = get_string_array(map, "tags").unwrap_or_default();
    let extras = parse_extras_map(map.get("extras"));
    let spatial = parse_spatial(map.get("spatial"))?;
    let body_sections: BTreeMap<String, String> = parse_h2_sections(&fm.body);

    Ok(Place {
        id: PlaceId::new(id),
        layer,
        kind,
        name,
        aliases,
        summary,
        tags,
        extras,
        body_sections,
        spatial,
        source_path: None,
    })
}

// ---------------------------------------------------------------------------
// YAML 추출 헬퍼 — group/person 파서와 동일 패턴.
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

fn parse_spatial(v: Option<&YamlValue>) -> Result<Spatial, PlaceMarkdownError> {
    let Some(v) = v else { return Ok(Spatial::default()); };
    if v.is_null() {
        return Ok(Spatial::default());
    }
    let map = v
        .as_mapping()
        .ok_or(PlaceMarkdownError::TypeMismatch {
            field: "spatial",
            expected: "mapping",
        })?;
    let parent_place = get_str(map, "parent_place").map(|s| PlaceId::new(s.to_string()));
    let relative_position = get_str(map, "relative_position").map(|s| s.trim().to_string());
    let bordering_places = get_string_array(map, "bordering_places")
        .unwrap_or_default()
        .into_iter()
        .map(PlaceId::new)
        .collect();
    let geography_refs = get_string_array(map, "geography_refs")
        .unwrap_or_default()
        .into_iter()
        .map(PlaceId::new)
        .collect();
    Ok(Spatial {
        parent_place,
        relative_position,
        bordering_places,
        geography_refs,
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

    // 장르 중립 fixture — 두 layer를 같은 파서로 처리.
    const NEUTRAL_SETTLEMENT: &str = r#"---
id: place-alpha
layer: settlement
kind: nation
name: Alpha Realm
aliases: [The Realm, Alpha Crown]
summary: |
  Multi-line summary describing the realm.
tags: [test, place, settlement, nation]
extras:
  capital: Alpha City
  population_note: largest in region
  controlling_group: group-alpha-court
spatial:
  parent_place: ~
  relative_position: center
  bordering_places: [place-beta, place-gamma]
  geography_refs: [place-alpha-plain]
---

## Overview
Prose paragraph.

## Governance
Power structure prose.
"#;

    const NEUTRAL_GEOGRAPHY: &str = r#"---
id: place-mt-alpha
layer: geography
kind: mountain-range
name: Alpha Mountains
aliases: [The Spine]
summary: |
  Mountain range on the western frontier.
tags: [test, place, geography, mountain-range]
extras:
  terrain_type: mountain-range
  climate: alpine, heavy winter snow
  hazards: [avalanche, fog]
  signature_features: [North Peak, Twin Pass]
spatial:
  parent_place: ~
  relative_position: west
  bordering_places: [place-alpha]
---

## Overview
Mountain prose.

## Terrain
Detailed terrain.
"#;

    #[test]
    fn parse_settlement_fixture_full_roundtrip() {
        let p = place_from_markdown(NEUTRAL_SETTLEMENT).expect("파싱 성공");
        assert_eq!(p.id.as_str(), "place-alpha");
        assert_eq!(p.layer, PlaceLayer::Settlement);
        assert_eq!(p.kind, "nation");
        assert_eq!(p.name, "Alpha Realm");
        assert_eq!(p.aliases, vec!["The Realm", "Alpha Crown"]);
        assert!(p.summary.contains("Multi-line"));
        assert!(p.tags.contains(&"settlement".to_string()));

        // extras
        assert_eq!(
            p.extras.get("capital").and_then(|v| v.as_str()),
            Some("Alpha City")
        );
        assert_eq!(p.controlling_group(), Some("group-alpha-court"));

        // spatial
        assert!(p.spatial.parent_place.is_none());
        assert_eq!(p.spatial.relative_position.as_deref(), Some("center"));
        assert_eq!(p.spatial.bordering_places.len(), 2);
        assert_eq!(p.spatial.bordering_places[0], PlaceId::new("place-beta"));
        assert_eq!(p.spatial.geography_refs, vec![PlaceId::new("place-alpha-plain")]);

        // body sections
        assert!(p.body_sections.contains_key("Overview"));
        assert!(p.body_sections.contains_key("Governance"));
    }

    #[test]
    fn parse_geography_fixture_full_roundtrip() {
        let p = place_from_markdown(NEUTRAL_GEOGRAPHY).expect("파싱 성공");
        assert_eq!(p.layer, PlaceLayer::Geography);
        assert_eq!(p.kind, "mountain-range");
        assert_eq!(p.name, "Alpha Mountains");

        // extras (geography 특화)
        assert_eq!(
            p.extras.get("terrain_type").and_then(|v| v.as_str()),
            Some("mountain-range")
        );
        // hazards는 sequence — JSON 배열로 보존.
        let hazards = p.extras.get("hazards").and_then(|v| v.as_array()).unwrap();
        assert_eq!(hazards.len(), 2);

        // spatial
        assert_eq!(p.spatial.relative_position.as_deref(), Some("west"));
        assert_eq!(p.spatial.bordering_places, vec![PlaceId::new("place-alpha")]);
        // geography는 geography_refs를 비움 (settlement에서만 의미).
        assert!(p.spatial.geography_refs.is_empty());

        assert!(p.body_sections.contains_key("Overview"));
        assert!(p.body_sections.contains_key("Terrain"));
    }

    #[test]
    fn missing_required_id_errs() {
        let md = "---\nlayer: settlement\nkind: nation\nname: x\n---\n";
        assert!(matches!(
            place_from_markdown(md),
            Err(PlaceMarkdownError::MissingField("id"))
        ));
    }

    #[test]
    fn missing_required_layer_errs() {
        let md = "---\nid: place-x\nkind: nation\nname: x\n---\n";
        assert!(matches!(
            place_from_markdown(md),
            Err(PlaceMarkdownError::MissingField("layer"))
        ));
    }

    #[test]
    fn invalid_layer_errs() {
        let md = "---\nid: place-x\nlayer: city\nkind: nation\nname: x\n---\n";
        match place_from_markdown(md) {
            Err(PlaceMarkdownError::InvalidLayer { value }) => assert_eq!(value, "city"),
            other => panic!("InvalidLayer expected, got: {other:?}"),
        }
    }

    #[test]
    fn empty_spatial_yields_default() {
        let md = "---\nid: place-x\nlayer: geography\nkind: jungle\nname: x\n---\n";
        let p = place_from_markdown(md).unwrap();
        assert_eq!(p.spatial, Spatial::default());
    }

    #[test]
    fn null_spatial_yields_default() {
        let md = "---\nid: place-x\nlayer: settlement\nkind: city\nname: x\nspatial: ~\n---\n";
        let p = place_from_markdown(md).unwrap();
        assert_eq!(p.spatial, Spatial::default());
    }

    #[test]
    fn parent_place_parses_as_placeid() {
        let md = "---\nid: place-x\nlayer: settlement\nkind: city\nname: x\nspatial:\n  parent_place: place-parent\n---\n";
        let p = place_from_markdown(md).unwrap();
        assert_eq!(p.spatial.parent_place, Some(PlaceId::new("place-parent")));
    }

    #[test]
    fn sect_kind_with_controlling_group_extras() {
        // sect 이중 등록 — extras.controlling_group으로 Group 외래키 시연.
        let md = r#"---
id: place-namgung-sega
layer: settlement
kind: sect
name: 남궁세가
extras:
  controlling_group: group-namgung
spatial:
  parent_place: place-namgung
---
"#;
        let p = place_from_markdown(md).unwrap();
        assert_eq!(p.kind, "sect");
        assert_eq!(p.controlling_group(), Some("group-namgung"));
        assert_eq!(p.spatial.parent_place, Some(PlaceId::new("place-namgung")));
    }
}
