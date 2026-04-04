use crate::domain::emotion::{EmotionState, Scene, SceneFocus};
use super::dto::{FocusInfoItem, SceneInfoResult};

/// Scene 및 Beat 관리 전담 서비스
pub struct SceneService;

impl SceneService {
    pub fn new() -> Self {
        Self
    }

    /// 현재 감정 상태에 따라 Scene 트리거를 체크하고,
    /// 충족되는 Focus가 있으면 해당 Focus를 반환합니다.
    pub fn check_trigger(
        &self,
        scene: &Scene,
        state: &EmotionState,
    ) -> Option<SceneFocus> {
        scene.check_trigger(state).cloned()
    }

    /// Scene 정보 요약 결과를 생성합니다.
    pub fn build_scene_info(&self, scene: &Scene) -> SceneInfoResult {
        let active_id = scene.active_focus_id();
        let focus_infos = scene
            .focuses()
            .iter()
            .map(|f| FocusInfoItem::from_domain(f, active_id == Some(f.id.as_str())))
            .collect();

        SceneInfoResult {
            has_scene: true,
            npc_id: Some(scene.npc_id().to_string()),
            partner_id: Some(scene.partner_id().to_string()),
            active_focus_id: scene.active_focus_id().map(|s| s.to_string()),
            significance: Some(scene.significance()),
            focuses: focus_infos,
        }
    }
}
