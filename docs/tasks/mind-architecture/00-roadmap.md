# Mind Architecture 마이그레이션 로드맵

> 살아있는 문서. relationships.md / action_triggers.md의 *target architecture*와 npc-mind-rs의 *current code*를 잇는 다리.
> 새 Phase 시작·검증 게이트 통과·결정 변경 시 이 문서 갱신.
>
> 별도 트랙 — Worldbuilding 도메인 phasing은 `../world building/00-roadmap.md` 참조.

## 1. 이 문서의 정체

`world building/00-roadmap.md`(Phase 0~N: Group/Person/Place/Atlas/Event/Era 등 도메인 구축)와는 *다른 축*의 작업. 두 트랙이 나란히 진행됨:

| | world building | mind-architecture (본 문서) |
|---|---|---|
| 축 | **무엇을** 만들 것인가 | **어떻게** 마이그레이션할 것인가 |
| 단위 | 데이터 도메인 phase | 아키텍처 phase |
| 산출물 | 도메인 구조체 + SQLite 스키마 | 도메인 모델 변경 + application service + read model |
| 관련 docs | `_schema.md`, `wuxia-core/` | `relationships.md` v0.7, `action_triggers.md` v0.1 |

**왜 이 문서가 필요한가**: relationships.md v0.7은 *목표 아키텍처*(4축 + BondKind + Channel 1/2/3 + ActionTrigger)를 정의. 코드는 *부분 구현*(3축 Relationship + DialogueOrchestrator/dispatch_v2 + RelationshipPolicy 무조건 follow-up). 누군가 docs만 읽으면 더 많이 구현된 것으로 오해. 이 문서가 그 갭을 명시.

## 2. 현재 상태 (2026-05-12 Phase 1/1.5/1.6 완료 후)

### 2.1 검증 수준 표기

본 절의 모든 항목에 verification level 표시:

- ✅ **직접 읽음** — 이번 spot-check에서 해당 파일 본문 확인
- ◯ **CLAUDE.md 인용** — 사용자가 갱신한 SOR 문서 기반, 현재 코드 직접 미검증
- △ **파일/디렉토리 존재만 확인** — list_directory 결과만, 내용 미확인

미래에 ◯/△가 ✅로 승격되려면 *해당 파일을 직접 읽은 시점* — Phase 작업 spec 작성 시 자연 발생.

### 2.2 Domain Layer (`src/domain/`)

| 모듈 | 상태 | 검증 |
|---|---|---|
| `relationship.rs` | 3축 Value Object (closeness/trust/power, ±1.0, Score 타입) | ✅ 1~120줄 |
| `event.rs` | DomainEvent + EventMetadata + 31 EventKind (Phase 1: +`DialogueReflected`) + RelationshipChangeCause 5 variants + `DialogueEndRequested.reflection` 필드 | ✅ 1~300줄 |
| `emotion/` | AppraisalEngine 모듈화 (Event/Action/Object/Compound 서브) | ◯ |
| `pad.rs`, `pad_anchors.rs`, `pad_table.rs` | StimulusEngine 기반 | △ |
| `personality.rs` | HEXACO 24 facet, Score 타입. **Phase 1 (A-min)**: `Npc.inner_compass: Option<String>` 필드 + `compass_short_label()` 메서드 | ✅ Phase 1 A-min 부분 |
| **`reflection.rs`** | **Phase 1 신규**: `TurnSnapshot` + `compute_significance(turns) -> f32` (4 신호 가중, 8.36µs/call) + `ReflectionResult` + `DeclarativeEventPlaceholder` + `PartnershipEventPlaceholder` | ✅ Phase 1 |
| `listener_perspective/` | Phase 7 마이그레이션, feature flag `listener_perspective` | △ (userMemories 인용) |

### 2.2.5 Ports Layer (`src/ports/`)

ISP 기반 모듈 분할 (이전 단일 `ports.rs`):

| 모듈 | 트레이트 | 검증 |
|---|---|---|
| `persistence.rs` | `MindRepository` + `NpcWorld` + `EmotionStore` + `SceneStore` (super-trait 분리) | ◯ |
| `personality.rs` | `PersonalityProfile`, `PadAnchorSource`, `AnchorLoadError` | ◯ |
| `guide.rs` | `GuideFormatter`, `Appraiser`, `StimulusProcessor` | ◯ |
| `memory.rs` | `MemoryStore` + `RumorStore` + `MemoryFramer` | ◯ |
| `analysis.rs` | `UtteranceAnalyzer` | ◯ |
| `chat.rs` [chat] | `ConversationPort` + `ChatResponse` + `InferenceTimings` + `LlmModelInfo` + `ConversationError(Timeout 포함)` | ◯ |
| `monitoring.rs` [chat] | `InferenceServerMonitor` + `ServerHealth` + `InferenceSlotInfo` + `ServerMetrics` (이전 Llama* → Inference* 일반화) | ◯ |
| **`reflection.rs`** [chat] | **Phase 1 신규**: `ReflectionPort` trait + `ReflectionPrompt` + `ReflectionError` | ✅ Phase 1 |

### 2.3 Application Layer (`src/application/`)

> **v0.3.0 정정 노트**: v1 경로 (`MindService` / `EventAwareMindService` / `Pipeline` / `CommandDispatcher::dispatch` / `shadow_v2`)는 모두 제거됨. 진입점은 `Director` / `CommandDispatcher::dispatch_v2` / `DialogueOrchestrator` (chat) 셋. v0.1 초안의 architecture-v2.md 인용은 stale이었음 — 이번 갱신에서 정정.

