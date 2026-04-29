// wuxia-core/src/psychology/emotion.rs
//
// ④층: OCC 감정 (OCC Emotions)
// "이 사건/행동/대상에 대해 어떤 감정을 느끼는가?"
//
// OCC 모델(Ortony, Clore & Collins 1988)을 무협 세계관에 맞게 적응.
// 22가지 감정 타입과 각 감정의 속성(범주, 극성, PAD 매핑, 반감기)을 정의한다.
//
// 3가지 평가 대상:
//   Event Consequence — 사건 결과 (목표 관련성)
//   Agent Action     — 행위자 행동 (기준 부합)
//   Object Aspect    — 대상 속성 (호감)

use serde::{Deserialize, Serialize};

use crate::shared::id::CharacterId;
use crate::shared::time::GameTime;

// ---------------------------------------------------------------------------
// EmotionType — 22가지 OCC 감정
// ---------------------------------------------------------------------------

/// OCC 모델의 22가지 감정 타입.
///
/// # Example
/// ```
/// use wuxia_core::psychology::EmotionType;
///
/// let emotion = EmotionType::Anger;
/// assert_eq!(emotion.category(), wuxia_core::psychology::EmotionCategory::Compound);
/// assert_eq!(emotion.valence(), wuxia_core::psychology::Valence::Negative);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmotionType {
    // -- Event Consequence: Well-being --
    /// 희열(喜悅) — 바람직한 사건 확정
    Joy,
    /// 고뇌(苦惱) — 바람직하지 않은 사건 확정
    Distress,

    // -- Event Consequence: Prospect-based --
    /// 기대(期待) — 바람직한 사건 전망
    Hope,
    /// 두려움(懼) — 바람직하지 않은 사건 전망
    Fear,
    /// 흡족(快) — 기대했던 바람직한 사건 확인
    Satisfaction,
    /// 절망(絕望) — 두려웠던 바람직하지 않은 사건 확인
    FearsConfirmed,
    /// 안도(安堵) — 두려웠던 사건이 일어나지 않음
    Relief,
    /// 실망(失望) — 기대했던 사건이 일어나지 않음
    Disappointment,

    // -- Event Consequence: Fortunes-of-others --
    /// 축하(慶) — 타인에게 바람직한 사건 (호감 대상)
    HappyFor,
    /// 측은(惻隱) — 타인에게 바람직하지 않은 사건 (호감 대상)
    Pity,
    /// 통쾌(痛快) — 적에게 바람직하지 않은 사건
    Gloating,
    /// 시기(忌) — 적에게 바람직한 사건
    Resentment,

    // -- Agent Action --
    /// 자부(自負) — 자신의 칭찬할 행동
    Pride,
    /// 수치(恥) — 자신의 비난할 행동
    Shame,
    /// 감탄(歎) — 타인의 칭찬할 행동
    Admiration,
    /// 비난(責) — 타인의 비난할 행동
    Reproach,

    // -- Compound (Agent + Event) --
    /// 뿌듯함(得意) — Pride + Joy (자기 칭찬할 행동이 좋은 결과)
    Gratification,
    /// 회한(悔恨) — Shame + Distress (자기 비난할 행동이 나쁜 결과)
    Remorse,
    /// 감은(感恩) — Admiration + Joy (타인 칭찬할 행동이 좋은 결과)
    Gratitude,
    /// 분노(憤怒) — Reproach + Distress (타인 비난할 행동이 나쁜 결과)
    Anger,

    // -- Object Aspect --
    /// 애착(愛) — 호감 대상
    Love,
    /// 혐오(厭) — 비호감 대상
    Hate,
}

/// 감정 범주.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmotionCategory {
    /// 사건의 결과에 대한 감정
    EventConsequence,
    /// 행위자의 행동에 대한 감정
    AgentAction,
    /// 대상의 속성에 대한 감정
    ObjectAspect,
    /// 복합 감정 (행동 + 사건)
    Compound,
}

/// 감정 극성.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Valence {
    Positive,
    Negative,
}

impl EmotionType {
    /// 모든 감정 타입 배열 (22종).
    pub const ALL: [EmotionType; 22] = [
        EmotionType::Joy,
        EmotionType::Distress,
        EmotionType::Hope,
        EmotionType::Fear,
        EmotionType::Satisfaction,
        EmotionType::FearsConfirmed,
        EmotionType::Relief,
        EmotionType::Disappointment,
        EmotionType::HappyFor,
        EmotionType::Pity,
        EmotionType::Gloating,
        EmotionType::Resentment,
        EmotionType::Pride,
        EmotionType::Shame,
        EmotionType::Admiration,
        EmotionType::Reproach,
        EmotionType::Gratification,
        EmotionType::Remorse,
        EmotionType::Gratitude,
        EmotionType::Anger,
        EmotionType::Love,
        EmotionType::Hate,
    ];

    /// 감정의 범주를 반환한다.
    pub fn category(&self) -> EmotionCategory {
        match self {
            EmotionType::Joy
            | EmotionType::Distress
            | EmotionType::Hope
            | EmotionType::Fear
            | EmotionType::Satisfaction
            | EmotionType::FearsConfirmed
            | EmotionType::Relief
            | EmotionType::Disappointment
            | EmotionType::HappyFor
            | EmotionType::Pity
            | EmotionType::Gloating
            | EmotionType::Resentment => EmotionCategory::EventConsequence,

            EmotionType::Pride
            | EmotionType::Shame
            | EmotionType::Admiration
            | EmotionType::Reproach => EmotionCategory::AgentAction,

            EmotionType::Gratification
            | EmotionType::Remorse
            | EmotionType::Gratitude
            | EmotionType::Anger => EmotionCategory::Compound,

            EmotionType::Love | EmotionType::Hate => EmotionCategory::ObjectAspect,
        }
    }

