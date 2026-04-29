// wuxia-core/src/growth/event.rs
//
// Growth Events — 성장 도메인에서 발생하는 이벤트들
//
// 능력치 변화가 발생할 때마다 이벤트를 생성한다.
// Application Service가 이 이벤트를 받아서 다른 도메인에 전파한다.
//
// 예: 단련으로 무력이 올라감 → GrowthEvent::StatTrained → 전투력 재계산 트리거
// 예: 연마로 독고구검 숙련도 상승 → GrowthEvent::ArtPracticed → 경지 돌파 연출
// 예: 노화로 체력이 줄어듦 → GrowthEvent::YearlyAgingApplied → 부상 위험 증가
//
// StatChange는 "무엇이 얼마나 왜 변했는지" 기록하는 Value Object.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::character::LifeStage;
use crate::shared::id::{CharacterId, MartialArtId};

use super::martial_art::MasteryLevel;
use super::stat::StatType;

// ---------------------------------------------------------------------------
// ChangeSource — 변화의 원인
// ---------------------------------------------------------------------------

/// 능력치가 변한 원인.
///
/// 향후 확장 가능: 부상(Injury), 비급습득(SecretManual), 약물(Medicine) 등.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeSource {
    /// 단련(鍛鍊)으로 인한 능력치 성장
    StatTraining,
    /// 연마(鍊磨)의 부산물로 인한 능력치 성장
    ArtPractice,
    /// 연간 노화에 의한 자연 변화 (성장 또는 쇠퇴)
    YearlyAging,
}

impl fmt::Display for ChangeSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// ---------------------------------------------------------------------------
// StatChange — 능력치 변화 한 건
// ---------------------------------------------------------------------------

/// 능력치 변화 한 건을 기록하는 Value Object.
///
/// 양수(+) = 성장, 음수(-) = 쇠퇴.
/// 실제 능력치(u32)에 적용할 때는 `GrowthProfile::apply_stat_change`가
/// 0~100 범위를 보장한다.
///
/// # Example
/// ```
/// use wuxia_core::growth::{StatChange, StatType, ChangeSource};
///
/// let change = StatChange::new(StatType::Vitality, -2, ChangeSource::YearlyAging);
/// assert_eq!(change.stat(), StatType::Vitality);
/// assert_eq!(change.delta(), -2);
/// assert!(change.is_decline());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatChange {
    stat: StatType,
    delta: i32,
    source: ChangeSource,
}

impl StatChange {
    /// 새 StatChange 생성.
    pub fn new(stat: StatType, delta: i32, source: ChangeSource) -> Self {
        Self {
            stat,
            delta,
            source,
        }
    }

    /// 변화된 능력치.
    pub fn stat(&self) -> StatType {
        self.stat
    }

    /// 변화량. 양수 = 성장, 음수 = 쇠퇴.
    pub fn delta(&self) -> i32 {
        self.delta
    }

    /// 변화의 원인.
    pub fn source(&self) -> ChangeSource {
        self.source
    }

    /// 성장인가? (delta > 0)
    pub fn is_growth(&self) -> bool {
        self.delta > 0
    }

    /// 쇠퇴인가? (delta < 0)
    pub fn is_decline(&self) -> bool {
        self.delta < 0
    }

    /// 변화 없음? (delta == 0)
    pub fn is_unchanged(&self) -> bool {
        self.delta == 0
    }
}

impl fmt::Display for StatChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.delta >= 0 { "+" } else { "" };
        write!(
            f,
            "{} {}{} ({})",
            self.stat, sign, self.delta, self.source
        )
    }
}

// ---------------------------------------------------------------------------
// GrowthEvent — 성장 도메인 이벤트
// ---------------------------------------------------------------------------

/// 성장 도메인에서 발생하는 이벤트.
///
/// DomainEvent::Growth(GrowthEvent)로 래핑되어 Application Service에 전달된다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GrowthEvent {
    /// 단련(鍛鍊)으로 능력치가 변했다.
    ///
    /// 예: 화산파에서 3개월간 체력 단련 → 체력 +5
    StatTrained {
        character_id: CharacterId,
        changes: Vec<StatChange>,
    },

    /// 연간 노화로 능력치가 자동 변동되었다.
    ///
    /// 예: 노년기 진입 → 체력 -2, 지혜 +1
    YearlyAgingApplied {
        character_id: CharacterId,
        life_stage: LifeStage,
        changes: Vec<StatChange>,
    },

    /// 무공 연마(鍊磨)로 숙련도가 올랐다.
    ///
    /// 부산물로 관련 능력치도 소폭 상승한다.
    /// old_mastery ≠ new_mastery이면 경지 돌파가 발생한 것이다.
    ///
    /// 예: 독고구검 연마 → 숙련도 45→48, 무력+1 체력+1
    ArtPracticed {
        character_id: CharacterId,
        martial_art_id: MartialArtId,
        proficiency_gain: u32,
        new_proficiency: u32,
        old_mastery: MasteryLevel,
        new_mastery: MasteryLevel,
        stat_changes: Vec<StatChange>,
    },
}

