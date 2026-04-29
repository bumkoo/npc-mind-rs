use super::*;
use crate::character::LifeStage;
use crate::growth::event::GrowthEvent;
use crate::growth::martial_art::MasteryLevel;
use crate::growth::stat::STAT_DEFAULT;
use crate::shared::MartialArtId;

// --- 생성 ---

#[test]
fn new_default_all_tens() {
    let profile = GrowthProfile::new_default(CharacterId::new(1));
    assert_eq!(profile.character_id(), CharacterId::new(1));

    for stat in StatType::all() {
        assert_eq!(
            profile.stat_value(*stat),
            STAT_DEFAULT,
            "{} should be {}",
            stat,
            STAT_DEFAULT
        );
    }
}

#[test]
fn new_with_stats_preserves_values() {
    let stats = StatBlock {
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
    let profile = GrowthProfile::new_with_stats(CharacterId::new(2), stats);

    assert_eq!(profile.stat_value(StatType::InnerPower), 50);
    assert_eq!(profile.stat_value(StatType::Wisdom), 30);
    assert_eq!(profile.stat_value(StatType::Strategy), 40);
    assert_eq!(profile.stat_value(StatType::Vitality), 80);
    assert_eq!(profile.stat_value(StatType::Agility), 60);
    assert_eq!(profile.stat_value(StatType::Strength), 70);
    assert_eq!(profile.stat_value(StatType::Willpower), 45);
    assert_eq!(profile.stat_value(StatType::Endurance), 55);
    assert_eq!(profile.stat_value(StatType::Empathy), 35);
}

// --- clamp ---

#[test]
fn new_with_stats_clamps_over_100() {
    let stats = StatBlock {
        inner_power: 150,
        wisdom: 200,
        strategy: 100,
        vitality: 0,
        agility: 50,
        strength: u32::MAX,
        willpower: 101,
        endurance: 99,
        empathy: 100,
    };
    let profile = GrowthProfile::new_with_stats(CharacterId::new(3), stats);

    assert_eq!(profile.stat_value(StatType::InnerPower), 100, "150 → 100");
    assert_eq!(profile.stat_value(StatType::Wisdom), 100, "200 → 100");
    assert_eq!(profile.stat_value(StatType::Strategy), 100, "100 → 100");
    assert_eq!(profile.stat_value(StatType::Vitality), 0, "0 → 0");
    assert_eq!(profile.stat_value(StatType::Agility), 50, "50 → 50");
    assert_eq!(profile.stat_value(StatType::Strength), 100, "MAX → 100");
    assert_eq!(profile.stat_value(StatType::Willpower), 100, "101 → 100");
    assert_eq!(profile.stat_value(StatType::Endurance), 99, "99 → 99");
    assert_eq!(profile.stat_value(StatType::Empathy), 100, "100 → 100");
}

// --- 합계 ---

#[test]
fn total_stats_default() {
    let profile = GrowthProfile::new_default(CharacterId::new(1));
    assert_eq!(profile.total_stats(), 90); // 10 × 9
}

#[test]
fn total_stats_custom() {
    let stats = StatBlock {
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
    let profile = GrowthProfile::new_with_stats(CharacterId::new(1), stats);
    assert_eq!(profile.total_stats(), 465);
}

// --- 범주별 합계 ---

#[test]
fn category_total_intellectual() {
    let stats = StatBlock {
        inner_power: 50,
        wisdom: 30,
        strategy: 40,
        ..StatBlock::default_stats()
    };
    let profile = GrowthProfile::new_with_stats(CharacterId::new(1), stats);
    assert_eq!(profile.category_total(StatCategory::Intellectual), 120); // 50+30+40
}

#[test]
fn category_total_physical() {
    let stats = StatBlock {
        vitality: 80,
        agility: 60,
        strength: 70,
        ..StatBlock::default_stats()
    };
    let profile = GrowthProfile::new_with_stats(CharacterId::new(1), stats);
    assert_eq!(profile.category_total(StatCategory::Physical), 210); // 80+60+70
}

#[test]
fn category_total_emotional() {
    let stats = StatBlock {
        willpower: 45,
        endurance: 55,
        empathy: 35,
        ..StatBlock::default_stats()
    };
    let profile = GrowthProfile::new_with_stats(CharacterId::new(1), stats);
    assert_eq!(profile.category_total(StatCategory::Emotional), 135); // 45+55+35
}

#[test]
fn all_category_totals_sum_to_total() {
    let stats = StatBlock {
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
    let profile = GrowthProfile::new_with_stats(CharacterId::new(1), stats);

    let cat_sum: u32 = StatCategory::all()
        .iter()
        .map(|c| profile.category_total(*c))
        .sum();

    assert_eq!(cat_sum, profile.total_stats());
}

// --- 전투력 ---

#[test]
fn combat_power_calculation() {
    let stats = StatBlock {
        inner_power: 50,  // ← 전투력에 포함
        wisdom: 30,
        strategy: 40,
        vitality: 80,
        agility: 60,      // ← 전투력에 포함
        strength: 70,     // ← 전투력에 포함
        willpower: 45,
        endurance: 55,
        empathy: 35,
    };
    let profile = GrowthProfile::new_with_stats(CharacterId::new(1), stats);

    // 무력(70) + 경공(60) + 내공(50) = 180
    assert_eq!(profile.combat_power(), 180);
}

#[test]
fn combat_power_default() {
    let profile = GrowthProfile::new_default(CharacterId::new(1));
    assert_eq!(profile.combat_power(), 30); // 10+10+10
}

// --- Display ---

#[test]
fn display_format() {
    let profile = GrowthProfile::new_default(CharacterId::new(1));
    let display = profile.to_string();
    assert!(display.contains("Growth Char-1"));
    assert!(display.contains("InnerPower:10"));
    assert!(display.contains("total:90"));
    assert!(display.contains("combat:30"));
}

// --- stats() getter ---

#[test]
fn stats_returns_reference() {
    let stats = StatBlock {
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
    let profile = GrowthProfile::new_with_stats(CharacterId::new(1), stats.clone());
    assert_eq!(*profile.stats(), stats);
}

// --- Serialization ---

#[test]
fn serialization_roundtrip() {
    let stats = StatBlock {
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
    let original = GrowthProfile::new_with_stats(CharacterId::new(42), stats);

    let json = serde_json::to_string(&original).unwrap();
    let restored: GrowthProfile = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.character_id(), original.character_id());
    for stat in StatType::all() {
        assert_eq!(
            restored.stat_value(*stat),
            original.stat_value(*stat),
            "Stat {} mismatch after roundtrip",
            stat,
        );
    }
}

// ===================================================================
// Iteration 2.2: train() 테스트
// ===================================================================

#[test]
fn train_youth_strength() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    let event = profile.train_stat(StatType::Strength, 4, LifeStage::Youth);

    // 10 + round(4 × 1.5) = 10 + 6 = 16
    assert_eq!(profile.stat_value(StatType::Strength), 16);

    match event {
        GrowthEvent::StatTrained { character_id, changes } => {
            assert_eq!(character_id, CharacterId::new(1));
            assert_eq!(changes.len(), 1);
            assert_eq!(changes[0].stat(), StatType::Strength);
            assert_eq!(changes[0].delta(), 6);
        }
        _ => panic!("Expected GrowthEvent::StatTrained"),
    }
}

#[test]
fn train_elder_minimum_growth() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    profile.train_stat(StatType::Wisdom, 1, LifeStage::Elder);

    // 10 + max(round(1 × 0.3), 1) = 10 + 1 = 11
    assert_eq!(profile.stat_value(StatType::Wisdom), 11);
}

#[test]
fn train_does_not_exceed_100() {
    let stats = StatBlock {
        inner_power: 98,
        ..StatBlock::default_stats()
    };
    let mut profile = GrowthProfile::new_with_stats(CharacterId::new(1), stats);
    profile.train_stat(StatType::InnerPower, 10, LifeStage::Youth);

    // 98 + 15 = 113 → clamp → 100
    assert_eq!(profile.stat_value(StatType::InnerPower), 100);
}

#[test]
fn train_zero_intensity_no_change() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    profile.train_stat(StatType::Vitality, 0, LifeStage::Youth);
    assert_eq!(profile.stat_value(StatType::Vitality), 10);
}

#[test]
fn train_does_not_affect_other_stats() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    profile.train_stat(StatType::Strength, 5, LifeStage::Prime);

    // Strength changed
    assert_eq!(profile.stat_value(StatType::Strength), 15); // 10 + 5
    // Others unchanged
    assert_eq!(profile.stat_value(StatType::Wisdom), 10);
    assert_eq!(profile.stat_value(StatType::Vitality), 10);
}

