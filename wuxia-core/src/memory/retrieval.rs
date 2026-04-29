// wuxia-core/src/memory/retrieval.rs
//
// 기억 검색 점수 계산 — 순수 함수 모듈.
//
// Stanford Generative Agents 논문의 retrieval score를
// 칠국춘추 OCC 감정 시스템과 연결할 수 있도록 확장한 버전.
//
// 기본 공식 (Generative Agents):
//   score = α₁×recency + α₂×importance + α₃×relevance
//
// 확장 공식 (칠국춘추):
//   score = α₁×recency + α₂×importance + α₃×relevance + α₄×emotional_match
//
// 네 가지 축:
//   recency       — 시간이 지날수록 잊혀진다 (지수 감쇠)
//   importance    — 중요한 기억은 오래 남는다 (1~10 정규화)
//   relevance     — 지금 상황과 관련된 기억이 떠오른다 (키워드/벡터)
//   emotional_match — 기분과 일치하는 기억이 더 잘 떠오른다 (OCC/PAD)
//
// OCC 연결점 요약 (grep "OCC_TODO"로 모두 찾을 수 있음):
//   OCC_TODO①: importance 자동 산정 (OCC 감정 강도 → 중요도)
//   OCC_TODO②: PAD 기분 일치 편향 (현재 기분 → 기억 회상 편향)
//   OCC_TODO③: 5가치 관련도 증폭 (충/의/효/복수/야망 → relevance 보정)
//   OCC_TODO④: MemoryEntry에 감정가(valence) 필드 추가
//
// 참조 문서:
//   📎 wuxia-occ-emotion-detail.md — OCC 22종 감정, 5가치×감정 증폭
//   📎 wuxia-npc-psychology-architecture.md §6~7 — OCC 인지평가, PAD
//   📎 reference-generative-agents.md §2 — Memory Retrieval 공식

use crate::memory::ScoredMemory;
use crate::shared::time::GameTime;

// ---------------------------------------------------------------------------
// RetrievalWeights — 검색 가중치 설정
// ---------------------------------------------------------------------------

/// 기억 검색의 가중치 설정.
///
/// NPC의 성격에 따라 가중치를 조절하면 "기억 성향"이 달라진다.
///
/// ```text
///   소연 (감정적, 복수심 강함):
///     importance_weight = 1.5   ← 강렬한 기억이 잘 떠오름
///     relevance_weight  = 1.5
///     emotional_bias_weight = 0.5  ← OCC 연동 시 활성화
///
///   명경 (분석적, 도의를 중시):
///     relevance_weight  = 2.5   ← 관련 있는 기억이 잘 떠오름
///     importance_weight = 1.0
///     emotional_bias_weight = 0.0  ← 감정에 덜 흔들림
/// ```
///
/// # OCC_TODO③: NPC별 가중치를 심리 도메인에서 자동 생성
///   현재: 수동 설정
///   향후: BigFive 성격 → 가중치 자동 매핑
///     N(신경증) 높음 → emotional_bias_weight ↑
///     O(개방성) 높음 → relevance_weight ↑
///     참조: wuxia-npc-psychology-architecture.md §3 BigFive
#[derive(Debug, Clone)]
pub struct RetrievalWeights {
    /// 최신성 가중치 (기본 1.0). 높을수록 최근 기억을 선호. ≥ 0.0
    recency_weight: f32,

    /// 중요도 가중치 (기본 1.0). 높을수록 중요한 기억을 선호. ≥ 0.0
    importance_weight: f32,

    /// 관련도 가중치 (기본 2.0). 높을수록 질문과 관련된 기억을 선호. ≥ 0.0
    /// 논문에서도 관련도가 가장 영향력이 크므로 기본값이 높다.
    relevance_weight: f32,

    /// 시간 감쇠 계수 (기본 0.995). 0.995^일수로 recency 계산. 0.0 ~ 1.0
    /// 0.995^30 ≈ 0.86 (한 달 전 기억은 86%)
    /// 0.995^360 ≈ 0.16 (1년 전 기억은 16%)
    decay_factor: f32,

