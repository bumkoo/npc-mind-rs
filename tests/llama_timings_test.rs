//! llama-server timings 캡처 테스트
//!
//! `TimingsCapturingClient`가 HTTP 응답에서 timings를 올바르게 추출하고,
//! SSE 스트림에서도 timings를 캡처하는지 검증한다.
//!
//! axum을 mock 서버로 사용하여 실제 llama-server 없이 테스트한다.

#![cfg(feature = "chat")]

use npc_mind::ports::{ChatResponse, LlamaTimings};
use std::sync::Arc;
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// 테스트 데이터
// ---------------------------------------------------------------------------

/// llama-server가 반환하는 전형적인 timings JSON
fn sample_timings_json() -> serde_json::Value {
    serde_json::json!({
        "prompt_n": 13,
        "prompt_ms": 338.304,
        "prompt_per_token_ms": 26.023,
        "prompt_per_second": 38.427,
        "predicted_n": 33,
        "predicted_ms": 1101.544,
        "predicted_per_token_ms": 33.380,
        "predicted_per_second": 29.958
    })
}

/// llama-server /v1/chat/completions 전체 응답 (non-streaming)
fn llama_completion_response() -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-1",
        "object": "chat.completion",
        "created": 1700000000_u64,
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "안녕하시오, 협객."
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 13,
            "completion_tokens": 33,
            "total_tokens": 46
        },
        "timings": sample_timings_json()
    })
}

/// OpenAI 표준 응답 (timings 없음)
fn openai_completion_response() -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-2",
        "object": "chat.completion",
        "created": 1700000000_u64,
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "Hello!"
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 5,
            "completion_tokens": 1,
            "total_tokens": 6
        }
    })
}

// ---------------------------------------------------------------------------
// LlamaTimings serde 테스트
// ---------------------------------------------------------------------------

#[test]
fn llama_timings_역직렬화() {
    let json = sample_timings_json();
    let timings: LlamaTimings = serde_json::from_value(json).unwrap();

    assert_eq!(timings.prompt_n, 13);
    assert!((timings.prompt_ms - 338.304).abs() < 0.001);
    assert_eq!(timings.predicted_n, 33);
    assert!((timings.predicted_per_second - 29.958).abs() < 0.001);
}

#[test]
fn llama_timings_직렬화_왕복() {
    let original = LlamaTimings {
        prompt_n: 10,
        prompt_ms: 100.0,
        prompt_per_token_ms: 10.0,
        prompt_per_second: 100.0,
        predicted_n: 50,
        predicted_ms: 500.0,
        predicted_per_token_ms: 10.0,
        predicted_per_second: 100.0,
    };

    let json = serde_json::to_string(&original).unwrap();
    let restored: LlamaTimings = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.prompt_n, original.prompt_n);
    assert_eq!(restored.predicted_n, original.predicted_n);
    assert!((restored.predicted_ms - original.predicted_ms).abs() < 0.001);
}

#[test]
fn chat_response_timings_있을_때() {
    let resp = ChatResponse {
        text: "응답".into(),
        timings: Some(LlamaTimings {
            prompt_n: 5,
            prompt_ms: 50.0,
            prompt_per_token_ms: 10.0,
            prompt_per_second: 100.0,
            predicted_n: 10,
            predicted_ms: 100.0,
            predicted_per_token_ms: 10.0,
            predicted_per_second: 100.0,
        }),
    };

    let json = serde_json::to_value(&resp).unwrap();
    assert!(json.get("timings").is_some());
    assert_eq!(json["text"], "응답");
}

#[test]
fn chat_response_timings_없을_때_skip() {
    let resp = ChatResponse {
        text: "응답".into(),
        timings: None,
    };

    let json = serde_json::to_value(&resp).unwrap();
    // skip_serializing_if = "Option::is_none" 이므로 timings 키 자체가 없어야 함
    assert!(json.get("timings").is_none());
}

#[test]
fn chat_response_timings_없는_json_역직렬화() {
    let json = serde_json::json!({ "text": "hello" });
    let resp: ChatResponse = serde_json::from_value(json).unwrap();
    assert_eq!(resp.text, "hello");
    assert!(resp.timings.is_none());
}

