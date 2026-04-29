use super::*;

fn cid() -> CharacterId {
    CharacterId::new(1)
}

fn cid2() -> CharacterId {
    CharacterId::new(2)
}

fn default_personality() -> HexacoPersonality {
    HexacoPersonality::new(cid(), 50, 50, 50, 50, 50, 50)
}

fn default_values() -> PracticalValues {
    PracticalValues::new(cid(), 50.0, 50.0, 50.0, 50.0, 50.0)
}

fn neutral_mood() -> PadState {
    PadState::neutral()
}

// -- ReflectionTier --

#[test]
fn reflection_tier_serialization() {
    let tiers = [
        ReflectionTier::Instant,
        ReflectionTier::Daily,
        ReflectionTier::TurningPoint,
        ReflectionTier::Life,
    ];
    for tier in tiers {
        let json = serde_json::to_string(&tier).unwrap();
        let restored: ReflectionTier = serde_json::from_str(&json).unwrap();
        assert_eq!(tier, restored);
    }
}

// -- EventConsequence 평가 --

#[test]
fn event_positive_confirmed_joy() {
    let appraisal = OccAppraisal {
        stimulus: OccStimulus::EventConsequence {
            description: "보물 발견".to_string(),
            is_prospective: false,
            concerns_other: None,
        },
        desirability: 0.8,
        praiseworthiness: 0.0,
        appealingness: 0.0,
        relevant_values: vec![],
    };
    let results = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &neutral_mood());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, EmotionType::Joy);
    assert!(results[0].1 > 0.0);
}

#[test]
fn event_negative_confirmed_distress() {
    let appraisal = OccAppraisal {
        stimulus: OccStimulus::EventConsequence {
            description: "제자 납치".to_string(),
            is_prospective: false,
            concerns_other: None,
        },
        desirability: -0.9,
        praiseworthiness: 0.0,
        appealingness: 0.0,
        relevant_values: vec![],
    };
    let results = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &neutral_mood());
    assert_eq!(results[0].0, EmotionType::Distress);
}

#[test]
fn event_positive_prospective_hope() {
    let appraisal = OccAppraisal {
        stimulus: OccStimulus::EventConsequence {
            description: "승리 가능성".to_string(),
            is_prospective: true,
            concerns_other: None,
        },
        desirability: 0.7,
        praiseworthiness: 0.0,
        appealingness: 0.0,
        relevant_values: vec![],
    };
    let results = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &neutral_mood());
    assert_eq!(results[0].0, EmotionType::Hope);
}

#[test]
fn event_negative_prospective_fear() {
    let appraisal = OccAppraisal {
        stimulus: OccStimulus::EventConsequence {
            description: "적 출현".to_string(),
            is_prospective: true,
            concerns_other: None,
        },
        desirability: -0.6,
        praiseworthiness: 0.0,
        appealingness: 0.0,
        relevant_values: vec![],
    };
    let results = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &neutral_mood());
    assert_eq!(results[0].0, EmotionType::Fear);
}

#[test]
fn event_other_positive_happy_for() {
    let appraisal = OccAppraisal {
        stimulus: OccStimulus::EventConsequence {
            description: "동료 승진".to_string(),
            is_prospective: false,
            concerns_other: Some(cid2()),
        },
        desirability: 0.5,
        praiseworthiness: 0.0,
        appealingness: 0.0,
        relevant_values: vec![],
    };
    let results = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &neutral_mood());
    assert_eq!(results[0].0, EmotionType::HappyFor);
}

#[test]
fn event_other_negative_pity() {
    let appraisal = OccAppraisal {
        stimulus: OccStimulus::EventConsequence {
            description: "동료 부상".to_string(),
            is_prospective: false,
            concerns_other: Some(cid2()),
        },
        desirability: -0.6,
        praiseworthiness: 0.0,
        appealingness: 0.0,
        relevant_values: vec![],
    };
    let results = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &neutral_mood());
    assert_eq!(results[0].0, EmotionType::Pity);
}

#[test]
fn event_near_zero_desirability_no_emotion() {
    let appraisal = OccAppraisal {
        stimulus: OccStimulus::EventConsequence {
            description: "무관심".to_string(),
            is_prospective: false,
            concerns_other: None,
        },
        desirability: 0.005,
        praiseworthiness: 0.0,
        appealingness: 0.0,
        relevant_values: vec![],
    };
    let results = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &neutral_mood());
    assert!(results.is_empty());
}

// -- AgentAction 평가 --

