//! NPC 관계 모델 — 4축 + BondKind + BondStatus + Partnership + type.
//!
//! relationships.md v0.7. Phase 2 Stage 1 마이그레이션 산출 — 3축 (closeness/trust/power)
//! → 4축 (trust/affinity/respect/wariness) + 신규 분류/상태/형식 enum 도입. `power` 폐기 (B-D4).
//!
//! ## DDD 분류: Aggregate (Value Object → Aggregate 승격)
//!
//! Stage 1에서 *상태 변경 가능 메서드 (`apply_delta`)* 도입과 함께 Value Object 패턴은
//! Aggregate 패턴으로 승격. owner_id + target_id 조합이 identity.
//!
//! ## Phase 2 Stage 범위
//!
//! - Stage 1 (현재): 도메인 본체 + 신규 enum + 시그니처 보존
//! - Stage 2: OCC → 4축 매핑 (`base_delta` + HEXACO + `update_axes_from_emotion`)
//! - Stage 3: `RelationshipUpdatedPayload` 6→8 + `event_bridge` + frontend 4축 표시
//! - Stage 4: 시나리오 JSON 마이그레이션 도구
//! - Stage 5: Narrative 검증
//! - Stage 6: Bench + Phase 2.3 handoff

mod axis;
mod bond;
mod partnership;

pub use axis::{AxisDelta, AxisKind, AxisScore, WarinessScore};
pub use bond::{BondKind, BondStatus};
pub use partnership::Partnership;

use crate::domain::emotion::RelationshipModifiers;
use crate::domain::tuning::profile;
use serde::{Deserialize, Serialize};

/// NPC와 상대(NPC 또는 플레이어) 사이의 관계 — 4축 + bond_* + partnership + type.
///
/// `power` 폐기 (Phase 2 B-D4) — 위계 정보는 `type_text` 자유 텍스트로 흡수.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    /// 관계 소유자 ID (누구의 관계인가)
    owner_id: String,
    /// 관계 대상 ID (누구에 대한 관계인가)
    target_id: String,

    // 4축 (B-D1: 별 타입). Stage 1: serde(default)로 3축 호환 — `trust/affinity/respect/wariness`
    // 누락된 v0.6 JSON은 NEUTRAL로 fallback. Stage 4 마이그레이션 도구로 정식 변환.
    #[serde(default)]
    trust: AxisScore,
    #[serde(default)]
    affinity: AxisScore,
    #[serde(default)]
    respect: AxisScore,
    #[serde(default)]
    wariness: WarinessScore,

    // 분류 + 상태
    #[serde(default)]
    bond_kind: Option<BondKind>,
    #[serde(default)]
    bond_status: BondStatus,
    #[serde(default)]
    partnership: Option<Partnership>,

    // 자유 텍스트 (B-D4: power 흡수)
    #[serde(rename = "type", default)]
    type_text: String,
    #[serde(default)]
    type_history: Vec<TypeChange>,
}

/// `type` 변경 이력 element (relationships.md v0.7 §2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeChange {
    pub from_type: String,
    pub to_type: String,
    /// 변경 맥락 (예: "의형제 결연 사건")
    pub note: String,
}

impl Relationship {
    /// 새 관계 생성 (4축 명시).
    pub fn new(
        owner_id: impl Into<String>,
        target_id: impl Into<String>,
        trust: AxisScore,
        affinity: AxisScore,
        respect: AxisScore,
        wariness: WarinessScore,
    ) -> Self {
        Self {
            owner_id: owner_id.into(),
            target_id: target_id.into(),
            trust,
            affinity,
            respect,
            wariness,
            bond_kind: None,
            bond_status: BondStatus::Active,
            partnership: None,
            type_text: String::new(),
            type_history: Vec::new(),
        }
    }

    /// 중립 관계 — 모든 4축 0, 그 외 default.
    ///
    /// **시그니처 보존** — 62곳 호출처 자동 흡수 (Stage 1.8).
    pub fn neutral(owner_id: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self::new(
            owner_id,
            target_id,
            AxisScore::NEUTRAL,
            AxisScore::NEUTRAL,
            AxisScore::NEUTRAL,
            WarinessScore::NEUTRAL,
        )
    }

    // --- 접근자 ---

