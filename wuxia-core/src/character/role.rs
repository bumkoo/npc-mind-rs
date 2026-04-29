// wuxia-core/src/character/role.rs
//
// CharacterRole — 캐릭터의 역할 값 객체.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::shared::i18n::Translatable;

/// The role a character plays in the game.
///
/// This determines HOW the character is controlled:
///   - Player:    Human input drives decisions
///   - Npc:       LLM drives decisions (via llama-cpp-2)
///   - Companion: Simplified AI (animal companions like 神雕, 汗血宝马)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CharacterRole {
    /// Controlled by the human player.
    Player,
    /// Controlled by the LLM-based AI system.
    Npc,
    /// Animal or mystical creature companion.
    /// Has simplified emotion and growth, but no speech.
    Companion,
}

impl fmt::Display for CharacterRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Translatable for CharacterRole {
    fn translation_key(&self) -> &'static str {
        match self {
            CharacterRole::Player => "character_role.player",
            CharacterRole::Npc => "character_role.npc",
            CharacterRole::Companion => "character_role.companion",
        }
    }
}
