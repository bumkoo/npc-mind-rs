// wuxia-core/src/application/training.rs
//
// TrainingService — "수련하면 강해지고, 피로해지고, 다칠 수도 있다"
//
// 성장 도메인과 캐릭터 도메인을 조율하는 Application Service.
// 수련(단련/연마) 요청을 받아서:
//   1. 수련 가능 여부 확인 (탈진, 부상)
//   2. 최대 강도 계산 & 제한
//   3. 능력치/숙련도 성장 적용
//   4. 피로 누적
//   5. 부상 판정
//
// ┌──────────────────────────────────────────────────┐
// │  TrainingService (얇은 조율자)                     │
// │                                                    │
// │  입력: Character + GrowthProfile + 수련 요청        │
// │  출력: TrainingOutcome + DomainEvent들              │
// │                                                    │
// │  "무력을 강도 5로 단련"                             │
// │    → 탈진? 부상? 확인                               │
// │    → 최대 강도 = 5 → OK                             │
// │    → 성장: 무력 +8                                  │
// │    → 피로: +5                                       │
// │    → 부상 판정: 5% → 무사                           │
// │    → 결과 반환                                      │
// └──────────────────────────────────────────────────┘

use crate::character::fatigue::FatigueLevel;
use crate::character::injury::{InjurySeverity, InjuryType};
use crate::character::Character;
use crate::growth::martial_art::MartialArtType;
use crate::growth::model::GrowthProfile;
use crate::growth::stat::StatType;
use crate::growth::training;
use crate::shared::event::DomainEvent;
use crate::shared::id::MartialArtId;

// ---------------------------------------------------------------------------
// TrainingError
// ---------------------------------------------------------------------------

/// 수련 시도 시 발생할 수 있는 에러.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrainingError {
    /// 탈진 상태 — 수련 불가.
    Exhausted,
    /// 부상으로 수련 불가 (골절, 주화입마).
    InjuryPreventsTraining,
    /// 요청 강도가 최대 강도 초과.
    IntensityTooHigh { requested: u32, max: u32 },
    /// 습득하지 않은 무공 연마 시도.
    ArtNotLearned(MartialArtId),
}

impl std::fmt::Display for TrainingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrainingError::Exhausted => write!(f, "탈진 상태 — 수련 불가"),
            TrainingError::InjuryPreventsTraining => {
                write!(f, "부상으로 수련 불가")
            }
            TrainingError::IntensityTooHigh { requested, max } => {
                write!(f, "강도 초과: 요청 {}, 최대 {}", requested, max)
            }
            TrainingError::ArtNotLearned(id) => {
                write!(f, "습득하지 않은 무공: {}", id)
            }
        }
    }
}

impl std::error::Error for TrainingError {}

// ---------------------------------------------------------------------------
// TrainingOutcome
// ---------------------------------------------------------------------------

/// 수련 결과를 담는 구조체.
///
/// 수련 후 무슨 일이 있었는지 호출자에게 알린다.
/// DomainEvent는 이벤트 버스에 전파하고,
/// TrainingOutcome은 UI/NPC AI에서 표시/판단에 사용.
#[derive(Debug, Clone, PartialEq)]
pub struct TrainingOutcome {
    /// 요청한 강도 (부상 페널티 적용 전).
    pub requested_intensity: u32,
    /// 실제 적용된 강도 (부상 페널티 차감 후).
    pub effective_intensity: u32,
    /// 피로 증가량.
    pub fatigue_gained: u32,
    /// 부상이 발생했는가?
    pub injury_occurred: bool,
    /// 의지 보너스로 한계를 넘긴 강도분.
    pub over_limit: bool,
}

// ---------------------------------------------------------------------------
// TrainingService
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// TrainingContext — prepare_training()의 결과물
// ---------------------------------------------------------------------------

/// 수련 전 공통 계산 결과를 담는 내부 Value Object.
///
/// prepare_training()이 생성하고, finalize_training()이 소비한다.
/// train_stat/train_art 사이의 중복 코드를 제거하기 위한 중간 객체.
struct TrainingContext {
    requested_intensity: u32,
    effective_intensity: u32,
    over_limit: bool,
    fatigue_level: FatigueLevel,
}