| 모듈 | 상태 | 검증 |
|---|---|---|
| `command/dispatcher.rs` (`CommandDispatcher<R>`) | dispatch_v2 단일 진입점. `with_default_handlers` / `with_memory` / `with_memory_full` / `with_world_overlay` / `with_scene_consolidation` / `with_rumor` 빌더. **Phase 1: `MAX_EVENTS_PER_COMMAND` 21 → 22** | ◯ |
| `command/uow.rs` (`UnitOfWork`) | Transactional BFS 변경 누적 → 일괄 commit. `HandlerShared`는 *출력 호환용 쉐이프*로 변경 | ◯ |
| `command/types.rs` | `Command::EndDialogue.reflection: Option<ReflectionResult>` 필드 (Phase 1 추가) | ✅ Phase 1 |
| `command/policies/` (8 핸들러) | `emotion`/`stimulus`/`guide`/`relationship`/`scene`/`information`/`rumor`/`world_overlay` (이전 `agents/` → `policies/` 리네임). **Phase 1: `RelationshipPolicy.handle_dialogue_end` 게이트 + 4 follow-up + `outer_loop_entry()` helper** | ✅ Phase 1 (RelationshipPolicy) |
| `command/{telling_ingestion,rumor_distribution,world_overlay,scene_consolidation,relationship_memory}_handler.rs` | Inline 핸들러 5종 — Step C/D 메모리 흡수 | ◯ |
| **`reflection_service.rs`** [chat] | **Phase 1 신규**: `ReflectionRunner` trait (dyn-compatible) + `ReflectionService<P: ReflectionPort>` + `ReflectionPromptBuilder` trait + `DefaultReflectionPromptBuilder` + `strip_json_envelope` helper (markdown fence) | ✅ Phase 1 |
| `dialogue_orchestrator.rs` (`DialogueOrchestrator<R, C>`) [chat] | LLM 다턴 오케스트레이터. `start_session`/`turn`/`end_session`. `BeatTransitioned` 발생 시 `update_system_prompt`. **Phase 1: `with_reflection(svc)` + `turn_buffers: HashMap<SessionId, Vec<TurnSnapshot>>` + `run_reflection()` helper** | ✅ Phase 1 |
| `director/` (`Director<R>`) | 다중 Scene facade. `start_scene`/`dispatch_to`/`end_scene`/`active_scenes`. **Phase 1 미통합** — `Director::end_scene` 경로의 Reflection 부착 미구현 (별도 작업) | △ Reflection 미통합 |
| `event_bus.rs` | tokio broadcast, futures::Stream 노출, lag 처리 | ✅ 1~80줄 |
| `event_store.rs` | 이벤트 영속화 (commit staging buffer) | △ |
| `memory_projector.rs` | EventBus 구독 기억 인덱싱 [embed] | ◯ |
| `dto/` (7 도메인 모듈) | `emotion`/`guide`/`information`/`relationship`/`rumor`/`scene`/`world` 분할. **Phase 1: `AfterDialogueResponse.reflection: Option<ReflectionResult>` 필드 추가** | ✅ Phase 1 |
| `adapter/reflection_via_chat.rs` [chat] | **Phase 1 신규**: `ConversationBackedReflectionPort<C>` — 같은 LLM 서버에 *별도 세션*, KV 캐시 분리 | ✅ Phase 1 |
| `error.rs` (`MindServiceError`) | 5 variants: NpcNotFound · RelationshipNotFound · InvalidSituation · EmotionStateNotFound · LocaleError | ◯ |

### 2.3.5 Mind Studio 통합 상태 (Phase 1.5/1.6 결과)

Phase 1 본체의 reflection 게이트는 *DialogueOrchestrator 경로*에 박혀 있어 Mind Studio
(`state.chat + domain_sync` ad-hoc) 경로에서는 미동작 → Phase 1.5에서 *mirror*. Phase 1.6에서
manual SSE emit을 EventBus 구독으로 일원화.

| 영역 | 상태 | 검증 |
|---|---|---|
| `AppState.reflection_service: Option<Arc<dyn ReflectionRunner>>` + `with_reflection()` | ✅ Phase 1.5 | [`state.rs`](../../../src/bin/mind-studio/state.rs) |
| `StateInner.turn_buffers: HashMap<String, Vec<TurnSnapshot>>` (chat-gated, serde skip) | ✅ Phase 1.5 | 동일 |
| `main.rs` 부팅 시 별도 RigChatAdapter → ConversationBackedReflectionPort → ReflectionService 자동 부착 | ✅ Phase 1.5 | [`main.rs`](../../../src/bin/mind-studio/main.rs) |
| `StudioService::process_chat_turn_result` 매 turn TurnSnapshot 누적 (DialogueOrchestrator.turn() ⑦ mirror) | ✅ Phase 1.5 | [`studio_service.rs`](../../../src/bin/mind-studio/studio_service.rs) |
| `StudioService::perform_after_dialogue(state, req, session_id)` + `run_reflection_for_session` | ✅ Phase 1.5 | 동일 |
| `domain_sync::dispatch_end_dialogue(state, inner, req, reflection)` 시그니처 확장 | ✅ Phase 1.5 | [`domain_sync.rs`](../../../src/bin/mind-studio/domain_sync.rs) |
| Frontend ReflectionView + '반추' 탭 + `useResultStore.lastAfterDialogue` | ✅ Phase 1.5 | [`ReflectionView.tsx`](../../../mind-studio-ui/src/components/result/ReflectionView.tsx) |
| `event_bridge.rs` (EventBus → SSE 자동 매핑 9개 도메인 이벤트) | ✅ Phase 1.6 | [`event_bridge.rs`](../../../src/bin/mind-studio/event_bridge.rs) |
| manual `state.emit()` 도메인 사실 11곳 제거 (UI-only emit만 잔존) | ✅ Phase 1.6 | studio_service · mcp_server · handlers/scenario · handlers/rumor |
| Director 경로 (`/api/v2/scenes/*`) SSE 자동 발행 | ⚠️ shared_dispatcher 경유 시만 동작 — `director_v2`는 *별도 dispatcher*라 본 bridge 범위 외 | 향후 통합 작업 후보 |
| `Director.end_scene` Reflection 통합 | ❌ 미구현 (디자인 결정 필요 — SceneTask turn_buffer 위치) | 별도 작업 |

### 2.4 Inner/Outer 골격 — 부분 구현됨 (★)

CLAUDE.md 기준 (v0.3.0 후), Inner/Outer 두 흐름은 다음 컴포넌트들로 *부분 구현*되어 있음:

**Inner Loop (대화 turn)** — `DialogueOrchestrator.turn` (chat feature):
- `Command::ApplyStimulus.dispatch_v2().await` → 도메인 동기 처리
- 처리 결과 events에 `BeatTransitioned` 포함 시 → `ConversationPort::update_system_prompt` 호출
- `ConversationPort::send_message` → 다음 turn

**Outer Loop 진입점** — `DialogueOrchestrator.end_session` (✅ Phase 1 완료):
- (reflection 있으면 또는 significance 있으면) `Command::EndDialogue.dispatch_v2().await`
- 도메인 안에서 `RelationshipPolicy`가 `DialogueEndRequested` 수신 → **4 follow-ups** 발행:
  1. `DialogueReflected` (항상, chitchat skip 케이스에도 박제)
  2. `RelationshipUpdated` (`outer_loop_entry()` 게이트 통과 시만 — chitchat은 skip)
  3. `EmotionCleared` (항상)
  4. `SceneEnded` (항상)
- chitchat (significance < 0.3 ∧ is_chitchat=true): **3 이벤트** (RelationshipUpdated skip)
- significant: **4 이벤트** 그대로
- legacy (reflection=None, significance=Some): **3 이벤트** (기존 무조건 동작 — RelationshipUpdated 발행, DialogueReflected 미발행)

