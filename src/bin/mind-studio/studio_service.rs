use crate::events::StateEvent;
use crate::state::{AppState, StateInner, TurnRecord, FORMAT_SCENARIO, FORMAT_RESULT};
use npc_mind::application::dto::*;

#[cfg(feature = "chat")]
use npc_mind::application::dialogue_test_service::{
    ChatStartRequest, ChatStartResponse, ChatTurnRequest, ChatTurnResponse, ChatEndRequest, ChatEndResponse,
};
use npc_mind::application::mind_service::MindService;
use npc_mind::application::situation_service::SituationService;
use crate::handlers::AppError;
use crate::repository::AppStateRepository;
use serde::Serialize;

/// Mind Studio 전용 비즈니스 로직 서비스
pub struct StudioService;

impl StudioService {
    /// 턴 기록을 저장하는 공통 헬퍼 메서드
    fn record_turn(
        inner: &mut StateInner,
        label_prefix: &str,
        action: &str,
        request: &impl Serialize,
        response: &impl Serialize,
        llm_model: Option<npc_mind::ports::LlmModelInfo>,
    ) {
        let turn_num = inner.turn_history.len() + 1;
        inner.turn_history.push(TurnRecord {
            label: format!("Turn {}: {}", turn_num, label_prefix),
            action: action.to_string(),
            request: serde_json::to_value(request).unwrap_or_default(),
            response: serde_json::to_value(response).unwrap_or_default(),
            llm_model,
        });
    }

    /// 상황 평가 및 프롬프트 생성 로직
    pub async fn perform_appraise(
        state: &AppState,
        req: AppraiseRequest,
    ) -> Result<AppraiseResponse, AppError> {
        let response = {
            let mut inner = state.inner.write().await;
            let collector = state.collector.clone();

            let mut service = MindService::new(AppStateRepository { inner: &mut *inner });

            let result = service.appraise(
                req.clone(),
                || {
                    collector.take_entries();
                },
                || collector.take_entries(),
            )?;

            let fmt = state.formatter.read().await;
            let response = result.format(&**fmt);

            // 턴 기록 통합 저장
            Self::record_turn(
                &mut *inner,
                &format!("appraise ({}→{})", req.npc_id, req.partner_id),
                "appraise",
                &req,
                &response,
                None,
            );

            inner.scenario_modified = true;
            response
        };
        state.emit(StateEvent::Appraised);
        state.emit(StateEvent::HistoryChanged);
        Ok(response)
    }

    /// PAD 자극 적용 로직
    pub async fn perform_stimulus(
        state: &AppState,
        req: StimulusRequest,
    ) -> Result<StimulusResponse, AppError> {
        let response = {
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

            let fmt = state.formatter.read().await;
            let response = result.format(&**fmt);

            // 레이블 결정
            let label = if response.beat_changed {
                // Beat 전환 시 스크립트 커서 리셋
                inner.script_cursor = 0;
                format!("stimulus+beat [{}] ({})", response.active_focus_id.as_deref().unwrap_or("?"), req.npc_id)
            } else {
                format!("stimulus ({})", req.npc_id)
            };

            // 턴 기록 통합 저장
            Self::record_turn(&mut *inner, &label, "stimulus", &req, &response, None);
            response
        };
        state.emit(StateEvent::StimulusApplied);
        state.emit(StateEvent::HistoryChanged);
        Ok(response)
    }

    /// 대화 종료 후 관계 갱신 로직
    pub async fn perform_after_dialogue(
        state: &AppState,
        req: AfterDialogueRequest,
    ) -> Result<AfterDialogueResponse, AppError> {
        let response = {
            let mut inner = state.inner.write().await;
            let mut service = MindService::new(AppStateRepository { inner: &mut *inner });

            let response = service.after_dialogue(req.clone())?;

            // 턴 기록 통합 저장
            Self::record_turn(
                &mut *inner,
                &format!("after_dialogue ({}→{})", req.npc_id, req.partner_id),
                "after_dialogue",
                &req,
                &response,
                None,
            );
            response
        };
        state.emit(StateEvent::AfterDialogue);
        state.emit(StateEvent::HistoryChanged);
        Ok(response)
    }

