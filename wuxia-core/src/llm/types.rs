// wuxia-core/src/llm/types.rs
//
// LLM 통신을 위한 데이터 타입 정의.
//
// 이 모듈은 순수 데이터 구조만 포함한다 (로직 없음).
// wuxia-core가 정의하고, wuxia-llm이 사용한다.
//
// 비유: 강호에서 전서구(傳書鴿)로 편지를 보낼 때,
//   LlmRequest  = 보내는 편지 (질문 + 지시)
//   LlmResponse = 돌아온 편지 (대답)
//   Message     = 편지 한 장 (누가 뭐라고 했는지)
//
// 설계 원칙:
//   - 캐릭터별 파라미터(CharacterSamplingProfile)와
//     시스템 공통 파라미터(SystemSamplingConfig)를 분리한다.
//   - CharacterSamplingProfile = "이 인물의 말투" (열전에서 정의)
//   - SystemSamplingConfig = "LLM 생성 품질 기준" (게임 전체 공통)

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Message — 대화 한 줄
// ---------------------------------------------------------------------------

/// 대화에서 누가 말했는지.
///
/// # Variants
/// - `System`: 시스템 프롬프트. NPC의 성격/상황 설명. 플레이어에게 보이지 않음.
/// - `User`: 플레이어가 입력한 텍스트.
/// - `Assistant`: NPC(소연 등)가 생성한 응답.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// 시스템 프롬프트 — "너는 소연이다. 23세 여성..."
    System,
    /// 플레이어 입력 — "넌 누구야?"
    User,
    /// NPC 응답 — "자유도시에서 제일 귀가 밝은 사람."
    Assistant,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
        }
    }
}

/// 하나의 대화 메시지.
///
/// # Example
/// ```
/// use wuxia_core::llm::{Message, Role};
///
/// let msg = Message::user("넌 누구야?");
/// assert_eq!(msg.role, Role::User);
/// assert_eq!(msg.content, "넌 누구야?");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    /// 시스템 메시지 생성.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }

    /// 플레이어(User) 메시지 생성.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    /// NPC(Assistant) 메시지 생성.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// CharacterSamplingProfile — 캐릭터별 고유 설정
// ---------------------------------------------------------------------------

/// 캐릭터별 LLM 샘플링 프로필.
///
/// NPC 열전에서 정의하는 "이 인물의 말투 특성".
/// 같은 시스템 프롬프트라도 이 값에 따라 생성 결과가 달라진다.
///
/// # 캐릭터별 예시
/// ```
/// use wuxia_core::llm::CharacterSamplingProfile;
///
/// // 소연: 활발하고 짧게 말함
/// let soyeon = CharacterSamplingProfile::new(0.7, 150, 1.1);
///
/// // 명경: 신중하고 길게 설명
/// let myungkyung = CharacterSamplingProfile::new(0.3, 300, 1.2);
///
/// // 조고: 계산적이고 돌려 말함
/// let jogo = CharacterSamplingProfile::new(0.5, 400, 1.1);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterSamplingProfile {
    /// 창의성/랜덤성 (0.0 = 확정적, 2.0 = 매우 랜덤).
    ///
    /// 캐릭터 성격과 연결:
    ///   외향적/즉흥적 캐릭터 → 높은 temperature
    ///   내성적/신중한 캐릭터 → 낮은 temperature
    pub temperature: f32,

    /// 최대 생성 토큰 수.
    ///
    /// 캐릭터 말투와 연결:
    ///   짧고 톡톡 끊는 말투 → 적은 토큰 (소연: 150)
    ///   길고 사려깊은 말투  → 많은 토큰 (명경: 300)
    pub max_tokens: u32,

    /// 반복 패널티 (1.0 = 없음, 높을수록 반복 억제).
    ///
    /// 말버릇이 있는 캐릭터는 낮게 설정 가능.
    pub repeat_penalty: f32,
}

impl CharacterSamplingProfile {
    /// 새 프로필 생성.
    pub fn new(temperature: f32, max_tokens: u32, repeat_penalty: f32) -> Self {
        Self {
            temperature,
            max_tokens,
            repeat_penalty,
        }
    }
}

impl Default for CharacterSamplingProfile {
    /// 기본값: 중립적 성격의 NPC.
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: 256,
            repeat_penalty: 1.1,
        }
    }
}

// ---------------------------------------------------------------------------
// SystemSamplingConfig — 시스템 공통 설정
// ---------------------------------------------------------------------------

