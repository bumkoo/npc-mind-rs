use crate::application::error::MindServiceError;
use crate::domain::emotion::{
    ActionFocus, DesirabilityForOther, EmotionCondition, EmotionState,
    EmotionType, EventFocus, FocusTrigger, ObjectFocus, Prospect, ProspectResult,
    RelationshipModifiers, Situation, ConditionThreshold
};
use crate::domain::guide::ActingGuide;
use crate::domain::personality::Npc;
use crate::domain::relationship::Relationship;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// 감정 평가 요청 — Beat 시작 시 사용
#[derive(Serialize, Deserialize, Clone)]
pub struct AppraiseRequest {
    /// 평가를 수행할 NPC의 ID
    pub npc_id: String,
    /// 대화 상대의 ID
    pub partner_id: String,
    /// 현재 상황 데이터 (DTO)
    /// Scene이 활성이면 생략 가능 — 활성 Focus의 situation을 자동 사용
    #[serde(default)]
    pub situation: Option<SituationInput>,
}

/// OCC 감정 평가를 위한 상황 입력
#[derive(Serialize, Deserialize, Clone)]
pub struct SituationInput {
    /// 상황에 대한 텍스트 설명 (예: "상대와 시장에서 만났다")
    pub description: String,
    /// 사건 평가 — Joy/Distress, Hope/Fear 등을 생성
    pub event: Option<EventInput>,
    /// 행위 평가 — Pride/Shame, Admiration/Reproach 등을 생성
    pub action: Option<ActionInput>,
    /// 대상 평가 — Love/Hate를 생성
    pub object: Option<ObjectInput>,
}

impl SituationInput {
    pub fn into_domain(
        self,
        event_other_modifiers: Option<RelationshipModifiers>,
        action_agent_modifiers: Option<RelationshipModifiers>,
        object_description: Option<String>,
        npc_id: &str,
    ) -> Result<Situation, MindServiceError> {
        let (event, action, object) =
            convert_focuses_owned(self.event, self.action, self.object, event_other_modifiers, action_agent_modifiers, object_description, npc_id)?;

        Situation::new(self.description, event, action, object)
            .map_err(|e| MindServiceError::InvalidSituation(e.to_string()))
    }
}

/// event/action/object 필드를 가진 입력 DTO의 공통 인터페이스
pub trait HasFocusFields {
    fn event(&self) -> Option<&EventInput>;
    fn action(&self) -> Option<&ActionInput>;
    fn object(&self) -> Option<&ObjectInput>;
}

impl HasFocusFields for SituationInput {
    fn event(&self) -> Option<&EventInput> { self.event.as_ref() }
    fn action(&self) -> Option<&ActionInput> { self.action.as_ref() }
    fn object(&self) -> Option<&ObjectInput> { self.object.as_ref() }
}

impl HasFocusFields for super::scene::SceneFocusInput {
    fn event(&self) -> Option<&EventInput> { self.event.as_ref() }
    fn action(&self) -> Option<&ActionInput> { self.action.as_ref() }
    fn object(&self) -> Option<&ObjectInput> { self.object.as_ref() }
}

/// event/action/object 3종 도메인 Focus 옵셔널 묶음
pub(crate) type ConvertedFocuses =
    (Option<EventFocus>, Option<ActionFocus>, Option<ObjectFocus>);

/// event/action/object DTO를 도메인 Focus로 일괄 변환 (소유권 기반)
pub(crate) fn convert_focuses_owned(
    event: Option<EventInput>,
    action: Option<ActionInput>,
    object: Option<ObjectInput>,
    event_other_modifiers: Option<RelationshipModifiers>,
    action_agent_modifiers: Option<RelationshipModifiers>,
    object_description: Option<String>,
    npc_id: &str,
) -> Result<ConvertedFocuses, MindServiceError> {
    let event = event.map(|e| e.into_domain(event_other_modifiers)).transpose()?;
    let action = action.map(|a| a.into_domain(action_agent_modifiers, npc_id)).transpose()?;
    let object = object.map(|o| o.into_domain(object_description)).transpose()?;
    Ok((event, action, object))
}

