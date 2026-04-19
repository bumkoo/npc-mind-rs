// B5.1: v1/v2 parallel run 테스트가 v1 `dispatch`를 비교 기준으로 호출함 → allow 필요.
#![allow(deprecated)]

//! CommandDispatcher `dispatch_v2()` 통합 테스트 (B안 Stage B3)
//!
//! v2 경로가 B1 EventHandler 체인을 올바르게 구동하는지 검증:
//! - Appraise/ApplyStimulus 커맨드 처리
//! - Beat 전환 follow-up cascade (StimulusApplied → BeatTransitioned → RelationshipUpdated)
//! - Inline projection handler 실행 (EmotionProjection/RelationshipProjection/SceneProjection)
//! - 안전 한계 (cascade depth, event budget)
//! - v1/v2 의미적 동등성 (parallel run)

mod common;

use common::TestContext;
use npc_mind::application::command::dispatcher::{
    CommandDispatcher, DispatchV2Error, MAX_CASCADE_DEPTH,
};
use npc_mind::application::command::types::Command;
use npc_mind::application::dto::SituationInput;
use npc_mind::application::dto::EventInput;
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::event::{EventKind, EventPayload};
use npc_mind::InMemoryRepository;

use std::sync::Arc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_dispatcher_v2(repo: InMemoryRepository) -> CommandDispatcher<InMemoryRepository> {
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    CommandDispatcher::new(repo, store, bus).with_default_handlers()
}

