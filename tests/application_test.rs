//! Application Service (MindService) 통합 테스트

mod common;

use npc_mind::application::dto::*;
use npc_mind::application::mind_service::MindService;
use npc_mind::application::situation_service::SituationService;
use npc_mind::domain::relationship::Relationship;
use npc_mind::{EmotionStore, SceneStore, NpcWorld};

use common::*;

#[test]
fn test_mind_service_full_flow() {
    let mut repo = MockRepository::new();
    let mu_baek = make_무백();
    let gyo_ryong = make_교룡();

    repo.add_npc(mu_baek.clone());
    repo.add_npc(gyo_ryong.clone());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let mut service = MindService::new(repo);

    // 1. 상황 평가 (Appraise)
    // 무백이 교룡의 정당한 행동(칭찬할 만한 일)을 목격함
    let req = AppraiseRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation: Some(SituationInput {
            description: "교룡이 불의를 보고 참지 못하고 도와주는 장면".to_string(),
            event: None,
            action: Some(ActionInput {
                description: "백성을 도와줌".to_string(),
                agent_id: Some("gyo_ryong".to_string()),
                praiseworthiness: 0.7,
            }),
            object: None,
        }),
    };

    let res = service
        .appraise(req, || {}, Vec::new)
        .expect("Appraisal failed");

    // 무백은 정의로우므로 Admiration(감탄)이 발생해야 함
    assert!(res.emotions.iter().any(|e| e.emotion_type == "Admiration"));
    assert!(res.mood > 0.0);
    assert!(!res.guide.npc_name.is_empty());

    // 2. 자극 적용 (Stimulus)
    // 교룡이 겸손하게 대답함 (Pleasure 자극)
    let stim_req = StimulusRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation_description: Some("교룡의 겸손한 태도".to_string()),
        pleasure: 0.5,
        arousal: 0.2,
        dominance: 0.0,
    };

    let res2 = service
        .apply_stimulus(stim_req, || {}, Vec::new)
        .expect("Stimulus failed");
    assert!(res2.mood > res.mood); // 기분이 더 좋아져야 함

    // 3. 관계 갱신 (After Dialogue)
    let after_req = AfterDialogueRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        significance: None,
    };

    let after_res = service
        .after_dialogue(after_req)
        .expect("After dialogue failed");

    // closeness가 상승했는지 확인 (0.0에서 시작, 긍정 대화 후 양수여야 함)
    assert!(after_res.after.closeness > after_res.before.closeness);
    // trust는 after_dialogue에서 변경되지 않음 (closeness만 갱신)
    assert!((after_res.after.trust - after_res.before.trust).abs() < 0.001);
}

#[test]
fn test_mind_service_errors() {
    let repo = MockRepository::new();
    let mut service = MindService::new(repo);

    let req = AppraiseRequest {
        npc_id: "non_existent".to_string(),
        partner_id: "any".to_string(),
        situation: Some(SituationInput {
            description: "test".to_string(),
            event: None,
            action: None,
            object: None,
        }),
    };

    let res = service.appraise(req, || {}, Vec::new);
    assert!(res.is_err());
}

