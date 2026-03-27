//! Relationship 도메인 모델 테스트
//!
//! 3축(closeness, trust, power) 기본 기능 + 감정 엔진 연동 + 대화 후 갱신
//! Relationship은 Value Object — 갱신 시 새 인스턴스 반환

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
    let rel = Relationship::neutral("npc_a", "mu_baek");
    assert_eq!(rel.owner_id(), "npc_a");
    assert_eq!(rel.target_id(), "mu_baek");
    assert_eq!(rel.closeness().value(), 0.0);
    assert_eq!(rel.trust().value(), 0.0);
    assert_eq!(rel.power().value(), 0.0);
}

#[test]
fn 빌더로_관계_생성() {
    let rel = RelationshipBuilder::new("mu_baek", "gyo_ryong")
        .closeness(s(0.9))
        .trust(s(0.8))
        .power(s(0.0))
        .build();
    assert_eq!(rel.owner_id(), "mu_baek");
    assert_eq!(rel.target_id(), "gyo_ryong");
    assert!((rel.closeness().value() - 0.9).abs() < 0.001);
    assert!((rel.trust().value() - 0.8).abs() < 0.001);
    assert_eq!(rel.power().value(), 0.0);
}

#[test]
fn 적대_관계_생성() {
    let rel = RelationshipBuilder::new("gyo_ryong", "enemy")
        .closeness(s(-0.8))
        .trust(s(-0.5))
        .power(s(-0.3))
        .build();
    assert!(rel.closeness().value() <= -0.4);
    assert!(rel.trust().value() < 0.0);
}

// ===========================================================================
// 감정 배율 (closeness)
// ===========================================================================

#[test]
fn 의형제_감정_배율이_높음() {
    let rel = RelationshipBuilder::new("mu_baek", "brother")
        .closeness(s(0.9))
        .build();
    let multiplier = rel.emotion_intensity_multiplier();
    // 1.0 + 0.9 * 0.5 = 1.45
    assert!(multiplier > 1.4);
}

#[test]
fn 무관한_사람_감정_배율은_기본() {
    let rel = Relationship::neutral("mu_baek", "stranger");
    let multiplier = rel.emotion_intensity_multiplier();
    // 1.0 + 0.0 * 0.5 = 1.0
    assert!((multiplier - 1.0).abs() < 0.001);
}

#[test]
fn 적대_관계는_감정_절제() {
    let rel = RelationshipBuilder::new("gyo_ryong", "enemy")
        .closeness(s(-0.8))
        .build();
    let multiplier = rel.emotion_intensity_multiplier();
    // 1.0 + (-0.8) * 0.5 = 0.6 — 원수 앞에선 감정 절제/경계
    assert!(multiplier < 1.0, "적대 관계는 감정 억제: {}", multiplier);
    assert!(multiplier > 0.5, "완전 무감각은 아님: {}", multiplier);
}

// ===========================================================================
// 신뢰도 배율 (trust)
//
// trust_emotion_modifier = 1.0 + trust × 0.3
// 신뢰하는 사람의 행동에 더 강하게 반응하고,
// 불신하는 사람의 행동에 덜 반응한다.
// ===========================================================================

#[test]
fn 신뢰하는_사이_감정_증폭() {
    let rel = RelationshipBuilder::new("mu_baek", "trusted")
        .trust(s(0.8))
        .build();
    let modifier = rel.trust_emotion_modifier();
    // 1.0 + 0.8 * 0.3 = 1.24
    assert!(modifier > 1.0,
        "신뢰하는 사이 → 감정 증폭: {}", modifier);
    assert!((modifier - 1.24).abs() < 0.001,
        "정확한 값: {}", modifier);
}

#[test]
fn 불신하는_사이_감정_약화() {
    let rel = RelationshipBuilder::new("mu_baek", "distrusted")
        .trust(s(-0.5))
        .build();
    let modifier = rel.trust_emotion_modifier();
    // 1.0 + (-0.5) * 0.3 = 0.85
    assert!(modifier < 1.0,
        "불신하는 사이 → 감정 약화: {}", modifier);
    assert!((modifier - 0.85).abs() < 0.001,
        "정확한 값: {}", modifier);
}

