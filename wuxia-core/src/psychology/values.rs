// wuxia-core/src/psychology/values.rs
//
// ③층: 실천 가치 (Practical Values) — "이 사건이 이 사람에게 얼마나 중요한가?"
//
// OCC 감정 평가의 직접 입력으로 사용되는 5가지 무협 가치.
// 코드에서 공식으로 계산한다 (<1ms).
//
// 5가치:
//   충(忠) — 조직/주군 헌신      → 조직 위협 시 분노/두려움
//   의(義) — 강호 도리           → 도덕 위반 시 분노/경멸
//   효(孝) — 가족/스승 도리      → 가족 위험 시 걱정/두려움
//   복수(復) — 원한 갚음         → 원수 출현 시 증오/결의
//   야망(野) — 높은 곳 추구      → 기회 시 흥분 / 좌절 시 분노
//
// 핵심 공식:
//   감정_강도 = 기본강도 × 해당_가치_수치 / 100

use serde::{Deserialize, Serialize};

use crate::shared::id::CharacterId;

use super::appraisal::ReflectionTier;
use super::event::PsychologyEvent;

// ---------------------------------------------------------------------------
// PracticalValueType — 5가치 분류
// ---------------------------------------------------------------------------

/// 5가지 실천 가치 분류.
///
/// OCC 감정 평가 시 관련 가치를 지정하는 데 사용한다.
///
/// # Example
/// ```
/// use wuxia_core::psychology::PracticalValueType;
///
/// let vt = PracticalValueType::Righteousness;
/// assert_eq!(format!("{:?}", vt), "Righteousness");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PracticalValueType {
    /// 충(忠) — 조직/주군에 헌신하는가?
    Loyalty,
    /// 의(義) — 강호 도리를 지키는가?
    Righteousness,
    /// 효(孝) — 가족/스승에 도리를 다하는가?
    FilialPiety,
    /// 복수(復) — 원한을 갚으려 하는가?
    Vengeance,
    /// 야망(野) — 높은 곳을 원하는가?
    Ambition,
}

impl PracticalValueType {
    /// 모든 가치 타입을 순회하기 위한 배열.
    pub const ALL: [PracticalValueType; 5] = [
        PracticalValueType::Loyalty,
        PracticalValueType::Righteousness,
        PracticalValueType::FilialPiety,
        PracticalValueType::Vengeance,
        PracticalValueType::Ambition,
    ];
}

// ---------------------------------------------------------------------------
// PracticalValues — ③층 집약체
// ---------------------------------------------------------------------------

/// ③층 실천 가치 (5가치).
///
/// OCC 감정 계산의 가중치로 직접 사용된다.
/// 각 가치는 0.0~100.0 범위로 클램프된다.
///
/// # Tier별 변경 범위
/// - Tier 1 (순간): ±5
/// - Tier 2 (일상): ±10
/// - Tier 3 (전환점): ±20
/// - Tier 4 (인생): ±20
///
/// # Example
/// ```
/// use wuxia_core::psychology::{PracticalValues, PracticalValueType, ReflectionTier};
/// use wuxia_core::shared::CharacterId;
///
/// let mut values = PracticalValues::new(
///     CharacterId::new(1),
///     90.0, 90.0, 70.0, 30.0, 20.0,
/// );
/// assert_eq!(values.get(PracticalValueType::Loyalty), 90.0);
///
/// let events = values.adjust(PracticalValueType::Vengeance, 10.0, ReflectionTier::Daily);
/// assert_eq!(values.get(PracticalValueType::Vengeance), 40.0);
/// assert_eq!(events.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PracticalValues {
    character_id: CharacterId,
    loyalty: f32,
    righteousness: f32,
    filial_piety: f32,
    vengeance: f32,
    ambition: f32,
}

/// Tier별 최대 변경 범위.
fn tier_max_delta(tier: &ReflectionTier) -> f32 {
    match tier {
        ReflectionTier::Instant => 5.0,
        ReflectionTier::Daily => 10.0,
        ReflectionTier::TurningPoint => 20.0,
        ReflectionTier::Life => 20.0,
    }
}

