//! LLM 연기 가이드 생성기
//!
//! NPC의 성격(HEXACO) + 현재 감정(OCC EmotionState)을 조합하여
//! LLM이 해당 NPC를 연기할 수 있는 가이드 텍스트를 생성한다.
//!
//! 이 모듈의 출력이 NPC 심리 엔진의 최종 산출물이다.

use serde::{Deserialize, Serialize};

use super::emotion::{EmotionState, EmotionType};
use super::personality::{Npc, HexacoProfile};

/// 감정의 유의미 판단 기준 (이 이상이면 연기에 반영)
const EMOTION_THRESHOLD: f32 = 0.2;

/// 감정 강도의 정성적 표현
fn intensity_label(intensity: f32) -> &'static str {
    if intensity >= 0.8 { "극도로 강한" }
    else if intensity >= 0.6 { "강한" }
    else if intensity >= 0.4 { "뚜렷한" }
    else if intensity >= 0.2 { "약한" }
    else { "미미한" }
}

/// 감정 유형의 한글 이름
fn emotion_name(etype: &EmotionType) -> &'static str {
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

/// 성격 차원의 정성적 표현
fn _dimension_label(value: f32) -> &'static str {
    if value >= 0.6 { "매우 높음" }
    else if value >= 0.3 { "높음" }
    else if value > -0.3 { "보통" }
    else if value > -0.6 { "낮음" }
    else { "매우 낮음" }
}

// ---------------------------------------------------------------------------
// 성격 요약
// ---------------------------------------------------------------------------

/// HEXACO 성격의 LLM용 요약
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalitySummary {
    /// 핵심 성격 특성 (자연어, 2~3문장)
    pub traits: String,
    /// 대화 스타일 지시 (어조, 말투)
    pub speech_style: String,
}

impl PersonalitySummary {
    pub fn from_profile(profile: &HexacoProfile) -> Self {
        let avg = profile.dimension_averages();
        let mut traits = Vec::new();
        let mut style = Vec::new();

        // H: 정직-겸손성
        if avg.h > 0.3 {
            traits.push("진실되고 공정하며 겸손한 성격이다.");
            style.push("솔직하고 꾸밈없이 말한다.");
        } else if avg.h < -0.3 {
            traits.push("교활하고 야심적이며 자기과시적인 성격이다.");
            style.push("속내를 쉽게 드러내지 않고, 필요하면 거짓도 섞는다.");
        }

        // E: 정서성
        if avg.e > 0.3 {
            traits.push("감정이 풍부하고 불안해하기 쉬운 성격이다.");
            style.push("감정을 표정과 말투에 드러내며, 걱정을 자주 내비친다.");
        } else if avg.e < -0.3 {
            traits.push("대담하고 독립적이며 감정을 잘 드러내지 않는다.");
            style.push("차분하고 담담하게 말하며, 동요하지 않는다.");
        }

        // X: 외향성
        if avg.x > 0.3 {
            traits.push("자신감 있고 사교적이며 활기가 넘친다.");
            style.push("적극적으로 대화를 이끌고, 목소리에 힘이 있다.");
        } else if avg.x < -0.3 {
            traits.push("내성적이고 과묵하며 조용한 성격이다.");
            style.push("말이 적고, 필요한 말만 간결하게 한다.");
        }

        // A: 원만성
        if avg.a > 0.3 {
            traits.push("관용적이고 온화하며 인내심이 강하다.");
            style.push("부드럽고 배려 깊은 어조로 말한다.");
        } else if avg.a < -0.3 {
            traits.push("원한을 품기 쉽고 비판적이며 참을성이 없다.");
            style.push("날카롭고 직설적이며, 화가 나면 거침없이 표현한다.");
        }

        // C: 성실성
        if avg.c > 0.3 {
            traits.push("체계적이고 근면하며 신중하게 행동한다.");
            style.push("논리적으로 말하고, 감정보다 이성을 앞세운다.");
        } else if avg.c < -0.3 {
            traits.push("자유분방하고 충동적이며 즉흥적이다.");
            style.push("거침없이 말하고, 생각나는 대로 행동한다.");
        }

        // O: 개방성
        if avg.o > 0.3 {
            traits.push("호기심이 많고 창의적이며 관습에 얽매이지 않는다.");
            style.push("비유나 독특한 표현을 즐겨 쓴다.");
        } else if avg.o < -0.3 {
            traits.push("전통을 존중하고 보수적이며 관습을 따른다.");
            style.push("격식을 차리고, 전통적인 표현을 쓴다.");
        }

        Self {
            traits: if traits.is_empty() {
                "평범하고 특별히 두드러지는 성격 특성이 없다.".to_string()
            } else {
                traits.join(" ")
            },
            speech_style: if style.is_empty() {
                "평범한 어조로 말한다.".to_string()
            } else {
                style.join(" ")
            },
        }
    }
}

