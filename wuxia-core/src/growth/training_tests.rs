use super::*;

// ===================================================================
// growth_multiplier
// ===================================================================

#[test]
fn multiplier_youth_fastest() {
    assert!(growth_multiplier(LifeStage::Youth) > growth_multiplier(LifeStage::Prime));
}

#[test]
fn multiplier_elder_slowest() {
    assert!(growth_multiplier(LifeStage::Elder) < growth_multiplier(LifeStage::Middle));
}

#[test]
fn multiplier_all_positive() {
    for stage in [LifeStage::Youth, LifeStage::Prime, LifeStage::Middle, LifeStage::Elder] {
        assert!(growth_multiplier(stage) > 0.0, "{:?} should have positive multiplier", stage);
    }
}

#[test]
fn multiplier_exact_values() {
    assert_eq!(growth_multiplier(LifeStage::Youth), 1.5);
    assert_eq!(growth_multiplier(LifeStage::Prime), 1.0);
    assert_eq!(growth_multiplier(LifeStage::Middle), 0.7);
    assert_eq!(growth_multiplier(LifeStage::Elder), 0.3);
}

// ===================================================================
// calculate_stat_training
// ===================================================================

#[test]
fn training_youth_gets_bonus() {
    // 강도 4 × 1.5 = 6
    let change = calculate_stat_training(StatType::Strength, 4, LifeStage::Youth);
    assert_eq!(change.stat(), StatType::Strength);
    assert_eq!(change.delta(), 6);
    assert_eq!(change.source(), ChangeSource::StatTraining);
}

#[test]
fn training_prime_baseline() {
    // 강도 4 × 1.0 = 4
    let change = calculate_stat_training(StatType::Strength, 4, LifeStage::Prime);
    assert_eq!(change.delta(), 4);
}

#[test]
fn training_middle_reduced() {
    // 강도 4 × 0.7 = 2.8 → round → 3
    let change = calculate_stat_training(StatType::Strength, 4, LifeStage::Middle);
    assert_eq!(change.delta(), 3);
}

#[test]
fn training_elder_minimum_one() {
    // 강도 1 × 0.3 = 0.3 → round → 0 → max(1) = 1
    let change = calculate_stat_training(StatType::Strength, 1, LifeStage::Elder);
    assert_eq!(change.delta(), 1, "노인도 수련하면 최소 1 성장");
}

#[test]
fn training_zero_intensity_no_growth() {
    let change = calculate_stat_training(StatType::Strength, 0, LifeStage::Youth);
    assert_eq!(change.delta(), 0, "강도 0이면 성장 없음");
}

#[test]
fn training_same_intensity_youth_beats_elder() {
    let youth = calculate_stat_training(StatType::InnerPower, 5, LifeStage::Youth);
    let elder = calculate_stat_training(StatType::InnerPower, 5, LifeStage::Elder);
    assert!(youth.delta() > elder.delta(),
        "같은 수련량, 청년({}) > 노인({})", youth.delta(), elder.delta());
}

#[test]
fn training_all_stages_any_stat() {
    // 모든 조합에서 panic 없이 동작하는지 확인
    for stage in [LifeStage::Youth, LifeStage::Prime, LifeStage::Middle, LifeStage::Elder] {
        for stat in StatType::all() {
            let change = calculate_stat_training(*stat, 3, stage);
            assert!(change.delta() >= 1, "intensity 3, {:?}/{:?} should grow", stage, stat);
        }
    }
}

#[test]
fn training_large_intensity() {
    // 강도 10 × 1.5 = 15
    let change = calculate_stat_training(StatType::Strength, 10, LifeStage::Youth);
    assert_eq!(change.delta(), 15);
}

// ===================================================================
// calculate_yearly_aging
// ===================================================================

#[test]
fn yearly_aging_youth_physical_growth() {
    let changes = calculate_yearly_aging(LifeStage::Youth);

    let vitality = find_change(&changes, StatType::Vitality);
    let agility = find_change(&changes, StatType::Agility);
    let strength = find_change(&changes, StatType::Strength);

    assert_eq!(vitality.delta(), 1, "청년: 체력 +1");
    assert_eq!(agility.delta(), 1, "청년: 경공 +1");
    assert_eq!(strength.delta(), 1, "청년: 무력 +1");
}