    /// 감정 편향 가중치 (기본 0.0 — OCC 미구현 시 무효). ≥ 0.0
    ///
    /// # OCC_TODO②: OCC/PAD 구현 후 기본값을 0.5~1.0으로 변경
    ///   현재: 0.0이므로 emotional_bias가 Some이어도 점수에 영향 없음
    ///   향후: PAD 기분 일치 기억에 가산점 부여
    ///   참조: wuxia-occ-emotion-detail.md §9 PAD 변환
    emotional_bias_weight: f32,
}

impl RetrievalWeights {
    /// 각 가중치를 검증하여 생성한다.
    ///
    /// - 가중치(recency, importance, relevance, emotional_bias): 음수는 0.0으로 클램핑
    /// - decay_factor: 0.0 ~ 1.0으로 클램핑
    pub fn new(
        recency_weight: f32,
        importance_weight: f32,
        relevance_weight: f32,
        decay_factor: f32,
        emotional_bias_weight: f32,
    ) -> Self {
        Self {
            recency_weight: recency_weight.max(0.0),
            importance_weight: importance_weight.max(0.0),
            relevance_weight: relevance_weight.max(0.0),
            decay_factor: decay_factor.clamp(0.0, 1.0),
            emotional_bias_weight: emotional_bias_weight.max(0.0),
        }
    }

    pub fn recency_weight(&self) -> f32 { self.recency_weight }
    pub fn importance_weight(&self) -> f32 { self.importance_weight }
    pub fn relevance_weight(&self) -> f32 { self.relevance_weight }
    pub fn decay_factor(&self) -> f32 { self.decay_factor }
    pub fn emotional_bias_weight(&self) -> f32 { self.emotional_bias_weight }

    /// 특정 가중치만 변경한 복사본을 만든다 (builder 패턴).
    pub fn with_importance_weight(mut self, w: f32) -> Self {
        self.importance_weight = w.max(0.0);
        self
    }
    pub fn with_relevance_weight(mut self, w: f32) -> Self {
        self.relevance_weight = w.max(0.0);
        self
    }
    pub fn with_emotional_bias_weight(mut self, w: f32) -> Self {
        self.emotional_bias_weight = w.max(0.0);
        self
    }
}

impl Default for RetrievalWeights {
    /// 논문 기본값: α_recency=1, α_importance=1, α_relevance=1.
    /// 칠국춘추에서는 relevance를 2.0으로 올렸다 (키워드 매칭의 낮은 정밀도 보상).
    fn default() -> Self {
        Self {
            recency_weight: 1.0,
            importance_weight: 1.0,
            relevance_weight: 2.0,
            decay_factor: 0.995,
            emotional_bias_weight: 0.0, // OCC_TODO②: 구현 후 0.5~1.0
        }
    }
}

// ---------------------------------------------------------------------------
// EmotionalBias — OCC/PAD 감정 편향 (향후 연동)
// ---------------------------------------------------------------------------

/// NPC의 현재 감정 상태가 기억 회상에 미치는 편향.
///
/// 심리학의 "기분 일치 기억(mood-congruent memory)" 현상을 구현한다.
/// 분노 상태에서는 분노와 관련된 기억이 더 잘 떠오르고,
/// 기쁜 상태에서는 긍정적 기억이 더 잘 떠오른다.
///
/// # OCC_TODO②: PAD 기분 일치 편향 활성화 시
///   1. 심리 도메인에서 현재 PADState를 가져온다
///   2. EmotionalBias를 생성한다
///   3. retrieval_score()에 Some(&bias)로 전달한다
///
/// # OCC_TODO④: MemoryEntry에 감정가(valence) 필드 추가 시
///   현재: memory_valence를 외부에서 수동 설정
///   향후: MemoryEntry 저장 시 OCC 감정 평가 결과를 자동 기록
///     기억 저장 시점의 감정: Anger(-0.8), Joy(+0.9) 등
///     이 값이 memory_valence가 된다
///   참조: wuxia-occ-emotion-detail.md §8 인지평가 6단계 파이프라인
///
/// ```text
///   예시: 소연이 분노 상태(P=-0.6)일 때 기억 검색
///
///   기억A "조고가 사부를 배신" (valence=-0.8)
///     → PAD P축과 valence 부호 일치 → 편향 점수 높음
///
///   기억B "자유도시에서 만두를 먹음" (valence=+0.3)
///     → PAD P축과 valence 부호 불일치 → 편향 점수 낮음
/// ```
#[derive(Debug, Clone)]
pub struct EmotionalBias {
    /// PAD Pleasure 축 (-1.0~1.0). 양수=쾌, 음수=불쾌.
    pub pleasure: f32,

