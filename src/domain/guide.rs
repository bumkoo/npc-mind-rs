//! LLM 연기 가이드 생성기
//!
//! NPC의 성격(HEXACO) + 현재 감정(OCC EmotionState)을 조합하여
//! LLM이 해당 NPC를 연기할 수 있는 구조화된 가이드를 생성한다.
//!
//! 이 모듈의 출력이 NPC 심리 엔진의 최종 산출물이다.
//! 텍스트/JSON 등 구체적 포맷 변환은 presentation 레이어(GuideFormatter)가 담당한다.

use serde::{Deserialize, Serialize};

use super::emotion::{EmotionState, EmotionType};
use super::personality::{HexacoProfile, Npc};

/// 감정의 유의미 판단 기준 (이 이상이면 연기에 반영)
const EMOTION_THRESHOLD: f32 = 0.2;

// ---------------------------------------------------------------------------
// 어조 열거형
// ---------------------------------------------------------------------------

/// 연기 어조 — 감정 + 성격 조합에서 도출
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tone {
    /// 억누른 분노가 느껴지는 차가운 어조
    SuppressedCold,
    /// 거칠고 공격적인 어조
    RoughAggressive,
    /// 불안하고 떨리는 어조
    AnxiousTrembling,
    /// 침울하지만 절제된 어조
    SomberRestrained,
    /// 밝고 활기찬 어조
    BrightLively,
    /// 경계하는 듯하지만 침착한 어조
    VigilantCalm,
    /// 긴장되고 불안한 어조
    TenseAnxious,
    /// 움츠러들고 작아진 어조
    ShrinkingSmall,
    /// 조용한 자신감이 묻어나는 어조
    QuietConfidence,
    /// 자랑스럽고 거만한 어조
    ProudArrogant,
    /// 냉소적이고 비판적인 어조
    CynicalCritical,
    /// 깊은 한숨이 섞인 어조
    DeepSighing,
    /// 진심 어린 따뜻한 어조
    SincerelyWarm,
    /// 질투가 묻어나는 씁쓸한 어조
    JealousBitter,
    /// 안타까움이 담긴 부드러운 어조
    CompassionateSoft,
    /// 편안하고 온화한 어조
    RelaxedGentle,
    /// 무거운 어조
    Heavy,
    /// 담담한 어조
    Calm,
}

// ---------------------------------------------------------------------------
// 태도 열거형
// ---------------------------------------------------------------------------

/// 연기 태도 — 감정 + 성격 조합에서 도출
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Attitude {
    /// 적대적이고 공격적인 태도
    HostileAggressive,
    /// 불만을 억누르지만 불편함이 드러나는 태도
    SuppressedDiscomfort,
    /// 상대를 판단하고 평가하는 태도
    Judgmental,
    /// 경계하며 방어적인 태도
    GuardedDefensive,
    /// 호의적이고 개방적인 태도
    FriendlyOpen,
    /// 방어적이고 닫힌 태도
    DefensiveClosed,
    /// 중립적이고 관망하는 태도
    NeutralObservant,
}

// ---------------------------------------------------------------------------
// 행동 경향 열거형
// ---------------------------------------------------------------------------

/// 행동 경향 — 감정 + 성격 조합에서 도출
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BehavioralTendency {
    /// 즉각적으로 맞서고 행동으로 옮기려 한다
    ImmediateConfrontation,
    /// 분노를 억누르고 계획적으로 대응하려 한다
    StrategicResponse,
    /// 화를 표출하되, 상황을 지켜본다
    ExpressAndObserve,
    /// 두려움에도 불구하고 맞서려 한다
    BraveConfrontation,
    /// 위험을 피하고 안전을 확보하려 한다
    SeekSafety,
    /// 화제를 돌리거나 자리를 피하려 한다
    AvoidOrDeflect,
    /// 대화에 적극적으로 참여하고 협조한다
    ActiveCooperation,
    /// 상황을 관찰하며 필요한 만큼만 반응한다
    ObserveAndRespond,
}

// ---------------------------------------------------------------------------
// 금지 사항 열거형
// ---------------------------------------------------------------------------

/// 금지 사항 — 감정 상태에 따른 행동 제약
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Restriction {
    /// 농담이나 가벼운 말투를 사용하지 않는다
    NoHumorOrLightTone,
    /// 상대에게 호의적으로 대하지 않는다
    NoFriendliness,
    /// 자신의 행동을 정당화하지 않는다
    NoSelfJustification,
    /// 허세를 부리거나 강한 척 하지 않는다
    NoBravado,
    /// 거짓말이나 과장을 하지 않는다
    NoLyingOrExaggeration,
}

