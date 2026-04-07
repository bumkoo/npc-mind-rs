//! rig-core 기반 ConversationPort 구현
//!
//! OpenAI-compatible API를 사용하는 로컬 추론 서버에 연결하여
//! Mind Engine의 ActingGuide 프롬프트로 다턴 대화를 수행한다.
//!
//! # 사용 예시
//!
//! ```rust,ignore
//! let adapter = RigChatAdapter::new(
//!     "http://127.0.0.1:8081/v1",
//!     "local-model",
//! );
//! adapter.start_session("s1", &prompt).await?;
//! let reply = adapter.send_message("s1", "안녕하시오.").await?;
//! ```

use crate::ports::{ConversationError, ConversationPort, DialogueRole, DialogueTurn, LlmInfoProvider, LlmModelInfo};
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::client::CompletionClient;
use rig::completion::{Chat, Message};
use rig::providers::openai;
use rig::streaming::{StreamedAssistantContent, StreamingChat};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// rig-core OpenAI provider를 사용하는 대화 어댑터
///
/// 세션별로 system_prompt + 대화 이력을 관리하며,
/// Beat 전환 시 system_prompt만 교체하고 이력은 유지한다.
pub struct RigChatAdapter {
    client: openai::CompletionsClient,
    model_name: RwLock<String>,
    base_url: String,
    sessions: RwLock<HashMap<String, ChatSession>>,
}

/// 개별 대화 세션 상태
struct ChatSession {
    system_prompt: String,
    /// rig Message 형식의 대화 이력 (LLM API 전달용)
    rig_history: Vec<Message>,
    /// 도메인 형식의 대화 이력 (반환용)
    dialogue_history: Vec<DialogueTurn>,
    /// 세션 고정 생성 설정
    generation_config: Option<LlmModelInfo>,
}

impl RigChatAdapter {
    /// 새 어댑터를 생성한다.
    ///
    /// - `base_url`: OpenAI-compatible API URL (예: `"http://127.0.0.1:8081/v1"`)
    /// - `model_name`: 추론 서버의 모델 이름 (예: `"local-model"`, `"qwen2.5"`)
    pub fn new(base_url: &str, model_name: &str) -> Self {
        // rig 0.33부터 OpenAI provider의 기본 API가 Responses API로 변경됨.
        // llama.cpp 등 OpenAI-compatible 로컬 서버는 Chat Completions API만 지원하므로
        // completions_api()로 명시적 전환이 필요함.
        let client = openai::Client::builder()
            .api_key("no-key-needed")
            .base_url(base_url)
            .build()
            .expect("OpenAI 호환 클라이언트 생성 실패")
            .completions_api();

        Self {
            client,
            model_name: RwLock::new(model_name.to_string()),
            base_url: base_url.to_string(),
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// LLM 서버의 `/models` 엔드포인트에서 모델명을 자동 감지하여 어댑터를 생성한다.
    ///
    /// 서버가 응답하지 않거나 모델 목록이 비어 있으면 `ConversationError::ConnectionError`를 반환한다.
    /// 호출부에서 `new()`로 폴백할 수 있다.
    pub async fn connect(base_url: &str) -> Result<Self, ConversationError> {
        let url = format!("{}/models", base_url.trim_end_matches('/'));

        let model_list: rig::model::ModelList = reqwest::get(&url)
            .await
            .map_err(|e| ConversationError::ConnectionError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ConversationError::ConnectionError(e.to_string()))?;

        let model_name = model_list
            .data
            .first()
            .map(|m| m.id.clone())
            .ok_or_else(|| {
                ConversationError::ConnectionError("모델 목록이 비어 있습니다".into())
            })?;

        Ok(Self::new(base_url, &model_name))
    }

    /// rig Agent를 빌드하고 chat()을 호출하는 내부 헬퍼
    async fn chat_with_agent(
        &self,
        system_prompt: &str,
        user_message: &str,
        history: Vec<Message>,
        config: &Option<LlmModelInfo>,
    ) -> Result<String, ConversationError> {
        let model_name = self.model_name.read().await;
        let mut builder = self.client.agent(&*model_name).preamble(system_prompt);

        // 동적 파라미터 적용
        if let Some(c) = config {
            if let Some(t) = c.temperature {
                builder = builder.temperature(t as f64);
            }
            if let Some(tp) = c.top_p {
                builder = builder.additional_params(serde_json::json!({ "top_p": tp }));
            }
            if let Some(mt) = c.max_tokens {
                builder = builder.max_tokens(mt.into());
            }
        }

        let agent = builder.build();

        let response: String = Chat::chat(&agent, user_message, history)
            .await
            .map_err(|e: rig::completion::PromptError| {
                ConversationError::InferenceError(e.to_string())
            })?;

        Ok(response)
    }

    /// rig Agent를 빌드하고 stream_chat()으로 토큰 스트리밍하는 내부 헬퍼
    ///
    /// 토큰을 `token_tx`로 실시간 전송하고, 완성된 전체 응답을 반환한다.
    async fn stream_chat_with_agent(
        &self,
        system_prompt: &str,
        user_message: &str,
        history: Vec<Message>,
        token_tx: tokio::sync::mpsc::Sender<String>,
        config: &Option<LlmModelInfo>,
    ) -> Result<String, ConversationError> {
        let model_name = self.model_name.read().await;
        let mut builder = self.client.agent(&*model_name).preamble(system_prompt);

        // 동적 파라미터 적용
        if let Some(c) = config {
            if let Some(t) = c.temperature {
                builder = builder.temperature(t as f64);
            }
            if let Some(tp) = c.top_p {
                builder = builder.additional_params(serde_json::json!({ "top_p": tp }));
            }
            if let Some(mt) = c.max_tokens {
                builder = builder.max_tokens(mt.into());
            }
        }

        let agent = builder.build();

        let mut stream = StreamingChat::stream_chat(&agent, user_message, history).await;

        let mut full_response = String::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::Text(text),
                )) => {
                    let s = text.text;
                    if !s.is_empty() {
                        full_response.push_str(&s);
                        let _ = token_tx.send(s).await;
                    }
                }
                Ok(_) => {
                    // ToolCall, Reasoning, FinalResponse 등은 무시
                }
                Err(e) => {
                    return Err(ConversationError::InferenceError(e.to_string()));
                }
            }
        }

        Ok(full_response)
    }
}

