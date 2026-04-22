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
    // Topic 있을 때만 supersede 판정에 필요. dispatch 전에 기존 Canonical을 조회해
    // 둬야 "실제로 supersede가 일어났는지"를 SSE에 정확히 반영할 수 있다.
    let pre_canonical_existed = match req.topic.as_deref() {
        Some(topic) => state
            .memory_store
            .get_canonical_by_topic(topic)
            .map_err(|e| AppError::Internal(format!("get_canonical_by_topic 실패: {}", e)))?
            .is_some(),
        None => false,
    };

    let mut inner = state.inner.write().await;
    let output = domain_sync::dispatch_apply_world_event(&state, &mut *inner, req).await?;

    let world_event_seen = output
        .events
        .iter()
        .any(|e| matches!(e.payload, EventPayload::WorldEventOccurred { .. }));

    drop(inner);

    if world_event_seen {
        state.emit(StateEvent::MemoryCreated);
        if pre_canonical_existed {
            // 기존 Canonical이 있었고 새 WorldEvent가 성공 → WorldOverlayHandler가
            // 그 한 건을 supersede했음이 확정. false positive 없이 방출.
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
