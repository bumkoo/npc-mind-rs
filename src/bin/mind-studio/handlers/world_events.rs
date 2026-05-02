//! Phase 5a Worldbuilding REST 엔드포인트 (embed feature 활성 시에만).
//!
//! Event는 두 번째 인스턴스 도메인. Phase 1·2·3·4 도메인과 동일한 `list_*`/`get_*`/
//! `search_*` 3종 패턴 미러링. **합성 핵심**인 `participants` (people·groups·places
//! 3 카테고리 외래키 셋)는 도메인 객체에 그대로 노출되며, 클라이언트가 별도 호출
//! (list_persons/list_groups/list_places)로 합성 view를 구성한다.
//!
//! - `GET  /api/world/events`              — list with filters (kind/category/participants_*/year_relative_*/genre_tag)
//! - `GET  /api/world/events/{id}`         — single event (participants·body_sections 전체)
//! - `GET  /api/world/events/search?q=...` — FTS5 trigram + LIKE fallback

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;

use crate::handlers::AppError;
use crate::state::AppState;

use npc_mind::domain::world::{Event, EventCategory, EventFilter, EventId};

#[derive(Deserialize, Default)]
pub struct ListQuery {
    pub kind: Option<String>,
    /// "historical" | "scheduled" | "legendary" — 알 수 없는 값은 400.
    pub category: Option<String>,
    pub participants_person: Option<String>,
    pub participants_group: Option<String>,
    pub participants_place: Option<String>,
    pub year_relative_min: Option<i32>,
    pub year_relative_max: Option<i32>,
    pub genre_tag: Option<String>,
}

pub async fn list_events(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Event>>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성 — NPC_MIND_WORLD_DB 환경변수 + world-load 실행 필요".into()))?;
    let category = match q.category.as_deref() {
        None => None,
        Some(s) => Some(EventCategory::from_str_loose(s).ok_or_else(|| {
            AppError::Internal(format!(
                "category '{s}' 알 수 없음 (허용: historical | scheduled | legendary)"
            ))
        })?),
    };
    let filter = EventFilter {
        kind: q.kind,
        category,
        participants_person: q.participants_person,
        participants_group: q.participants_group,
        participants_place: q.participants_place,
        year_relative_min: q.year_relative_min,
        year_relative_max: q.year_relative_max,
        genre_tag: q.genre_tag,
    };
    let events = store.list_events(filter).map_err(AppError::from)?;
    Ok(Json(events))
}

pub async fn get_event(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Event>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성".into()))?;
    let e = store
        .get_event(&EventId::new(id.clone()))
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("event '{id}' 없음")))?;
    Ok(Json(e))
}

#[derive(Deserialize, Default)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub top_k: Option<u32>,
}

pub async fn search_events(
    State(state): State<AppState>,
    Query(p): Query<SearchQuery>,
) -> Result<Json<Vec<Event>>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성".into()))?;
    let q = p.q.unwrap_or_default();
    let top_k = p.top_k.unwrap_or(5);
    let hits = store.search_events(&q, top_k).map_err(AppError::from)?;
    Ok(Json(hits))
}
