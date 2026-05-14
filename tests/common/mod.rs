//! 테스트 공통 유틸리티
//!
//! 무협 4인 캐릭터 빌더 + Score 헬퍼 + 관계 팩토리 + 테스트 컨텍스트
//! + 테스트 전용 InMemoryMemoryStore (in_memory_store 모듈)

#![allow(dead_code)]

pub mod in_memory_rumor;
pub mod in_memory_store;
#[cfg(feature = "chat")]
pub mod mock_chat;

use npc_mind::InMemoryRepository;
use npc_mind::domain::emotion::{
    ActionFocus, EmotionState, EmotionType, EventFocus, RelationshipModifiers, Situation,
};
use npc_mind::domain::personality::*;
use npc_mind::domain::relationship::{AxisScore, Relationship, WarinessScore};
use std::sync::{Arc, Mutex};

/// MockRepository는 InMemoryRepository의 별칭입니다 (기존 테스트 호환).
pub type MockRepository = InMemoryRepository;

pub fn score(v: f32) -> Score {
    Score::new(v, "").unwrap()
}

/// Phase 2 Stage 1 헬퍼 — ±1.0 정규화 값을 ±100 `AxisScore`로 변환.
/// 기존 `s(0.9)` 패턴이 `.closeness/.trust(...)`에 박혀 있던 *±1.0 의미*를 보존하면서
/// 4축 빌더에 자연스럽게 박힌다 (예: `.affinity(axis(0.9))`).
pub fn axis(v: f32) -> AxisScore {
    AxisScore::new(v * 100.0)
}

/// Phase 2 Stage 1 헬퍼 — ±100 절대값을 `AxisScore`로 직접 변환.
pub fn axis_pct(v: f32) -> AxisScore {
    AxisScore::new(v)
}

/// Phase 2 Stage 1 헬퍼 — ±1.0 정규화 값을 0..=100 `WarinessScore`로 변환.
pub fn wariness(v: f32) -> WarinessScore {
    WarinessScore::new(v * 100.0)
}

// ---------------------------------------------------------------------------
// 감정 헬퍼
// ---------------------------------------------------------------------------

pub fn find_emotion(state: &EmotionState, etype: EmotionType) -> Option<f32> {
    state
        .emotions()
        .iter()
        .find(|e| e.emotion_type() == etype)
        .map(|e| e.intensity())
}

pub fn has_emotion(state: &EmotionState, etype: EmotionType) -> bool {
    find_emotion(state, etype).is_some()
}

// ---------------------------------------------------------------------------
// 시나리오 헬퍼
// ---------------------------------------------------------------------------

/// 배신 상황 (desirability: -0.6, praiseworthiness: -0.7)
pub fn 배신_상황() -> Situation {
    배신_상황_with_desc("배신")
}

pub fn 배신_상황_with_desc(description: &str) -> Situation {
    Situation::new(
        description,
        Some(EventFocus {
            description: "".into(),
            desirability_for_self: -0.6,
            desirability_for_other: None,
            prospect: None,
        }),
        Some(ActionFocus {
            description: "".into(),
            agent_id: Some("partner".into()),
            modifiers: None,
            praiseworthiness: -0.7,
        }),
        None,
    )
    .unwrap()
}

// ---------------------------------------------------------------------------
// 관계 / 저장소 헬퍼
// ---------------------------------------------------------------------------

/// 테스트용 중립 관계 (관계 갱신 테스트 등에서 사용)
pub fn neutral_rel() -> Relationship {
    Relationship::neutral("npc", "test")
}

/// 테스트용 중립 modifier (감정 평가 시 기본값)
pub fn neutral_mods() -> RelationshipModifiers {
    RelationshipModifiers::neutral()
}

/// 표준 테스트 컨텍스트
///
/// 무백, 교룡이 미리 로드되어 있고 중립 관계가 설정된 상태로 시작합니다.
pub struct TestContext {
    pub repo: Arc<Mutex<InMemoryRepository>>,
    pub mu_baek: Npc,
    pub gyo_ryong: Npc,
}

impl TestContext {
    pub fn new() -> Self {
        let mut repo = InMemoryRepository::new();
        let mu_baek = make_무백();
        let gyo_ryong = make_교룡();

        repo.add_npc(mu_baek.clone());
        repo.add_npc(gyo_ryong.clone());
        repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

        Self {
            repo: Arc::new(Mutex::new(repo)),
            mu_baek,
            gyo_ryong,
        }
    }
}

