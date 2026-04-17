//! `EventStore::get_events_after_id` (신규 메서드) 단위 테스트
//!
//! broadcast lag 발생 시 소비자가 놓친 이벤트를 replay하기 위한 API.

use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::event::{DomainEvent, EventPayload};
use npc_mind::EventStore;

fn make_event(id: u64, aggregate: &str, seq: u64) -> DomainEvent {
    DomainEvent::new(
        id,
        aggregate.into(),
        seq,
        EventPayload::GuideGenerated {
            npc_id: "a".into(),
            partner_id: "b".into(),
        },
    )
}

#[test]
fn get_events_after_id_zero_returns_all() {
    let store = InMemoryEventStore::new();
    store.append(&[
        make_event(1, "x", 1),
        make_event(2, "x", 2),
        make_event(3, "y", 1),
    ]);

    let all = store.get_events_after_id(0);
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].id, 1);
    assert_eq!(all[2].id, 3);
}

#[test]
fn get_events_after_id_is_exclusive() {
    let store = InMemoryEventStore::new();
    store.append(&[make_event(1, "x", 1), make_event(2, "x", 2), make_event(3, "x", 3)]);

    let after_1 = store.get_events_after_id(1);
    assert_eq!(after_1.len(), 2);
    assert_eq!(after_1[0].id, 2);
    assert_eq!(after_1[1].id, 3);
}

#[test]
fn get_events_after_id_beyond_latest_returns_empty() {
    let store = InMemoryEventStore::new();
    store.append(&[make_event(1, "x", 1), make_event(2, "x", 2)]);

    let empty = store.get_events_after_id(100);
    assert!(empty.is_empty());
}

#[test]
fn get_events_after_id_crosses_aggregates() {
    // 여러 aggregate의 이벤트가 섞여도 global id 기준 필터
    let store = InMemoryEventStore::new();
    store.append(&[
        make_event(1, "a", 1),
        make_event(2, "b", 1),
        make_event(3, "a", 2),
        make_event(4, "c", 1),
    ]);

    let after_2 = store.get_events_after_id(2);
    assert_eq!(after_2.len(), 2);
    assert_eq!(after_2[0].id, 3);
    assert_eq!(after_2[1].id, 4);
    // aggregate 다양성 유지
    assert_eq!(after_2[0].aggregate_id, "a");
    assert_eq!(after_2[1].aggregate_id, "c");
}

#[test]
fn get_events_after_id_returns_cloned_data() {
    let store = InMemoryEventStore::new();
    store.append(&[make_event(1, "x", 1)]);

    let first = store.get_events_after_id(0);
    let second = store.get_events_after_id(0);
    // 두 번 조회해도 동일 내용을 얻을 수 있음(내부는 clone)
    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_eq!(first[0].id, second[0].id);
}
