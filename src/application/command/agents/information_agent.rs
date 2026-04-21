//! InformationAgent — 정보 전달 팬아웃 (Step C2, Mind 컨텍스트)
//!
//! `TellInformationRequested` 1개를 받아 listeners + overhearers 각각에 대해
//! `InformationTold` follow-up 이벤트를 발행한다 (B5: 청자당 1 이벤트 패턴).
//! 실제 `MemoryEntry` 생성은 Inline `TellingIngestionHandler`가 담당하며, 이 에이전트는
//! 순수 팬아웃 오케스트레이터다.
//!
//! **왜 Transactional인가**: follow-up 이벤트를 발행해야 하고, 해당 이벤트가 같은
//! 커맨드 commit에 묶여 EventStore에 기록되어야 하기 때문. Inline은 commit 후 실행이라
//! 이벤트 발행 채널이 아니다.

use crate::application::command::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::event::{DomainEvent, EventKind, EventPayload, ListenerRole};

pub struct InformationAgent;

impl InformationAgent {
    pub fn new() -> Self {
        Self
    }
}

impl Default for InformationAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for InformationAgent {
    fn name(&self) -> &'static str {
        "InformationAgent"
    }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![EventKind::TellInformationRequested])
    }

    fn mode(&self) -> DeliveryMode {
        DeliveryMode::Transactional {
            priority: priority::transactional::INFORMATION_TELLING,
            can_emit_follow_up: true,
        }
    }

    fn handle(
        &self,
        event: &DomainEvent,
        _ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        let EventPayload::TellInformationRequested {
            speaker,
            listeners,
            overhearers,
            claim,
            stated_confidence,
            origin_chain_in,
        } = &event.payload
        else {
            return Ok(HandlerResult::default());
        };

        // stated_confidence 방어적 클램프 — 외부 DTO는 자유로운 범위가 올 수 있다.
        let stated = stated_confidence.clamp(0.0, 1.0);

        let mut follow_ups = Vec::with_capacity(listeners.len() + overhearers.len());
        for listener in listeners {
            follow_ups.push(DomainEvent::new(
                0,
                listener.clone(), // aggregate_id = 청자 — B5 라우팅 (§3.3)
                0,
                EventPayload::InformationTold {
                    speaker: speaker.clone(),
                    listener: listener.clone(),
                    listener_role: ListenerRole::Direct,
                    claim: claim.clone(),
                    stated_confidence: stated,
                    origin_chain_in: origin_chain_in.clone(),
                },
            ));
        }
        for listener in overhearers {
            follow_ups.push(DomainEvent::new(
                0,
                listener.clone(),
                0,
                EventPayload::InformationTold {
                    speaker: speaker.clone(),
                    listener: listener.clone(),
                    listener_role: ListenerRole::Overhearer,
                    claim: claim.clone(),
                    stated_confidence: stated,
                    origin_chain_in: origin_chain_in.clone(),
                },
            ));
        }

        Ok(HandlerResult {
            follow_up_events: follow_ups,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::command::handler_v2::test_support::HandlerTestHarness;

    fn request_event(
        speaker: &str,
        listeners: &[&str],
        overhearers: &[&str],
        chain: &[&str],
    ) -> DomainEvent {
        DomainEvent::new(
            0,
            speaker.into(),
            0,
            EventPayload::TellInformationRequested {
                speaker: speaker.into(),
                listeners: listeners.iter().map(|s| s.to_string()).collect(),
                overhearers: overhearers.iter().map(|s| s.to_string()).collect(),
                claim: "claim".into(),
                stated_confidence: 0.8,
                origin_chain_in: chain.iter().map(|s| s.to_string()).collect(),
            },
        )
    }

    #[test]
    fn emits_one_information_told_per_listener_with_direct_role() {
        let agent = InformationAgent::new();
        let mut harness = HandlerTestHarness::new();
        let event = request_event("sage", &["pupil-a", "pupil-b"], &[], &[]);

        let result = harness.dispatch(&agent, event).expect("must succeed");

        assert_eq!(result.follow_up_events.len(), 2);
        for ev in &result.follow_up_events {
            assert_eq!(ev.kind(), EventKind::InformationTold);
            let EventPayload::InformationTold { listener_role, .. } = &ev.payload else {
                panic!("expected InformationTold");
            };
            assert_eq!(*listener_role, ListenerRole::Direct);
        }
    }

    #[test]
    fn overhearers_get_overhearer_role() {
        let agent = InformationAgent::new();
        let mut harness = HandlerTestHarness::new();
        let event = request_event("sage", &["pupil"], &["wanderer-a", "wanderer-b"], &[]);

        let result = harness.dispatch(&agent, event).expect("must succeed");

        assert_eq!(result.follow_up_events.len(), 3);
        let roles: Vec<ListenerRole> = result
            .follow_up_events
            .iter()
            .map(|ev| match &ev.payload {
                EventPayload::InformationTold { listener_role, .. } => *listener_role,
                _ => panic!("unexpected"),
            })
            .collect();
        assert_eq!(
            roles,
            vec![
                ListenerRole::Direct,
                ListenerRole::Overhearer,
                ListenerRole::Overhearer
            ]
        );
    }

    #[test]
    fn information_told_aggregate_key_is_listener() {
        let agent = InformationAgent::new();
        let mut harness = HandlerTestHarness::new();
        let event = request_event("sage", &["pupil"], &[], &[]);

        let result = harness.dispatch(&agent, event).expect("must succeed");
        assert_eq!(result.follow_up_events.len(), 1);
        assert_eq!(
            result.follow_up_events[0].aggregate_key(),
            crate::domain::aggregate::AggregateKey::Npc("pupil".into()),
            "B5: InformationTold must route to listener"
        );
    }

    #[test]
    fn origin_chain_in_is_passed_through_untouched() {
        let agent = InformationAgent::new();
        let mut harness = HandlerTestHarness::new();
        let event = request_event("relay", &["final"], &[], &["prior-a", "prior-b"]);

        let result = harness.dispatch(&agent, event).expect("must succeed");
        let EventPayload::InformationTold {
            origin_chain_in, ..
        } = &result.follow_up_events[0].payload
        else {
            panic!("expected InformationTold");
        };
        // 체인 확장(“speaker prepend")은 TellingIngestionHandler 책임이므로 여기서는 그대로.
        assert_eq!(
            origin_chain_in,
            &vec!["prior-a".to_string(), "prior-b".to_string()]
        );
    }

    #[test]
    fn stated_confidence_is_clamped_to_unit_range() {
        let agent = InformationAgent::new();
        let mut harness = HandlerTestHarness::new();
        let event = DomainEvent::new(
            0,
            "sage".into(),
            0,
            EventPayload::TellInformationRequested {
                speaker: "sage".into(),
                listeners: vec!["p".into()],
                overhearers: vec![],
                claim: "c".into(),
                stated_confidence: 1.5,
                origin_chain_in: vec![],
            },
        );
        let result = harness.dispatch(&agent, event).expect("must succeed");
        let EventPayload::InformationTold {
            stated_confidence, ..
        } = &result.follow_up_events[0].payload
        else {
            panic!("expected InformationTold");
        };
        assert_eq!(*stated_confidence, 1.0);
    }

    #[test]
    fn empty_listeners_and_overhearers_emit_no_follow_ups() {
        let agent = InformationAgent::new();
        let mut harness = HandlerTestHarness::new();
        let event = request_event("sage", &[], &[], &[]);
        let result = harness.dispatch(&agent, event).expect("must succeed");
        assert!(result.follow_up_events.is_empty());
    }

    #[test]
    fn ignores_unrelated_event_kind() {
        let agent = InformationAgent::new();
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