// ---------------------------------------------------------------------------
// 감정 요약
// ---------------------------------------------------------------------------

/// 현재 감정 상태의 LLM용 요약
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionSummary {
    /// 지배 감정 (가장 강한 감정)
    pub dominant: Option<String>,
    /// 유의미한 감정 목록 (강도 내림차순)
    pub active_emotions: Vec<String>,
    /// 전체 분위기 (-1.0=매우 부정, +1.0=매우 긍정)
    pub mood: f32,
    /// 분위기의 정성적 표현
    pub mood_label: String,
}

impl EmotionSummary {
    pub fn from_state(state: &EmotionState) -> Self {
        let dominant = state.dominant().map(|e|
            format!("{}({})", emotion_name(&e.emotion_type), intensity_label(e.intensity))
        );

        let active_emotions = state.significant(EMOTION_THRESHOLD)
            .iter()
            .map(|e| format!("{}({})",
                emotion_name(&e.emotion_type),
                intensity_label(e.intensity)))
            .collect();

        let mood = state.overall_valence();
        let mood_label = if mood > 0.5 { "매우 긍정적" }
            else if mood > 0.2 { "긍정적" }
            else if mood > -0.2 { "중립적" }
            else if mood > -0.5 { "부정적" }
            else { "매우 부정적" };

        Self {
            dominant,
            active_emotions,
            mood,
            mood_label: mood_label.to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// 연기 지시문
// ---------------------------------------------------------------------------

/// 감정 상태에서 도출된 구체적 연기 지시
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActingDirective {
    /// 어조 지시 (예: "냉소적이고 날카로운 어조")
    pub tone: String,
    /// 태도 지시 (예: "상대를 깔보는 태도")
    pub attitude: String,
    /// 행동 경향 (예: "공격적으로 맞서려 함")
    pub behavioral_tendency: String,
    /// 금지 사항 (예: "웃거나 농담하지 않는다")
    pub restrictions: Vec<String>,
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
        let tone = match dominant.map(|e| &e.emotion_type) {
            Some(EmotionType::Anger) => {
                if avg.c > 0.3 { "억누른 분노가 느껴지는 차가운 어조" }
                else { "거칠고 공격적인 어조" }
            }
            Some(EmotionType::Distress) => {
                if avg.e > 0.3 { "불안하고 떨리는 어조" }
                else { "침울하지만 절제된 어조" }
            }
            Some(EmotionType::Joy) => "밝고 활기찬 어조",
            Some(EmotionType::Fear) => {
                if avg.e < -0.3 { "경계하는 듯하지만 침착한 어조" }
                else { "긴장되고 불안한 어조" }
            }
            Some(EmotionType::Shame) => "움츠러들고 작아진 어조",
            Some(EmotionType::Pride) => {
                if avg.h > 0.3 { "조용한 자신감이 묻어나는 어조" }
                else { "자랑스럽고 거만한 어조" }
            }
            Some(EmotionType::Reproach) => "냉소적이고 비판적인 어조",
            Some(EmotionType::Disappointment) => "깊은 한숨이 섞인 어조",
            Some(EmotionType::Gratitude) => "진심 어린 따뜻한 어조",
            Some(EmotionType::Resentment) => "질투가 묻어나는 씁쓸한 어조",
            Some(EmotionType::Pity) => "안타까움이 담긴 부드러운 어조",
            _ => {
                if mood > 0.2 { "편안하고 온화한 어조" }
                else if mood < -0.2 { "무거운 어조" }
                else { "담담한 어조" }
            }
        };

        // --- 태도 결정 ---
        let attitude = if significant.iter().any(|e| e.emotion_type == EmotionType::Anger) {
            if avg.a < -0.3 {
                "적대적이고 공격적인 태도".to_string()
            } else {
                "불만을 억누르지만 불편함이 드러나는 태도".to_string()
            }
        } else if significant.iter().any(|e| e.emotion_type == EmotionType::Reproach) {
            "상대를 판단하고 평가하는 태도".to_string()
        } else if significant.iter().any(|e| e.emotion_type == EmotionType::Fear) {
            "경계하며 방어적인 태도".to_string()
        } else if mood > 0.3 {
            "호의적이고 개방적인 태도".to_string()
        } else if mood < -0.3 {
            "방어적이고 닫힌 태도".to_string()
        } else {
            "중립적이고 관망하는 태도".to_string()
        };

        // --- 행동 경향 결정 ---
        let behavioral_tendency = if significant.iter().any(|e| e.emotion_type == EmotionType::Anger) {
            if avg.c < -0.3 {
                "즉각적으로 맞서고 행동으로 옮기려 한다.".to_string()
            } else if avg.c > 0.3 {
                "분노를 억누르고 계획적으로 대응하려 한다.".to_string()
            } else {
                "화를 표출하되, 상황을 지켜본다.".to_string()
            }
        } else if significant.iter().any(|e| e.emotion_type == EmotionType::Fear) {
            if avg.e < -0.3 {
                "두려움에도 불구하고 맞서려 한다.".to_string()
            } else {
                "위험을 피하고 안전을 확보하려 한다.".to_string()
            }
        } else if significant.iter().any(|e| e.emotion_type == EmotionType::Shame) {
            "화제를 돌리거나 자리를 피하려 한다.".to_string()
        } else if mood > 0.3 {
            "대화에 적극적으로 참여하고 협조한다.".to_string()
        } else {
            "상황을 관찰하며 필요한 만큼만 반응한다.".to_string()
        };

        // --- 금지 사항 결정 ---
        let mut restrictions = Vec::new();

        if mood < -0.3 {
            restrictions.push("농담이나 가벼운 말투를 사용하지 않는다.".to_string());
        }
        if significant.iter().any(|e| e.emotion_type == EmotionType::Anger) {
            restrictions.push("상대에게 호의적으로 대하지 않는다.".to_string());
        }
        if significant.iter().any(|e| e.emotion_type == EmotionType::Shame) {
            restrictions.push("자신의 행동을 정당화하지 않는다.".to_string());
        }
        if significant.iter().any(|e| e.emotion_type == EmotionType::Fear) {
            restrictions.push("허세를 부리거나 강한 척 하지 않는다.".to_string());
        }
        if avg.h > 0.5 {
            restrictions.push("거짓말이나 과장을 하지 않는다.".to_string());
        }

        Self {
            tone: tone.to_string(),
            attitude,
            behavioral_tendency,
            restrictions,
        }
    }
}

// ---------------------------------------------------------------------------
// LLM 연기 가이드 (최종 산출물)
// ---------------------------------------------------------------------------

/// NPC 심리 엔진의 최종 산출물: LLM이 NPC를 연기하기 위한 가이드
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActingGuide {
    /// NPC 이름
    pub npc_name: String,
    /// NPC 설명
    pub npc_description: String,
    /// 성격 요약
    pub personality: PersonalitySummary,
    /// 감정 요약
    pub emotion: EmotionSummary,
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
            npc_name: npc.name.clone(),
            npc_description: npc.description.clone(),
            personality: PersonalitySummary::from_profile(&npc.personality),
            emotion: EmotionSummary::from_state(state),
            directive: ActingDirective::from_emotion_and_personality(state, &npc.personality),
            situation_description: situation_desc,
        }
    }

