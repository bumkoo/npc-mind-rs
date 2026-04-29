// wuxia-core/src/memory/port.rs
//
// Memory Repository Port — 헥사고날 아키텍처의 출력 포트.
//
// LlmPort와 동일한 패턴:
//   wuxia-core가 "이런 모양의 문이 필요하다"고 선언하고 (trait),
//   wuxia-memory가 "내가 그 문을 만들었다"고 구현한다 (impl).
//
// 의존성 방향:
//   wuxia-core (MemoryRepository trait 정의) ← wuxia-memory (InMemory/LanceDb 구현)
//   ↑ 이 방향은 절대 역전되지 않는다.
//
// 비유: 강호의 기억 서고(記憶書庫) 규약
//   MemoryRepository = "기억을 이 규격으로 보관하라" (규약)
//   InMemoryRepository = "나는 머릿속에 외운다" (Phase A, 테스트)
//   LanceDbRepository = "나는 비밀 서고에 보관한다" (Phase B, 벡터 검색)
//
// 두 구현체의 search() 차이:
//   InMemory: 키워드 매칭 → relevance 0.0 or 1.0 (단순 매칭)
//   LanceDb: 벡터 코사인 유사도 → relevance 0.0~1.0 (의미 유사도)
//   caller는 이 차이를 모른다 — 포트 추상화의 핵심.

use crate::shared::id::{CharacterId, MemoryId};
use crate::shared::port_error::PortError;

use super::types::{MemoryEntry, ScoredMemory};

/// NPC 기억 저장소 포트 (헥사고날 아키텍처).
///
/// `Send + Sync` 바운드:
///   Bevy의 `Res<dyn MemoryRepository>` 또는 `Arc<dyn MemoryRepository>`로
///   여러 시스템에서 공유할 수 있도록.
///
/// # 구현체
/// - `InMemoryRepository` (wuxia-memory): Phase A. HashMap 기반, 키워드 검색.
/// - `LanceDbRepository` (wuxia-memory): Phase B. 벡터 유사도 검색 + 영속성.
///
/// # Example (Mock 사용)
/// ```
/// use wuxia_core::memory::{MemoryRepository, MemoryEntry, MemoryType, ScoredMemory};
/// use wuxia_core::shared::id::{CharacterId, MemoryId};
/// use wuxia_core::shared::{GameTime, PortError};
///
/// struct MockMemoryRepo;
///
/// impl MemoryRepository for MockMemoryRepo {
///     fn save(&mut self, entry: MemoryEntry) -> Result<(), PortError> {
///         Ok(())
///     }
///     fn find_recent(&self, character_id: CharacterId, n: usize) -> Vec<MemoryEntry> {
///         vec![]
///     }
///     fn search(&self, character_id: CharacterId, query: &str, top_k: usize) -> Vec<ScoredMemory> {
///         vec![]
///     }
///     fn find_by_id(&self, memory_id: MemoryId) -> Option<MemoryEntry> {
///         None
///     }
///     fn count(&self, character_id: CharacterId) -> usize {
///         0
///     }
///     fn update_importance(&mut self, memory_id: MemoryId, new_importance: f32) -> Result<(), PortError> {
///         Ok(())
///     }
/// }
///
/// let mut repo = MockMemoryRepo;
/// let entry = MemoryEntry::new(
///     MemoryId::new(1),
///     CharacterId::new(5),
///     "소연이 시장에서 수상한 사내를 보았다".to_string(),
///     7.0,
///     MemoryType::Observation,
///     GameTime::new(1200, 3, 15),
///     vec!["시장".to_string()],
/// );
/// assert!(repo.save(entry).is_ok());
/// assert_eq!(repo.count(CharacterId::new(5)), 0); // Mock은 실제 저장 안 함
/// ```
pub trait MemoryRepository: Send + Sync {
    /// 기억을 저장한다.
    ///
    /// # Arguments
    /// * `entry` - 저장할 기억. 소유권 이전(move)으로 전달.
    ///
    /// # Returns
    /// * `Ok(())` - 저장 성공.
    /// * `Err(PortError)` - 저장 실패 (중복 ID, 저장소 오류 등).
    fn save(&mut self, entry: MemoryEntry) -> Result<(), PortError>;

    /// 최근 기억 N개를 시간 역순으로 조회한다. (구조적 쿼리)
    ///
    /// game_time 기준 내림차순. 키워드/의미 무관하게 최근 것만.
    /// NPC 대화 시 "최근 맥락"을 프롬프트에 삽입할 때 사용.
    ///
    /// # Arguments
    /// * `character_id` - 조회할 NPC.
    /// * `n` - 최대 반환 개수.
    fn find_recent(&self, character_id: CharacterId, n: usize) -> Vec<MemoryEntry>;

