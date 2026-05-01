//! Place 애그리거트 — Phase 3 Vertical Slice.
//!
//! **장르 중립 원칙**: 이 모듈은 wuxia/판타지/SF 어떤 어휘도 모른다. `kind`는
//! free-form `String`이며, 장르가 채운다 (`genres/wuxia/forms/place.toml`).
//!
//! **두 layer**: Settlement(공동체 관리 공간 — 국가·도시·문파)와 Geography(자연
//! 지형 — 산악·해안·밀림). 같은 좌표 위에 두 결이 포개지며, Settlement가 자기 위치한
//! Geography를 `spatial.geography_refs`로 참조한다 (Era overlay의 기반 — Phase 5+).
//!
//! Phase 3 외래키:
//! - `Group.headquarters` ↔ `Place.id` (활성 — Phase 1·2 텍스트 보존에서 승급)
//! - `Person.birthplace`/`current_location` ↔ `Place.id` (활성)
//! - `Place.spatial.parent_place` cycle (활성 — 같은 도메인 내, Phase 1 group과 동일 패턴)
//! - `Place.spatial.bordering_places`/`geography_refs` 존재 (활성)
//! - `Place.extras.controlling_group` ↔ `Group.id` (활성 — sect kind 이중 등록)

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use super::group::WorldError;

/// 장소 식별자 — `place-{slug}` 형식. slug는 ASCII 소문자·숫자·하이픈.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlaceId(pub String);

impl PlaceId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PlaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for PlaceId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for PlaceId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// 장소의 계층 — 같은 좌표 위에 포개지는 두 결.
///
/// - `Settlement`: 공동체 관리 공간. 국가·도시·자치령·문파 등 인간 시간 단위로 변동.
/// - `Geography`: 자연 지형. 산악·해안·밀림·초원·사막 등 지질학적 시간 단위, 거의 안 변함.
///
/// Settlement는 자기가 위치한 Geography를 `spatial.geography_refs`로 참조한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaceLayer {
    Settlement,
    Geography,
}

impl PlaceLayer {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Settlement => "settlement",
            Self::Geography => "geography",
        }
    }

    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "settlement" => Some(Self::Settlement),
            "geography" => Some(Self::Geography),
            _ => None,
        }
    }
}

/// 공간성 — 수직 포함(parent_place) + 수평 인접(bordering_places) + layer 간 layered
/// 관계(geography_refs).
///
/// `relative_position`은 schematic 위치 라벨("center"/"west"/"south-west" 등)이며
/// Phase 4 Atlas에서 다이어그램·맵 배치에 활용한다. Phase 3엔 텍스트만 보존.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Spatial {
    /// 수직 포함 (영토상 1:1). 도시→국가, 문파→국가, 광역 영역. cycle 검증 활성.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_place: Option<PlaceId>,
    /// schematic 위치 라벨 — Phase 4 Atlas 다이어그램에서 활용.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_position: Option<String>,
    /// 수평 인접 Place들. 같은 도메인 내 외래키 검증.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bordering_places: Vec<PlaceId>,
    /// (Settlement만 의미 있음) 어느 자연 지형 위에 layered. 외래키 검증 +
    /// 대상 layer가 `Geography`인지 검증.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub geography_refs: Vec<PlaceId>,
}

/// Place 애그리거트.
///
/// 핵심 책임:
/// - 정체성: id/name/aliases/kind + layer (일급 enum)
/// - 공간성: spatial (parent/bordering/geography_refs/relative_position)
/// - 자유 본문: body_sections (h2 헤더 → 본문)
/// - 장르 확장: extras (장르가 채우는 free-form JSON map; capital·climate·hazards·
///   controlling_group 등)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Place {
    pub id: PlaceId,
    pub layer: PlaceLayer,
    /// 장르가 채움 (Phase 3 wuxia: settlement={`nation`,`autonomous-zone`,`city`,`sect`},
    /// geography={`mountain-range`,`coast`,`jungle`,`grassland`,`desert`,`forest`,
    /// `river`,`lake`,`landmark`}).
    pub kind: String,
    pub name: String,
    /// 별호·옛 이름·자(字). 예: `["낙양","중원 황도"]`. FTS5 검색 대상에 포함.
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 장르 자유 확장. wuxia 예: `capital`/`ki_concentration`/`climate`/`hazards`/
    /// `controlling_group`(sect kind에서 Group ID 참조).
    #[serde(default)]
    pub extras: Map<String, Value>,
    /// H2 섹션 본문. `BTreeMap`이라 알파벳 정렬 순서로 보존되며, 작성 순서는
    /// 보존되지 않는다 (Group과 동일 정책).
    #[serde(default)]
    pub body_sections: BTreeMap<String, String>,
    #[serde(default)]
    pub spatial: Spatial,
    /// 마크다운 SoT 경로 (절대 또는 프로젝트 root 기준 상대).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}