// ---------------------------------------------------------------------------
// TrainingService
// ---------------------------------------------------------------------------

/// Application Service: 수련(단련/연마) 유스케이스 조율.
///
/// # 내부 흐름 (prepare → 도메인 호출 → finalize)
/// ```text
/// prepare_training():
///   1. can_train() → 탈진/부상 확인
///   2. calculate_max_intensity() → 최대 강도 확인
///   3. over_limit 판정
///   4. 부상 페널티 적용 → effective_intensity
///   → TrainingContext 반환
///
/// 호출자: growth.train_stat() 또는 growth.train_art()
///
/// finalize_training():
///   5. character.add_fatigue() → 피로 누적
///   6. calculate_injury_chance() → 부상 판정
///   7. DomainEvent 수집 + TrainingOutcome 반환
/// ```
///
/// # Example
/// ```
/// use wuxia_core::shared::CharacterId;
/// use wuxia_core::character::{Character, Gender, CharacterRole};
/// use wuxia_core::growth::{GrowthProfile, StatType};
/// use wuxia_core::application::TrainingService;
///
/// let service = TrainingService::new();
/// let mut character = Character::new(
///     CharacterId::new(1), "무명소졸".into(), None,
///     Gender::Male, 1180, 20, CharacterRole::Npc,
/// );
/// let mut growth = GrowthProfile::new_default(CharacterId::new(1));
///
/// let (outcome, events) = service.train_stat(
///     &mut character, &mut growth,
///     StatType::Strength, 1,
/// ).unwrap();
///
/// assert_eq!(outcome.effective_intensity, 1);
/// assert!(outcome.fatigue_gained > 0);
/// ```
pub struct TrainingService;

impl TrainingService {
    pub fn new() -> Self {
        Self
    }

    /// 능력치 단련(鍛鍊)을 수행한다.
    ///
    /// 성장 도메인과 캐릭터 도메인을 조율하여:
    /// - 능력치 성장 적용
    /// - 피로 누적
    /// - 부상 확률 판정 (확정적 — seed 기반, 추후 교체 가능)
    pub fn train_stat(
        &self,
        character: &mut Character,
        growth: &mut GrowthProfile,
        stat: StatType,
        requested_intensity: u32,
    ) -> Result<(TrainingOutcome, Vec<DomainEvent>), TrainingError> {
        let ctx = self.prepare_training(character, growth, requested_intensity)?;

        // --- 도메인 호출: 능력치 성장 ---
        let growth_event =
            growth.train_stat(stat, ctx.effective_intensity, character.life_stage());

        Ok(self.finalize_training(character, ctx, vec![growth_event.into()]))
    }

    /// 무공 연마(練磨)를 수행한다.
    ///
    /// `art_type`은 호출자가 MartialArt 정의에서 꺼내서 전달한다.
    pub fn train_art(
        &self,
        character: &mut Character,
        growth: &mut GrowthProfile,
        art_id: MartialArtId,
        art_type: MartialArtType,
        requested_intensity: u32,
    ) -> Result<(TrainingOutcome, Vec<DomainEvent>), TrainingError> {
        let ctx = self.prepare_training(character, growth, requested_intensity)?;

        // --- 도메인 호출: 무공 연마 ---
        let growth_event = growth
            .train_art(art_id, art_type, ctx.effective_intensity, character.life_stage())
            .map_err(|_| TrainingError::ArtNotLearned(art_id))?;

        Ok(self.finalize_training(character, ctx, vec![growth_event.into()]))
    }

    // --- Private helpers ---

