// wuxia-core/src/experience/conversation_action.rs
//
// ConversationAction — NPC 대화 Action (Phase 2 MVP).
//
// Action trait 구현. tick()에서 플레이어 입력 → LLM 대사 생성 → 대사 반환.
// finish()에서 Conversation 이벤트 + Summarize 태스크 생성.
//
// Phase 2에서는 극단 체크(embedding + anchors)를 포함하지 않음.
// Phase 2.5에서 EmbeddingPort + ExtremeAnchorSet 통합 예정.

use crate::llm::port::LlmPort;
use crate::llm::types::{CharacterSamplingProfile, LlmRequest, Message, SystemSamplingConfig};
use crate::shared::id::{CharacterId, ExperienceId, LocationId};
use crate::shared::time::GameTime;

use super::action::{Action, ActionResult};
use super::event::{ExperienceEvent, ExperienceHeader};
use super::handler::{AsyncTask, DialogueTurn, Speaker};

// ---------------------------------------------------------------------------
// ConversationEndReason
// ---------------------------------------------------------------------------

/// 대화 종료 사유.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversationEndReason {
    /// 플레이어가 종료 커맨드 입력
    PlayerQuit,
    /// 외부에서 강제 종료 (전투 발생 등)
    ForceEnd,
    /// 최대 턴 수 도달
    MaxTurnsReached,
}

// ---------------------------------------------------------------------------
// ConversationConfig
// ---------------------------------------------------------------------------

/// NPC 대화 설정.
#[derive(Debug, Clone)]
pub struct ConversationConfig {
    /// 최대 턴 수 (기본 30)
    pub max_turns: u32,
    /// 종료 커맨드 (기본 "/quit")
    pub quit_command: String,
}

