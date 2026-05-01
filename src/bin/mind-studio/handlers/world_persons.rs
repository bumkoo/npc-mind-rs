//! Phase 2 Worldbuilding REST 엔드포인트 — Person 조회 (embed feature 활성 시에만).
//!
//! - `GET /api/world/persons`              — list with filters (kind/status/affiliation/genre_tag)
//! - `GET /api/world/persons/{id}`         — single person
//! - `GET /api/world/persons/search?q=...` — FTS5 trigram

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;

use crate::handlers::AppError;
use crate::state::AppState;

use npc_mind::domain::world::{GroupId, Person, PersonFilter, PersonId, PersonStatus};

#[derive(Deserialize, Default)]
pub struct ListQuery {
    pub kind: Option<String>,
    pub status: Option<String>,
    pub affiliation: Option<String>,
    pub genre_tag: Option<String>,
}

pub async fn list_persons(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Person>>, AppError> {
    let store = state.world_store.as_ref().ok_or_else(|| {
        AppError::NotImplemented(
            "world index 미구성 — NPC_MIND_WORLD_DB 환경변수 + world-load 실행 필요".into(),
        )
    })?;
    let filter = PersonFilter {
        kind: q.kind,
        status: q.status.as_deref().and_then(PersonStatus::from_str_loose),
        affiliation: q.affiliation.map(GroupId::new),
        genre_tag: q.genre_tag,
    };
    let persons = store.list_persons(filter).map_err(AppError::from)?;
    Ok(Json(persons))
}

pub async fn get_person(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Person>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성".into()))?;
    let p = store
        .get_person(&PersonId::new(id.clone()))
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("person '{id}' 없음")))?;
    Ok(Json(p))
}

#[derive(Deserialize, Default)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub top_k: Option<u32>,
}

pub async fn search_persons(
    State(state): State<AppState>,
    Query(p): Query<SearchQuery>,
) -> Result<Json<Vec<Person>>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성".into()))?;
    let q = p.q.unwrap_or_default();
    let top_k = p.top_k.unwrap_or(5);
    let hits = store.search_persons(&q, top_k).map_err(AppError::from)?;
    Ok(Json(hits))
}
