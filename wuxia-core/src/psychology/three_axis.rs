// wuxia-core/src/psychology/three_axis.rs
//
// ②층: 3축 가치관 (Three-Axis Core Values)
// "이 사람의 존재 방식은 무엇인가?"
//
// LLM 서사 프롬프트에 사용되는 핵심 가치관.
// 같은 수치라도 신조(creed)의 방향에 따라 완전히 다른 인물이 된다.
//
// 3축:
//   믿음(信/Trust)     — "사람을 믿을 수 있는가?"
//   옳음(正/Rightness) — "옳은 것을 위해 잃을 수 있는가?"
//   바람(願/Want)      — "나는 무엇을 바라는가?"
//
// 핵심 원칙:
//   같은 옳음(正) 90이라도:
//     명경: "도의를 지켜야 한다" → 충↑ 의↑
//     조고: "힘이 곧 정의다"    → 충↓ 의↓ 야망↑

use serde::{Deserialize, Serialize};

use crate::shared::id::{CharacterId, MemoryId};

use super::appraisal::ReflectionTier;
use super::event::PsychologyEvent;

// ---------------------------------------------------------------------------
// AxisType — 3축 분류
// ---------------------------------------------------------------------------

/// 3축 가치관 분류.
///
/// # Example
/// ```
/// use wuxia_core::psychology::AxisType;
///
/// let axis = AxisType::Trust;
/// assert_eq!(format!("{:?}", axis), "Trust");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AxisType {
    /// 믿음(信) — "사람을 믿을 수 있는가?"
    Trust,
    /// 옳음(正) — "옳은 것을 위해 잃을 수 있는가?"
    Rightness,
    /// 바람(願) — "나는 무엇을 바라는가?"
    Want,
}

impl AxisType {
    /// 모든 축 타입 배열.
    pub const ALL: [AxisType; 3] = [AxisType::Trust, AxisType::Rightness, AxisType::Want];
}

// ---------------------------------------------------------------------------
// CreedCandidate — 대안 신조 후보
// ---------------------------------------------------------------------------

/// 대안 신조 후보.
///
/// NPC가 다른 인물이나 사건을 통해 접촉한 대안적 신조.
/// 접촉 횟수와 공명도가 쌓이면 Tier 3~4 성찰에서 신조 전환이 일어날 수 있다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreedCandidate {
    text: String,
    source: String,
    exposure_count: u32,
    resonance: f32, // 0.0~100.0
}

impl CreedCandidate {
    pub fn new(text: String, source: String) -> Self {
        Self {
            text,
            source,
            exposure_count: 1,
            resonance: 0.0,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn exposure_count(&self) -> u32 {
        self.exposure_count
    }
    pub fn resonance(&self) -> f32 {
        self.resonance
    }

    /// 접촉 횟수를 1 증가시킨다.
    pub fn increment_exposure(&mut self) {
        self.exposure_count = self.exposure_count.saturating_add(1);
    }

    /// 공명도를 설정한다. 0.0~100.0으로 클램프된다.
    pub fn set_resonance(&mut self, resonance: f32) {
        self.resonance = resonance.clamp(0.0, 100.0);
    }
}

// ---------------------------------------------------------------------------
// ValueAxis — 축별 상세 구조
// ---------------------------------------------------------------------------

/// 3축 가치관의 한 축.
///
/// 강도(intensity) + 신조(creed) + 대안 후보 + 형성 기억으로 구성.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueAxis {
    intensity: f32,
    creed: String,
    #[serde(default)]
    creed_candidates: Vec<CreedCandidate>,
    #[serde(default)]
    formation_memories: Vec<MemoryId>,
}

impl ValueAxis {
    pub fn new(intensity: f32, creed: String) -> Self {
        Self {
            intensity: intensity.clamp(0.0, 100.0),
            creed,
            creed_candidates: Vec::new(),
            formation_memories: Vec::new(),
        }
    }

    pub fn intensity(&self) -> f32 {
        self.intensity
    }
    pub fn creed(&self) -> &str {
        &self.creed
    }
    pub fn creed_candidates(&self) -> &[CreedCandidate] {
        &self.creed_candidates
    }
    pub fn formation_memories(&self) -> &[MemoryId] {
        &self.formation_memories
    }
}

// ---------------------------------------------------------------------------
// ThreeAxisValues — ②층 집약체
// ---------------------------------------------------------------------------

/// ②층 3축 가치관.
///
/// LLM 서사 프롬프트의 핵심 입력이며,
/// 같은 강도라도 신조 방향에 따라 완전히 다른 인물을 만든다.
///
/// # Tier별 강도 변경 범위
/// - Tier 1 (순간): ±5
/// - Tier 2 (일상): ±10
/// - Tier 3 (전환점): ±20
/// - Tier 4 (인생): ±30
///
/// # Example
/// ```
/// use wuxia_core::psychology::{ThreeAxisValues, AxisType, ValueAxis, ReflectionTier};
/// use wuxia_core::shared::CharacterId;
///
/// let mut values = ThreeAxisValues::new(
///     CharacterId::new(1),
///     ValueAxis::new(80.0, "사람을 믿는다".to_string()),
///     ValueAxis::new(90.0, "도의를 지켜야 한다".to_string()),
///     ValueAxis::new(50.0, "제자들을 지키겠다".to_string()),
/// );
/// assert_eq!(values.axis(AxisType::Trust).intensity(), 80.0);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreeAxisValues {
    character_id: CharacterId,
    trust: ValueAxis,
    rightness: ValueAxis,
    want: ValueAxis,
}

