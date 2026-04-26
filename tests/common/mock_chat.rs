//! `ConversationPort` 테스트용 목(mock) 구현
//!
//! 고정 응답을 반환하고 호출 이력을 `Arc<Mutex>` 벡터에 기록해
//! DialogueOrchestrator가 올바른 순서로 ConversationPort를 호출하는지 검증한다.

#![cfg(feature = "chat")]

use async_trait::async_trait;
use futures::Stream;
use npc_mind::ports::{
    ChatResponse, ConversationError, ConversationPort, DialogueRole, DialogueTurn, LlamaTimings,
    LlmModelInfo, StreamItem,
};
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

/// ConversationPort에 대한 호출 이력 항목
#[derive(Debug, Clone)]
pub enum ChatCall {
    StartSession {
        session_id: String,
        prompt: String,
    },
    SendMessage {
        session_id: String,
        user_message: String,
    },
    UpdateSystemPrompt {
        session_id: String,
        new_prompt: String,
    },
    EndSession {
        session_id: String,
    },
}

/// 설정 가능한 mock ConversationPort
///
/// - `responses`: `send_message`가 FIFO(선입선출)로 반환할 응답 큐.
///   비어있으면 기본값("mock response", timings=None)을 반환.
/// - `calls`: 모든 호출 이력.
pub struct MockConversationPort {
    pub calls: Arc<Mutex<Vec<ChatCall>>>,
    pub responses: Arc<Mutex<VecDeque<ChatResponse>>>,
    /// 세션별 누적된 대화 이력 (end_session 반환용)
    pub history: Arc<Mutex<Vec<DialogueTurn>>>,
}

impl MockConversationPort {
    pub fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            responses: Arc::new(Mutex::new(VecDeque::new())),
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 응답 큐에 뒤(push_back)로 추가 — `send_message`가 FIFO로 소비한다.
    pub fn with_response(self, text: &str, timings: Option<LlamaTimings>) -> Self {
        self.responses.lock().unwrap().push_back(ChatResponse {
            text: text.to_string(),
            timings,
        });
        self
    }

    pub fn calls(&self) -> Vec<ChatCall> {
        self.calls.lock().unwrap().clone()
    }
}

impl Default for MockConversationPort {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConversationPort for MockConversationPort {
    async fn start_session(
        &self,
        session_id: &str,
        system_prompt: &str,
        _generation_config: Option<LlmModelInfo>,
    ) -> Result<(), ConversationError> {
        self.calls.lock().unwrap().push(ChatCall::StartSession {
            session_id: session_id.to_string(),
            prompt: system_prompt.to_string(),
        });
        Ok(())
    }

    async fn send_message(
        &self,
        session_id: &str,
        user_message: &str,
    ) -> Result<ChatResponse, ConversationError> {
        self.calls.lock().unwrap().push(ChatCall::SendMessage {
            session_id: session_id.to_string(),
            user_message: user_message.to_string(),
        });

        // 이력 기록 (end_session이 반환할 데이터)
        {
            let mut h = self.history.lock().unwrap();
            h.push(DialogueTurn {
                role: DialogueRole::User,
                content: user_message.to_string(),
            });
        }

        let response = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(ChatResponse {
                text: "mock response".to_string(),
                timings: None,
            });

        self.history.lock().unwrap().push(DialogueTurn {
            role: DialogueRole::Assistant,
            content: response.text.clone(),
        });

        Ok(response)
    }

    fn send_message_stream<'a>(
        &'a self,
        session_id: &'a str,
        user_message: &'a str,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamItem, ConversationError>> + Send + 'a>> {
        // 테스트에서는 stream 경로의 토큰 분할을 검증하지 않으므로,
        // send_message 결과를 단일 Final item으로 그대로 반환한다.
        Box::pin(async_stream::stream! {
            match self.send_message(session_id, user_message).await {
                Ok(resp) => yield Ok(StreamItem::Final(resp)),
                Err(e) => yield Err(e),
            }
        })
    }

    async fn update_system_prompt(
        &self,
        session_id: &str,
        new_prompt: &str,
    ) -> Result<(), ConversationError> {
        self.calls
            .lock()
            .unwrap()
            .push(ChatCall::UpdateSystemPrompt {
                session_id: session_id.to_string(),
                new_prompt: new_prompt.to_string(),
            });
        Ok(())
    }

    async fn end_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<DialogueTurn>, ConversationError> {
        self.calls.lock().unwrap().push(ChatCall::EndSession {
            session_id: session_id.to_string(),
        });
        let mut history = self.history.lock().unwrap();
        let out = history.clone();
        history.clear();
        Ok(out)
    }
}
