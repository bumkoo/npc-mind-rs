# B안 구축 방안 — 다중 Scene 동시 실행 지원

**작성일:** 2026-04-19
**최종 갱신:** 2026-04-20 (B5.1 완료 시점)
**상태:** B0~B5.1 ✅ 완료 · B5.2~B5.4 진행 예정
**선행 문서:** [`unified-event-protocol-analysis.md`](unified-event-protocol-analysis.md) (개념 1~8 분석)
**관련:** [`system-design-eventbus-cqrs.md`](system-design-eventbus-cqrs.md) (현재 아키텍처)

## 진행 현황 요약 (2026-04-20)

| Stage | 상태 | 핵심 결과물 |
|---|---|---|
| B0 | ✅ 완료 | `EventHandler` trait · `HandlerShared` · `AggregateKey` · `priority` 상수 |
| B1 | ✅ 완료 | 4 Agent + StimulusAgent 신규 · `AppraiseRequested`/`StimulusApplyRequested` variant · `HandlerTestHarness` |
| B2 | ✅ 완료 | `{Emotion,Relationship,Scene}ProjectionHandler` Inline wrapper |
| B3 | ✅ 완료 | `dispatch_v2()` BFS loop · `with_default_handlers()` · v1/v2 parallel run |
| B4 S1 | ✅ 완료 | 6 Command 전부 v2 지원 · SceneAgent 신규 · 4 추가 *Requested variant · HandlerShared clear 시그널 |
| B4 S2 | ✅ 완료 | `Director` facade · `SceneId` composite key · `InMemoryRepository` multi-scene HashMap |
| B4 S3 Option A | ✅ 완료 | `BeatTransitioned.partner_id` · `SceneStore::get_scene_by_id` default method · multi-scene 정확성 회귀 가드 |
| B4 S3 Option B-Mini | ✅ 완료 | Mind Studio `/api/v2/*` 7 엔드포인트 · Director 통합 shadow path |
| B5.1 | ✅ 완료 | v1 API `#[deprecated(since="0.2.0")]` 마킹 · 내부·테스트 `#[allow(deprecated)]` |
| B5.2 | ⏳ 대기 | 내부 호출자(DialogueAgent 등) v2 경로로 마이그레이션 |
| B5.3 | ⏳ 대기 | v1 모듈·타입 삭제 (`Pipeline`/`Projection` trait/`EventAwareMindService`/v1 dispatch/v1 Agent handle_*) |
| B5.4 | ⏳ 대기 | `shadow_v2` 플래그 제거 (v2만 존재) |

**원래 §9.1 이연 결정 이력**:
- Command에 `scene_id` 필드 도입은 B4에서 도입 대신 **커맨드의 (npc_id, partner_id)를 SceneId로 활용**하는 실용적 접근 채택. `Command::aggregate_key()`가 동일 목적 달성.

**B-Plan 설계 대비 변경**:
- `ProjectionRegistry`가 `Vec<Arc<dyn EventHandler>>`로 **storage 전환**하는 작업(§8 B2 항목 2)은 **B3에서 자연스럽게 해소** — Dispatcher의 `inline_handlers: Vec<Arc<dyn EventHandler>>`가 그 역할. 기존 ProjectionRegistry는 B5.3에서 통째로 제거 예정.
- B4 S3에서 async dispatch_v2 / 실제 tokio SceneTask는 **아직 미구현** — Director는 sync façade. 필요 시점(LLM 호출 지연이 병렬화 이득을 낳을 때) 별도 세션에서 추진.

---

## 1. 배경과 동기

싱글 플레이어 무협 게임이지만 **여러 Scene이 동시에 진행**된다.

- 플레이어가 참여하는 **전경 Scene** 1개
- NPC끼리 대화가 이어지는 **배경 Scene** N개 (세계의 지속성)
- 각 Scene은 독립 LLM 세션 · 독립 DialogueAgent · 독립 turn 흐름

현재 구조(2-tier: Pipeline + EventBus)로는 동시 Scene에서 다음 문제가 생긴다.

1. **Aggregate 경계가 암묵적** — 같은 Scene의 commands가 인터리브되면 race 발생
2. **등록 경로가 세 갈래**(Pipeline, ProjectionRegistry, EventBus.subscribe) — Scene마다 독립 Dispatcher를 띄우려면 세 세팅을 모두 복제해야 함
3. **테스트에서 Fanout을 끄고 특정 Scene만 검증**하기가 어려움

이 세 문제를 한 번에 풀기 위해 B안을 도입한다.

## 2. 목표

**G1.** 여러 Scene이 각자 독립 tokio task에서 동시 진행되며, 같은 Scene 내부는 순차 처리가 보장됨.

**G2.** 모든 핸들러가 단일 `EventHandler` 트레이트를 구현하여 등록·관측·테스트가 일원화됨.

**G3.** 외부 관찰자(MemoryAgent, StoryAgent, SSE, UI)가 단일 EventBus에서 모든 Scene의 이벤트를 수신하되, 필요 시 `aggregate_key()`로 Scene별 demultiplex 가능.

**G4.** 현재 코드베이스는 Strangler Fig로 **단계별 안전하게** 전환. 각 Stage가 독립 mergeable.

**G5.** 테스트 피라미드 성립 — 핸들러 단위 / 체인 / 엔드투엔드 각각의 도구 제공.

## 3. 최종 아키텍처 개요

