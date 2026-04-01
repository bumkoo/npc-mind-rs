//! apply_stimulus 테스트
//!
//! 대화 중 대사 자극에 의한 감정 변동 검증.

mod common;

use common::{find_emotion, make_교룡, make_무백, neutral_rel, 배신_상황};
use npc_mind::domain::emotion::*;
use npc_mind::domain::pad::Pad;
use npc_mind::ports::{Appraiser, StimulusProcessor};

#[test]
fn 도발_자극이_anger를_증폭() {
    let yu = make_교룡();
    let initial =
        AppraisalEngine.appraise(yu.personality(), &배신_상황(), &neutral_rel().modifiers());
    let anger_before = find_emotion(&initial, EmotionType::Anger).unwrap();

    let provocation = Pad::new(-0.6, 0.7, 0.5);
    let after = StimulusEngine.apply_stimulus(yu.personality(), &initial, &provocation);
    let anger_after = find_emotion(&after, EmotionType::Anger).unwrap();

    assert!(
        anger_after > anger_before,
        "도발 후 Anger 증폭: {} → {}",
        anger_before,
        anger_after
    );
}

#[test]
fn 사과_자극이_anger를_감소() {
    let yu = make_교룡();
    let initial =
        AppraisalEngine.appraise(yu.personality(), &배신_상황(), &neutral_rel().modifiers());
    let anger_before = find_emotion(&initial, EmotionType::Anger).unwrap();

    let apology = Pad::new(0.5, -0.3, -0.4);
    let after = StimulusEngine.apply_stimulus(yu.personality(), &initial, &apology);
    let anger_after = find_emotion(&after, EmotionType::Anger).unwrap();

    assert!(
        anger_after < anger_before,
        "사과 후 Anger 감소: {} → {}",
        anger_before,
        anger_after
    );
}

#[test]
fn 교룡이_무백보다_부정_자극에_더_크게_반응() {
    let yu = make_교룡();
    let li = make_무백();
    let situation = 배신_상황();

    let yu_initial =
        AppraisalEngine.appraise(yu.personality(), &situation, &neutral_rel().modifiers());
    let li_initial =
        AppraisalEngine.appraise(li.personality(), &situation, &neutral_rel().modifiers());

    let provocation = Pad::new(-0.6, 0.7, 0.5);
    let yu_after = StimulusEngine.apply_stimulus(yu.personality(), &yu_initial, &provocation);
    let li_after = StimulusEngine.apply_stimulus(li.personality(), &li_initial, &provocation);

    let yu_delta = find_emotion(&yu_after, EmotionType::Anger).unwrap()
        - find_emotion(&yu_initial, EmotionType::Anger).unwrap();
    let li_delta = find_emotion(&li_after, EmotionType::Anger).unwrap()
        - find_emotion(&li_initial, EmotionType::Anger).unwrap();

    assert!(
        yu_delta > li_delta,
        "교룡 delta({}) > 무백 delta({}) — patience 차이",
        yu_delta,
        li_delta
    );
}

#[test]
fn 부정_자극_반복이면_분노_계속_증폭() {
    let li = make_무백();
    // 무백이 중간 수준 분노(0.5)인 상태에서 강한 도발 반복
    let mut initial = EmotionState::new();
    initial.add(Emotion::new(EmotionType::Anger, 0.5));

    let provocation = Pad::new(-0.6, 0.7, 0.5);

    let after1 = StimulusEngine.apply_stimulus(li.personality(), &initial, &provocation);
    let after2 = StimulusEngine.apply_stimulus(li.personality(), &after1, &provocation);
    let after3 = StimulusEngine.apply_stimulus(li.personality(), &after2, &provocation);

    let anger0 = find_emotion(&initial, EmotionType::Anger).unwrap();
    let anger1 = find_emotion(&after1, EmotionType::Anger).unwrap();
    let anger2 = find_emotion(&after2, EmotionType::Anger).unwrap();
    let anger3 = find_emotion(&after3, EmotionType::Anger).unwrap();

    assert!(anger1 > anger0, "턴1 증폭: {} → {}", anger0, anger1);
    assert!(anger2 > anger1, "턴2 증폭: {} → {}", anger1, anger2);
    assert!(anger3 > anger2, "턴3 증폭: {} → {}", anger2, anger3);
}

