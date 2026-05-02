//! `WorldRepository` 포트 — Phase 1 Group + Phase 2 Person.
//!
//! sync trait — `LoreStore`/`MemoryStore`/`RumorStore`와 동일 패턴. SQLite·인메모리
//! 모두 sync 동작이며 호출자가 필요 시 `tokio::task::spawn_blocking`으로 감싼다.

use crate::domain::world::{
    Atlas, AtlasFilter, AtlasId, Era, EraFilter, EraId, Event, EventFilter, EventId, Group,
    GroupFilter, GroupId, Person, PersonFilter, PersonId, Place, PlaceFilter, PlaceId, WorldError,
};

pub trait WorldRepository: Send + Sync {
    // ---------------------------------------------------------------------
    // Phase 1 — Group
    // ---------------------------------------------------------------------

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

    // ---------------------------------------------------------------------
    // Phase 2 — Person
    // ---------------------------------------------------------------------

    /// 필터 조건으로 인물 목록 조회. 결과는 id 오름차순.
    fn list_persons(&self, filter: PersonFilter) -> Result<Vec<Person>, WorldError>;

    /// id로 단일 인물 조회. 없으면 Ok(None).
    fn get_person(&self, id: &PersonId) -> Result<Option<Person>, WorldError>;

    /// FTS5 trigram 매치 — name + aliases + summary + body 결합 검색.
    fn search_persons(&self, query: &str, top_k: u32) -> Result<Vec<Person>, WorldError>;

    /// upsert 단건 — id 중복은 덮어쓴다. project_id는 `persons.project_id` 컬럼에 저장.
    fn upsert_person(&self, project_id: &str, person: &Person) -> Result<(), WorldError>;

    /// 카운트 — 진행률·상태 확인용.
    fn count_persons(&self, project_id: Option<&str>) -> Result<u64, WorldError>;

    // ---------------------------------------------------------------------
    // Phase 3 — Place
    // ---------------------------------------------------------------------

    /// 필터 조건으로 장소 목록 조회. 결과는 id 오름차순.
    fn list_places(&self, filter: PlaceFilter) -> Result<Vec<Place>, WorldError>;

    /// id로 단일 장소 조회. 없으면 Ok(None).
    fn get_place(&self, id: &PlaceId) -> Result<Option<Place>, WorldError>;

