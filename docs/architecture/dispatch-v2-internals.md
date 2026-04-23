# `dispatch_v2` 내부 동작

> **Deep-Dive #1.** [`system-overview.md` §7](system-overview.md) 의 1번 항목.
> 대상: `CommandDispatcher::dispatch_v2` 의 **구현 수준** 설명. 개념 다이어그램은 개관 문서에 있고, 이 문서는 **실제 코드가 어떻게 움직이는지**·**확장할 때 무엇을 건드려야 하는지** 에 집중한다.
>
> 근거 파일 (전부 실제 코드에서 확인됨):
> - `src/application/command/dispatcher.rs` — 오케스트레이터 본체
> - `src/application/command/handler_v2.rs` — `EventHandler` trait / `HandlerShared` / `EventHandlerContext`
> - `src/application/command/priority.rs` — 우선순위 상수 + 불변식 테스트
> - `src/application/command/types.rs` — `Command` enum + `aggregate_key()`
> - `src/domain/aggregate.rs` — `AggregateKey`
> - `src/application/command/agents/emotion_agent.rs` — Transactional 핸들러 예시
> - `src/application/command/projection_handlers.rs` — Inline 핸들러 예시

---

## 1. 한 장 요약

```
dispatch_v2(cmd).await
  │
  ├─ ① build_initial_event(cmd) ──────────▶ *Requested event (depth=0)
  │   ├─ 입력 검증 (InvalidSituation early-return)
  │   └─ aggregate_key 결정 (Scene / Npc / Relationship / Rumor / World)
  │
  ├─ ② repository.lock()  ← 단일 MutexGuard, 커맨드 수명 동안 보유
  │
  ├─ ③ Transactional phase (BFS loop)
  │   while queue.pop_front():
  │     depth/budget 가드 → DispatchV2Error 즉시 반환
  │     for h in transactional_handlers.iter()   ← priority 오름차순 사전정렬
  │       if !h.interest().matches(event) continue
  │       if h.mode() != Transactional         continue
  │       let result = h.handle(event, ctx)?   ← HandlerShared 쓰기, repo read-only
  │       if can_emit_follow_up:
  │         for fu in result.follow_up_events: queue.push_back((depth+1, fu))
  │     staging_buffer.push(event)
  │     prior_events.push(event)              ← 같은 커맨드의 과거 이벤트 조회용
  │
  ├─ ④ apply_shared_to_repository(repo, aggregate_key, &shared)
  │     emotion_state / relationship / scene save → clear_emotion_for / clear_scene
  │
  ├─ ⑤ commit_staging_buffer(..) → Vec<DomainEvent> (seq/id 할당 후 EventStore.append)
  │
  ├─ ⑥ Inline phase (커밋된 이벤트 각각에 대해)
  │   for event in committed:
  │     for h in inline_handlers.iter()       ← priority 오름차순 사전정렬
  │       if !h.interest().matches(event) continue
  │       if let Err(e) = h.handle(event, ctx):
  │         tracing::warn!(...)               ← 로그만, 커맨드는 계속
  │
  ├─ ⑦ drop(repo_guard)
  │
  └─ ⑧ Fanout phase
      for event in committed: event_bus.publish(event)

  → Ok(DispatchV2Output { events, shared })
```

**계약 한 줄**: 커맨드 하나는 **①→②→③의 동기 BFS**로 이벤트 그래프를 쌓은 뒤 **④⑤의 원자 커밋**을 지나, **⑥의 best-effort Inline**과 **⑧의 broadcast**로 끝난다. ③ 중 에러는 전체 실패로 롤백되고, ⑥ 중 에러는 로그만 남는다.

---

## 2. 파일 지도

