//! Projection을 `EventHandler`로 노출하는 Inline-mode 어댑터 (B안 B2)
//!
//! B5.1 (v0.2.0): v1 `Projection` trait 자체는 deprecated 되었지만, 본 wrapper들은
//! `Projection::apply`를 **내부적으로 재사용**한다. v1 API가 완전 제거되는 v0.3.0까지
//! `#[allow(deprecated)]`로 warning을 억제한다. v0.3.0 cutover 시 각 Projection 구조체가
//! 자체 `&self` apply를 제공하는 refactor가 예정됨.

#![allow(deprecated)]

//!
//! 기존 `src/application/projection.rs`의 `EmotionProjection` / `RelationshipProjection` /
//! `SceneProjection` 세 가지를 `Arc<Mutex<T>>`로 감싸 `EventHandler`를 구현한다.
//!
//! ## 설계
//!
//! - **Wrapper 방식**: 기존 Projection struct·trait·ProjectionRegistry는 **무변경**.
//!   v1 경로(`CommandDispatcher.dispatch → emit_events → projections.apply_all`)는 그대로 동작.
//! - **B3 dispatch_v2 소비**: 이 wrapper들은 B3에서 새 Dispatcher의 Inline 단계에서
//!   `Arc<dyn EventHandler>`로 수집되어 iteration된다.
//! - **테스트 관찰성**: 각 wrapper는 `projection()` getter로 내부 `Arc<Mutex<T>>`를 노출하므로
//!   테스트가 상태를 직접 검증할 수 있다.
//!
//! ## B2 Scope Note
//!
//! `ProjectionRegistry` 자체의 storage를 `Vec<Arc<dyn EventHandler>>`로 전환하는 작업
//! (B-Plan §8 Stage B2 항목 2)은 B3에서 dispatch_v2가 도입되며 ProjectionRegistry가
//! 새 타입으로 대체될 때 함께 수행한다. B2는 wrapper 핸들러 제공으로 한정하여
//! "쿼리 일관성 관련 기존 테스트 전수 통과" 조건을 엄격 보장한다.

use std::sync::{Arc, Mutex};

use super::handler_v2::{
    DeliveryMode, EventHandler, EventHandlerContext, HandlerError, HandlerInterest, HandlerResult,
};
use super::priority;
use crate::application::projection::{
    EmotionProjection, RelationshipProjection, SceneProjection,
};
use crate::domain::event::{DomainEvent, EventKind};

// ---------------------------------------------------------------------------
// EmotionProjectionHandler
// ---------------------------------------------------------------------------

/// `EmotionProjection`을 `EventHandler` (Inline mode)로 노출
///
/// `EmotionAppraised` / `StimulusApplied` / `EmotionCleared` 이벤트를 소비하여
/// 내부 `EmotionProjection.apply()`를 호출한다.
pub struct EmotionProjectionHandler {
    inner: Arc<Mutex<EmotionProjection>>,
}

impl EmotionProjectionHandler {
    /// 빈 Projection으로 시작
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(EmotionProjection::new())),
        }
    }

    /// 외부에서 준비한 공유 Projection 핸들을 주입
    pub fn from_shared(inner: Arc<Mutex<EmotionProjection>>) -> Self {
        Self { inner }
    }

    /// 내부 Projection의 공유 핸들 복제
    ///
    /// 호출자가 직접 `.lock()`으로 `&mut` guard를 획득하므로 **lock 관리는 호출자 책임**이다.
    /// 테스트·쿼리 경로에서 일반적으로 `.lock().unwrap().get_*()` 패턴으로 read-only 쿼리에만
    /// 사용되지만, 타입 시스템은 외부 변이를 막지 못한다. 같은 핸들을 여러 wrapper에 주입하면
    /// 같은 이벤트가 중복 apply될 수 있으므로 **한 projection 상태당 wrapper 1개** 원칙 유지.
    pub fn projection(&self) -> Arc<Mutex<EmotionProjection>> {
        self.inner.clone()
    }
}

