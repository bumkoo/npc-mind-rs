// wuxia-core/src/relationship/sentiment.rs
//
// 감정 판정 도메인 로직 — 극단 앵커 체크 + 판정 결과 타입.
//
// "한마디 말에 살의가 담겨 있는지, 은혜가 담겨 있는지를
//  가려내는 것은 강호인의 본능이다."
//
// 아키텍처:
//   위치: wuxia-core (순수 도메인 — 인프라 의존 없음)
//   의존: cosine_similarity (shared kernel의 벡터 유틸리티)
//
// 2단계 하이브리드:
//   ① 극단 앵커 임베딩 (~7ms): 매 턴, "누가 봐도 명확한" 극단 감정 즉시 감지
//   ② LLM 정기 판정 (~300ms): 12턴마다, 전체 감정 상태 정밀 판정
//
// [v4.2] | 2026-02-28

use serde::{Deserialize, Serialize};

use crate::shared::embedding::cosine_similarity;
use crate::shared::sentiment::SentimentDirection;

// ---------------------------------------------------------------------------
// ExtremeCheckResult — 극단 앵커 체크 결과
// ---------------------------------------------------------------------------

/// 극단 앵커 체크의 결과.
///
/// `ExtremeAnchorSet::check_extreme()`가 반환하는 값.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtremeCheckResult {
    /// threshold 이상인가
    triggered: bool,
    /// Warmth / Coldness / None
    direction: SentimentDirection,
    /// 최고 유사도 값 (warmth_max와 coldness_max 중 큰 쪽)
    max_similarity: f32,
    /// 호의 앵커 최대 유사도
    warmth_max: f32,
    /// 적대 앵커 최대 유사도
    coldness_max: f32,
    /// 판정에 사용된 threshold
    threshold: f32,
}

impl ExtremeCheckResult {
    pub fn triggered(&self) -> bool {
        self.triggered
    }

    pub fn direction(&self) -> SentimentDirection {
        self.direction
    }

    pub fn max_similarity(&self) -> f32 {
        self.max_similarity
    }

    pub fn warmth_max(&self) -> f32 {
        self.warmth_max
    }

    pub fn coldness_max(&self) -> f32 {
        self.coldness_max
    }

    pub fn threshold(&self) -> f32 {
        self.threshold
    }
}

// ---------------------------------------------------------------------------
// ExtremeAnchorSet — 극단 앵커 벡터 집합
// ---------------------------------------------------------------------------

/// 극단 앵커 벡터를 보유하고, 대사 벡터와의 유사도로 극단 감정 트리거 여부를 판정한다.
///
/// 벤치마크 결과:
///   BGE-M3 + 극단 앵커(생사 수준 20+20개):
///   진양성 100%, 거짓양성 0% @ threshold=0.60
///
/// # 판정 방식: 개별앵커 max (방식B)
/// warmth 앵커 20개와의 cosine similarity 중 최대값,
/// coldness 앵커 20개와의 cosine similarity 중 최대값을 구하고,
/// threshold 이상인 방향을 채택한다.
///
/// # Example
/// ```
/// use wuxia_core::relationship::sentiment::ExtremeAnchorSet;
/// use wuxia_core::shared::sentiment::SentimentDirection;
/// use wuxia_core::shared::embedding::l2_normalize;
///
/// let warmth = vec![l2_normalize(&[0.9, 0.1, 0.0])];
/// let coldness = vec![l2_normalize(&[0.0, 0.1, 0.9])];
/// let anchors = ExtremeAnchorSet::new(warmth, coldness, 0.8, 3);
///
/// let utterance = l2_normalize(&[0.95, 0.05, 0.0]);
/// let result = anchors.check_extreme(&utterance);
/// assert!(result.triggered());
/// assert_eq!(result.direction(), SentimentDirection::Warmth);
/// ```
pub struct ExtremeAnchorSet {
    /// 극단 호의 앵커 벡터 (20개)
    warmth_vectors: Vec<Vec<f32>>,
    /// 극단 적대 앵커 벡터 (20개)
    coldness_vectors: Vec<Vec<f32>>,
    /// 트리거 기준값 (0.60)
    threshold: f32,
    /// 벡터 차원 수 (BGE-M3: 1024)
    dimension: usize,
}

impl ExtremeAnchorSet {
    /// 새 ExtremeAnchorSet을 생성한다.
    ///
    /// # Panics
    /// - warmth/coldness 벡터가 모두 비어 있으면 panic.
    /// - 벡터 차원이 `dimension`과 다르면 panic.
    pub fn new(
        warmth_vectors: Vec<Vec<f32>>,
        coldness_vectors: Vec<Vec<f32>>,
        threshold: f32,
        dimension: usize,
    ) -> Self {
        assert!(
            !warmth_vectors.is_empty() || !coldness_vectors.is_empty(),
            "최소 하나의 앵커 벡터가 필요합니다"
        );
        for (i, v) in warmth_vectors.iter().enumerate() {
            assert_eq!(
                v.len(),
                dimension,
                "warmth 앵커[{}] 차원 불일치: {} vs {}",
                i,
                v.len(),
                dimension
            );
        }
        for (i, v) in coldness_vectors.iter().enumerate() {
            assert_eq!(
                v.len(),
                dimension,
                "coldness 앵커[{}] 차원 불일치: {} vs {}",
                i,
                v.len(),
                dimension
            );
        }
        Self {
            warmth_vectors,
            coldness_vectors,
            threshold,
            dimension,
        }
    }

