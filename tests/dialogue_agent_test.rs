// B5.1: DialogueAgent가 v1 dispatch를 사용 → test에서도 allow 필요.
#![allow(deprecated)]

//! DialogueAgent 통합 테스트 (Phase 4)
//!
//! DialogueAgent가 LLM(mock)과 CommandDispatcher를 올바르게 연결하여
//! Event Sourcing 경로로 대화 턴을 발행하는지 검증한다.

#![cfg(feature = "chat")]

mod common;

use common::mock_chat::{ChatCall, MockConversationPort};
use common::TestContext;

use npc_mind::application::command::types::Command;
use npc_mind::application::dto::{
    ActionInput, ConditionInput, EventInput, SceneFocusInput, SituationInput,
};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::event::EventPayload;
use npc_mind::domain::pad::Pad;
use npc_mind::presentation::builtin_toml;
use npc_mind::presentation::formatter::LocaleFormatter;
use npc_mind::ports::GuideFormatter;
use npc_mind::{DialogueAgent, EventStore, InMemoryRepository};

use std::sync::Arc;

// ---------------------------------------------------------------------------
// 공통 셋업
// ---------------------------------------------------------------------------

fn betrayal_situation() -> SituationInput {
    SituationInput {
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
    }
}

/// DialogueAgent + EventStore + ConversationPort mock 튜플 생성
fn setup() -> (
    DialogueAgent<InMemoryRepository, MockConversationPort>,
    Arc<InMemoryEventStore>,
    Arc<std::sync::Mutex<Vec<ChatCall>>>,
) {
    let ctx = TestContext::new();
    let (dispatcher, store, _bus) = common::v2_dispatcher_with_defaults(ctx.repo);

    let toml = builtin_toml("ko").expect("ko locale");
    let formatter: Arc<dyn GuideFormatter> =
        Arc::new(LocaleFormatter::from_toml(toml).expect("formatter"));

    let chat = MockConversationPort::new();
    let calls = chat.calls.clone();

    let agent = DialogueAgent::new(dispatcher, chat, formatter);
    (agent, store, calls)
}

// ---------------------------------------------------------------------------
// 1. start_session: Appraise + LLM start
// ---------------------------------------------------------------------------

#[tokio::test]
async fn start_session_emits_emotion_appraised_and_starts_llm() {
    let (mut agent, store, calls) = setup();

    let outcome = agent
        .start_session(
            "session-1",
            "mu_baek",
            "gyo_ryong",
            Some(betrayal_situation()),
        )
        .await
        .expect("start_session ok");

    assert_eq!(outcome.session_id, "session-1");
    assert!(!outcome.appraise.emotions.is_empty(), "감정 생성");
    assert!(
        !outcome.appraise.prompt.is_empty(),
        "프롬프트가 포맷팅되어야 함"
    );

    // EventStore: v2 dispatch_v2(Appraise)는 정확히 다음 3건을 순서대로 발행:
    //   AppraiseRequested → EmotionAppraised → GuideGenerated
    // (회귀 시 누락/추가 이벤트를 잡기 위해 정확한 시퀀스 검증)
    let events = store.get_all_events();
    let kinds: Vec<_> = events.iter().map(|e| e.payload_type()).collect();
    assert_eq!(
        kinds,
        vec!["AppraiseRequested", "EmotionAppraised", "GuideGenerated"],
        "v2 Appraise dispatch는 3 이벤트를 정해진 순서로 발행해야 함"
    );

    // ConversationPort: StartSession 1회
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    match &calls[0] {
        ChatCall::StartSession { session_id, prompt } => {
            assert_eq!(session_id, "session-1");
            assert!(!prompt.is_empty());
        }
        _ => panic!("첫 호출은 StartSession이어야 함"),
    }

    assert_eq!(agent.session_count(), 1);
}

