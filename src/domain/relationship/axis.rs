//! 관계 4축 점수 타입과 산술 연산.
//!
//! - `AxisScore`: trust / affinity / respect ±100 (음양 가능 축)
//! - `WarinessScore`: wariness 0..=100 (별 타입으로 음수 컴파일 시점 차단)
//! - `AxisDelta`: 4축이 한꺼번에 받는 변동 (base_delta 표 + intensity × HEXACO)
//! - `AxisModifier`: 4축에 곱해지는 배수 (HEXACO 보정 결과)
//! - `AxisKind`: 축 식별자 (base_delta 표 lookup용)
//!
//! relationships.md v0.7 §4. 본 모듈은 *불변식 강제 + 산술 인프라*만 박는다.
//! `base_delta` / `hexaco_modifier` / `update_axes_from_emotion`은 `mapping.rs`.

use serde::{Deserialize, Serialize};

/// 음양 가능 축의 점수 (trust / affinity / respect).
///
/// 범위: -100.0 ~ +100.0
/// 내부: f32 (base_delta × intensity × HEXACO 곱셈 정밀도 유지)
/// JSON: 정수 round 출력은 *Stage 3 payload schema*에서 결정 (Stage 1은 f32 그대로)
///
/// `#[serde(try_from = "f32")]`로 deserialize 경로도 `::new()` clamp를 거치게 한다.
/// 기본 derive는 newtype-transparent라 raw f32가 그대로 박혀 invariant가 깨진다.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f32")]
pub struct AxisScore(f32);

impl TryFrom<f32> for AxisScore {
    type Error = std::convert::Infallible;
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Ok(Self::new(value))
    }
}

impl AxisScore {
    pub const MIN: f32 = -100.0;
    pub const MAX: f32 = 100.0;
    pub const NEUTRAL: AxisScore = AxisScore(0.0);

    /// 입력을 ±100으로 clamp.
    pub fn new(value: f32) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }

    pub fn value(&self) -> f32 {
        self.0
    }

    /// delta를 더하고 clamp한 새 값.
    pub fn add(self, delta: f32) -> Self {
        Self::new(self.0 + delta)
    }
}

impl Default for AxisScore {
    fn default() -> Self {
        Self::NEUTRAL
    }
}

/// 경계심 축 점수 (wariness 전용).
///
/// 범위: 0.0 ~ +100.0
/// 별 타입이므로 *컴파일 시점*에 `AxisScore`와 혼동 차단.
/// `WarinessScore::new(-50.0)`은 runtime에 0.0으로 floor.
///
/// `#[serde(try_from = "f32")]`로 deserialize도 0.0 floor 적용 (음수 invariant 유지).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f32")]
pub struct WarinessScore(f32);

impl TryFrom<f32> for WarinessScore {
    type Error = std::convert::Infallible;
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Ok(Self::new(value))
    }
}

impl WarinessScore {
    pub const MIN: f32 = 0.0;
    pub const MAX: f32 = 100.0;
    pub const NEUTRAL: WarinessScore = WarinessScore(0.0);

    pub fn new(value: f32) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }

    pub fn value(&self) -> f32 {
        self.0
    }

    pub fn add(self, delta: f32) -> Self {
        Self::new(self.0 + delta)
    }
}

impl Default for WarinessScore {
    fn default() -> Self {
        Self::NEUTRAL
    }
}

/// 4축이 *동시에* 받는 변동.
///
/// base_delta 표 + HEXACO 곱셈 결과 (Stage 2 정의/사용).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AxisDelta {
    pub trust: f32,
    pub affinity: f32,
    pub respect: f32,
    pub wariness: f32,
}

impl AxisDelta {
    /// 스칼라 곱 (intensity × HEXACO modifier 등).
    pub fn scaled_by(self, factor: f32) -> Self {
        Self {
            trust: self.trust * factor,
            affinity: self.affinity * factor,
            respect: self.respect * factor,
            wariness: self.wariness * factor,
        }
    }
}

/// 두 `AxisDelta` 성분별 합산 (Stage 2 — 복합 감정의 base_delta 합산에 사용).
impl std::ops::Add for AxisDelta {
    type Output = AxisDelta;
    fn add(self, other: AxisDelta) -> AxisDelta {
        AxisDelta {
            trust: self.trust + other.trust,
            affinity: self.affinity + other.affinity,
            respect: self.respect + other.respect,
            wariness: self.wariness + other.wariness,
        }
    }
}

