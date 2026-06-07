//! Phase 2.4.3 — RelationshipModifiers 통합 재설계 검증 게이트.
//!
//! FROZEN spec: `docs/tasks/mind-architecture/task-rel-phase2.4.3-relationship-modifiers.md`.
//!
//! `Relationship::modifiers()`가 `magnitude`(trust 볼륨) + 두 렌즈(`tilt_warm`/`tilt_cold`)로
//! 통합되며, 소비처가 가지별로 렌즈를 선택한다:
//!   - action.rs (타인 행동): Admiration(pw≥0)=`tilt_warm`, Reproach(pw<0)=`tilt_cold`, 둘 다 `magnitude` 공통.
//!   - event.rs (타인의 운):  HappyFor/Pity=`tilt_warm`, Resentment/Gloating=`tilt_cold` (magnitude 미적용, B-D2).
//!
//! 본 파일은 spec §6.3(공감/적대 가지 새 modifier 반영 — appraise() 직접 측정) +
//! §6.4(gentleness 합산 과억제) 게이트를 박제한다. narrative S1~S4의 *4축* 박제
//! (`phase2_narrative_test.rs`)는 고정 emotion_state를 주입하므로 modifiers()를 경유하지
//! 않는다 → 본 재설계의 영향 0 (그 테스트는 무회귀). 모디파이어 신호는 *여기서* 격리 측정.

use npc_mind::domain::emotion::{
    ActionFocus, AppraisalEngine, DesirabilityForOther, EmotionState, EmotionType, EventFocus,
    RelationshipModifiers, Situation,
};
use npc_mind::domain::personality::{Npc, NpcBuilder, Score};
use npc_mind::domain::relationship::{AxisScore, Relationship, RelationshipBuilder, WarinessScore};
use npc_mind::domain::tuning::profile;
use npc_mind::ports::Appraiser;

fn sc(v: f32) -> Score {
    Score::new(v, "t").unwrap()
}

// ---------------------------------------------------------------------------
// 관계 픽스처 — 4축으로 magnitude/tilt 신호 격리.
// ---------------------------------------------------------------------------

fn rel_neutral() -> Relationship {
    Relationship::neutral("npc", "partner")
}

/// 친밀+신뢰 관계 — affinity·respect·trust↑, wariness 0. tilt_warm↑ / tilt_cold↓ / magnitude↑.
fn rel_close() -> Relationship {
    RelationshipBuilder::new("npc", "partner")
        .trust(AxisScore::new(70.0))
        .affinity(AxisScore::new(90.0))
        .respect(AxisScore::new(70.0))
        .wariness(WarinessScore::new(0.0))
        .build()
}

/// 적대 관계 — affinity·respect·trust↓, wariness↑. tilt_warm↓(FLOOR) / tilt_cold↑(CEIL) / magnitude↓.
fn rel_hostile() -> Relationship {
    RelationshipBuilder::new("npc", "partner")
        .trust(AxisScore::new(-70.0))
        .affinity(AxisScore::new(-90.0))
        .respect(AxisScore::new(-70.0))
        .wariness(WarinessScore::new(80.0))
        .build()
}

/// 최대 따뜻함이되 trust 0 — tilt_cold = FLOOR, magnitude = 1.0. 렌즈 FLOOR 가드레일 격리용.
fn rel_max_warm_no_trust() -> Relationship {
    RelationshipBuilder::new("npc", "partner")
        .trust(AxisScore::new(0.0))
        .affinity(AxisScore::new(100.0))
        .respect(AxisScore::new(100.0))
        .wariness(WarinessScore::new(0.0))
        .build()
}

// ---------------------------------------------------------------------------
// appraise 헬퍼.
// ---------------------------------------------------------------------------

