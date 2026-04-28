//! NPC 관계 모델
//!
//! NPC와 NPC, NPC와 플레이어 사이의 관계를 3축으로 모델링한다.
//! 각 축은 -1.0 ~ 1.0 범위의 Score를 사용한다.
//!
//! 3축:
//! - closeness (친밀도): 감정 반응의 전반적 배율 + Fortune-of-others 방향
//! - trust (신뢰도): 신뢰 방향에 따른 감정 증폭/약화
//! - power (상하 관계): 대사 톤 결정 (감정 엔진 영향 최소)
//!
//! ## DDD 분류: Value Object
//!
//! Relationship은 불변 Value Object다.
//! 상태를 변경하는 메서드는 새 인스턴스를 반환한다.
//! 소유자(owner_id)와 대상(target_id)의 조합이 동일성을 결정한다.
//!
//! 대화 중에는 고정이며, 대화 종료 후 새 인스턴스로 교체된다.

use serde::{Deserialize, Serialize};

use super::emotion::{EmotionState, RelationshipModifiers};
use super::personality::Score;

// ---------------------------------------------------------------------------
// 갱신 속도 — TuningProfile에서 조회
// ---------------------------------------------------------------------------

use crate::domain::tuning::profile;

// ---------------------------------------------------------------------------
// Relationship (Value Object)
// ---------------------------------------------------------------------------

/// NPC와 상대(NPC 또는 플레이어) 사이의 관계
///
/// 불변 Value Object — 상태 변경 시 새 인스턴스를 반환한다.
/// 3축 모두 Score(-1.0 ~ 1.0) 사용.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// 관계 소유자 ID (누구의 관계인가)
    owner_id: String,
    /// 관계 대상 ID (누구에 대한 관계인가)
    target_id: String,
    /// 친밀도 (-1.0=적대, 0.0=무관, 1.0=절친)
    /// 감정 반응의 전반적 배율 + Fortune-of-others 분기 방향
    closeness: Score,
    /// 신뢰도 (-1.0=불신, 0.0=중립, 1.0=전적 신뢰)
    /// 신뢰할수록 감정이 강해짐 (Action 브랜치 4개 감정에 적용)
    trust: Score,
    /// 상하 관계 (-1.0=하위, 0.0=대등, 1.0=상위)
    /// 대사 톤 결정 (감정 엔진 영향 최소)
    power: Score,
}

impl Relationship {
    /// 새 관계 생성
    pub fn new(
        owner_id: impl Into<String>,
        target_id: impl Into<String>,
        closeness: Score,
        trust: Score,
        power: Score,
    ) -> Self {
        Self {
            owner_id: owner_id.into(),
            target_id: target_id.into(),
            closeness,
            trust,
            power,
        }
    }

    /// 중립 관계 (모든 축 0.0)
    pub fn neutral(owner_id: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self {
            owner_id: owner_id.into(),
            target_id: target_id.into(),
            closeness: Score::neutral(),
            trust: Score::neutral(),
            power: Score::neutral(),
        }
    }

    // --- 접근자 ---

    pub fn owner_id(&self) -> &str {
        &self.owner_id
    }
    pub fn target_id(&self) -> &str {
        &self.target_id
    }
    pub fn closeness(&self) -> Score {
        self.closeness
    }
    pub fn trust(&self) -> Score {
        self.trust
    }
    pub fn power(&self) -> Score {
        self.power
    }

    // --- 감정 엔진 연동 (읽기 전용) ---

    /// 감정 평가에 필요한 modifier 값을 사전 계산하여 반환
    ///
    /// 감정 도메인이 Relationship의 내부 구조를 알 필요 없이
    /// 이 메서드로 추출한 RelationshipModifiers만 사용한다.
    pub fn modifiers(&self) -> RelationshipModifiers {
        RelationshipModifiers {
            intensity_multiplier: self.emotion_intensity_multiplier(),
            trust_modifier: self.trust_emotion_modifier(),
            empathy_modifier: self.empathy_rel_modifier(),
            hostility_modifier: self.hostility_rel_modifier(),
        }
    }

    /// 감정 반응 배율: closeness 방향에 따라 강화/약화
    /// 가까운 사이(+)면 감정 반응 강화, 적대적(-)이면 감정 절제/경계
    pub fn emotion_intensity_multiplier(&self) -> f32 {
        (1.0 + self.closeness.value() * profile().rel_closeness_intensity_weight).max(0.0)
    }

