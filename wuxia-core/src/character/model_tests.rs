use super::*;
use crate::character::CharacterEvent;
use crate::test_fixtures::make_character;

// --- Construction & Getters ---

#[test]
fn create_character() {
    let c = Character::new(
        CharacterId::new(1),
        "令狐冲".to_string(),
        Some("冲虚".to_string()),
        Gender::Male,
        1175,
        25,
        CharacterRole::Npc,
    );

    assert_eq!(c.id(), CharacterId::new(1));
    assert_eq!(c.name(), "令狐冲");
    assert_eq!(c.courtesy_name(), Some("冲虚"));
    assert_eq!(c.gender(), Gender::Male);
    assert_eq!(c.birth_year(), 1175);
    assert_eq!(c.age(), 25);
    assert_eq!(c.role(), CharacterRole::Npc);
}

#[test]
fn character_without_courtesy_name() {
    let c = make_character(1, "张无忌", 22, CharacterRole::Player);
    assert_eq!(c.courtesy_name(), None);
}

#[test]
fn character_life_stage_from_age() {
    assert_eq!(make_character(1, "A", 20, CharacterRole::Npc).life_stage(), LifeStage::Youth);
    assert_eq!(make_character(2, "B", 40, CharacterRole::Npc).life_stage(), LifeStage::Prime);
    assert_eq!(make_character(3, "C", 60, CharacterRole::Npc).life_stage(), LifeStage::Middle);
    assert_eq!(make_character(4, "D", 75, CharacterRole::Npc).life_stage(), LifeStage::Elder);
}

#[test]
fn companion_role() {
    let eagle = Character::new(
        CharacterId::new(99),
        "神雕".to_string(),
        None,
        Gender::Male,
        1140,
        60,
        CharacterRole::Companion,
    );
    assert_eq!(eagle.role(), CharacterRole::Companion);
    assert_eq!(eagle.life_stage(), LifeStage::Middle);
}

// --- Aging ---

#[test]
fn age_one_year_increments_age() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.age_one_year();
    assert_eq!(c.age(), 21);
}

#[test]
fn age_one_year_returns_aged_event() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    let events = c.age_one_year();

    assert_eq!(events.len(), 1); // Only Aged, no stage change
    match &events[0] {
        DomainEvent::Character(CharacterEvent::Aged { character_id, new_age }) => {
            assert_eq!(*character_id, CharacterId::new(1));
            assert_eq!(*new_age, 21);
        }
        other => panic!("Expected CharacterAged, got {:?}", other),
    }
}

#[test]
fn age_triggers_life_stage_change() {
    // Age 32 (Youth) → 33 (Prime)
    let mut c = make_character(1, "Test", 32, CharacterRole::Npc);
    assert_eq!(c.life_stage(), LifeStage::Youth);

    let events = c.age_one_year();
    assert_eq!(c.life_stage(), LifeStage::Prime);
    assert_eq!(events.len(), 2); // Aged + LifeStageChanged

    match &events[1] {
        DomainEvent::Character(CharacterEvent::LifeStageChanged { from, to, .. }) => {
            assert_eq!(*from, LifeStage::Youth);
            assert_eq!(*to, LifeStage::Prime);
        }
        other => panic!("Expected LifeStageChanged, got {:?}", other),
    }
}

#[test]
fn age_no_stage_change_within_same_stage() {
    // Age 20 → 21: both Youth, no stage change event
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    let events = c.age_one_year();
    assert_eq!(events.len(), 1); // Only Aged
}

#[test]
fn age_through_all_stages() {
    let mut c = make_character(1, "Test", 1, CharacterRole::Npc);
    let mut stage_changes = Vec::new();

    // Age from 1 to 100
    for _ in 0..99 {
        let events = c.age_one_year();
        for event in &events {
            if let DomainEvent::Character(CharacterEvent::LifeStageChanged { from, to, .. }) = event {
                stage_changes.push((*from, *to));
            }
        }
    }

    assert_eq!(c.age(), 100);
    assert_eq!(c.life_stage(), LifeStage::Elder);

    // Should have exactly 3 stage transitions
    assert_eq!(stage_changes.len(), 3);
    assert_eq!(stage_changes[0], (LifeStage::Youth, LifeStage::Prime));
    assert_eq!(stage_changes[1], (LifeStage::Prime, LifeStage::Middle));
    assert_eq!(stage_changes[2], (LifeStage::Middle, LifeStage::Elder));
}

