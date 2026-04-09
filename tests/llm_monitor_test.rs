//! llama-server 모니터링 엔드포인트 테스트
//!
//! `LlamaServerMonitor` 구현의 /health, /slots, /metrics 조회를 검증한다.
//! axum mock 서버를 사용하여 실제 llama-server 없이 테스트한다.

#![cfg(all(feature = "chat", feature = "mind-studio"))]

use npc_mind::ports::{LlamaHealth, LlamaMetrics, LlamaServerMonitor, LlamaSlotInfo};
use npc_mind::adapter::rig_chat::RigChatAdapter;

// ---------------------------------------------------------------------------
// 테스트 데이터
// ---------------------------------------------------------------------------

/// llama-server /health 응답
fn health_ok_json() -> serde_json::Value {
    serde_json::json!({ "status": "ok" })
}

/// llama-server /health — 로딩 중
fn health_loading_json() -> serde_json::Value {
    serde_json::json!({ "status": "loading model" })
}

/// llama-server /slots 응답 (2개 슬롯)
fn slots_json() -> serde_json::Value {
    serde_json::json!([
        {
            "id": 0,
            "state": 0,
            "n_past": 128,
            "n_predicted": 0,
            "is_processing": false,
            "task_id": -1,
            "model": "test-model",
            "n_ctx": 4096
        },
        {
            "id": 1,
            "state": 1,
            "n_past": 256,
            "n_predicted": 42,
            "is_processing": true,
            "task_id": 7,
            "model": "test-model",
            "n_ctx": 4096
        }
    ])
}

/// llama-server /metrics Prometheus 텍스트
fn metrics_text() -> &'static str {
    r#"# HELP llamacpp:prompt_tokens_total Number of prompt tokens processed.
# TYPE llamacpp:prompt_tokens_total counter
llamacpp:prompt_tokens_total 1234
# HELP llamacpp:tokens_predicted_total Number of generation tokens processed.
# TYPE llamacpp:tokens_predicted_total counter
llamacpp:tokens_predicted_total 5678
# HELP llamacpp:prompt_seconds_total Prompt process time
# TYPE llamacpp:prompt_seconds_total counter
llamacpp:prompt_seconds_total 12.5
# HELP llamacpp:tokens_predicted_seconds_total Predict process time
# TYPE llamacpp:tokens_predicted_seconds_total counter
llamacpp:tokens_predicted_seconds_total 45.3
# HELP llamacpp:n_decode_total Total number of llama_decode() calls
# TYPE llamacpp:n_decode_total counter
llamacpp:n_decode_total 890
# HELP llamacpp:n_busy_slots_per_decode Average number of busy slots per decode
# TYPE llamacpp:n_busy_slots_per_decode counter
llamacpp:n_busy_slots_per_decode 0.75
# HELP llamacpp:prompt_tokens_seconds Average prompt throughput in tokens/s.
# TYPE llamacpp:prompt_tokens_seconds gauge
llamacpp:prompt_tokens_seconds 98.72
# HELP llamacpp:predicted_tokens_seconds Average generation throughput in tokens/s.
# TYPE llamacpp:predicted_tokens_seconds gauge
llamacpp:predicted_tokens_seconds 125.38
# HELP llamacpp:kv_cache_usage_ratio KV-cache usage. 1 means 100 percent usage.
# TYPE llamacpp:kv_cache_usage_ratio gauge
llamacpp:kv_cache_usage_ratio 0.45
# HELP llamacpp:kv_cache_tokens KV-cache tokens.
# TYPE llamacpp:kv_cache_tokens gauge
llamacpp:kv_cache_tokens 2048
# HELP llamacpp:requests_processing Number of requests processing.
# TYPE llamacpp:requests_processing gauge
llamacpp:requests_processing 1
# HELP llamacpp:requests_deferred Number of requests deferred.
# TYPE llamacpp:requests_deferred gauge
llamacpp:requests_deferred 0
"#
}

/// /v1/models 응답 (RigChatAdapter 생성에 필요)
fn models_json() -> serde_json::Value {
    serde_json::json!({
        "object": "list",
        "data": [{ "id": "test-model", "object": "model" }]
    })
}

// ---------------------------------------------------------------------------
// Mock 서버 헬퍼
// ---------------------------------------------------------------------------

/// llama-server 전체를 시뮬레이션하는 mock 서버를 생성한다.
/// /health, /slots, /metrics, /v1/models 엔드포인트를 모두 제공.
async fn start_llama_mock_server() -> (String, tokio::task::JoinHandle<()>) {
    use axum::{Router, routing::get, Json};

    let app = Router::new()
        .route("/health", get(|| async { Json(health_ok_json()) }))
        .route("/slots", get(|| async { Json(slots_json()) }))
        .route(
            "/metrics",
            get(|| async { metrics_text() }),
        )
        .route("/v1/models", get(|| async { Json(models_json()) }));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{addr}"), handle)
}

