use serde::{Deserialize, Serialize};

/// `Command::ApplyWorldEvent` 요청 DTO.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApplyWorldEventRequest {
    pub world_id: String,
    #[serde(default)]
    pub topic: Option<String>,
    pub fact: String,
    #[serde(default = "default_world_significance")]
    pub significance: f32,
    #[serde(default)]
    pub witnesses: Vec<String>,
}

fn default_world_significance() -> f32 {
    0.5
}
