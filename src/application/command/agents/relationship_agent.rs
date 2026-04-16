//! RelAgent — 관계 갱신 전담

use crate::application::command::handler::{HandlerContext, HandlerOutput};
use crate::application::command::types::CommandResult;
use crate::application::mind_service::MindServiceError;
use crate::application::relationship_service::RelationshipService;
use crate::application::dto::AfterDialogueRequest;
use crate::domain::event::EventPayload;

/// 관계 갱신 에이전트
pub struct RelationshipAgent {
    service: RelationshipService,
}

impl RelationshipAgent {
    pub fn new() -> Self {
        Self {
            service: RelationshipService::new(),
        }
    }

    /// UpdateRelationship Command 처리 (Beat 종료)
    pub fn handle_update(
        &self,
        npc_id: &str,
        partner_id: &str,
        significance: &Option<f32>,
        ctx: &HandlerContext,
    ) -> Result<HandlerOutput, MindServiceError> {
        let rel = ctx.relationship.as_ref().ok_or_else(|| {
            MindServiceError::RelationshipNotFound(npc_id.into(), partner_id.into())
        })?;
        let emotion = ctx
            .emotion_state
            .as_ref()
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        let req = AfterDialogueRequest {
            npc_id: npc_id.to_string(),
            partner_id: partner_id.to_string(),
            significance: *significance,
        };
        let (new_rel, response) = self.service.update_relationship(rel, emotion, &req);

        let event = EventPayload::RelationshipUpdated {
            owner_id: npc_id.to_string(),
            target_id: partner_id.to_string(),
            before_closeness: response.before.closeness,
            before_trust: response.before.trust,
            before_power: response.before.power,
            after_closeness: response.after.closeness,
            after_trust: response.after.trust,
            after_power: response.after.power,
        };

        Ok(HandlerOutput {
            result: CommandResult::RelationshipUpdated(response),
            events: vec![event],
            new_emotion_state: None,
            new_relationship: Some((npc_id.to_string(), partner_id.to_string(), new_rel)),
            clear_emotion: None,
            clear_scene: false,
            save_scene: None,
        })
    }

    /// EndDialogue Command 처리 (관계 갱신 + 감정 초기화 + Scene 정리)
    pub fn handle_end_dialogue(
        &self,
        npc_id: &str,
        partner_id: &str,
        significance: &Option<f32>,
        ctx: &HandlerContext,
    ) -> Result<HandlerOutput, MindServiceError> {
        let rel = ctx.relationship.as_ref().ok_or_else(|| {
            MindServiceError::RelationshipNotFound(npc_id.into(), partner_id.into())
        })?;
        let emotion = ctx
            .emotion_state
            .as_ref()
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        let req = AfterDialogueRequest {
            npc_id: npc_id.to_string(),
            partner_id: partner_id.to_string(),
            significance: *significance,
        };
        let (new_rel, response) = self.service.update_relationship(rel, emotion, &req);

        let rel_event = EventPayload::RelationshipUpdated {
            owner_id: npc_id.to_string(),
            target_id: partner_id.to_string(),
            before_closeness: response.before.closeness,
            before_trust: response.before.trust,
            before_power: response.before.power,
            after_closeness: response.after.closeness,
            after_trust: response.after.trust,
            after_power: response.after.power,
        };

        let clear_event = EventPayload::EmotionCleared {
            npc_id: npc_id.to_string(),
        };

        let scene_event = EventPayload::SceneEnded {
            npc_id: npc_id.to_string(),
            partner_id: partner_id.to_string(),
        };

        Ok(HandlerOutput {
            result: CommandResult::DialogueEnded(response),
            events: vec![rel_event, clear_event, scene_event],
            new_relationship: Some((npc_id.to_string(), partner_id.to_string(), new_rel)),
            new_emotion_state: None,
            clear_emotion: Some(npc_id.to_string()),
            clear_scene: true,
            save_scene: None,
        })
    }
}

impl Default for RelationshipAgent {
    fn default() -> Self {
        Self::new()
    }
}