/// 시스템 공통 LLM 샘플링 설정.
///
/// 모든 NPC에게 동일하게 적용되는 "LLM 생성 품질 기준".
/// 게임 전체에서 하나만 존재한다.
///
/// 비유: 캐릭터마다 목소리(Profile)는 다르지만,
///       마이크 품질(SystemConfig)은 같아야 한다.
///
/// # Example
/// ```
/// use wuxia_core::llm::SystemSamplingConfig;
///
/// let config = SystemSamplingConfig::default();
/// assert_eq!(config.top_k, 40);
/// assert_eq!(config.top_p, 0.95);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemSamplingConfig {
    /// 상위 k개 후보만 고려 (1~100).
    pub top_k: u32,

    /// 누적확률 p까지만 고려 (0.0~1.0).
    pub top_p: f32,

    /// 확률 하한선 — 이 비율 미만인 토큰 제거 (0.0~1.0).
    pub min_p: f32,

    /// 난수 시드. None이면 매번 다른 결과.
    /// Some(값)이면 재현 가능 (디버깅/테스트용).
    pub seed: Option<u64>,

    /// 이 문자열이 생성되면 즉시 중단.
    /// 예: ["\n\n", "플레이어:"]
    pub stop_tokens: Vec<String>,
}

impl Default for SystemSamplingConfig {
    fn default() -> Self {
        Self {
            top_k: 40,
            top_p: 0.95,
            min_p: 0.05,
            seed: None,
            stop_tokens: vec![],
        }
    }
}

// ---------------------------------------------------------------------------
// LlmRequest — LLM에 보내는 요청
// ---------------------------------------------------------------------------

/// LLM에 보내는 생성 요청.
///
/// 시스템 프롬프트(NPC 성격) + 대화 이력 + 샘플링 설정으로 구성.
///
/// # Example
/// ```
/// use wuxia_core::llm::{LlmRequest, Message, CharacterSamplingProfile, SystemSamplingConfig};
///
/// let request = LlmRequest {
///     system_prompt: "너는 소연이다. 23세 여성...".to_string(),
///     messages: vec![Message::user("넌 누구야?")],
///     character_profile: CharacterSamplingProfile::new(0.7, 150, 1.1),
///     system_config: SystemSamplingConfig::default(),
///     system_reminder: None,
/// };
/// assert_eq!(request.messages.len(), 1);
/// ```
#[derive(Debug, Clone)]
pub struct LlmRequest {
    /// 시스템 프롬프트 — NPC의 성격, 상황, 말투 규칙 등.
    pub system_prompt: String,

    /// 대화 이력 — User/Assistant 메시지의 교대 순서.
    pub messages: Vec<Message>,

    /// 캐릭터별 샘플링 프로필.
    pub character_profile: CharacterSamplingProfile,

    /// 시스템 공통 샘플링 설정.
    pub system_config: SystemSamplingConfig,

    /// 마지막 user 턴에 삽입할 시스템 리마인더.
    ///
    /// 대화가 길어지면 첫 턴의 시스템 프롬프트가 멀어져서
    /// 캐릭터가 흐트러질 수 있다. 리마인더는 가장 최근 user 턴
    /// 끝에 삽입되어 핵심 규칙을 상기시킨다.
    ///
    /// `None`이면 리마인더를 삽입하지 않는다.
    /// 영어 키워드(`[System Reminder]`)를 사용하면 모델이
    /// 메타 지시로 더 강하게 인식하는 경향이 있다.
    pub system_reminder: Option<String>,
}

// ---------------------------------------------------------------------------
// LlmResponse — LLM이 돌려주는 응답
// ---------------------------------------------------------------------------

/// 생성이 끝난 이유.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    /// 자연스럽게 끝남 (EOS 토큰).
    EndOfText,
    /// 최대 토큰 한도 도달.
    MaxTokens,
    /// stop_tokens 중 하나가 생성됨.
    StopToken,
}

impl fmt::Display for StopReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StopReason::EndOfText => write!(f, "end_of_text"),
            StopReason::MaxTokens => write!(f, "max_tokens"),
            StopReason::StopToken => write!(f, "stop_token"),
        }
    }
}

/// LLM 생성 응답.
///
/// # Example
/// ```
/// use wuxia_core::llm::{LlmResponse, StopReason};
///
/// let response = LlmResponse {
///     text: "자유도시에서 제일 귀가 밝은 사람.".to_string(),
///     tokens_generated: 15,
///     prompt_tokens: 0,
///     stop_reason: StopReason::EndOfText,
/// };
/// assert!(!response.text.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// 생성된 텍스트 — NPC의 대사.
    pub text: String,

    /// 생성된 토큰 수 (성능 모니터링용).
    pub tokens_generated: u32,

    /// 입력 프롬프트의 토큰 수 (OpenAI 호환성 및 요금 계산용).
    pub prompt_tokens: u32,

    /// 생성이 끝난 이유.
    pub stop_reason: StopReason,
}

// ---------------------------------------------------------------------------
// LlmError — LLM 통신 에러
// ---------------------------------------------------------------------------

