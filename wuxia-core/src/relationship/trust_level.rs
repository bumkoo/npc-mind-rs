// wuxia-core/src/relationship/trust_level.rs
//
// TrustLevel — 신뢰도 수치를 5구간으로 분류.

use serde::{Deserialize, Serialize};

/// 신뢰도 수치를 5구간으로 분류한다.
///
/// LLM 프롬프트에 숫자 대신 자연어 설명을 삽입하기 위한 중간 계층.
/// 숫자 → enum → 설정 파일(toml) → 자연어 순으로 변환된다.
///
/// ```text
///   0~9    None         전혀 신뢰하지 않음
///   10~29  Wary         경계
///   30~49  Cautious     조심스러운 신뢰
///   50~69  Considerable 상당한 신뢰
///   70~100 Deep         깊은 신뢰
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrustLevel {
    /// 0~9: 전혀 신뢰하지 않음.
    None,
    /// 10~29: 경계. 쉽게 믿지 않는다.
    Wary,
    /// 30~49: 조심스러운 신뢰. 비밀은 아직.
    Cautious,
    /// 50~69: 상당한 신뢰. 중요한 이야기를 꺼낼 수 있다.
    Considerable,
    /// 70~100: 깊은 신뢰. 등을 맡길 수 있다.
    Deep,
}

impl TrustLevel {
    /// 신뢰도 수치(0~100)를 구간으로 변환한다.
    pub fn from_value(trust: f32) -> Self {
        match trust as u32 {
            0..10 => Self::None,
            10..30 => Self::Wary,
            30..50 => Self::Cautious,
            50..70 => Self::Considerable,
            _ => Self::Deep, // 70~100
        }
    }

    /// 설정 파일 조회용 키. descriptions.toml의 [trust_level.X]와 매칭.
    pub fn key(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Wary => "Wary",
            Self::Cautious => "Cautious",
            Self::Considerable => "Considerable",
            Self::Deep => "Deep",
        }
    }
}
