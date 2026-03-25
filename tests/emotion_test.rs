//! OCC 감정 모델 + HEXACO 연결 테스트
//!
//! 무협 4인 캐릭터가 같은 상황에서 다른 감정을 보이는지 검증

mod common;

use npc_mind::domain::emotion::*;
use npc_mind::domain::relationship::{Relationship, RelationshipBuilder};
use common::{make_무백, make_교룡, make_수련, make_소호, score as s, neutral_rel};

// ---------------------------------------------------------------------------
// 헬퍼: 감정 상태에서 특정 감정 찾기
// ---------------------------------------------------------------------------

fn find_emotion(state: &EmotionState, etype: EmotionType) -> Option<f32> {
    state.emotions().iter()
        .find(|e| e.emotion_type() == etype)
        .map(|e| e.intensity())
}

fn has_emotion(state: &EmotionState, etype: EmotionType) -> bool {
    find_emotion(state, etype).is_some()
}

// ===========================================================================
// 시나리오 1: "동료에게 배신당함"
// ===========================================================================

#[test]
fn 배신_무백은_절제된_분노() {
    let li = make_무백();
    let situation = Situation {
        description: "동료 무사가 적에게 아군의 위치를 밀고했다".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.7,
            outcome_for_self: Some(-0.6),
        },
    };

    let state = AppraisalEngine::appraise(li.personality(), &situation, &neutral_rel());

    let anger = find_emotion(&state, EmotionType::Anger).unwrap();
    assert!(anger > 0.0, "무백도 분노는 느낌: {}", anger);
    assert!(anger < 0.7, "하지만 patience↑로 억제됨: {}", anger);
    assert!(has_emotion(&state, EmotionType::Reproach));
}

#[test]
fn 배신_교룡은_폭발적_분노() {
    let yu = make_교룡();
    let situation = Situation {
        description: "동료 무사가 적에게 아군의 위치를 밀고했다".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.7,
            outcome_for_self: Some(-0.6),
        },
    };

    let state = AppraisalEngine::appraise(yu.personality(), &situation, &neutral_rel());

    let anger = find_emotion(&state, EmotionType::Anger).unwrap();
    assert!(anger > 0.5, "교룡의 분노는 폭발적: {}", anger);

    let li = make_무백();
    let li_state = AppraisalEngine::appraise(li.personality(), &situation, &neutral_rel());
    let li_anger = find_emotion(&li_state, EmotionType::Anger).unwrap();
    assert!(anger > li_anger,
        "교룡({}) > 무백({}) 분노", anger, li_anger);
}

#[test]
fn 배신_수련은_억눌린_고통() {
    let shu = make_수련();
    let situation = Situation {
        description: "동료 무사가 적에게 아군의 위치를 밀고했다".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.7,
            outcome_for_self: Some(-0.6),
        },
    };

    let state = AppraisalEngine::appraise(shu.personality(), &situation, &neutral_rel());

    let anger = find_emotion(&state, EmotionType::Anger).unwrap();
    let yu = make_교룡();
    let yu_anger = find_emotion(
        &AppraisalEngine::appraise(yu.personality(), &situation, &neutral_rel()),
        EmotionType::Anger,
    ).unwrap();
    assert!(anger < yu_anger,
        "수련({}) < 교룡({}) 분노 — 극도의 인내", anger, yu_anger);
}

// ===========================================================================
// 시나리오 2: "적의 대군이 다가오고 있다" (미래 전망)
// ===========================================================================

#[test]
fn 적_대군_무백은_담담한_두려움() {
    let li = make_무백();
    let situation = Situation {
        description: "적의 대군이 산 너머에서 다가오고 있다".into(),
        focus: SituationFocus::Event {
            desirability_for_self: -0.7,
            desirability_for_other: None,
            is_prospective: true,
            prior_expectation: None,
        },
    };

    let state = AppraisalEngine::appraise(li.personality(), &situation, &neutral_rel());
    let fear = find_emotion(&state, EmotionType::Fear).unwrap();
    assert!(fear > 0.0, "두려움은 있음: {}", fear);
}

#[test]
fn 적_대군_소호는_두려움_없이_행동() {
    let na = make_소호();
    let situation = Situation {
        description: "적의 대군이 산 너머에서 다가오고 있다".into(),
        focus: SituationFocus::Event {
            desirability_for_self: -0.7,
            desirability_for_other: None,
            is_prospective: true,
            prior_expectation: None,
        },
    };

    let na_state = AppraisalEngine::appraise(na.personality(), &situation, &neutral_rel());
    let na_fear = find_emotion(&na_state, EmotionType::Fear).unwrap();
    assert!(na_fear > 0.0, "소호도 기본 두려움은 있음: {}", na_fear);
}

