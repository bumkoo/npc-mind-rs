use super::*;

fn cid() -> CharacterId {
    CharacterId::new(1)
}

// -- 생성 --

#[test]
fn new_clamps_values() {
    let v = PracticalValues::new(cid(), 110.0, -10.0, 50.0, 0.0, 100.0);
    assert_eq!(v.loyalty(), 100.0);
    assert_eq!(v.righteousness(), 0.0);
    assert_eq!(v.filial_piety(), 50.0);
    assert_eq!(v.vengeance(), 0.0);
    assert_eq!(v.ambition(), 100.0);
}

#[test]
fn new_normal_values() {
    let v = PracticalValues::new(cid(), 90.0, 80.0, 70.0, 30.0, 20.0);
    assert_eq!(v.loyalty(), 90.0);
    assert_eq!(v.righteousness(), 80.0);
    assert_eq!(v.filial_piety(), 70.0);
    assert_eq!(v.vengeance(), 30.0);
    assert_eq!(v.ambition(), 20.0);
}

#[test]
fn character_id_getter() {
    let v = PracticalValues::new(cid(), 50.0, 50.0, 50.0, 50.0, 50.0);
    assert_eq!(v.character_id(), cid());
}

// -- get() --

#[test]
fn get_all_types() {
    let v = PracticalValues::new(cid(), 10.0, 20.0, 30.0, 40.0, 50.0);
    assert_eq!(v.get(PracticalValueType::Loyalty), 10.0);
    assert_eq!(v.get(PracticalValueType::Righteousness), 20.0);
    assert_eq!(v.get(PracticalValueType::FilialPiety), 30.0);
    assert_eq!(v.get(PracticalValueType::Vengeance), 40.0);
    assert_eq!(v.get(PracticalValueType::Ambition), 50.0);
}

// -- adjust() --

#[test]
fn adjust_positive_delta() {
    let mut v = PracticalValues::new(cid(), 50.0, 50.0, 50.0, 50.0, 50.0);
    let events = v.adjust(PracticalValueType::Loyalty, 3.0, ReflectionTier::Instant);
    assert_eq!(v.loyalty(), 53.0);
    assert_eq!(events.len(), 1);
}

#[test]
fn adjust_negative_delta() {
    let mut v = PracticalValues::new(cid(), 50.0, 50.0, 50.0, 50.0, 50.0);
    let events = v.adjust(PracticalValueType::Vengeance, -5.0, ReflectionTier::Instant);
    assert_eq!(v.vengeance(), 45.0);
    assert_eq!(events.len(), 1);
}

#[test]
fn adjust_zero_delta_noop() {
    // no-op rule: delta 0이면 이벤트 없음
    let mut v = PracticalValues::new(cid(), 50.0, 50.0, 50.0, 50.0, 50.0);
    let events = v.adjust(PracticalValueType::Loyalty, 0.0, ReflectionTier::Instant);
    assert!(events.is_empty());
    assert_eq!(v.loyalty(), 50.0);
}

#[test]
fn adjust_no_change_at_max_noop() {
    // 이미 100이면 양수 delta는 no-op
    let mut v = PracticalValues::new(cid(), 100.0, 50.0, 50.0, 50.0, 50.0);
    let events = v.adjust(PracticalValueType::Loyalty, 5.0, ReflectionTier::Instant);
    assert!(events.is_empty());
}

#[test]
fn adjust_no_change_at_min_noop() {
    // 이미 0이면 음수 delta는 no-op
    let mut v = PracticalValues::new(cid(), 0.0, 50.0, 50.0, 50.0, 50.0);
    let events = v.adjust(PracticalValueType::Loyalty, -5.0, ReflectionTier::Instant);
    assert!(events.is_empty());
}

// -- Tier별 범위 클램프 --

#[test]
fn tier1_clamps_at_5() {
    let mut v = PracticalValues::new(cid(), 50.0, 50.0, 50.0, 50.0, 50.0);
    v.adjust(PracticalValueType::Loyalty, 10.0, ReflectionTier::Instant);
    assert_eq!(v.loyalty(), 55.0); // 10 → clamped to 5
}

#[test]
fn tier1_clamps_negative_at_5() {
    let mut v = PracticalValues::new(cid(), 50.0, 50.0, 50.0, 50.0, 50.0);
    v.adjust(PracticalValueType::Loyalty, -10.0, ReflectionTier::Instant);
    assert_eq!(v.loyalty(), 45.0); // -10 → clamped to -5
}

#[test]
fn tier2_clamps_at_10() {
    let mut v = PracticalValues::new(cid(), 50.0, 50.0, 50.0, 50.0, 50.0);
    v.adjust(PracticalValueType::Righteousness, 20.0, ReflectionTier::Daily);
    assert_eq!(v.righteousness(), 60.0); // 20 → clamped to 10
}

#[test]
fn tier3_clamps_at_20() {
    let mut v = PracticalValues::new(cid(), 50.0, 50.0, 50.0, 50.0, 50.0);
    v.adjust(PracticalValueType::Vengeance, 30.0, ReflectionTier::TurningPoint);
    assert_eq!(v.vengeance(), 70.0); // 30 → clamped to 20
}

#[test]
fn tier4_clamps_at_20() {
    let mut v = PracticalValues::new(cid(), 50.0, 50.0, 50.0, 50.0, 50.0);
    v.adjust(PracticalValueType::Ambition, 30.0, ReflectionTier::Life);
    assert_eq!(v.ambition(), 70.0); // 30 → clamped to 20
}

// -- 값 범위 클램프 --

