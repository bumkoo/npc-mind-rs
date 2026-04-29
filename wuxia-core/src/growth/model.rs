// wuxia-core/src/growth/model.rs
//
// GrowthProfile — 캐릭터의 능력치 프로필
//
// 성장 도메인의 Aggregate Root.
// "이 사람은 얼마나 강한가?"에 대한 답을 가지고 있다.
//
// GrowthProfile은 CharacterId를 참조하여 캐릭터와 연결된다.
// 캐릭터 도메인은 "누구인가"를, 성장 도메인은 "얼마나 강한가"를 소유한다.
//
// 능력치 저장 규칙:
//   - 모든 값은 0~100 범위로 clamp된다
//   - 입력 시 초과값은 자동으로 100으로 잘린다
//   - 음수는 u32이므로 자연스럽게 0 이상
//
// 전투력 계산:
//   - combat_power = 무력 + 경공 + 내공 (기초 전투력, 맨몸)
//   - art_effective_power = base_power × 숙련도 × 관련능력치 (무공 실전위력)
//   - best_art_power = 가장 강한 무공의 실전위력
//   - total_stats  = 9개 능력치 합계 (종합 역량)
//   Phase 4(전투 도메인)에서 기초+실전 통합 전투력이 추가된다.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::character::LifeStage;
use crate::shared::id::{CharacterId, MartialArtId};

use super::event::{GrowthEvent, StatChange};
use super::martial_art::{MartialArtProficiency, MartialArtType};
use super::stat::{clamp_stat, StatBlock, StatCategory, StatType, STAT_MAX};
use super::{power, training, GrowthError};

// ---------------------------------------------------------------------------
// GrowthProfile (Aggregate Root)
// ---------------------------------------------------------------------------

/// 캐릭터의 능력치 프로필.
///
/// 성장 도메인의 Aggregate Root.
/// CharacterId로 캐릭터 도메인의 Character와 연결된다.
///
/// 모든 능력치는 0~100 범위로 유지된다.
///
/// # Example
/// ```
/// use wuxia_core::shared::CharacterId;
/// use wuxia_core::growth::{GrowthProfile, StatBlock, StatType, StatCategory};
///
/// // 기본 프로필 (모든 능력치 10)
/// let profile = GrowthProfile::new_default(CharacterId::new(1));
/// assert_eq!(profile.stat_value(StatType::Vitality), 10);
/// assert_eq!(profile.total_stats(), 90); // 10 × 9
///
/// // 커스텀 프로필
/// let warrior = GrowthProfile::new_with_stats(
///     CharacterId::new(2),
///     StatBlock {
///         inner_power: 50, wisdom: 30, strategy: 40,
///         vitality: 80, agility: 60, strength: 70,
///         willpower: 45, endurance: 55, empathy: 35,
///     },
/// );
/// assert_eq!(warrior.stat_value(StatType::Strength), 70);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthProfile {
    character_id: CharacterId,
    stats: StatBlock,
    learned_arts: Vec<MartialArtProficiency>,
}

impl GrowthProfile {
    /// 기본 프로필 생성 (모든 능력치 기본값 10, 습득 무공 없음).
    ///
    /// 일반 마을 주민이나 초기 캐릭터에 적합하다.
    pub fn new_default(character_id: CharacterId) -> Self {
        Self {
            character_id,
            stats: StatBlock::default_stats(),
            learned_arts: Vec::new(),
        }
    }

    /// 지정한 능력치로 프로필 생성 (습득 무공 없음).
    ///
    /// 100을 초과하는 값은 자동으로 100으로 clamp된다.
    ///
    /// 외부 파일에서 로드한 StatBlock을 그대로 넣을 수 있다:
    /// ```
    /// use wuxia_core::shared::CharacterId;
    /// use wuxia_core::growth::{GrowthProfile, StatBlock};
    ///
    /// let stats: StatBlock = serde_json::from_str(r#"{
    ///     "inner_power": 50, "wisdom": 30, "strategy": 40,
    ///     "vitality": 80, "agility": 60, "strength": 70,
    ///     "willpower": 45, "endurance": 55, "empathy": 35
    /// }"#).unwrap();
    ///
    /// let profile = GrowthProfile::new_with_stats(CharacterId::new(1), stats);
    /// assert_eq!(profile.stat_value(wuxia_core::growth::StatType::InnerPower), 50);
    /// ```
    pub fn new_with_stats(character_id: CharacterId, stats: StatBlock) -> Self {
        // 모든 값을 clamp하여 0~100 범위 보장
        let clamped = StatBlock {
            inner_power: clamp_stat(stats.inner_power),
            wisdom: clamp_stat(stats.wisdom),
            strategy: clamp_stat(stats.strategy),
            vitality: clamp_stat(stats.vitality),
            agility: clamp_stat(stats.agility),
            strength: clamp_stat(stats.strength),
            willpower: clamp_stat(stats.willpower),
            endurance: clamp_stat(stats.endurance),
            empathy: clamp_stat(stats.empathy),
        };

        Self {
            character_id,
            stats: clamped,
            learned_arts: Vec::new(),
        }
    }

