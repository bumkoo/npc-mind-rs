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

use crate::handlers::{
    perform_after_dialogue, perform_appraise, perform_stimulus, AfterDialogueRequest,
    AppraiseRequest, StimulusRequest,
};
use crate::state::AppState;

/// MCP 서버 인스턴스를 생성하고 도구를 등록합니다.
pub fn create_mcp_server() -> Arc<Server> {
    let mut builder = ServerBuilder::new("NPC Mind Studio")
        .version("0.1.0")
        .description("HEXACO 기반 NPC 심리 엔진 시뮬레이터 (SSE 모드)");

    // 1. NPC 목록 조회 도구
    builder = builder.tool(Tool::new("list_npcs", "등록된 모든 NPC 목록을 조회합니다."));

    // 2. 상황 평가 도구
    builder = builder.tool(
        Tool::new(
            "appraise",
            "상황을 평가하여 OCC 감정을 생성하고 LLM 연기 프롬프트를 반환합니다.",
        )
        .input_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "npc_id": { "type": "string" },
                "partner_id": { "type": "string" },
                "situation": { "type": "object" }
            },
            "required": ["npc_id", "partner_id", "situation"]
        })),
    );

    // 3. 대사 PAD 자극 도구
    builder = builder.tool(
        Tool::new(
            "apply_stimulus",
            "대사의 PAD 수치를 입력하여 NPC의 실시간 감정을 갱신합니다.",
        )
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
        })),
    );

    // 4. NPC별 권장 LLM 설정 조회
    builder = builder.tool(
        Tool::new("get_npc_llm_config", "NPC의 성격에 최적화된 LLM 생성 파라미터(Temperature, Top P)를 조회합니다.")
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "npc_id": { "type": "string" }
                },
                "required": ["npc_id"]
            }))
    );

    // 5. 마지막 심리 평가 추적 조회
    builder = builder.tool(
        Tool::new("get_last_appraisal_trace", "가장 최근에 실행된 감정 평가의 상세 추론 로그(Trace)를 조회합니다.")
    );

    Arc::new(builder.build())
}

/// MCP 도구 요청을 처리합니다. (AppState와 연동)
pub async fn handle_mcp_tool_call(
    state: &AppState,
    req: CallToolRequest,
) -> Result<Value, String> {
    match req.name.as_str() {
        "list_npcs" => {
            let inner = state.inner.read().await;
            let npcs: Vec<_> = inner.npcs.values().cloned().collect();
            Ok(serde_json::to_value(npcs).map_err(|e| e.to_string())?)
        }
        "appraise" => {
            let args: AppraiseRequest =
                serde_json::from_value(req.arguments).map_err(|e| e.to_string())?;
            let resp = perform_appraise(state, args)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
        }
        "apply_stimulus" => {
            let args: StimulusRequest =
                serde_json::from_value(req.arguments).map_err(|e| e.to_string())?;
            let resp = perform_stimulus(state, args)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
        }
        "get_npc_llm_config" => {
            let npc_id = req.arguments["npc_id"].as_str().ok_or("npc_id is required")?;
            let inner = state.inner.read().await;
            let npc = inner.npcs.get(npc_id).ok_or_else(|| format!("NPC {} not found", npc_id))?;
            let (temp, top_p) = npc.derive_llm_parameters();
            Ok(serde_json::json!({
                "npc_id": npc_id,
                "temperature": temp,
                "top_p": top_p
            }))
        }
        "get_last_appraisal_trace" => {
            let collector = state.collector.clone();
            let trace = collector.take_entries(); // 수집된 트레이스 가져오기
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
