//! API 핸들러 — CRUD + 파이프라인 실행

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use npc_mind::domain::emotion::*;
use npc_mind::domain::personality::Npc;
use npc_mind::domain::relationship::Relationship;

use npc_mind::application::dto::*;
use npc_mind::application::mind_service::{
    EmotionStore, MindService, MindServiceError, NpcWorld, SceneStore,
};
use npc_mind::ports::{LlmModelInfo, UtteranceAnalyzer};

use crate::state::*;

// ---------------------------------------------------------------------------
// WebUI 전용 에러 타입
// ---------------------------------------------------------------------------
pub enum AppError {
    Service(MindServiceError),
    Internal(String),
    /// 향후 리소스 조회 실패 핸들링용 — 현재 CRUD는 빈 결과를 허용하므로 미사용
    #[allow(dead_code)]
    NotFound(String),
    NotImplemented(String),
}

impl From<MindServiceError> for AppError {
    fn from(e: MindServiceError) -> Self {
        AppError::Service(e)
    }
}

impl From<npc_mind::ports::ConversationError> for AppError {
    fn from(e: npc_mind::ports::ConversationError) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Service(e) => match e {
                MindServiceError::NpcNotFound(_) | MindServiceError::RelationshipNotFound(_, _) => {
                    (StatusCode::NOT_FOUND, e.to_string())
                }
                MindServiceError::InvalidSituation(_) | MindServiceError::EmotionStateNotFound => {
                    (StatusCode::BAD_REQUEST, e.to_string())
                }
                MindServiceError::LocaleError(_) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
            },
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::NotImplemented(msg) => (StatusCode::NOT_IMPLEMENTED, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(serde_json::json!({
            "error": message
        }));

        (status, body).into_response()
    }
}

// ---------------------------------------------------------------------------
// Repository Wrapper for WebUI State
// ---------------------------------------------------------------------------
struct AppStateRepository<'a> {
    inner: &'a mut StateInner,
}

impl<'a> NpcWorld for AppStateRepository<'a> {
    fn get_npc(&self, id: &str) -> Option<Npc> {
        self.inner.npcs.get(id).map(|p| p.to_npc())
    }

    fn get_relationship(&self, owner_id: &str, target_id: &str) -> Option<Relationship> {
        self.inner
            .find_relationship(owner_id, target_id)
            .map(|r| r.to_relationship())
    }

    fn get_object_description(&self, object_id: &str) -> Option<String> {
        self.inner
            .objects
            .get(object_id)
            .map(|o| o.description.clone())
    }

    fn save_relationship(&mut self, owner_id: &str, target_id: &str, rel: Relationship) {
        let key = format!("{}:{}", owner_id, target_id);
        let existing_key = if self.inner.relationships.contains_key(&key) {
            key
        } else {
            let rev_key = format!("{}:{}", target_id, owner_id);
            if self.inner.relationships.contains_key(&rev_key) {
                rev_key
            } else {
                key
            }
        };

        self.inner.relationships.insert(
            existing_key,
            RelationshipData {
                owner_id: owner_id.to_string(),
                target_id: target_id.to_string(),
                closeness: rel.closeness().value(),
                trust: rel.trust().value(),
                power: rel.power().value(),
            },
        );
    }
}

impl<'a> EmotionStore for AppStateRepository<'a> {
    fn get_emotion_state(&self, npc_id: &str) -> Option<EmotionState> {
        self.inner.emotions.get(npc_id).cloned()
    }

    fn save_emotion_state(&mut self, npc_id: &str, state: EmotionState) {
        self.inner.emotions.insert(npc_id.to_string(), state);
    }

    fn clear_emotion_state(&mut self, npc_id: &str) {
        self.inner.emotions.remove(npc_id);
    }
}

impl<'a> SceneStore for AppStateRepository<'a> {
    fn get_scene(&self) -> Option<Scene> {
        let npc_id = self.inner.scene_npc_id.as_ref()?;
        let partner_id = self.inner.scene_partner_id.as_ref()?;
        let mut scene = Scene::new(
            npc_id.clone(),
            partner_id.clone(),
            self.inner.scene_focuses.clone(),
        );
        if let Some(ref id) = self.inner.active_focus_id {
            scene.set_active_focus(id.clone());
        }
        Some(scene)
    }

    fn save_scene(&mut self, scene: Scene) {
        self.inner.scene_npc_id = Some(scene.npc_id().to_string());
        self.inner.scene_partner_id = Some(scene.partner_id().to_string());
        self.inner.scene_focuses = scene.focuses().to_vec();
        self.inner.active_focus_id = scene.active_focus_id().map(|s| s.to_string());
    }

