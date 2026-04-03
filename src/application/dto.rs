use super::mind_service::{MindServiceError, NpcWorld};
use crate::domain::emotion::{
    ActionFocus, ConditionThreshold, DesirabilityForOther, EmotionCondition, EmotionState,
    EmotionType, EventFocus, FocusTrigger, ObjectFocus, Prospect, ProspectResult, SceneFocus,
    Situation,
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
    /// 평가 대상 NPC의 ID
    pub npc_id: String,
    /// 대화 상대의 ID (관계 조회에 사용)
    pub partner_id: String,
    /// 상황 정보 (Event/Action/Object 중 최소 하나 포함)
    pub situation: SituationInput,
}

/// OCC 감정 평가를 위한 상황 입력
///
/// Event, Action, Object 세 축 중 최소 하나를 제공해야 합니다.
/// 세 축을 동시에 제공하면 복합 감정(Gratitude, Anger 등)이 생성될 수 있습니다.
#[derive(Serialize, Deserialize, Clone)]
pub struct SituationInput {
    /// 상황 설명 (LLM 가이드의 상황 섹션에 표시)
    pub description: String,
    /// 사건 평가 — Joy/Distress, Hope/Fear 등을 생성
    pub event: Option<EventInput>,
    /// 행위 평가 — Pride/Shame, Admiration/Reproach 등을 생성
    pub action: Option<ActionInput>,
    /// 대상 평가 — Love/Hate를 생성
    pub object: Option<ObjectInput>,
}

impl SituationInput {
    pub fn to_domain<R: NpcWorld>(
        &self,
        repo: &R,
        npc_id: &str,
        partner_id: &str,
    ) -> Result<Situation, MindServiceError> {
        let event = self
            .event
            .as_ref()
            .map(|e| e.to_domain(repo, npc_id))
            .transpose()?;

        let action = self
            .action
            .as_ref()
            .map(|a| a.to_domain(repo, npc_id, partner_id))
            .transpose()?;

        let object = self
            .object
            .as_ref()
            .map(|o| o.to_domain(repo))
            .transpose()?;

        Situation::new(self.description.clone(), event, action, object)
            .map_err(|e| MindServiceError::InvalidSituation(e.to_string()))
    }
}

/// 사건(Event) 입력 — NPC가 사건을 얼마나 바람직하게 느끼는지
#[derive(Serialize, Deserialize, Clone)]
pub struct EventInput {
    /// 사건 설명 (예: "상대가 비밀을 털어놓았다")
    pub description: String,
    /// NPC 자신에 대한 바람직함 (-1.0=극히 불쾌 ~ 1.0=극히 바람직)
    pub desirability_for_self: f32,
    /// 타인에 대한 영향 (있으면 HappyFor/Pity/Gloating/Resentment 평가)
    pub other: Option<EventOtherInput>,
    /// 전망 상태: `"anticipation"`, `"hope_fulfilled"`, `"hope_unfulfilled"`,
    /// `"fear_unrealized"`, `"fear_confirmed"` 중 하나. 없으면 확정된 사건.
    pub prospect: Option<String>,
}

