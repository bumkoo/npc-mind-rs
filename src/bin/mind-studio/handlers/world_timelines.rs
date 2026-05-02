//! Phase 5b 체크포인트 2 Worldbuilding REST 엔드포인트 (embed feature 활성 시).
//!
//! Timeline은 두 번째 관계 도메인. references=Vec<EraId>는 도메인 객체에 그대로 노출되며
//! view 메서드 (eras_in/events_in/events_during/causal_chain) 합성은 클라이언트가 별도
//! 호출 (list_eras/get_era + list_events/get_event)로 구성. Phase 5b 체크포인트 2에서
//! 서버 측 합성 엔드포인트는 Phase 6+로 미룸.
//!
//! - `GET  /api/world/timelines`              — list with filters (kind/references_era/genre_tag)
//! - `GET  /api/world/timelines/{id}`         — single timeline
//! - `GET  /api/world/timelines/search?q=...` — FTS5 trigram + LIKE fallback

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;

use crate::handlers::AppError;
use crate::state::AppState;

use npc_mind::domain::world::{EraId, Timeline, TimelineFilter, TimelineId};

#[derive(Deserialize, Default)]
pub struct ListQuery {
    pub kind: Option<String>,
    /// 특정 era를 references에 포함하는 timeline만.
    pub references_era: Option<String>,
    pub genre_tag: Option<String>,
}

pub async fn list_timelines(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Timeline>>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성 — NPC_MIND_WORLD_DB 환경변수 + world-load 실행 필요".into()))?;
    let filter = TimelineFilter {
        kind: q.kind,
        references_era: q.references_era.map(EraId::new),
        genre_tag: q.genre_tag,
    };
    let timelines = store.list_timelines(filter).map_err(AppError::from)?;
    Ok(Json(timelines))
}

pub async fn get_timeline(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Timeline>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성".into()))?;
    let t = store
        .get_timeline(&TimelineId::new(id.clone()))
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("timeline '{id}' 없음")))?;
    Ok(Json(t))
}

#[derive(Deserialize, Default)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub top_k: Option<u32>,
}

pub async fn search_timelines(
    State(state): State<AppState>,
    Query(p): Query<SearchQuery>,
) -> Result<Json<Vec<Timeline>>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성".into()))?;
    let q = p.q.unwrap_or_default();
    let top_k = p.top_k.unwrap_or(5);
    let hits = store.search_timelines(&q, top_k).map_err(AppError::from)?;
    Ok(Json(hits))
}
