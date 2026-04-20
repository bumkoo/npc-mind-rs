//! Spawner — runtime-agnostic async task spawn 추상화 (B안 B4 Session 4)
//!
//! `Director`는 Scene 별 `SceneTask`를 spawn해야 하는데, 코어 라이브러리는
//! 특정 async 런타임(tokio/bevy/smol 등)에 종속되지 않는다.
//! 대신 caller가 자기 런타임의 spawn 함수를 `Spawner`로 감싸 주입한다.
//!
//! ## 운영 원칙
//! - 라이브러리 core는 `tokio::spawn`을 직접 호출하지 **않는다**. `tokio` deps는
//!   `features = ["sync"]`만 유지 — 외부 사용자가 tokio 런타임 없이도 이 크레이트를
//!   링크할 수 있어야 한다.
//! - 테스트·Mind Studio처럼 tokio 런타임이 이미 활성화된 환경에서는 단순한
//!   `Arc::new(|fut| { tokio::spawn(fut); })` 클로저로 충분.
//!
//! ## 사용 예
//! ```ignore
//! use std::sync::Arc;
//! use futures::future::BoxFuture;
//! use npc_mind::application::director::Spawner;
//!
//! let spawner: Arc<dyn Spawner> = Arc::new(|fut: BoxFuture<'static, ()>| {
//!     tokio::spawn(fut);
//! });
//! let director = Director::new(dispatcher, spawner);
//! ```

use futures::future::BoxFuture;

/// 런타임 중립 spawn 함수 추상화.
///
/// `Spawner::spawn`이 받은 future는 **detach**되어 runtime이 구동한다. Director는
/// 이 future가 언제 종료되는지 관찰하지 않는다(Fire-and-forget — 성공 이벤트는
/// `EventBus`로 관찰).
pub trait Spawner: Send + Sync + 'static {
    /// 주어진 future를 caller 런타임에서 실행하도록 스케줄링.
    fn spawn(&self, future: BoxFuture<'static, ()>);
}

/// 클로저로도 `Spawner`를 구현할 수 있도록 blanket impl 제공.
///
/// 이 impl 덕분에 caller는 trait object나 struct 정의 없이
/// `Arc::new(|fut| ...)` 형태로 spawner를 작성할 수 있다.
impl<F> Spawner for F
where
    F: Fn(BoxFuture<'static, ()>) + Send + Sync + 'static,
{
    fn spawn(&self, future: BoxFuture<'static, ()>) {
        (self)(future)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn closure_implements_spawner() {
        let counter = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&counter);
        let spawner: Arc<dyn Spawner> = Arc::new(move |_fut: BoxFuture<'static, ()>| {
            c.fetch_add(1, Ordering::Relaxed);
        });

        let dummy: BoxFuture<'static, ()> = Box::pin(async {});
        spawner.spawn(dummy);
        let dummy2: BoxFuture<'static, ()> = Box::pin(async {});
        spawner.spawn(dummy2);

        assert_eq!(counter.load(Ordering::Relaxed), 2);
    }
}
