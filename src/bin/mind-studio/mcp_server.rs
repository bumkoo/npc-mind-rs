use std::sync::Arc;
use std::convert::Infallible;
use axum::{
    extract::State,
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::mpsc;

// rmcp 0.16.0의 실제 타입들
use rmcp::service::Service;

use crate::handlers::{
    perform_appraise,
};
use npc_mind::application::dto::{AppraiseRequest};
use crate::state::AppState;

/// NPC Mind Studio를 위한 구체적인 MCP 서비스 객체
pub struct MindMcpService {
    state: AppState,
}

impl MindMcpService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// 도구 실행 로직을 처리하는 핵심 메서드
    pub async fn call_tool(&self, req_val: Value) -> Result<Value, String> {
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
                let inner = self.state.inner.read().await;
                let npcs: Vec<_> = inner.npcs.values().cloned().collect();
                Ok(serde_json::to_value(npcs).map_err(|e| e.to_string())?)
            }
            "appraise" => {
                let args: AppraiseRequest =
                    serde_json::from_value(arguments.clone()).map_err(|e| e.to_string())?;
                let resp = perform_appraise(&self.state, args)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
            }
            "get_npc_llm_config" => {
                let npc_id = arguments["npc_id"].as_str().ok_or("npc_id is required")?;
                let inner = self.state.inner.read().await;
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
}

// rmcp의 Service 트레이트 구현 (실제 라이브러리 사양에 맞춰 확장 가능)
// 여기서는 구체적인 타입 MindMcpService를 AppState에서 직접 관리하도록 함

/// MCP 서버 인스턴스를 생성합니다. (Any 제거, 구체적 타입 반환)
pub fn create_mcp_server(state: AppState) -> Arc<MindMcpService> {
    Arc::new(MindMcpService::new(state))
}

/// Axum 라우터에 MCP SSE 경로를 추가합니다.
pub fn mcp_router() -> Router<AppState> {
    Router::new()
        .route("/mcp/sse", get(mcp_sse_handler))
        .route("/mcp/message", post(mcp_message_handler))
}

async fn mcp_sse_handler(
    State(_state): State<AppState>,
) -> Sse<impl futures_util::stream::Stream<Item = Result<Event, Infallible>>> {
    let (_tx, rx) = mpsc::channel::<String>(100);
    
    let stream = tokio_stream::wrappers::ReceiverStream::new(rx)
        .map(|msg| Ok(Event::default().data(msg)));

    Sse::new(stream)
}

async fn mcp_message_handler(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    if let Some(mcp) = &state.mcp_server {
        match mcp.call_tool(payload).await {
            Ok(res) => Json(serde_json::json!({
                "jsonrpc": "2.0",
                "result": res,
                "id": 1 // 실제로는 요청 ID를 따라야 함
            })),
            Err(e) => Json(serde_json::json!({
                "jsonrpc": "2.0",
                "error": { "code": -32603, "message": e },
                "id": 1
            })),
        }
    } else {
        Json(serde_json::json!({"error": "MCP server not initialized"}))
    }
}
