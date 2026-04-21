//! Step C2 — TellInformation 커맨드 경로 end-to-end 통합 테스트.
//!
//! 커버리지:
//! - 청자당 1 `InformationTold` 팬아웃 (B5)
//! - `origin_chain_in` 길이 → Heard/Rumor 분기 (§7.1)
//! - `stated_confidence × normalized_trust` 신뢰도 계산
//! - `ListenerRole::Overhearer`가 동일 처리 경로를 탐
//! - `with_memory()` 미부착 시 MemoryStore 저장은 건너뛰되 이벤트는 여전히 발행됨
//!
//! `chat` feature 불필요, `embed` feature 불필요 — 테스트 전용 InMemoryMemoryStore 사용.

mod common;

use common::in_memory_store::InMemoryMemoryStore;
use npc_mind::application::command::{Command, CommandDispatcher};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::EventStore;
use npc_mind::domain::event::{EventKind, EventPayload, ListenerRole};
use npc_mind::domain::memory::{MemoryScope, MemorySource};
use npc_mind::domain::personality::{NpcBuilder, Score};
use npc_mind::domain::relationship::Relationship;
use npc_mind::ports::{MemoryQuery, MemoryScopeFilter, MemoryStore};
use npc_mind::{InMemoryRepository, TellInformationRequest};
use std::sync::Arc;

fn make_npc(id: &str) -> npc_mind::domain::personality::Npc {
    NpcBuilder::new(id, id).build()
}

fn trust_rel(owner: &str, target: &str, trust: f32) -> Relationship {
    Relationship::new(
        owner,
        target,
        Score::new(0.0, "closeness").unwrap(),
        Score::new(trust, "trust").unwrap(),
        Score::new(0.0, "power").unwrap(),
    )
}

fn build_dispatcher(
    store: Arc<InMemoryMemoryStore>,
) -> (
    CommandDispatcher<InMemoryRepository>,
    Arc<InMemoryEventStore>,
) {
    let mut repo = InMemoryRepository::new();
    repo.add_npc(make_npc("sage"));
    repo.add_npc(make_npc("pupil"));
    repo.add_npc(make_npc("wanderer"));
    repo.add_npc(make_npc("relay"));
    repo.add_npc(make_npc("final_listener"));
    // 기본 중립 관계 (청자 → 화자)
    repo.add_relationship(Relationship::neutral("pupil", "sage"));
    repo.add_relationship(Relationship::neutral("wanderer", "sage"));
    repo.add_relationship(Relationship::neutral("final_listener", "relay"));

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(repo, event_store.clone(), bus)
        .with_default_handlers()
        .with_memory(store.clone() as Arc<dyn MemoryStore>);

    (dispatcher, event_store)
}

fn collect_told(store: &InMemoryEventStore) -> Vec<EventPayload> {
    store
        .get_all_events()
        .into_iter()
        .filter(|e| e.kind() == EventKind::InformationTold)
        .map(|e| e.payload)
        .collect()
}

fn listener_entries(store: &dyn MemoryStore, listener: &str) -> Vec<npc_mind::MemoryEntry> {
    let results = store
        .search(MemoryQuery {
            scope_filter: Some(MemoryScopeFilter::Exact(MemoryScope::Personal {
                npc_id: listener.into(),
            })),
            limit: 100,
            ..Default::default()
        })
        .unwrap();
    results.into_iter().map(|r| r.entry).collect()
}

#[tokio::test]
async fn tell_information_emits_one_told_event_per_listener() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, event_store) = build_dispatcher(store.clone());

    let _ = dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into(), "wanderer".into()],
            overhearers: vec![],
            claim: "장문인이 바뀐다".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        }))
        .await
        .expect("dispatch must succeed");

    let tolds = collect_told(&event_store);
    assert_eq!(tolds.len(), 2, "B5: one InformationTold per listener");
    for p in tolds {
        let EventPayload::InformationTold { listener_role, .. } = p else {
            unreachable!()
        };
        assert_eq!(listener_role, ListenerRole::Direct);
    }
}

