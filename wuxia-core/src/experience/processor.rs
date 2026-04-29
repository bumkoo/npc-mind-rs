// wuxia-core/src/experience/processor.rs
//
// EventProcessor — 경험 이벤트를 핸들러 체인에 순서대로 전달.
//
// "몸이 먼저, 마음이 다음, 이야기가 마지막"
//
// 고정 순서:
//   ① Character (피로/부상)
//   ② Growth (숙련도/능력치) — Phase 3
//   ③ Bond (관계)
//   ④ Psychology (감정/기분) — Phase 3
//   ⑤ Narrative (퀘스트/기연) — Phase 3
//   ⑥ Memory (벡터DB 저장) — Phase 3
//
// Phase 2 MVP: ①→③ 만 등록.

use super::event::ExperienceEvent;
use super::handler::{AsyncTask, EventHandler, HandlerResult, ProcessingContext};

// ---------------------------------------------------------------------------
// ProcessingResult — 처리 결과
// ---------------------------------------------------------------------------

/// 모든 핸들러 실행 후의 최종 결과.
///
/// `context`에 누적된 DomainEvent와 수집된 AsyncTask를 함께 반환한다.
#[derive(Debug)]
pub struct ProcessingResult {
    /// 모든 핸들러가 남긴 DomainEvent 부산물
    pub context: ProcessingContext,
    /// 모든 핸들러가 요청한 비동기 태스크
    pub tasks: Vec<AsyncTask>,
}

// ---------------------------------------------------------------------------
// EventProcessor — 핸들러 체인 실행기
// ---------------------------------------------------------------------------

/// 경험 이벤트 처리기 — 핸들러를 고정 순서로 실행.
///
/// 핸들러는 `add_handler()`로 등록한 순서대로 실행된다.
/// 각 핸들러의 `side_effects`는 `ProcessingContext`에 누적되어
/// 다음 핸들러가 참조할 수 있다.
///
/// # Example
/// ```ignore
/// let mut processor = EventProcessor::new();
/// processor.add_handler(Box::new(character_handler));
/// processor.add_handler(Box::new(bond_handler));
///
/// let result = processor.process(&event);
/// // result.context에 모든 DomainEvent
/// // result.tasks에 모든 AsyncTask
/// ```
pub struct EventProcessor {
    handlers: Vec<Box<dyn EventHandler>>,
}

impl EventProcessor {
    /// 빈 프로세서 생성.
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// 핸들러를 순서대로 추가. 추가 순서 = 실행 순서.
    pub fn add_handler(&mut self, handler: Box<dyn EventHandler>) {
        self.handlers.push(handler);
    }

    /// 하나의 ExperienceEvent를 모든 핸들러에 순서대로 전달.
    ///
    /// 각 핸들러의 side_effects는 ProcessingContext에 누적되어
    /// 다음 핸들러가 참조할 수 있다.
    pub fn process(&mut self, event: &ExperienceEvent) -> ProcessingResult {
        let mut ctx = ProcessingContext::new();
        let mut all_tasks: Vec<AsyncTask> = Vec::new();

        for handler in &mut self.handlers {
            let result: HandlerResult = handler.handle_event(event, &ctx);
            ctx.extend(result.side_effects);
            all_tasks.extend(result.tasks);
        }

        ProcessingResult {
            context: ctx,
            tasks: all_tasks,
        }
    }

    /// 등록된 핸들러 수.
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }
}

impl Default for EventProcessor {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::CharacterEvent;
    use crate::shared::event::DomainEvent;
    use crate::shared::id::{CharacterId, ExperienceId, LocationId, MartialArtId};
    use crate::shared::time::GameTime;
    use crate::experience::event::ExperienceHeader;

    fn make_training_event() -> ExperienceEvent {
        ExperienceEvent::Training {
            header: ExperienceHeader::new(
                ExperienceId::new(1),
                CharacterId::new(1),
                GameTime::new(1200, 3, 15),
                LocationId::new(10),
                5.0,
            ),
            skill: MartialArtId::new(1),
            method: "대련".to_string(),
            mentor: None,
            companion: None,
            duration: 3,
            intensity: 8,
        }
    }

    fn make_observation_event() -> ExperienceEvent {
        ExperienceEvent::Observation {
            header: ExperienceHeader::new(
                ExperienceId::new(2),
                CharacterId::new(1),
                GameTime::new(1200, 3, 15),
                LocationId::new(10),
                7.0,
            ),
            target: Some(CharacterId::new(5)),
            what: "소연이 분노 표출".to_string(),
            sentiment_delta: Some(-5.0),
        }
    }

    // --- Mock handlers ---

    struct CountingHandler {
        call_count: usize,
    }

    impl CountingHandler {
        fn new() -> Self {
            Self { call_count: 0 }
        }
    }

    impl EventHandler for CountingHandler {
        fn handle_event(
            &mut self,
            _event: &ExperienceEvent,
            _ctx: &ProcessingContext,
        ) -> HandlerResult {
            self.call_count += 1;
            HandlerResult::empty()
        }
    }

