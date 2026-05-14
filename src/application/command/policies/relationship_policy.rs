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
use crate::domain::reflection::ReflectionResult;
use crate::domain::scene_id::SceneId;
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
                reflection,
            } => self.handle_dialogue_end(npc_id, partner_id, *significance, reflection, ctx),

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
        let _emotion = ctx.get_emotion_state(npc_id)?;
        let _ = significance;

        // Stage 1: after_dialogue 폐기. Stage 2의 `update_axes_from_emotion` 신설 자리.
        // 임시 no-op — 값은 보존하되 RelationshipUpdated 이벤트는 그대로 발행 (downstream 호환).
        // TODO(Stage 2): base_delta 표 + HEXACO 보정자 적용 후 apply_delta.
        let updated = relationship.clone();
        // Stage 1 ±1.0 contract 보존 — payload schema가 ±1.0이고 downstream
        // (memory_projector RELATIONSHIP_CHANGE_THRESHOLD=0.05, relationship_memory_handler
        // dominant_delta, frontend toFixed 등)이 ±1.0 가정. 4축 ±100 값을 ÷100으로 정규화.
        // Stage 3에서 payload 6→8 + scale 명시 시 정리. power는 0.0 (B-D4 폐기).
        let (bc, bt, bp) = (
            relationship.affinity().value() / 100.0,
            relationship.trust().value() / 100.0,
            0.0_f32,
        );
        let (ac, at, ap) = (
            updated.affinity().value() / 100.0,
            updated.trust().value() / 100.0,
            0.0_f32,
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

    /// DialogueEnd — Phase 1 Mind Architecture (relationships.md v0.7 §6) Reflection 게이트 적용.
    ///
    /// follow-up 발행 순서 (spec §4.4 결정 (라)):
    /// 1. `DialogueReflected` — `reflection.is_some()`일 때만, *항상 첫 번째* (chitchat 케이스 포함 박제)
    /// 2. `RelationshipUpdated` — 게이트 통과 시만 (chitchat skip)
    /// 3. `EmotionCleared` — 항상
    /// 4. `SceneEnded` — 항상
    ///
    /// 게이트 (spec §4.4 결정 8 / relationships.md v0.7 §6.4):
    /// - `reflection: Some(_)` 케이스: significance ≥ 0.3 OR !is_chitchat OR
    ///   declarative_events 비어있지 않음 OR partnership_event 있음 OR (Phase 3a/3b external/temporal — Phase 1엔 없음)
    /// - `reflection: None` 케이스: legacy_significance.is_some() (기존 무조건 동작 호환)
    fn handle_dialogue_end(
        &self,
        npc_id: &str,
        partner_id: &str,
        significance: Option<f32>,
        reflection: &Option<ReflectionResult>,
        ctx: &mut dyn DynamicHandlerContext,
    ) -> Result<HandlerResult, HandlerError> {
        // Scene/relationship/emotion lookup은 *모든* follow-up 발행에 필요.
        let sig = significance.unwrap_or(profile().beat_default_significance);
        let enter_outer = outer_loop_entry(reflection, significance);

        let mut follow_ups: Vec<DomainEvent> = Vec::with_capacity(4);

        // 1. DialogueReflected — reflection 있으면 항상 발행 (chitchat 박제)
        if let Some(refl) = reflection {
            follow_ups.push(DomainEvent::new(
                0,
                npc_id.to_string(),
                0,
                EventPayload::DialogueReflected {
                    npc_id: npc_id.to_string(),
                    partner_id: partner_id.to_string(),
                    scene_id: SceneId::new(npc_id.to_string(), partner_id.to_string()),
                    result: refl.clone(),
                },
            ));
        }

        // 2. RelationshipUpdated — 게이트 통과 시만 (chitchat skip 시 axes 보존).
        if enter_outer {
            let relationship = ctx.get_relationship(npc_id, partner_id)?;
            let _emotion = ctx.get_emotion_state(npc_id)?;
            let _ = sig;
            // Stage 1: after_dialogue 폐기 — Stage 2 placeholder. 임시 no-op.
            // TODO(Stage 2): base_delta + HEXACO + apply_delta.
            let updated = relationship.clone();
            // Stage 1 ±1.0 contract 보존 — affinity/trust를 ÷100으로 정규화 (위 cause 분기와 동일).
            let (bc, bt, bp) = (
                relationship.affinity().value() / 100.0,
                relationship.trust().value() / 100.0,
                0.0_f32,
            );
            let (ac, at, ap) = (
                updated.affinity().value() / 100.0,
                updated.trust().value() / 100.0,
                0.0_f32,
            );
            ctx.save_relationship(updated);
            follow_ups.push(DomainEvent::new(
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
            ));
        }

        // 3. EmotionCleared + 4. SceneEnded — Scene 종료는 항상 (감정 초기화 / Scene 정리).
        ctx.clear_emotion_for(npc_id.to_string());
        ctx.clear_scene();
        follow_ups.push(DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::EmotionCleared {
                npc_id: npc_id.to_string(),
            },
        ));
        follow_ups.push(DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::SceneEnded {
                npc_id: npc_id.to_string(),
                partner_id: partner_id.to_string(),
            },
        ));

        Ok(HandlerResult {
            follow_up_events: follow_ups,
        })
    }
}

