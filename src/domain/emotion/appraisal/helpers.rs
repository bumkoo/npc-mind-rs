//! 감정 생성 공통 헬퍼 메서드

use crate::domain::emotion::{Emotion, EmotionState, EmotionType, ProspectResult};
use tracing::trace;

/// 밸런스 페어 (Positive/Negative) 감정 추가 헬퍼
pub fn add_valence(
    state: &mut EmotionState,
    pos_type: EmotionType,
    neg_type: EmotionType,
    base_val: f32,
    weight: f32,
    modifier: f32,
    ctx: &str,
) {
    if weight <= 0.0 { return; }

    if base_val > 0.0 {
        let val = base_val * weight * modifier;
        trace!(emotion = ?pos_type, base_val, weight, modifier, result = val, context = %ctx);
        state.add(Emotion::with_context(pos_type, val, ctx));
    } else if base_val < 0.0 {
        let val = base_val.abs() * weight * modifier;
        trace!(emotion = ?neg_type, base_val = base_val.abs(), weight, modifier, result = val, context = %ctx);
        state.add(Emotion::with_context(neg_type, val, ctx));
    }
}

/// 전망 확인 (4종) 감정 추가 헬퍼
pub fn add_confirmation(
    state: &mut EmotionState,
    result: &ProspectResult,
    base_val: f32,
    weight: f32,
    ctx: &str,
) {
    let etype = match result {
        ProspectResult::HopeFulfilled => EmotionType::Satisfaction,
        ProspectResult::HopeUnfulfilled => EmotionType::Disappointment,
        ProspectResult::FearUnrealized => EmotionType::Relief,
        ProspectResult::FearConfirmed => EmotionType::FearsConfirmed,
    };
    let val = base_val.abs() * weight;
    trace!(emotion = ?etype, base_val = base_val.abs(), weight, result = val, context = %ctx);
    state.add(Emotion::with_context(etype, val, ctx));
}

/// 복합 감정 (두 감정의 조합) 생성 헬퍼
pub fn add_compound(
    state: &mut EmotionState,
    target_type: EmotionType,
    comp1_type: EmotionType,
    comp2_type: EmotionType,
    ctx: &str,
) {
    let val1 = state.intensity_of(comp1_type);
    let val2 = state.intensity_of(comp2_type);

    if val1 > 0.0 && val2 > 0.0 {
        let val = (val1 + val2) / 2.0;
        trace!(emotion = ?target_type, comp1_type = ?comp1_type, comp1_val = val1, comp2_type = ?comp2_type, comp2_val = val2, result = val, context = %ctx);
        state.add(Emotion::with_context(target_type, val, ctx));
    }
}
