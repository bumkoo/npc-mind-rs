# Phase 1 Mind Architecture — API 변경 안내

> Phase 1 (`relationships.md` v0.7 §6 Scene Boundary Reflection) 완료에 따른
> *외부 사용자(라이브러리 임베더, 자체 어댑터/테스트 작성자)*가 알아야 할 변경 모음.
>
> 0.x 버전 — breaking change 허용. 본 문서는 Phase 1 commit `87c8b32` ~ `433f64c`
> 사이의 누적 변화 요약. 상세 spec: [`docs/tasks/mind-architecture/task-rel-phase1-reflection.md`](../tasks/mind-architecture/task-rel-phase1-reflection.md).

## 1. Breaking changes

### 1.1 `Command::EndDialogue`에 `reflection` 필드 추가

```rust
// 이전
Command::EndDialogue {
    npc_id: String,
    partner_id: String,
    significance: Option<f32>,
}

// 이후
Command::EndDialogue {
    npc_id: String,
    partner_id: String,
    significance: Option<f32>,
    reflection: Option<ReflectionResult>,  // ★ 신규
}
```

**마이그레이션**: 기존 호출자에 `reflection: None` 명시 추가.
- `chat` feature 비활성 빌드: 항상 `None` (RelationshipPolicy의 None 분기로 기존 무조건 동작 호환)
- `chat` 활성 + ReflectionService 미부착: 동일하게 `None`
- `chat` 활성 + DialogueOrchestrator의 `with_reflection(svc)` 부착: orchestrator가 자동 채움

`ReflectionResult`는 chat feature 무관 순수 도메인 타입 (`crate::domain::reflection::ReflectionResult`) — 모든 빌드에서 컴파일 가능.

### 1.2 `EventPayload::DialogueEndRequested`에 `reflection` 필드 추가

`Command`와 같은 모양. `dispatch_v2`가 Command → EventPayload 변환 시 그대로 전달.
직접 `EventPayload::DialogueEndRequested { ... }` literal을 만드는 코드는 보정 필요.

### 1.3 `MAX_EVENTS_PER_COMMAND` 21 → 22

[src/application/command/dispatcher.rs:36](../../src/application/command/dispatcher.rs:36) — Phase 1
`DialogueReflected` 추가로 `EndDialogue` worst-case 8 → 9 이벤트 (안전 마진 큼).
의존 테스트 (`tests/memory_telling_test.rs::budget_exhaustion_test_*`)는 `21 → 22`로 갱신됨.

자체 fanout 큰 Command를 작성한 사용자: budget 초과 가능성 재확인.

### 1.4 `RelationshipPolicy.handle_dialogue_end` follow-up 순서 변경

이전: 무조건 3 follow-up (`RelationshipUpdated` → `EmotionCleared` → `SceneEnded`).

이후 (Phase 1 게이트 적용):
1. `DialogueReflected` (reflection.is_some()일 때만, *항상 첫 번째*)
2. `RelationshipUpdated` (게이트 통과 시만 — chitchat은 skip)
3. `EmotionCleared` (항상)
4. `SceneEnded` (항상)

**게이트 조건** (`relationships.md` v0.7 §6.4):
- `reflection: Some(_)`: `significance >= 0.3 OR !is_chitchat OR declarative_events 비어있지 않음 OR partnership_event 있음`
- `reflection: None`: `legacy_significance.is_some()` (기존 호환)

**영향**: chitchat 케이스에는 `RelationshipUpdated` 미발행 → axes 보존. 기존에 *모든*
EndDialogue가 RelationshipUpdated 발행한다고 가정한 외부 핸들러/테스트는 보정 필요.

## 2. 신규 기능

### 2.1 `Npc.inner_compass: Option<String>` (A-min)

[src/domain/personality.rs](../../src/domain/personality.rs) — 캐릭터 가치 한 줄.

```rust
// 빌더 패턴
let npc = NpcBuilder::new("lin_chong", "임충")
    .with_inner_compass("협지대자 위국위민")
    .build();

assert_eq!(npc.inner_compass(), Some("협지대자 위국위민"));
assert_eq!(npc.compass_short_label(), Some("협지대자 위국위민")); // Phase 1 alias
```

시나리오 JSON `npcs.<id>.inner_compass`에 `serde(default)` 호환 — 기존 시나리오는 `None`.

`taboo` / `life_question`은 Phase 3c에서 `InnerCompass` struct로 승격 예정 (forward-compat 보장).

### 2.2 `compute_significance(turns) -> f32` 결정론 함수

[src/domain/reflection.rs](../../src/domain/reflection.rs) — 4 신호 가중 합산:
- peak OCC intensity × 0.40
- PAD trajectory magnitude × 0.30
- OCC type diversity × 0.15
- Beat 전환 binary × 0.15

Per-call **8.36 µs** (Stage 5 bench 측정, 100x 마진).

