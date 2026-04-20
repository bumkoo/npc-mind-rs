//! Director — 다중 Scene 수명 관리 facade (B안 B4 Session 4)
//!
//! `CommandDispatcher`의 v2 경로를 **Scene 단위 tokio(또는 다른 런타임) task**로
//! 포장한 상위 레이어. Scene별 `mpsc::Sender<Command>`를 보관하여 커맨드를
//! 해당 Scene의 전용 task로 라우팅한다. 같은 Scene의 커맨드는 FIFO 순서 보장,
//! 서로 다른 Scene은 caller 런타임이 병렬 스케줄링한다.
//!
//! ## 책임
//! - Scene 시작/종료 lifecycle (내부적으로 `Command::StartScene` / `Command::EndDialogue` 래핑)
//! - `dispatch_to(scene_id, cmd)` — 커맨드의 npc/partner가 scene_id와 일치할 때만 허용
//! - `active_scenes()` — 현재 진행 중인 Scene id 목록
//! - v2 dispatch 경로 강제 (`dispatch_v2` 호출, v1 미사용)
//!
//! ## Fire-and-forget
//! `start_scene`/`dispatch_to`/`end_scene`은 커맨드를 SceneTask mpsc로 보낸 직후 리턴한다.
//! 커맨드의 실제 결과(이벤트)는 `dispatcher().event_bus().subscribe()`로 관찰.
//! `DispatchV2Error`는 SceneTask 내부에서 `tracing::error!`로만 기록된다.
//!
//! ## Repository 공유 모델
//! 하나의 `InMemoryRepository`가 모든 Scene의 NPC/관계/EmotionState를 공유한다.
//! `CommandDispatcher`가 내부적으로 `Arc<Mutex<R>>`로 감싸서 각 dispatch_v2 호출이
//! 짧은 lock 구간으로 serialize된다. `save_scene`은 `SceneId` 별 slot에 저장되어
//! Scene 간 오염 없음.
//!
//! ## 런타임 의존성
//! Director 자체는 `tokio::spawn`을 **호출하지 않는다**. 대신 생성 시
//! `Arc<dyn Spawner>`를 받아 Scene task를 등록한다. Mind Studio/tests는
//! 자기 tokio 런타임으로 spawn하는 closure를 주입한다.

mod scene_task;
mod spawner;

pub use spawner::Spawner;

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{RwLock, mpsc};

use crate::application::command::dispatcher::{CommandDispatcher, DispatchV2Error};
use crate::application::command::types::Command;
use crate::application::dto::SceneFocusInput;
use crate::domain::scene_id::SceneId;
use crate::ports::MindRepository;

use scene_task::spawn_scene_task;

/// Director 전용 에러
#[derive(Debug, thiserror::Error)]
pub enum DirectorError {
    /// `dispatch_to`의 scene_id가 active scenes 집합에 없음
    #[error("scene {0} is not active")]
    SceneNotActive(SceneId),

    /// `dispatch_to`의 커맨드 (npc_id, partner_id)가 scene_id와 불일치
    #[error("command target ({0}, {1}) does not match scene {2}")]
    SceneMismatch(String, String, SceneId),

    /// 이미 활성 Scene
    #[error("scene {0} is already active")]
    SceneAlreadyActive(SceneId),

    /// SceneTask mpsc 채널이 이미 닫힘 (receiver drop 등)
    #[error("scene {0} channel is closed")]
    SceneChannelClosed(SceneId),

    /// 하위 dispatch_v2 실패 — 초기 이벤트 생성 단계(Situation 해석 등)에서 발생 가능
    #[error(transparent)]
    Dispatch(#[from] DispatchV2Error),
}

/// Scene 수명을 관리하는 Dispatcher facade
///
/// `<R: MindRepository + Send + Sync + 'static>` 제약: SceneTask가 `Arc<CommandDispatcher<R>>`를
/// 런타임 스레드로 이동시키므로 Repository는 Send+Sync+'static이어야 한다.
/// `InMemoryRepository`는 이를 만족.
pub struct Director<R: MindRepository + Send + Sync + 'static> {
    dispatcher: Arc<CommandDispatcher<R>>,
    spawner: Arc<dyn Spawner>,
    /// Scene id → SceneTask mpsc sender. RwLock은 짧은 읽기(dispatch_to)와
    /// 드문 쓰기(start/end_scene) 패턴에 최적화.
    senders: RwLock<HashMap<SceneId, mpsc::Sender<Command>>>,
}

