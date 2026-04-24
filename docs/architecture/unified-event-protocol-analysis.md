# Unified Event Protocol 분석 — Pipeline + EventBus 통합 가능성

**작성일:** 2026-04-19
**상태:** 설계 검토 / ADR 후보
**대상:** `CommandDispatcher` + `Pipeline` (Tier 1) + `EventBus` (Tier 2)

---

## 1. 문제 정의

> Tier 1 Pipeline과 Tier 2 EventBus를 EventBus로 통합할 수 있는가?
> 동기 실행이 필요한 건 구독자가 순차 처리하도록 이벤트 프로토콜을 정의할 수 있지 않을까?

현재 구조는 2계층이다.

```
CommandDispatcher
  ├─ Pipeline (Tier 1)     : 순차 동기, PipelineState 전파, 실패 시 중단
  ├─ L1 Projection Registry: emit() 내부 sync, 쿼리 일관성
  └─ EventBus  (Tier 2)    : tokio::broadcast fan-out, Stream API, 비동기 독립 소비
```

이를 "하나의 Bus + DeliveryMode 프로토콜"로 통합할 수 있는지 평가한다.

## 2. 결론 (먼저 말하기)

**기술적으로 가능하지만, 완전 통합은 권장하지 않는다.**
대신 **"공통 프로토콜 정의 + 내부 실행은 분리 유지"**(B안)가 현실적 절충안이다.

- A안 완전 통합: 타입 안전성 손실 · 에러 경계 모호 · 추론 복잡도 증가
- **B안 프로토콜만 통합 ★ 권장**: 등록·관측은 단일화, 실행은 모드 분기
- C안 현상 유지: 추상화 비용이 명료성 이득보다 클 때 선택

## 3. Pipeline이 "정렬된 구독자"로 환원되지 않는 이유

`Pipeline` stage는 동일한 이벤트에 반응하는 복수 핸들러가 아니라, **서로의 출력에 데이터 흐름으로 의존**한다.

```
Stage 1 (EmotionPolicy)  →  emotion_state: EmotionState
Stage 2 (GuidePolicy)    ←  emotion_state를 읽어 ActingGuide 생성
```

단순 priority-sort만으로는 아래가 깨진다.

1. **타입 안전한 상태 전파**: 현재 `PipelineState.emotion_state: EmotionState`는 컴파일 타임 보장. 이벤트 페이로드로 옮기면 소비자가 캐스팅/옵셔널 해제를 반복.
2. **암묵적 계약의 드러남**: "Stage 2는 Stage 1 출력을 필요로 한다"는 계약이 priority 값에 숨음 → 리팩터링 중 실수하기 쉬움.
3. **트랜잭션 경계**: Pipeline은 "전부 성공 or 전부 실패" 원자 단위. 구독자 리스트는 "각자 best-effort"가 기본.

즉 Pipeline = **orchestration**(중앙 지휘), EventBus = **choreography**(각자 춤). 둘을 같은 기계로 끌어올리면 언어가 섞인다.

## 4. 통합 프로토콜 스케치 (만약 한다면)

### 4.1 트레이트

```rust
pub trait EventHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn interest(&self) -> HandlerInterest;     // 구독 이벤트 종류
    fn mode(&self) -> DeliveryMode;
    fn handle(
        &self,
        event: &DomainEvent,
        ctx: &mut HandlerContext<'_>,
    ) -> HandlerResult;
}

pub enum DeliveryMode {
    /// 커맨드 트랜잭션 내부에서 sync 실행. 에러는 커맨드 전체 실패.
    /// Pipeline stage가 여기 해당.
    Transactional {
        priority: i32,            // 낮을수록 먼저
        can_emit_follow_up: bool, // 후속 이벤트를 생성할 수 있는지
    },

    /// Sync이지만 best-effort. 에러는 로그.
    /// L1 Projection, 감사 훅 등.
    Inline { priority: i32 },

    /// 비동기 fan-out. 실패가 생산자에 전파되지 않음.
    /// MemoryProjector, SSE stream 등.
    Fanout,
}

pub struct HandlerContext<'a> {
    pub repo: &'a dyn MindRepository,
    pub event_store: &'a dyn EventStore,
    /// Transactional 핸들러 간 공유 상태 (PipelineState의 후신)
    pub shared: &'a mut HandlerShared,
    /// 이번 커맨드에서 이미 커밋된 이벤트들 (감사용)
    pub prior_events: &'a [DomainEvent],
}

pub struct HandlerResult {
    pub follow_up_events: Vec<DomainEvent>,
}
```

