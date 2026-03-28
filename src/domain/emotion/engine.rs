//! 감정 평가 엔진 (Appraisal Engine)
//!
//! AppraisalWeights 포트를 통해 성격 가중치를 받아
//! 상황(Situation)에서 OCC 감정(EmotionState)을 생성하는 핵심 로직.
//!
//! ## 도메인 서비스 — 순수 함수, 상태 없음
//!
//! 설계 원칙:
//! - 엔진은 성격 모델 내부를 모름 (AppraisalWeights trait만 의존)
//! - OCC 입력 변수별 weight 메서드 호출 → 감정 강도 산출
//! - Compound 감정은 이미 계산된 기초 감정값을 결합
//! - rel_mul/trust_mod은 타인 행동(Admiration/Reproach)에만 적용
//! - Action 3분기: 자기 / 대화 상대 / 제3자
//! - 모든 감정에 context(원인/맥락) 부착 → 프롬프트에 활용
//! - tracing: 구조화된 trace 이벤트 방출 (subscriber 없으면 no-op)

use tracing::trace;

use crate::ports::AppraisalWeights;
use crate::domain::relationship::Relationship;

use super::types::{Emotion, EmotionState, EmotionType};
use super::situation::*;

/// 도메인 서비스 — 성격(AppraisalWeights) × Relationship → OCC 감정
pub struct AppraisalEngine;

impl AppraisalEngine {
    pub fn appraise<P: AppraisalWeights>(
        personality: &P,
        situation: &Situation,
        relationship: &Relationship,
    ) -> EmotionState {
        let mut state = EmotionState::new();

        if let Some(event) = &situation.event {
            Self::appraise_event(personality, &mut state, event);
        }
        if let Some(action) = &situation.action {
            Self::appraise_action(personality, &mut state, relationship, action);
        }
        if let Some(object) = &situation.object {
            Self::appraise_object(personality, &mut state, object);
        }
        if let (Some(action), Some(_)) = (&situation.action, &situation.event) {
            Self::appraise_compound(&mut state, action.agent_id.is_none(), &situation.description);
        }

        state
    }

    fn appraise_event<P: AppraisalWeights>(p: &P, state: &mut EmotionState, event: &EventFocus) {
        let d = event.desirability_for_self;
        let ctx = &event.description;

        // 1. 전망 확인 (Satisfaction, Disappointment, Relief, FearsConfirmed)
        if let Some(Prospect::Confirmation(result)) = &event.prospect {
            let w = p.desirability_confirmation_weight(d);
            Self::add_confirmation(state, result, d, w, ctx);
            return;
        }

        // 2. 미래 전망 (Hope, Fear)
        if let Some(Prospect::Anticipation) = &event.prospect {
            let w = p.desirability_prospect_weight(d);
            Self::add_valence(state, EmotionType::Hope, EmotionType::Fear, d, w, 1.0, ctx);
            return;
        }

        // 3. 자기 복지 (Joy, Distress)
        let w = p.desirability_self_weight(d);
        Self::add_valence(state, EmotionType::Joy, EmotionType::Distress, d, w, 1.0, ctx);

        // 4. 타인의 운 (HappyFor, Pity, Resentment, Gloating)
        if let Some(other) = &event.desirability_for_other {
            let d_other = other.desirability;
            let other_ctx = format!("{} (대상: {})", ctx, other.target_id);

            // 공감 기반 (HappyFor, Pity)
            let emp_w = p.empathy_weight(d_other);
            let emp_mod = other.relationship.empathy_rel_modifier();
            Self::add_valence(state, EmotionType::HappyFor, EmotionType::Pity, d_other, emp_w, emp_mod, &other_ctx);

            // 적대 기반 (Resentment, Gloating)
            let hos_w = p.hostility_weight(d_other);
            let hos_mod = other.relationship.hostility_rel_modifier();
            Self::add_valence(state, EmotionType::Resentment, EmotionType::Gloating, d_other, hos_w, hos_mod, &other_ctx);
        }
    }

    fn appraise_action<P: AppraisalWeights>(
        p: &P,
        state: &mut EmotionState,
        dialogue_relationship: &Relationship,
        action: &ActionFocus,
    ) {
        let pw = action.praiseworthiness;
        let ctx = &action.description;

        match (&action.agent_id, &action.relationship) {
            (None, _) => {
                // 자기 행동 (Pride, Shame)
                let w = p.praiseworthiness_weight(true, pw);
                Self::add_valence(state, EmotionType::Pride, EmotionType::Shame, pw, w, 1.0, ctx);
            }
            (Some(_), rel) => {
                // 타인 행동 (Admiration, Reproach)
                let relationship = rel.as_ref().unwrap_or(dialogue_relationship);
                let w = p.praiseworthiness_weight(false, pw);
                let modifier = relationship.emotion_intensity_multiplier() * relationship.trust_emotion_modifier();
                Self::add_valence(state, EmotionType::Admiration, EmotionType::Reproach, pw, w, modifier, ctx);
            }
        }
    }

    fn appraise_compound(state: &mut EmotionState, is_self: bool, situation_desc: &str) {
        if is_self {
            Self::add_compound(state, EmotionType::Gratification, EmotionType::Pride, EmotionType::Joy, situation_desc);
            Self::add_compound(state, EmotionType::Remorse, EmotionType::Shame, EmotionType::Distress, situation_desc);
        } else {
            Self::add_compound(state, EmotionType::Gratitude, EmotionType::Admiration, EmotionType::Joy, situation_desc);
            Self::add_compound(state, EmotionType::Anger, EmotionType::Reproach, EmotionType::Distress, situation_desc);
        }
    }

    fn appraise_object<P: AppraisalWeights>(
        p: &P,
        state: &mut EmotionState,
        object: &ObjectFocus,
    ) {
        let ap = object.appealingness;
        let w = p.appealingness_weight(ap);
        let ctx = &object.target_description;
        
        Self::add_valence(state, EmotionType::Love, EmotionType::Hate, ap, w, 1.0, ctx);
    }

    // -----------------------------------------------------------------------
    // 감정 생성 헬퍼 (DRY)
    // -----------------------------------------------------------------------

    /// 밸런스 페어 (Positive/Negative) 감정 추가 헬퍼
    fn add_valence(
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
    fn add_confirmation(
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
    fn add_compound(
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
}

impl crate::ports::Appraiser for AppraisalEngine {
    fn appraise<P: AppraisalWeights>(
        &self,
        personality: &P,
        situation: &Situation,
        relationship: &Relationship,
    ) -> EmotionState {
        AppraisalEngine::appraise(personality, situation, relationship)
    }
}
