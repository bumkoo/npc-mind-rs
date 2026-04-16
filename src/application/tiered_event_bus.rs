//! TieredEventBus — 동기(Tier 1) + 비동기(Tier 2) 이벤트 발행
//!
//! - Tier 1: 동기 콜백 — Projection 갱신 등 즉시 완료 작업
//! - Tier 2: 채널 기반 — MemoryAgent, SummaryAgent 등 시간 소요 작업
//!
//! `publish()` 호출 시 Tier 1은 블로킹, Tier 2는 채널에 넣고 즉시 리턴.

use crate::domain::event::DomainEvent;
use std::sync::RwLock;

type SyncListener = Box<dyn Fn(&DomainEvent) + Send + Sync>;

/// Tier 2 비동기 이벤트 소비자 — 채널 기반 논블로킹
pub trait AsyncEventSink: Send + Sync {
    /// 이벤트를 채널에 전송 (논블로킹)
    fn send(&self, event: DomainEvent);
}

/// 2-Tier 이벤트 버스
pub struct TieredEventBus {
    sync_listeners: RwLock<Vec<SyncListener>>,
    async_sinks: RwLock<Vec<Box<dyn AsyncEventSink>>>,
}

impl TieredEventBus {
    pub fn new() -> Self {
        Self {
            sync_listeners: RwLock::new(Vec::new()),
            async_sinks: RwLock::new(Vec::new()),
        }
    }

    /// Tier 1 동기 리스너 등록
    pub fn subscribe_sync(&self, listener: impl Fn(&DomainEvent) + Send + Sync + 'static) {
        let mut listeners = self.sync_listeners.write().unwrap();
        listeners.push(Box::new(listener));
    }

    /// 하위 호환 별칭 — `subscribe_sync()`과 동일
    pub fn subscribe(&self, listener: impl Fn(&DomainEvent) + Send + Sync + 'static) {
        self.subscribe_sync(listener);
    }

    /// Tier 2 비동기 소비자 등록
    pub fn register_async(&self, sink: impl AsyncEventSink + 'static) {
        let mut sinks = self.async_sinks.write().unwrap();
        sinks.push(Box::new(sink));
    }

    /// 이벤트 발행
    ///
    /// 1. Tier 1 동기 리스너 즉시 실행 (블로킹)
    /// 2. Tier 2 비동기 소비자에게 전송 (논블로킹)
    pub fn publish(&self, event: &DomainEvent) {
        // Tier 1: 동기
        {
            let listeners = self.sync_listeners.read().unwrap();
            for listener in listeners.iter() {
                listener(event);
            }
        }

        // Tier 2: 비동기 (논블로킹)
        {
            let sinks = self.async_sinks.read().unwrap();
            for sink in sinks.iter() {
                sink.send(event.clone());
            }
        }
    }

    pub fn sync_listener_count(&self) -> usize {
        self.sync_listeners.read().unwrap().len()
    }

    pub fn async_sink_count(&self) -> usize {
        self.async_sinks.read().unwrap().len()
    }
}

impl Default for TieredEventBus {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// StdThreadSink — std::thread + mpsc 기반 Tier 2 소비자
// ---------------------------------------------------------------------------

/// 표준 라이브러리 기반 비동기 이벤트 소비자
///
/// 백그라운드 스레드에서 이벤트를 소비합니다.
/// Drop 시 sender가 해제되어 스레드가 자동 종료됩니다.
pub struct StdThreadSink {
    tx: std::sync::mpsc::Sender<DomainEvent>,
}

impl StdThreadSink {
    /// 핸들러 함수를 백그라운드 스레드에서 실행하는 sink 생성
    pub fn spawn(handler: impl Fn(DomainEvent) + Send + 'static) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                handler(event);
            }
            // sender가 모두 drop되면 recv()가 Err → 스레드 종료
        });
        Self { tx }
    }
}

impl AsyncEventSink for StdThreadSink {
    fn send(&self, event: DomainEvent) {
        let _ = self.tx.send(event);
    }
}

// ---------------------------------------------------------------------------
// TokioSink — tokio::mpsc 기반 (chat/mind-studio feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "chat")]
pub struct TokioSink {
    tx: tokio::sync::mpsc::UnboundedSender<DomainEvent>,
}

#[cfg(feature = "chat")]
impl TokioSink {
    /// tokio unbounded channel의 sender를 래핑
    ///
    /// 호출자가 receiver를 tokio task로 소비해야 합니다:
    /// ```rust,ignore
    /// let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    /// let sink = TokioSink::new(tx);
    /// bus.register_async(sink);
    /// tokio::spawn(async move {
    ///     while let Some(event) = rx.recv().await {
    ///         // 처리
    ///     }
    /// });
    /// ```
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<DomainEvent>) -> Self {
        Self { tx }
    }
}

#[cfg(feature = "chat")]
impl AsyncEventSink for TokioSink {
    fn send(&self, event: DomainEvent) {
        let _ = self.tx.send(event);
    }
}