### 4.2 Dispatcher 실행 루프

```rust
fn dispatch(&self, cmd: Command) -> Result<CommandOutput, DispatchError> {
    let mut queue: VecDeque<DomainEvent> = VecDeque::new();
    queue.push_back(cmd.into_initial_event());
    let mut ctx = HandlerContext::new(&self.repo, &self.event_store);

    while let Some(event) = queue.pop_front() {
        // 1. Transactional — priority 순, 실패 시 return
        for h in self.registry.transactional_for(&event) {
            let r = h.handle(&event, &mut ctx)?; // `?`가 트랜잭션 중단
            for ev in r.follow_up_events {
                queue.push_back(ev);
            }
        }

        // 2. EventStore append (순서 보장된 영속화 지점)
        self.event_store.append(&event)?;

        // 3. Inline — best-effort, 에러는 로그만
        for h in self.registry.inline_for(&event) {
            if let Err(e) = h.handle(&event, &mut ctx) {
                tracing::warn!(handler = h.name(), ?e, "inline handler failed");
            }
        }

        // 4. Fanout — broadcast
        self.broadcast.publish(Arc::new(event));
    }

    Ok(ctx.finish())
}
```

이 루프는 현재 2-tier 흐름과 의미적으로 **동일**하다. 바뀐 건 "등록 방식과 트레이트 이름이 하나로 정렬됐다"는 점뿐이다.

## 5. Tradeoff

### 5.1 통합의 장점

| 항목 | 이득 |
|------|------|
| 멘탈 모델 | "모든 것은 EventHandler" — 신규 개발자에게 설명 쉬움 |
| 관측성 | 트레이싱·메트릭·로깅 파이프 하나로 통합 |
| 구성 | scenario별 핸들러 조합을 런타임에 주입 가능 |
| 플러그인 진입점 | 단일 `register(handler)` 호출 |

### 5.2 통합의 비용

| 항목 | 비용 |
|------|------|
| 타입 안전성 | `PipelineState.emotion_state: EmotionState` → `HandlerShared`가 `HashMap<&str, Any>` 류로 전락하기 쉬움. 런타임 캐스팅 지옥 |
| 코드 추론 | "Command::Appraise 처리 순서"가 `priority` 숫자로 분산 → grep만으론 순서 파악 어려움 |
| 에러 경계 | Transactional 내부 `can_emit_follow_up=true`일 때 후속 이벤트의 핸들러도 트랜잭션? 무한 루프 방지는? 규칙이 한 겹 더 필요 |
| 성능 | broadcast 채널 + registry lookup이 직접 함수 호출보다 무거움 (핫 패스 appraise에서 체감 가능) |
| 대체 가치 | 현재 Pipeline은 ~100줄. 교체 비용 대비 이득이 애매 |

### 5.3 현 2-tier가 실제로 주는 보장

- Pipeline: 타입 안전한 state 전파 · 명시적 순서 · 트랜잭션 명확
- L1 Projection: 이벤트 커밋 직후 sync → 쿼리 일관성
- EventBus: runtime-agnostic `Stream` API · at-least-once replay
- 두 계층 사이 경계가 쓰기/읽기 경로를 분리하는 설계적 방어선 역할

## 6. 세 가지 선택지

### A. 완전 통합 — "모든 것은 EventBus 구독자"

모든 것을 `bus.subscribe_with_mode(mode)`로 등록. `Pipeline` 제거.

**부적합 이유:** 타입 안전한 state 전파와 트랜잭션 경계를 잃는다. NPC 엔진처럼 appraise → guide 체인의 **데이터 흐름 보장**이 도메인 핵심인 경우 choreography-only는 약한 선택.

### B. 프로토콜만 통합 — "공통 트레이트, 내부 분리 실행" ★ 권장

- `EventHandler` 트레이트를 공통 정의
- Pipeline stages · L1 Projections · Fanout subscribers 모두 같은 트레이트 구현
- Dispatcher의 **등록 포인트는 하나**, 내부적으론 `mode()`로 분류해 저장
- 실행 경로는 현재와 동일(Pipeline sync → EventStore → Projection sync → broadcast)

