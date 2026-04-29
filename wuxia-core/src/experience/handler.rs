// wuxia-core/src/experience/handler.rs
//
// EventHandler trait & ProcessingContext — 경험 이벤트 처리 인프라.
//
// 하나의 ExperienceEvent를 처리할 때 6개 핸들러가 고정 순서로 실행된다:
//   ① 캐릭터 (피로/부상)
//   ② 성장   (숙련도/능력치)
//   ③ Bond   (관계)
//   ④ 심리   (감정/기분)
//   ⑤ 서사   (퀘스트/기연)
//   ⑥ 기억   (벡터DB 저장)
//
// ProcessingContext는 앞 핸들러의 DomainEvent를 뒤 핸들러가 참조할 수 있게
// 같은 처리 라운드 내에서 공유되는 컨테이너.
//
// "몸이 먼저, 마음이 다음, 이야기가 마지막"

use serde::{Deserialize, Serialize};

use crate::shared::event::DomainEvent;
use crate::shared::id::{CharacterId, ExperienceId};
use crate::shared::sentiment::SentimentDirection;

use super::event::ExperienceEvent;

// ---------------------------------------------------------------------------
// AsyncTask — 비동기 태스크 (Phase 1에서는 placeholder)
// ---------------------------------------------------------------------------

/// 감정 판정용 대화 턴. wuxia-core 내에서 사용하는 순수 도메인 타입.
///
/// wuxia-llm의 대화 턴과는 별개 — 어댑터 의존 방지.
/// 조립 계층(wuxia-app)이 변환을 담당한다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueTurn {
    /// 발화자
    pub speaker: Speaker,
    /// 발화 내용
    pub text: String,
}

/// 대화 발화자.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Speaker {
    /// 플레이어
    Player,
    /// NPC
    Npc,
}

/// 비동기로 처리할 작업.
///
/// 태스크 완료 시 결과는 ExperienceEvent로 변환되어 큐에 넣어진다.
///
/// - `SentimentJudgment` — 극단 트리거 시 LLM 감정 판정 (CTX2, ~3초)
/// - `Summarize` — 대화 종료 후 LLM 요약 (CTX3, ~3초)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AsyncTask {
    /// 극단 트리거 시 LLM 감정 판정.
    ///
    /// 완료 시 `ExperienceEvent::Observation`으로 큐에 넣음.
    SentimentJudgment {
        /// 판정 대상 대화 상대
        counterpart: CharacterId,
        /// 대화 히스토리
        dialogue_history: Vec<DialogueTurn>,
        /// 극단 트리거 방향
        trigger_direction: SentimentDirection,
    },

    /// 대화 종료 후 LLM 요약.
    ///
    /// 완료 시 `ExperienceEvent::ConversationSummarized`로 큐에 넣음.
    Summarize {
        /// 원본 대화의 experience_id
        original_experience_id: ExperienceId,
        /// 원시 대화 내용
        raw_dialogue: String,
        /// 대화 턴 수
        turns: u32,
    },
}

// ---------------------------------------------------------------------------
// HandlerResult — 핸들러의 반환물
// ---------------------------------------------------------------------------

/// 경험 이벤트 핸들러의 반환 결과.
///
/// `side_effects`는 ProcessingContext에 누적되어 뒤 핸들러가 참조한다.
/// `tasks`는 비동기 작업으로 spawn된다.
#[derive(Debug, Clone, PartialEq)]
pub struct HandlerResult {
    /// 처리 부산물 — ProcessingContext에 누적됨
    pub side_effects: Vec<DomainEvent>,
    /// spawn할 비동기 작업
    pub tasks: Vec<AsyncTask>,
}

impl HandlerResult {
    /// 부산물이 없는 빈 결과.
    pub fn empty() -> Self {
        Self {
            side_effects: Vec::new(),
            tasks: Vec::new(),
        }
    }

