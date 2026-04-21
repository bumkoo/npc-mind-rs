use super::error::MindServiceError;
use crate::domain::emotion::{
    ActionFocus, ConditionThreshold, DesirabilityForOther, EmotionCondition, EmotionState,
    EmotionType, EventFocus, FocusTrigger, ObjectFocus, Prospect, ProspectResult,
    RelationshipModifiers, SceneFocus, Situation,
};
use crate::domain::guide::ActingGuide;
use crate::domain::personality::Npc;
use crate::domain::relationship::Relationship;
use crate::ports::GuideFormatter;
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
///
/// Event, Action, Object 세 축 중 최소 하나를 제공해야 합니다.
/// 세 축을 동시에 제공하면 복합 감정(Gratitude, Anger 등)이 생성될 수 있습니다.
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
    pub fn to_domain(
        &self,
        event_other_modifiers: Option<RelationshipModifiers>,
        action_agent_modifiers: Option<RelationshipModifiers>,
        object_description: Option<String>,
        npc_id: &str,
    ) -> Result<Situation, MindServiceError> {
        let (event, action, object) =
            convert_focuses(self, event_other_modifiers, action_agent_modifiers, object_description, npc_id)?;

        Situation::new(self.description.clone(), event, action, object)
            .map_err(|e| MindServiceError::InvalidSituation(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Focus 필드 공통 변환 — SituationInput / SceneFocusInput 중복 제거
// ---------------------------------------------------------------------------

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

impl HasFocusFields for SceneFocusInput {
    fn event(&self) -> Option<&EventInput> { self.event.as_ref() }
    fn action(&self) -> Option<&ActionInput> { self.action.as_ref() }
    fn object(&self) -> Option<&ObjectInput> { self.object.as_ref() }
}

/// event/action/object DTO를 도메인 Focus로 일괄 변환
pub(crate) fn convert_focuses(
    input: &impl HasFocusFields,
    event_other_modifiers: Option<RelationshipModifiers>,
    action_agent_modifiers: Option<RelationshipModifiers>,
    object_description: Option<String>,
    npc_id: &str,
) -> Result<(Option<EventFocus>, Option<ActionFocus>, Option<ObjectFocus>), MindServiceError> {
    let event = input.event().map(|e| e.to_domain(event_other_modifiers)).transpose()?;
    let action = input.action().map(|a| a.to_domain(action_agent_modifiers, npc_id)).transpose()?;
    let object = input.object().map(|o| o.to_domain(object_description)).transpose()?;
    Ok((event, action, object))
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

/// 사건(Event) 입력 — NPC가 사건을 얼마나 바람직하게 느끼는지
#[derive(Serialize, Deserialize, Clone)]
pub struct EventInput {
    /// 사건 설명 (예: "상대가 비밀을 털어놓았다")
    pub description: String,
    /// 자신에 대한 바람직함 (-1.0 ~ 1.0)
    pub desirability_for_self: f32,
    /// 타인에 대한 영향 (생략 가능)
    pub other: Option<EventOtherInput>,
    /// 전망 상태: `"anticipation"`, `"hope_fulfilled"`, `"hope_unfulfilled"`,
    /// `"fear_unrealized"`, `"fear_confirmed"`
    pub prospect: Option<String>,
}

impl EventInput {
    fn to_domain(
        &self,
        other_modifiers: Option<RelationshipModifiers>,
    ) -> Result<EventFocus, MindServiceError> {
        let other = if let Some(ref o) = self.other {
            let modifiers = other_modifiers.ok_or_else(|| {
                MindServiceError::InvalidSituation(format!(
                    "타인 영향 평가에 관계 정보가 필요합니다: {}",
                    o.target_id
                ))
            })?;
            Some(DesirabilityForOther {
                target_id: o.target_id.clone(),
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
            description: self.description.clone(),
            desirability_for_self: self.desirability_for_self,
            desirability_for_other: other,
            prospect,
        })
    }
}

/// 사건이 타인에게 미치는 영향
#[derive(Serialize, Deserialize, Clone)]
pub struct EventOtherInput {
    /// 영향을 받는 대상 NPC의 ID
    pub target_id: String,
    /// 타인에 대한 바람직함 (-1.0 ~ 1.0)
    pub desirability: f32,
}

/// 행위(Action) 입력 — 행위의 정당성/비난 여부 평가
#[derive(Serialize, Deserialize, Clone)]
pub struct ActionInput {
    /// 행위 설명 (예: "상대가 약속을 어겼다")
    pub description: String,
    /// 행위자 ID. `None`이면 NPC 자신, `Some`이면 타인의 행위
    pub agent_id: Option<String>,
    /// 칭찬/비난 정도 (-1.0=극히 비난받을 ~ 1.0=극히 칭찬받을)
    pub praiseworthiness: f32,
}

impl ActionInput {
    fn to_domain(
        &self,
        agent_modifiers: Option<RelationshipModifiers>,
        npc_id: &str,
    ) -> Result<ActionFocus, MindServiceError> {
        // agent_id가 NPC 자신의 ID와 같으면 None으로 정규화 — 엔진이 "자기 행위"로
        // 판별하여 Pride/Shame/Gratification 경로로 감정을 생성하도록 한다.
        let normalized_agent_id = match &self.agent_id {
            Some(id) if id == npc_id => None,
            other => other.clone(),
        };
        Ok(ActionFocus {
            description: self.description.clone(),
            agent_id: normalized_agent_id,
            praiseworthiness: self.praiseworthiness,
            modifiers: agent_modifiers,
        })
    }
}

/// 대상(Object) 입력 — 대상에 대한 매력도 평가 (Love/Hate)
#[derive(Serialize, Deserialize, Clone)]
pub struct ObjectInput {
    /// 대상 오브젝트 ID (저장소에서 설명 조회)
    pub target_id: String,
    /// 매력도 (-1.0=극히 혐오 ~ 1.0=극히 매력적)
    pub appealingness: f32,
}

impl ObjectInput {
    fn to_domain(&self, description: Option<String>) -> Result<ObjectFocus, MindServiceError> {
        let description = description.unwrap_or_else(|| self.target_id.clone());
        Ok(ObjectFocus {
            target_id: self.target_id.clone(),
            target_description: description,
            appealingness: self.appealingness,
        })
    }
}

// ---------------------------------------------------------------------------
// Domain Result 타입 — 포맷팅 전 도메인 데이터
// ---------------------------------------------------------------------------

/// Appraise 도메인 결과 — [`ActingGuide`] 포함, 포맷팅 전
pub struct AppraiseResult {
    /// 생성된 감정 목록
    pub emotions: Vec<EmotionOutput>,
    /// 지배 감정
    pub dominant: Option<EmotionOutput>,
    /// 전체 분위기 (-1.0 ~ 1.0)
    pub mood: f32,
    /// LLM 연기 가이드 (Tone, Attitude, Behavior, Restriction 포함)
    pub guide: ActingGuide,
    /// 평가 엔진 트레이스 로그
    pub trace: Vec<String>,
}

/// Stimulus 도메인 결과 — Beat 전환 정보 포함
pub struct StimulusResult {
    /// 자극 적용 후 감정 목록
    pub emotions: Vec<EmotionOutput>,
    /// 지배 감정
    pub dominant: Option<EmotionOutput>,
    /// 전체 분위기 (-1.0 ~ 1.0)
    pub mood: f32,
    /// LLM 연기 가이드
    pub guide: ActingGuide,
    /// 평가 엔진 트레이스 로그 (Beat 전환 시에만)
    pub trace: Vec<String>,
    /// Beat 전환 발생 여부
    pub beat_changed: bool,
    /// 전환된 Focus ID (전환 시에만 `Some`)
    pub active_focus_id: Option<String>,
    /// stimulus 엔진에 입력된 PAD 값
    pub input_pad: Option<PadOutput>,
}

/// Guide 도메인 결과 — [`ActingGuide`]만 포함
pub struct GuideResult {
    /// LLM 연기 가이드
    pub guide: ActingGuide,
}

/// Scene 시작 결과 (도메인) — 초기 appraise 결과 포함 가능
pub struct SceneResult {
    /// 등록된 Focus 수
    pub focus_count: usize,
    /// Initial Focus 자동 평가 결과 (있으면)
    pub initial_appraise: Option<AppraiseResult>,
    /// 활성화된 Focus ID (Initial Focus가 있으면 해당 ID)
    pub active_focus_id: Option<String>,
}

/// 포맷팅 가능한 도메인 결과 트레이트
pub trait CanFormat {
    /// 해당 결과의 포맷팅된 응답 타입
    type Response;
    /// GuideFormatter를 적용하여 Response로 변환
    fn format(self, formatter: &dyn GuideFormatter) -> Self::Response;
}

impl CanFormat for AppraiseResult {
    type Response = AppraiseResponse;
    fn format(self, formatter: &dyn GuideFormatter) -> Self::Response {
        AppraiseResponse {
            emotions: self.emotions,
            dominant: self.dominant,
            mood: self.mood,
            prompt: formatter.format_prompt(&self.guide),
            trace: self.trace,
        }
    }
}

impl CanFormat for StimulusResult {
    type Response = StimulusResponse;
    fn format(self, formatter: &dyn GuideFormatter) -> Self::Response {
        StimulusResponse {
            emotions: self.emotions,
            dominant: self.dominant,
            mood: self.mood,
            prompt: formatter.format_prompt(&self.guide),
            trace: self.trace,
            beat_changed: self.beat_changed,
            active_focus_id: self.active_focus_id,
            input_pad: self.input_pad,
        }
    }
}


impl CanFormat for GuideResult {
    type Response = GuideResponse;
    fn format(self, formatter: &dyn GuideFormatter) -> Self::Response {
        let prompt = formatter.format_prompt(&self.guide);
        let json = formatter.format_json(&self.guide).unwrap_or_default();
        GuideResponse { prompt, json }
    }
}

impl CanFormat for SceneResult {
    type Response = SceneResponse;
    fn format(self, formatter: &dyn GuideFormatter) -> Self::Response {
        SceneResponse {
            focus_count: self.focus_count,
            initial_appraise: self.initial_appraise.map(|r| r.format(formatter)),
            active_focus_id: self.active_focus_id,
        }
    }
}

// ---------------------------------------------------------------------------
// 헬퍼: EmotionState → 응답 필드 변환
// ---------------------------------------------------------------------------

/// EmotionState에서 공통 응답 필드를 추출합니다.
pub fn build_emotion_fields(
    state: &EmotionState,
) -> (Vec<EmotionOutput>, Option<EmotionOutput>, f32) {
    let emotions: Vec<EmotionOutput> = state
        .emotions()
        .iter()
        .map(EmotionOutput::from_emotion)
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

/// PAD 자극 적용 요청 — 대화 턴 중 상대 대사에 의한 감정 변동
#[derive(Serialize, Deserialize, Clone)]
pub struct StimulusRequest {
    /// 자극을 받는 NPC의 ID
    pub npc_id: String,
    /// 대화 상대의 ID
    pub partner_id: String,
    /// 현재 상황 설명 (가이드 갱신용, 생략 가능)
    pub situation_description: Option<String>,
    /// Pleasure 축 자극 (-1.0=불쾌 ~ 1.0=쾌적). PadAnalyzer 또는 수동 입력.
    pub pleasure: f32,
    /// Arousal 축 자극 (-1.0=이완 ~ 1.0=각성)
    pub arousal: f32,
    /// Dominance 축 자극 (-1.0=위축 ~ 1.0=지배)
    pub dominance: f32,
}

/// 대화/Beat 종료 후 관계 갱신 요청
#[derive(Serialize, Deserialize, Clone)]
pub struct AfterDialogueRequest {
    /// NPC의 ID
    pub npc_id: String,
    /// 대화 상대의 ID
    pub partner_id: String,
    /// 상황 중요도 (0.0~1.0). 중대한 사건일수록 관계 변동 폭이 커진다.
    pub significance: Option<f32>,
}

/// 대화 종료 후 관계 갱신 응답
#[derive(Serialize, Deserialize, Clone)]
pub struct AfterDialogueResponse {
    /// 관계 변동 전 값
    pub before: RelationshipValues,
    /// 관계 변동 후 값
    pub after: RelationshipValues,
}

/// 관계 상태 요약 값
#[derive(Serialize, Deserialize, Clone)]
pub struct RelationshipValues {
    /// 친밀도
    pub closeness: f32,
    /// 신뢰도
    pub trust: f32,
    /// 사회적 지위/권력 차이
    pub power: f32,
}

/// 가이드 생성 요청
#[derive(Serialize, Deserialize, Clone)]
pub struct GuideRequest {
    /// NPC ID
    pub npc_id: String,
    /// 대화 상대 ID
    pub partner_id: String,
    /// 현재 상황 설명 (생략 가능)
    pub situation_description: Option<String>,
}

/// Scene 등록 요청
#[derive(Serialize, Deserialize, Clone)]
pub struct SceneRequest {
    /// NPC ID
    pub npc_id: String,
    /// 대화 상대 ID
    pub partner_id: String,
    /// 상황 설명
    pub description: String,
    /// 상황 중요도 (기본 0.5)
    pub significance: Option<f32>,
    /// Focus 시나리오 목록
    pub focuses: Vec<SceneFocusInput>,
}

/// Scene Focus 입력 데이터
#[derive(Serialize, Deserialize, Clone)]
pub struct SceneFocusInput {
    /// Focus 고유 ID
    pub id: String,
    /// Focus 설명
    pub description: String,
    /// 트리거 조건 (OR [ AND[...] ])
    /// `None`이면 Initial Focus로 간주
    pub trigger: Option<Vec<Vec<ConditionInput>>>,
    /// 이 Focus 진입 시 발생할 사건 설정
    pub event: Option<EventInput>,
    /// 이 Focus 진입 시 발생할 행위 설정
    pub action: Option<ActionInput>,
    /// 이 Focus 진입 시 발생할 대상 설정
    pub object: Option<ObjectInput>,
    /// 테스트 스크립트 — Beat별 사전 정의 대사 목록
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub test_script: Vec<String>,
}

impl SceneFocusInput {
    pub fn to_domain(
        &self,
        event_other_modifiers: Option<RelationshipModifiers>,
        action_agent_modifiers: Option<RelationshipModifiers>,
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

/// 트리거 조건 입력
#[derive(Serialize, Deserialize, Clone)]
pub struct ConditionInput {
    /// 감정 유형 문자열
    pub emotion: String,
    /// 임계값 이하 조건 (`<`)
    pub below: Option<f32>,
    /// 임계값 이상 조건 (`>`)
    pub above: Option<f32>,
    /// 감정 부재 조건 (강도 0.05 미만)
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

// ---------------------------------------------------------------------------
// Presentation Response 타입 — 포맷팅 완료 데이터
// ---------------------------------------------------------------------------

/// 감정 평가 응답 (포맷팅 완료)
#[derive(Serialize, Deserialize, Clone)]
pub struct AppraiseResponse {
    /// 생성된 감정 목록 (강도 내림차순)
    pub emotions: Vec<EmotionOutput>,
    /// 지배 감정
    pub dominant: Option<EmotionOutput>,
    /// 전체 분위기 (-1.0 ~ 1.0)
    pub mood: f32,
    /// 포맷팅된 연기 가이드 프롬프트
    pub prompt: String,
    /// 평가 트레이스 로그
    pub trace: Vec<String>,
}

/// PAD 자극 적용 응답 (포맷팅 완료)
#[derive(Serialize, Deserialize, Clone)]
pub struct StimulusResponse {
    /// 자극 적용 후 감정 목록
    pub emotions: Vec<EmotionOutput>,
    /// 지배 감정
    pub dominant: Option<EmotionOutput>,
    /// 전체 분위기
    pub mood: f32,
    /// 갱신된 연기 가이드 프롬프트
    pub prompt: String,
    /// 평가 트레이스 로그 (Beat 전환 시에만)
    pub trace: Vec<String>,
    /// Beat 전환 여부
    pub beat_changed: bool,
    /// 현재 활성 Focus ID
    pub active_focus_id: Option<String>,
    /// 입력된 PAD 정보
    pub input_pad: Option<PadOutput>,
}

/// 가이드 재생성 응답 (포맷팅 완료)
#[derive(Serialize, Deserialize, Clone)]
pub struct GuideResponse {
    /// 연기 가이드 프롬프트
    pub prompt: String,
    /// 연기 가이드 JSON 데이터 (지원되는 경우)
    pub json: String,
}

/// Scene 등록 응답 (포맷팅 완료)
#[derive(Serialize, Deserialize, Clone)]
pub struct SceneResponse {
    /// 등록된 Focus 수
    pub focus_count: usize,
    /// Initial Focus 자동 평가 결과 (있으면)
    pub initial_appraise: Option<AppraiseResponse>,
    /// 활성화된 Focus ID (Initial Focus가 있으면 해당 ID)
    pub active_focus_id: Option<String>,
}

/// PAD 정보 출력용 DTO
#[derive(Serialize, Deserialize, Clone)]
pub struct PadOutput {
    pub pleasure: f32,
    pub arousal: f32,
    pub dominance: f32,
}

/// 감정 정보 출력용 DTO
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

/// Scene 상태 정보 응답 (scene-info)
#[derive(Serialize, Clone)]
pub struct SceneInfoResult {
    /// Scene 활성 여부
    pub has_scene: bool,
    /// NPC ID
    pub npc_id: Option<String>,
    /// 대화 상대 ID
    pub partner_id: Option<String>,
    /// 현재 활성 Focus ID
    pub active_focus_id: Option<String>,
    /// 상황 중요도 (0.0~1.0). Scene에 설정된 값.
    pub significance: Option<f32>,
    /// 모든 Focus 옵션의 상태 목록
    pub focuses: Vec<FocusInfoItem>,
    /// 현재 활성 Beat의 테스트 스크립트 커서 위치 (MCP에서 주입)
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
    /// 테스트 스크립트 — 이 Beat에서 사용할 사전 정의 대사 목록
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub test_script: Vec<String>,
}

impl FocusInfoItem {
    pub fn from_domain(f: &SceneFocus, is_active: bool) -> Self {
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
                Prospect::Confirmation(ProspectResult::HopeFulfilled) => {
                    "hope_fulfilled".to_string()
                }
                Prospect::Confirmation(ProspectResult::HopeUnfulfilled) => {
                    "hope_unfulfilled".to_string()
                }
                Prospect::Confirmation(ProspectResult::FearUnrealized) => {
                    "fear_unrealized".to_string()
                }
                Prospect::Confirmation(ProspectResult::FearConfirmed) => {
                    "fear_confirmed".to_string()
                }
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

        // agent_id: ActionInput::to_domain()에서 npc_id와 같으면 None으로 정규화됨
        // → None = 자기 행동(Pride/Shame), Some = 타인 행동(Admiration/Reproach)
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

/// Focus 내 Event 정보 (scene-info 조회용)
#[derive(Serialize, Clone)]
pub struct FocusEventInfo {
    pub description: String,
    pub desirability_for_self: f32,
    /// 타인 영향 존재 여부
    pub has_other: bool,
    /// 타인 영향 대상 NPC ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_target_id: Option<String>,
    /// 타인에게 바람직한 정도
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desirability_for_other: Option<f32>,
    /// 전망 문자열 (anticipation, hope_fulfilled, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prospect: Option<String>,
}

/// Focus 내 Action 정보 (scene-info 조회용)
#[derive(Serialize, Clone)]
pub struct FocusActionInfo {
    pub description: String,
    /// 행위자 ID (None = NPC 자신)
    pub agent_id: Option<String>,
    pub praiseworthiness: f32,
}

/// Focus 내 Object 정보 (scene-info 조회용)
#[derive(Serialize, Clone)]
pub struct FocusObjectInfo {
    pub target_id: String,
    pub appealingness: f32,
}

// ---------------------------------------------------------------------------
// TellInformation (Step C2 — Mind 컨텍스트 명령)
// ---------------------------------------------------------------------------

/// `Command::TellInformation` 요청 DTO.
///
/// 한 번의 발화로 `listeners`(직접 대상)와 `overhearers`(동석자 — 엿들은 자)에게
/// 정보를 전달한다. Dispatcher는 `TellInformationRequested`를 초기 이벤트로 만들고,
/// `InformationAgent`가 청자당 1개의 `InformationTold` follow-up을 팬아웃한다 (B5).
///
/// **청자 수 상한**: `MAX_EVENTS_PER_COMMAND=20`에 맞춰 listeners + overhearers ≤ 15를
/// 권장한다. 초과 시 `DispatchV2Error::EventBudgetExceeded`.
#[derive(Serialize, Deserialize, Clone)]
pub struct TellInformationRequest {
    /// 발화자 NPC ID
    pub speaker: String,
    /// 직접 대화 상대 목록 — `ListenerRole::Direct`로 전달됨
    pub listeners: Vec<String>,
    /// 엿들은 동석자 목록 — `ListenerRole::Overhearer`로 전달됨. 없으면 빈 벡터.
    #[serde(default)]
    pub overhearers: Vec<String>,
    /// 전달하려는 주장 본문 (청자 `MemoryEntry.content`가 됨)
    pub claim: String,
    /// 화자가 표명하는 확신도 [0.0, 1.0]. 청자 entry.confidence는 이 값에
    /// 청자의 화자에 대한 trust(정규화)를 곱한다.
    pub stated_confidence: f32,
    /// 화자가 이 정보를 어떤 체인으로 받았는지. 빈 vec = 화자가 직접 경험/목격.
    /// 청자의 origin_chain은 `[speaker, ...origin_chain_in]`이 된다.
    #[serde(default)]
    pub origin_chain_in: Vec<String>,
    /// 선택적 topic — Canonical 연결이 필요한 경우 (Step D 이후 본격 사용)
    #[serde(default)]
    pub topic: Option<String>,
}

/// `Command::TellInformation` 응답 DTO.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TellInformationResponse {
    /// 실제 발행된 `InformationTold` 이벤트 수 (= listeners + overhearers).
    pub listeners_informed: usize,
    /// 생성된 청자별 `MemoryEntry` id 목록 (청자 순서대로). MemoryStore가 미부착이면 empty.
    pub memory_entry_ids: Vec<String>,
}
