//! 포트 정의 — 헥사고날 아키텍처의 확장 포인트
//!
//! 도메인 핵심 로직의 추상화 경계를 정의한다.
//! 외부 어댑터는 이 트레이트를 구현하여 도메인과 연결된다.

use crate::domain::emotion::{EmotionState, Situation};
use crate::domain::guide::ActingGuide;
use crate::domain::personality::HexacoProfile;

/// 감정 평가 포트 — HEXACO 성격 기반 OCC 감정 생성
///
/// 다른 심리 모델로 교체하거나 테스트용 단순 구현을 제공할 수 있다.
pub trait Appraiser {
    /// 성격 + 상황 → 감정 상태 (1회성, 이전 맥락 없음)
    fn appraise(&self, personality: &HexacoProfile, situation: &Situation) -> EmotionState;

    /// 성격 + 상황 + 현재 감정 → 업데이트된 감정 상태
    fn appraise_with_context(
        &self,
        personality: &HexacoProfile,
        situation: &Situation,
        current_state: &EmotionState,
    ) -> EmotionState;
}

/// 연기 가이드 포맷터 포트 — 가이드를 특정 형식으로 변환
///
/// 다국어 지원, 다른 LLM 포맷 등 다양한 출력 형식을 제공할 수 있다.
pub trait GuideFormatter {
    /// 프롬프트 텍스트 생성
    fn format_prompt(&self, guide: &ActingGuide) -> String;

    /// JSON 출력 생성
    fn format_json(&self, guide: &ActingGuide) -> Result<String, serde_json::Error>;
}