    /// 감정의 극성을 반환한다.
    pub fn valence(&self) -> Valence {
        match self {
            EmotionType::Joy
            | EmotionType::Hope
            | EmotionType::Satisfaction
            | EmotionType::Relief
            | EmotionType::HappyFor
            | EmotionType::Gloating
            | EmotionType::Pride
            | EmotionType::Admiration
            | EmotionType::Gratification
            | EmotionType::Gratitude
            | EmotionType::Love => Valence::Positive,

            EmotionType::Distress
            | EmotionType::Fear
            | EmotionType::FearsConfirmed
            | EmotionType::Disappointment
            | EmotionType::Pity
            | EmotionType::Resentment
            | EmotionType::Shame
            | EmotionType::Reproach
            | EmotionType::Remorse
            | EmotionType::Anger
            | EmotionType::Hate => Valence::Negative,
        }
    }

    /// 반감기(게임 시간, 시간 단위)를 반환한다.
    ///
    /// Love/Hate는 무한 반감기 (감쇠 없음) — f32::INFINITY 반환.
    pub fn half_life_hours(&self) -> f32 {
        match self {
            EmotionType::Relief => 2.0,
            EmotionType::Satisfaction | EmotionType::Gloating | EmotionType::HappyFor => 3.0,
            EmotionType::Admiration => 4.0,
            EmotionType::Joy | EmotionType::Pride => 6.0,
            EmotionType::Distress
            | EmotionType::Gratification
            | EmotionType::Disappointment
            | EmotionType::Gratitude => 8.0,
            EmotionType::Hope | EmotionType::Fear => 12.0,
            EmotionType::Anger
            | EmotionType::Shame
            | EmotionType::Resentment
            | EmotionType::Reproach => 24.0,
            EmotionType::FearsConfirmed | EmotionType::Pity => 36.0,
            EmotionType::Remorse => 48.0,
            EmotionType::Love | EmotionType::Hate => f32::INFINITY,
        }
    }

    /// PAD 변화량 최대값을 반환한다: (ΔPleasure, ΔArousal, ΔDominance).
    ///
    /// 실제 적용: ΔP_actual = ΔP_max × (intensity / 100)
    pub fn pad_delta(&self) -> (f32, f32, f32) {
        match self {
            EmotionType::Joy => (0.4, 0.2, 0.1),
            EmotionType::Distress => (-0.4, 0.2, -0.2),
            EmotionType::Hope => (0.2, 0.1, 0.1),
            EmotionType::Fear => (-0.3, 0.4, -0.4),
            EmotionType::Satisfaction => (0.3, -0.1, 0.2),
            EmotionType::FearsConfirmed => (-0.5, 0.3, -0.5),
            EmotionType::Relief => (0.3, -0.3, 0.1),
            EmotionType::Disappointment => (-0.3, -0.1, -0.2),
            EmotionType::HappyFor => (0.2, 0.1, 0.1),
            EmotionType::Pity => (-0.2, 0.1, -0.1),
            EmotionType::Gloating => (0.3, 0.3, 0.3),
            EmotionType::Resentment => (-0.2, 0.2, -0.3),
            EmotionType::Pride => (0.3, 0.2, 0.3),
            EmotionType::Shame => (-0.3, 0.1, -0.4),
            EmotionType::Admiration => (0.2, 0.1, -0.1),
            EmotionType::Reproach => (-0.2, 0.2, 0.1),
            EmotionType::Gratification => (0.4, 0.2, 0.4),
            EmotionType::Remorse => (-0.4, 0.1, -0.4),
            EmotionType::Gratitude => (0.3, 0.1, -0.1),
            EmotionType::Anger => (-0.3, 0.5, 0.2),
            EmotionType::Love => (0.3, 0.1, 0.1),
            EmotionType::Hate => (-0.3, 0.2, 0.1),
        }
    }
}

// ---------------------------------------------------------------------------
// ActiveEmotion — 현재 활성 감정
// ---------------------------------------------------------------------------

/// 현재 활성 상태인 감정.
///
/// 시간 경과에 따라 intensity가 감쇠하며,
/// 임계값(기본 1.0) 이하가 되면 소멸한다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveEmotion {
    emotion_type: EmotionType,
    intensity: f32,
    source_description: String,
    source_agent: Option<CharacterId>,
    created_at: GameTime,
}

impl ActiveEmotion {
    /// 새 활성 감정을 생성한다. intensity는 0.0~100.0으로 클램프.
    pub fn new(
        emotion_type: EmotionType,
        intensity: f32,
        source_description: String,
        source_agent: Option<CharacterId>,
        created_at: GameTime,
    ) -> Self {
        Self {
            emotion_type,
            intensity: intensity.clamp(0.0, 100.0),
            source_description,
            source_agent,
            created_at,
        }
    }

    pub fn emotion_type(&self) -> EmotionType {
        self.emotion_type
    }
    pub fn intensity(&self) -> f32 {
        self.intensity
    }
    pub fn source_description(&self) -> &str {
        &self.source_description
    }
    pub fn source_agent(&self) -> Option<CharacterId> {
        self.source_agent
    }
    pub fn created_at(&self) -> GameTime {
        self.created_at
    }

    /// intensity를 직접 설정한다 (감쇠 적용 시 사용).
    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity.clamp(0.0, 100.0);
    }

    /// 임계값 이하인지 확인한다.
    pub fn is_expired(&self, threshold: f32) -> bool {
        self.intensity < threshold
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "emotion_tests.rs"]
mod tests;