#[tokio::test]
async fn direct_speaker_creates_heard_memory_in_listener_scope() {
    // 화자가 직접 경험을 말함 (origin_chain_in = []) → 청자 chain = [speaker], len=1 → Heard
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, _) = build_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into()],
            overhearers: vec![],
            claim: "직접 목격한 사건".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        }))
        .await
        .unwrap();

    let entries = listener_entries(&*store, "pupil");
    assert_eq!(entries.len(), 1);
    let e = &entries[0];
    assert_eq!(e.source, MemorySource::Heard);
    assert_eq!(e.content, "직접 목격한 사건");
    assert_eq!(e.origin_chain, vec!["sage".to_string()]);
    assert!(matches!(
        e.scope,
        MemoryScope::Personal { ref npc_id } if npc_id == "pupil"
    ));
}

#[tokio::test]
async fn relayed_information_classified_as_rumor_when_chain_length_two_or_more() {
    // 'relay'가 이미 들은 정보를 'final_listener'에게 전달.
    // 청자 chain = [relay, original_witness], len=2 → Rumor.
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, _) = build_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "relay".into(),
            listeners: vec!["final_listener".into()],
            overhearers: vec![],
            claim: "건너건너 들은 이야기".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec!["original_witness".into()],
            topic: None,
        }))
        .await
        .unwrap();

    let entries = listener_entries(&*store, "final_listener");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, MemorySource::Rumor);
    assert_eq!(
        entries[0].origin_chain,
        vec!["relay".to_string(), "original_witness".to_string()]
    );
}

#[tokio::test]
async fn confidence_multiplies_stated_by_normalized_trust() {
    // pupil의 sage에 대한 trust = 0.6 → normalized = 0.8
    // stated = 0.5 → confidence = 0.4
    let store = Arc::new(InMemoryMemoryStore::new());

    let mut repo = InMemoryRepository::new();
    repo.add_npc(make_npc("sage"));
    repo.add_npc(make_npc("pupil"));
    repo.add_relationship(trust_rel("pupil", "sage", 0.6));

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(repo, event_store.clone(), bus)
        .with_default_handlers()
        .with_memory(store.clone() as Arc<dyn MemoryStore>);

    dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into()],
            overhearers: vec![],
            claim: "반만 믿는 이야기".into(),
            stated_confidence: 0.5,
            origin_chain_in: vec![],
            topic: None,
        }))
        .await
        .unwrap();

    let entries = listener_entries(&*store, "pupil");
    assert_eq!(entries.len(), 1);
    assert!(
        (entries[0].confidence - 0.4).abs() < 1e-6,
        "expected ~0.4 (0.5 × 0.8), got {}",
        entries[0].confidence
    );
}

#[tokio::test]
async fn overhearers_receive_distinct_told_events_with_overhearer_role() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, event_store) = build_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into()],
            overhearers: vec!["wanderer".into()],
            claim: "엿들을 수도 있는 이야기".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        }))
        .await
        .unwrap();

    let tolds = collect_told(&event_store);
    assert_eq!(tolds.len(), 2);
    let roles: Vec<ListenerRole> = tolds
        .iter()
        .map(|p| match p {
            EventPayload::InformationTold { listener_role, .. } => *listener_role,
            _ => unreachable!(),
        })
        .collect();
    assert!(roles.contains(&ListenerRole::Direct));
    assert!(roles.contains(&ListenerRole::Overhearer));

    // 두 청자 모두 각자의 scope에 MemoryEntry가 만들어져야 한다.
    assert_eq!(listener_entries(&*store, "pupil").len(), 1);
    assert_eq!(listener_entries(&*store, "wanderer").len(), 1);
}

