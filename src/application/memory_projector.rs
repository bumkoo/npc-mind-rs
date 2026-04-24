//! MemoryProjector — EventBus 구독 기반 기억 인덱싱 에이전트
//!
//! 도메인 이벤트를 Stream으로 수신하여 NPC 기억으로 변환·인덱싱한다.
//! `embed` feature 필수 — TextEmbedder로 임베딩 생성.
//!
//! # 구독 모델
//!
//! `EventBus`가 `tokio::sync::broadcast` 기반이므로 소비자는 자기 async
//! 태스크에서 Stream을 폴링해야 한다. `run(bus, event_store)` 헬퍼가
//! 소비 Future를 돌려주고, subscribe는 `.await` 시점에 수행되므로 Future를
//! spawn한 뒤 발행된 이벤트는 모두 수신한다.
//!
//! # Lag 복구
//!
//! broadcast는 capacity를 초과하면 이벤트를 drop한다. `run`은
//! `subscribe_with_lag`로 Lagged 통지를 받고, `EventStore::get_events_after_id`
//! 로 놓친 이벤트를 replay하여 at-least-once 보장을 유지한다.
//! `last_processed_id`는 항상 단조 증가하므로, broadcast 잔여 이벤트가
//! replay 이후 뒤늦게 수신되더라도 커서가 역행하지 않는다.

use crate::application::event_bus::EventBus;
use crate::application::event_store::EventStore;
use crate::domain::event::{DomainEvent, EventPayload};
use crate::domain::memory::{MemoryEntry, MemoryType};
use crate::ports::{MemoryStore, TextEmbedder};

use futures::{Stream, StreamExt};
use std::future::Future;
use std::sync::{Arc, Mutex};

/// 관계 변화 유의미 판단 임계값
const RELATIONSHIP_CHANGE_THRESHOLD: f32 = 0.05;

/// 기억 인덱싱 에이전트
///
/// EventBus Stream을 구독하여 관련 이벤트 발생 시 자동으로 기억을
/// 생성·인덱싱한다. CommandHandler가 아닌 EventBus subscriber.
pub struct MemoryProjector {
    memory_store: Arc<dyn MemoryStore>,
    embedder: Arc<Mutex<dyn TextEmbedder + Send>>,
    id_counter: Mutex<u64>,
}

impl MemoryProjector {
    pub fn new(
        memory_store: Arc<dyn MemoryStore>,
        embedder: Arc<Mutex<dyn TextEmbedder + Send>>,
    ) -> Self {
        Self {
            memory_store,
            embedder,
            id_counter: Mutex::new(0),
        }
    }

