//! SceneTask — Scene 당 독립 async task (B안 B4 Session 4)
//!
//! 각 Scene은 자기 `mpsc::Receiver<Command>` 루프에서 커맨드를 순차 소비한다.
//! 같은 Scene의 커맨드는 mpsc FIFO로 순서 보장, 서로 다른 Scene은
//! caller 런타임이 각자의 task를 병렬 스케줄링 → 진짜 multi-scene 병렬성 달성.
//!
//! ## Fire-and-forget 계약
//! - 에러는 `tracing::error!`로만 기록. oneshot 응답 채널 없음.
//! - 성공은 `EventBus` broadcast (subscribe 측에서 관찰).
//! - `mpsc::Sender`가 drop되면 `recv()`가 `None` 반환 → 루프 자연 종료.
//!
//! ## 런타임 중립
//! 이 모듈은 `tokio::spawn`을 호출하지 **않는다**. Director가 caller가 주입한
//! `Arc<dyn Spawner>`를 통해 future를 spawn한다 → 라이브러리 core가 tokio 런타임에
//! 종속되지 않도록 유지.

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::application::command::dispatcher::CommandDispatcher;
use crate::application::command::types::Command;
use crate::domain::scene_id::SceneId;
use crate::domain::tuning::SCENE_TASK_CHANNEL_CAPACITY;
use crate::ports::MindRepository;

use super::spawner::Spawner;

/// Scene task를 caller 런타임에서 spawn하고 command sender를 반환.
///
/// 반환된 `mpsc::Sender<Command>`를 Director가 보관하고, 모든 dispatch_to 호출은
/// 이 sender로 forward된다. Sender가 drop되는 순간 task는 다음 recv()에서 None을 받고
/// 종료한다.
///
/// 채널 capacity는 `crate::domain::tuning::SCENE_TASK_CHANNEL_CAPACITY`.
pub(super) fn spawn_scene_task<R>(
    scene_id: SceneId,
    dispatcher: Arc<CommandDispatcher<R>>,
    spawner: &Arc<dyn Spawner>,
) -> mpsc::Sender<Command>
where
    R: MindRepository + Send + Sync + 'static,
{
    let (tx, mut rx) = mpsc::channel::<Command>(SCENE_TASK_CHANNEL_CAPACITY);
    let scene_id_for_log = scene_id.clone();

    spawner.spawn(Box::pin(async move {
        tracing::debug!(scene = %scene_id_for_log, "scene task started");
        while let Some(cmd) = rx.recv().await {
            if let Err(e) = dispatcher.dispatch_v2(cmd).await {
                tracing::error!(
                    scene = %scene_id_for_log,
                    error = %e,
                    "scene command dispatch_v2 failed"
                );
            }
        }
        tracing::debug!(scene = %scene_id_for_log, "scene task ended");
    }));

    tx
}