#[test]
fn all_roles_can_age() {
    for role in [CharacterRole::Player, CharacterRole::Npc, CharacterRole::Companion] {
        let mut c = make_character(1, "Test", 20, role);
        let events = c.age_one_year();
        assert_eq!(c.age(), 21);
        assert!(!events.is_empty());
    }
}

// --- Display ---

#[test]
fn display_format() {
    let c = Character::new(
        CharacterId::new(1),
        "令狐冲".to_string(),
        None,
        Gender::Male,
        1175,
        25,
        CharacterRole::Npc,
    );
    assert_eq!(c.to_string(), "[Char-1] 令狐冲 (Male, age 25, Youth, Npc)");
}

#[test]
fn is_alive() {
    let c = make_character(1, "Test", 20, CharacterRole::Npc);
    assert!(c.is_alive());
}

// --- Serialization ---

#[test]
fn serialization_roundtrip() {
    let original = Character::new(
        CharacterId::new(42),
        "张三丰".to_string(),
        Some("君宝".to_string()),
        Gender::Male,
        1100,
        100,
        CharacterRole::Npc,
    );

    let json = serde_json::to_string(&original).unwrap();
    let restored: Character = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.id(), original.id());
    assert_eq!(restored.name(), original.name());
    assert_eq!(restored.courtesy_name(), original.courtesy_name());
    assert_eq!(restored.gender(), original.gender());
    assert_eq!(restored.birth_year(), original.birth_year());
    assert_eq!(restored.age(), original.age());
    assert_eq!(restored.role(), original.role());
    assert_eq!(restored.fatigue(), original.fatigue());
}

#[test]
fn deserialization_without_fatigue_defaults_to_zero() {
    // Old format without fatigue field — CharacterId is a newtype (just u64)
    let json = r#"{"id":1,"name":"Test","courtesy_name":null,"gender":"Male","birth_year":1180,"current_age":20,"role":"Npc"}"#;
    let restored: Character = serde_json::from_str(json).unwrap();
    assert_eq!(restored.fatigue(), 0);
}

// =======================================================================
// Fatigue [v2.3A]
// =======================================================================

#[test]
fn new_character_has_zero_fatigue() {
    let c = make_character(1, "Test", 20, CharacterRole::Npc);
    assert_eq!(c.fatigue(), 0);
    assert_eq!(c.fatigue_level(), FatigueLevel::Fresh);
    assert!(!c.is_exhausted());
}

#[test]
fn add_fatigue_increases() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    let events = c.add_fatigue(25);
    assert_eq!(c.fatigue(), 25);
    assert_eq!(c.fatigue_level(), FatigueLevel::Mild);
    assert_eq!(events.len(), 1);

    match &events[0] {
        DomainEvent::Character(CharacterEvent::FatigueChanged {
            old_fatigue,
            new_fatigue,
            fatigue_level,
            ..
        }) => {
            assert_eq!(*old_fatigue, 0);
            assert_eq!(*new_fatigue, 25);
            assert_eq!(*fatigue_level, FatigueLevel::Mild);
        }
        other => panic!("Expected FatigueChanged, got {:?}", other),
    }
}

#[test]
fn add_fatigue_clamped_at_100() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.add_fatigue(90);
    c.add_fatigue(50); // 90 + 50 = clamped to 100
    assert_eq!(c.fatigue(), 100);
    assert_eq!(c.fatigue_level(), FatigueLevel::Exhausted);
    assert!(c.is_exhausted());
}

#[test]
fn add_fatigue_zero_no_event() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    let events = c.add_fatigue(0);
    assert!(events.is_empty());
}

#[test]
fn add_fatigue_at_max_no_event() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.add_fatigue(100);
    let events = c.add_fatigue(10); // already at max
    assert!(events.is_empty());
    assert_eq!(c.fatigue(), 100);
}

#[test]
fn recover_fatigue_decreases() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.add_fatigue(50);
    let events = c.recover_fatigue(20);
    assert_eq!(c.fatigue(), 30);
    assert_eq!(c.fatigue_level(), FatigueLevel::Mild);
    assert_eq!(events.len(), 1);
}

