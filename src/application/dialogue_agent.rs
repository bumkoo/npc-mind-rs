//! DialogueAgent — LLM 대사 생성 + EventBus 통합 오케스트레이터 (Phase 4)
//!
//! `CommandDispatcher`를 통해 상태 변경 Command를 발행하고,
//! `ConversationPort`로 LLM 다턴 대화를 진행하며,
//! 각 턴마다 `DialogueTurnCompleted` 이벤트를 EventBus에 발행한다.
//!
//! `DialogueTestService`는 `FormattedMindService` 기반의 얇은 래퍼였지만,
//! `DialogueAgent`는 Command/Event 경로로 동작하므로 `MemoryAgent` 같은
//! broadcast 구독자가 대화를 RAG에 인덱싱할 수 있다.
//!
//! # 대화 흐름
//!
//! ```text
//! start_session(session_id, npc, partner, situation)
//!   → Command::Appraise dispatch (EmotionAppraised 이벤트)
//!   → guide 프롬프트 포맷팅 → ConversationPort::start_session
//!
//! turn(session_id, user_utterance, pad_hint?)
//!   → DialogueTurnCompleted(user) 이벤트 발행
//!   → Command::ApplyStimulus dispatch (StimulusApplied / BeatTransitioned / RelationshipUpdated)
//!   → Beat 전환 시 system_prompt 갱신
//!   → ConversationPort::send_message → NPC 응답
//!   → DialogueTurnCompleted(assistant) 이벤트 발행
//!
//! end_session(session_id, significance?)
//!   → ConversationPort::end_session → 대화 이력
//!   → significance가 있으면 Command::EndDialogue dispatch
//!     (RelationshipUpdated / EmotionCleared / SceneEnded 이벤트 발행)
//! ```
//!
//! # DialogueTurnCompleted 이벤트 직접 발행
//!
//! 현재 Command enum에는 대화 턴 기록 전용 variant가 없으므로,
//! DialogueAgent는 `CommandDispatcher::event_store()` / `event_bus()` /
//! `projections()`를 통해 dispatcher와 동일한 발행 경로를 재사용한다.
//! 순서: append → L1 projection apply_all → broadcast publish.
//!
//! # 동시성
//!
//! `CommandDispatcher::dispatch`는 `&mut self`이므로 DialogueAgent도
//! `&mut self` 메서드로 설계된다. 동일 session에 대한 동시 턴은 허용하지 않는다.
//! 서로 다른 세션을 병렬 실행하려면 별도 DialogueAgent 인스턴스를 생성한다.

use std::collections::HashMap;
use std::sync::Arc;

use crate::application::command::{Command, CommandDispatcher, CommandResult};
use crate::application::command::handler::emotion_snapshot;
use crate::application::dto::{
    AppraiseResponse, CanFormat, SituationInput, StimulusResponse,
};
use crate::application::mind_service::MindServiceError;
use crate::domain::event::{DomainEvent, EventPayload};
use crate::domain::pad::Pad;
use crate::ports::{
    ChatResponse, ConversationError, ConversationPort, GuideFormatter, LlamaTimings,
    MindRepository, UtteranceAnalyzer,
};

// ---------------------------------------------------------------------------
// 출력 타입
// ---------------------------------------------------------------------------

/// 세션 시작 결과
#[derive(Clone)]
pub struct DialogueStartOutcome {
    pub session_id: String,
    pub appraise: AppraiseResponse,
}

/// 한 턴 결과
#[derive(Clone)]
pub struct DialogueTurnOutcome {
    /// NPC의 LLM 응답 텍스트
    pub npc_response: String,
    /// llama-server 성능 메트릭 (없으면 None)
    pub timings: Option<LlamaTimings>,
    /// 자극 적용 결과 (PAD가 있을 때만)
    pub stimulus: Option<StimulusResponse>,
    /// Beat 전환 여부
    pub beat_changed: bool,
}

