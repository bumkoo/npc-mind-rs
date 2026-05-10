# Task — Phase 1: Scene Boundary Reflection 도입

> **목적.** 현재 `DialogueEndRequested` → `RelationshipPolicy`가 *무조건* 3 follow-up을 발행하는 흐름에 **Reflection 단계**를 끼워, *잡담은 outer loop 건너뜀* / *의미 있는 사건은 그대로 진행*하도록 게이트 추가. LLM이 서사적 의미를 판정 (`is_chitchat`, `summary`, declarative_events placeholder), 엔진이 정량 `significance_score`를 결정론으로 계산. 둘이 합쳐 `DialogueReflected` 이벤트로 박제. `RelationshipPolicy`가 게이트 통과 시만 `RelationshipUpdated` 발행 (`EmotionCleared`/`SceneEnded`는 항상).
>
> **핵심 변경 위치.** Domain (`event.rs`, 신규 `reflection.rs` 또는 `relationship.rs` 확장), Application (`reflection_service.rs` 신설, `command/policies/relationship_policy.rs` 진입 조건 확장, `dialogue_orchestrator.rs` turn buffer 추가), Ports (`reflection.rs` 신규), Adapter (`reflection_via_chat.rs` 신규).
>
> **범위.** 4축 마이그레이션·BondKind·ActionTrigger·Channel 1/2/3 모두 *비포함* — Phase 2/3 이연. 본 phase는 *Reflection 인프라*만.
>
> **소요 예상.** Domain ~80 LoC + Ports/Adapter ~150 LoC + Application service ~200 LoC + RelationshipPolicy 변경 ~40 LoC + DialogueOrchestrator 변경 ~50 LoC + DTO/MCP/REST 확장 ~80 LoC + 테스트 5~7개.

---

## 1. 배경 — 현재 구조와 갭

### 1.1 현재 dispatch_v2 흐름 (Outer Loop entry)

```
DialogueOrchestrator.end_session(sid, significance?)
  ├─ ConversationPort::end_session
  └─ if significance:
       Command::EndDialogue.dispatch_v2().await
         → DialogueEndRequested { significance: Option<f32> }
         → RelationshipPolicy.handle:
             → RelationshipUpdated  (★ 무조건)
             → EmotionCleared        (무조건)
             → SceneEnded            (무조건)
```

### 1.2 갭 — 잡담도 axes 변화

`relationships.md` v0.7 §6.0 인용: base_delta 표(±10~25)를 *매 dialogue 종료 시* 적용하면 양극 도달이 너무 빠르다. 무협 시간감과 어긋남.

검증 사례:
- *길에서 행인과 인사* — 현재: 만나는 모든 NPC 관계가 미세 변동. 누적되면 axes drift.
- *의례적 잡담 (주막 종업원과 음식 주문)* — 현재: 같은 axes 변동.
- vs. *임충 산신묘 처단 사건* — 현재: 같은 base_delta. 한 턴의 큰 사건과 100턴의 잡담이 *동등 무게*.

→ 시스템이 *서사적 비중*을 구별하지 못함.

### 1.3 본 phase가 해결하는 것

**Reflection 단계 신설**: `after_dialogue` 시점에 LLM이 *서사적 의미*를 판정, 엔진이 *정량 격동도*를 계산. 둘이 합쳐 `DialogueReflected` 이벤트로 영속화. `RelationshipPolicy`가 *조건부 진입*.

게이트 통과 조건 (`relationships.md` v0.7 §6.4 가드레일):

```
significance >= 0.3
OR  !is_chitchat
OR  declarative_events 비어 있지 않음    (Phase 1엔 항상 비어 있음 — placeholder)
OR  external_events 비어 있지 않음        (Phase 3b 입력 — Phase 1엔 항상 비어 있음)
OR  temporal_signals 비어 있지 않음       (Phase 3a 입력 — Phase 1엔 항상 비어 있음)
```

→ Phase 1에서 *실효 게이트*는 `significance >= 0.3 OR !is_chitchat`. 나머지 셋은 미래 phase 입력.

### 1.4 본 phase가 *해결하지 않는* 것

`relationships.md` v0.7 §6.7 phasing 그대로:

| 항목 | 이연 phase |
|---|---|
| 4축 (trust/affinity/respect/wariness) 마이그레이션 | Phase 2 |
| BondKind / BondStatus / Partnership / type_history | Phase 2 |
| Channel 1 Declarative 적용 (declarative_events 활성화) | Phase 2 |
| 사회적 일관성 검증 5 카테고리 (A~E) | Phase 2 |
| 적용 모드 (allowlist/audit/reject 4-tier) | Phase 2 |
| Channel 2 Temporal (BondKind 카운터) | Phase 3a |
| Channel 3 External (EventPropagator + narrative_origin) | Phase 3b |
| ActionTriggerEvaluator + 추모 행동 emit | Phase 3c |
| Frontend ReflectionPanel (UI 표시) | Phase 1.5 follow-up |

본 phase는 *Reflection 인프라 + significance 엔진 계산 + chitchat 게이트* 셋만.

---

## 2. 목표

1. **`ReflectionService` 신설** — Application layer에서 LLM 호출 + 엔진 신호 결합. `<P: ReflectionPort>` 제네릭으로 OCP 준수.
2. **`compute_significance(turns)` 도메인 함수** — turn-level OCC/PAD 신호 4가지를 가중 합산 (peak_occ/pad_magnitude/diversity/beat_signal). 결정론, replay 가능.
3. **`ReflectionPort` 트레이트 + `ConversationBackedReflectionPort` 어댑터** — Phase 1 어댑터는 ConversationPort 별도 세션으로 같은 모델·다른 KV slot. 미래에 다른 모델용 어댑터 추가 시 ReflectionService 변경 0.
4. **`DialogueReflected` 신규 EventKind + payload** — Reflection 결과를 도메인 이벤트로 박제. Replay 결정성 확보.
5. **`RelationshipPolicy` 게이트 추가** — `DialogueEndRequested.payload.reflection`을 보고 RelationshipUpdated 조건부 발행. EmotionCleared/SceneEnded는 무조건 유지.

---

## 3. 완료 기준 (Definition of Done)

### 코드 레벨

- [ ] `src/ports/reflection.rs` 신규 파일 — `ReflectionPort` trait + `ReflectionPrompt` + `ReflectionError`. `[chat]` feature gate.
- [ ] `src/adapter/reflection_via_chat.rs` 신규 파일 — `ConversationBackedReflectionPort<C>` 구현체.
- [ ] `src/application/reflection_service.rs` 신규 파일 — `ReflectionService<P>` + `ReflectionPromptBuilder` trait + `DefaultReflectionPromptBuilder`.
- [ ] `src/domain/event.rs` — `DialogueReflected` 새 EventKind + payload (`ReflectionResult` 포함).
- [ ] `src/domain/event.rs` — `DialogueEndRequested` payload에 `reflection: Option<ReflectionResult>` 필드 추가 (Option은 chat feature 비활성 시 동작 호환).
- [ ] `compute_significance(turns: &[TurnSnapshot]) -> f32` 함수 + `TurnSnapshot` struct — 위치 Stage 0 결정 (`domain/relationship.rs` vs 신규 `domain/reflection.rs`).
- [ ] `src/application/command/policies/relationship_policy.rs` — `DialogueEndRequested` 핸들러 진입 조건 변경 (게이트 통과 시만 `RelationshipUpdated` 발행).
- [ ] `src/application/dialogue_orchestrator.rs` — `turn_buffers: HashMap<SessionId, Vec<TurnSnapshot>>` 신설, `turn()`에서 누적, `end_session()`에서 ReflectionService 호출.
- [ ] DTO 확장 — `AfterDialogueResponse` (또는 동등)에 `reflection: Option<ReflectionResult>` 필드 추가. 위치 Stage 0 결정.

### 테스트 레벨

- [ ] `compute_significance` 단위 테스트 — 4가지 신호 각각의 영향 검증. ≥4 케이스.
- [ ] `ReflectionService` 단위 테스트 — `MockReflectionPort` 주입. is_chitchat=true / =false 분기 검증. ≥3 케이스.
- [ ] `ConversationBackedReflectionPort` 통합 테스트 — `chat` feature 활성, 실제 LLM 호출 (또는 mock chat). 1 케이스.
- [ ] `RelationshipPolicy` 게이트 단위 테스트 — DialogueEndRequested payload reflection 변형으로 RelationshipUpdated 발행 여부 분기. ≥3 케이스.
- [ ] Narrative integration 테스트 — 잡담/일상/결단 3 밴드. 각 1 케이스.
- [ ] 회귀: 기존 `tests/dialogue_*` + `tests/dispatch_v2_test` 모두 통과 (회귀 0).

### 검증 grep (Stage 0/Stage 5에서 실행)

- [ ] `findstr /S /I "ReflectionPort" src\application\reflection_service.rs` — 트레이트만 import, 구체 어댑터 0건 (OCP 검증).
- [ ] `findstr /S /I "ConversationBackedReflectionPort" src\application\` — application/ 안에서 *어댑터 직접 참조 0건* (DI는 외부에서).
- [ ] `findstr /S /I "DialogueReflected" src\domain\event.rs src\application\command\policies\` — EventKind 추가 + 핸들러 발행 명시.

### 빌드/품질 게이트

- [ ] `cargo check --all-features` 통과.
- [ ] `cargo build --features chat` 통과.
- [ ] `cargo build --no-default-features` 통과 (chat feature 비활성 시 ReflectionService 코드 컴파일 제외, 기존 동작 유지).
- [ ] `cargo test --workspace --all-features` 통과 (회귀 0 + 신규 케이스).
- [ ] `cargo clippy --workspace --all-features -- -D warnings` 경고 없음.

### 수동 검증

- [ ] Mind Studio에서 잡담 시나리오 실행 → axes 변화 0, summary만 메모리에 저장 확인.
- [ ] Mind Studio에서 임충 산신묘 시나리오 실행 → DialogueReflected event 발행 + axes 변화 그대로 확인.
- [ ] LLM 응답이 invalid JSON일 때 fallback 동작 확인 (게임 진행 막힘 없음).

---

## 4. 전제 및 주의사항

### 4.1 수정 허용 범위

**필수 수정**:
- `src/ports/` — `reflection.rs` 신규 추가, `mod.rs` 한 줄 추가
- `src/adapter/` — `reflection_via_chat.rs` 신규, `memory_repository.rs` (시나리오 JSON `inner_compass` `serde(default)` deserialization — Stage 0 Findings F4)
- `src/application/` — `reflection_service.rs` 신규, `command/policies/relationship_policy.rs` 수정, `dialogue_orchestrator.rs` 수정, DTO 한 곳
- `src/domain/event.rs` — EventKind 추가, payload 확장
- **`src/domain/personality.rs` — A-min: `Npc.inner_compass: Option<String>` 필드 + `compass_short_label() -> Option<&str>` 메서드 (~30 LoC, Stage 0 Findings F4 결정)**
- `src/domain/relationship.rs` 또는 신규 `domain/reflection.rs` — `compute_significance` + `TurnSnapshot`
- `src/bin/mind-studio/` — `domain_sync.rs` (dispatch_end_dialogue 응답 확장), `mcp_server.rs` (dialogue_end tool 응답 확장), 관련 handler

**수정 금지**:
- 다른 도메인 모듈 (`pad`, `emotion`, `memory`, `world` 등) — *`personality`는 A-min minimal 변경 허용 (Stage 0 Findings F4)*
- 다른 application service (`scene_service`, `situation_service`, `memory_projector`, `director` 등)
- 다른 Inline 핸들러 (`telling_ingestion_handler`, `world_overlay_handler`, `scene_consolidation_handler`, `relationship_memory_handler`)
- `command/dispatcher.rs` (UoW 흐름 자체) — 신규 핸들러 등록만 *주입 시점*에 결정 (변경 없음)
- `worldbuilding/`, `lore/`, `bin/world_load.rs`, `bin/lore_ingest.rs`
- `mind-studio-ui/` (Frontend) — Phase 1.5 follow-up

### 4.2 Frontend 비변경 (Phase 1.5 follow-up)

`mind-studio-ui/`는 *Phase 1 범위 외*. Reflection 결과를 시각화하는 ReflectionPanel은 별도 task (`task-rel-phase1.5-frontend-reflection.md` 가칭).

Phase 1에서는 *백엔드 응답에 reflection 필드만* 포함. Frontend는 그 필드를 *무시*해도 동작. UI 갱신 시 발견 가능하도록 SSE `StateEvent::DialogueReflected` variant *최소 추가*는 해도 좋음 (1줄 — 진정 필수면 Stage 3에 포함, 아니면 Phase 1.5).

### 4.3 chat feature 비활성 시 동작 호환

`chat` feature는 LLM 대화 오케스트레이터의 활성 토글. 본 phase의 Reflection도 LLM 호출이라 `chat` feature와 *함께 활성*되어야 함.

**호환 정책**:

| 빌드 | ReflectionService | RelationshipPolicy 동작 |
|---|---|---|
| `--features chat` (활성) | 작동. ReflectionService 인스턴스 생성됨, end_session에서 호출. | 게이트 적용. `reflection: Some(...)` payload 보고 분기. |
| `--no-default-features` 또는 `chat` 비활성 | 코드 컴파일 제외. ReflectionService 없음. | `reflection: None` payload. 게이트 평가 시 *모든 조건 미충족* → outer loop *안 들어감*. |

→ **chat feature 없으면 `RelationshipUpdated` 발행이 *전부 차단*됨**. 이게 의도된 동작인지 결정 필요 (Stage 0 결정 항목).

대안: chat feature 비활성 시 `reflection: None`을 *"reflection 안 거쳤음"*으로 해석, RelationshipPolicy가 *기존 무조건 동작*으로 폴백. 이게 더 안전 (기존 사용자 코드 깨지지 않음).

→ 권장 (Stage 0에서 확정): **`reflection: None` → 기존 무조건 동작 (호환)**. `reflection: Some(_)` → 게이트 적용.

### 4.4 핵심 설계 결정 (확정 사항)

이번 spec 작성 전 대화에서 합의된 11개 결정 + Stage 0 Findings 신규 2개 (사)·(아) = **총 13개 결정** — Phase 1 구현 시 *반드시 준수*. Stage 0 결정 (가)~(마)는 Stage 0 Findings에서 확정, (바)는 A-min으로 선결정 (F4).

1. **LLM 호출은 dispatch_v2 *바깥*** (DialogueOrchestrator.end_session 안). dispatch_v2의 동기 fast-path 보존. UoW invariant 유지.
2. **별도 Reflection 에이전트** — 같은 LLM 모델, 별도 ConversationPort 세션. Dialogue 세션의 KV 캐시 유지.
3. **OCP 준수** — `ReflectionService<P: ReflectionPort>` 제네릭. Phase 2.5+에서 다른 모델 어댑터 추가 시 기존 코드 변경 0. *Stage 0 보정 (Findings F2 #2)*: DialogueOrchestrator는 `<R, C>` 유지 + `Option<Arc<dyn ReflectionService>>` 트레이트 객체 — generic 추가 0.
4. **Phase 1 어댑터** — `ConversationBackedReflectionPort<C: ConversationPort>`. 기존 ConversationPort 메서드(`start_session`/`send_message`/`end_session`)만 사용. 새 trait 메서드 추가 0.
5. **TurnSnapshot 누적** — DialogueOrchestrator의 `turn_buffers: HashMap<SessionId, Vec<TurnSnapshot>>` 책임. 매 `turn()`에서 누적, `end_session()`에서 회수 후 ReflectionService 인자로 전달. *Stage 0 보정 (Findings F1 #3)*: plain HashMap (`&mut self` 일관성), Mutex 불필요.
6. **Reflection을 dispatch_v2 입력에** — `Command::EndDialogue { reflection }`. payload로 박혀 들어감. 핸들러는 *읽기만*. *Stage 0 보정 (Findings F2 #1)*: `ReflectionResult`는 chat feature 무관 순수 도메인 타입 → 필드 항상 존재, chat 비활성 시 `None`.
7. **DialogueReflected 항상 발행** — RelationshipPolicy가 outer loop skip 케이스에서도 reflection 결과를 박제 (memory_projector가 summary 흡수, audit 가능).
8. **게이트 조건 정확** — `significance >= 0.3 OR !is_chitchat OR declarative_events 비어있지 않음 OR external_events 비어있지 않음 OR temporal_signals 비어있지 않음`.
9. **Phase 1 placeholder** — declarative_events / partnership_event는 항상 *empty / None*. ReflectionLlmOutput 스키마에 슬롯은 있음, 사용은 Phase 2.
10. **동기 실행** — Phase 1은 `end_session().await`이 reflection 완료까지 블록. Async 변형 (Director Spawner 활용)은 미래 phase.
11. **JSON 파싱 실패 fallback** — invalid JSON or LLM timeout 시 fallback ReflectionResult. is_chitchat=false (보수적: outer loop 진입), significance=engine 값, declarative 비어있음. 게임 진행 막히지 않음.
12. **(사) turn_buffers 소유 위치** — DialogueOrchestrator 내부 (Stage 0 Findings F3, **Bekay 확정 2026-05-10**). domain_sync는 별도 turn_buffers 미가짐 → Mind Studio가 DialogueOrchestrator 인스턴스를 통해서만 reflection.
13. **(아) `MAX_EVENTS_PER_COMMAND` 처리** — 21 → 22로 상수 인상 (Stage 0 Findings F3 옵션 (a), **Bekay 확정 2026-05-10**). 미인상 시 `EventBudgetExceeded`로 EndDialogue dispatch 실패. §11.7 위험 참조.

**선결정 (Stage 0 Findings F4)** — (바) `compass_short_label()` 신설 전략: **A-min 채택** — `Npc.inner_compass: Option<String>` + `compass_short_label() -> Option<&str>` (~30 LoC, [src/domain/personality.rs](src/domain/personality.rs)). taboo/life_question은 Phase 3c 승격.

---

## 5. 작업 명세

본 phase는 *6 stage*. Stage 0이 작업 시작 전 *불확실한 부분을 확정*하는 단계 — 결과가 spec §1의 "Impact Map"과 §4의 "핵심 설계 결정"을 *최신화*함.

### Stage 0 — Pre-flight Impact Analysis

**목적**: 코드 변경 전, ripple 영향 위치 모두 발견 + 미확정 결정 사항 확정. spec § 4.4의 11개 결정 *재확인* + 새로 떠오른 사항 catch.

**산출물**: 본 spec 문서 *내부에* "Stage 0 Findings" 섹션 추가 (별도 문서 아님). 결정 사항 박제.

#### 0.1 grep audit 패턴 (8개)

```bash
# 1. DialogueEndRequested 호출자 — 진입 조건 변경 영향
findstr /S /I "DialogueEndRequested" src\

