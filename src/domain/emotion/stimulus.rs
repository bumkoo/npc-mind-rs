//! 대사 자극 처리 (Stimulus Processor)
//!
//! 대화 중 대사가 NPC의 기존 감정 강도를 변동시키는 로직.
//! Situation을 재평가하지 않으며, 새 감정을 생성하지 않는다.
//!
//! 핵심 함수 3개, 30줄 미만:
//! - apply_stimulus: 루프 + delta 적용
//! - pad_dot: 단순 내적 (pad 모듈에서 가져옴)
//! - stimulus_absorb_rate: HEXACO 자극 수용도

use crate::domain::pad::{Pad, pad_dot, emotion_to_pad};
use crate::domain::personality::HexacoProfile;

use super::types::EmotionState;

/// 한 턴의 감정 변동량 제한 계수
const IMPACT_RATE: f32 = 0.1;
/// 감정 자연 소멸 기준 (이 이하면 제거)
const FADE_THRESHOLD: f32 = 0.05;

// ---------------------------------------------------------------------------
// 대사 자극 처리 엔진
// ---------------------------------------------------------------------------

/// 대사 자극에 의한 감정 변동 처리
pub struct StimulusEngine;

impl StimulusEngine {
    /// 대사 자극에 의한 감정 변동을 계산합니다.
    ///
    /// 기존 감정의 강도만 변동시키며, 새 감정을 생성하지 않습니다.
    /// - 자극과 같은 방향의 감정 → 증폭
    /// - 자극과 반대 방향의 감정 → 감소
    /// - 0.05 이하로 떨어진 감정 → 자연 소멸
    pub fn apply_stimulus(
        personality: &HexacoProfile,
        current_state: &EmotionState,
        stimulus: &Pad,
    ) -> EmotionState {
        let absorb = Self::stimulus_absorb_rate(personality, stimulus);
        let mut new_state = current_state.clone();

        // 리팩토링: emotions()가 Vec<Emotion>을 반환하므로 직접 순회합니다.
        for emotion in current_state.emotions() {
            let emotion_pad = emotion_to_pad(emotion.emotion_type());
            let alignment = pad_dot(&emotion_pad, stimulus);
            let delta = alignment * absorb * IMPACT_RATE;
            let new_intensity = (emotion.intensity() + delta).clamp(0.0, 1.0);

            if new_intensity < FADE_THRESHOLD {
                new_state.remove(emotion.emotion_type());
            } else {
                new_state.set_intensity(emotion.emotion_type(), new_intensity);
            }
        }

        new_state
    }

    /// HEXACO 성격에 기반한 자극 수용도(Absorb Rate)를 계산합니다.
    ///
    /// - E(정서성): 전반적 민감도 (높으면 자극을 더 크게 수용)
    /// - A.patience(인내심): 부정 자극 완충 (높으면 부정 자극을 걸러냄)
    /// - C.prudence(신중함): 감정 급변 억제 (높으면 변동 폭이 작아짐)
    fn stimulus_absorb_rate(p: &HexacoProfile, stimulus: &Pad) -> f32 {
        let avg = p.dimension_averages();
        let mut rate = 1.0;
        
        // 리팩토링: avg.e가 Score 타입이므로 intensity()를 사용하여 절대 강도를 가져옵니다.
        rate += avg.e.intensity() * 0.3;                             // E: 민감도
        if stimulus.pleasure < 0.0 {
            rate -= p.agreeableness.patience.value().max(0.0) * 0.4; // A: 부정 완충
        }
        rate -= p.conscientiousness.prudence.value().max(0.0) * 0.3; // C: 급변 억제
        rate.max(0.1) // 완전 무시 방지
    }
}

// ---------------------------------------------------------------------------
// StimulusProcessor 포트 구현
// ---------------------------------------------------------------------------

impl crate::ports::StimulusProcessor for StimulusEngine {
    fn apply_stimulus(
        &self,
        personality: &HexacoProfile,
        current_state: &EmotionState,
        stimulus: &Pad,
    ) -> EmotionState {
        StimulusEngine::apply_stimulus(personality, current_state, stimulus)
    }
}
