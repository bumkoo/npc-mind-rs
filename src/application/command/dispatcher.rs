// B5.1: v1 dispatch + Pipeline/Projection 관련 self-references는 모듈 내부 구현이므로
//        (외부 사용자의 deprecation warning은 여전히 발생) 모듈 전체 allow 처리.
#![allow(deprecated)]

//! CommandDispatcher — Agent 오케스트레이터
//!
//! Repository 소유, Agent 라우팅, Side-effect 적용, Event 발행을 담당합니다.
//!
//! ## B안 v1/v2 공존 (Stage B3)
//!
//! - `dispatch()`: v1 경로 — Command → Agent.handle_*() → HandlerOutput → side-effect 적용
//! - `dispatch_v2()`: v2 경로 — `Command::AppraiseRequested` / `StimulusApplyRequested`
//!   초기 이벤트로 시작 → EventHandler 체인 → staging_buffer → commit/inline/broadcast.
//!
//! v2는 현재 **Appraise, ApplyStimulus만 지원** (다른 4종 커맨드는 B4+에서 `*Requested`
//! 이벤트 variant 추가와 함께 이관). `shadow_v2` 플래그는 B4 Director가 참조할 수 있도록
//! 보존하지만 B3에서는 `dispatch()` 경로 분기에 사용하지 않는다(v1/v2 결과 타입 비호환).
//!
//! ## B4 Session 4 — Repository 공유 모델
//!
//! `repository: Arc<Mutex<R>>`로 감싸 `dispatch_v2(&self)`가 가능하도록 interior mutability.
//! SceneTask가 `Arc<CommandDispatcher<R>>`를 공유하여 Scene 간 repo 동시 접근을 직렬화한다.
//!
//! **Lock 보유 범위 (축소판 A의 의도된 대가):** `dispatch_v2` 본문은 진입 시 Mutex를
//! 한 번 잡아 transactional + inline phase 전체에 보유한다. Fanout(broadcast::send) 직전
//! drop. 결과:
//! - 한 Scene 내부 커맨드는 자연스럽게 순차 처리.
//! - **Scene 간에도 dispatch_v2가 Mutex 기준으로 serialize**되므로, b-plan §3이 약속한
//!   "Scene별 진짜 병렬"은 이 세션 범위에선 달성되지 않는다. LLM I/O 같은 bottleneck은
//!   DialogueAgent가 SceneTask 경계 **밖**에서 await하므로 여전히 병렬 유지.
//! - 진짜 CPU 병렬성은 MindRepository trait을 `&self` 시그니처로 전환(B5.3 이후)한 뒤
//!   per-aggregate 세분화 락 또는 Scene 소유권 분할로 달성 예정.

use crate::domain::aggregate::AggregateKey;
use crate::domain::event::{DomainEvent, EventPayload};
use crate::ports::{Appraiser, MindRepository};

use super::super::event_bus::EventBus;
use super::super::event_store::EventStore;
use super::super::mind_service::MindServiceError;
#[allow(deprecated)]
use super::super::pipeline::{Pipeline, PipelineState};
#[allow(deprecated)]
use super::super::projection::ProjectionRegistry;
use super::super::situation_service::SituationService;
use super::agents::{EmotionAgent, GuideAgent, RelationshipAgent, SceneAgent, StimulusAgent};
#[allow(deprecated)]
use super::handler::{HandlerContext, HandlerOutput};
use super::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerShared,
};
use super::projection_handlers::{
    EmotionProjectionHandler, RelationshipProjectionHandler, SceneProjectionHandler,
};
use super::types::{Command, CommandResult};

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};

// ---------------------------------------------------------------------------
// dispatch_v2 안전 한계
// ---------------------------------------------------------------------------

/// 이벤트 chain의 최대 cascade 깊이 (handler follow-up).
/// 초과 시 무한 루프·설계 오류 의심 → `DispatchV2Error::CascadeTooDeep`.
pub const MAX_CASCADE_DEPTH: u32 = 4;

/// 단일 커맨드에서 발행 가능한 최대 이벤트 수.
/// 초과 시 폭주 의심 → `DispatchV2Error::EventBudgetExceeded`.
pub const MAX_EVENTS_PER_COMMAND: usize = 20;

