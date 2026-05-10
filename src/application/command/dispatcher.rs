//! CommandDispatcher — Command를 도메인 이벤트로 변환하고 핸들러 체인을 실행 (v2)
//!
//! 설계 원칙:
//! 1. **Transactional 원자성**: Transactional 핸들러 체인은 단일 Unit of Work에 묶여
//!    커밋 시점에 일괄 저장된다.
//! 2. **Cascade 깊이 제한**: 이벤트 연쇄가 무한 루프에 빠지지 않도록 `MAX_CASCADE_DEPTH`로 가드.
//! 3. **Inline Projections**: 커밋 직후(Fanout 전) 인라인으로 프로젝션을 실행해 쿼리 일관성 확보.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use super::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerShared,
};
pub use super::types::{DispatchV2Error, DispatchV2Output};
use super::types::Command;
use crate::application::event_bus::EventBus;
use crate::application::event_store::EventStore;
use crate::application::situation_service::SituationService;
use crate::domain::aggregate::AggregateKey;
use crate::domain::emotion::{Situation, Scene};
use crate::domain::event::{DomainEvent, EventPayload};
use crate::ports::{MindRepository, EmotionStore, NpcWorld, SceneStore};
use crate::application::error::MindServiceError;
use crate::application::dto::SituationInput;

// ---------------------------------------------------------------------------
// dispatch_v2 안전 한계
// ---------------------------------------------------------------------------

/// 이벤트 chain의 최대 cascade 깊이 (handler follow-up).
pub const MAX_CASCADE_DEPTH: u32 = 4;

/// 단일 커맨드에서 발행 가능한 최대 이벤트 수.
///
/// Phase 1 Mind Architecture (Stage 0 Findings F3 (아) / spec §11.7) — `EndDialogue`
/// 경로의 worst-case가 7~8 이벤트 (DialogueEndRequested + RelationshipUpdated +
/// EmotionCleared + SceneEnded + 3 inline projection). `DialogueReflected`가
/// 항상 발행되어 1개 추가됨 → 8~9. 안전 마진 큼. 21 → 22로 인상.
pub const MAX_EVENTS_PER_COMMAND: usize = 22;

pub struct CommandDispatcher<R: MindRepository> {
    repository: Arc<Mutex<R>>,
    event_store: Arc<dyn EventStore>,
    event_bus: Arc<EventBus>,
    command_seq: AtomicU64,
    transactional_handlers: Vec<Arc<dyn EventHandler>>,
    inline_handlers: Vec<Arc<dyn EventHandler>>,
    situation_service: SituationService,
}

struct DispatchState {
    staging_buffer: Vec<DomainEvent>,
    parent_indices: Vec<Option<usize>>,
    depths: Vec<u32>,
}

impl Default for DispatchState {
    fn default() -> Self {
        Self {
            staging_buffer: Vec::with_capacity(8),
            parent_indices: Vec::with_capacity(8),
            depths: Vec::with_capacity(8),
        }
    }
}

