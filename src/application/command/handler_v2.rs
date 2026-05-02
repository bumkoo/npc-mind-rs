//! EventHandler 프로토콜 — B안(다중 Scene 동시 실행) 이행 Stage B0 뼈대
//!
//! Transactional / Inline / Fanout 세 실행 모드를 단일 `EventHandler` 트레이트로 통합한다.
//! Stage B0에서는 타입만 정의하며, 실제 핸들러 구현체·Dispatcher 통합은 B1~B4에서 진행된다.
//!
//! **현재(B0) 상태:** 외부에서 사용되지 않음. 컴파일 검증과 설계 고정 목적.
//!
//! 관련 설계: `docs/architecture/b-plan-implementation.md` §4 · `priority.rs` · `AggregateKey`.

#![allow(dead_code)]

use thiserror::Error;

use crate::domain::aggregate::AggregateKey;
use crate::domain::emotion::{EmotionState, Scene};
use crate::domain::event::{DomainEvent, EventKind};
use crate::domain::guide::ActingGuide;
use crate::domain::personality::Npc;
use crate::domain::relationship::Relationship;

use crate::application::event_store::EventStore;
use crate::domain::scene_id::SceneId;
use crate::ports::{EmotionStore, NpcWorld, SceneStore};

// ---------------------------------------------------------------------------
// EventHandler 트레이트
// ---------------------------------------------------------------------------

/// 이벤트를 처리하는 핸들러 — Transactional / Inline / Fanout 공통 인터페이스
///
/// Dispatcher는 핸들러의 `mode()`에 따라 실행 단계(트랜잭션 내부 / 커밋 후 동기 /
/// 비동기 broadcast)를 결정하며, `interest()`로 관심 이벤트를 필터링한다.
///
/// B0에서는 이 트레이트를 구현하는 타입이 아직 없다. B1에서 기존 Policy
/// (EmotionPolicy/GuidePolicy/RelationshipPolicy)가 이 트레이트를 추가 구현한다.
pub trait EventHandler: Send + Sync {
    /// 트레이싱·로깅·디스패처 오류 리포팅용 식별자
    fn name(&self) -> &'static str;

    /// 이 핸들러가 관심 갖는 이벤트 종류
    fn interest(&self) -> HandlerInterest;

    /// 실행 모드 (priority 포함)
    fn mode(&self) -> DeliveryMode;

    /// 실제 처리 — 에러는 `DeliveryMode`에 따라 다르게 취급된다
    /// (Transactional: 커맨드 전체 중단 / Inline: 로그만 / Fanout: 구독자 책임).
    fn handle(
        &self,
        event: &DomainEvent,
        ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError>;
}

// ---------------------------------------------------------------------------
// 관심 이벤트 필터
// ---------------------------------------------------------------------------

/// 핸들러가 어떤 이벤트에 관심 있는지 선언
///
/// Dispatcher가 `matches()`로 이벤트별 핸들러 목록을 사전 필터링한다.
pub enum HandlerInterest {
    /// 모든 이벤트
    All,
    /// 특정 종류만 (타입 안전)
    Kinds(Vec<EventKind>),
    /// 커스텀 술어 — 페이로드 내부 조건까지 보고 싶을 때
    Predicate(fn(&DomainEvent) -> bool),
}

impl HandlerInterest {
    pub fn matches(&self, event: &DomainEvent) -> bool {
        match self {
            HandlerInterest::All => true,
            HandlerInterest::Kinds(kinds) => kinds.contains(&event.kind()),
            HandlerInterest::Predicate(pred) => pred(event),
        }
    }
}

// ---------------------------------------------------------------------------
// DeliveryMode
// ---------------------------------------------------------------------------

/// 핸들러 실행 계약
///
/// priority는 같은 모드 내부에서만 의미가 있다. 상수값은 `priority` 모듈 참조.
pub enum DeliveryMode {
    /// 커맨드 트랜잭션 내부 sync 실행. 에러는 커맨드 전체 중단.
    ///
    /// `can_emit_follow_up = true`일 때만 `HandlerResult::follow_up_events`가 유효하며,
    /// Dispatcher는 이를 큐에 넣어 같은 트랜잭션 안에서 재귀 처리한다.
    Transactional {
        priority: i32,
        can_emit_follow_up: bool,
    },

