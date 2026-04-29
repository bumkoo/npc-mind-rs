use super::*;
use crate::shared::id::CharacterId;

fn cid() -> CharacterId {
    CharacterId::new(1)
}

// -- H 필터: 도덕 감정 --

#[test]
fn h_guilt_filter_h90() {
    // H=90 → ×(1.0 + 90×0.005) = ×1.45
    let f = h_guilt_filter(90);
    assert!((f - 1.45).abs() < 0.001);
}

#[test]
fn h_guilt_filter_h10() {
    // H=10 → ×(1.0 + 10×0.005) = ×1.05
    let f = h_guilt_filter(10);
    assert!((f - 1.05).abs() < 0.001);
}

#[test]
fn h_guilt_filter_h0() {
    // H=0 → ×1.0
    assert!((h_guilt_filter(0) - 1.0).abs() < f32::EPSILON);
}

#[test]
fn h_guilt_filter_h100() {
    // H=100 → ×1.5
    assert!((h_guilt_filter(100) - 1.5).abs() < 0.001);
}

#[test]
fn h_gloating_filter_h90() {
    // H=90 → ×(1.0 - 90×0.004) = ×0.64
    let f = h_gloating_filter(90);
    assert!((f - 0.64).abs() < 0.001);
}

#[test]
fn h_gloating_filter_h10() {
    // H=10 → ×(1.0 - 10×0.004) = ×0.96
    let f = h_gloating_filter(10);
    assert!((f - 0.96).abs() < 0.001);
}

#[test]
fn h_reproach_filter_h90() {
    // H=90 → ×(1.0 + 90×0.003) = ×1.27
    let f = h_reproach_filter(90);
    assert!((f - 1.27).abs() < 0.001);
}

#[test]
fn h_reproach_filter_h10() {
    // H=10 → ×(1.0 + 10×0.003) = ×1.03
    let f = h_reproach_filter(10);
    assert!((f - 1.03).abs() < 0.001);
}

// -- E 필터: 공포/공감 --

#[test]
fn e_fear_filter_e80() {
    // E=80 → ×(1.0 + 80×0.005) = ×1.40
    let f = e_fear_filter(80);
    assert!((f - 1.40).abs() < 0.001);
}

#[test]
fn e_fear_filter_e20() {
    // E=20 → ×(1.0 + 20×0.005) = ×1.10
    let f = e_fear_filter(20);
    assert!((f - 1.10).abs() < 0.001);
}

#[test]
fn e_pity_filter_e80() {
    // E=80 → ×(1.0 + 80×0.004) = ×1.32
    let f = e_pity_filter(80);
    assert!((f - 1.32).abs() < 0.001);
}

#[test]
fn e_pity_filter_e20() {
    // E=20 → ×(1.0 + 20×0.004) = ×1.08
    let f = e_pity_filter(20);
    assert!((f - 1.08).abs() < 0.001);
}

// -- A 필터: 분노/원한 --

#[test]
fn a_anger_filter_a80() {
    // A=80 → ×(1.0 - 80×0.004) = ×0.68
    let f = a_anger_filter(80);
    assert!((f - 0.68).abs() < 0.001);
}

#[test]
fn a_anger_filter_a10() {
    // A=10 → ×(1.0 - 10×0.004) = ×0.96
    let f = a_anger_filter(10);
    assert!((f - 0.96).abs() < 0.001);
}

#[test]
fn a_resentment_filter_a80() {
    // A=80 → ×(1.0 - 80×0.003) = ×0.76
    let f = a_resentment_filter(80);
    assert!((f - 0.76).abs() < 0.001);
}

#[test]
fn a_resentment_filter_a20() {
    // A=20 → ×(1.0 - 20×0.003) = ×0.94
    let f = a_resentment_filter(20);
    assert!((f - 0.94).abs() < 0.001);
}

// -- 통합 필터 --

#[test]
fn hexaco_filter_shame_uses_h() {
    let p = HexacoPersonality::new(cid(), 90, 50, 50, 50, 50, 50);
    let f = hexaco_emotion_filter(&EmotionType::Shame, &p);
    assert!((f - 1.45).abs() < 0.001);
}

