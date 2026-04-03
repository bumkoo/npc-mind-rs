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

/// 테스트용 AppState 생성 (embed 없음)
fn test_state() -> AppState {
    AppState::new(AppraisalCollector::new(), None)
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

    // 2. Message 엔드포인트 확인
    let req = serde_json::json!({"jsonrpc": "2.0", "method": "test", "id": 1});
    let resp = app.clone().oneshot(json_post("/mcp/message", req)).await.unwrap();
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
    let req = serde_json::json!({
        "method": "tools/call",
        "params": {
            "name": "list_npcs",
            "arguments": {}
        },
        "id": 1
    });
    let res = crate::mcp_server::handle_mcp_tool_call(&state, req).await.unwrap();
    assert_eq!(res.as_array().unwrap().len(), 1);
    assert_eq!(res[0]["id"], "mu_baek");

    // 2. get_npc_llm_config 도구 테스트
    let req = serde_json::json!({
        "method": "tools/call",
        "params": {
            "name": "get_npc_llm_config",
            "arguments": {"npc_id": "mu_baek"}
        },
        "id": 2
    });
    let res = crate::mcp_server::handle_mcp_tool_call(&state, req).await.unwrap();
    assert!(res["temperature"].as_f64().is_some());
    assert!(res["top_p"].as_f64().is_some());

    // 3. appraise 도구 테스트
    let req = serde_json::json!({
        "method": "tools/call",
        "params": {
            "name": "appraise",
            "arguments": {
                "npc_id": "mu_baek",
                "partner_id": "player",
                "situation": {
                    "description": "테스트 상황",
                    "event": {
                        "description": "선물",
                        "desirability_for_self": 0.8
                    }
                }
            }
        },
        "id": 3
    });
    let res = crate::mcp_server::handle_mcp_tool_call(&state, req).await.unwrap();
    assert!(res["mood"].as_f64().unwrap() > 0.0);
}