    /// 이벤트 커밋 직후 sync 실행. 에러는 로그만, 커맨드는 성공 반환.
    ///
    /// 주로 Projection(쓰기 경로의 L1 읽기 뷰 갱신)용.
    Inline { priority: i32 },

    /// 비동기 fan-out. 발행자는 구독자 처리를 기다리지 않는다.
    ///
    /// 주로 MemoryProjector/StoryAgent/SSE 같은 외부 관찰자용.
    Fanout,
}

// ---------------------------------------------------------------------------
// EventHandlerContext
// ---------------------------------------------------------------------------

/// 핸들러에 주입되는 실행 컨텍스트
///
/// - `repo`/`event_store`는 read-only trait object로 주입된다.
/// - `shared`는 같은 커맨드의 Transactional 핸들러 간 **mut 공유 상태** (per-command scratchpad).
/// - `prior_events`는 같은 커맨드에서 이미 발행된 이벤트 목록.
/// - `aggregate_key`는 커맨드·이벤트가 속한 aggregate — Fanout 구독자가 Scene별 demultiplex할 때도 사용.
///
/// **B0 Naming Note:** 기존 `handler::HandlerContext`와 구별하기 위해 `EventHandlerContext`로
/// 명명한다. B5 cutover 시점에 구 `handler.rs` 삭제 후 `HandlerContext`로 rename 예정.
///
/// **B1/B4 Thread-Safety Note:** `repo`에 `Send + Sync`를 인라인으로 요구한다 —
/// `MindRepository` 트레이트 자체에는 이 바운드가 없어서, SceneTask가 이 컨텍스트를
/// 워커 스레드로 넘길 때(`EventHandler: Send + Sync` 계약과 맞물려) 컴파일 에러를 예방한다.
/// `EventStore`는 트레이트 정의에 `Send + Sync`가 이미 포함되어 있어 별도 표기 불필요.
pub struct EventHandlerContext<'a> {
    pub world: &'a (dyn NpcWorld + Send + Sync),
    pub emotions: &'a (dyn EmotionStore + Send + Sync),
    pub scenes: &'a (dyn SceneStore + Send + Sync),
    pub event_store: &'a dyn EventStore,
    pub shared: &'a mut HandlerShared,
    pub prior_events: &'a [DomainEvent],
    pub aggregate_key: AggregateKey,
}

impl<'a> EventHandlerContext<'a> {
    /// NPC를 저장소에서 조회하거나 에러를 반환합니다.
    pub fn get_npc(&self, id: &str) -> Result<Npc, HandlerError> {
        self.world
            .get_npc(id)
            .ok_or_else(|| HandlerError::NpcNotFound(id.to_string()))
    }

    /// 관계를 공유 상태 또는 저장소에서 조회하거나 에러를 반환합니다.
    pub fn get_relationship(
        &self,
        owner_id: &str,
        target_id: &str,
    ) -> Result<Relationship, HandlerError> {
        self.shared
            .relationship
            .as_ref()
            .cloned()
            .or_else(|| self.world.get_relationship(owner_id, target_id))
            .ok_or_else(|| HandlerError::RelationshipNotFound {
                owner_id: owner_id.to_string(),
                target_id: target_id.to_string(),
            })
    }

    /// 감정 상태를 공유 상태 또는 저장소에서 조회하거나 에러를 반환합니다.
    pub fn get_emotion_state(&self, npc_id: &str) -> Result<EmotionState, HandlerError> {
        self.shared
            .emotion_state
            .as_ref()
            .cloned()
            .or_else(|| self.emotions.get_emotion_state(npc_id))
            .ok_or_else(|| HandlerError::EmotionStateNotFound(npc_id.to_string()))
    }

    /// Scene을 공유 상태 또는 저장소에서 조회합니다.
    pub fn get_scene_by_id(&self, scene_id: &SceneId) -> Option<Scene> {
        self.shared
            .scene
            .as_ref()
            .filter(|s| s.npc_id() == scene_id.npc_id && s.partner_id() == scene_id.partner_id)
            .cloned()
            .or_else(|| self.scenes.get_scene_by_id(scene_id))
    }
}

// ---------------------------------------------------------------------------
// HandlerShared
// ---------------------------------------------------------------------------

