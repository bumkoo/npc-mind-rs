//! OCC 감정 모델 + HEXACO 연결 테스트
//!
//! 무협 4인 캐릭터가 같은 상황에서 다른 감정을 보이는지 검증

use npc_mind::domain::personality::*;
use npc_mind::domain::emotion::*;

// ---------------------------------------------------------------------------
// 헬퍼: 4인 캐릭터 생성
// ---------------------------------------------------------------------------

fn score(v: f32) -> Score {
    Score::new(v, "").unwrap()
}

fn make_무백() -> Npc {
    let s = score;
    NpcBuilder::new("mu_baek", "무백")
        .honesty_humility(|h| {
            h.sincerity = s(0.8); h.fairness = s(0.7);
            h.greed_avoidance = s(0.6); h.modesty = s(0.5);
        })
        .emotionality(|e| {
            e.fearfulness = s(-0.6); e.anxiety = s(-0.4);
            e.dependence = s(-0.7); e.sentimentality = s(0.2);
        })
        .agreeableness(|a| {
            a.forgiveness = s(0.6); a.gentleness = s(0.7);
            a.flexibility = s(0.2); a.patience = s(0.8);
        })
        .conscientiousness(|c| {
            c.organization = s(0.4); c.diligence = s(0.8);
            c.perfectionism = s(0.6); c.prudence = s(0.7);
        })
        .build()
}

fn make_교룡() -> Npc {
    let s = score;
    NpcBuilder::new("gyo_ryong", "교룡")
        .honesty_humility(|h| {
            h.sincerity = s(-0.4); h.fairness = s(-0.5);
            h.greed_avoidance = s(-0.6); h.modesty = s(-0.7);
        })
        .extraversion(|x| {
            x.social_self_esteem = s(0.7); x.social_boldness = s(0.8);
            x.sociability = s(0.0); x.liveliness = s(0.6);
        })
        .agreeableness(|a| {
            a.forgiveness = s(-0.6); a.gentleness = s(-0.5);
            a.flexibility = s(-0.4); a.patience = s(-0.7);
        })
        .openness(|o| {
            o.aesthetic_appreciation = s(0.6); o.inquisitiveness = s(0.8);
            o.creativity = s(0.7); o.unconventionality = s(0.9);
        })
        .build()
}

fn make_수련() -> Npc {
    let s = score;
    NpcBuilder::new("shu_lien", "수련")
        .honesty_humility(|h| {
            h.sincerity = s(0.8); h.fairness = s(0.9);
            h.greed_avoidance = s(0.7); h.modesty = s(0.6);
        })
        .emotionality(|e| {
            e.fearfulness = s(-0.3); e.anxiety = s(0.2);
            e.dependence = s(-0.5); e.sentimentality = s(0.7);
        })
        .agreeableness(|a| {
            a.forgiveness = s(0.5); a.gentleness = s(0.6);
            a.flexibility = s(0.3); a.patience = s(0.9);
        })
        .conscientiousness(|c| {
            c.organization = s(0.6); c.diligence = s(0.8);
            c.perfectionism = s(0.5); c.prudence = s(0.9);
        })
        .build()
}

fn make_소호() -> Npc {
    let s = score;
    NpcBuilder::new("so_ho", "소호")
        .honesty_humility(|h| {
            h.sincerity = s(0.1); h.fairness = s(0.5);
            h.greed_avoidance = s(0.3); h.modesty = s(-0.3);
        })
        .emotionality(|e| {
            e.fearfulness = s(-0.7); e.anxiety = s(-0.5);
            e.dependence = s(-0.8); e.sentimentality = s(0.4);
        })
        .extraversion(|x| {
            x.social_self_esteem = s(0.6); x.social_boldness = s(0.7);
            x.sociability = s(0.5); x.liveliness = s(0.4);
        })
        .agreeableness(|a| {
            a.forgiveness = s(0.1); a.gentleness = s(-0.4);
            a.flexibility = s(0.3); a.patience = s(-0.3);
        })
        .conscientiousness(|c| {
            c.organization = s(-0.6); c.diligence = s(0.2);
            c.perfectionism = s(-0.4); c.prudence = s(-0.5);
        })
        .build()
}

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

    let state = AppraisalEngine::appraise(li.personality(), &situation);

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

    let state = AppraisalEngine::appraise(yu.personality(), &situation);

    let anger = find_emotion(&state, EmotionType::Anger).unwrap();
    assert!(anger > 0.5, "교룡의 분노는 폭발적: {}", anger);

    let li = make_무백();
    let li_state = AppraisalEngine::appraise(li.personality(), &situation);
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

    let state = AppraisalEngine::appraise(shu.personality(), &situation);

    let anger = find_emotion(&state, EmotionType::Anger).unwrap();
    let yu = make_교룡();
    let yu_anger = find_emotion(
        &AppraisalEngine::appraise(yu.personality(), &situation),
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

    let state = AppraisalEngine::appraise(li.personality(), &situation);
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

    let na_state = AppraisalEngine::appraise(na.personality(), &situation);
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

    let state = AppraisalEngine::appraise(li.personality(), &situation);
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

    let state = AppraisalEngine::appraise(yu.personality(), &situation);
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

    let li_state = AppraisalEngine::appraise(li.personality(), &situation);
    let shu_state = AppraisalEngine::appraise(shu.personality(), &situation);

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

    let state = AppraisalEngine::appraise(yu.personality(), &situation);
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

    let state = AppraisalEngine::appraise(yu.personality(), &situation);
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

    let state = AppraisalEngine::appraise(li.personality(), &situation);
    let significant = state.significant(0.2);
    assert!(!significant.is_empty(), "유의미한 감정이 있어야 함");
    for w in significant.windows(2) {
        assert!(w[0].intensity() >= w[1].intensity(), "강도 내림차순");
    }
}

