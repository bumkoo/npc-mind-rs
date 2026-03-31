//! HEXACO 성격 모델 테스트 (-1.0 ~ 1.0 범위)
//!
//! 무협 캐릭터를 예시로 사용하여 도메인 모델 검증

use npc_mind::domain::personality::*;
use npc_mind::ports::AppraisalWeights;

// ---------------------------------------------------------------------------
// Score Value Object 테스트
// ---------------------------------------------------------------------------

#[test]
fn score_유효_범위_내_생성() {
    let s = Score::new(0.8, "test").unwrap();
    assert_eq!(s.value(), 0.8);
}

#[test]
fn score_음수_생성() {
    let s = Score::new(-0.6, "negative").unwrap();
    assert_eq!(s.value(), -0.6);
}

#[test]
fn score_경계값_허용() {
    assert!(Score::new(-1.0, "min").is_ok());
    assert!(Score::new(1.0, "max").is_ok());
    assert!(Score::new(0.0, "neutral").is_ok());
}

#[test]
fn score_범위_초과시_에러() {
    assert!(Score::new(-1.1, "too_low").is_err());
    assert!(Score::new(1.1, "too_high").is_err());
}

#[test]
fn score_강도() {
    let pos = Score::new(0.7, "p").unwrap();
    let neg = Score::new(-0.7, "n").unwrap();
    assert!((pos.intensity() - 0.7).abs() < f32::EPSILON);
    assert!((neg.intensity() - 0.7).abs() < f32::EPSILON);
}

// ---------------------------------------------------------------------------
// 중립 프로필 테스트
// ---------------------------------------------------------------------------

#[test]
fn neutral_프로필_모든_차원_0() {
    let profile = HexacoProfile::neutral();
    let avg = profile.dimension_averages();
    // DimensionAverages의 필드는 이제 Score 타입임
    assert_eq!(avg.h.value(), 0.0);
    assert_eq!(avg.e.value(), 0.0);
    assert_eq!(avg.x.value(), 0.0);
    assert_eq!(avg.a.value(), 0.0);
    assert_eq!(avg.c.value(), 0.0);
    assert_eq!(avg.o.value(), 0.0);
}

// ---------------------------------------------------------------------------
// 무협 캐릭터 빌더 테스트 (-1.0 ~ 1.0 범위)
// ---------------------------------------------------------------------------

#[test]
fn 무백_정직한_검객() {
    let s = |v: f32| Score::new(v, "").unwrap();

    let mu_baek = NpcBuilder::new("mu_baek", "무백")
        .description("무당파의 고수. 청명검의 주인.")
        .honesty_humility(|h| {
            h.sincerity = s(0.8);
            h.fairness = s(0.7);
            h.greed_avoidance = s(0.6);
            h.modesty = s(0.5);
        })
        .emotionality(|e| {
            e.fearfulness = s(-0.6);
            e.anxiety = s(-0.4);
            e.dependence = s(-0.7);
            e.sentimentality = s(0.2);
        })
        .agreeableness(|a| {
            a.forgiveness = s(0.6);
            a.gentleness = s(0.7);
            a.flexibility = s(0.2);
            a.patience = s(0.8);
        })
        .conscientiousness(|c| {
            c.organization = s(0.4);
            c.diligence = s(0.8);
            c.perfectionism = s(0.6);
            c.prudence = s(0.7);
        })
        .build();

    let avg = mu_baek.personality().dimension_averages();

    // Score 타입을 직접 f32와 비교할 수 없으므로 .value() 사용
    assert!(avg.h.value() > 0.4, "무백의 정직-겸손성은 높아야 함: {}", avg.h.value());
    assert!(avg.e.value() < -0.2, "무백의 정서성은 낮아야 함: {}", avg.e.value());
    assert!(avg.a.value() > 0.4, "무백의 원만성은 높아야 함: {}", avg.a.value());
    assert!(avg.c.value() > 0.4, "무백의 성실성은 높아야 함: {}", avg.c.value());
}