    /// 수련 전 공통 계산: 수련 가능 여부, 최대 강도, over_limit, effective_intensity.
    ///
    /// train_stat/train_art 모두 동일한 전처리를 거치므로 여기에 한 번만 작성한다.
    fn prepare_training(
        &self,
        character: &Character,
        growth: &GrowthProfile,
        requested_intensity: u32,
    ) -> Result<TrainingContext, TrainingError> {
        // 1. 수련 가능 여부
        self.check_can_train(character)?;

        // 2. 최대 강도
        let vitality = growth.stats().get(StatType::Vitality);
        let endurance = growth.stats().get(StatType::Endurance);
        let willpower = growth.stats().get(StatType::Willpower);
        let fatigue_level = character.fatigue_level();

        let max_intensity =
            training::calculate_max_intensity(vitality, endurance, willpower, fatigue_level)
                .ok_or(TrainingError::Exhausted)?;

        if requested_intensity > max_intensity {
            return Err(TrainingError::IntensityTooHigh {
                requested: requested_intensity,
                max: max_intensity,
            });
        }

        // 3. over_limit 판정
        let base_max =
            training::calculate_base_max_intensity(vitality, endurance, fatigue_level)
                .unwrap_or(0);
        let over_limit = requested_intensity > base_max;

        // 4. 부상 페널티 → effective_intensity
        let injury_penalty = character
            .injury()
            .map(|inj| inj.intensity_penalty())
            .unwrap_or(0);
        let effective_intensity = requested_intensity.saturating_sub(injury_penalty).max(1);

        Ok(TrainingContext {
            requested_intensity,
            effective_intensity,
            over_limit,
            fatigue_level,
        })
    }

    /// 수련 후 공통 정리: 피로 누적, 부상 판정, 이벤트 수집.
    ///
    /// 도메인 호출(train_stat/train_art)에서 발생한 growth_events를 받아서
    /// 피로/부상 이벤트를 추가한 뒤 최종 결과를 반환한다.
    fn finalize_training(
        &self,
        character: &mut Character,
        ctx: TrainingContext,
        growth_events: Vec<DomainEvent>,
    ) -> (TrainingOutcome, Vec<DomainEvent>) {
        let mut all_events = growth_events;

        // 5. 피로 누적
        let fatigue_amount = training::calculate_fatigue_from_training(
            ctx.effective_intensity,
            ctx.fatigue_level,
        );
        let fatigue_events = character.add_fatigue(fatigue_amount);
        all_events.extend(fatigue_events);

        // 6. 부상 판정
        let injury_chance = training::calculate_injury_chance(
            ctx.requested_intensity,
            ctx.fatigue_level,
            ctx.over_limit,
        );
        let injury_occurred =
            Self::determine_injury(injury_chance, ctx.requested_intensity);

        if injury_occurred {
            let (injury_type, severity) =
                Self::determine_injury_type(ctx.requested_intensity);
            let injury_events = character.injure(injury_type, severity);
            all_events.extend(injury_events);
        }

        let outcome = TrainingOutcome {
            requested_intensity: ctx.requested_intensity,
            effective_intensity: ctx.effective_intensity,
            fatigue_gained: fatigue_amount,
            injury_occurred,
            over_limit: ctx.over_limit,
        };

        (outcome, all_events)
    }

    fn check_can_train(&self, character: &Character) -> Result<(), TrainingError> {
        if character.is_exhausted() {
            return Err(TrainingError::Exhausted);
        }
        if let Some(injury) = character.injury() {
            if injury.prevents_training() {
                return Err(TrainingError::InjuryPreventsTraining);
            }
        }
        Ok(())
    }

    /// 부상 발생 여부를 결정한다.
    ///
    /// 현재는 확정적(deterministic) 규칙:
    ///   injury_chance > 0.10 이고 intensity >= 7이면 부상 발생.
    ///
    /// 향후 RNG 기반으로 교체 예정.
    /// (테스트 가능성을 위해 별도 함수로 분리)
    fn determine_injury(injury_chance: f32, intensity: u32) -> bool {
        injury_chance > 0.10 && intensity >= 7
    }

    /// 강도에 따른 부상 유형과 심각도를 결정한다.
    fn determine_injury_type(intensity: u32) -> (InjuryType, InjurySeverity) {
        match intensity {
            0..=6 => (InjuryType::Bruise, InjurySeverity::Minor),
            7..=8 => (InjuryType::Strain, InjurySeverity::Major),
            _ => (InjuryType::Fracture, InjurySeverity::Critical), // 9+
        }
    }
}

impl Default for TrainingService {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::CharacterRole;
    use crate::growth::martial_art::MartialArtType;
    use crate::shared::id::CharacterId;

    fn make_character(id: u64, age: u32) -> Character {
        crate::test_fixtures::make_character(id, "테스트", age, CharacterRole::Npc)
    }