// ---------------------------------------------------------------------------
// 2. turn: DialogueTurnCompleted(user) → StimulusApplied → DialogueTurnCompleted(assistant)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn turn_emits_events_in_correct_order() {
    let (mut agent, store, calls) = setup();

    agent
        .start_session(
            "session-1",
            "mu_baek",
            "gyo_ryong",
            Some(betrayal_situation()),
        )
        .await
        .unwrap();

    let outcome = agent
        .turn(
            "session-1",
            "오랜만이군.",
            Some(Pad {
                pleasure: 0.5,
                arousal: 0.3,
                dominance: 0.2,
            }),
            None,
        )
        .await
        .expect("turn ok");

    assert_eq!(outcome.npc_response, "mock response");
    assert!(outcome.stimulus.is_some());

    // 전체 이벤트: v2 dispatch_v2는
    //   start_session: AppraiseRequested → EmotionAppraised → GuideGenerated
    //   turn:          DialogueTurnCompleted(user) → StimulusApplyRequested → StimulusApplied
    //                  (+ BeatTransitioned/RelationshipUpdated 가능) → DialogueTurnCompleted(assistant)
    let events = store.get_all_events();
    let kinds: Vec<_> = events.iter().map(|e| e.payload_type()).collect();

    // start_session은 정확히 3 이벤트 (전위)
    assert_eq!(
        &kinds[0..3],
        &["AppraiseRequested", "EmotionAppraised", "GuideGenerated"],
        "start_session prefix"
    );

    // turn 단계의 핵심 이벤트들이 (Beat 전환 가능성 있어 정확 시퀀스 X) 포함되어야 함
    assert!(kinds.contains(&"StimulusApplyRequested"));
    assert!(kinds.contains(&"StimulusApplied"));
    assert!(kinds.iter().filter(|k| **k == "DialogueTurnCompleted").count() == 2);

    // user DialogueTurnCompleted가 StimulusApplied보다 먼저 나와야 함
    let user_turn_idx = events.iter().position(|e| {
        matches!(
            &e.payload,
            EventPayload::DialogueTurnCompleted { speaker, .. } if speaker == "user"
        )
    });
    let stimulus_idx = events
        .iter()
        .position(|e| matches!(e.payload, EventPayload::StimulusApplied { .. }));
    let assistant_turn_idx = events.iter().position(|e| {
        matches!(
            &e.payload,
            EventPayload::DialogueTurnCompleted { speaker, .. } if speaker == "assistant"
        )
    });

    let user = user_turn_idx.expect("user 턴 이벤트");
    let stim = stimulus_idx.expect("stimulus 이벤트");
    let assistant = assistant_turn_idx.expect("assistant 턴 이벤트");

    assert!(user < stim, "user 턴이 stimulus 이전");
    assert!(stim < assistant, "stimulus가 assistant 턴 이전");

    // DialogueTurnCompleted 페이로드 내용 검증 (speaker/utterance/npc_id/partner_id)
    match &events[user].payload {
        EventPayload::DialogueTurnCompleted {
            speaker,
            utterance,
            npc_id,
            partner_id,
            ..
        } => {
            assert_eq!(speaker, "user");
            assert_eq!(utterance, "오랜만이군.");
            assert_eq!(npc_id, "mu_baek");
            assert_eq!(partner_id, "gyo_ryong");
        }
        _ => panic!("user 턴 payload 형식 오류"),
    }
    match &events[assistant].payload {
        EventPayload::DialogueTurnCompleted {
            speaker,
            utterance,
            npc_id,
            partner_id,
            ..
        } => {
            assert_eq!(speaker, "assistant");
            assert_eq!(utterance, "mock response");
            assert_eq!(npc_id, "mu_baek");
            assert_eq!(partner_id, "gyo_ryong");
        }
        _ => panic!("assistant 턴 payload 형식 오류"),
    }

    // ConversationPort.send_message가 user_utterance 원문으로 호출되었는지 확인
    let calls = calls.lock().unwrap();
    let send_call = calls
        .iter()
        .find(|c| matches!(c, ChatCall::SendMessage { .. }))
        .expect("send_message 호출 1회");
    match send_call {
        ChatCall::SendMessage {
            session_id,
            user_message,
        } => {
            assert_eq!(session_id, "session-1");
            assert_eq!(user_message, "오랜만이군.", "LLM에 전달된 대사가 원문과 동일");
        }
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// 3. turn without PAD: dispatcher 호출 없이 LLM만 호출
// ---------------------------------------------------------------------------

#[tokio::test]
async fn turn_without_pad_skips_stimulus_dispatch() {
    let (mut agent, store, _calls) = setup();

    agent
        .start_session(
            "session-1",
            "mu_baek",
            "gyo_ryong",
            Some(betrayal_situation()),
        )
        .await
        .unwrap();

    let pre_count = store.get_all_events().len();

    let outcome = agent
        .turn("session-1", "안녕", None, None)
        .await
        .expect("turn ok");

    assert!(outcome.stimulus.is_none(), "PAD 없음 → stimulus 없음");
    assert!(!outcome.beat_changed);

    // PAD 없을 때 증가한 이벤트는 user/assistant 턴 2건뿐이어야 한다.
    let post = store.get_all_events();
    let new_events: Vec<_> = post.iter().skip(pre_count).collect();
    assert_eq!(
        new_events.len(),
        2,
        "PAD 없으면 DialogueTurnCompleted 2건만 추가"
    );
    assert!(
        new_events
            .iter()
            .all(|e| matches!(e.payload, EventPayload::DialogueTurnCompleted { .. })),
        "추가된 이벤트 모두 DialogueTurnCompleted여야 함"
    );
}

// ---------------------------------------------------------------------------
// 4. end_session with significance: RelationshipUpdated + EmotionCleared + SceneEnded
// ---------------------------------------------------------------------------

#[tokio::test]
async fn end_session_with_significance_dispatches_end_dialogue() {
    let (mut agent, store, calls) = setup();

    agent
        .start_session(
            "session-1",
            "mu_baek",
            "gyo_ryong",
            Some(betrayal_situation()),
        )
        .await
        .unwrap();
    agent
        .turn(
            "session-1",
            "그만 가보겠다",
            Some(Pad {
                pleasure: 0.0,
                arousal: 0.0,
                dominance: 0.0,
            }),
            None,
        )
        .await
        .unwrap();

    let outcome = agent
        .end_session("session-1", Some(0.5))
        .await
        .expect("end_session ok");

    assert!(outcome.after_dialogue.is_some(), "관계 갱신 응답");
    assert!(!outcome.dialogue_history.is_empty(), "대화 이력 존재");

    // ConversationPort.end_session 호출
    let call_list = calls.lock().unwrap();
    assert!(call_list
        .iter()
        .any(|c| matches!(c, ChatCall::EndSession { .. })));

    // 이벤트: RelationshipUpdated, EmotionCleared, SceneEnded가 모두 발행됨
    let events = store.get_all_events();
    assert!(
        events
            .iter()
            .any(|e| matches!(e.payload, EventPayload::RelationshipUpdated { .. })),
        "RelationshipUpdated 발행"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e.payload, EventPayload::EmotionCleared { .. })),
        "EmotionCleared 발행"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e.payload, EventPayload::SceneEnded { .. })),
        "SceneEnded 발행"
    );

    assert_eq!(agent.session_count(), 0, "세션 메타 정리");
}