#[test]
fn test_dto_transformation_to_domain() {
    let mut repo = MockRepository::new();
    repo.add_relationship(Relationship::neutral("me", "target"));

    // 1. 정상 변환 테스트
    let input = SituationInput {
        description: "test".to_string(),
        event: Some(EventInput {
            description: "ev".to_string(),
            desirability_for_self: 0.5,
            other: Some(EventOtherInput {
                target_id: "target".to_string(),
                desirability: -0.3,
            }),
            prospect: Some("hope_fulfilled".into()),
        }),
        action: None,
        object: None,
    };

    let rel = repo.get_relationship("me", "target");
    let domain = input
        .to_domain(rel.as_ref().map(|r| r.modifiers()), None, None, "me")
        .expect("Transformation failed");

    assert_eq!(domain.description, "test");
    let ev = domain.event.unwrap();
    assert_eq!(ev.description, "ev");
    assert_eq!(ev.desirability_for_self, 0.5);

    // Prospect 매핑 확인
    match ev.prospect {
        Some(npc_mind::domain::emotion::Prospect::Confirmation(
            npc_mind::domain::emotion::ProspectResult::HopeFulfilled,
        )) => {}
        _ => panic!("Prospect mapping failed"),
    }

    // 타인 운 확인
    let other = ev.desirability_for_other.unwrap();
    assert_eq!(other.target_id, "target");
    assert_eq!(other.desirability, -0.3);

    // 2. 관계 없음 에러 테스트
    let bad_input = SituationInput {
        description: "test".to_string(),
        event: Some(EventInput {
            description: "ev".to_string(),
            desirability_for_self: 0.5,
            other: Some(EventOtherInput {
                target_id: "unknown".to_string(),
                desirability: 0.0,
            }),
            prospect: None,
        }),
        action: None,
        object: None,
    };

    let res = bad_input.to_domain(None, None, None, "me");
    assert!(matches!(
        res,
        Err(npc_mind::application::mind_service::MindServiceError::InvalidSituation(_))
    ));
}

// ===========================================================================
// after_beat vs after_dialogue 검증
// ===========================================================================

#[test]
fn after_beat_감정_유지() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let mut service = MindService::new(repo);

    // appraise로 감정 생성
    let req = AppraiseRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation: Some(SituationInput {
            description: "좋은 소식".to_string(),
            event: Some(EventInput {
                description: "좋은 일".to_string(),
                desirability_for_self: 0.6,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
        }),
    };

    service
        .appraise(req, || {}, Vec::new)
        .expect("appraise failed");

    // after_beat — 관계 갱신하되 감정 유지
    let beat_req = AfterDialogueRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        significance: None,
    };
    service.after_beat(beat_req).expect("after_beat failed");

    // 감정 상태가 여전히 존재해야 함
    let guide_req = GuideRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation_description: None,
    };
    let guide_res = service.generate_guide(guide_req);
    assert!(
        guide_res.is_ok(),
        "after_beat 후 감정 상태 존재 → 가이드 생성 성공"
    );
}

#[test]
fn after_dialogue_감정_초기화() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let mut service = MindService::new(repo);

    // appraise로 감정 생성
    let req = AppraiseRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation: Some(SituationInput {
            description: "좋은 소식".to_string(),
            event: Some(EventInput {
                description: "좋은 일".to_string(),
                desirability_for_self: 0.6,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
        }),
    };
    service
        .appraise(req, || {}, Vec::new)
        .expect("appraise failed");

    // after_dialogue — 관계 갱신 + 감정 초기화
    let dialogue_req = AfterDialogueRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        significance: None,
    };
    service
        .after_dialogue(dialogue_req)
        .expect("after_dialogue failed");

    // 감정 상태가 없어야 함 → 가이드 생성 실패
    let guide_req = GuideRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation_description: None,
    };
    let guide_res = service.generate_guide(guide_req);
    assert!(
        guide_res.is_err(),
        "after_dialogue 후 감정 상태 없음 → 가이드 생성 실패"
    );
}

#[test]
fn test_scene_persistence_and_clear() {
    let mut ctx = TestContext::new();
    let mut service = ctx.service();

    let focuses = vec![SceneFocusInput {
        id: "start".into(),
        description: "시작".into(),
        trigger: None,
        event: Some(EventInput {
            description: "초기 상황".into(),
            desirability_for_self: 0.1,
            other: None,
            prospect: None,
        }),
        action: None,
        object: None,
    }];

    let req = SceneRequest {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        description: "장면".into(),
        significance: None,
        focuses,
    };

    // 1. Scene 시작
    let result = service.start_scene(req, || {}, Vec::new).unwrap();
    assert_eq!(result.focus_count, 1);
    assert!(result.active_focus_id.is_some());

    // 2. 저장소에 Scene이 존재하는지 확인
    {
        let scene = service
            .repository()
            .get_scene()
            .expect("Scene이 저장되어야 함");
        assert_eq!(scene.npc_id(), "mu_baek");
        assert_eq!(scene.active_focus_id(), Some("start"));
    }

    // 3. Dialogue 종료 후 Scene이 삭제되는지 확인
    let after_req = AfterDialogueRequest {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        significance: Some(0.5),
    };
    service.after_dialogue(after_req).unwrap();

    assert!(
        service.repository().get_scene().is_none(),
        "Dialogue 종료 후 Scene은 삭제되어야 함"
    );
}

