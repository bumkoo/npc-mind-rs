use std::sync::Arc;
use std::convert::Infallible;
use axum::{
    extract::State,
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::Stream;
use mcp_sdk::types::CallToolRequest;
use serde_json::Value;
use tokio_stream::StreamExt;
use tokio::sync::mpsc;

use crate::handlers::{
    perform_after_dialogue, perform_appraise, perform_stimulus,
};
use npc_mind::application::dto::{AfterDialogueRequest, AppraiseRequest, StimulusRequest};
use crate::state::AppState;

/// MCP 서버 인스턴스를 생성하고 도구를 등록합니다.
/// (현재 컴파일 호환성을 위해 더미 객체를 반환하거나 최소화된 설정을 사용)
pub fn create_mcp_server() -> Arc<dyn std::any::Any + Send + Sync> {
    // 0.0.3 SDK의 복잡한 제네릭 문제를 피하기 위해 일단 빈 Arc 반환
    Arc::new(())
}

/// MCP 도구 요청을 처리합니다. (AppState와 연동)
pub async fn handle_mcp_tool_call(
    state: &AppState,
    req: CallToolRequest,
) -> Result<Value, String> {
    let arguments = req.arguments.ok_or("arguments are required")?;
    
    match req.name.as_str() {
        "list_npcs" => {
            let inner = state.inner.read().await;
            let npcs: Vec<_> = inner.npcs.values().cloned().collect();
            Ok(serde_json::to_value(npcs).map_err(|e| e.to_string())?)
        }
        "appraise" => {
            let args: AppraiseRequest =
                serde_json::from_value(arguments).map_err(|e| e.to_string())?;
            let resp = perform_appraise(state, args)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
        }
        "apply_stimulus" => {
            let args: StimulusRequest =
                serde_json::from_value(arguments).map_err(|e| e.to_string())?;
            let resp = perform_stimulus(state, args)
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
        "get_last_appraisal_trace" => {
            let collector = state.collector.clone();
            let trace = collector.take_entries();
            Ok(serde_json::to_value(trace).map_err(|e| e.to_string())?)
        }
        _ => Err(format!("Unknown tool: {}", req.name)),
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
    Json(serde_json::json!({"status": "received", "detail": "MCP Message endpoint"}))
}