#[test]
fn recover_fatigue_clamped_at_zero() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.add_fatigue(10);
    c.recover_fatigue(50); // 10 - 50 = clamped to 0
    assert_eq!(c.fatigue(), 0);
    assert_eq!(c.fatigue_level(), FatigueLevel::Fresh);
}

#[test]
fn recover_fatigue_zero_no_event() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.add_fatigue(30);
    let events = c.recover_fatigue(0);
    assert!(events.is_empty());
}

#[test]
fn recover_fatigue_at_min_no_event() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    let events = c.recover_fatigue(10); // already at 0
    assert!(events.is_empty());
}

#[test]
fn daily_rest_recovery_reduces_by_five() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.add_fatigue(30);
    let events = c.daily_rest_recovery();
    assert_eq!(c.fatigue(), 25);
    assert_eq!(events.len(), 1);
}

#[test]
fn daily_rest_recovery_does_not_go_below_zero() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.add_fatigue(3);
    c.daily_rest_recovery(); // 3 - 5 = 0
    assert_eq!(c.fatigue(), 0);
}

#[test]
fn fatigue_level_transitions() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);

    c.add_fatigue(20);
    assert_eq!(c.fatigue_level(), FatigueLevel::Fresh);

    c.add_fatigue(1); // 21
    assert_eq!(c.fatigue_level(), FatigueLevel::Mild);

    c.add_fatigue(20); // 41
    assert_eq!(c.fatigue_level(), FatigueLevel::Moderate);

    c.add_fatigue(20); // 61
    assert_eq!(c.fatigue_level(), FatigueLevel::Severe);

    c.add_fatigue(20); // 81
    assert_eq!(c.fatigue_level(), FatigueLevel::Exhausted);
    assert!(c.is_exhausted());
}

#[test]
fn fatigue_accumulation_and_recovery_cycle() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);

    // 3일 수련 (하루 15 피로)
    c.add_fatigue(15); // Day 1: 15
    c.add_fatigue(15); // Day 2: 30
    c.add_fatigue(15); // Day 3: 45
    assert_eq!(c.fatigue_level(), FatigueLevel::Moderate);

    // 3일 휴식 (하루 -5)
    c.daily_rest_recovery(); // 40
    c.daily_rest_recovery(); // 35
    c.daily_rest_recovery(); // 30
    assert_eq!(c.fatigue(), 30);
    assert_eq!(c.fatigue_level(), FatigueLevel::Mild);
}

#[test]
fn serialization_with_fatigue() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.add_fatigue(42);

    let json = serde_json::to_string(&c).unwrap();
    let restored: Character = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.fatigue(), 42);
}

// =======================================================================
// Injury [v2.3A]
// =======================================================================

use crate::character::injury::{InjuryType, InjurySeverity};

#[test]
fn new_character_has_no_injury() {
    let c = make_character(1, "Test", 20, CharacterRole::Npc);
    assert!(!c.is_injured());
    assert!(c.injury().is_none());
    assert!(c.can_train());
}

#[test]
fn injure_sets_injury() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    let events = c.injure(InjuryType::Strain, InjurySeverity::Major);

    assert!(c.is_injured());
    assert_eq!(c.injury().unwrap().injury_type(), InjuryType::Strain);
    assert_eq!(c.injury().unwrap().remaining_days(), 7);
    assert_eq!(events.len(), 1);

    match &events[0] {
        DomainEvent::Character(CharacterEvent::Injured {
            injury_type,
            severity,
            ..
        }) => {
            assert_eq!(*injury_type, InjuryType::Strain);
            assert_eq!(*severity, InjurySeverity::Major);
        }
        other => panic!("Expected Injured, got {:?}", other),
    }
}

#[test]
fn injure_replaces_existing_injury() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.injure(InjuryType::Bruise, InjurySeverity::Minor);
    c.injure(InjuryType::Fracture, InjurySeverity::Critical);

    assert_eq!(c.injury().unwrap().injury_type(), InjuryType::Fracture);
    assert_eq!(c.injury().unwrap().remaining_days(), 15);
}

#[test]
fn can_train_with_bruise() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.injure(InjuryType::Bruise, InjurySeverity::Minor);
    assert!(c.can_train(), "타박상으로는 수련 가능");
}

#[test]
fn cannot_train_with_fracture() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.injure(InjuryType::Fracture, InjurySeverity::Critical);
    assert!(!c.can_train(), "골절 시 수련 불가");
}

