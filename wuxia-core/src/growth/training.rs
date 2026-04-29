// wuxia-core/src/growth/training.rs
//
// Training & Aging Rules — 수련과 쇠퇴의 규칙
//
// 이 모듈은 "얼마나 성장/쇠퇴하는가?"의 규칙만 담는다.
// GrowthProfile(aggregate)에서 이 규칙을 호출하여 능력치를 변경한다.
//
// 규칙과 데이터(aggregate)를 분리하는 이유:
//   - 계수/테이블을 바꿀 때 aggregate를 건드리지 않아도 됨
//   - 규칙 함수가 순수 함수 → 테스트가 간단
//   - 향후 외부 파일(TOML/JSON)에서 테이블 로딩 가능
//
// ┌──────────────────────────────────────────────────────┐
// │  단련 (Stat Training / Conditioning)                │
// │                                                      │
// │  실제 성장량 = intensity × growth_multiplier(stage)   │
// │                                                      │
// │    청년(1.5x) > 장년(1.0x) > 중년(0.7x) > 노년(0.3x) │
// └──────────────────────────────────────────────────────┘
//
// ┌──────────────────────────────────────────────────────┐
// │  연마 (Art Practice / 鍊磨)                          │
// │                                                      │
// │  숙련도 상승 = intensity × multiplier × 0.8           │
// │  부산물 능력치 = intensity × multiplier × 0.25        │
// │                                                      │
// │  예) 청년, 강도4, 병기무공                            │
// │    숙련도 +5, 무력+2 체력+2 경공+2                    │
// └──────────────────────────────────────────────────────┘
//
// ┌──────────────────────────────────────────────────────┐
// │  연간 노화 (Yearly Aging)                            │
// │                                                      │
// │  매년 자동 적용: 생애 단계별 능력치 자연 변동         │
// │                                                      │
// │            체력 경공 무력 내공 지혜 책략 의지 인내 공감 │
// │  청년(~32)  +1  +1  +1   0   0   0   0   0   0     │
// │  장년(33~54) 0   0   0  +1  +1  +1   0   0   0     │
// │  중년(55~68)-1  -1   0  +1  +2  +1  +1   0  +1     │
// │  노년(69~) -2  -2  -1   0  +1  +1   0  -1   0     │
// └──────────────────────────────────────────────────────┘
//
// ┌──────────────────────────────────────────────────────┐
// │  최대 강도 (Max Intensity) [v2.3B]                    │
// │                                                      │
// │  base = (체력 + 인내) / 20                             │
// │  will_bonus = 의지 ≥ 70 → +1                           │
// │  fatigue_pen = Fresh(0) Mild(1) Mod(2) Sev(3)        │
// │  result = clamp(base + will - fatigue, 1, 10)         │
// │  Exhausted → None (수련 불가)                          │
// └──────────────────────────────────────────────────────┘
//
// ┌──────────────────────────────────────────────────────┐
// │  실전위력 (Effective Power)                           │
// │                                                      │
// │  = base_power × (proficiency/100) × (stat_avg/50)   │
// │                                                      │
// │  숙련도 0 → 실전위력 0 (못 쓰는 상태)                 │
// │  능력치 50 → ×1.0 기준점                              │
// │  화경+고능력치 → base_power 초과 가능 (극한 발현)      │
// └──────────────────────────────────────────────────────┘

use crate::character::LifeStage;
use crate::character::fatigue::FatigueLevel;

use super::event::{ChangeSource, StatChange};
use super::martial_art::MartialArtType;
use super::stat::StatType;

// ---------------------------------------------------------------------------
// Growth Multiplier — 생애 단계별 수련 효율
// ---------------------------------------------------------------------------

/// 생애 단계별 수련 성장 계수.
///
/// 같은 양의 수련(intensity)을 해도, 청년은 빠르게 성장하고
/// 노년은 느리게 성장한다.
///
/// ```
/// use wuxia_core::character::LifeStage;
/// use wuxia_core::growth::training::growth_multiplier;
///
/// assert_eq!(growth_multiplier(LifeStage::Youth), 1.5);
/// assert_eq!(growth_multiplier(LifeStage::Prime), 1.0);
/// assert_eq!(growth_multiplier(LifeStage::Middle), 0.7);
/// assert_eq!(growth_multiplier(LifeStage::Elder), 0.3);
/// ```
pub fn growth_multiplier(life_stage: LifeStage) -> f32 {
    match life_stage {
        LifeStage::Youth => 1.5,
        LifeStage::Prime => 1.0,
        LifeStage::Middle => 0.7,
        LifeStage::Elder => 0.3,
    }
}

