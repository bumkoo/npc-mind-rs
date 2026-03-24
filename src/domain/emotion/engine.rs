//! 감정 평가 엔진 (Appraisal Engine)
//!
//! HEXACO 성격을 가중치로 사용하여 상황에서 OCC 감정을 생성하는 핵심 로직

use crate::domain::personality::HexacoProfile;

use super::types::{Emotion, EmotionState, EmotionType};
use super::situation::{PriorExpectation, Situation, SituationFocus};

// ---------------------------------------------------------------------------
// 감정 평가 엔진
// ---------------------------------------------------------------------------

/// HEXACO 성격을 가중치로 사용하여 상황에서 OCC 감정을 생성
pub struct AppraisalEngine;

/// 현재 감정 상태가 새 평가에 미치는 영향 계수
struct EmotionalMomentum {
    /// 기존 부정 감정이 새 부정 감정을 증폭 (0.0 ~ 0.5)
    negative_bias: f32,
    /// 기존 긍정 감정이 새 긍정 감정을 증폭 (0.0 ~ 0.3)
    positive_bias: f32,
    /// 기존 Anger가 patience 브레이크를 약화 (0.0 ~ 0.5)
    anger_erosion: f32,
    /// 기존 Fear/Distress가 감정 민감도를 높임 (0.0 ~ 0.3)
    sensitivity_boost: f32,
}

impl EmotionalMomentum {
    /// 부정 valence → 부정 감정 증폭 계수
    const NEGATIVE_BIAS_FACTOR: f32 = 0.5;
    /// 긍정 valence → 긍정 감정 증폭 계수
    const POSITIVE_BIAS_FACTOR: f32 = 0.3;
    /// Anger → patience 약화 계수
    const ANGER_EROSION_FACTOR: f32 = 0.5;
    /// Fear/Distress → 민감도 상승 계수
    const SENSITIVITY_BOOST_FACTOR: f32 = 0.3;
}

impl EmotionalMomentum {
    fn from_state(state: &EmotionState) -> Self {
        let anger_intensity = state.emotions().iter()
            .find(|e| e.emotion_type() == EmotionType::Anger)
            .map_or(0.0, |e| e.intensity());

        let distress_intensity = state.emotions().iter()
            .find(|e| e.emotion_type() == EmotionType::Distress)
            .map_or(0.0, |e| e.intensity());

        let fear_intensity = state.emotions().iter()
            .find(|e| e.emotion_type() == EmotionType::Fear)
            .map_or(0.0, |e| e.intensity());

        let valence = state.overall_valence();

        Self {
            // 전체 valence가 부정적이면 새 부정 감정 증폭
            negative_bias: valence.min(0.0).abs() * Self::NEGATIVE_BIAS_FACTOR,
            // 전체 valence가 긍정적이면 새 긍정 감정 약간 증폭
            positive_bias: valence.max(0.0) * Self::POSITIVE_BIAS_FACTOR,
            // 기존 Anger가 patience 효과를 갉아먹음
            anger_erosion: anger_intensity * Self::ANGER_EROSION_FACTOR,
            // 기존 Fear/Distress가 감정 민감도를 높임
            sensitivity_boost: ((fear_intensity + distress_intensity) / 2.0) * Self::SENSITIVITY_BOOST_FACTOR,
        }
    }
}

impl AppraisalEngine {
    // --- 감정 평가 가중치 상수 ---