#[test]
fn 중립_trust_배율_기본값() {
    let rel = Relationship::neutral("mu_baek", "stranger");
    let modifier = rel.trust_emotion_modifier();
    // 1.0 + 0.0 * 0.3 = 1.0
    assert!((modifier - 1.0).abs() < 0.001,
        "중립 trust → 배율 1.0: {}", modifier);
}

#[test]
fn 극신뢰_배율_상한() {
    let rel = RelationshipBuilder::new("mu_baek", "soulmate")
        .trust(s(1.0))
        .build();
    let modifier = rel.trust_emotion_modifier();
    // 1.0 + 1.0 * 0.3 = 1.3
    assert!((modifier - 1.3).abs() < 0.001,
        "극신뢰 → 최대 1.3: {}", modifier);
}

#[test]
fn 극불신_배율_하한() {
    let rel = RelationshipBuilder::new("mu_baek", "nemesis")
        .trust(s(-1.0))
        .build();
    let modifier = rel.trust_emotion_modifier();
    // 1.0 + (-1.0) * 0.3 = 0.7
    assert!((modifier - 0.7).abs() < 0.001,
        "극불신 → 최소 0.7: {}", modifier);
}

// ===========================================================================
// 대화 후 갱신 (Value Object — 새 인스턴스 반환)
// ===========================================================================

#[test]
fn 대화후_배신하면_trust_하락() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .trust(s(0.5))
        .build();
    let updated = rel.with_updated_trust(-0.7);
    assert!(updated.trust().value() < rel.trust().value(),
        "배신 후 trust 하락: {} → {}", rel.trust().value(), updated.trust().value());
}

#[test]
fn 대화후_의로운_행동하면_trust_상승() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .trust(s(0.0))
        .build();
    let updated = rel.with_updated_trust(0.8);
    assert!(updated.trust().value() > rel.trust().value(),
        "의로운 행동 후 trust 상승: {} → {}", rel.trust().value(), updated.trust().value());
}

#[test]
fn 대화후_부정_감정이면_closeness_하락() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .closeness(s(0.5))
        .build();
    let updated = rel.with_updated_closeness(-0.6);
    assert!(updated.closeness().value() < rel.closeness().value(),
        "부정 대화 후 closeness 하락: {} → {}",
        rel.closeness().value(), updated.closeness().value());
}

#[test]
fn 대화후_긍정_감정이면_closeness_상승() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .closeness(s(0.0))
        .build();
    let updated = rel.with_updated_closeness(0.7);
    assert!(updated.closeness().value() > rel.closeness().value(),
        "긍정 대화 후 closeness 상승: {} → {}",
        rel.closeness().value(), updated.closeness().value());
}

#[test]
fn closeness_갱신은_매우_점진적() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .closeness(s(0.5))
        .build();
    let updated = rel.with_updated_closeness(-0.6);
    let expected = 0.5 - 0.03;
    assert!((updated.closeness().value() - expected).abs() < 0.001,
        "점진적 변화: {}", updated.closeness().value());
}

#[test]
fn trust_갱신은_중간_속도() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .trust(s(0.5))
        .build();
    let updated = rel.with_updated_trust(-0.7);
    let expected = 0.5 - 0.07;
    assert!((updated.trust().value() - expected).abs() < 0.001,
        "중간 속도 변화: {}", updated.trust().value());
}

#[test]
fn 원본_불변_검증() {
    let original = RelationshipBuilder::new("mu_baek", "target")
        .trust(s(0.5))
        .closeness(s(0.5))
        .build();
    let _updated = original.with_updated_trust(-0.7);

    assert!((original.trust().value() - 0.5).abs() < 0.001,
        "원본 trust 불변: {}", original.trust().value());
    assert!((original.closeness().value() - 0.5).abs() < 0.001,
        "원본 closeness 불변: {}", original.closeness().value());
}

