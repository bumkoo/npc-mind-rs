//! HEXACO 성격 모델 테스트 (-1.0 ~ 1.0 범위)
//!
//! 무협 캐릭터를 예시로 사용하여 도메인 모델 검증

use npc_mind::domain::personality::*;

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
fn score_높낮이_판별() {
    let high = Score::new(0.7, "h").unwrap();
    let low = Score::new(-0.6, "l").unwrap();
    let mid = Score::new(0.0, "m").unwrap();

    assert!(high.is_high());     // 0.4 이상 → 높음
    assert!(!high.is_low());
    assert!(low.is_low());       // -0.4 이하 → 낮음
    assert!(!low.is_high());
    assert!(!mid.is_high());
    assert!(!mid.is_low());      // 0.0 → 중립
}

#[test]
fn score_방향_판별() {
    let pos = Score::new(0.3, "p").unwrap();
    let neg = Score::new(-0.3, "n").unwrap();
    let zero = Score::new(0.0, "z").unwrap();

    assert!(pos.is_positive());
    assert!(!pos.is_negative());
    assert!(neg.is_negative());
    assert!(!neg.is_positive());
    assert!(!zero.is_positive());
    assert!(!zero.is_negative());
}

#[test]
fn score_강도() {
    let pos = Score::new(0.7, "p").unwrap();
    let neg = Score::new(-0.7, "n").unwrap();
    assert!((pos.intensity() - 0.7).abs() < f32::EPSILON);
    assert!((neg.intensity() - 0.7).abs() < f32::EPSILON);
}

#[test]
fn score_증폭() {
    let s = Score::new(0.5, "s").unwrap();
    // 0.5 × 1.5 = 0.75
    assert!((s.amplify(1.5) - 0.75).abs() < f32::EPSILON);
    // 클램핑: 0.5 × 3.0 = 1.5 → 1.0으로 제한
    assert!((s.amplify(3.0) - 1.0).abs() < f32::EPSILON);
}

#[test]
fn score_거리_계산() {
    let a = Score::new(-0.6, "a").unwrap();
    let b = Score::new(0.6, "b").unwrap();
    let dist = a.distance(&b);
    assert!((dist - 1.2).abs() < f32::EPSILON);
}

// ---------------------------------------------------------------------------
// 중립 프로필 테스트
// ---------------------------------------------------------------------------

#[test]
fn neutral_프로필_모든_차원_0() {
    let profile = HexacoProfile::neutral();
    let avg = profile.dimension_averages();
    assert_eq!(avg.h, 0.0);
    assert_eq!(avg.e, 0.0);
    assert_eq!(avg.x, 0.0);
    assert_eq!(avg.a, 0.0);
    assert_eq!(avg.c, 0.0);
    assert_eq!(avg.o, 0.0);
}

// ---------------------------------------------------------------------------
// 무협 캐릭터 빌더 테스트 (-1.0 ~ 1.0 범위)
// ---------------------------------------------------------------------------