| 파일 | 타입 / 함수 | 역할 |
|---|---|---|
| `command/dispatcher.rs` | `CommandDispatcher<R>` · `dispatch_v2` · `build_initial_event` · `apply_shared_to_repository` · `commit_staging_buffer` | 오케스트레이션, 5단계 파이프라인 |
| `command/dispatcher.rs` | `DispatchV2Output` · `DispatchV2Error` | 공개 반환 타입 |
| `command/dispatcher.rs` | `MAX_CASCADE_DEPTH=4` · `MAX_EVENTS_PER_COMMAND=20` | 안전 한계 상수 |
| `command/handler_v2.rs` | `EventHandler` trait · `DeliveryMode` · `HandlerInterest` · `HandlerResult` · `HandlerError` | 핸들러 프로토콜 |
| `command/handler_v2.rs` | `HandlerShared` · `EventHandlerContext` | 커맨드 범위 mutable 상태 + 컨텍스트 주입 |
| `command/handler_v2.rs` | `test_support::HandlerTestHarness` | L1 단위 테스트 지원 |
| `command/priority.rs` | `transactional::*` · `inline::*` + `invariants` 테스트 | 우선순위 상수 + 순서 회귀 방지 |
| `command/types.rs` | `Command` · `Command::aggregate_key()` | 10종 커맨드 + 라우팅 키 |
| `command/agents/*.rs` | 8 Transactional Agent | Appraise/Stimulus/Guide/Relationship/Scene/Information/Rumor/WorldOverlay |
| `command/projection_handlers.rs` | 3 Inline Projection wrapper | Emotion/Relationship/Scene read-view 갱신 |
| `command/{telling,rumor_distribution,world_overlay,relationship_memory,scene_consolidation}_handler.rs` | 5 Inline Ingestion handler | Memory 쓰기·흡수 |
| `domain/aggregate.rs` | `AggregateKey` · `npc_id_hint()` | 이벤트·커맨드 라우팅 식별자 |

---

## 3. 데이터 구조 3종

### 3.1 `Command` (10종)

`types.rs` 에 enum으로 정의. `aggregate_key()` 메서드가 `Scene` / `Npc` / `Relationship` / `Rumor` / `World` 로 라우팅 키를 결정한다.

```rust
pub enum Command {
    Appraise            { npc_id, partner_id, situation: Option<SituationInput> },
    ApplyStimulus       { npc_id, partner_id, pleasure, arousal, dominance, situation_description },
    GenerateGuide       { npc_id, partner_id, situation_description },
    UpdateRelationship  { npc_id, partner_id, significance: Option<f32> },
    EndDialogue         { npc_id, partner_id, significance: Option<f32> },
    StartScene          { npc_id, partner_id, significance: Option<f32>, focuses: Vec<SceneFocusInput> },
    TellInformation(TellInformationRequest),
    SeedRumor(SeedRumorRequest),
    SpreadRumor(SpreadRumorRequest),
    ApplyWorldEvent(ApplyWorldEventRequest),
}
```

`partner_id()` 는 `TellInformation` · `SeedRumor` · `SpreadRumor` · `ApplyWorldEvent` 에서 **빈 문자열을 반환**한다. Director가 Scene 기반 라우팅을 시도하지 않도록 이 네 커맨드는 dispatcher 직접 호출 경로로 처리해야 한다 (`types.rs` `partner_id()` 주석 참조).

### 3.2 `HandlerShared` — 커맨드 범위 scratchpad

`handler_v2.rs` 에 정의. Transactional 핸들러들이 같은 커맨드 안에서 **읽고/쓰고/지울** 수 있는 mutable 상태.

```rust
#[derive(Debug, Default)]
pub struct HandlerShared {
    pub emotion_state: Option<EmotionState>,
    pub relationship:  Option<Relationship>,
    pub scene:         Option<Scene>,
    pub guide:         Option<ActingGuide>,

    // destructive signals (B4.1) — None/false = 변화 없음
    pub clear_emotion_for: Option<String>,
    pub clear_scene:       bool,
}
```

- `Option<T>`는 "변화 없음 vs. 새로 세팅" 을 표현.
- **삭제**는 `Option`으로 표현 불가 → 별도 불린/ID 플래그 2개. `DialogueEndRequested`를 `RelationshipAgent`가 처리할 때 이 두 필드를 세팅하면 ④ 단계에서 `clear_emotion_state` / `clear_scene` 이 호출된다.
- 필드 추가는 **PR 리뷰 항목** (용어 drift 방지). 신규 도메인이 생기면 먼저 `HandlerShared` 필드부터 설계.

### 3.3 반환 타입

```rust
pub struct DispatchV2Output {
    pub events: Vec<DomainEvent>,   // ⑤에서 EventStore에 append된 최종본
    pub shared: HandlerShared,      // 핸들러 체인의 최종 스냅샷
}

#[derive(thiserror::Error)]
pub enum DispatchV2Error {
    InvalidSituation(String),                        // ① 단계 검증 실패
    CascadeTooDeep { depth: u32 },                   // depth > MAX_CASCADE_DEPTH
    EventBudgetExceeded,                             // staging_buffer.len() ≥ MAX_EVENTS_PER_COMMAND
    HandlerFailed { handler: &'static str, source: HandlerError },
}
```

