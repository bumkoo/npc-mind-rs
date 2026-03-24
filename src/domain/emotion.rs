//! OCC 감정 모델 (Ortony, Clore, Collins, 1988)
//!
//! 22개 감정 유형을 3개 분기로 분류:
//! 1. Event-based: 사건의 결과에 대한 반응 (joy, distress, hope, fear 등)
//! 2. Action-based: 행위자의 행동에 대한 반응 (pride, shame, admiration 등)
//! 3. Object-based: 대상에 대한 반응 (love, hate)
//!
//! 각 감정은 intensity(0.0 ~ 1.0)를 가지며,
//! HEXACO 성격이 appraisal 가중치로 작용하여 감정 강도를 조절한다.

use serde::{Deserialize, Serialize};

use super::personality::HexacoProfile;

// ---------------------------------------------------------------------------
// OCC 감정 유형 (22개)
// ---------------------------------------------------------------------------

/// OCC 모델의 22개 감정 유형
///
/// 3개 분기: Event(사건), Action(행동), Object(대상)
/// 각 감정은 양/음의 valence를 가짐
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmotionType {
    // === Event-based: 사건의 결과 ===

    // Well-being (자기 복지)
    /// 자신에게 바람직한 사건 → 기쁨
    Joy,
    /// 자신에게 바람직하지 않은 사건 → 고통
    Distress,

    // Fortune-of-others (타인의 운)
    /// 타인에게 바람직한 사건 + 나도 기쁨 → 대리기쁨
    HappyFor,
    /// 타인에게 바람직하지 않은 사건 + 내가 안타까움 → 동정
    Pity,
    /// 타인에게 바람직하지 않은 사건 + 내가 기쁨 → 고소함
    Gloating,
    /// 타인에게 바람직한 사건 + 내가 불쾌 → 시기/원망
    Resentment,

    // Prospect-based (전망)
    /// 바람직한 사건이 일어날 가능성 → 희망
    Hope,
    /// 바람직하지 않은 사건이 일어날 가능성 → 두려움
    Fear,
    /// 바랐던 일이 실현됨 → 만족
    Satisfaction,
    /// 바랐던 일이 실현되지 않음 → 실망
    Disappointment,
    /// 두려워했던 일이 일어나지 않음 → 안도
    Relief,
    /// 두려워했던 일이 실현됨 → 공포확인
    FearsConfirmed,

    // === Action-based: 행위자의 행동 ===

    // Attribution (귀인)
    /// 자신의 행동을 긍정 평가 → 자부심
    Pride,
    /// 자신의 행동을 부정 평가 → 수치심
    Shame,
    /// 타인의 행동을 긍정 평가 → 감탄
    Admiration,
    /// 타인의 행동을 부정 평가 → 비난
    Reproach,

    // Compound: Well-being + Attribution
    /// Pride + Joy → 자신의 좋은 행동이 좋은 결과를 낳음
    Gratification,
    /// Shame + Distress → 자신의 나쁜 행동이 나쁜 결과를 낳음
    Remorse,
    /// Admiration + Joy → 타인의 좋은 행동이 나에게 좋은 결과
    Gratitude,
    /// Reproach + Distress → 타인의 나쁜 행동이 나에게 나쁜 결과
    Anger,

    // === Object-based: 대상에 대한 반응 ===
    /// 매력적인 대상 → 좋아함
    Love,
    /// 비매력적인 대상 → 싫어함
    Hate,
}

impl EmotionType {
    /// 이 감정의 기본 valence (양수=긍정, 음수=부정)
    pub fn base_valence(&self) -> f32 {
        match self {
            Self::Joy | Self::HappyFor | Self::Hope |
            Self::Satisfaction | Self::Relief |
            Self::Pride | Self::Admiration |
            Self::Gratification | Self::Gratitude |
            Self::Love => 1.0,

            Self::Distress | Self::Pity | Self::Fear |
            Self::Disappointment | Self::FearsConfirmed |
            Self::Shame | Self::Reproach |
            Self::Remorse | Self::Anger |
            Self::Hate => -1.0,

            // Gloating/Resentment: 복합 valence
            Self::Gloating => 0.5,    // 긍정이지만 어두운 기쁨
            Self::Resentment => -0.5, // 부정이지만 질투 성격
        }
    }

    /// OCC 분기 분류
    pub fn branch(&self) -> EmotionBranch {
        match self {
            Self::Joy | Self::Distress |
            Self::HappyFor | Self::Pity | Self::Gloating | Self::Resentment |
            Self::Hope | Self::Fear |
            Self::Satisfaction | Self::Disappointment |
            Self::Relief | Self::FearsConfirmed => EmotionBranch::Event,

            Self::Pride | Self::Shame |
            Self::Admiration | Self::Reproach |
            Self::Gratification | Self::Remorse |
            Self::Gratitude | Self::Anger => EmotionBranch::Action,

            Self::Love | Self::Hate => EmotionBranch::Object,
        }
    }
}

