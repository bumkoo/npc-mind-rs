//! 행동(Action)에 대한 감정 평가 로직

use crate::domain::emotion::{ActionFocus, EmotionState, EmotionType, RelationshipModifiers};
use crate::ports::AppraisalWeights;
use super::helpers::*;

pub fn appraise<P: AppraisalWeights>(
    p: &P,
    state: &mut EmotionState,
    dialogue_modifiers: &RelationshipModifiers,
    action: &ActionFocus,
) {
    let pw = action.praiseworthiness;
    let ctx = &action.description;

    match (&action.agent_id, &action.modifiers) {
        (None, _) => {
            // 자기 행동 (Pride, Shame)
            let w = p.praiseworthiness_weight(true, pw);
            add_valence(state, EmotionType::Pride, EmotionType::Shame, pw, w, 1.0, ctx);
        }
        (Some(_), mods) => {
            // 타인 행동 (Admiration, Reproach)
            let mods = mods.as_ref().unwrap_or(dialogue_modifiers);
            let w = p.praiseworthiness_weight(false, pw);
            let modifier = mods.intensity_multiplier * mods.trust_modifier;
            add_valence(state, EmotionType::Admiration, EmotionType::Reproach, pw, w, modifier, ctx);
        }
    }
}