`HandlerError` variant는 HTTP 상태 매핑을 타입으로 표현한다:

| variant | HTTP | 의미 |
|---|---|---|
| `NpcNotFound(id)` | 404 | 주체 NPC 부재 |
| `RelationshipNotFound { owner, target }` | 404 | 관계 등록 안 됨 |
| `EmotionStateNotFound(id)` | 400 | 워크플로우 순서 오류 (appraise 선행 누락 등) |
| `InvalidInput(msg)` | 400 | DTO → 도메인 변환 실패 |
| `Infrastructure(static)` | 500 | Mutex poison 등 invariant 위반 |
| `Repository(msg)` | 500 | 저장소 I/O 실패 |

Mind Studio `AppError::V2Dispatch`는 문자열 매칭이 아닌 variant 매칭으로 이 네 종류를 HTTP로 분기한다.

---

## 4. 5단계 상세

### 4.1 ① 초기 이벤트 생성 — `build_initial_event(cmd)`

커맨드를 **`*Requested` 이벤트 하나**로 변환한다. 10종 매핑:

| Command | 초기 이벤트 `EventPayload` | 초기 aggregate_id |
|---|---|---|
| `Appraise` | `AppraiseRequested` | `npc_id` |
| `ApplyStimulus` | `StimulusApplyRequested` | `npc_id` |
| `GenerateGuide` | `GuideRequested` | `npc_id` |
| `UpdateRelationship` | `RelationshipUpdateRequested` | `npc_id` |
| `EndDialogue` | `DialogueEndRequested` | `npc_id` |
| `StartScene` | `SceneStartRequested { prebuilt_scene, initial_focus_id }` | `npc_id` |
| `TellInformation` | `TellInformationRequested` | `speaker` |
| `SeedRumor` | `SeedRumorRequested { pending_id }` | `format!("pending-{pending_id}")` |
| `SpreadRumor` | `SpreadRumorRequested` | `rumor_id` |
| `ApplyWorldEvent` | `ApplyWorldEventRequested` | `world_id` |

이 단계에서만 발생하는 조기 실패:

- `StartScene`: `SituationService::to_scene_focus` 실패 → `InvalidSituation`
- `Appraise` with `situation=None`: 활성 Scene·Focus 없으면 `InvalidSituation`
- `SeedRumor`: `topic=None && seed_content=None` → `InvalidSituation`
- `ApplyWorldEvent`: `world_id` 또는 `fact` 비어있음 → `InvalidSituation`

**`SeedRumor`의 `pending_id`** 는 `command_seq: AtomicU64` (dispatcher 내부)가 발급하는 커맨드별 고유 suffix다. 여러 Seed 커맨드가 동일한 "orphan" 버킷을 공유하지 않도록 하기 위함 (Step C3 사후 리뷰 C2). `event_store.next_id` 와 별개라서 **event id gap을 유발하지 않는다**.

### 4.2 ② Repository lock

`self.repository: Arc<Mutex<R>>` 의 `lock()` 을 잡고 **커맨드 수명 동안** 보유한다. 즉 **한 dispatcher 인스턴스에서 커맨드들은 직렬화** 된다. Director가 Scene별로 `SceneTask` mpsc 루프를 돌리는 이유 — 서로 다른 Scene의 커맨드가 서로를 블록하지 않도록 `Arc<CommandDispatcher>` 를 공유하면서도 mpsc가 자연스러운 직렬화 큐 역할을 해준다.

`EventHandlerContext::repo` 는 `&(dyn MindRepository + Send + Sync)` 로 read-only 주입된다. **Transactional 핸들러는 repo에 직접 쓰지 않고 `HandlerShared`에 쓴다**. 실제 repo 반영은 ④에서 한 번만 일어난다.

### 4.3 ③ Transactional BFS

