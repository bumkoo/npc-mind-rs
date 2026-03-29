//! 대화 후 Relationship 갱신 통합 테스트
//!
//! 전체 대화 흐름: appraise → apply_stimulus → after_dialogue
//! 대화 결과에 따라 관계가 변하는지 검증
//!
//! Relationship은 Value Object — 갱신 시 새 인스턴스로 교체

mod common;

use npc_mind::domain::emotion::*;
use npc_mind::domain::pad::Pad;
use npc_mind::domain::relationship::*;
use common::{make_무백, make_교룡, score as s};

fn find_emotion(state: &EmotionState, etype: EmotionType) -> Option<f32> {
    // state.emotions()는 이제 Vec<Emotion>을 반환하므로 직접 순회 가능
    state.emotions().into_iter()
        .find(|e| e.emotion_type() == etype)
        .map(|e| e.intensity())
}

/// 배신 상황 (Action + Event)
fn 배신_상황() -> Situation {
    Situation::new(
        "동료의 배신",
        Some(EventFocus {
            description: "".into(),
            desirability_for_self: -0.6,
            desirability_for_other: None,
            prospect: None,
        }),
        Some(ActionFocus {
            description: "".into(),
            agent_id: Some("partner".into()), relationship: None,
            praiseworthiness: -0.7,
        }),
        None,
    ).unwrap()
}

/// 갈등 상황 (Action + Event, 중간 강도)
fn 갈등_상황() -> Situation {
    Situation::new(
        "갈등",
        Some(EventFocus {
            description: "".into(),
            desirability_for_self: -0.4,
            desirability_for_other: None,
            prospect: None,
        }),
        Some(ActionFocus {
            description: "".into(),
            agent_id: Some("partner".into()), relationship: None,
            praiseworthiness: -0.5,
        }),
        None,
    ).unwrap()
}

// ===========================================================================
// 배신 대화 → trust 하락
// ===========================================================================

#[test]
fn 배신_대화_후_trust_하락() {
    let yu = make_교룡();
    let rel = RelationshipBuilder::new("gyo_ryong", "mu_baek")
        .closeness(s(0.8))
        .trust(s(0.5))
        .build();
    let situation = 배신_상황();

    let state = AppraisalEngine::appraise(yu.personality(), &situation, &rel);
    let provocation = Pad::new(-0.6, 0.7, 0.5);
    let state1 = StimulusEngine::apply_stimulus(yu.personality(), &state, &provocation);
    let state2 = StimulusEngine::apply_stimulus(yu.personality(), &state1, &provocation);
    let final_state = StimulusEngine::apply_stimulus(yu.personality(), &state2, &provocation);

    let updated = rel.after_dialogue(&final_state, situation.action.as_ref().map(|a| a.praiseworthiness), 0.0);

    assert!(updated.trust().value() < rel.trust().value(),
        "배신 대화 후 trust 하락: {} → {}", rel.trust().value(), updated.trust().value());
}

// ===========================================================================
// 부정 대화 → closeness 하락
// ===========================================================================

#[test]
fn 부정_대화_후_closeness_하락() {
    let yu = make_교룡();
    let rel = RelationshipBuilder::new("gyo_ryong", "mu_baek")
        .closeness(s(0.5))
        .build();
    let situation = 갈등_상황();

    let state = AppraisalEngine::appraise(yu.personality(), &situation, &rel);
    let updated = rel.after_dialogue(&state, situation.action.as_ref().map(|a| a.praiseworthiness), 0.0);

    assert!(updated.closeness().value() < rel.closeness().value(),
        "부정 대화 후 closeness 하락: {} → {}",
        rel.closeness().value(), updated.closeness().value());
}

// ===========================================================================
// 긍정 대화 → closeness 상승
// ===========================================================================