#[test]
fn train_same_stat_multiple_times() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    profile.train_stat(StatType::Agility, 3, LifeStage::Prime); // 10 + 3 = 13
    profile.train_stat(StatType::Agility, 3, LifeStage::Prime); // 13 + 3 = 16
    profile.train_stat(StatType::Agility, 3, LifeStage::Prime); // 16 + 3 = 19
    assert_eq!(profile.stat_value(StatType::Agility), 19);
}

// ===================================================================
// Iteration 2.2: apply_yearly_aging() 테스트
// ===================================================================

#[test]
fn yearly_aging_youth_grows_physical() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    let event = profile.apply_yearly_aging(LifeStage::Youth);

    assert_eq!(profile.stat_value(StatType::Vitality), 11);  // +1
    assert_eq!(profile.stat_value(StatType::Agility), 11);   // +1
    assert_eq!(profile.stat_value(StatType::Strength), 11);  // +1
    assert_eq!(profile.stat_value(StatType::Wisdom), 10);    // unchanged

    match event {
        GrowthEvent::YearlyAgingApplied { life_stage, changes, .. } => {
            assert_eq!(life_stage, LifeStage::Youth);
            assert_eq!(changes.len(), 3);
        }
        _ => panic!("Expected YearlyAgingApplied"),
    }
}