impl<R: MindRepository> CommandDispatcher<R> {
    pub fn new(
        repository: Arc<Mutex<R>>,
        event_store: Arc<dyn EventStore>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            repository,
            event_store,
            event_bus,
            command_seq: AtomicU64::new(1),
            transactional_handlers: Vec::new(),
            inline_handlers: Vec::new(),
            situation_service: SituationService::new(),
        }
    }

    /// 기본 탑재 핸들러들을 등록하여 반환 (Step C2 호환).
    pub fn with_default_handlers(mut self) -> Self {
        use super::policies::*;
        use super::projection_handlers::*;

        self = self.register_transactional(Arc::new(EmotionPolicy::new()));
        self = self.register_transactional(Arc::new(GuidePolicy::new()));
        self = self.register_transactional(Arc::new(RelationshipPolicy::new()));
        self = self.register_transactional(Arc::new(ScenePolicy::new()));
        self = self.register_transactional(Arc::new(StimulusPolicy::new()));
        self = self.register_transactional(Arc::new(WorldOverlayPolicy::new()));
        self = self.register_transactional(Arc::new(InformationPolicy::new()));

        self = self.register_inline(Arc::new(EmotionProjectionHandler::new()));
        self = self.register_inline(Arc::new(RelationshipProjectionHandler::new()));
        self = self.register_inline(Arc::new(SceneProjectionHandler::new()));
        self
    }

    /// Memory 저장소 연동 — **TellingIngestionHandler만** 부착 (Step C2 호환).
    pub fn with_memory(mut self, store: Arc<dyn crate::ports::MemoryStore>) -> Self {
        self = self.register_inline(Arc::new(super::telling_ingestion_handler::TellingIngestionHandler::new(store)));
        self
    }

    /// Memory 저장소 연동 — **TellingIngestionHandler + WorldOverlayHandler + SceneConsolidationHandler** 부착.
    pub fn with_memory_full(mut self, store: Arc<dyn crate::ports::MemoryStore>) -> Self {
        self = self.with_memory(store.clone());
        self = self.with_world_overlay(store.clone());
        self = self.with_scene_consolidation(store);
        self
    }

    /// Rumor 저장소 연동 — **RumorPolicy + RumorDistributionHandler** 부착 (Step C3).
    pub fn with_rumor(
        mut self,
        memory_store: Arc<dyn crate::ports::MemoryStore>,
        rumor_store: Arc<dyn crate::ports::RumorStore>,
    ) -> Self {
        self = self.register_transactional(Arc::new(super::policies::RumorPolicy::new(rumor_store.clone())));
        self = self.register_inline(Arc::new(
            super::rumor_distribution_handler::RumorDistributionHandler::new(memory_store, rumor_store),
        ));
        self
    }

    /// World 오버레이 연동 — **WorldOverlayHandler** 부착 (Step D).
    pub fn with_world_overlay(mut self, store: Arc<dyn crate::ports::MemoryStore>) -> Self {
        self = self.register_inline(Arc::new(super::world_overlay_handler::WorldOverlayHandler::new(store)));
        self
    }

    /// Scene 통합 연동 — **SceneConsolidationHandler** 부착 (Step D).
    pub fn with_scene_consolidation(mut self, store: Arc<dyn crate::ports::MemoryStore>) -> Self {
        self = self.register_inline(Arc::new(super::scene_consolidation_handler::SceneConsolidationHandler::new(store)));
        self
    }

    pub fn register_transactional(mut self, handler: Arc<dyn EventHandler>) -> Self {
        self.transactional_handlers.push(handler);
        self.transactional_handlers
            .sort_by_key(|h| transactional_priority(h.as_ref()));
        self
    }

    pub fn register_inline(mut self, handler: Arc<dyn EventHandler>) -> Self {
        self.inline_handlers.push(handler);
        self.inline_handlers
            .sort_by_key(|h| inline_priority(h.as_ref()));
        self
    }

    pub fn event_store(&self) -> &Arc<dyn EventStore> {
        &self.event_store
    }

    pub fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    pub fn transactional_handler_count(&self) -> usize {
        self.transactional_handlers.len()
    }

    pub fn inline_handler_count(&self) -> usize {
        self.inline_handlers.len()
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
        // 호출 단위 correlation_id 발급 — 함수 진입 직후 1회.
        let cid = self.command_seq.fetch_add(1, Ordering::SeqCst);

        let initial_event = self.build_initial_event(cmd)?;
        let aggregate_key = initial_event.aggregate_key();

        let mut uow = super::uow::UnitOfWork::new(&self.repository);
        let mut state = DispatchState::default();

        // 1. Transactional Phase (BFS)
        {
            let mut repo_guard = self.repository.lock().expect("repository mutex poisoned");
            self.execute_transactional_bfs(
                &initial_event,
                &aggregate_key,
                &mut *repo_guard,
                &mut uow,
                &mut state,
            )?;
        }

        // DispatchV2Output 호환을 위해 UoW에서 HandlerShared 복구 (commit 전 추출)
        let shared = HandlerShared {
            emotion_state: uow.emotion_state.as_ref().map(|(_, s)| s.clone()),
            relationship: uow.relationship.clone(),
            scene: uow.scene.clone(),
            guide: uow.guide.clone(),
            clear_emotion_for: uow.clear_emotion_for.clone(),
            clear_scene: uow.clear_scene,
        };

        // 2. Commit Phase (Repository + EventStore)
        // Unit of Work를 통해 도메인 상태 원자적 저장
        uow.commit().map_err(|source| DispatchV2Error::HandlerFailed {
            handler: "CommandDispatcher::Commit",
            source,
        })?;

        let committed = self.commit_staging_buffer(
            &aggregate_key,
            state.staging_buffer,
            cid,
            state.parent_indices,
            state.depths,
        );

        // 3. Inline Phase (Projections)
        {
            let mut uow_inline = super::uow::UnitOfWork::new(&self.repository);
            let mut repo_guard = self.repository.lock().expect("repository mutex poisoned");
            self.execute_inline_projections(&committed, &aggregate_key, &mut *repo_guard, &mut uow_inline);
        }

        // 4. Fanout Phase (EventBus)
        for event in &committed {
            self.event_bus.publish(event);
        }

        Ok(DispatchV2Output {
            events: committed,
            shared,
        })
    }

    fn build_initial_event(&self, cmd: Command) -> Result<DomainEvent, DispatchV2Error> {
        match cmd {
            Command::Appraise {
                npc_id,
                partner_id,
                situation,
            } => {
                let resolved = self.resolve_appraise_situation(&npc_id, situation)?;
                Ok(DomainEvent::new(
                    0,
                    npc_id.clone(),
                    0,
                    EventPayload::AppraiseRequested {
                        npc_id,
                        partner_id,
                        situation: Box::new(resolved),
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
                    pad: (pleasure, arousal, dominance),
                    situation_description,
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
                    situation_description,
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
                    significance,
                },
            )),
            Command::EndDialogue {
                npc_id,
                partner_id,
                significance,
                reflection,
            } => Ok(DomainEvent::new(
                0,
                npc_id.clone(),
                0,
                EventPayload::DialogueEndRequested {
                    npc_id: npc_id.clone(),
                    partner_id: partner_id.clone(),
                    significance,
                    reflection,
                },
            )),
            Command::TellInformation(req) => {
                let speaker = req.speaker.clone();
                Ok(DomainEvent::new(
                    0,
                    speaker,
                    0,
                    EventPayload::TellInformationRequested {
                        speaker: req.speaker,
                        listeners: req.listeners,
                        overhearers: req.overhearers,
                        claim: req.claim,
                        stated_confidence: req.stated_confidence.clamp(0.0, 1.0),
                        origin_chain_in: req.origin_chain_in,
                        topic: req.topic,
                    },
                ))
            }
            Command::SeedRumor(req) => {
                let origin: crate::domain::rumor::RumorOrigin = (&req.origin).into();
                let reach: crate::domain::rumor::ReachPolicy = (&req.reach).into();
                if req.topic.is_none() && req.seed_content.is_none() {
                    return Err(DispatchV2Error::InvalidSituation(
                        "SeedRumor: topic 없으면 seed_content 필수".into(),
                    ));
                }
                let pending_id = format!(
                    "{:012}",
                    self.command_seq.fetch_add(1, Ordering::SeqCst)
                );
                let agg_id = format!("pending-{pending_id}");
                Ok(DomainEvent::new(
                    0,
                    agg_id,
                    0,
                    EventPayload::SeedRumorRequested {
                        pending_id,
                        topic: req.topic,
                        seed_content: req.seed_content,
                        reach,
                        origin,
                    },
                ))
            }
            Command::SpreadRumor(req) => {
                let rumor_id = req.rumor_id.clone();
                Ok(DomainEvent::new(
                    0,
                    rumor_id,
                    0,
                    EventPayload::SpreadRumorRequested {
                        rumor_id: req.rumor_id,
                        extra_recipients: req.recipients,
                    },
                ))
            }
            Command::ApplyWorldEvent(req) => {
                if req.world_id.is_empty() {
                    return Err(DispatchV2Error::InvalidSituation(
                        "ApplyWorldEvent: world_id가 비어 있습니다".into(),
                    ));
                }
                if req.fact.trim().is_empty() {
                    return Err(DispatchV2Error::InvalidSituation(
                        "ApplyWorldEvent: fact가 비어 있습니다".into(),
                    ));
                }
                let world_id = req.world_id.clone();
                Ok(DomainEvent::new(
                    0,
                    world_id,
                    0,
                    EventPayload::ApplyWorldEventRequested {
                        world_id: req.world_id,
                        topic: req.topic,
                        fact: req.fact,
                        significance: req.significance.clamp(0.0, 1.0),
                        witnesses: req.witnesses,
                    },
                ))
            }
            Command::StartScene {
                npc_id,
                partner_id,
                significance,
                focuses,
            } => {
                let repo_guard = self.repository.lock().expect("repository mutex poisoned");
                let domain_focuses: Vec<_> = focuses
                    .into_iter()
                    .map(|f| {
                        self.situation_service
                            .to_scene_focus(&*repo_guard, f, &npc_id, &partner_id)
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
                        npc_id,
                        partner_id,
                        significance,
                        initial_focus_id,
                        prebuilt_scene: Box::new(prebuilt_scene),
                    },
                ))
            }
        }
    }

    fn resolve_appraise_situation(
        &self,
        npc_id: &str,
        situation: Option<Box<SituationInput>>,
    ) -> Result<Situation, DispatchV2Error> {
        match situation {
            Some(sit) => sit
                .into_domain(None, None, None, npc_id)
                .map_err(|e: MindServiceError| DispatchV2Error::InvalidSituation(e.to_string())),
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

    fn commit_staging_buffer(
        &self,
        _command_key: &AggregateKey,
        staging: Vec<DomainEvent>,
        cid: u64,
        parent_indices: Vec<Option<usize>>,
        depths: Vec<u32>,
    ) -> Vec<DomainEvent> {
        debug_assert_eq!(staging.len(), parent_indices.len());
        debug_assert_eq!(staging.len(), depths.len());

        let mut committed: Vec<DomainEvent> = Vec::with_capacity(staging.len());

        for (idx, event) in staging.into_iter().enumerate() {
            let per_event_id = event.aggregate_key().npc_id_hint().to_string();
            let id = self.event_store.next_id();
            let seq = self.event_store.next_sequence(&per_event_id);
            let mut e = DomainEvent::new(id, per_event_id, seq, event.payload)
                .with_correlation(cid)
                .with_cascade_depth(depths[idx]);
            if let Some(parent_idx) = parent_indices[idx] {
                e = e.with_parent(committed[parent_idx].id);
            }
            committed.push(e);
        }

        self.event_store.append(&committed);

        committed
    }

    fn execute_transactional_bfs<'a>(
        &self,
        initial_event: &DomainEvent,
        aggregate_key: &AggregateKey,
        repo: &mut R,
        uow: &mut super::uow::UnitOfWork<'a, R>,
        state: &mut DispatchState,
    ) -> Result<(), DispatchV2Error>
    where
        R: Send + Sync,
    {
        let mut event_queue: VecDeque<(u32, DomainEvent, Option<usize>)> = VecDeque::new();
        event_queue.push_back((0, initial_event.clone(), None));

        while let Some((depth, event, parent_idx)) = event_queue.pop_front() {
            if depth > MAX_CASCADE_DEPTH {
                return Err(DispatchV2Error::CascadeTooDeep { depth });
            }
            if state.staging_buffer.len() >= MAX_EVENTS_PER_COMMAND {
                return Err(DispatchV2Error::EventBudgetExceeded);
            }

            let my_idx = state.staging_buffer.len();

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
                    world: repo as &(dyn NpcWorld + Send + Sync),
                    emotions: repo as &(dyn EmotionStore + Send + Sync),
                    scenes: repo as &(dyn SceneStore + Send + Sync),
                    event_store: &*self.event_store,
                    uow,
                    prior_events: &state.staging_buffer,
                    aggregate_key: aggregate_key.clone(),
                };

                let result =
                    handler
                        .handle_v2(&event, &mut ctx)
                        .map_err(|source| DispatchV2Error::HandlerFailed {
                            handler: handler.name(),
                            source,
                        })?;

                if can_emit_follow_up {
                    for follow_up in result.follow_up_events {
                        event_queue.push_back((depth + 1, follow_up, Some(my_idx)));
                    }
                }
            }

            state.staging_buffer.push(event);
            state.parent_indices.push(parent_idx);
            state.depths.push(depth);
        }
        Ok(())
    }

    fn execute_inline_projections<'a>(
        &self,
        committed: &[DomainEvent],
        aggregate_key: &AggregateKey,
        repo: &mut R,
        uow: &mut super::uow::UnitOfWork<'a, R>,
    ) where
        R: Send + Sync,
    {
        for (idx, event) in committed.iter().enumerate() {
            let prior_events = &committed[0..idx];

            for handler in self.inline_handlers.iter() {
                if !handler.interest().matches(event) {
                    continue;
                }
                if !matches!(handler.mode(), DeliveryMode::Inline { .. }) {
                    continue;
                }
                let mut ctx = EventHandlerContext {
                    world: repo as &(dyn NpcWorld + Send + Sync),
                    emotions: repo as &(dyn EmotionStore + Send + Sync),
                    scenes: repo as &(dyn SceneStore + Send + Sync),
                    event_store: &*self.event_store,
                    uow,
                    prior_events,
                    aggregate_key: aggregate_key.clone(),
                };
                if let Err(e) = handler.handle_v2(event, &mut ctx) {
                    tracing::warn!(handler = handler.name(), error = %e, "inline handler failed");
                }
            }
        }
    }
}

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
