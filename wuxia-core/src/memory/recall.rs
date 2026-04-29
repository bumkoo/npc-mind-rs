// wuxia-core/src/memory/recall.rs
//
// recall_memories — 기억 회상 도메인 서비스.
//
// "NPC가 관련 기억을 떠올리는 행위"를 모델링한다.
// repo.search() (Output Port) + rank_memories() (기존 Domain Service)를
// 하나의 도메인 개념으로 묶는다.
//
// 프롬프트 포맷팅은 하지 않는다 (표현 관심사는 wuxia-llm 담당).
//
// 무협 비유:
//   소연이 "혈교"라는 말을 듣고 관련 기억을 머릿속에서 꺼내는 행위.
//   어떤 말투로 표현할지는 별개 문제 — 그건 wuxia-llm이 한다.
//
// 아키텍처:
//   위치: wuxia-core (Domain Service)
//   의존: MemoryRepository (Output Port), rank_memories() (같은 Core)
//   호출자: LiveContextProvider (wuxia-llm)

use crate::memory::port::MemoryRepository;
use crate::memory::retrieval::{rank_memories, EmotionalBias, RankedMemory, RetrievalWeights};
use crate::shared::id::CharacterId;
use crate::shared::time::GameTime;

/// NPC가 관련 기억을 떠올린다.
///
/// 1. `repo.search()` — 의미 기반 검색 (벡터 유사도 / 키워드)
/// 2. `rank_memories()` — 4축 랭킹 (recency + importance + relevance + emotion)
///
/// # Arguments
/// * `repo` — 기억 저장소 (Output Port).
/// * `character_id` — 기억을 회상하는 NPC.
/// * `query` — 검색어 (대화 맥락에서 추출).
/// * `current_time` — 현재 게임 시간 (recency 계산용).
/// * `weights` — NPC 성격에 따른 검색 가중치.
/// * `emotional_bias` — 현재 감정 상태 (OCC/PAD, 향후 연동).
/// * `search_top_k` — 벡터 검색 후보 수 (넓게 가져옴).
/// * `rank_top_k` — 최종 반환할 기억 수 (좁게 추림).
///
/// # Returns
/// 최종 점수 순으로 정렬된 `RankedMemory` 벡터 (최대 `rank_top_k`개).
///
/// # Example
/// ```text
///   소연이 "혈교가 움직인다"는 말을 들었을 때:
///
///   search("혈교", top_k=10) → 벡터 DB에서 10개 후보
///   rank(10개, weights, top_k=5) → 4축 랭킹 후 상위 5개
///
///   반환: [조고 배신(9.0), 혈교 무인(7.0), 사부 경고(6.5), ...]
/// ```
pub fn recall_memories(
    repo: &dyn MemoryRepository,
    character_id: CharacterId,
    query: &str,
    current_time: GameTime,
    weights: &RetrievalWeights,
    emotional_bias: Option<&EmotionalBias>,
    search_top_k: usize,
    rank_top_k: usize,
) -> Vec<RankedMemory> {
    let scored = repo.search(character_id, query, search_top_k);
    if scored.is_empty() {
        return vec![];
    }
    rank_memories(&scored, current_time, weights, emotional_bias, rank_top_k)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::port::MemoryRepository;
    use crate::memory::types::{MemoryEntry, MemoryType, ScoredMemory};
    use crate::shared::id::{CharacterId, MemoryId};
    use crate::shared::port_error::PortError;

    // ── Mock Repository ──────────────────────────────────────────────────

    struct MockRepo {
        entries: Vec<MemoryEntry>,
    }

    impl MockRepo {
        fn new(entries: Vec<MemoryEntry>) -> Self {
            Self { entries }
        }
    }

    impl MemoryRepository for MockRepo {
        fn save(&mut self, _entry: MemoryEntry) -> Result<(), PortError> {
            Ok(())
        }
        fn find_recent(&self, _cid: CharacterId, _n: usize) -> Vec<MemoryEntry> {
            vec![]
        }
        fn search(&self, cid: CharacterId, query: &str, top_k: usize) -> Vec<ScoredMemory> {
            self.entries
                .iter()
                .filter(|e| e.character_id() == cid)
                .filter(|e| {
                    e.content().contains(query)
                        || e.keywords().iter().any(|k| k.contains(query))
                })
                .take(top_k)
                .map(|e| ScoredMemory::new(e.clone(), 1.0))
                .collect()
        }
        fn find_by_id(&self, memory_id: MemoryId) -> Option<MemoryEntry> {
            self.entries.iter().find(|e| e.id() == memory_id).cloned()
        }
        fn count(&self, _cid: CharacterId) -> usize {
            self.entries.len()
        }
        fn update_importance(
            &mut self,
            _mid: MemoryId,
            _imp: f32,
        ) -> Result<(), PortError> {
            Ok(())
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    fn soyeon_id() -> CharacterId {
        CharacterId::new(5)
    }

    fn make_entry(
        id: u64,
        content: &str,
        importance: f32,
        day: u32,
        keywords: Vec<&str>,
    ) -> MemoryEntry {
        MemoryEntry::new(
            MemoryId::new(id),
            soyeon_id(),
            content.to_string(),
            importance,
            MemoryType::Observation,
            GameTime::new(1200, 3, day),
            keywords.into_iter().map(String::from).collect(),
        )
    }

    // ── Tests ────────────────────────────────────────────────────────────

    #[test]
    fn recall_empty_repo_returns_empty() {
        let repo = MockRepo::new(vec![]);
        let result = recall_memories(
            &repo,
            soyeon_id(),
            "혈교",
            GameTime::new(1200, 3, 15),
            &RetrievalWeights::default(),
            None,
            10,
            5,
        );
        assert!(result.is_empty());
    }

    #[test]
    fn recall_no_match_returns_empty() {
        let repo = MockRepo::new(vec![make_entry(
            1,
            "오늘 만두를 먹었다",
            2.0,
            14,
            vec!["만두"],
        )]);
        let result = recall_memories(
            &repo,
            soyeon_id(),
            "혈교",
            GameTime::new(1200, 3, 15),
            &RetrievalWeights::default(),
            None,
            10,
            5,
        );
        assert!(result.is_empty());
    }

    #[test]
    fn recall_returns_ranked_by_score() {
        let repo = MockRepo::new(vec![
            make_entry(1, "혈교 무인이 수상하다", 7.0, 10, vec!["혈교"]),
            make_entry(2, "혈교가 조고를 보냈다", 9.0, 14, vec!["혈교", "조고"]),
        ]);
        let result = recall_memories(
            &repo,
            soyeon_id(),
            "혈교",
            GameTime::new(1200, 3, 15),
            &RetrievalWeights::default(),
            None,
            10,
            5,
        );

        assert_eq!(result.len(), 2);
        // id=2가 더 최근이고 중요도 높으므로 1위
        assert_eq!(result[0].entry.id(), MemoryId::new(2));
        assert_eq!(result[1].entry.id(), MemoryId::new(1));
    }

    #[test]
    fn recall_respects_rank_top_k() {
        let repo = MockRepo::new(vec![
            make_entry(1, "혈교 기억1", 5.0, 10, vec!["혈교"]),
            make_entry(2, "혈교 기억2", 6.0, 12, vec!["혈교"]),
            make_entry(3, "혈교 기억3", 7.0, 14, vec!["혈교"]),
        ]);
        let result = recall_memories(
            &repo,
            soyeon_id(),
            "혈교",
            GameTime::new(1200, 3, 15),
            &RetrievalWeights::default(),
            None,
            10,
            2, // rank_top_k=2
        );

        assert_eq!(result.len(), 2);
        // 상위 2개만 반환, 가장 점수 높은 것이 1위
        assert_eq!(result[0].entry.id(), MemoryId::new(3));
    }

    #[test]
    fn recall_filters_by_character() {
        let mut entries = vec![make_entry(
            1,
            "소연의 혈교 기억",
            7.0,
            14,
            vec!["혈교"],
        )];
        // 명경의 기억 (다른 character_id)
        entries.push(MemoryEntry::new(
            MemoryId::new(2),
            CharacterId::new(99), // 명경
            "명경의 혈교 기억".to_string(),
            8.0,
            MemoryType::Observation,
            GameTime::new(1200, 3, 14),
            vec!["혈교".to_string()],
        ));

        let repo = MockRepo::new(entries);
        let result = recall_memories(
            &repo,
            soyeon_id(),
            "혈교",
            GameTime::new(1200, 3, 15),
            &RetrievalWeights::default(),
            None,
            10,
            5,
        );

        // 소연의 기억만 반환
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].entry.character_id(), soyeon_id());
    }

    #[test]
    fn recall_search_top_k_limits_candidates() {
        let repo = MockRepo::new(vec![
            make_entry(1, "혈교 기억1", 5.0, 10, vec!["혈교"]),
            make_entry(2, "혈교 기억2", 6.0, 12, vec!["혈교"]),
            make_entry(3, "혈교 기억3", 9.0, 14, vec!["혈교"]),
        ]);
        // search_top_k=1 → repo에서 1개만 가져옴
        let result = recall_memories(
            &repo,
            soyeon_id(),
            "혈교",
            GameTime::new(1200, 3, 15),
            &RetrievalWeights::default(),
            None,
            1, // search_top_k=1
            5, // rank_top_k=5
        );

        // search에서 1개만 가져왔으므로 결과도 1개
        assert_eq!(result.len(), 1);
    }
}