#[test]
fn hexaco_filter_anger_uses_a() {
    let p = HexacoPersonality::new(cid(), 50, 50, 50, 80, 50, 50);
    let f = hexaco_emotion_filter(&EmotionType::Anger, &p);
    assert!((f - 0.68).abs() < 0.001);
}

#[test]
fn hexaco_filter_fear_uses_e() {
    let p = HexacoPersonality::new(cid(), 50, 80, 50, 50, 50, 50);
    let f = hexaco_emotion_filter(&EmotionType::Fear, &p);
    assert!((f - 1.40).abs() < 0.001);
}

#[test]
fn hexaco_filter_joy_returns_1() {
    // Joy에는 HEXACO 필터가 없다
    let p = HexacoPersonality::new(cid(), 90, 90, 90, 90, 90, 90);
    let f = hexaco_emotion_filter(&EmotionType::Joy, &p);
    assert!((f - 1.0).abs() < f32::EPSILON);
}

#[test]
fn hexaco_filter_remorse_uses_h() {
    // Remorse = Shame + Distress → H 필터 적용
    let p = HexacoPersonality::new(cid(), 90, 50, 50, 50, 50, 50);
    let f = hexaco_emotion_filter(&EmotionType::Remorse, &p);
    assert!((f - 1.45).abs() < 0.001);
}

#[test]
fn hexaco_filter_fears_confirmed_uses_e() {
    let p = HexacoPersonality::new(cid(), 50, 80, 50, 50, 50, 50);
    let f = hexaco_emotion_filter(&EmotionType::FearsConfirmed, &p);
    assert!((f - 1.40).abs() < 0.001);
}

// -- 무협 시나리오 --

#[test]
fn scenario_myungkyung_moral_sensitivity() {
    // 명경: H90, E50, X50, A80, C90, O60
    // 도덕 위반 시 → Shame 강화(×1.45), Anger 억제(×0.68)
    let p = HexacoPersonality::new(cid(), 90, 50, 50, 80, 90, 60);

    let shame_f = hexaco_emotion_filter(&EmotionType::Shame, &p);
    let anger_f = hexaco_emotion_filter(&EmotionType::Anger, &p);
    let reproach_f = hexaco_emotion_filter(&EmotionType::Reproach, &p);

    assert!(shame_f > 1.4, "명경: 수치 강화");
    assert!(anger_f < 0.7, "명경: 분노 억제");
    assert!(reproach_f > 1.2, "명경: 비난 강화");
}

#[test]
fn scenario_jogo_amoral_fearless() {
    // 조고: H10, E20, X80, A10, C80, O50
    // → Shame 미미(×1.05), Anger 거의 억제 안됨(×0.96), Fear 낮음(×1.10)
    let p = HexacoPersonality::new(cid(), 10, 20, 80, 10, 80, 50);

    let shame_f = hexaco_emotion_filter(&EmotionType::Shame, &p);
    let anger_f = hexaco_emotion_filter(&EmotionType::Anger, &p);
    let fear_f = hexaco_emotion_filter(&EmotionType::Fear, &p);

    assert!(shame_f < 1.1, "조고: 수치 둔감");
    assert!(anger_f > 0.95, "조고: 분노 거의 억제 안됨");
    assert!(fear_f < 1.15, "조고: 두려움 낮음");
}

#[test]
fn scenario_soyeon_balanced_filters() {
    // 소연: H50, E60, X60, A40, C70, O70
    let p = HexacoPersonality::new(cid(), 50, 60, 60, 40, 70, 70);

    let shame_f = hexaco_emotion_filter(&EmotionType::Shame, &p);
    let anger_f = hexaco_emotion_filter(&EmotionType::Anger, &p);
    let fear_f = hexaco_emotion_filter(&EmotionType::Fear, &p);

    // H50 → Shame ×1.25
    assert!((shame_f - 1.25).abs() < 0.001);
    // A40 → Anger ×0.84
    assert!((anger_f - 0.84).abs() < 0.001);
    // E60 → Fear ×1.30
    assert!((fear_f - 1.30).abs() < 0.001);
}
