//! WorldOverlayPolicy — 세계 오버레이 사건 팬아웃 (Step D, Mind 컨텍스트)
//!
//! `ApplyWorldEventRequested`를 받아 `WorldEventOccurred` follow-up을 단일 발행한다.
//! Canonical MemoryEntry 생성 / supersede 는 Inline `WorldOverlayHandler`가 처리하며,
//! 이 폴리시는 순수 이벤트 변환 역할만 한다.
//!
//! **왜 별도 Agent로 유지하나** (리뷰 M1): 1:1 passthrough라 dispatcher의
//! `build_initial_event`에서 바로 `WorldEventOccurred`를 발행하는 쪽이 cascade depth가
//! 얕아지지만, 다음 이유로 Agent 단계를 유지한다:
//! 1. 다른 Command도 Transactional 체인을 통해 `*Requested → *Occurred` 흐름을 거친다
//!    (`InformationPolicy`, `ScenePolicy` 등) — 일관된 대칭.
//! 2. 향후 세계 사건 유효성 검증·Canonical 충돌 감지·다중 follow-up (예: 목격자 개별
//!    이벤트)이 필요해지면 dispatcher를 건드리지 않고 여기서 확장 가능.
//! 3. Event Sourcing 관점에서 `*Requested`와 `*Occurred`가 이벤트 스토어에 별도로 남아
//!    replay·audit 시 인과 분리가 명확.
//!
//! **Priority**: `WORLD_OVERLAY = 25` — Guide 직후, Relationship 이전 (§6.5 B6).

use crate::application::command::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::event::{DomainEvent, EventKind, EventPayload};

pub struct WorldOverlayPolicy;

impl WorldOverlayPolicy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WorldOverlayPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for WorldOverlayPolicy {
    fn name(&self) -> &'static str {
        "WorldOverlayPolicy"
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
        let agent = WorldOverlayPolicy::new();
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
        let agent = WorldOverlayPolicy::new();
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