// ---------------------------------------------------------------------------
// Training Calculation — 단련(鍛鍊) 성장량 계산
// ---------------------------------------------------------------------------

/// 단련(鍛鍊)에 의한 능력치 성장량을 계산한다.
///
/// `intensity`는 단련의 강도(1~10 정도 권장).
/// 실제 성장량 = round(intensity × growth_multiplier(life_stage)).
/// 최소 1 보장 (단련을 했으면 아주 작아도 성장한다).
///
/// 이 함수는 StatChange만 반환하며, 실제 적용은 하지 않는다.
/// GrowthProfile::train_stat()이 이 결과를 받아서 적용한다.
///
/// # Example
/// ```
/// use wuxia_core::character::LifeStage;
/// use wuxia_core::growth::training::calculate_stat_training;
/// use wuxia_core::growth::StatType;
///
/// // 청년이 강도 4로 무력 단련
/// let change = calculate_stat_training(StatType::Strength, 4, LifeStage::Youth);
/// assert_eq!(change.delta(), 6); // round(4 × 1.5) = 6
///
/// // 노년이 같은 강도로 단련
/// let change = calculate_stat_training(StatType::Strength, 4, LifeStage::Elder);
/// assert_eq!(change.delta(), 1); // round(4 × 0.3) = 1.2 → 1, 최소 1
/// ```
pub fn calculate_stat_training(
    stat: StatType,
    intensity: u32,
    life_stage: LifeStage,
) -> StatChange {
    let multiplier = growth_multiplier(life_stage);
    let raw = (intensity as f32 * multiplier).round() as i32;
    // 단련을 했으면(intensity > 0) 최소 1은 성장
    let delta = if intensity > 0 { raw.max(1) } else { 0 };
    StatChange::new(stat, delta, ChangeSource::StatTraining)
}

// ---------------------------------------------------------------------------
// Art Training Calculation — 연마(鍊磨) 성장량 계산
// ---------------------------------------------------------------------------

/// 연마 부산물의 능력치 상승 비율.
/// 단련 대비 25%만 능력치에 반영된다.
const SIDE_EFFECT_RATIO: f32 = 0.25;

/// 연마 숙련도 상승 계수.
/// 단련의 능력치 상승보다 약간 낮게 설정 (0.8배).
const ART_PROFICIENCY_RATIO: f32 = 0.8;

/// 연마 결과를 담는 구조체.
///
/// 숙련도 상승량과 부산물 능력치 변화를 함께 반환한다.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtTrainingResult {
    /// 무공 숙련도 상승량
    pub proficiency_gain: u32,
    /// 부산물 능력치 변화 (관련 능력치 3개)
    pub stat_side_effects: Vec<StatChange>,
}

/// 연마(鍊磨)에 의한 숙련도 상승 + 부산물 능력치 변화를 계산한다.
///
/// 숙련도 상승 = round(intensity × growth_multiplier × ART_PROFICIENCY_RATIO)
/// 부산물 능력치 = round(intensity × growth_multiplier × SIDE_EFFECT_RATIO)
///
/// 둘 다 intensity > 0이면 최소 1 보장.
///
/// # Example
/// ```
/// use wuxia_core::character::LifeStage;
/// use wuxia_core::growth::training::calculate_art_training;
/// use wuxia_core::growth::martial_art::MartialArtType;
///
/// // 청년이 강도 4로 병기 무공 연마
/// let result = calculate_art_training(MartialArtType::WeaponArt, 4, LifeStage::Youth);
/// assert_eq!(result.proficiency_gain, 5);  // round(4 × 1.5 × 0.8) = 4.8 → 5
/// assert_eq!(result.stat_side_effects.len(), 3); // 무력, 체력, 경공
/// assert_eq!(result.stat_side_effects[0].delta(), 2); // round(4 × 1.5 × 0.25) = 1.5 → 2
/// ```
pub fn calculate_art_training(
    art_type: MartialArtType,
    intensity: u32,
    life_stage: LifeStage,
) -> ArtTrainingResult {
    let multiplier = growth_multiplier(life_stage);

    // 숙련도 상승: intensity × multiplier × 0.8
    let prof_raw = (intensity as f32 * multiplier * ART_PROFICIENCY_RATIO).round() as u32;
    let proficiency_gain = if intensity > 0 { prof_raw.max(1) } else { 0 };

    // 부산물 능력치: 각 관련 stat에 대해 intensity × multiplier × 0.25
    let side_raw = (intensity as f32 * multiplier * SIDE_EFFECT_RATIO).round() as i32;
    let side_delta = if intensity > 0 { side_raw.max(1) } else { 0 };

    let stat_side_effects = art_type
        .related_stats()
        .iter()
        .map(|&stat| StatChange::new(stat, side_delta, ChangeSource::ArtPractice))
        .collect();

    ArtTrainingResult {
        proficiency_gain,
        stat_side_effects,
    }
}

