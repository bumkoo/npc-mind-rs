use super::*;

fn cid() -> CharacterId {
    CharacterId::new(1)
}

// -- 생성 --

#[test]
fn new_normal() {
    let p = HexacoPersonality::new(cid(), 90, 50, 50, 80, 90, 60);
    assert_eq!(p.h(), 90);
    assert_eq!(p.e(), 50);
    assert_eq!(p.x(), 50);
    assert_eq!(p.a(), 80);
    assert_eq!(p.c(), 90);
    assert_eq!(p.o(), 60);
}

#[test]
fn new_clamps_over_100() {
    let p = HexacoPersonality::new(cid(), 110, 200, 50, 50, 50, 50);
    assert_eq!(p.h(), 100);
    assert_eq!(p.e(), 100);
}

#[test]
fn character_id_getter() {
    let p = HexacoPersonality::new(cid(), 50, 50, 50, 50, 50, 50);
    assert_eq!(p.character_id(), cid());
}

// -- get() --

#[test]
fn get_all_factors() {
    let p = HexacoPersonality::new(cid(), 10, 20, 30, 40, 50, 60);
    assert_eq!(p.get(HexacoFactor::HonestyHumility), 10);
    assert_eq!(p.get(HexacoFactor::Emotionality), 20);
    assert_eq!(p.get(HexacoFactor::Extraversion), 30);
    assert_eq!(p.get(HexacoFactor::Agreeableness), 40);
    assert_eq!(p.get(HexacoFactor::Conscientiousness), 50);
    assert_eq!(p.get(HexacoFactor::Openness), 60);
}

// -- Tier 4 변경 --

#[test]
fn tier4_single_change() {
    let mut p = HexacoPersonality::new(cid(), 50, 50, 50, 50, 50, 50);
    let events = p.apply_tier4_change(&[(HexacoFactor::Extraversion, 5)]).unwrap();
    assert_eq!(p.x(), 55);
    assert_eq!(events.len(), 1);
    match &events[0] {
        PsychologyEvent::PersonalityChanged { factor, old_value, new_value, .. } => {
            assert_eq!(*factor, HexacoFactor::Extraversion);
            assert_eq!(*old_value, 50);
            assert_eq!(*new_value, 55);
        }
        _ => panic!("Expected PersonalityChanged"),
    }
}

#[test]
fn tier4_two_changes() {
    let mut p = HexacoPersonality::new(cid(), 50, 50, 30, 50, 50, 50);
    let events = p.apply_tier4_change(&[
        (HexacoFactor::Extraversion, 5),
        (HexacoFactor::Agreeableness, -3),
    ]).unwrap();
    assert_eq!(p.x(), 35);
    assert_eq!(p.a(), 47);
    assert_eq!(events.len(), 2);
}

#[test]
fn tier4_three_changes_error() {
    let mut p = HexacoPersonality::new(cid(), 50, 50, 50, 50, 50, 50);
    let result = p.apply_tier4_change(&[
        (HexacoFactor::HonestyHumility, 5),
        (HexacoFactor::Emotionality, -3),
        (HexacoFactor::Extraversion, 2),
    ]);
    assert!(result.is_err());
    match result.unwrap_err() {
        PsychologyError::TooManyPersonalityChanges { attempted, max_allowed } => {
            assert_eq!(attempted, 3);
            assert_eq!(max_allowed, 2);
        }
    }
    // 실패 시 원래 값 유지 — 3개 이상이므로 적용 전에 거부됨
    assert_eq!(p.h(), 50);
}

#[test]
fn tier4_delta_clamped_to_5() {
    let mut p = HexacoPersonality::new(cid(), 50, 50, 50, 50, 50, 50);
    p.apply_tier4_change(&[(HexacoFactor::HonestyHumility, 10)]).unwrap();
    assert_eq!(p.h(), 55); // 10 → clamped to 5
}

#[test]
fn tier4_delta_clamped_to_minus_5() {
    let mut p = HexacoPersonality::new(cid(), 50, 50, 50, 50, 50, 50);
    p.apply_tier4_change(&[(HexacoFactor::Agreeableness, -10)]).unwrap();
    assert_eq!(p.a(), 45); // -10 → clamped to -5
}

#[test]
fn tier4_result_clamps_to_100() {
    let mut p = HexacoPersonality::new(cid(), 98, 50, 50, 50, 50, 50);
    p.apply_tier4_change(&[(HexacoFactor::HonestyHumility, 5)]).unwrap();
    assert_eq!(p.h(), 100);
}

