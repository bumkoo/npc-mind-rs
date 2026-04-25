//! Projection 기반 Read Side 엔드포인트 (`/api/projection/*`).
//!
//! `StateInner` 직접 조회 경로와 **병렬로** 존재하며, CQRS Read Model이 실제
//! 외부 소비자를 가진다는 것을 입증하는 경로이다.
//!
//! 모든 핸들러는 `AppState`가 보관한 Projection `Arc<std::sync::Mutex<T>>`에서
//! read-only로 lock을 획득하고, 다른 thread의 poison은 `AppError::Internal`로
//! 매핑한다 (`EmotionProjectionHandler::handle`의 Infrastructure 패턴과 동일).
//!
//! ## 범위 제한
//!
//! 본 엔드포인트는 **`shared_dispatcher`의 Projection만 반영한다**. `/api/v2/*`
//! Director 경로는 별도 `CommandDispatcher` 인스턴스를 쓰며 내부적으로 독립된
//! Projection Arc를 가지므로 여기서 조회되지 않는다 (director_v2 노출은 별도
//! 태스크, task 명세 §10 참조).

use axum::{
    Json,
    extract::{Path, State},
};
use npc_mind::DomainEvent;
use serde::Serialize;

use crate::handlers::AppError;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Emotion
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct EmotionView {
    pub npc_id: String,
    pub mood: Option<f32>,
    pub dominant: Option<(String, f32)>,
    pub snapshot: Option<Vec<(String, f32)>>,
}

pub async fn get_emotion(
    State(state): State<AppState>,
    Path(npc_id): Path<String>,
) -> Result<Json<EmotionView>, AppError> {
    let proj = state
        .emotion_projection
        .lock()
        .map_err(|_| AppError::Internal("emotion projection mutex poisoned".into()))?;

    Ok(Json(EmotionView {
        mood: proj.get_mood(&npc_id),
        dominant: proj.get_dominant(&npc_id).cloned(),
        snapshot: proj.get_snapshot(&npc_id).cloned(),
        npc_id,
    }))
}

// ---------------------------------------------------------------------------
// Relationship
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct RelationshipView {
    pub owner: String,
    pub target: String,
    pub closeness: Option<f32>,
    pub trust: Option<f32>,
    pub power: Option<f32>,
}

pub async fn get_relationship(
    State(state): State<AppState>,
    Path((owner, target)): Path<(String, String)>,
) -> Result<Json<RelationshipView>, AppError> {
    let proj = state
        .relationship_projection
        .lock()
        .map_err(|_| AppError::Internal("relationship projection mutex poisoned".into()))?;

    let values = proj.get_values(&owner, &target);
    Ok(Json(RelationshipView {
        owner,
        target,
        closeness: values.map(|v| v.0),
        trust: values.map(|v| v.1),
        power: values.map(|v| v.2),
    }))
}

// ---------------------------------------------------------------------------
// Scene
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct SceneView {
    pub is_active: bool,
    pub active_focus_id: Option<String>,
}

pub async fn get_scene(State(state): State<AppState>) -> Result<Json<SceneView>, AppError> {
    let proj = state
        .scene_projection
        .lock()
        .map_err(|_| AppError::Internal("scene projection mutex poisoned".into()))?;

    Ok(Json(SceneView {
        is_active: proj.is_active(),
        active_focus_id: proj.active_focus_id().map(String::from),
    }))
}

// ---------------------------------------------------------------------------
// Trace — correlation_id로 묶인 이벤트 사슬 조회
// ---------------------------------------------------------------------------

/// 한 `dispatch_v2` 호출이 만든 모든 후속 이벤트 묶음을 반환한다.
///
/// 같은 cid 내에서는 EventStore에 추가된 순서를 그대로 보존한다 (시간 흐름과
/// cascade depth가 함께 반영됨). 명시적 timestamp 정렬이 필요하면 호출자가 처리.
///
/// **범위**: `shared_dispatcher`의 EventStore만 조회한다. `/api/v2/*` Director
/// 경로는 별도 dispatcher 인스턴스를 쓰며 그쪽 EventStore의 trace 노출은 별도
/// 태스크 (task 명세 §10).
#[derive(Serialize)]
pub struct TraceView {
    pub correlation_id: u64,
    pub event_count: usize,
    pub events: Vec<DomainEvent>,
}

pub async fn get_trace(
    State(state): State<AppState>,
    Path(correlation_id): Path<u64>,
) -> Result<Json<TraceView>, AppError> {
    let store = state.shared_dispatcher.event_store();
    let events = store.get_events_by_correlation(correlation_id);

    Ok(Json(TraceView {
        correlation_id,
        event_count: events.len(),
        events,
    }))
}
