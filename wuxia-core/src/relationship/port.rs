// wuxia-core/src/relationship/port.rs
//
// Relationship Repository Port — 헥사고날 아키텍처의 출력 포트.

use crate::shared::id::{CharacterId, RelationshipId};
use crate::shared::port_error::PortError;

use super::types::Relationship;
use crate::relationship::RelationshipChronicle;

/// 관계 저장소 포트 (헥사고날 아키텍처).
pub trait RelationshipRepository: Send + Sync {
    /// 변경 사항을 영구 저장소에 반영한다 (Flush).
    fn flush(&mut self) -> Result<(), PortError>;

    /// 관계를 저장하거나 갱신한다.
    fn save(&mut self, relationship: Relationship) -> Result<(), PortError>;

    /// ID로 관계를 조회한다.
    fn find_by_id(&self, id: RelationshipId) -> Option<Relationship>;

    /// source→target 방향의 관계를 조회한다.
    fn find_between(
        &self,
        source: CharacterId,
        target: CharacterId,
    ) -> Option<Relationship>;

    /// 한 캐릭터가 관련된 모든 관계를 반환한다.
    fn find_all_for(&self, character_id: CharacterId) -> Vec<Relationship>;

    /// 관계를 삭제한다.
    fn delete(&mut self, id: RelationshipId) -> Result<(), PortError>;
}

/// 관계 연대기(Chronicle) 저장소 포트. [v4]
pub trait ChronicleRepository: Send + Sync {
    /// 기록을 즉시 파일/DB에 반영한다 (Flush).
    fn flush(&mut self) -> Result<(), PortError>;

    /// 새 기록을 추가한다. 반환값은 생성된 기록의 seq 번호.
    fn append(&mut self, chronicle: RelationshipChronicle) -> Result<u64, PortError>;

    /// 두 캐릭터 사이의 모든 기록을 조회한다.
    fn find_by_pair(
        &self,
        source: CharacterId,
        target: CharacterId,
    ) -> Result<Vec<RelationshipChronicle>, PortError>;

    /// 특정 세션의 모든 기록을 조회한다.
    fn find_by_session(&self, session_id: &str) -> Result<Vec<RelationshipChronicle>, PortError>;

    /// 특정 변경 타입의 모든 기록을 조회한다.
    fn find_by_change_type(
        &self,
        source: CharacterId,
        target: CharacterId,
        change_type: &str,
    ) -> Result<Vec<RelationshipChronicle>, PortError>;

    /// 전체 기록 수를 반환한다.
    fn count(&self) -> Result<u64, PortError>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relationship::RelationshipType;
    use std::collections::HashMap;

    struct TestRelRepo {
        store: HashMap<RelationshipId, Relationship>,
    }

    impl TestRelRepo {
        fn new() -> Self {
            Self { store: HashMap::new() }
        }
    }

    impl RelationshipRepository for TestRelRepo {
        fn flush(&mut self) -> Result<(), PortError> { Ok(()) }

        fn save(&mut self, relationship: Relationship) -> Result<(), PortError> {
            self.store.insert(relationship.id(), relationship);
            Ok(())
        }

        fn find_by_id(&self, id: RelationshipId) -> Option<Relationship> {
            self.store.get(&id).cloned()
        }

        fn find_between(&self, source: CharacterId, target: CharacterId) -> Option<Relationship> {
            self.store.values().find(|r| r.source() == source && r.target() == target).cloned()
        }

        fn find_all_for(&self, character_id: CharacterId) -> Vec<Relationship> {
            self.store.values().filter(|r| r.source() == character_id || r.target() == character_id).cloned().collect()
        }

        fn delete(&mut self, id: RelationshipId) -> Result<(), PortError> {
            self.store.remove(&id).map(|_| ()).ok_or_else(|| PortError::not_found(format!("Relationship {} not found", id)))
        }
    }

    #[test]
    fn save_and_find_by_id() {
        let mut repo = TestRelRepo::new();
        let rel = crate::test_fixtures::make_relationship(1, 1, 2);
        repo.save(rel).unwrap();
        assert!(repo.find_by_id(RelationshipId::new(1)).is_some());
    }
}