// ---------------------------------------------------------------------------
// 5. end_session without significance: LLM만 종료, dispatcher 비호출
// ---------------------------------------------------------------------------

#[tokio::test]
async fn end_session_without_significance_skips_dispatch() {
    let (mut agent, store, _calls) = setup();

    agent
        .start_session(
            "session-1",
            "mu_baek",
            "gyo_ryong",
            Some(betrayal_situation()),
        )
        .await
        .unwrap();

    let pre = store.get_all_events().len();

    let outcome = agent
        .end_session("session-1", None)
        .await
        .expect("end_session ok");

    assert!(outcome.after_dialogue.is_none());
    assert_eq!(
        store.get_all_events().len(),
        pre,
        "significance 없음 → 이벤트 추가 없음"
    );
    assert_eq!(agent.session_count(), 0);
}

// ---------------------------------------------------------------------------
// 6. 존재하지 않는 세션 → SessionNotFound
// ---------------------------------------------------------------------------

#[tokio::test]
async fn turn_on_unknown_session_fails() {
    let (mut agent, _store, _calls) = setup();

    let result = agent.turn("missing", "안녕", None, None).await;
    match result {
        Err(npc_mind::DialogueAgentError::SessionNotFound(id)) => assert_eq!(id, "missing"),
        Err(other) => panic!("기대: SessionNotFound, 실제: {}", other),
        Ok(_) => panic!("세션이 없으므로 실패해야 함"),
    }
}

