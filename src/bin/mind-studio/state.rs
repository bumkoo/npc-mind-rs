//! 웹 UI 서버 상태 — NPC, 관계, 오브젝트 레지스트리

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{Mutex, RwLock};

use crate::events::StateEvent;
use crate::trace_collector::AppraisalCollector;
use npc_mind::application::command::dispatcher::CommandDispatcher;
use npc_mind::application::command::policies::{
    EmotionPolicy, GuidePolicy, InformationPolicy, RelationshipPolicy, ScenePolicy,
    StimulusPolicy, WorldOverlayPolicy,
};
use npc_mind::application::command::{
    EmotionProjectionHandler, RelationshipProjectionHandler, SceneProjectionHandler,
};
use npc_mind::application::director::{Director, Spawner};
#[cfg(feature = "chat")]
use npc_mind::application::reflection_service::ReflectionRunner;
#[cfg(feature = "chat")]
use npc_mind::domain::reflection::TurnSnapshot;
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::application::projection::{
    EmotionProjection, RelationshipProjection, SceneProjection,
};
use futures::future::BoxFuture;
use npc_mind::domain::emotion::EmotionState;
use npc_mind::domain::emotion::SceneFocus;
use npc_mind::InMemoryRepository;
#[cfg(feature = "listener_perspective")]
use npc_mind::domain::listener_perspective::ListenerPerspectiveConverter;
use npc_mind::ports::{LlmModelInfo, UtteranceAnalyzer};
#[cfg(feature = "embed")]
use npc_mind::ports::{MemoryStore, RumorStore};
#[cfg(feature = "embed")]
use npc_mind::lore::{LoreStore, Manifest};
#[cfg(feature = "embed")]
use npc_mind::worldbuilding::WorldRepository;