    /// 시나리오 파일 목록 스캔
    pub fn list_scenarios() -> Vec<ScenarioInfo> {
        let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        let mut scenarios = Vec::new();
        Self::scan_scenarios(&data_dir, &data_dir, &mut scenarios);
        scenarios.sort_by(|a, b| a.path.cmp(&b.path));
        scenarios
    }

    fn scan_scenarios(base: &std::path::Path, dir: &std::path::Path, out: &mut Vec<ScenarioInfo>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::scan_scenarios(base, &path, out);
                    continue;
                }
                if !path.extension().map(|e| e == "json").unwrap_or(false) {
                    continue;
                }
                let val = match std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                {
                    Some(v) => v,
                    None => continue,
                };
                let format_str = match val.get("format").and_then(|f| f.as_str()) {
                    Some(f) => f,
                    None => continue,
                };

                let has_results = if format_str == FORMAT_RESULT {
                    true
                } else if format_str == FORMAT_SCENARIO {
                    false
                } else {
                    continue;
                };

                if let Ok(rel) = path.strip_prefix(base) {
                    let rel_str = rel.to_string_lossy().replace('\\', "/");
                    let label = rel_str.trim_end_matches(".json").replace('/', " / ");
                    out.push(ScenarioInfo {
                        path: rel_str,
                        label,
                        has_results,
                    });
                }
            }
        }
    }

    /// 결과 저장 폴더 경로 계산
    pub async fn get_save_dir(state: &AppState) -> Result<SaveDirInfo, AppError> {
        let inner = state.inner.read().await;
        let loaded = inner
            .loaded_path
            .as_deref()
            .ok_or_else(|| AppError::Internal("로드된 시나리오가 없습니다".into()))?;

        let p = std::path::Path::new(loaded);
        let parent = p.parent().unwrap_or(std::path::Path::new("data"));
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("scenario");
        let result_dir = parent.join(stem);

        std::fs::create_dir_all(&result_dir)
            .map_err(|e| AppError::Internal(format!("폴더 생성 실패: {}", e)))?;

        let has_existing_results = result_dir.is_dir()
            && std::fs::read_dir(&result_dir).ok().map(|entries| {
                entries.flatten().any(|e| {
                    e.path().extension().map(|ext| ext == "json").unwrap_or(false)
                })
            }).unwrap_or(false);

        Ok(SaveDirInfo {
            dir: result_dir.to_string_lossy().replace('\\', "/"),
            loaded_path: loaded.to_string(),
            scenario_name: inner.scenario.name.clone(),
            scenario_modified: inner.scenario_modified,
            has_turn_history: !inner.turn_history.is_empty(),
            has_existing_results,
        })
    }

    /// 시나리오 데이터를 상태에 주입 및 초기화
    pub fn load_scene_into_state(loaded: &mut StateInner, scene_req: &SceneRequest) {
        let repo = AppStateRepository { inner: loaded };
        let focuses: Vec<npc_mind::domain::emotion::SceneFocus> = scene_req
            .focuses
            .iter()
            .filter_map(|f| {
                let ctx = SituationService::resolve_focus_context(
                    &repo, f, &scene_req.npc_id, &scene_req.partner_id,
                );
                f.to_domain(ctx.event_other_modifiers, ctx.action_agent_modifiers, ctx.object_description, &scene_req.npc_id).ok()
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

        let initial_input = scene_req.focuses.iter().find(|f| f.trigger.is_none());
        if let Some(fi) = initial_input {
            loaded.current_situation = Some(serde_json::Value::Object(Self::build_situation_map(
                fi,
                &scene_req.npc_id,
                &scene_req.partner_id,
            )));
        }
    }

    fn build_situation_map(
        fi: &SceneFocusInput,
        npc_id: &str,
        partner_id: &str,
    ) -> serde_json::Map<String, serde_json::Value> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct SituationFormData {
            desc: String, npc_id: String, partner_id: String, has_event: bool, 
            ev_desc: Option<String>, ev_self: Option<f32>, has_other: Option<bool>, 
            other_target: Option<String>, other_d: Option<f32>, prospect: Option<String>, 
            has_action: bool, ac_desc: Option<String>, agent_id: Option<String>, 
            pw: Option<f32>, has_object: bool, obj_target: Option<String>, obj_ap: Option<f32>,
        }
        let form = SituationFormData {
            desc: fi.description.clone(), npc_id: npc_id.to_string(), partner_id: partner_id.to_string(),
            has_event: fi.event.is_some(), ev_desc: fi.event.as_ref().map(|e| e.description.clone()),
            ev_self: fi.event.as_ref().map(|e| e.desirability_for_self),
            has_other: fi.event.as_ref().map(|e| e.other.is_some()),
            other_target: fi.event.as_ref().and_then(|e| e.other.as_ref().map(|o| o.target_id.clone())),
            other_d: fi.event.as_ref().and_then(|e| e.other.as_ref().map(|o| o.desirability)),
            prospect: fi.event.as_ref().and_then(|e| e.prospect.clone()),
            has_action: fi.action.is_some(), ac_desc: fi.action.as_ref().map(|a| a.description.clone()),
            agent_id: fi.action.as_ref().and_then(|a| a.agent_id.clone()),
            pw: fi.action.as_ref().map(|a| a.praiseworthiness),
            has_object: fi.object.is_some(), obj_target: fi.object.as_ref().map(|o| o.target_id.clone()),
            obj_ap: fi.object.as_ref().map(|o| o.appealingness),
        };
        match serde_json::to_value(form) {
            Ok(serde_json::Value::Object(map)) => map,
            _ => serde_json::Map::new(),
        }
    }

    // ---------------------------------------------------------------------------
    // Chat: LLM 대화 비즈니스 로직
    // ---------------------------------------------------------------------------

    #[cfg(feature = "chat")]
    pub async fn perform_chat_start(
        state: &AppState,
        req: ChatStartRequest,
    ) -> Result<ChatStartResponse, AppError> {
        let chat_port = state.chat.as_ref().ok_or_else(|| AppError::NotImplemented("chat feature가 비활성입니다.".into()))?;
        
        let mut inner = state.inner.write().await;
        let collector = state.collector.clone();
        
        // 1. NPC 정보 조회 및 파라미터 유도
        let npc_profile = inner.npcs.get(&req.appraise.npc_id).ok_or_else(|| AppError::Internal(format!("NPC {}를 찾을 수 없습니다", req.appraise.npc_id)))?;
        let (temp, top_p) = npc_profile.derive_llm_parameters();

        let mut service = MindService::new(AppStateRepository { inner: &mut *inner });
        // 이전 세션의 Beat 전환으로 인한 stale active_focus_id 초기화
        // (같은 시나리오로 여러 번 dialogue_start를 호출할 때 Beat 버그 방지)
        service.reset_scene_to_initial_focus();
        let result = service.appraise(req.appraise.clone(), || { collector.take_entries(); }, || collector.take_entries())?;
        let fmt = state.formatter.read().await;
        let response = result.format(&**fmt);
        
        // dialogue_start 시점에 LLM 서버에서 모델명을 재감지한다.
        // 서버 기동 이후 모델이 교체된 경우에도 정확한 모델명을 반영한다.
        let mut llm_model_info = if let Some(ref detector) = state.llm_detector {
            match detector.refresh_model_info().await {
                Ok(refreshed) => refreshed,
                Err(e) => {
                    tracing::warn!("LLM 모델 재감지 실패 ({}), 기존 정보 사용", e);
                    state.llm_info.as_ref().map(|i| i.get_model_info()).unwrap_or_default()
                }
            }
        } else {
            state.llm_info.as_ref().map(|i| i.get_model_info()).unwrap_or_default()
        };
        llm_model_info.temperature = Some(temp);
        llm_model_info.top_p = Some(top_p);
        if llm_model_info.max_tokens.is_none() {
            llm_model_info.max_tokens = Some(768);
        }
        
        chat_port.start_session(&req.session_id, &response.prompt, Some(llm_model_info.clone()))
            .await
            .map_err(|e: npc_mind::ports::ConversationError| AppError::Internal(e.to_string()))?;
        
        // save_dir 계산 (loaded_path가 있으면)
        let save_dir = inner.loaded_path.as_deref().map(|loaded| {
            let p = std::path::Path::new(loaded);
            let parent = p.parent().unwrap_or(std::path::Path::new("data"));
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("scenario");
            let result_dir = parent.join(stem);
            let _ = std::fs::create_dir_all(&result_dir);
            result_dir.to_string_lossy().replace('\\', "/")
        });

        // 대화 시작 시 스크립트 커서 초기화
        inner.script_cursor = 0;

        // 턴 기록 통합 저장
        Self::record_turn(&mut *inner, &format!("chat/start ({})", req.session_id), "chat_start", &req, &response, Some(llm_model_info.clone()));

        drop(inner);
        state.emit(StateEvent::ChatStarted);
        state.emit(StateEvent::HistoryChanged);
        Ok(ChatStartResponse { session_id: req.session_id, appraise: response, llm_model_info: Some(llm_model_info), save_dir })
    }

    /// 수동 PAD 입력 또는 임베딩 분석기를 통해 PAD 값을 해석합니다.
    ///
    /// listener_perspective Converter가 주입되어 있고 analyzer 임베딩이 가용하면
    /// 화자 PAD를 청자 PAD로 변환한 결과를 반환합니다 (Phase 7 Step 5).
    /// pad_hint 경로는 임베딩이 없으므로 변환 미발동 — 사용자 입력 PAD를 그대로 사용합니다.
    /// 변환 실패 시 화자 PAD fallback (silent failure 방지를 위해 warn 로그).
    #[cfg(feature = "chat")]
    async fn resolve_pad(
        state: &AppState,
        req: &ChatTurnRequest,
    ) -> Option<(f32, f32, f32)> {
        if let Some(ref pad_input) = req.pad {
            tracing::debug!("대화 턴: 수동 PAD 입력 사용 (P: {:.2}, A: {:.2}, D: {:.2})", pad_input.pleasure, pad_input.arousal, pad_input.dominance);
            return Some((pad_input.pleasure, pad_input.arousal, pad_input.dominance));
        }
        let analyzer = state.analyzer.as_ref()?;
        let mut analyzer = analyzer.lock().await;
        tracing::debug!("대화 턴: 임베딩 분석 시작 (텍스트: \"{}\")", req.utterance);
        let (speaker_pad, embedding) = match analyzer.analyze_with_embedding(&req.utterance) {
            Ok(pair) => pair,
            Err(e) => {
                tracing::error!("대화 턴: 임베딩 분석 실패: {:?}", e);
                return None;
            }
        };
        tracing::debug!(
            "대화 턴: 화자 PAD (P: {:.3}, A: {:.2}, D: {:.2})",
            speaker_pad.pleasure,
            speaker_pad.arousal,
            speaker_pad.dominance
        );
        let final_pad = Self::convert_to_listener_pad(state, &req.utterance, speaker_pad, embedding.as_deref());
        Some((final_pad.pleasure, final_pad.arousal, final_pad.dominance))
    }

    /// 화자 PAD → 청자 관점 PAD 변환 (Phase 7).
    /// converter 미주입 또는 임베딩 부재 시 화자 PAD를 그대로 반환.
    #[cfg(all(feature = "chat", feature = "listener_perspective"))]
    fn convert_to_listener_pad(
        state: &AppState,
        utterance: &str,
        speaker_pad: npc_mind::domain::pad::Pad,
        embedding: Option<&[f32]>,
    ) -> npc_mind::domain::pad::Pad {
        let (Some(converter), Some(emb)) = (state.converter.as_ref(), embedding) else {
            return speaker_pad;
        };
        match converter.convert(utterance, &speaker_pad, emb) {
            Ok(result) => {
                tracing::debug!(
                    "대화 턴: listener PAD 변환 (P: {:.3}, A: {:.2}, D: {:.2}, sign={:?}, magnitude={:?})",
                    result.listener_pad.pleasure,
                    result.listener_pad.arousal,
                    result.listener_pad.dominance,
                    result.meta.sign,
                    result.meta.magnitude
                );
                result.listener_pad
            }
            Err(e) => {
                tracing::warn!(
                    error = ?e,
                    utterance = utterance,
                    "listener-perspective conversion failed; falling back to speaker PAD"
                );
                speaker_pad
            }
        }
    }

    /// listener_perspective feature off 빌드 — 화자 PAD를 그대로 반환.
    #[cfg(all(feature = "chat", not(feature = "listener_perspective")))]
    fn convert_to_listener_pad(
        _state: &AppState,
        _utterance: &str,
        speaker_pad: npc_mind::domain::pad::Pad,
        _embedding: Option<&[f32]>,
    ) -> npc_mind::domain::pad::Pad {
        speaker_pad
    }

    /// PAD 값으로 stimulus를 적용하고 포맷된 응답을 반환합니다.
    #[cfg(feature = "chat")]
    fn apply_stimulus_with_pad(
        inner: &mut StateInner,
        collector: &crate::trace_collector::AppraisalCollector,
        formatter: &dyn npc_mind::ports::GuideFormatter,
        req: &ChatTurnRequest,
        pad: (f32, f32, f32),
    ) -> Result<StimulusResponse, AppError> {
        let stim_req = StimulusRequest {
            npc_id: req.npc_id.clone(),
            partner_id: req.partner_id.clone(),
            pleasure: pad.0,
            arousal: pad.1,
            dominance: pad.2,
            situation_description: req.situation_description.clone(),
        };
        let mut service = MindService::new(AppStateRepository { inner });
        let result = service.apply_stimulus(stim_req, || { collector.take_entries(); }, || collector.take_entries())?;
        Ok(result.format(formatter))
    }

    /// 대화 응답 수신 후 후속 처리 (PAD 분석, 심리 자극, 기록 저장)
    #[cfg(feature = "chat")]
    pub async fn process_chat_turn_result(
        state: &AppState,
        req: &ChatTurnRequest,
        npc_response: String,
    ) -> Result<(Option<StimulusResponse>, bool), AppError> {
        let chat_port = state.chat.as_ref().ok_or_else(|| AppError::NotImplemented("chat feature가 비활성입니다.".into()))?;

        let pad = Self::resolve_pad(state, req).await;

        let mut inner = state.inner.write().await;
        let fmt = state.formatter.read().await;
        let (stim_resp, changed) = if let Some(pad) = pad {
            let resp = Self::apply_stimulus_with_pad(
                &mut *inner, &state.collector, &**fmt, req, pad,
            )?;
            let changed = resp.beat_changed;
            if changed {
                chat_port.update_system_prompt(&req.session_id, &resp.prompt)
                    .await
                    .map_err(|e: npc_mind::ports::ConversationError| AppError::Internal(e.to_string()))?;
                // Beat 전환 시 스크립트 커서 리셋
                inner.script_cursor = 0;
            }
            (Some(resp), changed)
        } else {
            (None, false)
        };

        // 스크립트 커서 자동 전진: 발화가 현재 test_script의 커서 대사와 일치하면 전진
        {
            let cursor = inner.script_cursor;
            let active_id = inner.active_focus_id.clone();
            tracing::debug!(
                "script_cursor check: cursor={}, active_focus_id={:?}, scene_focuses_len={}",
                cursor, active_id, inner.scene_focuses.len()
            );
            if let Some(focus) = active_id.as_deref().and_then(|id| inner.scene_focuses.iter().find(|f| f.id == id)) {
                if !focus.test_script.is_empty() && cursor < focus.test_script.len() {
                    if req.utterance == focus.test_script[cursor] {
                        inner.script_cursor = cursor + 1;
                        tracing::debug!("script_cursor advanced: {} → {}", cursor, cursor + 1);
                    } else {
                        tracing::debug!(
                            "script_cursor NOT advanced: utterance mismatch. expected={:?}, got={:?}",
                            &focus.test_script[cursor][..focus.test_script[cursor].floor_char_boundary(30)],
                            &req.utterance[..req.utterance.floor_char_boundary(30)]
                        );
                    }
                }
            } else {
                tracing::warn!("script_cursor: active focus not found in scene_focuses");
            }
        }

        // 턴 기록 통합 저장
        let label_suffix = if stim_resp.is_none() { " (no PAD)" } else { "" };
        let resp_val = match &stim_resp {
            Some(resp) => {
                let mut val = serde_json::to_value(resp).unwrap_or_default();
                if let serde_json::Value::Object(ref mut map) = val {
                    map.insert("npc_response".into(), serde_json::Value::String(npc_response.clone()));
                }
                val
            }
            None => serde_json::json!({ "npc_response": &npc_response }),
        };
        Self::record_turn(
            &mut *inner,
            &format!("chat/message [{}→{}]{}", req.partner_id, req.npc_id, label_suffix),
            "chat_message",
            req,
            &resp_val,
            None,
        );

        drop(inner);
        state.emit(StateEvent::ChatTurnCompleted);
        state.emit(StateEvent::HistoryChanged);
        Ok((stim_resp, changed))
    }

    #[cfg(feature = "chat")]
    pub async fn perform_chat_message(
        state: &AppState,
        req: ChatTurnRequest,
    ) -> Result<ChatTurnResponse, AppError> {
        let chat_port = state.chat.as_ref().ok_or_else(|| AppError::NotImplemented("chat feature가 비활성입니다.".into()))?;
        let chat_resp = chat_port.send_message(&req.session_id, &req.utterance)
            .await
            .map_err(|e: npc_mind::ports::ConversationError| AppError::Internal(e.to_string()))?;
        let npc_response = chat_resp.text;
        let timings = chat_resp.timings;
        let (stimulus, beat_changed) = Self::process_chat_turn_result(state, &req, npc_response.clone()).await?;
        Ok(ChatTurnResponse { npc_response, stimulus, beat_changed, timings })
    }

    #[cfg(feature = "chat")]
    pub async fn perform_chat_end(
        state: &AppState,
        req: ChatEndRequest,
    ) -> Result<ChatEndResponse, AppError> {
        let chat_port = state.chat.as_ref().ok_or_else(|| AppError::NotImplemented("chat feature가 비활성입니다.".into()))?;
        let dialogue_history = chat_port.end_session(&req.session_id)
            .await
            .map_err(|e: npc_mind::ports::ConversationError| AppError::Internal(e.to_string()))?;
        let after_dialogue = if let Some(after_req) = req.after_dialogue {
            Self::perform_after_dialogue(state, after_req).await.ok()
        } else {
            None
        };
        state.emit(StateEvent::ChatEnded);
        state.emit(StateEvent::HistoryChanged);
        Ok(ChatEndResponse { dialogue_history, after_dialogue })
    }
}