# 2. EventKind 매칭 — DialogueReflected variant 추가 시 갱신 필요한 모든 match
findstr /S /I "EventKind::" src\application\command\
findstr /S /I "EventKind::" src\application\memory_projector.rs

# 3. ConversationPort 다중 세션 동시 사용 패턴 확인
findstr /S /I "start_session" src\adapter\rig_chat.rs
findstr /S /I "session_id" src\adapter\rig_chat.rs

# 4. AfterDialogueResponse (또는 동등 DTO) 위치
findstr /S /I "AfterDialogue" src\application\dto\
findstr /S /I "AfterDialogue" src\application\dialogue_orchestrator.rs

# 5. MAX_EVENTS_PER_COMMAND budget 영향
findstr /S /I "MAX_EVENTS_PER_COMMAND" src\application\command\

# 6. domain_sync helper의 dispatch_end_dialogue
findstr /S /I "dispatch_end_dialogue" src\bin\mind-studio\

# 7. dialogue_end MCP tool
findstr /S /I "dialogue_end" src\bin\mind-studio\mcp_server.rs

# 8. tests/dialogue_* — 회귀 테스트 영향
dir /B tests\dialogue_*.rs
```

각 grep 결과를 *이 spec 문서의 Stage 0 Findings 섹션에 캡처*. 영향 위치 별로 변경 종류 (additive / breaking / 영향 없음) 분류.

#### 0.2 핵심 파일 spot-read 목록 (4개)

이번 phase의 *변경 정확도*를 위해 직접 읽어야 하는 파일:

| 파일 | 확인 내용 |
|---|---|
| `src/adapter/rig_chat.rs` | `start_session`/`send_message`/`end_session` 동시 다중 세션 지원 여부 (HashMap 기반인지). 같은 RigChatAdapter 인스턴스가 dialogue_sid + reflection_sid 동시 처리 가능한지. |
| `src/application/command/policies/relationship_policy.rs` | 현재 `DialogueEndRequested` 핸들 코드. UoW.add_event 패턴, follow-up 발행 순서. |
| `src/application/dialogue_orchestrator.rs` | `start_session`/`turn`/`end_session` 시그니처. session 관리 패턴 (HashMap?). |
| `src/application/command/uow.rs` | UoW가 도메인 이벤트를 어떻게 누적하는지. RelationshipPolicy가 추가 이벤트 발행 시 호출 패턴. |

→ 이 4 파일을 직접 읽고 결과를 *Stage 0 Findings에 요약*. spec의 후속 stage가 정확한 패턴 위에서 작성될 수 있도록.

#### 0.3 결정 항목 확정 (Stage 0 종료 전)

다음 미결정 항목을 *코드 spot-read 결과 기반으로 확정*. 결정을 spec §4.4에 추가/변경.

**(가) `compute_significance` + `TurnSnapshot` 위치**:
- (a) `src/domain/relationship.rs`에 추가 — 기존 도메인 모듈에 흡수
- (b) `src/domain/reflection.rs` 신규 — 별도 도메인 모듈
- → 권장: **(b)** Phase 2/3에서 reflection 관련 도메인 함수가 더 늘어남. 미리 자리 마련. 단 Phase 1 분량 +1 파일.

**(나) DTO `reflection: Option<ReflectionResult>` 위치**:
- (a) `application/dto/scene.rs`에 추가 — AfterDialogueResponse가 거기 있다면
- (b) `application/dto/reflection.rs` 신규 — 별도 DTO 모듈
- → grep 0.1#4 결과 보고 결정. Phase 2의 ReflectionResult 확장 시 (b)가 깔끔.

**(다) chat feature 비활성 시 RelationshipPolicy 동작**:
- (a) reflection: None → 기존 무조건 동작 (호환 유지) ★ 권장
- (b) reflection: None → 모든 outer loop 차단 (chat feature 강제)
- → 라이브러리 GitHub 공개 목표 고려, **(a)** 호환성 우선.

**(라) `RelationshipPolicy.handle`의 follow-up 발행 순서**:
1. `DialogueReflected` 먼저
2. `RelationshipUpdated` (조건부)
3. `EmotionCleared`
4. `SceneEnded`

→ 현재 RelationshipPolicy 코드 읽고 (Stage 0.2#2) 기존 follow-up 순서 확인 후 *최소 변경*으로 새 발행 추가. 발행 순서가 trace에 박힘.

**(마) `Director`의 `end_scene`도 동일 패턴 적용?**:
- 현재 `Director::end_scene(scene_id, significance)`이 `Command::EndDialogue` 보냄 (CLAUDE.md 인용)
- Director 경로도 reflection 거쳐야 의미 있음
- 하지만 Director가 *spawned task*라 ReflectionService 호출 위치 신중. 단순화: Director 경로는 *Phase 1.5*로 미루고 Phase 1은 DialogueOrchestrator만.
- → 권장: **DialogueOrchestrator만 Phase 1**. Director는 별도 task에서 처리 (Stage 0 결정으로 박제).

#### 0.4 게이트 — Stage 0 종료 조건

다음 모두 충족 시 Stage 1 진입 허용:

- [ ] grep audit 8개 모두 실행 + 결과 spec §"Stage 0 Findings" 추가
- [ ] spot-read 4 파일 완료 + 핵심 패턴 spec에 박제
- [ ] 결정 항목 (가)~(마) 모두 확정 + spec §4.4에 추가
- [ ] 추가 위험 발견 시 spec §11에 추가
- [ ] Impact Map 표 (영향 파일 × 변경 종류) 완성

→ Stage 0 *코드 변경 0*. 분석·결정만. *spec 문서 갱신*이 산출물.

---

### Stage 0 Findings (2026-05-10 검토 결과)

> **검토 환경**: Claude (claude-opus-4-7) — 본 워크트리 `claude/silly-engelbart-293e54` (commit `771de13` base, spec/KICKOFF는 main commit `87c8b32` untracked로 메인 저장소에만 존재). cargo test baseline·dispatch_v2 latency 미실측 (LLM 의존 통합 테스트는 llama-server 가동 환경 필요 — 후속 측정).
> **검증 방법**: 8 grep audit 중 5 패턴 + 4 spot-read 모두 + 추가 6 파일. Explore 에이전트 2종 cross-check.

#### F1. Tier B 7-가정 검증 결과

| # | spec 가정 | 실제 코드 | 결과 |
|---|---|---|---|
| 1 | `RigChatAdapter` 다중 세션 동시 처리 | `Arc<RwLock<HashMap<String, ChatSession>>>` 기반 | ✅ — dialogue + reflection 세션 동시 보유 OK |
| 2 | `RelationshipPolicy` follow-up 발행 | `relationship_policy.rs:194-231 handle_dialogue_end` 정확히 3 follow-up + UoW clear 시그널 | ✅ — spec 가정 정확. 게이트 추가는 본 메서드 분기 |
| 3 | `DialogueOrchestrator` 세션→NPC 매핑 | `sessions: HashMap<String, SessionMeta>` (`&mut self` 직접, Mutex 없음) | ⚠️ — spec §2.5 `tokio::sync::Mutex<...>` 가정과 차이. **turn_buffers도 plain HashMap (`&mut self` 일관성)로** |
| 4 | UoW `add_event` 호출 패턴 | UoW는 dirty checking; follow-up 이벤트는 `HandlerResult.follow_up_events: Vec<DomainEvent>` 반환 | ⚠️ — spec §2.4 의사코드 "ctx.add_event"는 부정확. `Ok(HandlerResult { follow_up_events: vec![...] })` 반환 패턴이 맞음 |
| 5 | `AfterDialogueResponse` 위치 | `src/application/dto/relationship.rs` 별도 모듈 (DialogueOrchestrator는 `dto/mod.rs` re-export) | ✅ — 결정 (나)는 (b) 신규 `dto/reflection.rs`가 깔끔 |
| 6 | `Npc::compass_short_label()` 메서드 | ❌ 부재. `Npc` = `id/name/description/personality: HexacoProfile` 4 필드. **`inner_compass` 도메인 자체 부재** | ⚠️ → ✅ **A-min 채택** (사용자 확정 2026-05-10). F4 참조 |
| 7 | `uuid` crate 의존성 | `Cargo.toml:42` — `uuid = { version = "1.11", features = ["v4"], optional = true }` (embed/mind-studio gated) | ⚠️ — **chat feature 단독에서 미가용**. chat에 추가하거나 reflection_sid를 `format!("reflection-{epoch_ms}-{counter}")`로 우회 |

**가정 검증 결론**: 7개 중 ✅ 4 / ⚠️ 3 (F2 의사코드 보정 + F4 결정으로 모두 해소). 본질적 blocker 0건.

#### F2. spec이 다루지 않은 코드 사실 (5건)

1. **`Command::EndDialogue` 시그니처** ([types.rs:38-42](src/application/command/types.rs:38)) — 현재 `npc_id/partner_id/significance: Option<f32>` 3 필드. spec의 `reflection: Option<ReflectionResult>` 추가는 결정 (다)와 정합 — **`ReflectionResult`를 chat feature 무관 순수 도메인 타입으로** 두면 `EventPayload::DialogueEndRequested` 필드도 항상 존재 가능 (필드 자체는 항상 존재, chat 비활성 시 `None`).

2. **`DialogueOrchestrator` 제네릭** ([dialogue_orchestrator.rs:140](src/application/dialogue_orchestrator.rs:140)) — 현재 `<R: MindRepository, C: ConversationPort>`. spec §2.5의 `<R, C, P: ReflectionPort>` 추가는 모든 callsite 깨뜨림. **권장 보정**: `reflection_service: Option<Arc<dyn ReflectionService>>` 트레이트 객체 또는 `Option<Box<dyn ...>>` — generic 추가 0, OCP 유지, callsite 무영향.

3. **`EventPayload::DialogueEndRequested` 현재 payload** ([event.rs:39](src/domain/event.rs:39)) — 정확히 spec 가정과 일치. spec §7.2의 "기존 호출자 reflection: None 추가" 작업은 **Director 경로 포함 grep 필수** (F3 §6.6 참조).

4. **`EventKind` enum 변형 확장 영향** ([event.rs:23-77](src/domain/event.rs:23)) — 현재 28 variants. `DialogueReflected` 추가 시 (a) `payload_type()` 매칭, (b) `EventPayload::aggregate_key()` → `AggregateKey::Npc(npc_id)`, (c) `EventKind::iter()`는 부재 → 모든 `EventKind::` grep 위치 수동 추가, (d) `correlation_id`/`parent_event_id`는 BFS 큐잉 시 자동 설정.

5. **`RelationshipPolicy::handle_v2`의 partner_id 회수** — spec §2.4 의사코드 "..."는 단순 `.clone()`로 충족. **`DialogueEndRequested` payload에 이미 `partner_id` 포함** ([relationship_policy.rs:60-62](src/application/command/policies/relationship_policy.rs:60)). 별도 ctx 호출 불필요.

#### F3. 신규 결정 항목 (사)·(아)

기존 결정 (가)~(마) 5개 + 본 검토 신규 2개 (= 총 13개, 단 (바)는 F4로 선결정):

- **(사) turn_buffers 소유 위치** — DialogueOrchestrator vs Mind Studio AppState. **확정 (Bekay 2026-05-10)**: DialogueOrchestrator 내부 (`HashMap<String, Vec<TurnSnapshot>>`, `&mut self` 일관성). domain_sync는 별도 turn_buffers 미가짐 → Mind Studio가 DialogueOrchestrator 인스턴스를 통해서만 reflection.
- **(아) `MAX_EVENTS_PER_COMMAND` 처리** — 현재 21 (worst-case). `DialogueReflected` 1개 추가 → 22가 됨. 옵션: (a) 상수 22로 인상 (가장 단순) (b) 21 유지 + DialogueReflected를 Inline phase로 이관 (cascade 깊이 감소). **확정 (Bekay 2026-05-10)**: **(a) 상수 22로 인상** — 테스트 회귀 0, dispatcher.rs 한 줄 변경.

#### F4. ❗ A-min 결정 (사용자 확정 2026-05-10) — `Npc::compass_short_label()` 신설

spec §2.3 `DefaultReflectionPromptBuilder.build()`가 호출. 현재 부재. `Npc` 도메인에 **`inner_compass` 필드 자체 미존재** — `_schema.md`/character-validation 문서에만 정의된 디자인 콘텐츠.

**A-min 구현** (~30 LoC, [src/domain/personality.rs](src/domain/personality.rs)):
1. `Npc` struct: `inner_compass: Option<String>` 필드 추가 (4 → 5 필드)
2. `NpcBuilder::with_inner_compass(s)` 빌더 메서드 (NpcBuilder 패턴 일관성)
3. `Npc::inner_compass(&self) -> Option<&str>` getter
4. `Npc::compass_short_label(&self) -> Option<&str>` — Phase 1: `inner_compass.as_deref()` 그대로 (cut 없음). 후속에서 첫 N자 cut 또는 short_form 필드
5. 시나리오 JSON 로더 ([adapter/memory_repository.rs](src/adapter/memory_repository.rs) `from_file/from_json`): `serde(default)` deserialization — 기존 시나리오 호환

**범위**: compass 한 줄만. taboo/life_question은 Phase 3c (ActionTrigger) 시점에 `InnerCompass` struct로 승격. YAGNI.

**spec 본문 영향**: 
- §4.1 "수정 금지"에서 `personality` 제외 (minimal 변경 허용 명시)
- §9.3 "변경 없음" 표에서 `personality.rs` 제거
- §9.2 "수정 파일" 표에 `personality.rs` + `adapter/memory_repository.rs` 행 추가
- §11에 신규 위험 1건 (시나리오 JSON migration — F5 참조)

→ Stage 0 결정 (바) **선결정 — 폐지**.

#### F5. spec §11에 누락된 위험 (2건 추가)

§11.7 + §11.8로 본 spec 하단에 추가됨. 요약:
- **§11.7 event budget** — `MAX_EVENTS_PER_COMMAND = 21` → 22 인상 필요. 미인상 시 `EventBudgetExceeded` 에러로 `EndDialogue` dispatch 실패.
- **§11.8 inner_compass JSON migration** — 기존 시나리오 JSON에 `inner_compass` 키 부재. `serde(default)` + `Option`으로 회피. 검증 시나리오 3종 (chitchat-passerby/daily-training/lin-chong-shanshenmiao)에는 명시적으로 inner_compass 추가.

#### F6. 의사코드 보정 (Stage 1~2 진입 전)

spec 본문 의사코드 3곳 보정 필요:
- **§2.4 `ctx.add_event(...)` → `Ok(HandlerResult { follow_up_events: vec![...] })`** — UoW는 dirty checking. follow-up 이벤트는 HandlerResult 반환으로 전달 (BFS 큐잉).
- **§2.5 `cfg!(feature = "chat")` → `#[cfg(feature = "chat")]`** — 런타임 bool이 아니라 컴파일 타임 어노테이션이어야 ReflectionService 타입 컴파일 제외 가능.
- **§2.3 prompt builder의 `taboo`/`life_question` 처리** — F8.4가 발견한 갭. relationships.md §6.2 LLM 입력 명세는 **compass + taboo + life_question + 현재 PAD** 4종을 모두 요구. 그러나 Phase 1 A-min은 compass만 도메인에 추가 (F4). **Phase 1 prompt builder는 taboo/life_question을 None placeholder 또는 prompt에서 제외**. Phase 3c에서 `InnerCompass` struct 승격 시 자동으로 활성화. 현재 PAD는 turn_buffers의 `TurnSnapshot.pad_after`에서 회수 — `compute_significance`도 같은 데이터 사용하므로 추가 비용 0.

