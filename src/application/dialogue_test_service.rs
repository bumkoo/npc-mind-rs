//! 대화 테스트 DTO
//!
//! Mind Studio가 LLM 대화 루프에 주고받는 요청/응답 타입. 서비스 struct는 제거되었고
//! 실제 오케스트레이션은 [`DialogueOrchestrator`](crate::application::dialogue_orchestrator::DialogueOrchestrator)가
//! 담당한다.

use crate::application::dto::{
    AfterDialogueRequest, AfterDialogueResponse, AppraiseRequest, AppraiseResponse,
    StimulusResponse,
};
use crate::ports::{DialogueTurn, LlamaTimings};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// DTO
// ---------------------------------------------------------------------------

/// 대화 세션 시작 요청
#[derive(Serialize, Deserialize, Clone)]
pub struct ChatStartRequest {
    pub session_id: String,
    pub appraise: AppraiseRequest,
}

/// 대화 세션 시작 응답
#[derive(Serialize, Deserialize)]
pub struct ChatStartResponse {
    pub session_id: String,
    pub appraise: AppraiseResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub llm_model_info: Option<crate::ports::LlmModelInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub save_dir: Option<String>,
}

/// 대화 턴 요청
#[derive(Serialize, Deserialize, Clone)]
pub struct ChatTurnRequest {
    pub session_id: String,
    pub npc_id: String,
    pub partner_id: String,
    pub utterance: String,
    pub pad: Option<PadInput>,
    pub situation_description: Option<String>,
}

/// PAD 수동 입력
#[derive(Serialize, Deserialize, Clone)]
pub struct PadInput {
    pub pleasure: f32,
    pub arousal: f32,
    pub dominance: f32,
}

/// 대화 턴 응답
#[derive(Serialize, Deserialize)]
pub struct ChatTurnResponse {
    pub npc_response: String,
    pub stimulus: Option<StimulusResponse>,
    pub beat_changed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub timings: Option<LlamaTimings>,
}

/// 대화 세션 종료 요청
#[derive(Serialize, Deserialize, Clone)]
pub struct ChatEndRequest {
    pub session_id: String,
    pub after_dialogue: Option<AfterDialogueRequest>,
}

/// 대화 세션 종료 응답
#[derive(Serialize, Deserialize)]
pub struct ChatEndResponse {
    pub dialogue_history: Vec<DialogueTurn>,
    pub after_dialogue: Option<AfterDialogueResponse>,
}
