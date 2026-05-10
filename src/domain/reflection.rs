//! Scene Boundary Reflection — *결정론 부분*.
//!
//! Phase 1 Mind Architecture (relationships.md v0.7 §6) — Outer Loop 진입 게이트.
//! `dialogue_orchestrator.end_session`가 turn 누적 → ReflectionService 호출 →
//! `Command::EndDialogue { reflection }` payload로 박혀 RelationshipPolicy 게이트 평가.
//!
//! 본 모듈은 *LLM 무관*한 결정론 함수만 담는다:
//! - `TurnSnapshot` — 매 turn의 OCC + PAD + Beat 신호 누적용
//! - `compute_significance(turns)` — 4 신호 가중 합산 (relationships.md §6.3)
//! - `ReflectionResult` — LLM 출력 + engine 점수 합쳐진 결과 DTO
//!
//! LLM 호출(`ReflectionPort`)은 application layer (`reflection_service.rs`).

use serde::{Deserialize, Serialize};

use super::emotion::EmotionType;
use super::pad::Pad;

// ===========================================================================
// TurnSnapshot — 매 turn의 결정론 신호
// ===========================================================================

/// 한 dialogue turn의 결정론 신호 누적.
///
/// `DialogueOrchestrator`가 매 `turn()`에서 채워 `end_session()` 시점에
/// `ReflectionService::reflect`에 슬라이스로 전달한다.
///
/// 필드는 *결정론적*이고 *LLM 무관*. compute_significance가 100% 재현 가능.
#[derive(Debug, Clone)]
pub struct TurnSnapshot {
    /// user(또는 partner) 발화 원문
    pub user_utterance: String,
    /// NPC LLM 응답 원문
    pub npc_response: String,
    /// 본 turn에서 활성화된 OCC 감정 + 강도 [0.0, 1.0]
    pub occ_emotions: Vec<(EmotionType, f32)>,
    /// stimulus 적용 *전* PAD
    pub pad_before: Pad,
    /// stimulus 적용 *후* PAD
    pub pad_after: Pad,
    /// 본 turn에서 Beat 전환 발생 여부
    pub beat_changed: bool,
    /// 1-based turn 인덱스 (transcript 정렬용)
    pub turn_index: u32,
}

// ===========================================================================
// compute_significance — engine 정량 격동도
// ===========================================================================

/// 대화의 객관적 격동도 점수 — `[0.0, 1.0]`.
///
/// 4 신호의 가중 합산 (relationships.md v0.7 §6.3, 가중치는 *디자인 파라미터* —
/// Stage 4 narrative validation 후 `tuning.rs`로 이관 가능):
///
/// | # | 신호 | 가중치 |
/// |---|---|---|
/// | 1 | peak OCC intensity | 0.40 |
/// | 2 | PAD trajectory magnitude (turn 사이 delta 누적, 2.0 cap → /2.0 normalize) | 0.30 |
/// | 3 | OCC type diversity (distinct count / 5, 1.0 cap) | 0.15 |
/// | 4 | Beat 전환 발생 binary | 0.15 |
///
/// 빈 입력 시 0.0 (잡담 미만). RelationshipPolicy 게이트는
/// `>= 0.3 OR !is_chitchat OR ...`에서 본 점수를 사용한다.
pub fn compute_significance(turns: &[TurnSnapshot]) -> f32 {
    if turns.is_empty() {
        return 0.0;
    }

    // (1) Peak OCC intensity
    let peak_occ = turns
        .iter()
        .flat_map(|t| t.occ_emotions.iter().map(|(_, intensity)| *intensity))
        .fold(0.0_f32, f32::max);

    // (2) PAD trajectory magnitude — turn 사이 delta 유클리드 길이 누적, 2.0 cap → 0~1
    let pad_magnitude = turns
        .windows(2)
        .map(|w| pad_delta_magnitude(w[0].pad_after, w[1].pad_after))
        .sum::<f32>()
        .min(2.0)
        / 2.0;

    // (3) OCC type diversity — distinct kind 개수 / 5, 1.0 cap
    let diversity = {
        use std::collections::HashSet;
        let distinct: HashSet<EmotionType> = turns
            .iter()
            .flat_map(|t| t.occ_emotions.iter().map(|(kind, _)| *kind))
            .collect();
        (distinct.len() as f32 / 5.0).min(1.0)
    };

    // (4) Beat signal — binary
    let beat_signal = if turns.iter().any(|t| t.beat_changed) {
        1.0
    } else {
        0.0
    };

    (peak_occ * 0.40 + pad_magnitude * 0.30 + diversity * 0.15 + beat_signal * 0.15)
        .clamp(0.0, 1.0)
}