```
┌──────────────────────────────────────────────────────────────────┐
│ Director (싱글톤, 게임 루프 측)                                  │
│   - scenes: HashMap<SceneId, mpsc::Sender<Command>>              │
│   - bus: broadcast::Sender<Arc<DomainEvent>>                     │
│   - start_scene() / end_scene() / route_command()                │
└─────┬────────────────┬────────────────┬──────────────────────────┘
      │                │                │
   mpsc::Sender    mpsc::Sender     mpsc::Sender    (Scene 당 1 채널)
      │                │                │
      ▼                ▼                ▼
┌───────────┐    ┌───────────┐    ┌───────────┐
│SceneTask A│    │SceneTask B│    │SceneTask C│    (tokio::spawn 된 task들)
│(플레이어) │    │(배경)     │    │(배경)     │
│           │    │           │    │           │
│ loop {    │    │ loop {    │    │ loop {    │
│  cmd←rx   │    │  cmd←rx   │    │  cmd←rx   │
│  dispatch │    │  dispatch │    │  dispatch │     각 Scene은 자기 Dispatcher
│ }         │    │ }         │    │ }         │     (같은 repo/event_store/bus 공유)
└─────┬─────┘    └─────┬─────┘    └─────┬─────┘
      │                │                │
      └────────────────┼────────────────┘
                       │
              ┌────────▼─────────┐
              │ Shared EventBus  │
              │ (broadcast)      │
              └────────┬─────────┘
                       │ subscribe
      ┌────────────────┼────────────────┬──────────────┐
      ▼                ▼                ▼              ▼
  MemoryAgent      StoryAgent       SSE bridge     (future)
  (Fanout)         (Fanout)         (Fanout)       SummaryAgent
  이벤트→RAG       서사 판단        UI push        Fanout
```

### 3.1 계층 역할

| 계층 | 책임 | 생명주기 |
|---|---|---|
| **Director** | Scene 수명 관리, 커맨드 라우팅 | 프로세스 전역 |
| **SceneTask** | Scene 단위 순차 커맨드 처리 | Scene 시작~종료 |
| **CommandDispatcher** | 세 모드 핸들러 실행 루프 | SceneTask 내부 (또는 공유 인스턴스) |
| **EventHandler** | 실제 비즈니스 로직 | 등록 시점~Director 종료 |
| **Shared EventBus** | 전역 이벤트 방송 | 프로세스 전역 |
| **Fanout subscriber** | 이벤트 구독 외부 시스템 | 독립 tokio task |

### 3.2 공유와 격리

| 자원 | 공유/격리 | 이유 |
|---|---|---|
| `InMemoryRepository` | 공유 (`Arc<R>`) | 세계 상태는 전역 |
| `EventStore` | 공유 (`Arc<dyn>`) | append-only, 동시 쓰기 안전 |
| `MemoryStore` (RAG) | 공유 | 모든 Scene의 기억이 동일 저장소 |
| `EventBus` | 공유 | Fanout subscriber가 모든 Scene을 관찰 |
| `ConversationPort` (LLM) | **Scene별 독립 세션** | 각 Scene은 자기 대화 컨텍스트 |
| `HandlerShared` | **커맨드별 인스턴스** | 한 커맨드 처리 중에만 유효 |
| `Dispatcher` | 공유 or Scene별 — 둘 다 가능 | (§5.2 결정) |

---

## 4. 핵심 타입 정의

### 4.1 EventHandler 트레이트 (개념 1)

```rust
// src/application/command/handler_v2.rs

pub trait EventHandler: Send + Sync {
    /// 트레이싱·로깅용 식별자
    fn name(&self) -> &'static str;

    /// 이 핸들러가 관심 갖는 이벤트 종류
    fn interest(&self) -> HandlerInterest;

    /// 실행 계약
    fn mode(&self) -> DeliveryMode;

    /// 실제 처리
    fn handle(
        &self,
        event: &DomainEvent,
        ctx: &mut HandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError>;
}

pub enum HandlerInterest {
    /// 모든 이벤트
    All,
    /// 특정 종류만
    Kinds(Vec<EventKind>),
    /// 커스텀 술어
    Predicate(fn(&DomainEvent) -> bool),
}

impl HandlerInterest {
    pub fn matches(&self, event: &DomainEvent) -> bool { ... }
}
```

### 4.2 DeliveryMode enum (개념 2)

```rust
pub enum DeliveryMode {
    /// 커맨드 트랜잭션 내부 sync 실행. 에러가 커맨드 전체 중단.
    Transactional {
        priority: i32,
        can_emit_follow_up: bool,
    },

    /// 이벤트 커밋 직후 sync 실행. 에러는 로그, 커맨드는 계속.
    Inline { priority: i32 },

    /// 비동기 fan-out. 발행자는 기다리지 않음.
    Fanout,
}
```

### 4.3 HandlerContext / HandlerShared (개념 3)

```rust
pub struct HandlerContext<'a> {
    pub repo: &'a dyn MindRepository,
    pub event_store: &'a dyn EventStore,
    pub shared: &'a mut HandlerShared,
    pub prior_events: &'a [DomainEvent],
    /// 이 커맨드가 속한 Scene (있는 경우) — Fanout 구독자도 쓸 수 있게 이벤트에 복제
    pub aggregate_key: AggregateKey,
}

/// 같은 커맨드 내 Transactional 핸들러 간 공유 상태.
/// PipelineState의 후신. 필드는 큐레이션된 계약, 확장 주머니 아님.
#[derive(Default)]
pub struct HandlerShared {
    pub emotion_state: Option<EmotionState>,
    pub relationship: Option<Relationship>,
    pub scene: Option<Scene>,
    pub guide: Option<ActingGuide>,
    // 신규 필드 추가는 PR 리뷰 항목
}

pub struct HandlerResult {
    pub follow_up_events: Vec<DomainEvent>,
}

#[derive(thiserror::Error, Debug)]
pub enum HandlerError {
    #[error("precondition failed: {0}")]
    Precondition(&'static str),
    #[error("repository error: {0}")]
    Repo(#[from] RepoError),
    // ...
}
```

### 4.4 DomainEvent.aggregate_key() (파티셔닝 핵심)