### 2.3 `ReflectionPort` trait + `ConversationBackedReflectionPort` 어댑터 (chat feature)

[src/ports/reflection.rs](../../src/ports/reflection.rs) + [src/adapter/reflection_via_chat.rs](../../src/adapter/reflection_via_chat.rs).

다른 LLM 모델 사용 시 새 어댑터를 작성해 `ReflectionService`에 주입 — 본체 코드 변경 0 (OCP).

### 2.4 `DialogueOrchestrator::with_reflection(svc)` opt-in 빌더

```rust
let chat = Arc::new(RigChatAdapter::new(...));
let port = Arc::new(ConversationBackedReflectionPort::new(chat.clone()));
let svc = Arc::new(ReflectionService::new(
    port,
    Arc::new(DefaultReflectionPromptBuilder),
));

let orchestrator = DialogueOrchestrator::new(dispatcher, chat, formatter)
    .with_reflection(svc);  // ★ Opt-in
```

부착 시:
- `turn()`이 매 턴 `TurnSnapshot`을 누적
- `end_session()`이 reflection 호출 → `Command::EndDialogue { reflection: Some(_) }`
- 게이트가 chitchat skip

미부착 시 기존 동작 그대로.

### 2.5 `EventPayload::DialogueReflected` event + `EventKind::DialogueReflected`

```rust
EventPayload::DialogueReflected {
    npc_id: String,
    partner_id: String,
    scene_id: SceneId,
    result: ReflectionResult,
}
```

Aggregate: `Npc(npc_id)`. EventBus 구독자가 reflection 결과 audit / Memory 흡수 가능.

### 2.6 `AfterDialogueResponse.reflection: Option<ReflectionResult>`

[src/application/dto/relationship.rs](../../src/application/dto/relationship.rs) —
DTO 응답에 reflection 필드 추가 (`#[serde(skip_serializing_if = "Option::is_none")]`).

REST/MCP 응답이 reflection 객체 또는 누락. Frontend는 무시 가능.

## 3. 환경 / 빌드 노트

### 3.1 Windows UAC installer detection heuristic 회피 (테스트 실행 시)

`dispatch_v2_test.exe` 파일명에 "test" 키워드가 있어 Windows가 *Installer*로
추정 → elevation 요구 (OS error 740). 회피:

```powershell
$env:__COMPAT_LAYER = "RunAsInvoker"
cargo test --features chat,embed,listener_perspective
```

CI/CD 또는 다른 OS 환경에서는 영향 없음. 장기 fix는 `embed-resource` crate로
manifest 첨부 (별도 task — Phase 1 범위 외).

### 3.2 PowerShell `2>&1` ErrorRecord wrap (false positive exit 101)

PowerShell이 cargo의 `warning: ...` stderr를 NativeCommandError로 wrap → exit 101 false
positive. cargo 자체는 exit 0. 통과 카운트는 정확.

### 3.3 워크트리 cwd vs `../models/bge-m3` 하드코딩 (embed feature 테스트)

`tests/embed_test.rs`가 *프로젝트 루트 기준* 상대 경로 사용. git worktree에서는
junction 우회:

```powershell
mklink /J .claude\worktrees\models C:\Users\bumko\projects\models
```

장기 fix: `NPC_MIND_MODEL_DIR` 환경변수 우선 (별도 task).

## 4. 성능 영향

| 항목 | 측정 (Stage 5) |
|---|---|
| `compute_significance(10 turn)` | 8.36 µs/call |
| `dispatch_v2(EndDialogue)` chitchat skip | 24.17 µs (legacy 대비 0.82x) |
| `dispatch_v2(EndDialogue)` significant | 35.03 µs (legacy 대비 1.19x) |
| Reflection LLM 호출 (실측 대기) | ~2~5초 (디자이너 수동 검증) |

→ engine 부분 비용은 *무시 가능*. dialogue 종료 시 사용자 체감 latency는 reflection
LLM 호출 (~수초)이 dominant. Phase 1 alpha tester에 *수 초 대기* 사전 공지 권장.

## 5. 디자인 문서 변경

- `docs/game-design/2-characters/relationships.md` v0.7 §6 — Reflection 설계 (변경 없음, 본 phase의 입력 디자인)
- `docs/game-design/2-characters/_schema.md` — `inner_compass.compass`만 코드 반영 (Layer 2의 4-필드 nested struct는 Phase 3c 승격 예정)
- `docs/tasks/mind-architecture/00-roadmap.md` v0.5 — Phase 1 ✅ 완료 표기 + §6.5 디자인 문서 추적 갱신
- `docs/tasks/mind-architecture/phase1-checkpoint-report.md` — Stage 0~5 종합 체크포인트
- `docs/tasks/mind-architecture/task-rel-phase1-reflection.md` v0.12 — Phase 1 spec 종결 (Stage 0 Findings 포함)