    pub fn owner_id(&self) -> &str {
        &self.owner_id
    }
    pub fn target_id(&self) -> &str {
        &self.target_id
    }
    pub fn trust(&self) -> AxisScore {
        self.trust
    }
    pub fn affinity(&self) -> AxisScore {
        self.affinity
    }
    pub fn respect(&self) -> AxisScore {
        self.respect
    }
    pub fn wariness(&self) -> WarinessScore {
        self.wariness
    }
    pub fn bond_kind(&self) -> Option<BondKind> {
        self.bond_kind
    }
    pub fn bond_status(&self) -> &BondStatus {
        &self.bond_status
    }
    pub fn partnership(&self) -> Option<Partnership> {
        self.partnership
    }
    pub fn type_text(&self) -> &str {
        &self.type_text
    }
    pub fn type_history(&self) -> &[TypeChange] {
        &self.type_history
    }

    /// 4축 일괄 변동 (Stage 2 `update_axes_from_emotion`에서 호출 예정).
    /// BondStatus 차단은 호출 측 (Stage 2)에서 처리.
    pub fn apply_delta(&mut self, delta: &AxisDelta) {
        self.trust = self.trust.add(delta.trust);
        self.affinity = self.affinity.add(delta.affinity);
        self.respect = self.respect.add(delta.respect);
        self.wariness = self.wariness.add(delta.wariness);
    }

    // --- 감정 엔진 연동 (읽기 전용) ---

    /// 감정 평가에 필요한 modifier 값을 사전 계산.
    ///
    /// **Phase 2 Stage 1 — F1 흡수 정책**: `RelationshipModifiers` 4 필드
    /// (intensity/trust/empathy/hostility)는 Phase 2.3 정밀화 대기.
    /// 본 메서드는 *closeness 입력만 affinity로 swap* — 시맨틱 보존.
    /// tuning profile `rel_closeness_*_weight` 필드 이름도 그대로 유지 (Phase 2.3 rename).
    pub fn modifiers(&self) -> RelationshipModifiers {
        let affinity_norm = self.affinity.value() / 100.0; // -1.0..1.0 정규화
        let trust_norm = self.trust.value() / 100.0;
        let p = profile();
        RelationshipModifiers {
            intensity_multiplier: (1.0 + affinity_norm * p.rel_closeness_intensity_weight)
                .max(0.0),
            trust_modifier: 1.0 + trust_norm * p.rel_trust_emotion_weight,
            empathy_modifier: (1.0 + affinity_norm * p.rel_closeness_empathy_weight).max(0.0),
            hostility_modifier: (1.0 - affinity_norm * p.rel_closeness_hostility_weight).max(0.0),
        }
    }
}

// ---------------------------------------------------------------------------
// Relationship Builder — fluent API
// ---------------------------------------------------------------------------

/// 관계를 편리하게 생성하는 빌더 (4축 + 신규 enum 옵션 setter).
#[derive(Debug, Clone)]
pub struct RelationshipBuilder {
    owner_id: String,
    target_id: String,

    trust: AxisScore,
    affinity: AxisScore,
    respect: AxisScore,
    wariness: WarinessScore,

    bond_kind: Option<BondKind>,
    bond_status: BondStatus,
    partnership: Option<Partnership>,
    type_text: String,
    type_history: Vec<TypeChange>,
}

impl RelationshipBuilder {
    /// 새 빌더. 모든 4축 NEUTRAL, bond_status default = Active.
    pub fn new(owner_id: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self {
            owner_id: owner_id.into(),
            target_id: target_id.into(),
            trust: AxisScore::NEUTRAL,
            affinity: AxisScore::NEUTRAL,
            respect: AxisScore::NEUTRAL,
            wariness: WarinessScore::NEUTRAL,
            bond_kind: None,
            bond_status: BondStatus::Active,
            partnership: None,
            type_text: String::new(),
            type_history: Vec::new(),
        }
    }

    // 4축 setter
    pub fn trust(mut self, value: AxisScore) -> Self {
        self.trust = value;
        self
    }
    pub fn affinity(mut self, value: AxisScore) -> Self {
        self.affinity = value;
        self
    }
    pub fn respect(mut self, value: AxisScore) -> Self {
        self.respect = value;
        self
    }
    pub fn wariness(mut self, value: WarinessScore) -> Self {
        self.wariness = value;
        self
    }

    // 새 필드 setter — None은 setter 미호출로 표현
    pub fn bond_kind(mut self, value: BondKind) -> Self {
        self.bond_kind = Some(value);
        self
    }
    pub fn bond_status(mut self, value: BondStatus) -> Self {
        self.bond_status = value;
        self
    }
    pub fn partnership(mut self, value: Partnership) -> Self {
        self.partnership = Some(value);
        self
    }
    pub fn type_text(mut self, value: impl Into<String>) -> Self {
        self.type_text = value.into();
        self
    }
    pub fn type_history(mut self, value: Vec<TypeChange>) -> Self {
        self.type_history = value;
        self
    }