    /// 대사 벡터가 극단 앵커에 threshold 이상 유사한지 체크한다.
    ///
    /// 방식B (개별앵커 max): 벤치마크에서 검증된 방식.
    /// warmth/coldness 앵커 각각에서 max similarity를 구하고,
    /// threshold 이상인 방향을 채택한다.
    /// 둘 다 threshold 이상이면 높은 쪽을 채택한다.
    pub fn check_extreme(&self, utterance_vector: &[f32]) -> ExtremeCheckResult {
        assert_eq!(
            utterance_vector.len(),
            self.dimension,
            "대사 벡터 차원 불일치: {} vs {}",
            utterance_vector.len(),
            self.dimension
        );

        let warmth_max = self
            .warmth_vectors
            .iter()
            .map(|anchor| cosine_similarity(utterance_vector, anchor))
            .fold(f32::NEG_INFINITY, f32::max);

        let coldness_max = self
            .coldness_vectors
            .iter()
            .map(|anchor| cosine_similarity(utterance_vector, anchor))
            .fold(f32::NEG_INFINITY, f32::max);

        // 빈 벡터 집합이면 NEG_INFINITY → threshold 미달
        let warmth_max = if self.warmth_vectors.is_empty() {
            0.0
        } else {
            warmth_max
        };
        let coldness_max = if self.coldness_vectors.is_empty() {
            0.0
        } else {
            coldness_max
        };

        let warmth_triggered = warmth_max >= self.threshold;
        let coldness_triggered = coldness_max >= self.threshold;

        match (warmth_triggered, coldness_triggered) {
            (true, true) => {
                // 둘 다 threshold 이상 → 높은 쪽 채택
                if warmth_max >= coldness_max {
                    ExtremeCheckResult {
                        triggered: true,
                        direction: SentimentDirection::Warmth,
                        max_similarity: warmth_max,
                        warmth_max,
                        coldness_max,
                        threshold: self.threshold,
                    }
                } else {
                    ExtremeCheckResult {
                        triggered: true,
                        direction: SentimentDirection::Coldness,
                        max_similarity: coldness_max,
                        warmth_max,
                        coldness_max,
                        threshold: self.threshold,
                    }
                }
            }
            (true, false) => ExtremeCheckResult {
                triggered: true,
                direction: SentimentDirection::Warmth,
                max_similarity: warmth_max,
                warmth_max,
                coldness_max,
                threshold: self.threshold,
            },
            (false, true) => ExtremeCheckResult {
                triggered: true,
                direction: SentimentDirection::Coldness,
                max_similarity: coldness_max,
                warmth_max,
                coldness_max,
                threshold: self.threshold,
            },
            (false, false) => ExtremeCheckResult {
                triggered: false,
                direction: SentimentDirection::None,
                max_similarity: warmth_max.max(coldness_max),
                warmth_max,
                coldness_max,
                threshold: self.threshold,
            },
        }
    }

    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }
}

// ---------------------------------------------------------------------------
// SentimentJudgeConfig — 판정 설정
// ---------------------------------------------------------------------------

/// LLM 감정 판정 설정.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SentimentJudgeConfig {
    /// 정기 판정 주기 (턴 수). 기본 12.
    periodic_turns: u32,
    /// 같은 방향 극단 트리거 쿨다운 (턴 수). 기본 6. [v4.5]
    ///
    /// 극단 앵커가 트리거된 후, 같은 방향(Warmth/Coldness)의 연속 트리거를 차단하는 턴 수.
    /// 방향이 바뀌면 쿨다운과 무관하게 즉시 허용된다.
    #[serde(default = "default_cooldown_turns")]
    cooldown_turns: u32,
}

fn default_cooldown_turns() -> u32 {
    6
}

impl SentimentJudgeConfig {
    pub fn new(periodic_turns: u32) -> Self {
        Self {
            periodic_turns,
            cooldown_turns: 6,
        }
    }

    /// 쿨다운 턴 수를 지정하여 생성한다.
    pub fn with_cooldown(periodic_turns: u32, cooldown_turns: u32) -> Self {
        Self {
            periodic_turns,
            cooldown_turns,
        }
    }

    pub fn periodic_turns(&self) -> u32 {
        self.periodic_turns
    }

    pub fn cooldown_turns(&self) -> u32 {
        self.cooldown_turns
    }
}

impl Default for SentimentJudgeConfig {
    fn default() -> Self {
        Self {
            periodic_turns: 12,
            cooldown_turns: 6,
        }
    }
}

// ---------------------------------------------------------------------------
// TurnCounter — 정기 판정 턴 카운터
// ---------------------------------------------------------------------------

/// 정기 판정 턴 카운터.
///
/// `tick()`를 호출하면 카운터가 1 증가하고,
/// period에 도달하면 `true`를 반환하고 리셋한다.
///
/// # Example
/// ```
/// use wuxia_core::relationship::sentiment::TurnCounter;
///
/// let mut counter = TurnCounter::new(3);
/// assert!(!counter.tick()); // 1
/// assert!(!counter.tick()); // 2
/// assert!(counter.tick());  // 3 → true + 리셋
/// assert!(!counter.tick()); // 1 (리셋됨)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnCounter {
    count: u32,
    period: u32,
}

impl TurnCounter {
    pub fn new(period: u32) -> Self {
        Self { count: 0, period }
    }

    /// 카운터를 1 증가시킨다.
    ///
    /// period에 도달하면 `true`를 반환하고 카운터를 리셋한다.
    pub fn tick(&mut self) -> bool {
        self.count += 1;
        if self.count >= self.period {
            self.count = 0;
            true
        } else {
            false
        }
    }

    /// 카운터를 0으로 리셋한다.
    ///
    /// 극단 트리거 발생 시 카운터를 리셋하여
    /// 다시 period만큼 대기하도록 한다.
    pub fn reset(&mut self) {
        self.count = 0;
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn period(&self) -> u32 {
        self.period
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "sentiment_tests.rs"]
mod tests;
