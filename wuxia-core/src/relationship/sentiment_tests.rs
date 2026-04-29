use super::*;
use crate::shared::embedding::l2_normalize;
use crate::shared::sentiment::{DeltaSource, SentimentJudgment, judgment_to_delta};
use crate::relationship::{ConversationEffect, apply_conversation_effect};

// -- 테스트 헬퍼: 3차원 하드코딩 벡터 --

fn warmth_anchor() -> Vec<f32> {
    l2_normalize(&[0.9, 0.1, 0.0])
}

fn coldness_anchor() -> Vec<f32> {
    l2_normalize(&[0.0, 0.1, 0.9])
}

fn extreme_warmth_utterance() -> Vec<f32> {
    l2_normalize(&[0.95, 0.05, 0.0])
}

fn extreme_coldness_utterance() -> Vec<f32> {
    l2_normalize(&[0.0, 0.05, 0.95])
}

fn neutral_utterance() -> Vec<f32> {
    l2_normalize(&[0.1, 0.9, 0.1])
}

fn make_anchors(threshold: f32) -> ExtremeAnchorSet {
    ExtremeAnchorSet::new(
        vec![warmth_anchor()],
        vec![coldness_anchor()],
        threshold,
        3,
    )
}

// =======================================================================
// ExtremeAnchorSet
// =======================================================================

#[test]
fn extreme_warmth_triggers() {
    let anchors = make_anchors(0.8);
    let result = anchors.check_extreme(&extreme_warmth_utterance());
    assert!(result.triggered());
    assert_eq!(result.direction(), SentimentDirection::Warmth);
    assert!(result.max_similarity() > 0.8);
}

#[test]
fn extreme_coldness_triggers() {
    let anchors = make_anchors(0.8);
    let result = anchors.check_extreme(&extreme_coldness_utterance());
    assert!(result.triggered());
    assert_eq!(result.direction(), SentimentDirection::Coldness);
    assert!(result.max_similarity() > 0.8);
}

#[test]
fn neutral_no_trigger() {
    let anchors = make_anchors(0.8);
    let result = anchors.check_extreme(&neutral_utterance());
    assert!(!result.triggered());
    assert_eq!(result.direction(), SentimentDirection::None);
}

#[test]
fn threshold_boundary() {
    // warmth_anchor와 extreme_warmth_utterance의 cosine similarity 계산
    let sim = crate::shared::embedding::cosine_similarity(&warmth_anchor(), &extreme_warmth_utterance());

    // threshold를 sim보다 약간 높게 → 미달
    let anchors_high = make_anchors(sim + 0.01);
    let result = anchors_high.check_extreme(&extreme_warmth_utterance());
    assert!(!result.triggered());

    // threshold를 sim보다 약간 낮게 → 트리거
    let anchors_low = make_anchors(sim - 0.01);
    let result = anchors_low.check_extreme(&extreme_warmth_utterance());
    assert!(result.triggered());
}

#[test]
fn both_high_picks_higher() {
    // warmth와 coldness 앵커 모두에 가까운 벡터 (낮은 threshold)
    let anchors = ExtremeAnchorSet::new(
        vec![l2_normalize(&[1.0, 0.5, 0.0])],
        vec![l2_normalize(&[0.0, 0.5, 1.0])],
        0.1, // 매우 낮은 threshold → 둘 다 트리거
        3,
    );

    // warmth 쪽에 가까운 벡터
    let utterance = l2_normalize(&[0.8, 0.4, 0.1]);
    let result = anchors.check_extreme(&utterance);
    assert!(result.triggered());
    assert_eq!(result.direction(), SentimentDirection::Warmth);
}

#[test]
fn warmth_only_anchors() {
    // coldness 앵커 없이 warmth만
    let anchors = ExtremeAnchorSet::new(
        vec![warmth_anchor()],
        vec![], // coldness 비어있음
        0.8,
        3,
    );
    let result = anchors.check_extreme(&extreme_warmth_utterance());
    assert!(result.triggered());
    assert_eq!(result.direction(), SentimentDirection::Warmth);

    // coldness 벡터 → 트리거 안 됨
    let result2 = anchors.check_extreme(&extreme_coldness_utterance());
    assert!(!result2.triggered());
}

// =======================================================================
// TurnCounter
// =======================================================================

#[test]
fn turn_counter_below_period() {
    let mut counter = TurnCounter::new(12);
    for _ in 0..11 {
        assert!(!counter.tick());
    }
    assert_eq!(counter.count(), 11);
}

