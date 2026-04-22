//! 테스트 전용 InMemoryRumorStore — feature 의존성 없는 참조 구현.
//!
//! `RumorStore` 트레이트의 결정적 참조 구현으로서 통합 테스트에서만 사용한다.
//! 프로덕션 경로에서는 `SqliteRumorStore`(embed feature)가 기본 구현.

use npc_mind::domain::rumor::{ReachPolicy, Rumor, RumorStatus};
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

    /// 프로덕션 `SqliteRumorStore`가 `status IN ('active','fading')` + reach 필터를
    /// 적용하는 것을 반영해 최소한 `status` 필터는 여기서도 적용한다. reach 필터는
    /// 단순화를 위해 생략 — 호출자가 reach를 직접 검증해야 한다 (Step C3 사후 리뷰 M7).
    fn find_active_in_reach(&self, _reach: &ReachPolicy) -> Result<Vec<Rumor>, MemoryError> {
        Ok(self
            .inner
            .read()
            .unwrap()
            .iter()
            .filter(|r| matches!(r.status(), RumorStatus::Active | RumorStatus::Fading))
            .cloned()
            .collect())
    }

    fn list_all(&self) -> Result<Vec<Rumor>, MemoryError> {
        Ok(self.inner.read().unwrap().iter().cloned().collect())
    }
}
