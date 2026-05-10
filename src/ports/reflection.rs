//! Reflection Port — 대화 종료 후 서사 분석 LLM/AI 추상화.
//!
//! Phase 1 Mind Architecture (`relationships.md` v0.7 §6) — Outer Loop 진입 게이트.
//!
//! 본 트레이트는 *분석 호출*을 추상화. 구현체:
//! - Phase 1: `ConversationBackedReflectionPort` — 같은 LLM 모델, 별도 ConversationPort
//!   세션 (KV 캐시 격리)
//! - Phase 2.5+: 전용 모델/엔드포인트 (별도 LLM)
//! - Test: `MockReflectionPort` (in-process)
//!
//! `ReflectionService`는 본 트레이트에만 의존 — OCP 준수 (Stage 0 Findings F2 #2).
//! 구체 어댑터를 application layer에서 직접 import해서는 안 된다.

use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;

/// LLM에 보낼 reflection prompt 묶음.
///
/// Phase 1: `system_prompt` (분석가 페르소나) + `user_message` (transcript + 출력 지시).
/// `session_hint`는 별도 KV slot/세션 격리에 사용 (어댑터 구현 자율 — `None`이면 어댑터가
/// 자동 생성, `Some(..)`이면 호출자 추적 키로 사용).
#[cfg(feature = "chat")]
#[derive(Debug, Clone)]
pub struct ReflectionPrompt {
    pub system_prompt: String,
    pub user_message: String,
    pub session_hint: Option<String>,
}

/// Reflection 호출 실패 분류.
///
/// `ReflectionService`가 본 에러를 받아 `fallback_result`로 보수적 결과를 반환한다
/// (게임 진행 막힘 0). 상세 분기는 `application/reflection_service.rs`.
#[cfg(feature = "chat")]
#[derive(Debug, Error, Clone)]
pub enum ReflectionError {
    /// LLM 서버 통신 또는 추론 실패 (5xx, 네트워크, OOM 등)
    #[error("LLM 호출 실패: {0}")]
    LlmFailure(String),

    /// LLM 응답 타임아웃 (어댑터 with_timeout 빌더 적용 시)
    #[error("LLM 타임아웃 ({0:?})")]
    Timeout(Duration),

    /// 응답 파싱 실패 (JSON 깨짐, prefix/suffix 텍스트 등). `ReflectionService`가
    /// 자체 JSON 파싱하므로 본 variant는 어댑터 단계에서 거의 발생 안 함 — 향후
    /// 어댑터가 사전 검증을 추가할 때 사용.
    #[error("응답 파싱 오류: {0}")]
    ParseError(String),
}

/// Reflection 분석 추상화 — *분석 호출*만 책임. JSON 파싱·도메인 변환은
/// 호출자(`ReflectionService`)가 담당한다.
///
/// **Phase 1 어댑터 구현 원칙** (Stage 0 Findings F8.1):
/// - `ConversationBackedReflectionPort`는 `ConversationPort`의 기존 메서드만 사용
///   (`start_session` / `send_message` / `end_session`). 새 trait 메서드 추가 0.
/// - 어댑터 인스턴스는 dialogue 세션과 reflection 세션을 동시 보유 가능
///   (`Arc<RwLock<HashMap<String, ChatSession>>>` 기반).
#[cfg(feature = "chat")]
#[async_trait]
pub trait ReflectionPort: Send + Sync {
    /// LLM에 reflection prompt를 보내 *원본 텍스트 응답* 반환.
    /// JSON 파싱·도메인 변환은 호출자(`ReflectionService`) 책임.
    async fn analyze(&self, prompt: ReflectionPrompt) -> Result<String, ReflectionError>;
}
