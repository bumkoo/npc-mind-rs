//! Atlas 애그리거트 — Phase 4 Vertical Slice. 첫 **관계 도메인**.
//!
//! **장르 중립 원칙**: 이 모듈은 wuxia/판타지/SF 어떤 어휘도 모른다. `kind`는
//! free-form `String`이며, 장르가 채운다 (`genres/wuxia/forms/atlas.toml`).
//!
//! **도메인+뷰 이중성**: 9 인스턴스 도메인(Group/Person/Place/...)과 결이 다르다.
//! Atlas는 자기 고유 상태(`extent`/`references`/`body_sections`) + 자기 고유 로직
//! (인접 그래프 traversal·layer 필터) + 다른 도메인 합성 인터페이스(view 메서드)를
//! 모두 가진다. view 메서드는 도메인 객체에 직접 부착되며, View trait 일반화는
//! Phase 5+ 두 번째 관계 도메인(Timeline 등) 등장 시 추출한다.
//!
//! Phase 4 외래키:
//! - `Atlas.references` ↔ `Place.id` (활성 — world-load hard-fail)
//! - `Atlas.extras.era_id` ↔ `Era.id` (Phase 5 활성, Phase 4엔 텍스트만 보존)

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashSet};

use super::place::{Place, PlaceId, PlaceLayer};
use super::WorldError;
use crate::worldbuilding::WorldRepository;

/// Atlas 식별자 — `atlas-{slug}` 형식. slug는 ASCII 소문자·숫자·하이픈.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AtlasId(pub String);

impl AtlasId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AtlasId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for AtlasId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for AtlasId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Atlas의 좌표계·크기·단위 메타.
///
/// Phase 4엔 `projection = "schematic"`만 의미를 가진다 (단위 없는 격자/배치도).
/// `cartesian`/`hex-grid` 등 절대 좌표·격자는 Phase N+ 도입.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtlasExtent {
    /// `"schematic"` (Phase 4) | `"cartesian"` | `"hex-grid"` (Phase N+).
    pub projection: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_units: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height_units: Option<u32>,
    /// `"schematic"` (Phase 4 단위 의미 없음) | `"km"` | `"li"` (Phase N+).
    #[serde(default = "default_unit")]
    pub unit: String,
}

fn default_unit() -> String {
    "schematic".to_string()
}

impl Default for AtlasExtent {
    fn default() -> Self {
        Self {
            projection: "schematic".to_string(),
            width_units: None,
            height_units: None,
            unit: default_unit(),
        }
    }
}

/// Atlas 애그리거트 — 첫 관계 도메인.
///
/// 핵심 책임:
/// - 정체성: id/name/aliases/kind + summary/tags
/// - 좌표계 메타: extent (projection·units)
/// - **합성 핵심**: references (어느 Place들이 본 atlas에 등장)
/// - 자유 본문: body_sections (`## 배치 다이어그램` ASCII 등을 byte-exact 보존)
/// - 장르 확장: extras (era·era_id 등)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Atlas {
    pub id: AtlasId,
    /// 장르가 채움 (Phase 4 wuxia: `continent`·`region`·`city-map`).
    pub kind: String,
    pub name: String,
    /// 별호·옛 이름. 예: `["중원 대륙","칠국 대륙"]`. FTS5 검색 대상에 포함.
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 장르 자유 확장. wuxia 예: `era`(현재 시대 텍스트)·`era_id`(Phase 5 외래키)·
    /// `source_section`(원전 섹션 추적).
    #[serde(default)]
    pub extras: Map<String, Value>,
    #[serde(default)]
    pub extent: AtlasExtent,
    /// **핵심** — 본 atlas에 등장하는 Place들. world-load 시 외래키 hard-fail.
    /// 작성 순서가 곧 view에서 보이는 순서이며, 보통 좌상→우하 또는 다이어그램 순.
    #[serde(default)]
    pub references: Vec<PlaceId>,
    /// H2 섹션 본문. `BTreeMap`이라 알파벳 정렬 순서로 보존되며, 작성 순서는
    /// 보존되지 않는다 (Group/Place 동일 정책).
    ///
    /// **핵심**: `## 배치 다이어그램` 같은 ASCII art 섹션이 byte-exact로 보존되어야
    /// 한다. 마크다운 파서가 코드블록(```...```)을 그대로 본문에 넣으므로 깨지지 않는다.
    #[serde(default)]
    pub body_sections: BTreeMap<String, String>,
    /// 마크다운 SoT 경로 (절대 또는 프로젝트 root 기준 상대).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}

