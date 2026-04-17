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

    let task = tokio::spawn(agent.run(&bus, event_store_dyn));
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
    let memory_store: Arc<dyn MemoryStore> = Arc::new(InMemoryMemoryStore::new());
    let embedder: Arc<Mutex<dyn TextEmbedder + Send>> = Arc::new(Mutex::new(MockEmbedder));
    let agent = Arc::new(MemoryAgent::new(memory_store.clone(), embedder));

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let event_store_dyn: Arc<dyn EventStore> = event_store.clone();

    // 작은 capacity로 overflow 유도
    let bus = Arc::new(EventBus::with_capacity(2));

    // 구독을 먼저 확보(run() 호출 시 내부에서 subscribe)하되 polling은 지연
    let future = agent.run(&bus, event_store_dyn.clone());
    assert_eq!(bus.receiver_count(), 1, "run() 호출로 receiver 등록됨");

    // 구독자가 대기 중인 상태에서 capacity 초과 publish → 앞 이벤트 drop
    for i in 1..=6u64 {
        let ev = dialogue_event(i, &format!("발화 {i}"));
        event_store.append(&[ev.clone()]);
        bus.publish(&ev);
    }

    // 이제 소비 태스크 시작 — 첫 recv에서 Lagged 통지 → EventStore replay
    let task = tokio::spawn(future);

    // 처리 시간 확보
    tokio::time::sleep(Duration::from_millis(100)).await;

    // at-least-once 보장: drop된 이벤트도 EventStore replay로 복구되어 인덱싱.
    // 중복 인덱싱(broadcast 수신 + replay)은 허용.
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
async fn memory_agent_ignores_unrelated_events() {
    let memory_store: Arc<dyn MemoryStore> = Arc::new(InMemoryMemoryStore::new());
    let embedder: Arc<Mutex<dyn TextEmbedder + Send>> = Arc::new(Mutex::new(MockEmbedder));
    let agent = Arc::new(MemoryAgent::new(memory_store.clone(), embedder));

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let event_store_dyn: Arc<dyn EventStore> = event_store.clone();
    let bus = Arc::new(EventBus::new());

    let task = tokio::spawn(agent.run(&bus, event_store_dyn));
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
