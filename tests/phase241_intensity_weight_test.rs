//! Phase 2.4.1 intensity weight 검증 게이트 (spec §6 gate 3·4·5).
//!
//! FROZEN spec: `docs/tasks/mind-architecture/task-rel-phase2.4.1-intensity-weight.md`.
//! 변경 4 weight 함수(self/prospect/confirmation/praiseworthiness)의 정량 동작을
//! 박제 — 회귀 가드 + 게이트 재현. `effect(w)=facet×w` 선형이라 순수 계산으로 검증.
//!
//!   cargo test --test phase241_intensity_weight_test -- --nocapture
//!
//! ## 불변 케이스 deviation 0 (gate 3)
//! empathy/hostility/appealingness/stimulus 함수는 **미변경**(git diff 확인) →
//! 출력 항등. fear-lifecycle(Distress/Fear/Relief/FearsConfirmed)도 공식 보존 —
//! 본 파일 `gate3_unchanged_*` 가 보존값을 박제. emotion_test(52P)·pad_test(24P)
//! 무회귀가 추가 보증.

use npc_mind::domain::personality::{NpcBuilder, Score};
use npc_mind::ports::AppraisalWeights;

const EPS: f32 = 1e-5;

fn s(v: f32) -> Score {
    Score::new(v, "t").unwrap()
}

/// prudence 한 facet만 세팅, 나머지 0 (gate 4 통제군 fixture 정합).
fn prudence_only(prud: f32) -> npc_mind::domain::personality::Npc {
    NpcBuilder::new("p", "p")
        .conscientiousness(|c| c.prudence = s(prud))
        .build()
}

// ---------------------------------------------------------------------------
// Gate 4a — prudence → Reproach 평탄화 (셋 다 동일). 분해 후 prudence는 도덕
// 평가에서 제외되므로 hi/mid/lo가 동일 weight. (2.4.0 실측 0.700→0.742 오염 제거.)
// ---------------------------------------------------------------------------
#[test]
fn gate4a_prudence_does_not_affect_praiseworthiness() {
    let reproach = |prud: f32| prudence_only(prud).personality().praiseworthiness_weight(false, -0.7);
    let admire = |prud: f32| prudence_only(prud).personality().praiseworthiness_weight(false, 0.7);

    let (hi, mid, lo) = (reproach(0.8), reproach(0.0), reproach(-0.8));
    println!("[gate4a] Reproach weight — prud +0.8={hi} / 0={mid} / -0.8={lo} (구: 0.94/1.0/1.06 류 차등)");
    assert_eq!(hi, mid, "prudence는 더 이상 Reproach 강도에 영향 없음");
    assert_eq!(mid, lo, "prudence는 더 이상 Reproach 강도에 영향 없음");
    assert!((mid - 1.0).abs() < EPS, "나머지 facet 0 → Reproach weight = base 1.0");

    // Admiration도 동일 평탄화
    assert_eq!(admire(0.8), admire(-0.8), "prudence는 Admiration에도 영향 없음");
}

// ---------------------------------------------------------------------------
// Gate 4b — 정서성(E) 인물 Joy 하락. ①로 Joy는 X만 사용 → E 무관.
// Distress는 여전히 E로 증폭(불변).
// ---------------------------------------------------------------------------
#[test]
fn gate4b_emotionality_drops_joy_keeps_distress() {
    let emo = NpcBuilder::new("e", "e")
        .emotionality(|e| {
            e.fearfulness = s(0.8);
            e.anxiety = s(0.8);
            e.dependence = s(0.8);
            e.sentimentality = s(0.8);
        })
        .build(); // avg.e = 0.8

    let joy = emo.personality().desirability_self_weight(1.0);
    let distress = emo.personality().desirability_self_weight(-1.0);
    println!("[gate4b] E=0.8 — Joy={joy} (2.4.0 구값 1.24 → 2.4.1 1.00), Distress={distress} (불변 1.24)");

    assert!((joy - 1.0).abs() < EPS, "Joy = base + X(0) = 1.0 — E 제거");
    assert!((distress - 1.24).abs() < EPS, "Distress = base + E(0.24) = 1.24 — 불변");
    assert!(distress > joy, "E 인물의 증폭은 Joy가 아니라 Distress로 이동");
}

