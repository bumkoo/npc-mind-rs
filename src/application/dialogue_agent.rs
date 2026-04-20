//! DialogueAgent — LLM 대사 생성 + EventBus 통합 오케스트레이터 (Phase 4)
//!
//! **B5.2 (1/3):** 내부 dispatch 호출을 v2 경로(`dispatch_v2().await`)로 완전 이관.
//! 외부 API(start_session/turn/end_session) 시그니처는 변경 없음.
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
//!   → Command::Appraise dispatch_v2 (EmotionAppraised/GuideGenerated 이벤트)
//!   → guide 프롬프트 포맷팅 → ConversationPort::start_session
//!
//! turn(session_id, user_utterance, pad_hint?)
//!   → DialogueTurnCompleted(user) 이벤트 발행
//!   → Command::ApplyStimulus dispatch_v2 (StimulusApplied / BeatTransitioned / RelationshipUpdated)
//!   → Beat 전환 시 (events에 BeatTransitioned 존재) system_prompt 갱신
//!   → ConversationPort::send_message → NPC 응답
//!   → DialogueTurnCompleted(assistant) 이벤트 발행
//!
//! end_session(session_id, significance?)
//!   → ConversationPort::end_session → 대화 이력
//!   → significance가 있으면 Command::EndDialogue dispatch_v2
//!     (RelationshipUpdated / EmotionCleared / SceneEnded 이벤트 발행)
//! ```
//!
//! # DialogueTurnCompleted 이벤트 직접 발행
//!
//! 현재 Command enum에는 대화 턴 기록 전용 variant가 없으므로,
//! DialogueAgent는 `CommandDispatcher::event_store()` / `event_bus()`를 통해
//! dispatcher와 동일한 발행 경로를 재사용한다. 순서: append → broadcast publish.
//! (v2 inline handler들은 DialogueTurnCompleted에 관심 없으므로 생략됨.)
//!
//! # 동시성
//!
//! `CommandDispatcher::dispatch_v2`는 `&self`이지만 DialogueAgent는 `sessions` HashMap
//! 접근 때문에 `&mut self` 메서드를 유지한다. 동일 session에 대한 동시 턴은 허용하지
//! 않는다. 서로 다른 세션을 병렬 실행하려면 별도 DialogueAgent 인스턴스를 생성한다.

use std::collections::HashMap;
use std::sync::Arc;