impl Default for EmotionProjectionHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for EmotionProjectionHandler {
    fn name(&self) -> &'static str {
        "EmotionProjectionHandler"
    }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![
            EventKind::EmotionAppraised,
            EventKind::StimulusApplied,
            EventKind::EmotionCleared,
        ])
    }

    fn mode(&self) -> DeliveryMode {
        DeliveryMode::Inline {
            priority: priority::inline::EMOTION_PROJECTION,
        }
    }

    fn handle(
        &self,
        event: &DomainEvent,
        _ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        // Mutex poison은 다른 스레드에서의 panic을 의미. Inline 모드 계약(에러는 로그만,
        // 커맨드는 계속)을 존중해 Precondition으로 에스컬레이트하고 Dispatcher가 처리 위임.
        let mut proj = self
            .inner
            .lock()
            .map_err(|_| HandlerError::Precondition("emotion projection mutex poisoned"))?;
        proj.apply(event);
        Ok(HandlerResult::default())
    }
}

// ---------------------------------------------------------------------------
// RelationshipProjectionHandler
// ---------------------------------------------------------------------------

/// `RelationshipProjection`을 `EventHandler` (Inline mode)로 노출
pub struct RelationshipProjectionHandler {
    inner: Arc<Mutex<RelationshipProjection>>,
}

impl RelationshipProjectionHandler {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RelationshipProjection::new())),
        }
    }

    pub fn from_shared(inner: Arc<Mutex<RelationshipProjection>>) -> Self {
        Self { inner }
    }

    /// 내부 Projection의 공유 핸들 복제 — 계약은 `EmotionProjectionHandler::projection`과 동일.
    pub fn projection(&self) -> Arc<Mutex<RelationshipProjection>> {
        self.inner.clone()
    }
}

impl Default for RelationshipProjectionHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for RelationshipProjectionHandler {
    fn name(&self) -> &'static str {
        "RelationshipProjectionHandler"
    }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![EventKind::RelationshipUpdated])
    }

    fn mode(&self) -> DeliveryMode {
        DeliveryMode::Inline {
            priority: priority::inline::RELATIONSHIP_PROJECTION,
        }
    }

    fn handle(
        &self,
        event: &DomainEvent,
        _ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        // EmotionProjectionHandler와 동일: poison은 Precondition으로 에스컬레이트.
        let mut proj = self
            .inner
            .lock()
            .map_err(|_| HandlerError::Precondition("relationship projection mutex poisoned"))?;
        proj.apply(event);
        Ok(HandlerResult::default())
    }
}

// ---------------------------------------------------------------------------
// SceneProjectionHandler
// ---------------------------------------------------------------------------

/// `SceneProjection`을 `EventHandler` (Inline mode)로 노출
pub struct SceneProjectionHandler {
    inner: Arc<Mutex<SceneProjection>>,
}

impl SceneProjectionHandler {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SceneProjection::new())),
        }
    }

    pub fn from_shared(inner: Arc<Mutex<SceneProjection>>) -> Self {
        Self { inner }
    }

    /// 내부 Projection의 공유 핸들 복제 — 계약은 `EmotionProjectionHandler::projection`과 동일.
    pub fn projection(&self) -> Arc<Mutex<SceneProjection>> {
        self.inner.clone()
    }
}

impl Default for SceneProjectionHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for SceneProjectionHandler {
    fn name(&self) -> &'static str {
        "SceneProjectionHandler"
    }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![
            EventKind::SceneStarted,
            EventKind::BeatTransitioned,
            EventKind::SceneEnded,
        ])
    }

    fn mode(&self) -> DeliveryMode {
        DeliveryMode::Inline {
            priority: priority::inline::SCENE_PROJECTION,
        }
    }

    fn handle(
        &self,
        event: &DomainEvent,
        _ctx: &mut EventHandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        // EmotionProjectionHandler와 동일: poison은 Precondition으로 에스컬레이트.
        let mut proj = self
            .inner
            .lock()
            .map_err(|_| HandlerError::Precondition("scene projection mutex poisoned"))?;
        proj.apply(event);
        Ok(HandlerResult::default())
    }
}