#### F7. spec §10.11 Director 경로 — Phase 1에서도 grep 필수

spec §10.11은 Director.end_scene을 Phase 1.5로 미룸. 그러나 `EventPayload::DialogueEndRequested.reflection` 필드 추가 시 **모든 호출자에 `reflection: None` 명시 필수** — 컴파일 에러 회피. Director는 `Command::EndDialogue` dispatch하므로 [src/application/director/](src/application/director/) 디렉토리 grep + `reflection: None` 추가 작업이 Stage 1 마지막 cleanup에 포함.

#### F8. 추가 spot-read 결과 (2026-05-10 ✅ 모두 완료)

본 sub-section은 *원래* "구현 진입 전 권장" 체크리스트였으나, F11 baseline 측정 후 6개 spot-read 모두 완료. 결과 박제:

##### F8.1 ✅ `src/adapter/rig_chat.rs` (Tier B #1 직접 검증)

**구조**:
```rust
pub struct RigChatAdapter {
    sessions: Arc<RwLock<HashMap<String, ChatSession>>>,
    // ...
}

struct ChatSession {
    system_prompt: String,
    rig_history: Vec<Message>,
    dialogue_history: Vec<DialogueTurn>,
    generation_config: Option<LlmModelInfo>,
}
```

`start_session(&self, sid, ...)` / `send_message(&self, sid, ...)` / `end_session(&self, sid)` 모두 `&self` async + 내부 RwLock으로 동시성 보장.

**평가**: ✅ Phase 1 어댑터 (`ConversationBackedReflectionPort<C: ConversationPort>`)가 같은 RigChatAdapter 인스턴스로 dialogue + reflection 세션 동시 보유 가능. spec §2.2의 패턴 그대로 작동.

##### F8.2 ✅ `Cargo.toml` chat feature 섹션 (F1 #7 관련)

```toml
chat = ["dep:rig-core", "dep:async-trait", "tokio/rt-multi-thread", "tokio/macros", "dep:async-stream", "dep:reqwest", "dep:bytes"]
```

- `async-trait` ✅ chat에 이미 포함 — Phase 1 `ReflectionPort` trait의 `#[async_trait]` 사용 OK
- `uuid` ❌ chat 단독에서 *미가용* (embed/mind-studio feature gated, line 42). **결정**: reflection_sid는 `format!("reflection-{epoch_ms}-{counter}")` 또는 atomic counter 우회 (chat에 uuid 추가 안 함 — Phase 1 의존성 0건 유지 원칙)

##### F8.3 ✅ `src/application/director/` 디렉토리 (F7 grep 대상)

`Command::EndDialogue` literal 발생 위치 — Phase 1 Stage 1 마지막 cleanup에 `reflection: None` 추가 필수:

| 파일:line | 컨텍스트 |
|---|---|
| [src/application/director/mod.rs:187](src/application/director/mod.rs:187) | `Director::end_scene` 메서드 — `tx.send(Command::EndDialogue { npc_id, partner_id, significance })` |
| [tests/dispatch_v2_test.rs:494](tests/dispatch_v2_test.rs:494) | dispatch_v2 통합 테스트 |
| [tests/memory_consolidation_test.rs:40](tests/memory_consolidation_test.rs:40) | Memory Step D consolidation 테스트 |
| [tests/memory_relationship_cause_test.rs:98](tests/memory_relationship_cause_test.rs:98) | RelationshipChangeCause 테스트 |

→ **총 4개 callsite**. 컴파일 회피는 `reflection: None` 한 줄 추가로 충분. 본격 reflection 통합은 Phase 1.5 (spec §10.11 그대로).

##### F8.4 ✅ `relationships.md` v0.7 §6 본문 — 가드레일 5조건

**§6.4 가드레일 정확 문구** (lines 816~839):
```
Outer Loop 진입:
  significance >= 0.3
  OR  declarative_events 비어 있지 않음
  OR  external_events 비어 있지 않음
  OR  temporal_signals 비어 있지 않음

거꾸로: is_chitchat && significance < 0.3 → outer loop skip
```

spec 결정 8 ("significance >= 0.3 OR !is_chitchat OR ...")과 **본질 동등**, 단 표현 형식만 역(skip 조건) vs 양(진입 조건). 실제 분기 결과 동일.

**§6.2 LLM 입력 명세 갭** (lines 648~651) — ⚠️ **신규 발견**:
```
LLM 입력:
  - turn-level OCC 누적 리스트
  - 대화 transcript
  - NPC: compass / taboo / life_question / 현재 PAD       ← spec §2.3 누락
  - 대상 NPC 정보 / 현재 BondKind / axes
```

spec §2.3 `DefaultReflectionPromptBuilder.build()`는 `compass_short_label()`만 호출. **§6.2 명세는 `taboo` + `life_question` + 현재 PAD도 LLM 입력으로 요구**. 그러나 Phase 1 A-min은 compass만 추가 (taboo/life_question은 Phase 3c 이연). → **Phase 1 prompt builder는 taboo/life_question을 None placeholder 또는 제외 처리**. spec §2.3 의사코드 보정 필요 (F12 신규 보정 항목 — 변경 이력 v0.5 참조).

**§6.3 Engine significance 정의** (lines 675~707): 4 신호 + 가중치 (peak_occ 0.40 / pad_magnitude 0.30 / diversity 0.15 / beat_signal 0.15) — spec §1.1 코드 완전 일치 ✅.

##### F8.5 ✅ `dispatcher.rs` `MAX_EVENTS_PER_COMMAND` 21 도달 worst-case (F3 (아) 결정 입력)

```rust
pub const MAX_CASCADE_DEPTH: u32 = 4;          // line 33
pub const MAX_EVENTS_PER_COMMAND: usize = 21;  // line 36
// ... line 530:
if state.staging_buffer.len() >= MAX_EVENTS_PER_COMMAND {
    return Err(DispatchV2Error::EventBudgetExceeded);
}
```

worst-case 경로: `Command::EndDialogue` → `DialogueEndRequested(1) + RelationshipUpdated(2) + EmotionCleared(3) + SceneEnded(4) + (3 inline projection events)` = ~7. 다른 Command (TellInformation 청자 N개 fanout 등)이 더 길 수 있어 21 limit 안전 마진 큼. Phase 1 `DialogueReflected` 추가 시 8 → 22 limit으로 인상 (결정 (아))은 dispatcher.rs **한 줄** 변경 + 테스트 dependency 무 (`tests/dispatch_v2_test.rs:1054` 등은 `> 0` check만).

##### F8.6 ✅ `_schema.md` 본문 — 갭 표 정확화

00-roadmap.md §6.5의 ❓ 7행을 정확한 팩트로 치환 (별도 commit으로 roadmap 갱신):

| 필드 | _schema.md 정의 | 코드 구현 | 갭 | Phase |
|---|---|---|---|---|
| `Npc.id`/`name`/`description`/`personality` (HEXACO 24) | ✅ Layer 1 (identity + temperament) | ✅ NpcJson + 24 facet 완전 | 없음 | 완료 |
| `Npc.inner_compass` | ✅ **복합 객체** `{compass, taboo, life_question, taboo_crystallization}` | ❌ 부재 → Phase 1 A-min `Option<String>` (compass만) | 중 — Phase 1 partial | 1 (A-min) → 후속에서 `Option<InnerCompass>` 승격 forward-compat OK |
| `Relationship` 4축 (trust/affinity/respect/wariness, ±100) | ✅ v0.6 명시 | ⚠️ 3축 (closeness/trust/power, ±1.0) — closeness ≠ affinity 의미론 다름 | 큼 | 2 |
| `BondKind` 11 variants | ✅ v0.6 정의 | ❌ 부재 | 큼 | 2 |
| `BondStatus` 5 / `Partnership` 4 / `type`/`type_history` | ✅ 정의 | ❌ 부재 | 큼 | 2 |
| 행동 관련 | ⚠️ 분리 (`action_triggers.md` v0.1 이관) | ❌ 부재 | 큼 (별도 spec) | 3c |
| `Scene` / `Focus` | ◯ _schema.md 범위 외 | ✅ 완전 구현 (scene.rs + SceneJson) | 없음 | 완료 |

**Phase 1 A-min `Option<String>` forward-compat 평가**: ✅ — 현재 null로 시작 → 후속 phase에서 `Option<InnerCompass>` 객체로 승격 시 JSON serde 호환 유지. 마이그레이션 비용 낮음 (enum variant wrapping).

#### F9. Impact Map

| 영역 | 신규 | 수정 | 영향만 (테스트 fixture) |
|---|---|---|---|
| Domain | `domain/reflection.rs` | `event.rs` (EventKind+payload 확장) · **`personality.rs` (A-min)** | — |
| Ports | `ports/reflection.rs` | `mod.rs` re-export | — |
| Adapter | `adapter/reflection_via_chat.rs` | `mod.rs` re-export · **`memory_repository.rs` (JSON deserialize)** | — |
| Application | `reflection_service.rs` · `dto/reflection.rs` | `mod.rs` · `dto/mod.rs` · `command/types.rs` (조건부) · `policies/relationship_policy.rs` · `dialogue_orchestrator.rs` | — |
| Mind Studio | — | `state.rs` · `domain_sync.rs` · `mcp_server.rs` · `handlers/*` · (선택) `events.rs` | — |
| Tests | `reflection_service_test.rs` · `relationship_policy_phase1_test.rs` · `phase1_{chitchat,daily_training,shanshenmiao}_test.rs` | — | `dispatch_v2_test.rs` · `dialogue_*.rs` · `director_test.rs` (`reflection: None` 추가) |
| Scenarios | `data/scenarios/phase1-validation/{chitchat-passerby,daily-training,lin-chong-shanshenmiao}.json` | — | — |

#### F10. 권장 작업 순서 (Stage 1~5 진입 전)

| 순서 | 작업 | 산출물 |
|---|---|---|
| 0 | 워크트리 prep — 본 spec/KICKOFF/relationships.md를 작업 워크트리에 sync (`87c8b32` rebase 또는 cherry-pick) | 모든 reference doc 가용 |
| 1 | `cargo test --workspace --features chat,embed,listener_perspective` baseline 박제 + 환경 명시 (llama-server 가동 여부) — **✅ 완료 (2026-05-10): 1033 passed / 0 failed / 6 ignored / 289s walltime. F11 참조** | 회귀 검증 base |
| 2 | F8 추가 spot-read 6개 완료 + Findings에 결과 박제 — **✅ 완료 (2026-05-10): F8.1~F8.6 모두 ✅. 신규 발견 1건 (taboo/life_question prompt 처리) F6에 반영** | F1·F4·F7 보강 |
| 3 | F3 결정 (사)·(아) Bekay 확정 — **✅ 완료 (2026-05-10): (사) DialogueOrchestrator 내부 turn_buffers / (아) MAX_EVENTS_PER_COMMAND 22로 인상** | spec §4.4 13 결정 fix |
| 4 | **Stage 1 직전** — `Npc::inner_compass` + `compass_short_label()` 신설 (~30 LoC, **분리 commit**). 기존 NPC 생성 callsite는 `Option` 기본값으로 자동 호환 | A-min 인프라 완료 |
| 5 | Stage 1~5 spec대로. Stage 1 마지막에 모든 dispatch 호출자 (Director 포함) `reflection: None` 추가 | Phase 1 완료 |
| 6 | (신규) Stage 6 — `docs/changes/` changelog + `mind-architecture/00-roadmap.md` Phase 1 완료 표기 + checkpoint report | Phase 1 archive |

#### F11. Baseline 측정 결과 (2026-05-10)

> **목적**: F10 §1 작업 — Phase 1 작업 *전*의 cargo test 통과 카운트 + 시간을 박제. Stage 5 Phase 1 완료 시 회귀 비교용. F1·F4·F7·F11이 모두 ✅ 되면 Stage 1 진입 100% 준비 완료.

**환경**:
- **OS**: Windows 11 Home (10.0.26200)
- **Rust**: stable 1.94.0 / cargo 1.94.0 (x86_64-pc-windows-msvc)
- **Features**: `chat,embed,listener_perspective`
- **CRT**: `CFLAGS=/MD CXXFLAGS=/MD` (Windows ort 호환, CLAUDE.md 빌드 주의사항 §1)
- **Locale**: `NPC_MIND_ANCHOR_LANG=ko`
- **LLM 서버**: llama-server on `127.0.0.1:8081`, model `gemma-4-E4B-it-Q4_K_M.gguf` (7.5B params, 4.96GB) — *baseline 시점 가동 중*
- **Embed 모델**: `bge-m3` ONNX at `C:\Users\bumko\projects\models\bge-m3` (`model_quantized.onnx` ~570MB INT8 + `tokenizer.json` ~17MB)
- **Worktree commit**: `44bd753`
- **빌드 사전 처리**: `cargo clean` (이전 빌드 산출물의 CRT mismatch 회피, 5GB / 5767 files 삭제 후 재빌드)

**측정** (Run 2 — junction 적용 후 통과, 2026-05-10T22:52:54 ~ T22:57:43):

| 메트릭 | 값 |
|---|---|
| **통과 (passed)** | **1033** |
| 실패 (failed) | 0 |
| 무시 (ignored) | 6 |
| 전체 walltime (Run 2 only) | **289.18 s ≈ 4분 49초** |
| 첫 빌드 walltime (Run 1 + 2 합산) | ≈ 11분 (Run 1: 6분 30초 빌드+실행, Run 2: 4분 49초 실행+일부 재컴) |
| Test target 수 | 61 (lib unit + 60 integration/bench) |

**시간 분포 (>5s 테스트, total 289s 중 ~250s)**:
- `padmocha_bench` 등 PAD bench: 6~31s 각각 (총 ~150s)
- `dialogue_orchestrator_*` chat 통합: 21~41s 각각 (총 ~120s)
- 나머지 도메인 단위 테스트: <0.5s 각각 (총 ~10s)

**로그**: [docs/tasks/mind-architecture/baselines/cargo-test-2026-05-10-PASS.log](baselines/cargo-test-2026-05-10-PASS.log) (UTF-8, 91KB) + [README.md](baselines/README.md)

**baseline 측정 중 발견한 환경 이슈** (Phase 1 진입 전 *문서화 필수*):

1. **워크트리 cwd vs `../models/bge-m3` 하드코딩** — `tests/embed_test.rs:25` 등이 *프로젝트 루트 기준* `"../models/bge-m3/..."` 상대 경로 사용. 워크트리 (`.claude/worktrees/<name>/`)에서는 `../models`가 `.claude/worktrees/models`를 가리켜 토크나이저 로드 실패. **회피**: `mklink /J .claude\worktrees\models C:\Users\bumko\projects\models` junction 생성 (1회). **장기 fix 권장**: `NPC_MIND_MODEL_DIR` 환경변수를 우선시하도록 테스트 코드 갱신 (Phase 1 범위 외, 후속 task).
2. **CRT mismatch (`MSVCP140.dll` vs `libcpmt.lib`)** — `embed` feature 활성화 시 ort 정적 링크가 다른 CRT로 빌드된 산출물과 충돌 (`error LNK2005`). **회피**: `cargo clean` + `CFLAGS=/MD CXXFLAGS=/MD` 셸 환경변수 (CLAUDE.md 빌드 주의사항 §1 그대로). 환경변수 미설정 시 동일 에러 재발 — embed 빌드 전 *항상 명시* 필요.
3. **PowerShell `Out-File` 기본 인코딩 UTF-16 LE** — log 파일을 grep으로 파싱 시 매칭 0건. **회피**: `-Encoding utf8` 명시 또는 `iconv -f UTF-16LE -t UTF-8` 후처리. CLAUDE.md PowerShell 노트 §3 그대로.
4. **PowerShell `2>&1` ErrorRecord wrap** — cargo가 `warning:`을 stderr로 출력하면 PowerShell이 NativeCommandError로 wrap 후 `$LASTEXITCODE = 101` 반환 (cargo 자체는 exit 0). 본 baseline의 exit 101은 **false positive** — 통과 카운트는 정확. CLAUDE.md PowerShell 노트 §3 그대로.

**용도**:
- Phase 1 Stage 5 (Bench) 완료 시 같은 환경에서 재실행 → 회귀 0 검증 (target: ≥1033 passed, +Phase 1 신규 ≥10건)
- LLM 의존 dialogue 통합 테스트 시간 (~120s) 분리 측정 권장 (Phase 1 LLM 호출 1회 추가 영향 측정)

**다음 단계** (F10 §2~§4):
- F8 5개 spot-read (`rig_chat.rs` · `Cargo.toml` chat features · `director/` · `relationships.md §6` · `dispatcher.rs MAX_EVENTS_PER_COMMAND` · `_schema.md`)
- F4 A-min 분리 commit (Stage 1 직전, ~30 LoC)
- Stage 1~5 spec대로 진입

---

### Stage 1 — Domain 변경

**목적**: Reflection 인프라의 *결정론 부분* (LLM 무관)을 도메인에 추가.

