// wuxia-core/src/character/mod.rs
//
// Character Domain — "이 존재는 누구인가?"
//
// The character domain is the foundational identity layer.
// Every other domain references CharacterId to manage their own data.
//
// This domain owns:
//   - Identity (name, courtesy name, gender)
//   - Age and life stage transitions
//   - Role classification (Player / NPC / Companion)
//   - CharacterEvent (도메인 이벤트)
//
// This domain does NOT own (by design):
//   - Stats or martial arts  → Growth domain
//   - Emotions or personality → Psychology domain
//   - Relationships          → Relationship domain
//   - Sect membership        → World domain

pub mod event;
pub mod fatigue;
pub mod gender;
pub mod injury;
pub mod life_stage;
pub mod model;
pub mod role;

// Re-export for convenience: `use wuxia_core::character::Character;`
pub use event::CharacterEvent;
pub use fatigue::{FatigueLevel, DAILY_REST_RECOVERY, FATIGUE_MAX, FATIGUE_MIN};
pub use gender::Gender;
pub use injury::{Injury, InjurySeverity, InjuryType};
pub use life_stage::{LifeStage, PRIME_AGE, MIDDLE_AGE, ELDER_AGE};
pub use model::Character;
pub use role::CharacterRole;
