//! API 핸들러 — CRUD + 파이프라인 실행

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use npc_mind::domain::emotion::*;
use npc_mind::domain::relationship::Relationship;
use npc_mind::domain::personality::Npc;
use npc_mind::domain::guide::ActingGuide;
use npc_mind::domain::tuning::BEAT_MERGE_THRESHOLD;
use npc_mind::ports::GuideFormatter;
use npc_mind::presentation::korean::KoreanFormatter;

use npc_mind::application::dto::*;
use npc_mind::application::mind_service::{MindService, MindServiceError, MindRepository};
use npc_mind::ports::UtteranceAnalyzer;

use crate::state::*;

// ---------------------------------------------------------------------------
// WebUI 전용 에러 타입
// ---------------------------------------------------------------------------
pub enum AppError {
    Service(MindServiceError),
    Internal(String),
    #[allow(dead_code)]
    NotFound(String),
    NotImplemented(String),
}

impl From<MindServiceError> for AppError {
    fn from(e: MindServiceError) -> Self {
        AppError::Service(e)
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

impl<'a> MindRepository for AppStateRepository<'a> {
    fn get_npc(&self, id: &str) -> Option<Npc> {
        self.inner.npcs.get(id).map(|p| p.to_npc())
    }

    fn get_relationship(&self, owner_id: &str, target_id: &str) -> Option<Relationship> {
        self.inner.find_relationship(owner_id, target_id).map(|r| r.to_relationship())
    }

    fn get_object_description(&self, object_id: &str) -> Option<String> {
        self.inner.objects.get(object_id).map(|o| o.description.clone())
    }

    fn get_emotion_state(&self, npc_id: &str) -> Option<EmotionState> {
        self.inner.emotions.get(npc_id).cloned()
    }

    fn save_emotion_state(&mut self, npc_id: &str, state: EmotionState) {
        self.inner.emotions.insert(npc_id.to_string(), state);
    }

    fn clear_emotion_state(&mut self, npc_id: &str) {
        self.inner.emotions.remove(npc_id);
    }

    fn save_relationship(&mut self, owner_id: &str, target_id: &str, rel: Relationship) {
        let key = format!("{}:{}", owner_id, target_id);
        // We preserve the existing key if it exists, otherwise create a new one based on order
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

        self.inner.relationships.insert(existing_key, RelationshipData {
            owner_id: owner_id.to_string(),
            target_id: target_id.to_string(),
            closeness: rel.closeness().value(),
            trust: rel.trust().value(),
            power: rel.power().value(),
        });
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
pub async fn upsert_npc(
    State(state): State<AppState>,
    Json(npc): Json<NpcProfile>,
) -> StatusCode {
    let mut inner = state.inner.write().await;
    inner.npcs.insert(npc.id.clone(), npc);
    StatusCode::OK
}

/// DELETE /api/npcs/:id
pub async fn delete_npc(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> StatusCode {
    let mut inner = state.inner.write().await;
    inner.npcs.remove(&id);
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
    StatusCode::OK
}

/// DELETE /api/objects/:id
pub async fn delete_object(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> StatusCode {
    let mut inner = state.inner.write().await;
    inner.objects.remove(&id);
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
    
    let response = service.appraise(
        req.clone(),
        || { collector.take_entries(); }, // before
        || collector.take_entries(),      // after
    )?;

    // 턴 기록 저장
    let turn_num = inner.turn_history.len() + 1;
    inner.turn_history.push(TurnRecord {
        label: format!("Turn {}: appraise ({}→{})", turn_num, req.npc_id, req.partner_id),
        action: "appraise".into(),
        request: serde_json::to_value(&req).unwrap_or_default(),
        response: serde_json::to_value(&response).unwrap_or_default(),
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

    // 1. 기존 감정 강도 조정 (관성 적용)
    let mut service = MindService::new(AppStateRepository { inner: &mut *inner });
    collector.take_entries();
    let stimulus_result = service.apply_stimulus(req.clone())?;
    let trace = collector.take_entries();
    drop(service);

    // 2. Focus 전환 판단
    let mut beat_changed = false;
    let mut active_focus_id: Option<String> = None;
    let mut final_response = None;

    if !inner.scene_focuses.is_empty() {
        if let Some(emotion_state) = inner.emotions.get(&req.npc_id) {
            // 대기 중 Focus (Initial 제외)에서 조건 충족 확인 — 순서가 우선순위
            let triggered_idx = inner.scene_focuses.iter().position(|f| {
                f.trigger.is_met(emotion_state)
            });

            if let Some(idx) = triggered_idx {
                let focus = inner.scene_focuses[idx].clone();

                // 2a. after_beat — 현재 감정으로 관계 갱신 (clear 안 함)
                if let (Some(npc_id), Some(partner_id)) = (&inner.scene_npc_id, &inner.scene_partner_id) {
                    let beat_req = AfterDialogueRequest {
                        npc_id: npc_id.clone(),
                        partner_id: partner_id.clone(),
                        praiseworthiness: req.situation_description.as_ref().and(Some(0.0)),
                        significance: Some(0.5),
                    };
                    let mut svc = MindService::new(AppStateRepository { inner: &mut *inner });
                    let _ = svc.after_beat(beat_req);
                    drop(svc);
                }

                // 2b. 새 Focus로 appraise → merge_from_beat
                let situation = focus.to_situation()
                    .map_err(|e| AppError::Internal(e.to_string()))?;

                let npc = inner.npcs.get(&req.npc_id)
                    .ok_or_else(|| AppError::Internal(format!("NPC '{}' not found", req.npc_id)))?
                    .to_npc();

                let rel = inner.find_relationship(&req.npc_id, &req.partner_id)
                    .ok_or_else(|| AppError::Internal("Relationship not found".into()))?
                    .to_relationship();

                let previous = inner.emotions.get(&req.npc_id).cloned()
                    .unwrap_or_else(EmotionState::new);

                collector.take_entries();
                let new_state = AppraisalEngine::appraise(npc.personality(), &situation, &rel);
                let beat_trace = collector.take_entries();

                // 합치기: 이전 감정 중 threshold 이상 + 새 감정 (같은 종류는 max)
                let merged = EmotionState::merge_from_beat(
                    &previous,
                    &new_state,
                    npc_mind::domain::tuning::BEAT_MERGE_THRESHOLD,
                );

                inner.emotions.insert(req.npc_id.clone(), merged.clone());

                let response = build_emotion_response(
                    &npc, &merged, Some(focus.description.clone()), Some(&rel), beat_trace,
                );

                beat_changed = true;
                active_focus_id = Some(focus.id.clone());
                inner.active_focus_id = Some(focus.id.clone());
                final_response = Some(response);
            }
        }
    }

    // 3. 응답 구성
    let response = if let Some(resp) = final_response {
        // Beat 전환이 발생한 경우
        StimulusResponse {
            emotions: resp.emotions,
            dominant: resp.dominant,
            mood: resp.mood,
            prompt: resp.prompt,
            trace: resp.trace,
            beat_changed: true,
            active_focus_id,
        }
    } else {
        // Beat 전환 없음 — 기존 stimulus 결과 반환
        StimulusResponse {
            emotions: stimulus_result.emotions,
            dominant: stimulus_result.dominant,
            mood: stimulus_result.mood,
            prompt: stimulus_result.prompt,
            trace,
            beat_changed: false,
            active_focus_id: None,
        }
    };

    // 턴 기록
    let turn_num = inner.turn_history.len() + 1;
    let label = if response.beat_changed {
        format!("Turn {}: stimulus+beat [{}] ({})", turn_num, response.active_focus_id.as_deref().unwrap_or("?"), req.npc_id)
    } else {
        format!("Turn {}: stimulus ({})", turn_num, req.npc_id)
    };
    inner.turn_history.push(TurnRecord {
        label,
        action: "stimulus".into(),
        request: serde_json::to_value(&req).unwrap_or_default(),
        response: serde_json::to_value(&response).unwrap_or_default(),
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
    let response = service.generate_guide(req)?;
    Ok(Json(response))
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
        label: format!("Turn {}: after_dialogue ({}→{})", turn_num, req.npc_id, req.partner_id),
        action: "after_dialogue".into(),
        request: serde_json::to_value(&req).unwrap_or_default(),
        response: serde_json::to_value(&response).unwrap_or_default(),
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
    let analyzer = state.analyzer.as_ref()
        .ok_or_else(|| AppError::NotImplemented("embed feature가 비활성 상태입니다".into()))?;

    let mut analyzer = analyzer.lock().await;
    let pad = analyzer.analyze(&req.utterance)
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

#[derive(Serialize)]
pub struct SceneInfo {
    pub has_scene: bool,
    pub npc_id: Option<String>,
    pub partner_id: Option<String>,
    pub active_focus_id: Option<String>,
    pub focuses: Vec<FocusInfo>,
}

#[derive(Serialize)]
pub struct FocusInfo {
    pub id: String,
    pub description: String,
    pub is_active: bool,
    pub trigger_display: String,
}

/// GET /api/scene-info — 현재 Scene Focus 상태 조회
pub async fn get_scene_info(State(state): State<AppState>) -> Json<SceneInfo> {
    let inner = state.inner.read().await;

    if inner.scene_focuses.is_empty() {
        return Json(SceneInfo {
            has_scene: false,
            npc_id: None,
            partner_id: None,
            active_focus_id: None,
            focuses: vec![],
        });
    }

    let active_id = inner.active_focus_id.as_deref();
    let focuses = inner.scene_focuses.iter().map(|f| {
        let trigger_display = match &f.trigger {
            npc_mind::domain::emotion::FocusTrigger::Initial => "initial".to_string(),
            npc_mind::domain::emotion::FocusTrigger::Conditions(or_groups) => {
                or_groups.iter().map(|and_group| {
                    and_group.iter().map(|c| {
                        let threshold = match c.threshold {
                            npc_mind::domain::emotion::ConditionThreshold::Below(v) => format!("< {:.1}", v),
                            npc_mind::domain::emotion::ConditionThreshold::Above(v) => format!("> {:.1}", v),
                            npc_mind::domain::emotion::ConditionThreshold::Absent => "absent".to_string(),
                        };
                        format!("{:?} {}", c.emotion, threshold)
                    }).collect::<Vec<_>>().join(" AND ")
                }).collect::<Vec<_>>().join(" OR ")
            }
        };
        FocusInfo {
            id: f.id.clone(),
            description: f.description.clone(),
            is_active: active_id == Some(f.id.as_str()),
            trigger_display,
        }
    }).collect();

    Json(SceneInfo {
        has_scene: true,
        npc_id: inner.scene_npc_id.clone(),
        partner_id: inner.scene_partner_id.clone(),
        active_focus_id: inner.active_focus_id.clone(),
        focuses,
    })
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
    Json(inner.current_situation.clone().unwrap_or(serde_json::Value::Null))
}

/// PUT /api/situation — 상황 설정 패널 상태 저장
pub async fn put_situation(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> StatusCode {
    let mut inner = state.inner.write().await;
    inner.current_situation = Some(body);
    StatusCode::OK
}

// ---------------------------------------------------------------------------
// 저장/로드
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct SaveRequest {
    pub path: String,
}

/// POST /api/save — JSON 파일로 저장
pub async fn save_state(
    State(state): State<AppState>,
    Json(req): Json<SaveRequest>,
) -> Result<StatusCode, AppError> {
    let inner = state.inner.read().await;
    inner.save_to_file(std::path::Path::new(&req.path))
        .map_err(|e| AppError::Internal(e))?;
    Ok(StatusCode::OK)
}

/// POST /api/load — JSON 파일에서 로드 (scene 필드가 있으면 자동 Focus 등록)
pub async fn load_state(
    State(state): State<AppState>,
    Json(req): Json<SaveRequest>,
) -> Result<StatusCode, AppError> {
    let mut loaded = StateInner::load_from_file(std::path::Path::new(&req.path))
        .map_err(|e| AppError::Internal(e))?;

    // scene 필드가 있으면 Focus 자동 등록
    if let Some(ref scene_val) = loaded.scene {
        if let Ok(scene_req) = serde_json::from_value::<SceneRequest>(scene_val.clone()) {
            let repo = AppStateRepository { inner: &mut loaded };
            let focuses: Vec<npc_mind::domain::emotion::SceneFocus> = scene_req.focuses.iter()
                .filter_map(|f| f.to_domain(&repo, &scene_req.npc_id, &scene_req.partner_id).ok())
                .collect();
            drop(repo);

            // Initial Focus 자동 appraise
            let initial_focus = focuses.iter().find(|f| {
                matches!(f.trigger, npc_mind::domain::emotion::FocusTrigger::Initial)
            });

            if let Some(focus) = initial_focus {
                if let Ok(situation) = focus.to_situation() {
                    if let Some(npc_profile) = loaded.npcs.get(&scene_req.npc_id) {
                        let npc = npc_profile.to_npc();
                        if let Some(rel_data) = loaded.find_relationship(&scene_req.npc_id, &scene_req.partner_id) {
                            let rel = rel_data.to_relationship();
                            let emotion_state = AppraisalEngine::appraise(npc.personality(), &situation, &rel);
                            loaded.emotions.insert(scene_req.npc_id.clone(), emotion_state);
                        }
                    }
                }
                loaded.active_focus_id = Some(focus.id.clone());
            }

            // Initial Focus에서 current_situation 자동 생성 (UI 폼 복원용)
            let initial_input = scene_req.focuses.iter().find(|f| f.trigger.is_none());
            if let Some(fi) = initial_input {
                let mut sit = serde_json::Map::new();
                sit.insert("desc".into(), serde_json::json!(fi.description));
                sit.insert("npcId".into(), serde_json::json!(scene_req.npc_id));
                sit.insert("partnerId".into(), serde_json::json!(scene_req.partner_id));

                // Event
                if let Some(ref ev) = fi.event {
                    sit.insert("hasEvent".into(), serde_json::json!(true));
                    sit.insert("evDesc".into(), serde_json::json!(ev.description));
                    sit.insert("evSelf".into(), serde_json::json!(ev.desirability_for_self));
                    sit.insert("hasOther".into(), serde_json::json!(ev.other.is_some()));
                    if let Some(ref o) = ev.other {
                        sit.insert("otherTarget".into(), serde_json::json!(o.target_id));
                        sit.insert("otherD".into(), serde_json::json!(o.desirability));
                    }
                    sit.insert("prospect".into(), serde_json::json!(ev.prospect));
                } else {
                    sit.insert("hasEvent".into(), serde_json::json!(false));
                }

                // Action
                if let Some(ref ac) = fi.action {
                    sit.insert("hasAction".into(), serde_json::json!(true));
                    sit.insert("acDesc".into(), serde_json::json!(ac.description));
                    sit.insert("agentId".into(), serde_json::json!(ac.agent_id));
                    sit.insert("pw".into(), serde_json::json!(ac.praiseworthiness));
                } else {
                    sit.insert("hasAction".into(), serde_json::json!(false));
                }

                // Object
                if let Some(ref obj) = fi.object {
                    sit.insert("hasObject".into(), serde_json::json!(true));
                    sit.insert("objTarget".into(), serde_json::json!(obj.target_id));
                    sit.insert("objAp".into(), serde_json::json!(obj.appealingness));
                } else {
                    sit.insert("hasObject".into(), serde_json::json!(false));
                }

                loaded.current_situation = Some(serde_json::Value::Object(sit));
            }

            loaded.scene_npc_id = Some(scene_req.npc_id);
            loaded.scene_partner_id = Some(scene_req.partner_id);
            loaded.scene_focuses = focuses;
        }
    }

    let mut inner = state.inner.write().await;
    *inner = loaded;
    Ok(StatusCode::OK)
}


// ---------------------------------------------------------------------------
// 시나리오 목록 (data/ 폴더 스캔)
// ---------------------------------------------------------------------------

/// GET /api/scenarios — data/ 폴더에서 scenario.json 파일 목록 반환
pub async fn list_scenarios() -> Json<Vec<ScenarioInfo>> {
    let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let mut scenarios = Vec::new();
    scan_scenarios(&data_dir, &data_dir, &mut scenarios);
    scenarios.sort_by(|a, b| a.path.cmp(&b.path));
    Json(scenarios)
}

#[derive(Serialize)]
pub struct ScenarioInfo {
    /// data/ 기준 상대 경로 (슬래시 구분)
    pub path: String,
    /// 표시용 이름 (폴더 구조에서 추출)
    pub label: String,
}

fn scan_scenarios(base: &std::path::Path, dir: &std::path::Path, out: &mut Vec<ScenarioInfo>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_scenarios(base, &path, out);
            } else if path.file_name().map(|f| f == "scenario.json").unwrap_or(false) {
                if let Ok(rel) = path.parent().unwrap_or(&path).strip_prefix(base) {
                    let rel_str = rel.to_string_lossy().replace('\\', "/");
                    let label = rel_str.replace('/', " / ");
                    out.push(ScenarioInfo {
                        path: rel_str,
                        label,
                    });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 헬퍼: EmotionState → 응답 DTO 변환
// ---------------------------------------------------------------------------

fn build_emotion_response(
    npc: &Npc,
    emotion_state: &EmotionState,
    situation_desc: Option<String>,
    relationship: Option<&npc_mind::domain::relationship::Relationship>,
    trace: Vec<String>,
) -> AppraiseResponse {
    use npc_mind::domain::guide::ActingGuide;
    use npc_mind::ports::GuideFormatter;
    use npc_mind::presentation::korean::KoreanFormatter;

    let guide = ActingGuide::build(npc, emotion_state, situation_desc, relationship);
    let formatter = KoreanFormatter::new();
    let prompt = formatter.format_prompt(&guide);

    let emotions: Vec<EmotionOutput> = emotion_state.emotions().iter()
        .map(EmotionOutput::from_emotion).collect();
    let dominant = emotion_state.dominant().map(|e| EmotionOutput::from_emotion(&e));

    let mood = emotion_state.overall_valence();

    AppraiseResponse {
        emotions,
        dominant,
        mood,
        prompt,
        trace,
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

    // Focus 목록을 도메인으로 변환
    let repo = AppStateRepository { inner: &mut *inner };
    let focuses: Vec<npc_mind::domain::emotion::SceneFocus> = req.focuses.iter()
        .map(|f| f.to_domain(&repo, &req.npc_id, &req.partner_id))
        .collect::<Result<Vec<_>, _>>()?;
    drop(repo);

    let focus_count = focuses.len();

    // Initial trigger인 Focus 찾기
    let initial_focus = focuses.iter().find(|f| {
        matches!(f.trigger, npc_mind::domain::emotion::FocusTrigger::Initial)
    });

    let mut initial_appraise = None;
    let mut active_focus_id = None;

    // Initial Focus가 있으면 자동 appraise
    if let Some(focus) = initial_focus {
        let situation = focus.to_situation()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let npc = inner.npcs.get(&req.npc_id)
            .ok_or_else(|| AppError::Internal(format!("NPC '{}' not found", req.npc_id)))?
            .to_npc();

        let rel = inner.find_relationship(&req.npc_id, &req.partner_id)
            .ok_or_else(|| AppError::Internal(format!("Relationship not found")))?
            .to_relationship();

        collector.take_entries();
        let emotion_state = AppraisalEngine::appraise(npc.personality(), &situation, &rel);
        let trace = collector.take_entries();

        let response = build_emotion_response(&npc, &emotion_state, Some(focus.description.clone()), Some(&rel), trace);

        inner.emotions.insert(req.npc_id.clone(), emotion_state);

        // 턴 기록
        let turn_num = inner.turn_history.len() + 1;
        inner.turn_history.push(TurnRecord {
            label: format!("Turn {}: scene/appraise [{}] ({}→{})", turn_num, focus.id, req.npc_id, req.partner_id),
            action: "scene".into(),
            request: serde_json::to_value(&req).unwrap_or_default(),
            response: serde_json::to_value(&response).unwrap_or_default(),
        });

        active_focus_id = Some(focus.id.clone());
        initial_appraise = Some(response);
    }

    // Scene 상태 저장
    inner.scene_focuses = focuses;
    inner.scene_npc_id = Some(req.npc_id.clone());
    inner.scene_partner_id = Some(req.partner_id.clone());

    Ok(Json(SceneResponse {
        focus_count,
        initial_appraise,
        active_focus_id,
    }))
}