/// 타인 행동(action 가지) → Admiration(pw≥0)/Reproach(pw<0). dialogue_modifiers 경로.
fn appraise_action(npc: &Npc, pw: f32, mods: &RelationshipModifiers) -> EmotionState {
    let situation = Situation::new(
        "타인의 행동",
        None,
        Some(ActionFocus {
            description: String::new(),
            agent_id: Some("partner".into()),
            praiseworthiness: pw,
            modifiers: None, // dialogue_modifiers 사용
        }),
        None,
    )
    .unwrap();
    AppraisalEngine.appraise(npc.personality(), &situation, mods)
}

/// 타인의 운(event 가지) → HappyFor/Resentment(d>0) · Pity/Gloating(d<0). modifier = DesirabilityForOther.
fn appraise_other_fortune(npc: &Npc, d_other: f32, other_mods: RelationshipModifiers) -> EmotionState {
    let situation = Situation::new(
        "타인의 운",
        Some(EventFocus {
            description: String::new(),
            desirability_for_self: 0.0, // 자기 Joy/Distress 비활성 → 공감/적대만 격리
            desirability_for_other: Some(DesirabilityForOther {
                target_id: "other".into(),
                desirability: d_other,
                modifiers: other_mods,
            }),
            prospect: None,
        }),
        None,
        None,
    )
    .unwrap();
    AppraisalEngine.appraise(npc.personality(), &situation, &RelationshipModifiers::neutral())
}

// ---------------------------------------------------------------------------
// 1. modifiers() 박제 — 4축 → magnitude/tilt_warm/tilt_cold.
// ---------------------------------------------------------------------------

#[test]
fn modifiers_pins_for_fixtures() {
    let p = profile();
    let (floor, ceil) = (p.rel_mod_floor, p.rel_mod_ceil);

    let neutral = rel_neutral().modifiers();
    assert!((neutral.magnitude - 1.0).abs() < 1e-6);
    assert!((neutral.tilt_warm - 1.0).abs() < 1e-6);
    assert!((neutral.tilt_cold - 1.0).abs() < 1e-6);

    // close: magnitude 1+70×0.003=1.21 / lens 90×0.003+70×0.002=0.41 → warm 1.41, cold 0.59.
    let close = rel_close().modifiers();
    assert!((close.magnitude - 1.21).abs() < 1e-4, "magnitude {}", close.magnitude);
    assert!((close.tilt_warm - 1.41).abs() < 1e-4, "tilt_warm {}", close.tilt_warm);
    assert!((close.tilt_cold - 0.59).abs() < 1e-4, "tilt_cold {}", close.tilt_cold);

    // hostile: magnitude 1−0.21=0.79 / lens −0.27−0.14−0.24=−0.65 → warm 0.35→FLOOR, cold 1.65→CEIL.
    let hostile = rel_hostile().modifiers();
    assert!((hostile.magnitude - 0.79).abs() < 1e-4, "magnitude {}", hostile.magnitude);
    assert!((hostile.tilt_warm - floor).abs() < 1e-6, "tilt_warm floored {}", hostile.tilt_warm);
    assert!((hostile.tilt_cold - ceil).abs() < 1e-6, "tilt_cold ceiled {}", hostile.tilt_cold);

    // max-warm-no-trust: tilt_cold = FLOOR, magnitude = 1.0 (trust 0). 렌즈 FLOOR 가드레일.
    let mw = rel_max_warm_no_trust().modifiers();
    assert!((mw.magnitude - 1.0).abs() < 1e-6);
    assert!((mw.tilt_warm - ceil).abs() < 1e-6, "tilt_warm ceiled {}", mw.tilt_warm);
    assert!((mw.tilt_cold - floor).abs() < 1e-6, "tilt_cold floored {}", mw.tilt_cold);

    println!(
        "[modifiers] neutral={:?} close={:?} hostile={:?} max_warm={:?}",
        neutral, close, hostile, mw
    );
}

// ---------------------------------------------------------------------------
// 2. action 가지 — Admiration(warm) / Reproach(cold) 렌즈 방향.
// ---------------------------------------------------------------------------

