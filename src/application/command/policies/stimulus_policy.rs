//! StimulusPolicy — PAD 자극 적용 + Beat 전환 판정 전담 (B안 B1)
//!
//! 기존 `EmotionPolicy.handle_stimulus` 로직의 v2 포팅.
//! 현재 Dispatcher는 여전히 `EmotionPolicy.handle_stimulus`를 호출하며,
//! 이 Agent는 B3 `dispatch_v2()`가 생겨야 실제 호출된다. B1은 타입·테스트 준비 단계.
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
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest, HandlerResult,
};
use crate::application::command::priority;
use crate::domain::emotion::{AppraisalEngine, EmotionState, StimulusEngine};
use crate::domain::event::{DomainEvent, EventKind, EventPayload};
use crate::domain::pad::Pad;
use crate::domain::scene_id::SceneId;
use crate::domain::tuning::{BEAT_DEFAULT_SIGNIFICANCE, BEAT_MERGE_THRESHOLD};
use crate::ports::{Appraiser, StimulusProcessor};

/// PAD 자극 적용 + Beat 전환 판정 에이전트
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

    fn handle(
        &self,
        event: &DomainEvent,
        ctx: &mut EventHandlerContext<'_>,
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
        let current = ctx
            .repo
            .get_emotion_state(npc_id)
            .ok_or_else(|| HandlerError::EmotionStateNotFound(npc_id.clone()))?;

        let pad_struct = Pad {
            pleasure: pad.0,
            arousal: pad.1,
            dominance: pad.2,
        };
        let mood_before = current.overall_valence();

        let stimulated =
            self.stimulus_processor
                .apply_stimulus(npc.personality(), &current, &pad_struct);

        // B4 Session 3: Scene 조회를 (npc_id, partner_id)로 정확히 지정.
        // 이전에는 `ctx.repo.get_scene()` (단일 Scene legacy 경로)를 썼으나 `last_scene_id`
        // 가 다른 Scene을 가리킬 때 **잘못된 Scene의 trigger**를 검사하는 multi-scene 버그.
        let scene_id = SceneId::new(npc_id, partner_id);
        if let Some(scene) = ctx.repo.get_scene_by_id(&scene_id) {
            if let Some(focus) = scene.check_trigger(&stimulated).cloned() {
                let from_focus_id = scene.active_focus_id().map(|s| s.to_string());
                let situation = focus.to_situation().map_err(|e| {
                    HandlerError::InvalidInput(format!("focus to_situation failed: {e}"))
                })?;

                // Beat 전환용 임시 관계 갱신(modifiers 계산용 — 실제 저장은 RelationshipPolicy)
                let beat_rel = relationship.after_dialogue(&stimulated, BEAT_DEFAULT_SIGNIFICANCE);
                let new_state = self.appraiser.appraise(
                    npc.personality(),
                    &situation,
                    &beat_rel.modifiers(),
                );
                let merged =
                    EmotionState::merge_from_beat(&stimulated, &new_state, BEAT_MERGE_THRESHOLD);

                // Scene을 active_focus = 새 focus로 갱신해 공유 상태에 전파.
                // v1은 `save_scene: Some(new_scene)`로 Dispatcher write-back 지시했고,
                // v2는 `ctx.shared.scene`을 Dispatcher가 읽어 `apply_shared_to_repository`로
                // repo 갱신. 누락 시 다음 stimulus가 여전히 이전 active_focus를 읽어
                // Beat가 무한 재진입하는 버그(B3 리뷰 C1) 발생.
                let mut new_scene = scene.clone();
                new_scene.set_active_focus(focus.id.clone());

                // HandlerShared에 전파 (GuidePolicy/Projection이 참조)
                ctx.shared.emotion_state = Some(merged.clone());
                ctx.shared.relationship = Some(relationship.clone());
                ctx.shared.scene = Some(new_scene);

                let stimulus_event = DomainEvent::new(
                    0,
                    npc_id.clone(),
                    0,
                    EventPayload::StimulusApplied {
                        npc_id: npc_id.clone(),
                        partner_id: partner_id.clone(),
                        pad: *pad,
                        mood_before,
                        mood_after: merged.overall_valence(),
                        beat_changed: true,
                        emotion_snapshot: merged.snapshot(),
                    },
                );
                let beat_event = DomainEvent::new(
                    0,
                    npc_id.clone(),
                    0,
                    EventPayload::BeatTransitioned {
                        npc_id: npc_id.clone(),
                        partner_id: partner_id.clone(),
                        from_focus_id,
                        to_focus_id: focus.id.clone(),
                    },
                );

                return Ok(HandlerResult {
                    follow_up_events: vec![stimulus_event, beat_event],
                });
            }
        }

        // Beat 전환 없음
        ctx.shared.emotion_state = Some(stimulated.clone());
        ctx.shared.relationship = Some(relationship);

        let mood_after = stimulated.overall_valence();
        let stimulus_event = DomainEvent::new(
            0,
            npc_id.clone(),
            0,
            EventPayload::StimulusApplied {
                npc_id: npc_id.clone(),
                partner_id: partner_id.clone(),
                pad: *pad,
                mood_before,
                mood_after,
                beat_changed: false,
                emotion_snapshot: stimulated.snapshot(),
            },
        );

        Ok(HandlerResult {
            follow_up_events: vec![stimulus_event],
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

    /// Emotion state를 초기화하는 헬퍼 (최소한의 1 감정 주입)
    fn seed_emotion_state() -> EmotionState {
        // 빈 situation으로 Appraise하여 얻은 상태를 사용하는 대신
        // Default로 비어있는 상태를 사용 (필요 시 확장)
        EmotionState::default()
    }

    #[test]
    fn stimulus_without_scene_emits_single_stimulus_applied() {
        let agent = StimulusPolicy::new();
        let npc = NpcBuilder::new("alice", "Alice").build();
        let rel = Relationship::neutral("alice", "bob");
        let mut harness = HandlerTestHarness::new()
            .with_npc(npc)
            .with_relationship(rel)
            .with_emotion_state("alice", seed_emotion_state());

        let event = make_stim_request("alice", "bob");
        let result = harness.dispatch(&agent, event).expect("handler must succeed");

        assert_eq!(result.follow_up_events.len(), 1);
        assert_eq!(result.follow_up_events[0].kind(), EventKind::StimulusApplied);

        // beat_changed가 false인지 검증
        let EventPayload::StimulusApplied { beat_changed, .. } =
            &result.follow_up_events[0].payload
        else {
            panic!("expected StimulusApplied")
        };
        assert!(!beat_changed);
        assert!(harness.shared.emotion_state.is_some());
    }

    #[test]
    fn stimulus_with_triggered_scene_emits_stimulus_and_beat_transitioned() {
        let agent = StimulusPolicy::new();
        let npc = NpcBuilder::new("alice", "Alice").build();
        let rel = Relationship::neutral("alice", "bob");

        // Scene: 활성 focus "initial" + 트리거 충족 focus "next"
        // "next"의 trigger 조건은 Hate 부재(항상 참 — EmotionState::default는 Hate가 0.0)
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
        let result = harness
            .dispatch(&agent, event)
            .expect("beat transition should succeed");

        // StimulusApplied (beat=true) + BeatTransitioned
        assert_eq!(result.follow_up_events.len(), 2);

        let kinds: Vec<_> = result.follow_up_events.iter().map(|e| e.kind()).collect();
        assert_eq!(kinds, vec![EventKind::StimulusApplied, EventKind::BeatTransitioned]);

        let EventPayload::StimulusApplied { beat_changed, .. } =
            &result.follow_up_events[0].payload
        else {
            panic!("expected StimulusApplied")
        };
        assert!(beat_changed, "beat_changed must be true when trigger fires");

        let EventPayload::BeatTransitioned { to_focus_id, .. } =
            &result.follow_up_events[1].payload
        else {
            panic!("expected BeatTransitioned")
        };
        assert_eq!(to_focus_id, "next");
    }

    #[test]
    fn missing_emotion_state_returns_precondition_error() {
        let agent = StimulusPolicy::new();
        let npc = NpcBuilder::new("alice", "Alice").build();
        let rel = Relationship::neutral("alice", "bob");
        // emotion_state 미주입
        let mut harness = HandlerTestHarness::new()
            .with_npc(npc)
            .with_relationship(rel);

        let event = make_stim_request("alice", "bob");
        let err = harness
            .dispatch(&agent, event)
            .expect_err("must fail without emotion state");

        assert!(matches!(
            err,
            HandlerError::EmotionStateNotFound(ref id) if id == "alice"
        ));
    }
}
