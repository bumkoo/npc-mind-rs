//! 외부 의존성 어댑터
//!
//! 헥사고날 아키텍처에서 포트(ports.rs)를 구현하는 외부 어댑터들.
//! feature flag로 선택적 활성화.
//!
//! 도메인 로직(PadAnalyzer)은 domain/pad.rs에 있고,
//! 여기에는 인프라 구현만 있다.

#[cfg(feature = "embed")]
pub mod ort_embedder;

pub mod memory_repository;
pub mod toml_anchor_source;
pub mod json_anchor_source;
