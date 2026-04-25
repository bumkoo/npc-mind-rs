# Task — correlation_id 활성화 및 trace 조회 인프라

> **목적.** 본 리포지토리는 `DomainEvent.metadata.correlation_id` 필드와 `CommandDispatcher::commit_staging_buffer` 내 자동 부착 로직을 갖추고 있으나, **발급 트리거(`set_correlation_id`)를 호출하는 코드가 0개**여서 모든 저장 이벤트의 `correlation_id`가 `None` 상태이다. 본 태스크는 (1) `dispatch_v2` 호출 단위로 cid를 자동 발급하고, (2) 동시 호출에 안전한 per-call 전달 구조로 리팩터링하며, (3) cid 기반 trace 조회 인터페이스를 인프라부터 HTTP 엔드포인트까지 완성한다.
>
> **범위.** library core(`src/application/`, `src/domain/`)와 Mind Studio bin(`src/bin/mind-studio/`) 모두. 본 태스크는 read-side-activation과 달리 **library core 수정이 정당한 작업**이다 — correlation_id는 도메인 이벤트의 핵심 메타데이터이므로.
>
> **소요 예상.** library core ~80 LoC + Mind Studio ~60 LoC + 테스트 3~4개.

---

## 1. 배경 — 현재 상태 진단

### 1.1 검증된 사실 (코드 인용)

`src/application/command/dispatcher.rs`:

```rust
pub struct CommandDispatcher<R: MindRepository> {
    correlation_id: Arc<AtomicU64>,         // 인스턴스당 1개 (글로벌 슬롯)
    command_seq: Arc<AtomicU64>,            // 발급기로 쓸 수 있는 카운터
    ...
}

pub fn set_correlation_id(&self, id: u64) { ... }      // ★ 호출자 0개

fn commit_staging_buffer(&self, ...) -> Vec<DomainEvent> {
    for event in staging {
        ...
        if let Some(cid) = self.current_correlation_id() {
            e = e.with_correlation(cid);    // ★ set_correlation_id 호출 안 되면 항상 None
        }
        ...
    }
}
```

전수 검색 결과: `set_correlation_id`를 호출하는 코드는 정의 라인 외에 **존재하지 않음** (`findstr /S /N /I set_correlation_id` 확인).

### 1.2 결과 — 두 가지 손실
- **저장 데이터 손실**: 모든 EventStore 이벤트의 `metadata.correlation_id == None`. 한 dispatch가 만든 이벤트 묶음을 인과적으로 묶을 키가 없다.
- **관찰 가능성 손실**: DeepEval Phase 1 평가의 trace 단위, 디버깅 시 인과 사슬 복원, 분산 로깅 연동 — 모두 cid 기반인데 그 cid가 없음.

### 1.3 추가 이슈 — 글로벌 슬롯의 동시성 위험

`Arc<AtomicU64>`는 dispatcher 인스턴스 공유 슬롯이다. `set` 후 `current_correlation_id()` 사이에 다른 dispatch 호출이 들어와 `set`을 덮어쓰면 cid가 섞인다. 현재는 `dispatch_v2` 첫 줄의 repository mutex가 우연히 직렬화해주지만, **명시적 보증이 아닌 우연한 안전**이다. 다중 Scene 동시 실행으로 가면 더 위태로워진다.

---

## 2. 목표

1. `dispatch_v2`가 호출 단위로 cid를 **자동 발급**한다 (수동 `set_correlation_id` 불필요).
2. cid 전달 구조가 **per-call 로컬 변수**로 바뀐다. 글로벌 `Arc<AtomicU64> correlation_id` 슬롯은 제거.
3. `EventStore` trait에 `get_events_by_correlation(cid)` 메서드 추가, `InMemoryEventStore` 구현.
4. Mind Studio에 `GET /api/projection/trace/:correlation_id` 엔드포인트 추가.
5. cid가 실제로 부착되는지 검증하는 테스트 + 묶음 조회 테스트 추가.

---

## 3. 완료 기준 (Definition of Done)

