use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use npc_mind::application::mind_service::MindServiceError;

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