    fn clear_scene(&mut self) {
        self.inner.scene_npc_id = None;
        self.inner.scene_partner_id = None;
        self.inner.scene_focuses.clear();
        self.inner.active_focus_id = None;
    }
}

/// 읽기 전용 저장소 래퍼 (scene_info 등 불변 메서드용)
struct ReadOnlyAppStateRepo<'a> {
    inner: &'a StateInner,
}

impl<'a> NpcWorld for ReadOnlyAppStateRepo<'a> {
    fn get_npc(&self, id: &str) -> Option<Npc> {
        self.inner.npcs.get(id).map(|p| p.to_npc())
    }
    fn get_relationship(&self, owner_id: &str, target_id: &str) -> Option<Relationship> {
        self.inner
            .find_relationship(owner_id, target_id)
            .map(|r| r.to_relationship())
    }
    fn get_object_description(&self, _: &str) -> Option<String> {
        None
    }
    fn save_relationship(&mut self, _: &str, _: &str, _: Relationship) {
        unreachable!("read-only")
    }
}

impl<'a> EmotionStore for ReadOnlyAppStateRepo<'a> {
    fn get_emotion_state(&self, npc_id: &str) -> Option<EmotionState> {
        self.inner.emotions.get(npc_id).cloned()
    }
    fn save_emotion_state(&mut self, _: &str, _: EmotionState) {
        unreachable!("read-only")
    }
    fn clear_emotion_state(&mut self, _: &str) {
        unreachable!("read-only")
    }
}

impl<'a> SceneStore for ReadOnlyAppStateRepo<'a> {
    fn get_scene(&self) -> Option<Scene> {
        let npc_id = self.inner.scene_npc_id.as_ref()?;
        let partner_id = self.inner.scene_partner_id.as_ref()?;
        let mut scene = Scene::new(
            npc_id.clone(),
            partner_id.clone(),
            self.inner.scene_focuses.clone(),
        );
        if let Some(ref id) = self.inner.active_focus_id {
            scene.set_active_focus(id.clone());
        }
        Some(scene)
    }

