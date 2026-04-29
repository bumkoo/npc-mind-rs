// wuxia-core/src/shared/sentiment.rs
//
// 감정 판정 크로스커팅 타입 — Shared Kernel.
//
// SentimentDirection, SentimentJudgment, DeltaSource, judgment_to_delta는
// relationship 도메인과 LLM 어댑터(감정 판정 파이프라인) 양쪽에서 사용되는
// 크로스커팅 데이터 타입이다.
//
// 이 타입들은 순수 데이터 컨테이너 / 변환 함수로,
// 특정 도메인의 비즈니스 로직을 포함하지 않는다.
//
// [v4.6] shared kernel 이동

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// SentimentDirection — 감정 방향
// ---------------------------------------------------------------------------

/// NPC의 감정 방향.
///
/// 극단 앵커 체크와 LLM 판정 결과 모두에서 사용된다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SentimentDirection {
    /// 호의 (감사, 신뢰, 격려, 연민, 환영, 친밀감)
    Warmth,
    /// 적대 (적의, 분노, 경멸, 거부, 불신, 실망, 무시)
    Coldness,
    /// 감정 없음 (중립, 정보 전달, 일상 대화)
    None,
}

// ---------------------------------------------------------------------------
// SentimentJudgment — LLM 감정 판정 결과
// ---------------------------------------------------------------------------

/// LLM 감정 판정 결과.
///
/// LLM이 대화 컨텍스트를 보고 판정한 NPC 감정.
/// score는 그대로 affinity delta로 사용된다.
///
/// # Example
/// ```
/// use wuxia_core::shared::sentiment::{SentimentJudgment, SentimentDirection};
///
/// let judgment = SentimentJudgment::new(
///     SentimentDirection::Warmth,
///     2,
///     "NPC가 플레이어에게 감사를 표현함".to_string(),
/// );
/// assert_eq!(judgment.score(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SentimentJudgment {
    sentiment: SentimentDirection,
    /// -3 ~ +3 (클램프됨)
    score: i8,
    reason: String,
}

impl SentimentJudgment {
    /// 새 SentimentJudgment를 생성한다.
    ///
    /// score는 -3 ~ +3 범위로 클램프된다.
    pub fn new(sentiment: SentimentDirection, score: i8, reason: String) -> Self {
        Self {
            sentiment,
            score: score.clamp(-3, 3),
            reason,
        }
    }

    pub fn sentiment(&self) -> SentimentDirection {
        self.sentiment
    }

    pub fn score(&self) -> i8 {
        self.score
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

// ---------------------------------------------------------------------------
// DeltaSource — 호감도 변화의 출처
// ---------------------------------------------------------------------------

/// 호감도 변화의 출처.
///
/// 어떤 메커니즘으로 affinity delta가 산출되었는지 추적한다.
///
/// [v4.2] 감정 판정 시스템 도입으로 추가.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeltaSource {
    /// 12턴 정기 LLM 판정
    LlmPeriodicJudgment,
    /// 극단 임베딩 트리거 → LLM 판정
    LlmTriggeredJudgment,
    /// 기존 [affinity: N] 태그 (하위호환)
    LegacyTag,
}

impl Default for DeltaSource {
    fn default() -> Self {
        Self::LegacyTag
    }
}

// ---------------------------------------------------------------------------
// judgment_to_delta — 판정 결과 → affinity delta 변환
// ---------------------------------------------------------------------------

/// LLM 판정 결과의 score를 그대로 affinity delta로 반환한다.
///
/// 현재는 단순 전달이지만, 향후 관계 맥락에 따른 가중치 적용 등
/// 확장 포인트로 활용할 수 있다.
pub fn judgment_to_delta(judgment: &SentimentJudgment) -> i8 {
    judgment.score()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn judgment_positive_score() {
        let j = SentimentJudgment::new(SentimentDirection::Warmth, 3, "호의적".to_string());
        assert_eq!(judgment_to_delta(&j), 3);
    }

    #[test]
    fn judgment_negative_score() {
        let j = SentimentJudgment::new(SentimentDirection::Coldness, -3, "적대적".to_string());
        assert_eq!(judgment_to_delta(&j), -3);
    }

    #[test]
    fn judgment_zero_score() {
        let j = SentimentJudgment::new(SentimentDirection::None, 0, "중립".to_string());
        assert_eq!(judgment_to_delta(&j), 0);
    }

    #[test]
    fn judgment_score_clamped() {
        let j_high = SentimentJudgment::new(SentimentDirection::Warmth, 10, "과도".to_string());
        assert_eq!(j_high.score(), 3); // 10 → 3 클램프

        let j_low = SentimentJudgment::new(SentimentDirection::Coldness, -10, "과도".to_string());
        assert_eq!(j_low.score(), -3); // -10 → -3 클램프
    }

    #[test]
    fn judgment_warmth_positive_consistency() {
        // warmth → score 양수 정합성 (도메인 규칙은 아니지만 의미적으로 확인)
        let j = SentimentJudgment::new(SentimentDirection::Warmth, 2, "호의".to_string());
        assert_eq!(j.sentiment(), SentimentDirection::Warmth);
        assert!(j.score() > 0);
    }

    #[test]
    fn delta_source_default_is_legacy() {
        assert_eq!(DeltaSource::default(), DeltaSource::LegacyTag);
    }
}