→ Phase 1의 *gate 추가*는 완료. 아키텍처 자체(Director/CommandDispatcher/DialogueOrchestrator)는 그대로 유지. Phase 2에서 *declarative_events 분기*가 동일 게이트 위에 얹힘.

### 2.5 EventKind 인벤토리 (✅ 직접 verified, Phase 1 후 갱신)

```
Mind:           AppraiseRequested, EmotionAppraised,
                StimulusApplyRequested, StimulusApplied,
                BeatTransitioned, EmotionCleared
Scene:          SceneStartRequested, SceneStarted, SceneEnded
Relationship:   RelationshipUpdateRequested, RelationshipUpdated
Dialogue:       DialogueEndRequested, DialogueTurnCompleted,
                DialogueReflected ★ Phase 1 신규
Guide:          GuideRequested, GuideGenerated
Memory:         MemoryEntryCreated, MemoryEntrySuperseded,
                MemoryEntryConsolidated
Rumor:          SeedRumorRequested, SpreadRumorRequested,
                RumorSeeded, RumorSpread, RumorDistorted, RumorFaded
World:          ApplyWorldEventRequested, WorldEventOccurred,
                TellInformationRequested, InformationTold
```

총 30 → **31 EventKind** (`DialogueReflected` 추가). `EventPayload::DialogueEndRequested`에
`reflection: Option<ReflectionResult>` 필드 추가됨.

### 2.6 EventMetadata 현재 상태 (✅ 직접 verified)

```rust
pub struct EventMetadata {
    pub correlation_id: Option<u64>,
    pub parent_event_id: Option<EventId>,
    pub cascade_depth: u32,
}
```

**중요**: `parent_event_id`/`cascade_depth`는 *한 dispatch_v2 호출 내부의 연산 인과* 추적용. 한 NPC aggregate 안의 한 turn 처리에 한정됨. *cross-NPC 서사 인과*는 추적하지 않음. Phase 3b에서 `narrative_origin` 필드 추가 필요.

## 3. 목표 상태 (relationships.md v0.7 기준)

### 3.1 Relationship 모델
- **4 axes**: trust / affinity / respect / wariness (-100~+100, wariness는 0~+100)
- **BondKind**: 11 variants (지기 4 + Companion + Guardian + Mentor + 원수 4)
- **BondStatus**: 5 variants (Active/Resolved/Deceased/Dormant/Reactivating)
- **Partnership**: 4 variants (Spouse/Engaged/Lover/Separated)
- **type/type_history**: 자유 텍스트 + 이력
- 네 차원 *직교*

### 3.2 Inner/Outer 명시 분리
- Inner Loop: appraise → PAD → ActingGuide. axes 불변.
- Outer Loop: Reflection 분기 → axes/BondKind/BondStatus/Partnership 갱신.

### 3.3 Scene Boundary Reflection
- LLM (서사 의미) + Engine (정량 significance) 협업
- is_chitchat 게이트로 outer loop skip 가능
- `DialogueReflected` 도메인 이벤트로 박제 (replay 결정성)

### 3.4 3-channel transformation/partnership trigger
- Channel 1: Declarative (LLM emit + 사회적 일관성 검증 5종 + 적용 모드 4-tier)
- Channel 2: Temporal (BondKind 진입 시간 게이트 카운터 read model)
- Channel 3: External (EventPropagator + PropagationRule + narrative_origin)

### 3.5 ActionTriggerEvaluator (action_triggers.md v0.1)
- 5-dim feasibility (physical/power/social/self/moral)
- 29 ActionKind variants
- BondKind 분류 → 행동 emit
- 추모 행동 (HandleHeirloom 등) emit 통합

## 4. Gap Analysis

| 영역 | 현재 | 목표 | 갭 | Phase |
|---|---|---|---|---|
| Relationship 축 | 3 (±1.0) | 4 (±100) | 큼 — 모델 재작성 | 2 |
| BondKind | 없음 | 11 variants | 큼 | 2 |
| BondStatus | 없음 | 5 variants | 중 | 2 |
| Partnership | 없음 | 4 variants | 중 | 2 |
| type/type_history | 없음 | 자유 텍스트 + 이력 | 중 | 2 |
| Inner/Outer 분리 | 부분 (after_beat/after_dialogue) | 명시 분리 + Reflection 게이트 | 작음 | 1 |
| Reflection 단계 | 없음 | LLM + Engine 협업 service | 중 | 1 |
| Engine significance | 없음 (caller-supplied) | 결정론 계산 함수 | 작음 | 1 |
| is_chitchat 게이트 | 없음 (무조건 진입) | LLM 판정으로 outer loop skip | 작음 | 1 |
| DialogueReflected event | 없음 | 새 EventKind | 작음 | 1 |
| Channel 1 Declarative | 없음 | LLM emit + 검증 + 모드 | 큼 | 2 |
| Channel 2 Temporal | 없음 | BondKindCandidacy projection + 시간 게이트 | 큼 | 3a |
| Channel 3 External | 부분 (Rumor/Information 인프라 일부) | EventPropagator + PropagationRule | 큼 | 3b |
| narrative_origin | 없음 | EventMetadata 새 필드 | 작음 | 3b |
| ActionTriggerEvaluator | 없음 | 도메인 신설 (5-dim feasibility, 29 ActionKind) | 매우 큼 | 3c |
| 추모 행동 emit | 없음 | ActionTrigger의 한 분기 | 중 | 3c |

## 5. Phase 정의

### Phase 1 (v0.7) — Reflection + Significance + Chitchat Gate ✅ 완료 (2026-05-11)

**비포함**: 4축 마이그레이션, BondKind, ActionTrigger, Channel 2/3.

**포함**:
- 도메인: `compute_significance(turns)` 함수 (위치 Stage 0 결정 — `domain/relationship.rs` 또는 신설 `domain/reflection.rs`), `TurnSnapshot` 구조체, `DialogueReflected` 새 EventKind + payload (`domain/event.rs`)
- Application: `ReflectionService` 신설 (`application/reflection_service.rs`), `RelationshipPolicy` 진입 조건 변경 (`application/command/policies/relationship_policy.rs`). LLM 호출은 *dispatch_v2 바깥* (DialogueOrchestrator.end_session 안)에서 — UoW/dispatch 동기 fast-path 보존.
- MCP: `dialogue_end` tool에 reflection 결과 응답 노출 (디버깅용)
- 테스트: significance 4가지 신호 unit, 잡담/일상/결단 narrative integration

**위험**: 작음. 도메인 모델 그대로. 기존 테스트 영향 미미.

