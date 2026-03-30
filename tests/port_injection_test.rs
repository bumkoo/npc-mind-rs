//! 포트 주입 및 Scene/Beat 통합 테스트
//!
//! - MindService에 커스텀 Appraiser/StimulusProcessor 주입
//! - start_scene: Focus 등록 + 초기 appraise
//! - apply_stimulus: Beat 전환 통합
//! - scene_info: Scene 상태 조회
//! - load_scene_focuses: 시나리오 로드 시 scene 복원

mod common;

use npc_mind::application::dto::*;
use npc_mind::application::mind_service::MindService;
use npc_mind::EmotionStore;
use npc_mind::domain::emotion::*;
use npc_mind::domain::pad::Pad;
use npc_mind::domain::relationship::Relationship;
use npc_mind::ports::{Appraiser, AppraisalWeights, StimulusProcessor, StimulusWeights};

use common::*;

// ===========================================================================
// 커스텀 엔진 주입 테스트
// ===========================================================================

/// 항상 Joy(0.9)만 반환하는 Mock Appraiser
struct AlwaysJoyAppraiser;

impl Appraiser for AlwaysJoyAppraiser {
    fn appraise<P: AppraisalWeights>(
        &self,
        _personality: &P,
        _situation: &Situation,
        _dialogue_modifiers: &RelationshipModifiers,
    ) -> EmotionState {
        let mut state = EmotionState::new();
        state.add(Emotion::with_context(EmotionType::Joy, 0.9, "mock"));
        state
    }
}

/// 감정 변동 없이 그대로 반환하는 Mock StimulusProcessor
struct NoOpStimulusProcessor;

impl StimulusProcessor for NoOpStimulusProcessor {
    fn apply_stimulus<P: StimulusWeights>(
        &self,
        _personality: &P,
        current_state: &EmotionState,
        _stimulus: &Pad,
    ) -> EmotionState {
        current_state.clone()
    }
}

#[test]
fn 커스텀_appraiser_주입() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let mut service = MindService::with_engines(repo, AlwaysJoyAppraiser, StimulusEngine);

    let req = AppraiseRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation: SituationInput {
            description: "아무 상황".to_string(),
            event: Some(EventInput {
                description: "나쁜 일".to_string(),
                desirability_for_self: -0.8,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
        },
    };

    let result = service.appraise(req, || {}, || vec![]).unwrap();

    // 나쁜 상황이지만 AlwaysJoyAppraiser이므로 Joy만 존재
    assert!(result.emotions.iter().any(|e| e.emotion_type == "Joy"));
    assert_eq!(result.emotions.len(), 1);
    assert!(result.mood > 0.0);
}

#[test]
fn 커스텀_stimulus_processor_주입() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let mut service = MindService::with_engines(repo, AppraisalEngine, NoOpStimulusProcessor);

    // appraise로 감정 생성
    let req = AppraiseRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation: SituationInput {
            description: "좋은 일".to_string(),
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

    let appraise_result = service.appraise(req, || {}, || vec![]).unwrap();
    let before_mood = appraise_result.mood;

    // 부정적 자극 적용 — NoOp이므로 변동 없어야 함
    let stim_req = StimulusRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation_description: None,
        pleasure: -0.9,
        arousal: 0.8,
        dominance: 0.5,
    };

    let stim_result = service.apply_stimulus(stim_req, || {}, || vec![]).unwrap();

    // NoOp이므로 mood 변동 없음
    assert!((stim_result.mood - before_mood).abs() < 0.001,
        "NoOp processor: mood 변동 없어야 함 (before={}, after={})", before_mood, stim_result.mood);
}

#[test]
fn 기본_엔진_사용시_기존과_동일() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    // MindService::new는 기본 엔진 사용
    let mut service = MindService::new(repo);

    let req = AppraiseRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation: SituationInput {
            description: "배신".to_string(),
            event: Some(EventInput {
                description: "배신 사건".to_string(),
                desirability_for_self: -0.6,
                other: None,
                prospect: None,
            }),
            action: Some(ActionInput {
                description: "배신 행동".to_string(),
                agent_id: Some("gyo_ryong".to_string()),
                praiseworthiness: -0.7,
            }),
            object: None,
        },
    };

    let result = service.appraise(req, || {}, || vec![]).unwrap();

    // 기존 엔진이므로 부정적 감정 발생
    assert!(result.mood < 0.0);
    assert!(result.emotions.iter().any(|e| e.emotion_type == "Distress" || e.emotion_type == "Anger"));
}