#[test]
fn cannot_train_when_exhausted() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.add_fatigue(100);
    assert!(!c.can_train(), "탈진 시 수련 불가");
}

#[test]
fn cannot_train_with_qi_deviation() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.injure(InjuryType::QiDeviation, InjurySeverity::Critical);
    assert!(!c.can_train(), "주화입마 시 수련 불가");
}

#[test]
fn heal_daily_reduces_remaining_days() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.injure(InjuryType::Bruise, InjurySeverity::Minor); // 3일

    let events = c.heal_daily(); // 2일 남음
    assert!(events.is_empty(), "아직 완치 안됨 → 이벤트 없음");
    assert_eq!(c.injury().unwrap().remaining_days(), 2);
}

#[test]
fn heal_daily_completes_on_last_day() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.injure(InjuryType::Bruise, InjurySeverity::Minor); // 3일

    c.heal_daily(); // 2
    c.heal_daily(); // 1
    let events = c.heal_daily(); // 0 → 완치!

    assert!(!c.is_injured());
    assert_eq!(events.len(), 1);
    match &events[0] {
        DomainEvent::Character(CharacterEvent::InjuryHealed { injury_type, .. }) => {
            assert_eq!(*injury_type, InjuryType::Bruise);
        }
        other => panic!("Expected InjuryHealed, got {:?}", other),
    }
}

#[test]
fn heal_daily_no_injury_no_event() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    let events = c.heal_daily();
    assert!(events.is_empty());
}

#[test]
fn treat_injury_accelerates_healing() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.injure(InjuryType::Fracture, InjurySeverity::Critical); // 15일

    let events = c.treat_injury(5);
    assert!(events.is_empty(), "아직 10일 남음");
    assert_eq!(c.injury().unwrap().remaining_days(), 10);
}

#[test]
fn treat_injury_can_fully_heal() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.injure(InjuryType::Strain, InjurySeverity::Major); // 7일

    let events = c.treat_injury(10); // 7 - 10 = 0 → 완치
    assert!(!c.is_injured());
    assert_eq!(events.len(), 1);
}

#[test]
fn treat_injury_no_injury_no_event() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    let events = c.treat_injury(5);
    assert!(events.is_empty());
}

#[test]
fn serialization_with_injury() {
    let mut c = make_character(1, "Test", 20, CharacterRole::Npc);
    c.injure(InjuryType::QiDeviation, InjurySeverity::Critical);

    let json = serde_json::to_string(&c).unwrap();
    let restored: Character = serde_json::from_str(&json).unwrap();
    assert!(restored.is_injured());
    assert_eq!(restored.injury().unwrap().injury_type(), InjuryType::QiDeviation);
}

#[test]
fn deserialization_without_injury_defaults_to_none() {
    let json = r#"{"id":1,"name":"Test","courtesy_name":null,"gender":"Male","birth_year":1180,"current_age":20,"role":"Npc"}"#;
    let restored: Character = serde_json::from_str(json).unwrap();
    assert!(!restored.is_injured());
}

// --- 무협 시나리오: 부상과 수련 ---

#[test]
fn scenario_bruise_allows_limited_training() {
    let mut c = make_character(1, "무명소졸", 20, CharacterRole::Npc);
    c.injure(InjuryType::Bruise, InjurySeverity::Minor);

    assert!(c.can_train(), "타박상 — 수련 가능하지만 강도 제한");
    assert_eq!(c.injury().unwrap().intensity_penalty(), 1);
}

#[test]
fn scenario_qi_deviation_full_recovery_with_treatment() {
    let mut c = make_character(1, "令狐冲", 25, CharacterRole::Player);
    c.injure(InjuryType::QiDeviation, InjurySeverity::Critical);
    assert!(!c.can_train(), "주화입마 — 수련 불가");

    // 5일 자연 치유
    for _ in 0..5 { c.heal_daily(); }
    assert_eq!(c.injury().unwrap().remaining_days(), 10);

    // 동료 간호 (3일 단축)
    c.treat_injury(3);
    assert_eq!(c.injury().unwrap().remaining_days(), 7);

    // 의원 치료 (7일 단축) → 완치
    let events = c.treat_injury(7);
    assert!(!c.is_injured());
    assert!(c.can_train(), "완치 후 수련 재개 가능");
    assert_eq!(events.len(), 1);
}
