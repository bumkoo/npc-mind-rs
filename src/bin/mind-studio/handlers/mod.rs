use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use npc_mind::application::mind_service::MindServiceError;

use serde::{Deserialize, Serialize};
use crate::state::TurnRecord;

pub mod npc;
pub mod relationship;
pub mod object;
pub mod scenario;
#[cfg(feature = "chat")]
pub mod chat;

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
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

#[cfg(feature = "chat")]
impl From<npc_mind::ports::ConversationError> for AppError {
    fn from(e: npc_mind::ports::ConversationError) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
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
        };

        let body = Json(serde_json::json!({
            "error": message
        }));

        (status, body).into_response()
    }
}