// ===========================================================================
// Beat 전환 및 감정 병합 정밀 검증 시나리오
// ===========================================================================

#[test]
fn test_beat_transition_and_emotion_merging() {
    let mut ctx = TestContext::new();
    let mut service = ctx.service();

    // 1. Scene 설정 (교룡 대상)
    let focuses = vec![
        SceneFocusInput {
            id: "calm".into(),
            description: "평온한 대화".into(),
            trigger: None,
            event: Some(EventInput {
                description: "초기 상황".into(),
                desirability_for_self: 0.05, // 기쁨을 낮게 설정 (쉽게 사라지도록)
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
        },
        SceneFocusInput {
            id: "angry".into(),
            description: "갑작스러운 갈등".into(),
            trigger: Some(vec![vec![ConditionInput {
                emotion: "Joy".into(),
                absent: Some(true), // 기쁨이 완전히 사라지면 전환
                below: None,
                above: None,
            }]]),
            event: Some(EventInput {
                description: "모욕을 당함".into(),
                desirability_for_self: -0.6,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
        },
    ];

    // 2. Scene 시작 (교룡으로 시작)
    let start_req = SceneRequest {
        npc_id: "gyo_ryong".into(),
        partner_id: "mu_baek".into(),
        description: "테스트 장면".into(),
        significance: None,
        focuses,
    };
    service.start_scene(start_req, || {}, Vec::new).unwrap();

    // 3. 자극 적용: 교룡에게 강한 불쾌 자극을 주어 기쁨을 제거함
    let stim_req = StimulusRequest {
        npc_id: "gyo_ryong".into(),
        partner_id: "mu_baek".into(),
        situation_description: Some("무백의 원칙적인 잔소리".to_string()),
        pleasure: -1.0,
        arousal: -1.0,
        dominance: -1.0,
    };

    let stim_res = service.apply_stimulus(stim_req, || {}, Vec::new).unwrap();

    // 4. 검증: 교룡은 민감하여 기쁨이 바로 사라지고 전환되어야 함
    assert!(
        stim_res.beat_changed,
        "교룡은 기쁨이 사라지는 즉시 Beat가 전환되어야 함"
    );
    assert_eq!(stim_res.active_focus_id, Some("angry".to_string()));

    // 5. 핵심 검증: 감정이 병합되었는가?
    let final_state = service.repository().get_emotion_state("gyo_ryong").unwrap();

    // - "angry" 비트의 결과인 Distress 또는 Anger가 존재해야 함
    assert!(
        !final_state.emotions().is_empty(),
        "병합 후 감정이 존재해야 함"
    );

    // - 이전 비트의 데이터가 유실되지 않고 병합 로직이 정상 호출되었음을 확인
    // (이전 기쁨은 사라졌지만, 병합된 상태 자체는 유효해야 함)
    assert!(
        final_state.overall_valence() < 0.0,
        "병합 후 기분은 나빠져야 함"
    );
}

// ===========================================================================
// 리팩토링 검증: parse_trigger / convert_focuses / resolve_focus_context
// ===========================================================================

#[test]
fn test_scene_focus_input_trigger_none은_initial() {
    let input = SceneFocusInput {
        id: "f1".into(),
        description: "초기 포커스".into(),
        trigger: None,
        event: Some(EventInput {
            description: "이벤트".into(),
            desirability_for_self: 0.3,
            other: None,
            prospect: None,
        }),
        action: None,
        object: None,
    };

    let focus = input.to_domain(None, None, None, "npc").unwrap();
    assert_eq!(focus.id, "f1");
    assert!(focus.event.is_some());
    // trigger: None → FocusTrigger::Initial
    assert!(matches!(
        focus.trigger,
        npc_mind::domain::emotion::FocusTrigger::Initial
    ));
}

#[test]
fn test_scene_focus_input_trigger_conditions_변환() {
    let input = SceneFocusInput {
        id: "angry_beat".into(),
        description: "분노 전환".into(),
        trigger: Some(vec![
            // OR 그룹 1: Distress > 0.5 AND Joy < 0.2
            vec![
                ConditionInput { emotion: "Distress".into(), below: None, above: Some(0.5), absent: None },
                ConditionInput { emotion: "Joy".into(), below: Some(0.2), above: None, absent: None },
            ],
            // OR 그룹 2: Anger absent
            vec![
                ConditionInput { emotion: "Anger".into(), below: None, above: None, absent: Some(true) },
            ],
        ]),
        event: None,
        action: None,
        object: None,
    };

    let focus = input.to_domain(None, None, None, "npc").unwrap();
    match &focus.trigger {
        npc_mind::domain::emotion::FocusTrigger::Conditions(or_groups) => {
            assert_eq!(or_groups.len(), 2, "OR 그룹 2개");
            assert_eq!(or_groups[0].len(), 2, "첫 번째 OR 그룹에 AND 조건 2개");
            assert_eq!(or_groups[1].len(), 1, "두 번째 OR 그룹에 AND 조건 1개");
        }
        _ => panic!("Conditions 트리거여야 함"),
    }
}

#[test]
fn test_condition_input_잘못된_감정_문자열() {
    let input = SceneFocusInput {
        id: "bad".into(),
        description: "에러".into(),
        trigger: Some(vec![vec![
            ConditionInput { emotion: "InvalidEmotion".into(), below: Some(0.5), above: None, absent: None },
        ]]),
        event: None,
        action: None,
        object: None,
    };

    let result = input.to_domain(None, None, None, "npc");
    assert!(result.is_err(), "존재하지 않는 감정 유형은 에러");
}

#[test]
fn test_condition_input_조건_누락() {
    let input = SceneFocusInput {
        id: "bad".into(),
        description: "에러".into(),
        trigger: Some(vec![vec![
            ConditionInput { emotion: "Joy".into(), below: None, above: None, absent: None },
        ]]),
        event: None,
        action: None,
        object: None,
    };

    let result = input.to_domain(None, None, None, "npc");
    assert!(result.is_err(), "below/above/absent 중 하나 필요");
}

#[test]
fn test_scene_focus_input_3축_동시_변환() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));
    repo.add_object("sword", "명검 청룡");

    let input = SceneFocusInput {
        id: "complex".into(),
        description: "복합 포커스".into(),
        trigger: None,
        event: Some(EventInput {
            description: "사건 발생".into(),
            desirability_for_self: 0.6,
            other: Some(EventOtherInput {
                target_id: "gyo_ryong".into(),
                desirability: -0.4,
            }),
            prospect: None,
        }),
        action: Some(ActionInput {
            description: "행위".into(),
            agent_id: Some("gyo_ryong".into()),
            praiseworthiness: -0.5,
        }),
        object: Some(ObjectInput {
            target_id: "sword".into(),
            appealingness: 0.8,
        }),
    };

    let rel = repo.get_relationship("mu_baek", "gyo_ryong");
    let focus = input
        .to_domain(
            rel.as_ref().map(|r| r.modifiers()),
            rel.as_ref().map(|r| r.modifiers()),
            Some("명검 청룡".into()),
            "mu_baek",
        )
        .unwrap();

    assert!(focus.event.is_some(), "event 변환됨");
    assert!(focus.action.is_some(), "action 변환됨");
    assert!(focus.object.is_some(), "object 변환됨");
    assert_eq!(focus.object.unwrap().target_description, "명검 청룡");
}

#[test]
fn test_resolve_focus_context_관계_있음() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));
    repo.add_object("sword", "명검 청룡");

    let input = SituationInput {
        description: "테스트".into(),
        event: Some(EventInput {
            description: "ev".into(),
            desirability_for_self: 0.5,
            other: Some(EventOtherInput {
                target_id: "gyo_ryong".into(),
                desirability: 0.3,
            }),
            prospect: None,
        }),
        action: None,
        object: Some(ObjectInput {
            target_id: "sword".into(),
            appealingness: 0.5,
        }),
    };

    let ctx = SituationService::resolve_focus_context(&repo, &input, "mu_baek", "gyo_ryong");
    assert!(ctx.event_other_modifiers.is_some(), "관계 존재 → modifier 조회 성공");
    assert!(ctx.object_description.is_some(), "오브젝트 설명 조회 성공");
    assert_eq!(ctx.object_description.unwrap(), "명검 청룡");
}