```rust
let mut event_queue: VecDeque<(u32, DomainEvent)> = VecDeque::new();
let mut staging_buffer: Vec<DomainEvent> = Vec::new();
let mut prior_events:   Vec<DomainEvent> = Vec::new();
event_queue.push_back((0, initial_event));

while let Some((depth, event)) = event_queue.pop_front() {
    if depth > MAX_CASCADE_DEPTH            { return Err(CascadeTooDeep { depth }); }
    if staging_buffer.len() >= MAX_EVENTS... { return Err(EventBudgetExceeded); }

    for handler in self.transactional_handlers.iter() {      // ← priority 오름차순 사전정렬
        if !handler.interest().matches(&event) { continue; }
        let DeliveryMode::Transactional { can_emit_follow_up, .. } = handler.mode() else { continue; };

        let mut ctx = EventHandlerContext { repo, event_store, shared, prior_events, aggregate_key };
        let result = handler.handle(&event, &mut ctx)?;      // ← 에러 = 커맨드 전체 실패

        if can_emit_follow_up {
            for fu in result.follow_up_events {
                event_queue.push_back((depth + 1, fu));
            }
        }
    }

    staging_buffer.push(event.clone());
    prior_events.push(event);
}
```

핵심 포인트:

- **BFS**: depth가 깊어지는 follow-up은 큐 **뒤** 에 붙는다. 같은 depth의 모든 이벤트를 먼저 소진한 뒤 다음 depth로 넘어간다.
- **핸들러 정렬은 `register_*` 시점**에 `sort_by_key` 로 끝난다. 매 이벤트마다 정렬하지 않는다.
- **`interest()` 필터**가 먼저 돌고, 그 다음 **`mode()` 이중 확인**. Inline 핸들러가 실수로 `transactional_handlers` 에 들어간 경우 `debug_assert!`로 감지하지만 release에서는 `continue`로 조용히 스킵.
- **`prior_events`** 는 같은 커맨드 안에서 이미 지나간 이벤트 목록. 예: `GuideAgent`가 같은 커맨드 내 `EmotionAppraised`나 `StimulusApplied`를 조회할 수 있게 해주는 근거. 이게 없었다면 `HandlerShared`만으로는 "직전 감정 이벤트의 snapshot"을 얻기 어려웠다.
- **`can_emit_follow_up = false`** 인 핸들러가 `follow_up_events` 를 돌려주면 `debug_assert!` 폭발. Release에선 조용히 무시되지만 테스트에서 잡는다.

### 4.4 ④ `apply_shared_to_repository`

```rust
fn apply_shared_to_repository(repo: &mut R, aggregate_key: &AggregateKey, shared: &HandlerShared) {
    if let Some(state) = &shared.emotion_state {
        repo.save_emotion_state(aggregate_key.npc_id_hint(), state.clone());
    }
    if let Some(rel) = &shared.relationship {
        repo.save_relationship(rel.owner_id(), rel.target_id(), rel.clone());
    }
    if let Some(scene) = &shared.scene {
        repo.save_scene(scene.clone());
    }
    if let Some(npc_id) = &shared.clear_emotion_for { repo.clear_emotion_state(npc_id); }
    if shared.clear_scene                           { repo.clear_scene(); }
}
```

**순서 보장**: save → clear. 같은 커맨드에서 `emotion_state=Some(...)` 와 `clear_emotion_for=Some(npc)` 를 동시에 세팅하면 save가 먼저 찍히고 그 뒤에 clear가 지운다. `DialogueEndRequested`를 `RelationshipAgent`가 처리할 때 이 순서를 가정한다.

`npc_id_hint()` 는 `Scene{npc}` / `Npc(id)` / `Relationship{owner}` / `Memory/Rumor/World(id)` 에서 문자열 하나를 뽑아주는 헬퍼. Memory/Rumor/World 키에서는 **실제 NPC id가 아니다** — 로그·저장소 키 계산용 식별자로만 써야 한다.

### 4.5 ⑤ Commit — `commit_staging_buffer`

```rust
for event in staging {
    let per_event_id = event.aggregate_key().npc_id_hint().to_string();
    let id  = self.event_store.next_id();
    let seq = self.event_store.next_sequence(&per_event_id);
    let mut e = DomainEvent::new(id, per_event_id, seq, event.payload);
    if let Some(cid) = self.current_correlation_id() { e = e.with_correlation(cid); }
    self.event_store.append(&[e.clone()]);
    committed.push(e);
}
```

**의도적 설계**: `aggregate_id` 는 **각 이벤트의 payload가 스스로 선언한 aggregate_key** 로 결정된다 (`event.aggregate_key().npc_id_hint()`). 커맨드의 aggregate_key로 덮어쓰지 **않는다**. 이유:

