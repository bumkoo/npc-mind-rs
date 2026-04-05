use crate::domain::emotion::{SceneFocus, Situation};
use crate::ports::MindRepository;
use super::dto::{SceneFocusInput, SituationInput};
use super::mind_service::MindServiceError;

/// DTO를 도메인 모델(Situation, SceneFocus)로 변환하는 서비스
/// 
/// 저장소에서 관계 정보(modifiers)나 오브젝트 설명 등을 조회하여
/// DTO의 변환 메서드에 주입합니다.
pub struct SituationService;

impl SituationService {
    pub fn new() -> Self {
        Self
    }

    /// SituationInput DTO를 Situation 도메인 모델로 변환합니다.
    pub fn to_situation<R: MindRepository>(
        &self,
        repo: &R,
        input: &SituationInput,
        npc_id: &str,
        partner_id: &str,
    ) -> Result<Situation, MindServiceError> {
        let event_other_modifiers = input
            .event
            .as_ref()
            .and_then(|e| e.other.as_ref())
            .and_then(|o| {
                repo.get_relationship(npc_id, &o.target_id)
                    .map(|r| r.modifiers())
            });

        let action_agent_modifiers = input
            .action
            .as_ref()
            .and_then(|a| a.agent_id.as_ref())
            .filter(|&agent| agent != partner_id && agent != npc_id)
            .and_then(|agent| {
                repo.get_relationship(npc_id, agent)
                    .map(|r| r.modifiers())
            });

        let object_description = input
            .object
            .as_ref()
            .and_then(|o| repo.get_object_description(&o.target_id));

        input.to_domain(
            event_other_modifiers,
            action_agent_modifiers,
            object_description,
            npc_id,
        )
    }

    /// SceneFocusInput DTO를 SceneFocus 도메인 모델로 변환합니다.
    pub fn to_scene_focus<R: MindRepository>(
        &self,
        repo: &R,
        input: &SceneFocusInput,
        npc_id: &str,
        partner_id: &str,
    ) -> Result<SceneFocus, MindServiceError> {
        let event_other_modifiers = input
            .event
            .as_ref()
            .and_then(|e| e.other.as_ref())
            .and_then(|o| {
                repo.get_relationship(npc_id, &o.target_id)
                    .map(|r| r.modifiers())
            });

        let action_agent_modifiers = input
            .action
            .as_ref()
            .and_then(|a| a.agent_id.as_ref())
            .filter(|&agent| agent != partner_id && agent != npc_id)
            .and_then(|agent| {
                repo.get_relationship(npc_id, agent)
                    .map(|r| r.modifiers())
            });

        let object_description = input
            .object
            .as_ref()
            .and_then(|o| repo.get_object_description(&o.target_id));

        input.to_domain(
            event_other_modifiers,
            action_agent_modifiers,
            object_description,
            npc_id,
        )
    }
}