use crate::shared::event_macros::impl_event_name;

impl_event_name!(GrowthEvent {
    StatTrained => "GrowthStatTrained",
    YearlyAgingApplied => "GrowthYearlyAgingApplied",
    ArtPracticed => "GrowthArtPracticed",
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- StatChange --

    #[test]
    fn stat_change_growth() {
        let change = StatChange::new(StatType::Strength, 5, ChangeSource::StatTraining);
        assert_eq!(change.stat(), StatType::Strength);
        assert_eq!(change.delta(), 5);
        assert_eq!(change.source(), ChangeSource::StatTraining);
        assert!(change.is_growth());
        assert!(!change.is_decline());
        assert!(!change.is_unchanged());
    }

    #[test]
    fn stat_change_decline() {
        let change = StatChange::new(StatType::Vitality, -2, ChangeSource::YearlyAging);
        assert!(change.is_decline());
        assert!(!change.is_growth());
    }

    #[test]
    fn stat_change_zero() {
        let change = StatChange::new(StatType::Wisdom, 0, ChangeSource::YearlyAging);
        assert!(change.is_unchanged());
        assert!(!change.is_growth());
        assert!(!change.is_decline());
    }

    #[test]
    fn stat_change_display_positive() {
        let change = StatChange::new(StatType::Strength, 5, ChangeSource::StatTraining);
        assert_eq!(change.to_string(), "Strength +5 (StatTraining)");
    }

    #[test]
    fn stat_change_display_negative() {
        let change = StatChange::new(StatType::Vitality, -2, ChangeSource::YearlyAging);
        assert_eq!(change.to_string(), "Vitality -2 (YearlyAging)");
    }

    #[test]
    fn stat_change_serialization_roundtrip() {
        let original = StatChange::new(StatType::Agility, 3, ChangeSource::StatTraining);
        let json = serde_json::to_string(&original).unwrap();
        let restored: StatChange = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    // -- GrowthEvent --

    #[test]
    fn growth_event_stat_trained() {
        let event = GrowthEvent::StatTrained {
            character_id: CharacterId::new(1),
            changes: vec![
                StatChange::new(StatType::Strength, 3, ChangeSource::StatTraining),
            ],
        };
        assert_eq!(event.name(), "GrowthStatTrained");
    }

    #[test]
    fn growth_event_yearly_aging() {
        let event = GrowthEvent::YearlyAgingApplied {
            character_id: CharacterId::new(1),
            life_stage: LifeStage::Elder,
            changes: vec![
                StatChange::new(StatType::Vitality, -2, ChangeSource::YearlyAging),
                StatChange::new(StatType::Wisdom, 1, ChangeSource::YearlyAging),
            ],
        };
        assert_eq!(event.name(), "GrowthYearlyAgingApplied");
    }

    #[test]
    fn growth_event_serialization_roundtrip() {
        let event = GrowthEvent::StatTrained {
            character_id: CharacterId::new(42),
            changes: vec![
                StatChange::new(StatType::InnerPower, 5, ChangeSource::StatTraining),
                StatChange::new(StatType::Willpower, 2, ChangeSource::StatTraining),
            ],
        };
        let json = serde_json::to_string(&event).unwrap();
        let restored: GrowthEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, restored);
    }

    // -- ChangeSource --

    #[test]
    fn change_source_display() {
        assert_eq!(ChangeSource::StatTraining.to_string(), "StatTraining");
        assert_eq!(ChangeSource::ArtPractice.to_string(), "ArtPractice");
        assert_eq!(ChangeSource::YearlyAging.to_string(), "YearlyAging");
    }

    // -- ArtPracticed --

    #[test]
    fn growth_event_art_practiced() {
        use crate::shared::MartialArtId;

        let event = GrowthEvent::ArtPracticed {
            character_id: CharacterId::new(1),
            martial_art_id: MartialArtId::new(1),
            proficiency_gain: 3,
            new_proficiency: 48,
            old_mastery: MasteryLevel::Proficient,
            new_mastery: MasteryLevel::Proficient,
            stat_changes: vec![
                StatChange::new(StatType::Strength, 1, ChangeSource::ArtPractice),
            ],
        };
        assert_eq!(event.name(), "GrowthArtPracticed");
    }

    #[test]
    fn growth_event_art_practiced_serialization() {
        use crate::shared::MartialArtId;

        let event = GrowthEvent::ArtPracticed {
            character_id: CharacterId::new(1),
            martial_art_id: MartialArtId::new(5),
            proficiency_gain: 5,
            new_proficiency: 90,
            old_mastery: MasteryLevel::Mastered,
            new_mastery: MasteryLevel::Transcendent,
            stat_changes: vec![
                StatChange::new(StatType::InnerPower, 1, ChangeSource::ArtPractice),
                StatChange::new(StatType::Willpower, 1, ChangeSource::ArtPractice),
            ],
        };
        let json = serde_json::to_string(&event).unwrap();
        let restored: GrowthEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, restored);
    }
}
