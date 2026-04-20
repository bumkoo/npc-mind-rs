//! v2 Scene lifecycle 엔드포인트 (B안 B4 Session 3 Option B-Mini)
//!
//! `AppState.director_v2`를 경유하여 다중 Scene 관리 API를 REST로 노출한다.
//! 기존 v1 Mind Studio 경로와 **완전히 분리된** Repository를 쓰므로:
//! - v2 Director 내부 Repository에 NPC/Relationship 등록 후 Scene 시작 가능
//! - `POST /api/v2/npcs` / `POST /api/v2/relationships` 헬퍼로 Director 내부 repo 직접 편집
//! - v1 UI에 반영되지 않음 (shadow path)
//!
//! ## 엔드포인트
//! - `GET    /api/v2/scenes`                  — 활성 Scene 목록
//! - `POST   /api/v2/scenes/start`            — Scene 시작
//! - `POST   /api/v2/scenes/dispatch`         — Scene에 v2 커맨드 송신
//! - `DELETE /api/v2/scenes/{npc}/{partner}`  — Scene 종료
//! - `POST   /api/v2/npcs`                    — Director repo에 NPC 등록
//! - `POST   /api/v2/relationships`           — Director repo에 관계 등록

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use npc_mind::application::command::types::Command;
use npc_mind::application::dto::{SceneFocusInput, SituationInput};
use npc_mind::domain::personality::Npc;
use npc_mind::domain::relationship::Relationship;
use npc_mind::domain::scene_id::SceneId;
use npc_mind::ports::NpcWorld;

use super::AppError;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// DTO
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct SceneIdDto {
    pub npc_id: String,
    pub partner_id: String,
}

impl From<SceneId> for SceneIdDto {
    fn from(id: SceneId) -> Self {
        Self {
            npc_id: id.npc_id,
            partner_id: id.partner_id,
        }
    }
}

#[derive(Serialize)]
pub struct ActiveScenesResponse {
    pub scenes: Vec<SceneIdDto>,
}

#[derive(Deserialize)]
pub struct StartSceneRequest {
    pub npc_id: String,
    pub partner_id: String,
    #[serde(default)]
    pub significance: Option<f32>,
    #[serde(default)]
    pub focuses: Vec<SceneFocusInput>,
}

#[derive(Serialize)]
pub struct StartSceneResponse {
    pub scene_id: SceneIdDto,
}

/// Fire-and-forget 커맨드 전송 응답. B4 Session 4부터 Director는 커맨드를 SceneTask mpsc로
/// forward하고 즉시 반환한다. 발행 이벤트 관찰은 `event_bus().subscribe()`로 caller가 수행.
#[derive(Serialize)]
pub struct AckResponse {
    pub ok: bool,
}

#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum DispatchCommandBody {
    Appraise {
        npc_id: String,
        partner_id: String,
        #[serde(default)]
        situation: Option<SituationInput>,
    },
    ApplyStimulus {
        npc_id: String,
        partner_id: String,
        pleasure: f32,
        arousal: f32,
        dominance: f32,
        #[serde(default)]
        situation_description: Option<String>,
    },
    GenerateGuide {
        npc_id: String,
        partner_id: String,
        #[serde(default)]
        situation_description: Option<String>,
    },
    UpdateRelationship {
        npc_id: String,
        partner_id: String,
        #[serde(default)]
        significance: Option<f32>,
    },
}

impl DispatchCommandBody {
    fn into_command(self) -> Command {
        match self {
            Self::Appraise {
                npc_id,
                partner_id,
                situation,
            } => Command::Appraise {
                npc_id,
                partner_id,
                situation,
            },
            Self::ApplyStimulus {
                npc_id,
                partner_id,
                pleasure,
                arousal,
                dominance,
                situation_description,
            } => Command::ApplyStimulus {
                npc_id,
                partner_id,
                pleasure,
                arousal,
                dominance,
                situation_description,
            },
            Self::GenerateGuide {
                npc_id,
                partner_id,
                situation_description,
            } => Command::GenerateGuide {
                npc_id,
                partner_id,
                situation_description,
            },
            Self::UpdateRelationship {
                npc_id,
                partner_id,
                significance,
            } => Command::UpdateRelationship {
                npc_id,
                partner_id,
                significance,
            },
        }
    }
}

