use serde::{Deserialize, Serialize};

/// `Command::SeedRumor` 요청 DTO.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SeedRumorRequest {
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub seed_content: Option<String>,
    pub reach: RumorReachInput,
    pub origin: RumorOriginInput,
}

/// `Command::SpreadRumor` 요청 DTO.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpreadRumorRequest {
    pub rumor_id: String,
    pub recipients: Vec<String>,
    #[serde(default)]
    pub content_version: Option<String>,
}

/// `ReachPolicy`에 매핑되는 DTO.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RumorReachInput {
    #[serde(default)]
    pub regions: Vec<String>,
    #[serde(default)]
    pub factions: Vec<String>,
    #[serde(default)]
    pub npc_ids: Vec<String>,
    #[serde(default)]
    pub min_significance: f32,
}

/// `RumorOrigin`에 매핑되는 DTO.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RumorOriginInput {
    Seeded,
    FromWorldEvent { event_id: u64 },
    Authored {
        #[serde(default)]
        by: Option<String>,
    },
}

impl From<RumorReachInput> for crate::domain::rumor::ReachPolicy {
    fn from(r: RumorReachInput) -> Self {
        Self {
            regions: r.regions,
            factions: r.factions,
            npc_ids: r.npc_ids,
            min_significance: r.min_significance,
        }
    }
}

impl From<&RumorReachInput> for crate::domain::rumor::ReachPolicy {
    fn from(r: &RumorReachInput) -> Self {
        Self {
            regions: r.regions.clone(),
            factions: r.factions.clone(),
            npc_ids: r.npc_ids.clone(),
            min_significance: r.min_significance,
        }
    }
}

impl From<RumorOriginInput> for crate::domain::rumor::RumorOrigin {
    fn from(o: RumorOriginInput) -> Self {
        match o {
            RumorOriginInput::Seeded => Self::Seeded,
            RumorOriginInput::FromWorldEvent { event_id } => Self::FromWorldEvent { event_id },
            RumorOriginInput::Authored { by } => Self::Authored { by },
        }
    }
}

impl From<&RumorOriginInput> for crate::domain::rumor::RumorOrigin {
    fn from(o: &RumorOriginInput) -> Self {
        match o {
            RumorOriginInput::Seeded => Self::Seeded,
            RumorOriginInput::FromWorldEvent { event_id } => Self::FromWorldEvent {
                event_id: *event_id,
            },
            RumorOriginInput::Authored { by } => Self::Authored { by: by.clone() },
        }
    }
}

/// `Command::SeedRumor` 응답.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SeedRumorResponse {
    pub rumor_id: String,
}

/// `Command::SpreadRumor` 응답.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpreadRumorResponse {
    pub rumor_id: String,
    pub hop_index: u32,
    pub memory_entry_ids: Vec<String>,
}
