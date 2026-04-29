// wuxia-core/src/relationship/effect.rs
//
// apply_conversation_effect — 대화가 관계에 미치는 영향.
//
// "대화 결과가 두 사람의 관계를 변화시키는" 도메인 규칙.
//
// 무협 비유:
//   주막에서 나눈 대화 한마디가 원수를 친구로,
//   친구를 원수로 바꿀 수 있다.
//   그 규칙은 무인이 아닌, 강호의 이치(理致)에 해당한다.
//
// 아키텍처:
//   위치: wuxia-core (Domain Service)
//   의존: Relationship (같은 Core의 Entity)
//   호출자: ChatSession (wuxia-llm, Application Service)
//
// 확장 계획:
//   MVP:     affinity만 변경
//   Sprint4: 2축 분배 (affinity + trust)
//   Sprint5: 턴 수 기반 신뢰도 보너스
//   향후:    OCC 감정 반영, 성격 변화

use crate::shared::event::DomainEvent;
use crate::shared::sentiment::DeltaSource;

use super::Relationship;

// ---------------------------------------------------------------------------
// ConversationEffect — 대화의 관계 영향
// ---------------------------------------------------------------------------

/// 대화 한 턴의 관계 영향.
///
/// LLM 판정 또는 `[affinity: N]` 태그로부터 생성된다.
///
/// # Example
/// ```
/// use wuxia_core::relationship::ConversationEffect;
///
/// // LLM 응답에서 [affinity: +3] 파싱됨
/// let effect = ConversationEffect::new(3);
/// assert_eq!(effect.affinity_delta(), 3);
/// ```
///
/// # 확장 계획
/// ```text
///   MVP (현재):
///     affinity_delta만 사용
///
///   Sprint 4 (2축 분배):
///     trust_delta: i8,      // [trust: +2] 태그 추가
///
///   Sprint 5 (턴 기반 보너스):
///     turn_count: usize,    // 5턴 이상 대화 시 trust +2 보너스
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationEffect {
    /// LLM이 판단한 호감도 변화량 (-5 ~ +5).
    affinity_delta: i8,
    /// 이 delta가 산출된 경로. [v4.2]
    source: DeltaSource,
}

impl ConversationEffect {
    /// 새 ConversationEffect를 생성한다 (LegacyTag 기본값).
    ///
    /// 기존 코드 하위호환. `[affinity: N]` 태그 기반.
    pub fn new(affinity_delta: i8) -> Self {
        Self {
            affinity_delta,
            source: DeltaSource::LegacyTag,
        }
    }

    /// delta 출처를 지정하여 ConversationEffect를 생성한다. [v4.2]
    pub fn with_source(affinity_delta: i8, source: DeltaSource) -> Self {
        Self {
            affinity_delta,
            source,
        }
    }

    /// 호감도 변화량을 반환한다.
    pub fn affinity_delta(&self) -> i8 {
        self.affinity_delta
    }

    /// delta 출처를 반환한다. [v4.2]
    pub fn source(&self) -> DeltaSource {
        self.source
    }

    /// 변화가 없는지 확인한다.
    pub fn is_empty(&self) -> bool {
        self.affinity_delta == 0
    }
}

impl Default for ConversationEffect {
    fn default() -> Self {
        Self {
            affinity_delta: 0,
            source: DeltaSource::LegacyTag,
        }
    }
}

