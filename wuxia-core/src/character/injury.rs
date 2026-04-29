// wuxia-core/src/character/injury.rs
//
// Injury — 부상 시스템.
//
// 무협 세계에서 과도한 수련이나 전투는 부상을 유발한다.
// 부상은 수련을 제한하고, 치료가 필요한 상태를 만든다.
//
// 부상 유형 (InjuryType):
//   타박(Bruise)       — 가벼운 부상. 수련 강도 -1.
//   근육손상(Strain)   — 중간 부상. 수련 강도 -3.
//   골절(Fracture)     — 심각한 부상. 수련 불가.
//   주화입마(QiDeviation) — 내공 역류. 수련 불가 + 능력치 하락 위험.
//
// 부상 심각도 (InjurySeverity):
//   경상(Minor)  — 3일 자연 치유. 수련 가능(제한).
//   중상(Major)  — 7일 치유. 수련 불가.
//   치명(Critical) — 15일+ 치유. 수련 불가 + 후유증 가능.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::shared::i18n::Translatable;

// ---------------------------------------------------------------------------
// InjuryType
// ---------------------------------------------------------------------------

/// 부상 유형.
///
/// 무협적 해석:
///   Bruise       — 사소한 타박. 무리한 수련의 흔적.
///   Strain       — 근육/인대 손상. 과도한 단련의 대가.
///   Fracture     — 뼈가 부러짐. 전투나 낙상.
///   QiDeviation  — 주화입마(走火入魔). 내공 수련 실패의 최악의 결과.
///
/// ```
/// use wuxia_core::character::injury::InjuryType;
///
/// assert_eq!(InjuryType::Bruise.intensity_penalty(), 1);
/// assert!(InjuryType::QiDeviation.prevents_training());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InjuryType {
    /// 타박 — 가벼운 부상. 수련 강도 -1.
    Bruise,
    /// 근육손상 — 중간 부상. 수련 강도 -3.
    Strain,
    /// 골절 — 심각한 부상. 수련 불가.
    Fracture,
    /// 주화입마(走火入魔) — 내공 역류. 수련 불가.
    QiDeviation,
}

impl InjuryType {
    /// 수련 강도 페널티. 수련 가능 시 최대 강도에서 차감.
    pub fn intensity_penalty(&self) -> u32 {
        match self {
            InjuryType::Bruise => 1,
            InjuryType::Strain => 3,
            InjuryType::Fracture => 0,     // 수련 불가이므로 무의미
            InjuryType::QiDeviation => 0,  // 수련 불가이므로 무의미
        }
    }

    /// 이 부상 상태에서 수련이 가능한가?
    pub fn prevents_training(&self) -> bool {
        matches!(self, InjuryType::Fracture | InjuryType::QiDeviation)
    }

    /// 모든 부상 유형 목록.
    pub fn all() -> &'static [InjuryType] {
        &[
            InjuryType::Bruise,
            InjuryType::Strain,
            InjuryType::Fracture,
            InjuryType::QiDeviation,
        ]
    }
}

impl fmt::Display for InjuryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Translatable for InjuryType {
    fn translation_key(&self) -> &'static str {
        match self {
            InjuryType::Bruise => "injury_type.bruise",
            InjuryType::Strain => "injury_type.strain",
            InjuryType::Fracture => "injury_type.fracture",
            InjuryType::QiDeviation => "injury_type.qi_deviation",
        }
    }
}

// ---------------------------------------------------------------------------
// InjurySeverity
// ---------------------------------------------------------------------------

/// 부상 심각도.
///
/// 심각도에 따라 자연 치유 기간이 달라진다.
///
/// ```
/// use wuxia_core::character::injury::InjurySeverity;
///
/// assert_eq!(InjurySeverity::Minor.heal_days(), 3);
/// assert_eq!(InjurySeverity::Major.heal_days(), 7);
/// assert_eq!(InjurySeverity::Critical.heal_days(), 15);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InjurySeverity {
    /// 경상 — 3일 자연 치유.
    Minor,
    /// 중상 — 7일 치유. 의원이나 동료 간호 필요.
    Major,
    /// 치명 — 15일+ 치유. 전문 치료 필요.
    Critical,
}

