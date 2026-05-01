//! Phase 2 Worldbuilding REST 엔드포인트 — Person 조회 + 런타임 sync (embed feature 활성 시).
//!
//! - `GET  /api/world/persons`              — list with filters (kind/status/affiliation/genre_tag)
//! - `GET  /api/world/persons/{id}`         — single person
//! - `GET  /api/world/persons/search?q=...` — FTS5 trigram
//! - `POST /api/world/persons/sync`         — world_store → mind repo 일괄 재동기화
//!                                            (Phase 2 follow-up: 런타임 작가 워크플로우)

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};

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

#[derive(Serialize)]
pub struct SyncResponse {
    /// 본 호출에서 inner.npcs로 등록(또는 덮어쓰기)된 Person 수.
    pub synced: usize,
}

/// 런타임 sync — world_store의 active/player Person을 inner.npcs로 일괄 재동기화.
///
/// **사용 시나리오**: 작가가 외부에서 `world-load --reload`로 SQLite를 갱신한 뒤
/// mind-studio 재시작 없이 변경된 HEXACO를 즉시 시연하고 싶을 때.
///
/// **보존 보장** (Phase 2 본문 §3.5):
/// - emotion_state · 관계 · scene · memory: 보존 (`inner.emotions` / `inner.relationships` /
///   `inner.scene_*`은 별도 필드. `sync_world_persons_into_repo` → `rebuild_repo_from_inner`이
///   해당 필드를 그대로 다시 적용).
/// - NpcProfile (description · HEXACO 24 facet 등): **완전 덮어쓰기**. UI에서 수동 편집한
///   값이 있으면 사라진다 — `state.rs::sync_world_persons_into_repo` docstring 참조.
pub async fn sync_persons(
    State(state): State<AppState>,
) -> Result<Json<SyncResponse>, AppError> {
    if state.world_store.is_none() {
        return Err(AppError::NotImplemented(
            "world index 미구성 — NPC_MIND_WORLD_DB + world-load 실행 필요".into(),
        ));
    }
    let synced = state
        .sync_world_persons_into_repo()
        .await
        .map_err(AppError::from)?;
    Ok(Json(SyncResponse { synced }))
}
