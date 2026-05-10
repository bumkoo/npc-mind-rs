# CLAUDE.md

NPC Mind Engine — HEXACO 성격이 OCC 감정을 생성하고, LLM이 연기할 수 있도록 가이드를 출력하는 Rust 라이브러리.

라이브러리 형태로 배포되며, `Director`/`CommandDispatcher::dispatch_v2`가 유일한 진입점입니다.
v0.3.0에서 v1 경로(`MindService`/`EventAwareMindService`/`Pipeline`/`CommandDispatcher::dispatch`/`shadow_v2`)는
전부 제거되었습니다.

## 기술 스택
- **Language:** Rust (Edition 2024)
- **Architecture:** Hexagonal + DDD + **EventBus(tokio broadcast)/CQRS/Event Sourcing** + **Multi-Handler** (Policy + Projector + Orchestrator) + **Unit of Work** (트랜잭션 원자적 커밋, `application/command/uow.rs`)
- **Libraries:** `serde`/`serde_json`, `serde_yaml`(Worldbuilding 마크다운 frontmatter, default-on), `thiserror`, `tokio/sync`+`tokio-stream`+`futures` (EventBus 내부 구현), `axum`(WebUI), `tracing`, `ort`(ONNX 임베딩), `rig-core`(LLM 대화 클라이언트), `rusqlite`+`sqlite-vec`(RAG·Worldbuilding·Rumor 저장소 [embed] — FTS5 + vec0 벡터 인덱스), `epub`(Lore RAG EPUB 파서 [embed]), `regex` (`listener_perspective` feature, default-on — 한국어 정규식 프리필터)
- **런타임 정책:** 코어는 `tokio::sync`의 `broadcast`만 내부 구현으로 사용. 공개 API는 `futures::Stream` 타입만 노출하므로 **호출자는 tokio를 deps에 추가할 필요 없음**(Bevy 등 임의 async 런타임에서 Stream 소비 가능). `chat`/`mind-studio` feature가 tokio `rt-multi-thread` 런타임을 추가 활성화. `embed` feature는 sqlite-vec이 순수 C 확장이라 tokio 런타임을 전이시키지 않는다.

## 빌드 & 테스트

```bash
cargo build                        # 기본 빌드
cargo build --features embed       # 임베딩 포함 (bge-m3-onnx-rust)
cargo build --features chat        # LLM 대화 오케스트레이터 포함 (rig-core)
cargo test                         # 기본 테스트
cargo test --features embed        # 전체 테스트 (임베딩 포함)
cargo test --features "embed listener_perspective"  # Phase 7 Converter 포함 엔드투엔드
cargo test --features listener_perspective --lib domain::listener_perspective  # 39 도메인 단위
cargo test --no-default-features --features chat --test dialogue_no_lp_passthrough  # Phase 7 Step 5: LP off 회귀 감시

# 개별 테스트는 tests/ 디렉토리 참조
# PAD 벤치마크(pad_benchmark_test 등)는 --features embed 필요

# Worldbuilding ingest CLI (Phase 1·2·3·4 — Group / Person / Place / Atlas)
cargo run --features embed --bin world-load -- --project chilguk-chunchu          # 마크다운 → projects/<id>/build/world.sqlite
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload # 기존 SQLite 삭제 후 재생성

# 프론트엔드 빌드/테스트 (mind-studio-ui/)
cd mind-studio-ui && npm install        # 최초 의존성 설치
cd mind-studio-ui && npm run build      # 프로덕션 빌드 → src/bin/mind-studio/static/
cd mind-studio-ui && npm test           # Vitest 테스트 실행
cd mind-studio-ui && npm run dev        # 개발 서버 (http://localhost:5173, proxy → Axum)

# mind-studio 실행 (빌드된 UI 포함)
cargo run --features mind-studio,chat,embed --bin npc-mind-studio  # http://127.0.0.1:3000
```

### 환경변수 (주요)

```
NPC_MIND_CHAT_URL=http://127.0.0.1:8081/v1   # 로컬 LLM 서버 [chat feature]
NPC_MIND_MODEL_DIR=../models/bge-m3          # ONNX 모델 [embed feature]
NPC_MIND_ANCHOR_LANG=ko                       # PAD 앵커 언어 [embed feature]
MIND_STUDIO_HOST=127.0.0.1                    # 서버 바인딩 [mind-studio feature]
MIND_STUDIO_PORT=3000                         # 서버 포트 [mind-studio feature]
NPC_MIND_LORE_DB=data/corpus/lore.sqlite      # Lore RAG SQLite 경로 [embed feature]
NPC_MIND_LORE_MANIFEST=data/corpus/manifest.toml   # Lore 매니페스트 경로 [embed feature]
NPC_MIND_MEMORY_DB=data/runtime/memory.sqlite # Memory/Rumor SQLite (미설정 시 in-memory) [embed]
NPC_MIND_WORLD_DB=projects/<id>/build/world.sqlite # Worldbuilding SQLite [embed, mind-studio]
NPC_MIND_WORLD_PROJECTS=./projects            # world-load CLI 프로젝트 루트 [embed]
NPC_MIND_LP_DATA_DIR=data/listener_perspective # listener_perspective 학습/벤치 데이터
```

### Lore RAG (Phase 0)

원전 EPUB 3권을 임베딩+검색용 SQLite로 인덱싱. 원본 EPUB과 `data/corpus/lore.sqlite`는 모두
gitignore (외부 자료 + 빌드 산출물). 다른 머신에서 작업할 때:

```bash
# 1. wuxia-core/docs/Chinese-Literature/ 아래 manifest.toml에 등록된 EPUB 3권 배치
# 2. ../models/bge-m3/ 아래 ONNX 모델 배치
# 3. cargo run --features embed --bin lore-ingest -- --all
```

청킹·필터 정책이 바뀌면 (예: Phase 0 cleanup으로 ToC 챕터 + 50자 미만 청크 noise 필터
추가) 기존 인덱스를 재생성하기 위해 `--reembed`를 1회 실행:

```bash
cargo run --features embed --bin lore-ingest -- --all --reembed
```

`data/corpus/lore.sqlite`가 존재하면 Mind Studio가 자동으로 부착하고
`search_lore` / `list_corpora` / `get_chunk` MCP 도구를 노출한다.

### Worldbuilding (Phase 0-5c 완료 — Phase 5 시리즈 종결, Phase 6+ 예정)

NPC Mind Studio = **장르 중립 worldbuilding 도구**. 무협(칠국춘추)은 첫 사용자
프로젝트일 뿐, 도구 자체는 wuxia·SF·판타지 어휘를 모른다. 9 인스턴스 도메인
(Place·Person·Group·Item·Skill·Knowledge·Lore·Event·Era) + 2 관계 도메인 (Atlas·Timeline)
구조이며 Phase별로 추가된다.

**3 계층 분리** (절대 섞지 말 것):

```
src/                = 도구 (장르 영원히 모름)
  domain/world/     — 9+2 도메인 (Phase 1·2·3·4·5a·5b 활성: Group/Person/Place/Atlas/Event/Era/Timeline; Item/Skill/Knowledge/Lore stub)
  worldbuilding/    — 마크다운(SoT) → 도메인 변환 + WorldRepository 포트
  adapter/sqlite_world.rs — SqliteWorldStore (FTS5 + 외래키 인덱스)
genres/<name>/      = 장르 패키지 (도구 위에 입히는 옷)
  ex: genres/wuxia/{forms, genre.toml, markdown_template}
projects/<name>/    = 사용자 산출물 (마크다운 SoT + 빌드 SQLite)
  ex: projects/chilguk-chunchu/{project.toml, world/{group,person,place,atlas,event,era,timeline}/*.md}
                  build/world.sqlite (gitignore — 빌드 산출물)
```

**`world-load` 빌드 CLI** (마크다운 → SQLite ingest, embed feature):

```bash
# 프로젝트 ingest (Phase 1·2·3·4·5a·5b 모두 한 번에)
cargo run --features embed --bin world-load -- --project chilguk-chunchu

# 옵션
#   --reload       기존 SQLite 삭제 후 재생성
#   --no-mind      Person → Npc 변환 dry-run 끔 (--features 미사용 시)
#   --project <id> 필수 — projects/<id>/world/ 스캔
```

Phase별 외래키 검증 (위반 시 partial commit 방지):
- Phase 1 Group: `parent_group` cycle, `members`, `headquarters`, `allied_groups`/`rival_groups` 존재 여부 (Phase 2·3에서 일제히 에러 승격, 0건 보장)
- Phase 2 Person: `birthplace`/`current_location`(Phase 3 활성), 그룹 멤버십 양방향
- Phase 3 Place: `parent_place` cycle, `bordering_places`/`geography_refs` 존재, `Place.spatial.geography_refs` layer 일치, `controlling_group` (sect kind만)
- Phase 4 Atlas: `references` ↔ `places.id` 모두 존재 + 중복 금지
- Phase 5a Event: `participants.{persons,groups,places}` 외래키 0건 + `related_events` 양방향, `era_id`(Phase 5b 활성)
- Phase 5b Era: `parent_era`/`successor_era` 존재 + `key_events`(Phase 5a Event 양방향) + Atlas overlay
- Phase 5b Timeline: `events`/`eras` references 외래키 (관계 도메인)

**Mind Studio 통합** (`NPC_MIND_WORLD_DB` 환경변수로 빌드된 SQLite 부착):
- 부팅 시 `kind in {active, player}`인 Person을 자동으로 인메모리 `MindRepository`에
  `add_npc`로 등록 → dialogue/scene 경로가 즉시 NPC를 인식
- REST 엔드포인트 (embed feature):
  - `GET /api/world/{groups,persons,places,atlases,events,eras,timelines}` 목록
  - `GET /api/world/{...}/search?q=` FTS5 trigram 검색
  - `GET /api/world/{...}/{id}` 단건
  - `POST /api/world/persons/sync` 런타임 재동기화 (emotion/scene 보존)
- 부착 안 됨 시 위 핸들러는 501 NotImplemented 반환

**도메인 → mind 동기화** (`worldbuilding/mind_sync.rs`):
- `person_to_npc(&Person) -> Option<Npc>` — `kind in {active, player}`만 변환,
  HEXACO 6-dim → 4-facet spread (24 facet 정형 보존은 후속 phase)

설계 문서: [`docs/tasks/world building/00-roadmap.md`](docs/tasks/world building/00-roadmap.md) (10 Phase 흐름) +
phase별 task/report (`docs/tasks/task-phaseN-*.md` + `phaseN-checkpointM-report.md`).

### 빌드 주의사항 (Windows)

- `--features embed`: ort(ONNX Runtime) 정적 링크를 위해 `.cargo/config.toml`에서 CRT를 동적으로 통일해야 함. 변경 후 `cargo clean` 필수. CRT 통일을 위한 `CFLAGS=/MD` / `CXXFLAGS=/MD`는 셸/CI 환경변수로 직접 설정해야 한다 (Cargo `[env]`는 모든 타겟에 적용되어 Linux/macOS 빌드를 깨므로 config.toml에 두지 않는다).
- `--features chat`: rig-core 기본 TLS 백엔드(`aws-lc-sys`)가 MSVC에서 `__builtin_bswap` 링크 실패. Cargo.toml에서 `default-features = false, features = ["reqwest-native-tls"]` 사용.
- rig 0.33+ OpenAI provider는 기본 Responses API(`/v1/responses`) 사용. llama.cpp 등 로컬 서버는 Chat Completions만 지원하므로 `.completions_api()` 호출 필수.

## 프로젝트 구조 (주요 디렉토리)

