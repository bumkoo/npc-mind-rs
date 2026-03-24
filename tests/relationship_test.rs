//! Relationship 도메인 모델 테스트
//!
//! 3축(closeness, trust, power) 기본 기능 + 감정 엔진 연동 + 대화 후 갱신

mod common;

use npc_mind::domain::personality::Score;
use npc_mind::domain::relationship::*;

fn s(v: f32) -> Score {
    Score::new(v, "test").unwrap()
}

// ===========================================================================
// 기본 생성
// ===========================================================================

#[test]
fn 중립_관계_생성() {
    let rel = Relationship::neutral("mu_baek");
    assert_eq!(rel.target_id(), "mu_baek");
    assert_eq!(rel.closeness().value(), 0.0);
    assert_eq!(rel.trust().value(), 0.0);
    assert_eq!(rel.power().value(), 0.0);
}

#[test]
fn 빌더로_관계_생성() {
    let rel = RelationshipBuilder::new("gyo_ryong")
        .closeness(s(0.9))
        .trust(s(0.8))
        .power(s(0.0))
        .build();
    assert_eq!(rel.target_id(), "gyo_ryong");
    assert!((rel.closeness().value() - 0.9).abs() < 0.001);
    assert!((rel.trust().value() - 0.8).abs() < 0.001);
    assert_eq!(rel.power().value(), 0.0);
}

#[test]
fn 적대_관계_생성() {
    let rel = RelationshipBuilder::new("enemy")
        .closeness(s(-0.8))
        .trust(s(-0.5))
        .power(s(-0.3))
        .build();
    assert!(rel.closeness().is_low());
    assert!(rel.trust().is_negative());
}

// ===========================================================================
// 감정 배율 (closeness)
// ===========================================================================

#[test]
fn 의형제_감정_배율이_높음() {
    let rel = RelationshipBuilder::new("brother")
        .closeness(s(0.9))
        .build();
    let multiplier = rel.emotion_intensity_multiplier();
    // 1.0 + 0.9 * 0.5 = 1.45
    assert!(multiplier > 1.4);
}

#[test]
fn 무관한_사람_감정_배율은_기본() {
    let rel = Relationship::neutral("stranger");
    let multiplier = rel.emotion_intensity_multiplier();
    // 1.0 + 0.0 * 0.5 = 1.0
    assert!((multiplier - 1.0).abs() < 0.001);
}

#[test]
fn 적대_관계도_감정_배율이_높음() {
    // 적대(-0.8)도 절대값이 크므로 감정 반응 강함
    let rel = RelationshipBuilder::new("enemy")
        .closeness(s(-0.8))
        .build();
    let multiplier = rel.emotion_intensity_multiplier();
    // 1.0 + 0.8 * 0.5 = 1.4
    assert!(multiplier > 1.3);
}

// ===========================================================================
// 기대 위반 (trust)
// ===========================================================================

#[test]
fn 신뢰하던_상대의_배신은_기대_위반_극대() {
    let rel = RelationshipBuilder::new("trusted")
        .trust(s(0.8))
        .build();
    // trust 0.8인데 배신 praiseworthiness -0.7 → 차이 1.5
    let violation = rel.expectation_violation(-0.7);
    assert!(violation > 1.4, "기대 위반 극대: {}", violation);
}

#[test]
fn 불신하던_상대의_배신은_기대_부합() {
    let rel = RelationshipBuilder::new("distrusted")
        .trust(s(-0.5))
        .build();
    // trust -0.5인데 배신 praiseworthiness -0.7 → 차이 0.2
    let violation = rel.expectation_violation(-0.7);
    assert!(violation < 0.3, "기대 부합 (역시나): {}", violation);
}

#[test]
fn 불신하던_상대의_도움은_기대_위반() {
    let rel = RelationshipBuilder::new("distrusted")
        .trust(s(-0.5))
        .build();
    // trust -0.5인데 도움 praiseworthiness 0.7 → 차이 1.2
    let violation = rel.expectation_violation(0.7);
    assert!(violation > 1.0, "기대 위반 (예상 밖 도움): {}", violation);
}

#[test]
fn trust_감정_modifier_기대_위반시_증폭() {
    let rel = RelationshipBuilder::new("trusted")
        .trust(s(0.8))
        .build();
    let modifier = rel.trust_emotion_modifier(-0.7);
    // violation 1.5 → 0.5 + 1.5 * 0.5 = 1.25
    assert!(modifier > 1.0, "기대 위반 → 감정 증폭: {}", modifier);
}

#[test]
fn trust_감정_modifier_기대_부합시_완화() {
    let rel = RelationshipBuilder::new("distrusted")
        .trust(s(-0.5))
        .build();
    let modifier = rel.trust_emotion_modifier(-0.7);
    // violation 0.2 → 0.5 + 0.2 * 0.5 = 0.6
    assert!(modifier < 1.0, "기대 부합 → 감정 완화: {}", modifier);
}

// ===========================================================================
// 대화 후 갱신
// ===========================================================================

