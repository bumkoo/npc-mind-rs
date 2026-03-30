use crate::domain::emotion::{AppraisalEngine, StimulusEngine, EmotionState, SceneFocus, FocusTrigger};
use crate::domain::guide::ActingGuide;
use crate::domain::pad::Pad;
use crate::domain::personality::Npc;
use crate::domain::relationship::Relationship;
use crate::domain::tuning::{BEAT_MERGE_THRESHOLD, BEAT_DEFAULT_SIGNIFICANCE};
use crate::ports::{Appraiser, StimulusProcessor};

// 저장소 포트 재노출
pub use crate::ports::{MindRepository, NpcWorld, EmotionStore, SceneStore};

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
///
/// 도메인 로직만 수행하며, 포맷팅은 하지 않습니다.
/// 포맷팅이 포함된 응답이 필요하면 `FormattedMindService`를 사용하거나
/// 반환된 `*Result`에 `.format(formatter)`를 호출하세요.
///
/// 감정 평가 엔진(`A`)과 자극 처리 엔진(`S`)을 제네릭으로 받으며,
/// 기본값으로 `AppraisalEngine`과 `StimulusEngine`이 사용됩니다.
pub struct MindService<
    R: MindRepository,
    A: Appraiser = AppraisalEngine,
    S: StimulusProcessor = StimulusEngine,
> {
    repository: R,
    appraiser: A,
    stimulus_processor: S,
}

