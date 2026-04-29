// wuxia-core/src/psychology/event.rs
//
// Psychology Domain Events — 심리 도메인에서 발생하는 이벤트들.
//
// 7층 심리 아키텍처의 상태 변화를 추적한다.
// DomainEvent::Psychology(PsychologyEvent)로 감싸져
// Application Service에 전달된다.
//
// 비유: 강호 내면 소식 (江湖內面消息)
//   "명경의 옳음(正) 신조가 흔들렸다!" → CreedChanged
//   "소연이 분노했다!"                 → EmotionGenerated
//   "조고의 야망이 커졌다!"            → PracticalValueChanged

use serde::{Deserialize, Serialize};

use crate::shared::id::CharacterId;

use super::appraisal::ReflectionTier;
use super::emotion::EmotionType;
use super::personality::HexacoFactor;
use super::three_axis::AxisType;
use super::values::PracticalValueType;

/// 심리 도메인에서 발생하는 이벤트들.
///
/// # Example
/// ```
/// use wuxia_core::psychology::PsychologyEvent;
/// use wuxia_core::psychology::PracticalValueType;
/// use wuxia_core::psychology::ReflectionTier;
/// use wuxia_core::shared::id::CharacterId;
///
/// let event = PsychologyEvent::PracticalValueChanged {
///     character_id: CharacterId::new(1),
///     value_type: PracticalValueType::Righteousness,
///     old_value: 80.0,
///     new_value: 85.0,
///     tier: ReflectionTier::Instant,
/// };
/// assert_eq!(event.name(), "PsyValueChanged");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PsychologyEvent {
    // -- ②층: 3축가치관 --

    /// 3축가치관 강도가 변했다.
    ///
    /// "소연의 바람(願) 강도가 50에서 60으로 상승했다"
    AxisIntensityChanged {
        character_id: CharacterId,
        axis: AxisType,
        old_value: f32,
        new_value: f32,
        tier: ReflectionTier,
    },

    /// 3축가치관의 신조가 변했다.
    ///
    /// "명경의 옳음(正) 신조가 '도의를 지켜야 한다'에서 변화했다"
    CreedChanged {
        character_id: CharacterId,
        axis: AxisType,
        old_creed: String,
        new_creed: String,
    },

    /// 3축가치관에 대안 신조 후보가 추가되었다.
    ///
    /// "명경이 새로운 신조 후보에 접촉했다"
    CreedCandidateAdded {
        character_id: CharacterId,
        axis: AxisType,
        candidate_text: String,
    },

    // -- ③층: 5가치 --

    /// 실천 가치가 변했다.
    ///
    /// "소연의 복수(復) 가치가 70에서 80으로 상승했다"
    PracticalValueChanged {
        character_id: CharacterId,
        value_type: PracticalValueType,
        old_value: f32,
        new_value: f32,
        tier: ReflectionTier,
    },

    // -- ①층: HEXACO 성격 --

    /// HEXACO 성격 요소가 변했다 (Tier 4 인생성찰에서만 발생).
    ///
    /// "진야림의 외향성(X)이 30에서 35로 변했다"
    PersonalityChanged {
        character_id: CharacterId,
        factor: HexacoFactor,
        old_value: u32,
        new_value: u32,
    },

    // -- ④층: OCC 감정 --

    /// 새 감정이 생성되었다.
    ///
    /// "소연이 분노(72.5)를 느꼈다"
    EmotionGenerated {
        character_id: CharacterId,
        emotion_type: EmotionType,
        intensity: f32,
    },

    /// 감정이 시간 경과로 감쇠했다.
    EmotionDecayed {
        character_id: CharacterId,
        emotion_type: EmotionType,
        old_intensity: f32,
        new_intensity: f32,
    },

    /// 감정이 임계값 이하로 소멸했다.
    EmotionExpired {
        character_id: CharacterId,
        emotion_type: EmotionType,
    },

    // -- ⑤층: PAD 기분 --

    /// PAD 기분 상태가 변했다.
    MoodChanged {
        character_id: CharacterId,
        old_pleasure: f32,
        old_arousal: f32,
        old_dominance: f32,
        new_pleasure: f32,
        new_arousal: f32,
        new_dominance: f32,
    },

    // -- OCC 인지 평가 --

    /// 인지 평가가 완료되어 감정이 생성되었다.
    AppraisalCompleted {
        character_id: CharacterId,
        emotions_generated: u32,
    },
}

