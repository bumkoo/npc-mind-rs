//! Application Service (MindService) 통합 테스트

mod common;

use npc_mind::application::dto::*;
use npc_mind::application::mind_service::MindService;
use npc_mind::domain::relationship::Relationship;

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
        situation: SituationInput {
            description: "교룡이 불의를 보고 참지 못하고 도와주는 장면".to_string(),
            event: None,
            action: Some(ActionInput {
                description: "백성을 도와줌".to_string(),
                agent_id: Some("gyo_ryong".to_string()),
                praiseworthiness: 0.7,
            }),
            object: None,
        },
    };

    let res = service.appraise(req, || {}, || vec![]).expect("Appraisal failed");
    
    // 무백은 정의로우므로 Admiration(감탄)이 발생해야 함
    assert!(res.emotions.iter().any(|e| e.emotion_type == "Admiration"));
    assert!(res.mood > 0.0);
    assert!(!res.prompt.is_empty());

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

    let res2 = service.apply_stimulus(stim_req).expect("Stimulus failed");
    assert!(res2.mood > res.mood); // 기분이 더 좋아져야 함

    // 3. 관계 갱신 (After Dialogue)
    let after_req = AfterDialogueRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        praiseworthiness: Some(0.5), // 좋은 행동으로 마무리
        significance: None,
    };

    let after_res = service.after_dialogue(after_req).expect("After dialogue failed");
    
    // 관계 점수가 상승했는지 확인 (closeness가 0.0에서 시작했으므로 양수여야 함)
    assert!(after_res.after.closeness > after_res.before.closeness);
    assert!(after_res.after.trust > after_res.before.trust);
}

#[test]
fn test_mind_service_errors() {
    let repo = MockRepository::new();
    let mut service = MindService::new(repo);

    let req = AppraiseRequest {
        npc_id: "non_existent".to_string(),
        partner_id: "any".to_string(),
        situation: SituationInput {
            description: "test".to_string(),
            event: None,
            action: None,
            object: None,
        },
    };

    let res = service.appraise(req, || {}, || vec![]);
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

    let domain = input.to_domain(&repo, "me", "partner").expect("Transformation failed");

    assert_eq!(domain.description, "test");
    let ev = domain.event.unwrap();
    assert_eq!(ev.description, "ev");
    assert_eq!(ev.desirability_for_self, 0.5);
    
    // Prospect 매핑 확인
    match ev.prospect {
        Some(npc_mind::domain::emotion::Prospect::Confirmation(npc_mind::domain::emotion::ProspectResult::HopeFulfilled)) => {},
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

    let res = bad_input.to_domain(&repo, "me", "partner");
    assert!(matches!(res, Err(npc_mind::application::mind_service::MindServiceError::RelationshipNotFound(_, _))));
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
        situation: SituationInput {
            description: "좋은 소식".to_string(),
            event: Some(EventInput {
                description: "좋은 일".to_string(),
                desirability_for_self: 0.6,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
        },
    };

    service.appraise(req, || {}, || vec![]).expect("appraise failed");

    // after_beat — 관계 갱신하되 감정 유지
    let beat_req = AfterDialogueRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        praiseworthiness: Some(0.5),
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
    assert!(guide_res.is_ok(), "after_beat 후 감정 상태 존재 → 가이드 생성 성공");
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
        situation: SituationInput {
            description: "좋은 소식".to_string(),
            event: Some(EventInput {
                description: "좋은 일".to_string(),
                desirability_for_self: 0.6,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
        },
    };
    service.appraise(req, || {}, || vec![]).expect("appraise failed");

    // after_dialogue — 관계 갱신 + 감정 초기화
    let dialogue_req = AfterDialogueRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        praiseworthiness: Some(0.5),
        significance: None,
    };
    service.after_dialogue(dialogue_req).expect("after_dialogue failed");

    // 감정 상태가 없어야 함 → 가이드 생성 실패
    let guide_req = GuideRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation_description: None,
    };
    let guide_res = service.generate_guide(guide_req);
    assert!(guide_res.is_err(), "after_dialogue 후 감정 상태 없음 → 가이드 생성 실패");
}