// ===========================================================================
// 시나리오 5: 대화 중 감정 변화 (appraise_with_context)
// ===========================================================================

#[test]
fn 대화_교룡_3턴_감정_누적() {
    let yu = make_교룡();

    let turn1 = Situation {
        description: "그 검을 돌려주시오".into(),
        focus: SituationFocus::Event {
            desirability_for_self: -0.3,
            desirability_for_other: None,
            is_prospective: false,
            prior_expectation: None,
        },
    };
    let state1 = AppraisalEngine::appraise(yu.personality(), &turn1);
    let distress1 = find_emotion(&state1, EmotionType::Distress).unwrap_or(0.0);
    assert!(distress1 > 0.0, "턴1: 약간의 짜증: {}", distress1);

    let turn2 = Situation {
        description: "그건 내 사부의 유품이오".into(),
        focus: SituationFocus::Action {
            is_self_agent: true,
            praiseworthiness: -0.4,
            outcome_for_self: None,
        },
    };
    let state2 = AppraisalEngine::appraise_with_context(
        yu.personality(), &turn2, &state1
    );
    assert!(has_emotion(&state2, EmotionType::Shame),
        "턴2: 죄책감(Shame) 추가됨");
    assert!(has_emotion(&state2, EmotionType::Distress),
        "턴2: 턴1의 짜증(Distress)도 유지됨");

    let turn3 = Situation {
        description: "도둑질이라 부를 수밖에".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.6,
            outcome_for_self: Some(-0.5),
        },
    };
    let state3 = AppraisalEngine::appraise_with_context(
        yu.personality(), &turn3, &state2
    );

    let state3_no_context = AppraisalEngine::appraise(yu.personality(), &turn3);
    let anger_with_context = find_emotion(&state3, EmotionType::Anger).unwrap_or(0.0);
    let anger_no_context = find_emotion(&state3_no_context, EmotionType::Anger).unwrap_or(0.0);

    assert!(anger_with_context > anger_no_context,
        "대화 맥락이 있으면 분노가 더 강함: with={} > without={}",
        anger_with_context, anger_no_context);
}

#[test]
fn 대화_무백은_누적되어도_절제() {
    let li = make_무백();

    let turn1 = Situation {
        description: "교룡이 검을 돌려주지 않겠다고 함".into(),
        focus: SituationFocus::Event {
            desirability_for_self: -0.4,
            desirability_for_other: None,
            is_prospective: false,
            prior_expectation: None,
        },
    };
    let state1 = AppraisalEngine::appraise(li.personality(), &turn1);

    let turn2 = Situation {
        description: "교룡이 무례하게 거절함".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.5,
            outcome_for_self: Some(-0.4),
        },
    };
    let state2 = AppraisalEngine::appraise_with_context(
        li.personality(), &turn2, &state1
    );

    let li_anger = find_emotion(&state2, EmotionType::Anger).unwrap_or(0.0);

    let yu = make_교룡();
    let yu_state1 = AppraisalEngine::appraise(yu.personality(), &turn1);
    let yu_state2 = AppraisalEngine::appraise_with_context(
        yu.personality(), &turn2, &yu_state1
    );
    let yu_anger = find_emotion(&yu_state2, EmotionType::Anger).unwrap_or(0.0);

    assert!(li_anger < yu_anger,
        "2턴 누적 후에도 무백({}) < 교룡({}) 분노", li_anger, yu_anger);
}

#[test]
fn 대화_긍정_감정_누적() {
    let li = make_무백();

    let turn1 = Situation {
        description: "옛 동료가 살아 돌아왔다".into(),
        focus: SituationFocus::Event {
            desirability_for_self: 0.5,
            desirability_for_other: None,
            is_prospective: false,
            prior_expectation: None,
        },
    };
    let state1 = AppraisalEngine::appraise(li.personality(), &turn1);

    let turn2 = Situation {
        description: "그 동료가 해독약도 가져왔다".into(),
        focus: SituationFocus::Event {
            desirability_for_self: 0.5,
            desirability_for_other: None,
            is_prospective: false,
            prior_expectation: None,
        },
    };
    let state2_with = AppraisalEngine::appraise_with_context(
        li.personality(), &turn2, &state1
    );
    let state2_without = AppraisalEngine::appraise(li.personality(), &turn2);

    let joy_with = find_emotion(&state2_with, EmotionType::Joy).unwrap_or(0.0);
    let joy_without = find_emotion(&state2_without, EmotionType::Joy).unwrap_or(0.0);

    assert!(joy_with > joy_without,
        "연속 좋은 소식이면 기쁨이 더 강함: with={} > without={}",
        joy_with, joy_without);
}