#[async_trait::async_trait]
impl ConversationPort for RigChatAdapter {
    async fn start_session(
        &self,
        session_id: &str,
        system_prompt: &str,
        generation_config: Option<LlmModelInfo>,
    ) -> Result<(), ConversationError> {
        let session = ChatSession {
            system_prompt: system_prompt.to_string(),
            rig_history: Vec::new(),
            dialogue_history: vec![DialogueTurn {
                role: DialogueRole::System,
                content: system_prompt.to_string(),
            }],
            generation_config,
        };

        self.sessions
            .write()
            .await
            .insert(session_id.to_string(), session);

        Ok(())
    }

    async fn send_message(
        &self,
        session_id: &str,
        user_message: &str,
    ) -> Result<String, ConversationError> {
        // 1. 세션에서 현재 상태를 읽어옴
        let (system_prompt, history, config) = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(session_id)
                .ok_or_else(|| ConversationError::SessionNotFound(session_id.to_string()))?;
            (
                session.system_prompt.clone(),
                session.rig_history.clone(),
                session.generation_config.clone(),
            )
        };

        // 2. rig agent로 LLM 호출 (lock 해제 상태에서 — 블로킹 방지)
        let response = self
            .chat_with_agent(&system_prompt, user_message, history, &config)
            .await?;

        // 3. 이력 업데이트
        {
            let mut sessions = self.sessions.write().await;
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| ConversationError::SessionNotFound(session_id.to_string()))?;

            // rig 이력 (다음 API 호출에 전달)
            session.rig_history.push(Message::user(user_message));
            session
                .rig_history
                .push(Message::assistant(&response));

            // 도메인 이력 (반환용)
            session.dialogue_history.push(DialogueTurn {
                role: DialogueRole::User,
                content: user_message.to_string(),
            });
            session.dialogue_history.push(DialogueTurn {
                role: DialogueRole::Assistant,
                content: response.clone(),
            });
        }

        Ok(response)
    }

    async fn send_message_stream(
        &self,
        session_id: &str,
        user_message: &str,
        token_tx: tokio::sync::mpsc::Sender<String>,
    ) -> Result<String, ConversationError> {
        // 1. 세션에서 현재 상태를 읽어옴
        let (system_prompt, history, config) = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(session_id)
                .ok_or_else(|| ConversationError::SessionNotFound(session_id.to_string()))?;
            (
                session.system_prompt.clone(),
                session.rig_history.clone(),
                session.generation_config.clone(),
            )
        };

        // 2. 스트리밍 LLM 호출 (lock 해제 상태에서 — 블로킹 방지)
        let response = self
            .stream_chat_with_agent(&system_prompt, user_message, history, token_tx, &config)
            .await?;



        // 3. 이력 업데이트
        {
            let mut sessions = self.sessions.write().await;
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| ConversationError::SessionNotFound(session_id.to_string()))?;

            session.rig_history.push(Message::user(user_message));
            session
                .rig_history
                .push(Message::assistant(&response));

            session.dialogue_history.push(DialogueTurn {
                role: DialogueRole::User,
                content: user_message.to_string(),
            });
            session.dialogue_history.push(DialogueTurn {
                role: DialogueRole::Assistant,
                content: response.clone(),
            });
        }

        Ok(response)
    }

    async fn update_system_prompt(
        &self,
        session_id: &str,
        new_prompt: &str,
    ) -> Result<(), ConversationError> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(session_id.to_string()))?;

        session.system_prompt = new_prompt.to_string();

        // 프롬프트 변경을 이력에 기록 (디버깅용)
        session.dialogue_history.push(DialogueTurn {
            role: DialogueRole::System,
            content: new_prompt.to_string(),
        });

        Ok(())
    }

    async fn end_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<DialogueTurn>, ConversationError> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .remove(session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(session_id.to_string()))?;

        Ok(session.dialogue_history)
    }
}

impl LlmInfoProvider for RigChatAdapter {
    fn get_model_info(&self) -> LlmModelInfo {
        let model_name = self.model_name.try_read()
            .map(|n| n.clone())
            .unwrap_or_else(|_| "unknown".to_string());
        LlmModelInfo {
            provider_url: self.base_url.clone(),
            model_name,
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: None,
            seed: None,
        }
    }
}

#[async_trait::async_trait]
impl crate::ports::LlmModelDetector for RigChatAdapter {
    async fn refresh_model_info(&self) -> Result<LlmModelInfo, String> {
        let url = format!("{}/models", self.base_url.trim_end_matches('/'));

        let model_list: rig::model::ModelList = reqwest::get(&url)
            .await
            .map_err(|e| format!("LLM 서버 연결 실패: {}", e))?
            .json()
            .await
            .map_err(|e| format!("모델 목록 파싱 실패: {}", e))?;

        let new_name = model_list
            .data
            .first()
            .map(|m| m.id.clone())
            .ok_or_else(|| "모델 목록이 비어 있습니다".to_string())?;

        {
            let mut name = self.model_name.write().await;
            *name = new_name;
        }

        tracing::info!("LLM 모델 재감지 완료: {}", self.model_name.read().await);
        Ok(self.get_model_info())
    }
}
