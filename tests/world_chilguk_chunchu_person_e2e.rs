//! Phase 2 Checkpoint 1 — chilguk-chunchu npc-02 조고 단독 변환 엔드투엔드 테스트.
//!
//! 사양 `task-phase2-person-vertical-slice.md` §5 Step 3 자동화:
//! - `world/person/npc-02.md` → Person 파싱
//! - SqliteWorldStore 라운드트립
//! - HEXACO 6 dim 값 정합성 (§6.1 결정값)
//! - affiliation 외래키 활성 — group-daejin-court / group-shipsangsi에 npc-02 존재 검증
//! - npc-mind 변환 — Person → Npc 가능 여부
//! - FTS5 검색 — 별호("대진의 그림자") 매칭
//!
//! 실행: `cargo test --features embed --test world_chilguk_chunchu_person_e2e`

#![cfg(feature = "embed")]

use std::path::PathBuf;

use npc_mind::adapter::sqlite_world::SqliteWorldStore;
use npc_mind::domain::world::{
    GroupFilter, GroupId, Person, PersonFilter, PersonId, PersonStatus,
};
use npc_mind::worldbuilding::WorldRepository;
use npc_mind::worldbuilding::markdown::{group_from_markdown, person_from_markdown};
use npc_mind::worldbuilding::mind_sync::person_to_npc;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_npc02() -> Person {
    let path = project_root()
        .join("projects/chilguk-chunchu/world/person/npc-02.md");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read npc-02.md: {e}"));
    let mut p = person_from_markdown(&raw)
        .unwrap_or_else(|e| panic!("parse npc-02.md: {e}"));
    p.source_path = Some(path.to_string_lossy().to_string());
    p
}

fn load_all_groups() -> Vec<npc_mind::domain::world::Group> {
    let dir = project_root().join("projects/chilguk-chunchu/world/group");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|_| panic!("missing dir: {}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md"))
        .collect();
    paths.sort();
    let mut out = Vec::new();
    for p in paths {
        let raw = std::fs::read_to_string(&p).expect("read .md");
        out.push(group_from_markdown(&raw).unwrap_or_else(|e| panic!("{}: {e}", p.display())));
    }
    out
}

#[test]
fn npc02_parses_with_expected_identity() {
    let p = load_npc02();
    assert_eq!(p.id.as_str(), "npc-02");
    assert_eq!(p.kind, "active");
    assert!(p.name.contains("조고"));
    assert_eq!(p.status, PersonStatus::Alive);
    assert!(p.aliases.iter().any(|a| a.contains("대진의 그림자")));
    assert!(p.aliases.iter().any(|a| a.contains("십상시의 주인")));
    assert_eq!(p.temporal.age_at_game_start, Some(55));
}

#[test]
fn npc02_hexaco_matches_decision_values() {
    // §6.1 결정값. Score VO 범위 (-1.0~+1.0) 자동 검증.
    let p = load_npc02();
    let h = p.hexaco;
    assert!((h.honesty_humility.value() - -0.8).abs() < 1e-6);
    assert!((h.emotionality.value() - -0.3).abs() < 1e-6);
    assert!((h.extraversion.value() - -0.2).abs() < 1e-6);
    assert!((h.agreeableness.value() - -0.7).abs() < 1e-6);
    assert!((h.conscientiousness.value() - 0.7).abs() < 1e-6);
    assert!((h.openness.value() - 0.5).abs() < 1e-6);
}

#[test]
fn npc02_affiliation_references_existing_groups() {
    let p = load_npc02();
    let groups = load_all_groups();
    let group_ids: std::collections::HashSet<&str> =
        groups.iter().map(|g| g.id.as_str()).collect();

    assert!(p.affiliation.contains(&GroupId::new("group-daejin-court")));
    assert!(p.affiliation.contains(&GroupId::new("group-shipsangsi")));
    for a in &p.affiliation {
        assert!(
            group_ids.contains(a.as_str()),
            "Phase 2 외래키 활성: affiliation '{a}'이(가) groups에 없음"
        );
    }
}

#[test]
fn group_members_referencing_npc02_pass_fk_validation() {
    // 사양 Step 3 #5: group-daejin-court / group-shipsangsi의 members.npc-02 검증.
    let groups = load_all_groups();
    let by_id: std::collections::HashMap<&str, _> =
        groups.iter().map(|g| (g.id.as_str(), g)).collect();

    let dc = by_id.get("group-daejin-court").expect("group-daejin-court 필요");
    assert!(
        dc.members.iter().any(|m| m.person_id.as_deref() == Some("npc-02")),
        "group-daejin-court.members에 npc-02 참조 필요"
    );
    let sh = by_id.get("group-shipsangsi").expect("group-shipsangsi 필요");
    assert!(
        sh.members.iter().any(|m| m.person_id.as_deref() == Some("npc-02")),
        "group-shipsangsi.members에 npc-02 참조 필요"
    );

    // npc-02가 실제로 persons에 존재함 → FK 통과.
    let p = load_npc02();
    assert_eq!(p.id.as_str(), "npc-02");
}

