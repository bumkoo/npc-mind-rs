use super::*;

// -- 생성 --

#[test]
fn new_normal() {
    let m = PadState::new(0.5, -0.3, 0.1);
    assert_eq!(m.pleasure(), 0.5);
    assert_eq!(m.arousal(), -0.3);
    assert_eq!(m.dominance(), 0.1);
}

#[test]
fn new_clamps() {
    let m = PadState::new(1.5, -2.0, 0.0);
    assert_eq!(m.pleasure(), 1.0);
    assert_eq!(m.arousal(), -1.0);
    assert_eq!(m.dominance(), 0.0);
}

#[test]
fn neutral() {
    let m = PadState::neutral();
    assert_eq!(m.pleasure(), 0.0);
    assert_eq!(m.arousal(), 0.0);
    assert_eq!(m.dominance(), 0.0);
}

// -- with_emotion_applied --

#[test]
fn apply_anger_to_neutral() {
    let m = PadState::neutral();
    let m2 = m.with_emotion_applied(EmotionType::Anger, 100.0);
    // Anger PAD delta: (-0.3, +0.5, +0.2) × (100/100) = (-0.3, +0.5, +0.2)
    assert!((m2.pleasure() - (-0.3)).abs() < 0.001);
    assert!((m2.arousal() - 0.5).abs() < 0.001);
    assert!((m2.dominance() - 0.2).abs() < 0.001);
}

#[test]
fn apply_emotion_scales_with_intensity() {
    let m = PadState::neutral();
    let m2 = m.with_emotion_applied(EmotionType::Joy, 50.0);
    // Joy: (+0.4, +0.2, +0.1) × 0.5 = (+0.2, +0.1, +0.05)
    assert!((m2.pleasure() - 0.2).abs() < 0.001);
    assert!((m2.arousal() - 0.1).abs() < 0.001);
    assert!((m2.dominance() - 0.05).abs() < 0.001);
}

#[test]
fn apply_emotion_clamps_result() {
    let m = PadState::new(0.9, 0.0, 0.0);
    let m2 = m.with_emotion_applied(EmotionType::Joy, 100.0);
    // 0.9 + 0.4 = 1.3 → clamped to 1.0
    assert_eq!(m2.pleasure(), 1.0);
}

#[test]
fn apply_zero_intensity_no_change() {
    let m = PadState::new(0.5, 0.3, -0.2);
    let m2 = m.with_emotion_applied(EmotionType::Anger, 0.0);
    assert_eq!(m.pleasure(), m2.pleasure());
    assert_eq!(m.arousal(), m2.arousal());
    assert_eq!(m.dominance(), m2.dominance());
}

#[test]
fn apply_fear_reduces_dominance() {
    let m = PadState::neutral();
    let m2 = m.with_emotion_applied(EmotionType::Fear, 80.0);
    // Fear: (-0.3, +0.4, -0.4) × 0.8 = (-0.24, +0.32, -0.32)
    assert!(m2.dominance() < 0.0, "두려움 → 무력감");
    assert!(m2.arousal() > 0.0, "두려움 → 고각성");
}

#[test]
fn apply_multiple_emotions_cumulative() {
    let m = PadState::neutral();
    let m2 = m.with_emotion_applied(EmotionType::Anger, 60.0);
    let m3 = m2.with_emotion_applied(EmotionType::Fear, 40.0);
    // Anger: (-0.3×0.6, +0.5×0.6, +0.2×0.6) = (-0.18, +0.30, +0.12)
    // Fear:  (-0.3×0.4, +0.4×0.4, -0.4×0.4) = (-0.12, +0.16, -0.16)
    // Total: (-0.30, +0.46, -0.04)
    assert!(m3.pleasure() < m2.pleasure(), "추가 부정 감정 → P 더 하락");
}

// -- with_decay_toward_neutral --

#[test]
fn decay_reduces_values() {
    let m = PadState::new(0.8, -0.6, 0.4);
    let m2 = m.with_decay_toward_neutral(0.1);
    // 0.8 × 0.9 = 0.72
    assert!((m2.pleasure() - 0.72).abs() < 0.001);
    assert!((m2.arousal() - (-0.54)).abs() < 0.001);
    assert!((m2.dominance() - 0.36).abs() < 0.001);
}

#[test]
fn decay_rate_zero_no_change() {
    let m = PadState::new(0.5, -0.3, 0.1);
    let m2 = m.with_decay_toward_neutral(0.0);
    assert_eq!(m.pleasure(), m2.pleasure());
}

#[test]
fn decay_rate_one_goes_to_neutral() {
    let m = PadState::new(0.8, -0.6, 0.4);
    let m2 = m.with_decay_toward_neutral(1.0);
    assert_eq!(m2.pleasure(), 0.0);
    assert_eq!(m2.arousal(), 0.0);
    assert_eq!(m2.dominance(), 0.0);
}

#[test]
fn decay_rate_clamps() {
    let m = PadState::new(0.5, 0.5, 0.5);
    let m2 = m.with_decay_toward_neutral(2.0); // clamped to 1.0
    assert_eq!(m2.pleasure(), 0.0);
}

// -- mood_bias --

#[test]
fn mood_bias_neutral() {
    let m = PadState::neutral();
    assert!((m.mood_bias() - 1.0).abs() < f32::EPSILON);
}

#[test]
fn mood_bias_positive_pleasure() {
    let m = PadState::new(0.5, 0.0, 0.0);
    // 1.0 + 0.5 × 0.3 = 1.15
    assert!((m.mood_bias() - 1.15).abs() < 0.001);
}

#[test]
fn mood_bias_negative_pleasure() {
    let m = PadState::new(-0.8, 0.0, 0.0);
    // 1.0 + (-0.8) × 0.3 = 0.76
    assert!((m.mood_bias() - 0.76).abs() < 0.001);
}

#[test]
fn mood_bias_range() {
    // P=-1.0 → 0.7, P=+1.0 → 1.3
    let low = PadState::new(-1.0, 0.0, 0.0);
    let high = PadState::new(1.0, 0.0, 0.0);
    assert!((low.mood_bias() - 0.7).abs() < 0.001);
    assert!((high.mood_bias() - 1.3).abs() < 0.001);
}

// -- is_extreme --

#[test]
fn extreme_high_pleasure() {
    let m = PadState::new(0.85, 0.0, 0.0);
    assert!(m.is_extreme());
}

#[test]
fn extreme_high_arousal() {
    let m = PadState::new(0.0, -0.85, 0.0);
    assert!(m.is_extreme());
}

#[test]
fn not_extreme_moderate() {
    let m = PadState::new(0.5, 0.5, 0.5);
    assert!(!m.is_extreme());
}

#[test]
fn not_extreme_high_dominance_only() {
    // D alone doesn't trigger extreme
    let m = PadState::new(0.0, 0.0, 0.95);
    assert!(!m.is_extreme());
}

#[test]
fn extreme_boundary_exactly_0_8() {
    // |P| > 0.8 (not >=), so 0.8 is not extreme
    let m = PadState::new(0.8, 0.0, 0.0);
    assert!(!m.is_extreme());
}

// -- Serialization --

#[test]
fn serialization_roundtrip() {
    let m = PadState::new(0.5, -0.3, 0.1);
    let json = serde_json::to_string(&m).unwrap();
    let restored: PadState = serde_json::from_str(&json).unwrap();
    assert_eq!(m, restored);
}
