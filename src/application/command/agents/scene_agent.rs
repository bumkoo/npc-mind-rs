//! SceneAgent — Scene 시작 전담 (B안 B4.1)
//!
//! `SceneStartRequested` 이벤트를 수신하여 Scene을 `ctx.shared.scene`에 등록하고
//! 초기 Focus가 있으면 appraise를 수행해 `EmotionAppraised` follow-up을 발행한다.
//!
//! v1의 `CommandDispatcher::handle_start_scene`(dispatcher.rs)가 이 핸들러로 대체된다.
//! v1이 side-effect flag로 scene 저장을 지시했던 부분이 v2에서는 `ctx.shared.scene`으로,
//! 초기 감정도 `ctx.shared.emotion_state`로 전파되어 Dispatcher가 write-back.
//!
//! 가이드 생성은 이 agent 책임 밖 — GuideAgent가 `EmotionAppraised`에 반응해 자동 생성.

use crate::application::command::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::emotion::AppraisalEngine;
use crate::domain::event::{DomainEvent, EventKind, EventPayload};
use crate::ports::Appraiser;

/// Scene 시작 전담 에이전트
pub struct SceneAgent {
    appraiser: AppraisalEngine,
}

impl SceneAgent {
    pub fn new() -> Self {
        Self {
            appraiser: AppraisalEngine,
        }
    }
}

impl Default for SceneAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for SceneAgent {
    fn name(&self) -> &'static str {
        "SceneAgent"
    }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![EventKind::SceneStartRequested])
    }

    fn mode(&self) -> DeliveryMode {
        // SCENE_START < EMOTION_APPRAISAL (priority.rs invariant 고정). Scene 시작 후
        // 초기 EmotionAppraised가 GuideAgent로 cascade.
        DeliveryMode::Transactional {
            priority: priority::transactional::SCENE_START,
            can_emit_follow_up: true,
        }
    }

    fn handle(
        &self,
        event: &DomainEvent,
        ctx: &mut EventHandlerContext<'_>,
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
        let mut scene = prebuilt_scene.clone();

        let npc = ctx
            .repo
            .get_npc(npc_id)
            .ok_or(HandlerError::Precondition("npc not found"))?;
        let relationship = ctx
            .repo
            .get_relationship(npc_id, partner_id)
            .ok_or(HandlerError::Precondition("relationship not found"))?;

        // 초기 Focus가 있으면 appraise
        let (active_focus_id, emotion_state) = if let Some(initial) =
            initial_focus_id.as_ref().and_then(|id| {
                scene.focuses().iter().find(|f| f.id == *id).cloned()
            })
        {
            let situation = initial
                .to_situation()
                .map_err(|_| HandlerError::Precondition("initial focus to_situation failed"))?;
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

        // shared에 전파 — Dispatcher가 commit 후 repo에 반영
        ctx.shared.scene = Some(scene);
        if let Some(state) = &emotion_state {
            ctx.shared.emotion_state = Some(state.clone());
            ctx.shared.relationship = Some(relationship);
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
                initial_focus_id: active_focus_id.clone(),
            },
        );
        let mut follow_ups = vec![scene_started];

        if let Some(state) = emotion_state {
            let dominant = state
                .dominant()
                .map(|e| (format!("{:?}", e.emotion_type()), e.intensity()));
            let mood = state.overall_valence();
            let snapshot = state.snapshot();
            // situation_description은 Focus의 to_situation의 description에서 유도됨.
            // SceneStartRequested payload엔 명시 없으므로 None으로 둔다 — v1 handle_start_scene과 동일.
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
                prebuilt_scene,
            },
        )
    }

    #[test]
    fn scene_start_with_initial_focus_emits_scene_started_and_emotion_appraised() {
        let agent = SceneAgent::new();
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
        let result = harness.dispatch(&agent, event).expect("must succeed");

        // 순서 고정: SceneStarted → EmotionAppraised (SceneAgent가 한 트랜잭션에서 2 follow-ups)
        // SceneStarted가 먼저 나와야 Projection/downstream이 Scene 등록을 인지한 뒤
        // EmotionAppraised를 소비하는 의미상 올바른 순서.
        let kinds: Vec<_> = result.follow_up_events.iter().map(|e| e.kind()).collect();
        assert_eq!(
            kinds,
            vec![EventKind::SceneStarted, EventKind::EmotionAppraised],
            "SceneAgent는 SceneStarted 먼저, EmotionAppraised 뒤 순서로 발행"
        );
        assert!(harness.shared.scene.is_some(), "shared.scene 설정");
        assert!(
            harness.shared.emotion_state.is_some(),
            "초기 appraise 결과가 shared에 전파"
        );
    }

    #[test]
    fn scene_start_without_initial_focus_only_emits_scene_started() {
        let agent = SceneAgent::new();
        let npc = NpcBuilder::new("alice", "Alice").build();
        let rel = Relationship::neutral("alice", "bob");
        let mut harness = HandlerTestHarness::new().with_npc(npc).with_relationship(rel);

        // Initial trigger 없는 focus만 → 초기 appraise 없음
        let event = make_scene_start_req(
            "alice",
            "bob",
            vec![make_focus("pending", FocusTrigger::Conditions(vec![]))],
        );
        let result = harness.dispatch(&agent, event).expect("must succeed");

        assert_eq!(result.follow_up_events.len(), 1);
        assert_eq!(result.follow_up_events[0].kind(), EventKind::SceneStarted);
        assert!(harness.shared.scene.is_some());
        assert!(harness.shared.emotion_state.is_none());
    }

    #[test]
    fn missing_npc_returns_precondition_error() {
        let agent = SceneAgent::new();
        let mut harness = HandlerTestHarness::new(); // repo 비어있음

        let event = make_scene_start_req(
            "ghost",
            "nobody",
            vec![make_focus("initial", FocusTrigger::Initial)],
        );
        let err = harness.dispatch(&agent, event).expect_err("must fail");

        assert!(matches!(
            err,
            HandlerError::Precondition("npc not found")
        ));
    }
}