use crate::shared::event_macros::impl_event_name;

impl_event_name!(PsychologyEvent {
    AxisIntensityChanged => "PsyAxisIntensityChanged",
    CreedChanged => "PsyCreedChanged",
    CreedCandidateAdded => "PsyCreedCandidateAdded",
    PracticalValueChanged => "PsyValueChanged",
    PersonalityChanged => "PsyPersonalityChanged",
    EmotionGenerated => "PsyEmotionGenerated",
    EmotionDecayed => "PsyEmotionDecayed",
    EmotionExpired => "PsyEmotionExpired",
    MoodChanged => "PsyMoodChanged",
    AppraisalCompleted => "PsyAppraisalCompleted",
});

#[cfg(test)]
mod tests {
    use super::*;

    fn cid() -> CharacterId {
        CharacterId::new(1)
    }

    #[test]
    fn axis_intensity_changed_name() {
        let event = PsychologyEvent::AxisIntensityChanged {
            character_id: cid(),
            axis: AxisType::Trust,
            old_value: 50.0,
            new_value: 55.0,
            tier: ReflectionTier::Instant,
        };
        assert_eq!(event.name(), "PsyAxisIntensityChanged");
    }

    #[test]
    fn creed_changed_name() {
        let event = PsychologyEvent::CreedChanged {
            character_id: cid(),
            axis: AxisType::Rightness,
            old_creed: "도의를 지켜야 한다".to_string(),
            new_creed: "때로는 유연해야 한다".to_string(),
        };
        assert_eq!(event.name(), "PsyCreedChanged");
    }

    #[test]
    fn practical_value_changed_name() {
        let event = PsychologyEvent::PracticalValueChanged {
            character_id: cid(),
            value_type: PracticalValueType::Vengeance,
            old_value: 70.0,
            new_value: 80.0,
            tier: ReflectionTier::TurningPoint,
        };
        assert_eq!(event.name(), "PsyValueChanged");
    }

    #[test]
    fn personality_changed_name() {
        let event = PsychologyEvent::PersonalityChanged {
            character_id: cid(),
            factor: HexacoFactor::Extraversion,
            old_value: 30,
            new_value: 35,
        };
        assert_eq!(event.name(), "PsyPersonalityChanged");
    }

    #[test]
    fn emotion_generated_name() {
        let event = PsychologyEvent::EmotionGenerated {
            character_id: cid(),
            emotion_type: EmotionType::Anger,
            intensity: 72.5,
        };
        assert_eq!(event.name(), "PsyEmotionGenerated");
    }

    #[test]
    fn mood_changed_name() {
        let event = PsychologyEvent::MoodChanged {
            character_id: cid(),
            old_pleasure: 0.0,
            old_arousal: 0.0,
            old_dominance: 0.0,
            new_pleasure: -0.3,
            new_arousal: 0.5,
            new_dominance: 0.2,
        };
        assert_eq!(event.name(), "PsyMoodChanged");
    }

    #[test]
    fn serialization_roundtrip() {
        let events = vec![
            PsychologyEvent::AxisIntensityChanged {
                character_id: cid(),
                axis: AxisType::Want,
                old_value: 40.0,
                new_value: 50.0,
                tier: ReflectionTier::Daily,
            },
            PsychologyEvent::PersonalityChanged {
                character_id: cid(),
                factor: HexacoFactor::HonestyHumility,
                old_value: 90,
                new_value: 85,
            },
            PsychologyEvent::EmotionGenerated {
                character_id: cid(),
                emotion_type: EmotionType::Fear,
                intensity: 55.0,
            },
            PsychologyEvent::EmotionExpired {
                character_id: cid(),
                emotion_type: EmotionType::Joy,
            },
            PsychologyEvent::AppraisalCompleted {
                character_id: cid(),
                emotions_generated: 3,
            },
        ];
        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let restored: PsychologyEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(event, restored);
        }
    }

    #[test]
    fn clone_and_eq() {
        let a = PsychologyEvent::EmotionGenerated {
            character_id: cid(),
            emotion_type: EmotionType::Anger,
            intensity: 80.0,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }
}
