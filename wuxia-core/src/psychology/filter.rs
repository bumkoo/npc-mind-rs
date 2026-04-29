// wuxia-core/src/psychology/filter.rs
//
// HEXACO → 감정 필터 (HEXACO Emotion Filters)
//
// HEXACO 성격 요소가 OCC 감정의 강도를 조절하는 순수 함수.
// 모든 계산은 <1ms 내에 완료된다.
//
// 핵심 공식 (설계 문서 §6.3):
//   H: Guilt    × (1.0 + H × 0.005),  Gloating × (1.0 - H × 0.004)
//   E: Fear     × (1.0 + E × 0.005),  Pity     × (1.0 + E × 0.004)
//   A: Anger    × (1.0 - A × 0.004),  Resentment × (1.0 - A × 0.003)
//
// 예시:
//   명경(H90, A80) → Guilt ×1.45, Anger ×0.68 (도덕 민감 + 분노 억제)
//   조고(H10, A10) → Guilt ×1.05, Anger ×0.96 (도덕 둔감 + 분노 거의 억제 안됨)

use super::emotion::EmotionType;
use super::personality::HexacoPersonality;

// ---------------------------------------------------------------------------
// H 필터 — 도덕 감정 (Moral Emotions)
// ---------------------------------------------------------------------------

/// Guilt/Shame에 대한 H 필터.
/// H가 높을수록 죄책감/수치를 강하게 느낀다.
///
/// 공식: × (1.0 + H × 0.005)
/// H=90 → ×1.45, H=10 → ×1.05
pub fn h_guilt_filter(h: u32) -> f32 {
    1.0 + h as f32 * 0.005
}

/// Gloating에 대한 H 필터.
/// H가 높을수록 통쾌함을 덜 느낀다.
///
/// 공식: × (1.0 - H × 0.004)
/// H=90 → ×0.64, H=10 → ×0.96
pub fn h_gloating_filter(h: u32) -> f32 {
    1.0 - h as f32 * 0.004
}

/// Reproach에 대한 H 필터.
/// H가 높을수록 도덕적 비난을 강하게 느낀다.
///
/// 공식: × (1.0 + H × 0.003)
/// H=90 → ×1.27, H=10 → ×1.03
pub fn h_reproach_filter(h: u32) -> f32 {
    1.0 + h as f32 * 0.003
}

// ---------------------------------------------------------------------------
// E 필터 — 공포/공감 (Fear/Empathy)
// ---------------------------------------------------------------------------

/// Fear에 대한 E 필터.
/// E가 높을수록 두려움을 강하게 느낀다.
///
/// 공식: × (1.0 + E × 0.005)
/// E=80 → ×1.40, E=20 → ×1.10
pub fn e_fear_filter(e: u32) -> f32 {
    1.0 + e as f32 * 0.005
}

/// Pity에 대한 E 필터.
/// E가 높을수록 측은함을 강하게 느낀다.
///
/// 공식: × (1.0 + E × 0.004)
/// E=80 → ×1.32, E=20 → ×1.08
pub fn e_pity_filter(e: u32) -> f32 {
    1.0 + e as f32 * 0.004
}

// ---------------------------------------------------------------------------
// A 필터 — 분노/원한 조절 (Anger/Resentment Regulation)
// ---------------------------------------------------------------------------

/// Anger에 대한 A 필터.
/// A가 높을수록 분노를 억제한다.
///
/// 공식: × (1.0 - A × 0.004)
/// A=80 → ×0.68, A=10 → ×0.96
pub fn a_anger_filter(a: u32) -> f32 {
    1.0 - a as f32 * 0.004
}

/// Resentment에 대한 A 필터.
/// A가 높을수록 시기심을 억제한다.
///
/// 공식: × (1.0 - A × 0.003)
/// A=80 → ×0.76, A=20 → ×0.94
pub fn a_resentment_filter(a: u32) -> f32 {
    1.0 - a as f32 * 0.003
}

// ---------------------------------------------------------------------------
// 통합 필터 — 감정 타입별 HEXACO 필터 계수
// ---------------------------------------------------------------------------

/// 감정 타입과 HEXACO 성격에 따른 필터 계수를 반환한다.
///
/// 필터가 적용되지 않는 감정은 1.0 (변화 없음)을 반환한다.
///
/// # Example
/// ```
/// use wuxia_core::psychology::{EmotionType, HexacoPersonality};
/// use wuxia_core::psychology::filter::hexaco_emotion_filter;
/// use wuxia_core::shared::CharacterId;
///
/// // 명경: H90, E50, A80
/// let personality = HexacoPersonality::new(CharacterId::new(1), 90, 50, 50, 80, 90, 60);
///
/// let guilt_filter = hexaco_emotion_filter(&EmotionType::Shame, &personality);
/// assert!((guilt_filter - 1.45).abs() < 0.01, "H90 → Shame ×1.45");
///
/// let anger_filter = hexaco_emotion_filter(&EmotionType::Anger, &personality);
/// assert!((anger_filter - 0.68).abs() < 0.01, "A80 → Anger ×0.68");
/// ```
pub fn hexaco_emotion_filter(emotion: &EmotionType, personality: &HexacoPersonality) -> f32 {
    match emotion {
        // H 필터: 도덕 감정
        EmotionType::Shame => h_guilt_filter(personality.h()),
        EmotionType::Reproach => h_reproach_filter(personality.h()),
        EmotionType::Gloating => h_gloating_filter(personality.h()),
        // Remorse = Shame + Distress → H 필터 적용
        EmotionType::Remorse => h_guilt_filter(personality.h()),

        // E 필터: 공포/공감
        EmotionType::Fear => e_fear_filter(personality.e()),
        EmotionType::FearsConfirmed => e_fear_filter(personality.e()),
        EmotionType::Pity => e_pity_filter(personality.e()),

        // A 필터: 분노/원한
        EmotionType::Anger => a_anger_filter(personality.a()),
        EmotionType::Resentment => a_resentment_filter(personality.a()),

        // 필터 없는 감정 → 1.0 (변화 없음)
        _ => 1.0,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "filter_tests.rs"]
mod tests;
