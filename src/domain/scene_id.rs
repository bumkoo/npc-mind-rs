//! SceneId — 다중 Scene 식별자 (B안 B4 Session 2)
//!
//! 현재 Scene은 자기만의 고유 id 필드를 갖지 않으며, (npc_id, partner_id) 조합이
//! 유일한 식별자 역할을 해왔다. 다중 Scene 환경에서 이 composite key를 타입으로 고정해
//! `Director`의 Scene 라우팅·`InMemoryRepository`의 `HashMap` 키·`AggregateKey::Scene`
//! 등 여러 곳에서 일관되게 사용한다.
//!
//! **B4 Session 3 Migration Note:** Scene 자체에 UUID 또는 명시적 id 필드를 부여하면
//! 이 타입은 단순 wrapper로 좁아질 수 있다. 현 단계에서는 composite 유지.

use serde::{Deserialize, Serialize};
use std::fmt;

use super::emotion::Scene;

/// Scene 식별자 — (npc_id, partner_id) composite key
///
/// 두 필드의 순서가 의미를 갖는다 — `{npc_id: "a", partner_id: "b"}`와
/// `{npc_id: "b", partner_id: "a"}`는 다른 Scene.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SceneId {
    pub npc_id: String,
    pub partner_id: String,
}

impl SceneId {
    /// 직접 생성
    pub fn new(npc_id: impl Into<String>, partner_id: impl Into<String>) -> Self {
        Self {
            npc_id: npc_id.into(),
            partner_id: partner_id.into(),
        }
    }
}

impl From<&Scene> for SceneId {
    fn from(scene: &Scene) -> Self {
        Self {
            npc_id: scene.npc_id().to_string(),
            partner_id: scene.partner_id().to_string(),
        }
    }
}

impl fmt::Display for SceneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}↔{}", self.npc_id, self.partner_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::emotion::Scene;
    use std::collections::HashSet;

    #[test]
    fn new_builds_from_string_like() {
        let id = SceneId::new("a", "b");
        assert_eq!(id.npc_id, "a");
        assert_eq!(id.partner_id, "b");
    }

    #[test]
    fn composite_key_is_order_sensitive() {
        let a = SceneId::new("alice", "bob");
        let b = SceneId::new("bob", "alice");
        assert_ne!(a, b, "(npc, partner) 순서는 의미 있음");
    }

    #[test]
    fn from_scene_extracts_ids() {
        let scene = Scene::new("alice".into(), "bob".into(), vec![]);
        let id: SceneId = (&scene).into();
        assert_eq!(id, SceneId::new("alice", "bob"));
    }

    #[test]
    fn hashable_for_hashmap_key() {
        let mut set: HashSet<SceneId> = HashSet::new();
        set.insert(SceneId::new("a", "b"));
        set.insert(SceneId::new("a", "b"));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn display_shows_npc_partner_arrow() {
        let id = SceneId::new("a", "b");
        assert_eq!(id.to_string(), "a↔b");
    }
}
