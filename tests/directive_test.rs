//! ActingDirective 의사결정 트리 테스트
//!
//! 감정 + 성격 조합에서 어조(Tone), 태도(Attitude), 행동경향, 금지사항이
//! 올바르게 도출되는지 분기별로 검증합니다.

mod common;

use npc_mind::domain::emotion::*;
use npc_mind::domain::guide::*;
use npc_mind::domain::personality::*;
use common::{make_무백, make_교룡, score as s};

/// 특정 감정만 주입한 EmotionState를 만듭니다.
fn state_with(emotions: &[(EmotionType, f32)]) -> EmotionState {
    let mut state = EmotionState::new();
    for &(etype, intensity) in emotions {
        state.add(Emotion::new(etype, intensity));
    }
    state
}

// ===========================================================================
// 어조(Tone) 분기 테스트
// ===========================================================================

#[test]
fn anger_dominant_성실한_성격이면_suppressed_cold() {
    // 무백: C(성실성) 평균 > TRAIT_THRESHOLD(0.3)
    let li = make_무백();
    let state = state_with(&[(EmotionType::Anger, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert_eq!(d.tone, Tone::SuppressedCold,
        "Anger dominant + C↑ → SuppressedCold");
}

#[test]
fn anger_dominant_충동적_성격이면_rough_aggressive() {
    // 교룡: C(성실성) 평균 < -TRAIT_THRESHOLD
    let yu = make_교룡();
    let state = state_with(&[(EmotionType::Anger, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, yu.personality());
    assert_eq!(d.tone, Tone::RoughAggressive,
        "Anger dominant + C↓ → RoughAggressive");
}

#[test]
fn fear_dominant_대담한_성격이면_vigilant_calm() {
    // 무백: E(정서성) 평균 < -TRAIT_THRESHOLD (fearfulness=-0.6 등)
    let li = make_무백();
    let state = state_with(&[(EmotionType::Fear, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert_eq!(d.tone, Tone::VigilantCalm,
        "Fear dominant + E↓ → VigilantCalm");
}

#[test]
fn fear_dominant_정서적_성격이면_tense_anxious() {
    // E 높은 캐릭터 필요 — 수련 사용 (E 중립이므로 커스텀 빌드)
    let npc = NpcBuilder::new("test", "테스트")
        .emotionality(|e| {
            e.fearfulness = s(0.6); e.anxiety = s(0.7);
            e.dependence = s(0.5); e.sentimentality = s(0.4);
        })
        .build();
    let state = state_with(&[(EmotionType::Fear, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, npc.personality());
    assert_eq!(d.tone, Tone::TenseAnxious,
        "Fear dominant + E↑ → TenseAnxious");
}

#[test]
fn joy_dominant이면_bright_lively() {
    let li = make_무백();
    let state = state_with(&[(EmotionType::Joy, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert_eq!(d.tone, Tone::BrightLively);
}

#[test]
fn shame_dominant이면_shrinking_small() {
    let li = make_무백();
    let state = state_with(&[(EmotionType::Shame, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert_eq!(d.tone, Tone::ShrinkingSmall);
}

#[test]
fn pride_dominant_정직한_성격이면_quiet_confidence() {
    // 무백: H(정직겸손) 평균 > TRAIT_THRESHOLD
    let li = make_무백();
    let state = state_with(&[(EmotionType::Pride, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert_eq!(d.tone, Tone::QuietConfidence,
        "Pride dominant + H↑ → QuietConfidence");
}

#[test]
fn pride_dominant_교활한_성격이면_proud_arrogant() {
    // 교룡: H(정직겸손) 평균 < -TRAIT_THRESHOLD
    let yu = make_교룡();
    let state = state_with(&[(EmotionType::Pride, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, yu.personality());
    assert_eq!(d.tone, Tone::ProudArrogant,
        "Pride dominant + H↓ → ProudArrogant");
}

#[test]
fn 감정_없으면_mood_기반_tone() {
    let li = make_무백();

    // 빈 상태 → mood=0 → Calm
    let empty = EmotionState::new();
    let d = ActingDirective::from_emotion_and_personality(&empty, li.personality());
    assert_eq!(d.tone, Tone::Calm, "감정 없으면 Calm");
}

#[test]
fn 나머지_감정_tone_매핑() {
    let li = make_무백();

    let cases = [
        (EmotionType::Distress, Tone::SomberRestrained),  // 무백 E↓ → SomberRestrained
        (EmotionType::Reproach, Tone::CynicalCritical),
        (EmotionType::Disappointment, Tone::DeepSighing),
        (EmotionType::Gratitude, Tone::SincerelyWarm),
        (EmotionType::Resentment, Tone::JealousBitter),
        (EmotionType::Pity, Tone::CompassionateSoft),
    ];

    for (etype, expected_tone) in cases {
        let state = state_with(&[(etype, 0.8)]);
        let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
        assert_eq!(d.tone, expected_tone,
            "{:?} dominant → {:?}", etype, expected_tone);
    }
}

// ===========================================================================
// 태도(Attitude) 분기 테스트
// ===========================================================================

#[test]
fn anger_비판적_성격이면_hostile_aggressive() {
    // 교룡: A(원만성) 평균 < -TRAIT_THRESHOLD
    let yu = make_교룡();
    let state = state_with(&[(EmotionType::Anger, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, yu.personality());
    assert_eq!(d.attitude, Attitude::HostileAggressive,
        "Anger + A↓ → HostileAggressive");
}

#[test]
fn anger_관용적_성격이면_suppressed_discomfort() {
    // 무백: A(원만성) 평균 > TRAIT_THRESHOLD
    let li = make_무백();
    let state = state_with(&[(EmotionType::Anger, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert_eq!(d.attitude, Attitude::SuppressedDiscomfort,
        "Anger + A↑ → SuppressedDiscomfort");
}

#[test]
fn reproach이면_judgmental() {
    let li = make_무백();
    let state = state_with(&[(EmotionType::Reproach, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert_eq!(d.attitude, Attitude::Judgmental);
}

#[test]
fn fear이면_guarded_defensive() {
    let li = make_무백();
    let state = state_with(&[(EmotionType::Fear, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert_eq!(d.attitude, Attitude::GuardedDefensive);
}

#[test]
fn 긍정_mood이면_friendly_open() {
    let li = make_무백();
    let state = state_with(&[(EmotionType::Joy, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert_eq!(d.attitude, Attitude::FriendlyOpen,
        "Joy → 긍정 mood → FriendlyOpen");
}

#[test]
fn 부정_mood이면_defensive_closed() {
    let li = make_무백();
    let state = state_with(&[(EmotionType::Distress, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert_eq!(d.attitude, Attitude::DefensiveClosed,
        "Distress only (anger/reproach/fear 없음) → 부정 mood → DefensiveClosed");
}

#[test]
fn 중립이면_neutral_observant() {
    let li = make_무백();
    let empty = EmotionState::new();
    let d = ActingDirective::from_emotion_and_personality(&empty, li.personality());
    assert_eq!(d.attitude, Attitude::NeutralObservant);
}

// ===========================================================================
// 행동경향(BehavioralTendency) 분기 테스트
// ===========================================================================

#[test]
fn anger_충동적이면_immediate_confrontation() {
    // 교룡: C↓
    let yu = make_교룡();
    let state = state_with(&[(EmotionType::Anger, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, yu.personality());
    assert_eq!(d.behavioral_tendency, BehavioralTendency::ImmediateConfrontation);
}

#[test]
fn anger_성실하면_strategic_response() {
    // 무백: C↑
    let li = make_무백();
    let state = state_with(&[(EmotionType::Anger, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert_eq!(d.behavioral_tendency, BehavioralTendency::StrategicResponse);
}

#[test]
fn anger_중간_성실성이면_express_and_observe() {
    // C가 중립인 캐릭터
    let npc = NpcBuilder::new("test", "테스트").build(); // 기본값은 모두 중립
    let state = state_with(&[(EmotionType::Anger, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, npc.personality());
    assert_eq!(d.behavioral_tendency, BehavioralTendency::ExpressAndObserve);
}

#[test]
fn fear_대담하면_brave_confrontation() {
    // 무백: E↓
    let li = make_무백();
    let state = state_with(&[(EmotionType::Fear, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert_eq!(d.behavioral_tendency, BehavioralTendency::BraveConfrontation);
}

#[test]
fn fear_정서적이면_seek_safety() {
    let npc = NpcBuilder::new("test", "테스트")
        .emotionality(|e| {
            e.fearfulness = s(0.6); e.anxiety = s(0.7);
            e.dependence = s(0.5); e.sentimentality = s(0.4);
        })
        .build();
    let state = state_with(&[(EmotionType::Fear, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, npc.personality());
    assert_eq!(d.behavioral_tendency, BehavioralTendency::SeekSafety);
}

#[test]
fn shame이면_avoid_or_deflect() {
    let li = make_무백();
    let state = state_with(&[(EmotionType::Shame, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert_eq!(d.behavioral_tendency, BehavioralTendency::AvoidOrDeflect);
}

#[test]
fn 긍정_mood이면_active_cooperation() {
    let li = make_무백();
    let state = state_with(&[(EmotionType::Joy, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert_eq!(d.behavioral_tendency, BehavioralTendency::ActiveCooperation);
}

#[test]
fn 중립이면_observe_and_respond() {
    let li = make_무백();
    let empty = EmotionState::new();
    let d = ActingDirective::from_emotion_and_personality(&empty, li.personality());
    assert_eq!(d.behavioral_tendency, BehavioralTendency::ObserveAndRespond);
}

// ===========================================================================
// 금지사항(Restriction) 테스트
// ===========================================================================

#[test]
fn 부정_mood이면_no_humor() {
    let li = make_무백();
    let state = state_with(&[(EmotionType::Distress, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert!(d.restrictions.contains(&Restriction::NoHumorOrLightTone),
        "부정 mood → NoHumorOrLightTone");
}

#[test]
fn anger이면_no_friendliness() {
    let li = make_무백();
    let state = state_with(&[(EmotionType::Anger, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert!(d.restrictions.contains(&Restriction::NoFriendliness),
        "Anger → NoFriendliness");
}

#[test]
fn shame이면_no_self_justification() {
    let li = make_무백();
    let state = state_with(&[(EmotionType::Shame, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert!(d.restrictions.contains(&Restriction::NoSelfJustification),
        "Shame → NoSelfJustification");
}

#[test]
fn fear이면_no_bravado() {
    let li = make_무백();
    let state = state_with(&[(EmotionType::Fear, 0.8)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert!(d.restrictions.contains(&Restriction::NoBravado),
        "Fear → NoBravado");
}

#[test]
fn 정직한_성격이면_no_lying() {
    // 무백: H 평균 > HONESTY_RESTRICTION_THRESHOLD(0.5)
    let li = make_무백();
    let state = state_with(&[(EmotionType::Joy, 0.5)]);
    let d = ActingDirective::from_emotion_and_personality(&state, li.personality());
    assert!(d.restrictions.contains(&Restriction::NoLyingOrExaggeration),
        "H↑ → NoLyingOrExaggeration");
}

#[test]
fn 교활한_성격은_거짓말_허용() {
    let yu = make_교룡();
    let state = state_with(&[(EmotionType::Joy, 0.5)]);
    let d = ActingDirective::from_emotion_and_personality(&state, yu.personality());
    assert!(!d.restrictions.contains(&Restriction::NoLyingOrExaggeration),
        "H↓ → NoLyingOrExaggeration 없음");
}

#[test]
fn 빈_감정은_제한사항_최소() {
    let yu = make_교룡(); // H↓ → NoLying 없음
    let empty = EmotionState::new();
    let d = ActingDirective::from_emotion_and_personality(&empty, yu.personality());
    assert!(d.restrictions.is_empty(),
        "빈 감정 + H↓ → 금지사항 없음: {:?}", d.restrictions);
}

// ===========================================================================
// 열거형별 개별 decide 메서드 유닛 테스트 (리팩토링 검증)
// ===========================================================================

#[cfg(test)]
mod enum_decide_unit_tests {
    use super::*;

    #[test]
    fn test_tone_decide_logic() {
        let avg = NpcBuilder::new("t", "t").build().personality().dimension_averages();
        
        // 1. 감정 없음 + 중립 기분
        assert_eq!(Tone::decide(None, 0.0, &avg), Tone::Calm);
        
        // 2. 긍정 기분
        assert_eq!(Tone::decide(None, 0.5, &avg), Tone::RelaxedGentle);
        
        // 3. 부정 기분
        assert_eq!(Tone::decide(None, -0.5, &avg), Tone::Heavy);
        
        // 4. 특정 감정 dominant
        assert_eq!(Tone::decide(Some(EmotionType::Joy), 0.5, &avg), Tone::BrightLively);
        assert_eq!(Tone::decide(Some(EmotionType::Shame), -0.5, &avg), Tone::ShrinkingSmall);
    }

    #[test]
    fn test_attitude_decide_logic() {
        let avg = NpcBuilder::new("t", "t").build().personality().dimension_averages();

        // 1. 기본 중립
        assert_eq!(Attitude::decide(false, false, false, 0.0, &avg), Attitude::NeutralObservant);

        // 2. 비난(Reproach) 존재
        assert_eq!(Attitude::decide(false, true, false, 0.0, &avg), Attitude::Judgmental);

        // 3. 두려움(Fear) 존재
        assert_eq!(Attitude::decide(false, false, true, 0.0, &avg), Attitude::GuardedDefensive);
    }

    #[test]
    fn test_behavioral_tendency_decide_logic() {
        let avg = NpcBuilder::new("t", "t").build().personality().dimension_averages();

        // 1. 기본 관찰
        assert_eq!(BehavioralTendency::decide(false, false, false, 0.0, &avg), BehavioralTendency::ObserveAndRespond);

        // 2. 수치심(Shame) 존재
        assert_eq!(BehavioralTendency::decide(false, false, true, 0.0, &avg), BehavioralTendency::AvoidOrDeflect);

        // 3. 긍정 기분
        assert_eq!(BehavioralTendency::decide(false, false, false, 0.5, &avg), BehavioralTendency::ActiveCooperation);
    }

    #[test]
    fn test_restriction_evaluate_all_logic() {
        let avg = NpcBuilder::new("t", "t").build().personality().dimension_averages();

        // 1. 중립일 때 빈 목록
        assert!(Restriction::evaluate_all(false, false, false, 0.0, &avg).is_empty());

        // 2. 분노 시 NoFriendliness 포함
        let res = Restriction::evaluate_all(true, false, false, 0.0, &avg);
        assert!(res.contains(&Restriction::NoFriendliness));

        // 3. 부정 기분 시 NoHumor 포함
        let res = Restriction::evaluate_all(false, false, false, -0.5, &avg);
        assert!(res.contains(&Restriction::NoHumorOrLightTone));
    }
}