#[test]
fn adjust_clamps_to_100() {
    let mut v = PracticalValues::new(cid(), 95.0, 50.0, 50.0, 50.0, 50.0);
    v.adjust(PracticalValueType::Loyalty, 5.0, ReflectionTier::Instant);
    assert_eq!(v.loyalty(), 100.0);
}

#[test]
fn adjust_clamps_to_0() {
    let mut v = PracticalValues::new(cid(), 3.0, 50.0, 50.0, 50.0, 50.0);
    v.adjust(PracticalValueType::Loyalty, -5.0, ReflectionTier::Instant);
    assert_eq!(v.loyalty(), 0.0);
}

// -- Derived metrics --

#[test]
fn alignment_positive_for_righteous() {
    // 명경: 충90 의90 효70 복수30 야망20
    let v = PracticalValues::new(cid(), 90.0, 90.0, 70.0, 30.0, 20.0);
    let a = v.alignment();
    // (90+90+70) - (30+20) = 250 - 50 = 200
    assert_eq!(a, 200.0);
    assert!(a > 0.0, "의로운 방향");
}

#[test]
fn alignment_negative_for_ambitious() {
    // 조고: 충30 의10 효10 복수70 야망90
    let v = PracticalValues::new(cid(), 30.0, 10.0, 10.0, 70.0, 90.0);
    let a = v.alignment();
    // (30+10+10) - (70+90) = 50 - 160 = -110
    assert_eq!(a, -110.0);
    assert!(a < 0.0, "야망/복수 방향");
}

#[test]
fn alignment_zero_balanced() {
    let v = PracticalValues::new(cid(), 40.0, 40.0, 20.0, 50.0, 50.0);
    // (40+40+20) - (50+50) = 100 - 100 = 0
    assert_eq!(v.alignment(), 0.0);
}

#[test]
fn betrayal_potential_high_for_ambitious_disloyal() {
    // 야망100, 충0, 의0 → 최대 배신 가능성
    let v = PracticalValues::new(cid(), 0.0, 0.0, 50.0, 50.0, 100.0);
    let bp = v.betrayal_potential();
    // 100/100 × (1-0/100) × (1-0/100) = 1.0
    assert!((bp - 1.0).abs() < f32::EPSILON);
}

#[test]
fn betrayal_potential_low_for_loyal() {
    // 야망20, 충90, 의90
    let v = PracticalValues::new(cid(), 90.0, 90.0, 50.0, 50.0, 20.0);
    let bp = v.betrayal_potential();
    // 20/100 × (1-90/100) × (1-90/100) = 0.2 × 0.1 × 0.1 = 0.002
    assert!((bp - 0.002).abs() < 0.001);
}

#[test]
fn betrayal_potential_zero_when_no_ambition() {
    let v = PracticalValues::new(cid(), 50.0, 50.0, 50.0, 50.0, 0.0);
    assert_eq!(v.betrayal_potential(), 0.0);
}

// -- Event 내용 검증 --

#[test]
fn adjust_event_contains_correct_values() {
    let mut v = PracticalValues::new(cid(), 50.0, 50.0, 50.0, 50.0, 50.0);
    let events = v.adjust(PracticalValueType::Righteousness, 3.0, ReflectionTier::Instant);
    match &events[0] {
        PsychologyEvent::PracticalValueChanged {
            character_id,
            value_type,
            old_value,
            new_value,
            tier,
        } => {
            assert_eq!(*character_id, cid());
            assert_eq!(*value_type, PracticalValueType::Righteousness);
            assert_eq!(*old_value, 50.0);
            assert_eq!(*new_value, 53.0);
            assert_eq!(*tier, ReflectionTier::Instant);
        }
        _ => panic!("Expected PracticalValueChanged"),
    }
}

// -- PracticalValueType --

#[test]
fn all_types_has_five_variants() {
    assert_eq!(PracticalValueType::ALL.len(), 5);
}

// -- Serialization --

#[test]
fn serialization_roundtrip() {
    let original = PracticalValues::new(cid(), 90.0, 80.0, 70.0, 30.0, 20.0);
    let json = serde_json::to_string(&original).unwrap();
    let restored: PracticalValues = serde_json::from_str(&json).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn value_type_serialization() {
    for vt in PracticalValueType::ALL {
        let json = serde_json::to_string(&vt).unwrap();
        let restored: PracticalValueType = serde_json::from_str(&json).unwrap();
        assert_eq!(vt, restored);
    }
}

// -- 무협 시나리오 --

#[test]
fn scenario_soyeon_vengeance_rising() {
    // 소연: 복수 60 → 형제들의 원수 소식 → Tier3 전환점에서 +15
    let mut v = PracticalValues::new(cid(), 50.0, 60.0, 40.0, 60.0, 40.0);
    let initial_alignment = v.alignment();
    let events = v.adjust(PracticalValueType::Vengeance, 15.0, ReflectionTier::TurningPoint);
    assert_eq!(v.vengeance(), 75.0);
    assert_eq!(events.len(), 1);
    // alignment 감소: 복수 상승 → 야망/복수 방향으로 이동
    assert!(v.alignment() < initial_alignment, "복수 상승으로 alignment 감소");
}

#[test]
fn scenario_myungkyung_loyalty_conflict() {
    // 명경: 충90 → 조고의 압박에 제자를 포기해야 하는 상황
    // Tier3 전환점에서 충(忠) -15 (조직 충성 vs 제자 보호 갈등)
    let mut v = PracticalValues::new(cid(), 90.0, 90.0, 70.0, 30.0, 20.0);
    let initial_alignment = v.alignment();
    v.adjust(PracticalValueType::Loyalty, -15.0, ReflectionTier::TurningPoint);
    assert_eq!(v.loyalty(), 75.0);
    // alignment 감소 (충 하락)
    assert!(v.alignment() < initial_alignment);
}