#[test]
fn yearly_aging_elder_decline() {
    let stats = StatBlock {
        inner_power: 90, wisdom: 95, strategy: 85,
        vitality: 30, agility: 25, strength: 20,
        willpower: 80, endurance: 70, empathy: 85,
    };
    let mut profile = GrowthProfile::new_with_stats(CharacterId::new(1), stats);
    profile.apply_yearly_aging(LifeStage::Elder);

    assert_eq!(profile.stat_value(StatType::Vitality), 28);   // 30 - 2
    assert_eq!(profile.stat_value(StatType::Agility), 23);    // 25 - 2
    assert_eq!(profile.stat_value(StatType::Strength), 19);   // 20 - 1
    assert_eq!(profile.stat_value(StatType::Wisdom), 96);     // 95 + 1
    assert_eq!(profile.stat_value(StatType::Strategy), 86);   // 85 + 1
    assert_eq!(profile.stat_value(StatType::Endurance), 69);  // 70 - 1
    // Unchanged
    assert_eq!(profile.stat_value(StatType::InnerPower), 90);
    assert_eq!(profile.stat_value(StatType::Willpower), 80);
    assert_eq!(profile.stat_value(StatType::Empathy), 85);
}

#[test]
fn yearly_aging_does_not_go_below_zero() {
    let stats = StatBlock {
        vitality: 1,
        agility: 0,
        ..StatBlock::default_stats()
    };
    let mut profile = GrowthProfile::new_with_stats(CharacterId::new(1), stats);
    profile.apply_yearly_aging(LifeStage::Elder);

    // 1 - 2 = underflow → saturating_sub → 0
    assert_eq!(profile.stat_value(StatType::Vitality), 0);
    // 0 - 2 = underflow → saturating_sub → 0
    assert_eq!(profile.stat_value(StatType::Agility), 0);
}

#[test]
fn yearly_aging_does_not_exceed_100() {
    let stats = StatBlock {
        wisdom: 99,
        ..StatBlock::default_stats()
    };
    let mut profile = GrowthProfile::new_with_stats(CharacterId::new(1), stats);
    profile.apply_yearly_aging(LifeStage::Middle); // wisdom +2

    // 99 + 2 = 101 → clamp → 100
    assert_eq!(profile.stat_value(StatType::Wisdom), 100);
}

// ===================================================================
// 무협 시나리오 테스트 — 통합
// ===================================================================

