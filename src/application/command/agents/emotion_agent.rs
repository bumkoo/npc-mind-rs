//! EmotionAgent — 감정 평가/변동 전담
//!
//! MindService의 `execute_appraise_workflow()` + `apply_stimulus()` 핵심 로직을 추출.
//!
//! B5.1: v1 `handle_appraise`/`handle_stimulus`는 deprecated. v2 `impl EventHandler` 사용.
//! 내부 self-reference가 많아 모듈 레벨 allow.

#![allow(deprecated)]

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

    /// Appraise Command 처리 — **v1, deprecated**
    ///
    /// B5.1 (v0.2.0): v2 `EventHandler::handle` impl로 대체됨. v0.3.0 제거 예정.
    #[deprecated(
        since = "0.2.0",
        note = "v2 `impl EventHandler for EmotionAgent` 사용. v0.3.0에서 제거 예정."
    )]
    #[allow(deprecated)]
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

    /// ApplyStimulus Command 처리 — **v1, deprecated**
    ///
    /// B5.1 (v0.2.0): v2 `StimulusAgent` (별도 struct)가 Beat 전환까지 담당. v0.3.0 제거.
    #[deprecated(
        since = "0.2.0",
        note = "v2 `StimulusAgent` + `impl EventHandler` 로 대체. v0.3.0에서 제거 예정."
    )]
    #[allow(deprecated)]
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
            partner_id: partner_id.to_string(),
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

// ===========================================================================
// B1 — EventHandler impl (v2 진입점)
// ===========================================================================
//
// 기존 `handle_appraise`(v1)와 공존. Dispatcher는 아직 이 impl을 호출하지 않는다
// (B3에서 `dispatch_v2()`가 wiring). 차이점:
// - v1은 `HandlerContext`(dispatcher pre-fetched)에서 읽고 side-effect 플래그를 반환.
// - v2는 `EventHandlerContext.repo`에서 직접 읽고, `ctx.shared`에 상태 전파,
//   `follow_up_events`로 `EmotionAppraised` 발행.
// - v2는 repo 쓰기를 하지 않는다(Projection이 B2에서 담당).

use crate::application::command::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::event::{DomainEvent, EventKind};

impl EventHandler for EmotionAgent {
    fn name(&self) -> &'static str {
        "EmotionAgent"
    }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![EventKind::AppraiseRequested])
    }

    fn mode(&self) -> DeliveryMode {
        DeliveryMode::Transactional {
            priority: priority::transactional::EMOTION_APPRAISAL,
            can_emit_follow_up: true,
        }
    }

    fn handle(
        &self,
        event: &DomainEvent,
        ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        let EventPayload::AppraiseRequested {
            npc_id,
            partner_id,
            situation,
        } = &event.payload
        else {
            return Ok(HandlerResult::default());
        };

        let npc = ctx
            .repo
            .get_npc(npc_id)
            .ok_or(HandlerError::Precondition("npc not found"))?;
        let relationship = ctx
            .repo
            .get_relationship(npc_id, partner_id)
            .ok_or(HandlerError::Precondition("relationship not found"))?;

        let emotion_state =
            self.appraiser
                .appraise(npc.personality(), situation, &relationship.modifiers());

        // v1이 사용하던 Result 구성/dominant 계산은 CommandResult 생성에 필요한데,
        // v2에서는 후속 Projection/Dispatcher가 처리. 여기서는 이벤트 + shared 상태만.
        let dominant = emotion_state
            .dominant()
            .map(|e| (format!("{:?}", e.emotion_type()), e.intensity()));
        let mood = emotion_state.overall_valence();
        let snapshot = emotion_snapshot(&emotion_state);

        ctx.shared.emotion_state = Some(emotion_state.clone());
        ctx.shared.relationship = Some(relationship);

        let follow_up = DomainEvent::new(
            0,
            npc_id.clone(),
            0,
            EventPayload::EmotionAppraised {
                npc_id: npc_id.clone(),
                partner_id: partner_id.clone(),
                situation_description: Some(situation.description.clone()),
                dominant,
                mood,
                emotion_snapshot: snapshot,
            },
        );

        Ok(HandlerResult {
            follow_up_events: vec![follow_up],
        })
    }
}

// ===========================================================================
// B1 — L1 단위 테스트 (EventHandler impl 검증)
// ===========================================================================

#[cfg(test)]
mod handler_v2_tests {
    use super::*;
    use crate::application::command::handler_v2::test_support::HandlerTestHarness;
    use crate::application::command::handler_v2::HandlerError;
    use crate::domain::emotion::{EventFocus, Situation};
    use crate::domain::event::{DomainEvent, EventKind, EventPayload};
    use crate::domain::personality::NpcBuilder;
    use crate::domain::relationship::Relationship;

    fn positive_situation() -> Situation {
        Situation::new(
            "긍정적 상황",
            Some(EventFocus {
                description: "".into(),
                desirability_for_self: 0.8,
                desirability_for_other: None,
                prospect: None,
            }),
            None,
            None,
        )
        .unwrap()
    }

    fn make_request(npc_id: &str, partner_id: &str, situation: Situation) -> DomainEvent {
        DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::AppraiseRequested {
                npc_id: npc_id.to_string(),
                partner_id: partner_id.to_string(),
                situation,
            },
        )
    }

    #[test]
    fn appraise_request_emits_emotion_appraised_and_populates_shared() {
        let agent = EmotionAgent::new();
        let npc = NpcBuilder::new("alice", "Alice").build();
        let partner = NpcBuilder::new("bob", "Bob").build();
        let rel = Relationship::neutral("alice", "bob");

        let mut harness = HandlerTestHarness::new()
            .with_npc(npc)
            .with_npc(partner)
            .with_relationship(rel);

        let event = make_request("alice", "bob", positive_situation());
        let result = harness.dispatch(&agent, event).expect("handler must succeed");

        assert_eq!(result.follow_up_events.len(), 1);
        assert_eq!(result.follow_up_events[0].kind(), EventKind::EmotionAppraised);
        assert!(harness.shared.emotion_state.is_some());
        assert!(harness.shared.relationship.is_some());
    }

    #[test]
    fn ignores_unrelated_event_kind() {
        let agent = EmotionAgent::new();
        let mut harness = HandlerTestHarness::new();

        // GuideGenerated is unrelated to EmotionAgent's interest
        let event = DomainEvent::new(
            0,
            "alice".into(),
            0,
            EventPayload::GuideGenerated {
                npc_id: "alice".into(),
                partner_id: "bob".into(),
            },
        );

        let result = harness.dispatch(&agent, event).expect("unrelated event should no-op");
        assert!(result.follow_up_events.is_empty());
        assert!(harness.shared.emotion_state.is_none());
    }

    #[test]
    fn missing_npc_returns_precondition_error() {
        let agent = EmotionAgent::new();
        // Repo는 비어있음 — NPC가 존재하지 않아 Precondition 에러
        let mut harness = HandlerTestHarness::new();

        let event = make_request("ghost", "nobody", positive_situation());
        let err = harness.dispatch(&agent, event).expect_err("must fail without npc");

        assert!(matches!(err, HandlerError::Precondition("npc not found")));
    }
}
