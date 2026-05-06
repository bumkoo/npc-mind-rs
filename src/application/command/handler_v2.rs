//! EventHandler 프로토콜 — B안(다중 Scene 동시 실행) 이행 Stage B0 뼈대
//!
//! Transactional / Inline / Fanout 세 실행 모드를 단일 `EventHandler` 트레이트로 통합한다.
//! Stage B0에서는 타입만 정의하며, 실제 핸들러 구현체·Dispatcher 통합은 B1~B4에서 진행된다.

use crate::domain::aggregate::AggregateKey;
use crate::domain::emotion::{EmotionState, Scene};
use crate::domain::event::{DomainEvent, EventKind};
use crate::domain::guide::ActingGuide;
use crate::domain::personality::Npc;
use crate::domain::relationship::Relationship;
use crate::domain::scene_id::SceneId;
use crate::ports::{EmotionStore, NpcWorld, SceneStore};
use crate::application::event_store::EventStore;

// ---------------------------------------------------------------------------
// 에러 및 결과 타입
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("NPC '{0}'을(를) 찾을 수 없습니다")]
    NpcNotFound(String),

    #[error("'{owner_id}'와(과) '{target_id}' 사이의 관계를 찾을 수 없습니다")]
    RelationshipNotFound { owner_id: String, target_id: String },

    #[error("NPC '{0}'의 감정 상태를 찾을 수 없습니다")]
    EmotionStateNotFound(String),

    #[error("잘못된 입력: {0}")]
    InvalidInput(String),

    #[error("인프라 오류: {0}")]
    Infrastructure(&'static str),

    #[error("리포지토리 오류: {0}")]
    Repository(String),
}

/// 핸들러 실행 결과
#[derive(Debug, Default)]
pub struct HandlerResult {
    /// 이 핸들러에 의해 유발된 후속 도메인 이벤트 목록
    pub follow_up_events: Vec<DomainEvent>,
}

// ---------------------------------------------------------------------------
// 실행 컨텍스트
// ---------------------------------------------------------------------------

/// 이벤트 핸들러 실행 시 제공되는 환경 (v2)
///
/// **B1/B4 Thread-Safety Note:** `repo`에 `Send + Sync`를 인라인으로 요구한다 —
/// `Arc<Mutex<R>>` 형태의 단일 SSoT 리포지토리가 트랜잭션 내에서 공유될 수 있게 함.
pub struct EventHandlerContext<'a, 'b, R: crate::ports::MindRepository> {
    pub world: &'a (dyn NpcWorld + Send + Sync),
    pub emotions: &'a (dyn EmotionStore + Send + Sync),
    pub scenes: &'a (dyn SceneStore + Send + Sync),
    pub event_store: &'a dyn EventStore,
    pub uow: &'a mut super::uow::UnitOfWork<'b, R>,
    pub prior_events: &'a [DomainEvent],
    pub aggregate_key: AggregateKey,
}

impl<'a, 'b, R: crate::ports::MindRepository> EventHandlerContext<'a, 'b, R> {
    /// NPC를 저장소에서 조회하거나 에러를 반환합니다.
    pub fn get_npc(&self, id: &str) -> Result<Npc, HandlerError> {
        self.world
            .get_npc(id)
            .ok_or_else(|| HandlerError::NpcNotFound(id.to_string()))
    }

    /// 관계를 작업 단위 또는 저장소에서 조회하거나 에러를 반환합니다.
    pub fn get_relationship(
        &self,
        owner_id: &str,
        target_id: &str,
    ) -> Result<Relationship, HandlerError> {
        self.uow
            .relationship
            .as_ref()
            .cloned()
            .or_else(|| self.world.get_relationship(owner_id, target_id))
            .ok_or_else(|| HandlerError::RelationshipNotFound {
                owner_id: owner_id.to_string(),
                target_id: target_id.to_string(),
            })
    }

    /// 감정 상태를 작업 단위 또는 저장소에서 조회하거나 에러를 반환합니다.
    pub fn get_emotion_state(&self, npc_id: &str) -> Result<EmotionState, HandlerError> {
        self.uow
            .emotion_state
            .as_ref()
            .filter(|(id, _)| id == npc_id)
            .map(|(_, state)| state.clone())
            .or_else(|| self.emotions.get_emotion_state(npc_id))
            .ok_or_else(|| HandlerError::EmotionStateNotFound(npc_id.to_string()))
    }

    /// Scene을 작업 단위 또는 저장소에서 조회합니다.
    pub fn get_scene_by_id(&self, scene_id: &SceneId) -> Option<Scene> {
        self.uow
            .scene
            .as_ref()
            .filter(|s| s.npc_id() == scene_id.npc_id && s.partner_id() == scene_id.partner_id)
            .cloned()
            .or_else(|| self.scenes.get_scene_by_id(scene_id))
    }
}

