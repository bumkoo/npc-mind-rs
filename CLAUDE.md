# CLAUDE.md

NPC Mind Engine — HEXACO 성격이 OCC 감정을 생성하고, LLM이 연기할 수 있도록 가이드를 출력하는 Rust 라이브러리.

라이브러리 형태로 배포되며, `MindService`/`EventAwareMindService`/`CommandDispatcher`가 주요 진입점입니다.

## 기술 스택
- **Language:** Rust (Edition 2024)
- **Architecture:** Hexagonal + DDD + **EventBus(tokio broadcast)/CQRS/Event Sourcing** + **Multi-Agent**
- **Libraries:** `serde`/`serde_json`, `thiserror`, `tokio/sync`+`tokio-stream`+`futures` (EventBus 내부 구현), `axum`(WebUI), `tracing`, `ort`(ONNX 임베딩), `rig-core`(LLM Agent 대화), `rusqlite`+`sqlite-vec`(RAG 저장소 [embed] — FTS5 + vec0 벡터 인덱스)
- **런타임 정책:** 코어는 `tokio::sync`의 `broadcast`만 내부 구현으로 사용. 공개 API는 `futures::Stream` 타입만 노출하므로 **호출자는 tokio를 deps에 추가할 필요 없음**(Bevy 등 임의 async 런타임에서 Stream 소비 가능). `chat`/`mind-studio` feature가 tokio `rt-multi-thread` 런타임을 추가 활성화. `embed` feature는 sqlite-vec이 순수 C 확장이라 tokio 런타임을 전이시키지 않는다.

## 빌드 & 테스트