/// 세션 종료 결과
#[derive(Clone)]
pub struct DialogueEndOutcome {
    pub dialogue_history: Vec<crate::ports::DialogueTurn>,
    /// `Command::EndDialogue`가 dispatch되었으면 관계 갱신 결과
    pub after_dialogue: Option<crate::application::dto::AfterDialogueResponse>,
}

// ---------------------------------------------------------------------------
// 에러
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum DialogueAgentError {
    #[error("CommandDispatcher 에러: {0}")]
    Command(#[from] MindServiceError),
    #[error("ConversationPort 에러: {0}")]
    Conversation(#[from] ConversationError),
    #[error("PAD 분석 실패: {0}")]
    Analysis(String),
    #[error("세션을 찾을 수 없습니다: {0}")]
    SessionNotFound(String),
    #[error("예상하지 못한 CommandResult: {0}")]
    UnexpectedResult(&'static str),
}

// ---------------------------------------------------------------------------
// 세션 메타
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct SessionMeta {
    npc_id: String,
    partner_id: String,
}

// ---------------------------------------------------------------------------
// DialogueAgent
// ---------------------------------------------------------------------------

/// LLM 대사 생성 에이전트
///
/// - `R`: 도메인 저장소 (MindRepository)
/// - `C`: LLM 어댑터 (ConversationPort). `RigChatAdapter` 또는 테스트용 mock.
pub struct DialogueAgent<R: MindRepository, C: ConversationPort> {
    dispatcher: CommandDispatcher<R>,
    chat: C,
    formatter: Arc<dyn GuideFormatter>,
    analyzer: Option<Box<dyn UtteranceAnalyzer + Send>>,
    sessions: HashMap<String, SessionMeta>,
}

impl<R: MindRepository, C: ConversationPort> DialogueAgent<R, C> {
    /// 기본 생성자
    pub fn new(
        dispatcher: CommandDispatcher<R>,
        chat: C,
        formatter: Arc<dyn GuideFormatter>,
    ) -> Self {
        Self {
            dispatcher,
            chat,
            formatter,
            analyzer: None,
            sessions: HashMap::new(),
        }
    }

    /// PAD 자동 분석기 설정 (embed feature와 함께 사용)
    pub fn with_analyzer(mut self, analyzer: impl UtteranceAnalyzer + Send + 'static) -> Self {
        self.analyzer = Some(Box::new(analyzer));
        self
    }

    /// 내부 CommandDispatcher에 대한 참조
    pub fn dispatcher(&self) -> &CommandDispatcher<R> {
        &self.dispatcher
    }

    /// 내부 CommandDispatcher에 대한 가변 참조
    pub fn dispatcher_mut(&mut self) -> &mut CommandDispatcher<R> {
        &mut self.dispatcher
    }

    /// 활성 세션 수 (테스트/진단용)
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    // -----------------------------------------------------------------------
    // 공개 API
    // -----------------------------------------------------------------------

    /// 대화 세션을 시작한다.
    ///
    /// 1. `Command::Appraise` dispatch → 감정 + ActingGuide 생성 + EventBus 발행
    /// 2. 가이드를 프롬프트로 포맷팅
    /// 3. `ConversationPort::start_session`
    ///
    /// 같은 `session_id`를 가진 세션이 이미 존재하면 세션 메타가 새 값으로
    /// 덮어씌워지며, `ConversationPort`의 동작(대부분 에러 반환)에 맡긴다.
    /// LLM start_session이 성공한 이후에만 세션 메타를 기록하므로 실패 경로에서
    /// 메타가 오염되지 않는다.
    pub async fn start_session(
        &mut self,
        session_id: &str,
        npc_id: &str,
        partner_id: &str,
        situation: Option<SituationInput>,
    ) -> Result<DialogueStartOutcome, DialogueAgentError> {
        let cmd = Command::Appraise {
            npc_id: npc_id.to_string(),
            partner_id: partner_id.to_string(),
            situation,
        };

        let result = self.dispatcher.dispatch(cmd)?;
        let appraise_result = match result {
            CommandResult::Appraised(r) => r,
            _ => return Err(DialogueAgentError::UnexpectedResult("Appraise")),
        };

        // format()은 내부에서 format_prompt를 한 번 호출하므로, 결과의 prompt를
        // 그대로 재사용하여 동일 가이드를 중복 포맷팅하지 않는다.
        let appraise_resp: AppraiseResponse = appraise_result.format(&*self.formatter);

        // NPC 성격 기반 생성 파라미터 유도 (옵션)
        let generation_config = self
            .dispatcher
            .repository()
            .get_npc(npc_id)
            .map(|npc| {
                let mut cfg = crate::ports::LlmModelInfo::default();
                cfg.apply_npc_personality(&npc);
                cfg
            });

        self.chat
            .start_session(session_id, &appraise_resp.prompt, generation_config)
            .await?;

        self.sessions.insert(
            session_id.to_string(),
            SessionMeta {
                npc_id: npc_id.to_string(),
                partner_id: partner_id.to_string(),
            },
        );

        Ok(DialogueStartOutcome {
            session_id: session_id.to_string(),
            appraise: appraise_resp,
        })
    }

    /// 한 턴의 대화를 처리한다.
    ///
    /// 1. user 턴을 `DialogueTurnCompleted` 이벤트로 기록
    /// 2. PAD 결정(수동 > analyzer > 없음)
    /// 3. PAD가 있으면 `Command::ApplyStimulus` dispatch
    /// 4. Beat 전환 시 `update_system_prompt`
    /// 5. LLM 호출 → NPC 응답
    /// 6. assistant 턴을 `DialogueTurnCompleted` 이벤트로 기록
    ///
    /// 중간 단계에서 실패하면(예: stimulus dispatch, LLM 호출) 이미 발행된
    /// user 턴 이벤트는 EventStore에 남는다. 호출자는 실패 시 적절히 대응하여
    /// orphan 이벤트를 처리해야 한다. 재시도 시 동일 utterance에 대한 user 턴
    /// 이벤트가 중복될 수 있다.
    pub async fn turn(
        &mut self,
        session_id: &str,
        user_utterance: &str,
        pad_hint: Option<Pad>,
        situation_description: Option<String>,
    ) -> Result<DialogueTurnOutcome, DialogueAgentError> {
        let meta = self
            .sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| DialogueAgentError::SessionNotFound(session_id.to_string()))?;

        // ① user 턴 이벤트 발행 (stimulus 적용 이전의 감정 스냅샷)
        let user_snapshot = self.current_emotion_snapshot(&meta.npc_id);
        self.emit_dialogue_turn(&meta, "user", user_utterance, user_snapshot);

        // ② PAD 결정
        let pad = match pad_hint {
            Some(p) => Some(p),
            None => match self.analyzer.as_mut() {
                Some(analyzer) => Some(
                    analyzer
                        .analyze(user_utterance)
                        .map_err(|e| DialogueAgentError::Analysis(e.to_string()))?,
                ),
                None => None,
            },
        };

        // ③ stimulus 적용 (PAD가 있을 때)
        let (stimulus_resp, beat_changed) = if let Some(pad) = pad {
            let stim_cmd = Command::ApplyStimulus {
                npc_id: meta.npc_id.clone(),
                partner_id: meta.partner_id.clone(),
                pleasure: pad.pleasure,
                arousal: pad.arousal,
                dominance: pad.dominance,
                situation_description,
            };
            let result = self.dispatcher.dispatch(stim_cmd)?;
            let stim_result = match result {
                CommandResult::StimulusApplied(r) => r,
                _ => return Err(DialogueAgentError::UnexpectedResult("ApplyStimulus")),
            };
            let changed = stim_result.beat_changed;

            // ④ Beat 전환 시 system_prompt 갱신
            if changed {
                let new_prompt = self.formatter.format_prompt(&stim_result.guide);
                self.chat
                    .update_system_prompt(session_id, &new_prompt)
                    .await?;
            }

            let resp: StimulusResponse = stim_result.format(&*self.formatter);
            (Some(resp), changed)
        } else {
            (None, false)
        };

        // ⑤ LLM 호출
        let ChatResponse { text, timings } = self.chat.send_message(session_id, user_utterance).await?;

        // ⑥ assistant 턴 이벤트 발행 (stimulus 이후 갱신된 감정 스냅샷)
        let assistant_snapshot = self.current_emotion_snapshot(&meta.npc_id);
        self.emit_dialogue_turn(&meta, "assistant", &text, assistant_snapshot);

        Ok(DialogueTurnOutcome {
            npc_response: text,
            timings,
            stimulus: stimulus_resp,
            beat_changed,
        })
    }

    /// 대화 세션을 종료한다.
    ///
    /// - `significance`가 `Some`이면 `Command::EndDialogue`를 dispatch하여
    ///   관계 갱신 + 감정 초기화 + Scene 정리 이벤트를 발행한다.
    /// - `None`이면 LLM 세션만 종료 (상태 변경 없음).
    pub async fn end_session(
        &mut self,
        session_id: &str,
        significance: Option<f32>,
    ) -> Result<DialogueEndOutcome, DialogueAgentError> {
        let meta = self
            .sessions
            .remove(session_id)
            .ok_or_else(|| DialogueAgentError::SessionNotFound(session_id.to_string()))?;

        let dialogue_history = self.chat.end_session(session_id).await?;

        let after_dialogue = if let Some(sig) = significance {
            let cmd = Command::EndDialogue {
                npc_id: meta.npc_id.clone(),
                partner_id: meta.partner_id.clone(),
                significance: Some(sig),
            };
            let result = self.dispatcher.dispatch(cmd)?;
            match result {
                CommandResult::DialogueEnded(resp) => Some(resp),
                _ => return Err(DialogueAgentError::UnexpectedResult("EndDialogue")),
            }
        } else {
            None
        };

        Ok(DialogueEndOutcome {
            dialogue_history,
            after_dialogue,
        })
    }

    // -----------------------------------------------------------------------
    // 내부 헬퍼
    // -----------------------------------------------------------------------

    /// DialogueTurnCompleted 이벤트를 dispatcher와 동일 경로로 발행한다.
    ///
    /// append → L1 projection apply_all → broadcast publish 순서 유지.
    fn emit_dialogue_turn(
        &self,
        meta: &SessionMeta,
        speaker: &str,
        utterance: &str,
        emotion_snapshot: Vec<(String, f32)>,
    ) {
        let payload = EventPayload::DialogueTurnCompleted {
            npc_id: meta.npc_id.clone(),
            partner_id: meta.partner_id.clone(),
            speaker: speaker.to_string(),
            utterance: utterance.to_string(),
            emotion_snapshot,
        };

        let store = self.dispatcher.event_store();
        let bus = self.dispatcher.event_bus();
        let id = store.next_id();
        let seq = store.next_sequence(&meta.npc_id);
        let event = DomainEvent::new(id, meta.npc_id.clone(), seq, payload);
        store.append(&[event.clone()]);
        self.dispatcher
            .projections()
            .write()
            .unwrap()
            .apply_all(&event);
        bus.publish(&event);
    }

    /// 현재 저장된 감정 상태의 스냅샷을 가져온다.
    /// 저장소에 감정 상태가 없으면 빈 Vec 반환.
    fn current_emotion_snapshot(&self, npc_id: &str) -> Vec<(String, f32)> {
        self.dispatcher
            .repository()
            .get_emotion_state(npc_id)
            .map(|s| emotion_snapshot(&s))
            .unwrap_or_default()
    }
}