/// 서버 공유 상태
#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<RwLock<StateInner>>,
    pub collector: AppraisalCollector,
    /// 대사 → PAD 분석기 (embed feature 활성 시에만 Some)
    pub analyzer: Option<Arc<Mutex<dyn UtteranceAnalyzer + Send>>>,
    /// 화자 PAD → 청자 PAD 변환기 (Phase 7, listener_perspective feature)
    /// 주입 시 `resolve_pad`가 analyzer 임베딩을 재사용해 변환을 적용한다.
    /// 변환 실패는 화자 PAD fallback (silent failure 방지를 위해 warn 로그).
    #[cfg(feature = "listener_perspective")]
    pub converter: Option<Arc<dyn ListenerPerspectiveConverter>>,
    /// 연기 가이드 포맷터 (런타임 교체 가능 — set_prompt_override로 TOML 오버라이드 적용)
    pub formatter: Arc<RwLock<Arc<dyn npc_mind::ports::GuideFormatter>>>,
    /// 현재 적용 중인 TOML 오버라이드 (None이면 기본 빌트인)
    pub locale_overrides: Arc<RwLock<Option<String>>>,
    /// 실시간 상태 변경 이벤트 브로드캐스트 채널
    pub event_tx: tokio::sync::broadcast::Sender<StateEvent>,
    /// LLM 대화 오케스트레이터 (chat feature 활성 시에만 Some)
    #[cfg(feature = "chat")]
    pub chat: Option<Arc<dyn npc_mind::ports::ConversationPort>>,
    /// LLM 메타데이터 제공자
    #[cfg(feature = "chat")]
    pub llm_info: Option<Arc<dyn npc_mind::ports::LlmInfoProvider>>,
    /// LLM 모델 런타임 재감지 (dialogue_start 시점에 호출)
    #[cfg(feature = "chat")]
    pub llm_detector: Option<Arc<dyn npc_mind::ports::LlmModelDetector>>,
    /// llama-server 모니터링 (health, slots, metrics)
    #[cfg(feature = "chat")]
    pub llm_monitor: Option<Arc<dyn npc_mind::ports::InferenceServerMonitor>>,
    /// Phase 1.5 Mind Architecture (`relationships.md` v0.7 §6) — Scene Boundary
    /// Reflection runner. chat feature 활성 + main.rs에서 부착 시에만 Some.
    ///
    /// 부착되면 `perform_chat_end`(또는 `perform_after_dialogue`)가 turn buffer를
    /// 회수하여 `service.reflect()` 호출 → `Command::EndDialogue { reflection }`로
    /// dispatch. 미부착이면 reflection=None → legacy 호환 (기존 무조건 RelationshipUpdated).
    #[cfg(feature = "chat")]
    pub reflection_service: Option<Arc<dyn ReflectionRunner>>,
    /// MCP 서버 인스턴스 (정적 타입)
    pub mcp_server: Option<Arc<crate::mcp_server::MindMcpService>>,
    /// chat feature 비활성 시 컴파일 호환용
    #[cfg(not(feature = "chat"))]
    #[allow(dead_code)]
    pub chat: Option<()>,

    /// B4 Session 3 Option B-Mini: v2 dispatch 경로 shadow Director.
    ///
    /// 기존 `AppState.inner` (v1 경로)와 **완전히 분리된** Repository를 소유한다.
    /// `POST /api/v2/scenes/*` 엔드포인트가 이 Director를 통해 다중 Scene lifecycle을
    /// 제공. Mind Studio UI는 여전히 v1 경로를 사용하며, v2는 외부 호출자(Claude API
    /// 등) 전용 proof-of-concept 노출.
    ///
    /// NPC/Relationship은 Director 내부 Repository에 별도 등록해야 동작 — 이를 위해
    /// `POST /api/v2/npcs` / `POST /api/v2/relationships` 헬퍼 엔드포인트 제공 또는
    /// Director.sync_from_app_state (Session 4+) 로 스냅샷 동기화 가능.
    pub director_v2: Arc<Director<InMemoryRepository>>,

    /// B5.2 (3/3): request 간 재사용되는 CommandDispatcher.
    ///
    /// 내부 `Arc<Mutex<InMemoryRepository>>`를 소유하며, UI CRUD 시
    /// `AppState::rebuild_repo_from_inner()`로 repo를 재구성한다. Dispatch 시점에는
    /// 이미 fresh한 상태이므로 request마다 snapshot_to_repo를 수행하지 않는다.
    ///
    /// **알려진 한계**: 내부 `InMemoryEventStore`가 프로세스 수명 동안 모든 이벤트를
    /// 누적한다. 이전 ephemeral dispatcher 패턴은 request마다 store를 drop했으나
    /// 공유 dispatcher는 그렇지 않다. Mind Studio는 dev tool이고 주기적으로 재시작
    /// 되므로 실용상 문제없음 — 장기 실행 시 메모리 사용량 증가와 `next_sequence`
    /// O(N) scan 부하가 누적됨을 염두에 둘 것. Phase 8+ persistent store 도입 시 해소.
    pub shared_dispatcher: Arc<CommandDispatcher<InMemoryRepository>>,

    /// Step E1: Memory 저장소. `embed` feature 활성 시 `NPC_MIND_MEMORY_DB` 환경변수
    /// 경로(또는 `:memory:`)로 초기화되며 `shared_dispatcher`에 `with_memory_full`로
    /// 부착된다. REST 핸들러가 직접 조회할 때도 사용.
    #[cfg(feature = "embed")]
    pub memory_store: Arc<dyn MemoryStore>,

    /// Step E1: Rumor 저장소. `embed` feature 활성 시에만 존재하며 같은 DB 파일을 공유.
    #[cfg(feature = "embed")]
    pub rumor_store: Arc<dyn RumorStore>,

    /// Phase 0 Lore RAG: 장르 원전 임베딩 검색 저장소.
    /// `NPC_MIND_LORE_DB`(기본 `data/corpus/lore.sqlite`) 파일이 존재할 때만 Some.
    /// 부재 시 `search_lore` 등 MCP 도구는 "lore index 미구성" 에러를 반환한다.
    #[cfg(feature = "embed")]
    pub lore_store: Option<Arc<dyn LoreStore>>,
    /// Phase 0 Lore RAG: `data/corpus/manifest.toml` 캐시. `list_corpora` 도구가 사용.
    #[cfg(feature = "embed")]
    pub lore_manifest: Option<Arc<Manifest>>,

    /// Phase 1 Worldbuilding: Group SQLite 저장소.
    /// `NPC_MIND_WORLD_DB` 환경변수로 경로 지정. 부재 시 `list_groups`/`get_group`/
    /// `search_groups` MCP·REST 도구가 "world index 미구성" 에러를 반환.
    /// Phase 2+에서 Person/Place 등 도메인 추가에 따라 같은 DB가 확장된다.
    #[cfg(feature = "embed")]
    pub world_store: Option<Arc<dyn WorldRepository>>,

    // ---- Read Side — Projection 공유 핸들 ----
    //
    // 아래 3개 필드는 `shared_dispatcher`의 Inline Projection Handler와
    // **동일한 `Arc<Mutex<T>>`**를 공유한다. `/api/projection/*` Query 핸들러가
    // 이 Arc를 lock하여 읽으며, dispatch_v2 Inline phase가 같은 Arc를 통해 write
    // 한다. Mutex 종류는 `*ProjectionHandler::from_shared`의 시그니처에 맞춰
    // `std::sync::Mutex`를 사용한다 (tokio::sync::Mutex 아님).
    //
    // director_v2는 별개 Projection을 내부 소유하며 이 필드들과 분리됨 —
    // `/api/projection/*`는 shared_dispatcher 경로만 반영 (task 명세 §10).
    /// NPC별 mood / dominant / snapshot 뷰 (EmotionAppraised·StimulusApplied·EmotionCleared 구독).
    pub emotion_projection: Arc<StdMutex<EmotionProjection>>,
    /// (owner, target) 쌍의 closeness/trust/power 뷰 (RelationshipUpdated 구독).
    pub relationship_projection: Arc<StdMutex<RelationshipProjection>>,
    /// 활성 Scene 상태 뷰 (SceneStarted·BeatTransitioned·SceneEnded 구독).
    pub scene_projection: Arc<StdMutex<SceneProjection>>,
}