```rust
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum AggregateKey {
    Scene(SceneId),
    Npc(NpcId),              // Scene 밖에서 일어나는 단독 평가용
    Relationship(NpcId, NpcId),  // 관계 갱신용 (Scene 종료 시)
}

impl DomainEvent {
    pub fn aggregate_key(&self) -> AggregateKey {
        match &self.payload {
            EventPayload::SceneStarted { scene_id, .. }
            | EventPayload::SceneEnded { scene_id, .. }
            | EventPayload::BeatTransitioned { scene_id, .. }
            | EventPayload::DialogueTurnCompleted { scene_id, .. }
                => AggregateKey::Scene(scene_id.clone()),

            EventPayload::EmotionAppraised { npc_id, scene_id, .. }
            | EventPayload::StimulusApplied { npc_id, scene_id, .. }
                => scene_id.as_ref()
                    .map(|s| AggregateKey::Scene(s.clone()))
                    .unwrap_or_else(|| AggregateKey::Npc(npc_id.clone())),

            EventPayload::RelationshipUpdated { a, b, .. }
                => AggregateKey::Relationship(a.clone(), b.clone()),

            // ...
        }
    }
}
```

`Command`도 동일한 `aggregate_key()`를 갖는다 — Director가 이걸로 라우팅.

```rust
impl Command {
    pub fn aggregate_key(&self) -> AggregateKey {
        match self {
            Command::Appraise { npc, scene_id: Some(s), .. } => AggregateKey::Scene(s.clone()),
            Command::Appraise { npc, scene_id: None, .. } => AggregateKey::Npc(npc.clone()),
            // ...
        }
    }
}
```

---

## 5. 실행 기반

### 5.1 CommandDispatcher 실행 루프 (개념 4 + 5)

```rust
const MAX_CASCADE_DEPTH: u32 = 4;
const MAX_EVENTS_PER_COMMAND: usize = 20;

pub struct CommandDispatcher<R: MindRepository> {
    repo: Arc<R>,
    event_store: Arc<dyn EventStore>,
    broadcast: broadcast::Sender<Arc<DomainEvent>>,
    broadcast_enabled: bool,

    transactional: Vec<Arc<dyn EventHandler>>,  // priority 정렬
    inline: Vec<Arc<dyn EventHandler>>,          // priority 정렬
}

impl<R: MindRepository> CommandDispatcher<R> {
    pub async fn dispatch(&self, cmd: Command) -> Result<CommandOutput, DispatchError> {
        let aggregate = cmd.aggregate_key();
        let mut shared = HandlerShared::default();
        let mut prior_events: Vec<DomainEvent> = Vec::new();
        let mut event_queue: VecDeque<(u32, DomainEvent)> = VecDeque::new();
        let mut staging_buffer: Vec<DomainEvent> = Vec::new();

        event_queue.push_back((0, cmd.into_initial_event()));

        // === Transactional phase (All-or-nothing buffering) ===
        while let Some((depth, event)) = event_queue.pop_front() {
            if depth > MAX_CASCADE_DEPTH {
                return Err(DispatchError::CascadeTooDeep { depth });
            }
            if staging_buffer.len() + 1 > MAX_EVENTS_PER_COMMAND {
                return Err(DispatchError::EventBudgetExceeded);
            }

            for handler in self.transactional_matching(&event) {
                let DeliveryMode::Transactional { can_emit_follow_up, .. } = handler.mode()
                    else { unreachable!() };

                let mut ctx = HandlerContext {
                    repo: &*self.repo,
                    event_store: &*self.event_store,
                    shared: &mut shared,
                    prior_events: &prior_events,
                    aggregate_key: aggregate.clone(),
                };
                let result = handler.handle(&event, &mut ctx)?;

                if can_emit_follow_up {
                    for follow_up in result.follow_up_events {
                        event_queue.push_back((depth + 1, follow_up));
                    }
                } else {
                    debug_assert!(result.follow_up_events.is_empty());
                }
            }

            staging_buffer.push(event.clone());
            prior_events.push(event);
        }

        // === Commit phase ===
        for event in &staging_buffer {
            self.event_store.append(event).await?;
        }

        // === Inline phase (best-effort) ===
        for event in &staging_buffer {
            let arc = Arc::new(event.clone());
            for handler in self.inline_matching(&arc) {
                let mut ctx = HandlerContext {
                    repo: &*self.repo,
                    event_store: &*self.event_store,
                    shared: &mut shared,
                    prior_events: &prior_events,
                    aggregate_key: aggregate.clone(),
                };
                if let Err(e) = handler.handle(&arc, &mut ctx) {
                    tracing::warn!(handler = handler.name(), ?e, "inline handler failed");
                }
            }
        }

        // === Fanout phase ===
        if self.broadcast_enabled {
            for event in staging_buffer {
                let _ = self.broadcast.send(Arc::new(event));
            }
        }

        Ok(CommandOutput::from(shared))
    }
}
```

**핵심 불변 조건**
- Transactional 실패 → staging_buffer 폐기 → 어떤 이벤트도 외부에 노출 안 됨
- Commit 성공 이후 Inline/Fanout 실패는 커맨드 성공에 영향 없음
- 같은 커맨드에서 발행된 모든 이벤트는 **같은 aggregate_key** 를 물고 있음 (SceneTask 경계 보장)

### 5.2 SceneTask (Scene 당 tokio task)

