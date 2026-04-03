//! 빌트인 PAD 앵커 레지스트리
//!
//! `include_str!`로 컴파일 타임에 TOML 앵커 파일을 내장한다.
//! 로케일 레지스트리(`presentation/mod.rs`)와 동일한 패턴.

const BUILTIN_KO: &str = include_str!("../../locales/anchors/ko.toml");

/// 언어 코드로 빌트인 앵커 TOML을 반환
///
/// 지원하지 않는 언어이면 `None`.
pub fn builtin_anchor_toml(lang: &str) -> Option<&'static str> {
    match lang {
        "ko" => Some(BUILTIN_KO),
        _ => None,
    }
}