#[test]
fn 교룡_반항적_여검객() {
    let s = |v: f32| Score::new(v, "").unwrap();

    let gyo_ryong = NpcBuilder::new("gyo_ryong", "교룡")
        .description("귀족 가문의 딸이나 자유를 갈망하는 무림의 천재.")
        .honesty_humility(|h| {
            h.sincerity = s(-0.4);
            h.fairness = s(-0.5);
            h.greed_avoidance = s(-0.6);
            h.modesty = s(-0.7);
        })
        .extraversion(|x| {
            x.social_self_esteem = s(0.7);
            x.social_boldness = s(0.8);
            x.sociability = s(0.0);
            x.liveliness = s(0.6);
        })
        .agreeableness(|a| {
            a.forgiveness = s(-0.6);
            a.gentleness = s(-0.5);
            a.flexibility = s(-0.4);
            a.patience = s(-0.7);
        })
        .openness(|o| {
            o.aesthetic_appreciation = s(0.6);
            o.inquisitiveness = s(0.8);
            o.creativity = s(0.7);
            o.unconventionality = s(0.9);
        })
        .build();

    let avg = gyo_ryong.personality().dimension_averages();

    // Score 타입을 직접 f32와 비교할 수 없으므로 .value() 사용
    assert!(avg.h.value() < -0.4, "교룡의 정직-겸손성은 낮아야 함: {}", avg.h.value());
    assert!(avg.x.value() > 0.4, "교룡의 외향성은 높아야 함: {}", avg.x.value());
    assert!(avg.a.value() < -0.4, "교룡의 원만성은 낮아야 함: {}", avg.a.value());
    assert!(avg.o.value() > 0.7, "교룡의 개방성은 높아야 함: {}", avg.o.value());
}

// ---------------------------------------------------------------------------
// 핵심: 같은 상황 → 성격에 따라 다른 해석
// ---------------------------------------------------------------------------

#[test]
fn 같은_상황_다른_성격_다른_해석_가능성() {
    let s = |v: f32| Score::new(v, "").unwrap();

    let li = NpcBuilder::new("li", "무백")
        .agreeableness(|a| {
            a.forgiveness = s(0.6);
            a.patience = s(0.8);
            a.gentleness = s(0.7);
            a.flexibility = s(0.2);
        })
        .honesty_humility(|h| {
            h.sincerity = s(0.8);
            h.fairness = s(0.7);
            h.greed_avoidance = s(0.6);
            h.modesty = s(0.5);
        })
        .build();

    let yu = NpcBuilder::new("yu", "교룡")
        .agreeableness(|a| {
            a.forgiveness = s(-0.6);
            a.patience = s(-0.7);
            a.gentleness = s(-0.5);
            a.flexibility = s(-0.4);
        })
        .honesty_humility(|h| {
            h.sincerity = s(-0.4);
            h.fairness = s(-0.5);
            h.greed_avoidance = s(-0.6);
            h.modesty = s(-0.7);
        })
        .build();

    let li_avg = li.personality().dimension_averages();
    let yu_avg = yu.personality().dimension_averages();

    // Score 타입을 직접 비교할 수 없으므로 .value() 사용
    assert!(li_avg.a.value() > 0.0 && yu_avg.a.value() < 0.0,
        "무백(A={})은 양수, 교룡(A={})은 음수여야 함", li_avg.a.value(), yu_avg.a.value());
    assert!(li_avg.h.value() > 0.0 && yu_avg.h.value() < 0.0,
        "무백(H={})은 양수, 교룡(H={})은 음수여야 함", li_avg.h.value(), yu_avg.h.value());

    let a_gap = li_avg.a.value() - yu_avg.a.value();
    assert!(a_gap > 1.0,
        "원만성 차이({})가 1.0 이상이어야 감정 분기가 극적임", a_gap);
}

// ---------------------------------------------------------------------------
// Score::modifier() 선형 패턴: 방향에 따라 증폭/억제
// ---------------------------------------------------------------------------

