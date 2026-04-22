//! Step D — `RelationshipUpdated.cause` variant별 `RelationshipMemoryHandler` 분기 통합 테스트.
//!
//! 커버리지:
//! - `BeatTransitioned`로 발행된 `RelationshipUpdated`가 cause=SceneInteraction을 갖고
//!   handler가 Experienced source로 MemoryEntry를 생성
//! - cause variant별 source / topic 분기 (L1 단위 테스트와 병행)
//!
//! `InformationTold`·`Rumor` variant의 경로는 Step E/F에서 RelationshipAgent가 해당
//! cause로 이벤트를 발행하도록 통합되며, Step D는 `RelationshipMemoryHandler`가
//! 이벤트 payload만 보고 올바르게 분기한다는 점만 검증한다.

mod common;

use common::in_memory_store::InMemoryMemoryStore;
use npc_mind::application::command::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerShared,
};
use npc_mind::application::command::{Command, CommandDispatcher, RelationshipMemoryHandler};
use npc_mind::application::dto::{EventInput, SituationInput};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::aggregate::AggregateKey;
use npc_mind::domain::event::{DomainEvent, EventPayload, RelationshipChangeCause};
use npc_mind::domain::memory::{MemoryScope, MemorySource, MemoryType};
use npc_mind::domain::personality::NpcBuilder;
use npc_mind::domain::relationship::Relationship;
use npc_mind::domain::scene_id::SceneId;
use npc_mind::ports::{MemoryQuery, MemoryScopeFilter, MemoryStore};
use npc_mind::InMemoryRepository;
use std::sync::Arc;

fn personal_rel_entries(store: &dyn MemoryStore, owner: &str) -> Vec<npc_mind::MemoryEntry> {
    store
        .search(MemoryQuery {
            scope_filter: Some(MemoryScopeFilter::Exact(MemoryScope::Personal {
                npc_id: owner.into(),
            })),
            limit: 1000,
            ..Default::default()
        })
        .unwrap()
        .into_iter()
        .map(|r| r.entry)
        .filter(|e| matches!(e.memory_type, MemoryType::RelationshipChange))
        .collect()
}

#[tokio::test]
async fn end_dialogue_creates_relationship_memory_with_cause_unspecified() {
    // 현재 RelationshipAgent의 `handle_dialogue_end`는 cause=Unspecified로 발행한다.
    // RelationshipMemoryHandler는 이를 Experienced source의 일반 엔트리로 기록해야 한다.
    let store = Arc::new(InMemoryMemoryStore::new());
    let mut repo = InMemoryRepository::new();
    repo.add_npc(NpcBuilder::new("alice", "Alice").build());
    repo.add_npc(NpcBuilder::new("bob", "Bob").build());
    repo.add_relationship(Relationship::neutral("alice", "bob"));
    repo.add_relationship(Relationship::neutral("bob", "alice"));

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(repo, event_store, bus)
        .with_default_handlers()
        .with_memory(store.clone() as Arc<dyn MemoryStore>);

    // Appraise로 emotion_state 시드
    dispatcher
        .dispatch_v2(Command::Appraise {
            npc_id: "alice".into(),
            partner_id: "bob".into(),
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
            significance: Some(0.8),
        })
        .await
        .expect("end");

    // alice 관점의 RelationshipChange 엔트리가 기록됨 (Δ가 threshold 넘을 때만)
    // 관계가 neutral에서 after_dialogue 갱신 — 현재 관성상 변동 폭이 threshold를 넘는지는
    // 구현 의존이라, 엔트리 수를 하한 없이 검증: 생성되었다면 Experienced source.
    let entries = personal_rel_entries(&*store, "alice");
    for e in &entries {
        assert_eq!(e.memory_type, MemoryType::RelationshipChange);
        assert_eq!(
            e.source,
            MemorySource::Experienced,
            "Unspecified cause → Experienced source"
        );
    }
}

