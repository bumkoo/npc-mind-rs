//! PAD 도메인 모델 테스트
//!
//! PAD 구조체, pad_dot, OCC→PAD 매핑 검증

use npc_mind::domain::emotion::EmotionType;
use npc_mind::domain::pad::*;

// ===========================================================================
// 기본 생성
// ===========================================================================

#[test]
fn neutral_pad() {
    let p = Pad::neutral();
    assert_eq!(p.pleasure, 0.0);
    assert_eq!(p.arousal, 0.0);
    assert_eq!(p.dominance, 0.0);
}

#[test]
fn pad_생성() {
    let p = Pad::new(-0.6, 0.7, 0.5);
    assert!((p.pleasure - -0.6).abs() < 0.001);
    assert!((p.arousal - 0.7).abs() < 0.001);
    assert!((p.dominance - 0.5).abs() < 0.001);
}

// ===========================================================================
// pad_dot 내적 계산
// ===========================================================================

#[test]
fn 같은_방향이면_양수_내적() {
    let anger_pad = emotion_to_pad(EmotionType::Anger);
    let provocation = Pad::new(-0.6, 0.7, 0.5);  // 부정, 고각성, 지배적

    let dot = pad_dot(&anger_pad, &provocation);
    assert!(dot > 0.0,
        "Anger PAD와 도발 자극은 같은 방향: {}", dot);
}

#[test]
fn 반대_방향이면_음수_내적() {
    let anger_pad = emotion_to_pad(EmotionType::Anger);
    let apology = Pad::new(0.5, -0.3, -0.4);  // 긍정, 저각성, 복종적

    let dot = pad_dot(&anger_pad, &apology);
    assert!(dot < 0.0,
        "Anger PAD와 사과 자극은 반대 방향: {}", dot);
}

#[test]
fn 중립과의_내적은_0() {
    let anger_pad = emotion_to_pad(EmotionType::Anger);
    let neutral = Pad::neutral();

    let dot = pad_dot(&anger_pad, &neutral);
    assert!((dot - 0.0).abs() < 0.001,
        "중립 PAD와의 내적은 0: {}", dot);
}

#[test]
fn 자극이_강할수록_내적_절대값이_큼() {
    let anger_pad = emotion_to_pad(EmotionType::Anger);

    let weak = Pad::new(-0.2, 0.1, 0.1);     // 약한 부정 자극
    let strong = Pad::new(-0.8, 0.8, 0.6);    // 강한 부정 자극

    let dot_weak = pad_dot(&anger_pad, &weak);
    let dot_strong = pad_dot(&anger_pad, &strong);

    assert!(dot_strong > dot_weak,
        "강한 자극({}) > 약한 자극({}) 내적값",
        dot_strong, dot_weak);
}

// ===========================================================================
// OCC → PAD 매핑 검증
// ===========================================================================

#[test]
fn 부정_감정은_pleasure_음수() {
    let negative_emotions = [
        EmotionType::Distress, EmotionType::Fear,
        EmotionType::Anger, EmotionType::Shame,
        EmotionType::Disappointment, EmotionType::Hate,
    ];
    for etype in &negative_emotions {
        let pad = emotion_to_pad(*etype);
        assert!(pad.pleasure < 0.0,
            "{:?}의 P는 음수여야 함: {}", etype, pad.pleasure);
    }
}

#[test]
fn 긍정_감정은_pleasure_양수() {
    let positive_emotions = [
        EmotionType::Joy, EmotionType::Hope,
        EmotionType::Pride, EmotionType::Gratitude,
        EmotionType::Satisfaction, EmotionType::Love,
    ];
    for etype in &positive_emotions {
        let pad = emotion_to_pad(*etype);
        assert!(pad.pleasure > 0.0,
            "{:?}의 P는 양수여야 함: {}", etype, pad.pleasure);
    }
}

#[test]
fn anger는_불쾌_고각성_지배적() {
    let pad = emotion_to_pad(EmotionType::Anger);
    assert!(pad.pleasure < 0.0, "Anger P 음수: {}", pad.pleasure);
    assert!(pad.arousal > 0.0, "Anger A 양수: {}", pad.arousal);
    assert!(pad.dominance > 0.0, "Anger D 양수: {}", pad.dominance);
}

#[test]
fn fear는_불쾌_고각성_복종적() {
    let pad = emotion_to_pad(EmotionType::Fear);
    assert!(pad.pleasure < 0.0, "Fear P 음수: {}", pad.pleasure);
    assert!(pad.arousal > 0.0, "Fear A 양수: {}", pad.arousal);
    assert!(pad.dominance < 0.0, "Fear D 음수: {}", pad.dominance);
}

#[test]
fn anger와_fear는_dominance로_구분됨() {
    // PAD 논의에서 핵심: D축이 분노와 두려움을 갈라줌
    let anger = emotion_to_pad(EmotionType::Anger);
    let fear = emotion_to_pad(EmotionType::Fear);

    // 둘 다 P 음수, A 양수이지만 D가 다름
    assert!(anger.dominance > 0.0, "Anger D 양수 (지배적)");
    assert!(fear.dominance < 0.0, "Fear D 음수 (복종적)");
}

#[test]
fn shame은_불쾌_저각성_복종적() {
    let pad = emotion_to_pad(EmotionType::Shame);
    assert!(pad.pleasure < 0.0, "Shame P 음수");
    assert!(pad.dominance < 0.0, "Shame D 음수 (위축)");
}

// ===========================================================================
// 무협 대사 PAD 시나리오
// ===========================================================================

#[test]
fn 도발_자극은_anger와_공명하고_joy와_반발() {
    let provocation = Pad::new(-0.6, 0.7, 0.5);

    let anger_dot = pad_dot(&emotion_to_pad(EmotionType::Anger), &provocation);
    let joy_dot = pad_dot(&emotion_to_pad(EmotionType::Joy), &provocation);

    assert!(anger_dot > 0.0, "도발 → Anger 공명: {}", anger_dot);
    assert!(joy_dot < 0.0, "도발 → Joy 반발: {}", joy_dot);
}

#[test]
fn 사과_자극은_anger를_감소시킴() {
    let apology = Pad::new(0.5, -0.3, -0.4);

    let anger_dot = pad_dot(&emotion_to_pad(EmotionType::Anger), &apology);
    assert!(anger_dot < 0.0, "사과 → Anger 감소: {}", anger_dot);
}

#[test]
fn 감사_자극은_gratitude와_공명() {
    let thanks = Pad::new(0.7, 0.3, -0.3);

    let gratitude_dot = pad_dot(&emotion_to_pad(EmotionType::Gratitude), &thanks);
    assert!(gratitude_dot > 0.0, "감사 자극 → Gratitude 공명: {}", gratitude_dot);
}

#[test]
fn 위협_자극은_fear와_공명하고_anger와도_부분_공명() {
    // "네놈을 죽이겠다" — 불쾌, 고각성, 지배적 (위협하는 쪽 관점)
    let threat = Pad::new(-0.8, 0.8, 0.6);

    let fear_dot = pad_dot(&emotion_to_pad(EmotionType::Fear), &threat);
    let anger_dot = pad_dot(&emotion_to_pad(EmotionType::Anger), &threat);

    // Fear: P 공명, A 공명, D 반발 → 양수이지만 Anger보다 약함
    // Anger: P 공명, A 공명, D 공명 → 전축 공명으로 더 강함
    assert!(fear_dot > 0.0, "위협 → Fear 공명: {}", fear_dot);
    assert!(anger_dot > fear_dot,
        "위협 → Anger({}) > Fear({}) — D축 차이",
        anger_dot, fear_dot);
}