// ---------------------------------------------------------------------------
// DispatchV2 타입
// ---------------------------------------------------------------------------

/// v2 dispatch 결과 — 발행된 이벤트들과 핸들러 공유 상태
#[derive(Debug)]
pub struct DispatchV2Output {
    /// Commit 단계에서 event_store에 append된 최종 이벤트 목록 (정렬: transactional 실행 순서)
    pub events: Vec<DomainEvent>,
    /// 핸들러 간 공유 상태의 최종 스냅샷
    pub shared: HandlerShared,
}

/// v2 dispatch 에러
#[derive(Debug, thiserror::Error)]
pub enum DispatchV2Error {
    /// 해당 Command가 v2 경로에서 아직 지원되지 않음 (B4+에서 추가 예정)
    #[error("unsupported command in v2: {0}")]
    UnsupportedCommand(&'static str),

    /// 초기 이벤트 생성 시 Situation 해석 실패
    #[error("invalid situation: {0}")]
    InvalidSituation(String),

    /// Transactional handler chain이 안전 depth 초과
    #[error("cascade depth exceeded: {depth} > {max}", max = MAX_CASCADE_DEPTH)]
    CascadeTooDeep { depth: u32 },

    /// 발행 이벤트 수가 안전 budget 초과
    #[error("event budget exceeded: {limit}", limit = MAX_EVENTS_PER_COMMAND)]
    EventBudgetExceeded,

    /// Transactional handler가 에러 반환 → 커맨드 전체 중단 (staging_buffer 폐기)
    #[error("handler '{handler}' failed: {source}")]
    HandlerFailed {
        handler: &'static str,
        #[source]
        source: HandlerError,
    },
}

/// Command 기반 오케스트레이터
///
/// MindService의 대체 진입점. Agent에게 도메인 로직을 위임하고,
/// 결과를 repository에 write-back + EventStore/EventBus로 발행합니다.
pub struct CommandDispatcher<R: MindRepository> {
    /// B4 Session 4: 내부 `Arc<Mutex<R>>`로 공유 소유. `&self` dispatch_v2 경로가
    /// 가능하도록 repository mutation을 interior mutability로 가림. SceneTask가
    /// `Arc<CommandDispatcher<R>>`를 공유할 수 있는 전제.
    repository: Arc<Mutex<R>>,
    emotion_agent: EmotionAgent,
    guide_agent: GuideAgent,
    rel_agent: RelationshipAgent,
    situation_service: SituationService,
    event_store: Arc<dyn EventStore>,
    event_bus: Arc<EventBus>,
    projections: Arc<RwLock<ProjectionRegistry>>,
    /// B4 Session 4: `&mut self` 제거를 위해 AtomicU64로 전환. 0 = None.
    correlation_id: Arc<AtomicU64>,
    // --- B3 v2 경로 ---
    /// Transactional mode EventHandler들 (priority 오름차순 정렬)
    transactional_handlers: Vec<Arc<dyn EventHandler>>,
    /// Inline mode EventHandler들 (priority 오름차순 정렬)
    inline_handlers: Vec<Arc<dyn EventHandler>>,
    /// B4 Director가 v2 경로를 기본으로 쓸지 결정하는 플래그. B3에서는 dispatch()의
    /// 동작을 바꾸지 않으며(v1/v2 결과 타입 불일치), 외부 관찰자 힌트로만 기능한다.
    shadow_v2: bool,
}

impl<R: MindRepository> CommandDispatcher<R> {
    pub fn new(
        repository: R,
        event_store: Arc<dyn EventStore>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(repository)),
            emotion_agent: EmotionAgent::new(),
            guide_agent: GuideAgent::new(),
            rel_agent: RelationshipAgent::new(),
            situation_service: SituationService::new(),
            event_store,
            event_bus,
            projections: Arc::new(RwLock::new(ProjectionRegistry::new())),
            correlation_id: Arc::new(AtomicU64::new(0)),
            transactional_handlers: Vec::new(),
            inline_handlers: Vec::new(),
            shadow_v2: false,
        }
    }

    /// B1 Agent 4종 + B2 Projection wrapper 3종을 v2 경로에 일괄 등록
    ///
    /// - Transactional: Emotion/Stimulus/Guide/Relationship Agent
    /// - Inline: Emotion/Relationship/Scene Projection handler
    ///
    /// 개별 등록이 필요하면 `register_transactional` / `register_inline`을 직접 호출.
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

    /// Transactional EventHandler 등록 (priority 기준 오름차순 유지)
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

    /// Inline EventHandler 등록 (priority 기준 오름차순 유지)
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

    /// `shadow_v2` 플래그 설정
    ///
    /// **NOTE (B3):** 이 플래그는 B3 시점 `dispatch()` 경로 분기에 사용되지 **않는다**
    /// (v1 반환 `CommandResult`와 v2 반환 `DispatchV2Output`이 타입 호환 안 됨). B4에서
    /// `Director`가 v2를 기본 경로로 쓸지 결정할 때 참조할 **힌트 상태**로만 기능한다.
    /// 따라서 현재는 테스트에서만 값을 확인 가능. 외부 통합 사용자는 `dispatch_v2()`를
    /// 직접 호출하거나 B4를 기다릴 것.
    pub fn with_shadow_v2(mut self, enabled: bool) -> Self {
        self.shadow_v2 = enabled;
        self
    }

    /// 현재 `shadow_v2` 플래그 — B3에서는 observable effect 없음. `with_shadow_v2` 참조.
    pub fn shadow_v2(&self) -> bool {
        self.shadow_v2
    }

    /// 등록된 transactional handler 수 (테스트·관찰용)
    pub fn transactional_handler_count(&self) -> usize {
        self.transactional_handlers.len()
    }

    /// 등록된 inline handler 수 (테스트·관찰용)
    pub fn inline_handler_count(&self) -> usize {
        self.inline_handlers.len()
    }

    /// L1 Projection Registry 주입 (옵션) — **v1, deprecated**
    ///
    /// **B5.1 (v0.2.0):** v2 `register_inline()` + `EmotionProjectionHandler` 등으로 대체.
    #[deprecated(
        since = "0.2.0",
        note = "v2 `register_inline()` + `EmotionProjectionHandler`/`RelationshipProjectionHandler`/`SceneProjectionHandler` wrapper로 대체됨. v0.3.0에서 제거 예정."
    )]
    #[allow(deprecated)]
    pub fn with_projections(mut self, projections: Arc<RwLock<ProjectionRegistry>>) -> Self {
        self.projections = projections;
        self
    }

    /// 단일 Projection을 L1 registry에 추가 — **v1, deprecated**
    #[deprecated(
        since = "0.2.0",
        note = "v2 `register_inline(handler)` 사용. `EmotionProjectionHandler::new()` 등으로 wrapper를 감싸 등록. v0.3.0에서 제거 예정."
    )]
    #[allow(deprecated)]
    pub fn register_projection(&self, projection: impl crate::application::projection::Projection + 'static) {
        self.projections.write().unwrap().add(projection);
    }

    /// B4 Session 4: `&self` + AtomicU64로 전환 (SceneTask 공유 호환).
    /// `id == 0`은 "correlation id 없음"으로 해석됨 (DomainEvent 직렬화 시 0은 생략).
    pub fn set_correlation_id(&self, id: u64) {
        self.correlation_id.store(id, Ordering::Relaxed);
    }

    /// 현재 correlation id (0 = 없음)
    fn current_correlation_id(&self) -> Option<u64> {
        let v = self.correlation_id.load(Ordering::Relaxed);
        if v == 0 { None } else { Some(v) }
    }

    pub fn event_store(&self) -> &Arc<dyn EventStore> {
        &self.event_store
    }

    pub fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    /// v1 Projection Registry 참조 — **deprecated**
    #[deprecated(
        since = "0.2.0",
        note = "v2 `inline_handlers`로 대체됨. v0.3.0에서 제거 예정."
    )]
    #[allow(deprecated)]
    pub fn projections(&self) -> &Arc<RwLock<ProjectionRegistry>> {
        &self.projections
    }

    /// Repository Arc 공유 참조 — SceneTask 등에서 공유 소유가 필요할 때.
    pub fn repository_arc(&self) -> Arc<Mutex<R>> {
        Arc::clone(&self.repository)
    }

    /// Repository mutable guard — `add_npc`/`save_relationship` 등 `&mut self` 메서드 호출용.
    /// 반환된 MutexGuard는 `DerefMut<Target = R>`을 제공한다.
    pub fn repository_guard(&self) -> MutexGuard<'_, R> {
        self.repository.lock().expect("repository mutex poisoned")
    }

    /// Command 디스패치 — Agent 라우팅 + side-effect + event 발행 — **v1, deprecated**
    ///
    /// **B5.1 (v0.2.0):** `dispatch_v2()` + `with_default_handlers()` 로 대체.
    /// v1 경로는 v0.3.0에서 제거 예정.
    #[deprecated(
        since = "0.2.0",
        note = "v2 `dispatch_v2()` + `with_default_handlers()` 로 대체. v0.3.0에서 제거 예정."
    )]
    #[allow(deprecated)]
    pub fn dispatch(&self, cmd: Command) -> Result<CommandResult, MindServiceError> {
        let ctx = self.build_context(&cmd)?;

        let output = match &cmd {
            Command::Appraise {
                npc_id,
                partner_id,
                situation,
            } => self.emotion_agent.handle_appraise(npc_id, partner_id, situation, &ctx)?,

            Command::ApplyStimulus {
                npc_id,
                partner_id,
                pleasure,
                arousal,
                dominance,
                situation_description,
            } => self.emotion_agent.handle_stimulus(
                npc_id,
                partner_id,
                *pleasure,
                *arousal,
                *dominance,
                situation_description,
                &ctx,
            )?,

            Command::GenerateGuide {
                npc_id,
                partner_id,
                situation_description,
            } => self
                .guide_agent
                .handle_generate(npc_id, partner_id, situation_description, &ctx)?,

            Command::UpdateRelationship {
                npc_id,
                partner_id,
                significance,
            } => self
                .rel_agent
                .handle_update(npc_id, partner_id, significance, &ctx)?,

            Command::EndDialogue {
                npc_id,
                partner_id,
                significance,
            } => self
                .rel_agent
                .handle_end_dialogue(npc_id, partner_id, significance, &ctx)?,

            Command::StartScene {
                npc_id,
                partner_id,
                significance,
                focuses,
            } => self.handle_start_scene(npc_id, partner_id, significance, focuses, &ctx)?,
        };

        // Side-effect 적용
        self.apply_side_effects(&output);

        // Event 발행
        let aggregate_id = cmd.npc_id().to_string();
        self.emit_events(&aggregate_id, &output.events);

        Ok(output.result)
    }

    // -----------------------------------------------------------------------
    // 내부 헬퍼
    // -----------------------------------------------------------------------

    fn build_context(&self, cmd: &Command) -> Result<HandlerContext, MindServiceError> {
        let npc_id = cmd.npc_id();
        let partner_id = cmd.partner_id();

        let repo = self.repository.lock().expect("repository mutex poisoned");
        let npc = repo.get_npc(npc_id);
        let relationship = repo
            .get_relationship(npc_id, partner_id)
            .or_else(|| repo.get_relationship(partner_id, npc_id));
        let emotion_state = repo.get_emotion_state(npc_id);
        let scene = repo.get_scene();
        let partner_name = repo
            .get_npc(partner_id)
            .map(|n| n.name().to_string())
            .unwrap_or_else(|| partner_id.to_string());

        Ok(HandlerContext {
            npc,
            relationship,
            emotion_state,
            scene,
            partner_name,
        })
    }

    fn apply_side_effects(&self, output: &HandlerOutput) {
        let mut repo = self.repository.lock().expect("repository mutex poisoned");
        if let Some((npc_id, state)) = &output.new_emotion_state {
            repo.save_emotion_state(npc_id, state.clone());
        }
        if let Some((owner_id, target_id, rel)) = &output.new_relationship {
            repo.save_relationship(owner_id, target_id, rel.clone());
        }
        if let Some(npc_id) = &output.clear_emotion {
            repo.clear_emotion_state(npc_id);
        }
        if output.clear_scene {
            repo.clear_scene();
        }
        if let Some(scene) = &output.save_scene {
            repo.save_scene(scene.clone());
        }
    }

    fn emit_events(&self, aggregate_id: &str, payloads: &[EventPayload]) {
        for payload in payloads {
            let id = self.event_store.next_id();
            let seq = self.event_store.next_sequence(aggregate_id);
            let mut event = DomainEvent::new(id, aggregate_id.to_string(), seq, payload.clone());
            if let Some(cid) = self.current_correlation_id() {
                event = event.with_correlation(cid);
            }
            self.event_store.append(&[event.clone()]);
            // L1: Projection 동기 갱신 — publish 이전에 수행하여 쿼리 일관성 확보
            self.projections.write().unwrap().apply_all(&event);
            // L2: broadcast fan-out — 구독자(Agent/SSE)는 자기 async 태스크에서 소비
            self.event_bus.publish(&event);
        }
    }

    /// Pipeline 실행 — 순차 에이전트 체인 — **v1, deprecated**
    ///
    /// Pipeline의 단계들을 순차 실행하고, 축적된 side-effect를
    /// repository에 적용한 뒤, 이벤트를 발행합니다.
    ///
    /// **B5.1 (v0.2.0):** v2 dispatch_v2()의 transactional handler chain (follow_up_events
    /// cascade)으로 대체됨. v0.3.0에서 제거 예정.
    #[deprecated(
        since = "0.2.0",
        note = "v2 dispatch_v2()의 transactional chain으로 대체됨. v0.3.0에서 제거 예정."
    )]
    #[allow(deprecated)]
    pub fn execute_pipeline(
        &self,
        pipeline: Pipeline,
        cmd: &Command,
    ) -> Result<CommandResult, MindServiceError> {
        let ctx = self.build_context(cmd)?;
        let initial = PipelineState::new(ctx);

        let final_state = pipeline.execute(initial)?;

        // Side-effects 적용
        {
            let mut repo = self.repository.lock().expect("repository mutex poisoned");
            if let Some((npc_id, state)) = &final_state.new_emotion_state {
                repo.save_emotion_state(npc_id, state.clone());
            }
            if let Some((owner, target, rel)) = &final_state.new_relationship {
                repo.save_relationship(owner, target, rel.clone());
            }
            if let Some(npc_id) = &final_state.clear_emotion {
                repo.clear_emotion_state(npc_id);
            }
            if final_state.clear_scene {
                repo.clear_scene();
            }
            if let Some(scene) = &final_state.save_scene {
                repo.save_scene(scene.clone());
            }
        }

        // 이벤트 발행
        let aggregate_id = cmd.npc_id().to_string();
        self.emit_events(&aggregate_id, &final_state.accumulated_events);

        final_state.final_result.ok_or_else(|| {
            MindServiceError::InvalidSituation("파이프라인이 결과를 생성하지 못했습니다.".into())
        })
    }

    // -----------------------------------------------------------------------
    // dispatch_v2 (§5.1 실행 루프)
    //   - B3: BFS cascade + 초기 2 커맨드 지원
    //   - B4 S1: 6 커맨드 전부 지원 (SceneAgent 신규 + 4 *Requested variant)
    //   - B4 S4: `&mut self` → `async fn(&self)` 전환, 내부 Arc<Mutex<R>>로 공유
    // -----------------------------------------------------------------------

    /// v2 경로 dispatch — EventHandler 체인 실행 + 결과 이벤트 persist & fanout.
    ///
    /// **지원 커맨드 (B4 S1부터 6종 전부):** Appraise / ApplyStimulus / GenerateGuide /
    /// UpdateRelationship / EndDialogue / StartScene. 각각 대응하는 `*Requested` 초기
    /// 이벤트를 생성하고 transactional handler chain을 통해 실제 결과 이벤트를 발행한다.
    ///
    /// **이벤트 흐름**:
    /// 1. Command → 초기 이벤트 (6종 `*Requested` variant 중 하나)
    /// 2. Transactional phase: BFS 큐로 handler chain 실행, staging_buffer 적재
    ///    (에러 시 전체 abort → staging_buffer 폐기, event_store 미변경)
    /// 3. Repo write-back: `HandlerShared`의 emotion_state / relationship / scene /
    ///    clear_* 시그널을 repository에 반영
    /// 4. Commit phase: staging_buffer의 각 이벤트에 실ID·sequence 할당 후 event_store.append
    /// 5. Inline phase: Projection handler 동기 호출 (에러는 로그만)
    /// 6. Fanout phase: event_bus.publish (broadcast 구독자)
    pub async fn dispatch_v2(&self, cmd: Command) -> Result<DispatchV2Output, DispatchV2Error>
    where
        R: Send + Sync,
    {
        // 1. 초기 이벤트 구성
        let initial_event = self.build_initial_event(&cmd)?;
        let aggregate_key = initial_event.aggregate_key();

        // 2. Transactional phase
        //
        // B4 Session 4: Repository 접근은 `Arc<Mutex<R>>`로 공유되므로, dispatch_v2 전체 기간 동안
        // 단일 guard를 잡아 handler들에게 `&dyn MindRepository`로 전달한다. 본문 내부에 `.await`가
        // 없으므로 sync Mutex 보유가 안전(Send 제약 위반 없음).
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

            // 각 transactional handler를 priority 순으로 실행
            // (handler 목록은 &self 불변 참조, shared/prior_events는 mut 로컬)
            for handler in self.transactional_handlers.iter() {
                if !handler.interest().matches(&event) {
                    continue;
                }
                let DeliveryMode::Transactional {
                    can_emit_follow_up, ..
                } = handler.mode()
                else {
                    continue; // 방어: transactional 목록에 Inline/Fanout 혼입된 경우 스킵
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

        // 3. HandlerShared → Repository write-back
        //
        // v1은 `HandlerOutput.new_emotion_state` 등을 Dispatcher가 명시적으로 save했다.
        // v2는 handler들이 `ctx.shared`에만 쓰므로, transactional phase 성공 후 Dispatcher가
        // 공유 상태를 repo에 반영한다. B2 Projection handler는 읽기 뷰(projection의 HashMap)만
        // 갱신하며 domain state(MindRepository)는 건드리지 않으므로 이 단계가 필요.
        Self::apply_shared_to_repository(&mut *repo_guard, &aggregate_key, &shared);

        // 4. Commit phase — 실 ID/sequence 할당 후 event_store.append
        let committed = self.commit_staging_buffer(&aggregate_key, staging_buffer);

        // 5. Inline phase (best-effort)
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

        // Lock 해제 — Fanout은 broadcast::Sender이므로 lock 보유 불필요
        drop(repo_guard);

        // 6. Fanout phase
        for event in &committed {
            self.event_bus.publish(event);
        }

        Ok(DispatchV2Output {
            events: committed,
            shared,
        })
    }

    /// Command → 초기 DomainEvent. B4.1부터 6종 전부 지원.
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
                // SceneFocusInput(DTO) → SceneFocus(domain) 변환
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

    /// Appraise용 Situation 해석 — 명시되면 사용, 없으면 Scene의 활성/초기 Focus에서 추출.
    /// v1 `EmotionAgent::handle_appraise`의 동등 로직.
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

    /// v2 전용: transactional phase에서 `ctx.shared`에 축적된 상태를 Repository로 전파.
    ///
    /// v1의 `apply_side_effects`와 등가. 각 save 메서드의 key 추출:
    /// - `save_emotion_state`: `aggregate_key.npc_id_hint()` 사용 (NPC 단위).
    /// - `save_relationship`: `Relationship::owner_id`/`target_id` 접근자 사용 (Scene 컨텍스트
    ///   와 독립적으로 관계 식별).
    /// - `save_scene`: key 불필요 (`SceneStore::save_scene`은 Scene 전체를 저장).
    ///
    /// B4.1: destructive 시그널(`clear_emotion_for`, `clear_scene`)도 처리. Save가 Clear보다
    /// **먼저** 실행되어 동일 커맨드에서 save + clear가 양립할 때 최종 상태가 cleared가 되도록.
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
        // Destructive 시그널은 save 후 적용 — DialogueEnd 등에서 "update 후 clear" 시나리오.
        if let Some(npc_id) = &shared.clear_emotion_for {
            repo.clear_emotion_state(npc_id);
        }
        if shared.clear_scene {
            repo.clear_scene();
        }
    }

    /// Staging buffer의 이벤트 각각에 실 ID/sequence를 부여하고 event_store에 append.
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

    /// StartScene — 복합 처리 (Scene 생성 + 초기 Focus appraise)
    fn handle_start_scene(
        &self,
        npc_id: &str,
        partner_id: &str,
        significance: &Option<f32>,
        focuses: &[crate::application::dto::SceneFocusInput],
        ctx: &HandlerContext,
    ) -> Result<HandlerOutput, MindServiceError> {
        use crate::domain::emotion::Scene;

        // Focus 변환
        let repo_guard = self.repository.lock().expect("repository mutex poisoned");
        let domain_focuses: Vec<_> = focuses
            .iter()
            .map(|f| {
                self.situation_service
                    .to_scene_focus(&*repo_guard, f, npc_id, partner_id)
            })
            .collect::<Result<Vec<_>, _>>()?;
        drop(repo_guard);

        let focus_count = domain_focuses.len();
        let sig = significance.unwrap_or(0.5);
        let mut scene = Scene::with_significance(
            npc_id.to_string(),
            partner_id.to_string(),
            domain_focuses,
            sig,
        );

        // Initial Focus 찾기 + appraise
        let (initial_appraise, active_focus_id) = if let Some(focus) = scene.initial_focus().cloned()
        {
            let npc = ctx.npc.as_ref().ok_or_else(|| MindServiceError::NpcNotFound(npc_id.into()))?;
            let rel = ctx.relationship.as_ref().ok_or_else(|| {
                MindServiceError::RelationshipNotFound(npc_id.into(), partner_id.into())
            })?;

            let situation = focus
                .to_situation()
                .map_err(|e| MindServiceError::InvalidSituation(e.to_string()))?;
            let emotion_state = self.emotion_agent.appraiser.appraise(
                npc.personality(),
                &situation,
                &rel.modifiers(),
            );

            let result = crate::application::dto::build_appraise_result(
                npc,
                &emotion_state,
                Some(situation.description),
                Some(rel),
                &ctx.partner_name,
                vec![],
            );

            scene.set_active_focus(focus.id.clone());
            (Some((result, emotion_state)), Some(focus.id))
        } else {
            (None, None::<String>)
        };

        let scene_event = EventPayload::SceneStarted {
            npc_id: npc_id.to_string(),
            partner_id: partner_id.to_string(),
            focus_count,
            initial_focus_id: active_focus_id.clone(),
        };

        let mut events = vec![scene_event];
        let mut new_emotion = None;

        let appraise_result = if let Some((result, emotion_state)) = initial_appraise {
            let snapshot = crate::application::command::handler::emotion_snapshot(&emotion_state);
            events.push(EventPayload::EmotionAppraised {
                npc_id: npc_id.to_string(),
                partner_id: partner_id.to_string(),
                situation_description: None,
                dominant: result
                    .dominant
                    .as_ref()
                    .map(|d| (d.emotion_type.clone(), d.intensity)),
                mood: result.mood,
                emotion_snapshot: snapshot,
            });
            new_emotion = Some((npc_id.to_string(), emotion_state));
            Some(result)
        } else {
            None
        };

        let scene_result = crate::application::dto::SceneResult {
            focus_count,
            initial_appraise: appraise_result,
            active_focus_id,
        };

        Ok(HandlerOutput {
            result: CommandResult::SceneStarted(scene_result),
            events,
            new_emotion_state: new_emotion,
            new_relationship: None,
            clear_emotion: None,
            clear_scene: false,
            save_scene: Some(scene),
        })
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