impl<R: MindRepository + Send + Sync + 'static> Director<R> {
    /// 주어진 Dispatcher + Spawner를 감싸는 Director를 생성한다.
    /// Dispatcher는 `with_default_handlers()`가 호출된 상태여야 v2 경로가 정상 작동.
    pub fn new(dispatcher: CommandDispatcher<R>, spawner: Arc<dyn Spawner>) -> Self {
        Self {
            dispatcher: Arc::new(dispatcher),
            spawner,
            senders: RwLock::new(HashMap::new()),
        }
    }

    /// 내부 Dispatcher Arc — broadcast 구독(`event_bus()`), repository 조회 등에 사용.
    pub fn dispatcher(&self) -> &Arc<CommandDispatcher<R>> {
        &self.dispatcher
    }

    /// Scene 시작 — SceneTask spawn + 첫 메시지로 `Command::StartScene` 전송.
    ///
    /// 동일 (npc_id, partner_id) Scene이 이미 활성이면 `SceneAlreadyActive` 반환.
    /// 실제 SceneStarted/EmotionAppraised 이벤트는 EventBus subscriber가 관찰.
    pub async fn start_scene(
        &self,
        npc_id: impl Into<String>,
        partner_id: impl Into<String>,
        significance: Option<f32>,
        focuses: Vec<SceneFocusInput>,
    ) -> Result<SceneId, DirectorError> {
        let npc_id = npc_id.into();
        let partner_id = partner_id.into();
        let scene_id = SceneId::new(&npc_id, &partner_id);

        {
            let senders = self.senders.read().await;
            if senders.contains_key(&scene_id) {
                return Err(DirectorError::SceneAlreadyActive(scene_id));
            }
        }

        let tx = spawn_scene_task(scene_id.clone(), Arc::clone(&self.dispatcher), &self.spawner);
        tx.send(Command::StartScene {
            npc_id,
            partner_id,
            significance,
            focuses,
        })
        .await
        .map_err(|_| DirectorError::SceneChannelClosed(scene_id.clone()))?;

        self.senders.write().await.insert(scene_id.clone(), tx);
        Ok(scene_id)
    }

    /// 특정 Scene에 커맨드 송신 — v2 경로(fire-and-forget).
    ///
    /// 검증: scene_id가 active이어야 하고, 커맨드의 (npc_id, partner_id)가 scene_id와 일치해야 함.
    /// 커맨드는 SceneTask mpsc로 forward되며, 본 함수는 send 완료 직후 리턴한다.
    pub async fn dispatch_to(
        &self,
        scene_id: &SceneId,
        cmd: Command,
    ) -> Result<(), DirectorError> {
        let cmd_npc = cmd.npc_id().to_string();
        let cmd_partner = cmd.partner_id().to_string();
        if cmd_npc != scene_id.npc_id || cmd_partner != scene_id.partner_id {
            return Err(DirectorError::SceneMismatch(
                cmd_npc,
                cmd_partner,
                scene_id.clone(),
            ));
        }

        let senders = self.senders.read().await;
        let tx = senders
            .get(scene_id)
            .ok_or_else(|| DirectorError::SceneNotActive(scene_id.clone()))?;
        tx.send(cmd)
            .await
            .map_err(|_| DirectorError::SceneChannelClosed(scene_id.clone()))
    }

    /// Scene 종료 — `Command::EndDialogue` 전송 후 sender drop → task 자연 종료.
    ///
    /// sender 삭제 순서에 주의: 먼저 send → 후 remove. send가 완료되면 SceneTask의 mpsc
    /// buffer에 EndDialogue가 들어간 상태이므로 drop해도 처리된다.
    pub async fn end_scene(
        &self,
        scene_id: &SceneId,
        significance: Option<f32>,
    ) -> Result<(), DirectorError> {
        let tx = {
            let senders = self.senders.read().await;
            senders
                .get(scene_id)
                .cloned()
                .ok_or_else(|| DirectorError::SceneNotActive(scene_id.clone()))?
        };

        tx.send(Command::EndDialogue {
            npc_id: scene_id.npc_id.clone(),
            partner_id: scene_id.partner_id.clone(),
            significance,
        })
        .await
        .map_err(|_| DirectorError::SceneChannelClosed(scene_id.clone()))?;

        // Send 완료 후 map에서 제거 → 이 sender의 마지막 ref가 drop되면 task는
        // EndDialogue를 처리한 뒤 recv()=None으로 종료.
        self.senders.write().await.remove(scene_id);
        drop(tx);
        Ok(())
    }

    /// 활성 Scene id 목록 (순서는 비결정적 — HashMap 기반)
    pub async fn active_scenes(&self) -> Vec<SceneId> {
        self.senders.read().await.keys().cloned().collect()
    }

    /// Scene 활성 여부 질의
    pub async fn is_active(&self, scene_id: &SceneId) -> bool {
        self.senders.read().await.contains_key(scene_id)
    }
}
