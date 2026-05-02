#[cfg(feature = "chat")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerHealth {
    pub status: String,
}

#[cfg(feature = "chat")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InferenceSlotInfo {
    pub id: u32,
    #[serde(default)]
    pub state: u32,
    #[serde(default)]
    pub n_past: u32,
    #[serde(default)]
    pub n_predicted: u32,
    #[serde(default)]
    pub is_processing: bool,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

#[cfg(feature = "chat")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerMetrics {
    pub raw: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_total: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_predicted_total: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_seconds_total: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_predicted_seconds_total: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_decode_total: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_busy_slots_per_decode: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predicted_tokens_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kv_cache_usage_ratio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kv_cache_tokens: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requests_processing: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requests_deferred: Option<f64>,
}

#[cfg(feature = "chat")]
impl ServerMetrics {
    pub fn parse(raw: &str) -> Self {
        fn extract(text: &str, key: &str) -> Option<f64> {
            text.lines()
                .find(|l| l.starts_with(key) && !l.starts_with('#'))
                .and_then(|l| l.split_whitespace().last()?.parse().ok())
        }
        Self {
            prompt_tokens_total: extract(raw, "llamacpp:prompt_tokens_total"),
            tokens_predicted_total: extract(raw, "llamacpp:tokens_predicted_total"),
            prompt_seconds_total: extract(raw, "llamacpp:prompt_seconds_total"),
            tokens_predicted_seconds_total: extract(raw, "llamacpp:tokens_predicted_seconds_total"),
            n_decode_total: extract(raw, "llamacpp:n_decode_total"),
            n_busy_slots_per_decode: extract(raw, "llamacpp:n_busy_slots_per_decode"),
            prompt_tokens_seconds: extract(raw, "llamacpp:prompt_tokens_seconds"),
            predicted_tokens_seconds: extract(raw, "llamacpp:predicted_tokens_seconds"),
            kv_cache_usage_ratio: extract(raw, "llamacpp:kv_cache_usage_ratio"),
            kv_cache_tokens: extract(raw, "llamacpp:kv_cache_tokens"),
            requests_processing: extract(raw, "llamacpp:requests_processing"),
            requests_deferred: extract(raw, "llamacpp:requests_deferred"),
            raw: raw.to_string(),
        }
    }
}

/// LLM 추론 서버 모니터링 API 포트
#[cfg(feature = "chat")]
#[async_trait::async_trait]
pub trait InferenceServerMonitor: Send + Sync {
    async fn health(&self) -> Result<ServerHealth, String>;
    async fn slots(&self) -> Result<Vec<InferenceSlotInfo>, String>;
    async fn metrics(&self) -> Result<ServerMetrics, String>;
}
