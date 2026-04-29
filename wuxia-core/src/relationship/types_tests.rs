use super::*;

fn make_rel() -> Relationship {
    crate::test_fixtures::make_relationship(1, 1, 2)
}

// =======================================================================
// 생성 테스트
// =======================================================================

#[test]
fn new_relationship_has_zero_values() {
    let rel = make_rel();
    assert_eq!(rel.affinity(), 0.0);
    assert_eq!(rel.trust(), 0.0);
    assert_eq!(rel.interaction_count(), 0);
    assert!(rel.last_interaction().is_none());
    assert!(rel.relation_type().is_none());
}

#[test]
fn with_type_sets_relation_type() {
    let player = CharacterId::new(1);
    let soyeon = CharacterId::new(2);
    let rel = Relationship::with_type(
        RelationshipId::new(1),
        player,
        soyeon,
        RelationshipType::MasterDisciple,
    );
    assert_eq!(rel.relation_type(), Some(RelationshipType::MasterDisciple));
    assert_eq!(rel.affinity(), 0.0); // 수치는 여전히 0
}

// =======================================================================
// 수치 변경 + Clamp 테스트
// =======================================================================

#[test]
fn update_affinity_increases() {
    let mut rel = make_rel();
    rel.update_affinity(30.0);
    assert_eq!(rel.affinity(), 30.0);
}

#[test]
fn update_affinity_decreases() {
    let mut rel = make_rel();
    rel.update_affinity(50.0);
    rel.update_affinity(-20.0);
    assert_eq!(rel.affinity(), 30.0);
}

#[test]
fn affinity_goes_negative() {
    // 2축 모델: affinity는 -100까지 내려갈 수 있다
    let mut rel = make_rel();
    rel.update_affinity(-50.0);
    assert_eq!(rel.affinity(), -50.0);
}

#[test]
fn affinity_clamps_at_negative_hundred() {
    let mut rel = make_rel();
    rel.update_affinity(-150.0);
    assert_eq!(rel.affinity(), -100.0);
}

#[test]
fn affinity_clamps_at_hundred() {
    let mut rel = make_rel();
    rel.update_affinity(150.0);
    assert_eq!(rel.affinity(), 100.0);
}

#[test]
fn trust_clamps_at_boundaries() {
    let mut rel = make_rel();
    rel.update_trust(200.0);
    assert_eq!(rel.trust(), 100.0);
    rel.update_trust(-300.0);
    assert_eq!(rel.trust(), 0.0);
}

// =======================================================================
// RelationshipLevel 판정 테스트
// =======================================================================

#[test]
fn level_stranger_by_default() {
    let rel = make_rel();
    assert_eq!(rel.level(), RelationshipLevel::Stranger);
}

#[test]
fn level_acquaintance_by_affinity() {
    let mut rel = make_rel();
    rel.update_affinity(20.0);
    assert_eq!(rel.level(), RelationshipLevel::Acquaintance);
}

#[test]
fn level_acquaintance_by_trust_only() {
    let mut rel = make_rel();
    rel.update_trust(25.0);
    // affinity=0 이지만 trust>=20 이면 Acquaintance (OR 조건)
    assert_eq!(rel.level(), RelationshipLevel::Acquaintance);
}

#[test]
fn level_friendly_requires_both() {
    let mut rel = make_rel();
    rel.update_affinity(50.0);
    rel.update_trust(10.0); // trust 부족
    assert_eq!(rel.level(), RelationshipLevel::Acquaintance);

    rel.update_trust(20.0); // trust=30 이제 충족
    assert_eq!(rel.level(), RelationshipLevel::Friendly);
}

#[test]
fn level_close() {
    let mut rel = make_rel();
    rel.update_affinity(70.0);
    rel.update_trust(50.0);
    assert_eq!(rel.level(), RelationshipLevel::Close);
}

#[test]
fn level_intimate() {
    let mut rel = make_rel();
    rel.update_affinity(80.0);
    rel.update_trust(70.0);
    assert_eq!(rel.level(), RelationshipLevel::Intimate);
}

// =======================================================================
// 음수 호감도 → 적대/경계 판정 테스트
// =======================================================================