```rust
// src/application/director/scene_task.rs

pub struct SceneTask<R: MindRepository> {
    pub scene_id: SceneId,
    cmd_rx: mpsc::Receiver<Command>,
    dispatcher: Arc<CommandDispatcher<R>>,  // 공유 Dispatcher
    dialogue: Option<DialogueAgent<R, RigChatAdapter>>,  // 이 Scene의 LLM 세션
}

impl<R: MindRepository + 'static> SceneTask<R> {
    pub fn spawn(
        scene_id: SceneId,
        dispatcher: Arc<CommandDispatcher<R>>,
        dialogue_factory: impl FnOnce() -> DialogueAgent<R, RigChatAdapter>,
    ) -> mpsc::Sender<Command> {
        let (tx, rx) = mpsc::channel(32);
        let task = SceneTask {
            scene_id: scene_id.clone(),
            cmd_rx: rx,
            dispatcher,
            dialogue: Some(dialogue_factory()),
        };
        tokio::spawn(task.run());
        tx
    }

    async fn run(mut self) {
        tracing::info!(scene = %self.scene_id, "scene task started");
        while let Some(cmd) = self.cmd_rx.recv().await {
            if let Err(e) = self.handle_command(cmd).await {
                tracing::error!(scene = %self.scene_id, ?e, "command failed");
            }
        }
        tracing::info!(scene = %self.scene_id, "scene task ended");
    }

    async fn handle_command(&mut self, cmd: Command) -> Result<(), DispatchError> {
        match cmd {
            Command::DialogueTurn { utterance, .. } => {
                // LLM 호출이 필요한 커맨드는 DialogueAgent 경유
                if let Some(dialogue) = &mut self.dialogue {
                    let _response = dialogue.turn(&self.scene_id.to_string(), utterance, None, None).await?;
                    // DialogueAgent 내부가 Dispatcher로 이벤트 발행함
                }
                Ok(())
            }
            other => {
                self.dispatcher.dispatch(other).await.map(|_| ())
            }
        }
    }
}
```

**단일 writer 보장**: `while let Some(cmd)` 루프가 동기 순차라 이 Scene의 커맨드는 인터리브되지 않음. 다른 Scene은 각자 task라 병렬.

**Dispatcher 공유 vs Scene별**: Dispatcher는 **하나 공유**. 이유:
- 핸들러 등록이 Director 시작 시 한 번만 이뤄지면 충분
- Dispatcher 자체는 `&self` API — 동시 호출 안전
- Scene별 Dispatcher로 하면 핸들러 등록이 Scene 수만큼 복제됨

Dispatcher는 stateless 처리 엔진, **상태는 `HandlerShared`가 커맨드마다 새로**.

### 5.3 Director (Scene 수명 관리)

```rust
// src/application/director/mod.rs

pub struct Director<R: MindRepository> {
    dispatcher: Arc<CommandDispatcher<R>>,
    bus: broadcast::Sender<Arc<DomainEvent>>,
    scenes: RwLock<HashMap<SceneId, mpsc::Sender<Command>>>,
    repo: Arc<R>,
    // 필요 시 ConversationPort factory 등
}

impl<R: MindRepository + 'static> Director<R> {
    pub fn new(dispatcher: Arc<CommandDispatcher<R>>, bus: broadcast::Sender<Arc<DomainEvent>>, repo: Arc<R>) -> Self {
        Self { dispatcher, bus, scenes: RwLock::new(HashMap::new()), repo }
    }

    pub async fn start_scene(&self, scene: Scene, chat: RigChatAdapter) -> Result<(), DirectorError> {
        let scene_id = scene.id().clone();
        self.repo.save_scene(scene)?;

        let dialogue_factory = || {
            DialogueAgent::new(
                self.dispatcher.clone(),
                chat,
                Arc::new(LocaleFormatter::korean()),
            )
        };

        let tx = SceneTask::spawn(scene_id.clone(), self.dispatcher.clone(), dialogue_factory);
        self.scenes.write().await.insert(scene_id, tx);
        Ok(())
    }

    pub async fn send(&self, scene_id: &SceneId, cmd: Command) -> Result<(), DirectorError> {
        let scenes = self.scenes.read().await;
        let tx = scenes.get(scene_id).ok_or(DirectorError::SceneNotFound)?;
        tx.send(cmd).await.map_err(|_| DirectorError::SceneChannelClosed)?;
        Ok(())
    }

    pub async fn end_scene(&self, scene_id: &SceneId) -> Result<(), DirectorError> {
        // tx drop → SceneTask의 recv()가 None → 루프 종료
        self.scenes.write().await.remove(scene_id);
        Ok(())
    }

    pub async fn active_scenes(&self) -> Vec<SceneId> {
        self.scenes.read().await.keys().cloned().collect()
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<Arc<DomainEvent>> {
        self.bus.subscribe()
    }
}
```

### 5.4 EventBus (단일 broadcast)

이미 구현되어 있음. 변화 없음 — B안 이행은 Dispatcher 내부의 `broadcast.send()` 호출 경로만 재배치.

Fanout 구독자는 `director.subscribe_events()`로 `broadcast::Receiver` 획득 후 자기 task에서 소비.

```rust
// MemoryAgent 예시
let mut rx = director.subscribe_events();
tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        memory_agent.index(&event).await;
    }
});
```

Scene별 순서가 필요한 Fanout 구독자는 스스로 demultiplex:

```rust
let mut per_scene: HashMap<SceneId, VecDeque<DomainEvent>> = HashMap::new();
while let Ok(event) = rx.recv().await {
    if let AggregateKey::Scene(sid) = event.aggregate_key() {
        per_scene.entry(sid).or_default().push_back((*event).clone());
    }
}
```

---

## 6. 핸들러 모듈 레이아웃

### 6.1 priority 모듈 (개념 6)

