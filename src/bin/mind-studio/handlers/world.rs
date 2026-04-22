//! Step E1 — World 오버레이 REST 엔드포인트 (embed feature 활성 시에만 컴파일)
//!
//! `Command::ApplyWorldEvent`만 노출한다. 실제 Canonical `MemoryEntry` 생성은
//! Inline `WorldOverlayHandler`가 `shared_dispatcher`에 부착돼 있을 때만 일어나며,
//! 이는 `embed` feature에서 자동 배선됨.

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::domain_sync;
use crate::events::StateEvent;
use crate::handlers::AppError;
use crate::state::AppState;

use npc_mind::application::dto::ApplyWorldEventRequest;
use npc_mind::domain::event::EventPayload;

/// `POST /api/world/apply-event`
pub async fn apply_event(
    State(state): State<AppState>,
    Json(req): Json<ApplyWorldEventRequest>,
) -> Result<Json<ApplyWorldEventResponse>, AppError> {
    let had_topic = req.topic.is_some();

    let mut inner = state.inner.write().await;
    let output = domain_sync::dispatch_apply_world_event(&state, &mut inner, req).await?;

    let world_event_seen = output
        .events
        .iter()
        .any(|e| matches!(e.payload, EventPayload::WorldEventOccurred { .. }));

    drop(inner);

    if world_event_seen {
        state.emit(StateEvent::MemoryCreated);
        if had_topic {
            // Canonical 한 건 supersede가 일반적 (WorldOverlayHandler 규칙).
            // 실제 supersede 여부는 기존 Canonical 존재에 달려 있어 여기선
            // best-effort로 SSE만 트리거.
            state.emit(StateEvent::MemorySuperseded);
        }
    }

    Ok(Json(ApplyWorldEventResponse {
        applied: world_event_seen,
    }))
}

#[derive(Serialize)]
pub struct ApplyWorldEventResponse {
    pub applied: bool,
}
