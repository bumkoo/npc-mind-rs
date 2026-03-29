//! LLM 연기 가이드 생성 테스트
//!
//! 4인 캐릭터의 같은 상황에서 다른 연기 가이드가 생성되는지 검증

mod common;

use npc_mind::domain::emotion::*;
use npc_mind::domain::guide::*;
use npc_mind::domain::relationship::RelationshipBuilder;
use npc_mind::ports::GuideFormatter;
use npc_mind::presentation::korean::KoreanFormatter;
use common::{make_무백, make_교룡, score as s, neutral_rel};

/// 배신 상황 헬퍼 (Action + Event)
fn 배신_상황() -> Situation {
    Situation::new(
        "동료 무사가 적에게 아군의 위치를 밀고했다",
        Some(EventFocus {
            description: "".into(),
            desirability_for_self: -0.6,
            desirability_for_other: None,
            prospect: None,
        }),
        Some(ActionFocus {
            description: "".into(),
            agent_id: Some("partner".into()), relationship: None,
            praiseworthiness: -0.7,
        }),
        None,
    ).unwrap()
}

// ---------------------------------------------------------------------------
// 성격 스냅샷 테스트 (도메인)
// ---------------------------------------------------------------------------

#[test]
fn 무백_성격_스냅샷_정직_관용() {
    let li = make_무백();
    let snapshot = PersonalitySnapshot::from_profile(li.personality());

    assert!(snapshot.traits.contains(&PersonalityTrait::HonestAndModest),
        "무백의 성격에 '정직겸손' 포함: {:?}", snapshot.traits);
    assert!(snapshot.traits.contains(&PersonalityTrait::TolerantAndGentle),
        "무백의 성격에 '관용온화' 포함: {:?}", snapshot.traits);
    assert!(snapshot.speech_styles.contains(&SpeechStyle::FrankAndUnadorned)
        || snapshot.speech_styles.contains(&SpeechStyle::SoftAndConsiderate),
        "무백의 말투에 솔직/부드러운: {:?}", snapshot.speech_styles);
}

#[test]
fn 교룡_성격_스냅샷_교활_비판적() {
    let yu = make_교룡();
    let snapshot = PersonalitySnapshot::from_profile(yu.personality());

    assert!(snapshot.traits.contains(&PersonalityTrait::CunningAndAmbitious),
        "교룡의 성격에 '교활야심' 포함: {:?}", snapshot.traits);
    assert!(snapshot.traits.contains(&PersonalityTrait::GrudgingAndCritical),
        "교룡의 성격에 '비판적' 포함: {:?}", snapshot.traits);
    assert!(snapshot.traits.contains(&PersonalityTrait::CuriousAndCreative),
        "교룡의 성격에 '호기심' 포함: {:?}", snapshot.traits);
}

// ---------------------------------------------------------------------------
// 같은 상황 → 다른 연기 가이드 테스트
// ---------------------------------------------------------------------------

#[test]
fn 배신_무백_가이드_절제된_분노() {
    let li = make_무백();
    let situation = 배신_상황();
    let state = AppraisalEngine::appraise(li.personality(), &situation, &neutral_rel());
    let guide = ActingGuide::build(&li, &state, Some(situation.description.clone()), None);
    let formatter = KoreanFormatter::new();
    let prompt = formatter.format_prompt(&guide);

    assert!(prompt.contains("억누") || prompt.contains("절제") || prompt.contains("차가운"),
        "무백 가이드에 절제 관련 표현: {}", prompt);
    assert!(prompt.contains("호의적") || prompt.contains("금지"),
        "무백 가이드에 금지 사항 포함: {}", prompt);

    println!("=== 무백의 배신 가이드 ===\n{}", prompt);
}

#[test]
fn 배신_교룡_가이드_폭발적_분노() {
    let yu = make_교룡();
    let situation = 배신_상황();
    let state = AppraisalEngine::appraise(yu.personality(), &situation, &neutral_rel());
    let guide = ActingGuide::build(&yu, &state, Some(situation.description.clone()), None);
    let formatter = KoreanFormatter::new();
    let prompt = formatter.format_prompt(&guide);

    assert!(prompt.contains("거칠") || prompt.contains("공격"),
        "교룡 가이드에 공격적 표현: {}", prompt);
    assert!(prompt.contains("적대") || prompt.contains("공격적인 태도"),
        "교룡 가이드에 적대적 태도: {}", prompt);

    println!("=== 교룡의 배신 가이드 ===\n{}", prompt);
}

// ---------------------------------------------------------------------------
// 프롬프트 포맷 검증
// ---------------------------------------------------------------------------

#[test]
fn 가이드_프롬프트_구조_검증() {
    let li = make_무백();
    let situation = Situation::new(
        "좋은 소식을 들었다",
        Some(EventFocus {
            description: "".into(),
            desirability_for_self: 0.6,
            desirability_for_other: None,
            prospect: None,
        }),
        None,
        None,
    ).unwrap();
    let state = AppraisalEngine::appraise(li.personality(), &situation, &neutral_rel());
    let guide = ActingGuide::build(&li, &state, Some(situation.description.clone()), None);
    let formatter = KoreanFormatter::new();
    let prompt = formatter.format_prompt(&guide);

    assert!(prompt.contains("[NPC: 무백]"), "NPC 이름 섹션");
    assert!(prompt.contains("[성격]"), "성격 섹션");
    assert!(prompt.contains("[현재 감정]"), "감정 섹션");
    assert!(prompt.contains("[상황]"), "상황 섹션");
    assert!(prompt.contains("[연기 지시]"), "연기 지시 섹션");
    assert!(prompt.contains("[말투]"), "말투 섹션");
}

