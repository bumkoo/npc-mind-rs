//! Phase 3 Worldbuilding REST 엔드포인트 (embed feature 활성 시에만).
//!
//! - `GET  /api/world/places`              — list with filters (layer/kind/parent_place/genre_tag)
//! - `GET  /api/world/places/{id}`         — single place
//! - `GET  /api/world/places/search?q=...` — FTS5 trigram

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;

use crate::handlers::AppError;
use crate::state::AppState;

use npc_mind::domain::world::{Place, PlaceFilter, PlaceId, PlaceLayer};

#[derive(Deserialize, Default)]
pub struct ListQuery {
    pub layer: Option<String>,
    pub kind: Option<String>,
    pub parent_place: Option<String>,
    pub genre_tag: Option<String>,
}

pub async fn list_places(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Place>>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성 — NPC_MIND_WORLD_DB 환경변수 + world-load 실행 필요".into()))?;
    let filter = PlaceFilter {
        layer: q.layer.as_deref().and_then(PlaceLayer::from_str_loose),
        kind: q.kind,
        parent_place: q.parent_place.map(PlaceId::new),
        genre_tag: q.genre_tag,
    };
    let places = store
        .list_places(filter)
        .map_err(AppError::from)?;
    Ok(Json(places))
}

pub async fn get_place(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Place>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성".into()))?;
    let p = store
        .get_place(&PlaceId::new(id.clone()))
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("place '{id}' 없음")))?;
    Ok(Json(p))
}

#[derive(Deserialize, Default)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub top_k: Option<u32>,
}

pub async fn search_places(
    State(state): State<AppState>,
    Query(p): Query<SearchQuery>,
) -> Result<Json<Vec<Place>>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성".into()))?;
    let q = p.q.unwrap_or_default();
    let top_k = p.top_k.unwrap_or(5);
    let hits = store
        .search_places(&q, top_k)
        .map_err(AppError::from)?;
    Ok(Json(hits))
}
