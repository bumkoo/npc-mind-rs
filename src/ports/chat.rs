use crate::domain::personality::Npc;

/// 대화 턴 — 세션 내 한 턴의 발화
#[cfg(feature = "chat")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DialogueTurn {
    pub role: DialogueRole,
    pub content: String,
}

/// 발화 역할
#[cfg(feature = "chat")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DialogueRole {
    System,
    User,
    Assistant,
}

/// LLM 추론 엔진이 반환하는 성능 메트릭 (일반화된 형식)
#[cfg(feature = "chat")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InferenceTimings {
    pub prompt_n: u32,
    pub prompt_ms: f64,
    pub prompt_per_token_ms: f64,
    pub prompt_per_second: f64,
    pub predicted_n: u32,
    pub predicted_ms: f64,
    pub predicted_per_token_ms: f64,
    pub predicted_per_second: f64,
}

/// LLM 응답 + 선택적 성능 메트릭
#[cfg(feature = "chat")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatResponse {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub timings: Option<InferenceTimings>,
}

/// 스트리밍 응답의 단일 항목.
#[cfg(feature = "chat")]
#[derive(Debug, Clone)]
pub enum StreamItem {
    Token(String),
    Final(ChatResponse),
}

/// 대화 오케스트레이터 오류
#[cfg(feature = "chat")]
#[derive(Debug, thiserror::Error)]
pub enum ConversationError {
    #[error("LLM 연결 실패: {0}")]
    ConnectionError(String),
    #[error("세션을 찾을 수 없습니다: {0}")]
    SessionNotFound(String),
    #[error("LLM 추론 오류: {0}")]
    InferenceError(String),
}

/// LLM 모델 정보 DTO
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct LlmModelInfo {
    pub provider_url: String,
    pub model_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub seed: Option<u64>,
}

impl Default for LlmModelInfo {
    fn default() -> Self {
        Self {
            provider_url: "unknown".into(),
            model_name: "unknown".into(),
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

impl LlmModelInfo {
    pub fn apply_npc_personality(&mut self, npc: &Npc) {
        let (temp, top_p) = npc.derive_llm_parameters();
        self.temperature = Some(temp);
        self.top_p = Some(top_p);
    }
}

/// LLM 모델의 특성 및 메타데이터를 제공하는 포트
pub trait LlmInfoProvider: Send + Sync {
    fn get_model_info(&self) -> LlmModelInfo;
}

/// LLM 서버에서 모델 정보를 런타임에 재감지하는 포트
#[cfg(feature = "chat")]
#[async_trait::async_trait]
pub trait LlmModelDetector: Send + Sync {
    async fn refresh_model_info(&self) -> Result<LlmModelInfo, ConversationError>;
}

/// 대화 오케스트레이터 포트 — LLM과의 다턴 대화 세션을 추상화
#[cfg(feature = "chat")]
#[async_trait::async_trait]
pub trait ConversationPort: Send + Sync {
    async fn start_session(
        &self,
        session_id: &str,
        system_prompt: &str,
        generation_config: Option<LlmModelInfo>,
    ) -> Result<(), ConversationError>;

    async fn send_message(
        &self,
        session_id: &str,
        user_message: &str,
    ) -> Result<ChatResponse, ConversationError>;

    fn send_message_stream<'a>(
        &'a self,
        session_id: &'a str,
        user_message: &'a str,
    ) -> std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<StreamItem, ConversationError>> + Send + 'a>,
    >;

    async fn update_system_prompt(
        &self,
        session_id: &str,
        new_prompt: &str,
    ) -> Result<(), ConversationError>;

    async fn end_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<DialogueTurn>, ConversationError>;
}