- [ ] `dispatch_v2(cmd)` 후 반환된 `events: Vec<DomainEvent>`의 모든 항목에서 `event.metadata.correlation_id == Some(_)`.
- [ ] 같은 `dispatch_v2` 호출이 만든 모든 이벤트의 cid가 **서로 같다**.
- [ ] 서로 다른 `dispatch_v2` 호출이 만든 이벤트의 cid가 **서로 다르다** (단조 증가).
- [ ] `EventStore::get_events_by_correlation(cid)` 가 정확히 그 묶음만 반환한다.
- [ ] `GET /api/projection/trace/:cid` 가 timestamp 정렬된 JSON 배열을 반환한다.
- [ ] `cargo test --workspace --all-features` 모두 통과.
- [ ] `cargo clippy --workspace --all-features -- -D warnings` 경고 없음.
- [ ] Read-Side Activation 태스크의 drift 감지 테스트가 여전히 통과 (회귀 없음).

---

## 4. 전제 및 주의사항

### 4.1 Library core 수정이 허용되는 범위
- `src/application/command/dispatcher.rs` — cid 발급/전달 구조 변경 (필수)
- `src/application/event_store.rs` — `EventStore` trait에 메서드 추가, `InMemoryEventStore` 구현 (필수)
- `src/lib.rs` — re-export 추가 시
- 그 외 `src/domain/`, `src/adapter/`, `src/ports.rs`, `src/presentation/`는 **수정 금지**.

### 4.2 호환성 — Breaking change 여부
- `EventStore` trait에 메서드 **추가**: 외부 구현체가 있다면 깨짐. 현재 구현체는 `InMemoryEventStore` 하나뿐이므로 영향 없음. 향후 `SqliteEventStore` 추가 시 같은 메서드 구현 필요. **Breaking change로 간주하고 CHANGELOG에 명시**한다.
- `set_correlation_id` public API **제거**: 외부 호출자 0개이므로 안전. 다만 GitHub 공개를 염두에 두면 deprecation 단계를 거치는 게 안전 — 본 태스크에서는 **`#[deprecated]` 표시 후 내부 동작에서 분리**하고, 실제 제거는 별도 마이너 버전에 미룬다.
- `CommandDispatcher::correlation_id` 필드 **제거**: 내부 필드이므로 외부 영향 없음.

### 4.3 지켜야 할 원칙
- **Per-call 격리**: cid는 `dispatch_v2` 함수의 **로컬 변수**여야 한다. 인스턴스 필드에 저장하지 않는다.
- **단조 증가**: 같은 dispatcher의 두 호출은 cid가 단조 증가해야 한다. 동시 호출 race 없이.
- **0은 예약값**: `correlation_id = 0`은 "미설정"의 sentinel이므로 발급기는 1부터 시작. (현재 `command_seq: AtomicU64::new(1)` 그대로 유지.)
- **부착 위치 단일화**: 이벤트에 cid를 찍는 코드는 `commit_staging_buffer` **단 한 군데**에 머문다.

### 4.4 사전 확인 사항
1. `src/application/command/dispatcher.rs` — `commit_staging_buffer` 시그니처와 `correlation_id`/`command_seq` 필드 확인.
2. `src/application/event_store.rs` — `EventStore` trait의 기존 메서드와 `InMemoryEventStore`의 storage 구조 확인.
3. `src/domain/event.rs` — `DomainEvent::with_correlation`의 정확한 시그니처 확인.
4. `tests/dispatch_v2_test.rs` — 기존 dispatch 테스트가 cid 변화에 의존하는지 확인 (없을 것이지만 회귀 검증 필요).
5. `src/bin/mind-studio/handlers/query.rs` — Read-Side Activation에서 만든 핸들러 패턴 확인 (본 태스크의 trace 핸들러도 같은 패턴).

---

## 5. 작업 명세

### 5.1 작업 1 — `CommandDispatcher`에서 글로벌 cid 슬롯 제거

**파일:** `src/application/command/dispatcher.rs`

**변경:**

