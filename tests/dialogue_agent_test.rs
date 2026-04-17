//! DialogueAgent 통합 테스트 (Phase 4)
//!
//! DialogueAgent가 LLM(mock)과 CommandDispatcher를 올바르게 연결하여
//! Event Sourcing 경로로 대화 턴을 발행하는지 검증한다.

#![cfg(feature = "chat")]

mod common;

use common::mock_chat::{ChatCall, MockConversationPort};
use common::TestContext;

use npc_mind::application::command::dispatcher::CommandDispatcher;
use npc_mind::application::dto::{ActionInput, EventInput, SituationInput};
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
    let store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let store_dyn: Arc<dyn EventStore> = store.clone();
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(ctx.repo, store_dyn, bus);

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

    // EventStore: EmotionAppraised 1건
    let events = store.get_all_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0].payload,
        EventPayload::EmotionAppraised { .. }
    ));

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

    // 전체 이벤트: EmotionAppraised → DialogueTurnCompleted(user)
    //   → StimulusApplied (+ BeatTransitioned/RelationshipUpdated 가능) → DialogueTurnCompleted(assistant)
    let events = store.get_all_events();

    // 첫 이벤트는 EmotionAppraised (start_session)
    assert!(matches!(
        events[0].payload,
        EventPayload::EmotionAppraised { .. }
    ));

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

    // DialogueTurnCompleted 페이로드 내용 검증
    if let EventPayload::DialogueTurnCompleted {
        speaker, utterance, ..
    } = &events[user].payload
    {
        assert_eq!(speaker, "user");
        assert_eq!(utterance, "오랜만이군.");
    }
    if let EventPayload::DialogueTurnCompleted {
        speaker, utterance, ..
    } = &events[assistant].payload
    {
        assert_eq!(speaker, "assistant");
        assert_eq!(utterance, "mock response");
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

    // PAD 없을 때 이벤트 증가량: user 턴 1 + assistant 턴 1 = 2
    let post = store.get_all_events();
    assert_eq!(
        post.len() - pre_count,
        2,
        "PAD 없으면 DialogueTurnCompleted 2건만 추가"
    );
    assert!(post
        .iter()
        .all(|e| !matches!(e.payload, EventPayload::StimulusApplied { .. })
            || post.iter().take_while(|x| x.id <= e.id).count() <= pre_count));
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

// ---------------------------------------------------------------------------
// 7. ConversationPort에 EventBus 통한 실제 broadcast 수신 확인
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dialogue_turn_events_are_published_to_event_bus() {
    let ctx = TestContext::new();
    let store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());

    // 구독을 먼저 시작하여 이후 publish된 이벤트를 모두 수신
    let mut stream = Box::pin(bus.subscribe());

    let dispatcher = CommandDispatcher::new(ctx.repo, store.clone(), bus.clone());
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