    /// DomainEvent만 있는 결과.
    pub fn with_effects(side_effects: Vec<DomainEvent>) -> Self {
        Self {
            side_effects,
            tasks: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// ProcessingContext — 핸들러 간 공유 컨테이너
// ---------------------------------------------------------------------------

/// 하나의 ExperienceEvent를 처리하는 동안 핸들러 간 공유되는 맥락.
///
/// 앞 핸들러가 `side_effects`로 반환한 DomainEvent가 여기 누적되고,
/// 뒤 핸들러가 `has()`나 `iter()`로 참조하여 연쇄 반응을 처리한다.
///
/// 각 ExperienceEvent 처리마다 새로 생성되고, 처리가 끝나면 버려진다.
///
/// # Example
/// ```
/// use wuxia_core::experience::{ProcessingContext};
/// use wuxia_core::shared::DomainEvent;
/// use wuxia_core::time::TimeEvent;
///
/// let mut ctx = ProcessingContext::new();
/// assert!(ctx.is_empty());
///
/// ctx.add(DomainEvent::Time(TimeEvent::YearPassed { new_year: 1201 }));
/// assert_eq!(ctx.len(), 1);
/// assert!(ctx.has(|e| matches!(e, DomainEvent::Time(..))));
/// ```
#[derive(Debug, Clone)]
pub struct ProcessingContext {
    side_effects: Vec<DomainEvent>,
}

impl ProcessingContext {
    /// 빈 ProcessingContext 생성.
    pub fn new() -> Self {
        Self {
            side_effects: Vec::new(),
        }
    }

    /// DomainEvent 하나를 추가.
    pub fn add(&mut self, event: DomainEvent) {
        self.side_effects.push(event);
    }

    /// 여러 DomainEvent를 한번에 추가.
    pub fn extend(&mut self, events: Vec<DomainEvent>) {
        self.side_effects.extend(events);
    }

    /// 조건에 맞는 DomainEvent가 있는지 확인.
    pub fn has<F: Fn(&DomainEvent) -> bool>(&self, predicate: F) -> bool {
        self.side_effects.iter().any(predicate)
    }

    /// 누적된 모든 DomainEvent를 순회.
    pub fn iter(&self) -> impl Iterator<Item = &DomainEvent> {
        self.side_effects.iter()
    }

    /// 누적된 DomainEvent 슬라이스 반환.
    pub fn side_effects(&self) -> &[DomainEvent] {
        &self.side_effects
    }

    /// 누적된 DomainEvent 수.
    pub fn len(&self) -> usize {
        self.side_effects.len()
    }

    /// 비어있는지 확인.
    pub fn is_empty(&self) -> bool {
        self.side_effects.is_empty()
    }
}

impl Default for ProcessingContext {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// EventHandler trait — 경험 이벤트 핸들러
// ---------------------------------------------------------------------------

/// 경험 이벤트 핸들러 — 각 도메인이 구현.
///
/// 하나의 ExperienceEvent에 대해 자기 도메인의 상태를 갱신하고,
/// 부산물(DomainEvent)과 비동기 태스크를 반환한다.
///
/// # 구현 예정 (Phase 2~3)
/// - `CharacterHandler` — 피로/부상 계산
/// - `GrowthHandler` — 숙련도/능력치 계산
/// - `BondHandler` — 관계 갱신
/// - `PsychologyHandler` — 감정/기분 계산
/// - `NarrativeHandler` — 퀘스트/기연 확인
/// - `MemoryHandler` — 벡터DB 저장
pub trait EventHandler: Send + Sync {
    /// 경험 이벤트를 처리한다.
    ///
    /// # Arguments
    /// * `event` — 처리할 경험 이벤트
    /// * `ctx` — 앞 핸들러들이 남긴 처리 부산물 (읽기 전용)
    ///
    /// # Returns
    /// * `HandlerResult` — 이 핸들러의 부산물과 비동기 태스크
    fn handle_event(
        &mut self,
        event: &ExperienceEvent,
        ctx: &ProcessingContext,
    ) -> HandlerResult;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::CharacterEvent;
    use crate::shared::id::CharacterId;
    use crate::time::TimeEvent;

    // -- ProcessingContext --

    #[test]
    fn context_starts_empty() {
        let ctx = ProcessingContext::new();
        assert!(ctx.is_empty());
        assert_eq!(ctx.len(), 0);
        assert_eq!(ctx.side_effects().len(), 0);
    }

    #[test]
    fn context_add_single() {
        let mut ctx = ProcessingContext::new();
        ctx.add(DomainEvent::Time(TimeEvent::YearPassed { new_year: 1201 }));
        assert_eq!(ctx.len(), 1);
        assert!(!ctx.is_empty());
    }

    #[test]
    fn context_extend_multiple() {
        let mut ctx = ProcessingContext::new();
        ctx.extend(vec![
            DomainEvent::Time(TimeEvent::YearPassed { new_year: 1201 }),
            DomainEvent::Character(CharacterEvent::Aged {
                character_id: CharacterId::new(1),
                new_age: 26,
            }),
        ]);
        assert_eq!(ctx.len(), 2);
    }

    #[test]
    fn context_has_predicate() {
        let mut ctx = ProcessingContext::new();
        ctx.add(DomainEvent::Time(TimeEvent::YearPassed { new_year: 1201 }));

        // 시간 이벤트가 있는지 확인
        assert!(ctx.has(|e| matches!(e, DomainEvent::Time(..))));
        // 캐릭터 이벤트는 없음
        assert!(!ctx.has(|e| matches!(e, DomainEvent::Character(..))));
    }

    #[test]
    fn context_iter() {
        let mut ctx = ProcessingContext::new();
        ctx.add(DomainEvent::Time(TimeEvent::YearPassed { new_year: 1201 }));
        ctx.add(DomainEvent::Time(TimeEvent::YearPassed { new_year: 1202 }));

        let names: Vec<&str> = ctx.iter().map(|e| e.name()).collect();
        assert_eq!(names, vec!["YearPassed", "YearPassed"]);
    }

    #[test]
    fn context_side_effects_slice() {
        let mut ctx = ProcessingContext::new();
        ctx.add(DomainEvent::Time(TimeEvent::YearPassed { new_year: 1201 }));
        assert_eq!(ctx.side_effects().len(), 1);
    }

    #[test]
    fn context_default() {
        let ctx = ProcessingContext::default();
        assert!(ctx.is_empty());
    }

    // -- HandlerResult --

    #[test]
    fn handler_result_empty() {
        let result = HandlerResult::empty();
        assert!(result.side_effects.is_empty());
        assert!(result.tasks.is_empty());
    }

    #[test]
    fn handler_result_with_effects() {
        let result = HandlerResult::with_effects(vec![
            DomainEvent::Time(TimeEvent::YearPassed { new_year: 1201 }),
        ]);
        assert_eq!(result.side_effects.len(), 1);
        assert!(result.tasks.is_empty());
    }

    // -- Mock EventHandler --

    struct MockHandler {
        call_count: usize,
    }

    impl MockHandler {
        fn new() -> Self {
            Self { call_count: 0 }
        }
    }

    impl EventHandler for MockHandler {
        fn handle_event(
            &mut self,
            _event: &ExperienceEvent,
            _ctx: &ProcessingContext,
        ) -> HandlerResult {
            self.call_count += 1;
            HandlerResult::empty()
        }
    }

    #[test]
    fn mock_handler_implements_trait() {
        use crate::shared::id::{ExperienceId, LocationId, MartialArtId};
        use crate::shared::time::GameTime;
        use super::super::event::ExperienceHeader;

        let mut handler = MockHandler::new();
        let event = ExperienceEvent::Training {
            header: ExperienceHeader::new(
                ExperienceId::new(1),
                CharacterId::new(1),
                GameTime::new(1200, 3, 15),
                LocationId::new(10),
                5.0,
            ),
            skill: MartialArtId::new(1),
            method: String::new(),
            mentor: None,
            companion: None,
            duration: 1,
            intensity: 5,
        };
        let ctx = ProcessingContext::new();

        let result = handler.handle_event(&event, &ctx);
        assert!(result.side_effects.is_empty());
        assert_eq!(handler.call_count, 1);
    }

    #[test]
    fn handler_as_trait_object() {
        // dyn EventHandler로 사용 가능한지 확인 (Bevy Resource 대비)
        let handler: Box<dyn EventHandler> = Box::new(MockHandler::new());
        assert!(std::mem::size_of_val(&handler) > 0);
    }

    // -- DialogueTurn & Speaker --

    #[test]
    fn dialogue_turn_serialization() {
        let turn = DialogueTurn {
            speaker: Speaker::Player,
            text: "안녕하세요, 소연 낭자.".to_string(),
        };
        let json = serde_json::to_string(&turn).unwrap();
        let restored: DialogueTurn = serde_json::from_str(&json).unwrap();
        assert_eq!(turn, restored);
    }

    #[test]
    fn speaker_variants() {
        assert_ne!(Speaker::Player, Speaker::Npc);
        let json_p = serde_json::to_string(&Speaker::Player).unwrap();
        let json_n = serde_json::to_string(&Speaker::Npc).unwrap();
        assert_ne!(json_p, json_n);
    }

    // -- AsyncTask --

    #[test]
    fn sentiment_judgment_task_serialization() {
        use crate::shared::sentiment::SentimentDirection;

        let task = AsyncTask::SentimentJudgment {
            counterpart: CharacterId::new(5),
            dialogue_history: vec![
                DialogueTurn { speaker: Speaker::Player, text: "사부님 이야기 해주세요.".to_string() },
                DialogueTurn { speaker: Speaker::Npc, text: "사부님은... 돌아가셨어요.".to_string() },
            ],
            trigger_direction: SentimentDirection::Warmth,
        };
        let json = serde_json::to_string(&task).unwrap();
        let restored: AsyncTask = serde_json::from_str(&json).unwrap();
        assert_eq!(task, restored);
    }

    #[test]
    fn summarize_task_serialization() {
        use crate::shared::id::ExperienceId;

        let task = AsyncTask::Summarize {
            original_experience_id: ExperienceId::new(42),
            raw_dialogue: "플레이어: 안녕\nNPC: 반갑습니다".to_string(),
            turns: 2,
        };
        let json = serde_json::to_string(&task).unwrap();
        let restored: AsyncTask = serde_json::from_str(&json).unwrap();
        assert_eq!(task, restored);
    }

    #[test]
    fn handler_result_with_tasks() {
        use crate::shared::id::ExperienceId;

        let result = HandlerResult {
            side_effects: Vec::new(),
            tasks: vec![AsyncTask::Summarize {
                original_experience_id: ExperienceId::new(1),
                raw_dialogue: String::new(),
                turns: 0,
            }],
        };
        assert!(result.side_effects.is_empty());
        assert_eq!(result.tasks.len(), 1);
    }

    // -- 핸들러 순차 실행 시뮬레이션 --

    #[test]
    fn sequential_handler_execution_with_context() {
        use crate::shared::id::{ExperienceId, LocationId, MartialArtId};
        use crate::shared::time::GameTime;
        use super::super::event::ExperienceHeader;

        // ① 캐릭터 핸들러가 피로 이벤트를 생성
        struct FatigueHandler;
        impl EventHandler for FatigueHandler {
            fn handle_event(
                &mut self,
                _event: &ExperienceEvent,
                _ctx: &ProcessingContext,
            ) -> HandlerResult {
                HandlerResult::with_effects(vec![
                    DomainEvent::Character(CharacterEvent::FatigueChanged {
                        character_id: CharacterId::new(1),
                        old_fatigue: 50,
                        new_fatigue: 81,
                        fatigue_level: crate::character::FatigueLevel::Exhausted,
                    }),
                ])
            }
        }

        // ② 성장 핸들러가 피로 이벤트를 확인하여 효율 감소 적용
        struct GrowthHandler {
            fatigue_detected: bool,
        }
        impl EventHandler for GrowthHandler {
            fn handle_event(
                &mut self,
                _event: &ExperienceEvent,
                ctx: &ProcessingContext,
            ) -> HandlerResult {
                self.fatigue_detected = ctx.has(|e| {
                    matches!(e, DomainEvent::Character(CharacterEvent::FatigueChanged {
                        new_fatigue, ..
                    }) if *new_fatigue > 80)
                });
                HandlerResult::empty()
            }
        }

        let event = ExperienceEvent::Training {
            header: ExperienceHeader::new(
                ExperienceId::new(1),
                CharacterId::new(1),
                GameTime::new(1200, 3, 15),
                LocationId::new(10),
                5.0,
            ),
            skill: MartialArtId::new(1),
            method: String::new(),
            mentor: None,
            companion: None,
            duration: 3,
            intensity: 8,
        };

        // 시뮬레이션: 핸들러 순차 실행
        let mut ctx = ProcessingContext::new();

        let mut fatigue_handler = FatigueHandler;
        let result = fatigue_handler.handle_event(&event, &ctx);
        ctx.extend(result.side_effects);

        let mut growth_handler = GrowthHandler { fatigue_detected: false };
        growth_handler.handle_event(&event, &ctx);

        // 성장 핸들러가 피로 81 돌파를 감지했는지 확인
        assert!(growth_handler.fatigue_detected);
        assert_eq!(ctx.len(), 1);
    }
}
