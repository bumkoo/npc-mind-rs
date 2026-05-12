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

mod event_bridge;
mod events;
mod handlers;
mod mcp_server;
mod repository;
mod state;
mod studio_service;
mod domain_sync;
mod trace_collector;

#[cfg(test)]
mod handler_tests;

#[cfg(all(test, feature = "embed", feature = "listener_perspective"))]
mod init_tests;

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

    // listener_perspective feature 활성 시 EmbeddedConverter 초기화 (옵셔널 주입)
    // 모델/패턴 파일 로드 실패해도 Mind Studio는 동작 — 단지 변환이 비활성화됨
    #[cfg(all(feature = "embed", feature = "listener_perspective"))]
    {
        if let Some(converter) = init_listener_perspective_converter() {
            state = state.with_converter(std::sync::Arc::new(converter));
            tracing::info!("Listener-perspective Converter 초기화 완료");
        }
    }

    // chat feature 활성 시 RigChatAdapter 초기화 (MCP 서버보다 먼저 — clone 시 chat 포함)
    // 모델명은 dialogue_start 시점에 /v1/models에서 자동 감지한다.
    #[cfg(feature = "chat")]
    {
        use std::sync::Arc;
        let chat_url = std::env::var("NPC_MIND_CHAT_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8081/v1".to_string());

        let adapter = npc_mind::adapter::rig_chat::RigChatAdapter::new(&chat_url, "pending");
        tracing::info!("LLM 어댑터 생성 완료 (모델은 dialogue_start 시 자동 감지): url={}", chat_url);
        let arc_adapter = Arc::new(adapter);
        state = state.with_chat(arc_adapter.clone());
        state = state.with_llm_info(arc_adapter.clone());
        state = state.with_llm_detector(arc_adapter.clone());
        state = state.with_llm_monitor(arc_adapter);

        // Phase 1.5 Mind Architecture (relationships.md v0.7 §6) — Reflection 어댑터.
        //
        // dialogue 세션과 *별도 instance*로 RigChatAdapter를 한 번 더 생성.
        // 같은 모델·같은 LLM 서버를 가리키지만 KV 캐시·세션 풀이 분리되어
        // dialogue turn과 reflection 분석이 서로의 ChatSession을 깨지 않는다.
        // (`tests/phase1_real_llm_test.rs`가 같은 패턴으로 검증된 구성.)
        //
        // 부착 실패해도 Mind Studio는 정상 동작 — reflection 미부착 → legacy 경로.
        let reflection_adapter = Arc::new(
            npc_mind::adapter::rig_chat::RigChatAdapter::new(&chat_url, "pending"),
        );
        let reflection_port = Arc::new(
            npc_mind::adapter::reflection_via_chat::ConversationBackedReflectionPort::new(
                reflection_adapter,
            ),
        );
        let reflection_builder: Arc<dyn npc_mind::application::reflection_service::ReflectionPromptBuilder> =
            Arc::new(npc_mind::application::reflection_service::DefaultReflectionPromptBuilder);
        let reflection_service =
            npc_mind::application::reflection_service::ReflectionService::new(
                reflection_port,
                reflection_builder,
            );
        state = state.with_reflection(
            Arc::new(reflection_service)
                as Arc<dyn npc_mind::application::reflection_service::ReflectionRunner>,
        );
        tracing::info!(
            "Reflection 어댑터 초기화 완료 (Phase 1.5 — separate session pool, same url={})",
            chat_url
        );
    }

    // Phase 0 Lore RAG: SQLite 인덱스가 있으면 부착 (embed feature 한정).
    // 인덱스가 없거나 manifest를 못 읽어도 Mind Studio는 정상 시작 — search_lore 등은
    // "lore index 미구성" 메시지를 반환하게 된다.
    #[cfg(feature = "embed")]
    {
        use std::sync::Arc;
        let manifest_path = std::env::var_os("NPC_MIND_LORE_MANIFEST")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("data/corpus/manifest.toml"));
        let db_path = std::env::var_os("NPC_MIND_LORE_DB")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("data/corpus/lore.sqlite"));

        match (
            npc_mind::lore::Manifest::load(&manifest_path),
            db_path.exists().then(|| {
                npc_mind::lore::SqliteLoreStore::new(db_path.to_string_lossy().as_ref())
            }),
        ) {
            (Ok(manifest), Some(Ok(store))) => {
                state = state.with_lore(
                    Arc::new(store) as Arc<dyn npc_mind::lore::LoreStore>,
                    Arc::new(manifest),
                );
                tracing::info!(
                    "Lore RAG 부착 완료: manifest={} db={}",
                    manifest_path.display(),
                    db_path.display()
                );
            }
            (Ok(_), None) => {
                tracing::warn!(
                    "Lore RAG: SQLite 파일 없음 ({}). lore-ingest를 먼저 실행하세요.",
                    db_path.display()
                );
            }
            (Err(e), _) => {
                tracing::warn!("Lore RAG: manifest 로드 실패 ({}): {}", manifest_path.display(), e);
            }
            (_, Some(Err(e))) => {
                tracing::warn!("Lore RAG: SqliteLoreStore 초기화 실패: {:?}", e);
            }
        }
    }

    // Phase 1 Worldbuilding: NPC_MIND_WORLD_DB가 가리키는 SQLite가 있으면 부착.
    // 부재 시 list_groups 등 MCP/REST 도구는 "world index 미구성" 에러를 반환.
    #[cfg(feature = "embed")]
    {
        use std::sync::Arc;
        if let Some(path) = std::env::var_os("NPC_MIND_WORLD_DB") {
            let path = std::path::PathBuf::from(path);
            if path.is_file() {
                match npc_mind::adapter::sqlite_world::SqliteWorldStore::new(
                    path.to_string_lossy().as_ref(),
                ) {
                    Ok(store) => {
                        state = state.with_world(
                            Arc::new(store) as Arc<dyn npc_mind::worldbuilding::WorldRepository>,
                        );
                        tracing::info!("World store 부착 완료: {}", path.display());
                        // Phase 2 — active/player Person을 인메모리 MindRepository에 자동 등록.
                        // dialogue_start 등 v2 dispatch 경로가 npc-NN ID를 즉시 인식하게 한다.
                        match state.sync_world_persons_into_repo().await {
                            Ok(0) => {
                                tracing::info!(
                                    "Phase 2: world_store에 등록된 active/player 인물이 없음"
                                );
                            }
                            Ok(n) => {
                                tracing::info!(
                                    "Phase 2: {n}명의 Person을 mind repository에 자동 등록 완료"
                                );
                            }
                            Err(e) => {
                                tracing::warn!("Phase 2: Person 자동 등록 실패: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "World store 초기화 실패 ({}): {:?}",
                            path.display(),
                            e
                        );
                    }
                }
            } else {
                tracing::warn!(
                    "NPC_MIND_WORLD_DB={} 파일 없음 — world-load --project <id> 실행 필요",
                    path.display()
                );
            }
        }
    }

    // MCP 서버 초기화 (chat이 설정된 state를 clone)
    let mcp_server = mcp_server::create_mcp_server(state.clone());
    state = state.with_mcp(mcp_server);

    // Phase 1.6 — EventBus → SSE Bridge 시작.
    //
    // shared_dispatcher의 EventBus를 구독해 도메인 이벤트를 `StateEvent` SSE로
    // 자동 변환·재방출. 이로써 studio_service / domain_sync / handlers에 산재한
    // manual `state.emit(StateEvent::XXX)` 호출이 *도메인 사실에서 도출 가능한 9개*
    // 만큼 제거됨. UI-only 이벤트 (HistoryChanged 등)는 그대로 유지.
    //
    // 보너스: Director 경로(`/api/v2/scenes/*`)도 *shared_dispatcher 사용 시*
    // 자동 SSE 발행. 현재 director_v2는 별도 dispatcher라 본 bridge가 보지 못함 —
    // 향후 통합 작업 후보.
    {
        use std::sync::Arc;
        let bridge = Arc::new(event_bridge::EventBridge::new(state.event_tx.clone()));
        let bus = state.shared_dispatcher.event_bus().clone();
        let event_store = state.shared_dispatcher.event_store().clone();
        tokio::spawn(bridge.run(bus, event_store));
        tracing::info!("EventBus → SSE Bridge 시작");
    }

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
        .route("/api/scenario-seeds", get(handlers::scenario::get_scenario_seeds))
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
        .route("/api/load-result", post(handlers::scenario::load_result))
        // 실시간 상태 변경 이벤트 스트림
        .route("/api/events", get(handlers::events::sse_events))
        // Read Side Activation — Projection 기반 조회 (StateInner 조회와 병렬).
        // dispatch_v2 Inline phase가 `AppState`의 공유 Projection Arc를 업데이트하고,
        // 아래 세 엔드포인트가 같은 Arc를 읽어 반환한다 (drift 불가능).
        .route("/api/projection/emotion/{npc_id}", get(handlers::query::get_emotion))
        .route(
            "/api/projection/relationship/{owner}/{target}",
            get(handlers::query::get_relationship),
        )
        .route("/api/projection/scene", get(handlers::query::get_scene))
        // correlation_id로 묶인 이벤트 사슬 조회 (한 dispatch_v2 호출의 인과 그래프).
        // shared_dispatcher의 EventStore만 본다 — /api/v2/* Director 경로는 별도.
        .route(
            "/api/projection/trace/{correlation_id}",
            get(handlers::query::get_trace),
        )
        // B4 Session 3 Option B-Mini: v2 dispatch shadow path (Director 경유)
        // 기존 v1 경로(/api/scene, /api/appraise 등)와 분리된 별도 Repository
        .route("/api/v2/scenes", get(handlers::v2_scenes::list_active_scenes))
        .route("/api/v2/scenes/start", post(handlers::v2_scenes::start_scene))
        .route("/api/v2/scenes/dispatch", post(handlers::v2_scenes::dispatch_to_scene))
        .route(
            "/api/v2/scenes/{npc_id}/{partner_id}",
            delete(handlers::v2_scenes::end_scene),
        )
        .route("/api/v2/npcs", post(handlers::v2_scenes::upsert_npc_v2))
        .route(
            "/api/v2/relationships",
            post(handlers::v2_scenes::upsert_relationship_v2),
        )
        .route("/api/v2/scene-ids", get(handlers::v2_scenes::list_all_scene_ids));

    // chat feature 활성 시 대화 테스트 + LLM 모니터링 엔드포인트 추가
    #[cfg(feature = "chat")]
    let router = router
        .route("/api/chat/start", post(handlers::chat::chat_start))
        .route("/api/chat/message", post(handlers::chat::chat_message))
        .route("/api/chat/message/stream", post(handlers::chat::chat_message_stream))
        .route("/api/chat/end", post(handlers::chat::chat_end))
        .route("/api/llm/status", get(handlers::llm::llm_status))
        .route("/api/llm/health", get(handlers::llm::llm_health))
        .route("/api/llm/slots", get(handlers::llm::llm_slots))
        .route("/api/llm/metrics", get(handlers::llm::llm_metrics));

    // Step E1 — Memory / Rumor / World 엔드포인트 (embed feature 활성 시).
    // shared_dispatcher가 with_memory_full + with_rumor로 배선된 상태에서만 의미가 있다.
    #[cfg(feature = "embed")]
    let router = router
        .route("/api/memory/search", get(handlers::memory::search))
        .route("/api/memory/by-npc/{id}", get(handlers::memory::by_npc))
        .route("/api/memory/by-topic/{topic}", get(handlers::memory::by_topic))
        .route("/api/memory/canonical/{topic}", get(handlers::memory::canonical))
        .route("/api/memory/entries", post(handlers::memory::create_entry))
        .route("/api/memory/tell", post(handlers::memory::tell))
        .route("/api/world/apply-event", post(handlers::world::apply_event))
        .route("/api/rumors", get(handlers::rumor::list))
        .route("/api/rumors/seed", post(handlers::rumor::seed))
        .route("/api/rumors/{id}/spread", post(handlers::rumor::spread))
        // Phase 1 Worldbuilding — Group 조회. world_store 미부착 시 501 NotImplemented.
        // 순서 주의: `search`는 path-param보다 먼저 등록해야 axum 라우터가 우선 매치한다.
        .route("/api/world/groups", get(handlers::world_groups::list_groups))
        .route(
            "/api/world/groups/search",
            get(handlers::world_groups::search_groups),
        )
        .route("/api/world/groups/{id}", get(handlers::world_groups::get_group))
        // Phase 2 Worldbuilding — Person 조회 + 런타임 sync.
        .route("/api/world/persons", get(handlers::world_persons::list_persons))
        .route(
            "/api/world/persons/search",
            get(handlers::world_persons::search_persons),
        )
        // sync는 path param보다 먼저 등록 — axum 매칭 우선순위.
        .route(
            "/api/world/persons/sync",
            post(handlers::world_persons::sync_persons),
        )
        .route("/api/world/persons/{id}", get(handlers::world_persons::get_person))
        // Phase 3 Worldbuilding — Place 조회.
        .route("/api/world/places", get(handlers::world_places::list_places))
        .route(
            "/api/world/places/search",
            get(handlers::world_places::search_places),
        )
        .route("/api/world/places/{id}", get(handlers::world_places::get_place))
        // Phase 4 Worldbuilding — Atlas 조회 (첫 관계 도메인).
        // search는 path param {id}보다 먼저 등록 — axum 매칭 우선순위.
        .route("/api/world/atlases", get(handlers::world_atlases::list_atlases))
        .route(
            "/api/world/atlases/search",
            get(handlers::world_atlases::search_atlases),
        )
        .route("/api/world/atlases/{id}", get(handlers::world_atlases::get_atlas))
        // Phase 5a Worldbuilding — Event 조회 (두 번째 인스턴스 도메인).
        // search는 path param {id}보다 먼저 등록 — axum 매칭 우선순위.
        .route("/api/world/events", get(handlers::world_events::list_events))
        .route(
            "/api/world/events/search",
            get(handlers::world_events::search_events),
        )
        .route("/api/world/events/{id}", get(handlers::world_events::get_event))
        // Phase 5b Worldbuilding — Era 조회 (세 번째 인스턴스 도메인).
        // search는 path param {id}보다 먼저 등록 — axum 매칭 우선순위.
        .route("/api/world/eras", get(handlers::world_eras::list_eras))
        .route(
            "/api/world/eras/search",
            get(handlers::world_eras::search_eras),
        )
        .route("/api/world/eras/{id}", get(handlers::world_eras::get_era))
        // Phase 5b 체크포인트 2 Worldbuilding — Timeline 조회 (두 번째 관계 도메인).
        .route(
            "/api/world/timelines",
            get(handlers::world_timelines::list_timelines),
        )
        .route(
            "/api/world/timelines/search",
            get(handlers::world_timelines::search_timelines),
        )
        .route(
            "/api/world/timelines/{id}",
            get(handlers::world_timelines::get_timeline),
        );

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
    let source = FileAnchorSource::from_content(anchor_toml, AnchorFormat::Toml);

    PadAnalyzer::new(Box::new(embedder), &source).ok()
}

#[cfg(not(feature = "embed"))]
fn init_analyzer() -> Option<npc_mind::domain::pad::PadAnalyzer> {
    None
}

/// listener_perspective + embed feature 활성 시 EmbeddedConverter 초기화
///
/// 데이터 파일 경로는 `NPC_MIND_LP_DATA_DIR` 환경변수 또는 default `data/listener_perspective`.
/// 임베딩 모델은 `NPC_MIND_MODEL_DIR` 재사용 (PadAnalyzer와 동일).
/// 초기화 실패 시 `None`을 반환하여 Mind Studio는 변환 없이 정상 동작한다.
#[cfg(all(feature = "embed", feature = "listener_perspective"))]
fn init_listener_perspective_converter()
-> Option<npc_mind::domain::listener_perspective::EmbeddedConverter> {
    use npc_mind::adapter::ort_embedder::OrtEmbedder;

    let model_dir =
        std::env::var("NPC_MIND_MODEL_DIR").unwrap_or_else(|_| "../models/bge-m3".to_string());
    let model_path = std::path::Path::new(&model_dir).join("model_quantized.onnx");
    let tokenizer_path = std::path::Path::new(&model_dir).join("tokenizer.json");

    let data_dir = std::env::var("NPC_MIND_LP_DATA_DIR")
        .unwrap_or_else(|_| "data/listener_perspective".to_string());

    let mut embedder = match OrtEmbedder::new(&model_path, &tokenizer_path) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(
                "Listener-perspective Converter 초기화 스킵 — embedder 실패: {:?}",
                e
            );
            return None;
        }
    };

    build_converter_from_data_dir(&mut embedder, std::path::Path::new(&data_dir))
}

