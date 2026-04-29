use super::*;

fn cid() -> CharacterId {
    CharacterId::new(1)
}

fn make_axis(intensity: f32, creed: &str) -> ValueAxis {
    ValueAxis::new(intensity, creed.to_string())
}

fn make_values() -> ThreeAxisValues {
    ThreeAxisValues::new(
        cid(),
        make_axis(80.0, "사람을 믿는다"),
        make_axis(90.0, "도의를 지켜야 한다"),
        make_axis(50.0, "제자들을 지키겠다"),
    )
}

// -- ValueAxis 생성 --

#[test]
fn value_axis_clamps_intensity() {
    let axis = ValueAxis::new(120.0, "test".to_string());
    assert_eq!(axis.intensity(), 100.0);
    let axis2 = ValueAxis::new(-10.0, "test".to_string());
    assert_eq!(axis2.intensity(), 0.0);
}

#[test]
fn value_axis_normal() {
    let axis = make_axis(75.0, "도의");
    assert_eq!(axis.intensity(), 75.0);
    assert_eq!(axis.creed(), "도의");
    assert!(axis.creed_candidates().is_empty());
    assert!(axis.formation_memories().is_empty());
}

// -- CreedCandidate --

#[test]
fn creed_candidate_new() {
    let c = CreedCandidate::new("힘이 정의".to_string(), "조고".to_string());
    assert_eq!(c.text(), "힘이 정의");
    assert_eq!(c.source(), "조고");
    assert_eq!(c.exposure_count(), 1);
    assert_eq!(c.resonance(), 0.0);
}

#[test]
fn creed_candidate_increment_exposure() {
    let mut c = CreedCandidate::new("test".to_string(), "src".to_string());
    c.increment_exposure();
    assert_eq!(c.exposure_count(), 2);
    c.increment_exposure();
    assert_eq!(c.exposure_count(), 3);
}

#[test]
fn creed_candidate_set_resonance_clamps() {
    let mut c = CreedCandidate::new("test".to_string(), "src".to_string());
    c.set_resonance(50.0);
    assert_eq!(c.resonance(), 50.0);
    c.set_resonance(120.0);
    assert_eq!(c.resonance(), 100.0);
    c.set_resonance(-10.0);
    assert_eq!(c.resonance(), 0.0);
}

// -- ThreeAxisValues 생성 --

#[test]
fn new_three_axis() {
    let v = make_values();
    assert_eq!(v.character_id(), cid());
    assert_eq!(v.trust().intensity(), 80.0);
    assert_eq!(v.rightness().intensity(), 90.0);
    assert_eq!(v.want().intensity(), 50.0);
    assert_eq!(v.trust().creed(), "사람을 믿는다");
}

#[test]
fn axis_getter() {
    let v = make_values();
    assert_eq!(v.axis(AxisType::Trust).intensity(), 80.0);
    assert_eq!(v.axis(AxisType::Rightness).intensity(), 90.0);
    assert_eq!(v.axis(AxisType::Want).intensity(), 50.0);
}

// -- adjust_intensity --

#[test]
fn adjust_intensity_positive() {
    let mut v = make_values();
    let events = v.adjust_intensity(AxisType::Want, 3.0, ReflectionTier::Instant);
    assert_eq!(v.want().intensity(), 53.0);
    assert_eq!(events.len(), 1);
}

#[test]
fn adjust_intensity_negative() {
    let mut v = make_values();
    let events = v.adjust_intensity(AxisType::Trust, -5.0, ReflectionTier::Instant);
    assert_eq!(v.trust().intensity(), 75.0);
    assert_eq!(events.len(), 1);
}

#[test]
fn adjust_intensity_zero_noop() {
    let mut v = make_values();
    let events = v.adjust_intensity(AxisType::Trust, 0.0, ReflectionTier::Instant);
    assert!(events.is_empty());
}

#[test]
fn adjust_intensity_no_change_at_max_noop() {
    let mut v = ThreeAxisValues::new(
        cid(),
        make_axis(100.0, "max"),
        make_axis(50.0, "mid"),
        make_axis(50.0, "mid"),
    );
    let events = v.adjust_intensity(AxisType::Trust, 5.0, ReflectionTier::Instant);
    assert!(events.is_empty());
}

// -- Tier별 범위 --

#[test]
fn tier1_clamps_at_5() {
    let mut v = make_values();
    v.adjust_intensity(AxisType::Trust, 10.0, ReflectionTier::Instant);
    assert_eq!(v.trust().intensity(), 85.0); // 80 + 5
}

#[test]
fn tier2_clamps_at_10() {
    let mut v = make_values();
    v.adjust_intensity(AxisType::Trust, 20.0, ReflectionTier::Daily);
    assert_eq!(v.trust().intensity(), 90.0); // 80 + 10
}

#[test]
fn tier3_clamps_at_20() {
    let mut v = make_values();
    v.adjust_intensity(AxisType::Want, 30.0, ReflectionTier::TurningPoint);
    assert_eq!(v.want().intensity(), 70.0); // 50 + 20
}

