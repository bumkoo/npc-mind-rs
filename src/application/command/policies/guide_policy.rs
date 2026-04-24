//! GuidePolicy — 연기 가이드 생성 전담 (v2)
//!
//! `EmotionAppraised` / `StimulusApplied` / `GuideRequested` 이벤트에 자동 반응하여
//! 가이드를 생성한다. `ctx.shared.emotion_state`가 설정돼 있으면 이를 참조.

use crate::application::command::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::event::{DomainEvent, EventKind, EventPayload};
use crate::domain::guide::ActingGuide;

/// 연기 가이드 생성 폴리시
pub struct GuidePolicy;

impl GuidePolicy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GuidePolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for GuidePolicy {
    fn name(&self) -> &'static str {
        "GuidePolicy"
    }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![
            EventKind::EmotionAppraised,
            EventKind::StimulusApplied,
            // B4.1: GenerateGuide 커맨드의 초기 이벤트 (standalone guide generation)
            EventKind::GuideRequested,
        ])
    }

    fn mode(&self) -> DeliveryMode {
        DeliveryMode::Transactional {
            priority: priority::transactional::GUIDE_GENERATION,
            can_emit_follow_up: true,
        }
    }

    fn handle(
        &self,
        event: &DomainEvent,
        ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        let (npc_id, partner_id, situation_description) = match &event.payload {
            EventPayload::EmotionAppraised {
                npc_id,
                partner_id,
                situation_description,
                ..
            } => (npc_id, partner_id, situation_description.clone()),
            EventPayload::StimulusApplied {
                npc_id,
                partner_id,
                ..
            } => (npc_id, partner_id, None),
            EventPayload::GuideRequested {
                npc_id,
                partner_id,
                situation_description,
            } => (npc_id, partner_id, situation_description.clone()),
            _ => return Ok(HandlerResult::default()),
        };

        let npc = ctx
            .repo
            .get_npc(npc_id)
            .ok_or_else(|| HandlerError::NpcNotFound(npc_id.clone()))?;
        // B4.1: EmotionAppraised/StimulusApplied 경로는 shared.emotion_state에 이미 설정됨.
        //        GuideRequested(standalone) 경로는 shared가 비어있으므로 repo에서 조회.
        //        cloned로 소유권을 얻어 borrow 충돌 회피.
        let emotion_state = match ctx.shared.emotion_state.clone() {
            Some(s) => s,
            None => ctx
                .repo
                .get_emotion_state(npc_id)
                .ok_or_else(|| HandlerError::EmotionStateNotFound(npc_id.clone()))?,
        };
        let relationship = ctx.shared.relationship.as_ref().cloned().or_else(|| {
            ctx.repo.get_relationship(npc_id, partner_id)
        });

        let partner_name = ctx
            .repo
            .get_npc(partner_id)
            .map(|n| n.name().to_string())
            .unwrap_or_default();

        let guide = ActingGuide::build(
            &npc,
            &emotion_state,
            situation_description,
            relationship.as_ref(),
            &partner_name,
        );

        ctx.shared.guide = Some(guide);

        let follow_up = DomainEvent::new(
            0,
            npc_id.clone(),
            0,
            EventPayload::GuideGenerated {
                npc_id: npc_id.clone(),
                partner_id: partner_id.clone(),
            },
        );

        Ok(HandlerResult {
            follow_up_events: vec![follow_up],
        })
    }
}

// ===========================================================================
// B1 — L1 단위 테스트
// ===========================================================================

#[cfg(test)]
mod handler_v2_tests {
    use super::*;
    use crate::application::command::handler_v2::test_support::HandlerTestHarness;
    use crate::domain::emotion::EmotionState;
    use crate::domain::personality::NpcBuilder;
    use crate::domain::relationship::Relationship;

    fn make_emotion_appraised(npc_id: &str, partner_id: &str) -> DomainEvent {
        DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::EmotionAppraised {
                npc_id: npc_id.to_string(),
                partner_id: partner_id.to_string(),
                situation_description: Some("test".into()),
                dominant: None,
                mood: 0.0,
                emotion_snapshot: vec![],
            },
        )
    }

    fn make_stimulus_applied(npc_id: &str, partner_id: &str) -> DomainEvent {
        DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::StimulusApplied {
                npc_id: npc_id.to_string(),
                partner_id: partner_id.to_string(),
                pad: (0.0, 0.0, 0.0),
                mood_before: 0.0,
                mood_after: 0.0,
                beat_changed: false,
                emotion_snapshot: vec![],
            },
        )
    }

    #[test]
    fn emotion_appraised_event_generates_guide_and_populates_shared() {
        let policy = GuidePolicy::new();
        let npc = NpcBuilder::new("alice", "Alice").build();
        let partner = NpcBuilder::new("bob", "Bob").build();
        let rel = Relationship::neutral("alice", "bob");
        let mut harness = HandlerTestHarness::new()
            .with_npc(npc)
            .with_npc(partner)
            .with_relationship(rel)
            .with_shared_emotion_state(EmotionState::default());

        let event = make_emotion_appraised("alice", "bob");
        let result = harness.dispatch(&policy, event).expect("must succeed");

        assert_eq!(result.follow_up_events.len(), 1);
        assert_eq!(result.follow_up_events[0].kind(), EventKind::GuideGenerated);
        assert!(harness.shared.guide.is_some());
    }

    #[test]
    fn stimulus_applied_event_also_triggers_guide() {
        let policy = GuidePolicy::new();
        let npc = NpcBuilder::new("alice", "Alice").build();
        let rel = Relationship::neutral("alice", "bob");
        let mut harness = HandlerTestHarness::new()
            .with_npc(npc)
            .with_relationship(rel)
            .with_shared_emotion_state(EmotionState::default());

        let event = make_stimulus_applied("alice", "bob");
        let result = harness.dispatch(&policy, event).expect("must succeed");

        assert_eq!(result.follow_up_events.len(), 1);
        assert_eq!(result.follow_up_events[0].kind(), EventKind::GuideGenerated);
    }

    #[test]
    fn missing_shared_emotion_state_returns_precondition_error() {
        let policy = GuidePolicy::new();
        let npc = NpcBuilder::new("alice", "Alice").build();
        // shared.emotion_state 미주입
        let mut harness = HandlerTestHarness::new().with_npc(npc);

        let event = make_emotion_appraised("alice", "bob");
        let err = harness.dispatch(&policy, event).expect_err("must fail");

        // B4.1: shared 비어있으면 repo fallback → repo에도 없으면 EmotionStateNotFound
        assert!(matches!(
            err,
            HandlerError::EmotionStateNotFound(ref id) if id == "alice"
        ));
    }
}
