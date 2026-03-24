//! 한국어 연기 가이드 포맷터
//!
//! 도메인의 구조화된 가이드 데이터를 한국어 텍스트/JSON으로 변환한다.
//! 다른 언어 포맷터를 추가하려면 GuideFormatter 트레이트를 구현하면 된다.

use serde::Serialize;

use crate::domain::emotion::EmotionType;
use crate::domain::guide::*;
use crate::ports::GuideFormatter;

/// 한국어 연기 가이드 포맷터
pub struct KoreanFormatter;

impl KoreanFormatter {
    /// 감정 강도의 정성적 표현
    pub fn intensity_label(intensity: f32) -> &'static str {
        if intensity >= 0.8 { "극도로 강한" }
        else if intensity >= 0.6 { "강한" }
        else if intensity >= 0.4 { "뚜렷한" }
        else if intensity >= 0.2 { "약한" }
        else { "미미한" }
    }

    /// 감정 유형의 한글 이름
    pub fn emotion_name(etype: &EmotionType) -> &'static str {
        match etype {
            EmotionType::Joy => "기쁨",
            EmotionType::Distress => "고통",
            EmotionType::HappyFor => "대리기쁨",
            EmotionType::Pity => "동정",
            EmotionType::Gloating => "고소함",
            EmotionType::Resentment => "시기",
            EmotionType::Hope => "희망",
            EmotionType::Fear => "두려움",
            EmotionType::Satisfaction => "만족",
            EmotionType::Disappointment => "실망",
            EmotionType::Relief => "안도",
            EmotionType::FearsConfirmed => "공포확인",
            EmotionType::Pride => "자부심",
            EmotionType::Shame => "수치심",
            EmotionType::Admiration => "감탄",
            EmotionType::Reproach => "비난",
            EmotionType::Gratification => "뿌듯함",
            EmotionType::Remorse => "후회",
            EmotionType::Gratitude => "감사",
            EmotionType::Anger => "분노",
            EmotionType::Love => "호감",
            EmotionType::Hate => "혐오",
        }
    }

    /// 어조 한국어 라벨
    pub fn tone_label(tone: &Tone) -> &'static str {
        match tone {
            Tone::SuppressedCold => "억누른 분노가 느껴지는 차가운 어조",
            Tone::RoughAggressive => "거칠고 공격적인 어조",
            Tone::AnxiousTrembling => "불안하고 떨리는 어조",
            Tone::SomberRestrained => "침울하지만 절제된 어조",
            Tone::BrightLively => "밝고 활기찬 어조",
            Tone::VigilantCalm => "경계하는 듯하지만 침착한 어조",
            Tone::TenseAnxious => "긴장되고 불안한 어조",
            Tone::ShrinkingSmall => "움츠러들고 작아진 어조",
            Tone::QuietConfidence => "조용한 자신감이 묻어나는 어조",
            Tone::ProudArrogant => "자랑스럽고 거만한 어조",
            Tone::CynicalCritical => "냉소적이고 비판적인 어조",
            Tone::DeepSighing => "깊은 한숨이 섞인 어조",
            Tone::SincerelyWarm => "진심 어린 따뜻한 어조",
            Tone::JealousBitter => "질투가 묻어나는 씁쓸한 어조",
            Tone::CompassionateSoft => "안타까움이 담긴 부드러운 어조",
            Tone::RelaxedGentle => "편안하고 온화한 어조",
            Tone::Heavy => "무거운 어조",
            Tone::Calm => "담담한 어조",
        }
    }

    /// 태도 한국어 라벨
    pub fn attitude_label(attitude: &Attitude) -> &'static str {
        match attitude {
            Attitude::HostileAggressive => "적대적이고 공격적인 태도",
            Attitude::SuppressedDiscomfort => "불만을 억누르지만 불편함이 드러나는 태도",
            Attitude::Judgmental => "상대를 판단하고 평가하는 태도",
            Attitude::GuardedDefensive => "경계하며 방어적인 태도",
            Attitude::FriendlyOpen => "호의적이고 개방적인 태도",
            Attitude::DefensiveClosed => "방어적이고 닫힌 태도",
            Attitude::NeutralObservant => "중립적이고 관망하는 태도",
        }
    }

    /// 행동 경향 한국어 라벨
    pub fn behavioral_tendency_label(bt: &BehavioralTendency) -> &'static str {
        match bt {
            BehavioralTendency::ImmediateConfrontation => "즉각적으로 맞서고 행동으로 옮기려 한다.",
            BehavioralTendency::StrategicResponse => "분노를 억누르고 계획적으로 대응하려 한다.",
            BehavioralTendency::ExpressAndObserve => "화를 표출하되, 상황을 지켜본다.",
            BehavioralTendency::BraveConfrontation => "두려움에도 불구하고 맞서려 한다.",
            BehavioralTendency::SeekSafety => "위험을 피하고 안전을 확보하려 한다.",
            BehavioralTendency::AvoidOrDeflect => "화제를 돌리거나 자리를 피하려 한다.",
            BehavioralTendency::ActiveCooperation => "대화에 적극적으로 참여하고 협조한다.",
            BehavioralTendency::ObserveAndRespond => "상황을 관찰하며 필요한 만큼만 반응한다.",
        }
    }

    /// 금지 사항 한국어 라벨
    pub fn restriction_label(r: &Restriction) -> &'static str {
        match r {
            Restriction::NoHumorOrLightTone => "농담이나 가벼운 말투를 사용하지 않는다.",
            Restriction::NoFriendliness => "상대에게 호의적으로 대하지 않는다.",
            Restriction::NoSelfJustification => "자신의 행동을 정당화하지 않는다.",
            Restriction::NoBravado => "허세를 부리거나 강한 척 하지 않는다.",
            Restriction::NoLyingOrExaggeration => "거짓말이나 과장을 하지 않는다.",
        }
    }

    /// 성격 특성 한국어 라벨
    pub fn personality_trait_label(t: &PersonalityTrait) -> &'static str {
        match t {
            PersonalityTrait::HonestAndModest => "진실되고 공정하며 겸손한 성격이다.",
            PersonalityTrait::CunningAndAmbitious => "교활하고 야심적이며 자기과시적인 성격이다.",
            PersonalityTrait::EmotionalAndAnxious => "감정이 풍부하고 불안해하기 쉬운 성격이다.",
            PersonalityTrait::BoldAndIndependent => "대담하고 독립적이며 감정을 잘 드러내지 않는다.",
            PersonalityTrait::ConfidentAndSociable => "자신감 있고 사교적이며 활기가 넘친다.",
            PersonalityTrait::IntrovertedAndQuiet => "내성적이고 과묵하며 조용한 성격이다.",
            PersonalityTrait::TolerantAndGentle => "관용적이고 온화하며 인내심이 강하다.",
            PersonalityTrait::GrudgingAndCritical => "원한을 품기 쉽고 비판적이며 참을성이 없다.",
            PersonalityTrait::SystematicAndDiligent => "체계적이고 근면하며 신중하게 행동한다.",
            PersonalityTrait::FreeAndImpulsive => "자유분방하고 충동적이며 즉흥적이다.",
            PersonalityTrait::CuriousAndCreative => "호기심이 많고 창의적이며 관습에 얽매이지 않는다.",
            PersonalityTrait::TraditionalAndConservative => "전통을 존중하고 보수적이며 관습을 따른다.",
        }
    }

    /// 말투 스타일 한국어 라벨
    pub fn speech_style_label(s: &SpeechStyle) -> &'static str {
        match s {
            SpeechStyle::FrankAndUnadorned => "솔직하고 꾸밈없이 말한다.",
            SpeechStyle::HidesInnerThoughts => "속내를 쉽게 드러내지 않고, 필요하면 거짓도 섞는다.",
            SpeechStyle::ExpressiveAndWorried => "감정을 표정과 말투에 드러내며, 걱정을 자주 내비친다.",
            SpeechStyle::CalmAndComposed => "차분하고 담담하게 말하며, 동요하지 않는다.",
            SpeechStyle::ActiveAndForceful => "적극적으로 대화를 이끌고, 목소리에 힘이 있다.",
            SpeechStyle::BriefAndConcise => "말이 적고, 필요한 말만 간결하게 한다.",
            SpeechStyle::SoftAndConsiderate => "부드럽고 배려 깊은 어조로 말한다.",
            SpeechStyle::SharpAndDirect => "날카롭고 직설적이며, 화가 나면 거침없이 표현한다.",
            SpeechStyle::LogicalAndRational => "논리적으로 말하고, 감정보다 이성을 앞세운다.",
            SpeechStyle::UnfilteredAndSpontaneous => "거침없이 말하고, 생각나는 대로 행동한다.",
            SpeechStyle::MetaphoricalAndUnique => "비유나 독특한 표현을 즐겨 쓴다.",
            SpeechStyle::FormalAndTraditional => "격식을 차리고, 전통적인 표현을 쓴다.",
        }
    }

    /// 분위기 한국어 라벨
    pub fn mood_label(mood: f32) -> &'static str {
        if mood > 0.5 { "매우 긍정적" }
        else if mood > 0.2 { "긍정적" }
        else if mood > -0.2 { "중립적" }
        else if mood > -0.5 { "부정적" }
        else { "매우 부정적" }
    }

    /// 성격 특성 목록을 한국어 문장으로
    pub fn format_traits(snapshot: &PersonalitySnapshot) -> String {
        if snapshot.traits.is_empty() {
            "평범하고 특별히 두드러지는 성격 특성이 없다.".to_string()
        } else {
            snapshot.traits.iter()
                .map(|t| Self::personality_trait_label(t))
                .collect::<Vec<_>>()
                .join(" ")
        }
    }

    /// 말투 목록을 한국어 문장으로
    pub fn format_speech_styles(snapshot: &PersonalitySnapshot) -> String {
        if snapshot.speech_styles.is_empty() {
            "평범한 어조로 말한다.".to_string()
        } else {
            snapshot.speech_styles.iter()
                .map(|s| Self::speech_style_label(s))
                .collect::<Vec<_>>()
                .join(" ")
        }
    }
}

