use serde::{Deserialize, Serialize};
use crate::domain::emotion::{ActionFocus, DesirabilityForOther, EventFocus, ObjectFocus, Prospect, ProspectResult, Situation, SceneFocus, FocusTrigger, EmotionCondition, ConditionThreshold, EmotionType};
use super::mind_service::{MindRepository, MindServiceError};

#[derive(Serialize, Deserialize, Clone)]
pub struct AppraiseRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub situation: SituationInput,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SituationInput {
    pub description: String,
    pub event: Option<EventInput>,
    pub action: Option<ActionInput>,
    pub object: Option<ObjectInput>,
}

impl SituationInput {
    pub fn to_domain<R: MindRepository>(
        &self,
        repo: &R,
        npc_id: &str,
        partner_id: &str,
    ) -> Result<Situation, MindServiceError> {
        let event = self.event.as_ref()
            .map(|e| e.to_domain(repo, npc_id))
            .transpose()?;

        let action = self.action.as_ref()
            .map(|a| a.to_domain(repo, npc_id, partner_id))
            .transpose()?;

        let object = self.object.as_ref()
            .map(|o| o.to_domain(repo))
            .transpose()?;

        Situation::new(self.description.clone(), event, action, object)
            .map_err(|e| MindServiceError::InvalidSituation(e.to_string()))
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EventInput {
    pub description: String,
    pub desirability_for_self: f32,
    pub other: Option<EventOtherInput>,
    pub prospect: Option<String>, // "anticipation", "hope_fulfilled", etc.
}

impl EventInput {
    fn to_domain<R: MindRepository>(
        &self,
        repo: &R,
        npc_id: &str,
    ) -> Result<EventFocus, MindServiceError> {
        let other = if let Some(ref o) = self.other {
            let rel = repo.get_relationship(npc_id, &o.target_id)
                .ok_or_else(|| MindServiceError::RelationshipNotFound(npc_id.to_string(), o.target_id.clone()))?;
            Some(DesirabilityForOther {
                target_id: o.target_id.clone(),
                desirability: o.desirability,
                relationship: rel,
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
            description: self.description.clone(),
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

#[derive(Serialize, Deserialize, Clone)]
pub struct ActionInput {
    pub description: String,
    pub agent_id: Option<String>, // None=자기, Some=타인
    pub praiseworthiness: f32,
}

impl ActionInput {
    fn to_domain<R: MindRepository>(
        &self,
        repo: &R,
        npc_id: &str,
        partner_id: &str,
    ) -> Result<ActionFocus, MindServiceError> {
        let relationship = match &self.agent_id {
            Some(agent) if agent != partner_id => {
                repo.get_relationship(npc_id, agent)
            }
            _ => None,
        };
        Ok(ActionFocus {
            description: self.description.clone(),
            agent_id: self.agent_id.clone(),
            praiseworthiness: self.praiseworthiness,
            relationship,
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ObjectInput {
    pub target_id: String,
    pub appealingness: f32,
}

impl ObjectInput {
    fn to_domain<R: MindRepository>(
        &self,
        repo: &R,
    ) -> Result<ObjectFocus, MindServiceError> {
        let description = repo.get_object_description(&self.target_id)
            .unwrap_or_else(|| self.target_id.clone());
        Ok(ObjectFocus {
            target_id: self.target_id.clone(),
            target_description: description,
            appealingness: self.appealingness,
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppraiseResponse {
    pub emotions: Vec<EmotionOutput>,
    pub dominant: Option<EmotionOutput>,
    pub mood: f32,
    pub prompt: String,
    pub trace: Vec<String>,
}

/// Stimulus 응답 — Beat 전환 여부 포함
#[derive(Serialize, Deserialize, Clone)]
pub struct StimulusResponse {
    pub emotions: Vec<EmotionOutput>,
    pub dominant: Option<EmotionOutput>,
    pub mood: f32,
    pub prompt: String,
    pub trace: Vec<String>,
    /// Beat 전환이 발생했는지 여부
    pub beat_changed: bool,
    /// 현재 활성 Focus ID (전환 시 새 Focus ID)
    pub active_focus_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EmotionOutput {
    pub emotion_type: String,
    pub intensity: f32,
    pub context: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StimulusRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub situation_description: Option<String>,
    pub pleasure: f32,
    pub arousal: f32,
    pub dominance: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AfterDialogueRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub praiseworthiness: Option<f32>,
    /// 상황 중요도 (0.0~1.0). 중대한 사건일수록 관계 변동 폭이 커진다.
    /// None이면 기본값 0.0 (일상 대화 수준).
    pub significance: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AfterDialogueResponse {
    pub before: RelationshipValues,
    pub after: RelationshipValues,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RelationshipValues {
    pub closeness: f32,
    pub trust: f32,
    pub power: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GuideRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub situation_description: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GuideResponse {
    pub prompt: String,
    pub json: String,
}

// ---------------------------------------------------------------------------
// Scene (Focus 옵션 목록)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
pub struct SceneRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub description: String,
    pub focuses: Vec<SceneFocusInput>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SceneFocusInput {
    pub id: String,
    pub description: String,
    /// None이면 Initial, Some이면 Conditions (OR[AND[...]])
    pub trigger: Option<Vec<Vec<ConditionInput>>>,
    pub event: Option<EventInput>,
    pub action: Option<ActionInput>,
    pub object: Option<ObjectInput>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ConditionInput {
    pub emotion: String,
    pub below: Option<f32>,
    pub above: Option<f32>,
    pub absent: Option<bool>,
}

impl ConditionInput {
    fn to_domain(&self) -> Result<EmotionCondition, MindServiceError> {
        let emotion: EmotionType = serde_json::from_str(&format!("\"{}\"", self.emotion))
            .map_err(|_| MindServiceError::InvalidSituation(
                format!("알 수 없는 감정 유형: {}", self.emotion)
            ))?;

        let threshold = if let Some(v) = self.below {
            ConditionThreshold::Below(v)
        } else if let Some(v) = self.above {
            ConditionThreshold::Above(v)
        } else if self.absent == Some(true) {
            ConditionThreshold::Absent
        } else {
            return Err(MindServiceError::InvalidSituation(
                "조건에 below, above, absent 중 하나가 필요합니다".into()
            ));
        };

        Ok(EmotionCondition { emotion, threshold })
    }
}

impl SceneFocusInput {
    pub fn to_domain<R: MindRepository>(
        &self,
        repo: &R,
        npc_id: &str,
        partner_id: &str,
    ) -> Result<SceneFocus, MindServiceError> {
        let trigger = match &self.trigger {
            None => FocusTrigger::Initial,
            Some(or_groups) => {
                let conditions = or_groups.iter()
                    .map(|and_group| {
                        and_group.iter()
                            .map(|c| c.to_domain())
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                FocusTrigger::Conditions(conditions)
            }
        };

        let event = self.event.as_ref()
            .map(|e| e.to_domain(repo, npc_id))
            .transpose()?;
        let action = self.action.as_ref()
            .map(|a| a.to_domain(repo, npc_id, partner_id))
            .transpose()?;
        let object = self.object.as_ref()
            .map(|o| o.to_domain(repo))
            .transpose()?;

        Ok(SceneFocus {
            id: self.id.clone(),
            description: self.description.clone(),
            trigger,
            event,
            action,
            object,
        })
    }
}

/// Scene 등록 응답
#[derive(Serialize, Deserialize, Clone)]
pub struct SceneResponse {
    /// 등록된 Focus 수
    pub focus_count: usize,
    /// 초기 Focus에 의한 appraise 결과 (있으면)
    pub initial_appraise: Option<AppraiseResponse>,
    /// 현재 활성 Focus ID
    pub active_focus_id: Option<String>,
}