/// 축 식별자 (base_delta 표 lookup용).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxisKind {
    Trust,
    Affinity,
    Respect,
    Wariness,
}

/// 4축 각각에 *곱해지는* 배수.
///
/// `hexaco_modifier`의 출력. `base_delta * intensity * modifier` 식으로
/// `update_axes_from_emotion`이 사용. Default는 모두 1.0 (보정 없음).
///
/// AxisDelta(변동량 +/-)와 AxisModifier(배수 ×)는 의미가 다르므로 별 타입으로 분리.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisModifier {
    pub trust: f32,
    pub affinity: f32,
    pub respect: f32,
    pub wariness: f32,
}

impl Default for AxisModifier {
    fn default() -> Self {
        Self {
            trust: 1.0,
            affinity: 1.0,
            respect: 1.0,
            wariness: 1.0,
        }
    }
}

impl AxisModifier {
    /// 모든 축에 동일한 곱셈 (전역 룰 — A+ Patience / C+ Prudence / A- Forgiveness).
    pub fn combine_uniform(self, factor: f32) -> Self {
        Self {
            trust: self.trust * factor,
            affinity: self.affinity * factor,
            respect: self.respect * factor,
            wariness: self.wariness * factor,
        }
    }

    /// 단일 축에만 곱셈 (축별 룰 — H+ Sincerity → trust / E+ Anxiety → wariness).
    pub fn scale_axis(mut self, kind: AxisKind, factor: f32) -> Self {
        match kind {
            AxisKind::Trust => self.trust *= factor,
            AxisKind::Affinity => self.affinity *= factor,
            AxisKind::Respect => self.respect *= factor,
            AxisKind::Wariness => self.wariness *= factor,
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_score_clamps_at_upper_bound() {
        assert_eq!(AxisScore::new(150.0).value(), 100.0);
    }

    #[test]
    fn axis_score_clamps_at_lower_bound() {
        assert_eq!(AxisScore::new(-200.0).value(), -100.0);
    }

    #[test]
    fn axis_score_normal_value_preserved() {
        assert_eq!(AxisScore::new(50.0).value(), 50.0);
    }

    #[test]
    fn wariness_score_floors_at_zero() {
        // ★ 핵심 — 음수 입력은 runtime에 0.0으로 floor
        assert_eq!(WarinessScore::new(-50.0).value(), 0.0);
    }

    #[test]
    fn wariness_score_caps_at_hundred() {
        assert_eq!(WarinessScore::new(150.0).value(), 100.0);
    }

    #[test]
    fn axis_score_add_clamps_high() {
        assert_eq!(AxisScore::new(50.0).add(60.0).value(), 100.0);
    }

    #[test]
    fn axis_score_add_clamps_low() {
        assert_eq!(AxisScore::new(-50.0).add(-60.0).value(), -100.0);
    }

    #[test]
    fn wariness_add_clamps_high() {
        assert_eq!(WarinessScore::new(80.0).add(50.0).value(), 100.0);
    }

    #[test]
    fn wariness_add_clamps_low() {
        assert_eq!(WarinessScore::new(30.0).add(-50.0).value(), 0.0);
    }

    #[test]
    fn neutral_consts_are_zero() {
        assert_eq!(AxisScore::NEUTRAL.value(), 0.0);
        assert_eq!(WarinessScore::NEUTRAL.value(), 0.0);
    }

    #[test]
    fn default_equals_neutral() {
        assert_eq!(AxisScore::default(), AxisScore::NEUTRAL);
        assert_eq!(WarinessScore::default(), WarinessScore::NEUTRAL);
    }

    #[test]
    fn axis_delta_scaled_by_factor() {
        let d = AxisDelta {
            trust: 20.0,
            affinity: 10.0,
            respect: 0.0,
            wariness: -10.0,
        };
        let scaled = d.scaled_by(0.5);
        assert_eq!(scaled.trust, 10.0);
        assert_eq!(scaled.affinity, 5.0);
        assert_eq!(scaled.respect, 0.0);
        assert_eq!(scaled.wariness, -5.0);
    }

    #[test]
    fn axis_delta_add_sums_componentwise() {
        let a = AxisDelta {
            trust: 10.0,
            affinity: 5.0,
            respect: 3.0,
            wariness: 2.0,
        };
        let b = AxisDelta {
            trust: 5.0,
            affinity: -2.0,
            respect: 0.0,
            wariness: 4.0,
        };
        let sum = a + b;
        assert_eq!(sum.trust, 15.0);
        assert_eq!(sum.affinity, 3.0);
        assert_eq!(sum.respect, 3.0);
        assert_eq!(sum.wariness, 6.0);
    }

    #[test]
    fn axis_score_serde_round_trip() {
        let s = AxisScore::new(75.0);
        let json = serde_json::to_string(&s).unwrap();
        let back: AxisScore = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value(), 75.0);
    }

    #[test]
    fn wariness_score_serde_round_trip() {
        let s = WarinessScore::new(50.0);
        let json = serde_json::to_string(&s).unwrap();
        let back: WarinessScore = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value(), 50.0);
    }