**범위**: `domain/event.rs`, 신규 `domain/reflection.rs` (Stage 0 결정 (가) 적용).

**위험**: 작음. 기존 도메인 모듈과 *직교*. 기존 테스트 영향 미미.

#### 1.1 `TurnSnapshot` 구조체 + `compute_significance` 함수

위치 (Stage 0 결정에 따라): `src/domain/reflection.rs` 신규 또는 `src/domain/relationship.rs`에 추가.

```rust
//! Scene Boundary Reflection의 *결정론 부분*.
//! LLM 호출은 application layer (`reflection_service.rs`).

use crate::domain::emotion::OccEmotion;
use crate::domain::pad::Pad;

/// 한 dialogue turn의 결정론 신호 누적.
/// DialogueOrchestrator가 매 turn마다 채워 ReflectionService에 전달.
#[derive(Debug, Clone)]
pub struct TurnSnapshot {
    pub user_utterance: String,
    pub npc_response: String,
    pub occ_emotions: Vec<(OccEmotion, f32)>,    // OCC type + intensity
    pub pad_before: Pad,
    pub pad_after: Pad,
    pub beat_changed: bool,
    pub turn_index: u32,
}

/// 대화의 객관적 격동도 점수 (0.0 ~ 1.0).
/// 4가지 신호의 가중 합산 — relationships.md v0.7 §6.3.
///
/// 가중치는 *디자인 파라미터*. 검증 사례로 tuning 예정 (현 기본값으로 시작).
pub fn compute_significance(turns: &[TurnSnapshot]) -> f32 {
    if turns.is_empty() {
        return 0.0;
    }
    
    // (1) Peak OCC intensity — 0.40
    let peak_occ = turns.iter()
        .flat_map(|t| t.occ_emotions.iter().map(|(_, intensity)| *intensity))
        .fold(0.0f32, f32::max);
    
    // (2) PAD trajectory magnitude — 0.30
    //     매 turn 사이 PAD delta 누적 (clamp to 2.0 → /2.0 normalize)
    let pad_magnitude = turns.windows(2)
        .map(|w| {
            let delta = w[1].pad_after - w[0].pad_after;
            delta.magnitude()
        })
        .sum::<f32>()
        .min(2.0) / 2.0;
    
    // (3) OCC diversity — 0.15
    //     distinct OCC type 개수 / 5 (clamp 1.0)
    let diversity = {
        use std::collections::HashSet;
        let distinct: HashSet<_> = turns.iter()
            .flat_map(|t| t.occ_emotions.iter().map(|(kind, _)| *kind))
            .collect();
        (distinct.len() as f32 / 5.0).min(1.0)
    };
    
    // (4) Beat signal — 0.15
    //     Beat 전환 발생 여부 (binary)
    let beat_signal = if turns.iter().any(|t| t.beat_changed) { 1.0 } else { 0.0 };
    
    (peak_occ * 0.40
       + pad_magnitude * 0.30
       + diversity * 0.15
       + beat_signal * 0.15).clamp(0.0, 1.0)
}
```

**구현 노트**:
- `Pad::magnitude()` 함수가 이미 있는지 Stage 0에서 확인. 없으면 `(p.p.powi(2) + p.a.powi(2) + p.d.powi(2)).sqrt()` inline 구현.
- `OccEmotion` 비교/Hash trait 필요 — 이미 `Eq + Hash`인지 확인. 아니면 derive 추가.

#### 1.2 `ReflectionResult` 구조체

같은 모듈에 정의:

```rust
/// LLM의 서사 평가 + 엔진의 정량 점수 합산 결과.
/// `DialogueReflected` event payload + `DialogueEndRequested` payload에 포함.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReflectionResult {
    /// LLM 판정 — 이 대화가 *서사적으로 잉여*인가
    pub is_chitchat: bool,
    
    /// LLM 작성 — 1~2문장 요약
    pub summary: String,
    
    /// 엔진 계산 — 객관적 격동도
    pub significance_score: f32,
    
    /// LLM emit — 선언/의례 사건 (Phase 1엔 항상 빈 vec, Phase 2부터 활용)
    #[serde(default)]
    pub declarative_events: Vec<DeclarativeEventPlaceholder>,
    
    /// LLM emit — Partnership 사건 (Phase 1엔 항상 None, Phase 2부터 활용)
    #[serde(default)]
    pub partnership_event: Option<PartnershipEventPlaceholder>,
    
    /// 디버깅용 — 누적된 turn 개수
    pub turn_count: usize,
    
    /// 디버깅용 — LLM의 reasoning 텍스트 (calibration drift 감지)
    pub llm_reasoning: Option<String>,
}

/// Phase 1 placeholder. Phase 2에서 `relationships.md` v0.7 §6.4 Channel 1 schema로 확장.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeclarativeEventPlaceholder {
    pub kind: String,
    pub target: Option<String>,
    pub text: String,
}

/// Phase 1 placeholder. Phase 2에서 Spouse/Engaged/Lover/Separated enum으로 확장.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PartnershipEventPlaceholder {
    pub kind: String,
    pub reason: String,
}
```

**구현 노트**:
- `serde(default)` — backward compat 보장. Phase 1 응답에 빈 슬롯이지만 시리얼라이즈/디시리얼라이즈 무사.
- Phase 2에서 placeholder 타입을 *진짜 도메인 enum*으로 교체 시 schema 호환성 유지 (필드명만 일치하면 OK).

#### 1.3 `DialogueReflected` EventKind + payload

`src/domain/event.rs` 수정:

```rust
// EventKind enum에 variant 추가
pub enum EventKind {
    // ... 기존 variants ...
    DialogueReflected,    // ★ 신규 — Phase 1
}

// EventPayload enum에 variant 추가
pub enum EventPayload {
    // ... 기존 variants ...
    
    /// Phase 1 신규. Reflection 결과 박제. 항상 발행 (chitchat 케이스 포함).
    /// Subscribers: memory_projector (summary 흡수), audit/calibration tools.
    DialogueReflected {
        npc_id: NpcId,
        partner_id: NpcId,
        scene_id: SceneId,
        result: ReflectionResult,
    },
}
```

**구현 노트**:
- `EventKind` Display/serialization 함수가 있다면 새 variant 추가.
- `EventPayload`의 `aggregate_key()` 함수가 있다면 `DialogueReflected → AggregateKey::Npc(npc_id)` 추가 (관계 영향 아니므로 Npc aggregate).

#### 1.4 `DialogueEndRequested` payload 확장

```rust
// 기존
pub enum EventPayload {
    DialogueEndRequested {
        npc_id: NpcId,
        significance: Option<f32>,    // 기존 — 외부 주입값. Phase 1 후 deprecated.
    },
}

// Phase 1 후
pub enum EventPayload {
    DialogueEndRequested {
        npc_id: NpcId,
        significance: Option<f32>,                   // 호환 유지 (chat feature 비활성 시)
        reflection: Option<ReflectionResult>,        // ★ Phase 1 신규
    },
}
```

→ 두 옵셔널 필드 공존. chat feature 활성 + ReflectionService 거치면 `reflection: Some(_)`. 비활성 또는 호환 caller는 `reflection: None`.

#### 1.5 단계 게이트

- [ ] `cargo check --all-features` 통과
- [ ] `cargo test --workspace --all-features` 통과 (회귀 0)
- [ ] 신규 단위 테스트 작성:
  - `compute_significance_empty_returns_zero`
  - `compute_significance_high_peak_dominates`
  - `compute_significance_pad_trajectory_accumulates`
  - `compute_significance_occ_diversity_capped`
  - `compute_significance_beat_signal_binary`

→ Stage 1 완료 = 도메인이 *그 자체로* 컴파일·테스트 통과. Application layer는 *아직 사용 안 함* (Stage 2에서).

---

### Stage 2 — Application 변경

**목적**: Reflection의 *LLM 부분*을 application layer에 추가. ReflectionPort 추상화 + Phase 1 어댑터 + ReflectionService + RelationshipPolicy 게이트 + DialogueOrchestrator turn buffer.

**범위**: 5개 파일 신규, 2개 파일 수정.

**위험**: 중. `chat` feature 활성 경로에서 LLM 호출 1회 추가. RelationshipPolicy는 *진입 조건만* 변경 (기존 follow-up 로직 보존).

#### 2.1 `src/ports/reflection.rs` 신규 — ReflectionPort 트레이트

```rust
//! Reflection Port — 대화 종료 후 서사 분석 LLM/AI 추상화.
//!
//! 본 트레이트는 *분석 호출*을 추상화. 구현체:
//! - Phase 1: ConversationPort 기반 (같은 모델, 별도 세션)
//! - Phase 2.5+: 전용 모델/엔드포인트 (별도 LLM)
//! - Test: MockReflectionPort
//!
//! ReflectionService는 본 트레이트에만 의존 — OCP.

use async_trait::async_trait;
use thiserror::Error;
use std::time::Duration;

#[cfg(feature = "chat")]
#[derive(Debug, Clone)]
pub struct ReflectionPrompt {
    /// 분석가 페르소나의 system prompt
    pub system_prompt: String,
    /// User-side 메시지 — transcript + JSON 출력 지시
    pub user_message: String,
    /// 세션 ID 힌트 — KV 캐시 슬롯 / 격리에 활용 (옵셔널)
    pub session_hint: Option<String>,
}

#[cfg(feature = "chat")]
#[derive(Debug, Error)]
pub enum ReflectionError {
    #[error("LLM call failed: {0}")]
    LlmFailure(String),
    
    #[error("Timeout after {0:?}")]
    Timeout(Duration),
    
    #[error("Response parse error: {0}")]
    ParseError(String),
}

#[cfg(feature = "chat")]
#[async_trait]
pub trait ReflectionPort: Send + Sync {
    /// LLM에 reflection prompt를 보내 *원본 텍스트 응답* 반환.
    /// JSON 파싱·도메인 변환은 호출자(`ReflectionService`) 책임.
    async fn analyze(&self, prompt: ReflectionPrompt) -> Result<String, ReflectionError>;
}
```

`src/ports/mod.rs`에 한 줄 추가:

```rust
#[cfg(feature = "chat")]
pub mod reflection;
```

#### 2.2 `src/adapter/reflection_via_chat.rs` 신규 — Phase 1 어댑터

```rust
//! Phase 1 — ConversationPort 기반 ReflectionPort 구현.
//!
//! 동일 LLM 서버에 *별도 세션*을 띄움으로써 dialogue 세션의 KV 캐시 보존.
//! 같은 모델, 다른 system prompt, 다른 KV slot.

use crate::ports::chat::{ConversationPort, ConversationError};
use crate::ports::reflection::{ReflectionPort, ReflectionPrompt, ReflectionError};
use async_trait::async_trait;
use std::sync::Arc;

#[cfg(feature = "chat")]
pub struct ConversationBackedReflectionPort<C: ConversationPort> {
    chat: Arc<C>,
}

#[cfg(feature = "chat")]
impl<C: ConversationPort> ConversationBackedReflectionPort<C> {
    pub fn new(chat: Arc<C>) -> Self {
        Self { chat }
    }
}

#[cfg(feature = "chat")]
#[async_trait]
impl<C: ConversationPort + 'static> ReflectionPort for ConversationBackedReflectionPort<C> {
    async fn analyze(&self, prompt: ReflectionPrompt) -> Result<String, ReflectionError> {
        let sid = prompt.session_hint
            .clone()
            .unwrap_or_else(|| format!("reflection-{}", uuid::Uuid::new_v4()));
        
        // 1. 별도 reflection 세션 생성 (system_prompt = 분석가 페르소나)
        self.chat.start_session(&sid, prompt.system_prompt)
            .await
            .map_err(|e| ReflectionError::LlmFailure(e.to_string()))?;
        
        // 2. transcript + 지시 전송
        let response = self.chat.send_message(&sid, &prompt.user_message)
            .await
            .map_err(|e| match e {
                ConversationError::Timeout(d) => ReflectionError::Timeout(d),
                other => ReflectionError::LlmFailure(other.to_string()),
            })?;
        
        // 3. 세션 정리 (best-effort — 실패해도 결과 반환)
        let _ = self.chat.end_session(&sid).await;
        
        Ok(response.text)
    }
}
```

**구현 노트**:
- `uuid` crate 의존성 — 이미 있는지 Cargo.toml 확인 (아마 있음). 없으면 추가.
- `ConversationError`의 `Timeout` variant는 CLAUDE.md 기준 hexagonal refactor에서 추가됨 — 그대로 활용.
- `'static` lifetime bound — Arc로 감싸 Send + Sync 보장.

#### 2.3 `src/application/reflection_service.rs` 신규

```rust
//! ReflectionService — DialogueOrchestrator가 end_session 시 호출.
//! LLM 분석 결과 + 엔진 정량 계산을 결합해 ReflectionResult 산출.
//!
//! OCP: ReflectionPort에만 의존. 구체 어댑터 (`ConversationBackedReflectionPort` 등) 모름.

use crate::domain::reflection::{TurnSnapshot, ReflectionResult, compute_significance};
use crate::domain::Npc;
use crate::ports::reflection::{ReflectionPort, ReflectionPrompt, ReflectionError};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[cfg(feature = "chat")]
pub struct ReflectionService<P: ReflectionPort> {
    port: Arc<P>,
    prompt_builder: Arc<dyn ReflectionPromptBuilder>,
}

#[cfg(feature = "chat")]
impl<P: ReflectionPort + 'static> ReflectionService<P> {
    pub fn new(port: Arc<P>, prompt_builder: Arc<dyn ReflectionPromptBuilder>) -> Self {
        Self { port, prompt_builder }
    }
    
    /// 누적된 turn snapshot + NPC 컨텍스트 → ReflectionResult.
    /// JSON 파싱 실패 / LLM 타임아웃 시 fallback 반환 (게임 진행 보장).
    pub async fn reflect(
        &self,
        sid: &str,
        turns: &[TurnSnapshot],
        npc: &Npc,
        partner: &Npc,
    ) -> ReflectionResult {
        // (1) 엔진 부분 — 결정론, ms
        let significance = compute_significance(turns);
        
        // (2) Prompt 구성
        let prompt = match self.prompt_builder.build(npc, partner, turns, sid) {
            Ok(p) => p,
            Err(e) => return Self::fallback_result(turns, significance, format!("prompt build error: {e}")),
        };
        
        // (3) Port 호출 — 추상화에만 의존
        let raw_text = match self.port.analyze(prompt).await {
            Ok(t) => t,
            Err(e) => return Self::fallback_result(turns, significance, format!("LLM error: {e}")),
        };
        
        // (4) JSON 파싱
        let parsed: ReflectionLlmOutput = match serde_json::from_str(&raw_text) {
            Ok(p) => p,
            Err(e) => return Self::fallback_result(turns, significance, format!("parse error: {e}, raw: {raw_text}")),
        };
        
        // (5) 합치기
        ReflectionResult {
            is_chitchat: parsed.is_chitchat,
            summary: parsed.summary,
            significance_score: significance,
            declarative_events: parsed.declarative_events.unwrap_or_default(),
            partnership_event: parsed.partnership_event,
            turn_count: turns.len(),
            llm_reasoning: parsed.reasoning,
        }
    }
    
    /// JSON 파싱/LLM 실패 시 보수적 fallback.
    /// is_chitchat=false → outer loop 진입 보장 (안전한 기본값).
    fn fallback_result(turns: &[TurnSnapshot], significance: f32, reason: String) -> ReflectionResult {
        ReflectionResult {
            is_chitchat: false,    // 보수적: 의미 있는 사건으로 처리
            summary: "(reflection failed)".into(),
            significance_score: significance,
            declarative_events: vec![],
            partnership_event: None,
            turn_count: turns.len(),
            llm_reasoning: Some(format!("FALLBACK: {reason}")),
        }
    }
}

/// LLM 응답 JSON 스키마 (placeholder 필드 포함).
#[derive(Debug, Deserialize)]
struct ReflectionLlmOutput {
    is_chitchat: bool,
    summary: String,
    #[serde(default)]
    declarative_events: Option<Vec<DeclarativeEventPlaceholder>>,
    #[serde(default)]
    partnership_event: Option<PartnershipEventPlaceholder>,
    #[serde(default)]
    reasoning: Option<String>,
}

/// Prompt 구성 추상화 — 캐릭터별/장르별 customize 가능.
pub trait ReflectionPromptBuilder: Send + Sync {
    fn build(
        &self,
        npc: &Npc,
        partner: &Npc,
        turns: &[TurnSnapshot],
        session_hint: &str,
    ) -> Result<ReflectionPrompt, ReflectionError>;
}

/// 기본 wuxia 분석 prompt — Phase 1 default.
pub struct DefaultReflectionPromptBuilder;

impl ReflectionPromptBuilder for DefaultReflectionPromptBuilder {
    fn build(
        &self,
        npc: &Npc,
        partner: &Npc,
        turns: &[TurnSnapshot],
        session_hint: &str,
    ) -> Result<ReflectionPrompt, ReflectionError> {
        // System prompt: 분석가 페르소나
        let system_prompt = format!(
            "당신은 무협 서사 작가의 편집자입니다. NPC '{}'(은)는 {} 성향이며, \
             '{}'(와)과의 대화를 막 마쳤습니다. 이 대화가 *서사적으로 의미 있는 사건*인지 \
             아니면 *지나가는 잡담*인지 평가하세요. \
             반드시 다음 JSON 형식으로만 답하세요 (다른 텍스트 절대 금지):\n\
             {{\n\
               \"is_chitchat\": bool,\n\
               \"summary\": \"1~2문장 한국어 요약\",\n\
               \"declarative_events\": [],\n\
               \"partnership_event\": null,\n\
               \"reasoning\": \"이 판정의 근거 1~2문장\"\n\
             }}",
            npc.name(),
            npc.compass_short_label(),    // Phase 1: 일단 short label. Phase 2에서 더 풍부하게.
            partner.name(),
        );
        
        // User message: transcript + 지시
        let transcript = format_transcript(turns);
        let user_message = format!(
            "[대화 transcript]\n{transcript}\n\n\
             [지시]\n위 대화를 평가하세요. JSON으로만 답하세요."
        );
        
        Ok(ReflectionPrompt {
            system_prompt,
            user_message,
            session_hint: Some(session_hint.to_string()),
        })
    }
}

fn format_transcript(turns: &[TurnSnapshot]) -> String {
    turns.iter()
        .map(|t| format!("[turn {}]\n  user: {}\n  npc: {}", t.turn_index, t.user_utterance, t.npc_response))
        .collect::<Vec<_>>()
        .join("\n\n")
}
```