#[test]
fn 긍정_대화_후_closeness_상승() {
    let li = make_무백();
    let rel = RelationshipBuilder::new("mu_baek", "friend")
        .closeness(s(0.3))
        .build();

    let situation = Situation::new(
        "좋은 소식",
        Some(EventFocus {
            description: "".into(),
            desirability_for_self: 0.7,
            desirability_for_other: None,
            prospect: None,
        }),
        None,
        None,
    ).unwrap();

    let state = AppraisalEngine::appraise(li.personality(), &situation, &rel);
    let updated = rel.after_dialogue(&state, situation.action.as_ref().map(|a| a.praiseworthiness), 0.0);

    assert!(updated.closeness().value() > rel.closeness().value(),
        "긍정 대화 후 closeness 상승: {} → {}",
        rel.closeness().value(), updated.closeness().value());
}

// ===========================================================================
// Event 분기는 trust 미갱신
// ===========================================================================

#[test]
fn event_분기는_trust_변경_없음() {
    let li = make_무백();
    let rel = RelationshipBuilder::new("mu_baek", "target")
        .trust(s(0.5))
        .build();

    let situation = Situation::new(
        "적 대군 접근",
        Some(EventFocus {
            description: "".into(),
            desirability_for_self: -0.7,
            desirability_for_other: None,
            prospect: Some(Prospect::Anticipation),
        }),
        None,
        None,
    ).unwrap();

    let state = AppraisalEngine::appraise(li.personality(), &situation, &rel);
    let updated = rel.after_dialogue(&state, situation.action.as_ref().map(|a| a.praiseworthiness), 0.0);

    assert!((rel.trust().value() - updated.trust().value()).abs() < 0.001,
        "Event 분기 → trust 미변경: {} → {}",
        rel.trust().value(), updated.trust().value());
}

// ===========================================================================
// power는 대화로 변하지 않음
// ===========================================================================

#[test]
fn 대화_후_power_변경_없음() {
    let yu = make_교룡();
    let rel = RelationshipBuilder::new("gyo_ryong", "master")
        .power(s(-0.7))
        .build();
    let situation = 배신_상황();

    let state = AppraisalEngine::appraise(yu.personality(), &situation, &rel);
    let updated = rel.after_dialogue(&state, situation.action.as_ref().map(|a| a.praiseworthiness), 0.0);

    assert!((updated.power().value() - -0.7).abs() < 0.001,
        "power는 대화로 변하지 않음: {}", updated.power().value());
}

// ===========================================================================
// 전체 시나리오: 무백-교룡 의형제 배신 → 관계 악화
// ===========================================================================

#[test]
fn 시나리오_의형제_배신_후_관계_악화() {
    let yu = make_교룡();
    let rel = RelationshipBuilder::new("gyo_ryong", "mu_baek")
        .closeness(s(0.8))
        .trust(s(0.7))
        .power(s(0.0))
        .build();

    let situation = Situation::new(
        "무백이 교룡의 검을 빼앗아 관에 넘겼다",
        Some(EventFocus {
            description: "".into(),
            desirability_for_self: -0.7,
            desirability_for_other: None,
            prospect: None,
        }),
        Some(ActionFocus {
            description: "".into(),
            agent_id: Some("partner".into()), relationship: None,
            praiseworthiness: -0.8,
        }),
        None,
    ).unwrap();

    // 1. 상황 진입
    let initial_state = AppraisalEngine::appraise(yu.personality(), &situation, &rel);
    let anger = find_emotion(&initial_state, EmotionType::Anger).unwrap();
    assert!(anger > 0.5, "의형제 배신 → 강한 분노: {}", anger);

    // 2. 대화 — 도발 3턴
    let provocation = Pad::new(-0.7, 0.8, 0.6);
    let s1 = StimulusEngine::apply_stimulus(yu.personality(), &initial_state, &provocation);
    let s2 = StimulusEngine::apply_stimulus(yu.personality(), &s1, &provocation);
    let final_state = StimulusEngine::apply_stimulus(yu.personality(), &s2, &provocation);

    // 3. 대화 종료
    let updated = rel.after_dialogue(&final_state, situation.action.as_ref().map(|a| a.praiseworthiness), 0.0);

    assert!(updated.trust().value() < rel.trust().value(),
        "trust 급락: {} → {}", rel.trust().value(), updated.trust().value());
    assert!(rel.trust().value() - updated.trust().value() > 0.05,
        "trust 하락폭이 유의미: delta={}",
        rel.trust().value() - updated.trust().value());
    assert!(updated.closeness().value() < rel.closeness().value(),
        "closeness 하락: {} → {}",
        rel.closeness().value(), updated.closeness().value());
    assert!((updated.power().value() - 0.0).abs() < 0.001);
    assert!((rel.trust().value() - 0.7).abs() < 0.001, "원본 trust 불변");
    assert!((rel.closeness().value() - 0.8).abs() < 0.001, "원본 closeness 불변");

    println!("=== 의형제 배신 시나리오 ===");
    println!("trust: {} → {}", rel.trust().value(), updated.trust().value());
    println!("closeness: {} → {}", rel.closeness().value(), updated.closeness().value());
}