#[derive(Serialize)]
pub struct ScenarioInfo {
    pub path: String,
    pub label: String,
    pub has_results: bool,
}

#[derive(Serialize)]
pub struct SaveDirInfo {
    pub dir: String,
    pub loaded_path: String,
    pub scenario_name: String,
    pub scenario_modified: bool,
    pub has_turn_history: bool,
    pub has_existing_results: bool,
}

// ============================================================
// resolve_pad / convert_to_listener_pad 단위 테스트 (Phase 7 Step 5)
//
// DialogueAgent 통합 테스트(`tests/dialogue_converter_integration.rs`)와
// 동일한 4-시나리오 매트릭스를 Mind Studio 경로에서도 검증한다.
// 두 구현이 path-for-path로 분기되어 있어 drift 시 silent 회귀 위험이 있음.
// ============================================================
#[cfg(all(test, feature = "chat", feature = "listener_perspective"))]
mod tests {
    use super::*;
    use crate::trace_collector::AppraisalCollector;
    use npc_mind::application::dialogue_test_service::PadInput;
    use npc_mind::domain::listener_perspective::{
        ConvertMeta, ConvertPath, ConvertResult, ListenerPerspectiveConverter,
        ListenerPerspectiveError, Magnitude as LpMagnitude, Sign as LpSign,
    };
    use npc_mind::domain::pad::Pad;
    use npc_mind::ports::{EmbedError, UtteranceAnalyzer};
    use std::sync::Arc;