/// `CommandDispatcher::with_default_handlers`를 **수동으로 재현**하되 Projection은
/// 외부에서 주입받은 Arc로 교체하는 헬퍼.
///
/// - Transactional 7종: Scene/Emotion/Stimulus/Guide/Relationship/Information/WorldOverlay
///   (library core의 `with_default_handlers`와 동일한 순서·구성)
/// - Inline 3종: `from_shared(arc)`로 주입 — dispatcher가 업데이트하는 projection과
///   `/api/projection/*` 쿼리가 읽는 projection이 **같은 Arc**가 되도록 보장한다.
///
/// `with_default_handlers`를 그대로 쓰면 내부 `::new()`로 별도 Arc가 생성되어 중복
/// 등록이 되므로 본 헬퍼로 치환한다. Library core는 무변경.
fn build_shared_dispatcher(
    repo: Arc<StdMutex<InMemoryRepository>>,
    store: Arc<InMemoryEventStore>,
    bus: Arc<EventBus>,
    emotion_projection: Arc<StdMutex<EmotionProjection>>,
    relationship_projection: Arc<StdMutex<RelationshipProjection>>,
    scene_projection: Arc<StdMutex<SceneProjection>>,
) -> CommandDispatcher<InMemoryRepository> {
    CommandDispatcher::new(repo, store, bus)
        .register_transactional(Arc::new(ScenePolicy::new()))
        .register_transactional(Arc::new(EmotionPolicy::new()))
        .register_transactional(Arc::new(StimulusPolicy::new()))
        .register_transactional(Arc::new(GuidePolicy::new()))
        .register_transactional(Arc::new(RelationshipPolicy::new()))
        .register_transactional(Arc::new(InformationPolicy::new()))
        .register_transactional(Arc::new(WorldOverlayPolicy::new()))
        .register_inline(Arc::new(EmotionProjectionHandler::from_shared(
            emotion_projection,
        )))
        .register_inline(Arc::new(RelationshipProjectionHandler::from_shared(
            relationship_projection,
        )))
        .register_inline(Arc::new(SceneProjectionHandler::from_shared(
            scene_projection,
        )))
}

impl AppState {
    pub fn new(
        collector: AppraisalCollector,
        analyzer: Option<impl UtteranceAnalyzer + Send + 'static>,
    ) -> Self {
        let (event_tx, _) = tokio::sync::broadcast::channel(64);

        // Director v2 초기화 — 빈 Repository로 시작 (v1 AppState.inner와 분리)
        //
        // B4 Session 4: Director가 async가 되었으므로 Mutex 래퍼는 제거.
        // Spawner는 Mind Studio가 tokio 런타임 위에서 돌아가므로 tokio::spawn 클로저.
        let director_v2 = {
            let repo = InMemoryRepository::new();
            let repo_arc = Arc::new(StdMutex::new(repo));
            let store = Arc::new(InMemoryEventStore::new());
            let bus = Arc::new(EventBus::new());
            let dispatcher = CommandDispatcher::new(repo_arc, store, bus).with_default_handlers();
            let spawner: Arc<dyn Spawner> = Arc::new(|fut: BoxFuture<'static, ()>| {
                tokio::spawn(fut);
            });
            Arc::new(Director::new(dispatcher, spawner))
        };

        // Read Side Activation: Projection Arc를 **선행 생성**하고
        // dispatcher의 Inline handler와 AppState 필드가 같은 Arc를 공유하도록 주입한다.
        // 이 Arc들은 `with_default_handlers()`가 내부 생성하는 Projection과는 별개이므로,
        // default helper를 그대로 쓰면 중복 등록이 된다. 따라서 아래에서는
        // `with_default_handlers()` 대신 policy 7종을 수동 등록하고,
        // projection 3종만 `from_shared(arc)`로 주입한다 (library core 무변경).
        let emotion_projection: Arc<StdMutex<EmotionProjection>> =
            Arc::new(StdMutex::new(EmotionProjection::new()));
        let relationship_projection: Arc<StdMutex<RelationshipProjection>> =
            Arc::new(StdMutex::new(RelationshipProjection::new()));
        let scene_projection: Arc<StdMutex<SceneProjection>> =
            Arc::new(StdMutex::new(SceneProjection::new()));

