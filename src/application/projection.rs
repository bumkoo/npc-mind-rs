//! Projection — 이벤트 스트림에서 파생된 읽기 전용 뷰
//!
//! Phase 1에서는 검증용으로만 사용합니다.
//! InMemoryRepository가 여전히 실제 읽기 경로를 담당합니다.

use crate::domain::event::{DomainEvent, EventPayload};
use std::collections::HashMap;

/// 이벤트를 수신하여 뷰를 갱신하는 트레이트
pub trait Projection: Send + Sync {
    /// 이벤트 적용 — 뷰 갱신
    fn apply(&mut self, event: &DomainEvent);
}

// ---------------------------------------------------------------------------
// EmotionProjection — NPC별 mood + dominant 추적
// ---------------------------------------------------------------------------

/// NPC별 감정 요약 뷰
#[derive(Debug, Default)]
pub struct EmotionProjection {
    /// npc_id → mood (-1.0 ~ 1.0)
    moods: HashMap<String, f32>,
    /// npc_id → (emotion_type, intensity)
    dominants: HashMap<String, (String, f32)>,
    /// npc_id → 전체 감정 스냅샷 (Phase 2: primary read path)
    snapshots: HashMap<String, Vec<(String, f32)>>,
}

impl EmotionProjection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_mood(&self, npc_id: &str) -> Option<f32> {
        self.moods.get(npc_id).copied()
    }

    pub fn get_dominant(&self, npc_id: &str) -> Option<&(String, f32)> {
        self.dominants.get(npc_id)
    }

    /// 전체 감정 스냅샷 조회 (Phase 2)
    pub fn get_snapshot(&self, npc_id: &str) -> Option<&Vec<(String, f32)>> {
        self.snapshots.get(npc_id)
    }
}

impl Projection for EmotionProjection {
    fn apply(&mut self, event: &DomainEvent) {
        match &event.payload {
            EventPayload::EmotionAppraised {
                npc_id,
                mood,
                dominant,
                emotion_snapshot,
                ..
            } => {
                self.moods.insert(npc_id.clone(), *mood);
                if let Some(d) = dominant {
                    self.dominants.insert(npc_id.clone(), d.clone());
                }
                if !emotion_snapshot.is_empty() {
                    self.snapshots.insert(npc_id.clone(), emotion_snapshot.clone());
                }
            }
            EventPayload::StimulusApplied {
                npc_id,
                mood_after,
                emotion_snapshot,
                ..
            } => {
                self.moods.insert(npc_id.clone(), *mood_after);
                if !emotion_snapshot.is_empty() {
                    self.snapshots.insert(npc_id.clone(), emotion_snapshot.clone());
                }
            }
            EventPayload::EmotionCleared { npc_id } => {
                self.moods.remove(npc_id);
                self.dominants.remove(npc_id);
                self.snapshots.remove(npc_id);
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// RelationshipProjection — 관계 수치 추적
// ---------------------------------------------------------------------------

/// (owner, target) 쌍의 관계 수치 뷰
#[derive(Debug, Default)]
pub struct RelationshipProjection {
    /// (owner_id, target_id) → (closeness, trust, power)
    values: HashMap<(String, String), (f32, f32, f32)>,
}

impl RelationshipProjection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_values(&self, owner: &str, target: &str) -> Option<(f32, f32, f32)> {
        self.values
            .get(&(owner.to_string(), target.to_string()))
            .copied()
    }
}

impl Projection for RelationshipProjection {
    fn apply(&mut self, event: &DomainEvent) {
        if let EventPayload::RelationshipUpdated {
            owner_id,
            target_id,
            after_closeness,
            after_trust,
            after_power,
            ..
        } = &event.payload
        {
            self.values.insert(
                (owner_id.clone(), target_id.clone()),
                (*after_closeness, *after_trust, *after_power),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// SceneProjection — 활성 Scene 상태 추적
// ---------------------------------------------------------------------------

/// Scene 활성 상태 뷰
#[derive(Debug, Default)]
pub struct SceneProjection {
    /// (npc_id, partner_id, active_focus_id)
    active: Option<(String, String, Option<String>)>,
}

impl SceneProjection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub fn active_focus_id(&self) -> Option<&str> {
        self.active
            .as_ref()
            .and_then(|(_, _, f)| f.as_deref())
    }
}

impl Projection for SceneProjection {
    fn apply(&mut self, event: &DomainEvent) {
        match &event.payload {
            EventPayload::SceneStarted {
                npc_id,
                partner_id,
                initial_focus_id,
                ..
            } => {
                self.active = Some((
                    npc_id.clone(),
                    partner_id.clone(),
                    initial_focus_id.clone(),
                ));
            }
            EventPayload::BeatTransitioned { to_focus_id, .. } => {
                if let Some((_, _, ref mut focus)) = self.active {
                    *focus = Some(to_focus_id.clone());
                }
            }
            EventPayload::SceneEnded { .. } => {
                self.active = None;
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// ProjectionRegistry — 복수 Projection 일괄 적용
// ---------------------------------------------------------------------------

/// 여러 Projection을 묶어 이벤트를 일괄 적용
pub struct ProjectionRegistry {
    projections: Vec<Box<dyn Projection>>,
}

impl ProjectionRegistry {
    pub fn new() -> Self {
        Self {
            projections: Vec::new(),
        }
    }

    pub fn add(&mut self, projection: impl Projection + 'static) {
        self.projections.push(Box::new(projection));
    }

    pub fn apply_all(&mut self, event: &DomainEvent) {
        for proj in &mut self.projections {
            proj.apply(event);
        }
    }
}

impl Default for ProjectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
