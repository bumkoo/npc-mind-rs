//! Phase 3 Checkpoint 2 — chilguk-chunchu 전체 11 Place 정성 검증 E2E.
//!
//! 사양 `task-phase3-place-vertical-slice.md` §5 Step 5 자동화 + 디렉터 결정 5개 검증:
//! - 11 Place 등록 (8 settlement + 3 geography)
//! - sect 이중 등록 (place-namgung-sega ↔ group-namgung 양방향 외래키)
//! - geography_refs 양방향 (place-bukwon ↔ place-bukwon-grasslands 등)
//! - 외래키 결손 0건 (Phase 1·2 시드 hq/birthplace/current_location 모두 places에 존재)
//! - search_places 6쿼리: "검성"·"독관성"·"낙양"·"산악"·"초원"·"밀림"
//! - layer 필터·parent_place 필터·kind 필터 모두 동작
//!
//! 실행: `cargo test --features embed --test world_chilguk_chunchu_phase3_checkpoint2`

#![cfg(feature = "embed")]

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use npc_mind::adapter::sqlite_world::SqliteWorldStore;
use npc_mind::domain::world::{
    Group, GroupId, Person, Place, PlaceFilter, PlaceId, PlaceLayer,
};
use npc_mind::worldbuilding::WorldRepository;
use npc_mind::worldbuilding::markdown::{
    group_from_markdown, person_from_markdown, place_from_markdown,
};

const PROJECT: &str = "chilguk-chunchu";

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_all<T, F>(subdir: &str, parse: F) -> Vec<T>
where
    F: Fn(&str) -> T,
{
    let dir = project_root().join("projects").join(PROJECT).join("world").join(subdir);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|_| panic!("missing dir: {}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let raw = std::fs::read_to_string(&p).expect("read .md");
            parse(&raw)
        })
        .collect()
}

fn fresh_indexed_store() -> SqliteWorldStore {
    let store = SqliteWorldStore::in_memory().unwrap();
    let groups: Vec<Group> = load_all("group", |raw| group_from_markdown(raw).expect("group"));
    let persons: Vec<Person> = load_all("person", |raw| person_from_markdown(raw).expect("person"));
    let places: Vec<Place> = load_all("place", |raw| place_from_markdown(raw).expect("place"));
    for g in &groups {
        store.upsert_group(PROJECT, g).unwrap();
    }
    for p in &persons {
        store.upsert_person(PROJECT, p).unwrap();
    }
    for pl in &places {
        store.upsert_place(PROJECT, pl).unwrap();
    }
    store
}

