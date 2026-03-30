//! 감정 평가 세부 모듈 정의

pub mod event;
pub mod action;
pub mod object;
pub mod compound;
pub mod helpers;

use crate::domain::emotion::{EmotionState, Situation, RelationshipModifiers};
use crate::ports::AppraisalWeights;

/// 감정 평가의 각 분기를 처리하는 내부 통합 함수
pub fn process<P: AppraisalWeights>(
    personality: &P,
    situation: &Situation,
    dialogue_modifiers: &RelationshipModifiers,
) -> EmotionState {
    let mut state = EmotionState::new();

    if let Some(event) = &situation.event {
        event::appraise(personality, &mut state, event);
    }
    if let Some(action) = &situation.action {
        action::appraise(personality, &mut state, dialogue_modifiers, action);
    }
    if let Some(object) = &situation.object {
        object::appraise(personality, &mut state, object);
    }
    if let (Some(action), Some(_)) = (&situation.action, &situation.event) {
        compound::appraise(&mut state, action.agent_id.is_none(), &situation.description);
    }

    state
}