    /// 정해진 PAD + 임베딩을 반환하는 mock UtteranceAnalyzer
    struct ScriptedAnalyzer {
        pad: Pad,
        embedding: Option<Vec<f32>>,
    }

    impl UtteranceAnalyzer for ScriptedAnalyzer {
        fn analyze(&mut self, _utterance: &str) -> Result<Pad, EmbedError> {
            Ok(self.pad)
        }
        fn analyze_with_embedding(
            &mut self,
            _utterance: &str,
        ) -> Result<(Pad, Option<Vec<f32>>), EmbedError> {
            Ok((self.pad, self.embedding.clone()))
        }
    }

    /// 화자 pleasure 부호를 반전한 listener PAD를 반환
    struct InvertingConverter;

    impl ListenerPerspectiveConverter for InvertingConverter {
        fn convert(
            &self,
            _utterance: &str,
            speaker_pad: &Pad,
            _utterance_embedding: &[f32],
        ) -> Result<ConvertResult, ListenerPerspectiveError> {
            Ok(ConvertResult {
                listener_pad: Pad::new(
                    -speaker_pad.pleasure,
                    speaker_pad.arousal,
                    speaker_pad.dominance,
                ),
                meta: ConvertMeta {
                    path: ConvertPath::Classifier {
                        sign_margin: 0.5,
                        magnitude_margin: 0.3,
                    },
                    sign: LpSign::Invert,
                    magnitude: LpMagnitude::Normal,
                    applied_p_coef: -1.0,
                    applied_a_coef: 1.0,
                    applied_d_coef: 1.0,
                },
            })
        }
    }