impl Place {
    /// 최소 생성자. 테스트·도구용.
    pub fn new(
        id: impl Into<PlaceId>,
        layer: PlaceLayer,
        kind: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            layer,
            kind: kind.into(),
            name: name.into(),
            aliases: Vec::new(),
            summary: String::new(),
            tags: Vec::new(),
            extras: Map::new(),
            body_sections: BTreeMap::new(),
            spatial: Spatial::default(),
            source_path: None,
        }
    }

    /// `extras["controlling_group"]`을 String으로 추출 (없거나 비문자열이면 None).
    /// sect kind 이중 등록 패턴에서 Place(공간)→Group(조직) 외래키.
    pub fn controlling_group(&self) -> Option<&str> {
        self.extras.get("controlling_group").and_then(|v| v.as_str())
    }
}

/// 리스트 필터 — `WorldRepository::list_places`에 전달.
#[derive(Debug, Clone, Default)]
pub struct PlaceFilter {
    pub layer: Option<PlaceLayer>,
    pub kind: Option<String>,
    pub parent_place: Option<PlaceId>,
    /// `tags` 토큰 매칭. `wuxia`/`central-plain` 등.
    pub genre_tag: Option<String>,
}

// ---------------------------------------------------------------------------
// parent_place cycle 검증 — group의 detect_parent_group_cycle와 동일 알고리즘.
// ---------------------------------------------------------------------------

/// `places` 컬렉션 내 `parent_place` 사슬을 DFS로 따라가 자기 자신에 도달하는지 검사.
///
/// 결과는 canonical 형태(가장 작은 ID에서 시작)로 정렬되어 deterministic. 외래키
/// 결손(parent_place가 places에 없는 ID)은 cycle로 간주하지 않고 그냥 dangling으로
/// 끝낸다 — 결손 경고는 별도로 수집.
pub fn detect_parent_place_cycle(places: &[Place]) -> Vec<Vec<PlaceId>> {
    let by_id: HashMap<&PlaceId, &Place> = places.iter().map(|p| (&p.id, p)).collect();
    let mut seen_cycles: BTreeSet<Vec<PlaceId>> = BTreeSet::new();
    let mut sorted: Vec<&PlaceId> = by_id.keys().copied().collect();
    sorted.sort();
    for start in sorted {
        let mut visited: HashSet<&PlaceId> = HashSet::new();
        let mut path: Vec<PlaceId> = Vec::new();
        let mut cur: Option<&PlaceId> = Some(start);
        while let Some(id) = cur {
            if visited.contains(&id) {
                if let Some(idx) = path.iter().position(|p| p == id) {
                    let mut cyc: Vec<PlaceId> = path[idx..].to_vec();
                    rotate_to_min(&mut cyc);
                    seen_cycles.insert(cyc);
                }
                break;
            }
            visited.insert(id);
            path.push(id.clone());
            cur = by_id
                .get(id)
                .and_then(|p| p.spatial.parent_place.as_ref())
                .filter(|pid| by_id.contains_key(pid));
        }
    }
    seen_cycles.into_iter().collect()
}

fn rotate_to_min(cycle: &mut Vec<PlaceId>) {
    if cycle.is_empty() {
        return;
    }
    let mut min_idx = 0;
    for (i, id) in cycle.iter().enumerate() {
        if id < &cycle[min_idx] {
            min_idx = i;
        }
    }
    cycle.rotate_left(min_idx);
}

/// `WorldError`는 group.rs의 enum을 재사용. Place 도메인의 cycle도 `ParentCycle`
/// variant에 path로 표현된다 — 호출자가 path 형태("place-a → place-b → place-a")로
/// 구분 가능.
#[allow(dead_code)]
fn _world_error_marker(_e: WorldError) {}

