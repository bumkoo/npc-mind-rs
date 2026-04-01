//! 연기 가이드에서 사용하는 열거형 정의
//!
//! 어조, 태도, 행동 경향, 금지 사항, 성격 특성, 말투 스타일

use serde::{Deserialize, Serialize};

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

use crate::domain::personality::Score;

// ... (rest of imports)

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

impl PersonalityTrait {
    /// 특정 차원의 점수로부터 성격 특성을 도출합니다.
    pub fn evaluate(
        score: Score,
        threshold: f32,
        high_variant: Self,
        low_variant: Self,
    ) -> Option<Self> {
        let v = score.value();
        if v > threshold {
            Some(high_variant)
        } else if v < -threshold {
            Some(low_variant)
        } else {
            None
        }
    }
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

impl SpeechStyle {
    /// 특정 차원의 점수로부터 말투 스타일을 도출합니다.
    pub fn evaluate(
        score: Score,
        threshold: f32,
        high_variant: Self,
        low_variant: Self,
    ) -> Option<Self> {
        let v = score.value();
        if v > threshold {
            Some(high_variant)
        } else if v < -threshold {
            Some(low_variant)
        } else {
            None
        }
    }
}
