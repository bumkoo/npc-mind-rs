// wuxia-core/src/character/model.rs
//
// Character Entity — the foundational identity of every being in the wuxia world.
//
// In DDD terms, Character is an Aggregate Root for the character domain.
// It owns the most basic identity data: who you ARE, not what you CAN DO.
//
// [리팩터링 v2] Character::age_one_year()는 이제 CharacterEvent를 생성하고
// DomainEvent::Character(...)로 감싸서 반환한다.
//
// What Character owns:
//   ✓ Name, courtesy name (字)
//   ✓ Gender, age, birth year
//   ✓ Role (Player / NPC / Companion)
//   ✓ Life stage transitions
//
// What Character does NOT own (other domains handle these):
//   ✗ Stats, martial arts     → Growth domain (Iteration 2.1)
//   ✗ Emotions, personality   → Psychology domain (Iteration 2.3)
//   ✗ Relationships           → Relationship domain (Iteration 3.1)
//   ✗ Sect membership         → World domain (Iteration 3.3)

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::shared::event::DomainEvent;
use crate::shared::id::CharacterId;

use super::event::CharacterEvent;
use super::fatigue::{FatigueLevel, DAILY_REST_RECOVERY, FATIGUE_MAX, FATIGUE_MIN};
use super::gender::Gender;
use super::injury::{Injury, InjurySeverity, InjuryType};
use super::life_stage::LifeStage;
use super::role::CharacterRole;

// ---------------------------------------------------------------------------
// Character (Aggregate Root)
// ---------------------------------------------------------------------------

/// A character in the wuxia world.
///
/// This is the **Aggregate Root** of the character domain.
/// All fields are private — access through methods to enforce invariants.
///
/// # Example
/// ```
/// use wuxia_core::shared::CharacterId;
/// use wuxia_core::character::{Character, Gender, CharacterRole, LifeStage};
///
/// let linghu_chong = Character::new(
///     CharacterId::new(1),
///     "令狐冲".to_string(),
///     Some("冲虚".to_string()),
///     Gender::Male,
///     1180,  // birth year
///     25,    // starting age
///     CharacterRole::Npc,
/// );
///
/// assert_eq!(linghu_chong.name(), "令狐冲");
/// assert_eq!(linghu_chong.age(), 25);
/// assert_eq!(linghu_chong.life_stage(), LifeStage::Youth);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    id: CharacterId,
    name: String,
    courtesy_name: Option<String>,  // 字 (zi)
    gender: Gender,
    birth_year: u32,
    current_age: u32,
    role: CharacterRole,
    #[serde(default)]
    fatigue: u32, // 0~100, [v2.3A] defaults to 0 for backward compat
    #[serde(default)]
    injury: Option<Injury>, // [v2.3A] None = 부상 없음
}

impl Character {
    // --- Construction ---

    /// Create a new character with all required fields.
    pub fn new(
        id: CharacterId,
        name: String,
        courtesy_name: Option<String>,
        gender: Gender,
        birth_year: u32,
        starting_age: u32,
        role: CharacterRole,
    ) -> Self {
        Self {
            id,
            name,
            courtesy_name,
            gender,
            birth_year,
            current_age: starting_age,
            role,
            fatigue: FATIGUE_MIN,
            injury: None,
        }
    }

    // --- Getters ---

    pub fn id(&self) -> CharacterId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn courtesy_name(&self) -> Option<&str> {
        self.courtesy_name.as_deref()
    }

    pub fn gender(&self) -> Gender {
        self.gender
    }

    pub fn birth_year(&self) -> u32 {
        self.birth_year
    }

    pub fn age(&self) -> u32 {
        self.current_age
    }

    pub fn role(&self) -> CharacterRole {
        self.role
    }

    /// Current life stage, derived from age.
    pub fn life_stage(&self) -> LifeStage {
        LifeStage::from_age(self.current_age)
    }

    /// Check if the character is alive.
    /// For now, all characters are alive. Death will be added later.
    pub fn is_alive(&self) -> bool {
        true
    }

    /// Current fatigue level (0~100). [v2.3A]
    pub fn fatigue(&self) -> u32 {
        self.fatigue
    }

    /// Current fatigue level category. [v2.3A]
    pub fn fatigue_level(&self) -> FatigueLevel {
        FatigueLevel::from_fatigue(self.fatigue)
    }

