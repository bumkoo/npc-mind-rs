// wuxia-core/src/experience/action.rs
//
// Action trait — 행동 추상화.
//
// 게임 루프가 각 행동의 세부 로직을 모른 채 사용할 수 있도록
// 5가지 마이크로 루프 행동(대화/탐색/수련/전투/거래)을 일반화한다.
//
// | Action             | tick 1회      | 종료 조건          |
// |--------------------|---------------|-------------------|
// | ConversationAction | 대화 1턴      | /quit 또는 ForceEnd |
// | TrainingAction     | 수련 1시간대   | 목표 시간 도달      |
// | CombatAction       | 전투 1라운드   | 승/패/도주          |
// | TradeAction        | 거래 1단계    | 확정/취소           |
// | ExploreAction      | 이동 1구간    | 도착 또는 조우       |
//
// Phase 1에서는 trait과 ActionResult만 정의.
// 구체적인 Action 구현은 Phase 2~3에서 추가한다.

use super::event::ExperienceEvent;
use super::handler::AsyncTask;

// ---------------------------------------------------------------------------
// ActionResult — Action 한 틱의 반환물
// ---------------------------------------------------------------------------

/// Action 한 틱(tick)의 반환 결과.
///
/// 게임 루프는 이 결과만 보고 처리한다:
/// - `output` → 화면에 출력
/// - `events` → 이벤트 큐에 push
/// - `tasks` → pending_tasks에 spawn
///
/// # Example
/// ```
/// use wuxia_core::experience::ActionResult;
///
/// let result = ActionResult::with_output("소연: 반갑습니다, 소협.".to_string());
/// assert_eq!(result.output, "소연: 반갑습니다, 소협.");
/// assert!(result.events.is_empty());
/// assert!(result.tasks.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct ActionResult {
    /// 화면에 보여줄 텍스트 출력
    pub output: String,
    /// 이벤트 큐에 넣을 경험 이벤트
    pub events: Vec<ExperienceEvent>,
    /// spawn할 비동기 작업
    pub tasks: Vec<AsyncTask>,
}

impl ActionResult {
    /// 빈 결과 — 출력도 이벤트도 없음.
    pub fn empty() -> Self {
        Self {
            output: String::new(),
            events: Vec::new(),
            tasks: Vec::new(),
        }
    }

    /// 텍스트 출력만 있는 결과.
    pub fn with_output(output: String) -> Self {
        Self {
            output,
            events: Vec::new(),
            tasks: Vec::new(),
        }
    }

    /// 이벤트만 있는 결과 (화면 출력 없음).
    pub fn with_events(events: Vec<ExperienceEvent>) -> Self {
        Self {
            output: String::new(),
            events,
            tasks: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Action trait — 행동 추상화
// ---------------------------------------------------------------------------

/// 행동 추상화 — 게임 루프가 Action의 종류를 모른 채 사용.
///
/// 게임 루프:
/// ```ignore
/// loop {
///     let input = read_input();
///     let result = current_action.tick(&input);
///     for event in result.events { queue.push(event); }
///     for task in result.tasks { pending_tasks.push(spawn(task)); }
///     println!("{}", result.output);
///
///     if current_action.is_finished() {
///         let final_result = current_action.finish();
///         // ... final_result 처리 ...
///     }
/// }
/// ```
///
/// # 구현 예정 (Phase 2~3)
/// - `ConversationAction` — NPC 대화 (극단 체크 + 비동기 감정 판정)
/// - `TrainingAction` — 수련 (피로/성장 계산)
/// - `CombatAction` — 전투 (대결 시뮬레이션)
/// - `TradeAction` — 거래 (물품 교환)
/// - `ExploreAction` — 탐색 (이동 + 조우)
pub trait Action: Send {
    /// 플레이어 입력 한 틱을 처리한다.
    ///
    /// # Arguments
    /// * `input` — 플레이어 입력 텍스트
    ///
    /// # Returns
    /// * `ActionResult` — 출력, 이벤트, 비동기 태스크
    fn tick(&mut self, input: &str) -> ActionResult;

    /// 행동을 종료하고 최종 결과를 반환한다.
    ///
    /// 대화 종료 시 Conversation 이벤트 + Summarize 태스크를 반환하는 등.
    fn finish(&mut self) -> ActionResult;

    /// 행동이 완료되었는지 확인한다.
    fn is_finished(&self) -> bool;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- ActionResult --

    #[test]
    fn action_result_empty() {
        let result = ActionResult::empty();
        assert!(result.output.is_empty());
        assert!(result.events.is_empty());
        assert!(result.tasks.is_empty());
    }

    #[test]
    fn action_result_with_output() {
        let result = ActionResult::with_output("소연: 안녕하세요.".to_string());
        assert_eq!(result.output, "소연: 안녕하세요.");
        assert!(result.events.is_empty());
    }

    #[test]
    fn action_result_with_events() {
        use crate::experience::event::ExperienceHeader;
        use crate::shared::id::{CharacterId, ExperienceId, LocationId};
        use crate::shared::time::GameTime;

        let event = ExperienceEvent::Rest {
            header: ExperienceHeader::new(
                ExperienceId::new(1),
                CharacterId::new(1),
                GameTime::new(1200, 1, 1),
                LocationId::new(1),
                3.0,
            ),
            method: String::new(),
            recovery: 0.5,
        };
        let result = ActionResult::with_events(vec![event]);
        assert!(result.output.is_empty());
        assert_eq!(result.events.len(), 1);
    }

    // -- Mock Action --

    struct MockAction {
        turns: usize,
        max_turns: usize,
    }

    impl MockAction {
        fn new(max_turns: usize) -> Self {
            Self { turns: 0, max_turns }
        }
    }

    impl Action for MockAction {
        fn tick(&mut self, input: &str) -> ActionResult {
            self.turns += 1;
            ActionResult::with_output(format!("[턴 {}] 입력: {}", self.turns, input))
        }

        fn finish(&mut self) -> ActionResult {
            ActionResult::with_output(format!("{}턴 만에 종료", self.turns))
        }

        fn is_finished(&self) -> bool {
            self.turns >= self.max_turns
        }
    }

    #[test]
    fn mock_action_lifecycle() {
        let mut action = MockAction::new(3);
        assert!(!action.is_finished());

        let r1 = action.tick("안녕");
        assert!(r1.output.contains("턴 1"));
        assert!(!action.is_finished());

        action.tick("두번째");
        action.tick("세번째");
        assert!(action.is_finished());

        let final_result = action.finish();
        assert!(final_result.output.contains("3턴"));
    }

    #[test]
    fn action_as_trait_object() {
        let action: Box<dyn Action> = Box::new(MockAction::new(1));
        assert!(!action.is_finished());
    }

    // -- 게임 루프 시뮬레이션 --

    #[test]
    fn game_loop_simulation() {
        let mut action: Box<dyn Action> = Box::new(MockAction::new(2));
        let inputs = vec!["입력1", "입력2"];
        let mut outputs = Vec::new();

        for input in &inputs {
            let result = action.tick(input);
            outputs.push(result.output);

            if action.is_finished() {
                let final_result = action.finish();
                outputs.push(final_result.output);
                break;
            }
        }

        assert_eq!(outputs.len(), 3); // 2 ticks + 1 finish
        assert!(outputs[2].contains("2턴"));
    }
}
