use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct AppraiseRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub situation: SituationInput,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SituationInput {
    pub description: String,
    pub event: Option<EventInput>,
    pub action: Option<ActionInput>,
    pub object: Option<ObjectInput>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EventInput {
    pub description: String,
    pub desirability_for_self: f32,
    pub other: Option<EventOtherInput>,
    pub prospect: Option<String>, // "anticipation", "hope_fulfilled", etc.
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EventOtherInput {
    pub target_id: String,
    pub desirability: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ActionInput {
    pub description: String,
    pub agent_id: Option<String>, // None=자기, Some=타인
    pub praiseworthiness: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ObjectInput {
    pub target_id: String,
    pub appealingness: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppraiseResponse {
    pub emotions: Vec<EmotionOutput>,
    pub dominant: Option<EmotionOutput>,
    pub mood: f32,
    pub prompt: String,
    pub trace: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EmotionOutput {
    pub emotion_type: String,
    pub intensity: f32,
    pub context: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StimulusRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub situation_description: Option<String>,
    pub pleasure: f32,
    pub arousal: f32,
    pub dominance: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AfterDialogueRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub praiseworthiness: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AfterDialogueResponse {
    pub before: RelationshipValues,
    pub after: RelationshipValues,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RelationshipValues {
    pub closeness: f32,
    pub trust: f32,
    pub power: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GuideRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub situation_description: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GuideResponse {
    pub prompt: String,
    pub json: String,
}