    fn save_scene(&mut self, _: Scene) {
        unreachable!("read-only")
    }
    fn clear_scene(&mut self) {
        unreachable!("read-only")
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
// 파이프라인: 감정 평가
// ---------------------------------------------------------------------------

/// POST /api/appraise — 감정 평가 실행
pub async fn appraise(
    State(state): State<AppState>,
    Json(req): Json<AppraiseRequest>,
) -> Result<Json<AppraiseResponse>, AppError> {
    let mut inner = state.inner.write().await;
    let collector = state.collector.clone();

    let mut service = MindService::new(AppStateRepository { inner: &mut *inner });

    let result = service.appraise(
        req.clone(),
        || {
            collector.take_entries();
        }, // before
        || collector.take_entries(), // after
    )?;

    let response = result.format(&*state.formatter);

    // 턴 기록 저장
    let turn_num = inner.turn_history.len() + 1;
    inner.turn_history.push(TurnRecord {
        label: format!(
            "Turn {}: appraise ({}→{})",
            turn_num, req.npc_id, req.partner_id
        ),
        action: "appraise".into(),
        request: serde_json::to_value(&req).unwrap_or_default(),
        response: serde_json::to_value(&response).unwrap_or_default(),
        llm_model: None,
    });

    Ok(Json(response))
}

// ---------------------------------------------------------------------------
// 파이프라인: PAD 자극 적용
// ---------------------------------------------------------------------------

/// POST /api/stimulus — PAD 자극 적용 → 감정 변동 + Focus 전환 판단
pub async fn stimulus(
    State(state): State<AppState>,
    Json(req): Json<StimulusRequest>,
) -> Result<Json<StimulusResponse>, AppError> {
    let mut inner = state.inner.write().await;
    let collector = state.collector.clone();

    let mut service = MindService::new(AppStateRepository { inner: &mut *inner });
    let result = service.apply_stimulus(
        req.clone(),
        || {
            collector.take_entries();
        },
        || collector.take_entries(),
    )?;
    drop(service);

    let response = result.format(&*state.formatter);

    // 턴 기록
    let turn_num = inner.turn_history.len() + 1;
    let label = if response.beat_changed {
        format!(
            "Turn {}: stimulus+beat [{}] ({})",
            turn_num,
            response.active_focus_id.as_deref().unwrap_or("?"),
            req.npc_id
        )
    } else {
        format!("Turn {}: stimulus ({})", turn_num, req.npc_id)
    };
    inner.turn_history.push(TurnRecord {
        label,
        action: "stimulus".into(),
        request: serde_json::to_value(&req).unwrap_or_default(),
        response: serde_json::to_value(&response).unwrap_or_default(),
        llm_model: None,
    });

    Ok(Json(response))
}

// ---------------------------------------------------------------------------
// 파이프라인: 가이드 재생성 (현재 감정 상태 기준)
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

    let service = MindService::new(AppStateRepository { inner: &mut *inner });
    let result = service.generate_guide(req)?;
    Ok(Json(result.format(&*state.formatter)))
}

// ---------------------------------------------------------------------------
// 파이프라인: 대화 종료 → 관계 갱신
// ---------------------------------------------------------------------------

/// POST /api/after-dialogue — 대화 종료 → 관계 갱신
pub async fn after_dialogue(
    State(state): State<AppState>,
    Json(req): Json<AfterDialogueRequest>,
) -> Result<Json<AfterDialogueResponse>, AppError> {
    let mut inner = state.inner.write().await;
    let mut service = MindService::new(AppStateRepository { inner: &mut *inner });

    let response = service.after_dialogue(req.clone())?;

    // 턴 기록
    let turn_num = inner.turn_history.len() + 1;
    inner.turn_history.push(TurnRecord {
        label: format!(
            "Turn {}: after_dialogue ({}→{})",
            turn_num, req.npc_id, req.partner_id
        ),
        action: "after_dialogue".into(),
        request: serde_json::to_value(&req).unwrap_or_default(),
        response: serde_json::to_value(&response).unwrap_or_default(),
        llm_model: None,
    });

    Ok(Json(response))
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
// 시나리오 메타 조회
// ---------------------------------------------------------------------------

/// GET /api/scenario-meta — 현재 로드된 시나리오 메타 정보
pub async fn get_scenario_meta(State(state): State<AppState>) -> Json<ScenarioMeta> {
    let inner = state.inner.read().await;
    Json(inner.scenario.clone())
}

// ---------------------------------------------------------------------------
// Scene 정보 조회 (프론트엔드 읽기 전용 패널용)
// ---------------------------------------------------------------------------

/// GET /api/scene-info — 현재 Scene Focus 상태 조회
pub async fn get_scene_info(State(state): State<AppState>) -> Json<SceneInfoResult> {
    let inner = state.inner.read().await;
    let repo = ReadOnlyAppStateRepo { inner: &*inner };
    let service = MindService::new(repo);
    Json(service.scene_info())
}

// ---------------------------------------------------------------------------
// 턴 히스토리 조회
// ---------------------------------------------------------------------------

/// GET /api/history — 턴별 기록 조회
pub async fn get_history(State(state): State<AppState>) -> Json<Vec<TurnRecord>> {
    let inner = state.inner.read().await;
    Json(inner.turn_history.clone())
}

// ---------------------------------------------------------------------------
// 상황 설정 패널 상태 저장/조회
// ---------------------------------------------------------------------------

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
    /// "scenario" | "result" — 저장 유형 (기본: turn_history 유무로 자동 결정)
    #[serde(default)]
    pub save_type: Option<String>,
}

/// POST /api/save — JSON 파일로 저장
/// 결과 저장 응답 (저장된 경로 반환)
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
        _ => inner.turn_history.is_empty(), // 미지정 시 자동 결정
    };
    inner
        .save_to_file(std::path::Path::new(&save_path), as_scenario)
        .map_err(|e| AppError::Internal(e))?;
    if as_scenario {
        inner.scenario_modified = false;
        // 신규 이름으로 저장 시 loaded_path 갱신 → result 폴더 경로가 새 시나리오 기준으로 바뀜
        inner.loaded_path = Some(save_path.clone());
    }
    Ok(Json(SaveResponse { path: save_path }))
}

/// GET /api/save-dir — loaded_path 기반 결과 저장 폴더 경로 반환
/// 예: loaded="data/foo/scenario.json" → "data/foo/scenario_result"
pub async fn save_dir(
    State(state): State<AppState>,
) -> Result<Json<SaveDirResponse>, AppError> {
    let inner = state.inner.read().await;
    let loaded = inner
        .loaded_path
        .as_deref()
        .ok_or_else(|| AppError::Internal("로드된 시나리오가 없습니다".into()))?;

    let p = std::path::Path::new(loaded);
    let parent = p.parent().unwrap_or(std::path::Path::new("data"));
    let stem = p
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("scenario");
    let result_dir = parent.join(format!("{}_result", stem));

    // 폴더 생성 (없으면)
    std::fs::create_dir_all(&result_dir)
        .map_err(|e| AppError::Internal(format!("폴더 생성 실패: {}", e)))?;

    // 결과 폴더에 기존 파일 존재 여부
    let has_existing_results = result_dir.is_dir()
        && std::fs::read_dir(&result_dir)
            .ok()
            .map(|entries| entries.flatten().any(|e| {
                e.path().extension().map(|ext| ext == "json").unwrap_or(false)
            }))
            .unwrap_or(false);

    Ok(Json(SaveDirResponse {
        dir: result_dir.to_string_lossy().replace('\\', "/"),
        loaded_path: loaded.to_string(),
        scenario_name: inner.scenario.name.clone(),
        scenario_modified: inner.scenario_modified,
        has_turn_history: !inner.turn_history.is_empty(),
        has_existing_results,
    }))
}

