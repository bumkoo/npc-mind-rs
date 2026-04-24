//! Director 다중 Scene 통합 테스트 (B안 B4 Session 2)
//!
//! - Scene lifecycle (start/active/end)
//! - 다중 Scene 동시 활성
//! - Scene-scoped 커맨드 라우팅 및 mismatch 검증
//! - 이벤트 격리 (각 Scene의 이벤트가 aggregate_key::Scene로 구분되는지)
//! - Scene 종료가 다른 Scene에 영향 없음

mod common;

use common::{make_무백, make_교룡, make_수련, expect_event, expect_events, TestContext, SCENE_TASK_TEST_TIMEOUT};
use futures::future::BoxFuture;
use npc_mind::application::command::dispatcher::CommandDispatcher;
use npc_mind::application::command::types::Command;
use npc_mind::application::director::{Director, DirectorError, Spawner};
use npc_mind::application::dto::{EventInput, SceneFocusInput, SituationInput};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::aggregate::AggregateKey;
use npc_mind::domain::relationship::Relationship;
use npc_mind::domain::scene_id::SceneId;
use npc_mind::InMemoryRepository;
use npc_mind::EventStore;

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

fn test_spawner() -> Arc<dyn Spawner> {
    Arc::new(|fut: BoxFuture<'static, ()>| {
        tokio::spawn(fut);
    })
}

/// Scene task가 큐의 커맨드를 처리하도록 잠시 대기.
/// Fire-and-forget API라 반환값에 이벤트가 포함되지 않으므로
/// 후속 assertion 전에 short sleep이 필요.
const TASK_SETTLE: Duration = Duration::from_millis(100);

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
    Director::new(dispatcher, test_spawner())
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

#[tokio::test]
async fn director_new_has_no_active_scenes() {
    let ctx = TestContext::new();
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(ctx.repo, store, bus).with_default_handlers();
    let director = Director::new(dispatcher, test_spawner());
    assert!(director.active_scenes().await.is_empty());
}

#[tokio::test]
async fn start_scene_registers_scene_and_emits_lifecycle_events() {
    let director = three_npc_director();

    // Trigger **전에** subscribe 등록 (broadcast는 replay 없음).
    use npc_mind::domain::event::EventKind;
    let events_waiter = expect_events(
        director.dispatcher().event_bus(),
        &[EventKind::SceneStarted, EventKind::EmotionAppraised],
        SCENE_TASK_TEST_TIMEOUT,
    );

    let scene_id = director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            Some(0.5),
            vec![simple_initial_focus("initial")],
        )
        .await
        .expect("start_scene must succeed");

    assert_eq!(scene_id, SceneId::new("mu_baek", "gyo_ryong"));
    assert!(director.is_active(&scene_id).await);
    assert_eq!(director.active_scenes().await.len(), 1);

    // Fire-and-forget: EventBus timeout 기반 대기 → sleep보다 견고.
    events_waiter.await;
}

#[tokio::test]
async fn start_scene_rejects_duplicate_activation() {
    let director = three_npc_director();

    director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("initial")],
        )
        .await
        .expect("first start ok");

    let err = director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("initial")],
        )
        .await
        .expect_err("duplicate must fail");
    assert!(matches!(err, DirectorError::SceneAlreadyActive(_)));
}

#[tokio::test]
async fn end_scene_removes_from_active_list() {
    let director = three_npc_director();

    let scene_id = director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("initial")],
        )
        .await
        .unwrap();

    director
        .end_scene(&scene_id, Some(0.5))
        .await
        .expect("end must succeed");
    assert!(!director.is_active(&scene_id).await);
    assert!(director.active_scenes().await.is_empty());
}

#[tokio::test]
async fn end_scene_on_unknown_scene_returns_error() {
    let director = three_npc_director();
    let phantom = SceneId::new("ghost", "nobody");
    let err = director
        .end_scene(&phantom, None)
        .await
        .expect_err("unknown scene must fail");
    assert!(matches!(err, DirectorError::SceneNotActive(_)));
}

