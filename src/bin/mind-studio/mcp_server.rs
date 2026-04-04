use std::sync::Arc;
use std::convert::Infallible;
use axum::{
    extract::{State, Query},
    response::sse::{Event, Sse},
    routing::{get, post},
    Router, Json,
};
use futures_util::stream::{Stream, StreamExt};
use serde_json::Value;
use tokio::sync::{mpsc, RwLock};
use std::collections::HashMap;
use uuid::Uuid;

use crate::state::AppState;
use crate::studio_service::StudioService;
use crate::handlers::AppError;
use npc_mind::application::dto::*;

/// MCP 세션 관리자 (SSE 연결 유지용)
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
        let session_id = Uuid::new_v4().to_string();
        let (tx, rx) = mpsc::channel(100);
        self.sessions.write().await.insert(session_id.clone(), tx);
        (session_id, rx)
    }

    pub async fn send_to_session(&self, id: &str, msg: String) -> Result<(), String> {
        let sessions = self.sessions.read().await;
        if let Some(tx) = sessions.get(id) {
            tx.send(msg).await.map_err(|e| e.to_string())
        } else {
            Err("Session not found".into())
        }
    }

    #[allow(dead_code)]
    pub async fn remove_session(&self, id: &str) {
        self.sessions.write().await.remove(id);
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

    /// 도구 목록 조회 (25개)
    pub fn list_tools(&self) -> Vec<Value> {
        vec![
            // 세계 구축 (CRUD)
            serde_json::json!({ "name": "list_npcs", "description": "등록된 모든 NPC 목록을 조회합니다.", "inputSchema": { "type": "object", "properties": {} } }),
            serde_json::json!({ "name": "create_npc", "description": "NPC를 생성하거나 수정합니다 (HEXACO 24 facets).", "inputSchema": { "type": "object", "properties": { "npc": { "type": "object" } }, "required": ["npc"] } }),
            serde_json::json!({ "name": "delete_npc", "description": "NPC를 삭제합니다.", "inputSchema": { "type": "object", "properties": { "id": { "type": "string" } }, "required": ["id"] } }),
            serde_json::json!({ "name": "list_relationships", "description": "모든 관계 목록을 조회합니다.", "inputSchema": { "type": "object", "properties": {} } }),
            serde_json::json!({ "name": "create_relationship", "description": "관계 정보를 생성하거나 수정합니다.", "inputSchema": { "type": "object", "properties": { "rel": { "type": "object" } }, "required": ["rel"] } }),
            serde_json::json!({ "name": "delete_relationship", "description": "관계를 삭제합니다.", "inputSchema": { "type": "object", "properties": { "owner_id": { "type": "string" }, "target_id": { "type": "string" } }, "required": ["owner_id", "target_id"] } }),
            serde_json::json!({ "name": "list_objects", "description": "모든 오브젝트 목록을 조회합니다.", "inputSchema": { "type": "object", "properties": {} } }),
            serde_json::json!({ "name": "create_object", "description": "오브젝트 정보를 생성하거나 수정합니다.", "inputSchema": { "type": "object", "properties": { "obj": { "type": "object" } }, "required": ["obj"] } }),
            serde_json::json!({ "name": "delete_object", "description": "오브젝트를 삭제합니다.", "inputSchema": { "type": "object", "properties": { "id": { "type": "string" } }, "required": ["id"] } }),

            // 감정 파이프라인
            serde_json::json!({
                "name": "appraise",
                "description": "상황을 평가하여 OCC 감정을 생성하고 LLM 연기 프롬프트를 반환합니다.",
                "inputSchema": {
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
                "name": "apply_stimulus",
                "description": "대화 중 발생하는 PAD 자극을 적용하여 감정을 갱신하고 Beat 전환을 체크합니다.",
                "inputSchema": { "type": "object", "properties": { "req": { "type": "object" } }, "required": ["req"] }
            }),
            serde_json::json!({
                "name": "analyze_utterance",
                "description": "대사 텍스트를 분석하여 PAD 수치를 추출합니다 (embed feature 필요).",
                "inputSchema": { "type": "object", "properties": { "utterance": { "type": "string" } }, "required": ["utterance"] }
            }),
            serde_json::json!({
                "name": "generate_guide",
                "description": "현재 감정 상태 기반으로 연기 가이드를 재생성합니다.",
                "inputSchema": { "type": "object", "properties": { "req": { "type": "object" } }, "required": ["req"] }
            }),
            serde_json::json!({
                "name": "after_dialogue",
                "description": "대화를 종료하고 감정 상태를 관계 변화에 반영합니다.",
                "inputSchema": { "type": "object", "properties": { "req": { "type": "object" } }, "required": ["req"] }
            }),

            // 상태 관리
            serde_json::json!({ "name": "get_history", "description": "현재 세션의 턴별 히스토리를 조회합니다.", "inputSchema": { "type": "object", "properties": {} } }),
            serde_json::json!({ "name": "get_situation", "description": "현재 상황 설정 패널의 상태를 조회합니다.", "inputSchema": { "type": "object", "properties": {} } }),
            serde_json::json!({
                "name": "update_situation",
                "description": "상황 설정 패널 상태를 업데이트합니다 (WebUI 동기화용).",
                "inputSchema": { "type": "object", "properties": { "body": { "type": "object" } }, "required": ["body"] }
            }),
            serde_json::json!({
                "name": "get_test_report",
                "description": "현재 테스트 결과 분석 보고서(마크다운)를 조회합니다.",
                "inputSchema": { "type": "object", "properties": {} }
            }),
            serde_json::json!({
                "name": "update_test_report",
                "description": "테스트 결과 분석 보고서(마크다운)를 작성하거나 업데이트합니다.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "content": { "type": "string", "description": "마크다운 형식의 보고서 내용" } },
                    "required": ["content"]
                }
            }),

            // 시나리오 관리
            serde_json::json!({ "name": "list_scenarios", "description": "사용 가능한 시나리오 파일 목록을 조회합니다.", "inputSchema": { "type": "object", "properties": {} } }),
            serde_json::json!({ "name": "get_scenario_meta", "description": "현재 로드된 시나리오의 메타데이터를 조회합니다.", "inputSchema": { "type": "object", "properties": {} } }),
            serde_json::json!({ "name": "save_scenario", "description": "현재 상태를 지정된 경로에 JSON 파일로 저장합니다.", "inputSchema": { "type": "object", "properties": { "path": { "type": "string" }, "save_type": { "type": "string" } }, "required": ["path"] } }),
            serde_json::json!({ "name": "load_scenario", "description": "지정된 경로의 시나리오 파일을 로드합니다.", "inputSchema": { "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"] } }),
            
            // 기타
            serde_json::json!({
                "name": "get_npc_llm_config",
                "description": "NPC의 성격에 최적화된 LLM 생성 파라미터를 조회합니다.",
                "inputSchema": { "type": "object", "properties": { "npc_id": { "type": "string" } }, "required": ["npc_id"] }
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
            "create_npc" => {
                let npc: crate::state::NpcProfile = serde_json::from_value(arguments["npc"].clone()).map_err(|e| e.to_string())?;
                let mut inner = self.state.inner.write().await;
                inner.npcs.insert(npc.id.clone(), npc);
                inner.scenario_modified = true;
                Ok(serde_json::json!({ "status": "ok" }))
            }
            "delete_npc" => {
                let id = arguments["id"].as_str().ok_or("id is required")?;
                let mut inner = self.state.inner.write().await;
                inner.npcs.remove(id);
                inner.scenario_modified = true;
                Ok(serde_json::json!({ "status": "ok" }))
            }
            "list_relationships" => {
                let inner = self.state.inner.read().await;
                let rels: Vec<_> = inner.relationships.values().cloned().collect();
                Ok(serde_json::to_value(rels).map_err(|e| e.to_string())?)
            }
            "create_relationship" => {
                let rel: crate::state::RelationshipData = serde_json::from_value(arguments["rel"].clone()).map_err(|e| e.to_string())?;
                let mut inner = self.state.inner.write().await;
                inner.relationships.insert(rel.key(), rel);
                inner.scenario_modified = true;
                Ok(serde_json::json!({ "status": "ok" }))
            }
            "delete_relationship" => {
                let owner = arguments["owner_id"].as_str().ok_or("owner_id is required")?;
                let target = arguments["target_id"].as_str().ok_or("target_id is required")?;
                let mut inner = self.state.inner.write().await;
                inner.relationships.remove(&format!("{}:{}", owner, target));
                inner.scenario_modified = true;
                Ok(serde_json::json!({ "status": "ok" }))
            }
            "list_objects" => {
                let inner = self.state.inner.read().await;
                let objects: Vec<_> = inner.objects.values().cloned().collect();
                Ok(serde_json::to_value(objects).map_err(|e| e.to_string())?)
            }
            "create_object" => {
                let obj: crate::state::ObjectEntry = serde_json::from_value(arguments["obj"].clone()).map_err(|e| e.to_string())?;
                let mut inner = self.state.inner.write().await;
                inner.objects.insert(obj.id.clone(), obj);
                inner.scenario_modified = true;
                Ok(serde_json::json!({ "status": "ok" }))
            }
            "delete_object" => {
                let id = arguments["id"].as_str().ok_or("id is required")?;
                let mut inner = self.state.inner.write().await;
                inner.objects.remove(id);
                inner.scenario_modified = true;
                Ok(serde_json::json!({ "status": "ok" }))
            }
            "appraise" => {
                let args: AppraiseRequest = serde_json::from_value(arguments.clone()).map_err(|e| e.to_string())?;
                let resp = StudioService::perform_appraise(&self.state, args).await.map_err(|e: AppError| e.to_string())?;
                Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
            }
            "apply_stimulus" => {
                let req: StimulusRequest = serde_json::from_value(arguments["req"].clone()).map_err(|e| e.to_string())?;
                let resp = StudioService::perform_stimulus(&self.state, req).await.map_err(|e: AppError| e.to_string())?;
                Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
            }
            "analyze_utterance" => {
                let utterance = arguments["utterance"].as_str().ok_or("utterance is required")?;
                let analyzer = self.state.analyzer.as_ref().ok_or("analyzer not available")?;
                let mut analyzer = analyzer.lock().await;
                let pad = analyzer.analyze(utterance).map_err(|e| format!("{:?}", e))?;
                Ok(serde_json::json!(pad))
            }
            "after_dialogue" => {
                let req: AfterDialogueRequest = serde_json::from_value(arguments["req"].clone()).map_err(|e| e.to_string())?;
                let resp = StudioService::perform_after_dialogue(&self.state, req).await.map_err(|e: AppError| e.to_string())?;
                Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
            }
            "get_history" => {
                let inner = self.state.inner.read().await;
                Ok(serde_json::to_value(&inner.turn_history).map_err(|e| e.to_string())?)
            }
            "get_situation" => {
                let inner = self.state.inner.read().await;
                Ok(inner.current_situation.clone().unwrap_or(serde_json::Value::Null))
            }
            "update_situation" => {
                let mut inner = self.state.inner.write().await;
                inner.current_situation = Some(arguments["body"].clone());
                inner.scenario_modified = true;
                Ok(serde_json::json!({ "status": "ok" }))
            }
            "get_test_report" => {
                let inner = self.state.inner.read().await;
                Ok(serde_json::json!({ "content": inner.test_report }))
            }
            "update_test_report" => {
                let content = arguments["content"].as_str().ok_or("content is required")?;
                let mut inner = self.state.inner.write().await;
                inner.test_report = content.to_string();
                inner.scenario_modified = true;
                Ok(serde_json::json!({ "status": "ok" }))
            }
            "list_scenarios" => {
                Ok(serde_json::to_value(StudioService::list_scenarios()).unwrap_or_default())
            }
            "get_scenario_meta" => {
                let inner = self.state.inner.read().await;
                Ok(serde_json::to_value(&inner.scenario).map_err(|e| e.to_string())?)
            }
            "save_scenario" => {
                let path = arguments["path"].as_str().ok_or("path is required")?;
                let save_type = arguments["save_type"].as_str();
                let inner = self.state.inner.read().await;
                inner.save_to_file(std::path::Path::new(path), save_type == Some("scenario")).map_err(|e| e.to_string())?;
                Ok(serde_json::json!({ "status": "ok", "path": path }))
            }
            "load_scenario" => {
                let path = arguments["path"].as_str().ok_or("path is required")?;
                let mut loaded = crate::state::StateInner::load_from_file(std::path::Path::new(path)).map_err(|e| e.to_string())?;
                loaded.loaded_path = Some(path.to_string());
                let mut inner = self.state.inner.write().await;
                *inner = loaded;
                Ok(serde_json::json!({ "status": "ok" }))
            }
            "get_npc_llm_config" => {
                let npc_id = arguments["npc_id"].as_str().ok_or("npc_id is required")?;
                let inner = self.state.inner.read().await;
                let npc_profile = inner.npcs.get(npc_id).ok_or_else(|| format!("NPC {} not found", npc_id))?;
                let (temp, top_p) = npc_profile.derive_llm_parameters();
                Ok(serde_json::json!({ "npc_id": npc_id, "temperature": temp, "top_p": top_p }))
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
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let mcp = state
        .mcp_server
        .as_ref()
        .ok_or_else(|| AppError::Internal("MCP server not initialized".into()))?;
    let (session_id, rx) = mcp.session_manager.create_session().await;
        tracing::info!("[MCP] SSE 연결: session={}", session_id);

    let initial_event = Event::default()
        .event("endpoint")
        .data(format!("/mcp/message?session_id={}", session_id));

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(|msg| Ok(Event::default().data(msg)));

    let combined_stream =
        futures_util::stream::once(async move { Ok(initial_event) }).chain(stream);

    Ok(Sse::new(combined_stream))
}

async fn mcp_message_handler(
    State(state): State<AppState>,
    Query(query): Query<SessionQuery>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let mcp = state
        .mcp_server
        .as_ref()
        .ok_or_else(|| AppError::Internal("MCP server not initialized".into()))?;
    let id = payload["id"].clone();
    let method = payload["method"].as_str().unwrap_or("");
    tracing::info!("[MCP] 요청: method={}, id={}, session={}", method, id, query.session_id);

    // notifications (id 없음) — 응답 불필요, 즉시 리턴
    if method.starts_with("notifications/") {
        return Ok(Json(serde_json::json!({"status": "ok"})));
    }

    let result = match method {
        "initialize" => {
            Ok(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "npc-mind-studio",
                    "version": "0.1.0"
                },
                "capabilities": {
                    "tools": {}
                }
            }))
        },
        "ping" => {
            Ok(serde_json::json!({}))
        },
        "tools/list" => {
            Ok(serde_json::json!({ "tools": mcp.list_tools() }))
        },
        "tools/call" => {
            let name = payload["params"]["name"].as_str().unwrap_or("");
            let arguments = &payload["params"]["arguments"];
            tracing::info!("[MCP] tools/call: name={}", name);
            let tool_result = mcp.call_tool(name, arguments).await;
            tracing::info!("[MCP] tools/call 결과: name={}, ok={}", name, tool_result.is_ok());
            // MCP CallToolResult 표준 형식으로 래핑
            match tool_result {
                Ok(val) => Ok(serde_json::json!({
                    "content": [{ "type": "text", "text": val.to_string() }]
                })),
                Err(e) => Ok(serde_json::json!({
                    "content": [{ "type": "text", "text": e }],
                    "isError": true
                }))
            }
        },
        _ => Err(format!("Method not found: {}", method)),
    };

    match result {
        Ok(res_val) => {
            let json_res = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": res_val
            });
            let send_ok = mcp.session_manager.send_to_session(&query.session_id, json_res.to_string()).await;
            tracing::info!("[MCP] SSE 응답 전송: method={}, id={}, send_ok={}", method, id, send_ok.is_ok());
            Ok(Json(serde_json::json!({"status": "sent"})))
        },
        Err(e) => {
            let json_err = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32603, "message": e }
            });
            let send_ok = mcp.session_manager.send_to_session(&query.session_id, json_err.to_string()).await;
            tracing::warn!("[MCP] SSE 에러 전송: method={}, err={}, send_ok={}", method, e, send_ok.is_ok());
            Ok(Json(serde_json::json!({"status": "error_sent"})))
        }
    }
}

#[derive(serde::Deserialize)]
pub struct SessionQuery {
    pub session_id: String,
}
