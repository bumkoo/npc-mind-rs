// wuxia-core/src/memory/service.rs
//
// Memory Domain Services — 기억 도메인 상태 변경 + 이벤트 생성.
//
// DDD에서 도메인 상태 변경은 반드시 도메인 이벤트를 수반해야 한다.
// Memory 도메인은 Aggregate Root가 없으므로(append-only 특성),
// 도메인 서비스 함수가 이벤트 생성 책임을 갖는다.
//
// 3개 서비스:
//   store_memory — 기억 저장 + MemoryStored 이벤트
//   recall_and_emit — 기억 회상 + MemoryRecalled 이벤트
//   update_importance — 중요도 변경 + ImportanceUpdated 이벤트
//
// 설계 원칙:
//   - 기존 recall_memories()는 순수 함수로 유지 (방안 B)
//   - No-op 규칙: 실제 변경이 없으면 빈 Vec 반환
//   - MemoryRepository 포트는 순수 I/O — 이벤트를 모른다
//   - 서비스가 포트 호출 후 이벤트를 생성하여 반환
//
// 참고 패턴: relationship/effect.rs의 apply_conversation_effect
//
// 무협 비유:
//   store_memory = 소연이 새로운 경험을 기억에 새기다 (+ "새 기억!" 소식)
//   recall_and_emit = 소연이 과거를 떠올리다 (+ "회상했다!" 소식)
//   update_importance = 소연이 기억의 의미를 재평가하다 (+ "재평가!" 소식)

use crate::memory::event::MemoryEvent;
use crate::memory::port::MemoryRepository;
use crate::memory::recall::recall_memories;
use crate::memory::retrieval::{EmotionalBias, RankedMemory, RetrievalWeights};
use crate::memory::types::{MemoryEntry, MemoryType};
use crate::shared::id::CharacterId;
use crate::shared::port_error::PortError;
use crate::shared::time::GameTime;

// ---------------------------------------------------------------------------
// store_memory — 기억 저장 + 이벤트
// ---------------------------------------------------------------------------

/// 기억을 저장하고 MemoryStored 이벤트를 반환한다.
///
/// 검증 규칙:
///   - content가 비어있지 않아야 한다 (공백만으로는 불가)
///   - Reflection/Plan 유형은 source_ids가 비어있지 않아야 한다
///
/// # Arguments
/// * `repo` — 기억 저장소 (Output Port).
/// * `entry` — 저장할 기억.
///
/// # Returns
/// * `Ok(Vec<MemoryEvent>)` — MemoryStored 이벤트 1개.
/// * `Err(PortError)` — 검증 실패 또는 저장소 오류.
///
/// # Example
/// ```text
///   소연이 "자유도시 시장에서 수상한 사내를 보았다"를 기억에 새긴다:
///
///   store_memory(repo, entry)
///     → 검증 → repo.save() → MemoryStored 이벤트
///     → 심리 도메인이 이 이벤트를 받아 성찰 트리거 판단
/// ```
pub fn store_memory(
    repo: &mut dyn MemoryRepository,
    entry: MemoryEntry,
) -> Result<Vec<MemoryEvent>, PortError> {
    // ① 입력 검증
    if entry.content().trim().is_empty() {
        return Err(PortError::conflict("Memory content must not be empty"));
    }

    match entry.memory_type() {
        MemoryType::Reflection | MemoryType::Plan => {
            if entry.source_ids().is_empty() {
                return Err(PortError::conflict(format!(
                    "{} memory must have at least one source_id",
                    entry.memory_type()
                )));
            }
        }
        MemoryType::Observation => {} // source_ids 불필요
    }

    // 이벤트 데이터 캡처 (소유권 이전 전)
    let memory_id = entry.id();
    let character_id = entry.character_id();
    let memory_type = entry.memory_type();
    let importance = entry.importance();

    // ② 저장 실행
    repo.save(entry)?;

    // ③ 이벤트 생성
    Ok(vec![MemoryEvent::MemoryStored {
        memory_id,
        character_id,
        memory_type,
        importance,
    }])
}