impl InjurySeverity {
    /// 자연 치유에 필요한 기본 일수.
    pub fn heal_days(&self) -> u32 {
        match self {
            InjurySeverity::Minor => 3,
            InjurySeverity::Major => 7,
            InjurySeverity::Critical => 15,
        }
    }
}

impl fmt::Display for InjurySeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Translatable for InjurySeverity {
    fn translation_key(&self) -> &'static str {
        match self {
            InjurySeverity::Minor => "injury_severity.minor",
            InjurySeverity::Major => "injury_severity.major",
            InjurySeverity::Critical => "injury_severity.critical",
        }
    }
}

// ---------------------------------------------------------------------------
// Injury (Value Object)
// ---------------------------------------------------------------------------

/// 부상 상태.
///
/// Value Object — 불변 데이터. 남은 치유 일수(remaining_days)만 변동.
/// Character가 소유하며, Option<Injury>로 부상 유무를 표현.
///
/// ```
/// use wuxia_core::character::injury::{Injury, InjuryType, InjurySeverity};
///
/// let injury = Injury::new(InjuryType::Strain, InjurySeverity::Major);
/// assert_eq!(injury.remaining_days(), 7);
/// assert!(!injury.is_healed());
///
/// let after_rest = injury.after_daily_heal();
/// assert_eq!(after_rest.remaining_days(), 6);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Injury {
    injury_type: InjuryType,
    severity: InjurySeverity,
    remaining_days: u32,
}

impl Injury {
    /// 새 부상을 생성한다. 남은 치유 일수는 심각도에 따라 결정.
    pub fn new(injury_type: InjuryType, severity: InjurySeverity) -> Self {
        Self {
            injury_type,
            severity,
            remaining_days: severity.heal_days(),
        }
    }

    pub fn injury_type(&self) -> InjuryType {
        self.injury_type
    }

    pub fn severity(&self) -> InjurySeverity {
        self.severity
    }

    pub fn remaining_days(&self) -> u32 {
        self.remaining_days
    }

    /// 부상이 완치되었는가?
    pub fn is_healed(&self) -> bool {
        self.remaining_days == 0
    }

    /// 이 부상으로 수련이 불가능한가?
    pub fn prevents_training(&self) -> bool {
        self.injury_type.prevents_training()
    }

    /// 수련 강도 페널티.
    pub fn intensity_penalty(&self) -> u32 {
        self.injury_type.intensity_penalty()
    }

    /// 하루 자연 치유 후의 부상 상태를 반환한다. (불변 → 새 인스턴스)
    pub fn after_daily_heal(&self) -> Self {
        Self {
            injury_type: self.injury_type,
            severity: self.severity,
            remaining_days: self.remaining_days.saturating_sub(1),
        }
    }

    /// 의원/동료 치료로 치유를 가속한다. (불변 → 새 인스턴스)
    pub fn after_treatment(&self, days_reduced: u32) -> Self {
        Self {
            injury_type: self.injury_type,
            severity: self.severity,
            remaining_days: self.remaining_days.saturating_sub(days_reduced),
        }
    }
}

