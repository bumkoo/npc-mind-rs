use super::*;

// -- EmotionType 22종 --

#[test]
fn all_has_22_types() {
    assert_eq!(EmotionType::ALL.len(), 22);
}

#[test]
fn all_unique() {
    let mut set = std::collections::HashSet::new();
    for et in EmotionType::ALL {
        assert!(set.insert(et), "Duplicate: {:?}", et);
    }
}

// -- category() --

#[test]
fn event_consequence_emotions() {
    let ec = [
        EmotionType::Joy, EmotionType::Distress,
        EmotionType::Hope, EmotionType::Fear,
        EmotionType::Satisfaction, EmotionType::FearsConfirmed,
        EmotionType::Relief, EmotionType::Disappointment,
        EmotionType::HappyFor, EmotionType::Pity,
        EmotionType::Gloating, EmotionType::Resentment,
    ];
    for et in ec {
        assert_eq!(et.category(), EmotionCategory::EventConsequence, "{:?}", et);
    }
}

#[test]
fn agent_action_emotions() {
    let aa = [
        EmotionType::Pride, EmotionType::Shame,
        EmotionType::Admiration, EmotionType::Reproach,
    ];
    for et in aa {
        assert_eq!(et.category(), EmotionCategory::AgentAction, "{:?}", et);
    }
}

#[test]
fn compound_emotions() {
    let compound = [
        EmotionType::Gratification, EmotionType::Remorse,
        EmotionType::Gratitude, EmotionType::Anger,
    ];
    for et in compound {
        assert_eq!(et.category(), EmotionCategory::Compound, "{:?}", et);
    }
}

#[test]
fn object_aspect_emotions() {
    assert_eq!(EmotionType::Love.category(), EmotionCategory::ObjectAspect);
    assert_eq!(EmotionType::Hate.category(), EmotionCategory::ObjectAspect);
}

// -- valence() --

#[test]
fn positive_emotions() {
    let positive = [
        EmotionType::Joy, EmotionType::Hope, EmotionType::Satisfaction,
        EmotionType::Relief, EmotionType::HappyFor, EmotionType::Gloating,
        EmotionType::Pride, EmotionType::Admiration,
        EmotionType::Gratification, EmotionType::Gratitude, EmotionType::Love,
    ];
    for et in positive {
        assert_eq!(et.valence(), Valence::Positive, "{:?}", et);
    }
}

#[test]
fn negative_emotions() {
    let negative = [
        EmotionType::Distress, EmotionType::Fear, EmotionType::FearsConfirmed,
        EmotionType::Disappointment, EmotionType::Pity, EmotionType::Resentment,
        EmotionType::Shame, EmotionType::Reproach,
        EmotionType::Remorse, EmotionType::Anger, EmotionType::Hate,
    ];
    for et in negative {
        assert_eq!(et.valence(), Valence::Negative, "{:?}", et);
    }
}

#[test]
fn positive_and_negative_count() {
    let pos_count = EmotionType::ALL.iter().filter(|e| e.valence() == Valence::Positive).count();
    let neg_count = EmotionType::ALL.iter().filter(|e| e.valence() == Valence::Negative).count();
    assert_eq!(pos_count, 11);
    assert_eq!(neg_count, 11);
}

// -- half_life_hours() --

#[test]
fn relief_fastest_decay() {
    assert_eq!(EmotionType::Relief.half_life_hours(), 2.0);
}

#[test]
fn love_hate_no_decay() {
    assert!(EmotionType::Love.half_life_hours().is_infinite());
    assert!(EmotionType::Hate.half_life_hours().is_infinite());
}

#[test]
fn anger_slow_decay() {
    assert_eq!(EmotionType::Anger.half_life_hours(), 24.0);
}

#[test]
fn remorse_slowest_finite_decay() {
    assert_eq!(EmotionType::Remorse.half_life_hours(), 48.0);
}

#[test]
fn all_half_lives_positive() {
    for et in EmotionType::ALL {
        assert!(et.half_life_hours() > 0.0, "{:?}", et);
    }
}

// -- pad_delta() --