#[test]
fn level_wary() {
    // affinity <= -10 → Wary
    let mut rel = make_rel();
    rel.update_affinity(-15.0);
    assert_eq!(rel.level(), RelationshipLevel::Wary);
}

#[test]
fn level_hostile_by_negative_affinity() {
    // affinity <= -40 → Hostile
    let mut rel = make_rel();
    rel.update_affinity(-45.0);
    assert_eq!(rel.level(), RelationshipLevel::Hostile);
}

#[test]
fn level_enemy_by_negative_affinity() {
    // affinity <= -80 → Enemy
    let mut rel = make_rel();
    rel.update_affinity(-85.0);
    assert_eq!(rel.level(), RelationshipLevel::Enemy);
}

#[test]
fn negative_affinity_overrides_high_trust() {
    // "소연이 적으로 돌아서면, 신뢰가 높아도 적대로 판정한다."
    let mut rel = make_rel();
    rel.update_trust(80.0);
    rel.update_affinity(-50.0);
    assert_eq!(rel.level(), RelationshipLevel::Hostile);
}

#[test]
fn is_hostile_checks_negative_affinity() {
    let mut rel = make_rel();
    assert!(!rel.is_hostile());
    assert!(!rel.is_enemy());

    rel.update_affinity(-40.0);
    assert!(rel.is_hostile());
    assert!(!rel.is_enemy());

    rel.update_affinity(-40.0); // total -80
    assert!(rel.is_hostile());
    assert!(rel.is_enemy());
}

// =======================================================================
// 소연 시나리오 테스트
// =======================================================================

#[test]
fn soyeon_quest_trigger_progression() {
    // 소연 퀘스트 라인: 호감도 0 → 30 → 50 → 70 → 80
    let mut rel = make_rel();

    // 첫 만남 — Stranger
    assert_eq!(rel.level(), RelationshipLevel::Stranger);

    // 정보 거래 몇 번 → 호감30, 신뢰10
    rel.update_affinity(30.0);
    rel.update_trust(10.0);
    assert_eq!(rel.level(), RelationshipLevel::Acquaintance);

    // 개방 언급 시점 → 호감55, 신뢰35
    rel.update_affinity(25.0);
    rel.update_trust(25.0);
    assert_eq!(rel.level(), RelationshipLevel::Friendly);

    // 사부의 부탁 시점 → 호감75, 신뢰55
    rel.update_affinity(20.0);
    rel.update_trust(20.0);
    assert_eq!(rel.level(), RelationshipLevel::Close);

    // 진짜 소연 → 호감85, 신뢰75
    rel.update_affinity(10.0);
    rel.update_trust(20.0);
    assert_eq!(rel.level(), RelationshipLevel::Intimate);
}

#[test]
fn soyeon_bond_broken_via_negative_affinity() {
    // 플레이어가 조고 편 → 소연이 적으로 전환 (affinity를 음수로 내림)
    let mut rel = make_rel();
    rel.update_affinity(60.0);
    rel.update_trust(40.0);
    assert_eq!(rel.level(), RelationshipLevel::Friendly);

    // 배신으로 호감도 급락 → -85
    rel.update_affinity(-145.0); // 60 + (-145) = -85
    rel.set_relation_type(Some(RelationshipType::Enemy));
    assert_eq!(rel.level(), RelationshipLevel::Enemy);
    assert_eq!(rel.relation_type(), Some(RelationshipType::Enemy));
}

// =======================================================================
// 상호작용 기록 테스트
// =======================================================================

#[test]
fn record_interaction_updates_count_and_time() {
    let mut rel = make_rel();
    assert_eq!(rel.interaction_count(), 0);
    assert!(rel.last_interaction().is_none());

    let time1 = GameTime::new(1, 1, 1);
    rel.record_interaction(time1);
    assert_eq!(rel.interaction_count(), 1);
    assert_eq!(rel.last_interaction(), Some(time1));

    let time2 = GameTime::new(1, 1, 2);
    rel.record_interaction(time2);
    assert_eq!(rel.interaction_count(), 2);
    assert_eq!(rel.last_interaction(), Some(time2));
}

// =======================================================================
// RelationshipType 이름 테스트
// =======================================================================