/// 사건(Event) 입력
#[derive(Serialize, Deserialize, Clone)]
pub struct EventInput {
    pub description: String,
    pub desirability_for_self: f32,
    pub other: Option<EventOtherInput>,
    pub prospect: Option<String>,
}

impl EventInput {
    pub(crate) fn into_domain(
        self,
        other_modifiers: Option<RelationshipModifiers>,
    ) -> Result<EventFocus, MindServiceError> {
        let other = if let Some(o) = self.other {
            let modifiers = other_modifiers.ok_or_else(|| {
                MindServiceError::InvalidSituation(format!(
                    "타인 영향 평가에 관계 정보가 필요합니다: {}",
                    o.target_id
                ))
            })?;
            Some(DesirabilityForOther {
                target_id: o.target_id,
                desirability: o.desirability,
                modifiers,
            })
        } else {
            None
        };

        let prospect = self.prospect.as_deref().and_then(|p| match p {
            "anticipation" => Some(Prospect::Anticipation),
            "hope_fulfilled" => Some(Prospect::Confirmation(ProspectResult::HopeFulfilled)),
            "hope_unfulfilled" => Some(Prospect::Confirmation(ProspectResult::HopeUnfulfilled)),
            "fear_unrealized" => Some(Prospect::Confirmation(ProspectResult::FearUnrealized)),
            "fear_confirmed" => Some(Prospect::Confirmation(ProspectResult::FearConfirmed)),
            _ => None,
        });

        Ok(EventFocus {
            description: self.description,
            desirability_for_self: self.desirability_for_self,
            desirability_for_other: other,
            prospect,
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EventOtherInput {
    pub target_id: String,
    pub desirability: f32,
}

/// 행위(Action) 입력
#[derive(Serialize, Deserialize, Clone)]
pub struct ActionInput {
    pub description: String,
    pub agent_id: Option<String>,
    pub praiseworthiness: f32,
}

impl ActionInput {
    pub(crate) fn into_domain(
        self,
        agent_modifiers: Option<RelationshipModifiers>,
        npc_id: &str,
    ) -> Result<ActionFocus, MindServiceError> {
        let normalized_agent_id = match self.agent_id {
            Some(id) if id == npc_id => None,
            other => other,
        };
        Ok(ActionFocus {
            description: self.description,
            agent_id: normalized_agent_id,
            praiseworthiness: self.praiseworthiness,
            modifiers: agent_modifiers,
        })
    }
}

/// 대상(Object) 입력
#[derive(Serialize, Deserialize, Clone)]
pub struct ObjectInput {
    pub target_id: String,
    pub appealingness: f32,
}

impl ObjectInput {
    pub(crate) fn into_domain(self, description: Option<String>) -> Result<ObjectFocus, MindServiceError> {
        let description = description.unwrap_or(self.target_id.clone());
        Ok(ObjectFocus {
            target_id: self.target_id,
            target_description: description,
            appealingness: self.appealingness,
        })
    }
}

/// PAD 자극 적용 요청
#[derive(Serialize, Deserialize, Clone)]
pub struct StimulusRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub situation_description: Option<String>,
    pub pleasure: f32,
    pub arousal: f32,
    pub dominance: f32,
}

/// 트리거 조건 입력
#[derive(Serialize, Deserialize, Clone)]
pub struct ConditionInput {
    pub emotion: String,
    pub below: Option<f32>,
    pub above: Option<f32>,
    pub absent: Option<bool>,
}

impl ConditionInput {
    pub fn to_domain(&self) -> Result<EmotionCondition, MindServiceError> {
        let emotion = EmotionType::from_str(&self.emotion).map_err(|_| {
            MindServiceError::InvalidSituation(format!("알 수 없는 감정 유형: {}", self.emotion))
        })?;

        let threshold = if let Some(v) = self.below {
            ConditionThreshold::Below(v)
        } else if let Some(v) = self.above {
            ConditionThreshold::Above(v)
        } else if self.absent == Some(true) {
            ConditionThreshold::Absent
        } else {
            return Err(MindServiceError::InvalidSituation(
                "조건에 below, above, absent 중 하나가 필요합니다".into(),
            ));
        };

        Ok(EmotionCondition { emotion, threshold })
    }
}

/// trigger 조건 입력을 FocusTrigger 도메인 모델로 변환
pub(crate) fn parse_trigger(
    trigger: &Option<Vec<Vec<ConditionInput>>>,
) -> Result<FocusTrigger, MindServiceError> {
    let Some(or_groups) = trigger else {
        return Ok(FocusTrigger::Initial);
    };
    let conditions = or_groups
        .iter()
        .map(|and_group| {
            and_group.iter().map(|c| c.to_domain()).collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(FocusTrigger::Conditions(conditions))
}

/// PAD 정보 출력용 DTO
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PadOutput {
    pub pleasure: f32,
    pub arousal: f32,
    pub dominance: f32,
}

/// 감정 정보 출력용 DTO
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EmotionOutput {
    pub emotion_type: String,
    pub intensity: f32,
    pub context: Option<String>,
}

impl EmotionOutput {
    pub fn from_emotion(e: &crate::domain::emotion::Emotion) -> Self {
        Self {
            emotion_type: e.emotion_type().as_str().to_string(),
            intensity: e.intensity(),
            context: e.context().map(|s| s.to_string()),
        }
    }

    pub fn new(emotion_type: EmotionType, intensity: f32, context: Option<&str>) -> Self {
        Self {
            emotion_type: emotion_type.as_str().to_string(),
            intensity,
            context: context.map(|s| s.to_string()),
        }
    }
}

/// Appraise 도메인 결과
pub struct AppraiseResult {
    pub emotions: Vec<EmotionOutput>,
    pub dominant: Option<EmotionOutput>,
    pub mood: f32,
    pub guide: ActingGuide,
    pub trace: Vec<String>,
}

/// Stimulus 도메인 결과
pub struct StimulusResult {
    pub emotions: Vec<EmotionOutput>,
    pub dominant: Option<EmotionOutput>,
    pub mood: f32,
    pub guide: ActingGuide,
    pub trace: Vec<String>,
    pub beat_changed: bool,
    pub active_focus_id: Option<String>,
    pub input_pad: Option<PadOutput>,
}

/// EmotionState에서 공통 응답 필드를 추출
pub fn build_emotion_fields(
    state: &EmotionState,
) -> (Vec<EmotionOutput>, Option<EmotionOutput>, f32) {
    let emotions: Vec<EmotionOutput> = state
        .iter_active()
        .map(|(t, i, ctx)| EmotionOutput::new(t, i, ctx))
        .collect();
    let dominant = state.dominant().map(|e| EmotionOutput::from_emotion(&e));
    let mood = state.overall_valence();
    (emotions, dominant, mood)
}

/// NPC + EmotionState + 관계 → AppraiseResult 생성 헬퍼
pub fn build_appraise_result(
    npc: &Npc,
    state: &EmotionState,
    situation_desc: Option<String>,
    relationship: Option<&Relationship>,
    partner_name: &str,
    trace: Vec<String>,
) -> AppraiseResult {
    let (emotions, dominant, mood) = build_emotion_fields(state);
    let guide = ActingGuide::build(npc, state, situation_desc, relationship, partner_name);
    AppraiseResult {
        emotions,
        dominant,
        mood,
        guide,
        trace,
    }
}

/// 감정 평가 응답 (포맷팅 완료)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppraiseResponse {
    pub emotions: Vec<EmotionOutput>,
    pub dominant: Option<EmotionOutput>,
    pub mood: f32,
    pub prompt: String,
    pub trace: Vec<String>,
}

/// PAD 자극 적용 응답 (포맷팅 완료)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StimulusResponse {
    pub emotions: Vec<EmotionOutput>,
    pub dominant: Option<EmotionOutput>,
    pub mood: f32,
    pub prompt: String,
    pub trace: Vec<String>,
    pub beat_changed: bool,
    pub active_focus_id: Option<String>,
    pub input_pad: Option<PadOutput>,
}