```
src/
  application/    어플리케이션 계층, 라이브러리 진입점 (v2 단일 경로)
                  - error.rs (MindServiceError — 공용 서비스 에러, 5 variant: NpcNotFound/RelationshipNotFound/InvalidSituation/EmotionStateNotFound/LocaleError)
                  - dto/ 도메인별 DTO 모듈 (hexagonal refactor에서 단일 dto.rs에서 분리 — 구현 현황 표 참조)
                    - emotion.rs / guide.rs / information.rs / relationship.rs / rumor.rs / scene.rs / world.rs
                    - mod.rs (CanFormat trait + 공용 re-export)
                  - event_store.rs, event_bus.rs (Event Sourcing 인프라)
                  - projection.rs (EmotionProjection/RelationshipProjection/SceneProjection 구조체 — v2 wrapper가 재사용)
                  - memory_projector.rs (EventBus 구독 기억 인덱싱 [embed])
                  - scenario_seeds.rs (시나리오 JSON initial_rumors/world/faction/family seed → Rumor + MemoryEntry, Step E3.2)
                  - scene_service.rs, situation_service.rs (도메인 조립 helper)
                  - dialogue_orchestrator.rs [chat] (LLM 다턴 오케스트레이터)
                  - dialogue_test_service.rs [chat] (DTO 전용 — Chat*Request/Response)
                  - director/ (B안 B4 — 다중 Scene facade)
                    - mod.rs (Director: start_scene / end_scene / dispatch_to / active_scenes)
                    - scene_task.rs (spawn_scene_task — mpsc 루프)
                    - spawner.rs (Spawner trait — runtime-agnostic)
                  - command/ (CQRS Command Side — v2 단일 경로)
                    - types.rs (Command enum + aggregate_key + DispatchV2Output/Error)
                    - uow.rs (**UnitOfWork** — 트랜잭션 내 변경 애그리거트 추적 + 일괄 commit)
                    - handler_v2.rs (EventHandler::handle_v2 + EventHandlerContext<'a, 'b, R> + DynamicHandlerContext trait + HandlerShared(출력 쉐이프) + test_support::HandlerTestHarness)
                    - priority.rs (SCENE_START/EMOTION_APPRAISAL/STIMULUS_APPLICATION/GUIDE_GENERATION/WORLD_OVERLAY/RELATIONSHIP_UPDATE/INFORMATION_TELLING/RUMOR_SPREAD 상수 + inline WORLD_OVERLAY_INGESTION/RELATIONSHIP_MEMORY/SCENE_CONSOLIDATION + invariants)
                    - dispatcher.rs (CommandDispatcher: dispatch_v2 + with_default_handlers + with_memory/with_memory_full + with_rumor + with_world_overlay + with_scene_consolidation 빌더)
                    - projection_handlers.rs (EmotionProjectionHandler/RelationshipProjectionHandler/SceneProjectionHandler Inline EventHandler wrappers)
                    - telling_ingestion_handler.rs (TellingIngestionHandler — InformationTold → MemoryEntry(Heard/Rumor), Step C2)
                    - rumor_distribution_handler.rs (RumorDistributionHandler — RumorSpread → 수신자별 MemoryEntry, Step C3)
                    - world_overlay_handler.rs (WorldOverlayHandler — WorldEventOccurred → Canonical MemoryEntry + supersede, Step D)
                    - scene_consolidation_handler.rs (SceneConsolidationHandler — SceneEnded → Layer A→B 흡수, Step D)
                    - relationship_memory_handler.rs (RelationshipMemoryHandler — RelationshipUpdated.cause별 분기 → MemoryEntry(RelationshipChange), Step D)
                    - policies/ (impl EventHandler — hexagonal refactor에서 agents/ → policies/ 이름 정리)
                      - emotion_policy.rs (AppraiseRequested → EmotionAppraised)
                      - stimulus_policy.rs (StimulusApplyRequested → StimulusApplied/BeatTransitioned)
                      - guide_policy.rs (EmotionAppraised/StimulusApplied/GuideRequested → GuideGenerated)
                      - relationship_policy.rs (BeatTransitioned/RelationshipUpdateRequested/DialogueEndRequested)
                      - scene_policy.rs (SceneStartRequested → SceneStarted + EmotionAppraised)
                      - information_policy.rs (TellInformationRequested → 청자당 1 InformationTold, Step C2)
                      - rumor_policy.rs (Seed/SpreadRumorRequested → RumorSeeded/RumorSpread + RumorStore 연동, Step C3)
                      - world_overlay_policy.rs (ApplyWorldEventRequested → WorldEventOccurred, Step D)
  domain/         순수 도메인 로직
                  - personality (HEXACO), emotion (OCC appraisal), relationship, pad, guide
                  - listener_perspective [feature, default-on — Phase 7 Step 5] (화자 PAD → 청자 PAD 변환: prefilter + sign/magnitude k-NN + Converter trait, 88% baseline. DialogueOrchestrator · Mind Studio 양 경로에서 옵셔널 자동 적용)
                  - event.rs (DomainEvent + EventMetadata { correlation_id, parent_event_id, cascade_depth } 활성, EventPayload — 28 variants 포함 *Requested 10종 + Memory/Rumor/Information/World 13종, Event Sourcing)
                  - rumor.rs (Rumor 애그리거트 — RumorOrigin/ReachPolicy/RumorHop/RumorDistortion/RumorStatus + 불변식 I-RU-1~6, Step C1)
                  - aggregate.rs (AggregateKey: Scene/Npc/Relationship/Memory/Rumor/World)
                  - scene_id.rs (B안 B4 S2 — SceneId composite key)
                  - memory.rs (MemoryEntry, MemoryType, MemoryResult — RAG; MemoryScope/Source/Layer/Provenance Step A 확장)
                  - memory/ (도메인 서비스: ranker.rs MemoryRanker + service.rs MemoryAugmentationService — dialogue_orchestrator의 augmentation 로직을 도메인으로 추출)
                  - tuning.rs (조정 가능 파라미터 중앙 관리)
                  - world/ (Worldbuilding 9+2 도메인 — atlas/era/event/group/item/knowledge/lore/person/place/skill/timeline, **장르 어휘 절대 미사용**)
  worldbuilding/  마크다운(SoT) → 도메인 변환 + WorldRepository 포트 (sync trait)
                  - markdown/ (frontmatter YAML + H2 섹션 파서, atlas/era/event/group/person/place/timeline 변환기 7종)
                  - repository.rs (WorldRepository trait — list/get/search/upsert/count, get_*_batch override 가능)
                  - mind_sync.rs (Person → Npc 변환, kind in {active, player}만)
                  - builder.rs (Phase 2 폼 시스템 진입점 — Phase 1엔 빈 자리)
  lore/           Lore RAG (EPUB → SQLite 임베딩 인덱스) — corpus/ingest/query/store
  ports/          헥사고날 포트 트레이트 (hexagonal refactor에서 단일 ports.rs를 ISP 기반 모듈로 분할)
                  - persistence.rs (MindRepository + NpcWorld + EmotionStore + SceneStore — 분리된 super-trait + SceneStore::get_scene_by_id)
                  - personality.rs (PersonalityProfile, PadAnchorSource, AnchorLoadError)
                  - guide.rs (GuideFormatter, Appraiser, StimulusProcessor)
                  - memory.rs (MemoryStore + RumorStore + MemoryFramer)
                  - analysis.rs (UtteranceAnalyzer, EmbedError)
                  - chat.rs [chat] (ConversationPort + InferenceTimings + ChatResponse + LlmModelInfo + LlmInfoProvider/LlmModelDetector + ConversationError(Timeout 포함))
                  - monitoring.rs [chat] (InferenceServerMonitor + ServerHealth + InferenceSlotInfo + ServerMetrics + MonitoringError — hexagonal refactor에서 Llama* → Inference*/Server* 일반화)
  adapter/        포트 구현 (InMemoryRepository — multi-scene HashMap + last_scene_id, OrtEmbedder, RigChatAdapter,
                  SqliteMemoryStore/SqliteRumorStore/SqliteWorldStore [embed])
  presentation/   다국어 포맷터 (ko, en TOML 기반, deep merge 지원) + memory_formatter
  bin/mind-studio/  Axum REST API + SSE MCP 서버 + SSE 실시간 동기화 + static 파일 서빙
                  - main.rs       라우팅 셋업 + 부팅 (Worldbuilding 자동 부착 + Person sync)
                  - state.rs      AppState/StateInner — shared_dispatcher + memory_store/rumor_store(embed) + projection Arc 공유
                  - studio_service.rs  cross-handler 도메인 helper (시나리오 list, 자동 분석기 fallback 등)
                  - repository.rs ReadOnlyAppStateRepo — handler 내부 read 전용 어댑터
                  - mcp_server.rs MCP rmcp 서버 (search_lore/list_corpora/get_chunk + world 조회 + Mind 조작 도구)
                  - events.rs     SSE StateEvent 정의 + 송신 헬퍼
                  - domain_sync.rs  9 dispatch helper (appraise/stimulus/end_dialogue/guide/start_scene/tell_information/apply_world_event/seed_rumor/spread_rumor) + sync_from_repo (shared_dispatcher 재사용, per-request snapshot 제거됨)
                  - trace_collector.rs  AppraisalCollector — Mind Studio 전용 추적 수집기
                  - handlers/     REST 핸들러 (chat/events/llm/memory/npc/object/query/relationship/rumor/scenario/v2_scenes/world{,_atlases,_eras,_events,_groups,_persons,_places,_timelines})
                  - handler_tests.rs / init_tests.rs  in-process REST 통합 테스트
                  - /api/*       메인 UI 경로 — AppState(StateInner) 기반, B5.2 (2/3)부터 내부적으로 v2 dispatch_v2 호출
                  - /api/v2/*    Director shadow 경로 (B4 S3 B-Mini, 분리 Repository + SceneTask 실험용)
                  - /api/world/* Worldbuilding 조회 (groups/persons/places/atlases/events/eras/timelines, embed feature)
                  - /api/projection/{emotion,relationship,scene}/* Read Side projection 조회
                  - /api/projection/trace/{correlation_id}  dispatch_v2 한 호출의 인과 사슬 조회
  bin/lore_ingest.rs   EPUB → SQLite 인덱싱 CLI (embed feature)
  bin/world_load.rs    마크다운 → SQLite 빌드 CLI (embed feature, Phase 1·2·3·4·5a·5b)
tests/            통합 테스트 (TestContext 공유) — dispatch_v2_test, director_test, dialogue_*, world_chilguk_chunchu_*, orchestrator_error_propagation_test, rig_chat_timeout_test 등 v2 기준
locales/          ko.toml, en.toml + PAD 앵커 (locales/anchors/)
docs/             아키텍처/감정/성격/가이드 상세 문서 + tasks/(phase 명세) + changes/(API 변경 로그)
data/             소설 기반 테스트 시나리오 + 캐릭터 프리셋(presets/) + listener_perspective 학습 데이터 + corpus(Lore RAG)
genres/           장르 패키지 (e.g., wuxia/{forms, genre.toml, markdown_template})
projects/         사용자 worldbuilding 프로젝트 (e.g., chilguk-chunchu/{project.toml, world/, build/world.sqlite})
mind-studio-ui/   Vite + React + TypeScript + Zustand 프론트엔드 (빌드 → bin/mind-studio/static/)
```

## 아키텍처

### 계층 구조
1. **Domain** (`src/domain`): 순수 비즈니스 로직, 외부 의존성 없음
2. **Application** (`src/application`): 도메인 조립 및 흐름 제어, 라이브러리 사용자 진입점
3. **Ports** (`src/ports/`): 헥사고날 경계 정의 (`persistence`/`personality`/`guide`/`memory`/`analysis`/`chat`/`monitoring` ISP 분할)
4. **Infrastructure/Presentation** (`src/adapter`, `src/presentation`, `src/bin`): 외부 구현 및 API 노출

### 핵심 진입점

**`InMemoryRepository`** — 기본 `MindRepository` 구현체 (`adapter/memory_repository.rs`)
- `from_file("scenario.json")` / `from_json(json_str)` / `new()` + `add_npc()`/`add_relationship()`/`add_object()`
- `scenario_name()`, `scenario_description()`, `turn_history()` 메타데이터 접근자

**`CommandDispatcher<R>`** — CQRS Command 오케스트레이터 (`application/command/dispatcher.rs`)
- `::new(repo, event_store, event_bus)` — 기본 생성. 내부에서 `Arc<Mutex<R>>`로 감싸짐
- `.with_default_handlers()` — ScenePolicy/EmotionPolicy/StimulusPolicy/GuidePolicy/RelationshipPolicy/InformationPolicy/WorldOverlayPolicy + 3 Projection wrapper 자동 등록
- `.with_memory(store)` — **lean** (Step C2 호환): `TellingIngestionHandler`만 부착. 기존 콜러의 silent behavior change 방지용.
- `.with_memory_full(store)` — **Step D 전체 번들**: Telling + WorldOverlay + SceneConsolidation 3종 Inline 핸들러 일괄 부착 (RelationshipMemory는 별도 register_inline 필요).
- `.with_world_overlay(store)` / `.with_scene_consolidation(store)` — Step D 핸들러 단일 부착 빌더.
- `.with_rumor(memory_store, rumor_store)` — RumorPolicy (Transactional) + RumorDistributionHandler (Inline)
- `async fn dispatch_v2(&self, cmd) -> Result<DispatchV2Output, DispatchV2Error>` — 10 Command 지원 (Appraise/ApplyStimulus/GenerateGuide/UpdateRelationship/EndDialogue/StartScene/TellInformation/SeedRumor/SpreadRumor/ApplyWorldEvent). Command → 초기 *Requested 이벤트 → **Transactional BFS (UnitOfWork에 변경 누적)** → **UoW.commit() (repo 일괄 반영)** → Commit staging buffer (이벤트 영속화) → Inline projection (별도 UoW) → Fanout 순서.
  - `DispatchV2Output { events: Vec<DomainEvent>, shared: HandlerShared }` — `events`는 commit된 이벤트 시퀀스, `shared`는 핸들러 간 최종 scratchpad 스냅샷(UoW에서 출력 호환용으로 복원). 호출자(`DialogueOrchestrator`/`domain_sync`)가 Response DTO 재구성에 사용.
  - `DispatchV2Error` variants: `InvalidSituation` (400) · `CascadeTooDeep` / `EventBudgetExceeded` (500 invariant) · `HandlerFailed { handler, source: HandlerError }` (handler.error 매핑 그대로).