- 예: `TellInformation` 커맨드의 aggregate_key는 `Npc(speaker)` 지만, follow-up `InformationTold` 이벤트는 `Npc(listener)` 로 라우팅되어야 한다 (§3.3 B5 결정). `EventStore.get_events(listener_id)` 같은 청자 기반 질의가 올바르게 동작하려면 이벤트별 aggregate_id가 필요하다.
- 기존 이벤트(`EmotionAppraised` / `BeatTransitioned` / `RelationshipUpdated`)는 payload의 `npc_id_hint()` 가 커맨드의 것과 같아서 실질적 차이가 없다 — 새 커맨드를 추가할 때 이 점이 중요하다.

`correlation_id` 는 `AtomicU64`로 관리되며 `set_correlation_id` 로 외부에서 세팅된 값이 있으면 커밋되는 모든 이벤트에 동일 ID가 찍힌다. MCP/REST 호출 단위 추적용.

### 4.6 ⑥ Inline phase

```rust
for event in &committed {
    for handler in self.inline_handlers.iter() {        // priority 오름차순
        if !handler.interest().matches(event) { continue; }
        if !matches!(handler.mode(), DeliveryMode::Inline { .. }) { continue; }
        let mut ctx = EventHandlerContext { repo, event_store, shared, prior_events, aggregate_key };
        if let Err(e) = handler.handle(event, &mut ctx) {
            tracing::warn!(handler = handler.name(), error = %e, "inline handler failed");
        }
    }
}
```

- **이중 루프**: 바깥은 커밋된 이벤트 순회, 안쪽은 핸들러 순회. Projection 3종 → Memory 4종 순서로 실행된다 (priority 10/20/30 → 40/45/50/60).
- **에러는 `tracing::warn` 만** 찍고 계속. 커맨드는 성공 반환. 이게 Inline 계약.
- **아직 repo lock 안에서 실행**. `drop(repo_guard)` 는 ⑥이 끝난 뒤에 일어난다. 즉 Inline 핸들러도 repo를 read할 수 있고, 외부 읽기 요청은 Inline이 끝날 때까지 대기한다 — Mind Studio가 write 직후 re-fetch 해도 Projection이 이미 최신인 이유.
- `HandlerShared` 도 여전히 mutable로 주입되지만 관례상 Inline 핸들러는 **읽기만 한다**. 쓰기는 ④에서 이미 repo에 반영된 뒤라 의미가 없다.

### 4.7 ⑦⑧ Drop + Fanout

```rust
drop(repo_guard);
for event in &committed { self.event_bus.publish(event); }
Ok(DispatchV2Output { events: committed, shared })
```

- **lock을 먼저 놓는다**. fanout은 broadcast라서 구독자(MemoryAgent / SSE / 게임 엔진 등)가 repo를 읽으려 하면 데드락 위험이 있다. 순서가 중요.
- `event_bus.publish` 는 `tokio::broadcast::send` 로 귀결. 구독자 lag은 `subscribe_with_lag` 사용자가 replay로 복구.

---

## 5. `EventHandler` 프로토콜

```rust
pub trait EventHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn interest(&self) -> HandlerInterest;           // All / Kinds(Vec<EventKind>) / Predicate(fn)
    fn mode(&self) -> DeliveryMode;                   // Transactional / Inline / Fanout
    fn handle(&self, event: &DomainEvent, ctx: &mut EventHandlerContext<'_>)
        -> Result<HandlerResult, HandlerError>;
}

pub enum DeliveryMode {
    Transactional { priority: i32, can_emit_follow_up: bool },
    Inline        { priority: i32 },
    Fanout,
}
```

**주의**: `Fanout` variant는 trait 상에 정의되어 있지만 현재 `dispatch_v2` 가 사용하는 건 Transactional + Inline 두 개뿐이다. 외부 fan-out은 `EventBus.subscribe()` 로 별도 경로에서 구독한다. `Fanout` variant는 향후 "Dispatcher가 publish 전에 동기화해야 하는 fanout 구독자" 를 지원할 때를 위한 여지다.

**Transactional 핸들러가 지켜야 할 계약**

1. `repo` 는 read-only. 변경은 `ctx.shared.*` 로만.
2. `follow_up_events` 는 `can_emit_follow_up = true` 일 때만 반환.
3. 자기 관심사 아니면 `HandlerInterest::Kinds` 에 없어야 함 — `interest().matches()` 가 먼저 걸러주지만, `handle` 안에서도 payload 패턴 매칭 + else 분기로 방어하는 패턴을 유지 (`EmotionAgent` 참고).
4. 에러는 `커맨드 전체 중단`. 일상적 "이벤트가 내 관심 외" 상태를 에러로 만들지 말 것.