**구현 노트**:
- `Npc::compass_short_label()` — 현재 코드에 이런 메서드가 있는지 Stage 0에서 확인. 없으면 다른 방식 (compass enum 직접 인용 등).
- Prompt 한국어 버전 (장르 정합). 다국어 시 locales/ 활용 — Phase 1 범위 외, 일단 한국어 하드코딩.
- JSON 출력 강제 — LLM이 `{` 외 텍스트 prefixing하면 파싱 실패 → fallback. Robustness는 Stage 2 마지막 게이트에서 검증.

#### 2.4 `command/policies/relationship_policy.rs` 변경 — 진입 조건 게이트

기존 `DialogueEndRequested` 핸들에 게이트 추가. *기존 follow-up 로직은 보존*, 발행 여부만 조건부로.

```rust
// 기존 (요지)
match event.payload {
    EventPayload::DialogueEndRequested { npc_id, significance } => {
        // 무조건 3 follow-up 발행:
        ctx.add_event(EventPayload::RelationshipUpdated { ... });
        ctx.add_event(EventPayload::EmotionCleared { ... });
        ctx.add_event(EventPayload::SceneEnded { ... });
    }
}

// Phase 1 후
match event.payload {
    EventPayload::DialogueEndRequested { npc_id, significance, reflection } => {
        // (1) DialogueReflected 항상 발행 — reflection 있으면
        if let Some(refl) = &reflection {
            ctx.add_event(EventPayload::DialogueReflected {
                npc_id,
                partner_id: ...,    // scene context에서 회수
                scene_id: ...,
                result: refl.clone(),
            });
        }
        
        // (2) Outer Loop 진입 게이트 평가
        let enter_outer = compute_outer_loop_entry(&reflection, significance);
        
        if enter_outer {
            ctx.add_event(EventPayload::RelationshipUpdated { ... });    // 조건부
        }
        
        // (3) EmotionCleared / SceneEnded 항상 발행
        ctx.add_event(EventPayload::EmotionCleared { ... });
        ctx.add_event(EventPayload::SceneEnded { ... });
    }
}

/// 게이트 평가 — `relationships.md` v0.7 §6.4 가드레일.
fn compute_outer_loop_entry(
    reflection: &Option<ReflectionResult>,
    legacy_significance: Option<f32>,
) -> bool {
    match reflection {
        // chat feature 활성 + ReflectionService 거친 경우
        Some(refl) => {
            refl.significance_score >= 0.3
                || !refl.is_chitchat
                || !refl.declarative_events.is_empty()
                || refl.partnership_event.is_some()
                // external_events / temporal_signals: Phase 3a/3b 입력 (Phase 1엔 비어 있음)
        }
        // chat feature 비활성 또는 호환 caller — 기존 무조건 동작
        None => {
            legacy_significance.is_some()    // 기존 동작: significance가 있으면 outer loop 진입
        }
    }
}
```

**구현 노트**:
- 기존 핸들러 코드 보존 — `RelationshipUpdated` 발행 *내용*은 그대로. 발행 *여부*만 조건부.
- `partner_id` / `scene_id`는 RelationshipPolicy가 어떻게 회수하나? 기존 코드에서 `ctx`나 NPC repository에서 읽는 방식 *그대로 사용*. Stage 0의 spot-read에서 패턴 확인.
- DialogueReflected 발행 순서: *첫 번째*. 이유 — 이후 핸들러가 reflection 정보를 활용할 수 있게 BFS layer 1 시점에 박힘.

#### 2.5 `dialogue_orchestrator.rs` 변경 — turn_buffers + ReflectionService 호출

```rust
// 기존 struct
pub struct DialogueOrchestrator<R, C> {
    dispatcher: Arc<CommandDispatcher<R>>,
    chat: Arc<C>,
    formatter: Arc<dyn GuideFormatter>,
    // ... 기타 필드
}

// Phase 1 후 — ★ 신규 필드 + ReflectionService 의존성
pub struct DialogueOrchestrator<R, C, P>
where
    P: ReflectionPort + 'static,
{
    dispatcher: Arc<CommandDispatcher<R>>,
    chat: Arc<C>,
    formatter: Arc<dyn GuideFormatter>,
    // ... 기존 필드
    
    // ★ 신규 — 세션별 turn snapshot 누적
    turn_buffers: tokio::sync::Mutex<HashMap<SessionId, Vec<TurnSnapshot>>>,
    
    // ★ 신규 — Reflection 호출
    reflection_service: Arc<ReflectionService<P>>,
}
```

`turn()` 메서드 변경:

```rust
pub async fn turn(&self, sid: &str, utterance: &str, pad: Option<Pad>, sit_desc: Option<String>) 
    -> Result<ChatResponse, ...> 
{
    // 1. 기존 흐름 — Command::ApplyStimulus
    let stimulus_output = self.dispatcher.dispatch_v2(
        Command::ApplyStimulus { ... }
    ).await?;
    
    // 2. Beat 전환 시 update_system_prompt
    if has_beat_transition(&stimulus_output.events) {
        self.chat.update_system_prompt(sid, ...).await?;
    }
    
    // 3. LLM 응답 받기
    let chat_response = self.chat.send_message(sid, utterance).await?;
    
    // 4. ★ 신규 — TurnSnapshot 누적
    let snapshot = TurnSnapshot {
        user_utterance: utterance.to_string(),
        npc_response: chat_response.text.clone(),
        occ_emotions: extract_emotions(&stimulus_output.events),
        pad_before: extract_pad_before(&stimulus_output),
        pad_after: extract_pad_after(&stimulus_output),
        beat_changed: has_beat_transition(&stimulus_output.events),
        turn_index: self.next_turn_index(sid).await,
    };
    self.turn_buffers.lock().await
        .entry(sid.into())
        .or_default()
        .push(snapshot);
    
    Ok(chat_response)
}
```

`end_session()` 메서드 변경:

```rust
pub async fn end_session(
    &self,
    sid: &str,
    significance: Option<f32>,    // 호환 유지 (caller가 명시적 significance 주입 가능)
) -> Result<AfterDialogueResponse, ...> {
    // 1. ★ 신규 — Reflection 호출 (LLM 비동기, 수 초)
    let reflection = if cfg!(feature = "chat") {
        let turns = self.turn_buffers.lock().await
            .remove(sid)
            .unwrap_or_default();
        
        let (npc, partner) = self.lookup_session_npcs(sid).await?;
        
        let result = self.reflection_service.reflect(sid, &turns, &npc, &partner).await;
        Some(result)
    } else {
        None
    };
    
    // 2. ConversationPort 세션 종료 (기존)
    self.chat.end_session(sid).await?;
    
    // 3. dispatch_v2 호출 (Phase 1 — reflection을 payload로 박음)
    let dispatch_output = self.dispatcher.dispatch_v2(
        Command::EndDialogue {
            npc_id: ...,
            significance,                  // 호환 유지
            reflection: reflection.clone(),    // ★ 신규
        }
    ).await?;
    
    // 4. 응답 DTO 구성 — reflection 노출
    Ok(AfterDialogueResponse {
        events: dispatch_output.events,
        reflection,                          // ★ 신규
        // ...
    })
}
```

**구현 노트**:
- `cfg!(feature = "chat")` 가드 — 비활성 시 reflection: None. 단 본 메서드 자체가 chat feature gated이면 가드 생략 가능. 빌드 매트릭스 (`--no-default-features` 등) 정확한 처리는 Stage 0 결정.
- `lookup_session_npcs(sid)` — 기존 패턴 (DialogueOrchestrator가 세션→NPC 매핑 관리하는지) Stage 0에서 확인. 없으면 별도 helper 추가.
- Mutex 락 시간 *최소*: turn_buffer remove 즉시 unlock. Reflection 호출은 락 *밖에서*.

#### 2.6 빌더 패턴 — DI 설정

`MindStudio` 또는 라이브러리 사용자 코드에서 DI:

```rust
// chat feature 활성 시
let chat = Arc::new(RigChatAdapter::new(...));
let reflection_port = Arc::new(
    ConversationBackedReflectionPort::new(chat.clone())
);
let reflection_service = Arc::new(
    ReflectionService::new(
        reflection_port,
        Arc::new(DefaultReflectionPromptBuilder),
    )
);
let orchestrator = DialogueOrchestrator::new(dispatcher, chat, formatter)
    .with_reflection(reflection_service);    // ★ 신규 빌더 메서드
```

`DialogueOrchestrator`에 `.with_reflection(svc)` 빌더 추가. chat feature 비활성 시 이 메서드 없음 (코드 컴파일 제외).

#### 2.7 단계 게이트

- [ ] `cargo check --features chat` 통과
- [ ] `cargo test --features chat` 회귀 0
- [ ] `cargo build --no-default-features` 통과 (chat feature 없을 때 ReflectionService 코드 컴파일 제외 검증)
- [ ] 신규 단위 테스트:
  - `reflection_service_chitchat_returns_chitchat_result` (MockReflectionPort)
  - `reflection_service_significant_event_returns_full_result` (MockReflectionPort)
  - `reflection_service_invalid_json_returns_fallback` (MockReflectionPort, broken JSON)
  - `reflection_service_llm_timeout_returns_fallback` (MockReflectionPort, Timeout error)
  - `relationship_policy_skips_outer_loop_on_chitchat` — DialogueEndRequested with chitchat reflection → no RelationshipUpdated emit
  - `relationship_policy_enters_outer_loop_on_significant` — full reflection → RelationshipUpdated emit
  - `relationship_policy_legacy_caller_no_reflection` — None reflection + legacy significance → 기존 동작 (호환)
- [ ] grep 검증:
  - `findstr /S /I "ConversationBackedReflectionPort" src\application\` → 0건 (OCP 검증)
  - `findstr /S /I "ReflectionPort" src\application\reflection_service.rs` → import만, 구체 어댑터 0건

→ Stage 2 완료 = ReflectionService + RelationshipPolicy 게이트가 *통합 동작*. 단 외부 면 (DTO/REST/MCP)에는 아직 노출 안 됨.

---

### Stage 3 — 외부 면 노출 (DTO + domain_sync + MCP + REST)

**목적**: ReflectionResult를 외부 caller (Mind Studio frontend, MCP 사용자, 라이브러리 임베더)가 접근 가능하도록.

**범위**: 5개 파일 수정.

**위험**: 작음. 모두 *additive*. 기존 응답에 옵셔널 필드 추가만.

#### 3.1 `application/dto/` — AfterDialogueResponse 확장

위치 (Stage 0 결정 (나)):
- (a) `application/dto/scene.rs`에 추가 (AfterDialogueResponse가 거기 있다면)
- (b) `application/dto/reflection.rs` 신규 — 별도 모듈

권장 (b). Phase 2에서 ReflectionResult 슬롯 확장 예상 — 별도 모듈이 깔끔.

```rust
// src/application/dto/reflection.rs (or scene.rs에 추가)
#[cfg(feature = "chat")]
use crate::domain::reflection::ReflectionResult;

#[derive(Debug, Clone, serde::Serialize)]
pub struct AfterDialogueResponse {
    // ... 기존 필드
    
    /// Phase 1 신규. chat feature 비활성 시 None.
    #[cfg(feature = "chat")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reflection: Option<ReflectionResult>,
}
```

`application/dto/mod.rs`의 re-export 갱신.

#### 3.2 `bin/mind-studio/domain_sync.rs` — dispatch_end_dialogue 확장

```rust
// 기존 시그니처 (대략)
pub async fn dispatch_end_dialogue(
    state: &AppState,
    inner: &mut StateInner,
    req: AfterDialogueRequest,
) -> Result<AfterDialogueResponse, ...> {
    // Command::EndDialogue dispatch_v2 호출
    // 응답 DTO 구성
}

// Phase 1 후
pub async fn dispatch_end_dialogue(
    state: &AppState,
    inner: &mut StateInner,
    req: AfterDialogueRequest,
) -> Result<AfterDialogueResponse, ...> {
    // 1. ReflectionService 호출 (chat feature 활성 시)
    let reflection = #[cfg(feature = "chat")] {
        if let Some(reflection_service) = &state.reflection_service {
            let turns = state.turn_buffers.lock().await.remove(&req.sid).unwrap_or_default();
            let (npc, partner) = lookup_npcs_by_sid(...);
            Some(reflection_service.reflect(&req.sid, &turns, &npc, &partner).await)
        } else {
            None
        }
    };
    
    // 2. dispatch_v2 호출 (reflection을 payload로)
    let output = state.shared_dispatcher.dispatch_v2(
        Command::EndDialogue {
            npc_id: ...,
            significance: req.significance,
            reflection: reflection.clone(),
        }
    ).await?;
    
    // 3. 응답 DTO
    Ok(AfterDialogueResponse {
        // ... 기존 필드
        #[cfg(feature = "chat")]
        reflection,
    })
}
```

**구현 노트**:
- DialogueOrchestrator를 통해 호출되는 경로와 *직접* dispatch_end_dialogue 호출 경로 *둘 다* reflection 거치게 통일. 두 경로가 *서로 다른 동작*하면 안 됨.
- AppState에 `reflection_service: Option<Arc<ReflectionService<...>>>` 신설. 없으면 reflection: None (호환).
- AppState에 `turn_buffers` 신설 — DialogueOrchestrator의 인스턴스를 공유하든, AppState가 별도 관리하든. Stage 0의 spot-read에서 패턴 확인.

#### 3.3 `bin/mind-studio/mcp_server.rs` — dialogue_end MCP tool 응답 확장

```rust
// dialogue_end tool 응답 schema에 reflection 필드 추가
{
  "events": [...],
  "reflection": {                              ★ Phase 1 신규
    "is_chitchat": false,
    "summary": "임충이 산신묘에서 결단을 내렸다",
    "significance_score": 0.87,
    "declarative_events": [],
    "partnership_event": null,
    "turn_count": 12,
    "llm_reasoning": "OCC peak 0.95, PAD trajectory 1.4 — 격렬한 결단 사건"
  }
}
```

→ MCP tool description에 `reflection` 필드 명세 추가. 사용자 (또는 Claude/Claude Code 디버깅 시) reflection 결과 직접 조회 가능.

#### 3.4 `bin/mind-studio/handlers/` — REST handler 응답 확장

기존 dialogue 종료 REST endpoint (예: `POST /api/dialogue/end`)의 응답 JSON에 `reflection` 필드 추가. additive — 기존 frontend는 무시 가능.

#### 3.5 `bin/mind-studio/events.rs` — StateEvent 확장 (선택)

Frontend가 reflection 결과를 *실시간으로* 받고 싶다면 (Phase 1.5 frontend task의 사전 작업):

```rust
// StateEvent enum에 variant 추가 (Phase 1엔 backend만)
pub enum StateEvent {
    // ... 기존 variants
    DialogueReflected { sid: String, reflection: ReflectionResult },    // ★ Phase 1 (선택)
}
```

→ EventBus 구독 후 SSE 송신 핸들러에서 `EventPayload::DialogueReflected` → `StateEvent::DialogueReflected` 변환. Phase 1.5에서 frontend가 이를 구독.

본 phase에선 *백엔드 enum만 추가*. SSE 송신 코드는 추가해도 frontend가 무시. Phase 1.5에서 frontend ReflectionPanel 추가 시 활용.

#### 3.6 단계 게이트

- [ ] `cargo build --features chat,embed` 통과 (Mind Studio 전체)
- [ ] `cargo test` 회귀 0 + REST handler 통합 테스트 신규 1~2개
- [ ] MCP tool 수동 호출 — `dialogue_end` 응답에 reflection 포함 확인
- [ ] REST 수동 호출 — 같은 검증

→ Stage 3 완료 = 외부 caller가 reflection 결과 *접근 가능*. 단 frontend UI는 *없음* (Phase 1.5).

---

### Stage 4 — Narrative Validation

**목적**: significance 게이트가 *실제 무협 서사*에서 *서사적 비중*과 일치하는지 검증. 3 밴드 (낮음/중간/높음) 모두 커버.

**범위**: 시나리오 JSON 3개 추가 (`data/scenarios/phase1-validation/`), 디자이너 (Bekay) 수동 실행 + 결과 검토.

**위험**: 중. *튜닝 필요* 가능성. 게이트 임계값 (0.3) 또는 가중치 (0.40/0.30/0.15/0.15)가 실제 서사와 어긋나면 재조정.

#### 4.1 시나리오 1 — 낮음 (잡담)

**파일**: `data/scenarios/phase1-validation/chitchat-passerby.json`

**상황**: 임충이 길에서 만난 *행인*과 짧은 인사 교환.

**스크립트** (4 turn):
```
turn 1
  passerby: "오늘 날씨가 좋군요."
  npc (임충): "네, 그렇네요."
