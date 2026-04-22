# CLAUDE.md

NPC Mind Engine — HEXACO 성격이 OCC 감정을 생성하고, LLM이 연기할 수 있도록 가이드를 출력하는 Rust 라이브러리.

라이브러리 형태로 배포되며, `Director`/`CommandDispatcher::dispatch_v2`가 유일한 진입점입니다.
v0.3.0에서 v1 경로(`MindService`/`EventAwareMindService`/`Pipeline`/`CommandDispatcher::dispatch`/`shadow_v2`)는
전부 제거되었습니다.

## 기술 스택
- **Language:** Rust (Edition 2024)
- **Architecture:** Hexagonal + DDD + **EventBus(tokio broadcast)/CQRS/Event Sourcing** + **Multi-Agent**
- **Libraries:** `serde`/`serde_json`, `thiserror`, `tokio/sync`+`tokio-stream`+`futures` (EventBus 내부 구현), `axum`(WebUI), `tracing`, `ort`(ONNX 임베딩), `rig-core`(LLM Agent 대화), `rusqlite`+`sqlite-vec`(RAG 저장소 [embed] — FTS5 + vec0 벡터 인덱스), `regex` (`listener_perspective` feature, default-on — 한국어 정규식 프리필터)
- **런타임 정책:** 코어는 `tokio::sync`의 `broadcast`만 내부 구현으로 사용. 공개 API는 `futures::Stream` 타입만 노출하므로 **호출자는 tokio를 deps에 추가할 필요 없음**(Bevy 등 임의 async 런타임에서 Stream 소비 가능). `chat`/`mind-studio` feature가 tokio `rt-multi-thread` 런타임을 추가 활성화. `embed` feature는 sqlite-vec이 순수 C 확장이라 tokio 런타임을 전이시키지 않는다.

## 빌드 & 테스트

```bash
cargo build                        # 기본 빌드
cargo build --features embed       # 임베딩 포함 (bge-m3-onnx-rust)
cargo build --features chat        # LLM 대화 에이전트 포함 (rig-core)
cargo test                         # 기본 테스트
cargo test --features embed        # 전체 테스트 (임베딩 포함)
cargo test --features "embed listener_perspective"  # Phase 7 Converter 포함 엔드투엔드
cargo test --features listener_perspective --lib domain::listener_perspective  # 39 도메인 단위
cargo test --no-default-features --features chat --test dialogue_no_lp_passthrough  # Phase 7 Step 5: LP off 회귀 감시

# 개별 테스트는 tests/ 디렉토리 참조
# PAD 벤치마크(pad_benchmark_test 등)는 --features embed 필요

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
MIND_STUDIO_PORT=3000                         # 서버 포트 [mind-studio feature]
```

### 빌드 주의사항 (Windows)

- `--features embed`: ort(ONNX Runtime) 정적 링크를 위해 `.cargo/config.toml`에서 CRT를 동적으로 통일해야 함. 변경 후 `cargo clean` 필수.
- `--features chat`: rig-core 기본 TLS 백엔드(`aws-lc-sys`)가 MSVC에서 `__builtin_bswap` 링크 실패. Cargo.toml에서 `default-features = false, features = ["reqwest-native-tls"]` 사용.
- rig 0.33+ OpenAI provider는 기본 Responses API(`/v1/responses`) 사용. llama.cpp 등 로컬 서버는 Chat Completions만 지원하므로 `.completions_api()` 호출 필수.

## 프로젝트 구조 (주요 디렉토리)

