//! 감정 평가 엔진 (Appraisal Engine)
//!
//! HEXACO 성격 × Relationship 관계를 가중치로 사용하여
//! 상황(Situation)에서 OCC 감정(EmotionState)을 생성하는 핵심 로직.
//!
//! 설계 원칙:
//! - 상수 3개: PERSONALITY_WEIGHT(0.3), EMPATHY_BASE(0.5), FORTUNE_THRESHOLD(-0.2)
//! - 가중치 패턴 통일: 1.0 ± facet × PERSONALITY_WEIGHT
//! - 상황 진입 시 1회 평가 (대화 중 감정 변동은 apply_stimulus가 담당)

use crate::domain::personality::HexacoProfile;
use crate::domain::relationship::Relationship;

use super::types::{Emotion, EmotionState, EmotionType};
use super::situation::{PriorExpectation, Situation, SituationFocus};

// ---------------------------------------------------------------------------
// 감정 평가 엔진
// ---------------------------------------------------------------------------

/// HEXACO 성격 × Relationship을 가중치로 사용하여 감정을 생성
pub struct AppraisalEngine;

impl AppraisalEngine {
    // --- 상수 3개 ---

    /// 성격이 감정 강도에 미치는 범용 계수
    const W: f32 = 0.3;
    /// Fortune-of-others 기본 공감 강도
    const EMPATHY_BASE: f32 = 0.5;
    /// Fortune-of-others 발동 임계값 (H↓, A↓ 판정)
    const FORTUNE_THRESHOLD: f32 = -0.2;

    /// 성격 + 상황 + 관계 → 감정 상태 생성 (상황 진입 시 1회)
    pub fn appraise(
        personality: &HexacoProfile,
        situation: &Situation,
        relationship: &Relationship,
    ) -> EmotionState {
        let mut state = EmotionState::new();
        let rel_mul = relationship.emotion_intensity_multiplier();
        let closeness_val = relationship.closeness().value();

        match &situation.focus {
            SituationFocus::Event {
                desirability_for_self,
                desirability_for_other,
                is_prospective,
                prior_expectation,
            } => {
                Self::appraise_event(
                    personality, &mut state, rel_mul,
                    closeness_val,
                    *desirability_for_self,
                    *desirability_for_other,
                    *is_prospective,
                    *prior_expectation,
                );
            }
            SituationFocus::Action {
                is_self_agent,
                praiseworthiness,
                outcome_for_self,
            } => {
                let trust_mod = relationship.trust_emotion_modifier();
                Self::appraise_action(
                    personality, &mut state, rel_mul, trust_mod,
                    *is_self_agent,
                    *praiseworthiness,
                    *outcome_for_self,
                );
            }
            SituationFocus::Object { appealingness } => {
                Self::appraise_object(
                    personality, &mut state, rel_mul,
                    *appealingness,
                );
            }
        }
        state
    }