/// OCC 3대 분기
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmotionBranch {
    Event,  // 사건의 결과
    Action, // 행위자의 행동
    Object, // 대상의 속성
}

// ---------------------------------------------------------------------------
// 감정 인스턴스 (특정 감정 + 강도)
// ---------------------------------------------------------------------------

/// 하나의 감정 인스턴스: 감정 유형 + 강도
///
/// 필드는 캡슐화되어 있으며, getter를 통해 접근한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emotion {
    emotion_type: EmotionType,
    /// 감정 강도 (0.0 ~ 1.0, 0이면 느끼지 않음)
    intensity: f32,
}

impl Emotion {
    /// 감정 인스턴스 생성
    ///
    /// 강도는 의도적으로 0.0~1.0 범위로 클램핑된다.
    /// AppraisalEngine이 성격 가중치 곱셈 과정에서 범위를 초과하는
    /// 중간값을 생성할 수 있으므로, 정규화를 위해 클램핑을 사용한다.
    pub fn new(emotion_type: EmotionType, intensity: f32) -> Self {
        Self {
            emotion_type,
            intensity: intensity.clamp(0.0, 1.0),
        }
    }

    /// 감정 유형
    pub fn emotion_type(&self) -> EmotionType {
        self.emotion_type
    }

    /// 감정 강도 (0.0 ~ 1.0)
    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    /// 이 감정이 유의미한지 (강도가 threshold 이상)
    pub fn is_significant(&self, threshold: f32) -> bool {
        self.intensity >= threshold
    }
}

/// NPC의 현재 감정 상태: 여러 감정의 조합
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmotionState {
    emotions: Vec<Emotion>,
}

impl EmotionState {
    pub fn new() -> Self {
        Self { emotions: Vec::new() }
    }

    /// 감정 목록 접근
    pub fn emotions(&self) -> &[Emotion] {
        &self.emotions
    }

    /// 감정 추가 (같은 유형이면 강도 합산)
    pub fn add(&mut self, emotion: Emotion) {
        if let Some(existing) = self.emotions.iter_mut()
            .find(|e| e.emotion_type == emotion.emotion_type)
        {
            existing.intensity = (existing.intensity + emotion.intensity).clamp(0.0, 1.0);
        } else {
            self.emotions.push(emotion);
        }
    }

    /// 가장 강한 감정 반환
    pub fn dominant(&self) -> Option<&Emotion> {
        self.emotions.iter()
            .max_by(|a, b| a.intensity.partial_cmp(&b.intensity).unwrap())
    }

    /// threshold 이상의 유의미한 감정들만 반환 (강도 내림차순)
    pub fn significant(&self, threshold: f32) -> Vec<&Emotion> {
        let mut result: Vec<_> = self.emotions.iter()
            .filter(|e| e.is_significant(threshold))
            .collect();
        result.sort_by(|a, b| b.intensity.partial_cmp(&a.intensity).unwrap());
        result
    }