    /// 신뢰도 감정 배율: trust 방향에 따라 감정 증폭/약화
    ///
    /// 신뢰하는 사람의 행동에는 더 강하게 반응하고,
    /// 불신하는 사람의 행동에는 덜 반응한다.
    ///
    /// - 신뢰(0.8) → 1.24: "믿었는데!" / "역시 형이야"
    /// - 중립(0.0) → 1.0
    /// - 불신(-0.5) → 0.85: "역시나" / "뭔 꿍꿍이지"
    ///
    /// 1.0 ± trust × 0.3 패턴 (engine.rs의 W와 동일)
    pub fn trust_emotion_modifier(&self) -> f32 {
        1.0 + self.trust.value() * profile().rel_trust_emotion_weight
    }

    /// 공감 관계 배율: 가까울수록 공감(HappyFor/Pity) 증폭
    ///
    /// - 의형제(0.9) → 1.27: 가까운 사이라 더 공감
    /// - 중립(0.0) → 1.0
    /// - 원수(-0.8) → 0.76: 멀어서 공감 약함
    pub fn empathy_rel_modifier(&self) -> f32 {
        (1.0 + self.closeness.value() * profile().rel_closeness_empathy_weight).max(0.0)
    }

    /// 적대 관계 배율: 적대적일수록 적대감(Resentment/Gloating) 증폭
    ///
    /// - 의형제(0.9) → 0.73: 가까운 사이라 적대 억제
    /// - 중립(0.0) → 1.0
    /// - 원수(-0.8) → 1.24: 멀어서 적대 증폭
    pub fn hostility_rel_modifier(&self) -> f32 {
        (1.0 - self.closeness.value() * profile().rel_closeness_hostility_weight).max(0.0)
    }

    // --- 새 인스턴스 반환 (Value Object 패턴) ---

    /// closeness를 갱신한 새 Relationship 반환
    /// 대화의 전체 감정 결과(overall_valence) 기반. 매우 점진적.
    pub fn with_updated_closeness(&self, overall_valence: f32, significance: f32) -> Self {
        let p = profile();
        let multiplier = 1.0 + significance * p.significance_scale;
        Self {
            closeness: updated_score(
                self.closeness,
                overall_valence * p.closeness_update_rate * multiplier,
            ),
            ..self.clone()
        }
    }

    /// power를 변경한 새 Relationship 반환
    /// 게임 이벤트(승급, 내공 상실 등)에 의해 직접 설정.
    pub fn with_power(&self, power: Score) -> Self {
        Self {
            power,
            ..self.clone()
        }
    }

    /// 대화 종료 후 갱신된 새 Relationship 반환
    ///
    /// - closeness: 대화 최종 감정 결과 기반 (매우 점진적)
    /// - trust: 변경 없음 (향후 LLM 평가로 갱신 예정)
    /// - power: 변경 없음 (서사 이벤트에서만)
    /// - significance: 상황 중요도 (0.0~1.0). 클수록 변동 폭 증가.
    pub fn after_dialogue(
        &self,
        final_state: &EmotionState,
        significance: f32,
    ) -> Self {
        self.with_updated_closeness(final_state.overall_valence(), significance)
    }
}

// ---------------------------------------------------------------------------
// 헬퍼 함수
// ---------------------------------------------------------------------------

/// Score를 delta만큼 갱신한 새 Score 반환 (클램핑으로 항상 유효)
fn updated_score(current: Score, delta: f32) -> Score {
    Score::clamped(current.value() + delta)
}

// ---------------------------------------------------------------------------
// Relationship 빌더
// ---------------------------------------------------------------------------

/// 관계를 편리하게 생성하는 빌더
pub struct RelationshipBuilder {
    owner_id: String,
    target_id: String,
    closeness: Score,
    trust: Score,
    power: Score,
}

impl RelationshipBuilder {
    pub fn new(owner_id: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self {
            owner_id: owner_id.into(),
            target_id: target_id.into(),
            closeness: Score::neutral(),
            trust: Score::neutral(),
            power: Score::neutral(),
        }
    }

    pub fn closeness(mut self, value: Score) -> Self {
        self.closeness = value;
        self
    }

    pub fn trust(mut self, value: Score) -> Self {
        self.trust = value;
        self
    }

    pub fn power(mut self, value: Score) -> Self {
        self.power = value;
        self
    }

    pub fn build(self) -> Relationship {
        Relationship::new(
            self.owner_id,
            self.target_id,
            self.closeness,
            self.trust,
            self.power,
        )
    }
}

