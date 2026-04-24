//! `MemoryProjector` Stream 소비 로직 테스트
//!
//! 타이밍·실제 broadcast에 의존하지 않고 `MemoryProjector::consume_stream`에
//! 확정적 Stream을 주입해 검증한다.

#![cfg(feature = "embed")]

mod common;

use common::in_memory_store::InMemoryMemoryStore;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::event::{DomainEvent, EventPayload};
use npc_mind::ports::{EmbedError, MemoryStore, TextEmbedder};
use npc_mind::{EventStore, MemoryProjector};

use futures::stream;
use std::sync::{Arc, Mutex};

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

/// 테스트용 agent + memory_store + event_store 번들 생성
fn setup() -> (
    Arc<MemoryProjector>,
    Arc<InMemoryMemoryStore>,
    Arc<InMemoryEventStore>,
) {
    let memory_store: Arc<InMemoryMemoryStore> = Arc::new(InMemoryMemoryStore::new());
    let memory_store_dyn: Arc<dyn MemoryStore> = memory_store.clone();
    let embedder: Arc<Mutex<dyn TextEmbedder + Send>> = Arc::new(Mutex::new(MockEmbedder));
    let agent = Arc::new(MemoryProjector::new(memory_store_dyn, embedder));
    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    (agent, memory_store, event_store)
}

#[tokio::test]
async fn consume_stream_indexes_ok_events() {
    let (agent, memory_store, event_store) = setup();
    let event_store_dyn: Arc<dyn EventStore> = event_store.clone();

    let items: Vec<Result<Arc<DomainEvent>, u64>> = (1..=3u64)
        .map(|i| Ok(Arc::new(dialogue_event(i, &format!("발화 {i}")))))
        .collect();
    let s = Box::pin(stream::iter(items));

    agent.consume_stream(s, event_store_dyn).await;

    for i in 1..=3u64 {
        let hits = memory_store
            .search_by_keyword(&format!("발화 {i}"), None, 10)
            .unwrap();
        assert_eq!(hits.len(), 1, "발화 {i} 인덱싱 1건");
    }
}

#[tokio::test]
async fn consume_stream_ignores_unrelated_events() {
    let (agent, memory_store, event_store) = setup();
    let event_store_dyn: Arc<dyn EventStore> = event_store.clone();

    let unrelated = DomainEvent::new(
        1,
        "npc1".into(),
        1,
        EventPayload::GuideGenerated {
            npc_id: "npc1".into(),
            partner_id: "partner".into(),
        },
    );
    let s = Box::pin(stream::iter(vec![Ok(Arc::new(unrelated))]));

    agent.consume_stream(s, event_store_dyn).await;

    assert_eq!(memory_store.count(), 0, "GuideGenerated는 무시");
}

#[tokio::test]
async fn consume_stream_replays_from_event_store_on_lag() {
    // Err(Lagged) 수신 시 EventStore.get_events_after_id(last)로 replay하여
    // drop된 이벤트도 인덱싱되는지 검증.
    let (agent, memory_store, event_store) = setup();
    let event_store_dyn: Arc<dyn EventStore> = event_store.clone();

    // EventStore에 1~5 기록 (영속화 경로 시뮬레이션)
    for i in 1..=5u64 {
        event_store.append(&[dialogue_event(i, &format!("발화 {i}"))]);
    }

    // Stream 시퀀스: 이벤트 1,2 수신 후 broadcast에서 3,4,5 drop → Lagged 통지
    let items: Vec<Result<Arc<DomainEvent>, u64>> = vec![
        Ok(Arc::new(dialogue_event(1, "발화 1"))),
        Ok(Arc::new(dialogue_event(2, "발화 2"))),
        Err(3), // Lagged(3) — 3, 4, 5 drop
    ];
    let s = Box::pin(stream::iter(items));

    agent.consume_stream(s, event_store_dyn).await;

    // at-least-once: 1~5 모두 인덱싱돼야 함
    for i in 1..=5u64 {
        let hits = memory_store
            .search_by_keyword(&format!("발화 {i}"), None, 10)
            .unwrap();
        assert!(!hits.is_empty(), "발화 {i} 유실 — replay 실패");
    }
}

#[tokio::test]
async fn consume_stream_last_processed_id_is_monotonic() {
    // replay 이후 broadcast 잔여 이벤트가 뒤늦게 도착해도 이미 처리한
    // 이벤트는 건너뛰어야 한다(중복 인덱싱 방지 + 커서 역행 방지).
    let (agent, memory_store, event_store) = setup();
    let event_store_dyn: Arc<dyn EventStore> = event_store.clone();

    // EventStore에 1~5 기록
    for i in 1..=5u64 {
        event_store.append(&[dialogue_event(i, &format!("발화 {i}"))]);
    }

    let items: Vec<Result<Arc<DomainEvent>, u64>> = vec![
        Ok(Arc::new(dialogue_event(1, "발화 1"))),   // 처리, last=1
        Err(2),                                        // Lagged → replay 2~5, last=5
        Ok(Arc::new(dialogue_event(3, "발화 3"))),   // 3 <= last(5) — 건너뜀
        Ok(Arc::new(dialogue_event(4, "발화 4"))),   // 4 <= last(5) — 건너뜀
    ];
    let s = Box::pin(stream::iter(items));

    agent.consume_stream(s, event_store_dyn).await;

    // 각 발화당 정확히 1건만 존재 — 중복 인덱싱 없음
    for i in 1..=5u64 {
        let hits = memory_store
            .search_by_keyword(&format!("발화 {i}"), None, 10)
            .unwrap();
        assert_eq!(
            hits.len(),
            1,
            "발화 {i}는 정확히 1건이어야 함, 실제 {} 건",
            hits.len()
        );
    }
}

#[tokio::test]
async fn consume_stream_handles_consecutive_lag_without_duplicates() {
    // 연속된 Err(Lagged) 통지에서도 replay로 중복되지 않는지 확인
    let (agent, memory_store, event_store) = setup();
    let event_store_dyn: Arc<dyn EventStore> = event_store.clone();

    for i in 1..=3u64 {
        event_store.append(&[dialogue_event(i, &format!("발화 {i}"))]);
    }

    let items: Vec<Result<Arc<DomainEvent>, u64>> = vec![
        Err(3), // 첫 Lagged → replay 1~3, last=3
        Err(1), // 두 번째 Lagged → get_events_after_id(3) 빈 리스트, 영향 없음
    ];
    let s = Box::pin(stream::iter(items));

    agent.consume_stream(s, event_store_dyn).await;

    for i in 1..=3u64 {
        let hits = memory_store
            .search_by_keyword(&format!("발화 {i}"), None, 10)
            .unwrap();
        assert_eq!(hits.len(), 1, "발화 {i} 중복 없이 1건");
    }
}