/// 무백 — 정직하고 절제된 검객
/// H↑, E↓, A↑, C↑
#[test]
fn 무백_정직한_검객() {
    let s = |v: f32| Score::new(v, "").unwrap();

    let mu_baek = NpcBuilder::new("mu_baek", "무백")
        .description("무당파의 고수. 청명검의 주인.")
        .honesty_humility(|h| {
            h.sincerity = s(0.8);       // 매우 진실됨
            h.fairness = s(0.7);        // 공정함
            h.greed_avoidance = s(0.6); // 탐욕 없음
            h.modesty = s(0.5);         // 겸손
        })
        .emotionality(|e| {
            e.fearfulness = s(-0.6);    // 대담함
            e.anxiety = s(-0.4);        // 불안 없음
            e.dependence = s(-0.7);     // 독립적
            e.sentimentality = s(0.2);  // 약간 감상적
        })
        .agreeableness(|a| {
            a.forgiveness = s(0.6);     // 관용적
            a.gentleness = s(0.7);      // 온화함
            a.flexibility = s(0.2);     // 약간 유연
            a.patience = s(0.8);        // 매우 인내심
        })
        .conscientiousness(|c| {
            c.organization = s(0.4);    // 체계적
            c.diligence = s(0.8);       // 매우 근면
            c.perfectionism = s(0.6);   // 완벽 추구
            c.prudence = s(0.7);        // 신중함
        })
        .build();

    let avg = mu_baek.personality.dimension_averages();

    // H: 정직-겸손성이 높아야 함 (양수 영역)
    assert!(avg.h > 0.4, "무백의 정직-겸손성은 높아야 함: {}", avg.h);
    // E: 정서성이 낮아야 함 (음수 영역, 대담한 검객)
    assert!(avg.e < -0.2, "무백의 정서성은 낮아야 함: {}", avg.e);
    // A: 원만성이 높아야 함
    assert!(avg.a > 0.4, "무백의 원만성은 높아야 함: {}", avg.a);
    // C: 성실성이 높아야 함
    assert!(avg.c > 0.4, "무백의 성실성은 높아야 함: {}", avg.c);
}

/// 교룡 — 야심 있고 반항적인 귀족 여검객
/// H↓, X↑, A↓, O↑
#[test]
fn 교룡_반항적_여검객() {
    let s = |v: f32| Score::new(v, "").unwrap();

    let gyo_ryong = NpcBuilder::new("gyo_ryong", "교룡")
        .description("귀족 가문의 딸이나 자유를 갈망하는 무림의 천재.")
        .honesty_humility(|h| {
            h.sincerity = s(-0.4);      // 기만적
            h.fairness = s(-0.5);       // 불공정
            h.greed_avoidance = s(-0.6);// 탐욕적
            h.modesty = s(-0.7);        // 자기과시적
        })
        .extraversion(|x| {
            x.social_self_esteem = s(0.7);  // 높은 자존감
            x.social_boldness = s(0.8);     // 매우 대담
            x.sociability = s(0.0);         // 중립 (혼자도 괜찮)
            x.liveliness = s(0.6);          // 활기참
        })
        .agreeableness(|a| {
            a.forgiveness = s(-0.6);    // 원한을 품음
            a.gentleness = s(-0.5);     // 거칠음
            a.flexibility = s(-0.4);    // 완고함
            a.patience = s(-0.7);       // 참을성 없음
        })
        .openness(|o| {
            o.aesthetic_appreciation = s(0.6);  // 미적 감각
            o.inquisitiveness = s(0.8);         // 탐구심 강함
            o.creativity = s(0.7);              // 창의적
            o.unconventionality = s(0.9);       // 매우 비관습적
        })
        .build();

    let avg = gyo_ryong.personality.dimension_averages();

    // H: 정직-겸손성 낮음 (음수 영역)
    assert!(avg.h < -0.4, "교룡의 정직-겸손성은 낮아야 함: {}", avg.h);
    // X: 외향성 높음 (양수 영역)
    assert!(avg.x > 0.4, "교룡의 외향성은 높아야 함: {}", avg.x);
    // A: 원만성 낮음 (음수 영역)
    assert!(avg.a < -0.4, "교룡의 원만성은 낮아야 함: {}", avg.a);
    // O: 개방성 높음 (양수 영역)
    assert!(avg.o > 0.7, "교룡의 개방성은 높아야 함: {}", avg.o);
}

// ---------------------------------------------------------------------------
// 핵심: 같은 상황 → 성격에 따라 다른 해석 (2사이클 OCC 연결점)
// ---------------------------------------------------------------------------

