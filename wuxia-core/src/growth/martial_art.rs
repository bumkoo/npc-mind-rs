// wuxia-core/src/growth/martial_art.rs
//
// Martial Art Types — 무공(武功)의 정의
//
// 무공은 "설계도"이다. 변하지 않는다.
// 변하는 것은 캐릭터의 숙련도(MartialArtProficiency, Step 3에서 구현).
//
// ┌────────────────────────────────────────────────────────┐
// │  MartialArtType (5가지 유형)                           │
// │                                                        │
// │  내공(Internal) ─── 기를 다루는 무공                    │
// │  외공(External) ─── 육체를 단련하는 무공                │
// │  병기(Weapon)   ─── 무기를 사용하는 무공                │
// │  경공(Light)    ─── 몸놀림의 무공                      │
// │  암기(Hidden)   ─── 숨겨진 무기의 무공                  │
// └────────────────────────────────────────────────────────┘
//
// ┌────────────────────────────────────────────────────────┐
// │  MasteryLevel (4단계 경지)                             │
// │                                                        │
// │  입문(0~29) → 숙련(30~59) → 통달(60~89) → 화경(90~100)│
// └────────────────────────────────────────────────────────┘
//
// ┌────────────────────────────────────────────────────────┐
// │  MartialArt (무공 정의 — Value Object)                 │
// │                                                        │
// │  독고구검: WeaponArt, 위력 85                          │
// │  name_key → TOML에서 "독고구검" / "Dugu Nine Swords"   │
// └────────────────────────────────────────────────────────┘

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::shared::i18n::Translatable;
use crate::shared::MartialArtId;

use super::stat::StatType;

// ---------------------------------------------------------------------------
// MartialArtType — 무공 유형 (5가지)
// ---------------------------------------------------------------------------

/// 무공의 다섯 가지 유형.
///
/// 각 유형은 수련 시 주로 관련되는 능력치가 다르다.
/// 연마(train_art) 시 부산물로 관련 능력치가 소폭 상승한다.
///
/// ```
/// use wuxia_core::growth::martial_art::MartialArtType;
/// use wuxia_core::growth::StatType;
///
/// let weapon = MartialArtType::WeaponArt;
/// let stats = weapon.related_stats();
/// assert_eq!(stats.len(), 3);
/// assert!(stats.contains(&StatType::Strength));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MartialArtType {
    /// 내공심법 — 기(氣)를 다루는 무공. 주 관련: 내공, 의지, 인내
    InternalArt,
    /// 외공 — 육체를 단련하는 무공. 주 관련: 체력, 무력, 인내
    ExternalArt,
    /// 병기 — 무기를 사용하는 무공. 주 관련: 무력, 체력, 경공
    WeaponArt,
    /// 경공 — 몸놀림의 무공. 주 관련: 경공, 체력, 내공
    LightArt,
    /// 암기 — 숨겨진 무기의 무공. 주 관련: 책략, 경공, 무력
    HiddenWeaponArt,
}

impl MartialArtType {
    /// 이 무공 유형의 주 관련 능력치 목록 (3개).
    ///
    /// 연마 시 부산물로 이 능력치들이 소폭 상승한다.
    /// 첫 번째가 가장 주된 관련 능력치.
    ///
    /// ```
    /// use wuxia_core::growth::martial_art::MartialArtType;
    /// use wuxia_core::growth::StatType;
    ///
    /// // 내공심법 → 내공, 의지, 인내
    /// let stats = MartialArtType::InternalArt.related_stats();
    /// assert_eq!(stats, &[StatType::InnerPower, StatType::Willpower, StatType::Endurance]);
    ///
    /// // 병기 → 무력, 체력, 경공
    /// let stats = MartialArtType::WeaponArt.related_stats();
    /// assert_eq!(stats, &[StatType::Strength, StatType::Vitality, StatType::Agility]);
    /// ```
    pub fn related_stats(&self) -> &[StatType] {
        match self {
            MartialArtType::InternalArt => &[
                StatType::InnerPower,
                StatType::Willpower,
                StatType::Endurance,
            ],
            MartialArtType::ExternalArt => &[
                StatType::Vitality,
                StatType::Strength,
                StatType::Endurance,
            ],
            MartialArtType::WeaponArt => &[
                StatType::Strength,
                StatType::Vitality,
                StatType::Agility,
            ],
            MartialArtType::LightArt => &[
                StatType::Agility,
                StatType::Vitality,
                StatType::InnerPower,
            ],
            MartialArtType::HiddenWeaponArt => &[
                StatType::Strategy,
                StatType::Agility,
                StatType::Strength,
            ],
        }
    }

