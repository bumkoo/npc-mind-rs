//! Phase 5b 체크포인트 2 정성 평가 — Timeline view 메서드 4종 + atlas overlay 시연.
//!
//! 실행:
//! ```
//! cargo run --features embed --example phase5b_checkpoint2_eval
//! ```
//!
//! 출력:
//! - count_timelines + list_timelines() — 1건 (timeline-jungwon-history)
//! - view 메서드 4종 e2e:
//!   · eras_in(repo) → 5 era (작성 순서)
//!   · events_in(repo) → 6 사건 (era-founding 1 + era-fall-of-empire 5)
//!   · events_during(era-fall-of-empire, repo) → 5 사건
//!   · causal_chain(event-bloody-night, repo) → BFS 결과
//! - search_timelines 3쿼리 — "270년사"·"칠국 역사"·"main-history"
//! - Atlas overlay 양방향 — atlas-jungwon.era_id ↔ era-fall-of-empire

use npc_mind::adapter::sqlite_world::SqliteWorldStore;
use npc_mind::domain::world::{AtlasId, EraId, EventId, TimelineId};
use npc_mind::worldbuilding::WorldRepository;

fn print_section(title: &str) {
    println!("\n=== {title} ===");
}

fn main() {
    let store = SqliteWorldStore::new("projects/chilguk-chunchu/build/world.sqlite")
        .expect("world.sqlite 부착 — `world-load --project chilguk-chunchu` 먼저 실행");

    print_section("count_timelines(project=chilguk-chunchu)");
    println!(
        "  {} 건",
        store.count_timelines(Some("chilguk-chunchu")).unwrap()
    );

    let timeline = store
        .get_timeline(&TimelineId::new("timeline-jungwon-history"))
        .unwrap()
        .expect("timeline-jungwon-history 미존재");

    print_section("get_timeline(timeline-jungwon-history)");
    println!(
        "  id={} kind={} name={} references={:?}",
        timeline.id.as_str(),
        timeline.kind,
        timeline.name,
        timeline
            .references
            .iter()
            .map(|e| e.as_str())
            .collect::<Vec<_>>()
    );

    // -----------------------------------------------------------------------
    // view 메서드 4종 e2e
    // -----------------------------------------------------------------------

    print_section("view 메서드 1/4 — eras_in(repo) → 5 era (작성 순서)");
    let eras = timeline.eras_in(&store).unwrap();
    for e in &eras {
        let start = e
            .temporal
            .start_year_relative
            .map(|n| format!("{n:+}"))
            .unwrap_or_else(|| "?".into());
        let end = e
            .temporal
            .end_year_relative
            .map(|n| format!("{n:+}"))
            .unwrap_or_else(|| "?".into());
        println!(
            "  {:<25} kind={:<11} [{:>5}, {:>5})  key_events={}",
            e.id.as_str(),
            e.kind,
            start,
            end,
            e.key_events.len()
        );
    }
    assert_eq!(eras.len(), 5, "eras_in 결과 5건");

    print_section("view 메서드 2/4 — events_in(repo) → 6 사건 (key_events 평면화)");
    let events = timeline.events_in(&store).unwrap();
    for ev in &events {
        let yr = ev
            .temporal
            .year_relative
            .map(|n| format!("{n:+}"))
            .unwrap_or_else(|| "?".into());
        println!(
            "  {:<35} year_rel={:>5}  era={}",
            ev.id.as_str(),
            yr,
            ev.era_id.as_deref().unwrap_or("(none)")
        );
    }
    assert_eq!(events.len(), 6, "events_in 결과 6건 (1 founding + 5 fall-of-empire)");

    print_section("view 메서드 3/4 — events_during(era-fall-of-empire, repo) → 5 사건");
    let fall_events = timeline
        .events_during(&EraId::new("era-fall-of-empire"), &store)
        .unwrap();
    for ev in &fall_events {
        let yr = ev
            .temporal
            .year_relative
            .map(|n| format!("{n:+}"))
            .unwrap_or_else(|| "?".into());
        println!("  {:<35} year_rel={:>5}", ev.id.as_str(), yr);
    }
    assert_eq!(fall_events.len(), 5, "events_during(era-fall-of-empire) 결과 5건");

    print_section("view 메서드 4/4 — causal_chain(event-bloody-night, repo) → BFS 결과");
    let chain = timeline
        .causal_chain(&EventId::new("event-bloody-night"), &store)
        .unwrap();
    for (idx, ev) in chain.iter().enumerate() {
        println!(
            "  {}. {:<35} (related: {:?})",
            idx + 1,
            ev.id.as_str(),
            ev.related_events
                .iter()
                .map(|r| r.as_str())
                .collect::<Vec<_>>()
        );
    }
    println!("  (총 {}건 — BFS, timeline 경계 안)", chain.len());
    assert!(chain.len() >= 4, "causal_chain은 최소 seed + bloody-night 직접 related 3건 = 4건 이상");

    // -----------------------------------------------------------------------
    // search_timelines — 3 쿼리
    // -----------------------------------------------------------------------

    print_section("search_timelines 3쿼리");
    for q in ["270년사", "칠국 역사", "main-history"] {
        let hits = store.search_timelines(q, 5).unwrap();
        let ids: Vec<&str> = hits.iter().map(|t| t.id.as_str()).collect();
        println!("  search_timelines(\"{q}\"): {ids:?}");
    }

    // -----------------------------------------------------------------------
    // Atlas overlay — atlas-jungwon.era_id ↔ era-fall-of-empire 양방향 시연
    // -----------------------------------------------------------------------

    print_section("Atlas overlay — atlas-jungwon.era_id ↔ era-fall-of-empire 양방향");
    let atlas = store
        .get_atlas(&AtlasId::new("atlas-jungwon"))
        .unwrap()
        .expect("atlas-jungwon 미존재");
    println!(
        "  atlas-jungwon.era_id = {:?}",
        atlas.era_id().unwrap_or("(none)")
    );
    let era = store
        .get_era(&EraId::new(atlas.era_id().unwrap_or("")))
        .unwrap();
    match era {
        Some(e) => println!(
            "  ↳ resolved era: {} (kind={}, key_events={})",
            e.name,
            e.kind,
            e.key_events.len()
        ),
        None => println!("  ↳ era 결손 (이는 발생해선 안 됨 — world-load FK 활성)"),
    }

    // 역방향 — era-fall-of-empire가 atlas의 extras.era_id로 참조되는지.
    // (atlas는 era에 대한 직접 역참조 인덱스 X — list_atlases 후 era_id 비교로 확인)
    let atlases = store
        .list_atlases(npc_mind::domain::world::AtlasFilter::default())
        .unwrap();
    let fall_atlases: Vec<&str> = atlases
        .iter()
        .filter(|a| a.era_id() == Some("era-fall-of-empire"))
        .map(|a| a.id.as_str())
        .collect();
    println!(
        "  ↳ era-fall-of-empire를 era_id로 가지는 atlas 일람: {fall_atlases:?}"
    );

    print_section("✓ Phase 5b 체크포인트 2 view 메서드 4종 + atlas overlay 모두 통과");
}