#[test]
fn test_resolve_focus_context_관계_없음() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());

    let input = SituationInput {
        description: "테스트".into(),
        event: Some(EventInput {
            description: "ev".into(),
            desirability_for_self: 0.5,
            other: Some(EventOtherInput {
                target_id: "unknown".into(),
                desirability: 0.3,
            }),
            prospect: None,
        }),
        action: None,
        object: None,
    };

    let ctx = SituationService::resolve_focus_context(&repo, &input, "mu_baek", "partner");
    assert!(ctx.event_other_modifiers.is_none(), "관계 없음 → None");
}

#[test]
fn test_resolve_focus_context_action_agent가_npc_자신이면_무시() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    // agent_id가 npc_id와 같으면 modifier 조회하지 않음
    let input = SituationInput {
        description: "테스트".into(),
        event: None,
        action: Some(ActionInput {
            description: "행위".into(),
            agent_id: Some("mu_baek".into()),  // NPC 자신
            praiseworthiness: 0.5,
        }),
        object: None,
    };

    let ctx = SituationService::resolve_focus_context(&repo, &input, "mu_baek", "gyo_ryong");
    assert!(ctx.action_agent_modifiers.is_none(), "자기 자신이면 modifier 조회 안 함");
}

#[test]
fn test_resolve_focus_context_action_agent가_partner이면_무시() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    // agent_id가 partner_id와 같으면 modifier 조회하지 않음
    let input = SituationInput {
        description: "테스트".into(),
        event: None,
        action: Some(ActionInput {
            description: "행위".into(),
            agent_id: Some("gyo_ryong".into()),  // partner
            praiseworthiness: 0.5,
        }),
        object: None,
    };

    let ctx = SituationService::resolve_focus_context(&repo, &input, "mu_baek", "gyo_ryong");
    assert!(ctx.action_agent_modifiers.is_none(), "partner이면 modifier 조회 안 함");
}

#[test]
fn test_resolve_focus_context_action_제3자_agent() {
    let mut repo = MockRepository::new();
    let 수련 = make_수련();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_npc(수련);
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));
    repo.add_relationship(Relationship::neutral("mu_baek", "shu_lien"));

    // agent_id가 제3자이면 modifier 조회
    let input = SituationInput {
        description: "테스트".into(),
        event: None,
        action: Some(ActionInput {
            description: "수련의 행위".into(),
            agent_id: Some("shu_lien".into()),  // 제3자
            praiseworthiness: 0.7,
        }),
        object: None,
    };

    let ctx = SituationService::resolve_focus_context(&repo, &input, "mu_baek", "gyo_ryong");
    assert!(ctx.action_agent_modifiers.is_some(), "제3자 agent → modifier 조회 성공");
}
