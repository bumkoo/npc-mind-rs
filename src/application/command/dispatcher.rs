//! CommandDispatcher — Agent 오케스트레이터
//!
//! Repository 소유, Agent 라우팅, Side-effect 적용, Event 발행을 담당합니다.

use crate::domain::event::{DomainEvent, EventPayload};
use crate::ports::{Appraiser, MindRepository};

use super::super::event_bus::EventBus;
use super::super::event_store::EventStore;
use super::super::mind_service::MindServiceError;
use super::super::pipeline::{Pipeline, PipelineState};
use super::super::projection::ProjectionRegistry;
use super::super::situation_service::SituationService;
use super::agents::{EmotionAgent, GuideAgent, RelationshipAgent};
use super::handler::{HandlerContext, HandlerOutput};
use super::types::{Command, CommandResult};

use std::sync::{Arc, RwLock};

/// Command 기반 오케스트레이터
///
/// MindService의 대체 진입점. Agent에게 도메인 로직을 위임하고,
/// 결과를 repository에 write-back + EventStore/EventBus로 발행합니다.
pub struct CommandDispatcher<R: MindRepository> {
    repository: R,
    emotion_agent: EmotionAgent,
    guide_agent: GuideAgent,
    rel_agent: RelationshipAgent,
    situation_service: SituationService,
    event_store: Arc<dyn EventStore>,
    event_bus: Arc<EventBus>,
    projections: Arc<RwLock<ProjectionRegistry>>,
    correlation_id: Option<u64>,
}

impl<R: MindRepository> CommandDispatcher<R> {
    pub fn new(
        repository: R,
        event_store: Arc<dyn EventStore>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            repository,
            emotion_agent: EmotionAgent::new(),
            guide_agent: GuideAgent::new(),
            rel_agent: RelationshipAgent::new(),
            situation_service: SituationService::new(),
            event_store,
            event_bus,
            projections: Arc::new(RwLock::new(ProjectionRegistry::new())),
            correlation_id: None,
        }
    }

    /// L1 Projection Registry 주입 (옵션)
    ///
    /// 외부에서 이미 구성한 registry를 공유하려면 사용. 기본값은
    /// 빈 registry로, `register_projection`으로 항목을 추가할 수 있다.
    pub fn with_projections(mut self, projections: Arc<RwLock<ProjectionRegistry>>) -> Self {
        self.projections = projections;
        self
    }

    /// 단일 Projection을 L1 registry에 추가
    pub fn register_projection(&self, projection: impl crate::application::projection::Projection + 'static) {
        self.projections.write().unwrap().add(projection);
    }

    pub fn set_correlation_id(&mut self, id: u64) {
        self.correlation_id = Some(id);
    }

    pub fn event_store(&self) -> &Arc<dyn EventStore> {
        &self.event_store
    }

    pub fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    pub fn projections(&self) -> &Arc<RwLock<ProjectionRegistry>> {
        &self.projections
    }

    pub fn repository(&self) -> &R {
        &self.repository
    }

    pub fn repository_mut(&mut self) -> &mut R {
        &mut self.repository
    }

    /// Command 디스패치 — Agent 라우팅 + side-effect + event 발행
    pub fn dispatch(&mut self, cmd: Command) -> Result<CommandResult, MindServiceError> {
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

        let npc = self.repository.get_npc(npc_id);
        let relationship = self
            .repository
            .get_relationship(npc_id, partner_id)
            .or_else(|| self.repository.get_relationship(partner_id, npc_id));
        let emotion_state = self.repository.get_emotion_state(npc_id);
        let scene = self.repository.get_scene();
        let partner_name = self
            .repository
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

    fn apply_side_effects(&mut self, output: &HandlerOutput) {
        if let Some((npc_id, state)) = &output.new_emotion_state {
            self.repository.save_emotion_state(npc_id, state.clone());
        }
        if let Some((owner_id, target_id, rel)) = &output.new_relationship {
            self.repository
                .save_relationship(owner_id, target_id, rel.clone());
        }
        if let Some(npc_id) = &output.clear_emotion {
            self.repository.clear_emotion_state(npc_id);
        }
        if output.clear_scene {
            self.repository.clear_scene();
        }
        if let Some(scene) = &output.save_scene {
            self.repository.save_scene(scene.clone());
        }
    }

    fn emit_events(&self, aggregate_id: &str, payloads: &[EventPayload]) {
        for payload in payloads {
            let id = self.event_store.next_id();
            let seq = self.event_store.next_sequence(aggregate_id);
            let mut event = DomainEvent::new(id, aggregate_id.to_string(), seq, payload.clone());
            if let Some(cid) = self.correlation_id {
                event = event.with_correlation(cid);
            }
            self.event_store.append(&[event.clone()]);
            // L1: Projection 동기 갱신 — publish 이전에 수행하여 쿼리 일관성 확보
            self.projections.write().unwrap().apply_all(&event);
            // L2: broadcast fan-out — 구독자(Agent/SSE)는 자기 async 태스크에서 소비
            self.event_bus.publish(&event);
        }
    }

    /// Pipeline 실행 — 순차 에이전트 체인
    ///
    /// Pipeline의 단계들을 순차 실행하고, 축적된 side-effect를
    /// repository에 적용한 뒤, 이벤트를 발행합니다.
    pub fn execute_pipeline(
        &mut self,
        pipeline: Pipeline,
        cmd: &Command,
    ) -> Result<CommandResult, MindServiceError> {
        let ctx = self.build_context(cmd)?;
        let initial = PipelineState::new(ctx);

        let final_state = pipeline.execute(initial)?;

        // Side-effects 적용
        if let Some((npc_id, state)) = &final_state.new_emotion_state {
            self.repository.save_emotion_state(npc_id, state.clone());
        }
        if let Some((owner, target, rel)) = &final_state.new_relationship {
            self.repository.save_relationship(owner, target, rel.clone());
        }
        if let Some(npc_id) = &final_state.clear_emotion {
            self.repository.clear_emotion_state(npc_id);
        }
        if final_state.clear_scene {
            self.repository.clear_scene();
        }
        if let Some(scene) = &final_state.save_scene {
            self.repository.save_scene(scene.clone());
        }

        // 이벤트 발행
        let aggregate_id = cmd.npc_id().to_string();
        self.emit_events(&aggregate_id, &final_state.accumulated_events);

        final_state.final_result.ok_or_else(|| {
            MindServiceError::InvalidSituation("파이프라인이 결과를 생성하지 못했습니다.".into())
        })
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
        let domain_focuses: Vec<_> = focuses
            .iter()
            .map(|f| {
                self.situation_service
                    .to_scene_focus(&self.repository, f, npc_id, partner_id)
            })
            .collect::<Result<Vec<_>, _>>()?;

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
