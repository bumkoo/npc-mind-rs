// wuxia-core/src/shared/port_error.rs
//
// Port Error — 인프라 포트 연산 실패 에러.
//
// DomainError(비즈니스 로직 실패)와 분리된 인프라 수준 에러.
// 포트 트레이트(MemoryRepository, EmbeddingPort, RelationshipRepository,
// ChronicleRepository)의 반환 타입에 사용한다.
//
// LlmPort는 자체 LlmError를 사용하므로 PortError를 쓰지 않는다.
// LLM 실패는 의미론이 다르기 때문이다 (timeout, model not loaded 등).
//
// 설계 원칙:
//   - wuxia-core는 serde만 의존 → thiserror 미사용
//   - String보다 구조화된 에러 → kind로 패턴 매칭 가능
//   - From<String> 제공 → 기존 코드 점진적 마이그레이션

use serde::{Deserialize, Serialize};
use std::fmt;

/// 인프라 포트 에러 종류.
///
/// ```
/// use wuxia_core::shared::PortErrorKind;
///
/// let kind = PortErrorKind::NotFound;
/// assert_eq!(format!("{kind:?}"), "NotFound");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortErrorKind {
    /// 요청한 엔티티를 찾지 못함.
    NotFound,
    /// 중복 또는 충돌 (같은 ID로 재저장 시도 등).
    Conflict,
    /// 저장소 I/O 실패 (DB 다운, 파일 쓰기 실패 등).
    Storage,
    /// 데이터 변환/직렬화 실패.
    Conversion,
    /// 분류 불가 에러.
    Other,
}

/// 인프라 포트 연산 실패 에러.
///
/// `Result<T, String>` 대신 포트 트레이트에서 사용한다.
/// kind로 에러 유형을 구분하고, message로 상세 원인을 전달한다.
///
/// # Example
/// ```
/// use wuxia_core::shared::{PortError, PortErrorKind};
///
/// let err = PortError::not_found("Memory 42 not found");
/// assert_eq!(err.kind(), &PortErrorKind::NotFound);
/// assert_eq!(err.message(), "Memory 42 not found");
/// assert_eq!(err.to_string(), "NotFound: Memory 42 not found");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortError {
    kind: PortErrorKind,
    message: String,
}

impl PortError {
    /// 엔티티 미발견 에러.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self { kind: PortErrorKind::NotFound, message: message.into() }
    }

    /// 중복/충돌 에러.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self { kind: PortErrorKind::Conflict, message: message.into() }
    }

    /// 저장소 I/O 실패 에러.
    pub fn storage(message: impl Into<String>) -> Self {
        Self { kind: PortErrorKind::Storage, message: message.into() }
    }

    /// 데이터 변환/직렬화 실패 에러.
    pub fn conversion(message: impl Into<String>) -> Self {
        Self { kind: PortErrorKind::Conversion, message: message.into() }
    }

    /// 분류 불가 에러.
    pub fn other(message: impl Into<String>) -> Self {
        Self { kind: PortErrorKind::Other, message: message.into() }
    }

    /// 에러 종류를 반환한다.
    pub fn kind(&self) -> &PortErrorKind {
        &self.kind
    }

    /// 에러 메시지를 반환한다.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for PortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for PortError {}

/// 기존 String 에러로부터의 변환 (하위 호환).
impl From<String> for PortError {
    fn from(s: String) -> Self {
        Self { kind: PortErrorKind::Other, message: s }
    }
}

/// 기존 &str 에러로부터의 변환.
impl From<&str> for PortError {
    fn from(s: &str) -> Self {
        Self { kind: PortErrorKind::Other, message: s.to_string() }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_error() {
        let err = PortError::not_found("Memory 42 not found");
        assert_eq!(err.kind(), &PortErrorKind::NotFound);
        assert_eq!(err.message(), "Memory 42 not found");
        assert_eq!(err.to_string(), "NotFound: Memory 42 not found");
    }

    #[test]
    fn storage_error() {
        let err = PortError::storage("DB connection failed");
        assert_eq!(err.kind(), &PortErrorKind::Storage);
    }

    #[test]
    fn conflict_error() {
        let err = PortError::conflict("Duplicate ID");
        assert_eq!(err.kind(), &PortErrorKind::Conflict);
    }

    #[test]
    fn conversion_error() {
        let err = PortError::conversion("Arrow schema mismatch");
        assert_eq!(err.kind(), &PortErrorKind::Conversion);
    }

    #[test]
    fn other_error() {
        let err = PortError::other("Unknown issue");
        assert_eq!(err.kind(), &PortErrorKind::Other);
    }

    #[test]
    fn from_string() {
        let err: PortError = "legacy error".to_string().into();
        assert_eq!(err.kind(), &PortErrorKind::Other);
        assert_eq!(err.message(), "legacy error");
    }

    #[test]
    fn from_str() {
        let err: PortError = "str error".into();
        assert_eq!(err.kind(), &PortErrorKind::Other);
    }

    #[test]
    fn implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(PortError::storage("test"));
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn clone_and_eq() {
        let a = PortError::not_found("x");
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn serde_roundtrip() {
        let err = PortError::storage("DB down");
        let json = serde_json::to_string(&err).unwrap();
        let parsed: PortError = serde_json::from_str(&json).unwrap();
        assert_eq!(err, parsed);
    }

    #[test]
    fn different_kinds_not_equal() {
        let a = PortError::not_found("x");
        let b = PortError::storage("x");
        assert_ne!(a, b);
    }
}
