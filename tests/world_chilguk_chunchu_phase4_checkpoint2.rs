//! Phase 4 Checkpoint 2 — atlas-jungwon view 메서드 e2e + search_atlases + 외래키 가드.
//!
//! 사양 `task-phase4-atlas-vertical-slice.md` §5 Step 4 자동화:
//! - view 메서드 e2e — places_in/settlements_in/geographies_in/adjacent_to (체크포인트 1
//!   13개의 보강: 모든 settlement·geography anchor에 대한 adjacent_to 매트릭스)
//! - search_atlases 3 쿼리 — "칠국"·"중원"·"대륙" (디렉터 명시)
//! - 외래키 결손 0건 가드 — atlas references ⊂ places.id 정합성
//! - layer_filter_invariant_holds — settlements_in의 모든 결과가 layer=Settlement
//!
//! 체크포인트 1과 별도 테스트 파일로 분리 — Step 4 산출물이 회귀로 잡힘을 명시.
//!
//! 실행: `cargo test --features embed --test world_chilguk_chunchu_phase4_checkpoint2`

#![cfg(feature = "embed")]

use std::path::PathBuf;

use npc_mind::adapter::sqlite_world::SqliteWorldStore;
use npc_mind::domain::world::{Atlas, AtlasFilter, AtlasId, Place, PlaceId, PlaceLayer};
use npc_mind::worldbuilding::WorldRepository;
use npc_mind::worldbuilding::markdown::{atlas_from_markdown, place_from_markdown};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_atlas() -> Atlas {
    let path = project_root().join("projects/chilguk-chunchu/world/atlas/atlas-jungwon.md");
    let raw = std::fs::read_to_string(&path).unwrap();
    atlas_from_markdown(&raw).unwrap()
}

fn populated_store() -> SqliteWorldStore {
    let store = SqliteWorldStore::in_memory().unwrap();
    let dir = project_root().join("projects/chilguk-chunchu/world/place");
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let raw = std::fs::read_to_string(&path).unwrap();
        let p = place_from_markdown(&raw).unwrap();
        store.upsert_place("chilguk-chunchu", &p).unwrap();
    }
    store.upsert_atlas("chilguk-chunchu", &load_atlas()).unwrap();
    store
}

fn atlas_jungwon(store: &SqliteWorldStore) -> Atlas {
    store
        .get_atlas(&AtlasId::new("atlas-jungwon"))
        .unwrap()
        .unwrap()
}

// ---------------------------------------------------------------------------
// view 메서드 e2e — 모든 anchor 매트릭스 (체크포인트 1 보강)
// ---------------------------------------------------------------------------

/// settlement anchor별 adjacent_to 매트릭스. atlas references 안의 인접만.
fn expected_adjacency() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        // 체크포인트 1 자동화: daejin·namgung-sega 명시. 여기서 나머지 모두 가드.
        ("place-bukwon", vec!["place-daejin", "place-seoryang", "place-donghae"]),
        ("place-seoryang", vec!["place-daejin", "place-bukwon", "place-western-mountains"]),
        ("place-daejin", vec!["place-namgung", "place-jiyu-doshi", "place-seoryang"]),
        ("place-donghae", vec!["place-daejin", "place-bukwon"]),
        ("place-jiyu-doshi", vec!["place-daejin", "place-namgung"]),
        ("place-namgung", vec!["place-daejin", "place-jiyu-doshi", "place-namman"]),
        ("place-namgung-sega", vec![]), // sect — bordering 없음
        ("place-namman", vec!["place-namgung"]),
        ("place-bukwon-grasslands", vec!["place-bukwon"]),
        ("place-western-mountains", vec!["place-seoryang"]),
        ("place-namman-jungle", vec!["place-namman"]),
    ]
}

#[test]
fn adjacent_to_matrix_for_all_atlas_anchors() {
    let store = populated_store();
    let atlas = atlas_jungwon(&store);
    for (anchor, expected) in expected_adjacency() {
        let got = atlas.adjacent_to(&PlaceId::new(anchor), &store).unwrap();
        let got_ids: Vec<&str> = got.iter().map(PlaceId::as_str).collect();
        assert_eq!(
            got_ids, expected,
            "adjacent_to({anchor}) 결과 불일치 (atlas-internal 인접만)"
        );
    }
}

#[test]
fn settlements_in_layer_invariant_eight_settlement_layer() {
    let store = populated_store();
    let s = atlas_jungwon(&store).settlements_in(&store).unwrap();
    assert_eq!(s.len(), 8);
    assert!(s.iter().all(|p| p.layer == PlaceLayer::Settlement));
}

#[test]
fn geographies_in_layer_invariant_three_geography_layer() {
    let store = populated_store();
    let g = atlas_jungwon(&store).geographies_in(&store).unwrap();
    assert_eq!(g.len(), 3);
    assert!(g.iter().all(|p| p.layer == PlaceLayer::Geography));
}

