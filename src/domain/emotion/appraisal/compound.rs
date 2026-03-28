//! 복합 감정(Compound) 생성 로직

use crate::domain::emotion::{EmotionState, EmotionType};
use super::helpers::*;

pub fn appraise(state: &mut EmotionState, is_self: bool, situation_desc: &str) {
    if is_self {
        add_compound(state, EmotionType::Gratification, EmotionType::Pride, EmotionType::Joy, situation_desc);
        add_compound(state, EmotionType::Remorse, EmotionType::Shame, EmotionType::Distress, situation_desc);
    } else {
        add_compound(state, EmotionType::Gratitude, EmotionType::Admiration, EmotionType::Joy, situation_desc);
        add_compound(state, EmotionType::Anger, EmotionType::Reproach, EmotionType::Distress, situation_desc);
    }
}