```rust
pub struct CommandDispatcher<R: MindRepository> {
    repository: Arc<Mutex<R>>,
    situation_service: SituationService,
    event_store: Arc<dyn EventStore>,
    event_bus: Arc<EventBus>,
    // ★ 제거: correlation_id: Arc<AtomicU64>,
    /// dispatch_v2 호출 단위 cid 발급 + Rumor pending_id 발급에 공용으로 쓰인다.
    /// 1부터 시작하는 단조 증가 카운터. 0은 "미설정" sentinel로 예약.
    command_seq: Arc<AtomicU64>,
    transactional_handlers: Vec<Arc<dyn EventHandler>>,
    inline_handlers: Vec<Arc<dyn EventHandler>>,
}

impl<R: MindRepository> CommandDispatcher<R> {
    pub fn new(...) -> Self {
        Self {
            ...
            // ★ 제거: correlation_id: Arc::new(AtomicU64::new(0)),
            command_seq: Arc::new(AtomicU64::new(1)),  // 기존 그대로
            ...
        }
    }

    // ★ deprecated 처리 (제거는 다음 마이너 버전)
    #[deprecated(note = "cid는 dispatch_v2가 내부에서 자동 발급한다. 이 함수는 더 이상 효과가 없다.")]
    pub fn set_correlation_id(&self, _id: u64) {
        // no-op. 외부 호출자가 있으면 컴파일 경고로 알린다.
    }

    // ★ 제거: fn current_correlation_id(&self) -> Option<u64> { ... }
}
```

### 5.2 작업 2 — `dispatch_v2`에서 cid 자동 발급 + per-call 전달

**파일:** `src/application/command/dispatcher.rs`

**변경:**

```rust
pub async fn dispatch_v2(&self, cmd: Command) -> Result<DispatchV2Output, DispatchV2Error>
where
    R: Send + Sync,
{
    // ★ 호출 단위 cid 발급 — 함수 진입 직후 1회
    //   command_seq는 SeedRumor의 pending_id 발급기와 공유하지만, 발급된 정수는
    //   서로 다른 용도로 분기 사용되므로 충돌 없음.
    let cid = self.command_seq.fetch_add(1, Ordering::SeqCst);
    
    let initial_event = self.build_initial_event(&cmd)?;
    // ... 기존 BFS 루프 그대로 ...

    let committed = self.commit_staging_buffer(&aggregate_key, staging_buffer, cid);  // ★ cid 전달

    // ... 기존 inline phase / fanout 그대로 ...

    Ok(DispatchV2Output { events: committed, shared })
}
```

### 5.3 작업 3 — `commit_staging_buffer` 시그니처 확장

**파일:** `src/application/command/dispatcher.rs`

**변경:**

```rust
fn commit_staging_buffer(
    &self,
    _command_key: &AggregateKey,
    staging: Vec<DomainEvent>,
    cid: u64,                         // ★ 신규 인자
) -> Vec<DomainEvent> {
    let mut committed = Vec::with_capacity(staging.len());
    for event in staging {
        let per_event_id = event.aggregate_key().npc_id_hint().to_string();
        let id = self.event_store.next_id();
        let seq = self.event_store.next_sequence(&per_event_id);
        let mut e = DomainEvent::new(id, per_event_id, seq, event.payload);
        // ★ if let Some(...) 분기 제거 — cid는 항상 부착
        e = e.with_correlation(cid);
        self.event_store.append(&[e.clone()]);
        committed.push(e);
    }
    committed
}
```

### 5.4 작업 4 — `EventStore` trait에 묶음 조회 메서드 추가

**파일:** `src/application/event_store.rs`

**변경:**

```rust
pub trait EventStore: Send + Sync {
    // ... 기존 메서드 ...

    /// 같은 correlation_id로 발생한 이벤트 묶음 조회.
    /// 한 dispatch_v2 호출이 만든 모든 이벤트의 인과 사슬을 반환한다.
    /// 결과는 EventStore에 추가된 순서를 그대로 보존한다 (정렬은 호출자 책임).
    fn get_events_by_correlation(&self, correlation_id: u64) -> Vec<DomainEvent>;
}
```

`InMemoryEventStore` 구현:

```rust
fn get_events_by_correlation(&self, correlation_id: u64) -> Vec<DomainEvent> {
    let store = self.events.read().unwrap();
    store
        .iter()
        .filter(|e| e.metadata.correlation_id == Some(correlation_id))
        .cloned()
        .collect()
}
```

### 5.5 작업 5 — Mind Studio에 trace 조회 엔드포인트 추가

**파일:** `src/bin/mind-studio/handlers/query.rs` (Read-Side Activation에서 만든 파일에 핸들러 추가)

**변경:**