turn 2
  passerby: "어디로 가시는 길이오?"
  npc (임충): "... 그저 길을 걷고 있소."
turn 3
  passerby: "조심히 가시오."
  npc (임충): "고맙소."
turn 4 (대화 자연 종료)
```

**기대 결과**:
- LLM 판정: `is_chitchat: true`
- compute_significance: `< 0.2` (peak OCC 미미, PAD 변화 없음, OCC diversity 0~1, beat 전환 없음)
- 게이트: `0.15 < 0.3 AND is_chitchat = true` → 게이트 미통과
- 발행 이벤트: `DialogueReflected` (chitchat 박제), `EmotionCleared`, `SceneEnded`. **`RelationshipUpdated` 미발행**.
- axes: 변화 없음
- Memory: summary만 저장 ("길에서 행인과 의례적 인사")

**검증**: 
- 통합 테스트 `tests/phase1_chitchat_test.rs` — 시나리오 실행 후 events 시퀀스 검증
- 수동: Mind Studio에서 시나리오 로드, 대화 진행, axes 변화 0 확인

#### 4.2 시나리오 2 — 중간 (일상)

**파일**: `data/scenarios/phase1-validation/daily-training.json`

**상황**: *수련-춘설병 일상 무공 수련 대화*. 가르침이 있고 감정도 있지만 *transformation 사건*은 아님.

**스크립트** (8 turn 정도):
```
turn 1
  chunxueping: "어제 가르쳐주신 검법, 다시 한번 보여주실 수 있나요?"
  npc (수련): "오늘은 *움직임*이 아닌 *호흡*에 집중하자. ..."
turn 2~6: 가르침 + 시연 + 칭찬 + 격려 (Pride, Admiration 등 발화)
turn 7
  chunxueping: "사부님, 감사합니다. 오늘 많이 배웠어요."
  npc (수련): "내일 다시 보자."
turn 8 (대화 자연 종료)
```

**기대 결과**:
- LLM 판정: `is_chitchat: false` (가르침은 의미 있음)
- LLM `summary`: "수련이 춘설병에게 호흡 중심 검법을 가르침"
- compute_significance: `~0.4~0.6` (peak OCC ~0.5 — Pride/Admiration, PAD 미세, diversity 2~3)
- LLM `declarative_events`: `[]` (Phase 1 placeholder)
- 게이트: `significance >= 0.3 OR !is_chitchat` → 게이트 통과
- 발행 이벤트: `DialogueReflected`, `RelationshipUpdated` (★ 발행), `EmotionCleared`, `SceneEnded`
- axes: 미세 변화 (현 3축 closeness/trust 약간 ↑)
- Memory: summary 저장

**검증**: `tests/phase1_daily_training_test.rs`

#### 4.3 시나리오 3 — 높음 (결단)

**파일**: `data/scenarios/phase1-validation/lin-chong-shanshenmiao.json`

**상황**: 임충이 산신묘에서 *육겸을 발견하고 처단*. 한 장면 안에 *분노 격발 + 결심 + 행위*.

**스크립트** (10 turn 정도):
```
turn 1~3: 폭풍 속 산신묘 도착, 묘 안에서 사람 목소리 발견 (Surprise + Distress)
turn 4~5: 육겸 일행이 자기를 *암살*하러 온 사실을 들음 (Anger 격발, peak OCC ~0.95)
turn 6: 임충이 *나를 죽이려 했다는 걸 똑똑히 들었다*는 결심 (Resolution 격발)
turn 7~8: 처단 행위 (Beat 전환 — Initial → "처단" focus)
turn 9: 처단 후 *고요* (Pride + 공허, PAD 큰 진폭)
turn 10 (자연 종료)
```

**기대 결과**:
- LLM 판정: `is_chitchat: false`
- LLM `summary`: "임충이 산신묘에서 육겸을 처단하고 체제에 등을 돌리는 결단"
- compute_significance: `>= 0.85` (peak OCC ~0.95, PAD 큰 변화 ~1.5/2 → 0.75, diversity 4+, beat_signal 1.0)
- LLM `declarative_events`: `[]` (Phase 1 — Phase 2에서 "처단" 같은 type_transformation 식별)
- 게이트: 모든 조건 통과
- 발행 이벤트: `DialogueReflected` (큰 reasoning + significance), `RelationshipUpdated`, `EmotionCleared`, `SceneEnded`
- axes: 큰 변화 (육겸 관계 trust/affinity 극단 음수)
- Memory: summary + 인용 저장

**검증**: `tests/phase1_shanshenmiao_test.rs`. 기존 임충 시나리오와 비교 — RelationshipUpdated 발행 *내용*은 그대로 (게이트만 추가됨).

#### 4.4 디자이너 검증 체크리스트

3 시나리오 모두 Bekay가 직접 실행:

- [ ] 잡담 (시나리오 1) — Mind Studio에서 로드, dialogue 진행, *axes 변화 0* 시각적 확인. SSE 이벤트에 `RelationshipUpdated` *없음*.
- [ ] 일상 (시나리오 2) — *RelationshipUpdated 발행*, axes 미세 변화. summary 메모리에 저장됨.
- [ ] 결단 (시나리오 3) — 큰 axes 변화. DialogueReflected의 `llm_reasoning` 필드에 LLM이 *왜 이 점수*인지 합리적 설명.
- [ ] LLM이 *invalid JSON* emit하는 케이스 인공 생성 (예: prompt 망가뜨림) — fallback 동작, 게임 진행.
- [ ] 3 시나리오 모두 `significance_score`가 *낮음 (< 0.3) / 중간 (0.3 ~ 0.7) / 높음 (> 0.7)* 밴드에 들어가는지 확인. 안 들어가면 가중치 튜닝 필요.

#### 4.5 단계 게이트

- [ ] 3 시나리오 통합 테스트 모두 통과
- [ ] 수동 검증 체크리스트 모두 OK
- [ ] significance 가중치 *튜닝 결과* 기록 (필요 시 spec §11 위험에 추가, 또는 `compute_significance` 함수 위 주석에 박제)

→ Stage 4 완료 = *서사적 의미*와 *시스템 게이트*가 일치한다는 증거. 디자이너 검증 통과.

---

### Stage 5 — Performance Bench

**목적**: Reflection LLM 호출 1회 추가가 *전체 latency*에 미치는 영향 측정. 회귀 측정.

**범위**: 기존 bench 인프라 활용 + Phase 1 전용 케이스 추가.

**위험**: 작음. 측정만, 시스템 변경 없음.

#### 5.1 측정 항목

| 측정 | 방법 | 목표 |
|---|---|---|
| `compute_significance` 단독 latency | 도메인 함수 unit bench, N=100 | < 1ms (결정론, 작은 함수) |
| ReflectionService.reflect *전체* latency | 통합 bench, MockReflectionPort 사용 | LLM 부분 제외, 측정 가능한 오버헤드 |
| ReflectionService.reflect *실제 LLM* latency | 통합 bench, 실제 RigChatAdapter | gemma-3-12b 기준 ~2~5초 (모델/하드웨어 의존) |
| dispatch_v2(EndDialogue) latency 회귀 | Phase 1 전 vs 후 비교 | tolerance ± 10% (DialogueReflected emit 1회 추가만 해당) |
| 전체 end_session() latency | 통합 bench, 시나리오 1/2/3 실행 | 시나리오 2 (8 turn) 기준: ~3~6초 (LLM + dispatch) |

#### 5.2 회귀 baseline

기존 측정값 (Phase 1 작업 *시작* 시점):
- `dispatch_v2(EndDialogue)` baseline: 측정 후 spec에 박제
- 기존 `tests/dialogue_*` 통과 시간: 측정 후 spec에 박제

Phase 1 완료 시점에 같은 측정값 비교:
- dispatch_v2 회귀 < 10%
- 전체 dialogue test 시간: LLM 호출 1회 추가만큼만 증가

#### 5.3 LLM 호출 비용 분석

llama-server 메트릭 (`/metrics`) 활용:
- prompt token count
- predicted token count
- prompt processing time
- predicted token time
- KV cache hit rate (가능하면)

각 시나리오 당 reflection LLM 호출 1회의 비용:
- 시나리오 1 (잡담, 4 turn): prompt ~500~1000 tokens, predicted ~50~100 tokens → ~1~2초
- 시나리오 2 (일상, 8 turn): prompt ~1500~3000 tokens, predicted ~80~150 tokens → ~2~4초
- 시나리오 3 (결단, 10 turn): prompt ~2000~4000 tokens, predicted ~150~250 tokens → ~3~5초

**검증 항목**:
- KV 캐시 효율 — reflection_prompt가 매번 재처리되는지 (캐시 미스) 확인. 미스면 별도 reflection 세션의 prompt를 *고정*하고 transcript만 user message로 받는 방식이 효과적.
- Reflection 호출이 dialogue 호출 대비 *훨씬 빨라야* 함 — system prompt가 짧고 transcript가 짧으면 (turn 4~10).

#### 5.4 단계 게이트

- [ ] `compute_significance` 단독 bench < 1ms
- [ ] dispatch_v2 회귀 < 10%
- [ ] 시나리오 1/2/3 LLM 호출 latency 측정 + spec에 박제
- [ ] KV 캐시 동작 확인 (가능하면) — Phase 2 prompt 재구성 결정의 입력

→ Stage 5 완료 = 성능 회귀 0 + LLM 호출 비용 *명시적*. 사용자가 dialogue 종료 시 *수 초 대기*가 발생함을 alpha tester에 사전 공지.

---

## 6. 테스트

### 6.1 도메인 단위 테스트 (`compute_significance`)

위치: `src/domain/reflection.rs` (또는 `relationship.rs`) 안 `#[cfg(test)] mod tests`.

```rust
#[test]
fn compute_significance_empty_returns_zero() {
    assert_eq!(compute_significance(&[]), 0.0);
}

#[test]
fn compute_significance_high_peak_dominates() {
    let turns = vec![turn_with_occ_peak(0.95)];
    let s = compute_significance(&turns);
    // 0.95 * 0.40 = 0.38 (다른 신호 0이라 가정)
    assert!(s >= 0.35 && s <= 0.40);
}

#[test]
fn compute_significance_pad_trajectory_accumulates() {
    let turns = vec![
        turn_with_pad((0.0, 0.0, 0.0)),
        turn_with_pad((0.5, 0.5, 0.0)),    // delta magnitude ~0.71
        turn_with_pad((1.0, 1.0, 0.5)),    // delta magnitude ~0.87
    ];
    // total ~1.58 / 2.0 = 0.79, * 0.30 = 0.24
    let s = compute_significance(&turns);
    assert!(s >= 0.20 && s <= 0.30);
}

#[test]
fn compute_significance_occ_diversity_capped() {
    let turns = vec![turn_with_5_distinct_occ()];
    // 5/5 = 1.0, * 0.15 = 0.15
    let s = compute_significance(&turns);
    assert!((s - 0.15).abs() < 0.01);
}

#[test]
fn compute_significance_beat_signal_binary() {
    let with_beat = vec![turn_with_beat_change(true)];
    let without = vec![turn_with_beat_change(false)];
    
    let s_with = compute_significance(&with_beat);
    let s_without = compute_significance(&without);
    
    assert!((s_with - s_without - 0.15).abs() < 0.01);
}

#[test]
fn compute_significance_clamps_to_one() {
    let turns = vec![extreme_turn_all_signals_max()];
    assert!(compute_significance(&turns) <= 1.0);
}
```

### 6.2 ReflectionService 단위 테스트 (Mock 어댑터)

위치: `tests/reflection_service_test.rs`.

```rust
struct MockReflectionPort {
    canned: String,
    error: Option<ReflectionError>,
}

#[async_trait]
impl ReflectionPort for MockReflectionPort {
    async fn analyze(&self, _: ReflectionPrompt) -> Result<String, ReflectionError> {
        if let Some(e) = &self.error {
            Err(e.clone())
        } else {
            Ok(self.canned.clone())
        }
    }
}

#[tokio::test]
async fn reflection_service_chitchat_branch() {
    let mock = Arc::new(MockReflectionPort {
        canned: r#"{"is_chitchat": true, "summary": "지나가는 인사", "reasoning": "..."}"#.into(),
        error: None,
    });
    let service = ReflectionService::new(mock, Arc::new(DefaultReflectionPromptBuilder));
    
    let result = service.reflect("test", &empty_turns(), &test_npc(), &test_partner()).await;
    
    assert!(result.is_chitchat);
    assert_eq!(result.summary, "지나가는 인사");
    assert!(result.declarative_events.is_empty());
}

#[tokio::test]
async fn reflection_service_significant_event() {
    let mock = Arc::new(MockReflectionPort {
        canned: r#"{"is_chitchat": false, "summary": "결단 사건", "reasoning": "OCC 0.9+"}"#.into(),
        error: None,
    });
    let service = ReflectionService::new(mock, Arc::new(DefaultReflectionPromptBuilder));
    
    let result = service.reflect("test", &high_significance_turns(), &test_npc(), &test_partner()).await;
    
    assert!(!result.is_chitchat);
    assert!(result.significance_score > 0.7);
}

#[tokio::test]
async fn reflection_service_invalid_json_fallback() {
    let mock = Arc::new(MockReflectionPort {
        canned: "not json at all".into(),
        error: None,
    });
    let service = ReflectionService::new(mock, Arc::new(DefaultReflectionPromptBuilder));
    
    let result = service.reflect("test", &test_turns(), &test_npc(), &test_partner()).await;
    
    // fallback: is_chitchat=false (보수적), llm_reasoning에 FALLBACK 표기
    assert!(!result.is_chitchat);
    assert!(result.llm_reasoning.unwrap().contains("FALLBACK"));
}

#[tokio::test]
async fn reflection_service_timeout_fallback() {
    let mock = Arc::new(MockReflectionPort {
        canned: "".into(),
        error: Some(ReflectionError::Timeout(Duration::from_secs(10))),
    });
    let service = ReflectionService::new(mock, Arc::new(DefaultReflectionPromptBuilder));
    
    let result = service.reflect("test", &test_turns(), &test_npc(), &test_partner()).await;
    
    assert!(!result.is_chitchat);    // fallback: outer loop 진입
    assert!(result.llm_reasoning.unwrap().contains("Timeout"));
}
```

### 6.3 RelationshipPolicy 게이트 단위 테스트

위치: `tests/relationship_policy_phase1_test.rs`.

```rust
#[tokio::test]
async fn policy_skips_outer_loop_on_chitchat_low_significance() {
    let event = DialogueEndRequested {
        npc_id: ...,
        significance: None,
        reflection: Some(ReflectionResult {
            is_chitchat: true,
            significance_score: 0.15,
            // ...
        }),
    };
    let output = run_relationship_policy(event).await;
    
    // 발행 이벤트
    assert!(contains_event(&output, EventKind::DialogueReflected));
    assert!(!contains_event(&output, EventKind::RelationshipUpdated));    // ★ 미발행
    assert!(contains_event(&output, EventKind::EmotionCleared));
    assert!(contains_event(&output, EventKind::SceneEnded));
}

#[tokio::test]
async fn policy_enters_outer_loop_on_significant_event() {
    let event = DialogueEndRequested {
        reflection: Some(ReflectionResult {
            is_chitchat: false,
            significance_score: 0.85,
            // ...
        }),
        ..
    };
    let output = run_relationship_policy(event).await;
    
    assert!(contains_event(&output, EventKind::DialogueReflected));
    assert!(contains_event(&output, EventKind::RelationshipUpdated));    // ★ 발행
    assert!(contains_event(&output, EventKind::EmotionCleared));
    assert!(contains_event(&output, EventKind::SceneEnded));
}

#[tokio::test]
async fn policy_legacy_caller_no_reflection_uses_significance() {
    let event = DialogueEndRequested {
        npc_id: ...,
        significance: Some(0.5),    // legacy caller
        reflection: None,            // chat feature 비활성
    };
    let output = run_relationship_policy(event).await;
    
    // 호환: 기존 무조건 동작 (significance 있으면 outer loop)
    assert!(!contains_event(&output, EventKind::DialogueReflected));    // reflection 없으니 미발행
    assert!(contains_event(&output, EventKind::RelationshipUpdated));    // ★ 기존 동작
    assert!(contains_event(&output, EventKind::EmotionCleared));
    assert!(contains_event(&output, EventKind::SceneEnded));
}
```

