//! CommandHandler — Agent가 구현하는 핸들러 인터페이스

use crate::domain::emotion::{EmotionState, Scene};
use crate::domain::event::EventPayload;
use crate::domain::personality::Npc;
use crate::domain::relationship::Relationship;

use super::types::CommandResult;

/// Agent에게 전달되는 읽기 전용 컨텍스트
///
/// Dispatcher가 repository에서 clone하여 구성합니다.
/// Agent는 이 데이터만 읽고, 결과를 `HandlerOutput`으로 반환합니다.
#[derive(Debug, Clone)]
pub struct HandlerContext {
    pub npc: Option<Npc>,
    pub relationship: Option<Relationship>,
    pub emotion_state: Option<EmotionState>,
    pub scene: Option<Scene>,
    pub partner_name: String,
}

/// Agent 처리 결과
///
/// Dispatcher가 `new_emotion_state` / `new_relationship`을 repository에 write-back하고,
/// `events`를 EventStore/EventBus로 발행합니다.
pub struct HandlerOutput {
    /// 도메인 결과
    pub result: CommandResult,
    /// 발행할 이벤트 페이로드 목록
    pub events: Vec<EventPayload>,
    /// Agent가 생성한 새 EmotionState (Dispatcher가 save)
    pub new_emotion_state: Option<(String, EmotionState)>,
    /// Agent가 생성한 새 Relationship (Dispatcher가 save)
    pub new_relationship: Option<(String, String, Relationship)>,
    /// 감정 초기화 대상 NPC ID
    pub clear_emotion: Option<String>,
    /// Scene 초기화 여부
    pub clear_scene: bool,
    /// Scene 저장
    pub save_scene: Option<Scene>,
}

impl HandlerOutput {
    /// 단순 결과 + 이벤트만 반환 (side-effect 없음)
    pub fn simple(result: CommandResult, events: Vec<EventPayload>) -> Self {
        Self {
            result,
            events,
            new_emotion_state: None,
            new_relationship: None,
            clear_emotion: None,
            clear_scene: false,
            save_scene: None,
        }
    }
}

/// emotion_snapshot 헬퍼: EmotionState → Vec<(String, f32)>
pub fn emotion_snapshot(state: &EmotionState) -> Vec<(String, f32)> {
    state
        .emotions()
        .iter()
        .map(|e| (format!("{:?}", e.emotion_type()), e.intensity()))
        .collect()
}
