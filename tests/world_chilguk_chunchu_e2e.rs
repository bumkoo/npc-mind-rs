//! Phase 1 Vertical Slice — chilguk-chunchu 6 Group SoT 엔드투엔드 테스트.
//!
//! 사양 §5 체크포인트 2 정성 검증을 자동화:
//! - 6개 .md → Group 파싱 → SqliteWorldStore 라운드트립
//! - parent_group cycle 없음
//! - kind/parent/alignment 필터 정확성
//! - FTS5 trigram 매치 (alias, body, summary)
//!
//! 모델 파일 미필요 (FTS5만 사용, 임베딩 없음). embed feature만 활성하면 동작.
//!
//! 실행: `cargo test --features embed --test world_chilguk_chunchu_e2e`

#![cfg(feature = "embed")]

use std::collections::HashSet;
use std::path::PathBuf;

use npc_mind::adapter::sqlite_world::SqliteWorldStore;
use npc_mind::domain::world::{
    Group, GroupFilter, GroupId, GroupStatus, detect_parent_group_cycle,
};
use npc_mind::worldbuilding::WorldRepository;
use npc_mind::worldbuilding::markdown::group_from_markdown;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_all_groups() -> Vec<Group> {
    let dir = project_root()
        .join("projects/chilguk-chunchu/world/group");
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
        let mut g = group_from_markdown(&raw)
            .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        g.source_path = Some(p.to_string_lossy().to_string());
        out.push(g);
    }
    out
}

fn fresh_store_with_all() -> (SqliteWorldStore, Vec<Group>) {
    let groups = load_all_groups();
    assert_eq!(groups.len(), 6, "checkpoint 2 — 6 Group .md 필요");
    let store = SqliteWorldStore::in_memory().expect("sqlite in-memory");
    for g in &groups {
        store
            .upsert_group("chilguk-chunchu", g)
            .expect("upsert");
    }
    (store, groups)
}

#[test]
fn six_groups_parse_without_errors() {
    let groups = load_all_groups();
    let ids: HashSet<&str> = groups.iter().map(|g| g.id.as_str()).collect();
    for expected in [
        "group-daejin-court",
        "group-shipsangsi",
        "group-namgung",
        "group-mulim-mang",
        "group-cheonma-shingyo",
        "group-gaebang",
    ] {
        assert!(ids.contains(expected), "missing: {expected}");
    }
}

#[test]
fn no_parent_group_cycles() {
    let groups = load_all_groups();
    let cycles = detect_parent_group_cycle(&groups);
    assert!(
        cycles.is_empty(),
        "checkpoint 2 — parent_group cycle 검출됨: {:?}",
        cycles
    );
}

#[test]
fn shipsangsi_parent_resolves_to_daejin_court() {
    let groups = load_all_groups();
    let s = groups
        .iter()
        .find(|g| g.id.as_str() == "group-shipsangsi")
        .unwrap();
    assert_eq!(
        s.parent_group.as_ref().map(|g| g.as_str()),
        Some("group-daejin-court"),
        "수직 포함 시연 — 십상시 parent_group은 대진 황실"
    );
}

#[test]
fn list_filter_kind_alliance_returns_mulim_mang() {
    let (store, _) = fresh_store_with_all();
    let hits = store
        .list_groups(GroupFilter {
            kind: Some("alliance".into()),
            ..Default::default()
        })
        .unwrap();
    let ids: Vec<&str> = hits.iter().map(|g| g.id.as_str()).collect();
    assert_eq!(ids, vec!["group-mulim-mang"]);
}

#[test]
fn list_filter_kind_clan_returns_namgung() {
    let (store, _) = fresh_store_with_all();
    let hits = store
        .list_groups(GroupFilter {
            kind: Some("clan".into()),
            ..Default::default()
        })
        .unwrap();
    let ids: Vec<&str> = hits.iter().map(|g| g.id.as_str()).collect();
    assert_eq!(ids, vec!["group-namgung"]);
}

#[test]
fn list_filter_parent_daejin_returns_shipsangsi() {
    let (store, _) = fresh_store_with_all();
    let hits = store
        .list_groups(GroupFilter {
            parent_group: Some(GroupId::new("group-daejin-court")),
            ..Default::default()
        })
        .unwrap();
    let ids: Vec<&str> = hits.iter().map(|g| g.id.as_str()).collect();
    assert_eq!(ids, vec!["group-shipsangsi"], "수직 포함 시연");
}

#[test]
fn list_filter_alignment_orthodox_returns_three() {
    let (store, _) = fresh_store_with_all();
    let hits = store
        .list_groups(GroupFilter {
            alignment: Some("orthodox".into()),
            ..Default::default()
        })
        .unwrap();
    let ids: HashSet<&str> = hits.iter().map(|g| g.id.as_str()).collect();
    // 무림맹 + 남궁 + 개방
    assert!(ids.contains("group-mulim-mang"));
    assert!(ids.contains("group-namgung"));
    assert!(ids.contains("group-gaebang"));
    assert_eq!(hits.len(), 3, "wuxia alignment 표준화 3 건");
}