    // --- Event-based 감정 평가 ---
    fn appraise_event(
        p: &HexacoProfile,
        state: &mut EmotionState,
        rel_mul: f32,
        closeness_value: f32,
        desirability_self: f32,
        desirability_other: Option<f32>,
        is_prospective: bool,
        prior: Option<PriorExpectation>,
    ) {
        let avg = p.dimension_averages();
        let w = Self::W;

        // 공통 가중치 — 전부 같은 패턴
        let emotional_amp = 1.0 + avg.e.abs() * w;
        let positive_amp = 1.0 + avg.x.max(0.0) * w;
        let negative_mod = 1.0 - avg.a.max(0.0) * w;
        let impulse_mod = 1.0 - p.conscientiousness.prudence.value().max(0.0) * w;

        // 1. 이전 기대 확인 → Satisfaction / Disappointment / Relief / FearsConfirmed
        if let Some(expectation) = prior {
            let base = desirability_self.abs() * emotional_amp * rel_mul;
            match expectation {
                PriorExpectation::HopeFulfilled =>
                    state.add(Emotion::new(EmotionType::Satisfaction, base)),
                PriorExpectation::HopeUnfulfilled =>
                    state.add(Emotion::new(EmotionType::Disappointment, base)),
                PriorExpectation::FearUnrealized =>
                    state.add(Emotion::new(EmotionType::Relief, base)),
                PriorExpectation::FearConfirmed =>
                    state.add(Emotion::new(EmotionType::FearsConfirmed, base)),
            }
            return;
        }

        // 2. 미래 전망 → Hope / Fear
        if is_prospective {
            if desirability_self > 0.0 {
                state.add(Emotion::new(EmotionType::Hope,
                    desirability_self * positive_amp * rel_mul));
            } else if desirability_self < 0.0 {
                let fear_amp = 1.0 + p.emotionality.fearfulness.value().abs() * w;
                state.add(Emotion::new(EmotionType::Fear,
                    desirability_self.abs() * emotional_amp * fear_amp * rel_mul));
            }
            return;
        }

        // 3. 자기 복지 → Joy / Distress
        if desirability_self > 0.0 {
            state.add(Emotion::new(EmotionType::Joy,
                desirability_self * emotional_amp * positive_amp * rel_mul));
        } else if desirability_self < 0.0 {
            state.add(Emotion::new(EmotionType::Distress,
                desirability_self.abs() * emotional_amp * negative_mod * impulse_mod * rel_mul));
        }

        // 4. 타인의 운 → HappyFor / Pity / Gloating / Resentment
        //
        // Fortune-of-others에서는 rel_mul(closeness 절대값)을 적용하지 않는다.
        // affinity_mod/hostility_mod가 closeness의 방향 + 강도를 모두 담당한다.
        //   친구(+0.8) → affinity_mod=1.24 (증폭), hostility_mod=0.76 (억제)
        //   원수(-0.8) → affinity_mod=0.76 (억제), hostility_mod=1.24 (증폭)
        //   무관(0.0)  → 둘 다 1.0 (기본값)
        //
        // rel_mul을 곱하면 closeness가 이중 적용되어
        // "원수의 행운에 기뻐하는 강도"가 "무관한 사람의 행운에 기뻐하는 강도"보다
        // 커지는 역전 현상이 발생한다.
        if let Some(desir_other) = desirability_other {
            let t = Self::FORTUNE_THRESHOLD;
            let h = avg.h;
            let a = avg.a;

            // closeness 방향 조절자 (Fortune-of-others 전용)
            let affinity_mod = 1.0 + closeness_value * w;
            let hostility_mod = 1.0 - closeness_value * w;

            if desir_other > 0.0 {
                if h > 0.0 || a > 0.0 {
                    let empathy = (h.max(0.0) + a.max(0.0)) / 2.0;
                    state.add(Emotion::new(EmotionType::HappyFor,
                        desir_other * (Self::EMPATHY_BASE + empathy * Self::EMPATHY_BASE) * affinity_mod));
                }
                if h < t {
                    state.add(Emotion::new(EmotionType::Resentment,
                        desir_other * h.abs() * negative_mod * hostility_mod));
                }
            } else if desir_other < 0.0 {
                let abs = desir_other.abs();
                if a > 0.0 || p.emotionality.sentimentality.value() > 0.0 {
                    let compassion = (a.max(0.0)
                        + p.emotionality.sentimentality.value().max(0.0)) / 2.0;
                    state.add(Emotion::new(EmotionType::Pity,
                        abs * (Self::EMPATHY_BASE + compassion * Self::EMPATHY_BASE) * affinity_mod));
                }
                if h < t && a < t {
                    let cruelty = (h.abs() + a.abs()) / 2.0;
                    state.add(Emotion::new(EmotionType::Gloating,
                        abs * cruelty * hostility_mod));
                }
            }
        }
    }