// ---------------------------------------------------------------------------
// B2 — L1 단위 테스트
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::command::handler_v2::test_support::HandlerTestHarness;
    use crate::domain::event::EventPayload;

    fn make_event(payload: EventPayload) -> DomainEvent {
        DomainEvent::new(0, "npc".into(), 0, payload)
    }

    // -----------------------------------------------------------------------
    // EmotionProjectionHandler
    // -----------------------------------------------------------------------

    #[test]
    fn emotion_appraised_event_updates_projection() {
        let handler = EmotionProjectionHandler::new();
        let mut harness = HandlerTestHarness::new();

        let event = make_event(EventPayload::EmotionAppraised {
            npc_id: "alice".into(),
            partner_id: "bob".into(),
            situation_description: None,
            dominant: Some(("Joy".into(), 0.8)),
            mood: 0.6,
            emotion_snapshot: vec![("Joy".into(), 0.8)],
        });

        harness.dispatch(&handler, event).expect("handler must succeed");

        let proj = handler.projection();
        let p = proj.lock().unwrap();
        assert_eq!(p.get_mood("alice"), Some(0.6));
        assert_eq!(p.get_dominant("alice"), Some(&("Joy".into(), 0.8)));
    }

    #[test]
    fn stimulus_applied_updates_mood_after() {
        let handler = EmotionProjectionHandler::new();
        let mut harness = HandlerTestHarness::new();

        let event = make_event(EventPayload::StimulusApplied {
            npc_id: "alice".into(),
            partner_id: "bob".into(),
            pad: (0.5, 0.2, 0.0),
            mood_before: 0.2,
            mood_after: 0.5,
            beat_changed: false,
            emotion_snapshot: vec![("Joy".into(), 0.5)],
        });

        harness.dispatch(&handler, event).expect("handler must succeed");

        let p = handler.projection();
        let p = p.lock().unwrap();
        assert_eq!(p.get_mood("alice"), Some(0.5));
    }

    #[test]
    fn emotion_cleared_removes_projection_entries() {
        let handler = EmotionProjectionHandler::new();
        let mut harness = HandlerTestHarness::new();

        // Seed with an appraise
        let seed = make_event(EventPayload::EmotionAppraised {
            npc_id: "alice".into(),
            partner_id: "bob".into(),
            situation_description: None,
            dominant: Some(("Joy".into(), 0.8)),
            mood: 0.6,
            emotion_snapshot: vec![("Joy".into(), 0.8)],
        });
        harness.dispatch(&handler, seed).unwrap();

        // Clear
        let clear = make_event(EventPayload::EmotionCleared {
            npc_id: "alice".into(),
        });
        harness.dispatch(&handler, clear).unwrap();

        let p = handler.projection();
        let p = p.lock().unwrap();
        assert!(p.get_mood("alice").is_none());
        assert!(p.get_dominant("alice").is_none());
    }

    // -----------------------------------------------------------------------
    // RelationshipProjectionHandler
    // -----------------------------------------------------------------------

    #[test]
    fn relationship_updated_stores_after_values() {
        let handler = RelationshipProjectionHandler::new();
        let mut harness = HandlerTestHarness::new();

        let event = make_event(EventPayload::RelationshipUpdated {
            owner_id: "alice".into(),
            target_id: "bob".into(),
            before_closeness: 0.1,
            before_trust: 0.2,
            before_power: 0.3,
            after_closeness: 0.4,
            after_trust: 0.5,
            after_power: 0.6,
        });

        harness.dispatch(&handler, event).unwrap();

        let p = handler.projection();
        let p = p.lock().unwrap();
        assert_eq!(p.get_values("alice", "bob"), Some((0.4, 0.5, 0.6)));
    }

    #[test]
    fn unrelated_event_ignored_by_relationship_projection() {
        let handler = RelationshipProjectionHandler::new();
        let mut harness = HandlerTestHarness::new();

        // GuideGenerated는 RelationshipProjection의 interest 밖.
        // 그러나 HandlerTestHarness는 interest 필터링을 하지 않고 무조건 handle을 호출하므로,
        // projection 내부 match에서 자연스럽게 걸러져 no-op이 된다.
        let event = make_event(EventPayload::GuideGenerated {
            npc_id: "alice".into(),
            partner_id: "bob".into(),
        });

        harness.dispatch(&handler, event).unwrap();

        let p = handler.projection();
        let p = p.lock().unwrap();
        assert!(p.get_values("alice", "bob").is_none());
    }

    #[test]
    fn repeated_relationship_updates_overwrite() {
        let handler = RelationshipProjectionHandler::new();
        let mut harness = HandlerTestHarness::new();

        let ev1 = make_event(EventPayload::RelationshipUpdated {
            owner_id: "a".into(),
            target_id: "b".into(),
            before_closeness: 0.0,
            before_trust: 0.0,
            before_power: 0.0,
            after_closeness: 0.1,
            after_trust: 0.1,
            after_power: 0.1,
        });
        let ev2 = make_event(EventPayload::RelationshipUpdated {
            owner_id: "a".into(),
            target_id: "b".into(),
            before_closeness: 0.1,
            before_trust: 0.1,
            before_power: 0.1,
            after_closeness: 0.7,
            after_trust: 0.8,
            after_power: 0.9,
        });

        harness.dispatch(&handler, ev1).unwrap();
        harness.dispatch(&handler, ev2).unwrap();

        let p = handler.projection();
        let p = p.lock().unwrap();
        assert_eq!(p.get_values("a", "b"), Some((0.7, 0.8, 0.9)));
    }

    // -----------------------------------------------------------------------
    // SceneProjectionHandler
    // -----------------------------------------------------------------------

    #[test]
    fn scene_started_sets_active_focus() {
        let handler = SceneProjectionHandler::new();
        let mut harness = HandlerTestHarness::new();

        let event = make_event(EventPayload::SceneStarted {
            npc_id: "alice".into(),
            partner_id: "bob".into(),
            focus_count: 2,
            initial_focus_id: Some("initial".into()),
        });

        harness.dispatch(&handler, event).unwrap();

        let p = handler.projection();
        let p = p.lock().unwrap();
        assert!(p.is_active());
        assert_eq!(p.active_focus_id(), Some("initial"));
    }

    #[test]
    fn beat_transitioned_updates_focus_id() {
        let handler = SceneProjectionHandler::new();
        let mut harness = HandlerTestHarness::new();

        let start = make_event(EventPayload::SceneStarted {
            npc_id: "alice".into(),
            partner_id: "bob".into(),
            focus_count: 2,
            initial_focus_id: Some("initial".into()),
        });
        let beat = make_event(EventPayload::BeatTransitioned {
            npc_id: "alice".into(),
            partner_id: "bob".into(),
            from_focus_id: Some("initial".into()),
            to_focus_id: "next".into(),
        });

        harness.dispatch(&handler, start).unwrap();
        harness.dispatch(&handler, beat).unwrap();

        let p = handler.projection();
        let p = p.lock().unwrap();
        assert_eq!(p.active_focus_id(), Some("next"));
    }

    #[test]
    fn scene_ended_clears_active() {
        let handler = SceneProjectionHandler::new();
        let mut harness = HandlerTestHarness::new();

        let start = make_event(EventPayload::SceneStarted {
            npc_id: "alice".into(),
            partner_id: "bob".into(),
            focus_count: 1,
            initial_focus_id: Some("initial".into()),
        });
        let end = make_event(EventPayload::SceneEnded {
            npc_id: "alice".into(),
            partner_id: "bob".into(),
        });

        harness.dispatch(&handler, start).unwrap();
        harness.dispatch(&handler, end).unwrap();

        let p = handler.projection();
        let p = p.lock().unwrap();
        assert!(!p.is_active());
    }
}