#[test]
fn yearly_aging_youth_no_intellectual_change() {
    let changes = calculate_yearly_aging(LifeStage::Youth);
    assert!(find_change_opt(&changes, StatType::Wisdom).is_none(), "청년: 지혜 변동 없음");
    assert!(find_change_opt(&changes, StatType::Strategy).is_none(), "청년: 책략 변동 없음");
}

#[test]
fn yearly_aging_prime_intellectual_growth() {
    let changes = calculate_yearly_aging(LifeStage::Prime);

    assert_eq!(find_change(&changes, StatType::InnerPower).delta(), 1, "장년: 내공 +1");
    assert_eq!(find_change(&changes, StatType::Wisdom).delta(), 1, "장년: 지혜 +1");
    assert_eq!(find_change(&changes, StatType::Strategy).delta(), 1, "장년: 책략 +1");
}

#[test]
fn yearly_aging_prime_no_physical_decline() {
    let changes = calculate_yearly_aging(LifeStage::Prime);
    assert!(find_change_opt(&changes, StatType::Vitality).is_none(), "장년: 체력 변동 없음");
    assert!(find_change_opt(&changes, StatType::Agility).is_none(), "장년: 경공 변동 없음");
}

#[test]
fn yearly_aging_middle_mixed() {
    let changes = calculate_yearly_aging(LifeStage::Middle);

    // 쇠퇴
    assert_eq!(find_change(&changes, StatType::Vitality).delta(), -1, "중년: 체력 -1");
    assert_eq!(find_change(&changes, StatType::Agility).delta(), -1, "중년: 경공 -1");
    // 성장
    assert_eq!(find_change(&changes, StatType::Wisdom).delta(), 2, "중년: 지혜 +2");
    assert_eq!(find_change(&changes, StatType::Willpower).delta(), 1, "중년: 의지 +1");
    assert_eq!(find_change(&changes, StatType::Empathy).delta(), 1, "중년: 공감 +1");
}

#[test]
fn yearly_aging_elder_severe_decline() {
    let changes = calculate_yearly_aging(LifeStage::Elder);

    assert_eq!(find_change(&changes, StatType::Vitality).delta(), -2, "노년: 체력 -2");
    assert_eq!(find_change(&changes, StatType::Agility).delta(), -2, "노년: 경공 -2");
    assert_eq!(find_change(&changes, StatType::Strength).delta(), -1, "노년: 무력 -1");
    assert_eq!(find_change(&changes, StatType::Endurance).delta(), -1, "노년: 인내 -1");
}

#[test]
fn yearly_aging_elder_still_gains_wisdom() {
    let changes = calculate_yearly_aging(LifeStage::Elder);
    assert_eq!(find_change(&changes, StatType::Wisdom).delta(), 1, "노년: 지혜 +1");
    assert_eq!(find_change(&changes, StatType::Strategy).delta(), 1, "노년: 책략 +1");
}

#[test]
fn yearly_aging_no_zero_deltas() {
    // 모든 단계에서 delta == 0인 항목이 없어야 함 (불필요한 이벤트 방지)
    for stage in [LifeStage::Youth, LifeStage::Prime, LifeStage::Middle, LifeStage::Elder] {
        let changes = calculate_yearly_aging(stage);
        for change in &changes {
            assert_ne!(change.delta(), 0, "{:?}: {:?} delta should not be 0", stage, change.stat());
        }
    }
}

// ===================================================================
// 무협 시나리오 테스트
// ===================================================================

