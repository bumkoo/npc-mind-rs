use serde::{Deserialize, Serialize};

/// `Command::TellInformation` 요청 DTO.
#[derive(Serialize, Deserialize, Clone)]
pub struct TellInformationRequest {
    pub speaker: String,
    pub listeners: Vec<String>,
    #[serde(default)]
    pub overhearers: Vec<String>,
    pub claim: String,
    pub stated_confidence: f32,
    #[serde(default)]
    pub origin_chain_in: Vec<String>,
    #[serde(default)]
    pub topic: Option<String>,
}

/// `Command::TellInformation` 응답 DTO.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TellInformationResponse {
    pub listeners_informed: usize,
    pub memory_entry_ids: Vec<String>,
}
