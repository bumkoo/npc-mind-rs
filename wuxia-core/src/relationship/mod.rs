// wuxia-core/src/relationship/mod.rs
//
// 관계 도메인 (Relationship Domain)
// "이 둘은 어떤 사이인가?"
//
// 강호 인맥첩 — 두 캐릭터 사이의 호감, 신뢰를 추적한다.
// 관계 변화가 퀘스트 트리거, 피로 회복, NPC 행동에 영향을 미친다.
//
// # 모듈 구조
// - types:              Relationship aggregate root
// - relationship_type:  RelationshipType 값 객체
// - level:              RelationshipLevel 값 객체
// - trust_level:        TrustLevel 값 객체
// - event:              RelationshipEvent (도메인 이벤트)
// - port:               RelationshipRepository trait (헥사고날 출력 포트)

pub mod chronicle;
pub mod description;
pub mod effect;
pub mod event;
pub mod level;
pub mod port;
pub mod relationship_type;
pub mod sentiment;
pub mod trust_level;
pub mod types;

// Re-exports for convenience
pub use chronicle::{
    CauseSource, ChangeType, RelationshipChronicle,
};
pub use description::{LocalizedDesc, RelationshipDescriptions};
pub use effect::{ConversationEffect, apply_conversation_effect};
pub use event::RelationshipEvent;
pub use level::RelationshipLevel;
pub use port::{RelationshipRepository, ChronicleRepository};
pub use relationship_type::RelationshipType;
// SentimentDirection, SentimentJudgment, DeltaSource, judgment_to_delta → shared::sentiment
pub use sentiment::{
    ExtremeAnchorSet, ExtremeCheckResult, SentimentJudgeConfig, TurnCounter,
};
pub use trust_level::TrustLevel;
pub use types::{Affinity, Relationship, Trust};
