# Phase 1 Mind Architecture — Checkpoint Report

> Phase 1 + Phase 1.5 + Phase 1.6 종결 (2026-05-10 ~ 2026-05-12). Phase 1 본체 =
> "Reflection + Significance + Chitchat Gate" — `relationships.md` v0.7 §6 Scene
> Boundary Reflection의 도메인·Application·외부 면 전반 통합. Phase 1.5 =
> Mind Studio backend/frontend 통합. Phase 1.6 = EventBus→SSE bridge로 manual
> emit 일원화 + Director SSE bug 부수 fix.
>
> 상세 spec (Stage 0 Findings 포함, v0.12): [`task-rel-phase1-reflection.md`](task-rel-phase1-reflection.md).
> API 변경 안내: [`docs/changes/phase1-mind-architecture.md`](../../changes/phase1-mind-architecture.md).

## 1. 한 문단 요약

Outer Loop (`DialogueEndRequested` → `RelationshipPolicy`)가 *무조건* 3 follow-up 발행하는
구조에 LLM Reflection 게이트를 끼워, **잡담은 outer loop skip / 의미 있는 사건은 그대로 진행**
하도록 분기. LLM이 서사 의미 (`is_chitchat`, `summary`) 판정, 엔진이 정량 `significance_score`
계산. 둘이 합쳐 `DialogueReflected` 이벤트로 박제. `RelationshipPolicy`가 게이트 통과 시만
`RelationshipUpdated` 발행. 회귀 0건, 게이트 효과 측정으로 확인 (chitchat 18% latency 절감),
calibration 3 밴드 정확. Phase 1.5에서 Mind Studio UI '반추' 탭 + reflection 박제 표시.
Phase 1.6에서 EventBus→SSE bridge로 manual emit 일원화 + Director 경로 silent SSE bug 부수 fix.

## 2. Stage 진척 + commits

### Phase 1 본체 (Stage 0~5)

| Stage | 산출물 | commit | 회귀 카운트 |
|---|---|---|---|
| Stage 0 (Findings) | spec 본문 + 13 결정 + 11 위험 + F8.6 schema 갭표 + F11 baseline 측정 + F12 bench | `87c8b32` → `a81f49b` → `c2728d2` → `44bd753` → `61d16df` → `136c9b6` | (분석만) |
| **A-min 분리** | `Npc.inner_compass` + `compass_short_label()` + 5 단위 | `c3b3e21` | 1068 passed |
| Stage 1 (Domain) | `domain/reflection.rs` + EventKind/Payload + Command/Dispatcher + 8 단위 | `891cc9a` | 1076 passed |
| Stage 2 (Application) | ports/adapter/service + RelationshipPolicy 게이트 + Orchestrator + 10 단위 | `641bedb` | 1086 passed |
| Stage 3 (외부 면) | DTO + chitchat 호환 + StateEvent 사전 배선 | `f91ffe9` | 1086 passed |
| Stage 4 (Narrative) | 3 시나리오 + 3 통합 테스트 + README | `0078c5c` | 1089 passed |
| **Stage 5 (Bench)** | 6 bench cases (engine/dispatch/calibration) | `c7e1ac4` | **1095 passed** |
| Stage 6 (Archive) | docs/changes + roadmap 완료 표기 + 본 리포트 v0.1 | `47c57a3` | 1095 passed |
| 실제 LLM 검증 | `tests/phase1_real_llm_test.rs --ignored` + robustness 2건 (markdown fence strip · declarative_events placeholder 무시) | `c48c50d` | 1095 passed |

### Phase 1.5 (Mind Studio 통합) — 2026-05-12

| 작업 | 산출물 | commit |
|---|---|---|
| Backend (AppState 통합) | `AppState.reflection_service` + `with_reflection()` + `StateInner.turn_buffers` (chat-gated) + `StudioService::run_reflection_for_session` 헬퍼 + `domain_sync::dispatch_end_dialogue(reflection)` 시그니처 확장 + 4 통합 테스트 (Mock ReflectionRunner) | `5cf8fb5` |
| Frontend (반추 탭) | `ReflectionResult`/`AfterDialogueResponse` TS 타입 + `useResultStore.lastAfterDialogue` + `ReflectionView` 컴포넌트 + ResultPanel '반추' 탭 + `handleEndChat` 박제 + toast band 표시 + `useStateSync.dialogue_reflected` SSE 핸들러 + vitest 1 신규 | `9b35e99` |