- **안전 한계**: `MAX_CASCADE_DEPTH = 4`, `MAX_EVENTS_PER_COMMAND = 21` (현재 worst-case 커맨드 체인 — `EndDialogue`(3 follow-up) · `TellInformation`(청자 N개) 등 — 기반 실측 상한. 정확한 산정·테스트는 `application/command/dispatcher.rs` 참조)
- `event_store()` / `event_bus()` — 내부 의존성 노출
- `repository_guard() -> MutexGuard<R>` — NPC/관계 등록 같은 `&mut self` 메서드 호출용. `repository_arc() -> Arc<Mutex<R>>` — 공유 소유가 필요한 드문 경우.
- `.register_transactional(h)` / `.register_inline(h)` — 커스텀 EventHandler 등록

**`Director<R>`** — 다중 Scene facade (`application/director/mod.rs`, B안 B4 Session 4 async 재작성)
- `::new(dispatcher, spawner: Arc<dyn Spawner>)` — CommandDispatcher + runtime-agnostic Spawner로 Scene task 관리
- `async start_scene(npc, partner, significance, focuses) -> SceneId` — SceneTask spawn + `Command::StartScene` 첫 메시지 fire-and-forget
- `async dispatch_to(scene_id, cmd) -> ()` — mpsc send, 결과는 `event_bus().subscribe()`로 관찰
- `async end_scene(scene_id, significance)` → `Command::EndDialogue` 전송 + sender drop → SceneTask 자연 종료
- `async active_scenes()` / `async is_active(scene_id)` — 활성 Scene 목록
- `dispatcher() -> &Arc<CommandDispatcher<R>>` — broadcast 구독, repository guard 접근용
- `DirectorError::{SceneNotActive, SceneAlreadyActive, SceneMismatch, SceneChannelClosed, Dispatch}` — lifecycle 에러 variant
- **Spawner injection** (runtime-agnostic): `Arc::new(|fut: BoxFuture<'static, ()>| { tokio::spawn(fut); })` 같은 클로저로 주입. 라이브러리 core는 `tokio::spawn` 미호출 → `tokio/rt` feature 불필요. Bevy/async-std 등 임의 런타임 호환.

**`DialogueOrchestrator<R, C>`** — LLM 대사 생성 오케스트레이터 (`application/dialogue_orchestrator.rs`, chat feature)
- `CommandDispatcher<R>` + `ConversationPort` 조합으로 Event Sourcing 경로에 맞춘 LLM 다턴 대화
- **전제**: dispatcher는 `.with_default_handlers()`가 호출된 상태여야 함 (v2 path 사용).
- `::new(dispatcher, chat, formatter)` 빌더:
  - `.with_analyzer(analyzer)` — 발화 → PAD 변환기 (embed feature, `PadAnalyzer`)
  - `.with_memory(store, framer)` — Memory Step B push (BeatTransitioned 시 기억 블록 prepend)
  - `.with_memory_locale(lang)` — `LocaleMemoryFramer` 라벨 언어
  - `.with_converter(converter)` — listener_perspective PAD 변환기 (`Converter` trait, Phase 7 default-on. 미주입 시 자동 폴백)
- `start_session(sid, npc, partner, situation?)` — `Command::Appraise` **dispatch_v2.await** + LLM 세션 시작
- `turn(sid, utterance, pad?, sit_desc?)` — user 턴 이벤트 → `Command::ApplyStimulus` **dispatch_v2.await** → (events에 `BeatTransitioned` 존재 시 `update_system_prompt`) → `send_message` → assistant 턴 이벤트
- `end_session(sid, significance?)` — LLM 세션 종료 + (significance 있으면) `Command::EndDialogue` **dispatch_v2.await**

### 주요 Command (v2 단일 경로)

| Command | 초기 이벤트 | 용도 |
|---|---|---|
| `Appraise` | `AppraiseRequested` | 초기 상황 판단 및 감정 생성 |
| `ApplyStimulus` | `StimulusApplyRequested` | 대화 중 실시간 감정 변화 + Beat 전환 자동 처리 |
| `GenerateGuide` | `GuideRequested` | 현재 감정에서 가이드 재생성 |
| `UpdateRelationship` | `RelationshipUpdateRequested` | 명시적 관계 갱신 |
| `EndDialogue` | `DialogueEndRequested` | Scene 종료 + 관계 정산 (3 follow-up 이벤트) |
| `StartScene` | `SceneStartRequested` | Scene 시작 + 초기 focus appraise |
| `TellInformation` | `TellInformationRequested` | 화자 → 청자·동석자에게 정보 전달 → 청자당 `InformationTold` + `MemoryEntry(Heard/Rumor)` (Step C2) |
| `SeedRumor` | `SeedRumorRequested` | 새 Rumor 애그리거트 시딩 → `RumorSeeded` (Step C3) |
| `SpreadRumor` | `SpreadRumorRequested` | 기존 Rumor 홉 추가 → `RumorSpread` + 수신자별 `MemoryEntry(Rumor)` (Step C3) |
| `ApplyWorldEvent` | `ApplyWorldEventRequested` | 세계 사건 적용 → `WorldEventOccurred` + Canonical `MemoryEntry(World, Seeded)` + 기존 Topic supersede (Step D) |

### 주요 포트 (전체는 `src/ports/` 참조)

| 포트 | 용도 | 기본 구현체 |
|------|------|----------|
| `MindRepository` | `NpcWorld` + `EmotionStore` + `SceneStore` 통합 super-trait | `InMemoryRepository` |
| `Appraiser` | OCC 감정 평가 엔진 | `AppraisalEngine` |
| `StimulusProcessor` | PAD 자극 처리 엔진 | `StimulusEngine` |
| `GuideFormatter` | 가이드 → 텍스트/JSON | `LocaleFormatter` |
| `UtteranceAnalyzer` | 대사 → PAD ([embed feature]) | `PadAnalyzer` |
| `ConversationPort` | LLM 다턴 대화 세션 ([chat feature]) | `RigChatAdapter` (with_timeout 빌더 — 기본 60s) |
| `InferenceServerMonitor` | 추론 서버 모니터링: health/slots/metrics ([chat feature], 이전 `LlamaServerMonitor`에서 일반화) | `RigChatAdapter` |
| `MemoryStore` | RAG 기억 저장/검색 | `SqliteMemoryStore` [embed] (FTS5 + sqlite-vec vec0). 테스트 전용 `InMemoryMemoryStore`는 `tests/common/in_memory_store.rs` |
| `RumorStore` | Rumor 애그리거트 저장/검색 (Step C1) | `SqliteRumorStore` [embed]. 테스트 전용 `InMemoryRumorStore`는 `tests/common/in_memory_rumor.rs` |
| `MemoryFramer` | 기억 엔트리 → 프롬프트 블록 (Source별 라벨, Step B) | `LocaleMemoryFramer` (`presentation/memory_formatter.rs`, ko/en 빌트인) |
| `EventStore` | 도메인 이벤트 영속화 (append-only) | `InMemoryEventStore` |

### 감정 평가 흐름

`AppraisalEngine`은 세부 모듈로 분리되어 있습니다:
- **event** (Joy/Distress/Hope/Fear), **action** (Pride/Admiration/Anger), **object** (Love/Hate)
- **compound**: 기초 감정 결합 — Gratification(Pride+Joy), Remorse(Shame+Distress), Gratitude(Admiration+Joy), Anger(Reproach+Distress)
- 성격 가중치 패턴: `BASE + (Score × W)` — `personality.rs` 내부 상수 관리
- 관계 변조: closeness(공감/적대 강도 배율), trust(행동 평가 가중치)


## EventBus · CQRS · Event Sourcing · Multi-Handler

> 상세 설계: [`docs/architecture/system-design-eventbus-cqrs.md`](docs/architecture/system-design-eventbus-cqrs.md)

### 아키텍처 개요

B안(v2) 이행 완료. v0.3.0에서 v1 경로(Pipeline/MindService/FormattedMindService/EventAwareMindService/`dispatch`/`shadow_v2`) 전부 제거됨.

```
┌─ Director (B안 B4, 다중 Scene facade) ──────────────────────┐
│  start_scene / dispatch_to(scene_id, cmd) / end_scene        │
│  active_scenes / DirectorError::Scene{NotActive|Mismatch|…}  │
├─ CommandDispatcher::dispatch_v2 (v2 write side) ────────────┤
│  Command → initial *Requested event → BFS cascade →          │
│    [Transactional handlers + UnitOfWork] → UoW.commit() →    │
│    [Append to EventStore] → [Inline projections] → [Fanout]  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐        │
│  │ Scene    │ │ Emotion  │ │Stimulus  │ │  Guide   │        │
│  │ Policy   │ │ Policy   │ │ Policy   │ │ Policy   │        │
│  │ (pri 5)  │ │ (pri 10) │ │ (pri 15) │ │ (pri 20) │        │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘        │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐         │
│  │World Overlay │ │Relationship  │ │ Information  │         │
│  │ Policy (25)  │ │ Policy (30)  │ │ Policy (35)  │         │
│  └──────────────┘ └──────────────┘ └──────────────┘         │
│                     (Step D)        (Step C2)                │
│  ┌──────────────┐                                            │
│  │   Rumor      │                                            │
│  │ Policy (40)  │                                            │
│  └──────────────┘                                            │
│                     (Step C3)                                │
│  Inline: Emotion/Relationship/Scene ProjectionHandler +      │
│          TellingIngestionHandler (C2) +                      │
│          RumorDistributionHandler (C3) +                     │
│          WorldOverlayHandler (D, pri 45) +                   │
│          SceneConsolidationHandler (D, pri 60) +             │
│          RelationshipMemoryHandler (D, pri 50, 별도 부착*)   │
│  *RelationshipMemoryHandler는 `with_memory_full`에 포함되지   │
│   않는다. 호출부에서 `register_inline(...)`로 명시 등록.      │
├─ DialogueOrchestrator (Dispatcher + ConversationPort wrapper) [chat]┤
│  start_session / turn / end_session async API               │
│  Beat 전환 시 ConversationPort.update_system_prompt         │
├─ EventBus (tokio::broadcast fan-out) ──────────────────────┤
│  subscribe() → impl Stream<Arc<DomainEvent>> (runtime-agnostic)│
├─ MemoryProjector (broadcast 구독) [embed] ──────────────────────┤
│  DialogueTurnCompleted/RelationshipUpdated → 임베딩 → RAG    │
└─────────────────────────────────────────────────────────────┘
```

### 이벤트 흐름 (v2)

```
Command 수신
  → CommandDispatcher.dispatch_v2(cmd)
  → build_initial_event(cmd) → *Requested event (enqueue depth=0)
  → UnitOfWork::new(&repo) — 트랜잭션 시작
  → [Transactional phase — BFS]
       각 event pop → priority 오름차순 transactional_handlers 실행
         → DynamicHandlerContext 헬퍼로 UoW에 변경 예약
            (save_emotion_state/save_relationship/save_scene/set_guide/clear_*)
         → follow_up_events → queue.push(depth+1) [MAX_CASCADE_DEPTH=4 가드]
       event → staging_buffer [MAX_EVENTS_PER_COMMAND=21 가드]
  → HandlerShared 스냅샷 추출 (DispatchV2Output 출력 호환)
  → uow.commit() — repository에 save_*/clear_* 일괄 반영 (Unit of Work 원자적 커밋)
  → [Commit phase] staging_buffer → event_store.append (실 ID/seq 할당)
  → [Inline phase] 별도 UoW로 projection handlers — best-effort, 에러는 로그만
  → [Fanout phase] event_bus.publish (tokio::broadcast)
```

### DomainEvent (28 variants)

