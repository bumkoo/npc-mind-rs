//! OCC → PAD 매핑 테이블 (Gebhard 2005, ALMA 모델)
//!
//! 22개 OCC 감정 유형의 PAD 좌표를 한 곳에 모아 관리한다.
//! 플레이테스트로 조정할 때 이 파일만 보면 된다.
//!
//! 출처: Gebhard, P. (2005). ALMA — A Layered Model of Affect.
//!       Proceedings of AAMAS'05. Table 1.
//!
//! FearsConfirmed는 Gebhard 테이블에 미포함 — 프로젝트 커스텀 값.

use super::pad::Pad;

// ---------------------------------------------------------------------------
// Event: Well-being (자신에게 일어난 사건의 바람직함)
// ---------------------------------------------------------------------------

pub const JOY_PAD: Pad             = Pad { pleasure:  0.40, arousal:  0.20, dominance:  0.10 };
pub const DISTRESS_PAD: Pad        = Pad { pleasure: -0.40, arousal: -0.20, dominance: -0.50 };

// ---------------------------------------------------------------------------
// Event: Fortune-of-others (타인에게 일어난 사건)
// ---------------------------------------------------------------------------

pub const HAPPY_FOR_PAD: Pad       = Pad { pleasure:  0.40, arousal:  0.20, dominance:  0.20 };
pub const PITY_PAD: Pad            = Pad { pleasure: -0.40, arousal: -0.20, dominance: -0.50 };
pub const GLOATING_PAD: Pad        = Pad { pleasure:  0.30, arousal: -0.30, dominance: -0.10 };
pub const RESENTMENT_PAD: Pad      = Pad { pleasure: -0.20, arousal: -0.30, dominance: -0.20 };

// ---------------------------------------------------------------------------
// Event: Prospect-based (기대/전망)
// ---------------------------------------------------------------------------

pub const HOPE_PAD: Pad            = Pad { pleasure:  0.20, arousal:  0.20, dominance: -0.10 };
pub const FEAR_PAD: Pad            = Pad { pleasure: -0.64, arousal:  0.60, dominance: -0.43 };
pub const SATISFACTION_PAD: Pad    = Pad { pleasure:  0.30, arousal: -0.20, dominance:  0.40 };
pub const DISAPPOINTMENT_PAD: Pad  = Pad { pleasure: -0.30, arousal:  0.10, dominance: -0.40 };
pub const RELIEF_PAD: Pad          = Pad { pleasure:  0.20, arousal: -0.30, dominance:  0.40 };
/// Gebhard 테이블에 미포함 — 프로젝트 커스텀 값
pub const FEARS_CONFIRMED_PAD: Pad = Pad { pleasure: -0.50, arousal:  0.30, dominance: -0.60 };

// ---------------------------------------------------------------------------
// Action: Attribution (행위의 정당성)
// ---------------------------------------------------------------------------

pub const PRIDE_PAD: Pad           = Pad { pleasure:  0.40, arousal:  0.30, dominance:  0.30 };
/// D:-0.90 — Gebhard 원본(-0.60)에서 커스텀 조정.
/// Shame은 OCC 감정 중 가장 복종적. Remorse(D:-0.60)와 차별화.
pub const SHAME_PAD: Pad           = Pad { pleasure: -0.30, arousal:  0.10, dominance: -0.90 };
pub const ADMIRATION_PAD: Pad      = Pad { pleasure:  0.50, arousal:  0.30, dominance: -0.20 };
pub const REPROACH_PAD: Pad        = Pad { pleasure: -0.30, arousal: -0.10, dominance:  0.40 };

// ---------------------------------------------------------------------------
// Action: Compound (복합 감정)
// ---------------------------------------------------------------------------

pub const GRATIFICATION_PAD: Pad   = Pad { pleasure:  0.60, arousal:  0.50, dominance:  0.40 };
pub const REMORSE_PAD: Pad         = Pad { pleasure: -0.30, arousal:  0.10, dominance: -0.60 };
pub const GRATITUDE_PAD: Pad       = Pad { pleasure:  0.40, arousal:  0.20, dominance: -0.30 };
pub const ANGER_PAD: Pad           = Pad { pleasure: -0.51, arousal:  0.59, dominance:  0.25 };

// ---------------------------------------------------------------------------
// Object (대상의 매력도)
// ---------------------------------------------------------------------------

pub const LOVE_PAD: Pad            = Pad { pleasure:  0.30, arousal:  0.10, dominance:  0.20 };
pub const HATE_PAD: Pad            = Pad { pleasure: -0.60, arousal:  0.60, dominance:  0.30 };
