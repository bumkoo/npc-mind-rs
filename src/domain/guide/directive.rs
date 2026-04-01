//! 연기 지시문 — 감정 + 성격에서 도출된 구체적 연기 지시

use serde::{Deserialize, Serialize};

use crate::domain::emotion::{EmotionState, EmotionType};
use crate::domain::personality::DimensionAverages;
use crate::ports::PersonalityProfile;

use super::enums::{Attitude, BehavioralTendency, Restriction, Tone};
use super::{EMOTION_THRESHOLD, HONESTY_RESTRICTION_THRESHOLD, MOOD_THRESHOLD, TRAIT_THRESHOLD};

/// 감정 상태에서 도출된 구체적 연기 지시
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActingDirective {
    /// 어조
    pub tone: Tone,
    /// 태도
    pub attitude: Attitude,
    /// 행동 경향
    pub behavioral_tendency: BehavioralTendency,
    /// 금지 사항
    pub restrictions: Vec<Restriction>,
}

impl ActingDirective {
    /// 감정과 성격을 기반으로 구체적인 연기 지시를 생성합니다.
    pub fn from_emotion_and_personality(
        state: &EmotionState,
        profile: &impl PersonalityProfile,
    ) -> Self {
        let avg = profile.dimension_averages();
        let mood = state.overall_valence();

        // 판단에 필요한 정보 요약
        let dominant = state.dominant().map(|e| e.emotion_type());
        let significant = state.significant(EMOTION_THRESHOLD);
        let has_anger = significant
            .iter()
            .any(|e| e.emotion_type() == EmotionType::Anger);
        let has_fear = significant
            .iter()
            .any(|e| e.emotion_type() == EmotionType::Fear);
        let has_shame = significant
            .iter()
            .any(|e| e.emotion_type() == EmotionType::Shame);
        let has_reproach = significant
            .iter()
            .any(|e| e.emotion_type() == EmotionType::Reproach);

        Self {
            tone: Tone::decide(dominant, mood, &avg),
            attitude: Attitude::decide(has_anger, has_reproach, has_fear, mood, &avg),
            behavioral_tendency: BehavioralTendency::decide(
                has_anger, has_fear, has_shame, mood, &avg,
            ),
            restrictions: Restriction::evaluate_all(has_anger, has_fear, has_shame, mood, &avg),
        }
    }
}

// ---------------------------------------------------------------------------
// 각 요소별 의사결정 로직 (Enums에 로직 위임)
// ---------------------------------------------------------------------------

impl Tone {
    pub fn decide(dominant: Option<EmotionType>, mood: f32, avg: &DimensionAverages) -> Self {
        let t = TRAIT_THRESHOLD;
        match dominant {
            Some(EmotionType::Anger) => {
                if avg.c.value() > t {
                    Self::SuppressedCold
                } else {
                    Self::RoughAggressive
                }
            }
            Some(EmotionType::Distress) => {
                if avg.e.value() > t {
                    Self::AnxiousTrembling
                } else {
                    Self::SomberRestrained
                }
            }
            Some(EmotionType::Joy) => Self::BrightLively,
            Some(EmotionType::Fear) => {
                if avg.e.value() < -t {
                    Self::VigilantCalm
                } else {
                    Self::TenseAnxious
                }
            }
            Some(EmotionType::Shame) => Self::ShrinkingSmall,
            Some(EmotionType::Pride) => {
                if avg.h.value() > t {
                    Self::QuietConfidence
                } else {
                    Self::ProudArrogant
                }
            }
            Some(EmotionType::Reproach) => Self::CynicalCritical,
            Some(EmotionType::Disappointment) => Self::DeepSighing,
            Some(EmotionType::Gratitude) => Self::SincerelyWarm,
            Some(EmotionType::Resentment) => Self::JealousBitter,
            Some(EmotionType::Pity) => Self::CompassionateSoft,
            _ => {
                if mood > EMOTION_THRESHOLD {
                    Self::RelaxedGentle
                } else if mood < -EMOTION_THRESHOLD {
                    Self::Heavy
                } else {
                    Self::Calm
                }
            }
        }
    }
}

impl Attitude {
    pub fn decide(
        has_anger: bool,
        has_reproach: bool,
        has_fear: bool,
        mood: f32,
        avg: &DimensionAverages,
    ) -> Self {
        let t = TRAIT_THRESHOLD;
        if has_anger {
            if avg.a.value() < -t {
                Self::HostileAggressive
            } else {
                Self::SuppressedDiscomfort
            }
        } else if has_reproach {
            Self::Judgmental
        } else if has_fear {
            Self::GuardedDefensive
        } else if mood > MOOD_THRESHOLD {
            Self::FriendlyOpen
        } else if mood < -MOOD_THRESHOLD {
            Self::DefensiveClosed
        } else {
            Self::NeutralObservant
        }
    }
}

impl BehavioralTendency {
    pub fn decide(
        has_anger: bool,
        has_fear: bool,
        has_shame: bool,
        mood: f32,
        avg: &DimensionAverages,
    ) -> Self {
        let t = TRAIT_THRESHOLD;
        if has_anger {
            if avg.c.value() < -t {
                Self::ImmediateConfrontation
            } else if avg.c.value() > t {
                Self::StrategicResponse
            } else {
                Self::ExpressAndObserve
            }
        } else if has_fear {
            if avg.e.value() < -t {
                Self::BraveConfrontation
            } else {
                Self::SeekSafety
            }
        } else if has_shame {
            Self::AvoidOrDeflect
        } else if mood > MOOD_THRESHOLD {
            Self::ActiveCooperation
        } else {
            Self::ObserveAndRespond
        }
    }
}

impl Restriction {
    pub fn evaluate_all(
        has_anger: bool,
        has_fear: bool,
        has_shame: bool,
        mood: f32,
        avg: &DimensionAverages,
    ) -> Vec<Self> {
        let mut restrictions = Vec::new();

        if mood < -MOOD_THRESHOLD {
            restrictions.push(Self::NoHumorOrLightTone);
        }
        if has_anger {
            restrictions.push(Self::NoFriendliness);
        }
        if has_shame {
            restrictions.push(Self::NoSelfJustification);
        }
        if has_fear {
            restrictions.push(Self::NoBravado);
        }
        if avg.h.value() > HONESTY_RESTRICTION_THRESHOLD {
            restrictions.push(Self::NoLyingOrExaggeration);
        }

        restrictions
    }
}
