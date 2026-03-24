//! 연기 지시문 — 감정 + 성격에서 도출된 구체적 연기 지시

use serde::{Deserialize, Serialize};

use crate::domain::emotion::{EmotionState, EmotionType};
use crate::domain::personality::HexacoProfile;

use super::{EMOTION_THRESHOLD, TRAIT_THRESHOLD, MOOD_THRESHOLD, HONESTY_RESTRICTION_THRESHOLD};
use super::enums::{Tone, Attitude, BehavioralTendency, Restriction};

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
    pub fn from_emotion_and_personality(
        state: &EmotionState,
        profile: &HexacoProfile,
    ) -> Self {
        let avg = profile.dimension_averages();
        let mood = state.overall_valence();
        let dominant = state.dominant();
        let significant = state.significant(EMOTION_THRESHOLD);

        // --- 어조 결정 ---
        let t = TRAIT_THRESHOLD;
        let tone = match dominant.map(|e| e.emotion_type()) {
            Some(EmotionType::Anger) => {
                if avg.c > t { Tone::SuppressedCold }
                else { Tone::RoughAggressive }
            }
            Some(EmotionType::Distress) => {
                if avg.e > t { Tone::AnxiousTrembling }
                else { Tone::SomberRestrained }
            }
            Some(EmotionType::Joy) => Tone::BrightLively,
            Some(EmotionType::Fear) => {
                if avg.e < -t { Tone::VigilantCalm }
                else { Tone::TenseAnxious }
            }
            Some(EmotionType::Shame) => Tone::ShrinkingSmall,
            Some(EmotionType::Pride) => {
                if avg.h > t { Tone::QuietConfidence }
                else { Tone::ProudArrogant }
            }
            Some(EmotionType::Reproach) => Tone::CynicalCritical,
            Some(EmotionType::Disappointment) => Tone::DeepSighing,
            Some(EmotionType::Gratitude) => Tone::SincerelyWarm,
            Some(EmotionType::Resentment) => Tone::JealousBitter,
            Some(EmotionType::Pity) => Tone::CompassionateSoft,
            _ => {
                if mood > EMOTION_THRESHOLD { Tone::RelaxedGentle }
                else if mood < -EMOTION_THRESHOLD { Tone::Heavy }
                else { Tone::Calm }
            }
        };

        // --- 태도 결정 ---
        let attitude = if significant.iter().any(|e| e.emotion_type() == EmotionType::Anger) {
            if avg.a < -t {
                Attitude::HostileAggressive
            } else {
                Attitude::SuppressedDiscomfort
            }
        } else if significant.iter().any(|e| e.emotion_type() == EmotionType::Reproach) {
            Attitude::Judgmental
        } else if significant.iter().any(|e| e.emotion_type() == EmotionType::Fear) {
            Attitude::GuardedDefensive
        } else if mood > MOOD_THRESHOLD {
            Attitude::FriendlyOpen
        } else if mood < -MOOD_THRESHOLD {
            Attitude::DefensiveClosed
        } else {
            Attitude::NeutralObservant
        };

        // --- 행동 경향 결정 ---
        let behavioral_tendency = if significant.iter().any(|e| e.emotion_type() == EmotionType::Anger) {
            if avg.c < -t {
                BehavioralTendency::ImmediateConfrontation
            } else if avg.c > t {
                BehavioralTendency::StrategicResponse
            } else {
                BehavioralTendency::ExpressAndObserve
            }
        } else if significant.iter().any(|e| e.emotion_type() == EmotionType::Fear) {
            if avg.e < -t {
                BehavioralTendency::BraveConfrontation
            } else {
                BehavioralTendency::SeekSafety
            }
        } else if significant.iter().any(|e| e.emotion_type() == EmotionType::Shame) {
            BehavioralTendency::AvoidOrDeflect
        } else if mood > MOOD_THRESHOLD {
            BehavioralTendency::ActiveCooperation
        } else {
            BehavioralTendency::ObserveAndRespond
        };

        // --- 금지 사항 결정 ---
        let mut restrictions = Vec::new();

        if mood < -MOOD_THRESHOLD {
            restrictions.push(Restriction::NoHumorOrLightTone);
        }
        if significant.iter().any(|e| e.emotion_type() == EmotionType::Anger) {
            restrictions.push(Restriction::NoFriendliness);
        }
        if significant.iter().any(|e| e.emotion_type() == EmotionType::Shame) {
            restrictions.push(Restriction::NoSelfJustification);
        }
        if significant.iter().any(|e| e.emotion_type() == EmotionType::Fear) {
            restrictions.push(Restriction::NoBravado);
        }
        if avg.h > HONESTY_RESTRICTION_THRESHOLD {
            restrictions.push(Restriction::NoLyingOrExaggeration);
        }

        Self {
            tone,
            attitude,
            behavioral_tendency,
            restrictions,
        }
    }
}
