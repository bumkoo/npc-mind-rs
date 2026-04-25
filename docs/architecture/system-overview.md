# NPC Mind Engine — 시스템 구조 개관

> **문서 목적.** 현재 리포지토리(2026-04 기준, B5.3 · Memory Step E3.3 반영)의 전체 구조를 한눈에 파악하기 위한 **전략적 개관(strategic overview)** 이다.
> 세부 설계·튜닝·데이터 모델은 각 하위 문서로 위임한다. 이 문서는 "어디에 무엇이 있는가 / 왜 그렇게 나눴는가" 에 집중한다.
>
> **읽는 순서 제안.**
> 1. §1 한 문장 요약 → §2 설계 원칙 → §3 레이어 개관
> 2. §4 런타임 흐름 3종 (dispatch_v2 / 대화 루프 / Memory 주입) 로 시스템이 "어떻게 움직이는지" 머릿속에 그린다
> 3. §5 기능 모듈 현황 으로 지금까지 구현된 Phase/Step 지도 확인
> 4. §7 Deep-Dive 후보 목록에서 다음 세션에서 자세히 파고들 주제 선택

---

## 1. 한 문장 요약

**NPC Mind Engine은 HEXACO 성격이 OCC 감정을 생성하고, 이를 LLM이 "연기"할 수 있도록 가이드·대사·기억 주입을 관리하는 Rust 라이브러리**다.
라이브러리는 `Director` / `CommandDispatcher::dispatch_v2` 를 유일한 진입점으로 하여, EventBus · CQRS · Event Sourcing · Multi-Handler 아키텍처 위에서 동작한다. Mind Studio(Axum + React)는 이 엔진을 사람이 직접 구동·관찰할 수 있도록 감싸놓은 개발 도구다.

---

## 2. 설계 원칙 (Why)

| 원칙 | 의도 | 현재 구현상의 귀결 |
|---|---|---|
| **Hexagonal + DDD** | 도메인(감정·관계·기억)을 외부 I/O(LLM·DB·웹)로부터 격리 | `domain/`은 외부 deps 없음. `ports.rs`가 경계. adapter/presentation/bin만 바깥 세계에 닿음 |
| **CQRS + Event Sourcing** | 상태 변화를 "무엇이 일어났는가"로 표현해 재현·디버깅·멀티 핸들러 연결을 쉽게 | 단 하나의 write 경로 `dispatch_v2`. 모든 상태 변화는 `DomainEvent`로 append. Projection은 별개 |
| **Runtime-agnostic core** | 엔진이 Bevy·async-std 등 임의 게임 런타임에 얹힐 수 있어야 함 | 코어는 `tokio::sync::broadcast`만 내부 사용. 공개 API는 `futures::Stream`. `Spawner` trait으로 task 생성 주입. `tokio::spawn` 금지 |
| **Observable-by-default** | 심리 엔진은 "왜 이런 감정이 나왔는지" 설명되어야 함 | Event Log 보존 + SSE fan-out + Mind Studio UI로 실시간 관찰 |
| **Feature gating** | 임베딩·LLM·웹 서버는 선택적 비용 | `embed` / `chat` / `mind-studio` / `listener_perspective` feature로 분할 |
| **단일 write 경로** | 우회로 금지 → 상태 drift 방지 | v0.3.0에서 v1 경로(Pipeline/MindService/shadow_v2) 전면 제거. `dispatch_v2`만 존재 |

---

## 3. 레이어 개관