**Inline 핸들러가 지켜야 할 계약**

1. `priority` 는 `priority::inline::*` 상수에서 선택.
2. `follow_up_events` 는 **돌려주지 말 것** (Inline은 follow-up을 제공하지 않음, dispatcher가 무시하지만 혼란만 유발).
3. 에러는 로그로만 흘러간다 — 중요한 invariant 위반은 `HandlerError::Infrastructure` 로 에스컬레이트.

**`EventHandlerContext` 필드**

```rust
pub struct EventHandlerContext<'a> {
    pub repo:          &'a (dyn MindRepository + Send + Sync),
    pub event_store:   &'a dyn EventStore,
    pub shared:        &'a mut HandlerShared,
    pub prior_events:  &'a [DomainEvent],
    pub aggregate_key: AggregateKey,
}
```

`repo`에 `Send + Sync`를 **인라인으로** 요구하는 이유 — `MindRepository` trait 자체에는 그 바운드가 없는데, SceneTask가 이 컨텍스트를 워커 스레드로 넘길 때 컴파일 에러를 예방하기 위함이다 (`handler_v2.rs` 주석).

---

## 6. 우선순위 상수

`priority.rs` 의 `invariants` 모듈이 순서 회귀를 **테스트로 지킨다**. 상수 하나 바꿀 때 실수로 순서 뒤집기 불가.

### 6.1 Transactional (작은 값 먼저)

| 상수 | 값 | 핸들러 | 왜 이 순서 |
|---|---|---|---|
| `SCENE_START` | 5 | SceneAgent | 감정 평가의 전제 |
| `EMOTION_APPRAISAL` | 10 | EmotionAgent | 자극/가이드가 emotion_state 의존 |
| `STIMULUS_APPLICATION` | 15 | StimulusAgent | 자극 변동은 감정 평가 뒤 |
| `GUIDE_GENERATION` | 20 | GuideAgent | 감정·자극 완료 후 가이드 작성 |
| `WORLD_OVERLAY` | 25 | WorldOverlayAgent | Guide 이후, 관계 이전 |
| `RELATIONSHIP_UPDATE` | 30 | RelationshipAgent | Scene/Beat 종료 시 |
| `INFORMATION_TELLING` | 35 | InformationAgent | 청자의 현재 trust 반영 위해 관계 갱신 뒤 |
| `RUMOR_SPREAD` | 40 | RumorAgent | 정보 전달 뒤 |
| `AUDIT` | 90 | (예약) | 마지막 감사 로깅용 자리 |

### 6.2 Inline (작은 값 먼저)

| 상수 | 값 | 핸들러 |
|---|---|---|
| `EMOTION_PROJECTION` | 10 | EmotionProjectionHandler |
| `RELATIONSHIP_PROJECTION` | 20 | RelationshipProjectionHandler |
| `SCENE_PROJECTION` | 30 | SceneProjectionHandler |
| `MEMORY_INGESTION` | 40 | TellingIngestionHandler / RumorDistributionHandler |
| `WORLD_OVERLAY_INGESTION` | 45 | WorldOverlayHandler |
| `RELATIONSHIP_MEMORY` | 50 | RelationshipMemoryHandler |
| `SCENE_CONSOLIDATION` | 60 | SceneConsolidationHandler |

**불변식으로 지켜지는 규칙** (모두 `priority.rs::invariants` 테스트):
- `EMOTION_APPRAISAL < GUIDE_GENERATION`
- `STIMULUS_APPLICATION < GUIDE_GENERATION`
- `SCENE_START < EMOTION_APPRAISAL`
- `INFORMATION_TELLING > RELATIONSHIP_UPDATE`
- `RUMOR_SPREAD > INFORMATION_TELLING`
- `WORLD_OVERLAY ∈ (GUIDE_GENERATION, RELATIONSHIP_UPDATE)`
- Inline: `MEMORY_INGESTION > SCENE_PROJECTION`
- Inline: `WORLD_OVERLAY_INGESTION > MEMORY_INGESTION > ... > SCENE_CONSOLIDATION (마지막)`

---

## 7. 안전 한계

```rust
pub const MAX_CASCADE_DEPTH: u32     = 4;
pub const MAX_EVENTS_PER_COMMAND: usize = 20;
```

**왜 이 숫자인가.**