        // Step E1: embed feature 활성 시 MemoryStore/RumorStore를 먼저 초기화한 뒤
        // 공유 dispatcher에 with_memory_full + with_rumor로 부착한다.
        // `NPC_MIND_MEMORY_DB` 환경변수가 있으면 해당 파일, 없으면 in-memory SQLite.
        // 두 store가 같은 DB 파일/인스턴스를 공유 — SqliteRumorStore가 init_schema에서
        // rumors/rumor_hops/rumor_distortions 테이블을 선제 생성하도록 설계됨(§7.4).
        #[cfg(feature = "embed")]
        let (shared_dispatcher, memory_store, rumor_store) = {
            use npc_mind::adapter::sqlite_memory::SqliteMemoryStore;
            use npc_mind::adapter::sqlite_rumor::SqliteRumorStore;

            let db_path = std::env::var("NPC_MIND_MEMORY_DB").ok();
            let mem: Arc<dyn MemoryStore> = match db_path.as_deref() {
                Some(path) => Arc::new(
                    SqliteMemoryStore::new(path)
                        .expect("MemoryStore 초기화 실패 — NPC_MIND_MEMORY_DB 경로 확인"),
                ),
                None => Arc::new(
                    SqliteMemoryStore::in_memory().expect("MemoryStore in-memory 초기화 실패"),
                ),
            };
            let rum: Arc<dyn RumorStore> = match db_path.as_deref() {
                Some(path) => Arc::new(
                    SqliteRumorStore::new(path)
                        .expect("RumorStore 초기화 실패 — NPC_MIND_MEMORY_DB 경로 확인"),
                ),
                None => Arc::new(
                    SqliteRumorStore::in_memory().expect("RumorStore in-memory 초기화 실패"),
                ),
            };

            let repo = InMemoryRepository::new();
            let repo_arc = Arc::new(StdMutex::new(repo));
            let store = Arc::new(InMemoryEventStore::new());
            let bus = Arc::new(EventBus::new());
            let dispatcher = Arc::new(
                build_shared_dispatcher(
                    repo_arc,
                    store,
                    bus,
                    emotion_projection.clone(),
                    relationship_projection.clone(),
                    scene_projection.clone(),
                )
                .with_memory_full(mem.clone())
                .with_rumor(mem.clone(), rum.clone()),
            );
            (dispatcher, mem, rum)
        };

        // embed 미활성 — memory/rumor 빌더 없이 같은 구성.
        #[cfg(not(feature = "embed"))]
        let shared_dispatcher = {
            let repo = InMemoryRepository::new();
            let repo_arc = Arc::new(StdMutex::new(repo));
            let store = Arc::new(InMemoryEventStore::new());
            let bus = Arc::new(EventBus::new());
            Arc::new(build_shared_dispatcher(
                repo_arc,
                store,
                bus,
                emotion_projection.clone(),
                relationship_projection.clone(),
                scene_projection.clone(),
            ))
        };

