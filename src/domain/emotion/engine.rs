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

        // 1. 전망 확인
        if let Some(Prospect::Confirmation(result)) = &event.prospect {
            let w = p.desirability_confirmation_weight(d);
            let val = d.abs() * w;
            let etype = match result {
                ProspectResult::HopeFulfilled => EmotionType::Satisfaction,
                ProspectResult::HopeUnfulfilled => EmotionType::Disappointment,
                ProspectResult::FearUnrealized => EmotionType::Relief,
                ProspectResult::FearConfirmed => EmotionType::FearsConfirmed,
            };
            trace!(emotion = ?etype, base_val = d, weight = w, result = val, context = %ctx);
            state.add(Emotion::with_context(etype, val, ctx));
            return;
        }

        // 2. 미래 전망
        if let Some(Prospect::Anticipation) = &event.prospect {
            let w = p.desirability_prospect_weight(d);
            if d > 0.0 {
                let val = d * w;
                trace!(emotion = ?EmotionType::Hope, base_val = d, weight = w, result = val, context = %ctx);
                state.add(Emotion::with_context(EmotionType::Hope, val, ctx));
            } else if d < 0.0 {
                let val = d.abs() * w;
                trace!(emotion = ?EmotionType::Fear, base_val = d, weight = w, result = val, context = %ctx);
                state.add(Emotion::with_context(EmotionType::Fear, val, ctx));
            }
            return;
        }

        // 3. 자기 복지
        let w = p.desirability_self_weight(d);
        if d > 0.0 {
            let val = d * w;
            trace!(emotion = ?EmotionType::Joy, base_val = d, weight = w, result = val, context = %ctx);
            state.add(Emotion::with_context(EmotionType::Joy, val, ctx));
        } else if d < 0.0 {
            let val = d.abs() * w;
            trace!(emotion = ?EmotionType::Distress, base_val = d, weight = w, result = val, context = %ctx);
            state.add(Emotion::with_context(EmotionType::Distress, val, ctx));
        }

        // 4. 타인의 운
        if let Some(other) = &event.desirability_for_other {
            let d_other = other.desirability;
            let other_ctx = format!("{} (대상: {})", ctx, other.target_id);

            let emp_w = p.empathy_weight(d_other);
            if emp_w > 0.0 {
                let rel_mod = other.relationship.empathy_rel_modifier();
                if d_other > 0.0 {
                    let val = d_other * emp_w * rel_mod;
                    trace!(emotion = ?EmotionType::HappyFor, base_val = d_other, weight = emp_w, multiplier = rel_mod, result = val, context = %other_ctx);
                    state.add(Emotion::with_context(EmotionType::HappyFor, val, &other_ctx));
                } else if d_other < 0.0 {
                    let val = d_other.abs() * emp_w * rel_mod;
                    trace!(emotion = ?EmotionType::Pity, base_val = d_other, weight = emp_w, multiplier = rel_mod, result = val, context = %other_ctx);
                    state.add(Emotion::with_context(EmotionType::Pity, val, &other_ctx));
                }
            }

            let hos_w = p.hostility_weight(d_other);
            if hos_w > 0.0 {
                let rel_mod = other.relationship.hostility_rel_modifier();
                if d_other > 0.0 {
                    let val = d_other * hos_w * rel_mod;
                    trace!(emotion = ?EmotionType::Resentment, base_val = d_other, weight = hos_w, multiplier = rel_mod, result = val, context = %other_ctx);
                    state.add(Emotion::with_context(EmotionType::Resentment, val, &other_ctx));
                } else if d_other < 0.0 {
                    let val = d_other.abs() * hos_w * rel_mod;
                    trace!(emotion = ?EmotionType::Gloating, base_val = d_other, weight = hos_w, multiplier = rel_mod, result = val, context = %other_ctx);
                    state.add(Emotion::with_context(EmotionType::Gloating, val, &other_ctx));
                }
            }
        }
    }

    fn appraise_action<P: AppraisalWeights>(
        p: &P,
        state: &mut EmotionState,
        dialogue_relationship: &Relationship,
        action: &ActionFocus,
    ) {
        let ctx = &action.description;

        match (&action.agent_id, &action.relationship) {
            (None, _) => {
                let pw = action.praiseworthiness;
                let w = p.praiseworthiness_weight(true, pw);
                if pw > 0.0 {
                    let val = pw * w;
                    trace!(emotion = ?EmotionType::Pride, base_val = pw, weight = w, result = val, context = %ctx);
                    state.add(Emotion::with_context(EmotionType::Pride, val, ctx));
                } else if pw < 0.0 {
                    let val = pw.abs() * w;
                    trace!(emotion = ?EmotionType::Shame, base_val = pw, weight = w, result = val, context = %ctx);
                    state.add(Emotion::with_context(EmotionType::Shame, val, ctx));
                }
            }
            (Some(_), Some(third_party_rel)) => {
                let rel_mul = third_party_rel.emotion_intensity_multiplier();
                let trust_mod = third_party_rel.trust_emotion_modifier();
                Self::apply_other_action(p, state, action, rel_mul, trust_mod);
            }
            (Some(_), None) => {
                let rel_mul = dialogue_relationship.emotion_intensity_multiplier();
                let trust_mod = dialogue_relationship.trust_emotion_modifier();
                Self::apply_other_action(p, state, action, rel_mul, trust_mod);
            }
        }
    }

    fn apply_other_action<P: AppraisalWeights>(
        p: &P,
        state: &mut EmotionState,
        action: &ActionFocus,
        rel_mul: f32,
        trust_mod: f32,
    ) {
        let pw = action.praiseworthiness;
        let w = p.praiseworthiness_weight(false, pw);
        let ctx = &action.description;
        if pw > 0.0 {
            let val = pw * w * trust_mod * rel_mul;
            trace!(emotion = ?EmotionType::Admiration, base_val = pw, weight = w, rel_mul = rel_mul, trust_mod = trust_mod, result = val, context = %ctx);
            state.add(Emotion::with_context(EmotionType::Admiration, val, ctx));
        } else if pw < 0.0 {
            let val = pw.abs() * w * trust_mod * rel_mul;
            trace!(emotion = ?EmotionType::Reproach, base_val = pw, weight = w, rel_mul = rel_mul, trust_mod = trust_mod, result = val, context = %ctx);
            state.add(Emotion::with_context(EmotionType::Reproach, val, ctx));
        }
    }

    fn appraise_compound(state: &mut EmotionState, is_self: bool, situation_desc: &str) {
        if is_self {
            let pride = state.intensity_of(EmotionType::Pride);
            let joy = state.intensity_of(EmotionType::Joy);
            if pride > 0.0 && joy > 0.0 {
                let val = (pride + joy) / 2.0;
                trace!(emotion = ?EmotionType::Gratification, comp1_type = ?EmotionType::Pride, comp1_val = pride, comp2_type = ?EmotionType::Joy, comp2_val = joy, result = val);
                state.add(Emotion::with_context(EmotionType::Gratification, val, situation_desc));
            }
            let shame = state.intensity_of(EmotionType::Shame);
            let distress = state.intensity_of(EmotionType::Distress);
            if shame > 0.0 && distress > 0.0 {
                let val = (shame + distress) / 2.0;
                trace!(emotion = ?EmotionType::Remorse, comp1_type = ?EmotionType::Shame, comp1_val = shame, comp2_type = ?EmotionType::Distress, comp2_val = distress, result = val);
                state.add(Emotion::with_context(EmotionType::Remorse, val, situation_desc));
            }
        } else {
            let admiration = state.intensity_of(EmotionType::Admiration);
            let joy = state.intensity_of(EmotionType::Joy);
            if admiration > 0.0 && joy > 0.0 {
                let val = (admiration + joy) / 2.0;
                trace!(emotion = ?EmotionType::Gratitude, comp1_type = ?EmotionType::Admiration, comp1_val = admiration, comp2_type = ?EmotionType::Joy, comp2_val = joy, result = val);
                state.add(Emotion::with_context(EmotionType::Gratitude, val, situation_desc));
            }
            let reproach = state.intensity_of(EmotionType::Reproach);
            let distress = state.intensity_of(EmotionType::Distress);
            if reproach > 0.0 && distress > 0.0 {
                let val = (reproach + distress) / 2.0;
                trace!(emotion = ?EmotionType::Anger, comp1_type = ?EmotionType::Reproach, comp1_val = reproach, comp2_type = ?EmotionType::Distress, comp2_val = distress, result = val);
                state.add(Emotion::with_context(EmotionType::Anger, val, situation_desc));
            }
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

        if ap > 0.0 {
            let val = ap * w;
            trace!(emotion = ?EmotionType::Love, base_val = ap, weight = w, result = val, context = %ctx);
            state.add(Emotion::with_context(EmotionType::Love, val, ctx));
        } else if ap < 0.0 {
            let val = ap.abs() * w;
            trace!(emotion = ?EmotionType::Hate, base_val = ap, weight = w, result = val, context = %ctx);
            state.add(Emotion::with_context(EmotionType::Hate, val, ctx));
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
