//! Listener-perspective PAD 변환 도메인 모듈 (Phase 7)
//!
//! 화자 PAD → 청자 PAD 변환을 담당한다.
//!
//! ## 구성
//!
//! - [`types`] — Sign, Magnitude, PrefilterHit, 에러 타입
//! - [`prefilter`] — 정규식 기반 프리필터 (Phase 3)
//! - [`prototype`] — 공통 프로토타입 로더
//! - [`classifier`] — k-NN 공통 수학 (cosine_sim, top_k_mean)
//! - [`sign_classifier`] — 2-way 부호 분류기 (Phase 1)
//! - [`magnitude_classifier`] — 3-way 강도 분류기 (Phase 4)
//! - [`magnitude_coef`] — Magnitude 계수 테이블 + bin 경계
//! - [`converter`] — Prefilter + Sign + Magnitude 통합 trait + `EmbeddedConverter` (Phase 7 Step 3)
//!
//! ## 활성화
//!
//! 이 모듈은 `listener_perspective` feature 에서만 컴파일된다.
//! 기본 빌드에는 포함되지 않는다 (회귀 방지).
//!
//! ```bash
//! cargo test --features listener_perspective
//! ```
//!
//! ## 설계 문서
//!
//! - `docs/emotion/sign-classifier-design.md`
//! - `docs/emotion/phase7-converter-integration.md`

pub mod classifier;
pub mod converter;
pub mod magnitude_classifier;
pub mod magnitude_coef;
pub mod prefilter;
pub mod prototype;
pub mod sign_classifier;
pub mod types;

pub use converter::{
    ConvertMeta, ConvertPath, ConvertResult, EmbeddedConverter, ListenerPerspectiveConverter,
    convert_or_fallback,
};
pub use magnitude_classifier::{MagnitudeClassifier, MagnitudeClassifyResult};
pub use magnitude_coef::{MagnitudeBinThresholds, MagnitudeCoefTable};
pub use prefilter::Prefilter;
pub use prototype::{load_prototypes_from_path, load_prototypes_from_toml, Prototype, PrototypeSet};
pub use sign_classifier::{SignClassifier, SignClassifyResult};
pub use types::{ListenerPerspectiveError, Magnitude, PrefilterHit, Sign};
