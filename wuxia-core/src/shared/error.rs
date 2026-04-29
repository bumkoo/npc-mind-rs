// wuxia-core/src/shared/error.rs
//
// Domain Errors — what can go wrong in the wuxia world's logic.
//
// These errors represent BUSINESS LOGIC failures, not infrastructure failures.
// Infrastructure errors (DB down, LLM timeout) are handled in the adapter layer.
//
// Examples:
//   - "Cannot join a sect that is already full"      → domain error
//   - "Database connection refused"                   → adapter error (not here)
//   - "Invalid month number 13"                       → validation error
//   - "Character not found"                           → not found error

use std::fmt;



/// Errors that can occur in domain logic.
///
/// Each variant describes a specific business rule violation.
/// More variants will be added as domains are implemented.
#[derive(Debug, Clone, PartialEq)]
pub enum DomainError {
    /// A required entity was not found.
    /// Example: "Looking for CharacterId(99) but it doesn't exist."
    NotFound {
        entity_type: &'static str,
        id: String,
    },

    /// Input validation failed.
    /// Example: "Month must be 1-12, got 13."
    ValidationError {
        field: String,
        message: String,
    },

    /// A business rule was violated.
    /// Example: "Cannot have more than 7 nations."
    BusinessRuleViolation {
        rule: String,
        detail: String,
    },

    /// A duplicate entity was detected.
    /// Example: "A character with this ID already exists."
    DuplicateEntity {
        entity_type: &'static str,
        id: String,
    },
}

impl DomainError {
    // --- Convenience constructors ---

    pub fn not_found(entity_type: &'static str, id: impl fmt::Display) -> Self {
        Self::NotFound {
            entity_type,
            id: id.to_string(),
        }
    }

    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ValidationError {
            field: field.into(),
            message: message.into(),
        }
    }

    pub fn business_rule(rule: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::BusinessRuleViolation {
            rule: rule.into(),
            detail: detail.into(),
        }
    }

    pub fn duplicate(entity_type: &'static str, id: impl fmt::Display) -> Self {
        Self::DuplicateEntity {
            entity_type,
            id: id.to_string(),
        }
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::NotFound { entity_type, id } => {
                write!(f, "{} not found: {}", entity_type, id)
            }
            DomainError::ValidationError { field, message } => {
                write!(f, "Validation error on '{}': {}", field, message)
            }
            DomainError::BusinessRuleViolation { rule, detail } => {
                write!(f, "Business rule '{}' violated: {}", rule, detail)
            }
            DomainError::DuplicateEntity { entity_type, id } => {
                write!(f, "Duplicate {}: {}", entity_type, id)
            }
        }
    }
}

impl std::error::Error for DomainError {}

/// Convenience type alias for domain operations.
pub type DomainResult<T> = Result<T, DomainError>;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::id::{CharacterId, ItemId, SectId};

    #[test]
    fn not_found_error() {
        let err = DomainError::not_found("Character", CharacterId::new(99));
        assert_eq!(err.to_string(), "Character not found: Char-99");
    }

    #[test]
    fn validation_error() {
        let err = DomainError::validation("month", "must be 1-12, got 13");
        assert_eq!(
            err.to_string(),
            "Validation error on 'month': must be 1-12, got 13"
        );
    }

    #[test]
    fn business_rule_error() {
        let err = DomainError::business_rule(
            "max_nations",
            "Cannot have more than 7 nations",
        );
        assert_eq!(
            err.to_string(),
            "Business rule 'max_nations' violated: Cannot have more than 7 nations"
        );
    }

    #[test]
    fn duplicate_error() {
        let err = DomainError::duplicate("Character", CharacterId::new(1));
        assert_eq!(err.to_string(), "Duplicate Character: Char-1");
    }

    #[test]
    fn domain_result_ok() {
        let result: DomainResult<u32> = Ok(42);
        assert!(result.is_ok());
    }

    #[test]
    fn domain_result_err() {
        let result: DomainResult<u32> =
            Err(DomainError::not_found("Item", ItemId::new(7)));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Item not found: Item-7");
    }

    #[test]
    fn error_is_clone_and_eq() {
        let a = DomainError::not_found("Sect", SectId::new(3));
        let b = a.clone();
        assert_eq!(a, b);
    }
}
