//! CommandDispatcher 통합 테스트
//!
//! Dispatcher를 통한 Agent 호출이 MindService와 동일한 결과를 생성하는지 검증합니다.
//!
//! B5.1: v1 `CommandDispatcher::dispatch` 기준 테스트 집합. v2 path 테스트는
//! `tests/dispatch_v2_test.rs` 참조. v1 제거(v0.3.0) 시 본 파일도 삭제 예정.

#![allow(deprecated)]

mod common;

use common::TestContext;
use npc_mind::application::command::dispatcher::CommandDispatcher;
use npc_mind::application::command::types::{Command, CommandResult};
use npc_mind::application::dto::*;
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::application::mind_service::MindService;
use npc_mind::application::projection::{EmotionProjection, Projection};
use npc_mind::domain::event::{DomainEvent, EventPayload};
use npc_mind::{EventStore, InMemoryRepository};

use std::sync::{Arc, RwLock};

/// 테스트용 공유 Projection 래퍼
struct Shared<P: Projection + Send + Sync>(Arc<RwLock<P>>);

impl<P: Projection + Send + Sync> Projection for Shared<P> {
    fn apply(&mut self, event: &DomainEvent) {
        self.0.write().unwrap().apply(event);
    }
}

fn make_dispatcher(
    repo: InMemoryRepository,
) -> (
    CommandDispatcher<InMemoryRepository>,
    Arc<InMemoryEventStore>,
) {
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(repo, store.clone(), bus);
    (dispatcher, store)
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
            action: Some(ActionInput {
                description: "행위".into(),
                agent_id: Some("gyo_ryong".into()),
                praiseworthiness: -0.7,
            }),
            object: None,
        }),
    }
}

fn stimulus_cmd() -> Command {
    Command::ApplyStimulus {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        pleasure: 0.5,
        arousal: 0.3,
        dominance: 0.2,
        situation_description: None,
    }
}

// ---------------------------------------------------------------------------
// 테스트
// ---------------------------------------------------------------------------

#[test]
fn dispatcher_appraise_produces_result() {
    let ctx = TestContext::new();
    let (mut dispatcher, store) = make_dispatcher(ctx.repo);

    let result = dispatcher.dispatch(appraise_cmd()).unwrap();
    let CommandResult::Appraised(appraise) = result else {
        panic!("Expected Appraised result");
    };

    // 감정이 생성되었는지
    assert!(!appraise.emotions.is_empty());
    assert!(appraise.mood != 0.0 || !appraise.emotions.is_empty());

    // 이벤트 발행 확인
    let events = store.get_all_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].payload, EventPayload::EmotionAppraised { .. }));
}

#[test]
fn dispatcher_appraise_matches_mind_service() {
    // MindService와 CommandDispatcher의 결과 비교
    let mut ctx1 = TestContext::new();
    let ctx2 = TestContext::new();

    // MindService
    let mut service = MindService::new(&mut ctx1.repo);
    let req = AppraiseRequest {
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
            action: Some(ActionInput {
                description: "행위".into(),
                agent_id: Some("gyo_ryong".into()),
                praiseworthiness: -0.7,
            }),
            object: None,
        }),
    };
    let direct = service.appraise(req, || {}, Vec::new).unwrap();

    // CommandDispatcher
    let (mut dispatcher, _) = make_dispatcher(ctx2.repo);
    let result = dispatcher.dispatch(appraise_cmd()).unwrap();
    let CommandResult::Appraised(dispatched) = result else {
        panic!("Expected Appraised");
    };

    assert_eq!(direct.mood, dispatched.mood);
    assert_eq!(direct.emotions.len(), dispatched.emotions.len());
    assert_eq!(
        direct.dominant.as_ref().map(|d| &d.emotion_type),
        dispatched.dominant.as_ref().map(|d| &d.emotion_type),
    );
}

#[test]
fn dispatcher_stimulus_no_beat_change() {
    let ctx = TestContext::new();
    let (mut dispatcher, store) = make_dispatcher(ctx.repo);

    // appraise 먼저
    dispatcher.dispatch(appraise_cmd()).unwrap();

    // stimulus
    let result = dispatcher.dispatch(stimulus_cmd()).unwrap();
    let CommandResult::StimulusApplied(stim) = result else {
        panic!("Expected StimulusApplied");
    };

    assert!(!stim.beat_changed);

    // 이벤트: EmotionAppraised + StimulusApplied
    let events = store.get_all_events();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[1].payload, EventPayload::StimulusApplied { .. }));
}