    /// 모든 무공 유형을 배열로 반환한다.
    pub fn all() -> &'static [MartialArtType] {
        &[
            MartialArtType::InternalArt,
            MartialArtType::ExternalArt,
            MartialArtType::WeaponArt,
            MartialArtType::LightArt,
            MartialArtType::HiddenWeaponArt,
        ]
    }
}

impl Translatable for MartialArtType {
    fn translation_key(&self) -> &'static str {
        match self {
            MartialArtType::InternalArt => "martial_art_type.internal",
            MartialArtType::ExternalArt => "martial_art_type.external",
            MartialArtType::WeaponArt => "martial_art_type.weapon",
            MartialArtType::LightArt => "martial_art_type.light",
            MartialArtType::HiddenWeaponArt => "martial_art_type.hidden_weapon",
        }
    }
}

impl fmt::Display for MartialArtType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// ---------------------------------------------------------------------------
// MasteryLevel — 경지 (4단계)
// ---------------------------------------------------------------------------

/// 무공 숙련의 네 단계 경지.
///
/// 숙련도(0~100)에 따라 자동으로 결정된다.
/// 경지가 오를수록 무공의 실전 위력이 비약적으로 상승한다.
///
/// ```
/// use wuxia_core::growth::martial_art::MasteryLevel;
///
/// assert_eq!(MasteryLevel::from_proficiency(0), MasteryLevel::Beginner);
/// assert_eq!(MasteryLevel::from_proficiency(29), MasteryLevel::Beginner);
/// assert_eq!(MasteryLevel::from_proficiency(30), MasteryLevel::Proficient);
/// assert_eq!(MasteryLevel::from_proficiency(60), MasteryLevel::Mastered);
/// assert_eq!(MasteryLevel::from_proficiency(90), MasteryLevel::Transcendent);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MasteryLevel {
    /// 입문(入門) — 초식을 따라한다. 숙련도 0~29.
    Beginner,
    /// 숙련(熟練) — 초식이 자연스러워진다. 숙련도 30~59.
    Proficient,
    /// 통달(通達) — 초식을 넘어선 이해. 숙련도 60~89.
    Mastered,
    /// 화경(化境) — 무공이 몸의 일부. 숙련도 90~100.
    Transcendent,
}

/// 각 경지의 시작 숙련도 상수.
const PROFICIENT_THRESHOLD: u32 = 30;
const MASTERED_THRESHOLD: u32 = 60;
const TRANSCENDENT_THRESHOLD: u32 = 90;

impl MasteryLevel {
    /// 숙련도로부터 경지를 판정한다.
    pub fn from_proficiency(proficiency: u32) -> Self {
        match proficiency {
            0..PROFICIENT_THRESHOLD => MasteryLevel::Beginner,
            PROFICIENT_THRESHOLD..MASTERED_THRESHOLD => MasteryLevel::Proficient,
            MASTERED_THRESHOLD..TRANSCENDENT_THRESHOLD => MasteryLevel::Mastered,
            TRANSCENDENT_THRESHOLD.. => MasteryLevel::Transcendent,
        }
    }