#[test]
fn scenario_young_warrior_vs_elder_sage() {
    // 열혈 청년 무인: 육체 강함, 지혜 낮음
    let young = GrowthProfile::new_with_stats(
        CharacterId::new(1),
        StatBlock {
            inner_power: 30,
            wisdom: 15,
            strategy: 20,
            vitality: 75,
            agility: 70,
            strength: 80,
            willpower: 50,
            endurance: 40,
            empathy: 20,
        },
    );

    // 노년 현자: 지혜 높음, 육체 쇠퇴
    let elder = GrowthProfile::new_with_stats(
        CharacterId::new(2),
        StatBlock {
            inner_power: 90,
            wisdom: 95,
            strategy: 85,
            vitality: 30,
            agility: 25,
            strength: 20,
            willpower: 80,
            endurance: 70,
            empathy: 85,
        },
    );

    // 청년: 육체 > 지적
    assert!(
        young.category_total(StatCategory::Physical)
            > young.category_total(StatCategory::Intellectual)
    );

    // 노인: 지적 > 육체
    assert!(
        elder.category_total(StatCategory::Intellectual)
            > elder.category_total(StatCategory::Physical)
    );

    // 전투력: 청년(80+70+30=180) > 노인(20+25+90=135)
    assert!(young.combat_power() > elder.combat_power());

    // 총 능력치: 노인(580) > 청년(400) — 경험이 깊다
    assert!(elder.total_stats() > young.total_stats());
}

#[test]
fn scenario_training_youth_vs_elder() {
    // 같은 강도(5)로 무력 수련: 청년 vs 노인
    let mut young = GrowthProfile::new_default(CharacterId::new(1));
    let mut elder = GrowthProfile::new_default(CharacterId::new(2));

    young.train_stat(StatType::Strength, 5, LifeStage::Youth);
    elder.train_stat(StatType::Strength, 5, LifeStage::Elder);

    // 청년: 10 + round(5 × 1.5) = 10 + 8 = 18
    // 노인: 10 + round(5 × 0.3) = 10 + 2 = 12
    assert!(young.stat_value(StatType::Strength) > elder.stat_value(StatType::Strength),
        "같은 수련, 청년({}) > 노인({})",
        young.stat_value(StatType::Strength),
        elder.stat_value(StatType::Strength));
}

#[test]
fn scenario_20_year_life_journey() {
    // 열혈 청년(25세)이 20년 동안 성장과 노화를 겪는 시뮬레이션
    let mut profile = GrowthProfile::new_with_stats(
        CharacterId::new(1),
        StatBlock {
            inner_power: 30, wisdom: 15, strategy: 20,
            vitality: 60, agility: 55, strength: 65,
            willpower: 40, endurance: 35, empathy: 25,
        },
    );

    let initial_vitality = profile.stat_value(StatType::Vitality);
    let initial_wisdom = profile.stat_value(StatType::Wisdom);

    // 25세~32세: 청년 (8년)
    for _ in 0..8 {
        profile.apply_yearly_aging(LifeStage::Youth);
    }
    let vitality_after_youth = profile.stat_value(StatType::Vitality);

    // 33세~44세: 장년 (12년)
    for _ in 0..12 {
        profile.apply_yearly_aging(LifeStage::Prime);
    }

    let final_vitality = profile.stat_value(StatType::Vitality);
    let final_wisdom = profile.stat_value(StatType::Wisdom);

    // 청년기에 체력이 올랐어야 함
    assert!(vitality_after_youth > initial_vitality,
        "청년기 체력: {} → {}", initial_vitality, vitality_after_youth);

    // 장년기에 체력은 유지 (변동 없음)
    assert_eq!(final_vitality, vitality_after_youth,
        "장년기 체력 유지: {}", final_vitality);

    // 20년간 지혜는 성장 (장년기 12년 × +1)
    assert!(final_wisdom > initial_wisdom,
        "20년간 지혜: {} → {}", initial_wisdom, final_wisdom);
}

// =======================================================================
// learn_art / train_art — 무공 습득 및 연마 [Iteration 2.3 Step 3]
// =======================================================================

#[test]
fn learn_art_success() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    assert!(profile.learn_art(MartialArtId::new(1)).is_ok());
    assert_eq!(profile.learned_arts().len(), 1);
}

#[test]
fn learn_art_duplicate_error() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    profile.learn_art(MartialArtId::new(1)).unwrap();
    let err = profile.learn_art(MartialArtId::new(1)).unwrap_err();
    assert_eq!(err, GrowthError::ArtAlreadyLearned(MartialArtId::new(1)));
}

#[test]
fn learn_multiple_arts() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    profile.learn_art(MartialArtId::new(1)).unwrap();
    profile.learn_art(MartialArtId::new(2)).unwrap();
    profile.learn_art(MartialArtId::new(3)).unwrap();
    assert_eq!(profile.learned_arts().len(), 3);
}