```bash
cargo build                        # 기본 빌드
cargo build --features embed       # 임베딩 포함 (bge-m3-onnx-rust)
cargo build --features chat        # LLM 대화 에이전트 포함 (rig-core)
cargo test                         # 기본 테스트
cargo test --features embed        # 전체 테스트 (임베딩 포함)

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
  application/    어플리케이션 계층, 라이브러리 진입점
                  - mind_service.rs (MindService), formatted_service.rs (FormattedMindService)
                  - event_service.rs (EventAwareMindService — Strangler Fig 래퍼)
                  - event_store.rs, event_bus.rs (Event Sourcing 인프라)
                  - pipeline.rs (순차 에이전트 체인), tiered_event_bus.rs (동기/비동기 2-Tier)
                  - projection.rs (이벤트 파생 읽기 뷰)
                  - memory_store.rs (InMemoryMemoryStore)
                  - memory_agent.rs (EventBus 구독 기억 인덱싱 [embed])
                  - command/ (CQRS Command Side)
                    - types.rs (Command enum), handler.rs (HandlerContext/Output)
                    - dispatcher.rs (CommandDispatcher 오케스트레이터)
                    - agents/ (EmotionAgent, GuideAgent, RelationshipAgent)
                  + relationship_service, scene_service, situation_service, dialogue_test_service
  domain/         순수 도메인 로직
                  - personality (HEXACO), emotion (OCC appraisal), relationship, pad, guide
                  - event.rs (DomainEvent, EventPayload — Event Sourcing)
                  - memory.rs (MemoryEntry, MemoryType, MemoryResult — RAG)
                  - tuning.rs (조정 가능 파라미터 중앙 관리)
  ports.rs        헥사고날 포트 트레이트 전체 + MemoryStore 포트
  adapter/        포트 구현 (InMemoryRepository, OrtEmbedder, RigChatAdapter, SqliteMemoryStore [embed])
  presentation/   다국어 포맷터 (ko, en TOML 기반, deep merge 지원)
  bin/mind-studio/  Axum REST API + SSE MCP 서버 + SSE 실시간 동기화 + static 파일 서빙
tests/            통합 테스트 (TestContext 공유)
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

**`MindService<R, A, S>`** — 도메인 결과 반환 (포맷팅 없음)
- `MindService::new(repo)` 또는 `::with_engines(repo, appraiser, stimulus)`
- 반환: `AppraiseResult`, `StimulusResult`, `GuideResult` (`ActingGuide` 포함)

**`FormattedMindService<R, A, S>`** — 포맷팅된 프롬프트 반환
- `::new(repo, "ko")` / `::with_overrides()` / `::with_custom_locale()` / `::with_formatter()`
- 반환: `AppraiseResponse`, `StimulusResponse` 등 (`prompt: String` 포함)

**`EventAwareMindService<R, A, S>`** — Strangler Fig 래퍼 (`application/event_service.rs`)
- `MindService`를 감싸서 동일 API 유지 + 모든 호출에 `DomainEvent` 발행
- `::new(inner, event_store, event_bus)` / `::with_default_events(repo)`
- `EventStore`에 append-only 기록 + `EventBus`로 실시간 발행

**`CommandDispatcher<R>`** — CQRS Command 오케스트레이터 (`application/command/dispatcher.rs`)
- `Command` enum → Agent 라우팅 → side-effect 적용 → 이벤트 발행
- `::new(repo, event_store, event_bus)` / `.with_tiered_bus(tiered_bus)`
- `dispatch(cmd)` — 기존 방식 (직접 라우팅)
- `execute_pipeline(pipeline, cmd)` — Pipeline 방식 (순차 에이전트 체인)

### 주요 메서드
- `appraise()` — 초기 상황 판단 및 감정 생성
- `apply_stimulus()` — 대화 중 실시간 감정 변화 + Beat 전환 자동 처리
- `start_scene()` / `scene_info()` / `load_scene_focuses()` — Scene 관리
- `after_beat()` / `after_dialogue()` — 관계 갱신
- `generate_guide()` — 현재 감정에서 가이드 재생성

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

기존 `MindService` (God Object, 632L)를 Strangler Fig 패턴으로 점진 전환 중.

```
┌─ MindService (기존, 유지) ──────────────────────────────────┐
│  appraise / apply_stimulus / after_dialogue / ...            │
├─ EventAwareMindService (래퍼) ──────────────────────────────┤
│  기존 API 동일 + DomainEvent 발행                            │
├─ CommandDispatcher (CQRS Write Side) ───────────────────────┤
│  Command → Agent 라우팅 → HandlerOutput → side-effect + event│
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                     │
│  │ Emotion  │ │  Guide   │ │   Rel    │                     │
│  │  Agent   │ │  Agent   │ │  Agent   │                     │
│  └──────────┘ └──────────┘ └──────────┘                     │
├─ Pipeline (Tier 1 - 순차 동기 에이전트 체인) ──────────────┤
│  Stage 1 → Stage 2 → Stage 3 (컨텍스트 전파, 에러 시 중단)  │
├─ L1 Projection Registry (Dispatcher 내부, 쓰기 경로) ──────┤
│  EmotionProjection · RelationshipProjection · SceneProjection│
│  emit 시 apply_all()로 동기 갱신 → 쿼리 일관성 보장         │
├─ EventBus (Tier 2 - tokio::broadcast fan-out) ─────────────┤
│  subscribe() → impl Stream<Arc<DomainEvent>> (runtime-agnostic)│
│  구독자는 자기 async 런타임에서 .next().await 소비           │
├─ MemoryAgent (broadcast 구독) [embed] ──────────────────────┤
│  DialogueTurnCompleted/RelationshipUpdated → 임베딩 → RAG    │
│  Lag 시 EventStore.get_events_after_id()로 replay (at-least-once)│
└─────────────────────────────────────────────────────────────┘
```

### 이벤트 흐름

```
Command 수신
  → CommandDispatcher.dispatch() 또는 execute_pipeline()
  → Agent.handle_*() → HandlerOutput { result, events, side-effects }
  → repository write-back (emotion_state, relationship, scene)
  → EventStore.append() (영속화 먼저)
  → ProjectionRegistry.apply_all() (L1 동기 — 쿼리 일관성)
  → EventBus.publish() (tokio::broadcast fan-out, Tier 2 비동기 소비자들)
```

### DomainEvent (9 variants)

| EventPayload | 발생 시점 |
|-------------|----------|
| `EmotionAppraised` | appraise 완료 (emotion_snapshot 포함) |
| `StimulusApplied` | PAD 자극 적용 (emotion_snapshot 포함) |
| `BeatTransitioned` | Focus 전환 |
| `SceneStarted` / `SceneEnded` | Scene 시작/종료 |
| `RelationshipUpdated` | 관계 갱신 (before/after 6값) |
| `GuideGenerated` | 가이드 생성 |
| `DialogueTurnCompleted` | 대화 턴 완료 (utterance + speaker) |
| `EmotionCleared` | 감정 초기화 |

### Pipeline (순차 에이전트 체인)

파이프라인 내부는 **순차** (앞 단계 결과가 뒤 단계 입력), 외부 EventBus 구독자는 **비동기** (독립 실행).

```rust
let pipeline = Pipeline::new()
    .add_stage(/* EmotionAgent */)   // Stage 1: 감정 평가
    .add_stage(/* GuideAgent */);    // Stage 2: Stage 1의 emotion_state로 가이드 생성

