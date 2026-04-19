pub mod agents;
pub mod dispatcher;
pub mod handler;
pub mod handler_v2;
pub mod priority;
pub mod projection_handlers;
pub mod types;

pub use agents::{EmotionAgent, GuideAgent, RelationshipAgent};
pub use dispatcher::CommandDispatcher;
// B5.1 (v0.2.0): v1 HandlerContext/HandlerOutput는 deprecated. 호환성 유지 위해 재-export.
#[allow(deprecated)]
pub use handler::{HandlerContext, HandlerOutput};
pub use projection_handlers::{
    EmotionProjectionHandler, RelationshipProjectionHandler, SceneProjectionHandler,
};
pub use types::{Command, CommandResult};