#[tokio::test]
async fn without_memory_builder_events_still_fire_but_no_memory_stored() {
    // with_memory() 미부착 — 이벤트 스트림만 흐르고 MemoryStore 저장은 없어야 함.
    let mut repo = InMemoryRepository::new();
    repo.add_npc(make_npc("sage"));
    repo.add_npc(make_npc("pupil"));
    repo.add_relationship(Relationship::neutral("pupil", "sage"));

    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(repo, event_store.clone(), bus)
        .with_default_handlers();
    // 주의: with_memory 호출 없음.

    dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into()],
            overhearers: vec![],
            claim: "저장 안 되는 이야기".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        }))
        .await
        .unwrap();

    // InformationTold는 발행된다
    let tolds = collect_told(&event_store);
    assert_eq!(tolds.len(), 1);

    // 그러나 MemoryStore에 저장되지 않는다 (외부 소비자가 없음)
    let external_store = InMemoryMemoryStore::new();
    assert_eq!(external_store.count(), 0);
}

#[tokio::test]
async fn empty_listeners_no_events_no_memory() {
    // 청자 0 → follow-up 0, 커맨드 자체는 성공 (초기 *Requested는 여전히 commit).
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, event_store) = build_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec![],
            overhearers: vec![],
            claim: "아무도 안 듣는 이야기".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        }))
        .await
        .unwrap();

    let tolds = collect_told(&event_store);
    assert!(tolds.is_empty());
    assert_eq!(store.count(), 0);
}

// ---------------------------------------------------------------------------
// Post-review fixes: C1 라우팅, C2 dedup, M1 결정적 id, M3 topic, M7 budget
// ---------------------------------------------------------------------------

#[tokio::test]
async fn information_told_stored_under_listener_aggregate_id_not_speaker() {
    // C1 회귀 가드 — commit_staging_buffer가 payload의 aggregate_key를 보존해야
    // get_events(listener)로 InformationTold를 찾을 수 있다 (§3.3 B5).
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, event_store) = build_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into(), "wanderer".into()],
            overhearers: vec![],
            claim: "x".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        }))
        .await
        .unwrap();

    let pupil_events = event_store.get_events("pupil");
    let wanderer_events = event_store.get_events("wanderer");
    let sage_events = event_store.get_events("sage");

    // 각 청자 자신의 aggregate 아래에 InformationTold가 정확히 1개.
    assert_eq!(
        pupil_events
            .iter()
            .filter(|e| e.kind() == EventKind::InformationTold)
            .count(),
        1,
        "pupil aggregate에서 InformationTold 1개 발견되어야 함"
    );
    assert_eq!(
        wanderer_events
            .iter()
            .filter(|e| e.kind() == EventKind::InformationTold)
            .count(),
        1
    );
    // speaker 쪽에는 InformationTold가 없어야 하지만 TellInformationRequested는 있다.
    assert_eq!(
        sage_events
            .iter()
            .filter(|e| e.kind() == EventKind::InformationTold)
            .count(),
        0,
        "speaker aggregate에는 InformationTold가 없어야 함"
    );
    assert!(sage_events
        .iter()
        .any(|e| e.kind() == EventKind::TellInformationRequested));
}

#[tokio::test]
async fn listeners_overhearers_overlap_deduped_to_single_entry_as_direct() {
    // C2 회귀 가드 — 같은 NPC가 listeners와 overhearers에 모두 있으면 Direct 하나만 발행.
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, event_store) = build_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into()],
            overhearers: vec!["pupil".into(), "wanderer".into()],
            claim: "one-per-listener".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        }))
        .await
        .unwrap();

    let tolds = collect_told(&event_store);
    assert_eq!(tolds.len(), 2, "pupil dedup → pupil(Direct) + wanderer(Overhearer) = 2");

    // pupil 한 명당 MemoryEntry도 1개만.
    assert_eq!(listener_entries(&*store, "pupil").len(), 1);
    assert_eq!(listener_entries(&*store, "wanderer").len(), 1);

    let pupil_role = tolds
        .iter()
        .find_map(|p| match p {
            EventPayload::InformationTold {
                listener,
                listener_role,
                ..
            } if listener == "pupil" => Some(*listener_role),
            _ => None,
        })
        .unwrap();
    assert_eq!(pupil_role, ListenerRole::Direct, "Direct가 Overhearer를 덮어써야 함");
}