    /// 정서성(E)이 감정 반응 강도에 미치는 증폭 계수
    const EMOTIONALITY_AMP_FACTOR: f32 = 0.3;
    /// 외향성(X)이 긍정 감정에 미치는 증폭 계수
    const EXTRAVERSION_POSITIVE_FACTOR: f32 = 0.3;
    /// 원만성(A)이 분노 완화에 미치는 계수
    const AGREEABLENESS_ANGER_MOD: f32 = 0.4;
    /// 신중함(C.prudence)이 충동 억제에 미치는 계수
    const PRUDENCE_IMPULSE_FACTOR: f32 = 0.3;
    /// 두려움(E.fearfulness)이 Fear 감정을 증폭하는 계수
    const FEARFULNESS_AMP_FACTOR: f32 = 0.5;
    /// 성실성(C)이 자기 기준(pride/shame)에 미치는 증폭 계수
    const STANDARDS_AMP_FACTOR: f32 = 0.3;
    /// 겸손(H.modesty)이 Pride를 약화하는 계수
    const MODESTY_PRIDE_FACTOR: f32 = 0.3;
    /// 온화함(A.gentleness)이 비난(Reproach)을 약화하는 계수
    const GENTLENESS_REPROACH_FACTOR: f32 = 0.3;
    /// 진실성(H.sincerity)이 감사(Gratitude)를 증폭하는 계수
    const SINCERITY_GRATITUDE_FACTOR: f32 = 0.3;
    /// 인내(A.patience)가 분노(Anger)를 억제하는 계수
    const PATIENCE_ANGER_FACTOR: f32 = 0.4;
    /// 미적 감상(O.aesthetic)이 대상 반응을 증폭하는 계수
    const AESTHETIC_AMP_FACTOR: f32 = 0.3;
    /// 공감 계수 (HappyFor/Pity 기본 강도)
    const EMPATHY_BASE: f32 = 0.5;

    /// 성격 + 상황 → 감정 상태 생성 (1회성, 이전 맥락 없음)
    pub fn appraise(personality: &HexacoProfile, situation: &Situation) -> EmotionState {
        Self::evaluate(personality, situation, &EmotionState::new())
    }

    /// 성격 + 상황 + 현재 감정 → 업데이트된 감정 상태
    ///
    /// 대화 중 감정 변화의 핵심:
    /// - 현재 감정이 새 평가의 가중치로 작용
    /// - 이미 화난 상태에서 추가 자극 → 분노가 더 쉽게 폭발
    /// - 이미 기쁜 상태에서 좋은 소식 → 기쁨이 더 증폭
    /// - 새 감정은 기존 감정 위에 누적(add)됨
    pub fn appraise_with_context(
        personality: &HexacoProfile,
        situation: &Situation,
        current_state: &EmotionState,
    ) -> EmotionState {
        Self::evaluate(personality, situation, current_state)
    }

