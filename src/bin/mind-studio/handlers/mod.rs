use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use npc_mind::application::error::MindServiceError;

use serde::{Deserialize, Serialize};
use crate::state::TurnRecord;

/// 공통 CRUD 핸들러 구현 매크로 (단일 ID 기준)
#[macro_export]
macro_rules! impl_crud_handlers {
    ($item_type:ty, $field:ident, $list_fn:ident, $upsert_fn:ident, $delete_fn:ident, $event:expr) => {
        pub async fn $list_fn(
            axum::extract::State(state): axum::extract::State<crate::state::AppState>
        ) -> axum::Json<Vec<$item_type>> {
            let inner = state.inner.read().await;
            let mut items: Vec<$item_type> = inner.$field.values().cloned().collect();
            items.sort_by(|a, b| a.id.cmp(&b.id));
            axum::Json(items)
        }

        pub async fn $upsert_fn(
            axum::extract::State(state): axum::extract::State<crate::state::AppState>,
            axum::Json(item): axum::Json<$item_type>,
        ) -> axum::http::StatusCode {
            {
                let mut inner = state.inner.write().await;
                inner.$field.insert(item.id.clone(), item);
                inner.scenario_modified = true;
            }
            // B5.2 (3/3): UI write가 domain state를 바꾼 경우 공유 repo도 재구성.
            state.rebuild_repo_from_inner().await;
            state.emit($event);
            axum::http::StatusCode::OK
        }

        pub async fn $delete_fn(
            axum::extract::State(state): axum::extract::State<crate::state::AppState>,
            axum::extract::Path(id): axum::extract::Path<String>,
        ) -> axum::http::StatusCode {
            {
                let mut inner = state.inner.write().await;
                inner.$field.remove(&id);
                inner.scenario_modified = true;
            }
            state.rebuild_repo_from_inner().await;
            state.emit($event);
            axum::http::StatusCode::OK
        }
    };

    // 관계(Relationship) 전용 — .key() 메서드 활용
    ($item_type:ty, $field:ident, $list_fn:ident, $upsert_fn:ident, $delete_fn:ident, relationship, $event:expr) => {
        pub async fn $list_fn(
            axum::extract::State(state): axum::extract::State<crate::state::AppState>
        ) -> axum::Json<Vec<$item_type>> {
            let inner = state.inner.read().await;
            let mut items: Vec<$item_type> = inner.$field.values().cloned().collect();
            items.sort_by(|a, b| a.key().cmp(&b.key()));
            axum::Json(items)
        }

        pub async fn $upsert_fn(
            axum::extract::State(state): axum::extract::State<crate::state::AppState>,
            axum::Json(rel): axum::Json<$item_type>,
        ) -> axum::http::StatusCode {
            {
                let mut inner = state.inner.write().await;
                let key = rel.key();
                inner.$field.insert(key, rel);
                inner.scenario_modified = true;
            }
            state.rebuild_repo_from_inner().await;
            state.emit($event);
            axum::http::StatusCode::OK
        }

        pub async fn $delete_fn(
            axum::extract::State(state): axum::extract::State<crate::state::AppState>,
            axum::extract::Path((owner, target)): axum::extract::Path<(String, String)>,
        ) -> axum::http::StatusCode {
            {
                let mut inner = state.inner.write().await;
                let key = format!("{}:{}", owner, target);
                inner.$field.remove(&key);
                inner.scenario_modified = true;
            }
            state.rebuild_repo_from_inner().await;
            state.emit($event);
            axum::http::StatusCode::OK
        }
    };
}

pub mod npc;
pub mod relationship;
pub mod object;
pub mod scenario;
pub mod events;
pub mod v2_scenes;
#[cfg(feature = "chat")]
pub mod chat;
#[cfg(feature = "chat")]
pub mod llm;

#[derive(Serialize, Deserialize)]
pub struct SaveRequest {
    pub path: String,
    #[serde(default)]
    pub save_type: Option<String>,
}

#[derive(Serialize)]
pub struct SaveResponse {
    pub path: String,
}

#[derive(Serialize)]
pub struct LoadResultResponse {
    pub turn_history: Vec<TurnRecord>,
}

