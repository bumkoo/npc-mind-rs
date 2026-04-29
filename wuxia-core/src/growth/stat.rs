// wuxia-core/src/growth/stat.rs
//
// Stat Types — 무협 세계의 능력치 분류 체계
//
// 무협 소설에서 캐릭터의 힘은 세 가지 측면으로 나뉜다:
//
//   지적(Intellectual) — 내면의 힘과 지적 능력
//     내공(InnerPower): 기(氣)를 다루는 능력. 모든 무공의 기반.
//     지혜(Wisdom):     세상을 보는 통찰력. 나이가 들수록 성장.
//     책략(Strategy):   전략적 사고. 전투와 정치 모두에 영향.
//
//   육체(Physical) — 몸의 능력
//     체력(Vitality):   건강과 지구력. 노년에 쇠퇴.
//     경공(Agility):    몸놀림의 빠르기. 청년기에 최고.
//     무력(Strength):   순수한 물리적 힘.
//
//   감정(Emotional) — 마음의 힘
//     의지(Willpower):  고통과 유혹에 저항하는 힘.
//     인내(Endurance):  오랜 수련을 견디는 정신력.
//     공감(Empathy):    타인을 이해하는 능력. 관계에 영향.
//
// 능력치 범위: 0~100
//   0  = 일반인 이하 (부상, 극도의 쇠퇴)
//   10 = 일반인 평균
//   30 = 무림 입문 수준
//   50 = 중급 무인
//   70 = 일류 고수
//   90 = 절정 고수
//   100 = 전설급 (장삼봉, 독고구패 등)

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::shared::i18n::Translatable;

/// 능력치의 최대값.
pub const STAT_MAX: u32 = 100;

/// 능력치의 최소값.
pub const STAT_MIN: u32 = 0;

/// 기본 능력치 (new_default 시 사용).
pub const STAT_DEFAULT: u32 = 10;

// ---------------------------------------------------------------------------
// StatCategory — 능력치 대분류 (3개)
// ---------------------------------------------------------------------------

/// 능력치의 세 가지 범주.
///
/// 무협 세계에서 인물의 역량은 단순히 무력만이 아니다.
/// 지적 능력, 육체적 능력, 감정적 능력이 모두 중요하다.
///
/// ```
/// use wuxia_core::growth::StatCategory;
///
/// assert_eq!(StatCategory::Intellectual.stats().len(), 3);
/// assert_eq!(StatCategory::Physical.stats().len(), 3);
/// assert_eq!(StatCategory::Emotional.stats().len(), 3);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatCategory {
    /// 지적 능력: 내공, 지혜, 책략
    Intellectual,
    /// 육체적 능력: 체력, 경공, 무력
    Physical,
    /// 감정적 능력: 의지, 인내, 공감
    Emotional,
}

impl StatCategory {
    /// 이 범주에 속하는 능력치 목록을 반환한다.
    pub fn stats(&self) -> &[StatType] {
        match self {
            StatCategory::Intellectual => &[
                StatType::InnerPower,
                StatType::Wisdom,
                StatType::Strategy,
            ],
            StatCategory::Physical => &[
                StatType::Vitality,
                StatType::Agility,
                StatType::Strength,
            ],
            StatCategory::Emotional => &[
                StatType::Willpower,
                StatType::Endurance,
                StatType::Empathy,
            ],
        }
    }

    /// 모든 범주를 반환한다.
    pub fn all() -> &'static [StatCategory] {
        &[
            StatCategory::Intellectual,
            StatCategory::Physical,
            StatCategory::Emotional,
        ]
    }
}

impl fmt::Display for StatCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Translatable for StatCategory {
    fn translation_key(&self) -> &'static str {
        match self {
            StatCategory::Intellectual => "stat_category.intellectual",
            StatCategory::Physical => "stat_category.physical",
            StatCategory::Emotional => "stat_category.emotional",
        }
    }
}

// ---------------------------------------------------------------------------
// StatType — 개별 능력치 (9개)
// ---------------------------------------------------------------------------