#[test]
fn scenario_20_years_physical_decline() {
    // 장삼봉: 55세부터 20년간 노화 → 체력이 크게 감소해야 함
    let mut total_vitality_change: i32 = 0;

    // 55~68 = 14년 중년 (체력 -1/년)
    for _ in 0..14 {
        let changes = calculate_yearly_aging(LifeStage::Middle);
        if let Some(c) = find_change_opt(&changes, StatType::Vitality) {
            total_vitality_change += c.delta();
        }
    }
    // 69~74 = 6년 노년 (체력 -2/년)
    for _ in 0..6 {
        let changes = calculate_yearly_aging(LifeStage::Elder);
        if let Some(c) = find_change_opt(&changes, StatType::Vitality) {
            total_vitality_change += c.delta();
        }
    }

    // 14×(-1) + 6×(-2) = -14 + -12 = -26
    assert_eq!(total_vitality_change, -26, "20년간 체력 변화");
}

#[test]
fn scenario_20_years_wisdom_growth() {
    // 같은 20년 동안 지혜는 계속 성장
    let mut total_wisdom_change: i32 = 0;

    // 14년 중년 (지혜 +2/년)
    for _ in 0..14 {
        let changes = calculate_yearly_aging(LifeStage::Middle);
        if let Some(c) = find_change_opt(&changes, StatType::Wisdom) {
            total_wisdom_change += c.delta();
        }
    }
    // 6년 노년 (지혜 +1/년)
    for _ in 0..6 {
        let changes = calculate_yearly_aging(LifeStage::Elder);
        if let Some(c) = find_change_opt(&changes, StatType::Wisdom) {
            total_wisdom_change += c.delta();
        }
    }

    // 14×2 + 6×1 = 28 + 6 = 34
    assert_eq!(total_wisdom_change, 34, "20년간 지혜 변화");
}

// ===================================================================
// Fatigue from Training [v2.3A Step 3]
// ===================================================================

use crate::character::FatigueLevel;

#[test]
fn fatigue_fresh_one_to_one() {
    assert_eq!(calculate_fatigue_from_training(4, FatigueLevel::Fresh), 4);
}

#[test]
fn fatigue_mild_slight_increase() {
    // 4 × 1.2 = 4.8 → 5
    assert_eq!(calculate_fatigue_from_training(4, FatigueLevel::Mild), 5);
}

#[test]
fn fatigue_moderate_multiplier() {
    // 4 × 1.5 = 6
    assert_eq!(calculate_fatigue_from_training(4, FatigueLevel::Moderate), 6);
}

#[test]
fn fatigue_severe_double() {
    // 4 × 2.0 = 8
    assert_eq!(calculate_fatigue_from_training(4, FatigueLevel::Severe), 8);
}

#[test]
fn fatigue_zero_intensity_no_fatigue() {
    assert_eq!(calculate_fatigue_from_training(0, FatigueLevel::Fresh), 0);
}

#[test]
fn fatigue_minimum_one_when_training() {
    // 1 × 1.0 = 1
    assert_eq!(calculate_fatigue_from_training(1, FatigueLevel::Fresh), 1);
}

// ===================================================================
// Injury Chance [v2.3A Step 4]
// ===================================================================

#[test]
fn injury_chance_fresh_low_intensity_zero() {
    let chance = calculate_injury_chance(3, FatigueLevel::Fresh, false);
    assert!((chance - 0.0).abs() < f32::EPSILON, "양호+저강도 = 0%");
}

#[test]
fn injury_chance_severe_base() {
    let chance = calculate_injury_chance(3, FatigueLevel::Severe, false);
    assert!((chance - 0.15).abs() < f32::EPSILON, "심각+저강도 = 15%");
}

#[test]
fn injury_chance_high_intensity_adds() {
    // 강도 8, Fresh: base 0 + (8-5)×0.03 = 0.09
    let chance = calculate_injury_chance(8, FatigueLevel::Fresh, false);
    assert!((chance - 0.09).abs() < 0.001);
}

#[test]
fn injury_chance_over_limit_adds() {
    // 강도 3, Fresh, over_limit: base 0 + 0 + 0.10 = 0.10
    let chance = calculate_injury_chance(3, FatigueLevel::Fresh, true);
    assert!((chance - 0.10).abs() < f32::EPSILON);
}

#[test]
fn injury_chance_all_factors_combined() {
    // 강도 8, Severe, over_limit: 0.15 + 0.09 + 0.10 = 0.34
    let chance = calculate_injury_chance(8, FatigueLevel::Severe, true);
    assert!((chance - 0.34).abs() < 0.001);
}