    /// LLM에게 전달할 프롬프트 텍스트 생성
    pub fn to_prompt(&self) -> String {
        let mut lines = Vec::new();

        // --- NPC 기본 정보 ---
        lines.push(format!("[NPC: {}]", self.npc_name));
        if !self.npc_description.is_empty() {
            lines.push(self.npc_description.clone());
        }
        lines.push(String::new());

        // --- 성격 ---
        lines.push("[성격]".to_string());
        lines.push(self.personality.traits.clone());
        lines.push(String::new());

        // --- 현재 감정 ---
        lines.push("[현재 감정]".to_string());
        if let Some(ref dom) = self.emotion.dominant {
            lines.push(format!("지배 감정: {}", dom));
        }
        if !self.emotion.active_emotions.is_empty() {
            lines.push(format!("활성 감정: {}", self.emotion.active_emotions.join(", ")));
        }
        lines.push(format!("전체 분위기: {}", self.emotion.mood_label));
        lines.push(String::new());
        // --- 상황 ---
        if let Some(ref desc) = self.situation_description {
            lines.push("[상황]".to_string());
            lines.push(desc.clone());
            lines.push(String::new());
        }

        // --- 연기 지시 ---
        lines.push("[연기 지시]".to_string());
        lines.push(format!("어조: {}", self.directive.tone));
        lines.push(format!("태도: {}", self.directive.attitude));
        lines.push(format!("행동 경향: {}", self.directive.behavioral_tendency));
        lines.push(String::new());

        // --- 말투 ---
        lines.push("[말투]".to_string());
        lines.push(self.personality.speech_style.clone());
        lines.push(String::new());

        // --- 금지 사항 ---
        if !self.directive.restrictions.is_empty() {
            lines.push("[금지 사항]".to_string());
            for r in &self.directive.restrictions {
                lines.push(format!("- {}", r));
            }
        }

        lines.join("\n")
    }

    /// JSON 형태로 출력 (API 연동용)
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