- **Depth 4**: 현실 최악 체인을 세어보면 `SceneStartRequested(0) → SceneStarted + AppraiseRequested(1) → EmotionAppraised(2) → GuideRequested(3) → GuideGenerated(4)` 정도. 여유 1 더 둬서 4. 그보다 깊어지면 보통 설계 실수.
- **이벤트 20개**: 같은 커맨드에서 이보다 많은 이벤트가 쏟아지면 십중팔구 방송 커맨드(여러 청자에게 동시 전달)를 트랜잭션으로 잘못 설계한 것. 부분 실패 시 롤백할 원자 단위가 너무 크다.

**가드 체크 타이밍**: 큐 `pop` 직후에 검사한다. 즉 이미 push된 follow-up은 queue에 들어가고, 꺼내지는 순간 실패한다. staging_buffer는 **건드리지 않은 채** `DispatchV2Error` 를 반환 — `event_store.append` 도 호출되지 않아 롤백 걱정 없음.

---

## 8. Builder 비교

dispatcher는 기본 핸들러 세트를 체이닝으로 조립한다.

```rust
CommandDispatcher::new(repo, event_store, event_bus)
    .with_default_handlers()          // ① 필수
    .with_memory(memory_store)         //   or with_memory_full(memory_store)
    .with_rumor(memory_store, rumor_store)
```

| Builder | 등록하는 핸들러 | 용도 |
|---|---|---|
| `with_default_handlers()` | SceneAgent, EmotionAgent, StimulusAgent, GuideAgent, RelationshipAgent, InformationAgent, WorldOverlayAgent (Transactional) + 3 Projection (Inline) | 코어 Mind 엔진만 |
| `with_memory(store)` | + `TellingIngestionHandler` (Inline) | Step C2 lean — 기존 콜러 호환용 |
| `with_memory_full(store)` | + `TellingIngestion` + `WorldOverlay` + `RelationshipMemory` + `SceneConsolidation` (모두 Inline) | Step D 전체 번들 |
| `with_rumor(memory, rumor)` | + `RumorAgent` (Transactional) + `RumorDistributionHandler` (Inline) | 소문 서브시스템 |

**중요**: `with_memory` / `with_memory_full` 는 **상호 배타가 아니다** — 같은 dispatcher에서 같은 `TellingIngestionHandler` 가 두 번 등록되어 중복 실행될 수 있다. `with_memory_full` 을 쓰려면 `with_memory` 는 호출하지 말 것. 현재 코드에 방어 장치는 없음 (리뷰 H5).

커스텀 핸들러는 `register_transactional(Arc<dyn EventHandler>)` / `register_inline(...)` 로 직접 붙일 수 있다. `debug_assert!` 로 mode 불일치를 잡아준다.

---

## 9. 확장: 무엇을 건드려야 하는가

### 9.1 새 Command 추가

1. `types.rs::Command` 에 variant 추가.
2. `Command::aggregate_key()` · `npc_id()` · `partner_id()` 에 arm 추가.
3. `dispatcher.rs::build_initial_event` 에 arm 추가 — `*Requested` 이벤트 생성.
4. `domain/event.rs::EventPayload` 에 `*Requested` variant 추가 + `aggregate_key()` 에 매핑.
5. 처리할 `Transactional` 핸들러가 없으면 Agent 새로 만들기 (§9.2).
6. `with_default_handlers` 에 해당 Agent 등록할지, 별도 builder 만들지 결정.
7. DTO 필요하면 `application/dto.rs`.
8. 통합 테스트: `tests/dispatch_v2_test.rs` 에 커맨드 호출 → 예상 이벤트 검증.

### 9.2 새 Transactional Agent

1. `command/agents/your_agent.rs` 에 struct + `impl EventHandler`.
2. `interest()` — `HandlerInterest::Kinds(vec![EventKind::...])`.
3. `mode()` — `DeliveryMode::Transactional { priority: priority::transactional::YOUR_SLOT, can_emit_follow_up: true|false }`.
4. `priority.rs::transactional` 에 상수 추가. `invariants` 모듈에 순서 테스트 추가.
5. `handle(event, ctx)`:
   - payload 패턴 매칭 (비일치면 `Ok(HandlerResult::default())`)
   - repo에서 필요한 엔티티 조회 (`ctx.repo.get_*`), 없으면 `HandlerError::*NotFound`
   - 도메인 로직 실행
   - `ctx.shared.*` 에 결과 write
   - follow-up 필요 시 `follow_up_events` 에 push