#### v2 초기 이벤트 (10종 *Requested — Command → initial event)
| EventPayload | 생성 계기 | 소비 Handler |
|---|---|---|
| `AppraiseRequested` | `Command::Appraise` | EmotionPolicy |
| `StimulusApplyRequested` | `Command::ApplyStimulus` | StimulusPolicy |
| `GuideRequested` | `Command::GenerateGuide` | GuidePolicy |
| `RelationshipUpdateRequested` | `Command::UpdateRelationship` | RelationshipPolicy |
| `DialogueEndRequested` | `Command::EndDialogue` | RelationshipPolicy (3 follow-ups) |
| `SceneStartRequested` | `Command::StartScene` | ScenePolicy (prebuilt_scene 포함) |
| `TellInformationRequested` | `Command::TellInformation` | InformationPolicy (청자당 1 InformationTold) — Step C2 |
| `SeedRumorRequested` | `Command::SeedRumor` | RumorPolicy (pending_id로 커맨드별 고유 aggregate) — Step C3 |
| `SpreadRumorRequested` | `Command::SpreadRumor` | RumorPolicy (RumorSpread + hop 기록) — Step C3 |
| `ApplyWorldEventRequested` | `Command::ApplyWorldEvent` | WorldOverlayPolicy (WorldEventOccurred follow-up) — Step D |

#### 결과 이벤트 (9종 Mind + 8종 Memory/Rumor)
| EventPayload | 발생 시점 |
|---|---|
| `EmotionAppraised` | appraise 완료 (emotion_snapshot 포함) |
| `StimulusApplied` | PAD 자극 적용 (emotion_snapshot 포함) |
| `BeatTransitioned` | Focus 전환 — **B4 S3 Option A: `partner_id` 필드** (multi-scene 정확성) |
| `SceneStarted` / `SceneEnded` | Scene 시작/종료 |
| `RelationshipUpdated` | 관계 갱신 (before/after 6값 + `cause: RelationshipChangeCause`) |
| `GuideGenerated` | 가이드 생성 |
| `DialogueTurnCompleted` | 대화 턴 완료 (npc_id, partner_id, speaker, utterance, emotion_snapshot) |
| `EmotionCleared` | 감정 초기화 |
| `InformationTold` | Mind→Memory — 화자가 청자/동석자 각자에 발화 (listener_role, topic 포함) Step C2 |
| `MemoryEntryCreated` / `MemoryEntrySuperseded` / `MemoryEntryConsolidated` | Memory 엔트리 수명주기 (Step C1 선언; 현재 Inline 핸들러들이 `MemoryStore`를 직접 호출하고 이벤트는 미발행 — Step F의 Memory 이벤트 팬아웃 과제로 이관) |
| `RumorSeeded` / `RumorSpread` | Rumor 시딩·확산 (Step C3) |
| `RumorDistorted` / `RumorFaded` | 변형·종결 (Step F 발행 예정) |
| `WorldEventOccurred` | 세계 사건 오버레이 → Canonical `MemoryEntry(World, Seeded)` 생성 + 같은 topic supersede (Step D) |

#### AggregateKey 매핑 (라우팅 기준)
- `Scene { npc_id, partner_id }`: SceneStarted/Ended/StartRequested, DialogueEndRequested, BeatTransitioned
- `Relationship { owner_id, target_id }`: RelationshipUpdated/UpdateRequested
- `Npc(npc_id)`: AppraiseRequested/EmotionAppraised/StimulusApply(Requested)/GuideRequested/GuideGenerated/DialogueTurnCompleted/EmotionCleared · `TellInformationRequested`(speaker) · `InformationTold`(listener — B5 청자 기반 라우팅)
- `Rumor(rumor_id)`: `RumorSeeded/Spread/Distorted/Faded`, `SpreadRumorRequested`. `SeedRumorRequested`는 `Rumor("pending-<pending_id>")`로 커맨드별 고유 (Step C3 사후 리뷰 C2)
- `Memory(entry_id)`: `MemoryEntryCreated/Superseded/Consolidated` (선언만 — Step F에서 이벤트 팬아웃 활성화 예정)
- `World(world_id)`: `ApplyWorldEventRequested/WorldEventOccurred` (Step D)

### Pipeline (v0.3.0 제거됨)

v2 `dispatch_v2`의 transactional handler chain (BFS + follow_up_events)이 Pipeline을 대체.
`with_default_handlers()` + `dispatch_v2(cmd)` 조합이 유일한 write 경로.

### v2 EventHandler + UnitOfWork

모든 Policy + Projection wrapper는 공통 `EventHandler` trait을 구현. UoW 도입과 함께
`handle()` → `handle_v2()` + `DynamicHandlerContext` 동적 디스패치로 전환됨 (구현 현황
표의 "Unit of Work pattern" 행 참조):

```rust
pub trait EventHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn interest(&self) -> HandlerInterest;          // Kinds(vec![EventKind::...])
    fn mode(&self) -> DeliveryMode;                  // Transactional/Inline
    fn handle_v2(
        &self,
        event: &DomainEvent,
        ctx: &mut dyn DynamicHandlerContext,
    ) -> Result<HandlerResult, HandlerError>;
}
```

**`UnitOfWork<'a, R>`** (`application/command/uow.rs`) — 단일 dispatch_v2 호출
범위에서 변경된 애그리거트(NPC 감정·관계·Scene·guide)를 dirty checking으로 추적하고
`commit()` 시 `Arc<Mutex<R>>`에 일괄 반영하는 작업 단위. dispatcher는 Transactional BFS
종료 후 `uow.commit()`을 호출하고, Inline projection 단계에서는 별도 `UnitOfWork`
인스턴스로 격리한다. 핸들러는 UoW를 직접 보지 않고 `DynamicHandlerContext` 메서드
(`save_emotion_state`/`save_relationship`/`save_scene`/`clear_emotion_for`/`clear_scene`/
`set_guide`)를 통해 변경을 예약한다.

**`HandlerShared`** — `DispatchV2Output.shared`로만 노출되는 출력 쉐이프 (UoW의
최종 상태 스냅샷). 호출자 DTO 재구성용으로만 사용되며 핸들러 내부 mutation은
UoW 측에서 일어난다. 필드는 `emotion_state`/`relationship`/`scene`/`guide` +
destructive 시그널 (`clear_emotion_for`/`clear_scene`).

`EventHandlerContext<'a, 'b, R>` 헬퍼 (UoW 도입과 함께 ISP 적용 — `world: &dyn NpcWorld`,
`emotions: &dyn EmotionStore`, `scenes: &dyn SceneStore`로 분리). 핸들러는
`DynamicHandlerContext` 메서드를 통해 자동 `HandlerError` 매핑을 받는다:
- `ctx.get_npc(id) -> Result<Npc, HandlerError>` — `HandlerError::NpcNotFound`
- `ctx.get_relationship(owner, target) -> Result<Relationship, _>` — UoW 우선, repo fallback
- `ctx.get_emotion_state(npc_id) -> Result<EmotionState, _>` — UoW 우선, repo fallback
- `ctx.get_scene_by_id(&SceneId) -> Option<Scene>` — UoW 우선, repo fallback

**EventMetadata 활성** (2026-04-25): 모든 이벤트는 `correlation_id`(dispatch_v2 호출
단위 cid, 1부터) + `parent_event_id`(BFS 큐잉 시 자동) + `cascade_depth` 필드를
가진다. `EventStore::get_events_by_correlation(cid)` + Mind Studio
`/api/projection/trace/{cid}`로 한 dispatch의 전체 인과 트리 조회 가능.

### EventBus (tokio::broadcast 기반)

| 계층 | 실행 | 용도 | 구현 |
|------|------|------|------|
| **Transactional handlers** | `dispatch_v2` 내부 BFS | v2 커맨드 내부 폴리시 체인 | priority 오름차순 반복 |
| **Inline projections** | commit 후 동기 | 쿼리 일관성 뷰 | `dispatch_v2` Inline phase |
| **EventBus** (Fanout) | `send()` 후 broadcast | 외부 Projector·SSE·구독자 | `subscribe() -> impl Stream<Arc<DomainEvent>>` |

**공개 API 원칙**: `EventBus.subscribe()`가 반환하는 `futures::Stream`은 runtime-agnostic. Bevy·smol·async-std 등 임의 executor에서 폴링 가능. tokio는 내부 구현 디테일이며 호출자 deps에 노출되지 않음.

**Lag 복구**: `broadcast`는 capacity 초과 시 가장 오래된 이벤트를 덮어쓴다. at-least-once가 필요한 소비자는 `subscribe_with_lag()`로 `Lagged(n)` 통지를 받고 `EventStore.get_events_after_id(last_id)`로 replay한다. (`MemoryProjector::run`이 이 패턴 구현)

### 기억 시스템 (RAG) [embed feature]

```
MemoryProjector (EventBus subscriber)
  → 이벤트 수신 → MemoryEntry 구성 → TextEmbedder 임베딩 → MemoryStore.index()

SqliteMemoryStore (기본 구현, 단일 SQLite 파일, schema v2):
  ├── schema_meta    (마이그레이션 버전 관리)
  ├── memories       (일반 테이블 — 메타 + 원문 TEXT + Step A 신규 13 컬럼)
  ├── memories_fts   (FTS5 가상 테이블, tokenize='trigram' — 한글/CJK 전문 검색)
  ├── memories_vec   (sqlite-vec vec0 가상 테이블 — 코사인 ANN, FLOAT[dim])
  │                    partition key: "personal:<id>" | "relationship:<a>:<b>" 등
  └── rumors/rumor_hops/rumor_distortions (Step C에서 사용 예정, 빈 테이블 선제 생성)
  세 레이어가 id로 조인. search_by_meaning: vec0 Top-K → memories batch load.
  FTS5 trigram 토크나이저는 3-gram 기반이라 한글 단어 경계 문제를 우회한다
  (SQLite 3.34+). 예외 시 LIKE fallback으로 방어.
  v1 DB는 최초 오픈 시 자동 v2 마이그레이션 (ALTER TABLE + vec0 재생성, 트랜잭션).

테스트 전용:
  tests/common/in_memory_store.rs — InMemoryMemoryStore (brute-force cosine).
  라이브러리 public API로 노출되지 않음.
```

**Memory Step A 확장 (완료)**: `MemoryEntry`가 scope/source/provenance/layer/topic/confidence/
recall_count/superseded_by/consolidated_into 등을 포함. Scope는 Personal(기존 호환) 외에도
Relationship(대칭 a≤b 정규화) · Faction · Family · World 5종. Canonical = `Seeded ∧ World`
(τ=∞). `MemoryRanker`가 Source 우선 필터 + 5요소 점수(vec×retention×source×emotion×recency)로
랭킹. 기존 `MemoryType::Dialogue`/`SceneEnd`/`Relationship`는 serde alias로 역호환되며 신규
코드는 `DialogueTurn`/`SceneSummary`/`RelationshipChange`를 사용한다. 상세 설계: [`docs/memory/`](docs/memory/).

**Memory Step B 주입 (완료, [chat feature])**: `DialogueOrchestrator::with_memory(store, framer)` opt-in
빌더로 활성화. 활성화되면 `start_session` 1회 + `BeatTransitioned` 발생 시
`inject_memory_push(npc, query, pad)`가 다음 파이프라인으로 "떠오르는 기억" 블록을 시스템
프롬프트 앞에 prepend한다:

```
DialogueOrchestrator.start_session/turn(BeatTransitioned)
  → query 임베딩 (analyzer 있으면 analyze_with_embedding, 없으면 None)
  → MemoryStore.search(MemoryQuery {
        scope_filter: NpcAllowed(npc),     // Personal + World + Relationship(참여)
        exclude_superseded: true,
        exclude_consolidated_source: true,
        min_retention: MEMORY_RETENTION_CUTOFF (0.10),
        limit: MEMORY_PUSH_TOP_K * 3,       // Ranker 전 oversample
     })
  → MemoryRanker (1단계 Source 우선 + 2단계 5요소 점수) → Top-K (기본 5)
  → MemoryStore.record_recall(id, now_ms)   // best-effort
  → LocaleMemoryFramer.frame_block(entries, locale)
     → "[겪음]/[목격]/[전해 들음]/[강호에 떠도는 소문]" 라벨 + header/footer
  → format!("{block}{system_prompt}")
  → ConversationPort.start_session / update_system_prompt
```

미부착 시 `inject_memory_push`는 빈 문자열 반환 (no-op). 구
`search_by_meaning`/`search_by_keyword`/`get_recent`는 `#[deprecated(since="0.4.0")]` 마킹
(완전 제거는 Step D 이후). Pull 경로(`recall_memory` tool) · 매 turn 재주입 옵션은 Step F.