    /// 빌드 — 같은 모듈 내 private 필드 직접 packing.
    pub fn build(self) -> Relationship {
        Relationship {
            owner_id: self.owner_id,
            target_id: self.target_id,
            trust: self.trust,
            affinity: self.affinity,
            respect: self.respect,
            wariness: self.wariness,
            bond_kind: self.bond_kind,
            bond_status: self.bond_status,
            partnership: self.partnership,
            type_text: self.type_text,
            type_history: self.type_history,
        }
    }
}

// ---------------------------------------------------------------------------
// 단위 테스트 — Stage 1.9 (Relationship + Builder + TypeChange)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_initializes_4_axes_and_defaults() {
        let r = Relationship::new(
            "a",
            "b",
            AxisScore::new(50.0),
            AxisScore::new(40.0),
            AxisScore::new(30.0),
            WarinessScore::new(20.0),
        );
        assert_eq!(r.trust().value(), 50.0);
        assert_eq!(r.affinity().value(), 40.0);
        assert_eq!(r.respect().value(), 30.0);
        assert_eq!(r.wariness().value(), 20.0);
        assert_eq!(r.bond_kind(), None);
        assert_eq!(r.bond_status(), &BondStatus::Active);
        assert_eq!(r.partnership(), None);
        assert_eq!(r.type_text(), "");
        assert!(r.type_history().is_empty());
    }

    #[test]
    fn neutral_signature_preserved_and_zero_axes() {
        let r = Relationship::neutral("a", "b");
        assert_eq!(r.trust().value(), 0.0);
        assert_eq!(r.affinity().value(), 0.0);
        assert_eq!(r.respect().value(), 0.0);
        assert_eq!(r.wariness().value(), 0.0);
        assert_eq!(r.bond_kind(), None);
        assert_eq!(r.bond_status(), &BondStatus::Active);
        assert_eq!(r.partnership(), None);
        assert!(r.type_text().is_empty());
        assert!(r.type_history().is_empty());
    }

    #[test]
    fn apply_delta_updates_all_4_axes() {
        let mut r = Relationship::neutral("a", "b");
        r.apply_delta(&AxisDelta {
            trust: 20.0,
            affinity: 10.0,
            respect: 5.0,
            wariness: 15.0,
        });
        assert_eq!(r.trust().value(), 20.0);
        assert_eq!(r.affinity().value(), 10.0);
        assert_eq!(r.respect().value(), 5.0);
        assert_eq!(r.wariness().value(), 15.0);
    }

    #[test]
    fn apply_delta_clamps_at_bounds() {
        let mut r = Relationship::new(
            "a",
            "b",
            AxisScore::new(90.0),
            AxisScore::NEUTRAL,
            AxisScore::NEUTRAL,
            WarinessScore::new(5.0),
        );
        r.apply_delta(&AxisDelta {
            trust: 30.0,
            affinity: 0.0,
            respect: 0.0,
            wariness: -20.0,
        });
        assert_eq!(r.trust().value(), 100.0); // cap
        assert_eq!(r.wariness().value(), 0.0); // floor
    }

    #[test]
    fn modifiers_uses_affinity_input_with_preserved_field_names() {
        let r = Relationship::new(
            "a",
            "b",
            AxisScore::new(50.0),
            AxisScore::new(80.0),
            AxisScore::NEUTRAL,
            WarinessScore::NEUTRAL,
        );
        let m = r.modifiers();
        let p = profile();
        let affinity_norm = 0.8_f32;
        let trust_norm = 0.5_f32;
        let expected_intensity = (1.0 + affinity_norm * p.rel_closeness_intensity_weight).max(0.0);
        let expected_trust = 1.0 + trust_norm * p.rel_trust_emotion_weight;
        let expected_empathy = (1.0 + affinity_norm * p.rel_closeness_empathy_weight).max(0.0);
        let expected_hostility =
            (1.0 - affinity_norm * p.rel_closeness_hostility_weight).max(0.0);
        assert!((m.intensity_multiplier - expected_intensity).abs() < 1e-6);
        assert!((m.trust_modifier - expected_trust).abs() < 1e-6);
        assert!((m.empathy_modifier - expected_empathy).abs() < 1e-6);
        assert!((m.hostility_modifier - expected_hostility).abs() < 1e-6);
    }

    #[test]
    fn neutral_modifiers_all_unit() {
        let r = Relationship::neutral("a", "b");
        let m = r.modifiers();
        assert!((m.intensity_multiplier - 1.0).abs() < 1e-6);
        assert!((m.trust_modifier - 1.0).abs() < 1e-6);
        assert!((m.empathy_modifier - 1.0).abs() < 1e-6);
        assert!((m.hostility_modifier - 1.0).abs() < 1e-6);
    }

    #[test]
    fn builder_chain_4_axes() {
        let r = RelationshipBuilder::new("a", "b")
            .trust(AxisScore::new(50.0))
            .affinity(AxisScore::new(40.0))
            .respect(AxisScore::new(30.0))
            .wariness(WarinessScore::new(20.0))
            .build();
        assert_eq!(r.trust().value(), 50.0);
        assert_eq!(r.affinity().value(), 40.0);
        assert_eq!(r.respect().value(), 30.0);
        assert_eq!(r.wariness().value(), 20.0);
        assert_eq!(r.bond_kind(), None);
        assert_eq!(r.bond_status(), &BondStatus::Active);
    }

    #[test]
    fn builder_partial_setters_apply_defaults() {
        let r = RelationshipBuilder::new("a", "b")
            .trust(AxisScore::new(50.0))
            .build();
        assert_eq!(r.trust().value(), 50.0);
        assert_eq!(r.affinity().value(), 0.0);
        assert_eq!(r.respect().value(), 0.0);
        assert_eq!(r.wariness().value(), 0.0);
    }

    #[test]
    fn builder_bond_kind_wraps_in_some() {
        let r = RelationshipBuilder::new("a", "b")
            .bond_kind(BondKind::SwornBrothers)
            .build();
        assert_eq!(r.bond_kind(), Some(BondKind::SwornBrothers));
    }

    #[test]
    fn builder_partnership_wraps_in_some() {
        let r = RelationshipBuilder::new("a", "b")
            .partnership(Partnership::Spouse)
            .build();
        assert_eq!(r.partnership(), Some(Partnership::Spouse));
    }

    #[test]
    fn builder_type_text_and_history() {
        let history = vec![TypeChange {
            from_type: "동료".into(),
            to_type: "원수".into(),
            note: "산신묘 사건".into(),
        }];
        let r = RelationshipBuilder::new("a", "b")
            .type_text("의형제")
            .type_history(history.clone())
            .build();
        assert_eq!(r.type_text(), "의형제");
        assert_eq!(r.type_history(), history.as_slice());
    }

    #[test]
    fn builder_full_chain_all_fields() {
        let r = RelationshipBuilder::new("a", "b")
            .trust(AxisScore::new(50.0))
            .affinity(AxisScore::new(60.0))
            .respect(AxisScore::new(40.0))
            .wariness(WarinessScore::new(10.0))
            .bond_kind(BondKind::SwornBrothers)
            .bond_status(BondStatus::Active)
            .partnership(Partnership::Lover)
            .type_text("의형제이자 연인")
            .build();
        assert_eq!(r.trust().value(), 50.0);
        assert_eq!(r.affinity().value(), 60.0);
        assert_eq!(r.respect().value(), 40.0);
        assert_eq!(r.wariness().value(), 10.0);
        assert_eq!(r.bond_kind(), Some(BondKind::SwornBrothers));
        assert_eq!(r.bond_status(), &BondStatus::Active);
        assert_eq!(r.partnership(), Some(Partnership::Lover));
        assert_eq!(r.type_text(), "의형제이자 연인");
    }

    #[test]
    fn relationship_serde_round_trip_with_type_key() {
        let r = RelationshipBuilder::new("a", "b")
            .trust(AxisScore::new(50.0))
            .affinity(AxisScore::new(60.0))
            .type_text("의형제")
            .build();
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"type\":\"의형제\""));
        let back: Relationship = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn relationship_serde_defaults_missing_fields() {
        let json = r#"{
            "owner_id": "a",
            "target_id": "b",
            "trust": 0.0,
            "affinity": 0.0,
            "respect": 0.0,
            "wariness": 0.0
        }"#;
        let r: Relationship = serde_json::from_str(json).unwrap();
        assert_eq!(r.bond_status(), &BondStatus::Active);
        assert!(r.type_history().is_empty());
        assert_eq!(r.bond_kind(), None);
        assert_eq!(r.partnership(), None);
        assert_eq!(r.type_text(), "");
    }

    #[test]
    fn type_change_serde_round_trip() {
        let tc = TypeChange {
            from_type: "조정 동료".into(),
            to_type: "처단 대상".into(),
            note: "산신묘 사건".into(),
        };
        let json = serde_json::to_string(&tc).unwrap();
        let back: TypeChange = serde_json::from_str(&json).unwrap();
        assert_eq!(back, tc);
    }
}