impl EventInput {
    fn to_domain<R: NpcWorld>(
        &self,
        repo: &R,
        npc_id: &str,
    ) -> Result<EventFocus, MindServiceError> {
        let other = if let Some(ref o) = self.other {
            let rel = repo.get_relationship(npc_id, &o.target_id).ok_or_else(|| {
                MindServiceError::RelationshipNotFound(npc_id.to_string(), o.target_id.clone())
            })?;
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
    fn to_domain<R: NpcWorld>(
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

/// 대상(Object) 입력 — 대상에 대한 매력도 평가 (Love/Hate)
#[derive(Serialize, Deserialize, Clone)]
pub struct ObjectInput {
    /// 대상 오브젝트 ID (저장소에서 설명 조회)
    pub target_id: String,
    /// 매력도 (-1.0=극히 혐오 ~ 1.0=극히 매력적)
    pub appealingness: f32,
}

impl ObjectInput {
    fn to_domain<R: NpcWorld>(&self, repo: &R) -> Result<ObjectFocus, MindServiceError> {
        let description = repo
            .get_object_description(&self.target_id)
            .unwrap_or_else(|| self.target_id.clone());
        Ok(ObjectFocus {
            target_id: self.target_id.clone(),
            target_description: description,
            appealingness: self.appealingness,
        })
    }
}

/// 감정 평가 응답 (포맷팅 완료) — `FormattedMindService`가 반환
#[derive(Serialize, Deserialize, Clone)]
pub struct AppraiseResponse {
    /// 생성된 감정 목록 (강도 내림차순)
    pub emotions: Vec<EmotionOutput>,
    /// 가장 강한 지배 감정 (있으면)
    pub dominant: Option<EmotionOutput>,
    /// 전체 분위기 (-1.0=매우 부정 ~ 1.0=매우 긍정)
    pub mood: f32,
    /// LLM에 전달할 포맷팅된 연기 가이드 프롬프트
    pub prompt: String,
    /// 평가 엔진 트레이스 로그 (디버깅용)
    pub trace: Vec<String>,
}

/// Stimulus 응답 (포맷팅 완료) — Beat 전환 여부 포함
#[derive(Serialize, Deserialize, Clone)]
pub struct StimulusResponse {
    /// 자극 적용 후 감정 목록
    pub emotions: Vec<EmotionOutput>,
    /// 가장 강한 지배 감정 (있으면)
    pub dominant: Option<EmotionOutput>,
    /// 전체 분위기 (-1.0 ~ 1.0)
    pub mood: f32,
    /// LLM에 전달할 포맷팅된 연기 가이드 프롬프트
    pub prompt: String,
    /// 평가 엔진 트레이스 로그 (Beat 전환 시에만 발생)
    pub trace: Vec<String>,
    /// Beat 전환이 발생했는지 여부
    pub beat_changed: bool,
    /// 현재 활성 Focus ID (전환 시 새 Focus ID)
    pub active_focus_id: Option<String>,
    /// stimulus 엔진에 입력된 PAD 값 (프론트엔드 자극 탭용)
    pub input_pad: Option<PadOutput>,
}

/// PAD 축 값 출력
#[derive(Serialize, Deserialize, Clone)]
pub struct PadOutput {
    pub pleasure: f32,
    pub arousal: f32,
    pub dominance: f32,
}

/// 개별 감정 출력
#[derive(Serialize, Deserialize, Clone)]
pub struct EmotionOutput {
    /// OCC 감정 유형 이름 (예: `"Joy"`, `"Anger"`, `"Gratitude"`)
    pub emotion_type: String,
    /// 감정 강도 (0.0 ~ 1.0)
    pub intensity: f32,
    /// 감정의 맥락 설명 (평가 시 생성된 상황 요약)
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

/// Appraise 도메인 결과 — [`ActingGuide`] 포함, 포맷팅 전
///
/// `MindService`가 반환하는 raw 결과입니다.
/// `result.format(&formatter)` 호출로 `AppraiseResponse`로 변환합니다.
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
///
/// `result.format(&formatter)` 호출로 `StimulusResponse`로 변환합니다.
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
            input_pad: self.input_pad,
        }
    }
}

/// Guide 도메인 결과 — [`ActingGuide`]만 포함
///
/// `result.format(&formatter)` 호출로 `GuideResponse`로 변환합니다.
pub struct GuideResult {
    /// LLM 연기 가이드
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
pub(crate) fn build_emotion_fields(
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
pub(crate) fn build_appraise_result(
    npc: &Npc,
    state: &EmotionState,
    situation_desc: Option<String>,
    relationship: Option<&Relationship>,
    trace: Vec<String>,
) -> AppraiseResult {
    let (emotions, dominant, mood) = build_emotion_fields(state);
    let guide = ActingGuide::build(npc, state, situation_desc, relationship);
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
    /// `None`이면 기본값 0.0 (일상 대화 수준).
    pub significance: Option<f32>,
}

/// 관계 갱신 응답 — 변동 전후 값 비교
#[derive(Serialize, Deserialize, Clone)]
pub struct AfterDialogueResponse {
    /// 갱신 전 관계 값
    pub before: RelationshipValues,
    /// 갱신 후 관계 값
    pub after: RelationshipValues,
}

/// 관계 3축 수치 (스냅샷)
#[derive(Serialize, Deserialize, Clone)]
pub struct RelationshipValues {
    /// 친밀도 (-1.0=적대 ~ 1.0=매우 친밀)
    pub closeness: f32,
    /// 신뢰도 (-1.0=불신 ~ 1.0=전적 신뢰)
    pub trust: f32,
    /// 상하 관계 (-1.0=NPC가 약자 ~ 1.0=NPC가 강자)
    pub power: f32,
}

/// 가이드 재생성 요청 — 감정 변경 없이 현재 상태에서 가이드만 재구성
#[derive(Serialize, Deserialize, Clone)]
pub struct GuideRequest {
    /// NPC의 ID
    pub npc_id: String,
    /// 대화 상대의 ID
    pub partner_id: String,
    /// 상황 설명 (가이드의 상황 섹션에 표시, 생략 가능)
    pub situation_description: Option<String>,
}

/// 가이드 재생성 응답 (포맷팅 완료)
#[derive(Serialize, Deserialize, Clone)]
pub struct GuideResponse {
    /// LLM에 전달할 텍스트 프롬프트
    pub prompt: String,
    /// 구조화된 JSON 형태의 가이드 데이터
    pub json: String,
}

// ---------------------------------------------------------------------------
// Scene (Focus 옵션 목록)
// ---------------------------------------------------------------------------

/// Scene 시작 요청 — Focus 옵션 목록 등록
#[derive(Serialize, Deserialize, Clone)]
pub struct SceneRequest {
    /// 주체 NPC의 ID
    pub npc_id: String,
    /// 대화 상대의 ID
    pub partner_id: String,
    /// Scene 전체 설명
    pub description: String,
    /// Focus 옵션 목록 (Initial 1개 + Condition N개)
    pub focuses: Vec<SceneFocusInput>,
    /// 상황 중요도 (0.0~1.0). 대화 종료 시 관계 변동 배율에 반영.
    /// `None`이면 기본값 0.5.
    #[serde(default = "default_significance")]
    pub significance: Option<f32>,
}

fn default_significance() -> Option<f32> {
    Some(0.5)
}

/// Focus 옵션 입력 — Beat 전환의 단위
///
/// `trigger`가 `None`이면 Initial Focus (Scene 시작 시 즉시 적용),
/// `Some`이면 감정 조건이 충족될 때 자동으로 Beat 전환됩니다.
#[derive(Serialize, Deserialize, Clone)]
pub struct SceneFocusInput {
    /// Focus 고유 ID
    pub id: String,
    /// Focus 상황 설명 (Beat 전환 시 새 상황으로 사용)
    pub description: String,
    /// 트리거 조건. `None` = Initial, `Some` = OR[AND[...]] 구조의 감정 조건
    pub trigger: Option<Vec<Vec<ConditionInput>>>,
    /// 이 Focus의 사건 평가 (Beat 전환 시 새로 appraise)
    pub event: Option<EventInput>,
    /// 이 Focus의 행위 평가
    pub action: Option<ActionInput>,
    /// 이 Focus의 대상 평가
    pub object: Option<ObjectInput>,
}

/// Focus 트리거의 개별 감정 조건
///
/// `below`, `above`, `absent` 중 정확히 하나를 지정해야 합니다.
#[derive(Serialize, Deserialize, Clone)]
pub struct ConditionInput {
    /// OCC 감정 유형 이름 (예: `"Joy"`, `"Distress"`, `"Anger"`)
    pub emotion: String,
    /// 감정 강도가 이 값 미만일 때 충족
    pub below: Option<f32>,
    /// 감정 강도가 이 값 초과일 때 충족
    pub above: Option<f32>,
    /// `true`이면 해당 감정이 없을 때 충족
    pub absent: Option<bool>,
}

impl ConditionInput {
    fn to_domain(&self) -> Result<EmotionCondition, MindServiceError> {
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

impl SceneFocusInput {
    pub fn to_domain<R: NpcWorld>(
        &self,
        repo: &R,
        npc_id: &str,
        partner_id: &str,
    ) -> Result<SceneFocus, MindServiceError> {
        let trigger = match &self.trigger {
            None => FocusTrigger::Initial,
            Some(or_groups) => {
                let conditions = or_groups
                    .iter()
                    .map(|and_group| {
                        and_group
                            .iter()
                            .map(|c| c.to_domain())
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                FocusTrigger::Conditions(conditions)
            }
        };

        let event = self
            .event
            .as_ref()
            .map(|e| e.to_domain(repo, npc_id))
            .transpose()?;
        let action = self
            .action
            .as_ref()
            .map(|a| a.to_domain(repo, npc_id, partner_id))
            .transpose()?;
        let object = self
            .object
            .as_ref()
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
///
/// `result.format(&formatter)` 호출로 `SceneResponse`로 변환합니다.
pub struct SceneResult {
    /// 등록된 Focus 수
    pub focus_count: usize,
    /// Initial Focus 자동 평가 결과 (있으면)
    pub initial_appraise: Option<AppraiseResult>,
    /// 활성화된 Focus ID (Initial Focus가 있으면 해당 ID)
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

// ... (rest of imports)

/// Scene Focus 상태 조회 결과
#[derive(Serialize, Clone)]
pub struct SceneInfoResult {
    /// Scene이 활성 상태인지 여부
    pub has_scene: bool,
    /// 주체 NPC의 ID (Scene이 없으면 `None`)
    pub npc_id: Option<String>,
    /// 대화 상대의 ID
    pub partner_id: Option<String>,
    /// 현재 활성 Focus ID
    pub active_focus_id: Option<String>,
    /// 상황 중요도 (0.0~1.0). Scene에 설정된 값.
    pub significance: Option<f32>,
    /// 모든 Focus 옵션의 상태 목록
    pub focuses: Vec<FocusInfoItem>,
}

/// Focus 개별 항목 정보 (조회용)
#[derive(Serialize, Clone)]
pub struct FocusInfoItem {
    /// Focus 고유 ID
    pub id: String,
    /// Focus 상황 설명
    pub description: String,
    /// 현재 활성 상태인지 여부
    pub is_active: bool,
    /// 트리거 조건의 사람이 읽을 수 있는 표현 (예: `"Joy > 0.5 AND Distress absent"`)
    pub trigger_display: String,
    /// 이 Focus의 사건 평가 설정
    pub event: Option<FocusEventInfo>,
    /// 이 Focus의 행위 평가 설정
    pub action: Option<FocusActionInfo>,
    /// 이 Focus의 대상 평가 설정
    pub object: Option<FocusObjectInfo>,
}

/// Focus 내 Event 정보 (scene-info 조회용)
#[derive(Serialize, Clone)]
pub struct FocusEventInfo {
    pub description: String,
    pub desirability_for_self: f32,
    pub has_other: bool,
    pub desirability_for_other: Option<f32>,
    pub prospect: Option<String>,
}

/// Focus 내 Action 정보 (scene-info 조회용)
#[derive(Serialize, Clone)]
pub struct FocusActionInfo {
    pub description: String,
    pub agent_id: Option<String>,
    pub praiseworthiness: f32,
}

/// Focus 내 Object 정보 (scene-info 조회용)
#[derive(Serialize, Clone)]
pub struct FocusObjectInfo {
    pub target_id: String,
    pub target_description: String,
    pub appealingness: f32,
}

impl FocusInfoItem {
    pub fn from_domain(f: &SceneFocus, is_active: bool) -> Self {
        let trigger_display = match &f.trigger {
            FocusTrigger::Initial => "initial".to_string(),
            FocusTrigger::Conditions(or_groups) => or_groups
                .iter()
                .map(|and_group| {
                    and_group
                        .iter()
                        .map(|c| {
                            let threshold = match c.threshold {
                                ConditionThreshold::Below(v) => format!("< {:.1}", v),
                                ConditionThreshold::Above(v) => format!("> {:.1}", v),
                                ConditionThreshold::Absent => "absent".to_string(),
                            };
                            format!("{:?} {}", c.emotion, threshold)
                        })
                        .collect::<Vec<_>>()
                        .join(" AND ")
                })
                .collect::<Vec<_>>()
                .join(" OR "),
        };

        let event = f.event.as_ref().map(|e| {
            let (has_other, desirability_for_other) = match &e.desirability_for_other {
                Some(d) => (true, Some(d.desirability)),
                None => (false, None),
            };
            let prospect = e.prospect.as_ref().map(|p| format!("{:?}", p));
            FocusEventInfo {
                description: e.description.clone(),
                desirability_for_self: e.desirability_for_self,
                has_other,
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
            target_description: o.target_description.clone(),
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
        }
    }
}
