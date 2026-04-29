// wuxia-core/src/psychology/personality.rs
//
// ①층: HEXACO 성격 (HEXACO Personality)
// "이 사람은 어떤 행동 스타일을 가졌는가?"
//
// 거의 불변의 성격 기질. 오직 Tier 4 인생 성찰에서만 변경 가능하며,
// 최대 2개 요소를 ±5 범위에서만 조정할 수 있다.
//
// 6 요소 (0~100):
//   H — 정직-겸손 (Honesty-Humility): "공정한가?"
//   E — 정서성 (Emotionality): "안전/가족에 민감한가?"
//   X — 외향성 (eXtraversion): "사회적으로 자신감 있는가?"
//   A — 친화성 (Agreeableness): "도발에 참을 수 있는가?"
//   C — 성실성 (Conscientiousness): "규율 있고 계획적인가?"
//   O — 개방성 (Openness): "새로운 경험에 열린가?"
//
// 핵심 규칙:
//   - Tier 1~3: 성격 변경 불가
//   - Tier 4: 최대 2개 요소, 각 ±5
//   - 나이 드리프트 없음

use serde::{Deserialize, Serialize};

use crate::shared::id::CharacterId;

use super::event::PsychologyEvent;

// ---------------------------------------------------------------------------
// HexacoFactor — 6요소 분류
// ---------------------------------------------------------------------------

/// HEXACO 6요소 분류.
///
/// # Example
/// ```
/// use wuxia_core::psychology::HexacoFactor;
///
/// let f = HexacoFactor::HonestyHumility;
/// assert_eq!(format!("{:?}", f), "HonestyHumility");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HexacoFactor {
    /// H — 정직-겸손
    HonestyHumility,
    /// E — 정서성
    Emotionality,
    /// X — 외향성
    Extraversion,
    /// A — 친화성
    Agreeableness,
    /// C — 성실성
    Conscientiousness,
    /// O — 개방성
    Openness,
}

impl HexacoFactor {
    /// 모든 요소 배열.
    pub const ALL: [HexacoFactor; 6] = [
        HexacoFactor::HonestyHumility,
        HexacoFactor::Emotionality,
        HexacoFactor::Extraversion,
        HexacoFactor::Agreeableness,
        HexacoFactor::Conscientiousness,
        HexacoFactor::Openness,
    ];
}

// ---------------------------------------------------------------------------
// PsychologyError
// ---------------------------------------------------------------------------

/// 심리 도메인 에러.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PsychologyError {
    /// Tier 4 성격 변경에서 3개 이상의 요소를 변경하려 함.
    TooManyPersonalityChanges {
        attempted: usize,
        max_allowed: usize,
    },
}

impl std::fmt::Display for PsychologyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PsychologyError::TooManyPersonalityChanges {
                attempted,
                max_allowed,
            } => {
                write!(
                    f,
                    "성격 변경 초과: {}개 시도 (최대 {}개)",
                    attempted, max_allowed
                )
            }
        }
    }
}

impl std::error::Error for PsychologyError {}

// ---------------------------------------------------------------------------
// HexacoPersonality — ①층 집약체
// ---------------------------------------------------------------------------

/// Tier 4 변경 시 최대 변경 가능 요소 수.
const MAX_TIER4_CHANGES: usize = 2;
/// Tier 4 변경 시 요소당 최대 변경 범위.
const MAX_TIER4_DELTA: i32 = 5;

/// ①층 HEXACO 성격.
///
/// 거의 불변이며, 오직 Tier 4 인생 성찰에서만 변경 가능.
/// 변경 제약: 최대 2개 요소, 각 ±5.
///
/// # Example
/// ```
/// use wuxia_core::psychology::{HexacoPersonality, HexacoFactor};
/// use wuxia_core::shared::CharacterId;
///
/// // 명경: H90 E50 X50 A80 C90 O60
/// let personality = HexacoPersonality::new(
///     CharacterId::new(1), 90, 50, 50, 80, 90, 60,
/// );
/// assert_eq!(personality.get(HexacoFactor::HonestyHumility), 90);
/// assert_eq!(personality.get(HexacoFactor::Agreeableness), 80);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HexacoPersonality {
    character_id: CharacterId,
    honesty_humility: u32,
    emotionality: u32,
    extraversion: u32,
    agreeableness: u32,
    conscientiousness: u32,
    openness: u32,
}

impl HexacoPersonality {
    /// 새 HEXACO 성격을 생성한다. 각 값은 0~100으로 클램프된다.
    pub fn new(
        character_id: CharacterId,
        h: u32,
        e: u32,
        x: u32,
        a: u32,
        c: u32,
        o: u32,
    ) -> Self {
        Self {
            character_id,
            honesty_humility: h.min(100),
            emotionality: e.min(100),
            extraversion: x.min(100),
            agreeableness: a.min(100),
            conscientiousness: c.min(100),
            openness: o.min(100),
        }
    }

