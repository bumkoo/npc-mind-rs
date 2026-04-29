// wuxia-core/src/memory/mod.rs
//
// Memory Module — NPC 기억 스트림을 위한 타입과 포트.
//
// 헥사고날 아키텍처에서 llm/ 모듈과 대칭적 역할:
//   wuxia-core가 타입과 trait을 정의하고,
//   wuxia-memory가 구현한다.
//
// 모듈 구조:
//   types.rs — 데이터 타입 (MemoryType, MemoryEntry, ScoredMemory)
//   port.rs  — MemoryRepository trait (save, search, ...)
//   event.rs — MemoryEvent enum (MemoryStored, MemoryRecalled, ...)
//
// 도메인 소유 관계:
//   타입 정의 → 여기 (memory/)
//   비즈니스 규칙 → psychology/ (향후)
//   저장소 구현 → wuxia-memory/ (InMemory, LanceDB)
//
// 사용 예:
//   use wuxia_core::memory::{MemoryRepository, MemoryEntry, MemoryType};

pub mod event;
pub mod port;
pub mod recall;
pub mod retrieval;
pub mod service;
pub mod types;

// Re-export for convenience
// EmbeddingPort, cosine_similarity, l2_normalize → shared/embedding.rs (범용 인프라)
pub use event::MemoryEvent;
pub use port::MemoryRepository;
pub use retrieval::{EmotionalBias, RankedMemory, RetrievalWeights, rank_memories, retrieval_score};
pub use recall::recall_memories;
pub use service::{store_memory, recall_and_emit, update_importance};
pub use types::{MemoryEntry, MemoryImportance, MemoryType, ScoredMemory};
