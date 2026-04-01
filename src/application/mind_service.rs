use crate::domain::emotion::{
    AppraisalEngine, EmotionState, Scene, SceneFocus, Situation, StimulusEngine,
};
use crate::domain::guide::ActingGuide;
use crate::domain::pad::Pad;
use crate::domain::personality::Npc;
use crate::domain::relationship::Relationship;
use crate::domain::tuning::{BEAT_DEFAULT_SIGNIFICANCE, BEAT_MERGE_THRESHOLD};
use crate::ports::{Appraiser, StimulusProcessor};

// 저장소 포트 재노출
pub use crate::ports::{EmotionStore, MindRepository, NpcWorld, SceneStore};

use super::dto::*;

/// 오류 타입: 서비스 계층에서 발생하는 도메인 오류
#[derive(Debug, thiserror::Error)]
pub enum MindServiceError {
    #[error("NPC '{0}'를 찾을 수 없습니다.")]
    NpcNotFound(String),
    #[error("관계 '{0}↔{1}'를 찾을 수 없습니다.")]
    RelationshipNotFound(String, String),
    #[error("상황 정보가 잘못되었습니다: {0}")]
    InvalidSituation(String),
    #[error("현재 감정 상태를 찾을 수 없습니다. (먼저 평가를 실행하세요)")]
    EmotionStateNotFound,
    #[error("로케일 초기화 실패: {0}")]
    LocaleError(String),
}

/// Mind 엔진의 핵심 진입점 (Application Service)
pub struct MindService<
    R: MindRepository,
    A: Appraiser = AppraisalEngine,
    S: StimulusProcessor = StimulusEngine,
> {
    repository: R,
    appraiser: A,
    stimulus_processor: S,
}

impl<R: MindRepository> MindService<R> {
    pub fn new(repository: R) -> Self {
        Self {
            repository,
            appraiser: AppraisalEngine,
            stimulus_processor: StimulusEngine,
        }
    }
}

impl<R: MindRepository, A: Appraiser, S: StimulusProcessor> MindService<R, A, S> {
    pub fn with_engines(repository: R, appraiser: A, stimulus_processor: S) -> Self {
        Self {
            repository,
            appraiser,
            stimulus_processor,
        }
    }

    pub fn repository_mut(&mut self) -> &mut R {
        &mut self.repository
    }
    pub fn repository(&self) -> &R {
        &self.repository
    }

    // ---------------------------------------------------------------------------
    // 공통 내부 헬퍼 (중복 제거)
    // ---------------------------------------------------------------------------

    /// NPC와 관계 정보를 한 번에 조회합니다.
    fn prepare_context(
        &self,
        npc_id: &str,
        partner_id: &str,
    ) -> Result<(Npc, Relationship), MindServiceError> {
        let npc = self
            .repository
            .get_npc(npc_id)
            .ok_or_else(|| MindServiceError::NpcNotFound(npc_id.to_string()))?;

        let relationship = self
            .repository
            .get_relationship(npc_id, partner_id)
            .ok_or_else(|| {
                MindServiceError::RelationshipNotFound(npc_id.to_string(), partner_id.to_string())
            })?;

        Ok((npc, relationship))
    }

    /// 평가 엔진을 실행하고 결과를 저장한 뒤 AppraiseResult를 반환합니다.
    fn execute_appraise_workflow(
        &mut self,
        npc: &Npc,
        relationship: &Relationship,
        situation: &Situation,
        mut before_eval: impl FnMut(),
        mut after_eval: impl FnMut() -> Vec<String>,
    ) -> AppraiseResult {
        before_eval();
        let emotion_state =
            self.appraiser
                .appraise(npc.personality(), situation, &relationship.modifiers());
        let trace = after_eval();

        let result = build_appraise_result(
            npc,
            &emotion_state,
            Some(situation.description.clone()),
            Some(relationship),
            trace,
        );

        self.repository.save_emotion_state(npc.id(), emotion_state);
        result
    }

    // ---------------------------------------------------------------------------
    // 공개 API 메서드
    // ---------------------------------------------------------------------------

