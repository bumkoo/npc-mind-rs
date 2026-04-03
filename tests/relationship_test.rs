//! Relationship 도메인 모델 테스트
//!
//! 3축(closeness, trust, power) 기본 기능 + 감정 엔진 연동 + 대화 후 갱신

mod common;

use common::{TestContext, score as s};
use npc_mind::domain::emotion::EmotionState;
use npc_mind::domain::relationship::*;

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
    assert!(multiplier > 1.4);
}

#[test]
fn 무관한_사람_감정_배율은_기본() {
    let rel = Relationship::neutral("mu_baek", "stranger");
    let multiplier = rel.emotion_intensity_multiplier();
    assert!((multiplier - 1.0).abs() < 0.001);
}

#[test]
fn 적대_관계는_감정_절제() {
    let rel = RelationshipBuilder::new("gyo_ryong", "enemy")
        .closeness(s(-0.8))
        .build();
    let multiplier = rel.emotion_intensity_multiplier();
    assert!(multiplier < 1.0);
    assert!(multiplier > 0.5);
}

// ===========================================================================
// 신뢰도 배율 (trust)
// ===========================================================================

#[test]
fn 신뢰하는_사이_감정_증폭() {
    let rel = RelationshipBuilder::new("mu_baek", "trusted")
        .trust(s(0.8))
        .build();
    let modifier = rel.trust_emotion_modifier();
    assert!(modifier > 1.0);
    assert!((modifier - 1.24).abs() < 0.001);
}

#[test]
fn 불신하는_사이_감정_약화() {
    let rel = RelationshipBuilder::new("mu_baek", "distrusted")
        .trust(s(-0.5))
        .build();
    let modifier = rel.trust_emotion_modifier();
    assert!(modifier < 1.0);
    assert!((modifier - 0.85).abs() < 0.001);
}

#[test]
fn 중립_trust_배율_기본값() {
    let rel = Relationship::neutral("mu_baek", "stranger");
    let modifier = rel.trust_emotion_modifier();
    assert!((modifier - 1.0).abs() < 0.001);
}

#[test]
fn 극신뢰_배율_상한() {
    let rel = RelationshipBuilder::new("mu_baek", "soulmate")
        .trust(s(1.0))
        .build();
    let modifier = rel.trust_emotion_modifier();
    assert!((modifier - 1.3).abs() < 0.001);
}

#[test]
fn 극불신_배율_하한() {
    let rel = RelationshipBuilder::new("mu_baek", "nemesis")
        .trust(s(-1.0))
        .build();
    let modifier = rel.trust_emotion_modifier();
    assert!((modifier - 0.7).abs() < 0.001);
}

// ===========================================================================
// 대화 후 갱신
// ===========================================================================

#[test]
fn 대화후_부정_valence이면_closeness_감소() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .closeness(s(0.5))
        .build();
    let updated = rel.with_updated_closeness(-0.7, 0.0);
    assert!(updated.closeness().value() < rel.closeness().value());
}

#[test]
fn 대화후_긍정_valence이면_closeness_증가() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .closeness(s(0.0))
        .build();
    let updated = rel.with_updated_closeness(0.8, 0.0);
    assert!(updated.closeness().value() > rel.closeness().value());
}

#[test]
fn 대화후_부정_감정이면_closeness_하락() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .closeness(s(0.5))
        .build();
    let updated = rel.with_updated_closeness(-0.6, 0.0);
    assert!(updated.closeness().value() < rel.closeness().value());
}

#[test]
fn 대화후_긍정_감정이면_closeness_상승() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .closeness(s(0.0))
        .build();
    let updated = rel.with_updated_closeness(0.7, 0.0);
    assert!(updated.closeness().value() > rel.closeness().value());
}

#[test]
fn closeness_갱신은_매우_점진적() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .closeness(s(0.5))
        .build();
    let updated = rel.with_updated_closeness(-0.6, 0.0);
    let expected = 0.5 - 0.03;
    assert!((updated.closeness().value() - expected).abs() < 0.001);
}