#[test]
fn 가이드_json_출력() {
    let yu = make_교룡();
    let situation = 배신_상황();
    let state = AppraisalEngine::appraise(yu.personality(), &situation, &neutral_rel());
    let guide = ActingGuide::build(&yu, &state, Some("배신".into()), None);
    let formatter = KoreanFormatter::new();
    let json = formatter.format_json(&guide).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["npc_name"].as_str() == Some("교룡"));
    assert!(parsed["directive"]["tone"].is_string());
    assert!(parsed["emotion"]["mood"].is_f64());

    println!("=== 교룡 JSON ===\n{}", json);
}

#[test]
fn 같은_상황_무백과_교룡_어조가_다름() {
    let li = make_무백();
    let yu = make_교룡();
    let situation = 배신_상황();

    let li_state = AppraisalEngine::appraise(li.personality(), &situation, &neutral_rel());
    let yu_state = AppraisalEngine::appraise(yu.personality(), &situation, &neutral_rel());

    let li_guide = ActingGuide::build(&li, &li_state, None, None);
    let yu_guide = ActingGuide::build(&yu, &yu_state, None, None);

    // 엔진 보장: 성격 차이로 인해 감정 강도가 달라야 함
    let li_anger = li_state.emotions().iter()
        .find(|e| e.emotion_type() == EmotionType::Anger)
        .map(|e| e.intensity()).unwrap_or(0.0);
    let yu_anger = yu_state.emotions().iter()
        .find(|e| e.emotion_type() == EmotionType::Anger)
        .map(|e| e.intensity()).unwrap_or(0.0);
    assert!(yu_anger > li_anger,
        "교룡 분노({}) > 무백 분노({}) — 성격 차이 반영",
        yu_anger, li_anger);

    // 가이드: 어조와 태도 모두 달라야 함
    assert_ne!(li_guide.directive.tone, yu_guide.directive.tone,
        "무백({:?})과 교룡({:?})의 어조가 달라야 함",
        li_guide.directive.tone, yu_guide.directive.tone);

    assert_ne!(li_guide.directive.attitude, yu_guide.directive.attitude,
        "무백({:?})과 교룡({:?})의 태도가 달라야 함",
        li_guide.directive.attitude, yu_guide.directive.attitude);
}

// ---------------------------------------------------------------------------
// 관계 포함 가이드 테스트
// ---------------------------------------------------------------------------

#[test]
fn 관계_포함_가이드_프롬프트에_관계_섹션() {
    let li = make_무백();
    let situation = 배신_상황();

    let brother = RelationshipBuilder::new("mu_baek", "gyo_ryong")
        .closeness(s(0.9))
        .trust(s(0.8))
        .power(s(0.0))
        .build();

    let state = AppraisalEngine::appraise(li.personality(), &situation, &brother);
    let guide = ActingGuide::build(&li, &state, Some("동료의 배신".into()), Some(&brother));
    let formatter = KoreanFormatter::new();
    let prompt = formatter.format_prompt(&guide);

    assert!(prompt.contains("[상대와의 관계]"), "관계 섹션 헤더: {}", prompt);
    assert!(prompt.contains("친밀도"), "친밀도 라벨: {}", prompt);
    assert!(prompt.contains("신뢰도"), "신뢰도 라벨: {}", prompt);
    assert!(prompt.contains("상하 관계"), "상하 관계 라벨: {}", prompt);

    println!("=== 의형제 배신 가이드 (관계 포함) ===\n{}", prompt);
}

#[test]
fn 관계_포함_json에_관계_데이터() {
    let yu = make_교룡();
    let enemy = RelationshipBuilder::new("gyo_ryong", "enemy")
        .closeness(s(-0.7))
        .trust(s(-0.8))
        .power(s(0.0))
        .build();

    let situation = 배신_상황();
    let state = AppraisalEngine::appraise(yu.personality(), &situation, &enemy);

    let guide = ActingGuide::build(&yu, &state, Some("배신".into()), Some(&enemy));
    let formatter = KoreanFormatter::new();
    let json = formatter.format_json(&guide).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(parsed["relationship"].is_object(), "JSON에 관계 데이터 포함");
    assert!(parsed["relationship"]["target_name"].as_str() == Some("enemy"));
    assert!(parsed["relationship"]["closeness"].is_string());
    assert!(parsed["relationship"]["trust"].is_string());
    assert!(parsed["relationship"]["power"].is_string());

    println!("=== 숙적 배신 JSON (관계 포함) ===\n{}", json);
}

#[test]
fn 관계_없으면_json에_관계_없음() {
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
    ).unwrap();
    let state = AppraisalEngine::appraise(li.personality(), &situation, &neutral_rel());

    let guide = ActingGuide::build(&li, &state, None, None);
    let formatter = KoreanFormatter::new();
    let json = formatter.format_json(&guide).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(parsed.get("relationship").is_none(),
        "관계 없으면 JSON에서 생략");
}

// ===========================================================================
// 이슈 4: PowerLevel 5단계 분류 검증
// ===========================================================================

#[test]
fn power_minus03은_low() {
    assert_eq!(PowerLevel::from_score(-0.3), PowerLevel::Low);
}

#[test]
fn power_0은_neutral() {
    assert_eq!(PowerLevel::from_score(0.0), PowerLevel::Neutral);
}

#[test]
fn power_05는_high() {
    assert_eq!(PowerLevel::from_score(0.5), PowerLevel::High);
}

#[test]
fn power_07은_very_high() {
    assert_eq!(PowerLevel::from_score(0.7), PowerLevel::VeryHigh);
}

#[test]
fn power_minus07은_very_low() {
    assert_eq!(PowerLevel::from_score(-0.7), PowerLevel::VeryLow);
}