```
src/
  application/    어플리케이션 계층, 라이브러리 진입점 (v2 단일 경로)
                  - error.rs (MindServiceError — 공용 서비스 에러)
                  - dto.rs (Appraise/Stimulus/Guide/AfterDialogue Request·Response)
                  - event_store.rs, event_bus.rs (Event Sourcing 인프라)
                  - projection.rs (EmotionProjection/RelationshipProjection/SceneProjection 구조체 — v2 wrapper가 재사용)
                  - memory_agent.rs (EventBus 구독 기억 인덱싱 [embed])
                  - scene_service.rs, situation_service.rs (도메인 조립 helper)
                  - dialogue_agent.rs [chat] (LLM 다턴 오케스트레이터)
                  - dialogue_test_service.rs [chat] (DTO 전용 — Chat*Request/Response)
                  - director/ (B안 B4 — 다중 Scene facade)
                    - mod.rs (Director: start_scene / end_scene / dispatch_to / active_scenes)
                    - scene_task.rs (spawn_scene_task — mpsc 루프)
                    - spawner.rs (Spawner trait — runtime-agnostic)
                  - command/ (CQRS Command Side — v2 단일 경로)
                    - types.rs (Command enum + aggregate_key)
                    - handler_v2.rs (EventHandler trait, EventHandlerContext, HandlerShared + test_support::HandlerTestHarness)
                    - priority.rs (SCENE_START/EMOTION_APPRAISAL/STIMULUS_APPLICATION/GUIDE_GENERATION/RELATIONSHIP_UPDATE/INFORMATION_TELLING/RUMOR_SPREAD 상수 + invariants)
                    - dispatcher.rs (CommandDispatcher: dispatch_v2 + with_default_handlers + with_memory + with_rumor 빌더)
                    - projection_handlers.rs (EmotionProjectionHandler/RelationshipProjectionHandler/SceneProjectionHandler Inline EventHandler wrappers)
                    - telling_ingestion_handler.rs (TellingIngestionHandler — InformationTold → MemoryEntry(Heard/Rumor), Step C2)
                    - rumor_distribution_handler.rs (RumorDistributionHandler — RumorSpread → 수신자별 MemoryEntry, Step C3)
                    - agents/ (impl EventHandler)
                      - emotion_agent.rs (AppraiseRequested → EmotionAppraised)
                      - stimulus_agent.rs (StimulusApplyRequested → StimulusApplied/BeatTransitioned)
                      - guide_agent.rs (EmotionAppraised/StimulusApplied/GuideRequested → GuideGenerated)
                      - relationship_agent.rs (BeatTransitioned/RelationshipUpdateRequested/DialogueEndRequested)
                      - scene_agent.rs (SceneStartRequested → SceneStarted + EmotionAppraised)
                      - information_agent.rs (TellInformationRequested → 청자당 1 InformationTold, Step C2)
                      - rumor_agent.rs (Seed/SpreadRumorRequested → RumorSeeded/RumorSpread + RumorStore 연동, Step C3)
  domain/         순수 도메인 로직
                  - personality (HEXACO), emotion (OCC appraisal), relationship, pad, guide
                  - listener_perspective [feature, default-on — Phase 7 Step 5] (화자 PAD → 청자 PAD 변환: prefilter + sign/magnitude k-NN + Converter trait, 88% baseline. DialogueAgent · Mind Studio 양 경로에서 옵셔널 자동 적용)
                  - event.rs (DomainEvent, EventPayload — 26 variants 포함 *Requested 9종 + Memory/Rumor/Information 11종, Event Sourcing)
                  - rumor.rs (Rumor 애그리거트 — RumorOrigin/ReachPolicy/RumorHop/RumorDistortion/RumorStatus + 불변식 I-RU-1~6, Step C1)
                  - aggregate.rs (B안 B0 — AggregateKey: Scene/Npc/Relationship)
                  - scene_id.rs (B안 B4 S2 — SceneId composite key)
                  - memory.rs (MemoryEntry, MemoryType, MemoryResult — RAG)
                  - tuning.rs (조정 가능 파라미터 중앙 관리)
  ports.rs        헥사고날 포트 트레이트 전체 + MemoryStore 포트 + SceneStore::get_scene_by_id (B4 S3 multi-scene)
  adapter/        포트 구현 (InMemoryRepository — multi-scene HashMap + last_scene_id, OrtEmbedder, RigChatAdapter, SqliteMemoryStore [embed])
  presentation/   다국어 포맷터 (ko, en TOML 기반, deep merge 지원)
  bin/mind-studio/  Axum REST API + SSE MCP 서버 + SSE 실시간 동기화 + static 파일 서빙
                  - /api/*       메인 UI 경로 — AppState(StateInner) 기반, B5.2 (2/3)부터 내부적으로 v2 dispatch_v2 호출
                  - /api/v2/*    Director shadow 경로 (B4 S3 B-Mini, 분리 Repository + SceneTask 실험용)
                  - domain_sync.rs  5 dispatch helper + sync_from_repo (shared_dispatcher 재사용, per-request snapshot 제거됨)
tests/            통합 테스트 (TestContext 공유) — dispatch_v2_test, director_test, dialogue_* 등 v2 기준
locales/          ko.toml, en.toml + PAD 앵커 (locales/anchors/)
docs/             아키텍처/감정/성격/가이드 상세 문서
data/             소설 기반 테스트 시나리오 + 캐릭터 프리셋(presets/)
mind-studio-ui/   Vite + React + TypeScript + Zustand 프론트엔드 (빌드 → bin/mind-studio/static/)
```

## 아키텍처

### 계층 구조
1. **Domain** (`src/domain`): 순수 비즈니스 로직, 외부 의존성 없음
2. **Application** (`src/application`): 도메인 조립 및 흐름 제어, 라이브러리 사용자 진입점
3. **Ports** (`src/ports.rs`): 헥사고날 경계 정의
4. **Infrastructure/Presentation** (`src/adapter`, `src/presentation`, `src/bin`): 외부 구현 및 API 노출

### 핵심 진입점

