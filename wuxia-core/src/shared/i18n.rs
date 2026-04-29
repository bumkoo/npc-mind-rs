// wuxia-core/src/shared/i18n.rs
//
// Internationalization — domain types don't know what language they're called.
//
// The wuxia world is universal; "내공" and "Inner Power" and "内功"
// are all names for the same thing. Domain enums carry only a
// translation KEY, and this module resolves keys to localized strings.
//
// Design:
//   Translatable  — trait for any type that has a translation key
//   Locale        — supported languages
//   Translations  — key→string store (포맷 중립: 파싱은 wuxia-data가 담당)
//
// Usage (wuxia-data와 함께):
//   // wuxia-data에서 TOML 파싱
//   let entries = wuxia_data::loader::load_translations_toml(toml_str)?;
//   // wuxia-core에서 Translations 생성
//   let tr = Translations::from_entries(Locale::Ko, entries);
//   let name = tr.get(StatType::InnerPower.translation_key());
//   // → "내공"

use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// Locale
// ---------------------------------------------------------------------------

/// Supported languages.
///
/// ```
/// use wuxia_core::shared::i18n::Locale;
///
/// assert_eq!(Locale::Ko.code(), "ko");
/// assert_eq!(Locale::En.code(), "en");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    /// 한국어
    Ko,
    /// English
    En,
}

impl Locale {
    /// IETF language code.
    pub fn code(&self) -> &'static str {
        match self {
            Locale::Ko => "ko",
            Locale::En => "en",
        }
    }

    /// All supported locales.
    pub fn all() -> &'static [Locale] {
        &[Locale::Ko, Locale::En]
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

// ---------------------------------------------------------------------------
// Translatable trait
// ---------------------------------------------------------------------------

/// Any domain type that can be displayed in multiple languages.
///
/// Implementors provide a dot-separated key (e.g., "stat.inner_power")
/// which is looked up in the locale files.
///
/// ```
/// use wuxia_core::shared::i18n::Translatable;
/// use wuxia_core::growth::StatType;
///
/// assert_eq!(StatType::InnerPower.translation_key(), "stat.inner_power");
/// ```
pub trait Translatable {
    /// The dot-separated translation key.
    ///
    /// Convention: `"{domain}.{variant_snake_case}"`
    ///   - "stat.inner_power"
    ///   - "stat_category.intellectual"
    ///   - "life_stage.youth"
    ///   - "season.spring"
    fn translation_key(&self) -> &'static str;
}

// ---------------------------------------------------------------------------
// Translations
// ---------------------------------------------------------------------------

/// A store of translated strings for one locale.
///
/// 포맷 중립: 이미 파싱된 `HashMap<String, String>`을 받는다.
/// TOML/JSON 파싱은 wuxia-data crate가 담당한다.
///
/// key는 dot-separated: "stat.inner_power" → "내공"
///
/// # Example
/// ```
/// use wuxia_core::shared::i18n::{Locale, Translations, Translatable};
/// use wuxia_core::growth::StatType;
/// use std::collections::HashMap;
///
/// let mut entries = HashMap::new();
/// entries.insert("stat.inner_power".to_string(), "내공".to_string());
///
/// let tr = Translations::from_entries(Locale::Ko, entries);
/// assert_eq!(tr.get(StatType::InnerPower.translation_key()), "내공");
/// ```
pub struct Translations {
    locale: Locale,
    /// Flattened map: "stat.inner_power" → "내공"
    entries: HashMap<String, String>,
}

impl Translations {
    /// 이미 파싱된 entries로 Translations를 생성한다.
    ///
    /// entries의 key는 "section.key" 형식 (예: "stat.inner_power").
    /// 파싱 자체는 wuxia-data 등 외부 crate가 담당한다.
    pub fn from_entries(locale: Locale, entries: HashMap<String, String>) -> Self {
        Self { locale, entries }
    }

    /// Look up a translation by dot-separated key.
    ///
    /// Returns the key itself as fallback if not found.
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.entries
            .get(key)
            .map(|s| s.as_str())
            .unwrap_or(key)
    }

    /// Look up a Translatable item.
    pub fn translate<T: Translatable>(&self, item: &T) -> &str {
        self.get(item.translation_key())
    }

    /// The locale this store serves.
    pub fn locale(&self) -> Locale {
        self.locale
    }

    /// Number of entries loaded.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl fmt::Debug for Translations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Translations({}, {} entries)", self.locale, self.len())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ko() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("stat.inner_power".into(), "내공".into());
        m.insert("stat.wisdom".into(), "지혜".into());
        m.insert("stat.strategy".into(), "책략".into());
        m.insert("stat_category.intellectual".into(), "지적".into());
        m.insert("stat_category.physical".into(), "육체".into());
        m.insert("stat_category.emotional".into(), "감정".into());
        m.insert("life_stage.youth".into(), "청년".into());
        m.insert("life_stage.prime".into(), "장년".into());
        m
    }

    fn sample_en() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("stat.inner_power".into(), "Inner Power".into());
        m.insert("stat.wisdom".into(), "Wisdom".into());
        m.insert("stat.strategy".into(), "Strategy".into());
        m.insert("stat_category.intellectual".into(), "Intellectual".into());
        m.insert("stat_category.physical".into(), "Physical".into());
        m.insert("stat_category.emotional".into(), "Emotional".into());
        m.insert("life_stage.youth".into(), "Youth".into());
        m.insert("life_stage.prime".into(), "Prime".into());
        m
    }

    #[test]
    fn load_ko() {
        let tr = Translations::from_entries(Locale::Ko, sample_ko());
        assert_eq!(tr.locale(), Locale::Ko);
        assert_eq!(tr.get("stat.inner_power"), "내공");
        assert_eq!(tr.get("stat.wisdom"), "지혜");
        assert_eq!(tr.get("stat_category.intellectual"), "지적");
        assert_eq!(tr.get("life_stage.youth"), "청년");
    }

    #[test]
    fn load_en() {
        let tr = Translations::from_entries(Locale::En, sample_en());
        assert_eq!(tr.locale(), Locale::En);
        assert_eq!(tr.get("stat.inner_power"), "Inner Power");
        assert_eq!(tr.get("stat_category.intellectual"), "Intellectual");
    }

    #[test]
    fn missing_key_returns_key_itself() {
        let tr = Translations::from_entries(Locale::Ko, sample_ko());
        assert_eq!(tr.get("nonexistent.key"), "nonexistent.key");
    }

    #[test]
    fn entry_count() {
        let tr = Translations::from_entries(Locale::Ko, sample_ko());
        assert_eq!(tr.len(), 8);
    }

    #[test]
    fn locale_code() {
        assert_eq!(Locale::Ko.code(), "ko");
        assert_eq!(Locale::En.code(), "en");
    }

    #[test]
    fn locale_display() {
        assert_eq!(Locale::Ko.to_string(), "ko");
        assert_eq!(Locale::En.to_string(), "en");
    }
}