// ===========================================================================
// start_scene 테스트
// ===========================================================================

fn scene_req_with_initial() -> SceneRequest {
    SceneRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        description: "테스트 장면".to_string(),
        focuses: vec![
            SceneFocusInput {
                id: "initial_focus".to_string(),
                description: "초기 상황".to_string(),
                trigger: None, // Initial
                event: Some(EventInput {
                    description: "좋은 소식".to_string(),
                    desirability_for_self: 0.5,
                    other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
            },
            SceneFocusInput {
                id: "anger_focus".to_string(),
                description: "분노 전환".to_string(),
                trigger: Some(vec![vec![ConditionInput {
                    emotion: "Anger".to_string(),
                    above: Some(0.3),
                    below: None,
                    absent: None,
                }]]),
                event: Some(EventInput {
                    description: "배신 발각".to_string(),
                    desirability_for_self: -0.8,
                    other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
            },
        ],
    }
}

#[test]
fn start_scene_초기_focus_appraise() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let mut service = MindService::new(repo);
    let result = service.start_scene(scene_req_with_initial(), || {}, || vec![]).unwrap();

    assert_eq!(result.focus_count, 2);
    assert!(result.initial_appraise.is_some());
    assert_eq!(result.active_focus_id, Some("initial_focus".to_string()));

    // 초기 Focus는 긍정적 → Joy 존재
    let appraise = result.initial_appraise.unwrap();
    assert!(appraise.emotions.iter().any(|e| e.emotion_type == "Joy"));
    assert!(appraise.mood > 0.0);
}

#[test]
fn start_scene_focus가_없으면_appraise_없음() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let mut service = MindService::new(repo);

    // 모든 Focus에 trigger 조건이 있음 (Initial 없음)
    let req = SceneRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        description: "장면".to_string(),
        focuses: vec![SceneFocusInput {
            id: "conditional".to_string(),
            description: "조건부".to_string(),
            trigger: Some(vec![vec![ConditionInput {
                emotion: "Anger".to_string(),
                above: Some(0.5),
                below: None,
                absent: None,
            }]]),
            event: Some(EventInput {
                description: "사건".to_string(),
                desirability_for_self: -0.5,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
        }],
    };

    let result = service.start_scene(req, || {}, || vec![]).unwrap();

    assert_eq!(result.focus_count, 1);
    assert!(result.initial_appraise.is_none());
    assert!(result.active_focus_id.is_none());
}

// ===========================================================================
// scene_info 테스트
// ===========================================================================

#[test]
fn scene_info_scene_없으면_빈_결과() {
    let repo = MockRepository::new();
    let service = MindService::new(repo);

    let info = service.scene_info();
    assert!(!info.has_scene);
    assert!(info.focuses.is_empty());
}

#[test]
fn scene_info_scene_등록_후_상태_조회() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let mut service = MindService::new(repo);
    service.start_scene(scene_req_with_initial(), || {}, || vec![]).unwrap();

    let info = service.scene_info();
    assert!(info.has_scene);
    assert_eq!(info.focuses.len(), 2);
    assert_eq!(info.npc_id, Some("mu_baek".to_string()));
    assert_eq!(info.partner_id, Some("gyo_ryong".to_string()));
    assert_eq!(info.active_focus_id, Some("initial_focus".to_string()));

    // 활성 Focus 확인
    let initial = info.focuses.iter().find(|f| f.id == "initial_focus").unwrap();
    assert!(initial.is_active);
    assert_eq!(initial.trigger_display, "initial");

    let anger = info.focuses.iter().find(|f| f.id == "anger_focus").unwrap();
    assert!(!anger.is_active);
    assert!(anger.trigger_display.contains("Anger"));
}

// ===========================================================================
// apply_stimulus + Beat 전환 통합 테스트
// ===========================================================================

#[test]
fn stimulus_scene_없으면_beat_전환_안됨() {
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
            description: "좋은 일".to_string(),
            event: Some(EventInput {
                description: "좋은 일".to_string(),
                desirability_for_self: 0.5,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
        },
    };
    service.appraise(req, || {}, || vec![]).unwrap();

    let stim = StimulusRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation_description: None,
        pleasure: -0.5,
        arousal: 0.5,
        dominance: 0.0,
    };
    let result = service.apply_stimulus(stim, || {}, || vec![]).unwrap();

    assert!(!result.beat_changed);
    assert!(result.active_focus_id.is_none());
}