/// 대화 결과를 관계에 반영하는 도메인 규칙.
///
/// MVP: affinity만 변경.
/// 향후: 3축 분배, 턴 수 기반 신뢰도 보너스 등 확장.
///
/// 변경이 발생하면 `Vec<DomainEvent>`를 반환한다.
/// 변화 없으면 빈 Vec.
///
/// # Arguments
/// * `relationship` — 변경할 관계 (mutable reference).
/// * `effect` — 대화에서 발생한 영향.
///
/// # Example
/// ```
/// use wuxia_core::relationship::{Relationship, ConversationEffect, apply_conversation_effect};
/// use wuxia_core::shared::id::{CharacterId, RelationshipId};
///
/// let mut rel = Relationship::new(
///     RelationshipId::new(1),
///     CharacterId::new(1),
///     CharacterId::new(2),
/// );
///
/// let effect = ConversationEffect::new(3);
/// let events = apply_conversation_effect(&mut rel, &effect);
/// assert_eq!(rel.affinity(), 3.0);
/// assert_eq!(events.len(), 1); // AffinityChanged
/// ```
pub fn apply_conversation_effect(
    relationship: &mut Relationship,
    effect: &ConversationEffect,
) -> Vec<DomainEvent> {
    if effect.is_empty() {
        return Vec::new();
    }
    relationship.update_affinity(effect.affinity_delta() as f32)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relationship::RelationshipLevel;

    fn make_rel() -> Relationship {
        crate::test_fixtures::make_relationship(1, 1, 2)
    }

    fn make_rel_with_affinity(affinity: f32) -> Relationship {
        let mut rel = make_rel();
        rel.update_affinity(affinity);
        rel
    }

    // =======================================================================
    // ConversationEffect 기본
    // =======================================================================

    #[test]
    fn effect_new_and_getters() {
        let effect = ConversationEffect::new(3);
        assert_eq!(effect.affinity_delta(), 3);
        assert!(!effect.is_empty());
    }

    #[test]
    fn effect_default_is_empty() {
        let effect = ConversationEffect::default();
        assert_eq!(effect.affinity_delta(), 0);
        assert!(effect.is_empty());
    }

    #[test]
    fn effect_negative_delta() {
        let effect = ConversationEffect::new(-5);
        assert_eq!(effect.affinity_delta(), -5);
        assert!(!effect.is_empty());
    }

    // =======================================================================
    // apply_conversation_effect — 기본 동작
    // =======================================================================

    #[test]
    fn apply_positive_delta() {
        let mut rel = make_rel();
        apply_conversation_effect(&mut rel, &ConversationEffect::new(3));
        assert_eq!(rel.affinity(), 3.0);
    }

    #[test]
    fn apply_negative_delta() {
        let mut rel = make_rel_with_affinity(50.0);
        apply_conversation_effect(&mut rel, &ConversationEffect::new(-4));
        assert_eq!(rel.affinity(), 46.0);
    }

    #[test]
    fn apply_zero_delta_no_change() {
        let mut rel = make_rel_with_affinity(30.0);
        apply_conversation_effect(&mut rel, &ConversationEffect::new(0));
        assert_eq!(rel.affinity(), 30.0);
    }

    #[test]
    fn apply_default_effect_no_change() {
        let mut rel = make_rel_with_affinity(30.0);
        apply_conversation_effect(&mut rel, &ConversationEffect::default());
        assert_eq!(rel.affinity(), 30.0);
    }

    // =======================================================================
    // apply_conversation_effect — clamp 경계
    // =======================================================================

    #[test]
    fn apply_clamps_at_max() {
        let mut rel = make_rel_with_affinity(98.0);
        apply_conversation_effect(&mut rel, &ConversationEffect::new(5));
        assert_eq!(rel.affinity(), 100.0); // 103 → 100 clamped
    }

    #[test]
    fn apply_clamps_at_min() {
        let mut rel = make_rel_with_affinity(-98.0);
        apply_conversation_effect(&mut rel, &ConversationEffect::new(-5));
        assert_eq!(rel.affinity(), -100.0); // -103 → -100 clamped
    }

    // =======================================================================
    // apply_conversation_effect — 누적 시나리오
    // =======================================================================

    #[test]
    fn cumulative_effects_over_multiple_turns() {
        // 5턴 대화: +3, +2, -1, +4, +2 = 총 +10
        let mut rel = make_rel_with_affinity(20.0);

        let deltas = [3, 2, -1, 4, 2];
        for d in deltas {
            apply_conversation_effect(&mut rel, &ConversationEffect::new(d));
        }

        assert_eq!(rel.affinity(), 30.0);
    }

    #[test]
    fn effects_can_change_relationship_level() {
        // Stranger → Acquaintance 전환
        let mut rel = make_rel_with_affinity(18.0);
        assert_eq!(rel.level(), RelationshipLevel::Stranger);

        // +3으로 호감도 21 → Acquaintance
        apply_conversation_effect(&mut rel, &ConversationEffect::new(3));
        assert_eq!(rel.affinity(), 21.0);
        assert_eq!(rel.level(), RelationshipLevel::Acquaintance);
    }

    // =======================================================================
    // 소연 시나리오
    // =======================================================================

    #[test]
    fn soyeon_warming_up_scenario() {
        // 소연과 3번 대화로 관계 진전
        let mut rel = make_rel();
        assert_eq!(rel.level(), RelationshipLevel::Stranger);

        // 1차 대화: 경계하며 정보 교환 → +5
        apply_conversation_effect(&mut rel, &ConversationEffect::new(5));
        assert_eq!(rel.affinity(), 5.0);

        // 2차 대화: 조금 마음을 열다 → +5
        apply_conversation_effect(&mut rel, &ConversationEffect::new(5));
        assert_eq!(rel.affinity(), 10.0);

        // 3차 대화: 유용한 정보 제공 → +5
        apply_conversation_effect(&mut rel, &ConversationEffect::new(5));
        assert_eq!(rel.affinity(), 15.0);
        // 아직 Stranger (20 미만)
        assert_eq!(rel.level(), RelationshipLevel::Stranger);

        // 4차 대화: 진심 어린 공감 → +5
        apply_conversation_effect(&mut rel, &ConversationEffect::new(5));
        assert_eq!(rel.affinity(), 20.0);
        assert_eq!(rel.level(), RelationshipLevel::Acquaintance);
    }

    #[test]
    fn soyeon_angered_scenario() {
        // 소연 호감 30에서 모욕적 발언 → -5
        let mut rel = make_rel_with_affinity(30.0);
        assert_eq!(rel.level(), RelationshipLevel::Acquaintance);

        apply_conversation_effect(&mut rel, &ConversationEffect::new(-5));
        assert_eq!(rel.affinity(), 25.0);
        // 아직 Acquaintance (20 이상)

        apply_conversation_effect(&mut rel, &ConversationEffect::new(-5));
        assert_eq!(rel.affinity(), 20.0);
        // 경계: 정확히 20 → 아직 Acquaintance

        apply_conversation_effect(&mut rel, &ConversationEffect::new(-1));
        assert_eq!(rel.affinity(), 19.0);
        // 19 < 20 → Stranger로 강등
        assert_eq!(rel.level(), RelationshipLevel::Stranger);
    }

    // =======================================================================
    // 이벤트 반환 검증
    // =======================================================================

    #[test]
    fn apply_returns_affinity_changed_event() {
        let mut rel = make_rel_with_affinity(10.0);
        let events = apply_conversation_effect(&mut rel, &ConversationEffect::new(3));
        assert_eq!(events.len(), 1);
        // AffinityChanged만 — 레벨은 Stranger 유지 (13 < 20)
    }

    #[test]
    fn apply_zero_returns_empty_events() {
        let mut rel = make_rel_with_affinity(30.0);
        let events = apply_conversation_effect(&mut rel, &ConversationEffect::new(0));
        assert!(events.is_empty());
    }

    #[test]
    fn apply_default_returns_empty_events() {
        let mut rel = make_rel_with_affinity(30.0);
        let events = apply_conversation_effect(&mut rel, &ConversationEffect::default());
        assert!(events.is_empty());
    }

    #[test]
    fn apply_level_transition_returns_two_events() {
        // 18 + 3 = 21 → Stranger → Acquaintance
        let mut rel = make_rel_with_affinity(18.0);
        let events = apply_conversation_effect(&mut rel, &ConversationEffect::new(3));
        assert_eq!(events.len(), 2); // AffinityChanged + LevelChanged
    }

    #[test]
    fn apply_clamp_at_max_still_emits_event() {
        // 98 + 5 = 100 (clamped) — 값이 변했으므로 이벤트 발행
        let mut rel = make_rel_with_affinity(98.0);
        let events = apply_conversation_effect(&mut rel, &ConversationEffect::new(5));
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn apply_clamp_at_max_no_op() {
        // 100 + 5 = 100 (no change) — 이벤트 없음
        let mut rel = make_rel_with_affinity(100.0);
        let events = apply_conversation_effect(&mut rel, &ConversationEffect::new(5));
        assert!(events.is_empty());
    }
}