// ---------------------------------------------------------------------------
// recall_and_emit — 기억 회상 + 이벤트
// ---------------------------------------------------------------------------

/// 기억을 회상하고 MemoryRecalled 이벤트를 반환한다.
///
/// 기존 recall_memories()를 내부 호출하여 로직 중복을 방지한다.
/// 결과가 비어있으면 이벤트도 생성하지 않는다 (no-op 규칙).
///
/// # Returns
/// * `(Vec<RankedMemory>, Vec<MemoryEvent>)` — 순위화된 기억 + 이벤트.
///   - recalled_ids는 순위 순 정렬: [0] = 최고 점수 기억.
///
/// # Example
/// ```text
///   소연이 "혈교"라는 말을 듣고 관련 기억을 떠올린다:
///
///   recall_and_emit(repo, soyeon, "혈교", ...)
///     → recall_memories() → 4축 랭킹 → 상위 5개
///     → MemoryRecalled { recalled_ids: [id2, id1, id5, ...] }
///     → 심리 도메인이 자주 회상되는 기억의 중요도 상향 검토
/// ```
pub fn recall_and_emit(
    repo: &dyn MemoryRepository,
    character_id: CharacterId,
    query: &str,
    current_time: GameTime,
    weights: &RetrievalWeights,
    emotional_bias: Option<&EmotionalBias>,
    search_top_k: usize,
    rank_top_k: usize,
) -> (Vec<RankedMemory>, Vec<MemoryEvent>) {
    let ranked = recall_memories(
        repo,
        character_id,
        query,
        current_time,
        weights,
        emotional_bias,
        search_top_k,
        rank_top_k,
    );

    if ranked.is_empty() {
        return (vec![], vec![]);
    }

    let recalled_ids = ranked.iter().map(|r| r.entry.id()).collect();

    let event = MemoryEvent::MemoryRecalled {
        character_id,
        recalled_ids,
    };

    (ranked, vec![event])
}

// ---------------------------------------------------------------------------
// update_importance — 중요도 변경 + 이벤트
// ---------------------------------------------------------------------------

/// 기억의 중요도를 변경하고 ImportanceUpdated 이벤트를 반환한다.
///
/// 검증 규칙:
///   - 해당 memory_id가 존재해야 한다
///   - 새 중요도는 1.0~10.0으로 clamp된다
///   - 기존과 동일한 값이면 빈 Vec 반환 (no-op)
///
/// # Returns
/// * `Ok(Vec<MemoryEvent>)` — ImportanceUpdated 이벤트 0~1개.
/// * `Err(PortError)` — 기억이 존재하지 않거나 저장소 오류.
///
/// # Example
/// ```text
///   소연이 일상적으로 스쳐 지나간 대화(3.0)가 실은 중요한 단서였음을 깨달았다:
///
///   update_importance(repo, memory_id, 8.0)
///     → find_by_id → old=3.0, new=8.0 → repo.update_importance()
///     → ImportanceUpdated { old: 3.0, new: 8.0 }
/// ```
pub fn update_importance(
    repo: &mut dyn MemoryRepository,
    memory_id: crate::shared::id::MemoryId,
    new_importance: f32,
) -> Result<Vec<MemoryEvent>, PortError> {
    let clamped = new_importance.clamp(1.0, 10.0);

    // ① 기존 기억 조회 (old_importance 확인)
    let existing = repo.find_by_id(memory_id)
        .ok_or_else(|| PortError::not_found(format!("Memory {} not found", memory_id)))?;

    let old_importance = existing.importance();
    let character_id = existing.character_id();

    // ② No-op: 동일 값이면 이벤트 없이 반환
    if (old_importance - clamped).abs() < f32::EPSILON {
        return Ok(vec![]);
    }

    // ③ 중요도 갱신 실행
    repo.update_importance(memory_id, clamped)?;

    // ④ 이벤트 생성
    Ok(vec![MemoryEvent::ImportanceUpdated {
        memory_id,
        character_id,
        old_importance,
        new_importance: clamped,
    }])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