    /// EventBus를 구독하고 소비 Future를 반환
    ///
    /// 호출자가 자기 async 런타임(tokio::spawn / bevy_tasks 등)에서
    /// 반환된 Future를 spawn해야 실제 소비가 시작된다. `subscribe`는
    /// Future가 처음 polled될 때 수행되므로 spawn 이후 발행된 이벤트는
    /// 모두 수신된다.
    ///
    /// `EventBus`는 `Clone`이 저렴(`Arc<Sender>` 공유)하므로 `.clone()`으로
    /// 넘기면 된다.
    ///
    /// ```rust,ignore
    /// let agent = Arc::new(MemoryProjector::new(store, embedder));
    /// tokio::spawn(agent.run(bus.clone(), event_store));
    /// ```
    ///
    /// `event_store`는 broadcast lag 발생 시 놓친 이벤트를 replay하여
    /// at-least-once 보장을 유지하기 위해 사용된다.
    pub fn run(
        self: Arc<Self>,
        bus: EventBus,
        event_store: Arc<dyn EventStore>,
    ) -> impl Future<Output = ()> + Send + 'static {
        let agent = self;
        async move {
            // subscribe는 Future가 polled된 시점에 수행 — spawn 이후 publish
            // 된 이벤트는 모두 수신된다.
            let stream = Box::pin(bus.subscribe_with_lag());
            agent.consume_stream(stream, event_store).await;
        }
    }

    /// Stream 소비 루프 — `run`의 핵심 로직을 분리하여 테스트 가능하게 한 것
    ///
    /// 이 함수는 `Stream<Item = Result<Arc<DomainEvent>, u64>>`을 그대로 받으므로
    /// 테스트에서 `futures::stream::iter`로 확정적 시퀀스를 주입해 broadcast·
    /// 타이밍 의존성 없이 at-least-once 보장과 `last_processed_id` 단조성을
    /// 검증할 수 있다.
    pub async fn consume_stream<S>(
        self: Arc<Self>,
        mut stream: std::pin::Pin<Box<S>>,
        event_store: Arc<dyn EventStore>,
    ) where
        S: Stream<Item = Result<Arc<DomainEvent>, u64>> + Send + ?Sized,
    {
        // 이미 처리한 이벤트의 최대 id. broadcast 잔여 이벤트가 replay
        // 이후에 뒤늦게 수신돼도 커서가 역행하지 않도록 max로 갱신한다.
        let mut last_processed_id: u64 = 0;
        while let Some(item) = stream.next().await {
            match item {
                Ok(event) => {
                    let id = event.id;
                    if id <= last_processed_id {
                        // 이미 replay로 처리된 이벤트가 뒤늦게 도착 — 중복 방지
                        continue;
                    }
                    self.on_event(&event);
                    last_processed_id = id;
                }
                Err(skipped) => {
                    tracing::warn!(
                        skipped,
                        last_processed_id,
                        "MemoryProjector: broadcast lag detected, replaying from event store"
                    );
                    let missed = event_store.get_events_after_id(last_processed_id);
                    for ev in missed {
                        let id = ev.id;
                        self.on_event(&ev);
                        last_processed_id = last_processed_id.max(id);
                    }
                }
            }
        }
    }

    /// 이벤트 처리 (Stream 루프 내부에서 호출)
    fn on_event(&self, event: &DomainEvent) {
        match &event.payload {
            EventPayload::DialogueTurnCompleted {
                npc_id,
                utterance,
                speaker,
                ..
            } => {
                self.index_dialogue(event, npc_id, utterance, speaker);
            }

            EventPayload::RelationshipUpdated {
                owner_id,
                target_id,
                before_closeness,
                after_closeness,
                before_trust,
                after_trust,
                ..
            } => {
                let delta = (after_closeness - before_closeness).abs()
                    + (after_trust - before_trust).abs();
                if delta > RELATIONSHIP_CHANGE_THRESHOLD {
                    self.index_relationship(event, owner_id, target_id, delta);
                }
            }

            EventPayload::BeatTransitioned {
                npc_id,
                from_focus_id,
                to_focus_id,
                partner_id: _,
            } => {
                self.index_beat_transition(event, npc_id, from_focus_id, to_focus_id);
            }

            EventPayload::SceneEnded {
                npc_id,
                partner_id,
            } => {
                self.index_scene_end(event, npc_id, partner_id);
            }

            _ => {}
        }
    }

    fn index_dialogue(&self, event: &DomainEvent, npc_id: &str, utterance: &str, speaker: &str) {
        let content = format!("[{}] {}", speaker, utterance);
        let entry = self.make_entry(npc_id, &content, event, MemoryType::DialogueTurn);
        let embedding = self.embed(&content);
        self.persist(entry, embedding, event.id);
    }

    fn index_relationship(
        &self,
        event: &DomainEvent,
        owner_id: &str,
        target_id: &str,
        delta: f32,
    ) {
        let content = format!(
            "{}와의 관계가 변화함 (변동폭: {:.2})",
            target_id, delta
        );
        let entry = self.make_entry(owner_id, &content, event, MemoryType::RelationshipChange);
        let embedding = self.embed(&content);
        self.persist(entry, embedding, event.id);
    }

    fn index_beat_transition(
        &self,
        event: &DomainEvent,
        npc_id: &str,
        from_focus_id: &Option<String>,
        to_focus_id: &str,
    ) {
        let from = from_focus_id.as_deref().unwrap_or("(없음)");
        let content = format!("감정 전환: {} → {}", from, to_focus_id);
        let entry = self.make_entry(npc_id, &content, event, MemoryType::BeatTransition);
        let embedding = self.embed(&content);
        self.persist(entry, embedding, event.id);
    }

    fn index_scene_end(&self, event: &DomainEvent, npc_id: &str, partner_id: &str) {
        let content = format!("{}와의 대화가 종료됨", partner_id);
        let entry = self.make_entry(npc_id, &content, event, MemoryType::SceneSummary);
        let embedding = self.embed(&content);
        self.persist(entry, embedding, event.id);
    }

    /// MemoryStore 저장 — 실패 시 로깅하여 디버깅 가시성 확보
    fn persist(&self, entry: MemoryEntry, embedding: Option<Vec<f32>>, event_id: u64) {
        if let Err(e) = self.memory_store.index(entry, embedding) {
            tracing::warn!(event_id, error = ?e, "MemoryProjector: memory_store.index failed");
        }
    }

    fn make_entry(
        &self,
        npc_id: &str,
        content: &str,
        event: &DomainEvent,
        memory_type: MemoryType,
    ) -> MemoryEntry {
        let id = self.next_id();
        MemoryEntry::personal(
            id,
            npc_id,
            content,
            None,
            event.timestamp_ms,
            event.id,
            memory_type,
        )
    }

    fn next_id(&self) -> String {
        let mut counter = self.id_counter.lock().unwrap();
        *counter += 1;
        format!("mem-{:06}", *counter)
    }

    fn embed(&self, text: &str) -> Option<Vec<f32>> {
        let mut embedder = match self.embedder.lock() {
            Ok(e) => e,
            Err(e) => {
                tracing::error!(error = ?e, "MemoryProjector: embedder mutex poisoned");
                return None;
            }
        };
        match embedder.embed(&[text]) {
            Ok(v) => v.into_iter().next(),
            Err(e) => {
                tracing::warn!(error = ?e, text_len = text.len(), "MemoryProjector: embedding failed");
                None
            }
        }
    }
}