/// 9개의 개별 능력치.
///
/// ```
/// use wuxia_core::growth::StatType;
/// use wuxia_core::shared::i18n::Translatable;
///
/// assert_eq!(StatType::all().len(), 9);
/// assert_eq!(StatType::InnerPower.translation_key(), "stat.inner_power");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatType {
    // -- 지적 (Intellectual) --
    /// 내공(內功): 기를 다루는 능력. 무공의 기반.
    InnerPower,
    /// 지혜(智慧): 통찰력. 노년에 성장.
    Wisdom,
    /// 책략(策略): 전략적 사고.
    Strategy,

    // -- 육체 (Physical) --
    /// 체력(體力): 건강과 지구력. 노년에 쇠퇴.
    Vitality,
    /// 경공(輕功): 몸놀림의 빠르기.
    Agility,
    /// 무력(武力): 물리적 힘.
    Strength,

    // -- 감정 (Emotional) --
    /// 의지(意志): 저항하는 힘.
    Willpower,
    /// 인내(忍耐): 수련을 견디는 정신력.
    Endurance,
    /// 공감(共感): 타인 이해. 관계에 영향.
    Empathy,
}

impl StatType {
    /// 전체 9개 능력치 목록.
    pub fn all() -> &'static [StatType] {
        &[
            StatType::InnerPower,
            StatType::Wisdom,
            StatType::Strategy,
            StatType::Vitality,
            StatType::Agility,
            StatType::Strength,
            StatType::Willpower,
            StatType::Endurance,
            StatType::Empathy,
        ]
    }

    /// 이 능력치가 속하는 범주.
    pub fn category(&self) -> StatCategory {
        match self {
            StatType::InnerPower | StatType::Wisdom | StatType::Strategy => {
                StatCategory::Intellectual
            }
            StatType::Vitality | StatType::Agility | StatType::Strength => {
                StatCategory::Physical
            }
            StatType::Willpower | StatType::Endurance | StatType::Empathy => {
                StatCategory::Emotional
            }
        }
    }
}

impl Translatable for StatType {
    fn translation_key(&self) -> &'static str {
        match self {
            StatType::InnerPower => "stat.inner_power",
            StatType::Wisdom => "stat.wisdom",
            StatType::Strategy => "stat.strategy",
            StatType::Vitality => "stat.vitality",
            StatType::Agility => "stat.agility",
            StatType::Strength => "stat.strength",
            StatType::Willpower => "stat.willpower",
            StatType::Endurance => "stat.endurance",
            StatType::Empathy => "stat.empathy",
        }
    }
}

impl fmt::Display for StatType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// ---------------------------------------------------------------------------
// StatBlock — 9개 능력치를 묶어서 전달하는 Value Object
// ---------------------------------------------------------------------------

/// 9개 능력치를 이름 붙여서 묶는 구조체.
///
/// 외부 파일(JSON/TOML)에서 로드하거나 GrowthProfile을 생성할 때 사용한다.
/// Serde를 derive하므로 파일 ↔ 코드 변환이 자동이다.
///
/// 각 값은 0~100 범위를 권장하지만, StatBlock 자체는 범위를 강제하지 않는다.
/// 범위 강제는 GrowthProfile이 담당한다 (clamp).
///
/// # Example
/// ```
/// use wuxia_core::growth::StatBlock;
///
/// // 열혈 청년 무인 프리셋
/// let young_warrior = StatBlock {
///     inner_power: 20,
///     wisdom: 15,
///     strategy: 10,
///     vitality: 60,
///     agility: 50,
///     strength: 55,
///     willpower: 40,
///     endurance: 35,
///     empathy: 25,
/// };
///
/// assert_eq!(young_warrior.vitality, 60);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatBlock {
    // -- 지적 --
    pub inner_power: u32,
    pub wisdom: u32,
    pub strategy: u32,
    // -- 육체 --
    pub vitality: u32,
    pub agility: u32,
    pub strength: u32,
    // -- 감정 --
    pub willpower: u32,
    pub endurance: u32,
    pub empathy: u32,
}