### 6.4 Narrative integration 테스트

위치: `tests/phase1_chitchat_test.rs`, `tests/phase1_daily_training_test.rs`, `tests/phase1_shanshenmiao_test.rs`.

각 테스트:
1. 시나리오 JSON 로드
2. DialogueOrchestrator로 대화 진행 (mock LLM 또는 실제 LLM)
3. end_session 후 events 시퀀스 검증
4. axes 변화 검증 (잡담은 0, 일상은 미세, 결단은 큼)

**Mock LLM 사용 vs 실제 LLM 사용 결정**:
- *Mock LLM*: 빠름, 결정론, CI에서 항상 동작. 단 LLM의 *실제 판정 능력*은 검증 안 됨.
- *실제 LLM*: 진짜 검증. 단 CI에서 실행 어려움 (llama-server 필요).

→ 권장: **Mock으로 흐름 검증** + **실제 LLM은 디자이너 수동 검증** (Stage 4.4).

### 6.5 회귀 테스트

기존 테스트 모두 통과해야 함:
- `tests/dispatch_v2_test.rs`
- `tests/dialogue_*` (모든 dialogue 관련 테스트)
- `tests/director_test.rs`
- 기타 기존 테스트

회귀 검증: Phase 1 작업 *시작 시점*에 `cargo test --workspace --all-features` 실행 → 통과 카운트 박제. 작업 *완료 시점*에 같은 명령 실행 → 같거나 더 많이 통과.

### 6.6 grep 검증

PR 전 마지막 자동 검증:

```bash
# OCP 위반 검사 — application/이 구체 어댑터를 직접 import하면 안 됨
findstr /S /I "ConversationBackedReflectionPort" src\application\
# 결과: 0건이어야 함

# ReflectionService가 추상화에만 의존하는지
findstr /S /I "use crate::adapter" src\application\reflection_service.rs
# 결과: 0건이어야 함 (추상화 import만)

# DialogueReflected EventKind가 정확히 정의됐는지
findstr /S /I "DialogueReflected" src\domain\event.rs
# 결과: variant 정의 + payload definition

# DialogueReflected가 RelationshipPolicy에서 정확히 발행되는지
findstr /S /I "DialogueReflected" src\application\command\policies\relationship_policy.rs
# 결과: emit 코드 1+

# Phase 1 placeholder가 정확히 빈 vec / None으로 emit되는지 (코드 검토)
findstr /S /I "declarative_events" src\application\reflection_service.rs
# 결과: 빈 vec 또는 unwrap_or_default()
```

---

## 7. 점진적 도입 순서

본 phase는 *6 stage 순차 진행*. 각 stage가 *그 자체로* compile + test 통과해야 다음 stage 진입.

### 7.1 권장 순서

| 순서 | Stage | 게이트 |
|---|---|---|
| 1 | Stage 0 — Pre-flight Impact Analysis | spec §"Stage 0 Findings" 추가, 결정 (가)~(마) 확정 |
| 2 | Stage 1 — Domain (TurnSnapshot, compute_significance, EventKind 추가) | `cargo check --all-features` 통과, 도메인 unit test ≥ 5개 |
| 3 | Stage 2.1~2.3 — Ports + Adapter + ReflectionService (LLM 부분, RelationshipPolicy *변경 없이*) | `cargo build --features chat` 통과, ReflectionService unit test (mock 어댑터) ≥ 4개 |
| 4 | Stage 2.4~2.5 — RelationshipPolicy 게이트 + DialogueOrchestrator turn buffer | RelationshipPolicy 단위 test ≥ 3개, 기존 dialogue test 회귀 0 |
| 5 | Stage 2.6 — DI 빌더 — `with_reflection` | Mind Studio 빌드 통과 (`cargo run --features mind-studio,chat,embed`) |
| 6 | Stage 3 — DTO + domain_sync + MCP + REST | REST/MCP 수동 호출로 reflection 필드 응답 확인 |
| 7 | Stage 4 — Narrative validation 3 시나리오 | 시나리오 통합 테스트 3개 통과 + 디자이너 수동 검증 OK |
| 8 | Stage 5 — Bench | latency 측정 + 회귀 < 10% |

### 7.2 단계별 *빌드 가능 상태* 보장

각 stage가 끝나면 *그 시점에 빌드·테스트가 통과*해야 함. 다음 stage가 *깨지지 않은 상태*에서 시작.

**예외**: Stage 1과 Stage 2.1~2.3 사이 — Stage 1에서 `DialogueEndRequested.reflection` 필드 추가하면 *기존 호출자*가 깨짐 (필드 미제공). Stage 2.4에서 RelationshipPolicy가 그 필드를 처리하기 *전*에는 모든 dispatch 호출자에 `reflection: None` 명시 필요.

→ Stage 1 마지막에 *모든 기존 호출자 (orchestrator/domain_sync/tests)에 `reflection: None` 추가*. Stage 2.5에서 reflection 실제 값 채우기로 교체.

이렇게 분리하면 Stage 1과 Stage 2.4 사이가 *컴파일 통과*하고 *기존 동작 호환*.

### 7.3 작업 분담 (Tier 정책)

본 spec은 *Tier A·B*까지 (요구사항·상위 설계·spec 작성·핵심 코드 spot-check). 실제 구현은 *Tier C* (Claude Code 또는 Bekay 직접):

| Stage | Tier A·B (spec 저자) 책임 | Tier C (구현자) 책임 |
|---|---|---|
| 0 | grep 패턴 정의, 결정 항목 명시 | grep 실행, spot-read, 결정 확정 → spec 갱신 |
| 1 | 코드 예제 (도메인) 제시 | 실제 컴파일 + 단위 test |
| 2 | 아키텍처 패턴 (OCP, mock test) 제시 | 실제 구현 + 통합 test |
| 3 | DTO/MCP/REST 변경 위치 | 실제 변경 + 통합 test |
| 4 | 시나리오 JSON 가이드 + 기대 결과 | 시나리오 작성 + 디자이너 검증 |
| 5 | 측정 항목 + baseline 정의 | 측정 실행 + spec에 결과 박제 |

---

## 8. 체크리스트 (PR 전)

### 코드

#### Domain
- [ ] `src/domain/reflection.rs` (또는 `relationship.rs` 확장) — `TurnSnapshot` struct
- [ ] 같은 위치 — `compute_significance(turns: &[TurnSnapshot]) -> f32`
- [ ] 같은 위치 — `ReflectionResult` struct + placeholder 타입 2개
- [ ] `src/domain/event.rs` — `EventKind::DialogueReflected` variant 추가
- [ ] `src/domain/event.rs` — `EventPayload::DialogueReflected { ... }` variant 추가
- [ ] `src/domain/event.rs` — `EventPayload::DialogueEndRequested`에 `reflection: Option<ReflectionResult>` 필드 추가
- [ ] 모든 EventKind 매칭 (Display, serialize 등)이 새 variant 처리

#### Ports
- [ ] `src/ports/reflection.rs` 신규 — `ReflectionPort` trait + `ReflectionPrompt` + `ReflectionError`
- [ ] `src/ports/mod.rs` — `pub mod reflection;` 추가

#### Adapter
- [ ] `src/adapter/reflection_via_chat.rs` 신규 — `ConversationBackedReflectionPort<C>`
- [ ] `src/adapter/mod.rs` — re-export 추가

#### Application
- [ ] `src/application/reflection_service.rs` 신규 — `ReflectionService<P>` + `ReflectionPromptBuilder` trait + `DefaultReflectionPromptBuilder`
- [ ] `src/application/mod.rs` — re-export 추가
- [ ] `src/application/command/policies/relationship_policy.rs` — `DialogueEndRequested` 핸들러 게이트 추가, DialogueReflected 발행
- [ ] `src/application/dialogue_orchestrator.rs` — `turn_buffers` 필드, `turn()` 누적, `end_session()` reflection 호출, `with_reflection()` 빌더

#### DTO
- [ ] `src/application/dto/reflection.rs` 신규 (또는 scene.rs 확장) — AfterDialogueResponse 확장
- [ ] `src/application/dto/mod.rs` — re-export 갱신

#### Mind Studio
- [ ] `src/bin/mind-studio/state.rs` — AppState에 reflection_service 필드, turn_buffers 필드
- [ ] `src/bin/mind-studio/domain_sync.rs` — `dispatch_end_dialogue` reflection 호출 + 응답 확장
- [ ] `src/bin/mind-studio/mcp_server.rs` — `dialogue_end` MCP tool 응답 schema 확장
- [ ] `src/bin/mind-studio/handlers/` — 관련 REST handler 응답 확장
- [ ] (선택) `src/bin/mind-studio/events.rs` — StateEvent에 DialogueReflected variant 추가

### 테스트

- [ ] 도메인 단위 test ≥ 6개 (`compute_significance`)
- [ ] ReflectionService 단위 test ≥ 4개 (Mock 어댑터 — chitchat / significant / invalid JSON / timeout)
- [ ] RelationshipPolicy 게이트 test ≥ 3개 (chitchat skip / significant enter / legacy compat)
- [ ] Narrative integration test 3개 (잡담/일상/결단)
- [ ] grep 검증 모두 통과 (§6.6)
- [ ] 회귀 0 (`cargo test --workspace --all-features`)

### 빌드

- [ ] `cargo check --all-features` 통과
- [ ] `cargo build --features chat` 통과
- [ ] `cargo build --no-default-features` 통과
- [ ] `cargo build --features mind-studio,chat,embed --bin npc-mind-studio` 통과
- [ ] `cargo clippy --workspace --all-features -- -D warnings` 경고 없음

### 수동

- [ ] Mind Studio에서 잡담 시나리오 실행 → axes 변화 0
- [ ] Mind Studio에서 결단 시나리오 실행 → axes 큰 변화 + DialogueReflected 박제
- [ ] LLM invalid JSON fallback 동작 확인 (인공 prompt 손상)
- [ ] LLM timeout fallback 동작 확인 (llama-server 강제 중단)

### 문서

- [ ] 본 spec 문서의 "Stage 0 Findings" 섹션 채움
- [ ] `CLAUDE.md` *External Document Index*에 본 task spec 한 줄 추가 — `task-rel-phase1-reflection.md`
- [ ] `mind-architecture/00-roadmap.md` Phase 1 항목에 *완료* 표기 + checkpoint report 링크
- [ ] checkpoint report 작성 — `docs/tasks/mind-architecture/phase1-checkpoint-report.md` 가칭
- [ ] (선택) `docs/changes/` 폴더에 API 변경 로그 추가 (`AfterDialogueResponse` 확장 안내)

---

## 9. 관련 파일 (작업 시 참조 경로)

### 9.1 신규 파일

| 역할 | 경로 | 분량 |
|---|---|---|
| ReflectionPort 트레이트 | `src/ports/reflection.rs` | ~50 LoC |
| Phase 1 어댑터 | `src/adapter/reflection_via_chat.rs` | ~70 LoC |
| ReflectionService + PromptBuilder | `src/application/reflection_service.rs` | ~200 LoC |
| ReflectionResult DTO | `src/application/dto/reflection.rs` (또는 scene.rs 확장) | ~30 LoC |
| 도메인 reflection 모듈 | `src/domain/reflection.rs` (Stage 0 결정 시) | ~150 LoC |
| 시나리오 1 (잡담) | `data/scenarios/phase1-validation/chitchat-passerby.json` | ~80 LoC |
| 시나리오 2 (일상) | `data/scenarios/phase1-validation/daily-training.json` | ~150 LoC |
| 시나리오 3 (결단) | `data/scenarios/phase1-validation/lin-chong-shanshenmiao.json` | ~250 LoC |
| 통합 테스트 1 | `tests/phase1_chitchat_test.rs` | ~80 LoC |
| 통합 테스트 2 | `tests/phase1_daily_training_test.rs` | ~100 LoC |
| 통합 테스트 3 | `tests/phase1_shanshenmiao_test.rs` | ~120 LoC |
| 단위 테스트 (ReflectionService) | `tests/reflection_service_test.rs` | ~150 LoC |
| 단위 테스트 (RelationshipPolicy 게이트) | `tests/relationship_policy_phase1_test.rs` | ~100 LoC |

### 9.2 수정 파일

| 역할 | 경로 | 변경 종류 |
|---|---|---|
| EventKind + EventPayload | `src/domain/event.rs` | variant 2개 추가, payload 1개 확장 |
| **A-min: Npc.inner_compass + compass_short_label** | **`src/domain/personality.rs`** | **`inner_compass: Option<String>` 필드 + getter + `compass_short_label() -> Option<&str>` + `NpcBuilder::with_inner_compass()` (~30 LoC, Stage 0 Findings F4)** |
| **A-min: 시나리오 JSON inner_compass deserialize** | **`src/adapter/memory_repository.rs`** | **`from_file/from_json` 경로에 `serde(default)` 호환 — 기존 시나리오 무영향 (Stage 0 Findings F4)** |
| dispatcher 안전한계 (조건부) | `src/application/command/dispatcher.rs` | `MAX_EVENTS_PER_COMMAND = 21` → 22 (결정 13 / Stage 0 Findings F3 (아), §11.7 참조) |
| RelationshipPolicy 핸들러 | `src/application/command/policies/relationship_policy.rs` | DialogueEndRequested 핸들 게이트 추가, DialogueReflected 발행 추가 |
| DialogueOrchestrator | `src/application/dialogue_orchestrator.rs` | turn_buffers 필드 + turn() 누적 + end_session() reflection + with_reflection() 빌더 |
| Ports mod | `src/ports/mod.rs` | `pub mod reflection;` 한 줄 |
| Adapter mod | `src/adapter/mod.rs` | reflection_via_chat re-export |
| Application mod | `src/application/mod.rs` | reflection_service re-export |
| DTO mod | `src/application/dto/mod.rs` | re-export 갱신 |
| Mind Studio AppState | `src/bin/mind-studio/state.rs` | reflection_service + turn_buffers 필드 |
| Mind Studio domain_sync | `src/bin/mind-studio/domain_sync.rs` | dispatch_end_dialogue reflection 호출 + 응답 확장 |
| Mind Studio MCP server | `src/bin/mind-studio/mcp_server.rs` | dialogue_end tool schema 확장 |
| Mind Studio REST handlers | `src/bin/mind-studio/handlers/` | 관련 dialogue 종료 핸들러 응답 확장 |
| Mind Studio events (선택) | `src/bin/mind-studio/events.rs` | StateEvent에 DialogueReflected variant 추가 |

### 9.3 변경 없음 (영향 받지 않는 파일)

확인용 명시 — 본 phase가 *건드리지 않는 곳*:

- `src/domain/pad.rs`, `pad_anchors.rs`, `pad_table.rs` *(`personality.rs`는 A-min으로 §9.2로 이동 — Stage 0 Findings F4)*
- `src/domain/emotion/` 전체
- `src/domain/relationship.rs` (Stage 0 결정 (가)에서 (b) 선택 시 — `domain/reflection.rs` 신설이라 relationship.rs 변경 0)
- `src/domain/memory/`, `src/domain/rumor.rs`
- `src/domain/world/` 전체
- `src/domain/listener_perspective/`
- `src/application/command/uow.rs` — 변경 없음 *(`dispatcher.rs`의 `MAX_EVENTS_PER_COMMAND` 상수는 §9.2로 이동 — 결정 13 / Stage 0 Findings F3 (아))*
- `src/application/command/policies/` 중 relationship_policy 외 (emotion/stimulus/guide/scene/information/rumor/world_overlay)
- `src/application/command/{telling_ingestion,rumor_distribution,world_overlay,scene_consolidation,relationship_memory}_handler.rs` — Inline 핸들러 5종
- `src/application/director/` — Director 경로는 Phase 1.5
- `src/application/{event_bus,event_store,memory_projector,scene_service,situation_service}.rs`
- `src/adapter/` 중 `rig_chat.rs`, `sqlite_*` 등 — *변경 없음* (ConversationPort 시그니처 그대로). *`memory_repository.rs`는 A-min JSON deserialize로 §9.2 이동 — Stage 0 Findings F4*
- `src/ports/` 중 `chat.rs` 자체 — *변경 없음* (트레이트 시그니처 그대로 유지, 새 트레이트 reflection.rs는 별도)
- `src/worldbuilding/`, `src/lore/`
- `src/bin/world_load.rs`, `src/bin/lore_ingest.rs`
- `mind-studio-ui/` 전체 (Phase 1.5)
- `genres/`, `projects/`, `wuxia-core/`

### 9.4 의존성 추가 검토

`Cargo.toml` 변경 가능성:
- `uuid` crate — `ConversationBackedReflectionPort`에서 reflection_sid 생성에 사용. 이미 의존성에 있는지 Stage 0에서 확인.
  - 없으면: `[dependencies] uuid = { version = "1", features = ["v4"] }` 추가
- `serde_json` — `ReflectionService`에서 LLM 응답 파싱. 이미 의존성에 있음 (확실).
- `async-trait` — `ReflectionPort` trait에 `#[async_trait]`. 이미 의존성에 있음 (ConversationPort에서 사용 중).

