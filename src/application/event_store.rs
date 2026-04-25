//! Event Store — 도메인 이벤트 영속화 (append-only)

use crate::domain::event::{DomainEvent, EventId};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

/// 이벤트 저장소 포트
///
/// `&self`로 append를 받아 `Arc<dyn EventStore>` 공유가 가능합니다.
/// 내부 가변성(interior mutability)으로 동시성을 처리합니다.
pub trait EventStore: Send + Sync {
    /// 이벤트 추가 (append-only)
    fn append(&self, events: &[DomainEvent]);

    /// 특정 aggregate의 이벤트 스트림 조회
    fn get_events(&self, aggregate_id: &str) -> Vec<DomainEvent>;

    /// 전체 이벤트 조회
    fn get_all_events(&self) -> Vec<DomainEvent>;

    /// 주어진 event id 이후(exclusive)의 이벤트 조회 — broadcast lag 복구용
    fn get_events_after_id(&self, after_id: EventId) -> Vec<DomainEvent>;

    /// 같은 correlation_id로 발생한 이벤트 묶음 조회.
    ///
    /// 한 `dispatch_v2` 호출이 만든 모든 이벤트의 인과 사슬을 반환한다.
    /// 결과는 EventStore에 추가된 순서를 그대로 보존한다 (정렬은 호출자 책임).
    /// `correlation_id == 0`은 "미설정" sentinel이라 매치되는 이벤트가 없다.
    fn get_events_by_correlation(&self, correlation_id: u64) -> Vec<DomainEvent>;

    /// 다음 이벤트 ID 발급
    fn next_id(&self) -> EventId;

    /// 특정 aggregate의 다음 시퀀스 번호
    fn next_sequence(&self, aggregate_id: &str) -> u64;
}

/// 인메모리 이벤트 저장소 — 개발/테스트용
pub struct InMemoryEventStore {
    events: RwLock<Vec<DomainEvent>>,
    next_id: AtomicU64,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(Vec::new()),
            next_id: AtomicU64::new(1),
        }
    }
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EventStore for InMemoryEventStore {
    fn append(&self, events: &[DomainEvent]) {
        let mut store = self.events.write().unwrap();
        store.extend(events.iter().cloned());
    }

    fn get_events(&self, aggregate_id: &str) -> Vec<DomainEvent> {
        let store = self.events.read().unwrap();
        store
            .iter()
            .filter(|e| e.aggregate_id == aggregate_id)
            .cloned()
            .collect()
    }

    fn get_all_events(&self) -> Vec<DomainEvent> {
        let store = self.events.read().unwrap();
        store.clone()
    }

    fn get_events_after_id(&self, after_id: EventId) -> Vec<DomainEvent> {
        let store = self.events.read().unwrap();
        store.iter().filter(|e| e.id > after_id).cloned().collect()
    }

    fn get_events_by_correlation(&self, correlation_id: u64) -> Vec<DomainEvent> {
        let store = self.events.read().unwrap();
        store
            .iter()
            .filter(|e| e.metadata.correlation_id == Some(correlation_id))
            .cloned()
            .collect()
    }

    fn next_id(&self) -> EventId {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    fn next_sequence(&self, aggregate_id: &str) -> u64 {
        let store = self.events.read().unwrap();
        let count = store
            .iter()
            .filter(|e| e.aggregate_id == aggregate_id)
            .count();
        count as u64 + 1
    }
}