**`InMemoryRepository`** — 기본 `MindRepository` 구현체 (`adapter/memory_repository.rs`)
- `from_file("scenario.json")` / `from_json(json_str)` / `new()` + `add_npc()`/`add_relationship()`/`add_object()`
- `scenario_name()`, `scenario_description()`, `turn_history()` 메타데이터 접근자

**`CommandDispatcher<R>`** — CQRS Command 오케스트레이터 (`application/command/dispatcher.rs`)
- `::new(repo, event_store, event_bus)` — 기본 생성. 내부에서 `Arc<Mutex<R>>`로 감싸짐
- `.with_default_handlers()` — SceneAgent/EmotionAgent/StimulusAgent/GuideAgent/RelationshipAgent + 3 Projection wrapper 자동 등록
- `async fn dispatch_v2(&self, cmd) -> Result<DispatchV2Output, DispatchV2Error>` — 6 Command 전부 지원 (Appraise/ApplyStimulus/GenerateGuide/UpdateRelationship/EndDialogue/StartScene). Command → 초기 *Requested 이벤트 → Transactional BFS → HandlerShared write-back → Commit → Inline projection → Fanout 순서.
- **안전 한계**: `MAX_CASCADE_DEPTH = 4`, `MAX_EVENTS_PER_COMMAND = 20`
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

**`DialogueAgent<R, C>`** — LLM 대사 생성 오케스트레이터 (`application/dialogue_agent.rs`, chat feature)
- `CommandDispatcher<R>` + `ConversationPort` 조합으로 Event Sourcing 경로에 맞춘 LLM 다턴 대화
- **전제**: dispatcher는 `.with_default_handlers()`가 호출된 상태여야 함 (v2 path 사용).
- `::new(dispatcher, chat, formatter)` / `.with_analyzer(analyzer)`
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

### 주요 포트 (전체는 `ports.rs` 참조)

| 포트 | 용도 | 기본 구현체 |
|------|------|----------|
| `MindRepository` | `NpcWorld` + `EmotionStore` + `SceneStore` 통합 super-trait | `InMemoryRepository` |
| `Appraiser` | OCC 감정 평가 엔진 | `AppraisalEngine` |
| `StimulusProcessor` | PAD 자극 처리 엔진 | `StimulusEngine` |
| `GuideFormatter` | 가이드 → 텍스트/JSON | `LocaleFormatter` |
| `UtteranceAnalyzer` | 대사 → PAD ([embed feature]) | `PadAnalyzer` |
| `ConversationPort` | LLM 다턴 대화 세션 ([chat feature]) | `RigChatAdapter` |
| `LlamaServerMonitor` | llama-server 모니터링: health/slots/metrics ([chat feature]) | `RigChatAdapter` |
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


## EventBus · CQRS · Event Sourcing · Multi-Agent

> 상세 설계: [`docs/architecture/system-design-eventbus-cqrs.md`](docs/architecture/system-design-eventbus-cqrs.md)

### 아키텍처 개요

B안(v2) 이행 완료. v0.3.0에서 v1 경로(Pipeline/MindService/FormattedMindService/EventAwareMindService/`dispatch`/`shadow_v2`) 전부 제거됨.

```
┌─ Director (B안 B4, 다중 Scene facade) ──────────────────────┐
│  start_scene / dispatch_to(scene_id, cmd) / end_scene        │
│  active_scenes / DirectorError::Scene{NotActive|Mismatch|…}  │
├─ CommandDispatcher::dispatch_v2 (v2 write side) ────────────┤
│  Command → initial *Requested event → BFS cascade →          │
│    [Transactional handlers] → HandlerShared write-back →     │
│    [Commit to EventStore] → [Inline projections] → [Fanout]  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐        │
│  │ Scene    │ │ Emotion  │ │Stimulus  │ │  Guide   │        │
│  │ Agent    │ │  Agent   │ │  Agent   │ │  Agent   │        │
│  │ (pri 5)  │ │ (pri 10) │ │ (pri 15) │ │ (pri 20) │        │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘        │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐         │
│  │Relationship  │ │ Information  │ │   Rumor      │         │
│  │  Agent (30)  │ │  Agent (35)  │ │  Agent (40)  │         │
│  └──────────────┘ └──────────────┘ └──────────────┘         │
│                     (Step C2)        (Step C3)               │
│  Inline: Emotion/Relationship/Scene ProjectionHandler +      │
│          TellingIngestionHandler (C2) +                      │
│          RumorDistributionHandler (C3)                       │
├─ DialogueAgent (Dispatcher + ConversationPort wrapper) [chat]┤
│  start_session / turn / end_session async API               │
│  Beat 전환 시 ConversationPort.update_system_prompt         │
├─ EventBus (tokio::broadcast fan-out) ──────────────────────┤
│  subscribe() → impl Stream<Arc<DomainEvent>> (runtime-agnostic)│
├─ MemoryAgent (broadcast 구독) [embed] ──────────────────────┤
│  DialogueTurnCompleted/RelationshipUpdated → 임베딩 → RAG    │
└─────────────────────────────────────────────────────────────┘
```

