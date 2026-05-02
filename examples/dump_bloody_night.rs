//! Phase 5a 체크포인트 1 진단용 — event-bloody-night JSON dump.
//!
//! 실행:
//! ```
//! cargo run --features embed --example dump_bloody_night
//! ```

use npc_mind::adapter::sqlite_world::SqliteWorldStore;
use npc_mind::domain::world::{EventFilter, EventId};
use npc_mind::worldbuilding::WorldRepository;

fn main() {
    let store = SqliteWorldStore::new("projects/chilguk-chunchu/build/world.sqlite")
        .expect("world.sqlite 부착");
    let e = store
        .get_event(&EventId::new("event-bloody-night"))
        .unwrap()
        .expect("event-bloody-night 미존재");
    println!("=== get_event(event-bloody-night) ===");
    println!("{}", serde_json::to_string_pretty(&e).unwrap());

    println!(
        "\n=== count_events(project=chilguk-chunchu) = {} ===",
        store.count_events(Some("chilguk-chunchu")).unwrap()
    );

    println!("\n=== list_events(participants_person=npc-02) ===");
    let f = EventFilter {
        participants_person: Some("npc-02".into()),
        ..Default::default()
    };
    for ev in store.list_events(f).unwrap() {
        println!(
            "- {} ({}, year_relative={:?})",
            ev.id,
            ev.name,
            ev.year_relative()
        );
    }
}