```rust
use npc_mind::DomainEvent;

// ---------------------------------------------------------------------------
// Trace — correlation_id로 묶인 이벤트 사슬 조회
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct TraceView {
    pub correlation_id: u64,
    pub event_count: usize,
    pub events: Vec<DomainEvent>,
}

pub async fn get_trace(
    State(state): State<AppState>,
    Path(correlation_id): Path<u64>,
) -> Result<Json<TraceView>, AppError> {
    let store = state.shared_dispatcher.event_store();
    let mut events = store.get_events_by_correlation(correlation_id);
    
    // 같은 cid 내에서 timestamp 정렬은 의미가 약함 (cascade 깊이가 더 정확함).
    // EventStore의 추가 순서를 그대로 보존 — 시간 흐름과 cascade depth가 함께 반영된다.
    // 명시적 재정렬이 필요하면 호출자가 처리.
    
    Ok(Json(TraceView {
        correlation_id,
        event_count: events.len(),
        events,
    }))
}
```

**라우터 등록** (`src/bin/mind-studio/main.rs` 또는 router 구성 파일):

```rust
.route(
    "/api/projection/trace/:correlation_id",
    get(query::get_trace),
)
```

**참고:** 이 엔드포인트는 `shared_dispatcher`의 EventStore만 조회한다. `/api/v2/*` Director 경로의 별도 인스턴스는 포함하지 않는다 (Read-Side Activation 태스크의 동일 제약).

---

## 6. 테스트 요구사항

### 6.1 cid 부착 검증 (필수)

**위치:** `tests/dispatch_v2_test.rs` 또는 신규 `tests/correlation_id_test.rs`

```rust
#[tokio::test]
async fn dispatch_v2_attaches_correlation_id_to_all_events() {
    let dispatcher = build_test_dispatcher().await;
    
    let result = dispatcher.dispatch_v2(Command::Appraise { /* ... */ }).await.unwrap();
    
    assert!(!result.events.is_empty(), "expected at least one event");
    let first_cid = result.events[0].metadata.correlation_id
        .expect("first event must have correlation_id");
    
    for ev in &result.events {
        assert_eq!(
            ev.metadata.correlation_id,
            Some(first_cid),
            "all events of one dispatch must share the same correlation_id"
        );
    }
}
```

### 6.2 단조 증가 검증 (필수)

```rust
#[tokio::test]
async fn distinct_dispatch_calls_get_distinct_correlation_ids() {
    let dispatcher = build_test_dispatcher().await;
    
    let r1 = dispatcher.dispatch_v2(Command::Appraise { /* ... */ }).await.unwrap();
    let r2 = dispatcher.dispatch_v2(Command::Appraise { /* ... */ }).await.unwrap();
    
    let cid1 = r1.events[0].metadata.correlation_id.unwrap();
    let cid2 = r2.events[0].metadata.correlation_id.unwrap();
    
    assert_ne!(cid1, cid2, "different dispatch calls must have different cids");
    assert!(cid2 > cid1, "cid must be monotonically increasing");
}
```

### 6.3 묶음 조회 검증 (필수)

```rust
#[tokio::test]
async fn event_store_returns_correct_correlation_bundle() {
    let dispatcher = build_test_dispatcher().await;
    
    let result = dispatcher.dispatch_v2(Command::ApplyStimulus { /* ... */ }).await.unwrap();
    let cid = result.events[0].metadata.correlation_id.unwrap();
    let expected_count = result.events.len();
    
    let bundle = dispatcher.event_store().get_events_by_correlation(cid);
    
    assert_eq!(bundle.len(), expected_count, "bundle size mismatch");
    for ev in &bundle {
        assert_eq!(ev.metadata.correlation_id, Some(cid));
    }
}
```

### 6.4 Trace 엔드포인트 smoke test (권장)

`axum::Router`를 빌드해 `/api/projection/trace/:cid`를 호출하고 200 + JSON 형태 검증.

### 6.5 회귀 확인 (필수)

- `cargo test --workspace --all-features` 모든 기존 테스트 통과
- Read-Side Activation의 drift 감지 테스트가 여전히 통과
- 기존 `tests/dispatch_v2_test.rs`의 모든 시나리오가 통과 (cid 변화에 의존하지 않음)

---

## 7. 점진적 도입 순서 (권장)

