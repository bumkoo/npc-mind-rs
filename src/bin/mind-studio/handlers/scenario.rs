use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use npc_mind::application::dto::*;
use serde::{Deserialize, Serialize};
use crate::events::StateEvent;
use crate::state::*;
use crate::studio_service::{StudioService, ScenarioInfo, SaveDirInfo};
use crate::repository::{ReadOnlyAppStateRepo};
use super::AppError;

/// list_scenarios 반환 경로(data/ 하위 상대경로)를 실제 파일 경로로 변환
fn resolve_data_path(path: &str) -> String {
    let p = std::path::Path::new(path);
    if p.is_absolute() || path.starts_with("data/") || path.starts_with("data\\") {
        return path.to_string();
    }
    format!("data/{}", path)
}

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

/// GET /api/scene-info — Scene 상태 조회 (B5.2 2/3: SceneService 직접 호출)
pub async fn get_scene_info(State(state): State<AppState>) -> Json<SceneInfoResult> {
    let inner = state.inner.read().await;
    let repo = ReadOnlyAppStateRepo { inner: &*inner };
    use npc_mind::ports::SceneStore;
    let mut info = match repo.get_scene() {
        Some(scene) => npc_mind::application::scene_service::SceneService::new().build_scene_info(&scene),
        None => SceneInfoResult {
            has_scene: false,
            npc_id: None,
            partner_id: None,
            active_focus_id: None,
            significance: None,
            focuses: vec![],
            script_cursor: None,
        },
    };
    // 스크립트 커서 주입 (도메인 서비스는 커서를 모르므로 여기서 주입)
    if info.has_scene {
        info.script_cursor = Some(inner.script_cursor);
    }
    Json(info)
}

/// POST /api/scene — Scene 시작
pub async fn scene(
    State(state): State<AppState>,
    Json(req): Json<SceneRequest>,
) -> Result<Json<SceneResponse>, AppError> {
    let response = {
        let mut inner = state.inner.write().await;
        let collector = state.collector.clone();
        collector.take_entries();
        let mut result = crate::domain_sync::dispatch_start_scene(&state, &mut *inner, req.clone()).await?;
        // initial_appraise의 trace만 우선 채움 (v1에서도 최종 response 단계만 trace 채움)
        if let Some(ref mut initial) = result.initial_appraise {
            initial.trace = collector.take_entries();
        } else {
            let _ = collector.take_entries();
        }
        let response = { let fmt = state.formatter.read().await; result.format(&**fmt) };
        if response.initial_appraise.is_some() {
            let turn_num = inner.turn_history.len() + 1;
            inner.turn_history.push(TurnRecord { label: format!("Turn {}: scene/appraise [{}] ({}→{})", turn_num, response.active_focus_id.as_deref().unwrap_or("?"), req.npc_id, req.partner_id), action: "scene".into(), request: serde_json::to_value(&req).unwrap_or_default(), response: serde_json::to_value(&response).unwrap_or_default(), llm_model: None });
        }
        response
    };
    state.emit(StateEvent::SceneStarted);
    state.emit(StateEvent::HistoryChanged);
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
    {
        let mut inner = state.inner.write().await;
        inner.current_situation = Some(body);
        inner.scenario_modified = true;
    }
    state.emit(StateEvent::SituationChanged);
    StatusCode::OK
}

/// POST /api/guide — 가이드 재생성
pub async fn guide(
    State(state): State<AppState>,
    Json(mut req): Json<GuideRequest>,
) -> Result<Json<GuideResponse>, AppError> {
    let response = {
        let mut inner = state.inner.write().await;
        if req.situation_description.is_none() {
            if let Some(ref sit) = inner.current_situation {
                req.situation_description = sit.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
            }
        }
        let result = crate::domain_sync::dispatch_generate_guide(&state, &mut *inner, req).await?;
        let fmt = state.formatter.read().await;
        result.format(&**fmt)
    };
    state.emit(StateEvent::GuideGenerated);
    Ok(Json(response))
}

/// POST /api/save
pub async fn save_state(State(state): State<AppState>, Json(req): Json<super::SaveRequest>) -> Result<Json<super::SaveResponse>, AppError> {
    let save_path = req.path.clone();
    {
        let mut inner = state.inner.write().await;
        if save_path.is_empty() { return Err(AppError::Internal("저장 경로가 비어있습니다".into())); }
        let path_obj = std::path::Path::new(&save_path);
        match req.save_type.as_deref() {
            Some("report") => {
                inner.save_report_to_file(path_obj).map_err(AppError::Internal)?;
            }
            Some("all") => {
                inner.save_to_file(path_obj, false).map_err(AppError::Internal)?;
                let report_path = path_obj.with_extension("md");
                let _ = inner.save_report_to_file(&report_path);
            }
            _ => {
                let as_scenario = match req.save_type.as_deref() { Some("scenario") => true, Some("result") => false, _ => inner.turn_history.is_empty() };
                inner.save_to_file(path_obj, as_scenario).map_err(AppError::Internal)?;
                if as_scenario { inner.scenario_modified = false; inner.loaded_path = Some(save_path.clone()); }
            }
        }
    }
    state.emit(StateEvent::ScenarioSaved);
    Ok(Json(super::SaveResponse { path: save_path }))
}

