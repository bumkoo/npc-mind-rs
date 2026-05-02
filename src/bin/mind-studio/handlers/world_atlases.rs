//! Phase 4 Worldbuilding REST 엔드포인트 (embed feature 활성 시에만).
//!
//! Atlas는 첫 관계 도메인. Phase 1·2·3 인스턴스 도메인(Group/Person/Place)과 동일한
//! `list_*`/`get_*`/`search_*` 3종 패턴을 미러링하되, **합성 핵심**인 references는
//! Atlas 도메인 객체에 그대로 노출되어 view 메서드(`places_in` 등)는 클라이언트가
//! 별도 호출로 합성한다 (Phase 4엔 서버 측 합성 엔드포인트 없음 — Phase 5+에서 검토).
//!
//! - `GET  /api/world/atlases`              — list with filters (kind/genre_tag)
//! - `GET  /api/world/atlases/{id}`         — single atlas (references·body_sections 전체)
//! - `GET  /api/world/atlases/search?q=...` — FTS5 trigram + LIKE fallback

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;

use crate::handlers::AppError;
use crate::state::AppState;

use npc_mind::domain::world::{Atlas, AtlasFilter, AtlasId};

#[derive(Deserialize, Default)]
pub struct ListQuery {
    pub kind: Option<String>,
    pub genre_tag: Option<String>,
}

pub async fn list_atlases(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Atlas>>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성 — NPC_MIND_WORLD_DB 환경변수 + world-load 실행 필요".into()))?;
    let filter = AtlasFilter {
        kind: q.kind,
        genre_tag: q.genre_tag,
    };
    let atlases = store
        .list_atlases(filter)
        .map_err(AppError::from)?;
    Ok(Json(atlases))
}

pub async fn get_atlas(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Atlas>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성".into()))?;
    let a = store
        .get_atlas(&AtlasId::new(id.clone()))
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("atlas '{id}' 없음")))?;
    Ok(Json(a))
}

#[derive(Deserialize, Default)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub top_k: Option<u32>,
}

pub async fn search_atlases(
    State(state): State<AppState>,
    Query(p): Query<SearchQuery>,
) -> Result<Json<Vec<Atlas>>, AppError> {
    let store = state
        .world_store
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("world index 미구성".into()))?;
    let q = p.q.unwrap_or_default();
    let top_k = p.top_k.unwrap_or(5);
    let hits = store
        .search_atlases(&q, top_k)
        .map_err(AppError::from)?;
    Ok(Json(hits))
}