    fn make_growth(id: u64) -> GrowthProfile {
        use crate::growth::stat::StatBlock;
        // 체력 60, 인내 40, 의지 50 → 기본 max_intensity = 5
        GrowthProfile::new_with_stats(
            CharacterId::new(id),
            StatBlock {
                inner_power: 10, wisdom: 10, strategy: 10,
                vitality: 60, agility: 10, strength: 10,
                willpower: 50, endurance: 40, empathy: 10,
            },
        )
    }

    fn make_growth_high_willpower(id: u64) -> GrowthProfile {
        use crate::growth::stat::StatBlock;
        // 체력 60, 인내 40, 의지 80 → max = 6 (base 5 + will 1)
        GrowthProfile::new_with_stats(
            CharacterId::new(id),
            StatBlock {
                inner_power: 10, wisdom: 10, strategy: 10,
                vitality: 60, agility: 10, strength: 10,
                willpower: 80, endurance: 40, empathy: 10,
            },
        )
    }

    // ===================================================================
    // train_stat 기본
    // ===================================================================

    #[test]
    fn train_stat_basic_success() {
        let service = TrainingService::new();
        let mut character = make_character(1, 20);
        let mut growth = make_growth(1);

        let (outcome, events) = service
            .train_stat(&mut character, &mut growth, StatType::Strength, 4)
            .unwrap();

        assert_eq!(outcome.requested_intensity, 4);
        assert_eq!(outcome.effective_intensity, 4);
        assert!(outcome.fatigue_gained > 0);
        assert!(!outcome.injury_occurred);
        assert!(!outcome.over_limit);
        assert!(!events.is_empty(), "성장 + 피로 이벤트");
    }

    #[test]
    fn train_stat_increases_fatigue() {
        let service = TrainingService::new();
        let mut character = make_character(1, 20);
        let mut growth = make_growth(1);

        let old_fatigue = character.fatigue();
        service
            .train_stat(&mut character, &mut growth, StatType::Strength, 4)
            .unwrap();

        assert!(
            character.fatigue() > old_fatigue,
            "수련 후 피로 증가"
        );
    }

    #[test]
    fn train_stat_growth_applied() {
        let service = TrainingService::new();
        let mut character = make_character(1, 20);
        let mut growth = make_growth(1);

        let old_strength = growth.stats().get(StatType::Strength);
        service
            .train_stat(&mut character, &mut growth, StatType::Strength, 4)
            .unwrap();

        assert!(
            growth.stats().get(StatType::Strength) > old_strength,
            "무력이 올라야 한다"
        );
    }

    // ===================================================================
    // 수련 불가 상황
    // ===================================================================

    #[test]
    fn train_stat_exhausted_blocked() {
        let service = TrainingService::new();
        let mut character = make_character(1, 20);
        character.add_fatigue(100);
        let mut growth = make_growth(1);

        let result = service.train_stat(&mut character, &mut growth, StatType::Strength, 4);
        assert_eq!(result, Err(TrainingError::Exhausted));
    }

    #[test]
    fn train_stat_fracture_blocked() {
        let service = TrainingService::new();
        let mut character = make_character(1, 20);
        character.injure(InjuryType::Fracture, InjurySeverity::Critical);
        let mut growth = make_growth(1);

        let result = service.train_stat(&mut character, &mut growth, StatType::Strength, 4);
        assert_eq!(result, Err(TrainingError::InjuryPreventsTraining));
    }

    #[test]
    fn train_stat_qi_deviation_blocked() {
        let service = TrainingService::new();
        let mut character = make_character(1, 20);
        character.injure(InjuryType::QiDeviation, InjurySeverity::Critical);
        let mut growth = make_growth(1);

        let result = service.train_stat(&mut character, &mut growth, StatType::Strength, 1);
        assert_eq!(result, Err(TrainingError::InjuryPreventsTraining));
    }

    // ===================================================================
    // 최대 강도 제한
    // ===================================================================

    #[test]
    fn train_stat_over_max_intensity_rejected() {
        let service = TrainingService::new();
        let mut character = make_character(1, 20);
        let mut growth = make_growth(1); // max = 5

        let result = service.train_stat(&mut character, &mut growth, StatType::Strength, 6);
        assert_eq!(
            result,
            Err(TrainingError::IntensityTooHigh {
                requested: 6,
                max: 5
            })
        );
    }

