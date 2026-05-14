//! Partnership — 관계의 형식적 동반 상태.
//!
//! relationships.md v0.7 §3.6.
//! BondKind와 *완전 직교* — 정략결혼 = trust 0 + Spouse 가능.
//! axes와 *직접 연동 X*. 변화 동력은 *공식 사건* (Phase 2.5 declarative_events `PartnershipChange`).

use serde::{Deserialize, Serialize};

/// 형식적 동반 상태 (relationships.md v0.7 §3.6).
///
/// `Relationship.partnership: Option<Partnership>` 패턴으로 사용 (None = 형식 관계 없음).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Partnership {
    Spouse,
    Engaged,
    Lover,
    Separated,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn serde_round_trip_all_variants() {
        for v in [
            Partnership::Spouse,
            Partnership::Engaged,
            Partnership::Lover,
            Partnership::Separated,
        ] {
            let json = serde_json::to_string(&v).unwrap();
            let back: Partnership = serde_json::from_str(&json).unwrap();
            assert_eq!(back, v);
        }
    }

    #[test]
    fn serde_snake_case_labels() {
        assert_eq!(
            serde_json::to_string(&Partnership::Spouse).unwrap(),
            "\"spouse\""
        );
        assert_eq!(
            serde_json::to_string(&Partnership::Separated).unwrap(),
            "\"separated\""
        );
    }

    #[test]
    fn copy_and_eq_work() {
        let a = Partnership::Spouse;
        let b = a; // Copy
        assert_eq!(a, b);
    }

    #[test]
    fn hash_in_set() {
        let mut s = HashSet::new();
        s.insert(Partnership::Spouse);
        s.insert(Partnership::Engaged);
        assert!(s.contains(&Partnership::Spouse));
        assert!(!s.contains(&Partnership::Lover));
    }
}
