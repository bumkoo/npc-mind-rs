//! BondKind + BondStatus — 관계의 정서·기능 분류 + 활동 상태.
//!
//! relationships.md v0.7 §3.1 (BondKind 11) + §3.5 (BondStatus 5).
//! Phase 2는 *enum 정의 + 영역 헬퍼 / accepts_live_input*까지.
//! 자동 진입/이탈 (시간 게이트 + 임계값) 및 전이 룰은 Phase 3a (Channel 2 Temporal).

use crate::domain::world::event::EventId;
use serde::{Deserialize, Serialize};

/// 관계의 정서·기능적 분류 (relationships.md v0.7 §3.1).
///
/// 11 variants 4 영역:
/// - 지기·동반 (양극 임계): 6종 — SwornBrothers, MasterDisciple, Soulmate, LoyalRetainer, Companion, Guardian
/// - 멘토 (중간극 임계): 1종 — Mentor
/// - 원수 (음극 임계): 4종 — BloodEnemy, ArchRival, Betrayer, Oppressor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BondKind {
    // 지기·동반 — 양극 임계 (6종)
    SwornBrothers,
    MasterDisciple,
    Soulmate,
    LoyalRetainer,
    Companion,
    Guardian,

    // 멘토 — 중간극 임계
    Mentor,

    // 원수 — 음극 임계 (4종)
    BloodEnemy,
    ArchRival,
    Betrayer,
    Oppressor,
}

impl BondKind {
    /// 지기 4종 (SwornBrothers, MasterDisciple, Soulmate, LoyalRetainer).
    /// 중국어 *지기(知己)* — 깊은 정신적 동지/지음.
    pub fn is_zhiji(&self) -> bool {
        matches!(
            self,
            Self::SwornBrothers | Self::MasterDisciple | Self::Soulmate | Self::LoyalRetainer
        )
    }

    /// 평생의 우인 (Companion).
    pub fn is_companion_class(&self) -> bool {
        matches!(self, Self::Companion)
    }

    /// 부모-자녀형 (Guardian).
    pub fn is_guardian(&self) -> bool {
        matches!(self, Self::Guardian)
    }

    /// 인생 선배·후배 (Mentor).
    pub fn is_mentor(&self) -> bool {
        matches!(self, Self::Mentor)
    }

    /// 원수 4종 (BloodEnemy, ArchRival, Betrayer, Oppressor).
    pub fn is_enemy(&self) -> bool {
        matches!(
            self,
            Self::BloodEnemy | Self::ArchRival | Self::Betrayer | Self::Oppressor
        )
    }
}

/// 관계의 활동 상태 (relationships.md v0.7 §3.5).
///
/// - `Active`: 정상 활성. axes 자동 변동.
/// - `Resolved { reason }`: terminal — 화해/매듭. axes freeze.
/// - `Deceased`: terminal — 대상 사망. axes freeze.
/// - `Dormant`: 휴면 (오랜 미접촉). axes freeze. 트리거로 Reactivating 전이 가능.
/// - `Reactivating { trigger }`: 복귀 중 (transient state). axes 받기 시작 — *연속적 회복*.
///
/// 전이 룰은 Phase 3a (Channel 2 Temporal). Phase 2는 enum + `accepts_live_input()`까지.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BondStatus {
    Active,
    Resolved { reason: String },
    Deceased,
    Dormant,
    Reactivating { trigger: EventId },
}

impl Default for BondStatus {
    fn default() -> Self {
        BondStatus::Active
    }
}