// ---------------------------------------------------------------------------
// 성격 특성 열거형
// ---------------------------------------------------------------------------

/// HEXACO 성격에서 도출된 특성 서술자
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PersonalityTrait {
    /// H↑: 진실되고 공정하며 겸손한 성격
    HonestAndModest,
    /// H↓: 교활하고 야심적이며 자기과시적인 성격
    CunningAndAmbitious,
    /// E↑: 감정이 풍부하고 불안해하기 쉬운 성격
    EmotionalAndAnxious,
    /// E↓: 대담하고 독립적이며 감정을 잘 드러내지 않는 성격
    BoldAndIndependent,
    /// X↑: 자신감 있고 사교적이며 활기가 넘치는 성격
    ConfidentAndSociable,
    /// X↓: 내성적이고 과묵하며 조용한 성격
    IntrovertedAndQuiet,
    /// A↑: 관용적이고 온화하며 인내심이 강한 성격
    TolerantAndGentle,
    /// A↓: 원한을 품기 쉽고 비판적이며 참을성이 없는 성격
    GrudgingAndCritical,
    /// C↑: 체계적이고 근면하며 신중하게 행동하는 성격
    SystematicAndDiligent,
    /// C↓: 자유분방하고 충동적이며 즉흥적인 성격
    FreeAndImpulsive,
    /// O↑: 호기심이 많고 창의적이며 관습에 얽매이지 않는 성격
    CuriousAndCreative,
    /// O↓: 전통을 존중하고 보수적이며 관습을 따르는 성격
    TraditionalAndConservative,
}

// ---------------------------------------------------------------------------
// 말투 스타일 열거형
// ---------------------------------------------------------------------------

/// HEXACO 성격에서 도출된 대화 스타일
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpeechStyle {
    /// H↑: 솔직하고 꾸밈없이 말한다
    FrankAndUnadorned,
    /// H↓: 속내를 쉽게 드러내지 않고, 필요하면 거짓도 섞는다
    HidesInnerThoughts,
    /// E↑: 감정을 표정과 말투에 드러내며, 걱정을 자주 내비친다
    ExpressiveAndWorried,
    /// E↓: 차분하고 담담하게 말하며, 동요하지 않는다
    CalmAndComposed,
    /// X↑: 적극적으로 대화를 이끌고, 목소리에 힘이 있다
    ActiveAndForceful,
    /// X↓: 말이 적고, 필요한 말만 간결하게 한다
    BriefAndConcise,
    /// A↑: 부드럽고 배려 깊은 어조로 말한다
    SoftAndConsiderate,
    /// A↓: 날카롭고 직설적이며, 화가 나면 거침없이 표현한다
    SharpAndDirect,
    /// C↑: 논리적으로 말하고, 감정보다 이성을 앞세운다
    LogicalAndRational,
    /// C↓: 거침없이 말하고, 생각나는 대로 행동한다
    UnfilteredAndSpontaneous,
    /// O↑: 비유나 독특한 표현을 즐겨 쓴다
    MetaphoricalAndUnique,
    /// O↓: 격식을 차리고, 전통적인 표현을 쓴다
    FormalAndTraditional,
}

// ---------------------------------------------------------------------------
// 성격 스냅샷
// ---------------------------------------------------------------------------

/// HEXACO 성격의 구조화된 요약 — 도메인 데이터
///
/// 한국어 텍스트 렌더링은 presentation 레이어가 담당한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalitySnapshot {
    /// 핵심 성격 특성 목록
    pub traits: Vec<PersonalityTrait>,
    /// 대화 스타일 목록
    pub speech_styles: Vec<SpeechStyle>,
}

