// wuxia-core/src/experience/bus.rs
//
// EventBus — 경험 이벤트 큐 포트 & 인메모리 구현.
//
// 이벤트 큐에는 ExperienceEvent만 들어간다 (원칙 8).
// 핸들러 간 연쇄 반응은 ProcessingContext(DomainEvent)로 처리.
//
// Port & Adapter:
//   wuxia-core:  EventBus trait + InMemoryEventBus (VecDeque, 테스트/MVP)
//   wuxia-game:  BevyEventBridge (Bevy EventWriter/EventReader, Phase 5)

use std::collections::VecDeque;

use super::event::ExperienceEvent;

// ---------------------------------------------------------------------------
// EventBus — 포트 trait
// ---------------------------------------------------------------------------

/// 경험 이벤트 큐 포트 (헥사고날 아키텍처).
///
/// ExperienceEvent를 넣고 꺼내는 FIFO 큐 인터페이스.
/// DomainEvent는 이 큐에 들어가지 않는다 — ProcessingContext에서만 사용.
///
/// # 구현체
/// - `InMemoryEventBus` (wuxia-core): VecDeque 기반, 테스트/MVP용.
/// - `BevyEventBridge` (wuxia-game): Bevy EventWriter/EventReader, Phase 5.
///
/// # Example
/// ```
/// use wuxia_core::experience::{EventBus, InMemoryEventBus, ExperienceEvent, ExperienceHeader};
/// use wuxia_core::shared::id::{ExperienceId, CharacterId, LocationId};
/// use wuxia_core::shared::GameTime;
///
/// let mut bus = InMemoryEventBus::new();
/// assert!(bus.is_empty());
///
/// let event = ExperienceEvent::Rest {
///     header: ExperienceHeader::new(
///         ExperienceId::new(1),
///         CharacterId::new(1),
///         GameTime::new(1200, 1, 1),
///         LocationId::new(1),
///         3.0,
///     ),
///     method: "수면".to_string(),
///     recovery: 0.5,
/// };
/// bus.push(event);
/// assert_eq!(bus.len(), 1);
///
/// let polled = bus.poll();
/// assert!(polled.is_some());
/// assert!(bus.is_empty());
/// ```
pub trait EventBus: Send + Sync {
    /// 이벤트를 큐에 넣는다.
    fn push(&mut self, event: ExperienceEvent);

    /// 큐에서 이벤트를 하나 꺼낸다 (FIFO). 비면 None.
    fn poll(&mut self) -> Option<ExperienceEvent>;

    /// 큐가 비어있는지 확인.
    fn is_empty(&self) -> bool;

    /// 큐에 남은 이벤트 수.
    fn len(&self) -> usize;
}

// ---------------------------------------------------------------------------
// InMemoryEventBus — 인메모리 구현 (VecDeque)
// ---------------------------------------------------------------------------

/// 인메모리 이벤트 버스 (VecDeque 기반).
///
/// 테스트와 MVP에서 사용. 단순 FIFO 큐.
/// Phase 5에서 BevyEventBridge로 교체 가능 (포트 추상화).
#[derive(Debug, Default)]
pub struct InMemoryEventBus {
    queue: VecDeque<ExperienceEvent>,
}

impl InMemoryEventBus {
    /// 빈 이벤트 버스 생성.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// 지정된 용량으로 이벤트 버스 생성.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(capacity),
        }
    }
}

impl EventBus for InMemoryEventBus {
    fn push(&mut self, event: ExperienceEvent) {
        self.queue.push_back(event);
    }

