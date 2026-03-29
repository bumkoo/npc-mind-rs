//! 사건(Event)에 대한 감정 평가 로직

use crate::domain::emotion::{EmotionState, EmotionType, EventFocus, Prospect, ProspectResult};
use crate::ports::AppraisalWeights;
use super::helpers::*;

pub fn appraise<P: AppraisalWeights>(p: &P, state: &mut EmotionState, event: &EventFocus) {
    let d = event.desirability_for_self;
    let ctx = &event.description;

    // 1. 전망 확인 (Satisfaction, Disappointment, Relief, FearsConfirmed)
    if let Some(Prospect::Confirmation(result)) = &event.prospect {
        let w = p.desirability_confirmation_weight(d);
        add_confirmation(state, result, d, w, ctx);

        // 사건이 발생하지 않은 경우: 확인 감정만 생성
        // 사건이 발생한 경우(HopeFulfilled, FearConfirmed): Joy/Distress도 필요 → fall-through
        match result {
            ProspectResult::HopeUnfulfilled | ProspectResult::FearUnrealized => return,
            _ => {} // HopeFulfilled, FearConfirmed → 아래 Joy/Distress 로직 계속
        }
    }

    // 2. 미래 전망 (Hope, Fear)
    if let Some(Prospect::Anticipation) = &event.prospect {
        let w = p.desirability_prospect_weight(d);
        add_valence(state, EmotionType::Hope, EmotionType::Fear, d, w, 1.0, ctx);
        return;
    }

    // 3. 자기 복지 (Joy, Distress)
    let w = p.desirability_self_weight(d);
    add_valence(state, EmotionType::Joy, EmotionType::Distress, d, w, 1.0, ctx);

    // 4. 타인의 운 (HappyFor, Pity, Resentment, Gloating)
    if let Some(other) = &event.desirability_for_other {
        let d_other = other.desirability;
        let other_ctx = format!("{} (대상: {})", ctx, other.target_id);

        // 공감 기반 (HappyFor, Pity)
        let emp_w = p.empathy_weight(d_other);
        let emp_mod = other.relationship.empathy_rel_modifier();
        add_valence(state, EmotionType::HappyFor, EmotionType::Pity, d_other, emp_w, emp_mod, &other_ctx);

        // 적대 기반 (Resentment, Gloating)
        let hos_w = p.hostility_weight(d_other);
        let hos_mod = other.relationship.hostility_rel_modifier();
        add_valence(state, EmotionType::Resentment, EmotionType::Gloating, d_other, hos_w, hos_mod, &other_ctx);
    }
}
