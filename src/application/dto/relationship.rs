use serde::{Deserialize, Serialize};

use crate::domain::reflection::ReflectionResult;

/// 대화 종료 후 관계 갱신 응답
///
/// `reflection` (Phase 1 Mind Architecture, relationships.md v0.7 §6) — chat feature
/// 활성 + ReflectionService 부착 + dispatch 시 채워짐. chat 비활성 또는 미부착이면
/// `None`. `ReflectionResult`는 chat feature 무관 순수 도메인 타입이라 모든 빌드에서
/// serde 호환 (Stage 0 Findings F2 #1).
///
/// chitchat skip 케이스에도 `reflection`은 `Some(_)` (DialogueReflected 박제 보존),
/// 단 `before == after` (axes 변화 0). Frontend는 `before/after` 비교로 outer loop
/// 진입 여부 판별 가능.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AfterDialogueResponse {
    pub before: RelationshipValues,
    pub after: RelationshipValues,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reflection: Option<ReflectionResult>,
}

/// 관계 상태 요약 값
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RelationshipValues {
    pub closeness: f32,
    pub trust: f32,
    pub power: f32,
}

/// 대화/Beat 종료 후 관계 갱신 요청
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AfterDialogueRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub significance: Option<f32>,
}