/// 같은 커맨드 내 Transactional 핸들러들이 공유·갱신하는 scratchpad
///
/// `PipelineState`의 후신. **필드는 큐레이션된 계약**이며, 신규 필드 추가는
/// PR 리뷰 항목(용어 drift 방지).
///
/// 기존 `handler::HandlerContext`(read-only snapshot)와는 역할이 다르다 — 이쪽은
/// 핸들러 실행 중 갱신되는 mutable 상태.
#[derive(Debug, Default)]
pub struct HandlerShared {
    pub emotion_state: Option<EmotionState>,
    pub relationship: Option<Relationship>,
    pub scene: Option<Scene>,
    pub guide: Option<ActingGuide>,

    // --- B4.1 destructive signals ---
    //
    // Option<T>/None은 "변화 없음"을 의미하므로 "clear" 액션을 표현할 수 없다.
    // Dispatcher가 transactional phase 종료 후 이 플래그들을 읽어 `repo.clear_*`를 호출.
    // DialogueEndRequested 처리 시 RelationshipPolicy가 이 필드들을 설정.
    /// 감정 상태를 clear할 NPC id (Some일 때만 clear 적용)
    pub clear_emotion_for: Option<String>,
    /// 활성 Scene을 clear할지 여부
    pub clear_scene: bool,
}

// ---------------------------------------------------------------------------
// HandlerResult / HandlerError
// ---------------------------------------------------------------------------

/// 핸들러 처리 결과
///
/// `follow_up_events`는 `DeliveryMode::Transactional { can_emit_follow_up: true }`에서만
/// 유효. Dispatcher가 이를 다음 depth로 큐잉한다.
#[derive(Debug, Default)]
pub struct HandlerResult {
    pub follow_up_events: Vec<DomainEvent>,
}

/// 핸들러 실행 에러
///
/// 각 variant는 HTTP 매핑 의미(404/400/500)를 타입으로 표현한다.
/// Mind Studio `AppError::V2Dispatch` 매핑이 문자열 매칭 대신 variant 매칭으로 분기.
#[derive(Debug, Error)]
pub enum HandlerError {
    /// NPC를 저장소에서 찾지 못함 — 404 mapping
    #[error("npc not found: {0}")]
    NpcNotFound(String),

    /// 관계가 등록되지 않음 — 404 mapping
    #[error("relationship not found: {owner_id}↔{target_id}")]
    RelationshipNotFound { owner_id: String, target_id: String },

    /// 감정 상태가 없음 (appraise 선행 누락 등) — 400 mapping (워크플로우 순서 오류)
    #[error("emotion state not found for npc '{0}'")]
    EmotionStateNotFound(String),

    /// DTO → 도메인 변환 실패 등 입력 검증 오류 — 400 mapping
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Projection mutex poison 등 인프라/invariant 위반 — 500 mapping
    #[error("infrastructure error: {0}")]
    Infrastructure(&'static str),

    /// 저장소 쓰기/읽기 실패 — 500 mapping
    #[error("repository error: {0}")]
    Repository(String),
}

// ---------------------------------------------------------------------------
// 설계 고정 테스트
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// L1 테스트 Harness (B1)
// ---------------------------------------------------------------------------

/// L1 단위 테스트용 Harness — Dispatcher 없이 `EventHandler`를 직접 실행
///
/// Policy별 L1 unit test가 공통으로 사용. 모든 field는 `pub`으로 노출되어
/// 테스트 코드가 필요 시 repo/event_store/shared를 직접 조작할 수 있다.
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::adapter::memory_repository::InMemoryRepository;
    use crate::application::event_store::InMemoryEventStore;
    use crate::domain::emotion::{EmotionState, Scene};
    use crate::domain::event::DomainEvent;
    use crate::domain::personality::Npc;
    use crate::domain::relationship::Relationship;
    use crate::ports::{EmotionStore, SceneStore};

    pub struct HandlerTestHarness {
        pub repo: InMemoryRepository,
        pub event_store: InMemoryEventStore,
        pub shared: HandlerShared,
    }

    impl HandlerTestHarness {
        pub fn new() -> Self {
            Self {
                repo: InMemoryRepository::new(),
                event_store: InMemoryEventStore::new(),
                shared: HandlerShared::default(),
            }
        }

        pub fn with_npc(mut self, npc: Npc) -> Self {
            self.repo.add_npc(npc);
            self
        }

