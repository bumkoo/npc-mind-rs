// wuxia-core/src/character/gender.rs
//
// Gender — 캐릭터의 성별 값 객체.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::shared::i18n::Translatable;

/// Biological sex of a character.
///
/// Kept simple for now. In wuxia stories, gender affects:
/// - Social expectations (disguise plots are common)
/// - Some martial art restrictions (in traditional settings)
/// - Narrative tropes (female warriors defying convention)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
}

impl fmt::Display for Gender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Translatable for Gender {
    fn translation_key(&self) -> &'static str {
        match self {
            Gender::Male => "gender.male",
            Gender::Female => "gender.female",
        }
    }
}