**검증 게이트**:
1. `cargo check` 통과
2. `cargo test` 회귀 0 + 신규 unit/integration 통과
3. Bench: dialogue 종료 시 LLM 1회 추가 호출 latency 측정
4. Narrative cases — significance *낮음/중간/높음* 3 밴드 모두 커버:
   - **낮음 (잡담)**: 길에서 만난 행인과의 무의미 대화 → is_chitchat=true, axes 변화 0, summary만 메모리 저장
   - **중간 (일상)**: 수련-춘설병 일상 무공 수련 대화 → is_chitchat=false, declarative_events 비어 있음, 기존 RelationshipPolicy 미세 axes 변화
   - **높음 (결단)**: 임충 산신묘 처단 사건 → DialogueReflected에 reasoning 박제, 기존 axes 큰 변화 작동

**산출물 spec**: [`task-rel-phase1-reflection.md`](task-rel-phase1-reflection.md) (v0.1 작성됨, 2026-05-10).

**완료 조건**: 위 4 게이트 통과 + Phase 1 checkpoint report.

**완료 결과**: 18 commits (`87c8b32` → `fb22400`). 1095 회귀 + Phase 1.5/1.6 ~12 신규 테스트.
상세 — [`phase1-checkpoint-report.md`](phase1-checkpoint-report.md). 핵심:
- `Npc.inner_compass: Option<String>` (A-min)
- `domain/reflection.rs` (TurnSnapshot · compute_significance · ReflectionResult)
- `EventKind::DialogueReflected` + `Command::EndDialogue.reflection`
- `ports/reflection.rs` (OCP) + `adapter/reflection_via_chat.rs` + `application/reflection_service.rs`
- `RelationshipPolicy.handle_dialogue_end` 게이트 + 4 follow-up
- `DialogueOrchestrator.with_reflection() + turn_buffers`
- chitchat 18% latency 절감 + calibration 3 밴드 정확 (0.000/0.461/0.980)

### Phase 1.5 (Mind Studio 통합) ✅ 완료 (2026-05-12)

Phase 1의 reflection 게이트는 *DialogueOrchestrator 경로*에 박혀 있었음. Mind Studio는
DialogueOrchestrator를 *사용하지 않고* `state.chat + domain_sync` ad-hoc 패턴이라
reflection 미동작 상태. Phase 1.5에서 같은 로직을 Mind Studio 경로에 *mirror*.

**산출물** (commits `5cf8fb5` + `9b35e99`):
- **Backend**: `AppState.reflection_service` + `with_reflection()` + `StateInner.turn_buffers` + `StudioService::run_reflection_for_session` + `domain_sync::dispatch_end_dialogue(reflection)` 시그니처 확장 + 4 통합 테스트 (Mock ReflectionRunner)
- **Frontend**: `ReflectionView.tsx` 신규 + ResultPanel '반추' 탭 + Zustand `lastAfterDialogue` + `handleEndChat` 박제 + toast band + `useStateSync.dialogue_reflected` 핸들러
- **회귀**: backend 64+478 / frontend 100 vitest

**trade-off** (CLAUDE.md 미반영, 본 문서에만 명시): DialogueOrchestrator와 Mind Studio가
*동일 reflection 로직을 두 군데*에 보유 → drift 위험. Mind Studio가 외부 노출되거나 drift
누적 시 cutover 검토.

### Phase 1.6 (EventBus → SSE Bridge) ✅ 완료 (2026-05-12)

Phase 1.5 manual SSE emit이 9개 도메인 사실에 각각 박혀 있음 + `/api/v2/scenes/*`
(Director 경로)가 manual emit 미경유로 SSE silent bug. EventBus 구독 bridge로 일원화.

**산출물** (commit `fb22400`):
- `src/bin/mind-studio/event_bridge.rs` 신설 (~250 LoC, `MemoryProjector` 패턴 mirror — subscribe_with_lag + Lagged replay)
- `map_event(&DomainEvent) -> Vec<StateEvent>` 결정론 매핑 9개 (`EmotionAppraised`/`StimulusApplied`/`GuideGenerated`/`SceneStarted`/`SceneEnded→AfterDialogue`/`DialogueTurnCompleted(speaker=assistant)→ChatTurnCompleted`/`DialogueReflected`/`MemoryEntry{Created,Superseded,Consolidated}`/`Rumor{Seeded,Spread}`)
- `main.rs`에서 `tokio::spawn(bridge.run(...))` 부팅 시 1회
- manual `state.emit()` 11곳 제거 (studio_service 5 + mcp_server 2 + handlers/scenario 2 + handlers/rumor 2). UI-only emit (HistoryChanged/SituationChanged/CRUD 등) *유지*.
- **보너스**: `/api/v2/scenes/*` Director 경로도 자동 SSE 발행 (단 `director_v2`는 *별도 dispatcher*라 본 bridge 범위 외 — shared_dispatcher 통합은 별도 작업)
- 7 단위 + 1 통합 신규

### Phase 2 (v0.8) — 4-axis + BondKind + type 도메인 마이그레이션 ✅ **완료 (2026-05-16)**

**범위 변경 (2026-05-13)**: 본래 roadmap의 Phase 2 (4-axis + BondKind + Channel 1)를 **얇은 phase 3개로 분할**:
- **Phase 2** ← 도메인 마이그레이션 only (본 phase) ✅ **완료**
- **Phase 2.3** ← appraise 정비 (신설, Stage 0 §3.6 시뮬레이션 검증 결과)
- **Phase 2.5** ← Channel 1 + axis_modulation (LLM 통합)

분할 근거: Phase 1 → 1.5 → 1.6 패턴 정합. 각 phase = 단일 책임. 회귀 위험 분산.

**Stage 분해 (실행 결과)**: Stage 0 (사실조사) → 1 (도메인) → 2 (OCC→4축 매핑) → 3 (wire+frontend) → 4 (시나리오 v0.7 영구 변환 + v0.6 code 0건화) → 5 (narrative 박제) → 6 (bench + 회고 + Phase 2.3 KICKOFF). 총 6 Stage.

**산출 요약**:
- 도메인: `domain/relationship/{mod,axis,bond,partnership}.rs` 신설 (4축 + BondKind 11/BondStatus 5/Partnership 4 + type 자유 텍스트). `power` 폐기 (B-D4). `AxisScore`/`WarinessScore` 2 타입 분리로 wariness 음수 컴파일 시점 차단 (B-D1).
- 매핑: `mapping.rs` 48셀 base_delta + HEXACO 6 보정룰 + `update_axes_from_emotion` 단일 진입점.
- Wire: payload 6→8 필드 + ÷100 layer 4겹 제거 (B-D-A ±100 raw) + frontend 4축 + 한글 라벨.
- 데이터: 4파일 v0.7 영구 변환 + 2 데이터셋 `_discarded-v0.6/`로 폐기 + v0.6 code 3 경로 제거.
- 검증: D3 3밴드 exact 보존 (0.000/0.461/0.980). D2 latency 임계값 50% 마진. S1~S3 정량 박제 + S4 정성.

