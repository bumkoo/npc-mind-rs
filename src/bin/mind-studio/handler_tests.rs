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