### 이벤트 흐름 (v2)

```
Command 수신
  → CommandDispatcher.dispatch_v2(cmd)
  → build_initial_event(cmd) → *Requested event (enqueue depth=0)
  → [Transactional phase — BFS]
       각 event pop → priority 오름차순 transactional_handlers 실행
         → HandlerShared 상태 전파 (emotion_state/relationship/scene/guide/clear_*)
         → follow_up_events → queue.push(depth+1) [MAX_CASCADE_DEPTH=4 가드]
       event → staging_buffer [MAX_EVENTS_PER_COMMAND=20 가드]
  → apply_shared_to_repository (save_* + clear_*)
  → [Commit phase] staging_buffer → event_store.append (실 ID/seq 할당)
  → [Inline phase] projection handlers — best-effort, 에러는 로그만
  → [Fanout phase] event_bus.publish (tokio::broadcast)
```

### DomainEvent (26 variants)

#### v2 초기 이벤트 (9종 *Requested — Command → initial event)
| EventPayload | 생성 계기 | 소비 Handler |
|---|---|---|
| `AppraiseRequested` | `Command::Appraise` | EmotionAgent |
| `StimulusApplyRequested` | `Command::ApplyStimulus` | StimulusAgent |
| `GuideRequested` | `Command::GenerateGuide` | GuideAgent |
| `RelationshipUpdateRequested` | `Command::UpdateRelationship` | RelationshipAgent |
| `DialogueEndRequested` | `Command::EndDialogue` | RelationshipAgent (3 follow-ups) |
| `SceneStartRequested` | `Command::StartScene` | SceneAgent (prebuilt_scene 포함) |
| `TellInformationRequested` | `Command::TellInformation` | InformationAgent (청자당 1 InformationTold) — Step C2 |
| `SeedRumorRequested` | `Command::SeedRumor` | RumorAgent (pending_id로 커맨드별 고유 aggregate) — Step C3 |
| `SpreadRumorRequested` | `Command::SpreadRumor` | RumorAgent (RumorSpread + hop 기록) — Step C3 |

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
| `MemoryEntryCreated` / `MemoryEntrySuperseded` / `MemoryEntryConsolidated` | Memory 엔트리 수명주기 (Step C1 선언, 발행은 Step D) |
| `RumorSeeded` / `RumorSpread` | Rumor 시딩·확산 (Step C3) |
| `RumorDistorted` / `RumorFaded` | 변형·종결 (Step F 발행 예정) |

#### AggregateKey 매핑 (라우팅 기준)
- `Scene { npc_id, partner_id }`: SceneStarted/Ended/StartRequested, DialogueEndRequested, BeatTransitioned
- `Relationship { owner_id, target_id }`: RelationshipUpdated/UpdateRequested
- `Npc(npc_id)`: AppraiseRequested/EmotionAppraised/StimulusApply(Requested)/GuideRequested/GuideGenerated/DialogueTurnCompleted/EmotionCleared · `TellInformationRequested`(speaker) · `InformationTold`(listener — B5 청자 기반 라우팅)
- `Rumor(rumor_id)`: `RumorSeeded/Spread/Distorted/Faded`, `SpreadRumorRequested`. `SeedRumorRequested`는 `Rumor("pending-<pending_id>")`로 커맨드별 고유 (Step C3 사후 리뷰 C2)
- `Memory(entry_id)`: `MemoryEntryCreated/Superseded/Consolidated` (Step D에서 사용)
- `World(world_id)`: Step D `ApplyWorldEventRequested/WorldEventOccurred`에서 사용 예정

### Pipeline (v0.3.0 제거됨)

v2 `dispatch_v2`의 transactional handler chain (BFS + follow_up_events)이 Pipeline을 대체.
`with_default_handlers()` + `dispatch_v2(cmd)` 조합이 유일한 write 경로.

### v2 EventHandler + HandlerShared

모든 Agent + Projection wrapper는 공통 `EventHandler` trait을 구현:

```rust
pub trait EventHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn interest(&self) -> HandlerInterest;          // Kinds(vec![EventKind::...])
    fn mode(&self) -> DeliveryMode;                  // Transactional/Inline
    fn handle(&self, event: &DomainEvent, ctx: &mut EventHandlerContext) -> Result<HandlerResult, HandlerError>;
}
```