// ===========================================================================
// 시나리오 3: "라이벌이 무림맹주에 추대됨" (타인의 운)
// ===========================================================================

#[test]
fn 라이벌_승진_무백은_대리기쁨() {
    let li = make_무백();
    let situation = Situation {
        description: "오랜 라이벌이 무림맹주에 추대되었다".into(),
        focus: SituationFocus::Event {
            desirability_for_self: 0.0,
            desirability_for_other: Some(0.8),
            is_prospective: false,
            prior_expectation: None,
        },
    };

    let state = AppraisalEngine::appraise(li.personality(), &situation, &neutral_rel());
    assert!(has_emotion(&state, EmotionType::HappyFor),
        "무백은 대리 기쁨을 느낌");
    assert!(!has_emotion(&state, EmotionType::Resentment),
        "무백은 시기하지 않음");
}

#[test]
fn 라이벌_승진_교룡은_시기() {
    let yu = make_교룡();
    let situation = Situation {
        description: "오랜 라이벌이 무림맹주에 추대되었다".into(),
        focus: SituationFocus::Event {
            desirability_for_self: 0.0,
            desirability_for_other: Some(0.8),
            is_prospective: false,
            prior_expectation: None,
        },
    };

    let state = AppraisalEngine::appraise(yu.personality(), &situation, &neutral_rel());
    assert!(has_emotion(&state, EmotionType::Resentment),
        "교룡은 시기를 느낌");
}

// ===========================================================================
// 시나리오 4: "해독약 구하기 실패"
// ===========================================================================

#[test]
fn 해독약_실패_실망_강도_비교() {
    let li = make_무백();
    let shu = make_수련();
    let situation = Situation {
        description: "사부의 독을 치료할 해독약을 끝내 구하지 못했다".into(),
        focus: SituationFocus::Event {
            desirability_for_self: -0.8,
            desirability_for_other: None,
            is_prospective: false,
            prior_expectation: Some(PriorExpectation::HopeUnfulfilled),
        },
    };

    let li_state = AppraisalEngine::appraise(li.personality(), &situation, &neutral_rel());
    let shu_state = AppraisalEngine::appraise(shu.personality(), &situation, &neutral_rel());

    let li_disap = find_emotion(&li_state, EmotionType::Disappointment).unwrap();
    let shu_disap = find_emotion(&shu_state, EmotionType::Disappointment).unwrap();

    assert!(li_disap > 0.5, "무백도 깊은 실망: {}", li_disap);
    assert!(shu_disap > 0.5, "수련도 깊은 실망: {}", shu_disap);
}

// ===========================================================================
// EmotionState 기능 테스트
// ===========================================================================

#[test]
fn 감정_상태_전체_valence() {
    let yu = make_교룡();
    let situation = Situation {
        description: "배신".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.7,
            outcome_for_self: Some(-0.6),
        },
    };

    let state = AppraisalEngine::appraise(yu.personality(), &situation, &neutral_rel());
    let valence = state.overall_valence();
    assert!(valence < 0.0,
        "배신당한 교룡의 전체 감정은 부정적: {}", valence);
}

#[test]
fn 감정_상태_dominant_감정() {
    let yu = make_교룡();
    let situation = Situation {
        description: "배신".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.7,
            outcome_for_self: Some(-0.6),
        },
    };

    let state = AppraisalEngine::appraise(yu.personality(), &situation, &neutral_rel());
    let dom = state.dominant().unwrap();
    assert!(
        dom.emotion_type() == EmotionType::Anger
        || dom.emotion_type() == EmotionType::Reproach,
        "교룡의 지배 감정: {:?} (강도 {})", dom.emotion_type(), dom.intensity()
    );
}

#[test]
fn 감정_significant_필터링() {
    let li = make_무백();
    let situation = Situation {
        description: "배신".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.7,
            outcome_for_self: Some(-0.6),
        },
    };

    let state = AppraisalEngine::appraise(li.personality(), &situation, &neutral_rel());
    let significant = state.significant(0.2);
    assert!(!significant.is_empty(), "유의미한 감정이 있어야 함");
    for w in significant.windows(2) {
        assert!(w[0].intensity() >= w[1].intensity(), "강도 내림차순");
    }
}