#[test]
fn admiration_amplified_by_warm_suppressed_by_cold() {
    // 중립 성격(facet 0) → praiseworthiness_weight = base 1.0. 모디파이어 신호만 격리.
    // pw=0.5 → close 증폭(×1.706) 후에도 강도 < 1.0 (Emotion intensity [0,1] clamp 회피).
    let npc = NpcBuilder::new("n", "n").build();
    let pw = 0.5;

    let adm_neutral = appraise_action(&npc, pw, &rel_neutral().modifiers()).intensity_of(EmotionType::Admiration);
    let adm_close = appraise_action(&npc, pw, &rel_close().modifiers()).intensity_of(EmotionType::Admiration);
    let adm_hostile = appraise_action(&npc, pw, &rel_hostile().modifiers()).intensity_of(EmotionType::Admiration);

    println!("[Admiration] neutral={adm_neutral} close={adm_close} hostile={adm_hostile}");
    assert!(adm_close > adm_neutral, "친밀 관계 → 따뜻함 렌즈+볼륨이 Admiration 증폭");
    assert!(adm_hostile < adm_neutral, "적대 관계 → Admiration 억제");

    // 정량: Admiration modifier = magnitude × tilt_warm. close/neutral 비율 박제.
    let m = rel_close().modifiers();
    let ratio = adm_close / adm_neutral;
    assert!((ratio - m.magnitude * m.tilt_warm).abs() < 1e-3, "ratio {ratio}");
}

#[test]
fn reproach_suppressed_by_warm_amplified_by_cold() {
    // pw=-0.5 → hostile 증폭(×1.185) 후에도 강도 < 1.0.
    let npc = NpcBuilder::new("n", "n").build();
    let pw = -0.5;

    let rep_neutral = appraise_action(&npc, pw, &rel_neutral().modifiers()).intensity_of(EmotionType::Reproach);
    let rep_close = appraise_action(&npc, pw, &rel_close().modifiers()).intensity_of(EmotionType::Reproach);
    let rep_hostile = appraise_action(&npc, pw, &rel_hostile().modifiers()).intensity_of(EmotionType::Reproach);

    println!("[Reproach] neutral={rep_neutral} close={rep_close} hostile={rep_hostile}");
    assert!(rep_close < rep_neutral, "친밀 관계 → 차가움 렌즈가 Reproach 억제(봐줌)");
    assert!(rep_hostile > rep_neutral, "적대 관계 → Reproach 증폭(배신감)");

    // 정량: Reproach modifier = magnitude × tilt_cold.
    let m = rel_close().modifiers();
    let ratio = rep_close / rep_neutral;
    assert!((ratio - m.magnitude * m.tilt_cold).abs() < 1e-3, "ratio {ratio}");
}

// ---------------------------------------------------------------------------
// 3. event 가지 — 공감(HappyFor, warm) / 적대(Resentment, cold). magnitude 미적용(B-D2).
// ---------------------------------------------------------------------------

#[test]
fn empathy_warm_hostility_cold_event_branch() {
    // 적대 가지(Resentment/Gloating)는 BASE_HOSTILITY=0이라 *낮은 정직* 성격에서만 발동.
    // H=-0.5 → hostility_weight(0.3)=0.35>0, empathy_weight(0.3)=0.5−0.2=0.3>0 (둘 다 양수).
    let npc = NpcBuilder::new("e", "e")
        .honesty_humility(|h| {
            h.sincerity = sc(-0.5);
            h.fairness = sc(-0.5);
            h.greed_avoidance = sc(-0.5);
            h.modesty = sc(-0.5);
        })
        .build();
    let d_other = 0.3; // 타인에게 좋은 일 → HappyFor(공감) + Resentment(적대)

    let happy_neutral = appraise_other_fortune(&npc, d_other, rel_neutral().modifiers()).intensity_of(EmotionType::HappyFor);
    let happy_close = appraise_other_fortune(&npc, d_other, rel_close().modifiers()).intensity_of(EmotionType::HappyFor);
    let resent_neutral = appraise_other_fortune(&npc, d_other, rel_neutral().modifiers()).intensity_of(EmotionType::Resentment);
    let resent_close = appraise_other_fortune(&npc, d_other, rel_close().modifiers()).intensity_of(EmotionType::Resentment);

    println!(
        "[empathy/hostility] HappyFor neutral={happy_neutral} close={happy_close} | Resentment neutral={resent_neutral} close={resent_close}"
    );
    assert!(happy_close > happy_neutral, "친밀 관계 → 따뜻함 렌즈가 HappyFor 증폭");
    assert!(resent_close < resent_neutral, "친밀 관계 → 차가움 렌즈가 Resentment 억제");

    // event 가지는 magnitude 미적용 → 비율은 tilt_warm/tilt_cold 그대로 (B-D2).
    let m = rel_close().modifiers();
    assert!(((happy_close / happy_neutral) - m.tilt_warm).abs() < 1e-3);
    assert!(((resent_close / resent_neutral) - m.tilt_cold).abs() < 1e-3);
}

