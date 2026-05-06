//! StimulusPolicy — PAD 자극 적용 + Beat 전환 판정 전담 (B안 B1)
//!
//! 기존 `EmotionPolicy.handle_stimulus` 로직의 v2 포팅.
//! 현재 Dispatcher는 여전히 `EmotionPolicy.handle_stimulus`를 호출하며,
//! 이 Policy는 B3 `dispatch_v2()`가 생겨야 실제 호출된다. B1은 타입·테스트 준비 단계.
//!
//! **v1/v2 차이:**
//! - v1: `EmotionPolicy.handle_stimulus`가 Beat 전환 시점에 `RelationshipUpdated` 이벤트를 inline으로 발행.
//!   관계 갱신은 **pre-merge 감정(`stimulated`)** 기반.
//! - v2: `StimulusPolicy`는 `StimulusApplied` + `BeatTransitioned`만 follow_up으로 발행하고,
//!   관계 갱신은 후속 `RelationshipPolicy`(우선순위 30)가 `BeatTransitioned`를 받아 처리.
//!   이때 `RelationshipPolicy`는 `ctx.shared.emotion_state`(= **merged 감정**)을 입력으로 쓴다.
//!
//! **의도적 의미론 변경 (v2 개선):** Beat 전환 후 관계 갱신은 "전환 완료 후 최종 감정 상태"를
//! 반영하는 것이 의미상 자연스러우므로 v2는 `merged` 기반을 채택. v1의 `stimulated` 기반은
//! inline 발행을 위한 실용적 선택이었음. B3 parallel run 테스트에서는 이 차이가
//! **expected diff**로 분류되어야 하며, 의미적 동등성(감정 방향·Beat 트리거 일치) 관점에서만
//! 비교한다.
//!
//! 이로써 책임 분리가 명확해지며, B-Plan §6.2 우선순위 테이블에 정합한다.

use crate::application::command::handler_v2::{
    DeliveryMode, DynamicHandlerContext, EventHandler, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::emotion::{AppraisalEngine, EmotionState, StimulusEngine};
use crate::domain::event::{DomainEvent, EventKind, EventPayload};
use crate::domain::pad::Pad;
use crate::domain::scene_id::SceneId;
use crate::domain::tuning::profile;
use crate::ports::{Appraiser, StimulusProcessor};

/// PAD 자극 적용 + Beat 전환 판정 폴리시
///
/// Appraisal/Stimulus/Scene 평가기를 모두 소유. Scene trigger 체크는 도메인
/// `Scene::check_trigger`를 직접 호출(v1의 `SceneService` 래퍼 대신).
pub struct StimulusPolicy {
    appraiser: AppraisalEngine,
    stimulus_processor: StimulusEngine,
}

impl StimulusPolicy {
    pub fn new() -> Self {
        Self {
            appraiser: AppraisalEngine,
            stimulus_processor: StimulusEngine,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn process_beat_transition(
        &self,
        ctx: &mut dyn DynamicHandlerContext,
        npc_id: &str,
        partner_id: &str,
        npc: crate::domain::personality::Npc,
        relationship: crate::domain::relationship::Relationship,
        pad: &Pad,
        mood_before: f32,
        stimulated: &EmotionState,
        scene: &crate::domain::emotion::Scene,
        focus: &crate::domain::emotion::SceneFocus,
    ) -> Result<HandlerResult, HandlerError> {
        let from_focus_id = scene.active_focus_id().map(|s| s.to_string());
        let situation = focus.to_situation().map_err(|e| {
            HandlerError::InvalidInput(format!("focus to_situation failed: {e}"))
        })?;

        // Beat 전환용 임시 관계 갱신(modifiers 계산용 — 실제 저장은 RelationshipPolicy)
        let tuning = profile();
        let beat_rel = relationship.after_dialogue(stimulated, tuning.beat_default_significance);
        let new_state = self.appraiser.appraise(
            npc.personality(),
            &situation,
            &beat_rel.modifiers(),
        );
        let merged =
            EmotionState::merge_from_beat(stimulated, &new_state, tuning.beat_merge_threshold);

        // Scene을 active_focus = 새 focus로 갱신해 UoW에 전파.
        let mut new_scene = scene.clone();
        new_scene.set_active_focus(focus.id.clone());

        // UoW에 등록
        ctx.save_emotion_state(npc_id.to_string(), merged.clone());
        ctx.save_relationship(relationship.clone());
        ctx.save_scene(new_scene);

        let stimulus_event = DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::StimulusApplied(Box::new(
                crate::domain::event::StimulusAppliedPayload {
                    npc_id: npc_id.to_string(),
                    partner_id: partner_id.to_string(),
                    pad: (pad.pleasure, pad.arousal, pad.dominance),
                    mood_before,
                    mood_after: merged.overall_valence(),
                    beat_triggered: true,
                    emotion_snapshot: merged.snapshot(),
                },
            )),
        );
        let beat_event = DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::BeatTransitioned {
                npc_id: npc_id.to_string(),
                partner_id: partner_id.to_string(),
                from_focus_id,
                to_focus_id: focus.id.clone(),
            },
        );

        Ok(HandlerResult {
            follow_up_events: vec![stimulus_event, beat_event],
        })
    }

    fn process_simple_stimulus(
        &self,
        ctx: &mut dyn DynamicHandlerContext,
        npc_id: &str,
        partner_id: &str,
        relationship: crate::domain::relationship::Relationship,
        stimulated: &EmotionState,
        mood_before: f32,
        pad: &Pad,
    ) -> Result<HandlerResult, HandlerError> {
        ctx.save_emotion_state(npc_id.to_string(), stimulated.clone());
        ctx.save_relationship(relationship);

        let mood_after = stimulated.overall_valence();
        let stimulus_event = DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::StimulusApplied(Box::new(
                crate::domain::event::StimulusAppliedPayload {
                    npc_id: npc_id.to_string(),
                    partner_id: partner_id.to_string(),
                    pad: (pad.pleasure, pad.arousal, pad.dominance),
                    mood_before,
                    mood_after,
                    beat_triggered: false,
                    emotion_snapshot: stimulated.snapshot(),
                },
            )),
        );

        Ok(HandlerResult {
            follow_up_events: vec![stimulus_event],
        })
    }
}

