//! Phase 5b Worldbuilding REST 엔드포인트 (embed feature 활성 시에만).
//!
//! Era는 세 번째 인스턴스 도메인. Phase 1·2·3·4·5a 도메인과 동일한 `list_*`/`get_*`/
//! `search_*` 3종 패턴 미러링. **합성 핵심**인 `key_events` (Era → Event 단방향
//! 외래키)는 도메인 객체에 그대로 노출되며, 클라이언트가 별도 호출
//! (list_events/get_event)로 합성 view를 구성한다.
//!
//! - `GET  /api/world/eras`              — list with filters (kind/contains_year/genre_tag)
//! - `GET  /api/world/eras/{id}`         — single era (key_events·body_sections 전체)
//! - `GET  /api/world/eras/search?q=...` — FTS5 trigram + LIKE fallback

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;

use crate::handlers::AppError;
use crate::state::AppState;

use npc_mind::domain::world::{Era, EraFilter, EraId};

#[derive(Deserialize, Default)]
pub struct ListQuery {
    pub kind: Option<String>,
    /// 본 era가 포함하는 year_relative (boundary 정책 §3.3 — start inclusive · end exclusive).
    pub contains_year: Option<i32>,
    pub genre_tag: Option<String>,
}

pub async fn list_eras(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Era>>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성 — NPC_MIND_WORLD_DB 환경변수 + world-load 실행 필요".into()))?;
    let filter = EraFilter {
        kind: q.kind,
        contains_year: q.contains_year,
        genre_tag: q.genre_tag,
    };
    let eras = store.list_eras(filter).map_err(AppError::from)?;
    Ok(Json(eras))
}

pub async fn get_era(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Era>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성".into()))?;
    let e = store
        .get_era(&EraId::new(id.clone()))
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("era '{id}' 없음")))?;
    Ok(Json(e))
}

#[derive(Deserialize, Default)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub top_k: Option<u32>,
}

pub async fn search_eras(
    State(state): State<AppState>,
    Query(p): Query<SearchQuery>,
) -> Result<Json<Vec<Era>>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성".into()))?;
    let q = p.q.unwrap_or_default();
    let top_k = p.top_k.unwrap_or(5);
    let hits = store.search_eras(&q, top_k).map_err(AppError::from)?;
    Ok(Json(hits))
}
