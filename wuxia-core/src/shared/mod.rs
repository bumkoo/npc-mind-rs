// wuxia-core/src/shared/mod.rs
//
// Shared Kernel — the basic building blocks every domain uses.
//
// In DDD terms, this is the "Shared Kernel": types so fundamental
// that all Bounded Contexts agree on their definition.
//
// Think of it as 강호(Jianghu) common language:
//   - IDs:       Every person/place/thing has a unique identity
//   - GameTime:  Everyone agrees on what day it is
//   - Events:    The news system that domains communicate through
//   - Errors:    Standard way to report problems

pub mod embedding;
pub mod sentiment;
pub mod id;
pub mod time;
pub mod event;
pub mod event_macros;
pub mod error;
pub mod port_error;
pub mod i18n;
pub mod prompt_config;

// Re-export for convenience: `use wuxia_core::shared::CharacterId;`
pub use id::{
    CharacterId, ExperienceId, ItemId, LocationId, SectId, NationId, MartialArtId, MemoryId, RelationshipId,
};
pub use time::{
    GameTime, Season, Watch,
    DAYS_PER_MONTH, DAYS_PER_YEAR, MONTHS_PER_YEAR, WATCHES_PER_DAY, WATCHES_PER_YEAR,
};
pub use event::DomainEvent;
pub use error::{DomainError, DomainResult};
pub use port_error::{PortError, PortErrorKind};
pub use embedding::{EmbeddingPort, cosine_similarity, l2_normalize};
pub use sentiment::{DeltaSource, SentimentDirection, SentimentJudgment, judgment_to_delta};
pub use i18n::{Locale, Translatable, Translations};
pub use prompt_config::{MemoryLabelsConfig, PromptConfig, PromptTemplates};