#[test]
fn 성격_modifier_선형_패턴() {
    // 까칠한 사람(patience=-0.7)은 부정 감정을 증폭
    let impatient = Score::new(-0.7, "patience").unwrap();
    let mod_impatient = impatient.modifier(-0.3); // 1.0 - (-0.7)*0.3 = 1.21
    assert!(mod_impatient > 1.0,
        "까칠함(-0.7)은 부정 감정 증폭: {}", mod_impatient);

    // 관용적인 사람(patience=0.7)은 부정 감정을 억제
    let patient = Score::new(0.7, "patience").unwrap();
    let mod_patient = patient.modifier(-0.3); // 1.0 - 0.7*0.3 = 0.79
    assert!(mod_patient < 1.0,
        "관용(0.7)은 부정 감정 억제: {}", mod_patient);

    // 중립(0.0)은 변동 없음
    let neutral = Score::neutral();
    let mod_neutral = neutral.modifier(-0.3); // 1.0
    assert!((mod_neutral - 1.0).abs() < f32::EPSILON,
        "중립은 변동 없음: {}", mod_neutral);
}

// ---------------------------------------------------------------------------
// JSON 직렬화 테스트
// ---------------------------------------------------------------------------

#[test]
fn npc_json_직렬화_역직렬화() {
    let s = |v: f32| Score::new(v, "").unwrap();

    let npc = NpcBuilder::new("test_npc", "테스트")
        .description("테스트용 NPC")
        .honesty_humility(|h| { h.sincerity = s(0.8); })
        .agreeableness(|a| { a.forgiveness = s(-0.5); })
        .build();

    let json = serde_json::to_string_pretty(&npc).unwrap();
    let restored: Npc = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.id(), "test_npc");
    assert_eq!(restored.name(), "테스트");
    assert_eq!(restored.personality().honesty_humility.sincerity.value(), 0.8);
    assert_eq!(restored.personality().agreeableness.forgiveness.value(), -0.5);
    assert_eq!(restored.personality().extraversion.sociability.value(), 0.0);
}

// ---------------------------------------------------------------------------
// 수련 — 의리와 절제의 여검객
// ---------------------------------------------------------------------------

#[test]
fn 수련_절제의_여검객() {
    let s = |v: f32| Score::new(v, "").unwrap();

    let shu_lien = NpcBuilder::new("shu_lien", "수련")
        .description("강호의 여협. 무백과 서로 사랑하나 의리로 감정을 억누른다.")
        .honesty_humility(|h| {
            h.sincerity = s(0.8);
            h.fairness = s(0.9);
            h.greed_avoidance = s(0.7);
            h.modesty = s(0.6);
        })
        .emotionality(|e| {
            e.fearfulness = s(-0.3);
            e.anxiety = s(0.2);
            e.dependence = s(-0.5);
            e.sentimentality = s(0.7);
        })
        .extraversion(|x| {
            x.social_self_esteem = s(0.4);
            x.social_boldness = s(0.3);
            x.sociability = s(-0.2);
            x.liveliness = s(-0.3);
        })
        .agreeableness(|a| {
            a.forgiveness = s(0.5);
            a.gentleness = s(0.6);
            a.flexibility = s(0.3);
            a.patience = s(0.9);
        })
        .conscientiousness(|c| {
            c.organization = s(0.6);
            c.diligence = s(0.8);
            c.perfectionism = s(0.5);
            c.prudence = s(0.9);
        })
        .openness(|o| {
            o.aesthetic_appreciation = s(0.3);
            o.inquisitiveness = s(-0.1);
            o.creativity = s(0.0);
            o.unconventionality = s(-0.6);
        })
        .build();

    let avg = shu_lien.personality().dimension_averages();

    // Score 연산 시 .value() 또는 .intensity() 사용
    assert!(avg.h.value() > 0.6, "수련의 정직-겸손성은 매우 높아야 함: {}", avg.h.value());
    assert!(avg.e.intensity() < 0.3, "수련의 정서성은 복합적(중립 근처)이어야 함: {}", avg.e.value());
    assert!(avg.a.value() > 0.4, "수련의 원만성은 높아야 함: {}", avg.a.value());
    assert!(avg.c.value() > 0.6, "수련의 성실성은 매우 높아야 함: {}", avg.c.value());
    assert!(avg.o.value() < 0.0, "수련의 개방성은 낮아야 함(전통적): {}", avg.o.value());
}

// ---------------------------------------------------------------------------
// 소호 — 자유로운 영혼의 강호 낭인
// ---------------------------------------------------------------------------

