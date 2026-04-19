//! Director — 다중 Scene 수명 관리 facade (B안 B4 Session 2)
//!
//! `CommandDispatcher`의 v2 경로를 **Scene 단위로 라우팅**하는 상위 레이어.
//! 단일 Dispatcher와 공유 Repository를 감싸며, 활성 Scene 목록을 관리하고
//! 커맨드의 scene_id가 해당 Scene의 (npc_id, partner_id)와 일치하는지 검증한다.
//!
//! ## 책임
//! - Scene 시작/종료 lifecycle (내부적으로 `Command::StartScene` / `Command::EndDialogue` 래핑)
//! - `dispatch_to(scene_id, cmd)` — 커맨드의 npc/partner가 scene_id와 일치할 때만 허용
//! - `active_scenes()` — 현재 진행 중인 Scene id 목록
//! - v2 dispatch 경로 강제 (`dispatch_v2` 호출, v1 미사용)
//!
//! ## 한계 (Session 3에서 해소 예정)
//! - **병렬 실행 없음** — Sync Dispatcher 하나를 공유하므로 Scene 간 커맨드는 직렬 처리
//! - tokio 미사용 — mpsc/broadcast는 Session 3에서 도입
//! - Mind Studio 통합은 Session 3
//!
//! ## Repository 공유 모델
//! 하나의 `InMemoryRepository`가 모든 Scene의 NPC/관계/EmotionState를 공유한다.
//! Scene들은 `InMemoryRepository.scenes: HashMap<SceneId, Scene>`에 저장되어 격리됨.
//! 감정은 NPC 단위(scene 간 공유) — 같은 NPC가 두 Scene에 참여하는 경우 최신 대화의
//! 감정이 양쪽에 반영된다 (의도적 단순화; Session 3에서 Scene-scoped emotion 검토).

use crate::application::command::dispatcher::{CommandDispatcher, DispatchV2Error, DispatchV2Output};
use crate::application::command::types::Command;
use crate::application::dto::SceneFocusInput;
use crate::domain::scene_id::SceneId;
use crate::ports::MindRepository;

use std::collections::HashSet;

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

    /// 하위 dispatch_v2 실패
    #[error(transparent)]
    Dispatch(#[from] DispatchV2Error),
}

/// Scene 수명을 관리하는 Dispatcher facade
///
/// `<R: MindRepository + Send + Sync>` 제약: v2 경로가 `&(dyn MindRepository + Send + Sync)`를
/// 요구하므로 Repository 타입이 Send+Sync여야 한다. `InMemoryRepository`는 이를 만족.
pub struct Director<R: MindRepository + Send + Sync> {
    dispatcher: CommandDispatcher<R>,
    active_scenes: HashSet<SceneId>,
}

impl<R: MindRepository + Send + Sync> Director<R> {
    /// 주어진 Dispatcher를 감싸는 Director를 생성한다.
    /// Dispatcher는 `with_default_handlers()`가 호출된 상태여야 v2 경로가 정상 작동.
    pub fn new(dispatcher: CommandDispatcher<R>) -> Self {
        Self {
            dispatcher,
            active_scenes: HashSet::new(),
        }
    }

    /// Scene 시작 — 내부적으로 `Command::StartScene`을 dispatch_v2.
    ///
    /// 동일 (npc_id, partner_id) Scene이 이미 활성이면 `SceneAlreadyActive` 반환.
    pub fn start_scene(
        &mut self,
        npc_id: impl Into<String>,
        partner_id: impl Into<String>,
        significance: Option<f32>,
        focuses: Vec<SceneFocusInput>,
    ) -> Result<(SceneId, DispatchV2Output), DirectorError> {
        let npc_id = npc_id.into();
        let partner_id = partner_id.into();
        let scene_id = SceneId::new(&npc_id, &partner_id);

        if self.active_scenes.contains(&scene_id) {
            return Err(DirectorError::SceneAlreadyActive(scene_id));
        }

        let out = self.dispatcher.dispatch_v2(Command::StartScene {
            npc_id,
            partner_id,
            significance,
            focuses,
        })?;
        self.active_scenes.insert(scene_id.clone());
        Ok((scene_id, out))
    }

    /// 특정 Scene에 커맨드 송신 — v2 경로.
    ///
    /// 검증: scene_id가 active이어야 하고, 커맨드의 (npc_id, partner_id)가 scene_id와 일치해야 함.
    pub fn dispatch_to(
        &mut self,
        scene_id: &SceneId,
        cmd: Command,
    ) -> Result<DispatchV2Output, DirectorError> {
        if !self.active_scenes.contains(scene_id) {
            return Err(DirectorError::SceneNotActive(scene_id.clone()));
        }
        let cmd_npc = cmd.npc_id().to_string();
        let cmd_partner = cmd.partner_id().to_string();
        if cmd_npc != scene_id.npc_id || cmd_partner != scene_id.partner_id {
            return Err(DirectorError::SceneMismatch(
                cmd_npc,
                cmd_partner,
                scene_id.clone(),
            ));
        }
        Ok(self.dispatcher.dispatch_v2(cmd)?)
    }

    /// Scene 종료 — 내부적으로 `Command::EndDialogue` dispatch_v2 + active_scenes 제거.
    pub fn end_scene(
        &mut self,
        scene_id: &SceneId,
        significance: Option<f32>,
    ) -> Result<DispatchV2Output, DirectorError> {
        if !self.active_scenes.contains(scene_id) {
            return Err(DirectorError::SceneNotActive(scene_id.clone()));
        }
        let out = self.dispatcher.dispatch_v2(Command::EndDialogue {
            npc_id: scene_id.npc_id.clone(),
            partner_id: scene_id.partner_id.clone(),
            significance,
        })?;
        self.active_scenes.remove(scene_id);
        Ok(out)
    }

    /// 활성 Scene id 목록 (순서는 비결정적 — HashSet 기반)
    pub fn active_scenes(&self) -> Vec<SceneId> {
        self.active_scenes.iter().cloned().collect()
    }

    /// Scene 활성 여부 질의
    pub fn is_active(&self, scene_id: &SceneId) -> bool {
        self.active_scenes.contains(scene_id)
    }

    /// 내부 Dispatcher 참조 (Session 3까지는 외부 통합 테스트가 직접 dispatcher.event_store
    /// 등을 관찰해야 함)
    pub fn dispatcher(&self) -> &CommandDispatcher<R> {
        &self.dispatcher
    }

    /// 내부 Dispatcher mutable 참조
    pub fn dispatcher_mut(&mut self) -> &mut CommandDispatcher<R> {
        &mut self.dispatcher
    }
}