impl BondStatus {
    /// 4축 자동 변동을 받는지 (Stage 2 base_delta 차단의 핵심 헬퍼).
    ///
    /// - Active: true
    /// - Reactivating: true ★ (연속적 회복 — 재회 시 axes 다시 받기)
    /// - Dormant: false (휴면)
    /// - Resolved: false (terminal freeze)
    /// - Deceased: false (terminal freeze)
    pub fn accepts_live_input(&self) -> bool {
        matches!(self, BondStatus::Active | BondStatus::Reactivating { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zhiji_classifies_four_variants() {
        assert!(BondKind::SwornBrothers.is_zhiji());
        assert!(BondKind::MasterDisciple.is_zhiji());
        assert!(BondKind::Soulmate.is_zhiji());
        assert!(BondKind::LoyalRetainer.is_zhiji());
        assert!(!BondKind::Companion.is_zhiji());
        assert!(!BondKind::Guardian.is_zhiji());
        assert!(!BondKind::Mentor.is_zhiji());
        assert!(!BondKind::BloodEnemy.is_zhiji());
    }

    #[test]
    fn companion_guardian_mentor_classifiers() {
        assert!(BondKind::Companion.is_companion_class());
        assert!(BondKind::Guardian.is_guardian());
        assert!(BondKind::Mentor.is_mentor());
        // 지기와 평생의 우인 구별
        assert!(!BondKind::SwornBrothers.is_companion_class());
    }

    #[test]
    fn enemy_classifies_four_variants() {
        assert!(BondKind::BloodEnemy.is_enemy());
        assert!(BondKind::ArchRival.is_enemy());
        assert!(BondKind::Betrayer.is_enemy());
        assert!(BondKind::Oppressor.is_enemy());
        assert!(!BondKind::Mentor.is_enemy());
    }

    #[test]
    fn each_variant_belongs_to_exactly_one_region() {
        // 11 variants 각각: is_zhiji + is_companion_class + is_guardian + is_mentor + is_enemy 의 합이 정확히 1
        let all = [
            BondKind::SwornBrothers,
            BondKind::MasterDisciple,
            BondKind::Soulmate,
            BondKind::LoyalRetainer,
            BondKind::Companion,
            BondKind::Guardian,
            BondKind::Mentor,
            BondKind::BloodEnemy,
            BondKind::ArchRival,
            BondKind::Betrayer,
            BondKind::Oppressor,
        ];
        for k in all {
            let n = [
                k.is_zhiji(),
                k.is_companion_class(),
                k.is_guardian(),
                k.is_mentor(),
                k.is_enemy(),
            ]
            .iter()
            .filter(|b| **b)
            .count();
            assert_eq!(n, 1, "variant {:?} should belong to exactly one region", k);
        }
    }

    #[test]
    fn bond_kind_serde_snake_case() {
        let json = serde_json::to_string(&BondKind::SwornBrothers).unwrap();
        assert_eq!(json, "\"sworn_brothers\"");
        let back: BondKind = serde_json::from_str("\"blood_enemy\"").unwrap();
        assert_eq!(back, BondKind::BloodEnemy);
    }

    #[test]
    fn bond_kind_serde_round_trip_all() {
        let all = [
            BondKind::SwornBrothers,
            BondKind::MasterDisciple,
            BondKind::LoyalRetainer,
            BondKind::BloodEnemy,
        ];
        for k in all {
            let json = serde_json::to_string(&k).unwrap();
            let back: BondKind = serde_json::from_str(&json).unwrap();
            assert_eq!(back, k);
        }
    }

    #[test]
    fn active_and_reactivating_accept_live_input() {
        assert!(BondStatus::Active.accepts_live_input());
        assert!(
            BondStatus::Reactivating {
                trigger: EventId("evt_001".into())
            }
            .accepts_live_input()
        );
    }

    #[test]
    fn dormant_resolved_deceased_block_live_input() {
        assert!(!BondStatus::Dormant.accepts_live_input());
        assert!(
            !BondStatus::Resolved {
                reason: "사화".into()
            }
            .accepts_live_input()
        );
        assert!(!BondStatus::Deceased.accepts_live_input());
    }

    #[test]
    fn default_is_active() {
        assert_eq!(BondStatus::default(), BondStatus::Active);
    }

    #[test]
    fn bond_status_serde_active() {
        let json = serde_json::to_string(&BondStatus::Active).unwrap();
        assert_eq!(json, r#"{"kind":"active"}"#);
        let back: BondStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, BondStatus::Active);
    }

    #[test]
    fn bond_status_serde_resolved_with_payload() {
        let s = BondStatus::Resolved {
            reason: "사화".into(),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: BondStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn bond_status_serde_reactivating_with_event_id() {
        let s = BondStatus::Reactivating {
            trigger: EventId("evt_001".into()),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: BondStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }
}
