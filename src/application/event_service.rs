//! EventAwareMindService — Strangler Fig 래퍼
//!
//! 기존 `MindService`를 감싸서 모든 상태 변경을 `DomainEvent`로 기록합니다.
//! API 시그니처는 동일하며, 결과도 동일합니다. 이벤트는 부수 효과로 발생합니다.

use crate::domain::event::{DomainEvent, EventPayload};
use crate::ports::{Appraiser, MindRepository, StimulusProcessor};

use super::dto::*;
use super::event_bus::EventBus;
use super::event_store::{EventStore, InMemoryEventStore};
use super::mind_service::{MindService, MindServiceError};

use std::sync::Arc;

/// 이벤트 발행을 추가한 MindService 래퍼 (Strangler Fig Pattern)
///
/// 모든 공개 메서드는 내부 `MindService`에 위임 후,
/// 성공 시 해당 동작의 `DomainEvent`를 생성하여 저장·발행합니다.
pub struct EventAwareMindService<
    R: MindRepository,
    A: Appraiser,
    S: StimulusProcessor,
> {
    inner: MindService<R, A, S>,
    event_store: Arc<dyn EventStore>,
    event_bus: Arc<EventBus>,
    correlation_id: Option<u64>,
}

impl<R: MindRepository, A: Appraiser, S: StimulusProcessor>
    EventAwareMindService<R, A, S>
{
    /// 명시적 생성자
    pub fn new(
        inner: MindService<R, A, S>,
        event_store: Arc<dyn EventStore>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            inner,
            event_store,
            event_bus,
            correlation_id: None,
        }
    }

    /// correlation_id 설정 — 같은 요청에서 발생한 이벤트 묶음 추적
    pub fn set_correlation_id(&mut self, id: u64) {
        self.correlation_id = Some(id);
    }

    /// EventStore 접근자 (테스트/디버그용)
    pub fn event_store(&self) -> &Arc<dyn EventStore> {
        &self.event_store
    }

    /// EventBus 접근자
    pub fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    /// 내부 MindService 접근자
    pub fn inner(&self) -> &MindService<R, A, S> {
        &self.inner
    }

    /// 내부 MindService 가변 접근자
    pub fn inner_mut(&mut self) -> &mut MindService<R, A, S> {
        &mut self.inner
    }

    /// 저장소 접근자 (패스스루)
    pub fn repository(&self) -> &R {
        self.inner.repository()
    }

    /// 저장소 가변 접근자 (패스스루)
    pub fn repository_mut(&mut self) -> &mut R {
        self.inner.repository_mut()
    }

    // -----------------------------------------------------------------------
    // 이벤트 헬퍼
    // -----------------------------------------------------------------------

    fn emit(&self, aggregate_id: &str, payload: EventPayload) {
        let id = self.event_store.next_id();
        let seq = self.event_store.next_sequence(aggregate_id);
        let mut event = DomainEvent::new(id, aggregate_id.to_string(), seq, payload);
        if let Some(cid) = self.correlation_id {
            event = event.with_correlation(cid);
        }
        self.event_store.append(&[event.clone()]);
        self.event_bus.publish(&event);
    }

    // -----------------------------------------------------------------------
    // 래핑된 공개 메서드
    // -----------------------------------------------------------------------

    /// `MindService::appraise` + `EmotionAppraised` 이벤트
    pub fn appraise(
        &mut self,
        req: AppraiseRequest,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<AppraiseResult, MindServiceError> {
        let npc_id = req.npc_id.clone();
        let partner_id = req.partner_id.clone();
        let situation_desc = req.situation.as_ref().map(|s| s.description.clone());

        let result = self.inner.appraise(req, before_eval, after_eval)?;

        let snapshot: Vec<(String, f32)> = result
            .emotions
            .iter()
            .map(|e| (e.emotion_type.clone(), e.intensity))
            .collect();

        self.emit(
            &npc_id,
            EventPayload::EmotionAppraised {
                npc_id: npc_id.clone(),
                partner_id,
                situation_description: situation_desc,
                dominant: result
                    .dominant
                    .as_ref()
                    .map(|d| (d.emotion_type.clone(), d.intensity)),
                mood: result.mood,
                emotion_snapshot: snapshot,
            },
        );

        Ok(result)
    }

    /// `MindService::apply_stimulus` + `StimulusApplied` (+ `BeatTransitioned`) 이벤트
    pub fn apply_stimulus(
        &mut self,
        req: StimulusRequest,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<StimulusResult, MindServiceError> {
        let npc_id = req.npc_id.clone();
        let partner_id = req.partner_id.clone();
        let pad = (req.pleasure, req.arousal, req.dominance);

        // 사전 mood 읽기
        let mood_before = self
            .inner
            .repository()
            .get_emotion_state(&npc_id)
            .map(|s| s.overall_valence())
            .unwrap_or(0.0);

        // Scene의 현재 active_focus_id 기록 (Beat 전환 전)
        let before_focus_id = self
            .inner
            .repository()
            .get_scene()
            .and_then(|s| s.active_focus_id().map(|id| id.to_string()));

        let result = self.inner.apply_stimulus(req, before_eval, after_eval)?;

        let snapshot: Vec<(String, f32)> = result
            .emotions
            .iter()
            .map(|e| (e.emotion_type.clone(), e.intensity))
            .collect();

        self.emit(
            &npc_id,
            EventPayload::StimulusApplied {
                npc_id: npc_id.clone(),
                partner_id: partner_id.clone(),
                pad,
                mood_before,
                mood_after: result.mood,
                beat_changed: result.beat_changed,
                emotion_snapshot: snapshot,
            },
        );

        if result.beat_changed {
            if let Some(ref to_focus_id) = result.active_focus_id {
                self.emit(
                    &npc_id,
                    EventPayload::BeatTransitioned {
                        npc_id: npc_id.clone(),
                        from_focus_id: before_focus_id,
                        to_focus_id: to_focus_id.clone(),
                    },
                );
            }
        }

        Ok(result)
    }

    /// `MindService::start_scene` + `SceneStarted` (+ `EmotionAppraised`) 이벤트
    pub fn start_scene(
        &mut self,
        req: SceneRequest,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<SceneResult, MindServiceError> {
        let npc_id = req.npc_id.clone();
        let partner_id = req.partner_id.clone();

        let result = self.inner.start_scene(req, before_eval, after_eval)?;

        self.emit(
            &npc_id,
            EventPayload::SceneStarted {
                npc_id: npc_id.clone(),
                partner_id: partner_id.clone(),
                focus_count: result.focus_count,
                initial_focus_id: result.active_focus_id.clone(),
            },
        );

        if let Some(ref appraise_result) = result.initial_appraise {
            let snapshot: Vec<(String, f32)> = appraise_result
                .emotions
                .iter()
                .map(|e| (e.emotion_type.clone(), e.intensity))
                .collect();

            self.emit(
                &npc_id,
                EventPayload::EmotionAppraised {
                    npc_id: npc_id.clone(),
                    partner_id,
                    situation_description: None,
                    dominant: appraise_result
                        .dominant
                        .as_ref()
                        .map(|d| (d.emotion_type.clone(), d.intensity)),
                    mood: appraise_result.mood,
                    emotion_snapshot: snapshot,
                },
            );
        }

        Ok(result)
    }

    /// `MindService::after_dialogue` + 3개 이벤트
    pub fn after_dialogue(
        &mut self,
        req: AfterDialogueRequest,
    ) -> Result<AfterDialogueResponse, MindServiceError> {
        let npc_id = req.npc_id.clone();
        let partner_id = req.partner_id.clone();

        let result = self.inner.after_dialogue(req)?;

        self.emit(
            &npc_id,
            EventPayload::RelationshipUpdated {
                owner_id: npc_id.clone(),
                target_id: partner_id.clone(),
                before_closeness: result.before.closeness,
                before_trust: result.before.trust,
                before_power: result.before.power,
                after_closeness: result.after.closeness,
                after_trust: result.after.trust,
                after_power: result.after.power,
            },
        );

        self.emit(&npc_id, EventPayload::EmotionCleared { npc_id: npc_id.clone() });

        self.emit(
            &npc_id,
            EventPayload::SceneEnded {
                npc_id: npc_id.clone(),
                partner_id,
            },
        );

        Ok(result)
    }

    /// `MindService::after_beat` + `RelationshipUpdated` 이벤트
    pub fn after_beat(
        &mut self,
        req: AfterDialogueRequest,
    ) -> Result<AfterDialogueResponse, MindServiceError> {
        let npc_id = req.npc_id.clone();
        let partner_id = req.partner_id.clone();

        let result = self.inner.after_beat(req)?;

        self.emit(
            &npc_id,
            EventPayload::RelationshipUpdated {
                owner_id: npc_id.clone(),
                target_id: partner_id,
                before_closeness: result.before.closeness,
                before_trust: result.before.trust,
                before_power: result.before.power,
                after_closeness: result.after.closeness,
                after_trust: result.after.trust,
                after_power: result.after.power,
            },
        );

        Ok(result)
    }

    /// `MindService::generate_guide` + `GuideGenerated` 이벤트
    pub fn generate_guide(&self, req: GuideRequest) -> Result<GuideResult, MindServiceError> {
        let npc_id = req.npc_id.clone();
        let partner_id = req.partner_id.clone();

        let result = self.inner.generate_guide(req)?;

        self.emit(
            &npc_id,
            EventPayload::GuideGenerated {
                npc_id: npc_id.clone(),
                partner_id,
            },
        );

        Ok(result)
    }

    /// `MindService::scene_info` — 읽기 전용, 이벤트 없음
    pub fn scene_info(&self) -> SceneInfoResult {
        self.inner.scene_info()
    }

    /// `MindService::reset_scene_to_initial_focus` — 패스스루
    pub fn reset_scene_to_initial_focus(&mut self) -> Option<String> {
        self.inner.reset_scene_to_initial_focus()
    }

    /// `MindService::load_scene_focuses` + 조건부 `SceneStarted` 이벤트
    pub fn load_scene_focuses(
        &mut self,
        focuses: Vec<crate::domain::emotion::SceneFocus>,
        npc_id: String,
        partner_id: String,
        significance: f32,
    ) -> Result<Option<AppraiseResult>, MindServiceError> {
        let result = self.inner.load_scene_focuses(
            focuses.clone(),
            npc_id.clone(),
            partner_id.clone(),
            significance,
        )?;

        if result.is_some() {
            self.emit(
                &npc_id,
                EventPayload::SceneStarted {
                    npc_id: npc_id.clone(),
                    partner_id,
                    focus_count: focuses.len(),
                    initial_focus_id: self
                        .inner
                        .repository()
                        .get_scene()
                        .and_then(|s| s.active_focus_id().map(|id| id.to_string())),
                },
            );
        }

        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// 편의 생성자 (기본 엔진)
// ---------------------------------------------------------------------------

impl<R: MindRepository> EventAwareMindService<R, crate::domain::emotion::AppraisalEngine, crate::domain::emotion::StimulusEngine> {
    /// 기본 엔진 + 기본 InMemoryEventStore로 생성
    pub fn with_default_events(repository: R) -> Self {
        let inner = MindService::new(repository);
        let event_store: Arc<dyn EventStore> = Arc::new(InMemoryEventStore::new());
        let event_bus = Arc::new(EventBus::new());
        Self::new(inner, event_store, event_bus)
    }
}