#[test]
fn 소호_자유로운_낭인() {
    let s = |v: f32| Score::new(v, "").unwrap();

    let so_ho = NpcBuilder::new("so_ho", "소호")
        .description("강호를 떠도는 낭인. 냉소적 겉모습 뒤에 뜨거운 의리를 숨긴다.")
        .honesty_humility(|h| {
            h.sincerity = s(0.1);
            h.fairness = s(0.5);
            h.greed_avoidance = s(0.3);
            h.modesty = s(-0.3);
        })
        .emotionality(|e| {
            e.fearfulness = s(-0.7);
            e.anxiety = s(-0.5);
            e.dependence = s(-0.8);
            e.sentimentality = s(0.4);
        })
        .extraversion(|x| {
            x.social_self_esteem = s(0.6);
            x.social_boldness = s(0.7);
            x.sociability = s(0.5);
            x.liveliness = s(0.4);
        })
        .agreeableness(|a| {
            a.forgiveness = s(0.1);
            a.gentleness = s(-0.4);
            a.flexibility = s(0.3);
            a.patience = s(-0.3);
        })
        .conscientiousness(|c| {
            c.organization = s(-0.6);
            c.diligence = s(0.2);
            c.perfectionism = s(-0.4);
            c.prudence = s(-0.5);
        })
        .openness(|o| {
            o.aesthetic_appreciation = s(0.4);
            o.inquisitiveness = s(0.7);
            o.creativity = s(0.5);
            o.unconventionality = s(0.8);
        })
        .build();

    let avg = so_ho.personality().dimension_averages();

    // Score 비교를 위해 .value() 사용
    assert!(avg.e.value() < -0.3, "소호의 정서성은 낮아야 함(대담): {}", avg.e.value());
    assert!(avg.x.value() > 0.4, "소호의 외향성은 높아야 함: {}", avg.x.value());
    assert!(avg.c.value() < -0.2, "소호의 성실성은 낮아야 함(자유분방): {}", avg.c.value());
    assert!(avg.o.value() > 0.5, "소호의 개방성은 높아야 함: {}", avg.o.value());
}

// ---------------------------------------------------------------------------
// 4인 캐릭터 성격 대비
// ---------------------------------------------------------------------------

#[test]
fn 사인_성격_대비() {
    let s = |v: f32| Score::new(v, "").unwrap();

    let li = NpcBuilder::new("li", "무백")
        .agreeableness(|a| { a.patience = s(0.8); a.forgiveness = s(0.6); a.gentleness = s(0.7); a.flexibility = s(0.2); })
        .emotionality(|e| { e.fearfulness = s(-0.6); e.anxiety = s(-0.4); e.dependence = s(-0.7); e.sentimentality = s(0.2); })
        .build();

    let shu = NpcBuilder::new("shu", "수련")
        .conscientiousness(|c| { c.prudence = s(0.9); c.diligence = s(0.8); c.organization = s(0.6); c.perfectionism = s(0.5); })
        .emotionality(|e| { e.sentimentality = s(0.7); e.fearfulness = s(-0.3); e.anxiety = s(0.2); e.dependence = s(-0.5); })
        .build();

    let yu = NpcBuilder::new("yu", "교룡")
        .agreeableness(|a| { a.patience = s(-0.7); a.forgiveness = s(-0.6); a.gentleness = s(-0.5); a.flexibility = s(-0.4); })
        .extraversion(|x| { x.social_boldness = s(0.8); x.social_self_esteem = s(0.7); x.sociability = s(0.0); x.liveliness = s(0.6); })
        .build();

    let na = NpcBuilder::new("na", "소호")
        .emotionality(|e| { e.fearfulness = s(-0.7); e.anxiety = s(-0.5); e.dependence = s(-0.8); e.sentimentality = s(0.4); })
        .conscientiousness(|c| { c.organization = s(-0.6); c.prudence = s(-0.5); c.diligence = s(0.2); c.perfectionism = s(-0.4); })
        .build();

    let li_avg = li.personality().dimension_averages();
    let shu_avg = shu.personality().dimension_averages();
    let yu_avg = yu.personality().dimension_averages();
    let na_avg = na.personality().dimension_averages();

    // Score 비교를 위해 .value() 사용
    assert!(li_avg.a.value() > 0.4 && li_avg.e.value() < -0.2);
    assert!(shu_avg.c.value() > 0.6);
    assert!(yu_avg.a.value() < -0.4 && yu_avg.x.value() > 0.4);
    assert!(na_avg.e.value() < -0.3 && na_avg.c.value() < -0.2);
}