// ---------------------------------------------------------------------------
// WebUI 전용 에러 타입
// ---------------------------------------------------------------------------
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Service(#[from] MindServiceError),
    #[error("Internal error: {0}")]
    Internal(String),
    #[allow(dead_code)]
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    /// B4 Session 3 Option B-Mini: v2 Director lifecycle 에러 (강타입 보존).
    /// variant별로 HTTP 상태 코드 분기: SceneNotActive → 404, SceneMismatch → 400,
    /// SceneAlreadyActive → 409.
    #[error(transparent)]
    Director(npc_mind::application::director::DirectorError),
    /// v2 dispatch 에러 (강타입 보존).
    /// InvalidSituation은 400 (client), CascadeTooDeep/EventBudgetExceeded/
    /// HandlerFailed는 500 (server invariant 위반).
    #[error(transparent)]
    V2Dispatch(#[from] npc_mind::application::command::dispatcher::DispatchV2Error),
}

impl From<npc_mind::application::director::DirectorError> for AppError {
    fn from(e: npc_mind::application::director::DirectorError) -> Self {
        use npc_mind::application::director::DirectorError as D;
        // `Dispatch` variant만 v2Dispatch로 분리, 나머지는 Director에서 HTTP 매핑.
        match e {
            D::Dispatch(de) => AppError::V2Dispatch(de),
            other => AppError::Director(other),
        }
    }
}

#[cfg(feature = "chat")]
impl From<npc_mind::ports::ConversationError> for AppError {
    fn from(e: npc_mind::ports::ConversationError) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        use npc_mind::application::command::dispatcher::DispatchV2Error as Dv2;
        use npc_mind::application::director::DirectorError as Derr;

        let (status, message) = match self {
            AppError::Service(ref e) => match e {
                MindServiceError::NpcNotFound(_) | MindServiceError::RelationshipNotFound(_, _) => {
                    (StatusCode::NOT_FOUND, e.to_string())
                }
                MindServiceError::InvalidSituation(_) | MindServiceError::EmotionStateNotFound => {
                    (StatusCode::BAD_REQUEST, e.to_string())
                }
                _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            },
            AppError::NotFound(ref msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::NotImplemented(ref msg) => (StatusCode::NOT_IMPLEMENTED, msg.clone()),
            AppError::Internal(ref msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            // B4 Session 3 Option B-Mini: Director lifecycle variant 별 매핑
            AppError::Director(ref e) => match e {
                Derr::SceneAlreadyActive(_) => (StatusCode::CONFLICT, e.to_string()),
                Derr::SceneNotActive(_) => (StatusCode::NOT_FOUND, e.to_string()),
                Derr::SceneMismatch(_, _, _) => (StatusCode::BAD_REQUEST, e.to_string()),
                // SceneChannelClosed: SceneTask receiver가 drop된 비정상 상태 — 서버 invariant 위반.
                Derr::SceneChannelClosed(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                // Dispatch variant는 From에서 V2Dispatch로 분리되므로 도달 불가
                Derr::Dispatch(_) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            },
            // v2 dispatch: 클라이언트 입력 오류(400/404) vs 서버 invariant 위반(500) 분기
            AppError::V2Dispatch(ref e) => match e {
                // InvalidSituation의 메시지에 "not found"가 섞여있으면 404, 그 외 400.
                Dv2::InvalidSituation(msg) => {
                    if msg.to_lowercase().contains("not found") {
                        (StatusCode::NOT_FOUND, e.to_string())
                    } else {
                        (StatusCode::BAD_REQUEST, e.to_string())
                    }
                }
                // HandlerFailed의 source variant별 HTTP 매핑:
                //   - NpcNotFound / RelationshipNotFound → 404 (리소스 부재)
                //   - EmotionStateNotFound → 400 (워크플로우 순서 오류: appraise 선행 누락)
                //   - InvalidInput → 400 (DTO 검증 실패)
                //   - Infrastructure / Repository → 500 (서버 invariant 위반)
                Dv2::HandlerFailed { source, .. } => {
                    use npc_mind::application::command::handler_v2::HandlerError;
                    match source {
                        HandlerError::NpcNotFound(_)
                        | HandlerError::RelationshipNotFound { .. } => {
                            (StatusCode::NOT_FOUND, e.to_string())
                        }
                        HandlerError::EmotionStateNotFound(_) | HandlerError::InvalidInput(_) => {
                            (StatusCode::BAD_REQUEST, e.to_string())
                        }
                        HandlerError::Infrastructure(_) | HandlerError::Repository(_) => {
                            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                        }
                    }
                }
                Dv2::CascadeTooDeep { .. } | Dv2::EventBudgetExceeded => {
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
            },
        };

        let body = Json(serde_json::json!({
            "error": message
        }));

        (status, body).into_response()
    }
}