`HandlerShared` (커맨드 범위 mutable scratchpad):
- `emotion_state: Option<EmotionState>` · `relationship: Option<Relationship>` · `scene: Option<Scene>` · `guide: Option<ActingGuide>`
- destructive 시그널: `clear_emotion_for: Option<String>` · `clear_scene: bool` (B4.1 DialogueEnd)

### EventBus (tokio::broadcast 기반)

| 계층 | 실행 | 용도 | 구현 |
|------|------|------|------|
| **Transactional handlers** | `dispatch_v2` 내부 BFS | v2 커맨드 내부 에이전트 체인 | priority 오름차순 반복 |
| **Inline projections** | commit 후 동기 | 쿼리 일관성 뷰 | `dispatch_v2` Inline phase |
| **EventBus** (Fanout) | `send()` 후 broadcast | 외부 Agent·SSE·구독자 | `subscribe() -> impl Stream<Arc<DomainEvent>>` |

**공개 API 원칙**: `EventBus.subscribe()`가 반환하는 `futures::Stream`은 runtime-agnostic. Bevy·smol·async-std 등 임의 executor에서 폴링 가능. tokio는 내부 구현 디테일이며 호출자 deps에 노출되지 않음.

**Lag 복구**: `broadcast`는 capacity 초과 시 가장 오래된 이벤트를 덮어쓴다. at-least-once가 필요한 소비자는 `subscribe_with_lag()`로 `Lagged(n)` 통지를 받고 `EventStore.get_events_after_id(last_id)`로 replay한다. (`MemoryAgent::run`이 이 패턴 구현)

### 기억 시스템 (RAG) [embed feature]