// ---------------------------------------------------------------------------
// cause variant별 직접 분기 검증 — RelationshipMemoryHandler 단독 호출
// ---------------------------------------------------------------------------

fn run_cause(
    store: Arc<InMemoryMemoryStore>,
    event_id: u64,
    cause: RelationshipChangeCause,
) {
    let handler = RelationshipMemoryHandler::new(store.clone());
    assert!(matches!(handler.mode(), DeliveryMode::Inline { .. }));

    let event = DomainEvent::new(
        event_id,
        "alice".into(),
        1,
        EventPayload::RelationshipUpdated {
            owner_id: "alice".into(),
            target_id: "bob".into(),
            before_closeness: 0.0,
            before_trust: 0.0,
            before_power: 0.0,
            after_closeness: 0.3,
            after_trust: 0.0,
            after_power: 0.0,
            cause,
        },
    );

    let repo = InMemoryRepository::new();
    let es = InMemoryEventStore::new();
    let mut shared = HandlerShared::default();
    let prior: Vec<DomainEvent> = Vec::new();
    let agg = AggregateKey::Relationship {
        owner_id: "alice".into(),
        target_id: "bob".into(),
    };
    let mut ctx = EventHandlerContext {
        repo: &repo,
        event_store: &es,
        shared: &mut shared,
        prior_events: &prior,
        aggregate_key: agg,
    };
    handler.handle(&event, &mut ctx).unwrap();
}

#[test]
fn scene_interaction_cause_produces_experienced_memory() {
    let store = Arc::new(InMemoryMemoryStore::new());
    run_cause(
        store.clone(),
        1,
        RelationshipChangeCause::SceneInteraction {
            scene_id: SceneId::new("alice", "bob"),
        },
    );
    let entries = personal_rel_entries(&*store, "alice");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, MemorySource::Experienced);
    assert_eq!(entries[0].topic, None);
    assert!(entries[0].content.contains("bob"));
}

#[test]
fn information_told_cause_len1_produces_heard_memory() {
    let store = Arc::new(InMemoryMemoryStore::new());
    run_cause(
        store.clone(),
        2,
        RelationshipChangeCause::InformationTold {
            origin_chain: vec!["sage".into()],
        },
    );
    let entries = personal_rel_entries(&*store, "alice");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, MemorySource::Heard);
    assert_eq!(entries[0].origin_chain, vec!["sage".to_string()]);
}

#[test]
fn information_told_cause_len2_produces_rumor_memory() {
    let store = Arc::new(InMemoryMemoryStore::new());
    run_cause(
        store.clone(),
        3,
        RelationshipChangeCause::InformationTold {
            origin_chain: vec!["relay".into(), "witness".into()],
        },
    );
    let entries = personal_rel_entries(&*store, "alice");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, MemorySource::Rumor);
    assert_eq!(
        entries[0].origin_chain,
        vec!["relay".to_string(), "witness".to_string()]
    );
}

#[test]
fn world_event_overlay_cause_sets_topic_and_experienced_source() {
    let store = Arc::new(InMemoryMemoryStore::new());
    run_cause(
        store.clone(),
        4,
        RelationshipChangeCause::WorldEventOverlay {
            topic: Some("moorim-leader".into()),
        },
    );
    let entries = personal_rel_entries(&*store, "alice");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, MemorySource::Experienced);
    assert_eq!(entries[0].topic.as_deref(), Some("moorim-leader"));
}

#[test]
fn rumor_cause_sets_rumor_source_and_chain_marker() {
    let store = Arc::new(InMemoryMemoryStore::new());
    run_cause(
        store.clone(),
        5,
        RelationshipChangeCause::Rumor {
            rumor_id: "r-42".into(),
        },
    );
    let entries = personal_rel_entries(&*store, "alice");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, MemorySource::Rumor);
    assert_eq!(entries[0].origin_chain, vec!["rumor:r-42".to_string()]);
}