    /// 이 경지에 진입하기 위한 최소 숙련도.
    ///
    /// ```
    /// use wuxia_core::growth::martial_art::MasteryLevel;
    ///
    /// assert_eq!(MasteryLevel::Beginner.threshold(), 0);
    /// assert_eq!(MasteryLevel::Proficient.threshold(), 30);
    /// assert_eq!(MasteryLevel::Mastered.threshold(), 60);
    /// assert_eq!(MasteryLevel::Transcendent.threshold(), 90);
    /// ```
    pub fn threshold(&self) -> u32 {
        match self {
            MasteryLevel::Beginner => 0,
            MasteryLevel::Proficient => PROFICIENT_THRESHOLD,
            MasteryLevel::Mastered => MASTERED_THRESHOLD,
            MasteryLevel::Transcendent => TRANSCENDENT_THRESHOLD,
        }
    }

    /// 모든 경지를 순서대로 반환한다.
    pub fn all() -> &'static [MasteryLevel] {
        &[
            MasteryLevel::Beginner,
            MasteryLevel::Proficient,
            MasteryLevel::Mastered,
            MasteryLevel::Transcendent,
        ]
    }
}

impl Translatable for MasteryLevel {
    fn translation_key(&self) -> &'static str {
        match self {
            MasteryLevel::Beginner => "mastery_level.beginner",
            MasteryLevel::Proficient => "mastery_level.proficient",
            MasteryLevel::Mastered => "mastery_level.mastered",
            MasteryLevel::Transcendent => "mastery_level.transcendent",
        }
    }
}

impl fmt::Display for MasteryLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// ---------------------------------------------------------------------------
// MartialArt — 무공 정의 (Value Object)
// ---------------------------------------------------------------------------

/// 하나의 무공을 정의하는 Value Object.
///
/// 무공 자체는 변하지 않는 "설계도"이다.
/// 캐릭터가 이 무공을 얼마나 잘 쓰는지는
/// `MartialArtProficiency`(Step 3)가 별도로 관리한다.
///
/// `name_key`는 i18n 번역 키로, TOML 파일에서 이름을 조회한다.
///
/// ```
/// use wuxia_core::shared::MartialArtId;
/// use wuxia_core::growth::martial_art::{MartialArt, MartialArtType};
/// use wuxia_core::growth::StatType;
///
/// let art = MartialArt::new(
///     MartialArtId::new(1),
///     "martial_art.dugu_nine_swords".to_string(),
///     MartialArtType::WeaponArt,
///     85,
/// );
///
/// assert_eq!(art.name_key(), "martial_art.dugu_nine_swords");
/// assert_eq!(art.art_type(), MartialArtType::WeaponArt);
/// assert_eq!(art.base_power(), 85);
/// assert!(art.related_stats().contains(&StatType::Strength));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MartialArt {
    id: MartialArtId,
    /// i18n 번역 키. TOML에서 이름 조회: tr.get(name_key) → "독고구검"
    name_key: String,
    art_type: MartialArtType,
    /// 기본 위력 (1~100). 숙련도와 능력치로 실전위력이 결정된다.
    base_power: u32,
}

impl MartialArt {
    /// 새 무공을 정의한다.
    ///
    /// # Arguments
    /// * `id` - 고유 식별자
    /// * `name_key` - i18n 번역 키 (예: "martial_art.dugu_nine_swords")
    /// * `art_type` - 무공 유형 (내공/외공/병기/경공/암기)
    /// * `base_power` - 기본 위력 (1~100)
    pub fn new(
        id: MartialArtId,
        name_key: String,
        art_type: MartialArtType,
        base_power: u32,
    ) -> Self {
        Self {
            id,
            name_key,
            art_type,
            base_power: base_power.clamp(1, 100),
        }
    }

    /// 고유 식별자.
    pub fn id(&self) -> MartialArtId {
        self.id
    }

    /// i18n 번역 키. `Translations::get(name_key)`로 이름 조회.
    pub fn name_key(&self) -> &str {
        &self.name_key
    }

    /// 무공 유형.
    pub fn art_type(&self) -> MartialArtType {
        self.art_type
    }

    /// 기본 위력 (1~100).
    pub fn base_power(&self) -> u32 {
        self.base_power
    }