        Self {
            inner: Arc::new(RwLock::new(StateInner::default())),
            collector,
            analyzer: analyzer.map(|a| Arc::new(Mutex::new(a)) as Arc<Mutex<dyn UtteranceAnalyzer + Send>>),
            #[cfg(feature = "listener_perspective")]
            converter: None,
            formatter: Arc::new(RwLock::new(Arc::new(npc_mind::presentation::korean::KoreanFormatter::new()) as Arc<dyn npc_mind::ports::GuideFormatter>)),
            locale_overrides: Arc::new(RwLock::new(None)),
            event_tx,
            chat: None,
            #[cfg(feature = "chat")]
            llm_info: None,
            #[cfg(feature = "chat")]
            llm_detector: None,
            #[cfg(feature = "chat")]
            llm_monitor: None,
            #[cfg(feature = "chat")]
            reflection_service: None,
            mcp_server: None,
            director_v2,
            shared_dispatcher,
            #[cfg(feature = "embed")]
            memory_store,
            #[cfg(feature = "embed")]
            rumor_store,
            #[cfg(feature = "embed")]
            lore_store: None,
            #[cfg(feature = "embed")]
            lore_manifest: None,
            #[cfg(feature = "embed")]
            world_store: None,
            emotion_projection,
            relationship_projection,
            scene_projection,
        }
    }

    /// B5.2 (3/3): StateInner의 도메인 데이터를 공유 repo로 재구성.
    ///
    /// UI CRUD·scenario load 직후 호출해 dispatch 경로가 보는 repo 상태를 최신화.
    /// Reset+rebuild 방식 — NPC/관계/감정/Scene 전체를 한 번에 교체해 drift 불가능.
    pub async fn rebuild_repo_from_inner(&self) {
        use npc_mind::ports::{EmotionStore, SceneStore};
        let inner = self.inner.read().await;
        let mut repo = self.shared_dispatcher.repository_guard();
        *repo = InMemoryRepository::new();
        for profile in inner.npcs.values() {
            repo.add_npc(profile.to_npc());
        }
        for rel in inner.relationships.values() {
            repo.add_relationship(rel.to_relationship());
        }
        for obj in inner.objects.values() {
            repo.add_object(obj.id.clone(), obj.description.clone());
        }
        for (id, state) in &inner.emotions {
            repo.save_emotion_state(id, state.clone());
        }
        if let (Some(n), Some(p)) = (
            inner.scene_npc_id.as_ref(),
            inner.scene_partner_id.as_ref(),
        ) {
            let mut scene = npc_mind::domain::emotion::Scene::new(
                n.clone(),
                p.clone(),
                inner.scene_focuses.clone(),
            );
            if let Some(ref id) = inner.active_focus_id {
                scene.set_active_focus(id.clone());
            }
            repo.save_scene(scene);
        }
    }

    /// 상태 변경 이벤트를 브로드캐스트한다. 수신자가 없으면 무시.
    pub fn emit(&self, event: StateEvent) {
        let _ = self.event_tx.send(event);
    }

    /// MCP 서버 인스턴스를 설정한다.
    pub fn with_mcp(mut self, server: Arc<crate::mcp_server::MindMcpService>) -> Self {
        self.mcp_server = Some(server);
        self
    }

    /// LLM 대화 오케스트레이터를 설정한다 (chat feature 활성 시).
    #[cfg(feature = "chat")]
    pub fn with_chat(mut self, chat: Arc<dyn npc_mind::ports::ConversationPort>) -> Self {
        self.chat = Some(chat);
        self
    }

    /// LLM 메타데이터 제공자를 설정한다.
    #[cfg(feature = "chat")]
    pub fn with_llm_info(mut self, llm_info: Arc<dyn npc_mind::ports::LlmInfoProvider>) -> Self {
        self.llm_info = Some(llm_info);
        self
    }

    /// LLM 모델 런타임 재감지기를 설정한다.
    #[cfg(feature = "chat")]
    pub fn with_llm_detector(mut self, detector: Arc<dyn npc_mind::ports::LlmModelDetector>) -> Self {
        self.llm_detector = Some(detector);
        self
    }

    /// llama-server 모니터를 설정한다.
    #[cfg(feature = "chat")]
    pub fn with_llm_monitor(mut self, monitor: Arc<dyn npc_mind::ports::InferenceServerMonitor>) -> Self {
        self.llm_monitor = Some(monitor);
        self
    }

    /// Phase 1.5 — ReflectionService 부착 (chat feature).
    ///
    /// 부착 후 `perform_chat_end` 또는 `perform_after_dialogue` 경로가 turn buffer를
    /// 회수해 `service.reflect()`를 호출하고 결과를 `Command::EndDialogue { reflection }`로
    /// dispatch. 미부착 시 `reflection: None` → 기존 무조건 RelationshipUpdated 동작.
    #[cfg(feature = "chat")]
    pub fn with_reflection(mut self, service: Arc<dyn ReflectionRunner>) -> Self {
        self.reflection_service = Some(service);
        self
    }

    /// 청자 관점 PAD 변환기를 설정한다 (Phase 7, listener_perspective feature).
    #[cfg(feature = "listener_perspective")]
    pub fn with_converter(mut self, converter: Arc<dyn ListenerPerspectiveConverter>) -> Self {
        self.converter = Some(converter);
        self
    }

    /// Phase 0 Lore RAG 저장소·매니페스트를 부착 (embed feature).
    /// `search_lore`/`list_corpora`/`get_chunk` MCP 도구가 이 핸들로 동작한다.
    #[cfg(feature = "embed")]
    pub fn with_lore(
        mut self,
        store: Arc<dyn LoreStore>,
        manifest: Arc<Manifest>,
    ) -> Self {
        self.lore_store = Some(store);
        self.lore_manifest = Some(manifest);
        self
    }

    /// Phase 1 Worldbuilding 저장소를 부착 (embed feature).
    /// `list_groups`/`get_group`/`search_groups` MCP·REST 도구가 이 핸들로 동작한다.
    /// Phase 2부터는 `list_persons`/`get_person`/`search_persons`도 같은 store에서.
    #[cfg(feature = "embed")]
    pub fn with_world(mut self, store: Arc<dyn WorldRepository>) -> Self {
        self.world_store = Some(store);
        self
    }

    /// Phase 2 — world_store에 등록된 active/player Person을 inner.npcs에 자동 등록.
    ///
    /// 호출 시점: 부착 직후(`with_world` 호출 다음) 또는 시나리오 로드 후.
    ///
    /// **덮어쓰기 정책 (Code review #5)**:
    /// - 같은 NpcId가 inner.npcs에 이미 있으면 `NpcProfile` 전체를 SQLite의 값으로 **완전히 교체**.
    ///   사용자가 UI에서 description·HEXACO facet을 수동 편집한 적이 있어도 그 편집은 사라진다.
    ///   현재 호출 site는 mind-studio 부팅 시 1회뿐이라 실해는 발생하지 않으나, 향후 런타임
    ///   재동기화 endpoint(`task-phase2-followup-runtime-sync.md`)가 추가될 경우 작가의 in-memory
    ///   편집을 보호하려면 (a) 이 메서드를 그대로 사용하되 작가에게 "수동 편집은 sync 후 사라진다"
    ///   고 명시하거나 (b) merge 정책을 도입(기존 description/facets 유지) — 둘 중 하나를 명확히
    ///   결정해야 한다. 본 메서드는 (a)를 채택.
    /// - emotion_state · 관계 · scene · memory는 **보존**된다. 이는 `inner.emotions` /
    ///   `inner.relationships` / `inner.scene_*` 가 별도 필드이며 본 메서드는 `inner.npcs`만
    ///   교체하기 때문이다. `rebuild_repo_from_inner()`은 inner의 다른 필드를 그대로 다시
    ///   shared repo에 반영한다.
    ///
    /// 반환: 등록된 인물 수 (필터링 후). world_store가 부재하면 0.
    #[cfg(feature = "embed")]
    pub async fn sync_world_persons_into_repo(&self) -> Result<usize, npc_mind::domain::world::WorldError> {
        let store = match self.world_store.as_ref() {
            Some(s) => s.clone(),
            None => return Ok(0),
        };
        // PersonFilter::default()로 전체 조회 후 mind 적격성으로 필터.
        // (kind 필터를 SQL 단에 두면 active/player를 두 번 호출해야 하므로 메모리 필터가 단순.)
        let persons = store.list_persons(npc_mind::domain::world::PersonFilter::default())?;
        let mut count = 0;
        let mut inner = self.inner.write().await;
        for p in &persons {
            if !p.is_mind_eligible() {
                continue;
            }
            let profile = NpcProfile::from_person(p);
            inner.npcs.insert(profile.id.clone(), profile);
            count += 1;
        }
        drop(inner);
        if count > 0 {
            self.rebuild_repo_from_inner().await;
        }
        Ok(count)
    }
}