```
┌────────────────────────────────────────────────────────────────────────┐
│ Presentation / Binaries                                                │
│   - presentation/   : LocaleFormatter (ko, en TOML)                    │
│   - bin/mind-studio : Axum REST + SSE + static UI (dev tool)           │
│                       /api/projection/{emotion,relationship,scene}     │
│                       /api/projection/trace/{cid} — 인과 사슬 조회      │
│                         (flat events + tree {event,children} 동봉)     │
├────────────────────────────────────────────────────────────────────────┤
│ Application (조립·흐름 제어)  —  라이브러리 사용자의 진입점              │
│   Director                (multi-scene facade, Spawner 주입)            │
│   CommandDispatcher       (dispatch_v2 = 유일 write 경로)              │
│   DialogueOrchestrator [chat]    (LLM 다턴 오케스트레이터)                     │
│   EventBus / EventStore   (broadcast fan-out / append-only log,        │
│                            dispatch 단위 correlation_id 자동 부착 +    │
│                            cmd 내 parent_event_id / cascade_depth 추적)│
│   MemoryProjector [embed]     (broadcast 구독 → 임베딩 → RAG 저장)          │
│   Handler Agents          : ScenePolicy / EmotionPolicy / StimulusPolicy /│
│                             GuidePolicy / RelationshipPolicy /           │
│                             InformationPolicy / RumorPolicy /            │
│                             WorldOverlayPolicy                          │
│   Inline Projection/      : Emotion·Relationship·Scene Projection +    │
│   Ingestion Handlers        TellingIngestion·RumorDistribution·        │
│                             WorldOverlay·RelationshipMemory·           │
│                             SceneConsolidation                         │
├────────────────────────────────────────────────────────────────────────┤
│ Ports (src/ports.rs) — 헥사고날 경계                                    │
│   MindRepository · Appraiser · StimulusProcessor · GuideFormatter      │
│   UtteranceAnalyzer · ConversationPort · LlamaServerMonitor            │
│   MemoryStore · RumorStore · MemoryFramer · EventStore                 │
├────────────────────────────────────────────────────────────────────────┤
│ Domain — 순수 로직, 외부 의존 없음                                      │
│   personality (HEXACO 6차원 facet + 가중치)                             │
│   emotion     (OCC 22 감정 + compound, AppraisalEngine)                │
│   pad         (PAD 좌표 / 앵커 기반 자극 매핑)                          │
│   relationship (closeness/trust + change cause)                        │
│   scene / beat / focus / trigger                                       │
│   memory (MemoryEntry, Scope, Source, Provenance, Layer, Ranker)       │
│   rumor  (Rumor aggregate, RumorOrigin, ReachPolicy, Hop, Distortion)  │
│   event  (DomainEvent, EventPayload 28종, AggregateKey)                │
│   listener_perspective [feature] (화자 PAD → 청자 PAD Converter)        │
│   tuning  (튜닝 상수 중앙 관리)                                          │
├────────────────────────────────────────────────────────────────────────┤
│ Adapter — 포트 구현                                                     │
│   InMemoryRepository (multi-scene HashMap, 기본 MindRepository)         │
│   OrtEmbedder [embed]        (bge-m3 ONNX)                              │
│   RigChatAdapter [chat]      (rig-core, llama-server timings 캡처)     │
│   SqliteMemoryStore [embed]  (FTS5 trigram + sqlite-vec vec0, v2 스키마)│
│   SqliteRumorStore [embed]                                              │
│   InMemoryEventStore                                                    │
└────────────────────────────────────────────────────────────────────────┘
```

### 3.1 디렉토리 ↔ 역할 매핑

| 디렉토리 | 역할 | 외부 deps 허용? |
|---|---|---|
| `src/domain/` | 순수 도메인 로직 (성격·감정·PAD·관계·Scene·기억·이벤트 타입) | 불가 — std + serde 수준 |
| `src/ports.rs` | 헥사고날 경계 trait 정의 | 불가 |
| `src/application/` | 도메인 조립, CQRS dispatcher, EventBus, Handler, DialogueOrchestrator, Director | 가능(도메인/포트 경유) |
| `src/adapter/` | 포트 구현체 | 외부 라이브러리 OK |
| `src/presentation/` | locale formatter, memory framer | 표준 라이브러리 위주 |
| `src/bin/mind-studio/` | Axum REST + SSE 서버 + static UI | 전체 deps 허용 |
| `mind-studio-ui/` | Vite + React + Zustand 프론트엔드 | npm 생태계 |
| `locales/` | ko/en TOML + PAD 앵커 | - |
| `data/presets/` | 캐릭터 프리셋·테스트 시나리오 JSON | - |
| `tests/` | 통합 테스트 (TestContext 공유) | - |