/// 두 PAD 사이 유클리드 거리. `Pad::magnitude()`가 없어 inline 정의.
fn pad_delta_magnitude(a: Pad, b: Pad) -> f32 {
    let dp = a.pleasure - b.pleasure;
    let da = a.arousal - b.arousal;
    let dd = a.dominance - b.dominance;
    (dp * dp + da * da + dd * dd).sqrt()
}

// ===========================================================================
// ReflectionResult — LLM + engine 합산 결과
// ===========================================================================

/// LLM의 서사 평가 + 엔진의 정량 점수 합산 결과.
/// `EventPayload::DialogueReflected.result` + `EventPayload::DialogueEndRequested.reflection`에 박힘.
///
/// 본 타입은 *chat feature 무관* 순수 도메인 DTO — chat 비활성 빌드도 컴파일 가능.
/// chat 비활성 시 caller는 `reflection: None`을 dispatch (호환).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReflectionResult {
    /// LLM 판정 — 이 대화가 *서사적으로 잉여*인가
    pub is_chitchat: bool,
    /// LLM 작성 — 1~2문장 한국어 요약
    pub summary: String,
    /// 엔진 계산 — 객관적 격동도 (`compute_significance` 결과)
    pub significance_score: f32,
    /// LLM emit — 선언/의례 사건 (Phase 1엔 항상 빈 vec, Phase 2 Channel 1 활성화)
    #[serde(default)]
    pub declarative_events: Vec<DeclarativeEventPlaceholder>,
    /// LLM emit — Partnership 사건 (Phase 1엔 항상 None, Phase 2 활성화)
    #[serde(default)]
    pub partnership_event: Option<PartnershipEventPlaceholder>,
    /// 디버깅용 — 누적된 turn 개수
    pub turn_count: usize,
    /// 디버깅용 — LLM의 reasoning 텍스트 (calibration drift 감지)
    #[serde(default)]
    pub llm_reasoning: Option<String>,
}

/// Phase 1 placeholder. Phase 2에서 `relationships.md` v0.7 §6.4 Channel 1 schema로 확장.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeclarativeEventPlaceholder {
    pub kind: String,
    pub target: Option<String>,
    pub text: String,
}

/// Phase 1 placeholder. Phase 2에서 Spouse/Engaged/Lover/Separated enum으로 확장.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartnershipEventPlaceholder {
    pub kind: String,
    pub reason: String,
}

