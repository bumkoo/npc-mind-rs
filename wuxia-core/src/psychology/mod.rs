// wuxia-core/src/psychology/mod.rs
//
// 심리 도메인 (Psychology Domain)
// "이 사람은 어떤 내면을 가졌는가?"
//
// 7층 NPC 심리 아키텍처의 ①~⑤층을 구현한다.
// ⑥행동(Utility AI), ⑦성찰(LLM)은 타입 정의만 포함하며
// 실제 LLM 연동은 Phase 5에서 구현한다.
//
// # 7층 구조
//   ①층 성격 (HEXACO)    — personality.rs (거의 불변)
//   ②층 3축가치관         — three_axis.rs (느린 변화, LLM 서사용)
//   ③층 5가치             — values.rs (중간 변화, OCC 공식 입력)
//   ④층 감정 (OCC 22종)   — emotion.rs (빠른 변화)
//   ⑤층 기분 (PAD)        — mood.rs (빠른 변화, 감정 누적)
//   ⑥층 행동              — appraisal.rs::ReflectionTier (타입만)
//   ⑦층 성찰              — appraisal.rs::ReflectionTier (타입만)
//
// # 순수 함수
//   filter.rs    — HEXACO → 감정 필터
//   decay.rs     — 감정 감쇠
//   appraisal.rs — OCC 인지 평가 → 감정 생성

pub mod appraisal;
pub mod decay;
pub mod emotion;
pub mod event;
pub mod filter;
pub mod mood;
pub mod personality;
pub mod preset;
pub mod three_axis;
pub mod values;

// Re-exports for convenience
pub use appraisal::{OccAppraisal, OccStimulus, ReflectionTier, appraise_to_emotions};
pub use decay::{DEFAULT_EXPIRY_THRESHOLD, cleanup_expired, decay_emotion};
pub use filter::hexaco_emotion_filter;
pub use emotion::{ActiveEmotion, EmotionCategory, EmotionType, Valence};
pub use event::PsychologyEvent;
pub use mood::PadState;
pub use personality::{HexacoFactor, HexacoPersonality, PsychologyError};
pub use three_axis::{AxisType, CreedCandidate, ThreeAxisValues, ValueAxis};
pub use values::{PracticalValueType, PracticalValues};