→ Phase 1은 *새 dependency 0~1개*. uuid 외 추가 없음.

---

## 10. Out of Scope / 후속 작업

본 phase에서 **하지 않는다**:

### 10.1 Frontend ReflectionPanel — Phase 1.5 follow-up

`mind-studio-ui/`에 Reflection 결과 시각화 UI 추가는 Phase 1.5. 본 phase는 *백엔드 응답에 reflection 필드만* 포함. Frontend 무시 가능.

별도 task: `task-rel-phase1.5-frontend-reflection.md` (가칭). 작업:
- `useStateSync` 훅에 `DialogueReflected` 이벤트 구독
- `ReflectionPanel` 컴포넌트 신설 (요약, significance, declarative_events placeholder, reasoning 표시)
- ResultPanel에 reflection 탭 추가
- DialogueReflected 발행 시 자동 갱신

### 10.2 LLM-engine drift dashboard

LLM이 *어떻게 판정했는지* (is_chitchat) vs 엔진이 *어떻게 계산했는지* (significance) 사이의 drift를 추적·시각화하는 도구. 캘리브레이션 디버깅용.

후속 작업 (Phase 1.5 또는 별도):
- DialogueReflected event 누적 → drift 통계
- 상관관계 시각화 (LLM → significance 분포)
- Calibration 임계값 조정 가이드

### 10.3 Async Reflection 실행 (Director Spawner 활용)

end_session이 *즉시 반환*하고 Reflection이 *백그라운드*에서 처리되는 패턴. Phase 1은 *동기*.

후속 phase 가능성:
- Director가 spawned reflection task 관리
- end_session 즉시 반환 → 사용자 화면 즉시 진행
- Reflection 완료 시 EventBus broadcast → frontend 자동 갱신 (eventual consistency)

이 변경은 *DialogueOrchestrator 인터페이스 변경* 가능성 있음 (Future 반환 등). Phase 2 또는 그 이후.

### 10.4 별도 LLM 모델용 ReflectionPort 구현 — Phase 2.5+

`DedicatedReflectionPort` 또는 동등 — 다른 LLM 엔드포인트 (예: Qwen-30B, 더 강한 분석 모델) 사용. 본 phase의 OCP 준수 덕에 *추가만* 하면 됨.

후속 작업:
- 새 어댑터 파일 (`src/adapter/dedicated_reflection.rs`)
- Cargo.toml에 LLM client 의존성 추가
- DI 빌더 변경 (`with_reflection`에 다른 어댑터 주입)
- ReflectionService / DialogueOrchestrator / 도메인 코드 *변경 0*

### 10.5 4-axis 마이그레이션 — Phase 2

`Relationship` 타입을 3축 (closeness/trust/power, ±1.0)에서 4축 (trust/affinity/respect/wariness, ±100) 로 재작성. `relationships.md` v0.7 §1 모델 적용.

별도 task: `task-rel-phase2-fouraxis-bondkind.md`.

### 10.6 BondKind / BondStatus / Partnership / type_history — Phase 2

11 BondKind variants (지기 4 + Companion + Guardian + Mentor + 원수 4), 5 BondStatus (Active/Resolved/Deceased/Dormant/Reactivating), 4 Partnership (Spouse/Engaged/Lover/Separated). 자유 텍스트 `type` + 이력.

별도 task: `task-rel-phase2-fouraxis-bondkind.md` (Phase 2와 함께).

### 10.7 Channel 1 Declarative 처리 — Phase 2

ReflectionService가 emit한 `declarative_events`를 *실제로 검증·적용*. 본 phase는 placeholder (항상 빈 vec).

Phase 2 작업:
- 사회적 일관성 검증 5 카테고리 (A. Structural / B. Precondition / C. BondStatus Block / D. Mutuality / E. Domain Knowledge)
- 적용 모드 4-tier (무설정/모드만/Alternatives/+Hints)
- DeclarativeEvent → BondKind/Partnership/type 갱신

### 10.8 Channel 2 Temporal — Phase 3a

BondKindCandidacy projection. 시간 게이트 카운터 (Guardian 7일 / Mentor 14일 / SwornBrothers·Companion 30일 / 원수 즉시). axes 임계 + 누적 시간 → BondKind 자동 진입/이탈.

별도 task: `task-rel-phase3a-temporal.md`.

### 10.9 Channel 3 External + narrative_origin — Phase 3b

EventPropagator + PropagationRule (사망/처단/결혼/배신). `narrative_origin` EventMetadata 필드. cross-NPC saga.

별도 task: `task-rel-phase3b-external.md`.

### 10.10 ActionTriggerEvaluator + 추모 행동 — Phase 3c

5-dim feasibility (physical/power/social/self/moral) + 29 ActionKind. BondKind → ActionTrigger 룰. 추모 행동 emit.

별도 task: `task-rel-phase3c-actiontrigger.md`.

### 10.11 Director.end_scene 경로의 Reflection — Phase 1.5

CLAUDE.md: `Director.end_scene(scene_id, significance)`이 `Command::EndDialogue` 보내는 별도 진입점. 본 phase는 *DialogueOrchestrator만*.

Phase 1.5 task에서 Director 경로도 동등 reflection 거치게 통일.

---

## 11. 위험 요소

### 11.1 LLM JSON 파싱 실패

**증상**: LLM이 응답에 *JSON 외 텍스트* prefix/suffix를 추가 (예: `"좋습니다, 다음과 같이 분석합니다: {...}"`). serde_json::from_str 파싱 실패.

**완화**:
- `ReflectionService`에 `fallback_result` 구현 (§2.3) — invalid JSON 시 보수적 fallback (is_chitchat=false → outer loop 진입). 게임 진행 막힘 0.
- 더 robust한 파싱 — 응답 텍스트에서 첫 `{` ~ 마지막 `}` 추출. JSON pre/suffix 제거 후 파싱 시도. (Phase 1.5 또는 후속.)
- Stage 4 narrative validation에서 *real LLM*으로 verify — 실제 발생률 측정.

**잔여 위험**: LLM이 *완전히 깨진 출력*을 반복하면 매 dialogue가 fallback으로 outer loop 진입. axes drift 가능. → Phase 1.5에서 calibration drift 모니터링.

### 11.2 LLM 타임아웃 (>10초)

**증상**: llama-server 부하·OOM·KV 캐시 미스 등. ConversationError::Timeout 발생.

**완화**:
- ConversationPort의 `Timeout` variant 처리 (CLAUDE.md hexagonal refactor 후 추가됨)
- ReflectionError::Timeout → fallback_result (is_chitchat=false → outer loop 진입)
- llama-server 모니터링 (`InferenceServerMonitor`) 활용 — slot 상태 추적. 빌드 시점 통합 가능 (선택).

**잔여 위험**: 타임아웃이 잦으면 사용자가 *수 초 대기 후 fallback* 경험. UX 저하. → Stage 5 bench에서 latency 분포 측정 후 임계값 조정.

### 11.3 ConversationPort 다중 세션 동시 처리 가능 여부

**증상**: 같은 RigChatAdapter 인스턴스가 dialogue_sid + reflection_sid를 *동시* 처리할 때 race / state corruption.

**완화**:
- Stage 0 spot-read의 핵심 항목 — RigChatAdapter 세션 관리 패턴 (HashMap?) 확인
- 만약 세션이 HashMap으로 안전하게 관리되면 OK
- 만약 단일 세션 가정 코드라면 *별도 RigChatAdapter 인스턴스* 사용 또는 ReflectionService 호출 시 dialogue 세션 *완전 종료 후* (현재 spec 그대로 — end_session이 dialogue 종료 후 reflection 호출)

**잔여 위험**: Phase 1 spec은 *순차 호출* 가정 (dialogue 종료 → reflection 시작). 동시 호출 케이스는 의도적 회피.

### 11.4 RelationshipPolicy.handle 변경 시 follow-up 발행 순서

**증상**: 기존 RelationshipPolicy가 RelationshipUpdated/EmotionCleared/SceneEnded를 *특정 순서*로 발행. 이 순서가 *다른 핸들러 (memory_projector 등)*가 의존하는 invariant일 수 있음.

**완화**:
- Stage 0 spot-read에서 현재 RelationshipPolicy 코드 직접 확인 — 발행 순서 파악
- Phase 1 변경: *기존 순서 보존* + DialogueReflected를 *맨 앞에* 추가 + RelationshipUpdated 조건부
- 통합 테스트에서 *발행 순서* 검증 (events 시퀀스 비교)

**잔여 위험**: SceneConsolidationHandler가 SceneEnded 받기 *전에* RelationshipUpdated 받는 invariant가 있으면 → 본 phase가 그 순서 보존하므로 OK. 단 invariant 미발견 시 Phase 1.5에서 발견 가능성.

### 11.5 잡담 게이트 calibration 부정확

**증상**: significance 가중치 (0.40/0.30/0.15/0.15) 또는 임계값 (0.3) 또는 LLM is_chitchat 판정이 *실제 무협 서사*와 어긋남. 잡담이 outer loop 진입하거나 의미 있는 사건이 skip됨.

**완화**:
- Stage 4 narrative validation 3 시나리오 — 낮음/중간/높음 밴드 모두 검증
- 디자이너 (Bekay) 수동 검증 — *서사적 직관*과 시스템 게이트 일치 여부
- 필요 시 가중치 / 임계값 *튜닝* — `tuning.rs`에 reflection 파라미터 추가 가능
- LLM prompt 보강 (예: 무협 시간감 명시, "한 행인과의 인사는 chitchat" 같은 in-context 학습)

**잔여 위험**: Phase 1 종료 시점의 calibration이 *완벽하지 않을 수 있음*. Phase 2 시작 시 alpha 사용 데이터로 *다시 튜닝*.

### 11.6 chat feature 비활성 시 breaking change 가능성

**증상**: `--no-default-features`로 빌드한 라이브러리 사용자가 본 phase 후 빌드 실패. `DialogueEndRequested.reflection` 필드 미제공.

**완화**:
- `reflection: Option<ReflectionResult>` — Some/None 모두 허용. 기존 `None` 호출자 호환.
- `#[serde(default)]` — 기존 시리얼라이즈된 이벤트 deserialize 호환.
- chat feature 비활성 빌드 시 ReflectionService 코드 *컴파일 제외*. AppState에 reflection_service: None 가능.
- Stage 2.7의 빌드 게이트 — `cargo build --no-default-features` 통과 필수.

**잔여 위험**: 외부 사용자가 자체 어댑터/테스트로 EndDialogue 직접 dispatch하는 경우, 새 `reflection` 필드 명시 필요. 0.x 버전이라 breaking 허용 가능. CLAUDE.md changelog에 명시.

### 11.7 `MAX_EVENTS_PER_COMMAND = 21` budget 초과 (Stage 0 Findings F3·F5)

**증상**: `EndDialogue` 경로의 worst-case가 현재 21 이벤트 (`MAX_EVENTS_PER_COMMAND = 21`로 설정). `DialogueReflected` 1개 추가 시 22가 되어 dispatcher가 `DispatchV2Error::EventBudgetExceeded`로 실패.

**완화**:
- 결정 13 (아) — `dispatcher.rs`의 `MAX_EVENTS_PER_COMMAND` 상수 22로 인상 (단순)
- 또는 `DialogueReflected`를 Inline phase로 이관 — cascade 깊이 변경 없이 분리. 단 Stage 1~2 spec과 어긋나므로 권장 안 함
- Stage 5 bench에서 worst-case 이벤트 수 *재실측* — 21 외 다른 dispatch (TellInformation 등)도 22 도달하는지 확인. 안전마진 고려 시 24~25 권장 가능

**잔여 위험**: 후속 phase에서 더 많은 follow-up 추가 시 같은 문제 반복. 위험 catalog에 "event budget" 항목 상시 모니터링 필요.

### 11.8 시나리오 JSON `inner_compass` migration (Stage 0 Findings F4)

**증상**: A-min으로 `Npc.inner_compass: Option<String>` 추가. 기존 시나리오 JSON ([data/scenarios/](data/scenarios/) · [data/treasure_island/](data/treasure_island/) · [data/presets/](data/presets/) 등)에는 `inner_compass` 키 부재. deserialize 실패 또는 *모든 NPC compass 미설정*.

**완화**:
- `serde(default)` + `Option<String>` — deserialize 시 키 부재 → `None`. **기존 시나리오 무영향** (compile + runtime 모두 호환).
- 검증 시나리오 3종 (chitchat-passerby/daily-training/lin-chong-shanshenmiao)에는 명시적으로 inner_compass 추가 — Stage 4 narrative validation에서 Reflection prompt가 작동하는지 검증.
- 후속: 모든 wuxia 시나리오에 inner_compass 점진 추가 — Phase 2 디자이너 작업 (별도 task).

**잔여 위험**: chat feature 활성 + reflection 호출 + NPC `inner_compass = None` 시 prompt builder가 빈 라벨 → LLM이 캐릭터 톤 못 잡음. fallback: `Npc::name()`만 사용해도 chitchat 판정은 가능. 검증 시나리오에 inner_compass 명시로 회피.

---

## 변경 이력

| 버전 | 일자 | 변경 |
|---|---|---|
| v0.1 | 2026-05-10 | 초안. relationships.md v0.7 §6 + 00-roadmap.md v0.2 Phase 1 정의 기반. 6 stage (Stage 0 Pre-flight Impact Analysis 포함) + 11개 핵심 결정 + 3 narrative validation 시나리오 + OCP 준수 (`ReflectionPort` trait + `ConversationBackedReflectionPort` 어댑터). |
| v0.2 | 2026-05-10 | **Stage 0 Findings 박제** (10 sub-section F1~F10 — Tier B 7-가정 검증, spec 의사코드 보정 5건, 결정 (사)·(아) 추가 → 13개 결정, A-min `Npc.inner_compass` 선결정 (F4), 위험 §11.7 (event budget) + §11.8 (JSON migration) 추가, 의사코드 §2.4 / §2.5 보정 노트 (F6), F7 Director grep 필수, F8 추가 spot-read 5개, F9 Impact Map, F10 권장 작업 순서 6단계). 연쇄 수정: §4.1 수정 금지 표에서 `personality` 제외 / §9.2 `personality.rs` + `memory_repository.rs` + `dispatcher.rs` 행 추가 / §9.3 해당 항목 이동 표기. |
| v0.3 | 2026-05-10 | **결정 (사)·(아) Bekay 확정**: (사) turn_buffers는 DialogueOrchestrator 내부 (`HashMap<String, Vec<TurnSnapshot>>`, `&mut self` 일관성). (아) `MAX_EVENTS_PER_COMMAND` 21 → 22로 상수 인상 (옵션 (a)). §4.4 결정 12·13 *보류 표기 → 확정 표기*. F3 본문도 같은 갱신. F10 §3 작업 ✅ 완료 표기. F8에 `_schema.md` spot-read 항목 추가 (00-roadmap.md §6.5 ❓ 행 해소 입력). **Phase 1 Stage 1 인계 100% 준비 완료**. |
| v0.4 | 2026-05-10 | **F11 Baseline 측정 결과 박제** — 1033 passed / 0 failed / 6 ignored / 289s walltime (Run 2, junction 적용 후 통과). 환경 (Rust 1.94.0 / chat,embed,listener_perspective / llama-server gemma-4-E4B + bge-m3 ONNX). baseline 측정 중 발견한 환경 이슈 4건 문서화 (워크트리 cwd vs `../models` 하드코딩 → junction 회피, CRT mismatch → `cargo clean` + `CFLAGS=/MD`, PowerShell UTF-16 default, PowerShell `2>&1` ErrorRecord wrap). 로그 산출물 `baselines/cargo-test-2026-05-10-PASS.log` (91KB) + `baselines/README.md`. F10 §1 ✅ 완료 표기. |
| v0.5 | 2026-05-10 | **F8 spot-read 6항목 모두 ✅ 박제** — F8.1 RigChatAdapter 다중 세션 (`Arc<RwLock<HashMap<String, ChatSession>>>` verified) / F8.2 Cargo.toml chat feature `dep:async-trait` 포함 (uuid는 미포함, reflection_sid는 epoch_ms+counter 우회) / F8.3 Director::end_scene callsite 4곳 catalog (Director mod.rs:187 + 테스트 3곳) / F8.4 relationships.md §6.4 가드레일 본문 vs spec 결정 8 — 본질 동등 (역형식 표현) / F8.5 dispatcher.rs MAX_EVENTS_PER_COMMAND=21 worst-case 7~8 → 22 인상 안전 마진 큼 / F8.6 _schema.md 갭 표 7행 정확화 (00-roadmap.md §6.5 ❓→팩트 별도 commit). **신규 발견 1건**: relationships.md §6.2 LLM 입력 명세가 **taboo/life_question/현재 PAD**도 요구. Phase 1 A-min은 compass만 추가하므로 prompt builder에서 taboo/life_question은 None placeholder 또는 제외 필요 — F6 의사코드 보정 항목 1건 신규 추가. F10 §2 ✅ 완료 표기. **Stage 1 인계 100% 준비 완료** (재확인). |
