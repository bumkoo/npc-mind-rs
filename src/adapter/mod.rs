//! 외부 의존성 어댑터
//!
//! 헥사고날 아키텍처에서 포트(ports.rs)를 구현하는 외부 어댑터들.
//! feature flag로 선택적 활성화.

#[cfg(feature = "embed")]
pub mod embed_analyzer;