**메트릭** (Phase 2 종결, `cargo test --lib --tests --bins`): 843 passed / 0 failed / 2 ignored / 65 묶음.

**디자이너 잔여 (Phase 2.3 인계)**: intensity 0.4 / S1~S3 narrative 타당성 / S4 정성 — Phase 2 종결 게이트 차단 아님.

**부채 (Phase 2.3 인계)**: ÷100 잔존 (telling_ingestion + modifiers + RelationshipLevel) + closeness/power src 12 파일/69 매치 재카탈로그 + state.rs:666~671 커스텀 Deserialize 잔존 (Stage 4 미처리) + B-D9 result.json 자동 dump 인프라 부재.

**산출물 spec**: [`task-rel-phase2-domain-migration.md`](task-rel-phase2-domain-migration.md) **v1.0 FROZEN**. 종합 보고: [`phase2-checkpoint-report.md`](phase2-checkpoint-report.md). 회고: `phase2-stage{1~6}-*.md` 6종.

### Phase 2.3 — appraise 정비 (★ 신설 2026-05-13)

**배경**: Phase 2 Stage 0 §3.6 시뮬레이션 검증 (S1~S4)에서 발견된 *appraise 입력 의존성* 문제. appraise는 디자이너 박은 Beat focus 완전성에 의존 — ActionFocus 누락 시 Admiration/Reproach 자동 생성 0. *상식적 추론* 자동화 안 됨.

**포함**:
- 시뮬레이션 시나리오 set 공식화 (`data/scenarios/appraise-validation/` 신설 — S1~S4 + 신규 ~15 케이스)
- 각 케이스 ground truth (기대 OCC list + intensity + 기대 4축 변화) 명시
- 누락 OCC 검증/경고 (I1): EventFocus desirability 강한데 ActionFocus 없음 → 경고 등 정적 룰. LLM 호출 0.
- Compound 감정 식별 확장 (현재 4개 → 추가 후보 검증: HappyFor+Admiration / Pity+Reproach 등 시뮬레이션 검증 후 채택)
- `RelationshipModifiers` 정밀화 (4축 환경에서 누락 modifier 검증, 예: `respect_modifier` 신설 여부)
- HEXACO 보정자 정량 미세조정 (v0.7 §4.3 6 보정 룰)
- base_delta 48셀 시나리오 기반 정량 미세조정

**의존**: Phase 2 종결 (4축 도메인 안정 후)

**위험**: 중. appraise 변경이 *base_delta 결과*도 흔들 수 있음 (Phase 2 통로 A 회귀). 무한 튜닝 위험 — 게이트 명확 정의 필요.

**검증 게이트**:
1. compile + 기존 테스트
2. 공식 시나리오 set 회귀 (S1~S4 + 신규 케이스 모두 ground truth와 ±N 이내)
3. Bench 회귀 측정

**산출물 spec**: [`task-rel-phase2.3-appraise-tuning.md`](task-rel-phase2.3-appraise-tuning.md) — 🟡 **DRAFT** 작성됨 (Phase 2 Stage 6 작업 6). 본체는 Phase 2.3 진입 시 작성. **KICKOFF (정본 인계)**: [`PHASE2.3-KICKOFF.md`](PHASE2.3-KICKOFF.md) v1.2 — Stage 4·5·6 인계 5항 (result.json dump / intensity 0.4 / S1~S3 narrative / S4 정성 / 메트릭 baseline) + 역대조 게이트 + closeness/power 12 파일/69 매치 재카탈로그 플래그.

### Phase 2.5 (v0.8.5) — Channel 1 Declarative + axis_modulation

**범위 변경 (2026-05-13)**: 본래 Phase 2의 Channel 1 부분이 본 phase로 *분리*. axis_modulation도 함께.

**포함**:
- Channel 1 Declarative 활성화: `declarative_events` / `partnership_event` placeholder (Phase 2에서 enum/필드 정의됨)에 LLM emit + 엔진 검증 + 적용 흐름 신설
- 사회적 일관성 검증 5 카테고리 (A~E)
- 4-tier 적용 모드
- **★ axis_modulation 3지선다**: Reflection LLM 출력 schema에 `axis_modulation` 필드 신설 (low/default/high → ±5/0/+5). 추가 LLM 호출 0 (기존 reflection 호출 활용). 엔진 산출 baseline + LLM 미세조정.
- 새 cause variant (B-D7): `DeclarativeBondFormation` / `PartnershipChange` / `BondStatusChange` 등 — Phase 2.5 시점에 명명 확정
- declarative_events 상한 N (B-D11)

**의존**: Phase 2 (도메인 enum/필드 정의 + RelationshipUpdater 안정), Phase 2.3 권장 (appraise 안정된 baseline 위에서 LLM modulation 적용)

**위험**: 큼. LLM 출력 schema 확장. 사회적 일관성 검증 디자인. 정략혼 등 *맥락 의존 reject* 룰 디자인.

**검증 게이트**:
1. compile + 기존 테스트 + 신규
2. Narrative cases:
   - 임충-노지심 야저림 의형제 결연 (Channel 1 Declarative + bond_kind: SwornBrothers)
   - 곽정-황용 결혼식 (Channel 1 Declarative + partnership: Spouse, 연애결혼)
   - 와호장룡 옥교룡 정략혼 도주 (Channel 1 emit → 사회적 일관성 검증 D reject — 양방향 동의 위반)
   - 산신묘 큰 도약 (Phase 2 점진 -49 trust + Phase 2.5 declarative_events 큰 도약 -35 = 시나리오 박힌 -30~-50 도달)
3. axis_modulation 결정론 검증 (같은 reflection prompt → 안정적 출력)

**산출물 spec**: `task-rel-phase2.5-channel1.md` (Phase 2.3 종결 후 작성).

### Phase 3a (v0.9) — Channel 2 Temporal

**포함**:
- BondKindCandidacy read model projection (application/projection 신설)
- 시간 게이트 카운터 (Guardian 7일 / Mentor 14일 / SwornBrothers/Companion 30일 / 원수 즉시)
- BondKindEntered/Exited 자동 emit
- 카운터 영속화 (SQLite read model)

**의존**: Phase 2 (BondKind enum 필요)

**위험**: 중. 새 application service + read model. 도메인 변경 없음.

**검증 게이트**:
1. compile + 기존 테스트 + 신규
2. Replay 결정성: 같은 입력 시퀀스 → 같은 BondKindCandidacy 상태
3. Narrative cases:
   - 이모백-수련 30년 누적 → Soulmate 자연 진입 (compress된 시간 시뮬레이션)
   - 곤란 시 동행 7일 → Companion 진입
   - 임계 떨어진 후 즉시 이탈 (지기/멘토)
   - 임계 위 회복 후 30일 → 원수 이탈 (비대칭)