/// 파일 포맷 식별자
pub const FORMAT_SCENARIO: &str = "mind-studio/scenario";
pub const FORMAT_RESULT: &str = "mind-studio/result";

/// 내부 상태 (RwLock으로 보호)
#[derive(Default, Serialize, Deserialize)]
pub struct StateInner {
    /// 파일 포맷 식별자 ("mind-studio/scenario" | "mind-studio/result")
    #[serde(default)]
    pub format: String,
    /// NPC 프로필 레지스트리 (key: npc_id)
    pub npcs: HashMap<String, NpcProfile>,
    /// 관계 레지스트리 (key: "owner_id:target_id")
    pub relationships: HashMap<String, RelationshipData>,
    /// 오브젝트 레지스트리 (key: object_id)
    pub objects: HashMap<String, ObjectEntry>,
    /// 현재 감정 상태 (key: npc_id) — 직렬화 제외
    #[serde(skip)]
    pub emotions: HashMap<String, EmotionState>,
    /// 시나리오 메타데이터
    pub scenario: ScenarioMeta,
    /// 턴별 기록 (장면 설정 + 감정 평가 + 프롬프트)
    #[serde(default)]
    pub turn_history: Vec<TurnRecord>,
    /// 현재 상황 설정 패널 상태 (프론트엔드 폼 값 보존용)
    #[serde(default)]
    pub current_situation: Option<serde_json::Value>,
    /// 테스트 결과 분석 보고서 (마크다운 포맷)
    #[serde(default)]
    pub test_report: String,
    /// Scene 정보 (시나리오 JSON에 저장됨)
    #[serde(default)]
    pub scene: Option<serde_json::Value>,
    /// 시나리오 JSON에 선언된 memory/rumor 시드 (Step E3.2).
    ///
    /// `#[serde(flatten)]`로 시나리오 JSON 최상위에 `initial_rumors` / `world_knowledge` /
    /// `faction_knowledge` / `family_facts` 4개 필드가 평면 배치된다. 파싱만 수행하고
    /// store 주입은 `load_state` 핸들러가 `apply_scenario_seeds`로 분리해 처리한다.
    #[serde(default, flatten)]
    pub scenario_seeds: npc_mind::application::scenario_seeds::ScenarioSeeds,
    /// Scene Focus 옵션 목록 (런타임 — 직렬화 제외)
    #[serde(skip)]
    pub scene_focuses: Vec<SceneFocus>,
    /// 현재 활성 Focus ID (런타임)
    #[serde(skip)]
    pub active_focus_id: Option<String>,
    /// 현재 Scene의 NPC ID (런타임)
    #[serde(skip)]
    pub scene_npc_id: Option<String>,
    /// 현재 Scene의 대화 상대 ID (런타임)
    #[serde(skip)]
    pub scene_partner_id: Option<String>,
    /// 로드한 파일 경로 (런타임 — 결과 저장 시 자동 경로 계산용)
    #[serde(skip)]
    pub loaded_path: Option<String>,
    /// 시나리오 수정 여부 (런타임 — 저장 분기 판단용)
    #[serde(skip)]
    pub scenario_modified: bool,
    /// 테스트 스크립트 커서 — 현재 Beat의 test_script에서 다음에 사용할 대사 인덱스.
    /// Beat 전환 시 0으로 리셋된다.
    #[serde(skip)]
    pub script_cursor: usize,
    /// Phase 1.5 Mind Architecture: 세션별 TurnSnapshot 누적 버퍼.
    ///
    /// `reflection_service` 부착 시에만 채워진다. `process_chat_turn_result`가
    /// 각 chat 턴마다 푸시, `perform_chat_end`가 회수하여 `service.reflect()`에 전달.
    /// `#[serde(skip)]`로 직렬화 제외 (런타임 전용).
    ///
    /// DialogueOrchestrator의 `turn_buffers`와 동일한 책임이지만 Mind Studio는
    /// DialogueOrchestrator를 사용하지 않고 ad-hoc 경로(state.chat + domain_sync)를
    /// 직접 호출하므로 본 필드가 별도 필요. spec §3.2 결정 (사) 그대로.
    #[cfg(feature = "chat")]
    #[serde(skip)]
    pub turn_buffers: HashMap<String, Vec<TurnSnapshot>>,
}

/// 턴별 기록 — 장면 설정, 감정 결과, 프롬프트를 JSON으로 보존
#[derive(Clone, Serialize, Deserialize)]
pub struct TurnRecord {
    /// 턴 라벨 (예: "Turn 1: 유령 공포")
    pub label: String,
    /// 파이프라인 종류 ("appraise" | "stimulus" | "after_dialogue")
    pub action: String,
    /// 요청 파라미터 (SituationInput 등)
    pub request: serde_json::Value,
    /// 응답 결과 (감정, 프롬프트, trace 등)
    pub response: serde_json::Value,
    /// 첫 턴에 기록되는 모델 스냅샷
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub llm_model: Option<LlmModelInfo>,
}

