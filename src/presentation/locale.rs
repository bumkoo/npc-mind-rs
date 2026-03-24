//! 로케일 번들 — TOML 파일에서 다국어 텍스트를 로드하는 구조체
//!
//! enum variant 이름을 TOML 키로 사용하여 HashMap으로 매핑한다.
//! 새로운 enum variant가 추가되면 TOML 파일만 업데이트하면 된다.

use std::collections::HashMap;

use serde::Deserialize;

use crate::domain::emotion::EmotionType;
use crate::domain::guide::*;

// ---------------------------------------------------------------------------
// VariantName 트레이트 — enum variant → &'static str (컴파일 타임)
// ---------------------------------------------------------------------------

/// enum variant의 이름을 &'static str로 반환하는 트레이트
pub trait VariantName {
    fn variant_name(&self) -> &'static str;
}

/// enum variant 이름 매핑 매크로 — stringify!로 컴파일 타임에 &'static str 생성
macro_rules! impl_variant_name {
    ($type:ty, { $($variant:ident),* $(,)? }) => {
        impl VariantName for $type {
            fn variant_name(&self) -> &'static str {
                match self {
                    $(Self::$variant => stringify!($variant),)*
                }
            }
        }
    };
}

impl_variant_name!(EmotionType, {
    Joy, Distress, HappyFor, Pity, Gloating, Resentment,
    Hope, Fear, Satisfaction, Disappointment, Relief, FearsConfirmed,
    Pride, Shame, Admiration, Reproach, Gratification, Remorse,
    Gratitude, Anger, Love, Hate,
});

impl_variant_name!(Tone, {
    SuppressedCold, RoughAggressive, AnxiousTrembling, SomberRestrained,
    BrightLively, VigilantCalm, TenseAnxious, ShrinkingSmall,
    QuietConfidence, ProudArrogant, CynicalCritical, DeepSighing,
    SincerelyWarm, JealousBitter, CompassionateSoft, RelaxedGentle,
    Heavy, Calm,
});

impl_variant_name!(Attitude, {
    HostileAggressive, SuppressedDiscomfort, Judgmental,
    GuardedDefensive, FriendlyOpen, DefensiveClosed, NeutralObservant,
});

impl_variant_name!(BehavioralTendency, {
    ImmediateConfrontation, StrategicResponse, ExpressAndObserve,
    BraveConfrontation, SeekSafety, AvoidOrDeflect,
    ActiveCooperation, ObserveAndRespond,
});

impl_variant_name!(Restriction, {
    NoHumorOrLightTone, NoFriendliness, NoSelfJustification,
    NoBravado, NoLyingOrExaggeration,
});

impl_variant_name!(RelationshipLevel, {
    VeryHigh, High, Neutral, Low, VeryLow,
});

impl_variant_name!(PowerLevel, {
    Superior, Equal, Subordinate,
});

impl_variant_name!(PersonalityTrait, {
    HonestAndModest, CunningAndAmbitious, EmotionalAndAnxious,
    BoldAndIndependent, ConfidentAndSociable, IntrovertedAndQuiet,
    TolerantAndGentle, GrudgingAndCritical, SystematicAndDiligent,
    FreeAndImpulsive, CuriousAndCreative, TraditionalAndConservative,
});

impl_variant_name!(SpeechStyle, {
    FrankAndUnadorned, HidesInnerThoughts, ExpressiveAndWorried,
    CalmAndComposed, ActiveAndForceful, BriefAndConcise,
    SoftAndConsiderate, SharpAndDirect, LogicalAndRational,
    UnfilteredAndSpontaneous, MetaphoricalAndUnique, FormalAndTraditional,
});

// ---------------------------------------------------------------------------
// 로케일 번들 구조체
// ---------------------------------------------------------------------------