    /// Is this character exhausted (탈진)? [v2.3A]
    /// Exhausted characters cannot train.
    pub fn is_exhausted(&self) -> bool {
        self.fatigue_level() == FatigueLevel::Exhausted
    }

    /// Current injury, if any. [v2.3A]
    pub fn injury(&self) -> Option<&Injury> {
        self.injury.as_ref()
    }

    /// Is this character injured? [v2.3A]
    pub fn is_injured(&self) -> bool {
        self.injury.is_some()
    }

    /// Can this character train? [v2.3A]
    /// Blocked by: exhaustion OR injury that prevents training.
    pub fn can_train(&self) -> bool {
        if self.is_exhausted() {
            return false;
        }
        match &self.injury {
            Some(injury) => !injury.prevents_training(),
            None => true,
        }
    }

    // --- Commands (state changes) ---

    /// Age the character by one year.
    ///
    /// Returns a list of domain events that occurred:
    /// - Always: `CharacterEvent::Aged` (wrapped in DomainEvent)
    /// - If life stage changed: `CharacterEvent::LifeStageChanged`
    ///
    /// This is the ONLY way to change a character's age.
    /// The game clock calls this once per year for every living character.
    ///
    /// # Example
    /// ```
    /// use wuxia_core::shared::{CharacterId, DomainEvent};
    /// use wuxia_core::character::{Character, Gender, CharacterRole, CharacterEvent};
    ///
    /// let mut hero = Character::new(
    ///     CharacterId::new(1), "Test".into(), None,
    ///     Gender::Male, 1168, 32, CharacterRole::Player,
    /// );
    ///
    /// // Age 32 → 33: Youth → Prime transition!
    /// let events = hero.age_one_year();
    /// assert_eq!(hero.age(), 33);
    /// assert_eq!(events.len(), 2); // Aged + LifeStageChanged
    /// ```
    pub fn age_one_year(&mut self) -> Vec<DomainEvent> {
        let old_stage = self.life_stage();
        self.current_age += 1;
        let new_stage = self.life_stage();

        let mut events = vec![
            CharacterEvent::Aged {
                character_id: self.id,
                new_age: self.current_age,
            }.into()
        ];

        if old_stage != new_stage {
            events.push(
                CharacterEvent::LifeStageChanged {
                    character_id: self.id,
                    from: old_stage,
                    to: new_stage,
                }.into()
            );
        }

        events
    }

    /// 피로를 추가한다. 0~100 범위로 clamped. [v2.3A]
    ///
    /// 수련, 전투, 여행 등 활동 후 호출.
    /// 피로가 변하지 않으면 이벤트를 발생시키지 않는다.
    ///
    /// # Example
    /// ```
    /// use wuxia_core::shared::CharacterId;
    /// use wuxia_core::character::{Character, Gender, CharacterRole, FatigueLevel};
    ///
    /// let mut hero = Character::new(
    ///     CharacterId::new(1), "Test".into(), None,
    ///     Gender::Male, 1180, 20, CharacterRole::Player,
    /// );
    /// assert_eq!(hero.fatigue(), 0);
    ///
    /// let events = hero.add_fatigue(25);
    /// assert_eq!(hero.fatigue(), 25);
    /// assert_eq!(hero.fatigue_level(), FatigueLevel::Mild);
    /// assert_eq!(events.len(), 1);
    /// ```
    pub fn add_fatigue(&mut self, amount: u32) -> Vec<DomainEvent> {
        let old = self.fatigue;
        self.fatigue = (self.fatigue.saturating_add(amount)).min(FATIGUE_MAX);

        if old == self.fatigue {
            return Vec::new();
        }

        vec![CharacterEvent::FatigueChanged {
            character_id: self.id,
            old_fatigue: old,
            new_fatigue: self.fatigue,
            fatigue_level: self.fatigue_level(),
        }
        .into()]
    }

