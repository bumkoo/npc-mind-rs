//! Phase 3 Checkpoint 1 — chilguk-chunchu 대진(settlement) + 서부 산악(geography)
//! 두 layer 단독 변환 엔드투엔드 테스트.
//!
//! 사양 `task-phase3-place-vertical-slice.md` §5 Step 3 자동화:
//! - `world/place/place-daejin.md` (settlement) + `world/place/place-western-mountains.md` (geography) 파싱
//! - SqliteWorldStore 라운드트립 — 두 Place 모든 필드 보존
//! - layer 분기 검증 — 두 Place가 각자 다른 layer enum + extras 사용
//! - aliases·spatial·extras 정합성 검증
//! - FTS5 검색 — 별호("낙양"·"산악"·"중원 황도") 매칭
//! - parent_place cycle 검출 — 두 Place 모두 parent 없음 → cycles 0건
//!
//! world-load CLI는 Phase 1·2 시드의 미해결 Place ID 참조 때문에 의도적으로 fail하므로
//! 본 통합 테스트는 SQLite 라운드트립과 layer 분기 검증을 별도로 수행한다.
//!
//! 실행: `cargo test --features embed --test world_chilguk_chunchu_phase3_checkpoint1`

#![cfg(feature = "embed")]

use std::path::PathBuf;

use npc_mind::adapter::sqlite_world::SqliteWorldStore;
use npc_mind::domain::world::{
    Place, PlaceFilter, PlaceId, PlaceLayer, detect_parent_place_cycle,
};
use npc_mind::worldbuilding::WorldRepository;
use npc_mind::worldbuilding::markdown::place_from_markdown;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_place(filename: &str) -> Place {
    let path = project_root()
        .join("projects/chilguk-chunchu/world/place")
        .join(filename);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut p = place_from_markdown(&raw)
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    p.source_path = Some(path.to_string_lossy().to_string());
    p
}

#[test]
fn daejin_settlement_parses_with_expected_identity() {
    let p = load_place("place-daejin.md");
    assert_eq!(p.id.as_str(), "place-daejin");
    assert_eq!(p.layer, PlaceLayer::Settlement);
    assert_eq!(p.kind, "nation");
    assert_eq!(p.name, "대진(大辰)");

    // aliases — 작성 시 결정한 3개
    assert_eq!(p.aliases.len(), 3);
    assert!(p.aliases.contains(&"중원 황도".to_string()));
    assert!(p.aliases.contains(&"옛 통일제국".to_string()));
    assert!(p.aliases.contains(&"축소 제국".to_string()));

    // 무협 특화 extras
    assert_eq!(p.extras.get("capital").and_then(|v| v.as_str()), Some("낙양(洛陽)"));
    assert_eq!(
        p.extras.get("ki_concentration").and_then(|v| v.as_str()),
        Some("보통")
    );
    // sect 이중 등록 패턴은 nation에서도 controlling_group으로 황실 외래키 표현 가능.
    assert_eq!(p.controlling_group(), Some("group-daejin-court"));

    // spatial — 최상위 정치체
    assert!(p.spatial.parent_place.is_none());
    assert_eq!(p.spatial.relative_position.as_deref(), Some("center"));
    assert_eq!(p.spatial.bordering_places.len(), 3);
    // geography_refs는 의도적으로 비움 — place-jungwon-plain은 Phase 4·5+에 정의 예정.
    assert!(p.spatial.geography_refs.is_empty());

    // 권장 H2 섹션 — settlement 양식 6 + 옵션
    assert!(p.body_sections.contains_key("개요"));
    assert!(p.body_sections.contains_key("통치"));
    assert!(p.body_sections.contains_key("핵심 NPC"));
    assert!(p.body_sections.contains_key("핵심 갈등"));
    assert!(p.body_sections.contains_key("플레이어가 방문할 이유"));
    assert!(p.body_sections.contains_key("전사(前史)"));
}

#[test]
fn western_mountains_geography_parses_with_expected_identity() {
    let p = load_place("place-western-mountains.md");
    assert_eq!(p.id.as_str(), "place-western-mountains");
    assert_eq!(p.layer, PlaceLayer::Geography);
    assert_eq!(p.kind, "mountain-range");
    assert_eq!(p.name, "서부 산악지대");

    // aliases — 작성 시 결정한 2개
    assert_eq!(p.aliases.len(), 2);
    assert!(p.aliases.contains(&"서령산맥".to_string()));
    assert!(p.aliases.contains(&"만년설봉".to_string()));

    // geography 특화 extras (작성 시 추론한 값들)
    assert_eq!(
        p.extras.get("terrain_type").and_then(|v| v.as_str()),
        Some("mountain-range")
    );
    let hazards = p
        .extras
        .get("hazards")
        .and_then(|v| v.as_array())
        .expect("hazards가 array여야 함");
    assert_eq!(hazards.len(), 4);
    let signature = p
        .extras
        .get("signature_features")
        .and_then(|v| v.as_array())
        .expect("signature_features가 array여야 함");
    assert_eq!(signature.len(), 3);

    // spatial — geography는 geography_refs 비어 있어야 (settlement에서만 의미)
    assert!(p.spatial.parent_place.is_none());
    assert_eq!(p.spatial.relative_position.as_deref(), Some("west"));
    assert_eq!(p.spatial.bordering_places.len(), 1);
    assert!(p.spatial.geography_refs.is_empty());

    // 권장 H2 섹션 — geography 양식
    assert!(p.body_sections.contains_key("개요"));
    assert!(p.body_sections.contains_key("지형·기후"));
    assert!(p.body_sections.contains_key("위험·서식 생물"));
    assert!(p.body_sections.contains_key("인접 정치체"));
    assert!(p.body_sections.contains_key("자원·산물"));
    assert!(p.body_sections.contains_key("플레이어가 방문할 이유"));
}