/// "배신 상황"에서 성격 차이가 감정 해석 차이를 만든다
/// 무백: 관용적(A+) + 정직(H+) → "실망(disappointment)"
/// 교룡: 원한(A-) + 교활(H-) → "분노(anger) + 복수심"
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

    let li_avg = li.personality.dimension_averages();
    let yu_avg = yu.personality.dimension_averages();

    // 무백은 관용적(양수), 교룡은 원한(음수) — 부호가 반대
    assert!(li_avg.a > 0.0 && yu_avg.a < 0.0,
        "무백(A={})은 양수, 교룡(A={})은 음수여야 함", li_avg.a, yu_avg.a);
    assert!(li_avg.h > 0.0 && yu_avg.h < 0.0,
        "무백(H={})은 양수, 교룡(H={})은 음수여야 함", li_avg.h, yu_avg.h);

    // -1~1 범위의 이점: 차이가 곧 감정 분기의 크기
    let a_gap = li_avg.a - yu_avg.a;
    assert!(a_gap > 1.0,
        "원만성 차이({})가 1.0 이상이어야 감정 분기가 극적임", a_gap);
}

// ---------------------------------------------------------------------------
// -1~1 범위의 핵심 이점: 감정 × 성격 = 단순 곱셈 증폭
// ---------------------------------------------------------------------------

#[test]
fn 감정_성격_곱셈_증폭() {
    // 부정 감정(-0.3) × 까칠한 성격(성격이 증폭 역할)
    let emotion_raw = -0.3_f32;
    let patience = Score::new(-0.7, "patience").unwrap(); // 참을성 없음

    // 증폭 계수: 1.0 + |성격값| (성격이 극단적일수록 증폭)
    let amplification = 1.0 + patience.intensity();
    let result = (emotion_raw * amplification).clamp(-1.0, 1.0);

    // -0.3 × 1.7 = -0.51 → 부정 감정이 증폭됨
    assert!(result < emotion_raw,
        "까칠한 성격이 부정 감정을 증폭해야 함: {} → {}", emotion_raw, result);

    // 관용적 성격(+0.7)이면 부정 감정을 완화
    let patient = Score::new(0.7, "patience").unwrap();
    let dampening = 1.0 - patient.value() * 0.5; // 양수 성격은 부정감정 완화
    let result_calm = (emotion_raw * dampening).clamp(-1.0, 1.0);

    assert!(result_calm > emotion_raw,
        "관용적 성격이 부정 감정을 완화해야 함: {} → {}", emotion_raw, result_calm);
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

    assert_eq!(restored.id, NpcId("test_npc".to_string()));
    assert_eq!(restored.name, "테스트");
    assert_eq!(restored.personality.honesty_humility.sincerity.value(), 0.8);
    assert_eq!(restored.personality.agreeableness.forgiveness.value(), -0.5);
    // 설정하지 않은 차원은 중립(0.0)
    assert_eq!(restored.personality.extraversion.sociability.value(), 0.0);
}

// ---------------------------------------------------------------------------
// 수련 — 의리와 절제의 여검객
// ---------------------------------------------------------------------------

