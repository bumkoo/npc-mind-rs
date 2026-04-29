// wuxia-core/src/llm/mod.rs
//
// LLM Module — Local LLM 통신을 위한 포트와 데이터 타입.
//
// 헥사고날 아키텍처에서 "출력 포트(Output Port)" 역할:
//   wuxia-core가 trait을 정의하고,
//   wuxia-llm이 구현한다.
//
// 모듈 구조:
//   types.rs — 데이터 타입 (Message, LlmRequest, LlmResponse, ...)
//   port.rs  — LlmPort trait (generate 메서드)
//
// 사용 예:
//   use wuxia_core::llm::{LlmPort, LlmRequest, Message};

pub mod port;
pub mod types;

// Re-export for convenience
pub use port::{LlmPort, LlmTokenCallback};
pub use types::{
    CharacterSamplingProfile, LlmError, LlmRequest, LlmResponse, Message, Role,
    StopReason, SystemSamplingConfig,
};
