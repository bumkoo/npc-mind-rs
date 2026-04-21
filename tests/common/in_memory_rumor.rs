//! 테스트 전용 InMemoryRumorStore — feature 의존성 없는 참조 구현.
//!
//! `RumorStore` 트레이트의 결정적 참조 구현으로서 통합 테스트에서만 사용한다.
//! 프로덕션 경로에서는 `SqliteRumorStore`(embed feature)가 기본 구현.

use npc_mind::domain::rumor::{ReachPolicy, Rumor};
use npc_mind::ports::{MemoryError, RumorStore};
use std::sync::RwLock;

pub struct InMemoryRumorStore {
    inner: RwLock<Vec<Rumor>>,
}

impl InMemoryRumorStore {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Vec::new()),
        }
    }
}

impl Default for InMemoryRumorStore {
    fn default() -> Self {
        Self::new()
    }
}

impl RumorStore for InMemoryRumorStore {
    fn save(&self, rumor: &Rumor) -> Result<(), MemoryError> {
        let mut g = self.inner.write().unwrap();
        if let Some(pos) = g.iter().position(|r| r.id == rumor.id) {
            g[pos] = rumor.clone();
        } else {
            g.push(rumor.clone());
        }
        Ok(())
    }

    fn load(&self, id: &str) -> Result<Option<Rumor>, MemoryError> {
        Ok(self
            .inner
            .read()
            .unwrap()
            .iter()
            .find(|r| r.id == id)
            .cloned())
    }

    fn find_by_topic(&self, topic: &str) -> Result<Vec<Rumor>, MemoryError> {
        Ok(self
            .inner
            .read()
            .unwrap()
            .iter()
            .filter(|r| r.topic.as_deref() == Some(topic))
            .cloned()
            .collect())
    }

    fn find_active_in_reach(&self, _reach: &ReachPolicy) -> Result<Vec<Rumor>, MemoryError> {
        Ok(self.inner.read().unwrap().clone())
    }
}
