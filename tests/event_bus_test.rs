//! `EventBus` v2 신규 API 단위 테스트
//!
//! - `new()` / `with_capacity(n)` 생성
//! - `publish`/`subscribe` 기본 전달
//! - `subscribe_with_lag` — capacity 초과 시 `Err(n)` 통지
//! - 복수 구독자 fan-out
//! - `receiver_count` 동적 변화

use futures::StreamExt;
use npc_mind::application::event_bus::{EventBus, DEFAULT_CAPACITY};
use npc_mind::domain::event::{DomainEvent, EventPayload};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn make_event(id: u64) -> DomainEvent {
    DomainEvent::new(
        id,
        "test".into(),
        id,
        EventPayload::GuideGenerated {
            npc_id: "a".into(),
            partner_id: "b".into(),
        },
    )
}

#[test]
fn default_capacity_constant_is_positive() {
    assert!(DEFAULT_CAPACITY > 0);
}

#[test]
fn new_and_with_capacity_create_distinct_buses() {
    let bus_default = EventBus::new();
    let bus_custom = EventBus::with_capacity(4);
    assert_eq!(bus_default.receiver_count(), 0);
    assert_eq!(bus_custom.receiver_count(), 0);
}

#[test]
fn receiver_count_tracks_active_subscribers() {
    let bus = EventBus::new();
    assert_eq!(bus.receiver_count(), 0);

    let s1 = bus.subscribe();
    assert_eq!(bus.receiver_count(), 1);

    let s2 = bus.subscribe();
    assert_eq!(bus.receiver_count(), 2);

    drop(s1);
    assert_eq!(bus.receiver_count(), 1);

    drop(s2);
    assert_eq!(bus.receiver_count(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn subscribe_delivers_events_to_single_consumer() {
    let bus = EventBus::new();
    let mut stream = Box::pin(bus.subscribe());

    bus.publish(&make_event(1));
    bus.publish(&make_event(2));

    let first = stream.next().await.expect("first event");
    assert_eq!(first.id, 1);
    let second = stream.next().await.expect("second event");
    assert_eq!(second.id, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multiple_subscribers_each_receive_all_events() {
    let bus = EventBus::new();
    let mut a = Box::pin(bus.subscribe());
    let mut b = Box::pin(bus.subscribe());

    bus.publish(&make_event(10));
    bus.publish(&make_event(11));

    let a1 = a.next().await.unwrap();
    let a2 = a.next().await.unwrap();
    let b1 = b.next().await.unwrap();
    let b2 = b.next().await.unwrap();

    assert_eq!((a1.id, a2.id), (10, 11));
    assert_eq!((b1.id, b2.id), (10, 11));
}

#[test]
fn publish_without_subscribers_does_not_panic_or_error() {
    let bus = EventBus::new();
    // broadcast send에서 Err(SendError)가 나도 내부에서 무시돼야 함
    for i in 0..10 {
        bus.publish(&make_event(i));
    }
    assert_eq!(bus.receiver_count(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn subscribe_with_lag_reports_dropped_events() {
    // capacity = 2. 구독자가 대기 중일 때 3번 이상 발행하면 lag 발생
    let bus = EventBus::with_capacity(2);
    let mut stream = Box::pin(bus.subscribe_with_lag());

    // capacity 초과로 drop 발생 유도
    for i in 1..=5 {
        bus.publish(&make_event(i));
    }

    // 첫 수신은 Err(Lagged(n)) — broadcast는 다음 recv에서 lag 통지
    let first = stream.next().await.expect("first item");
    assert!(first.is_err(), "capacity 초과 시 첫 수신은 Err");
    let dropped = first.unwrap_err();
    assert!(dropped > 0, "drop된 이벤트 수는 양수");

    // 이후 수신은 유효 구간의 이벤트들
    let mut received_ids = Vec::new();
    while let Some(Ok(ev)) = stream.next().await {
        received_ids.push(ev.id);
        if received_ids.len() >= 2 {
            break;
        }
    }
    assert_eq!(received_ids.len(), 2, "유효 구간 이벤트 2개 수신");
    // 뒤쪽 이벤트(4, 5)가 남아야 함
    assert!(received_ids.iter().all(|&id| id >= 3));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn subscribe_default_filters_out_lag_errors() {
    let bus = EventBus::with_capacity(2);
    let mut stream = Box::pin(bus.subscribe());

    for i in 1..=5 {
        bus.publish(&make_event(i));
    }

    // 일반 subscribe는 Lagged를 삼키고 유효 이벤트만 통과
    let mut ids = Vec::new();
    while let Some(ev) = stream.next().await {
        ids.push(ev.id);
        if ids.len() >= 2 {
            break;
        }
    }
    assert_eq!(ids.len(), 2);
    assert!(ids.iter().all(|&id| id > 0), "유효 이벤트만 수신");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slow_subscriber_does_not_block_fast_one() {
    // 빠른 구독자 A와 느린 B가 공존. publish는 둘 다를 막지 않고 리턴해야 함
    let bus = EventBus::with_capacity(4);
    let mut fast = Box::pin(bus.subscribe());
    let _slow = bus.subscribe(); // 소비하지 않음

    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    let consumer = tokio::spawn(async move {
        while let Some(_ev) = fast.next().await {
            c.fetch_add(1, Ordering::SeqCst);
        }
    });

    for i in 1..=3 {
        bus.publish(&make_event(i));
    }

    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    drop(bus);
    let _ = consumer.await;

    assert_eq!(count.load(Ordering::SeqCst), 3);
}

#[test]
#[cfg(not(debug_assertions))]
fn with_capacity_zero_is_clamped_to_one_in_release() {
    // debug 빌드에서는 debug_assert가 panic하지만 release에서는 clamp로 살아남아야 한다
    let bus = EventBus::with_capacity(0);
    let _rx = bus.subscribe();
    bus.publish(&make_event(1));
    // panic 없이 실행되면 통과
}

#[test]
#[should_panic(expected = "EventBus capacity must be greater than 0")]
#[cfg(debug_assertions)]
fn with_capacity_zero_panics_in_debug() {
    let _ = EventBus::with_capacity(0);
}

#[test]
fn bus_is_clone_and_shares_sender() {
    let bus_a = EventBus::new();
    let bus_b = bus_a.clone();

    let _rx = bus_a.subscribe();
    // Clone된 bus의 receiver_count에도 반영됨(같은 sender)
    assert_eq!(bus_b.receiver_count(), 1);
}
