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

    let json = body_json(resp).await;
    assert!(json["error"].as_str().unwrap().contains("nonexistent"));
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
