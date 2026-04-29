// wuxia-core/src/psychology/appraisal.rs
//
// OCC 인지 평가 (OCC Cognitive Appraisal)
//
// 자극(OccStimulus) → 평가(OccAppraisal) → 감정(EmotionType, intensity)
//
// 핵심 공식:
//   final_intensity = |appraisal| × 100 × value_weight × mood_bias × hexaco_filter
//
// 3가지 평가 대상:
//   EventConsequence — 사건의 결과 (바람직성)
//   AgentAction      — 행위자의 행동 (칭찬할 만함)
//   ObjectAspect     — 대상의 속성 (호감)
//
// 4가지 복합 감정:
//   뿌듯함 = Pride + Joy
//   회한   = Shame + Distress
//   감은   = Admiration + Joy
//   분노   = Reproach + Distress

use serde::{Deserialize, Serialize};

use crate::shared::id::CharacterId;

use super::emotion::EmotionType;
use super::filter::hexaco_emotion_filter;
use super::mood::PadState;
use super::personality::HexacoPersonality;
use super::values::{PracticalValueType, PracticalValues};

// ---------------------------------------------------------------------------
// ReflectionTier — 성찰 등급
// ---------------------------------------------------------------------------

/// 성찰 등급 (변경 범위를 결정한다).
///
/// # Example
/// ```
/// use wuxia_core::psychology::ReflectionTier;
///
/// let tier = ReflectionTier::Instant;
/// assert_eq!(format!("{:?}", tier), "Instant");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReflectionTier {
    /// Tier 1: 순간 반응 (코드, <1ms)
    Instant,
    /// Tier 2: 일상 성찰 (LLM, 하루 1회)
    Daily,
    /// Tier 3: 전환점 성찰 (LLM, 중대 사건)
    TurningPoint,
    /// Tier 4: 인생 성찰 (LLM, 매우 드묾)
    Life,
}

// ---------------------------------------------------------------------------
// OccStimulus — 자극
// ---------------------------------------------------------------------------

/// OCC 인지 평가의 자극.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OccStimulus {
    /// 사건의 결과 (목표 관련성으로 평가).
    EventConsequence {
        description: String,
        /// true=전망(아직 확정 아님), false=확정(이미 일어남)
        is_prospective: bool,
        /// Some=타인의 운에 대한 감정, None=자기 사건
        concerns_other: Option<CharacterId>,
    },
    /// 행위자의 행동 (기준 부합으로 평가).
    AgentAction {
        agent_id: CharacterId,
        /// true=자기 행동, false=타인 행동
        is_self: bool,
    },
    /// 대상의 속성 (호감으로 평가).
    ObjectAspect {
        description: String,
        /// 친숙도 0.0~100.0
        familiarity: f32,
    },
}

// ---------------------------------------------------------------------------
// OccAppraisal — 평가 결과
// ---------------------------------------------------------------------------

/// OCC 인지 평가 결과.
///
/// 자극에 대한 3가지 평가 차원의 값을 담는다.
/// 해당하지 않는 차원은 0.0으로 설정한다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OccAppraisal {
    pub stimulus: OccStimulus,
    /// 바람직성 (-1.0 ~ +1.0) — 사건용
    pub desirability: f32,
    /// 칭찬할 만함 (-1.0 ~ +1.0) — 행동용
    pub praiseworthiness: f32,
    /// 호감 (-1.0 ~ +1.0) — 대상용
    pub appealingness: f32,
    /// 관련 가치와 관여도 (가치, 관여도 0.0~1.0)
    pub relevant_values: Vec<(PracticalValueType, f32)>,
}

// ---------------------------------------------------------------------------
// 평가 → 감정 변환
// ---------------------------------------------------------------------------

