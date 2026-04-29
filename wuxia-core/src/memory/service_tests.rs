use super::*;
use crate::memory::port::MemoryRepository;
use crate::memory::types::{MemoryEntry, MemoryType, ScoredMemory};
use crate::shared::id::{CharacterId, MemoryId};
use crate::shared::port_error::PortError;
use crate::shared::time::GameTime;

// ── Mock Repository ──────────────────────────────────────────────────

struct MockRepo {
    entries: Vec<MemoryEntry>,
}

impl MockRepo {
    fn new() -> Self {
        Self { entries: vec![] }
    }

    fn with_entries(entries: Vec<MemoryEntry>) -> Self {
        Self { entries }
    }
}

impl MemoryRepository for MockRepo {
    fn save(&mut self, entry: MemoryEntry) -> Result<(), PortError> {
        if self.entries.iter().any(|e| e.id() == entry.id()) {
            return Err(PortError::conflict(format!("Duplicate memory ID: {}", entry.id())));
        }
        self.entries.push(entry);
        Ok(())
    }

    fn find_recent(&self, character_id: CharacterId, n: usize) -> Vec<MemoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.character_id() == character_id)
            .rev()
            .take(n)
            .cloned()
            .collect()
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

    fn count(&self, character_id: CharacterId) -> usize {
        self.entries
            .iter()
            .filter(|e| e.character_id() == character_id)
            .count()
    }

    fn update_importance(
        &mut self,
        memory_id: MemoryId,
        new_importance: f32,
    ) -> Result<(), PortError> {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id() == memory_id) {
            entry.update_importance(new_importance);
            Ok(())
        } else {
            Err(PortError::not_found(format!("Memory {} not found", memory_id)))
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn soyeon_id() -> CharacterId {
    CharacterId::new(5)
}

fn make_observation(id: u64, content: &str, importance: f32, day: u32) -> MemoryEntry {
    MemoryEntry::new(
        MemoryId::new(id),
        soyeon_id(),
        content.to_string(),
        importance,
        MemoryType::Observation,
        GameTime::new(1200, 3, day),
        content.split_whitespace().map(String::from).collect(),
    )
}

fn make_reflection(
    id: u64,
    content: &str,
    importance: f32,
    day: u32,
    source_ids: Vec<u64>,
) -> MemoryEntry {
    MemoryEntry::with_sources(
        MemoryId::new(id),
        soyeon_id(),
        content.to_string(),
        importance,
        MemoryType::Reflection,
        GameTime::new(1200, 3, day),
        content.split_whitespace().map(String::from).collect(),
        source_ids.into_iter().map(MemoryId::new).collect(),
        Some(2),
    )
}

fn make_plan(
    id: u64,
    content: &str,
    importance: f32,
    day: u32,
    source_ids: Vec<u64>,
) -> MemoryEntry {
    MemoryEntry::with_sources(
        MemoryId::new(id),
        soyeon_id(),
        content.to_string(),
        importance,
        MemoryType::Plan,
        GameTime::new(1200, 3, day),
        content.split_whitespace().map(String::from).collect(),
        source_ids.into_iter().map(MemoryId::new).collect(),
        None,
    )
}

// =====================================================================
// store_memory
// =====================================================================

#[test]
fn store_observation_success() {
    let mut repo = MockRepo::new();
    let entry = make_observation(1, "시장에서 수상한 사내를 보았다", 7.0, 15);

    let events = store_memory(&mut repo, entry).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(repo.count(soyeon_id()), 1);

    if let MemoryEvent::MemoryStored {
        memory_id,
        character_id,
        memory_type,
        importance,
    } = &events[0]
    {
        assert_eq!(*memory_id, MemoryId::new(1));
        assert_eq!(*character_id, soyeon_id());
        assert_eq!(*memory_type, MemoryType::Observation);
        assert_eq!(*importance, 7.0);
    } else {
        panic!("Expected MemoryStored event");
    }
}

#[test]
fn store_reflection_success() {
    let mut repo = MockRepo::new();
    let entry = make_reflection(10, "나는 환멸을 느끼고 있다", 9.0, 20, vec![1, 2, 3]);

    let events = store_memory(&mut repo, entry).unwrap();
    assert_eq!(events.len(), 1);
    if let MemoryEvent::MemoryStored { memory_type, .. } = &events[0] {
        assert_eq!(*memory_type, MemoryType::Reflection);
    }
}

#[test]
fn store_plan_success() {
    let mut repo = MockRepo::new();
    let entry = make_plan(20, "내일 밤 몰래 떠나겠다", 6.0, 20, vec![10]);

    let events = store_memory(&mut repo, entry).unwrap();
    assert_eq!(events.len(), 1);
    if let MemoryEvent::MemoryStored { memory_type, .. } = &events[0] {
        assert_eq!(*memory_type, MemoryType::Plan);
    }
}

#[test]
fn store_empty_content_fails() {
    let mut repo = MockRepo::new();
    let entry = MemoryEntry::new(
        MemoryId::new(1),
        soyeon_id(),
        "".to_string(),
        5.0,
        MemoryType::Observation,
        GameTime::new(1200, 1, 1),
        vec![],
    );

    let result = store_memory(&mut repo, entry);
    assert!(result.is_err());
    assert!(result.unwrap_err().message().contains("empty"));
}

#[test]
fn store_whitespace_content_fails() {
    let mut repo = MockRepo::new();
    let entry = MemoryEntry::new(
        MemoryId::new(1),
        soyeon_id(),
        "   \t\n  ".to_string(),
        5.0,
        MemoryType::Observation,
        GameTime::new(1200, 1, 1),
        vec![],
    );

    let result = store_memory(&mut repo, entry);
    assert!(result.is_err());
}

#[test]
fn store_reflection_without_sources_fails() {
    let mut repo = MockRepo::new();
    // Reflection인데 source_ids가 빈 목록 (new()로 생성 → source_ids = [])
    let entry = MemoryEntry::new(
        MemoryId::new(10),
        soyeon_id(),
        "성찰이지만 근거 없음".to_string(),
        8.0,
        MemoryType::Reflection,
        GameTime::new(1200, 3, 20),
        vec!["성찰".to_string()],
    );

    let result = store_memory(&mut repo, entry);
    assert!(result.is_err());
    assert!(result.unwrap_err().message().contains("source_id"));
}

#[test]
fn store_plan_without_sources_fails() {
    let mut repo = MockRepo::new();
    let entry = MemoryEntry::new(
        MemoryId::new(20),
        soyeon_id(),
        "계획이지만 근거 없음".to_string(),
        6.0,
        MemoryType::Plan,
        GameTime::new(1200, 3, 20),
        vec!["계획".to_string()],
    );

    let result = store_memory(&mut repo, entry);
    assert!(result.is_err());
    assert!(result.unwrap_err().message().contains("source_id"));
}

#[test]
fn store_observation_without_sources_ok() {
    // Observation은 source_ids가 비어있어도 OK
    let mut repo = MockRepo::new();
    let entry = make_observation(1, "평범한 관찰", 3.0, 1);

    let result = store_memory(&mut repo, entry);
    assert!(result.is_ok());
}

#[test]
fn store_duplicate_id_fails() {
    let mut repo = MockRepo::new();
    let entry1 = make_observation(1, "첫 기억", 5.0, 1);
    let entry2 = make_observation(1, "중복 ID 기억", 6.0, 2);

    store_memory(&mut repo, entry1).unwrap();
    let result = store_memory(&mut repo, entry2);
    assert!(result.is_err());
    assert!(result.unwrap_err().message().contains("Duplicate"));
}

#[test]
fn store_importance_boundary_values() {
    // importance는 MemoryEntry::new()에서 이미 1.0~10.0 clamp됨
    let mut repo = MockRepo::new();

    let entry_min = make_observation(1, "최소 중요도", 1.0, 1);
    let events = store_memory(&mut repo, entry_min).unwrap();
    if let MemoryEvent::MemoryStored { importance, .. } = &events[0] {
        assert_eq!(*importance, 1.0);
    }

    let entry_max = make_observation(2, "최대 중요도", 10.0, 2);
    let events = store_memory(&mut repo, entry_max).unwrap();
    if let MemoryEvent::MemoryStored { importance, .. } = &events[0] {
        assert_eq!(*importance, 10.0);
    }
}

#[test]
fn store_event_name_is_memory_stored() {
    let mut repo = MockRepo::new();
    let entry = make_observation(1, "이벤트 이름 테스트", 5.0, 1);
    let events = store_memory(&mut repo, entry).unwrap();
    assert_eq!(events[0].name(), "MemoryStored");
}

// =====================================================================
// recall_and_emit
// =====================================================================

#[test]
fn recall_empty_repo_no_events() {
    let repo = MockRepo::new();
    let (ranked, events) = recall_and_emit(
        &repo,
        soyeon_id(),
        "혈교",
        GameTime::new(1200, 3, 15),
        &RetrievalWeights::default(),
        None,
        10,
        5,
    );
    assert!(ranked.is_empty());
    assert!(events.is_empty()); // no-op
}

#[test]
fn recall_no_match_no_events() {
    let repo = MockRepo::with_entries(vec![make_observation(
        1,
        "만두를 먹었다",
        2.0,
        14,
    )]);
    let (ranked, events) = recall_and_emit(
        &repo,
        soyeon_id(),
        "혈교",
        GameTime::new(1200, 3, 15),
        &RetrievalWeights::default(),
        None,
        10,
        5,
    );
    assert!(ranked.is_empty());
    assert!(events.is_empty());
}

#[test]
fn recall_with_results_emits_event() {
    let repo = MockRepo::with_entries(vec![
        make_observation(1, "혈교 무인이 수상하다", 7.0, 10),
        make_observation(2, "혈교가 조고를 보냈다", 9.0, 14),
    ]);
    let (ranked, events) = recall_and_emit(
        &repo,
        soyeon_id(),
        "혈교",
        GameTime::new(1200, 3, 15),
        &RetrievalWeights::default(),
        None,
        10,
        5,
    );

    // 기억 2개 반환
    assert_eq!(ranked.len(), 2);
    // 이벤트 1개 (요약)
    assert_eq!(events.len(), 1);

    if let MemoryEvent::MemoryRecalled {
        character_id,
        recalled_ids,
    } = &events[0]
    {
        assert_eq!(*character_id, soyeon_id());
        assert_eq!(recalled_ids.len(), 2);
        // [0] = 최고 점수 기억 = ranked[0]
        assert_eq!(recalled_ids[0], ranked[0].entry.id());
    } else {
        panic!("Expected MemoryRecalled event");
    }
}

#[test]
fn recall_ids_preserve_rank_order() {
    let repo = MockRepo::with_entries(vec![
        make_observation(1, "혈교 기억1", 5.0, 10),
        make_observation(2, "혈교 기억2", 6.0, 12),
        make_observation(3, "혈교 기억3", 9.0, 14),
    ]);
    let (ranked, events) = recall_and_emit(
        &repo,
        soyeon_id(),
        "혈교",
        GameTime::new(1200, 3, 15),
        &RetrievalWeights::default(),
        None,
        10,
        5,
    );

    if let MemoryEvent::MemoryRecalled { recalled_ids, .. } = &events[0] {
        // recalled_ids 순서 == ranked 순서
        for (i, ranked_mem) in ranked.iter().enumerate() {
            assert_eq!(recalled_ids[i], ranked_mem.entry.id());
        }
    }
}

#[test]
fn recall_respects_rank_top_k() {
    let repo = MockRepo::with_entries(vec![
        make_observation(1, "혈교 기억1", 5.0, 10),
        make_observation(2, "혈교 기억2", 6.0, 12),
        make_observation(3, "혈교 기억3", 9.0, 14),
    ]);
    let (ranked, events) = recall_and_emit(
        &repo,
        soyeon_id(),
        "혈교",
        GameTime::new(1200, 3, 15),
        &RetrievalWeights::default(),
        None,
        10,
        2, // rank_top_k=2
    );

    assert_eq!(ranked.len(), 2);
    if let MemoryEvent::MemoryRecalled { recalled_ids, .. } = &events[0] {
        assert_eq!(recalled_ids.len(), 2);
    }
}

#[test]
fn recall_event_name_is_memory_recalled() {
    let repo = MockRepo::with_entries(vec![make_observation(
        1,
        "혈교 기억",
        5.0,
        10,
    )]);
    let (_, events) = recall_and_emit(
        &repo,
        soyeon_id(),
        "혈교",
        GameTime::new(1200, 3, 15),
        &RetrievalWeights::default(),
        None,
        10,
        5,
    );
    assert_eq!(events[0].name(), "MemoryRecalled");
}

// =====================================================================
// update_importance
// =====================================================================

#[test]
fn update_importance_success() {
    let mut repo = MockRepo::with_entries(vec![make_observation(
        1,
        "중요한 기억",
        3.0,
        15,
    )]);

    let events = update_importance(&mut repo, MemoryId::new(1), 8.0).unwrap();
    assert_eq!(events.len(), 1);

    if let MemoryEvent::ImportanceUpdated {
        memory_id,
        character_id,
        old_importance,
        new_importance,
    } = &events[0]
    {
        assert_eq!(*memory_id, MemoryId::new(1));
        assert_eq!(*character_id, soyeon_id());
        assert_eq!(*old_importance, 3.0);
        assert_eq!(*new_importance, 8.0);
    } else {
        panic!("Expected ImportanceUpdated event");
    }
}

#[test]
fn update_importance_not_found() {
    let mut repo = MockRepo::new();
    let result = update_importance(&mut repo, MemoryId::new(999), 5.0);
    assert!(result.is_err());
    assert!(result.unwrap_err().message().contains("not found"));
}

#[test]
fn update_importance_same_value_noop() {
    let mut repo = MockRepo::with_entries(vec![make_observation(
        1,
        "기억",
        5.0,
        15,
    )]);

    // 동일 값 → 빈 이벤트 (no-op)
    let events = update_importance(&mut repo, MemoryId::new(1), 5.0).unwrap();
    assert!(events.is_empty());
}

#[test]
fn update_importance_clamped_high() {
    let mut repo = MockRepo::with_entries(vec![make_observation(
        1,
        "기억",
        5.0,
        15,
    )]);

    // 11.0 → clamp → 10.0
    let events = update_importance(&mut repo, MemoryId::new(1), 11.0).unwrap();
    assert_eq!(events.len(), 1);
    if let MemoryEvent::ImportanceUpdated { new_importance, .. } = &events[0] {
        assert_eq!(*new_importance, 10.0);
    }
}

#[test]
fn update_importance_clamped_low() {
    let mut repo = MockRepo::with_entries(vec![make_observation(
        1,
        "기억",
        5.0,
        15,
    )]);

    // 0.0 → clamp → 1.0
    let events = update_importance(&mut repo, MemoryId::new(1), 0.0).unwrap();
    assert_eq!(events.len(), 1);
    if let MemoryEvent::ImportanceUpdated { new_importance, .. } = &events[0] {
        assert_eq!(*new_importance, 1.0);
    }
}

#[test]
fn update_importance_upward() {
    // 상향 조정: 사소한 대화가 중요한 단서였음을 깨달음
    let mut repo = MockRepo::with_entries(vec![make_observation(
        1,
        "스쳐 지나간 대화",
        3.0,
        10,
    )]);

    let events = update_importance(&mut repo, MemoryId::new(1), 9.0).unwrap();
    if let MemoryEvent::ImportanceUpdated {
        old_importance,
        new_importance,
        ..
    } = &events[0]
    {
        assert!(*new_importance > *old_importance);
    }
}

#[test]
fn update_importance_downward() {
    // 하향 조정: 관계 단절로 상대방 관련 기억이 덜 중요해짐
    let mut repo = MockRepo::with_entries(vec![make_observation(
        1,
        "사형과의 대화",
        8.0,
        10,
    )]);

    let events = update_importance(&mut repo, MemoryId::new(1), 3.0).unwrap();
    if let MemoryEvent::ImportanceUpdated {
        old_importance,
        new_importance,
        ..
    } = &events[0]
    {
        assert!(*new_importance < *old_importance);
    }
}

#[test]
fn update_importance_event_name() {
    let mut repo = MockRepo::with_entries(vec![make_observation(
        1,
        "기억",
        5.0,
        15,
    )]);
    let events = update_importance(&mut repo, MemoryId::new(1), 8.0).unwrap();
    assert_eq!(events[0].name(), "MemoryImportanceUpdated");
}

#[test]
fn update_importance_repo_reflects_change() {
    let mut repo = MockRepo::with_entries(vec![make_observation(
        1,
        "기억",
        3.0,
        15,
    )]);

    update_importance(&mut repo, MemoryId::new(1), 9.0).unwrap();

    // repo에서 직접 확인 — 실제로 변경됨
    let found = repo.find_by_id(MemoryId::new(1)).unwrap();
    assert_eq!(found.importance(), 9.0);
}