/// 특정 엔드포인트만 제공하는 부분 mock 서버
async fn start_partial_mock_server() -> (String, tokio::task::JoinHandle<()>) {
    use axum::{Router, routing::get, Json};
    use axum::http::StatusCode;

    // /health만 제공, /slots과 /metrics는 404
    let app = Router::new()
        .route("/health", get(|| async { Json(health_ok_json()) }))
        .route("/v1/models", get(|| async { Json(models_json()) }))
        .fallback(|| async { StatusCode::NOT_FOUND });

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{addr}"), handle)
}

// ---------------------------------------------------------------------------
// LlamaMetrics::parse 단위 테스트
// ---------------------------------------------------------------------------

#[test]
fn metrics_parse_전체_필드() {
    let metrics = LlamaMetrics::parse(metrics_text());

    assert_eq!(metrics.prompt_tokens_total, Some(1234.0));
    assert_eq!(metrics.tokens_predicted_total, Some(5678.0));
    assert_eq!(metrics.prompt_seconds_total, Some(12.5));
    assert_eq!(metrics.tokens_predicted_seconds_total, Some(45.3));
    assert_eq!(metrics.n_decode_total, Some(890.0));
    assert_eq!(metrics.n_busy_slots_per_decode, Some(0.75));
    assert_eq!(metrics.prompt_tokens_seconds, Some(98.72));
    assert_eq!(metrics.predicted_tokens_seconds, Some(125.38));
    assert_eq!(metrics.kv_cache_usage_ratio, Some(0.45));
    assert_eq!(metrics.kv_cache_tokens, Some(2048.0));
    assert_eq!(metrics.requests_processing, Some(1.0));
    assert_eq!(metrics.requests_deferred, Some(0.0));
}

#[test]
fn metrics_parse_빈_텍스트() {
    let metrics = LlamaMetrics::parse("");

    assert!(metrics.prompt_tokens_total.is_none());
    assert!(metrics.tokens_predicted_total.is_none());
    assert!(metrics.kv_cache_usage_ratio.is_none());
    assert!(metrics.raw.is_empty());
}

#[test]
fn metrics_parse_부분_메트릭() {
    let partial = r#"# HELP llamacpp:kv_cache_usage_ratio KV-cache usage.
# TYPE llamacpp:kv_cache_usage_ratio gauge
llamacpp:kv_cache_usage_ratio 0.87
"#;
    let metrics = LlamaMetrics::parse(partial);

    assert_eq!(metrics.kv_cache_usage_ratio, Some(0.87));
    assert!(metrics.prompt_tokens_total.is_none());
    assert!(metrics.tokens_predicted_total.is_none());
}

#[test]
fn metrics_parse_주석_라인_무시() {
    // # 으로 시작하는 주석은 메트릭으로 파싱되지 않아야 함
    let text = "# llamacpp:kv_cache_usage_ratio 0.99\nllamacpp:kv_cache_usage_ratio 0.45\n";
    let metrics = LlamaMetrics::parse(text);
    assert_eq!(metrics.kv_cache_usage_ratio, Some(0.45));
}

#[test]
fn metrics_parse_raw_보존() {
    let text = "llamacpp:kv_cache_usage_ratio 0.5\n";
    let metrics = LlamaMetrics::parse(text);
    assert_eq!(metrics.raw, text);
}

// ---------------------------------------------------------------------------
// LlamaHealth serde 테스트
// ---------------------------------------------------------------------------

#[test]
fn health_역직렬화_ok() {
    let health: LlamaHealth = serde_json::from_value(health_ok_json()).unwrap();
    assert_eq!(health.status, "ok");
}

#[test]
fn health_역직렬화_loading() {
    let health: LlamaHealth = serde_json::from_value(health_loading_json()).unwrap();
    assert_eq!(health.status, "loading model");
}

#[test]
fn health_직렬화_왕복() {
    let original = LlamaHealth { status: "ok".into() };
    let json = serde_json::to_string(&original).unwrap();
    let restored: LlamaHealth = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.status, "ok");
}

// ---------------------------------------------------------------------------
// LlamaSlotInfo serde 테스트
// ---------------------------------------------------------------------------

#[test]
fn slot_역직렬화_idle() {
    let slots: Vec<LlamaSlotInfo> = serde_json::from_value(slots_json()).unwrap();
    assert_eq!(slots.len(), 2);

    let idle = &slots[0];
    assert_eq!(idle.id, 0);
    assert_eq!(idle.state, 0);
    assert_eq!(idle.n_past, 128);
    assert_eq!(idle.n_predicted, 0);
    assert!(!idle.is_processing);
}

