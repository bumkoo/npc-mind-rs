//! 성격/감정 스냅샷 — 도메인 데이터의 구조화된 요약

use serde::{Deserialize, Serialize};

use super::enums::{PersonalityTrait, SpeechStyle};
use crate::domain::emotion::{EmotionState, EmotionType};
use crate::domain::relationship::Relationship;
use crate::domain::tuning::profile;
use crate::ports::PersonalityProfile;

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
    /// 성격 프로필에서 두드러지는 특성을 추출합니다.
    pub fn from_profile(personality: &impl PersonalityProfile) -> Self {
        let avg = personality.dimension_averages();
        let t = profile().trait_threshold;

        let mut traits = Vec::new();
        let mut styles = Vec::new();

        // H: 정직-겸손성
        if let Some(tr) = PersonalityTrait::evaluate(
            avg.h,
            t,
            PersonalityTrait::HonestAndModest,
            PersonalityTrait::CunningAndAmbitious,
        ) {
            traits.push(tr);
        }
        if let Some(st) = SpeechStyle::evaluate(
            avg.h,
            t,
            SpeechStyle::FrankAndUnadorned,
            SpeechStyle::HidesInnerThoughts,
        ) {
            styles.push(st);
        }

        // E: 정서성
        if let Some(tr) = PersonalityTrait::evaluate(
            avg.e,
            t,
            PersonalityTrait::EmotionalAndAnxious,
            PersonalityTrait::BoldAndIndependent,
        ) {
            traits.push(tr);
        }
        if let Some(st) = SpeechStyle::evaluate(
            avg.e,
            t,
            SpeechStyle::ExpressiveAndWorried,
            SpeechStyle::CalmAndComposed,
        ) {
            styles.push(st);
        }

        // X: 외향성
        if let Some(tr) = PersonalityTrait::evaluate(
            avg.x,
            t,
            PersonalityTrait::ConfidentAndSociable,
            PersonalityTrait::IntrovertedAndQuiet,
        ) {
            traits.push(tr);
        }
        if let Some(st) = SpeechStyle::evaluate(
            avg.x,
            t,
            SpeechStyle::ActiveAndForceful,
            SpeechStyle::BriefAndConcise,
        ) {
            styles.push(st);
        }

        // A: 원만성
        if let Some(tr) = PersonalityTrait::evaluate(
            avg.a,
            t,
            PersonalityTrait::TolerantAndGentle,
            PersonalityTrait::GrudgingAndCritical,
        ) {
            traits.push(tr);
        }
        if let Some(st) = SpeechStyle::evaluate(
            avg.a,
            t,
            SpeechStyle::SoftAndConsiderate,
            SpeechStyle::SharpAndDirect,
        ) {
            styles.push(st);
        }

        // C: 성실성
        if let Some(tr) = PersonalityTrait::evaluate(
            avg.c,
            t,
            PersonalityTrait::SystematicAndDiligent,
            PersonalityTrait::FreeAndImpulsive,
        ) {
            traits.push(tr);
        }
        if let Some(st) = SpeechStyle::evaluate(
            avg.c,
            t,
            SpeechStyle::LogicalAndRational,
            SpeechStyle::UnfilteredAndSpontaneous,
        ) {
            styles.push(st);
        }

        // O: 개방성
        if let Some(tr) = PersonalityTrait::evaluate(
            avg.o,
            t,
            PersonalityTrait::CuriousAndCreative,
            PersonalityTrait::TraditionalAndConservative,
        ) {
            traits.push(tr);
        }
        if let Some(st) = SpeechStyle::evaluate(
            avg.o,
            t,
            SpeechStyle::MetaphoricalAndUnique,
            SpeechStyle::FormalAndTraditional,
        ) {
            styles.push(st);
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
        let dominant = state.dominant().map(|e| EmotionEntry {
            emotion_type: e.emotion_type(),
            intensity: e.intensity(),
            context: e.context().map(|s| s.to_string()),
        });

        let threshold = profile().emotion_threshold;
        let mut active_emotions: Vec<EmotionEntry> = state
            .iter_active()
            .filter(|&(_, i, _)| i >= threshold)
            .map(|(t, i, ctx)| EmotionEntry {
                emotion_type: t,
                intensity: i,
                context: ctx.map(|s| s.to_string()),
            })
            .collect();

        active_emotions.sort_by(|a, b| {
            b.intensity
                .partial_cmp(&a.intensity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

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
/// Score 값(±100 raw)을 라벨 인덱스로 변환하여
/// presentation 레이어에서 다국어 렌더링을 가능하게 한다.
///
/// **Phase 2.3 §A (P-D-4)**: 4축 presentation. `closeness_level → affinity_level`,
/// `respect_level`/`wariness_level` 신설, `power_level` 폐기 (B-D4 — 위계 정보는
/// `Relationship.type_text` 자유 텍스트로 흡수).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSnapshot {
    /// 상대방 이름/ID
    pub target_name: String,
    /// 친화도 라벨 인덱스 (구 closeness_level → affinity_level)
    pub affinity_level: RelationshipLevel,
    /// 신뢰도 라벨 인덱스
    pub trust_level: RelationshipLevel,
    /// 존경도 라벨 인덱스 (Phase 2.3 신설)
    pub respect_level: RelationshipLevel,
    /// 경계심 라벨 인덱스 (Phase 2.3 신설)
    pub wariness_level: RelationshipLevel,
}

/// 관계 강도 수준 (4축 공용)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationshipLevel {
    /// > LEVEL_VERY_HIGH_THRESHOLD: 매우 높음
    VeryHigh,
    /// > LEVEL_HIGH_THRESHOLD: 높음
    High,
    /// > LEVEL_LOW_THRESHOLD: 중립
    Neutral,
    /// > LEVEL_VERY_LOW_THRESHOLD: 낮음
    Low,
    /// <= LEVEL_VERY_LOW_THRESHOLD: 매우 낮음
    VeryLow,
}

impl RelationshipLevel {
    /// 입력은 ±100 raw scale (Phase 2.3 §A: threshold const도 ±100 native).
    pub fn from_score(value: f32) -> Self {
        let p = profile();
        if value > p.level_very_high_threshold {
            Self::VeryHigh
        } else if value > p.level_high_threshold {
            Self::High
        } else if value > p.level_low_threshold {
            Self::Neutral
        } else if value > p.level_very_low_threshold {
            Self::Low
        } else {
            Self::VeryLow
        }
    }
}

impl RelationshipSnapshot {
    /// Relationship에서 스냅샷 생성
    ///
    /// `partner_name`은 표시용 파트너 NPC 이름(Npc::name()). 비어 있으면
    /// `Relationship::target_id()`로 fallback한다.
    pub fn from_relationship(rel: &Relationship, partner_name: &str) -> Self {
        let name = if partner_name.is_empty() {
            rel.target_id().to_string()
        } else {
            partner_name.to_string()
        };
        Self {
            target_name: name,
            // Phase 2.3 §A: ±100 raw 직통 (level_*_threshold도 ±100 native).
            affinity_level: RelationshipLevel::from_score(rel.affinity().value()),
            trust_level: RelationshipLevel::from_score(rel.trust().value()),
            respect_level: RelationshipLevel::from_score(rel.respect().value()),
            wariness_level: RelationshipLevel::from_score(rel.wariness().value()),
        }
    }
}
