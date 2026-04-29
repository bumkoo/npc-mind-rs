// wuxia-core/src/character/fatigue.rs
//
// Fatigue Level — 피로 단계.
//
// 무협 세계에서 수련과 전투는 무조건 피로를 유발한다.
// 피로가 쌓이면 수련 효율이 떨어지고, 극에 달하면 부상으로 이어진다.
//
// 피로 단계 (5단계):
//   양호(Fresh)    0~20:  정상 상태
//   경미(Mild)    21~40:  약간의 피로
//   보통(Moderate) 41~60: 효율 저하 시작
//   심각(Severe)  61~80:  부상 위험 증가
//   탈진(Exhausted) 81~100: 강제 휴식 필요, 수련 불가

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::shared::i18n::Translatable;

/// 피로도 최소/최대값.
pub const FATIGUE_MIN: u32 = 0;
pub const FATIGUE_MAX: u32 = 100;

/// 하루 수면 회복량 (밤 시간대).
pub const DAILY_REST_RECOVERY: u32 = 5;

/// 피로 단계.
///
/// 피로 수치(0~100)에서 자동으로 결정된다.
///
/// ```
/// use wuxia_core::character::FatigueLevel;
///
/// assert_eq!(FatigueLevel::from_fatigue(0), FatigueLevel::Fresh);
/// assert_eq!(FatigueLevel::from_fatigue(20), FatigueLevel::Fresh);
/// assert_eq!(FatigueLevel::from_fatigue(21), FatigueLevel::Mild);
/// assert_eq!(FatigueLevel::from_fatigue(100), FatigueLevel::Exhausted);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FatigueLevel {
    /// 양호 (0~20): 정상 상태. 수련 효율 100%.
    Fresh,
    /// 경미 (21~40): 약간의 피로. 큰 영향 없음.
    Mild,
    /// 보통 (41~60): 효율 저하 시작. 주의 필요.
    Moderate,
    /// 심각 (61~80): 부상 위험 증가. 휴식 권장.
    Severe,
    /// 탈진 (81~100): 강제 휴식 필요. 수련 불가.
    Exhausted,
}

impl FatigueLevel {
    /// 피로 수치에서 피로 단계를 결정한다.
    pub fn from_fatigue(fatigue: u32) -> Self {
        match fatigue {
            0..=20 => FatigueLevel::Fresh,
            21..=40 => FatigueLevel::Mild,
            41..=60 => FatigueLevel::Moderate,
            61..=80 => FatigueLevel::Severe,
            _ => FatigueLevel::Exhausted, // 81~100+
        }
    }
}

impl fmt::Display for FatigueLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Translatable for FatigueLevel {
    fn translation_key(&self) -> &'static str {
        match self {
            FatigueLevel::Fresh => "fatigue_level.fresh",
            FatigueLevel::Mild => "fatigue_level.mild",
            FatigueLevel::Moderate => "fatigue_level.moderate",
            FatigueLevel::Severe => "fatigue_level.severe",
            FatigueLevel::Exhausted => "fatigue_level.exhausted",
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
    fn from_fatigue_fresh() {
        assert_eq!(FatigueLevel::from_fatigue(0), FatigueLevel::Fresh);
        assert_eq!(FatigueLevel::from_fatigue(10), FatigueLevel::Fresh);
        assert_eq!(FatigueLevel::from_fatigue(20), FatigueLevel::Fresh);
    }

    #[test]
    fn from_fatigue_mild() {
        assert_eq!(FatigueLevel::from_fatigue(21), FatigueLevel::Mild);
        assert_eq!(FatigueLevel::from_fatigue(30), FatigueLevel::Mild);
        assert_eq!(FatigueLevel::from_fatigue(40), FatigueLevel::Mild);
    }

    #[test]
    fn from_fatigue_moderate() {
        assert_eq!(FatigueLevel::from_fatigue(41), FatigueLevel::Moderate);
        assert_eq!(FatigueLevel::from_fatigue(50), FatigueLevel::Moderate);
        assert_eq!(FatigueLevel::from_fatigue(60), FatigueLevel::Moderate);
    }

    #[test]
    fn from_fatigue_severe() {
        assert_eq!(FatigueLevel::from_fatigue(61), FatigueLevel::Severe);
        assert_eq!(FatigueLevel::from_fatigue(70), FatigueLevel::Severe);
        assert_eq!(FatigueLevel::from_fatigue(80), FatigueLevel::Severe);
    }

    #[test]
    fn from_fatigue_exhausted() {
        assert_eq!(FatigueLevel::from_fatigue(81), FatigueLevel::Exhausted);
        assert_eq!(FatigueLevel::from_fatigue(90), FatigueLevel::Exhausted);
        assert_eq!(FatigueLevel::from_fatigue(100), FatigueLevel::Exhausted);
    }

    #[test]
    fn from_fatigue_over_max_is_exhausted() {
        assert_eq!(FatigueLevel::from_fatigue(150), FatigueLevel::Exhausted);
    }

    #[test]
    fn display() {
        assert_eq!(FatigueLevel::Fresh.to_string(), "Fresh");
        assert_eq!(FatigueLevel::Exhausted.to_string(), "Exhausted");
    }

    #[test]
    fn translatable() {
        assert_eq!(FatigueLevel::Fresh.translation_key(), "fatigue_level.fresh");
        assert_eq!(
            FatigueLevel::Exhausted.translation_key(),
            "fatigue_level.exhausted"
        );
    }

    #[test]
    fn serialization_roundtrip() {
        let levels = [
            FatigueLevel::Fresh,
            FatigueLevel::Mild,
            FatigueLevel::Moderate,
            FatigueLevel::Severe,
            FatigueLevel::Exhausted,
        ];
        for level in levels {
            let json = serde_json::to_string(&level).unwrap();
            let restored: FatigueLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(level, restored);
        }
    }
}
