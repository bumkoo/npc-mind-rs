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
//! - 상황 진입 시 1회 평가 (대화 중 감정 변동은 apply_stimulus가 담당)
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
        let rel_mul = relationship.emotion_intensity_multiplier();

        if let Some(event) = &situation.event {
            Self::appraise_event(personality, &mut state, rel_mul, event);
        }
        if let Some(action) = &situation.action {
            let trust_mod = relationship.trust_emotion_modifier();
            Self::appraise_action(personality, &mut state, rel_mul, trust_mod, action);
        }
        if let Some(object) = &situation.object {
            Self::appraise_object(personality, &mut state, rel_mul, object);
        }
        if let (Some(action), Some(_)) = (&situation.action, &situation.event) {
            Self::appraise_compound(&mut state, action.is_self_agent);
        }

        state
    }

    fn appraise_event<P: AppraisalWeights>(
        p: &P,
        state: &mut EmotionState,
        rel_mul: f32,
        event: &EventFocus,
    ) {
        let d = event.desirability_for_self;

        // 1. 전망 확인
        if let Some(Prospect::Confirmation(result)) = &event.prospect {
            let w = p.desirability_confirmation_weight(d);
            let val = d.abs() * w * rel_mul;
            let etype = match result {
                ProspectResult::HopeFulfilled => EmotionType::Satisfaction,
                ProspectResult::HopeUnfulfilled => EmotionType::Disappointment,
                ProspectResult::FearUnrealized => EmotionType::Relief,
                ProspectResult::FearConfirmed => EmotionType::FearsConfirmed,
            };
            trace!(emotion = ?etype, base_val = d, weight = w, multiplier = rel_mul, result = val);
            state.add(Emotion::new(etype, val));
            return;
        }

        // 2. 미래 전망
        if let Some(Prospect::Anticipation) = &event.prospect {
            let w = p.desirability_prospect_weight(d);
            if d > 0.0 {
                let val = d * w * rel_mul;
                trace!(emotion = ?EmotionType::Hope, base_val = d, weight = w, multiplier = rel_mul, result = val);
                state.add(Emotion::new(EmotionType::Hope, val));
            } else if d < 0.0 {
                let val = d.abs() * w * rel_mul;
                trace!(emotion = ?EmotionType::Fear, base_val = d, weight = w, multiplier = rel_mul, result = val);
                state.add(Emotion::new(EmotionType::Fear, val));
            }
            return;
        }

        // 3. 자기 복지
        let w = p.desirability_self_weight(d);
        if d > 0.0 {
            let val = d * w * rel_mul;
            trace!(emotion = ?EmotionType::Joy, base_val = d, weight = w, multiplier = rel_mul, result = val);
            state.add(Emotion::new(EmotionType::Joy, val));
        } else if d < 0.0 {
            let val = d.abs() * w * rel_mul;
            trace!(emotion = ?EmotionType::Distress, base_val = d, weight = w, multiplier = rel_mul, result = val);
            state.add(Emotion::new(EmotionType::Distress, val));
        }

        // 4. 타인의 운
        if let Some(other) = &event.desirability_for_other {
            let d_other = other.desirability;

            let emp_w = p.empathy_weight(d_other);
            if emp_w > 0.0 {
                let rel_mod = other.relationship.empathy_rel_modifier();
                if d_other > 0.0 {
                    let val = d_other * emp_w * rel_mod;
                    trace!(emotion = ?EmotionType::HappyFor, base_val = d_other, weight = emp_w, multiplier = rel_mod, result = val);
                    state.add(Emotion::new(EmotionType::HappyFor, val));
                } else if d_other < 0.0 {
                    let val = d_other.abs() * emp_w * rel_mod;
                    trace!(emotion = ?EmotionType::Pity, base_val = d_other, weight = emp_w, multiplier = rel_mod, result = val);
                    state.add(Emotion::new(EmotionType::Pity, val));
                }
            }

            let hos_w = p.hostility_weight(d_other);
            if hos_w > 0.0 {
                let rel_mod = other.relationship.hostility_rel_modifier();
                if d_other > 0.0 {
                    let val = d_other * hos_w * rel_mod;
                    trace!(emotion = ?EmotionType::Resentment, base_val = d_other, weight = hos_w, multiplier = rel_mod, result = val);
                    state.add(Emotion::new(EmotionType::Resentment, val));
                } else if d_other < 0.0 {
                    let val = d_other.abs() * hos_w * rel_mod;
                    trace!(emotion = ?EmotionType::Gloating, base_val = d_other, weight = hos_w, multiplier = rel_mod, result = val);
                    state.add(Emotion::new(EmotionType::Gloating, val));
                }
            }
        }
    }

    fn appraise_action<P: AppraisalWeights>(
        p: &P,
        state: &mut EmotionState,
        rel_mul: f32,
        trust_mod: f32,
        action: &ActionFocus,
    ) {
        let pw = action.praiseworthiness;
        let w = p.praiseworthiness_weight(action.is_self_agent, pw);

        if action.is_self_agent {
            if pw > 0.0 {
                let val = pw * w;
                trace!(emotion = ?EmotionType::Pride, base_val = pw, weight = w, result = val);
                state.add(Emotion::new(EmotionType::Pride, val));
            } else if pw < 0.0 {
                let val = pw.abs() * w;
                trace!(emotion = ?EmotionType::Shame, base_val = pw, weight = w, result = val);
                state.add(Emotion::new(EmotionType::Shame, val));
            }
        } else {
            if pw > 0.0 {
                let val = pw * w * trust_mod * rel_mul;
                trace!(emotion = ?EmotionType::Admiration, base_val = pw, weight = w, multiplier = rel_mul, trust_mod = trust_mod, result = val);
                state.add(Emotion::new(EmotionType::Admiration, val));
            } else if pw < 0.0 {
                let val = pw.abs() * w * trust_mod * rel_mul;
                trace!(emotion = ?EmotionType::Reproach, base_val = pw, weight = w, multiplier = rel_mul, trust_mod = trust_mod, result = val);
                state.add(Emotion::new(EmotionType::Reproach, val));
            }
        }
    }

    fn appraise_compound(
        state: &mut EmotionState,
        is_self_agent: bool,
    ) {
        if is_self_agent {
            let pride = state.intensity_of(EmotionType::Pride);
            let joy = state.intensity_of(EmotionType::Joy);
            if pride > 0.0 && joy > 0.0 {
                let val = (pride + joy) / 2.0;
                trace!(emotion = ?EmotionType::Gratification, comp1_type = ?EmotionType::Pride, comp1_val = pride, comp2_type = ?EmotionType::Joy, comp2_val = joy, result = val);
                state.add(Emotion::new(EmotionType::Gratification, val));
            }
            let shame = state.intensity_of(EmotionType::Shame);
            let distress = state.intensity_of(EmotionType::Distress);
            if shame > 0.0 && distress > 0.0 {
                let val = (shame + distress) / 2.0;
                trace!(emotion = ?EmotionType::Remorse, comp1_type = ?EmotionType::Shame, comp1_val = shame, comp2_type = ?EmotionType::Distress, comp2_val = distress, result = val);
                state.add(Emotion::new(EmotionType::Remorse, val));
            }
        } else {
            let admiration = state.intensity_of(EmotionType::Admiration);
            let joy = state.intensity_of(EmotionType::Joy);
            if admiration > 0.0 && joy > 0.0 {
                let val = (admiration + joy) / 2.0;
                trace!(emotion = ?EmotionType::Gratitude, comp1_type = ?EmotionType::Admiration, comp1_val = admiration, comp2_type = ?EmotionType::Joy, comp2_val = joy, result = val);
                state.add(Emotion::new(EmotionType::Gratitude, val));
            }
            let reproach = state.intensity_of(EmotionType::Reproach);
            let distress = state.intensity_of(EmotionType::Distress);
            if reproach > 0.0 && distress > 0.0 {
                let val = (reproach + distress) / 2.0;
                trace!(emotion = ?EmotionType::Anger, comp1_type = ?EmotionType::Reproach, comp1_val = reproach, comp2_type = ?EmotionType::Distress, comp2_val = distress, result = val);
                state.add(Emotion::new(EmotionType::Anger, val));
            }
        }
    }

    fn appraise_object<P: AppraisalWeights>(
        p: &P,
        state: &mut EmotionState,
        rel_mul: f32,
        object: &ObjectFocus,
    ) {
        let ap = object.appealingness;
        let w = p.appealingness_weight(ap);

        if ap > 0.0 {
            let val = ap * w * rel_mul;
            trace!(emotion = ?EmotionType::Love, base_val = ap, weight = w, multiplier = rel_mul, result = val);
            state.add(Emotion::new(EmotionType::Love, val));
        } else if ap < 0.0 {
            let val = ap.abs() * w * rel_mul;
            trace!(emotion = ?EmotionType::Hate, base_val = ap, weight = w, multiplier = rel_mul, result = val);
            state.add(Emotion::new(EmotionType::Hate, val));
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