#[test]
fn 긍정_자극이_joy를_증폭() {
    let li = make_무백();
    let situation = Situation::new(
        "좋은 소식",
        Some(EventFocus {
            description: "".into(),
            desirability_for_self: 0.6,
            desirability_for_other: None,
            prospect: None,
        }),
        None,
        None,
    )
    .unwrap();

    let initial =
        AppraisalEngine.appraise(li.personality(), &situation, &neutral_rel().modifiers());
    let joy_before = find_emotion(&initial, EmotionType::Joy).unwrap();

    let positive = Pad::new(0.7, 0.3, 0.2);
    let after = StimulusEngine.apply_stimulus(li.personality(), &initial, &positive);
    let joy_after = find_emotion(&after, EmotionType::Joy).unwrap();

    assert!(
        joy_after > joy_before,
        "긍정 자극 후 Joy 증폭: {} → {}",
        joy_before,
        joy_after
    );
}

#[test]
fn 반대_자극_반복이면_감정_소멸() {
    let li = make_무백();
    let situation = Situation::new(
        "약한 불쾌",
        Some(EventFocus {
            description: "".into(),
            desirability_for_self: -0.2,
            desirability_for_other: None,
            prospect: None,
        }),
        None,
        None,
    )
    .unwrap();

    let initial =
        AppraisalEngine.appraise(li.personality(), &situation, &neutral_rel().modifiers());
    assert!(
        find_emotion(&initial, EmotionType::Distress).is_some(),
        "초기 Distress 존재"
    );

    let positive = Pad::new(0.8, -0.3, 0.3);
    let mut state = initial;
    for _ in 0..20 {
        state = StimulusEngine.apply_stimulus(li.personality(), &state, &positive);
    }

    assert!(
        find_emotion(&state, EmotionType::Distress).is_none(),
        "반복 긍정 자극 후 Distress 소멸"
    );
}

#[test]
fn 자극으로_새_감정이_생기지_않음() {
    let yu = make_교룡();
    let initial =
        AppraisalEngine.appraise(yu.personality(), &배신_상황(), &neutral_rel().modifiers());
    let initial_types: Vec<_> = initial
        .emotions()
        .iter()
        .map(|e| e.emotion_type())
        .collect();

    let positive = Pad::new(0.9, 0.5, 0.3);
    let after = StimulusEngine.apply_stimulus(yu.personality(), &initial, &positive);

    for emotion in after.emotions() {
        assert!(
            initial_types.contains(&emotion.emotion_type()),
            "새 감정 {:?}이 생겨서는 안 됨",
            emotion.emotion_type()
        );
    }
}

#[test]
fn 중립_자극은_감정_변동_없음() {
    let yu = make_교룡();
    let initial =
        AppraisalEngine.appraise(yu.personality(), &배신_상황(), &neutral_rel().modifiers());
    let neutral = Pad::neutral();
    let after = StimulusEngine.apply_stimulus(yu.personality(), &initial, &neutral);

    let anger_before = find_emotion(&initial, EmotionType::Anger).unwrap();
    let anger_after = find_emotion(&after, EmotionType::Anger).unwrap();

    assert!(
        (anger_before - anger_after).abs() < 0.001,
        "중립 자극: 변동 없음 {} → {}",
        anger_before,
        anger_after
    );
}

// ===========================================================================
// 관성(inertia) 검증
// ===========================================================================

#[test]
fn 강한_감정은_약한_감정보다_변동_작음() {
    let yu = make_교룡();
    let apology = Pad::new(0.5, -0.3, -0.4);

    // 강한 Anger (0.8)
    let mut strong = EmotionState::new();
    strong.add(Emotion::new(EmotionType::Anger, 0.8));
    let strong_after = StimulusEngine.apply_stimulus(yu.personality(), &strong, &apology);
    let strong_delta = (find_emotion(&strong_after, EmotionType::Anger).unwrap() - 0.8).abs();

    // 약한 Anger (0.3)
    let mut weak = EmotionState::new();
    weak.add(Emotion::new(EmotionType::Anger, 0.3));
    let weak_after = StimulusEngine.apply_stimulus(yu.personality(), &weak, &apology);
    let weak_delta = (find_emotion(&weak_after, EmotionType::Anger).unwrap() - 0.3).abs();

    assert!(
        strong_delta < weak_delta,
        "강한 감정(0.8) 변동={:.4} < 약한 감정(0.3) 변동={:.4}",
        strong_delta,
        weak_delta
    );
}

#[test]
fn intensity_1에서도_최소_관성으로_변동() {
    let yu = make_교룡();
    let apology = Pad::new(0.5, -0.3, -0.4);

    let mut state = EmotionState::new();
    state.add(Emotion::new(EmotionType::Anger, 1.0));
    let after = StimulusEngine.apply_stimulus(yu.personality(), &state, &apology);
    let anger_after = find_emotion(&after, EmotionType::Anger).unwrap();

    assert!(
        anger_after < 1.0,
        "intensity=1.0에서도 최소 관성으로 변동: {} → {}",
        1.0,
        anger_after
    );
}
