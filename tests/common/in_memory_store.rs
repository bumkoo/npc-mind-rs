//! 테스트 전용 InMemoryMemoryStore — 라이브러리 코어에서 제외된 brute-force cosine 스토어.
//!
//! `MemoryStore` 트레이트의 결정적 참조 구현으로서 테스트에서만 사용한다.
//! 프로덕션 경로에서는 `SqliteMemoryStore`(sqlite-vec 기반)가 기본 구현이다.

use npc_mind::domain::memory::{MemoryEntry, MemoryResult, MemoryScope};
use npc_mind::ports::{MemoryError, MemoryQuery, MemoryScopeFilter, MemoryStore};
use std::sync::RwLock;

/// 인메모리 기억 저장소 (테스트용).
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
        // 같은 id가 이미 있으면 대체 (재인덱싱 호환)
        if let Some(pos) = entries.iter().position(|(e, _)| e.id == entry.id) {
            entries[pos] = (entry, embedding);
        } else {
            entries.push((entry, embedding));
        }
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
            .filter(|(e, emb)| emb.is_some() && npc_id.map_or(true, |id| e.legacy_npc_id() == id))
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
                    && npc_id.map_or(true, |id| e.legacy_npc_id() == id)
            })
            .take(limit)
            .map(|(entry, _)| MemoryResult {
                entry: entry.clone(),
                relevance_score: 1.0,
            })
            .collect();
        Ok(results)
    }

    fn get_recent(&self, npc_id: &str, limit: usize) -> Result<Vec<MemoryEntry>, MemoryError> {
        let entries = self.entries.read().unwrap();
        let mut filtered: Vec<_> = entries
            .iter()
            .filter(|(e, _)| e.legacy_npc_id() == npc_id)
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

    // -----------------------------------------------------------------------
    // Step A 신규 메서드 — brute-force 참조 구현
    // -----------------------------------------------------------------------

    fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryResult>, MemoryError> {
        let entries = self.entries.read().unwrap();
        let mut results: Vec<MemoryResult> = entries
            .iter()
            .filter(|(e, _)| {
                // scope_filter
                match &query.scope_filter {
                    None | Some(MemoryScopeFilter::Any) => true,
                    Some(MemoryScopeFilter::Exact(target)) => &e.scope == target,
                    Some(MemoryScopeFilter::NpcAllowed(npc)) => match &e.scope {
                        MemoryScope::Personal { npc_id } => npc_id == npc,
                        MemoryScope::World { .. } => true,
                        MemoryScope::Relationship { a, b } => a == npc || b == npc,
                        // Faction/Family 소속 Join은 Step A 범위 밖
                        _ => false,
                    },
                }
            })
            .filter(|(e, _)| {
                // source_filter
                query
                    .source_filter
                    .as_ref()
                    .map(|srcs| srcs.contains(&e.source))
                    .unwrap_or(true)
            })
            .filter(|(e, _)| {
                // layer_filter
                query.layer_filter.map(|l| e.layer == l).unwrap_or(true)
            })
            .filter(|(e, _)| {
                // topic
                query
                    .topic
                    .as_ref()
                    .map(|t| e.topic.as_deref() == Some(t.as_str()))
                    .unwrap_or(true)
            })
            .filter(|(e, _)| {
                // exclude_superseded
                !query.exclude_superseded || e.superseded_by.is_none()
            })
            .filter(|(e, _)| {
                // exclude_consolidated_source
                !query.exclude_consolidated_source || e.consolidated_into.is_none()
            })
            .map(|(entry, emb)| {
                let score = match (&query.embedding, emb.as_ref()) {
                    (Some(q), Some(e)) => cosine_sim(q, e),
                    _ => 1.0,
                };
                MemoryResult {
                    entry: entry.clone(),
                    relevance_score: score,
                }
            })
            .collect();

        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        if query.limit > 0 {
            results.truncate(query.limit);
        }
        Ok(results)
    }

    fn get_by_id(&self, id: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        let entries = self.entries.read().unwrap();
        Ok(entries.iter().find(|(e, _)| e.id == id).map(|(e, _)| e.clone()))
    }

    fn get_by_topic_latest(&self, topic: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        let entries = self.entries.read().unwrap();
        let mut candidates: Vec<&MemoryEntry> = entries
            .iter()
            .map(|(e, _)| e)
            .filter(|e| {
                e.topic.as_deref() == Some(topic) && e.superseded_by.is_none()
            })
            .collect();
        candidates.sort_by(|a, b| b.created_seq.cmp(&a.created_seq));
        Ok(candidates.first().map(|e| (*e).clone()))
    }

    fn get_canonical_by_topic(&self, topic: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        let entries = self.entries.read().unwrap();
        let mut candidates: Vec<&MemoryEntry> = entries
            .iter()
            .map(|(e, _)| e)
            .filter(|e| {
                e.topic.as_deref() == Some(topic)
                    && e.provenance.is_canonical(&e.scope)
                    && e.superseded_by.is_none()
            })
            .collect();
        candidates.sort_by(|a, b| b.created_seq.cmp(&a.created_seq));
        Ok(candidates.first().map(|e| (*e).clone()))
    }

    fn mark_superseded(&self, old_id: &str, new_id: &str) -> Result<(), MemoryError> {
        let mut entries = self.entries.write().unwrap();
        if let Some((e, _)) = entries.iter_mut().find(|(e, _)| e.id == old_id) {
            e.superseded_by = Some(new_id.to_string());
        }
        Ok(())
    }

    fn mark_consolidated(&self, a_ids: &[String], b_id: &str) -> Result<(), MemoryError> {
        let mut entries = self.entries.write().unwrap();
        for (e, _) in entries.iter_mut() {
            if a_ids.iter().any(|id| id == &e.id) {
                e.consolidated_into = Some(b_id.to_string());
            }
        }
        Ok(())
    }

    fn record_recall(&self, id: &str, now_ms: u64) -> Result<(), MemoryError> {
        let mut entries = self.entries.write().unwrap();
        if let Some((e, _)) = entries.iter_mut().find(|(e, _)| e.id == id) {
            e.last_recalled_at = Some(now_ms);
            e.recall_count = e.recall_count.saturating_add(1);
        }
        Ok(())
    }
}

fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}
