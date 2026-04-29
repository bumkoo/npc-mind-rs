// wuxia-core/src/growth/power.rs
//
// Power Calculation Rules — 전투력 계산 규칙
//
// training.rs와 동일한 패턴: 순수 함수로 전투력 규칙을 분리한다.
// GrowthProfile(aggregate)에서 이 규칙을 호출하여 전투력을 계산한다.
//
// 규칙과 데이터(aggregate)를 분리하는 이유:
//   - 전투력 공식을 바꿀 때 aggregate를 건드리지 않아도 됨
//   - 규칙 함수가 순수 함수 → 테스트가 간단
//   - 향후 전투 도메인에서도 재사용 가능
//
// ┌──────────────────────────────────────────────────────────┐
// │  기초 전투력 (Combat Power)                              │
// │                                                          │
// │  combat_power = 무력(Strength) + 경공(Agility) + 내공    │
// │                                                          │
// │  맨몸으로 "얼마나 강한가"의 지표.                         │
// │  Phase 4(전투 도메인)에서 실전위력과 통합될 예정.         │
// └──────────────────────────────────────────────────────────┘

use crate::shared::id::MartialArtId;

use super::martial_art::{MartialArtProficiency, MartialArtType};
use super::stat::{StatBlock, StatType};
use super::training;

/// 기초 전투력을 계산한다: 무력 + 경공 + 내공.
///
/// 순수 능력치 기반의 전투력. 무공 없이 "맨몸으로 얼마나 강한가".
///
/// # Example
/// ```
/// use wuxia_core::growth::power;
/// use wuxia_core::growth::StatBlock;
///
/// let stats = StatBlock {
///     inner_power: 50, wisdom: 30, strategy: 40,
///     vitality: 80, agility: 60, strength: 70,
///     willpower: 45, endurance: 55, empathy: 35,
/// };
/// // 무력(70) + 경공(60) + 내공(50) = 180
/// assert_eq!(power::calculate_combat_power(&stats), 180);
/// ```
pub fn calculate_combat_power(stats: &StatBlock) -> u32 {
    stats.get(StatType::Strength)
        + stats.get(StatType::Agility)
        + stats.get(StatType::InnerPower)
}

/// 무공 하나의 실전위력을 계산한다.
///
/// `base_power`: 무공 정의의 기본 위력.
/// `proficiency`: 해당 무공의 숙련도 (0~100).
/// `art_type`: 무공 유형 (관련 능력치를 결정).
/// `stats`: 캐릭터의 능력치 블록.
///
/// 내부적으로 `training::calculate_effective_power`를 호출한다.
pub fn calculate_art_effective_power(
    stats: &StatBlock,
    proficiency: u32,
    base_power: u32,
    art_type: MartialArtType,
) -> u32 {
    let related = art_type.related_stats();
    let stat_sum: u32 = related.iter().map(|s| stats.get(*s)).sum();
    let stat_avg = stat_sum / related.len() as u32;

    training::calculate_effective_power(base_power, proficiency, stat_avg)
}

/// 습득한 무공 중 가장 강한 실전위력을 반환한다.
///
/// `learned_arts`: 습득한 무공 숙련도 목록.
/// `arts`: (MartialArtId, base_power, MartialArtType) 튜플 슬라이스.
/// `stats`: 캐릭터의 능력치 블록.
///
/// 무공이 없거나 모두 미습득이면 0.
pub fn calculate_best_art_power(
    stats: &StatBlock,
    learned_arts: &[MartialArtProficiency],
    arts: &[(MartialArtId, u32, MartialArtType)],
) -> u32 {
    arts.iter()
        .map(|(id, base_power, art_type)| {
            let proficiency = learned_arts
                .iter()
                .find(|p| p.martial_art_id() == *id)
                .map(|p| p.proficiency())
                .unwrap_or(0);

            if proficiency == 0 {
                return 0;
            }

            calculate_art_effective_power(stats, proficiency, *base_power, *art_type)
        })
        .max()
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_stats() -> StatBlock {
        StatBlock::default_stats()
    }

    fn warrior_stats() -> StatBlock {
        StatBlock {
            inner_power: 50,
            wisdom: 30,
            strategy: 40,
            vitality: 80,
            agility: 60,
            strength: 70,
            willpower: 45,
            endurance: 55,
            empathy: 35,
        }
    }

    // --- 기초 전투력 ---

    #[test]
    fn combat_power_default_stats() {
        let stats = default_stats();
        // 10 + 10 + 10 = 30
        assert_eq!(calculate_combat_power(&stats), 30);
    }

    #[test]
    fn combat_power_warrior_stats() {
        let stats = warrior_stats();
        // 무력(70) + 경공(60) + 내공(50) = 180
        assert_eq!(calculate_combat_power(&stats), 180);
    }

    #[test]
    fn combat_power_zero_stats() {
        let stats = StatBlock {
            inner_power: 0,
            wisdom: 0,
            strategy: 0,
            vitality: 0,
            agility: 0,
            strength: 0,
            willpower: 0,
            endurance: 0,
            empathy: 0,
        };
        assert_eq!(calculate_combat_power(&stats), 0);
    }

    #[test]
    fn combat_power_max_stats() {
        let stats = StatBlock {
            inner_power: 100,
            wisdom: 100,
            strategy: 100,
            vitality: 100,
            agility: 100,
            strength: 100,
            willpower: 100,
            endurance: 100,
            empathy: 100,
        };
        assert_eq!(calculate_combat_power(&stats), 300);
    }

    // --- 실전위력 ---

    #[test]
    fn art_effective_power_zero_proficiency() {
        let stats = warrior_stats();
        assert_eq!(
            calculate_art_effective_power(&stats, 0, 85, MartialArtType::WeaponArt),
            0
        );
    }

    #[test]
    fn art_effective_power_positive() {
        let stats = warrior_stats();
        let power = calculate_art_effective_power(&stats, 50, 85, MartialArtType::WeaponArt);
        assert!(power > 0);
    }

    // --- 최강 무공 ---

    #[test]
    fn best_art_power_empty_arts() {
        let stats = warrior_stats();
        assert_eq!(calculate_best_art_power(&stats, &[], &[]), 0);
    }

    #[test]
    fn best_art_power_no_learned_arts() {
        let stats = warrior_stats();
        let arts = [(MartialArtId::new(1), 85, MartialArtType::WeaponArt)];
        assert_eq!(calculate_best_art_power(&stats, &[], &arts), 0);
    }

    #[test]
    fn best_art_power_with_learned_art() {
        let stats = warrior_stats();
        let mut prof = MartialArtProficiency::new(MartialArtId::new(1));
        prof.add_proficiency(50);
        let learned = vec![prof];
        let arts = [(MartialArtId::new(1), 85, MartialArtType::WeaponArt)];
        let power = calculate_best_art_power(&stats, &learned, &arts);
        assert!(power > 0);
    }
}