    // --- Getters ---

    /// 이 프로필의 소유자 캐릭터 ID.
    pub fn character_id(&self) -> CharacterId {
        self.character_id
    }

    /// 특정 능력치 조회.
    pub fn stat_value(&self, stat: StatType) -> u32 {
        self.stats.get(stat)
    }

    /// 내부 StatBlock 참조.
    pub fn stats(&self) -> &StatBlock {
        &self.stats
    }

    /// 특정 범주의 능력치 합계.
    ///
    /// ```
    /// use wuxia_core::shared::CharacterId;
    /// use wuxia_core::growth::{GrowthProfile, StatCategory};
    ///
    /// let profile = GrowthProfile::new_default(CharacterId::new(1));
    /// assert_eq!(profile.category_total(StatCategory::Intellectual), 30); // 10+10+10
    /// ```
    pub fn category_total(&self, category: StatCategory) -> u32 {
        category
            .stats()
            .iter()
            .map(|s| self.stats.get(*s))
            .sum()
    }

    /// 기초 전투력: 무력 + 경공 + 내공.
    ///
    /// 순수 능력치 기반의 전투력. 무공 없이 "맨몸으로 얼마나 강한가".
    /// 무공의 실전위력은 `art_effective_power()`로 별도 계산한다.
    ///
    /// Phase 4(전투 도메인)에서 기초 전투력 + 실전위력을 통합한
    /// 종합 전투력이 추가될 예정이다.
    pub fn combat_power(&self) -> u32 {
        power::calculate_combat_power(&self.stats)
    }

    /// 9개 능력치 총합.
    ///
    /// 캐릭터의 종합 역량을 나타내는 단순 지표.
    pub fn total_stats(&self) -> u32 {
        self.stats.total()
    }

    // --- Commands (상태 변경) --- [Iteration 2.2]

    /// 단련(鍛鍊)한다. 능력치 성장량을 계산하고 즉시 적용한다.
    ///
    /// `intensity`는 단련 강도 (1~10 권장).
    /// 실제 성장량은 LifeStage에 따라 달라진다:
    ///   청년(1.5x) > 장년(1.0x) > 중년(0.7x) > 노년(0.3x)
    ///
    /// 반환: GrowthEvent::StatTrained (어떤 능력치가 얼마나 변했는지 기록)
    ///
    /// # Example
    /// ```
    /// use wuxia_core::shared::CharacterId;
    /// use wuxia_core::character::LifeStage;
    /// use wuxia_core::growth::{GrowthProfile, StatType, GrowthEvent};
    ///
    /// let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    /// let event = profile.train_stat(StatType::Strength, 4, LifeStage::Youth);
    ///
    /// // 기본값 10 + round(4 × 1.5) = 10 + 6 = 16
    /// assert_eq!(profile.stat_value(StatType::Strength), 16);
    /// ```
    pub fn train_stat(
        &mut self,
        stat: StatType,
        intensity: u32,
        life_stage: LifeStage,
    ) -> GrowthEvent {
        let change = training::calculate_stat_training(stat, intensity, life_stage);
        self.apply_stat_change(&change);

        GrowthEvent::StatTrained {
            character_id: self.character_id,
            changes: vec![change],
        }
    }

    // --- 무공 관련 Commands --- [Iteration 2.3 Step 3]

    /// 무공을 습득한다 (숙련도 0에서 시작).
    ///
    /// 이미 습득한 무공이면 GrowthError::ArtAlreadyLearned를 반환한다.
    ///
    /// # Example
    /// ```
    /// use wuxia_core::shared::{CharacterId, MartialArtId};
    /// use wuxia_core::growth::GrowthProfile;
    ///
    /// let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    /// assert!(profile.learn_art(MartialArtId::new(1)).is_ok());
    /// assert!(profile.learn_art(MartialArtId::new(1)).is_err()); // 중복
    /// ```
    pub fn learn_art(&mut self, martial_art_id: MartialArtId) -> Result<(), GrowthError> {
        if self.learned_arts.iter().any(|p| p.martial_art_id() == martial_art_id) {
            return Err(GrowthError::ArtAlreadyLearned(martial_art_id));
        }
        self.learned_arts.push(MartialArtProficiency::new(martial_art_id));
        Ok(())
    }

