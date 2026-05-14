//! Phase 1 Mind Architecture (relationships.md v0.7 §6) Stage 4 narrative validation 1/3.
//!
//! **시나리오**: 임충이 길에서 행인과 의례적 인사 — 잡담.
//! **데이터**: `data/scenarios/phase1-validation/chitchat-passerby.json`
//! **기대 동작** (RelationshipPolicy 게이트가 outer loop skip):
//! - DialogueReflected 발행 (chitchat 박제)
//! - RelationshipUpdated **미발행** (axes 보존)
//! - EmotionCleared, SceneEnded 항상
//! - 최종 follow-up: 3개 (DialogueReflected + EmotionCleared + SceneEnded)
//!
//! Mock LLM 사용 — 실제 LLM은 디자이너(Bekay) Mind Studio 수동 검증 (spec §4.4).

use std::sync::{Arc, Mutex};

use npc_mind::adapter::memory_repository::InMemoryRepository;
use npc_mind::application::command::{Command, CommandDispatcher};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::event::{EventKind, EventPayload};
use npc_mind::domain::reflection::ReflectionResult;
use npc_mind::ports::{EmotionStore, NpcWorld};

const SCENARIO_PATH: &str = "data/scenarios/phase1-validation/chitchat-passerby.json";

fn chitchat_reflection() -> ReflectionResult {
    ReflectionResult {
        is_chitchat: true,
        summary: "행인과 의례적 인사 — 서사적 비중 없음".into(),
        significance_score: 0.05,
        declarative_events: vec![],
        partnership_event: None,
        turn_count: 3,
        llm_reasoning: Some("의례적 인사. OCC peak 0, PAD trajectory 0".into()),
    }
}

#[tokio::test]
async fn chitchat_skips_outer_loop_emits_three_events_and_preserves_axes() {
    // 1. 시나리오 로드 (inner_compass 포함된 NPC 2명 + 관계 1개)
    let repo = InMemoryRepository::from_file(SCENARIO_PATH).expect("시나리오 로드 OK");
    let npc = repo.get_npc("lin_chong").expect("임충");
    assert_eq!(
        npc.compass_short_label(),
        Some("체제의 법도와 의리를 지킨다 — 칠지에 검을 빼지 않는다"),
        "A-min inner_compass JSON 호환 검증"
    );

    // 2. dispatcher 셋업
    let repo_arc = Arc::new(Mutex::new(repo));
    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(repo_arc.clone(), event_store, bus)
        .with_default_handlers();

    // 3. 사전 emotion state — appraise 안 거치고 default. RelationshipPolicy의 게이트가
    //    chitchat skip이므로 emotion lookup도 발생 안 함 (lookup skip 조건 검증).
    dispatcher
        .repository_guard()
        .save_emotion_state("lin_chong", Default::default());

    // 4. Command::EndDialogue dispatch — reflection으로 chitchat 박제
    let initial_relationship = repo_arc
        .lock()
        .unwrap()
        .get_relationship("lin_chong", "passerby")
        .expect("관계 존재");
    let initial_closeness = initial_relationship.affinity().value();
    let initial_trust = initial_relationship.trust().value();

    let output = dispatcher
        .dispatch_v2(Command::EndDialogue {
            npc_id: "lin_chong".into(),
            partner_id: "passerby".into(),
            significance: None,
            reflection: Some(chitchat_reflection()),
        })
        .await
        .expect("dispatch OK");

    // 5. 이벤트 시퀀스 검증 — DialogueEndRequested(initial) + 3 follow-ups
    //    (DialogueReflected + EmotionCleared + SceneEnded). RelationshipUpdated **미발행**.
    let kinds: Vec<EventKind> = output.events.iter().map(|e| e.kind()).collect();
    assert!(
        kinds.contains(&EventKind::DialogueEndRequested),
        "초기 이벤트 박제"
    );
    assert!(
        kinds.contains(&EventKind::DialogueReflected),
        "chitchat도 reflection 박제"
    );
    assert!(
        kinds.contains(&EventKind::EmotionCleared),
        "EmotionCleared 항상 발행"
    );
    assert!(
        kinds.contains(&EventKind::SceneEnded),
        "SceneEnded 항상 발행"
    );
    assert!(
        !kinds.contains(&EventKind::RelationshipUpdated),
        "★ chitchat skip — RelationshipUpdated 미발행 (axes 보존)"
    );
    // 합계: DialogueEndRequested(1) + DialogueReflected(1) + EmotionCleared(1) + SceneEnded(1)
    // = 4 (initial + 3 follow-ups)
    assert_eq!(output.events.len(), 4, "총 4 이벤트 (1 initial + 3 follow-up)");

    // 6. axes 보존 검증 — relationship 그대로
    let after_relationship = repo_arc
        .lock()
        .unwrap()
        .get_relationship("lin_chong", "passerby")
        .expect("관계 여전히 존재");
    assert_eq!(
        after_relationship.affinity().value(),
        initial_closeness,
        "★ closeness 보존 (chitchat outer loop skip)"
    );
    assert_eq!(
        after_relationship.trust().value(),
        initial_trust,
        "★ trust 보존"
    );

    // 7. DialogueReflected payload에 reflection 박제 검증
    let reflected_event = output
        .events
        .iter()
        .find(|e| e.kind() == EventKind::DialogueReflected)
        .expect("DialogueReflected 존재");
    if let EventPayload::DialogueReflected { result, npc_id, partner_id, .. } = &reflected_event.payload {
        assert!(result.is_chitchat, "is_chitchat 보존");
        assert_eq!(npc_id, "lin_chong");
        assert_eq!(partner_id, "passerby");
    } else {
        panic!("DialogueReflected payload 타입 mismatch");
    }
}