#[test]
fn dispatcher_generate_guide() {
    let ctx = TestContext::new();
    let (mut dispatcher, store) = make_dispatcher(ctx.repo);

    // appraise 먼저
    dispatcher.dispatch(appraise_cmd()).unwrap();

    // guide
    let cmd = Command::GenerateGuide {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        situation_description: None,
    };
    let result = dispatcher.dispatch(cmd).unwrap();
    let CommandResult::GuideGenerated(guide) = result else {
        panic!("Expected GuideGenerated");
    };

    assert!(!guide.guide.npc_name.is_empty());

    // 이벤트: EmotionAppraised + GuideGenerated
    let events = store.get_all_events();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[1].payload, EventPayload::GuideGenerated { .. }));
}

#[test]
fn dispatcher_end_dialogue_emits_three_events() {
    let ctx = TestContext::new();
    let (mut dispatcher, store) = make_dispatcher(ctx.repo);

    // appraise
    dispatcher.dispatch(appraise_cmd()).unwrap();

    // end_dialogue
    let cmd = Command::EndDialogue {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        significance: Some(0.5),
    };
    let result = dispatcher.dispatch(cmd).unwrap();
    let CommandResult::DialogueEnded(response) = result else {
        panic!("Expected DialogueEnded");
    };

    assert_ne!(response.before.closeness, response.after.closeness);

    // 이벤트: Appraised + RelUpdated + EmotionCleared + SceneEnded = 4
    let events = store.get_all_events();
    assert_eq!(events.len(), 4);
    assert!(matches!(events[1].payload, EventPayload::RelationshipUpdated { .. }));
    assert!(matches!(events[2].payload, EventPayload::EmotionCleared { .. }));
    assert!(matches!(events[3].payload, EventPayload::SceneEnded { .. }));
}

