use std::sync::Arc;
use std::convert::Infallible;
use axum::{
    extract::State,
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::Stream;
use rmcp::model::{CallToolRequest, Tool};
use serde_json::Value;
use tokio_stream::StreamExt;
use tokio::sync::mpsc;

use crate::handlers::{
    perform_appraise,
};
use npc_mind::application::dto::{AppraiseRequest};
use crate::state::AppState;

/// RMCP 서버 인스턴스를 생성하고 도구를 등록합니다.
/// (rmcp 0.16은 McpService 또는 유사한 구조를 사용할 수 있음)
pub fn create_mcp_server() -> Arc<dyn std::any::Any + Send + Sync> {
    // 0.16 SDK의 구체적인 서버 타입을 맞추기 위해 일단 Any로 우회
    Arc::new(())
}

/// MCP 도구 요청을 처리합니다. (AppState와 연동)
/// 특정 라이브러리 타입에 의존하지 않도록 JSON Value로 처리
pub async fn handle_mcp_tool_call(
    state: &AppState,
    req_val: Value,
) -> Result<Value, String> {
    // JSON-RPC 구조에서 params.name과 params.arguments 추출 시도
    let name = req_val["params"]["name"].as_str()
        .or_else(|| req_val["name"].as_str())
        .ok_or("tool name is required")?;
    
    let arguments = &req_val["params"]["arguments"];
    let arguments = if arguments.is_null() {
        &req_val["arguments"]
    } else {
        arguments
    };

    match name {
        "list_npcs" => {
            let inner = state.inner.read().await;
            let npcs: Vec<_> = inner.npcs.values().cloned().collect();
            Ok(serde_json::to_value(npcs).map_err(|e| e.to_string())?)
        }
        "appraise" => {
            let args: AppraiseRequest =
                serde_json::from_value(arguments.clone()).map_err(|e| e.to_string())?;
            let resp = perform_appraise(state, args)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
        }
        "get_npc_llm_config" => {
            let npc_id = arguments["npc_id"].as_str().ok_or("npc_id is required")?;
            let inner = state.inner.read().await;
            let npc_profile = inner.npcs.get(npc_id).ok_or_else(|| format!("NPC {} not found", npc_id))?;
            let (temp, top_p) = npc_profile.to_npc().derive_llm_parameters();
            Ok(serde_json::json!({
                "npc_id": npc_id,
                "temperature": temp,
                "top_p": top_p
            }))
        }
        _ => Err(format!("Unknown tool: {}", name)),
    }
}

/// Axum 라우터에 MCP SSE 경로를 추가합니다.
pub fn mcp_router() -> Router<AppState> {
    Router::new()
        .route("/mcp/sse", get(mcp_sse_handler))
        .route("/mcp/message", post(mcp_message_handler))
}

async fn mcp_sse_handler(
    State(_state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (_tx, rx) = mpsc::channel::<String>(100);
    
    let stream = tokio_stream::wrappers::ReceiverStream::new(rx)
        .map(|msg| Ok(Event::default().data(msg)));

    Sse::new(stream)
}

async fn mcp_message_handler(
    State(_state): State<AppState>,
    Json(_payload): Json<Value>,
) -> Json<Value> {
    Json(serde_json::json!({"status": "received", "detail": "MCP Message endpoint via RMCP"}))
}
