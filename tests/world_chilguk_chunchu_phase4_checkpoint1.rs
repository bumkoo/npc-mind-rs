//! Phase 4 Checkpoint 1 — chilguk-chunchu `atlas-jungwon` 단독 변환 엔드투엔드.
//!
//! 사양 `task-phase4-atlas-vertical-slice.md` §5 Step 3 자동화:
//! - `world/atlas/atlas-jungwon.md` 파싱 → references 11 Place + body_sections
//! - SqliteWorldStore 라운드트립 — atlases 테이블 + place_atlas_refs 양방향 인덱스
//! - **byte-exact ASCII 다이어그램 보존** (`## 배치 다이어그램` body section)
//! - references = Phase 3 산출 11 Place 모두 (8 settlement + 3 geography)
//! - view 메서드 e2e — places_in/settlements_in/geographies_in/adjacent_to
//! - 외래키 결손 0건 (Phase 4 활성)
//!
//! 실행: `cargo test --features embed --test world_chilguk_chunchu_phase4_checkpoint1`

#![cfg(feature = "embed")]

use std::path::PathBuf;

use npc_mind::adapter::sqlite_world::SqliteWorldStore;
use npc_mind::domain::world::{
    Atlas, AtlasFilter, AtlasId, Place, PlaceFilter, PlaceId, PlaceLayer,
};
use npc_mind::worldbuilding::WorldRepository;
use npc_mind::worldbuilding::markdown::{atlas_from_markdown, place_from_markdown};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_atlas_from_disk() -> Atlas {
    let path = project_root().join("projects/chilguk-chunchu/world/atlas/atlas-jungwon.md");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut a = atlas_from_markdown(&raw)
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    a.source_path = Some(path.to_string_lossy().to_string());
    a
}

fn load_all_places() -> Vec<Place> {
    let dir = project_root().join("projects/chilguk-chunchu/world/place");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let raw = std::fs::read_to_string(&path).unwrap();
        let mut p = place_from_markdown(&raw)
            .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
        p.source_path = Some(path.to_string_lossy().to_string());
        out.push(p);
    }
    out
}

/// 헬퍼: 메모리 store에 atlas + 모든 place 적재.
fn populated_store() -> SqliteWorldStore {
    let store = SqliteWorldStore::in_memory().unwrap();
    let atlas = load_atlas_from_disk();
    for p in load_all_places() {
        store.upsert_place("chilguk-chunchu", &p).unwrap();
    }
    store.upsert_atlas("chilguk-chunchu", &atlas).unwrap();
    store
}

#[test]
fn atlas_jungwon_parses_with_expected_identity() {
    let a = load_atlas_from_disk();
    assert_eq!(a.id.as_str(), "atlas-jungwon");
    assert_eq!(a.kind, "continent");
    assert_eq!(a.name, "칠국춘추 대륙");
    // aliases — 디렉터 결정한 2개.
    assert_eq!(a.aliases.len(), 2);
    assert!(a.aliases.contains(&"중원 대륙".to_string()));
    assert!(a.aliases.contains(&"칠국 대륙".to_string()));

    // extras — Phase 4 era 텍스트만, era_id는 비움 (Phase 5 진입 시 활성).
    assert_eq!(a.era(), Some("현재 (칠국춘추 270년차)"));
    assert!(a.era_id().is_none());

    // extent — schematic 7×7
    assert_eq!(a.extent.projection, "schematic");
    assert_eq!(a.extent.width_units, Some(7));
    assert_eq!(a.extent.height_units, Some(7));
    assert_eq!(a.extent.unit, "schematic");

    // 권장 H2 섹션 6개.
    for section in [
        "개요",
        "칠국 일람",
        "배치 다이어그램",
        "자연 영역 분포",
        "정치체 분포",
        "주요 통로·연결",
        "전사(前史)",
    ] {
        assert!(
            a.body_sections.contains_key(section),
            "{section} 섹션이 body_sections에 있어야 함"
        );
    }
}

#[test]
fn atlas_jungwon_references_contain_all_eleven_places() {
    let a = load_atlas_from_disk();
    assert_eq!(a.references.len(), 11);
    let expected = [
        "place-bukwon",
        "place-bukwon-grasslands",
        "place-seoryang",
        "place-western-mountains",
        "place-daejin",
        "place-donghae",
        "place-jiyu-doshi",
        "place-namgung",
        "place-namgung-sega",
        "place-namman",
        "place-namman-jungle",
    ];
    let actual: Vec<&str> = a.references.iter().map(PlaceId::as_str).collect();
    assert_eq!(
        actual, expected,
        "references는 좌상→우하 순서로 11개여야 함 (settlement 8 + geography 3)"
    );
}

