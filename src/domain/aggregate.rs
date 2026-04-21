//! AggregateKey — 이벤트·커맨드의 애그리게이트 경계 식별자
//!
//! B안(다중 Scene 동시 실행) 이행 Stage B0 선행 준비 타입.
//! Scene/Npc/Relationship 세 종류의 aggregate로 이벤트 스트림을 분할하여
//! 향후 SceneTask가 자기 aggregate의 이벤트만 순차 처리할 수 있게 한다.
//!
//! 현재 코드베이스는 `EventPayload` / `Command`에 `scene_id` 필드가 없어
//! Scene 키는 `(npc_id, partner_id)` 조합으로 임시 식별한다.
//! B1+에서 scene_id 필드가 추가되면 Emotion/Stimulus 계열 이벤트를
//! `Scene` 키로 승격하는 리팩터링이 필요하다.

use serde::{Deserialize, Serialize};
use std::fmt;

/// 이벤트·커맨드 라우팅 기준이 되는 애그리게이트 식별자
///
/// 같은 `AggregateKey`를 공유하는 이벤트들은 하나의 SceneTask/순차 처리 단위로
/// 묶이며, 서로 다른 키를 가진 이벤트 간에는 순서 보장이 없다.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AggregateKey {
    /// Scene 단위 — 같은 (npc_id, partner_id)의 대화가 하나의 Scene을 이룬다
    Scene {
        npc_id: String,
        partner_id: String,
    },
    /// 특정 NPC의 Scene 밖 개별 평가
    Npc(String),
    /// 관계 업데이트 — (owner, target) 쌍
    Relationship {
        owner_id: String,
        target_id: String,
    },
    /// Memory 애그리거트 — `MemoryEntryId` 단위 라우팅 (Step C1 foundation, §3.3).
    /// `MemoryEntryCreated/Superseded/Consolidated` 이벤트가 이 키를 쓴다.
    Memory(String),
    /// Rumor 애그리거트 — `RumorId` 단위 라우팅 (Step C1, §3.3).
    /// `RumorSeeded/Spread/Distorted/Faded` 및 `SeedRumorRequested/SpreadRumorRequested`가
    /// 이 키를 쓴다.
    Rumor(String),
    /// World 오버레이 — `WorldId` 단위 라우팅 (Step C1 foundation, §3.3).
    /// `WorldEventOccurred/ApplyWorldEventRequested` 이벤트가 이 키를 쓴다 (Step D에서 사용).
    World(String),
}

impl AggregateKey {
    /// 이 aggregate와 연관된 NPC id 힌트 (로깅/트레이싱용)
    ///
    /// Scene/Npc은 `npc_id`를, Relationship은 `owner_id`를 반환.
    /// Memory/Rumor/World는 NPC 소속이 아니므로 애그리거트 id를 그대로 반환한다
    /// (로그 가독성 목적 — "실제 NPC id"로 해석하면 안 됨).
    pub fn npc_id_hint(&self) -> &str {
        match self {
            AggregateKey::Scene { npc_id, .. } => npc_id,
            AggregateKey::Npc(npc_id) => npc_id,
            AggregateKey::Relationship { owner_id, .. } => owner_id,
            AggregateKey::Memory(id) => id,
            AggregateKey::Rumor(id) => id,
            AggregateKey::World(id) => id,
        }
    }
}

impl fmt::Display for AggregateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AggregateKey::Scene { npc_id, partner_id } => {
                write!(f, "Scene({npc_id}↔{partner_id})")
            }
            AggregateKey::Npc(npc_id) => write!(f, "Npc({npc_id})"),
            AggregateKey::Relationship {
                owner_id,
                target_id,
            } => write!(f, "Rel({owner_id}→{target_id})"),
            AggregateKey::Memory(id) => write!(f, "Memory({id})"),
            AggregateKey::Rumor(id) => write!(f, "Rumor({id})"),
            AggregateKey::World(id) => write!(f, "World({id})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn scene_key_equality_respects_both_ids() {
        let a = AggregateKey::Scene {
            npc_id: "muback".into(),
            partner_id: "gyoryong".into(),
        };
        let b = AggregateKey::Scene {
            npc_id: "muback".into(),
            partner_id: "gyoryong".into(),
        };
        let c = AggregateKey::Scene {
            npc_id: "gyoryong".into(),
            partner_id: "muback".into(),
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn different_variants_never_equal() {
        let scene = AggregateKey::Scene {
            npc_id: "a".into(),
            partner_id: "b".into(),
        };
        let npc = AggregateKey::Npc("a".into());
        let rel = AggregateKey::Relationship {
            owner_id: "a".into(),
            target_id: "b".into(),
        };
        assert_ne!(scene, npc);
        assert_ne!(scene, rel);
        assert_ne!(npc, rel);
    }

    #[test]
    fn hashable_for_use_as_map_key() {
        let mut set = HashSet::new();
        set.insert(AggregateKey::Npc("muback".into()));
        set.insert(AggregateKey::Npc("muback".into()));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn npc_id_hint_returns_correct_field() {
        assert_eq!(AggregateKey::Npc("x".into()).npc_id_hint(), "x");
        assert_eq!(
            AggregateKey::Scene {
                npc_id: "x".into(),
                partner_id: "y".into()
            }
            .npc_id_hint(),
            "x"
        );
        assert_eq!(
            AggregateKey::Relationship {
                owner_id: "x".into(),
                target_id: "y".into()
            }
            .npc_id_hint(),
            "x"
        );
    }

    #[test]
    fn display_format_is_stable() {
        assert_eq!(AggregateKey::Npc("a".into()).to_string(), "Npc(a)");
        assert_eq!(
            AggregateKey::Scene {
                npc_id: "a".into(),
                partner_id: "b".into()
            }
            .to_string(),
            "Scene(a↔b)"
        );
        assert_eq!(
            AggregateKey::Relationship {
                owner_id: "a".into(),
                target_id: "b".into()
            }
            .to_string(),
            "Rel(a→b)"
        );
    }

    #[test]
    fn memory_rumor_world_variants_format_and_hint() {
        assert_eq!(AggregateKey::Memory("m1".into()).to_string(), "Memory(m1)");
        assert_eq!(AggregateKey::Rumor("r1".into()).to_string(), "Rumor(r1)");
        assert_eq!(AggregateKey::World("w1".into()).to_string(), "World(w1)");

        assert_eq!(AggregateKey::Memory("m1".into()).npc_id_hint(), "m1");
        assert_eq!(AggregateKey::Rumor("r1".into()).npc_id_hint(), "r1");
        assert_eq!(AggregateKey::World("w1".into()).npc_id_hint(), "w1");

        // 서로 다른 variant는 서로 같지 않다.
        assert_ne!(
            AggregateKey::Memory("x".into()),
            AggregateKey::Rumor("x".into())
        );
        assert_ne!(
            AggregateKey::Rumor("x".into()),
            AggregateKey::World("x".into())
        );
    }
}