#[test]
fn relationship_type_names() {
    assert_eq!(RelationshipType::MasterDisciple.name(), "사제");
    assert_eq!(RelationshipType::Lover.name(), "연인");
    assert_eq!(RelationshipType::Enemy.name(), "적");
    assert_eq!(RelationshipType::Patron.name(), "후원자");
}

// =======================================================================
// RelationshipLevel 이름 테스트
// =======================================================================

#[test]
fn relationship_level_names() {
    assert_eq!(RelationshipLevel::Stranger.name(), "모르는 사이");
    assert_eq!(RelationshipLevel::Intimate.name(), "깊은 유대");
    assert_eq!(RelationshipLevel::Enemy.name(), "원수");
    assert_eq!(RelationshipLevel::Wary.name(), "경계");
}

// =======================================================================
// 경계값(boundary) 테스트
// =======================================================================

#[test]
fn level_boundary_just_below_thresholds() {
    let mut rel = make_rel();
    rel.update_affinity(19.9);
    rel.update_trust(19.9);
    assert_eq!(rel.level(), RelationshipLevel::Stranger);

    rel.update_affinity(0.1); // affinity = 20.0
    assert_eq!(rel.level(), RelationshipLevel::Acquaintance);
}

#[test]
fn wary_boundary_minus10() {
    let mut rel = make_rel();
    rel.update_affinity(-9.9);
    // -9.9 > -10 → Stranger
    assert_eq!(rel.level(), RelationshipLevel::Stranger);

    rel.update_affinity(-0.1); // affinity = -10.0
    // -10.0 <= -10 → Wary
    assert_eq!(rel.level(), RelationshipLevel::Wary);
}

#[test]
fn hostile_boundary_minus40() {
    let mut rel = make_rel();
    rel.update_affinity(-39.9);
    assert_eq!(rel.level(), RelationshipLevel::Wary);

    rel.update_affinity(-0.1); // affinity = -40.0
    assert_eq!(rel.level(), RelationshipLevel::Hostile);
    assert!(rel.is_hostile());
}

#[test]
fn enemy_boundary_minus80() {
    let mut rel = make_rel();
    rel.update_affinity(-79.9);
    assert_eq!(rel.level(), RelationshipLevel::Hostile);
    assert!(!rel.is_enemy());

    rel.update_affinity(-0.1); // affinity = -80.0
    assert_eq!(rel.level(), RelationshipLevel::Enemy);
    assert!(rel.is_enemy());
}

// =======================================================================
// set_relation_type 테스트
// =======================================================================

#[test]
fn set_and_clear_relation_type() {
    let mut rel = make_rel();
    assert!(rel.relation_type().is_none());

    rel.set_relation_type(Some(RelationshipType::Friend));
    assert_eq!(rel.relation_type(), Some(RelationshipType::Friend));

    rel.set_relation_type(None);
    assert!(rel.relation_type().is_none());
}

// =======================================================================
// Serialization roundtrip
// =======================================================================

#[test]
fn serde_roundtrip() {
    let mut rel = make_rel();
    rel.update_affinity(55.0);
    rel.update_trust(30.0);
    rel.set_relation_type(Some(RelationshipType::Friend));

    let json = serde_json::to_string(&rel).expect("serialize");
    let deserialized: Relationship = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(rel, deserialized);
}

#[test]
fn serde_roundtrip_negative_affinity() {
    let mut rel = make_rel();
    rel.update_affinity(-75.0);
    rel.update_trust(10.0);

    let json = serde_json::to_string(&rel).expect("serialize");
    let deserialized: Relationship = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(rel, deserialized);
    assert_eq!(deserialized.affinity(), -75.0);
}

// =======================================================================
// TrustLevel 테스트
// =======================================================================

#[test]
fn trust_level_from_value_boundaries() {
    assert_eq!(TrustLevel::from_value(0.0), TrustLevel::None);
    assert_eq!(TrustLevel::from_value(9.0), TrustLevel::None);
    assert_eq!(TrustLevel::from_value(10.0), TrustLevel::Wary);
    assert_eq!(TrustLevel::from_value(29.0), TrustLevel::Wary);
    assert_eq!(TrustLevel::from_value(30.0), TrustLevel::Cautious);
    assert_eq!(TrustLevel::from_value(49.0), TrustLevel::Cautious);
    assert_eq!(TrustLevel::from_value(50.0), TrustLevel::Considerable);
    assert_eq!(TrustLevel::from_value(69.0), TrustLevel::Considerable);
    assert_eq!(TrustLevel::from_value(70.0), TrustLevel::Deep);
    assert_eq!(TrustLevel::from_value(100.0), TrustLevel::Deep);
}