/// 값을 0.0~100.0 범위로 클램프.
fn clamp_value(v: f32) -> f32 {
    v.clamp(0.0, 100.0)
}

impl PracticalValues {
    /// 새 실천 가치 생성. 모든 값은 0.0~100.0으로 클램프된다.
    pub fn new(
        character_id: CharacterId,
        loyalty: f32,
        righteousness: f32,
        filial_piety: f32,
        vengeance: f32,
        ambition: f32,
    ) -> Self {
        Self {
            character_id,
            loyalty: clamp_value(loyalty),
            righteousness: clamp_value(righteousness),
            filial_piety: clamp_value(filial_piety),
            vengeance: clamp_value(vengeance),
            ambition: clamp_value(ambition),
        }
    }

    // -- Getters --

    pub fn character_id(&self) -> CharacterId {
        self.character_id
    }

    /// 특정 가치의 현재 수치를 반환한다.
    pub fn get(&self, value_type: PracticalValueType) -> f32 {
        match value_type {
            PracticalValueType::Loyalty => self.loyalty,
            PracticalValueType::Righteousness => self.righteousness,
            PracticalValueType::FilialPiety => self.filial_piety,
            PracticalValueType::Vengeance => self.vengeance,
            PracticalValueType::Ambition => self.ambition,
        }
    }

    pub fn loyalty(&self) -> f32 {
        self.loyalty
    }
    pub fn righteousness(&self) -> f32 {
        self.righteousness
    }
    pub fn filial_piety(&self) -> f32 {
        self.filial_piety
    }
    pub fn vengeance(&self) -> f32 {
        self.vengeance
    }
    pub fn ambition(&self) -> f32 {
        self.ambition
    }

    // -- Commands --

    /// 가치를 조정한다. Tier별 범위를 초과하면 클램프한다.
    ///
    /// delta가 0이거나 실제 변화가 없으면 빈 Vec를 반환한다 (no-op rule).
    pub fn adjust(
        &mut self,
        value_type: PracticalValueType,
        delta: f32,
        tier: ReflectionTier,
    ) -> Vec<PsychologyEvent> {
        let max_delta = tier_max_delta(&tier);
        let clamped_delta = delta.clamp(-max_delta, max_delta);

        let field = self.field_mut(value_type);
        let old = *field;
        *field = clamp_value(old + clamped_delta);
        let new = *field;

        // no-op rule: 실제 변화 없으면 이벤트 생성하지 않음
        if (new - old).abs() < f32::EPSILON {
            return vec![];
        }

        vec![PsychologyEvent::PracticalValueChanged {
            character_id: self.character_id,
            value_type,
            old_value: old,
            new_value: new,
            tier,
        }]
    }

    // -- Derived metrics --

    /// 정의로운 정렬 점수 (충+의+효 vs 복수+야망).
    ///
    /// 양수이면 의로운 방향, 음수이면 야망/복수 방향.
    /// 범위: -200.0 ~ +300.0
    pub fn alignment(&self) -> f32 {
        (self.loyalty + self.righteousness + self.filial_piety)
            - (self.vengeance + self.ambition)
    }

    /// 배신 가능성 지표.
    ///
    /// 야망이 높고 충/의가 낮을수록 높다.
    /// 범위: 0.0 ~ 1.0
    pub fn betrayal_potential(&self) -> f32 {
        let result = self.ambition / 100.0
            * (1.0 - self.loyalty / 100.0)
            * (1.0 - self.righteousness / 100.0);
        result.clamp(0.0, 1.0)
    }

    // -- Internal --

    fn field_mut(&mut self, value_type: PracticalValueType) -> &mut f32 {
        match value_type {
            PracticalValueType::Loyalty => &mut self.loyalty,
            PracticalValueType::Righteousness => &mut self.righteousness,
            PracticalValueType::FilialPiety => &mut self.filial_piety,
            PracticalValueType::Vengeance => &mut self.vengeance,
            PracticalValueType::Ambition => &mut self.ambition,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "values_tests.rs"]
mod tests;
