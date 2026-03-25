//! PAD 도메인 모델 테스트
//!
//! PAD 구조체, pad_dot(P·A 방향 × D 격차 스케일러), OCC→PAD 매핑 검증
//!
//! pad_dot 공식: (P_a × P_b + A_a × A_b) × (1.0 + |D_a - D_b| × 0.3)
//! - P·A 내적이 공명 방향(증폭/감소)을 결정
//! - D축 차이가 그 효과의 강도를 배율로 조절
//! - 상세: docs/pad-stimulus-design-decisions.md

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
// pad_dot: P·A 공명 × D 격차 스케일러
// ===========================================================================

#[test]
fn 같은_방향이면_양수_내적() {
    let anger_pad = emotion_to_pad(EmotionType::Anger);
    let provocation = Pad::new(-0.6, 0.7, 0.5);

    let dot = pad_dot(&anger_pad, &provocation);
    // P·A: (-0.51)×(-0.6) + (0.59)×(0.7) = 0.306 + 0.413 = 0.719
    // D_scale: 1 + |0.25-0.5|×0.3 = 1.075
    // result: 0.719 × 1.075 = 0.773
    assert!(dot > 0.0,
        "Anger PAD와 도발 자극은 P·A 공명: {}", dot);
}

#[test]
fn 반대_방향이면_음수_내적() {
    let anger_pad = emotion_to_pad(EmotionType::Anger);
    let apology = Pad::new(0.5, -0.3, -0.4);

    let dot = pad_dot(&anger_pad, &apology);
    // P·A: (-0.51)×(0.5) + (0.59)×(-0.3) = -0.255 - 0.177 = -0.432
    // D_scale: 1 + |0.25-(-0.4)|×0.3 = 1.195
    // result: -0.432 × 1.195 = -0.516
    assert!(dot < 0.0,
        "Anger PAD와 사과 자극은 P·A 반발: {}", dot);
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

    let weak = Pad::new(-0.2, 0.1, 0.1);
    let strong = Pad::new(-0.8, 0.8, 0.6);

    let dot_weak = pad_dot(&anger_pad, &weak);
    let dot_strong = pad_dot(&anger_pad, &strong);

    assert!(dot_strong > dot_weak,
        "강한 자극({}) > 약한 자극({}) 내적값",
        dot_strong, dot_weak);
}

// ===========================================================================
// D축 격차 스케일러 검증
// ===========================================================================

#[test]
fn d_격차가_클수록_효과가_강함() {
    // 같은 P·A 자극, D만 다른 두 자극
    let anger_pad = emotion_to_pad(EmotionType::Anger); // D:+0.25

    // 비슷한 D → 스케일러 작음
    let same_d = Pad::new(-0.6, 0.7, 0.3);  // |0.25-0.3| = 0.05
    // 먼 D → 스케일러 큼
    let far_d = Pad::new(-0.6, 0.7, -0.8);  // |0.25-(-0.8)| = 1.05

    let dot_same = pad_dot(&anger_pad, &same_d);
    let dot_far = pad_dot(&anger_pad, &far_d);

    assert!(dot_far > dot_same,
        "D 격차 큰 자극({}) > D 격차 작은 자극({}) — 스케일러 효과",
        dot_far, dot_same);
}

#[test]
fn d_격차가_0이면_스케일러_1() {
    // D가 동일하면 스케일러 = 1.0 → P·A 결과 그대로
    let a = Pad::new(-0.5, 0.5, 0.3);
    let b = Pad::new(-0.5, 0.5, 0.3); // 동일한 D

    let dot = pad_dot(&a, &b);
    let pa_only = a.pleasure * b.pleasure + a.arousal * b.arousal;

    assert!((dot - pa_only).abs() < 0.001,
        "D 격차 0 → 스케일러 1.0 → P·A 결과 그대로: dot={}, pa={}", dot, pa_only);
}