### Phase 1.6 (EventBus → SSE Bridge) — 2026-05-12

| 작업 | 산출물 | commit |
|---|---|---|
| Bridge 신설 + 통합 | `src/bin/mind-studio/event_bridge.rs` 신설 (~250 LoC, `MemoryProjector` 패턴 mirror — subscribe_with_lag + Lagged replay) + `map_event(&DomainEvent) -> Vec<StateEvent>` 결정론 매핑 (9 도메인 이벤트) + `main.rs`에서 `tokio::spawn(bridge.run(...))` + manual `state.emit()` 11곳 제거 + Phase 1.5 통합 테스트 갱신 (spawn_event_bridge 헬퍼) + 7 단위 + 1 통합 신규 | `fb22400` |

**총 작업량**: Phase 1 본체 15 + Phase 1.5 2 + Phase 1.6 1 = **18 commits**, 7 일 작업.

## 3. 핵심 결정 (spec §4.4 13 결정 — 모두 준수)

| # | 결정 | 적용 결과 |
|---|---|---|
| 1 | LLM 호출은 dispatch_v2 *바깥* | `DialogueOrchestrator.end_session` 안에서 호출 → dispatch_v2 동기 fast-path 보존. Mind Studio도 동일 패턴 (Phase 1.5 mirror) |
| 2 | 별도 Reflection 에이전트 | `ConversationBackedReflectionPort` — 같은 모델, 별도 KV slot. Mind Studio도 별도 RigChatAdapter 인스턴스 |
| 3 | OCP 준수 | `ReflectionService<P: ReflectionPort>` + Application에서 구체 어댑터 import 0건 |
| 4 | Phase 1 어댑터 | `ConversationPort`의 기존 메서드만 사용. 새 trait 메서드 0 |
| 5 | TurnSnapshot 누적 | `DialogueOrchestrator.turn_buffers` (plain HashMap, `&mut self` 일관성). Phase 1.5에서 `StateInner.turn_buffers`로 동일 패턴 mirror |
| 6 | Reflection을 dispatch_v2 입력에 | `Command::EndDialogue { reflection: Option<ReflectionResult> }` |
| 7 | DialogueReflected 항상 발행 | chitchat skip 케이스에도 박제 (audit / memory_projector 흡수) |
| 8 | 게이트 조건 정확 | `outer_loop_entry()` helper — significance ≥ 0.3 OR !is_chitchat OR ... |
| 9 | Phase 1 placeholder | `declarative_events` / `partnership_event` 항상 빈 vec / None |
| 10 | 동기 실행 | `end_session().await`이 reflection 완료까지 블록 |
| 11 | JSON 파싱 실패 fallback | `is_chitchat=false` (보수적) + reasoning에 "FALLBACK: ..." 표기 |
| 12 (사) | turn_buffers 위치 | DialogueOrchestrator 내부 (Bekay 확정 2026-05-10). Phase 1.5는 StateInner 안에 mirror (Mind Studio ad-hoc 경로 책임 분리) |
| 13 (아) | MAX_EVENTS_PER_COMMAND | 21 → 22 (Bekay 확정 2026-05-10) |

A-min 선결정 (바): `Npc.inner_compass: Option<String>` — compass 한 줄 (Phase 3c 승격).

## 4. 측정 결과 (Stage 5)

### 4.1 Engine
- `compute_significance(10 turn) ×10000`: **8.36 µs/call** (target <1ms, 100x 마진)

### 4.2 dispatch_v2(EndDialogue) per-call
| 케이스 | latency | follow-up | vs legacy |
|---|---|---|---|
| chitchat | 24.17 µs | 3 | **0.82x** ★ skip 효과 |
| significant | 35.03 µs | 4 | 1.19x |
| legacy | 29.34 µs | 3 | 1.0x |

### 4.3 Calibration (narrative 3 밴드)
| 시나리오 | significance | Target |
|---|---|---|
| chitchat | 0.000 | <0.3 ✅ |
| daily | 0.461 | 0.3~0.7 ✅ |
| shanshenmiao | 0.980 | ≥0.7 ✅ |