#[test]
fn injury_chance_capped_at_80_percent() {
    // 극단적 상황: 강도 30, Severe, over_limit
    let chance = calculate_injury_chance(30, FatigueLevel::Severe, true);
    assert!((chance - 0.8).abs() < f32::EPSILON, "최대 80%로 제한");
}

// ===================================================================
// Max Intensity [v2.3B]
// ===================================================================

#[test]
fn max_intensity_healthy_youth() {
    // (60+40)/20 = 5, 의지 50 < 70 → +0, Fresh → -0 = 5
    assert_eq!(calculate_max_intensity(60, 40, 50, FatigueLevel::Fresh), Some(5));
}

#[test]
fn max_intensity_willpower_bonus() {
    // (60+40)/20 = 5, 의지 80 ≥ 70 → +1, Fresh → -0 = 6
    assert_eq!(calculate_max_intensity(60, 40, 80, FatigueLevel::Fresh), Some(6));
}

#[test]
fn max_intensity_willpower_exactly_70() {
    // 의지 70 ≥ 70 → +1
    assert_eq!(calculate_max_intensity(60, 40, 70, FatigueLevel::Fresh), Some(6));
}

#[test]
fn max_intensity_willpower_69_no_bonus() {
    // 의지 69 < 70 → +0
    assert_eq!(calculate_max_intensity(60, 40, 69, FatigueLevel::Fresh), Some(5));
}

#[test]
fn max_intensity_strong_elder() {
    // (25+20)/20 = 2, 의지 80 ≥ 70 → +1, Fresh → -0 = 3
    assert_eq!(calculate_max_intensity(25, 20, 80, FatigueLevel::Fresh), Some(3));
}

#[test]
fn max_intensity_fatigue_mild_penalty() {
    // (60+40)/20 = 5, 의지 50 < 70, Mild → -1 = 4
    assert_eq!(calculate_max_intensity(60, 40, 50, FatigueLevel::Mild), Some(4));
}

#[test]
fn max_intensity_fatigue_moderate_penalty() {
    // (60+40)/20 = 5, Moderate → -2 = 3
    assert_eq!(calculate_max_intensity(60, 40, 50, FatigueLevel::Moderate), Some(3));
}

#[test]
fn max_intensity_fatigue_severe_penalty() {
    // (60+40)/20 = 5, Severe → -3 = 2
    assert_eq!(calculate_max_intensity(60, 40, 50, FatigueLevel::Severe), Some(2));
}

#[test]
fn max_intensity_exhausted_returns_none() {
    assert_eq!(calculate_max_intensity(60, 40, 50, FatigueLevel::Exhausted), None);
}

#[test]
fn max_intensity_minimum_one() {
    // (10+10)/20 = 1, Severe → -3 → raw -2 → clamp → 1
    assert_eq!(calculate_max_intensity(10, 10, 10, FatigueLevel::Severe), Some(1));
}

#[test]
fn max_intensity_capped_at_ten() {
    // (100+100)/20 = 10, 의지 80 → +1, raw 11 → clamp → 10
    assert_eq!(calculate_max_intensity(100, 100, 80, FatigueLevel::Fresh), Some(10));
}

#[test]
fn base_max_intensity_no_will_bonus() {
    // (60+40)/20 = 5, Fresh → -0 = 5 (의지 무시)
    assert_eq!(calculate_base_max_intensity(60, 40, FatigueLevel::Fresh), Some(5));
}

#[test]
fn base_max_intensity_exhausted_returns_none() {
    assert_eq!(calculate_base_max_intensity(60, 40, FatigueLevel::Exhausted), None);
}

#[test]
fn willpower_bonus_is_over_limit_detection() {
    // TrainingService용: full - base = 의지보너스 초과분
    let full = calculate_max_intensity(60, 40, 80, FatigueLevel::Fresh).unwrap();
    let base = calculate_base_max_intensity(60, 40, FatigueLevel::Fresh).unwrap();
    assert_eq!(full - base, 1, "의지보너스 = 한계 초과 강도");
}

