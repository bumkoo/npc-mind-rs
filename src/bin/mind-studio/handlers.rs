//! API 핸들러 — CRUD + 파이프라인 인터페이스

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use npc_mind::application::dto::*;
use npc_mind::application::mind_service::{MindServiceError};
use npc_mind::ports::UtteranceAnalyzer;

use crate::state::*;
use crate::studio_service::{StudioService, ReadOnlyAppStateRepo, ScenarioInfo, SaveDirInfo};

// ---------------------------------------------------------------------------
// WebUI 전용 에러 타입
// ---------------------------------------------------------------------------
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Service(#[from] MindServiceError),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

#[cfg(feature = "chat")]
impl From<npc_mind::ports::ConversationError> for AppError {
    fn from(e: npc_mind::ports::ConversationError) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Service(ref e) => match e {
                MindServiceError::NpcNotFound(_) | MindServiceError::RelationshipNotFound(_, _) => {
                    (StatusCode::NOT_FOUND, e.to_string())
                }
                MindServiceError::InvalidSituation(_) | MindServiceError::EmotionStateNotFound => {
                    (StatusCode::BAD_REQUEST, e.to_string())
                }
                _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            },
            AppError::NotFound(ref msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::NotImplemented(ref msg) => (StatusCode::NOT_IMPLEMENTED, msg.clone()),
            AppError::Internal(ref msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = Json(serde_json::json!({
            "error": message
        }));

        (status, body).into_response()
    }
}

// ---------------------------------------------------------------------------
// NPC CRUD
// ---------------------------------------------------------------------------

/// GET /api/npcs — 전체 NPC 목록
pub async fn list_npcs(State(state): State<AppState>) -> Json<Vec<NpcProfile>> {
    let inner = state.inner.read().await;
    let mut npcs: Vec<NpcProfile> = inner.npcs.values().cloned().collect();
    npcs.sort_by(|a, b| a.id.cmp(&b.id));
    Json(npcs)
}

/// POST /api/npcs — NPC 생성/업데이트
pub async fn upsert_npc(State(state): State<AppState>, Json(npc): Json<NpcProfile>) -> StatusCode {
    let mut inner = state.inner.write().await;
    inner.npcs.insert(npc.id.clone(), npc);
    inner.scenario_modified = true;
    StatusCode::OK
}

/// DELETE /api/npcs/:id
pub async fn delete_npc(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> StatusCode {
    let mut inner = state.inner.write().await;
    inner.npcs.remove(&id);
    inner.scenario_modified = true;
    StatusCode::OK
}

// ---------------------------------------------------------------------------
// Relationship CRUD
// ---------------------------------------------------------------------------

/// GET /api/relationships
pub async fn list_relationships(State(state): State<AppState>) -> Json<Vec<RelationshipData>> {
    let inner = state.inner.read().await;
    let mut rels: Vec<RelationshipData> = inner.relationships.values().cloned().collect();
    rels.sort_by(|a, b| a.key().cmp(&b.key()));
    Json(rels)
}

/// POST /api/relationships
pub async fn upsert_relationship(
    State(state): State<AppState>,
    Json(rel): Json<RelationshipData>,
) -> StatusCode {
    let mut inner = state.inner.write().await;
    let key = rel.key();
    inner.relationships.insert(key, rel);
    inner.scenario_modified = true;
    StatusCode::OK
}

/// DELETE /api/relationships/:owner_id/:target_id
pub async fn delete_relationship(
    State(state): State<AppState>,
    axum::extract::Path((owner, target)): axum::extract::Path<(String, String)>,
) -> StatusCode {
    let mut inner = state.inner.write().await;
    let key = format!("{owner}:{target}");
    inner.relationships.remove(&key);
    inner.scenario_modified = true;
    StatusCode::OK
}

// ---------------------------------------------------------------------------
// Object CRUD
// ---------------------------------------------------------------------------

/// GET /api/objects
pub async fn list_objects(State(state): State<AppState>) -> Json<Vec<ObjectEntry>> {
    let inner = state.inner.read().await;
    let mut objs: Vec<ObjectEntry> = inner.objects.values().cloned().collect();
    objs.sort_by(|a, b| a.id.cmp(&b.id));
    Json(objs)
}

/// POST /api/objects
pub async fn upsert_object(
    State(state): State<AppState>,
    Json(obj): Json<ObjectEntry>,
) -> StatusCode {
    let mut inner = state.inner.write().await;
    inner.objects.insert(obj.id.clone(), obj);
    inner.scenario_modified = true;
    StatusCode::OK
}

/// DELETE /api/objects/:id
pub async fn delete_object(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> StatusCode {
    let mut inner = state.inner.write().await;
    inner.objects.remove(&id);
    inner.scenario_modified = true;
    StatusCode::OK
}

// ---------------------------------------------------------------------------
// 파이프라인: 감정 평가 및 자극
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// 가이드 재생성
// ---------------------------------------------------------------------------

/// POST /api/guide — 현재 감정 상태에서 가이드 재생성
pub async fn guide(
    State(state): State<AppState>,
    Json(mut req): Json<GuideRequest>,
) -> Result<Json<GuideResponse>, AppError> {
    let mut inner = state.inner.write().await;

    // 저장된 상황 설명을 fallback으로 사용
    if req.situation_description.is_none() {
        if let Some(ref sit) = inner.current_situation {
            req.situation_description = sit
                .get("description")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
        }
    }

    let mut service = npc_mind::application::mind_service::MindService::new(crate::studio_service::AppStateRepository { inner: &mut *inner });
    let result = service.generate_guide(req)?;
    Ok(Json(result.format(&*state.formatter)))
}

// ---------------------------------------------------------------------------
// 대사 → PAD 분석
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct AnalyzeUtteranceRequest {
    pub utterance: String,
}

#[derive(Serialize)]
pub struct AnalyzeUtteranceResponse {
    pub pleasure: f32,
    pub arousal: f32,
    pub dominance: f32,
}

/// POST /api/analyze-utterance — 대사 텍스트를 PAD 값으로 변환
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

// ---------------------------------------------------------------------------
// 시나리오 & Scene 조회
// ---------------------------------------------------------------------------

/// GET /api/scenario-meta — 현재 로드된 시나리오 메타 정보
pub async fn get_scenario_meta(State(state): State<AppState>) -> Json<ScenarioMeta> {
    let inner = state.inner.read().await;
    Json(inner.scenario.clone())
}

/// GET /api/scene-info — 현재 Scene Focus 상태 조회
pub async fn get_scene_info(State(state): State<AppState>) -> Json<SceneInfoResult> {
    let inner = state.inner.read().await;
    let repo = ReadOnlyAppStateRepo { inner: &*inner };
    let service = npc_mind::application::mind_service::MindService::new(repo);
    Json(service.scene_info())
}

/// GET /api/history — 턴별 기록 조회
pub async fn get_history(State(state): State<AppState>) -> Json<Vec<TurnRecord>> {
    let inner = state.inner.read().await;
    Json(inner.turn_history.clone())
}

/// GET /api/situation — 현재 상황 설정 패널 상태 조회
pub async fn get_situation(State(state): State<AppState>) -> Json<serde_json::Value> {
    let inner = state.inner.read().await;
    Json(
        inner
            .current_situation
            .clone()
            .unwrap_or(serde_json::Value::Null),
    )
}

/// PUT /api/situation — 상황 설정 패널 상태 저장
pub async fn put_situation(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> StatusCode {
    let mut inner = state.inner.write().await;
    inner.current_situation = Some(body);
    inner.scenario_modified = true;
    StatusCode::OK
}

// ---------------------------------------------------------------------------
// 저장/로드
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct SaveRequest {
    pub path: String,
    #[serde(default)]
    pub save_type: Option<String>,
}

#[derive(Serialize)]
pub struct SaveResponse {
    pub path: String,
}

pub async fn save_state(
    State(state): State<AppState>,
    Json(req): Json<SaveRequest>,
) -> Result<Json<SaveResponse>, AppError> {
    let mut inner = state.inner.write().await;
    let save_path = req.path.clone();
    if save_path.is_empty() {
        return Err(AppError::Internal("저장 경로가 비어있습니다".into()));
    }
    let as_scenario = match req.save_type.as_deref() {
        Some("scenario") => true,
        Some("result") => false,
        _ => inner.turn_history.is_empty(),
    };
    inner
        .save_to_file(std::path::Path::new(&save_path), as_scenario)
        .map_err(|e| AppError::Internal(e))?;
    if as_scenario {
        inner.scenario_modified = false;
        inner.loaded_path = Some(save_path.clone());
    }
    Ok(Json(SaveResponse { path: save_path }))
}

pub async fn save_dir(
    State(state): State<AppState>,
) -> Result<Json<SaveDirInfo>, AppError> {
    let info = StudioService::get_save_dir(&state).await?;
    Ok(Json(info))
}

pub async fn load_state(
    State(state): State<AppState>,
    Json(req): Json<SaveRequest>,
) -> Result<StatusCode, AppError> {
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

pub async fn load_result(
    State(state): State<AppState>,
    Json(req): Json<SaveRequest>,
) -> Result<Json<LoadResultResponse>, AppError> {
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
    Ok(Json(LoadResultResponse { turn_history: history }))
}

#[derive(Serialize)]
pub struct LoadResultResponse {
    pub turn_history: Vec<TurnRecord>,
}

/// GET /api/scenarios — data/ 폴더에서 Mind Studio JSON 파일 목록 반환
pub async fn list_scenarios() -> Json<Vec<ScenarioInfo>> {
    Json(StudioService::list_scenarios())
}

// ---------------------------------------------------------------------------
// Scene: Focus 옵션 등록 + 초기 Focus appraise
// ---------------------------------------------------------------------------

/// POST /api/scene — Scene 시작: Focus 목록 등록 + 초기 Focus 자동 appraise
pub async fn scene(
    State(state): State<AppState>,
    Json(req): Json<SceneRequest>,
) -> Result<Json<SceneResponse>, AppError> {
    let mut inner = state.inner.write().await;
    let collector = state.collector.clone();
    let mut service = npc_mind::application::mind_service::MindService::new(crate::studio_service::AppStateRepository { inner: &mut *inner });
    let result = service.start_scene(req.clone(), || { collector.take_entries(); }, || collector.take_entries())?;
    let response = result.format(&*state.formatter);
    if response.initial_appraise.is_some() {
        let turn_num = inner.turn_history.len() + 1;
        inner.turn_history.push(TurnRecord { label: format!("Turn {}: scene/appraise [{}] ({}→{})", turn_num, response.active_focus_id.as_deref().unwrap_or("?"), req.npc_id, req.partner_id), action: "scene".into(), request: serde_json::to_value(&req).unwrap_or_default(), response: serde_json::to_value(&response).unwrap_or_default(), llm_model: None });
    }
    Ok(Json(response))
}

// ---------------------------------------------------------------------------
// Chat: LLM 대화 테스트 (chat feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "chat")]
pub mod chat_handlers {
    use super::*;

    /// POST /api/chat/start — 대화 세션 시작
    pub async fn chat_start(
        State(state): State<AppState>,
        Json(req): Json<ChatStartRequest>,
    ) -> Result<Json<ChatStartResponse>, AppError> {
        let response = StudioService::perform_chat_start(&state, req).await?;
        Ok(Json(response))
    }

    /// POST /api/chat/message — 대사 전송 → NPC 응답 + 감정 변화
    pub async fn chat_message(
        State(state): State<AppState>,
        Json(req): Json<ChatTurnRequest>,
    ) -> Result<Json<ChatTurnResponse>, AppError> {
        let response = StudioService::perform_chat_message(&state, req).await?;
        Ok(Json(response))
    }

    /// POST /api/chat/message/stream — 대사 전송 → NPC 응답 스트리밍 + 감정 변화
    pub async fn chat_message_stream(
        State(state): State<AppState>,
        Json(req): Json<ChatTurnRequest>,
    ) -> axum::response::Sse<impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>
    {
        let stream = async_stream::stream! {
            let chat_state = match state.chat.as_ref() { Some(c) => c, None => { yield Ok(axum::response::sse::Event::default().event("error").data("chat feature가 비활성입니다.")); return; } };
            let (token_tx, mut token_rx) = tokio::sync::mpsc::channel::<String>(64);
            let session_id = req.session_id.clone();
            let utterance = req.utterance.clone();
            let chat_state_clone = chat_state.clone();
            let llm_task = tokio::spawn(async move { chat_state_clone.send_message_stream(&session_id, &utterance, token_tx).await });
            while let Some(token) = token_rx.recv().await { yield Ok(axum::response::sse::Event::default().event("token").data(token)); }
            let npc_response = match llm_task.await { Ok(Ok(resp)) => resp, Ok(Err(e)) => { yield Ok(axum::response::sse::Event::default().event("error").data(e.to_string())); return; } Err(e) => { yield Ok(axum::response::sse::Event::default().event("error").data(format!("태스크 패닉: {e}"))); return; } };
            
            // 스트리밍 완료 후 턴 처리 로직은 perform_chat_message의 내부 로직 재사용 가능하도록 리팩토링 여지 있으나,
            // 일단은 handlers.rs의 스트림 특수성 유지
            let pad = if let Some(ref pad_input) = req.pad { Some((pad_input.pleasure, pad_input.arousal, pad_input.dominance)) } else if let Some(ref analyzer) = state.analyzer { let mut analyzer = analyzer.lock().await; match analyzer.analyze(&req.utterance) { Ok(p) => Some((p.pleasure, p.arousal, p.dominance)), Err(_) => None } } else { None };
            let (stimulus, beat_changed) = if let Some((p, a, d)) = pad {
                let stim_req = StimulusRequest { npc_id: req.npc_id.clone(), partner_id: req.partner_id.clone(), pleasure: p, arousal: a, dominance: d, situation_description: req.situation_description.clone() };
                let result = { let mut inner = state.inner.write().await; let collector = state.collector.clone(); let mut service = npc_mind::application::mind_service::MindService::new(crate::studio_service::AppStateRepository { inner: &mut *inner }); match service.apply_stimulus(stim_req, || { collector.take_entries(); }, || collector.take_entries()) { Ok(r) => r, Err(e) => { yield Ok(axum::response::sse::Event::default().event("error").data(e.to_string())); return; } } };
                let stim_resp = result.format(&*state.formatter);
                let changed = stim_resp.beat_changed;
                if changed { if let Err(e) = chat_state.update_system_prompt(&req.session_id, &stim_resp.prompt).await { yield Ok(axum::response::sse::Event::default().event("error").data(e.to_string())); return; } }
                { let mut inner = state.inner.write().await; let turn_num = inner.turn_history.len() + 1; let mut resp_val = serde_json::to_value(&stim_resp).unwrap_or_default(); if let serde_json::Value::Object(ref mut map) = resp_val { map.insert("npc_response".into(), serde_json::Value::String(npc_response.clone())); } inner.turn_history.push(TurnRecord { label: format!("Turn {}: chat/message [{}→{}]", turn_num, req.partner_id, req.npc_id), action: "chat_message".into(), request: serde_json::to_value(&req).unwrap_or_default(), response: resp_val, llm_model: None }); }
                (Some(stim_resp), changed)
            } else {
                let mut inner = state.inner.write().await; let turn_num = inner.turn_history.len() + 1; inner.turn_history.push(TurnRecord { label: format!("Turn {}: chat/message [{}→{}] (no PAD)", turn_num, req.partner_id, req.npc_id), action: "chat_message".into(), request: serde_json::to_value(&req).unwrap_or_default(), response: serde_json::json!({ "npc_response": &npc_response }), llm_model: None }); (None, false)
            };
            let final_response = ChatTurnResponse { npc_response, stimulus, beat_changed };
            yield Ok(axum::response::sse::Event::default().event("done").data(serde_json::to_string(&final_response).unwrap_or_default()));
        };
        axum::response::Sse::new(stream)
    }

    /// POST /api/chat/end — 세션 종료 + 대화 이력 반환
    pub async fn chat_end(
        State(state): State<AppState>,
        Json(req): Json<ChatEndRequest>,
    ) -> Result<Json<ChatEndResponse>, AppError> {
        let response = StudioService::perform_chat_end(&state, req).await?;
        Ok(Json(response))
    }
}