**sqlite-vec 등록**: `SqliteMemoryStore` 최초 생성 시 `sqlite3_auto_extension(sqlite3_vec_init)`을
프로세스 전역에 `Once`로 한 번만 등록. sqlite-vec는 순수 C 확장이라 **tokio 런타임을 요구하지 않는다**.

**임베딩 차원**: bge-m3는 1024 (`DEFAULT_EMBEDDING_DIM`). 다른 모델은
`SqliteMemoryStore::with_dim(path, dim)` / `in_memory_with_dim(dim)`으로 런타임 지정.
vec0는 스키마에 차원이 고정되므로 모델 교체 시 DB 재생성 필요.

**쿼리**: `SELECT id, distance FROM memories_vec WHERE embedding MATCH ? AND k = ? ORDER BY distance`.
Top-K `(id, distance)`를 vec0에서 받아 `memories`에서 id로 batch load → `MemoryEntry` 복원.
relevance_score = `1.0 - cosine_distance`.

### 구현 현황

| 단계 | 상태 | 내용 |
|------|------|------|
| Phase 1 | ✅ 완료 | EventBus, EventStore, Projections (구조체만 남음) |
| Phase 2 | ✅ 완료 | Command, EmotionPolicy, GuidePolicy, RelAgent, CommandDispatcher (v2 단일 경로) |
| Phase 3 | ✅ 완료 | MemoryProjector, MemoryStore, SqliteMemoryStore, DialogueTurnCompleted |
| EventBus v2 | ✅ 완료 | tokio::broadcast 단일화, runtime-agnostic Stream API, MemoryProjector replay 기반 at-least-once |
| Phase 4 | ✅ 완료 | DialogueOrchestrator — CommandDispatcher + ConversationPort 통합 오케스트레이터 (chat feature) |
| **B안 B0** | ✅ 완료 | EventHandler trait · HandlerShared · AggregateKey · priority 상수 뼈대 |
| **B안 B1** | ✅ 완료 | 4 Policy v2 EventHandler 구현 + StimulusPolicy 신규 + 2 *Requested variant (AppraiseRequested/StimulusApplyRequested) + HandlerTestHarness |
| **B안 B2** | ✅ 완료 | EmotionProjectionHandler/RelationshipProjectionHandler/SceneProjectionHandler (Inline wrapper) |
| **B안 B3** | ✅ 완료 | `dispatch_v2()` BFS loop + `with_default_handlers()` + parallel run 비교 (Appraise/ApplyStimulus) |
| **B안 B4 S1** | ✅ 완료 | 6 Command 전부 v2 지원 + ScenePolicy 신규 + 4 *Requested variant (Guide/RelationshipUpdate/DialogueEnd/SceneStart) + HandlerShared clear 시그널 |
| **B안 B4 S2** | ✅ 완료 | Director + SceneId + InMemoryRepository multi-scene HashMap refactor + 11 E2E 테스트 |
| **B안 B4 S3 Option A** | ✅ 완료 | BeatTransitioned.partner_id 추가 + SceneStore::get_scene_by_id + StimulusPolicy multi-scene fix + 회귀 가드 |
| **B안 B4 S3 Option B-Mini** | ✅ 완료 | Mind Studio `/api/v2/*` shadow 엔드포인트 (7개) + Director 통합 + 7 integration 테스트 |
| **B안 B4 S4 (축소판 A)** | ✅ 완료 | async `dispatch_v2(&self)` + `Arc<Mutex<R>>` 내부 공유 + `Spawner` trait + `SceneTask` mpsc 루프 + Director 전면 async 재작성 (fire-and-forget) + tests cutover. 런타임 중립 유지(`tokio::spawn` 미호출). |
| **B안 B5.1** | ✅ 완료 | Pipeline/Projection trait/EventAwareMindService/HandlerContext·Output/v1 dispatch/v1 Policy handle_* 전부 `#[deprecated(since="0.2.0")]` 마킹, v0.3.0 제거 예정 |
| **B안 B5.2** | ✅ 완료 | (1/3) DialogueOrchestrator v2 마이그레이션. (2/3) Mind Studio handler v2 마이그레이션. (3/3) AppState 통합 — `shared_dispatcher` 도입, per-request snapshot 제거, UI CRUD/scenario load가 `rebuild_repo_from_inner`로 공유 repo 동기화. |
| **B안 B5.3** | ✅ 완료 | v1 모듈·타입 삭제 — Pipeline/Projection trait/EventAwareMindService/MindService/FormattedMindService/HandlerContext·Output/v1 Policy handle_*/AppStateRepository(mut)/DialogueTestService struct/v1 dispatch/shadow_v2 전부 제거. `emotion_snapshot` 헬퍼 → `EmotionState::snapshot()` 메서드로 이관. `MindServiceError` → `application::error` 모듈로 분리. v1 테스트 파일 8종(application/event/command/pipeline/locale/port_injection/repository/coverage_gap) 삭제 + dispatch_v2_test 안의 v1 parallel 테스트 3종 삭제. |
| B안 B5.4 | 불필요 | B5.3에서 `shadow_v2` 이미 제거. |
| **Memory Step A** | ✅ 완료 | `MemoryScope`/`MemorySource`/`Provenance`/`MemoryLayer` VO + `MemoryEntry` 13 필드 확장 + `MemoryType` rename (serde alias 역호환) + `MemoryRanker` 2단계 (Source 우선 + 5요소 점수) + `DecayTauTable` + SQLite v2 자동 마이그레이션 + `MemoryStore` 7 신규 메서드 + `MemoryQuery`/`MemoryScopeFilter` + `RelationshipUpdated.cause` hook. 행동 변화 없이 foundation만. 상세: [`docs/memory/03-implementation-design.md`](docs/memory/03-implementation-design.md) |
| **Memory Step B** | ✅ 완료 | `MemoryFramer` trait + `LocaleMemoryFramer` (Source별 라벨, ko/en locale 빌트인) + `[memory.framing]` locale 섹션 + `DialogueOrchestrator::with_memory(store, framer)` opt-in + `inject_memory_push` 내부 메서드 (NpcAllowed scope 검색 → MemoryRanker 2단계 → Top-K 프롬프트 블록) + `start_session` 1회 + `BeatTransitioned` 시 재주입. 구 `search_by_meaning`/`search_by_keyword`/`get_recent` `#[deprecated(since="0.4.0")]` 마킹. |
| **Memory Step C1** | ✅ 완료 | Rumor 도메인 foundation — `Rumor` 애그리거트 (`src/domain/rumor.rs`) + `RumorOrigin`/`ReachPolicy`/`RumorHop`/`RumorDistortion`/`RumorStatus` + 불변식 I-RU-1~6. `RumorStore` 포트 + `SqliteRumorStore` [embed]. `AggregateKey::Memory/Rumor/World` variant. `EventPayload` 11 신규 variant (`Memory*`/`Rumor*`/`TellInformationRequested`/`InformationTold` 등). 행동 변화 없음. 사후 리뷰에서 schema v3 migration(composite PK)·cycle detection·reach_overlaps 등 7건 수정. 커밋 `bcb0581` + 사후 `30d7f94`. |
| **Memory Step C2** | ✅ 완료 | `Command::TellInformation` + `TellInformationRequest`/`Response` DTO + `InformationPolicy` (Transactional, priority `INFORMATION_TELLING=35`) + `TellingIngestionHandler` (Inline) + `CommandDispatcher::with_memory(store)` 빌더. 청자당 1 `InformationTold` follow-up (B5) + listener `MemoryEntry(Heard/Rumor)` 생성. `stated_confidence × normalized_trust` 신뢰도, origin_chain 기반 Heard/Rumor 자동 분류. 12개 통합 테스트. 커밋 `f410e74` + 사후 `ff3d032`(C1 dispatcher aggregate_id routing 수정, C2 dedup, M1 deterministic id, M3 topic pass-through, M7 budget test). |
| **Memory Step C3** | ✅ 완료 | `Command::SeedRumor` + `Command::SpreadRumor` + `SeedRumorRequest`/`SpreadRumorRequest` DTO + `RumorPolicy` (Transactional, priority `RUMOR_SPREAD=40`) + `RumorDistributionHandler` (Inline). Canonical 해소 3-tier (Distortion → Canonical via `get_canonical_by_topic` → seed_content fallback). `RUMOR_HOP_CONFIDENCE_DECAY^hop_index` 감쇠 + `RUMOR_MIN_CONFIDENCE` floor. `with_rumor(memory_store, rumor_store)` 빌더. 11개 통합 테스트 (rumor_spread + rumor_canonical_resolution). 커밋 `d088470` + 사후 `8413857`(rumor_id event.id=0 버그) + `5ebf37f`(C2 pending_id으로 orphan 공용 버킷 제거, M1 RumorPolicy 자체 counter, §14 원자성 재정의, Step F 명기). |
| **Memory Step D** | ✅ 완료 (+리뷰 반영) | `Command::ApplyWorldEvent` + `ApplyWorldEventRequest` DTO + `WorldOverlayPolicy` (Transactional, priority `WORLD_OVERLAY=25`) + `WorldOverlayHandler` (Inline, priority `WORLD_OVERLAY_INGESTION=45`) — Canonical `MemoryEntry(scope=World, provenance=Seeded)` 생성 + 같은 topic **Canonical 1건만** supersede (다른 NPC의 Personal Heard/Rumor는 보존, 리뷰 B1). `SceneConsolidationHandler` (Inline, priority `SCENE_CONSOLIDATION=60`) — `SceneEnded` 구독 → **참여 NPC별**로 자기 Layer A(`DialogueTurn/BeatTransition`)만 수집해 Personal Scope `SceneSummary` Layer B 엔트리 생성 + `mark_consolidated`, `topic="scene:{a}:{b}"` (리뷰 B3, M7). 휴리스틱 첫·끝 content 요약 (120자 cap). `RelationshipMemoryHandler` (Inline, priority `RELATIONSHIP_MEMORY=50`) — `RelationshipUpdated.cause` 5 variant별 source/topic/content 분기 (`MEMORY_RELATIONSHIP_DELTA_THRESHOLD=0.05` 미만 skip, 주도 축 라벨 content에 포함 — 리뷰 H4). `RelationshipPolicy.BeatTransitioned` 경로에서 cause=`SceneInteraction { scene_id }` 설정. Builder: `with_memory(store)` = lean Telling만, `with_memory_full(store)` = Step D 4종 전체 (리뷰 H5). 16개 통합 테스트 (consolidation 3 + world overlay 7 + relationship cause 7 E2E 포함) + 17개 단위 테스트. 범위 외 (Step F): LLM 기반 Consolidator / witness 개별 MemoryEntry / target 관점 Relationship 엔트리 / DialogueEnd cause=SceneInteraction 승격. |
| **Memory Step E1** | ✅ 완료 (+리뷰 반영) | Mind Studio 백엔드 REST + SSE 배선. `AppState`에 embed-gated `memory_store`/`rumor_store` 필드 + `shared_dispatcher`에 `with_memory_full` + `with_rumor` 자동 부착 (`NPC_MIND_MEMORY_DB` 환경변수, 미설정 시 `:memory:`). `RumorStore::list_all()` 포트 메서드 신설 (+ Sqlite·InMemory·Spy 3종 impl). REST 10 엔드포인트 (`/api/memory/{search,by-npc/{id},by-topic/{topic},canonical/{topic},entries,tell}` + `/api/world/apply-event` + `/api/rumors{,/seed,/{id}/spread}`, 전부 embed feature gated). `domain_sync`에 4 dispatch 헬퍼 (`tell_information`/`apply_world_event`/`seed_rumor`/`spread_rumor`). SSE `StateEvent` 5 variant (`MemoryCreated/Superseded/Consolidated`, `RumorSeeded/Spread`). 통합 테스트 5종 (manual seed / tell+SSE / world apply → canonical / by-topic history / seed+spread). 커밋 `3356675` + 사후 리뷰 `e63d638` (M1 Superseded SSE 오탐 제거, M2 by-topic limit 쿼리, M4 search 스모크, L5 `&mut *inner` 스타일 통일). 범위 외: 프런트엔드 UI → Step E2, 시나리오 JSON `initial_rumors`/`world_knowledge` → Step E3, director_v2 배선/Memory 이벤트 팬아웃 → Step F. |
| **Memory Step E3.3** | ✅ 완료 (+follow-up) | 시드 조회 UI + 로드 warnings 가시화. `GET /api/scenario-seeds` 엔드포인트 — `StateInner.scenario_seeds`(E3.2에서 추가됨) 그대로 반환. 프런트에 `useSceneStore.scenarioSeeds` + `ScenarioSeedsView` 조회 전용 패널 (ResultPanel "시드" 탭, 4 섹션 · RumorSeedRow 4-tier 변종 라벨 · 메모리 메타 뱃지 · 빈 상태 안내). `loadHandlers.loadScenario`가 `LoadResponse.applied_*` count를 success 토스트에, `warnings`를 error 토스트(3건 초과 시 묶음 + `console.warn` 폴백, `String()` 방어)로 노출. `useStateSync`가 초기 마운트 + `scenario_loaded/result_loaded` 시 `/api/scenario-seeds` fetch(useRefresh는 제외 — CRUD refresh 오염 방지). 커밋 `fcf50ec` + follow-up(M1 이중 fetch · M2 토스트 스팸 · L1/L3/L4 방어·라벨링). 범위 외: 시드 편집 GUI · §17.3 결정 3 정식 문서화. |
| **EventMetadata 활성** | ✅ 완료 (2026-04-25) | `correlation_id` (dispatch_v2 호출 단위 cid 자동 발급, 함수 로컬) + `parent_event_id` (BFS 큐잉 시 자동) + `cascade_depth` 활성화. `EventStore::get_events_by_correlation(cid)` 신설. Mind Studio `GET /api/projection/trace/{cid}` 엔드포인트로 한 dispatch의 인과 트리 조회. |
| **Worldbuilding Phase 0** | ✅ 완료 | Lore RAG MCP — wuxia-core/docs EPUB 22권 인덱싱 + `search_lore`/`list_corpora`/`get_chunk` MCP 도구. `data/corpus/lore.sqlite` 자동 부착. |
| **Worldbuilding Phase 1** | ✅ 완료 (2026-04-30) | Group 도메인 (`domain/world/group.rs`) — temporal·parent·allied/rival 관계, 마크다운 frontmatter+H2 파서. SqliteWorldStore + `world-load` CLI. 6 Group 통과. |
| **Worldbuilding Phase 2** | ✅ 완료 (2026-05-01) | Person 도메인 + HEXACO 6-dim — Group 외래키 활성. `mind_sync.rs::person_to_npc`로 active/player Person 자동 NPC 등록. **2.1 Player follow-up** (id="player" 시작값) + **2.2 Runtime sync** (`POST /api/world/persons/sync`, emotion 보존). |
| **Worldbuilding Phase 3** | ✅ 완료 (2026-05-01) | Place 도메인 (settlement+geography 2 layer) — sect/geography_refs 양방향 + `parent_place` cycle. Phase 1·2 외래키 일제히 에러 승격 (`headquarters`/`birthplace`/`current_location` 등 0건 보장). 11 Place. |
| **Worldbuilding Phase 4** | ✅ 완료 (2026-05-01) | Atlas 첫 관계 도메인 (도메인+뷰 이중성) — `references` ↔ Place FK + `place_atlas_refs` 양방향 인덱스 + view 메서드 (places_in 등 N+1 회피용 `get_places_batch`). atlas-jungwon ASCII 다이어그램. `EventHandlerContext::get_npc/relationship/emotion_state` 헬퍼 중앙화 (commit `9ff5645`). |
| **Worldbuilding Phase 5a** | ✅ 완료 (2026-05-02) | Event 도메인 (두 번째 인스턴스) — 6 Event + `participants.{persons,groups,places}` 외래키 0건 + `related_events` 양방향 + alias 패턴 일관. MCP/REST `world_events` 도구. bloody-night ingest E2E. |
| **Worldbuilding Phase 5b** | ✅ 완료 (2026-05-02) | Era + Timeline + Atlas overlay (View trait 보류) — 5 Era + 1 Timeline + view 메서드 4종 + Atlas overlay 양방향 + 외래키 0건. Phase 4·5a `era_id` 외래키 활성. |
| **Worldbuilding Phase 5c.1** | ✅ 완료 (2026-05-02) | Historical NPCs follow-up — 임서운 + 7 historical/active npc 정밀 매핑 + Phase 5a Event 외래키 갱신 (핵심 분기 0건). 직교 플래그 + `extras.secret` 컨벤션 정형화. |
| **Worldbuilding Phase 5c.2** | ✅ 완료 (2026-05-03) ★ **Phase 5 시리즈 종결** | Mid-era Events follow-up — 6 mid-era event(founding/prosperity/turning/decline) + era key_events 4종 갱신 + Phase 5a 6 event related_events 역방향 정합 + 5c.1 npc 외래키 활성 + 신규 kind 5종 도입. Dynamic FK validation. |
| Worldbuilding Phase 6+ | ⏳ 예정 | Skill → Item → Knowledge → Lore. Roadmap: [`docs/tasks/world building/00-roadmap.md`](docs/tasks/world%20building/00-roadmap.md). |
| **Hexagonal refactor** | ✅ 완료 (2026-05-02, `76237c2`) | `src/ports.rs` → 7-모듈 ISP 분할 (`ports/{persistence,personality,guide,memory,analysis,chat,monitoring}`). `src/application/dto.rs` → 도메인별 7-모듈 분할. `LlamaServerMonitor`/`LlamaTimings`/`LlamaHealth`/`LlamaSlotInfo`/`LlamaMetrics` → `InferenceServerMonitor`/`InferenceTimings`/`ServerHealth`/`InferenceSlotInfo`/`ServerMetrics` (인프라 누출 제거). `MemoryAugmentationService` 도메인 추출. `MindRepository` + `EventHandlerContext`에 ISP 적용. 357 단위 테스트 0 회귀. |
| **Performance & safety pass** | ✅ 완료 (2026-05-03, `11eccbb`) | `Command` 64% 축소 (248B→88B) + `EventPayload` 34% 축소 (280B→184B) — 전략적 boxing. `SqliteMemoryStore::search_by_meaning` N+1 제거 (SQL JOIN). `EmotionState::iter_active()` zero-allocation + emotion `as_str()`. `into_domain` 소유권 기반 DTO 변환. `unwrap()` → idiomatic 에러 핸들링. Prometheus parser single-pass (>90% CPU 절감). |
| **에러 처리 + LLM 타임아웃** | ✅ 완료 (`166bde6`) | `RigChatAdapter::with_timeout(Duration)` 빌더 (기본 60s) + `ConversationError::Timeout(Duration)` variant + `AppError::Dialogue(Conversation(Timeout))` → HTTP 504. handler 레이어가 `?` operator로 `MindServiceError`/`DispatchV2Error`/`DialogueOrchestratorError`를 단일 `AppError`로 흡수. `tests/orchestrator_error_propagation_test.rs` + `tests/rig_chat_timeout_test.rs` 신설. |
| **Unit of Work pattern** | ✅ 완료 (`bb746c2`) | `UnitOfWork<'a, R>` 도입 (`application/command/uow.rs`) — Transactional BFS 동안 변경된 애그리거트를 dirty checking으로 추적하고 `commit()`로 일괄 반영. `EventHandler::handle()` → `handle_v2(&mut dyn DynamicHandlerContext)`로 시그니처 변경(타입 은닉). `EventHandlerContext<'a, 'b, R>` 이중 수명 도입(borrow-checker + deadlock 회피). `HandlerShared`는 `DispatchV2Output.shared` 출력 쉐이프로만 잔존. dispatcher 트랜잭션 수명주기 정교화 (Transactional → UoW.commit → EventStore append → Inline projection(별도 UoW) → Fanout). |
| **Memory keyword search 구현** | ✅ 완료 (`dbf5624`) | `SqliteMemoryStore::search_by_keyword`가 FTS5(trigram) 기반으로 실 구현 (이전: 빈 반환). `#[deprecated(since="0.4.0")]` 마킹은 유지(권장 경로는 `MemoryStore::search(MemoryQuery {..})`). `InMemoryMemoryStore` search 로직 버그 수정 + memory_projector_test 회복. |
| **Mind Architecture Phase 1 (Reflection + Chitchat Gate)** | ✅ 완료 (2026-05-11, `c3b3e21` → `c7e1ac4`) | `relationships.md` v0.7 §6 Scene Boundary Reflection. `domain/reflection.rs` (TurnSnapshot + compute_significance + ReflectionResult) + `Npc.inner_compass: Option<String>` (A-min) + `EventKind::DialogueReflected` + `Command::EndDialogue.reflection` + `ports/reflection.rs::ReflectionPort` + `adapter/reflection_via_chat.rs::ConversationBackedReflectionPort` + `application/reflection_service.rs::ReflectionService<P>` (OCP, Mock 5 cases) + `RelationshipPolicy.handle_dialogue_end` 게이트 (4 follow-up: DialogueReflected → 조건부 RelationshipUpdated → EmotionCleared → SceneEnded) + `DialogueOrchestrator.with_reflection() + turn_buffers` + `MAX_EVENTS_PER_COMMAND` 22로 인상 + `AfterDialogueResponse.reflection` 필드 (chitchat 호환 fallback). 회귀 1095 passed. chitchat 18% latency 절감 (게이트 효과). Calibration 3 밴드 정확 (0.000 / 0.461 / 0.980). 3 narrative validation 시나리오 (`data/scenarios/phase1-validation/`). 상세: [`docs/tasks/mind-architecture/phase1-checkpoint-report.md`](docs/tasks/mind-architecture/phase1-checkpoint-report.md). |
| Phase 5 (npc-mind) | 미구현 | StoryAgent (서사 진행 판단) |
| Phase 6 (npc-mind) | 미구현 | Tool 시스템 (ToolRegistry) |
| Phase 7 (npc-mind) | 미구현 | WorldKnowledgeStore (세계관 정적 지식) — Worldbuilding Phase 6+가 이를 흡수 검토 |
| Phase 8 (npc-mind) | 미구현 | SummaryAgent (컨텍스트 윈도우 관리) |

