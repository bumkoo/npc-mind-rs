//! MemoryRanker — 2단계 랭킹 (Source 우선 필터 + 5요소 가중 점수)
//!
//! Step A에서는 순수 함수로 구현하고 단위 테스트만 제공. 실제 호출 경로(DialogueAgent 주입 등)는
//! Step B 이후에 연결된다.

use super::{MemoryEntry, MemorySource, MemoryType, Provenance};
use crate::domain::tuning::{
    DAY_MS, DECAY_TAU_DEFAULT_DAYS, EMOTION_PROXIMITY_BONUS, RECALL_BOOST_FACTOR,
    RECENCY_BOOST_TAU_DAYS, SIMILARITY_CLUSTER_THRESHOLD,
};

// ---------------------------------------------------------------------------
// 공개 타입
// ---------------------------------------------------------------------------

/// Ranker 입력 후보 — MemoryStore에서 뽑힌 엔트리 + 의미 유사도.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub entry: MemoryEntry,
    /// 벡터 의미 유사도 (0.0~1.0). 키워드-only 검색이면 `1.0` 채우고 temporal_recency만 쓰는 식.
    pub vec_similarity: f32,
    /// 1단계 Topic-없는 클러스터링에 쓸 임베딩. None이면 Topic 없는 엔트리는 단독 그룹.
    pub embedding: Option<Vec<f32>>,
}

/// Ranker 질의 맥락.
#[derive(Debug, Clone, Default)]
pub struct RankQuery {
    /// 현재 화자의 PAD — emotion_proximity 계산용
    pub current_pad: Option<(f32, f32, f32)>,
    /// 반환 상한 (0이면 전부)
    pub limit: usize,
    /// 최소 점수 컷오프. `MEMORY_RETENTION_CUTOFF` 기본값을 시사.
    pub min_score_cutoff: f32,
}

/// Ranker 결과 — 엔트리 + 최종 점수.
#[derive(Debug, Clone)]
pub struct RankedEntry {
    pub entry: MemoryEntry,
    pub score: f32,
}

// ---------------------------------------------------------------------------
// DecayTauTable — (MemoryType, MemorySource, Provenance) → τ(days)
// ---------------------------------------------------------------------------

/// 기억 유형 × 출처 × 계보 3축 감쇠 τ 룩업 테이블.
///
/// Canonical(`Seeded + World`) 판정은 엔트리 레벨에서 처리(retention_curve)되므로, 이 테이블은
/// τ 값을 days로만 표현한다. 미매핑 조합은 `DECAY_TAU_DEFAULT_DAYS`를 사용.
#[derive(Debug, Clone)]
pub struct DecayTauTable {
    entries: Vec<(MemoryType, MemorySource, Provenance, f32)>,
    default_days: f32,
}

impl DecayTauTable {
    /// 기본 룩업 (문서 §9 표 기준값).
    pub fn default_table() -> Self {
        use MemorySource::*;
        use MemoryType::*;
        use Provenance::*;
        let entries = vec![
            (DialogueTurn, Experienced, Runtime, 15.0),
            (DialogueTurn, Witnessed, Runtime, 30.0),
            (DialogueTurn, Heard, Runtime, 14.0),
            (DialogueTurn, Rumor, Runtime, 7.0),
            (BeatTransition, Experienced, Runtime, 45.0),
            (BeatTransition, Witnessed, Runtime, 45.0),
            (SceneSummary, Experienced, Runtime, 90.0),
            (SceneSummary, Witnessed, Runtime, 90.0),
            (RelationshipChange, Experienced, Runtime, 60.0),
            (RelationshipChange, Witnessed, Runtime, 60.0),
            (WorldEvent, Experienced, Runtime, 180.0),
            (WorldEvent, Witnessed, Runtime, 180.0),
            // Seeded 공용 지식 — 시드 상태에서는 영구 (f32::INFINITY)
            (FactionKnowledge, Experienced, Seeded, f32::INFINITY),
            (FamilyFact, Experienced, Seeded, f32::INFINITY),
        ];
        Self {
            entries,
            default_days: DECAY_TAU_DEFAULT_DAYS,
        }
    }

