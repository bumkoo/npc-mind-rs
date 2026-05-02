use serde::{Deserialize, Serialize};

/// 대화 종료 후 관계 갱신 응답
#[derive(Serialize, Deserialize, Clone)]
pub struct AfterDialogueResponse {
    pub before: RelationshipValues,
    pub after: RelationshipValues,
}

/// 관계 상태 요약 값
#[derive(Serialize, Deserialize, Clone)]
pub struct RelationshipValues {
    pub closeness: f32,
    pub trust: f32,
    pub power: f32,
}

/// 대화/Beat 종료 후 관계 갱신 요청
#[derive(Serialize, Deserialize, Clone)]
pub struct AfterDialogueRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub significance: Option<f32>,
}
