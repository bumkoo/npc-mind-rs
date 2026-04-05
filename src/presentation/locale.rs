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
    VeryHigh, High, Neutral, Low, VeryLow,
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
    /// 친밀도 수준 번역 (RelationshipLevel variant name → 번역)
    pub closeness_level: HashMap<String, String>,
    /// 신뢰도 수준 번역 (RelationshipLevel variant name → 번역)
    pub trust_level: HashMap<String, String>,
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
    /// 역할 섹션: "[역할]"
    pub section_role: String,
    /// 역할 지시: "당신은 {name}입니다..."
    pub role_instruction: String,
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
    /// 감정 구성: "감정 구성:"
    pub emotion_composition: String,
    /// 지배 감정 라벨: "지배" / "dominant"
    pub dominant_label: String,
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
    /// 관계 섹션 (파트너 없을 때): "[상대와의 관계]"
    pub section_relationship: String,
    /// 관계 섹션 (파트너 있을 때): "[상대와의 관계: {partner_name} — ...]"
    pub section_relationship_with_partner: String,
    /// 친밀도: "친밀도: {level}"
    pub relationship_closeness: String,
    /// 신뢰도: "신뢰도: {level}"
    pub relationship_trust: String,
    /// 상하 관계: "상하 관계: {level}"
    pub relationship_power: String,
    /// 응답 규칙 섹션: "[응답 규칙]"
    pub section_response_rules: String,
    /// 응답 규칙 — 길이
    pub response_rule_length: String,
    /// 응답 규칙 — 반복 금지
    pub response_rule_no_repetition: String,
    /// 응답 규칙 — 대사만
    pub response_rule_dialogue_only: String,
}

// ---------------------------------------------------------------------------
// LocaleBundle 구현
// ---------------------------------------------------------------------------

impl LocaleBundle {
    /// TOML 문자열에서 로케일 번들을 파싱
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    /// base TOML 위에 override TOML을 부분 덮어쓰기하여 번들 생성
    ///
    /// override TOML에 정의된 키만 교체되고, 나머지는 base 값이 유지됩니다.
    /// ```toml
    /// # override.toml — 일부 키만 정의
    /// [emotion]
    /// Anger = "살기"
    /// Joy = "환희"
    /// ```
    pub fn from_toml_with_overrides(base: &str, overrides: &str) -> Result<Self, toml::de::Error> {
        let mut base_val: toml::Value = toml::from_str(base)?;
        let over_val: toml::Value = toml::from_str(overrides)?;
        deep_merge(&mut base_val, over_val);
        base_val.try_into()
    }

    /// 감정 강도 → 라벨 변환
    pub fn intensity_label(&self, intensity: f32) -> &str {
        if intensity >= 0.8 {
            &self.intensity.extreme
        } else if intensity >= 0.6 {
            &self.intensity.strong
        } else if intensity >= 0.4 {
            &self.intensity.noticeable
        } else if intensity >= 0.2 {
            &self.intensity.weak
        } else {
            &self.intensity.faint
        }
    }

    /// 전체 분위기 → 라벨 변환
    pub fn mood_label(&self, mood: f32) -> &str {
        if mood > 0.5 {
            &self.mood.very_positive
        } else if mood > 0.2 {
            &self.mood.positive
        } else if mood > -0.2 {
            &self.mood.neutral
        } else if mood > -0.5 {
            &self.mood.negative
        } else {
            &self.mood.very_negative
        }
    }

    /// VariantName 트레이트를 구현한 enum → 번역 테이블에서 조회하는 제네릭 헬퍼
    ///
    /// 번역이 없으면 variant 이름을 그대로 반환 (폴백).
    fn lookup<'a>(&'a self, map: &'a HashMap<String, String>, item: &dyn VariantName) -> &'a str {
        let key = item.variant_name();
        map.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// EmotionType → 번역된 이름
    pub fn emotion_name(&self, etype: &EmotionType) -> &str {
        self.lookup(&self.emotion, etype)
    }

    /// Tone → 번역된 설명
    pub fn tone_label(&self, tone: &Tone) -> &str {
        self.lookup(&self.tone, tone)
    }

    /// Attitude → 번역된 설명
    pub fn attitude_label(&self, attitude: &Attitude) -> &str {
        self.lookup(&self.attitude, attitude)
    }

    /// BehavioralTendency → 번역된 설명
    pub fn behavioral_tendency_label(&self, bt: &BehavioralTendency) -> &str {
        self.lookup(&self.behavioral_tendency, bt)
    }

    /// Restriction → 번역된 설명
    pub fn restriction_label(&self, r: &Restriction) -> &str {
        self.lookup(&self.restriction, r)
    }

    /// PersonalityTrait → 번역된 설명
    pub fn personality_trait_label(&self, t: &PersonalityTrait) -> &str {
        self.lookup(&self.personality_trait, t)
    }

    /// SpeechStyle → 번역된 설명
    pub fn speech_style_label(&self, s: &SpeechStyle) -> &str {
        self.lookup(&self.speech_style, s)
    }

    /// RelationshipLevel → 친밀도 번역
    pub fn closeness_level_label(&self, level: &RelationshipLevel) -> &str {
        self.lookup(&self.closeness_level, level)
    }

    /// RelationshipLevel → 신뢰도 번역
    pub fn trust_level_label(&self, level: &RelationshipLevel) -> &str {
        self.lookup(&self.trust_level, level)
    }

    /// PowerLevel → 번역된 설명
    pub fn power_level_label(&self, level: &PowerLevel) -> &str {
        self.lookup(&self.power_level, level)
    }

    /// 성격 특성 목록을 번역된 문장으로 조합
    pub fn format_traits(&self, snapshot: &PersonalitySnapshot) -> String {
        if snapshot.traits.is_empty() {
            self.fallback.no_traits.clone()
        } else {
            snapshot
                .traits
                .iter()
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
            snapshot
                .speech_styles
                .iter()
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

// ---------------------------------------------------------------------------
// TOML 값 deep merge
// ---------------------------------------------------------------------------

/// base 위에 over를 재귀적으로 덮어씁니다.
/// Table 끼리는 키별로 병합하고, 그 외 값은 over가 덮어씁니다.
fn deep_merge(base: &mut toml::Value, over: toml::Value) {
    match (base, over) {
        (toml::Value::Table(base_t), toml::Value::Table(over_t)) => {
            for (k, v) in over_t {
                match base_t.get_mut(&k) {
                    Some(existing) => deep_merge(existing, v),
                    None => {
                        base_t.insert(k, v);
                    }
                }
            }
        }
        (base, over) => *base = over,
    }
}
