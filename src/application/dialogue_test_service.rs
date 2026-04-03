//! 대화 테스트 오케스트레이터
//!
//! MindService의 프롬프트 생성 + ConversationPort의 LLM 대화를
//! 하나의 루프로 결합하여 프롬프트 품질을 테스트한다.
//!
//! # 대화 흐름
//!
//! ```text
//! ┌──────────────┐                    ┌──────────────────┐
//! │  Formatted   │  ① appraise()     │  ConversationPort │
//! │  MindService │ ── prompt ──────▶ │  (rig / mock)     │
//! │              │                    │                   │
//! │              │  ③ stimulus()     │                   │
//! │              │ ◀─ NPC 응답 ────  │                   │
//! │              │                    │                   │
//! │              │  ④ beat 전환 시:  │                   │
//! │              │  update_prompt    │                   │
//! │              │ ── new prompt ──▶ │                   │
//! └──────────────┘                    └──────────────────┘
//! ```
//!
//! # 사용 예시
//!
//! ```rust,ignore
//! let repo = InMemoryRepository::from_file("scenario.json")?;
//! let adapter = RigChatAdapter::new("http://127.0.0.1:8081/v1", "model");
//!
//! let mut service = DialogueTestService::new(repo, "ko", adapter)?;
//!
//! // 세션 시작 (appraise + agent 생성)
//! let start = service.start_chat(start_req).await?;
//!
//! // 대사 교환
//! let turn = service.chat_turn(turn_req).await?;
//!
//! // 세션 종료 (관계 갱신 + 이력 반환)
//! let end = service.end_chat(end_req).await?;
//! ```

use crate::domain::emotion::{AppraisalEngine, StimulusEngine};
use crate::domain::pad::Pad;
use crate::ports::{
    Appraiser, ConversationError, ConversationPort, DialogueTurn, MindRepository,
    StimulusProcessor, UtteranceAnalyzer,
};

use super::dto::*;
use super::formatted_service::FormattedMindService;
use super::mind_service::MindServiceError;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// DTO
// ---------------------------------------------------------------------------

/// 대화 세션 시작 요청
#[derive(Serialize, Deserialize, Clone)]
pub struct ChatStartRequest {
    /// 세션 ID (고유 식별자)
    pub session_id: String,
    /// 감정 평가 요청 (appraise 파라미터)
    pub appraise: AppraiseRequest,
}

/// 대화 세션 시작 응답
#[derive(Serialize, Deserialize)]
pub struct ChatStartResponse {
    /// 세션 ID
    pub session_id: String,
    /// 초기 감정 평가 결과 (프롬프트 포함)
    pub appraise: AppraiseResponse,
    /// 세션에 사용된 LLM 모델 정보
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub llm_model_info: Option<crate::ports::LlmModelInfo>,
}

/// 대화 턴 요청
#[derive(Serialize, Deserialize, Clone)]
pub struct ChatTurnRequest {
    /// 세션 ID
    pub session_id: String,
    /// NPC ID
    pub npc_id: String,
    /// 대화 상대 ID
    pub partner_id: String,
    /// 상대의 대사 (Player 또는 상대 NPC)
    pub utterance: String,
    /// PAD 자극값 (수동 입력, analyzer 없을 때 사용)
    pub pad: Option<PadInput>,
    /// 상황 설명 (stimulus 적용 시 사용)
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
    /// NPC의 응답 (LLM 출력)
    pub npc_response: String,
    /// stimulus 적용 결과 (PAD가 있을 때)
    pub stimulus: Option<StimulusResponse>,
    /// Beat 전환 여부
    pub beat_changed: bool,
}

/// 대화 세션 종료 요청
#[derive(Serialize, Deserialize, Clone)]
pub struct ChatEndRequest {
    /// 세션 ID
    pub session_id: String,
    /// after_dialogue 요청 (관계 갱신)
    pub after_dialogue: Option<AfterDialogueRequest>,
}

/// 대화 세션 종료 응답
#[derive(Serialize, Deserialize)]
pub struct ChatEndResponse {
    /// 전체 대화 이력
    pub dialogue_history: Vec<DialogueTurn>,
    /// 관계 갱신 결과
    pub after_dialogue: Option<AfterDialogueResponse>,
}

// ---------------------------------------------------------------------------
// 에러
// ---------------------------------------------------------------------------