/// GET /api/save-dir
pub async fn save_dir(State(state): State<AppState>) -> Result<Json<SaveDirInfo>, AppError> {
    let info = StudioService::get_save_dir(&state).await?;
    Ok(Json(info))
}

/// `POST /api/load` 응답 — Step E3.2에서 `warnings` 필드 추가. 기존 클라이언트는
/// JSON body를 무시해도 200 OK만 보면 되므로 호환.
#[derive(Serialize)]
pub struct LoadResponse {
    /// 시드 적용 중 발생한 경고 목록 (embed feature 경로에서만 채워짐).
    #[serde(default)]
    pub warnings: Vec<String>,
    /// 적용된 Rumor 수 (embed feature 경로).
    #[serde(default)]
    pub applied_rumors: usize,
    /// 적용된 MemoryEntry 수 (embed feature 경로).
    #[serde(default)]
    pub applied_memories: usize,
}

/// POST /api/load
pub async fn load_state(State(state): State<AppState>, Json(req): Json<super::SaveRequest>) -> Result<Json<LoadResponse>, AppError> {
    let resolved = resolve_data_path(&req.path);
    let mut loaded = StateInner::load_from_file(std::path::Path::new(&resolved)).map_err(|e| AppError::Internal(e))?;
    loaded.turn_history.clear();
    loaded.loaded_path = Some(resolved);
    let scene_cfg = loaded.scene.clone();
    #[cfg(feature = "embed")]
    let seeds = loaded.scenario_seeds.clone();
    {
        let mut inner = state.inner.write().await;
        *inner = loaded;
    }
    // B5.2 (3/3): 먼저 inner에 시나리오를 붙이고, 공유 repo를 fresh하게 만든 뒤
    // dispatch_v2(StartScene) 계열 호출을 실행한다.
    state.rebuild_repo_from_inner().await;
    // Step E3.2: 시나리오 JSON의 memory/rumor 시드를 Mind Studio store에 주입.
    // embed feature 비활성 경로에서는 저장소 자체가 없으므로 스킵.
    #[cfg(feature = "embed")]
    let report = apply_scenario_seeds(&state, &seeds);
    #[cfg(not(feature = "embed"))]
    let report = SeedReport::default();
    if let Some(scene_val) = scene_cfg {
        if let Ok(scene_req) = serde_json::from_value::<SceneRequest>(scene_val) {
            StudioService::load_scene_into_state(&state, &scene_req).await;
        }
    }
    state.emit(StateEvent::ScenarioLoaded);
    Ok(Json(LoadResponse {
        warnings: report.warnings,
        applied_rumors: report.applied_rumors,
        applied_memories: report.applied_memories,
    }))
}

/// POST /api/load-result
pub async fn load_result(State(state): State<AppState>, Json(req): Json<super::SaveRequest>) -> Result<Json<super::LoadResultResponse>, AppError> {
    let resolved = resolve_data_path(&req.path);
    let mut loaded = StateInner::load_from_file(std::path::Path::new(&resolved)).map_err(|e| AppError::Internal(e))?;
    loaded.loaded_path = Some(resolved);
    let scene_cfg = loaded.scene.clone();
    let history = loaded.turn_history.clone();
    #[cfg(feature = "embed")]
    let seeds = loaded.scenario_seeds.clone();
    {
        let mut inner = state.inner.write().await;
        *inner = loaded;
    }
    state.rebuild_repo_from_inner().await;
    #[cfg(feature = "embed")]
    {
        // load_result도 동일하게 시드 적용. warnings는 LoadResultResponse 스키마가
        // turn_history만 돌려주므로 여기선 로그만 남기고 응답에 싣지 않는다 (파일 기반
        // 복원이라 작가 편집 경로가 아님).
        let _report = apply_scenario_seeds(&state, &seeds);
    }
    if let Some(scene_val) = scene_cfg {
        if let Ok(scene_req) = serde_json::from_value::<SceneRequest>(scene_val) {
            StudioService::load_scene_into_state(&state, &scene_req).await;
        }
    }
    state.emit(StateEvent::ResultLoaded);
    Ok(Json(super::LoadResultResponse { turn_history: history }))
}

/// Step E3.2 `apply_scenario_seeds` 결과 보고.
///
/// Mind Studio `load_state`가 UI 응답(`LoadResponse.warnings`)에 실어 작가에게
/// 시드 빌드/저장 실패를 가시화한다. embed feature 비활성 경로에서도 default가
/// 반환되므로 구조체는 cfg 게이트 없이 상시 컴파일.
#[derive(Debug, Default, serde::Serialize)]
pub struct SeedReport {
    pub applied_rumors: usize,
    pub applied_memories: usize,
    pub warnings: Vec<String>,
}

