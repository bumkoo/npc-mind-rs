//! 성격/감정 스냅샷 — 도메인 데이터의 구조화된 요약

use serde::{Deserialize, Serialize};

use crate::domain::emotion::{EmotionState, EmotionType};
use crate::domain::personality::HexacoProfile;

use super::{EMOTION_THRESHOLD, TRAIT_THRESHOLD};
use super::enums::{PersonalityTrait, SpeechStyle};

// ---------------------------------------------------------------------------
// 성격 스냅샷
// ---------------------------------------------------------------------------

/// HEXACO 성격의 구조화된 요약 — 도메인 데이터
///
/// 한국어 텍스트 렌더링은 presentation 레이어가 담당한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalitySnapshot {
    /// 핵심 성격 특성 목록
    pub traits: Vec<PersonalityTrait>,
    /// 대화 스타일 목록
    pub speech_styles: Vec<SpeechStyle>,
}

impl PersonalitySnapshot {
    /// HEXACO 프로필에서 두드러지는 특성을 추출
    pub fn from_profile(profile: &HexacoProfile) -> Self {
        let avg = profile.dimension_averages();
        let t = TRAIT_THRESHOLD;
        let mut traits = Vec::new();
        let mut styles = Vec::new();

        // H: 정직-겸손성
        if avg.h > t {
            traits.push(PersonalityTrait::HonestAndModest);
            styles.push(SpeechStyle::FrankAndUnadorned);
        } else if avg.h < -t {
            traits.push(PersonalityTrait::CunningAndAmbitious);
            styles.push(SpeechStyle::HidesInnerThoughts);
        }

        // E: 정서성
        if avg.e > t {
            traits.push(PersonalityTrait::EmotionalAndAnxious);
            styles.push(SpeechStyle::ExpressiveAndWorried);
        } else if avg.e < -t {
            traits.push(PersonalityTrait::BoldAndIndependent);
            styles.push(SpeechStyle::CalmAndComposed);
        }

        // X: 외향성
        if avg.x > t {
            traits.push(PersonalityTrait::ConfidentAndSociable);
            styles.push(SpeechStyle::ActiveAndForceful);
        } else if avg.x < -t {
            traits.push(PersonalityTrait::IntrovertedAndQuiet);
            styles.push(SpeechStyle::BriefAndConcise);
        }

        // A: 원만성
        if avg.a > t {
            traits.push(PersonalityTrait::TolerantAndGentle);
            styles.push(SpeechStyle::SoftAndConsiderate);
        } else if avg.a < -t {
            traits.push(PersonalityTrait::GrudgingAndCritical);
            styles.push(SpeechStyle::SharpAndDirect);
        }

        // C: 성실성
        if avg.c > t {
            traits.push(PersonalityTrait::SystematicAndDiligent);
            styles.push(SpeechStyle::LogicalAndRational);
        } else if avg.c < -t {
            traits.push(PersonalityTrait::FreeAndImpulsive);
            styles.push(SpeechStyle::UnfilteredAndSpontaneous);
        }

        // O: 개방성
        if avg.o > t {
            traits.push(PersonalityTrait::CuriousAndCreative);
            styles.push(SpeechStyle::MetaphoricalAndUnique);
        } else if avg.o < -t {
            traits.push(PersonalityTrait::TraditionalAndConservative);
            styles.push(SpeechStyle::FormalAndTraditional);
        }

        Self {
            traits,
            speech_styles: styles,
        }
    }
}

// ---------------------------------------------------------------------------
// 감정 항목 (감정 유형 + 강도)
// ---------------------------------------------------------------------------

/// 감정 유형과 강도의 명명된 쌍
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionEntry {
    /// 감정 유형
    pub emotion_type: EmotionType,
    /// 감정 강도 (0.0 ~ 1.0)
    pub intensity: f32,
}

// ---------------------------------------------------------------------------
// 감정 스냅샷
// ---------------------------------------------------------------------------

/// 현재 감정 상태의 구조화된 요약 — 도메인 데이터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionSnapshot {
    /// 지배 감정 (가장 강한 감정)
    pub dominant: Option<EmotionEntry>,
    /// 유의미한 감정 목록 (강도 내림차순)
    pub active_emotions: Vec<EmotionEntry>,
    /// 전체 분위기 (-1.0=매우 부정, +1.0=매우 긍정)
    pub mood: f32,
}

impl EmotionSnapshot {
    /// EmotionState에서 스냅샷 생성
    pub fn from_state(state: &EmotionState) -> Self {
        let dominant = state.dominant()
            .map(|e| EmotionEntry { emotion_type: e.emotion_type(), intensity: e.intensity() });

        let active_emotions = state.significant(EMOTION_THRESHOLD)
            .iter()
            .map(|e| EmotionEntry { emotion_type: e.emotion_type(), intensity: e.intensity() })
            .collect();

        let mood = state.overall_valence();

        Self {
            dominant,
            active_emotions,
            mood,
        }
    }
}