#[test]
fn tier4_result_clamps_to_0() {
    let mut p = HexacoPersonality::new(cid(), 2, 50, 50, 50, 50, 50);
    p.apply_tier4_change(&[(HexacoFactor::HonestyHumility, -5)]).unwrap();
    assert_eq!(p.h(), 0);
}

#[test]
fn tier4_zero_delta_ignored() {
    let mut p = HexacoPersonality::new(cid(), 50, 50, 50, 50, 50, 50);
    let events = p.apply_tier4_change(&[
        (HexacoFactor::HonestyHumility, 0),
        (HexacoFactor::Emotionality, 3),
        (HexacoFactor::Extraversion, 0),
        (HexacoFactor::Agreeableness, -2),
    ]).unwrap();
    // 0인 변경은 무시 → 실제 변경은 2개
    assert_eq!(events.len(), 2);
    assert_eq!(p.e(), 53);
    assert_eq!(p.a(), 48);
}

#[test]
fn tier4_no_event_when_no_actual_change() {
    let mut p = HexacoPersonality::new(cid(), 100, 50, 50, 50, 50, 50);
    let events = p.apply_tier4_change(&[(HexacoFactor::HonestyHumility, 5)]).unwrap();
    // 이미 100이므로 실제 변화 없음
    assert!(events.is_empty());
}

// -- Derived metrics --

#[test]
fn emotional_reactivity() {
    let p = HexacoPersonality::new(cid(), 50, 80, 50, 50, 50, 50);
    assert!((p.emotional_reactivity() - 0.8).abs() < f32::EPSILON);
}

#[test]
fn anger_suppression() {
    let p = HexacoPersonality::new(cid(), 50, 50, 50, 80, 50, 50);
    assert!((p.anger_suppression() - 0.8).abs() < f32::EPSILON);
}

#[test]
fn moral_sensitivity() {
    let p = HexacoPersonality::new(cid(), 90, 50, 50, 50, 50, 50);
    assert!((p.moral_sensitivity() - 0.9).abs() < f32::EPSILON);
}

#[test]
fn impulse_control() {
    let p = HexacoPersonality::new(cid(), 50, 50, 50, 50, 90, 50);
    assert!((p.impulse_control() - 0.9).abs() < f32::EPSILON);
}

#[test]
fn complex_emotion_tolerance() {
    let p = HexacoPersonality::new(cid(), 50, 50, 50, 50, 50, 80);
    assert!((p.complex_emotion_tolerance() - 0.8).abs() < f32::EPSILON);
}

// -- HexacoFactor --

#[test]
fn all_factors_has_six() {
    assert_eq!(HexacoFactor::ALL.len(), 6);
}

// -- PsychologyError --

#[test]
fn error_display() {
    let err = PsychologyError::TooManyPersonalityChanges {
        attempted: 3,
        max_allowed: 2,
    };
    assert!(err.to_string().contains("3"));
    assert!(err.to_string().contains("2"));
}

// -- Serialization --

#[test]
fn serialization_roundtrip() {
    let original = HexacoPersonality::new(cid(), 90, 50, 50, 80, 90, 60);
    let json = serde_json::to_string(&original).unwrap();
    let restored: HexacoPersonality = serde_json::from_str(&json).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn factor_serialization() {
    for f in HexacoFactor::ALL {
        let json = serde_json::to_string(&f).unwrap();
        let restored: HexacoFactor = serde_json::from_str(&json).unwrap();
        assert_eq!(f, restored);
    }
}

// -- 무협 시나리오 --

#[test]
fn scenario_jinyarim_tier4_opening_up() {
    // 진야림: X30 → 플레이어와의 긴 여정 후 Tier4에서 X+5
    let mut p = HexacoPersonality::new(cid(), 60, 40, 30, 60, 30, 50);
    let events = p.apply_tier4_change(&[
        (HexacoFactor::Extraversion, 5),
    ]).unwrap();
    assert_eq!(p.x(), 35);
    assert_eq!(events.len(), 1);
}

#[test]
fn scenario_namgung_tier4_humility_drop() {
    // 남궁현: H40 → 야망 달성 후 Tier4에서 H-5, A-3
    let mut p = HexacoPersonality::new(cid(), 40, 50, 70, 30, 70, 60);
    let events = p.apply_tier4_change(&[
        (HexacoFactor::HonestyHumility, -5),
        (HexacoFactor::Agreeableness, -3),
    ]).unwrap();
    assert_eq!(p.h(), 35);
    assert_eq!(p.a(), 27);
    assert_eq!(events.len(), 2);
}
