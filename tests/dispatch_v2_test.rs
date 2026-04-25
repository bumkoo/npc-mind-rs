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

#[tokio::test]
async fn v2_appraise_emits_request_appraised_guide_sequence() {
    let ctx = TestContext::new();
    let dispatcher = make_dispatcher_v2(ctx.repo);

    let out = dispatcher.dispatch_v2(appraise_cmd()).await.expect("must succeed");

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
    assert!(out.shared.emotion_state.is_some(), "EmotionPolicy가 shared에 emotion_state 주입");
    assert!(out.shared.guide.is_some(), "GuidePolicy가 shared에 guide 주입");
}

// ---------------------------------------------------------------------------
// 기본 동작: ApplyStimulus (Beat 전환 없음)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn v2_stimulus_without_beat_emits_request_applied_guide() {
    let ctx = TestContext::new();
    let dispatcher = make_dispatcher_v2(ctx.repo);

    // Scene 없이 stimulus만 — Beat 전환 없음
    // 먼저 Appraise로 emotion_state 주입
    dispatcher
        .dispatch_v2(appraise_cmd())
        .await
        .expect("seed appraise");

    let out = dispatcher.dispatch_v2(stimulus_cmd()).await.expect("must succeed");

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

#[tokio::test]
async fn v2_stimulus_with_beat_trigger_cascades_to_relationship_update() {
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

    let dispatcher = make_dispatcher_v2(repo);

    // emotion_state seed
    dispatcher
        .dispatch_v2(appraise_cmd())
        .await
        .expect("seed appraise");

    let out = dispatcher.dispatch_v2(stimulus_cmd()).await.expect("must succeed");

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
        "RelationshipPolicy가 BeatTransitioned에 반응: {:?}",
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

#[tokio::test]
async fn v2_appraise_persists_events_to_event_store() {
    use npc_mind::EventStore;

    let ctx = TestContext::new();
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher =
        CommandDispatcher::new(ctx.repo, store.clone(), bus).with_default_handlers();

    dispatcher.dispatch_v2(appraise_cmd()).await.expect("ok");

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

#[tokio::test]
async fn v2_max_cascade_depth_is_enforced() {
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
                priority: 5, // EmotionPolicy보다 먼저 실행
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
    let dispatcher = CommandDispatcher::new(ctx.repo, store, bus)
        .with_default_handlers()
        .register_transactional(Arc::new(LoopingHandler));

    let err = dispatcher
        .dispatch_v2(appraise_cmd())
        .await
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

// B5.3: v1/v2 parallel 테스트 및 shadow_v2 플래그 테스트는 v1 제거와 함께 삭제됨.

/// C1 회귀 가드 — Beat 전환 후 **repo 측 Scene의 active_focus_id가 실제로 갱신**되는지.
/// StimulusPolicy가 `ctx.shared.scene`에 새 Scene을 넣지 않으면 Dispatcher write-back이
/// 누락되어 다음 stimulus가 여전히 이전 focus를 보고 무한 Beat 재진입이 발생한다.
#[tokio::test]
async fn v2_beat_transition_persists_new_active_focus_to_repo() {
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

    let dispatcher = make_dispatcher_v2(repo);
    dispatcher.dispatch_v2(appraise_cmd()).await.expect("seed");
    dispatcher.dispatch_v2(stimulus_cmd()).await.expect("beat stimulus");

    // repo에서 Scene을 다시 조회 — active_focus_id가 "next"로 갱신돼야 함.
    let scene = dispatcher
        .repository_guard()
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
        7,
        "Scene/Emotion/Stimulus/Guide/Relationship/Information/WorldOverlay 7종 (Step D에서 WorldOverlayPolicy 추가)"
    );
    assert_eq!(
        d.inline_handler_count(),
        3,
        "Emotion/Relationship/Scene Projection 3종 (Memory 계열은 with_memory() 별도 부착)"
    );
}

// ---------------------------------------------------------------------------
// B4.1 — 4 추가 커맨드 dispatch_v2 지원
// ---------------------------------------------------------------------------

#[tokio::test]
async fn v2_generate_guide_emits_requested_and_generated() {
    let ctx = TestContext::new();
    let dispatcher = make_dispatcher_v2(ctx.repo);

    // seed: emotion_state가 repo에 있어야 GuidePolicy fallback이 성공
    dispatcher.dispatch_v2(appraise_cmd()).await.expect("seed");

    let out = dispatcher
        .dispatch_v2(Command::GenerateGuide {
            npc_id: "mu_baek".into(),
            partner_id: "gyo_ryong".into(),
            situation_description: Some("test".into()),
        })
        .await
        .expect("must succeed");

    let kinds = event_kinds(&out.events);
    assert_eq!(kinds, vec![EventKind::GuideRequested, EventKind::GuideGenerated]);
    assert!(out.shared.guide.is_some());
}

#[tokio::test]
async fn v2_update_relationship_emits_requested_and_updated() {
    let ctx = TestContext::new();
    let dispatcher = make_dispatcher_v2(ctx.repo);

    dispatcher.dispatch_v2(appraise_cmd()).await.expect("seed");

    let out = dispatcher
        .dispatch_v2(Command::UpdateRelationship {
            npc_id: "mu_baek".into(),
            partner_id: "gyo_ryong".into(),
            significance: Some(0.7),
        })
        .await
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

#[tokio::test]
async fn v2_end_dialogue_emits_three_follow_ups_and_clears_repo_state() {
    use npc_mind::ports::{EmotionStore, NpcWorld, SceneStore};

    let ctx = TestContext::new();
    let dispatcher = make_dispatcher_v2(ctx.repo);

    // seed: emotion_state + Scene 주입
    dispatcher.dispatch_v2(appraise_cmd()).await.expect("seed");
    // sanity check: seed 후 repo에 emotion_state 존재
    assert!(
        dispatcher
            .repository_guard()
            .get_emotion_state("mu_baek")
            .is_some(),
        "seed 후 repo에 emotion_state 있어야 함"
    );
    // 관계 초기값 기록 (DialogueEnd 후 변경되는지 비교용)
    let (bc_before, bt_before) = {
        let rel_before = dispatcher
            .repository_guard()
            .get_relationship("mu_baek", "gyo_ryong")
            .expect("seed 관계 존재");
        (rel_before.closeness().value(), rel_before.trust().value())
    };

    let out = dispatcher
        .dispatch_v2(Command::EndDialogue {
            npc_id: "mu_baek".into(),
            partner_id: "gyo_ryong".into(),
            significance: Some(0.9),
        })
        .await
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
            .repository_guard()
            .get_emotion_state("mu_baek")
            .is_none(),
        "EmotionCleared → repo.clear_emotion_state 호출"
    );
    assert!(
        dispatcher.repository_guard().get_scene().is_none(),
        "SceneEnded → repo.clear_scene 호출"
    );

    // B4.1 리뷰 m9: relationship save도 확인 — DialogueEnd의 after_dialogue 결과가 repo에 반영
    let (bc_after, bt_after) = {
        let rel_after = dispatcher
            .repository_guard()
            .get_relationship("mu_baek", "gyo_ryong")
            .expect("clear 대상 아닌 관계는 유지");
        (rel_after.closeness().value(), rel_after.trust().value())
    };
    assert!(
        (bc_after - bc_before).abs() > f32::EPSILON || (bt_after - bt_before).abs() > f32::EPSILON,
        "DialogueEnd는 관계를 갱신해야 함 (before: ({bc_before},{bt_before}), after: ({bc_after},{bt_after}))"
    );
}

#[tokio::test]
async fn v2_start_scene_with_initial_focus_cascades_to_emotion_and_guide() {
    use npc_mind::application::dto::{EventInput, SceneFocusInput};

    let ctx = TestContext::new();
    let dispatcher = make_dispatcher_v2(ctx.repo);

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
        .await
        .expect("must succeed");

    // 기대 체인:
    //   SceneStartRequested → ScenePolicy → SceneStarted + EmotionAppraised
    //   EmotionAppraised → GuidePolicy → GuideGenerated
    let kinds = event_kinds(&out.events);
    assert!(kinds.contains(&EventKind::SceneStartRequested));
    assert!(kinds.contains(&EventKind::SceneStarted));
    assert!(kinds.contains(&EventKind::EmotionAppraised));
    assert!(kinds.contains(&EventKind::GuideGenerated));

    assert!(out.shared.scene.is_some());
    assert!(out.shared.emotion_state.is_some());
    assert!(out.shared.guide.is_some());
}

// B5.3: v1/v2 parallel 테스트는 v1 제거와 함께 삭제됨.

// ---------------------------------------------------------------------------
// correlation_id activation (Stage 1, docs/tasks/correlation-id-activation.md §6.1·6.2)
// ---------------------------------------------------------------------------

/// 6.1: dispatch_v2 한 호출이 만든 모든 이벤트가 같은 cid로 묶인다.
#[tokio::test]
async fn dispatch_v2_attaches_correlation_id_to_all_events() {
    let ctx = TestContext::new();
    let dispatcher = make_dispatcher_v2(ctx.repo);

    let result = dispatcher.dispatch_v2(appraise_cmd()).await.expect("must succeed");

    assert!(!result.events.is_empty(), "expected at least one event");
    let first_cid = result.events[0]
        .metadata
        .correlation_id
        .expect("first event must have correlation_id");

    for ev in &result.events {
        assert_eq!(
            ev.metadata.correlation_id,
            Some(first_cid),
            "all events of one dispatch must share the same correlation_id"
        );
    }
}

/// 6.2: 서로 다른 dispatch_v2 호출은 서로 다른 cid를 갖고, 단조 증가한다.
#[tokio::test]
async fn distinct_dispatch_calls_get_distinct_correlation_ids() {
    let ctx = TestContext::new();
    let dispatcher = make_dispatcher_v2(ctx.repo);

    let r1 = dispatcher.dispatch_v2(appraise_cmd()).await.expect("must succeed");
    let r2 = dispatcher.dispatch_v2(appraise_cmd()).await.expect("must succeed");

    let cid1 = r1.events[0].metadata.correlation_id.expect("r1 cid");
    let cid2 = r2.events[0].metadata.correlation_id.expect("r2 cid");

    assert_ne!(cid1, cid2, "different dispatch calls must have different cids");
    assert!(cid2 > cid1, "cid must be monotonically increasing: {cid1} → {cid2}");
}

/// 6.3: EventStore::get_events_by_correlation는 그 cid로 묶인 이벤트만 정확히 반환한다.
///
/// Appraise → Stimulus → Appraise 3 dispatch를 실행해 다중 이벤트 묶음을 확인하고,
/// 각 dispatch의 cid로 조회한 묶음이 서로 섞이지 않음을 검증한다.
#[tokio::test]
async fn event_store_returns_correct_correlation_bundle() {
    let ctx = TestContext::new();
    let dispatcher = make_dispatcher_v2(ctx.repo);

    // Appraise로 emotion_state seed
    let r0 = dispatcher.dispatch_v2(appraise_cmd()).await.expect("seed");
    // Stimulus는 cascade가 더 길어 묶음 검증에 적합
    let r1 = dispatcher.dispatch_v2(stimulus_cmd()).await.expect("r1");
    let r2 = dispatcher.dispatch_v2(appraise_cmd()).await.expect("r2");

    let cid0 = r0.events[0].metadata.correlation_id.unwrap();
    let cid1 = r1.events[0].metadata.correlation_id.unwrap();
    let cid2 = r2.events[0].metadata.correlation_id.unwrap();

    let bundle0 = dispatcher.event_store().get_events_by_correlation(cid0);
    let bundle1 = dispatcher.event_store().get_events_by_correlation(cid1);
    let bundle2 = dispatcher.event_store().get_events_by_correlation(cid2);

    assert_eq!(bundle0.len(), r0.events.len(), "bundle0 size mismatch");
    assert_eq!(bundle1.len(), r1.events.len(), "bundle1 size mismatch");
    assert_eq!(bundle2.len(), r2.events.len(), "bundle2 size mismatch");

    for ev in &bundle0 {
        assert_eq!(ev.metadata.correlation_id, Some(cid0));
    }
    for ev in &bundle1 {
        assert_eq!(ev.metadata.correlation_id, Some(cid1));
    }
    for ev in &bundle2 {
        assert_eq!(ev.metadata.correlation_id, Some(cid2));
    }

    // 묶음 합 = 전체 이벤트 수 (다른 묶음으로의 누수 없음)
    let total = dispatcher.event_store().get_all_events().len();
    assert_eq!(bundle0.len() + bundle1.len() + bundle2.len(), total);

    // sentinel: cid 0은 매치되는 이벤트 없음.
    let empty = dispatcher.event_store().get_events_by_correlation(0);
    assert!(empty.is_empty(), "cid 0 is reserved sentinel — no events should match");
}

/// 6.5: 동시 dispatch_v2 호출 N개가 모두 distinct cid를 받고, 각 묶음 안에서는
/// cid가 균일하다 (cross-contamination 없음).
///
/// task 명세 §12.1 — per-call 격리가 동시 호출에서도 보장된다는 핵심 개선점의
/// 회귀 가드. 현재 dispatch_v2는 repository mutex로 직렬화되지만 그 제약이 풀려도
/// cid 계약이 유지되어야 한다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_dispatch_calls_get_distinct_correlation_ids() {
    use std::collections::HashSet;
    use std::sync::Arc;

    let ctx = TestContext::new();
    let dispatcher = Arc::new(make_dispatcher_v2(ctx.repo));

    const N: usize = 16;
    let mut handles = Vec::with_capacity(N);
    for _ in 0..N {
        let d = dispatcher.clone();
        handles.push(tokio::spawn(async move { d.dispatch_v2(appraise_cmd()).await }));
    }

    let mut all_cids = Vec::with_capacity(N);
    for h in handles {
        let result = h.await.expect("task panic").expect("dispatch failed");
        assert!(!result.events.is_empty(), "every dispatch must emit events");
        let bundle_cid = result.events[0]
            .metadata
            .correlation_id
            .expect("every event must carry correlation_id");
        // 묶음 안 모든 이벤트의 cid가 동일 (cross-contamination 없음)
        for ev in &result.events {
            assert_eq!(
                ev.metadata.correlation_id,
                Some(bundle_cid),
                "concurrent dispatch: bundle cid must be uniform"
            );
        }
        all_cids.push(bundle_cid);
    }

    let unique: HashSet<_> = all_cids.iter().copied().collect();
    assert_eq!(
        unique.len(),
        N,
        "concurrent dispatches must produce N distinct cids, got {} unique out of {N}",
        unique.len()
    );
}

// parent_event_id / cascade_depth 트리 구조 검증

/// 헬퍼: appraise(seed) → stimulus dispatch. 결과 이벤트 묶음을 반환.
async fn run_seeded_stimulus(
    dispatcher: &CommandDispatcher<InMemoryRepository>,
) -> Vec<npc_mind::DomainEvent> {
    dispatcher.dispatch_v2(appraise_cmd()).await.expect("seed");
    dispatcher
        .dispatch_v2(stimulus_cmd())
        .await
        .expect("stimulus")
        .events
}

#[tokio::test]
async fn cascade_depth_increases_along_follow_up_chain() {
    let ctx = TestContext::new();
    let dispatcher = make_dispatcher_v2(ctx.repo);
    let events = run_seeded_stimulus(&dispatcher).await;

    let initial = &events[0];
    assert_eq!(initial.metadata.cascade_depth, 0);
    assert!(initial.metadata.parent_event_id.is_none());

    let max_depth = events
        .iter()
        .map(|e| e.metadata.cascade_depth)
        .max()
        .expect("at least one event");
    assert!(
        max_depth > 0,
        "stimulus cmd should produce at least one follow-up event (max_depth was {max_depth})"
    );
}

#[tokio::test]
async fn parent_event_id_forms_valid_tree() {
    use std::collections::HashSet;

    let ctx = TestContext::new();
    let dispatcher = make_dispatcher_v2(ctx.repo);
    let events = run_seeded_stimulus(&dispatcher).await;

    let event_ids: HashSet<_> = events.iter().map(|e| e.id).collect();

    for ev in &events {
        if let Some(parent_id) = ev.metadata.parent_event_id {
            assert!(
                event_ids.contains(&parent_id),
                "parent_event_id {parent_id} must point to an event within the same correlation bundle"
            );
        }
    }

    let roots: Vec<_> = events
        .iter()
        .filter(|e| e.metadata.parent_event_id.is_none())
        .collect();
    assert_eq!(roots.len(), 1, "exactly one root event expected");
    assert_eq!(roots[0].metadata.cascade_depth, 0);
}

#[tokio::test]
async fn child_depth_is_parent_plus_one() {
    use std::collections::HashMap;

    let ctx = TestContext::new();
    let dispatcher = make_dispatcher_v2(ctx.repo);
    let events = run_seeded_stimulus(&dispatcher).await;

    let by_id: HashMap<_, _> = events.iter().map(|e| (e.id, e)).collect();

    for ev in &events {
        if let Some(parent_id) = ev.metadata.parent_event_id {
            let parent = by_id.get(&parent_id).expect("parent must exist");
            assert_eq!(
                ev.metadata.cascade_depth,
                parent.metadata.cascade_depth + 1,
                "child {} depth ({}) must equal parent {} depth ({}) + 1",
                ev.id,
                ev.metadata.cascade_depth,
                parent.id,
                parent.metadata.cascade_depth
            );
        }
    }
}

#[tokio::test]
async fn event_store_returns_event_by_id() {
    let ctx = TestContext::new();
    let dispatcher = make_dispatcher_v2(ctx.repo);

    let result = dispatcher
        .dispatch_v2(appraise_cmd())
        .await
        .expect("must succeed");

    let target = &result.events[0];
    let fetched = dispatcher.event_store().get_event_by_id(target.id);
    assert!(fetched.is_some(), "stored event must be retrievable by id");
    assert_eq!(fetched.unwrap().id, target.id);

    if result.events.len() > 1 {
        let leaf = result.events.last().unwrap();
        let mut current = leaf.clone();
        while let Some(parent_id) = current.metadata.parent_event_id {
            current = dispatcher
                .event_store()
                .get_event_by_id(parent_id)
                .expect("parent must be retrievable along the chain");
        }
        assert_eq!(current.metadata.cascade_depth, 0, "chain must terminate at root");
    }

    let missing = dispatcher.event_store().get_event_by_id(99_999_999);
    assert!(missing.is_none());
}

/// 수동 인과 트리 시각화 도우미. 회귀 가드 아님:
///   `cargo test --test dispatch_v2_test print_causal_tree -- --ignored --nocapture`
#[tokio::test]
#[ignore]
async fn print_causal_tree_for_stimulus() {
    use std::collections::HashMap;

    let ctx = TestContext::new();
    let dispatcher = make_dispatcher_v2(ctx.repo);
    let events = run_seeded_stimulus(&dispatcher).await;

    let cid = events[0].metadata.correlation_id.unwrap();
    let bundle = dispatcher.event_store().get_events_by_correlation(cid);

    println!("\n--- correlation_id = {cid} ({} events) ---", bundle.len());
    let by_parent: HashMap<Option<npc_mind::domain::event::EventId>, Vec<&npc_mind::DomainEvent>> =
        bundle.iter().fold(HashMap::new(), |mut acc, e| {
            acc.entry(e.metadata.parent_event_id).or_default().push(e);
            acc
        });

    fn render(
        ev: &npc_mind::DomainEvent,
        by_parent: &HashMap<Option<npc_mind::domain::event::EventId>, Vec<&npc_mind::DomainEvent>>,
        indent: usize,
    ) {
        println!(
            "{:indent$}#{} {:?} (depth={})",
            "",
            ev.id,
            ev.kind(),
            ev.metadata.cascade_depth,
            indent = indent
        );
        if let Some(children) = by_parent.get(&Some(ev.id)) {
            for c in children {
                render(c, by_parent, indent + 2);
            }
        }
    }

    let roots: Vec<_> = bundle
        .iter()
        .filter(|e| e.metadata.parent_event_id.is_none())
        .collect();
    for r in roots {
        render(r, &by_parent, 0);
    }
}
