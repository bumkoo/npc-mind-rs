//! NPC Mind 웹 UI — axum 서버 진입점
//!
//! Claude와 Bekay가 함께 사용하는 NPC 심리 엔진 협업 도구.
//! - Claude: API (Invoke-WebRequest)로 NPC 생성, 감정 평가, 프롬프트 검증
//! - Bekay: 브라우저 UI에서 결과 확인, 슬라이더 조작, 실험

use axum::Router;
use axum::routing::{delete, get, post};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod handlers;
mod mcp_server;
mod repository;
mod state;
mod studio_service;
mod trace_collector;

#[cfg(test)]
mod handler_tests;

use crate::state::AppState;
use crate::trace_collector::AppraisalCollector;

#[tokio::main]
async fn main() {
    // 1. 로깅 초기화 및 Trace 수집기 등록
    let collector = AppraisalCollector::new();
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "npc_mind_studio=debug,npc_mind=trace,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(collector.clone())
        .init();

    let analyzer = init_analyzer();
    let mut state = AppState::new(collector, analyzer);

    // chat feature 활성 시 RigChatAdapter 초기화 (MCP 서버보다 먼저 — clone 시 chat 포함)
    #[cfg(feature = "chat")]
    {
        use std::sync::Arc;
        let chat_url = std::env::var("NPC_MIND_CHAT_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8081/v1".to_string());

        let adapter = match npc_mind::adapter::rig_chat::RigChatAdapter::connect(&chat_url).await {
            Ok(a) => {
                tracing::info!("LLM 모델 자동 감지 완료: url={}", chat_url);
                a
            }
            Err(e) => {
                tracing::warn!("모델 목록 조회 실패 ({}), 기본값으로 생성: url={}", e, chat_url);
                npc_mind::adapter::rig_chat::RigChatAdapter::new(&chat_url, "model")
            }
        };
        let arc_adapter = Arc::new(adapter);
        state = state.with_chat(arc_adapter.clone());
        state = state.with_llm_info(arc_adapter);
    }

    // MCP 서버 초기화 (chat이 설정된 state를 clone)
    let mcp_server = mcp_server::create_mcp_server(state.clone());
    state = state.with_mcp(mcp_server);

    // 2. 라우터 빌드
    let app = build_api_router(state);

    // 3. 서버 실행
    let host = std::env::var("MIND_STUDIO_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("MIND_STUDIO_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let addr_str = format!("{}:{}", host, port);
    let addr: SocketAddr = addr_str
        .parse()
        .expect("잘못된 서버 주소 형식입니다. MIND_STUDIO_HOST/PORT를 확인하세요.");

    tracing::info!("NPC Mind Studio 서버 시작: http://{}", addr);
    tracing::info!("Static UI: http://{}/", addr);
    tracing::info!("MCP SSE: http://{}/mcp/sse", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Mind Studio 서버 포트 바인딩 실패 — MIND_STUDIO_PORT 환경변수를 확인하세요");
    axum::serve(listener, app)
        .await
        .expect("Mind Studio 서버 실행 중 오류 발생");
}

fn build_api_router(state: AppState) -> Router {
    let router = Router::new()
        // 정적 파일 서빙 (UI)
        .fallback_service(ServeDir::new("src/bin/mind-studio/static"))
        // CORS 허용
        .layer(CorsLayer::permissive())
        // NPC
        .route("/api/npcs", get(handlers::npc::list_npcs).post(handlers::npc::upsert_npc))
        .route("/api/npcs/{id}", delete(handlers::npc::delete_npc))
        // Relationship
        .route("/api/relationships", get(handlers::relationship::list_relationships).post(handlers::relationship::upsert_relationship))
        .route("/api/relationships/{owner_id}/{target_id}", delete(handlers::relationship::delete_relationship))
        // Object
        .route("/api/objects", get(handlers::object::list_objects).post(handlers::object::upsert_object))
        .route("/api/objects/{id}", delete(handlers::object::delete_object))
        // Scenario & Action
        .route("/api/scenarios", get(handlers::scenario::list_scenarios))
        .route("/api/scenario-meta", get(handlers::scenario::get_scenario_meta))
        .route("/api/appraise", post(handlers::scenario::appraise))
        .route("/api/stimulus", post(handlers::scenario::stimulus))
        .route("/api/after-dialogue", post(handlers::scenario::after_dialogue))
        .route("/api/guide", post(handlers::scenario::guide))
        .route("/api/scene", post(handlers::scenario::scene))
        .route("/api/scene-info", get(handlers::scenario::get_scene_info))
        .route("/api/history", get(handlers::scenario::get_history))
        .route("/api/situation", get(handlers::scenario::get_situation).put(handlers::scenario::put_situation))
        .route("/api/test-report", get(handlers::scenario::get_test_report).put(handlers::scenario::put_test_report))
        .route("/api/analyze-utterance", post(handlers::scenario::analyze_utterance))
        // Persistence
        .route("/api/save", post(handlers::scenario::save_state))
        .route("/api/save-dir", get(handlers::scenario::save_dir))
        .route("/api/load", post(handlers::scenario::load_state))
        .route("/api/load-result", post(handlers::scenario::load_result));

    // chat feature 활성 시 대화 테스트 엔드포인트 추가
    #[cfg(feature = "chat")]
    let router = router
        .route("/api/chat/start", post(handlers::chat::chat_start))
        .route("/api/chat/message", post(handlers::chat::chat_message))
        .route("/api/chat/message/stream", post(handlers::chat::chat_message_stream))
        .route("/api/chat/end", post(handlers::chat::chat_end));

    // MCP 라우터 병합
    let router = router.merge(mcp_server::mcp_router());

    router.with_state(state)
}

/// embed feature 활성 시 PadAnalyzer 초기화
#[cfg(feature = "embed")]
fn init_analyzer() -> Option<npc_mind::domain::pad::PadAnalyzer> {
    use npc_mind::adapter::file_anchor_source::{AnchorFormat, FileAnchorSource};
    use npc_mind::adapter::ort_embedder::OrtEmbedder;
    use npc_mind::domain::pad::PadAnalyzer;
    use npc_mind::domain::pad_anchors::builtin_anchor_toml;

    let model_dir =
        std::env::var("NPC_MIND_MODEL_DIR").unwrap_or_else(|_| "../models/bge-m3".to_string());
    let model_path = std::path::Path::new(&model_dir).join("model_quantized.onnx");
    let tokenizer_path = std::path::Path::new(&model_dir).join("tokenizer.json");

    let anchor_lang = std::env::var("NPC_MIND_ANCHOR_LANG").unwrap_or_else(|_| "ko".to_string());
    let anchor_toml = builtin_anchor_toml(&anchor_lang).or_else(|| {
        eprintln!("빌트인 앵커 없음 (lang={anchor_lang}), ko 폴백");
        builtin_anchor_toml("ko")
    })?;

    let embedder = OrtEmbedder::new(&model_path, &tokenizer_path).ok()?;
    let source = FileAnchorSource::from_content(&anchor_toml, AnchorFormat::Toml);

    PadAnalyzer::new(Box::new(embedder), &source).ok()
}

#[cfg(not(feature = "embed"))]
fn init_analyzer() -> Option<npc_mind::domain::pad::PadAnalyzer> {
    None
}
