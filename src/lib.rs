//! NPC Mind Engine
//!
//! 성격(HEXACO)이 상황(Context)을 해석하여
//! 감정(OCC)을 생성하고, LLM 연기 가이드를 출력하는 엔진.

pub mod application;
pub mod domain;
pub mod ports;
pub mod presentation;
pub mod adapter;

// ---------------------------------------------------------------------------
// 편의 재노출 — 라이브러리 사용자의 주요 진입점
// ---------------------------------------------------------------------------

pub use application::mind_service::{MindService, MindRepository, MindServiceError};
pub use application::formatted_service::FormattedMindService;
pub use application::dto::{
    AppraiseRequest, AppraiseResponse, AppraiseResult,
    StimulusRequest, StimulusResponse, StimulusResult,
    GuideRequest, GuideResponse, GuideResult,
    AfterDialogueRequest, AfterDialogueResponse,
    SceneRequest, SceneResponse,
};
pub use ports::{GuideFormatter, Appraiser, StimulusProcessor, PersonalityProfile};
pub use presentation::formatter::LocaleFormatter;
pub use presentation::builtin_toml;