```rust
// src/application/command/priority.rs

pub mod transactional {
    /// 감정 평가 — 가장 먼저.
    pub const EMOTION_APPRAISAL: i32 = 10;

    /// 자극 적용 (PAD → 감정 변동).
    pub const STIMULUS_APPLICATION: i32 = 15;

    /// 가이드 생성 — 감정 평가/자극 이후.
    /// **의존:** HandlerShared.emotion_state
    pub const GUIDE_GENERATION: i32 = 20;

    /// 관계 갱신 — Scene/Beat 종료 시.
    pub const RELATIONSHIP_UPDATE: i32 = 30;

    /// 감사 로그.
    pub const AUDIT: i32 = 90;
}

pub mod inline {
    pub const EMOTION_PROJECTION: i32 = 10;
    pub const RELATIONSHIP_PROJECTION: i32 = 20;
    pub const SCENE_PROJECTION: i32 = 30;
}

#[cfg(test)]
mod invariants {
    use super::*;

    #[test]
    fn emotion_before_guide() {
        assert!(transactional::EMOTION_APPRAISAL < transactional::GUIDE_GENERATION);
    }

    #[test]
    fn stimulus_before_guide() {
        assert!(transactional::STIMULUS_APPLICATION < transactional::GUIDE_GENERATION);
    }

    #[test]
    fn audit_is_last() {
        assert!(transactional::AUDIT > transactional::RELATIONSHIP_UPDATE);
    }
}
```

### 6.2 Transactional 핸들러

| 핸들러 | priority | follow_up | 이벤트 관심 |
|---|---|---|---|
| `EmotionAgent` | 10 | yes | `AppraiseRequested` |
| `StimulusAgent` | 15 | yes (BeatTransitioned 발행 가능) | `StimulusApplyRequested` |
| `GuideAgent` | 20 | no | `EmotionAppraised`, `StimulusApplied` |
| `RelationshipAgent` | 30 | no | `SceneEnded`, `BeatTransitioned` |
| `AuditHandler` | 90 | no | `All` (디버그 로그) |

### 6.3 Inline 핸들러 (Projection)

| 핸들러 | priority | 이벤트 관심 |
|---|---|---|
| `EmotionProjection` | 10 | `EmotionAppraised`, `StimulusApplied` |
| `RelationshipProjection` | 20 | `RelationshipUpdated` |
| `SceneProjection` | 30 | `SceneStarted`, `BeatTransitioned`, `SceneEnded` |

### 6.4 Fanout 구독자

| 구독자 | 활성 feature | 목적 |
|---|---|---|
| `MemoryAgent` | `embed` | 대화·관계 이벤트를 RAG에 인덱싱 |
| `StoryAgent` (신규) | 본 로드맵 Phase 5 | 서사 방향 판단 |
| `SummaryAgent` (신규) | 본 로드맵 Phase 8 | LLM 컨텍스트 압축 |
| `SseEventBridge` | `mind-studio` | UI로 실시간 push |

---

## 7. 테스트 도구

### 7.1 DispatcherBuilder (개념 7)

```rust
pub struct DispatcherBuilder<R: MindRepository> {
    repo: Arc<R>,
    event_store: Arc<dyn EventStore>,
    bus: broadcast::Sender<Arc<DomainEvent>>,
    handlers: Vec<Arc<dyn EventHandler>>,
    broadcast_enabled: bool,
}

impl<R: MindRepository> DispatcherBuilder<R> {
    pub fn new(repo: Arc<R>, event_store: Arc<dyn EventStore>) -> Self { ... }

    pub fn for_production(repo: Arc<R>, event_store: Arc<dyn EventStore>) -> Self {
        Self::new(repo, event_store)
            .register(Arc::new(EmotionAgent::default()))
            .register(Arc::new(StimulusAgent::default()))
            .register(Arc::new(GuideAgent::default()))
            .register(Arc::new(RelationshipAgent::default()))
            .register(Arc::new(EmotionProjection::new()))
            .register(Arc::new(RelationshipProjection::new()))
            .register(Arc::new(SceneProjection::new()))
    }

    pub fn for_test(repo: Arc<R>, event_store: Arc<dyn EventStore>) -> Self {
        Self::new(repo, event_store).disable_fanout()
    }

    pub fn register(mut self, handler: Arc<dyn EventHandler>) -> Self { ... }
    pub fn disable_fanout(mut self) -> Self { ... }
    pub fn with_event_recorder(mut self) -> (Self, EventRecorder) { ... }

    pub fn build(self) -> CommandDispatcher<R> { ... }
}
```

### 7.2 EventRecorder

L3 엔드투엔드 테스트용 Fanout subscriber — 이벤트를 수집해 `drain().await`로 관찰.

### 7.3 HandlerTestHarness

L1 단위 테스트용 — Dispatcher 없이 `HandlerContext`를 수동 조립해 핸들러만 호출.

### 7.4 MockConversationPort

scripted 응답을 반환하는 in-memory `ConversationPort` 구현. llama-server 없이 L3 테스트.

### 7.5 다중 Scene 테스트 유틸

```rust
pub struct MultiSceneTestHarness<R: MindRepository> {
    pub director: Director<R>,
    pub recorder: EventRecorder,
}

impl<R: MindRepository> MultiSceneTestHarness<R> {
    pub async fn start_scenes(&mut self, scenes: Vec<Scene>) { ... }
    pub async fn send_to(&self, scene: &SceneId, cmd: Command) { ... }
    pub async fn assert_scene_events(&self, scene: &SceneId, expected: &[EventKind]) { ... }
}
```

`aggregate_key`로 recorder의 이벤트를 Scene별로 분리해 검증. "두 Scene이 동시 실행 시 각 Scene의 이벤트 순서가 보존되는가" 같은 검증에 사용.

---

## 8. 이행 로드맵 — Stage B0 ~ B5

### Stage B0 — 새 타입 정의 (선행 준비) ✅ 완료

**목표:** 새 API의 뼈대 타입을 도입하되 어디서도 사용하지 않음.