    /// 이 무공의 주 관련 능력치 (art_type에 위임).
    pub fn related_stats(&self) -> &[StatType] {
        self.art_type.related_stats()
    }
}

impl fmt::Display for MartialArt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}, 위력 {})", self.name_key, self.art_type, self.base_power)
    }
}

// ---------------------------------------------------------------------------
// MartialArtProficiency — 캐릭터의 무공 숙련도
// ---------------------------------------------------------------------------

/// 캐릭터가 특정 무공을 얼마나 익혔는지 기록하는 Value Object.
///
/// GrowthProfile이 Vec으로 소유한다.
/// MartialArt(무공 정의)와 1:1로 연결되며,
/// 숙련도가 오르면 경지(MasteryLevel)가 자동으로 갱신된다.
///
/// ```
/// use wuxia_core::shared::MartialArtId;
/// use wuxia_core::growth::martial_art::{MartialArtProficiency, MasteryLevel};
///
/// let mut prof = MartialArtProficiency::new(MartialArtId::new(1));
/// assert_eq!(prof.proficiency(), 0);
/// assert_eq!(prof.mastery_level(), MasteryLevel::Beginner);
///
/// let new_mastery = prof.add_proficiency(35);
/// assert_eq!(prof.proficiency(), 35);
/// assert_eq!(new_mastery, MasteryLevel::Proficient);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MartialArtProficiency {
    martial_art_id: MartialArtId,
    proficiency: u32,
    mastery_level: MasteryLevel,
}

impl MartialArtProficiency {
    /// 새 무공 습득 (숙련도 0, 입문 경지).
    pub fn new(martial_art_id: MartialArtId) -> Self {
        Self {
            martial_art_id,
            proficiency: 0,
            mastery_level: MasteryLevel::Beginner,
        }
    }

    /// 숙련도를 증가시키고 새 경지를 반환한다.
    ///
    /// 숙련도는 0~100으로 clamp된다.
    /// 경지가 바뀌면 새 경지를 반환하므로, 호출자가 경지 돌파를 감지할 수 있다.
    pub fn add_proficiency(&mut self, amount: u32) -> MasteryLevel {
        self.proficiency = (self.proficiency + amount).min(100);
        self.mastery_level = MasteryLevel::from_proficiency(self.proficiency);
        self.mastery_level
    }

    /// 연결된 무공 ID.
    pub fn martial_art_id(&self) -> MartialArtId {
        self.martial_art_id
    }

    /// 현재 숙련도 (0~100).
    pub fn proficiency(&self) -> u32 {
        self.proficiency
    }

    /// 현재 경지.
    pub fn mastery_level(&self) -> MasteryLevel {
        self.mastery_level
    }
}