    /// 무공을 연마(鍊磨)한다. 숙련도 상승 + 부산물 능력치 변화.
    ///
    /// `art_type`은 Application Service가 MartialArt에서 꺼내서 전달한다.
    ///
    /// 반환: GrowthEvent::ArtPracticed (숙련도, 경지, 부산물 기록)
    ///
    /// # Example
    /// ```
    /// use wuxia_core::shared::{CharacterId, MartialArtId};
    /// use wuxia_core::character::LifeStage;
    /// use wuxia_core::growth::{GrowthProfile, GrowthEvent};
    /// use wuxia_core::growth::martial_art::MartialArtType;
    ///
    /// let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    /// let art_id = MartialArtId::new(1);
    /// profile.learn_art(art_id).unwrap();
    ///
    /// let event = profile.train_art(art_id, MartialArtType::WeaponArt, 4, LifeStage::Youth).unwrap();
    /// match event {
    ///     GrowthEvent::ArtPracticed { proficiency_gain, .. } => {
    ///         assert!(proficiency_gain > 0);
    ///     }
    ///     _ => panic!("Expected ArtPracticed"),
    /// }
    /// ```
    pub fn train_art(
        &mut self,
        martial_art_id: MartialArtId,
        art_type: MartialArtType,
        intensity: u32,
        life_stage: LifeStage,
    ) -> Result<GrowthEvent, GrowthError> {
        // 습득 여부 확인
        let prof = self
            .learned_arts
            .iter_mut()
            .find(|p| p.martial_art_id() == martial_art_id)
            .ok_or(GrowthError::ArtNotLearned(martial_art_id))?;

        let old_mastery = prof.mastery_level();

        // 연마 계산
        let result = training::calculate_art_training(art_type, intensity, life_stage);

        // 숙련도 적용
        let new_mastery = prof.add_proficiency(result.proficiency_gain);
        let new_proficiency = prof.proficiency();

        // 부산물 능력치 적용
        for change in &result.stat_side_effects {
            self.apply_stat_change(change);
        }

        Ok(GrowthEvent::ArtPracticed {
            character_id: self.character_id,
            martial_art_id,
            proficiency_gain: result.proficiency_gain,
            new_proficiency,
            old_mastery,
            new_mastery,
            stat_changes: result.stat_side_effects,
        })
    }

    /// 특정 무공의 숙련도 조회.
    pub fn art_proficiency(&self, martial_art_id: MartialArtId) -> Option<&MartialArtProficiency> {
        self.learned_arts.iter().find(|p| p.martial_art_id() == martial_art_id)
    }

    /// 습득한 무공 목록.
    pub fn learned_arts(&self) -> &[MartialArtProficiency] {
        &self.learned_arts
    }

    // --- 실전위력 계산 --- [Iteration 2.3 Step 4]

    /// 습득한 무공 하나의 실전위력을 계산한다.
    ///
    /// `base_power`와 `art_type`은 MartialArt(정의)에서 가져온다.
    /// Application Service가 MartialArt를 조회하여 전달한다.
    ///
    /// 습득하지 않은 무공이면 0을 반환한다.
    ///
    /// # Example
    /// ```
    /// use wuxia_core::shared::{CharacterId, MartialArtId};
    /// use wuxia_core::character::LifeStage;
    /// use wuxia_core::growth::GrowthProfile;
    /// use wuxia_core::growth::martial_art::MartialArtType;
    ///
    /// let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    /// let art_id = MartialArtId::new(1);
    ///
    /// // 습득 전: 0
    /// assert_eq!(profile.art_effective_power(art_id, 85, MartialArtType::WeaponArt), 0);
    ///
    /// // 습득 후 연마
    /// profile.learn_art(art_id).unwrap();
    /// profile.train_art(art_id, MartialArtType::WeaponArt, 4, LifeStage::Youth).unwrap();
    /// assert!(profile.art_effective_power(art_id, 85, MartialArtType::WeaponArt) > 0);
    /// ```
    pub fn art_effective_power(
        &self,
        martial_art_id: MartialArtId,
        base_power: u32,
        art_type: MartialArtType,
    ) -> u32 {
        let proficiency = match self.art_proficiency(martial_art_id) {
            Some(p) => p.proficiency(),
            None => return 0,
        };

        power::calculate_art_effective_power(&self.stats, proficiency, base_power, art_type)
    }