**산출물 spec**: `task-rel-phase3a-temporal.md`.

### Phase 3b (v0.9) — Channel 3 External + narrative_origin

**포함**:
- EventPropagator application service 신설
- PropagationRule 첫 셋: 사망 / 처단 / 결혼 / 배신
- Awareness Tier 1 (즉시 인지) — 첫 구현
- `narrative_origin` EventMetadata 필드 신설 (parent_event_id의 cross-NPC 확장)
- Saga 패턴: NPC A의 source event → NPC B의 NpcLearnedAbout 명령 발행

**의존**: Phase 2 (BondKind/BondStatus 필요)

**위험**: 큼. 새 application service. cross-aggregate 흐름. 기존 tokio broadcast EventBus를 subscriber로 hook.

**검증 게이트**:
1. compile + 기존 테스트 + 신규
2. cross-NPC 이벤트 전파 테스트 (NPC A 사망 → NPC B 관계 갱신)
3. narrative_origin chain replay 검증
4. Narrative cases:
   - 무송 → 반금련 BloodEnemy (무대 사망 cross-reference + 무송 인지 시점)
   - 임충이 육겸 처단 → 육겸 가족의 임충 BloodEnemy 시드 (Rule 2 역방향)
   - 이모백 자연사 → 수련 BondStatus Deceased (Rule 1 직접, killer 없음)

**산출물 spec**: `task-rel-phase3b-external.md`.

### Phase 3c (v0.9) — ActionTriggerEvaluator + 추모 행동

**포함**:
- ActionTriggerEvaluator 도메인 모듈 신설 (`src/domain/action_trigger.rs`)
- 5-dim feasibility (physical/power/social/self/moral) — 비대칭 가중 `positive.powf(0.6) * qualifier.powf(0.4)`
- 29 ActionKind variants (action_triggers.md §5)
- BondKind → ActionKind 룰
- 추모 행동 emit (relationships.md §4.5.5 RecollectionAction 5종 통합)
- moral_alignment < 0.1이면 전체 0 (taboo 위반 차단)

**의존**: Phase 3a + 3b (BondKindEntered, BondStatusChanged, NpcLearnedAbout 모두 입력)

**위험**: 매우 큼. 새 도메인 모듈. 5-dim feasibility 룰 정밀 튜닝 필요.

**검증 게이트**:
1. compile + 기존 + 신규 테스트
2. 5-dim feasibility 각 dimension 단위 테스트
3. 비대칭 가중 (positive vs qualifier) 동작 검증
4. Narrative cases:
   - 임충 양산박 합류 (SystemicResistance) — power_balance + social_permission이 양산박 도착 시 점프
   - 임충 야저림 taboo (moral_alignment < 0.1) — SystemicResistance 차단 → 결과 도주
   - 수련 노년기 이모백 기일 → HandleHeirloom 추모 행동 emit
   - 무송 추적 → 반금련 즉결 처단 (BloodEnemy + 인지 + 능력 조합)

**산출물 spec**: `task-rel-phase3c-actiontrigger.md`.

## 6. Concept → Code 매핑

| Concept | Doc § | 현재 코드 | Phase 후 위치 |
|---|---|---|---|
| OCC appraise | rel §4.1 | `src/domain/emotion/` | (변경 없음) |
| PAD | rel §4.4 Inner | `src/domain/pad.rs` | (변경 없음) |
| Inner Loop | rel §4.4 + §6.1 | `application/dialogue_orchestrator.rs::turn` + `BeatTransitioned`-trigger `update_system_prompt` | (변경 없음) |
| Outer Loop entry | rel §4.4 + §6.1 | `DialogueOrchestrator::end_session` → `dispatch_v2(EndDialogue)` → `RelationshipPolicy` | Phase 1: ReflectionService 통과 후 dispatch (게이트 적용) |
| Reflection | rel §6.2 | — | Phase 1: `src/application/reflection_service.rs` 신설 |
| Engine significance | rel §6.3 | — | Phase 1: `src/domain/relationship.rs` 또는 신설 `domain/reflection.rs` (Stage 0 결정) |
| DialogueReflected | rel §6.2 | — | Phase 1: `src/domain/event.rs` EventKind 추가 |
| DTO (Reflection 응답) | rel §6.2 | `application/dto/` (7 도메인 분할) | Phase 1: `dto/scene.rs` 흡수 vs `dto/reflection.rs` 신설 (Stage 0 결정) |
| UoW 영향 | (본 문서 §2.3) | `application/command/uow.rs` (HandlerShared 출력 쉐이프) | Phase 1: `RelationshipPolicy` 변경 시 *UoW 변경 등록* 패턴 확인 (Stage 0) |
| 4 axes | rel §1 | ✅ **Phase 2 완료** — `src/domain/relationship/{mod,axis}.rs` (trust/affinity/respect/wariness ±100, AxisScore/WarinessScore 2 타입 분리) | (변경 없음) |
| BondKind | rel §3.1 | ✅ **Phase 2 완료** — `src/domain/relationship/bond.rs` (11 variants enum) | (변경 없음) |
| BondStatus | rel §3.5 | ✅ **Phase 2 완료** — `src/domain/relationship/bond.rs` (5 variants + `accepts_live_input()` 게이트) | (변경 없음) |
| Partnership | rel §3.6 | ✅ **Phase 2 완료** — `src/domain/relationship/partnership.rs` (4 variants enum) | (변경 없음) |
| type/type_history | rel §2 | ✅ **Phase 2 완료** — `src/domain/relationship/mod.rs::Relationship.type_text + type_history` 자유 텍스트 + 이력 (B-D4 `power` 흡수) | (변경 없음) |
| Channel 1 Declarative | rel §6.4 | — | Phase 2: `reflection_service.rs` 확장 + `command/policies/relationship_policy.rs` 진입 조건 변경 |
| 사회적 일관성 검증 (A~E) | rel §6.4 | — | Phase 2: `command/policies/relationship_policy.rs` 확장 |
| 적용 모드 (4-tier) | rel §6.4 | — | Phase 2: scenario JSON schema |
| Channel 2 (BondKindCandidacy) | rel §6.4 | — | Phase 3a: `src/application/projection/bond_kind_candidacy.rs` |
| Channel 3 (EventPropagator) | rel §6.4 | (Rumor/Information 인프라 일부) | Phase 3b: `src/application/event_propagator.rs` |
| narrative_origin | (본 문서 §2.6) | — | Phase 3b: `EventMetadata` 확장 |
| ActionTriggerEvaluator | action §5 | — | Phase 3c: `src/domain/action_trigger.rs` |
| 추모 행동 emit | rel §4.5.5 | — | Phase 3c: ActionTriggerEvaluator의 한 분기 |