    /// 항상 실패 — fallback 경로 검증용
    struct FailingConverter;

    impl ListenerPerspectiveConverter for FailingConverter {
        fn convert(
            &self,
            _utterance: &str,
            _speaker_pad: &Pad,
            _utterance_embedding: &[f32],
        ) -> Result<ConvertResult, ListenerPerspectiveError> {
            Err(ListenerPerspectiveError::Embed(
                "intentional failure".to_string(),
            ))
        }
    }

    fn make_state(
        analyzer: Option<ScriptedAnalyzer>,
        converter: Option<Arc<dyn ListenerPerspectiveConverter>>,
    ) -> AppState {
        let mut state = AppState::new(AppraisalCollector::new(), analyzer);
        if let Some(c) = converter {
            state = state.with_converter(c);
        }
        state
    }

    fn make_request(utterance: &str, pad: Option<PadInput>) -> ChatTurnRequest {
        ChatTurnRequest {
            session_id: "s1".into(),
            npc_id: "mu_baek".into(),
            partner_id: "gyo_ryong".into(),
            utterance: utterance.into(),
            pad,
            situation_description: None,
        }
    }

    /// (a) Converter 주입 + analyzer 임베딩 → 변환된 listener PAD
    #[tokio::test]
    async fn resolve_pad_with_converter_and_embedding_inverts() {
        let analyzer = ScriptedAnalyzer {
            pad: Pad::new(0.6, 0.3, 0.1),
            embedding: Some(vec![1.0, 2.0, 3.0]),
        };
        let state = make_state(Some(analyzer), Some(Arc::new(InvertingConverter)));
        let req = make_request("test utterance", None);

        let (p, a, d) = StudioService::resolve_pad(&state, &req)
            .await
            .expect("PAD 반환");

        assert!(
            (p - (-0.6)).abs() < 1e-5,
            "pleasure 변환 (speaker +0.6 → listener -0.6), 실제={p}"
        );
        assert!((a - 0.3).abs() < 1e-5, "arousal 유지");
        assert!((d - 0.1).abs() < 1e-5, "dominance 유지");
    }

