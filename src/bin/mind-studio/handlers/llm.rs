//! llama-server 모니터링 핸들러
//!
//! 서버 상태(health), 슬롯(slots), 메트릭(metrics)을 조회하고
//! 모델 정보를 포함한 통합 상태를 반환한다.

use axum::Json;
use axum::extract::State;
use npc_mind::ports::{LlamaHealth, LlamaMetrics, LlamaSlotInfo, LlmModelInfo};
use serde::Serialize;

use crate::state::AppState;
use super::AppError;

/// 통합 LLM 서버 상태 응답
///
/// 개별 항목 조회가 실패해도 나머지는 정상 반환한다 (부분 실패 허용).
#[derive(Serialize)]
pub struct LlmStatusResponse {
    pub health: Option<LlamaHealth>,
    pub model: Option<LlmModelInfo>,
    pub slots: Option<Vec<LlamaSlotInfo>>,
    pub metrics: Option<LlamaMetrics>,
}

/// GET /api/llm/status — 통합 서버 상태
pub async fn llm_status(
    State(state): State<AppState>,
) -> Result<Json<LlmStatusResponse>, AppError> {
    let monitor = state.llm_monitor.as_ref().ok_or_else(|| {
        AppError::NotImplemented("LLM 모니터가 설정되지 않았습니다 (chat feature 필요)".into())
    })?;

    let model = state.llm_info.as_ref().map(|info| info.get_model_info());

    // 개별 항목은 실패해도 None으로 처리
    let health = monitor.health().await.ok();
    let slots = monitor.slots().await.ok();
    let metrics = monitor.metrics().await.ok();

    Ok(Json(LlmStatusResponse {
        health,
        model,
        slots,
        metrics,
    }))
}

/// GET /api/llm/health — 서버 헬스 체크
pub async fn llm_health(
    State(state): State<AppState>,
) -> Result<Json<LlamaHealth>, AppError> {
    let monitor = state.llm_monitor.as_ref().ok_or_else(|| {
        AppError::NotImplemented("LLM 모니터가 설정되지 않았습니다".into())
    })?;

    monitor
        .health()
        .await
        .map(Json)
        .map_err(AppError::Internal)
}

/// GET /api/llm/slots — 슬롯 상태 조회
pub async fn llm_slots(
    State(state): State<AppState>,
) -> Result<Json<Vec<LlamaSlotInfo>>, AppError> {
    let monitor = state.llm_monitor.as_ref().ok_or_else(|| {
        AppError::NotImplemented("LLM 모니터가 설정되지 않았습니다".into())
    })?;

    monitor
        .slots()
        .await
        .map(Json)
        .map_err(AppError::Internal)
}

/// GET /api/llm/metrics — Prometheus 메트릭 조회
pub async fn llm_metrics(
    State(state): State<AppState>,
) -> Result<Json<LlamaMetrics>, AppError> {
    let monitor = state.llm_monitor.as_ref().ok_or_else(|| {
        AppError::NotImplemented("LLM 모니터가 설정되지 않았습니다".into())
    })?;

    monitor
        .metrics()
        .await
        .map(Json)
        .map_err(AppError::Internal)
}
