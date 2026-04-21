//! Step C3 — Rumor Canonical 해소 규칙 통합 테스트 (§2.6 Canonical 해소표).
//!
//! 세 경로를 각각 검증:
//! | 상태 | topic | Canonical | seed_content | resolved content |
//! |---|---|---|---|---|
//! | 일반 Rumor | Some | **있음** | None | Canonical.content |
//! | 고아 Rumor | None | n/a | Some | seed_content |
//! | 예보된 사실 | Some | **없음** | Some | seed_content |
//!
//! Canonical은 `MemoryStore`에 `(provenance=Seeded, scope=World)`로 미리 index.

mod common;

use common::in_memory_rumor::InMemoryRumorStore;
use common::in_memory_store::InMemoryMemoryStore;
use npc_mind::application::command::{Command, CommandDispatcher};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::memory::{
    MemoryEntry, MemoryLayer, MemoryScope, MemorySource, MemoryType, Provenance,
};
use npc_mind::domain::personality::NpcBuilder;
use npc_mind::ports::{MemoryQuery, MemoryScopeFilter, MemoryStore, RumorStore};
use npc_mind::{
    InMemoryRepository, RumorOriginInput, RumorReachInput, SeedRumorRequest, SpreadRumorRequest,
};
use std::sync::Arc;

fn build(
    canonical: Option<MemoryEntry>,
) -> (
    CommandDispatcher<InMemoryRepository>,
    Arc<InMemoryMemoryStore>,
    Arc<InMemoryRumorStore>,
) {
    let mut repo = InMemoryRepository::new();
    for id in ["listener-a", "listener-b"] {
        repo.add_npc(NpcBuilder::new(id, id).build());
    }
    let event_store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let memory_store = Arc::new(InMemoryMemoryStore::new());
    let rumor_store = Arc::new(InMemoryRumorStore::new());

    // Canonical 선행 등록 (있을 경우)
    if let Some(c) = canonical {
        (&*memory_store as &dyn MemoryStore)
            .index(c, None)
            .unwrap();
    }

    let dispatcher = CommandDispatcher::new(repo, event_store, bus)
        .with_default_handlers()
        .with_memory(memory_store.clone() as Arc<dyn MemoryStore>)
        .with_rumor(
            memory_store.clone() as Arc<dyn MemoryStore>,
            rumor_store.clone() as Arc<dyn RumorStore>,
        );
    (dispatcher, memory_store, rumor_store)
}

fn recipient_entry(store: &dyn MemoryStore, npc: &str) -> MemoryEntry {
    store
        .search(MemoryQuery {
            scope_filter: Some(MemoryScopeFilter::Exact(MemoryScope::Personal {
                npc_id: npc.into(),
            })),
            limit: 10,
            ..Default::default()
        })
        .unwrap()
        .into_iter()
        .map(|r| r.entry)
        .next()
        .expect("entry must exist")
}

fn canonical_entry(topic: &str, content: &str) -> MemoryEntry {
    #[allow(deprecated)]
    MemoryEntry {
        id: format!("canon-{topic}"),
        created_seq: 0,
        event_id: 0,
        scope: MemoryScope::World {
            world_id: "jianghu".into(),
        },
        source: MemorySource::Experienced,
        provenance: Provenance::Seeded,
        memory_type: MemoryType::WorldEvent,
        layer: MemoryLayer::A,
        content: content.into(),
        topic: Some(topic.into()),
        emotional_context: None,
        timestamp_ms: 0,
        last_recalled_at: None,
        recall_count: 0,
        origin_chain: vec![],
        confidence: 1.0,
        acquired_by: None,
        superseded_by: None,
        consolidated_into: None,
        npc_id: "jianghu".into(),
    }
}

#[tokio::test]
async fn rumor_with_topic_and_canonical_resolves_to_canonical_content() {
    // 행 1: topic=Some, Canonical=있음, seed_content=None → Canonical
    let canonical = canonical_entry("leader-change", "공식 발표: 장문인 교체");
    let (dispatcher, memory_store, rumor_store) = build(Some(canonical));

    dispatcher
        .dispatch_v2(Command::SeedRumor(SeedRumorRequest {
            topic: Some("leader-change".into()),
            seed_content: None, // ← Canonical을 쓰므로 필요 없음
            reach: RumorReachInput::default(),
            origin: RumorOriginInput::Seeded,
        }))
        .await
        .unwrap();
    let rumor_id = rumor_store
        .find_by_topic("leader-change")
        .unwrap()
        .pop()
        .unwrap()
        .id;

    dispatcher
        .dispatch_v2(Command::SpreadRumor(SpreadRumorRequest {
            rumor_id,
            recipients: vec!["listener-a".into()],
            content_version: None,
        }))
        .await
        .unwrap();

    let entry = recipient_entry(&*memory_store, "listener-a");
    assert_eq!(entry.content, "공식 발표: 장문인 교체");
    assert_eq!(entry.topic.as_deref(), Some("leader-change"));
    assert_eq!(entry.source, MemorySource::Rumor);
}

#[tokio::test]
async fn orphan_rumor_resolves_to_seed_content() {
    // 행 2: topic=None, Canonical=n/a, seed_content=Some → seed_content
    let (dispatcher, memory_store, rumor_store) = build(None);

    dispatcher
        .dispatch_v2(Command::SeedRumor(SeedRumorRequest {
            topic: None,
            seed_content: Some("강호에 이상한 기운이 돈다".into()),
            reach: RumorReachInput::default(),
            origin: RumorOriginInput::Authored { by: None },
        }))
        .await
        .unwrap();

    // 고아 Rumor는 aggregate_key가 "orphan" 기반 — find_by_topic 대신 find_active_in_reach 사용
    let reach = npc_mind::domain::rumor::ReachPolicy::default();
    let rumor = rumor_store
        .find_active_in_reach(&reach)
        .unwrap()
        .pop()
        .unwrap();
    assert!(rumor.is_orphan());

    dispatcher
        .dispatch_v2(Command::SpreadRumor(SpreadRumorRequest {
            rumor_id: rumor.id,
            recipients: vec!["listener-a".into()],
            content_version: None,
        }))
        .await
        .unwrap();

    let entry = recipient_entry(&*memory_store, "listener-a");
    assert_eq!(entry.content, "강호에 이상한 기운이 돈다");
    assert!(entry.topic.is_none(), "orphan rumor entry has no topic");
}

#[tokio::test]
async fn forecast_rumor_with_topic_but_no_canonical_uses_seed_content() {
    // 행 3: topic=Some, Canonical=없음, seed_content=Some → seed_content
    let (dispatcher, memory_store, rumor_store) = build(None);

    dispatcher
        .dispatch_v2(Command::SeedRumor(SeedRumorRequest {
            topic: Some("future-event".into()),
            seed_content: Some("조만간 큰 사건이 있다더라".into()),
            reach: RumorReachInput::default(),
            origin: RumorOriginInput::Authored {
                by: Some("informant".into()),
            },
        }))
        .await
        .unwrap();
    let rumor_id = rumor_store
        .find_by_topic("future-event")
        .unwrap()
        .pop()
        .unwrap()
        .id;

    dispatcher
        .dispatch_v2(Command::SpreadRumor(SpreadRumorRequest {
            rumor_id,
            recipients: vec!["listener-a".into()],
            content_version: None,
        }))
        .await
        .unwrap();

    let entry = recipient_entry(&*memory_store, "listener-a");
    // Canonical이 아직 없으므로 seed_content가 사용됨
    assert_eq!(entry.content, "조만간 큰 사건이 있다더라");
    assert_eq!(entry.topic.as_deref(), Some("future-event"));
}