#[test]
fn list_filter_status_declining_includes_daejin_court() {
    let (store, _) = fresh_store_with_all();
    let hits = store
        .list_groups(GroupFilter {
            status: Some(GroupStatus::Declining),
            ..Default::default()
        })
        .unwrap();
    let ids: HashSet<&str> = hits.iter().map(|g| g.id.as_str()).collect();
    assert!(ids.contains("group-daejin-court"));
    assert!(ids.contains("group-mulim-mang"));
}

#[test]
fn rival_relationship_mulim_mang_vs_cheonma() {
    let (store, _) = fresh_store_with_all();
    let mulim = store
        .get_group(&GroupId::new("group-mulim-mang"))
        .unwrap()
        .unwrap();
    assert!(
        mulim
            .rival_groups
            .iter()
            .any(|g| g.as_str() == "group-cheonma-shingyo"),
        "정파 vs 사파 적대 시연"
    );
    let cheonma = store
        .get_group(&GroupId::new("group-cheonma-shingyo"))
        .unwrap()
        .unwrap();
    assert!(
        cheonma
            .rival_groups
            .iter()
            .any(|g| g.as_str() == "group-mulim-mang"),
        "사파 vs 정파 적대 대칭 시연"
    );
}

#[test]
fn search_alias_kupailbang_matches_mulim_mang() {
    let (store, _) = fresh_store_with_all();
    let hits = store.search_groups("구파일방", 5).unwrap();
    let ids: Vec<&str> = hits.iter().map(|g| g.id.as_str()).collect();
    assert!(
        ids.contains(&"group-mulim-mang"),
        "alias 매칭 — 무림맹의 별호 '구파일방' → ids={ids:?}"
    );
}

#[test]
fn search_body_puppet_emperor_matches_daejin() {
    let (store, _) = fresh_store_with_all();
    let hits = store.search_groups("꼭두각시", 5).unwrap();
    let ids: Vec<&str> = hits.iter().map(|g| g.id.as_str()).collect();
    assert!(
        ids.contains(&"group-daejin-court"),
        "body 매칭 — 천순제 꼭두각시 표현 → ids={ids:?}"
    );
}

#[test]
fn search_assassination_matches_shipsangsi() {
    let (store, _) = fresh_store_with_all();
    let hits = store.search_groups("암살", 5).unwrap();
    let ids: Vec<&str> = hits.iter().map(|g| g.id.as_str()).collect();
    assert!(
        ids.contains(&"group-shipsangsi"),
        "body 매칭 — 십상시 암살 활동 영역 → ids={ids:?}"
    );
}

#[test]
fn search_demonic_matches_cheonma_shingyo() {
    let (store, _) = fresh_store_with_all();
    let hits = store.search_groups("사파", 5).unwrap();
    let ids: Vec<&str> = hits.iter().map(|g| g.id.as_str()).collect();
    assert!(
        ids.contains(&"group-cheonma-shingyo"),
        "body 매칭 — 천마신교 = 사파 → ids={ids:?}"
    );
}

#[test]
fn full_roundtrip_preserves_temporal_and_members() {
    let (store, groups) = fresh_store_with_all();
    let daejin_md = groups
        .iter()
        .find(|g| g.id.as_str() == "group-daejin-court")
        .unwrap()
        .clone();
    let daejin_db = store
        .get_group(&GroupId::new("group-daejin-court"))
        .unwrap()
        .unwrap();
    // 모든 핵심 필드 보존 검증. source_path는 SqliteWorldStore가 그대로 저장/복원하나
    // 본 테스트의 in-memory store는 e2e 입력에서 setter로 주입한 절대경로를 그대로 보존.
    assert_eq!(daejin_db.id, daejin_md.id);
    assert_eq!(daejin_db.kind, daejin_md.kind);
    assert_eq!(daejin_db.name, daejin_md.name);
    assert_eq!(daejin_db.aliases, daejin_md.aliases);
    assert_eq!(daejin_db.summary, daejin_md.summary);
    assert_eq!(daejin_db.tags, daejin_md.tags);
    assert_eq!(daejin_db.temporal, daejin_md.temporal);
    assert_eq!(daejin_db.members, daejin_md.members);
    assert_eq!(daejin_db.headquarters, daejin_md.headquarters);
    assert_eq!(daejin_db.rival_groups, daejin_md.rival_groups);
    // alignment 캐시도 보존
    assert_eq!(daejin_db.alignment(), Some("imperial"));
}
