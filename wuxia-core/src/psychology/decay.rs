// wuxia-core/src/psychology/decay.rs
//
// 감정 감쇠 (Emotion Decay)
//
// 반감기 기반 지수 감쇠 공식:
//   intensity(t) = intensity₀ × e^(-λ × Δt)
//   where λ = ln(2) / half_life
//
// Love/Hate는 반감기가 무한(∞)이므로 감쇠하지 않는다.
// 임계값(기본 1.0) 이하의 감정은 소멸 처리한다.

use super::emotion::ActiveEmotion;

/// 기본 소멸 임계값.
pub const DEFAULT_EXPIRY_THRESHOLD: f32 = 1.0;

/// 감정 강도를 감쇠시킨다.
///
/// # Arguments
/// - `intensity`: 현재 강도 (0.0~100.0)
/// - `half_life_hours`: 반감기 (게임 시간, 시간 단위). 무한이면 감쇠 없음.
/// - `elapsed_hours`: 경과 시간 (게임 시간, 시간 단위)
///
/// # Returns
/// 감쇠 후 강도 (0.0 이상)
///
/// # Example
/// ```
/// use wuxia_core::psychology::decay::decay_emotion;
///
/// // 강도 80, 반감기 6시간, 6시간 경과 → 약 40
/// let result = decay_emotion(80.0, 6.0, 6.0);
/// assert!((result - 40.0).abs() < 0.1);
///
/// // 강도 80, 반감기 6시간, 12시간 경과 → 약 20
/// let result2 = decay_emotion(80.0, 6.0, 12.0);
/// assert!((result2 - 20.0).abs() < 0.1);
/// ```
pub fn decay_emotion(intensity: f32, half_life_hours: f32, elapsed_hours: f32) -> f32 {
    if half_life_hours.is_infinite() || half_life_hours <= 0.0 || elapsed_hours <= 0.0 {
        return intensity;
    }

    let lambda = f32::ln(2.0) / half_life_hours;
    let result = intensity * f32::exp(-lambda * elapsed_hours);
    result.max(0.0)
}

/// 만료된 감정을 목록에서 제거한다.
///
/// threshold 미만의 intensity를 가진 감정을 제거하고,
/// 제거된 감정 타입 목록을 반환한다.
pub fn cleanup_expired(
    emotions: &mut Vec<ActiveEmotion>,
    threshold: f32,
) -> Vec<super::emotion::EmotionType> {
    let mut expired = Vec::new();
    emotions.retain(|e| {
        if e.is_expired(threshold) {
            expired.push(e.emotion_type());
            false
        } else {
            true
        }
    });
    expired
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psychology::emotion::{ActiveEmotion, EmotionType};
    use crate::shared::time::GameTime;

    // -- decay_emotion --

    #[test]
    fn one_half_life() {
        // 반감기 경과 → 50%
        let result = decay_emotion(100.0, 6.0, 6.0);
        assert!((result - 50.0).abs() < 0.1);
    }

    #[test]
    fn two_half_lives() {
        // 반감기 × 2 → 25%
        let result = decay_emotion(100.0, 6.0, 12.0);
        assert!((result - 25.0).abs() < 0.1);
    }

    #[test]
    fn three_half_lives() {
        // 반감기 × 3 → 12.5%
        let result = decay_emotion(100.0, 6.0, 18.0);
        assert!((result - 12.5).abs() < 0.5);
    }

    #[test]
    fn zero_elapsed() {
        let result = decay_emotion(80.0, 6.0, 0.0);
        assert_eq!(result, 80.0);
    }

    #[test]
    fn infinite_half_life_no_decay() {
        let result = decay_emotion(80.0, f32::INFINITY, 1000.0);
        assert_eq!(result, 80.0);
    }

    #[test]
    fn zero_intensity() {
        let result = decay_emotion(0.0, 6.0, 6.0);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn very_long_time_approaches_zero() {
        let result = decay_emotion(100.0, 6.0, 600.0); // 100 half-lives
        assert!(result < 0.001);
    }

    #[test]
    fn negative_half_life_no_decay() {
        // 방어: 음수 반감기 → 감쇠 없음
        let result = decay_emotion(80.0, -5.0, 6.0);
        assert_eq!(result, 80.0);
    }

    #[test]
    fn anger_24h_half_life() {
        // Anger: 반감기 24시간
        let result = decay_emotion(80.0, 24.0, 24.0);
        assert!((result - 40.0).abs() < 0.1);
    }

    #[test]
    fn relief_2h_fast_decay() {
        // Relief: 반감기 2시간, 6시간 경과 → 3반감기 → 12.5%
        let result = decay_emotion(80.0, 2.0, 6.0);
        assert!((result - 10.0).abs() < 0.5);
    }

    // -- cleanup_expired --

    #[test]
    fn cleanup_removes_expired() {
        let mut emotions = vec![
            ActiveEmotion::new(EmotionType::Joy, 50.0, "a".to_string(), None, GameTime::new(1200, 1, 1)),
            ActiveEmotion::new(EmotionType::Fear, 0.5, "b".to_string(), None, GameTime::new(1200, 1, 1)),
            ActiveEmotion::new(EmotionType::Anger, 80.0, "c".to_string(), None, GameTime::new(1200, 1, 1)),
        ];
        let expired = cleanup_expired(&mut emotions, DEFAULT_EXPIRY_THRESHOLD);
        assert_eq!(emotions.len(), 2);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], EmotionType::Fear);
    }

    #[test]
    fn cleanup_none_expired() {
        let mut emotions = vec![
            ActiveEmotion::new(EmotionType::Joy, 50.0, "a".to_string(), None, GameTime::new(1200, 1, 1)),
        ];
        let expired = cleanup_expired(&mut emotions, DEFAULT_EXPIRY_THRESHOLD);
        assert!(expired.is_empty());
        assert_eq!(emotions.len(), 1);
    }

    #[test]
    fn cleanup_all_expired() {
        let mut emotions = vec![
            ActiveEmotion::new(EmotionType::Relief, 0.3, "a".to_string(), None, GameTime::new(1200, 1, 1)),
            ActiveEmotion::new(EmotionType::HappyFor, 0.1, "b".to_string(), None, GameTime::new(1200, 1, 1)),
        ];
        let expired = cleanup_expired(&mut emotions, DEFAULT_EXPIRY_THRESHOLD);
        assert!(emotions.is_empty());
        assert_eq!(expired.len(), 2);
    }

    #[test]
    fn cleanup_empty() {
        let mut emotions: Vec<ActiveEmotion> = vec![];
        let expired = cleanup_expired(&mut emotions, DEFAULT_EXPIRY_THRESHOLD);
        assert!(expired.is_empty());
    }
}