// ---------------------------------------------------------------------------
// 리팩토링 무결성 검증: 수식 결과값 정밀 테스트
// ---------------------------------------------------------------------------

#[test]
fn test_score_effect_calculation() {
    let s = Score::new(0.5, "test").unwrap();
    let weight = 0.3;
    
    // 리팩토링된 effect() 메서드 결과가 (값 * 가중치)와 동일한지 확인
    assert_eq!(s.effect(weight), 0.5 * 0.3);
    
    let neg_s = Score::new(-0.8, "test").unwrap();
    assert_eq!(neg_s.effect(weight), -0.8 * 0.3);
}

#[test]
fn test_appraisal_weights_consistency() {
    // 1. 극단적으로 예민한 성격 (E=1.0)
    let emotional = NpcBuilder::new("e", "e")
        .emotionality(|e| {
            e.fearfulness = Score::new(1.0, "").unwrap();
            e.anxiety = Score::new(1.0, "").unwrap();
            e.dependence = Score::new(1.0, "").unwrap();
            e.sentimentality = Score::new(1.0, "").unwrap();
        })
        .build();
    
    // desirability_self_weight (Joy/Distress)
    // d > 0: BASE_SELF(1.0) + E_effect(1.0*0.3) + X_effect(0.0*0.3) = 1.3
    let w = emotional.personality().desirability_self_weight(1.0);
    assert!((w - 1.3).abs() < f32::EPSILON, "E=1.0 -> weight=1.3 (Joy)");
    
    // 2. 극단적으로 무던한 성격 (E=-1.0)
    let cold = NpcBuilder::new("c", "c")
        .emotionality(|e| {
            e.fearfulness = Score::new(-1.0, "").unwrap();
            e.anxiety = Score::new(-1.0, "").unwrap();
            e.dependence = Score::new(-1.0, "").unwrap();
            e.sentimentality = Score::new(-1.0, "").unwrap();
        })
        .build();
    
    // d > 0: BASE_SELF(1.0) + E_effect(-1.0*0.3) + X_effect(0.0*0.3) = 0.7
    let w2 = cold.personality().desirability_self_weight(1.0);
    assert!((w2 - 0.7).abs() < f32::EPSILON, "E=-1.0 -> weight=0.7 (Joy)");
}

#[test]
fn test_empathy_and_hostility_weights() {
    // 정직하고 원만한 성격 (H=1.0, A=1.0)
    let saint = NpcBuilder::new("s", "s")
        .honesty_humility(|h| {
            h.sincerity = Score::new(1.0, "").unwrap();
            h.fairness = Score::new(1.0, "").unwrap();
            h.greed_avoidance = Score::new(1.0, "").unwrap();
            h.modesty = Score::new(1.0, "").unwrap();
        })
        .agreeableness(|a| {
            a.forgiveness = Score::new(1.0, "").unwrap();
            a.gentleness = Score::new(1.0, "").unwrap();
            a.flexibility = Score::new(1.0, "").unwrap();
            a.patience = Score::new(1.0, "").unwrap();
        })
        .build();

    // d > 0 (타인에게 좋은 일): BASE_EMPATHY(0.5) + H_effect(1.0*0.4) + A_effect(1.0*0.4) = 1.3
    let emp = saint.personality().empathy_weight(1.0);
    assert!((emp - 1.3).abs() < f32::EPSILON, "H=1, A=1 -> empathy=1.3");

    // d > 0 (타인에게 좋은 일): BASE_HOSTILITY(0.0) - H_effect(1.0*0.7) = -0.7 -> clamp(0.0)
    let host = saint.personality().hostility_weight(1.0);
    assert_eq!(host, 0.0, "정직하면 적대감(Resentment) 0");
}
