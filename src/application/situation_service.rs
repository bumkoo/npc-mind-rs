use crate::domain::emotion::{RelationshipModifiers, SceneFocus, Situation};
use crate::ports::MindRepository;
use super::dto::{HasFocusFields, SceneFocusInput, SituationInput};
use super::error::MindServiceError;

/// DTO를 도메인 모델(Situation, SceneFocus)로 변환하는 서비스
///
/// 저장소에서 관계 정보(modifiers)나 오브젝트 설명 등을 조회하여
/// DTO의 변환 메서드에 주입합니다.
pub struct SituationService;

/// event/action/object 변환에 필요한 저장소 조회 결과
pub struct FocusContext {
    pub event_other_modifiers: Option<RelationshipModifiers>,
    pub action_agent_modifiers: Option<RelationshipModifiers>,
    pub object_description: Option<String>,
}

impl SituationService {
    pub fn new() -> Self {
        Self
    }

    /// event/action/object에 필요한 context를 repository에서 일괄 조회
    pub fn resolve_focus_context<R: MindRepository>(
        repo: &R,
        input: &impl HasFocusFields,
        npc_id: &str,
        partner_id: &str,
    ) -> FocusContext {
        let event_other_modifiers = input
            .event()
            .and_then(|e| e.other.as_ref())
            .and_then(|o| {
                repo.get_relationship(npc_id, &o.target_id)
                    .map(|r| r.modifiers())
            });

        let action_agent_modifiers = input
            .action()
            .and_then(|a| a.agent_id.as_ref())
            .filter(|&agent| agent != partner_id && agent != npc_id)
            .and_then(|agent| {
                repo.get_relationship(npc_id, agent)
                    .map(|r| r.modifiers())
            });

        let object_description = input
            .object()
            .and_then(|o| repo.get_object_description(&o.target_id));

        FocusContext { event_other_modifiers, action_agent_modifiers, object_description }
    }

    /// SituationInput DTO를 Situation 도메인 모델로 변환합니다.
    pub fn to_situation<R: MindRepository>(
        &self,
        repo: &R,
        input: &SituationInput,
        npc_id: &str,
        partner_id: &str,
    ) -> Result<Situation, MindServiceError> {
        let ctx = Self::resolve_focus_context(repo, input, npc_id, partner_id);
        input.to_domain(ctx.event_other_modifiers, ctx.action_agent_modifiers, ctx.object_description, npc_id)
    }

    /// SceneFocusInput DTO를 SceneFocus 도메인 모델로 변환합니다.
    pub fn to_scene_focus<R: MindRepository>(
        &self,
        repo: &R,
        input: &SceneFocusInput,
        npc_id: &str,
        partner_id: &str,
    ) -> Result<SceneFocus, MindServiceError> {
        let ctx = Self::resolve_focus_context(repo, input, npc_id, partner_id);
        input.to_domain(ctx.event_other_modifiers, ctx.action_agent_modifiers, ctx.object_description, npc_id)
    }
}