#[test]
fn action_self_positive_pride() {
    let appraisal = OccAppraisal {
        stimulus: OccStimulus::AgentAction {
            agent_id: cid(),
            is_self: true,
        },
        desirability: 0.0,
        praiseworthiness: 0.8,
        appealingness: 0.0,
        relevant_values: vec![],
    };
    let results = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &neutral_mood());
    assert_eq!(results[0].0, EmotionType::Pride);
}

#[test]
fn action_self_negative_shame() {
    let appraisal = OccAppraisal {
        stimulus: OccStimulus::AgentAction {
            agent_id: cid(),
            is_self: true,
        },
        desirability: 0.0,
        praiseworthiness: -0.7,
        appealingness: 0.0,
        relevant_values: vec![],
    };
    let results = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &neutral_mood());
    assert_eq!(results[0].0, EmotionType::Shame);
}

#[test]
fn action_other_positive_admiration() {
    let appraisal = OccAppraisal {
        stimulus: OccStimulus::AgentAction {
            agent_id: cid2(),
            is_self: false,
        },
        desirability: 0.0,
        praiseworthiness: 0.9,
        appealingness: 0.0,
        relevant_values: vec![],
    };
    let results = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &neutral_mood());
    assert_eq!(results[0].0, EmotionType::Admiration);
}

#[test]
fn action_other_negative_reproach() {
    let appraisal = OccAppraisal {
        stimulus: OccStimulus::AgentAction {
            agent_id: cid2(),
            is_self: false,
        },
        desirability: 0.0,
        praiseworthiness: -0.9,
        appealingness: 0.0,
        relevant_values: vec![],
    };
    let results = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &neutral_mood());
    assert_eq!(results[0].0, EmotionType::Reproach);
}

// -- 복합 감정 --

#[test]
fn compound_reproach_plus_distress_anger() {
    // 타인의 비난할 행동(-) + 나쁜 결과(-) → Anger
    let appraisal = OccAppraisal {
        stimulus: OccStimulus::AgentAction {
            agent_id: cid2(),
            is_self: false,
        },
        desirability: -0.9,
        praiseworthiness: -0.95,
        appealingness: 0.0,
        relevant_values: vec![(PracticalValueType::Righteousness, 0.9)],
    };
    let results = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &neutral_mood());
    // Reproach + Anger (compound)
    let types: Vec<EmotionType> = results.iter().map(|(t, _)| *t).collect();
    assert!(types.contains(&EmotionType::Reproach));
    assert!(types.contains(&EmotionType::Anger));
}

#[test]
fn compound_admiration_plus_joy_gratitude() {
    // 타인의 칭찬할 행동(+) + 좋은 결과(+) → Gratitude
    let appraisal = OccAppraisal {
        stimulus: OccStimulus::AgentAction {
            agent_id: cid2(),
            is_self: false,
        },
        desirability: 0.8,
        praiseworthiness: 0.9,
        appealingness: 0.0,
        relevant_values: vec![],
    };
    let results = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &neutral_mood());
    let types: Vec<EmotionType> = results.iter().map(|(t, _)| *t).collect();
    assert!(types.contains(&EmotionType::Admiration));
    assert!(types.contains(&EmotionType::Gratitude));
}

#[test]
fn compound_pride_plus_joy_gratification() {
    // 자기 칭찬할 행동(+) + 좋은 결과(+) → Gratification
    let appraisal = OccAppraisal {
        stimulus: OccStimulus::AgentAction {
            agent_id: cid(),
            is_self: true,
        },
        desirability: 0.8,
        praiseworthiness: 0.8,
        appealingness: 0.0,
        relevant_values: vec![],
    };
    let results = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &neutral_mood());
    let types: Vec<EmotionType> = results.iter().map(|(t, _)| *t).collect();
    assert!(types.contains(&EmotionType::Pride));
    assert!(types.contains(&EmotionType::Gratification));
}

#[test]
fn compound_shame_plus_distress_remorse() {
    // 자기 비난할 행동(-) + 나쁜 결과(-) → Remorse
    let appraisal = OccAppraisal {
        stimulus: OccStimulus::AgentAction {
            agent_id: cid(),
            is_self: true,
        },
        desirability: -0.8,
        praiseworthiness: -0.8,
        appealingness: 0.0,
        relevant_values: vec![],
    };
    let results = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &neutral_mood());
    let types: Vec<EmotionType> = results.iter().map(|(t, _)| *t).collect();
    assert!(types.contains(&EmotionType::Shame));
    assert!(types.contains(&EmotionType::Remorse));
}