전체 B안 설계 참조: [`docs/architecture/b-plan-implementation.md`](docs/architecture/b-plan-implementation.md).

## 개발 컨벤션

### DTO 분리 (Result / Response)
- `*Result` (도메인): `ActingGuide` 포함, 포맷팅 전. 도메인 엔진(`AppraisalEngine`/`StimulusEngine`) 내부 타입
- `*Response` (포맷팅 완료): `prompt: String` 포함. `DispatchV2Output` → `DialogueOrchestrator`/`domain_sync` 헬퍼가 formatter 적용해 생성
- `ChatResponse` (chat 포트): `text + timings`. `ConversationPort`가 반환
- 변환: `result.format(&formatter)` → `Response` (`CanFormat` 트레이트)

### 네이밍 (DDD)
- Domain Services: `~Engine` / `~Analyzer`
- Application Services: `~Service`
- Ports: 행위 명사 (`src/ports/`)
- Domain Events: 과거형

### 에러 처리
- 서비스 계층: `MindServiceError` (`application::error`) 반환 — NpcNotFound/RelationshipNotFound/InvalidSituation/EmotionStateNotFound/LocaleError (5 variant)
- dispatch 계층: `DispatchV2Error` (`CommandDispatcher`) — HandlerFailed/CascadeTooDeep/EventBudgetExceeded/InvalidSituation
- 핸들러 계층: `HandlerError` (`application::command::handler_v2`) — NpcNotFound/RelationshipNotFound/EmotionStateNotFound/InvalidInput/Infrastructure/Repository
- 대화 계층: `ConversationError` — ConnectionError/SessionNotFound/InferenceError/Timeout(Duration)
- 모니터링: `MonitoringError` — Connection/HttpStatus/Parse/Other
- 웹 계층(`mind-studio`): `AppError` (`From<MindServiceError>` + `From<DispatchV2Error>` + `From<DialogueOrchestratorError>` + `From<MonitoringError>` + `From<WorldError>` + `From<DirectorError>`) → variant별 HTTP 상태 자동 매핑(`IntoResponse`). Timeout → 504, Connection → 502, NotFound → 404, InvalidSituation → 400, invariant 위반 → 500.

### 데이터 변환 (Mapping)
- DTO(`SituationInput` 등)는 `SituationService`를 통해 도메인 모델로 변환
- DTO는 저장소 의존성 없는 순수 데이터 구조체
- 서비스가 저장소(`MindRepository`)에서 관계/오브젝트 정보를 조회하여 변환 시 주입

### 테스트 (TestContext)
- 모든 통합 테스트는 `tests/common/mod.rs`의 `TestContext`를 사용
- 캐릭터 생성 / 저장소 초기화 중복 코드 방지, 일관된 테스트 환경 보장

## 용어 정의