#[test]
fn ascii_diagram_preserved_byte_exact_through_disk_and_sqlite() {
    // §0.3 다이어그램의 핵심 시각 라인이 raw .md 파일과 atlas_from_markdown 결과,
    // SQLite 라운드트립까지 모두 동일해야 한다.
    let raw = std::fs::read_to_string(
        project_root().join("projects/chilguk-chunchu/world/atlas/atlas-jungwon.md"),
    )
    .unwrap();

    // 시각적 검증을 위한 sentinel 라인들 (seven-nations.md §0.3에서 그대로).
    let sentinels = [
        "                    ┌──────────────────┐",
        "                    │     북 원        │",
        "                    │   (초원/유목)     │",
        "                    │   왕정(오르두)    │",
        "                    └────────┬─────────┘",
        "         ┌───────────────────┼──────────────────┐",
        "    ┌────┴────┐        ┌─────┴─────┐      ┌────┴────┐",
        "    │  서 량   │        │   대 진    │      │  동 해   │",
        "    │ 독관성   │        │  낙양     │      │  해문    │",
        "    └────┬────┘        └─────┬─────┘      └────┬────┘",
        "         │            ┌──────┴──────┐           │",
        "         │            │  자유도시    │           │",
        "         │            └──────┬──────┘           │",
        "    ┌────┴────┐        ┌─────┴─────┐            │",
        "    │  (산악)  │        │   남 궁    │            │",
        "    │         │        │  검성     │            │",
        "    └─────────┘        └─────┬─────┘            │",
        "                        ┌────┴────┐             │",
        "                        │  남 만   │             │",
        "                        │ (남방밀림)│             │",
        "                        │ 만왕성   │             │",
        "                        └─────────┘",
    ];
    for s in sentinels {
        assert!(
            raw.contains(s),
            "원본 .md 파일이 §0.3 sentinel 라인 누락: {s:?}"
        );
    }

    // 1차 변환 — 마크다운 파서가 펜스 안의 ## 가짜 헤더·들여쓰기를 깨지 않아야 함.
    let atlas = atlas_from_markdown(&raw).unwrap();
    let parsed_diagram = atlas
        .body_sections
        .get("배치 다이어그램")
        .expect("배치 다이어그램 섹션 필요");
    for s in sentinels {
        assert!(
            parsed_diagram.contains(s),
            "마크다운 파싱 결과에서 §0.3 sentinel 라인 누락: {s:?}"
        );
    }
    assert!(
        parsed_diagram.starts_with("```"),
        "다이어그램은 코드블록 ``` 펜스로 시작해야 byte-exact 보존 가능"
    );
    assert!(
        parsed_diagram.ends_with("```"),
        "다이어그램은 코드블록 ``` 펜스로 끝나야 byte-exact 보존 가능"
    );

    // 2차 — SQLite 라운드트립.
    let store = SqliteWorldStore::in_memory().unwrap();
    store.upsert_atlas("p", &atlas).unwrap();
    let back = store
        .get_atlas(&AtlasId::new("atlas-jungwon"))
        .unwrap()
        .unwrap();
    let restored_diagram = back.body_sections.get("배치 다이어그램").unwrap();
    assert_eq!(
        restored_diagram, parsed_diagram,
        "SQLite 라운드트립 후에도 다이어그램 본문이 byte-exact 보존되어야 함"
    );
    for s in sentinels {
        assert!(
            restored_diagram.contains(s),
            "SQLite 라운드트립 후 §0.3 sentinel 라인 누락: {s:?}"
        );
    }
}

#[test]
fn world_load_indexes_atlas_with_zero_fk_residual() {
    // populated_store가 11 Place + 1 Atlas를 로드하는데, atlas references 11개가 모두
    // place에 존재해야 atlases·place_atlas_refs upsert가 PK 위반 없이 통과한다.
    let store = populated_store();
    assert_eq!(store.count_atlases(Some("chilguk-chunchu")).unwrap(), 1);
    assert_eq!(store.count_places(Some("chilguk-chunchu")).unwrap(), 11);

    // place_atlas_refs는 atlas 1 × references 11 = 11 row.
    let atlas = store
        .get_atlas(&AtlasId::new("atlas-jungwon"))
        .unwrap()
        .unwrap();
    assert_eq!(atlas.references.len(), 11);
    // 모든 references가 places 테이블에 있는지 명시적으로 확인.
    for pid in &atlas.references {
        assert!(
            store.get_place(pid).unwrap().is_some(),
            "FK 결손: {pid} 가 places 테이블에 없음"
        );
    }
}

#[test]
fn places_in_returns_all_eleven_in_reference_order() {
    let store = populated_store();
    let atlas = store
        .get_atlas(&AtlasId::new("atlas-jungwon"))
        .unwrap()
        .unwrap();
    let places = atlas.places_in(&store).unwrap();
    assert_eq!(places.len(), 11);
    // references 작성 순서 그대로 보존.
    let ids: Vec<&str> = places.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "place-bukwon",
            "place-bukwon-grasslands",
            "place-seoryang",
            "place-western-mountains",
            "place-daejin",
            "place-donghae",
            "place-jiyu-doshi",
            "place-namgung",
            "place-namgung-sega",
            "place-namman",
            "place-namman-jungle",
        ]
    );
}