    /// ★ deserialize 경로도 ::new() clamp를 거쳐야 한다 (try_from invariant).
    /// 이전에는 newtype-transparent derive로 raw f32가 박혀 invariant가 깨졌다.
    #[test]
    fn axis_score_deserialize_clamps_out_of_range() {
        let high: AxisScore = serde_json::from_str("250.0").unwrap();
        assert_eq!(high.value(), 100.0);
        let low: AxisScore = serde_json::from_str("-250.0").unwrap();
        assert_eq!(low.value(), -100.0);
    }

    #[test]
    fn wariness_score_deserialize_floors_negative() {
        let neg: WarinessScore = serde_json::from_str("-50.0").unwrap();
        assert_eq!(neg.value(), 0.0);
        let high: WarinessScore = serde_json::from_str("250.0").unwrap();
        assert_eq!(high.value(), 100.0);
    }

    // -----------------------------------------------------------------------
    // AxisModifier (Stage 2.3)
    // -----------------------------------------------------------------------

    #[test]
    fn axis_modifier_default_is_unit() {
        let m = AxisModifier::default();
        assert_eq!(m.trust, 1.0);
        assert_eq!(m.affinity, 1.0);
        assert_eq!(m.respect, 1.0);
        assert_eq!(m.wariness, 1.0);
    }

    #[test]
    fn axis_modifier_combine_uniform_multiplies_all_axes() {
        let m = AxisModifier::default().combine_uniform(0.7);
        assert!((m.trust - 0.7).abs() < 1e-6);
        assert!((m.affinity - 0.7).abs() < 1e-6);
        assert!((m.respect - 0.7).abs() < 1e-6);
        assert!((m.wariness - 0.7).abs() < 1e-6);
    }

    #[test]
    fn axis_modifier_scale_axis_targets_single_axis() {
        let m = AxisModifier::default().scale_axis(AxisKind::Trust, 1.2);
        assert!((m.trust - 1.2).abs() < 1e-6);
        assert!((m.affinity - 1.0).abs() < 1e-6);
        assert!((m.respect - 1.0).abs() < 1e-6);
        assert!((m.wariness - 1.0).abs() < 1e-6);

        let m = AxisModifier::default().scale_axis(AxisKind::Wariness, 1.3);
        assert!((m.wariness - 1.3).abs() < 1e-6);
        assert!((m.trust - 1.0).abs() < 1e-6);
    }

    #[test]
    fn axis_modifier_chained_rules_compose() {
        // S2 임충 케이스: H+ Sincerity ×1.2 trust → A- Forgiveness ×1.5 전역 → C+ Prudence ×0.8 전역
        let m = AxisModifier::default()
            .scale_axis(AxisKind::Trust, 1.2)
            .combine_uniform(1.5)
            .combine_uniform(0.8);
        // trust: 1.0 * 1.2 * 1.5 * 0.8 = 1.44
        // others: 1.0 * 1.5 * 0.8 = 1.2
        assert!((m.trust - 1.44).abs() < 1e-6);
        assert!((m.affinity - 1.2).abs() < 1e-6);
        assert!((m.respect - 1.2).abs() < 1e-6);
        assert!((m.wariness - 1.2).abs() < 1e-6);
    }
}
