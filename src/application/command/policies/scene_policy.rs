//! ScenePolicy — Scene 시작 전담 (B안 B4.1)
//!
//! `SceneStartRequested` 이벤트를 수신하여 Scene을 `UnitOfWork`에 등록하고
//! 초기 Focus가 있으면 appraise를 수행해 `EmotionAppraised` follow-up을 발행한다.
//!
//! 가이드 생성은 이 policy 책임 밖 — GuidePolicy가 `EmotionAppraised`에 반응해 자동 생성.

use crate::application::command::handler_v2::{
    DeliveryMode, DynamicHandlerContext, EventHandler, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::emotion::{AppraisalEngine, EmotionState};
use crate::domain::event::{DomainEvent, EventKind, EventPayload};
use crate::ports::personality::Appraiser;

/// Scene 시작 전담 폴리시
pub struct ScenePolicy {
    appraiser: AppraisalEngine,
}

impl ScenePolicy {
    pub fn new() -> Self {
        Self {
            appraiser: AppraisalEngine,
        }
    }
}

impl Default for ScenePolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for ScenePolicy {
    fn name(&self) -> &'static str {
        "ScenePolicy"
    }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![EventKind::SceneStartRequested])
    }

    fn mode(&self) -> DeliveryMode {
        // SCENE_START < EMOTION_APPRAISAL (priority.rs invariant 고정). Scene 시작 후
        // 초기 EmotionAppraised가 GuidePolicy로 cascade.
        DeliveryMode::Transactional {
            priority: priority::transactional::SCENE_START,
            can_emit_follow_up: true,
        }
    }

    fn handle_v2(
        &self,
        event: &DomainEvent,
        ctx: &mut dyn DynamicHandlerContext,
    ) -> Result<HandlerResult, HandlerError> {
        let EventPayload::SceneStartRequested {
            npc_id,
            partner_id,
            significance: _,
            initial_focus_id,
            prebuilt_scene,
        } = &event.payload
        else {
            return Ok(HandlerResult::default());
        };

        let focus_count = prebuilt_scene.focuses().len();
        let mut scene = (**prebuilt_scene).clone();

        let npc = ctx.get_npc(npc_id)?;
        let relationship = ctx.get_relationship(npc_id, partner_id)?;

        // 초기 Focus가 있으면 appraise
        let (_active_focus_id, emotion_state): (Option<String>, Option<EmotionState>) = if let Some(initial) =
            initial_focus_id.as_ref().and_then(|id| {
                scene.focuses().iter().find(|f| f.id == *id).cloned()
            })
        {
            let situation = initial
                .to_situation()
                .map_err(|e| HandlerError::InvalidInput(
                    format!("initial focus to_situation failed: {e}"),
                ))?;
            let state = self.appraiser.appraise(
                npc.personality(),
                &situation,
                &relationship.modifiers(),
            );
            scene.set_active_focus(initial.id.clone());
            (Some(initial.id), Some(state))
        } else {
            (None, None)
        };

        // UoW에 등록
        ctx.save_scene(scene);
        if let Some(state) = &emotion_state {
            ctx.save_emotion_state(npc_id.clone(), state.clone());
            ctx.save_relationship(relationship);
        }

        // follow-ups: SceneStarted + (옵션) EmotionAppraised
        let scene_started = DomainEvent::new(
            0,
            npc_id.clone(),
            0,
            EventPayload::SceneStarted {
                npc_id: npc_id.clone(),
                partner_id: partner_id.clone(),
                focus_count,
                initial_focus_id: initial_focus_id.clone(),
            },
        );
        let mut follow_ups = vec![scene_started];

        if let Some(state) = emotion_state {
            let dominant = state
                .dominant()
                .map(|e: crate::domain::emotion::Emotion| (format!("{:?}", e.emotion_type()), e.intensity()));
            let mood = state.overall_valence();
            let snapshot = state.snapshot();
            let emotion_event = DomainEvent::new(
                0,
                npc_id.clone(),
                0,
                EventPayload::EmotionAppraised {
                    npc_id: npc_id.clone(),
                    partner_id: partner_id.clone(),
                    situation_description: None,
                    dominant,
                    mood,
                    emotion_snapshot: snapshot,
                },
            );
            follow_ups.push(emotion_event);
        }

        Ok(HandlerResult {
            follow_up_events: follow_ups,
        })
    }
}

