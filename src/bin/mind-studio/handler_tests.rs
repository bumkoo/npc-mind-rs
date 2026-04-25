//! Mind Studio 핸들러 HTTP 레벨 통합 테스트
//!
//! `tower::ServiceExt::oneshot`으로 실제 HTTP 요청/응답을 검증합니다.
//! 서버를 띄우지 않고 라우터에 직접 요청을 보냅니다.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::state::*;
use crate::trace_collector::AppraisalCollector;
use npc_mind::ports::UtteranceAnalyzer;
use npc_mind::domain::pad::Pad;
use std::sync::{Arc, Mutex};

/// 분석 호출을 감시하는 테스트용 스파이 분석기
struct SpyAnalyzer {
    calls: Arc<Mutex<Vec<String>>>,
}

impl UtteranceAnalyzer for SpyAnalyzer {
    fn analyze(&mut self, utterance: &str) -> Result<Pad, npc_mind::ports::EmbedError> {
        let mut calls = self.calls.lock().unwrap();
        calls.push(utterance.to_string());
        Ok(Pad::new(0.1, 0.2, 0.3))
    }
}

/// 테스트용 AppState 생성 (Spy 분석기 포함)
#[allow(dead_code)] // chat feature 활성 시 test_state_with_spy_and_chat이 사용되지만, off 빌드에서는 이 헬퍼가 fallback
fn test_state_with_spy() -> (AppState, Arc<Mutex<Vec<String>>>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let spy = SpyAnalyzer { calls: calls.clone() };
    let state = AppState::new(AppraisalCollector::new(), Some(spy));
    let mcp = crate::mcp_server::create_mcp_server(state.clone());
    (state.with_mcp(mcp), calls)
}

/// chat 어댑터 호출을 무시하는 테스트용 mock ConversationPort.
/// `process_chat_turn_result`가 chat 어댑터 부재로 즉시 NotImplemented를 반환하지
/// 않게 하기 위한 최소 구현 — 분석기/심리 자극 검증 흐름에 집중.
#[cfg(feature = "chat")]
struct SilentChatPort;

#[cfg(feature = "chat")]
#[async_trait::async_trait]
impl npc_mind::ports::ConversationPort for SilentChatPort {
    async fn start_session(
        &self,
        _session_id: &str,
        _system_prompt: &str,
        _generation_config: Option<npc_mind::ports::LlmModelInfo>,
    ) -> Result<(), npc_mind::ports::ConversationError> {
        Ok(())
    }

    async fn send_message(
        &self,
        _session_id: &str,
        _user_message: &str,
    ) -> Result<npc_mind::ports::ChatResponse, npc_mind::ports::ConversationError> {
        Ok(npc_mind::ports::ChatResponse {
            text: "mock NPC response".to_string(),
            timings: None,
        })
    }

    async fn send_message_stream(
        &self,
        _session_id: &str,
        _user_message: &str,
        _token_tx: tokio::sync::mpsc::Sender<String>,
    ) -> Result<npc_mind::ports::ChatResponse, npc_mind::ports::ConversationError> {
        Ok(npc_mind::ports::ChatResponse {
            text: "mock NPC response".to_string(),
            timings: None,
        })
    }

    async fn update_system_prompt(
        &self,
        _session_id: &str,
        _new_prompt: &str,
    ) -> Result<(), npc_mind::ports::ConversationError> {
        Ok(())
    }

    async fn end_session(
        &self,
        _session_id: &str,
    ) -> Result<Vec<npc_mind::ports::DialogueTurn>, npc_mind::ports::ConversationError> {
        Ok(Vec::new())
    }
}

/// Spy 분석기 + Silent chat 어댑터를 모두 주입한 AppState.
/// `process_chat_turn_result` 검증용 — chat feature 활성 시에만 컴파일.
#[cfg(feature = "chat")]
fn test_state_with_spy_and_chat() -> (AppState, Arc<Mutex<Vec<String>>>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let spy = SpyAnalyzer { calls: calls.clone() };
    let state = AppState::new(AppraisalCollector::new(), Some(spy))
        .with_chat(Arc::new(SilentChatPort));
    let mcp = crate::mcp_server::create_mcp_server(state.clone());
    (state.with_mcp(mcp), calls)
}

/// 테스트용 AppState 생성 (embed 없음)
fn test_state() -> AppState {
    // None의 구체적 타입을 명시하여 타입 추론 오류 방지 (UtteranceAnalyzer 구현체 중 하나 선택)
    let state = AppState::new(AppraisalCollector::new(), None::<SpyAnalyzer>);
    let mcp = crate::mcp_server::create_mcp_server(state.clone());
    state.with_mcp(mcp)
}

/// 테스트용 라우터 생성
fn test_app() -> axum::Router {
    crate::build_api_router(test_state())
}

