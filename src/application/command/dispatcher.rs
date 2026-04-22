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
use super::agents::{
    EmotionAgent, GuideAgent, InformationAgent, RelationshipAgent, RumorAgent, SceneAgent,
    StimulusAgent, WorldOverlayAgent,
};
use super::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerShared,
};
use super::projection_handlers::{
    EmotionProjectionHandler, RelationshipProjectionHandler, SceneProjectionHandler,
};
use super::relationship_memory_handler::RelationshipMemoryHandler;
use super::rumor_distribution_handler::RumorDistributionHandler;
use super::scene_consolidation_handler::SceneConsolidationHandler;
use super::telling_ingestion_handler::TellingIngestionHandler;
use super::types::Command;
use super::world_overlay_handler::WorldOverlayHandler;
use crate::domain::rumor::{ReachPolicy, RumorOrigin};
use crate::ports::{MemoryStore, RumorStore};

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
    /// 커맨드별 고유 suffix 생성용 내부 카운터. `Command::SeedRumor`의 pending_id 등에
    /// 쓰이며, event_store의 next_id와 별개로 관리되어 **event id gap을 유발하지 않는다**
    /// (Step C3 사후 리뷰 M1). 프로세스 수명 동안만 유일, replay 시 리셋되므로 id
    /// 결정성은 §15 결정 유보 범위.
    command_seq: Arc<AtomicU64>,
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
            command_seq: Arc::new(AtomicU64::new(1)),
            transactional_handlers: Vec::new(),
            inline_handlers: Vec::new(),
        }
    }

    /// 6 Agent + 3 Projection wrapper를 기본 등록.
    ///
    /// Step C2 이후: `InformationAgent`도 기본 포함. Memory 인덱싱 Inline 핸들러
    /// (`TellingIngestionHandler`)는 `MemoryStore` 주입이 필요하므로 `with_memory()`
    /// 빌더로 따로 부착한다.
    pub fn with_default_handlers(mut self) -> Self {
        self = self.register_transactional(Arc::new(SceneAgent::new()));
        self = self.register_transactional(Arc::new(EmotionAgent::new()));
        self = self.register_transactional(Arc::new(StimulusAgent::new()));
        self = self.register_transactional(Arc::new(GuideAgent::new()));
        self = self.register_transactional(Arc::new(RelationshipAgent::new()));
        self = self.register_transactional(Arc::new(InformationAgent::new()));
        self = self.register_transactional(Arc::new(WorldOverlayAgent::new()));
        self = self.register_inline(Arc::new(EmotionProjectionHandler::new()));
        self = self.register_inline(Arc::new(RelationshipProjectionHandler::new()));
        self = self.register_inline(Arc::new(SceneProjectionHandler::new()));
        self
    }

    /// Memory 저장소 연동 — **TellingIngestionHandler만** 부착 (Step C2 호환).
    ///
    /// Step C2부터 존재한 lean 경로. `Command::TellInformation`으로 생성되는
    /// `InformationTold` 이벤트를 받아 청자별 `MemoryEntry(Heard/Rumor)`를 저장한다.
    ///
    /// Step D의 추가 핸들러(WorldOverlay/RelationshipMemory/SceneConsolidation)는 이
    /// 빌더가 **등록하지 않는다**. 해당 기능을 함께 쓰려면 `with_memory_full(store)`를
    /// 대신 호출한다 (리뷰 H5: 기존 콜러의 semantic break 방지).
    ///
    /// MemoryStore가 없는 환경(테스트·단순 시나리오)에서는 이 빌더를 호출하지 않으면
    /// `Command::TellInformation`은 `InformationTold` 이벤트만 발행되고 실제 저장은
    /// 건너뛴다.
    pub fn with_memory(mut self, store: Arc<dyn MemoryStore>) -> Self {
        self = self.register_inline(Arc::new(TellingIngestionHandler::new(store)));
        self
    }

    /// Memory 저장소 연동 — Step D 전체 번들 (Telling + WorldOverlay + RelationshipMemory
    /// + SceneConsolidation).
    ///
    /// `with_memory`가 Step C2 동작만 유지하는 반면, 이 빌더는 Step D 기능 전체를 켠다.
    /// 4종 Inline 핸들러가 `priority::inline::MEMORY_INGESTION`(40) → `WORLD_OVERLAY_INGESTION`(45)
    /// → `RELATIONSHIP_MEMORY`(50) → `SCENE_CONSOLIDATION`(60) 순서로 실행된다.
    ///
    /// 부작용:
    /// - `InformationTold` → 청자 `MemoryEntry(Heard/Rumor)`
    /// - `WorldEventOccurred` → Canonical `MemoryEntry(World, Seeded)` + topic Canonical supersede
    /// - `RelationshipUpdated` → `MemoryEntry(RelationshipChange)` (Δ ≥ 0.05)
    /// - `SceneEnded` → 참여 NPC별 Layer B `SceneSummary` + Layer A `consolidated_into` 마킹
    pub fn with_memory_full(mut self, store: Arc<dyn MemoryStore>) -> Self {
        self = self.register_inline(Arc::new(TellingIngestionHandler::new(store.clone())));
        self = self.register_inline(Arc::new(WorldOverlayHandler::new(store.clone())));
        self = self.register_inline(Arc::new(RelationshipMemoryHandler::new(store.clone())));
        self = self.register_inline(Arc::new(SceneConsolidationHandler::new(store)));
        self
    }

    /// 소문(Rumor) 서브시스템 연동 (Step C3~).
    ///
    /// 두 핸들러를 등록한다:
    /// - **`RumorAgent`** (Transactional) — `Seed/SpreadRumorRequested` 처리,
    ///   `Rumor` 애그리거트를 `RumorStore`에 저장하고 `RumorSeeded`/`RumorSpread`
    ///   follow-up을 발행.
    /// - **`RumorDistributionHandler`** (Inline) — `RumorSpread` 구독해 각 수신자에게
    ///   `MemoryEntry(Rumor)`를 `MemoryStore`에 저장 (content 해소는 §2.6 규칙을 따름).
    ///
    /// `MemoryStore`와 `RumorStore` 둘 다 필요하다. 둘이 없는 환경에서는
    /// `register_transactional`/`register_inline`으로 개별 등록 가능.
    pub fn with_rumor(
        mut self,
        memory_store: Arc<dyn MemoryStore>,
        rumor_store: Arc<dyn RumorStore>,
    ) -> Self {
        self = self.register_transactional(Arc::new(RumorAgent::new(rumor_store.clone())));
        self = self.register_inline(Arc::new(RumorDistributionHandler::new(
            memory_store,
            rumor_store,
        )));
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
            Command::TellInformation(req) => Ok(DomainEvent::new(
                0,
                req.speaker.clone(),
                0,
                EventPayload::TellInformationRequested {
                    speaker: req.speaker.clone(),
                    listeners: req.listeners.clone(),
                    overhearers: req.overhearers.clone(),
                    claim: req.claim.clone(),
                    stated_confidence: req.stated_confidence.clamp(0.0, 1.0),
                    origin_chain_in: req.origin_chain_in.clone(),
                    topic: req.topic.clone(),
                },
            )),
            Command::SeedRumor(req) => {
                // DTO→도메인 변환은 `impl From<&RumorOriginInput>` / `<&RumorReachInput>`
                // 가 담당 (C3 리뷰 m2에서 인라인 match 제거).
                let origin: RumorOrigin = (&req.origin).into();
                let reach: ReachPolicy = (&req.reach).into();
                // 고아 Rumor는 seed_content 필수 — DTO 단계에서 빠르게 reject.
                if req.topic.is_none() && req.seed_content.is_none() {
                    return Err(DispatchV2Error::InvalidSituation(
                        "SeedRumor: topic 없으면 seed_content 필수".into(),
                    ));
                }
                // 커맨드별 고유 pending_id — 복수의 SeedRumor가 "orphan" 공용 버킷을
                // 공유하지 않도록 (Step C3 사후 리뷰 C2).
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
                        topic: req.topic.clone(),
                        seed_content: req.seed_content.clone(),
                        reach,
                        origin,
                    },
                ))
            }
            Command::SpreadRumor(req) => Ok(DomainEvent::new(
                0,
                req.rumor_id.clone(),
                0,
                EventPayload::SpreadRumorRequested {
                    rumor_id: req.rumor_id.clone(),
                    extra_recipients: req.recipients.clone(),
                },
            )),
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
                Ok(DomainEvent::new(
                    0,
                    req.world_id.clone(),
                    0,
                    EventPayload::ApplyWorldEventRequested {
                        world_id: req.world_id.clone(),
                        topic: req.topic.clone(),
                        fact: req.fact.clone(),
                        significance: req.significance.clamp(0.0, 1.0),
                        witnesses: req.witnesses.clone(),
                    },
                ))
            }
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
        _command_key: &AggregateKey,
        staging: Vec<DomainEvent>,
    ) -> Vec<DomainEvent> {
        // 각 이벤트의 aggregate_id는 **payload의 자기 aggregate_key**로 결정한다.
        // 커맨드의 aggregate_key는 참고용이며 덮어쓰기에 쓰지 않는다 — 그래야
        // `EventStore.get_events(listener)` 같은 청자 기반 질의가 §3.3 B5
        // (`InformationTold → Npc(listener)`)를 정확히 반영한다. 기존 이벤트
        // (EmotionAppraised·BeatTransitioned·RelationshipUpdated 등)는 payload의
        // `npc_id_hint`가 커맨드의 것과 같아서 저장값이 변하지 않는다.
        let mut committed = Vec::with_capacity(staging.len());
        for event in staging {
            let per_event_id = event.aggregate_key().npc_id_hint().to_string();
            let id = self.event_store.next_id();
            let seq = self.event_store.next_sequence(&per_event_id);
            let mut e = DomainEvent::new(id, per_event_id, seq, event.payload);
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
