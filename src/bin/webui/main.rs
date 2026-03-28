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

#[tokio::main]
async fn main() {
    // tracing 초기화
    let collector = trace_collector::AppraisalCollector::new();
    tracing_subscriber::registry()
        .with(collector.clone())
        .init();

    let state = AppState::new(collector);

    // 정적 파일 경로
    let static_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/bin/webui/static");

    let app = Router::new()
        // --- API ---
        .route("/api/npcs", get(handlers::list_npcs).post(handlers::upsert_npc))
        .route("/api/npcs/{id}", delete(handlers::delete_npc))
        .route("/api/relationships", get(handlers::list_relationships).post(handlers::upsert_relationship))
        .route("/api/relationships/{owner}/{target}", delete(handlers::delete_relationship))
        .route("/api/objects", get(handlers::list_objects).post(handlers::upsert_object))
        .route("/api/objects/{id}", delete(handlers::delete_object))
        .route("/api/appraise", post(handlers::appraise))
        .route("/api/save", post(handlers::save_state))
        .route("/api/load", post(handlers::load_state))
        // --- 정적 파일 (SPA) ---
        .fallback_service(ServeDir::new(static_dir))
        .with_state(state);

    let addr = "127.0.0.1:3000";
    println!("NPC Mind WebUI: http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
