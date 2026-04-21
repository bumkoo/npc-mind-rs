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
    GuideRequest, GuideResponse, GuideResult, RumorOriginInput, RumorReachInput, SceneRequest,
    SceneResponse, SeedRumorRequest, SpreadRumorRequest, StimulusRequest, StimulusResponse,
    StimulusResult, TellInformationRequest,
};
// Response DTOs (TellInformationResponse / SeedRumorResponse / SpreadRumorResponse)는
// 현재 dispatcher가 생성·반환하지 않으므로 공개 노출에서 뺀다. typed facade가 필요한
// 시점(Step D에서 dispatch 결과 타입 확장 논의)에 공개하거나 제거 재검토.
pub use ports::{
    AnchorLoadError, Appraiser, EmotionStore, GuideFormatter, MindRepository, NpcWorld,
    PadAnchorSource, PersonalityProfile, SceneStore, StimulusProcessor,
};
pub use presentation::builtin_toml;
pub use presentation::formatter::LocaleFormatter;

// --- Event Sourcing / EventBus ---
pub use application::event_bus::EventBus;
pub use application::event_store::{EventStore, InMemoryEventStore};
pub use domain::event::{DomainEvent, EventMetadata, EventPayload};

// --- CQRS Command / Agent (v2) ---
pub use application::command::{
    Command, CommandDispatcher, EmotionAgent, EmotionProjectionHandler, GuideAgent,
    InformationAgent, RelationshipAgent, RelationshipProjectionHandler, RumorAgent,
    RumorDistributionHandler, SceneProjectionHandler, TellingIngestionHandler,
};

// --- Memory / RAG ---
pub use domain::memory::{MemoryEntry, MemoryResult, MemoryType};
pub use ports::{MemoryError, MemoryStore};

// --- Rumor (Step C1 foundation) ---
pub use domain::rumor::{
    ReachPolicy, Rumor, RumorDistortion, RumorError, RumorHop, RumorOrigin, RumorStatus,
};
pub use ports::RumorStore;

#[cfg(feature = "embed")]
pub use adapter::sqlite_memory::{SqliteMemoryStore, DEFAULT_EMBEDDING_DIM};
#[cfg(feature = "embed")]
pub use adapter::sqlite_rumor::SqliteRumorStore;
#[cfg(feature = "embed")]
pub use application::memory_agent::MemoryAgent;

#[cfg(feature = "chat")]
pub use adapter::rig_chat::RigChatAdapter;
#[cfg(feature = "chat")]
pub use application::dialogue_agent::{
    DialogueAgent, DialogueAgentError, DialogueEndOutcome, DialogueStartOutcome,
    DialogueTurnOutcome,
};
#[cfg(feature = "chat")]
pub use application::dialogue_test_service::{
    ChatEndRequest, ChatEndResponse, ChatStartRequest, ChatStartResponse, ChatTurnRequest,
    ChatTurnResponse, PadInput,
};
#[cfg(feature = "chat")]
pub use ports::{
    ChatResponse, ConversationError, ConversationPort, DialogueRole, DialogueTurn, LlamaHealth,
    LlamaMetrics, LlamaServerMonitor, LlamaSlotInfo, LlamaTimings,
};