#[derive(Serialize)]
pub struct SaveDirResponse {
    /// 결과 저장 폴더 경로
    pub dir: String,
    /// 원본 시나리오 파일 경로
    pub loaded_path: String,
    /// 시나리오 이름
    pub scenario_name: String,
    /// 시나리오 수정 여부
    pub scenario_modified: bool,
    /// 대화 기록 존재 여부 (대화 종료 후)
    pub has_turn_history: bool,
    /// 결과 폴더에 기존 결과 파일 존재 여부
    pub has_existing_results: bool,
}

/// POST /api/load — JSON 파일에서 로드 (scene 필드가 있으면 자동 Focus 등록, turn_history 무시)
pub async fn load_state(
    State(state): State<AppState>,
    Json(req): Json<SaveRequest>,
) -> Result<StatusCode, AppError> {
    let mut loaded = StateInner::load_from_file(std::path::Path::new(&req.path))
        .map_err(|e| AppError::Internal(e))?;

    // 시나리오 로드 시 turn_history는 비움 (깨끗한 상태에서 시작)
    loaded.turn_history.clear();
    // 로드 경로 기억 (결과 저장 시 자동 경로 계산용)
    loaded.loaded_path = Some(req.path.clone());

    if let Some(ref scene_val) = loaded.scene {
        if let Ok(scene_req) = serde_json::from_value::<SceneRequest>(scene_val.clone()) {
            load_scene_into_state(&mut loaded, &scene_req);
        }
    }

    let mut inner = state.inner.write().await;
    *inner = loaded;
    Ok(StatusCode::OK)
}

/// POST /api/load-result — 테스트 결과 로드 (turn_history 포함, 읽기전용 뷰용)
pub async fn load_result(
    State(state): State<AppState>,
    Json(req): Json<SaveRequest>,
) -> Result<Json<LoadResultResponse>, AppError> {
    let mut loaded = StateInner::load_from_file(std::path::Path::new(&req.path))
        .map_err(|e| AppError::Internal(e))?;

    // 로드 경로 기억
    loaded.loaded_path = Some(req.path.clone());

    if let Some(ref scene_val) = loaded.scene {
        if let Ok(scene_req) = serde_json::from_value::<SceneRequest>(scene_val.clone()) {
            load_scene_into_state(&mut loaded, &scene_req);
        }
    }

    // turn_history를 응답에 포함
    let history = loaded.turn_history.clone();

    let mut inner = state.inner.write().await;
    *inner = loaded;

    Ok(Json(LoadResultResponse { turn_history: history }))
}

#[derive(Serialize)]
pub struct LoadResultResponse {
    pub turn_history: Vec<TurnRecord>,
}

/// Scene 필드를 파싱하여 Focus 등록 + UI 폼 복원용 situation 생성
fn load_scene_into_state(loaded: &mut StateInner, scene_req: &SceneRequest) {
    // Focus 변환 + MindService 로드
    let repo = AppStateRepository { inner: loaded };
    let focuses: Vec<npc_mind::domain::emotion::SceneFocus> = scene_req
        .focuses
        .iter()
        .filter_map(|f| {
            f.to_domain(&repo, &scene_req.npc_id, &scene_req.partner_id)
                .ok()
        })
        .collect();
    drop(repo);

    let significance = scene_req.significance.unwrap_or(0.5);
    let mut service = MindService::new(AppStateRepository { inner: loaded });
    let _ = service.load_scene_focuses(
        focuses,
        scene_req.npc_id.clone(),
        scene_req.partner_id.clone(),
        significance,
    );
    drop(service);

    // Initial Focus → UI 폼 복원용 situation 맵 생성
    let initial_input = scene_req.focuses.iter().find(|f| f.trigger.is_none());
    if let Some(fi) = initial_input {
        loaded.current_situation = Some(serde_json::Value::Object(build_situation_map(
            fi,
            &scene_req.npc_id,
            &scene_req.partner_id,
        )));
    }
}

