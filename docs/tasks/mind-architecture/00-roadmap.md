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

## 2. 현재 상태 (2026-05-09 spot-check 기준)

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
| `event.rs` | DomainEvent + EventMetadata + 30 EventKind + RelationshipChangeCause 5 variants | ✅ 1~300줄 |
| `emotion/` | AppraisalEngine 모듈화 (Event/Action/Object/Compound 서브) | ◯ |
| `pad.rs`, `pad_anchors.rs`, `pad_table.rs` | StimulusEngine 기반 | △ |
| `personality.rs` | HEXACO 24 facet, Score 타입 | △ |
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

### 2.3 Application Layer (`src/application/`)

> **v0.3.0 정정 노트**: v1 경로 (`MindService` / `EventAwareMindService` / `Pipeline` / `CommandDispatcher::dispatch` / `shadow_v2`)는 모두 제거됨. 진입점은 `Director` / `CommandDispatcher::dispatch_v2` / `DialogueOrchestrator` (chat) 셋. v0.1 초안의 architecture-v2.md 인용은 stale이었음 — 이번 갱신에서 정정.

| 모듈 | 상태 | 검증 |
|---|---|---|
| `command/dispatcher.rs` (`CommandDispatcher<R>`) | dispatch_v2 단일 진입점. `with_default_handlers` / `with_memory` / `with_memory_full` / `with_world_overlay` / `with_scene_consolidation` / `with_rumor` 빌더 | ◯ |
| `command/uow.rs` (`UnitOfWork`) | Transactional BFS 변경 누적 → 일괄 commit. `HandlerShared`는 *출력 호환용 쉐이프*로 변경 | ◯ |
| `command/policies/` (8 핸들러) | `emotion`/`stimulus`/`guide`/`relationship`/`scene`/`information`/`rumor`/`world_overlay` (이전 `agents/` → `policies/` 리네임) | ◯ |
| `command/{telling_ingestion,rumor_distribution,world_overlay,scene_consolidation,relationship_memory}_handler.rs` | Inline 핸들러 5종 — Step C/D 메모리 흡수 | ◯ |
| `dialogue_orchestrator.rs` (`DialogueOrchestrator<R, C>`) [chat] | LLM 다턴 오케스트레이터. `start_session`/`turn`/`end_session`. `BeatTransitioned` 발생 시 `update_system_prompt` | ◯ |
| `director/` (`Director<R>`) | 다중 Scene facade. `start_scene`/`dispatch_to`/`end_scene`/`active_scenes` | ◯ |
| `event_bus.rs` | tokio broadcast, futures::Stream 노출, lag 처리 | ✅ 1~80줄 |
| `event_store.rs` | 이벤트 영속화 (commit staging buffer) | △ |
| `memory_projector.rs` | EventBus 구독 기억 인덱싱 [embed] | ◯ |
| `dto/` (7 도메인 모듈) | `emotion`/`guide`/`information`/`relationship`/`rumor`/`scene`/`world` 분할 | ◯ |
| `error.rs` (`MindServiceError`) | 5 variants: NpcNotFound · RelationshipNotFound · InvalidSituation · EmotionStateNotFound · LocaleError | ◯ |

### 2.4 Inner/Outer 골격 — 부분 구현됨 (★)

CLAUDE.md 기준 (v0.3.0 후), Inner/Outer 두 흐름은 다음 컴포넌트들로 *부분 구현*되어 있음:

**Inner Loop (대화 turn)** — `DialogueOrchestrator.turn` (chat feature):
- `Command::ApplyStimulus.dispatch_v2().await` → 도메인 동기 처리
- 처리 결과 events에 `BeatTransitioned` 포함 시 → `ConversationPort::update_system_prompt` 호출
- `ConversationPort::send_message` → 다음 turn

**Outer Loop 진입점** — `DialogueOrchestrator.end_session`:
- (significance 있으면) `Command::EndDialogue.dispatch_v2().await`
- 도메인 안에서 `RelationshipPolicy`가 `DialogueEndRequested` 수신 → 3 follow-ups (`RelationshipUpdated` + `EmotionCleared` + `SceneEnded`) 일괄 발행
- 현재: 무조건 작동. 잡담도 axes 변화.
- 목표 (Phase 1): Reflection 단계 추가 → is_chitchat / significance 게이트 통과 시만 RelationshipUpdated 발행 (EmotionCleared/SceneEnded는 항상)

→ Phase 1의 핵심 작업이 정확히 이 *gate 추가*. 아키텍처 자체(Director/CommandDispatcher/DialogueOrchestrator)는 그대로 유지.

### 2.5 EventKind 인벤토리 (✅ 직접 verified)

```
Mind:           AppraiseRequested, EmotionAppraised,
                StimulusApplyRequested, StimulusApplied,
                BeatTransitioned, EmotionCleared
Scene:          SceneStartRequested, SceneStarted, SceneEnded
Relationship:   RelationshipUpdateRequested, RelationshipUpdated
Dialogue:       DialogueEndRequested, DialogueTurnCompleted
Guide:          GuideRequested, GuideGenerated
Memory:         MemoryEntryCreated, MemoryEntrySuperseded,
                MemoryEntryConsolidated
Rumor:          SeedRumorRequested, SpreadRumorRequested,
                RumorSeeded, RumorSpread, RumorDistorted, RumorFaded
World:          ApplyWorldEventRequested, WorldEventOccurred,
                TellInformationRequested, InformationTold
```

`DialogueReflected` 등 Phase 1 신규 EventKind는 미정의.

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

### Phase 1 (v0.7) — Reflection + Significance + Chitchat Gate

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