| 용어 | 영문 | 정의 | 관련 Command |
|------|------|------|----------|
| **장면** | Scene | 하나의 연속된 대화 단위. 시작과 끝이 있음 | `Command::StartScene` / `Command::EndDialogue` |
| **비트** | Beat | 장면 안에서 감정 흐름이 전환되는 시점 | `Command::Appraise` / `BeatTransitioned` follow-up |
| **대사** | Utterance | 실제 캐릭터가 말하는 한 줄의 대사 | `Command::ApplyStimulus` 입력 |

## Scene Focus 시스템

게임이 Scene 시작 시 Focus 옵션 목록을 제공하고, 엔진이 stimulus 처리 중 감정 상태 조건(`FocusTrigger`)을 평가하여 자동으로 Beat 전환을 판단합니다. Beat 전환 로직은 `Command::ApplyStimulus` → `StimulusPolicy`에서 처리되며, `BeatTransitioned` 이벤트를 follow-up으로 발행합니다.

### 데이터 구조
- `Scene`: 도메인 애그리거트 루트 (npc_id, partner_id, focuses, active_focus_id)
- `SceneFocus`: Focus 옵션 (id, description, trigger, event/action/object)
- `FocusTrigger`: `Initial`(즉시 적용) 또는 `Conditions`(감정 조건)
- `EmotionCondition`: 감정 유형 + 임계값 (`Below`/`Above`/`Absent`)
- 조건 구조: `OR [ AND[...], AND[...] ]` — 외부 배열 OR, 내부 배열 AND

### Scene 애그리거트 메서드
- `Scene::new(npc_id, partner_id, focuses)` — 생성
- `Scene::check_trigger(&state)` — 대기 Focus 중 조건 충족된 것 반환
- `Scene::set_active_focus(focus_id)` — 활성 Focus 설정
- `Scene::initial_focus()` — `Initial` 트리거를 가진 Focus 검색

### Beat 전환 흐름

```
Command::ApplyStimulus → StimulusPolicy.handle()
  → 1. 감정 강도 조정 (관성 적용) → StimulusApplied (follow-up)
  → 2. scene.check_trigger(&state) — 대기 중 Focus의 조건 체크
  → 3. 조건 충족 시 → transition_beat():
       a. update_beat_relationship() — 관계 갱신 (감정 유지)
       b. scene.set_active_focus() + 새 Focus로 appraise
       c. merge_from_beat (이전 감정 + 새 감정 합치기)
       d. BeatTransitioned (follow-up, partner_id 포함 — B4 S3 Option A)
  → 4. HandlerShared.scene 갱신 → apply_shared_to_repository에서 save_scene
```

### 감정 합치기 (merge_from_beat)
- 같은 감정: max 기준으로 강도 + context 유지
- 이전 감정 중 `BEAT_MERGE_THRESHOLD`(0.2) 미만: 소멸
- 새 감정만 있으면: 그대로 추가

## Stimulus 관성 공식

```
inertia = max(1.0 - intensity, STIMULUS_MIN_INERTIA)
delta = pad_dot × absorb_rate × STIMULUS_IMPACT_RATE × inertia
```

- 강한 감정(intensity 높음) → inertia 작음 → 자극에 덜 흔들림
- 약한 감정(intensity 낮음) → inertia 큼 → 자극에 쉽게 변함
- intensity=1.0이어도 최소 관성(0.30)으로 변동 보장

## 튜닝 프로파일 (주요, 전체는 `src/domain/tuning.rs` 참조)

도메인 정책 파라미터는 `TuningProfile` 구조체로 관리되며 `tuning::profile()`로 조회합니다.
프로세스 시작 시 1회 `tuning::install(TuningProfile { ... })`로 주입 가능 (미설치 시 `Default`).

| TuningProfile 필드 | 기본값 | 용도 |
|------|-----|------|
| `stimulus_impact_rate` | 0.5 | stimulus 감정 변동 계수 |
| `stimulus_min_inertia` | 0.30 | 관성 최소값 (intensity=1.0에서도 반응 보장) |
| `beat_merge_threshold` | 0.2 | Beat 합치기 시 이전 감정 소멸 기준 |
| `trust_update_rate` | 0.1 | 신뢰 갱신 계수 |
| `closeness_update_rate` | 0.05 | 친밀도 갱신 계수 |
| `significance_scale` | 3.0 | 상황 중요도 배율 (sig=1.0 → 4배) |
| `emotion_threshold` | 0.2 | 감정 유의미 판단 기준 (가이드 반영) |
| `trait_threshold` | 0.3 | 성격 특성 추출 임계값 |

`DAY_MS`는 시간 단위 상수로 `pub const DAY_MS: u64`로 별도 노출됩니다 (튜닝 대상 아님).

### 마이그레이션 — 이전 `pub const` API에서 전환 (Breaking Change)

리뷰 #1 후속으로 모든 튜닝 `pub const`가 `TuningProfile` 필드로 이동됨. 외부 사용자 코드 전환 필요:

```rust
// 이전 (compile error 발생)
use npc_mind::domain::tuning::STIMULUS_IMPACT_RATE;
let rate = STIMULUS_IMPACT_RATE;

// 이후
use npc_mind::domain::tuning::profile;
let rate = profile().stimulus_impact_rate;

// 값을 바꿀 때 (프로세스 시작 시 1회)
use npc_mind::domain::tuning::{install, TuningProfile};
install(TuningProfile {
    stimulus_impact_rate: 0.7,
    ..Default::default()
}).expect("once only");
```

상수명 → 필드명 규칙: `SCREAMING_SNAKE` → `snake_case` (예: `BEAT_MERGE_THRESHOLD` → `beat_merge_threshold`).
`DAY_MS`는 그대로 유지. `install()`은 `validate()` 통과 시에만 성공하며, 두 번째 호출은 `InstallError::AlreadyInstalled` 반환.

파일별 로컬 상수(`personality.rs`의 `W_STANDARD`/`BASE_*`/`CLAMP_*` 등, `pad_table.rs`의 22개 감정별 PAD 좌표)는 해당 파일 상단에 정의되어 있습니다.

## Mind Studio (개발 도구)

Claude(API)와 Bekay(브라우저)가 동시에 사용하는 심리 엔진 시뮬레이터. Mind Studio handlers는 `domain_sync` 모듈을 경유해 **v2 `dispatch_v2`** 경로로 동작합니다 (B5.2 (2/3) 이후).

### 아키텍처

- **백엔드**: Axum REST API + SSE MCP 서버 (`src/bin/mind-studio/`)
- **프론트엔드**: Vite + React 18 + TypeScript + Zustand (`mind-studio-ui/`)
- 빌드 출력이 `src/bin/mind-studio/static/`에 배치되어 Axum `ServeDir`로 서빙
- **실시간 동기화**: `broadcast` 채널 → SSE `GET /api/events` → 프론트엔드 `EventSource`
  - MCP 도구 호출 또는 REST 핸들러가 상태 변경 시 `StateEvent` emit
  - 프론트엔드 `useStateSync` 훅이 이벤트 종류별 targeted re-fetch로 Zustand 업데이트
  - 이벤트 누락(lagged) 시 `resync` → 전체 refresh fallback

### 도메인 동기화 (`domain_sync.rs`) — B5.2 (3/3)

`AppState.shared_dispatcher` (with_default_handlers 적용됨)가 request 간
재사용되며, 공유 `Arc<Mutex<InMemoryRepository>>`를 내부 소유한다.

**dispatch 경로** (appraise/stimulus/after_dialogue/guide/start_scene):
1. `state.inner.write().await` 획득
2. `state.shared_dispatcher.dispatch_v2(cmd).await` — EventHandler 체인 실행
3. `HandlerShared` + `output.events` → UI DTO 재구성
4. `sync_from_repo(&shared_repo, &mut inner)` — 갱신된 관계/감정/Scene을 UI 레이어로 역반영

**UI CRUD 경로** (POST/PUT/DELETE NPC·관계·오브젝트, scenario load):
- inner에 변경 적용 후 `state.rebuild_repo_from_inner().await` 호출
- StateInner의 도메인 데이터를 공유 repo로 reset+rebuild (drift 불가능)
- `impl_crud_handlers!` 매크로가 자동으로 호출하므로 REST/MCP CRUD는 투명
- 재구성 대상: NPCs · Relationships · Objects · Emotions · Scene (부착 시점 기준 전부)

**공개 helper** (`crate::domain_sync::*`):
- `dispatch_appraise`, `dispatch_stimulus`, `dispatch_end_dialogue`, `dispatch_generate_guide`, `dispatch_start_scene`
- `dispatch_tell_information`, `dispatch_apply_world_event`, `dispatch_seed_rumor`, `dispatch_spread_rumor` (Step E1)
  — 시그니처: `(state: &AppState, inner: &mut StateInner, req) -> Result<...>`
- `sync_from_repo(&InMemoryRepository, &mut StateInner)` — dispatch 후 역반영

공유 repo 재구성 entrypoint: `AppState::rebuild_repo_from_inner()`.

**성능**: per-request snapshot·ephemeral dispatcher·Arc 재생성 모두 제거.
UI write 시점에만 repo 재구성 비용 발생.

**알려진 한계**: `shared_dispatcher`가 내부 소유한 `InMemoryEventStore`는
프로세스 수명 동안 모든 이벤트를 누적한다. 이전 ephemeral 패턴은 request마다
store를 drop했으나 공유 dispatcher는 그렇지 않다. Mind Studio는 dev tool이라
실용상 문제 없지만 장기 실행 시 메모리 증가와 `next_sequence` O(N) scan
부하가 늘어난다. 영구 store (Phase 8+) 도입 시 해소 예정.

### 실행 방법

```bash
# 프론트엔드 빌드 (최초 1회 또는 UI 변경 시)
cd mind-studio-ui && npm install && npm run build

# Axum 서버 실행 (빌드된 UI 포함)
cargo run --features mind-studio,chat,embed --bin npc-mind-studio  # http://127.0.0.1:3000

# 프론트엔드 개발 모드 (HMR, API proxy → Axum 3000)
cd mind-studio-ui && npm run dev  # http://localhost:5173

# 프론트엔드 테스트
cd mind-studio-ui && npm test
```

### 프론트엔드 구조 (`mind-studio-ui/`)

```
src/
  App.tsx               레이아웃 셸 (스토어 연결)
  api/client.ts         fetch wrapper (get/post/put/del/postJson)
  stores/               Zustand 스토어 5개 (Entity, UI, Result, Chat, Scene)
  handlers/             비즈니스 로직 (appHandlers, loadHandlers)
  hooks/                useToast, useRefresh, useChatPolling, useAutoSave, useStateSync
  components/
    sidebar/            NPC/관계/오브젝트 목록
    modals/             NpcModal, RelModal, ObjModal
    situation/          SituationPanel, FocusEditor
    chat/               ChatPanel (SSE 스트리밍)
    result/             ResultPanel + 10개 서브뷰
  types/index.ts        공유 TypeScript 타입
  __tests__/            Vitest 테스트 (스토어/핸들러/API/훅)
```

### 주요 기능
- NPC/관계/오브젝트 CRUD, 감정 평가, 가이드 생성, 대사→PAD 자동 분석(embed), 시나리오 로드/세이브, 턴 히스토리, 테스트 레포트
- **Scene Focus 패널**: 시나리오 JSON에 정의된 Focus 옵션 목록을 읽기 전용으로 표시 (활성/대기 상태, trigger 조건, test_script)
- **Beat 전환 표시**: stimulus 결과에서 Beat 전환 발생 시 시각적 배너
- **테스트 스크립트**: 각 Beat의 `test_script` 대사 목록을 Focus 패널에 표시하고, 대화 입력 영역에서 '스크립트 전송' 버튼으로 순차 전송 가능
- **LLM 대화 테스트**(`chat` feature): 로컬 LLM과 다턴 대화, Beat 전환 시 system prompt 동적 갱신
- **LLM 서버 모니터링**(`chat` feature): `/api/llm/status`로 llama-server 상태(health/slots/metrics) 통합 조회
- **Memory/Rumor 조회·주입**(`embed` feature, Memory Step E1): `AppState`가 `shared_dispatcher`에 `with_memory_full` + `with_rumor` 자동 부착 (`NPC_MIND_MEMORY_DB` 환경변수로 DB 경로, 미설정 시 in-memory SQLite). 엔드포인트 10종 — `GET /api/memory/{search,by-npc/{id},by-topic/{topic},canonical/{topic}}` · `POST /api/memory/{entries,tell}` · `POST /api/world/apply-event` · `GET /api/rumors` · `POST /api/rumors/{seed,{id}/spread}`. 프런트엔드 UI는 Step E2에서 별도 진행.
- **시나리오 시드 조회 + 로드 경고**(Memory Step E3.3): `GET /api/scenario-seeds`로 `StateInner.scenario_seeds` 반환. "시드" 탭에서 `initial_rumors`/`world_knowledge`/`faction_knowledge`/`family_facts` 4 섹션 read-only 표시(변종 라벨·메모리 메타 뱃지). 시나리오 로드 시 `applied_*` count를 토스트에, `warnings`를 error 토스트(3건 초과 시 묶음+console.warn)로 노출. 편집은 JSON 직접 편집 또는 소문 탭의 런타임 시드 폼.
- **실시간 상태 동기화**: `tokio::sync::broadcast` → SSE `/api/events` → `EventSource` (useStateSync 훅). MCP/REST 상태 변경이 UI에 자동 반영. `StateEvent`는 NPC/Relationship/Scene/Chat 외에 `MemoryCreated/Superseded/Consolidated`·`RumorSeeded/Spread`(Step E1) 포함.
- **Worldbuilding 조회**(`embed` feature): `NPC_MIND_WORLD_DB` 환경변수로 `world-load`가 빌드한 SQLite를 부착하면 4 도메인 조회 + 자동 Person sync 활성화. 엔드포인트: `GET /api/world/{groups,persons,places,atlases}` 목록·`/search?q=`·`/{id}` 단건 + `POST /api/world/persons/sync` 런타임 재동기화. 부팅 시 `kind in {active, player}` Person을 인메모리 `MindRepository`에 자동 등록 (emotion/scene 보존).
- **인과 사슬 추적**: `GET /api/projection/trace/{correlation_id}` — 한 `dispatch_v2` 호출의 모든 이벤트(parent_event_id 트리)를 묶어 반환. DeepEval Phase 1 trace 단위 입력으로 사용 가능.
- REST API 엔드포인트 전체는 `src/bin/mind-studio/handlers/` 참조