/// UI 폼 복원용 상황 데이터 — SceneFocusInput → 평탄화된 JSON 구조
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SituationFormData {
    desc: String,
    npc_id: String,
    partner_id: String,
    has_event: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    ev_desc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ev_self: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    has_other: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    other_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    other_d: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prospect: Option<String>,
    has_action: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    ac_desc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pw: Option<f32>,
    has_object: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    obj_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    obj_ap: Option<f32>,
}

/// SceneFocusInput에서 UI 폼 복원용 JSON 맵을 생성합니다.
fn build_situation_map(
    fi: &SceneFocusInput,
    npc_id: &str,
    partner_id: &str,
) -> serde_json::Map<String, serde_json::Value> {
    let form = SituationFormData {
        desc: fi.description.clone(),
        npc_id: npc_id.to_string(),
        partner_id: partner_id.to_string(),
        has_event: fi.event.is_some(),
        ev_desc: fi.event.as_ref().map(|e| e.description.clone()),
        ev_self: fi.event.as_ref().map(|e| e.desirability_for_self),
        has_other: fi.event.as_ref().map(|e| e.other.is_some()),
        other_target: fi.event.as_ref().and_then(|e| e.other.as_ref().map(|o| o.target_id.clone())),
        other_d: fi.event.as_ref().and_then(|e| e.other.as_ref().map(|o| o.desirability)),
        prospect: fi.event.as_ref().and_then(|e| e.prospect.clone()),
        has_action: fi.action.is_some(),
        ac_desc: fi.action.as_ref().map(|a| a.description.clone()),
        agent_id: fi.action.as_ref().and_then(|a| a.agent_id.clone()),
        pw: fi.action.as_ref().map(|a| a.praiseworthiness),
        has_object: fi.object.is_some(),
        obj_target: fi.object.as_ref().map(|o| o.target_id.clone()),
        obj_ap: fi.object.as_ref().map(|o| o.appealingness),
    };

    match serde_json::to_value(form) {
        Ok(serde_json::Value::Object(map)) => map,
        _ => serde_json::Map::new(),
    }
}

// ---------------------------------------------------------------------------
// 시나리오 목록 (data/ 폴더 스캔)
// ---------------------------------------------------------------------------

use crate::state::{FORMAT_SCENARIO, FORMAT_RESULT};

/// GET /api/scenarios — data/ 폴더에서 Mind Studio JSON 파일 목록 반환
pub async fn list_scenarios() -> Json<Vec<ScenarioInfo>> {
    let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let mut scenarios = Vec::new();
    scan_scenarios(&data_dir, &data_dir, &mut scenarios);
    scenarios.sort_by(|a, b| a.path.cmp(&b.path));
    Json(scenarios)
}

#[derive(Serialize)]
pub struct ScenarioInfo {
    /// data/ 기준 상대 경로 (슬래시 구분, 파일명 포함)
    pub path: String,
    /// 표시용 이름
    pub label: String,
    /// 테스트 결과 파일인지 여부
    pub has_results: bool,
}

/// data/ 재귀 스캔: 모든 .json → format 필드 또는 npcs 유무로 Mind Studio 파일 판별
fn scan_scenarios(base: &std::path::Path, dir: &std::path::Path, out: &mut Vec<ScenarioInfo>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_scenarios(base, &path, out);
                continue;
            }
            // .json 파일만 처리
            let is_json = path.extension().map(|e| e == "json").unwrap_or(false);
            if !is_json {
                continue;
            }
            // 빠른 파싱
            let val = match std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            {
                Some(v) => v,
                None => continue,
            };
            // format 필드로 판별 (필수)
            let format_str = match val.get("format").and_then(|f| f.as_str()) {
                Some(f) => f,
                None => continue, // format 필드 없으면 무시
            };

            // Mind Studio 파일만 인식
            let has_results = if format_str == FORMAT_RESULT {
                true
            } else if format_str == FORMAT_SCENARIO {
                false
            } else {
                continue; // 알 수 없는 format → 무시
            };

            // 상대 경로 (파일명 포함)
            if let Ok(rel) = path.strip_prefix(base) {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                // 라벨: 폴더/파일명에서 .json 제거, / → " / "
                let label = rel_str
                    .trim_end_matches(".json")
                    .replace('/', " / ");
                out.push(ScenarioInfo {
                    path: rel_str,
                    label,
                    has_results,
                });
            }
        }
    }
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

    let mut service = MindService::new(AppStateRepository { inner: &mut *inner });
    let result = service.start_scene(
        req.clone(),
        || {
            collector.take_entries();
        },
        || collector.take_entries(),
    )?;
    drop(service);

    let response = result.format(&*state.formatter);

    // 턴 기록 (초기 appraise가 있을 때만)
    if response.initial_appraise.is_some() {
        let turn_num = inner.turn_history.len() + 1;
        inner.turn_history.push(TurnRecord {
            label: format!(
                "Turn {}: scene/appraise [{}] ({}→{})",
                turn_num,
                response.active_focus_id.as_deref().unwrap_or("?"),
                req.npc_id,
                req.partner_id
            ),
            action: "scene".into(),
            request: serde_json::to_value(&req).unwrap_or_default(),
            response: serde_json::to_value(&response).unwrap_or_default(),
            llm_model: None,
        });
    }

    Ok(Json(response))
}