// --- 무협 시나리오 ---

#[test]
fn scenario_doc_examples_match() {
    // growth-mechanic-decisions-v1.md 예시 테이블 재현
    // 건강한 청년: 체력 60, 인내 40, 의지 50, 피로 0 → 5
    assert_eq!(calculate_max_intensity(60, 40, 50, FatigueLevel::Fresh), Some(5));

    // 지친 청년: 체력 60, 인내 40, 의지 50, 피로 2 (≈ Moderate)
    // 문서에서 피로=2 → FatigueLevel 아님, 감산값=2 → Moderate
    assert_eq!(calculate_max_intensity(60, 40, 50, FatigueLevel::Moderate), Some(3));

    // 의지 강한 노인: 체력 25, 인내 20, 의지 80, 피로 0 → 3
    assert_eq!(calculate_max_intensity(25, 20, 80, FatigueLevel::Fresh), Some(3));
}

#[test]
fn scenario_fatigued_warrior_reduced_training() {
    // 무인이 심각한 피로 상태에서 간신히 수련
    let fresh_max = calculate_max_intensity(70, 60, 75, FatigueLevel::Fresh).unwrap();
    let severe_max = calculate_max_intensity(70, 60, 75, FatigueLevel::Severe).unwrap();

    assert_eq!(fresh_max, 7);  // (130/20)=6 + will=1 = 7
    assert_eq!(severe_max, 4); // 6 + 1 - 3 = 4
    assert!(fresh_max > severe_max, "피로가 쌓이면 수련 강도 제한");
}

// ===================================================================
// Helpers
// ===================================================================

fn find_change_opt(changes: &[StatChange], stat: StatType) -> Option<&StatChange> {
    changes.iter().find(|c| c.stat() == stat)
}

fn find_change(changes: &[StatChange], stat: StatType) -> &StatChange {
    find_change_opt(changes, stat)
        .unwrap_or_else(|| panic!("StatChange for {:?} not found", stat))
}

// ===================================================================
// Art Training — 연마 계산
// ===================================================================

use super::MartialArtType;

#[test]
fn art_training_youth_proficiency_gain() {
    // 청년, 강도 4: round(4 × 1.5 × 0.8) = round(4.8) = 5
    let result = calculate_art_training(MartialArtType::WeaponArt, 4, LifeStage::Youth);
    assert_eq!(result.proficiency_gain, 5);
}

#[test]
fn art_training_prime_proficiency_gain() {
    // 장년, 강도 4: round(4 × 1.0 × 0.8) = round(3.2) = 3
    let result = calculate_art_training(MartialArtType::InternalArt, 4, LifeStage::Prime);
    assert_eq!(result.proficiency_gain, 3);
}

#[test]
fn art_training_elder_minimum_one() {
    // 노년, 강도 1: round(1 × 0.3 × 0.8) = round(0.24) = 0 → 최소 1
    let result = calculate_art_training(MartialArtType::LightArt, 1, LifeStage::Elder);
    assert_eq!(result.proficiency_gain, 1);
}

#[test]
fn art_training_zero_intensity() {
    let result = calculate_art_training(MartialArtType::WeaponArt, 0, LifeStage::Youth);
    assert_eq!(result.proficiency_gain, 0);
    assert!(result.stat_side_effects.iter().all(|c| c.delta() == 0));
}

#[test]
fn art_training_side_effect_count() {
    // 모든 무공 유형은 정확히 3개의 부산물 능력치 변화를 생성
    for art_type in MartialArtType::all() {
        let result = calculate_art_training(*art_type, 4, LifeStage::Youth);
        assert_eq!(
            result.stat_side_effects.len(),
            3,
            "{:?} should have 3 side effects",
            art_type
        );
    }
}

#[test]
fn art_training_side_effect_stats_match_type() {
    // 병기무공 → 무력, 체력, 경공
    let result = calculate_art_training(MartialArtType::WeaponArt, 4, LifeStage::Youth);
    assert_eq!(result.stat_side_effects[0].stat(), StatType::Strength);
    assert_eq!(result.stat_side_effects[1].stat(), StatType::Vitality);
    assert_eq!(result.stat_side_effects[2].stat(), StatType::Agility);
}

