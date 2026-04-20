use crate::domain::emotion::Scene;
use super::dto::{FocusInfoItem, SceneInfoResult};

/// Scene 정보 뷰 빌더
///
/// Mind Studio가 현재 활성 Scene 상태를 REST 응답용 DTO로 변환할 때 사용한다.
/// trigger 평가는 도메인 `Scene::check_trigger`를 직접 호출하므로 이 서비스 책임 아님.
pub struct SceneService;

impl SceneService {
    pub fn new() -> Self {
        Self
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
            script_cursor: None, // MCP 레벨에서 주입
        }
    }
}