// ---------------------------------------------------------------------------
// Gate 5 — 저-X(외향성) 인물 Hope/Satisfaction 의 0.5 floor 접촉.
// Hope = Satisfaction = base + X − Pru. 극단 저-X(-1.0)+고 prudence(1.0)에서
// e = -0.3 - 0.2 = -0.5 → CLAMP_STANDARD 하한 0.5.
// ---------------------------------------------------------------------------
#[test]
fn gate5_low_extraversion_hope_satisfaction_clamp_floor() {
    let low_x_prud = NpcBuilder::new("l", "l")
        .extraversion(|x| {
            x.social_self_esteem = s(-1.0);
            x.social_boldness = s(-1.0);
            x.sociability = s(-1.0);
            x.liveliness = s(-1.0);
        })
        .conscientiousness(|c| c.prudence = s(1.0))
        .build(); // avg.x = -1.0, prud = 1.0

    let hope = low_x_prud.personality().desirability_prospect_weight(1.0);
    let satisfaction = low_x_prud.personality().desirability_confirmation_weight(false);
    println!("[gate5] low-X(-1.0)+prud(1.0) — Hope={hope}, Satisfaction={satisfaction} (floor 0.5 접촉)");
    assert!((hope - 0.5).abs() < EPS, "Hope = base + X(-0.3) - Pru(0.2) = 0.5 (floor)");
    assert!((satisfaction - 0.5).abs() < EPS, "Satisfaction(hope축) 동일 공식 → 0.5 (floor)");

    // 대조: prudence 0이면 floor 미접촉(0.7) — clamp이 극단에서만 작동함을 보증.
    let low_x_only = NpcBuilder::new("l2", "l2")
        .extraversion(|x| {
            x.social_self_esteem = s(-1.0);
            x.social_boldness = s(-1.0);
            x.sociability = s(-1.0);
            x.liveliness = s(-1.0);
        })
        .build();
    let hope_no_floor = low_x_only.personality().desirability_prospect_weight(1.0);
    assert!((hope_no_floor - 0.7).abs() < EPS, "X(-1.0)만: 0.7 — floor 미접촉");
}

// ---------------------------------------------------------------------------
// Gate 3 (변경 케이스) — praiseworthiness facet 비대칭 박제.
// 프로필: diligence 0.5 / perfectionism 0.5 / modesty 0.4 / gentleness 0.4.
//   dil = 0.5×0.10 = 0.05 (공통)
//   Pride       = 1.0 + 0.05 + (−0.5×0.10) + (−0.4×0.3) = 1.0 + 0.05 −0.05 −0.12 = 0.88
//   Shame       = 1.0 + 0.05 + ( 0.5×0.20) + ( 0.4×0.3) = 1.0 + 0.05 +0.10 +0.12 = 1.27
//   Admiration  = 1.0 + 0.05 + ( 0.5×0.15) + ( 0.4×0.3) = 1.0 + 0.05 +0.075+0.12 = 1.245
//   Reproach    = 1.0 + 0.05 + ( 0.5×0.20) + (−0.4×0.3) = 1.0 + 0.05 +0.10 −0.12 = 1.03
// ---------------------------------------------------------------------------
#[test]
fn gate3_changed_praiseworthiness_asymmetry() {
    let p = NpcBuilder::new("c", "c")
        .conscientiousness(|c| {
            c.diligence = s(0.5);
            c.perfectionism = s(0.5);
        })
        .honesty_humility(|h| h.modesty = s(0.4))
        .agreeableness(|a| a.gentleness = s(0.4))
        .build();

    let pride = p.personality().praiseworthiness_weight(true, 0.7);
    let shame = p.personality().praiseworthiness_weight(true, -0.7);
    let admiration = p.personality().praiseworthiness_weight(false, 0.7);
    let reproach = p.personality().praiseworthiness_weight(false, -0.7);
    println!("[gate3-changed] dil0.5/perf0.5/mod0.4/gen0.4 — Pride={pride} Shame={shame} Admiration={admiration} Reproach={reproach}");

    assert!((pride - 0.88).abs() < EPS, "Pride 0.88");
    assert!((shame - 1.27).abs() < EPS, "Shame 1.27");
    assert!((admiration - 1.245).abs() < EPS, "Admiration 1.245");
    assert!((reproach - 1.03).abs() < EPS, "Reproach 1.03");
}