    /// 3축 룩업.
    pub fn lookup(&self, ty: &MemoryType, src: MemorySource, prov: Provenance) -> f32 {
        for (t, s, p, v) in &self.entries {
            if t == ty && *s == src && *p == prov {
                return *v;
            }
        }
        self.default_days
    }
}

impl Default for DecayTauTable {
    fn default() -> Self {
        Self::default_table()
    }
}

// ---------------------------------------------------------------------------
// 1단계 — Source 우선 필터 (A9)
// ---------------------------------------------------------------------------

/// 동일 Topic(또는 유사 내용 클러스터) 후보 중 `source.priority()` 최소치만 살린다.
///
/// - Topic이 있는 후보는 Topic별 그룹.
/// - Topic이 없는 후보는 embedding cosine ≥ `SIMILARITY_CLUSTER_THRESHOLD`로 근사 클러스터링.
///   embedding이 없는 후보는 단독 그룹.
/// - 각 그룹에서 `min(priority)` 후보만 남긴다. 서로 다른 그룹 간에는 필터링하지 않음.
pub fn filter_by_source_priority(candidates: Vec<Candidate>) -> Vec<Candidate> {
    // Topic이 있는 후보와 없는 후보로 분리
    let mut by_topic: std::collections::HashMap<String, Vec<Candidate>> =
        std::collections::HashMap::new();
    let mut topicless: Vec<Candidate> = Vec::new();

    for c in candidates {
        if let Some(topic) = c.entry.topic.clone() {
            by_topic.entry(topic).or_default().push(c);
        } else {
            topicless.push(c);
        }
    }

    let mut out: Vec<Candidate> = Vec::new();

    // Topic 그룹: 최소 priority만 살림
    for (_, group) in by_topic {
        out.extend(keep_min_priority(group));
    }

    // Topic-없는 후보: embedding cosine 클러스터링
    let clusters = cluster_by_embedding(topicless, SIMILARITY_CLUSTER_THRESHOLD);
    for group in clusters {
        out.extend(keep_min_priority(group));
    }

    out
}

fn keep_min_priority(mut group: Vec<Candidate>) -> Vec<Candidate> {
    if group.is_empty() {
        return group;
    }
    let min_prio = group.iter().map(|c| c.entry.source.priority()).min().unwrap();
    group.retain(|c| c.entry.source.priority() == min_prio);
    group
}

