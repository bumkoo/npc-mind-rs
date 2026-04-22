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
use npc_mind::ports::{MemoryQuery, MemoryScopeFilter, MemoryStore};
use npc_mind::application::dto::{EventInput, SituationInput};
use npc_mind::InMemoryRepository;
use std::sync::Arc;

/// Appraise를 수동 호출해 emotion_state를 초기화한다. EndDialogue 사전 조건.
async fn seed_emotion(dispatcher: &CommandDispatcher<InMemoryRepository>, npc: &str, partner: &str) {
    dispatcher
        .dispatch_v2(Command::Appraise {
            npc_id: npc.into(),
            partner_id: partner.into(),
            situation: Some(SituationInput {
                description: "장면 준비".into(),
                event: Some(EventInput {
                    description: "평범한 만남".into(),
                    desirability_for_self: 0.2,
                    other: None,
                    prospect: None,
                }),
                action: None,
                object: None,
            }),
        })
        .await
        .expect("appraise seed");
}

fn dispatcher_with_memory(
    store: Arc<InMemoryMemoryStore>,
) -> CommandDispatcher<InMemoryRepository> {
    let mut repo = InMemoryRepository::new();
    repo.add_npc(NpcBuilder::new("alice", "Alice").build());
    repo.add_npc(NpcBuilder::new("bob", "Bob").build());
    repo.add_relationship(Relationship::neutral("alice", "bob"));
    repo.add_relationship(Relationship::neutral("bob", "alice"));

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    CommandDispatcher::new(repo, event_store, bus)
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

fn personal_entries(store: &dyn MemoryStore, npc: &str) -> Vec<MemoryEntry> {
    store
        .search(MemoryQuery {
            scope_filter: Some(MemoryScopeFilter::Exact(MemoryScope::Personal {
                npc_id: npc.into(),
            })),
            limit: 1000,
            ..Default::default()
        })
        .unwrap()
        .into_iter()
        .map(|r| r.entry)
        .collect()
}

#[tokio::test]
async fn scene_end_consolidates_layer_a_into_layer_b_summary() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let dispatcher = dispatcher_with_memory(store.clone());

    // 10턴짜리 Scene — alice·bob 양쪽에 고르게 시드
    for i in 0..10 {
        let (npc, partner) = if i % 2 == 0 {
            ("alice", "bob")
        } else {
            ("bob", "alice")
        };
        seed_turn(
            &*store,
            &format!("turn-{i}"),
            npc,
            &format!("{partner}와 나눈 {i}번째 대화"),
            (i + 1) as u64,
        );
    }
    // Appraise(seed emotion) → Scene → EndDialogue 순서.
    seed_emotion(&dispatcher, "alice", "bob").await;
    dispatcher
        .dispatch_v2(Command::StartScene {
            npc_id: "alice".into(),
            partner_id: "bob".into(),
            significance: Some(0.5),
            focuses: vec![],
        })
        .await
        .expect("start scene");

    dispatcher
        .dispatch_v2(Command::EndDialogue {
            npc_id: "alice".into(),
            partner_id: "bob".into(),
            significance: Some(0.5),
        })
        .await
        .expect("end dialogue must succeed");

    // 리뷰 B3: per-NPC Personal Summary. 각 NPC 관점에서 자기 Summary가 1개씩.
    let alice_entries = personal_entries(&*store, "alice");
    let bob_entries = personal_entries(&*store, "bob");
    let alice_summary = alice_entries
        .iter()
        .find(|e| matches!(e.memory_type, MemoryType::SceneSummary))
        .expect("alice summary");
    let bob_summary = bob_entries
        .iter()
        .find(|e| matches!(e.memory_type, MemoryType::SceneSummary))
        .expect("bob summary");

    assert_eq!(alice_summary.layer, MemoryLayer::B);
    assert_eq!(bob_summary.layer, MemoryLayer::B);
    assert_ne!(alice_summary.id, bob_summary.id, "서로 다른 엔트리");
    // 리뷰 M7: topic "scene:{a}:{b}" (정규화)
    assert_eq!(alice_summary.topic.as_deref(), Some("scene:alice:bob"));
    assert_eq!(bob_summary.topic.as_deref(), Some("scene:alice:bob"));

    // alice 관점 Layer A 엔트리는 alice summary를 가리킨다
    let alice_turns: Vec<_> = alice_entries
        .iter()
        .filter(|e| matches!(e.memory_type, MemoryType::DialogueTurn))
        .collect();
    assert!(!alice_turns.is_empty());
    for e in &alice_turns {
        assert_eq!(
            e.consolidated_into.as_deref(),
            Some(alice_summary.id.as_str()),
            "alice Layer A({})는 alice summary를 가리켜야",
            e.id
        );
    }
    // bob 관점 Layer A도 bob summary를 가리킨다
    let bob_turns: Vec<_> = bob_entries
        .iter()
        .filter(|e| matches!(e.memory_type, MemoryType::DialogueTurn))
        .collect();
    assert!(!bob_turns.is_empty());
    for e in &bob_turns {
        assert_eq!(
            e.consolidated_into.as_deref(),
            Some(bob_summary.id.as_str()),
            "bob Layer A({})는 bob summary를 가리켜야 (리뷰 B3 관점 분리)",
            e.id
        );
    }
}