#[test]
fn dispatcher_start_scene() {
    let ctx = TestContext::new();
    let (mut dispatcher, store) = make_dispatcher(ctx.repo);

    let cmd = Command::StartScene {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        significance: Some(0.5),
        focuses: vec![SceneFocusInput {
            id: "focus_initial".into(),
            description: "초기 상황".into(),
            trigger: None, // Initial
            event: Some(EventInput {
                description: "사건".into(),
                desirability_for_self: -0.3,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
            test_script: vec![],
        }],
    };

    let result = dispatcher.dispatch(cmd).unwrap();
    let CommandResult::SceneStarted(scene) = result else {
        panic!("Expected SceneStarted");
    };

    assert_eq!(scene.focus_count, 1);
    assert!(scene.initial_appraise.is_some());
    assert_eq!(scene.active_focus_id, Some("focus_initial".into()));

    // SceneStarted + EmotionAppraised
    let events = store.get_all_events();
    assert!(events.len() >= 2);
    assert!(matches!(events[0].payload, EventPayload::SceneStarted { .. }));
    assert!(matches!(events[1].payload, EventPayload::EmotionAppraised { .. }));
}

#[test]
fn enriched_event_contains_emotion_snapshot() {
    let ctx = TestContext::new();
    let (mut dispatcher, store) = make_dispatcher(ctx.repo);

    dispatcher.dispatch(appraise_cmd()).unwrap();

    let events = store.get_all_events();
    if let EventPayload::EmotionAppraised { emotion_snapshot, .. } = &events[0].payload {
        assert!(!emotion_snapshot.is_empty(), "snapshot must contain emotions");
    } else {
        panic!("Expected EmotionAppraised");
    }
}

#[test]
fn projection_updates_from_dispatcher_events() {
    let ctx = TestContext::new();
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());

    let proj = Arc::new(RwLock::new(EmotionProjection::new()));
    let mut dispatcher = CommandDispatcher::new(ctx.repo, store.clone(), bus);
    dispatcher.register_projection(Shared(proj.clone()));

    dispatcher.dispatch(appraise_cmd()).unwrap();

    // L1 Projection은 dispatch 직후 즉시 최신 상태 (쿼리 일관성)
    let p = proj.read().unwrap();
    assert!(p.get_mood("mu_baek").is_some());
    assert!(p.get_snapshot("mu_baek").is_some());
    let snapshot = p.get_snapshot("mu_baek").unwrap();
    assert!(!snapshot.is_empty());
}

#[test]
fn with_projections_shares_registry_across_services() {
    use npc_mind::application::projection::ProjectionRegistry;

    // 외부에서 생성한 registry를 여러 dispatcher에 공유 주입
    let ctx = TestContext::new();
    let shared_registry = Arc::new(RwLock::new(ProjectionRegistry::new()));

    let proj = Arc::new(RwLock::new(EmotionProjection::new()));
    shared_registry
        .write()
        .unwrap()
        .add(Shared(proj.clone()));

    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let mut dispatcher = CommandDispatcher::new(ctx.repo, store, bus)
        .with_projections(shared_registry.clone());

    dispatcher.dispatch(appraise_cmd()).unwrap();

    // 공유 registry에 등록된 projection이 dispatch로 갱신됨
    let p = proj.read().unwrap();
    assert!(p.get_mood("mu_baek").is_some());

    // 접근자도 같은 registry를 가리킴
    assert!(Arc::ptr_eq(dispatcher.projections(), &shared_registry));
}

#[test]
fn projections_update_in_order_of_registration() {
    // L1 Projection은 apply_all 호출로 등록 순서대로 적용됨
    let ctx = TestContext::new();
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());

    let order = Arc::new(RwLock::new(Vec::<&'static str>::new()));

    struct OrderRecorder {
        tag: &'static str,
        log: Arc<RwLock<Vec<&'static str>>>,
    }
    impl npc_mind::application::projection::Projection for OrderRecorder {
        fn apply(&mut self, _event: &DomainEvent) {
            self.log.write().unwrap().push(self.tag);
        }
    }

    let dispatcher = CommandDispatcher::new(ctx.repo, store, bus);
    dispatcher.register_projection(OrderRecorder {
        tag: "first",
        log: order.clone(),
    });
    dispatcher.register_projection(OrderRecorder {
        tag: "second",
        log: order.clone(),
    });

    let mut dispatcher = dispatcher;
    dispatcher.dispatch(appraise_cmd()).unwrap();

    let log = order.read().unwrap();
    // appraise는 이벤트 1개 → 등록 순서로 2회 실행
    assert_eq!(*log, vec!["first", "second"]);
}

#[test]
fn full_workflow_appraise_stimulus_guide_end() {
    let ctx = TestContext::new();
    let (mut dispatcher, store) = make_dispatcher(ctx.repo);

    // 1. Appraise
    dispatcher.dispatch(appraise_cmd()).unwrap();

    // 2. Stimulus
    dispatcher.dispatch(stimulus_cmd()).unwrap();

    // 3. Guide
    let guide_cmd = Command::GenerateGuide {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        situation_description: None,
    };
    dispatcher.dispatch(guide_cmd).unwrap();

    // 4. End Dialogue
    let end_cmd = Command::EndDialogue {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        significance: Some(0.5),
    };
    dispatcher.dispatch(end_cmd).unwrap();

    // 이벤트 순서:
    // Appraised, StimulusApplied, GuideGenerated, RelUpdated, EmotionCleared, SceneEnded
    let events = store.get_all_events();
    assert_eq!(events.len(), 6);
    assert!(matches!(events[0].payload, EventPayload::EmotionAppraised { .. }));
    assert!(matches!(events[1].payload, EventPayload::StimulusApplied { .. }));
    assert!(matches!(events[2].payload, EventPayload::GuideGenerated { .. }));
    assert!(matches!(events[3].payload, EventPayload::RelationshipUpdated { .. }));
    assert!(matches!(events[4].payload, EventPayload::EmotionCleared { .. }));
    assert!(matches!(events[5].payload, EventPayload::SceneEnded { .. }));

    // 모든 이벤트의 aggregate_id가 mu_baek
    for e in &events {
        assert_eq!(e.aggregate_id, "mu_baek");
    }

    // sequence가 1부터 증가
    for (i, e) in events.iter().enumerate() {
        assert_eq!(e.sequence, (i + 1) as u64);
    }
}
