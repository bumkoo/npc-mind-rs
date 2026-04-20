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

use crate::events::StateEvent;
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

    /// list_scenarios 반환 경로(data/ 하위 상대경로)를 실제 파일 경로로 변환
    fn resolve_data_path(path: &str) -> String {
        let p = std::path::Path::new(path);
        if p.is_absolute() || path.starts_with("data/") || path.starts_with("data\\") {
            return path.to_string();
        }
        format!("data/{}", path)
    }

    /// Scene JSON을 SceneRequest DTO 형식으로 정규화 (create_full_scenario용)
    fn normalize_scene_json(mut scene: Value) -> Value {
        let Some(obj) = scene.as_object_mut() else { return scene; };
        if let Some(focuses) = obj.get_mut("focuses") {
            if focuses.is_object() {
                let map = focuses.as_object().cloned().unwrap_or_default();
                let mut arr: Vec<Value> = map.into_iter().map(|(key, mut focus_val)| {
                    if let Some(focus_obj) = focus_val.as_object_mut() {
                        if !focus_obj.contains_key("id") {
                            focus_obj.insert("id".to_string(), Value::String(key));
                        }
                    }
                    focus_val
                }).collect();
                for focus in arr.iter_mut() {
                    Self::flatten_focus_situation(focus);
                    Self::normalize_trigger_field(focus);
                }
                *focuses = Value::Array(arr);
            } else if focuses.is_array() {
                if let Some(arr) = focuses.as_array_mut() {
                    for focus in arr.iter_mut() {
                        Self::flatten_focus_situation(focus);
                        Self::normalize_trigger_field(focus);
                    }
                }
            }
        }
        scene
    }

    fn flatten_focus_situation(focus: &mut Value) {
        let Some(focus_obj) = focus.as_object_mut() else { return; };
        if let Some(situation) = focus_obj.remove("situation") {
            if let Value::Object(sit_map) = situation {
                for (k, v) in sit_map {
                    if !focus_obj.contains_key(&k) {
                        focus_obj.insert(k, v);
                    }
                }
            }
        }
    }

    fn normalize_trigger_field(focus: &mut Value) {
        let Some(focus_obj) = focus.as_object_mut() else { return; };
        if focus_obj.contains_key("trigger") { return; }
        let Some(old_trigger) = focus_obj.remove("trigger_to_next") else { return; };
        let Some(old_obj) = old_trigger.as_object() else { return; };
        let Some(conditions) = old_obj.get("conditions").and_then(|c| c.as_array()) else { return; };
        let converted: Vec<Value> = conditions.iter().filter_map(|c| {
            let co = c.as_object()?;
            let emotion = co.get("emotion")?.clone();
            let threshold = co.get("threshold")?.clone();
            let cond_type = co.get("type").and_then(|t| t.as_str()).unwrap_or("above");
            let mut new_cond = serde_json::Map::new();
            new_cond.insert("emotion".to_string(), emotion);
            new_cond.insert(cond_type.to_string(), threshold);
            Some(Value::Object(new_cond))
        }).collect();
        let logic_type = old_obj.get("type").and_then(|t| t.as_str()).unwrap_or("or");
        let trigger = if logic_type == "and" {
            Value::Array(vec![Value::Array(converted)])
        } else {
            Value::Array(converted.into_iter().map(|c| Value::Array(vec![c])).collect())
        };
        focus_obj.insert("trigger".to_string(), trigger);
    }

    /// 도구 목록 조회 (34개)
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
                "description": "상황을 평가하여 OCC 감정을 생성하고 LLM 연기 프롬프트를 반환합니다. Scene이 활성이면 situation 생략 가능 — 활성 Focus의 데이터를 자동 사용합니다.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "npc_id": { "type": "string" },
                        "partner_id": { "type": "string" },
                        "situation": { "type": "object", "description": "Scene 활성 시 생략 가능" }
                    },
                    "required": ["npc_id", "partner_id"]
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
            serde_json::json!({ "name": "save_scenario", "description": "현재 상태를 지정된 경로에 저장합니다. save_type: 'scenario'(시나리오 JSON, turn_history 제외), 'result'(결과 JSON, turn_history 포함, 기본값), 'report'(test_report 마크다운), 'all'(result JSON + report MD 동시 저장, 경로의 확장자만 .md로 변경).", "inputSchema": { "type": "object", "properties": { "path": { "type": "string" }, "save_type": { "type": "string", "enum": ["scenario", "result", "report", "all"] } }, "required": ["path"] } }),
            serde_json::json!({ "name": "load_scenario", "description": "지정된 경로의 시나리오 파일을 로드합니다.", "inputSchema": { "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"] } }),
            
            // 소스 텍스트 & 시나리오 생성
            serde_json::json!({ "name": "list_source_texts", "description": "data/ 폴더의 소스 텍스트(.txt) 파일 목록과 크기를 조회합니다.", "inputSchema": { "type": "object", "properties": {} } }),
            serde_json::json!({
                "name": "read_source_text",
                "description": "소스 텍스트 파일을 읽습니다. chapter를 지정하면 해당 챕터만, 생략하면 챕터 목록을 반환합니다.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "txt 파일 경로 (data/ 하위 상대경로 또는 파일명)" },
                        "chapter": { "type": "integer", "description": "읽을 챕터 번호 (1-based). 생략 시 챕터 목록 반환" }
                    },
                    "required": ["path"]
                }
            }),
            serde_json::json!({
                "name": "create_full_scenario",
                "description": "NPC, 관계, Scene을 한 번에 생성하고 시나리오 파일로 저장합니다. scenario 객체에 npcs/relationships/objects/scene/scenario(meta)를 포함합니다.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "save_path": { "type": "string", "description": "저장 경로 (data/ 하위, 예: treasure_island/ch01/session_001/scenario.json)" },
                        "scenario": { "type": "object", "description": "{ scenario: {name,description,notes}, npcs: {id: NpcProfile}, relationships: {key: RelData}, objects: {id: ObjEntry}, scene: SceneRequest }" }
                    },
                    "required": ["save_path", "scenario"]
                }
            }),

            // Scene 관리
            serde_json::json!({
                "name": "start_scene",
                "description": "Scene을 시작합니다. Focus 옵션 목록을 등록하고 초기 Focus를 자동 appraise합니다.",
                "inputSchema": { "type": "object", "properties": { "req": { "type": "object" } }, "required": ["req"] }
            }),
            serde_json::json!({ "name": "get_scene_info", "description": "현재 Scene의 Focus 상태를 조회합니다 (활성 Focus, 대기 Focus, trigger 조건).", "inputSchema": { "type": "object", "properties": {} } }),

            // 결과 관리
            serde_json::json!({ "name": "get_save_dir", "description": "현재 로드된 시나리오의 결과 저장 디렉토리 경로를 계산합니다.", "inputSchema": { "type": "object", "properties": {} } }),
            serde_json::json!({ "name": "load_result", "description": "테스트 결과 파일을 로드합니다 (턴 히스토리 포함).", "inputSchema": { "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"] } }),

            // LLM 대화 테스트 (chat feature)
            serde_json::json!({
                "name": "dialogue_start",
                "description": "대화 세션을 시작합니다. appraise 후 생성된 프롬프트를 system prompt로 로컬 LLM 세션을 생성합니다. Scene이 활성이면 situation 생략 가능. 반환값에 save_dir(결과 저장 경로)이 포함됩니다. (chat feature 필요)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": { "type": "string", "description": "세션 고유 ID" },
                        "appraise": { "type": "object", "description": "AppraiseRequest (npc_id, partner_id, situation). Scene이 활성이면 situation 생략 가능" }
                    },
                    "required": ["session_id", "appraise"]
                }
            }),
            serde_json::json!({
                "name": "dialogue_turn",
                "description": "대사를 LLM에 전송하고 NPC 역할로 응답을 받습니다. PAD stimulus가 자동 적용되고 Beat 전환 시 system prompt가 갱신됩니다. (chat feature 필요)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": { "type": "string" },
                        "npc_id": { "type": "string" },
                        "partner_id": { "type": "string" },
                        "utterance": { "type": "string", "description": "상대 대사 (Player 또는 대화 상대)" },
                        "pad": { "type": "object", "description": "수동 PAD 입력 {pleasure, arousal, dominance}. 생략 시 자동 분석" },
                        "situation_description": { "type": "string", "description": "상황 설명 (선택)" }
                    },
                    "required": ["session_id", "npc_id", "partner_id", "utterance"]
                }
            }),
            serde_json::json!({
                "name": "dialogue_end",
                "description": "대화 세션을 종료하고 관계를 갱신합니다. after_dialogue에 {npc_id, partner_id, significance}를 포함하면 세션 종료와 관계 갱신을 한 번에 처리합니다. (chat feature 필요)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": { "type": "string" },
                        "after_dialogue": { "type": "object", "description": "AfterDialogueRequest {npc_id, partner_id, significance}. 생략 시 관계 갱신 없이 종료" }
                    },
                    "required": ["session_id"]
                }
            }),

            // 테스트 스크립트
            serde_json::json!({
                "name": "get_next_utterance",
                "description": "현재 Beat의 test_script에서 다음 대사를 조회하고 커서를 전진합니다. 스크립트가 없거나 소진되면 exhausted=true를 반환합니다. advance=false로 호출하면 커서를 전진하지 않고 peek만 합니다.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "advance": { "type": "boolean", "description": "true(기본값)이면 커서 전진, false이면 peek만" }
                    }
                }
            }),

            // 기타
            serde_json::json!({
                "name": "get_npc_llm_config",
                "description": "NPC의 성격에 최적화된 LLM 생성 파라미터를 조회합니다.",
                "inputSchema": { "type": "object", "properties": { "npc_id": { "type": "string" } }, "required": ["npc_id"] }
            }),

            // 프롬프트 오버라이드 (A/B 테스트)
            serde_json::json!({
                "name": "set_prompt_override",
                "description": "TOML 형식의 프롬프트 오버라이드를 적용합니다. 빌트인 한국어 템플릿 위에 부분 덮어쓰기됩니다. 서버 재시작 없이 즉시 반영. 빈 문자열이면 기본값으로 복원. A/B 테스트에 사용합니다.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "overrides": { "type": "string", "description": "TOML 형식 오버라이드. 예: [template]\\nrole_instruction = \"당신은 {name}입니다.\"" },
                        "lang": { "type": "string", "description": "베이스 언어 (기본값: ko)" }
                    },
                    "required": ["overrides"]
                }
            }),
            serde_json::json!({
                "name": "get_prompt_override",
                "description": "현재 적용 중인 프롬프트 오버라이드 TOML을 조회합니다. 없으면 null 반환.",
                "inputSchema": { "type": "object", "properties": {} }
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
                {
                    let mut inner = self.state.inner.write().await;
                    inner.npcs.insert(npc.id.clone(), npc);
                    inner.scenario_modified = true;
                }
                self.state.emit(StateEvent::NpcChanged);
                Ok(serde_json::json!({ "status": "ok" }))
            }
            "delete_npc" => {
                let id = arguments["id"].as_str().ok_or("id is required")?;
                {
                    let mut inner = self.state.inner.write().await;
                    inner.npcs.remove(id);
                    inner.scenario_modified = true;
                }
                self.state.emit(StateEvent::NpcChanged);
                Ok(serde_json::json!({ "status": "ok" }))
            }
            "list_relationships" => {
                let inner = self.state.inner.read().await;
                let rels: Vec<_> = inner.relationships.values().cloned().collect();
                Ok(serde_json::to_value(rels).map_err(|e| e.to_string())?)
            }
            "create_relationship" => {
                let rel: crate::state::RelationshipData = serde_json::from_value(arguments["rel"].clone()).map_err(|e| e.to_string())?;
                {
                    let mut inner = self.state.inner.write().await;
                    inner.relationships.insert(rel.key(), rel);
                    inner.scenario_modified = true;
                }
                self.state.emit(StateEvent::RelationshipChanged);
                Ok(serde_json::json!({ "status": "ok" }))
            }
            "delete_relationship" => {
                let owner = arguments["owner_id"].as_str().ok_or("owner_id is required")?;
                let target = arguments["target_id"].as_str().ok_or("target_id is required")?;
                {
                    let mut inner = self.state.inner.write().await;
                    inner.relationships.remove(&format!("{}:{}", owner, target));
                    inner.scenario_modified = true;
                }
                self.state.emit(StateEvent::RelationshipChanged);
                Ok(serde_json::json!({ "status": "ok" }))
            }
            "list_objects" => {
                let inner = self.state.inner.read().await;
                let objects: Vec<_> = inner.objects.values().cloned().collect();
                Ok(serde_json::to_value(objects).map_err(|e| e.to_string())?)
            }
            "create_object" => {
                let obj: crate::state::ObjectEntry = serde_json::from_value(arguments["obj"].clone()).map_err(|e| e.to_string())?;
                {
                    let mut inner = self.state.inner.write().await;
                    inner.objects.insert(obj.id.clone(), obj);
                    inner.scenario_modified = true;
                }
                self.state.emit(StateEvent::ObjectChanged);
                Ok(serde_json::json!({ "status": "ok" }))
            }
            "delete_object" => {
                let id = arguments["id"].as_str().ok_or("id is required")?;
                {
                    let mut inner = self.state.inner.write().await;
                    inner.objects.remove(id);
                    inner.scenario_modified = true;
                }
                self.state.emit(StateEvent::ObjectChanged);
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
            "generate_guide" => {
                let mut req: npc_mind::application::dto::GuideRequest = serde_json::from_value(arguments["req"].clone()).map_err(|e| e.to_string())?;
                let response = {
                    let mut inner = self.state.inner.write().await;
                    // situation_description이 없으면 현재 상황에서 자동 추출
                    if req.situation_description.is_none() {
                        if let Some(ref sit) = inner.current_situation {
                            req.situation_description = sit.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
                        }
                    }
                    let result = crate::domain_sync::dispatch_generate_guide(&mut *inner, req)
                        .await
                        .map_err(|e| e.to_string())?;
                    let fmt = self.state.formatter.read().await;
                    result.format(&**fmt)
                };
                self.state.emit(StateEvent::GuideGenerated);
                Ok(serde_json::to_value(response).map_err(|e| e.to_string())?)
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
                {
                    let mut inner = self.state.inner.write().await;
                    inner.current_situation = Some(arguments["body"].clone());
                    inner.scenario_modified = true;
                }
                self.state.emit(StateEvent::SituationChanged);
                Ok(serde_json::json!({ "status": "ok" }))
            }
            "get_test_report" => {
                let inner = self.state.inner.read().await;
                Ok(serde_json::json!({ "content": inner.test_report }))
            }
            "update_test_report" => {
                let content = arguments["content"].as_str().ok_or("content is required")?;
                {
                    let mut inner = self.state.inner.write().await;
                    inner.test_report = content.to_string();
                    inner.scenario_modified = true;
                }
                self.state.emit(StateEvent::TestReportChanged);
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
                let resolved = Self::resolve_data_path(path);
                let save_type = arguments["save_type"].as_str();
                let inner = self.state.inner.read().await;
                let path_obj = std::path::Path::new(&resolved);
                match save_type {
                    Some("report") => {
                        inner.save_report_to_file(path_obj).map_err(|e| e.to_string())?;
                        drop(inner);
                        self.state.emit(StateEvent::ScenarioSaved);
                        Ok(serde_json::json!({ "status": "ok", "path": resolved, "saved": ["report"] }))
                    }
                    Some("all") => {
                        // JSON 결과 + 마크다운 레포트 동시 저장
                        inner.save_to_file(path_obj, false).map_err(|e| e.to_string())?;
                        let report_path = path_obj.with_extension("md");
                        let report_saved = inner.save_report_to_file(&report_path).is_ok();
                        let saved: Vec<&str> = if report_saved {
                            vec!["result", "report"]
                        } else {
                            vec!["result"]
                        };
                        drop(inner);
                        self.state.emit(StateEvent::ScenarioSaved);
                        Ok(serde_json::json!({
                            "status": "ok",
                            "path": resolved,
                            "report_path": report_path.to_string_lossy(),
                            "saved": saved,
                        }))
                    }
                    _ => {
                        // "scenario" | "result" | None: 기존 동작 유지
                        inner.save_to_file(path_obj, save_type == Some("scenario")).map_err(|e| e.to_string())?;
                        drop(inner);
                        self.state.emit(StateEvent::ScenarioSaved);
                        Ok(serde_json::json!({ "status": "ok", "path": resolved }))
                    }
                }
            }
            "load_scenario" => {
                let path = arguments["path"].as_str().ok_or("path is required")?;
                let resolved = Self::resolve_data_path(path);
                let mut loaded = crate::state::StateInner::load_from_file(std::path::Path::new(&resolved)).map_err(|e| e.to_string())?;
                loaded.loaded_path = Some(resolved.clone());
                // Scene 자동 복원: scene 필드가 있으면 런타임 상태에 반영
                let mut scene_restored = false;
                if let Some(ref scene_val) = loaded.scene {
                    if let Ok(scene_req) = serde_json::from_value::<npc_mind::application::dto::SceneRequest>(scene_val.clone()) {
                        StudioService::load_scene_into_state(&mut loaded, &scene_req).await;
                        scene_restored = true;
                    }
                }
                {
                    let mut inner = self.state.inner.write().await;
                    *inner = loaded;
                }
                self.state.emit(StateEvent::ScenarioLoaded);
                Ok(serde_json::json!({ "status": "ok", "resolved_path": resolved, "scene_restored": scene_restored }))
            }
            "get_npc_llm_config" => {
                let npc_id = arguments["npc_id"].as_str().ok_or("npc_id is required")?;
                let inner = self.state.inner.read().await;
                let npc_profile = inner.npcs.get(npc_id).ok_or_else(|| format!("NPC {} not found", npc_id))?;
                let (temp, top_p) = npc_profile.derive_llm_parameters();
                Ok(serde_json::json!({ "npc_id": npc_id, "temperature": temp, "top_p": top_p }))
            }
            "start_scene" => {
                let req: npc_mind::application::dto::SceneRequest = serde_json::from_value(arguments["req"].clone()).map_err(|e| e.to_string())?;
                let response = {
                    let mut inner = self.state.inner.write().await;
                    let collector = self.state.collector.clone();
                    collector.take_entries();
                    let mut result = crate::domain_sync::dispatch_start_scene(&mut *inner, req)
                        .await
                        .map_err(|e| e.to_string())?;
                    if let Some(ref mut initial) = result.initial_appraise {
                        initial.trace = collector.take_entries();
                    } else {
                        let _ = collector.take_entries();
                    }
                    let fmt = self.state.formatter.read().await;
                    result.format(&**fmt)
                };
                self.state.emit(StateEvent::SceneStarted);
                self.state.emit(StateEvent::HistoryChanged);
                Ok(serde_json::to_value(response).map_err(|e| e.to_string())?)
            }
            "get_scene_info" => {
                let inner = self.state.inner.read().await;
                let repo = crate::repository::ReadOnlyAppStateRepo { inner: &*inner };
                use npc_mind::ports::SceneStore;
                let mut info = match repo.get_scene() {
                    Some(scene) => npc_mind::application::scene_service::SceneService::new().build_scene_info(&scene),
                    None => npc_mind::application::dto::SceneInfoResult {
                        has_scene: false,
                        npc_id: None,
                        partner_id: None,
                        active_focus_id: None,
                        significance: None,
                        focuses: vec![],
                        script_cursor: None,
                    },
                };
                // 스크립트 커서 주입
                if info.has_scene {
                    info.script_cursor = Some(inner.script_cursor);
                }
                Ok(serde_json::to_value(info).map_err(|e| e.to_string())?)
            }
            "get_save_dir" => {
                let info = StudioService::get_save_dir(&self.state).await.map_err(|e: AppError| e.to_string())?;
                Ok(serde_json::to_value(info).map_err(|e| e.to_string())?)
            }
            "load_result" => {
                let path = arguments["path"].as_str().ok_or("path is required")?;
                let resolved = Self::resolve_data_path(path);
                let mut loaded = crate::state::StateInner::load_from_file(std::path::Path::new(&resolved)).map_err(|e| e.to_string())?;
                loaded.loaded_path = Some(resolved.clone());
                // Scene 복원
                if let Some(ref scene_val) = loaded.scene {
                    if let Ok(scene_req) = serde_json::from_value::<npc_mind::application::dto::SceneRequest>(scene_val.clone()) {
                        StudioService::load_scene_into_state(&mut loaded, &scene_req).await;
                    }
                }
                let history_count = loaded.turn_history.len();
                {
                    let mut inner = self.state.inner.write().await;
                    *inner = loaded;
                }
                self.state.emit(StateEvent::ResultLoaded);
                Ok(serde_json::json!({ "status": "ok", "resolved_path": resolved, "turn_count": history_count }))
            }
            "list_source_texts" => {
                let data_dir = std::path::Path::new("data");
                let mut files = Vec::new();
                if let Ok(entries) = std::fs::read_dir(data_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().map(|e| e == "txt").unwrap_or(false) {
                            if let Ok(meta) = std::fs::metadata(&path) {
                                files.push(serde_json::json!({
                                    "name": path.file_name().unwrap_or_default().to_string_lossy(),
                                    "path": path.to_string_lossy().replace('\\', "/"),
                                    "size_kb": meta.len() / 1024
                                }));
                            }
                        }
                    }
                }
                files.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
                Ok(serde_json::json!({ "files": files }))
            }
            "read_source_text" => {
                let path = arguments["path"].as_str().ok_or("path is required")?;
                let resolved = Self::resolve_data_path(path);
                // .txt 확장자가 없으면 data/ 직접 탐색
                let file_path = if std::path::Path::new(&resolved).exists() {
                    resolved.clone()
                } else {
                    // 파일명만 주어진 경우 data/ 에서 찾기
                    let candidate = format!("data/{}", path);
                    if std::path::Path::new(&candidate).exists() {
                        candidate
                    } else {
                        return Err(format!("File not found: {}", path));
                    }
                };
                let content = std::fs::read_to_string(&file_path).map_err(|e| format!("{}: {}", file_path, e))?;
                let lines: Vec<&str> = content.lines().collect();
                
                // 챕터 경계 감지 (regex 없이 문자열 매칭)
                fn is_chapter_heading(line: &str) -> bool {
                    let t = line.trim().to_uppercase();
                    (t.starts_with("CHAPTER ") || t.starts_with("BOOK ") || t.starts_with("PART "))
                        && t.len() > 5 && t.len() < 200
                }
                let mut chapters: Vec<(usize, String)> = Vec::new();
                for (i, line) in lines.iter().enumerate() {
                    if is_chapter_heading(line) {
                        chapters.push((i, line.trim().to_string()));
                    }
                }
                
                let chapter_num = arguments.get("chapter").and_then(|v| v.as_u64()).map(|v| v as usize);
                
                if let Some(ch) = chapter_num {
                    if ch == 0 || ch > chapters.len() {
                        return Err(format!("Chapter {} not found. Total chapters: {}", ch, chapters.len()));
                    }
                    let start = chapters[ch - 1].0;
                    let end = if ch < chapters.len() { chapters[ch].0 } else { lines.len() };
                    let chapter_lines: Vec<&str> = lines[start..end].to_vec();
                    let text = chapter_lines.join("\n");
                    Ok(serde_json::json!({
                        "chapter": ch,
                        "title": chapters[ch - 1].1,
                        "line_start": start + 1,
                        "line_end": end,
                        "line_count": end - start,
                        "text": text
                    }))
                } else {
                    // 챕터 목록 반환
                    let chapter_list: Vec<Value> = chapters.iter().enumerate().map(|(i, (line, title))| {
                        let next_line = if i + 1 < chapters.len() { chapters[i + 1].0 } else { lines.len() };
                        serde_json::json!({
                            "number": i + 1,
                            "title": title,
                            "line_start": line + 1,
                            "line_count": next_line - line
                        })
                    }).collect();
                    Ok(serde_json::json!({
                        "file": file_path,
                        "total_lines": lines.len(),
                        "chapter_count": chapters.len(),
                        "chapters": chapter_list
                    }))
                }
            }
            "create_full_scenario" => {
                let save_path = arguments["save_path"].as_str().ok_or("save_path is required")?;
                let resolved_path = Self::resolve_data_path(save_path);
                let scenario_val = &arguments["scenario"];
                
                // StateInner 생성
                let mut state_inner = crate::state::StateInner::default();
                state_inner.format = crate::state::FORMAT_SCENARIO.to_string();
                
                // scenario meta
                if let Some(meta) = scenario_val.get("scenario") {
                    state_inner.scenario = serde_json::from_value(meta.clone()).unwrap_or_default();
                }
                
                // npcs
                if let Some(npcs) = scenario_val.get("npcs") {
                    if let Ok(npcs_map) = serde_json::from_value::<std::collections::HashMap<String, crate::state::NpcProfile>>(npcs.clone()) {
                        state_inner.npcs = npcs_map;
                    }
                }
                
                // relationships — 키를 owner_id:target_id 형식으로 자동 정규화
                if let Some(rels) = scenario_val.get("relationships") {
                    if let Ok(rels_map) = serde_json::from_value::<std::collections::HashMap<String, crate::state::RelationshipData>>(rels.clone()) {
                        // 입력 키가 owner_id:target_id 형식이 아닐 수 있으므로,
                        // RelationshipData의 owner_id/target_id로부터 정규 키를 재생성
                        state_inner.relationships = rels_map.into_values()
                            .map(|rel| (rel.key(), rel))
                            .collect();
                    }
                }
                
                // objects
                if let Some(objs) = scenario_val.get("objects") {
                    if let Ok(objs_map) = serde_json::from_value::<std::collections::HashMap<String, crate::state::ObjectEntry>>(objs.clone()) {
                        state_inner.objects = objs_map;
                    }
                }
                
                // scene — focuses 객체→배열 변환 및 situation→event/action/object 평탄화
                // 저장 전에 SceneRequest 역직렬화로 필수 필드 검증
                let validated_scene_req = if let Some(scene) = scenario_val.get("scene") {
                    let normalized = Self::normalize_scene_json(scene.clone());
                    let scene_req = serde_json::from_value::<npc_mind::application::dto::SceneRequest>(normalized.clone())
                        .map_err(|e| format!("scene 검증 실패: {e}. 필수 필드: npc_id, partner_id, description, focuses"))?;
                    state_inner.scene = Some(normalized);
                    Some(scene_req)
                } else {
                    None
                };

                // 파일 저장
                state_inner.save_to_file(std::path::Path::new(&resolved_path), true).map_err(|e| e.to_string())?;

                // 서버 상태에도 로드
                state_inner.loaded_path = Some(resolved_path.clone());
                if let Some(scene_req) = validated_scene_req {
                    StudioService::load_scene_into_state(&mut state_inner, &scene_req).await;
                }
                let npc_count = state_inner.npcs.len();
                let rel_count = state_inner.relationships.len();
                {
                    let mut inner = self.state.inner.write().await;
                    *inner = state_inner;
                }
                self.state.emit(StateEvent::ScenarioLoaded);
                Ok(serde_json::json!({
                    "status": "ok",
                    "path": resolved_path,
                    "npcs": npc_count,
                    "relationships": rel_count
                }))
            }
            "get_next_utterance" => {
                let advance = arguments.get("advance").and_then(|v| v.as_bool()).unwrap_or(true);
                let mut inner = self.state.inner.write().await;
                // 현재 활성 Focus의 test_script에서 커서 위치의 대사를 반환
                let active_id = inner.active_focus_id.clone();
                let focus = active_id.as_deref().and_then(|id| {
                    inner.scene_focuses.iter().find(|f| f.id == id)
                });
                match focus {
                    Some(f) if !f.test_script.is_empty() => {
                        let cursor = inner.script_cursor;
                        if cursor < f.test_script.len() {
                            let utterance = f.test_script[cursor].clone();
                            let remaining = f.test_script.len() - cursor - 1;
                            let total = f.test_script.len();
                            let beat_id = f.id.clone();
                            if advance {
                                inner.script_cursor = cursor + 1;
                                drop(inner);
                                self.state.emit(StateEvent::SceneInfoChanged);
                            }
                            Ok(serde_json::json!({
                                "utterance": utterance,
                                "beat_id": beat_id,
                                "index": cursor,
                                "remaining": remaining,
                                "total": total,
                                "exhausted": false
                            }))
                        } else {
                            Ok(serde_json::json!({
                                "beat_id": f.id,
                                "index": cursor,
                                "total": f.test_script.len(),
                                "exhausted": true,
                                "message": "현재 Beat의 모든 스크립트 대사를 소진했습니다."
                            }))
                        }
                    }
                    Some(f) => {
                        Ok(serde_json::json!({
                            "beat_id": f.id,
                            "exhausted": true,
                            "message": "현재 Beat에 test_script가 정의되지 않았습니다."
                        }))
                    }
                    None => {
                        Err("활성 Scene Focus가 없습니다. start_scene 또는 load_scenario를 먼저 호출하세요.".into())
                    }
                }
            }
            "dialogue_start" => {
                #[cfg(feature = "chat")]
                {
                    let req: npc_mind::application::dialogue_test_service::ChatStartRequest =
                        serde_json::from_value(arguments.clone()).map_err(|e| e.to_string())?;
                    let resp = StudioService::perform_chat_start(&self.state, req)
                        .await
                        .map_err(|e: AppError| e.to_string())?;
                    Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
                }
                #[cfg(not(feature = "chat"))]
                {
                    Err("chat feature가 비활성입니다. --features chat으로 빌드하세요.".into())
                }
            }
            "dialogue_turn" => {
                #[cfg(feature = "chat")]
                {
                    let req: npc_mind::application::dialogue_test_service::ChatTurnRequest =
                        serde_json::from_value(arguments.clone()).map_err(|e| e.to_string())?;
                    let resp = StudioService::perform_chat_message(&self.state, req)
                        .await
                        .map_err(|e: AppError| e.to_string())?;
                    Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
                }
                #[cfg(not(feature = "chat"))]
                {
                    Err("chat feature가 비활성입니다. --features chat으로 빌드하세요.".into())
                }
            }
            "dialogue_end" => {
                #[cfg(feature = "chat")]
                {
                    let req: npc_mind::application::dialogue_test_service::ChatEndRequest =
                        serde_json::from_value(arguments.clone()).map_err(|e| e.to_string())?;
                    let resp = StudioService::perform_chat_end(&self.state, req)
                        .await
                        .map_err(|e: AppError| e.to_string())?;
                    Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
                }
                #[cfg(not(feature = "chat"))]
                {
                    Err("chat feature가 비활성입니다. --features chat으로 빌드하세요.".into())
                }
            }
            "set_prompt_override" => {
                let overrides = arguments.get("overrides").and_then(|v| v.as_str()).unwrap_or("");
                let lang = arguments.get("lang").and_then(|v| v.as_str()).unwrap_or("ko");

                if overrides.is_empty() {
                    // 오버라이드 해제 → 기본 포맷터 복원
                    let base_toml = npc_mind::presentation::builtin_toml(lang)
                        .ok_or_else(|| format!("Unsupported language: {}", lang))?;
                    let bundle = npc_mind::presentation::locale::LocaleBundle::from_toml(base_toml)
                        .map_err(|e| format!("Failed to parse base TOML: {}", e))?;
                    let new_fmt = Arc::new(npc_mind::presentation::formatter::LocaleFormatter::new(bundle))
                        as Arc<dyn npc_mind::ports::GuideFormatter>;
                    *self.state.formatter.write().await = Arc::clone(&new_fmt);
                    *self.state.locale_overrides.write().await = None;
                    Ok(serde_json::json!({ "status": "reset", "lang": lang }))
                } else {
                    // TOML 오버라이드 적용
                    let base_toml = npc_mind::presentation::builtin_toml(lang)
                        .ok_or_else(|| format!("Unsupported language: {}", lang))?;
                    let bundle = npc_mind::presentation::locale::LocaleBundle::from_toml_with_overrides(base_toml, overrides)
                        .map_err(|e| format!("TOML parse error: {}", e))?;
                    let new_fmt = Arc::new(npc_mind::presentation::formatter::LocaleFormatter::new(bundle))
                        as Arc<dyn npc_mind::ports::GuideFormatter>;
                    *self.state.formatter.write().await = Arc::clone(&new_fmt);
                    *self.state.locale_overrides.write().await = Some(overrides.to_string());
                    Ok(serde_json::json!({ "status": "applied", "lang": lang, "overrides": overrides }))
                }
            }
            "get_prompt_override" => {
                let current = self.state.locale_overrides.read().await;
                match &*current {
                    Some(toml_str) => Ok(serde_json::json!({ "active": true, "overrides": toml_str })),
                    None => Ok(serde_json::json!({ "active": false, "overrides": null })),
                }
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