    fn poll(&mut self) -> Option<ExperienceEvent> {
        self.queue.pop_front()
    }

    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    fn len(&self) -> usize {
        self.queue.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experience::event::ExperienceHeader;
    use crate::shared::id::{CharacterId, ExperienceId, LocationId, MartialArtId};
    use crate::shared::time::GameTime;

    fn make_event(id: u64, name_suffix: &str) -> ExperienceEvent {
        ExperienceEvent::Observation {
            header: ExperienceHeader::new(
                ExperienceId::new(id),
                CharacterId::new(1),
                GameTime::new(1200, 3, 15),
                LocationId::new(10),
                5.0,
            ),
            target: None,
            what: format!("관찰 {}", name_suffix),
            sentiment_delta: None,
        }
    }

    // -- 기본 동작 --

    #[test]
    fn new_bus_is_empty() {
        let bus = InMemoryEventBus::new();
        assert!(bus.is_empty());
        assert_eq!(bus.len(), 0);
    }

    #[test]
    fn push_increases_len() {
        let mut bus = InMemoryEventBus::new();
        bus.push(make_event(1, "A"));
        assert_eq!(bus.len(), 1);
        assert!(!bus.is_empty());

        bus.push(make_event(2, "B"));
        assert_eq!(bus.len(), 2);
    }

    #[test]
    fn poll_returns_fifo_order() {
        let mut bus = InMemoryEventBus::new();
        bus.push(make_event(1, "first"));
        bus.push(make_event(2, "second"));
        bus.push(make_event(3, "third"));

        // FIFO — 먼저 넣은 것이 먼저 나온다
        let first = bus.poll().unwrap();
        assert_eq!(first.header().experience_id, ExperienceId::new(1));

        let second = bus.poll().unwrap();
        assert_eq!(second.header().experience_id, ExperienceId::new(2));

        let third = bus.poll().unwrap();
        assert_eq!(third.header().experience_id, ExperienceId::new(3));
    }

    #[test]
    fn poll_empty_returns_none() {
        let mut bus = InMemoryEventBus::new();
        assert!(bus.poll().is_none());
    }

    #[test]
    fn poll_decreases_len() {
        let mut bus = InMemoryEventBus::new();
        bus.push(make_event(1, "A"));
        bus.push(make_event(2, "B"));

        bus.poll();
        assert_eq!(bus.len(), 1);

        bus.poll();
        assert_eq!(bus.len(), 0);
        assert!(bus.is_empty());
    }

    #[test]
    fn with_capacity() {
        let bus = InMemoryEventBus::with_capacity(100);
        assert!(bus.is_empty());
    }

    // -- trait object --

    #[test]
    fn trait_object_works() {
        let mut bus: Box<dyn EventBus> = Box::new(InMemoryEventBus::new());
        bus.push(make_event(1, "A"));
        assert_eq!(bus.len(), 1);
        assert!(bus.poll().is_some());
        assert!(bus.is_empty());
    }

    // -- 게임 루프 시뮬레이션 --

    #[test]
    fn game_loop_drain_simulation() {
        // 게임 루프에서 큐를 비울 때까지 이벤트 처리하는 패턴
        let mut bus = InMemoryEventBus::new();
        bus.push(make_event(1, "수련"));
        bus.push(make_event(2, "대화"));
        bus.push(make_event(3, "관찰"));

        let mut processed = Vec::new();
        while let Some(event) = bus.poll() {
            processed.push(event.header().experience_id);
        }

        assert_eq!(processed.len(), 3);
        assert_eq!(processed[0], ExperienceId::new(1));
        assert_eq!(processed[1], ExperienceId::new(2));
        assert_eq!(processed[2], ExperienceId::new(3));
        assert!(bus.is_empty());
    }

    // -- 혼합 이벤트 유형 --

    #[test]
    fn mixed_event_types_in_queue() {
        let mut bus = InMemoryEventBus::new();

        // 다양한 유형의 이벤트를 큐에 넣을 수 있다
        bus.push(ExperienceEvent::Training {
            header: ExperienceHeader::new(
                ExperienceId::new(1),
                CharacterId::new(1),
                GameTime::new(1200, 3, 15),
                LocationId::new(10),
                5.0,
            ),
            skill: MartialArtId::new(1),
            method: String::new(),
            mentor: None,
            companion: None,
            duration: 1,
            intensity: 5,
        });
        bus.push(make_event(2, "관찰"));

        assert_eq!(bus.len(), 2);
        assert_eq!(bus.poll().unwrap().name(), "ExpTraining");
        assert_eq!(bus.poll().unwrap().name(), "ExpObservation");
    }

    // -- Default --

    #[test]
    fn default_is_empty() {
        let bus = InMemoryEventBus::default();
        assert!(bus.is_empty());
    }
}