---

## 4. 런타임 흐름 (데이터가 흐르는 3개의 축)

### 4.1 Command → Event (write 경로, `dispatch_v2`)

```
Command (e.g. Appraise / ApplyStimulus / TellInformation / SpreadRumor / ApplyWorldEvent)
  │
  │ CommandDispatcher::dispatch_v2(cmd)
  ▼
build_initial_event(cmd)            ──▶  *Requested event (depth=0)
  │
  │ Transactional phase (BFS, 최대 MAX_CASCADE_DEPTH=4, MAX_EVENTS_PER_COMMAND=20)
  │   ├─ priority 오름차순 handler 실행
  │   │     SCENE_START(5) → EMOTION_APPRAISAL(10) → STIMULUS_APPLICATION(15)
  │   │   → GUIDE_GENERATION(20) → WORLD_OVERLAY(25) → RELATIONSHIP_UPDATE(30)
  │   │   → INFORMATION_TELLING(35) → RUMOR_SPREAD(40)
  │   ├─ HandlerShared scratchpad에 상태 전파 (emotion_state/relationship/scene/guide/clear_*)
  │   └─ follow_up_events → queue에 depth+1로 push
  ▼
apply_shared_to_repository (save_* / clear_*)
  │
  │ Commit phase
  ▼
staging_buffer → EventStore.append (seq/id 할당)
  │
  │ Inline phase (best-effort, 에러는 로그만)
  ▼
Inline handlers:
  EmotionProjection / RelationshipProjection / SceneProjection
  TellingIngestion(C2) · RumorDistribution(C3)
  WorldOverlay(45) · RelationshipMemory(50) · SceneConsolidation(60)
  │
  │ Fanout phase
  ▼
EventBus.publish → tokio::broadcast → subscribe() 스트림
                   ├─ MemoryProjector [embed] (임베딩 + RAG 색인)
                   ├─ Mind Studio SSE (/api/events)
                   └─ 사용자 정의 구독자 (게임 엔진 등)
```

### 4.2 LLM 대화 루프 (`DialogueOrchestrator`, `chat` feature)

```
start_session(sid, npc, partner, situation?)
  → dispatch_v2(Command::Appraise)
       → AppraiseRequested → EmotionAppraised → GuideGenerated
  → (with_memory 부착 시) inject_memory_push: MemoryRanker Top-K → 프롬프트 블록 prepend
  → ConversationPort.start_session(prompt)

turn(sid, utterance, pad?, situation?)
  → dispatch_v2(Command::ApplyStimulus)
       → StimulusApplied  (+ BeatTransitioned?)
  → BeatTransitioned 발생 시:
       - inject_memory_push 재실행
       - ConversationPort.update_system_prompt
  → ConversationPort.send_message → ChatResponse { text, timings }
  → listener_perspective 변환 (feature on일 때 자동)

end_session(sid, significance?)
  → ConversationPort.end_session
  → significance 있으면 dispatch_v2(Command::EndDialogue)
       → RelationshipUpdated + SceneEnded + EmotionCleared
       → Inline: RelationshipMemory + SceneConsolidation 트리거
```

### 4.3 Memory RAG 주입 (Step B, `embed + chat`)

```
사용자 대사 / 시스템 상황
  │
  ▼
query 임베딩 (analyzer 있으면 analyze_with_embedding, 없으면 text-only)
  │
  ▼
MemoryStore.search(MemoryQuery {
    scope_filter: NpcAllowed(npc)      // Personal + World + Relationship(참여)
    exclude_superseded / exclude_consolidated_source
    min_retention ≥ MEMORY_RETENTION_CUTOFF (0.10)
    limit: MEMORY_PUSH_TOP_K * 3       // Ranker용 oversample
})
  │
  ▼
MemoryRanker (2-stage):
  1) Source 우선순위 (Seeded > Experienced > Witnessed > Heard > Rumor)
  2) 5요소 점수: vec × retention × source × emotion × recency
  │
  ▼
Top-K 선택 + MemoryStore.record_recall(id, now_ms)
  │
  ▼
LocaleMemoryFramer.frame_block
  "[겪음] … / [목격] … / [전해 들음] … / [강호에 떠도는 소문] …"
  │
  ▼
format!("{block}{system_prompt}") → ConversationPort
```

