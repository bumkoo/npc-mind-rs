use serde::{Deserialize, Serialize};
use crate::domain::emotion::{ActionFocus, DesirabilityForOther, EventFocus, ObjectFocus, Prospect, ProspectResult, Situation, SceneFocus, FocusTrigger, EmotionCondition, ConditionThreshold, EmotionType, EmotionState};
use crate::domain::guide::ActingGuide;
use crate::domain::personality::Npc;
use crate::domain::relationship::Relationship;
use crate::ports::GuideFormatter;
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
                modifiers: rel.modifiers(),
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
        let modifiers = match &self.agent_id {
            Some(agent) if agent != partner_id => {
                repo.get_relationship(npc_id, agent).map(|r| r.modifiers())
            }
            _ => None,
        };
        Ok(ActionFocus {
            description: self.description.clone(),
            agent_id: self.agent_id.clone(),
            praiseworthiness: self.praiseworthiness,
            modifiers,
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

impl EmotionOutput {
    pub fn from_emotion(e: &crate::domain::emotion::Emotion) -> Self {
        Self {
            emotion_type: format!("{:?}", e.emotion_type()),
            intensity: e.intensity(),
            context: e.context().map(|s| s.to_string()),
        }
    }
}

// ---------------------------------------------------------------------------
// Domain Result 타입 — 포맷팅 전 도메인 데이터
// ---------------------------------------------------------------------------

/// Appraise 도메인 결과 — ActingGuide 포함, 포맷팅 전
pub struct AppraiseResult {
    pub emotions: Vec<EmotionOutput>,
    pub dominant: Option<EmotionOutput>,
    pub mood: f32,
    pub guide: ActingGuide,
    pub trace: Vec<String>,
}

impl AppraiseResult {
    /// GuideFormatter를 적용하여 AppraiseResponse로 변환
    pub fn format(self, formatter: &dyn GuideFormatter) -> AppraiseResponse {
        AppraiseResponse {
            emotions: self.emotions,
            dominant: self.dominant,
            mood: self.mood,
            prompt: formatter.format_prompt(&self.guide),
            trace: self.trace,
        }
    }
}

/// Stimulus 도메인 결과 — Beat 전환 정보 포함
pub struct StimulusResult {
    pub emotions: Vec<EmotionOutput>,
    pub dominant: Option<EmotionOutput>,
    pub mood: f32,
    pub guide: ActingGuide,
    pub trace: Vec<String>,
    pub beat_changed: bool,
    pub active_focus_id: Option<String>,
}

impl StimulusResult {
    /// GuideFormatter를 적용하여 StimulusResponse로 변환
    pub fn format(self, formatter: &dyn GuideFormatter) -> StimulusResponse {
        StimulusResponse {
            emotions: self.emotions,
            dominant: self.dominant,
            mood: self.mood,
            prompt: formatter.format_prompt(&self.guide),
            trace: self.trace,
            beat_changed: self.beat_changed,
            active_focus_id: self.active_focus_id,
        }
    }
}

/// Guide 도메인 결과 — ActingGuide만 포함
pub struct GuideResult {
    pub guide: ActingGuide,
}

impl GuideResult {
    /// GuideFormatter를 적용하여 GuideResponse로 변환
    pub fn format(self, formatter: &dyn GuideFormatter) -> GuideResponse {
        let prompt = formatter.format_prompt(&self.guide);
        let json = formatter.format_json(&self.guide).unwrap_or_default();
        GuideResponse { prompt, json }
    }
}

// ---------------------------------------------------------------------------
// 헬퍼: EmotionState → 응답 필드 변환
// ---------------------------------------------------------------------------

/// EmotionState에서 공통 응답 필드를 추출합니다.
pub(crate) fn build_emotion_fields(state: &EmotionState) -> (Vec<EmotionOutput>, Option<EmotionOutput>, f32) {
    let emotions: Vec<EmotionOutput> = state.emotions().iter()
        .map(EmotionOutput::from_emotion).collect();
    let dominant = state.dominant().map(|e| EmotionOutput::from_emotion(&e));
    let mood = state.overall_valence();
    (emotions, dominant, mood)
}

/// NPC + EmotionState + 관계 → AppraiseResult 생성 헬퍼
pub(crate) fn build_appraise_result(
    npc: &Npc,
    state: &EmotionState,
    situation_desc: Option<String>,
    relationship: Option<&Relationship>,
    trace: Vec<String>,
) -> AppraiseResult {
    let (emotions, dominant, mood) = build_emotion_fields(state);
    let guide = ActingGuide::build(npc, state, situation_desc, relationship);
    AppraiseResult { emotions, dominant, mood, guide, trace }
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

/// Scene 등록 응답 (포맷팅 완료)
#[derive(Serialize, Deserialize, Clone)]
pub struct SceneResponse {
    /// 등록된 Focus 수
    pub focus_count: usize,
    /// 초기 Focus에 의한 appraise 결과 (있으면)
    pub initial_appraise: Option<AppraiseResponse>,
    /// 현재 활성 Focus ID
    pub active_focus_id: Option<String>,
}

/// Scene 등록 도메인 결과 (포맷팅 전)
pub struct SceneResult {
    pub focus_count: usize,
    pub initial_appraise: Option<AppraiseResult>,
    pub active_focus_id: Option<String>,
}

impl SceneResult {
    /// GuideFormatter를 적용하여 SceneResponse로 변환
    pub fn format(self, formatter: &dyn GuideFormatter) -> SceneResponse {
        SceneResponse {
            focus_count: self.focus_count,
            initial_appraise: self.initial_appraise.map(|r| r.format(formatter)),
            active_focus_id: self.active_focus_id,
        }
    }
}

/// Scene Focus 상태 조회 결과
#[derive(Serialize, Clone)]
pub struct SceneInfoResult {
    pub has_scene: bool,
    pub npc_id: Option<String>,
    pub partner_id: Option<String>,
    pub active_focus_id: Option<String>,
    pub focuses: Vec<FocusInfoItem>,
}

/// Focus 개별 항목 정보
#[derive(Serialize, Clone)]
pub struct FocusInfoItem {
    pub id: String,
    pub description: String,
    pub is_active: bool,
    pub trigger_display: String,
}