impl PersonalitySnapshot {
    /// HEXACO 프로필에서 두드러지는 특성을 추출
    pub fn from_profile(profile: &HexacoProfile) -> Self {
        let avg = profile.dimension_averages();
        let mut traits = Vec::new();
        let mut styles = Vec::new();

        // H: 정직-겸손성
        if avg.h > 0.3 {
            traits.push(PersonalityTrait::HonestAndModest);
            styles.push(SpeechStyle::FrankAndUnadorned);
        } else if avg.h < -0.3 {
            traits.push(PersonalityTrait::CunningAndAmbitious);
            styles.push(SpeechStyle::HidesInnerThoughts);
        }

        // E: 정서성
        if avg.e > 0.3 {
            traits.push(PersonalityTrait::EmotionalAndAnxious);
            styles.push(SpeechStyle::ExpressiveAndWorried);
        } else if avg.e < -0.3 {
            traits.push(PersonalityTrait::BoldAndIndependent);
            styles.push(SpeechStyle::CalmAndComposed);
        }

        // X: 외향성
        if avg.x > 0.3 {
            traits.push(PersonalityTrait::ConfidentAndSociable);
            styles.push(SpeechStyle::ActiveAndForceful);
        } else if avg.x < -0.3 {
            traits.push(PersonalityTrait::IntrovertedAndQuiet);
            styles.push(SpeechStyle::BriefAndConcise);
        }

        // A: 원만성
        if avg.a > 0.3 {
            traits.push(PersonalityTrait::TolerantAndGentle);
            styles.push(SpeechStyle::SoftAndConsiderate);
        } else if avg.a < -0.3 {
            traits.push(PersonalityTrait::GrudgingAndCritical);
            styles.push(SpeechStyle::SharpAndDirect);
        }

        // C: 성실성
        if avg.c > 0.3 {
            traits.push(PersonalityTrait::SystematicAndDiligent);
            styles.push(SpeechStyle::LogicalAndRational);
        } else if avg.c < -0.3 {
            traits.push(PersonalityTrait::FreeAndImpulsive);
            styles.push(SpeechStyle::UnfilteredAndSpontaneous);
        }

        // O: 개방성
        if avg.o > 0.3 {
            traits.push(PersonalityTrait::CuriousAndCreative);
            styles.push(SpeechStyle::MetaphoricalAndUnique);
        } else if avg.o < -0.3 {
            traits.push(PersonalityTrait::TraditionalAndConservative);
            styles.push(SpeechStyle::FormalAndTraditional);
        }

        Self {
            traits,
            speech_styles: styles,
        }
    }
}

// ---------------------------------------------------------------------------
// 감정 스냅샷
// ---------------------------------------------------------------------------

/// 현재 감정 상태의 구조화된 요약 — 도메인 데이터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionSnapshot {
    /// 지배 감정 (가장 강한 감정의 유형과 강도)
    pub dominant: Option<(EmotionType, f32)>,
    /// 유의미한 감정 목록 (강도 내림차순, 유형과 강도 쌍)
    pub active_emotions: Vec<(EmotionType, f32)>,
    /// 전체 분위기 (-1.0=매우 부정, +1.0=매우 긍정)
    pub mood: f32,
}

impl EmotionSnapshot {
    /// EmotionState에서 스냅샷 생성
    pub fn from_state(state: &EmotionState) -> Self {
        let dominant = state.dominant()
            .map(|e| (e.emotion_type(), e.intensity()));

        let active_emotions = state.significant(EMOTION_THRESHOLD)
            .iter()
            .map(|e| (e.emotion_type(), e.intensity()))
            .collect();

        let mood = state.overall_valence();

        Self {
            dominant,
            active_emotions,
            mood,
        }
    }
}

// ---------------------------------------------------------------------------
// 연기 지시문
// ---------------------------------------------------------------------------

/// 감정 상태에서 도출된 구체적 연기 지시
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActingDirective {
    /// 어조
    pub tone: Tone,
    /// 태도
    pub attitude: Attitude,
    /// 행동 경향
    pub behavioral_tendency: BehavioralTendency,
    /// 금지 사항
    pub restrictions: Vec<Restriction>,
}

