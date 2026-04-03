use std::sync::Arc;
use crate::state::{AppState, StateInner, TurnRecord, RelationshipData, FORMAT_SCENARIO, FORMAT_RESULT};
use npc_mind::application::dto::{
    AfterDialogueRequest, AfterDialogueResponse, AppraiseRequest, AppraiseResponse,
    StimulusRequest, StimulusResponse, SceneRequest, SceneFocusInput, SceneInfoResult,
};
#[cfg(feature = "chat")]
use npc_mind::application::dialogue_test_service::{
    ChatStartRequest, ChatStartResponse, ChatTurnRequest, ChatTurnResponse, ChatEndRequest, ChatEndResponse,
};
use npc_mind::application::mind_service::{MindService};
use npc_mind::domain::personality::Npc;
use npc_mind::domain::relationship::Relationship;
use npc_mind::domain::emotion::{EmotionState, Scene};
use npc_mind::ports::{NpcWorld, EmotionStore, SceneStore};
use crate::handlers::AppError;
use serde::Serialize;

/// Mind Studio 전용 비즈니스 로직 서비스
pub struct StudioService;

impl StudioService {
    /// 상황 평가 및 프롬프트 생성 로직
    pub async fn perform_appraise(
        state: &AppState,
        req: AppraiseRequest,
    ) -> Result<AppraiseResponse, AppError> {
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

        inner.scenario_modified = true;
        Ok(response)
    }

    /// PAD 자극 적용 로직
    pub async fn perform_stimulus(
        state: &AppState,
        req: StimulusRequest,
    ) -> Result<StimulusResponse, AppError> {
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

        Ok(response)
    }

    /// 대화 종료 후 관계 갱신 로직
    pub async fn perform_after_dialogue(
        state: &AppState,
        req: AfterDialogueRequest,
    ) -> Result<AfterDialogueResponse, AppError> {
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
        let result_dir = parent.join(format!("{}_result", stem));

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
        
        // 1. NPC 정보 조회 및 파라미터 유도 (NpcProfile 메서드 사용)
        let npc_profile = inner.npcs.get(&req.appraise.npc_id).ok_or_else(|| AppError::Internal(format!("NPC {}를 찾을 수 없습니다", req.appraise.npc_id)))?;
        let (temp, top_p) = npc_profile.derive_llm_parameters();

        let mut service = MindService::new(AppStateRepository { inner: &mut *inner });
        let result = service.appraise(req.appraise.clone(), || { collector.take_entries(); }, || collector.take_entries())?;
        let response = result.format(&*state.formatter);
        
        let mut llm_model_info = state.llm_info.as_ref().map(|info| info.get_model_info()).unwrap_or_default();
        llm_model_info.temperature = Some(temp);
        llm_model_info.top_p = Some(top_p);
        
        chat_port.start_session(&req.session_id, &response.prompt, Some(llm_model_info.clone())).await.map_err(|e| AppError::Internal(e.to_string()))?;
        
        let turn_num = inner.turn_history.len() + 1;
        inner.turn_history.push(TurnRecord { label: format!("Turn {}: chat/start ({})", turn_num, req.session_id), action: "chat_start".into(), request: serde_json::to_value(&req).unwrap_or_default(), response: serde_json::to_value(&response).unwrap_or_default(), llm_model: Some(llm_model_info.clone()) });
        
        Ok(ChatStartResponse { session_id: req.session_id, appraise: response, llm_model_info: Some(llm_model_info) })
    }

    #[cfg(feature = "chat")]
    pub async fn perform_chat_message(
        state: &AppState,
        req: ChatTurnRequest,
    ) -> Result<ChatTurnResponse, AppError> {
        let chat_port = state.chat.as_ref().ok_or_else(|| AppError::NotImplemented("chat feature가 비활성입니다.".into()))?;
        let npc_response = chat_port.send_message(&req.session_id, &req.utterance).await?;
        
        let pad = if let Some(ref pad_input) = req.pad { Some((pad_input.pleasure, pad_input.arousal, pad_input.dominance)) } else if let Some(ref analyzer) = state.analyzer { let mut analyzer = analyzer.lock().await; match analyzer.analyze(&req.utterance) { Ok(p) => Some((p.pleasure, p.arousal, p.dominance)), Err(_) => None } } else { None };
        
        let (stimulus, beat_changed) = if let Some((p, a, d)) = pad {
            let stim_req = StimulusRequest { npc_id: req.npc_id.clone(), partner_id: req.partner_id.clone(), pleasure: p, arousal: a, dominance: d, situation_description: req.situation_description.clone() };
            let mut inner = state.inner.write().await;
            let collector = state.collector.clone();
            let mut service = MindService::new(AppStateRepository { inner: &mut *inner });
            let result = service.apply_stimulus(stim_req, || { collector.take_entries(); }, || collector.take_entries())?;
            let stim_resp = result.format(&*state.formatter);
            let changed = stim_resp.beat_changed;
            if changed { chat_port.update_system_prompt(&req.session_id, &stim_resp.prompt).await.map_err(|e| AppError::Internal(e.to_string()))?; }
            let turn_num = inner.turn_history.len() + 1;
            let mut resp_val = serde_json::to_value(&stim_resp).unwrap_or_default();
            if let serde_json::Value::Object(ref mut map) = resp_val { map.insert("npc_response".into(), serde_json::Value::String(npc_response.clone())); }
            inner.turn_history.push(TurnRecord { label: format!("Turn {}: chat/message [{}→{}]", turn_num, req.partner_id, req.npc_id), action: "chat_message".into(), request: serde_json::to_value(&req).unwrap_or_default(), response: resp_val, llm_model: None });
            (Some(stim_resp), changed)
        } else {
            let mut inner = state.inner.write().await;
            let turn_num = inner.turn_history.len() + 1;
            inner.turn_history.push(TurnRecord { label: format!("Turn {}: chat/message [{}→{}] (no PAD)", turn_num, req.partner_id, req.npc_id), action: "chat_message".into(), request: serde_json::to_value(&req).unwrap_or_default(), response: serde_json::json!({ "npc_response": &npc_response }), llm_model: None });
            (None, false)
        };
        Ok(ChatTurnResponse { npc_response, stimulus, beat_changed })
    }

    #[cfg(feature = "chat")]
    pub async fn perform_chat_end(
        state: &AppState,
        req: ChatEndRequest,
    ) -> Result<ChatEndResponse, AppError> {
        let chat_port = state.chat.as_ref().ok_or_else(|| AppError::NotImplemented("chat feature가 비활성입니다.".into()))?;
        let dialogue_history = chat_port.end_session(&req.session_id).await.map_err(|e| AppError::Internal(e.to_string()))?;
        let after_dialogue = if let Some(after_req) = req.after_dialogue {
            Self::perform_after_dialogue(state, after_req).await.ok()
        } else {
            None
        };
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

// ---------------------------------------------------------------------------
// Repository Wrappers (내부용)
// ---------------------------------------------------------------------------

pub struct AppStateRepository<'a> {
    pub inner: &'a mut StateInner,
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
pub struct ReadOnlyAppStateRepo<'a> {
    pub inner: &'a StateInner,
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
