//! Phase 1 Mind Architecture (relationships.md v0.7 §6) Stage 4 narrative validation 2/3.
//!
//! **시나리오**: 수련이 춘설병에게 호흡 중심 검법 가르침 — 일상.
//! **데이터**: `data/scenarios/phase1-validation/daily-training.json`
//! **기대 동작** (게이트 통과 — significance >= 0.3 OR !is_chitchat):
//! - DialogueReflected 발행
//! - RelationshipUpdated 발행 (★ outer loop 진입)
//! - EmotionCleared, SceneEnded 항상
//! - 최종 follow-up: 4개 (DialogueReflected + RelationshipUpdated + EmotionCleared + SceneEnded)
//! - axes 미세 변화 (closeness/trust 약간 ↑)

use std::sync::{Arc, Mutex};

use npc_mind::adapter::memory_repository::InMemoryRepository;
use npc_mind::application::command::{Command, CommandDispatcher};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::event::EventKind;
use npc_mind::domain::reflection::ReflectionResult;
use npc_mind::ports::{EmotionStore, NpcWorld};

const SCENARIO_PATH: &str = "data/scenarios/phase1-validation/daily-training.json";

fn daily_reflection() -> ReflectionResult {
    ReflectionResult {
        is_chitchat: false,
        summary: "수련이 춘설병에게 호흡 중심 검법을 가르치고 격려".into(),
        significance_score: 0.50,
        declarative_events: vec![],
        partnership_event: None,
        turn_count: 8,
        llm_reasoning: Some("가르침 + 격려 (Pride/Admiration). transformation 아님".into()),
    }
}

#[tokio::test]
#[ignore = "Stage 2 4축 update_axes_from_emotion 매핑 도입 후 재활성화 — Stage 1은 no-op (이벤트 발행은 OK, 값 변화 0)"]
async fn daily_training_enters_outer_loop_emits_four_events_and_updates_axes() {
    // 1. 시나리오 로드
    let repo = InMemoryRepository::from_file(SCENARIO_PATH).expect("시나리오 로드 OK");
    let mentor = repo.get_npc("yu_shulien").expect("수련");
    assert_eq!(
        mentor.compass_short_label(),
        Some("공성명수신퇴(功成名遂身退) — 공을 이루었으니 물러난다"),
    );

    // 2. dispatcher 셋업
    let repo_arc = Arc::new(Mutex::new(repo));
    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(repo_arc.clone(), event_store, bus)
        .with_default_handlers();

    // 3. emotion state 사전 설정 — 가르침 후 mild Pride
    {
        use npc_mind::domain::emotion::{EmotionState, EmotionType};
        let mut state = EmotionState::default();
        state.set_intensity(EmotionType::Pride, 0.4);
        dispatcher
            .repository_guard()
            .save_emotion_state("yu_shulien", state);
    }

    // 4. 사전 axes 박제
    let initial_closeness = {
        let repo = repo_arc.lock().unwrap();
        repo.get_relationship("yu_shulien", "chunxueping")
            .unwrap()
            .affinity()
            .value()
    };

    // 5. dispatch
    let output = dispatcher
        .dispatch_v2(Command::EndDialogue {
            npc_id: "yu_shulien".into(),
            partner_id: "chunxueping".into(),
            significance: None,
            reflection: Some(daily_reflection()),
        })
        .await
        .expect("dispatch OK");

    // 6. 이벤트 시퀀스 — 4 follow-ups (RelationshipUpdated 포함)
    let kinds: Vec<EventKind> = output.events.iter().map(|e| e.kind()).collect();
    assert!(kinds.contains(&EventKind::DialogueReflected));
    assert!(
        kinds.contains(&EventKind::RelationshipUpdated),
        "★ outer loop 진입 — RelationshipUpdated 발행"
    );
    assert!(kinds.contains(&EventKind::EmotionCleared));
    assert!(kinds.contains(&EventKind::SceneEnded));
    // DialogueEndRequested(1) + DialogueReflected + RelationshipUpdated + EmotionCleared + SceneEnded = 5
    assert_eq!(output.events.len(), 5, "총 5 이벤트");

    // 7. axes 변화 — significance 0.5에 Pride 0.4 영향 → 약간의 변화 기대.
    //    구체 수치는 RelationshipPolicy 내부 계산 — 여기서는 *변화 발생*만 검증.
    let after_closeness = {
        let repo = repo_arc.lock().unwrap();
        repo.get_relationship("yu_shulien", "chunxueping")
            .unwrap()
            .affinity()
            .value()
    };
    assert!(
        (after_closeness - initial_closeness).abs() > f32::EPSILON,
        "★ closeness 변화 (mid-significance — 미세 갱신 발생)"
    );
}
