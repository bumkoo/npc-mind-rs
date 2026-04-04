use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use npc_mind::application::dto::*;
use serde::{Deserialize, Serialize};
use crate::state::*;
use crate::studio_service::{StudioService, ScenarioInfo, SaveDirInfo};
use crate::repository::{ReadOnlyAppStateRepo};
use super::AppError;

/// POST /api/appraise — 감정 평가 실행
pub async fn appraise(
    State(state): State<AppState>,
    Json(req): Json<AppraiseRequest>,
) -> Result<Json<AppraiseResponse>, AppError> {
    let response = StudioService::perform_appraise(&state, req).await?;
    Ok(Json(response))
}

/// POST /api/stimulus — PAD 자극 적용
pub async fn stimulus(
    State(state): State<AppState>,
    Json(req): Json<StimulusRequest>,
) -> Result<Json<StimulusResponse>, AppError> {
    let response = StudioService::perform_stimulus(&state, req).await?;
    Ok(Json(response))
}

/// POST /api/after-dialogue — 대화 종료 → 관계 갱신
pub async fn after_dialogue(
    State(state): State<AppState>,
    Json(req): Json<AfterDialogueRequest>,
) -> Result<Json<AfterDialogueResponse>, AppError> {
    let response = StudioService::perform_after_dialogue(&state, req).await?;
    Ok(Json(response))
}

/// GET /api/scenarios — 시나리오 목록
pub async fn list_scenarios() -> Json<Vec<ScenarioInfo>> {
    Json(StudioService::list_scenarios())
}

/// GET /api/scenario-meta — 현재 시나리오 정보
pub async fn get_scenario_meta(State(state): State<AppState>) -> Json<ScenarioMeta> {
    let inner = state.inner.read().await;
    Json(inner.scenario.clone())
}

/// GET /api/scene-info — Scene 상태 조회
pub async fn get_scene_info(State(state): State<AppState>) -> Json<SceneInfoResult> {
    let inner = state.inner.read().await;
    let repo = ReadOnlyAppStateRepo { inner: &*inner };
    let service = npc_mind::application::mind_service::MindService::new(repo);
    Json(service.scene_info())
}

/// POST /api/scene — Scene 시작
pub async fn scene(
    State(state): State<AppState>,
    Json(req): Json<SceneRequest>,
) -> Result<Json<SceneResponse>, AppError> {
    let mut inner = state.inner.write().await;
    let collector = state.collector.clone();
    let mut service = npc_mind::application::mind_service::MindService::new(crate::repository::AppStateRepository { inner: &mut *inner });
    let result = service.start_scene(req.clone(), || { collector.take_entries(); }, || collector.take_entries())?;
    let response = result.format(&*state.formatter);
    if response.initial_appraise.is_some() {
        let turn_num = inner.turn_history.len() + 1;
        inner.turn_history.push(TurnRecord { label: format!("Turn {}: scene/appraise [{}] ({}→{})", turn_num, response.active_focus_id.as_deref().unwrap_or("?"), req.npc_id, req.partner_id), action: "scene".into(), request: serde_json::to_value(&req).unwrap_or_default(), response: serde_json::to_value(&response).unwrap_or_default(), llm_model: None });
    }
    Ok(Json(response))
}

/// GET /api/history — 기록 조회
pub async fn get_history(State(state): State<AppState>) -> Json<Vec<TurnRecord>> {
    let inner = state.inner.read().await;
    Json(inner.turn_history.clone())
}

/// GET /api/situation
pub async fn get_situation(State(state): State<AppState>) -> Json<serde_json::Value> {
    let inner = state.inner.read().await;
    Json(inner.current_situation.clone().unwrap_or(serde_json::Value::Null))
}

/// PUT /api/situation
pub async fn put_situation(State(state): State<AppState>, Json(body): Json<serde_json::Value>) -> StatusCode {
    let mut inner = state.inner.write().await;
    inner.current_situation = Some(body);
    inner.scenario_modified = true;
    StatusCode::OK
}

/// POST /api/guide — 가이드 재생성
pub async fn guide(
    State(state): State<AppState>,
    Json(mut req): Json<GuideRequest>,
) -> Result<Json<GuideResponse>, AppError> {
    let mut inner = state.inner.write().await;
    if req.situation_description.is_none() {
        if let Some(ref sit) = inner.current_situation {
            req.situation_description = sit.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
        }
    }
    let service = npc_mind::application::mind_service::MindService::new(crate::repository::AppStateRepository { inner: &mut *inner });
    let result = service.generate_guide(req)?;
    Ok(Json(result.format(&*state.formatter)))
}