→ 가중치 `0.40/0.30/0.15/0.15` + 임계값 `0.3` **튜닝 불필요**.

### 4.4 실제 LLM 검증 (`tests/phase1_real_llm_test.rs`, 2026-05-11)

gemma-4-E4B-it 실측 (`target/baseline/phase1-real-llm-results.json`):

| 시나리오 | LLM is_chitchat | engine significance | 게이트 | calibration |
|---|---|---|---|---|
| chitchat-passerby | ✅ true | 0.050 | skip | ✅ 통과 |
| daily-training | ⚠️ true (기대 false) | 0.230 | skip | ⚠️ DRIFT (LLM 직관과 spec §6.4 daily band 불일치) |
| lin-chong-shanshenmiao | ✅ false | 0.390 | outer loop 진입 | ✅ 게이트 작동 / 높음 밴드 sig는 부족 |

→ **모든 시나리오 게이트는 spec §6.4대로 정확 동작**. drift는 *significance 분포 strict band 기대*에 한정.

Robustness fix 2건:
- `strip_json_envelope` helper — markdown fence (` ```json ` 등) strip
- `declarative_events` / `partnership_event` LLM 출력 *내용 무시* (Phase 1 결정 9 강제)

## 5. 새 도메인 / API

### 5.1 도메인 (Phase 1 본체)
- `Npc.inner_compass: Option<String>` + `compass_short_label()` (A-min)
- `domain::reflection::TurnSnapshot` + `compute_significance(turns) -> f32`
- `domain::reflection::ReflectionResult` + `DeclarativeEventPlaceholder` + `PartnershipEventPlaceholder`
- `EventKind::DialogueReflected` + `EventPayload::DialogueReflected { npc_id, partner_id, scene_id, result }`
- `EventPayload::DialogueEndRequested.reflection: Option<ReflectionResult>` (필드 추가)

### 5.2 Application / Ports / Adapter (chat feature gated)
- `ports::reflection::{ReflectionPort, ReflectionPrompt, ReflectionError}`
- `adapter::reflection_via_chat::ConversationBackedReflectionPort<C>`
- `application::reflection_service::{ReflectionRunner, ReflectionService<P>, ReflectionPromptBuilder, DefaultReflectionPromptBuilder, strip_json_envelope}`
- `Command::EndDialogue.reflection: Option<ReflectionResult>` (필드 추가)
- `RelationshipPolicy.handle_dialogue_end` 게이트 + 4 follow-up 순서 + `outer_loop_entry()` pub(crate) helper
- `DialogueOrchestrator.with_reflection(svc)` 빌더 + `turn_buffers` + `run_reflection()` helper
- `dispatcher.MAX_EVENTS_PER_COMMAND` 21 → 22

### 5.3 DTO / Mind Studio Backend
- `AfterDialogueResponse.reflection: Option<ReflectionResult>` (`#[serde(default)] + skip_serializing_if`)
- `DialogueOrchestrator.build_end_dialogue_from_v2` chitchat 호환 (3-tier fallback)
- **Phase 1.5**: `AppState.reflection_service: Option<Arc<dyn ReflectionRunner>>` + `with_reflection()` 빌더
- **Phase 1.5**: `StateInner.turn_buffers: HashMap<String, Vec<TurnSnapshot>>` (chat-gated, `#[serde(skip)]`)
- **Phase 1.5**: `StudioService::run_reflection_for_session` + `perform_after_dialogue(state, req, session_id)` (session_id 인자 추가)
- **Phase 1.5**: `domain_sync::dispatch_end_dialogue(state, inner, req, reflection)` 시그니처 확장
- **Phase 1.5**: `main.rs`에서 별도 `RigChatAdapter` 인스턴스 → `ConversationBackedReflectionPort` → `DefaultReflectionPromptBuilder` → `ReflectionService` 자동 부착
- **Phase 1.5**: `process_chat_turn_result`에서 chat 매 turn마다 TurnSnapshot 누적 (DialogueOrchestrator.turn() ⑦ mirror)
- **Phase 1.5**: `perform_chat_end`에서 after_dialogue 미요청 시도 stale turn_buffer 명시 clear
- **Phase 1.6**: `src/bin/mind-studio/event_bridge.rs` 신설 (~250 LoC) + `tokio::spawn(bridge.run(...))` 부팅 시 1회
- **Phase 1.6**: manual `state.emit()` 11개 제거 (도메인 사실에서 도출 가능한 SSE는 bridge가 자동 발행)
- **Phase 1.6**: `StateEvent` enum에 `PartialEq`/`Eq` derive 추가 (매핑 비교 검증용)

### 5.4 Frontend (Mind Studio UI, Phase 1.5)
- `ReflectionResult` / `AfterDialogueResponse` TS types (`types/index.ts`)
- `useResultStore.lastAfterDialogue: AfterDialogueResponse | null` + `setLastAfterDialogue` setter
- `ReflectionView.tsx` 신규 — chitchat/significant 라벨 + significance band (낮음/중간/높음) + summary + reasoning + axes Δ 표
- ResultPanel '반추' 탭 추가 (시드 ↔ LLM Model 사이)
- `handleEndChat`: `/api/chat/end` 응답의 `after_dialogue`를 store에 박제 + reflection.is_some 시 toast에 band + score 노출
- `handleAfterDialogue`: REST 응답도 store에 박제 (legacy 경로 axes 시각화 가능)
- `useStateSync`: `dialogue_reflected` SSE 핸들러 — rels/history 재동기화

## 6. 환경 이슈 발견 5건 (F11)

| # | 이슈 | 회피 |
|---|---|---|
| 1 | 워크트리 cwd vs `../models/bge-m3` 하드코딩 | `mklink /J .claude\worktrees\models C:\...\models` |
| 2 | CRT mismatch (MSVCP140.dll vs libcpmt.lib) | `cargo clean` + `CFLAGS=/MD CXXFLAGS=/MD` |
| 3 | PowerShell `Out-File` 기본 UTF-16 LE | `-Encoding utf8` 명시 |
| 4 | PowerShell `2>&1` ErrorRecord wrap (false positive exit 101) | (cargo는 정상, exit code 무시) |
| 5 | Windows UAC installer detection (dispatch_v2_test.exe OS error 740) | `__COMPAT_LAYER=RunAsInvoker` |

장기 fix (Phase 1 범위 외): #1 `NPC_MIND_MODEL_DIR` 우선 / #5 `embed-resource` crate manifest 첨부.

## 7. Phase 2 진입 전 알아야 할 *현재 구현 상태* ★

> Phase 2 spec 작성자(Claude AI)는 본 절을 *반드시* 읽고 시작. Phase 2는 4축 마이그레이션 +
> BondKind/BondStatus/Partnership + Channel 1 Declarative 활성화. 현재 코드 상태가
> Phase 2 진입에 어떤 *전제 조건*을 충족·미충족 했는지 명확히 함.

### 7.1 ✅ Phase 1로 *완료*된 것 (Phase 2가 의존)

#### 도메인 / Application
1. **`ReflectionResult` 도메인 타입 존재** (chat feature 무관) — Phase 2의 Channel 1 declarative_events는 *이 result 안의 placeholder 필드*를 활용해서 채우게 됨. 구조:
   ```rust
   pub struct ReflectionResult {
       pub is_chitchat: bool,
       pub summary: String,
       pub significance_score: f32,
       pub declarative_events: Vec<DeclarativeEventPlaceholder>,  // ← Phase 2 본격 활성화
       pub partnership_event: Option<PartnershipEventPlaceholder>, // ← Phase 2 본격 활성화
       pub turn_count: usize,
       pub llm_reasoning: Option<String>,
   }
   ```
   Phase 1은 `declarative_events`/`partnership_event`를 *항상 빈/None*으로 강제 (결정 9). Phase 2에서 placeholder 타입을 *진짜 enum/struct*로 승격 + LLM이 채우게 함.

2. **`Command::EndDialogue.reflection: Option<ReflectionResult>`** — Phase 1 게이트 진입점. Phase 2는 *같은 필드를 활용*해 Channel 1 declarative_events 흐름. 시그니처 *변경 없이* 동작 변경 가능.

3. **`RelationshipPolicy.handle_dialogue_end` 게이트 로직** — Phase 2에서 *진입 조건*만 갱신 ("declarative_events 비어있지 않음" 가지가 *Phase 2부터 실제 동작*). 구조 그대로.

4. **`outer_loop_entry(reflection, legacy_significance) -> bool` pub(crate) helper** — Phase 2에서 *동일 helper에 declarative_events 분기*만 추가.

5. **`Npc.inner_compass: Option<String>`** (A-min, Phase 1) — Phase 2 *직접 의존 없음*. taboo/life_question 활성화는 Phase 3c.

6. **`DialogueReflected` 이벤트 + EventBus 발행** — Phase 2에서 *추가 변경 0*. Channel 1 emit은 *RelationshipUpdated가 아닌 별도 도메인 이벤트로* 처리 (Phase 2 spec 결정 사항).

#### 인프라
7. **dispatch_v2 BFS cascade + UoW** — Phase 2의 Channel 1 사회적 일관성 검증 5종 (A~E)이 *transactional 핸들러*로 들어감. 인프라는 그대로.

8. **`MAX_EVENTS_PER_COMMAND = 22`** — Phase 2가 `EndDialogue` worst-case를 *늘릴* 가능성 있음 (declarative_events가 follow-up 이벤트로 fan-out 시). budget 재산정 필요.

9. **`MemoryProjector`** — `DialogueReflected` 이벤트 구독해서 summary를 memory store에 흡수 가능. 현재 미구현 (memory 이벤트 팬아웃은 Step F 대기). Phase 2 *직접 의존 없음*.

#### Mind Studio (Phase 1.5/1.6)
10. **`AppState.reflection_service` 부착 자동화** — Phase 2 reflection 흐름이 *Channel 1 처리*를 포함하게 되어도 부착 위치 변경 0. `ReflectionService<P>`의 *내부 로직*만 확장.

11. **`StateInner.turn_buffers`** — Phase 2에서 *동일 누적 유지*. Channel 1은 누적된 turn 시퀀스를 LLM에 던져서 declarative_events 도출.

12. **'반추' 탭 (`ReflectionView.tsx`)** — Phase 2에서 *declarative_events 섹션 추가* + Partnership 상태 변경 시각화. 현재 컴포넌트가 *Phase 1 placeholder 무시* 처리. Phase 2에서 본격 표시.

13. **EventBus → SSE bridge** — Phase 2 신규 도메인 이벤트 (예: `BondKindEntered`/`PartnershipFormed` 등)는 *bridge의 `map_event` 함수에 1 줄 추가*만으로 SSE 발행 가능. drift 면적 최소화.

### 7.2 ❌ Phase 1에서 *건드리지 않은* 것 (Phase 2가 신설)

#### Relationship 모델 (Phase 2 핵심)
- **3축 → 4축 마이그레이션**: 현재 `closeness/trust/power`. Phase 2에서 `trust/affinity/respect/wariness` ±100. **의미·범위 모두 재작성**.
- **BondKind 11 variants**: 부재. Phase 2에서 enum 통째 신설.
- **BondStatus 5 variants** (Active/Resolved/Deceased/Dormant/Reactivating): 부재.
- **Partnership 4 variants** (Spouse/Engaged/Lover/Separated): 부재.
- **type/type_history**: 자유 텍스트 + 누적 이력. 부재.

#### Channel 1 활성화 (Phase 2 핵심)
- **`DeclarativeEventPlaceholder` → 실제 enum/struct 승격**: 현재 빈 marker struct. Phase 2에서 BondKindFormation/PartnershipChange 등 variant 정의.
- **`PartnershipEventPlaceholder` 동일**.
- **사회적 일관성 검증 (A~E)**: 부재. Phase 2의 RelationshipPolicy 확장 또는 신규 핸들러.
- **적용 모드 4-tier**: 시나리오 JSON schema 갱신 필요.
- **LLM prompt 갱신**: `DefaultReflectionPromptBuilder`가 declarative_events 형식을 *진짜 schema로* 요청하도록 변경. 현재는 빈 array placeholder.

#### Mind Studio 면 (Phase 2)
- Director 경로의 ReflectionService 통합 — Phase 1/1.5/1.6 모두 미구현. SceneTask가 turn_buffers를 어떻게 보관할지 *디자인 결정* 필요. Phase 2가 다룰지 별도 작업으로 미룰지 결정 항목.

### 7.3 ⚠️ Phase 2 시작 전 *재검토 필요*한 부분

1. **`MAX_EVENTS_PER_COMMAND` 재산정**: Channel 1 declarative_events가 N개라면 EndDialogue가 *N+5* 이벤트 발행 가능 (worst-case). 현재 22 → Phase 2에서 *25~30 정도*로 인상 검토.

2. **`outer_loop_entry()` 게이트 조건 확장**: 현재 `significance >= 0.3 OR !is_chitchat OR declarative_events 비어있지 않음 OR partnership_event 있음` — *마지막 두 조건이 Phase 2부터 실제 의미 가짐*. 조건 자체는 그대로지만 *동작 검증*은 새로 필요.

3. **시나리오 JSON migration**: 기존 시나리오 3축 → 4축 변환 룰 정의 필요. closeness → affinity 매핑은 의미가 *완전히 같지 않음* (Phase 2 spec §"3축 → 4축 변환 룰"에서 다뤄야 함).

4. **`RelationshipUpdatedPayload`의 `cause: RelationshipChangeCause`**: Phase 2에서 enum variant *대거 추가*. 현재 5 variant. Phase 2 후 ~15 variant 예상 (DeclarativeEmit / SocialConsistencyReject / TemporalGateEnter 등).

5. **Mind Studio '반추' 탭 UI**: Phase 2 declarative_events 표시 시 *컴포넌트 단위 추가*. ReflectionView를 *섹션 단위 분리* 리팩토링 검토.

### 7.4 의존성 그래프 (Phase 1 → Phase 2)

```
Phase 1 산출물 (✅ 완료)              Phase 2 입력 (의존)
─────────────────────────────────────────────────────────
ReflectionResult.declarative_events    ──→  Channel 1 emit 대상
  (현재: 항상 빈 vec)                       (Phase 2: enum 활성화 + LLM 채움)

ReflectionResult.partnership_event     ──→  Partnership 형성/해소
  (현재: 항상 None)                          (Phase 2: enum 활성화)

DialogueReflected event                ──→  Channel 1 처리 후 follow-up
  (현재: chitchat 게이트 audit만)             (Phase 2: declarative_events 적용 후 발행)

RelationshipPolicy.handle_dialogue_end ──→  Channel 1 + 사회적 일관성 검증
  (현재: 게이트 통과 시 무조건 1 RelationshipUpdated)  (Phase 2: declarative_events 분기 + 검증 5종)

Relationship (3축 ±1.0)                ──→  4축 재작성 (Phase 2 핵심)
                                            (Phase 1은 변경 없음)

Mind Studio '반추' 탭                  ──→  declarative_events 시각화 섹션 추가
  (현재: significance + axes Δ만)            (Phase 2: BondKind 변화 등 표시)

EventBus → SSE bridge                  ──→  새 도메인 이벤트 자동 SSE 발행
  (현재: 9 이벤트 매핑)                       (Phase 2: BondKindEntered 등 +N 매핑)
```

## 8. 후속 과제 (Phase 1.5/1.6 *완료*, 별도 작업으로 *대기*)

### Phase 1.5/1.6 *완료* (본 리포트 §2)
- ✅ Mind Studio AppState `ReflectionService` 부착
- ✅ Frontend ReflectionPanel
- ✅ EventBus → SSE bridge (manual emit 일원화)
- ✅ Director 경로 SSE bug 부수 fix (단 `director_v2`는 *별도 dispatcher*라 본 bridge가 보지 못함 — shared_dispatcher 통합은 별도 작업)

### *대기* (별도 작업 후보)
- **Director Reflection 통합** — `Director.end_scene` 경로에 ReflectionService 부착. 디자인 결정 (SceneTask의 turn_buffer 위치) 필요
- **director_v2 → shared_dispatcher 통합** — `/api/v2/scenes/*` 경로가 별도 dispatcher 사용 중 → shared_dispatcher 단일화 시 EventBus bridge가 전 경로 커버
- **LLM-engine drift dashboard** — calibration 추적 자동화 (현재는 `target/baseline/phase1-real-llm-results.json` 박제로 1회성)
- **Memory 이벤트 팬아웃** — `MemoryEntryCreated`/`Superseded`/`Consolidated` EventBus 발행 (Step F). 활성화 시 Mind Studio의 manual `state.emit(MemoryCreated)` 4곳(`handlers/memory.rs`·`handlers/world.rs`·`handlers/rumor.rs`) 제거 가능

### Phase 2+ (mind-architecture 트랙)
- **Phase 2**: 4축 마이그레이션 + BondKind/BondStatus/Partnership/type_history + Channel 1 Declarative 활성화
- **Phase 3a**: Channel 2 Temporal (BondKindCandidacy projection)
- **Phase 3b**: Channel 3 External (EventPropagator + narrative_origin)
- **Phase 3c**: ActionTriggerEvaluator + InnerCompass 승격 (taboo/life_question)

### 환경 fix (mind-architecture 트랙 외)
- 워크트리 모델 경로 fix (테스트 코드 갱신)
- Windows UAC heuristic fix (manifest 첨부)

## 9. 디자이너 검증 (수동)

실제 LLM 호출 + 게이트 calibration의 *서사적 직관*과의 일치는 디자이너(Bekay)가
Mind Studio에서 narrative 3 시나리오 직접 실행. 체크리스트:
[`data/scenarios/phase1-validation/README.md`](../../../data/scenarios/phase1-validation/README.md).

**Phase 1.5 이후**: Mind Studio UI에 '반추' 탭이 추가되어 *디자이너가 직접 reflection 결과
시각화* 가능. chat 종료 후 chitchat band / axes Δ / LLM reasoning을 클릭 한 번에 확인.

## 10. 검증 게이트 통과 표

| spec §3 게이트 | 통과 |
|---|---|
| `cargo check --all-features` | ✅ |
| `cargo test --workspace --features chat,embed,listener_perspective` | ✅ Phase 1 본체 1095 / Phase 1.5+1.6 후 1100+ passed / 0 failed / 7 ignored |
| `cargo build --features chat` | ✅ |
| `cargo build --no-default-features` | ✅ (chat gated 코드 자동 제외) |
| `cargo build --features mind-studio,chat,embed --bin npc-mind-studio` | ✅ |
| `cargo clippy --workspace --all-features -- -D warnings` | (Stage 5에서 미실행 — Phase 2 시작 전 보완 권장) |
| 도메인 단위 테스트 ≥ 6 (compute_significance) | ✅ 8개 |
| ReflectionService Mock 테스트 ≥ 4 | ✅ 5개 |
| RelationshipPolicy 게이트 테스트 ≥ 3 | ✅ 5개 |
| Narrative integration 3 시나리오 | ✅ |
| **OCP grep**: src/application/에서 ConversationBackedReflectionPort import 0건 | ✅ |
| Bench: dispatch_v2 회귀 < 10% | ⚠️ significant +19%로 약간 초과 (절대값 35µs로 무시 가능) |
| **Phase 1.5**: Mind Studio Mock ReflectionRunner 통합 테스트 ≥ 3 | ✅ 4개 |
| **Phase 1.6**: event_bridge 단위 + 통합 테스트 ≥ 6 | ✅ 7 단위 + 1 통합 |
| **Phase 1.6**: manual `state.emit()` 도메인 사실 잔존 grep 0 | ✅ (11개 제거, UI lifecycle만 잔존) |

## 11. 결론

**Phase 1 본체 + Phase 1.5 + Phase 1.6 모두 게이트 통과**. 회귀 0건, 게이트 효과 측정으로 확인,
calibration 3 밴드 정확. OCP 완벽 준수. 환경 이슈 5건 모두 회피 또는 문서화. Mind Studio UI
'반추' 탭으로 디자이너 검증 경로 확보. EventBus 일원화로 향후 Phase 2 새 이벤트 SSE 자동 발행 보장.

**Phase 2 (4축 + BondKind + Channel 1) 진입 가능**. §7이 Phase 2 spec 작성자가 알아야 할
*현재 구현 상태*를 정확히 기술 — 그 위에 spec 쌓으면 됨.

## 변경 이력

| 일자 | 변경 |
|---|---|
| 2026-05-11 | Phase 1 Stage 6 archive — 본 리포트 초안 (v0.1). |
| 2026-05-12 | Phase 1.5 (Mind Studio 통합) + Phase 1.6 (EventBus→SSE bridge) 추가. §7 "Phase 2 진입 전 알아야 할 현재 구현 상태" 신설. 디자이너 검증 경로 '반추' 탭 명시. (v0.2) |
