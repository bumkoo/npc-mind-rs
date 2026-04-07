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
            test_script: vec![],
        },
        SceneFocus {
            id: "next".into(),
            description: "다음".into(),
            trigger: FocusTrigger::Conditions(vec![]),
            event: None,
            action: None,
            object: None,
            test_script: vec![],
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
        test_script: vec![],
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
        test_script: vec![],
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

#[test]
fn test_scene_check_trigger_excludes_active_focus() {
    // State latching 검증:
    // 활성 Focus는 재전환 대상에서 제외되어야 한다.
    // 지배 감정이 임계값을 계속 상회해도 beat_changed 중복 발생 방지.
    let focuses = vec![
        SceneFocus {
            id: "distressed".into(),
            description: "고통 focus".into(),
            trigger: FocusTrigger::Conditions(vec![vec![EmotionCondition {
                emotion: EmotionType::Distress,
                threshold: ConditionThreshold::Above(0.7),
            }]]),
            event: None,
            action: None,
            object: None,
            test_script: vec![],
        },
        SceneFocus {
            id: "calm".into(),
            description: "평온 focus".into(),
            trigger: FocusTrigger::Conditions(vec![vec![EmotionCondition {
                emotion: EmotionType::Joy,
                threshold: ConditionThreshold::Above(0.8),
            }]]),
            event: None,
            action: None,
            object: None,
            test_script: vec![],
        },
    ];

    let mut scene = Scene::new("npc".into(), "partner".into(), focuses);
    let mut state = EmotionState::new();
    state.add(Emotion::new(EmotionType::Distress, 0.95));

    // 1. 초기 상태: Distress 0.95 → distressed focus 트리거
    let triggered = scene.check_trigger(&state).expect("트리거되어야 함");
    assert_eq!(triggered.id, "distressed");

    // 2. distressed를 활성화
    scene.set_active_focus("distressed".into());

    // 3. Distress가 여전히 높아도 (0.97) distressed는 재전환 대상에서 제외됨
    state.add(Emotion::new(EmotionType::Distress, 0.97));
    assert!(
        scene.check_trigger(&state).is_none(),
        "활성 focus는 재전환되지 않아야 함"
    );

    // 4. 다른 focus의 trigger가 충족되면 전환 가능
    state.add(Emotion::new(EmotionType::Joy, 0.9));
    let next = scene
        .check_trigger(&state)
        .expect("다른 focus로 전환되어야 함");
    assert_eq!(next.id, "calm");
}


#[test]
fn test_reset_to_initial_focus_clears_stale_state() {
    // Bug regression: dialogue_start 시 stale active_focus_id 초기화 검증
    //
    // 이전 세션에서 apply_stimulus가 Beat 전환을 일으켜 active_focus_id가
    // "escalated"로 바뀐 상태에서 새 대화 세션을 시작하면,
    // 첫 턴에 다시 Beat 전환이 재발생하는 버그를 방지한다.
    let focuses = vec![
        SceneFocus {
            id: "initial_calm".into(),
            description: "초기 상황".into(),
            trigger: FocusTrigger::Initial,
            event: None,
            action: None,
            object: None,
            test_script: vec![],
        },
        SceneFocus {
            id: "escalated".into(),
            description: "고조된 상황".into(),
            trigger: FocusTrigger::Conditions(vec![vec![EmotionCondition {
                emotion: EmotionType::Anger,
                threshold: ConditionThreshold::Above(0.7),
            }]]),
            event: None,
            action: None,
            object: None,
            test_script: vec![],
        },
    ];

    let mut scene = Scene::new("npc".into(), "partner".into(), focuses);

    // 1. 이전 세션에서 Beat 전환이 발생했다고 가정 → active = "escalated"
    scene.set_active_focus("escalated".into());
    assert_eq!(scene.active_focus_id(), Some("escalated"));

    // 2. 새 대화 세션 시작 → reset_to_initial_focus 호출
    let reset_id = scene.reset_to_initial_focus();
    assert_eq!(reset_id.as_deref(), Some("initial_calm"));
    assert_eq!(scene.active_focus_id(), Some("initial_calm"));

    // 3. 첫 턴에 Anger가 여전히 높아도 "initial_calm"은 latching되어 재전환 안 함
    let mut state = EmotionState::new();
    state.add(Emotion::new(EmotionType::Anger, 0.8));
    let next = scene.check_trigger(&state).expect("escalated로 전환되어야 함");
    assert_eq!(next.id, "escalated");
    // active는 initial_calm이므로 escalated는 여전히 전환 대상이 맞음
    // (stale state였다면 escalated가 active라서 전환 안 됐을 것)
}

#[test]
fn test_reset_to_initial_focus_no_initial() {
    // Initial Focus가 없으면 active_focus_id가 None으로 초기화됨
    let focuses = vec![SceneFocus {
        id: "only_conditional".into(),
        description: "조건부 focus".into(),
        trigger: FocusTrigger::Conditions(vec![vec![EmotionCondition {
            emotion: EmotionType::Joy,
            threshold: ConditionThreshold::Above(0.5),
        }]]),
        event: None,
        action: None,
        object: None,
        test_script: vec![],
    }];

    let mut scene = Scene::new("npc".into(), "partner".into(), focuses);
    scene.set_active_focus("only_conditional".into());

    let reset_id = scene.reset_to_initial_focus();
    assert!(reset_id.is_none());
    assert!(scene.active_focus_id().is_none());
}