6. `dispatcher.rs::with_default_handlers` 또는 별도 builder에 `register_transactional(Arc::new(YourAgent::new()))` 추가.
7. L1 테스트: `handler_v2::test_support::HandlerTestHarness` 로 단위 테스트.

### 9.3 새 Inline Handler

- Transactional과 동일하지만 `DeliveryMode::Inline { priority: priority::inline::YOUR_SLOT }`.
- follow-up 없음. `HandlerResult::default()` 반환.
- 에러는 `tracing::warn` 로 흘러감을 인지하고, 인프라 invariant는 `Infrastructure(static_str)` 로.
- repo는 read-only로 사용. 쓰기가 필요하면 Inline이 아니라 Transactional에서 `HandlerShared` 경유로.

### 9.4 새 `HandlerShared` 필드

신규 도메인 스테이트(예: 신규 Aggregate에서 커맨드 범위로 집계할 값)가 필요한 경우에만. 용어 drift 방지 위해 PR 리뷰 항목. 추가 시 `apply_shared_to_repository` 도 대응 — 그렇지 않으면 `HandlerShared` 에만 쌓이고 repo에 반영 안 됨.

### 9.5 새 `AggregateKey` variant

Memory/Rumor/World 추가 전례 (Step C1). 필요한 경우:

1. `domain/aggregate.rs::AggregateKey` 에 variant + `npc_id_hint()` arm.
2. 관련 `EventPayload::aggregate_key()` 구현.
3. `Command::aggregate_key()` 에 신규 커맨드 매핑.
4. Director가 이 키를 어떻게 라우팅할지 결정 — `Scene` 외의 키는 현재 dispatcher 직접 호출 경로로 처리.

---

## 10. 유의사항·함정 모음

1. **Transactional 핸들러에서 repo에 직접 write 금지.** Lock을 가진 guard가 read-only로 주입되어 컴파일러가 잡아주지만, `RefCell` 같은 우회로 뚫으면 ④와 충돌.
2. **`follow_up_events` 에 과거 이벤트 넣지 말 것.** BFS 큐에 들어가 중복 실행 + depth 가드 폭발.
3. **같은 커맨드에서 emotion 세팅 + clear를 동시에 내면 clear가 이긴다.** 의도 맞는지 확인.
4. **`with_memory` 와 `with_memory_full` 동시 호출 금지.** TellingIngestion 중복 등록. 현재 방어 장치 없음.
5. **새 이벤트의 `aggregate_key()` 가 커맨드와 다른 경우**, `commit_staging_buffer` 가 payload 기준으로 aggregate_id를 찍으므로 `EventStore.get_events_by_aggregate(id)` 질의가 예상과 다를 수 있다. 이건 버그가 아니라 의도된 설계지만, 테스트 작성 시 혼동 주의.
6. **`InMemoryEventStore` 는 프로세스 수명 동안 누적**. Mind Studio 장기 실행 시 메모리·`next_sequence` O(N) scan 비용. 영구 store는 Phase 8+.
7. **`set_correlation_id` 는 `&self` + `AtomicU64`** 이지만 값은 전역 (dispatcher 단위). 동시 여러 커맨드가 같은 dispatcher를 쓰면 상관관계 ID가 섞일 수 있다. 현재 Mind Studio는 repo lock이 커맨드를 직렬화하므로 실질적 충돌은 없다.
8. **`Command::SeedRumor` 의 초기 aggregate_id는 `"pending-<seq>"`** — 실제 RumorId가 부여되기 전 임시값. 이 값으로 `EventStore` 를 질의하면 `SeedRumorRequested` 하나만 나온다. RumorAgent가 발행하는 `RumorSeeded` 는 진짜 RumorId로 찍혀 별도 aggregate로 저장된다.

---

## 11. 관련 문서

- [`system-overview.md`](system-overview.md) — 전체 구조 개관 (이 문서의 부모)
- [`system-design-eventbus-cqrs.md`](system-design-eventbus-cqrs.md) — EventBus / CQRS / Event Sourcing / Multi-Agent / RAG 상세 설계
- [`b-plan-implementation.md`](b-plan-implementation.md) — B안 마이그레이션 단계별 설계 (B0 ~ B5.3)
- 다음 Deep-Dive 후보: **#2 EventHandler 카탈로그** — 8 Transactional + 8 Inline 핸들러의 입력/출력/부수효과 매트릭스
