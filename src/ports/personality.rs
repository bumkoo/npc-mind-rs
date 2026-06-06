use crate::domain::personality::DimensionAverages;
use crate::domain::emotion::{EmotionState, RelationshipModifiers, Situation};
use crate::domain::pad::Pad;

/// 성격 프로필 포트 — 가이드 생성 시 성격 차원 요약을 제공
pub trait PersonalityProfile {
    /// 성격 차원별 평균 점수를 반환
    fn dimension_averages(&self) -> DimensionAverages;
}

/// OCC 감정 평가 가중치 포트 — 성격 모델이 자극의 해석 강도를 반환
pub trait AppraisalWeights {
    fn desirability_self_weight(&self, desirability: f32) -> f32;
    fn desirability_prospect_weight(&self, desirability: f32) -> f32;
    fn desirability_confirmation_weight(&self, is_fear_axis: bool) -> f32;
    fn empathy_weight(&self, desirability: f32) -> f32;
    fn hostility_weight(&self, desirability: f32) -> f32;
    fn praiseworthiness_weight(&self, is_self: bool, praiseworthiness: f32) -> f32;
    fn appealingness_weight(&self, appealingness: f32) -> f32;
}

/// 감정 평가 엔진 포트 — 성격 × 상황 × 관계 modifier 기반 OCC 감정 생성
pub trait Appraiser {
    fn appraise<P: AppraisalWeights>(
        &self,
        personality: &P,
        situation: &Situation,
        dialogue_modifiers: &RelationshipModifiers,
    ) -> EmotionState;
}

/// 대사 자극 수용도 포트 — 성격이 자극을 얼마나 크게 수용하는가
pub trait StimulusWeights {
    fn stimulus_absorb_rate(&self, stimulus: &Pad) -> f32;
}

/// 대사 자극 처리 포트 — 대화 매 턴 감정 변동
pub trait StimulusProcessor {
    fn apply_stimulus<P: StimulusWeights>(
        &self,
        personality: &P,
        current_state: &EmotionState,
        stimulus: &Pad,
    ) -> EmotionState;
}
