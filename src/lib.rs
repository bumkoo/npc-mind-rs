//! NPC Mind Engine
//!
//! 성격(HEXACO)이 상황(Context)을 해석하여
//! 감정(OCC)을 생성하고, LLM 연기 가이드를 출력하는 엔진.

pub mod adapter;
pub mod application;
pub mod domain;
pub mod ports;
pub mod presentation;

// ---------------------------------------------------------------------------
// 편의 재노출 — 라이브러리 사용자의 주요 진입점
// ---------------------------------------------------------------------------

pub use adapter::memory_repository::{InMemoryRepository, RepositoryLoadError};
pub use application::dto::{
    AfterDialogueRequest, AfterDialogueResponse, AppraiseRequest, AppraiseResponse, AppraiseResult,
    GuideRequest, GuideResponse, GuideResult, SceneRequest, SceneResponse, StimulusRequest,
    StimulusResponse, StimulusResult,
};
pub use application::formatted_service::FormattedMindService;
pub use application::mind_service::{MindRepository, MindService, MindServiceError};
pub use ports::{
    AnchorLoadError, Appraiser, EmotionStore, GuideFormatter, NpcWorld, PadAnchorSource,
    PersonalityProfile, SceneStore, StimulusProcessor,
};
pub use presentation::builtin_toml;
pub use presentation::formatter::LocaleFormatter;

// --- Event Sourcing (Phase 1) ---
pub use application::event_bus::EventBus;
pub use application::event_service::EventAwareMindService;
pub use application::event_store::{EventStore, InMemoryEventStore};
pub use application::projection::{
    EmotionProjection, Projection, ProjectionRegistry, RelationshipProjection, SceneProjection,
};
pub use domain::event::{DomainEvent, EventMetadata, EventPayload};

// --- CQRS Command/Agent (Phase 2) ---
pub use application::command::{
    Command, CommandDispatcher, CommandResult, EmotionAgent, GuideAgent, HandlerContext,
    HandlerOutput, RelationshipAgent,
};

// --- Memory / RAG (Phase 3) ---
pub use application::memory_store::InMemoryMemoryStore;
pub use domain::memory::{MemoryEntry, MemoryResult, MemoryType};
pub use ports::{MemoryError, MemoryStore};

#[cfg(feature = "embed")]
pub use adapter::sqlite_memory::SqliteMemoryStore;
#[cfg(feature = "embed")]
pub use application::memory_agent::MemoryAgent;

// --- Pipeline + TieredEventBus ---
pub use application::pipeline::{Pipeline, PipelineStage, PipelineState};
pub use application::tiered_event_bus::{AsyncEventSink, StdThreadSink, TieredEventBus};

#[cfg(feature = "chat")]
pub use application::tiered_event_bus::TokioSink;

#[cfg(feature = "chat")]
pub use adapter::rig_chat::RigChatAdapter;
#[cfg(feature = "chat")]
pub use application::dialogue_test_service::{
    ChatEndRequest, ChatEndResponse, ChatStartRequest, ChatStartResponse, ChatTurnRequest,
    ChatTurnResponse, DialogueTestError, DialogueTestService, PadInput,
};
#[cfg(feature = "chat")]
pub use ports::{
    ChatResponse, ConversationError, ConversationPort, DialogueRole, DialogueTurn, LlamaHealth,
    LlamaMetrics, LlamaServerMonitor, LlamaSlotInfo, LlamaTimings,
};