/// 무백 — 정의로운 검객. 의리와 절제를 중시한다.
pub fn make_무백() -> Npc {
    let s = score;
    NpcBuilder::new("mu_baek", "무백")
        .description("정의로운 검객. 의리와 절제를 중시한다.")
        .honesty_humility(|h| {
            h.sincerity = s(0.8);
            h.fairness = s(0.7);
            h.greed_avoidance = s(0.6);
            h.modesty = s(0.5);
        })
        .emotionality(|e| {
            e.fearfulness = s(-0.6);
            e.anxiety = s(-0.4);
            e.dependence = s(-0.7);
            e.sentimentality = s(0.2);
        })
        .agreeableness(|a| {
            a.forgiveness = s(0.6);
            a.gentleness = s(0.7);
            a.flexibility = s(0.2);
            a.patience = s(0.8);
        })
        .conscientiousness(|c| {
            c.organization = s(0.4);
            c.diligence = s(0.8);
            c.perfectionism = s(0.6);
            c.prudence = s(0.7);
        })
        .build()
}

/// 교룡 — 야심적인 여검객. 자유를 갈망하며 관습을 거부한다.
pub fn make_교룡() -> Npc {
    let s = score;
    NpcBuilder::new("gyo_ryong", "교룡")
        .description("야심적인 여검객. 자유를 갈망하며 관습을 거부한다.")
        .honesty_humility(|h| {
            h.sincerity = s(-0.4);
            h.fairness = s(-0.5);
            h.greed_avoidance = s(-0.6);
            h.modesty = s(-0.7);
        })
        .emotionality(|e| {
            e.fearfulness = s(0.8);
            e.anxiety = s(0.7);
            e.dependence = s(0.5);
            e.sentimentality = s(0.6);
        })
        .extraversion(|x| {
            x.social_self_esteem = s(0.7);
            x.social_boldness = s(0.8);
            x.sociability = s(0.0);
            x.liveliness = s(0.6);
        })
        .agreeableness(|a| {
            a.forgiveness = s(-0.6);
            a.gentleness = s(-0.5);
            a.flexibility = s(-0.4);
            a.patience = s(-0.7);
        })
        .conscientiousness(|c| {
            c.organization = s(-0.5);
            c.diligence = s(-0.3);
            c.perfectionism = s(-0.4);
            c.prudence = s(-0.6);
        })
        .openness(|o| {
            o.aesthetic_appreciation = s(0.6);
            o.inquisitiveness = s(0.8);
            o.creativity = s(0.7);
            o.unconventionality = s(0.9);
        })
        .build()
}

/// 수련 — 절제의 여검객
pub fn make_수련() -> Npc {
    let s = score;
    NpcBuilder::new("shu_lien", "수련")
        .description("절제의 여검객. 의무와 명예를 삶의 기둥으로 삼는다.")
        .honesty_humility(|h| {
            h.sincerity = s(0.8);
            h.fairness = s(0.9);
            h.greed_avoidance = s(0.7);
            h.modesty = s(0.6);
        })
        .emotionality(|e| {
            e.fearfulness = s(-0.3);
            e.anxiety = s(0.2);
            e.dependence = s(-0.5);
            e.sentimentality = s(0.7);
        })
        .agreeableness(|a| {
            a.forgiveness = s(0.5);
            a.gentleness = s(0.6);
            a.flexibility = s(0.3);
            a.patience = s(0.9);
        })
        .conscientiousness(|c| {
            c.organization = s(0.6);
            c.diligence = s(0.8);
            c.perfectionism = s(0.5);
            c.prudence = s(0.9);
        })
        .build()
}