/// 시나리오 메타데이터
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct ScenarioMeta {
    pub name: String,
    pub description: String,
    /// 평가 노트 (Claude가 작성). 작가가 손으로 쓰는 시나리오 JSON에서 흔히 누락되므로
    /// `#[serde(default)]`로 빈 Vec를 허용한다 (Memory Step E3.2 회귀 가드 — `notes` 누락
    /// 시 `StateInner::load_from_file`이 500을 떨구는 문제 방지).
    #[serde(default)]
    pub notes: Vec<String>,
}

/// NPC 프로필 (HEXACO 24 facet + 메타)
#[derive(Clone, Serialize, Deserialize)]
pub struct NpcProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    // H: 정직-겸손성
    #[serde(default)]
    pub sincerity: f32,
    #[serde(default)]
    pub fairness: f32,
    #[serde(default)]
    pub greed_avoidance: f32,
    #[serde(default)]
    pub modesty: f32,
    // E: 정서성
    #[serde(default)]
    pub fearfulness: f32,
    #[serde(default)]
    pub anxiety: f32,
    #[serde(default)]
    pub dependence: f32,
    #[serde(default)]
    pub sentimentality: f32,
    // X: 외향성
    #[serde(default)]
    pub social_self_esteem: f32,
    #[serde(default)]
    pub social_boldness: f32,
    #[serde(default)]
    pub sociability: f32,
    #[serde(default)]
    pub liveliness: f32,
    // A: 원만성
    #[serde(default)]
    pub forgiveness: f32,
    #[serde(default)]
    pub gentleness: f32,
    #[serde(default)]
    pub flexibility: f32,
    #[serde(default)]
    pub patience: f32,
    // C: 성실성
    #[serde(default)]
    pub organization: f32,
    #[serde(default)]
    pub diligence: f32,
    #[serde(default)]
    pub perfectionism: f32,
    #[serde(default)]
    pub prudence: f32,
    // O: 경험개방성
    #[serde(default)]
    pub aesthetic_appreciation: f32,
    #[serde(default)]
    pub inquisitiveness: f32,
    #[serde(default)]
    pub creativity: f32,
    #[serde(default)]
    pub unconventionality: f32,
}

/// 관계 데이터 (Stage 3 — 4축 ±100 raw, `RelationshipUpdatedPayload`와 정합)
///
/// 필드 순서 — trust → affinity → respect → wariness.
/// 값 contract — trust/affinity/respect: ±100, wariness: 0~100.
///
/// **v0.6 시나리오 JSON 호환 (자동 ×100 변환)**:
/// 커스텀 `Deserialize` impl이 v0.6 키(`closeness` 또는 `power`) 존재를 감지하면
/// **자동으로 `trust × 100`, `closeness × 100 → affinity`** 변환을 적용한다 (Stage 3 리뷰 H1).
/// `power`는 폐기 (B-D4) — 값 무시. save-roundtrip 시 v0.7 4-field로 저장되며 값도 정확.
/// v0.6 감지 시 `tracing::warn!`로 표시 — Stage 4 마이그레이션 도구 실행 권장.
#[derive(Clone, Serialize, Deserialize)]
pub struct RelationshipData {
    pub owner_id: String,
    pub target_id: String,
    #[serde(default)]
    pub trust: f32,
    #[serde(default)]
    pub affinity: f32,
    #[serde(default)]
    pub respect: f32,
    #[serde(default)]
    pub wariness: f32,
}

impl RelationshipData {
    /// 레지스트리 키 생성 ("owner:target")
    pub fn key(&self) -> String {
        format!("{}:{}", self.owner_id, self.target_id)
    }
}

/// 오브젝트 등록 정보
#[derive(Clone, Serialize, Deserialize)]
pub struct ObjectEntry {
    pub id: String,
    pub description: String,
    /// 카테고리 (사물/장소/NPC특성 등 — 선택적)
    pub category: Option<String>,
}

// ---------------------------------------------------------------------------
// 도메인 변환
// ---------------------------------------------------------------------------

use npc_mind::domain::personality::{Npc, NpcBuilder, Score};
use npc_mind::domain::relationship::{Relationship, RelationshipBuilder};

impl NpcProfile {
    /// Phase 2: worldbuilding `Person` → `NpcProfile` 변환.
    /// HEXACO 6 dim 값을 4 facet에 spread (`worldbuilding::mind_sync`와 동일 정책).
    /// `kind in {active, player}`만 호출 — 호출자가 사전 필터링.
    #[cfg(feature = "embed")]
    pub fn from_person(person: &npc_mind::domain::world::Person) -> Self {
        let h = &person.hexaco;
        let hh = h.honesty_humility.value();
        let em = h.emotionality.value();
        let ex = h.extraversion.value();
        let ag = h.agreeableness.value();
        let co = h.conscientiousness.value();
        let op = h.openness.value();
        Self {
            id: person.id.as_str().to_string(),
            name: person.name.clone(),
            description: person.summary.clone(),
            // H
            sincerity: hh,
            fairness: hh,
            greed_avoidance: hh,
            modesty: hh,
            // E
            fearfulness: em,
            anxiety: em,
            dependence: em,
            sentimentality: em,
            // X
            social_self_esteem: ex,
            social_boldness: ex,
            sociability: ex,
            liveliness: ex,
            // A
            forgiveness: ag,
            gentleness: ag,
            flexibility: ag,
            patience: ag,
            // C
            organization: co,
            diligence: co,
            perfectionism: co,
            prudence: co,
            // O
            aesthetic_appreciation: op,
            inquisitiveness: op,
            creativity: op,
            unconventionality: op,
        }
    }

