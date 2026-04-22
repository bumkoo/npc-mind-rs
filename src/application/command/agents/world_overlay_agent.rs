//! WorldOverlayAgent — 세계 오버레이 사건 팬아웃 (Step D, Mind 컨텍스트)
//!
//! `ApplyWorldEventRequested`를 받아 `WorldEventOccurred` follow-up을 단일 발행한다.
//! Canonical MemoryEntry 생성 / supersede 는 Inline `WorldOverlayHandler`가 처리하며,
//! 이 에이전트는 순수 이벤트 변환 역할만 한다.
//!
//! **Priority**: `WORLD_OVERLAY = 25` — Guide 직후, Relationship 이전 (§6.5 B6).

use crate::application::command::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::event::{DomainEvent, EventKind, EventPayload};

pub struct WorldOverlayAgent;

impl WorldOverlayAgent {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WorldOverlayAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for WorldOverlayAgent {
    fn name(&self) -> &'static str {
        "WorldOverlayAgent"
    }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![EventKind::ApplyWorldEventRequested])
    }

    fn mode(&self) -> DeliveryMode {
        DeliveryMode::Transactional {
            priority: priority::transactional::WORLD_OVERLAY,
            can_emit_follow_up: true,
        }
    }

    fn handle(
        &self,
        event: &DomainEvent,
        _ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        let EventPayload::ApplyWorldEventRequested {
            world_id,
            topic,
            fact,
            significance,
            witnesses,
        } = &event.payload
        else {
            return Ok(HandlerResult::default());
        };

        let occurred = DomainEvent::new(
            0,
            world_id.clone(),
            0,
            EventPayload::WorldEventOccurred {
                world_id: world_id.clone(),
                topic: topic.clone(),
                fact: fact.clone(),
                significance: *significance,
                witnesses: witnesses.clone(),
            },
        );

        Ok(HandlerResult {
            follow_up_events: vec![occurred],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::command::handler_v2::test_support::HandlerTestHarness;

    fn request_event(world_id: &str, topic: Option<&str>, fact: &str) -> DomainEvent {
        DomainEvent::new(
            0,
            world_id.into(),
            0,
            EventPayload::ApplyWorldEventRequested {
                world_id: world_id.into(),
                topic: topic.map(String::from),
                fact: fact.into(),
                significance: 0.6,
                witnesses: vec!["a".into()],
            },
        )
    }

    #[test]
    fn emits_world_event_occurred_with_same_payload() {
        let agent = WorldOverlayAgent::new();
        let mut harness = HandlerTestHarness::new();
        let result = harness
            .dispatch(&agent, request_event("jianghu", Some("leader"), "새 맹주 등장"))
            .expect("must succeed");

        assert_eq!(result.follow_up_events.len(), 1);
        let ev = &result.follow_up_events[0];
        assert_eq!(ev.kind(), EventKind::WorldEventOccurred);
        let EventPayload::WorldEventOccurred {
            world_id,
            topic,
            fact,
            significance,
            witnesses,
        } = &ev.payload
        else {
            panic!("expected WorldEventOccurred");
        };
        assert_eq!(world_id, "jianghu");
        assert_eq!(topic.as_deref(), Some("leader"));
        assert_eq!(fact, "새 맹주 등장");
        assert!((*significance - 0.6).abs() < 1e-6);
        assert_eq!(witnesses, &vec!["a".to_string()]);
    }

    #[test]
    fn ignores_unrelated_event_kind() {
        let agent = WorldOverlayAgent::new();
        let mut harness = HandlerTestHarness::new();
        let event = DomainEvent::new(
            0,
            "x".into(),
            0,
            EventPayload::EmotionCleared { npc_id: "x".into() },
        );
        let result = harness.dispatch(&agent, event).expect("must succeed");
        assert!(result.follow_up_events.is_empty());
    }
}
