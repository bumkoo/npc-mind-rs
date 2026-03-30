//! 대사 자극 처리 (Stimulus Processor)
//!
//! 대화 중 대사가 NPC의 기존 감정 강도를 변동시키는 로직.
//! Situation을 재평가하지 않으며, 새 감정을 생성하지 않는다.
//!
//! 설계 원칙:
//! - 엔진은 성격 모델 내부를 모름 (StimulusWeights trait만 의존)
//! - 성격이 "자극 수용도"를 캡슐화하여 반환
//! - 기존 감정의 강도만 변동, 새 감정 생성 없음

use tracing::trace;

use crate::ports::StimulusWeights;
use crate::domain::pad::{Pad, pad_dot, emotion_to_pad};
use crate::domain::tuning::{STIMULUS_IMPACT_RATE, STIMULUS_FADE_THRESHOLD, STIMULUS_MIN_INERTIA};

use super::types::EmotionState;

/// 대사 자극에 의한 감정 변동 처리
///
/// Zero-sized type. `StimulusProcessor` 트레이트를 구현하며,
/// `MindService`에 기본 자극 처리 엔진으로 주입됩니다.
pub struct StimulusEngine;

impl crate::ports::StimulusProcessor for StimulusEngine {
    /// 대사 자극에 의한 감정 변동을 계산합니다.
    ///
    /// 기존 감정의 강도만 변동시키며, 새 감정을 생성하지 않습니다.
    /// - 자극과 같은 방향의 감정 → 증폭
    /// - 자극과 반대 방향의 감정 → 감소
    /// - 0.05 이하로 떨어진 감정 → 자연 소멸
    fn apply_stimulus<P: StimulusWeights>(
        &self,
        personality: &P,
        current_state: &EmotionState,
        stimulus: &Pad,
    ) -> EmotionState {
        let absorb = personality.stimulus_absorb_rate(stimulus);
        trace!(absorb_rate = absorb, pleasure = stimulus.pleasure, arousal = stimulus.arousal, dominance = stimulus.dominance);
        let mut new_state = current_state.clone();

        for emotion in current_state.emotions() {
            let emotion_pad = emotion_to_pad(emotion.emotion_type());
            let alignment = pad_dot(&emotion_pad, stimulus);
            let inertia = (1.0 - emotion.intensity()).max(STIMULUS_MIN_INERTIA);
            let delta = alignment * absorb * STIMULUS_IMPACT_RATE * inertia;
            let old_intensity = emotion.intensity();
            let new_intensity = (old_intensity + delta).clamp(0.0, 1.0);

            if new_intensity < STIMULUS_FADE_THRESHOLD {
                trace!(emotion = ?emotion.emotion_type(), old = old_intensity, delta = delta, result = "faded");
                new_state.remove(emotion.emotion_type());
            } else {
                trace!(emotion = ?emotion.emotion_type(), old = old_intensity, delta = delta, new = new_intensity);
                new_state.set_intensity(emotion.emotion_type(), new_intensity);
            }
        }

        new_state
    }
}