    /// 습득한 무공 중 가장 강한 실전위력을 반환한다.
    ///
    /// `arts`는 (MartialArtId, base_power, MartialArtType) 튜플 슬라이스.
    /// Application Service가 MartialArt 목록에서 구성하여 전달한다.
    ///
    /// 무공이 없거나 모두 미습득이면 0.
    ///
    /// # Example
    /// ```
    /// use wuxia_core::shared::{CharacterId, MartialArtId};
    /// use wuxia_core::character::LifeStage;
    /// use wuxia_core::growth::GrowthProfile;
    /// use wuxia_core::growth::martial_art::MartialArtType;
    ///
    /// let mut profile = GrowthProfile::new_default(CharacterId::new(1));
    /// let art1 = MartialArtId::new(1);
    /// let art2 = MartialArtId::new(2);
    /// profile.learn_art(art1).unwrap();
    /// profile.learn_art(art2).unwrap();
    ///
    /// // 연마
    /// profile.train_art(art1, MartialArtType::WeaponArt, 4, LifeStage::Youth).unwrap();
    /// profile.train_art(art2, MartialArtType::InternalArt, 2, LifeStage::Youth).unwrap();
    ///
    /// let arts = [
    ///     (art1, 85, MartialArtType::WeaponArt),
    ///     (art2, 60, MartialArtType::InternalArt),
    /// ];
    /// let best = profile.best_art_power(&arts);
    /// assert!(best > 0);
    /// ```
    pub fn best_art_power(
        &self,
        arts: &[(MartialArtId, u32, MartialArtType)],
    ) -> u32 {
        power::calculate_best_art_power(&self.stats, &self.learned_arts, arts)
    }

    /// 연간 노화를 적용한다.
    ///
    /// 매년 YearPassed 이벤트에 반응하여 호출된다.
    /// LifeStage에 따라 자동으로 능력치가 변동된다:
    ///   청년: 체력/경공/무력 성장
    ///   장년: 내공/지혜/책략 성장
    ///   중년: 체력 쇠퇴, 지혜 성장
    ///   노년: 전반적 쇠퇴, 지혜/책략 약간 성장
    ///
    /// # Example
    /// ```
    /// use wuxia_core::shared::CharacterId;
    /// use wuxia_core::character::LifeStage;
    /// use wuxia_core::growth::{GrowthProfile, StatBlock, StatType};
    ///
    /// let mut profile = GrowthProfile::new_with_stats(
    ///     CharacterId::new(1),
    ///     StatBlock {
    ///         inner_power: 50, wisdom: 80, strategy: 60,
    ///         vitality: 30, agility: 25, strength: 20,
    ///         willpower: 70, endurance: 65, empathy: 75,
    ///     },
    /// );
    ///
    /// // 노년 노화: 체력 -2, 경공 -2, 무력 -1, 지혜 +1, 책략 +1, 인내 -1
    /// let event = profile.apply_yearly_aging(LifeStage::Elder);
    /// assert_eq!(profile.stat_value(StatType::Vitality), 28);  // 30 - 2
    /// assert_eq!(profile.stat_value(StatType::Wisdom), 81);    // 80 + 1
    /// ```
    pub fn apply_yearly_aging(
        &mut self,
        life_stage: LifeStage,
    ) -> GrowthEvent {
        let changes = training::calculate_yearly_aging(life_stage);
        for change in &changes {
            self.apply_stat_change(change);
        }

        GrowthEvent::YearlyAgingApplied {
            character_id: self.character_id,
            life_stage,
            changes,
        }
    }

    /// StatChange 하나를 능력치에 적용한다 (내부 헬퍼).
    ///
    /// delta가 양수면 증가, 음수면 감소.
    /// 결과는 항상 0~100 범위로 clamp된다.
    fn apply_stat_change(&mut self, change: &StatChange) {
        let current = self.stats.get(change.stat());
        let new_value = if change.delta() >= 0 {
            // 성장: u32 overflow 방지를 위해 clamp 후 더하기
            clamp_stat(current.saturating_add(change.delta() as u32))
        } else {
            // 쇠퇴: underflow 방지 (0 이하 방지)
            current.saturating_sub(change.delta().unsigned_abs())
        };
        self.stats.set(change.stat(), new_value.min(STAT_MAX));
    }
}

impl fmt::Display for GrowthProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // e.g., "[Growth Char-1] InnerPower:50 Wisdom:30 Strategy:40 | Vitality:80 Agility:60 Strength:70 | Willpower:45 Endurance:55 Empathy:35 (total:465, combat:180)"
        write!(
            f,
            "[Growth {}] {}:{} {}:{} {}:{} | {}:{} {}:{} {}:{} | {}:{} {}:{} {}:{} (total:{}, combat:{})",
            self.character_id,
            StatType::InnerPower, self.stats.inner_power,
            StatType::Wisdom, self.stats.wisdom,
            StatType::Strategy, self.stats.strategy,
            StatType::Vitality, self.stats.vitality,
            StatType::Agility, self.stats.agility,
            StatType::Strength, self.stats.strength,
            StatType::Willpower, self.stats.willpower,
            StatType::Endurance, self.stats.endurance,
            StatType::Empathy, self.stats.empathy,
            self.total_stats(),
            self.combat_power(),
        )
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