impl ActingDirective {
    pub fn from_emotion_and_personality(
        state: &EmotionState,
        profile: &HexacoProfile,
    ) -> Self {
        let avg = profile.dimension_averages();
        let mood = state.overall_valence();
        let dominant = state.dominant();
        let significant = state.significant(EMOTION_THRESHOLD);

        // --- 어조 결정 ---
        let tone = match dominant.map(|e| e.emotion_type()) {
            Some(EmotionType::Anger) => {
                if avg.c > 0.3 { Tone::SuppressedCold }
                else { Tone::RoughAggressive }
            }
            Some(EmotionType::Distress) => {
                if avg.e > 0.3 { Tone::AnxiousTrembling }
                else { Tone::SomberRestrained }
            }
            Some(EmotionType::Joy) => Tone::BrightLively,
            Some(EmotionType::Fear) => {
                if avg.e < -0.3 { Tone::VigilantCalm }
                else { Tone::TenseAnxious }
            }
            Some(EmotionType::Shame) => Tone::ShrinkingSmall,
            Some(EmotionType::Pride) => {
                if avg.h > 0.3 { Tone::QuietConfidence }
                else { Tone::ProudArrogant }
            }
            Some(EmotionType::Reproach) => Tone::CynicalCritical,
            Some(EmotionType::Disappointment) => Tone::DeepSighing,
            Some(EmotionType::Gratitude) => Tone::SincerelyWarm,
            Some(EmotionType::Resentment) => Tone::JealousBitter,
            Some(EmotionType::Pity) => Tone::CompassionateSoft,
            _ => {
                if mood > 0.2 { Tone::RelaxedGentle }
                else if mood < -0.2 { Tone::Heavy }
                else { Tone::Calm }
            }
        };

        // --- 태도 결정 ---
        let attitude = if significant.iter().any(|e| e.emotion_type() == EmotionType::Anger) {
            if avg.a < -0.3 {
                Attitude::HostileAggressive
            } else {
                Attitude::SuppressedDiscomfort
            }
        } else if significant.iter().any(|e| e.emotion_type() == EmotionType::Reproach) {
            Attitude::Judgmental
        } else if significant.iter().any(|e| e.emotion_type() == EmotionType::Fear) {
            Attitude::GuardedDefensive
        } else if mood > 0.3 {
            Attitude::FriendlyOpen
        } else if mood < -0.3 {
            Attitude::DefensiveClosed
        } else {
            Attitude::NeutralObservant
        };

        // --- 행동 경향 결정 ---
        let behavioral_tendency = if significant.iter().any(|e| e.emotion_type() == EmotionType::Anger) {
            if avg.c < -0.3 {
                BehavioralTendency::ImmediateConfrontation
            } else if avg.c > 0.3 {
                BehavioralTendency::StrategicResponse
            } else {
                BehavioralTendency::ExpressAndObserve
            }
        } else if significant.iter().any(|e| e.emotion_type() == EmotionType::Fear) {
            if avg.e < -0.3 {
                BehavioralTendency::BraveConfrontation
            } else {
                BehavioralTendency::SeekSafety
            }
        } else if significant.iter().any(|e| e.emotion_type() == EmotionType::Shame) {
            BehavioralTendency::AvoidOrDeflect
        } else if mood > 0.3 {
            BehavioralTendency::ActiveCooperation
        } else {
            BehavioralTendency::ObserveAndRespond
        };

        // --- 금지 사항 결정 ---
        let mut restrictions = Vec::new();

        if mood < -0.3 {
            restrictions.push(Restriction::NoHumorOrLightTone);
        }
        if significant.iter().any(|e| e.emotion_type() == EmotionType::Anger) {
            restrictions.push(Restriction::NoFriendliness);
        }
        if significant.iter().any(|e| e.emotion_type() == EmotionType::Shame) {
            restrictions.push(Restriction::NoSelfJustification);
        }
        if significant.iter().any(|e| e.emotion_type() == EmotionType::Fear) {
            restrictions.push(Restriction::NoBravado);
        }
        if avg.h > 0.5 {
            restrictions.push(Restriction::NoLyingOrExaggeration);
        }

        Self {
            tone,
            attitude,
            behavioral_tendency,
            restrictions,
        }
    }
}

// ---------------------------------------------------------------------------
// LLM 연기 가이드 (최종 산출물)
// ---------------------------------------------------------------------------

/// NPC 심리 엔진의 최종 산출물: LLM이 NPC를 연기하기 위한 구조화된 가이드
///
/// 텍스트/JSON 등 구체적 포맷 변환은 `GuideFormatter` 트레이트 구현체가 담당한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActingGuide {
    /// NPC 이름
    pub npc_name: String,
    /// NPC 설명
    pub npc_description: String,
    /// 성격 스냅샷
    pub personality: PersonalitySnapshot,
    /// 감정 스냅샷
    pub emotion: EmotionSnapshot,
    /// 연기 지시문
    pub directive: ActingDirective,
    /// 상황 설명 (있으면)
    pub situation_description: Option<String>,
}

impl ActingGuide {
    /// NPC + EmotionState → ActingGuide 생성
    pub fn build(
        npc: &Npc,
        state: &EmotionState,
        situation_desc: Option<String>,
    ) -> Self {
        Self {
            npc_name: npc.name().to_string(),
            npc_description: npc.description().to_string(),
            personality: PersonalitySnapshot::from_profile(npc.personality()),
            emotion: EmotionSnapshot::from_state(state),
            directive: ActingDirective::from_emotion_and_personality(state, npc.personality()),
            situation_description: situation_desc,
        }
    }
}
