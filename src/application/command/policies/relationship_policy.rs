//! RelationshipPolicy — 관계 갱신 전담 (v2)
//!
//! `BeatTransitioned` / `DialogueEndRequested` / `RelationshipUpdateRequested` 이벤트에
//! 반응하여 관계를 갱신한다. `UnitOfWork`에 설정된 감정 상태(StimulusPolicy가 merge 후 설정한
//! post-merge 감정)를 입력으로 받는다.

use crate::application::command::handler_v2::{
    DeliveryMode, DynamicHandlerContext, EventHandler, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::event::{DomainEvent, EventKind, EventPayload};
use crate::domain::tuning::profile;

/// 관계 갱신 폴리시
pub struct RelationshipPolicy;

impl RelationshipPolicy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RelationshipPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for RelationshipPolicy {
    fn name(&self) -> &'static str {
        "RelationshipPolicy"
    }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![
            // Beat 전환 반응 — 관계 갱신
            EventKind::BeatTransitioned,
            // B4.1: UpdateRelationship 커맨드 초기 이벤트
            EventKind::RelationshipUpdateRequested,
            // B4.1: EndDialogue 커맨드 초기 이벤트 — 3 follow-ups 발행
            EventKind::DialogueEndRequested,
        ])
    }

    fn mode(&self) -> DeliveryMode {
        DeliveryMode::Transactional {
            priority: priority::transactional::RELATIONSHIP_UPDATE,
            can_emit_follow_up: true,
        }
    }

    fn handle_v2(
        &self,
        event: &DomainEvent,
        ctx: &mut dyn DynamicHandlerContext,
    ) -> Result<HandlerResult, HandlerError> {
        // 이벤트별 분기 — DialogueEndRequested는 3 follow-ups + clear 시그널을 별도 경로로 처리
        match &event.payload {
            EventPayload::DialogueEndRequested {
                npc_id,
                partner_id,
                significance,
                reflection: _,
            } => self.handle_dialogue_end(npc_id, partner_id, *significance, ctx),

            EventPayload::RelationshipUpdateRequested {
                npc_id,
                partner_id,
                significance,
            } => self.handle_relationship_update(
                npc_id,
                partner_id,
                significance.unwrap_or(profile().beat_default_significance),
                ctx,
            ),

            EventPayload::BeatTransitioned {
                npc_id, partner_id, ..
            } => {
                // B4 Session 3 (Option A): payload에 partner_id가 추가되어 multi-scene
                // 오동작 수정.
                self.handle_relationship_update_with_cause(
                    npc_id,
                    partner_id,
                    profile().beat_default_significance,
                    crate::domain::event::RelationshipChangeCause::SceneInteraction {
                        scene_id: crate::domain::scene_id::SceneId::new(
                            npc_id.clone(),
                            partner_id.clone(),
                        ),
                    },
                    ctx,
                )
            }

            _ => Ok(HandlerResult::default()),
        }
    }
}

// Helper methods for RelationshipPolicy's EventHandler impl.
impl RelationshipPolicy {
    /// 공용 관계 갱신 로직 — `RelationshipUpdateRequested` (cause 미확정) 경로용.
    fn handle_relationship_update(
        &self,
        npc_id: &str,
        partner_id: &str,
        significance: f32,
        ctx: &mut dyn DynamicHandlerContext,
    ) -> Result<HandlerResult, HandlerError> {
        self.handle_relationship_update_with_cause(
            npc_id,
            partner_id,
            significance,
            crate::domain::event::RelationshipChangeCause::Unspecified,
            ctx,
        )
    }

