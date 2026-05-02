//! Phase 5a 체크포인트 2 정성 평가 — 6 Event 검증.
//!
//! 실행:
//! ```
//! cargo run --features embed --example phase5a_eval
//! ```
//!
//! 출력:
//! - count_events
//! - list_events() — 6 사건 일람 (id ASC)
//! - list_events(category=historical) — 6건
//! - list_events(participants_person=npc-02) — 조고 관여 사건
//! - list_events(participants_place=place-daejin) — 대진 관여 사건
//! - list_events(year_relative_min=-30, year_relative_max=0) — 30년 전 ~ 현재
//! - search_events 6쿼리 (혈교·붉은 밤·임서운·화산·건국·독립)
//! - related_events 양방향 시연 (bloody-night ↔ hwasan-fall)
//! - get_event(event-bloody-night) detail

use npc_mind::adapter::sqlite_world::SqliteWorldStore;
use npc_mind::domain::world::{EventCategory, EventFilter, EventId};
use npc_mind::worldbuilding::WorldRepository;

fn one_line_summary(e: &npc_mind::domain::world::Event) -> String {
    let yr = e
        .temporal
        .year_relative
        .map(|n| format!("{n:+}"))
        .unwrap_or_else(|| "n/a".into());
    let related = if e.related_events.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "[{}]",
            e.related_events
                .iter()
                .map(|r| r.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    format!(
        "  {:<35} {:<12} {:<10} year_rel={:>5}  related={}",
        e.id.as_str(),
        e.kind,
        e.category.as_str(),
        yr,
        related
    )
}

fn print_section(title: &str) {
    println!("\n=== {title} ===");
}

fn print_events(label: &str, events: &[npc_mind::domain::world::Event]) {
    print_section(label);
    for e in events {
        println!("{}", one_line_summary(e));
    }
    println!("  ({} 건)", events.len());
}

fn main() {
    let store = SqliteWorldStore::new("projects/chilguk-chunchu/build/world.sqlite")
        .expect("world.sqlite 부착 — `world-load --project chilguk-chunchu` 먼저 실행");

    print_section("count_events(project=chilguk-chunchu)");
    println!(
        "  {} 건",
        store.count_events(Some("chilguk-chunchu")).unwrap()
    );

    print_events(
        "list_events() — 전체 (id ASC)",
        &store.list_events(EventFilter::default()).unwrap(),
    );

    print_events(
        "list_events(category=historical)",
        &store
            .list_events(EventFilter {
                category: Some(EventCategory::Historical),
                ..Default::default()
            })
            .unwrap(),
    );

    print_events(
        "list_events(participants_person=npc-02 — 조고 관여 사건)",
        &store
            .list_events(EventFilter {
                participants_person: Some("npc-02".into()),
                ..Default::default()
            })
            .unwrap(),
    );

    print_events(
        "list_events(participants_person=npc-01 — 명경 관여 사건)",
        &store
            .list_events(EventFilter {
                participants_person: Some("npc-01".into()),
                ..Default::default()
            })
            .unwrap(),
    );

    print_events(
        "list_events(participants_place=place-daejin — 대진 관여 사건)",
        &store
            .list_events(EventFilter {
                participants_place: Some("place-daejin".into()),
                ..Default::default()
            })
            .unwrap(),
    );

    print_events(
        "list_events(year_relative_min=-30, year_relative_max=0 — 30년 전 ~ 현재)",
        &store
            .list_events(EventFilter {
                year_relative_min: Some(-30),
                year_relative_max: Some(0),
                ..Default::default()
            })
            .unwrap(),
    );

    print_events(
        "list_events(year_relative_min=-300, year_relative_max=-100 — 변곡기 이전 100년)",
        &store
            .list_events(EventFilter {
                year_relative_min: Some(-300),
                year_relative_max: Some(-100),
                ..Default::default()
            })
            .unwrap(),
    );

    // search_events 6쿼리
    for q in ["혈교", "붉은 밤", "임서운", "화산", "건국", "독립"] {
        print_section(&format!("search_events(\"{q}\", top_k=5)"));
        for e in store.search_events(q, 5).unwrap() {
            println!("{}", one_line_summary(&e));
        }
    }

    // related_events 양방향 시연
    print_section("related_events 양방향 시연 (bloody-night ↔ hwasan-fall)");
    let bn = store
        .get_event(&EventId::new("event-bloody-night"))
        .unwrap()
        .unwrap();
    let hf = store
        .get_event(&EventId::new("event-hwasan-fall"))
        .unwrap()
        .unwrap();
    println!(
        "  bloody-night.related_events: {:?}",
        bn.related_events
            .iter()
            .map(|r| r.as_str())
            .collect::<Vec<_>>()
    );
    println!(
        "  hwasan-fall.related_events: {:?}",
        hf.related_events
            .iter()
            .map(|r| r.as_str())
            .collect::<Vec<_>>()
    );
    let bn_has_hf = bn
        .related_events
        .iter()
        .any(|r| r.as_str() == "event-hwasan-fall");
    let hf_has_bn = hf
        .related_events
        .iter()
        .any(|r| r.as_str() == "event-bloody-night");
    println!(
        "  ✓ bloody-night ⊃ hwasan-fall: {bn_has_hf}\n  ✓ hwasan-fall ⊃ bloody-night: {hf_has_bn}"
    );

    // 인과 사슬
    print_section("인과 사슬 (related_events 연쇄 — empire-founding → ... → six-states-independence)");
    for id in [
        "event-empire-founding",
        "event-bloody-cult-rebellion-2nd",
        "event-blood-disappearance",
        "event-bloody-night",
        "event-hwasan-fall",
        "event-six-states-independence",
    ] {
        let e = store.get_event(&EventId::new(id)).unwrap().unwrap();
        let related: Vec<&str> = e.related_events.iter().map(|r| r.as_str()).collect();
        println!(
            "  {:<35} year_rel={:>5} → related: {:?}",
            id,
            e.temporal
                .year_relative
                .map(|n| format!("{n:+}"))
                .unwrap_or_else(|| "n/a".into()),
            related
        );
    }
}
