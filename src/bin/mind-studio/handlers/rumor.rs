//! Step E1 — Rumor REST 엔드포인트 (embed feature 활성 시에만 컴파일)

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::domain_sync;
use crate::events::StateEvent;
use crate::handlers::AppError;
use crate::state::AppState;

use npc_mind::application::dto::{SeedRumorRequest, SpreadRumorRequest};
use npc_mind::domain::event::EventPayload;
use npc_mind::domain::rumor::Rumor;

#[derive(Serialize)]
pub struct ListResponse {
    pub rumors: Vec<Rumor>,
}

/// `GET /api/rumors` — 전체 소문 목록 (Active/Fading/Faded 포함).
pub async fn list(State(state): State<AppState>) -> Result<Json<ListResponse>, AppError> {
    let rumors = state
        .rumor_store
        .list_all()
        .map_err(|e| AppError::Internal(format!("RumorStore.list_all 실패: {}", e)))?;
    Ok(Json(ListResponse { rumors }))
}

/// `POST /api/rumors/seed` — `Command::SeedRumor` dispatch.
pub async fn seed(
    State(state): State<AppState>,
    Json(req): Json<SeedRumorRequest>,
) -> Result<Json<SeedResponse>, AppError> {
    let mut inner = state.inner.write().await;
    let output = domain_sync::dispatch_seed_rumor(&state, &mut *inner, req).await?;

    // 생성된 rumor_id를 RumorSeeded 이벤트에서 추출.
    let rumor_id = output
        .events
        .iter()
        .find_map(|e| match &e.payload {
            EventPayload::RumorSeeded { rumor_id, .. } => Some(rumor_id.clone()),
            _ => None,
        })
        .ok_or_else(|| AppError::Internal("RumorSeeded 이벤트 부재".into()))?;

    drop(inner);
    state.emit(StateEvent::RumorSeeded);

    Ok(Json(SeedResponse { rumor_id }))
}

#[derive(Serialize)]
pub struct SeedResponse {
    pub rumor_id: String,
}

/// `POST /api/rumors/:id/spread` — `Command::SpreadRumor` dispatch.
pub async fn spread(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(mut body): Json<SpreadBody>,
) -> Result<Json<SpreadResponse>, AppError> {
    // URL path의 id가 body의 rumor_id를 override한다 (REST 관례).
    let req = SpreadRumorRequest {
        rumor_id: id,
        recipients: std::mem::take(&mut body.recipients),
        content_version: body.content_version.take(),
    };

    let mut inner = state.inner.write().await;
    let output = domain_sync::dispatch_spread_rumor(&state, &mut *inner, req).await?;

    let (hop_index, spread_count) = output
        .events
        .iter()
        .find_map(|e| match &e.payload {
            EventPayload::RumorSpread {
                hop_index,
                recipients,
                ..
            } => Some((*hop_index, recipients.len())),
            _ => None,
        })
        .ok_or_else(|| AppError::Internal("RumorSpread 이벤트 부재".into()))?;

    drop(inner);
    state.emit(StateEvent::RumorSpread);
    if spread_count > 0 {
        state.emit(StateEvent::MemoryCreated);
    }

    Ok(Json(SpreadResponse {
        hop_index,
        recipient_count: spread_count,
    }))
}

/// `/api/rumors/:id/spread` 요청 body. URL path가 rumor_id를 담기 때문에 DTO의
/// rumor_id 필드는 생략 가능하게 만들었다.
#[derive(Deserialize, Default)]
pub struct SpreadBody {
    pub recipients: Vec<String>,
    #[serde(default)]
    pub content_version: Option<String>,
}

#[derive(Serialize)]
pub struct SpreadResponse {
    pub hop_index: u32,
    pub recipient_count: usize,
}