#[test]
fn art_proficiency_found() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    profile.learn_art(MartialArtId::new(5)).unwrap();
    let prof = profile.art_proficiency(MartialArtId::new(5));
    assert!(prof.is_some());
    assert_eq!(prof.unwrap().proficiency(), 0);
}

#[test]
fn art_proficiency_not_found() {
    let profile = GrowthProfile::new_default(CharacterId::new(1));
    assert!(profile.art_proficiency(MartialArtId::new(99)).is_none());
}

#[test]
fn train_art_not_learned_error() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    let err = profile
        .train_art(MartialArtId::new(1), MartialArtType::WeaponArt, 4, LifeStage::Youth)
        .unwrap_err();
    assert_eq!(err, GrowthError::ArtNotLearned(MartialArtId::new(1)));
}

#[test]
fn train_art_success_proficiency_increases() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    let art_id = MartialArtId::new(1);
    profile.learn_art(art_id).unwrap();

    let event = profile
        .train_art(art_id, MartialArtType::WeaponArt, 4, LifeStage::Youth)
        .unwrap();

    // 숙련도 확인: round(4 × 1.5 × 0.8) = 5
    let prof = profile.art_proficiency(art_id).unwrap();
    assert_eq!(prof.proficiency(), 5);

    match event {
        GrowthEvent::ArtPracticed {
            proficiency_gain,
            new_proficiency,
            ..
        } => {
            assert_eq!(proficiency_gain, 5);
            assert_eq!(new_proficiency, 5);
        }
        _ => panic!("Expected ArtPracticed"),
    }
}

#[test]
fn train_art_side_effect_stats_applied() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    let art_id = MartialArtId::new(1);
    profile.learn_art(art_id).unwrap();

    // 병기무공 연마 → 무력/체력/경공 부산물
    profile
        .train_art(art_id, MartialArtType::WeaponArt, 4, LifeStage::Youth)
        .unwrap();

    // 부산물: round(4 × 1.5 × 0.25) = 2
    assert_eq!(profile.stat_value(StatType::Strength), STAT_DEFAULT + 2);
    assert_eq!(profile.stat_value(StatType::Vitality), STAT_DEFAULT + 2);
    assert_eq!(profile.stat_value(StatType::Agility), STAT_DEFAULT + 2);

    // 관련 없는 능력치는 변하지 않음
    assert_eq!(profile.stat_value(StatType::Wisdom), STAT_DEFAULT);
}

#[test]
fn train_art_mastery_breakthrough_event() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    let art_id = MartialArtId::new(1);
    profile.learn_art(art_id).unwrap();

    // 숙련도를 29까지 올려놓기 (여러 번 연마)
    // 노년 강도 1 → 매번 숙련도 +1
    for _ in 0..29 {
        profile
            .train_art(art_id, MartialArtType::InternalArt, 1, LifeStage::Elder)
            .unwrap();
    }
    assert_eq!(
        profile.art_proficiency(art_id).unwrap().mastery_level(),
        MasteryLevel::Beginner
    );

    // 한 번 더 → 30 도달 → 숙련 경지 돌파!
    let event = profile
        .train_art(art_id, MartialArtType::InternalArt, 1, LifeStage::Elder)
        .unwrap();

    match event {
        GrowthEvent::ArtPracticed {
            old_mastery,
            new_mastery,
            ..
        } => {
            assert_eq!(old_mastery, MasteryLevel::Beginner);
            assert_eq!(new_mastery, MasteryLevel::Proficient);
        }
        _ => panic!("Expected ArtPracticed"),
    }
}

#[test]
fn train_art_proficiency_capped_at_100() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    let art_id = MartialArtId::new(1);
    profile.learn_art(art_id).unwrap();

    // 고강도로 많이 연마
    for _ in 0..50 {
        profile
            .train_art(art_id, MartialArtType::WeaponArt, 10, LifeStage::Youth)
            .unwrap();
    }

    let prof = profile.art_proficiency(art_id).unwrap();
    assert_eq!(prof.proficiency(), 100);
    assert_eq!(prof.mastery_level(), MasteryLevel::Transcendent);
}