#[test]
fn closeness_갱신은_점진적_속도() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .closeness(s(0.5))
        .build();
    let updated = rel.with_updated_closeness(-0.7, 0.0);
    // delta = -0.7 * CLOSENESS_UPDATE_RATE(0.05) = -0.035
    let expected = 0.5 - 0.035;
    assert!((updated.closeness().value() - expected).abs() < 0.001);
}

#[test]
fn 원본_불변_검증() {
    let original = RelationshipBuilder::new("mu_baek", "target")
        .trust(s(0.5))
        .closeness(s(0.5))
        .build();
    let _updated = original.with_updated_closeness(-0.7, 0.0);

    assert!((original.trust().value() - 0.5).abs() < 0.001);
    assert!((original.closeness().value() - 0.5).abs() < 0.001);
}

#[test]
fn 갱신_체이닝() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .closeness(s(0.0))
        .build();
    let updated = rel
        .with_updated_closeness(0.5, 0.0)
        .with_updated_closeness(0.3, 0.0);

    assert!(updated.closeness().value() > 0.0);
    assert_eq!(rel.closeness().value(), 0.0);
}

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
    let _ctx = TestContext::new();
    let rel = RelationshipBuilder::new("mu_baek", "gyo_ryong")
        .closeness(s(0.8))
        .trust(s(0.3))
        .power(s(0.0))
        .build();

    assert!(rel.emotion_intensity_multiplier() > 1.3);
    assert!(rel.trust_emotion_modifier() > 1.0);
}

#[test]
fn 교룡의_숙적_관계() {
    let _ctx = TestContext::new();
    let rel = RelationshipBuilder::new("gyo_ryong", "enemy")
        .closeness(s(-0.7))
        .trust(s(-0.8))
        .power(s(0.0))
        .build();

    assert!(rel.emotion_intensity_multiplier() < 1.0);
    assert!(rel.trust_emotion_modifier() < 1.0);
}

#[test]
fn 수련과_사부_관계() {
    let _ctx = TestContext::new();
    let rel = RelationshipBuilder::new("shu_lien", "master")
        .closeness(s(0.9))
        .trust(s(0.9))
        .power(s(-0.7))
        .build();

    assert!(rel.emotion_intensity_multiplier() > 1.4);
    assert!(rel.power().value() <= -0.4);
    assert!(rel.trust_emotion_modifier() > 1.2);
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
    let rel = RelationshipBuilder::new("mu_baek", "gyo_ryong").build();
    let json = serde_json::to_string(&rel).unwrap();
    assert!(json.contains("mu_baek"));
    assert!(json.contains("gyo_ryong"));
}

#[test]
fn after_dialogue_종합_갱신() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .closeness(s(0.0))
        .trust(s(0.0))
        .build();

    let mut state = EmotionState::new();
    state.add(npc_mind::domain::emotion::Emotion::new(
        npc_mind::domain::emotion::EmotionType::Joy,
        0.8,
    ));

    let updated = rel.after_dialogue(&state, 0.5);
    assert!(updated.closeness().value() > 0.0);
}

// ===========================================================================
// 이슈 2: significance 배율 검증
// ===========================================================================

#[test]
fn significance_0이면_기본_변동() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .closeness(s(0.0))
        .build();
    let updated = rel.with_updated_closeness(0.8, 0.0);
    let expected = 0.8 * 0.05; // valence × CLOSENESS_UPDATE_RATE
    assert!((updated.closeness().value() - expected).abs() < 0.001);
}

#[test]
fn significance_1이면_4배_변동() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .closeness(s(0.0))
        .build();
    let updated = rel.with_updated_closeness(0.8, 1.0);
    let expected = 0.8 * 0.05 * 4.0; // valence × CLOSENESS_UPDATE_RATE × (1 + 1.0 × 3.0)
    assert!((updated.closeness().value() - expected).abs() < 0.001);
}

#[test]
fn significance_closeness에도_적용() {
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .closeness(s(0.0))
        .build();
    let base = rel.with_updated_closeness(0.5, 0.0);
    let amplified = rel.with_updated_closeness(0.5, 1.0);
    // amplified = base × 4배
    assert!((amplified.closeness().value() / base.closeness().value() - 4.0).abs() < 0.01);
}