/// 탐욕적 클러스터링 — 각 후보를 첫 센트로이드에 대한 cosine≥threshold인 그룹에 배정.
/// embedding 없는 후보는 단독 클러스터.
fn cluster_by_embedding(candidates: Vec<Candidate>, threshold: f32) -> Vec<Vec<Candidate>> {
    let mut clusters: Vec<Vec<Candidate>> = Vec::new();
    for c in candidates {
        match &c.embedding {
            Some(emb) => {
                let mut placed = false;
                for cluster in clusters.iter_mut() {
                    if let Some(centroid) = cluster.first().and_then(|x| x.embedding.as_ref()) {
                        if cosine(emb, centroid) >= threshold {
                            cluster.push(c.clone());
                            placed = true;
                            break;
                        }
                    }
                }
                if !placed {
                    clusters.push(vec![c]);
                }
            }
            None => clusters.push(vec![c]),
        }
    }
    clusters
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..len {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

// ---------------------------------------------------------------------------
// 2단계 — 5요소 가중 점수 (A10)
// ---------------------------------------------------------------------------

/// 최종 점수 = `vec_similarity × retention × source_confidence × emotion_proximity × temporal_recency`.
pub fn final_score(
    entry: &MemoryEntry,
    vec_similarity: f32,
    query_pad: Option<(f32, f32, f32)>,
    now_ms: u64,
    tau_table: &DecayTauTable,
) -> f32 {
    let retention = retention_curve(entry, now_ms, tau_table);
    let source_confidence = entry.source.weight() * entry.confidence;
    let emotion_proximity = query_pad
        .and_then(|q| entry.emotional_context.map(|e| pad_cosine(e, q)))
        .map(|c| 1.0 + c * EMOTION_PROXIMITY_BONUS)
        .unwrap_or(1.0);
    let temporal_recency = recency_boost(entry.timestamp_ms, now_ms);

    vec_similarity * retention * source_confidence * emotion_proximity * temporal_recency
}

/// retention = `exp(-age_days / τ) × (1 + ln1p(recall_count) × boost)`, clamp [0,1].
/// Canonical(`Seeded ∧ World` — τ=∞)은 1.0 고정.
pub fn retention_curve(e: &MemoryEntry, now_ms: u64, tau: &DecayTauTable) -> f32 {
    // Canonical 단축: Seeded + World scope이면 τ 룩업도 무한 → 1.0
    if e.provenance.is_canonical(&e.scope) {
        return 1.0;
    }
    let tau_days = tau.lookup(&e.memory_type, e.source, e.provenance);
    if tau_days.is_infinite() {
        return 1.0;
    }

    let ref_ms = e.last_recalled_at.unwrap_or(e.timestamp_ms);
    let age_ms = now_ms.saturating_sub(ref_ms);
    let age_days = age_ms as f32 / DAY_MS as f32;

    let base = (-age_days / tau_days).exp();
    let boost = 1.0 + (e.recall_count as f32).ln_1p() * RECALL_BOOST_FACTOR;
    (base * boost).clamp(0.0, 1.0)
}

/// `exp(-age_days / τ_recency)` — 최근 장면 우선 단기 가산.
fn recency_boost(timestamp_ms: u64, now_ms: u64) -> f32 {
    let age_days = now_ms.saturating_sub(timestamp_ms) as f32 / DAY_MS as f32;
    (-age_days / RECENCY_BOOST_TAU_DAYS).exp()
}

/// PAD 삼차원 코사인 유사도 (-1.0 ~ 1.0). 영벡터면 0.
fn pad_cosine(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let dot = a.0 * b.0 + a.1 * b.1 + a.2 * b.2;
    let na = (a.0 * a.0 + a.1 * a.1 + a.2 * a.2).sqrt();
    let nb = (b.0 * b.0 + b.1 * b.1 + b.2 * b.2).sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}

// ---------------------------------------------------------------------------
// MemoryRanker — public API
// ---------------------------------------------------------------------------

/// 2단계 랭커 — filter_by_source_priority → final_score 정렬/컷오프.
pub struct MemoryRanker<'a> {
    pub tau_table: &'a DecayTauTable,
}

impl<'a> MemoryRanker<'a> {
    pub fn new(tau_table: &'a DecayTauTable) -> Self {
        Self { tau_table }
    }

    pub fn rank(
        &self,
        candidates: Vec<Candidate>,
        query: &RankQuery,
        now_ms: u64,
    ) -> Vec<RankedEntry> {
        let filtered = filter_by_source_priority(candidates);
        let mut scored: Vec<RankedEntry> = filtered
            .into_iter()
            .map(|c| {
                let score = final_score(
                    &c.entry,
                    c.vec_similarity,
                    query.current_pad,
                    now_ms,
                    self.tau_table,
                );
                RankedEntry { entry: c.entry, score }
            })
            .filter(|r| r.score >= query.min_score_cutoff)
            .collect();
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if query.limit > 0 && scored.len() > query.limit {
            scored.truncate(query.limit);
        }
        scored
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::memory::{MemoryEntry, MemoryScope, MemorySource, MemoryType, Provenance};

    fn experienced_entry(id: &str, topic: Option<&str>, ts_ms: u64) -> MemoryEntry {
        let mut e = MemoryEntry::personal(
            id,
            "npc1",
            format!("{id} content"),
            None,
            ts_ms,
            1,
            MemoryType::DialogueTurn,
        );
        e.topic = topic.map(|s| s.to_string());
        e.source = MemorySource::Experienced;
        e
    }

    fn heard_entry(id: &str, topic: Option<&str>, ts_ms: u64) -> MemoryEntry {
        let mut e = experienced_entry(id, topic, ts_ms);
        e.source = MemorySource::Heard;
        e.confidence = 0.7;
        e
    }

    #[test]
    fn ranker_filter_source_priority_drops_lower_within_same_topic() {
        let cands = vec![
            Candidate {
                entry: experienced_entry("e1", Some("t"), 0),
                vec_similarity: 0.9,
                embedding: None,
            },
            Candidate {
                entry: heard_entry("h1", Some("t"), 0),
                vec_similarity: 0.95,
                embedding: None,
            },
        ];
        let out = filter_by_source_priority(cands);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].entry.id, "e1");
    }

    #[test]
    fn ranker_filter_different_topics_not_dropped() {
        let cands = vec![
            Candidate {
                entry: experienced_entry("e1", Some("topic_a"), 0),
                vec_similarity: 0.9,
                embedding: None,
            },
            Candidate {
                entry: heard_entry("h1", Some("topic_b"), 0),
                vec_similarity: 0.9,
                embedding: None,
            },
        ];
        let out = filter_by_source_priority(cands);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn ranker_retention_canonical_is_one() {
        let table = DecayTauTable::default_table();
        let mut e = MemoryEntry::personal(
            "c1",
            "_",
            "fact",
            None,
            0,
            1,
            MemoryType::WorldEvent,
        );
        e.scope = MemoryScope::World {
            world_id: "jianghu".into(),
        };
        e.provenance = Provenance::Seeded;
        // 1000일 경과해도 Canonical이면 1.0
        let now_ms = 1000 * DAY_MS;
        assert!((retention_curve(&e, now_ms, &table) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ranker_retention_decay_at_tau_is_about_e_inverse() {
        let table = DecayTauTable::default_table();
        // DialogueTurn + Experienced + Runtime → τ=15일
        let e = MemoryEntry::personal(
            "x",
            "npc1",
            "hi",
            None,
            0,
            1,
            MemoryType::DialogueTurn,
        );
        let now_ms = 15 * DAY_MS;
        let r = retention_curve(&e, now_ms, &table);
        // e^-1 ≈ 0.3679, recall_count=0이므로 boost=1
        assert!((r - (1.0 / std::f32::consts::E)).abs() < 0.01, "r={r}");
    }

    #[test]
    fn ranker_recall_count_boosts_retention() {
        let table = DecayTauTable::default_table();
        let mut e0 = MemoryEntry::personal("x", "npc1", "hi", None, 0, 1, MemoryType::DialogueTurn);
        let mut e5 = e0.clone();
        e5.recall_count = 5;
        e5.id = "y".into();
        let now_ms = 10 * DAY_MS;
        e0.recall_count = 0;
        let r0 = retention_curve(&e0, now_ms, &table);
        let r5 = retention_curve(&e5, now_ms, &table);
        assert!(r5 > r0, "recall_count boost failed: r0={r0}, r5={r5}");
    }

    #[test]
    fn ranker_final_score_emotion_proximity_adds_bonus() {
        let table = DecayTauTable::default_table();
        let mut e = MemoryEntry::personal(
            "x",
            "npc1",
            "hi",
            Some((0.5, 0.2, 0.1)),
            0,
            1,
            MemoryType::DialogueTurn,
        );
        e.source = MemorySource::Experienced;
        // 같은 방향 PAD (cosine=1) → 점수 상향
        let s_match = final_score(&e, 1.0, Some((1.0, 0.4, 0.2)), 0, &table);
        // 반대 PAD (cosine=-1) → 점수 하향
        let s_opposite = final_score(&e, 1.0, Some((-1.0, -0.4, -0.2)), 0, &table);
        assert!(s_match > s_opposite, "match={s_match} opposite={s_opposite}");
    }

    #[test]
    fn ranker_rank_orders_and_limits() {
        let table = DecayTauTable::default_table();
        let q = RankQuery {
            current_pad: None,
            limit: 2,
            min_score_cutoff: 0.0,
        };
        let cands = vec![
            Candidate {
                entry: experienced_entry("a", Some("ta"), 0),
                vec_similarity: 0.9,
                embedding: None,
            },
            Candidate {
                entry: experienced_entry("b", Some("tb"), 0),
                vec_similarity: 0.5,
                embedding: None,
            },
            Candidate {
                entry: experienced_entry("c", Some("tc"), 0),
                vec_similarity: 0.7,
                embedding: None,
            },
        ];
        let ranker = MemoryRanker::new(&table);
        let out = ranker.rank(cands, &q, 0);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].entry.id, "a");
        assert_eq!(out[1].entry.id, "c");
    }
}
