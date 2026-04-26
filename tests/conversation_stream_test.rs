//! `ConversationPort::send_message_stream`의 Stream 반환 패턴 단위 테스트
//!
//! - tokens N개 + Final 1개 순서 검증
//! - Final 후 stream 종료 검증
//! - 에러가 `Err(item)`으로 전파되는지 검증
//!
//! 실제 LLM 서버 의존성 없이 ConversationPort를 직접 구현한 mock으로 검증한다.

#![cfg(feature = "chat")]

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use npc_mind::ports::{
    ChatResponse, ConversationError, ConversationPort, DialogueTurn, LlmModelInfo, StreamItem,
};
use std::pin::Pin;

/// 미리 정해진 토큰 시퀀스를 yield한 뒤 Final을 발행하는 mock 포트.
struct ScriptedStreamingPort {
    tokens: Vec<String>,
}

#[async_trait]
impl ConversationPort for ScriptedStreamingPort {
    async fn start_session(
        &self,
        _session_id: &str,
        _system_prompt: &str,
        _generation_config: Option<LlmModelInfo>,
    ) -> Result<(), ConversationError> {
        Ok(())
    }

    async fn send_message(
        &self,
        _session_id: &str,
        _user_message: &str,
    ) -> Result<ChatResponse, ConversationError> {
        Ok(ChatResponse {
            text: self.tokens.concat(),
            timings: None,
        })
    }

    fn send_message_stream<'a>(
        &'a self,
        _session_id: &'a str,
        _user_message: &'a str,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamItem, ConversationError>> + Send + 'a>> {
        let tokens = self.tokens.clone();
        Box::pin(async_stream::stream! {
            let mut acc = String::new();
            for t in tokens.iter() {
                acc.push_str(t);
                yield Ok(StreamItem::Token(t.clone()));
            }
            yield Ok(StreamItem::Final(ChatResponse {
                text: acc,
                timings: None,
            }));
        })
    }

    async fn update_system_prompt(
        &self,
        _session_id: &str,
        _new_prompt: &str,
    ) -> Result<(), ConversationError> {
        Ok(())
    }

    async fn end_session(
        &self,
        _session_id: &str,
    ) -> Result<Vec<DialogueTurn>, ConversationError> {
        Ok(Vec::new())
    }
}

/// 즉시 에러를 yield하는 mock 포트.
struct FailingStreamingPort;

#[async_trait]
impl ConversationPort for FailingStreamingPort {
    async fn start_session(
        &self,
        _session_id: &str,
        _system_prompt: &str,
        _generation_config: Option<LlmModelInfo>,
    ) -> Result<(), ConversationError> {
        Ok(())
    }

    async fn send_message(
        &self,
        _session_id: &str,
        _user_message: &str,
    ) -> Result<ChatResponse, ConversationError> {
        Err(ConversationError::InferenceError("scripted failure".into()))
    }

    fn send_message_stream<'a>(
        &'a self,
        _session_id: &'a str,
        _user_message: &'a str,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamItem, ConversationError>> + Send + 'a>> {
        Box::pin(async_stream::stream! {
            yield Err(ConversationError::InferenceError("scripted failure".into()));
        })
    }

    async fn update_system_prompt(
        &self,
        _session_id: &str,
        _new_prompt: &str,
    ) -> Result<(), ConversationError> {
        Ok(())
    }

    async fn end_session(
        &self,
        _session_id: &str,
    ) -> Result<Vec<DialogueTurn>, ConversationError> {
        Ok(Vec::new())
    }
}

/// 6.1 — Token N개 → Final 1회 순서 검증, Final.text가 누적 토큰과 일치.
#[tokio::test]
async fn stream_yields_tokens_then_final() {
    let port = ScriptedStreamingPort {
        tokens: vec!["안녕".into(), "하".into(), "시오".into()],
    };
    port.start_session("s1", "system prompt", None).await.unwrap();

    let mut stream = port.send_message_stream("s1", "오랜만이군");
    let mut tokens = Vec::new();
    let mut final_resp = None;

    while let Some(item) = stream.next().await {
        match item.unwrap() {
            StreamItem::Token(t) => tokens.push(t),
            StreamItem::Final(resp) => {
                assert!(final_resp.is_none(), "Final must yield exactly once");
                final_resp = Some(resp);
            }
        }
    }

    assert!(!tokens.is_empty(), "expected at least one token");
    let final_resp = final_resp.expect("Final must be yielded");
    let concatenated: String = tokens.into_iter().collect();
    assert_eq!(
        final_resp.text, concatenated,
        "Final.text must match concatenated tokens"
    );
}

/// 6.2 — Final 발행 후 stream이 정상 종료된다.
#[tokio::test]
async fn stream_terminates_after_final() {
    let port = ScriptedStreamingPort {
        tokens: vec!["hi".into()],
    };
    port.start_session("s2", "prompt", None).await.unwrap();

    let mut stream = port.send_message_stream("s2", "msg");
    let mut saw_final = false;
    let mut total = 0usize;
    while let Some(item) = stream.next().await {
        total += 1;
        if let Ok(StreamItem::Final(_)) = item {
            saw_final = true;
        }
    }

    assert!(saw_final, "stream must yield Final before terminating");
    assert!(total >= 2, "should see at least 1 token + 1 Final");
}

/// 6.3 — 에러가 `Err(item)`으로 전파되고 stream이 종료된다.
#[tokio::test]
async fn stream_propagates_errors() {
    let port = FailingStreamingPort;
    port.start_session("s3", "prompt", None).await.unwrap();

    let mut stream = port.send_message_stream("s3", "msg");
    let mut saw_error = false;
    while let Some(item) = stream.next().await {
        if item.is_err() {
            saw_error = true;
            break;
        }
    }

    assert!(saw_error, "stream must propagate errors as Err item");
}