dispatcher.execute_pipeline(pipeline, &cmd)?;
```

`PipelineState`가 단계 간 `emotion_state`/`relationship`/`scene` 전파.

### EventBus (tokio::broadcast 기반)

| 계층 | 실행 | 용도 | 구현 |
|------|------|------|------|
| **Pipeline** (Tier 1) | 순차 동기 | 커맨드 내부 에이전트 체인 | `Pipeline.execute()` stages |
| **L1 Projection** | emit 내 동기 | 쿼리 일관성 뷰 | Dispatcher가 `ProjectionRegistry.apply_all()` 직접 호출 |
| **EventBus** (Tier 2) | `send()` 후 broadcast | Agent·SSE·외부 소비자 | `subscribe() -> impl Stream<Arc<DomainEvent>>` |

**공개 API 원칙**: `EventBus.subscribe()`가 반환하는 `futures::Stream`은 runtime-agnostic. Bevy·smol·async-std 등 임의 executor에서 폴링 가능. tokio는 내부 구현 디테일이며 호출자 deps에 노출되지 않음.

**Lag 복구**: `broadcast`는 capacity 초과 시 가장 오래된 이벤트를 덮어쓴다. at-least-once가 필요한 소비자는 `subscribe_with_lag()`로 `Lagged(n)` 통지를 받고 `EventStore.get_events_after_id(last_id)`로 replay한다. (`MemoryAgent::run`이 이 패턴 구현)

### 기억 시스템 (RAG) [embed feature]

```
MemoryAgent (EventBus subscriber)
  → 이벤트 수신 → MemoryEntry 구성 → TextEmbedder 임베딩 → MemoryStore.index()

SqliteMemoryStore (기본 구현, 단일 SQLite 파일):
  ├── memories       (일반 테이블 — 메타 + 원문 TEXT)
  ├── memories_fts   (FTS5 가상 테이블 — 키워드 전문 검색)
  └── memories_vec   (sqlite-vec vec0 가상 테이블 — 코사인 ANN, FLOAT[dim])
  세 레이어가 id로 조인. search_by_meaning: vec0 Top-K → memories batch load.

테스트 전용:
  tests/common/in_memory_store.rs — InMemoryMemoryStore (brute-force cosine).
  라이브러리 public API로 노출되지 않음.