impl Atlas {
    /// 최소 생성자. 테스트·도구용.
    pub fn new(id: impl Into<AtlasId>, kind: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            name: name.into(),
            aliases: Vec::new(),
            summary: String::new(),
            tags: Vec::new(),
            extras: Map::new(),
            extent: AtlasExtent::default(),
            references: Vec::new(),
            body_sections: BTreeMap::new(),
            source_path: None,
        }
    }

    /// `extras["era"]`을 String으로 추출 (없거나 비문자열이면 None).
    /// Phase 4엔 잠정 텍스트 ("현재 (칠국춘추 270년차)" 등). Phase 5에서 era_id로 정형화.
    pub fn era(&self) -> Option<&str> {
        self.extras.get("era").and_then(|v| v.as_str())
    }

    /// `extras["era_id"]`을 String으로 추출. Phase 4엔 텍스트만 보존되며 검증 비활성.
    /// Phase 5에서 Era 도메인 외래키로 활성화 예정.
    pub fn era_id(&self) -> Option<&str> {
        self.extras.get("era_id").and_then(|v| v.as_str())
    }

    // -----------------------------------------------------------------------
    // 도메인+뷰 이중성 — view 메서드. 다른 도메인(Place) 합성.
    //
    // Phase 5+에서 두 번째 관계 도메인(Timeline·OrgChart 등)이 등장하면 공통 패턴을
    // `View<DomainItem>` trait으로 추출한다. Phase 4엔 Atlas 단독.
    // -----------------------------------------------------------------------

    /// 본 atlas의 `references`를 따라 Place 정보 합성.
    ///
    /// references 작성 순서대로 반환. 결손된 ID는 stderr 경고 없이 그냥 누락됨
    /// (world-load 시 hard-fail로 결손 0건이 보장되므로 정상 데이터에선 누락 X).
    pub fn places_in<R: WorldRepository + ?Sized>(
        &self,
        repo: &R,
    ) -> Result<Vec<Place>, WorldError> {
        let mut out = Vec::with_capacity(self.references.len());
        for id in &self.references {
            if let Some(p) = repo.get_place(id)? {
                out.push(p);
            }
        }
        Ok(out)
    }

    /// settlement layer만.
    pub fn settlements_in<R: WorldRepository + ?Sized>(
        &self,
        repo: &R,
    ) -> Result<Vec<Place>, WorldError> {
        Ok(self
            .places_in(repo)?
            .into_iter()
            .filter(|p| p.layer == PlaceLayer::Settlement)
            .collect())
    }

    /// geography layer만.
    pub fn geographies_in<R: WorldRepository + ?Sized>(
        &self,
        repo: &R,
    ) -> Result<Vec<Place>, WorldError> {
        Ok(self
            .places_in(repo)?
            .into_iter()
            .filter(|p| p.layer == PlaceLayer::Geography)
            .collect())
    }

    /// 특정 Place의 atlas-국한 인접 그래프.
    ///
    /// Place의 `spatial.bordering_places`를 따라 인접 ID를 모으되, 본 atlas의
    /// `references`에 있는 것만 남긴다 (atlas 경계 밖은 무시). Place가 atlas의
    /// references에 없거나 repo에 없으면 빈 Vec.
    pub fn adjacent_to<R: WorldRepository + ?Sized>(
        &self,
        place_id: &PlaceId,
        repo: &R,
    ) -> Result<Vec<PlaceId>, WorldError> {
        let in_atlas: HashSet<&PlaceId> = self.references.iter().collect();
        if !in_atlas.contains(place_id) {
            return Ok(Vec::new());
        }
        let Some(place) = repo.get_place(place_id)? else {
            return Ok(Vec::new());
        };
        Ok(place
            .spatial
            .bordering_places
            .into_iter()
            .filter(|id| in_atlas.contains(id))
            .collect())
    }
}

/// 리스트 필터 — `WorldRepository::list_atlases`에 전달.
#[derive(Debug, Clone, Default)]
pub struct AtlasFilter {
    pub kind: Option<String>,
    /// `tags` 토큰 매칭. `wuxia`/`continent` 등.
    pub genre_tag: Option<String>,
}

