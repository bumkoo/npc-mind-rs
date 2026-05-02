use crate::application::error::MindServiceError;
use crate::domain::emotion::SceneFocus;
use crate::ports::GuideFormatter;
use serde::{Deserialize, Serialize};
use super::emotion::{AppraiseResult, AppraiseResponse, convert_focuses, parse_trigger, EventInput, ActionInput, ObjectInput, ConditionInput};

/// Scene 등록 요청
#[derive(Serialize, Deserialize, Clone)]
pub struct SceneRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub description: String,
    pub significance: Option<f32>,
    pub focuses: Vec<SceneFocusInput>,
}

/// Scene Focus 입력 데이터
#[derive(Serialize, Deserialize, Clone)]
pub struct SceneFocusInput {
    pub id: String,
    pub description: String,
    pub trigger: Option<Vec<Vec<ConditionInput>>>,
    pub event: Option<EventInput>,
    pub action: Option<ActionInput>,
    pub object: Option<ObjectInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub test_script: Vec<String>,
}

impl SceneFocusInput {
    pub fn to_domain(
        &self,
        event_other_modifiers: Option<crate::domain::emotion::RelationshipModifiers>,
        action_agent_modifiers: Option<crate::domain::emotion::RelationshipModifiers>,
        object_description: Option<String>,
        npc_id: &str,
    ) -> Result<SceneFocus, MindServiceError> {
        let trigger = parse_trigger(&self.trigger)?;
        let (event, action, object) =
            convert_focuses(self, event_other_modifiers, action_agent_modifiers, object_description, npc_id)?;

        Ok(SceneFocus {
            id: self.id.clone(),
            description: self.description.clone(),
            trigger,
            event,
            action,
            object,
            test_script: self.test_script.clone(),
        })
    }
}

/// Scene 시작 결과 (도메인)
pub struct SceneResult {
    pub focus_count: usize,
    pub initial_appraise: Option<AppraiseResult>,
    pub active_focus_id: Option<String>,
}

/// Scene 등록 응답 (포맷팅 완료)
#[derive(Serialize, Deserialize, Clone)]
pub struct SceneResponse {
    pub focus_count: usize,
    pub initial_appraise: Option<AppraiseResponse>,
    pub active_focus_id: Option<String>,
}

impl super::CanFormat for SceneResult {
    type Response = SceneResponse;
    fn format(self, formatter: &dyn GuideFormatter) -> Self::Response {
        SceneResponse {
            focus_count: self.focus_count,
            initial_appraise: self.initial_appraise.map(|r| super::CanFormat::format(r, formatter)),
            active_focus_id: self.active_focus_id,
        }
    }
}

/// Scene 상태 정보 응답 (scene-info)
#[derive(Serialize, Clone)]
pub struct SceneInfoResult {
    pub has_scene: bool,
    pub npc_id: Option<String>,
    pub partner_id: Option<String>,
    pub active_focus_id: Option<String>,
    pub significance: Option<f32>,
    pub focuses: Vec<FocusInfoItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_cursor: Option<usize>,
}

/// 개별 Focus 정보 (UI 출력용)
#[derive(Serialize, Clone)]
pub struct FocusInfoItem {
    pub id: String,
    pub description: String,
    pub is_active: bool,
    pub trigger_display: String,
    pub event: Option<FocusEventInfo>,
    pub action: Option<FocusActionInfo>,
    pub object: Option<FocusObjectInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub test_script: Vec<String>,
}

impl FocusInfoItem {
    pub fn from_domain(f: &SceneFocus, is_active: bool) -> Self {
        use crate::domain::emotion::{FocusTrigger, ConditionThreshold, Prospect, ProspectResult};
        let trigger_display = match &f.trigger {
            FocusTrigger::Initial => "초기 활성 (Initial)".into(),
            FocusTrigger::Conditions(groups) => {
                let or_parts: Vec<String> = groups
                    .iter()
                    .map(|and_group| {
                        let and_parts: Vec<String> = and_group
                            .iter()
                            .map(|c| {
                                let threshold = match c.threshold {
                                    ConditionThreshold::Below(v) => format!("< {}", v),
                                    ConditionThreshold::Above(v) => format!("> {}", v),
                                    ConditionThreshold::Absent => "absent".into(),
                                };
                                format!("{:?} {}", c.emotion, threshold)
                            })
                            .collect();
                        format!("({})", and_parts.join(" AND "))
                    })
                    .collect();
                or_parts.join(" OR ")
            }
        };

        let event = f.event.as_ref().map(|e| {
            let (has_other, other_target_id, desirability_for_other) =
                match &e.desirability_for_other {
                    Some(other) => (true, Some(other.target_id.clone()), Some(other.desirability)),
                    None => (false, None, None),
                };
            let prospect = e.prospect.as_ref().map(|p| match p {
                Prospect::Anticipation => "anticipation".to_string(),
                Prospect::Confirmation(ProspectResult::HopeFulfilled) => "hope_fulfilled".to_string(),
                Prospect::Confirmation(ProspectResult::HopeUnfulfilled) => "hope_unfulfilled".to_string(),
                Prospect::Confirmation(ProspectResult::FearUnrealized) => "fear_unrealized".to_string(),
                Prospect::Confirmation(ProspectResult::FearConfirmed) => "fear_confirmed".to_string(),
            });
            FocusEventInfo {
                description: e.description.clone(),
                desirability_for_self: e.desirability_for_self,
                has_other,
                other_target_id,
                desirability_for_other,
                prospect,
            }
        });

        let action = f.action.as_ref().map(|a| FocusActionInfo {
            description: a.description.clone(),
            agent_id: a.agent_id.clone(),
            praiseworthiness: a.praiseworthiness,
        });

        let object = f.object.as_ref().map(|o| FocusObjectInfo {
            target_id: o.target_id.clone(),
            appealingness: o.appealingness,
        });

        Self {
            id: f.id.clone(),
            description: f.description.clone(),
            is_active,
            trigger_display,
            event,
            action,
            object,
            test_script: f.test_script.clone(),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct FocusEventInfo {
    pub description: String,
    pub desirability_for_self: f32,
    pub has_other: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_target_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desirability_for_other: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prospect: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct FocusActionInfo {
    pub description: String,
    pub agent_id: Option<String>,
    pub praiseworthiness: f32,
}

#[derive(Serialize, Clone)]
pub struct FocusObjectInfo {
    pub target_id: String,
    pub appealingness: f32,
}