// -- ObjectAspect 평가 --

#[test]
fn object_positive_love() {
    let appraisal = OccAppraisal {
        stimulus: OccStimulus::ObjectAspect {
            description: "사부의 검".to_string(),
            familiarity: 80.0,
        },
        desirability: 0.0,
        praiseworthiness: 0.0,
        appealingness: 0.7,
        relevant_values: vec![],
    };
    let results = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &neutral_mood());
    assert_eq!(results[0].0, EmotionType::Love);
}

#[test]
fn object_negative_hate() {
    let appraisal = OccAppraisal {
        stimulus: OccStimulus::ObjectAspect {
            description: "조고의 인장".to_string(),
            familiarity: 50.0,
        },
        desirability: 0.0,
        praiseworthiness: 0.0,
        appealingness: -0.8,
        relevant_values: vec![],
    };
    let results = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &neutral_mood());
    assert_eq!(results[0].0, EmotionType::Hate);
}

// -- 가치 가중 --

#[test]
fn value_weight_amplifies_intensity() {
    let high_values = PracticalValues::new(cid(), 50.0, 90.0, 50.0, 50.0, 50.0);
    let low_values = PracticalValues::new(cid(), 50.0, 10.0, 50.0, 50.0, 50.0);

    let appraisal = OccAppraisal {
        stimulus: OccStimulus::EventConsequence {
            description: "도덕 위반".to_string(),
            is_prospective: false,
            concerns_other: None,
        },
        desirability: -0.8,
        praiseworthiness: 0.0,
        appealingness: 0.0,
        relevant_values: vec![(PracticalValueType::Righteousness, 1.0)],
    };

    let high_result = appraise_to_emotions(&appraisal, &high_values, &default_personality(), &neutral_mood());
    let low_result = appraise_to_emotions(&appraisal, &low_values, &default_personality(), &neutral_mood());

    assert!(high_result[0].1 > low_result[0].1,
        "의(90)인 사람이 의(10)인 사람보다 더 강한 고뇌를 느낀다");
}

// -- 기분 편향 --

#[test]
fn mood_bias_affects_intensity() {
    let good_mood = PadState::new(0.8, 0.0, 0.0); // P +0.8 → bias 1.24
    let bad_mood = PadState::new(-0.8, 0.0, 0.0);  // P -0.8 → bias 0.76

    let appraisal = OccAppraisal {
        stimulus: OccStimulus::EventConsequence {
            description: "보물 발견".to_string(),
            is_prospective: false,
            concerns_other: None,
        },
        desirability: 0.7,
        praiseworthiness: 0.0,
        appealingness: 0.0,
        relevant_values: vec![],
    };

    let good_result = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &good_mood);
    let bad_result = appraise_to_emotions(&appraisal, &default_values(), &default_personality(), &bad_mood);

    assert!(good_result[0].1 > bad_result[0].1,
        "기분 좋을 때 긍정 감정이 더 강하다");
}

// -- 성격 필터 --

#[test]
fn personality_filter_affects_basic_emotions() {
    let h90 = HexacoPersonality::new(cid(), 90, 50, 50, 50, 50, 50);
    let h10 = HexacoPersonality::new(cid(), 10, 50, 50, 50, 50, 50);

    let appraisal = OccAppraisal {
        stimulus: OccStimulus::AgentAction {
            agent_id: cid(),
            is_self: true,
        },
        desirability: 0.0,
        praiseworthiness: -0.8,
        appealingness: 0.0,
        relevant_values: vec![],
    };

    let res90 = appraise_to_emotions(&appraisal, &default_values(), &h90, &neutral_mood());
    let res10 = appraise_to_emotions(&appraisal, &default_values(), &h10, &neutral_mood());

    let shame90 = res90.iter().find(|(t, _)| *t == EmotionType::Shame).unwrap().1;
    let shame10 = res10.iter().find(|(t, _)| *t == EmotionType::Shame).unwrap().1;

    assert!(shame90 > shame10, "H90인 사람이 H10인 사람보다 수치심을 더 강하게 느껴야 함");
}

