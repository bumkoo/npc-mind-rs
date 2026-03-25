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

/// 인프라 포트: 텍스트 → 벡터 변환 (임베딩)
///
/// 임베딩 모델(fastembed, ort, Python 서버 등)이 이 트레이트를 구현.
/// 도메인(PadAnalyzer)은 이 트레이트에만 의존하고
/// 구체적 임베딩 구현을 알지 못한다.
pub trait TextEmbedder {
    /// 텍스트 목록 → 임베딩 벡터 목록
    fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError>;
}

/// 임베딩 오류
#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("임베딩 모델 초기화 실패: {0}")]
    InitError(String),
    #[error("임베딩 추론 실패: {0}")]
    InferenceError(String),
}

/// 도메인 포트: 대사 → PAD 변환
///
/// PadAnalyzer가 이 트레이트를 구현.
/// TextEmbedder로 벡터를 얻고, 앵커 비교로 PAD를 계산.
pub trait UtteranceAnalyzer {
    /// 대사 텍스트 → PAD (Pleasure, Arousal, Dominance)
    fn analyze(&mut self, utterance: &str) -> Result<Pad, EmbedError>;
}

/// 관계 저장소 포트 — NPC 간 관계의 영속화
///
/// 대화 종료 후 갱신된 Relationship를 저장하고,
/// 다음 대화 시작 시 로드하는 책임.
/// 인메모리, 파일, DB 등 구체적 저장 방식은 어댑터가 결정.
pub trait RelationshipRepository {
    /// NPC→상대 관계 조회. 없으면 None.
    fn find(&self, owner_id: &str, target_id: &str) -> Option<Relationship>;

    /// NPC→상대 관계 저장 (생성 또는 갱신)
    fn save(&mut self, owner_id: &str, relationship: &Relationship);

    /// 특정 NPC의 모든 관계 조회
    fn find_all(&self, owner_id: &str) -> Vec<Relationship>;
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
