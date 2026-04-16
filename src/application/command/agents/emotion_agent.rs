//! EmotionAgent — 감정 평가/변동 전담
//!
//! MindService의 `execute_appraise_workflow()` + `apply_stimulus()` 핵심 로직을 추출.

use crate::application::command::handler::{emotion_snapshot, HandlerContext, HandlerOutput};
use crate::application::command::types::CommandResult;
use crate::application::dto::*;
use crate::application::mind_service::MindServiceError;
use crate::application::scene_service::SceneService;
use crate::domain::emotion::{AppraisalEngine, EmotionState, StimulusEngine};
use crate::domain::event::EventPayload;
use crate::domain::guide::ActingGuide;
use crate::domain::pad::Pad;
use crate::domain::tuning::{BEAT_DEFAULT_SIGNIFICANCE, BEAT_MERGE_THRESHOLD};
use crate::ports::{Appraiser, StimulusProcessor};

/// 감정 평가 + 자극 처리 에이전트
pub struct EmotionAgent {
    pub(crate) appraiser: AppraisalEngine,
    stimulus_processor: StimulusEngine,
    scene_service: SceneService,
}

impl EmotionAgent {
    pub fn new() -> Self {
        Self {
            appraiser: AppraisalEngine,
            stimulus_processor: StimulusEngine,
            scene_service: SceneService::new(),
        }
    }

    /// Appraise Command 처리
    pub fn handle_appraise(
        &self,
        npc_id: &str,
        partner_id: &str,
        situation: &Option<SituationInput>,
        ctx: &HandlerContext,
    ) -> Result<HandlerOutput, MindServiceError> {
        let npc = ctx.npc.as_ref().ok_or_else(|| MindServiceError::NpcNotFound(npc_id.into()))?;
        let rel = ctx.relationship.as_ref().ok_or_else(|| {
            MindServiceError::RelationshipNotFound(npc_id.into(), partner_id.into())
        })?;

        // Situation 해석: 명시되면 사용, 없으면 Scene의 활성 Focus에서 추출
        let domain_situation = match situation {
            Some(sit) => {
                // SituationInput → Situation (SituationService 없이 직접 변환 — modifiers는 관계에서)
                sit.to_domain(None, None, None, npc_id)?
            }
            None => {
                // Scene에서 추출
                let scene = ctx.scene.as_ref().ok_or_else(|| {
                    MindServiceError::InvalidSituation(
                        "situation이 생략되었으나 활성 Scene이 없습니다.".into(),
                    )
                })?;
                let focus = scene
                    .active_focus_id()
                    .and_then(|id| scene.focuses().iter().find(|f| f.id == id))
                    .or_else(|| scene.initial_focus())
                    .ok_or_else(|| {
                        MindServiceError::InvalidSituation("활성/초기 Focus가 없습니다.".into())
                    })?;
                focus
                    .to_situation()
                    .map_err(|e| MindServiceError::InvalidSituation(e.to_string()))?
            }
        };

        // Appraiser 실행
        let emotion_state = self.appraiser.appraise(
            npc.personality(),
            &domain_situation,
            &rel.modifiers(),
        );

        let snapshot = emotion_snapshot(&emotion_state);
        let result = build_appraise_result(
            npc,
            &emotion_state,
            Some(domain_situation.description.clone()),
            Some(rel),
            &ctx.partner_name,
            vec![], // trace 없음 (콜백은 MindService 전용)
        );

        let event = EventPayload::EmotionAppraised {
            npc_id: npc_id.to_string(),
            partner_id: partner_id.to_string(),
            situation_description: Some(domain_situation.description),
            dominant: result
                .dominant
                .as_ref()
                .map(|d| (d.emotion_type.clone(), d.intensity)),
            mood: result.mood,
            emotion_snapshot: snapshot,
        };

        Ok(HandlerOutput {
            result: CommandResult::Appraised(result),
            events: vec![event],
            new_emotion_state: Some((npc_id.to_string(), emotion_state)),
            new_relationship: None,
            clear_emotion: None,
            clear_scene: false,
            save_scene: None,
        })
    }

