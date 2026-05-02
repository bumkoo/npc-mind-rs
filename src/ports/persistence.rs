use crate::domain::personality::Npc;
use crate::domain::relationship::Relationship;
use crate::domain::emotion::{EmotionState, Scene};
use crate::domain::scene_id::SceneId;

/// NPC/관계/오브젝트 월드 — 게임 세계 데이터 조회 및 관계 갱신
pub trait NpcWorld {
    fn get_npc(&self, id: &str) -> Option<Npc>;
    fn get_relationship(&self, owner_id: &str, target_id: &str) -> Option<Relationship>;
    fn get_object_description(&self, object_id: &str) -> Option<String>;
    fn save_relationship(&mut self, owner_id: &str, target_id: &str, rel: Relationship);
}

/// 감정 상태 저장소 — NPC별 감정 상태 CRUD
pub trait EmotionStore {
    fn get_emotion_state(&self, npc_id: &str) -> Option<EmotionState>;
    fn save_emotion_state(&mut self, npc_id: &str, state: EmotionState);
    fn clear_emotion_state(&mut self, npc_id: &str);
}

/// Scene 상태 저장소 — Scene/Focus/Beat 관리
pub trait SceneStore {
    fn get_scene(&self) -> Option<Scene>;
    fn save_scene(&mut self, scene: Scene);
    fn clear_scene(&mut self);
    fn get_scene_by_id(&self, scene_id: &SceneId) -> Option<Scene> {
        self.get_scene().filter(|s| {
            s.npc_id() == scene_id.npc_id && s.partner_id() == scene_id.partner_id
        })
    }
}

/// 편의 super-trait — 3개 포트를 모두 구현하면 자동으로 MindRepository
pub trait MindRepository: NpcWorld + EmotionStore + SceneStore {}

/// 3개 포트를 모두 구현한 타입은 자동으로 MindRepository
impl<T: NpcWorld + EmotionStore + SceneStore> MindRepository for T {}