## 6.5 디자인 문서 추적

본 표는 *디자인 문서*의 각 섹션이 *어느 phase에서 코드에 반영되는지* 추적. 디자이너 시점에서 *작성한 디자인이 언제 결실 맺는지* 한눈에 보기 위함. 디자인 문서 진화 또는 phase 완료 시 본 표 갱신.

### relationships.md 추적 (v0.7 기준)

| 섹션 | 정의 | 반영 phase | 현재 % | 완료 마커 |
|---|---|---|---|---|
| §0 명제 | LLM↔Engine 분업 6 명제 | 1 + 0-pillars Pillar 6 격상 | **100%** ✅ Phase 1 완료 | Phase 1 완료 |
| §1 4 axes | trust/affinity/respect/wariness, ±100 | 2 | **100%** ✅ | Phase 2 완료 (2026-05-16) |
| §2 type / type_history | 자유 텍스트 + 이력 | 2 | **100%** ✅ | Phase 2 완료 |
| §3.1 BondKind 11종 | 지기 4 + Companion + Guardian + Mentor + 원수 4 | 2 | **100%** ✅ (enum 신설, 인스턴스 명시는 디자이너 narrative 검토 후) | Phase 2 완료 |
| §3.5 BondStatus 5종 | Active/Resolved/Deceased/Dormant/Reactivating | 2 | **100%** ✅ (`accepts_live_input()` 게이트 활성) | Phase 2 완료 |
| §3.6 Partnership 4종 | Spouse/Engaged/Lover/Separated | 2 | **100%** ✅ (enum 신설) | Phase 2 완료 |
| §4.1~4.4 transformation rules | 변환 임계 + delta + Channel 1/2/3 | 2 (Ch1 partial: base_delta + HEXACO + BondStatus 차단 + clamp) + 3a (Ch2) + 3b (Ch3) | **Ch1 partial 60%** (Phase 2.5에서 declarative_events + axis_modulation 완성) | Phase 3b 완료 시 100% |
| §4.5.5 추모 행동 | RecollectionAction 5종 | 3c | 0% | Phase 3c |
| §5 LLM acting guide | ActingGuide 명세 | 부분 — PAD 기반 acting guide 코드 존재 | ~40% | Phase 2에서 풍부화 |
| **§6 Scene Boundary Reflection** | **LLM↔Engine 분업·Reflection·is_chitchat 게이트·DialogueReflected** | **1 + 1.5 + 1.6 ✅ 완료** | **100%** | Phase 1/1.5/1.6 완료 (2026-05-11~12) |
| §7 미정의 영역 | 후속 작업 카탈로그 | - | - | - |

### action_triggers.md 추적 (v0.1 기준)

| 섹션 | 정의 | 반영 phase | 현재 % |
|---|---|---|---|
| §1 흐름도 + RelationshipUpdater 입력 | 트리거 흐름 정합 (v0.7 노트만 추가됨) | 3c | 0% |
| §2 5-dim feasibility | physical / power / social / self / moral | 3c | 0% |
| §3 비대칭 가중 | `positive^0.6 × qualifier^0.4` | 3c | 0% |
| §4 moral_alignment 차단 | < 0.1이면 전체 0 (taboo) | 3c | 0% |
| §5 29 ActionKind | SystemicResistance · RevengeQuest · HandleHeirloom 등 | 3c | 0% |

→ **action_triggers.md 전체가 Phase 3c**. 그 전까지 코드 0%. Phase 3a/3b 출력 (BondKindEntered/NpcLearnedAbout 등) 모두 입력으로 받음.

### _schema.md 추적 (시나리오 JSON schema, *현재 SOR*)

✅ **_schema.md ↔ 코드 schema 동기화 spot-check 완료 (2026-05-10, Phase 1 Stage 0 Findings F8.6)** — 본 표 7행 모두 ❓ → 정확한 팩트로 치환.

| 필드/섹션 | _schema.md v0.6 정의 | 코드 구현 상태 | 갭 분류 | 동기화 phase |
|---|---|---|---|---|
| `Npc.id` / `name` / `description` / `personality` (HEXACO 24) | ✅ Layer 1 정의 (identity + temperament) | ✅ NpcJson + 24 facet 완전 ([memory_repository.rs:72-126](../../../src/adapter/memory_repository.rs)) | 없음 | 완료 |
| `Npc.inner_compass` (compass/taboo/life_question/taboo_crystallization 4-필드 nested) | ✅ Layer 2 복합 객체 | ❌ 부재 → Phase 1 A-min `Option<String>` (compass만, [personality.rs:335](../../../src/domain/personality.rs:335) 신설 예정) | 중 — Phase 1 partial. Forward-compat OK (null start → 후속 `Option<InnerCompass>` 승격 시 serde 호환) | **1 ★ (A-min)** → 3c (taboo/life_question 승격) |
| `Relationship` 4축 (trust/affinity/respect/wariness, ±100) | ✅ v0.6 명시 | ⚠️ 3축 (closeness/trust/power, ±1.0) — closeness ≠ affinity 의미론 다름 | 큼 — 의미·범위 모두 재작성 | 2 |
| `BondKind` 11 variants (지기 6 + Mentor + 원수 4) | ✅ v0.6 정의 (Companion·Guardian 포함) | ❌ 부재 | 큼 — enum 통째 신설 | 2 |
| `BondStatus` 5 / `Partnership` 4 / `type` / `type_history` | ✅ 정의 (Active/Resolved/Deceased/Dormant/Reactivating · Spouse/Engaged/Lover/Separated · 자유 텍스트 + 누적 배열) | ❌ 부재 | 큼 — enum + 자유 텍스트 + history 배열 신설 | 2 |
| 행동 관련 필드 (ActionKind / 5-dim feasibility) | ⚠️ _schema.md 범위 외 (분리 — `action_triggers.md` v0.1 이관, v0.6 신설 원칙 §0 "분류와 행동은 분리") | ❌ 부재 (`domain/action_trigger.rs` 미존재) | 큼 — 별도 spec 본위 | 3c |
| `Scene` / `SceneFocus` / `FocusTrigger` | ◯ _schema.md 범위 외 (인물 schema만 다룸) | ✅ 완전 구현 ([scene.rs](../../../src/domain/emotion/scene.rs) + SceneJson [memory_repository.rs:217-227](../../../src/adapter/memory_repository.rs:217)) | 없음 | 완료 |

### 종합 — 디자인-코드 정합 진척