// ===========================================================================
// 시나리오 6: Relationship이 감정 강도에 미치는 영향
// ===========================================================================

#[test]
fn 의형제의_배신이_남의_배신보다_분노가_큼() {
    let yu = make_교룡();
    let situation = Situation {
        description: "배신".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.7,
            outcome_for_self: Some(-0.6),
        },
    };

    let brother = RelationshipBuilder::new("gyo_ryong", "brother")
        .closeness(s(0.9))
        .trust(s(0.8))
        .build();
    let stranger = Relationship::neutral("gyo_ryong", "stranger");

    let state_brother = AppraisalEngine::appraise(yu.personality(), &situation, &brother);
    let state_stranger = AppraisalEngine::appraise(yu.personality(), &situation, &stranger);

    let anger_brother = find_emotion(&state_brother, EmotionType::Anger).unwrap();
    let anger_stranger = find_emotion(&state_stranger, EmotionType::Anger).unwrap();

    assert!(anger_brother > anger_stranger,
        "의형제 배신({}) > 남 배신({}) 분노", anger_brother, anger_stranger);
}

#[test]
fn 신뢰하던_상대의_배신이_기대위반으로_더_강함() {
    let li = make_무백();
    let situation = Situation {
        description: "배신".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.7,
            outcome_for_self: Some(-0.6),
        },
    };

    let trusted = RelationshipBuilder::new("mu_baek", "trusted")
        .trust(s(0.8))
        .build();
    let distrusted = RelationshipBuilder::new("mu_baek", "distrusted")
        .trust(s(-0.5))
        .build();

    let state_trusted = AppraisalEngine::appraise(li.personality(), &situation, &trusted);
    let state_distrusted = AppraisalEngine::appraise(li.personality(), &situation, &distrusted);

    let anger_trusted = find_emotion(&state_trusted, EmotionType::Anger).unwrap();
    let anger_distrusted = find_emotion(&state_distrusted, EmotionType::Anger).unwrap();

    assert!(anger_trusted > anger_distrusted,
        "신뢰 배신({}) > 불신 배신({}) — 기대 위반 효과",
        anger_trusted, anger_distrusted);
}

#[test]
fn 가까운_사이의_좋은_일에_더_기뻐함() {
    let li = make_무백();
    let situation = Situation {
        description: "동료 승진".into(),
        focus: SituationFocus::Event {
            desirability_for_self: 0.0,
            desirability_for_other: Some(0.8),
            is_prospective: false,
            prior_expectation: None,
        },
    };

    let close = RelationshipBuilder::new("mu_baek", "close")
        .closeness(s(0.9))
        .build();
    let distant = Relationship::neutral("mu_baek", "distant");

    let state_close = AppraisalEngine::appraise(li.personality(), &situation, &close);
    let state_distant = AppraisalEngine::appraise(li.personality(), &situation, &distant);

    let happy_close = find_emotion(&state_close, EmotionType::HappyFor).unwrap();
    let happy_distant = find_emotion(&state_distant, EmotionType::HappyFor).unwrap();

    assert!(happy_close > happy_distant,
        "가까운 사이({}) > 먼 사이({}) 대리기쁨", happy_close, happy_distant);
}

#[test]
fn 적대관계의_좋은일에_교룡은_더_강한_시기() {
    let yu = make_교룡();
    let situation = Situation {
        description: "라이벌 승진".into(),
        focus: SituationFocus::Event {
            desirability_for_self: 0.0,
            desirability_for_other: Some(0.8),
            is_prospective: false,
            prior_expectation: None,
        },
    };

    let rival = RelationshipBuilder::new("gyo_ryong", "rival")
        .closeness(s(-0.7))
        .build();
    let nobody = Relationship::neutral("gyo_ryong", "nobody");

    let state_rival = AppraisalEngine::appraise(yu.personality(), &situation, &rival);
    let state_nobody = AppraisalEngine::appraise(yu.personality(), &situation, &nobody);

    let resent_rival = find_emotion(&state_rival, EmotionType::Resentment).unwrap();
    let resent_nobody = find_emotion(&state_nobody, EmotionType::Resentment).unwrap();

    assert!(resent_rival > resent_nobody,
        "라이벌({}) > 남({}) 시기 — closeness 절대값 효과",
        resent_rival, resent_nobody);
}