    /// ApplyStimulus Command 처리
    pub fn handle_stimulus(
        &self,
        npc_id: &str,
        partner_id: &str,
        pleasure: f32,
        arousal: f32,
        dominance: f32,
        situation_description: &Option<String>,
        ctx: &HandlerContext,
    ) -> Result<HandlerOutput, MindServiceError> {
        let npc = ctx.npc.as_ref().ok_or_else(|| MindServiceError::NpcNotFound(npc_id.into()))?;
        let rel = ctx.relationship.as_ref().ok_or_else(|| {
            MindServiceError::RelationshipNotFound(npc_id.into(), partner_id.into())
        })?;
        let current = ctx
            .emotion_state
            .as_ref()
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        let pad = Pad { pleasure, arousal, dominance };
        let mood_before = current.overall_valence();

        // Stimulus 적용
        let stimulated = self
            .stimulus_processor
            .apply_stimulus(npc.personality(), current, &pad);

        // Beat 전환 체크
        if let Some(ref scene) = ctx.scene {
            if let Some(focus) = self.scene_service.check_trigger(scene, &stimulated) {
                return self.handle_beat_transition(
                    npc_id,
                    partner_id,
                    npc,
                    rel,
                    scene,
                    &stimulated,
                    focus,
                    pad,
                    mood_before,
                    &ctx.partner_name,
                );
            }
        }

        // Beat 전환 없음
        let snapshot = emotion_snapshot(&stimulated);
        let (emotions, dominant, mood) = build_emotion_fields(&stimulated);
        let guide = ActingGuide::build(
            npc,
            &stimulated,
            situation_description.clone(),
            Some(rel),
            &ctx.partner_name,
        );

        let active_focus_id = ctx
            .scene
            .as_ref()
            .and_then(|s| s.active_focus_id().map(|id| id.to_string()));

        let result = StimulusResult {
            emotions,
            dominant,
            mood,
            guide,
            trace: vec![],
            beat_changed: false,
            active_focus_id,
            input_pad: Some(PadOutput { pleasure, arousal, dominance }),
        };

        let event = EventPayload::StimulusApplied {
            npc_id: npc_id.to_string(),
            partner_id: partner_id.to_string(),
            pad: (pleasure, arousal, dominance),
            mood_before,
            mood_after: result.mood,
            beat_changed: false,
            emotion_snapshot: snapshot,
        };

        Ok(HandlerOutput {
            result: CommandResult::StimulusApplied(result),
            events: vec![event],
            new_emotion_state: Some((npc_id.to_string(), stimulated)),
            new_relationship: None,
            clear_emotion: None,
            clear_scene: false,
            save_scene: None,
        })
    }

    /// Beat 전환 처리 (transition_beat 추출)
    fn handle_beat_transition(
        &self,
        npc_id: &str,
        partner_id: &str,
        npc: &crate::domain::personality::Npc,
        rel: &crate::domain::relationship::Relationship,
        scene: &crate::domain::emotion::Scene,
        stimulated: &EmotionState,
        focus: crate::domain::emotion::SceneFocus,
        input_pad: Pad,
        mood_before: f32,
        partner_name: &str,
    ) -> Result<HandlerOutput, MindServiceError> {
        let from_focus_id = scene.active_focus_id().map(|s| s.to_string());

        // Beat 관계 갱신용 요약 (beat_default_significance)
        let beat_rel_update = rel.after_dialogue(stimulated, BEAT_DEFAULT_SIGNIFICANCE);

        // 새 Focus로 appraise
        let situation = focus
            .to_situation()
            .map_err(|e| MindServiceError::InvalidSituation(e.to_string()))?;
        let new_state = self.appraiser.appraise(
            npc.personality(),
            &situation,
            &beat_rel_update.modifiers(),
        );

        // 감정 병합
        let merged = EmotionState::merge_from_beat(stimulated, &new_state, BEAT_MERGE_THRESHOLD);
        let snapshot = emotion_snapshot(&merged);
        let (emotions, dominant, mood) = build_emotion_fields(&merged);
        let guide = ActingGuide::build(
            npc,
            &merged,
            Some(focus.description.clone()),
            Some(rel),
            partner_name,
        );

        let focus_id = focus.id.clone();
        let mut new_scene = scene.clone();
        new_scene.set_active_focus(focus_id.clone());

        let result = StimulusResult {
            emotions,
            dominant,
            mood,
            guide,
            trace: vec![],
            beat_changed: true,
            active_focus_id: Some(focus_id.clone()),
            input_pad: Some(PadOutput {
                pleasure: input_pad.pleasure,
                arousal: input_pad.arousal,
                dominance: input_pad.dominance,
            }),
        };

        let stimulus_event = EventPayload::StimulusApplied {
            npc_id: npc_id.to_string(),
            partner_id: partner_id.to_string(),
            pad: (input_pad.pleasure, input_pad.arousal, input_pad.dominance),
            mood_before,
            mood_after: result.mood,
            beat_changed: true,
            emotion_snapshot: snapshot,
        };

        let beat_event = EventPayload::BeatTransitioned {
            npc_id: npc_id.to_string(),
            from_focus_id,
            to_focus_id: focus_id,
        };

        // Beat 관계 갱신 이벤트
        let rel_event = EventPayload::RelationshipUpdated {
            owner_id: npc_id.to_string(),
            target_id: partner_id.to_string(),
            before_closeness: rel.closeness().value(),
            before_trust: rel.trust().value(),
            before_power: rel.power().value(),
            after_closeness: beat_rel_update.closeness().value(),
            after_trust: beat_rel_update.trust().value(),
            after_power: beat_rel_update.power().value(),
        };

        Ok(HandlerOutput {
            result: CommandResult::StimulusApplied(result),
            events: vec![stimulus_event, beat_event, rel_event],
            new_emotion_state: Some((npc_id.to_string(), merged)),
            new_relationship: Some((
                npc_id.to_string(),
                partner_id.to_string(),
                beat_rel_update,
            )),
            clear_emotion: None,
            clear_scene: false,
            save_scene: Some(new_scene),
        })
    }
}

impl Default for EmotionAgent {
    fn default() -> Self {
        Self::new()
    }
}
