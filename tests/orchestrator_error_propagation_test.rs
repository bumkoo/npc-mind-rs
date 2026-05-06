//! DialogueOrchestrator 에러 전파 및 변환 검증 테스트
//!
//! 하위 레이어(Handler, Dispatcher, Adapter)의 에러가 Orchestrator를 거쳐
//! 올바른 variant로 변환되는지 검증한다.

#![cfg(feature = "chat")]

mod common;

use common::mock_chat::MockConversationPort;
use common::TestContext;
use npc_mind::ports::ConversationError;
use npc_mind::application::command::dispatcher::DispatchV2Error;
use npc_mind::{DialogueOrchestrator, DialogueOrchestratorError};
use npc_mind::ports::UtteranceAnalyzer;
use npc_mind::domain::pad::Pad;

use std::sync::Arc;

fn valid_situation() -> npc_mind::application::dto::SituationInput {
    npc_mind::application::dto::SituationInput {
        description: "만남".into(),
        event: Some(npc_mind::application::dto::EventInput {
            description: "만남 사건".into(),
            desirability_for_self: 0.0,
            other: None,
            prospect: None,
        }),
        action: None,
        object: None,
    }
}

// ---------------------------------------------------------------------------
// 1. DispatchV2Error 전파 검증
// ---------------------------------------------------------------------------

#[tokio::test]
async fn orchestrator_propagates_handler_error_from_dispatcher() {
    let ctx = TestContext::new();
    let alice = npc_mind::domain::personality::NpcBuilder::new("alice", "Alice").build();
    let bob = npc_mind::domain::personality::NpcBuilder::new("bob", "Bob").build();
    
    {
        let mut repo = ctx.repo.lock().unwrap();
        repo.add_npc(alice);
        repo.add_npc(bob);
    }

    let (dispatcher, _store, _bus) = common::v2_dispatcher_with_defaults(ctx.repo.clone());
    let toml = npc_mind::presentation::builtin_toml("ko").unwrap();
    let formatter = Arc::new(npc_mind::presentation::formatter::LocaleFormatter::from_toml(toml).unwrap());
    let chat = MockConversationPort::new();
    
    let mut orchestrator = DialogueOrchestrator::new(dispatcher, chat, formatter);

    // 상황 데이터 alice -> bob (하지만 이벤트 내부에 ghost가 섞여있음)
    let mut situation = valid_situation();
    situation.event.as_mut().unwrap().other = Some(npc_mind::application::dto::EventOtherInput {
        target_id: "ghost".into(),
        desirability: -0.5,
    });

    let result = orchestrator.start_session("s1", "alice", "bob", Some(situation)).await;

    match result {
        // 현재 dispatcher.rs 구현상 into_domain 실패는 InvalidSituation(String)으로 변환됨
        Err(DialogueOrchestratorError::DispatchV2(DispatchV2Error::InvalidSituation(msg))) => {
            assert!(msg.contains("ghost"));
        }
        _ => panic!("Expected InvalidSituation containing 'ghost', got {:?}", result),
    }
}

// ---------------------------------------------------------------------------
// 2. ConversationError (LLM) 전파 및 타임아웃 검증
// ---------------------------------------------------------------------------

#[tokio::test]
async fn orchestrator_propagates_conversation_timeout() {
    let ctx = TestContext::new();
    let alice = npc_mind::domain::personality::NpcBuilder::new("alice", "Alice").build();
    let bob = npc_mind::domain::personality::NpcBuilder::new("bob", "Bob").build();
    
    {
        let mut repo = ctx.repo.lock().unwrap();
        repo.add_npc(alice);
        repo.add_npc(bob);
        repo.add_relationship(npc_mind::domain::relationship::Relationship::neutral("alice", "bob"));
    }

    let (dispatcher, _store, _bus) = common::v2_dispatcher_with_defaults(ctx.repo.clone());
    let toml = npc_mind::presentation::builtin_toml("ko").unwrap();
    let formatter = Arc::new(npc_mind::presentation::formatter::LocaleFormatter::from_toml(toml).unwrap());
    
    struct FailingChat;
    #[async_trait::async_trait]
    impl npc_mind::ports::ConversationPort for FailingChat {
        async fn start_session(&self, _: &str, _: &str, _: Option<npc_mind::ports::LlmModelInfo>) -> Result<(), ConversationError> {
            Err(ConversationError::Timeout(std::time::Duration::from_secs(5)))
        }
        async fn send_message(&self, _: &str, _: &str) -> Result<npc_mind::ports::ChatResponse, ConversationError> { Ok(npc_mind::ports::ChatResponse { text: "".into(), timings: None }) }
        fn send_message_stream<'a>(&'a self, _: &'a str, _: &'a str) -> std::pin::Pin<Box<dyn futures::Stream<Item = Result<npc_mind::ports::StreamItem, ConversationError>> + Send + 'a>> { Box::pin(futures::stream::empty()) }
        async fn update_system_prompt(&self, _: &str, _: &str) -> Result<(), ConversationError> { Ok(()) }
        async fn end_session(&self, _: &str) -> Result<Vec<npc_mind::ports::DialogueTurn>, ConversationError> { Ok(vec![]) }
    }

    let mut orchestrator = DialogueOrchestrator::new(dispatcher, FailingChat, formatter);
    
    let result = orchestrator.start_session("s1", "alice", "bob", Some(valid_situation())).await;

    match result {
        Err(DialogueOrchestratorError::Conversation(ConversationError::Timeout(d))) => {
            assert_eq!(d, std::time::Duration::from_secs(5));
        }
        _ => panic!("Expected Conversation(Timeout) error, got {:?}", result),
    }
}

// ---------------------------------------------------------------------------
// 3. EmbedError (Analyzer) 전파 검증
// ---------------------------------------------------------------------------

struct FailingAnalyzer;
impl UtteranceAnalyzer for FailingAnalyzer {
    fn analyze(&mut self, _: &str) -> Result<Pad, npc_mind::ports::EmbedError> {
        Err(npc_mind::ports::EmbedError::InferenceError("forced failure".into()))
    }
}

#[tokio::test]
async fn orchestrator_propagates_analyzer_error() {
    let ctx = TestContext::new();
    let alice = npc_mind::domain::personality::NpcBuilder::new("alice", "Alice").build();
    let bob = npc_mind::domain::personality::NpcBuilder::new("bob", "Bob").build();
    
    {
        let mut repo = ctx.repo.lock().unwrap();
        repo.add_npc(alice);
        repo.add_npc(bob);
        repo.add_relationship(npc_mind::domain::relationship::Relationship::neutral("alice", "bob"));
    }

    let (dispatcher, _store, _bus) = common::v2_dispatcher_with_defaults(ctx.repo.clone());
    let toml = npc_mind::presentation::builtin_toml("ko").unwrap();
    let formatter = Arc::new(npc_mind::presentation::formatter::LocaleFormatter::from_toml(toml).unwrap());
    let chat = MockConversationPort::new();

    let mut orchestrator = DialogueOrchestrator::new(dispatcher, chat, formatter)
        .with_analyzer(FailingAnalyzer);

    orchestrator.start_session("s1", "alice", "bob", Some(valid_situation())).await.unwrap();

    let result = orchestrator.turn("s1", "hello", None, None).await;

    match result {
        Err(DialogueOrchestratorError::Embed(npc_mind::ports::EmbedError::InferenceError(msg))) => {
            assert_eq!(msg, "forced failure");
        }
        _ => panic!("Expected Embed(InferenceError) error, got {:?}", result),
    }
}