impl fmt::Display for Injury {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}, {}일 남음)",
            self.injury_type, self.severity, self.remaining_days
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- InjuryType ---

    #[test]
    fn injury_type_intensity_penalty() {
        assert_eq!(InjuryType::Bruise.intensity_penalty(), 1);
        assert_eq!(InjuryType::Strain.intensity_penalty(), 3);
        assert_eq!(InjuryType::Fracture.intensity_penalty(), 0);
        assert_eq!(InjuryType::QiDeviation.intensity_penalty(), 0);
    }

    #[test]
    fn injury_type_prevents_training() {
        assert!(!InjuryType::Bruise.prevents_training());
        assert!(!InjuryType::Strain.prevents_training());
        assert!(InjuryType::Fracture.prevents_training());
        assert!(InjuryType::QiDeviation.prevents_training());
    }

    #[test]
    fn injury_type_all_count() {
        assert_eq!(InjuryType::all().len(), 4);
    }

    #[test]
    fn injury_type_display() {
        assert_eq!(InjuryType::QiDeviation.to_string(), "QiDeviation");
    }

    #[test]
    fn injury_type_translatable() {
        assert_eq!(InjuryType::Bruise.translation_key(), "injury_type.bruise");
        assert_eq!(InjuryType::QiDeviation.translation_key(), "injury_type.qi_deviation");
    }

    #[test]
    fn injury_type_serialization_roundtrip() {
        for injury_type in InjuryType::all() {
            let json = serde_json::to_string(injury_type).unwrap();
            let restored: InjuryType = serde_json::from_str(&json).unwrap();
            assert_eq!(*injury_type, restored);
        }
    }

    // --- InjurySeverity ---

    #[test]
    fn severity_heal_days() {
        assert_eq!(InjurySeverity::Minor.heal_days(), 3);
        assert_eq!(InjurySeverity::Major.heal_days(), 7);
        assert_eq!(InjurySeverity::Critical.heal_days(), 15);
    }

    #[test]
    fn severity_display() {
        assert_eq!(InjurySeverity::Critical.to_string(), "Critical");
    }

    #[test]
    fn severity_translatable() {
        assert_eq!(InjurySeverity::Minor.translation_key(), "injury_severity.minor");
    }

    // --- Injury ---

    #[test]
    fn new_injury_has_correct_remaining_days() {
        let injury = Injury::new(InjuryType::Strain, InjurySeverity::Major);
        assert_eq!(injury.injury_type(), InjuryType::Strain);
        assert_eq!(injury.severity(), InjurySeverity::Major);
        assert_eq!(injury.remaining_days(), 7);
        assert!(!injury.is_healed());
    }

    #[test]
    fn after_daily_heal_reduces_by_one() {
        let mut injury = Injury::new(InjuryType::Bruise, InjurySeverity::Minor);
        for _ in 0..3 {
            injury = injury.after_daily_heal();
        }
        assert!(injury.is_healed());
    }

    #[test]
    fn after_daily_heal_does_not_go_below_zero() {
        let injury = Injury::new(InjuryType::Bruise, InjurySeverity::Minor);
        let healed = injury.after_daily_heal().after_daily_heal().after_daily_heal();
        let over = healed.after_daily_heal(); // already 0
        assert_eq!(over.remaining_days(), 0);
    }

    #[test]
    fn after_treatment_accelerates_healing() {
        let injury = Injury::new(InjuryType::Fracture, InjurySeverity::Critical);
        let treated = injury.after_treatment(5);
        assert_eq!(treated.remaining_days(), 10);
    }

    #[test]
    fn after_treatment_clamps_to_zero() {
        let injury = Injury::new(InjuryType::Bruise, InjurySeverity::Minor);
        let treated = injury.after_treatment(100);
        assert!(treated.is_healed());
    }

    #[test]
    fn injury_prevents_training_delegation() {
        assert!(Injury::new(InjuryType::Fracture, InjurySeverity::Critical).prevents_training());
        assert!(!Injury::new(InjuryType::Bruise, InjurySeverity::Minor).prevents_training());
    }

    #[test]
    fn injury_intensity_penalty_delegation() {
        assert_eq!(Injury::new(InjuryType::Strain, InjurySeverity::Major).intensity_penalty(), 3);
    }

    #[test]
    fn injury_display() {
        let injury = Injury::new(InjuryType::QiDeviation, InjurySeverity::Critical);
        assert_eq!(injury.to_string(), "QiDeviation (Critical, 15일 남음)");
    }

    #[test]
    fn serialization_roundtrip() {
        let original = Injury::new(InjuryType::Strain, InjurySeverity::Major);
        let json = serde_json::to_string(&original).unwrap();
        let restored: Injury = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    // --- 무협 시나리오 ---

    #[test]
    fn scenario_bruise_heals_in_three_days() {
        let mut injury = Injury::new(InjuryType::Bruise, InjurySeverity::Minor);
        for _ in 0..3 {
            injury = injury.after_daily_heal();
        }
        assert!(injury.is_healed(), "타박상은 3일이면 낫는다");
    }

    #[test]
    fn scenario_qi_deviation_long_recovery() {
        let injury = Injury::new(InjuryType::QiDeviation, InjurySeverity::Critical);
        assert!(injury.prevents_training(), "주화입마에 걸리면 수련 불가");
        assert_eq!(injury.remaining_days(), 15, "치명적 부상은 15일 필요");

        let nursed = injury.after_treatment(3);
        assert_eq!(nursed.remaining_days(), 12, "동료 간호로 3일 단축");
    }
}