/// 핸들러 컨텍스트에 대한 동적 디스패치 인터페이스 (UoW 타입 은닉용)
pub trait DynamicHandlerContext {
    fn get_npc(&self, id: &str) -> Result<Npc, HandlerError>;
    fn get_relationship(&self, owner_id: &str, target_id: &str) -> Result<Relationship, HandlerError>;
    fn get_emotion_state(&self, npc_id: &str) -> Result<EmotionState, HandlerError>;
    fn get_scene_by_id(&self, scene_id: &SceneId) -> Option<Scene>;
    
    // --- UoW 가변 조작 ---
    fn save_emotion_state(&mut self, npc_id: String, state: EmotionState);
    fn save_relationship(&mut self, relationship: Relationship);
    fn save_scene(&mut self, scene: Scene);
    fn clear_emotion_for(&mut self, npc_id: String);
    fn clear_scene(&mut self);
    
    // --- Metadata ---
    fn aggregate_key(&self) -> &AggregateKey;
    fn prior_events(&self) -> &[DomainEvent];
    fn guide(&self) -> Option<&crate::domain::guide::ActingGuide>;
    fn set_guide(&mut self, guide: crate::domain::guide::ActingGuide);
}

impl<'a, 'b, R: crate::ports::MindRepository> DynamicHandlerContext for EventHandlerContext<'a, 'b, R> {
    fn get_npc(&self, id: &str) -> Result<Npc, HandlerError> {
        self.get_npc(id)
    }
    fn get_relationship(&self, owner_id: &str, target_id: &str) -> Result<Relationship, HandlerError> {
        self.get_relationship(owner_id, target_id)
    }
    fn get_emotion_state(&self, npc_id: &str) -> Result<EmotionState, HandlerError> {
        self.get_emotion_state(npc_id)
    }
    fn get_scene_by_id(&self, scene_id: &SceneId) -> Option<Scene> {
        self.get_scene_by_id(scene_id)
    }
    fn save_emotion_state(&mut self, npc_id: String, state: EmotionState) {
        self.uow.save_emotion_state(npc_id, state);
    }
    fn save_relationship(&mut self, relationship: Relationship) {
        self.uow.save_relationship(relationship);
    }
    fn save_scene(&mut self, scene: Scene) {
        self.uow.save_scene(scene);
    }
    fn clear_emotion_for(&mut self, npc_id: String) {
        self.uow.clear_emotion_for(npc_id);
    }
    fn clear_scene(&mut self) {
        self.uow.clear_scene();
    }
    fn aggregate_key(&self) -> &AggregateKey {
        &self.aggregate_key
    }
    fn prior_events(&self) -> &[DomainEvent] {
        self.prior_events
    }
    fn guide(&self) -> Option<&ActingGuide> {
        self.uow.guide.as_ref()
    }
    fn set_guide(&mut self, guide: ActingGuide) {
        self.uow.guide = Some(guide);
    }
}

// ---------------------------------------------------------------------------
// 관심 이벤트 필터
// ---------------------------------------------------------------------------

/// 핸들러가 어떤 이벤트에 관심 있는지 선언
pub enum HandlerInterest {
    /// 특정 이벤트 종류 목록
    Kinds(Vec<EventKind>),
    /// 모든 이벤트 수신 (B-Plan §7.3 "EventAware" 호환)
    All,
}

impl HandlerInterest {
    pub fn matches(&self, event: &DomainEvent) -> bool {
        match self {
            HandlerInterest::Kinds(kinds) => kinds.contains(&event.kind()),
            HandlerInterest::All => true,
        }
    }
}

// ---------------------------------------------------------------------------
// 핸들러 모드 (Transactional / Inline / Fanout)
// ---------------------------------------------------------------------------

/// 이벤트의 실행 및 일관성 모델
pub enum DeliveryMode {
    /// 커맨드 실행 컨텍스트와 단일 트랜잭션으로 묶임. 에러 시 전체 중단.
    /// priority: 실행 순서 (낮을수록 먼저 실행).
    /// can_emit_follow_up: 처리 결과로 새 이벤트를 발행할 수 있는지 여부.
    Transactional {
        priority: i32,
        can_emit_follow_up: bool,
    },
    /// 커맨드 완료 직후 인라인 실행. 에러 시 로그만 기록.
    Inline { priority: i32 },
    /// 비동기/원격 구독자. 에러 시 재시도 등 구독자 책임.
    Fanout,
}

// ---------------------------------------------------------------------------
// 핸들러 프로토콜
// ---------------------------------------------------------------------------