#[tokio::test]
async fn memory_entry_id_is_deterministic_from_event_id_and_listener() {
    // M1 회귀 가드 — id가 event.id + listener로 생성되어 카운터 레이스에 의존하지 않음.
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, _) = build_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into(), "wanderer".into()],
            overhearers: vec![],
            claim: "x".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        }))
        .await
        .unwrap();

    let pupil_entry = listener_entries(&*store, "pupil").pop().unwrap();
    let wanderer_entry = listener_entries(&*store, "wanderer").pop().unwrap();

    // 같은 포맷 `mem-{event_id:012}-{listener}`.
    assert!(pupil_entry.id.starts_with("mem-"));
    assert!(pupil_entry.id.ends_with("-pupil"));
    assert!(wanderer_entry.id.ends_with("-wanderer"));
    assert_ne!(pupil_entry.id, wanderer_entry.id);
}

#[tokio::test]
async fn topic_is_threaded_from_dto_through_event_to_memory_entry() {
    // M3 회귀 가드 — DTO.topic → TellInformationRequested.topic → InformationTold.topic
    //                 → MemoryEntry.topic 까지 보존.
    let store = Arc::new(InMemoryMemoryStore::new());
    let (dispatcher, event_store) = build_dispatcher(store.clone());

    dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: vec!["pupil".into()],
            overhearers: vec![],
            claim: "맹주가 바뀐다".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: Some("moorim-leader-change".into()),
        }))
        .await
        .unwrap();

    // InformationTold payload
    let told = collect_told(&event_store).pop().unwrap();
    let EventPayload::InformationTold { topic, .. } = told else {
        panic!("expected InformationTold");
    };
    assert_eq!(topic.as_deref(), Some("moorim-leader-change"));

    // MemoryEntry.topic
    let entry = listener_entries(&*store, "pupil").pop().unwrap();
    assert_eq!(entry.topic.as_deref(), Some("moorim-leader-change"));
}

#[tokio::test]
async fn event_budget_exceeded_when_recipients_plus_initial_exceed_max_events_per_command() {
    // M7 회귀 가드 — MAX_EVENTS_PER_COMMAND=20 경계.
    // staging_buffer.len() >= 20 체크는 초기 이벤트 1 + follow-up N을 모두 카운트하므로
    // 총 합 ≤ 20이어야 한다. 청자 20명 → 초기 1 + 20 follow-up = 21 → EventBudgetExceeded.
    let store = Arc::new(InMemoryMemoryStore::new());

    let mut repo = InMemoryRepository::new();
    repo.add_npc(make_npc("sage"));
    for i in 0..20 {
        let id = format!("listener-{i}");
        repo.add_npc(make_npc(&id));
        repo.add_relationship(Relationship::neutral(&id, "sage"));
    }
    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(repo, event_store.clone(), bus)
        .with_default_handlers()
        .with_memory(store.clone() as Arc<dyn MemoryStore>);

    // 19명 → 초기 1 + 19 follow-up = 20, 통과
    let listeners_19: Vec<String> = (0..19).map(|i| format!("listener-{i}")).collect();
    let ok = dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: listeners_19,
            overhearers: vec![],
            claim: "ok boundary".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        }))
        .await;
    assert!(ok.is_ok(), "19 청자는 경계 내 — 성공해야 함: {ok:?}");

    // 20명 → 초기 1 + 20 follow-up = 21, 실패
    let listeners_20: Vec<String> = (0..20).map(|i| format!("listener-{i}")).collect();
    let err = dispatcher
        .dispatch_v2(Command::TellInformation(TellInformationRequest {
            speaker: "sage".into(),
            listeners: listeners_20,
            overhearers: vec![],
            claim: "over budget".into(),
            stated_confidence: 1.0,
            origin_chain_in: vec![],
            topic: None,
        }))
        .await
        .expect_err("20 청자는 예산 초과로 실패해야 함");
    // Display 문자열 또는 Debug에서 "budget" 키워드 확인
    let msg = format!("{err:?}");
    assert!(
        msg.contains("EventBudget") || msg.contains("budget"),
        "예산 초과 에러 기대, 실제: {msg}"
    );
}