#[test]
fn personality_filter_affects_fear() {
    let e90 = HexacoPersonality::new(cid(), 50, 90, 50, 50, 50, 50);
    let e10 = HexacoPersonality::new(cid(), 50, 10, 50, 50, 50, 50);

    let appraisal = OccAppraisal {
        stimulus: OccStimulus::EventConsequence {
            description: "위험".to_string(),
            is_prospective: true,
            concerns_other: None,
        },
        desirability: -0.8,
        praiseworthiness: 0.0,
        appealingness: 0.0,
        relevant_values: vec![],
    };

    let res90 = appraise_to_emotions(&appraisal, &default_values(), &e90, &neutral_mood());
    let res10 = appraise_to_emotions(&appraisal, &default_values(), &e10, &neutral_mood());

    let fear90 = res90.iter().find(|(t, _)| *t == EmotionType::Fear).unwrap().1;
    let fear10 = res10.iter().find(|(t, _)| *t == EmotionType::Fear).unwrap().1;

    assert!(fear90 > fear10, "E90인 사람이 E10인 사람보다 두려움을 더 강하게 느껴야 함");
}

// -- OccStimulus serialization --

#[test]
fn stimulus_serialization() {
    let stimuli = vec![
        OccStimulus::EventConsequence {
            description: "test".to_string(),
            is_prospective: true,
            concerns_other: Some(cid2()),
        },
        OccStimulus::AgentAction {
            agent_id: cid(),
            is_self: true,
        },
        OccStimulus::ObjectAspect {
            description: "sword".to_string(),
            familiarity: 80.0,
        },
    ];
    for s in stimuli {
        let json = serde_json::to_string(&s).unwrap();
        let restored: OccStimulus = serde_json::from_str(&json).unwrap();
        assert_eq!(s, restored);
    }
}

// -- 무협 시나리오 --

#[test]
fn scenario_myungkyung_disciple_abducted() {
    // 명경: 의90, 충90, H90, A80
    // 제자 납치 → 타인(조고)의 비난할 행동(-0.95) + 나쁜 결과(-0.9)
    let personality = HexacoPersonality::new(cid(), 90, 50, 50, 80, 90, 60);
    let values = PracticalValues::new(cid(), 90.0, 90.0, 70.0, 30.0, 20.0);
    let mood = PadState::neutral();

    let appraisal = OccAppraisal {
        stimulus: OccStimulus::AgentAction {
            agent_id: cid2(),
            is_self: false,
        },
        desirability: -0.9,
        praiseworthiness: -0.95,
        appealingness: 0.0,
        relevant_values: vec![
            (PracticalValueType::Righteousness, 0.9),
            (PracticalValueType::Loyalty, 0.8),
        ],
    };

    let results = appraise_to_emotions(&appraisal, &values, &personality, &mood);
    let types: Vec<EmotionType> = results.iter().map(|(t, _)| *t).collect();

    // Reproach (비난) + Anger (복합 분노) 생성
    assert!(types.contains(&EmotionType::Reproach), "도덕 위반 → 비난");
    assert!(types.contains(&EmotionType::Anger), "비난 + 고뇌 → 분노");

    // 하지만 A80이므로 분노는 억제됨
    if let Some((_, anger_intensity)) = results.iter().find(|(t, _)| *t == EmotionType::Anger) {
        // A=80 → anger filter ×0.68
        // 분노가 존재하지만 억제된 상태
        assert!(*anger_intensity < 100.0, "A80 → 분노 억제");
    }
}

#[test]
fn scenario_jogo_insulted() {
    // 조고: H10, A10 → 모욕 시 분노 거의 억제 안됨
    let personality = HexacoPersonality::new(cid(), 10, 20, 80, 10, 80, 50);
    let values = PracticalValues::new(cid(), 30.0, 10.0, 10.0, 70.0, 90.0);
    let mood = PadState::neutral();

    let appraisal = OccAppraisal {
        stimulus: OccStimulus::AgentAction {
            agent_id: cid2(),
            is_self: false,
        },
        desirability: -0.8,
        praiseworthiness: -0.9,
        appealingness: 0.0,
        relevant_values: vec![(PracticalValueType::Ambition, 0.8)],
    };

    let results = appraise_to_emotions(&appraisal, &values, &personality, &mood);
    let types: Vec<EmotionType> = results.iter().map(|(t, _)| *t).collect();

    assert!(types.contains(&EmotionType::Anger), "조고도 분노한다");
    // A=10 → anger filter ×0.96 → 거의 억제 안됨
    if let Some((_, anger_intensity)) = results.iter().find(|(t, _)| *t == EmotionType::Anger) {
        assert!(*anger_intensity > 50.0, "A10 → 분노 거의 억제 안됨");
    }
}