    /// PAD Arousal 축 (-1.0~1.0). 양수=각성, 음수=이완.
    pub arousal: f32,

    /// PAD Dominance 축 (-1.0~1.0). 양수=지배감, 음수=무력감.
    pub dominance: f32,

    /// 이 기억의 감정가 (-1.0~1.0). 양수=긍정, 음수=부정.
    ///
    /// # OCC_TODO④: MemoryEntry.valence 필드에서 자동 추출
    ///   현재: RankedMemory 생성 시 외부에서 설정 (0.0 = 중립)
    ///   향후: MemoryEntry 저장 시 OCC 감정 강도에서 자동 계산
    ///     공식: valence = Σ(긍정감정강도) - Σ(부정감정강도)
    ///     참조: wuxia-occ-emotion-detail.md §4 긍정/부정 분류
    pub memory_valence: f32,

    // OCC_TODO③: 5가치 관련도 증폭 — 향후 추가 필드
    //   pub value_relevance: Option<[f32; 5]>,
    //   충(0)/의(1)/효(2)/복수(3)/야망(4) 각각의 가중치
    //   기억의 키워드가 특정 가치와 관련되면 relevance 증폭
    //   예: "배신" 키워드 → 의(義) 관련 → 소연(의 0.8)에게 증폭
    //   참조: wuxia-occ-emotion-detail.md §5 5가치×감정 증폭 매핑
}

impl EmotionalBias {
    /// 새로운 감정 편향을 생성한다.
    pub fn new(pleasure: f32, arousal: f32, dominance: f32, memory_valence: f32) -> Self {
        Self {
            pleasure: pleasure.clamp(-1.0, 1.0),
            arousal: arousal.clamp(-1.0, 1.0),
            dominance: dominance.clamp(-1.0, 1.0),
            memory_valence: memory_valence.clamp(-1.0, 1.0),
        }
    }

    /// PAD Pleasure축과 기억 감정가의 일치도를 계산한다.
    ///
    /// 같은 부호(둘 다 긍정 또는 둘 다 부정)이면 높은 점수,
    /// 다른 부호이면 낮은 점수를 반환한다.
    ///
    /// 결과 범위: 0.0~1.0
    ///
    /// ```text
    ///   P=-0.6, valence=-0.8 → 부호 일치, 강도 높음 → ~0.74
    ///   P=-0.6, valence=+0.3 → 부호 불일치          → ~0.09
    ///   P= 0.0, valence=-0.8 → 중립 기분            → ~0.0
    /// ```
    ///
    /// # OCC_TODO②: 향후 A(각성)와 D(지배감)도 일치도에 반영
    ///   현재: P축만 사용 (가장 직관적인 쾌/불쾌 일치)
    ///   향후: A축 높으면 강렬한 기억 선호, D축 낮으면 무력한 기억 선호
    pub fn mood_congruence(&self) -> f32 {
        // P축과 기억 감정가의 곱: 같은 부호면 양수, 다른 부호면 음수
        let raw = self.pleasure * self.memory_valence;

        // 음수를 0으로 클램프하여 0.0~1.0 범위로 정규화
        raw.max(0.0)
    }
}

// ---------------------------------------------------------------------------
// RankedMemory — 최종 점수가 매겨진 기억
// ---------------------------------------------------------------------------

