//! OCC → PAD 매핑 테이블 (Gebhard 2005, ALMA 모델)
//!
//! 22개 OCC 감정 유형의 PAD 좌표를 한 곳에 모아 관리한다.
//! 플레이테스트로 조정할 때 이 파일만 보면 된다.
//!
//! # 출처
//!
//! 기본값: Gebhard, P. (2005). ALMA — A Layered Model of Affect.
//!         Proceedings of AAMAS'05. Table 1.
//! 실험값: Hoffmann et al. (2012). Mapping discrete emotions into the
//!         dimensional space: An empirical approach. IEEE SMC 2012.
//!
//! # Gebhard 2005 원본과의 차이 (2026-03-30 검토)
//!
//! Gebhard 논문 인용 테이블과 ALMA 구현체(inBloom)에 일부 차이가 존재한다.
//! 아래는 논문 테이블 기준으로 정렬한 뒤, 엔진 설계에 맞게 조정한 항목이다.
//!
//! ## Gebhard 원본 부호 오류 수정 (7개 감정, 10개 값)
//!
//! 기존 코드에 Gebhard 원본 대비 A축 부호가 반전되어 있던 값을 수정했다.
//!
//! | 감정           | 축 | 수정 전 | Gebhard 원본 |
//! |----------------|-----|---------|-------------|
//! | Distress       | A   | +0.20   | -0.20       |
//! | Gloating       | A   | +0.30   | -0.30       |
//! | Gloating       | D   | +0.30   | -0.10       |
//! | Resentment     | A   | +0.30   | -0.30       |
//! | Disappointment | A   | -0.40   | +0.10       |
//! | Reproach       | A   | +0.20   | -0.10       |
//! | Relief         | D   | 0.20    | 0.40        |
//! | Gratification  | P   | 0.50    | 0.60        |
//! | Gratification  | A   | 0.40    | 0.50        |
//!
//! ## Gebhard 원본과 의도적으로 다르게 유지한 값
//!
//! Gebhard 이론값(inBloom 구현체)과 Hoffmann 2012 실험값이 충돌하는 경우,
//! pad_dot 공명 설계에 적합한 쪽을 선택했다.
//!
//! | 감정           | 축 | 현재 값 | Gebhard(inBloom) | Hoffmann 2012 | 선택 근거                          |
//! |----------------|-----|---------|-----------------|---------------|------------------------------------|
//! | Fear           | A   | +0.60   | -0.60           | +0.47         | 고각성이어야 위협 자극과 공명       |
//! | FearsConfirmed | A   | +0.30   | -0.30           | +0.42         | Fear와 같은 맥락                   |
//! | FearsConfirmed | D   | -0.60   | -0.70           | -0.52         | 중간값 채택                        |
//! | Admiration     | D   | -0.20   | 0.00            | +0.05         | 감탄 시 약간의 복종성이 자연스러움  |
//!
//! ## 커스텀 조정
//!
//! | 감정           | 축 | Gebhard | 현재 값 | 근거                                             |
//! |----------------|-----|---------|---------|--------------------------------------------------|
//! | Shame          | D   | -0.60   | -0.90   | OCC 최고 복종 감정. Remorse(D:-0.60)와 차별화.    |
//! | FearsConfirmed | -   | 미포함  | 커스텀  | Gebhard 테이블에 없어 프로젝트 자체 설정.          |

use super::pad::Pad;

// ---------------------------------------------------------------------------
// Event: Well-being (자신에게 일어난 사건의 바람직함)
// ---------------------------------------------------------------------------

pub const JOY_PAD: Pad = Pad {
    pleasure: 0.40,
    arousal: 0.20,
    dominance: 0.10,
};
pub const DISTRESS_PAD: Pad = Pad {
    pleasure: -0.40,
    arousal: -0.20,
    dominance: -0.50,
};

// ---------------------------------------------------------------------------
// Event: Fortune-of-others (타인에게 일어난 사건)
// ---------------------------------------------------------------------------

pub const HAPPY_FOR_PAD: Pad = Pad {
    pleasure: 0.40,
    arousal: 0.20,
    dominance: 0.20,
};
pub const PITY_PAD: Pad = Pad {
    pleasure: -0.40,
    arousal: -0.20,
    dominance: -0.50,
};
pub const GLOATING_PAD: Pad = Pad {
    pleasure: 0.30,
    arousal: -0.30,
    dominance: -0.10,
};
pub const RESENTMENT_PAD: Pad = Pad {
    pleasure: -0.20,
    arousal: -0.30,
    dominance: -0.20,
};

// ---------------------------------------------------------------------------
// Event: Prospect-based (기대/전망)
// ---------------------------------------------------------------------------

pub const HOPE_PAD: Pad = Pad {
    pleasure: 0.20,
    arousal: 0.20,
    dominance: -0.10,
};
pub const FEAR_PAD: Pad = Pad {
    pleasure: -0.64,
    arousal: 0.60,
    dominance: -0.43,
};
pub const SATISFACTION_PAD: Pad = Pad {
    pleasure: 0.30,
    arousal: -0.20,
    dominance: 0.40,
};
pub const DISAPPOINTMENT_PAD: Pad = Pad {
    pleasure: -0.30,
    arousal: 0.10,
    dominance: -0.40,
};
pub const RELIEF_PAD: Pad = Pad {
    pleasure: 0.20,
    arousal: -0.30,
    dominance: 0.40,
};
/// Gebhard 테이블에 미포함 — 프로젝트 커스텀 값
pub const FEARS_CONFIRMED_PAD: Pad = Pad {
    pleasure: -0.50,
    arousal: 0.30,
    dominance: -0.60,
};

// ---------------------------------------------------------------------------
// Action: Attribution (행위의 정당성)
// ---------------------------------------------------------------------------

pub const PRIDE_PAD: Pad = Pad {
    pleasure: 0.40,
    arousal: 0.30,
    dominance: 0.30,
};
/// D:-0.90 — Gebhard 원본(-0.60)에서 커스텀 조정.
/// Shame은 OCC 감정 중 가장 복종적. Remorse(D:-0.60)와 차별화.
pub const SHAME_PAD: Pad = Pad {
    pleasure: -0.30,
    arousal: 0.10,
    dominance: -0.90,
};
pub const ADMIRATION_PAD: Pad = Pad {
    pleasure: 0.50,
    arousal: 0.30,
    dominance: -0.20,
};
pub const REPROACH_PAD: Pad = Pad {
    pleasure: -0.30,
    arousal: -0.10,
    dominance: 0.40,
};

// ---------------------------------------------------------------------------
// Action: Compound (복합 감정)
// ---------------------------------------------------------------------------

pub const GRATIFICATION_PAD: Pad = Pad {
    pleasure: 0.60,
    arousal: 0.50,
    dominance: 0.40,
};
pub const REMORSE_PAD: Pad = Pad {
    pleasure: -0.30,
    arousal: 0.10,
    dominance: -0.60,
};
pub const GRATITUDE_PAD: Pad = Pad {
    pleasure: 0.40,
    arousal: 0.20,
    dominance: -0.30,
};
pub const ANGER_PAD: Pad = Pad {
    pleasure: -0.51,
    arousal: 0.59,
    dominance: 0.25,
};

// ---------------------------------------------------------------------------
// Object (대상의 매력도)
// ---------------------------------------------------------------------------

pub const LOVE_PAD: Pad = Pad {
    pleasure: 0.30,
    arousal: 0.10,
    dominance: 0.20,
};
pub const HATE_PAD: Pad = Pad {
    pleasure: -0.60,
    arousal: 0.60,
    dominance: 0.30,
};