---

## 5. 기능 모듈 현황 (완료된 Phase/Step 지도)

각 항목의 상세는 CLAUDE.md의 "구현 현황" 표와 해당 doc 링크 참조. 여기는 **어떤 블록이 어느 상태인지** 를 한 눈에 보기 위함.

### 5.1 핵심 엔진 (완료)
- **Phase 1–4**: EventBus / EventStore / Projection / CQRS Policy(Emotion/Stimulus/Guide/Relationship) / MemoryProjector / DialogueOrchestrator
- **B안 B0–B4 S4**: EventHandler trait → v2 단일 경로 완성. Director(multi-scene facade) + Spawner 추상화로 런타임 중립
- **B안 B5.1–B5.3**: v1 전면 제거. `shared_dispatcher`로 Mind Studio 동기화 모델 통일

### 5.2 Memory/Rumor 서브시스템 (완료 + 일부 진행)
- **Step A (foundation)**: Scope/Source/Provenance/Layer + Ranker 2단계 + SQLite v2 마이그레이션
- **Step B (push 주입)**: `DialogueOrchestrator::with_memory`, `BeatTransitioned` 재주입, 라벨링된 framer
- **Step C1 (Rumor 도메인)**: Rumor aggregate, RumorStore, 불변식 I-RU-1~6
- **Step C2 (TellInformation)**: 화자→청자 정보 전달 → `InformationTold` + `MemoryEntry(Heard/Rumor)`
- **Step C3 (Rumor 확산)**: Seed/Spread, Canonical 3-tier 해소, 홉 감쇠
- **Step D (Ingestion 번들)**: WorldOverlay + SceneConsolidation + RelationshipMemory Inline 핸들러
- **Step E1 (Mind Studio 배선)**: 10 REST 엔드포인트 + 5 SSE variant + `with_memory_full` + `with_rumor` 자동 부착
- **Step E3.3 (시드 조회 UI + 로드 경고)**: 완료. 편집 GUI는 범위 외
- **Step F (미정)**: Memory 이벤트 팬아웃, pull recall, rumor distort/fade, LLM-based consolidator

### 5.3 Listener-Perspective Converter (Phase 7, 완료)
- default-on feature(`listener_perspective`). 88% baseline
- 한국어 정규식 프리필터 → sign/magnitude k-NN → 화자 PAD를 청자 관점으로 변환
- DialogueOrchestrator · Mind Studio 양쪽에서 자동 적용

### 5.4 Mind Studio (dev tool, 완료)
- 백엔드: Axum REST + SSE + MCP 서버, `/api/*` (메인) + `/api/v2/*` (Director shadow)
- 프론트: Vite + React 18 + Zustand, NPC/관계/오브젝트 CRUD · 시나리오 로드 · 테스트 레포트
- Memory Step E1·E3.3 UI (시드 조회 + 로드 경고 토스트)

### 5.5 미구현 / 다음 로드맵
- **Phase 5** StoryAgent (서사 진행 판단)
- **Phase 6** Tool 시스템 (ToolRegistry)
- **Phase 7(원래 번호)** WorldKnowledgeStore 통합
- **Phase 8** SummaryAgent (컨텍스트 윈도우 관리)
- **Memory Step F** Memory 이벤트 팬아웃, pull recall, LLM consolidator, rumor distort/fade

---

## 6. 주요 결정과 트레이드오프 (What · Why · Cost)