#[test]
fn stimulus_beat_전환_trigger_충족() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_교룡()); // 교룡은 부정 자극에 강하게 반응
    repo.add_npc(make_무백());
    repo.add_relationship(Relationship::neutral("gyo_ryong", "mu_baek"));

    let mut service = MindService::new(repo);

    // Scene 등록 — 초기 Focus + Distress > 0.3 시 전환
    let scene_req = SceneRequest {
        npc_id: "gyo_ryong".to_string(),
        partner_id: "mu_baek".to_string(),
        description: "장면".to_string(),
        focuses: vec![
            SceneFocusInput {
                id: "initial".to_string(),
                description: "평화로운 시작".to_string(),
                trigger: None,
                event: Some(EventInput {
                    description: "좋은 소식".to_string(),
                    desirability_for_self: 0.3,
                    other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
            },
            SceneFocusInput {
                id: "distress_focus".to_string(),
                description: "고통 전환".to_string(),
                trigger: Some(vec![vec![ConditionInput {
                    emotion: "Distress".to_string(),
                    above: Some(0.1),
                    below: None,
                    absent: None,
                }]]),
                event: Some(EventInput {
                    description: "나쁜 소식".to_string(),
                    desirability_for_self: -0.7,
                    other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
            },
        ],
    };

    service.start_scene(scene_req, || {}, || vec![]).unwrap();

    // 강한 부정 자극 반복 → Distress 발생 → Beat 전환 예상
    // Joy가 있는 상태에서 부정 자극으로 Joy 감소 → Distress는 stimulus로 직접 생성 안 됨
    // 하지만 trigger 조건은 현재 감정 상태로 체크됨
    // 여러 번 부정 자극을 주면 Joy가 소멸할 수 있지만 Distress는 appraise에서만 생성
    // → Beat 전환 검증을 위해 AlwaysJoyAppraiser 대신 직접 감정을 설정

    // 수동으로 Distress 감정 설정하여 trigger 조건 충족시키기
    let mut state = service.repository().get_emotion_state("gyo_ryong").unwrap();
    state.add(Emotion::new(EmotionType::Distress, 0.5));
    service.repository_mut().save_emotion_state("gyo_ryong", state);

    // 중립 자극 (감정은 거의 변동 없지만 trigger 체크는 수행)
    let stim = StimulusRequest {
        npc_id: "gyo_ryong".to_string(),
        partner_id: "mu_baek".to_string(),
        situation_description: None,
        pleasure: 0.0,
        arousal: 0.0,
        dominance: 0.0,
    };
    let result = service.apply_stimulus(stim, || {}, || vec![]).unwrap();

    assert!(result.beat_changed, "Distress > 0.1 조건 충족 → Beat 전환");
    assert_eq!(result.active_focus_id, Some("distress_focus".to_string()));
}

#[test]
fn stimulus_beat_전환_후_active_focus_변경() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let mut service = MindService::new(repo);

    // Scene 등록 — Joy absent 조건
    let scene_req = SceneRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        description: "장면".to_string(),
        focuses: vec![
            SceneFocusInput {
                id: "happy".to_string(),
                description: "기쁨 상태".to_string(),
                trigger: None,
                event: Some(EventInput {
                    description: "좋은 소식".to_string(),
                    desirability_for_self: 0.5,
                    other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
            },
            SceneFocusInput {
                id: "joy_gone".to_string(),
                description: "기쁨이 사라진 상태".to_string(),
                trigger: Some(vec![vec![ConditionInput {
                    emotion: "Joy".to_string(),
                    below: None,
                    above: None,
                    absent: Some(true),
                }]]),
                event: Some(EventInput {
                    description: "공허함".to_string(),
                    desirability_for_self: -0.3,
                    other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
            },
        ],
    };

    service.start_scene(scene_req, || {}, || vec![]).unwrap();
    assert_eq!(service.scene_info().active_focus_id, Some("happy".to_string()));

    // Joy를 수동으로 제거하여 absent 조건 충족
    let mut state = service.repository().get_emotion_state("mu_baek").unwrap();
    state.remove(EmotionType::Joy);
    service.repository_mut().save_emotion_state("mu_baek", state);

    let stim = StimulusRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation_description: None,
        pleasure: 0.0,
        arousal: 0.0,
        dominance: 0.0,
    };
    let result = service.apply_stimulus(stim, || {}, || vec![]).unwrap();

    assert!(result.beat_changed);
    assert_eq!(result.active_focus_id, Some("joy_gone".to_string()));

    // scene_info도 업데이트 확인
    assert_eq!(service.scene_info().active_focus_id, Some("joy_gone".to_string()));
}

// ===========================================================================
// load_scene_focuses 테스트
// ===========================================================================

#[test]
fn load_scene_focuses_초기_appraise() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let mut service = MindService::new(repo);

    // SceneFocus 직접 생성
    let focuses = vec![SceneFocus {
        id: "loaded_focus".to_string(),
        description: "로드된 장면".to_string(),
        trigger: FocusTrigger::Initial,
        event: Some(EventFocus {
            description: "좋은 일".to_string(),
            desirability_for_self: 0.5,
            desirability_for_other: None,
            prospect: None,
        }),
        action: None,
        object: None,
    }];

    let result = service.load_scene_focuses(
        focuses,
        "mu_baek".to_string(),
        "gyo_ryong".to_string(),
    ).unwrap();

    assert!(result.is_some());
    let appraise = result.unwrap();
    assert!(appraise.mood > 0.0);

    // scene_info에서 확인
    let info = service.scene_info();
    assert!(info.has_scene);
    assert_eq!(info.active_focus_id, Some("loaded_focus".to_string()));
}

#[test]
fn load_scene_focuses_initial_없으면_appraise_없음() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let mut service = MindService::new(repo);

    let focuses = vec![SceneFocus {
        id: "conditional".to_string(),
        description: "조건부".to_string(),
        trigger: FocusTrigger::Conditions(vec![vec![EmotionCondition {
            emotion: EmotionType::Anger,
            threshold: ConditionThreshold::Above(0.5),
        }]]),
        event: Some(EventFocus {
            description: "사건".to_string(),
            desirability_for_self: -0.5,
            desirability_for_other: None,
            prospect: None,
        }),
        action: None,
        object: None,
    }];

    let result = service.load_scene_focuses(
        focuses,
        "mu_baek".to_string(),
        "gyo_ryong".to_string(),
    ).unwrap();

    assert!(result.is_none());
    assert!(service.scene_info().has_scene);
    assert!(service.scene_info().active_focus_id.is_none());
}

// ===========================================================================
// FormattedMindService + Scene 통합
// ===========================================================================

#[test]
fn formatted_service_start_scene() {
    use npc_mind::application::formatted_service::FormattedMindService;

    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let mut service = FormattedMindService::new(repo, "ko").unwrap();
    let response = service.start_scene(scene_req_with_initial(), || {}, || vec![]).unwrap();

    assert_eq!(response.focus_count, 2);
    assert!(response.initial_appraise.is_some());

    // 포맷팅된 prompt 포함 확인
    let appraise = response.initial_appraise.unwrap();
    assert!(appraise.prompt.contains("[NPC:"));
    assert!(appraise.prompt.contains("[성격]"));
}

#[test]
fn formatted_service_stimulus_beat_전환_포맷팅() {
    use npc_mind::application::formatted_service::FormattedMindService;

    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let mut service = FormattedMindService::new(repo, "ko").unwrap();

    // Scene 등록 + 초기 appraise
    let scene_req = SceneRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        description: "장면".to_string(),
        focuses: vec![
            SceneFocusInput {
                id: "start".to_string(),
                description: "시작".to_string(),
                trigger: None,
                event: Some(EventInput {
                    description: "좋은 일".to_string(),
                    desirability_for_self: 0.5,
                    other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
            },
            SceneFocusInput {
                id: "fear_focus".to_string(),
                description: "공포 전환".to_string(),
                trigger: Some(vec![vec![ConditionInput {
                    emotion: "Fear".to_string(),
                    above: Some(0.1),
                    below: None,
                    absent: None,
                }]]),
                event: Some(EventInput {
                    description: "위협".to_string(),
                    desirability_for_self: -0.6,
                    other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
            },
        ],
    };

    service.start_scene(scene_req, || {}, || vec![]).unwrap();

    // Fear를 수동으로 설정
    let mut state = service.repository().get_emotion_state("mu_baek").unwrap();
    state.add(Emotion::new(EmotionType::Fear, 0.5));
    service.repository_mut().save_emotion_state("mu_baek", state);

    let stim = StimulusRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation_description: None,
        pleasure: 0.0,
        arousal: 0.0,
        dominance: 0.0,
    };
    let response = service.apply_stimulus(stim, || {}, || vec![]).unwrap();

    assert!(response.beat_changed);
    assert_eq!(response.active_focus_id, Some("fear_focus".to_string()));
    assert!(!response.prompt.is_empty()); // 포맷팅된 프롬프트
}