/// `end_session`도 존재하지 않는 세션에서는 `SessionNotFound`로 실패해야 한다.
/// LLM도 호출되지 않아야 한다 (세션 메타 확인이 먼저).
#[tokio::test]
async fn end_session_on_unknown_session_fails() {
    let (mut agent, _store, calls) = setup();

    let result = agent.end_session("missing", Some(0.5)).await;
    match result {
        Err(npc_mind::DialogueAgentError::SessionNotFound(id)) => assert_eq!(id, "missing"),
        Err(other) => panic!("기대: SessionNotFound, 실제: {}", other),
        Ok(_) => panic!("세션이 없으므로 실패해야 함"),
    }

    // 세션 메타 확인이 실패했으므로 ConversationPort는 호출되지 않아야 한다.
    assert!(
        calls.lock().unwrap().is_empty(),
        "세션 확인 실패 시 LLM 호출 없음"
    );
}

// ---------------------------------------------------------------------------
// 7. Beat 전환 시 ConversationPort.update_system_prompt 호출
// ---------------------------------------------------------------------------

/// Beat 전환이 발생하면 DialogueAgent는 새 프롬프트로 system prompt를 갱신해야 한다.
/// Scene 설정 전략은 application_test.rs::test_beat_transition_and_emotion_merging와 동일 —
/// 교룡(gyo_ryong)은 감정 민감도가 높아 Joy가 쉽게 소멸하고,
/// Joy absent 조건의 Focus로 바로 전환된다.
#[tokio::test]
async fn beat_transition_calls_update_system_prompt() {
    let (mut agent, store, calls) = setup();

    // Scene 설정 — 두 Focus: 초기(약한 Joy) + 전환(Joy 소멸 시)
    let scene_cmd = Command::StartScene {
        npc_id: "gyo_ryong".into(),
        partner_id: "mu_baek".into(),
        significance: Some(0.5),
        focuses: vec![
            SceneFocusInput {
                id: "calm".into(),
                description: "평온한 대화".into(),
                trigger: None, // Initial
                event: Some(EventInput {
                    description: "초기 상황".into(),
                    desirability_for_self: 0.05, // 약한 Joy — 쉽게 소멸
                    other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
                test_script: vec![],
            },
            SceneFocusInput {
                id: "angry".into(),
                description: "갑작스러운 갈등".into(),
                trigger: Some(vec![vec![ConditionInput {
                    emotion: "Joy".into(),
                    absent: Some(true), // Joy가 사라지면 전환
                    below: None,
                    above: None,
                }]]),
                event: Some(EventInput {
                    description: "모욕을 당함".into(),
                    desirability_for_self: -0.6,
                    other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
                test_script: vec![],
            },
        ],
    };
    agent.dispatcher().dispatch_v2(scene_cmd).await.unwrap();

    // LLM 세션 시작 (Scene의 active focus로 자동 appraise)
    agent
        .start_session("s", "gyo_ryong", "mu_baek", None)
        .await
        .unwrap();

    // 교룡에게 강한 불쾌 자극 → Joy 소멸 → Beat 전환
    let outcome = agent
        .turn(
            "s",
            "원칙 타령은 지겹군.",
            Some(Pad {
                pleasure: -1.0,
                arousal: -1.0,
                dominance: -1.0,
            }),
            None,
        )
        .await
        .expect("turn ok");

    assert!(outcome.beat_changed, "민감한 교룡은 즉시 Beat 전환");

    // BeatTransitioned 이벤트 발행 확인
    let events = store.get_all_events();
    assert!(
        events
            .iter()
            .any(|e| matches!(e.payload, EventPayload::BeatTransitioned { .. })),
        "BeatTransitioned 이벤트 발행"
    );

    // DialogueAgent가 update_system_prompt를 호출했는지 확인
    let calls = calls.lock().unwrap();
    let update_call = calls
        .iter()
        .find(|c| matches!(c, ChatCall::UpdateSystemPrompt { .. }))
        .expect("Beat 전환 시 update_system_prompt 호출");
    match update_call {
        ChatCall::UpdateSystemPrompt {
            session_id,
            new_prompt,
        } => {
            assert_eq!(session_id, "s");
            assert!(!new_prompt.is_empty(), "새 프롬프트 비어있지 않음");
        }
        _ => unreachable!(),
    }

    // 호출 순서: StartSession → UpdateSystemPrompt → SendMessage
    // (DialogueAgent.turn 내부: stimulus → [beat이면 update_prompt] → send_message)
    let update_idx = calls
        .iter()
        .position(|c| matches!(c, ChatCall::UpdateSystemPrompt { .. }))
        .unwrap();
    let send_idx = calls
        .iter()
        .position(|c| matches!(c, ChatCall::SendMessage { .. }))
        .unwrap();
    assert!(
        update_idx < send_idx,
        "update_system_prompt가 send_message 이전에 호출되어야 함"
    );
}

// ---------------------------------------------------------------------------
// 8. ConversationPort에 EventBus 통한 실제 broadcast 수신 확인
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dialogue_turn_events_are_published_to_event_bus() {
    let ctx = TestContext::new();
    let store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());

    // 구독을 먼저 시작하여 이후 publish된 이벤트를 모두 수신
    let mut stream = Box::pin(bus.subscribe());

    let store_dyn: Arc<dyn EventStore> = store.clone();
    let dispatcher = common::v2_dispatcher(ctx.repo, store_dyn, bus.clone());
    let toml = builtin_toml("ko").unwrap();
    let formatter: Arc<dyn GuideFormatter> = Arc::new(LocaleFormatter::from_toml(toml).unwrap());
    let mut agent = DialogueAgent::new(dispatcher, MockConversationPort::new(), formatter);

    agent
        .start_session(
            "s",
            "mu_baek",
            "gyo_ryong",
            Some(betrayal_situation()),
        )
        .await
        .unwrap();
    agent
        .turn(
            "s",
            "...",
            Some(Pad {
                pleasure: 0.0,
                arousal: 0.0,
                dominance: 0.0,
            }),
            None,
        )
        .await
        .unwrap();

    // 발행된 이벤트 수신
    use futures::StreamExt;
    let mut types = Vec::new();
    for _ in 0..8 {
        // 최대 8개까지 읽어 빠르게 종료
        match tokio::time::timeout(std::time::Duration::from_millis(50), stream.next()).await {
            Ok(Some(ev)) => types.push(ev.payload_type()),
            _ => break,
        }
    }

    assert!(types.contains(&"EmotionAppraised"));
    assert!(types.contains(&"DialogueTurnCompleted"));
    assert!(types.contains(&"StimulusApplied"));
}