/// 응답 바디를 JSON으로 파싱하는 헬퍼
async fn body_json(response: axum::http::Response<Body>) -> serde_json::Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// JSON POST 요청 빌더 헬퍼
fn json_post(uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

/// JSON PUT 요청 빌더 헬퍼
fn json_put(uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("PUT")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

/// GET 요청 빌더 헬퍼
fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

/// DELETE 요청 빌더 헬퍼
fn delete(uri: &str) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

// 무백 캐릭터 프로필 (테스트 픽스처)
fn mu_baek_profile() -> serde_json::Value {
    serde_json::json!({
        "id": "mu_baek",
        "name": "무백",
        "description": "강호의 검객",
        "sincerity": 0.3, "fairness": 0.4, "greed_avoidance": 0.5, "modesty": 0.2,
        "fearfulness": -0.3, "anxiety": -0.2, "dependence": -0.4, "sentimentality": 0.1,
        "social_self_esteem": 0.5, "social_boldness": 0.6, "sociability": 0.0, "liveliness": -0.2,
        "forgiveness": -0.3, "gentleness": -0.2, "flexibility": -0.1, "patience": 0.3,
        "organization": 0.2, "diligence": 0.5, "perfectionism": 0.3, "prudence": 0.4,
        "aesthetic_appreciation": 0.3, "inquisitiveness": 0.2, "creativity": 0.1, "unconventionality": 0.0
    })
}

fn gyo_ryong_profile() -> serde_json::Value {
    serde_json::json!({
        "id": "gyo_ryong",
        "name": "교룡",
        "description": "떠돌이 도적",
        "sincerity": -0.5, "fairness": -0.3, "greed_avoidance": -0.6, "modesty": -0.4,
        "fearfulness": 0.1, "anxiety": 0.2, "dependence": -0.1, "sentimentality": -0.2,
        "social_self_esteem": 0.3, "social_boldness": 0.4, "sociability": 0.5, "liveliness": 0.6,
        "forgiveness": -0.5, "gentleness": -0.4, "flexibility": 0.3, "patience": -0.3,
        "organization": -0.3, "diligence": -0.2, "perfectionism": -0.1, "prudence": -0.4,
        "aesthetic_appreciation": 0.1, "inquisitiveness": 0.4, "creativity": 0.5, "unconventionality": 0.6
    })
}

fn relationship_data() -> serde_json::Value {
    serde_json::json!({
        "owner_id": "mu_baek",
        "target_id": "gyo_ryong",
        "closeness": -0.3,
        "trust": -0.5,
        "power": 0.4
    })
}

// =========================================================================
// NPC CRUD
// =========================================================================

#[tokio::test]
async fn npc_crud_lifecycle() {
    let app = test_app();

    // 1. 빈 목록 확인
    let resp = app.clone().oneshot(get("/api/npcs")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json.as_array().unwrap().len(), 0);

    // 2. NPC 생성
    let resp = app
        .clone()
        .oneshot(json_post("/api/npcs", mu_baek_profile()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. 목록에서 확인
    let resp = app.clone().oneshot(get("/api/npcs")).await.unwrap();
    let json = body_json(resp).await;
    let npcs = json.as_array().unwrap();
    assert_eq!(npcs.len(), 1);
    assert_eq!(npcs[0]["id"], "mu_baek");
    assert_eq!(npcs[0]["name"], "무백");

    // 4. 삭제
    let resp = app
        .clone()
        .oneshot(delete("/api/npcs/mu_baek"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 5. 삭제 확인
    let resp = app.clone().oneshot(get("/api/npcs")).await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json.as_array().unwrap().len(), 0);
}

// =========================================================================
// Relationship CRUD
// =========================================================================

#[tokio::test]
async fn relationship_crud_lifecycle() {
    let app = test_app();

    // 생성
    let resp = app
        .clone()
        .oneshot(json_post("/api/relationships", relationship_data()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 조회
    let resp = app
        .clone()
        .oneshot(get("/api/relationships"))
        .await
        .unwrap();
    let json = body_json(resp).await;
    let rels = json.as_array().unwrap();
    assert_eq!(rels.len(), 1);
    assert_eq!(rels[0]["owner_id"], "mu_baek");

    // 삭제
    let resp = app
        .clone()
        .oneshot(delete("/api/relationships/mu_baek/gyo_ryong"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 삭제 확인
    let resp = app
        .clone()
        .oneshot(get("/api/relationships"))
        .await
        .unwrap();
    let json = body_json(resp).await;
    assert_eq!(json.as_array().unwrap().len(), 0);
}

// =========================================================================
// Object CRUD
// =========================================================================

#[tokio::test]
async fn object_crud_lifecycle() {
    let app = test_app();

    let obj = serde_json::json!({
        "id": "ancient_sword",
        "description": "고대의 명검",
        "category": "weapon"
    });

    // 생성
    let resp = app
        .clone()
        .oneshot(json_post("/api/objects", obj))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 조회
    let resp = app.clone().oneshot(get("/api/objects")).await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json.as_array().unwrap().len(), 1);

    // 삭제
    let resp = app
        .clone()
        .oneshot(delete("/api/objects/ancient_sword"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

// =========================================================================
// 감정 평가 파이프라인
// =========================================================================

/// NPC + 관계 등록 → appraise 성공 검증
#[tokio::test]
async fn appraise_success() {
    let app = test_app();

    // 사전 데이터 등록
    app.clone()
        .oneshot(json_post("/api/npcs", mu_baek_profile()))
        .await
        .unwrap();
    app.clone()
        .oneshot(json_post("/api/npcs", gyo_ryong_profile()))
        .await
        .unwrap();
    app.clone()
        .oneshot(json_post("/api/relationships", relationship_data()))
        .await
        .unwrap();

    // 감정 평가
    let req = serde_json::json!({
        "npc_id": "mu_baek",
        "partner_id": "gyo_ryong",
        "situation": {
            "description": "교룡이 마을 사람들의 식량을 약탈했다",
            "event": {
                "description": "약탈 사건",
                "desirability_for_self": -0.7,
                "other": null,
                "prospect": null
            },
            "action": {
                "description": "교룡의 약탈 행위",
                "agent_id": "gyo_ryong",
                "praiseworthiness": -0.8
            },
            "object": null
        }
    });

    let resp = app.clone().oneshot(json_post("/api/appraise", req)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let json = body_json(resp).await;
    // 감정 목록이 비어있지 않아야 함
    assert!(!json["emotions"].as_array().unwrap().is_empty());
    // mood는 부정적이어야 함 (불쾌한 사건)
    assert!(json["mood"].as_f64().unwrap() < 0.0);
    // prompt가 비어있지 않아야 함
    assert!(!json["prompt"].as_str().unwrap().is_empty());
}

/// NPC가 없을 때 appraise → 404
#[tokio::test]
async fn appraise_npc_not_found() {
    let app = test_app();

    let req = serde_json::json!({
        "npc_id": "nonexistent",
        "partner_id": "also_missing",
        "situation": {
            "description": "test",
            "event": {
                "description": "test event",
                "desirability_for_self": 0.5,
                "other": null,
                "prospect": null
            }
        }
    });

    let resp = app.oneshot(json_post("/api/appraise", req)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // HandlerError variants가 npc_id를 owned String으로 보존하므로 요청된 NPC id가
    // 에러 메시지에 포함되어야 함 — 다중 NPC 환경에서 디버깅에 필수.
    let json = body_json(resp).await;
    let err = json["error"].as_str().unwrap();
    assert!(err.to_lowercase().contains("not found"), "error: {}", err);
    assert!(err.contains("nonexistent"), "error should include npc_id: {}", err);
}

/// appraise 없이 stimulus → 400 (EmotionStateNotFound)
#[tokio::test]
async fn stimulus_without_appraise_returns_bad_request() {
    let app = test_app();

    // NPC + 관계만 등록 (appraise 안 함)
    app.clone()
        .oneshot(json_post("/api/npcs", mu_baek_profile()))
        .await
        .unwrap();
    app.clone()
        .oneshot(json_post("/api/npcs", gyo_ryong_profile()))
        .await
        .unwrap();
    app.clone()
        .oneshot(json_post("/api/relationships", relationship_data()))
        .await
        .unwrap();

    let req = serde_json::json!({
        "npc_id": "mu_baek",
        "partner_id": "gyo_ryong",
        "pleasure": -0.3,
        "arousal": 0.5,
        "dominance": -0.2
    });

    let resp = app.oneshot(json_post("/api/stimulus", req)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// analyze-utterance (embed feature 없이) → 501
#[tokio::test]
async fn analyze_utterance_without_embed_returns_not_implemented() {
    let app = test_app();

    let req = serde_json::json!({ "utterance": "네 이놈!" });
    let resp = app
        .oneshot(json_post("/api/analyze-utterance", req))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
}

// =========================================================================
// appraise → stimulus → after_dialogue 풀 파이프라인
// =========================================================================

#[tokio::test]
async fn full_pipeline_appraise_stimulus_after_dialogue() {
    let app = test_app();

    // 데이터 등록
    app.clone()
        .oneshot(json_post("/api/npcs", mu_baek_profile()))
        .await
        .unwrap();
    app.clone()
        .oneshot(json_post("/api/npcs", gyo_ryong_profile()))
        .await
        .unwrap();
    app.clone()
        .oneshot(json_post("/api/relationships", relationship_data()))
        .await
        .unwrap();

    // 1. appraise
    let appraise_req = serde_json::json!({
        "npc_id": "mu_baek",
        "partner_id": "gyo_ryong",
        "situation": {
            "description": "교룡과의 조우",
            "event": {
                "description": "예상치 못한 만남",
                "desirability_for_self": -0.3,
                "other": null,
                "prospect": null
            }
        }
    });
    let resp = app
        .clone()
        .oneshot(json_post("/api/appraise", appraise_req))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. stimulus
    let stim_req = serde_json::json!({
        "npc_id": "mu_baek",
        "partner_id": "gyo_ryong",
        "pleasure": -0.5,
        "arousal": 0.6,
        "dominance": -0.3
    });
    let resp = app
        .clone()
        .oneshot(json_post("/api/stimulus", stim_req))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let stim_json = body_json(resp).await;
    assert!(!stim_json["emotions"].as_array().unwrap().is_empty());
    assert_eq!(stim_json["beat_changed"].as_bool().unwrap(), false);

    // 3. after_dialogue
    let after_req = serde_json::json!({
        "npc_id": "mu_baek",
        "partner_id": "gyo_ryong",
        "significance": 0.6
    });
    let resp = app
        .clone()
        .oneshot(json_post("/api/after-dialogue", after_req))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let after_json = body_json(resp).await;
    // 관계 변동 전후 값이 존재해야 함
    assert!(after_json["before"]["closeness"].as_f64().is_some());
    assert!(after_json["after"]["closeness"].as_f64().is_some());

    // 4. 턴 히스토리에 3건 기록
    let resp = app.clone().oneshot(get("/api/history")).await.unwrap();
    let history = body_json(resp).await;
    assert_eq!(history.as_array().unwrap().len(), 3);
}

// =========================================================================
// 상황 패널 상태 저장/조회 round-trip
// =========================================================================

#[tokio::test]
async fn situation_panel_round_trip() {
    let app = test_app();

    let sit = serde_json::json!({
        "description": "테스트 상황",
        "npcId": "mu_baek",
        "hasEvent": true
    });

    // 저장
    let resp = app
        .clone()
        .oneshot(json_put("/api/situation", sit.clone()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 조회
    let resp = app.clone().oneshot(get("/api/situation")).await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["description"], "테스트 상황");
    assert_eq!(json["npcId"], "mu_baek");
}

// =========================================================================
// Scene 정보 (초기 상태)
// =========================================================================

#[tokio::test]
async fn scene_info_empty_initially() {
    let app = test_app();

    let resp = app.oneshot(get("/api/scene-info")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let json = body_json(resp).await;
    assert_eq!(json["has_scene"].as_bool().unwrap(), false);
    assert_eq!(json["focuses"].as_array().unwrap().len(), 0);
}

// =========================================================================
// 시나리오 메타 (초기 상태)
// =========================================================================

#[tokio::test]
async fn scenario_meta_empty_initially() {
    let app = test_app();

    let resp = app.oneshot(get("/api/scenario-meta")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let json = body_json(resp).await;
    assert_eq!(json["name"], "");
}

// =========================================================================
// 상태머신 회귀 테스트 — scenario_modified / save_type / loaded_path
// =========================================================================

/// tempdir 하위에 고유 테스트 경로를 생성하는 헬퍼
fn test_path(name: &str) -> String {
    let dir = std::env::temp_dir().join("npc_mind_test");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name).to_string_lossy().replace('\\', "/")
}

/// 테스트 끝나고 임시 파일 정리
fn cleanup_test_path(path: &str) {
    let _ = std::fs::remove_file(path);
}

/// NPC + 관계 등록 헬퍼 (CRUD 후 scenario_modified = true)
async fn seed_data(app: &axum::Router) {
    app.clone()
        .oneshot(json_post("/api/npcs", mu_baek_profile()))
        .await
        .unwrap();
    app.clone()
        .oneshot(json_post("/api/npcs", gyo_ryong_profile()))
        .await
        .unwrap();
    app.clone()
        .oneshot(json_post("/api/relationships", relationship_data()))
        .await
        .unwrap();
}

// =========================================================================
// scenario_modified 플래그 — CRUD 후 true, save 후 false
// =========================================================================

#[tokio::test]
async fn scenario_modified_set_after_npc_crud() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());

    // 초기: modified = false
    {
        let inner = state.inner.read().await;
        assert!(!inner.scenario_modified, "초기 상태는 modified=false");
    }

    // NPC 생성 → modified = true
    app.clone()
        .oneshot(json_post("/api/npcs", mu_baek_profile()))
        .await
        .unwrap();

    {
        let inner = state.inner.read().await;
        assert!(inner.scenario_modified, "NPC 생성 후 modified=true");
    }
}

#[tokio::test]
async fn scenario_modified_set_after_relationship_crud() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());

    app.clone()
        .oneshot(json_post("/api/relationships", relationship_data()))
        .await
        .unwrap();

    let inner = state.inner.read().await;
    assert!(inner.scenario_modified, "관계 생성 후 modified=true");
}

#[tokio::test]
async fn scenario_modified_set_after_object_crud() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());

    let obj = serde_json::json!({
        "id": "sword", "description": "검", "category": "weapon"
    });
    app.clone()
        .oneshot(json_post("/api/objects", obj))
        .await
        .unwrap();

    let inner = state.inner.read().await;
    assert!(inner.scenario_modified, "오브젝트 생성 후 modified=true");
}

#[tokio::test]
async fn scenario_modified_set_after_put_situation() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());

    let sit = serde_json::json!({"description": "테스트"});
    app.clone()
        .oneshot(json_put("/api/situation", sit))
        .await
        .unwrap();

    let inner = state.inner.read().await;
    assert!(inner.scenario_modified, "상황 저장 후 modified=true");
}

#[tokio::test]
async fn scenario_modified_reset_after_scenario_save() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());
    let path = test_path("modified_reset_test.json");

    // CRUD → modified = true
    app.clone()
        .oneshot(json_post("/api/npcs", mu_baek_profile()))
        .await
        .unwrap();

    // scenario 저장 → modified = false
    let save_req = serde_json::json!({
        "path": path,
        "save_type": "scenario"
    });
    let resp = app.clone()
        .oneshot(json_post("/api/save", save_req))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    {
        let inner = state.inner.read().await;
        assert!(!inner.scenario_modified, "시나리오 저장 후 modified=false");
    }

    cleanup_test_path(&path);
}

// =========================================================================
// save_type 분기 — scenario vs result 포맷
// =========================================================================

#[tokio::test]
async fn save_type_scenario_excludes_turn_history() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());
    let path = test_path("save_scenario_format.json");

    // 데이터 등록
    seed_data(&app).await;

    // 시나리오 저장
    let save_req = serde_json::json!({
        "path": path,
        "save_type": "scenario"
    });
    app.clone()
        .oneshot(json_post("/api/save", save_req))
        .await
        .unwrap();

    // 파일 파싱: format=scenario, turn_history 없음
    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(content["format"], "mind-studio/scenario");
    assert!(
        content.get("turn_history").is_none(),
        "시나리오 파일에 turn_history가 없어야 함"
    );

    cleanup_test_path(&path);
}

#[tokio::test]
async fn save_type_result_includes_turn_history() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());
    let path = test_path("save_result_format.json");

    // 데이터 + appraise → turn_history 생성
    seed_data(&app).await;
    let appraise_req = serde_json::json!({
        "npc_id": "mu_baek",
        "partner_id": "gyo_ryong",
        "situation": {
            "description": "조우",
            "event": {
                "description": "만남",
                "desirability_for_self": -0.3,
                "other": null,
                "prospect": null
            }
        }
    });
    app.clone()
        .oneshot(json_post("/api/appraise", appraise_req))
        .await
        .unwrap();

    // 결과 저장
    let save_req = serde_json::json!({
        "path": path,
        "save_type": "result"
    });
    app.clone()
        .oneshot(json_post("/api/save", save_req))
        .await
        .unwrap();

    // 파일 파싱: format=result, turn_history 존재
    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(content["format"], "mind-studio/result");
    assert!(
        content["turn_history"].as_array().unwrap().len() > 0,
        "결과 파일에 turn_history가 있어야 함"
    );

    cleanup_test_path(&path);
}

// =========================================================================
// loaded_path 갱신 — save-as 후 경로 변경
// =========================================================================

#[tokio::test]
async fn loaded_path_updated_on_scenario_save() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());
    let path1 = test_path("loaded_path_s1.json");
    let path2 = test_path("loaded_path_s2.json");

    seed_data(&app).await;

    // 첫 번째 시나리오 저장 → loaded_path = path1
    let save1 = serde_json::json!({"path": path1, "save_type": "scenario"});
    app.clone()
        .oneshot(json_post("/api/save", save1))
        .await
        .unwrap();

    {
        let inner = state.inner.read().await;
        assert_eq!(inner.loaded_path.as_deref(), Some(path1.as_str()));
    }

    // 다른 이름으로 시나리오 저장 → loaded_path = path2
    let save2 = serde_json::json!({"path": path2, "save_type": "scenario"});
    app.clone()
        .oneshot(json_post("/api/save", save2))
        .await
        .unwrap();

    {
        let inner = state.inner.read().await;
        assert_eq!(
            inner.loaded_path.as_deref(),
            Some(path2.as_str()),
            "save-as 후 loaded_path가 새 경로로 갱신되어야 함"
        );
    }

    cleanup_test_path(&path1);
    cleanup_test_path(&path2);
}

#[tokio::test]
async fn loaded_path_not_changed_on_result_save() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());
    let scenario_path = test_path("lp_scenario.json");
    let result_path = test_path("lp_result.json");

    seed_data(&app).await;

    // 시나리오 저장 → loaded_path = scenario_path
    let save_s = serde_json::json!({"path": scenario_path, "save_type": "scenario"});
    app.clone()
        .oneshot(json_post("/api/save", save_s))
        .await
        .unwrap();

    // 결과 저장 → loaded_path 유지 (scenario_path)
    let save_r = serde_json::json!({"path": result_path, "save_type": "result"});
    app.clone()
        .oneshot(json_post("/api/save", save_r))
        .await
        .unwrap();

    {
        let inner = state.inner.read().await;
        assert_eq!(
            inner.loaded_path.as_deref(),
            Some(scenario_path.as_str()),
            "결과 저장은 loaded_path를 변경하지 않아야 함"
        );
    }

    cleanup_test_path(&scenario_path);
    cleanup_test_path(&result_path);
}