#[test]
fn train_art_serialization_roundtrip() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    profile.learn_art(MartialArtId::new(1)).unwrap();
    profile
        .train_art(MartialArtId::new(1), MartialArtType::InternalArt, 4, LifeStage::Youth)
        .unwrap();

    let json = serde_json::to_string(&profile).unwrap();
    let restored: GrowthProfile = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.learned_arts().len(), 1);
    assert_eq!(
        restored.art_proficiency(MartialArtId::new(1)).unwrap().proficiency(),
        profile.art_proficiency(MartialArtId::new(1)).unwrap().proficiency()
    );
}

#[test]
fn scenario_art_vs_stat_training() {
    // 같은 조건에서 단련 vs 연마 비교
    let mut profile_stat = GrowthProfile::new_default(CharacterId::new(1));
    let mut profile_art = GrowthProfile::new_default(CharacterId::new(2));
    let art_id = MartialArtId::new(1);
    profile_art.learn_art(art_id).unwrap();

    // 단련: 무력 직접 수련
    profile_stat.train_stat(StatType::Strength, 4, LifeStage::Youth);

    // 연마: 병기무공 연마 (무력은 부산물)
    profile_art
        .train_art(art_id, MartialArtType::WeaponArt, 4, LifeStage::Youth)
        .unwrap();

    // 단련이 더 효과적 (6 vs 2)
    assert!(
        profile_stat.stat_value(StatType::Strength) > profile_art.stat_value(StatType::Strength),
        "단련({}) > 연마 부산물({})",
        profile_stat.stat_value(StatType::Strength),
        profile_art.stat_value(StatType::Strength)
    );

    // 하지만 연마는 무공 숙련도도 올린다
    assert!(profile_art.art_proficiency(art_id).unwrap().proficiency() > 0);
}

// =======================================================================
// art_effective_power / best_art_power — 실전위력 [Iteration 2.3 Step 4]
// =======================================================================

#[test]
fn art_effective_power_not_learned_returns_zero() {
    let profile = GrowthProfile::new_default(CharacterId::new(1));
    assert_eq!(
        profile.art_effective_power(MartialArtId::new(1), 85, MartialArtType::WeaponArt),
        0
    );
}

#[test]
fn art_effective_power_zero_proficiency() {
    // 습득했지만 연마 안 한 상태 → 숙련도 0 → 실전위력 0
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    profile.learn_art(MartialArtId::new(1)).unwrap();
    assert_eq!(
        profile.art_effective_power(MartialArtId::new(1), 85, MartialArtType::WeaponArt),
        0
    );
}

#[test]
fn art_effective_power_after_training() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    let art_id = MartialArtId::new(1);
    profile.learn_art(art_id).unwrap();

    // 청년 강도 4 → 숙련도 5, 부산물로 무력/체력/경공 각 +2
    profile
        .train_art(art_id, MartialArtType::WeaponArt, 4, LifeStage::Youth)
        .unwrap();

    let power = profile.art_effective_power(art_id, 85, MartialArtType::WeaponArt);

    // 숙련도 5, 관련능력치(Str=12, Vit=12, Agi=12) 평균 12
    // 85 × (5/100) × (12/50) = 85 × 0.05 × 0.24 = 1.02 → 1
    assert_eq!(power, 1);
}

#[test]
fn art_effective_power_high_proficiency() {
    let mut profile = GrowthProfile::new_with_stats(
        CharacterId::new(1),
        StatBlock {
            strength: 70, vitality: 60, agility: 65,
            inner_power: 50, wisdom: 30, strategy: 40,
            willpower: 45, endurance: 55, empathy: 35,
        },
    );
    let art_id = MartialArtId::new(1);
    profile.learn_art(art_id).unwrap();

    // 숙련도를 60까지 올리기 (고강도 연마 반복)
    for _ in 0..20 {
        profile
            .train_art(art_id, MartialArtType::WeaponArt, 10, LifeStage::Youth)
            .unwrap();
    }

    let prof = profile.art_proficiency(art_id).unwrap().proficiency();
    assert!(prof >= 60, "숙련도가 60 이상이어야: {}", prof);

    let power = profile.art_effective_power(art_id, 85, MartialArtType::WeaponArt);
    assert!(power > 50, "통달 수준의 실전위력이어야: {}", power);
}

#[test]
fn best_art_power_empty_arts() {
    let profile = GrowthProfile::new_default(CharacterId::new(1));
    assert_eq!(profile.best_art_power(&[]), 0);
}