        pub fn with_relationship(mut self, rel: Relationship) -> Self {
            self.repo.add_relationship(rel);
            self
        }

        pub fn with_emotion_state(mut self, npc_id: &str, state: EmotionState) -> Self {
            self.repo.save_emotion_state(npc_id, state);
            self
        }

        pub fn with_scene(mut self, scene: Scene) -> Self {
            self.repo.save_scene(scene);
            self
        }

        pub fn with_shared_emotion_state(mut self, state: EmotionState) -> Self {
            self.shared.emotion_state = Some(state);
            self
        }

        pub fn with_shared_relationship(mut self, rel: Relationship) -> Self {
            self.shared.relationship = Some(rel);
            self
        }

        /// Handler 실행. aggregate_key는 event에서 도출된다.
        pub fn dispatch<H: EventHandler>(
            &mut self,
            handler: &H,
            event: DomainEvent,
        ) -> Result<HandlerResult, HandlerError> {
            let aggregate_key = event.aggregate_key();
            let prior_events: Vec<DomainEvent> = Vec::new();
            let mut ctx = EventHandlerContext {
                world: &self.repo,
                emotions: &self.repo,
                scenes: &self.repo,
                event_store: &self.event_store,
                shared: &mut self.shared,
                prior_events: &prior_events,
                aggregate_key,
            };
            handler.handle(&event, &mut ctx)
        }
    }

    impl Default for HandlerTestHarness {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::EventPayload;

    fn make_event(payload: EventPayload) -> DomainEvent {
        DomainEvent::new(1, "test".into(), 1, payload)
    }

    #[test]
    fn interest_all_matches_any_event() {
        let ev = make_event(EventPayload::EmotionCleared { npc_id: "a".into() });
        assert!(HandlerInterest::All.matches(&ev));
    }

    #[test]
    fn interest_kinds_filters_precisely() {
        let emotion = make_event(EventPayload::EmotionCleared { npc_id: "a".into() });
        let scene_started = make_event(EventPayload::SceneStarted {
            npc_id: "a".into(),
            partner_id: "b".into(),
            focus_count: 0,
            initial_focus_id: None,
        });

        let interest = HandlerInterest::Kinds(vec![EventKind::SceneStarted]);
        assert!(!interest.matches(&emotion));
        assert!(interest.matches(&scene_started));
    }

    #[test]
    fn interest_predicate_passes_event_reference() {
        let interest = HandlerInterest::Predicate(|ev| {
            matches!(ev.payload, EventPayload::EmotionCleared { .. })
        });
        let emotion = make_event(EventPayload::EmotionCleared { npc_id: "a".into() });
        let guide = make_event(EventPayload::GuideGenerated {
            npc_id: "a".into(),
            partner_id: "b".into(),
        });
        assert!(interest.matches(&emotion));
        assert!(!interest.matches(&guide));
    }

    #[test]
    fn handler_shared_default_is_all_none() {
        let s = HandlerShared::default();
        assert!(s.emotion_state.is_none());
        assert!(s.relationship.is_none());
        assert!(s.scene.is_none());
        assert!(s.guide.is_none());
        // B4.1 destructive signals — Dispatcher가 save → clear 순서로 반영하므로
        // 기본값이 "clear 없음"인지 보장.
        assert!(s.clear_emotion_for.is_none());
        assert!(!s.clear_scene);
    }

    #[test]
    fn handler_result_default_is_empty_followups() {
        let r = HandlerResult::default();
        assert!(r.follow_up_events.is_empty());
    }

    #[test]
    fn handler_error_display_is_readable() {
        let e = HandlerError::NpcNotFound("alice".into());
        assert_eq!(e.to_string(), "npc not found: alice");
        let e = HandlerError::RelationshipNotFound {
            owner_id: "alice".into(),
            target_id: "bob".into(),
        };
        assert_eq!(e.to_string(), "relationship not found: alice↔bob");
        let e = HandlerError::EmotionStateNotFound("alice".into());
        assert_eq!(e.to_string(), "emotion state not found for npc 'alice'");
        let e = HandlerError::InvalidInput("bad focus".into());
        assert_eq!(e.to_string(), "invalid input: bad focus");
        let e = HandlerError::Infrastructure("mutex poisoned");
        assert_eq!(e.to_string(), "infrastructure error: mutex poisoned");
        let e = HandlerError::Repository("save failed".into());
        assert_eq!(e.to_string(), "repository error: save failed");
    }

