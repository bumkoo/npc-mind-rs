//! MemoryAgent — EventBus 구독 기반 기억 인덱싱 에이전트
//!
//! 도메인 이벤트를 수신하여 NPC 기억으로 변환·인덱싱합니다.
//! `embed` feature 필수 — TextEmbedder로 임베딩 생성.

use crate::application::event_bus::EventBus;
use crate::domain::event::{DomainEvent, EventPayload};
use crate::domain::memory::{MemoryEntry, MemoryType};
use crate::ports::{MemoryStore, TextEmbedder};

use std::sync::{Arc, Mutex};

/// 관계 변화 유의미 판단 임계값
const RELATIONSHIP_CHANGE_THRESHOLD: f32 = 0.05;

/// 기억 인덱싱 에이전트
///
/// EventBus를 구독하여 관련 이벤트 발생 시 자동으로 기억을 생성·인덱싱합니다.
/// CommandHandler가 아닌 EventBus subscriber입니다.
pub struct MemoryAgent {
    memory_store: Arc<dyn MemoryStore>,
    embedder: Arc<Mutex<dyn TextEmbedder + Send>>,
    id_counter: Mutex<u64>,
}

impl MemoryAgent {
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

    /// EventBus에 자기 자신을 구독자로 등록 (동기, Tier 1)
    pub fn subscribe_to(self: &Arc<Self>, bus: &EventBus) {
        let agent = Arc::clone(self);
        bus.subscribe(move |event| {
            agent.on_event(event);
        });
    }

    /// TieredEventBus의 Tier 2에 등록 (비동기, 백그라운드 스레드)
    ///
    /// `subscribe_to`와 달리 dispatch()를 블로킹하지 않습니다.
    /// 임베딩(~50ms) 등 시간 소요 작업이 백그라운드에서 실행됩니다.
    pub fn register_async(
        self: &Arc<Self>,
        bus: &crate::application::tiered_event_bus::TieredEventBus,
    ) {
        let agent = Arc::clone(self);
        let sink = crate::application::tiered_event_bus::StdThreadSink::spawn(move |event| {
            agent.on_event(&event);
        });
        bus.register_async(sink);
    }

    /// 이벤트 처리 (EventBus 콜백에서 호출)
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
        let entry = self.make_entry(npc_id, &content, event, MemoryType::Dialogue);
        let embedding = self.embed(&content);
        let _ = self.memory_store.index(entry, embedding);
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
        let entry = self.make_entry(owner_id, &content, event, MemoryType::Relationship);
        let embedding = self.embed(&content);
        let _ = self.memory_store.index(entry, embedding);
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
        let _ = self.memory_store.index(entry, embedding);
    }

    fn index_scene_end(&self, event: &DomainEvent, npc_id: &str, partner_id: &str) {
        let content = format!("{}와의 대화가 종료됨", partner_id);
        let entry = self.make_entry(npc_id, &content, event, MemoryType::SceneEnd);
        let embedding = self.embed(&content);
        let _ = self.memory_store.index(entry, embedding);
    }

    fn make_entry(
        &self,
        npc_id: &str,
        content: &str,
        event: &DomainEvent,
        memory_type: MemoryType,
    ) -> MemoryEntry {
        let id = self.next_id();
        MemoryEntry {
            id,
            npc_id: npc_id.to_string(),
            content: content.to_string(),
            emotional_context: None,
            timestamp_ms: event.timestamp_ms,
            event_id: event.id,
            memory_type,
        }
    }

    fn next_id(&self) -> String {
        let mut counter = self.id_counter.lock().unwrap();
        *counter += 1;
        format!("mem-{:06}", *counter)
    }

    fn embed(&self, text: &str) -> Option<Vec<f32>> {
        let mut embedder = self.embedder.lock().ok()?;
        let result = embedder.embed(&[text]).ok()?;
        result.into_iter().next()
    }
}