// ---------------------------------------------------------------------------
// TimingsCapturingClient — mock 서버 통합 테스트
// ---------------------------------------------------------------------------

use npc_mind::adapter::llama_timings::TimingsCapturingClient;
use rig::http_client::HttpClientExt;

/// mock 서버를 띄우고 URL을 반환하는 헬퍼
async fn start_mock_server(
    response_body: serde_json::Value,
) -> (String, tokio::task::JoinHandle<()>) {
    use axum::{Router, routing::post, Json};

    let body = response_body.clone();
    let app = Router::new().route(
        "/v1/chat/completions",
        post(move || {
            let b = body.clone();
            async move { Json(b) }
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{addr}"), handle)
}

#[tokio::test]
async fn llama_서버_응답에서_timings_캡처() {
    let (base_url, _server) = start_mock_server(llama_completion_response()).await;

    let store = Arc::new(RwLock::new(None));
    let client = TimingsCapturingClient::new(store.clone());

    // POST 요청 구성
    let body = serde_json::to_vec(&serde_json::json!({
        "model": "test",
        "messages": [{"role": "user", "content": "hi"}]
    }))
    .unwrap();

    let req = rig::http_client::Request::builder()
        .method("POST")
        .uri(format!("{base_url}/v1/chat/completions"))
        .header("content-type", "application/json")
        .body(body)
        .unwrap();

    let response = client.send::<_, Vec<u8>>(req).await.unwrap();

    // 1. 응답 body가 rig에게 정상 전달되는지 확인
    let body_bytes = response.into_body().await.unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(
        body_json["choices"][0]["message"]["content"],
        "안녕하시오, 협객."
    );

    // 2. timings가 캡처되었는지 확인
    let timings = store.read().await;
    assert!(timings.is_some(), "timings가 캡처되어야 합니다");
    let t = timings.as_ref().unwrap();
    assert_eq!(t.prompt_n, 13);
    assert_eq!(t.predicted_n, 33);
    assert!((t.predicted_per_second - 29.958).abs() < 0.001);
}

#[tokio::test]
async fn openai_응답에서_timings_none() {
    let (base_url, _server) = start_mock_server(openai_completion_response()).await;

    let store = Arc::new(RwLock::new(None));
    let client = TimingsCapturingClient::new(store.clone());

    let body = serde_json::to_vec(&serde_json::json!({
        "model": "gpt-4o",
        "messages": [{"role": "user", "content": "hi"}]
    }))
    .unwrap();

    let req = rig::http_client::Request::builder()
        .method("POST")
        .uri(format!("{base_url}/v1/chat/completions"))
        .header("content-type", "application/json")
        .body(body)
        .unwrap();

    let _response = client.send::<_, Vec<u8>>(req).await.unwrap();

    // OpenAI 표준 응답에는 timings 없음
    let timings = store.read().await;
    assert!(timings.is_none(), "OpenAI 응답에는 timings가 없어야 합니다");
}

#[tokio::test]
async fn 연속_요청시_timings_갱신() {
    // 첫 번째: timings 있음
    let (url1, _s1) = start_mock_server(llama_completion_response()).await;
    // 두 번째: timings 없음
    let (url2, _s2) = start_mock_server(openai_completion_response()).await;

    let store = Arc::new(RwLock::new(None));
    let client = TimingsCapturingClient::new(store.clone());

    // 1차 요청: llama → timings 있음
    let body = serde_json::to_vec(&serde_json::json!({"model":"a","messages":[]})).unwrap();
    let req = rig::http_client::Request::builder()
        .method("POST")
        .uri(format!("{url1}/v1/chat/completions"))
        .header("content-type", "application/json")
        .body(body)
        .unwrap();
    let _resp = client.send::<_, Vec<u8>>(req).await.unwrap();
    assert!(store.read().await.is_some());

    // 2차 요청: openai → timings 없음으로 갱신
    let body = serde_json::to_vec(&serde_json::json!({"model":"b","messages":[]})).unwrap();
    let req = rig::http_client::Request::builder()
        .method("POST")
        .uri(format!("{url2}/v1/chat/completions"))
        .header("content-type", "application/json")
        .body(body)
        .unwrap();
    let _resp = client.send::<_, Vec<u8>>(req).await.unwrap();
    assert!(
        store.read().await.is_none(),
        "OpenAI 응답 후 timings가 None으로 갱신되어야 합니다"
    );
}

// ---------------------------------------------------------------------------
// SSE 스트리밍 timings 캡처 테스트
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sse_스트림에서_timings_캡처() {
    use axum::{Router, routing::post, response::sse};
    use futures::stream;

    // SSE mock 서버: llama-server 스트리밍 응답을 시뮬레이션
    let app = Router::new().route(
        "/v1/chat/completions",
        post(|| async {
            let chunks = vec![
                // 일반 토큰 청크
                sse::Event::default()
                    .data(r#"{"choices":[{"delta":{"content":"안녕"},"finish_reason":null}]}"#),
                // 마지막 청크: finish_reason + timings
                sse::Event::default().data(format!(
                    r#"{{"choices":[{{"delta":{{}},"finish_reason":"stop"}}],"timings":{}}}"#,
                    serde_json::to_string(&sample_timings_json()).unwrap()
                )),
                // [DONE]
                sse::Event::default().data("[DONE]"),
            ];
            sse::Sse::new(stream::iter(chunks.into_iter().map(Ok::<_, std::convert::Infallible>)))
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let store = Arc::new(RwLock::new(None));
    let client = TimingsCapturingClient::new(store.clone());

    let body = serde_json::to_vec(&serde_json::json!({
        "model": "test",
        "messages": [{"role": "user", "content": "hi"}],
        "stream": true
    }))
    .unwrap();

    let req = rig::http_client::Request::builder()
        .method("POST")
        .uri(format!("http://{addr}/v1/chat/completions"))
        .header("content-type", "application/json")
        .body(body)
        .unwrap();

    let response = client.send_streaming(req).await.unwrap();

    // 스트림을 끝까지 소비
    use futures::StreamExt;
    let mut stream = response.into_body();
    while let Some(chunk) = stream.next().await {
        let _ = chunk; // 소비만 함
    }

    // tokio::spawn으로 timings를 저장하므로 잠시 대기
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let timings = store.read().await;
    assert!(timings.is_some(), "SSE 스트림에서 timings가 캡처되어야 합니다");
    let t = timings.as_ref().unwrap();
    assert_eq!(t.prompt_n, 13);
    assert_eq!(t.predicted_n, 33);
}

#[tokio::test]
async fn sse_스트림_timings_없으면_none_유지() {
    use axum::{Router, routing::post, response::sse};
    use futures::stream;

    // 표준 OpenAI 스트리밍 응답 (timings 없음)
    let app = Router::new().route(
        "/v1/chat/completions",
        post(|| async {
            let chunks = vec![
                sse::Event::default()
                    .data(r#"{"choices":[{"delta":{"content":"Hi"},"finish_reason":null}]}"#),
                sse::Event::default()
                    .data(r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#),
                sse::Event::default().data("[DONE]"),
            ];
            sse::Sse::new(stream::iter(chunks.into_iter().map(Ok::<_, std::convert::Infallible>)))
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let store = Arc::new(RwLock::new(None));
    let client = TimingsCapturingClient::new(store.clone());

    let body = serde_json::to_vec(&serde_json::json!({
        "model": "test",
        "messages": [],
        "stream": true
    }))
    .unwrap();

    let req = rig::http_client::Request::builder()
        .method("POST")
        .uri(format!("http://{addr}/v1/chat/completions"))
        .header("content-type", "application/json")
        .body(body)
        .unwrap();

    let response = client.send_streaming(req).await.unwrap();

    use futures::StreamExt;
    let mut stream = response.into_body();
    while let Some(_) = stream.next().await {}

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    assert!(
        store.read().await.is_none(),
        "timings가 없는 스트림에서는 None이어야 합니다"
    );
}
