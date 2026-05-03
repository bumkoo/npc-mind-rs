use crate::domain::memory::ranker::{Candidate, DecayTauTable, MemoryRanker, RankQuery};
use crate::domain::tuning::profile;
use crate::ports::{MemoryFramer, MemoryQuery, MemoryScopeFilter, MemoryStore, UtteranceAnalyzer};

/// 기억 증강 서비스 — 상황에 맞는 기억을 검색, 랭킹, 프레이밍하여 프롬프트에 주입할 블록 생성
pub struct MemoryAugmentationService;

impl MemoryAugmentationService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MemoryAugmentationService {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryAugmentationService {
    /// 주어진 쿼리와 상태(PAD)를 기반으로 관련 기억을 찾아 프롬프트 블록으로 반환
    pub fn augment<A: UtteranceAnalyzer + ?Sized>(
        &self,
        npc_id: &str,
        query: &str,
        pad: Option<(f32, f32, f32)>,
        analyzer: Option<&mut A>,
        store: &dyn MemoryStore,
        framer: &dyn MemoryFramer,
        locale: &str,
    ) -> String {
        // 1) 임베딩 생성 — analyzer가 있으면 쿼리 텍스트로 임베딩 생성.
        let query_embedding: Option<Vec<f32>> = match analyzer {
            Some(a) => match a.analyze_with_embedding(query) {
                Ok((_pad, emb)) => emb.map(|e| e.into_inner()),
                Err(e) => {
                    tracing::debug!("MemoryAugmentationService.augment: embedding 실패 {:?}", e);
                    None
                }
            },
            None => None,
        };

        // 2) MemoryStore 검색 — NpcAllowed scope, Top-K * 3 oversample (Ranker가 다시 K로 줄임)
        let tuning = profile();
        let top_k = tuning.memory_push_top_k;
        let oversample = (top_k * 3).max(top_k);
        
        let mem_query = MemoryQuery {
            text: Some(query.to_string()),
            embedding: query_embedding,
            scope_filter: Some(MemoryScopeFilter::NpcAllowed(npc_id.to_string())),
            source_filter: None,
            layer_filter: None,
            topic: None,
            exclude_superseded: true,
            exclude_consolidated_source: true,
            min_retention: Some(tuning.memory_retention_cutoff),
            current_pad: pad,
            limit: oversample,
        };

        let results = match store.search(mem_query) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("MemoryAugmentationService.augment: store.search 실패 {:?}", e);
                return String::new();
            }
        };

        if results.is_empty() {
            return String::new();
        }

        // 3) Ranker 적용 — 1단계 Source 우선 필터 + 2단계 5요소 점수
        let candidates: Vec<Candidate> = results
            .into_iter()
            .map(|r| Candidate {
                entry: r.entry,
                vec_similarity: r.relevance_score,
                embedding: None,
            })
            .collect();

        let tau = DecayTauTable::default_table();
        let ranker = MemoryRanker::new(&tau);
        let rq = RankQuery {
            current_pad: pad,
            limit: top_k,
            min_score_cutoff: 0.0,
        };

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let ranked = ranker.rank(candidates, &rq, now_ms);
        if ranked.is_empty() {
            return String::new();
        }

        // 4) record_recall (best-effort)
        for r in &ranked {
            if let Err(e) = store.record_recall(&r.entry.id, now_ms) {
                tracing::debug!(
                    "MemoryAugmentationService.augment: record_recall({}) 실패 {:?}",
                    r.entry.id,
                    e
                );
            }
        }

        // 5) Framer로 블록 구성
        let final_entries: Vec<_> = ranked.into_iter().map(|c| c.entry).collect();
        framer.frame_block(&final_entries, locale)
    }
}