impl GuideFormatter for KoreanFormatter {
    fn format_prompt(&self, guide: &ActingGuide) -> String {
        let mut lines = Vec::new();

        // --- NPC 기본 정보 ---
        lines.push(format!("[NPC: {}]", guide.npc_name));
        if !guide.npc_description.is_empty() {
            lines.push(guide.npc_description.clone());
        }
        lines.push(String::new());

        // --- 성격 ---
        lines.push("[성격]".to_string());
        lines.push(Self::format_traits(&guide.personality));
        lines.push(String::new());

        // --- 현재 감정 ---
        lines.push("[현재 감정]".to_string());
        if let Some((ref etype, intensity)) = guide.emotion.dominant {
            lines.push(format!("지배 감정: {}({})",
                Self::emotion_name(etype),
                Self::intensity_label(intensity)));
        }
        if !guide.emotion.active_emotions.is_empty() {
            let emotions_str: Vec<String> = guide.emotion.active_emotions.iter()
                .map(|(etype, intensity)| format!("{}({})",
                    Self::emotion_name(etype),
                    Self::intensity_label(*intensity)))
                .collect();
            lines.push(format!("활성 감정: {}", emotions_str.join(", ")));
        }
        lines.push(format!("전체 분위기: {}", Self::mood_label(guide.emotion.mood)));
        lines.push(String::new());

        // --- 상황 ---
        if let Some(ref desc) = guide.situation_description {
            lines.push("[상황]".to_string());
            lines.push(desc.clone());
            lines.push(String::new());
        }

        // --- 연기 지시 ---
        lines.push("[연기 지시]".to_string());
        lines.push(format!("어조: {}", Self::tone_label(&guide.directive.tone)));
        lines.push(format!("태도: {}", Self::attitude_label(&guide.directive.attitude)));
        lines.push(format!("행동 경향: {}",
            Self::behavioral_tendency_label(&guide.directive.behavioral_tendency)));
        lines.push(String::new());

        // --- 말투 ---
        lines.push("[말투]".to_string());
        lines.push(Self::format_speech_styles(&guide.personality));
        lines.push(String::new());

        // --- 금지 사항 ---
        if !guide.directive.restrictions.is_empty() {
            lines.push("[금지 사항]".to_string());
            for r in &guide.directive.restrictions {
                lines.push(format!("- {}", Self::restriction_label(r)));
            }
        }

        lines.join("\n")
    }