// ===========================================================================
// 단위 테스트 — compute_significance 4 신호 + clamp + 빈 입력
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot_with(
        occ: Vec<(EmotionType, f32)>,
        pad_before: Pad,
        pad_after: Pad,
        beat_changed: bool,
    ) -> TurnSnapshot {
        TurnSnapshot {
            user_utterance: String::new(),
            npc_response: String::new(),
            occ_emotions: occ,
            pad_before,
            pad_after,
            beat_changed,
            turn_index: 0,
        }
    }

    #[test]
    fn compute_significance_empty_returns_zero() {
        assert_eq!(compute_significance(&[]), 0.0);
    }

    #[test]
    fn compute_significance_high_peak_dominates() {
        // OCC peak 0.95 만 활성, 다른 신호 0
        let turns = vec![snapshot_with(
            vec![(EmotionType::Anger, 0.95)],
            Pad::neutral(),
            Pad::neutral(),
            false,
        )];
        let s = compute_significance(&turns);
        // peak 0.95 * 0.40 = 0.38 + diversity 1/5*0.15 = 0.03 = 0.41
        assert!(s > 0.40 && s < 0.42, "expected ~0.41, got {s}");
    }

    #[test]
    fn compute_significance_pad_trajectory_accumulates() {
        // 3 turn 시퀀스 → windows(2) 2개 window → 두 delta 합산
        // turn[0].pad_after=(0,0,0) → turn[1].pad_after=(0.5,0.5,0): delta = sqrt(0.5) ≈ 0.707
        // turn[1].pad_after=(0.5,0.5,0) → turn[2].pad_after=(1.0,1.0,0.5): delta = sqrt(0.75) ≈ 0.866
        // total ≈ 1.573, 2.0 cap → 0.787, * 0.30 ≈ 0.236
        let turns = vec![
            snapshot_with(vec![], Pad::neutral(), Pad::neutral(), false),
            snapshot_with(vec![], Pad::neutral(), Pad::new(0.5, 0.5, 0.0), false),
            snapshot_with(
                vec![],
                Pad::new(0.5, 0.5, 0.0),
                Pad::new(1.0, 1.0, 0.5),
                false,
            ),
        ];
        let s = compute_significance(&turns);
        assert!(s > 0.20 && s < 0.27, "expected ~0.24, got {s}");
    }

    #[test]
    fn compute_significance_occ_diversity_capped_at_five_distinct() {
        // 5 distinct kind 모두 강도 0 → diversity만 활성
        let turns = vec![snapshot_with(
            vec![
                (EmotionType::Joy, 0.0),
                (EmotionType::Distress, 0.0),
                (EmotionType::Pride, 0.0),
                (EmotionType::Shame, 0.0),
                (EmotionType::Anger, 0.0),
            ],
            Pad::neutral(),
            Pad::neutral(),
            false,
        )];
        let s = compute_significance(&turns);
        // diversity 5/5 = 1.0 * 0.15 = 0.15. peak 0.0, pad 0.0, beat 0.0
        assert!((s - 0.15).abs() < 0.001, "expected 0.15, got {s}");
    }

    #[test]
    fn compute_significance_beat_signal_binary() {
        let with_beat = vec![snapshot_with(vec![], Pad::neutral(), Pad::neutral(), true)];
        let without = vec![snapshot_with(vec![], Pad::neutral(), Pad::neutral(), false)];

        let s_with = compute_significance(&with_beat);
        let s_without = compute_significance(&without);

        // 차이는 정확히 0.15 (beat_signal * 0.15)
        assert!(
            (s_with - s_without - 0.15).abs() < 0.001,
            "beat delta should be 0.15, got {}",
            s_with - s_without
        );
    }

    #[test]
    fn compute_significance_clamps_to_one() {
        // 모든 신호 최대로 — peak 1.0, pad 큰 변화 + beat + 5 distinct
        let pad_max = Pad::new(1.0, 1.0, 1.0);
        let pad_min = Pad::new(-1.0, -1.0, -1.0);
        let turns = vec![
            snapshot_with(
                vec![
                    (EmotionType::Joy, 1.0),
                    (EmotionType::Distress, 1.0),
                    (EmotionType::Pride, 1.0),
                    (EmotionType::Shame, 1.0),
                    (EmotionType::Anger, 1.0),
                ],
                pad_min,
                pad_max,
                true,
            ),
            snapshot_with(vec![], pad_max, pad_min, false),
        ];
        let s = compute_significance(&turns);
        assert!(s <= 1.0, "must clamp to 1.0, got {s}");
        assert!(s > 0.85, "all-max should be near 1.0, got {s}");
    }

    #[test]
    fn reflection_result_serde_roundtrip_preserves_all_fields() {
        let r = ReflectionResult {
            is_chitchat: false,
            summary: "결단 사건".into(),
            significance_score: 0.87,
            declarative_events: vec![DeclarativeEventPlaceholder {
                kind: "execute".into(),
                target: Some("lu_qian".into()),
                text: "임충이 육겸을 처단".into(),
            }],
            partnership_event: None,
            turn_count: 12,
            llm_reasoning: Some("OCC peak 0.95, PAD trajectory 1.4".into()),
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: ReflectionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn reflection_result_deserialize_minimal_fields_uses_defaults() {
        // Phase 1 fallback 또는 Phase 2 신규 필드 호환성 — 최소 필드만 명시
        let json = r#"{
            "is_chitchat": true,
            "summary": "지나가는 인사",
            "significance_score": 0.05,
            "turn_count": 2
        }"#;
        let r: ReflectionResult = serde_json::from_str(json).expect("minimal fields OK");
        assert!(r.is_chitchat);
        assert!(r.declarative_events.is_empty());
        assert!(r.partnership_event.is_none());
        assert!(r.llm_reasoning.is_none());
    }
}