#[test]
fn art_training_side_effect_values() {
    // 청년, 강도 4: round(4 × 1.5 × 0.25) = round(1.5) = 2
    let result = calculate_art_training(MartialArtType::WeaponArt, 4, LifeStage::Youth);
    for change in &result.stat_side_effects {
        assert_eq!(change.delta(), 2);
        assert_eq!(change.source(), ChangeSource::ArtPractice);
    }
}

#[test]
fn art_training_side_effect_minimum_one() {
    // 노년, 강도 1: round(1 × 0.3 × 0.25) = round(0.075) = 0 → 최소 1
    let result = calculate_art_training(MartialArtType::InternalArt, 1, LifeStage::Elder);
    for change in &result.stat_side_effects {
        assert_eq!(change.delta(), 1);
    }
}

#[test]
fn art_training_vs_stat_training_comparison() {
    // 같은 조건에서 연마 부산물 < 단련 직접 성장
    let stat_change = calculate_stat_training(StatType::Strength, 4, LifeStage::Youth);
    let art_result = calculate_art_training(MartialArtType::WeaponArt, 4, LifeStage::Youth);
    let side_effect = find_change(&art_result.stat_side_effects, StatType::Strength);

    // 단련: 6, 부산물: 2 → 부산물이 더 작다
    assert!(
        side_effect.delta() < stat_change.delta(),
        "side effect {} should be less than direct training {}",
        side_effect.delta(),
        stat_change.delta()
    );
}

// ===================================================================
// Effective Power — 실전위력 계산
// ===================================================================

#[test]
fn effective_power_basic_calculation() {
    // 85 × (45/100) × (60/50) = 85 × 0.45 × 1.2 = 45.9 → 46
    assert_eq!(calculate_effective_power(85, 45, 60), 46);
}

#[test]
fn effective_power_zero_proficiency_is_zero() {
    // 숙련도 0이면 아무리 강해도 실전위력 0
    assert_eq!(calculate_effective_power(100, 0, 100), 0);
    assert_eq!(calculate_effective_power(85, 0, 80), 0);
}

#[test]
fn effective_power_zero_base_power() {
    // 기본위력 0인 무공은 실전위력도 0
    assert_eq!(calculate_effective_power(0, 100, 100), 0);
}

#[test]
fn effective_power_max_values() {
    // 최대: 100 × (100/100) × (100/50) = 100 × 1.0 × 2.0 = 200
    assert_eq!(calculate_effective_power(100, 100, 100), 200);
}

#[test]
fn effective_power_average_stat_is_baseline() {
    // 능력치 평균 50 = ×1.0 기준점
    // 85 × (100/100) × (50/50) = 85
    assert_eq!(calculate_effective_power(85, 100, 50), 85);
}

#[test]
fn effective_power_low_stats_reduces() {
    // 능력치 평균 30 → ×0.6
    // 85 × (100/100) × (30/50) = 85 × 0.6 = 51
    assert_eq!(calculate_effective_power(85, 100, 30), 51);
}

#[test]
fn effective_power_high_stats_amplifies() {
    // 능력치 평균 80 → ×1.6 → 기본위력 초과!
    // 85 × (100/100) × (80/50) = 85 × 1.6 = 136
    assert_eq!(calculate_effective_power(85, 100, 80), 136);
}

#[test]
fn effective_power_realistic_scenario() {
    // 입문(숙련도 10), 보통 능력치(50)
    // 85 × 0.1 × 1.0 = 8.5 → 9
    assert_eq!(calculate_effective_power(85, 10, 50), 9);

    // 숙련(숙련도 45), 좋은 능력치(65)
    // 85 × 0.45 × 1.3 = 49.725 → 50
    assert_eq!(calculate_effective_power(85, 45, 65), 50);

    // 화경(숙련도 95), 뛰어난 능력치(75)
    // 85 × 0.95 × 1.5 = 121.125 → 121
    assert_eq!(calculate_effective_power(85, 95, 75), 121);
}
