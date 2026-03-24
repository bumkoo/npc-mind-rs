//! 대화 후 Relationship 갱신 통합 테스트
//!
//! 전체 대화 흐름: appraise → apply_stimulus → update_after_dialogue
//! 대화 결과에 따라 관계가 변하는지 검증

mod common;

use npc_mind::domain::emotion::*;
use npc_mind::domain::pad::Pad;
use npc_mind::domain::relationship::*;
use common::{make_무백, make_교룡, score as s};

fn find_emotion(state: &EmotionState, etype: EmotionType) -> Option<f32> {
    state.emotions().iter()
        .find(|e| e.emotion_type() == etype)
        .map(|e| e.intensity())
}

// ===========================================================================
// 배신 대화 → trust 하락
// ===========================================================================

#[test]
fn 배신_대화_후_trust_하락() {
    let yu = make_교룡();
    let mut rel = RelationshipBuilder::new("mu_baek")
        .closeness(s(0.8))
        .trust(s(0.5))
        .build();

    let situation = Situation {
        description: "동료의 배신".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.7,
            outcome_for_self: Some(-0.6),
        },
    };

    // 1. 상황 진입
    let state = AppraisalEngine::appraise(yu.personality(), &situation, &rel);

    // 2. 대화 중 도발 3턴
    let provocation = Pad::new(-0.6, 0.7, 0.5);
    let state1 = StimulusEngine::apply_stimulus(yu.personality(), &state, &provocation);
    let state2 = StimulusEngine::apply_stimulus(yu.personality(), &state1, &provocation);
    let final_state = StimulusEngine::apply_stimulus(yu.personality(), &state2, &provocation);

    // 3. 대화 종료 → 관계 갱신
    let trust_before = rel.trust().value();
    rel.update_after_dialogue(&final_state, &situation);
    let trust_after = rel.trust().value();

    assert!(trust_after < trust_before,
        "배신 대화 후 trust 하락: {} → {}", trust_before, trust_after);
}

// ===========================================================================
// 부정 대화 → closeness 하락
// ===========================================================================

#[test]
fn 부정_대화_후_closeness_하락() {
    let yu = make_교룡();
    let mut rel = RelationshipBuilder::new("mu_baek")
        .closeness(s(0.5))
        .build();

    let situation = Situation {
        description: "갈등".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.5,
            outcome_for_self: Some(-0.4),
        },
    };

    let state = AppraisalEngine::appraise(yu.personality(), &situation, &rel);

    // 대화 종료 (자극 없이 바로 갱신)
    let closeness_before = rel.closeness().value();
    rel.update_after_dialogue(&state, &situation);
    let closeness_after = rel.closeness().value();

    assert!(closeness_after < closeness_before,
        "부정 대화 후 closeness 하락: {} → {}", closeness_before, closeness_after);
}

// ===========================================================================
// 긍정 대화 → closeness 상승
// ===========================================================================

#[test]
fn 긍정_대화_후_closeness_상승() {
    let li = make_무백();
    let mut rel = RelationshipBuilder::new("friend")
        .closeness(s(0.3))
        .build();

    let situation = Situation {
        description: "좋은 소식".into(),
        focus: SituationFocus::Event {
            desirability_for_self: 0.7,
            desirability_for_other: None,
            is_prospective: false,
            prior_expectation: None,
        },
    };

    let state = AppraisalEngine::appraise(li.personality(), &situation, &rel);

    let closeness_before = rel.closeness().value();
    rel.update_after_dialogue(&state, &situation);
    let closeness_after = rel.closeness().value();

    assert!(closeness_after > closeness_before,
        "긍정 대화 후 closeness 상승: {} → {}", closeness_before, closeness_after);
}

// ===========================================================================
// Event 분기는 trust 미갱신
// ===========================================================================

#[test]
fn event_분기는_trust_변경_없음() {
    let li = make_무백();
    let mut rel = RelationshipBuilder::new("target")
        .trust(s(0.5))
        .build();

    let situation = Situation {
        description: "적 대군 접근".into(),
        focus: SituationFocus::Event {
            desirability_for_self: -0.7,
            desirability_for_other: None,
            is_prospective: true,
            prior_expectation: None,
        },
    };

    let state = AppraisalEngine::appraise(li.personality(), &situation, &rel);

    let trust_before = rel.trust().value();
    rel.update_after_dialogue(&state, &situation);
    let trust_after = rel.trust().value();

    assert!((trust_before - trust_after).abs() < 0.001,
        "Event 분기 → trust 미변경: {} → {}", trust_before, trust_after);
}

// ===========================================================================
// power는 대화로 변하지 않음
// ===========================================================================

#[test]
fn 대화_후_power_변경_없음() {
    let yu = make_교룡();
    let mut rel = RelationshipBuilder::new("master")
        .power(s(-0.7))
        .build();

    let situation = Situation {
        description: "갈등".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.7,
            outcome_for_self: Some(-0.6),
        },
    };

    let state = AppraisalEngine::appraise(yu.personality(), &situation, &rel);
    rel.update_after_dialogue(&state, &situation);

    assert!((rel.power().value() - -0.7).abs() < 0.001,
        "power는 대화로 변하지 않음: {}", rel.power().value());
}

