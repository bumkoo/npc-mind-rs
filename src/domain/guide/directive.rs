//! 연기 지시문 — 감정 + 성격에서 도출된 구체적 연기 지시

use serde::{Deserialize, Serialize};

use crate::domain::emotion::{EmotionState, EmotionType};
use crate::domain::personality::DimensionAverages;
use crate::domain::tuning::profile;
use crate::ports::PersonalityProfile;

use super::enums::{Attitude, BehavioralTendency, Restriction, Tone};

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
        personality: &impl PersonalityProfile,
    ) -> Self {
        let avg = personality.dimension_averages();
        let mood = state.overall_valence();

        // 판단에 필요한 정보 요약
        let dominant = state.dominant().map(|e| e.emotion_type());
        let threshold = profile().emotion_threshold;
        let has_anger = state.intensity_of(EmotionType::Anger) >= threshold;
        let has_fear = state.intensity_of(EmotionType::Fear) >= threshold;
        let has_shame = state.intensity_of(EmotionType::Shame) >= threshold;
        let has_reproach = state.intensity_of(EmotionType::Reproach) >= threshold;

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
// 각 요소별 의사결정 로직
// ---------------------------------------------------------------------------

impl Tone {
    pub fn decide(dominant: Option<EmotionType>, mood: f32, avg: &DimensionAverages) -> Self {
        if let Some(etype) = dominant {
            Self::decide_from_dominant(etype, avg)
        } else {
            Self::decide_from_mood(mood)
        }
    }

    fn decide_from_dominant(etype: EmotionType, avg: &DimensionAverages) -> Self {
        let t = profile().trait_threshold;
        match etype {
            EmotionType::Anger => {
                if avg.c.value() > t {
                    Self::SuppressedCold
                } else {
                    Self::RoughAggressive
                }
            }
            EmotionType::Distress => {
                if avg.e.value() > t {
                    Self::AnxiousTrembling
                } else {
                    Self::SomberRestrained
                }
            }
            EmotionType::Joy => Self::BrightLively,
            EmotionType::Fear => {
                if avg.e.value() < -t {
                    Self::VigilantCalm
                } else {
                    Self::TenseAnxious
                }
            }
            EmotionType::Shame => Self::ShrinkingSmall,
            EmotionType::Pride => {
                if avg.h.value() > t {
                    Self::QuietConfidence
                } else {
                    Self::ProudArrogant
                }
            }
            EmotionType::Reproach => Self::CynicalCritical,
            EmotionType::Disappointment => Self::DeepSighing,
            EmotionType::Gratitude => Self::SincerelyWarm,
            EmotionType::Resentment => Self::JealousBitter,
            EmotionType::Pity => Self::CompassionateSoft,
            _ => Self::Calm,
        }
    }

    fn decide_from_mood(mood: f32) -> Self {
        let threshold = profile().emotion_threshold;
        if mood > threshold {
            Self::RelaxedGentle
        } else if mood < -threshold {
            Self::Heavy
        } else {
            Self::Calm
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
        if has_anger {
            Self::decide_for_anger(avg)
        } else if has_reproach {
            Self::Judgmental
        } else if has_fear {
            Self::GuardedDefensive
        } else {
            Self::decide_from_mood(mood)
        }
    }

    fn decide_for_anger(avg: &DimensionAverages) -> Self {
        if avg.a.value() < -profile().trait_threshold {
            Self::HostileAggressive
        } else {
            Self::SuppressedDiscomfort
        }
    }

    fn decide_from_mood(mood: f32) -> Self {
        let threshold = profile().mood_threshold;
        if mood > threshold {
            Self::FriendlyOpen
        } else if mood < -threshold {
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
        if has_anger {
            Self::decide_for_anger(avg)
        } else if has_fear {
            Self::decide_for_fear(avg)
        } else if has_shame {
            Self::AvoidOrDeflect
        } else if mood > profile().mood_threshold {
            Self::ActiveCooperation
        } else {
            Self::ObserveAndRespond
        }
    }

    fn decide_for_anger(avg: &DimensionAverages) -> Self {
        let t = profile().trait_threshold;
        if avg.c.value() < -t {
            Self::ImmediateConfrontation
        } else if avg.c.value() > t {
            Self::StrategicResponse
        } else {
            Self::ExpressAndObserve
        }
    }

    fn decide_for_fear(avg: &DimensionAverages) -> Self {
        if avg.e.value() < -profile().trait_threshold {
            Self::BraveConfrontation
        } else {
            Self::SeekSafety
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
        let p = profile();
        let mut restrictions = Vec::new();

        if mood < -p.mood_threshold {
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
        if avg.h.value() > p.honesty_restriction_threshold {
            restrictions.push(Self::NoLyingOrExaggeration);
        }

        restrictions
    }
}
