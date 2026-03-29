//! 성격/감정 스냅샷 — 도메인 데이터의 구조화된 요약

use serde::{Deserialize, Serialize};

use crate::domain::emotion::{EmotionState, EmotionType};
use crate::domain::personality::HexacoProfile;
use crate::domain::relationship::Relationship;

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
    /// HEXACO 프로필에서 두드러지는 특성을 추출합니다.
    pub fn from_profile(profile: &HexacoProfile) -> Self {
        let avg = profile.dimension_averages();
        let t = TRAIT_THRESHOLD;
        let mut traits = Vec::new();
        let mut styles = Vec::new();

        // 리팩토링: avg의 필드들이 Score 타입이므로 .value()로 비교를 수행합니다.
        // H: 정직-겸손성
        if avg.h.value() > t {
            traits.push(PersonalityTrait::HonestAndModest);
            styles.push(SpeechStyle::FrankAndUnadorned);
        } else if avg.h.value() < -t {
            traits.push(PersonalityTrait::CunningAndAmbitious);
            styles.push(SpeechStyle::HidesInnerThoughts);
        }

        // E: 정서성
        if avg.e.value() > t {
            traits.push(PersonalityTrait::EmotionalAndAnxious);
            styles.push(SpeechStyle::ExpressiveAndWorried);
        } else if avg.e.value() < -t {
            traits.push(PersonalityTrait::BoldAndIndependent);
            styles.push(SpeechStyle::CalmAndComposed);
        }

        // X: 외향성
        if avg.x.value() > t {
            traits.push(PersonalityTrait::ConfidentAndSociable);
            styles.push(SpeechStyle::ActiveAndForceful);
        } else if avg.x.value() < -t {
            traits.push(PersonalityTrait::IntrovertedAndQuiet);
            styles.push(SpeechStyle::BriefAndConcise);
        }

        // A: 원만성
        if avg.a.value() > t {
            traits.push(PersonalityTrait::TolerantAndGentle);
            styles.push(SpeechStyle::SoftAndConsiderate);
        } else if avg.a.value() < -t {
            traits.push(PersonalityTrait::GrudgingAndCritical);
            styles.push(SpeechStyle::SharpAndDirect);
        }

        // C: 성실성
        if avg.c.value() > t {
            traits.push(PersonalityTrait::SystematicAndDiligent);
            styles.push(SpeechStyle::LogicalAndRational);
        } else if avg.c.value() < -t {
            traits.push(PersonalityTrait::FreeAndImpulsive);
            styles.push(SpeechStyle::UnfilteredAndSpontaneous);
        }

        // O: 개방성
        if avg.o.value() > t {
            traits.push(PersonalityTrait::CuriousAndCreative);
            styles.push(SpeechStyle::MetaphoricalAndUnique);
        } else if avg.o.value() < -t {
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
    /// 감정의 원인/맥락 (LLM 프롬프트에 포함됨)
    pub context: Option<String>,
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
    /// EmotionState에서 스냅샷 요약을 생성합니다.
    pub fn from_state(state: &EmotionState) -> Self {
        let dominant = state.dominant()
            .map(|e| EmotionEntry {
                emotion_type: e.emotion_type(),
                intensity: e.intensity(),
                context: e.context().map(|s| s.to_string()),
            });

        let active_emotions = state.significant(EMOTION_THRESHOLD)
            .iter()
            .map(|e| EmotionEntry {
                emotion_type: e.emotion_type(),
                intensity: e.intensity(),
                context: e.context().map(|s| s.to_string()),
            })
            .collect();

        let mood = state.overall_valence();

        Self {
            dominant,
            active_emotions,
            mood,
        }
    }
}


// ---------------------------------------------------------------------------
// 관계 스냅샷
// ---------------------------------------------------------------------------

/// 관계의 구조화된 요약 — 도메인 데이터
///
/// Score 값(-1.0~1.0)을 라벨 인덱스로 변환하여
/// presentation 레이어에서 다국어 렌더링을 가능하게 한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSnapshot {
    /// 상대방 이름/ID
    pub target_name: String,
    /// 친밀도 라벨 인덱스
    pub closeness_level: RelationshipLevel,
    /// 신뢰도 라벨 인덱스
    pub trust_level: RelationshipLevel,
    /// 상하 관계 라벨 인덱스
    pub power_level: PowerLevel,
}

/// 관계 강도 수준 (closeness, trust 공용)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationshipLevel {
    /// > 0.6: 매우 높음
    VeryHigh,
    /// > 0.2: 높음
    High,
    /// > -0.2: 중립
    Neutral,
    /// > -0.6: 낮음
    Low,
    /// <= -0.6: 매우 낮음
    VeryLow,
}

/// 상하 관계 수준
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PowerLevel {
    /// > 0.6: 절대 상위자 (문주, 장문인 등)
    VeryHigh,
    /// > 0.2: 상위자 (사부 등)
    High,
    /// > -0.2: 대등
    Neutral,
    /// > -0.6: 하위자 (제자 등)
    Low,
    /// <= -0.6: 절대 하위자 (하인, 종 등)
    VeryLow,
}

impl RelationshipLevel {
    pub fn from_score(value: f32) -> Self {
        if value > 0.6 { Self::VeryHigh }
        else if value > 0.2 { Self::High }
        else if value > -0.2 { Self::Neutral }
        else if value > -0.6 { Self::Low }
        else { Self::VeryLow }
    }
}

impl PowerLevel {
    pub fn from_score(value: f32) -> Self {
        if value > 0.6 { Self::VeryHigh }
        else if value > 0.2 { Self::High }
        else if value > -0.2 { Self::Neutral }
        else if value > -0.6 { Self::Low }
        else { Self::VeryLow }
    }
}


impl RelationshipSnapshot {
    /// Relationship에서 스냅샷 생성
    pub fn from_relationship(rel: &Relationship) -> Self {
        Self {
            target_name: rel.target_id().to_string(),
            closeness_level: RelationshipLevel::from_score(rel.closeness().value()),
            trust_level: RelationshipLevel::from_score(rel.trust().value()),
            power_level: PowerLevel::from_score(rel.power().value()),
        }
    }
}