// =========================================================================
// 시나리오 로드 → turn_history 비움 + loaded_path 설정
// =========================================================================

#[tokio::test]
async fn load_scenario_clears_turn_history_and_sets_loaded_path() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());
    let path = test_path("load_scenario_test.json");

    seed_data(&app).await;

    // 시나리오 저장
    let save_req = serde_json::json!({"path": path, "save_type": "scenario"});
    app.clone()
        .oneshot(json_post("/api/save", save_req))
        .await
        .unwrap();

    // appraise → turn_history 1건 생성
    let appraise_req = serde_json::json!({
        "npc_id": "mu_baek",
        "partner_id": "gyo_ryong",
        "situation": {
            "description": "테스트",
            "event": {
                "description": "이벤트",
                "desirability_for_self": -0.5,
                "other": null,
                "prospect": null
            }
        }
    });
    app.clone()
        .oneshot(json_post("/api/appraise", appraise_req))
        .await
        .unwrap();

    // history에 1건 존재 확인
    let resp = app.clone().oneshot(get("/api/history")).await.unwrap();
    let history = body_json(resp).await;
    assert_eq!(history.as_array().unwrap().len(), 1);

    // 시나리오 다시 로드 → turn_history 비워짐
    let load_req = serde_json::json!({"path": path});
    let resp = app.clone()
        .oneshot(json_post("/api/load", load_req))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // history 비워짐 확인
    let resp = app.clone().oneshot(get("/api/history")).await.unwrap();
    let history = body_json(resp).await;
    assert_eq!(
        history.as_array().unwrap().len(),
        0,
        "시나리오 로드 후 turn_history가 비워져야 함"
    );

    // loaded_path 설정 확인
    {
        let inner = state.inner.read().await;
        assert_eq!(inner.loaded_path.as_deref(), Some(path.as_str()));
    }

    cleanup_test_path(&path);
}

// =========================================================================
// 결과 로드 → turn_history 포함 반환
// =========================================================================

#[tokio::test]
async fn load_result_returns_turn_history() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());
    let scenario_path = test_path("lr_scenario.json");
    let result_path = test_path("lr_result.json");

    seed_data(&app).await;

    // 시나리오 저장
    app.clone()
        .oneshot(json_post(
            "/api/save",
            serde_json::json!({"path": scenario_path, "save_type": "scenario"}),
        ))
        .await
        .unwrap();

    // appraise
    let appraise_req = serde_json::json!({
        "npc_id": "mu_baek",
        "partner_id": "gyo_ryong",
        "situation": {
            "description": "조우",
            "event": {
                "description": "이벤트",
                "desirability_for_self": -0.3,
                "other": null,
                "prospect": null
            }
        }
    });
    app.clone()
        .oneshot(json_post("/api/appraise", appraise_req))
        .await
        .unwrap();

    // 결과 저장
    app.clone()
        .oneshot(json_post(
            "/api/save",
            serde_json::json!({"path": result_path, "save_type": "result"}),
        ))
        .await
        .unwrap();

    // 결과 로드
    let resp = app
        .clone()
        .oneshot(json_post(
            "/api/load-result",
            serde_json::json!({"path": result_path}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let json = body_json(resp).await;
    let history = json["turn_history"].as_array().unwrap();
    assert!(
        !history.is_empty(),
        "결과 로드 시 turn_history가 반환되어야 함"
    );
    assert_eq!(history[0]["action"], "appraise");

    cleanup_test_path(&scenario_path);
    cleanup_test_path(&result_path);
}

// =========================================================================
// 전체 워크플로우 — 로드→수정→시나리오저장→평가→자극→결과저장→결과로드
// =========================================================================

#[tokio::test]
async fn full_state_machine_workflow() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());
    let s1_path = test_path("wf_s1.json");
    let s2_path = test_path("wf_s2.json");
    let result_path = test_path("wf_s2_result/1.json");

    // === 1. 초기 시나리오 생성 + 저장 ===
    seed_data(&app).await;
    app.clone()
        .oneshot(json_post(
            "/api/save",
            serde_json::json!({"path": s1_path, "save_type": "scenario"}),
        ))
        .await
        .unwrap();

    // === 2. 시나리오 로드 ===
    app.clone()
        .oneshot(json_post("/api/load", serde_json::json!({"path": s1_path})))
        .await
        .unwrap();

    {
        let inner = state.inner.read().await;
        assert_eq!(inner.loaded_path.as_deref(), Some(s1_path.as_str()));
        assert!(!inner.scenario_modified, "로드 직후 modified=false");
    }

    // === 3. 수정 (NPC 추가) → modified=true ===
    let extra_npc = serde_json::json!({
        "id": "extra", "name": "추가NPC", "description": "테스트용"
    });
    app.clone()
        .oneshot(json_post("/api/npcs", extra_npc))
        .await
        .unwrap();

    {
        let inner = state.inner.read().await;
        assert!(inner.scenario_modified, "수정 후 modified=true");
    }

    // === 4. 다른 이름으로 시나리오 저장 → loaded_path 갱신 ===
    app.clone()
        .oneshot(json_post(
            "/api/save",
            serde_json::json!({"path": s2_path, "save_type": "scenario"}),
        ))
        .await
        .unwrap();

    {
        let inner = state.inner.read().await;
        assert_eq!(
            inner.loaded_path.as_deref(),
            Some(s2_path.as_str()),
            "save-as 후 loaded_path가 s2로 변경"
        );
        assert!(!inner.scenario_modified, "저장 후 modified=false");
    }

    // === 5. 감정 평가 ===
    let appraise_req = serde_json::json!({
        "npc_id": "mu_baek",
        "partner_id": "gyo_ryong",
        "situation": {
            "description": "대치",
            "event": {
                "description": "교룡 재등장",
                "desirability_for_self": -0.6,
                "other": null,
                "prospect": null
            },
            "action": {
                "description": "위협",
                "agent_id": "gyo_ryong",
                "praiseworthiness": -0.7
            }
        }
    });
    let resp = app.clone()
        .oneshot(json_post("/api/appraise", appraise_req))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // === 6. 자극 적용 ===
    let stim_req = serde_json::json!({
        "npc_id": "mu_baek",
        "partner_id": "gyo_ryong",
        "pleasure": -0.4,
        "arousal": 0.5,
        "dominance": -0.2
    });
    let resp = app.clone()
        .oneshot(json_post("/api/stimulus", stim_req))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // === 7. after_dialogue ===
    let after_req = serde_json::json!({
        "npc_id": "mu_baek",
        "partner_id": "gyo_ryong",
        "significance": 0.5
    });
    app.clone()
        .oneshot(json_post("/api/after-dialogue", after_req))
        .await
        .unwrap();

    // === 8. 결과 저장 (loaded_path 유지) ===
    app.clone()
        .oneshot(json_post(
            "/api/save",
            serde_json::json!({"path": result_path, "save_type": "result"}),
        ))
        .await
        .unwrap();

    {
        let inner = state.inner.read().await;
        assert_eq!(
            inner.loaded_path.as_deref(),
            Some(s2_path.as_str()),
            "결과 저장 후에도 loaded_path는 시나리오 경로 유지"
        );
    }

    // === 9. 결과 파일 검증 ===
    let result_content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&result_path).unwrap()).unwrap();
    assert_eq!(result_content["format"], "mind-studio/result");
    assert!(result_content["turn_history"].as_array().unwrap().len() >= 3);

    // === 10. 결과 로드 ===
    let resp = app.clone()
        .oneshot(json_post(
            "/api/load-result",
            serde_json::json!({"path": result_path}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let history = json["turn_history"].as_array().unwrap();
    assert!(history.len() >= 3, "결과 로드 시 전체 turn_history 반환");

    // 정리
    cleanup_test_path(&s1_path);
    cleanup_test_path(&s2_path);
    cleanup_test_path(&result_path);
    let _ = std::fs::remove_dir(test_path("wf_s2_result"));
}

// =========================================================================
// NPC 삭제 후 scenario_modified 확인
// =========================================================================

#[tokio::test]
async fn scenario_modified_set_after_npc_delete() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());
    let path = test_path("delete_modified.json");

    // NPC 생성 + 시나리오 저장 → modified=false
    app.clone()
        .oneshot(json_post("/api/npcs", mu_baek_profile()))
        .await
        .unwrap();
    app.clone()
        .oneshot(json_post(
            "/api/save",
            serde_json::json!({"path": path, "save_type": "scenario"}),
        ))
        .await
        .unwrap();

    {
        let inner = state.inner.read().await;
        assert!(!inner.scenario_modified);
    }

    // NPC 삭제 → modified=true
    app.clone()
        .oneshot(delete("/api/npcs/mu_baek"))
        .await
        .unwrap();

    {
        let inner = state.inner.read().await;
        assert!(inner.scenario_modified, "NPC 삭제 후 modified=true");
    }

    cleanup_test_path(&path);
}

// =========================================================================
// 관계 삭제 후 scenario_modified 확인
// =========================================================================

#[tokio::test]
async fn scenario_modified_set_after_relationship_delete() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());
    let path = test_path("rel_delete_modified.json");

    // 관계 생성 + 시나리오 저장 → modified=false
    app.clone()
        .oneshot(json_post("/api/relationships", relationship_data()))
        .await
        .unwrap();
    app.clone()
        .oneshot(json_post(
            "/api/save",
            serde_json::json!({"path": path, "save_type": "scenario"}),
        ))
        .await
        .unwrap();

    // 관계 삭제 → modified=true
    app.clone()
        .oneshot(delete("/api/relationships/mu_baek/gyo_ryong"))
        .await
        .unwrap();

    {
        let inner = state.inner.read().await;
        assert!(inner.scenario_modified, "관계 삭제 후 modified=true");
    }

    cleanup_test_path(&path);
}