#[test]
fn turn_counter_at_period() {
    let mut counter = TurnCounter::new(12);
    for _ in 0..11 {
        counter.tick();
    }
    assert!(counter.tick()); // 12번째 → true
    assert_eq!(counter.count(), 0); // 리셋됨
}

#[test]
fn turn_counter_reset_then_full_cycle() {
    let mut counter = TurnCounter::new(3);
    counter.tick(); // 1
    counter.reset(); // → 0
    assert_eq!(counter.count(), 0);

    // 다시 3번 필요
    assert!(!counter.tick()); // 1
    assert!(!counter.tick()); // 2
    assert!(counter.tick());  // 3 → true
}

#[test]
fn turn_counter_consecutive_periods() {
    let mut counter = TurnCounter::new(3);
    assert!(! counter.tick()); // 1
    assert!(!counter.tick()); // 2
    assert!(counter.tick());  // 3 → true

    assert!(!counter.tick()); // 1
    assert!(!counter.tick()); // 2
    assert!(counter.tick());  // 3 → true again
}

// =======================================================================
// ConversationEffect 확장
// =======================================================================

#[test]
fn effect_legacy_default() {
    let effect = ConversationEffect::new(3);
    assert_eq!(effect.source(), DeltaSource::LegacyTag);
}

#[test]
fn effect_with_source_periodic() {
    let effect = ConversationEffect::with_source(2, DeltaSource::LlmPeriodicJudgment);
    assert_eq!(effect.source(), DeltaSource::LlmPeriodicJudgment);
    assert_eq!(effect.affinity_delta(), 2);
}

#[test]
fn effect_with_source_triggered() {
    let effect = ConversationEffect::with_source(-1, DeltaSource::LlmTriggeredJudgment);
    assert_eq!(effect.source(), DeltaSource::LlmTriggeredJudgment);
    assert_eq!(effect.affinity_delta(), -1);
}

// =======================================================================
// 통합 시나리오
// =======================================================================

#[test]
fn integration_extreme_trigger_to_effect() {
    // 극단 트리거 → judgment → delta → ConversationEffect 전체 흐름
    let anchors = make_anchors(0.8);
    let result = anchors.check_extreme(&extreme_warmth_utterance());
    assert!(result.triggered());

    // LLM 판정 시뮬레이션
    let judgment = SentimentJudgment::new(
        SentimentDirection::Warmth,
        3,
        "극단적 호의".to_string(),
    );
    let delta = judgment_to_delta(&judgment);
    let effect = ConversationEffect::with_source(delta, DeltaSource::LlmTriggeredJudgment);

    // Relationship에 적용
    let mut rel = crate::test_fixtures::make_relationship(1, 1, 2);
    let events = apply_conversation_effect(&mut rel, &effect);
    assert_eq!(rel.affinity(), 3.0);
    assert_eq!(events.len(), 1); // AffinityChanged
}

#[test]
fn integration_periodic_trigger() {
    // 12턴 경과 → 정기 판정 흐름
    let mut counter = TurnCounter::new(12);
    let anchors = make_anchors(0.8);

    for _ in 0..11 {
        let result = anchors.check_extreme(&neutral_utterance());
        assert!(!result.triggered());
        assert!(!counter.tick());
    }

    // 12번째 턴
    let result = anchors.check_extreme(&neutral_utterance());
    assert!(!result.triggered()); // 극단 아님
    assert!(counter.tick());      // 정기 판정 트리거

    // LLM 판정 시뮬레이션
    let judgment = SentimentJudgment::new(
        SentimentDirection::Warmth,
        1,
        "약간 호의적".to_string(),
    );
    let delta = judgment_to_delta(&judgment);
    let effect = ConversationEffect::with_source(delta, DeltaSource::LlmPeriodicJudgment);

    let mut rel = crate::test_fixtures::make_relationship(1, 1, 2);
    apply_conversation_effect(&mut rel, &effect);
    assert_eq!(rel.affinity(), 1.0);
}

#[test]
fn integration_extreme_trigger_resets_counter() {
    let mut counter = TurnCounter::new(12);

    // 5턴 진행
    for _ in 0..5 {
        counter.tick();
    }
    assert_eq!(counter.count(), 5);

    // 극단 트리거 발생 → 카운터 리셋
    counter.reset();
    assert_eq!(counter.count(), 0);

    // 다시 12턴 필요
    for _ in 0..11 {
        assert!(!counter.tick());
    }
    assert!(counter.tick()); // 12번째
}