**작업:**
- [x] `src/application/command/handler_v2.rs` 생성
  - `EventHandler` trait, `DeliveryMode`, `HandlerInterest`
  - `EventHandlerContext`, `HandlerShared`, `HandlerResult`, `HandlerError`
- [x] `src/application/command/priority.rs` 생성
  - `transactional`, `inline` 모듈 + 불변 조건 테스트
- [x] `AggregateKey` enum + `DomainEvent::aggregate_key()` + `Command::aggregate_key()`
- [x] `cargo test` 전부 통과

**Acceptance:** ✅ 컴파일 OK, 기존 테스트 영향 없음.

**실제 변경**: `AggregateKey`는 plan의 `application::command::aggregate_key`가 아닌 `domain::aggregate`에 배치 (도메인 계층 역방향 import 방지).

### Stage B1 — 기존 Agent가 EventHandler 추가 구현 ✅ 완료

**목표:** EmotionAgent, StimulusAgent(신규 분리), GuideAgent, RelationshipAgent가 새 트레이트 구현.

**작업:**
- [x] 각 Agent의 기존 로직을 v1 메서드로 유지, v2 `impl EventHandler`는 병렬 구현
- [x] `impl EventHandler for EmotionAgent` / `StimulusAgent` / `GuideAgent` / `RelationshipAgent`
- [x] 각 Agent에 대한 L1 단위 테스트 작성 (`HandlerTestHarness`)
- [x] 현재 `apply_stimulus`의 Beat 전환 로직을 `StimulusAgent`로 분리 + BeatTransitioned follow_up 발행 검증
- [x] `EventPayload::AppraiseRequested` · `StimulusApplyRequested` variant 추가 (v2 커맨드 진입용)

**Acceptance:** ✅ 모든 Agent가 새 트레이트 구현 + L1 12 테스트 통과 + 기존 통합 테스트 무영향.

### Stage B2 — Projection을 EventHandler로 포팅 ✅ 완료

**목표:** EmotionProjection, RelationshipProjection, SceneProjection이 Inline mode의 EventHandler로 동작.

**작업:**
- [x] Wrapper 방식 채택 — `EmotionProjectionHandler` / `RelationshipProjectionHandler` / `SceneProjectionHandler`가 `Arc<Mutex<T>>`로 기존 Projection을 감싸 `impl EventHandler`
- [x] 9 L1 테스트 추가 (3 × 3)
- [x] ProjectionRegistry storage 전환은 B3의 Dispatcher `inline_handlers`로 이관 (B5.3에서 완전 제거)

**Acceptance:** ✅ 쿼리 일관성 기존 테스트 전수 통과.

### Stage B3 — Dispatcher에 shadow v2 경로 추가 ✅ 완료

**목표:** `dispatch_v2()` 구현. flag로 v1/v2 선택.

**작업:**
- [x] `dispatch_v2()` = §5.1의 실행 루프 (BFS + MAX_CASCADE_DEPTH=4 + MAX_EVENTS_PER_COMMAND=20)
- [x] `CommandDispatcher`에 `shadow_v2: bool` 플래그 (기본값 false, observable effect는 B4 Director 통합 시점으로 유예)
- [x] `with_default_handlers()` — 기본 5 transactional + 3 inline 일괄 등록
- [x] `register_transactional(h)` / `register_inline(h)` — priority 기준 자동 정렬
- [x] v1/v2 parallel run 테스트 — 결과 이벤트 필터링(초기 *Requested + 자동 GuideGenerated 제외) 후 kind 시퀀스 동등성 검증
- [x] `apply_shared_to_repository` — transactional phase 종료 후 `ctx.shared`의 save/clear를 repo에 반영 (plan §5.1 외 보완 단계)

**Acceptance:** ✅ Parallel run 테스트 통과, 전체 기존 테스트 무영향.

### Stage B4 — Director + SceneTask 도입 & v2 기본값 전환

**Session 1 ✅ 완료** — v2 경로 완성 (6 커맨드)
- [x] 4 추가 *Requested variant: `GuideRequested`, `RelationshipUpdateRequested`, `DialogueEndRequested`, `SceneStartRequested`(prebuilt_scene 포함)
- [x] GuideAgent/RelationshipAgent 확장 (interest + shared→repo fallback)
- [x] `SceneAgent` 신규 (priority::SCENE_START = 5, Scene 빌드 + 초기 focus appraise)
- [x] `HandlerShared` destructive signals: `clear_emotion_for` / `clear_scene`
- [x] dispatch_v2가 6 Command 전부 지원

**Session 2 ✅ 완료** — Director + multi-scene facade
- [x] `SceneId { npc_id, partner_id }` composite key 도메인 타입
- [x] `InMemoryRepository` 내부 refactor: `scene: Option<Scene>` → `scenes: HashMap<SceneId, Scene>` + `last_scene_id` 트래커
- [x] `get_scene_by_id` · `list_scene_ids` · `clear_scene_by_id` inherent 메서드
- [x] `src/application/director/mod.rs` — `Director` 구현 (start_scene/end_scene/dispatch_to/active_scenes)
- [x] 다중 Scene E2E 테스트 11개 (lifecycle/mismatch/격리)

**Session 3 Option A ✅ 완료** — multi-scene 정확성
- [x] `BeatTransitioned` payload에 `partner_id` 필드 추가 → AggregateKey::Scene 승격
- [x] `SceneStore::get_scene_by_id` default method trait 확장 (backward compat)
- [x] `StimulusAgent` Beat 분기가 `ctx.repo.get_scene_by_id(&scene_id)` 사용 → multi-scene 오동작 수정
- [x] `RelationshipAgent::handle_beat_transition`의 `ctx.repo.get_scene()` fallback 제거
- [x] 회귀 가드 테스트 `beat_transition_in_scene_a_updates_scene_a_relationship_not_scene_b`