fn appraise_cmd() -> Command {
    Command::Appraise {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        situation: Some(SituationInput {
            description: "배신 상황".into(),
            event: Some(EventInput {
                description: "사건".into(),
                desirability_for_self: -0.6,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
        }),
    }
}

fn stimulus_cmd() -> Command {
    Command::ApplyStimulus {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        pleasure: 0.3,
        arousal: 0.1,
        dominance: 0.0,
        situation_description: Some("test".into()),
    }
}

fn event_kinds(events: &[npc_mind::DomainEvent]) -> Vec<EventKind> {
    events.iter().map(|e| e.kind()).collect()
}

// ---------------------------------------------------------------------------
// 기본 동작: Appraise
// ---------------------------------------------------------------------------

#[test]
fn v2_appraise_emits_request_appraised_guide_sequence() {
    let ctx = TestContext::new();
    let mut dispatcher = make_dispatcher_v2(ctx.repo);

    let out = dispatcher.dispatch_v2(appraise_cmd()).expect("must succeed");

    // 기대 순서: AppraiseRequested → EmotionAppraised → GuideGenerated
    assert_eq!(
        event_kinds(&out.events),
        vec![
            EventKind::AppraiseRequested,
            EventKind::EmotionAppraised,
            EventKind::GuideGenerated,
        ],
        "v2 cascade는 초기 Requested + transactional handler chain을 모두 기록"
    );

    // HandlerShared에 전파된 상태 검증
    assert!(out.shared.emotion_state.is_some(), "EmotionAgent가 shared에 emotion_state 주입");
    assert!(out.shared.guide.is_some(), "GuideAgent가 shared에 guide 주입");
}

// ---------------------------------------------------------------------------
// 기본 동작: ApplyStimulus (Beat 전환 없음)
// ---------------------------------------------------------------------------

#[test]
fn v2_stimulus_without_beat_emits_request_applied_guide() {
    let ctx = TestContext::new();
    let mut dispatcher = make_dispatcher_v2(ctx.repo);

    // Scene 없이 stimulus만 — Beat 전환 없음
    // 먼저 Appraise로 emotion_state 주입
    dispatcher
        .dispatch_v2(appraise_cmd())
        .expect("seed appraise");

    let out = dispatcher.dispatch_v2(stimulus_cmd()).expect("must succeed");

    // StimulusApplyRequested → StimulusApplied → GuideGenerated
    let kinds = event_kinds(&out.events);
    assert_eq!(kinds[0], EventKind::StimulusApplyRequested);
    assert_eq!(kinds[1], EventKind::StimulusApplied);
    assert_eq!(kinds[2], EventKind::GuideGenerated);

    // StimulusApplied의 beat_changed=false 검증
    let EventPayload::StimulusApplied { beat_changed, .. } = &out.events[1].payload else {
        panic!("expected StimulusApplied")
    };
    assert!(!beat_changed, "Scene 없으면 beat_changed=false");
}

// ---------------------------------------------------------------------------
// Beat 전환 cascade
// ---------------------------------------------------------------------------

#[test]
fn v2_stimulus_with_beat_trigger_cascades_to_relationship_update() {
    use npc_mind::domain::emotion::{
        ConditionThreshold, EmotionCondition, EmotionType, EventFocus, FocusTrigger, Scene,
        SceneFocus,
    };
    use npc_mind::ports::SceneStore;

    let ctx = TestContext::new();
    let mut repo = ctx.repo;

    // Beat 트리거 가능 Scene 주입: 활성 focus "initial" + 조건 충족하는 "next"
    let scene = {
        let focuses = vec![
            SceneFocus {
                id: "initial".into(),
                description: "초기".into(),
                trigger: FocusTrigger::Initial,
                event: Some(EventFocus {
                    description: "".into(),
                    desirability_for_self: 0.3,
                    desirability_for_other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
                test_script: vec![],
            },
            SceneFocus {
                id: "next".into(),
                description: "다음".into(),
                trigger: FocusTrigger::Conditions(vec![vec![EmotionCondition {
                    emotion: EmotionType::Hate,
                    threshold: ConditionThreshold::Absent,
                }]]),
                event: Some(EventFocus {
                    description: "".into(),
                    desirability_for_self: 0.2,
                    desirability_for_other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
                test_script: vec![],
            },
        ];
        let mut s = Scene::new("mu_baek".into(), "gyo_ryong".into(), focuses);
        s.set_active_focus("initial".into());
        s
    };
    repo.save_scene(scene);

    let mut dispatcher = make_dispatcher_v2(repo);

    // emotion_state seed
    dispatcher
        .dispatch_v2(appraise_cmd())
        .expect("seed appraise");

    let out = dispatcher.dispatch_v2(stimulus_cmd()).expect("must succeed");

    let kinds = event_kinds(&out.events);
    // 기대: StimulusApplyRequested, StimulusApplied(beat=true), GuideGenerated,
    //       BeatTransitioned, RelationshipUpdated
    assert_eq!(kinds[0], EventKind::StimulusApplyRequested);
    assert_eq!(kinds[1], EventKind::StimulusApplied);
    assert!(
        kinds.contains(&EventKind::BeatTransitioned),
        "Beat 전환 follow-up 발행: {:?}",
        kinds
    );
    assert!(
        kinds.contains(&EventKind::RelationshipUpdated),
        "RelationshipAgent가 BeatTransitioned에 반응: {:?}",
        kinds
    );

    // StimulusApplied.beat_changed=true 검증
    let EventPayload::StimulusApplied { beat_changed, .. } = &out.events[1].payload else {
        panic!("expected StimulusApplied at index 1")
    };
    assert!(*beat_changed, "Beat trigger 충족 시 beat_changed=true");
}

// ---------------------------------------------------------------------------
// Inline Projection 갱신 검증 (event_store → inline handler)
// ---------------------------------------------------------------------------

#[test]
fn v2_appraise_persists_events_to_event_store() {
    use npc_mind::EventStore;

    let ctx = TestContext::new();
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let mut dispatcher =
        CommandDispatcher::new(ctx.repo, store.clone(), bus).with_default_handlers();

    dispatcher.dispatch_v2(appraise_cmd()).expect("ok");

    // event_store에 3 이벤트가 append됨
    let all = store.get_all_events();
    assert_eq!(all.len(), 3);
    assert!(all.iter().all(|e| e.id > 0), "commit 단계가 실 ID를 할당");
    assert!(
        all.iter().all(|e| e.sequence > 0),
        "commit 단계가 실 sequence를 할당"
    );
}

// ---------------------------------------------------------------------------
// 미지원 커맨드
// ---------------------------------------------------------------------------

// B4.1: v2가 6 커맨드 전부 지원 — UnsupportedCommand variant는 남아있으나 현재는 unreachable.
//        테스트는 "4 신규 커맨드 각각이 올바른 *Requested 이벤트로 변환·처리"로 대체.

// ---------------------------------------------------------------------------
// 안전 한계 — cascade depth는 ~4 정도인데 현재 체인은 2~3 수준이므로
// 직접 강제할 mock handler를 등록해 검증
// ---------------------------------------------------------------------------

#[test]
fn v2_max_cascade_depth_is_enforced() {
    use npc_mind::application::command::handler_v2::{
        DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest,
        HandlerResult,
    };
    use npc_mind::domain::event::DomainEvent;

    /// 자기 자신의 follow-up을 무한 재발행하는 mock handler → cascade 깊이 초과 유도
    struct LoopingHandler;

    impl EventHandler for LoopingHandler {
        fn name(&self) -> &'static str {
            "LoopingHandler"
        }
        fn interest(&self) -> HandlerInterest {
            HandlerInterest::Kinds(vec![EventKind::AppraiseRequested])
        }
        fn mode(&self) -> DeliveryMode {
            DeliveryMode::Transactional {
                priority: 5, // EmotionAgent보다 먼저 실행
                can_emit_follow_up: true,
            }
        }
        fn handle(
            &self,
            event: &DomainEvent,
            _ctx: &mut EventHandlerContext<'_>,
        ) -> Result<HandlerResult, HandlerError> {
            // 같은 종류의 이벤트를 follow-up으로 재발행 → 무한 cascade
            Ok(HandlerResult {
                follow_up_events: vec![event.clone()],
            })
        }
    }

    let ctx = TestContext::new();
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let mut dispatcher = CommandDispatcher::new(ctx.repo, store, bus)
        .with_default_handlers()
        .register_transactional(Arc::new(LoopingHandler));

    let err = dispatcher
        .dispatch_v2(appraise_cmd())
        .expect_err("must hit cascade depth limit");

    match err {
        DispatchV2Error::CascadeTooDeep { depth } => {
            assert!(depth > MAX_CASCADE_DEPTH);
        }
        DispatchV2Error::EventBudgetExceeded => {
            // budget이 depth보다 먼저 걸려도 OK (둘 다 safety bound)
        }
        other => panic!("expected cascade/budget error, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// v1/v2 의미 동등성 parallel run
//
// v1 events는 v2의 "실제 비즈니스 이벤트"와 같아야 한다 (*Requested 제외, 자동 GuideGenerated
// 제외). v2 구동 이벤트 중 `EmotionAppraised`·`StimulusApplied`·`RelationshipUpdated`·
// `BeatTransitioned`·`SceneEnded` 등 "결과" 이벤트가 v1과 일치하는지 확인.
// ---------------------------------------------------------------------------

#[test]
fn v1_v2_parallel_appraise_produces_equivalent_result_events() {
    use npc_mind::EventStore;

    // v1 실행
    let ctx1 = TestContext::new();
    let store1 = Arc::new(InMemoryEventStore::new());
    let bus1 = Arc::new(EventBus::new());
    let mut disp1 = CommandDispatcher::new(ctx1.repo, store1.clone(), bus1);
    disp1.dispatch(appraise_cmd()).expect("v1 ok");
    let v1_events = store1.get_all_events();

    // v2 실행
    let ctx2 = TestContext::new();
    let store2 = Arc::new(InMemoryEventStore::new());
    let bus2 = Arc::new(EventBus::new());
    let mut disp2 =
        CommandDispatcher::new(ctx2.repo, store2.clone(), bus2).with_default_handlers();
    disp2.dispatch_v2(appraise_cmd()).expect("v2 ok");
    let v2_events = store2.get_all_events();

    // v1 결과 이벤트: EmotionAppraised 만
    assert_eq!(event_kinds(&v1_events), vec![EventKind::EmotionAppraised]);

    // v2 결과 이벤트에서 *Requested / GuideGenerated 제거 후 v1과 같은 순서인지
    let v2_filtered: Vec<_> = v2_events
        .iter()
        .filter(|e| {
            !matches!(
                e.kind(),
                EventKind::AppraiseRequested
                    | EventKind::StimulusApplyRequested
                    | EventKind::GuideGenerated
            )
        })
        .map(|e| e.kind())
        .collect();
    assert_eq!(
        v2_filtered,
        vec![EventKind::EmotionAppraised],
        "v2는 *Requested/자동 GuideGenerated 제외 시 v1과 동일한 결과 이벤트 시퀀스"
    );

    // 이벤트의 aggregate_key도 같은 NPC를 가리켜야 함
    let v1_keys: Vec<_> = v1_events.iter().map(|e| e.aggregate_key()).collect();
    let v2_filtered_keys: Vec<_> = v2_events
        .iter()
        .filter(|e| !matches!(e.kind(), EventKind::AppraiseRequested | EventKind::GuideGenerated))
        .map(|e| e.aggregate_key())
        .collect();
    assert_eq!(v1_keys, v2_filtered_keys);
}

#[test]
fn v2_shadow_flag_defaults_false_and_can_be_set() {
    let ctx = TestContext::new();
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let d = CommandDispatcher::new(ctx.repo, store, bus);
    assert!(!d.shadow_v2(), "기본값 false");

    let ctx2 = TestContext::new();
    let store2 = Arc::new(InMemoryEventStore::new());
    let bus2 = Arc::new(EventBus::new());
    let d2 = CommandDispatcher::new(ctx2.repo, store2, bus2).with_shadow_v2(true);
    assert!(d2.shadow_v2(), "with_shadow_v2(true) 후 true");
}

/// C1 회귀 가드 — Beat 전환 후 **repo 측 Scene의 active_focus_id가 실제로 갱신**되는지.
/// StimulusAgent가 `ctx.shared.scene`에 새 Scene을 넣지 않으면 Dispatcher write-back이
/// 누락되어 다음 stimulus가 여전히 이전 focus를 보고 무한 Beat 재진입이 발생한다.
#[test]
fn v2_beat_transition_persists_new_active_focus_to_repo() {
    use npc_mind::domain::emotion::{
        ConditionThreshold, EmotionCondition, EmotionType, EventFocus, FocusTrigger, Scene,
        SceneFocus,
    };
    use npc_mind::ports::SceneStore;

    let ctx = TestContext::new();
    let mut repo = ctx.repo;

    let scene = {
        let focuses = vec![
            SceneFocus {
                id: "initial".into(),
                description: "초기".into(),
                trigger: FocusTrigger::Initial,
                event: Some(EventFocus {
                    description: "".into(),
                    desirability_for_self: 0.3,
                    desirability_for_other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
                test_script: vec![],
            },
            SceneFocus {
                id: "next".into(),
                description: "다음".into(),
                trigger: FocusTrigger::Conditions(vec![vec![EmotionCondition {
                    emotion: EmotionType::Hate,
                    threshold: ConditionThreshold::Absent,
                }]]),
                event: Some(EventFocus {
                    description: "".into(),
                    desirability_for_self: 0.2,
                    desirability_for_other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
                test_script: vec![],
            },
        ];
        let mut s = Scene::new("mu_baek".into(), "gyo_ryong".into(), focuses);
        s.set_active_focus("initial".into());
        s
    };
    repo.save_scene(scene);

    let mut dispatcher = make_dispatcher_v2(repo);
    dispatcher.dispatch_v2(appraise_cmd()).expect("seed");
    dispatcher.dispatch_v2(stimulus_cmd()).expect("beat stimulus");

    // repo에서 Scene을 다시 조회 — active_focus_id가 "next"로 갱신돼야 함.
    let scene = dispatcher
        .repository()
        .get_scene()
        .expect("scene still active");
    assert_eq!(
        scene.active_focus_id(),
        Some("next"),
        "Beat 전환 후 repo Scene의 active_focus_id가 갱신되지 않으면 다음 stimulus에서 Beat 무한 재진입"
    );
}

#[test]
fn with_default_handlers_registers_expected_counts() {
    let ctx = TestContext::new();
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let d = CommandDispatcher::new(ctx.repo, store, bus).with_default_handlers();
    assert_eq!(
        d.transactional_handler_count(),
        5,
        "Scene/Emotion/Stimulus/Guide/Relationship 5종 (B4.1에 SceneAgent 추가)"
    );
    assert_eq!(
        d.inline_handler_count(),
        3,
        "Emotion/Relationship/Scene Projection 3종"
    );
}

// ---------------------------------------------------------------------------
// B4.1 — 4 추가 커맨드 dispatch_v2 지원
// ---------------------------------------------------------------------------

#[test]
fn v2_generate_guide_emits_requested_and_generated() {
    let ctx = TestContext::new();
    let mut dispatcher = make_dispatcher_v2(ctx.repo);

    // seed: emotion_state가 repo에 있어야 GuideAgent fallback이 성공
    dispatcher.dispatch_v2(appraise_cmd()).expect("seed");

    let out = dispatcher
        .dispatch_v2(Command::GenerateGuide {
            npc_id: "mu_baek".into(),
            partner_id: "gyo_ryong".into(),
            situation_description: Some("test".into()),
        })
        .expect("must succeed");

    let kinds = event_kinds(&out.events);
    assert_eq!(kinds, vec![EventKind::GuideRequested, EventKind::GuideGenerated]);
    assert!(out.shared.guide.is_some());
}

#[test]
fn v2_update_relationship_emits_requested_and_updated() {
    let ctx = TestContext::new();
    let mut dispatcher = make_dispatcher_v2(ctx.repo);

    dispatcher.dispatch_v2(appraise_cmd()).expect("seed");

    let out = dispatcher
        .dispatch_v2(Command::UpdateRelationship {
            npc_id: "mu_baek".into(),
            partner_id: "gyo_ryong".into(),
            significance: Some(0.7),
        })
        .expect("must succeed");

    let kinds = event_kinds(&out.events);
    assert_eq!(
        kinds,
        vec![
            EventKind::RelationshipUpdateRequested,
            EventKind::RelationshipUpdated,
        ]
    );
    assert!(out.shared.relationship.is_some());
}

#[test]
fn v2_end_dialogue_emits_three_follow_ups_and_clears_repo_state() {
    use npc_mind::ports::{EmotionStore, NpcWorld, SceneStore};

    let ctx = TestContext::new();
    let mut dispatcher = make_dispatcher_v2(ctx.repo);

    // seed: emotion_state + Scene 주입
    dispatcher.dispatch_v2(appraise_cmd()).expect("seed");
    // sanity check: seed 후 repo에 emotion_state 존재
    assert!(
        dispatcher
            .repository()
            .get_emotion_state("mu_baek")
            .is_some(),
        "seed 후 repo에 emotion_state 있어야 함"
    );
    // 관계 초기값 기록 (DialogueEnd 후 변경되는지 비교용)
    let rel_before = dispatcher
        .repository()
        .get_relationship("mu_baek", "gyo_ryong")
        .expect("seed 관계 존재");
    let (bc_before, bt_before) = (rel_before.closeness().value(), rel_before.trust().value());

    let out = dispatcher
        .dispatch_v2(Command::EndDialogue {
            npc_id: "mu_baek".into(),
            partner_id: "gyo_ryong".into(),
            significance: Some(0.9),
        })
        .expect("must succeed");

    // DialogueEndRequested + RelationshipUpdated + EmotionCleared + SceneEnded
    let kinds = event_kinds(&out.events);
    assert_eq!(
        kinds,
        vec![
            EventKind::DialogueEndRequested,
            EventKind::RelationshipUpdated,
            EventKind::EmotionCleared,
            EventKind::SceneEnded,
        ]
    );

    // Clear 시그널이 commit 후 적용됐는지
    assert!(
        dispatcher
            .repository()
            .get_emotion_state("mu_baek")
            .is_none(),
        "EmotionCleared → repo.clear_emotion_state 호출"
    );
    assert!(
        dispatcher.repository().get_scene().is_none(),
        "SceneEnded → repo.clear_scene 호출"
    );

    // B4.1 리뷰 m9: relationship save도 확인 — DialogueEnd의 after_dialogue 결과가 repo에 반영
    let rel_after = dispatcher
        .repository()
        .get_relationship("mu_baek", "gyo_ryong")
        .expect("clear 대상 아닌 관계는 유지");
    let (bc_after, bt_after) = (rel_after.closeness().value(), rel_after.trust().value());
    assert!(
        (bc_after - bc_before).abs() > f32::EPSILON || (bt_after - bt_before).abs() > f32::EPSILON,
        "DialogueEnd는 관계를 갱신해야 함 (before: ({bc_before},{bt_before}), after: ({bc_after},{bt_after}))"
    );
}

#[test]
fn v2_start_scene_with_initial_focus_cascades_to_emotion_and_guide() {
    use npc_mind::application::dto::{EventInput, SceneFocusInput};

    let ctx = TestContext::new();
    let mut dispatcher = make_dispatcher_v2(ctx.repo);

    let out = dispatcher
        .dispatch_v2(Command::StartScene {
            npc_id: "mu_baek".into(),
            partner_id: "gyo_ryong".into(),
            significance: Some(0.5),
            focuses: vec![SceneFocusInput {
                id: "initial".into(),
                description: "초기".into(),
                trigger: None, // Initial focus
                event: Some(EventInput {
                    description: "시작".into(),
                    desirability_for_self: 0.3,
                    other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
                test_script: vec![],
            }],
        })
        .expect("must succeed");

    // 기대 체인:
    //   SceneStartRequested → SceneAgent → SceneStarted + EmotionAppraised
    //   EmotionAppraised → GuideAgent → GuideGenerated
    let kinds = event_kinds(&out.events);
    assert!(kinds.contains(&EventKind::SceneStartRequested));
    assert!(kinds.contains(&EventKind::SceneStarted));
    assert!(kinds.contains(&EventKind::EmotionAppraised));
    assert!(kinds.contains(&EventKind::GuideGenerated));

    assert!(out.shared.scene.is_some());
    assert!(out.shared.emotion_state.is_some());
    assert!(out.shared.guide.is_some());
}

// ---------------------------------------------------------------------------
// B4.1 — 확장된 parallel run: 4 신규 커맨드 각각 v1 결과 이벤트와 동등
// ---------------------------------------------------------------------------

/// v1과 v2의 "결과 이벤트" 시퀀스 동등성 확인 (초기 *Requested + 자동 GuideGenerated 제외)
fn compare_v1_v2_result_events(v1: Vec<EventKind>, v2: Vec<EventKind>) {
    let filtered: Vec<_> = v2
        .into_iter()
        .filter(|k| {
            !matches!(
                k,
                EventKind::AppraiseRequested
                    | EventKind::StimulusApplyRequested
                    | EventKind::GuideRequested
                    | EventKind::RelationshipUpdateRequested
                    | EventKind::DialogueEndRequested
                    | EventKind::SceneStartRequested
                    // 자동 가이드 생성은 v1에 없으므로 제외
                    | EventKind::GuideGenerated
            )
        })
        .collect();
    assert_eq!(filtered, v1);
}

#[test]
fn v1_v2_parallel_update_relationship_matches_result_events() {
    use npc_mind::EventStore;

    // v1
    let ctx1 = TestContext::new();
    let store1 = Arc::new(InMemoryEventStore::new());
    let bus1 = Arc::new(EventBus::new());
    let mut d1 = CommandDispatcher::new(ctx1.repo, store1.clone(), bus1);
    d1.dispatch(appraise_cmd()).unwrap();
    d1.dispatch(Command::UpdateRelationship {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        significance: Some(0.5),
    })
    .unwrap();
    let v1_kinds: Vec<_> = store1
        .get_all_events()
        .iter()
        .filter(|e| e.kind() == EventKind::RelationshipUpdated)
        .map(|e| e.kind())
        .collect();

    // v2
    let ctx2 = TestContext::new();
    let store2 = Arc::new(InMemoryEventStore::new());
    let bus2 = Arc::new(EventBus::new());
    let mut d2 = CommandDispatcher::new(ctx2.repo, store2.clone(), bus2).with_default_handlers();
    d2.dispatch_v2(appraise_cmd()).unwrap();
    d2.dispatch_v2(Command::UpdateRelationship {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        significance: Some(0.5),
    })
    .unwrap();
    let v2_kinds: Vec<_> = store2
        .get_all_events()
        .iter()
        .filter(|e| e.kind() == EventKind::RelationshipUpdated)
        .map(|e| e.kind())
        .collect();

    // v1·v2 공통으로 RelationshipUpdated 1개 (appraise는 관계 갱신 없음)
    compare_v1_v2_result_events(v1_kinds, v2_kinds);
}

#[test]
fn v1_v2_parallel_end_dialogue_matches_cleanup_events() {
    use npc_mind::EventStore;

    let ctx1 = TestContext::new();
    let store1 = Arc::new(InMemoryEventStore::new());
    let bus1 = Arc::new(EventBus::new());
    let mut d1 = CommandDispatcher::new(ctx1.repo, store1.clone(), bus1);
    d1.dispatch(appraise_cmd()).unwrap();
    d1.dispatch(Command::EndDialogue {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        significance: Some(0.8),
    })
    .unwrap();
    // v1 결과 이벤트: EmotionAppraised(from appraise) + RelationshipUpdated + EmotionCleared + SceneEnded
    let v1_kinds: Vec<_> = store1
        .get_all_events()
        .iter()
        .map(|e| e.kind())
        .collect();

    let ctx2 = TestContext::new();
    let store2 = Arc::new(InMemoryEventStore::new());
    let bus2 = Arc::new(EventBus::new());
    let mut d2 = CommandDispatcher::new(ctx2.repo, store2.clone(), bus2).with_default_handlers();
    d2.dispatch_v2(appraise_cmd()).unwrap();
    d2.dispatch_v2(Command::EndDialogue {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        significance: Some(0.8),
    })
    .unwrap();
    let v2_kinds: Vec<_> = store2
        .get_all_events()
        .iter()
        .map(|e| e.kind())
        .collect();

    compare_v1_v2_result_events(v1_kinds, v2_kinds);
}