/// LLM 인프라 에러.
///
/// DomainError와 분리된다:
///   DomainError = 비즈니스 로직 실패 ("7국 초과")
///   LlmError    = 인프라 실패 ("모델 로딩 실패")
#[derive(Debug, Clone, PartialEq)]
pub enum LlmError {
    /// 모델이 로드되지 않음.
    ModelNotLoaded { detail: String },

    /// 생성 실패 (내부 오류).
    GenerationFailed { detail: String },

    /// 타임아웃.
    Timeout { elapsed_ms: u64 },

    /// 응답 파싱 실패.
    InvalidResponse { detail: String },

    /// 지원하지 않는 모델 아키텍처.
    ///
    /// 새 모델 계열 추가 시 ModelArch enum에 variant를 추가하고
    /// chat template / BOS 등을 구현해야 한다.
    UnsupportedModel { detail: String },
}

impl LlmError {
    pub fn model_not_loaded(detail: impl Into<String>) -> Self {
        Self::ModelNotLoaded {
            detail: detail.into(),
        }
    }

    pub fn generation_failed(detail: impl Into<String>) -> Self {
        Self::GenerationFailed {
            detail: detail.into(),
        }
    }

    pub fn timeout(elapsed_ms: u64) -> Self {
        Self::Timeout { elapsed_ms }
    }

    pub fn invalid_response(detail: impl Into<String>) -> Self {
        Self::InvalidResponse {
            detail: detail.into(),
        }
    }

    pub fn unsupported_model(detail: impl Into<String>) -> Self {
        Self::UnsupportedModel {
            detail: detail.into(),
        }
    }
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::ModelNotLoaded { detail } => {
                write!(f, "Model not loaded: {}", detail)
            }
            LlmError::GenerationFailed { detail } => {
                write!(f, "Generation failed: {}", detail)
            }
            LlmError::Timeout { elapsed_ms } => {
                write!(f, "LLM timeout after {}ms", elapsed_ms)
            }
            LlmError::InvalidResponse { detail } => {
                write!(f, "Invalid LLM response: {}", detail)
            }
            LlmError::UnsupportedModel { detail } => {
                write!(f, "Unsupported model: {}", detail)
            }
        }
    }
}

