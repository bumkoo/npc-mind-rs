// wuxia-core/src/character/life_stage.rs
//
// Life Stage — the phases of a character's life in the wuxia world.
//
// In wuxia stories, a character's life stage deeply affects their journey:
//   Youth  (청년): Learning martial arts, forming bonds, reckless courage
//   Prime  (장년): Peak strength, taking on responsibilities, leading
//   Middle (중년): Experience over power, mentoring the young
//   Elder  (노년): Wisdom at its peak, body declining, legacy matters
//
// These stages will later affect:
//   - Growth rates (Iteration 2.2: youth trains faster)
//   - Stat decay   (Iteration 2.2: elder loses vitality, gains wisdom)
//   - NPC behavior (Phase 3: elder NPCs speak differently)
//   - Story events (Phase 4: retirement arcs, succession crises)

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::shared::i18n::Translatable;

// ---------------------------------------------------------------------------
// Age thresholds for life stages
// ---------------------------------------------------------------------------

/// Age at which Youth ends and Prime begins.
pub const PRIME_AGE: u32 = 33;

/// Age at which Prime ends and Middle begins.
pub const MIDDLE_AGE: u32 = 55;

/// Age at which Middle ends and Elder begins.
pub const ELDER_AGE: u32 = 69;

// ---------------------------------------------------------------------------
// LifeStage enum
// ---------------------------------------------------------------------------

/// The four life stages of a character.
///
/// Each stage represents a distinct phase with different narrative weight
/// and (in later iterations) different growth/decay modifiers.
///
/// ```
/// use wuxia_core::character::LifeStage;
///
/// assert_eq!(LifeStage::from_age(20), LifeStage::Youth);
/// assert_eq!(LifeStage::from_age(40), LifeStage::Prime);
/// assert_eq!(LifeStage::from_age(60), LifeStage::Middle);
/// assert_eq!(LifeStage::from_age(75), LifeStage::Elder);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifeStage {
    /// 청년 (~32): Fast learner, impulsive, forming identity.
    /// Think of young Linghu Chong — talented but reckless.
    Youth,

    /// 장년 (33~54): Peak power, taking leadership roles.
    /// Think of Qiao Feng — at the height of his abilities.
    Prime,

    /// 중년 (55~68): Experienced, strategic, beginning to slow.
    /// Think of Yue Buqun — scheming, experienced, past physical peak.
    Middle,

    /// 노년 (69~): Wisdom over strength, legacy and mentoring.
    /// Think of Zhang Sanfeng — ancient, wise, still formidable.
    Elder,
}

impl LifeStage {
    /// Determine life stage from age.
    ///
    /// This is a pure function — same age always gives same stage.
    pub fn from_age(age: u32) -> Self {
        match age {
            0..PRIME_AGE => LifeStage::Youth,
            PRIME_AGE..MIDDLE_AGE => LifeStage::Prime,
            MIDDLE_AGE..ELDER_AGE => LifeStage::Middle,
            ELDER_AGE.. => LifeStage::Elder,
        }
    }
}

impl fmt::Display for LifeStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Translatable for LifeStage {
    fn translation_key(&self) -> &'static str {
        match self {
            LifeStage::Youth => "life_stage.youth",
            LifeStage::Prime => "life_stage.prime",
            LifeStage::Middle => "life_stage.middle",
            LifeStage::Elder => "life_stage.elder",
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youth_stage() {
        // 0세부터 32세까지는 청년
        assert_eq!(LifeStage::from_age(0), LifeStage::Youth);
        assert_eq!(LifeStage::from_age(15), LifeStage::Youth);
        assert_eq!(LifeStage::from_age(32), LifeStage::Youth);
    }

    #[test]
    fn prime_stage() {
        // 33세부터 54세까지는 장년
        assert_eq!(LifeStage::from_age(33), LifeStage::Prime);
        assert_eq!(LifeStage::from_age(40), LifeStage::Prime);
        assert_eq!(LifeStage::from_age(54), LifeStage::Prime);
    }

    #[test]
    fn middle_stage() {
        // 55세부터 68세까지는 중년
        assert_eq!(LifeStage::from_age(55), LifeStage::Middle);
        assert_eq!(LifeStage::from_age(62), LifeStage::Middle);
        assert_eq!(LifeStage::from_age(68), LifeStage::Middle);
    }

    #[test]
    fn elder_stage() {
        // 69세부터는 노년
        assert_eq!(LifeStage::from_age(69), LifeStage::Elder);
        assert_eq!(LifeStage::from_age(80), LifeStage::Elder);
        assert_eq!(LifeStage::from_age(100), LifeStage::Elder);
    }

    #[test]
    fn boundary_transitions() {
        // 경계값에서 정확히 전환되는지 확인
        assert_eq!(LifeStage::from_age(32), LifeStage::Youth);
        assert_eq!(LifeStage::from_age(33), LifeStage::Prime);

        assert_eq!(LifeStage::from_age(54), LifeStage::Prime);
        assert_eq!(LifeStage::from_age(55), LifeStage::Middle);

        assert_eq!(LifeStage::from_age(68), LifeStage::Middle);
        assert_eq!(LifeStage::from_age(69), LifeStage::Elder);
    }

    #[test]
    fn display_format() {
        assert_eq!(LifeStage::Youth.to_string(), "Youth");
        assert_eq!(LifeStage::Prime.to_string(), "Prime");
        assert_eq!(LifeStage::Middle.to_string(), "Middle");
        assert_eq!(LifeStage::Elder.to_string(), "Elder");
    }

    #[test]
    fn clone_and_eq() {
        let a = LifeStage::Prime;
        let b = a; // Copy
        assert_eq!(a, b);
    }

    #[test]
    fn serialization_roundtrip() {
        let original = LifeStage::Middle;
        let json = serde_json::to_string(&original).unwrap();
        let restored: LifeStage = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }
}
