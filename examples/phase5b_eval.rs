//! Phase 5b 체크포인트 1 정성 평가 — 5 Era + Phase 5a/4 era_id 외래키 활성 검증.
//!
//! 실행:
//! ```
//! cargo run --features embed --example phase5b_eval
//! ```
//!
//! 출력:
//! - count_eras
//! - list_eras() — 5 시대 일람 (id ASC, boundary 정합 검증)
//! - list_eras(contains_year=-30) — boundary 케이스 (era-fall-of-empire 단독)
//! - list_eras(contains_year=-31) — era-decline 단독
//! - list_eras(contains_year=0) — 현재(270년차)는 어느 era에도 속하지 않음
//! - list_eras(contains_year=-270) — era-founding 단독 (start inclusive)
//! - search_eras 4쿼리 (붕괴·분열·건국·270)
//! - 6 Event era_id 매핑 결과
//! - atlas-jungwon era_id 매핑 결과

use npc_mind::adapter::sqlite_world::SqliteWorldStore;
use npc_mind::domain::world::{AtlasId, EraFilter, EraId, EventFilter};
use npc_mind::worldbuilding::WorldRepository;

fn era_one_line(e: &npc_mind::domain::world::Era) -> String {
    let start = e
        .temporal
        .start_year_relative
        .map(|n| format!("{n:+}"))
        .unwrap_or_else(|| "n/a".into());
    let end = e
        .temporal
        .end_year_relative
        .map(|n| format!("{n:+}"))
        .unwrap_or_else(|| "n/a".into());
    let dur = e
        .duration_years()
        .map(|n| format!("{n}y"))
        .unwrap_or_else(|| "n/a".into());
    format!(
        "  {:<25} {:<11} [{:>5}, {:>5}) ({:>3}) key_events={}",
        e.id.as_str(),
        e.kind,
        start,
        end,
        dur,
        e.key_events.len()
    )
}

fn print_section(title: &str) {
    println!("\n=== {title} ===");
}

fn print_eras(label: &str, eras: &[npc_mind::domain::world::Era]) {
    print_section(label);
    for e in eras {
        println!("{}", era_one_line(e));
    }
    println!("  ({} 건)", eras.len());
}

fn main() {
    let store = SqliteWorldStore::new("projects/chilguk-chunchu/build/world.sqlite")
        .expect("world.sqlite 부착 — `world-load --project chilguk-chunchu` 먼저 실행");

    print_section("count_eras(project=chilguk-chunchu)");
    println!(
        "  {} 건",
        store.count_eras(Some("chilguk-chunchu")).unwrap()
    );

    print_eras(
        "list_eras() — 전체 (id ASC)",
        &store.list_eras(EraFilter::default()).unwrap(),
    );

    // boundary 정책 §3.3 시연
    print_section("boundary 정책 §3.3 — start inclusive · end exclusive");
    for (label, year) in [
        ("year=-270 (원년 = era-founding start inclusive)", -270),
        ("year=-31 (era-decline 끝 직전)", -31),
        ("year=-30 (era-fall-of-empire start inclusive — boundary)", -30),
        ("year=-7 (event-six-states-independence)", -7),
        ("year=0 (현재 270년차 — 어느 era에도 속하지 않음)", 0),
    ] {
        let hits = store
            .list_eras(EraFilter {
                contains_year: Some(year),
                ..Default::default()
            })
            .unwrap();
        let ids: Vec<&str> = hits.iter().map(|e| e.id.as_str()).collect();
        println!("  contains_year({year}): {ids:?}  // {label}");
    }

    // search 4쿼리
    print_section("search_eras 4쿼리");
    for q in ["붕괴", "분열", "건국", "270"] {
        let hits = store.search_eras(q, 5).unwrap();
        let ids: Vec<&str> = hits.iter().map(|e| e.id.as_str()).collect();
        println!("  search_eras(\"{q}\"): {ids:?}");
    }

    // 6 Event era_id 매핑
    print_section("6 Event era_id 매핑 (Phase 5a 텍스트 → Phase 5b 활성)");
    let events = store.list_events(EventFilter::default()).unwrap();
    for ev in &events {
        let yr = ev
            .temporal
            .year_relative
            .map(|n| format!("{n:+}"))
            .unwrap_or_else(|| "n/a".into());
        println!(
            "  {:<35} year_rel={:>5}  era_id={:?}",
            ev.id.as_str(),
            yr,
            ev.era_id.as_deref().unwrap_or("(none)")
        );
    }

    // atlas-jungwon era_id 매핑
    print_section("atlas-jungwon era_id 매핑 (Phase 4 텍스트 → Phase 5b 활성)");
    let atlas = store
        .get_atlas(&AtlasId::new("atlas-jungwon"))
        .unwrap()
        .expect("atlas-jungwon 미존재");
    println!(
        "  {} (kind={}) → extras.era_id = {:?}",
        atlas.id.as_str(),
        atlas.kind,
        atlas.era_id().unwrap_or("(none)")
    );

    // era-fall-of-empire의 key_events 확인
    print_section("era-fall-of-empire.key_events (시간순 5건 권장)");
    let era = store
        .get_era(&EraId::new("era-fall-of-empire"))
        .unwrap()
        .expect("era-fall-of-empire 미존재");
    for (idx, ke) in era.key_events.iter().enumerate() {
        println!("  {}. {}", idx + 1, ke.as_str());
    }
}