/// 소호 — 자유로운 낭인
pub fn make_소호() -> Npc {
    let s = score;
    NpcBuilder::new("so_ho", "소호")
        .description("자유로운 낭인. 직감과 행동으로 세상을 살아간다.")
        .honesty_humility(|h| {
            h.sincerity = s(0.1);
            h.fairness = s(0.5);
            h.greed_avoidance = s(0.3);
            h.modesty = s(-0.3);
        })
        .emotionality(|e| {
            e.fearfulness = s(-0.7);
            e.anxiety = s(-0.5);
            e.dependence = s(-0.8);
            e.sentimentality = s(0.4);
        })
        .extraversion(|x| {
            x.social_self_esteem = s(0.6);
            x.social_boldness = s(0.7);
            x.sociability = s(0.5);
            x.liveliness = s(0.4);
        })
        .agreeableness(|a| {
            a.forgiveness = s(0.1);
            a.gentleness = s(-0.4);
            a.flexibility = s(0.3);
            a.patience = s(-0.3);
        })
        .conscientiousness(|c| {
            c.organization = s(-0.6);
            c.diligence = s(0.2);
            c.perfectionism = s(-0.4);
            c.prudence = s(-0.5);
        })
        .build()
}

// ---------------------------------------------------------------------------
// B4 Session 4 — EventBus 관찰 헬퍼
// ---------------------------------------------------------------------------

use futures::{Stream, StreamExt};
use npc_mind::application::event_bus::EventBus;
use npc_mind::domain::event::{DomainEvent, EventKind};
use std::time::Duration;

/// Director/SceneTask 테스트의 기본 타임아웃 — 느린 CI runner도 포용.
pub const SCENE_TASK_TEST_TIMEOUT: Duration = Duration::from_secs(2);

pub fn expect_event(
    bus: &EventBus,
    kind: EventKind,
    timeout: Duration,
) -> impl std::future::Future<Output = Arc<DomainEvent>> {
    let mut rx = Box::pin(bus.subscribe()) as std::pin::Pin<Box<dyn Stream<Item = Arc<DomainEvent>> + Send>>;
    async move {
        let fut = async {
            while let Some(event) = rx.next().await {
                if event.kind() == kind {
                    return event;
                }
            }
            panic!("EventBus stream closed before matching event kind: {:?}", kind);
        };
        tokio::time::timeout(timeout, fut)
            .await
            .unwrap_or_else(|_| panic!("timeout waiting for event kind: {:?}", kind))
    }
}

pub fn expect_events(
    bus: &EventBus,
    kinds: &[EventKind],
    timeout: Duration,
) -> impl std::future::Future<Output = Vec<Arc<DomainEvent>>> {
    let mut rx = Box::pin(bus.subscribe()) as std::pin::Pin<Box<dyn Stream<Item = Arc<DomainEvent>> + Send>>;
    let expected: std::collections::HashSet<_> = kinds.iter().cloned().collect();
    let kinds_owned: Vec<EventKind> = kinds.to_vec();
    async move {
        let fut = async {
            let mut collected: Vec<Arc<DomainEvent>> = Vec::new();
            let mut remaining = expected;
            while let Some(event) = rx.next().await {
                if remaining.remove(&event.kind()) {
                    collected.push(event);
                    if remaining.is_empty() {
                        return collected;
                    }
                }
            }
            panic!(
                "EventBus stream closed before all expected kinds arrived (missing: {:?})",
                remaining
            );
        };
        tokio::time::timeout(timeout, fut)
            .await
            .unwrap_or_else(|_| panic!("timeout waiting for events: {:?}", kinds_owned))
    }
}

// ---------------------------------------------------------------------------
// B5.2 — v2 dispatcher 빌더
// ---------------------------------------------------------------------------

use npc_mind::application::command::CommandDispatcher;
use npc_mind::application::event_store::{EventStore, InMemoryEventStore};

/// `with_default_handlers()`가 적용된 v2 CommandDispatcher 생성.
pub fn v2_dispatcher(
    repo: Arc<Mutex<InMemoryRepository>>,
    store: Arc<dyn EventStore>,
    bus: Arc<EventBus>,
) -> CommandDispatcher<InMemoryRepository> {
    CommandDispatcher::new(repo, store, bus).with_default_handlers()
}

/// 기본 InMemoryEventStore + EventBus + v2 핸들러 등록 dispatcher 일괄 생성.
pub fn v2_dispatcher_with_defaults(
    repo: Arc<Mutex<InMemoryRepository>>,
) -> (
    CommandDispatcher<InMemoryRepository>,
    Arc<InMemoryEventStore>,
    Arc<EventBus>,
) {
    let store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let store_dyn: Arc<dyn EventStore> = store.clone();
    let bus = Arc::new(EventBus::new());
    let dispatcher = v2_dispatcher(repo, store_dyn, bus.clone());
    (dispatcher, store, bus)
}