    /// NPC 도메인 객체로 변환
    pub fn to_npc(&self) -> Npc {
        let s = |v: f32| Score::clamped(v);
        NpcBuilder::new(&self.id, &self.name)
            .description(&self.description)
            .honesty_humility(|h| {
                h.sincerity = s(self.sincerity);
                h.fairness = s(self.fairness);
                h.greed_avoidance = s(self.greed_avoidance);
                h.modesty = s(self.modesty);
            })
            .emotionality(|e| {
                e.fearfulness = s(self.fearfulness);
                e.anxiety = s(self.anxiety);
                e.dependence = s(self.dependence);
                e.sentimentality = s(self.sentimentality);
            })
            .extraversion(|x| {
                x.social_self_esteem = s(self.social_self_esteem);
                x.social_boldness = s(self.social_boldness);
                x.sociability = s(self.sociability);
                x.liveliness = s(self.liveliness);
            })
            .agreeableness(|a| {
                a.forgiveness = s(self.forgiveness);
                a.gentleness = s(self.gentleness);
                a.flexibility = s(self.flexibility);
                a.patience = s(self.patience);
            })
            .conscientiousness(|c| {
                c.organization = s(self.organization);
                c.diligence = s(self.diligence);
                c.perfectionism = s(self.perfectionism);
                c.prudence = s(self.prudence);
            })
            .openness(|o| {
                o.aesthetic_appreciation = s(self.aesthetic_appreciation);
                o.inquisitiveness = s(self.inquisitiveness);
                o.creativity = s(self.creativity);
                o.unconventionality = s(self.unconventionality);
            })
            .build()
    }

    /// 성격 기반 LLM 파라미터 유도 (도메인 로직 위임)
    pub fn derive_llm_parameters(&self) -> (f32, f32) {
        self.to_npc().personality().derive_llm_parameters()
    }
}

impl RelationshipData {
    /// Relationship 도메인 객체로 변환.
    ///
    /// Stage 3 — 필드가 ±100 raw 스케일. 산술 변환 없음. v0.6 시나리오 JSON은
    /// `Deserialize` impl에서 자동 ×100 변환되므로 본 메서드 진입 시점에는 항상 ±100.
    pub fn to_relationship(&self) -> Relationship {
        use npc_mind::domain::relationship::{AxisScore, WarinessScore};
        RelationshipBuilder::new(&self.owner_id, &self.target_id)
            .trust(AxisScore::new(self.trust))
            .affinity(AxisScore::new(self.affinity))
            .respect(AxisScore::new(self.respect))
            .wariness(WarinessScore::new(self.wariness))
            .build()
    }
}

impl StateInner {
    /// 관계 조회 (양방향 — owner:target 또는 target:owner)
    pub fn find_relationship(&self, id_a: &str, id_b: &str) -> Option<&RelationshipData> {
        let key1 = format!("{id_a}:{id_b}");
        let key2 = format!("{id_b}:{id_a}");
        self.relationships
            .get(&key1)
            .or_else(|| self.relationships.get(&key2))
    }

    /// JSON 파일로 저장 (format 자동 설정)
    /// `as_scenario` = true → 시나리오로 저장 (turn_history 제외, format=scenario)
    /// `as_scenario` = false → 결과로 저장 (turn_history 포함, format=result)
    pub fn save_to_file(&self, path: &std::path::Path, as_scenario: bool) -> Result<(), String> {
        let mut snapshot = serde_json::to_value(self).map_err(|e| e.to_string())?;
        if as_scenario {
            snapshot["format"] = serde_json::Value::String(FORMAT_SCENARIO.to_string());
            // 시나리오 저장 시 turn_history 제외
            if let serde_json::Value::Object(ref mut map) = snapshot {
                map.remove("turn_history");
            }
        } else {
            snapshot["format"] = serde_json::Value::String(FORMAT_RESULT.to_string());
        }
        let json = serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?;
        // 부모 디렉토리가 없으면 생성
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    /// 테스트 보고서(마크다운)를 파일로 저장합니다.
    /// `test_report` 필드의 내용을 그대로 .md 파일로 기록합니다.
    pub fn save_report_to_file(&self, path: &std::path::Path) -> Result<(), String> {
        if self.test_report.is_empty() {
            return Err("test_report가 비어있습니다. update_test_report를 먼저 호출하세요.".into());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, &self.test_report).map_err(|e| e.to_string())
    }

    /// JSON 파일에서 로드
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, String> {
        let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&json).map_err(|e| e.to_string())
    }
}
