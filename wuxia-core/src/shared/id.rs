// wuxia-core/src/shared/id.rs
//
// Identity types for every entity in the wuxia world.
//
// WHY Newtype pattern?
// ---------------------
// Without it, all IDs are just `u64` and you can accidentally do:
//
//     let hero: u64 = 1;
//     let sword: u64 = 1;
//     find_character(sword);  // Compiles! But it's a bug.
//
// With Newtype, the compiler catches this mistake:
//
//     let hero = CharacterId(1);
//     let sword = ItemId(1);
//     find_character(sword);  // ERROR: expected CharacterId, found ItemId
//
// A small wrapper. A big safety net.

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Macro to reduce boilerplate: each ID type needs the same derives + Display
// ---------------------------------------------------------------------------
macro_rules! define_id {
    (
        $(#[$meta:meta])*
        $name:ident, $prefix:expr
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub u64);

        impl $name {
            /// Create a new ID with the given numeric value.
            pub fn new(value: u64) -> Self {
                Self(value)
            }

            /// Get the raw numeric value.
            pub fn value(&self) -> u64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                // e.g., "Char-42", "Item-7", "Loc-13"
                write!(f, "{}-{}", $prefix, self.0)
            }
        }
    };
}

// ---------------------------------------------------------------------------
// ID types — one for each domain entity
// ---------------------------------------------------------------------------

define_id!(
    /// Unique identity for a character (player, NPC, or companion).
    /// Example: CharacterId(1) represents "Linghu Chong" in the world.
    CharacterId, "Char"
);

define_id!(
    /// Unique identity for an item (weapon, book, food, etc.).
    ItemId, "Item"
);

define_id!(
    /// Unique identity for a location (city, mountain, tavern, etc.).
    LocationId, "Loc"
);

define_id!(
    /// Unique identity for a martial arts sect (Huashan, Shaolin, etc.).
    SectId, "Sect"
);

define_id!(
    /// Unique identity for a nation (dynasty or republic).
    NationId, "Nation"
);

define_id!(
    /// Unique identity for a martial art technique.
    MartialArtId, "Art"
);

define_id!(
    /// Unique identity for a memory entry in the NPC memory stream.
    /// Used by the psychology domain's reflection system to reference
    /// formation memories (형성기억) and build reflection trees.
    MemoryId, "Mem"
);

define_id!(
    /// Unique identity for a relationship between two characters.
    /// Tracks affinity and trust between source and target.
    RelationshipId, "Rel"
);

define_id!(
    /// Unique identity for an experience event in the event queue.
    /// Each experience (training, combat, conversation, etc.) gets a unique ID
    /// that links the event to its memory entry in the vector DB.
    ExperienceId, "Exp"
);

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_creation_and_value() {
        let id = CharacterId::new(42);
        assert_eq!(id.value(), 42);
    }

    #[test]
    fn id_equality() {
        // Same type, same value → equal
        let a = CharacterId::new(1);
        let b = CharacterId::new(1);
        assert_eq!(a, b);

        // Same type, different value → not equal
        let c = CharacterId::new(2);
        assert_ne!(a, c);
    }

    #[test]
    fn id_display_format() {
        // Each ID type has a human-readable prefix
        assert_eq!(CharacterId::new(1).to_string(), "Char-1");
        assert_eq!(ItemId::new(7).to_string(), "Item-7");
        assert_eq!(LocationId::new(13).to_string(), "Loc-13");
        assert_eq!(SectId::new(3).to_string(), "Sect-3");
        assert_eq!(NationId::new(5).to_string(), "Nation-5");
        assert_eq!(MartialArtId::new(9).to_string(), "Art-9");
        assert_eq!(MemoryId::new(100).to_string(), "Mem-100");
        assert_eq!(RelationshipId::new(42).to_string(), "Rel-42");
        assert_eq!(ExperienceId::new(1).to_string(), "Exp-1");
    }

    #[test]
    fn id_copy_semantics() {
        // IDs are Copy — no ownership issues when passing around
        let original = CharacterId::new(10);
        let copied = original; // Copy, not move
        assert_eq!(original, copied); // Both still valid
    }

    #[test]
    fn id_hash_works_in_collections() {
        // IDs can be used as HashMap keys
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(CharacterId::new(1), "Linghu Chong");
        map.insert(CharacterId::new(2), "Yue Buqun");

        assert_eq!(map.get(&CharacterId::new(1)), Some(&"Linghu Chong"));
        assert_eq!(map.get(&CharacterId::new(99)), None);
    }

    #[test]
    fn different_id_types_are_distinct() {
        // This is the key safety feature:
        // CharacterId(1) and ItemId(1) are different types.
        // They cannot be mixed up at compile time.
        //
        // The following would NOT compile (uncomment to verify):
        //   let character: CharacterId = ItemId::new(1);  // ERROR!
        //
        // We can only verify at runtime that they are semantically different:
        let char_id = CharacterId::new(1);
        let item_id = ItemId::new(1);
        // They hold the same number but are completely different types.
        assert_eq!(char_id.value(), item_id.value()); // Same number
        // But you can't compare them directly: char_id == item_id won't compile
    }

    #[test]
    fn id_serialization_roundtrip() {
        let original = CharacterId::new(42);
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(json, "42"); // Serializes as plain number

        let restored: CharacterId = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }
}