외부에서 볼 땐 "단일 Bus", 내부는 2-tier 유지. 멘탈 모델 단순화 + 타입 안전 유지.

```rust
impl EventHandler for EmotionPolicy {
    fn mode(&self) -> DeliveryMode {
        DeliveryMode::Transactional { priority: 10, can_emit_follow_up: true }
    }
}

impl EventHandler for EmotionProjection {
    fn mode(&self) -> DeliveryMode { DeliveryMode::Inline { priority: 100 } }
}

impl EventHandler for MemoryProjector {
    fn mode(&self) -> DeliveryMode { DeliveryMode::Fanout }
}
```

### C. 현 상태 유지

`Pipeline`과 `EventBus`를 명시적으로 다른 API로 남김. 추상화 비용 > 명료성 이득으로 판단될 때.

## 7. B안 도입을 위한 체크리스트

1. **`HandlerShared` 설계**: `PipelineState`를 대체할 타입 — `HashMap<Any>`는 피하고 **struct with Option fields**로 유지
   ```rust
   pub struct HandlerShared {
       pub emotion_snapshot: Option<EmotionState>,
       pub relationship_snapshot: Option<Relationship>,
       pub scene_snapshot: Option<Scene>,
       // ... 알려진 전파 대상만
   }
   ```
   제너릭/Any로 가면 Pipeline의 가치를 날리는 것. 필드 목록은 컴파일러가 지켜준다.

2. **follow_up 이벤트의 전파 깊이 제한**: cycle/폭주 방지 위해 `max_depth` 가드 필요.

3. **priority 숫자는 상수로 관리**:
   ```rust
   pub mod priority {
       pub const EMOTION_APPRAISAL: i32 = 10;
       pub const GUIDE_GENERATION: i32 = 20;
       pub const PROJECTION_EMOTION: i32 = 100;
       // ...
   }
   ```
   매직 넘버 산재 방지. `emotion < guide` 식으로 코드로 관계를 드러낼 것.

4. **테스트 모드**: 특정 DeliveryMode만 활성화한 dispatcher 빌더 제공 — `.without_fanout()`, `.only_transactional()` 같은 것. 단위 테스트에서 broadcast 구독자 설정 안 해도 되도록.

5. **기존 API 호환**: `CommandDispatcher::with_projections()` / `.subscribe()`를 잠깐 유지한 뒤 서서히 `.register(handler)`로 흡수 (Strangler Fig 재사용).

## 8. 언제 B안을 실제로 땡길지

현재는 C안(현상 유지)이 과투자 아님. 아래 신호가 보이면 B안으로 이동 권장.

- Fanout 구독자가 3개 이상(MemoryProjector 외 StoryAgent/SummaryAgent/외부 MCP 브릿지 등)
- Scenario별로 Transactional 핸들러 조합이 달라짐(예: 액션 씬 vs 대화 씬에서 다른 Guide 전략)
- 디버깅 시 "이 Command가 어느 핸들러까지 돌았나"를 한 포인트에서 관측하고 싶다는 요구가 반복됨
- 플러그인/외부 기여자가 핸들러를 주입하는 구조(예: Rhai/Lua 스크립트 핸들러)

## 9. 되돌아볼 포인트

- **백프레셔**: 통합 후에도 Fanout은 여전히 `tokio::broadcast` 기반. 느린 소비자 → lag → replay 패턴은 유지.
- **순서 보장**: Transactional 내부는 priority로 정렬이 안정적. Fanout은 broadcast가 FIFO를 **대체로** 보장하지만 lag 발생 시 replay로 재정렬해야 함 — 이건 현재와 동일.
- **이벤트 스키마 진화**: 통합 후 새 핸들러 추가가 쉬워지는 대신, 이벤트 payload 변경의 파급 범위도 넓어짐. `#[serde(other)]` · non_exhaustive 같은 완충 장치 조기 도입.

## 10. 최종 권장

1. 지금은 **C안 유지** — 2-tier가 명확한 경계를 준다.
2. 통합을 진행한다면 반드시 **B안**. A안(`Pipeline` 삭제 후 모든 것을 broadcast)은 피할 것.
3. B안으로 갈 때도 `HandlerShared`는 struct + Option 필드로 유지해 타입 안전 state 전파를 포기하지 말 것.
4. 통합 전제 조건(8장)이 충족되기 전까진 이 문서를 ADR 초안 상태로 보관.