    // -- Getters --

    pub fn character_id(&self) -> CharacterId {
        self.character_id
    }

    /// 특정 요소의 값을 반환한다.
    pub fn get(&self, factor: HexacoFactor) -> u32 {
        match factor {
            HexacoFactor::HonestyHumility => self.honesty_humility,
            HexacoFactor::Emotionality => self.emotionality,
            HexacoFactor::Extraversion => self.extraversion,
            HexacoFactor::Agreeableness => self.agreeableness,
            HexacoFactor::Conscientiousness => self.conscientiousness,
            HexacoFactor::Openness => self.openness,
        }
    }

    // 편의 getters
    pub fn h(&self) -> u32 {
        self.honesty_humility
    }
    pub fn e(&self) -> u32 {
        self.emotionality
    }
    pub fn x(&self) -> u32 {
        self.extraversion
    }
    pub fn a(&self) -> u32 {
        self.agreeableness
    }
    pub fn c(&self) -> u32 {
        self.conscientiousness
    }
    pub fn o(&self) -> u32 {
        self.openness
    }

    // -- Commands --

    /// Tier 4 인생 성찰에서 성격을 변경한다.
    ///
    /// # 제약
    /// - 최대 2개 요소만 변경 가능
    /// - 각 요소당 ±5 범위
    /// - 결과값은 0~100으로 클램프
    ///
    /// # Errors
    /// - `PsychologyError::TooManyPersonalityChanges`: 3개 이상 요소 변경 시도
    pub fn apply_tier4_change(
        &mut self,
        changes: &[(HexacoFactor, i32)],
    ) -> Result<Vec<PsychologyEvent>, PsychologyError> {
        // 실제 변화가 있는 요소만 카운트
        let effective_changes: Vec<(HexacoFactor, i32)> = changes
            .iter()
            .filter(|(_, delta)| *delta != 0)
            .copied()
            .collect();

        if effective_changes.len() > MAX_TIER4_CHANGES {
            return Err(PsychologyError::TooManyPersonalityChanges {
                attempted: effective_changes.len(),
                max_allowed: MAX_TIER4_CHANGES,
            });
        }

        let mut events = Vec::new();

        for (factor, delta) in &effective_changes {
            let clamped = if *delta > MAX_TIER4_DELTA {
                MAX_TIER4_DELTA
            } else if *delta < -MAX_TIER4_DELTA {
                -MAX_TIER4_DELTA
            } else {
                *delta
            };
            let field = self.field_mut(*factor);
            let old = *field;
            let raw = (old as i32 + clamped).max(0).min(100);
            let new_val = raw as u32;
            *field = new_val;

            if new_val != old {
                events.push(PsychologyEvent::PersonalityChanged {
                    character_id: self.character_id,
                    factor: *factor,
                    old_value: old,
                    new_value: new_val,
                });
            }
        }

        Ok(events)
    }

    // -- Derived metrics --

    /// 정서적 반응성 (E 기반).
    /// E가 높을수록 두려움/공감에 민감하다.
    /// 범위: 0.0 ~ 1.0
    pub fn emotional_reactivity(&self) -> f32 {
        self.emotionality as f32 / 100.0
    }

    /// 분노 억제력 (A 기반).
    /// A가 높을수록 분노를 잘 억제한다.
    /// 범위: 0.0 ~ 1.0
    pub fn anger_suppression(&self) -> f32 {
        self.agreeableness as f32 / 100.0
    }

    /// 도덕적 민감도 (H 기반).
    /// H가 높을수록 죄책감/수치를 강하게 느낀다.
    /// 범위: 0.0 ~ 1.0
    pub fn moral_sensitivity(&self) -> f32 {
        self.honesty_humility as f32 / 100.0
    }

    /// 충동 억제력 (C 기반).
    /// C가 높을수록 감정→행동 전환을 억제한다.
    /// 범위: 0.0 ~ 1.0
    pub fn impulse_control(&self) -> f32 {
        self.conscientiousness as f32 / 100.0
    }

    /// 복합 감정 수용력 (O 기반).
    /// O가 높을수록 여러 감정을 동시에 경험할 수 있다.
    /// 범위: 0.0 ~ 1.0
    pub fn complex_emotion_tolerance(&self) -> f32 {
        self.openness as f32 / 100.0
    }

    // -- Internal --

    fn field_mut(&mut self, factor: HexacoFactor) -> &mut u32 {
        match factor {
            HexacoFactor::HonestyHumility => &mut self.honesty_humility,
            HexacoFactor::Emotionality => &mut self.emotionality,
            HexacoFactor::Extraversion => &mut self.extraversion,
            HexacoFactor::Agreeableness => &mut self.agreeableness,
            HexacoFactor::Conscientiousness => &mut self.conscientiousness,
            HexacoFactor::Openness => &mut self.openness,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "personality_tests.rs"]
mod tests;