// =========================================================================
// 오브젝트 삭제 후 scenario_modified 확인
// =========================================================================

#[tokio::test]
async fn scenario_modified_set_after_object_delete() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());
    let path = test_path("obj_delete_modified.json");

    let obj = serde_json::json!({
        "id": "sword", "description": "검", "category": "weapon"
    });
    app.clone()
        .oneshot(json_post("/api/objects", obj))
        .await
        .unwrap();
    app.clone()
        .oneshot(json_post(
            "/api/save",
            serde_json::json!({"path": path, "save_type": "scenario"}),
        ))
        .await
        .unwrap();

    // 오브젝트 삭제 → modified=true
    app.clone()
        .oneshot(delete("/api/objects/sword"))
        .await
        .unwrap();

    {
        let inner = state.inner.read().await;
        assert!(inner.scenario_modified, "오브젝트 삭제 후 modified=true");
    }

    cleanup_test_path(&path);
}

// =========================================================================
// MCP 통합 테스트
// =========================================================================

#[tokio::test]
async fn mcp_endpoints_reachable() {
    let app = test_app();

    // 1. SSE 엔드포인트 확인
    let resp = app.clone().oneshot(get("/mcp/sse")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers()["content-type"], "text/event-stream");

    // 2. Message 엔드포인트 확인 (session_id 쿼리 필수)
    let req = serde_json::json!({"jsonrpc": "2.0", "method": "tools/list", "id": 1});
    let resp = app.clone().oneshot(json_post("/mcp/message?session_id=test-session", req)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn mcp_tool_call_logic() {
    let state = test_state();
    
    // 사전 데이터: NPC 등록
    {
        let mut inner = state.inner.write().await;
        let profile: NpcProfile = serde_json::from_value(mu_baek_profile()).unwrap();
        inner.npcs.insert(profile.id.clone(), profile);

        // 관계 등록 (mu_baek -> player)
        inner.relationships.insert("mu_baek:player".into(), RelationshipData {
            owner_id: "mu_baek".into(),
            target_id: "player".into(),
            closeness: 0.0,
            trust: 0.0,
            power: 0.0,
        });
    }
    // B5.2 (3/3): inner 직접 조작 후 공유 repo 재구성 필요 (REST CRUD 경로가 아니므로)
    state.rebuild_repo_from_inner().await;

    // 1. list_npcs 도구 테스트 (JSON 구조 사용)
    let mcp = state.mcp_server.as_ref().unwrap();
    let res: serde_json::Value = mcp.call_tool("list_npcs", &serde_json::json!({})).await.unwrap();
    assert_eq!(res.as_array().unwrap().len(), 1);
    assert_eq!(res[0]["id"], "mu_baek");

    // 2. get_npc_llm_config 도구 테스트
    let res: serde_json::Value = mcp.call_tool("get_npc_llm_config", &serde_json::json!({"npc_id": "mu_baek"})).await.unwrap();
    assert!(res["temperature"].as_f64().is_some());
    assert!(res["top_p"].as_f64().is_some());

    // 3. appraise 도구 테스트
    let res: serde_json::Value = mcp.call_tool("appraise", &serde_json::json!({
        "npc_id": "mu_baek",
        "partner_id": "player",
        "situation": {
            "description": "테스트 상황",
            "event": {
                "description": "선물",
                "desirability_for_self": 0.8
            }
        }
    })).await.unwrap();
    assert!(res["mood"].as_f64().unwrap() > 0.0);
}

#[tokio::test]
async fn test_studio_analysis_pipeline_integrity() {
    // chat feature 활성 시 SilentChatPort 주입 — process_chat_turn_result 호출에 필요
    #[cfg(feature = "chat")]
    let (state, _calls) = test_state_with_spy_and_chat();
    #[cfg(not(feature = "chat"))]
    let (state, _calls) = test_state_with_spy();

    // 1. 사전 데이터 등록
    {
        let mut inner = state.inner.write().await;
        let mu_baek: NpcProfile = serde_json::from_value(mu_baek_profile()).unwrap();
        let gyo_ryong: NpcProfile = serde_json::from_value(gyo_ryong_profile()).unwrap();
        inner.npcs.insert(mu_baek.id.clone(), mu_baek);
        inner.npcs.insert(gyo_ryong.id.clone(), gyo_ryong);
        inner.relationships.insert("mu_baek:gyo_ryong".into(), RelationshipData {
            owner_id: "mu_baek".into(),
            target_id: "gyo_ryong".into(),
            closeness: 0.0, trust: 0.0, power: 0.0,
        });
        // appraise 상태 생성
        inner.emotions.insert("mu_baek".into(), npc_mind::domain::emotion::EmotionState::default());
    }
    state.rebuild_repo_from_inner().await;

    #[cfg(feature = "chat")]
    {
        use npc_mind::application::dialogue_test_service::ChatTurnRequest;
        let chat_req = ChatTurnRequest {
            session_id: "test-session".into(),
            npc_id: "mu_baek".into(),
            partner_id: "gyo_ryong".into(),
            utterance: "사용자의 아주 화나는 대사".into(),
            pad: None, // 자동 분석 유도
            situation_description: None,
        };

        // 직접 호출 검증
        let (stim, _) = crate::studio_service::StudioService::process_chat_turn_result(
            &state, &chat_req, "NPC의 정중한 대답".into()
        ).await.unwrap();

        // 검증 A: 분석 대상이 NPC 대답이 아닌 '사용자 대사'여야 함
        let calls_lock = _calls.lock().unwrap();
        assert_eq!(calls_lock.len(), 1, "분석기가 한 번 호출되어야 함");
        assert_eq!(calls_lock[0], "사용자의 아주 화나는 대사", "분석 대상은 반드시 사용자 대사여야 함");

        // 검증 B: 반환된 결과에 input_pad가 포함되어야 함 (UI 슬라이더 반영용)
        let stimulus = stim.expect("자극 결과가 반환되어야 함");
        assert!(stimulus.input_pad.is_some(), "결과에 input_pad가 포함되어야 함");
        assert_eq!(stimulus.input_pad.unwrap().pleasure, 0.1);

        // 검증 C: 히스토리에 기록된 response에 input_pad가 저장되어야 함 (결과 로드용)
        let inner = state.inner.read().await;
        let last_turn = inner.turn_history.last().expect("히스토리가 기록되어야 함");
        assert_eq!(last_turn.action, "chat_message");
        assert!(last_turn.response["input_pad"].is_object(), "히스토리 응답 데이터에 input_pad 객체가 있어야 함");
        // f32(0.1) → JSON Number(f64)로 widening 시 정밀도가 노출되므로 epsilon 비교
        let pleasure = last_turn.response["input_pad"]["pleasure"]
            .as_f64()
            .expect("pleasure는 숫자여야 함");
        assert!(
            (pleasure - 0.1).abs() < 1e-5,
            "pleasure ≈ 0.1 (f32 정밀도), 실제={pleasure}"
        );
    }
}

// =========================================================================
// 테스트 결과 보고서 (Test Report) 검증
// =========================================================================

#[tokio::test]
async fn test_report_round_trip() {
    let app = test_app();

    let content = "# 테스트 결과 보고서\n\n- 감정 분석 완료\n- 프롬프트 적절함";
    let body = serde_json::json!({ "content": content });

    // 1. 보고서 저장 (PUT)
    let resp = app
        .clone()
        .oneshot(json_put("/api/test-report", body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. 보고서 조회 (GET)
    let resp = app.clone().oneshot(get("/api/test-report")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["content"], content);
}

#[tokio::test]
async fn test_report_persistence_across_save_load() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());
    let path = test_path("report_persistence.json");

    // 1. 보고서 데이터 설정
    let content = "영구 보존되어야 할 보고서 내용";
    app.clone()
        .oneshot(json_put("/api/test-report", serde_json::json!({ "content": content })))
        .await
        .unwrap();

    // 2. 시나리오 저장 (보고서 필드 포함됨)
    let save_req = serde_json::json!({
        "path": path,
        "save_type": "scenario"
    });
    app.clone().oneshot(json_post("/api/save", save_req)).await.unwrap();

    // 3. 상태 초기화 및 로드
    app.clone().oneshot(json_post("/api/load", serde_json::json!({ "path": path }))).await.unwrap();

    // 4. 보고서 복원 확인
    let resp = app.clone().oneshot(get("/api/test-report")).await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["content"], content, "시나리오 로드 후 보고서 내용이 복원되어야 함");

    cleanup_test_path(&path);
}

// =========================================================================
// LLM 모니터링 엔드포인트 테스트
// =========================================================================

/// chat feature 활성이지만 llm_monitor가 None일 때 501 반환
#[cfg(feature = "chat")]
#[tokio::test]
async fn llm_status_모니터_없으면_501() {
    let app = test_app();

    let resp = app.oneshot(get("/api/llm/status")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
}

#[cfg(feature = "chat")]
#[tokio::test]
async fn llm_health_모니터_없으면_501() {
    let app = test_app();

    let resp = app.oneshot(get("/api/llm/health")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
}

#[cfg(feature = "chat")]
#[tokio::test]
async fn llm_slots_모니터_없으면_501() {
    let app = test_app();

    let resp = app.oneshot(get("/api/llm/slots")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
}

#[cfg(feature = "chat")]
#[tokio::test]
async fn llm_metrics_모니터_없으면_501() {
    let app = test_app();

    let resp = app.oneshot(get("/api/llm/metrics")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
}

/// mock llama-server를 통한 통합 엔드포인트 테스트
#[cfg(feature = "chat")]
#[tokio::test]
async fn llm_status_mock_서버_통합() {
    use std::sync::Arc;

    // mock llama-server 기동
    let app_router = {
        use axum::{Router, routing::get as axum_get, Json};

        Router::new()
            .route("/health", axum_get(|| async {
                Json(serde_json::json!({ "status": "ok" }))
            }))
            .route("/slots", axum_get(|| async {
                Json(serde_json::json!([
                    { "id": 0, "state": 0, "n_past": 0, "n_predicted": 0, "is_processing": false }
                ]))
            }))
            .route("/metrics", axum_get(|| async {
                "llamacpp:kv_cache_usage_ratio 0.33\nllamacpp:prompt_tokens_total 100\n"
            }))
            .route("/v1/models", axum_get(|| async {
                Json(serde_json::json!({
                    "object": "list",
                    "data": [{ "id": "mock-model", "object": "model" }]
                }))
            }))
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app_router).await.unwrap() });

    // RigChatAdapter를 mock 서버에 연결
    let adapter = npc_mind::adapter::rig_chat::RigChatAdapter::new(
        &format!("http://{addr}/v1"),
        "mock-model",
    );
    let arc_adapter = Arc::new(adapter);

    // AppState에 모니터 설정
    let state = AppState::new(AppraisalCollector::new(), None::<SpyAnalyzer>)
        .with_llm_info(arc_adapter.clone())
        .with_llm_monitor(arc_adapter);
    let mcp = crate::mcp_server::create_mcp_server(state.clone());
    let app = crate::build_api_router(state.with_mcp(mcp));

    // GET /api/llm/status
    let resp = app.clone().oneshot(get("/api/llm/status")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;

    assert_eq!(json["health"]["status"], "ok");
    assert_eq!(json["model"]["model_name"], "mock-model");
    assert_eq!(json["slots"][0]["id"], 0);
    assert_eq!(json["metrics"]["kv_cache_usage_ratio"], 0.33);
    assert_eq!(json["metrics"]["prompt_tokens_total"], 100.0);

    // 개별 엔드포인트도 동작
    let resp = app.clone().oneshot(get("/api/llm/health")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "ok");

    let resp = app.clone().oneshot(get("/api/llm/slots")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json.as_array().unwrap().len() == 1);

    let resp = app.clone().oneshot(get("/api/llm/metrics")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["kv_cache_usage_ratio"], 0.33);
}

// ---------------------------------------------------------------------------
// SSE 이벤트 브로드캐스트 테스트
// ---------------------------------------------------------------------------

#[tokio::test]
async fn broadcast_npc_changed_on_create() {
    let state = test_state();
    let mut rx = state.event_tx.subscribe();

    let app = crate::build_api_router(state);

    // NPC 생성 → NpcChanged 이벤트 발행 확인
    let resp = app.oneshot(json_post("/api/npcs", mu_baek_profile())).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let event = rx.try_recv().unwrap();
    assert_eq!(event.name(), "npc_changed");
}

#[tokio::test]
async fn broadcast_npc_changed_on_delete() {
    let state = test_state();

    // NPC 먼저 생성
    let app = crate::build_api_router(state.clone());
    let resp = app.oneshot(json_post("/api/npcs", mu_baek_profile())).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 구독 시작 (생성 이벤트 이후)
    let mut rx = state.event_tx.subscribe();

    let app = crate::build_api_router(state);
    let resp = app.oneshot(delete("/api/npcs/mu_baek")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let event = rx.try_recv().unwrap();
    assert_eq!(event.name(), "npc_changed");
}

#[tokio::test]
async fn broadcast_situation_changed_on_put() {
    let state = test_state();
    let mut rx = state.event_tx.subscribe();

    let app = crate::build_api_router(state);
    let body = serde_json::json!({ "description": "테스트 상황" });
    let resp = app.oneshot(json_put("/api/situation", body)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let event = rx.try_recv().unwrap();
    assert_eq!(event.name(), "situation_changed");
}

#[tokio::test]
async fn broadcast_test_report_changed_on_put() {
    let state = test_state();
    let mut rx = state.event_tx.subscribe();

    let app = crate::build_api_router(state);
    let body = serde_json::json!({ "content": "# 테스트 보고서" });
    let resp = app.oneshot(json_put("/api/test-report", body)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let event = rx.try_recv().unwrap();
    assert_eq!(event.name(), "test_report_changed");
}

#[tokio::test]
async fn no_receivers_does_not_panic() {
    // 수신자 없이 emit해도 패닉 없음
    let state = test_state();
    state.emit(crate::events::StateEvent::NpcChanged);
    state.emit(crate::events::StateEvent::ScenarioLoaded);
}

// ===========================================================================
// B4 Session 3 Option B-Mini: v2 Director 통합 엔드포인트
// ===========================================================================

/// v2 API는 자체 Repository를 씀 — NPC/Relationship을 POST로 등록 후 Scene 시작.
/// 테스트 전용 헬퍼 — 무백·교룡 NPC + 중립 관계 등록 요청 바디 생성
fn npc_json_muback() -> serde_json::Value {
    serde_json::json!({
        "id": "mu_baek",
        "name": "무백",
        "description": "정의로운 검객",
        "personality": {
            "honesty_humility": { "sincerity": 0.8, "fairness": 0.7, "greed_avoidance": 0.6, "modesty": 0.5 },
            "emotionality": { "fearfulness": -0.6, "anxiety": -0.4, "dependence": -0.7, "sentimentality": 0.2 },
            "extraversion": { "social_self_esteem": 0.5, "social_boldness": 0.5, "sociability": 0.5, "liveliness": 0.5 },
            "agreeableness": { "forgiveness": 0.6, "gentleness": 0.7, "flexibility": 0.2, "patience": 0.8 },
            "conscientiousness": { "organization": 0.4, "diligence": 0.8, "perfectionism": 0.6, "prudence": 0.7 },
            "openness": { "aesthetic_appreciation": 0.3, "inquisitiveness": 0.4, "creativity": 0.3, "unconventionality": 0.2 }
        }
    })
}

fn npc_json_gyoryong() -> serde_json::Value {
    serde_json::json!({
        "id": "gyo_ryong",
        "name": "교룡",
        "description": "야심적인 검객",
        "personality": {
            "honesty_humility": { "sincerity": -0.4, "fairness": -0.5, "greed_avoidance": -0.6, "modesty": -0.7 },
            "emotionality": { "fearfulness": 0.8, "anxiety": 0.7, "dependence": 0.5, "sentimentality": 0.6 },
            "extraversion": { "social_self_esteem": 0.7, "social_boldness": 0.8, "sociability": 0.0, "liveliness": 0.6 },
            "agreeableness": { "forgiveness": -0.6, "gentleness": -0.5, "flexibility": -0.4, "patience": -0.7 },
            "conscientiousness": { "organization": -0.5, "diligence": -0.3, "perfectionism": -0.4, "prudence": -0.6 },
            "openness": { "aesthetic_appreciation": 0.6, "inquisitiveness": 0.8, "creativity": 0.7, "unconventionality": 0.9 }
        }
    })
}

fn rel_json_neutral(owner: &str, target: &str) -> serde_json::Value {
    serde_json::json!({
        "owner_id": owner,
        "target_id": target,
        "closeness": 0.0,
        "trust": 0.0,
        "power": 0.0,
    })
}

#[tokio::test]
async fn v2_empty_director_lists_no_active_scenes() {
    let app = test_app();
    let resp = app
        .oneshot(Request::builder().uri("/api/v2/scenes").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["scenes"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn v2_seed_npcs_and_start_scene_returns_scene_id() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());

    // 1. NPC 2명 등록
    for npc in [npc_json_muback(), npc_json_gyoryong()] {
        let resp = app.clone().oneshot(json_post("/api/v2/npcs", npc)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
    // 2. 중립 관계 등록
    let resp = app
        .clone()
        .oneshot(json_post("/api/v2/relationships", rel_json_neutral("mu_baek", "gyo_ryong")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Scene 시작 (Initial focus 1개)
    let start_body = serde_json::json!({
        "npc_id": "mu_baek",
        "partner_id": "gyo_ryong",
        "significance": 0.5,
        "focuses": [{
            "id": "initial",
            "description": "첫 만남",
            "trigger": null,
            "event": {
                "description": "마주침",
                "desirability_for_self": 0.2,
                "other": null,
                "prospect": null
            },
            "action": null,
            "object": null,
            "test_script": []
        }]
    });
    let resp = app
        .clone()
        .oneshot(json_post("/api/v2/scenes/start", start_body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["scene_id"]["npc_id"], "mu_baek");
    assert_eq!(body["scene_id"]["partner_id"], "gyo_ryong");
    // B4 Session 4: start_scene은 fire-and-forget — SceneId만 돌려준다.
    // 초기 이벤트 발행 관찰은 event_bus().subscribe() 경로에서 수행.

    // 4. GET /api/v2/scenes 에서 활성 Scene 확인
    let resp = app
        .clone()
        .oneshot(Request::builder().uri("/api/v2/scenes").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = body_json(resp).await;
    assert_eq!(body["scenes"].as_array().unwrap().len(), 1);
    assert_eq!(body["scenes"][0]["npc_id"], "mu_baek");
}

#[tokio::test]
async fn v2_dispatch_appraise_to_active_scene_emits_events() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());

    // seed + start scene
    for npc in [npc_json_muback(), npc_json_gyoryong()] {
        app.clone().oneshot(json_post("/api/v2/npcs", npc)).await.unwrap();
    }
    app.clone()
        .oneshot(json_post("/api/v2/relationships", rel_json_neutral("mu_baek", "gyo_ryong")))
        .await
        .unwrap();
    app.clone()
        .oneshot(json_post(
            "/api/v2/scenes/start",
            serde_json::json!({
                "npc_id": "mu_baek",
                "partner_id": "gyo_ryong",
                "focuses": [{
                    "id": "initial",
                    "description": "첫 만남",
                    "trigger": null,
                    "event": { "description": "x", "desirability_for_self": 0.1, "other": null, "prospect": null },
                    "action": null,
                    "object": null,
                    "test_script": []
                }]
            }),
        ))
        .await
        .unwrap();

    // Appraise 커맨드 dispatch
    let dispatch_body = serde_json::json!({
        "scene_id": { "npc_id": "mu_baek", "partner_id": "gyo_ryong" },
        "command": "appraise",
        "npc_id": "mu_baek",
        "partner_id": "gyo_ryong",
        "situation": {
            "description": "스트레스 상황",
            "event": { "description": "y", "desirability_for_self": -0.3, "other": null, "prospect": null },
            "action": null,
            "object": null
        }
    });
    let resp = app
        .clone()
        .oneshot(json_post("/api/v2/scenes/dispatch", dispatch_body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    // B4 Session 4: fire-and-forget — ack만 확인하고 실제 이벤트 발행은 EventStore에서 관찰.
    assert_eq!(body["ok"], serde_json::Value::Bool(true));

    // SceneTask가 StartScene + Appraise 커맨드를 처리할 시간 확보
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let events = state.director_v2.dispatcher().event_store().get_all_events();
    let kinds: Vec<String> = events.iter().map(|e| format!("{:?}", e.kind())).collect();
    assert!(kinds.iter().any(|k| k == "AppraiseRequested"), "got kinds: {:?}", kinds);
    assert!(kinds.iter().any(|k| k == "EmotionAppraised"), "got kinds: {:?}", kinds);
}

#[tokio::test]
async fn v2_end_scene_removes_from_active_list() {
    let state = test_state();
    let app = crate::build_api_router(state.clone());

    for npc in [npc_json_muback(), npc_json_gyoryong()] {
        app.clone().oneshot(json_post("/api/v2/npcs", npc)).await.unwrap();
    }
    app.clone()
        .oneshot(json_post("/api/v2/relationships", rel_json_neutral("mu_baek", "gyo_ryong")))
        .await
        .unwrap();
    app.clone()
        .oneshot(json_post(
            "/api/v2/scenes/start",
            serde_json::json!({
                "npc_id": "mu_baek",
                "partner_id": "gyo_ryong",
                "focuses": [{
                    "id": "initial",
                    "description": "x",
                    "trigger": null,
                    "event": { "description": "y", "desirability_for_self": 0.1, "other": null, "prospect": null },
                    "action": null,
                    "object": null,
                    "test_script": []
                }]
            }),
        ))
        .await
        .unwrap();

    // DELETE 경유 end
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v2/scenes/mu_baek/gyo_ryong")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // active list 비었는지 확인
    let resp = app
        .clone()
        .oneshot(Request::builder().uri("/api/v2/scenes").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = body_json(resp).await;
    assert_eq!(body["scenes"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn v2_dispatch_to_inactive_scene_returns_not_found() {
    // B4 Session 3 code review I3: SceneNotActive → 404 (이전: 409)
    let app = test_app();
    let body = serde_json::json!({
        "scene_id": { "npc_id": "ghost", "partner_id": "nobody" },
        "command": "appraise",
        "npc_id": "ghost",
        "partner_id": "nobody",
        "situation": null
    });
    let resp = app
        .oneshot(json_post("/api/v2/scenes/dispatch", body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn v2_start_scene_duplicate_returns_conflict() {
    // SceneAlreadyActive → 409 CONFLICT
    let state = test_state();
    let app = crate::build_api_router(state.clone());

    for npc in [npc_json_muback(), npc_json_gyoryong()] {
        app.clone().oneshot(json_post("/api/v2/npcs", npc)).await.unwrap();
    }
    app.clone()
        .oneshot(json_post("/api/v2/relationships", rel_json_neutral("mu_baek", "gyo_ryong")))
        .await
        .unwrap();
    let start_body = serde_json::json!({
        "npc_id": "mu_baek",
        "partner_id": "gyo_ryong",
        "focuses": [{
            "id": "initial", "description": "x", "trigger": null,
            "event": { "description": "y", "desirability_for_self": 0.1, "other": null, "prospect": null },
            "action": null, "object": null, "test_script": []
        }]
    });
    // 첫 시도 — OK
    let resp = app.clone().oneshot(json_post("/api/v2/scenes/start", start_body.clone())).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    // 같은 scene_id 재시도 → 409
    let resp = app.clone().oneshot(json_post("/api/v2/scenes/start", start_body)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn v2_dispatch_scene_mismatch_returns_bad_request() {
    // SceneMismatch → 400 BAD_REQUEST
    // scene_id=(mu_baek, gyo_ryong)인 활성 Scene에 command.npc/partner=(mu_baek, su_ryeon) 송신
    let state = test_state();
    let app = crate::build_api_router(state.clone());

    for npc in [npc_json_muback(), npc_json_gyoryong()] {
        app.clone().oneshot(json_post("/api/v2/npcs", npc)).await.unwrap();
    }
    app.clone()
        .oneshot(json_post("/api/v2/relationships", rel_json_neutral("mu_baek", "gyo_ryong")))
        .await
        .unwrap();
    app.clone()
        .oneshot(json_post(
            "/api/v2/scenes/start",
            serde_json::json!({
                "npc_id": "mu_baek",
                "partner_id": "gyo_ryong",
                "focuses": [{
                    "id": "initial", "description": "x", "trigger": null,
                    "event": { "description": "y", "desirability_for_self": 0.1, "other": null, "prospect": null },
                    "action": null, "object": null, "test_script": []
                }]
            }),
        ))
        .await
        .unwrap();

    // scene_id와 command.npc/partner 불일치
    let body = serde_json::json!({
        "scene_id": { "npc_id": "mu_baek", "partner_id": "gyo_ryong" },
        "command": "appraise",
        "npc_id": "mu_baek",
        "partner_id": "su_ryeon",  // Scene의 partner와 다름
        "situation": null
    });
    let resp = app
        .oneshot(json_post("/api/v2/scenes/dispatch", body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ---------------------------------------------------------------------------
// Step E1 — Memory / Rumor / World REST 엔드포인트 (embed feature 전용)
//
// `shared_dispatcher`가 `with_memory_full` + `with_rumor`로 배선된 상태에서 전체
// 흐름을 검증한다. `AppState::new()`가 `NPC_MIND_MEMORY_DB` 미설정 시 in-memory
// SQLite로 저장소를 구성하므로 테스트 격리는 자동이다.
// ---------------------------------------------------------------------------

#[cfg(feature = "embed")]
mod memory_endpoints {
    use super::*;
    use crate::events::StateEvent;

    /// NPC 두 명과 관계를 준비한 테스트 앱.
    fn seeded_app() -> (axum::Router, crate::state::AppState) {
        let state = test_state();
        let app = crate::build_api_router(state.clone());
        (app, state)
    }

    async fn seed_two_npcs(app: &axum::Router) {
        for npc in [mu_baek_profile(), gyo_ryong_profile()] {
            let resp = app.clone().oneshot(json_post("/api/npcs", npc)).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
        }
        let rel = serde_json::json!({
            "owner_id": "mu_baek",
            "target_id": "gyo_ryong",
            "closeness": 0.3,
            "trust": 0.2,
            "power": 0.1
        });
        let resp = app.clone().oneshot(json_post("/api/relationships", rel)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn manual_entry_seed_appears_in_by_npc() {
        let (app, _state) = seeded_app();
        seed_two_npcs(&app).await;

        // 작가 도구 경로: Seeded Personal 엔트리를 수동 주입.
        // JSON 직접 구성 — `MemoryEntry`의 serde default가 누락 필드를 채운다.
        // scope: `#[serde(tag = "kind", rename_all = "snake_case")]`.
        // source/provenance: `rename_all = "snake_case"`.
        // layer: `rename_all = "UPPERCASE"`.
        // memory_type: default PascalCase (serde alias 유지용).
        let body = serde_json::json!({
            "id": "seeded-1",
            "created_seq": 1,
            "event_id": 1,
            "scope": { "kind": "personal", "npc_id": "mu_baek" },
            "source": "experienced",
            "provenance": "seeded",
            "memory_type": "DialogueTurn",
            "layer": "A",
            "content": "첫 만남의 기억",
            "emotional_context": null,
            "timestamp_ms": 100,
            "npc_id": "mu_baek"
        });
        let resp = app
            .clone()
            .oneshot(json_post("/api/memory/entries", body))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let resp = app
            .clone()
            .oneshot(get("/api/memory/by-npc/mu_baek"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        let entries = body.get("entries").and_then(|e| e.as_array()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["id"], "seeded-1");
    }

    #[tokio::test]
    async fn tell_creates_memory_entry_and_emits_sse() {
        let (app, state) = seeded_app();
        seed_two_npcs(&app).await;

        // 테스트가 `state.emit` 이전에 구독해야 이벤트를 받을 수 있음.
        let mut rx = state.event_tx.subscribe();

        let req = serde_json::json!({
            "speaker": "gyo_ryong",
            "listeners": ["mu_baek"],
            "overhearers": [],
            "claim": "장문인이 교체됐다",
            "stated_confidence": 0.9,
            "origin_chain_in": [],
            "topic": "sect:leader"
        });
        let resp = app
            .clone()
            .oneshot(json_post("/api/memory/tell", req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["listeners_informed"], 1);

        // SSE 이벤트 — MemoryCreated 수신 확인.
        let mut saw_memory_created = false;
        while let Ok(ev) = rx.try_recv() {
            if matches!(ev, StateEvent::MemoryCreated) {
                saw_memory_created = true;
                break;
            }
        }
        assert!(saw_memory_created, "StateEvent::MemoryCreated가 방출되지 않음");

        // mu_baek에게 Heard 엔트리가 생성됐는지 확인.
        let resp = app
            .clone()
            .oneshot(get("/api/memory/by-npc/mu_baek"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        let entries = body.get("entries").and_then(|e| e.as_array()).unwrap();
        assert!(!entries.is_empty(), "mu_baek의 Heard 엔트리가 없음");
        assert_eq!(entries[0]["source"], "heard");

        // `/api/memory/search`도 동일 엔트리를 필터로 잡아야 함 (M4 스모크).
        let resp = app
            .clone()
            .oneshot(get("/api/memory/search?npc=mu_baek&source=heard"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        let entries = body.get("entries").and_then(|e| e.as_array()).unwrap();
        assert!(!entries.is_empty(), "search?source=heard 필터가 엔트리를 반환해야 함");
        assert!(entries.iter().all(|e| e["source"] == "heard"));
    }

    #[tokio::test]
    async fn apply_world_event_creates_canonical() {
        let (app, _state) = seeded_app();
        seed_two_npcs(&app).await;

        let req = serde_json::json!({
            "world_id": "jianghu",
            "topic": "sect:leader",
            "fact": "새 장문인은 백운이다",
            "significance": 0.8,
            "witnesses": []
        });
        let resp = app
            .clone()
            .oneshot(json_post("/api/world/apply-event", req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["applied"], true);

        let resp = app
            .clone()
            .oneshot(get("/api/memory/canonical/sect:leader"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        let entry = &body["entry"];
        assert!(entry.is_object(), "Canonical 엔트리가 반환되지 않음");
        assert_eq!(entry["provenance"], "seeded");
    }

    #[tokio::test]
    async fn by_topic_returns_history_including_superseded() {
        let (app, _state) = seeded_app();
        seed_two_npcs(&app).await;

        for fact in ["첫 발표", "두 번째 발표"] {
            let req = serde_json::json!({
                "world_id": "jianghu",
                "topic": "sect:news",
                "fact": fact,
                "significance": 0.5,
                "witnesses": []
            });
            let resp = app
                .clone()
                .oneshot(json_post("/api/world/apply-event", req))
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
        }

        let resp = app
            .clone()
            .oneshot(get("/api/memory/by-topic/sect:news"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        let entries = body.get("entries").and_then(|e| e.as_array()).unwrap();
        assert_eq!(entries.len(), 2, "superseded 포함 전체 이력을 반환해야 함");
    }

    #[tokio::test]
    async fn seed_then_spread_rumor_creates_recipient_memories() {
        let (app, state) = seeded_app();
        seed_two_npcs(&app).await;

        let mut rx = state.event_tx.subscribe();

        let seed_req = serde_json::json!({
            "topic": null,
            "seed_content": "강호에 도사가 나타났다",
            "reach": { "regions": [], "factions": [], "npc_ids": [], "min_significance": 0.0 },
            "origin": { "kind": "authored", "by": null }
        });
        let resp = app
            .clone()
            .oneshot(json_post("/api/rumors/seed", seed_req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        let rumor_id = body["rumor_id"].as_str().unwrap().to_string();

        // `GET /api/rumors` — 방금 시딩한 소문 1건이 목록에.
        let resp = app
            .clone()
            .oneshot(get("/api/rumors"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        let rumors = body.get("rumors").and_then(|v| v.as_array()).unwrap();
        assert_eq!(rumors.len(), 1);
        assert_eq!(rumors[0]["id"], rumor_id);

        // 확산.
        let spread_req = serde_json::json!({
            "recipients": ["mu_baek", "gyo_ryong"],
            "content_version": null
        });
        let uri = format!("/api/rumors/{}/spread", rumor_id);
        let resp = app
            .clone()
            .oneshot(json_post(&uri, spread_req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["hop_index"], 0);
        assert_eq!(body["recipient_count"], 2);

        // SSE — RumorSeeded + RumorSpread + MemoryCreated 방출됐는지.
        let mut saw_seed = false;
        let mut saw_spread = false;
        let mut saw_memory = false;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                StateEvent::RumorSeeded => saw_seed = true,
                StateEvent::RumorSpread => saw_spread = true,
                StateEvent::MemoryCreated => saw_memory = true,
                _ => {}
            }
        }
        assert!(saw_seed && saw_spread && saw_memory, "SSE 방출 누락: seed={} spread={} memory={}", saw_seed, saw_spread, saw_memory);

        // 각 수신자의 기억 목록에 Rumor source 엔트리가 있어야 함.
        for npc in ["mu_baek", "gyo_ryong"] {
            let uri = format!("/api/memory/by-npc/{}", npc);
            let resp = app.clone().oneshot(get(&uri)).await.unwrap();
            let body = body_json(resp).await;
            let entries = body.get("entries").and_then(|e| e.as_array()).unwrap();
            let has_rumor = entries.iter().any(|e| e["source"] == "rumor");
            assert!(has_rumor, "{}의 Rumor 엔트리가 없음", npc);
        }
    }

    // -------------------------------------------------------------------
    // Step E3.2 — 시나리오 JSON seeding
    // -------------------------------------------------------------------

    /// 시나리오 JSON을 tempfile에 써서 경로를 반환. TempDir은 호출자가 살아있게 소유.
    fn write_scenario(tmp: &tempfile::TempDir, json: &serde_json::Value) -> std::path::PathBuf {
        use std::io::Write;
        let path = tmp.path().join("scenario.json");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "{}", json).unwrap();
        path
    }

    #[tokio::test]
    async fn load_scenario_seeds_memory_and_rumor_into_stores() {
        let (app, _state) = seeded_app();

        // 시나리오 JSON 작성 — world_knowledge + faction_knowledge + initial_rumors.
        let scenario_json = serde_json::json!({
            "format": "mind-studio/scenario",
            "npcs": {},
            "relationships": {},
            "objects": {},
            "scenario": { "name": "seed-test", "description": "" },
            "world_knowledge": [
                {
                    "world_id": "jianghu",
                    "topic": "sect:leader",
                    "content": "장문인은 백운이다"
                }
            ],
            "faction_knowledge": {
                "sect_yun": [
                    { "id": "sy-1", "content": "문파의 비전은 천뢰검법이다" }
                ]
            },
            "initial_rumors": [
                {
                    "id": "r-seed-1",
                    "topic": "sect:leader",
                    "reach": { "regions": [], "factions": [], "npc_ids": [], "min_significance": 0.0 },
                    "origin": { "kind": "seeded" }
                }
            ]
        });

        let tmp = tempfile::tempdir().unwrap();
        let tmp_path = write_scenario(&tmp, &scenario_json);
        let load_req = serde_json::json!({"path": tmp_path.to_string_lossy()});
        let resp = app.clone().oneshot(json_post("/api/load", load_req)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // M1: load 응답이 warnings/applied count를 담는지 확인.
        let body = body_json(resp).await;
        assert_eq!(body["applied_rumors"], 1);
        assert_eq!(body["applied_memories"], 2);
        assert_eq!(body["warnings"].as_array().unwrap().len(), 0);

        // world_knowledge 검증 — canonical 조회.
        let resp = app
            .clone()
            .oneshot(get("/api/memory/canonical/sect:leader"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        let entry = &body["entry"];
        assert!(entry.is_object(), "Canonical 엔트리 부재");
        assert_eq!(entry["content"], "장문인은 백운이다");
        assert_eq!(entry["provenance"], "seeded");
        assert_eq!(entry["scope"]["kind"], "world");
        assert_eq!(entry["scope"]["world_id"], "jianghu");

        // faction_knowledge 검증 — search로 source/scope 필터.
        let resp = app
            .clone()
            .oneshot(get("/api/memory/search?source=experienced&limit=50"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        let entries = body["entries"].as_array().unwrap();
        let has_faction = entries.iter().any(|e| e["id"] == "sy-1");
        assert!(has_faction, "sect_yun 문파 지식 시드가 저장되지 않음");

        // initial_rumors 검증 — /api/rumors 목록.
        let resp = app.clone().oneshot(get("/api/rumors")).await.unwrap();
        let body = body_json(resp).await;
        let rumors = body["rumors"].as_array().unwrap();
        assert!(
            rumors.iter().any(|r| r["id"] == "r-seed-1"),
            "initial_rumors가 시딩되지 않음",
        );
    }

    #[tokio::test]
    async fn load_scenario_without_seed_sections_leaves_stores_empty() {
        let (app, _state) = seeded_app();

        // 시드 섹션 전혀 없는 "기존 시나리오" 포맷 — 회귀 감시.
        let scenario_json = serde_json::json!({
            "format": "mind-studio/scenario",
            "npcs": {},
            "relationships": {},
            "objects": {},
            "scenario": { "name": "no-seed", "description": "" }
        });

        let tmp = tempfile::tempdir().unwrap();
        let tmp_path = write_scenario(&tmp, &scenario_json);
        let load_req = serde_json::json!({"path": tmp_path.to_string_lossy()});
        let resp = app.clone().oneshot(json_post("/api/load", load_req)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // 소문 목록 비어 있음.
        let resp = app.clone().oneshot(get("/api/rumors")).await.unwrap();
        let body = body_json(resp).await;
        assert_eq!(body["rumors"].as_array().unwrap().len(), 0);
    }

    // H1 회귀 — 두 시나리오 순차 로드 시 두 번째가 첫 번째의 시드를 덮어씀.
    #[tokio::test]
    async fn consecutive_scenario_loads_clear_previous_seeds() {
        let (app, _state) = seeded_app();

        let scenario_a = serde_json::json!({
            "format": "mind-studio/scenario",
            "npcs": {},
            "relationships": {},
            "objects": {},
            "scenario": { "name": "A", "description": "" },
            "initial_rumors": [
                {
                    "id": "rumor-a",
                    "topic": "a:topic",
                    "reach": { "regions": [], "factions": [], "npc_ids": [], "min_significance": 0.0 },
                    "origin": { "kind": "seeded" }
                }
            ]
        });
        let scenario_b = serde_json::json!({
            "format": "mind-studio/scenario",
            "npcs": {},
            "relationships": {},
            "objects": {},
            "scenario": { "name": "B", "description": "" },
            "initial_rumors": [
                {
                    "id": "rumor-b",
                    "topic": "b:topic",
                    "reach": { "regions": [], "factions": [], "npc_ids": [], "min_significance": 0.0 },
                    "origin": { "kind": "seeded" }
                }
            ]
        });

        let tmp = tempfile::tempdir().unwrap();
        let path_a = write_scenario(&tmp, &scenario_a);
        let resp = app.clone().oneshot(json_post("/api/load", serde_json::json!({"path": path_a.to_string_lossy()}))).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // A 로드 직후 rumor-a만 존재.
        let resp = app.clone().oneshot(get("/api/rumors")).await.unwrap();
        let body = body_json(resp).await;
        let rumors = body["rumors"].as_array().unwrap();
        assert_eq!(rumors.len(), 1);
        assert_eq!(rumors[0]["id"], "rumor-a");

        // B 로드 후 rumor-a는 사라지고 rumor-b만 남아야 함.
        let path_b = write_scenario(&tmp, &scenario_b);
        let resp = app.clone().oneshot(json_post("/api/load", serde_json::json!({"path": path_b.to_string_lossy()}))).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let resp = app.clone().oneshot(get("/api/rumors")).await.unwrap();
        let body = body_json(resp).await;
        let rumors = body["rumors"].as_array().unwrap();
        assert_eq!(rumors.len(), 1, "H1: 이전 시나리오의 rumor-a가 clear되지 않음");
        assert_eq!(rumors[0]["id"], "rumor-b");
    }

    // L4 회귀 — origin 미지정도 기본 Seeded로 파싱되어 시딩 성공.
    #[tokio::test]
    async fn load_scenario_rumor_without_origin_defaults_to_seeded() {
        let (app, _state) = seeded_app();

        let scenario_json = serde_json::json!({
            "format": "mind-studio/scenario",
            "npcs": {},
            "relationships": {},
            "objects": {},
            "scenario": { "name": "no-origin", "description": "" },
            "initial_rumors": [
                { "id": "r-implicit", "topic": "t:x" }
            ]
        });

        let tmp = tempfile::tempdir().unwrap();
        let tmp_path = write_scenario(&tmp, &scenario_json);
        let load_req = serde_json::json!({"path": tmp_path.to_string_lossy()});
        let resp = app.clone().oneshot(json_post("/api/load", load_req)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["applied_rumors"], 1);
        assert_eq!(body["warnings"].as_array().unwrap().len(), 0);

        // 저장된 rumor의 origin이 seeded인지.
        let resp = app.clone().oneshot(get("/api/rumors")).await.unwrap();
        let body = body_json(resp).await;
        let rumor = &body["rumors"][0];
        assert_eq!(rumor["id"], "r-implicit");
        assert_eq!(rumor["origin"]["kind"], "seeded");
    }

    // M1 회귀 — 잘못된 시드는 warnings에 수집되고 나머지 시드는 적용.
    #[tokio::test]
    async fn invalid_seed_is_reported_in_warnings() {
        let (app, _state) = seeded_app();

        // initial_rumors[0]: topic/seed_content 모두 없음 → OrphanRumorMissingSeed 에러.
        // initial_rumors[1]: 정상 — 이 건은 적용돼야 함.
        let scenario_json = serde_json::json!({
            "format": "mind-studio/scenario",
            "npcs": {},
            "relationships": {},
            "objects": {},
            "scenario": { "name": "invalid", "description": "" },
            "initial_rumors": [
                { "id": "broken" },
                { "id": "ok", "topic": "t:ok" }
            ]
        });

        let tmp = tempfile::tempdir().unwrap();
        let tmp_path = write_scenario(&tmp, &scenario_json);
        let load_req = serde_json::json!({"path": tmp_path.to_string_lossy()});
        let resp = app.clone().oneshot(json_post("/api/load", load_req)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["applied_rumors"], 1, "정상 시드는 적용");
        let warnings = body["warnings"].as_array().unwrap();
        assert!(!warnings.is_empty(), "잘못된 시드의 warning이 응답에 포함되어야 함");
        let joined: String = warnings.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" | ");
        assert!(joined.contains("rumor[0]"), "warning 메시지에 실패한 index가 포함되어야 함");
    }
}

// =========================================================================
// Read Side Activation — Projection 기반 쿼리 drift 감지
// =========================================================================
//
// `/api/appraise`·`/api/scene` 같은 Write 경로가 실행된 뒤, `/api/projection/*`
// Read 경로가 **같은 값**을 돌려준다는 것을 보증한다. dispatcher Inline phase가
// 업데이트하는 Projection과 query 핸들러가 읽는 Projection이 동일한 Arc여야
// 테스트가 통과하므로, Arc 불일치(= silent drift)가 발생하면 즉시 실패한다.

mod projection_drift {
    use super::*;

    /// state를 유지한 채 Router를 생성하는 헬퍼 (appraise 후 projection 직접 검증용)
    fn app_with_state() -> (axum::Router, AppState) {
        let state = test_state();
        let app = crate::build_api_router(state.clone());
        (app, state)
    }

    async fn seed_two_npcs_and_rel(app: &axum::Router) {
        app.clone()
            .oneshot(json_post("/api/npcs", mu_baek_profile()))
            .await
            .unwrap();
        app.clone()
            .oneshot(json_post("/api/npcs", gyo_ryong_profile()))
            .await
            .unwrap();
        app.clone()
            .oneshot(json_post("/api/relationships", relationship_data()))
            .await
            .unwrap();
    }

    /// Write: `/api/appraise` → Read: `/api/projection/emotion/:id`
    ///
    /// `AppraiseResponse.mood`와 `EmotionProjection.get_mood`가 일치해야 한다.
    /// Arc가 분리되어 있으면 Projection 쪽이 `None`을 반환하여 이 테스트가 실패.
    #[tokio::test]
    async fn emotion_projection_matches_appraise_response() {
        let (app, _state) = app_with_state();
        seed_two_npcs_and_rel(&app).await;

        let appraise_req = serde_json::json!({
            "npc_id": "mu_baek",
            "partner_id": "gyo_ryong",
            "situation": {
                "description": "교룡이 마을 사람들의 식량을 약탈했다",
                "event": {
                    "description": "약탈 사건",
                    "desirability_for_self": -0.7,
                    "other": null,
                    "prospect": null
                },
                "action": {
                    "description": "교룡의 약탈 행위",
                    "agent_id": "gyo_ryong",
                    "praiseworthiness": -0.8
                },
                "object": null
            }
        });

        let resp = app
            .clone()
            .oneshot(json_post("/api/appraise", appraise_req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let write_body = body_json(resp).await;
        let expected_mood = write_body["mood"].as_f64().expect("mood must be f64") as f32;

        let resp = app
            .clone()
            .oneshot(get("/api/projection/emotion/mu_baek"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let read_body = body_json(resp).await;

        assert_eq!(read_body["npc_id"], "mu_baek");
        let projection_mood = read_body["mood"]
            .as_f64()
            .expect("projection mood must be populated after appraise")
            as f32;

        assert!(
            (expected_mood - projection_mood).abs() < 1e-5,
            "drift detected: appraise.mood={expected_mood}, projection.mood={projection_mood}"
        );

        // dominant/snapshot도 비어 있지 않아야 — Arc가 공유되어 이벤트가 도달했다는 증거.
        assert!(
            !read_body["dominant"].is_null(),
            "projection dominant must be populated after appraise"
        );
        let snapshot = read_body["snapshot"]
            .as_array()
            .expect("projection snapshot must be an array");
        assert!(!snapshot.is_empty(), "projection snapshot must not be empty");
    }

    /// appraise 전에는 projection이 비어 있어야 — 초기 상태 sanity check.
    #[tokio::test]
    async fn emotion_projection_empty_before_any_command() {
        let (app, _state) = app_with_state();

        let resp = app
            .clone()
            .oneshot(get("/api/projection/emotion/mu_baek"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert!(body["mood"].is_null());
        assert!(body["dominant"].is_null());
        assert!(body["snapshot"].is_null());
    }

    /// Scene projection drift: `/api/scene` 시작 → `/api/projection/scene`이
    /// `is_active=true`·`active_focus_id`를 반환.
    #[tokio::test]
    async fn scene_projection_reflects_scene_start() {
        let (app, _state) = app_with_state();
        seed_two_npcs_and_rel(&app).await;

        // 초기 상태: 비활성
        let resp = app
            .clone()
            .oneshot(get("/api/projection/scene"))
            .await
            .unwrap();
        let body = body_json(resp).await;
        assert_eq!(body["is_active"], false);
        assert!(body["active_focus_id"].is_null());

        // Scene 시작 — trigger=null은 Initial focus로 간주된다 (SceneFocusInput::trigger).
        let scene_req = serde_json::json!({
            "npc_id": "mu_baek",
            "partner_id": "gyo_ryong",
            "description": "첫 접촉 장면",
            "significance": 0.5,
            "focuses": [
                {
                    "id": "focus-init",
                    "description": "첫 접촉",
                    "trigger": null,
                    "event": {
                        "description": "조우",
                        "desirability_for_self": -0.2,
                        "other": null,
                        "prospect": null
                    },
                    "action": null,
                    "object": null
                }
            ]
        });
        let resp = app
            .clone()
            .oneshot(json_post("/api/scene", scene_req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Read: projection이 활성이어야 함 + focus_id 노출
        let resp = app
            .clone()
            .oneshot(get("/api/projection/scene"))
            .await
            .unwrap();
        let body = body_json(resp).await;
        assert_eq!(
            body["is_active"], true,
            "scene projection must be active after /api/scene"
        );
        assert_eq!(
            body["active_focus_id"].as_str(),
            Some("focus-init"),
            "projection must expose initial focus id"
        );
    }

    /// SceneEnded drift 대칭 체크: scene 시작 → after_dialogue → projection이 비활성으로 돌아와야.
    ///
    /// `SceneProjection`은 `SceneEnded`에서 `active=None`으로 clear된다
    /// (`projection.rs` L164-166). `after_dialogue`가 `DialogueEndRequested` → `SceneEnded`
    /// follow-up을 발행하므로, Inline phase에서 같은 Arc의 Projection이 비워져야 한다.
    /// Arc가 분리되어 있으면 active 상태가 남아 이 테스트가 실패.
    #[tokio::test]
    async fn scene_projection_clears_after_dialogue_end() {
        let (app, _state) = app_with_state();
        seed_two_npcs_and_rel(&app).await;

        // Scene 시작
        let scene_req = serde_json::json!({
            "npc_id": "mu_baek",
            "partner_id": "gyo_ryong",
            "description": "대화 장면",
            "significance": 0.5,
            "focuses": [
                {
                    "id": "focus-init",
                    "description": "첫 접촉",
                    "trigger": null,
                    "event": {
                        "description": "조우",
                        "desirability_for_self": -0.2,
                        "other": null,
                        "prospect": null
                    },
                    "action": null,
                    "object": null
                }
            ]
        });
        let resp = app
            .clone()
            .oneshot(json_post("/api/scene", scene_req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // pre-condition: projection 활성
        let body = body_json(
            app.clone()
                .oneshot(get("/api/projection/scene"))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(body["is_active"], true, "pre-condition: scene active");

        // after_dialogue → SceneEnded → projection clear
        let after_req = serde_json::json!({
            "npc_id": "mu_baek",
            "partner_id": "gyo_ryong",
            "significance": 0.6
        });
        let resp = app
            .clone()
            .oneshot(json_post("/api/after-dialogue", after_req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // post-condition: projection 비활성
        let body = body_json(
            app.clone()
                .oneshot(get("/api/projection/scene"))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(
            body["is_active"], false,
            "scene projection must clear after after_dialogue (SceneEnded)"
        );
        assert!(
            body["active_focus_id"].is_null(),
            "active_focus_id must be null after scene ended"
        );
    }

    /// Relationship projection drift: after_dialogue 후 닫힌 값을 /api/projection/relationship이 반환.
    /// drift가 발생하면 `closeness=null`이 돌아오므로 즉시 실패한다.
    #[tokio::test]
    async fn relationship_projection_reflects_after_dialogue() {
        let (app, _state) = app_with_state();
        seed_two_npcs_and_rel(&app).await;

        // appraise → after_dialogue 파이프라인을 태워 RelationshipUpdated를 강제.
        let appraise_req = serde_json::json!({
            "npc_id": "mu_baek",
            "partner_id": "gyo_ryong",
            "situation": {
                "description": "짧은 조우",
                "event": {
                    "description": "조우",
                    "desirability_for_self": -0.2,
                    "other": null,
                    "prospect": null
                }
            }
        });
        let resp = app
            .clone()
            .oneshot(json_post("/api/appraise", appraise_req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let after_req = serde_json::json!({
            "npc_id": "mu_baek",
            "partner_id": "gyo_ryong",
            "significance": 0.6
        });
        let resp = app
            .clone()
            .oneshot(json_post("/api/after-dialogue", after_req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let after_body = body_json(resp).await;
        let expected_closeness = after_body["after"]["closeness"]
            .as_f64()
            .expect("after.closeness must exist") as f32;
        let expected_trust = after_body["after"]["trust"]
            .as_f64()
            .expect("after.trust must exist") as f32;

        let resp = app
            .clone()
            .oneshot(get("/api/projection/relationship/mu_baek/gyo_ryong"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        let proj_closeness = body["closeness"]
            .as_f64()
            .expect("projection closeness must be populated after RelationshipUpdated")
            as f32;
        let proj_trust = body["trust"]
            .as_f64()
            .expect("projection trust must be populated")
            as f32;

        assert!(
            (expected_closeness - proj_closeness).abs() < 1e-5,
            "closeness drift: after_dialogue={expected_closeness}, projection={proj_closeness}"
        );
        assert!(
            (expected_trust - proj_trust).abs() < 1e-5,
            "trust drift: after_dialogue={expected_trust}, projection={proj_trust}"
        );
    }

    /// AppState 필드를 직접 lock해도 같은 값이 보여야 — Arc 공유의 가장 강한 증거.
    /// HTTP 경로를 통하지 않는 raw access 검증.
    #[tokio::test]
    async fn appstate_projection_handle_shares_arc_with_dispatcher() {
        let (app, state) = app_with_state();
        seed_two_npcs_and_rel(&app).await;

        let appraise_req = serde_json::json!({
            "npc_id": "mu_baek",
            "partner_id": "gyo_ryong",
            "situation": {
                "description": "평범한 대화",
                "event": {
                    "description": "대화",
                    "desirability_for_self": 0.1,
                    "other": null,
                    "prospect": null
                }
            }
        });
        let resp = app
            .clone()
            .oneshot(json_post("/api/appraise", appraise_req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let http_mood = body_json(resp).await["mood"].as_f64().unwrap() as f32;

        let proj = state.emotion_projection.lock().unwrap();
        let direct_mood = proj
            .get_mood("mu_baek")
            .expect("Arc must be shared: projection must have mood after dispatch");
        assert!(
            (http_mood - direct_mood).abs() < 1e-5,
            "direct Arc access and HTTP appraise diverge: http={http_mood}, direct={direct_mood}"
        );
    }

    /// 6.4 (선택): GET /api/projection/trace/{cid} 가 실제 묶음을 반환한다.
    ///
    /// /api/appraise 한 번 → EventStore에서 cid 추출 → trace 엔드포인트로 같은 묶음 회수.
    /// HTTP 200 + JSON 형태(correlation_id/event_count/events) + 묶음 무결성을 검증한다.
    #[tokio::test]
    async fn trace_endpoint_returns_correlation_bundle() {
        let (app, state) = app_with_state();
        seed_two_npcs_and_rel(&app).await;

        let appraise_req = serde_json::json!({
            "npc_id": "mu_baek",
            "partner_id": "gyo_ryong",
            "situation": {
                "description": "교룡 등장",
                "event": {
                    "description": "사건",
                    "desirability_for_self": -0.5,
                    "other": null,
                    "prospect": null
                }
            }
        });
        let resp = app
            .clone()
            .oneshot(json_post("/api/appraise", appraise_req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // EventStore에서 직접 cid 추출 (HTTP 응답이 cid를 노출하지 않으므로).
        let store = state.shared_dispatcher.event_store();
        let all_events = store.get_all_events();
        assert!(!all_events.is_empty(), "appraise should have emitted events");
        let cid = all_events
            .last()
            .expect("at least one event")
            .metadata
            .correlation_id
            .expect("dispatch_v2 must attach correlation_id");
        let expected_count = store.get_events_by_correlation(cid).len();
        assert!(expected_count > 0, "expected non-empty bundle for cid={cid}");

        // /api/projection/trace/{cid} 호출
        let resp = app
            .clone()
            .oneshot(get(&format!("/api/projection/trace/{cid}")))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "trace endpoint must return 200");
        let body = body_json(resp).await;

        assert_eq!(body["correlation_id"].as_u64().unwrap(), cid);
        assert_eq!(body["event_count"].as_u64().unwrap() as usize, expected_count);
        let events = body["events"].as_array().expect("events must be array");
        assert_eq!(events.len(), expected_count);
        for ev in events {
            assert_eq!(
                ev["metadata"]["correlation_id"].as_u64().unwrap(),
                cid,
                "every event in the bundle must carry the same cid"
            );
        }

        // 존재하지 않는 cid는 빈 묶음으로 200 응답.
        let resp = app
            .clone()
            .oneshot(get("/api/projection/trace/999999999"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["event_count"].as_u64().unwrap(), 0);
        assert!(body["events"].as_array().unwrap().is_empty());
    }
}