#[test]
fn d_스케일러는_방향을_바꾸지_않음() {
    // P·A가 음수(감소)인 상황에서 D 격차가 커도 음수 유지
    let shame_pad = emotion_to_pad(EmotionType::Shame); // D:-0.60
    let comfort = Pad::new(0.5, -0.3, 0.5);  // D:+0.5 → 큰 격차

    let dot = pad_dot(&shame_pad, &comfort);
    // P·A: -0.15 - 0.03 = -0.18 (감소 방향)
    // D_scale: 1 + |(-0.60)-(0.5)|×0.3 = 1.33
    // result: -0.18 × 1.33 = -0.239 → 여전히 음수 (감소)
    assert!(dot < 0.0,
        "D 스케일러는 P·A 방향을 유지: {}", dot);
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
    let anger = emotion_to_pad(EmotionType::Anger);
    let fear = emotion_to_pad(EmotionType::Fear);

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
fn 위협_자극은_fear와_anger_모두_공명하되_fear가_더_강함() {
    // "네놈을 죽이겠다" — 불쾌, 고각성, 지배적 (위협하는 쪽 관점)
    let threat = Pad::new(-0.8, 0.8, 0.6);

    let fear_dot = pad_dot(&emotion_to_pad(EmotionType::Fear), &threat);
    let anger_dot = pad_dot(&emotion_to_pad(EmotionType::Anger), &threat);

    // Fear: P·A(+0.992) × D_scale(1+|(-0.43)-0.6|×0.3 = 1.31) = 1.30
    // Anger: P·A(+0.880) × D_scale(1+|0.25-0.6|×0.3 = 1.105) = 0.972
    // Fear > Anger — 위협엔 두려움이 먼저 (P·A가 더 크고 D 격차도 더 큼)
    assert!(fear_dot > 0.0, "위협 → Fear 공명: {}", fear_dot);
    assert!(anger_dot > 0.0, "위협 → Anger도 공명: {}", anger_dot);
    assert!(fear_dot > anger_dot,
        "위협 → Fear({}) > Anger({}) — 위협엔 두려움이 먼저",
        fear_dot, anger_dot);
}

// ===========================================================================
// D축 스케일러 핵심 검증: Shame, Fear에서 자연스러운 결과
//
// 기존 3축 내적의 문제:
// - Shame(D:-0.60) + 비난(D:+0.5) → D 내적 반발 → Shame 안 변함 ❌
// - Shame(D:-0.60) + 위로(D:-0.4) → D 내적 공명 → Shame 증폭 ❌
//
// D 스케일러 방안:
// - P·A가 방향, D 격차가 강도 → 모든 시나리오 자연스러움
// ===========================================================================

#[test]
fn 비난_자극은_shame을_증폭_d격차로_더_강하게() {
    // "네가 한 짓을 부끄럽게 여겨라!" — 불쾌, 고각성, 지배적
    let rebuke = Pad::new(-0.6, 0.7, 0.5);
    let shame_dot = pad_dot(&emotion_to_pad(EmotionType::Shame), &rebuke);

    // P·A: +0.18 + 0.07 = +0.25 (증폭 방향)
    // D_scale: 1 + |(-0.60)-(+0.5)|×0.3 = 1.33
    // result: +0.25 × 1.33 = +0.333
    assert!(shame_dot > 0.25,
        "비난 → Shame 증폭, D 격차로 P·A보다 강함: {} (P·A만이면 0.25)", shame_dot);
}

#[test]
fn 위로_자극은_shame을_감소() {
    // "괜찮소, 누구나 실수하오" — 쾌, 저각성, 복종적
    let comfort = Pad::new(0.5, -0.3, -0.4);
    let shame_dot = pad_dot(&emotion_to_pad(EmotionType::Shame), &comfort);

    // P·A: -0.15 - 0.03 = -0.18 (감소 방향)
    // D_scale: 1 + |(-0.60)-(-0.4)|×0.3 = 1.06 (비슷한 D → 약한 스케일링)
    // result: -0.18 × 1.06 = -0.191
    assert!(shame_dot < 0.0,
        "위로 → Shame 감소: {}", shame_dot);
}

#[test]
fn 위로_자극은_fear를_감소_보호자일수록_효과적() {
    // "걱정 마시오, 내가 지켜주겠소" — 쾌, 저각성, 지배적(보호)
    let reassure = Pad::new(0.6, -0.2, 0.5);
    let fear_dot = pad_dot(&emotion_to_pad(EmotionType::Fear), &reassure);

    // P·A: -0.384 - 0.12 = -0.504 (감소 방향)
    // D_scale: 1 + |(-0.43)-(+0.5)|×0.3 = 1.28 (큰 격차 → 강한 스케일링)
    // result: -0.504 × 1.28 = -0.645
    assert!(fear_dot < -0.5,
        "보호자 위로 → Fear 강한 감소 (D 격차 효과): {}", fear_dot);
}

#[test]
fn 위협_자극은_fear를_증폭_강자일수록_효과적() {
    // "항복하지 않으면 모두 죽는다" — 강한 불쾌, 고각성, 지배적
    let threat = Pad::new(-0.8, 0.8, 0.6);
    let fear_dot = pad_dot(&emotion_to_pad(EmotionType::Fear), &threat);

    // P·A: +0.512 + 0.48 = +0.992 (증폭 방향)
    // D_scale: 1 + |(-0.43)-(+0.6)|×0.3 = 1.31 (큰 격차 → 강한 스케일링)
    // result: +0.992 × 1.31 = +1.30
    assert!(fear_dot > 1.0,
        "강자 위협 → Fear 매우 강한 증폭: {}", fear_dot);
}

#[test]
fn 같은_사과도_복종적이면_anger에_더_효과적() {
    let anger_pad = emotion_to_pad(EmotionType::Anger); // D:+0.25

    // 당당한 사과 (D:+0.3) — D 격차 작음
    let proud_apology = Pad::new(0.5, -0.3, 0.3);
    // 비굴한 사과 (D:-0.5) — D 격차 큼
    let humble_apology = Pad::new(0.5, -0.3, -0.5);

    let dot_proud = pad_dot(&anger_pad, &proud_apology);
    let dot_humble = pad_dot(&anger_pad, &humble_apology);

    // 둘 다 음수(감소)지만 비굴한 사과가 더 효과적
    assert!(dot_proud < 0.0, "당당한 사과도 Anger 감소: {}", dot_proud);
    assert!(dot_humble < dot_proud,
        "비굴한 사과({}) < 당당한 사과({}) — 더 많이 감소 (D 격차 효과)",
        dot_humble, dot_proud);
}