#[test]
fn places_indexed_with_expected_counts() {
    let store = fresh_indexed_store();
    assert_eq!(store.count_places(Some(PROJECT)).unwrap(), 11);

    // settlement 8건 (7국 + namgung-sega sect)
    let settlements = store
        .list_places(PlaceFilter {
            layer: Some(PlaceLayer::Settlement),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(settlements.len(), 8, "settlements: {:?}", ids(&settlements));

    // geography 3건 (서부 산악 + bukwon-grasslands + namman-jungle)
    let geographies = store
        .list_places(PlaceFilter {
            layer: Some(PlaceLayer::Geography),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(geographies.len(), 3, "geographies: {:?}", ids(&geographies));

    // 7국(nation) settlement
    let nations = store
        .list_places(PlaceFilter {
            kind: Some("nation".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(nations.len(), 6, "nation: {:?}", ids(&nations));
    // (autonomous-zone 1건 — 자유도시)
    let autozones = store
        .list_places(PlaceFilter {
            kind: Some("autonomous-zone".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(autozones.len(), 1);
    assert_eq!(autozones[0].id.as_str(), "place-jiyu-doshi");
    // sect 1건
    let sects = store
        .list_places(PlaceFilter {
            kind: Some("sect".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(sects.len(), 1);
    assert_eq!(sects[0].id.as_str(), "place-namgung-sega");
}

#[test]
fn list_places_filter_by_parent_place_returns_namgung_sega() {
    let store = fresh_indexed_store();
    let kids = store
        .list_places(PlaceFilter {
            parent_place: Some(PlaceId::new("place-namgung")),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(kids.len(), 1);
    assert_eq!(kids[0].id.as_str(), "place-namgung-sega");
}

#[test]
fn sect_double_registration_bidirectional() {
    // sect 이중 등록: place-namgung-sega ↔ group-namgung 양방향 외래키.
    let store = fresh_indexed_store();

    // Place → Group 방향
    let sega = store
        .get_place(&PlaceId::new("place-namgung-sega"))
        .unwrap()
        .unwrap();
    assert_eq!(sega.layer, PlaceLayer::Settlement);
    assert_eq!(sega.kind, "sect");
    assert_eq!(sega.controlling_group(), Some("group-namgung"));
    assert_eq!(sega.spatial.parent_place.as_ref().unwrap().as_str(), "place-namgung");

    // Group → Place 방향
    let namgung_group = store
        .get_group(&GroupId::new("group-namgung"))
        .unwrap()
        .unwrap();
    assert_eq!(namgung_group.headquarters.as_deref(), Some("place-namgung-sega"));
}

#[test]
fn geography_refs_bidirectional_with_bukwon() {
    // place-bukwon (settlement) ↔ place-bukwon-grasslands (geography)
    // - settlement.geography_refs에 geography가 있고
    // - geography.bordering_places에 settlement가 있어 cross-layer 인접 시연.
    let store = fresh_indexed_store();
    let bukwon = store
        .get_place(&PlaceId::new("place-bukwon"))
        .unwrap()
        .unwrap();
    assert!(
        bukwon
            .spatial
            .geography_refs
            .iter()
            .any(|g| g.as_str() == "place-bukwon-grasslands"),
        "place-bukwon.spatial.geography_refs에 place-bukwon-grasslands 있어야 함"
    );

    let grass = store
        .get_place(&PlaceId::new("place-bukwon-grasslands"))
        .unwrap()
        .unwrap();
    assert_eq!(grass.layer, PlaceLayer::Geography);
    assert_eq!(grass.kind, "grassland");
    assert!(
        grass
            .spatial
            .bordering_places
            .iter()
            .any(|b| b.as_str() == "place-bukwon"),
        "place-bukwon-grasslands.spatial.bordering_places에 place-bukwon 있어야 함"
    );
}

#[test]
fn geography_refs_layer_constraint_holds() {
    // 모든 settlement의 geography_refs target은 layer=Geography이어야 (world-load FK가 검증).
    // 본 테스트는 중복 안전망: SQLite에 적재된 데이터 자체에서 invariant 유지 확인.
    let store = fresh_indexed_store();
    let settlements = store
        .list_places(PlaceFilter {
            layer: Some(PlaceLayer::Settlement),
            ..Default::default()
        })
        .unwrap();
    for s in &settlements {
        for gref in &s.spatial.geography_refs {
            let target = store.get_place(gref).unwrap().unwrap_or_else(|| {
                panic!("{} → geography_refs '{}' 결손", s.id, gref)
            });
            assert_eq!(
                target.layer,
                PlaceLayer::Geography,
                "{}.geography_refs '{}' 의 layer가 geography 여야 함 — 실제: {:?}",
                s.id,
                gref,
                target.layer
            );
        }
    }
}

#[test]
fn fk_zero_phase1_phase2_seeds_all_resolve() {
    // Phase 1·2 시드의 모든 hq/birthplace/current_location ID가 places에 존재.
    let store = fresh_indexed_store();
    let groups = store.list_groups(Default::default()).unwrap();
    let persons = store.list_persons(Default::default()).unwrap();
    let places = store.list_places(Default::default()).unwrap();
    let place_ids: HashSet<&str> = places.iter().map(|p| p.id.as_str()).collect();

    let mut missing: Vec<String> = Vec::new();
    for g in &groups {
        if let Some(hq) = &g.headquarters
            && !hq.is_empty()
            && !place_ids.contains(hq.as_str())
        {
            missing.push(format!("{}.headquarters → {}", g.id, hq));
        }
    }
    for p in &persons {
        if let Some(b) = &p.birthplace
            && !b.is_empty()
            && !place_ids.contains(b.as_str())
        {
            missing.push(format!("{}.birthplace → {}", p.id, b));
        }
        if let Some(c) = &p.current_location
            && !c.is_empty()
            && !place_ids.contains(c.as_str())
        {
            missing.push(format!("{}.current_location → {}", p.id, c));
        }
    }
    assert!(missing.is_empty(), "외래키 결손 잔여: {:#?}", missing);
}

#[test]
fn search_places_six_queries_match_expected_targets() {
    let store = fresh_indexed_store();

    // 사양 §5 Step 5 + 디렉터 6쿼리.
    let cases: &[(&str, &str)] = &[
        ("검성", "place-namgung"),               // body·extras에 검성(劍城)
        ("독관성", "place-seoryang"),            // body·extras에 독관성(毒關城)
        ("낙양", "place-daejin"),                // body·extras에 낙양(洛陽)
        ("산악", "place-western-mountains"),     // alias 산맥 + body 산악
        ("초원", "place-bukwon-grasslands"),     // body 초원
        ("밀림", "place-namman-jungle"),         // body 밀림
    ];

    for (q, expected) in cases {
        let hits = store.search_places(q, 10).unwrap();
        assert!(
            hits.iter().any(|p| p.id.as_str() == *expected),
            "search_places({q:?})가 {expected}를 매치하지 못함 — hits: {:?}",
            ids(&hits)
        );
    }
}

#[test]
fn parent_place_cycle_is_zero_for_full_dataset() {
    use npc_mind::domain::world::detect_parent_place_cycle;
    let places = load_all("place", |raw| place_from_markdown(raw).expect("place"));
    let cycles = detect_parent_place_cycle(&places);
    assert!(cycles.is_empty(), "cycles 0건이어야 함: {cycles:?}");
}

#[test]
fn place_daejin_borders_resolve_after_step4() {
    // 체크포인트 1엔 의도적 결손이었던 place-daejin.spatial.bordering_places가
    // Step 4의 6 settlement 추가로 모두 places에 존재해야 함.
    let store = fresh_indexed_store();
    let daejin = store
        .get_place(&PlaceId::new("place-daejin"))
        .unwrap()
        .unwrap();
    for b in &daejin.spatial.bordering_places {
        let target = store.get_place(b).unwrap();
        assert!(
            target.is_some(),
            "place-daejin.bordering_places '{}' 가 places에 없음",
            b
        );
    }
}

fn ids(places: &[Place]) -> Vec<&str> {
    places.iter().map(|p| p.id.as_str()).collect()
}

// 컴파일러가 안 쓰는 import를 경고 안 내도록.
#[allow(dead_code)]
fn _unused(_: &Path) {}