#[test]
fn trust_level_keys() {
    assert_eq!(TrustLevel::None.key(), "None");
    assert_eq!(TrustLevel::Wary.key(), "Wary");
    assert_eq!(TrustLevel::Cautious.key(), "Cautious");
    assert_eq!(TrustLevel::Considerable.key(), "Considerable");
    assert_eq!(TrustLevel::Deep.key(), "Deep");
}

// =======================================================================
// Relationship 편의 메서드 (trust_level) 테스트
// =======================================================================

#[test]
fn relationship_trust_level() {
    let mut rel = make_rel();
    assert_eq!(rel.trust_level(), TrustLevel::None);

    rel.update_trust(35.0);
    assert_eq!(rel.trust_level(), TrustLevel::Cautious);

    rel.update_trust(40.0); // total 75
    assert_eq!(rel.trust_level(), TrustLevel::Deep);
}

#[test]
fn relationship_level_key() {
    assert_eq!(RelationshipLevel::Stranger.key(), "Stranger");
    assert_eq!(RelationshipLevel::Friendly.key(), "Friendly");
    assert_eq!(RelationshipLevel::Enemy.key(), "Enemy");
    assert_eq!(RelationshipLevel::Wary.key(), "Wary");
}

// =======================================================================
// 소연 시나리오: 전체 구간 연동 테스트
// =======================================================================

#[test]
fn soyeon_scenario_all_levels_combined() {
    // 소연 호감55 신뢰35 → Friendly + Cautious
    let mut rel = make_rel();
    rel.update_affinity(55.0);
    rel.update_trust(35.0);

    assert_eq!(rel.level(), RelationshipLevel::Friendly);
    assert_eq!(rel.level().key(), "Friendly");
    assert_eq!(rel.trust_level(), TrustLevel::Cautious);
    assert_eq!(rel.trust_level().key(), "Cautious");
}

// =======================================================================
// 이벤트 발행 테스트 — mutation 메서드가 올바른 DomainEvent를 반환하는지
// =======================================================================

use crate::relationship::RelationshipEvent;
use crate::shared::event::DomainEvent;

/// DomainEvent에서 RelationshipEvent를 추출하는 헬퍼.
fn unwrap_rel_event(event: &DomainEvent) -> &RelationshipEvent {
    match event {
        DomainEvent::Relationship(e) => e,
        other => panic!("expected Relationship event, got {:?}", other),
    }
}

#[test]
fn update_affinity_emits_affinity_changed() {
    let mut rel = make_rel();
    let events = rel.update_affinity(30.0);
    // AffinityChanged + LevelChanged (Stranger → Acquaintance)
    assert_eq!(events.len(), 2);
    match unwrap_rel_event(&events[0]) {
        RelationshipEvent::AffinityChanged {
            old_value,
            new_value,
            ..
        } => {
            assert_eq!(*old_value, 0.0);
            assert_eq!(*new_value, 30.0);
        }
        other => panic!("expected AffinityChanged, got {:?}", other),
    }
    match unwrap_rel_event(&events[1]) {
        RelationshipEvent::LevelChanged {
            old_level,
            new_level,
            ..
        } => {
            assert_eq!(*old_level, RelationshipLevel::Stranger);
            assert_eq!(*new_level, RelationshipLevel::Acquaintance);
        }
        other => panic!("expected LevelChanged, got {:?}", other),
    }
}

#[test]
fn update_affinity_no_level_change_single_event() {
    // 10 → 15: 둘 다 Stranger, LevelChanged 없어야 함
    let mut rel = make_rel();
    rel.update_affinity(10.0);
    let events = rel.update_affinity(5.0);
    assert_eq!(events.len(), 1);
    assert!(matches!(
        unwrap_rel_event(&events[0]),
        RelationshipEvent::AffinityChanged { .. }
    ));
}

