use npc_mind::domain::emotion::*;

#[test]
fn test_scene_initial_focus() {
    let focuses = vec![
        SceneFocus {
            id: "start".into(),
            description: "시작".into(),
            trigger: FocusTrigger::Initial,
            event: None,
            action: None,
            object: None,
        },
        SceneFocus {
            id: "next".into(),
            description: "다음".into(),
            trigger: FocusTrigger::Conditions(vec![]),
            event: None,
            action: None,
            object: None,
        },
    ];

    let scene = Scene::new("npc".into(), "partner".into(), focuses);

    assert_eq!(scene.npc_id(), "npc");
    assert_eq!(scene.partner_id(), "partner");
    assert_eq!(scene.initial_focus().unwrap().id, "start");
    assert!(scene.active_focus_id().is_none());
}

#[test]
fn test_scene_check_trigger() {
    let focuses = vec![SceneFocus {
        id: "angry_focus".into(),
        description: "화남".into(),
        trigger: FocusTrigger::Conditions(vec![vec![EmotionCondition {
            emotion: EmotionType::Anger,
            threshold: ConditionThreshold::Above(0.5),
        }]]),
        event: None,
        action: None,
        object: None,
    }];

    let mut scene = Scene::new("npc".into(), "partner".into(), focuses);
    let mut state = EmotionState::new();

    // 조건 미충족
    assert!(scene.check_trigger(&state).is_none());

    // 조건 충족 (Anger > 0.5)
    state.add(Emotion::new(EmotionType::Anger, 0.7));
    let triggered = scene.check_trigger(&state).expect("트리거되어야 함");
    assert_eq!(triggered.id, "angry_focus");

    // 활성 포커스 설정
    scene.set_active_focus(triggered.id.clone());
    assert_eq!(scene.active_focus_id().unwrap(), "angry_focus");
}

#[test]
fn test_scene_trigger_complex_conditions() {
    // OR [ AND[Joy > 0.5, Pride > 0.3], AND[Anger > 0.8] ]
    let trigger = FocusTrigger::Conditions(vec![
        vec![
            EmotionCondition {
                emotion: EmotionType::Joy,
                threshold: ConditionThreshold::Above(0.5),
            },
            EmotionCondition {
                emotion: EmotionType::Pride,
                threshold: ConditionThreshold::Above(0.3),
            },
        ],
        vec![EmotionCondition {
            emotion: EmotionType::Anger,
            threshold: ConditionThreshold::Above(0.8),
        }],
    ]);

    let focus = SceneFocus {
        id: "complex".into(),
        description: "복합".into(),
        trigger,
        event: None,
        action: None,
        object: None,
    };

    let scene = Scene::new("npc".into(), "partner".into(), vec![focus]);
    let mut state = EmotionState::new();

    // 1. 아무것도 없음 -> false
    assert!(scene.check_trigger(&state).is_none());

    // 2. Joy만 충족 -> false (AND 조건)
    state.add(Emotion::new(EmotionType::Joy, 0.6));
    assert!(scene.check_trigger(&state).is_none());

    // 3. Joy + Pride 충족 -> true (첫 번째 AND 그룹)
    state.add(Emotion::new(EmotionType::Pride, 0.4));
    assert!(scene.check_trigger(&state).is_some());

    // 4. Anger만 매우 높음 -> true (두 번째 AND 그룹, OR 관계)
    let mut state2 = EmotionState::new();
    state2.add(Emotion::new(EmotionType::Anger, 0.9));
    assert!(scene.check_trigger(&state2).is_some());
}