/// 수련: 무백을 사랑하지만 의리로 감정을 억누르는 여전사
/// H↑(의리, 공정), E 복합(억눌린 감성), X 보통(당당하지만 내성적),
/// A↑(인내, 온화), C↑↑(극도의 절제와 규율), O↓(전통적, 관습 존중)
#[test]
fn 수련_절제의_여검객() {
    let s = |v: f32| Score::new(v, "").unwrap();

    let shu_lien = NpcBuilder::new("shu_lien", "수련")
        .description("강호의 여협. 무백과 서로 사랑하나 의리로 감정을 억누른다.")
        .honesty_humility(|h| {
            h.sincerity = s(0.8);       // 매우 진실됨
            h.fairness = s(0.9);        // 극도로 공정
            h.greed_avoidance = s(0.7); // 탐욕 없음
            h.modesty = s(0.6);         // 겸손하나 자부심도 있음
        })
        .emotionality(|e| {
            e.fearfulness = s(-0.3);    // 대담한 편
            e.anxiety = s(0.2);         // 약간의 내적 불안 (억눌린 감정)
            e.dependence = s(-0.5);     // 독립적
            e.sentimentality = s(0.7);  // 깊은 감성 (억누르지만 강렬)
        })
        .extraversion(|x| {
            x.social_self_esteem = s(0.4);  // 자신감 있음
            x.social_boldness = s(0.3);     // 당당하지만 나서지 않음
            x.sociability = s(-0.2);        // 내성적인 편
            x.liveliness = s(-0.3);         // 차분함
        })
        .agreeableness(|a| {
            a.forgiveness = s(0.5);     // 용서할 줄 앎
            a.gentleness = s(0.6);      // 온화함
            a.flexibility = s(0.3);     // 어느 정도 유연
            a.patience = s(0.9);        // 극도의 인내 (감정을 수년간 억누름)
        })
        .conscientiousness(|c| {
            c.organization = s(0.6);    // 체계적
            c.diligence = s(0.8);       // 매우 근면
            c.perfectionism = s(0.5);   // 적당한 완벽주의
            c.prudence = s(0.9);        // 극도로 신중 (감정 표현에서도)
        })
        .openness(|o| {
            o.aesthetic_appreciation = s(0.3);  // 약간의 미적 감각
            o.inquisitiveness = s(-0.1);        // 새것 탐구보단 지킴
            o.creativity = s(0.0);              // 중립
            o.unconventionality = s(-0.6);      // 전통과 관습을 매우 존중
        })
        .build();

    let avg = shu_lien.personality.dimension_averages();

    // H: 의리와 공정함 (높은 양수)
    assert!(avg.h > 0.6, "수련의 정직-겸손성은 매우 높아야 함: {}", avg.h);
    // E: 복합적 — 억눌린 감성이 있어 극단적이지 않음
    assert!(avg.e.abs() < 0.3, "수련의 정서성은 복합적(중립 근처)이어야 함: {}", avg.e);
    // A: 인내와 온화함
    assert!(avg.a > 0.4, "수련의 원만성은 높아야 함: {}", avg.a);
    // C: 극도의 절제 — 가장 높은 차원
    assert!(avg.c > 0.6, "수련의 성실성은 매우 높아야 함: {}", avg.c);
    // O: 전통적 (음수 쪽)
    assert!(avg.o < 0.0, "수련의 개방성은 낮아야 함(전통적): {}", avg.o);

    // 수련 vs 무백: 둘 다 H↑ A↑이지만, C에서 수련이 더 극단적
    // 수련 vs 교룡: 거의 모든 차원에서 반대
}

// ---------------------------------------------------------------------------
// 소호 — 자유로운 영혼의 강호 낭인
// ---------------------------------------------------------------------------