/// 로케일 번들 — 하나의 TOML 파일에서 로드된 전체 번역 데이터
#[derive(Debug, Deserialize)]
pub struct LocaleBundle {
    /// 언어 메타데이터
    pub meta: Meta,
    /// 감정 강도 라벨
    pub intensity: IntensityLabels,
    /// 전체 분위기 라벨
    pub mood: MoodLabels,
    /// 감정 유형 번역 (EmotionType variant name → 번역)
    pub emotion: HashMap<String, String>,
    /// 어조 번역 (Tone variant name → 번역)
    pub tone: HashMap<String, String>,
    /// 태도 번역 (Attitude variant name → 번역)
    pub attitude: HashMap<String, String>,
    /// 행동 경향 번역 (BehavioralTendency variant name → 번역)
    pub behavioral_tendency: HashMap<String, String>,
    /// 금지 사항 번역 (Restriction variant name → 번역)
    pub restriction: HashMap<String, String>,
    /// 성격 특성 번역 (PersonalityTrait variant name → 번역)
    pub personality_trait: HashMap<String, String>,
    /// 말투 스타일 번역 (SpeechStyle variant name → 번역)
    pub speech_style: HashMap<String, String>,
    /// 관계 수준 번역 (RelationshipLevel variant name → 번역)
    pub relationship_level: HashMap<String, String>,
    /// 상하 관계 수준 번역 (PowerLevel variant name → 번역)
    pub power_level: HashMap<String, String>,
    /// 폴백 텍스트 (특성/말투가 없을 때)
    pub fallback: FallbackLabels,
    /// 프롬프트 템플릿 (섹션 헤더, 포맷 패턴)
    pub template: TemplateStrings,
}

/// 언어 메타데이터
#[derive(Debug, Deserialize)]
pub struct Meta {
    /// 언어 코드 (예: "ko", "en")
    pub language: String,
    /// 언어 이름 (예: "한국어", "English")
    pub name: String,
}

/// 감정 강도 라벨 — f32 값을 정성적 표현으로 변환
#[derive(Debug, Deserialize)]
pub struct IntensityLabels {
    /// >= 0.8: 극도로 강한
    pub extreme: String,
    /// >= 0.6: 강한
    pub strong: String,
    /// >= 0.4: 뚜렷한
    pub noticeable: String,
    /// >= 0.2: 약한
    pub weak: String,
    /// < 0.2: 미미한
    pub faint: String,
}

/// 전체 분위기 라벨 — mood f32 값을 정성적 표현으로 변환
#[derive(Debug, Deserialize)]
pub struct MoodLabels {
    /// > 0.5: 매우 긍정적
    pub very_positive: String,
    /// > 0.2: 긍정적
    pub positive: String,
    /// > -0.2: 중립적
    pub neutral: String,
    /// > -0.5: 부정적
    pub negative: String,
    /// <= -0.5: 매우 부정적
    pub very_negative: String,
}

/// 폴백 텍스트 — 특성/말투가 없을 때 사용
#[derive(Debug, Deserialize)]
pub struct FallbackLabels {
    /// 성격 특성이 없을 때: "평범하고 특별히 두드러지는 성격 특성이 없다."
    pub no_traits: String,
    /// 말투 특성이 없을 때: "평범한 어조로 말한다."
    pub no_speech_style: String,
}

/// 프롬프트 템플릿 — LLM 프롬프트의 골격
///
/// `{name}`, `{emotion}`, `{intensity}` 등은 런타임에 치환되는 플레이스홀더.
#[derive(Debug, Deserialize)]
pub struct TemplateStrings {
    /// NPC 이름 섹션: "[NPC: {name}]"
    pub section_npc: String,
    /// 성격 섹션: "[성격]"
    pub section_personality: String,
    /// 감정 섹션: "[현재 감정]"
    pub section_emotion: String,
    /// 상황 섹션: "[상황]"
    pub section_situation: String,
    /// 연기 지시 섹션: "[연기 지시]"
    pub section_directive: String,
    /// 말투 섹션: "[말투]"
    pub section_speech: String,
    /// 금지 사항 섹션: "[금지 사항]"
    pub section_restriction: String,
    /// 지배 감정: "지배 감정: {emotion}({intensity})"
    pub dominant_emotion: String,
    /// 활성 감정: "활성 감정: {list}"
    pub active_emotions: String,
    /// 전체 분위기: "전체 분위기: {mood}"
    pub overall_mood: String,
    /// 어조: "어조: {tone}"
    pub directive_tone: String,
    /// 태도: "태도: {attitude}"
    pub directive_attitude: String,
    /// 행동 경향: "행동 경향: {behavior}"
    pub directive_behavior: String,
    /// 금지 사항 항목: "- {restriction}"
    pub restriction_item: String,
    /// 관계 섹션: "[상대와의 관계]"
    pub section_relationship: String,
    /// 친밀도: "친밀도: {level}"
    pub relationship_closeness: String,
    /// 신뢰도: "신뢰도: {level}"
    pub relationship_trust: String,
    /// 상하 관계: "상하 관계: {level}"
    pub relationship_power: String,
}