#[test]
fn npc02_sqlite_roundtrip_preserves_all_fields() {
    let p = load_npc02();
    let store = SqliteWorldStore::in_memory().expect("sqlite in-memory");
    store.upsert_person("chilguk-chunchu", &p).expect("upsert");

    let back = store
        .get_person(&PersonId::new("npc-02"))
        .expect("get_person")
        .expect("Some");

    // hexaco·affiliation·temporal·aliases·extras 모두 보존.
    assert_eq!(back.hexaco, p.hexaco);
    assert_eq!(back.affiliation, p.affiliation);
    assert_eq!(back.temporal, p.temporal);
    assert_eq!(back.aliases, p.aliases);
    assert_eq!(back.tags, p.tags);
    assert!(back.extras.contains_key("game_role"));
    assert!(back.body_sections.contains_key("개요"));
    assert!(back.body_sections.contains_key("HEXACO 분석"));
}

#[test]
fn npc02_search_matches_alias_and_summary() {
    let p = load_npc02();
    let store = SqliteWorldStore::in_memory().unwrap();
    store.upsert_person("chilguk-chunchu", &p).unwrap();

    // alias 매칭 — 별호 "대진의 그림자".
    let hits = store.search_persons("대진의 그림자", 5).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id.as_str(), "npc-02");

    // summary 매칭.
    let hits = store.search_persons("환관 출신", 5).unwrap();
    assert!(!hits.is_empty(), "summary/body에 '환관 출신' 매칭 기대");
    assert_eq!(hits[0].id.as_str(), "npc-02");
}

#[test]
fn npc02_filter_by_affiliation() {
    let p = load_npc02();
    let store = SqliteWorldStore::in_memory().unwrap();
    store.upsert_person("chilguk-chunchu", &p).unwrap();

    let hits = store
        .list_persons(PersonFilter {
            affiliation: Some(GroupId::new("group-daejin-court")),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id.as_str(), "npc-02");

    let hits = store
        .list_persons(PersonFilter {
            affiliation: Some(GroupId::new("group-shipsangsi")),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(hits.len(), 1);

    // 무관한 그룹은 0건.
    let hits = store
        .list_persons(PersonFilter {
            affiliation: Some(GroupId::new("group-namgung")),
            ..Default::default()
        })
        .unwrap();
    assert!(hits.is_empty());
}

#[test]
fn npc02_converts_to_npc_with_correct_personality() {
    // npc-mind 통합 — Person → Npc 변환 (HEXACO Score VO 범위 검증 + 4 facet spread).
    let p = load_npc02();
    let npc = person_to_npc(&p).expect("active kind should produce Npc");
    assert_eq!(npc.id(), "npc-02");
    assert!(npc.name().contains("조고"));
    assert!(!npc.description().is_empty(), "description = summary");

    // 6 dim 평균이 입력값과 일치.
    let avg = npc.personality().dimension_averages();
    assert!((avg.h.value() - -0.8).abs() < 1e-6);
    assert!((avg.c.value() - 0.7).abs() < 1e-6);

    // LLM parameter 유도 — H 매우 낮음 + C 높음 → temperature가 base보다 낮을 것.
    let (temp, top_p) = npc.derive_llm_parameters();
    assert!(temp.is_finite() && temp > 0.0 && temp < 2.0);
    assert!(top_p.is_finite() && top_p > 0.0 && top_p <= 1.0);
}

#[test]
fn person_count_after_load_is_one() {
    let p = load_npc02();
    let store = SqliteWorldStore::in_memory().unwrap();
    store.upsert_person("chilguk-chunchu", &p).unwrap();
    assert_eq!(store.count_persons(Some("chilguk-chunchu")).unwrap(), 1);
    assert_eq!(store.count_persons(None).unwrap(), 1);
}

#[test]
fn list_persons_kind_active_returns_npc02() {
    let p = load_npc02();
    let store = SqliteWorldStore::in_memory().unwrap();
    store.upsert_person("chilguk-chunchu", &p).unwrap();

    let actives = store
        .list_persons(PersonFilter {
            kind: Some("active".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(actives.len(), 1);
    assert_eq!(actives[0].id.as_str(), "npc-02");
    assert_eq!(actives[0].status, PersonStatus::Alive);
}

#[test]
fn group_filter_unaffected_by_persons_table() {
    // 회귀 가드: persons 테이블 추가가 groups 쿼리에 영향 없어야 함.
    let p = load_npc02();
    let groups = load_all_groups();
    let store = SqliteWorldStore::in_memory().unwrap();
    for g in &groups {
        store.upsert_group("chilguk-chunchu", g).unwrap();
    }
    store.upsert_person("chilguk-chunchu", &p).unwrap();

    let imperials = store
        .list_groups(GroupFilter {
            alignment: Some("imperial".into()),
            ..Default::default()
        })
        .unwrap();
    assert!(imperials.iter().any(|g| g.id.as_str() == "group-daejin-court"));
}