    /// 의미 기반 검색. (시맨틱 쿼리)
    ///
    /// query와 관련된 기억을 relevance_score와 함께 반환.
    /// 구현체에 따라 검색 방식이 달라진다:
    ///   - InMemory: keywords 매칭 (0.0 or 1.0)
    ///   - LanceDb: 벡터 코사인 유사도 (0.0~1.0)
    ///
    /// Caller(ConversationService)는 반환된 ScoredMemory에
    /// retrieval_score()를 적용하여 최종 순위를 결정한다.
    ///
    /// # Arguments
    /// * `character_id` - 검색 대상 NPC.
    /// * `query` - 검색어 (자연어).
    /// * `top_k` - 최대 반환 개수.
    fn search(&self, character_id: CharacterId, query: &str, top_k: usize)
        -> Vec<ScoredMemory>;

    /// ID로 단일 기억을 조회한다.
    ///
    /// update_importance 서비스에서 old_importance 확인 등에 사용.
    ///
    /// # Returns
    /// * `Some(entry)` — 해당 ID의 기억이 존재.
    /// * `None` — 해당 ID의 기억이 없음.
    fn find_by_id(&self, memory_id: MemoryId) -> Option<MemoryEntry>;

    /// 특정 NPC의 총 기억 수를 반환한다.
    ///
    /// 모니터링/디버깅 및 "기억 용량" 게임 메카닉에 사용.
    fn count(&self, character_id: CharacterId) -> usize;

    /// 기억의 중요도를 재평가한다.
    ///
    /// Tier 2 일상 성찰에서 사용. 성찰 과정에서
    /// 과거 기억의 중요도를 올리거나 내릴 수 있다.
    /// 예: "그때 스쳐 지나간 대화가 사실 중요한 단서였다" → 3.0 → 8.0
    ///
    /// # Arguments
    /// * `memory_id` - 재평가할 기억의 ID.
    /// * `new_importance` - 새 중요도 (1.0~10.0).
    fn update_importance(
        &mut self,
        memory_id: MemoryId,
        new_importance: f32,
    ) -> Result<(), PortError>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryType;
    use crate::shared::time::GameTime;

    /// 테스트용 Mock — trait 구현 가능 여부 검증.
    struct TestMockRepo {
        stored: Vec<MemoryEntry>,
    }

    impl TestMockRepo {
        fn new() -> Self {
            Self {
                stored: Vec::new(),
            }
        }
    }

    impl MemoryRepository for TestMockRepo {
        fn save(&mut self, entry: MemoryEntry) -> Result<(), PortError> {
            self.stored.push(entry);
            Ok(())
        }

        fn find_recent(&self, character_id: CharacterId, n: usize) -> Vec<MemoryEntry> {
            self.stored
                .iter()
                .filter(|e| e.character_id() == character_id)
                .rev()
                .take(n)
                .cloned()
                .collect()
        }

        fn search(
            &self,
            character_id: CharacterId,
            query: &str,
            top_k: usize,
        ) -> Vec<ScoredMemory> {
            self.stored
                .iter()
                .filter(|e| e.character_id() == character_id)
                .filter(|e| {
                    e.keywords().iter().any(|k| k.contains(query))
                        || e.content().contains(query)
                })
                .take(top_k)
                .map(|e| ScoredMemory::new(e.clone(), 1.0))
                .collect()
        }

        fn find_by_id(&self, memory_id: MemoryId) -> Option<MemoryEntry> {
            self.stored.iter().find(|e| e.id() == memory_id).cloned()
        }

        fn count(&self, character_id: CharacterId) -> usize {
            self.stored
                .iter()
                .filter(|e| e.character_id() == character_id)
                .count()
        }

        fn update_importance(
            &mut self,
            memory_id: MemoryId,
            new_importance: f32,
        ) -> Result<(), PortError> {
            if let Some(entry) = self.stored.iter_mut().find(|e| e.id() == memory_id) {
                entry.update_importance(new_importance);
                Ok(())
            } else {
                Err(PortError::not_found(format!("Memory {} not found", memory_id)))
            }
        }
    }

    fn make_entry(id: u64, char_id: u64, content: &str, importance: f32) -> MemoryEntry {
        MemoryEntry::new(
            MemoryId::new(id),
            CharacterId::new(char_id),
            content.to_string(),
            importance,
            MemoryType::Observation,
            GameTime::new(1200, 3, id as u32), // day = id for simple ordering
            content.split_whitespace().map(String::from).collect(),
        )
    }