// ---------------------------------------------------------------------------
// 단위 테스트
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::world::{
        place::{Place, PlaceFilter, PlaceLayer},
        person::{Person, PersonFilter, PersonId},
        group::{Group, GroupFilter, GroupId},
    };
    use std::sync::Mutex;

    /// 최소 in-memory repo — view 메서드 테스트용. WorldRepository의 place 메서드만
    /// 의미 있게 구현하고 group/person 메서드는 unimplemented.
    struct MiniRepo {
        places: Mutex<Vec<Place>>,
    }

    impl MiniRepo {
        fn new(places: Vec<Place>) -> Self {
            Self {
                places: Mutex::new(places),
            }
        }
    }

    impl WorldRepository for MiniRepo {
        fn list_groups(&self, _: GroupFilter) -> Result<Vec<Group>, WorldError> {
            unimplemented!("MiniRepo: group 미사용")
        }
        fn get_group(&self, _: &GroupId) -> Result<Option<Group>, WorldError> {
            unimplemented!()
        }
        fn search_groups(&self, _: &str, _: u32) -> Result<Vec<Group>, WorldError> {
            unimplemented!()
        }
        fn upsert_group(&self, _: &str, _: &Group) -> Result<(), WorldError> {
            unimplemented!()
        }
        fn count_groups(&self, _: Option<&str>) -> Result<u64, WorldError> {
            unimplemented!()
        }
        fn list_persons(&self, _: PersonFilter) -> Result<Vec<Person>, WorldError> {
            unimplemented!()
        }
        fn get_person(&self, _: &PersonId) -> Result<Option<Person>, WorldError> {
            unimplemented!()
        }
        fn search_persons(&self, _: &str, _: u32) -> Result<Vec<Person>, WorldError> {
            unimplemented!()
        }
        fn upsert_person(&self, _: &str, _: &Person) -> Result<(), WorldError> {
            unimplemented!()
        }
        fn count_persons(&self, _: Option<&str>) -> Result<u64, WorldError> {
            unimplemented!()
        }
        fn list_places(&self, _: PlaceFilter) -> Result<Vec<Place>, WorldError> {
            Ok(self.places.lock().unwrap().clone())
        }
        fn get_place(&self, id: &PlaceId) -> Result<Option<Place>, WorldError> {
            Ok(self
                .places
                .lock()
                .unwrap()
                .iter()
                .find(|p| &p.id == id)
                .cloned())
        }
        fn search_places(&self, _: &str, _: u32) -> Result<Vec<Place>, WorldError> {
            unimplemented!()
        }
        fn upsert_place(&self, _: &str, _: &Place) -> Result<(), WorldError> {
            unimplemented!()
        }
        fn count_places(&self, _: Option<&str>) -> Result<u64, WorldError> {
            unimplemented!()
        }
        fn list_atlases(&self, _: AtlasFilter) -> Result<Vec<Atlas>, WorldError> {
            unimplemented!()
        }
        fn get_atlas(&self, _: &AtlasId) -> Result<Option<Atlas>, WorldError> {
            unimplemented!()
        }
        fn search_atlases(&self, _: &str, _: u32) -> Result<Vec<Atlas>, WorldError> {
            unimplemented!()
        }
        fn upsert_atlas(&self, _: &str, _: &Atlas) -> Result<(), WorldError> {
            unimplemented!()
        }
        fn count_atlases(&self, _: Option<&str>) -> Result<u64, WorldError> {
            unimplemented!()
        }
    }

    fn settlement(id: &str, bordering: &[&str]) -> Place {
        let mut p = Place::new(id, PlaceLayer::Settlement, "nation", id);
        p.spatial.bordering_places = bordering.iter().map(|s| PlaceId::new(*s)).collect();
        p
    }

    fn geography(id: &str) -> Place {
        Place::new(id, PlaceLayer::Geography, "mountain-range", id)
    }

    fn sample_atlas(refs: Vec<&str>) -> Atlas {
        let mut a = Atlas::new("atlas-test", "continent", "Test Atlas");
        a.references = refs.into_iter().map(PlaceId::new).collect();
        a
    }

    #[test]
    fn atlas_new_sets_defaults() {
        let a = Atlas::new("atlas-x", "continent", "X");
        assert_eq!(a.id.as_str(), "atlas-x");
        assert_eq!(a.kind, "continent");
        assert_eq!(a.name, "X");
        assert!(a.references.is_empty());
        assert!(a.body_sections.is_empty());
        assert_eq!(a.extent.projection, "schematic");
        assert_eq!(a.extent.unit, "schematic");
        assert!(a.extent.width_units.is_none());
    }

    #[test]
    fn atlas_extent_default_is_schematic() {
        let e = AtlasExtent::default();
        assert_eq!(e.projection, "schematic");
        assert_eq!(e.unit, "schematic");
    }

    #[test]
    fn atlas_extent_serde_skip_when_units_none() {
        let e = AtlasExtent::default();
        let json = serde_json::to_string(&e).unwrap();
        // width/height_units None → skip. projection·unit은 default라도 직렬화.
        assert!(!json.contains("width_units"));
        assert!(!json.contains("height_units"));
        assert!(json.contains("\"projection\":\"schematic\""));
    }

    #[test]
    fn atlas_full_serde_roundtrip() {
        let mut a = Atlas::new("atlas-jungwon", "continent", "칠국춘추 대륙");
        a.aliases = vec!["중원 대륙".into(), "칠국 대륙".into()];
        a.summary = "대륙 요약".into();
        a.tags = vec!["wuxia".into(), "atlas".into()];
        a.extent = AtlasExtent {
            projection: "schematic".into(),
            width_units: Some(7),
            height_units: Some(7),
            unit: "schematic".into(),
        };
        a.references = vec![PlaceId::new("place-daejin"), PlaceId::new("place-namgung")];
        a.body_sections
            .insert("배치 다이어그램".into(), "┌──┐\n│중원│\n└──┘".into());
        a.extras
            .insert("era".into(), Value::String("현재 (칠국춘추 270년차)".into()));

        let json = serde_json::to_string(&a).unwrap();
        let back: Atlas = serde_json::from_str(&json).unwrap();
        assert_eq!(back, a);
    }

    #[test]
    fn era_helpers_extract_from_extras() {
        let mut a = Atlas::new("atlas-x", "continent", "X");
        a.extras
            .insert("era".into(), Value::String("현재".into()));
        a.extras
            .insert("era_id".into(), Value::String("era-current".into()));
        assert_eq!(a.era(), Some("현재"));
        assert_eq!(a.era_id(), Some("era-current"));

        let a2 = Atlas::new("atlas-y", "continent", "Y");
        assert!(a2.era().is_none());
        assert!(a2.era_id().is_none());
    }

    #[test]
    fn places_in_returns_in_reference_order() {
        let repo = MiniRepo::new(vec![
            settlement("place-a", &[]),
            settlement("place-b", &[]),
            settlement("place-c", &[]),
        ]);
        // references는 a, c, b 순서로 — places_in 결과도 동일 순서.
        let atlas = sample_atlas(vec!["place-a", "place-c", "place-b"]);
        let got = atlas.places_in(&repo).unwrap();
        let ids: Vec<&str> = got.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["place-a", "place-c", "place-b"]);
    }

    #[test]
    fn places_in_skips_missing_silently() {
        // world-load가 hard-fail로 결손 0건을 보장하므로 view는 silent skip.
        let repo = MiniRepo::new(vec![settlement("place-a", &[])]);
        let atlas = sample_atlas(vec!["place-a", "place-missing"]);
        let got = atlas.places_in(&repo).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id.as_str(), "place-a");
    }

    #[test]
    fn settlements_and_geographies_filter_by_layer() {
        let repo = MiniRepo::new(vec![
            settlement("place-s1", &[]),
            settlement("place-s2", &[]),
            geography("place-g1"),
        ]);
        let atlas = sample_atlas(vec!["place-s1", "place-g1", "place-s2"]);
        let s = atlas.settlements_in(&repo).unwrap();
        let g = atlas.geographies_in(&repo).unwrap();
        assert_eq!(s.len(), 2);
        assert!(s.iter().all(|p| p.layer == PlaceLayer::Settlement));
        assert_eq!(g.len(), 1);
        assert!(g.iter().all(|p| p.layer == PlaceLayer::Geography));
    }

    #[test]
    fn adjacent_to_filters_to_atlas_boundary() {
        // place-a borders {b, c, outside}. atlas references={a, b, c}.
        // adjacent_to(a) → {b, c} (outside는 atlas 밖이라 제외).
        let repo = MiniRepo::new(vec![
            settlement("place-a", &["place-b", "place-c", "place-outside"]),
            settlement("place-b", &["place-a"]),
            settlement("place-c", &["place-a"]),
            settlement("place-outside", &["place-a"]),
        ]);
        let atlas = sample_atlas(vec!["place-a", "place-b", "place-c"]);
        let adj = atlas.adjacent_to(&PlaceId::new("place-a"), &repo).unwrap();
        let ids: Vec<&str> = adj.iter().map(|p| p.as_str()).collect();
        assert_eq!(ids, vec!["place-b", "place-c"]);
    }

    #[test]
    fn adjacent_to_returns_empty_when_place_not_in_atlas() {
        // atlas references={a}. adjacent_to(outside) → 빈 Vec (Phase 4 결정: 사일런트).
        let repo = MiniRepo::new(vec![
            settlement("place-a", &[]),
            settlement("place-outside", &["place-a"]),
        ]);
        let atlas = sample_atlas(vec!["place-a"]);
        let adj = atlas
            .adjacent_to(&PlaceId::new("place-outside"), &repo)
            .unwrap();
        assert!(adj.is_empty());
    }

    #[test]
    fn adjacent_to_returns_empty_when_place_missing_from_repo() {
        // references에는 있으나 repo엔 결손 — 그래도 panic 없이 빈 Vec.
        let repo = MiniRepo::new(vec![]);
        let atlas = sample_atlas(vec!["place-ghost"]);
        let adj = atlas
            .adjacent_to(&PlaceId::new("place-ghost"), &repo)
            .unwrap();
        assert!(adj.is_empty());
    }
}
