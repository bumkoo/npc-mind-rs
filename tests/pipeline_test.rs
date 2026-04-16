//! Pipeline + TieredEventBus 테스트

mod common;

use common::TestContext;
use npc_mind::application::command::agents::{EmotionAgent, GuideAgent};
use npc_mind::application::command::dispatcher::CommandDispatcher;
use npc_mind::application::command::types::{Command, CommandResult};
use npc_mind::application::dto::*;
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::application::mind_service::MindServiceError;
use npc_mind::application::pipeline::{Pipeline, PipelineState};
use npc_mind::application::tiered_event_bus::{StdThreadSink, TieredEventBus};
use npc_mind::domain::event::EventPayload;
use npc_mind::{EventStore, InMemoryRepository};

use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

fn make_dispatcher(repo: InMemoryRepository) -> (CommandDispatcher<InMemoryRepository>, Arc<InMemoryEventStore>) {
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

// ---------------------------------------------------------------------------
// Pipeline 단위 테스트
// ---------------------------------------------------------------------------

#[test]
fn pipeline_two_stages_propagate_context() {
    // EmotionAgent → GuideAgent 파이프라인을 CommandDispatcher를 통해 실행
    let ctx = TestContext::new();
    let (mut dispatcher, store) = make_dispatcher(ctx.repo);

    let cmd = appraise_cmd();
    let emotion_agent = EmotionAgent::new();
    let guide_agent = GuideAgent::new();

    let pipeline = Pipeline::new()
        .add_stage(Box::new(move |state| {
            emotion_agent.handle_appraise(
                "mu_baek", "gyo_ryong",
                &Some(SituationInput {
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
                &state.context,
            )
        }))
        .add_stage(Box::new(move |state| {
            // Stage 2: Stage 1에서 감정 상태가 전파되었는지 확인
            assert!(state.context.emotion_state.is_some(), "emotion_state must propagate");
            guide_agent.handle_generate(
                "mu_baek", "gyo_ryong", &None, &state.context,
            )
        }));

    let result = dispatcher.execute_pipeline(pipeline, &cmd).unwrap();

    // 최종 결과는 GuideGenerated
    assert!(matches!(result, CommandResult::GuideGenerated(_)));

    // 이벤트: EmotionAppraised + GuideGenerated
    let events = store.get_all_events();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0].payload, EventPayload::EmotionAppraised { .. }));
    assert!(matches!(events[1].payload, EventPayload::GuideGenerated { .. }));
}

#[test]
fn pipeline_stops_on_error() {
    let ctx = TestContext::new();
    let (mut dispatcher, store) = make_dispatcher(ctx.repo);
    let cmd = appraise_cmd();

    let pipeline = Pipeline::new()
        .add_stage(Box::new(|_| {
            Err(MindServiceError::NpcNotFound("nonexistent".into()))
        }))
        .add_stage(Box::new(|_| {
            panic!("Stage 2 should not run");
        }));

    let result = dispatcher.execute_pipeline(pipeline, &cmd);
    assert!(result.is_err());
    assert_eq!(store.get_all_events().len(), 0); // 에러 시 이��트 없음
}

#[test]
fn empty_pipeline_returns_error() {
    let ctx = TestContext::new();
    let (mut dispatcher, _) = make_dispatcher(ctx.repo);
    let cmd = appraise_cmd();

    let pipeline = Pipeline::new();
    let result = dispatcher.execute_pipeline(pipeline, &cmd);
    assert!(result.is_err());
}

#[test]
fn pipeline_accumulates_events() {
    let ctx = TestContext::new();
    let (mut dispatcher, store) = make_dispatcher(ctx.repo);
    let cmd = appraise_cmd();

    let emotion_agent = EmotionAgent::new();

    let pipeline = Pipeline::new()
        .add_stage(Box::new(move |state| {
            emotion_agent.handle_appraise(
                "mu_baek", "gyo_ryong",
                &Some(SituationInput {
                    description: "배신".into(),
                    event: Some(EventInput {
                        description: "".into(),
                        desirability_for_self: -0.6,
                        other: None,
                        prospect: None,
                    }),
                    action: None,
                    object: None,
                }),
                &state.context,
            )
        }));

    dispatcher.execute_pipeline(pipeline, &cmd).unwrap();
    assert!(!store.get_all_events().is_empty());
}

// ---------------------------------------------------------------------------
// TieredEventBus 테스트
// ---------------------------------------------------------------------------

#[test]
fn tiered_bus_sync_listener_called_inline() {
    let bus = TieredEventBus::new();
    let counter = Arc::new(AtomicUsize::new(0));

    let c = counter.clone();
    bus.subscribe_sync(move |_event| {
        c.fetch_add(1, Ordering::SeqCst);
    });

    let event = npc_mind::DomainEvent::new(
        1, "test".into(), 1,
        EventPayload::GuideGenerated { npc_id: "a".into(), partner_id: "b".into() },
    );
    bus.publish(&event);

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn tiered_bus_async_sink_receives_event() {
    let bus = TieredEventBus::new();
    let counter = Arc::new(AtomicUsize::new(0));

    let c = counter.clone();
    let sink = StdThreadSink::spawn(move |_event| {
        c.fetch_add(1, Ordering::SeqCst);
    });
    bus.register_async(sink);

    let event = npc_mind::DomainEvent::new(
        1, "test".into(), 1,
        EventPayload::GuideGenerated { npc_id: "a".into(), partner_id: "b".into() },
    );
    bus.publish(&event);

    // 백그라운드 스레드가 처리할 시간
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn tiered_bus_subscribe_alias_is_sync() {
    let bus = TieredEventBus::new();
    let counter = Arc::new(AtomicUsize::new(0));

    let c = counter.clone();
    bus.subscribe(move |_| { c.fetch_add(1, Ordering::SeqCst); });

    assert_eq!(bus.sync_listener_count(), 1);
    assert_eq!(bus.async_sink_count(), 0);
}

#[test]
fn dispatcher_with_tiered_bus_emits_to_both() {
    let ctx = TestContext::new();
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let tiered = Arc::new(TieredEventBus::new());

    let tiered_counter = Arc::new(AtomicUsize::new(0));
    let c = tiered_counter.clone();
    tiered.subscribe_sync(move |_| { c.fetch_add(1, Ordering::SeqCst); });

    let mut dispatcher = CommandDispatcher::new(ctx.repo, store.clone(), bus)
        .with_tiered_bus(tiered);

    dispatcher.dispatch(appraise_cmd()).unwrap();

    // EventStore에 저장됨
    assert!(!store.get_all_events().is_empty());
    // TieredEventBus 리스너도 호출됨
    assert!(tiered_counter.load(Ordering::SeqCst) > 0);
}