    /// 상황을 평가하여 감정을 생성하고 ActingGuide를 포함한 결과를 반환합니다.
    pub fn appraise(
        &mut self,
        req: AppraiseRequest,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<AppraiseResult, MindServiceError> {
        let (npc, relationship) = self.prepare_context(&req.npc_id, &req.partner_id)?;
        let situation = req
            .situation
            .to_domain(&self.repository, &req.npc_id, &req.partner_id)?;

        Ok(
            self.execute_appraise_workflow(
                &npc,
                &relationship,
                &situation,
                before_eval,
                after_eval,
            ),
        )
    }

    /// 대화 턴 중 발생한 PAD 자극을 적용하여 감정을 갱신합니다.
    pub fn apply_stimulus(
        &mut self,
        req: StimulusRequest,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<StimulusResult, MindServiceError> {
        let (npc, relationship) = self.prepare_context(&req.npc_id, &req.partner_id)?;

        let current = self
            .repository
            .get_emotion_state(&req.npc_id)
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        // 1. PAD 자극 적용 (관성)
        let pad = Pad {
            pleasure: req.pleasure,
            arousal: req.arousal,
            dominance: req.dominance,
        };
        let stimulated_state =
            self.stimulus_processor
                .apply_stimulus(npc.personality(), &current, &pad);
        self.repository
            .save_emotion_state(&req.npc_id, stimulated_state.clone());

        // 2. Scene Focus trigger 체크 → Beat 전환
        if let Some((mut scene, focus)) = self
            .repository
            .get_scene()
            .and_then(|s| s.check_trigger(&stimulated_state).cloned().map(|f| (s, f)))
        {
            return self.transition_beat(
                &req.npc_id,
                &npc,
                &relationship,
                &mut scene,
                focus,
                before_eval,
                after_eval,
            );
        }

        // 3. Beat 전환 없음
        let (emotions, dominant, mood) = build_emotion_fields(&stimulated_state);
        let guide = ActingGuide::build(
            &npc,
            &stimulated_state,
            req.situation_description.clone(),
            Some(&relationship),
        );

        Ok(StimulusResult {
            emotions,
            dominant,
            mood,
            guide,
            trace: vec![],
            beat_changed: false,
            active_focus_id: None,
        })
    }

    /// Beat 전환: 관계 갱신 → 새 Focus appraise → 감정 병합
    fn transition_beat(
        &mut self,
        npc_id: &str,
        npc: &Npc,
        relationship: &Relationship,
        scene: &mut Scene,
        focus: SceneFocus,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<StimulusResult, MindServiceError> {
        self.update_beat_relationship(scene);

        let situation = focus
            .to_situation()
            .map_err(|e| MindServiceError::InvalidSituation(e.to_string()))?;

        let previous = self
            .repository
            .get_emotion_state(npc_id)
            .unwrap_or_default();

        // 통합 워크플로우 호출 (결과는 internal_appraise가 저장함)
        let app_result =
            self.execute_appraise_workflow(npc, relationship, &situation, before_eval, after_eval);

        // 병합 처리 (이전 감정 + 새 감정)
        let new_state = self
            .repository
            .get_emotion_state(npc_id)
            .unwrap_or_default();
        let merged = EmotionState::merge_from_beat(&previous, &new_state, BEAT_MERGE_THRESHOLD);
        self.repository.save_emotion_state(npc_id, merged.clone());

        // 병합된 상태로 필드 재구성
        let (emotions, dominant, mood) = build_emotion_fields(&merged);
        let guide = ActingGuide::build(
            npc,
            &merged,
            Some(focus.description.clone()),
            Some(relationship),
        );

        let focus_id = focus.id.clone();
        scene.set_active_focus(focus_id.clone());
        self.repository.save_scene(scene.clone());

        Ok(StimulusResult {
            emotions,
            dominant,
            mood,
            guide,
            trace: app_result.trace,
            beat_changed: true,
            active_focus_id: Some(focus_id),
        })
    }

    fn update_beat_relationship(&mut self, scene: &Scene) {
        let beat_req = AfterDialogueRequest {
            npc_id: scene.npc_id().to_string(),
            partner_id: scene.partner_id().to_string(),
            praiseworthiness: Some(0.0),
            significance: Some(BEAT_DEFAULT_SIGNIFICANCE),
        };
        let _ = self.update_relationship(&beat_req);
    }

    pub fn start_scene(
        &mut self,
        req: SceneRequest,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<SceneResult, MindServiceError> {
        let focuses: Vec<SceneFocus> = req
            .focuses
            .iter()
            .map(|f| f.to_domain(&self.repository, &req.npc_id, &req.partner_id))
            .collect::<Result<Vec<_>, _>>()?;

        let focus_count = focuses.len();
        let mut scene = Scene::new(req.npc_id.clone(), req.partner_id.clone(), focuses);

        let (initial_appraise, active_focus_id) =
            self.appraise_initial_focus(&mut scene, before_eval, after_eval)?;

        self.repository.save_scene(scene);
        Ok(SceneResult {
            focus_count,
            initial_appraise,
            active_focus_id,
        })
    }

    pub fn scene_info(&self) -> SceneInfoResult {
        let Some(scene) = self.repository.get_scene() else {
            return SceneInfoResult {
                has_scene: false,
                npc_id: None,
                partner_id: None,
                active_focus_id: None,
                focuses: vec![],
            };
        };

        let active_id = scene.active_focus_id();
        let focus_infos = scene
            .focuses()
            .iter()
            .map(|f| FocusInfoItem::from_domain(f, active_id == Some(f.id.as_str())))
            .collect();

        SceneInfoResult {
            has_scene: true,
            npc_id: Some(scene.npc_id().to_string()),
            partner_id: Some(scene.partner_id().to_string()),
            active_focus_id: scene.active_focus_id().map(|s| s.to_string()),
            focuses: focus_infos,
        }
    }

    pub fn load_scene_focuses(
        &mut self,
        focuses: Vec<SceneFocus>,
        npc_id: String,
        partner_id: String,
    ) -> Result<Option<AppraiseResult>, MindServiceError> {
        let mut scene = Scene::new(npc_id, partner_id, focuses);
        let (result, _) = self.appraise_initial_focus(&mut scene, || {}, Vec::new)?;
        self.repository.save_scene(scene);
        Ok(result)
    }

    fn appraise_initial_focus(
        &mut self,
        scene: &mut Scene,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<(Option<AppraiseResult>, Option<String>), MindServiceError> {
        let npc_id = scene.npc_id();
        let partner_id = scene.partner_id();

        let Some(focus) = scene.initial_focus().cloned() else {
            return Ok((None, None));
        };

        let (npc, relationship) = self.prepare_context(npc_id, partner_id)?;
        let situation = focus
            .to_situation()
            .map_err(|e| MindServiceError::InvalidSituation(e.to_string()))?;

        let result = self.execute_appraise_workflow(
            &npc,
            &relationship,
            &situation,
            before_eval,
            after_eval,
        );
        scene.set_active_focus(focus.id.clone());

        Ok((Some(result), Some(focus.id)))
    }

    pub fn generate_guide(&self, req: GuideRequest) -> Result<GuideResult, MindServiceError> {
        let (npc, relationship) = self.prepare_context(&req.npc_id, &req.partner_id)?;
        let emotion_state = self
            .repository
            .get_emotion_state(&req.npc_id)
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        let guide = ActingGuide::build(
            &npc,
            &emotion_state,
            req.situation_description.clone(),
            Some(&relationship),
        );
        Ok(GuideResult { guide })
    }

    pub fn after_dialogue(
        &mut self,
        req: AfterDialogueRequest,
    ) -> Result<AfterDialogueResponse, MindServiceError> {
        let response = self.update_relationship(&req)?;
        self.repository.clear_emotion_state(&req.npc_id);
        self.repository.clear_scene();
        Ok(response)
    }

    pub fn after_beat(
        &mut self,
        req: AfterDialogueRequest,
    ) -> Result<AfterDialogueResponse, MindServiceError> {
        self.update_relationship(&req)
    }

    fn update_relationship(
        &mut self,
        req: &AfterDialogueRequest,
    ) -> Result<AfterDialogueResponse, MindServiceError> {
        let relationship = self
            .repository
            .get_relationship(&req.npc_id, &req.partner_id)
            .or_else(|| {
                self.repository
                    .get_relationship(&req.partner_id, &req.npc_id)
            })
            .ok_or_else(|| {
                MindServiceError::RelationshipNotFound(req.npc_id.clone(), req.partner_id.clone())
            })?;

        let emotion_state = self
            .repository
            .get_emotion_state(&req.npc_id)
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        let before = RelationshipValues {
            closeness: relationship.closeness().value(),
            trust: relationship.trust().value(),
            power: relationship.power().value(),
        };

        let significance = req.significance.unwrap_or(0.0).clamp(0.0, 1.0);
        let new_rel =
            relationship.after_dialogue(&emotion_state, req.praiseworthiness, significance);

        let after = RelationshipValues {
            closeness: new_rel.closeness().value(),
            trust: new_rel.trust().value(),
            power: new_rel.power().value(),
        };

        self.repository
            .save_relationship(&req.npc_id, &req.partner_id, new_rel);
        Ok(AfterDialogueResponse { before, after })
    }
}