impl StatBlock {
    /// 모든 능력치가 기본값(10)인 StatBlock을 생성한다.
    pub fn default_stats() -> Self {
        Self {
            inner_power: STAT_DEFAULT,
            wisdom: STAT_DEFAULT,
            strategy: STAT_DEFAULT,
            vitality: STAT_DEFAULT,
            agility: STAT_DEFAULT,
            strength: STAT_DEFAULT,
            willpower: STAT_DEFAULT,
            endurance: STAT_DEFAULT,
            empathy: STAT_DEFAULT,
        }
    }

    /// 특정 StatType의 값을 조회한다.
    pub fn get(&self, stat: StatType) -> u32 {
        match stat {
            StatType::InnerPower => self.inner_power,
            StatType::Wisdom => self.wisdom,
            StatType::Strategy => self.strategy,
            StatType::Vitality => self.vitality,
            StatType::Agility => self.agility,
            StatType::Strength => self.strength,
            StatType::Willpower => self.willpower,
            StatType::Endurance => self.endurance,
            StatType::Empathy => self.empathy,
        }
    }

    /// 특정 StatType의 값을 설정한다.
    pub fn set(&mut self, stat: StatType, value: u32) {
        match stat {
            StatType::InnerPower => self.inner_power = value,
            StatType::Wisdom => self.wisdom = value,
            StatType::Strategy => self.strategy = value,
            StatType::Vitality => self.vitality = value,
            StatType::Agility => self.agility = value,
            StatType::Strength => self.strength = value,
            StatType::Willpower => self.willpower = value,
            StatType::Endurance => self.endurance = value,
            StatType::Empathy => self.empathy = value,
        }
    }

    /// 9개 능력치의 합계.
    pub fn total(&self) -> u32 {
        self.inner_power
            + self.wisdom
            + self.strategy
            + self.vitality
            + self.agility
            + self.strength
            + self.willpower
            + self.endurance
            + self.empathy
    }
}

impl Default for StatBlock {
    fn default() -> Self {
        Self::default_stats()
    }
}

// ---------------------------------------------------------------------------
// 헬퍼: clamp 함수
// ---------------------------------------------------------------------------