```
MemoryAgent (EventBus subscriber)
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

**Memory Step B 주입 (완료, [chat feature])**: `DialogueAgent::with_memory(store, framer)` opt-in
빌더로 활성화. 활성화되면 `start_session` 1회 + `BeatTransitioned` 발생 시
`inject_memory_push(npc, query, pad)`가 다음 파이프라인으로 "떠오르는 기억" 블록을 시스템
프롬프트 앞에 prepend한다:

```
DialogueAgent.start_session/turn(BeatTransitioned)
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
| Phase 2 | ✅ 완료 | Command, EmotionAgent, GuideAgent, RelAgent, CommandDispatcher (v2 단일 경로) |
| Phase 3 | ✅ 완료 | MemoryAgent, MemoryStore, SqliteMemoryStore, DialogueTurnCompleted |
| EventBus v2 | ✅ 완료 | tokio::broadcast 단일화, runtime-agnostic Stream API, MemoryAgent replay 기반 at-least-once |
| Phase 4 | ✅ 완료 | DialogueAgent — CommandDispatcher + ConversationPort 통합 오케스트레이터 (chat feature) |
| **B안 B0** | ✅ 완료 | EventHandler trait · HandlerShared · AggregateKey · priority 상수 뼈대 |
| **B안 B1** | ✅ 완료 | 4 Agent v2 EventHandler 구현 + StimulusAgent 신규 + 2 *Requested variant (AppraiseRequested/StimulusApplyRequested) + HandlerTestHarness |
| **B안 B2** | ✅ 완료 | EmotionProjectionHandler/RelationshipProjectionHandler/SceneProjectionHandler (Inline wrapper) |
| **B안 B3** | ✅ 완료 | `dispatch_v2()` BFS loop + `with_default_handlers()` + parallel run 비교 (Appraise/ApplyStimulus) |
| **B안 B4 S1** | ✅ 완료 | 6 Command 전부 v2 지원 + SceneAgent 신규 + 4 *Requested variant (Guide/RelationshipUpdate/DialogueEnd/SceneStart) + HandlerShared clear 시그널 |
| **B안 B4 S2** | ✅ 완료 | Director + SceneId + InMemoryRepository multi-scene HashMap refactor + 11 E2E 테스트 |
| **B안 B4 S3 Option A** | ✅ 완료 | BeatTransitioned.partner_id 추가 + SceneStore::get_scene_by_id + StimulusAgent multi-scene fix + 회귀 가드 |
| **B안 B4 S3 Option B-Mini** | ✅ 완료 | Mind Studio `/api/v2/*` shadow 엔드포인트 (7개) + Director 통합 + 7 integration 테스트 |
| **B안 B4 S4 (축소판 A)** | ✅ 완료 | async `dispatch_v2(&self)` + `Arc<Mutex<R>>` 내부 공유 + `Spawner` trait + `SceneTask` mpsc 루프 + Director 전면 async 재작성 (fire-and-forget) + tests cutover. 런타임 중립 유지(`tokio::spawn` 미호출). |
| **B안 B5.1** | ✅ 완료 | Pipeline/Projection trait/EventAwareMindService/HandlerContext·Output/v1 dispatch/v1 Agent handle_* 전부 `#[deprecated(since="0.2.0")]` 마킹, v0.3.0 제거 예정 |
| **B안 B5.2** | ✅ 완료 | (1/3) DialogueAgent v2 마이그레이션. (2/3) Mind Studio handler v2 마이그레이션. (3/3) AppState 통합 — `shared_dispatcher` 도입, per-request snapshot 제거, UI CRUD/scenario load가 `rebuild_repo_from_inner`로 공유 repo 동기화. |
| **B안 B5.3** | ✅ 완료 | v1 모듈·타입 삭제 — Pipeline/Projection trait/EventAwareMindService/MindService/FormattedMindService/HandlerContext·Output/v1 Agent handle_*/AppStateRepository(mut)/DialogueTestService struct/v1 dispatch/shadow_v2 전부 제거. `emotion_snapshot` 헬퍼 → `EmotionState::snapshot()` 메서드로 이관. `MindServiceError` → `application::error` 모듈로 분리. v1 테스트 파일 8종(application/event/command/pipeline/locale/port_injection/repository/coverage_gap) 삭제 + dispatch_v2_test 안의 v1 parallel 테스트 3종 삭제. |
| B안 B5.4 | 불필요 | B5.3에서 `shadow_v2` 이미 제거. |
| **Memory Step A** | ✅ 완료 | `MemoryScope`/`MemorySource`/`Provenance`/`MemoryLayer` VO + `MemoryEntry` 13 필드 확장 + `MemoryType` rename (serde alias 역호환) + `MemoryRanker` 2단계 (Source 우선 + 5요소 점수) + `DecayTauTable` + SQLite v2 자동 마이그레이션 + `MemoryStore` 7 신규 메서드 + `MemoryQuery`/`MemoryScopeFilter` + `RelationshipUpdated.cause` hook. 행동 변화 없이 foundation만. 상세: [`docs/memory/03-implementation-design.md`](docs/memory/03-implementation-design.md) |
| **Memory Step B** | ✅ 완료 | `MemoryFramer` trait + `LocaleMemoryFramer` (Source별 라벨, ko/en locale 빌트인) + `[memory.framing]` locale 섹션 + `DialogueAgent::with_memory(store, framer)` opt-in + `inject_memory_push` 내부 메서드 (NpcAllowed scope 검색 → MemoryRanker 2단계 → Top-K 프롬프트 블록) + `start_session` 1회 + `BeatTransitioned` 시 재주입. 구 `search_by_meaning`/`search_by_keyword`/`get_recent` `#[deprecated(since="0.4.0")]` 마킹. |
| **Memory Step C1** | ✅ 완료 | Rumor 도메인 foundation — `Rumor` 애그리거트 (`src/domain/rumor.rs`) + `RumorOrigin`/`ReachPolicy`/`RumorHop`/`RumorDistortion`/`RumorStatus` + 불변식 I-RU-1~6. `RumorStore` 포트 + `SqliteRumorStore` [embed]. `AggregateKey::Memory/Rumor/World` variant. `EventPayload` 11 신규 variant (`Memory*`/`Rumor*`/`TellInformationRequested`/`InformationTold` 등). 행동 변화 없음. 사후 리뷰에서 schema v3 migration(composite PK)·cycle detection·reach_overlaps 등 7건 수정. 커밋 `bcb0581` + 사후 `30d7f94`. |
| **Memory Step C2** | ✅ 완료 | `Command::TellInformation` + `TellInformationRequest`/`Response` DTO + `InformationAgent` (Transactional, priority `INFORMATION_TELLING=35`) + `TellingIngestionHandler` (Inline) + `CommandDispatcher::with_memory(store)` 빌더. 청자당 1 `InformationTold` follow-up (B5) + listener `MemoryEntry(Heard/Rumor)` 생성. `stated_confidence × normalized_trust` 신뢰도, origin_chain 기반 Heard/Rumor 자동 분류. 12개 통합 테스트. 커밋 `f410e74` + 사후 `ff3d032`(C1 dispatcher aggregate_id routing 수정, C2 dedup, M1 deterministic id, M3 topic pass-through, M7 budget test). |
| **Memory Step C3** | ✅ 완료 | `Command::SeedRumor` + `Command::SpreadRumor` + `SeedRumorRequest`/`SpreadRumorRequest` DTO + `RumorAgent` (Transactional, priority `RUMOR_SPREAD=40`) + `RumorDistributionHandler` (Inline). Canonical 해소 3-tier (Distortion → Canonical via `get_canonical_by_topic` → seed_content fallback). `RUMOR_HOP_CONFIDENCE_DECAY^hop_index` 감쇠 + `RUMOR_MIN_CONFIDENCE` floor. `with_rumor(memory_store, rumor_store)` 빌더. 11개 통합 테스트 (rumor_spread + rumor_canonical_resolution). 커밋 `d088470` + 사후 `8413857`(rumor_id event.id=0 버그) + `5ebf37f`(C2 pending_id으로 orphan 공용 버킷 제거, M1 RumorAgent 자체 counter, §14 원자성 재정의, Step F 명기). |
| Memory Step D | 미구현 | `SceneConsolidationHandler` (Layer A→B) + `WorldOverlayAgent` + `Command::ApplyWorldEvent` + `RelationshipMemoryHandler` cause-분기 |
| Phase 5 | 미구현 | StoryAgent (서사 진행 판단) |
| Phase 6 | 미구현 | Tool 시스템 (ToolRegistry) |
| Phase 7 | 미구현 | WorldKnowledgeStore (세계관 정적 지식) |
| Phase 8 | 미구현 | SummaryAgent (컨텍스트 윈도우 관리) |