// ---------------------------------------------------------------------------
// Gate 3 (변경 케이스) — hope-lifecycle은 X 주도. Joy/Hope/Satisfaction/
// Disappointment가 X(외향성)로 증폭됨을 박제.
// ---------------------------------------------------------------------------
#[test]
fn gate3_changed_hope_lifecycle_uses_extraversion() {
    let extro = NpcBuilder::new("x", "x")
        .extraversion(|x| {
            x.social_self_esteem = s(0.8);
            x.social_boldness = s(0.8);
            x.sociability = s(0.8);
            x.liveliness = s(0.8);
        })
        .build(); // avg.x = 0.8

    let joy = extro.personality().desirability_self_weight(1.0);
    let hope = extro.personality().desirability_prospect_weight(1.0);
    let satisfaction = extro.personality().desirability_confirmation_weight(false);
    println!("[gate3-changed] X=0.8 — Joy={joy} Hope={hope} Satisfaction={satisfaction} (전부 X 주도)");

    // Joy = base + X(0.24) = 1.24
    assert!((joy - 1.24).abs() < EPS, "Joy = base + X(0.24)");
    // Hope = Satisfaction = base + X(0.24) - Pru(0) = 1.24
    assert!((hope - 1.24).abs() < EPS, "Hope = base + X - Pru");
    assert!((satisfaction - 1.24).abs() < EPS, "Satisfaction(hope축) = base + X - Pru");
}

// ---------------------------------------------------------------------------
// Spec §3.4 계수 근거 — 겸손+근면+완벽주의(전부 0.7) 인물 Shame 포화 회피.
//   Shame = 1.0 + 0.7×0.10 + 0.7×0.20 + 0.7×0.3 = 1.0 + 0.07 + 0.14 + 0.21 = 1.42 < 1.5
// ---------------------------------------------------------------------------
#[test]
fn spec_budget_conservation_shame_below_saturation() {
    let humble_perfectionist = NpcBuilder::new("h", "h")
        .conscientiousness(|c| {
            c.diligence = s(0.7);
            c.perfectionism = s(0.7);
        })
        .honesty_humility(|h| h.modesty = s(0.7))
        .build();

    let shame = humble_perfectionist.personality().praiseworthiness_weight(true, -0.7);
    println!("[spec-budget] 겸손0.7+근면0.7+완벽0.7 — Shame={shame} (< 1.5 포화 회피)");
    assert!((shame - 1.42).abs() < EPS, "Shame 1.42 — 예산보존 검증값");
    assert!(shame < 1.5, "CLAMP_STANDARD 상한 1.5 미만 — 포화 회피");
}

// ---------------------------------------------------------------------------
// Gate 3 (불변 케이스) — fear-lifecycle은 E 주도 보존. Distress/Fear/Relief/
// FearsConfirmed 공식 byte-identical to pre-2.4.1. (보존값 박제 = deviation 0 기준.)
//   avg.e = 0.8, fearfulness = 0.8, prud = 0.
//   Distress      = base + E(0.24) − A(0) − Pru(0) = 1.24
//   Fear          = base + E(0.24) + fearfulness(0.24)  = 1.48
//   Relief/FearsConfirmed = base + E(0.24) − Pru(0)     = 1.24
// ---------------------------------------------------------------------------
#[test]
fn gate3_unchanged_fear_lifecycle_uses_emotionality() {
    let emo = NpcBuilder::new("e", "e")
        .emotionality(|e| {
            e.fearfulness = s(0.8);
            e.anxiety = s(0.8);
            e.dependence = s(0.8);
            e.sentimentality = s(0.8);
        })
        .build(); // avg.e = 0.8, fearfulness = 0.8

    let distress = emo.personality().desirability_self_weight(-1.0);
    let fear = emo.personality().desirability_prospect_weight(-1.0);
    let fear_axis_confirm = emo.personality().desirability_confirmation_weight(true);
    println!("[gate3-unchanged] E=0.8 — Distress={distress} Fear={fear} Relief/FearsConfirmed={fear_axis_confirm} (전부 불변)");

    assert!((distress - 1.24).abs() < EPS, "Distress 불변 1.24");
    assert!((fear - 1.48).abs() < EPS, "Fear 불변 1.48");
    assert!((fear_axis_confirm - 1.24).abs() < EPS, "fear축 confirmation 불변 1.24");
}
