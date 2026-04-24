//! EventBus — `tokio::sync::broadcast` 기반 fan-out 이벤트 버스
//!
//! 공개 API는 `futures::Stream`만 노출하여 호출자가 tokio를 인식할 필요가 없다.
//! 내부 구현은 `tokio::sync::broadcast::Sender`와
//! `tokio_stream::wrappers::BroadcastStream`을 사용한다.
//!
//! - `publish(&event)`: sync. `broadcast::Sender::send()`가 sync이므로
//!   dispatch 경로 전체가 async로 바뀌지 않는다.
//! - `subscribe()`: `impl Stream<Item = Arc<DomainEvent>>` 반환. 소비자는
//!   자기 async 런타임(tokio, bevy_tasks, async-std 등)에서 폴링한다.
//!
//! # Lag 처리
//!
//! `broadcast` 채널은 capacity를 초과하면 가장 오래된 이벤트를 덮어쓴다.
//! `subscribe()`가 돌려주는 Stream은 `Lagged` 통지를 삼키므로 엄밀한
//! at-least-once가 필요한 소비자는 `EventStore::get_events_after_id`로
//! 마지막 처리 id 이후 이벤트를 replay해야 한다. (MemoryProjector 참조)

use crate::domain::event::DomainEvent;
use futures::{Stream, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

/// 기본 채널 capacity.
///
/// 소비자가 일시적으로 지연돼도 수백 이벤트까지는 lag 없이 수신한다.
/// 장기 지연이 예상되는 소비자는 별도 replay 로직으로 복구한다.
pub const DEFAULT_CAPACITY: usize = 256;

/// 이벤트 버스 — broadcast 기반 fan-out
///
/// `Clone`은 `Sender`의 저렴한 Arc clone이므로 여러 서비스에서 공유해도 된다.
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<Arc<DomainEvent>>,
}

impl EventBus {
    /// 기본 capacity로 생성
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// 명시적 capacity로 생성
    ///
    /// `tokio::sync::broadcast::channel`은 `capacity == 0`에서 panic하므로
    /// 최소 1로 클램프한다. 실수 방지를 위해 debug 빌드에서는 assert도 켠다.
    pub fn with_capacity(capacity: usize) -> Self {
        debug_assert!(capacity > 0, "EventBus capacity must be greater than 0");
        let (sender, _) = broadcast::channel(capacity.max(1));
        Self { sender }
    }

    /// 이벤트 발행 (sync)
    ///
    /// 현재 구독자가 0명이면 이벤트는 drop된다. 영속화는 `EventStore`가 담당.
    pub fn publish(&self, event: &DomainEvent) {
        // 구독자가 없으면 SendError가 반환되지만 정책상 무시한다.
        let _ = self.sender.send(Arc::new(event.clone()));
    }

    /// 구독 스트림 생성
    ///
    /// 호출자는 반환된 Stream을 자기 async 런타임에서 소비한다.
    /// Lagged 통지는 내부적으로 걸러진다.
    pub fn subscribe(&self) -> impl Stream<Item = Arc<DomainEvent>> + Send + 'static {
        BroadcastStream::new(self.sender.subscribe()).filter_map(|r| async move { r.ok() })
    }

    /// Lag 통지를 포함한 구독 스트림
    ///
    /// at-least-once 복구가 필요한 소비자용. `Err(skipped)`은 건너뛴 이벤트 수를 나타낸다.
    pub fn subscribe_with_lag(
        &self,
    ) -> impl Stream<Item = Result<Arc<DomainEvent>, u64>> + Send + 'static {
        BroadcastStream::new(self.sender.subscribe()).map(|r| match r {
            Ok(ev) => Ok(ev),
            Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => Err(n),
        })
    }

    /// 현재 활성 구독자 수
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
