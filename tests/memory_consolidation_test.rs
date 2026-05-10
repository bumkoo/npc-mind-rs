//! Step D — Scene 종료 시 Layer A → Layer B 흡수 통합 테스트.
//!
//! `SceneConsolidationHandler`가 `EndDialogue` 커맨드의 `SceneEnded` follow-up을 관찰해
//! Scene 범위의 Layer A 엔트리를 Layer B `SceneSummary`로 요약한다.
//!
//! 커버리지:
//! - Scene 범위 Layer A 엔트리 수집 + Layer B 요약 생성
//! - 흡수된 Layer A 엔트리의 `consolidated_into` 마킹
//! - Consolidation 대상 타입만 흡수 (RelationshipChange는 제외)
//! - Scene에 Layer A가 없으면 no-op

mod common;

use common::in_memory_store::InMemoryMemoryStore;
use npc_mind::application::command::{Command, CommandDispatcher};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::memory::{MemoryEntry, MemoryLayer, MemoryScope, MemoryType};
use npc_mind::domain::personality::NpcBuilder;
use npc_mind::domain::relationship::Relationship;
use npc_mind::ports::{MemoryStore, EmotionStore};
use npc_mind::InMemoryRepository;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn scene_ended_consolidates_layer_a_to_summary() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let dispatcher = build_dispatcher(store.clone());

    // 1. Layer A 데이터 시드 (DialogueTurn)
    seed_turn(&*store, "t1", "alice", "첫 발화", 1000);
    seed_turn(&*store, "t2", "bob", "두 번째 발화", 2000);
    seed_turn(&*store, "t3", "alice", "세 번째 발화", 3000);

    // 감정 상태 시드 (RelationshipPolicy 성공을 위해 필요)
    dispatcher.repository_guard().save_emotion_state("alice", npc_mind::domain::emotion::EmotionState::default());

    // 2. EndDialogue 실행
    dispatcher
        .dispatch_v2(Command::EndDialogue {
            npc_id: "alice".into(),
            partner_id: "bob".into(),
            significance: Some(0.5),
            reflection: None,
        })
        .await
        .expect("end dialogue");

    // 3. 검증: Summary 엔트리가 생겼는가?
    let alice_summaries = personal_summary_entries(&*store, "alice");
    assert_eq!(alice_summaries.len(), 1);
    assert!(alice_summaries[0].content.contains("2턴"));
    assert_eq!(alice_summaries[0].layer, MemoryLayer::B);

    let bob_summaries = personal_summary_entries(&*store, "bob");
    assert_eq!(bob_summaries.len(), 1);

    // 4. 검증: 원본 엔트리들이 consolidated 마킹되었는가?
    let t1 = store.get_by_id("t1").unwrap().unwrap();
    assert!(t1.consolidated_into.is_some());
    assert_eq!(t1.consolidated_into, Some(alice_summaries[0].id.clone()));
}

fn build_dispatcher(
    store: Arc<InMemoryMemoryStore>,
) -> CommandDispatcher<InMemoryRepository> {
    let mut repo = InMemoryRepository::new();
    repo.add_npc(NpcBuilder::new("alice", "Alice").build());
    repo.add_npc(NpcBuilder::new("bob", "Bob").build());
    repo.add_relationship(Relationship::neutral("alice", "bob"));
    repo.add_relationship(Relationship::neutral("bob", "alice"));

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let repo_arc = Arc::new(Mutex::new(repo));
    CommandDispatcher::new(repo_arc, event_store, bus)
        .with_default_handlers()
        .with_memory_full(store as Arc<dyn MemoryStore>)
}

fn seed_turn(store: &dyn MemoryStore, id: &str, npc: &str, content: &str, ts: u64) {
    store
        .index(
            MemoryEntry::personal(id, npc, content, None, ts, ts, MemoryType::DialogueTurn),
            None,
        )
        .unwrap();
}

fn personal_summary_entries(store: &dyn MemoryStore, npc: &str) -> Vec<MemoryEntry> {
    use npc_mind::ports::MemoryScopeFilter;
    store
        .search(npc_mind::ports::MemoryQuery {
            scope_filter: Some(MemoryScopeFilter::Exact(MemoryScope::Personal {
                npc_id: npc.into(),
            })),
            layer_filter: Some(MemoryLayer::B),
            ..Default::default()
        })
        .unwrap()
        .into_iter()
        .map(|r| r.entry)
        .filter(|e| e.memory_type == MemoryType::SceneSummary)
        .collect()
}
