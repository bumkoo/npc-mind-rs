use std::convert::Infallible;
use std::sync::Arc;
use axum::{
    extract::State,
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::Stream;
use mcp_sdk::server::{Server, ServerBuilder};
use mcp_sdk::types::{Tool, CallToolRequest};
use serde_json::Value;
use tokio_stream::StreamExt;
use tokio::sync::mpsc;

use crate::state::AppState;

/// MCP 서버 인스턴스를 생성하고 도구를 등록합니다.
pub fn create_mcp_server() -> Arc<Server> {
    let mut builder = ServerBuilder::new("NPC Mind Studio")
        .version("0.1.0")
        .description("HEXACO 기반 NPC 심리 엔진 시뮬레이터 (SSE 모드)");

    // 1. NPC 목록 조회 도구
    builder = builder.tool(
        Tool::new("list_npcs", "등록된 모든 NPC 목록을 조회합니다.")
    );

    // 2. 상황 평가 도구
    builder = builder.tool(
        Tool::new("appraise", "상황을 평가하여 OCC 감정을 생성하고 LLM 연기 프롬프트를 반환합니다.")
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "npc_id": { "type": "string" },
                    "partner_id": { "type": "string" },
                    "situation": { "type": "object" }
                },
                "required": ["npc_id", "partner_id", "situation"]
            }))
    );

    // 3. 대사 PAD 자극 도구
    builder = builder.tool(
        Tool::new("apply_stimulus", "대사의 PAD 수치를 입력하여 NPC의 실시간 감정을 갱신합니다.")
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "npc_id": { "type": "string" },
                    "partner_id": { "type": "string" },
                    "pleasure": { "type": "number" },
                    "arousal": { "type": "number" },
                    "dominance": { "type": "number" }
                },
                "required": ["npc_id", "partner_id", "pleasure", "arousal", "dominance"]
            }))
    );

    Arc::new(builder.build())
}

/// Axum 라우터에 MCP SSE 경로를 추가합니다.
pub fn mcp_router() -> Router<AppState> {
    Router::new()
        .route("/mcp/sse", get(mcp_sse_handler))
        .route("/mcp/message", post(mcp_message_handler))
}

async fn mcp_sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::channel(100);
    
    // MCP 서버에 SSE 전송 통로 등록 (SDK 사양에 따라 구현)
    // let session_id = state.mcp_server.add_sse_session(tx).await;
    
    let stream = tokio_stream::wrappers::ReceiverStream::new(rx)
        .map(|msg| Ok(Event::default().data(msg)));

    Sse::new(stream)
}

async fn mcp_message_handler(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    // 클라이언트의 POST 메시지를 MCP 서버로 전달하고 응답을 받음
    // let response = state.mcp_server.handle_message(payload).await;
    // Json(response)
    Json(serde_json::json!({"status": "received", "detail": "MCP Message endpoint"}))
}