## LLM 대화 테스트 (`chat` feature)

Mind Engine이 생성한 프롬프트를 실제 LLM에 system prompt로 주입하고 다턴 대화로 NPC 연기 품질을 검증합니다.

- **ConversationPort** (`ports/chat.rs`): LLM 대화 세션 추상화 — `start_session`, `send_message`, `update_system_prompt`, `end_session`
  - `send_message()` / `send_message_stream()`은 `ChatResponse { text, timings: Option<InferenceTimings> }` 반환
  - `ConversationError::Timeout(Duration)` variant — `RigChatAdapter::with_timeout()` 적용 시 발생 (Mind Studio가 504 Gateway Timeout으로 매핑)
- **RigChatAdapter** (`adapter/rig_chat.rs`): rig-core 0.33 `openai::CompletionsClient<TimingsCapturingClient>` 기반 구현. 세션별 system_prompt + rig_history + dialogue_history 관리. `with_timeout(Duration)` 빌더 (기본 60s). `InferenceServerMonitor` 구현도 포함 (이전 `LlamaServerMonitor`에서 일반화)
- **TimingsCapturingClient** (`adapter/llama_timings.rs`): rig의 `HttpClientExt` 래퍼. HTTP 응답에서 llama-server `timings`를 캡처하여 `ChatResponse`에 포함. rig 소스 수정 없이 `ClientBuilder.http_client()`로 주입. `with_client()`로 외부 `reqwest::Client` 주입 지원
- **DialogueOrchestrator** (`application/dialogue_orchestrator.rs`): `CommandDispatcher` + `ConversationPort` 오케스트레이터. `start_session`/`turn`/`end_session` API
- **dialogue_test_service.rs**: Mind Studio ↔ DialogueOrchestrator DTO (`Chat*Request`/`Chat*Response`) 전용. 오케스트레이션 struct는 없음 (v0.3.0에서 제거됨)

### llama-server Timings 캡처

llama-server는 `/v1/chat/completions` 응답에 `timings` 객체(prompt/predicted 처리 속도)를 포함한다.
rig-core의 OpenAI 응답 타입은 이 필드를 무시하므로, `TimingsCapturingClient`가 HTTP 계층에서 가로챈다.

```
[llama-server] → JSON 응답 (timings 포함)
       ↓
[TimingsCapturingClient] → timings 파싱 & 저장 → Arc<Mutex<Option<InferenceTimings>>>
       ↓ (body 그대로 전달)
[rig CompletionModel] → CompletionResponse 파싱 (timings 무시)
       ↓
[RigChatAdapter] → ChatResponse { text, timings }
```

- **Non-streaming** (`send()`): 응답 body 전체를 읽어 `timings` 추출 후 rig에 전달
- **Streaming** (`send_streaming()`): SSE 청크를 래핑하여 `"timings"` 포함 청크에서 캡처
- **주요 타입**: `InferenceTimings` (8개 필드 — `prompt_n/ms/per_token_ms/per_second` + `predicted_*` 동일), `ChatResponse { text, timings: Option<InferenceTimings> }`

### 추론 서버 모니터링 (`InferenceServerMonitor`)

llama-server 등 OpenAI-compatible 추론 서버는 Chat Completions 외에 서버 관리용 엔드포인트를 제공한다.
`InferenceServerMonitor` 포트 트레이트(이전 `LlamaServerMonitor`에서 일반화)가 이를 추상화하고, `RigChatAdapter`가 구현한다.

| 메서드 | 엔드포인트 | 반환 타입 | 용도 |
|--------|-----------|-----------|------|
| `health()` | `GET /health` | `ServerHealth` | 서버 상태 (`ok`, `loading model` 등) |
| `slots()` | `GET /slots` | `Vec<InferenceSlotInfo>` | 슬롯별 idle/processing 상태, 토큰 수 |
| `metrics()` | `GET /metrics` | `ServerMetrics` | Prometheus 메트릭 (KV 캐시, 처리 속도 등 — single-pass parser) |

**URL 관리**: `base_url` (`http://host:port/v1`)에서 `/v1`을 제거하여 `server_url` (`http://host:port`)을 도출. 모니터링 엔드포인트는 `/v1` 없이 root 경로를 사용한다.

**커넥션 풀 공유**: `RigChatAdapter`가 단일 `reqwest::Client`를 생성하여 rig 통신(`/v1/chat/completions`), 모델 감지(`/v1/models`), 모니터링(`/health`, `/slots`, `/metrics`) 모두에 공유한다. `TimingsCapturingClient::with_client()`로 주입.

```
[RigChatAdapter]
  ├─ http_client: reqwest::Client  ← 단일 클라이언트 (공유 커넥션 풀)
  ├─ CompletionsClient<TimingsCapturingClient>  ← rig용 (같은 풀)
  ├─ refresh_model_info() → GET /v1/models      ← 같은 풀
  └─ health/slots/metrics → GET /health 등       ← 같은 풀
```

**Mind Studio REST 엔드포인트** (`handlers/llm.rs`):
- `GET /api/llm/status` — 통합 상태 (health + model + slots + metrics, 부분 실패 허용)
- `GET /api/llm/health` — 헬스 체크
- `GET /api/llm/slots` — 슬롯 상태
- `GET /api/llm/metrics` — Prometheus 메트릭 (파싱 + 원문)

대화 루프 (DialogueOrchestrator 기준):
```
start_session(sid, npc, partner, situation?)
  → Command::Appraise.dispatch_v2 → AppraiseRequested → EmotionAppraised → GuideGenerated
  → ConversationPort.start_session(prompt)

turn(sid, utterance, pad?, sit_desc?)
  → Command::ApplyStimulus.dispatch_v2 → StimulusApplyRequested → StimulusApplied (+ BeatTransitioned?)
  → BeatTransitioned 발생 시 ConversationPort.update_system_prompt
  → ConversationPort.send_message → ChatResponse { text, timings }

end_session(sid, significance?)
  → ConversationPort.end_session
  → (significance 있으면) Command::EndDialogue.dispatch_v2 → RelationshipUpdated + SceneEnded + EmotionCleared
```

## 외부 문서 인덱스

- **API 레퍼런스**: [`docs/api/api-reference.md`](docs/api/api-reference.md) — 공개 API, DTO, 포트, 도메인 타입
- **통합 가이드**: [`docs/api/integration-guide.md`](docs/api/integration-guide.md) — 외부 프로젝트 통합 단계별 가이드
- **아키텍처 v2**: [`docs/architecture/architecture-v2.md`](docs/architecture/architecture-v2.md)
- **아키텍처 v3 (EventBus/CQRS)**: [`docs/architecture/system-design-eventbus-cqrs.md`](docs/architecture/system-design-eventbus-cqrs.md) — EventBus, CQRS, Event Sourcing, Multi-Handler, RAG 시스템 디자인
- **dispatch_v2 internals**: [`docs/architecture/dispatch-v2-internals.md`](docs/architecture/dispatch-v2-internals.md) — BFS cascade · correlation_id · parent_event_id · safety bounds
- **EventHandler 카탈로그**: [`docs/architecture/event-handler-catalog.md`](docs/architecture/event-handler-catalog.md)
- **프론트엔드 아키텍처**: [`docs/architecture/frontend-architecture.md`](docs/architecture/frontend-architecture.md) — Vite+React+Zustand 구조, 스토어 설계, 데이터 흐름, 컴포넌트 트리
- **협업 워크플로우**: [`docs/collaboration-workflow.md`](docs/collaboration-workflow.md)
- **Worldbuilding 로드맵**: [`docs/tasks/world building/00-roadmap.md`](docs/tasks/world%20building/00-roadmap.md) — 10 Phase 흐름 + Phase별 task/checkpoint report (`docs/tasks/archive/` 종결분 보존)
- **Mind 아키텍처 마이그레이션 로드맵**: [`docs/tasks/mind-architecture/00-roadmap.md`](docs/tasks/mind-architecture/00-roadmap.md) — Phase 1/2/3a/3b/3c (Reflection·4축·BondKind·Channel·ActionTrigger). relationships.md v0.7 동반 트랙.
- **Phase 1 task spec**: [`docs/tasks/mind-architecture/task-rel-phase1-reflection.md`](docs/tasks/mind-architecture/task-rel-phase1-reflection.md) — Scene Boundary Reflection 도입. 6 stage (Stage 0 Pre-flight Impact Analysis 포함), OCP 준수 (`ReflectionPort` trait + `ConversationBackedReflectionPort` 어댑터), 3 narrative validation 시나리오. **✅ 완료 (2026-05-11)** — checkpoint report 참조.
- **Phase 1 checkpoint report**: [`docs/tasks/mind-architecture/phase1-checkpoint-report.md`](docs/tasks/mind-architecture/phase1-checkpoint-report.md) — Stage 0~5 종결. 회귀 1095 passed, chitchat 18% latency 절감, calibration 3 밴드 정확.
- **Phase 1 kickoff 가이드**: [`docs/tasks/mind-architecture/PHASE1-KICKOFF.md`](docs/tasks/mind-architecture/PHASE1-KICKOFF.md) — Claude Code 인계용 진입 가이드. 작업 시작 순서 + Tier 권한 + 체크포인트 보고 형식.
- **Phase 1 API 변경 안내**: [`docs/changes/phase1-mind-architecture.md`](docs/changes/phase1-mind-architecture.md) — 외부 사용자용 breaking change 모음 (`Command::EndDialogue.reflection` 필드 / `Npc.inner_compass` / `MAX_EVENTS_PER_COMMAND` 22 / 환경 이슈 5건).
- **감정 엔진**: [`docs/emotion/`](docs/emotion/) — OCC 모델, HEXACO 매핑, PAD 앵커 매트릭스, appraisal 엔진 설계
- **Listener-perspective 변환** (Phase 7): [`docs/emotion/sign-classifier-design.md`](docs/emotion/sign-classifier-design.md) (부호/강도 분류기 설계 + §3.7 Register 전략) + [`docs/emotion/phase7-converter-integration.md`](docs/emotion/phase7-converter-integration.md) (프로덕션 통합, **Step 1-5+ 완료** — 88% baseline, default-on, DialogueOrchestrator · Mind Studio 통합, §6.1 테스트 카탈로그 71개)
- **성격 모델**: [`docs/personality/`](docs/personality/) — HEXACO 6차원 facet 상세
- **가이드 매핑**: [`docs/guide/guide-mapping-table.md`](docs/guide/guide-mapping-table.md)
- **메모리 시스템**: [`docs/memory/`](docs/memory/) — Step A-E 구현 설계 + Memory Scope/Source/Layer/Provenance VO + framing
- **테스트 스크립트**: `mcp/skills/npc-scenario-creator/SKILL.md` (4-1단계) + `mcp/skills/npc-mind-testing/SKILL.md` (원칙 4, 커서 관리)
- **언어 설정**: [`docs/locale-guide.md`](docs/locale-guide.md)
- **MCP 서버 설정**: `.mcp.json` (프로젝트 루트)
