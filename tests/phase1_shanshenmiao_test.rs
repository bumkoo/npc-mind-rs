//! Phase 1 Mind Architecture (relationships.md v0.7 §6) Stage 4 narrative validation 3/3.
//!
//! **시나리오**: 임충이 산신묘에서 육겸 처단 — 결단.
//! **데이터**: `data/scenarios/phase1-validation/lin-chong-shanshenmiao.json`
//! **기대 동작** (게이트 모든 조건 통과):
//! - DialogueReflected 발행 (큰 reasoning + significance ≥ 0.85)
//! - RelationshipUpdated 발행 (★ outer loop 진입)
//! - EmotionCleared, SceneEnded 항상
//! - 최종 follow-up: 4개
//! - axes 큰 변화 (lin_chong → lu_qian closeness/trust 극단 음수 방향)

use std::sync::{Arc, Mutex};

use npc_mind::adapter::memory_repository::InMemoryRepository;
use npc_mind::application::command::{Command, CommandDispatcher};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::event::{EventKind, EventPayload};
use npc_mind::domain::reflection::ReflectionResult;
use npc_mind::ports::{EmotionStore, NpcWorld};

const SCENARIO_PATH: &str = "data/scenarios/phase1-validation/lin-chong-shanshenmiao.json";

fn shanshenmiao_reflection() -> ReflectionResult {
    ReflectionResult {
        is_chitchat: false,
        summary: "임충이 산신묘에서 육겸을 처단하고 체제에 등을 돌리는 결단".into(),
        significance_score: 0.92,
        declarative_events: vec![],
        partnership_event: None,
        turn_count: 9,
        llm_reasoning: Some(
            "OCC peak 0.95+ (Anger), PAD trajectory 큼 (분노→공허), beat 전환 (도착→처단)".into(),
        ),
    }
}

#[tokio::test]
async fn shanshenmiao_high_band_emits_four_events_and_reverses_axes_strongly() {
    // 1. 시나리오 로드
    let repo = InMemoryRepository::from_file(SCENARIO_PATH).expect("시나리오 로드 OK");
    let lin = repo.get_npc("lin_chong").expect("임충");
    assert!(
        lin.compass_short_label()
            .unwrap_or("")
            .contains("의리 없는 자는"),
        "임충 compass 변화 (전환점) 반영"
    );

    // 2. dispatcher 셋업
    let repo_arc = Arc::new(Mutex::new(repo));
    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(repo_arc.clone(), event_store, bus)
        .with_default_handlers();

    // 3. emotion state — Anger 격발
    {
        use npc_mind::domain::emotion::{EmotionState, EmotionType};
        let mut state = EmotionState::default();
        state.set_intensity(EmotionType::Anger, 0.95);
        state.set_intensity(EmotionType::Hate, 0.8);
        dispatcher
            .repository_guard()
            .save_emotion_state("lin_chong", state);
    }

    // 4. 사전 axes (긍정 — "옛 친구")
    let initial_closeness = {
        let repo = repo_arc.lock().unwrap();
        repo.get_relationship("lin_chong", "lu_qian")
            .unwrap()
            .closeness()
            .value()
    };
    assert!(
        initial_closeness > 0.0,
        "초기 closeness 양수 (옛 친구 관계)"
    );

    // 5. dispatch
    let output = dispatcher
        .dispatch_v2(Command::EndDialogue {
            npc_id: "lin_chong".into(),
            partner_id: "lu_qian".into(),
            significance: None,
            reflection: Some(shanshenmiao_reflection()),
        })
        .await
        .expect("dispatch OK");

    // 6. 이벤트 시퀀스 — 4 follow-ups
    let kinds: Vec<EventKind> = output.events.iter().map(|e| e.kind()).collect();
    assert!(kinds.contains(&EventKind::DialogueReflected));
    assert!(
        kinds.contains(&EventKind::RelationshipUpdated),
        "★ 결단 — RelationshipUpdated 발행"
    );
    assert!(kinds.contains(&EventKind::EmotionCleared));
    assert!(kinds.contains(&EventKind::SceneEnded));
    assert_eq!(output.events.len(), 5, "총 5 이벤트");

    // 7. axes 큰 변화 — 양수 closeness가 음의 방향으로 강하게 이동.
    //    Hate + Anger 강도 + significance 0.92 → after_dialogue 함수가 음수로 끌어내림 기대.
    let after_relationship = {
        let repo = repo_arc.lock().unwrap();
        repo.get_relationship("lin_chong", "lu_qian")
            .unwrap()
            .clone()
    };
    let after_closeness = after_relationship.closeness().value();
    let delta = after_closeness - initial_closeness;
    assert!(
        delta < -0.05,
        "★ closeness 음의 방향 큰 변화 (배신·처단 → 적대): delta={delta} (initial={initial_closeness}, after={after_closeness})"
    );

    // 8. DialogueReflected의 significance_score 박제 검증
    let reflected = output
        .events
        .iter()
        .find(|e| e.kind() == EventKind::DialogueReflected)
        .expect("DialogueReflected 존재");
    if let EventPayload::DialogueReflected { result, .. } = &reflected.payload {
        assert!(result.significance_score >= 0.85);
        assert!(!result.is_chitchat);
        assert!(
            result
                .llm_reasoning
                .as_ref()
                .map(|s| s.contains("OCC peak"))
                .unwrap_or(false)
        );
    }
}
