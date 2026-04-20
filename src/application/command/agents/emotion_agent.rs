//! EmotionAgent — 감정 평가 전담 (v2)

use crate::application::command::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::emotion::AppraisalEngine;
use crate::domain::event::{DomainEvent, EventKind, EventPayload};
use crate::ports::Appraiser;

/// 감정 평가 에이전트 (v2)
pub struct EmotionAgent {
    pub(crate) appraiser: AppraisalEngine,
}

impl EmotionAgent {
    pub fn new() -> Self {
        Self {
            appraiser: AppraisalEngine,
        }
    }
}

impl Default for EmotionAgent {
    fn default() -> Self {
        Self::new()
    }
}

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
            .ok_or_else(|| HandlerError::NpcNotFound(npc_id.clone()))?;
        let relationship = ctx
            .repo
            .get_relationship(npc_id, partner_id)
            .ok_or_else(|| HandlerError::RelationshipNotFound {
                owner_id: npc_id.clone(),
                target_id: partner_id.clone(),
            })?;

        let emotion_state =
            self.appraiser
                .appraise(npc.personality(), situation, &relationship.modifiers());

        let dominant = emotion_state
            .dominant()
            .map(|e| (format!("{:?}", e.emotion_type()), e.intensity()));
        let mood = emotion_state.overall_valence();
        let snapshot = emotion_state.snapshot();

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
// Unit tests
// ===========================================================================

#[cfg(test)]
mod handler_v2_tests {
    use super::*;
    use crate::application::command::handler_v2::test_support::HandlerTestHarness;
    use crate::domain::emotion::{EventFocus, Situation};
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
        let mut harness = HandlerTestHarness::new();

        let event = make_request("ghost", "nobody", positive_situation());
        let err = harness.dispatch(&agent, event).expect_err("must fail without npc");

        assert!(matches!(err, HandlerError::NpcNotFound(ref id) if id == "ghost"));
    }
}
