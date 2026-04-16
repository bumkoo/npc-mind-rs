//! EventBus — 콜백 기반 이벤트 발행/구독
//!
//! tokio 의존 없이 `std::sync::RwLock`만 사용합니다.
//! Mind Studio에서 `tokio::sync::broadcast`로 브릿지할 수 있습니다.

use crate::domain::event::DomainEvent;
use std::sync::RwLock;

type Listener = Box<dyn Fn(&DomainEvent) + Send + Sync>;

/// 동기 이벤트 버스 — 구독자에게 이벤트를 즉시 전달
pub struct EventBus {
    listeners: RwLock<Vec<Listener>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            listeners: RwLock::new(Vec::new()),
        }
    }

    /// 이벤트 구독 — 콜백은 `publish` 호출 시 동기적으로 실행됨
    pub fn subscribe(&self, listener: impl Fn(&DomainEvent) + Send + Sync + 'static) {
        let mut listeners = self.listeners.write().unwrap();
        listeners.push(Box::new(listener));
    }

    /// 이벤트 발행 — 모든 구독자에게 순차 전달
    pub fn publish(&self, event: &DomainEvent) {
        let listeners = self.listeners.read().unwrap();
        for listener in listeners.iter() {
            listener(event);
        }
    }

    /// 현재 구독자 수
    pub fn listener_count(&self) -> usize {
        let listeners = self.listeners.read().unwrap();
        listeners.len()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