/// Stage B0: 통합 이벤트 핸들러 트레이트
pub trait EventHandler: Send + Sync {
    /// 트레이싱·로깅·디스패처 오류 리포팅용 식별자
    fn name(&self) -> &'static str;

    /// 이 핸들러가 관심 갖는 이벤트 종류
    fn interest(&self) -> HandlerInterest;

    /// 실행 모드 (priority 포함)
    fn mode(&self) -> DeliveryMode;

    /// 실제 처리 — 에러는 `DeliveryMode`에 따라 다르게 취급된다
    /// (Transactional: 커맨드 전체 중단 / Inline: 로그만 / Fanout: 구독자 책임).
    fn handle_v2(
        &self,
        event: &DomainEvent,
        ctx: &mut dyn DynamicHandlerContext,
    ) -> Result<HandlerResult, HandlerError>;
}

// ---------------------------------------------------------------------------
// 횡단 관심사: 공유 스크래치패드 (HandlerShared)
// ---------------------------------------------------------------------------

/// Transactional 핸들러들이 같은 트랜잭션 내에서 공유하는 임시 상태
///
/// B4.1 (v0.2.0): 삭제 시그널(`clear_emotion_for`, `clear_scene`) 추가.
#[derive(Debug, Default, Clone)]
pub struct HandlerShared {
    pub emotion_state: Option<EmotionState>,
    pub relationship: Option<Relationship>,
    pub scene: Option<Scene>,
    pub guide: Option<ActingGuide>,

    // destructive signals — None/false = 변화 없음
    pub clear_emotion_for: Option<String>,
    pub clear_scene: bool,
}

// ---------------------------------------------------------------------------
// 테스트 지원
// ---------------------------------------------------------------------------

#[cfg(test)]
pub mod test_support {
    use std::sync::{Arc, Mutex};
    use super::*;
    use crate::InMemoryRepository;
    use crate::application::event_store::InMemoryEventStore;
    use crate::ports::{EmotionStore, SceneStore};

    pub struct HandlerTestHarness {
        pub repo: InMemoryRepository,
        pub event_store: InMemoryEventStore,
        pub repo_arc: Arc<Mutex<InMemoryRepository>>,
    }

    impl HandlerTestHarness {
        pub fn new() -> Self {
            let repo = InMemoryRepository::new();
            let repo_arc = Arc::new(Mutex::new(repo));
            Self {
                repo: InMemoryRepository::new(), 
                event_store: InMemoryEventStore::new(),
                repo_arc,
            }
        }

        pub fn with_npc(self, npc: Npc) -> Self {
            self.repo_arc.lock().unwrap().add_npc(npc);
            self
        }

        pub fn with_relationship(self, rel: Relationship) -> Self {
            self.repo_arc.lock().unwrap().add_relationship(rel);
            self
        }

        pub fn with_emotion_state(self, npc_id: &str, state: EmotionState) -> Self {
            self.repo_arc.lock().unwrap().save_emotion_state(npc_id, state);
            self
        }

        pub fn with_scene(self, scene: Scene) -> Self {
            self.repo_arc.lock().unwrap().save_scene(scene);
            self
        }

        /// Handler 실행. aggregate_key는 event에서 도출된다.
        pub fn dispatch<H: EventHandler>(
            &mut self,
            handler: &H,
            event: DomainEvent,
        ) -> Result<(HandlerResult, super::super::uow::UnitOfWork<'_, InMemoryRepository>), HandlerError> {
            let aggregate_key = event.aggregate_key();
            let prior_events: Vec<DomainEvent> = Vec::new();
            let mut uow = super::super::uow::UnitOfWork::new(&self.repo_arc);
            
            let repo_guard = self.repo_arc.lock().unwrap();
            let mut ctx = EventHandlerContext {
                world: &*repo_guard,
                emotions: &*repo_guard,
                scenes: &*repo_guard,
                event_store: &self.event_store,
                uow: &mut uow,
                prior_events: &prior_events,
                aggregate_key,
            };
            let res = handler.handle_v2(&event, &mut ctx)?;
            Ok((res, uow))
        }
    }

    impl Default for HandlerTestHarness {
        fn default() -> Self {
            Self::new()
        }
    }

    #[test]
    fn shared_state_destructive_signals_work() {
        let mut s = HandlerShared {
            emotion_state: Some(EmotionState::default()),
            ..Default::default()
        };
        s.clear_emotion_for = Some("a".into());
        s.clear_scene = true;

        assert!(s.emotion_state.is_some());
        assert_eq!(s.clear_emotion_for.as_deref(), Some("a"));
        assert!(s.clear_scene);
    }
}