use crate::application::command::dispatcher::DispatchV2Error;
use crate::application::command::{Command, CommandDispatcher};
use crate::application::command::handler::emotion_snapshot;
use crate::application::dto::{
    build_appraise_result, build_emotion_fields, AppraiseResponse, AppraiseResult,
    AfterDialogueResponse, CanFormat, PadOutput, RelationshipValues, SituationInput,
    StimulusResponse, StimulusResult,
};
use crate::application::mind_service::MindServiceError;
use crate::domain::event::{DomainEvent, EventKind, EventPayload};
#[cfg(feature = "listener_perspective")]
use crate::domain::listener_perspective::ListenerPerspectiveConverter;
use crate::domain::pad::{Pad, UtteranceEmbedding};
use crate::domain::personality::Npc;
use crate::domain::relationship::Relationship;
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
    #[error("CommandDispatcher v2 에러: {0}")]
    DispatchV2(#[from] DispatchV2Error),
    #[error("ConversationPort 에러: {0}")]
    Conversation(#[from] ConversationError),
    #[error("PAD 분석 실패: {0}")]
    Analysis(String),
    #[error("세션을 찾을 수 없습니다: {0}")]
    SessionNotFound(String),
    /// v2 dispatch 이후 기대 이벤트/상태 재구성 실패 (HandlerShared에 필수 필드 부재 등)
    #[error("dispatch_v2 결과에서 {0}을(를) 재구성할 수 없습니다")]
    ResultReconstruction(&'static str),
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
pub struct DialogueAgent<R: MindRepository + Send + Sync + 'static, C: ConversationPort> {
    dispatcher: CommandDispatcher<R>,
    chat: C,
    formatter: Arc<dyn GuideFormatter>,
    analyzer: Option<Box<dyn UtteranceAnalyzer + Send>>,
    /// Phase 7: 화자 PAD → 청자 PAD 변환기 (옵셔널, listener_perspective feature)
    #[cfg(feature = "listener_perspective")]
    converter: Option<Arc<dyn ListenerPerspectiveConverter>>,
    sessions: HashMap<String, SessionMeta>,
}

impl<R: MindRepository + Send + Sync + 'static, C: ConversationPort> DialogueAgent<R, C> {
    /// 기본 생성자.
    ///
    /// **전제**: 전달받는 `dispatcher`는 `.with_default_handlers()`가 호출된 상태여야 한다.
    /// DialogueAgent는 내부적으로 `dispatcher.dispatch_v2(Command::Appraise / ApplyStimulus /
    /// EndDialogue)`를 호출하며, 결과를 `HandlerShared` 기반으로 재구성한다. 기본 핸들러(Emotion /
    /// Stimulus / Guide / Relationship / Scene Agent + 3 inline projection)가 등록되어 있지
    /// 않으면 `DialogueAgentError::ResultReconstruction`이 반환된다.
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
            #[cfg(feature = "listener_perspective")]
            converter: None,
            sessions: HashMap::new(),
        }
    }

    /// PAD 자동 분석기 설정 (embed feature와 함께 사용)
    pub fn with_analyzer(mut self, analyzer: impl UtteranceAnalyzer + Send + 'static) -> Self {
        self.analyzer = Some(Box::new(analyzer));
        self
    }

    /// 청자 관점 PAD 변환기 설정 (Phase 7, listener_perspective feature)
    ///
    /// 주입 시 `turn()` 안에서 화자 PAD를 청자 PAD로 변환하여
    /// `Command::ApplyStimulus`에 dispatch한다. 변환은 다음 조건을 모두 만족할 때만 수행:
    /// 1. analyzer가 발화 임베딩을 함께 반환 (`PadAnalyzer` 등)
    /// 2. `pad_hint`가 없음 (수동 PAD는 그대로 사용)
    ///
    /// 변환 실패 시 화자 PAD를 그대로 사용하고 `tracing::warn!` 로그를 남긴다.
    #[cfg(feature = "listener_perspective")]
    pub fn with_converter(
        mut self,
        converter: Arc<dyn ListenerPerspectiveConverter>,
    ) -> Self {
        self.converter = Some(converter);
        self
    }

    /// 내부 CommandDispatcher에 대한 참조. dispatch_v2는 `&self`로 호출 가능하므로
    /// 이 참조만으로 외부에서 추가 커맨드를 발행하거나 broadcast 구독이 가능하다.
    pub fn dispatcher(&self) -> &CommandDispatcher<R> {
        &self.dispatcher
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

        let output = self.dispatcher.dispatch_v2(cmd).await?;
        let appraise_result = self.build_appraise_from_v2(&output, npc_id, partner_id)?;

        // format()은 내부에서 format_prompt를 한 번 호출하므로, 결과의 prompt를
        // 그대로 재사용하여 동일 가이드를 중복 포맷팅하지 않는다.
        let appraise_resp: AppraiseResponse = appraise_result.format(&*self.formatter);

        // NPC 성격 기반 생성 파라미터 유도 (옵션)
        let generation_config = self
            .dispatcher
            .repository_guard()
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

        // ② PAD 결정 — pad_hint > analyzer.analyze_with_embedding > None
        // utterance_embedding은 listener-perspective 변환에 재사용 (analyzer 경로일 때만 가용)
        let (speaker_pad, utterance_embedding): (Option<Pad>, Option<UtteranceEmbedding>) =
            match pad_hint {
                Some(p) => (Some(p), None),
                None => match self.analyzer.as_mut() {
                    Some(analyzer) => {
                        let (p, emb) = analyzer
                            .analyze_with_embedding(user_utterance)
                            .map_err(|e| DialogueAgentError::Analysis(e.to_string()))?;
                        (Some(p), emb)
                    }
                    None => (None, None),
                },
            };

        // ②.5 화자 PAD → 청자 PAD 변환 (Phase 7, listener_perspective feature)
        let pad = self.convert_to_listener_pad(
            user_utterance,
            speaker_pad,
            utterance_embedding.as_deref(),
        );

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
            let output = self.dispatcher.dispatch_v2(stim_cmd).await?;
            let stim_result =
                self.build_stimulus_from_v2(&output, &meta.npc_id, &meta.partner_id, pad)?;
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
            let output = self.dispatcher.dispatch_v2(cmd).await?;
            Some(self.build_end_dialogue_from_v2(&output)?)
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

    /// 화자 PAD를 청자 관점 PAD로 변환한다 (Phase 7).
    ///
    /// 도메인 헬퍼 `domain::listener_perspective::convert_or_fallback`에 위임 —
    /// converter 미주입 / 임베딩 부재 / 변환 실패 모두 화자 PAD 그대로 반환.
    /// LP feature off 빌드는 converter 필드 자체가 컴파일에서 제외되므로
    /// 항상 speaker PAD가 dispatch된다.
    fn convert_to_listener_pad(
        &self,
        _utterance: &str,
        speaker_pad: Option<Pad>,
        _utterance_embedding: Option<&[f32]>,
    ) -> Option<Pad> {
        let speaker = speaker_pad?;
        #[cfg(feature = "listener_perspective")]
        {
            let listener = crate::domain::listener_perspective::convert_or_fallback(
                self.converter.as_deref(),
                _utterance,
                speaker,
                _utterance_embedding,
            );
            tracing::debug!(
                "DialogueAgent.turn: PAD {{ P: {:.3}, A: {:.2}, D: {:.2} }} (converter on)",
                listener.pleasure,
                listener.arousal,
                listener.dominance
            );
            Some(listener)
        }
        #[cfg(not(feature = "listener_perspective"))]
        {
            Some(speaker)
        }
    }

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

        // v2 경로: event_store.append + event_bus.publish만. v2 inline handler들은
        // DialogueTurnCompleted에 관심 없으므로 projections().apply_all는 no-op였으며 제거.
        let store = self.dispatcher.event_store();
        let bus = self.dispatcher.event_bus();
        let id = store.next_id();
        let seq = store.next_sequence(&meta.npc_id);
        let event = DomainEvent::new(id, meta.npc_id.clone(), seq, payload);
        store.append(&[event.clone()]);
        bus.publish(&event);
    }

    /// 현재 저장된 감정 상태의 스냅샷을 가져온다.
    /// 저장소에 감정 상태가 없으면 빈 Vec 반환.
    fn current_emotion_snapshot(&self, npc_id: &str) -> Vec<(String, f32)> {
        self.dispatcher
            .repository_guard()
            .get_emotion_state(npc_id)
            .map(|s| emotion_snapshot(&s))
            .unwrap_or_default()
    }

    // -----------------------------------------------------------------------
    // v2 mapping helpers — DispatchV2Output → v1 DTO 재구성
    //
    // dispatch_v2는 HandlerShared + 이벤트 목록을 반환하므로, DialogueAgent의 기존
    // 반환 DTO (AppraiseResult / StimulusResult / AfterDialogueResponse)를
    // 만들려면 여기서 재조립한다. NPC/Partner 이름과 Relationship은 repo에서 조회.
    // -----------------------------------------------------------------------

    /// `dispatch_v2(Command::Appraise)` 결과 → `AppraiseResult`
    fn build_appraise_from_v2(
        &self,
        output: &crate::application::command::dispatcher::DispatchV2Output,
        npc_id: &str,
        partner_id: &str,
    ) -> Result<AppraiseResult, DialogueAgentError> {
        let state = output
            .shared
            .emotion_state
            .as_ref()
            .ok_or(DialogueAgentError::ResultReconstruction("EmotionState"))?;

        let (npc, partner_name, rel) = self.fetch_npc_partner_rel(npc_id, partner_id)?;

        // 이벤트에서 situation_description 추출 (EmotionAppraised payload)
        let situation_desc = output.events.iter().find_map(|e| match &e.payload {
            EventPayload::EmotionAppraised {
                situation_description, ..
            } => situation_description.clone(),
            _ => None,
        });

        // relationship은 shared 우선, 없으면 repo fallback
        let effective_rel = output.shared.relationship.as_ref().or(rel.as_ref());

        Ok(build_appraise_result(
            &npc,
            state,
            situation_desc,
            effective_rel,
            &partner_name,
            vec![],
        ))
    }

    /// `dispatch_v2(Command::ApplyStimulus)` 결과 → `StimulusResult`
    ///
    /// beat_changed는 `output.events`에 `BeatTransitioned`가 있는지로 판정 (v2 진실).
    fn build_stimulus_from_v2(
        &self,
        output: &crate::application::command::dispatcher::DispatchV2Output,
        npc_id: &str,
        partner_id: &str,
        input_pad: Pad,
    ) -> Result<StimulusResult, DialogueAgentError> {
        let state = output
            .shared
            .emotion_state
            .as_ref()
            .ok_or(DialogueAgentError::ResultReconstruction("EmotionState"))?;
        let guide = output
            .shared
            .guide
            .as_ref()
            .cloned()
            .ok_or(DialogueAgentError::ResultReconstruction("ActingGuide"))?;

        let (_, partner_name, rel) = self.fetch_npc_partner_rel(npc_id, partner_id)?;
        let _ = (partner_name, rel); // reserved for future richer mapping

        let (emotions, dominant, mood) = build_emotion_fields(state);
        let beat_changed = output
            .events
            .iter()
            .any(|e| matches!(e.kind(), EventKind::BeatTransitioned));

        // active_focus_id: repo의 현재 Scene에서 조회 (dispatch_v2 write-back 후 상태 반영됨)
        let active_focus_id = self
            .dispatcher
            .repository_guard()
            .get_scene()
            .and_then(|s| s.active_focus_id().map(|id| id.to_string()));

        Ok(StimulusResult {
            emotions,
            dominant,
            mood,
            guide,
            trace: vec![],
            beat_changed,
            active_focus_id,
            input_pad: Some(PadOutput {
                pleasure: input_pad.pleasure,
                arousal: input_pad.arousal,
                dominance: input_pad.dominance,
            }),
        })
    }

    /// `dispatch_v2(Command::EndDialogue)` 결과 → `AfterDialogueResponse`
    ///
    /// `RelationshipUpdated` 이벤트의 before/after 6필드로 스냅샷 재구성.
    fn build_end_dialogue_from_v2(
        &self,
        output: &crate::application::command::dispatcher::DispatchV2Output,
    ) -> Result<AfterDialogueResponse, DialogueAgentError> {
        output
            .events
            .iter()
            .find_map(|e| match &e.payload {
                EventPayload::RelationshipUpdated {
                    before_closeness,
                    before_trust,
                    before_power,
                    after_closeness,
                    after_trust,
                    after_power,
                    ..
                } => Some(AfterDialogueResponse {
                    before: RelationshipValues {
                        closeness: *before_closeness,
                        trust: *before_trust,
                        power: *before_power,
                    },
                    after: RelationshipValues {
                        closeness: *after_closeness,
                        trust: *after_trust,
                        power: *after_power,
                    },
                }),
                _ => None,
            })
            .ok_or(DialogueAgentError::ResultReconstruction(
                "RelationshipUpdated event",
            ))
    }

    /// NPC + Partner NPC (→ name) + 관계를 repo에서 한 번에 조회.
    fn fetch_npc_partner_rel(
        &self,
        npc_id: &str,
        partner_id: &str,
    ) -> Result<(Npc, String, Option<Relationship>), DialogueAgentError> {
        let guard = self.dispatcher.repository_guard();
        let npc = guard
            .get_npc(npc_id)
            .ok_or(DialogueAgentError::ResultReconstruction("Npc"))?;
        let partner_name = guard
            .get_npc(partner_id)
            .map(|p| p.name().to_string())
            .unwrap_or_else(|| partner_id.to_string());
        let rel = guard
            .get_relationship(npc_id, partner_id)
            .or_else(|| guard.get_relationship(partner_id, npc_id));
        Ok((npc, partner_name, rel))
    }
}