#[test]
fn update_affinity_no_op_returns_empty() {
    // affinity 이미 -100, delta -5 → clamp 후 여전히 -100
    let mut rel = make_rel();
    rel.update_affinity(-100.0);
    let events = rel.update_affinity(-5.0);
    assert!(events.is_empty());
}

#[test]
fn update_affinity_at_max_no_op() {
    let mut rel = make_rel();
    rel.update_affinity(100.0);
    let events = rel.update_affinity(10.0);
    assert!(events.is_empty());
}

#[test]
fn update_trust_emits_trust_changed() {
    let mut rel = make_rel();
    let events = rel.update_trust(25.0);
    assert_eq!(events.len(), 2); // TrustChanged + LevelChanged (Stranger → Acquaintance)
    assert!(matches!(
        unwrap_rel_event(&events[0]),
        RelationshipEvent::TrustChanged { .. }
    ));
    assert!(matches!(
        unwrap_rel_event(&events[1]),
        RelationshipEvent::LevelChanged { .. }
    ));
}

#[test]
fn update_trust_no_op_returns_empty() {
    let mut rel = make_rel();
    let events = rel.update_trust(0.0);
    assert!(events.is_empty());
}

#[test]
fn set_relation_type_emits_type_changed() {
    let mut rel = make_rel();
    let events = rel.set_relation_type(Some(RelationshipType::Friend));
    assert_eq!(events.len(), 1);
    match unwrap_rel_event(&events[0]) {
        RelationshipEvent::TypeChanged {
            old_type, new_type, ..
        } => {
            assert_eq!(*old_type, None);
            assert_eq!(*new_type, Some(RelationshipType::Friend));
        }
        other => panic!("expected TypeChanged, got {:?}", other),
    }
}

#[test]
fn set_relation_type_no_op_returns_empty() {
    let mut rel = make_rel();
    // 이미 None인데 None 설정 → no-op
    let events = rel.set_relation_type(None);
    assert!(events.is_empty());
}

#[test]
fn record_interaction_emits_event() {
    let mut rel = make_rel();
    let time = GameTime::new(1, 1, 1);
    let events = rel.record_interaction(time);
    assert_eq!(events.len(), 1);
    match unwrap_rel_event(&events[0]) {
        RelationshipEvent::InteractionRecorded {
            interaction_count, ..
        } => {
            assert_eq!(*interaction_count, 1);
        }
        other => panic!("expected InteractionRecorded, got {:?}", other),
    }
}

#[test]
fn level_transition_from_friendly_to_hostile_via_affinity() {
    // Friendly → Hostile 전환 via 음수 affinity
    let mut rel = make_rel();
    rel.update_affinity(55.0);
    rel.update_trust(35.0);
    assert_eq!(rel.level(), RelationshipLevel::Friendly);

    // 호감도 급락: 55 + (-100) = -45 → Hostile
    let events = rel.update_affinity(-100.0);
    assert_eq!(events.len(), 2);
    match unwrap_rel_event(&events[1]) {
        RelationshipEvent::LevelChanged {
            old_level,
            new_level,
            ..
        } => {
            assert_eq!(*old_level, RelationshipLevel::Friendly);
            assert_eq!(*new_level, RelationshipLevel::Hostile);
        }
        other => panic!("expected LevelChanged, got {:?}", other),
    }
}

#[test]
fn negative_affinity_emits_affinity_changed() {
    let mut rel = make_rel();
    let events = rel.update_affinity(-30.0);
    assert_eq!(events.len(), 2); // AffinityChanged + LevelChanged (Stranger → Wary)
    match unwrap_rel_event(&events[0]) {
        RelationshipEvent::AffinityChanged {
            old_value,
            new_value,
            ..
        } => {
            assert_eq!(*old_value, 0.0);
            assert_eq!(*new_value, -30.0);
        }
        other => panic!("expected AffinityChanged, got {:?}", other),
    }
    match unwrap_rel_event(&events[1]) {
        RelationshipEvent::LevelChanged {
            old_level,
            new_level,
            ..
        } => {
            assert_eq!(*old_level, RelationshipLevel::Stranger);
            assert_eq!(*new_level, RelationshipLevel::Wary);
        }
        other => panic!("expected LevelChanged, got {:?}", other),
    }
}