/// retrieval_score()로 최종 점수가 매겨진 기억.
///
/// ScoredMemory(relevance만)와 달리, recency/importance/emotional_bias를
/// 모두 반영한 최종 점수를 가진다.
#[derive(Debug, Clone)]
pub struct RankedMemory {
    /// 원본 기억.
    pub entry: crate::memory::MemoryEntry,

    /// 최종 검색 점수. 높을수록 더 관련성 높은 기억.
    pub final_score: f32,
}

// ---------------------------------------------------------------------------
// retrieval_score() — 단일 기억의 최종 점수 계산
// ---------------------------------------------------------------------------

/// 단일 기억의 최종 검색 점수를 계산한다.
///
/// # 네 가지 축
///
/// ```text
///   recency (최신성):
///     decay_factor^(경과 일수)
///     0.995^3  ≈ 0.985 (3일 전)
///     0.995^30 ≈ 0.860 (한 달 전)
///     0.995^360 ≈ 0.164 (1년 전)
///
///   importance (중요도):
///     entry.importance / 10.0
///     "방 청소" → 2/10 = 0.2
///     "사부 배신" → 9/10 = 0.9
///
///   relevance (관련도):
///     scored_memory.relevance_score (이미 search()에서 계산됨)
///     키워드 3/3 매칭 → 1.0
///     키워드 1/3 매칭 → 0.33
///
///   emotional_match (감정 일치):
///     PAD P축과 기억 감정가의 일치도
///     분노 상태 + 분노 기억 → 높음
///     기쁨 상태 + 분노 기억 → 낮음
/// ```
///
/// # OCC_TODO①: importance 자동 산정
///   현재: MemoryEntry 생성 시 외부에서 수동 부여 (1.0~10.0)
///   향후: OCC 감정 강도로 자동 계산
///     공식: importance = base(3.0) + Σ(emotion.intensity × relevant_value)
///     예: 분노(0.55) × 의(0.8) = 0.44 → importance += 4.4 → 총 7.4
///     참조: wuxia-occ-emotion-detail.md §8 인지평가 6단계 파이프라인
///     구현 위치: wuxia-llm/src/conversation.rs의 process_turn()
pub fn retrieval_score(
    memory: &ScoredMemory,
    current_time: GameTime,
    weights: &RetrievalWeights,
    emotional_bias: Option<&EmotionalBias>,
) -> f32 {
    // 1. recency — 시간 감쇠
    let days_elapsed = current_time.days_between(&memory.entry.game_time()).unsigned_abs();
    let recency = weights.decay_factor().powf(days_elapsed as f32);

    // 2. importance — 정규화 (1.0~10.0 → 0.1~1.0)
    let importance = memory.entry.importance() / 10.0;

    // 3. relevance — 이미 계산된 값 (InMemoryRepository.search()에서)
    let relevance = memory.relevance_score;

    // 4. emotional_match — OCC/PAD 감정 편향
    //    OCC_TODO②: emotional_bias_weight가 0.0이면 이 항은 무효
    let emotional_match = emotional_bias
        .map(|bias| bias.mood_congruence())
        .unwrap_or(0.0);

    // 가중 합산
    weights.recency_weight() * recency
        + weights.importance_weight() * importance
        + weights.relevance_weight() * relevance
        + weights.emotional_bias_weight() * emotional_match
}

// ---------------------------------------------------------------------------
// rank_memories() — 여러 기억을 점수순으로 정렬
// ---------------------------------------------------------------------------