impl std::error::Error for LlmError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Message --

    #[test]
    fn message_constructors() {
        let sys = Message::system("너는 소연이다.");
        assert_eq!(sys.role, Role::System);
        assert_eq!(sys.content, "너는 소연이다.");

        let usr = Message::user("넌 누구야?");
        assert_eq!(usr.role, Role::User);

        let ast = Message::assistant("자유도시에서 제일 귀가 밝은 사람.");
        assert_eq!(ast.role, Role::Assistant);
    }

    #[test]
    fn role_display() {
        assert_eq!(Role::System.to_string(), "system");
        assert_eq!(Role::User.to_string(), "user");
        assert_eq!(Role::Assistant.to_string(), "assistant");
    }

    #[test]
    fn message_clone_and_eq() {
        let a = Message::user("안녕");
        let b = a.clone();
        assert_eq!(a, b);
    }

    // -- CharacterSamplingProfile --

    #[test]
    fn character_profile_soyeon() {
        let soyeon = CharacterSamplingProfile::new(0.7, 150, 1.1);
        assert_eq!(soyeon.temperature, 0.7);
        assert_eq!(soyeon.max_tokens, 150);
        assert_eq!(soyeon.repeat_penalty, 1.1);
    }

    #[test]
    fn character_profile_myungkyung() {
        let myungkyung = CharacterSamplingProfile::new(0.3, 300, 1.2);
        assert_eq!(myungkyung.temperature, 0.3);
        assert_eq!(myungkyung.max_tokens, 300);
    }

    #[test]
    fn character_profile_default() {
        let default = CharacterSamplingProfile::default();
        assert_eq!(default.temperature, 0.7);
        assert_eq!(default.max_tokens, 256);
        assert_eq!(default.repeat_penalty, 1.1);
    }

    #[test]
    fn character_profiles_differ() {
        let soyeon = CharacterSamplingProfile::new(0.7, 150, 1.1);
        let myungkyung = CharacterSamplingProfile::new(0.3, 300, 1.2);
        assert_ne!(soyeon, myungkyung);
    }

    // -- SystemSamplingConfig --

    #[test]
    fn system_config_default() {
        let config = SystemSamplingConfig::default();
        assert_eq!(config.top_k, 40);
        assert_eq!(config.top_p, 0.95);
        assert_eq!(config.min_p, 0.05);
        assert_eq!(config.seed, None);
        assert!(config.stop_tokens.is_empty());
    }

    #[test]
    fn system_config_with_stop_tokens() {
        let config = SystemSamplingConfig {
            stop_tokens: vec!["\n\n".to_string(), "플레이어:".to_string()],
            ..SystemSamplingConfig::default()
        };
        assert_eq!(config.stop_tokens.len(), 2);
        assert_eq!(config.stop_tokens[1], "플레이어:");
    }

    #[test]
    fn system_config_with_seed() {
        let config = SystemSamplingConfig {
            seed: Some(12345),
            ..SystemSamplingConfig::default()
        };
        assert_eq!(config.seed, Some(12345));
    }

    // -- LlmRequest --

    #[test]
    fn llm_request_construction() {
        let request = LlmRequest {
            system_prompt: "너는 소연이다.".to_string(),
            messages: vec![Message::user("넌 누구야?")],
            character_profile: CharacterSamplingProfile::new(0.7, 150, 1.1),
            system_config: SystemSamplingConfig::default(),
            system_reminder: None,
        };
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.character_profile.temperature, 0.7);
        assert_eq!(request.system_config.top_k, 40);
        assert!(request.system_reminder.is_none());
    }

    #[test]
    fn llm_request_multi_turn() {
        let request = LlmRequest {
            system_prompt: "너는 소연이다.".to_string(),
            messages: vec![
                Message::user("넌 누구야?"),
                Message::assistant("자유도시에서 제일 귀가 밝은 사람."),
                Message::user("이름이 뭔데?"),
            ],
            character_profile: CharacterSamplingProfile::default(),
            system_config: SystemSamplingConfig::default(),
            system_reminder: None,
        };
        assert_eq!(request.messages.len(), 3);
        assert_eq!(request.messages[0].role, Role::User);
        assert_eq!(request.messages[1].role, Role::Assistant);
        assert_eq!(request.messages[2].role, Role::User);
    }

    // -- LlmResponse --

    #[test]
    fn llm_response_construction() {
        let response = LlmResponse {
            text: "자유도시에서 제일 귀가 밝은 사람.".to_string(),
            tokens_generated: 15,
            prompt_tokens: 100,
            stop_reason: StopReason::EndOfText,
        };
        assert_eq!(response.tokens_generated, 15);
        assert_eq!(response.prompt_tokens, 100);
        assert_eq!(response.stop_reason, StopReason::EndOfText);
    }

    #[test]
    fn stop_reason_display() {
        assert_eq!(StopReason::EndOfText.to_string(), "end_of_text");
        assert_eq!(StopReason::MaxTokens.to_string(), "max_tokens");
        assert_eq!(StopReason::StopToken.to_string(), "stop_token");
    }

    // -- LlmError --

    #[test]
    fn llm_error_model_not_loaded() {
        let err = LlmError::model_not_loaded("gemma3-4b.gguf not found");
        assert_eq!(err.to_string(), "Model not loaded: gemma3-4b.gguf not found");
    }

    #[test]
    fn llm_error_generation_failed() {
        let err = LlmError::generation_failed("context overflow");
        assert_eq!(err.to_string(), "Generation failed: context overflow");
    }

    #[test]
    fn llm_error_timeout() {
        let err = LlmError::timeout(5000);
        assert_eq!(err.to_string(), "LLM timeout after 5000ms");
    }

    #[test]
    fn llm_error_invalid_response() {
        let err = LlmError::invalid_response("empty output");
        assert_eq!(err.to_string(), "Invalid LLM response: empty output");
    }

    #[test]
    fn llm_error_unsupported_model() {
        let err = LlmError::unsupported_model("아키텍처 'llama'는 아직 지원하지 않습니다.");
        assert_eq!(
            err.to_string(),
            "Unsupported model: 아키텍처 'llama'는 아직 지원하지 않습니다."
        );
    }

    #[test]
    fn llm_error_is_clone_and_eq() {
        let a = LlmError::timeout(3000);
        let b = a.clone();
        assert_eq!(a, b);
    }

    // -- Serialization --

    #[test]
    fn message_serialization_roundtrip() {
        let msg = Message::user("검 팔 생각 없어?");
        let json = serde_json::to_string(&msg).unwrap();
        let restored: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, restored);
    }

    #[test]
    fn character_profile_serialization() {
        let profile = CharacterSamplingProfile::new(0.7, 150, 1.1);
        let json = serde_json::to_string(&profile).unwrap();
        let restored: CharacterSamplingProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(profile, restored);
    }

    #[test]
    fn system_config_serialization() {
        let config = SystemSamplingConfig {
            seed: Some(42),
            stop_tokens: vec!["END".to_string()],
            ..SystemSamplingConfig::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let restored: SystemSamplingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, restored);
    }

    #[test]
    fn stop_reason_serialization() {
        let reasons = vec![
            StopReason::EndOfText,
            StopReason::MaxTokens,
            StopReason::StopToken,
        ];
        for reason in reasons {
            let json = serde_json::to_string(&reason).unwrap();
            let restored: StopReason = serde_json::from_str(&json).unwrap();
            assert_eq!(reason, restored);
        }
    }
}