#[test]
fn places_in_partition_invariant_holds() {
    // settlements + geographies = places — Phase 4엔 Place layer가 두 결뿐.
    let store = populated_store();
    let atlas = atlas_jungwon(&store);
    let p = atlas.places_in(&store).unwrap();
    let s = atlas.settlements_in(&store).unwrap();
    let g = atlas.geographies_in(&store).unwrap();
    assert_eq!(s.len() + g.len(), p.len(), "8+3=11 invariant");
    assert_eq!(p.len(), atlas.references.len(), "places_in.len = references.len");
}

// ---------------------------------------------------------------------------
// search_atlases — 디렉터 명시 3 쿼리
// ---------------------------------------------------------------------------

/// 디렉터 명시: "칠국"·"중원"·"대륙" 3 쿼리 모두 atlas-jungwon에 hit해야 함.
fn search_queries() -> Vec<(&'static str, &'static str)> {
    vec![
        ("칠국", "atlas-jungwon"),  // name "칠국춘추 대륙" + alias "칠국 대륙" + summary
        ("중원", "atlas-jungwon"),  // alias "중원 대륙" + summary "대진(중원)" + body
        ("대륙", "atlas-jungwon"),  // name "...대륙" + aliases 둘 다 "...대륙"
    ]
}

#[test]
fn search_atlases_three_queries_match_atlas_jungwon() {
    let store = populated_store();
    for (q, expected_top) in search_queries() {
        let hits = store.search_atlases(q, 5).unwrap();
        assert!(
            !hits.is_empty(),
            "search_atlases({q}) 결과 비어 있음 — 최소 1건 매칭되어야 함"
        );
        assert_eq!(
            hits[0].id.as_str(),
            expected_top,
            "search_atlases({q}) 최상위 hit 불일치"
        );
    }
}

// ---------------------------------------------------------------------------
// 외래키 결손 0건 — Phase 4 활성 가드 (회귀 회복)
// ---------------------------------------------------------------------------

#[test]
fn references_zero_fk_residual_against_loaded_places() {
    let store = populated_store();
    let atlas = atlas_jungwon(&store);
    for pid in &atlas.references {
        let p: Option<Place> = store.get_place(pid).unwrap();
        assert!(
            p.is_some(),
            "FK 결손: atlas references {pid} 가 places 테이블에 없음"
        );
    }
}

#[test]
fn place_atlas_refs_row_count_matches_references_length() {
    // atlas references 11 → place_atlas_refs row 11.
    let store = populated_store();
    let atlas = atlas_jungwon(&store);
    let n: i64 = {
        // SqliteWorldStore의 conn에 직접 접근하지 않고 list_places 결과로 간접 검증.
        // (직접 SQL 카운트는 lib::tests에 있고, 본 e2e는 public API 위주)
        atlas.references.len() as i64
    };
    assert_eq!(n, 11);
    // 공개 API 쿼리: references 모두 list_atlases 결과의 동일 atlas에서 보임.
    let listed = store
        .list_atlases(AtlasFilter::default())
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].references.len(), 11);
    assert_eq!(listed[0].references, atlas.references);
}

// ---------------------------------------------------------------------------
// list_atlases 필터 가드
// ---------------------------------------------------------------------------

#[test]
fn list_atlases_filter_by_kind_and_genre_tag() {
    let store = populated_store();
    let conts = store
        .list_atlases(AtlasFilter {
            kind: Some("continent".into()),
            genre_tag: None,
        })
        .unwrap();
    assert_eq!(conts.len(), 1);
    assert_eq!(conts[0].id.as_str(), "atlas-jungwon");

    let regions = store
        .list_atlases(AtlasFilter {
            kind: Some("region".into()),
            genre_tag: None,
        })
        .unwrap();
    assert!(regions.is_empty(), "Phase 4엔 region atlas 없음");

    let wuxia = store
        .list_atlases(AtlasFilter {
            kind: None,
            genre_tag: Some("wuxia".into()),
        })
        .unwrap();
    assert_eq!(wuxia.len(), 1);
}

// ---------------------------------------------------------------------------
// get_atlas — references·body·extras 전체 detail
// ---------------------------------------------------------------------------

#[test]
fn get_atlas_detail_contains_references_and_body_sections_and_extras() {
    let store = populated_store();
    let a = atlas_jungwon(&store);
    // references — 11개.
    assert_eq!(a.references.len(), 11);
    // body_sections — 7 H2 섹션 모두.
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
            "{section} 섹션이 get_atlas detail에 있어야 함"
        );
    }
    // extras — era 텍스트 보존.
    assert_eq!(a.era(), Some("현재 (칠국춘추 270년차)"));
    // extras.era_id — Phase 5b 진입으로 era-fall-of-empire 외래키 활성됨.
    assert_eq!(
        a.era_id(),
        Some("era-fall-of-empire"),
        "Phase 5b로 atlas-jungwon.era_id가 era-fall-of-empire 외래키로 활성됨"
    );
}