#[test]
fn best_art_power_selects_strongest() {
    let mut profile = GrowthProfile::new_with_stats(
        CharacterId::new(1),
        StatBlock {
            strength: 60, vitality: 60, agility: 60,
            inner_power: 60, wisdom: 40, strategy: 40,
            willpower: 50, endurance: 50, empathy: 40,
        },
    );
    let art1 = MartialArtId::new(1); // 병기, 위력 85
    let art2 = MartialArtId::new(2); // 내공, 위력 60
    profile.learn_art(art1).unwrap();
    profile.learn_art(art2).unwrap();

    // 같은 횟수 연마
    for _ in 0..10 {
        profile.train_art(art1, MartialArtType::WeaponArt, 4, LifeStage::Youth).unwrap();
        profile.train_art(art2, MartialArtType::InternalArt, 4, LifeStage::Youth).unwrap();
    }

    let power1 = profile.art_effective_power(art1, 85, MartialArtType::WeaponArt);
    let power2 = profile.art_effective_power(art2, 60, MartialArtType::InternalArt);

    let arts = [
        (art1, 85, MartialArtType::WeaponArt),
        (art2, 60, MartialArtType::InternalArt),
    ];
    let best = profile.best_art_power(&arts);

    // best는 둘 중 큰 것
    assert_eq!(best, power1.max(power2));
    assert!(best > 0);
}

#[test]
fn best_art_power_ignores_unlearned() {
    let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    let art1 = MartialArtId::new(1);
    let art2 = MartialArtId::new(2); // 미습득

    profile.learn_art(art1).unwrap();
    // art1만 연마
    for _ in 0..10 {
        profile.train_art(art1, MartialArtType::WeaponArt, 4, LifeStage::Youth).unwrap();
    }

    let arts = [
        (art1, 85, MartialArtType::WeaponArt),
        (art2, 100, MartialArtType::InternalArt), // 미습득 → 0
    ];
    let best = profile.best_art_power(&arts);
    let power1 = profile.art_effective_power(art1, 85, MartialArtType::WeaponArt);
    assert_eq!(best, power1);
}

#[test]
fn scenario_martial_artist_vs_brute() {
    // 무공은 없지만 능력치가 높은 "장사"
    let brute = GrowthProfile::new_with_stats(
        CharacterId::new(1),
        StatBlock {
            strength: 95, vitality: 85, agility: 85,
            inner_power: 60, wisdom: 15, strategy: 10,
            willpower: 60, endurance: 80, empathy: 10,
        },
    );

    // 능력치는 보통이지만 화경을 향해 수련하는 "고수"
    let mut master = GrowthProfile::new_with_stats(
        CharacterId::new(2),
        StatBlock {
            strength: 45, vitality: 45, agility: 45,
            inner_power: 50, wisdom: 60, strategy: 50,
            willpower: 70, endurance: 55, empathy: 45,
        },
    );
    let art_id = MartialArtId::new(1);
    master.learn_art(art_id).unwrap();
    // 고강도 8회 → 숙련도 96(화경), 부산물 Str/Vit/Agi 각 +32
    for _ in 0..8 {
        master
            .train_art(art_id, MartialArtType::WeaponArt, 10, LifeStage::Youth)
            .unwrap();
    }

    // 기초 전투력: 장사가 더 높다
    // 장사: 95+85+60=240, 고수: 77+77+50=204
    assert!(
        brute.combat_power() > master.combat_power(),
        "기초 전투력: 장사({}) > 고수({})",
        brute.combat_power(),
        master.combat_power()
    );

    // 실전위력: 고수가 압도적
    let master_power = master.art_effective_power(art_id, 85, MartialArtType::WeaponArt);
    assert!(
        master_power > 0,
        "고수의 실전위력이 존재해야: {}",
        master_power
    );

    // 장사의 무공 실전위력은 0 (무공을 모른다)
    let brute_power = brute.art_effective_power(art_id, 85, MartialArtType::WeaponArt);
    assert_eq!(brute_power, 0, "장사는 무공을 모른다");

    // "무공을 배운 자가 힘만 센 자를 이긴다"
    assert!(
        master_power > brute_power,
        "실전위력: 고수({}) > 장사({})",
        master_power,
        brute_power
    );
}
