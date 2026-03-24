//! 포트 정의 — 헥사고날 아키텍처의 확장 포인트
//!
//! 도메인 핵심 로직의 추상화 경계를 정의한다.
//! 외부 어댑터는 이 트레이트를 구현하여 도메인과 연결된다.

use crate::domain::emotion::{EmotionState, Situation};
use crate::domain::guide::ActingGuide;
use crate::domain::pad::Pad;
use crate::domain::personality::HexacoProfile;
use crate::domain::relationship::Relationship;

/// 감정 평가 포트 — HEXACO 성격 × Relationship 기반 OCC 감정 생성
///
/// 상황 진입 시 1회 평가. 대화 중 감정 변동은 StimulusProcessor가 담당.
pub trait Appraiser {
    /// 성격 + 상황 + 관계 → 감정 상태 (상황 진입 시 1회)
    fn appraise(
        &self,
        personality: &HexacoProfile,
        situation: &Situation,
        relationship: &Relationship,
    ) -> EmotionState;
}

/// 대사 자극 처리 포트 — 대화 매 턴 감정 변동
///
/// 기존 감정의 강도만 변동. 새 감정 생성 없음.
pub trait StimulusProcessor {
    /// 성격 + 현재 감정 + PAD 자극 → 갱신된 감정 상태
    fn apply_stimulus(
        &self,
        personality: &HexacoProfile,
        current_state: &EmotionState,
        stimulus: &Pad,
    ) -> EmotionState;
}

/// 대사 감정 분석 포트 — 플레이어 자유 입력 → PAD 변환
///
/// 대사 텍스트를 PAD 3축 좌표로 변환.
/// fastembed 등 외부 임베딩 모델 어댑터가 이 트레이트를 구현.
pub trait UtteranceAnalyzer {
    /// 대사 텍스트 → PAD (Pleasure, Arousal, Dominance)
    fn analyze(&mut self, utterance: &str) -> Pad;
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
