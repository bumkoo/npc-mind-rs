//! Mind Studio AppState → MindRepository 읽기 전용 어댑터
//!
//! `StateInner`(UI 스키마)에서 `NpcWorld` / `EmotionStore` / `SceneStore`를 조회 가능한
//! 얇은 래퍼. 쓰기는 `domain_sync` helper가 별도 `InMemoryRepository`를 경유하므로
//! 본 래퍼는 read-only. save_* 메서드는 `unreachable!()`로 방어한다.

use crate::state::StateInner;
use npc_mind::domain::emotion::{EmotionState, Scene};
use npc_mind::domain::personality::Npc;
use npc_mind::domain::relationship::Relationship;
use npc_mind::ports::{EmotionStore, NpcWorld, SceneStore};

/// 읽기 전용 저장소 래퍼 (불변 조회 전용)
pub struct ReadOnlyAppStateRepo<'a> {
    pub inner: &'a StateInner,
}

impl<'a> NpcWorld for ReadOnlyAppStateRepo<'a> {
    fn get_npc(&self, id: &str) -> Option<Npc> {
        self.inner.npcs.get(id).map(|p| p.to_npc())
    }
    fn get_relationship(&self, owner_id: &str, target_id: &str) -> Option<Relationship> {
        self.inner
            .find_relationship(owner_id, target_id)
            .map(|r| r.to_relationship())
    }
    fn get_object_description(&self, _: &str) -> Option<String> {
        None
    }
    fn save_relationship(&mut self, _: &str, _: &str, _: Relationship) {
        unreachable!("read-only")
    }
}

impl<'a> EmotionStore for ReadOnlyAppStateRepo<'a> {
    fn get_emotion_state(&self, npc_id: &str) -> Option<EmotionState> {
        self.inner.emotions.get(npc_id).cloned()
    }
    fn save_emotion_state(&mut self, _: &str, _: EmotionState) {
        unreachable!("read-only")
    }
    fn clear_emotion_state(&mut self, _: &str) {
        unreachable!("read-only")
    }
}

impl<'a> SceneStore for ReadOnlyAppStateRepo<'a> {
    fn get_scene(&self) -> Option<Scene> {
        let npc_id = self.inner.scene_npc_id.as_ref()?;
        let partner_id = self.inner.scene_partner_id.as_ref()?;
        let mut scene = Scene::new(
            npc_id.clone(),
            partner_id.clone(),
            self.inner.scene_focuses.clone(),
        );
        if let Some(ref id) = self.inner.active_focus_id {
            scene.set_active_focus(id.clone());
        }
        Some(scene)
    }

    fn save_scene(&mut self, _: Scene) {
        unreachable!("read-only")
    }
    fn clear_scene(&mut self) {
        unreachable!("read-only")
    }
}
