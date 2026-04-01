//! InMemoryRepository 통합 테스트
//!
//! - 프로그래밍 방식 (new + add_npc/add_relationship)
//! - Mind Studio scenario.json 로드 (from_file / from_json)
//! - Scene 포함 JSON 로드

use npc_mind::{
    InMemoryRepository, MindService, FormattedMindService,
    AppraiseRequest, NpcWorld, EmotionStore, SceneStore,
};
use npc_mind::application::dto::*;
use npc_mind::domain::personality::{NpcBuilder, Score};
use npc_mind::domain::relationship::RelationshipBuilder;

// ---------------------------------------------------------------------------
// 프로그래밍 방식 테스트
// ---------------------------------------------------------------------------

#[test]
fn new_repository_is_empty() {
    let repo = InMemoryRepository::new();
    assert!(repo.get_npc("any").is_none());
    assert!(repo.get_relationship("a", "b").is_none());
    assert!(repo.get_object_description("obj").is_none());
    assert!(repo.get_emotion_state("npc").is_none());
    assert!(repo.get_scene().is_none());
}

#[test]
fn add_npc_and_retrieve() {
    let mut repo = InMemoryRepository::new();
    let npc = NpcBuilder::new("test_npc", "테스트")
        .description("테스트 NPC")
        .build();
    repo.add_npc(npc);

    let found = repo.get_npc("test_npc");
    assert!(found.is_some());
    assert_eq!(found.unwrap().name(), "테스트");
}

#[test]
fn add_relationship_and_retrieve_bidirectional() {
    let mut repo = InMemoryRepository::new();
    let rel = RelationshipBuilder::new("a", "b")
        .closeness(Score::clamped(0.5))
        .trust(Score::clamped(0.8))
        .build();
    repo.add_relationship(rel);

    // 정방향 조회
    assert!(repo.get_relationship("a", "b").is_some());
    // 역방향 조회
    assert!(repo.get_relationship("b", "a").is_some());
}

#[test]
fn add_object_and_retrieve() {
    let mut repo = InMemoryRepository::new();
    repo.add_object("sword", "명검 천하제일검");

    assert_eq!(repo.get_object_description("sword").unwrap(), "명검 천하제일검");
    assert!(repo.get_object_description("nothing").is_none());
}

#[test]
fn programmatic_repository_with_mind_service() {
    let mut repo = InMemoryRepository::new();
    let npc = NpcBuilder::new("mu_baek", "무백")
        .description("검객")
        .honesty_humility(|h| h.sincerity = Score::clamped(0.8))
        .build();
    repo.add_npc(npc);
    repo.add_relationship(
        RelationshipBuilder::new("mu_baek", "player").build()
    );

    let mut service = MindService::new(&mut repo);
    let result = service.appraise(AppraiseRequest {
        npc_id: "mu_baek".into(),
        partner_id: "player".into(),
        situation: SituationInput {
            description: "좋은 일이 일어났다".into(),
            event: Some(EventInput {
                description: "기쁜 소식".into(),
                desirability_for_self: 0.7,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
        },
    }, || {}, || vec![]);

    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(!result.emotions.is_empty());
}

// ---------------------------------------------------------------------------
// JSON 로드 테스트
// ---------------------------------------------------------------------------

#[test]
fn from_file_loads_scenario_json() {
    let repo = InMemoryRepository::from_file(
        "data/huckleberry_finn/ch8_jackson_island_meeting/session_001/scenario.json"
    ).unwrap();

    // NPC 로드 확인
    assert!(repo.get_npc("mu_baek").is_some());
    assert!(repo.get_npc("gyo_ryong").is_some());
    assert!(repo.get_npc("jim").is_some());
    assert!(repo.get_npc("huck").is_some());

    // 관계 로드 확인
    assert!(repo.get_relationship("mu_baek", "gyo_ryong").is_some());

    // 시나리오 메타데이터
    assert_eq!(repo.scenario_name(), "Jackson Island Meeting #1");
    assert!(!repo.scenario_description().is_empty());
}

#[test]
fn from_file_loads_scene_with_focuses() {
    let repo = InMemoryRepository::from_file(
        "data/huckleberry_finn/ch15_fog_trash/session_001/scenario.json"
    ).unwrap();

    // NPC/관계
    assert!(repo.get_npc("jim").is_some());
    assert!(repo.get_npc("huck").is_some());
    assert!(repo.get_relationship("jim", "huck").is_some());

    // Scene 로드 확인
    let scene = repo.get_scene();
    assert!(scene.is_some());
    let scene = scene.unwrap();
    assert_eq!(scene.focuses().len(), 2);

    // Initial focus가 active로 설정됨
    assert!(scene.active_focus_id().is_some());
}

#[test]
fn from_file_with_scene_can_run_service() {
    let mut repo = InMemoryRepository::from_file(
        "data/huckleberry_finn/ch15_fog_trash/session_001/scenario.json"
    ).unwrap();

    let service = MindService::new(&mut repo);
    let info = service.scene_info();
    assert!(info.has_scene);
    assert_eq!(info.focuses.len(), 2);
}

#[test]
fn from_json_minimal() {
    let json = r#"{
        "npcs": {
            "npc1": {
                "id": "npc1",
                "name": "테스트",
                "description": "테스트 NPC",
                "sincerity": 0.5
            }
        },
        "relationships": {
            "npc1:player": {
                "owner_id": "npc1",
                "target_id": "player",
                "closeness": 0.3,
                "trust": 0.5,
                "power": 0.0
            }
        },
        "objects": {}
    }"#;

    let repo = InMemoryRepository::from_json(json).unwrap();
    let npc = repo.get_npc("npc1").unwrap();
    assert_eq!(npc.name(), "테스트");

    let rel = repo.get_relationship("npc1", "player").unwrap();
    assert!((rel.closeness().value() - 0.3).abs() < 0.01);
}

#[test]
fn from_json_missing_fields_use_defaults() {
    // 최소한의 JSON — 빈 객체도 파싱 가능
    let json = r#"{ "npcs": {}, "relationships": {} }"#;
    let repo = InMemoryRepository::from_json(json).unwrap();
    assert!(repo.scenario_name().is_empty());
    assert!(repo.turn_history().is_empty());
}

#[test]
fn from_json_invalid_json_returns_error() {
    let result = InMemoryRepository::from_json("not valid json");
    assert!(result.is_err());
}

#[test]
fn from_file_nonexistent_returns_error() {
    let result = InMemoryRepository::from_file("nonexistent.json");
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// FormattedMindService 연동 테스트
// ---------------------------------------------------------------------------

#[test]
fn formatted_service_with_loaded_repository() {
    let repo = InMemoryRepository::from_file(
        "data/huckleberry_finn/ch15_fog_trash/session_001/scenario.json"
    ).unwrap();

    let mut service = FormattedMindService::new(repo, "ko").unwrap();
    let response = service.appraise(AppraiseRequest {
        npc_id: "jim".into(),
        partner_id: "huck".into(),
        situation: SituationInput {
            description: "헉이 거짓말로 짐을 속였다".into(),
            event: Some(EventInput {
                description: "거짓말 발각".into(),
                desirability_for_self: -0.8,
                other: None,
                prospect: None,
            }),
            action: Some(ActionInput {
                description: "기만 행위".into(),
                agent_id: Some("huck".into()),
                praiseworthiness: -0.8,
            }),
            object: None,
        },
    }, || {}, || vec![]);

    assert!(response.is_ok());
    let response = response.unwrap();
    assert!(!response.prompt.is_empty());
    assert!(!response.emotions.is_empty());
}