    use super::test_support::HandlerTestHarness;
    use crate::domain::emotion::EmotionState;
    use crate::domain::personality::{HexacoProfile, Npc};
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };

    /// 외부 검증용 — 실행 여부와 ctx 가시성을 확인하는 probe handler
    struct ProbeHandler {
        ran: Arc<AtomicBool>,
        saw_npc: Arc<AtomicBool>,
        npc_id: &'static str,
    }

    impl EventHandler for ProbeHandler {
        fn name(&self) -> &'static str {
            "ProbeHandler"
        }
        fn interest(&self) -> HandlerInterest {
            HandlerInterest::All
        }
        fn mode(&self) -> DeliveryMode {
            DeliveryMode::Transactional {
                priority: 0,
                can_emit_follow_up: false,
            }
        }
        fn handle(
            &self,
            _event: &DomainEvent,
            ctx: &mut EventHandlerContext<'_>,
        ) -> Result<HandlerResult, HandlerError> {
            self.ran.store(true, Ordering::SeqCst);
            if ctx.world.get_npc(self.npc_id).is_some() {
                self.saw_npc.store(true, Ordering::SeqCst);
            }
            Ok(HandlerResult::default())
        }
    }

    fn make_npc(id: &str) -> Npc {
        Npc::new(id, id, "test", HexacoProfile::neutral())
    }

    fn cleared_event(npc_id: &str) -> DomainEvent {
        DomainEvent::new(
            42,
            npc_id.into(),
            1,
            EventPayload::EmotionCleared {
                npc_id: npc_id.into(),
            },
        )
    }

    #[test]
    fn harness_dispatch_routes_event_to_handler() {
        let ran = Arc::new(AtomicBool::new(false));
        let probe = ProbeHandler {
            ran: ran.clone(),
            saw_npc: Arc::new(AtomicBool::new(false)),
            npc_id: "x",
        };
        let mut h = HandlerTestHarness::new();
        h.dispatch(&probe, cleared_event("x")).expect("ok");
        assert!(ran.load(Ordering::SeqCst), "handler가 호출되어야 함");
    }

    #[test]
    fn harness_dispatch_passes_repo_to_handler() {
        let saw = Arc::new(AtomicBool::new(false));
        let probe = ProbeHandler {
            ran: Arc::new(AtomicBool::new(false)),
            saw_npc: saw.clone(),
            npc_id: "alice",
        };
        let mut h = HandlerTestHarness::new().with_npc(make_npc("alice"));
        h.dispatch(&probe, cleared_event("alice")).expect("ok");
        assert!(
            saw.load(Ordering::SeqCst),
            "ctx.repo로 with_npc로 추가한 NPC가 보여야 함"
        );
    }

    #[test]
    fn harness_isolates_shared_state_between_instances() {
        let mut h1 = HandlerTestHarness::new();
        let h2 = HandlerTestHarness::new();
        h1.shared.clear_scene = true;

        assert!(h1.shared.clear_scene);
        assert!(!h2.shared.clear_scene);
    }

    /// ctx.shared.emotion_state를 항상 채우는 probe handler
    struct EmotionFillHandler;

    impl EventHandler for EmotionFillHandler {
        fn name(&self) -> &'static str {
            "EmotionFillHandler"
        }
        fn interest(&self) -> HandlerInterest {
            HandlerInterest::All
        }
        fn mode(&self) -> DeliveryMode {
            DeliveryMode::Transactional {
                priority: 0,
                can_emit_follow_up: false,
            }
        }
        fn handle(
            &self,
            _event: &DomainEvent,
            ctx: &mut EventHandlerContext<'_>,
        ) -> Result<HandlerResult, HandlerError> {
            ctx.shared.emotion_state = Some(EmotionState::new());
            Ok(HandlerResult::default())
        }
    }

    #[test]
    fn harness_shared_state_persists_across_dispatch_calls() {
        let mut h = HandlerTestHarness::new();
        h.dispatch(&EmotionFillHandler, cleared_event("a")).unwrap();
        assert!(h.shared.emotion_state.is_some());

        let saw = Arc::new(AtomicUsize::new(0));
        struct ReadShared(Arc<AtomicUsize>);
        impl EventHandler for ReadShared {
            fn name(&self) -> &'static str { "ReadShared" }
            fn interest(&self) -> HandlerInterest { HandlerInterest::All }
            fn mode(&self) -> DeliveryMode {
                DeliveryMode::Transactional { priority: 0, can_emit_follow_up: false }
            }
            fn handle(
                &self,
                _event: &DomainEvent,
                ctx: &mut EventHandlerContext<'_>,
            ) -> Result<HandlerResult, HandlerError> {
                if ctx.shared.emotion_state.is_some() {
                    self.0.fetch_add(1, Ordering::SeqCst);
                }
                Ok(HandlerResult::default())
            }
        }
        h.dispatch(&ReadShared(saw.clone()), cleared_event("a")).unwrap();
        assert_eq!(saw.load(Ordering::SeqCst), 1, "2차 dispatch가 1차 shared를 봐야 함");
    }

    /// 항상 InvalidInput 에러를 반환하는 probe
    struct AlwaysFailHandler;
    impl EventHandler for AlwaysFailHandler {
        fn name(&self) -> &'static str { "AlwaysFailHandler" }
        fn interest(&self) -> HandlerInterest { HandlerInterest::All }
        fn mode(&self) -> DeliveryMode {
            DeliveryMode::Transactional { priority: 0, can_emit_follow_up: false }
        }
        fn handle(
            &self,
            _event: &DomainEvent,
            _ctx: &mut EventHandlerContext<'_>,
        ) -> Result<HandlerResult, HandlerError> {
            Err(HandlerError::InvalidInput("forced".into()))
        }
    }

    #[test]
    fn harness_propagates_handler_error() {
        let mut h = HandlerTestHarness::new();
        let result = h.dispatch(&AlwaysFailHandler, cleared_event("x"));
        match result {
            Err(HandlerError::InvalidInput(msg)) => assert_eq!(msg, "forced"),
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn harness_aggregate_key_derived_from_event() {
        struct CapturedKey(Arc<std::sync::Mutex<Option<AggregateKey>>>);
        impl EventHandler for CapturedKey {
            fn name(&self) -> &'static str { "CapturedKey" }
            fn interest(&self) -> HandlerInterest { HandlerInterest::All }
            fn mode(&self) -> DeliveryMode {
                DeliveryMode::Transactional { priority: 0, can_emit_follow_up: false }
            }
            fn handle(
                &self,
                _event: &DomainEvent,
                ctx: &mut EventHandlerContext<'_>,
            ) -> Result<HandlerResult, HandlerError> {
                *self.0.lock().unwrap() = Some(ctx.aggregate_key.clone());
                Ok(HandlerResult::default())
            }
        }
        let captured = Arc::new(std::sync::Mutex::new(None));
        let event = cleared_event("npc-7");
        let expected = event.aggregate_key();

        let mut h = HandlerTestHarness::new();
        h.dispatch(&CapturedKey(captured.clone()), event).unwrap();

        assert_eq!(captured.lock().unwrap().as_ref(), Some(&expected));
    }

    #[test]
    fn interest_kinds_empty_matches_nothing() {
        let interest = HandlerInterest::Kinds(vec![]);
        let ev = cleared_event("a");
        assert!(!interest.matches(&ev));
    }

    #[test]
    fn interest_predicate_inspects_payload_fields() {
        // Predicate는 페이로드 내부 값을 검사할 수 있어야 함
        let interest = HandlerInterest::Predicate(|ev| match &ev.payload {
            EventPayload::EmotionCleared { npc_id } => npc_id == "match-me",
            _ => false,
        });
        assert!(interest.matches(&cleared_event("match-me")));
        assert!(!interest.matches(&cleared_event("other")));
    }

    #[test]
    fn handler_shared_clear_signals_independent_of_optional_state() {
        // clear_emotion_for/clear_scene 시그널은 set_*과 독립적으로 설정 가능
        let s = HandlerShared {
            emotion_state: Some(EmotionState::new()),
            clear_emotion_for: Some("a".into()),
            clear_scene: true,
            ..HandlerShared::default()
        };
        // 두 그룹이 독립적인 필드 — 동시 설정해도 충돌 없음 (Dispatcher가 save 후 clear 적용)
        assert!(s.emotion_state.is_some());
        assert_eq!(s.clear_emotion_for.as_deref(), Some("a"));
        assert!(s.clear_scene);
    }
}
