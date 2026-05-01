//! Phase 2 Follow-up — kind="player" 단독 슬라이스 검증.
//!
//! 사양 `task-phase2-followup-player-character.md` Done Criteria 자동화:
//! - `projects/chilguk-chunchu/world/person/player.md` 파싱 + kind="player"
//! - `worldbuilding::mind_sync::person_to_npc`이 player를 적격으로 처리
//! - SqliteWorldStore 라운드트립 + 다른 active 인물과 공존
//! - HEXACO 시작값(§3.3 권장값) 정확 매칭 회귀 가드
//! - aliases·affiliation·extras 보존
//!
//! 실행: `cargo test --features embed --test world_chilguk_chunchu_player_e2e`

#![cfg(feature = "embed")]

use std::path::PathBuf;

use npc_mind::adapter::sqlite_world::SqliteWorldStore;
use npc_mind::domain::world::{PersonFilter, PersonId, PersonStatus};
use npc_mind::worldbuilding::WorldRepository;
use npc_mind::worldbuilding::markdown::person_from_markdown;
use npc_mind::worldbuilding::mind_sync::person_to_npc;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_player() -> npc_mind::domain::world::Person {
    let path = project_root()
        .join("projects/chilguk-chunchu/world/person/player.md");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read player.md: {e}"));
    person_from_markdown(&raw).unwrap_or_else(|e| panic!("parse player.md: {e}"))
}

#[test]
fn player_parses_with_correct_kind_and_id() {
    let p = load_player();
    assert_eq!(p.id.as_str(), "player");
    assert_eq!(p.kind, "player");
    assert_eq!(p.status, PersonStatus::Alive);
    assert_eq!(p.temporal.age_at_game_start, Some(17));
    assert!(p.affiliation.is_empty(), "무소속 — affiliation 빈 의도");
}

#[test]
fn player_hexaco_matches_recommended_baseline() {
    // §3.3 디렉터 권장값 그대로.
    let p = load_player();
    let h = p.hexaco;
    assert!((h.honesty_humility.value() - 0.5).abs() < 1e-6);
    assert!((h.emotionality.value() - 0.3).abs() < 1e-6);
    assert!((h.extraversion.value() - 0.0).abs() < 1e-6);
    assert!((h.agreeableness.value() - 0.4).abs() < 1e-6);
    assert!((h.conscientiousness.value() - 0.5).abs() < 1e-6);
    assert!((h.openness.value() - 0.5).abs() < 1e-6);
}

#[test]
fn player_is_mind_eligible() {
    // Q2·B 정책 — kind="player"는 mind 적격이어야 active와 동일 흐름으로 등록.
    let p = load_player();
    assert!(p.is_mind_eligible());
    let npc = person_to_npc(&p).expect("kind=player는 person_to_npc Some 반환");
    assert_eq!(npc.id(), "player");
    assert!(!npc.description().is_empty(), "summary가 description으로 전달되어야 함");

    // 6 dim 평균이 baseline과 일치.
    let avg = npc.personality().dimension_averages();
    assert!((avg.h.value() - 0.5).abs() < 1e-6);
    assert!((avg.e.value() - 0.3).abs() < 1e-6);
    assert!((avg.o.value() - 0.5).abs() < 1e-6);

    // LLM 파라미터 도출 가능 — derive_llm_parameters가 정상 동작.
    let (temp, top_p) = npc.derive_llm_parameters();
    assert!(temp.is_finite() && temp > 0.0 && temp < 2.0);
    assert!(top_p.is_finite() && top_p > 0.0 && top_p <= 1.0);
}

#[test]
fn player_sqlite_roundtrip_preserves_all_fields() {
    let p = load_player();
    let store = SqliteWorldStore::in_memory().expect("sqlite in-memory");
    store.upsert_person("chilguk-chunchu", &p).expect("upsert");

    let back = store
        .get_person(&PersonId::new("player"))
        .expect("get_person")
        .expect("Some");
    assert_eq!(back.kind, "player");
    assert_eq!(back.hexaco, p.hexaco);
    assert_eq!(back.aliases, p.aliases);
    assert!(back.extras.contains_key("starting_inventory"));
    assert_eq!(
        back.extras.get("player_init").and_then(|v| v.as_bool()),
        Some(true),
        "player_init 마커 보존"
    );
    assert!(back.body_sections.contains_key("HEXACO 분석"));
}

#[test]
fn list_persons_kind_player_returns_only_player() {
    // 다른 active 인물과 공존해도 kind=player 필터로 정확히 분리.
    let store = SqliteWorldStore::in_memory().unwrap();
    let player = load_player();
    store.upsert_person("test", &player).unwrap();

    // 가짜 active 인물도 추가 — 필터가 정확히 걸러내는지.
    let mut active = npc_mind::domain::world::Person::new("npc-test", "active", "테스트");
    store.upsert_person("test", &active).unwrap();
    active.id = npc_mind::domain::world::PersonId::new("npc-test-2");
    store.upsert_person("test", &active).unwrap();

    let players = store
        .list_persons(PersonFilter {
            kind: Some("player".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(players.len(), 1);
    assert_eq!(players[0].id.as_str(), "player");

    let actives = store
        .list_persons(PersonFilter {
            kind: Some("active".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(actives.len(), 2);
    assert!(!actives.iter().any(|p| p.kind == "player"));
}

#[test]
fn player_count_combined_with_seven_actives_is_eight() {
    // 사양 §4 — world-load 후 mind eligible = 8 (active 7 + player 1).
    // 본 테스트는 SqliteWorldStore 레벨에서 8 인물 공존을 검증.
    let store = SqliteWorldStore::in_memory().unwrap();
    let player = load_player();
    store.upsert_person("test", &player).unwrap();

    // 가상 active 7명 (실 .md 의존성 없이 단순 검증).
    for i in 0..7 {
        let p = npc_mind::domain::world::Person::new(
            format!("npc-{i}"),
            "active",
            format!("Person {i}"),
        );
        store.upsert_person("test", &p).unwrap();
    }

    assert_eq!(store.count_persons(None).unwrap(), 8);

    // 모두 mind 적격 (active + player).
    let all = store.list_persons(PersonFilter::default()).unwrap();
    let eligible_count = all.iter().filter(|p| p.is_mind_eligible()).count();
    assert_eq!(eligible_count, 8);
}

#[test]
fn player_extras_carry_starting_inventory() {
    // player_init·starting_inventory·starting_location 마커 검증.
    let p = load_player();

    let inventory = p
        .extras
        .get("starting_inventory")
        .and_then(|v| v.as_array())
        .expect("starting_inventory 배열 필요");
    assert!(!inventory.is_empty(), "최소 1개 항목");
    assert!(
        inventory.iter().any(|v| v.as_str().is_some_and(|s| s.contains("혈매화검"))),
        "혈매화검 보유 — 화산파 멸문 후 유일한 연결고리"
    );

    let starting_location = p
        .extras
        .get("starting_location")
        .and_then(|v| v.as_str())
        .expect("starting_location 필요");
    assert!(starting_location.contains("free-cities"));
}
