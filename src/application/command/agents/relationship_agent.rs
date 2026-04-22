//! RelationshipAgent — 관계 갱신 전담 (v2)
//!
//! `BeatTransitioned` / `DialogueEndRequested` / `RelationshipUpdateRequested` 이벤트에
//! 반응하여 관계를 갱신한다. `ctx.shared.emotion_state`(StimulusAgent가 merge 후 설정한
//! post-merge 감정)를 입력으로 받는다.

use crate::application::command::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::event::{DomainEvent, EventKind, EventPayload};
use crate::domain::tuning::BEAT_DEFAULT_SIGNIFICANCE;

/// 관계 갱신 에이전트
pub struct RelationshipAgent;

impl RelationshipAgent {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RelationshipAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for RelationshipAgent {
    fn name(&self) -> &'static str {
        "RelationshipAgent"
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

    fn handle(
        &self,
        event: &DomainEvent,
        ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        // 이벤트별 분기 — DialogueEndRequested는 3 follow-ups + clear 시그널을 별도 경로로 처리
        match &event.payload {
            EventPayload::DialogueEndRequested {
                npc_id,
                partner_id,
                significance,
            } => self.handle_dialogue_end(npc_id, partner_id, *significance, ctx),

            EventPayload::RelationshipUpdateRequested {
                npc_id,
                partner_id,
                significance,
            } => self.handle_relationship_update(
                npc_id,
                partner_id,
                significance.unwrap_or(BEAT_DEFAULT_SIGNIFICANCE),
                ctx,
            ),

            EventPayload::BeatTransitioned {
                npc_id, partner_id, ..
            } => {
                // B4 Session 3 (Option A): payload에 partner_id가 추가되어 multi-scene
                // 오동작 수정. 이전에는 `ctx.repo.get_scene()` fallback이 `last_scene_id`를
                // 읽어 다중 Scene 환경에서 **잘못된 Scene의 관계**를 갱신할 수 있었다.
                //
                // Step D: cause를 `SceneInteraction { scene_id }` 로 설정해
                // `RelationshipMemoryHandler`가 관점을 분기할 수 있게 한다.
                self.handle_relationship_update_with_cause(
                    npc_id,
                    partner_id,
                    BEAT_DEFAULT_SIGNIFICANCE,
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

// Helper methods for RelationshipAgent's EventHandler impl.
impl RelationshipAgent {
    /// 공용 관계 갱신 로직 — `RelationshipUpdateRequested` (cause 미확정) 경로용.
    fn handle_relationship_update(
        &self,
        npc_id: &str,
        partner_id: &str,
        significance: f32,
        ctx: &mut EventHandlerContext<'_>,
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
        ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        let relationship = ctx
            .repo
            .get_relationship(npc_id, partner_id)
            .ok_or_else(|| HandlerError::RelationshipNotFound {
                owner_id: npc_id.to_string(),
                target_id: partner_id.to_string(),
            })?;
        let emotion = ctx
            .shared
            .emotion_state
            .clone()
            .or_else(|| ctx.repo.get_emotion_state(npc_id))
            .ok_or_else(|| HandlerError::EmotionStateNotFound(npc_id.to_string()))?;

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
        ctx.shared.relationship = Some(updated);

        let follow_up = DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::RelationshipUpdated {
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
        );
        Ok(HandlerResult {
            follow_up_events: vec![follow_up],
        })
    }

    /// DialogueEnd — 관계 갱신 + 감정 clear + scene clear. 3 follow-ups.
    ///
    /// v1 `RelationshipAgent::handle_end_dialogue` 등가. 차이점:
    /// - v1: HandlerOutput의 `clear_emotion` / `clear_scene` 플래그로 Dispatcher에 지시
    /// - v2: `ctx.shared.clear_emotion_for` / `ctx.shared.clear_scene` 시그널 설정 →
    ///   Dispatcher의 `apply_shared_to_repository`가 commit phase 후 실행
    fn handle_dialogue_end(
        &self,
        npc_id: &str,
        partner_id: &str,
        significance: Option<f32>,
        ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        let sig = significance.unwrap_or(BEAT_DEFAULT_SIGNIFICANCE);
        let relationship = ctx
            .repo
            .get_relationship(npc_id, partner_id)
            .ok_or_else(|| HandlerError::RelationshipNotFound {
                owner_id: npc_id.to_string(),
                target_id: partner_id.to_string(),
            })?;
        let emotion = ctx
            .shared
            .emotion_state
            .clone()
            .or_else(|| ctx.repo.get_emotion_state(npc_id))
            .ok_or_else(|| HandlerError::EmotionStateNotFound(npc_id.to_string()))?;

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
        ctx.shared.relationship = Some(updated);
        ctx.shared.clear_emotion_for = Some(npc_id.to_string());
        ctx.shared.clear_scene = true;

        // 3 follow-ups: RelationshipUpdated + EmotionCleared + SceneEnded
        // SceneEnded는 터미널 이벤트 — 다른 transactional handler가 interest 가지지 않음
        // (RelationshipAgent 본인도 interest에서 SceneEnded 제외했으므로 재진입 없음).
        // TODO(step-f): DialogueEnd는 장면 종료 직전 관계 정산이므로 cause는 의미상
        // `SceneInteraction { scene_id }`에 가깝다. 다만 DialogueEndRequested 페이로드는
        // scene_id를 직접 운반하지 않고 (npc_id, partner_id)로부터 합성되며, 장면이
        // end 직전까지 유효한지 확인하지 않고 cause를 단정하면 인과 오표기 위험이 있다.
        // Step F에서 DialogueEndRequested에 scene_id를 명시 필드로 추가하거나
        // 장면 유효성 검증 후 cause를 `SceneInteraction`으로 승격할 것.
        let rel_event = DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::RelationshipUpdated {
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
        let agent = RelationshipAgent::new();
        let rel = Relationship::neutral("alice", "bob");
        let mut harness = HandlerTestHarness::new()
            .with_relationship(rel)
            .with_shared_emotion_state(EmotionState::default());

        let event = make_dialogue_end("alice", "bob", Some(0.8));
        let result = harness.dispatch(&agent, event).expect("must succeed");

        // 3 follow-ups: RelationshipUpdated, EmotionCleared, SceneEnded
        assert_eq!(result.follow_up_events.len(), 3);
        let kinds: Vec<_> = result.follow_up_events.iter().map(|e| e.kind()).collect();
        assert_eq!(
            kinds,
            vec![
                EventKind::RelationshipUpdated,
                EventKind::EmotionCleared,
                EventKind::SceneEnded,
            ]
        );

        // Clear 시그널 — Dispatcher의 apply_shared_to_repository가 commit 후 실행
        assert_eq!(
            harness.shared.clear_emotion_for.as_deref(),
            Some("alice"),
            "EmotionCleared를 위해 npc_id 기록"
        );
        assert!(harness.shared.clear_scene, "SceneEnded를 위해 flag 설정");
    }

    #[test]
    fn scene_ended_no_longer_in_interest_produces_no_follow_ups() {
        // B4.1: RelationshipAgent는 더 이상 SceneEnded에 반응하지 않는다
        //       (DialogueEndRequested가 그 역할을 담당).
        let agent = RelationshipAgent::new();
        let rel = Relationship::neutral("alice", "bob");
        let mut harness = HandlerTestHarness::new()
            .with_relationship(rel)
            .with_shared_emotion_state(EmotionState::default());

        let event = make_scene_ended("alice", "bob");
        let result = harness
            .dispatch(&agent, event)
            .expect("interest 밖 이벤트는 no-op");
        assert!(result.follow_up_events.is_empty());
    }

    #[test]
    fn beat_transitioned_uses_active_scene_for_partner_id() {
        let agent = RelationshipAgent::new();
        let rel = Relationship::neutral("alice", "charlie");
        // Scene의 partner_id="charlie"가 BeatTransitioned의 partner 도출원
        let scene = Scene::new(
            "alice".into(),
            "charlie".into(),
            vec![minimal_focus("initial", FocusTrigger::Initial)],
        );

        let mut harness = HandlerTestHarness::new()
            .with_relationship(rel)
            .with_scene(scene)
            .with_shared_emotion_state(EmotionState::default());

        let event = make_beat_transitioned("alice", "charlie");
        let result = harness
            .dispatch(&agent, event)
            .expect("should derive partner from scene");

        assert_eq!(result.follow_up_events.len(), 1);
        let EventPayload::RelationshipUpdated { target_id, .. } =
            &result.follow_up_events[0].payload
        else {
            panic!("expected RelationshipUpdated")
        };
        assert_eq!(target_id, "charlie");
    }

    #[test]
    fn missing_relationship_returns_precondition_error() {
        let agent = RelationshipAgent::new();
        // relationship 미주입
        let mut harness =
            HandlerTestHarness::new().with_shared_emotion_state(EmotionState::default());

        // B4.1: DialogueEndRequested로 변경 (SceneEnded는 이제 interest 밖)
        let event = make_dialogue_end("alice", "bob", None);
        let err = harness.dispatch(&agent, event).expect_err("must fail");

        assert!(matches!(
            err,
            HandlerError::RelationshipNotFound { ref owner_id, ref target_id }
                if owner_id == "alice" && target_id == "bob"
        ));
    }
}
