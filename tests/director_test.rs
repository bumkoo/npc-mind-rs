//! Director 다중 Scene 통합 테스트 (B안 B4 Session 2)
//!
//! - Scene lifecycle (start/active/end)
//! - 다중 Scene 동시 활성
//! - Scene-scoped 커맨드 라우팅 및 mismatch 검증
//! - 이벤트 격리 (각 Scene의 이벤트가 aggregate_key::Scene로 구분되는지)
//! - Scene 종료가 다른 Scene에 영향 없음

mod common;

use common::{make_무백, make_교룡, make_수련, TestContext};
use npc_mind::application::command::dispatcher::CommandDispatcher;
use npc_mind::application::command::types::Command;
use npc_mind::application::director::{Director, DirectorError};
use npc_mind::application::dto::{EventInput, SceneFocusInput, SituationInput};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::aggregate::AggregateKey;
use npc_mind::domain::relationship::Relationship;
use npc_mind::domain::scene_id::SceneId;
use npc_mind::InMemoryRepository;
use npc_mind::EventStore;

use std::sync::Arc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// 3개 NPC(무백/교룡/수련)로 준비된 Repository + Director
fn three_npc_director() -> Director<InMemoryRepository> {
    let mut repo = InMemoryRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_npc(make_수련());
    // 양방향 관계 등록: (무백↔교룡), (무백↔수련)
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));
    repo.add_relationship(Relationship::neutral("gyo_ryong", "mu_baek"));
    repo.add_relationship(Relationship::neutral("mu_baek", "su_ryeon"));
    repo.add_relationship(Relationship::neutral("su_ryeon", "mu_baek"));

    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(repo, store, bus).with_default_handlers();
    Director::new(dispatcher)
}

fn simple_initial_focus(id: &str) -> SceneFocusInput {
    SceneFocusInput {
        id: id.into(),
        description: format!("{id} 초기"),
        trigger: None, // Initial
        event: Some(EventInput {
            description: format!("{id} 사건"),
            desirability_for_self: 0.3,
            other: None,
            prospect: None,
        }),
        action: None,
        object: None,
        test_script: vec![],
    }
}

fn appraise_for(scene_id: &SceneId) -> Command {
    Command::Appraise {
        npc_id: scene_id.npc_id.clone(),
        partner_id: scene_id.partner_id.clone(),
        situation: Some(SituationInput {
            description: "검증 상황".into(),
            event: Some(EventInput {
                description: "test".into(),
                desirability_for_self: 0.1,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
        }),
    }
}

// ---------------------------------------------------------------------------
// 1. Scene lifecycle
// ---------------------------------------------------------------------------

#[test]
fn director_new_has_no_active_scenes() {
    let ctx = TestContext::new();
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(ctx.repo, store, bus).with_default_handlers();
    let director = Director::new(dispatcher);
    assert!(director.active_scenes().is_empty());
}

#[test]
fn start_scene_registers_scene_and_emits_lifecycle_events() {
    let mut director = three_npc_director();

    let (scene_id, out) = director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            Some(0.5),
            vec![simple_initial_focus("initial")],
        )
        .expect("start_scene must succeed");

    assert_eq!(scene_id, SceneId::new("mu_baek", "gyo_ryong"));
    assert!(director.is_active(&scene_id));
    assert_eq!(director.active_scenes().len(), 1);

    // SceneStarted가 발행되었고, 초기 focus가 있어 EmotionAppraised + GuideGenerated도 cascade
    let kinds: Vec<_> = out.events.iter().map(|e| e.kind()).collect();
    use npc_mind::domain::event::EventKind;
    assert!(kinds.contains(&EventKind::SceneStarted));
    assert!(kinds.contains(&EventKind::EmotionAppraised));
}

#[test]
fn start_scene_rejects_duplicate_activation() {
    let mut director = three_npc_director();

    director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("initial")],
        )
        .expect("first start ok");

    let err = director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("initial")],
        )
        .expect_err("duplicate must fail");
    assert!(matches!(err, DirectorError::SceneAlreadyActive(_)));
}

#[test]
fn end_scene_removes_from_active_list() {
    let mut director = three_npc_director();

    let (scene_id, _) = director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("initial")],
        )
        .unwrap();

    director
        .end_scene(&scene_id, Some(0.5))
        .expect("end must succeed");
    assert!(!director.is_active(&scene_id));
    assert!(director.active_scenes().is_empty());
}