// ---------------------------------------------------------------------------
// Effective Power — 무공 실전위력 계산
// ---------------------------------------------------------------------------

/// 무공 한 가지의 실전위력을 계산한다.
///
/// 공식: base_power × (proficiency / 100) × (related_stat_avg / 50)
///
/// - `base_power`: 무공 자체의 기본 위력 (1~100)
/// - `proficiency`: 캐릭터의 숙련도 (0~100)
/// - `related_stat_avg`: 관련 능력치 3개의 평균 (0~100)
///
/// 50으로 나누는 이유: 능력치 50이 "보통 사람" → ×1.0 기준점.
/// 능력치가 높으면 같은 무공이라도 더 강하게 발휘된다.
///
/// 숙련도 0이면 실전위력 0 (배웠지만 실전에서 못 쓰는 상태).
/// 화경(90+) + 고능력치(70+)면 base_power를 초과할 수 있다
/// (무공의 극한을 끌어내는 고수).
///
/// # Example
/// ```
/// use wuxia_core::growth::training::calculate_effective_power;
///
/// // 독고구검(85위력), 숙련도 45, 관련능력치 평균 60
/// let power = calculate_effective_power(85, 45, 60);
/// assert_eq!(power, 46); // 85 × 0.45 × 1.2 = 45.9 → 46
///
/// // 숙련도 0이면 실전위력 0
/// assert_eq!(calculate_effective_power(85, 0, 80), 0);
/// ```
pub fn calculate_effective_power(
    base_power: u32,
    proficiency: u32,
    related_stat_avg: u32,
) -> u32 {
    if proficiency == 0 {
        return 0;
    }
    let power = base_power as f32 * (proficiency as f32 / 100.0) * (related_stat_avg as f32 / 50.0);
    power.round() as u32
}

// ---------------------------------------------------------------------------
// Fatigue from Training — 수련에 의한 피로 증가량 [v2.3A Step 3]
// ---------------------------------------------------------------------------

/// 수련 후 발생하는 피로량을 계산한다.
///
/// 기본 공식: intensity × fatigue_multiplier(fatigue_level)
///
/// 이미 피로가 높을수록 같은 강도의 수련이 더 많은 피로를 유발한다.
/// ("피곤할 때 무리하면 배로 지친다")
///
/// # Example
/// ```
/// use wuxia_core::character::FatigueLevel;
/// use wuxia_core::growth::training::calculate_fatigue_from_training;
///
/// // 양호 상태, 강도 4 → 피로 4
/// assert_eq!(calculate_fatigue_from_training(4, FatigueLevel::Fresh), 4);
///
/// // 심각 상태, 강도 4 → 피로 8 (×2.0)
/// assert_eq!(calculate_fatigue_from_training(4, FatigueLevel::Severe), 8);
/// ```
pub fn calculate_fatigue_from_training(intensity: u32, fatigue_level: FatigueLevel) -> u32 {
    let multiplier = fatigue_training_multiplier(fatigue_level);
    let raw = (intensity as f32 * multiplier).round() as u32;
    if intensity > 0 { raw.max(1) } else { 0 }
}

/// 피로 단계별 수련 피로 배수.
///
/// 양호 상태에서는 피로가 적게 쌓이고,
/// 심각 상태에서는 같은 수련에도 피로가 2배로 쌓인다.
fn fatigue_training_multiplier(fatigue_level: FatigueLevel) -> f32 {
    match fatigue_level {
        FatigueLevel::Fresh => 1.0,
        FatigueLevel::Mild => 1.2,
        FatigueLevel::Moderate => 1.5,
        FatigueLevel::Severe => 2.0,
        FatigueLevel::Exhausted => 0.0, // 수련 불가 → 이 함수 호출되면 안 됨
    }
}

// ---------------------------------------------------------------------------
// Injury Chance — 부상 확률 계산 [v2.3A Step 4]
// ---------------------------------------------------------------------------