#[test]
fn 대화후_배신하면_trust_하락() {
    let mut rel = RelationshipBuilder::new("target")
        .trust(s(0.5))
        .build();
    let before = rel.trust().value();
    rel.update_trust(-0.7);  // 배신 행위
    let after = rel.trust().value();
    assert!(after < before, "배신 후 trust 하락: {} → {}", before, after);
}

#[test]
fn 대화후_의로운_행동하면_trust_상승() {
    let mut rel = RelationshipBuilder::new("target")
        .trust(s(0.0))
        .build();
    let before = rel.trust().value();
    rel.update_trust(0.8);  // 의로운 행위
    let after = rel.trust().value();
    assert!(after > before, "의로운 행동 후 trust 상승: {} → {}", before, after);
}

#[test]
fn 대화후_부정_감정이면_closeness_하락() {
    let mut rel = RelationshipBuilder::new("target")
        .closeness(s(0.5))
        .build();
    let before = rel.closeness().value();
    rel.update_closeness(-0.6);  // 부정적 대화 결과
    let after = rel.closeness().value();
    assert!(after < before, "부정 대화 후 closeness 하락: {} → {}", before, after);
}

#[test]
fn 대화후_긍정_감정이면_closeness_상승() {
    let mut rel = RelationshipBuilder::new("target")
        .closeness(s(0.0))
        .build();
    let before = rel.closeness().value();
    rel.update_closeness(0.7);  // 긍정적 대화 결과
    let after = rel.closeness().value();
    assert!(after > before, "긍정 대화 후 closeness 상승: {} → {}", before, after);
}

#[test]
fn closeness_갱신은_매우_점진적() {
    let mut rel = RelationshipBuilder::new("target")
        .closeness(s(0.5))
        .build();
    rel.update_closeness(-0.6);
    // CLOSENESS_UPDATE_RATE = 0.05 → -0.6 × 0.05 = -0.03
    let expected = 0.5 - 0.03;
    assert!((rel.closeness().value() - expected).abs() < 0.001,
        "점진적 변화: {}", rel.closeness().value());
}

#[test]
fn trust_갱신은_중간_속도() {
    let mut rel = RelationshipBuilder::new("target")
        .trust(s(0.5))
        .build();
    rel.update_trust(-0.7);
    // TRUST_UPDATE_RATE = 0.1 → -0.7 × 0.1 = -0.07
    let expected = 0.5 - 0.07;
    assert!((rel.trust().value() - expected).abs() < 0.001,
        "중간 속도 변화: {}", rel.trust().value());
}

// ===========================================================================
// power 설정
// ===========================================================================

#[test]
fn power_게임이벤트로_직접_설정() {
    let mut rel = RelationshipBuilder::new("target")
        .power(s(0.0))
        .build();
    assert_eq!(rel.power().value(), 0.0);
    rel.set_power(s(0.8));  // 무림맹주 추대
    assert!((rel.power().value() - 0.8).abs() < 0.001);
}

// ===========================================================================
// 무협 시나리오: 4인 캐릭터 관계
// ===========================================================================

#[test]
fn 무백과_교룡_의형제_관계() {
    // 무백 → 교룡: 의형제이지만 교룡을 완전히 믿지는 못함
    let rel = RelationshipBuilder::new("gyo_ryong")
        .closeness(s(0.8))
        .trust(s(0.3))
        .power(s(0.0))  // 대등
        .build();

    // 가까운 사이 → 감정 배율 높음
    assert!(rel.emotion_intensity_multiplier() > 1.3);
    // trust 약간 양수인데 배신(-0.7) → 기대 위반
    assert!(rel.expectation_violation(-0.7) > 0.9);
}

#[test]
fn 교룡의_숙적_관계() {
    // 교룡 → 숙적: 적대적이고 불신
    let rel = RelationshipBuilder::new("enemy")
        .closeness(s(-0.7))
        .trust(s(-0.8))
        .power(s(0.0))
        .build();

    // 적대적이지만 감정 배율은 높음 (관심이 있으니까)
    assert!(rel.emotion_intensity_multiplier() > 1.3);
    // 불신(-0.8)인데 배신(-0.7) → 기대 부합 (역시나)
    let violation = rel.expectation_violation(-0.7);
    assert!(violation < 0.2, "숙적의 배신은 예상된 것: {}", violation);
}

#[test]
fn 수련과_사부_관계() {
    // 수련 → 사부: 매우 가까운 사이, 완전 신뢰, 하위
    let rel = RelationshipBuilder::new("master")
        .closeness(s(0.9))
        .trust(s(0.9))
        .power(s(-0.7))  // 사부가 상위
        .build();

    assert!(rel.emotion_intensity_multiplier() > 1.4);
    assert!(rel.power().is_low());  // 하위 관계
}

#[test]
fn 직렬화_역직렬화() {
    let rel = RelationshipBuilder::new("mu_baek")
        .closeness(s(0.8))
        .trust(s(0.3))
        .power(s(0.0))
        .build();

    let json = serde_json::to_string(&rel).unwrap();
    let deserialized: Relationship = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.target_id(), "mu_baek");
    assert!((deserialized.closeness().value() - 0.8).abs() < 0.001);
}
