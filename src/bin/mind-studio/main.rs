//! NPC Mind 웹 UI — axum 서버 진입점
//!
//! Claude와 Bekay가 함께 사용하는 NPC 심리 엔진 협업 도구.
//! - Claude: API (Invoke-WebRequest)로 NPC 생성, 감정 평가, 프롬프트 검증
//! - Bekay: 브라우저 UI에서 결과 확인, 슬라이더 조작, 실험

use axum::routing::{delete, get, post};
use axum::Router;
use tower_http::services::ServeDir;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod handlers;
mod state;
mod trace_collector;

use state::AppState;

/// embed feature 활성 시 PadAnalyzer 초기화
#[cfg(feature = "embed")]
fn init_analyzer() -> Option<npc_mind::domain::pad::PadAnalyzer> {
    use npc_mind::adapter::ort_embedder::OrtEmbedder;
    use npc_mind::domain::pad::PadAnalyzer;

    let model_dir = std::env::var("NPC_MIND_MODEL_DIR")
        .unwrap_or_else(|_| "../models/bge-m3".to_string());
    let model_path = std::path::Path::new(&model_dir).join("model_quantized.onnx");
    let tokenizer_path = std::path::Path::new(&model_dir).join("tokenizer.json");

    match OrtEmbedder::new(&model_path, &tokenizer_path) {
        Ok(embedder) => match PadAnalyzer::new(Box::new(embedder)) {
            Ok(analyzer) => {
                println!("PAD Analyzer: 초기화 완료 (embed 활성)");
                Some(analyzer)
            }
            Err(e) => { eprintln!("PAD Analyzer 앵커 초기화 실패: {e:?}"); None }
        }
        Err(e) => { eprintln!("OrtEmbedder 초기화 실패: {e:?}"); None }
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
    let state = AppState::new(collector, analyzer);

    // 정적 파일 경로
    let static_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/bin/mind-studio/static");

    let app = Router::new()
        // --- API ---
        .route("/api/npcs", get(handlers::list_npcs).post(handlers::upsert_npc))
        .route("/api/npcs/{id}", delete(handlers::delete_npc))
        .route("/api/relationships", get(handlers::list_relationships).post(handlers::upsert_relationship))
        .route("/api/relationships/{owner}/{target}", delete(handlers::delete_relationship))
        .route("/api/objects", get(handlers::list_objects).post(handlers::upsert_object))
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
        .route("/api/situation", get(handlers::get_situation).put(handlers::put_situation))
        .route("/api/save", post(handlers::save_state))
        .route("/api/load", post(handlers::load_state))
        // --- 정적 파일 (SPA) ---
        .fallback_service(ServeDir::new(static_dir))
        .with_state(state);

    let addr = "127.0.0.1:3000";
    println!("NPC Mind Studio: http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
