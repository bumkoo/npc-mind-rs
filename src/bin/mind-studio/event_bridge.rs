//! Phase 1.6 — EventBus → SSE Bridge.
//!
//! `CommandDispatcher.dispatch_v2`가 발행하는 도메인 이벤트를 구독해서
//! Mind Studio UI용 `StateEvent` SSE로 자동 변환한다. 기존에는 각 핸들러/서비스가
//! manual로 `state.emit(StateEvent::XXX)`을 호출했지만, Phase 1.6 이후 도메인 사실에서
//! 도출 가능한 SSE는 *전부 본 bridge가 일괄 발행*한다.
//!
//! ## 책임 분리 (Phase 1.6)
//!
//! - **본 bridge (도메인 사실에서 도출)**: `Appraised`/`StimulusApplied`/`GuideGenerated`/
//!   `SceneStarted`/`AfterDialogue`/`ChatTurnCompleted`/`DialogueReflected`/
//!   `MemoryCreated|Superseded|Consolidated`/`RumorSeeded|Spread`
//! - **호출자 manual emit으로 유지 (UI 라이프사이클)**: `HistoryChanged`/
//!   `SituationChanged`/`TestReportChanged`/`ScenarioLoaded`/`ScenarioSaved`/
//!   `ResultLoaded`/`NpcChanged`/`RelationshipChanged`/`ObjectChanged`/
//!   `SceneInfoChanged`/`ChatStarted`/`ChatEnded`
//!
//! ## Lagged replay (MemoryProjector 패턴 mirror)
//!
//! `tokio::broadcast`는 capacity 초과 시 오래된 이벤트를 *덮어쓴다*. Bridge는
//! `subscribe_with_lag`로 Lagged 통지를 받고 `EventStore::get_events_after_id`로
//! 누락분을 replay한다. SSE 자체는 best-effort 알림이라 치명적이지 않지만,
//! Director 경로 등 burst가 발생할 때 *완전히 잃지는 않게* 보장한다.
//!
//! ## 보너스 — Director 경로 SSE bug fix
//!
//! 기존에는 `/api/v2/scenes/*`(Director 경로)가 dispatch_v2를 직접 호출해
//! 도메인 이벤트는 발행하지만 Mind Studio의 manual `state.emit()`을 거치지 않아
//! *SSE가 전혀 안 날아갔다*. Bridge가 shared_dispatcher의 EventBus를 구독하므로
//! Director 경로의 dispatch도 본 bridge를 통해 자동 SSE 발행.
//!
//! (단 현재 `AppState.director_v2`는 *별도 dispatcher/EventStore*를 보유 — Director
//! 자체 이벤트는 본 bridge가 보지 못함. shared_dispatcher 통합은 후속 작업.)

use std::sync::Arc;

use futures::{Stream, StreamExt};

use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::EventStore;
use npc_mind::domain::event::{DomainEvent, EventPayload};

use crate::events::StateEvent;

/// EventBus를 구독해 도메인 이벤트를 `StateEvent` SSE로 변환·재방출하는 배경 작업.
pub struct EventBridge {
    tx: tokio::sync::broadcast::Sender<StateEvent>,
}

impl EventBridge {
    pub fn new(tx: tokio::sync::broadcast::Sender<StateEvent>) -> Self {
        Self { tx }
    }