#[test]
fn settlements_in_returns_eight() {
    let store = populated_store();
    let atlas = store.get_atlas(&AtlasId::new("atlas-jungwon")).unwrap().unwrap();
    let settlements = atlas.settlements_in(&store).unwrap();
    assert_eq!(settlements.len(), 8);
    assert!(settlements.iter().all(|p| p.layer == PlaceLayer::Settlement));
}

#[test]
fn geographies_in_returns_three() {
    let store = populated_store();
    let atlas = store.get_atlas(&AtlasId::new("atlas-jungwon")).unwrap().unwrap();
    let geographies = atlas.geographies_in(&store).unwrap();
    assert_eq!(geographies.len(), 3);
    assert!(geographies.iter().all(|p| p.layer == PlaceLayer::Geography));
    let ids: Vec<&str> = geographies.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.contains(&"place-western-mountains"));
    assert!(ids.contains(&"place-bukwon-grasslands"));
    assert!(ids.contains(&"place-namman-jungle"));
}

#[test]
fn adjacent_to_daejin_returns_three_atlas_internal_neighbors() {
    let store = populated_store();
    let atlas = store.get_atlas(&AtlasId::new("atlas-jungwon")).unwrap().unwrap();
    let adj = atlas
        .adjacent_to(&PlaceId::new("place-daejin"), &store)
        .unwrap();
    // place-daejin.bordering_places = [place-namgung, place-jiyu-doshi, place-seoryang],
    // 모두 atlas references에 있으므로 3건.
    let ids: Vec<&str> = adj.iter().map(PlaceId::as_str).collect();
    assert_eq!(
        ids,
        vec!["place-namgung", "place-jiyu-doshi", "place-seoryang"]
    );
}

#[test]
fn adjacent_to_namgung_sega_returns_zero() {
    let store = populated_store();
    let atlas = store.get_atlas(&AtlasId::new("atlas-jungwon")).unwrap().unwrap();
    let adj = atlas
        .adjacent_to(&PlaceId::new("place-namgung-sega"), &store)
        .unwrap();
    // sect의 bordering_places는 비어 있으므로 0. parent_place·controlling_group으로만
    // 연결된다 (Phase 4 spec §5 Step 4 atlas_adjacent_to_namgung_sega_returns_zero_or_one).
    assert!(adj.is_empty(), "sect의 atlas-인접은 0이어야 함");
}

#[test]
fn layer_filter_invariant_holds_for_settlements_and_geographies() {
    // settlements_in의 모든 결과가 Settlement, geographies_in의 모든 결과가 Geography.
    // settlements + geographies = places_in (다른 layer는 atlas references에 없음).
    let store = populated_store();
    let atlas = store.get_atlas(&AtlasId::new("atlas-jungwon")).unwrap().unwrap();
    let s = atlas.settlements_in(&store).unwrap();
    let g = atlas.geographies_in(&store).unwrap();
    let p = atlas.places_in(&store).unwrap();
    assert_eq!(s.len() + g.len(), p.len(), "8+3=11 invariant");
    assert!(s.iter().all(|x| x.layer == PlaceLayer::Settlement));
    assert!(g.iter().all(|x| x.layer == PlaceLayer::Geography));
}

#[test]
fn list_atlases_filter_by_kind_continent_returns_one() {
    let store = populated_store();
    let conts = store
        .list_atlases(AtlasFilter {
            kind: Some("continent".into()),
            genre_tag: None,
        })
        .unwrap();
    assert_eq!(conts.len(), 1);
    assert_eq!(conts[0].id.as_str(), "atlas-jungwon");
}

#[test]
fn search_atlases_finds_by_alias_and_summary() {
    let store = populated_store();
    let hits1 = store.search_atlases("중원", 5).unwrap();
    assert!(!hits1.is_empty());
    assert_eq!(hits1[0].id.as_str(), "atlas-jungwon");

    let hits2 = store.search_atlases("칠국", 5).unwrap();
    assert!(!hits2.is_empty());
}

#[test]
fn place_id_appears_in_place_atlas_refs_via_list_places() {
    // 정상 데이터 정합성 — atlas references가 가리키는 모든 place가 list_places에서 검색 가능.
    let store = populated_store();
    let all_places = store.list_places(PlaceFilter::default()).unwrap();
    let all_ids: std::collections::HashSet<&str> =
        all_places.iter().map(|p| p.id.as_str()).collect();
    let atlas = store.get_atlas(&AtlasId::new("atlas-jungwon")).unwrap().unwrap();
    for pid in &atlas.references {
        assert!(
            all_ids.contains(pid.as_str()),
            "atlas references {pid} 가 list_places 결과에 없음 (FK 결손)"
        );
    }
}