/// OCC 인지 평가를 감정으로 변환한다.
///
/// 핵심 공식:
///   base_intensity = |appraisal_value| × 100
///   value_weight = Σ(value_score × relevance) / 100
///   mood_bias = 1.0 + P × 0.3
///   personality_filter = hexaco_emotion_filter(...)
///   final = base × value_weight × mood_bias × filter, clamped 0~100
///
/// 복합 감정이 발생할 수 있다 (예: Reproach + Distress → Anger).
///
/// # Example
/// ```
/// use wuxia_core::psychology::{
///     PracticalValues, HexacoPersonality, PadState,
///     PracticalValueType, ReflectionTier, EmotionType,
/// };
/// use wuxia_core::psychology::appraisal::*;
/// use wuxia_core::shared::CharacterId;
///
/// let values = PracticalValues::new(CharacterId::new(1), 90.0, 90.0, 70.0, 30.0, 20.0);
/// let personality = HexacoPersonality::new(CharacterId::new(1), 90, 50, 50, 80, 90, 60);
/// let mood = PadState::neutral();
///
/// let appraisal = OccAppraisal {
///     stimulus: OccStimulus::AgentAction {
///         agent_id: CharacterId::new(2),
///         is_self: false,
///     },
///     desirability: -0.9,
///     praiseworthiness: -0.95,
///     appealingness: 0.0,
///     relevant_values: vec![(PracticalValueType::Righteousness, 0.9)],
/// };
///
/// let emotions = appraise_to_emotions(&appraisal, &values, &personality, &mood);
/// assert!(!emotions.is_empty());
/// ```
pub fn appraise_to_emotions(
    appraisal: &OccAppraisal,
    values: &PracticalValues,
    personality: &HexacoPersonality,
    mood: &PadState,
) -> Vec<(EmotionType, f32)> {
    let mut results = Vec::new();

    match &appraisal.stimulus {
        OccStimulus::EventConsequence {
            is_prospective,
            concerns_other,
            ..
        } => {
            let d = appraisal.desirability;
            if d.abs() < 0.01 {
                return results;
            }

            match (concerns_other, d > 0.0, *is_prospective) {
                // 자기 사건
                (None, true, false) => {
                    results.push((EmotionType::Joy, calc(EmotionType::Joy, d, appraisal, values, personality, mood)));
                }
                (None, false, false) => {
                    results.push((EmotionType::Distress, calc(EmotionType::Distress, d, appraisal, values, personality, mood)));
                }
                (None, true, true) => {
                    results.push((EmotionType::Hope, calc(EmotionType::Hope, d, appraisal, values, personality, mood)));
                }
                (None, false, true) => {
                    results.push((EmotionType::Fear, calc(EmotionType::Fear, d, appraisal, values, personality, mood)));
                }
                // 타인 사건
                (Some(_), true, _) => {
                    results.push((EmotionType::HappyFor, calc(EmotionType::HappyFor, d, appraisal, values, personality, mood)));
                }
                (Some(_), false, _) => {
                    results.push((EmotionType::Pity, calc(EmotionType::Pity, d, appraisal, values, personality, mood)));
                }
            }
        }

        OccStimulus::AgentAction { is_self, .. } => {
            let p = appraisal.praiseworthiness;
            if p.abs() < 0.01 {
                return results;
            }

            let action_emotion = match (*is_self, p > 0.0) {
                (true, true) => EmotionType::Pride,
                (true, false) => EmotionType::Shame,
                (false, true) => EmotionType::Admiration,
                (false, false) => EmotionType::Reproach,
            };
            results.push((action_emotion, calc(action_emotion, p, appraisal, values, personality, mood)));

            // 복합 감정: 행동 + 사건 결과가 동시에 존재할 때
            let d = appraisal.desirability;
            if d.abs() >= 0.01 {
                let event_emotion = if d > 0.0 {
                    EmotionType::Joy
                } else {
                    EmotionType::Distress
                };
                let event_intensity = calc(event_emotion, d, appraisal, values, personality, mood);

                // 복합 감정 생성
                let compound = match (action_emotion, event_emotion) {
                    (EmotionType::Pride, EmotionType::Joy) => Some(EmotionType::Gratification),
                    (EmotionType::Shame, EmotionType::Distress) => Some(EmotionType::Remorse),
                    (EmotionType::Admiration, EmotionType::Joy) => Some(EmotionType::Gratitude),
                    (EmotionType::Reproach, EmotionType::Distress) => Some(EmotionType::Anger),
                    _ => None,
                };

                if let Some(compound_type) = compound {
                    let action_intensity = results.last().map(|(_, i)| *i).unwrap_or(0.0);
                    // 복합 감정 강도 = max(action, event) × 1.2, capped at 100
                    let compound_intensity = (action_intensity.max(event_intensity) * 1.2).min(100.0);
                    // calc에서 personality_filter를 적용하므로 여기서 직접 호출
                    let filtered = compound_intensity
                        * hexaco_emotion_filter(&compound_type, personality);
                    results.push((compound_type, filtered.clamp(0.0, 100.0)));
                }
            }
        }

        OccStimulus::ObjectAspect { .. } => {
            let a = appraisal.appealingness;
            if a.abs() < 0.01 {
                return results;
            }

            let emotion = if a > 0.0 {
                EmotionType::Love
            } else {
                EmotionType::Hate
            };
            results.push((emotion, calc(emotion, a, appraisal, values, personality, mood)));
        }
    }

    results
}

/// 감정 강도를 계산하는 내부 함수.
///
/// formula: base × value_weight × mood_bias × personality_filter
fn calc(
    emotion_type: EmotionType,
    appraisal_value: f32,
    appraisal: &OccAppraisal,
    values: &PracticalValues,
    personality: &HexacoPersonality,
    mood: &PadState,
) -> f32 {
    let base = appraisal_value.abs() * 100.0;

    // 가치 가중: 관련 가치의 가중 합
    let value_weight = if appraisal.relevant_values.is_empty() {
        1.0 // 관련 가치가 없으면 1.0 (기본)
    } else {
        let weighted_sum: f32 = appraisal
            .relevant_values
            .iter()
            .map(|(vt, relevance)| values.get(*vt) * relevance)
            .sum();
        let total_relevance: f32 = appraisal.relevant_values.iter().map(|(_, r)| r).sum();
        if total_relevance > 0.0 {
            weighted_sum / (total_relevance * 100.0)
        } else {
            1.0
        }
    };

    let mood_bias = mood.mood_bias();
    let personality_filter = hexaco_emotion_filter(&emotion_type, personality);

    let result = base * value_weight * mood_bias * personality_filter;
    result.clamp(0.0, 100.0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "appraisal_tests.rs"]
mod tests;