/// Step E3.2: ScenarioSeeds를 Mind Studio의 memory_store/rumor_store에 인덱싱.
///
/// 먼저 **두 store를 전부 비운다** (시나리오 로드 = fresh slate). 그 뒤 시드를 주입.
/// 개별 실패는 `SeedReport.warnings`에 수집되고 `tracing::warn!`에도 기록. clear 자체
/// 실패는 fatal — warning으로 보고하지만 seed 주입은 계속 시도 (best-effort).
///
/// `MemoryEntry.created_seq`는 시드 간 안정적 정렬을 위해 배치 내에서 1..N으로 부여.
/// 런타임 seq와 겹칠 수 있으나 id가 달라 `INSERT OR REPLACE` 충돌은 없음.
#[cfg(feature = "embed")]
fn apply_scenario_seeds(
    state: &AppState,
    seeds: &npc_mind::application::scenario_seeds::ScenarioSeeds,
) -> SeedReport {
    use npc_mind::domain::memory::MemoryScope;
    let mut report = SeedReport::default();

    // H1: 이전 시나리오 잔존물 제거. 실패는 warning + 주입 계속 (store 상태가 나빠지지만
    // seed를 완전히 누락시키는 것보단 낫다).
    if let Err(e) = state.memory_store.clear_all() {
        let msg = format!("memory_store.clear_all 실패: {e}");
        tracing::warn!("{msg}");
        report.warnings.push(msg);
    }
    if let Err(e) = state.rumor_store.clear_all() {
        let msg = format!("rumor_store.clear_all 실패: {e}");
        tracing::warn!("{msg}");
        report.warnings.push(msg);
    }

    if seeds.is_empty() {
        return report;
    }

    // Rumor 시드.
    for (idx, seed) in seeds.initial_rumors.iter().enumerate() {
        let fallback = format!("{idx}");
        match seed.clone().into_rumor(&fallback) {
            Ok(rumor) => match state.rumor_store.save(&rumor) {
                Ok(()) => report.applied_rumors += 1,
                Err(e) => {
                    let msg = format!("scenario seed rumor[{idx}] 저장 실패: {e}");
                    tracing::warn!("{msg}");
                    report.warnings.push(msg);
                }
            },
            Err(e) => {
                let msg = format!("scenario seed rumor[{idx}] 빌드 실패: {e}");
                tracing::warn!("{msg}");
                report.warnings.push(msg);
            }
        }
    }

    // MemoryEntry 시드 — World → Faction → Family 순으로 created_seq 연속 부여.
    let mut seq: u64 = 1;
    let index_entry = |state: &AppState, mut entry: npc_mind::domain::memory::MemoryEntry,
                        seq: &mut u64,
                        report: &mut SeedReport,
                        label: &str| {
        entry.created_seq = *seq;
        *seq += 1;
        match state.memory_store.index(entry, None) {
            Ok(()) => report.applied_memories += 1,
            Err(e) => {
                let msg = format!("scenario seed {label} 저장 실패: {e}");
                tracing::warn!("{msg}");
                report.warnings.push(msg);
            }
        }
    };

    for (idx, seed) in seeds.world_knowledge.iter().enumerate() {
        let fallback = format!("world-{idx}");
        let entry = seed.clone().into_entry(&fallback);
        index_entry(state, entry, &mut seq, &mut report, &format!("world_knowledge[{idx}]"));
    }

    for (faction_id, entries) in &seeds.faction_knowledge {
        for (idx, seed) in entries.iter().enumerate() {
            let fallback = format!("faction-{faction_id}-{idx}");
            let scope = MemoryScope::Faction { faction_id: faction_id.clone() };
            let entry = seed.clone().into_entry(scope, &fallback);
            index_entry(
                state,
                entry,
                &mut seq,
                &mut report,
                &format!("faction_knowledge[{faction_id}][{idx}]"),
            );
        }
    }

    for (family_id, entries) in &seeds.family_facts {
        for (idx, seed) in entries.iter().enumerate() {
            let fallback = format!("family-{family_id}-{idx}");
            let scope = MemoryScope::Family { family_id: family_id.clone() };
            let entry = seed.clone().into_entry(scope, &fallback);
            index_entry(
                state,
                entry,
                &mut seq,
                &mut report,
                &format!("family_facts[{family_id}][{idx}]"),
            );
        }
    }

    report
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
    let ok = {
        let mut inner = state.inner.write().await;
        if let Some(content) = body.get("content").and_then(|v| v.as_str()) {
            inner.test_report = content.to_string();
            inner.scenario_modified = true;
            true
        } else {
            false
        }
    };
    if ok {
        state.emit(StateEvent::TestReportChanged);
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