/// 소호: 관습에 얽매이지 않는 자유로운 영혼이나
/// 내면엔 의리와 정이 있다. 겉으로는 냉소적이지만 속은 따뜻하다.
/// H 중간(겉과 속이 다름), E↓(대담), X↑(사교적, 활기),
/// A 복합(겉은 냉소, 속은 정), C↓(자유분방), O↑(비관습적, 호기심)
#[test]
fn 소호_자유로운_낭인() {
    let s = |v: f32| Score::new(v, "").unwrap();

    let so_ho = NpcBuilder::new("so_ho", "소호")
        .description("강호를 떠도는 낭인. 냉소적 겉모습 뒤에 뜨거운 의리를 숨긴다.")
        .honesty_humility(|h| {
            h.sincerity = s(0.1);       // 겉과 속이 다름 (진심을 잘 안 보임)
            h.fairness = s(0.5);        // 약자에겐 공정
            h.greed_avoidance = s(0.3); // 돈에 초연한 편
            h.modesty = s(-0.3);        // 허세가 좀 있음
        })
        .emotionality(|e| {
            e.fearfulness = s(-0.7);    // 매우 대담
            e.anxiety = s(-0.5);        // 불안 없음
            e.dependence = s(-0.8);     // 극도로 독립적
            e.sentimentality = s(0.4);  // 속은 의외로 감성적
        })
        .extraversion(|x| {
            x.social_self_esteem = s(0.6);  // 자신감 있음
            x.social_boldness = s(0.7);     // 대담하게 나섬
            x.sociability = s(0.5);         // 사교적, 주막에서 잘 어울림
            x.liveliness = s(0.4);          // 활기참
        })
        .agreeableness(|a| {
            a.forgiveness = s(0.1);     // 원한은 안 품지만 쉽게 용서도 안 함
            a.gentleness = s(-0.4);     // 겉으로 거침 (냉소적)
            a.flexibility = s(0.3);     // 상황에 유연
            a.patience = s(-0.3);       // 참을성 적음
        })
        .conscientiousness(|c| {
            c.organization = s(-0.6);   // 체계 싫어함
            c.diligence = s(0.2);       // 관심 있는 것만 파고듦
            c.perfectionism = s(-0.4);  // 대충대충
            c.prudence = s(-0.5);       // 충동적
        })
        .openness(|o| {
            o.aesthetic_appreciation = s(0.4);  // 풍류를 즐김
            o.inquisitiveness = s(0.7);         // 호기심 강함
            o.creativity = s(0.5);              // 기발한 해결책
            o.unconventionality = s(0.8);       // 관습 무시
        })
        .build();

    let avg = so_ho.personality.dimension_averages();

    // E: 대담함 (강한 음수)
    assert!(avg.e < -0.3, "소호의 정서성은 낮아야 함(대담): {}", avg.e);
    // X: 사교적, 활기 (양수)
    assert!(avg.x > 0.4, "소호의 외향성은 높아야 함: {}", avg.x);
    // C: 자유분방 (음수)
    assert!(avg.c < -0.2, "소호의 성실성은 낮아야 함(자유분방): {}", avg.c);
    // O: 비관습적, 호기심 (높은 양수)
    assert!(avg.o > 0.5, "소호의 개방성은 높아야 함: {}", avg.o);
}

// ---------------------------------------------------------------------------
// 4인 캐릭터 성격 대비 — 2사이클 OCC에서 감정 분기의 기초
// ---------------------------------------------------------------------------

/// 같은 "사부가 독에 맞아 쓰러짐" 상황에서 4인의 반응 예측
/// 성격 조합이 감정 해석을 결정하는 것을 보여줌
#[test]
fn 사인_성격_대비() {
    let s = |v: f32| Score::new(v, "").unwrap();

    // 간략 프로필 (핵심 차원만)
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

    let li_avg = li.personality.dimension_averages();
    let shu_avg = shu.personality.dimension_averages();
    let yu_avg = yu.personality.dimension_averages();
    let na_avg = na.personality.dimension_averages();

    // "사부가 독에 맞아 쓰러짐" 상황에서의 예상 감정 분기:
    //
    // 무백 (A↑, E↓): 슬픔 + 차분한 결의 → "반드시 해독약을 구하겠다"
    //   → patience 높고 fearfulness 낮음 = 감정 억누르고 행동
    assert!(li_avg.a > 0.4 && li_avg.e < -0.2);

    // 수련 (C↑↑, E복합): 깊은 슬픔을 억누르며 체계적 대응
    //   → prudence 극도로 높음 = 감정 숨기고 계획 세움
    assert!(shu_avg.c > 0.6);

    // 교룡 (A↓, X↑): 분노 폭발 → "누가 했어! 당장 찾아서 죽여!"
    //   → patience 극도로 낮고 boldness 높음 = 즉각 공격적 반응
    assert!(yu_avg.a < -0.4 && yu_avg.x > 0.4);

    // 소호 (E↓, C↓): 대담 + 충동적 → "일단 뛰어들어 보자"
    //   → fearfulness 극도로 낮고 prudence 낮음 = 계획 없이 행동
    assert!(na_avg.e < -0.3 && na_avg.c < -0.2);
}