    /// cause를 명시적으로 지정해 관계 갱신 이벤트를 발행한다 (Step D 확장).
    fn handle_relationship_update_with_cause(
        &self,
        npc_id: &str,
        partner_id: &str,
        significance: f32,
        cause: crate::domain::event::RelationshipChangeCause,
        ctx: &mut dyn DynamicHandlerContext,
    ) -> Result<HandlerResult, HandlerError> {
        let relationship = ctx.get_relationship(npc_id, partner_id)?;
        let emotion = ctx.get_emotion_state(npc_id)?;

        let updated = relationship.after_dialogue(&emotion, significance);
        let (bc, bt, bp) = (
            relationship.closeness().value(),
            relationship.trust().value(),
            relationship.power().value(),
        );
        let (ac, at, ap) = (
            updated.closeness().value(),
            updated.trust().value(),
            updated.power().value(),
        );
        ctx.save_relationship(updated);

        let follow_up = DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::RelationshipUpdated(Box::new(
                crate::domain::event::RelationshipUpdatedPayload {
                    owner_id: npc_id.to_string(),
                    target_id: partner_id.to_string(),
                    before_closeness: bc,
                    before_trust: bt,
                    before_power: bp,
                    after_closeness: ac,
                    after_trust: at,
                    after_power: ap,
                    cause,
                },
            )),
        );
        Ok(HandlerResult {
            follow_up_events: vec![follow_up],
        })
    }

    /// DialogueEnd — 관계 갱신 + 감정 clear + scene clear. 3 follow-ups.
    fn handle_dialogue_end(
        &self,
        npc_id: &str,
        partner_id: &str,
        significance: Option<f32>,
        ctx: &mut dyn DynamicHandlerContext,
    ) -> Result<HandlerResult, HandlerError> {
        let sig = significance.unwrap_or(profile().beat_default_significance);
        let relationship = ctx.get_relationship(npc_id, partner_id)?;
        let emotion = ctx.get_emotion_state(npc_id)?;

        let updated = relationship.after_dialogue(&emotion, sig);
        let (bc, bt, bp) = (
            relationship.closeness().value(),
            relationship.trust().value(),
            relationship.power().value(),
        );
        let (ac, at, ap) = (
            updated.closeness().value(),
            updated.trust().value(),
            updated.power().value(),
        );
        ctx.save_relationship(updated);
        ctx.clear_emotion_for(npc_id.to_string());
        ctx.clear_scene();

        // 3 follow-ups: RelationshipUpdated + EmotionCleared + SceneEnded
        let rel_event = DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::RelationshipUpdated(Box::new(
                crate::domain::event::RelationshipUpdatedPayload {
                    owner_id: npc_id.to_string(),
                    target_id: partner_id.to_string(),
                    before_closeness: bc,
                    before_trust: bt,
                    before_power: bp,
                    after_closeness: ac,
                    after_trust: at,
                    after_power: ap,
                    cause: crate::domain::event::RelationshipChangeCause::Unspecified,
                },
            )),
        );
        let clear_event = DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::EmotionCleared {
                npc_id: npc_id.to_string(),
            },
        );
        let scene_event = DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::SceneEnded {
                npc_id: npc_id.to_string(),
                partner_id: partner_id.to_string(),
            },
        );
        Ok(HandlerResult {
            follow_up_events: vec![rel_event, clear_event, scene_event],
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
    use crate::application::command::handler_v2::HandlerError;
    use crate::domain::emotion::{EmotionState, EventFocus, FocusTrigger, Scene, SceneFocus};
    use crate::domain::event::{DomainEvent, EventKind, EventPayload};
    use crate::domain::relationship::Relationship;

    fn make_scene_ended(npc_id: &str, partner_id: &str) -> DomainEvent {
        DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::SceneEnded {
                npc_id: npc_id.to_string(),
                partner_id: partner_id.to_string(),
            },
        )
    }

    fn make_dialogue_end(npc_id: &str, partner_id: &str, significance: Option<f32>) -> DomainEvent {
        DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::DialogueEndRequested {
                npc_id: npc_id.to_string(),
                partner_id: partner_id.to_string(),
                significance,
                reflection: None,
            },
        )
    }

    fn make_beat_transitioned(npc_id: &str, partner_id: &str) -> DomainEvent {
        DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::BeatTransitioned {
                npc_id: npc_id.to_string(),
                partner_id: partner_id.to_string(),
                from_focus_id: Some("initial".into()),
                to_focus_id: "next".into(),
            },
        )
    }

    fn minimal_focus(id: &str, trigger: FocusTrigger) -> SceneFocus {
        SceneFocus {
            id: id.into(),
            description: id.into(),
            trigger,
            event: Some(EventFocus {
                description: "".into(),
                desirability_for_self: 0.1,
                desirability_for_other: None,
                prospect: None,
            }),
            action: None,
            object: None,
            test_script: vec![],
        }
    }

    #[test]
    fn dialogue_end_emits_three_follow_ups_and_sets_clear_signals() {
        let policy = RelationshipPolicy::new();
        let rel = Relationship::neutral("alice", "bob");
        let mut harness = HandlerTestHarness::new()
            .with_relationship(rel)
            .with_emotion_state("alice", EmotionState::default());

        let event = make_dialogue_end("alice", "bob", Some(0.8));
        let (result, uow) = harness.dispatch(&policy, event).expect("must succeed");

        // 3 follow-ups: RelationshipUpdated, EmotionCleared, SceneEnded
        assert_eq!(result.follow_up_events.len(), 3);
        let kinds: Vec<EventKind> = result.follow_up_events.iter().map(|e: &DomainEvent| e.kind()).collect();
        assert_eq!(
            kinds,
            vec![
                EventKind::RelationshipUpdated,
                EventKind::EmotionCleared,
                EventKind::SceneEnded,
            ]
        );

        // Clear 시그널
        assert_eq!(
            uow.clear_emotion_for.as_deref(),
            Some("alice"),
            "EmotionCleared를 위해 npc_id 기록"
        );
        assert!(uow.clear_scene, "SceneEnded를 위해 flag 설정");
    }

    #[test]
    fn scene_ended_no_longer_in_interest_produces_no_follow_ups() {
        let policy = RelationshipPolicy::new();
        let rel = Relationship::neutral("alice", "bob");
        let mut harness = HandlerTestHarness::new()
            .with_relationship(rel)
            .with_emotion_state("alice", EmotionState::default());

        let event = make_scene_ended("alice", "bob");
        let (result, _) = harness
            .dispatch(&policy, event)
            .expect("interest 밖 이벤트는 no-op");
        assert!(result.follow_up_events.is_empty());
    }

    #[test]
    fn beat_transitioned_uses_active_scene_for_partner_id() {
        let policy = RelationshipPolicy::new();
        let rel = Relationship::neutral("alice", "charlie");
        let scene = Scene::new(
            "alice".into(),
            "charlie".into(),
            vec![minimal_focus("initial", FocusTrigger::Initial)],
        );

        let mut harness = HandlerTestHarness::new()
            .with_relationship(rel)
            .with_scene(scene)
            .with_emotion_state("alice", EmotionState::default());

        let event = make_beat_transitioned("alice", "charlie");
        let (result, _) = harness
            .dispatch(&policy, event)
            .expect("should derive partner from scene");

        assert_eq!(result.follow_up_events.len(), 1);
        let EventPayload::RelationshipUpdated(p) = &result.follow_up_events[0].payload else {
            panic!("expected RelationshipUpdated")
        };
        assert_eq!(p.target_id, "charlie");
    }

    #[test]
    fn missing_relationship_returns_precondition_error() {
        let policy = RelationshipPolicy::new();
        let mut harness =
            HandlerTestHarness::new().with_emotion_state("alice", EmotionState::default());

        let event = make_dialogue_end("alice", "bob", None);
        let err = harness.dispatch(&policy, event).expect_err("must fail");

        assert!(matches!(
            err,
            HandlerError::RelationshipNotFound { ref owner_id, ref target_id }
                if owner_id == "alice" && target_id == "bob"
        ));
    }
}