impl Default for ConversationConfig {
    fn default() -> Self {
        Self {
            max_turns: 30,
            quit_command: "/quit".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// ConversationAction
// ---------------------------------------------------------------------------

/// NPC 대화 Action.
///
/// `tick()`: 플레이어 입력 → LLM 대사 생성 → ActionResult 반환.
/// `finish()`: Conversation 이벤트 + Summarize 태스크 생성.
///
/// Phase 2 MVP — 극단 체크 없음.
pub struct ConversationAction {
    // --- 포트 ---
    llm: Box<dyn LlmPort>,

    // --- 대화 메타 ---
    subject: CharacterId,
    counterpart: CharacterId,
    location: LocationId,
    game_time: GameTime,
    system_prompt: String,
    config: ConversationConfig,

    // --- 턴 관리 ---
    turns: Vec<DialogueTurn>,
    turn_count: u32,
    finished: bool,
    end_reason: Option<ConversationEndReason>,

    // --- ID 생성 ---
    next_experience_id: u64,
}

impl ConversationAction {
    /// 새 ConversationAction 생성.
    pub fn new(
        llm: Box<dyn LlmPort>,
        subject: CharacterId,
        counterpart: CharacterId,
        location: LocationId,
        game_time: GameTime,
        system_prompt: String,
        config: ConversationConfig,
        start_experience_id: u64,
    ) -> Self {
        Self {
            llm,
            subject,
            counterpart,
            location,
            game_time,
            system_prompt,
            config,
            turns: Vec::new(),
            turn_count: 0,
            finished: false,
            end_reason: None,
            next_experience_id: start_experience_id,
        }
    }

    /// 종료 사유 반환.
    pub fn end_reason(&self) -> Option<ConversationEndReason> {
        self.end_reason
    }

    /// 턴 수 반환.
    pub fn turn_count(&self) -> u32 {
        self.turn_count
    }

    /// 대화 히스토리를 LLM Message로 변환.
    fn build_messages(&self) -> Vec<Message> {
        self.turns
            .iter()
            .map(|t| match t.speaker {
                Speaker::Player => Message::user(&t.text),
                Speaker::Npc => Message::assistant(&t.text),
            })
            .collect()
    }

    fn generate_id(&mut self) -> ExperienceId {
        let id = ExperienceId::new(self.next_experience_id);
        self.next_experience_id += 1;
        id
    }

    /// 원시 대화를 문자열로 재구성.
    fn build_raw_dialogue(&self) -> String {
        self.turns
            .iter()
            .map(|t| {
                let label = match t.speaker {
                    Speaker::Player => "플레이어",
                    Speaker::Npc => "NPC",
                };
                format!("{}: {}", label, t.text)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Action for ConversationAction {
    fn tick(&mut self, input: &str) -> ActionResult {
        // 1. 종료 커맨드 체크
        if input.trim() == self.config.quit_command {
            self.finished = true;
            self.end_reason = Some(ConversationEndReason::PlayerQuit);
            return ActionResult::empty();
        }

        // 2. 최대 턴 체크
        if self.turn_count >= self.config.max_turns {
            self.finished = true;
            self.end_reason = Some(ConversationEndReason::MaxTurnsReached);
            return ActionResult::empty();
        }

        // 3. 플레이어 입력 기록
        self.turns.push(DialogueTurn {
            speaker: Speaker::Player,
            text: input.to_string(),
        });

        // 4. LLM 대사 생성
        let messages = self.build_messages();
        let request = LlmRequest {
            system_prompt: self.system_prompt.clone(),
            messages,
            character_profile: CharacterSamplingProfile::default(),
            system_config: SystemSamplingConfig::default(),
            system_reminder: None,
        };

        let npc_text = match self.llm.generate(&request) {
            Ok(response) => response.text,
            Err(e) => {
                // LLM 오류 시 턴 취소 (플레이어 입력 제거)
                self.turns.pop();
                return ActionResult::with_output(format!("[LLM 오류: {}]", e));
            }
        };

        // 5. NPC 대사 기록
        self.turns.push(DialogueTurn {
            speaker: Speaker::Npc,
            text: npc_text.clone(),
        });
        self.turn_count += 1;

        ActionResult::with_output(npc_text)
    }

    fn finish(&mut self) -> ActionResult {
        let raw_dialogue = self.build_raw_dialogue();
        let exp_id = self.generate_id();

        let conversation_event = ExperienceEvent::Conversation {
            header: ExperienceHeader::new(
                exp_id,
                self.subject,
                self.game_time,
                self.location,
                5.0, // 기본 중요도 — 향후 turns/극단 횟수 기반 계산
            ),
            counterpart: self.counterpart,
            turns: self.turn_count,
            raw_dialogue: raw_dialogue.clone(),
        };

        let summarize_task = AsyncTask::Summarize {
            original_experience_id: exp_id,
            raw_dialogue,
            turns: self.turn_count,
        };

        ActionResult {
            output: String::new(),
            events: vec![conversation_event],
            tasks: vec![summarize_task],
        }
    }

    fn is_finished(&self) -> bool {
        self.finished
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::port::LlmTokenCallback;
    use crate::llm::types::{LlmError, LlmResponse, StopReason};

    // --- TestMockLlm ---

    struct TestMockLlm {
        fixed_response: String,
    }

    impl TestMockLlm {
        fn new(response: &str) -> Self {
            Self {
                fixed_response: response.to_string(),
            }
        }
    }

    impl LlmPort for TestMockLlm {
        fn generate_with_callback(
            &self,
            _request: &LlmRequest,
            _callback: Option<LlmTokenCallback>,
        ) -> Result<LlmResponse, LlmError> {
            Ok(LlmResponse {
                text: self.fixed_response.clone(),
                tokens_generated: 5,
                prompt_tokens: 10,
                stop_reason: StopReason::EndOfText,
            })
        }
    }

    struct ErrorMockLlm;

    impl LlmPort for ErrorMockLlm {
        fn generate_with_callback(
            &self,
            _request: &LlmRequest,
            _callback: Option<LlmTokenCallback>,
        ) -> Result<LlmResponse, LlmError> {
            Err(LlmError::generation_failed("테스트 오류"))
        }
    }

    fn make_action(llm: Box<dyn LlmPort>) -> ConversationAction {
        ConversationAction::new(
            llm,
            CharacterId::new(1),
            CharacterId::new(5),
            LocationId::new(10),
            GameTime::new(1200, 3, 15),
            "너는 소연이다.".to_string(),
            ConversationConfig::default(),
            100,
        )
    }

    fn make_action_with_max_turns(llm: Box<dyn LlmPort>, max_turns: u32) -> ConversationAction {
        ConversationAction::new(
            llm,
            CharacterId::new(1),
            CharacterId::new(5),
            LocationId::new(10),
            GameTime::new(1200, 3, 15),
            "너는 소연이다.".to_string(),
            ConversationConfig {
                max_turns,
                ..ConversationConfig::default()
            },
            100,
        )
    }

    // --- Tests ---

    #[test]
    fn tick_generates_npc_response() {
        let mut action = make_action(Box::new(TestMockLlm::new("반갑습니다, 소협.")));
        let result = action.tick("안녕하세요");

        assert_eq!(result.output, "반갑습니다, 소협.");
        assert!(result.events.is_empty());
        assert!(result.tasks.is_empty());
    }

    #[test]
    fn tick_records_turns() {
        let mut action = make_action(Box::new(TestMockLlm::new("응답")));

        action.tick("입력1");
        assert_eq!(action.turn_count(), 1);

        action.tick("입력2");
        assert_eq!(action.turn_count(), 2);
    }

    #[test]
    fn quit_command_finishes() {
        let mut action = make_action(Box::new(TestMockLlm::new("응답")));

        let result = action.tick("/quit");

        assert!(action.is_finished());
        assert_eq!(action.end_reason(), Some(ConversationEndReason::PlayerQuit));
        assert!(result.output.is_empty()); // quit은 출력 없음
    }

    #[test]
    fn max_turns_finishes() {
        let mut action = make_action_with_max_turns(Box::new(TestMockLlm::new("응답")), 2);

        action.tick("첫째");
        assert!(!action.is_finished());

        action.tick("둘째");
        assert!(!action.is_finished()); // 2턴 완료, 아직 안 끝남

        // 3번째 tick에서 max_turns 도달 감지
        let result = action.tick("셋째");
        assert!(action.is_finished());
        assert_eq!(
            action.end_reason(),
            Some(ConversationEndReason::MaxTurnsReached)
        );
        assert!(result.output.is_empty());
    }

    #[test]
    fn finish_emits_conversation_event() {
        let mut action = make_action(Box::new(TestMockLlm::new("응답")));
        action.tick("안녕");

        let result = action.finish();

        assert_eq!(result.events.len(), 1);
        match &result.events[0] {
            ExperienceEvent::Conversation { counterpart, turns, .. } => {
                assert_eq!(*counterpart, CharacterId::new(5));
                assert_eq!(*turns, 1);
            }
            other => panic!("Expected Conversation event, got {:?}", other),
        }
    }

    #[test]
    fn finish_emits_summarize_task() {
        let mut action = make_action(Box::new(TestMockLlm::new("응답")));
        action.tick("안녕");

        let result = action.finish();

        assert_eq!(result.tasks.len(), 1);
        match &result.tasks[0] {
            AsyncTask::Summarize { turns, .. } => {
                assert_eq!(*turns, 1);
            }
            other => panic!("Expected Summarize task, got {:?}", other),
        }
    }

    #[test]
    fn finish_raw_dialogue_contains_all_turns() {
        let mut action = make_action(Box::new(TestMockLlm::new("NPC 대사")));
        action.tick("플레이어 대사 1");
        action.tick("플레이어 대사 2");

        let result = action.finish();

        match &result.events[0] {
            ExperienceEvent::Conversation { raw_dialogue, .. } => {
                assert!(raw_dialogue.contains("플레이어 대사 1"));
                assert!(raw_dialogue.contains("플레이어 대사 2"));
                assert!(raw_dialogue.contains("NPC 대사"));
            }
            _ => panic!("Expected Conversation event"),
        }
    }

    #[test]
    fn llm_error_returns_error_output() {
        let mut action = make_action(Box::new(ErrorMockLlm));
        let result = action.tick("안녕");

        assert!(result.output.contains("LLM 오류"));
        assert_eq!(action.turn_count(), 0); // 오류 시 턴 카운트 안 올라감
    }

    #[test]
    fn game_loop_simulation() {
        let mut action: Box<dyn Action> =
            Box::new(make_action_with_max_turns(Box::new(TestMockLlm::new("응답")), 3));

        let inputs = ["입력1", "입력2", "입력3", "입력4"];
        let mut outputs = Vec::new();

        for input in &inputs {
            let result = action.tick(input);
            outputs.push(result.output);

            if action.is_finished() {
                let final_result = action.finish();
                assert_eq!(final_result.events.len(), 1);
                assert_eq!(final_result.tasks.len(), 1);
                break;
            }
        }

        // 3턴 응답 + 4번째 tick은 빈 출력 (max turns)
        assert_eq!(outputs.len(), 4);
        assert_eq!(outputs[0], "응답");
        assert!(outputs[3].is_empty()); // max turns 도달
    }

    #[test]
    fn action_as_trait_object() {
        let action: Box<dyn Action> = Box::new(make_action(Box::new(TestMockLlm::new("응답"))));
        assert!(!action.is_finished());
    }

    #[test]
    fn default_config() {
        let config = ConversationConfig::default();
        assert_eq!(config.max_turns, 30);
        assert_eq!(config.quit_command, "/quit");
    }
}
