//! 테스트 커버리지 갭 보완 테스트
//!
//! P1: overall_valence, merge_from_beat 경계값
//! P2: 역방향 관계 조회, emotion_to_pad 전수 검증, format_prompt 엣지케이스
//! P3: trust/closeness 수식 정밀 검증

mod common;

use common::{TestContext, make_무백, score as s};
use npc_mind::application::dto::*;
use npc_mind::domain::emotion::*;
use npc_mind::domain::guide::ActingGuide;
use npc_mind::domain::pad::{Pad, emotion_to_pad, pad_dot};
use npc_mind::domain::relationship::RelationshipBuilder;
use npc_mind::ports::GuideFormatter;
use npc_mind::presentation::korean::KoreanFormatter;

// ===========================================================================
// P1: overall_valence() 유닛 테스트
// ===========================================================================

#[test]
fn overall_valence_빈_상태는_0() {
    let state = EmotionState::new();
    assert_eq!(state.overall_valence(), 0.0, "감정 없으면 valence=0.0");
}

#[test]
fn overall_valence_단일_긍정_감정() {
    let mut state = EmotionState::new();
    state.add(Emotion::new(EmotionType::Joy, 0.8));

    let v = state.overall_valence();
    // Joy의 base_valence=1.0, intensity=0.8 → valence = 1.0 * 0.8 / 1 = 0.8
    assert!(
        (v - 0.8).abs() < 0.001,
        "Joy(0.8) → valence={v}, expected 0.8"
    );
}

#[test]
fn overall_valence_단일_부정_감정() {
    let mut state = EmotionState::new();
    state.add(Emotion::new(EmotionType::Anger, 0.6));

    let v = state.overall_valence();
    // Anger base_valence=-1.0, intensity=0.6 → -0.6 / 1 = -0.6
    assert!(
        (v - (-0.6)).abs() < 0.001,
        "Anger(0.6) → valence={v}, expected -0.6"
    );
}

#[test]
fn overall_valence_양음_혼합_가중평균() {
    let mut state = EmotionState::new();
    state.add(Emotion::new(EmotionType::Joy, 0.8)); // +1.0 * 0.8 = +0.8
    state.add(Emotion::new(EmotionType::Anger, 0.4)); // -1.0 * 0.4 = -0.4

    let v = state.overall_valence();
    // (0.8 + (-0.4)) / 2 = 0.2
    assert!(
        (v - 0.2).abs() < 0.001,
        "Joy(0.8)+Anger(0.4) → valence={v}, expected 0.2"
    );
}

#[test]
fn overall_valence_gloating의_특수_valence() {
    let mut state = EmotionState::new();
    state.add(Emotion::new(EmotionType::Gloating, 1.0));

    let v = state.overall_valence();
    // Gloating base_valence=0.5, intensity=1.0 → 0.5
    assert!(
        (v - 0.5).abs() < 0.001,
        "Gloating(1.0) → valence={v}, expected 0.5"
    );
}

// ===========================================================================
// P1: merge_from_beat() 경계값 테스트
// ===========================================================================

#[test]
fn merge_동일_강도면_새것이_유지() {
    let mut prev = EmotionState::new();
    prev.add(Emotion::with_context(EmotionType::Anger, 0.5, "이전 원인"));
    let mut new_state = EmotionState::new();
    new_state.add(Emotion::with_context(EmotionType::Anger, 0.5, "새 원인"));

    let merged = EmotionState::merge_from_beat(&prev, &new_state, 0.2);

    assert!((merged.intensity_of(EmotionType::Anger) - 0.5).abs() < 0.001);
    // 동일 강도 → 새것이 유지 (prev > new가 아니므로)
    assert_eq!(
        merged.context_of(EmotionType::Anger).unwrap(),
        "새 원인",
        "동일 강도면 new_state의 context 유지"
    );
}

#[test]
fn merge_threshold_0이면_모든_이전_감정_유지() {
    let mut prev = EmotionState::new();
    prev.add(Emotion::new(EmotionType::Fear, 0.01)); // 아주 약한 감정
    let new_state = EmotionState::new();

    let merged = EmotionState::merge_from_beat(&prev, &new_state, 0.0);

    assert!(
        merged.intensity_of(EmotionType::Fear) > 0.0,
        "threshold=0 → 이전의 아주 약한 감정도 유지"
    );
}

