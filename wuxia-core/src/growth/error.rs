// wuxia-core/src/growth/error.rs
//
// GrowthError — 성장 도메인 에러.
//
// 다른 도메인의 에러 패턴과 일관성 유지:
//   shared/error.rs      → DomainError
//   shared/port_error.rs → PortError
//   growth/error.rs      → GrowthError (이 파일)

use crate::shared::id::MartialArtId;
use std::fmt;

/// 성장 도메인에서 발생할 수 있는 에러.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrowthError {
    /// 이미 습득한 무공을 다시 습득하려 함.
    ArtAlreadyLearned(MartialArtId),
    /// 습득하지 않은 무공을 연마하려 함.
    ArtNotLearned(MartialArtId),
}

impl fmt::Display for GrowthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GrowthError::ArtAlreadyLearned(id) => {
                write!(f, "이미 습득한 무공: {}", id)
            }
            GrowthError::ArtNotLearned(id) => {
                write!(f, "습득하지 않은 무공: {}", id)
            }
        }
    }
}

impl std::error::Error for GrowthError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn art_already_learned_display() {
        let err = GrowthError::ArtAlreadyLearned(MartialArtId::new(5));
        assert!(err.to_string().contains("5"));
    }

    #[test]
    fn art_not_learned_display() {
        let err = GrowthError::ArtNotLearned(MartialArtId::new(3));
        assert!(err.to_string().contains("3"));
    }
}
