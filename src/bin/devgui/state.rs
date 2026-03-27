//! GUI 상태 — raw f32/String/bool로 egui 슬라이더와 직접 바인딩

/// 프리셋 선택지
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PresetChoice {
    Custom,
    무백,
    교룡,
    수련,
    소호,
}

impl PresetChoice {
    pub const ALL: &[Self] = &[
        Self::Custom,
        Self::무백,
        Self::교룡,
        Self::수련,
        Self::소호,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Custom => "커스텀",
            Self::무백 => "무백 (정의로운 검객)",
            Self::교룡 => "교룡 (야심적인 여검객)",
            Self::수련 => "수련 (절제의 여검객)",
            Self::소호 => "소호 (자유로운 낭인)",
        }
    }
}

/// 포커스 타입 선택
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusType {
    Event,
    Action,
    Object,
}

impl FocusType {
    pub const ALL: &[Self] = &[Self::Event, Self::Action, Self::Object];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Event => "Event (사건)",
            Self::Action => "Action (행동)",
            Self::Object => "Object (대상)",
        }
    }
}

/// Prospect 선택
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProspectChoice {
    None,
    Anticipation,
    HopeFulfilled,
    HopeUnfulfilled,
    FearUnrealized,
    FearConfirmed,
}

impl ProspectChoice {
    pub const ALL: &[Self] = &[
        Self::None,
        Self::Anticipation,
        Self::HopeFulfilled,
        Self::HopeUnfulfilled,
        Self::FearUnrealized,
        Self::FearConfirmed,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "없음",
            Self::Anticipation => "Anticipation (전망)",
            Self::HopeFulfilled => "HopeFulfilled (희망 실현)",
            Self::HopeUnfulfilled => "HopeUnfulfilled (희망 좌절)",
            Self::FearUnrealized => "FearUnrealized (공포 해소)",
            Self::FearConfirmed => "FearConfirmed (공포 확인)",
        }
    }
}

/// 상황의 포커스 하나
#[derive(Debug, Clone)]
pub struct FocusEntry {
    pub focus_type: FocusType,
    // Event
    pub event_description: String,
    pub desirability_for_self: f32,
    pub has_other: bool,
    pub other_target_id: String,
    pub desirability_for_other: f32,
    pub other_closeness: f32,
    pub other_trust: f32,
    pub other_power: f32,
    pub prospect: ProspectChoice,
    // Action
    pub action_description: String,
    pub is_self_agent: bool,
    pub praiseworthiness: f32,
    // Object
    pub object_target_id: String,
    pub object_target_description: String,
    pub appealingness: f32,
}

impl Default for FocusEntry {
    fn default() -> Self {
        Self {
            focus_type: FocusType::Event,
            event_description: String::new(),
            desirability_for_self: 0.0,
            has_other: false,
            other_target_id: String::new(),
            desirability_for_other: 0.0,
            other_closeness: 0.0,
            other_trust: 0.0,
            other_power: 0.0,
            prospect: ProspectChoice::None,
            action_description: String::new(),
            is_self_agent: false,
            praiseworthiness: 0.0,
            object_target_id: String::new(),
            object_target_description: String::new(),
            appealingness: 0.0,
        }
    }
}

/// GUI 전체 입력 상태
pub struct GuiState {
    // NPC 정보
    pub npc_id: String,
    pub npc_name: String,
    pub npc_description: String,

    // HEXACO 24 facets
    // H: 정직-겸손성
    pub sincerity: f32,
    pub fairness: f32,
    pub greed_avoidance: f32,
    pub modesty: f32,
    // E: 정서성
    pub fearfulness: f32,
    pub anxiety: f32,
    pub dependence: f32,
    pub sentimentality: f32,
    // X: 외향성
    pub social_self_esteem: f32,
    pub social_boldness: f32,
    pub sociability: f32,
    pub liveliness: f32,
    // A: 원만성
    pub forgiveness: f32,
    pub gentleness: f32,
    pub flexibility: f32,
    pub patience: f32,
    // C: 성실성
    pub organization: f32,
    pub diligence: f32,
    pub perfectionism: f32,
    pub prudence: f32,
    // O: 경험개방성
    pub aesthetic_appreciation: f32,
    pub inquisitiveness: f32,
    pub creativity: f32,
    pub unconventionality: f32,

    // 상황
    pub situation_description: String,
    pub focuses: Vec<FocusEntry>,

    // 관계
    pub rel_owner_id: String,
    pub rel_target_id: String,
    pub closeness: f32,
    pub trust: f32,
    pub power: f32,

    // PAD 자극
    pub utterance_text: String,
    pub pad_pleasure: f32,
    pub pad_arousal: f32,
    pub pad_dominance: f32,

    // 프리셋
    pub selected_preset: PresetChoice,
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            npc_id: "npc_1".into(),
            npc_name: "NPC".into(),
            npc_description: String::new(),
            sincerity: 0.0,
            fairness: 0.0,
            greed_avoidance: 0.0,
            modesty: 0.0,
            fearfulness: 0.0,
            anxiety: 0.0,
            dependence: 0.0,
            sentimentality: 0.0,
            social_self_esteem: 0.0,
            social_boldness: 0.0,
            sociability: 0.0,
            liveliness: 0.0,
            forgiveness: 0.0,
            gentleness: 0.0,
            flexibility: 0.0,
            patience: 0.0,
            organization: 0.0,
            diligence: 0.0,
            perfectionism: 0.0,
            prudence: 0.0,
            aesthetic_appreciation: 0.0,
            inquisitiveness: 0.0,
            creativity: 0.0,
            unconventionality: 0.0,
            situation_description: String::new(),
            focuses: vec![FocusEntry::default()],
            rel_owner_id: "npc_1".into(),
            rel_target_id: "player".into(),
            closeness: 0.0,
            trust: 0.0,
            power: 0.0,
            utterance_text: String::new(),
            pad_pleasure: 0.0,
            pad_arousal: 0.0,
            pad_dominance: 0.0,
            selected_preset: PresetChoice::Custom,
        }
    }
}