#[test]
fn merge_양쪽_다_여러_감정_있으면_각각_처리() {
    let mut prev = EmotionState::new();
    prev.add(Emotion::new(EmotionType::Anger, 0.8)); // prev > new → 유지
    prev.add(Emotion::new(EmotionType::Fear, 0.3)); // prev < new → new 유지
    prev.add(Emotion::new(EmotionType::Shame, 0.1)); // threshold 미만 → 소멸

    let mut new_state = EmotionState::new();
    new_state.add(Emotion::new(EmotionType::Anger, 0.4));
    new_state.add(Emotion::new(EmotionType::Fear, 0.6));
    new_state.add(Emotion::new(EmotionType::Joy, 0.7)); // new에만 있음

    let merged = EmotionState::merge_from_beat(&prev, &new_state, 0.2);

    assert!(
        (merged.intensity_of(EmotionType::Anger) - 0.8).abs() < 0.001,
        "Anger: prev(0.8) > new(0.4) → 0.8 유지"
    );
    assert!(
        (merged.intensity_of(EmotionType::Fear) - 0.6).abs() < 0.001,
        "Fear: new(0.6) > prev(0.3) → 0.6 유지"
    );
    assert_eq!(
        merged.intensity_of(EmotionType::Shame),
        0.0,
        "Shame: prev(0.1) < threshold(0.2) → 소멸"
    );
    assert!(
        (merged.intensity_of(EmotionType::Joy) - 0.7).abs() < 0.001,
        "Joy: new에만 있음 → 0.7 그대로"
    );
}

// ===========================================================================
// P2: update_relationship 역방향 조회 테스트
// ===========================================================================