// ---------------------------------------------------------------------------
// 2. dispatch_to 검증
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dispatch_to_routes_to_correct_scene() {
    let director = three_npc_director();
    let scene_id = director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("initial")],
        )
        .await
        .unwrap();

    director
        .dispatch_to(&scene_id, appraise_for(&scene_id))
        .await
        .expect("ok");

    // Fire-and-forget: Scene task 처리 대기 → event_store에 Appraise 계열 이벤트가 기록됨
    sleep(TASK_SETTLE).await;
    let events = director.dispatcher().event_store().get_all_events();
    assert!(!events.is_empty(), "dispatch_to 이후 이벤트가 기록되어야 함");
}

#[tokio::test]
async fn dispatch_to_inactive_scene_returns_error() {
    let director = three_npc_director();
    let phantom = SceneId::new("ghost", "nobody");
    let err = director
        .dispatch_to(&phantom, appraise_for(&phantom))
        .await
        .expect_err("must fail");
    assert!(matches!(err, DirectorError::SceneNotActive(_)));
}

#[tokio::test]
async fn dispatch_to_rejects_command_targeting_different_scene() {
    let director = three_npc_director();
    let scene_id = director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("initial")],
        )
        .await
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
        .await
        .expect_err("must fail mismatch");
    assert!(matches!(err, DirectorError::SceneMismatch(_, _, _)));
}

// ---------------------------------------------------------------------------
// 3. 다중 Scene 동시 활성 + 이벤트 격리
// ---------------------------------------------------------------------------

#[tokio::test]
async fn two_scenes_coexist_and_events_are_aggregate_separated() {
    let director = three_npc_director();

    let scene_a = director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("a_initial")],
        )
        .await
        .unwrap();
    let scene_b = director
        .start_scene(
            "mu_baek",
            "su_ryeon",
            None,
            vec![simple_initial_focus("b_initial")],
        )
        .await
        .unwrap();

    assert_eq!(director.active_scenes().await.len(), 2);

    // 각 Scene에 appraise 커맨드 송신
    director
        .dispatch_to(&scene_a, appraise_for(&scene_a))
        .await
        .unwrap();
    director
        .dispatch_to(&scene_b, appraise_for(&scene_b))
        .await
        .unwrap();

    // Fire-and-forget: Scene task가 커맨드들을 처리할 시간 필요
    sleep(TASK_SETTLE).await;

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

#[tokio::test]
async fn ending_one_scene_leaves_other_active() {
    let director = three_npc_director();

    let scene_a = director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("a_initial")],
        )
        .await
        .unwrap();
    let scene_b = director
        .start_scene(
            "mu_baek",
            "su_ryeon",
            None,
            vec![simple_initial_focus("b_initial")],
        )
        .await
        .unwrap();

    director.end_scene(&scene_a, None).await.unwrap();

    assert!(!director.is_active(&scene_a).await);
    assert!(director.is_active(&scene_b).await, "scene_b는 영향 없이 유지");
    assert_eq!(director.active_scenes().await.len(), 1);

    // scene_b에 계속 커맨드 송신 가능
    director
        .dispatch_to(&scene_b, appraise_for(&scene_b))
        .await
        .expect("scene_b는 독립적으로 동작");
}

/// B4 Session 3 (Option A) 회귀 가드 — Beat 전환 시 BeatTransitioned.partner_id가 payload에
/// 담겨있어 RelationshipPolicy가 **올바른 Scene의 관계**를 갱신함을 확인.
///
/// 시나리오:
/// 1. Scene A (mu_baek↔gyo_ryong)와 Scene B (mu_baek↔su_ryeon)를 동시 활성화
/// 2. Scene B를 먼저 시작하여 `InMemoryRepository.last_scene_id`를 B로 둠
/// 3. Scene A에서 Beat 전환 유발 stimulus 실행
/// 4. 이전 (Session 2) 구현에서는 RelationshipPolicy가 `ctx.repo.get_scene()` →
///    last_scene_id가 가리키는 **Scene B의 partner_id(su_ryeon)** 를 읽어 잘못된 관계 갱신
/// 5. 이번 수정 후에는 event.partner_id(gyo_ryong)을 직접 읽어 올바른 관계를 갱신
#[tokio::test]
async fn beat_transition_in_scene_a_updates_scene_a_relationship_not_scene_b() {
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
    let dispatcher = CommandDispatcher::new(repo, store.clone(), bus).with_default_handlers();

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
        .await
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
        .await
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
        "RelationshipPolicy는 Scene A의 관계(mu_baek→gyo_ryong)를 갱신해야 함 — \
         이전 버그에서는 last_scene_id(Scene B)의 su_ryeon을 target으로 잡았음"
    );
}