    #[test]
    fn train_stat_at_exactly_max_intensity_ok() {
        let service = TrainingService::new();
        let mut character = make_character(1, 20);
        let mut growth = make_growth(1); // max = 5

        let result = service.train_stat(&mut character, &mut growth, StatType::Strength, 5);
        assert!(result.is_ok());
    }

    // ===================================================================
    // over_limit 판정
    // ===================================================================

    #[test]
    fn train_stat_not_over_limit_when_below_base() {
        let service = TrainingService::new();
        let mut character = make_character(1, 20);
        let mut growth = make_growth_high_willpower(1); // max=6, base=5

        let (outcome, _) = service
            .train_stat(&mut character, &mut growth, StatType::Strength, 5)
            .unwrap();

        assert!(!outcome.over_limit, "base 이하 → over_limit 아님");
    }

    #[test]
    fn train_stat_over_limit_when_using_willpower_bonus() {
        let service = TrainingService::new();
        let mut character = make_character(1, 20);
        let mut growth = make_growth_high_willpower(1); // max=6, base=5

        let (outcome, _) = service
            .train_stat(&mut character, &mut growth, StatType::Strength, 6)
            .unwrap();

        assert!(outcome.over_limit, "의지 보너스로 한계 초과 → over_limit");
    }

    // ===================================================================
    // 부상 페널티
    // ===================================================================

    #[test]
    fn train_stat_with_bruise_reduces_effective_intensity() {
        let service = TrainingService::new();
        let mut character = make_character(1, 20);
        character.injure(InjuryType::Bruise, InjurySeverity::Minor); // penalty 1
        let mut growth = make_growth(1);

        let (outcome, _) = service
            .train_stat(&mut character, &mut growth, StatType::Strength, 4)
            .unwrap();

        assert_eq!(outcome.effective_intensity, 3, "4 - 1(타박) = 3");
    }

    #[test]
    fn train_stat_with_strain_reduces_more() {
        let service = TrainingService::new();
        let mut character = make_character(1, 20);
        character.injure(InjuryType::Strain, InjurySeverity::Minor); // penalty 3
        let mut growth = make_growth(1);

        let (outcome, _) = service
            .train_stat(&mut character, &mut growth, StatType::Strength, 4)
            .unwrap();

        assert_eq!(outcome.effective_intensity, 1, "4 - 3(근육손상) = 1");
    }

    #[test]
    fn train_stat_effective_minimum_one() {
        let service = TrainingService::new();
        let mut character = make_character(1, 20);
        character.injure(InjuryType::Strain, InjurySeverity::Minor); // penalty 3
        let mut growth = make_growth(1);

        let (outcome, _) = service
            .train_stat(&mut character, &mut growth, StatType::Strength, 2)
            .unwrap();

        assert_eq!(outcome.effective_intensity, 1, "2 - 3 = 최소 1");
    }

    // ===================================================================
    // train_art 기본
    // ===================================================================

    #[test]
    fn train_art_basic_success() {
        let service = TrainingService::new();
        let mut character = make_character(1, 20);
        let mut growth = make_growth(1);

        let art_id = MartialArtId::new(1);
        growth.learn_art(art_id).unwrap();

        let (outcome, events) = service
            .train_art(&mut character, &mut growth, art_id, MartialArtType::WeaponArt, 4)
            .unwrap();

        assert_eq!(outcome.effective_intensity, 4);
        assert!(outcome.fatigue_gained > 0);
        assert!(!events.is_empty());
    }

    #[test]
    fn train_art_not_learned_error() {
        let service = TrainingService::new();
        let mut character = make_character(1, 20);
        let mut growth = make_growth(1);

        let result = service.train_art(
            &mut character,
            &mut growth,
            MartialArtId::new(99),
            MartialArtType::WeaponArt,
            4,
        );
        assert_eq!(result, Err(TrainingError::ArtNotLearned(MartialArtId::new(99))));
    }