    #[test]
    fn mock_implements_trait() {
        let mut repo = TestMockRepo::new();
        let entry = make_entry(1, 5, "시장에서 수상한 사내", 7.0);
        assert!(repo.save(entry).is_ok());
        assert_eq!(repo.count(CharacterId::new(5)), 1);
    }

    #[test]
    fn find_recent_returns_latest_first() {
        let mut repo = TestMockRepo::new();
        repo.save(make_entry(1, 5, "첫번째 기억", 3.0)).unwrap();
        repo.save(make_entry(2, 5, "두번째 기억", 5.0)).unwrap();
        repo.save(make_entry(3, 5, "세번째 기억", 7.0)).unwrap();

        let recent = repo.find_recent(CharacterId::new(5), 2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id(), MemoryId::new(3)); // 최신
        assert_eq!(recent[1].id(), MemoryId::new(2));
    }

    #[test]
    fn find_recent_filters_by_character() {
        let mut repo = TestMockRepo::new();
        repo.save(make_entry(1, 5, "소연의 기억", 5.0)).unwrap();
        repo.save(make_entry(2, 7, "명경의 기억", 5.0)).unwrap();

        let soyeon = repo.find_recent(CharacterId::new(5), 10);
        assert_eq!(soyeon.len(), 1);
        assert_eq!(soyeon[0].character_id(), CharacterId::new(5));
    }

    #[test]
    fn search_returns_scored_memories() {
        let mut repo = TestMockRepo::new();
        repo.save(make_entry(1, 5, "시장에서 수상한 사내를 보았다", 7.0))
            .unwrap();
        repo.save(make_entry(2, 5, "오늘 만두를 먹었다", 2.0))
            .unwrap();

        let results = repo.search(CharacterId::new(5), "수상한", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entry.id(), MemoryId::new(1));
        assert_eq!(results[0].relevance_score, 1.0);
    }

    #[test]
    fn search_no_match_returns_empty() {
        let mut repo = TestMockRepo::new();
        repo.save(make_entry(1, 5, "평범한 하루", 2.0)).unwrap();

        let results = repo.search(CharacterId::new(5), "혈교", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn find_by_id_existing() {
        let mut repo = TestMockRepo::new();
        repo.save(make_entry(1, 5, "시장에서 수상한 사내", 7.0)).unwrap();

        let found = repo.find_by_id(MemoryId::new(1));
        assert!(found.is_some());
        assert_eq!(found.unwrap().importance(), 7.0);
    }

    #[test]
    fn find_by_id_not_found() {
        let repo = TestMockRepo::new();
        assert!(repo.find_by_id(MemoryId::new(999)).is_none());
    }

    #[test]
    fn update_importance_success() {
        let mut repo = TestMockRepo::new();
        repo.save(make_entry(1, 5, "중요한 기억", 3.0)).unwrap();

        assert!(repo.update_importance(MemoryId::new(1), 9.0).is_ok());

        let recent = repo.find_recent(CharacterId::new(5), 1);
        assert_eq!(recent[0].importance(), 9.0);
    }

    #[test]
    fn update_importance_not_found() {
        let mut repo = TestMockRepo::new();
        let result = repo.update_importance(MemoryId::new(999), 5.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().message().contains("not found"));
    }

    #[test]
    fn count_empty() {
        let repo = TestMockRepo::new();
        assert_eq!(repo.count(CharacterId::new(5)), 0);
    }

    #[test]
    fn trait_object_works() {
        // dyn MemoryRepository 로 사용 가능한지 확인 (Bevy Resource 대비)
        let repo: Box<dyn MemoryRepository> = Box::new(TestMockRepo::new());
        assert_eq!(repo.count(CharacterId::new(5)), 0);
    }

    #[test]
    fn arc_trait_object_works() {
        // Arc<dyn MemoryRepository>는 &self 메서드만 가능.
        // &mut self 메서드가 있어서 Arc로 직접 호출은 안 되지만,
        // Arc<RwLock<dyn MemoryRepository>> 패턴은 가능.
        // 여기서는 Send + Sync 바운드만 검증.
        use std::sync::{Arc, RwLock};

        let repo: Arc<RwLock<dyn MemoryRepository>> =
            Arc::new(RwLock::new(TestMockRepo::new()));

        {
            let reader = repo.read().unwrap();
            assert_eq!(reader.count(CharacterId::new(5)), 0);
        }

        {
            let mut writer = repo.write().unwrap();
            let entry = make_entry(1, 5, "테스트 기억", 5.0);
            assert!(writer.save(entry).is_ok());
        }

        {
            let reader = repo.read().unwrap();
            assert_eq!(reader.count(CharacterId::new(5)), 1);
        }
    }
}