    /// 배경 task로 spawn. `tokio::spawn(bridge.run(bus, event_store))`로 호출.
    ///
    /// `bus.subscribe_with_lag()`는 future가 polled되는 시점에 구독을 시작 —
    /// spawn 이후 발행되는 이벤트는 모두 수신된다. spawn 이전 이벤트는 누락하지만
    /// Mind Studio는 부팅 직후 dispatch가 거의 없으므로 무시 가능.
    ///
    /// `bus`는 `Arc<EventBus>`로 받아 ownership 이전 — `event_bus()` 반환값을
    /// 그대로 `.clone()`해서 넘기면 된다 (Arc clone은 저렴).
    pub fn run(
        self: Arc<Self>,
        bus: Arc<EventBus>,
        event_store: Arc<dyn EventStore>,
    ) -> impl std::future::Future<Output = ()> + Send + 'static {
        let bridge = self;
        async move {
            let stream = Box::pin(bus.subscribe_with_lag());
            bridge.consume_stream(stream, event_store).await;
        }
    }

    /// Stream 소비 루프 — `run`의 핵심 로직을 분리해 결정론 테스트 가능.
    ///
    /// MemoryProjector::consume_stream과 *동일 패턴* (이미 검증된 at-least-once
    /// 보장). Lagged 시 EventStore에서 last_processed_id 기반 replay,
    /// `*Requested` 결과 이벤트는 replay 시 emit 금지 (audit-only).
    pub async fn consume_stream<S>(
        self: Arc<Self>,
        mut stream: std::pin::Pin<Box<S>>,
        event_store: Arc<dyn EventStore>,
    ) where
        S: Stream<Item = Result<Arc<DomainEvent>, u64>> + Send + ?Sized,
    {
        let mut last_processed_id: u64 = 0;
        while let Some(item) = stream.next().await {
            match item {
                Ok(event) => {
                    let id = event.id;
                    if id <= last_processed_id {
                        continue;
                    }
                    self.emit_for_event(&event);
                    last_processed_id = id;
                }
                Err(skipped) => {
                    tracing::warn!(
                        skipped,
                        last_processed_id,
                        "EventBridge: broadcast lag detected, replaying from event store"
                    );
                    let missed = event_store.get_events_after_id(last_processed_id);
                    for ev in missed {
                        let id = ev.id;
                        if ev.is_command_intent() {
                            last_processed_id = last_processed_id.max(id);
                            continue;
                        }
                        self.emit_for_event(&ev);
                        last_processed_id = last_processed_id.max(id);
                    }
                }
            }
        }
    }

    /// DomainEvent → StateEvent 매핑 + 송신. 매핑되지 않는 이벤트는 무시.
    ///
    /// 매핑 규칙은 `map_event`에 분리 — 단위 테스트가 결정론으로 검증.
    fn emit_for_event(&self, event: &DomainEvent) {
        for state_event in map_event(event) {
            // 수신자 0이면 `send`가 Err을 반환 — 정상이므로 무시.
            let _ = self.tx.send(state_event);
        }
    }
}

/// 도메인 이벤트 1개를 0~N개의 `StateEvent`로 매핑.
///
/// 대부분 1:1 또는 1:0. `BeatTransitioned`는 0개 (UI는 stimulus 응답의 beat_changed
/// 플래그로 충분). `DialogueTurnCompleted`는 화자가 "assistant"일 때만 emit (1 chat
/// turn = user/assistant 2개 이벤트 발행되지만 UI는 assistant 시점에만 갱신).
pub fn map_event(event: &DomainEvent) -> Vec<StateEvent> {
    match &event.payload {
        // ─── 도메인 사실 → SSE 자동 발행 ───
        EventPayload::EmotionAppraised { .. } => vec![StateEvent::Appraised],
        EventPayload::StimulusApplied(_) => vec![StateEvent::StimulusApplied],
        EventPayload::GuideGenerated { .. } => vec![StateEvent::GuideGenerated],
        EventPayload::SceneStarted { .. } => vec![StateEvent::SceneStarted],
        // SceneEnded == "dialogue 종료 + 관계 정산 완료". Mind Studio의 AfterDialogue
        // 의미와 정확히 일치. RelationshipUpdated를 트리거로 쓰면 chitchat skip 케이스
        // (Phase 1 게이트)에 누락되므로 SceneEnded가 적절.
        EventPayload::SceneEnded { .. } => vec![StateEvent::AfterDialogue],
        // chat 1 turn = user + assistant 2 이벤트 발행. UI 갱신은 assistant 시점 1회면 충분.
        EventPayload::DialogueTurnCompleted { speaker, .. } if speaker == "assistant" => {
            vec![StateEvent::ChatTurnCompleted]
        }
        EventPayload::DialogueReflected { .. } => vec![StateEvent::DialogueReflected],
        EventPayload::MemoryEntryCreated(_) => vec![StateEvent::MemoryCreated],
        EventPayload::MemoryEntrySuperseded { .. } => vec![StateEvent::MemorySuperseded],
        EventPayload::MemoryEntryConsolidated { .. } => vec![StateEvent::MemoryConsolidated],
        EventPayload::RumorSeeded { .. } => vec![StateEvent::RumorSeeded],
        EventPayload::RumorSpread { .. } => vec![StateEvent::RumorSpread],
        // ─── 나머지: SSE 매핑 없음 (UI 영향 0 또는 manual emit로 처리) ───
        _ => vec![],
    }
}