전체 B안 설계 참조: [`docs/architecture/b-plan-implementation.md`](docs/architecture/b-plan-implementation.md)

## 개발 컨벤션

### DTO 분리 (Result / Response)
- `*Result` (도메인): `ActingGuide` 포함, 포맷팅 전. 도메인 엔진(`AppraisalEngine`/`StimulusEngine`) 내부 타입
- `*Response` (포맷팅 완료): `prompt: String` 포함. `DispatchV2Output` → `DialogueAgent`/`domain_sync` 헬퍼가 formatter 적용해 생성
- `ChatResponse` (chat 포트): `text + timings`. `ConversationPort`가 반환
- 변환: `result.format(&formatter)` → `Response` (`CanFormat` 트레이트)

### 네이밍 (DDD)
- Domain Services: `~Engine` / `~Analyzer`
- Application Services: `~Service`
- Ports: 행위 명사 (`ports.rs`)
- Domain Events: 과거형

### 에러 처리
- 서비스 계층: `MindServiceError` (`application::error`) 반환 — NpcNotFound/RelationshipNotFound/InvalidSituation/EmotionStateNotFound/LocaleError
- dispatch 계층: `DispatchV2Error` (`CommandDispatcher`) — HandlerFailed/CascadeTooDeep/EventBudgetExceeded/InvalidSituation
- 웹 계층(`mind-studio`): `AppError` → 적절한 HTTP 상태 코드와 JSON으로 자동 변환 (`IntoResponse`)

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

게임이 Scene 시작 시 Focus 옵션 목록을 제공하고, 엔진이 stimulus 처리 중 감정 상태 조건(`FocusTrigger`)을 평가하여 자동으로 Beat 전환을 판단합니다. Beat 전환 로직은 `Command::ApplyStimulus` → `StimulusAgent`에서 처리되며, `BeatTransitioned` 이벤트를 follow-up으로 발행합니다.

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
Command::ApplyStimulus → StimulusAgent.handle()
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

## 튜닝 상수 (주요, 전체는 `src/domain/tuning.rs` 참조)

| 상수 | 값 | 용도 |
|------|-----|------|
| `STIMULUS_IMPACT_RATE` | 0.5 | stimulus 감정 변동 계수 |
| `STIMULUS_MIN_INERTIA` | 0.30 | 관성 최소값 (intensity=1.0에서도 반응 보장) |
| `BEAT_MERGE_THRESHOLD` | 0.2 | Beat 합치기 시 이전 감정 소멸 기준 |
| `TRUST_UPDATE_RATE` | 0.1 | 신뢰 갱신 계수 |
| `CLOSENESS_UPDATE_RATE` | 0.05 | 친밀도 갱신 계수 |
| `SIGNIFICANCE_SCALE` | 3.0 | 상황 중요도 배율 (sig=1.0 → 4배) |
| `EMOTION_THRESHOLD` | 0.2 | 감정 유의미 판단 기준 (가이드 반영) |
| `TRAIT_THRESHOLD` | 0.3 | 성격 특성 추출 임계값 |

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
- **실시간 상태 동기화**: `tokio::sync::broadcast` → SSE `/api/events` → `EventSource` (useStateSync 훅). MCP/REST 상태 변경이 UI에 자동 반영
- REST API 엔드포인트 전체는 `src/bin/mind-studio/handlers/` 참조

## LLM 대화 테스트 (`chat` feature)

Mind Engine이 생성한 프롬프트를 실제 LLM에 system prompt로 주입하고 다턴 대화로 NPC 연기 품질을 검증합니다.

- **ConversationPort** (`ports.rs`): LLM 대화 세션 추상화 — `start_session`, `send_message`, `update_system_prompt`, `end_session`
  - `send_message()` / `send_message_stream()`은 `ChatResponse { text, timings }` 반환
