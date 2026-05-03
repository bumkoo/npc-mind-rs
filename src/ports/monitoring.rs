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
        let mut metrics = Self {
            prompt_tokens_total: None,
            tokens_predicted_total: None,
            prompt_seconds_total: None,
            tokens_predicted_seconds_total: None,
            n_decode_total: None,
            n_busy_slots_per_decode: None,
            prompt_tokens_seconds: None,
            predicted_tokens_seconds: None,
            kv_cache_usage_ratio: None,
            kv_cache_tokens: None,
            requests_processing: None,
            requests_deferred: None,
            raw: raw.to_string(),
        };

        for line in raw.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            let mut parts = line.split_whitespace();
            let Some(key) = parts.next() else { continue };
            let Some(value_str) = parts.next() else { continue };
            let Ok(value) = value_str.parse::<f64>() else {
                continue;
            };

            match key {
                "llamacpp:prompt_tokens_total" => metrics.prompt_tokens_total = Some(value),
                "llamacpp:tokens_predicted_total" => metrics.tokens_predicted_total = Some(value),
                "llamacpp:prompt_seconds_total" => metrics.prompt_seconds_total = Some(value),
                "llamacpp:tokens_predicted_seconds_total" => {
                    metrics.tokens_predicted_seconds_total = Some(value)
                }
                "llamacpp:n_decode_total" => metrics.n_decode_total = Some(value),
                "llamacpp:n_busy_slots_per_decode" => metrics.n_busy_slots_per_decode = Some(value),
                "llamacpp:prompt_tokens_seconds" => metrics.prompt_tokens_seconds = Some(value),
                "llamacpp:predicted_tokens_seconds" => metrics.predicted_tokens_seconds = Some(value),
                "llamacpp:kv_cache_usage_ratio" => metrics.kv_cache_usage_ratio = Some(value),
                "llamacpp:kv_cache_tokens" => metrics.kv_cache_tokens = Some(value),
                "llamacpp:requests_processing" => metrics.requests_processing = Some(value),
                "llamacpp:requests_deferred" => metrics.requests_deferred = Some(value),
                _ => {}
            }
        }
        metrics
    }
}

/// 모니터링 관련 오류
#[derive(Debug, thiserror::Error)]
pub enum MonitoringError {
    #[error("LLM 서버 연결 실패: {0}")]
    Connection(String),
    #[error("응답 상태 오류 ({0}): {1}")]
    HttpStatus(u16, String),
    #[error("응답 파싱 실패: {0}")]
    Parse(String),
    #[error("기타 오류: {0}")]
    Other(String),
}

/// LLM 추론 서버 모니터링 API 포트
#[cfg(feature = "chat")]
#[async_trait::async_trait]
pub trait InferenceServerMonitor: Send + Sync {
    async fn health(&self) -> Result<ServerHealth, MonitoringError>;
    async fn slots(&self) -> Result<Vec<InferenceSlotInfo>, MonitoringError>;
    async fn metrics(&self) -> Result<ServerMetrics, MonitoringError>;
}