**Session 3 Option B-Mini ✅ 완료** — Mind Studio Director 통합 (shadow)
- [x] `AppState.director_v2: Arc<Mutex<Director<InMemoryRepository>>>` 필드 (v1 AppState와 분리된 월드)
- [x] `/api/v2/scenes` 시리즈 7 엔드포인트
- [x] `AppError::Director(DirectorError)` / `V2Dispatch(DispatchV2Error)` 강타입 + variant별 HTTP 매핑 (404/400/409/500)
- [x] 7 integration 테스트

**Session 4+ ⏳ 이연**
- [ ] async dispatch_v2 + 실제 tokio SceneTask (mpsc+broadcast) — 필요 시점에 별도 세션
- [ ] Mind Studio v1 AppState · v2 Director Repository 통합 (Option B-Medium)
- [ ] `shadow_v2: true` 기본값 전환 (Director가 default가 되는 시점)

**Acceptance (Session 3까지 기준):** ✅
- 기존 통합 테스트 전수 통과 (v1/v2 양 경로)
- 다중 Scene E2E 테스트 통과
- Mind Studio v2 shadow 엔드포인트에서 다중 Scene 조작 가능

### Stage B5 — 구 API deprecated & 제거

**목표:** v1 경로 완전 제거.

**작업 (단계별 PR):**
- **B5.1 ✅ 완료**: `Pipeline`/`PipelineState`/`PipelineStage`, `Projection` trait/`ProjectionRegistry`, `CommandDispatcher::dispatch/execute_pipeline/register_projection/with_projections/projections()`, `handler::HandlerContext`/`HandlerOutput`, Agent v1 메서드 (`handle_appraise`/`handle_stimulus`/`handle_generate`/`handle_update`/`handle_end_dialogue`), `EventAwareMindService` 전체에 `#[deprecated(since = "0.2.0", note = "v0.3.0에서 제거 예정")]` 마킹. 내부 호출자 · 테스트 파일 상단에 `#![allow(deprecated)]` 추가. `cargo check` warning 0건.
- **B5.2 ⏳ 대기**: 내부 호출자(DialogueAgent, Mind Studio scenario handlers 등) v2 경로로 마이그레이션
- **B5.3 ⏳ 대기**: `Pipeline`, `PipelineState`, v1 `Projection` trait, v1 `dispatch`, `EventAwareMindService`, `handler` (v1) 모듈 통째로 삭제
- **B5.4 ⏳ 대기**: `shadow_v2` 플래그 제거 (v2만 존재)

**Acceptance (B5 최종):**
- 코드베이스에 `Pipeline` 구조체 참조 0건
- `cargo clippy -- -D warnings` 통과
- 라이브러리 버전 0.3.0 bump (breaking change)

---

## 9. 다중 Scene 관련 특화 작업

### 9.1 Command에 `scene_id` 명시

현재 일부 Command는 scene_id를 implicit하게 처리(활성 Scene 가정). 다중 Scene 환경에서는 **모든 Scene-scoped Command에 scene_id 필수**:

```rust
pub enum Command {
    Appraise { npc: NpcId, partner: NpcId, scene_id: Option<SceneId>, /* ... */ },
    ApplyStimulus { npc: NpcId, scene_id: SceneId, /* ... */ },  // Scene 안에서만 의미
    DialogueTurn { scene_id: SceneId, utterance: String, /* ... */ },
    StartScene { scene: Scene },
    EndScene { scene_id: SceneId, /* ... */ },
    // ...
}
```

**Stage B4에서 Command 시그니처 확장**. 기존 호출자는 `scene_id: None`으로 호환 유지.

### 9.2 llama-server slot 전략

llama-server는 `--parallel N`으로 N개 slot 병렬 inference 지원.

**권장 설정:**
- `--parallel 4` (활성 Scene 수 + 여유)
- Scene 수 > slot 수 시 llama-server가 자동 큐잉

**Mind Studio 모니터링:**
- 이미 있는 `/api/llm/slots`로 slot 상태 관찰
- Scene 시작 시 idle slot 부족하면 경고 (UI 배너)

### 9.3 플레이어 Scene 우선순위

배경 Scene들이 플레이어 반응을 방해하지 않도록:

- 배경 Scene의 turn 간격에 `tokio::time::sleep(Duration::from_millis(500~2000))` 삽입
- 또는 Director에 "플레이어 커맨드 인입 시 배경 Scene들을 일시 중지" 옵션

**Stage B4에서 간단한 sleep 방식으로 시작**. 복잡한 스케줄링은 Phase 5+에서 StoryAgent가 결정.

### 9.4 Scene 종료 및 리소스 정리

Scene 종료 시 정리해야 할 것:
- SceneTask의 mpsc::Sender drop → task 자연 종료
- DialogueAgent의 LLM 세션 end_session
- (RelationshipAgent가 EndDialogue 커맨드 처리로) 관계 갱신
- Scene의 final 이벤트 `SceneEnded` 발행

순서:
```rust
director.send(scene_id, Command::EndScene { significance }).await?;
// → RelationshipAgent가 관계 갱신
// → SceneEnded 이벤트 발행
// → (MemoryAgent가 RAG 인덱싱)
director.end_scene(scene_id).await?;
// → mpsc::Sender drop → SceneTask 종료
// → DialogueAgent drop → LLM 세션 종료
```

---

## 10. 위험과 완화

