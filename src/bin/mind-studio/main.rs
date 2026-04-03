//! NPC Mind 웹 UI — axum 서버 진입점
//!
//! Claude와 Bekay가 함께 사용하는 NPC 심리 엔진 협업 도구.
//! - Claude: API (Invoke-WebRequest)로 NPC 생성, 감정 평가, 프롬프트 검증
//! - Bekay: 브라우저 UI에서 결과 확인, 슬라이더 조작, 실험

use axum::Router;
use axum::routing::{delete, get, post};
use tower_http::services::ServeDir;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod handlers;
mod mcp_server;
mod state;
mod trace_collector;

#[cfg(test)]
mod handler_tests;

use state::AppState;

/// API 라우터를 생성합니다 (테스트에서도 재사용).
fn build_api_router(state: AppState) -> Router {
    let router = Router::new()
        .route(
            "/api/npcs",
            get(handlers::list_npcs).post(handlers::upsert_npc),
        )
        .route("/api/npcs/{id}", delete(handlers::delete_npc))
        .route(
            "/api/relationships",
            get(handlers::list_relationships).post(handlers::upsert_relationship),
        )
        .route(
            "/api/relationships/{owner}/{target}",
            delete(handlers::delete_relationship),
        )
        .route(
            "/api/objects",
            get(handlers::list_objects).post(handlers::upsert_object),
        )
        .route("/api/objects/{id}", delete(handlers::delete_object))
        .route("/api/appraise", post(handlers::appraise))
        .route("/api/stimulus", post(handlers::stimulus))
        .route("/api/scene", post(handlers::scene))
        .route("/api/guide", post(handlers::guide))
        .route("/api/after-dialogue", post(handlers::after_dialogue))
        .route("/api/scenarios", get(handlers::list_scenarios))
        .route("/api/scenario-meta", get(handlers::get_scenario_meta))
        .route("/api/scene-info", get(handlers::get_scene_info))
        .route("/api/analyze-utterance", post(handlers::analyze_utterance))
        .route("/api/history", get(handlers::get_history))
        .route(
            "/api/situation",
            get(handlers::get_situation).put(handlers::put_situation),
        )
        .route("/api/save", post(handlers::save_state))
        .route("/api/save-dir", get(handlers::save_dir))
        .route("/api/load", post(handlers::load_state))
        .route("/api/load-result", post(handlers::load_result));

    // chat feature 활성 시 대화 테스트 엔드포인트 추가
    #[cfg(feature = "chat")]
    let router = router
        .route("/api/chat/start", post(handlers::chat_handlers::chat_start))
        .route(
            "/api/chat/message",
            post(handlers::chat_handlers::chat_message),
        )
        .route(
            "/api/chat/message/stream",
            post(handlers::chat_handlers::chat_message_stream),
        )
        .route("/api/chat/end", post(handlers::chat_handlers::chat_end));

    // MCP 라우터 병합
    let router = router.merge(mcp_server::mcp_router());

    router.with_state(state)
}

/// embed feature 활성 시 PadAnalyzer 초기화
#[cfg(feature = "embed")]
fn init_analyzer() -> Option<npc_mind::domain::pad::PadAnalyzer> {
    use npc_mind::adapter::ort_embedder::OrtEmbedder;
    use npc_mind::adapter::toml_anchor_source::TomlAnchorSource;
    use npc_mind::domain::pad::PadAnalyzer;
    use npc_mind::domain::pad_anchors::builtin_anchor_toml;

    let model_dir =
        std::env::var("NPC_MIND_MODEL_DIR").unwrap_or_else(|_| "../models/bge-m3".to_string());
    let model_path = std::path::Path::new(&model_dir).join("model_quantized.onnx");
    let tokenizer_path = std::path::Path::new(&model_dir).join("tokenizer.json");

    let anchor_lang = std::env::var("NPC_MIND_ANCHOR_LANG").unwrap_or_else(|_| "ko".to_string());
    let anchor_toml = builtin_anchor_toml(&anchor_lang).unwrap_or_else(|| {
        eprintln!("빌트인 앵커 없음 (lang={anchor_lang}), ko 폴백");
        builtin_anchor_toml("ko").unwrap()
    });
    let source = TomlAnchorSource::from_content(anchor_toml)
        .with_cache_path(format!("locales/anchors/{anchor_lang}.embeddings.json"));

    match OrtEmbedder::new(&model_path, &tokenizer_path) {
        Ok(embedder) => match PadAnalyzer::new(Box::new(embedder), &source) {
            Ok(analyzer) => {
                println!("PAD Analyzer: 초기화 완료 (embed 활성, lang={anchor_lang})");
                Some(analyzer)
            }
            Err(e) => {
                eprintln!("PAD Analyzer 앵커 초기화 실패: {e:?}");
                None
            }
        },
        Err(e) => {
            eprintln!("OrtEmbedder 초기화 실패: {e:?}");
            None
        }
    }
}

#[cfg(not(feature = "embed"))]
fn init_analyzer() -> Option<npc_mind::domain::pad::PadAnalyzer> {
    None
}

#[tokio::main]
async fn main() {
    // tracing 초기화
    let collector = trace_collector::AppraisalCollector::new();
    tracing_subscriber::registry()
        .with(collector.clone())
        .init();

    let analyzer = init_analyzer();
    let mut state = AppState::new(collector, analyzer);

    // MCP 서버 초기화
    let mcp_server = mcp_server::create_mcp_server();
    state = state.with_mcp(mcp_server);

    // chat feature 활성 시 RigChatAdapter 초기화
    #[cfg(feature = "chat")]
    {
        let chat_url = std::env::var("NPC_MIND_CHAT_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8081/v1".to_string());
        let chat_model = std::env::var("NPC_MIND_CHAT_MODEL")
            .unwrap_or_else(|_| "local-model".to_string());
        let adapter = std::sync::Arc::new(npc_mind::adapter::rig_chat::RigChatAdapter::new(&chat_url, &chat_model));
        state = state.with_chat(adapter.clone()).with_llm_info(adapter);
        println!("Chat Agent: 초기화 완료 (url={chat_url}, model={chat_model})");
    }

    // 정적 파일 경로
    let static_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bin/mind-studio/static");

    let app = build_api_router(state)
        .fallback_service(ServeDir::new(static_dir));

    let port = std::env::var("MIND_STUDIO_PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("127.0.0.1:{port}");
    println!("NPC Mind Studio: http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Mind Studio 서버 포트 바인딩 실패 — MIND_STUDIO_PORT 환경변수를 확인하세요");
    axum::serve(listener, app)
        .await
        .expect("Mind Studio 서버 실행 중 오류 발생");
}
