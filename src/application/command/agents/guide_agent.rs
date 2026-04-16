//! GuideAgent — 연기 가이드 생성 전담
//!
//! 순수 함수 래퍼. Side-effect 없음.

use crate::application::command::handler::{HandlerContext, HandlerOutput};
use crate::application::command::types::CommandResult;
use crate::application::dto::GuideResult;
use crate::application::mind_service::MindServiceError;
use crate::domain::event::EventPayload;
use crate::domain::guide::ActingGuide;

/// 연기 가이드 생성 에이전트
pub struct GuideAgent;

impl GuideAgent {
    pub fn new() -> Self {
        Self
    }

    pub fn handle_generate(
        &self,
        npc_id: &str,
        partner_id: &str,
        situation_description: &Option<String>,
        ctx: &HandlerContext,
    ) -> Result<HandlerOutput, MindServiceError> {
        let npc = ctx.npc.as_ref().ok_or_else(|| MindServiceError::NpcNotFound(npc_id.into()))?;
        let emotion_state = ctx
            .emotion_state
            .as_ref()
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        let guide = ActingGuide::build(
            npc,
            emotion_state,
            situation_description.clone(),
            ctx.relationship.as_ref(),
            &ctx.partner_name,
        );

        let event = EventPayload::GuideGenerated {
            npc_id: npc_id.to_string(),
            partner_id: partner_id.to_string(),
        };

        Ok(HandlerOutput::simple(
            CommandResult::GuideGenerated(GuideResult { guide }),
            vec![event],
        ))
    }
}

impl Default for GuideAgent {
    fn default() -> Self {
        Self::new()
    }
}