// ===========================================================================
// 단위 테스트 — 매핑 결정론 + Stream replay 검증
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use npc_mind::application::event_store::InMemoryEventStore;
    use npc_mind::domain::event::{DomainEvent, EventPayload};
    use npc_mind::domain::reflection::ReflectionResult;
    use npc_mind::domain::scene_id::SceneId;

    fn mk_event(payload: EventPayload, id: u64) -> Arc<DomainEvent> {
        Arc::new(DomainEvent::new(id, "test".into(), id, payload))
    }

    fn appraised(id: u64) -> Arc<DomainEvent> {
        mk_event(
            EventPayload::EmotionAppraised {
                npc_id: "n".into(),
                partner_id: "p".into(),
                situation_description: None,
                dominant: None,
                mood: 0.0,
                emotion_snapshot: vec![],
            },
            id,
        )
    }

    fn scene_started(id: u64) -> Arc<DomainEvent> {
        mk_event(
            EventPayload::SceneStarted {
                npc_id: "n".into(),
                partner_id: "p".into(),
                focus_count: 1,
                initial_focus_id: None,
            },
            id,
        )
    }

    fn dialogue_turn(speaker: &str, id: u64) -> Arc<DomainEvent> {
        mk_event(
            EventPayload::DialogueTurnCompleted {
                npc_id: "n".into(),
                partner_id: "p".into(),
                speaker: speaker.into(),
                utterance: "u".into(),
                emotion_snapshot: vec![],
            },
            id,
        )
    }

    #[test]
    fn maps_appraised() {
        assert_eq!(map_event(&appraised(1)), vec![StateEvent::Appraised]);
    }

    #[test]
    fn maps_scene_ended_to_after_dialogue() {
        let ev = mk_event(
            EventPayload::SceneEnded {
                npc_id: "n".into(),
                partner_id: "p".into(),
            },
            1,
        );
        assert_eq!(map_event(&ev), vec![StateEvent::AfterDialogue]);
    }

    #[test]
    fn dialogue_turn_assistant_emits_chat_turn_completed() {
        assert_eq!(
            map_event(&dialogue_turn("assistant", 1)),
            vec![StateEvent::ChatTurnCompleted]
        );
    }

    #[test]
    fn dialogue_turn_user_does_not_emit() {
        assert_eq!(map_event(&dialogue_turn("user", 1)), Vec::<StateEvent>::new());
    }

    #[test]
    fn dialogue_reflected_emits() {
        let ev = mk_event(
            EventPayload::DialogueReflected {
                npc_id: "n".into(),
                partner_id: "p".into(),
                scene_id: SceneId::new("n", "p"),
                result: ReflectionResult {
                    is_chitchat: false,
                    summary: "s".into(),
                    significance_score: 0.5,
                    declarative_events: vec![],
                    partnership_event: None,
                    turn_count: 1,
                    llm_reasoning: None,
                },
            },
            1,
        );
        assert_eq!(map_event(&ev), vec![StateEvent::DialogueReflected]);
    }

    /// dispatch가 발행한 이벤트 시퀀스가 *순서대로* SSE로 emit되는지.
    #[tokio::test]
    async fn consume_stream_emits_state_events_in_order() {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<StateEvent>(32);
        let bridge = Arc::new(EventBridge::new(tx));

        // SceneStarted → EmotionAppraised → DialogueTurnCompleted(user) → DialogueTurnCompleted(assistant)
        let events: Vec<Result<Arc<DomainEvent>, u64>> = vec![
            Ok(scene_started(1)),
            Ok(appraised(2)),
            Ok(dialogue_turn("user", 3)),
            Ok(dialogue_turn("assistant", 4)),
        ];

        let stream = Box::pin(stream::iter(events));
        let store: Arc<dyn EventStore> = Arc::new(InMemoryEventStore::new());
        bridge.consume_stream(stream, store).await;

        let mut received = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            received.push(ev);
        }
        assert_eq!(
            received,
            vec![
                StateEvent::SceneStarted,
                StateEvent::Appraised,
                StateEvent::ChatTurnCompleted,
            ]
        );
    }

    /// Lagged 통지 시 EventStore.replay를 수행하고 누락 이벤트의 매핑을 emit.
    #[tokio::test]
    async fn lagged_triggers_replay_from_event_store() {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<StateEvent>(32);
        let bridge = Arc::new(EventBridge::new(tx));

        let store = Arc::new(InMemoryEventStore::new());
        let replay_ev = DomainEvent::new(
            5,
            "test".into(),
            5,
            EventPayload::SceneStarted {
                npc_id: "n".into(),
                partner_id: "p".into(),
                focus_count: 0,
                initial_focus_id: None,
            },
        );
        store.append(&[replay_ev]);

        // Stream: 정상 이벤트 1개 (id=1) → Lagged → 끝
        let events: Vec<Result<Arc<DomainEvent>, u64>> = vec![Ok(appraised(1)), Err(3)];

        let stream = Box::pin(stream::iter(events));
        bridge
            .consume_stream(stream, store.clone() as Arc<dyn EventStore>)
            .await;

        let mut received = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            received.push(ev);
        }
        // id=1 (Appraised) + replay id=5 (SceneStarted)
        assert_eq!(
            received,
            vec![StateEvent::Appraised, StateEvent::SceneStarted]
        );
    }
}
