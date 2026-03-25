//! 상황(Situation) — 감정 생성의 입력
//!
//! OCC 3대 분기(Event, Action, Object)를 `Vec<SituationFocus>`로 동시에 받을 수 있다.
//! 엔진은 순수 함수 — ID 없이 전부 Value Object.
//! 상황/감정 추적은 게임 시스템의 책임.
//!
//! ## v2 변경사항
//!
//! - Situation.focus → Situation.focuses: Vec (3분기 동시 수용)
//! - EventFocus.desirability_for_other → DesirabilityForOther (대상 정보 포함)
//! - is_prospective + prior_expectation → Option<Prospect> 통합
//! - ActionFocus에서 outcome_for_self 제거 (Event 동시 전달로 대체)

use serde::{Deserialize, Serialize};

use crate::domain::relationship::Relationship;

// ---------------------------------------------------------------------------
// Situation (Value Object)
// ---------------------------------------------------------------------------

/// 상황 설명 — 감정 엔진의 입력
///
/// Value Object — ID 없음. 게임 시스템이 외부에서 추적.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Situation {
    /// 상황 설명 텍스트 (LLM 가이드용)
    pub description: String,
    /// OCC 3분기 초점 (1~3개, Event/Action/Object 동시 가능)
    ///
    /// 같은 사건에 대한 여러 관점을 동시에 담는다.
    /// "사형제가 밀고하고 독을 탔다" →
    ///   [Action(밀고 비난), Event(독 피해), Object(독약 혐오)]
    ///
    /// Compound 감정(Anger, Gratitude 등)은 엔진이
    /// Vec에서 Action+Event 동시 존재를 감지하여 자동 생성.
    pub focuses: Vec<SituationFocus>,
}

// ---------------------------------------------------------------------------
// SituationFocus (enum 유지 — Value Object)
// ---------------------------------------------------------------------------

/// 상황의 초점 — OCC 3대 분기와 대응
///
/// Value Object — enum 유지하면서 Vec으로 동시 수용.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SituationFocus {
    /// 사건 발생 (누군가에게 무슨 일이 일어남)
    Event(EventFocus),
    /// 행동 평가 (누군가가 무엇을 했음)
    Action(ActionFocus),
    /// 대상 인식 (무언가를 접함)
    Object(ObjectFocus),
}

// ---------------------------------------------------------------------------
// EventFocus (Value Object)
// ---------------------------------------------------------------------------

/// 사건 초점 — Well-being, Fortune-of-others, Prospect 하위 분기
///
/// Value Object
///
/// - desirability_for_self: 필수. 모든 사건은 나에게 영향이 있다.
/// - desirability_for_other: 선택. 타인 관련 시 대상 정보 포함.
/// - prospect: 선택. None이면 현재 사건, Some이면 전망 관련.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFocus {
    /// 사건이 자신에게 바람직한 정도 (-1.0 ~ 1.0)
    /// 양수 → Joy, 음수 → Distress
    pub desirability_for_self: f32,
    /// 사건이 타인에게 바람직한 정도 (대상 정보 포함)
    /// Fortune-of-others 분기: HappyFor, Pity, Gloating, Resentment
    pub desirability_for_other: Option<DesirabilityForOther>,
    /// 전망 정보 (None이면 현재/과거 사건)
    /// Prospect 분기: Hope, Fear, Satisfaction, Disappointment, Relief, FearsConfirmed
    pub prospect: Option<Prospect>,
}

/// 타인에 대한 바람직함 — Fortune-of-others 분기용
///
/// Value Object
///
/// 대화 상대와 사건의 영향 대상이 다를 수 있다.
/// "무백이 교룡과 대화 중, 소호가 비무에서 패했다" →
///   대화 상대 관계: 무백→교룡 (appraise의 relationship)
///   사건 대상 관계: 무백→소호 (여기의 relationship)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesirabilityForOther {
    /// 사건의 영향을 받는 타인의 ID
    pub target_id: String,
    /// 그 사람에게 얼마나 좋은/나쁜 일인가 (-1.0 ~ 1.0)
    pub desirability: f32,
    /// 나와 그 사람의 관계 (호출자가 조회하여 제공)
    pub relationship: Relationship,
}

// ---------------------------------------------------------------------------
// Prospect (Value Object) — 전망 시퀀스
// ---------------------------------------------------------------------------

/// 전망 — 미래 예측 또는 이전 전망의 확인
///
/// Value Object
///
/// Anticipation: 미래 전망 → Hope 또는 Fear 생성
/// Confirmation: 이전 전망의 확인 결과 → Satisfaction/Disappointment/Relief/FearsConfirmed
///
/// 전망 시퀀스 연결(어떤 Hope가 어떤 확인과 이어지는지)은
/// 게임 시스템의 책임. 엔진은 Confirmation의 result만 보고 감정을 생성.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Prospect {
    /// 미래 전망 (Hope/Fear 생성)
    /// desirability_for_self 양수 → Hope, 음수 → Fear
    Anticipation,
    /// 이전 전망의 확인 결과
    /// 게임 시스템이 "어떤 Hope/Fear의 결과인지" 판단하여 result를 설정
    Confirmation(ProspectResult),
}

/// 전망 확인 결과
///
/// Value Object
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ProspectResult {
    /// 바랐던 일이 실현됨 → Satisfaction
    HopeFulfilled,
    /// 바랐던 일이 실현되지 않음 → Disappointment
    HopeUnfulfilled,
    /// 두려워했던 일이 실현되지 않음 → Relief
    FearUnrealized,
    /// 두려워했던 일이 실현됨 → FearsConfirmed
    FearConfirmed,
}

// ---------------------------------------------------------------------------
// ActionFocus (Value Object)
// ---------------------------------------------------------------------------

/// 행동 초점 — Attribution (Pride/Shame/Admiration/Reproach)
///
/// Value Object
///
/// outcome_for_self는 제거됨.
/// 행동의 결과가 나에게 미친 영향은 EventFocus로 동시 전달하여
/// 엔진이 Compound 감정(Anger, Gratitude 등)을 자동 생성.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionFocus {
    /// 행위자가 자기 자신인지
    pub is_self_agent: bool,
    /// 행동의 칭찬받을만한 정도 (-1.0=비난, +1.0=칭찬)
    pub praiseworthiness: f32,
}

// ---------------------------------------------------------------------------
// ObjectFocus (Value Object)
// ---------------------------------------------------------------------------

/// 대상 초점 — Love/Hate
///
/// Value Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectFocus {
    /// 대상의 매력도 (-1.0=혐오, +1.0=매력)
    pub appealingness: f32,
}

// ---------------------------------------------------------------------------
// Situation 헬퍼
// ---------------------------------------------------------------------------

impl Situation {
    /// focuses에서 첫 번째 EventFocus 찾기
    pub fn find_event(&self) -> Option<&EventFocus> {
        self.focuses.iter().find_map(|f| match f {
            SituationFocus::Event(e) => Some(e),
            _ => None,
        })
    }

    /// focuses에서 첫 번째 ActionFocus 찾기
    pub fn find_action(&self) -> Option<&ActionFocus> {
        self.focuses.iter().find_map(|f| match f {
            SituationFocus::Action(a) => Some(a),
            _ => None,
        })
    }

    /// focuses에서 첫 번째 ObjectFocus 찾기
    pub fn find_object(&self) -> Option<&ObjectFocus> {
        self.focuses.iter().find_map(|f| match f {
            SituationFocus::Object(o) => Some(o),
            _ => None,
        })
    }
}
