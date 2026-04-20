//! CommandDispatcher — v2 전용 Agent 오케스트레이터
//!
//! `dispatch_v2(cmd)`로 Command → 초기 *Requested 이벤트 → Transactional BFS →
//! HandlerShared write-back → Commit → Inline projection → Fanout 순서로 처리합니다.
//!
//! ## B4 Session 4 — Repository 공유 모델
//!
//! `repository: Arc<Mutex<R>>`로 감싸 `dispatch_v2(&self)`가 가능하도록 interior mutability.
//! SceneTask가 `Arc<CommandDispatcher<R>>`를 공유하여 Scene 간 repo 동시 접근을 직렬화한다.

use crate::domain::aggregate::AggregateKey;
use crate::domain::event::{DomainEvent, EventPayload};
use crate::ports::MindRepository;

use super::super::event_bus::EventBus;
use super::super::event_store::EventStore;
use super::super::situation_service::SituationService;
use super::agents::{EmotionAgent, GuideAgent, RelationshipAgent, SceneAgent, StimulusAgent};
use super::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerShared,
};
use super::projection_handlers::{
    EmotionProjectionHandler, RelationshipProjectionHandler, SceneProjectionHandler,
};
use super::types::Command;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

// ---------------------------------------------------------------------------
// dispatch_v2 안전 한계
// ---------------------------------------------------------------------------

/// 이벤트 chain의 최대 cascade 깊이 (handler follow-up).
pub const MAX_CASCADE_DEPTH: u32 = 4;

/// 단일 커맨드에서 발행 가능한 최대 이벤트 수.
pub const MAX_EVENTS_PER_COMMAND: usize = 20;

// ---------------------------------------------------------------------------
// DispatchV2 타입
// ---------------------------------------------------------------------------

/// v2 dispatch 결과 — 발행된 이벤트들과 핸들러 공유 상태
#[derive(Debug)]
pub struct DispatchV2Output {
    /// Commit 단계에서 event_store에 append된 최종 이벤트 목록
    pub events: Vec<DomainEvent>,
    /// 핸들러 간 공유 상태의 최종 스냅샷
    pub shared: HandlerShared,
}

/// v2 dispatch 에러
#[derive(Debug, thiserror::Error)]
pub enum DispatchV2Error {
    #[error("invalid situation: {0}")]
    InvalidSituation(String),

    #[error("cascade depth exceeded: {depth} > {max}", max = MAX_CASCADE_DEPTH)]
    CascadeTooDeep { depth: u32 },

    #[error("event budget exceeded: {limit}", limit = MAX_EVENTS_PER_COMMAND)]
    EventBudgetExceeded,

    #[error("handler '{handler}' failed: {source}")]
    HandlerFailed {
        handler: &'static str,
        #[source]
        source: HandlerError,
    },
}

/// Command 기반 오케스트레이터 (v2 단일 경로)
pub struct CommandDispatcher<R: MindRepository> {
    repository: Arc<Mutex<R>>,
    situation_service: SituationService,
    event_store: Arc<dyn EventStore>,
    event_bus: Arc<EventBus>,
    correlation_id: Arc<AtomicU64>,
    transactional_handlers: Vec<Arc<dyn EventHandler>>,
    inline_handlers: Vec<Arc<dyn EventHandler>>,
}

