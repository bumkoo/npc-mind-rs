//! `WorldRepository` 포트 — Phase 1엔 groups 메서드만.
//!
//! sync trait — `LoreStore`/`MemoryStore`/`RumorStore`와 동일 패턴. SQLite·인메모리
//! 모두 sync 동작이며 호출자가 필요 시 `tokio::task::spawn_blocking`으로 감싼다.

use crate::domain::world::{Group, GroupFilter, GroupId, WorldError};

pub trait WorldRepository: Send + Sync {
    /// 필터 조건으로 그룹 목록 조회. 결과는 id 오름차순.
    fn list_groups(&self, filter: GroupFilter) -> Result<Vec<Group>, WorldError>;

    /// id로 단일 그룹 조회. 없으면 Ok(None).
    fn get_group(&self, id: &GroupId) -> Result<Option<Group>, WorldError>;

    /// FTS5 trigram 매치 — name + aliases + summary + body 결합 검색.
    fn search_groups(&self, query: &str, top_k: u32) -> Result<Vec<Group>, WorldError>;

    /// upsert 단건 — id 중복은 덮어쓴다. project_id는 `groups.project_id` 컬럼에 저장.
    fn upsert_group(&self, project_id: &str, group: &Group) -> Result<(), WorldError>;

    /// 카운트 — 진행률·상태 확인용.
    fn count_groups(&self, project_id: Option<&str>) -> Result<u64, WorldError>;
}
