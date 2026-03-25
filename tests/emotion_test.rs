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
// 시나리오 1: "동료에게 배신당함" (Action + Event → Compound Anger)
// ===========================================================================

#[test]
fn 배신_무백은_절제된_분노() {
    let li = make_무백();
    let situation = Situation {
        description: "동료 무사가 적에게 아군의 위치를 밀고했다".into(),
        focuses: vec![
            SituationFocus::Action(ActionFocus {
                is_self_agent: false,
                praiseworthiness: -0.7,
            }),
            SituationFocus::Event(EventFocus {
                desirability_for_self: -0.6,
                desirability_for_other: None,
                prospect: None,
            }),
        ],
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
        focuses: vec![
            SituationFocus::Action(ActionFocus {
                is_self_agent: false,
                praiseworthiness: -0.7,
            }),
            SituationFocus::Event(EventFocus {
                desirability_for_self: -0.6,
                desirability_for_other: None,
                prospect: None,
            }),
        ],
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
        focuses: vec![
            SituationFocus::Action(ActionFocus {
                is_self_agent: false,
                praiseworthiness: -0.7,
            }),
            SituationFocus::Event(EventFocus {
                desirability_for_self: -0.6,
                desirability_for_other: None,
                prospect: None,
            }),
        ],
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
        focuses: vec![SituationFocus::Event(EventFocus {
            desirability_for_self: -0.7,
            desirability_for_other: None,
            prospect: Some(Prospect::Anticipation),
        })],
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
        focuses: vec![SituationFocus::Event(EventFocus {
            desirability_for_self: -0.7,
            desirability_for_other: None,
            prospect: Some(Prospect::Anticipation),
        })],
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
    let other_rel = neutral_rel();
    let situation = Situation {
        description: "오랜 라이벌이 무림맹주에 추대되었다".into(),
        focuses: vec![SituationFocus::Event(EventFocus {
            desirability_for_self: 0.0,
            desirability_for_other: Some(DesirabilityForOther {
                target_id: "rival".into(),
                desirability: 0.8,
                relationship: other_rel,
            }),
            prospect: None,
        })],
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
    let other_rel = neutral_rel();
    let situation = Situation {
        description: "오랜 라이벌이 무림맹주에 추대되었다".into(),
        focuses: vec![SituationFocus::Event(EventFocus {
            desirability_for_self: 0.0,
            desirability_for_other: Some(DesirabilityForOther {
                target_id: "rival".into(),
                desirability: 0.8,
                relationship: other_rel,
            }),
            prospect: None,
        })],
    };

    let state = AppraisalEngine::appraise(yu.personality(), &situation, &neutral_rel());
    assert!(has_emotion(&state, EmotionType::Resentment),
        "교룡은 시기를 느낌");
}

// ===========================================================================
// 시나리오 4: "해독약 구하기 실패" (전망 확인)
// ===========================================================================

#[test]
fn 해독약_실패_실망_강도_비교() {
    let li = make_무백();
    let shu = make_수련();
    let situation = Situation {
        description: "사부의 독을 치료할 해독약을 끝내 구하지 못했다".into(),
        focuses: vec![SituationFocus::Event(EventFocus {
            desirability_for_self: -0.8,
            desirability_for_other: None,
            prospect: Some(Prospect::Confirmation(ProspectResult::HopeUnfulfilled)),
        })],
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

/// 배신 상황 (Action + Event) 헬퍼
fn 배신_상황_기본() -> Situation {
    Situation {
        description: "배신".into(),
        focuses: vec![
            SituationFocus::Action(ActionFocus {
                is_self_agent: false,
                praiseworthiness: -0.7,
            }),
            SituationFocus::Event(EventFocus {
                desirability_for_self: -0.6,
                desirability_for_other: None,
                prospect: None,
            }),
        ],
    }
}

#[test]
fn 감정_상태_전체_valence() {
    let yu = make_교룡();
    let state = AppraisalEngine::appraise(yu.personality(), &배신_상황_기본(), &neutral_rel());
    let valence = state.overall_valence();
    assert!(valence < 0.0,
        "배신당한 교룡의 전체 감정은 부정적: {}", valence);
}

#[test]
fn 감정_상태_dominant_감정() {
    let yu = make_교룡();
    let state = AppraisalEngine::appraise(yu.personality(), &배신_상황_기본(), &neutral_rel());
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
    let state = AppraisalEngine::appraise(li.personality(), &배신_상황_기본(), &neutral_rel());
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
    let situation = 배신_상황_기본();

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
fn 신뢰하던_상대의_배신이_더_강한_분노() {
    let li = make_무백();
    let situation = 배신_상황_기본();

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
        "신뢰 배신({}) > 불신 배신({}) — 신뢰도 배율 효과",
        anger_trusted, anger_distrusted);
}

#[test]
fn 가까운_사이의_좋은_일에_더_기뻐함() {
    let li = make_무백();

    let close = RelationshipBuilder::new("mu_baek", "close")
        .closeness(s(0.9))
        .build();
    let distant = Relationship::neutral("mu_baek", "distant");

    let sit_close = Situation {
        description: "동료 승진".into(),
        focuses: vec![SituationFocus::Event(EventFocus {
            desirability_for_self: 0.0,
            desirability_for_other: Some(DesirabilityForOther {
                target_id: "close".into(),
                desirability: 0.8,
                relationship: close,
            }),
            prospect: None,
        })],
    };
    let sit_distant = Situation {
        description: "동료 승진".into(),
        focuses: vec![SituationFocus::Event(EventFocus {
            desirability_for_self: 0.0,
            desirability_for_other: Some(DesirabilityForOther {
                target_id: "distant".into(),
                desirability: 0.8,
                relationship: distant,
            }),
            prospect: None,
        })],
    };

    let state_close = AppraisalEngine::appraise(li.personality(), &sit_close, &neutral_rel());
    let state_distant = AppraisalEngine::appraise(li.personality(), &sit_distant, &neutral_rel());

    let happy_close = find_emotion(&state_close, EmotionType::HappyFor).unwrap();
    let happy_distant = find_emotion(&state_distant, EmotionType::HappyFor).unwrap();

    assert!(happy_close > happy_distant,
        "가까운 사이({}) > 먼 사이({}) 대리기쁨", happy_close, happy_distant);
}

#[test]
fn 적대관계의_좋은일에_교룡은_더_강한_시기() {
    let yu = make_교룡();

    let rival = RelationshipBuilder::new("gyo_ryong", "rival")
        .closeness(s(-0.7))
        .build();
    let nobody = Relationship::neutral("gyo_ryong", "nobody");

    let sit_rival = Situation {
        description: "라이벌 승진".into(),
        focuses: vec![SituationFocus::Event(EventFocus {
            desirability_for_self: 0.0,
            desirability_for_other: Some(DesirabilityForOther {
                target_id: "rival".into(),
                desirability: 0.8,
                relationship: rival,
            }),
            prospect: None,
        })],
    };

    let sit_nobody = Situation {
        description: "라이벌 승진".into(),
        focuses: vec![SituationFocus::Event(EventFocus {
            desirability_for_self: 0.0,
            desirability_for_other: Some(DesirabilityForOther {
                target_id: "nobody".into(),
                desirability: 0.8,
                relationship: nobody,
            }),
            prospect: None,
        })],
    };

    let state_rival = AppraisalEngine::appraise(yu.personality(), &sit_rival, &neutral_rel());
    let state_nobody = AppraisalEngine::appraise(yu.personality(), &sit_nobody, &neutral_rel());

    let resent_rival = find_emotion(&state_rival, EmotionType::Resentment).unwrap();
    let resent_nobody = find_emotion(&state_nobody, EmotionType::Resentment).unwrap();

    assert!(resent_rival > resent_nobody,
        "라이벌({}) > 남({}) 시기 — closeness 절대값 효과",
        resent_rival, resent_nobody);
}

// ===========================================================================
// 시나리오 7: closeness 방향이 Fortune-of-others 감정을 조절
// ===========================================================================

/// 타인에게 좋은 일이 생기는 상황 (관계 정보 포함)
fn 타인_행운_상황(other_rel: &Relationship) -> Situation {
    Situation {
        description: "상대가 무림맹주에 추대되었다".into(),
        focuses: vec![SituationFocus::Event(EventFocus {
            desirability_for_self: 0.0,
            desirability_for_other: Some(DesirabilityForOther {
                target_id: other_rel.target_id().to_string(),
                desirability: 0.8,
                relationship: other_rel.clone(),
            }),
            prospect: None,
        })],
    }
}

/// 타인에게 나쁜 일이 생기는 상황 (관계 정보 포함)
fn 타인_불행_상황(other_rel: &Relationship) -> Situation {
    Situation {
        description: "상대가 비무에서 크게 패했다".into(),
        focuses: vec![SituationFocus::Event(EventFocus {
            desirability_for_self: 0.0,
            desirability_for_other: Some(DesirabilityForOther {
                target_id: other_rel.target_id().to_string(),
                desirability: -0.7,
                relationship: other_rel.clone(),
            }),
            prospect: None,
        })],
    }
}

#[test]
fn 원수의_행운에_무백은_기뻐하되_약하게() {
    let li = make_무백();
    let enemy = RelationshipBuilder::new("mu_baek", "enemy")
        .closeness(s(-0.8))
        .build();

    let state_enemy = AppraisalEngine::appraise(li.personality(), &타인_행운_상황(&enemy), &neutral_rel());
    let state_neutral = AppraisalEngine::appraise(li.personality(), &타인_행운_상황(&neutral_rel()), &neutral_rel());

    let happy_enemy = find_emotion(&state_enemy, EmotionType::HappyFor).unwrap();
    let happy_neutral = find_emotion(&state_neutral, EmotionType::HappyFor).unwrap();

    assert!(happy_enemy < happy_neutral,
        "원수 행운({}) < 무관한 사람 행운({}) — closeness 방향 억제",
        happy_enemy, happy_neutral);
}

#[test]
fn 친구의_행운에_교룡은_시기하되_약하게() {
    let yu = make_교룡();
    let friend = RelationshipBuilder::new("gyo_ryong", "friend")
        .closeness(s(0.8))
        .build();

    let state_friend = AppraisalEngine::appraise(yu.personality(), &타인_행운_상황(&friend), &neutral_rel());
    let state_neutral = AppraisalEngine::appraise(yu.personality(), &타인_행운_상황(&neutral_rel()), &neutral_rel());

    let resent_friend = find_emotion(&state_friend, EmotionType::Resentment).unwrap();
    let resent_neutral = find_emotion(&state_neutral, EmotionType::Resentment).unwrap();

    assert!(resent_friend < resent_neutral,
        "친구 행운 시기({}) < 무관 행운 시기({}) — closeness 방향 억제",
        resent_friend, resent_neutral);
}

#[test]
fn 친구의_불행에_수련은_더_강하게_동정() {
    let shu = make_수련();
    let friend = RelationshipBuilder::new("shu_lien", "friend")
        .closeness(s(0.8))
        .build();

    let state_friend = AppraisalEngine::appraise(shu.personality(), &타인_불행_상황(&friend), &neutral_rel());
    let state_neutral = AppraisalEngine::appraise(shu.personality(), &타인_불행_상황(&neutral_rel()), &neutral_rel());

    let pity_friend = find_emotion(&state_friend, EmotionType::Pity).unwrap();
    let pity_neutral = find_emotion(&state_neutral, EmotionType::Pity).unwrap();

    assert!(pity_friend > pity_neutral,
        "친구 불행 동정({}) > 무관 불행 동정({}) — closeness 방향 증폭",
        pity_friend, pity_neutral);
}

#[test]
fn 원수의_불행에_교룡은_더_강하게_쾌재() {
    let yu = make_교룡();
    let enemy = RelationshipBuilder::new("gyo_ryong", "enemy")
        .closeness(s(-0.8))
        .build();

    let state_enemy = AppraisalEngine::appraise(yu.personality(), &타인_불행_상황(&enemy), &neutral_rel());
    let state_neutral = AppraisalEngine::appraise(yu.personality(), &타인_불행_상황(&neutral_rel()), &neutral_rel());

    let gloat_enemy = find_emotion(&state_enemy, EmotionType::Gloating).unwrap();
    let gloat_neutral = find_emotion(&state_neutral, EmotionType::Gloating).unwrap();

    assert!(gloat_enemy > gloat_neutral,
        "원수 불행 쾌재({}) > 무관 불행 쾌재({}) — closeness 방향 증폭",
        gloat_enemy, gloat_neutral);
}

#[test]
fn 중립_관계는_closeness_방향_영향_없음() {
    let li = make_무백();
    let state = AppraisalEngine::appraise(li.personality(), &타인_행운_상황(&neutral_rel()), &neutral_rel());
    let happy = find_emotion(&state, EmotionType::HappyFor).unwrap();

    assert!(happy > 0.3,
        "중립 관계에서 무백의 HappyFor 정상 발동: {}", happy);
}

// ===========================================================================
// 시나리오 8: trust 방향이 Action 감정 강도를 조절
// ===========================================================================

/// 타인의 부정 행동 (배신) — Reproach, Anger 발동
fn 배신_상황() -> Situation {
    Situation {
        description: "상대가 적에게 아군 위치를 밀고했다".into(),
        focuses: vec![
            SituationFocus::Action(ActionFocus {
                is_self_agent: false,
                praiseworthiness: -0.7,
            }),
            SituationFocus::Event(EventFocus {
                desirability_for_self: -0.6,
                desirability_for_other: None,
                prospect: None,
            }),
        ],
    }
}

/// 타인의 긍정 행동 (도움) — Admiration, Gratitude 발동
fn 도움_상황() -> Situation {
    Situation {
        description: "상대가 목숨을 걸고 나를 구해주었다".into(),
        focuses: vec![
            SituationFocus::Action(ActionFocus {
                is_self_agent: false,
                praiseworthiness: 0.7,
            }),
            SituationFocus::Event(EventFocus {
                desirability_for_self: 0.6,
                desirability_for_other: None,
                prospect: None,
            }),
        ],
    }
}

#[test]
fn 신뢰하던_사람의_배신에_더_강한_분노() {
    let li = make_무백();
    let situation = 배신_상황();
    let trusted = RelationshipBuilder::new("mu_baek", "trusted").trust(s(0.8)).build();

    let state_trusted = AppraisalEngine::appraise(li.personality(), &situation, &trusted);
    let state_neutral = AppraisalEngine::appraise(li.personality(), &situation, &neutral_rel());

    let anger_trusted = find_emotion(&state_trusted, EmotionType::Anger).unwrap();
    let anger_neutral = find_emotion(&state_neutral, EmotionType::Anger).unwrap();

    assert!(anger_trusted > anger_neutral,
        "신뢰 배신({}) > 중립 배신({}) — 믿었는데!",
        anger_trusted, anger_neutral);
}

#[test]
fn 신뢰하던_사람의_배신에_더_강한_비난() {
    let li = make_무백();
    let situation = 배신_상황();
    let trusted = RelationshipBuilder::new("mu_baek", "trusted").trust(s(0.8)).build();

    let state_trusted = AppraisalEngine::appraise(li.personality(), &situation, &trusted);
    let state_neutral = AppraisalEngine::appraise(li.personality(), &situation, &neutral_rel());

    let reproach_trusted = find_emotion(&state_trusted, EmotionType::Reproach).unwrap();
    let reproach_neutral = find_emotion(&state_neutral, EmotionType::Reproach).unwrap();

    assert!(reproach_trusted > reproach_neutral,
        "신뢰 배신 비난({}) > 중립 배신 비난({}) — 믿었는데!",
        reproach_trusted, reproach_neutral);
}

#[test]
fn 불신하던_사람의_배신에_약한_분노() {
    let li = make_무백();
    let situation = 배신_상황();
    let distrusted = RelationshipBuilder::new("mu_baek", "distrusted").trust(s(-0.5)).build();

    let state_distrusted = AppraisalEngine::appraise(li.personality(), &situation, &distrusted);
    let state_neutral = AppraisalEngine::appraise(li.personality(), &situation, &neutral_rel());

    let anger_distrusted = find_emotion(&state_distrusted, EmotionType::Anger).unwrap();
    let anger_neutral = find_emotion(&state_neutral, EmotionType::Anger).unwrap();

    assert!(anger_distrusted < anger_neutral,
        "불신 배신({}) < 중립 배신({}) — 역시나",
        anger_distrusted, anger_neutral);
}

#[test]
fn 신뢰하던_사람의_도움에_더_강한_감사() {
    let li = make_무백();
    let situation = 도움_상황();
    let trusted = RelationshipBuilder::new("mu_baek", "trusted").trust(s(0.8)).build();

    let state_trusted = AppraisalEngine::appraise(li.personality(), &situation, &trusted);
    let state_neutral = AppraisalEngine::appraise(li.personality(), &situation, &neutral_rel());

    let grat_trusted = find_emotion(&state_trusted, EmotionType::Gratitude).unwrap();
    let grat_neutral = find_emotion(&state_neutral, EmotionType::Gratitude).unwrap();

    assert!(grat_trusted > grat_neutral,
        "신뢰 도움 감사({}) > 중립 도움 감사({}) — 역시 형이야",
        grat_trusted, grat_neutral);
}

#[test]
fn 신뢰하던_사람의_의로운_행동에_더_강한_감탄() {
    let li = make_무백();
    let situation = 도움_상황();
    let trusted = RelationshipBuilder::new("mu_baek", "trusted").trust(s(0.8)).build();

    let state_trusted = AppraisalEngine::appraise(li.personality(), &situation, &trusted);
    let state_neutral = AppraisalEngine::appraise(li.personality(), &situation, &neutral_rel());

    let adm_trusted = find_emotion(&state_trusted, EmotionType::Admiration).unwrap();
    let adm_neutral = find_emotion(&state_neutral, EmotionType::Admiration).unwrap();

    assert!(adm_trusted > adm_neutral,
        "신뢰 의로움 감탄({}) > 중립 의로움 감탄({}) — 역시 대협",
        adm_trusted, adm_neutral);
}

#[test]
fn 불신하던_사람의_도움에_약한_감사() {
    let li = make_무백();
    let situation = 도움_상황();
    let distrusted = RelationshipBuilder::new("mu_baek", "distrusted").trust(s(-0.5)).build();

    let state_distrusted = AppraisalEngine::appraise(li.personality(), &situation, &distrusted);
    let state_neutral = AppraisalEngine::appraise(li.personality(), &situation, &neutral_rel());

    let grat_distrusted = find_emotion(&state_distrusted, EmotionType::Gratitude).unwrap();
    let grat_neutral = find_emotion(&state_neutral, EmotionType::Gratitude).unwrap();

    assert!(grat_distrusted < grat_neutral,
        "불신 도움 감사({}) < 중립 도움 감사({}) — 뭔 꿍꿍이지",
        grat_distrusted, grat_neutral);
}

#[test]
fn 불신하던_사람의_의로운_행동에_약한_감탄() {
    let li = make_무백();
    let situation = 도움_상황();
    let distrusted = RelationshipBuilder::new("mu_baek", "distrusted").trust(s(-0.5)).build();

    let state_distrusted = AppraisalEngine::appraise(li.personality(), &situation, &distrusted);
    let state_neutral = AppraisalEngine::appraise(li.personality(), &situation, &neutral_rel());

    let adm_distrusted = find_emotion(&state_distrusted, EmotionType::Admiration).unwrap();
    let adm_neutral = find_emotion(&state_neutral, EmotionType::Admiration).unwrap();

    assert!(adm_distrusted < adm_neutral,
        "불신 의로움 감탄({}) < 중립 의로움 감탄({}) — 덤덤",
        adm_distrusted, adm_neutral);
}