/// 대화 테스트 서비스 에러
#[derive(Debug, thiserror::Error)]
pub enum DialogueTestError {
    #[error("Mind 서비스 에러: {0}")]
    MindService(#[from] MindServiceError),
    #[error("대화 에이전트 에러: {0}")]
    Conversation(#[from] ConversationError),
    #[error("PAD 분석 에러: {0}")]
    Analysis(String),
}

// ---------------------------------------------------------------------------
// 서비스
// ---------------------------------------------------------------------------

/// 대화 테스트 오케스트레이터
///
/// `FormattedMindService`(감정 + 프롬프트)와 `ConversationPort`(LLM 대화)를
/// 결합하여 프롬프트 품질 테스트를 위한 대화 루프를 제공한다.
pub struct DialogueTestService<
    R: MindRepository,
    C: ConversationPort,
    A: Appraiser = AppraisalEngine,
    S: StimulusProcessor = StimulusEngine,
> {
    mind: FormattedMindService<R, A, S>,
    chat: C,
    analyzer: Option<Box<dyn UtteranceAnalyzer + Send>>,
}

impl<R: MindRepository, C: ConversationPort> DialogueTestService<R, C> {
    /// 기본 엔진으로 서비스를 생성한다.
    ///
    /// - `repository`: NPC/관계/감정 저장소
    /// - `lang`: 빌트인 로케일 ("ko" 또는 "en")
    /// - `chat`: 대화 에이전트 (ConversationPort 구현체)
    pub fn new(repository: R, lang: &str, chat: C) -> Result<Self, MindServiceError> {
        let mind = FormattedMindService::new(repository, lang)?;
        Ok(Self {
            mind,
            chat,
            analyzer: None,
        })
    }

    /// PAD 자동 분석기를 설정한다 (embed feature 사용 시).
    pub fn with_analyzer(mut self, analyzer: impl UtteranceAnalyzer + Send + 'static) -> Self {
        self.analyzer = Some(Box::new(analyzer));
        self
    }
}

impl<R: MindRepository, C: ConversationPort, A: Appraiser, S: StimulusProcessor>
    DialogueTestService<R, C, A, S>
{
    /// 내부 FormattedMindService의 저장소에 대한 가변 참조
    pub fn mind_mut(&mut self) -> &mut FormattedMindService<R, A, S> {
        &mut self.mind
    }

    /// 내부 FormattedMindService에 대한 불변 참조
    pub fn mind(&self) -> &FormattedMindService<R, A, S> {
        &self.mind
    }

    /// 대화 세션을 시작한다.
    ///
    /// 1. `appraise()`로 초기 감정 + 프롬프트 생성
    /// 2. 생성된 프롬프트로 LLM 세션 시작
    pub async fn start_chat(
        &mut self,
        req: ChatStartRequest,
    ) -> Result<ChatStartResponse, DialogueTestError> {
        // 1. NPC 정보 조회 및 파라미터 유도
        let npc_profile = self
            .mind
            .repository()
            .get_npc(&req.appraise.npc_id)
            .ok_or_else(|| DialogueTestError::MindService(MindServiceError::NpcNotFound(req.appraise.npc_id.clone())))?;

        let mut generation_config = crate::ports::LlmModelInfo::default();
        generation_config.apply_npc_personality(&npc_profile);

        // 2. 감정 평가 실행
        let appraise_resp = self.mind.appraise(req.appraise, || {}, || Vec::new())?;

        // 3. LLM 세션 시작 (유도된 파라미터 적용)
        self.chat
            .start_session(&req.session_id, &appraise_resp.prompt, Some(generation_config.clone()))
            .await?;


        Ok(ChatStartResponse {
            session_id: req.session_id,
            appraise: appraise_resp,
            llm_model_info: Some(generation_config),
        })
    }

    /// 한 턴의 대화를 처리한다.
    ///
    /// 1. 상대 대사를 LLM에 전달 → NPC 응답
    /// 2. PAD 분석 (자동 또는 수동)
    /// 3. stimulus 적용 → 감정 변화
    /// 4. Beat 전환 시 → system_prompt 갱신
    pub async fn chat_turn(
        &mut self,
        req: ChatTurnRequest,
    ) -> Result<ChatTurnResponse, DialogueTestError> {
        // 1. LLM에 상대 대사 전달 → NPC 응답 (세션 고정 파라미터 사용)
        let npc_response = self
            .chat
            .send_message(&req.session_id, &req.utterance)
            .await?;

        // 2. PAD 결정: 수동 입력 > 자동 분석 > 없음
        let pad = if let Some(pad_input) = &req.pad {
            Some(Pad {
                pleasure: pad_input.pleasure,
                arousal: pad_input.arousal,
                dominance: pad_input.dominance,
            })
        } else if let Some(analyzer) = &mut self.analyzer {
            Some(
                analyzer
                    .analyze(&req.utterance)
                    .map_err(|e| DialogueTestError::Analysis(e.to_string()))?,
            )
        } else {
            None
        };

        // 3. PAD가 있으면 stimulus 적용
        let (stimulus, beat_changed) = if let Some(pad) = pad {
            let stim_req = StimulusRequest {
                npc_id: req.npc_id.clone(),
                partner_id: req.partner_id.clone(),
                pleasure: pad.pleasure,
                arousal: pad.arousal,
                dominance: pad.dominance,
                situation_description: req.situation_description,
            };

            let stim_resp = self.mind.apply_stimulus(stim_req, || {}, || Vec::new())?;
            let changed = stim_resp.beat_changed;

            // 4. Beat 전환 시 → system_prompt 갱신
            if changed {
                self.chat
                    .update_system_prompt(&req.session_id, &stim_resp.prompt)
                    .await?;
            }

            (Some(stim_resp), changed)
        } else {
            (None, false)
        };

        Ok(ChatTurnResponse {
            npc_response,
            stimulus,
            beat_changed,
        })
    }

    /// 대화 세션을 종료한다.
    ///
    /// 1. LLM 세션 종료 → 대화 이력 반환
    /// 2. (선택) after_dialogue → 관계 갱신
    pub async fn end_chat(
        &mut self,
        req: ChatEndRequest,
    ) -> Result<ChatEndResponse, DialogueTestError> {
        // 1. 세션 종료 → 이력
        let dialogue_history = self.chat.end_session(&req.session_id).await?;

        // 2. 관계 갱신
        let after_dialogue = if let Some(after_req) = req.after_dialogue {
            Some(self.mind.after_dialogue(after_req)?)
        } else {
            None
        };

        Ok(ChatEndResponse {
            dialogue_history,
            after_dialogue,
        })
    }

    /// 현재 감정 상태에서 가이드만 재생성한다.
    pub fn regenerate_guide(&self, req: GuideRequest) -> Result<GuideResponse, MindServiceError> {
        self.mind.generate_guide(req)
    }
}
