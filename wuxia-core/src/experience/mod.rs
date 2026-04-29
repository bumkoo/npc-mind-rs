// wuxia-core/src/experience/mod.rs
//
// Experience Domain — 경험-기억-이벤트 통합 아키텍처.
//
// 경험이 발생하면 그것이 곧 기억이 되고,
// 동시에 이벤트로 각 도메인에 전달된다.
// 모든 현재 상태는 기억의 누적 결과.
//
// 핵심 원칙:
//   - ExperienceEvent만 큐에 들어간다 (DomainEvent는 ProcessingContext에)
//   - 핸들러는 고정 순서로 실행 (캐릭터→성장→Bond→심리→서사→기억)
//   - 비동기 결과도 ExperienceEvent로 큐에 들어온다
//   - Action이 tick/finish로 ExperienceEvent를 생성한다

pub mod action;
pub mod bus;
pub mod conversation_action;
pub mod event;
pub mod handler;
pub mod handlers;
pub mod processor;

// Re-exports
pub use action::{Action, ActionResult};
pub use bus::{EventBus, InMemoryEventBus};
pub use conversation_action::{ConversationAction, ConversationConfig, ConversationEndReason};
pub use event::{CombatResult, ExperienceEvent, ExperienceHeader};
pub use handler::{AsyncTask, DialogueTurn, EventHandler, HandlerResult, ProcessingContext, Speaker};
pub use handlers::bond_handler::{BondHandler, RelationshipKey};
pub use handlers::character_handler::CharacterHandler;
pub use processor::{EventProcessor, ProcessingResult};
