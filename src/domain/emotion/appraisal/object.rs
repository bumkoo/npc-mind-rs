//! 대상(Object)에 대한 감정 평가 로직

use crate::domain::emotion::{EmotionState, EmotionType, ObjectFocus};
use crate::ports::AppraisalWeights;
use super::helpers::*;

pub fn appraise<P: AppraisalWeights>(
    p: &P,
    state: &mut EmotionState,
    object: &ObjectFocus,
) {
    let ap = object.appealingness;
    let w = p.appealingness_weight(ap);
    let ctx = &object.target_description;
    
    add_valence(state, EmotionType::Love, EmotionType::Hate, ap, w, 1.0, ctx);
}
