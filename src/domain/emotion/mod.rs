//! OCC 감정 모델 (Ortony, Clore, Collins, 1988)
//!
//! 22개 감정 유형을 3개 분기로 분류:
//! 1. Event-based: 사건의 결과에 대한 반응 (joy, distress, hope, fear 등)
//! 2. Action-based: 행위자의 행동에 대한 반응 (pride, shame, admiration 등)
//! 3. Object-based: 대상에 대한 반응 (love, hate)
//!
//! 각 감정은 intensity(0.0 ~ 1.0)를 가지며,
//! HEXACO 성격이 appraisal 가중치로 작용하여 감정 강도를 조절한다.

mod appraisal;
mod engine;
mod scene;
mod situation;
mod stimulus;
mod types;

pub use engine::*;
pub use scene::*;
pub use situation::*;
pub use stimulus::*;
pub use types::*;