```
relationships.md     [██▒▒▒▒▒▒▒▒]   ~15%  (PAD acting guide 부분 + §0 명제 100% + §6 Reflection 100%)
                       ↑
                       Phase 1/1.5/1.6 ✅ 완료 (2026-05-11~12)
                       다음 도약: Phase 2 — §1 4축 + §3 BondKind/BondStatus/Partnership + §6.4 Channel 1
                                (+50%로 도약 예정, 누적 ~65%)

action_triggers.md   [▒▒▒▒▒▒▒▒▒▒]    0%
                       ↑
                       Phase 3c까지 코드 변경 0

_schema.md           [███▒▒▒▒▒▒▒]   ~30% verified (Phase 1 F8.6 spot-check 후 갱신)
                       ↑                Layer 1 (HEXACO 24+identity) ✅ + Scene/Focus ✅ 완료
                       Phase 1 inner_compass partial (+5%, A-min compass만) ✅
                       Phase 2 4축+BondKind+BondStatus+Partnership+type/type_history (+50%)
                       Phase 3c inner_compass full (taboo/life_question 승격) + 행동 (별도 spec)
```

### 디자인 문서 진화 정책

**Phase 시작 시 입력 디자인 문서 freeze**:
- 각 phase task spec §1.1에 *입력 디자인 문서 + 버전 (또는 commit hash)* 명시
- 진행 중 phase는 *그 시점의 freeze된 버전*으로 작업
- 디자인 문서 진화 (v0.7 → v0.8 등)는 *진행 중 phase에 영향 0* — 다음 phase 시작 시 최신 버전으로 freeze 갱신

**Phase 완료 시 본 §6.5 표 갱신**:
- 해당 섹션 % → 100% 또는 `완료 마커` 갱신
- *디자인 문서 자체*가 진화했다면 해당 행도 *최신* 정의로 갱신

**디자인 문서 추가 작성 시점**:
- 새 디자인 영역 (예: `companions.md`, `reputation.md`)을 *작성*하면 *동시에* 본 §6.5에 추적 행 추가
- 어느 phase에서 코드 반영할지 *작성 시점에* 결정 (또는 *미정*으로 명시)

## 7. 유지 정책

이 문서를 갱신하는 시점:
1. 새 Phase 시작 또는 종결
2. 주요 design doc 개정 (예: relationships.md v0.7 → v0.8)
3. 코드의 큰 마이그레이션 완료 (Phase X "✅ 완료" 표시)
4. Gap analysis 표의 *현재* 컬럼이 코드 변화로 정확하지 않게 됐을 때
5. §2의 verification level이 변화했을 때 (◯/△ → ✅ 승격)
6. §6.5 디자인 문서 추적 — Phase 완료 또는 디자인 문서 진화 시 % 또는 완료 마커 갱신

이 문서가 *대체하지 않는* 것:
- design docs (game-design/) — *무엇*을 정의
- architecture docs — *왜*를 정의
- 본 문서는 *어디·언제·얼마나*의 매핑

목표 아키텍처에 대해 모호할 때는 **design docs가 우선**.

각 Phase의 *상세* spec은 별도 task 문서로 작성:
- `task-rel-phase1-reflection.md`
- `task-rel-phase2-fouraxis-bondkind.md`
- `task-rel-phase3a-temporal.md`
- `task-rel-phase3b-external.md`
- `task-rel-phase3c-actiontrigger.md`

이 문서는 *상위 phasing*만, task 문서가 *구현 명세 + 검증 단계 + grep 게이트*.

## 변경 이력

| 버전 | 일자 | 변경 |
|---|---|---|
| v0.1 | 2026-05-09 | 초안. relationships.md v0.7과 동반 신설. Phase 1/2/3a/3b/3c 정의 + Gap analysis + Concept-Code 매핑 + verification level 표기. |
| v0.2 | 2026-05-10 | CLAUDE.md 갱신 반영: **MindService 폐기 정정** (v0.3.0 제거 — Director/CommandDispatcher/DialogueOrchestrator 단일화, §1·§2.3·§2.4 정정). **ports/ ISP 분할** (§2.2.5 신설, 7 모듈). **`agents/` → `policies/`** 리네임 반영 (§2.3, §6 매핑). **UnitOfWork 도입** 반영 (§2.3, §6 신설 행). **dto/ 7 도메인 분할** 반영 (§2.3). **Phase 1 §5 코드 경로 정확화**. |
| v0.3 | 2026-05-10 | **§6.5 디자인 문서 추적 신설** — relationships.md / action_triggers.md / _schema.md 각 섹션이 어느 phase에서 코드 반영되는지 명시적 매핑. 종합 진척 그래프. 디자인 문서 진화 정책 (Phase 시작 시 freeze, 완료 시 표 갱신, 새 문서 추가 시 동시 추적 행 추가). §7에 갱신 규칙 6번 항목 추가. _schema.md ↔ 코드 schema 동기화 spot-check 필요 (추정 행 ❓ 표기). 디자이너(Bekay) 시점에서 *디자인이 언제 결실 맺는지* 추적 가능. |
| v0.4 | 2026-05-10 | **§6.5 _schema.md 추적 표 ❓ 7행 → 정확한 팩트로 치환** (Phase 1 Stage 0 Findings F8.6 결과). _schema.md v0.6 vs 코드: Layer 1 (HEXACO+identity) ✅ 완료 / inner_compass ❌ → Phase 1 A-min `Option<String>` partial / 4축·BondKind·BondStatus·Partnership·type 모두 큼 갭 → Phase 2 / 행동 별도 spec → Phase 3c / Scene/Focus _schema.md 범위 외 + 코드 완료. 진척 그래프 보정 — _schema.md ~60% 추정 → ~30% verified (이전 추정 과대평가). |
| v0.5 | 2026-05-11 | Phase 1.5 / 1.6 완료 반영. §2 Mind Studio 통합 표 신설. §5 Phase 1/1.5/1.6 ✅ 표기. §6.5 §0+§6 100% 갱신. EventKind 31개 / domain/reflection.rs / ports/reflection.rs / reflection_service.rs / adapter/reflection_via_chat.rs / event_bridge.rs 추가. |
| v0.6 | 2026-05-13 | **Phase 2 범위 변경 — 얇은 phase 3개로 분할**: Phase 2 (도메인 마이그레이션 only) / Phase 2.3 (appraise 정비 ★ 신설) / Phase 2.5 (Channel 1 + axis_modulation). Phase 2 Stage 0 §3.6 시뮬레이션 검증 (S1~S4)의 *appraise 입력 의존성* 발견이 Phase 2.3 신설 근거. Phase 2 본문 갱신 (power 폐기 / type 흡수 / OCC → 4축 자동 갱신 T1 시점 / B-D6/D12/D13/D14 결정 박힘). Phase 2.5 본문 갱신 (axis_modulation 3지선다). 산출물 spec 파일명 변경 `task-rel-phase2-fouraxis-bondkind.md` → `task-rel-phase2-domain-migration.md`. |