// ===========================================================================
// 여러 대화 걸친 관계 누적 변화
// ===========================================================================

#[test]
fn 여러_대화에_걸쳐_관계_누적_변화() {
    let li = make_무백();
    let rel0 = RelationshipBuilder::new("mu_baek", "ally")
        .closeness(s(0.0))
        .trust(s(0.0))
        .build();

    // 대화 1: 긍정 (도움)
    let good_situation = Situation::new(
        "동료가 도움을 줌",
        Some(EventFocus {
            description: "".into(),
            desirability_for_self: 0.5,
            desirability_for_other: None,
            prospect: None,
        }),
        Some(ActionFocus {
            description: "".into(),
            agent_id: Some("partner".into()), relationship: None,
            praiseworthiness: 0.6,
        }),
        None,
    ).unwrap();
    let state1 = AppraisalEngine::appraise(li.personality(), &good_situation, &rel0);
    let rel1 = rel0.after_dialogue(&state1, good_situation.action.as_ref().map(|a| a.praiseworthiness), 0.0);

    // 대화 2: 또 긍정
    let state2 = AppraisalEngine::appraise(li.personality(), &good_situation, &rel1);
    let rel2 = rel1.after_dialogue(&state2, good_situation.action.as_ref().map(|a| a.praiseworthiness), 0.0);

    // 대화 3: 부정 (배신)
    let bad_situation = Situation::new(
        "배신",
        Some(EventFocus {
            description: "".into(),
            desirability_for_self: -0.5,
            desirability_for_other: None,
            prospect: None,
        }),
        Some(ActionFocus {
            description: "".into(),
            agent_id: Some("partner".into()), relationship: None,
            praiseworthiness: -0.7,
        }),
        None,
    ).unwrap();
    let state3 = AppraisalEngine::appraise(li.personality(), &bad_situation, &rel2);
    let rel3 = rel2.after_dialogue(&state3, bad_situation.action.as_ref().map(|a| a.praiseworthiness), 0.0);

    // 검증: 긍정 대화로 관계 개선
    assert!(rel1.closeness().value() > 0.0,
        "대화1 후 closeness 상승: {}", rel1.closeness().value());
    assert!(rel1.trust().value() > 0.0,
        "대화1 후 trust 상승: {}", rel1.trust().value());

    // 검증: 연속 긍정으로 더 개선
    assert!(rel2.trust().value() > rel1.trust().value(),
        "대화2 후 trust 더 상승");

    // 검증: 배신으로 trust 하락
    assert!(rel3.trust().value() < rel2.trust().value(),
        "배신 후 trust 하락");

    // 검증: 원본들 전부 불변
    assert_eq!(rel0.trust().value(), 0.0, "rel0 불변");

    println!("=== 관계 변화 추적 ===");
    println!("초기:       c={:.3}, t={:.3}", rel0.closeness().value(), rel0.trust().value());
    println!("대화1(긍정): c={:.3}, t={:.3}", rel1.closeness().value(), rel1.trust().value());
    println!("대화2(긍정): c={:.3}, t={:.3}", rel2.closeness().value(), rel2.trust().value());
    println!("대화3(배신): c={:.3}, t={:.3}", rel3.closeness().value(), rel3.trust().value());
}