#[test]
fn slot_역직렬화_processing() {
    let slots: Vec<LlamaSlotInfo> = serde_json::from_value(slots_json()).unwrap();

    let busy = &slots[1];
    assert_eq!(busy.id, 1);
    assert_eq!(busy.state, 1);
    assert_eq!(busy.n_past, 256);
    assert_eq!(busy.n_predicted, 42);
    assert!(busy.is_processing);
}

#[test]
fn slot_extra_필드_보존() {
    let slots: Vec<LlamaSlotInfo> = serde_json::from_value(slots_json()).unwrap();
    // "model", "n_ctx" 등 정의되지 않은 필드가 extra에 보존되어야 함
    let idle = &slots[0];
    assert_eq!(idle.extra["model"], "test-model");
    assert_eq!(idle.extra["n_ctx"], 4096);
    assert_eq!(idle.extra["task_id"], -1);
}

#[test]
fn slot_최소_필드만_있어도_역직렬화() {
    // id만 필수, 나머지는 default
    let minimal = serde_json::json!([{ "id": 0 }]);
    let slots: Vec<LlamaSlotInfo> = serde_json::from_value(minimal).unwrap();
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].id, 0);
    assert_eq!(slots[0].state, 0);
    assert!(!slots[0].is_processing);
}

// ---------------------------------------------------------------------------
// LlamaServerMonitor — mock 서버 통합 테스트
// ---------------------------------------------------------------------------

#[tokio::test]
async fn monitor_health_ok() {
    let (server_url, _handle) = start_llama_mock_server().await;
    let adapter = RigChatAdapter::new(&format!("{server_url}/v1"), "test-model");

    let health = adapter.health().await.unwrap();
    assert_eq!(health.status, "ok");
}

#[tokio::test]
async fn monitor_slots_조회() {
    let (server_url, _handle) = start_llama_mock_server().await;
    let adapter = RigChatAdapter::new(&format!("{server_url}/v1"), "test-model");

    let slots = adapter.slots().await.unwrap();
    assert_eq!(slots.len(), 2);
    assert_eq!(slots[0].id, 0);
    assert!(!slots[0].is_processing);
    assert_eq!(slots[1].id, 1);
    assert!(slots[1].is_processing);
    assert_eq!(slots[1].n_predicted, 42);
}

#[tokio::test]
async fn monitor_metrics_조회() {
    let (server_url, _handle) = start_llama_mock_server().await;
    let adapter = RigChatAdapter::new(&format!("{server_url}/v1"), "test-model");

    let metrics = adapter.metrics().await.unwrap();
    assert_eq!(metrics.prompt_tokens_total, Some(1234.0));
    assert_eq!(metrics.tokens_predicted_total, Some(5678.0));
    assert_eq!(metrics.kv_cache_usage_ratio, Some(0.45));
    assert_eq!(metrics.predicted_tokens_seconds, Some(125.38));
    assert!(metrics.raw.contains("llamacpp:"));
}

#[tokio::test]
async fn monitor_서버_미지원_엔드포인트_에러() {
    let (server_url, _handle) = start_partial_mock_server().await;
    let adapter = RigChatAdapter::new(&format!("{server_url}/v1"), "test-model");

    // /health는 성공
    let health = adapter.health().await;
    assert!(health.is_ok());

    // /slots은 404 → 파싱 에러
    let slots = adapter.slots().await;
    assert!(slots.is_err());

    // /metrics도 404 → 에러
    let metrics = adapter.metrics().await;
    assert!(metrics.is_err());
}

#[tokio::test]
async fn monitor_서버_미접속_에러() {
    // 존재하지 않는 서버 주소
    let adapter = RigChatAdapter::new("http://127.0.0.1:1/v1", "test-model");

    let health = adapter.health().await;
    assert!(health.is_err());
    assert!(health.unwrap_err().contains("실패"));
}

#[tokio::test]
async fn monitor_base_url_v1_제거_확인() {
    // /v1 경로가 포함된 base_url에서 server_url이 올바르게 도출되는지 검증
    let (server_url, _handle) = start_llama_mock_server().await;

    // /v1 suffix 있는 경우
    let adapter = RigChatAdapter::new(&format!("{server_url}/v1"), "test-model");
    let health = adapter.health().await.unwrap();
    assert_eq!(health.status, "ok");

    // /v1/ trailing slash 있는 경우
    let adapter2 = RigChatAdapter::new(&format!("{server_url}/v1/"), "test-model");
    let health2 = adapter2.health().await.unwrap();
    assert_eq!(health2.status, "ok");
}

// ---------------------------------------------------------------------------
// HTTP 클라이언트 공유 검증 (with_client)
// ---------------------------------------------------------------------------

#[test]
fn with_client_생성자_동작() {
    use npc_mind::adapter::llama_timings::TimingsCapturingClient;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    let shared_client = reqwest::Client::new();
    let store = Arc::new(RwLock::new(None));

    // with_client로 생성해도 정상 동작
    let _capturing = TimingsCapturingClient::with_client(shared_client, store);
}