/// 수련 후 부상 발생 확률(0.0~1.0)을 계산한다.
///
/// 공식:
///   base_chance = 피로 단계별 기본 확률
///   intensity_bonus = max(0, intensity - 5) × 0.03  (강도 6부터 추가)
///   over_limit_bonus = 최대 강도 초과 시 +0.10
///   total = base_chance + intensity_bonus + over_limit_bonus  (최대 0.8)
///
/// # Example
/// ```
/// use wuxia_core::character::FatigueLevel;
/// use wuxia_core::growth::training::calculate_injury_chance;
///
/// // 양호 + 강도 3 → 거의 0%
/// let chance = calculate_injury_chance(3, FatigueLevel::Fresh, false);
/// assert!(chance < 0.01);
///
/// // 심각 + 강도 8 + 한계 초과 → 높은 확률
/// let chance = calculate_injury_chance(8, FatigueLevel::Severe, true);
/// assert!(chance > 0.30);
/// ```
pub fn calculate_injury_chance(
    intensity: u32,
    fatigue_level: FatigueLevel,
    over_max_intensity: bool,
) -> f32 {
    let base = base_injury_chance(fatigue_level);
    // 강도 6 이상이면 초과분 × 3%
    let intensity_bonus = if intensity > 5 {
        (intensity - 5) as f32 * 0.03
    } else {
        0.0
    };
    // 의지로 한계를 넘긴 경우 +10%
    let over_limit_bonus = if over_max_intensity { 0.10 } else { 0.0 };

    (base + intensity_bonus + over_limit_bonus).min(0.8)
}

/// 피로 단계별 부상 기본 확률.
fn base_injury_chance(fatigue_level: FatigueLevel) -> f32 {
    match fatigue_level {
        FatigueLevel::Fresh => 0.0,
        FatigueLevel::Mild => 0.02,
        FatigueLevel::Moderate => 0.05,
        FatigueLevel::Severe => 0.15,
        FatigueLevel::Exhausted => 0.0, // 수련 불가
    }
}

// ---------------------------------------------------------------------------
// Max Intensity — 최대 수련 강도 [v2.3B]
// ---------------------------------------------------------------------------

/// 캐릭터가 수행할 수 있는 최대 수련 강도를 계산한다.
///
/// 무협적 의미: "체력과 인내가 수련의 그릇이고, 의지가 한 숟갈 더 떠준다.
/// 하지만 피로가 쌓이면 그릇이 줄어든다."
///
/// 공식:
///   base         = (vitality + endurance) / 20
///   will_bonus   = if willpower >= 70 { 1 } else { 0 }
///   fatigue_pen  = 피로 단계별 페널티 (0 / 1 / 2 / 3 / 수련불가)
///   result       = clamp(base + will_bonus - fatigue_pen, 1, 10)
///
/// `Exhausted` 상태이면 `None`을 반환한다 (수련 불가).
///
/// # Example
/// ```
/// use wuxia_core::character::FatigueLevel;
/// use wuxia_core::growth::training::calculate_max_intensity;
///
/// // 건강한 청년: (60+40)/20 = 5, 의지 50 < 70 → +0, Fresh → -0 = 5
/// assert_eq!(calculate_max_intensity(60, 40, 50, FatigueLevel::Fresh), Some(5));
///
/// // 의지 강한 노인: (25+20)/20 = 2, 의지 80 ≥ 70 → +1, Fresh → -0 = 3
/// assert_eq!(calculate_max_intensity(25, 20, 80, FatigueLevel::Fresh), Some(3));
///
/// // 탈진 상태 → 수련 불가
/// assert_eq!(calculate_max_intensity(60, 40, 50, FatigueLevel::Exhausted), None);
/// ```
pub fn calculate_max_intensity(
    vitality: u32,
    endurance: u32,
    willpower: u32,
    fatigue_level: FatigueLevel,
) -> Option<u32> {
    if fatigue_level == FatigueLevel::Exhausted {
        return None; // 수련 불가
    }

    let base = (vitality + endurance) / 20;
    let will_bonus: u32 = if willpower >= 70 { 1 } else { 0 };
    let fatigue_pen = fatigue_intensity_penalty(fatigue_level);

    // i32로 변환하여 음수 처리 후 clamp
    let raw = base as i32 + will_bonus as i32 - fatigue_pen as i32;
    let clamped = raw.clamp(1, 10) as u32;

    Some(clamped)
}