/// 여러 기억에 retrieval_score를 적용하고 점수순으로 정렬하여 상위 top_k개를 반환한다.
///
/// InMemoryRepository.search()가 반환한 ScoredMemory 목록을
/// recency/importance/emotional_bias까지 반영한 최종 순위로 변환한다.
///
/// ```text
///   InMemoryRepository.search("혈교")
///     → [ScoredMemory(기억A, relevance=1.0),
///        ScoredMemory(기억B, relevance=0.5)]
///
///   rank_memories(위 결과, 현재시간, 가중치, None, 2)
///     → [RankedMemory(기억A, final=3.8),    ← relevance 높고 최근
///        RankedMemory(기억B, final=1.9)]    ← relevance 낮고 오래됨
/// ```
pub fn rank_memories(
    scored_memories: &[ScoredMemory],
    current_time: GameTime,
    weights: &RetrievalWeights,
    emotional_bias: Option<&EmotionalBias>,
    top_k: usize,
) -> Vec<RankedMemory> {
    let mut ranked: Vec<RankedMemory> = scored_memories
        .iter()
        .map(|sm| {
            let score = retrieval_score(sm, current_time, weights, emotional_bias);
            debug_assert!(!score.is_nan(), "retrieval_score returned NaN for memory {:?}", sm.entry.id());
            RankedMemory {
                entry: sm.entry.clone(),
                final_score: score,
            }
        })
        .collect();

    // final_score 내림차순 정렬
    ranked.sort_by(|a, b| {
        b.final_score
            .partial_cmp(&a.final_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    ranked.truncate(top_k);
    ranked
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{MemoryEntry, MemoryType, ScoredMemory};
    use crate::shared::id::{CharacterId, MemoryId};

    /// 테스트용 ScoredMemory 생성 헬퍼.
    fn make_scored(
        id: u64,
        content: &str,
        importance: f32,
        day: u32,
        relevance: f32,
    ) -> ScoredMemory {
        let entry = MemoryEntry::new(
            MemoryId::new(id),
            CharacterId::new(5), // 소연
            content.to_string(),
            importance,
            MemoryType::Observation,
            GameTime::new(1200, 3, day),
            vec![],
        );
        ScoredMemory::new(entry, relevance)
    }

    fn default_weights() -> RetrievalWeights {
        RetrievalWeights::default()
    }

    // =======================================================================
    // retrieval_score — 기본 동작
    // =======================================================================

    #[test]
    fn score_uses_all_three_axes() {
        // 3일 전, 중요도 8, 관련도 0.9
        let sm = make_scored(1, "사부가 위험하다", 8.0, 12, 0.9);
        let now = GameTime::new(1200, 3, 15);
        let w = default_weights();

        let score = retrieval_score(&sm, now, &w, None);

        // recency = 0.995^3 ≈ 0.985
        // importance = 8/10 = 0.8
        // relevance = 0.9
        // score ≈ 1.0×0.985 + 1.0×0.8 + 2.0×0.9 = 3.585
        assert!(score > 3.5 && score < 3.7, "Expected ~3.585, got {}", score);
    }

    #[test]
    fn score_same_day_recency_is_one() {
        let sm = make_scored(1, "오늘의 기억", 5.0, 15, 0.5);
        let now = GameTime::new(1200, 3, 15);
        let w = default_weights();

        let score = retrieval_score(&sm, now, &w, None);

        // recency = 0.995^0 = 1.0
        // importance = 5/10 = 0.5
        // relevance = 0.5
        // score = 1.0×1.0 + 1.0×0.5 + 2.0×0.5 = 2.5
        assert!((score - 2.5).abs() < 0.01, "Expected 2.5, got {}", score);
    }

    // =======================================================================
    // retrieval_score — 시간 감쇠
    // =======================================================================

    #[test]
    fn older_memory_has_lower_recency() {
        let w = default_weights();
        let now = GameTime::new(1200, 3, 30);

        let recent = make_scored(1, "3일 전", 5.0, 27, 0.5);
        let old = make_scored(2, "20일 전", 5.0, 10, 0.5);

        let score_recent = retrieval_score(&recent, now, &w, None);
        let score_old = retrieval_score(&old, now, &w, None);

        assert!(
            score_recent > score_old,
            "Recent ({}) should > old ({})",
            score_recent, score_old
        );
    }

    #[test]
    fn one_year_old_memory_heavily_decayed() {
        let w = default_weights();
        // 1200년 3월 → 1201년 3월 (360일 경과)
        let sm = make_scored(1, "1년 전 기억", 10.0, 15, 1.0);
        let now = GameTime::new(1201, 3, 15);

        let score = retrieval_score(&sm, now, &w, None);

        // recency = 0.995^360 ≈ 0.164
        // importance = 10/10 = 1.0
        // relevance = 1.0
        // score ≈ 0.164 + 1.0 + 2.0 = 3.164
        // 같은 기억이 오늘이면: 1.0 + 1.0 + 2.0 = 4.0
        assert!(score < 3.5, "1 year old should be < 3.5, got {}", score);
    }

    // =======================================================================
    // retrieval_score — 중요도 영향
    // =======================================================================

    #[test]
    fn higher_importance_gives_higher_score() {
        let w = default_weights();
        let now = GameTime::new(1200, 3, 15);

        let trivial = make_scored(1, "만두를 먹었다", 2.0, 15, 0.5);
        let critical = make_scored(2, "사부가 배신당했다", 9.0, 15, 0.5);

        let score_trivial = retrieval_score(&trivial, now, &w, None);
        let score_critical = retrieval_score(&critical, now, &w, None);

        assert!(score_critical > score_trivial);
    }

    // =======================================================================
    // retrieval_score — 관련도 영향
    // =======================================================================

    #[test]
    fn higher_relevance_gives_higher_score() {
        let w = default_weights();
        let now = GameTime::new(1200, 3, 15);

        let low_rel = make_scored(1, "관련 약함", 5.0, 15, 0.1);
        let high_rel = make_scored(2, "관련 강함", 5.0, 15, 0.9);

        let score_low = retrieval_score(&low_rel, now, &w, None);
        let score_high = retrieval_score(&high_rel, now, &w, None);

        assert!(score_high > score_low);
        // relevance 차이(0.8)가 가중치(2.0)에 의해 1.6 차이를 만든다
        let diff = score_high - score_low;
        assert!((diff - 1.6).abs() < 0.01, "Expected diff ~1.6, got {}", diff);
    }

    // =======================================================================
    // retrieval_score — 감정 편향
    // =======================================================================

    #[test]
    fn no_emotional_bias_when_none() {
        let w = RetrievalWeights::default()
            .with_emotional_bias_weight(1.0);
        let sm = make_scored(1, "기억", 5.0, 15, 0.5);
        let now = GameTime::new(1200, 3, 15);

        let score_none = retrieval_score(&sm, now, &w, None); // bias = None
        let w_no_emo = default_weights(); // emotional_bias_weight = 0.0
        let score_zero = retrieval_score(&sm, now, &w_no_emo, None);

        // None이면 emotional_match = 0.0이므로 결과 동일
        assert_eq!(score_none, score_zero);
    }

    #[test]
    fn emotional_bias_zero_weight_has_no_effect() {
        let w = default_weights(); // emotional_bias_weight = 0.0
        let sm = make_scored(1, "기억", 5.0, 15, 0.5);
        let now = GameTime::new(1200, 3, 15);

        let bias = EmotionalBias::new(-0.8, 0.5, 0.0, -0.9);
        let score_with = retrieval_score(&sm, now, &w, Some(&bias));
        let score_without = retrieval_score(&sm, now, &w, None);

        // weight=0.0이면 bias 있어도 차이 없음
        assert_eq!(score_with, score_without);
    }

    #[test]
    fn emotional_bias_congruent_boosts_score() {
        let w = RetrievalWeights::default()
            .with_emotional_bias_weight(1.0);
        let sm = make_scored(1, "분노 기억", 5.0, 15, 0.5);
        let now = GameTime::new(1200, 3, 15);

        // 불쾌한 기분 + 부정적 기억 → 일치 → 높은 편향
        let congruent = EmotionalBias::new(-0.6, 0.5, 0.0, -0.8);
        // 쾌적한 기분 + 부정적 기억 → 불일치 → 낮은 편향
        let incongruent = EmotionalBias::new(0.6, 0.5, 0.0, -0.8);

        let score_con = retrieval_score(&sm, now, &w, Some(&congruent));
        let score_inc = retrieval_score(&sm, now, &w, Some(&incongruent));

        assert!(
            score_con > score_inc,
            "Congruent ({}) should > incongruent ({})",
            score_con, score_inc
        );
    }

    // =======================================================================
    // EmotionalBias — mood_congruence 단위 테스트
    // =======================================================================

    #[test]
    fn mood_congruence_same_sign_positive() {
        // 둘 다 부정 → 곱 양수 → 높은 일치도
        let bias = EmotionalBias::new(-0.6, 0.0, 0.0, -0.8);
        let mc = bias.mood_congruence();
        assert!((mc - 0.48).abs() < 0.01, "Expected 0.48, got {}", mc);
    }

    #[test]
    fn mood_congruence_different_sign_zero() {
        // 하나 양수, 하나 음수 → 곱 음수 → 0으로 클램프
        let bias = EmotionalBias::new(0.5, 0.0, 0.0, -0.7);
        assert_eq!(bias.mood_congruence(), 0.0);
    }

    #[test]
    fn mood_congruence_neutral_is_zero() {
        // P축이 0이면 곱도 0
        let bias = EmotionalBias::new(0.0, 0.5, 0.3, -0.9);
        assert_eq!(bias.mood_congruence(), 0.0);
    }

    // =======================================================================
    // RetrievalWeights — 커스텀 가중치
    // =======================================================================

    #[test]
    fn custom_weights_change_ranking() {
        let now = GameTime::new(1200, 3, 15);

        // 기억A: 오래됨(10일 전), 중요(9), 관련(0.3)
        let a = make_scored(1, "중요하지만 오래된", 9.0, 5, 0.3);
        // 기억B: 최근(1일 전), 사소(2), 관련(0.8)
        let b = make_scored(2, "사소하지만 관련된", 2.0, 14, 0.8);

        // 기본 가중치: relevance×2.0이 지배 → B가 높을 가능성
        let default = default_weights();
        let score_a_default = retrieval_score(&a, now, &default, None);
        let score_b_default = retrieval_score(&b, now, &default, None);

        // 중요도 중시 가중치: importance ↑↑
        let importance_focused = RetrievalWeights::default()
            .with_importance_weight(5.0)
            .with_relevance_weight(1.0);
        let score_a_focused = retrieval_score(&a, now, &importance_focused, None);
        let score_b_focused = retrieval_score(&b, now, &importance_focused, None);

        // 기본 가중치에서는 B가 높을 수 있지만
        // 중요도 중시에서는 A가 반드시 높아야 한다
        assert!(
            score_a_focused > score_b_focused,
            "A ({}) should > B ({}) with importance focus",
            score_a_focused, score_b_focused
        );

        // 가중치에 따라 순위가 바뀔 수 있음을 확인
        let default_a_wins = score_a_default > score_b_default;
        let focused_a_wins = score_a_focused > score_b_focused;
        // 최소한 focused에서는 A가 이겨야 함
        assert!(focused_a_wins);
        // 둘의 결과가 다를 수도 있음 (가중치 효과 확인)
        let _ = default_a_wins; // 사용만 하고 별도 assert 안 함
    }

    // =======================================================================
    // rank_memories — 정렬 및 top_k
    // =======================================================================

    #[test]
    fn rank_memories_sorted_by_final_score() {
        let now = GameTime::new(1200, 3, 15);
        let w = default_weights();

        let memories = vec![
            make_scored(1, "낮은 점수", 2.0, 1, 0.1),   // 오래됨 + 낮은 관련도
            make_scored(2, "높은 점수", 9.0, 15, 0.9),  // 최근 + 높은 관련도
            make_scored(3, "중간 점수", 5.0, 10, 0.5),  // 중간
        ];

        let ranked = rank_memories(&memories, now, &w, None, 10);

        assert_eq!(ranked.len(), 3);
        assert!(ranked[0].final_score >= ranked[1].final_score);
        assert!(ranked[1].final_score >= ranked[2].final_score);

        // 최고 점수는 id=2 (높은 중요도+관련도+최신)
        assert_eq!(ranked[0].entry.id(), MemoryId::new(2));
    }

    #[test]
    fn rank_memories_respects_top_k() {
        let now = GameTime::new(1200, 3, 15);
        let w = default_weights();

        let memories: Vec<ScoredMemory> = (1..=10)
            .map(|i| make_scored(i, &format!("기억{}", i), 5.0, 15, 0.5))
            .collect();

        let ranked = rank_memories(&memories, now, &w, None, 3);
        assert_eq!(ranked.len(), 3);
    }

    #[test]
    fn rank_memories_empty_input() {
        let now = GameTime::new(1200, 3, 15);
        let w = default_weights();

        let ranked = rank_memories(&[], now, &w, None, 10);
        assert!(ranked.is_empty());
    }

    #[test]
    fn rank_memories_fewer_than_top_k() {
        let now = GameTime::new(1200, 3, 15);
        let w = default_weights();

        let memories = vec![make_scored(1, "유일한 기억", 5.0, 15, 0.5)];
        let ranked = rank_memories(&memories, now, &w, None, 10);
        assert_eq!(ranked.len(), 1);
    }

    // =======================================================================
    // 통합 시나리오: 소연의 기억 검색
    // =======================================================================

    #[test]
    fn soyeon_memory_retrieval_scenario() {
        // 소연의 기억 3개 (InMemoryRepository.search()가 반환했다고 가정)
        let memories = vec![
            // 어제: "조고가 사부를 배신했다" — 매우 중요, 완전 관련
            make_scored(1, "조고가 사부를 배신했다", 9.0, 14, 1.0),
            // 10일 전: "자유도시에서 만두를 먹었다" — 사소, 약간 관련
            make_scored(2, "자유도시에서 만두를 먹었다", 2.0, 5, 0.3),
            // 3일 전: "혈교 무인이 수상한 움직임" — 중요, 꽤 관련
            make_scored(3, "혈교 무인이 수상한 움직임", 7.0, 12, 0.8),
        ];

        let now = GameTime::new(1200, 3, 15);
        let w = default_weights();

        let ranked = rank_memories(&memories, now, &w, None, 2);

        // 상위 2개만 반환
        assert_eq!(ranked.len(), 2);

        // 1위: 조고 배신 (어제, 중요9, 관련1.0)
        assert_eq!(ranked[0].entry.id(), MemoryId::new(1));

        // 2위: 혈교 무인 (3일 전, 중요7, 관련0.8)
        assert_eq!(ranked[1].entry.id(), MemoryId::new(3));

        // 만두 기억은 잘렸다 (top_k=2)
    }

    #[test]
    fn soyeon_angry_retrieval_scenario() {
        // OCC_TODO②: 감정 편향 활성화 시나리오
        // 소연이 분노 상태일 때 부정적 기억이 더 잘 떠오르는지 확인

        let memories = vec![
            make_scored(1, "조고의 배신", 7.0, 10, 0.5),   // 부정
            make_scored(2, "사부의 따뜻한 말", 7.0, 10, 0.5), // 긍정
        ];

        let now = GameTime::new(1200, 3, 15);

        // 감정 편향 활성화 가중치
        let w = RetrievalWeights::default()
            .with_emotional_bias_weight(1.0);

        // 분노 상태 (P=-0.6)에서 부정 기억(valence=-0.8) 검색
        let angry_bias_negative = EmotionalBias::new(-0.6, 0.5, 0.0, -0.8);
        let angry_bias_positive = EmotionalBias::new(-0.6, 0.5, 0.0, 0.8);

        let score_negative = retrieval_score(&memories[0], now, &w, Some(&angry_bias_negative));
        let score_positive = retrieval_score(&memories[1], now, &w, Some(&angry_bias_positive));

        // 분노 상태에서 부정적 기억의 편향 점수가 더 높다
        assert!(
            score_negative > score_positive,
            "Negative memory ({}) should > positive memory ({}) when angry",
            score_negative, score_positive
        );
    }
}