#[test]
fn tier4_clamps_at_30() {
    let mut v = make_values();
    v.adjust_intensity(AxisType::Want, 50.0, ReflectionTier::Life);
    assert_eq!(v.want().intensity(), 80.0); // 50 + 30
}

// -- update_creed --

#[test]
fn update_creed_changes() {
    let mut v = make_values();
    let events = v.update_creed(AxisType::Rightness, "때로는 유연해야 한다".to_string());
    assert_eq!(v.rightness().creed(), "때로는 유연해야 한다");
    assert_eq!(events.len(), 1);
    match &events[0] {
        PsychologyEvent::CreedChanged { old_creed, new_creed, .. } => {
            assert_eq!(old_creed, "도의를 지켜야 한다");
            assert_eq!(new_creed, "때로는 유연해야 한다");
        }
        _ => panic!("Expected CreedChanged"),
    }
}

#[test]
fn update_creed_same_noop() {
    let mut v = make_values();
    let events = v.update_creed(AxisType::Trust, "사람을 믿는다".to_string());
    assert!(events.is_empty());
}

// -- add_creed_candidate --

#[test]
fn add_creed_candidate_appends() {
    let mut v = make_values();
    let candidate = CreedCandidate::new("힘이 정의".to_string(), "조고".to_string());
    let events = v.add_creed_candidate(AxisType::Rightness, candidate);
    assert_eq!(v.rightness().creed_candidates().len(), 1);
    assert_eq!(events.len(), 1);
}

#[test]
fn increment_candidate_exposure_works() {
    let mut v = make_values();
    let candidate = CreedCandidate::new("힘이 정의".to_string(), "조고".to_string());
    v.add_creed_candidate(AxisType::Rightness, candidate);
    v.increment_candidate_exposure(AxisType::Rightness, 0);
    assert_eq!(v.rightness().creed_candidates()[0].exposure_count(), 2);
}

#[test]
fn increment_candidate_out_of_bounds_safe() {
    let mut v = make_values();
    // 범위 밖 → 아무 일도 안 일어남
    v.increment_candidate_exposure(AxisType::Trust, 99);
}

// -- add_formation_memory --

#[test]
fn add_formation_memory_appends() {
    let mut v = make_values();
    v.add_formation_memory(AxisType::Trust, MemoryId::new(42));
    assert_eq!(v.trust().formation_memories().len(), 1);
    assert_eq!(v.trust().formation_memories()[0], MemoryId::new(42));
}

// -- AxisType --

#[test]
fn axis_type_all_has_three() {
    assert_eq!(AxisType::ALL.len(), 3);
}

// -- Serialization --

#[test]
fn serialization_roundtrip() {
    let mut v = make_values();
    v.add_creed_candidate(
        AxisType::Rightness,
        CreedCandidate::new("힘이 정의".to_string(), "조고".to_string()),
    );
    v.add_formation_memory(AxisType::Trust, MemoryId::new(1));

    let json = serde_json::to_string(&v).unwrap();
    let restored: ThreeAxisValues = serde_json::from_str(&json).unwrap();
    assert_eq!(v, restored);
}

// -- 무협 시나리오 --

#[test]
fn scenario_myungkyung_creed_conflict() {
    // 명경: 옳음(正) 90, "도의를 지켜야 한다"
    // 조고의 설득으로 대안 신조 접촉
    let mut v = ThreeAxisValues::new(
        cid(),
        make_axis(70.0, "제자를 믿는다"),
        make_axis(90.0, "도의를 지켜야 한다"),
        make_axis(60.0, "제자들을 지키겠다"),
    );

    // 조고 만남 → 대안 접촉
    v.add_creed_candidate(
        AxisType::Rightness,
        CreedCandidate::new("때로는 타협해야 한다".to_string(), "조고".to_string()),
    );
    assert_eq!(v.rightness().creed_candidates().len(), 1);

    // 반복 접촉
    v.increment_candidate_exposure(AxisType::Rightness, 0);
    v.increment_candidate_exposure(AxisType::Rightness, 0);
    assert_eq!(v.rightness().creed_candidates()[0].exposure_count(), 3);

    // 옳음 강도는 아직 변하지 않음
    assert_eq!(v.rightness().intensity(), 90.0);
}

#[test]
fn scenario_soyeon_want_axis_rising() {
    // 소연: 바람(願) 70, "원수를 갚고 정보망을 세우겠다"
    // Tier 3 전환점 → 강도 +15
    let mut v = ThreeAxisValues::new(
        cid(),
        make_axis(50.0, "사부를 믿는다"),
        make_axis(60.0, "강호 도리를 지킨다"),
        make_axis(70.0, "원수를 갚고 정보망을 세우겠다"),
    );

    let events = v.adjust_intensity(AxisType::Want, 15.0, ReflectionTier::TurningPoint);
    assert_eq!(v.want().intensity(), 85.0);
    assert_eq!(events.len(), 1);
}
