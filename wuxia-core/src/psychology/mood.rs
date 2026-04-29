// wuxia-core/src/psychology/mood.rs
//
// ⑤층: PAD 기분 상태 (PAD Mood State)
// "이 사람의 현재 기분 톤은 어떤가?"
//
// Mehrabian & Russell(1974)의 PAD 모델:
//   P (Pleasure)   — 쾌/불쾌 (-1.0 ~ +1.0)
//   A (Arousal)    — 각성/이완 (-1.0 ~ +1.0)
//   D (Dominance)  — 지배감/무력감 (-1.0 ~ +1.0)
//
// OCC 감정이 발생할 때마다 PAD가 미세 조정되며,
// PAD 상태는 다음 감정 평가의 편향(bias)으로 작용한다.
//
// P = -0.9 (적대적 기분):
//   → 부정 감정 증폭, 긍정 감정 억제
//   → "평소엔 감사할 일인데 지금은 짜증난다"

use serde::{Deserialize, Serialize};

use super::emotion::EmotionType;

// ---------------------------------------------------------------------------
// PadState — PAD 기분 상태
// ---------------------------------------------------------------------------

/// PAD 기분 상태 (Mehrabian & Russell).
///
/// 각 축은 -1.0 ~ +1.0 범위로 클램프된다.
///
/// # Example
/// ```
/// use wuxia_core::psychology::{PadState, EmotionType};
///
/// let mut mood = PadState::neutral();
/// assert_eq!(mood.pleasure(), 0.0);
///
/// let new_mood = mood.with_emotion_applied(EmotionType::Anger, 80.0);
/// assert!(new_mood.pleasure() < 0.0, "분노 → 불쾌");
/// assert!(new_mood.arousal() > 0.0, "분노 → 고각성");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PadState {
    pleasure: f32,
    arousal: f32,
    dominance: f32,
}

fn clamp_pad(v: f32) -> f32 {
    v.clamp(-1.0, 1.0)
}

impl PadState {
    /// 특정 PAD 값으로 생성한다. 각 축은 -1.0~+1.0으로 클램프.
    pub fn new(pleasure: f32, arousal: f32, dominance: f32) -> Self {
        Self {
            pleasure: clamp_pad(pleasure),
            arousal: clamp_pad(arousal),
            dominance: clamp_pad(dominance),
        }
    }

    /// 중립 기분 (0, 0, 0).
    pub fn neutral() -> Self {
        Self {
            pleasure: 0.0,
            arousal: 0.0,
            dominance: 0.0,
        }
    }

    // -- Getters --

    pub fn pleasure(&self) -> f32 {
        self.pleasure
    }
    pub fn arousal(&self) -> f32 {
        self.arousal
    }
    pub fn dominance(&self) -> f32 {
        self.dominance
    }

    // -- 감정 적용 --

    /// 감정을 적용한 새 PadState를 반환한다.
    ///
    /// ΔP_actual = ΔP_max × (intensity / 100)
    pub fn with_emotion_applied(&self, emotion_type: EmotionType, intensity: f32) -> Self {
        let (dp, da, dd) = emotion_type.pad_delta();
        let scale = intensity / 100.0;

        Self::new(
            self.pleasure + dp * scale,
            self.arousal + da * scale,
            self.dominance + dd * scale,
        )
    }

    // -- 자연 감쇠 --

    /// 중립(0,0,0)을 향해 자연 감쇠한 새 PadState를 반환한다.
    ///
    /// rate: 감쇠율 (0.0~1.0). 0.1이면 현재값의 10%만큼 중립 방향으로 이동.
    pub fn with_decay_toward_neutral(&self, rate: f32) -> Self {
        let rate = rate.clamp(0.0, 1.0);
        Self {
            pleasure: self.pleasure * (1.0 - rate),
            arousal: self.arousal * (1.0 - rate),
            dominance: self.dominance * (1.0 - rate),
        }
    }

    // -- 편향 계수 --

    /// 현재 기분이 감정 평가에 미치는 편향 계수.
    ///
    /// P > 0: 긍정 감정 증폭, 부정 감정 억제
    /// P < 0: 부정 감정 증폭, 긍정 감정 억제
    ///
    /// 범위: 0.7 ~ 1.3
    pub fn mood_bias(&self) -> f32 {
        1.0 + self.pleasure * 0.3
    }

    /// PAD 극단 상태 여부 (Tier 3 성찰 트리거 조건).
    ///
    /// |P| > 0.8 또는 |A| > 0.8이면 극단.
    pub fn is_extreme(&self) -> bool {
        self.pleasure.abs() > 0.8 || self.arousal.abs() > 0.8
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "mood_tests.rs"]
mod tests;