#[tokio::test]
async fn scene_with_no_layer_a_entries_has_no_summary() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let dispatcher = dispatcher_with_memory(store.clone());

    // Scene 시작·종료만, 턴 시드 없음
    seed_emotion(&dispatcher, "alice", "bob").await;
    dispatcher
        .dispatch_v2(Command::StartScene {
            npc_id: "alice".into(),
            partner_id: "bob".into(),
            significance: Some(0.5),
            focuses: vec![],
        })
        .await
        .unwrap();
    dispatcher
        .dispatch_v2(Command::EndDialogue {
            npc_id: "alice".into(),
            partner_id: "bob".into(),
            significance: Some(0.5),
        })
        .await
        .unwrap();

    // SceneConsolidation이 Layer A 엔트리를 찾지 못해 요약을 생성하지 않는다.
    // 다만 RelationshipMemoryHandler가 Δ가 threshold 넘을 경우 엔트리를 만들 수 있으므로,
    // SceneSummary(Layer B) 엔트리는 0이어야 함을 검증한다.
    let entries = personal_entries(&*store, "alice");
    let summaries = entries
        .iter()
        .filter(|e| matches!(e.memory_type, MemoryType::SceneSummary))
        .count();
    assert_eq!(summaries, 0, "Layer A 없으면 SceneSummary도 만들지 않음");
}

#[tokio::test]
async fn relationship_change_type_not_consolidated() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let dispatcher = dispatcher_with_memory(store.clone());

    // 1. 일반 DialogueTurn 1개 시드
    seed_turn(&*store, "t1", "alice", "일반 턴", 1);
    // 2. RelationshipChange 타입 1개 시드 — 직접 MemoryEntry::personal로
    let rel = MemoryEntry::personal(
        "r1",
        "alice",
        "관계 변화",
        None,
        1,
        1,
        MemoryType::RelationshipChange,
    );
    store.index(rel, None).unwrap();

    seed_emotion(&dispatcher, "alice", "bob").await;
    dispatcher
        .dispatch_v2(Command::StartScene {
            npc_id: "alice".into(),
            partner_id: "bob".into(),
            significance: Some(0.5),
            focuses: vec![],
        })
        .await
        .unwrap();
    dispatcher
        .dispatch_v2(Command::EndDialogue {
            npc_id: "alice".into(),
            partner_id: "bob".into(),
            significance: Some(0.5),
        })
        .await
        .unwrap();

    let all = personal_entries(&*store, "alice");
    let rel_e = all.iter().find(|e| e.id == "r1").unwrap();
    assert!(
        rel_e.consolidated_into.is_none(),
        "RelationshipChange는 Consolidation 대상 아님"
    );
    let turn_e = all.iter().find(|e| e.id == "t1").unwrap();
    assert!(turn_e.consolidated_into.is_some(), "DialogueTurn은 흡수");
}