### 1단계 — 발급/전달 구조만 (library core 변경의 최소 단위)
- 작업 1 (글로벌 슬롯 제거 + deprecated 처리)
- 작업 2 (자동 발급)
- 작업 3 (commit 시그니처 확장)
- 테스트 6.1 + 6.2 통과 확인
- 이 시점에 모든 신규 이벤트가 cid를 갖기 시작한다

### 2단계 — 묶음 조회 인프라
- 작업 4 (`EventStore::get_events_by_correlation` 추가)
- 테스트 6.3 통과 확인
- Library core 변경 완료 — 이후는 Mind Studio bin만 건드림

### 3단계 — HTTP 엔드포인트
- 작업 5 (`/api/projection/trace/:cid`)
- 테스트 6.4 통과 확인

### 4단계 — 문서화
- README의 "CQRS + Event Sourcing" 섹션에 trace 조회 가능성 명시
- `docs/architecture/system-overview.md` §3 "Application" 박스에 `/api/projection/trace` 추가
- `CHANGELOG.md` 또는 신규 `docs/changes/`에 EventStore trait의 메서드 추가를 breaking change로 기록

---

## 8. 체크리스트 (PR 올리기 전)

### Library core
- [ ] `CommandDispatcher`의 `correlation_id: Arc<AtomicU64>` 필드 제거
- [ ] `CommandDispatcher::new`에서 해당 필드 초기화 코드 제거
- [ ] `set_correlation_id`에 `#[deprecated]` 추가, 내부 동작은 no-op
- [ ] `current_correlation_id` private 메서드 제거
- [ ] `dispatch_v2` 진입 시 `command_seq.fetch_add(1, SeqCst)`로 cid 발급
- [ ] `commit_staging_buffer`에 `cid: u64` 인자 추가
- [ ] `commit_staging_buffer` 내 `if let Some(...)` 분기 제거, 항상 `with_correlation` 호출
- [ ] `EventStore` trait에 `get_events_by_correlation` 추가
- [ ] `InMemoryEventStore`에 해당 메서드 구현
- [ ] `src/lib.rs`에 필요시 re-export 추가

### Mind Studio
- [ ] `handlers/query.rs`에 `TraceView` 구조체 + `get_trace` 핸들러 추가
- [ ] `main.rs` 또는 router에 `/api/projection/trace/:correlation_id` 등록

### 테스트
- [ ] `dispatch_v2_attaches_correlation_id_to_all_events` 통과
- [ ] `distinct_dispatch_calls_get_distinct_correlation_ids` 통과
- [ ] `event_store_returns_correct_correlation_bundle` 통과
- [ ] Trace 엔드포인트 smoke test (선택)
- [ ] `cargo test --workspace --all-features` 전체 통과
- [ ] `cargo test --workspace --no-default-features` 통과
- [ ] `cargo clippy --workspace --all-features -- -D warnings` 경고 없음
- [ ] `cargo build` 시 `set_correlation_id` 사용처가 없으므로 deprecated 경고도 안 떠야 함

### 수동 smoke test
- [ ] Mind Studio 기동 → Appraise 실행 → EventStore에 저장된 이벤트의 cid 확인
- [ ] `curl http://localhost:PORT/api/projection/trace/<cid>` 로 묶음 정상 수신
- [ ] 두 번 Appraise 후 두 trace의 cid가 다름을 확인

---

## 9. 관련 파일 (작업 시 참조 경로)

| 역할 | 경로 | 변경 여부 |
|---|---|---|
| Dispatcher 본체 | `src/application/command/dispatcher.rs` | 수정 |
| EventStore trait + InMemory 구현 | `src/application/event_store.rs` | 수정 |
| DomainEvent 정의 (`with_correlation` 메서드) | `src/domain/event.rs` | 읽기 전용 |
| 라이브러리 공개 export | `src/lib.rs` | 필요시 추가 |
| Mind Studio AppState | `src/bin/mind-studio/state.rs` | 읽기 전용 (이미 dispatcher 참조 가능) |
| Mind Studio query 핸들러 | `src/bin/mind-studio/handlers/query.rs` | 핸들러 추가 |
| Mind Studio 라우터 | `src/bin/mind-studio/main.rs` | route 추가 |
| 기존 dispatch 테스트 | `tests/dispatch_v2_test.rs` | 회귀 검증, 신규 테스트 추가 가능 |

---

## 10. Out of Scope / 후속 작업

본 태스크에서 **하지 않는다**:

