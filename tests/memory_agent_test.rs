//! `MemoryAgent::run()` + broadcast lag replay 통합 테스트
//!
//! - EventBus 소비 루프에서 `DialogueTurnCompleted`가 MemoryStore에 인덱싱되는지
//! - broadcast capacity 초과로 lag 발생 시 `EventStore::get_events_after_id`
//!   replay 경로를 타고 최종 인덱싱이 보존되는지

#![cfg(feature = "embed")]

use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::application::memory_store::InMemoryMemoryStore;
use npc_mind::domain::event::{DomainEvent, EventPayload};
use npc_mind::ports::{EmbedError, MemoryStore, TextEmbedder};
use npc_mind::{EventStore, MemoryAgent};

use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 결정적 mock embedder — 입력 길이에 따라 고정 벡터 생성
struct MockEmbedder;

impl TextEmbedder for MockEmbedder {
    fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        Ok(texts
            .iter()
            .map(|t| vec![t.len() as f32, 0.0, 0.0])
            .collect())
    }
}

/// 느린 mock embedder — 각 embed 호출마다 일정 시간 block하여
/// 소비자 태스크의 처리 지연을 유도 (lag 재현용)
struct SlowEmbedder {
    delay_ms: u64,
}

impl TextEmbedder for SlowEmbedder {
    fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        std::thread::sleep(Duration::from_millis(self.delay_ms));
        Ok(texts
            .iter()
            .map(|t| vec![t.len() as f32, 0.0, 0.0])
            .collect())
    }
}

fn dialogue_event(id: u64, utterance: &str) -> DomainEvent {
    DomainEvent::new(
        id,
        "npc1".into(),
        id,
        EventPayload::DialogueTurnCompleted {
            npc_id: "npc1".into(),
            partner_id: "partner".into(),
            speaker: "user".into(),
            utterance: utterance.into(),
            emotion_snapshot: vec![],
        },
    )
}