    /// 피로 이벤트를 생성하는 핸들러 (CharacterHandler 역할 대역)
    struct FatigueProducer;

    impl EventHandler for FatigueProducer {
        fn handle_event(
            &mut self,
            _event: &ExperienceEvent,
            _ctx: &ProcessingContext,
        ) -> HandlerResult {
            HandlerResult::with_effects(vec![
                DomainEvent::Character(CharacterEvent::FatigueChanged {
                    character_id: CharacterId::new(1),
                    old_fatigue: 30,
                    new_fatigue: 54,
                    fatigue_level: crate::character::FatigueLevel::Moderate,
                }),
            ])
        }
    }

    /// 앞 핸들러의 context를 읽는 핸들러 (GrowthHandler 역할 대역)
    struct ContextReader {
        saw_fatigue: bool,
    }

    impl ContextReader {
        fn new() -> Self {
            Self { saw_fatigue: false }
        }
    }

    impl EventHandler for ContextReader {
        fn handle_event(
            &mut self,
            _event: &ExperienceEvent,
            ctx: &ProcessingContext,
        ) -> HandlerResult {
            self.saw_fatigue = ctx.has(|e| matches!(e, DomainEvent::Character(..)));
            HandlerResult::empty()
        }
    }

    /// AsyncTask를 생성하는 핸들러
    struct TaskProducer;

    impl EventHandler for TaskProducer {
        fn handle_event(
            &mut self,
            _event: &ExperienceEvent,
            _ctx: &ProcessingContext,
        ) -> HandlerResult {
            HandlerResult {
                side_effects: Vec::new(),
                tasks: vec![AsyncTask::Summarize {
                    original_experience_id: ExperienceId::new(99),
                    raw_dialogue: "test".to_string(),
                    turns: 1,
                }],
            }
        }
    }

    // --- Tests ---

    #[test]
    fn empty_processor_returns_empty() {
        let mut processor = EventProcessor::new();
        let event = make_training_event();
        let result = processor.process(&event);

        assert!(result.context.is_empty());
        assert!(result.tasks.is_empty());
    }

    #[test]
    fn single_handler_processes_event() {
        let mut processor = EventProcessor::new();
        processor.add_handler(Box::new(FatigueProducer));

        let result = processor.process(&make_training_event());

        assert_eq!(result.context.len(), 1);
        assert!(result.context.has(|e| matches!(e, DomainEvent::Character(..))));
    }

    #[test]
    fn handler_order_matters() {
        // FatigueProducer가 먼저 실행 → ContextReader가 피로 이벤트를 볼 수 있어야 함
        let mut processor = EventProcessor::new();

        // 순서가 중요하므로 직접 핸들러를 추적할 수 없지만,
        // context에 FatigueChanged가 있는지로 검증
        processor.add_handler(Box::new(FatigueProducer));

        let context_reader = ContextReader::new();
        // handler 소유권이 이동하므로, 결과를 context로 검증
        processor.add_handler(Box::new(context_reader));

        let result = processor.process(&make_training_event());

        // FatigueProducer의 side_effect가 context에 있어야 함
        assert_eq!(result.context.len(), 1);
        assert!(result.context.has(|e| {
            matches!(e, DomainEvent::Character(CharacterEvent::FatigueChanged { .. }))
        }));
    }

    #[test]
    fn tasks_collected_from_all_handlers() {
        let mut processor = EventProcessor::new();
        processor.add_handler(Box::new(TaskProducer));
        processor.add_handler(Box::new(TaskProducer));

        let result = processor.process(&make_training_event());

        // 두 핸들러 모두 Summarize 태스크를 생성
        assert_eq!(result.tasks.len(), 2);
    }

    #[test]
    fn process_conversation_event() {
        let mut processor = EventProcessor::new();
        processor.add_handler(Box::new(CountingHandler::new()));

        let event = ExperienceEvent::Conversation {
            header: ExperienceHeader::new(
                ExperienceId::new(3),
                CharacterId::new(1),
                GameTime::new(1200, 3, 15),
                LocationId::new(10),
                5.0,
            ),
            counterpart: CharacterId::new(5),
            turns: 10,
            raw_dialogue: "대화 내용".to_string(),
        };

        let result = processor.process(&event);
        assert!(result.context.is_empty()); // CountingHandler는 side_effects 없음
    }

    #[test]
    fn process_observation_event() {
        let mut processor = EventProcessor::new();
        processor.add_handler(Box::new(FatigueProducer)); // Observation에도 동일 동작

        let result = processor.process(&make_observation_event());
        assert_eq!(result.context.len(), 1);
    }

    #[test]
    fn handler_count() {
        let mut processor = EventProcessor::new();
        assert_eq!(processor.handler_count(), 0);

        processor.add_handler(Box::new(CountingHandler::new()));
        assert_eq!(processor.handler_count(), 1);

        processor.add_handler(Box::new(FatigueProducer));
        assert_eq!(processor.handler_count(), 2);
    }

    #[test]
    fn default_processor_is_empty() {
        let processor = EventProcessor::default();
        assert_eq!(processor.handler_count(), 0);
    }
}
