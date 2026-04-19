//! Pipeline + EventBus broadcast 테스트
//!
//! B5.1: v1 Pipeline 기준. v0.3.0에서 본 파일도 삭제 예정.

#![allow(deprecated)]

mod common;

use common::TestContext;
use npc_mind::application::command::agents::{EmotionAgent, GuideAgent};
use npc_mind::application::command::dispatcher::CommandDispatcher;
use npc_mind::application::command::types::{Command, CommandResult};
use npc_mind::application::dto::*;
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::application::mind_service::MindServiceError;
use npc_mind::application::pipeline::Pipeline;
use npc_mind::domain::event::EventPayload;
use npc_mind::{EventStore, InMemoryRepository};

use futures::StreamExt;
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
// EventBus broadcast Stream 테스트
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bus_broadcast_stream_delivers_event() {
    let bus = EventBus::new();
    let counter = Arc::new(AtomicUsize::new(0));

    // subscribe 후 자기 태스크에서 Stream 소비
    let mut stream = Box::pin(bus.subscribe());
    let c = counter.clone();
    let handle = tokio::spawn(async move {
        while let Some(_event) = stream.next().await {
            c.fetch_add(1, Ordering::SeqCst);
        }
    });

    // 수신자가 준비될 때까지 짧게 대기
    tokio::task::yield_now().await;

    let event = npc_mind::DomainEvent::new(
        1,
        "test".into(),
        1,
        EventPayload::GuideGenerated {
            npc_id: "a".into(),
            partner_id: "b".into(),
        },
    );
    bus.publish(&event);

    // async 전달 시간 확보
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    drop(bus);
    let _ = handle.await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dispatcher_publishes_to_broadcast_subscribers() {
    let ctx = TestContext::new();
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());

    let counter = Arc::new(AtomicUsize::new(0));
    let mut stream = Box::pin(bus.subscribe());
    let c = counter.clone();
    let handle = tokio::spawn(async move {
        while let Some(_ev) = stream.next().await {
            c.fetch_add(1, Ordering::SeqCst);
        }
    });

    let mut dispatcher = CommandDispatcher::new(ctx.repo, store.clone(), bus);
    tokio::task::yield_now().await;

    dispatcher.dispatch(appraise_cmd()).unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    // EventStore에 저장됨
    assert!(!store.get_all_events().is_empty());
    // Broadcast 구독자도 이벤트 수신
    assert!(counter.load(Ordering::SeqCst) > 0);

    drop(dispatcher);
    let _ = handle.await;
}

#[test]
fn bus_publish_without_subscribers_does_not_panic() {
    let bus = EventBus::new();
    let event = npc_mind::DomainEvent::new(
        1,
        "test".into(),
        1,
        EventPayload::GuideGenerated {
            npc_id: "a".into(),
            partner_id: "b".into(),
        },
    );
    // 구독자 0명 — 이벤트는 drop되지만 panic/에러 없어야 함
    bus.publish(&event);
    assert_eq!(bus.receiver_count(), 0);
}