#[test]
fn 역방향_관계_조회로_after_dialogue_성공() {
    let mut ctx = TestContext::new();

    // 관계가 "mu_baek:gyo_ryong"으로 저장되어 있음
    // after_dialogue를 "gyo_ryong → mu_baek" 방향으로 호출
    let npc_id = "gyo_ryong";
    let partner_id = "mu_baek";

    // appraise 먼저 실행 (emotion_state 필요)
    let appraise_req = AppraiseRequest {
        npc_id: npc_id.into(),
        partner_id: partner_id.into(),
        situation: Some(SituationInput {
            description: "좋은 일이 있었다".into(),
            event: Some(EventInput {
                description: "".into(),
                desirability_for_self: 0.5,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
        }),
    };

    let mut service = ctx.service();
    service.appraise(appraise_req, || {}, Vec::new).unwrap();

    // 역방향 after_dialogue
    let result = service.after_dialogue(AfterDialogueRequest {
        npc_id: npc_id.into(),
        partner_id: partner_id.into(),
        significance: Some(0.3),
    });

    assert!(
        result.is_ok(),
        "역방향 관계 조회로 after_dialogue 성공해야 함: {:?}",
        result.err()
    );
}

#[test]
fn 관계_없으면_after_dialogue_에러() {
    let mut ctx = TestContext::new();

    // 존재하지 않는 관계
    let result = ctx.service().after_dialogue(AfterDialogueRequest {
        npc_id: "mu_baek".into(),
        partner_id: "unknown".into(),
        significance: None,
    });

    assert!(result.is_err(), "관계 없으면 에러");
}

// ===========================================================================
// P2: emotion_to_pad() 전체 22개 좌표 검증
// ===========================================================================

#[test]
fn emotion_to_pad_22개_전수_검증() {
    // 각 감정의 PAD 좌표가 올바른지, P/A/D 뒤바뀜이 없는지 검증
    let expected: Vec<(EmotionType, f32, f32, f32)> = vec![
        // Event: Well-being
        (EmotionType::Joy, 0.40, 0.20, 0.10),
        (EmotionType::Distress, -0.40, -0.20, -0.50),
        // Event: Fortune-of-others
        (EmotionType::HappyFor, 0.40, 0.20, 0.20),
        (EmotionType::Pity, -0.40, -0.20, -0.50),
        (EmotionType::Gloating, 0.30, -0.30, -0.10),
        (EmotionType::Resentment, -0.20, -0.30, -0.20),
        // Event: Prospect-based
        (EmotionType::Hope, 0.20, 0.20, -0.10),
        (EmotionType::Fear, -0.64, 0.60, -0.43),
        (EmotionType::Satisfaction, 0.30, -0.20, 0.40),
        (EmotionType::Disappointment, -0.30, 0.10, -0.40),
        (EmotionType::Relief, 0.20, -0.30, 0.40),
        (EmotionType::FearsConfirmed, -0.50, 0.30, -0.60),
        // Action: Attribution
        (EmotionType::Pride, 0.40, 0.30, 0.30),
        (EmotionType::Shame, -0.30, 0.10, -0.90),
        (EmotionType::Admiration, 0.50, 0.30, -0.20),
        (EmotionType::Reproach, -0.30, -0.10, 0.40),
        // Action: Compound
        (EmotionType::Gratification, 0.60, 0.50, 0.40),
        (EmotionType::Remorse, -0.30, 0.10, -0.60),
        (EmotionType::Gratitude, 0.40, 0.20, -0.30),
        (EmotionType::Anger, -0.51, 0.59, 0.25),
        // Object
        (EmotionType::Love, 0.30, 0.10, 0.20),
        (EmotionType::Hate, -0.60, 0.60, 0.30),
    ];

    for (etype, exp_p, exp_a, exp_d) in expected {
        let pad = emotion_to_pad(etype);
        assert!(
            (pad.pleasure - exp_p).abs() < 0.001,
            "{:?} P: got {}, expected {}",
            etype,
            pad.pleasure,
            exp_p
        );
        assert!(
            (pad.arousal - exp_a).abs() < 0.001,
            "{:?} A: got {}, expected {}",
            etype,
            pad.arousal,
            exp_a
        );
        assert!(
            (pad.dominance - exp_d).abs() < 0.001,
            "{:?} D: got {}, expected {}",
            etype,
            pad.dominance,
            exp_d
        );
    }
}

#[test]
fn emotion_to_pad_긍정_감정은_p_양수() {
    let positive = [
        EmotionType::Joy,
        EmotionType::HappyFor,
        EmotionType::Hope,
        EmotionType::Satisfaction,
        EmotionType::Relief,
        EmotionType::Pride,
        EmotionType::Admiration,
        EmotionType::Gratification,
        EmotionType::Gratitude,
        EmotionType::Love,
    ];
    for etype in positive {
        let pad = emotion_to_pad(etype);
        assert!(
            pad.pleasure > 0.0,
            "{:?}는 긍정 감정이므로 P > 0 이어야 함 (got {})",
            etype,
            pad.pleasure
        );
    }
}

#[test]
fn emotion_to_pad_부정_감정은_p_음수() {
    let negative = [
        EmotionType::Distress,
        EmotionType::Pity,
        EmotionType::Resentment,
        EmotionType::Fear,
        EmotionType::Disappointment,
        EmotionType::FearsConfirmed,
        EmotionType::Shame,
        EmotionType::Reproach,
        EmotionType::Remorse,
        EmotionType::Anger,
        EmotionType::Hate,
    ];
    for etype in negative {
        let pad = emotion_to_pad(etype);
        assert!(
            pad.pleasure < 0.0,
            "{:?}는 부정 감정이므로 P < 0 이어야 함 (got {})",
            etype,
            pad.pleasure
        );
    }
}

// ===========================================================================
// P2: pad_dot() 엣지케이스
// ===========================================================================

#[test]
fn pad_dot_동일_pad_자기정렬() {
    let joy = emotion_to_pad(EmotionType::Joy); // P:0.40, A:0.20, D:0.10
    let result = pad_dot(&joy, &joy);
    // P*P + A*A = 0.16 + 0.04 = 0.20, D_gap=0 → 0.20 * 1.0 = 0.20
    assert!(
        (result - 0.20).abs() < 0.001,
        "자기 정렬: {result}, expected 0.20"
    );
}

#[test]
fn pad_dot_최대_d_격차_스케일링() {
    let a = Pad::new(0.5, 0.5, 1.0);
    let b = Pad::new(0.5, 0.5, -1.0);
    let result = pad_dot(&a, &b);
    // PA = 0.25 + 0.25 = 0.5, D_gap = 2.0 → 0.5 * (1 + 2.0 * 0.3) = 0.5 * 1.6 = 0.8
    assert!(
        (result - 0.8).abs() < 0.001,
        "최대 D 격차: {result}, expected 0.8"
    );
}

// ===========================================================================
// P2: format_prompt() 엣지케이스
// ===========================================================================

#[test]
fn format_prompt_빈_감정_상태() {
    let li = make_무백();
    let state = EmotionState::new();
    let guide = ActingGuide::build(&li, &state, None, None, "");
    let formatter = KoreanFormatter::new();
    let prompt = formatter.format_prompt(&guide);

    assert!(prompt.contains("[현재 감정]"), "감정 섹션 헤더는 항상 존재");
    assert!(prompt.contains("전체 분위기"), "분위기 라벨은 항상 존재");
    assert!(!prompt.contains("감정 구성:"), "빈 감정이면 감정 구성 없음");
    assert!(!prompt.contains("[상황]"), "상황 없으면 상황 섹션 없음");
    assert!(
        !prompt.contains("[상대와의 관계]"),
        "관계 없으면 관계 섹션 없음"
    );
}

#[test]
fn format_prompt_관계_포함_빈_감정() {
    let li = make_무백();
    let state = EmotionState::new();
    let brother = RelationshipBuilder::new("mu_baek", "gyo_ryong")
        .closeness(s(0.8))
        .trust(s(0.7))
        .build();
    let guide = ActingGuide::build(&li, &state, Some("테스트 상황".into()), Some(&brother), "교룡");
    let formatter = KoreanFormatter::new();
    let prompt = formatter.format_prompt(&guide);

    assert!(prompt.contains("[상황]"), "상황 있으면 상황 섹션 존재");
    assert!(prompt.contains("테스트 상황"), "상황 설명 포함");
    assert!(
        prompt.contains("[상대와의 관계:") || prompt.contains("[상대와의 관계]"),
        "관계 있으면 관계 섹션 존재"
    );
}

// ===========================================================================
// P3: trust/closeness 수식 정밀 검증
// ===========================================================================


#[test]
fn closeness_음수_valence면_하락() {
    let rel = RelationshipBuilder::new("a", "b").closeness(s(0.3)).build();
    let updated = rel.with_updated_closeness(-0.6, 0.0);
    assert!(
        updated.closeness().value() < 0.3,
        "음수 valence → closeness 하락: {}",
        updated.closeness().value()
    );

    // delta = -0.6 * 0.05 * 1.0 = -0.03
    let expected = 0.3 - 0.03;
    assert!(
        (updated.closeness().value() - expected).abs() < 0.001,
        "closeness: got {}, expected {}",
        updated.closeness().value(),
        expected
    );
}

#[test]
fn closeness_경계값_상한_클램핑() {
    // closeness가 이미 높은 상태에서 큰 긍정 valence → 1.0 클램핑
    let rel = RelationshipBuilder::new("a", "b").closeness(s(0.95)).build();
    let updated = rel.with_updated_closeness(1.0, 1.0);
    assert!(
        updated.closeness().value() <= 1.0,
        "closeness 상한 클램핑: {}",
        updated.closeness().value()
    );
}

#[test]
fn closeness_경계값_하한_클램핑() {
    let rel = RelationshipBuilder::new("a", "b").closeness(s(-0.95)).build();
    let updated = rel.with_updated_closeness(-1.0, 1.0);
    assert!(
        updated.closeness().value() >= -1.0,
        "closeness 하한 클램핑: {}",
        updated.closeness().value()
    );
}


#[test]
fn after_dialogue_power는_변경_없음() {
    let rel = RelationshipBuilder::new("a", "b").power(s(0.7)).build();

    let mut state = EmotionState::new();
    state.add(Emotion::new(EmotionType::Anger, 0.9));

    let updated = rel.after_dialogue(&state, 0.5);

    assert!(
        (updated.power().value() - 0.7).abs() < 0.001,
        "power는 after_dialogue에서 변경 안 됨: {}",
        updated.power().value()
    );
}