- **RigChatAdapter** (`adapter/rig_chat.rs`): rig-core 0.33 `openai::CompletionsClient<TimingsCapturingClient>` 기반 구현. 세션별 system_prompt + rig_history + dialogue_history 관리. `LlamaServerMonitor` 구현도 포함
- **TimingsCapturingClient** (`adapter/llama_timings.rs`): rig의 `HttpClientExt` 래퍼. HTTP 응답에서 llama-server `timings`를 캡처하여 `ChatResponse`에 포함. rig 소스 수정 없이 `ClientBuilder.http_client()`로 주입. `with_client()`로 외부 `reqwest::Client` 주입 지원
- **DialogueAgent** (`application/dialogue_agent.rs`): `CommandDispatcher` + `ConversationPort` 오케스트레이터. `start_session`/`turn`/`end_session` API
- **dialogue_test_service.rs**: Mind Studio ↔ DialogueAgent DTO (`Chat*Request`/`Chat*Response`) 전용. 오케스트레이션 struct는 없음 (v0.3.0에서 제거됨)

### llama-server Timings 캡처

llama-server는 `/v1/chat/completions` 응답에 `timings` 객체(prompt/predicted 처리 속도)를 포함한다.
rig-core의 OpenAI 응답 타입은 이 필드를 무시하므로, `TimingsCapturingClient`가 HTTP 계층에서 가로챈다.

```
[llama-server] → JSON 응답 (timings 포함)
       ↓
[TimingsCapturingClient] → timings 파싱 & 저장 → Arc<RwLock<Option<LlamaTimings>>>
       ↓ (body 그대로 전달)
[rig CompletionModel] → CompletionResponse 파싱 (timings 무시)
       ↓
[RigChatAdapter] → ChatResponse { text, timings }
```

- **Non-streaming** (`send()`): 응답 body 전체를 읽어 `timings` 추출 후 rig에 전달
- **Streaming** (`send_streaming()`): SSE 청크를 래핑하여 `"timings"` 포함 청크에서 캡처
- **주요 타입**: `LlamaTimings` (8개 필드), `ChatResponse { text, timings: Option<LlamaTimings> }`

### llama-server 모니터링 (`LlamaServerMonitor`)

llama-server는 Chat Completions 외에 서버 관리용 엔드포인트를 제공한다.
`LlamaServerMonitor` 포트 트레이트가 이를 추상화하고, `RigChatAdapter`가 구현한다.

| 메서드 | llama-server 엔드포인트 | 반환 타입 | 용도 |
|--------|----------------------|-----------|------|
| `health()` | `GET /health` | `LlamaHealth` | 서버 상태 (`ok`, `loading model` 등) |
| `slots()` | `GET /slots` | `Vec<LlamaSlotInfo>` | 슬롯별 idle/processing 상태, 토큰 수 |
| `metrics()` | `GET /metrics` | `LlamaMetrics` | Prometheus 메트릭 (KV 캐시, 처리 속도 등) |

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

대화 루프 (DialogueAgent 기준):
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
- **아키텍처 v3 (EventBus/CQRS)**: [`docs/architecture/system-design-eventbus-cqrs.md`](docs/architecture/system-design-eventbus-cqrs.md) — EventBus, CQRS, Event Sourcing, Multi-Agent, RAG 시스템 디자인
- **프론트엔드 아키텍처**: [`docs/architecture/frontend-architecture.md`](docs/architecture/frontend-architecture.md) — Vite+React+Zustand 구조, 스토어 설계, 데이터 흐름, 컴포넌트 트리
- **협업 워크플로우**: [`docs/collaboration-workflow.md`](docs/collaboration-workflow.md)
- **감정 엔진**: [`docs/emotion/`](docs/emotion/) — OCC 모델, HEXACO 매핑, PAD 앵커 매트릭스, appraisal 엔진 설계
- **Listener-perspective 변환** (Phase 7): [`docs/emotion/sign-classifier-design.md`](docs/emotion/sign-classifier-design.md) (부호/강도 분류기 설계 + §3.7 Register 전략) + [`docs/emotion/phase7-converter-integration.md`](docs/emotion/phase7-converter-integration.md) (프로덕션 통합, **Step 1-5+ 완료** — 88% baseline, default-on, DialogueAgent · Mind Studio 통합, §6.1 테스트 카탈로그 71개)
- **성격 모델**: [`docs/personality/`](docs/personality/) — HEXACO 6차원 facet 상세
- **가이드 매핑**: [`docs/guide/guide-mapping-table.md`](docs/guide/guide-mapping-table.md)
- **테스트 스크립트**: `mcp/skills/npc-scenario-creator/SKILL.md` (4-1단계) + `mcp/skills/npc-mind-testing/SKILL.md` (원칙 4, 커서 관리)
- **언어 설정**: [`docs/locale-guide.md`](docs/locale-guide.md)
- **MCP 서버 설정**: `.mcp.json` (프로젝트 루트)
