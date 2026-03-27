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

use crate::ports::AppraisalWeights;
use crate::domain::relationship::Relationship;

use super::types::{Emotion, EmotionState, EmotionType};
use super::situation::*;

// ---------------------------------------------------------------------------
// 감정 평가 엔진
// ---------------------------------------------------------------------------

/// 도메인 서비스 — 성격(AppraisalWeights) × Relationship → OCC 감정
pub struct AppraisalEngine;

impl AppraisalEngine {

    /// 성격 + 상황 + 관계 → 감정 상태 생성 (상황 진입 시 1회)
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

    // --- Event-based 감정 평가 ---
    fn appraise_event<P: AppraisalWeights>(
        p: &P,
        state: &mut EmotionState,
        rel_mul: f32,
        event: &EventFocus,
    ) {
        let d = event.desirability_for_self;

        // 1. 전망 확인 → Satisfaction / Disappointment / Relief / FearsConfirmed
        if let Some(Prospect::Confirmation(result)) = &event.prospect {
            let w = p.desirability_confirmation_weight(d);
            let base = d.abs() * w * rel_mul;
            match result {
                ProspectResult::HopeFulfilled =>
                    state.add(Emotion::new(EmotionType::Satisfaction, base)),
                ProspectResult::HopeUnfulfilled =>
                    state.add(Emotion::new(EmotionType::Disappointment, base)),
                ProspectResult::FearUnrealized =>
                    state.add(Emotion::new(EmotionType::Relief, base)),
                ProspectResult::FearConfirmed =>
                    state.add(Emotion::new(EmotionType::FearsConfirmed, base)),
            }
            return;
        }

        // 2. 미래 전망 → Hope / Fear
        if let Some(Prospect::Anticipation) = &event.prospect {
            let w = p.desirability_prospect_weight(d);
            if d > 0.0 {
                state.add(Emotion::new(EmotionType::Hope, d * w * rel_mul));
            } else if d < 0.0 {
                state.add(Emotion::new(EmotionType::Fear, d.abs() * w * rel_mul));
            }
            return;
        }

        // 3. 자기 복지 → Joy / Distress
        let w = p.desirability_self_weight(d);
        if d > 0.0 {
            state.add(Emotion::new(EmotionType::Joy, d * w * rel_mul));
        } else if d < 0.0 {
            state.add(Emotion::new(EmotionType::Distress, d.abs() * w * rel_mul));
        }

        // 4. 타인의 운 → HappyFor / Pity / Gloating / Resentment
        if let Some(other) = &event.desirability_for_other {
            let d_other = other.desirability;

            // 공감 채널: HappyFor / Pity
            let emp_w = p.empathy_weight(d_other);
            if emp_w > 0.0 {
                let rel_mod = other.relationship.empathy_rel_modifier();
                if d_other > 0.0 {
                    state.add(Emotion::new(EmotionType::HappyFor, d_other * emp_w * rel_mod));
                } else if d_other < 0.0 {
                    state.add(Emotion::new(EmotionType::Pity, d_other.abs() * emp_w * rel_mod));
                }
            }

            // 적대 채널: Resentment / Gloating
            let hos_w = p.hostility_weight(d_other);
            if hos_w > 0.0 {
                let rel_mod = other.relationship.hostility_rel_modifier();
                if d_other > 0.0 {
                    state.add(Emotion::new(EmotionType::Resentment, d_other * hos_w * rel_mod));
                } else if d_other < 0.0 {
                    state.add(Emotion::new(EmotionType::Gloating, d_other.abs() * hos_w * rel_mod));
                }
            }
        }
    }

    // --- Action-based 감정 평가 ---
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
            // 자기 행동 — trust 무관, rel_mul 무관
            if pw > 0.0 {
                state.add(Emotion::new(EmotionType::Pride, pw * w));
            } else if pw < 0.0 {
                state.add(Emotion::new(EmotionType::Shame, pw.abs() * w));
            }
        } else {
            // 타인 행동 — trust_mod, rel_mul 적용
            if pw > 0.0 {
                state.add(Emotion::new(EmotionType::Admiration, pw * w * trust_mod * rel_mul));
            } else if pw < 0.0 {
                state.add(Emotion::new(EmotionType::Reproach, pw.abs() * w * trust_mod * rel_mul));
            }
        }
    }

    // --- Compound 감정: 이미 계산된 기초 감정값을 결합 ---
    fn appraise_compound(
        state: &mut EmotionState,
        is_self_agent: bool,
    ) {
        if is_self_agent {
            let pride = state.intensity_of(EmotionType::Pride);
            let joy = state.intensity_of(EmotionType::Joy);
            if pride > 0.0 && joy > 0.0 {
                state.add(Emotion::new(EmotionType::Gratification, (pride + joy) / 2.0));
            }
            let shame = state.intensity_of(EmotionType::Shame);
            let distress = state.intensity_of(EmotionType::Distress);
            if shame > 0.0 && distress > 0.0 {
                state.add(Emotion::new(EmotionType::Remorse, (shame + distress) / 2.0));
            }
        } else {
            let admiration = state.intensity_of(EmotionType::Admiration);
            let joy = state.intensity_of(EmotionType::Joy);
            if admiration > 0.0 && joy > 0.0 {
                state.add(Emotion::new(EmotionType::Gratitude, (admiration + joy) / 2.0));
            }
            let reproach = state.intensity_of(EmotionType::Reproach);
            let distress = state.intensity_of(EmotionType::Distress);
            if reproach > 0.0 && distress > 0.0 {
                state.add(Emotion::new(EmotionType::Anger, (reproach + distress) / 2.0));
            }
        }
    }

    // --- Object-based 감정 평가 ---
    fn appraise_object<P: AppraisalWeights>(
        p: &P,
        state: &mut EmotionState,
        rel_mul: f32,
        object: &ObjectFocus,
    ) {
        let ap = object.appealingness;
        let w = p.appealingness_weight(ap);

        if ap > 0.0 {
            state.add(Emotion::new(EmotionType::Love, ap * w * rel_mul));
        } else if ap < 0.0 {
            state.add(Emotion::new(EmotionType::Hate, ap.abs() * w * rel_mul));
        }
    }
}

// ---------------------------------------------------------------------------
// Appraiser 포트 구현
// ---------------------------------------------------------------------------

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