/// Tier별 강도 최대 변경 범위.
fn tier_max_delta(tier: &ReflectionTier) -> f32 {
    match tier {
        ReflectionTier::Instant => 5.0,
        ReflectionTier::Daily => 10.0,
        ReflectionTier::TurningPoint => 20.0,
        ReflectionTier::Life => 30.0,
    }
}

impl ThreeAxisValues {
    pub fn new(
        character_id: CharacterId,
        trust: ValueAxis,
        rightness: ValueAxis,
        want: ValueAxis,
    ) -> Self {
        Self {
            character_id,
            trust,
            rightness,
            want,
        }
    }

    // -- Getters --

    pub fn character_id(&self) -> CharacterId {
        self.character_id
    }

    pub fn trust(&self) -> &ValueAxis {
        &self.trust
    }
    pub fn rightness(&self) -> &ValueAxis {
        &self.rightness
    }
    pub fn want(&self) -> &ValueAxis {
        &self.want
    }

    /// 축 타입으로 해당 축에 접근한다.
    pub fn axis(&self, axis_type: AxisType) -> &ValueAxis {
        match axis_type {
            AxisType::Trust => &self.trust,
            AxisType::Rightness => &self.rightness,
            AxisType::Want => &self.want,
        }
    }

    // -- Commands --

    /// 축의 강도를 조정한다. Tier별 범위를 초과하면 클램프.
    ///
    /// 실제 변화 없으면 빈 Vec 반환 (no-op rule).
    pub fn adjust_intensity(
        &mut self,
        axis: AxisType,
        delta: f32,
        tier: ReflectionTier,
    ) -> Vec<PsychologyEvent> {
        let max_delta = tier_max_delta(&tier);
        let clamped_delta = delta.clamp(-max_delta, max_delta);

        let va = self.axis_mut(axis);
        let old = va.intensity;
        va.intensity = (old + clamped_delta).clamp(0.0, 100.0);
        let new = va.intensity;

        if (new - old).abs() < f32::EPSILON {
            return vec![];
        }

        vec![PsychologyEvent::AxisIntensityChanged {
            character_id: self.character_id,
            axis,
            old_value: old,
            new_value: new,
            tier,
        }]
    }

    /// 축의 신조를 변경한다.
    ///
    /// 같은 신조이면 빈 Vec 반환 (no-op rule).
    pub fn update_creed(
        &mut self,
        axis: AxisType,
        new_creed: String,
    ) -> Vec<PsychologyEvent> {
        let va = self.axis_mut(axis);
        if va.creed == new_creed {
            return vec![];
        }

        let old_creed = std::mem::replace(&mut va.creed, new_creed.clone());

        vec![PsychologyEvent::CreedChanged {
            character_id: self.character_id,
            axis,
            old_creed,
            new_creed,
        }]
    }

    /// 대안 신조 후보를 추가한다.
    pub fn add_creed_candidate(
        &mut self,
        axis: AxisType,
        candidate: CreedCandidate,
    ) -> Vec<PsychologyEvent> {
        let text = candidate.text().to_string();
        let va = self.axis_mut(axis);
        va.creed_candidates.push(candidate);

        vec![PsychologyEvent::CreedCandidateAdded {
            character_id: self.character_id,
            axis,
            candidate_text: text,
        }]
    }

    /// 대안 후보의 접촉 횟수를 증가시킨다.
    /// 인덱스 범위를 벗어나면 아무 일도 하지 않는다.
    pub fn increment_candidate_exposure(&mut self, axis: AxisType, idx: usize) {
        let va = self.axis_mut(axis);
        if let Some(candidate) = va.creed_candidates.get_mut(idx) {
            candidate.increment_exposure();
        }
    }

    /// 형성 기억을 추가한다.
    pub fn add_formation_memory(&mut self, axis: AxisType, memory_id: MemoryId) {
        let va = self.axis_mut(axis);
        va.formation_memories.push(memory_id);
    }

    // -- Internal --

    fn axis_mut(&mut self, axis: AxisType) -> &mut ValueAxis {
        match axis {
            AxisType::Trust => &mut self.trust,
            AxisType::Rightness => &mut self.rightness,
            AxisType::Want => &mut self.want,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "three_axis_tests.rs"]
mod tests;
