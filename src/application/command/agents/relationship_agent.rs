//! RelAgent — 관계 갱신 전담
//!
//! B5.1: v1 `handle_update`/`handle_end_dialogue`는 deprecated. v2 `impl EventHandler` 사용.

#![allow(deprecated)]

use crate::application::command::handler::{HandlerContext, HandlerOutput};
use crate::application::command::types::CommandResult;
use crate::application::mind_service::MindServiceError;
use crate::application::relationship_service::RelationshipService;
use crate::application::dto::AfterDialogueRequest;
use crate::domain::event::EventPayload;

/// 관계 갱신 에이전트
pub struct RelationshipAgent {
    service: RelationshipService,
}

impl RelationshipAgent {
    pub fn new() -> Self {
        Self {
            service: RelationshipService::new(),
        }
    }

    /// UpdateRelationship Command 처리 (Beat 종료) — **v1, deprecated**
    #[deprecated(
        since = "0.2.0",
        note = "v2 `impl EventHandler for RelationshipAgent` (RelationshipUpdateRequested 수신) 사용. v0.3.0에서 제거 예정."
    )]
    #[allow(deprecated)]
    pub fn handle_update(
        &self,
        npc_id: &str,
        partner_id: &str,
        significance: &Option<f32>,
        ctx: &HandlerContext,
    ) -> Result<HandlerOutput, MindServiceError> {
        let rel = ctx.relationship.as_ref().ok_or_else(|| {
            MindServiceError::RelationshipNotFound(npc_id.into(), partner_id.into())
        })?;
        let emotion = ctx
            .emotion_state
            .as_ref()
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        let req = AfterDialogueRequest {
            npc_id: npc_id.to_string(),
            partner_id: partner_id.to_string(),
            significance: *significance,
        };
        let (new_rel, response) = self.service.update_relationship(rel, emotion, &req);

        let event = EventPayload::RelationshipUpdated {
            owner_id: npc_id.to_string(),
            target_id: partner_id.to_string(),
            before_closeness: response.before.closeness,
            before_trust: response.before.trust,
            before_power: response.before.power,
            after_closeness: response.after.closeness,
            after_trust: response.after.trust,
            after_power: response.after.power,
        };

        Ok(HandlerOutput {
            result: CommandResult::RelationshipUpdated(response),
            events: vec![event],
            new_emotion_state: None,
            new_relationship: Some((npc_id.to_string(), partner_id.to_string(), new_rel)),
            clear_emotion: None,
            clear_scene: false,
            save_scene: None,
        })
    }

    /// EndDialogue Command 처리 (관계 갱신 + 감정 초기화 + Scene 정리) — **v1, deprecated**
    #[deprecated(
        since = "0.2.0",
        note = "v2 `impl EventHandler for RelationshipAgent` (DialogueEndRequested 수신, 3 follow-ups 발행) 사용. v0.3.0에서 제거 예정."
    )]
    #[allow(deprecated)]
    pub fn handle_end_dialogue(
        &self,
        npc_id: &str,
        partner_id: &str,
        significance: &Option<f32>,
        ctx: &HandlerContext,
    ) -> Result<HandlerOutput, MindServiceError> {
        let rel = ctx.relationship.as_ref().ok_or_else(|| {
            MindServiceError::RelationshipNotFound(npc_id.into(), partner_id.into())
        })?;
        let emotion = ctx
            .emotion_state
            .as_ref()
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        let req = AfterDialogueRequest {
            npc_id: npc_id.to_string(),
            partner_id: partner_id.to_string(),
            significance: *significance,
        };
        let (new_rel, response) = self.service.update_relationship(rel, emotion, &req);

        let rel_event = EventPayload::RelationshipUpdated {
            owner_id: npc_id.to_string(),
            target_id: partner_id.to_string(),
            before_closeness: response.before.closeness,
            before_trust: response.before.trust,
            before_power: response.before.power,
            after_closeness: response.after.closeness,
            after_trust: response.after.trust,
            after_power: response.after.power,
        };

        let clear_event = EventPayload::EmotionCleared {
            npc_id: npc_id.to_string(),
        };

        let scene_event = EventPayload::SceneEnded {
            npc_id: npc_id.to_string(),
            partner_id: partner_id.to_string(),
        };

        Ok(HandlerOutput {
            result: CommandResult::DialogueEnded(response),
            events: vec![rel_event, clear_event, scene_event],
            new_relationship: Some((npc_id.to_string(), partner_id.to_string(), new_rel)),
            new_emotion_state: None,
            clear_emotion: Some(npc_id.to_string()),
            clear_scene: true,
            save_scene: None,
        })
    }
}

impl Default for RelationshipAgent {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// B1 — EventHandler impl (v2 진입점)
// ===========================================================================
//
// v2에서는 `SceneEnded` / `BeatTransitioned` 이벤트에 **자동 반응**하여
// 관계를 갱신한다. v1의 `handle_update` / `handle_end_dialogue`는
// Dispatcher가 여전히 사용(EndDialogue/UpdateRelationship 커맨드 경로).
//
// SceneEnded: final 관계 갱신(significance=기본값, 실제 값은 B3+에서 Command payload로 주입).
// BeatTransitioned: Beat-level 관계 갱신(BEAT_DEFAULT_SIGNIFICANCE).
//
// **v1/v2 의미론 차이 (의도):** v1의 `EmotionAgent.handle_beat_transition`은 관계 갱신 시
// **pre-merge 감정(`stimulated`)** 을 썼다. v2는 `ctx.shared.emotion_state`(StimulusAgent가
// merge 후 설정한 **post-merge 감정**)을 입력으로 받는다. Beat 전환 후의 최종 감정 상태를
// 반영하는 것이 의미상 자연스러우므로 채택된 개선. B3 parallel run에서는 closeness/trust/power
// 수치가 살짝 달라질 수 있으므로 expected diff로 처리 필요.
//
// B-Plan §6.2는 `follow_up: no`로 표기되지만, RelationshipUpdated는 하류에서 필요한
// 터미널 이벤트이므로 follow_up으로 발행한다(cascade하지 않아도 event_store에는 기록).

use crate::application::command::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::event::{DomainEvent, EventKind};
use crate::domain::tuning::BEAT_DEFAULT_SIGNIFICANCE;

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
                self.handle_relationship_update(
                    npc_id,
                    partner_id,
                    BEAT_DEFAULT_SIGNIFICANCE,
                    ctx,
                )
            }

            _ => Ok(HandlerResult::default()),
        }
    }
}

// Helper methods for RelationshipAgent's EventHandler impl.
impl RelationshipAgent {
    /// 공용 관계 갱신 로직 — BeatTransitioned + RelationshipUpdateRequested 공유
    fn handle_relationship_update(
        &self,
        npc_id: &str,
        partner_id: &str,
        significance: f32,
        ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        let relationship = ctx
            .repo
            .get_relationship(npc_id, partner_id)
            .ok_or(HandlerError::Precondition("relationship not found"))?;
        let emotion = ctx
            .shared
            .emotion_state
            .clone()
            .or_else(|| ctx.repo.get_emotion_state(npc_id))
            .ok_or(HandlerError::Precondition("emotion state not found"))?;

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
            .ok_or(HandlerError::Precondition("relationship not found"))?;
        let emotion = ctx
            .shared
            .emotion_state
            .clone()
            .or_else(|| ctx.repo.get_emotion_state(npc_id))
            .ok_or(HandlerError::Precondition("emotion state not found"))?;

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
            HandlerError::Precondition("relationship not found")
        ));
    }
}
