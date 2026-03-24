//! 상황(Situation) — 감정 생성의 입력

use serde::{Deserialize, Serialize};

/// 상황의 초점 — OCC 3대 분기와 대응
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SituationFocus {
    /// 사건 발생 (누군가에게 무슨 일이 일어남)
    Event {
        /// 사건이 자신에게 바람직한 정도 (-1.0 ~ 1.0)
        desirability_for_self: f32,
        /// 사건이 타인에게 바람직한 정도 (-1.0 ~ 1.0, None이면 해당 없음)
        desirability_for_other: Option<f32>,
        /// 미래 사건인지 (true면 prospect-based 감정)
        is_prospective: bool,
        /// 이전에 예상했던 사건의 실현 여부 (None이면 새 사건)
        prior_expectation: Option<PriorExpectation>,
    },
    /// 행동 평가 (누군가가 무엇을 했음)
    Action {
        /// 행위자가 자기 자신인지
        is_self_agent: bool,
        /// 행동의 칭찬받을만한 정도 (-1.0=비난, +1.0=칭찬)
        praiseworthiness: f32,
        /// 행동의 결과가 자신에게 미친 영향 (-1.0 ~ 1.0, None이면 해당 없음)
        outcome_for_self: Option<f32>,
    },
    /// 대상 인식 (무언가를 접함)
    Object {
        /// 대상의 매력도 (-1.0=혐오, +1.0=매력)
        appealingness: f32,
    },
}

/// 이전 기대 상태 (prospect-based 감정용)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PriorExpectation {
    /// 바랐던 일이 실현됨 → Satisfaction
    HopeFulfilled,
    /// 바랐던 일이 실현되지 않음 → Disappointment
    HopeUnfulfilled,
    /// 두려워했던 일이 실현되지 않음 → Relief
    FearUnrealized,
    /// 두려워했던 일이 실현됨 → FearsConfirmed
    FearConfirmed,
}

/// 상황 설명 — 감정 엔진의 입력
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Situation {
    /// 상황 설명 텍스트
    pub description: String,
    /// 상황의 초점
    pub focus: SituationFocus,
}