/// 능력치를 0~100 범위로 제한한다.
pub fn clamp_stat(value: u32) -> u32 {
    value.min(STAT_MAX)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- StatCategory --

    #[test]
    fn category_has_three_stats_each() {
        for cat in StatCategory::all() {
            assert_eq!(cat.stats().len(), 3, "{} should have 3 stats", cat);
        }
    }

    #[test]
    fn all_categories() {
        assert_eq!(StatCategory::all().len(), 3);
    }

    #[test]
    fn category_display() {
        assert_eq!(StatCategory::Intellectual.to_string(), "Intellectual");
        assert_eq!(StatCategory::Physical.to_string(), "Physical");
        assert_eq!(StatCategory::Emotional.to_string(), "Emotional");
    }

    // -- StatType --

    #[test]
    fn all_stats_count_nine() {
        assert_eq!(StatType::all().len(), 9);
    }

    #[test]
    fn stat_category_mapping() {
        // 지적
        assert_eq!(StatType::InnerPower.category(), StatCategory::Intellectual);
        assert_eq!(StatType::Wisdom.category(), StatCategory::Intellectual);
        assert_eq!(StatType::Strategy.category(), StatCategory::Intellectual);
        // 육체
        assert_eq!(StatType::Vitality.category(), StatCategory::Physical);
        assert_eq!(StatType::Agility.category(), StatCategory::Physical);
        assert_eq!(StatType::Strength.category(), StatCategory::Physical);
        // 감정
        assert_eq!(StatType::Willpower.category(), StatCategory::Emotional);
        assert_eq!(StatType::Endurance.category(), StatCategory::Emotional);
        assert_eq!(StatType::Empathy.category(), StatCategory::Emotional);
    }

    #[test]
    fn stat_translation_keys() {
        use crate::shared::i18n::Translatable;
        assert_eq!(StatType::InnerPower.translation_key(), "stat.inner_power");
        assert_eq!(StatType::Wisdom.translation_key(), "stat.wisdom");
        assert_eq!(StatType::Strategy.translation_key(), "stat.strategy");
        assert_eq!(StatType::Vitality.translation_key(), "stat.vitality");
        assert_eq!(StatType::Agility.translation_key(), "stat.agility");
        assert_eq!(StatType::Strength.translation_key(), "stat.strength");
        assert_eq!(StatType::Willpower.translation_key(), "stat.willpower");
        assert_eq!(StatType::Endurance.translation_key(), "stat.endurance");
        assert_eq!(StatType::Empathy.translation_key(), "stat.empathy");
    }

    #[test]
    fn stat_category_translation_keys() {
        use crate::shared::i18n::Translatable;
        assert_eq!(StatCategory::Intellectual.translation_key(), "stat_category.intellectual");
        assert_eq!(StatCategory::Physical.translation_key(), "stat_category.physical");
        assert_eq!(StatCategory::Emotional.translation_key(), "stat_category.emotional");
    }

    #[test]
    fn stat_display_uses_debug_name() {
        assert_eq!(StatType::InnerPower.to_string(), "InnerPower");
    }

    #[test]
    fn every_stat_belongs_to_its_category() {
        // StatCategory.stats()와 StatType.category()가 일관적인지 검증
        for cat in StatCategory::all() {
            for stat in cat.stats() {
                assert_eq!(
                    stat.category(),
                    *cat,
                    "{} should belong to {}",
                    stat,
                    cat
                );
            }
        }
    }

    // -- StatBlock --

    #[test]
    fn default_stats_all_ten() {
        let block = StatBlock::default_stats();
        for stat in StatType::all() {
            assert_eq!(block.get(*stat), 10, "{} should be 10", stat);
        }
    }

    #[test]
    fn default_trait() {
        let block = StatBlock::default();
        assert_eq!(block.total(), 90); // 10 × 9
    }

    #[test]
    fn custom_stat_block() {
        let block = StatBlock {
            inner_power: 50,
            wisdom: 30,
            strategy: 40,
            vitality: 80,
            agility: 60,
            strength: 70,
            willpower: 45,
            endurance: 55,
            empathy: 35,
        };
        assert_eq!(block.get(StatType::InnerPower), 50);
        assert_eq!(block.get(StatType::Vitality), 80);
        assert_eq!(block.total(), 465);
    }

    #[test]
    fn stat_block_get_set() {
        let mut block = StatBlock::default_stats();
        block.set(StatType::InnerPower, 99);
        assert_eq!(block.get(StatType::InnerPower), 99);
        // 다른 능력치는 변하지 않음
        assert_eq!(block.get(StatType::Wisdom), 10);
    }

    #[test]
    fn stat_block_serialization() {
        let original = StatBlock {
            inner_power: 50,
            wisdom: 30,
            strategy: 40,
            vitality: 80,
            agility: 60,
            strength: 70,
            willpower: 45,
            endurance: 55,
            empathy: 35,
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: StatBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn stat_block_json_has_named_fields() {
        // 외부 파일에서 필드명으로 접근 가능한지 확인
        let json = r#"{
            "inner_power": 50,
            "wisdom": 30,
            "strategy": 40,
            "vitality": 80,
            "agility": 60,
            "strength": 70,
            "willpower": 45,
            "endurance": 55,
            "empathy": 35
        }"#;
        let block: StatBlock = serde_json::from_str(json).unwrap();
        assert_eq!(block.inner_power, 50);
        assert_eq!(block.empathy, 35);
    }

    // -- clamp --

    #[test]
    fn clamp_within_range() {
        assert_eq!(clamp_stat(50), 50);
        assert_eq!(clamp_stat(0), 0);
        assert_eq!(clamp_stat(100), 100);
    }

    #[test]
    fn clamp_over_max() {
        assert_eq!(clamp_stat(150), 100);
        assert_eq!(clamp_stat(u32::MAX), 100);
    }
}