/// 의지 보너스 없이의 순수 기본 최대 강도.
///
/// TrainingService에서 "의지 보너스로 한계를 넘긴 강도"
/// 여부를 판정하는 데 사용.
///
/// # Example
/// ```
/// use wuxia_core::character::FatigueLevel;
/// use wuxia_core::growth::training::{calculate_max_intensity, calculate_base_max_intensity};
///
/// let full = calculate_max_intensity(60, 40, 80, FatigueLevel::Fresh).unwrap();
/// let base = calculate_base_max_intensity(60, 40, FatigueLevel::Fresh).unwrap();
/// assert_eq!(full, 6);  // 5 + 의지보너스 1
/// assert_eq!(base, 5);  // 의지보너스 없이
/// ```
pub fn calculate_base_max_intensity(
    vitality: u32,
    endurance: u32,
    fatigue_level: FatigueLevel,
) -> Option<u32> {
    if fatigue_level == FatigueLevel::Exhausted {
        return None;
    }

    let base = (vitality + endurance) / 20;
    let fatigue_pen = fatigue_intensity_penalty(fatigue_level);

    let raw = base as i32 - fatigue_pen as i32;
    let clamped = raw.clamp(1, 10) as u32;

    Some(clamped)
}

/// 피로 단계별 최대 강도 감산.
fn fatigue_intensity_penalty(fatigue_level: FatigueLevel) -> u32 {
    match fatigue_level {
        FatigueLevel::Fresh => 0,
        FatigueLevel::Mild => 1,
        FatigueLevel::Moderate => 2,
        FatigueLevel::Severe => 3,
        FatigueLevel::Exhausted => 0, // 수련 불가이므로 도달하지 않음
    }
}

// ---------------------------------------------------------------------------
// Yearly Aging — 연간 자연 변동
// ---------------------------------------------------------------------------

/// 생애 단계별 연간 자연 변동 테이블.
///
/// 무협적 해석:
///   청년(~32):  몸이 자라는 시기. 체력/경공/무력 자연 성장.
///   장년(33~54): 내공이 원숙해지고 지혜와 책략이 깊어지는 절정기.
///   중년(55~68): 육체 쇠퇴 시작. 하지만 지혜/의지/공감이 보상.
///   노년(69~):  장삼봉처럼 지혜는 남지만, 몸이 확연히 약해짐.
///
/// ```
/// use wuxia_core::character::LifeStage;
/// use wuxia_core::growth::training::calculate_yearly_aging;
/// use wuxia_core::growth::StatType;
///
/// // 중년: 체력 쇠퇴, 지혜 성장
/// let changes = calculate_yearly_aging(LifeStage::Middle);
/// let vitality_change = changes.iter().find(|c| c.stat() == StatType::Vitality).unwrap();
/// let wisdom_change = changes.iter().find(|c| c.stat() == StatType::Wisdom).unwrap();
/// assert_eq!(vitality_change.delta(), -1);
/// assert_eq!(wisdom_change.delta(), 2);
/// ```
pub fn calculate_yearly_aging(life_stage: LifeStage) -> Vec<StatChange> {
    let table: &[(StatType, i32)] = match life_stage {
        //                               delta
        LifeStage::Youth => &[
            (StatType::Vitality, 1),   // 체력 ↑
            (StatType::Agility, 1),    // 경공 ↑
            (StatType::Strength, 1),   // 무력 ↑
        ],
        LifeStage::Prime => &[
            (StatType::InnerPower, 1), // 내공 ↑
            (StatType::Wisdom, 1),     // 지혜 ↑
            (StatType::Strategy, 1),   // 책략 ↑
        ],
        LifeStage::Middle => &[
            (StatType::Vitality, -1),  // 체력 ↓
            (StatType::Agility, -1),   // 경공 ↓
            (StatType::InnerPower, 1), // 내공 ↑
            (StatType::Wisdom, 2),     // 지혜 ↑↑
            (StatType::Strategy, 1),   // 책략 ↑
            (StatType::Willpower, 1),  // 의지 ↑
            (StatType::Empathy, 1),    // 공감 ↑
        ],
        LifeStage::Elder => &[
            (StatType::Vitality, -2),  // 체력 ↓↓
            (StatType::Agility, -2),   // 경공 ↓↓
            (StatType::Strength, -1),  // 무력 ↓
            (StatType::Wisdom, 1),     // 지혜 ↑
            (StatType::Strategy, 1),   // 책략 ↑
            (StatType::Endurance, -1), // 인내 ↓
        ],
    };

    table
        .iter()
        .filter(|(_, delta)| *delta != 0)
        .map(|(stat, delta)| StatChange::new(*stat, *delta, ChangeSource::YearlyAging))
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "training_tests.rs"]
mod tests;