impl<R: MindRepository> CommandDispatcher<R> {
    pub fn new(
        repository: R,
        event_store: Arc<dyn EventStore>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(repository)),
            situation_service: SituationService::new(),
            event_store,
            event_bus,
            correlation_id: Arc::new(AtomicU64::new(0)),
            transactional_handlers: Vec::new(),
            inline_handlers: Vec::new(),
        }
    }

    /// 5 Agent + 3 Projection wrapper를 기본 등록.
    pub fn with_default_handlers(mut self) -> Self {
        self = self.register_transactional(Arc::new(SceneAgent::new()));
        self = self.register_transactional(Arc::new(EmotionAgent::new()));
        self = self.register_transactional(Arc::new(StimulusAgent::new()));
        self = self.register_transactional(Arc::new(GuideAgent::new()));
        self = self.register_transactional(Arc::new(RelationshipAgent::new()));
        self = self.register_inline(Arc::new(EmotionProjectionHandler::new()));
        self = self.register_inline(Arc::new(RelationshipProjectionHandler::new()));
        self = self.register_inline(Arc::new(SceneProjectionHandler::new()));
        self
    }

    pub fn register_transactional(mut self, handler: Arc<dyn EventHandler>) -> Self {
        debug_assert!(
            matches!(handler.mode(), DeliveryMode::Transactional { .. }),
            "register_transactional called with non-Transactional handler: {}",
            handler.name()
        );
        self.transactional_handlers.push(handler);
        self.transactional_handlers
            .sort_by_key(|h| transactional_priority(h.as_ref()));
        self
    }

    pub fn register_inline(mut self, handler: Arc<dyn EventHandler>) -> Self {
        debug_assert!(
            matches!(handler.mode(), DeliveryMode::Inline { .. }),
            "register_inline called with non-Inline handler: {}",
            handler.name()
        );
        self.inline_handlers.push(handler);
        self.inline_handlers
            .sort_by_key(|h| inline_priority(h.as_ref()));
        self
    }

    pub fn transactional_handler_count(&self) -> usize {
        self.transactional_handlers.len()
    }

    pub fn inline_handler_count(&self) -> usize {
        self.inline_handlers.len()
    }

    pub fn set_correlation_id(&self, id: u64) {
        self.correlation_id.store(id, Ordering::SeqCst);
    }

    fn current_correlation_id(&self) -> Option<u64> {
        let v = self.correlation_id.load(Ordering::SeqCst);
        (v != 0).then_some(v)
    }

    pub fn event_store(&self) -> &Arc<dyn EventStore> {
        &self.event_store
    }

    pub fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    pub fn repository_arc(&self) -> Arc<Mutex<R>> {
        self.repository.clone()
    }

    pub fn repository_guard(&self) -> MutexGuard<'_, R> {
        self.repository.lock().expect("repository mutex poisoned")
    }

    /// Command를 v2 경로로 처리합니다. 6 Command 전부 지원.
    pub async fn dispatch_v2(&self, cmd: Command) -> Result<DispatchV2Output, DispatchV2Error>
    where
        R: Send + Sync,
    {
        let initial_event = self.build_initial_event(&cmd)?;
        let aggregate_key = initial_event.aggregate_key();

        let mut repo_guard = self.repository.lock().expect("repository mutex poisoned");

        let mut shared = HandlerShared::default();
        let mut prior_events: Vec<DomainEvent> = Vec::new();
        let mut event_queue: VecDeque<(u32, DomainEvent)> = VecDeque::new();
        let mut staging_buffer: Vec<DomainEvent> = Vec::new();

        event_queue.push_back((0, initial_event));

        while let Some((depth, event)) = event_queue.pop_front() {
            if depth > MAX_CASCADE_DEPTH {
                return Err(DispatchV2Error::CascadeTooDeep { depth });
            }
            if staging_buffer.len() >= MAX_EVENTS_PER_COMMAND {
                return Err(DispatchV2Error::EventBudgetExceeded);
            }

            for handler in self.transactional_handlers.iter() {
                if !handler.interest().matches(&event) {
                    continue;
                }
                let DeliveryMode::Transactional {
                    can_emit_follow_up, ..
                } = handler.mode()
                else {
                    continue;
                };

                let mut ctx = EventHandlerContext {
                    repo: &*repo_guard as &(dyn MindRepository + Send + Sync),
                    event_store: &*self.event_store,
                    shared: &mut shared,
                    prior_events: &prior_events,
                    aggregate_key: aggregate_key.clone(),
                };

                let result =
                    handler
                        .handle(&event, &mut ctx)
                        .map_err(|source| DispatchV2Error::HandlerFailed {
                            handler: handler.name(),
                            source,
                        })?;

                if can_emit_follow_up {
                    for follow_up in result.follow_up_events {
                        event_queue.push_back((depth + 1, follow_up));
                    }
                } else {
                    debug_assert!(
                        result.follow_up_events.is_empty(),
                        "handler {} declared can_emit_follow_up=false but returned events",
                        handler.name()
                    );
                }
            }

            staging_buffer.push(event.clone());
            prior_events.push(event);
        }

        Self::apply_shared_to_repository(&mut *repo_guard, &aggregate_key, &shared);

        let committed = self.commit_staging_buffer(&aggregate_key, staging_buffer);

        for event in &committed {
            for handler in self.inline_handlers.iter() {
                if !handler.interest().matches(event) {
                    continue;
                }
                if !matches!(handler.mode(), DeliveryMode::Inline { .. }) {
                    continue;
                }
                let mut ctx = EventHandlerContext {
                    repo: &*repo_guard as &(dyn MindRepository + Send + Sync),
                    event_store: &*self.event_store,
                    shared: &mut shared,
                    prior_events: &prior_events,
                    aggregate_key: aggregate_key.clone(),
                };
                if let Err(e) = handler.handle(event, &mut ctx) {
                    tracing::warn!(handler = handler.name(), error = %e, "inline handler failed");
                }
            }
        }

        drop(repo_guard);

        for event in &committed {
            self.event_bus.publish(event);
        }

        Ok(DispatchV2Output {
            events: committed,
            shared,
        })
    }

    fn build_initial_event(&self, cmd: &Command) -> Result<DomainEvent, DispatchV2Error> {
        match cmd {
            Command::Appraise {
                npc_id,
                partner_id,
                situation,
            } => {
                let resolved = self.resolve_appraise_situation(npc_id, situation)?;
                Ok(DomainEvent::new(
                    0,
                    npc_id.clone(),
                    0,
                    EventPayload::AppraiseRequested {
                        npc_id: npc_id.clone(),
                        partner_id: partner_id.clone(),
                        situation: resolved,
                    },
                ))
            }
            Command::ApplyStimulus {
                npc_id,
                partner_id,
                pleasure,
                arousal,
                dominance,
                situation_description,
            } => Ok(DomainEvent::new(
                0,
                npc_id.clone(),
                0,
                EventPayload::StimulusApplyRequested {
                    npc_id: npc_id.clone(),
                    partner_id: partner_id.clone(),
                    pad: (*pleasure, *arousal, *dominance),
                    situation_description: situation_description.clone(),
                },
            )),
            Command::GenerateGuide {
                npc_id,
                partner_id,
                situation_description,
            } => Ok(DomainEvent::new(
                0,
                npc_id.clone(),
                0,
                EventPayload::GuideRequested {
                    npc_id: npc_id.clone(),
                    partner_id: partner_id.clone(),
                    situation_description: situation_description.clone(),
                },
            )),
            Command::UpdateRelationship {
                npc_id,
                partner_id,
                significance,
            } => Ok(DomainEvent::new(
                0,
                npc_id.clone(),
                0,
                EventPayload::RelationshipUpdateRequested {
                    npc_id: npc_id.clone(),
                    partner_id: partner_id.clone(),
                    significance: *significance,
                },
            )),
            Command::EndDialogue {
                npc_id,
                partner_id,
                significance,
            } => Ok(DomainEvent::new(
                0,
                npc_id.clone(),
                0,
                EventPayload::DialogueEndRequested {
                    npc_id: npc_id.clone(),
                    partner_id: partner_id.clone(),
                    significance: *significance,
                },
            )),
            Command::StartScene {
                npc_id,
                partner_id,
                significance,
                focuses,
            } => {
                use crate::domain::emotion::Scene;
                let repo_guard = self.repository.lock().expect("repository mutex poisoned");
                let domain_focuses: Vec<_> = focuses
                    .iter()
                    .map(|f| {
                        self.situation_service
                            .to_scene_focus(&*repo_guard, f, npc_id, partner_id)
                            .map_err(|e| DispatchV2Error::InvalidSituation(e.to_string()))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                drop(repo_guard);

                let sig = significance.unwrap_or(0.5);
                let prebuilt_scene = Scene::with_significance(
                    npc_id.clone(),
                    partner_id.clone(),
                    domain_focuses,
                    sig,
                );
                let initial_focus_id = prebuilt_scene.initial_focus().map(|f| f.id.clone());

                Ok(DomainEvent::new(
                    0,
                    npc_id.clone(),
                    0,
                    EventPayload::SceneStartRequested {
                        npc_id: npc_id.clone(),
                        partner_id: partner_id.clone(),
                        significance: *significance,
                        initial_focus_id,
                        prebuilt_scene,
                    },
                ))
            }
        }
    }

    fn resolve_appraise_situation(
        &self,
        npc_id: &str,
        situation: &Option<crate::application::dto::SituationInput>,
    ) -> Result<crate::domain::emotion::Situation, DispatchV2Error> {
        match situation {
            Some(sit) => sit
                .to_domain(None, None, None, npc_id)
                .map_err(|e| DispatchV2Error::InvalidSituation(e.to_string())),
            None => {
                let scene = self
                    .repository
                    .lock()
                    .expect("repository mutex poisoned")
                    .get_scene()
                    .ok_or_else(|| {
                        DispatchV2Error::InvalidSituation(
                            "situation이 생략되었으나 활성 Scene이 없습니다.".into(),
                        )
                    })?;
                let focus = scene
                    .active_focus_id()
                    .and_then(|id| scene.focuses().iter().find(|f| f.id == id).cloned())
                    .or_else(|| scene.initial_focus().cloned())
                    .ok_or_else(|| {
                        DispatchV2Error::InvalidSituation("활성/초기 Focus가 없습니다.".into())
                    })?;
                focus
                    .to_situation()
                    .map_err(|e| DispatchV2Error::InvalidSituation(e.to_string()))
            }
        }
    }

    fn apply_shared_to_repository(
        repo: &mut R,
        aggregate_key: &AggregateKey,
        shared: &HandlerShared,
    ) {
        if let Some(state) = &shared.emotion_state {
            let npc_id = aggregate_key.npc_id_hint();
            repo.save_emotion_state(npc_id, state.clone());
        }
        if let Some(rel) = &shared.relationship {
            repo.save_relationship(rel.owner_id(), rel.target_id(), rel.clone());
        }
        if let Some(scene) = &shared.scene {
            repo.save_scene(scene.clone());
        }
        if let Some(npc_id) = &shared.clear_emotion_for {
            repo.clear_emotion_state(npc_id);
        }
        if shared.clear_scene {
            repo.clear_scene();
        }
    }

    fn commit_staging_buffer(
        &self,
        aggregate_key: &AggregateKey,
        staging: Vec<DomainEvent>,
    ) -> Vec<DomainEvent> {
        let aggregate_id = aggregate_key.npc_id_hint().to_string();
        let mut committed = Vec::with_capacity(staging.len());
        for event in staging {
            let id = self.event_store.next_id();
            let seq = self.event_store.next_sequence(&aggregate_id);
            let mut e = DomainEvent::new(id, aggregate_id.clone(), seq, event.payload);
            if let Some(cid) = self.current_correlation_id() {
                e = e.with_correlation(cid);
            }
            self.event_store.append(&[e.clone()]);
            committed.push(e);
        }
        committed
    }
}

// ---------------------------------------------------------------------------
// 내부 헬퍼 — handler priority 추출 (register_* 정렬용)
// ---------------------------------------------------------------------------

fn transactional_priority(h: &dyn EventHandler) -> i32 {
    match h.mode() {
        DeliveryMode::Transactional { priority, .. } => priority,
        _ => 0,
    }
}

fn inline_priority(h: &dyn EventHandler) -> i32 {
    match h.mode() {
        DeliveryMode::Inline { priority } => priority,
        _ => 0,
    }
}