| 결정 | 동기 | 트레이드오프 |
|---|---|---|
| `dispatch_v2` 단일 write 경로 | v1과의 이중 경로가 drift·테스트 분열을 유발 | 마이그레이션 비용(B5.1–B5.3) 치렀고 이제 우회로 없음 |
| BFS 캐스케이드 (깊이·이벤트 수 가드) | handler 간 follow-up 체인이 무한 증식 가능 | `MAX_CASCADE_DEPTH=4` / `MAX_EVENTS_PER_COMMAND=20` — 초과 시 `CascadeTooDeep`·`EventBudgetExceeded` |
| tokio::broadcast (at-most-once + Lagged 복구) | 낮은 지연 + 다수 구독자 | 느린 구독자는 lag 가능. MemoryProjector는 `subscribe_with_lag + EventStore.get_events_after_id`로 replay |
| `Spawner` trait로 task 생성 주입 | 코어가 tokio/rt에 강결합되면 Bevy 등 불가 | 호출자가 클로저(`Arc::new(|fut| tokio::spawn(fut))`)를 직접 제공해야 함 |
| SQLite FTS5 trigram + sqlite-vec vec0 | 한글 FTS(단어 경계 문제) + 임베딩 ANN을 한 파일에 | vec0 스키마에 차원 고정 → 모델 교체 시 DB 재생성 필요 |
| `InMemoryEventStore` 프로세스 수명 누적 | 단순·빠름, 개발 툴 용도 | 장기 실행 시 메모리·`next_sequence` O(N) scan 부담. 영구 store는 Phase 8+ |
| Listener-Perspective default-on | 청자 관점 변환이 대화 품질의 큰 부분 | 미세한 성능 비용 + 회귀 감시 전용 테스트(`dialogue_no_lp_passthrough`) 유지 |
| Mind Studio `shared_dispatcher` + `rebuild_repo_from_inner` | per-request dispatcher/snapshot은 drift 위험 | UI write 시점에만 재구성 비용 발생, 트레이드오프 수용 |

---

## 7. 다음 세션에서 Deep-Dive 할 후보 (권장 순서)

> 이 문서는 "지도"로 남기고, 아래 각 항목을 개별 세션에서 파일 단위로 자세히 기술한다.

| # | 주제 | 이유 / 담을 내용 | 관련 코드·기존 문서 |
|---|---|---|---|
| 1 | **dispatch_v2 내부 동작** ✅ | BFS 캐스케이드 / HandlerShared / 가드·에러 / 이벤트 라우팅 키 | [`dispatch-v2-internals.md`](dispatch-v2-internals.md) · `application/command/dispatcher.rs`, `handler_v2.rs`, `priority.rs` |
| 2 | **EventHandler 카탈로그** ✅ | 8 transactional + 8 inline handler의 입력/출력/부수효과 매트릭스 | [`event-handler-catalog.md`](event-handler-catalog.md) · `application/command/policies/*`, `application/command/*_handler.rs` |
| 3 | **Memory Ranker + Framer 파이프라인** | Scope filter, 5요소 점수, Source 우선, framer locale 전략, 회귀 실패 패턴 | `domain/memory/*`, `presentation/memory_formatter.rs` · `docs/memory/03-implementation-design.md` |
| 4 | **Rumor 수명주기** | Seed/Spread/Distort/Fade 불변식, Canonical 3-tier 해소, aggregate key 라우팅 | `domain/rumor.rs`, `agents/rumor_policy.rs`, `rumor_distribution_handler.rs` |
| 5 | **Scene · Beat · Focus Trigger 엔진** | 상태기계, `check_trigger`, `merge_from_beat`, B4 S3 Option A partner_id 분리 | `domain/scene/*`, `agents/stimulus_policy.rs`, `agents/scene_policy.rs` |
| 6 | **감정 appraisal 내부 구조** | event/action/object 축, compound 결합, 성격 가중치 패턴, 관계 변조 | `domain/emotion/*` · `docs/emotion/*` |
| 7 | **Listener-Perspective Converter (Phase 7)** | 프리필터 → sign/magnitude k-NN → register 전략, 88% baseline의 한계 | `domain/listener_perspective/*` · `docs/emotion/phase7-converter-integration.md`, `sign-classifier-design.md` |
| 8 | **LLM 어댑터 레이어** | rig-core 통합, `TimingsCapturingClient`, `LlamaServerMonitor`, 커넥션 풀 공유 | `adapter/rig_chat.rs`, `adapter/llama_timings.rs` |
| 9 | **Mind Studio 동기화 모델** | `shared_dispatcher`, `rebuild_repo_from_inner`, SSE `StateEvent` fanout, `/api/v2/*` shadow | `bin/mind-studio/*`, `domain_sync.rs` · `docs/architecture/frontend-architecture.md` |
| 10 | **프론트엔드 아키텍처 (Zustand 5 스토어)** | 스토어 책임 분리, `useStateSync` re-fetch 전략, 시드 패널 UX | `mind-studio-ui/src/**` · `docs/architecture/frontend-architecture.md` |
| 11 | **튜닝 상수 지도** | `tuning.rs` 중앙 상수 + 파일별 로컬 상수(personality W_*, pad_table 22감정) | `domain/tuning.rs`, `personality.rs`, `pad_table.rs` |
| 12 | **테스트 전략** | `TestContext`, HandlerTestHarness, Feature-gated 통합 테스트, 회귀 감시 테스트들 | `tests/common/*`, `tests/dispatch_v2_test.rs`, `dialogue_no_lp_passthrough` 등 |
| 13 | **Runtime-agnostic 설계의 실전 제약** | `Spawner` 주입 경로, Bevy 통합 시 체크리스트, tokio 누출 방지 | `application/director/spawner.rs`, `event_bus.rs` |
| 14 | **미구현 Phase 5–8 로드맵** | StoryAgent/Tool/WorldKnowledge/Summary의 인터페이스 초안 + 삽입 지점 | CLAUDE.md 구현 현황 표, `docs/architecture/b-plan-implementation.md` |