/// POST /api/save
pub async fn save_state(State(state): State<AppState>, Json(req): Json<super::SaveRequest>) -> Result<Json<super::SaveResponse>, AppError> {
    let mut inner = state.inner.write().await;
    let save_path = req.path.clone();
    if save_path.is_empty() { return Err(AppError::Internal("저장 경로가 비어있습니다".into())); }
    let as_scenario = match req.save_type.as_deref() { Some("scenario") => true, Some("result") => false, _ => inner.turn_history.is_empty() };
    inner.save_to_file(std::path::Path::new(&save_path), as_scenario).map_err(|e| AppError::Internal(e))?;
    if as_scenario { inner.scenario_modified = false; inner.loaded_path = Some(save_path.clone()); }
    Ok(Json(super::SaveResponse { path: save_path }))
}

/// GET /api/save-dir
pub async fn save_dir(State(state): State<AppState>) -> Result<Json<SaveDirInfo>, AppError> {
    let info = StudioService::get_save_dir(&state).await?;
    Ok(Json(info))
}

/// POST /api/load
pub async fn load_state(State(state): State<AppState>, Json(req): Json<super::SaveRequest>) -> Result<StatusCode, AppError> {
    let mut loaded = StateInner::load_from_file(std::path::Path::new(&req.path)).map_err(|e| AppError::Internal(e))?;
    loaded.turn_history.clear();
    loaded.loaded_path = Some(req.path.clone());
    if let Some(ref scene_val) = loaded.scene {
        if let Ok(scene_req) = serde_json::from_value::<SceneRequest>(scene_val.clone()) {
            StudioService::load_scene_into_state(&mut loaded, &scene_req);
        }
    }
    let mut inner = state.inner.write().await;
    *inner = loaded;
    Ok(StatusCode::OK)
}

/// POST /api/load-result
pub async fn load_result(State(state): State<AppState>, Json(req): Json<super::SaveRequest>) -> Result<Json<super::LoadResultResponse>, AppError> {
    let mut loaded = StateInner::load_from_file(std::path::Path::new(&req.path)).map_err(|e| AppError::Internal(e))?;
    loaded.loaded_path = Some(req.path.clone());
    if let Some(ref scene_val) = loaded.scene {
        if let Ok(scene_req) = serde_json::from_value::<SceneRequest>(scene_val.clone()) {
            StudioService::load_scene_into_state(&mut loaded, &scene_req);
        }
    }
    let history = loaded.turn_history.clone();
    let mut inner = state.inner.write().await;
    *inner = loaded;
    Ok(Json(super::LoadResultResponse { turn_history: history }))
}

/// GET /api/test-report
pub async fn get_test_report(State(state): State<AppState>) -> Json<serde_json::Value> {
    let inner = state.inner.read().await;
    Json(serde_json::json!({ "content": inner.test_report }))
}

/// PUT /api/test-report
pub async fn put_test_report(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> StatusCode {
    let mut inner = state.inner.write().await;
    if let Some(content) = body.get("content").and_then(|v| v.as_str()) {
        inner.test_report = content.to_string();
        inner.scenario_modified = true;
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    }
}

#[cfg(feature = "embed")]
#[derive(Deserialize)]
pub struct AnalyzeUtteranceRequest {
    pub utterance: String,
}

#[cfg(feature = "embed")]
#[derive(Serialize)]
pub struct AnalyzeUtteranceResponse {
    pub pleasure: f32,
    pub arousal: f32,
    pub dominance: f32,
}

#[cfg(feature = "embed")]
pub async fn analyze_utterance(
    State(state): State<AppState>,
    Json(req): Json<AnalyzeUtteranceRequest>,
) -> Result<Json<AnalyzeUtteranceResponse>, AppError> {
    let analyzer = state
        .analyzer
        .as_ref()
        .ok_or_else(|| AppError::NotImplemented("embed feature가 비활성 상태입니다".into()))?;

    let mut analyzer = analyzer.lock().await;
    let pad = analyzer
        .analyze(&req.utterance)
        .map_err(|e| AppError::Internal(format!("PAD 분석 실패: {e:?}")))?;

    Ok(Json(AnalyzeUtteranceResponse {
        pleasure: pad.pleasure,
        arousal: pad.arousal,
        dominance: pad.dominance,
    }))
}

#[cfg(not(feature = "embed"))]
pub async fn analyze_utterance(
    State(_state): State<AppState>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::NotImplemented("embed feature가 비활성 상태입니다".into()))
}