#[tokio::test]
async fn repository_holds_both_scenes_by_id() {
    let director = three_npc_director();

    director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("a_initial")],
        )
        .await
        .unwrap();
    director
        .start_scene(
            "mu_baek",
            "su_ryeon",
            None,
            vec![simple_initial_focus("b_initial")],
        )
        .await
        .unwrap();

    // Fire-and-forget: Scene tasks가 StartScene 커맨드를 처리해야 repo.scenes에 저장됨
    sleep(TASK_SETTLE).await;

    let repo = director.dispatcher().repository_guard();
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

// ---------------------------------------------------------------------------
// B4 Session 4 후속 — SceneTask 종료 + concurrent start_scene race
// ---------------------------------------------------------------------------

/// Sender drop → receiver None → SceneTask 루프가 자연 종료되는지 검증.
///
/// `end_scene`은 EndDialogue 송신 후 senders map에서 sender를 제거하고 drop한다.
/// 이때 SceneTask receiver가 None을 받고 루프를 탈출한다. 탈출은 broadcast에서
/// 관찰할 수 없으므로, 이 테스트는 "종료 후 같은 SceneId로 재시작"이 가능함을 통해
/// 간접 검증한다.
#[tokio::test]
async fn scene_task_terminates_after_end_scene_and_allows_restart() {
    use npc_mind::domain::event::EventKind;

    let director = three_npc_director();
    let scene_id = SceneId::new("mu_baek", "gyo_ryong");

    // 1. start → end
    let ended_waiter = expect_event(
        director.dispatcher().event_bus(),
        EventKind::SceneEnded,
        SCENE_TASK_TEST_TIMEOUT,
    );
    director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("initial")],
        )
        .await
        .unwrap();
    director.end_scene(&scene_id, Some(0.5)).await.unwrap();
    ended_waiter.await;

    // 2. 종료된 scene이 active 목록에서 제거되었는지
    assert!(!director.is_active(&scene_id).await);

    // 3. 같은 SceneId로 재시작이 허용되는지 (SceneTask 정상 종료 → 이전 sender가 map에 없음)
    let restart_waiter = expect_event(
        director.dispatcher().event_bus(),
        EventKind::SceneStarted,
        SCENE_TASK_TEST_TIMEOUT,
    );
    director
        .start_scene(
            "mu_baek",
            "gyo_ryong",
            None,
            vec![simple_initial_focus("restart_initial")],
        )
        .await
        .expect("종료 후 같은 SceneId 재시작 허용되어야 함");
    restart_waiter.await;
}

/// 동시 `start_scene` 호출이 같은 scene_id로 진입하는 race condition 회귀 가드.
///
/// B4 Session 4 초기 구현은 `read() → check → write()` 분리된 TOCTOU 버그가 있었다.
/// 현재는 `write()` lock을 critical section 전체에 유지하여 하나만 성공하고
/// 나머지는 `SceneAlreadyActive`를 받는다.
#[tokio::test]
async fn concurrent_start_scene_with_same_id_elects_single_winner() {
    let director = Arc::new(three_npc_director());

    // 동시 5회 start_scene 호출 (같은 scene_id)
    let mut handles = Vec::new();
    for i in 0..5 {
        let dir = Arc::clone(&director);
        handles.push(tokio::spawn(async move {
            dir.start_scene(
                "mu_baek",
                "gyo_ryong",
                None,
                vec![simple_initial_focus(&format!("attempt_{}", i))],
            )
            .await
        }));
    }

    let results: Vec<_> = futures::future::join_all(handles).await;
    let ok_count = results.iter().filter(|r| matches!(r, Ok(Ok(_)))).count();
    let already_active_count = results
        .iter()
        .filter(|r| matches!(r, Ok(Err(DirectorError::SceneAlreadyActive(_)))))
        .count();

    assert_eq!(
        ok_count, 1,
        "정확히 1개 호출만 성공해야 함 (TOCTOU 회귀 방지)"
    );
    assert_eq!(
        already_active_count, 4,
        "나머지 4개는 SceneAlreadyActive로 거부되어야 함"
    );

    assert_eq!(director.active_scenes().await.len(), 1);
}