// ===========================================================================
// B4.1 — L1 단위 테스트
// ===========================================================================

#[cfg(test)]
mod handler_v2_tests {
    use super::*;
    use crate::application::command::handler_v2::test_support::HandlerTestHarness;
    use crate::application::command::handler_v2::HandlerError;
    use crate::domain::emotion::{EventFocus, FocusTrigger, Scene, SceneFocus};
    use crate::domain::personality::NpcBuilder;
    use crate::domain::relationship::Relationship;

    fn make_focus(id: &str, trigger: FocusTrigger) -> SceneFocus {
        SceneFocus {
            id: id.into(),
            description: id.into(),
            trigger,
            event: Some(EventFocus {
                description: "".into(),
                desirability_for_self: 0.2,
                desirability_for_other: None,
                prospect: None,
            }),
            action: None,
            object: None,
            test_script: vec![],
        }
    }

    fn make_scene_start_req(npc_id: &str, partner_id: &str, focuses: Vec<SceneFocus>) -> DomainEvent {
        let initial_focus_id = focuses
            .iter()
            .find(|f| matches!(f.trigger, FocusTrigger::Initial))
            .map(|f| f.id.clone());
        let prebuilt_scene = Scene::new(npc_id.into(), partner_id.into(), focuses);
        DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::SceneStartRequested {
                npc_id: npc_id.to_string(),
                partner_id: partner_id.to_string(),
                significance: Some(0.5),
                initial_focus_id,
                prebuilt_scene: Box::new(prebuilt_scene),
            },
        )
    }

    #[test]
    fn scene_start_with_initial_focus_emits_scene_started_and_emotion_appraised() {
        let policy = ScenePolicy::new();
        let npc = NpcBuilder::new("alice", "Alice").build();
        let partner = NpcBuilder::new("bob", "Bob").build();
        let rel = Relationship::neutral("alice", "bob");
        let mut harness = HandlerTestHarness::new()
            .with_npc(npc)
            .with_npc(partner)
            .with_relationship(rel);

        let event = make_scene_start_req(
            "alice",
            "bob",
            vec![make_focus("initial", FocusTrigger::Initial)],
        );
        let (result, uow) = harness.dispatch(&policy, event).expect("must succeed");

        let kinds: Vec<_> = result.follow_up_events.iter().map(|e| e.kind()).collect();
        assert_eq!(
            kinds,
            vec![EventKind::SceneStarted, EventKind::EmotionAppraised],
            "ScenePolicy는 SceneStarted 먼저, EmotionAppraised 뒤 순서로 발행"
        );
        assert!(uow.scene.is_some());
        assert!(uow.emotion_state.is_some());
    }

    #[test]
    fn scene_start_without_initial_focus_only_emits_scene_started() {
        let policy = ScenePolicy::new();
        let npc = NpcBuilder::new("alice", "Alice").build();
        let rel = Relationship::neutral("alice", "bob");
        let mut harness = HandlerTestHarness::new().with_npc(npc).with_relationship(rel);

        let event = make_scene_start_req(
            "alice",
            "bob",
            vec![make_focus("pending", FocusTrigger::Conditions(vec![]))],
        );
        let (result, uow) = harness.dispatch(&policy, event).expect("must succeed");

        assert_eq!(result.follow_up_events.len(), 1);
        assert_eq!(result.follow_up_events[0].kind(), EventKind::SceneStarted);
        assert!(uow.scene.is_some());
        assert!(uow.emotion_state.is_none());
    }

    #[test]
    fn missing_npc_returns_precondition_error() {
        let policy = ScenePolicy::new();
        let mut harness = HandlerTestHarness::new(); 

        let event = make_scene_start_req(
            "ghost",
            "nobody",
            vec![make_focus("initial", FocusTrigger::Initial)],
        );
        let err = harness.dispatch(&policy, event).expect_err("must fail");

        assert!(matches!(
            err,
            HandlerError::NpcNotFound(ref id) if id == "ghost"
        ));
    }
}