| 위험 | 가능성 | 영향 | 완화 |
|---|---|---|---|
| v1/v2 parallel run에서 미묘한 차이 | 중 | 중 | 의미적 동등성 비교 함수 사용, 이벤트 id·timestamp 제외 |
| 다중 Scene 시 repo race | 중 | 고 | `InMemoryRepository`의 내부 락이 이미 Send+Sync. 각 연산은 atomic. SceneTask가 커맨드 단위 단일 writer 보장 |
| llama-server slot 부족 | 중 | 중 | `/api/llm/slots` 모니터링 + UI 경고 |
| 배경 Scene 폭주 (끝없는 turn) | 저 | 중 | Scene마다 "최대 턴 수" 상한 + sleep |
| DialogueAgent.turn()이 `&mut self`인데 SceneTask 내부라 괜찮음 | - | - | SceneTask가 `self.dialogue: Option<DialogueAgent>`로 소유. 동시 호출 불가능 |
| Fanout subscriber lag | 중 | 저 | 이미 있는 `subscribe_with_lag()` + EventStore replay 패턴 |
| 이행 기간의 심리적 부담 | 고 | 중 | 각 Stage를 독립 PR로 체크박스 관리 |

---

## 11. 타임라인 (솔로 개발자 기준)

**일일 작업량:** 1~2시간 가정, 주말 집중 가능 시 주 8~10시간.

| Stage | 작업량 | 일정 |
|---|---|---|
| B0 | 3~4시간 | Week 1 |
| B1 | 1~2주 | Week 1~2 |
| B2 | 3~5일 | Week 3 |
| B3 | 1~2주 | Week 4~5 |
| B4 | 2~3주 | Week 6~8 |
| B5 | 1주 | Week 9 |

**총 예상 기간:** 약 2~2.5개월. 중간 휴지기 포함하면 3개월.

각 Stage는 **독립 mergeable**. 중간에 다른 기능 개발이나 일상 작업 끼워 넣어도 진행 가능.

---

## 12. 착수 전 체크리스트

- [ ] 이 문서를 검토하고 불명확한 부분 정리
- [ ] `docs/architecture/system-design-eventbus-cqrs.md`의 해당 섹션 업데이트 계획 수립
- [ ] 테스트 시나리오 목록 확정 (Stage B3의 parallel run용)
- [ ] Stage B4의 다중 Scene 엔드투엔드 시나리오 정의
  - 예: "동시 2 Scene에서 NPC A와 NPC B가 각자 대화, 5턴씩 진행, 감정 상태 최종 검증"
- [ ] llama-server `--parallel 4` 실행 확인

## 13. 참고 문서

- [`unified-event-protocol-analysis.md`](unified-event-protocol-analysis.md) — B안 개념 1~8 이론적 분석
- [`system-design-eventbus-cqrs.md`](system-design-eventbus-cqrs.md) — 현재 EventBus/CQRS 아키텍처 (B안 이행 후 업데이트 필요)
- [`architecture-v2.md`](architecture-v2.md) — 전체 아키텍처 v2
- [`frontend-architecture.md`](frontend-architecture.md) — Mind Studio UI 구조 (Stage B4에서 Scene 목록 UI 고려)

---

## 부록 A. EventHandler 구현 예시 — EmotionAgent

```rust
// src/application/command/agents/emotion_agent.rs

pub struct EmotionAgent {
    engine: AppraisalEngine,
}

impl EventHandler for EmotionAgent {
    fn name(&self) -> &'static str { "EmotionAgent" }

    fn interest(&self) -> HandlerInterest {
        HandlerInterest::Kinds(vec![
            EventKind::AppraiseRequested,
        ])
    }

    fn mode(&self) -> DeliveryMode {
        DeliveryMode::Transactional {
            priority: priority::transactional::EMOTION_APPRAISAL,
            can_emit_follow_up: true,
        }
    }

    fn handle(
        &self,
        event: &DomainEvent,
        ctx: &mut HandlerContext<'_>,
    ) -> Result<HandlerResult, HandlerError> {
        let EventPayload::AppraiseRequested { npc_id, partner_id, scene_id, situation } = &event.payload
            else { return Ok(HandlerResult::default()) };

        let npc = ctx.repo.get_npc(npc_id)?;
        let partner = ctx.repo.get_npc(partner_id)?;
        let relationship = ctx.repo.get_relationship(npc_id, partner_id).ok();

        let emotion_state = self.engine.appraise(&npc, &partner, relationship.as_ref(), situation);

        // 타입 안전 state 전파
        ctx.shared.emotion_state = Some(emotion_state.clone());
        ctx.shared.relationship = relationship;

        // repo 쓰기
        ctx.repo.save_emotion_state(npc_id, &emotion_state)?;

        // follow_up: 감정 평가 완료 이벤트 발행
        Ok(HandlerResult {
            follow_up_events: vec![
                DomainEvent::emotion_appraised(npc_id, scene_id.as_ref(), &emotion_state),
            ],
        })
    }
}
```

## 부록 B. 다중 Scene 테스트 예시

```rust
#[tokio::test]
async fn two_scenes_progress_independently() {
    let harness = MultiSceneTestHarness::new().await;

    harness.start_scenes(vec![
        scene_a_chungang_encounter(),
        scene_b_background_dialogue(),
    ]).await;

    // Scene A와 B에 각각 커맨드 송신 (동시)
    let (r_a, r_b) = tokio::join!(
        harness.send_to(&"scene_a".into(), Command::appraise_a()),
        harness.send_to(&"scene_b".into(), Command::appraise_b()),
    );
    r_a.unwrap(); r_b.unwrap();

    // 각 Scene의 이벤트가 순서대로 기록되었는지 검증
    harness.assert_scene_events(&"scene_a".into(), &[
        EventKind::AppraiseRequested,
        EventKind::EmotionAppraised,
        EventKind::GuideGenerated,
    ]).await;

    harness.assert_scene_events(&"scene_b".into(), &[
        EventKind::AppraiseRequested,
        EventKind::EmotionAppraised,
        EventKind::GuideGenerated,
    ]).await;

    // Scene A의 이벤트가 Scene B를 오염시키지 않음
    harness.assert_no_cross_contamination().await;
}
```