impl fmt::Display for MartialArtProficiency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (숙련도 {}, {:?})",
            self.martial_art_id, self.proficiency, self.mastery_level
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // =======================================================================
    // MartialArtType
    // =======================================================================

    #[test]
    fn art_type_related_stats_count() {
        // 모든 유형은 정확히 3개의 관련 능력치를 가진다
        for art_type in MartialArtType::all() {
            assert_eq!(
                art_type.related_stats().len(),
                3,
                "{:?} should have 3 related stats",
                art_type
            );
        }
    }

    #[test]
    fn art_type_internal_stats() {
        let stats = MartialArtType::InternalArt.related_stats();
        assert_eq!(stats[0], StatType::InnerPower);
        assert_eq!(stats[1], StatType::Willpower);
        assert_eq!(stats[2], StatType::Endurance);
    }

    #[test]
    fn art_type_external_stats() {
        let stats = MartialArtType::ExternalArt.related_stats();
        assert_eq!(stats[0], StatType::Vitality);
        assert_eq!(stats[1], StatType::Strength);
        assert_eq!(stats[2], StatType::Endurance);
    }

    #[test]
    fn art_type_weapon_stats() {
        let stats = MartialArtType::WeaponArt.related_stats();
        assert_eq!(stats[0], StatType::Strength);
        assert_eq!(stats[1], StatType::Vitality);
        assert_eq!(stats[2], StatType::Agility);
    }

    #[test]
    fn art_type_light_stats() {
        let stats = MartialArtType::LightArt.related_stats();
        assert_eq!(stats[0], StatType::Agility);
        assert_eq!(stats[1], StatType::Vitality);
        assert_eq!(stats[2], StatType::InnerPower);
    }

    #[test]
    fn art_type_hidden_weapon_stats() {
        let stats = MartialArtType::HiddenWeaponArt.related_stats();
        assert_eq!(stats[0], StatType::Strategy);
        assert_eq!(stats[1], StatType::Agility);
        assert_eq!(stats[2], StatType::Strength);
    }

    #[test]
    fn art_type_no_duplicates_in_related_stats() {
        // 관련 능력치에 중복이 없어야 한다
        for art_type in MartialArtType::all() {
            let stats = art_type.related_stats();
            for (i, s) in stats.iter().enumerate() {
                for (j, t) in stats.iter().enumerate() {
                    if i != j {
                        assert_ne!(s, t, "{:?} has duplicate stat {:?}", art_type, s);
                    }
                }
            }
        }
    }

    #[test]
    fn art_type_all_count() {
        assert_eq!(MartialArtType::all().len(), 5);
    }

    #[test]
    fn art_type_translation_keys() {
        assert_eq!(
            MartialArtType::InternalArt.translation_key(),
            "martial_art_type.internal"
        );
        assert_eq!(
            MartialArtType::WeaponArt.translation_key(),
            "martial_art_type.weapon"
        );
    }

    #[test]
    fn art_type_serialization_roundtrip() {
        let original = MartialArtType::WeaponArt;
        let json = serde_json::to_string(&original).unwrap();
        let restored: MartialArtType = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    // =======================================================================
    // MasteryLevel
    // =======================================================================

    #[test]
    fn mastery_from_proficiency_beginner() {
        assert_eq!(MasteryLevel::from_proficiency(0), MasteryLevel::Beginner);
        assert_eq!(MasteryLevel::from_proficiency(15), MasteryLevel::Beginner);
        assert_eq!(MasteryLevel::from_proficiency(29), MasteryLevel::Beginner);
    }

    #[test]
    fn mastery_from_proficiency_proficient() {
        assert_eq!(MasteryLevel::from_proficiency(30), MasteryLevel::Proficient);
        assert_eq!(MasteryLevel::from_proficiency(45), MasteryLevel::Proficient);
        assert_eq!(MasteryLevel::from_proficiency(59), MasteryLevel::Proficient);
    }

    #[test]
    fn mastery_from_proficiency_mastered() {
        assert_eq!(MasteryLevel::from_proficiency(60), MasteryLevel::Mastered);
        assert_eq!(MasteryLevel::from_proficiency(75), MasteryLevel::Mastered);
        assert_eq!(MasteryLevel::from_proficiency(89), MasteryLevel::Mastered);
    }

    #[test]
    fn mastery_from_proficiency_transcendent() {
        assert_eq!(MasteryLevel::from_proficiency(90), MasteryLevel::Transcendent);
        assert_eq!(MasteryLevel::from_proficiency(95), MasteryLevel::Transcendent);
        assert_eq!(MasteryLevel::from_proficiency(100), MasteryLevel::Transcendent);
    }

    #[test]
    fn mastery_threshold() {
        assert_eq!(MasteryLevel::Beginner.threshold(), 0);
        assert_eq!(MasteryLevel::Proficient.threshold(), 30);
        assert_eq!(MasteryLevel::Mastered.threshold(), 60);
        assert_eq!(MasteryLevel::Transcendent.threshold(), 90);
    }

    #[test]
    fn mastery_all_count() {
        assert_eq!(MasteryLevel::all().len(), 4);
    }

    #[test]
    fn mastery_translation_keys() {
        assert_eq!(
            MasteryLevel::Beginner.translation_key(),
            "mastery_level.beginner"
        );
        assert_eq!(
            MasteryLevel::Transcendent.translation_key(),
            "mastery_level.transcendent"
        );
    }

    #[test]
    fn mastery_serialization_roundtrip() {
        let original = MasteryLevel::Mastered;
        let json = serde_json::to_string(&original).unwrap();
        let restored: MasteryLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    // =======================================================================
    // MartialArt
    // =======================================================================

    #[test]
    fn martial_art_creation() {
        let art = MartialArt::new(
            MartialArtId::new(1),
            "martial_art.dugu_nine_swords".to_string(),
            MartialArtType::WeaponArt,
            85,
        );
        assert_eq!(art.id(), MartialArtId::new(1));
        assert_eq!(art.name_key(), "martial_art.dugu_nine_swords");
        assert_eq!(art.art_type(), MartialArtType::WeaponArt);
        assert_eq!(art.base_power(), 85);
    }

    #[test]
    fn martial_art_related_stats_delegates_to_type() {
        let art = MartialArt::new(
            MartialArtId::new(1),
            "martial_art.test".to_string(),
            MartialArtType::InternalArt,
            50,
        );
        assert_eq!(
            art.related_stats(),
            MartialArtType::InternalArt.related_stats()
        );
    }

    #[test]
    fn martial_art_base_power_clamped() {
        // 0 → 1 (최소)
        let art = MartialArt::new(
            MartialArtId::new(1),
            "martial_art.weak".to_string(),
            MartialArtType::ExternalArt,
            0,
        );
        assert_eq!(art.base_power(), 1);

        // 200 → 100 (최대)
        let art = MartialArt::new(
            MartialArtId::new(2),
            "martial_art.overpowered".to_string(),
            MartialArtType::ExternalArt,
            200,
        );
        assert_eq!(art.base_power(), 100);
    }

    #[test]
    fn martial_art_serialization_roundtrip() {
        let original = MartialArt::new(
            MartialArtId::new(42),
            "martial_art.taiji_sword".to_string(),
            MartialArtType::WeaponArt,
            70,
        );
        let json = serde_json::to_string(&original).unwrap();
        let restored: MartialArt = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn martial_art_display() {
        let art = MartialArt::new(
            MartialArtId::new(1),
            "martial_art.dugu_nine_swords".to_string(),
            MartialArtType::WeaponArt,
            85,
        );
        let display = art.to_string();
        assert!(display.contains("martial_art.dugu_nine_swords"));
        assert!(display.contains("WeaponArt"));
        assert!(display.contains("85"));
    }

    // =======================================================================
    // i18n 통합 테스트
    // =======================================================================

    #[test]
    fn martial_art_name_from_entries() {
        use crate::shared::i18n::{Locale, Translations};
        use std::collections::HashMap;

        let mut entries = HashMap::new();
        entries.insert("martial_art.dugu_nine_swords".into(), "독고구검".into());
        entries.insert("martial_art.taiji_sword".into(), "태극검법".into());
        let tr = Translations::from_entries(Locale::Ko, entries);

        let art = MartialArt::new(
            MartialArtId::new(1),
            "martial_art.dugu_nine_swords".to_string(),
            MartialArtType::WeaponArt,
            85,
        );

        assert_eq!(tr.get(art.name_key()), "독고구검");
    }

    #[test]
    fn martial_art_type_name_from_entries() {
        use crate::shared::i18n::{Locale, Translations};
        use std::collections::HashMap;

        let mut entries = HashMap::new();
        entries.insert("martial_art_type.internal".into(), "내공심법".into());
        entries.insert("martial_art_type.weapon".into(), "병기".into());
        let tr = Translations::from_entries(Locale::Ko, entries);

        assert_eq!(tr.translate(&MartialArtType::InternalArt), "내공심법");
        assert_eq!(tr.translate(&MartialArtType::WeaponArt), "병기");
    }

    #[test]
    fn mastery_level_name_from_entries() {
        use crate::shared::i18n::{Locale, Translations};
        use std::collections::HashMap;

        let mut entries = HashMap::new();
        entries.insert("mastery_level.beginner".into(), "입문".into());
        entries.insert("mastery_level.transcendent".into(), "화경".into());
        let tr = Translations::from_entries(Locale::Ko, entries);

        assert_eq!(tr.translate(&MasteryLevel::Beginner), "입문");
        assert_eq!(tr.translate(&MasteryLevel::Transcendent), "화경");
    }

    #[test]
    fn missing_martial_art_name_returns_key() {
        use crate::shared::i18n::{Locale, Translations};
        use std::collections::HashMap;

        // 빈 entries → 키 자체가 폴백
        let tr = Translations::from_entries(Locale::Ko, HashMap::new());

        let art = MartialArt::new(
            MartialArtId::new(1),
            "martial_art.unknown_art".to_string(),
            MartialArtType::WeaponArt,
            50,
        );

        assert_eq!(tr.get(art.name_key()), "martial_art.unknown_art");
    }

    // =======================================================================
    // MartialArtProficiency
    // =======================================================================

    #[test]
    fn proficiency_new_starts_at_zero() {
        let prof = MartialArtProficiency::new(MartialArtId::new(1));
        assert_eq!(prof.martial_art_id(), MartialArtId::new(1));
        assert_eq!(prof.proficiency(), 0);
        assert_eq!(prof.mastery_level(), MasteryLevel::Beginner);
    }

    #[test]
    fn proficiency_add_increases() {
        let mut prof = MartialArtProficiency::new(MartialArtId::new(1));
        let mastery = prof.add_proficiency(10);
        assert_eq!(prof.proficiency(), 10);
        assert_eq!(mastery, MasteryLevel::Beginner);
    }

    #[test]
    fn proficiency_add_accumulates() {
        let mut prof = MartialArtProficiency::new(MartialArtId::new(1));
        prof.add_proficiency(15);
        prof.add_proficiency(20);
        assert_eq!(prof.proficiency(), 35);
        assert_eq!(prof.mastery_level(), MasteryLevel::Proficient);
    }

    #[test]
    fn proficiency_mastery_updates_on_threshold() {
        let mut prof = MartialArtProficiency::new(MartialArtId::new(1));

        prof.add_proficiency(29);
        assert_eq!(prof.mastery_level(), MasteryLevel::Beginner);

        let mastery = prof.add_proficiency(1); // 29 → 30
        assert_eq!(prof.proficiency(), 30);
        assert_eq!(mastery, MasteryLevel::Proficient);
    }

    #[test]
    fn proficiency_clamped_at_100() {
        let mut prof = MartialArtProficiency::new(MartialArtId::new(1));
        prof.add_proficiency(200);
        assert_eq!(prof.proficiency(), 100);
        assert_eq!(prof.mastery_level(), MasteryLevel::Transcendent);
    }

    #[test]
    fn proficiency_all_mastery_transitions() {
        let mut prof = MartialArtProficiency::new(MartialArtId::new(1));

        prof.add_proficiency(30);
        assert_eq!(prof.mastery_level(), MasteryLevel::Proficient);

        prof.add_proficiency(30); // → 60
        assert_eq!(prof.mastery_level(), MasteryLevel::Mastered);

        prof.add_proficiency(30); // → 90
        assert_eq!(prof.mastery_level(), MasteryLevel::Transcendent);
    }

    #[test]
    fn proficiency_serialization_roundtrip() {
        let mut prof = MartialArtProficiency::new(MartialArtId::new(42));
        prof.add_proficiency(55);

        let json = serde_json::to_string(&prof).unwrap();
        let restored: MartialArtProficiency = serde_json::from_str(&json).unwrap();
        assert_eq!(prof, restored);
    }

    #[test]
    fn proficiency_display() {
        let mut prof = MartialArtProficiency::new(MartialArtId::new(1));
        prof.add_proficiency(45);
        let display = prof.to_string();
        assert!(display.contains("45"));
        assert!(display.contains("Proficient"));
    }
}