    /// (b) Converter 미주입 + analyzer 사용 → speaker PAD 그대로
    #[tokio::test]
    async fn resolve_pad_without_converter_returns_speaker_pad() {
        let analyzer = ScriptedAnalyzer {
            pad: Pad::new(0.6, 0.3, 0.1),
            embedding: Some(vec![1.0, 2.0, 3.0]),
        };
        let state = make_state(Some(analyzer), None);
        let req = make_request("test utterance", None);

        let (p, a, d) = StudioService::resolve_pad(&state, &req)
            .await
            .expect("PAD 반환");

        assert!((p - 0.6).abs() < 1e-5);
        assert!((a - 0.3).abs() < 1e-5);
        assert!((d - 0.1).abs() < 1e-5);
    }

    /// (c) pad_hint 사용 → analyzer/converter 모두 우회 (pad_hint 그대로)
    #[tokio::test]
    async fn resolve_pad_pad_hint_short_circuits_conversion() {
        // analyzer는 호출되어선 안 됨 — 호출되면 (0,0,0)이 나옴
        let analyzer = ScriptedAnalyzer {
            pad: Pad::new(0.0, 0.0, 0.0),
            embedding: Some(vec![1.0, 2.0, 3.0]),
        };
        let state = make_state(Some(analyzer), Some(Arc::new(InvertingConverter)));
        let req = make_request(
            "test utterance",
            Some(PadInput {
                pleasure: 0.6,
                arousal: 0.3,
                dominance: 0.1,
            }),
        );

        let (p, _, _) = StudioService::resolve_pad(&state, &req)
            .await
            .expect("PAD 반환");

        assert!(
            (p - 0.6).abs() < 1e-5,
            "pad_hint 그대로 (변환 미발동), 실제={p}"
        );
    }

    /// (d) Converter 변환 실패 → speaker PAD fallback
    #[tokio::test]
    async fn resolve_pad_converter_failure_falls_back() {
        let analyzer = ScriptedAnalyzer {
            pad: Pad::new(0.6, 0.3, 0.1),
            embedding: Some(vec![1.0, 2.0, 3.0]),
        };
        let state = make_state(Some(analyzer), Some(Arc::new(FailingConverter)));
        let req = make_request("test utterance", None);

        let (p, a, d) = StudioService::resolve_pad(&state, &req)
            .await
            .expect("PAD 반환");

        assert!(
            (p - 0.6).abs() < 1e-5,
            "변환 실패 시 speaker PAD fallback, 실제={p}"
        );
        assert!((a - 0.3).abs() < 1e-5);
        assert!((d - 0.1).abs() < 1e-5);
    }
}