### Phase 2 (v0.8) — 4-axis + BondKind + Channel 1

**포함**:
- 도메인: `Relationship` 재작성 (4축 ±100), BondKind/BondStatus/Partnership/type/type_history enum 및 함수
- Application: Channel 1 처리 (declarative_events emit, 사회적 일관성 검증 5 카테고리 A~E, 적용 모드 4-tier)
- 마이그레이션: 기존 3축 → 4축 변환 룰 (예: closeness → affinity, trust 그대로 + 음수 의미 추가)
- 시나리오 JSON schema 갱신 (`_schema.md` v0.7)
- 검증 인물 시나리오 갱신 (임충/수련/연청/노년기 수련 등)

**위험**: 큼. 도메인 모델 재작성. 기존 OCC → axes 매핑 함수 모두 재작성. 테스트 대량 갱신.

**검증 게이트**:
1. compile + 기존 테스트 (마이그레이션 변환 후) + 신규 unit + 일관성 테스트
2. Bench 회귀 측정
3. Narrative cases:
   - 임충-노지심 야저림 의형제 결연 (Channel 1 Declarative + bond_kind: SwornBrothers)
   - 곽정-황용 결혼식 (Channel 1 Declarative + partnership: Spouse, 연애결혼)
   - 와호장룡 옥교룡 정략혼 도주 (Channel 1 emit → 사회적 일관성 검증 D reject — 양방향 동의 위반)

**산출물 spec**: `task-rel-phase2-fouraxis-bondkind.md`.

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
| 4 axes | rel §1 | 3축 (closeness/trust/power) | Phase 2: `src/domain/relationship.rs` 재작성 |
| BondKind | rel §3.1 | — | Phase 2: 새 enum |
| BondStatus | rel §3.5 | — | Phase 2 |
| Partnership | rel §3.6 | — | Phase 2 |
| type/type_history | rel §2 | — | Phase 2 |
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
| §0 명제 | LLM↔Engine 분업 6 명제 | 1 + 0-pillars Pillar 6 격상 | 50% (00-pillars.md v0.2 반영) | Phase 1 완료 시 100% |
| §1 4 axes | trust/affinity/respect/wariness, ±100 | 2 | 0% | Phase 2 |
| §2 type / type_history | 자유 텍스트 + 이력 | 2 | 0% | Phase 2 |
| §3.1 BondKind 11종 | 지기 4 + Companion + Guardian + Mentor + 원수 4 | 2 | 0% | Phase 2 |
| §3.5 BondStatus 5종 | Active/Resolved/Deceased/Dormant/Reactivating | 2 | 0% | Phase 2 |
| §3.6 Partnership 4종 | Spouse/Engaged/Lover/Separated | 2 | 0% | Phase 2 |
| §4.1~4.4 transformation rules | 변환 임계 + delta + Channel 1/2/3 | 2 (Ch1) + 3a (Ch2) + 3b (Ch3) | 0% | Phase 3b 완료 시 |
| §4.5.5 추모 행동 | RecollectionAction 5종 | 3c | 0% | Phase 3c |
| §5 LLM acting guide | ActingGuide 명세 | 부분 — PAD 기반 acting guide 코드 존재 | ~40% | Phase 2에서 풍부화 |
| **§6 Scene Boundary Reflection** | **LLM↔Engine 분업·Reflection·is_chitchat 게이트·DialogueReflected** | **1 ★ 진행 중** | **0% → Phase 1 완료 시 100%** | Phase 1 완료 |
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

⚠️ **_schema.md ↔ 코드 schema 동기화 spot-check 미완료** — 본 추적 표는 *부분 추정*. Phase 1 Stage 0 Findings F8에 "_schema.md 본문 직접 read + 코드 schema와 갭 catch" 항목 추가 권장.

| 필드/섹션 | _schema.md 정의 | 코드 schema | 갭 | 동기화 phase |
|---|---|---|---|---|
| `Npc.id` / `name` / `description` / `personality` | ❓ 추정 — 정의됨 | ✅ 4 필드 (Phase 1 F4 verified) | 없음 (추정) | 동기화 완료 |
| `Npc.inner_compass` | ❓ 미확인 | ❌ 부재 | A-min Phase 1 추가 | **1 ★** |
| `Relationship` 4 axes | ❓ 미확인 | 3축 (closeness/trust/power) | ❓ | 2 |
| `BondKind` 필드 | ❓ 미확인 | ❌ 부재 | ❓ | 2 |
| `BondStatus` / `Partnership` / `type` / `type_history` | ❓ 미확인 | ❌ 부재 | ❓ | 2 |
| 행동 관련 필드 (선택) | ❓ 미확인 | ❌ 부재 | ❓ | 3c |
| Scene / Beat / Focus | ❓ 미확인 | ✅ 코드 존재 | ❓ | 동기화 완료 (추정) |

→ **`_schema.md` *어느 버전*인지·코드와 얼마나 어긋나 있는지 spot-check 필요**. Phase 1 진행 중 자연 발견 또는 별도 audit task.

### 종합 — 디자인-코드 정합 진척

```
relationships.md     [█▒▒▒▒▒▒▒▒▒]   ~5%   (PAD 기반 acting guide만 부분)
                       ↑
                       Phase 1 진행 중 — §6 Reflection 100% 예정 (~15%로 도약)

action_triggers.md   [▒▒▒▒▒▒▒▒▒▒]    0%
                       ↑
                       Phase 3c까지 코드 변경 0

_schema.md           [██████▒▒▒▒]   ~60% 추정 (verified: id/name/description/personality)
                       ↑
                       Phase 1 inner_compass +1 필드, Phase 2에 4축+BondKind 등 큰 갱신
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