    // --- Action-based 감정 평가 ---
    fn appraise_action(
        p: &HexacoProfile,
        state: &mut EmotionState,
        rel_mul: f32,
        trust_mod: f32,
        is_self_agent: bool,
        praiseworthiness: f32,
        outcome_for_self: Option<f32>,
    ) {
        let avg = p.dimension_averages();
        let w = Self::W;
        let standards_amp = 1.0 + avg.c.abs() * w;

        if is_self_agent {
            // 자기 행동 — trust 무관
            if praiseworthiness > 0.0 {
                let pride_mod = 1.0 - p.honesty_humility.modesty.value().max(0.0) * w;
                state.add(Emotion::new(EmotionType::Pride,
                    praiseworthiness * standards_amp * pride_mod * rel_mul));
            } else {
                state.add(Emotion::new(EmotionType::Shame,
                    praiseworthiness.abs() * standards_amp * rel_mul));
            }
        } else {
            // 타인 행동 — trust_mod 적용
            if praiseworthiness > 0.0 {
                state.add(Emotion::new(EmotionType::Admiration,
                    praiseworthiness * standards_amp * trust_mod * rel_mul));
            } else {
                let reproach_mod = 1.0 - p.agreeableness.gentleness.value().max(0.0) * w;
                state.add(Emotion::new(EmotionType::Reproach,
                    praiseworthiness.abs() * standards_amp * reproach_mod * trust_mod * rel_mul));
            }
        }

        // Compound 감정
        if let Some(outcome) = outcome_for_self {
            if is_self_agent {
                // 자기 행동 + 결과 — trust 무관
                if praiseworthiness > 0.0 && outcome > 0.0 {
                    state.add(Emotion::new(EmotionType::Gratification,
                        (praiseworthiness + outcome) / 2.0 * standards_amp * rel_mul));
                } else if praiseworthiness < 0.0 && outcome < 0.0 {
                    state.add(Emotion::new(EmotionType::Remorse,
                        (praiseworthiness.abs() + outcome.abs()) / 2.0 * standards_amp * rel_mul));
                }
            } else {
                // 타인 행동 + 결과 — trust_mod 적용
                if praiseworthiness > 0.0 && outcome > 0.0 {
                    let gratitude_amp = 1.0 + p.honesty_humility.sincerity.value().max(0.0) * w;
                    state.add(Emotion::new(EmotionType::Gratitude,
                        (praiseworthiness + outcome) / 2.0 * gratitude_amp * trust_mod * rel_mul));
                } else if praiseworthiness < 0.0 && outcome < 0.0 {
                    let anger_mod = 1.0 - p.agreeableness.patience.value() * w;
                    state.add(Emotion::new(EmotionType::Anger,
                        (praiseworthiness.abs() + outcome.abs()) / 2.0 * anger_mod * trust_mod * rel_mul));
                }
            }
        }
    }

    // --- Object-based 감정 평가 ---
    fn appraise_object(
        p: &HexacoProfile,
        state: &mut EmotionState,
        rel_mul: f32,
        appealingness: f32,
    ) {
        let aesthetic_amp = 1.0 + p.openness.aesthetic_appreciation.value().abs() * Self::W;

        if appealingness > 0.0 {
            state.add(Emotion::new(EmotionType::Love,
                appealingness * aesthetic_amp * rel_mul));
        } else if appealingness < 0.0 {
            state.add(Emotion::new(EmotionType::Hate,
                appealingness.abs() * aesthetic_amp * rel_mul));
        }
    }
}

// ---------------------------------------------------------------------------
// Appraiser 포트 구현
// ---------------------------------------------------------------------------

impl crate::ports::Appraiser for AppraisalEngine {
    fn appraise(
        &self,
        personality: &HexacoProfile,
        situation: &Situation,
        relationship: &Relationship,
    ) -> EmotionState {
        AppraisalEngine::appraise(personality, situation, relationship)
    }
}