#[derive(Deserialize)]
pub struct DispatchToSceneRequest {
    pub scene_id: SceneIdDto,
    #[serde(flatten)]
    pub command: DispatchCommandBody,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/v2/scenes — 활성 Scene 목록
pub async fn list_active_scenes(
    State(state): State<AppState>,
) -> Json<ActiveScenesResponse> {
    let scenes: Vec<SceneIdDto> = state
        .director_v2
        .active_scenes()
        .await
        .into_iter()
        .map(SceneIdDto::from)
        .collect();
    Json(ActiveScenesResponse { scenes })
}

/// POST /api/v2/scenes/start — Scene 시작 (fire-and-forget)
///
/// B4 Session 4: Director가 SceneTask를 spawn하고 즉시 `SceneId`만 반환한다.
/// 초기 `SceneStarted`/`EmotionAppraised` 이벤트 관찰은 `event_bus().subscribe()` 필요.
pub async fn start_scene(
    State(state): State<AppState>,
    Json(req): Json<StartSceneRequest>,
) -> Result<Json<StartSceneResponse>, AppError> {
    let scene_id = state
        .director_v2
        .start_scene(req.npc_id, req.partner_id, req.significance, req.focuses)
        .await?;
    Ok(Json(StartSceneResponse {
        scene_id: scene_id.into(),
    }))
}

/// POST /api/v2/scenes/dispatch — 특정 Scene에 커맨드 송신 (fire-and-forget)
pub async fn dispatch_to_scene(
    State(state): State<AppState>,
    Json(req): Json<DispatchToSceneRequest>,
) -> Result<Json<AckResponse>, AppError> {
    let scene_id = SceneId::new(req.scene_id.npc_id, req.scene_id.partner_id);
    let cmd = req.command.into_command();
    state.director_v2.dispatch_to(&scene_id, cmd).await?;
    Ok(Json(AckResponse { ok: true }))
}

/// DELETE /api/v2/scenes/{npc_id}/{partner_id} — Scene 종료 (fire-and-forget)
pub async fn end_scene(
    State(state): State<AppState>,
    Path((npc_id, partner_id)): Path<(String, String)>,
) -> Result<Json<AckResponse>, AppError> {
    let scene_id = SceneId::new(npc_id, partner_id);
    state.director_v2.end_scene(&scene_id, None).await?;
    Ok(Json(AckResponse { ok: true }))
}

/// POST /api/v2/npcs — Director 내부 Repository에 NPC 등록
///
/// Mind Studio v1 AppState와 분리된 Repository이므로 v2 경로 쓰려면 별도 등록 필요.
pub async fn upsert_npc_v2(
    State(state): State<AppState>,
    Json(npc): Json<Npc>,
) -> StatusCode {
    state
        .director_v2
        .dispatcher()
        .repository_guard()
        .add_npc(npc);
    StatusCode::OK
}

/// POST /api/v2/relationships — Director 내부 Repository에 관계 등록
pub async fn upsert_relationship_v2(
    State(state): State<AppState>,
    Json(rel): Json<Relationship>,
) -> StatusCode {
    let owner = rel.owner_id().to_string();
    let target = rel.target_id().to_string();
    state
        .director_v2
        .dispatcher()
        .repository_guard()
        .save_relationship(&owner, &target, rel);
    StatusCode::OK
}

/// GET /api/v2/scene-ids — Director Repository에 저장된 Scene id 전수 (active + 이전)
pub async fn list_all_scene_ids(
    State(state): State<AppState>,
) -> Json<ActiveScenesResponse> {
    let scenes: Vec<SceneIdDto> = state
        .director_v2
        .dispatcher()
        .repository_guard()
        .list_scene_ids()
        .into_iter()
        .map(SceneIdDto::from)
        .collect();
    Json(ActiveScenesResponse { scenes })
}