// ---------------------------------------------------------------------------
// Chat: LLM 대화 테스트 (chat feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "chat")]
pub mod chat_handlers {
    use super::*;
    use npc_mind::application::dialogue_test_service::*;

    /// POST /api/chat/start — 대화 세션 시작 (appraise + LLM agent 생성)
    pub async fn chat_start(
        State(state): State<AppState>,
        Json(req): Json<ChatStartRequest>,
    ) -> Result<Json<ChatStartResponse>, AppError> {
        let chat_state = state
            .chat
            .as_ref()
            .ok_or_else(|| AppError::NotImplemented("chat feature가 비활성입니다.".into()))?;

        // 1. appraise로 프롬프트 생성
        let mut inner = state.inner.write().await;
        let collector = state.collector.clone();

        let mut service = MindService::new(AppStateRepository { inner: &mut *inner });
        
        // NPC 정보로 파라미터 계산 (대화 시작 시 1회)
        let (temp, top_p) = {
            let npc = service.repository().get_npc(&req.appraise.npc_id).ok_or_else(|| {
                AppError::Internal(format!("NPC {}를 찾을 수 없습니다", req.appraise.npc_id))
            })?;
            npc.derive_llm_parameters()
        };

        let result = service.appraise(
            req.appraise.clone(),
            || {
                collector.take_entries();
            },
            || collector.take_entries(),
        )?;
        drop(service);

        let response = result.format(&*state.formatter);

        // 3. 모델 정보 캡처 (글로벌 정보 + NPC별 파라미터)
        let mut llm_model_info = state.llm_info.as_ref().map(|info| info.get_model_info()).unwrap_or(LlmModelInfo {
            provider_url: "unknown".into(),
            model_name: "unknown".into(),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: None,
            seed: None,
        });
        llm_model_info.temperature = Some(temp);
        llm_model_info.top_p = Some(top_p);

        // 2. LLM 세션 시작 (파라미터 고정 전달)
        chat_state
            .start_session(&req.session_id, &response.prompt, Some(llm_model_info.clone()))
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        // 턴 기록
        let turn_num = inner.turn_history.len() + 1;
        inner.turn_history.push(TurnRecord {
            label: format!("Turn {}: chat/start ({})", turn_num, req.session_id),
            action: "chat_start".into(),
            request: serde_json::to_value(&req).unwrap_or_default(),
            response: serde_json::to_value(&response).unwrap_or_default(),
            llm_model: Some(llm_model_info.clone()),
        });

        Ok(Json(ChatStartResponse {
            session_id: req.session_id,
            appraise: response,
            llm_model_info: Some(llm_model_info),
        }))
    }