- **외부 cid 주입 API.** HTTP 헤더(예: `X-Request-Id`)로 들어온 cid를 dispatch에 전달하는 기능은 별도 태스크. 현재는 dispatcher가 자체 발급만 한다.
- **Director_v2의 cid 노출.** `/api/v2/*` 경로는 별도 dispatcher 인스턴스를 쓰며, 그 EventStore의 trace 노출은 별도 태스크 (Read-Side Activation §10과 동일 제약).
- **Cross-process correlation.** 게임 클라이언트·LLM 서버 등 외부 프로세스와 cid를 공유하는 분산 trace 통합 (W3C Trace Context, OpenTelemetry)은 장기 과제.
- **EventStore 영속화.** SQLite로의 영속화 시 `(correlation_id)` 컬럼 인덱스 등은 별도 태스크.
- **Cid 단조 증가의 영속성.** 프로세스 재시작 시 `command_seq`가 1로 리셋되므로 같은 cid가 재사용될 수 있다 (다른 프로세스 세션에서). 영속화 시점에 `MAX(correlation_id) + 1`로 복원하는 로직이 필요하지만 본 태스크 범위 밖.
- **`set_correlation_id` 완전 제거.** 본 태스크는 deprecation까지. 실제 제거는 다음 마이너 버전 (외부 사용자 공개 후 한 번의 사이클 경과).

---

## 11. 완료 후 후속 선언

본 태스크가 완료되면:

### 11.1 README 갱신
"CQRS + Event Sourcing + EventBus" 슬로건 옆에 **실행 증거**가 하나 더 붙는다:

```
- 모든 도메인 이벤트는 dispatch 호출 단위 correlation_id로 묶여 저장됨
- GET /api/projection/trace/:cid 로 한 요청의 전체 인과 사슬을 조회 가능
```

### 11.2 system-overview.md §3 갱신
"Application" 박스에 다음 줄 추가:

```
Trace Endpoint        /api/projection/trace/:correlation_id
                      (한 dispatch_v2 호출의 모든 후속 이벤트 묶음 — 평가/디버깅용)
```

### 11.3 평가 파이프라인 prerequisite 해제
DeepEval Phase 1 평가의 trace 단위가 실제 데이터로 채워지기 시작한다. 다음 후속 태스크의 입력으로 사용 가능:

- LLM-as-a-Judge용 trace 포맷터 (cid 기반으로 묶음 → markdown/JSON 변환)
- Character Fidelity / Scene Appropriateness / Directive Clarity 자동 평가 prototype

### 11.4 죽은 코드 청산 알림
`set_correlation_id`는 deprecation 마킹된 상태로 남는다. **다음 마이너 버전(예: v0.5)**에서 제거할 후보로 `docs/changes/` 또는 changelog에 기록한다.

---

## 12. 위험 요소

### 12.1 동시성 — 현재 안전
`command_seq.fetch_add`는 `AtomicU64` 연산이라 동시 호출에서도 단조 증가가 보장된다. cid를 함수 로컬 변수로 들고 있으므로 dispatch 호출 간 간섭 없음. **현재 설계가 동시 dispatch에 안전하다는 게 본 태스크의 핵심 개선점이다.**

### 12.2 cid 0의 의미
`AtomicU64::new(1)` 시작이므로 발급된 cid는 항상 ≥ 1. 0은 "미설정" sentinel로 예약되며, 어떤 dispatch도 cid 0을 쓰지 않는다. EventStore의 검색에서 `correlation_id == Some(0)`인 이벤트는 존재하지 않아야 한다.

### 12.3 프로세스 재시작 시 cid 충돌 가능성 — 영속화 후 이슈
현재는 InMemory라 재시작 시 EventStore 자체가 비므로 충돌 없음. SQLite 영속화 후엔 **반드시** 시작 시 `SELECT MAX(correlation_id) FROM events`로 카운터를 복원해야 한다. 본 태스크에서는 이 부분을 §10 Out of Scope로 명시하고, 영속화 태스크에서 다룬다.

### 12.4 외부 EventStore 구현체의 영향
현재 `EventStore` trait의 외부 구현체는 0개이므로 영향 없음. 미래의 `SqliteEventStore` 등이 추가될 때 본 트레이트 변경이 영향 미친다는 점을 changelog에 명시.