    /// 여러 id 일괄 조회. 결과는 입력 `ids` 순서대로 반환되며 결손 id는 누락
    /// (사일런트). Atlas의 view 메서드(`places_in` 등)가 N+1 round-trip을 피하도록
    /// 추가됨.
    ///
    /// **기본 구현**은 `get_place`를 ids만큼 반복한다 — 후방 호환. SqliteWorldStore
    /// 등 N round-trip을 피할 수 있는 backend는 단일 `IN(...)` 쿼리로 override.
    fn get_places_batch(&self, ids: &[PlaceId]) -> Result<Vec<Place>, WorldError> {
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(p) = self.get_place(id)? {
                out.push(p);
            }
        }
        Ok(out)
    }

    /// FTS5 trigram 매치 — name + aliases + summary + body 결합 검색.
    fn search_places(&self, query: &str, top_k: u32) -> Result<Vec<Place>, WorldError>;

    /// upsert 단건 — id 중복은 덮어쓴다.
    fn upsert_place(&self, project_id: &str, place: &Place) -> Result<(), WorldError>;

    /// 카운트 — 진행률·상태 확인용.
    fn count_places(&self, project_id: Option<&str>) -> Result<u64, WorldError>;

    // ---------------------------------------------------------------------
    // Phase 4 — Atlas (첫 관계 도메인)
    // ---------------------------------------------------------------------

    /// 필터 조건으로 atlas 목록 조회. 결과는 id 오름차순.
    fn list_atlases(&self, filter: AtlasFilter) -> Result<Vec<Atlas>, WorldError>;

    /// id로 단일 atlas 조회. 없으면 Ok(None). references·body_sections 전체 포함.
    fn get_atlas(&self, id: &AtlasId) -> Result<Option<Atlas>, WorldError>;

    /// FTS5 trigram 매치 — name + aliases + summary + body 결합 검색.
    fn search_atlases(&self, query: &str, top_k: u32) -> Result<Vec<Atlas>, WorldError>;

    /// upsert 단건 — id 중복은 덮어쓴다.
    ///
    /// **Source-of-truth**: `atlases.references_json`이 단일 권위. `place_atlas_refs`는
    /// 역방향 인덱스 전용으로, 같은 트랜잭션 안에서 delete-then-insert 패턴으로 동기화된다.
    /// 외부 도구가 둘 중 하나만 변경하면 silent drift 발생 가능 — 반드시 둘 다 갱신할 것.
    /// 자세한 SoT 계약은 `SqliteWorldStore::migrate_v4` 문서 참조.
    fn upsert_atlas(&self, project_id: &str, atlas: &Atlas) -> Result<(), WorldError>;

    /// 카운트 — 진행률·상태 확인용.
    fn count_atlases(&self, project_id: Option<&str>) -> Result<u64, WorldError>;

    // ---------------------------------------------------------------------
    // Phase 5a — Event (두 번째 인스턴스 도메인)
    // ---------------------------------------------------------------------

    /// 필터 조건으로 사건 목록 조회. 결과는 id 오름차순.
    ///
    /// `EventFilter::participants_*`는 `event_participants_refs` 인덱스를 활용해
    /// 특정 인물·그룹·장소가 관여한 사건만 추리는 데 사용. `year_relative_min/max`는
    /// `events.year_relative` 캐시 컬럼으로 시기별 정렬.
    fn list_events(&self, filter: EventFilter) -> Result<Vec<Event>, WorldError>;

    /// id로 단일 사건 조회. 없으면 Ok(None). participants·body_sections 전체 포함.
    fn get_event(&self, id: &EventId) -> Result<Option<Event>, WorldError>;

    /// FTS5 trigram 매치 — name + aliases + summary + body 결합 검색.
    fn search_events(&self, query: &str, top_k: u32) -> Result<Vec<Event>, WorldError>;

    /// upsert 단건 — id 중복은 덮어쓴다.
    ///
    /// **Source-of-truth**: `events.participants_json`이 단일 권위. `event_participants_refs`는
    /// 역방향 인덱스 전용이며 같은 트랜잭션 안에서 delete-then-insert로 동기화된다.
    /// 외부 도구가 둘 중 하나만 변경하면 silent drift 발생 가능 — Atlas와 동일한 SoT 계약.
    fn upsert_event(&self, project_id: &str, event: &Event) -> Result<(), WorldError>;

    /// 카운트 — 진행률·상태 확인용.
    fn count_events(&self, project_id: Option<&str>) -> Result<u64, WorldError>;

    // ---------------------------------------------------------------------
    // Phase 5b — Era (세 번째 인스턴스 도메인)
    // ---------------------------------------------------------------------

    /// 필터 조건으로 시대 목록 조회. 결과는 id 오름차순.
    ///
    /// `EraFilter::contains_year`는 boundary 정책 §3.3 적용 — `start_year_relative <= ?
    /// AND end_year_relative > ?` (start inclusive · end exclusive).
    fn list_eras(&self, filter: EraFilter) -> Result<Vec<Era>, WorldError>;

    /// id로 단일 시대 조회. 없으면 Ok(None). key_events·body_sections 전체 포함.
    fn get_era(&self, id: &EraId) -> Result<Option<Era>, WorldError>;

    /// FTS5 trigram 매치 — name + aliases + summary + body 결합 검색.
    fn search_eras(&self, query: &str, top_k: u32) -> Result<Vec<Era>, WorldError>;

    /// upsert 단건 — id 중복은 덮어쓴다.
    fn upsert_era(&self, project_id: &str, era: &Era) -> Result<(), WorldError>;

    /// 카운트 — 진행률·상태 확인용.
    fn count_eras(&self, project_id: Option<&str>) -> Result<u64, WorldError>;
}
