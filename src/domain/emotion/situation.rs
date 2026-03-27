//! 상황(Situation) — 감정 생성의 입력
//!
//! OCC 3대 분기(Event, Action, Object)를 각각 Option 필드로 표현.
//! 컴파일 타임에 각 타입 최대 1개 보장, 최소 1개는 스마트 생성자로 검증.
//! 엔진은 순수 함수 — ID 없이 전부 Value Object.
//! 상황/감정 추적은 게임 시스템의 책임.
//!
//! ## v3 변경사항
//!
//! - Situation.focuses: Vec<SituationFocus> → event/action/object: Option
//! - SituationFocus enum 제거 — 컴파일 타임 타입 안전성 확보
//! - Situation::new() 스마트 생성자로 "최소 1개" 불변식 보장
//!
//! ## v2 변경사항
//!
//! - EventFocus.desirability_for_other → DesirabilityForOther (대상 정보 포함)
//! - is_prospective + prior_expectation → Option<Prospect> 통합
//! - ActionFocus에서 outcome_for_self 제거 (Event 동시 전달로 대체)

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::domain::relationship::Relationship;

// ---------------------------------------------------------------------------
// SituationError
// ---------------------------------------------------------------------------

/// Situation 생성 오류
#[derive(Debug, Clone)]
pub enum SituationError {
    /// Event, Action, Object 중 하나 이상 필요
    NoFocus,
}

impl fmt::Display for SituationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFocus => write!(f, "최소 하나의 Focus(Event/Action/Object)가 필요합니다"),
        }
    }
}

impl std::error::Error for SituationError {}

// ---------------------------------------------------------------------------
// Situation (Value Object)
// ---------------------------------------------------------------------------

/// 상황 설명 — 감정 엔진의 입력
///
/// Value Object — ID 없음. 게임 시스템이 외부에서 추적.
///
/// OCC 3분기 초점을 각각 Option으로 보유.
/// 컴파일 타임에 각 타입 최대 1개 보장.
/// 최소 1개는 `Situation::new()`에서 런타임 검증.
///
/// "사형제가 밀고하고 독을 탔다" →
///   event: Some(독 피해), action: Some(밀고 비난), object: Some(독약 혐오)
///
/// Compound 감정(Anger, Gratitude 등)은 엔진이
/// action + event 동시 존재를 감지하여 자동 생성.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Situation {
    /// 상황 설명 텍스트 (LLM 가이드용)
    pub description: String,
    /// 사건 초점 (누군가에게 무슨 일이 일어남)
    pub event: Option<EventFocus>,
    /// 행동 초점 (누군가가 무엇을 했음)
    pub action: Option<ActionFocus>,
    /// 대상 초점 (무언가를 접함)
    pub object: Option<ObjectFocus>,
}

impl Situation {
    /// 스마트 생성자 — 최소 1개 Focus 불변식 보장
    pub fn new(
        description: impl Into<String>,
        event: Option<EventFocus>,
        action: Option<ActionFocus>,
        object: Option<ObjectFocus>,
    ) -> Result<Self, SituationError> {
        if event.is_none() && action.is_none() && object.is_none() {
            return Err(SituationError::NoFocus);
        }
        Ok(Self {
            description: description.into(),
            event,
            action,
            object,
        })
    }

    /// Focus 개수 반환
    pub fn focus_count(&self) -> usize {
        self.event.is_some() as usize
            + self.action.is_some() as usize
            + self.object.is_some() as usize
    }
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