// ---------------------------------------------------------------------------
// 단위 테스트 — 내부 헬퍼 / 경계값 / Modifier 일관성
// 행위 시나리오는 tests/relationship_test.rs 참조.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::personality::SCORE_MAX;

    fn s(v: f32) -> Score {
        Score::new(v, "test").expect("범위 내 값")
    }

    #[test]
    fn updated_score_clamps_at_upper_boundary() {
        // 0.95 + 0.5 = 1.45 → clamp → 1.0
        let result = updated_score(s(0.95), 0.5);
        assert_eq!(result.value(), SCORE_MAX);
    }

    #[test]
    fn updated_score_clamps_at_lower_boundary() {
        let result = updated_score(s(-0.95), -0.5);
        assert_eq!(result.value(), -SCORE_MAX);
    }

    #[test]
    fn updated_score_normal_addition_within_range() {
        let result = updated_score(s(0.2), 0.3);
        assert!((result.value() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn emotion_intensity_multiplier_floored_at_zero() {
        // closeness=-1, weight=0.5 → 1 + (-1)*0.5 = 0.5
        // 음수 방지 max(0.0)는 weight가 1.0 이상일 때만 트리거되지만 구현 일관성 검증
        let r = Relationship::new("a", "b", s(-1.0), Score::neutral(), Score::neutral());
        let m = r.emotion_intensity_multiplier();
        assert!(m >= 0.0);
        assert!((m - 0.5).abs() < 1e-6);
    }

    #[test]
    fn empathy_rel_modifier_never_negative_at_extreme_enmity() {
        let r = Relationship::new("a", "b", s(-1.0), Score::neutral(), Score::neutral());
        let m = r.empathy_rel_modifier();
        assert!(m >= 0.0);
    }

    #[test]
    fn hostility_rel_modifier_never_negative_at_extreme_intimacy() {
        let r = Relationship::new("a", "b", s(1.0), Score::neutral(), Score::neutral());
        let m = r.hostility_rel_modifier();
        assert!(m >= 0.0);
    }

    #[test]
    fn modifiers_struct_matches_individual_methods() {
        let r = Relationship::new("a", "b", s(0.4), s(-0.3), s(0.5));
        let m = r.modifiers();
        assert_eq!(m.intensity_multiplier, r.emotion_intensity_multiplier());
        assert_eq!(m.trust_modifier, r.trust_emotion_modifier());
        assert_eq!(m.empathy_modifier, r.empathy_rel_modifier());
        assert_eq!(m.hostility_modifier, r.hostility_rel_modifier());
    }

    #[test]
    fn with_updated_closeness_significance_zero_baseline_delta() {
        // sig=0 → multiplier = 1.0; valence=+1.0 → delta = 1.0 * 0.05 * 1 = 0.05
        let r = Relationship::neutral("a", "b");
        let updated = r.with_updated_closeness(1.0, 0.0);
        assert!((updated.closeness().value() - 0.05).abs() < 1e-6);
    }

    #[test]
    fn with_updated_closeness_significance_one_quadruples_delta() {
        // sig=1 → multiplier = 1 + 1*3 = 4; valence=+1.0 → delta = 1.0 * 0.05 * 4 = 0.20
        let r = Relationship::neutral("a", "b");
        let updated = r.with_updated_closeness(1.0, 1.0);
        assert!((updated.closeness().value() - 0.20).abs() < 1e-6);
    }

    #[test]
    fn with_updated_closeness_does_not_mutate_trust_or_power() {
        let original = Relationship::new("a", "b", s(0.0), s(0.5), s(-0.3));
        let updated = original.with_updated_closeness(0.5, 0.5);
        assert_eq!(updated.trust().value(), 0.5);
        assert_eq!(updated.power().value(), -0.3);
        // 원본 불변
        assert_eq!(original.closeness().value(), 0.0);
    }

    #[test]
    fn with_power_preserves_closeness_and_trust() {
        let original = Relationship::new("a", "b", s(0.4), s(-0.2), s(0.0));
        let updated = original.with_power(s(0.9));
        assert_eq!(updated.closeness().value(), 0.4);
        assert_eq!(updated.trust().value(), -0.2);
        assert_eq!(updated.power().value(), 0.9);
    }

    #[test]
    fn after_dialogue_only_modifies_closeness() {
        use crate::domain::emotion::{Emotion, EmotionState, EmotionType};

        let mut state = EmotionState::new();
        state.add(Emotion::new(EmotionType::Joy, 0.6));
        let valence_sign = state.overall_valence().signum();

        let original = Relationship::new("a", "b", s(0.0), s(0.4), s(0.2));
        let updated = original.after_dialogue(&state, 0.5);

        assert_eq!(updated.trust().value(), 0.4);
        assert_eq!(updated.power().value(), 0.2);
        // closeness는 valence 부호 방향으로 이동
        assert!(updated.closeness().value() * valence_sign > 0.0);
    }

    #[test]
    fn neutral_empathy_and_hostility_modifiers_are_unit() {
        // intensity/trust modifier는 tests/relationship_test.rs에서 커버됨;
        // empathy/hostility는 신규 커버리지.
        let r = Relationship::neutral("a", "b");
        assert_eq!(r.empathy_rel_modifier(), 1.0);
        assert_eq!(r.hostility_rel_modifier(), 1.0);
    }
}
