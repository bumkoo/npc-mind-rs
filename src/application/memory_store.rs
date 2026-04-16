//! InMemoryMemoryStore — 개발/테스트용 기억 저장소
//!
//! Vec + brute-force cosine. Feature flag 없이 무조건 포함.

use crate::domain::memory::{MemoryEntry, MemoryResult};
use crate::ports::{MemoryError, MemoryStore};
use std::sync::RwLock;

/// 인메모리 기억 저장소
pub struct InMemoryMemoryStore {
    entries: RwLock<Vec<(MemoryEntry, Option<Vec<f32>>)>>,
}

impl InMemoryMemoryStore {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
        }
    }
}

impl Default for InMemoryMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStore for InMemoryMemoryStore {
    fn index(&self, entry: MemoryEntry, embedding: Option<Vec<f32>>) -> Result<(), MemoryError> {
        let mut entries = self.entries.write().unwrap();
        entries.push((entry, embedding));
        Ok(())
    }

    fn search_by_meaning(
        &self,
        query_embedding: &[f32],
        npc_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryResult>, MemoryError> {
        let entries = self.entries.read().unwrap();
        let mut scored: Vec<_> = entries
            .iter()
            .filter(|(e, emb)| {
                emb.is_some() && npc_id.map_or(true, |id| e.npc_id == id)
            })
            .map(|(entry, emb)| {
                let score = cosine_sim(query_embedding, emb.as_ref().unwrap());
                MemoryResult {
                    entry: entry.clone(),
                    relevance_score: score,
                }
            })
            .collect();

        scored.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        scored.truncate(limit);
        Ok(scored)
    }

    fn search_by_keyword(
        &self,
        keyword: &str,
        npc_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryResult>, MemoryError> {
        let entries = self.entries.read().unwrap();
        let keyword_lower = keyword.to_lowercase();
        let results: Vec<_> = entries
            .iter()
            .filter(|(e, _)| {
                e.content.to_lowercase().contains(&keyword_lower)
                    && npc_id.map_or(true, |id| e.npc_id == id)
            })
            .take(limit)
            .map(|(entry, _)| MemoryResult {
                entry: entry.clone(),
                relevance_score: 1.0,
            })
            .collect();
        Ok(results)
    }

    fn get_recent(
        &self,
        npc_id: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        let entries = self.entries.read().unwrap();
        let mut filtered: Vec<_> = entries
            .iter()
            .filter(|(e, _)| e.npc_id == npc_id)
            .map(|(e, _)| e.clone())
            .collect();
        filtered.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
        filtered.truncate(limit);
        Ok(filtered)
    }

    fn count(&self) -> usize {
        let entries = self.entries.read().unwrap();
        entries.len()
    }
}

/// 코사인 유사도 (brute-force)
fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}