    /// 전체 감정 valence (양수=긍정적 상태, 음수=부정적 상태)
    pub fn overall_valence(&self) -> f32 {
        if self.emotions.is_empty() { return 0.0; }
        let sum: f32 = self.emotions.iter()
            .map(|e| e.emotion_type.base_valence() * e.intensity)
            .sum();
        (sum / self.emotions.len() as f32).clamp(-1.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// 상황(Situation) — 감정 생성의 입력
// ---------------------------------------------------------------------------

/// 상황의 초점 — OCC 3대 분기와 대응
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SituationFocus {
    /// 사건 발생 (누군가에게 무슨 일이 일어남)
    Event {
        /// 사건이 자신에게 바람직한 정도 (-1.0 ~ 1.0)
        desirability_for_self: f32,
        /// 사건이 타인에게 바람직한 정도 (-1.0 ~ 1.0, None이면 해당 없음)
        desirability_for_other: Option<f32>,
        /// 미래 사건인지 (true면 prospect-based 감정)
        is_prospective: bool,
        /// 이전에 예상했던 사건의 실현 여부 (None이면 새 사건)
        prior_expectation: Option<PriorExpectation>,
    },
    /// 행동 평가 (누군가가 무엇을 했음)
    Action {
        /// 행위자가 자기 자신인지
        is_self_agent: bool,
        /// 행동의 칭찬받을만한 정도 (-1.0=비난, +1.0=칭찬)
        praiseworthiness: f32,
        /// 행동의 결과가 자신에게 미친 영향 (-1.0 ~ 1.0, None이면 해당 없음)
        outcome_for_self: Option<f32>,
    },
    /// 대상 인식 (무언가를 접함)
    Object {
        /// 대상의 매력도 (-1.0=혐오, +1.0=매력)
        appealingness: f32,
    },
}

/// 이전 기대 상태 (prospect-based 감정용)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PriorExpectation {
    /// 바랐던 일이 실현됨 → Satisfaction
    HopeFulfilled,
    /// 바랐던 일이 실현되지 않음 → Disappointment
    HopeUnfulfilled,
    /// 두려워했던 일이 실현되지 않음 → Relief
    FearUnrealized,
    /// 두려워했던 일이 실현됨 → FearsConfirmed
    FearConfirmed,
}

/// 상황 설명 — 감정 엔진의 입력
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Situation {
    /// 상황 설명 텍스트
    pub description: String,
    /// 상황의 초점
    pub focus: SituationFocus,
}

// ---------------------------------------------------------------------------
// 감정 평가 엔진 (Appraisal Engine)
// HEXACO 성격 → OCC 감정 생성의 핵심
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
    fn from_state(state: &EmotionState) -> Self {
        let anger_intensity = state.emotions.iter()
            .find(|e| e.emotion_type == EmotionType::Anger)
            .map_or(0.0, |e| e.intensity);

        let distress_intensity = state.emotions.iter()
            .find(|e| e.emotion_type == EmotionType::Distress)
            .map_or(0.0, |e| e.intensity);

        let fear_intensity = state.emotions.iter()
            .find(|e| e.emotion_type == EmotionType::Fear)
            .map_or(0.0, |e| e.intensity);

        let valence = state.overall_valence();

        Self {
            // 전체 valence가 부정적이면 새 부정 감정 증폭
            negative_bias: valence.min(0.0).abs() * 0.5,
            // 전체 valence가 긍정적이면 새 긍정 감정 약간 증폭
            positive_bias: valence.max(0.0) * 0.3,
            // 기존 Anger가 patience 효과를 갉아먹음
            anger_erosion: anger_intensity * 0.5,
            // 기존 Fear/Distress가 감정 민감도를 높임
            sensitivity_boost: ((fear_intensity + distress_intensity) / 2.0) * 0.3,
        }
    }
}

impl AppraisalEngine {
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
        let emotional_amp = 1.0 + avg.e.abs() * 0.3 + m.sensitivity_boost;
        // X(외향성): 긍정 감정 증폭. 높으면 기쁨이 더 강함
        // + 기존 긍정 감정이 긍정 반응을 증폭
        let positive_amp = 1.0 + avg.x.max(0.0) * 0.3 + m.positive_bias;
        // A(원만성): 부정 감정 완화/증폭. 높으면 부정 감정 약화
        // + 기존 Anger가 patience 브레이크를 갉아먹음
        let anger_mod = (1.0 - avg.a * 0.4) + m.anger_erosion + m.negative_bias;
        // C(성실성): prudence가 즉각 반응 억제
        let impulse_mod = 1.0 - p.conscientiousness.prudence.value().max(0.0) * 0.3;

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
                let fear_amp = 1.0 + p.emotionality.fearfulness.value().max(0.0) * 0.5;
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
                        desir_other * (0.5 + empathy * 0.5)));
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
                        other_abs * (0.5 + compassion * 0.5)));
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
        let standards_amp = 1.0 + avg.c.abs() * 0.3;
        // H(정직-겸손성): 겸손하면 pride 약화, shame 증폭
        let modesty_effect = p.honesty_humility.modesty.value();

        if is_self_agent {
            // 자기 행동 평가
            if praiseworthiness > 0.0 {
                // 겸손하면(H↑) pride가 줄어듦
                let pride_mod = 1.0 - modesty_effect.max(0.0) * 0.3;
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
                let reproach_amp = (1.0 - p.agreeableness.gentleness.value() * 0.3)
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
                        + p.honesty_humility.sincerity.value().max(0.0) * 0.3;
                    state.add(Emotion::new(EmotionType::Gratitude,
                        (praiseworthiness + outcome) / 2.0 * gratitude_amp));
                } else if praiseworthiness < 0.0 && outcome < 0.0 {
                    // 타인의 나쁜 행동 + 나에게 나쁜 결과 → Anger
                    // A↓이면 분노 증폭, patience↓이면 더 강하게
                    // + 기존 Anger가 patience 브레이크를 약화시킴
                    let anger_amp = (1.0
                        - p.agreeableness.patience.value() * 0.4)
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
            + p.openness.aesthetic_appreciation.value().abs() * 0.3;

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
        Self::evaluate(personality, situation, &EmotionState::new())
    }

    fn appraise_with_context(
        &self,
        personality: &HexacoProfile,
        situation: &Situation,
        current_state: &EmotionState,
    ) -> EmotionState {
        Self::evaluate(personality, situation, current_state)
    }
}
