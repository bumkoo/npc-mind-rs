//! Listener-perspective 변환에 쓰이는 도메인 타입들
//!
//! Sign, Magnitude, PrefilterHit — Prefilter 와 분류기 양쪽에서 공유되는 값 객체.
//!
//! 설계: docs/emotion/sign-classifier-design.md §2.1, §3.1
//!       docs/emotion/phase7-converter-integration.md §3

use thiserror::Error;

/// 청자 P 부호가 화자와 같은가 반대인가 (§2.1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sign {
    Keep,
    Invert,
}

impl Sign {
    /// +1 (Keep) / -1 (Invert) 값 변환
    pub fn as_f32(&self) -> f32 {
        match self {
            Sign::Keep => 1.0,
            Sign::Invert => -1.0,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Sign::Keep => "keep",
            Sign::Invert => "invert",
        }
    }
}

impl std::str::FromStr for Sign {
    type Err = ListenerPerspectiveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "keep" => Ok(Sign::Keep),
            "invert" => Ok(Sign::Invert),
            other => Err(ListenerPerspectiveError::InvalidSign(other.to_string())),
        }
    }
}

/// 청자가 체감하는 강도 구간 (§3.1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Magnitude {
    /// |P_L| < 0.15 — 배경 노이즈, 사과·간청·중립 정보
    Weak,
    /// 0.15 ≤ |P_L| < 0.4 — 인지 가능, 표준 감사·칭찬
    Normal,
    /// |P_L| ≥ 0.4 — 즉각 반응, 극찬·위협·빈정·강한 비난
    Strong,
}

impl Magnitude {
    pub fn as_str(&self) -> &'static str {
        match self {
            Magnitude::Weak => "weak",
            Magnitude::Normal => "normal",
            Magnitude::Strong => "strong",
        }
    }
}

impl std::str::FromStr for Magnitude {
    type Err = ListenerPerspectiveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "weak" => Ok(Magnitude::Weak),
            "normal" => Ok(Magnitude::Normal),
            "strong" => Ok(Magnitude::Strong),
            other => Err(ListenerPerspectiveError::InvalidMagnitude(other.to_string())),
        }
    }
}

/// Prefilter 매칭 결과 — hit 시 변환식에 주입되는 값들
#[derive(Debug, Clone)]
pub struct PrefilterHit {
    pub sign: Sign,
    pub magnitude: Magnitude,
    /// 변환식 P_L = sign × coef[magnitude] × p_s_default 의 P_S 기본값
    pub p_s_default: f32,
    pub matched_category: String,
    pub matched_pattern: String,
}

/// Listener-perspective 도메인의 모든 에러
#[derive(Debug, Error)]
pub enum ListenerPerspectiveError {
    #[error("알 수 없는 sign: '{0}' (허용: keep|invert)")]
    InvalidSign(String),

    #[error("알 수 없는 magnitude: '{0}' (허용: weak|normal|strong)")]
    InvalidMagnitude(String),

    #[error("프리필터 패턴 파일 로드 실패: {0}")]
    PatternIo(String),

    #[error("프리필터 패턴 파싱 실패: {0}")]
    PatternParse(String),

    #[error("프리필터 정규식 컴파일 실패 (카테고리 '{category}', 패턴 '{pattern}'): {reason}")]
    PatternCompile {
        category: String,
        pattern: String,
        reason: String,
    },

    #[error("프로토타입 파일 로드 실패: {0}")]
    PrototypeIo(String),

    #[error("프로토타입 파싱 실패: {0}")]
    PrototypeParse(String),

    #[error("프로토타입 메타 불일치 (기대 group='{expected}', 실제='{actual}')")]
    PrototypeGroupMismatch { expected: String, actual: String },

    #[error("임베딩 실패: {0}")]
    Embed(String),

    #[error("프로토타입 세트가 비어있음 (group={0})")]
    EmptyPrototypes(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn sign_from_str_valid() {
        assert_eq!(Sign::from_str("keep").unwrap(), Sign::Keep);
        assert_eq!(Sign::from_str("invert").unwrap(), Sign::Invert);
    }

    #[test]
    fn sign_from_str_invalid() {
        assert!(matches!(
            Sign::from_str("flip").unwrap_err(),
            ListenerPerspectiveError::InvalidSign(_)
        ));
    }

    #[test]
    fn sign_as_f32() {
        assert_eq!(Sign::Keep.as_f32(), 1.0);
        assert_eq!(Sign::Invert.as_f32(), -1.0);
    }

    #[test]
    fn magnitude_from_str_valid() {
        assert_eq!(Magnitude::from_str("weak").unwrap(), Magnitude::Weak);
        assert_eq!(Magnitude::from_str("normal").unwrap(), Magnitude::Normal);
        assert_eq!(Magnitude::from_str("strong").unwrap(), Magnitude::Strong);
    }

    #[test]
    fn magnitude_from_str_invalid() {
        assert!(matches!(
            Magnitude::from_str("medium").unwrap_err(),
            ListenerPerspectiveError::InvalidMagnitude(_)
        ));
    }
}
