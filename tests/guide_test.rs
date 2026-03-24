//! LLM 연기 가이드 생성 테스트
//!
//! 4인 캐릭터의 같은 상황에서 다른 연기 가이드가 생성되는지 검증

mod common;

use npc_mind::domain::emotion::*;
use npc_mind::domain::guide::*;
use npc_mind::ports::GuideFormatter;
use npc_mind::presentation::korean::KoreanFormatter;
use common::{make_무백, make_교룡};

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
    let situation = Situation {
        description: "동료 무사가 적에게 아군의 위치를 밀고했다".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.7,
            outcome_for_self: Some(-0.6),
        },
    };
    let state = AppraisalEngine::appraise(li.personality(), &situation);
    let guide = ActingGuide::build(&li, &state, Some(situation.description.clone()));
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
    let situation = Situation {
        description: "동료 무사가 적에게 아군의 위치를 밀고했다".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.7,
            outcome_for_self: Some(-0.6),
        },
    };
    let state = AppraisalEngine::appraise(yu.personality(), &situation);
    let guide = ActingGuide::build(&yu, &state, Some(situation.description.clone()));
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
    let situation = Situation {
        description: "좋은 소식을 들었다".into(),
        focus: SituationFocus::Event {
            desirability_for_self: 0.6,
            desirability_for_other: None,
            is_prospective: false,
            prior_expectation: None,
        },
    };
    let state = AppraisalEngine::appraise(li.personality(), &situation);
    let guide = ActingGuide::build(&li, &state, Some(situation.description.clone()));
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
    let state = AppraisalEngine::appraise(yu.personality(), &Situation {
        description: "배신".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.7,
            outcome_for_self: Some(-0.6),
        },
    });
    let guide = ActingGuide::build(&yu, &state, Some("배신".into()));
    let formatter = KoreanFormatter::new();
    let json = formatter.format_json(&guide).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["npc_name"].as_str() == Some("교룡"));
    assert!(parsed["directive"]["tone"].is_string());
    assert!(parsed["emotion"]["mood"].is_f64());

    println!("=== 교룡 JSON ===\n{}", json);
}

// ---------------------------------------------------------------------------
// 같은 상황, 다른 어조 비교
// ---------------------------------------------------------------------------

#[test]
fn 같은_상황_무백과_교룡_어조가_다름() {
    let li = make_무백();
    let yu = make_교룡();
    let situation = Situation {
        description: "동료의 배신".into(),
        focus: SituationFocus::Action {
            is_self_agent: false,
            praiseworthiness: -0.7,
            outcome_for_self: Some(-0.6),
        },
    };

    let li_state = AppraisalEngine::appraise(li.personality(), &situation);
    let yu_state = AppraisalEngine::appraise(yu.personality(), &situation);

    let li_guide = ActingGuide::build(&li, &li_state, None);
    let yu_guide = ActingGuide::build(&yu, &yu_state, None);

    assert_ne!(li_guide.directive.tone, yu_guide.directive.tone,
        "무백({:?})과 교룡({:?})의 어조가 달라야 함",
        li_guide.directive.tone, yu_guide.directive.tone);

    assert_ne!(li_guide.directive.attitude, yu_guide.directive.attitude,
        "무백({:?})과 교룡({:?})의 태도가 달라야 함",
        li_guide.directive.attitude, yu_guide.directive.attitude);
}