/// 순수 헬퍼: 외부 주입 embedder + data 디렉토리로 EmbeddedConverter 빌드.
///
/// `init_listener_perspective_converter`의 env/모델 의존부와 분리되어
/// 테스트 가능(Mock embedder + tempfile). 패턴/프로토타입 로드 실패 시
/// `tracing::warn!` 후 `None` 반환 (graceful degradation).
#[cfg(all(feature = "embed", feature = "listener_perspective"))]
fn build_converter_from_data_dir(
    embedder: &mut dyn npc_mind::ports::TextEmbedder,
    data_root: &std::path::Path,
) -> Option<npc_mind::domain::listener_perspective::EmbeddedConverter> {
    use npc_mind::domain::listener_perspective::EmbeddedConverter;

    EmbeddedConverter::from_paths(
        embedder,
        data_root.join("prefilter/patterns.toml"),
        data_root.join("prototypes/sign_keep.toml"),
        data_root.join("prototypes/sign_invert.toml"),
        data_root.join("prototypes/magnitude_weak.toml"),
        data_root.join("prototypes/magnitude_normal.toml"),
        data_root.join("prototypes/magnitude_strong.toml"),
    )
    .map_err(|e| {
        tracing::warn!(
            "Listener-perspective Converter 초기화 스킵 — 패턴/프로토타입 로드 실패: {:?}",
            e
        );
    })
    .ok()
}