// ---------------------------------------------------------------------------
// 4. gentleness 합산 과억제 (§6.4, B-D6) — 온화 NPC × 친밀 상대 Reproach.
//    온화 성격(Reproach weight↓) + 친밀 관계(tilt_cold→FLOOR)가 *합산*되어도
//    FLOOR가 관계 억제를 50%로 제한 → Reproach가 0으로 붕괴하지 않음.
// ---------------------------------------------------------------------------

#[test]
fn gentleness_plus_close_partner_reproach_is_suppressed_but_floored() {
    let p = profile();
    let floor = p.rel_mod_floor;
    let pw = -0.8;

    // 온화 NPC: gentleness만 높임 (다른 facet 0 → praiseworthiness_weight 신호 격리).
    let gentle = NpcBuilder::new("g", "g")
        .agreeableness(|a| a.gentleness = sc(0.9))
        .build();
    // 비온화 기준선: 전 facet 0 → Reproach weight = base 1.0.
    let plain = NpcBuilder::new("p", "p").build();

    let mw = rel_max_warm_no_trust().modifiers();
    assert!((mw.tilt_cold - floor).abs() < 1e-6, "관계 floor 가드레일 전제");
    assert!((mw.magnitude - 1.0).abs() < 1e-6);

    let rep_plain_neutral = appraise_action(&plain, pw, &rel_neutral().modifiers()).intensity_of(EmotionType::Reproach);
    let rep_gentle_neutral = appraise_action(&gentle, pw, &rel_neutral().modifiers()).intensity_of(EmotionType::Reproach);
    let rep_gentle_close = appraise_action(&gentle, pw, &mw).intensity_of(EmotionType::Reproach);

    println!(
        "[gentleness] plain×neutral={rep_plain_neutral} gentle×neutral={rep_gentle_neutral} gentle×close={rep_gentle_close} (floor={floor})"
    );

    // (1) 성격 억제: 온화 NPC는 동일 상황에서 Reproach가 더 약하다.
    assert!(rep_gentle_neutral < rep_plain_neutral, "gentleness가 Reproach weight를 낮춤");

    // (2) 합산 억제: 온화 성격 위에 친밀 관계가 추가로 억제 → close < neutral < plain.
    assert!(rep_gentle_close < rep_gentle_neutral, "친밀 관계 차가움 렌즈가 추가 억제");
    assert!(rep_gentle_close < rep_plain_neutral, "합산 억제 < 비온화 기준선");

    // (3) FLOOR 가드레일: 관계 억제는 정확히 50%까지만 (magnitude 1.0 × tilt_cold floor).
    //     → 합산 과억제에도 Reproach가 0으로 붕괴하지 않음.
    let expected = rep_gentle_neutral * floor;
    assert!(
        (rep_gentle_close - expected).abs() < 1e-3,
        "관계 억제는 FLOOR(={floor})에 묶임: got {rep_gentle_close}, expected {expected}"
    );
    assert!(rep_gentle_close > 0.0, "Reproach 잔존 — 과억제로 소멸하지 않음");
}
