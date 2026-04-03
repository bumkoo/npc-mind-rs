use std::sync::Arc;
use crate::state::{AppState, StateInner, TurnRecord, RelationshipData};
use npc_mind::application::dto::{
    AfterDialogueRequest, AfterDialogueResponse, AppraiseRequest, AppraiseResponse,
    StimulusRequest, StimulusResponse,
};
use npc_mind::application::mind_service::MindService;
use npc_mind::domain::personality::Npc;
use npc_mind::domain::relationship::Relationship;
use npc_mind::domain::emotion::{EmotionState, Scene};
use npc_mind::ports::{NpcWorld, EmotionStore, SceneStore};
use crate::handlers::AppError;

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