---

## 8. 관련 문서 지도

현재 리포지토리의 아키텍처·도메인 관련 문서.

**아키텍처 계열**
- [`architecture-v2.md`](architecture-v2.md) — v2 전반 구조 (B안 이전)
- [`b-plan-implementation.md`](b-plan-implementation.md) — B안 단계별 설계
- [`system-design-eventbus-cqrs.md`](system-design-eventbus-cqrs.md) — EventBus / CQRS / Event Sourcing / Multi-Handler / RAG 상세
- [`frontend-architecture.md`](frontend-architecture.md) — Mind Studio UI 구조
- [`adr-001-rig-agent-integration.md`](adr-001-rig-agent-integration.md) — rig 통합 결정
- [`unified-event-protocol-analysis.md`](unified-event-protocol-analysis.md), [`situation-structure.md`](situation-structure.md), [`npc-tool-memory-design.md`](npc-tool-memory-design.md), [`system-design-review-2026-04.md`](system-design-review-2026-04.md)

**도메인 계열**
- [`../emotion/`](../emotion/) — OCC 모델, HEXACO↔OCC 매핑, PAD 앵커, appraisal 엔진, Listener-Perspective
- [`../personality/`](../personality/) — HEXACO 6차원 facet 가이드
- [`../memory/`](../memory/) — 용어·DDD·구현 설계
- [`../guide/guide-mapping-table.md`](../guide/guide-mapping-table.md) — 감정 → 가이드 매핑
- [`../api/`](../api/) — API 레퍼런스 / 통합 가이드
- [`../locale-guide.md`](../locale-guide.md), [`../collaboration-workflow.md`](../collaboration-workflow.md)

---

## 9. 이 문서의 유지 정책

- **언제 갱신하나.** 레이어 배치가 바뀔 때 / 새 Phase·Step이 "완료"로 전환될 때 / Deep-Dive 문서가 신설될 때 §7 표에 링크 추가.
- **언제 갱신하지 않나.** 튜닝 상수 숫자, 개별 handler 내부 로직, 테스트 추가 — 이것들은 `tuning.rs` · 해당 deep-dive 문서 · 코드가 진실 원천.
- **원칙.** 이 문서는 **지도**이지 **실상**이 아니다. 숫자·시그니처·파일 경로가 상세하게 필요해지는 순간, 이 문서에서 빼서 deep-dive 문서로 옮긴다.