// 기본 엔진 사용 (기존 코드 호환)
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
    /// 커스텀 엔진을 주입하여 생성합니다.
    pub fn with_engines(repository: R, appraiser: A, stimulus_processor: S) -> Self {
        Self { repository, appraiser, stimulus_processor }
    }

    /// 저장소에 대한 가변 참조를 반환합니다.
    pub fn repository_mut(&mut self) -> &mut R {
        &mut self.repository
    }

    /// 저장소에 대한 불변 참조를 반환합니다.
    pub fn repository(&self) -> &R {
        &self.repository
    }

    /// 상황을 평가하여 감정을 생성하고 ActingGuide를 포함한 결과를 반환합니다.
    pub fn appraise(
        &mut self,
        req: AppraiseRequest,
        mut before_eval: impl FnMut(),
        mut after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<AppraiseResult, MindServiceError> {
        let npc = self.repository.get_npc(&req.npc_id)
            .ok_or_else(|| MindServiceError::NpcNotFound(req.npc_id.clone()))?;

        let relationship = self.repository.get_relationship(&req.npc_id, &req.partner_id)
            .ok_or_else(|| MindServiceError::RelationshipNotFound(req.npc_id.clone(), req.partner_id.clone()))?;

        let situation = req.situation.to_domain(&self.repository, &req.npc_id, &req.partner_id)?;

        before_eval();
        let emotion_state = self.appraiser.appraise(npc.personality(), &situation, &relationship.modifiers());
        let trace = after_eval();

        let result = build_appraise_result(
            &npc, &emotion_state,
            Some(situation.description.clone()),
            Some(&relationship),
            trace,
        );

        self.repository.save_emotion_state(&req.npc_id, emotion_state);

        Ok(result)
    }

    /// 대화 턴 중 발생한 PAD 자극을 적용하여 감정을 갱신합니다.
    ///
    /// Scene Focus가 등록되어 있으면 자극 적용 후 trigger 조건을 체크하여
    /// Beat 전환을 자동으로 처리합니다.
    pub fn apply_stimulus(
        &mut self,
        req: StimulusRequest,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<StimulusResult, MindServiceError> {
        let npc = self.repository.get_npc(&req.npc_id)
            .ok_or_else(|| MindServiceError::NpcNotFound(req.npc_id.clone()))?;

        let relationship = self.repository.get_relationship(&req.npc_id, &req.partner_id)
            .ok_or_else(|| MindServiceError::RelationshipNotFound(req.npc_id.clone(), req.partner_id.clone()))?;

        let current = self.repository.get_emotion_state(&req.npc_id)
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        // 1. PAD 자극 적용 (관성)
        let pad = Pad { pleasure: req.pleasure, arousal: req.arousal, dominance: req.dominance };
        let stimulated_state = self.stimulus_processor.apply_stimulus(npc.personality(), &current, &pad);
        self.repository.save_emotion_state(&req.npc_id, stimulated_state.clone());

        // 2. Scene Focus trigger 체크 → Beat 전환
        let focuses = self.repository.get_scene_focuses().to_vec();
        if let Some(focus) = focuses.iter().find(|f| f.trigger.is_met(&stimulated_state)) {
            return self.transition_beat(
                &req.npc_id, &npc, &relationship, focus, before_eval, after_eval,
            );
        }

        // 3. Beat 전환 없음 — 기존 stimulus 결과 반환
        let (emotions, dominant, mood) = build_emotion_fields(&stimulated_state);
        let guide = ActingGuide::build(&npc, &stimulated_state, req.situation_description.clone(), Some(&relationship));

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
        focus: &SceneFocus,
        mut before_eval: impl FnMut(),
        mut after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<StimulusResult, MindServiceError> {
        // 관계 갱신 (감정 유지)
        self.update_beat_relationship();

        // 새 Focus appraise → merge
        let situation = focus.to_situation()
            .map_err(|e| MindServiceError::InvalidSituation(e.to_string()))?;

        let previous = self.repository.get_emotion_state(npc_id)
            .unwrap_or_else(EmotionState::new);

        before_eval();
        let new_state = self.appraiser.appraise(npc.personality(), &situation, &relationship.modifiers());
        let beat_trace = after_eval();

        let merged = EmotionState::merge_from_beat(&previous, &new_state, BEAT_MERGE_THRESHOLD);
        self.repository.save_emotion_state(npc_id, merged.clone());

        let (emotions, dominant, mood) = build_emotion_fields(&merged);
        let guide = ActingGuide::build(npc, &merged, Some(focus.description.clone()), Some(relationship));

        let focus_id = focus.id.clone();
        self.repository.set_active_focus_id(Some(focus_id.clone()));

        Ok(StimulusResult {
            emotions,
            dominant,
            mood,
            guide,
            trace: beat_trace,
            beat_changed: true,
            active_focus_id: Some(focus_id),
        })
    }

    /// Beat 종료 시 Scene의 NPC/파트너 관계를 갱신합니다.
    fn update_beat_relationship(&mut self) {
        if let (Some(npc_id), Some(partner_id)) = (
            self.repository.get_scene_npc_id().map(|s| s.to_string()),
            self.repository.get_scene_partner_id().map(|s| s.to_string()),
        ) {
            let beat_req = AfterDialogueRequest {
                npc_id,
                partner_id,
                praiseworthiness: Some(0.0),
                significance: Some(BEAT_DEFAULT_SIGNIFICANCE),
            };
            let _ = self.update_relationship(&beat_req);
        }
    }

    /// Scene 시작: Focus 목록 등록 + 초기 Focus 자동 appraise
    pub fn start_scene(
        &mut self,
        req: SceneRequest,
        mut before_eval: impl FnMut(),
        mut after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<SceneResult, MindServiceError> {
        let focuses: Vec<SceneFocus> = req.focuses.iter()
            .map(|f| f.to_domain(&self.repository, &req.npc_id, &req.partner_id))
            .collect::<Result<Vec<_>, _>>()?;

        let focus_count = focuses.len();

        self.save_scene_state(focuses, &req.npc_id, &req.partner_id);

        let (initial_appraise, active_focus_id) = self.appraise_initial_focus(
            &req.npc_id, &req.partner_id,
            || before_eval(), || after_eval(),
        )?;

        Ok(SceneResult { focus_count, initial_appraise, active_focus_id })
    }

    /// 현재 Scene Focus 상태를 조회합니다.
    pub fn scene_info(&self) -> SceneInfoResult {
        let focuses = self.repository.get_scene_focuses();
        if focuses.is_empty() {
            return SceneInfoResult {
                has_scene: false,
                npc_id: None,
                partner_id: None,
                active_focus_id: None,
                focuses: vec![],
            };
        }

        let active_id = self.repository.get_active_focus_id();
        let focus_infos = focuses.iter().map(|f| {
            let trigger_display = match &f.trigger {
                FocusTrigger::Initial => "initial".to_string(),
                FocusTrigger::Conditions(or_groups) => {
                    or_groups.iter().map(|and_group| {
                        and_group.iter().map(|c| {
                            let threshold = match c.threshold {
                                crate::domain::emotion::ConditionThreshold::Below(v) => format!("< {:.1}", v),
                                crate::domain::emotion::ConditionThreshold::Above(v) => format!("> {:.1}", v),
                                crate::domain::emotion::ConditionThreshold::Absent => "absent".to_string(),
                            };
                            format!("{:?} {}", c.emotion, threshold)
                        }).collect::<Vec<_>>().join(" AND ")
                    }).collect::<Vec<_>>().join(" OR ")
                }
            };
            FocusInfoItem {
                id: f.id.clone(),
                description: f.description.clone(),
                is_active: active_id == Some(f.id.as_str()),
                trigger_display,
            }
        }).collect();

        SceneInfoResult {
            has_scene: true,
            npc_id: self.repository.get_scene_npc_id().map(|s| s.to_string()),
            partner_id: self.repository.get_scene_partner_id().map(|s| s.to_string()),
            active_focus_id: self.repository.get_active_focus_id().map(|s| s.to_string()),
            focuses: focus_infos,
        }
    }

    /// Scene Focus를 직접 로드합니다 (시나리오 파일 로드 시 사용).
    ///
    /// Initial Focus가 있으면 자동 appraise하고 결과를 반환합니다.
    pub fn load_scene_focuses(
        &mut self,
        focuses: Vec<SceneFocus>,
        npc_id: String,
        partner_id: String,
    ) -> Result<Option<AppraiseResult>, MindServiceError> {
        self.save_scene_state(focuses, &npc_id, &partner_id);
        let (result, _) = self.appraise_initial_focus(&npc_id, &partner_id, || {}, Vec::new)?;
        Ok(result)
    }

    /// Scene 상태를 저장합니다 (Focus 목록 + NPC/파트너 ID).
    fn save_scene_state(&mut self, focuses: Vec<SceneFocus>, npc_id: &str, partner_id: &str) {
        self.repository.set_scene_focuses(focuses);
        self.repository.set_scene_ids(npc_id.to_string(), partner_id.to_string());
    }

    /// Initial Focus를 찾아 appraise합니다. 없으면 (None, None) 반환.
    fn appraise_initial_focus(
        &mut self,
        npc_id: &str,
        partner_id: &str,
        mut before_eval: impl FnMut(),
        mut after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<(Option<AppraiseResult>, Option<String>), MindServiceError> {
        let focuses = self.repository.get_scene_focuses().to_vec();
        let initial = focuses.into_iter()
            .find(|f| matches!(f.trigger, FocusTrigger::Initial));

        let Some(focus) = initial else {
            return Ok((None, None));
        };

        let npc = self.repository.get_npc(npc_id)
            .ok_or_else(|| MindServiceError::NpcNotFound(npc_id.to_string()))?;

        let relationship = self.repository.get_relationship(npc_id, partner_id)
            .ok_or_else(|| MindServiceError::RelationshipNotFound(npc_id.to_string(), partner_id.to_string()))?;

        let situation = focus.to_situation()
            .map_err(|e| MindServiceError::InvalidSituation(e.to_string()))?;

        before_eval();
        let emotion_state = self.appraiser.appraise(npc.personality(), &situation, &relationship.modifiers());
        let trace = after_eval();

        let result = build_appraise_result(
            &npc, &emotion_state,
            Some(focus.description.clone()),
            Some(&relationship),
            trace,
        );

        self.repository.save_emotion_state(npc_id, emotion_state);
        self.repository.set_active_focus_id(Some(focus.id.clone()));

        Ok((Some(result), Some(focus.id)))
    }

    /// 현재 감정 상태 기반으로 가이드를 재생성합니다.
    pub fn generate_guide(&self, req: GuideRequest) -> Result<GuideResult, MindServiceError> {
        let npc = self.repository.get_npc(&req.npc_id)
            .ok_or_else(|| MindServiceError::NpcNotFound(req.npc_id.clone()))?;

        let relationship = self.repository.get_relationship(&req.npc_id, &req.partner_id)
            .ok_or_else(|| MindServiceError::RelationshipNotFound(req.npc_id.clone(), req.partner_id.clone()))?;

        let emotion_state = self.repository.get_emotion_state(&req.npc_id)
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        let guide = ActingGuide::build(&npc, &emotion_state, req.situation_description.clone(), Some(&relationship));

        Ok(GuideResult { guide })
    }

    /// 대화가 종료된 후, 현재 감정 상태를 바탕으로 관계를 갱신합니다.
    /// 감정 상태를 초기화합니다.
    pub fn after_dialogue(&mut self, req: AfterDialogueRequest) -> Result<AfterDialogueResponse, MindServiceError> {
        let response = self.update_relationship(&req)?;
        self.repository.clear_emotion_state(&req.npc_id);
        Ok(response)
    }

    /// Beat 종료 시 관계를 갱신합니다.
    /// 감정 상태는 초기화하지 않습니다 (같은 Scene 내에서 계속 사용).
    pub fn after_beat(&mut self, req: AfterDialogueRequest) -> Result<AfterDialogueResponse, MindServiceError> {
        self.update_relationship(&req)
    }

    /// 관계 갱신 공통 로직
    fn update_relationship(&mut self, req: &AfterDialogueRequest) -> Result<AfterDialogueResponse, MindServiceError> {
        let relationship = self.repository.get_relationship(&req.npc_id, &req.partner_id)
            .or_else(|| self.repository.get_relationship(&req.partner_id, &req.npc_id))
            .ok_or_else(|| MindServiceError::RelationshipNotFound(req.npc_id.clone(), req.partner_id.clone()))?;

        let emotion_state = self.repository.get_emotion_state(&req.npc_id)
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        let before = RelationshipValues {
            closeness: relationship.closeness().value(),
            trust: relationship.trust().value(),
            power: relationship.power().value(),
        };

        let significance = req.significance.unwrap_or(0.0).clamp(0.0, 1.0);
        let new_rel = relationship.after_dialogue(&emotion_state, req.praiseworthiness, significance);

        let after = RelationshipValues {
            closeness: new_rel.closeness().value(),
            trust: new_rel.trust().value(),
            power: new_rel.power().value(),
        };

        self.repository.save_relationship(&req.npc_id, &req.partner_id, new_rel);

        Ok(AfterDialogueResponse { before, after })
    }
}