    /// 내부 평가 구현 — appraise와 appraise_with_context의 공통 로직
    fn evaluate(
        personality: &HexacoProfile,
        situation: &Situation,
        current_state: &EmotionState,
    ) -> EmotionState {
        let momentum = EmotionalMomentum::from_state(current_state);
        let mut state = current_state.clone();

        match &situation.focus {
            SituationFocus::Event {
                desirability_for_self,
                desirability_for_other,
                is_prospective,
                prior_expectation,
            } => {
                Self::appraise_event(
                    personality, &mut state, &momentum,
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
                Self::appraise_action(
                    personality, &mut state, &momentum,
                    *is_self_agent,
                    *praiseworthiness,
                    *outcome_for_self,
                );
            }
            SituationFocus::Object { appealingness } => {
                Self::appraise_object(
                    personality, &mut state, &momentum,
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
        m: &EmotionalMomentum,
        desirability_self: f32,
        desirability_other: Option<f32>,
        is_prospective: bool,
        prior: Option<PriorExpectation>,
    ) {
        let avg = p.dimension_averages();

        // E(정서성): 감정 반응의 전반적 증폭. 높으면 더 강하게 느낌
        // + 기존 Fear/Distress가 민감도를 높임
        let emotional_amp = 1.0 + avg.e.abs() * Self::EMOTIONALITY_AMP_FACTOR + m.sensitivity_boost;
        // X(외향성): 긍정 감정 증폭. 높으면 기쁨이 더 강함
        // + 기존 긍정 감정이 긍정 반응을 증폭
        let positive_amp = 1.0 + avg.x.max(0.0) * Self::EXTRAVERSION_POSITIVE_FACTOR + m.positive_bias;
        // A(원만성): 부정 감정 완화/증폭. 높으면 부정 감정 약화
        // + 기존 Anger가 patience 브레이크를 갉아먹음
        let anger_mod = (1.0 - avg.a * Self::AGREEABLENESS_ANGER_MOD) + m.anger_erosion + m.negative_bias;
        // C(성실성): prudence가 즉각 반응 억제
        let impulse_mod = 1.0 - p.conscientiousness.prudence.value().max(0.0) * Self::PRUDENCE_IMPULSE_FACTOR;

        // --- 이전 기대에 대한 확인 감정 (Prospect-based confirmation) ---
        if let Some(expectation) = prior {
            let base = desirability_self.abs();
            match expectation {
                PriorExpectation::HopeFulfilled => {
                    state.add(Emotion::new(EmotionType::Satisfaction,
                        base * positive_amp));
                }
                PriorExpectation::HopeUnfulfilled => {
                    state.add(Emotion::new(EmotionType::Disappointment,
                        base * emotional_amp));
                }
                PriorExpectation::FearUnrealized => {
                    state.add(Emotion::new(EmotionType::Relief,
                        base * positive_amp));
                }
                PriorExpectation::FearConfirmed => {
                    state.add(Emotion::new(EmotionType::FearsConfirmed,
                        base * emotional_amp));
                }
            }
            return; // 확인 감정은 단독 처리
        }

        // --- 미래 전망 감정 (Prospect-based) ---
        if is_prospective {
            if desirability_self > 0.0 {
                state.add(Emotion::new(EmotionType::Hope,
                    desirability_self * positive_amp));
            } else if desirability_self < 0.0 {
                // E(정서성) 높으면 두려움 증폭
                let fear_amp = 1.0 + p.emotionality.fearfulness.value().max(0.0) * Self::FEARFULNESS_AMP_FACTOR;
                state.add(Emotion::new(EmotionType::Fear,
                    desirability_self.abs() * fear_amp * emotional_amp));
            }
            return;
        }

        // --- 자기 복지 감정 (Well-being) ---
        if desirability_self > 0.0 {
            state.add(Emotion::new(EmotionType::Joy,
                desirability_self * positive_amp));
        } else if desirability_self < 0.0 {
            state.add(Emotion::new(EmotionType::Distress,
                desirability_self.abs() * emotional_amp * impulse_mod));
        }

        // --- 타인의 운 감정 (Fortune-of-others) ---
        if let Some(desir_other) = desirability_other {
            let h = avg.h; // 정직-겸손성
            let a = avg.a; // 원만성

            if desir_other > 0.0 {
                // 타인에게 좋은 일 발생
                if h > 0.0 || a > 0.0 {
                    // H↑ or A↑ → 대리 기쁨 (HappyFor)
                    let empathy = (h.max(0.0) + a.max(0.0)) / 2.0;
                    state.add(Emotion::new(EmotionType::HappyFor,
                        desir_other * (Self::EMPATHY_BASE + empathy * Self::EMPATHY_BASE)));
                }
                if h < -0.2 {
                    // H↓ → 시기 (Resentment): 교활하고 탐욕적이면 질투
                    state.add(Emotion::new(EmotionType::Resentment,
                        desir_other * h.abs() * anger_mod));
                }
            } else if desir_other < 0.0 {
                // 타인에게 나쁜 일 발생
                let other_abs = desir_other.abs();
                if a > 0.0 || p.emotionality.sentimentality.value() > 0.0 {
                    // A↑ or 감상성↑ → 동정 (Pity)
                    let compassion = (a.max(0.0)
                        + p.emotionality.sentimentality.value().max(0.0)) / 2.0;
                    state.add(Emotion::new(EmotionType::Pity,
                        other_abs * (Self::EMPATHY_BASE + compassion * Self::EMPATHY_BASE)));
                }
                if h < -0.2 && a < -0.2 {
                    // H↓ + A↓ → 고소함 (Gloating)
                    let cruelty = (h.abs() + a.abs()) / 2.0;
                    state.add(Emotion::new(EmotionType::Gloating,
                        other_abs * cruelty));
                }
            }
        }
    }

    // --- Action-based 감정 평가 ---
    fn appraise_action(
        p: &HexacoProfile,
        state: &mut EmotionState,
        m: &EmotionalMomentum,
        is_self_agent: bool,
        praiseworthiness: f32,
        outcome_for_self: Option<f32>,
    ) {
        let avg = p.dimension_averages();

        // C(성실성): 자기 기준이 높으면 pride/shame 증폭
        let standards_amp = 1.0 + avg.c.abs() * Self::STANDARDS_AMP_FACTOR;
        // H(정직-겸손성): 겸손하면 pride 약화, shame 증폭
        let modesty_effect = p.honesty_humility.modesty.value();

        if is_self_agent {
            // 자기 행동 평가
            if praiseworthiness > 0.0 {
                // 겸손하면(H↑) pride가 줄어듦
                let pride_mod = 1.0 - modesty_effect.max(0.0) * Self::MODESTY_PRIDE_FACTOR;
                state.add(Emotion::new(EmotionType::Pride,
                    praiseworthiness * standards_amp * pride_mod));
            } else {
                // 성실하면(C↑) shame이 증폭 (자기 기준 위반)
                state.add(Emotion::new(EmotionType::Shame,
                    praiseworthiness.abs() * standards_amp));
            }
        } else {
            // 타인 행동 평가
            if praiseworthiness > 0.0 {
                state.add(Emotion::new(EmotionType::Admiration,
                    praiseworthiness * standards_amp));
            } else {
                // A(원만성) 낮으면 비난이 강화 + 기존 부정감정이 비난 증폭
                let reproach_amp = (1.0 - p.agreeableness.gentleness.value() * Self::GENTLENESS_REPROACH_FACTOR)
                    + m.negative_bias;
                state.add(Emotion::new(EmotionType::Reproach,
                    praiseworthiness.abs() * reproach_amp));
            }
        }

        // --- Compound: Action + Event 결합 감정 ---
        if let Some(outcome) = outcome_for_self {
            if is_self_agent {
                if praiseworthiness > 0.0 && outcome > 0.0 {
                    // 내 좋은 행동 + 좋은 결과 → Gratification
                    state.add(Emotion::new(EmotionType::Gratification,
                        (praiseworthiness + outcome) / 2.0 * standards_amp));
                } else if praiseworthiness < 0.0 && outcome < 0.0 {
                    // 내 나쁜 행동 + 나쁜 결과 → Remorse
                    state.add(Emotion::new(EmotionType::Remorse,
                        (praiseworthiness.abs() + outcome.abs()) / 2.0 * standards_amp));
                }
            } else {
                if praiseworthiness > 0.0 && outcome > 0.0 {
                    // 타인의 좋은 행동 + 나에게 좋은 결과 → Gratitude
                    let gratitude_amp = 1.0
                        + p.honesty_humility.sincerity.value().max(0.0) * Self::SINCERITY_GRATITUDE_FACTOR;
                    state.add(Emotion::new(EmotionType::Gratitude,
                        (praiseworthiness + outcome) / 2.0 * gratitude_amp));
                } else if praiseworthiness < 0.0 && outcome < 0.0 {
                    // 타인의 나쁜 행동 + 나에게 나쁜 결과 → Anger
                    // A↓이면 분노 증폭, patience↓이면 더 강하게
                    // + 기존 Anger가 patience 브레이크를 약화시킴
                    let anger_amp = (1.0
                        - p.agreeableness.patience.value() * Self::PATIENCE_ANGER_FACTOR)
                        + m.anger_erosion + m.negative_bias;
                    state.add(Emotion::new(EmotionType::Anger,
                        (praiseworthiness.abs() + outcome.abs()) / 2.0 * anger_amp));
                }
            }
        }
    }

    // --- Object-based 감정 평가 ---
    fn appraise_object(
        p: &HexacoProfile,
        state: &mut EmotionState,
        _m: &EmotionalMomentum,
        appealingness: f32,
    ) {
        // O(개방성): 미적 감상력이 높으면 대상에 대한 반응 증폭
        let aesthetic_amp = 1.0
            + p.openness.aesthetic_appreciation.value().abs() * Self::AESTHETIC_AMP_FACTOR;

        if appealingness > 0.0 {
            state.add(Emotion::new(EmotionType::Love,
                appealingness * aesthetic_amp));
        } else if appealingness < 0.0 {
            state.add(Emotion::new(EmotionType::Hate,
                appealingness.abs() * aesthetic_amp));
        }
    }
}

// ---------------------------------------------------------------------------
// Appraiser 포트 구현
// ---------------------------------------------------------------------------

impl crate::ports::Appraiser for AppraisalEngine {
    fn appraise(&self, personality: &HexacoProfile, situation: &Situation) -> EmotionState {
        AppraisalEngine::evaluate(personality, situation, &EmotionState::new())
    }

    fn appraise_with_context(
        &self,
        personality: &HexacoProfile,
        situation: &Situation,
        current_state: &EmotionState,
    ) -> EmotionState {
        AppraisalEngine::evaluate(personality, situation, current_state)
    }
}
