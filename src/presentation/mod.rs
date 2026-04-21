pub mod formatter;
pub mod korean;
pub mod locale;
pub mod memory_formatter;

// ---------------------------------------------------------------------------
// 빌트인 로케일 레지스트리
// ---------------------------------------------------------------------------

/// 내장 한국어 로케일 TOML
const BUILTIN_KO: &str = include_str!("../../locales/ko.toml");
/// 내장 영어 로케일 TOML
const BUILTIN_EN: &str = include_str!("../../locales/en.toml");

/// 빌트인 로케일 TOML을 언어 코드로 조회합니다.
///
/// 지원 언어: `"ko"` (한국어), `"en"` (영어)
pub fn builtin_toml(lang: &str) -> Option<&'static str> {
    match lang {
        "ko" => Some(BUILTIN_KO),
        "en" => Some(BUILTIN_EN),
        _ => None,
    }
}