```

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
| Phase 1 | ✅ 완료 | EventBus, EventStore, EventAwareMindService, Projections |
| Phase 2 | ✅ 완료 | Command/CommandResult, EmotionAgent, GuideAgent, RelAgent, CommandDispatcher |
| Phase 3 | ✅ 완료 | MemoryAgent, MemoryStore, SqliteMemoryStore, DialogueTurnCompleted |
| Pipeline | ✅ 완료 | Pipeline (순차 체인) |
| EventBus v2 | ✅ 완료 | tokio::broadcast 단일화, L1 Projection Registry(Dispatcher 내부), runtime-agnostic Stream API, MemoryAgent replay 기반 at-least-once |
| Phase 4 | 미구현 | DialogueAgent (LLM 대사 생성, tokio async 필요) |
| Phase 5 | 미구현 | StoryAgent (서사 진행 판단) |
| Phase 6 | 미구현 | Tool 시스템 (ToolRegistry) |
| Phase 7 | 미구현 | WorldKnowledgeStore (세계관 정적 지식) |
| Phase 8 | 미구현 | SummaryAgent (컨텍스트 윈도우 관리) |

## 개발 컨벤션

### DTO 분리 (Result / Response)
- `*Result` (도메인): `ActingGuide` 포함, 포맷팅 전. `MindService`가 반환
- `*Response` (포맷팅 완료): `prompt: String` 포함. `FormattedMindService`가 반환
- `ChatResponse` (chat 포트): `text + timings`. `ConversationPort`가 반환
- 변환: `result.format(&formatter)` → `Response` (`CanFormat` 트레이트)

### 네이밍 (DDD)
- Domain Services: `~Engine` / `~Analyzer`
- Application Services: `~Service`
- Ports: 행위 명사 (`ports.rs`)
- Domain Events: 과거형

### 에러 처리
- 서비스 계층: `MindServiceError` 반환
- 웹 계층(`mind-studio`): `AppError` → 적절한 HTTP 상태 코드와 JSON으로 자동 변환 (`IntoResponse`)

### 데이터 변환 (Mapping)
- DTO(`SituationInput` 등)는 `SituationService`를 통해 도메인 모델로 변환
- DTO는 저장소 의존성 없는 순수 데이터 구조체
- 서비스가 저장소(`MindRepository`)에서 관계/오브젝트 정보를 조회하여 변환 시 주입

### 테스트 (TestContext)
- 모든 통합 테스트는 `tests/common/mod.rs`의 `TestContext`를 사용
- 캐릭터 생성 / 저장소 초기화 중복 코드 방지, 일관된 테스트 환경 보장

## 용어 정의

| 용어 | 영문 | 정의 | 엔진 호출 |
|------|------|------|----------|
| **장면** | Scene | 하나의 연속된 대화 단위. 시작과 끝이 있음 | `after_dialogue()` 1회 |
| **비트** | Beat | 장면 안에서 감정 흐름이 전환되는 시점 | `appraise()` 1회 |
| **대사** | Utterance | 실제 캐릭터가 말하는 한 줄의 대사 | `apply_stimulus()` 입력 |

## Scene Focus 시스템

게임이 Scene 시작 시 Focus 옵션 목록을 제공하고, 엔진이 stimulus 처리 중 감정 상태 조건(`FocusTrigger`)을 평가하여 자동으로 Beat 전환을 판단합니다. Beat 전환 로직은 `MindService.apply_stimulus()` → `transition_beat()`에서 처리됩니다.

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
apply_stimulus 호출
  → 1. 감정 강도 조정 (관성 적용)
  → 2. scene.check_trigger(&state) — 대기 중 Focus의 조건 체크
  → 3. 조건 충족 시 → transition_beat():
       a. update_beat_relationship() — 관계 갱신 (감정 유지)
       b. scene.set_active_focus() + 새 Focus로 appraise
       c. merge_from_beat (이전 감정 + 새 감정 합치기)
  → 4. repo.save_scene() + StimulusResult.beat_changed = true
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

Claude(API)와 Bekay(브라우저)가 동시에 사용하는 심리 엔진 시뮬레이터. Mind Studio handlers는 `MindService` API의 얇은 래퍼입니다.

### 아키텍처

- **백엔드**: Axum REST API + SSE MCP 서버 (`src/bin/mind-studio/`)
- **프론트엔드**: Vite + React 18 + TypeScript + Zustand (`mind-studio-ui/`)
- 빌드 출력이 `src/bin/mind-studio/static/`에 배치되어 Axum `ServeDir`로 서빙
- **실시간 동기화**: `broadcast` 채널 → SSE `GET /api/events` → 프론트엔드 `EventSource`
  - MCP 도구 호출 또는 REST 핸들러가 상태 변경 시 `StateEvent` emit
  - 프론트엔드 `useStateSync` 훅이 이벤트 종류별 targeted re-fetch로 Zustand 업데이트
  - 이벤트 누락(lagged) 시 `resync` → 전체 refresh fallback

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
- **DialogueTestService** (`application/dialogue_test_service.rs`): `FormattedMindService` + `ConversationPort` 오케스트레이터

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

대화 루프:
```
appraise → start_session(prompt)
  → { send_message(상대 대사) → ChatResponse(text + timings) → apply_stimulus(PAD) → [Beat 전환 시 update_system_prompt] }
  → end_session → after_dialogue(관계 갱신)
```

## 외부 문서 인덱스

- **API 레퍼런스**: [`docs/api/api-reference.md`](docs/api/api-reference.md) — 공개 API, DTO, 포트, 도메인 타입
- **통합 가이드**: [`docs/api/integration-guide.md`](docs/api/integration-guide.md) — 외부 프로젝트 통합 단계별 가이드
- **아키텍처 v2**: [`docs/architecture/architecture-v2.md`](docs/architecture/architecture-v2.md)
- **아키텍처 v3 (EventBus/CQRS)**: [`docs/architecture/system-design-eventbus-cqrs.md`](docs/architecture/system-design-eventbus-cqrs.md) — EventBus, CQRS, Event Sourcing, Multi-Agent, RAG 시스템 디자인
- **프론트엔드 아키텍처**: [`docs/architecture/frontend-architecture.md`](docs/architecture/frontend-architecture.md) — Vite+React+Zustand 구조, 스토어 설계, 데이터 흐름, 컴포넌트 트리
- **협업 워크플로우**: [`docs/collaboration-workflow.md`](docs/collaboration-workflow.md)
- **감정 엔진**: [`docs/emotion/`](docs/emotion/) — OCC 모델, HEXACO 매핑, PAD 앵커 매트릭스, appraisal 엔진 설계
- **성격 모델**: [`docs/personality/`](docs/personality/) — HEXACO 6차원 facet 상세
- **가이드 매핑**: [`docs/guide/guide-mapping-table.md`](docs/guide/guide-mapping-table.md)
- **테스트 스크립트**: `mcp/skills/npc-scenario-creator/SKILL.md` (4-1단계) + `mcp/skills/npc-mind-testing/SKILL.md` (원칙 4, 커서 관리)
- **언어 설정**: [`docs/locale-guide.md`](docs/locale-guide.md)
- **MCP 서버 설정**: `.mcp.json` (프로젝트 루트)
