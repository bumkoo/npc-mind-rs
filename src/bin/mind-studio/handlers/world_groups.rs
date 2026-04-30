//! Phase 1 Worldbuilding REST 엔드포인트 (embed feature 활성 시에만).
//!
//! - `GET  /api/world/groups`              — list with filters (kind/status/parent_group/alignment/genre_tag)
//! - `GET  /api/world/groups/{id}`         — single group
//! - `GET  /api/world/groups/search?q=...` — FTS5 trigram

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;

use crate::handlers::AppError;
use crate::state::AppState;

use npc_mind::domain::world::{Group, GroupFilter, GroupId, GroupStatus};

#[derive(Deserialize, Default)]
pub struct ListQuery {
    pub kind: Option<String>,
    pub status: Option<String>,
    pub parent_group: Option<String>,
    pub alignment: Option<String>,
    pub genre_tag: Option<String>,
}

pub async fn list_groups(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Group>>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성 — NPC_MIND_WORLD_DB 환경변수 + world-load 실행 필요".into()))?;
    let filter = GroupFilter {
        kind: q.kind,
        status: q.status.as_deref().and_then(GroupStatus::from_str_loose),
        parent_group: q.parent_group.map(GroupId::new),
        genre_tag: q.genre_tag,
        alignment: q.alignment,
    };
    let groups = store
        .list_groups(filter)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(groups))
}

pub async fn get_group(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Group>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성".into()))?;
    let g = store
        .get_group(&GroupId::new(id.clone()))
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("group '{id}' 없음")))?;
    Ok(Json(g))
}

#[derive(Deserialize, Default)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub top_k: Option<u32>,
}

pub async fn search_groups(
    State(state): State<AppState>,
    Query(p): Query<SearchQuery>,
) -> Result<Json<Vec<Group>>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성".into()))?;
    let q = p.q.unwrap_or_default();
    let top_k = p.top_k.unwrap_or(5);
    let hits = store
        .search_groups(&q, top_k)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(hits))
}