// ===========================================================================
// 전체 시나리오: 무백-교룡 의형제 배신 → 관계 악화
// ===========================================================================

#[test]
fn 시나리오_의형제_배신_후_관계_악화() {
    let yu = make_교룡();

    // 초기: 의형제 관계
    let mut rel = RelationshipBuilder::new("mu_baek")
        .closeness(s(0.8))
        .trust(s(0.7))
        .power(s(0.0))
        .build();

    let situation = Situation {
        description: "무백이 교룡의 검을 빼앗아 관에 넘겼다".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.8,
            outcome_for_self: Some(-0.7),
        },
    };

    // 1. 상황 진입 — 의형제의 배신이라 감정 극대
    let initial_state = AppraisalEngine::appraise(yu.personality(), &situation, &rel);
    let anger = find_emotion(&initial_state, EmotionType::Anger).unwrap();
    assert!(anger > 0.5, "의형제 배신 → 강한 분노: {}", anger);

    // 2. 대화 — 도발 3턴으로 분노 증폭
    let provocation = Pad::new(-0.7, 0.8, 0.6);
    let s1 = StimulusEngine::apply_stimulus(yu.personality(), &initial_state, &provocation);
    let s2 = StimulusEngine::apply_stimulus(yu.personality(), &s1, &provocation);
    let final_state = StimulusEngine::apply_stimulus(yu.personality(), &s2, &provocation);

    // 3. 대화 종료 → 관계 갱신
    let trust_before = rel.trust().value();
    let closeness_before = rel.closeness().value();

    rel.update_after_dialogue(&final_state, &situation);

    let trust_after = rel.trust().value();
    let closeness_after = rel.closeness().value();

    // 검증: trust 급락 (배신 praiseworthiness -0.8)
    assert!(trust_after < trust_before,
        "trust 급락: {} → {}", trust_before, trust_after);
    assert!(trust_before - trust_after > 0.05,
        "trust 하락폭이 유의미: delta={}", trust_before - trust_after);

    // 검증: closeness도 하락 (부정 감정 valence)
    assert!(closeness_after < closeness_before,
        "closeness 하락: {} → {}", closeness_before, closeness_after);

    // 검증: power는 불변
    assert!((rel.power().value() - 0.0).abs() < 0.001);

    println!("=== 의형제 배신 시나리오 ===");
    println!("trust: {} → {}", trust_before, trust_after);
    println!("closeness: {} → {}", closeness_before, closeness_after);
}

// ===========================================================================
// 여러 대화 걸친 관계 누적 변화
// ===========================================================================

#[test]
fn 여러_대화에_걸쳐_관계_누적_변화() {
    let li = make_무백();
    let mut rel = RelationshipBuilder::new("ally")
        .closeness(s(0.0))
        .trust(s(0.0))
        .build();

    // 대화 1: 긍정 (도움)
    let good_situation = Situation {
        description: "동료가 도움을 줌".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: 0.6,
            outcome_for_self: Some(0.5),
        },
    };
    let state1 = AppraisalEngine::appraise(li.personality(), &good_situation, &rel);
    rel.update_after_dialogue(&state1, &good_situation);

    let after_good = (rel.closeness().value(), rel.trust().value());

    // 대화 2: 또 긍정
    let state2 = AppraisalEngine::appraise(li.personality(), &good_situation, &rel);
    rel.update_after_dialogue(&state2, &good_situation);

    let after_good2 = (rel.closeness().value(), rel.trust().value());

    // 대화 3: 부정 (배신)
    let bad_situation = Situation {
        description: "배신".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.7,
            outcome_for_self: Some(-0.5),
        },
    };
    let state3 = AppraisalEngine::appraise(li.personality(), &bad_situation, &rel);
    rel.update_after_dialogue(&state3, &bad_situation);

    let after_bad = (rel.closeness().value(), rel.trust().value());

    // 검증: 긍정 대화로 관계 개선
    assert!(after_good.0 > 0.0, "대화1 후 closeness 상승: {}", after_good.0);
    assert!(after_good.1 > 0.0, "대화1 후 trust 상승: {}", after_good.1);

    // 검증: 연속 긍정으로 더 개선
    assert!(after_good2.1 > after_good.1, "대화2 후 trust 더 상승");

    // 검증: 배신으로 trust 하락 (but closeness는 누적이라 아직 양수일 수 있음)
    assert!(after_bad.1 < after_good2.1, "배신 후 trust 하락");

    println!("=== 관계 변화 추적 ===");
    println!("초기: closeness=0.0, trust=0.0");
    println!("대화1(긍정): c={:.3}, t={:.3}", after_good.0, after_good.1);
    println!("대화2(긍정): c={:.3}, t={:.3}", after_good2.0, after_good2.1);
    println!("대화3(배신): c={:.3}, t={:.3}", after_bad.0, after_bad.1);
}
