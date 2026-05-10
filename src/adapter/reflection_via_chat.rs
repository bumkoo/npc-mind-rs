//! Phase 1 Reflection 어댑터 — `ConversationPort` 기반.
//!
//! 같은 LLM 서버에 *별도 세션*을 띄움으로써 dialogue 세션의 KV 캐시 보존.
//! 같은 모델, 다른 system prompt, 다른 KV slot.
//!
//! Stage 0 Findings F8.1 verified: `RigChatAdapter`의
//! `Arc<RwLock<HashMap<String, ChatSession>>>` 구조 덕분에 같은 어댑터 인스턴스로
//! dialogue + reflection 세션 동시 보유 가능.
//!
//! Stage 0 Findings F8.2: `uuid` crate가 chat feature 단독에서 미가용이므로
//! reflection_sid는 `format!("reflection-{epoch_ms}-{counter}")` 패턴으로 우회.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;

use crate::ports::chat::{ConversationError, ConversationPort};
use crate::ports::reflection::{ReflectionError, ReflectionPort, ReflectionPrompt};

#[cfg(feature = "chat")]
pub struct ConversationBackedReflectionPort<C: ConversationPort> {
    chat: Arc<C>,
    /// reflection_sid 생성용 monotonic counter (chat 단독에서 uuid 미가용 우회)
    counter: AtomicU64,
}

#[cfg(feature = "chat")]
impl<C: ConversationPort> ConversationBackedReflectionPort<C> {
    pub fn new(chat: Arc<C>) -> Self {
        Self {
            chat,
            counter: AtomicU64::new(0),
        }
    }

    /// `reflection-<epoch_ms>-<counter>` 형태로 충돌 없는 세션 ID 생성.
    /// `prompt.session_hint`가 있으면 그대로 사용 (호출자 추적 키 우선).
    fn resolve_session_id(&self, hint: Option<&str>) -> String {
        if let Some(h) = hint {
            return h.to_string();
        }
        let counter = self.counter.fetch_add(1, Ordering::Relaxed);
        let epoch_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        format!("reflection-{epoch_ms}-{counter}")
    }
}

#[cfg(feature = "chat")]
#[async_trait]
impl<C: ConversationPort + 'static> ReflectionPort for ConversationBackedReflectionPort<C> {
    async fn analyze(&self, prompt: ReflectionPrompt) -> Result<String, ReflectionError> {
        let sid = self.resolve_session_id(prompt.session_hint.as_deref());

        // 1. 별도 reflection 세션 생성 (system_prompt = 분석가 페르소나).
        //    generation_config는 None — 기본값 사용 (NPC personality 영향 받지 않는
        //    중립 분석 prompt). 향후 reflection 전용 temperature 인하 가능.
        self.chat
            .start_session(&sid, &prompt.system_prompt, None)
            .await
            .map_err(map_conversation_err)?;

        // 2. transcript + 지시 전송 → text 응답.
        let response_result = self.chat.send_message(&sid, &prompt.user_message).await;

        // 3. 세션 정리 (best-effort — send_message 실패 시도 cleanup).
        let _ = self.chat.end_session(&sid).await;

        let response = response_result.map_err(map_conversation_err)?;
        Ok(response.text)
    }
}

/// `ConversationError` → `ReflectionError` 매핑. `Timeout`은 그대로 보존.
fn map_conversation_err(e: ConversationError) -> ReflectionError {
    match e {
        ConversationError::Timeout(d) => ReflectionError::Timeout(d),
        other => ReflectionError::LlmFailure(other.to_string()),
    }
}