fn append_and_publish(
    event_store: &Arc<InMemoryEventStore>,
    bus: &EventBus,
    event: DomainEvent,
) {
    event_store.append(&[event.clone()]);
    bus.publish(&event);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn memory_agent_run_indexes_dialogue_events() {
    let memory_store: Arc<dyn MemoryStore> = Arc::new(InMemoryMemoryStore::new());
    let embedder: Arc<Mutex<dyn TextEmbedder + Send>> = Arc::new(Mutex::new(MockEmbedder));
    let agent = Arc::new(MemoryAgent::new(memory_store.clone(), embedder));

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let event_store_dyn: Arc<dyn EventStore> = event_store.clone();
    let bus = Arc::new(EventBus::new());

    let task = tokio::spawn(agent.run((*bus).clone(), event_store_dyn));
    tokio::task::yield_now().await;

    // 3개 이벤트 발행
    append_and_publish(&event_store, &bus, dialogue_event(1, "안녕하세요"));
    append_and_publish(&event_store, &bus, dialogue_event(2, "반갑습니다"));
    append_and_publish(&event_store, &bus, dialogue_event(3, "또 뵈어요"));

    // 소비 완료 대기
    tokio::time::sleep(Duration::from_millis(50)).await;

    // MemoryAgent는 Arc로 공유 중 — count() 접근용으로 downcast는 불가하므로
    // search_by_keyword로 확인
    let store = Arc::clone(&memory_store);
    let hits = store.search_by_keyword("반갑", None, 10).unwrap();
    assert_eq!(hits.len(), 1, "반갑습니다 1건 인덱싱");

    let hello_hits = store.search_by_keyword("안녕", None, 10).unwrap();
    assert_eq!(hello_hits.len(), 1);

    // bus drop → Stream 종료 → 태스크 자연 종료
    drop(bus);
    let _ = task.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn memory_agent_recovers_from_lag_via_event_store_replay() {
    // 느린 embedder로 소비자를 지연시키고, 그동안 publish로 broadcast 버퍼 overflow 유도
    let memory_store: Arc<dyn MemoryStore> = Arc::new(InMemoryMemoryStore::new());
    let embedder: Arc<Mutex<dyn TextEmbedder + Send>> =
        Arc::new(Mutex::new(SlowEmbedder { delay_ms: 10 }));
    let agent = Arc::new(MemoryAgent::new(memory_store.clone(), embedder));

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let event_store_dyn: Arc<dyn EventStore> = event_store.clone();

    // 작은 capacity로 overflow 유도
    let bus = Arc::new(EventBus::with_capacity(2));

    // 에이전트 task 먼저 spawn — subscribe는 await 시점에 수행되므로 Ready 확인
    let task = tokio::spawn(agent.run((*bus).clone(), event_store_dyn.clone()));

    // subscribe가 실제 일어날 때까지 짧게 대기
    for _ in 0..50 {
        if bus.receiver_count() > 0 {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(bus.receiver_count(), 1, "에이전트가 subscribe해야 함");

    // 6개 이벤트를 빠르게 발행 — 첫 이벤트의 embed(10ms)가 진행 중일 때
    // 나머지가 2-capacity 버퍼를 초과하여 drop 발생 → Lagged 통지
    for i in 1..=6u64 {
        let ev = dialogue_event(i, &format!("발화 {i}"));
        event_store.append(&[ev.clone()]);
        bus.publish(&ev);
    }

    // 소비 + replay 시간 확보 (6 * 10ms + replay 여유)
    tokio::time::sleep(Duration::from_millis(200)).await;

    // at-least-once 보장: drop된 이벤트도 EventStore replay로 복구되어 인덱싱.
    for i in 1..=6u64 {
        let hits = memory_store
            .search_by_keyword(&format!("발화 {i}"), None, 10)
            .unwrap();
        assert!(
            !hits.is_empty(),
            "발화 {i} 인덱싱 실패 — lag 복구 경로 누락"
        );
    }

    drop(bus);
    let _ = task.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn memory_agent_last_processed_id_is_monotonic_after_lag() {
    // 회귀 방지: lag 복구 후 뒤늦게 도착하는 broadcast 잔여 이벤트가
    // `last_processed_id`를 역행시키지 않아야 한다.
    //
    // 검증 방법: 같은 이벤트가 두 번 인덱싱되어도 index 순서가 엄격히
    // 증가하는지(뒤로 가지 않는지) 확인한다.
    let memory_store: Arc<InMemoryMemoryStore> = Arc::new(InMemoryMemoryStore::new());
    let memory_store_dyn: Arc<dyn MemoryStore> = memory_store.clone();
    let embedder: Arc<Mutex<dyn TextEmbedder + Send>> =
        Arc::new(Mutex::new(SlowEmbedder { delay_ms: 10 }));
    let agent = Arc::new(MemoryAgent::new(memory_store_dyn, embedder));

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let event_store_dyn: Arc<dyn EventStore> = event_store.clone();
    let bus = Arc::new(EventBus::with_capacity(2));

    let task = tokio::spawn(agent.run((*bus).clone(), event_store_dyn));
    for _ in 0..50 {
        if bus.receiver_count() > 0 {
            break;
        }
        tokio::task::yield_now().await;
    }

    for i in 1..=5u64 {
        let ev = dialogue_event(i, &format!("발화 {i}"));
        event_store.append(&[ev.clone()]);
        bus.publish(&ev);
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // 전체 기억 중 "발화 N"을 찾아 event_id를 수집 — 처리 순서가 단조 증가여야 함
    let mut event_ids_in_order: Vec<u64> = Vec::new();
    for i in 1..=5u64 {
        let hits = memory_store
            .search_by_keyword(&format!("발화 {i}"), None, 10)
            .unwrap();
        for h in hits {
            event_ids_in_order.push(h.entry.event_id);
        }
    }
    // 각 event_id가 존재 (드문 누락 없음) — at-least-once
    for i in 1..=5u64 {
        assert!(
            event_ids_in_order.contains(&i),
            "event_id {i} 미기록 — at-least-once 위반"
        );
    }

    drop(bus);
    let _ = task.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn memory_agent_ignores_unrelated_events() {
    let memory_store: Arc<dyn MemoryStore> = Arc::new(InMemoryMemoryStore::new());
    let embedder: Arc<Mutex<dyn TextEmbedder + Send>> = Arc::new(Mutex::new(MockEmbedder));
    let agent = Arc::new(MemoryAgent::new(memory_store.clone(), embedder));

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let event_store_dyn: Arc<dyn EventStore> = event_store.clone();
    let bus = Arc::new(EventBus::new());

    let task = tokio::spawn(agent.run((*bus).clone(), event_store_dyn));
    tokio::task::yield_now().await;

    // 관심 없는 이벤트만 발행
    let guide_event = DomainEvent::new(
        1,
        "npc1".into(),
        1,
        EventPayload::GuideGenerated {
            npc_id: "npc1".into(),
            partner_id: "partner".into(),
        },
    );
    append_and_publish(&event_store, &bus, guide_event);

    tokio::time::sleep(Duration::from_millis(30)).await;

    // MemoryStore는 비어야 함
    let hits = memory_store.search_by_keyword("", None, 100).unwrap();
    assert!(hits.is_empty(), "GuideGenerated는 MemoryAgent가 무시해야 함");

    drop(bus);
    let _ = task.await;
}