// ---------------------------------------------------------------------------
// 단위 테스트
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn p_with_parent(id: &str, parent: Option<&str>) -> Place {
        let mut p = Place::new(id, PlaceLayer::Settlement, "city", id);
        p.spatial.parent_place = parent.map(PlaceId::new);
        p
    }

    #[test]
    fn place_new_sets_defaults() {
        let p = Place::new("place-x", PlaceLayer::Settlement, "nation", "X");
        assert_eq!(p.id.as_str(), "place-x");
        assert_eq!(p.layer, PlaceLayer::Settlement);
        assert_eq!(p.kind, "nation");
        assert_eq!(p.name, "X");
        assert!(p.aliases.is_empty());
        assert!(p.spatial.parent_place.is_none());
        assert!(p.spatial.bordering_places.is_empty());
        assert!(p.spatial.geography_refs.is_empty());
    }

    #[test]
    fn place_layer_serde_lowercase() {
        let s = serde_json::to_string(&PlaceLayer::Settlement).unwrap();
        assert_eq!(s, "\"settlement\"");
        let g: PlaceLayer = serde_json::from_str("\"geography\"").unwrap();
        assert_eq!(g, PlaceLayer::Geography);
    }

    #[test]
    fn place_layer_from_str_loose() {
        assert_eq!(
            PlaceLayer::from_str_loose("Settlement"),
            Some(PlaceLayer::Settlement)
        );
        assert_eq!(
            PlaceLayer::from_str_loose("  geography  "),
            Some(PlaceLayer::Geography)
        );
        assert_eq!(PlaceLayer::from_str_loose("city"), None);
    }

    #[test]
    fn spatial_serde_skip_when_empty() {
        let s = Spatial::default();
        let json = serde_json::to_string(&s).unwrap();
        // 모든 필드가 default — 직렬화 시 빈 객체.
        assert_eq!(json, "{}");
    }

    #[test]
    fn spatial_full_roundtrip() {
        let s = Spatial {
            parent_place: Some(PlaceId::new("place-parent")),
            relative_position: Some("south-west".into()),
            bordering_places: vec![PlaceId::new("place-a"), PlaceId::new("place-b")],
            geography_refs: vec![PlaceId::new("place-mt")],
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: Spatial = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn place_full_serde_roundtrip() {
        let mut p = Place::new("place-daejin", PlaceLayer::Settlement, "nation", "대진");
        p.aliases = vec!["낙양".into(), "중원 황도".into()];
        p.summary = "축소 제국".into();
        p.tags = vec!["wuxia".into(), "place".into()];
        p.extras
            .insert("capital".into(), Value::String("낙양".into()));
        p.body_sections.insert("개요".into(), "산문".into());
        p.spatial.parent_place = None;
        p.spatial.relative_position = Some("center".into());
        p.spatial.bordering_places = vec![PlaceId::new("place-namgung")];

        let json = serde_json::to_string(&p).unwrap();
        let back: Place = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn controlling_group_extracts_from_extras() {
        let mut p = Place::new("place-namgung-sega", PlaceLayer::Settlement, "sect", "남궁세가");
        p.extras
            .insert("controlling_group".into(), Value::String("group-namgung".into()));
        assert_eq!(p.controlling_group(), Some("group-namgung"));

        let p_none = Place::new("place-empty", PlaceLayer::Geography, "mountain-range", "x");
        assert_eq!(p_none.controlling_group(), None);
    }

    #[test]
    fn cycle_detection_finds_self_loop() {
        let places = vec![p_with_parent("place-a", Some("place-a"))];
        let cycles = detect_parent_place_cycle(&places);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0], vec![PlaceId::new("place-a")]);
    }

    #[test]
    fn cycle_detection_finds_two_node_cycle() {
        let places = vec![
            p_with_parent("place-a", Some("place-b")),
            p_with_parent("place-b", Some("place-a")),
        ];
        let cycles = detect_parent_place_cycle(&places);
        assert_eq!(cycles.len(), 1);
        assert_eq!(
            cycles[0],
            vec![PlaceId::new("place-a"), PlaceId::new("place-b")]
        );
    }

    #[test]
    fn cycle_detection_clean_chain_returns_empty() {
        // place-namgung-geomseong → place-namgung (no cycle)
        let places = vec![
            p_with_parent("place-namgung-geomseong", Some("place-namgung")),
            p_with_parent("place-namgung", None),
        ];
        let cycles = detect_parent_place_cycle(&places);
        assert!(cycles.is_empty());
    }

    #[test]
    fn cycle_detection_three_node_cycle_canonical() {
        // a → b → c → a — canonical: 가장 작은 'a' 시작.
        let places = vec![
            p_with_parent("place-a", Some("place-b")),
            p_with_parent("place-b", Some("place-c")),
            p_with_parent("place-c", Some("place-a")),
        ];
        let cycles = detect_parent_place_cycle(&places);
        assert_eq!(cycles.len(), 1);
        assert_eq!(
            cycles[0],
            vec![
                PlaceId::new("place-a"),
                PlaceId::new("place-b"),
                PlaceId::new("place-c"),
            ]
        );
    }

    #[test]
    fn cycle_detection_dangling_parent_is_not_cycle() {
        // 외래키 결손은 cycle 아님 — 결손 경고는 호출자가 별도 수집.
        let places = vec![p_with_parent("place-x", Some("place-missing"))];
        let cycles = detect_parent_place_cycle(&places);
        assert!(cycles.is_empty());
    }
}