#[test]
fn end_scene_on_unknown_scene_returns_error() {
    let mut director = three_npc_director();
    let phantom = SceneId::new("ghost", "nobody");
    let err = director
        .end_scene(&phantom, None)
        .expect_err("unknown scene must fail");
    assert!(matches!(err, DirectorError::SceneNotActive(_)));
}

// ---------------------------------------------------------------------------
// 2. dispatch_to 검증
// ---------------------------------------------------------------------------

#[test]
fn dispatch_to_routes_to_correct_scene() {
    let mut director = three_npc_director();
    let (scene_id, _) = director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("initial")],
        )
        .unwrap();

    let out = director
        .dispatch_to(&scene_id, appraise_for(&scene_id))
        .expect("ok");
    assert!(!out.events.is_empty());
}

#[test]
fn dispatch_to_inactive_scene_returns_error() {
    let mut director = three_npc_director();
    let phantom = SceneId::new("ghost", "nobody");
    let err = director
        .dispatch_to(&phantom, appraise_for(&phantom))
        .expect_err("must fail");
    assert!(matches!(err, DirectorError::SceneNotActive(_)));
}

#[test]
fn dispatch_to_rejects_command_targeting_different_scene() {
    let mut director = three_npc_director();
    let (scene_id, _) = director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("initial")],
        )
        .unwrap();

    // scene_id는 mu_baek↔gyo_ryong인데 커맨드는 mu_baek↔su_ryeon
    let wrong_cmd = Command::Appraise {
        npc_id: "mu_baek".into(),
        partner_id: "su_ryeon".into(),
        situation: Some(SituationInput {
            description: "wrong".into(),
            event: Some(EventInput {
                description: "x".into(),
                desirability_for_self: 0.1,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
        }),
    };
    let err = director
        .dispatch_to(&scene_id, wrong_cmd)
        .expect_err("must fail mismatch");
    assert!(matches!(err, DirectorError::SceneMismatch(_, _, _)));
}

// ---------------------------------------------------------------------------
// 3. 다중 Scene 동시 활성 + 이벤트 격리
// ---------------------------------------------------------------------------

#[test]
fn two_scenes_coexist_and_events_are_aggregate_separated() {
    let mut director = three_npc_director();

    let (scene_a, _) = director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("a_initial")],
        )
        .unwrap();
    let (scene_b, _) = director
        .start_scene(
            "mu_baek",
            "su_ryeon",
            None,
            vec![simple_initial_focus("b_initial")],
        )
        .unwrap();

    assert_eq!(director.active_scenes().len(), 2);

    // 각 Scene에 appraise 커맨드 송신
    director
        .dispatch_to(&scene_a, appraise_for(&scene_a))
        .unwrap();
    director
        .dispatch_to(&scene_b, appraise_for(&scene_b))
        .unwrap();

    // 이벤트 스토어에서 각 Scene의 이벤트가 aggregate_key::Scene으로 구분되는지 확인
    let events = director.dispatcher().event_store().get_all_events();
    let scene_a_key = AggregateKey::Scene {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
    };
    let scene_b_key = AggregateKey::Scene {
        npc_id: "mu_baek".into(),
        partner_id: "su_ryeon".into(),
    };
    let npc_a_key = AggregateKey::Npc("mu_baek".into());

    // SceneStarted는 Scene aggregate, Emotion/Guide/StimulusApplied는 Npc aggregate
    let scene_a_events = events
        .iter()
        .filter(|e| e.aggregate_key() == scene_a_key)
        .count();
    let scene_b_events = events
        .iter()
        .filter(|e| e.aggregate_key() == scene_b_key)
        .count();
    let npc_events = events
        .iter()
        .filter(|e| e.aggregate_key() == npc_a_key)
        .count();

    assert!(scene_a_events > 0, "scene_a 관련 이벤트 존재");
    assert!(scene_b_events > 0, "scene_b 관련 이벤트 존재");
    assert!(
        npc_events > 0,
        "NPC aggregate 이벤트도 존재 (Emotion/Guide/Stimulus 계열)"
    );
}