/// Outer Loop 진입 게이트 평가.
///
/// `relationships.md` v0.7 §6.4 가드레일 (양형식 = 진입 조건):
///
/// ```text
/// significance >= 0.3
/// OR  !is_chitchat
/// OR  declarative_events 비어있지 않음
/// OR  partnership_event 있음
/// OR  external_events 비어있지 않음     (Phase 3b 입력 — 현재 미존재)
/// OR  temporal_signals 비어있지 않음    (Phase 3a 입력 — 현재 미존재)
/// ```
///
/// `reflection: None` 케이스: chat feature 비활성 또는 호환 caller — 기존 무조건
/// 동작으로 fallback (legacy_significance.is_some() 시 진입). 0.x 사용자 코드 깨짐 0.
pub(crate) fn outer_loop_entry(
    reflection: &Option<ReflectionResult>,
    legacy_significance: Option<f32>,
) -> bool {
    match reflection {
        Some(refl) => {
            refl.significance_score >= 0.3
                || !refl.is_chitchat
                || !refl.declarative_events.is_empty()
                || refl.partnership_event.is_some()
        }
        None => legacy_significance.is_some(),
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
    fn missing_relationship_returns_precondition_error_when_outer_loop_enters() {
        // Phase 1 Mind Architecture: 게이트 통과 시에만 relationship lookup 발생.
        // legacy significance Some(0.5) → reflection=None이지만 호환 분기로 진입.
        let policy = RelationshipPolicy::new();
        let mut harness =
            HandlerTestHarness::new().with_emotion_state("alice", EmotionState::default());

        let event = make_dialogue_end("alice", "bob", Some(0.5));
        let err = harness.dispatch(&policy, event).expect_err("must fail");

        assert!(matches!(
            err,
            HandlerError::RelationshipNotFound { ref owner_id, ref target_id }
                if owner_id == "alice" && target_id == "bob"
        ));
    }

    // -----------------------------------------------------------------------
    // Phase 1 Mind Architecture — Reflection 게이트 단위 테스트 (Stage 2.6)
    // -----------------------------------------------------------------------

    use crate::domain::reflection::{DeclarativeEventPlaceholder, ReflectionResult};

    fn make_dialogue_end_with_reflection(
        npc_id: &str,
        partner_id: &str,
        significance: Option<f32>,
        reflection: Option<ReflectionResult>,
    ) -> DomainEvent {
        DomainEvent::new(
            0,
            npc_id.to_string(),
            0,
            EventPayload::DialogueEndRequested {
                npc_id: npc_id.to_string(),
                partner_id: partner_id.to_string(),
                significance,
                reflection,
            },
        )
    }

    fn chitchat_reflection() -> ReflectionResult {
        ReflectionResult {
            is_chitchat: true,
            summary: "지나가는 인사".into(),
            significance_score: 0.05,
            declarative_events: vec![],
            partnership_event: None,
            turn_count: 2,
            llm_reasoning: Some("의례적".into()),
        }
    }

    fn significant_reflection() -> ReflectionResult {
        ReflectionResult {
            is_chitchat: false,
            summary: "결단 사건".into(),
            significance_score: 0.85,
            declarative_events: vec![],
            partnership_event: None,
            turn_count: 12,
            llm_reasoning: Some("OCC peak 0.95".into()),
        }
    }

    #[test]
    fn chitchat_skips_outer_loop_emits_only_reflected_clear_scene() {
        let policy = RelationshipPolicy::new();
        // 의도적으로 relationship 미부착 — 게이트 skip이라 lookup 발생 안 함을 검증.
        let mut harness = HandlerTestHarness::new()
            .with_emotion_state("alice", EmotionState::default());

        let event = make_dialogue_end_with_reflection(
            "alice",
            "bob",
            None,
            Some(chitchat_reflection()),
        );
        let (result, uow) = harness.dispatch(&policy, event).expect("must succeed");

        // 3 follow-ups: DialogueReflected + EmotionCleared + SceneEnded
        // (RelationshipUpdated 미발행 → axes 보존)
        assert_eq!(result.follow_up_events.len(), 3);
        let kinds: Vec<EventKind> =
            result.follow_up_events.iter().map(|e| e.kind()).collect();
        assert_eq!(
            kinds,
            vec![
                EventKind::DialogueReflected,
                EventKind::EmotionCleared,
                EventKind::SceneEnded,
            ]
        );

        // Clear 시그널은 항상
        assert_eq!(uow.clear_emotion_for.as_deref(), Some("alice"));
        assert!(uow.clear_scene);
    }

    #[test]
    fn significant_reflection_enters_outer_loop_emits_four_follow_ups() {
        let policy = RelationshipPolicy::new();
        let rel = Relationship::neutral("alice", "bob");
        let mut harness = HandlerTestHarness::new()
            .with_relationship(rel)
            .with_emotion_state("alice", EmotionState::default());

        let event = make_dialogue_end_with_reflection(
            "alice",
            "bob",
            None,
            Some(significant_reflection()),
        );
        let (result, _) = harness.dispatch(&policy, event).expect("must succeed");

        // 4 follow-ups: DialogueReflected + RelationshipUpdated + EmotionCleared + SceneEnded
        assert_eq!(result.follow_up_events.len(), 4);
        let kinds: Vec<EventKind> =
            result.follow_up_events.iter().map(|e| e.kind()).collect();
        assert_eq!(
            kinds,
            vec![
                EventKind::DialogueReflected,
                EventKind::RelationshipUpdated,
                EventKind::EmotionCleared,
                EventKind::SceneEnded,
            ]
        );
    }

    #[test]
    fn legacy_caller_no_reflection_uses_legacy_significance_branch() {
        // chat feature 비활성 또는 호환 caller — reflection=None + significance=Some
        // 시 기존 무조건 동작 (RelationshipUpdated 발행, DialogueReflected 미발행).
        let policy = RelationshipPolicy::new();
        let rel = Relationship::neutral("alice", "bob");
        let mut harness = HandlerTestHarness::new()
            .with_relationship(rel)
            .with_emotion_state("alice", EmotionState::default());

        let event = make_dialogue_end_with_reflection("alice", "bob", Some(0.7), None);
        let (result, _) = harness.dispatch(&policy, event).expect("must succeed");

        assert_eq!(result.follow_up_events.len(), 3);
        let kinds: Vec<EventKind> =
            result.follow_up_events.iter().map(|e| e.kind()).collect();
        assert_eq!(
            kinds,
            vec![
                EventKind::RelationshipUpdated,
                EventKind::EmotionCleared,
                EventKind::SceneEnded,
            ]
        );
    }

    #[test]
    fn declarative_event_forces_outer_loop_even_when_chitchat_and_low_significance() {
        // is_chitchat=true + significance=0.05 + declarative_events 비어있지 않음
        // → 게이트 통과 (Channel 1 Declarative 활성화 사전 동작)
        let mut refl = chitchat_reflection();
        refl.declarative_events.push(DeclarativeEventPlaceholder {
            kind: "execute".into(),
            target: Some("lu_qian".into()),
            text: "처단".into(),
        });

        let policy = RelationshipPolicy::new();
        let rel = Relationship::neutral("alice", "bob");
        let mut harness = HandlerTestHarness::new()
            .with_relationship(rel)
            .with_emotion_state("alice", EmotionState::default());

        let event = make_dialogue_end_with_reflection("alice", "bob", None, Some(refl));
        let (result, _) = harness.dispatch(&policy, event).expect("must succeed");

        // 4 follow-ups
        assert_eq!(result.follow_up_events.len(), 4);
        assert!(result.follow_up_events.iter().any(|e| e.kind() == EventKind::RelationshipUpdated));
    }

    #[test]
    fn outer_loop_entry_unit_truth_table() {
        use super::outer_loop_entry;

        // Some(refl) 케이스
        let mut refl = chitchat_reflection();
        // (1) significance >= 0.3 OR !is_chitchat 모두 false → skip
        assert!(!outer_loop_entry(&Some(refl.clone()), None));

        // (2) significance >= 0.3 → enter
        refl.significance_score = 0.5;
        assert!(outer_loop_entry(&Some(refl.clone()), None));

        // (3) is_chitchat=false → enter
        refl.significance_score = 0.05;
        refl.is_chitchat = false;
        assert!(outer_loop_entry(&Some(refl.clone()), None));

        // (4) declarative_events 있음 → enter
        refl.is_chitchat = true;
        refl.declarative_events.push(DeclarativeEventPlaceholder {
            kind: "k".into(),
            target: None,
            text: "t".into(),
        });
        assert!(outer_loop_entry(&Some(refl.clone()), None));

        // None 케이스 — legacy 호환
        assert!(!outer_loop_entry(&None, None));
        assert!(outer_loop_entry(&None, Some(0.0)));
        assert!(outer_loop_entry(&None, Some(0.9)));
    }
}