    /// POST /api/chat/message — 대사 전송 → NPC 응답 + 감정 변화
    pub async fn chat_message(
        State(state): State<AppState>,
        Json(req): Json<ChatTurnRequest>,
    ) -> Result<Json<ChatTurnResponse>, AppError> {
        let chat_state = state
            .chat
            .as_ref()
            .ok_or_else(|| AppError::NotImplemented("chat feature가 비활성입니다.".into()))?;

        // 1. LLM에 대사 전달 → NPC 응답 (파라미터는 이미 세션에 저장됨)
        let npc_response = chat_state
            .send_message(&req.session_id, &req.utterance)
            .await?;

        // 2. PAD 결정 (수동 입력 > 자동 분석 > 없음)
        let pad = if let Some(ref pad_input) = req.pad {
            Some((pad_input.pleasure, pad_input.arousal, pad_input.dominance))
        } else if let Some(ref analyzer) = state.analyzer {
            let mut analyzer = analyzer.lock().await;
            match analyzer.analyze(&req.utterance) {
                Ok(p) => Some((p.pleasure, p.arousal, p.dominance)),
                Err(_) => None,
            }
        } else {
            None
        };

        // 3. stimulus 적용
        let (stimulus, beat_changed) = if let Some((p, a, d)) = pad {
            let stim_req = StimulusRequest {
                npc_id: req.npc_id.clone(),
                partner_id: req.partner_id.clone(),
                pleasure: p,
                arousal: a,
                dominance: d,
                situation_description: req.situation_description.clone(),
            };

            let mut inner = state.inner.write().await;
            let collector = state.collector.clone();
            let mut service = MindService::new(AppStateRepository { inner: &mut *inner });
            let result = service.apply_stimulus(
                stim_req,
                || {
                    collector.take_entries();
                },
                || collector.take_entries(),
            )?;
            drop(service);

            let stim_resp = result.format(&*state.formatter);
            let changed = stim_resp.beat_changed;

            // 4. Beat 전환 시 system_prompt 갱신
            if changed {
                chat_state
                    .update_system_prompt(&req.session_id, &stim_resp.prompt)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }

            // 턴 기록 — npc_response 텍스트를 response에 포함
            let turn_num = inner.turn_history.len() + 1;
            let mut resp_val = serde_json::to_value(&stim_resp).unwrap_or_default();
            if let serde_json::Value::Object(ref mut map) = resp_val {
                map.insert("npc_response".into(), serde_json::Value::String(npc_response.clone()));
            }
                    inner.turn_history.push(TurnRecord {
                        label: format!(
                            "Turn {}: chat/message [{}→{}]",
                            turn_num, req.partner_id, req.npc_id
                        ),
                        action: "chat_message".into(),
                        request: serde_json::to_value(&req).unwrap_or_default(),
                        response: resp_val,
                        llm_model: None,
                    });

            (Some(stim_resp), changed)
        } else {
            // PAD 없이 — 대사만 교환 (감정 변화 없음)
            let mut inner = state.inner.write().await;
            let turn_num = inner.turn_history.len() + 1;
            inner.turn_history.push(TurnRecord {
                label: format!(
                    "Turn {}: chat/message [{}→{}] (no PAD)",
                    turn_num, req.partner_id, req.npc_id
                ),
                action: "chat_message".into(),
                request: serde_json::to_value(&req).unwrap_or_default(),
                response: serde_json::json!({ "npc_response": &npc_response }),
                llm_model: None,
            });

            (None, false)
        };

        Ok(Json(ChatTurnResponse {
            npc_response,
            stimulus,
            beat_changed,
        }))
    }