// ---------------------------------------------------------------------------
// LocaleBundle 구현
// ---------------------------------------------------------------------------

impl LocaleBundle {
    /// TOML 문자열에서 로케일 번들을 파싱
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    /// 감정 강도 → 라벨 변환
    pub fn intensity_label(&self, intensity: f32) -> &str {
        if intensity >= 0.8 { &self.intensity.extreme }
        else if intensity >= 0.6 { &self.intensity.strong }
        else if intensity >= 0.4 { &self.intensity.noticeable }
        else if intensity >= 0.2 { &self.intensity.weak }
        else { &self.intensity.faint }
    }

    /// 전체 분위기 → 라벨 변환
    pub fn mood_label(&self, mood: f32) -> &str {
        if mood > 0.5 { &self.mood.very_positive }
        else if mood > 0.2 { &self.mood.positive }
        else if mood > -0.2 { &self.mood.neutral }
        else if mood > -0.5 { &self.mood.negative }
        else { &self.mood.very_negative }
    }

    /// EmotionType → 번역된 이름
    pub fn emotion_name(&self, etype: &EmotionType) -> &str {
        let key = etype.variant_name();
        self.emotion.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// Tone → 번역된 설명
    pub fn tone_label(&self, tone: &Tone) -> &str {
        let key = tone.variant_name();
        self.tone.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// Attitude → 번역된 설명
    pub fn attitude_label(&self, attitude: &Attitude) -> &str {
        let key = attitude.variant_name();
        self.attitude.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// BehavioralTendency → 번역된 설명
    pub fn behavioral_tendency_label(&self, bt: &BehavioralTendency) -> &str {
        let key = bt.variant_name();
        self.behavioral_tendency.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// Restriction → 번역된 설명
    pub fn restriction_label(&self, r: &Restriction) -> &str {
        let key = r.variant_name();
        self.restriction.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// PersonalityTrait → 번역된 설명
    pub fn personality_trait_label(&self, t: &PersonalityTrait) -> &str {
        let key = t.variant_name();
        self.personality_trait.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// SpeechStyle → 번역된 설명
    pub fn speech_style_label(&self, s: &SpeechStyle) -> &str {
        let key = s.variant_name();
        self.speech_style.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// RelationshipLevel → 번역된 설명
    pub fn relationship_level_label(&self, level: &RelationshipLevel) -> &str {
        let key = level.variant_name();
        self.relationship_level.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// PowerLevel → 번역된 설명
    pub fn power_level_label(&self, level: &PowerLevel) -> &str {
        let key = level.variant_name();
        self.power_level.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// 성격 특성 목록을 번역된 문장으로 조합
    pub fn format_traits(&self, snapshot: &PersonalitySnapshot) -> String {
        if snapshot.traits.is_empty() {
            self.fallback.no_traits.clone()
        } else {
            snapshot.traits.iter()
                .map(|t| self.personality_trait_label(t))
                .collect::<Vec<_>>()
                .join(" ")
        }
    }

    /// 말투 목록을 번역된 문장으로 조합
    pub fn format_speech_styles(&self, snapshot: &PersonalitySnapshot) -> String {
        if snapshot.speech_styles.is_empty() {
            self.fallback.no_speech_style.clone()
        } else {
            snapshot.speech_styles.iter()
                .map(|s| self.speech_style_label(s))
                .collect::<Vec<_>>()
                .join(" ")
        }
    }

    /// 템플릿 문자열의 플레이스홀더를 치환
    pub fn render_template(&self, template: &str, vars: &[(&str, &str)]) -> String {
        let mut result = template.to_string();
        for (key, value) in vars {
            result = result.replace(&format!("{{{}}}", key), value);
        }
        result
    }
}
