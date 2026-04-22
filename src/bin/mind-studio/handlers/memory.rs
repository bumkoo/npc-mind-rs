//! Step E1 — Memory REST 엔드포인트 (embed feature 활성 시에만 컴파일)
//!
//! 기존 `shared_dispatcher`에 붙은 `MemoryStore`를 직접 조회하거나
//! `Command::TellInformation`을 dispatch한다. 응답은 도메인 `MemoryEntry`를
//! 그대로 직렬화 — DTO wrapping은 생략해 포맷 시프트 리스크를 줄였다.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::domain_sync;
use crate::events::StateEvent;
use crate::handlers::AppError;
use crate::state::AppState;

use npc_mind::application::dto::TellInformationRequest;
use npc_mind::domain::event::EventKind;
use npc_mind::domain::memory::{MemoryEntry, MemoryLayer, MemorySource};
use npc_mind::ports::{MemoryQuery, MemoryScopeFilter};

#[derive(Deserialize, Default)]
pub struct SearchQuery {
    /// NPC 필터 (Personal + 참여 Relationship + World 접근 허용)
    pub npc: Option<String>,
    /// Topic 필터
    pub topic: Option<String>,
    /// `a` | `b` — Layer 필터
    pub layer: Option<String>,
    /// `experienced|witnessed|heard|rumor` 중 일부 (CSV)
    pub source: Option<String>,
    /// 결과 상한 (기본 20)
    pub limit: Option<usize>,
    /// **현재 미사용** — `SqliteMemoryStore::search`가 relevance=1.0 고정이라 semantic
    /// 검색 미지원. 필드는 API 스펙 호환을 위해 남겨두고 경고 주석.
    #[serde(default)]
    pub q: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct ByNpcQuery {
    pub limit: Option<usize>,
    pub layer: Option<String>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub entries: Vec<MemoryEntry>,
}

fn parse_layer(s: Option<&str>) -> Option<MemoryLayer> {
    match s? {
        "a" | "A" => Some(MemoryLayer::A),
        "b" | "B" => Some(MemoryLayer::B),
        _ => None,
    }
}

fn parse_sources(csv: Option<&str>) -> Option<Vec<MemorySource>> {
    let raw = csv?;
    let list: Vec<MemorySource> = raw
        .split(',')
        .filter_map(|s| match s.trim() {
            "experienced" => Some(MemorySource::Experienced),
            "witnessed" => Some(MemorySource::Witnessed),
            "heard" => Some(MemorySource::Heard),
            "rumor" => Some(MemorySource::Rumor),
            _ => None,
        })
        .collect();
    if list.is_empty() {
        None
    } else {
        Some(list)
    }
}

/// `GET /api/memory/search?npc=&topic=&layer=&source=&limit=`
pub async fn search(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, AppError> {
    let limit = q.limit.unwrap_or(20);
    let scope_filter = q
        .npc
        .as_ref()
        .map(|n| MemoryScopeFilter::NpcAllowed(n.clone()));
    let query = MemoryQuery {
        text: q.q.clone(),
        embedding: None,
        scope_filter,
        source_filter: parse_sources(q.source.as_deref()),
        layer_filter: parse_layer(q.layer.as_deref()),
        topic: q.topic.clone(),
        exclude_superseded: true,
        exclude_consolidated_source: false,
        min_retention: None,
        current_pad: None,
        limit,
    };
    let results = state
        .memory_store
        .search(query)
        .map_err(|e| AppError::Internal(format!("MemoryStore.search 실패: {}", e)))?;
    Ok(Json(SearchResponse {
        entries: results.into_iter().map(|r| r.entry).collect(),
    }))
}

/// `GET /api/memory/by-npc/:id?limit=&layer=`
pub async fn by_npc(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<ByNpcQuery>,
) -> Result<Json<SearchResponse>, AppError> {
    let limit = q.limit.unwrap_or(50);
    let query = MemoryQuery {
        text: None,
        embedding: None,
        scope_filter: Some(MemoryScopeFilter::NpcAllowed(id)),
        source_filter: None,
        layer_filter: parse_layer(q.layer.as_deref()),
        topic: None,
        exclude_superseded: true,
        exclude_consolidated_source: false,
        min_retention: None,
        current_pad: None,
        limit,
    };
    let results = state
        .memory_store
        .search(query)
        .map_err(|e| AppError::Internal(format!("MemoryStore.search 실패: {}", e)))?;
    Ok(Json(SearchResponse {
        entries: results.into_iter().map(|r| r.entry).collect(),
    }))
}

/// `GET /api/memory/by-topic/:topic` — supersede 이력 전체 포함.
pub async fn by_topic(
    State(state): State<AppState>,
    Path(topic): Path<String>,
) -> Result<Json<SearchResponse>, AppError> {
    let query = MemoryQuery {
        text: None,
        embedding: None,
        scope_filter: None,
        source_filter: None,
        layer_filter: None,
        topic: Some(topic),
        exclude_superseded: false,
        exclude_consolidated_source: false,
        min_retention: None,
        current_pad: None,
        limit: 50,
    };
    let results = state
        .memory_store
        .search(query)
        .map_err(|e| AppError::Internal(format!("MemoryStore.search 실패: {}", e)))?;
    Ok(Json(SearchResponse {
        entries: results.into_iter().map(|r| r.entry).collect(),
    }))
}

#[derive(Serialize)]
pub struct CanonicalResponse {
    pub entry: Option<MemoryEntry>,
}

/// `GET /api/memory/canonical/:topic` — Seeded + World scope 1건.
pub async fn canonical(
    State(state): State<AppState>,
    Path(topic): Path<String>,
) -> Result<Json<CanonicalResponse>, AppError> {
    let entry = state
        .memory_store
        .get_canonical_by_topic(&topic)
        .map_err(|e| AppError::Internal(format!("get_canonical_by_topic 실패: {}", e)))?;
    Ok(Json(CanonicalResponse { entry }))
}

/// `POST /api/memory/entries` — 수동 주입 (작가 도구).
///
/// 전달된 `MemoryEntry`를 그대로 `MemoryStore.index`에 저장한다. 임베딩은 생성하지
/// 않으므로 semantic 검색 대상은 되지 못함(메타 필터 검색은 가능). provenance는
/// 호출자 책임 — 일반적으로 `Seeded`로 세팅된 상태로 넘기는 것을 가정.
pub async fn create_entry(
    State(state): State<AppState>,
    Json(entry): Json<MemoryEntry>,
) -> Result<axum::http::StatusCode, AppError> {
    state
        .memory_store
        .index(entry, None)
        .map_err(|e| AppError::Internal(format!("MemoryStore.index 실패: {}", e)))?;
    state.emit(StateEvent::MemoryCreated);
    Ok(axum::http::StatusCode::CREATED)
}

/// `POST /api/memory/tell` — `Command::TellInformation` dispatch.
pub async fn tell(
    State(state): State<AppState>,
    Json(req): Json<TellInformationRequest>,
) -> Result<Json<TellResponse>, AppError> {
    let mut inner = state.inner.write().await;
    let output = domain_sync::dispatch_tell_information(&state, &mut inner, req).await?;

    let listeners_informed = output
        .events
        .iter()
        .filter(|e| matches!(e.kind(), EventKind::InformationTold))
        .count();

    drop(inner);
    if listeners_informed > 0 {
        state.emit(StateEvent::MemoryCreated);
    }

    Ok(Json(TellResponse { listeners_informed }))
}

#[derive(Serialize)]
pub struct TellResponse {
    pub listeners_informed: usize,
}