    fn format_json(&self, guide: &ActingGuide) -> Result<String, serde_json::Error> {
        let output = KoreanGuideOutput::from(guide);
        serde_json::to_string_pretty(&output)
    }
}

// ---------------------------------------------------------------------------
// JSON 출력용 DTO — 한국어 텍스트를 포함한 직렬화 전용 구조체
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct KoreanGuideOutput {
    npc_name: String,
    npc_description: String,
    personality: KoreanPersonalityOutput,
    emotion: KoreanEmotionOutput,
    directive: KoreanDirectiveOutput,
    #[serde(skip_serializing_if = "Option::is_none")]
    situation_description: Option<String>,
}

#[derive(Serialize)]
struct KoreanPersonalityOutput {
    traits: String,
    speech_style: String,
}

#[derive(Serialize)]
struct KoreanEmotionOutput {
    dominant: Option<String>,
    active_emotions: Vec<String>,
    mood: f32,
    mood_label: String,
}

#[derive(Serialize)]
struct KoreanDirectiveOutput {
    tone: String,
    attitude: String,
    behavioral_tendency: String,
    restrictions: Vec<String>,
}

impl From<&ActingGuide> for KoreanGuideOutput {
    fn from(guide: &ActingGuide) -> Self {
        Self {
            npc_name: guide.npc_name.clone(),
            npc_description: guide.npc_description.clone(),
            personality: KoreanPersonalityOutput {
                traits: KoreanFormatter::format_traits(&guide.personality),
                speech_style: KoreanFormatter::format_speech_styles(&guide.personality),
            },
            emotion: KoreanEmotionOutput {
                dominant: guide.emotion.dominant.as_ref().map(|(etype, intensity)|
                    format!("{}({})",
                        KoreanFormatter::emotion_name(etype),
                        KoreanFormatter::intensity_label(*intensity))),
                active_emotions: guide.emotion.active_emotions.iter()
                    .map(|(etype, intensity)| format!("{}({})",
                        KoreanFormatter::emotion_name(etype),
                        KoreanFormatter::intensity_label(*intensity)))
                    .collect(),
                mood: guide.emotion.mood,
                mood_label: KoreanFormatter::mood_label(guide.emotion.mood).to_string(),
            },
            directive: KoreanDirectiveOutput {
                tone: KoreanFormatter::tone_label(&guide.directive.tone).to_string(),
                attitude: KoreanFormatter::attitude_label(&guide.directive.attitude).to_string(),
                behavioral_tendency: KoreanFormatter::behavioral_tendency_label(
                    &guide.directive.behavioral_tendency).to_string(),
                restrictions: guide.directive.restrictions.iter()
                    .map(|r| KoreanFormatter::restriction_label(r).to_string())
                    .collect(),
            },
            situation_description: guide.situation_description.clone(),
        }
    }
}