#[test]
fn layer_branching_two_places_use_distinct_extras_keys() {
    let daejin = load_place("place-daejin.md");
    let mt = load_place("place-western-mountains.md");
    assert_ne!(daejin.layer, mt.layer);

    // settlement에만 있어야 하는 extras 키
    assert!(daejin.extras.contains_key("capital"));
    assert!(daejin.extras.contains_key("polity"));
    assert!(daejin.extras.contains_key("controlling_group"));
    assert!(!mt.extras.contains_key("capital"));
    assert!(!mt.extras.contains_key("polity"));

    // geography에만 있어야 하는 extras 키
    assert!(mt.extras.contains_key("terrain_type"));
    assert!(mt.extras.contains_key("climate"));
    assert!(mt.extras.contains_key("hazards"));
    assert!(mt.extras.contains_key("signature_features"));
    assert!(!daejin.extras.contains_key("terrain_type"));
    assert!(!daejin.extras.contains_key("hazards"));
}

#[test]
fn sqlite_roundtrip_preserves_all_fields_for_both_layers() {
    let daejin = load_place("place-daejin.md");
    let mt = load_place("place-western-mountains.md");

    let store = SqliteWorldStore::in_memory().unwrap();
    store.upsert_place("chilguk-chunchu", &daejin).unwrap();
    store.upsert_place("chilguk-chunchu", &mt).unwrap();

    let back_daejin = store
        .get_place(&PlaceId::new("place-daejin"))
        .unwrap()
        .unwrap();
    assert_eq!(back_daejin, daejin);

    let back_mt = store
        .get_place(&PlaceId::new("place-western-mountains"))
        .unwrap()
        .unwrap();
    assert_eq!(back_mt, mt);

    // count
    assert_eq!(store.count_places(Some("chilguk-chunchu")).unwrap(), 2);
}

#[test]
fn list_places_filters_by_layer() {
    let daejin = load_place("place-daejin.md");
    let mt = load_place("place-western-mountains.md");

    let store = SqliteWorldStore::in_memory().unwrap();
    store.upsert_place("p", &daejin).unwrap();
    store.upsert_place("p", &mt).unwrap();

    let settlements = store
        .list_places(PlaceFilter {
            layer: Some(PlaceLayer::Settlement),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(settlements.len(), 1);
    assert_eq!(settlements[0].id.as_str(), "place-daejin");

    let geographies = store
        .list_places(PlaceFilter {
            layer: Some(PlaceLayer::Geography),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(geographies.len(), 1);
    assert_eq!(geographies[0].id.as_str(), "place-western-mountains");
}

#[test]
fn search_places_matches_alias_and_body() {
    let daejin = load_place("place-daejin.md");
    let mt = load_place("place-western-mountains.md");

    let store = SqliteWorldStore::in_memory().unwrap();
    store.upsert_place("p", &daejin).unwrap();
    store.upsert_place("p", &mt).unwrap();

    // alias 매칭 — "중원 황도"는 daejin alias
    let hits = store.search_places("중원 황도", 5).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id.as_str(), "place-daejin");

    // body 매칭 — "산악"은 mt 본문에 다수 등장
    let hits = store.search_places("산악", 5).unwrap();
    assert!(hits.iter().any(|p| p.id.as_str() == "place-western-mountains"));

    // body 매칭 — "낙양"은 daejin 본문·extras에 등장
    let hits = store.search_places("낙양", 5).unwrap();
    assert!(hits.iter().any(|p| p.id.as_str() == "place-daejin"));
}

/// 보고서용 JSON dump — 평소엔 ignore. 실행:
/// `cargo test --features embed --test world_chilguk_chunchu_phase3_checkpoint1 \
///    dump_places_json -- --ignored --nocapture`
#[test]
#[ignore]
fn dump_places_json() {
    for name in ["place-daejin.md", "place-western-mountains.md"] {
        let p = load_place(name);
        // source_path는 환경 의존이라 dump에서 제거.
        let mut p = p;
        p.source_path = None;
        let json = serde_json::to_string_pretty(&p).unwrap();
        println!("=== {name} ===");
        println!("{json}");
        println!();
    }
}

#[test]
fn parent_place_cycle_detection_passes_for_two_top_level_places() {
    // 두 Place 모두 parent_place=null → cycle 0건.
    let daejin = load_place("place-daejin.md");
    let mt = load_place("place-western-mountains.md");
    let cycles = detect_parent_place_cycle(&[daejin, mt]);
    assert!(
        cycles.is_empty(),
        "checkpoint 1엔 cycle이 없어야 함: {cycles:?}"
    );
}