    /// POST /api/chat/message/stream — 대사 전송 → NPC 응답 스트리밍 + 감정 변화
    ///
    /// SSE 이벤트 형식:
    /// - `event: token` / `data: <text_chunk>` — 토큰 청크
    /// - `event: done`  / `data: <ChatTurnResponse JSON>` — 완료 + stimulus 결과
    /// - `event: error` / `data: <message>` — 에러
    pub async fn chat_message_stream(
        State(state): State<AppState>,
        Json(req): Json<ChatTurnRequest>,
    ) -> axum::response::Sse<impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>
    {
        let stream = async_stream::stream! {
            // chat feature 확인
            let chat_state = match state.chat.as_ref() {
                Some(c) => c,
                None => {
                    yield Ok(axum::response::sse::Event::default()
                        .event("error")
                        .data("chat feature가 비활성입니다."));
                    return;
                }
            };

            // 1. 토큰 스트리밍을 위한 mpsc 채널
            let (token_tx, mut token_rx) = tokio::sync::mpsc::channel::<String>(64);

            // 2. LLM 스트리밍 호출을 백그라운드 태스크로 실행
            let session_id = req.session_id.clone();
            let utterance = req.utterance.clone();
            let chat_state_clone = chat_state.clone();
            let llm_task = tokio::spawn(async move {
                chat_state_clone
                    .send_message_stream(&session_id, &utterance, token_tx)
                    .await
            });

            // 3. 토큰 도착마다 SSE event 전송
            while let Some(token) = token_rx.recv().await {
                yield Ok(axum::response::sse::Event::default()
                    .event("token")
                    .data(token));
            }

            // 4. LLM 응답 완료 — 전체 응답 수집
            let npc_response = match llm_task.await {
                Ok(Ok(resp)) => resp,
                Ok(Err(e)) => {
                    yield Ok(axum::response::sse::Event::default()
                        .event("error")
                        .data(e.to_string()));
                    return;
                }
                Err(e) => {
                    yield Ok(axum::response::sse::Event::default()
                        .event("error")
                        .data(format!("태스크 패닉: {e}")));
                    return;
                }
            };

            // 5. PAD 결정 (수동 입력 > 자동 분석 > 없음)
            let pad = if let Some(ref pad_input) = req.pad {
                Some((pad_input.pleasure, pad_input.arousal, pad_input.dominance))
            } else if let Some(ref analyzer) = state.analyzer {
                let mut analyzer = analyzer.lock().await;
                match analyzer.analyze(&req.utterance) {
                    Ok(p) => Some((p.pleasure, p.arousal, p.dominance)),
                    Err(_) => None,
                }
            } else {
                None
            };

            // 6. stimulus 적용
            let (stimulus, beat_changed) = if let Some((p, a, d)) = pad {
                let stim_req = StimulusRequest {
                    npc_id: req.npc_id.clone(),
                    partner_id: req.partner_id.clone(),
                    pleasure: p,
                    arousal: a,
                    dominance: d,
                    situation_description: req.situation_description.clone(),
                };

                let result = {
                    let mut inner = state.inner.write().await;
                    let collector = state.collector.clone();
                    let mut service = MindService::new(AppStateRepository { inner: &mut *inner });
                    match service.apply_stimulus(
                        stim_req,
                        || { collector.take_entries(); },
                        || collector.take_entries(),
                    ) {
                        Ok(r) => r,
                        Err(e) => {
                            yield Ok(axum::response::sse::Event::default()
                                .event("error")
                                .data(e.to_string()));
                            return;
                        }
                    }
                };

                let stim_resp = result.format(&*state.formatter);
                let changed = stim_resp.beat_changed;

                // Beat 전환 시 system_prompt 갱신
                if changed {
                    if let Err(e) = chat_state
                        .update_system_prompt(&req.session_id, &stim_resp.prompt)
                        .await
                    {
                        yield Ok(axum::response::sse::Event::default()
                            .event("error")
                            .data(e.to_string()));
                        return;
                    }
                }

                // 턴 기록 — npc_response 텍스트를 response에 포함
                {
                    let mut inner = state.inner.write().await;
                    let turn_num = inner.turn_history.len() + 1;
                    let mut resp_val = serde_json::to_value(&stim_resp).unwrap_or_default();
                    if let serde_json::Value::Object(ref mut map) = resp_val {
                        map.insert("npc_response".into(), serde_json::Value::String(npc_response.clone()));
                    }
                    inner.turn_history.push(TurnRecord {
                        label: format!(
                            "Turn {}: chat/message [{}→{}]",
                            turn_num, req.partner_id, req.npc_id
                        ),
                        action: "chat_message".into(),
                        request: serde_json::to_value(&req).unwrap_or_default(),
                        response: resp_val,
                        llm_model: None,
                    });
                }

                (Some(stim_resp), changed)
            } else {
                // PAD 없이 — 대사만 교환 (감정 변화 없음)
                let mut inner = state.inner.write().await;
                let turn_num = inner.turn_history.len() + 1;
                inner.turn_history.push(TurnRecord {
                    label: format!(
                        "Turn {}: chat/message [{}→{}] (no PAD)",
                        turn_num, req.partner_id, req.npc_id
                    ),
                    action: "chat_message".into(),
                    request: serde_json::to_value(&req).unwrap_or_default(),
                    response: serde_json::json!({ "npc_response": &npc_response }),
                    llm_model: None,
                });

                (None, false)
            };

            // 7. 최종 결과를 done 이벤트로 전송
            let final_response = ChatTurnResponse {
                npc_response,
                stimulus,
                beat_changed,
            };
            yield Ok(axum::response::sse::Event::default()
                .event("done")
                .data(serde_json::to_string(&final_response).unwrap_or_default()));
        };

        axum::response::Sse::new(stream)
    }

    /// POST /api/chat/end — 세션 종료 + 대화 이력 반환
    pub async fn chat_end(
        State(state): State<AppState>,
        Json(req): Json<ChatEndRequest>,
    ) -> Result<Json<ChatEndResponse>, AppError> {
        let chat_state = state
            .chat
            .as_ref()
            .ok_or_else(|| AppError::NotImplemented("chat feature가 비활성입니다.".into()))?;

        // 1. 세션 종료 → 이력
        let dialogue_history = chat_state
            .end_session(&req.session_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        // 2. 관계 갱신 (선택)
        let after_dialogue = if let Some(after_req) = req.after_dialogue {
            let mut inner = state.inner.write().await;
            let mut service = MindService::new(AppStateRepository { inner: &mut *inner });
            let resp = service.after_dialogue(after_req)?;
            Some(resp)
        } else {
            None
        };

        Ok(Json(ChatEndResponse {
            dialogue_history,
            after_dialogue,
        }))
    }
}