    /// 피로를 회복한다. 0~100 범위로 clamped. [v2.3A]
    ///
    /// 휴식, 아이템 사용, 위치 보너스 등으로 호출.
    ///
    /// # Example
    /// ```
    /// use wuxia_core::shared::CharacterId;
    /// use wuxia_core::character::{Character, Gender, CharacterRole, FatigueLevel};
    ///
    /// let mut hero = Character::new(
    ///     CharacterId::new(1), "Test".into(), None,
    ///     Gender::Male, 1180, 20, CharacterRole::Player,
    /// );
    /// hero.add_fatigue(50);
    ///
    /// let events = hero.recover_fatigue(20);
    /// assert_eq!(hero.fatigue(), 30);
    /// assert_eq!(hero.fatigue_level(), FatigueLevel::Mild);
    /// ```
    pub fn recover_fatigue(&mut self, amount: u32) -> Vec<DomainEvent> {
        let old = self.fatigue;
        self.fatigue = self.fatigue.saturating_sub(amount).max(FATIGUE_MIN);

        if old == self.fatigue {
            return Vec::new();
        }

        vec![CharacterEvent::FatigueChanged {
            character_id: self.id,
            old_fatigue: old,
            new_fatigue: self.fatigue,
            fatigue_level: self.fatigue_level(),
        }
        .into()]
    }

    /// 하루 수면 회복 (-5). [v2.3A]
    ///
    /// 매일 Night 시간대에 수면으로 피로가 자연 회복된다.
    /// DayPassed 이벤트 처리 시 Application Service가 호출.
    pub fn daily_rest_recovery(&mut self) -> Vec<DomainEvent> {
        self.recover_fatigue(DAILY_REST_RECOVERY)
    }

    /// 부상을 입힌다. [v2.3A]
    ///
    /// 기존 부상이 있으면 더 심각한 쪽으로 교체된다.
    /// ("설상가상" — 부상 위에 또 부상)
    ///
    /// # Example
    /// ```
    /// use wuxia_core::shared::CharacterId;
    /// use wuxia_core::character::{Character, Gender, CharacterRole};
    /// use wuxia_core::character::injury::{InjuryType, InjurySeverity};
    ///
    /// let mut hero = Character::new(
    ///     CharacterId::new(1), "Test".into(), None,
    ///     Gender::Male, 1180, 20, CharacterRole::Player,
    /// );
    ///
    /// let events = hero.injure(InjuryType::Strain, InjurySeverity::Major);
    /// assert!(hero.is_injured());
    /// assert_eq!(events.len(), 1);
    /// ```
    pub fn injure(
        &mut self,
        injury_type: InjuryType,
        severity: InjurySeverity,
    ) -> Vec<DomainEvent> {
        self.injury = Some(Injury::new(injury_type, severity));

        vec![CharacterEvent::Injured {
            character_id: self.id,
            injury_type,
            severity,
        }
        .into()]
    }

    /// 하루 자연 치유를 진행한다. [v2.3A]
    ///
    /// 부상의 남은 일수를 1 줄인다.
    /// 완치되면 injury를 None으로 설정하고 InjuryHealed 이벤트를 발생.
    /// 부상이 없으면 아무 일도 안 일어난다.
    pub fn heal_daily(&mut self) -> Vec<DomainEvent> {
        let Some(current) = &self.injury else {
            return Vec::new();
        };

        let healed = current.after_daily_heal();
        if healed.is_healed() {
            let injury_type = current.injury_type();
            self.injury = None;
            vec![CharacterEvent::InjuryHealed {
                character_id: self.id,
                injury_type,
            }
            .into()]
        } else {
            self.injury = Some(healed);
            Vec::new()
        }
    }

    /// 의원/동료 치료로 치유를 가속한다. [v2.3A]
    ///
    /// 남은 일수를 days_reduced만큼 줄인다.
    /// 완치되면 InjuryHealed 이벤트 발생.
    pub fn treat_injury(&mut self, days_reduced: u32) -> Vec<DomainEvent> {
        let Some(current) = &self.injury else {
            return Vec::new();
        };

        let treated = current.after_treatment(days_reduced);
        if treated.is_healed() {
            let injury_type = current.injury_type();
            self.injury = None;
            vec![CharacterEvent::InjuryHealed {
                character_id: self.id,
                injury_type,
            }
            .into()]
        } else {
            self.injury = Some(treated);
            Vec::new()
        }
    }
}

impl fmt::Display for Character {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // e.g., "[Char-1] 令狐冲 (Male, 25, Youth, NPC)"
        write!(
            f,
            "[{}] {} ({}, age {}, {}, {})",
            self.id,
            self.name,
            self.gender,
            self.current_age,
            self.life_stage(),
            self.role,
        )
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