#[test]
fn joy_positive_pleasure() {
    let (p, a, d) = EmotionType::Joy.pad_delta();
    assert!(p > 0.0);
    assert!(a > 0.0);
    assert!(d > 0.0);
}

#[test]
fn anger_negative_pleasure_high_arousal() {
    let (p, a, d) = EmotionType::Anger.pad_delta();
    assert!(p < 0.0, "분노는 불쾌");
    assert_eq!(a, 0.5, "분노는 최고 각성");
    assert!(d > 0.0, "분노는 지배감");
}

#[test]
fn fear_negative_dominance() {
    let (p, _a, d) = EmotionType::Fear.pad_delta();
    assert!(p < 0.0, "두려움은 불쾌");
    assert!(d < 0.0, "두려움은 무력감");
    assert_eq!(d, -0.4);
}

#[test]
fn fears_confirmed_extreme_negative() {
    let (p, _a, d) = EmotionType::FearsConfirmed.pad_delta();
    assert_eq!(p, -0.5, "절망은 극도의 불쾌");
    assert_eq!(d, -0.5, "절망은 극도의 무력감");
}

#[test]
fn all_pad_deltas_in_range() {
    for et in EmotionType::ALL {
        let (p, a, d) = et.pad_delta();
        assert!((-1.0..=1.0).contains(&p), "{:?} P={}", et, p);
        assert!((-1.0..=1.0).contains(&a), "{:?} A={}", et, a);
        assert!((-1.0..=1.0).contains(&d), "{:?} D={}", et, d);
    }
}

// -- ActiveEmotion --

#[test]
fn active_emotion_new() {
    let ae = ActiveEmotion::new(
        EmotionType::Anger,
        72.5,
        "제자 납치".to_string(),
        Some(CharacterId::new(2)),
        GameTime::new(1200, 3, 15),
    );
    assert_eq!(ae.emotion_type(), EmotionType::Anger);
    assert_eq!(ae.intensity(), 72.5);
    assert_eq!(ae.source_description(), "제자 납치");
    assert_eq!(ae.source_agent(), Some(CharacterId::new(2)));
}

#[test]
fn active_emotion_clamps_intensity() {
    let ae = ActiveEmotion::new(
        EmotionType::Joy,
        150.0,
        "test".to_string(),
        None,
        GameTime::new(1200, 1, 1),
    );
    assert_eq!(ae.intensity(), 100.0);

    let ae2 = ActiveEmotion::new(
        EmotionType::Joy,
        -10.0,
        "test".to_string(),
        None,
        GameTime::new(1200, 1, 1),
    );
    assert_eq!(ae2.intensity(), 0.0);
}

#[test]
fn active_emotion_set_intensity() {
    let mut ae = ActiveEmotion::new(
        EmotionType::Fear,
        80.0,
        "test".to_string(),
        None,
        GameTime::new(1200, 1, 1),
    );
    ae.set_intensity(40.0);
    assert_eq!(ae.intensity(), 40.0);
    ae.set_intensity(110.0);
    assert_eq!(ae.intensity(), 100.0);
}

#[test]
fn active_emotion_is_expired() {
    let ae = ActiveEmotion::new(
        EmotionType::Relief,
        0.5,
        "test".to_string(),
        None,
        GameTime::new(1200, 1, 1),
    );
    assert!(ae.is_expired(1.0), "0.5 < 1.0 → expired");
    assert!(!ae.is_expired(0.3), "0.5 >= 0.3 → not expired");
}

// -- Serialization --

#[test]
fn emotion_type_serialization() {
    for et in EmotionType::ALL {
        let json = serde_json::to_string(&et).unwrap();
        let restored: EmotionType = serde_json::from_str(&json).unwrap();
        assert_eq!(et, restored);
    }
}

#[test]
fn active_emotion_serialization() {
    let ae = ActiveEmotion::new(
        EmotionType::Anger,
        72.5,
        "제자 납치".to_string(),
        Some(CharacterId::new(2)),
        GameTime::new(1200, 3, 15),
    );
    let json = serde_json::to_string(&ae).unwrap();
    let restored: ActiveEmotion = serde_json::from_str(&json).unwrap();
    assert_eq!(ae, restored);
}