#[test]
fn 갱신_체이닝() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .trust(s(0.0))
        .closeness(s(0.0))
        .build();
    let updated = rel
        .with_updated_trust(0.8)
        .with_updated_closeness(0.5);

    assert!(updated.trust().value() > 0.0);
    assert!(updated.closeness().value() > 0.0);
    assert_eq!(rel.trust().value(), 0.0);
    assert_eq!(rel.closeness().value(), 0.0);
}

// ===========================================================================
// power 설정 (Value Object — 새 인스턴스 반환)
// ===========================================================================

#[test]
fn power_게임이벤트로_직접_설정() {
    let rel = RelationshipBuilder::new("shu_lien", "target")
        .power(s(0.0))
        .build();
    assert_eq!(rel.power().value(), 0.0);
    let updated = rel.with_power(s(0.8));
    assert!((updated.power().value() - 0.8).abs() < 0.001);
    assert_eq!(rel.power().value(), 0.0);
}

// ===========================================================================
// 무협 시나리오: 4인 캐릭터 관계
// ===========================================================================

#[test]
fn 무백과_교룡_의형제_관계() {
    let rel = RelationshipBuilder::new("mu_baek", "gyo_ryong")
        .closeness(s(0.8))
        .trust(s(0.3))
        .power(s(0.0))
        .build();

    // 가까운 사이 → 감정 배율 높음
    assert!(rel.emotion_intensity_multiplier() > 1.3);
    // trust 양수 → 행동에 대한 감정 증폭
    assert!(rel.trust_emotion_modifier() > 1.0,
        "의형제라 trust 양수 → 감정 증폭: {}", rel.trust_emotion_modifier());
}

#[test]
fn 교룡의_숙적_관계() {
    let rel = RelationshipBuilder::new("gyo_ryong", "enemy")
        .closeness(s(-0.7))
        .trust(s(-0.8))
        .power(s(0.0))
        .build();

    // 적대적이면 전반적 감정 절제 (경계/억제)
    assert!(rel.emotion_intensity_multiplier() < 1.0,
        "숙적 앞에선 감정 절제: {}", rel.emotion_intensity_multiplier());
    // trust 음수 → 행동에 대한 감정 약화 ("역시나")
    assert!(rel.trust_emotion_modifier() < 1.0,
        "숙적이라 trust 음수 → 감정 약화: {}", rel.trust_emotion_modifier());
}

#[test]
fn 수련과_사부_관계() {
    let rel = RelationshipBuilder::new("shu_lien", "master")
        .closeness(s(0.9))
        .trust(s(0.9))
        .power(s(-0.7))
        .build();

    assert!(rel.emotion_intensity_multiplier() > 1.4);
    assert!(rel.power().value() <= -0.4);
    // 극신뢰 → 행동에 대한 감정 강하게 증폭
    assert!(rel.trust_emotion_modifier() > 1.2,
        "사부를 극신뢰 → 강한 증폭: {}", rel.trust_emotion_modifier());
}

#[test]
fn 직렬화_역직렬화() {
    let rel = RelationshipBuilder::new("mu_baek", "gyo_ryong")
        .closeness(s(0.8))
        .trust(s(0.3))
        .power(s(0.0))
        .build();

    let json = serde_json::to_string(&rel).unwrap();
    let deserialized: Relationship = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.owner_id(), "mu_baek");
    assert_eq!(deserialized.target_id(), "gyo_ryong");
    assert!((deserialized.closeness().value() - 0.8).abs() < 0.001);
}

#[test]
fn owner_id_직렬화에_포함() {
    let rel = RelationshipBuilder::new("mu_baek", "gyo_ryong")
        .build();
    let json = serde_json::to_string(&rel).unwrap();
    assert!(json.contains("mu_baek"), "JSON에 owner_id 포함: {}", json);
    assert!(json.contains("gyo_ryong"), "JSON에 target_id 포함: {}", json);
}