impl Default for StimulusPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for StimulusPolicy {
    fn name(&self) -> &'static str {
        "StimulusPolicy"
    }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![EventKind::StimulusApplyRequested])
    }

    fn mode(&self) -> DeliveryMode {
        DeliveryMode::Transactional {
            priority: priority::transactional::STIMULUS_APPLICATION,
            can_emit_follow_up: true,
        }
    }

    fn handle_v2(
        &self,
        event: &DomainEvent,
        ctx: &mut dyn DynamicHandlerContext,
    ) -> Result<HandlerResult, HandlerError> {
        let EventPayload::StimulusApplyRequested {
            npc_id,
            partner_id,
            pad,
            situation_description: _,
        } = &event.payload
        else {
            return Ok(HandlerResult::default());
        };

        let npc = ctx.get_npc(npc_id)?;
        let relationship = ctx.get_relationship(npc_id, partner_id)?;
        let current = ctx.get_emotion_state(npc_id)?;

        let pad_struct = Pad {
            pleasure: pad.0,
            arousal: pad.1,
            dominance: pad.2,
        };
        let mood_before = current.overall_valence();

        let stimulated =
            self.stimulus_processor
                .apply_stimulus(npc.personality(), &current, &pad_struct);

        let scene_id = SceneId::new(npc_id, partner_id);
        if let Some(scene) = ctx.get_scene_by_id(&scene_id) {
            if let Some(focus) = scene.check_trigger(&stimulated).cloned() {
                return self.process_beat_transition(
                    ctx,
                    npc_id,
                    partner_id,
                    npc,
                    relationship,
                    &pad_struct,
                    mood_before,
                    &stimulated,
                    &scene,
                    &focus,
                );
            }
        }

        // Beat 전환 없음
        self.process_simple_stimulus(
            ctx,
            npc_id,
            partner_id,
            relationship,
            &stimulated,
            mood_before,
            &pad_struct,
        )
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
    use crate::domain::emotion::{
        ConditionThreshold, EmotionCondition, EmotionState, EmotionType, EventFocus, FocusTrigger,
        Scene, SceneFocus,
    };
    use crate::domain::event::{DomainEvent, EventKind, EventPayload};
    use crate::domain::personality::NpcBuilder;
    use crate::domain::relationship::Relationship;

    fn positive_event_focus() -> EventFocus {
        EventFocus {
            description: "".into(),
            desirability_for_self: 0.5,
            desirability_for_other: None,
            prospect: None,
        }
    }

    fn make_stim_request(npc_id: &str, partner_id: &str) -> DomainEvent {
        DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::StimulusApplyRequested {
                npc_id: npc_id.to_string(),
                partner_id: partner_id.to_string(),
                pad: (0.3, 0.1, 0.0),
                situation_description: None,
            },
        )
    }

    fn seed_emotion_state() -> EmotionState {
        EmotionState::default()
    }

    #[test]
    fn stimulus_without_scene_emits_single_stimulus_applied() {
        let policy = StimulusPolicy::new();
        let npc = NpcBuilder::new("alice", "Alice").build();
        let rel = Relationship::neutral("alice", "bob");
        let mut harness = HandlerTestHarness::new()
            .with_npc(npc)
            .with_relationship(rel)
            .with_emotion_state("alice", seed_emotion_state());

        let event = make_stim_request("alice", "bob");
        let (result, uow) = harness.dispatch(&policy, event).expect("handler must succeed");

        assert_eq!(result.follow_up_events.len(), 1);
        assert_eq!(result.follow_up_events[0].kind(), EventKind::StimulusApplied);

        let EventPayload::StimulusApplied(p) = &result.follow_up_events[0].payload else {
            panic!("expected StimulusApplied")
        };
        assert!(!p.beat_triggered);
        assert!(uow.emotion_state.is_some());
    }

    #[test]
    fn stimulus_with_triggered_scene_emits_stimulus_and_beat_transitioned() {
        let policy = StimulusPolicy::new();
        let npc = NpcBuilder::new("alice", "Alice").build();
        let rel = Relationship::neutral("alice", "bob");

        let scene = Scene::new(
            "alice".into(),
            "bob".into(),
            vec![
                SceneFocus {
                    id: "initial".into(),
                    description: "초기".into(),
                    trigger: FocusTrigger::Initial,
                    event: Some(positive_event_focus()),
                    action: None,
                    object: None,
                    test_script: vec![],
                },
                SceneFocus {
                    id: "next".into(),
                    description: "다음 Beat".into(),
                    trigger: FocusTrigger::Conditions(vec![vec![EmotionCondition {
                        emotion: EmotionType::Hate,
                        threshold: ConditionThreshold::Absent,
                    }]]),
                    event: Some(positive_event_focus()),
                    action: None,
                    object: None,
                    test_script: vec![],
                },
            ],
        );
        let mut scene = scene;
        scene.set_active_focus("initial".into());

        let mut harness = HandlerTestHarness::new()
            .with_npc(npc)
            .with_relationship(rel)
            .with_emotion_state("alice", seed_emotion_state())
            .with_scene(scene);

        let event = make_stim_request("alice", "bob");
        let (result, uow) = harness
            .dispatch(&policy, event)
            .expect("beat transition should succeed");

        assert_eq!(result.follow_up_events.len(), 2);

        let kinds: Vec<_> = result.follow_up_events.iter().map(|e| e.kind()).collect();
        assert_eq!(kinds, vec![EventKind::StimulusApplied, EventKind::BeatTransitioned]);

        let EventPayload::StimulusApplied(p) = &result.follow_up_events[0].payload else {
            panic!("expected StimulusApplied")
        };
        assert!(p.beat_triggered, "beat_triggered must be true when trigger fires");

        let EventPayload::BeatTransitioned { to_focus_id, .. } =
            &result.follow_up_events[1].payload
        else {
            panic!("expected BeatTransitioned")
        };
        assert_eq!(to_focus_id, "next");
        assert!(uow.scene.is_some());
    }

    #[test]
    fn missing_emotion_state_returns_precondition_error() {
        let policy = StimulusPolicy::new();
        let npc = NpcBuilder::new("alice", "Alice").build();
        let rel = Relationship::neutral("alice", "bob");
        let mut harness = HandlerTestHarness::new()
            .with_npc(npc)
            .with_relationship(rel);

        let event = make_stim_request("alice", "bob");
        let err = harness
            .dispatch(&policy, event)
            .expect_err("must fail without emotion state");

        assert!(matches!(
            err,
            HandlerError::EmotionStateNotFound(ref id) if id == "alice"
        ));
    }

    #[test]
    fn npc_not_found_returns_error() {
        let policy = StimulusPolicy::new();
        let mut harness = HandlerTestHarness::new();
        let event = make_stim_request("non_existent", "bob");

        let err = harness.dispatch(&policy, event).expect_err("must fail");
        assert!(matches!(err, HandlerError::NpcNotFound(ref id) if id == "non_existent"));
    }

    #[test]
    fn relationship_not_found_returns_error() {
        let policy = StimulusPolicy::new();
        let npc = NpcBuilder::new("alice", "Alice").build();
        let mut harness = HandlerTestHarness::new().with_npc(npc);
        let event = make_stim_request("alice", "bob"); 

        let err = harness.dispatch(&policy, event).expect_err("must fail");
        assert!(matches!(err, HandlerError::RelationshipNotFound { ref owner_id, .. } if owner_id == "alice"));
    }

    #[test]
    fn multi_scene_isolation_test() {
        let policy = StimulusPolicy::new();
        let alice = NpcBuilder::new("alice", "Alice").build();
        let rel_bob = Relationship::neutral("alice", "bob");
        let rel_charlie = Relationship::neutral("alice", "charlie");

        let scene_bob = Scene::new("alice".into(), "bob".into(), vec![]);

        let char_focus = SceneFocus {
            id: "char-init".into(),
            description: "Charlie 초기".into(),
            trigger: FocusTrigger::Initial,
            event: None,
            action: None,
            object: None,
            test_script: vec![],
        };
        let mut scene_charlie = Scene::new("alice".into(), "charlie".into(), vec![char_focus]);
        scene_charlie.set_active_focus("char-init".into());

        let mut harness = HandlerTestHarness::new()
            .with_npc(alice)
            .with_relationship(rel_bob)
            .with_relationship(rel_charlie)
            .with_emotion_state("alice", seed_emotion_state())
            .with_scene(scene_bob)
            .with_scene(scene_charlie);

        let event = make_stim_request("alice", "bob");
        harness.dispatch(&policy, event).expect("success");

        let repo = harness.repo_arc.lock().unwrap();
        let charlie_scene = repo
            .get_scene_by_id(&SceneId::new("alice", "charlie"))
            .expect("Charlie scene must exist");
        assert_eq!(
            charlie_scene.active_focus_id().unwrap(),
            "char-init",
            "Charlie's scene focus must not change when interacting with Bob"
        );
    }

    #[test]
    fn beat_transition_appraisal_respects_relationship_modifiers() {
        use crate::domain::personality::Score;
        use crate::domain::emotion::ActionFocus;
        let policy = StimulusPolicy::new();
        let npc = NpcBuilder::new("alice", "Alice")
            .honesty_humility(|h| {
                h.sincerity = Score::new(0.8, "").unwrap();
                h.fairness = Score::new(0.8, "").unwrap();
                h.greed_avoidance = Score::new(0.8, "").unwrap();
                h.modesty = Score::new(0.8, "").unwrap();
            })
            .build();

        let rel = Relationship::new(
            "alice",
            "bob",
            Score::new(-0.8, "").unwrap(),
            Score::neutral(),
            Score::neutral(),
        );

        let scene = Scene::new(
            "alice".into(),
            "bob".into(),
            vec![
                SceneFocus {
                    id: "init".into(),
                    description: "초기".into(),
                    trigger: FocusTrigger::Initial,
                    event: None,
                    action: None,
                    object: None,
                    test_script: vec![],
                },
                SceneFocus {
                    id: "next".into(),
                    description: "다음".into(),
                    trigger: FocusTrigger::Conditions(vec![vec![]]), // Always triggers
                    event: Some(EventFocus {
                        description: "상대방의 무례한 행동".into(),
                        desirability_for_self: -0.5,
                        desirability_for_other: None,
                        prospect: None,
                    }),
                    action: Some(ActionFocus {
                        description: "무례한 발언".into(),
                        agent_id: Some("bob".into()),
                        praiseworthiness: -0.5,
                        modifiers: None,
                    }),
                    object: None,
                    test_script: vec![],
                },
            ],
        );
        let mut scene = scene;
        scene.set_active_focus("init".into());

        let mut harness = HandlerTestHarness::new()
            .with_npc(npc)
            .with_relationship(rel)
            .with_emotion_state("alice", seed_emotion_state())
            .with_scene(scene);

        let event = make_stim_request("alice", "bob");
        let (_, uow) = harness.dispatch(&policy, event).expect("success");

        let merged = uow.emotion_state.as_ref().map(|(_, s)| s).expect("must have emotion");
        let anger = merged
            .emotions()
            .iter()
            .find(|e| e.emotion_type() == EmotionType::Anger)
            .map(|e| e.intensity())
            .unwrap_or(0.0);

        assert!(anger > 0.1, "Anger ({}) should be triggered and amplified", anger);
    }
}
