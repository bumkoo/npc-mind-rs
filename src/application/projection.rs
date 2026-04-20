//! Projection — 이벤트 스트림에서 파생된 읽기 전용 뷰 (v2)
//!
//! `EmotionProjectionHandler` 등 v2 wrapper가 내부적으로 재사용하는 상태 컨테이너.
//! 이벤트 적용은 `apply(&mut self, &DomainEvent)` inherent 메서드로 수행한다.

use crate::domain::event::{DomainEvent, EventPayload};
use std::collections::HashMap;

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
    /// npc_id → 전체 감정 스냅샷
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

    pub fn get_snapshot(&self, npc_id: &str) -> Option<&Vec<(String, f32)>> {
        self.snapshots.get(npc_id)
    }

    pub fn apply(&mut self, event: &DomainEvent) {
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

    pub fn apply(&mut self, event: &DomainEvent) {
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

    pub fn apply(&mut self, event: &DomainEvent) {
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
