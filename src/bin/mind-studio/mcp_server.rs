use std::sync::Arc;
use std::convert::Infallible;
use axum::{
    extract::{State, Query},
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::{Stream, StreamExt};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use std::collections::HashMap;

use crate::studio_service::StudioService;
use crate::handlers::AppError;
use npc_mind::application::dto::{AppraiseRequest};
use crate::state::AppState;

/// SSE 세션 관리자
pub struct McpSessionManager {
    /// 세션 ID -> SSE 전송 채널
    sessions: RwLock<HashMap<String, mpsc::Sender<String>>>,
}

impl McpSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn create_session(&self) -> (String, mpsc::Receiver<String>) {
        let (tx, rx) = mpsc::channel(100);
        let session_id = Uuid::new_v4().to_string();
        self.sessions.write().await.insert(session_id.clone(), tx);
        (session_id, rx)
    }

    pub async fn remove_session(&self, id: &str) {
        self.sessions.write().await.remove(id);
    }

    pub async fn send_to_session(&self, id: &str, msg: String) -> Result<(), String> {
        let sessions = self.sessions.read().await;
        if let Some(tx) = sessions.get(id) {
            tx.send(msg).await.map_err(|e| e.to_string())
        } else {
            Err("Session not found".into())
        }
    }
}

/// MCP 서비스 객체
pub struct MindMcpService {
    state: AppState,
    pub session_manager: McpSessionManager,
}

impl MindMcpService {
    pub fn new(state: AppState) -> Self {
        Self { 
            state,
            session_manager: McpSessionManager::new(),
        }
    }

    /// 도구 목록 조회
    pub fn list_tools(&self) -> Vec<Value> {
        vec![
            serde_json::json!({
                "name": "list_npcs",
                "description": "등록된 모든 NPC 목록을 조회합니다.",
                "input_schema": { "type": "object", "properties": {} }
            }),
            serde_json::json!({
                "name": "appraise",
                "description": "상황을 평가하여 OCC 감정을 생성하고 LLM 연기 프롬프트를 반환합니다.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "npc_id": { "type": "string" },
                        "partner_id": { "type": "string" },
                        "situation": { "type": "object" }
                    },
                    "required": ["npc_id", "partner_id", "situation"]
                }
            }),
            serde_json::json!({
                "name": "get_npc_llm_config",
                "description": "NPC의 성격에 최적화된 LLM 생성 파라미터를 조회합니다.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "npc_id": { "type": "string" }
                    },
                    "required": ["npc_id"]
                }
            })
        ]
    }

    /// 도구 실행 로직
    pub async fn call_tool(&self, name: &str, arguments: &Value) -> Result<Value, String> {
        match name {
            "list_npcs" => {
                let inner = self.state.inner.read().await;
                let npcs: Vec<_> = inner.npcs.values().cloned().collect();
                Ok(serde_json::to_value(npcs).map_err(|e| e.to_string())?)
            }
            "appraise" => {
                let args: AppraiseRequest =
                    serde_json::from_value(arguments.clone()).map_err(|e| e.to_string())?;
                let resp = StudioService::perform_appraise(&self.state, args)
                    .await
                    .map_err(|e: AppError| e.to_string())?;
                Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
            }
            "get_npc_llm_config" => {
                let npc_id = arguments["npc_id"].as_str().ok_or("npc_id is required")?;
                let inner = self.state.inner.read().await;
                let npc_profile = inner.npcs.get(npc_id).ok_or_else(|| format!("NPC {} not found", npc_id))?;
                let (temp, top_p) = npc_profile.derive_llm_parameters();
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

pub fn create_mcp_server(state: AppState) -> Arc<MindMcpService> {
    Arc::new(MindMcpService::new(state))
}

pub fn mcp_router() -> Router<AppState> {
    Router::new()
        .route("/mcp/sse", get(mcp_sse_handler))
        .route("/mcp/message", post(mcp_message_handler))
}

async fn mcp_sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mcp = state.mcp_server.as_ref().expect("MCP server not initialized");
    let (session_id, rx) = mcp.session_manager.create_session().await;

    let initial_event = Event::default()
        .event("endpoint")
        .data(format!("/mcp/message?session_id={}", session_id));

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx)
        .map(|msg| Ok(Event::default().data(msg)));

    let combined_stream = futures_util::stream::once(async move { Ok(initial_event) })
        .chain(stream);

    Sse::new(combined_stream)
}

async fn mcp_message_handler(
    State(state): State<AppState>,
    Query(query): Query<SessionQuery>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    let mcp = state.mcp_server.as_ref().unwrap();
    let id = payload["id"].clone();
    let method = payload["method"].as_str().unwrap_or("");
    
    let result = match method {
        "tools/list" => {
            Ok(serde_json::json!({ "tools": mcp.list_tools() }))
        },
        "tools/call" => {
            let name = payload["params"]["name"].as_str().unwrap_or("");
            let args = &payload["params"]["arguments"];
            mcp.call_tool(name, args).await
        },
        _ => Err(format!("Unsupported method: {}", method)),
    };

    match result {
        Ok(res_val) => {
            let json_res = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": res_val
            });
            let _ = mcp.session_manager.send_to_session(&query.session_id, json_res.to_string()).await;
            Json(serde_json::json!({"status": "sent"}))
        },
        Err(e) => {
            let json_err = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32603, "message": e }
            });
            let _ = mcp.session_manager.send_to_session(&query.session_id, json_err.to_string()).await;
            Json(serde_json::json!({"status": "error_sent"}))
        }
    }
}

#[derive(Deserialize)]
pub struct SessionQuery {
    pub session_id: String,
}
