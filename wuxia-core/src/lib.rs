// wuxia-core/src/lib.rs
//
// Wuxia RPG - Pure Domain Logic Library
//
// This crate contains ALL game logic with ZERO game engine dependency.
// It can be tested with `cargo test -p wuxia-core` in milliseconds.
//
// Module structure follows Domain-Driven Design:
//   shared/       - Shared Kernel (IDs, GameTime, Events, Errors, Ports)
//   character/    - Character domain (identity, age, role, life stage)
//   time/         - Time domain (GameClock, day/season/year progression)
//   growth/       - Growth domain (stats, training, martial arts)
//   llm/          - LLM port (LlmPort trait, Message, sampling)
//   memory/       - Memory domain (MemoryEntry, retrieval, recall)
//   psychology/   - Psychology domain (HEXACO, OCC emotions, PAD mood)
//   relationship/ - Relationship domain (affinity/trust, sentiment, chronicle)
//   application/  - Application Services (domain orchestration, no business logic)

pub mod application;
pub mod character;
pub mod experience;
pub mod growth;
pub mod llm;
pub mod memory;
pub mod psychology;
pub mod relationship;
pub mod shared;
pub mod time;

#[cfg(test)]
pub mod test_fixtures;

// ─── Facade re-exports ──────────────────────────────────────────────
// Commonly used types available at the crate root: `use wuxia_core::CharacterId;`

// Shared Kernel — IDs
pub use shared::{
    CharacterId, ExperienceId, ItemId, LocationId, SectId, NationId, MartialArtId, MemoryId, RelationshipId,
};
// Shared Kernel — time, events, errors
pub use shared::{GameTime, Season, Watch, DomainEvent, DomainError, DomainResult};
pub use shared::{PortError, PortErrorKind};
// Shared Kernel — i18n & config
pub use shared::{Locale, Translatable};
// Shared Kernel — ports
pub use shared::EmbeddingPort;