#[test]
fn ending_one_scene_leaves_other_active() {
    let mut director = three_npc_director();

    let (scene_a, _) = director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("a_initial")],
        )
        .unwrap();
    let (scene_b, _) = director
        .start_scene(
            "mu_baek",
            "su_ryeon",
            None,
            vec![simple_initial_focus("b_initial")],
        )
        .unwrap();

    director.end_scene(&scene_a, None).unwrap();

    assert!(!director.is_active(&scene_a));
    assert!(director.is_active(&scene_b), "scene_b는 영향 없이 유지");
    assert_eq!(director.active_scenes().len(), 1);

    // scene_b에 계속 커맨드 송신 가능
    director
        .dispatch_to(&scene_b, appraise_for(&scene_b))
        .expect("scene_b는 독립적으로 동작");
}

/// B4 Session 3 (Option A) 회귀 가드 — Beat 전환 시 BeatTransitioned.partner_id가 payload에
/// 담겨있어 RelationshipAgent가 **올바른 Scene의 관계**를 갱신함을 확인.
///
/// 시나리오:
/// 1. Scene A (mu_baek↔gyo_ryong)와 Scene B (mu_baek↔su_ryeon)를 동시 활성화
/// 2. Scene B를 먼저 시작하여 `InMemoryRepository.last_scene_id`를 B로 둠
/// 3. Scene A에서 Beat 전환 유발 stimulus 실행
/// 4. 이전 (Session 2) 구현에서는 RelationshipAgent가 `ctx.repo.get_scene()` →
///    last_scene_id가 가리키는 **Scene B의 partner_id(su_ryeon)** 를 읽어 잘못된 관계 갱신
/// 5. 이번 수정 후에는 event.partner_id(gyo_ryong)을 직접 읽어 올바른 관계를 갱신
#[test]
fn beat_transition_in_scene_a_updates_scene_a_relationship_not_scene_b() {
    use npc_mind::domain::emotion::{
        ConditionThreshold, EmotionCondition, EmotionType, EventFocus, FocusTrigger, SceneFocus,
    };
    use npc_mind::domain::event::{EventKind, EventPayload};

    // Beat 트리거 가능한 Scene은 도메인 타입으로 직접 구성해 repo에 주입
    // (SceneFocusInput DTO의 trigger 필드가 ConditionInput Vec 형태라 값 생성이 번거로움).
    let mut repo = InMemoryRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_npc(make_수련());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));
    repo.add_relationship(Relationship::neutral("gyo_ryong", "mu_baek"));
    repo.add_relationship(Relationship::neutral("mu_baek", "su_ryeon"));
    repo.add_relationship(Relationship::neutral("su_ryeon", "mu_baek"));

    // Scene A (mu_baek↔gyo_ryong): Beat 트리거 가능 구조
    let scene_a = {
        let focuses = vec![
            SceneFocus {
                id: "a_initial".into(),
                description: "A 초기".into(),
                trigger: FocusTrigger::Initial,
                event: Some(EventFocus {
                    description: "".into(),
                    desirability_for_self: 0.3,
                    desirability_for_other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
                test_script: vec![],
            },
            SceneFocus {
                id: "a_next".into(),
                description: "A 다음".into(),
                trigger: FocusTrigger::Conditions(vec![vec![EmotionCondition {
                    emotion: EmotionType::Hate,
                    threshold: ConditionThreshold::Absent,
                }]]),
                event: Some(EventFocus {
                    description: "".into(),
                    desirability_for_self: 0.2,
                    desirability_for_other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
                test_script: vec![],
            },
        ];
        let mut s =
            npc_mind::domain::emotion::Scene::new("mu_baek".into(), "gyo_ryong".into(), focuses);
        s.set_active_focus("a_initial".into());
        s
    };
    // Scene B (mu_baek↔su_ryeon): 단순 initial만
    let scene_b = {
        let focuses = vec![SceneFocus {
            id: "b_initial".into(),
            description: "B 초기".into(),
            trigger: FocusTrigger::Initial,
            event: Some(EventFocus {
                description: "".into(),
                desirability_for_self: 0.1,
                desirability_for_other: None,
                prospect: None,
            }),
            action: None,
            object: None,
            test_script: vec![],
        }];
        let mut s =
            npc_mind::domain::emotion::Scene::new("mu_baek".into(), "su_ryeon".into(), focuses);
        s.set_active_focus("b_initial".into());
        s
    };

    // Repository에 Scene A 먼저, 그 다음 Scene B 저장 →
    // last_scene_id = Scene B (mu_baek↔su_ryeon) 상태. 이전 버그에서는 Beat 전환 시
    // 이 last_scene_id를 읽어 잘못된 관계(su_ryeon)를 갱신.
    use npc_mind::ports::SceneStore;
    repo.save_scene(scene_a);
    repo.save_scene(scene_b);
    assert_eq!(
        repo.get_scene().unwrap().partner_id(),
        "su_ryeon",
        "last_scene_id가 Scene B를 가리키는 상태"
    );

    // Scene A에 대해 appraise + stimulus 순서로 dispatch → Beat 전환 유발
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let mut dispatcher = CommandDispatcher::new(repo, store.clone(), bus).with_default_handlers();

    // seed emotion_state for mu_baek (via appraise-like command against Scene A partner)
    dispatcher
        .dispatch_v2(Command::Appraise {
            npc_id: "mu_baek".into(),
            partner_id: "gyo_ryong".into(),
            situation: Some(SituationInput {
                description: "시드".into(),
                event: Some(EventInput {
                    description: "x".into(),
                    desirability_for_self: 0.1,
                    other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
            }),
        })
        .expect("seed appraise");

    // stimulus → Beat 전환 유도. Scene A의 "a_next"가 Hate Absent 조건이라 반드시 트리거.
    dispatcher
        .dispatch_v2(Command::ApplyStimulus {
            npc_id: "mu_baek".into(),
            partner_id: "gyo_ryong".into(),
            pleasure: 0.4,
            arousal: 0.2,
            dominance: 0.0,
            situation_description: None,
        })
        .expect("stimulus triggering beat");

    // BeatTransitioned 이벤트를 event_store에서 찾아 partner_id 검증
    let all = store.get_all_events();
    let beat = all
        .iter()
        .find(|e| e.kind() == EventKind::BeatTransitioned)
        .expect("BeatTransitioned 발행되어야 함");
    let EventPayload::BeatTransitioned { partner_id, .. } = &beat.payload else {
        panic!("kind가 BeatTransitioned인데 payload가 맞지 않음")
    };
    assert_eq!(
        partner_id, "gyo_ryong",
        "BeatTransitioned.partner_id는 Scene A의 partner를 정확히 가리켜야 함"
    );

    // RelationshipUpdated 이벤트도 Scene A의 관계를 가리키는지 확인
    let rel_updated = all
        .iter()
        .find(|e| e.kind() == EventKind::RelationshipUpdated)
        .expect("BeatTransitioned 후 RelationshipUpdated 발행");
    let EventPayload::RelationshipUpdated {
        owner_id,
        target_id,
        ..
    } = &rel_updated.payload
    else {
        panic!("payload mismatch")
    };
    assert_eq!(
        (owner_id.as_str(), target_id.as_str()),
        ("mu_baek", "gyo_ryong"),
        "RelationshipAgent는 Scene A의 관계(mu_baek→gyo_ryong)를 갱신해야 함 — \
         이전 버그에서는 last_scene_id(Scene B)의 su_ryeon을 target으로 잡았음"
    );
}

#[test]
fn repository_holds_both_scenes_by_id() {
    let mut director = three_npc_director();

    director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("a_initial")],
        )
        .unwrap();
    director
        .start_scene(
            "mu_baek",
            "su_ryeon",
            None,
            vec![simple_initial_focus("b_initial")],
        )
        .unwrap();

    let repo = director.dispatcher().repository();
    let ids = repo.list_scene_ids();
    assert_eq!(ids.len(), 2, "InMemoryRepository.scenes HashMap에 2 Scene 보존");

    let scene_a = repo
        .get_scene_by_id(&SceneId::new("mu_baek", "gyo_ryong"))
        .expect("scene_a");
    let scene_b = repo
        .get_scene_by_id(&SceneId::new("mu_baek", "su_ryeon"))
        .expect("scene_b");
    assert_eq!(scene_a.partner_id(), "gyo_ryong");
    assert_eq!(scene_b.partner_id(), "su_ryeon");
}