    #[test]
    fn train_art_exhausted_blocked() {
        let service = TrainingService::new();
        let mut character = make_character(1, 20);
        character.add_fatigue(100);
        let mut growth = make_growth(1);

        let result = service.train_art(
            &mut character,
            &mut growth,
            MartialArtId::new(1),
            MartialArtType::WeaponArt,
            4,
        );
        assert_eq!(result, Err(TrainingError::Exhausted));
    }

    // ===================================================================
    // 부상 발생 (deterministic rule)
    // ===================================================================

    #[test]
    fn determine_injury_low_chance_low_intensity_no_injury() {
        assert!(!TrainingService::determine_injury(0.05, 4));
    }

    #[test]
    fn determine_injury_high_chance_high_intensity_triggers() {
        // chance > 0.10 && intensity >= 7
        assert!(TrainingService::determine_injury(0.15, 7));
    }

    #[test]
    fn determine_injury_high_chance_low_intensity_no_injury() {
        // chance > 0.10 but intensity < 7
        assert!(!TrainingService::determine_injury(0.15, 5));
    }

    #[test]
    fn determine_injury_type_low_intensity() {
        let (t, s) = TrainingService::determine_injury_type(5);
        assert_eq!(t, InjuryType::Bruise);
        assert_eq!(s, InjurySeverity::Minor);
    }

    #[test]
    fn determine_injury_type_medium_intensity() {
        let (t, s) = TrainingService::determine_injury_type(8);
        assert_eq!(t, InjuryType::Strain);
        assert_eq!(s, InjurySeverity::Major);
    }

    #[test]
    fn determine_injury_type_high_intensity() {
        let (t, s) = TrainingService::determine_injury_type(9);
        assert_eq!(t, InjuryType::Fracture);
        assert_eq!(s, InjurySeverity::Critical);
    }

    // ===================================================================
    // TrainingError display
    // ===================================================================

    #[test]
    fn error_display_exhausted() {
        assert!(TrainingError::Exhausted.to_string().contains("탈진"));
    }

    #[test]
    fn error_display_intensity_too_high() {
        let err = TrainingError::IntensityTooHigh {
            requested: 8,
            max: 5,
        };
        let s = err.to_string();
        assert!(s.contains("8"));
        assert!(s.contains("5"));
    }

    // ===================================================================
    // 무협 시나리오
    // ===================================================================

    #[test]
    fn scenario_daily_training_cycle() {
        let service = TrainingService::new();
        let mut character = make_character(1, 20);
        let mut growth = make_growth(1);

        // 3일간 매일 강도 4로 단련
        for day in 0..3 {
            let result = service.train_stat(
                &mut character,
                &mut growth,
                StatType::Strength,
                4,
            );
            // 피로가 누적되면 max_intensity가 줄어들 수 있음
            match result {
                Ok((outcome, _)) => {
                    assert!(outcome.fatigue_gained > 0, "day {}: 피로 증가", day);
                }
                Err(TrainingError::IntensityTooHigh { max, .. }) => {
                    // 피로 누적으로 최대 강도가 줄었음 — 정상
                    assert!(
                        max < 4,
                        "day {}: 최대 강도가 줄어서 실패 (max={})",
                        day,
                        max
                    );
                    break;
                }
                Err(e) => panic!("day {}: 예상치 못한 에러: {:?}", day, e),
            }
        }

        // 수련 후 피로가 누적되었어야 함
        assert!(
            character.fatigue() > 0,
            "3일 수련 후 피로 누적"
        );

        // 무력이 올라있어야 함
        assert!(
            growth.stats().get(StatType::Strength) > 10,
            "무력 성장"
        );
    }

    #[test]
    fn scenario_injured_warrior_limited_training() {
        let service = TrainingService::new();
        let mut character = make_character(1, 25);
        character.injure(InjuryType::Bruise, InjurySeverity::Minor);
        let mut growth = make_growth(1);

        // 타박상 — 수련 가능하지만 강도 -1
        let (outcome, _) = service
            .train_stat(&mut character, &mut growth, StatType::Strength, 4)
            .unwrap();

        assert_eq!(outcome.effective_intensity, 3, "타박 페널티로 강도 감소");
        assert!(outcome.fatigue_gained > 0);
    }
}
