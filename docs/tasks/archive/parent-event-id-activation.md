# Task — parent_event_id + cascade_depth 활성화 (cmd 안의 인과 트리)

> **목적.** `correlation-id-activation` 태스크 완료로 "한 cmd가 만든 이벤트 묶음"은 추적 가능해졌으나, 묶음 내부의 **부모-자식 관계**가 EventStore에 보존되지 않는다. dispatcher의 BFS 루프는 처리 중 부모-자식 관계와 cascade 깊이를 임시로 알고 있지만, commit 시 그 정보를 버리고 있다. 본 태스크는 `EventMetadata`에 두 필드(`parent_event_id`, `cascade_depth`)를 추가하고 BFS 루프에서 그 값을 채워 영구 보존한다. 결과적으로 한 cmd 안의 인과 트리를 정확히 시각화·평가할 수 있게 된다.
>
> **선행 조건.** `correlation-id-activation` 태스크 완료. 모든 신규 이벤트에 `metadata.correlation_id`가 부착되어 있어야 한다.
>
> **범위 제한.** 본 태스크는 **cmd 안의 인과만** 다룬다. cmd 사이의 인과(`causation_id`)는 별도 태스크로 분리한다 — 그쪽은 호출자가 명시적으로 인과를 선언해야 하는 설계 결정이 필요하다.
>
> **소요 예상.** library core ~80 LoC + Mind Studio ~30 LoC + 테스트 4~5개.

---

## 1. 배경 — 현재 BFS가 알고 있지만 버리는 정보

### 1.1 검증된 사실 (코드 인용)

`src/application/command/dispatcher.rs` BFS 루프:

```rust
while let Some((depth, event)) = event_queue.pop_front() {
    // depth는 알고 있음 — 그러나 commit 시 버려짐
    
    for handler in self.transactional_handlers.iter() {
        // ... handler.handle(&event, ...) ...
        for follow_up in result.follow_up_events {
            // event(부모) → follow_up(자식) 관계가 여기서 분명함
            // 그러나 follow_up에 부모 정보가 새겨지지 않음
            event_queue.push_back((depth + 1, follow_up));
        }
    }
    staging_buffer.push(event.clone());
}
```

`commit_staging_buffer`에서:

```rust
for event in staging {
    let id = self.event_store.next_id();    // 여기서 id 할당
    let seq = self.event_store.next_sequence(&per_event_id);
    let mut e = DomainEvent::new(id, per_event_id, seq, event.payload);
    e = e.with_correlation(cid);
    // ★ 부모 id, cascade depth는 어디에도 기록되지 않음
}
```

### 1.2 손실되는 두 정보

- **부모 event id**: BFS 루프에서 `follow_up`을 push할 때 부모 `event`의 정체를 알지만, follow_up이 staging_buffer에 들어가는 순간 부모와의 연결이 끊긴다. commit 단계에서 id가 할당되므로, 부모의 id를 follow_up에 새기는 것도 어렵다 — 부모도 아직 commit 전이라 id가 None일 수 있다.
- **cascade depth**: BFS 루프 변수로만 존재. staging_buffer에 들어가는 시점에 사라진다.

### 1.3 결과 — 인과 트리 복원 불가

EventStore에 저장된 데이터로는 다음 질문에 정확히 답할 수 없다:
- "이 GuideGenerated의 직접 부모는 어떤 EmotionAppraised인가?"
- "이 cmd의 인과 트리에서 가장 깊은 분기는 무엇인가?"
- "cascade depth N에 가장 자주 등장하는 이벤트 종류는?"

핸들러 코드 지식으로 추론은 가능하지만, 데이터 자체에서는 불가능하다. DeepEval Phase 1+ 평가에서 trace 트리 구조를 입력으로 쓰려면 이 두 필드가 필수다.

---

## 2. 목표

1. `EventMetadata`에 `parent_event_id: Option<EventId>`, `cascade_depth: u32` 추가.
2. `dispatch_v2`의 BFS 루프가 부모-자식 관계와 cascade depth를 추적하여 commit 시 staging 이벤트에 채운다.
3. **부모 id 발급 시점 문제 해결**: 부모도 commit 전에는 id가 0(미할당)이므로, 부모 id 할당 후 자식의 metadata를 채우는 2-pass 구조가 필요하다.
4. `EventStore` trait에 `get_event_by_id(id: EventId)` 추가 (트리 거슬러 가기에 필요).
5. Mind Studio의 `/api/projection/trace/:cid` 응답에 트리 구조 표현 추가.

---

## 3. 완료 기준 (Definition of Done)

- [ ] 모든 신규 이벤트에 `metadata.cascade_depth`가 채워진다 (initial cmd 이벤트는 0).
- [ ] follow-up 이벤트는 `metadata.parent_event_id == Some(부모의 id)`를 가진다.
- [ ] Initial cmd 이벤트는 `metadata.parent_event_id == None`을 가진다.
- [ ] 같은 cid 안에서 `parent_event_id`를 따라가면 모든 이벤트가 단일 root(initial)로 수렴한다.
- [ ] `EventStore::get_event_by_id(id)` 가 정확히 그 id의 이벤트를 반환한다.
- [ ] `cargo test --workspace --all-features` 모두 통과.
- [ ] `cargo clippy --workspace --all-features -- -D warnings` 경고 없음.
- [ ] correlation-id-activation 태스크의 테스트가 여전히 통과 (회귀 없음).

---

## 4. 전제 및 주의사항

### 4.1 Library core 수정이 허용되는 범위
- `src/domain/event.rs` — `EventMetadata` 구조체 확장 (필수)
- `src/application/command/dispatcher.rs` — BFS 추적 + commit 2-pass 구조 (필수)
- `src/application/event_store.rs` — `get_event_by_id` 추가 (필수)
- `src/lib.rs` — re-export 영향 없음 (EventMetadata는 이미 노출됨)

### 4.2 호환성 — Breaking change

- `EventMetadata` 구조체에 필드 **추가**: serde derive로 자동 deserialize되지만, **기존 직렬화된 데이터에는 새 필드가 없다**. `#[serde(default)]`로 기본값을 부여한다.
  - `parent_event_id`: `Option<EventId>` → 기본 `None`
  - `cascade_depth`: `u32` → 기본 `0`
- `EventStore` trait에 메서드 **추가**: 외부 구현체 깨짐. 현재 구현체 0개이므로 영향 없음.

### 4.3 지켜야 할 원칙

- **2-pass commit**: 부모 id는 부모가 EventStore에 commit되어 id를 할당받은 직후에만 알 수 있다. BFS staging 시 "부모를 가리키는 인덱스"만 들고 있다가, commit 단계에서 부모 id가 할당된 후 자식 metadata를 채우는 방식이 필요하다.
- **Initial 이벤트는 parent 없음**: cmd의 첫 이벤트(`build_initial_event` 결과)는 `parent_event_id = None`, `cascade_depth = 0`.
- **Cascade depth는 BFS 깊이**: BFS 루프의 `depth` 변수를 그대로 쓴다. handler priority와는 무관 (priority는 같은 depth 내 실행 순서일 뿐).
- **Self-follow-up도 정상**: 한 핸들러가 자기 입력 이벤트를 부모로 하는 follow-up을 만드는 게 일반적이다 (예: StimulusPolicy가 StimulusApplied를 만드는 패턴). depth +1로 처리된다.

### 4.4 사전 확인 사항

1. `src/domain/event.rs` — `EventMetadata` 현재 필드 (`correlation_id`만 있는지) 확인.
2. `src/application/command/dispatcher.rs` — BFS 루프와 `commit_staging_buffer`의 정확한 시그니처 확인.
3. `src/application/event_store.rs` — `next_id` 발급 시점과 `append`의 관계 확인.
4. `tests/dispatch_v2_test.rs` — 기존 테스트가 EventMetadata 구조에 의존하는 부분 확인.

### 4.5 핵심 설계 결정 — Staging 단계의 부모 추적

BFS 루프에서 staging_buffer에 이벤트를 넣을 때, 그 이벤트의 부모를 어떻게 추적할 것인가? 두 옵션:

- **(A) Staging 인덱스 기반**: staging_buffer의 부모 위치 인덱스를 들고 있다가, commit 단계에서 인덱스 → 실제 id 매핑.
- **(B) 임시 토큰 기반**: BFS 시점에 임시 단조 증가 카운터로 토큰 부여, 부모-자식을 토큰으로 연결, commit 시 토큰 → id 매핑.

**(A)를 채택한다.** 이유: staging_buffer가 BFS 처리 순서 그대로이므로 인덱스가 자연스럽고, 추가 카운터가 불필요하다. 자식의 부모 인덱스는 자식이 staging_buffer에 들어가기 전에 결정되므로 안정적이다.

---

## 5. 작업 명세

### 5.1 작업 1 — `EventMetadata` 확장

**파일:** `src/domain/event.rs`

**변경:**

```rust
/// 이벤트 추적 메타데이터
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventMetadata {
    /// 같은 요청에서 파생된 이벤트 묶음 ID
    pub correlation_id: Option<u64>,
    
    /// 이 이벤트를 발생시킨 부모 이벤트의 id.
    /// `None`이면 initial cmd 이벤트(트리의 root).
    /// 같은 cid 안에서 이 필드를 따라 거슬러 올라가면 root에 도달한다.
    #[serde(default)]
    pub parent_event_id: Option<EventId>,
    
    /// BFS cascade 깊이. initial cmd 이벤트가 0, 그 follow-up이 1, ...
    /// 같은 cid 안에서 트리의 layer를 표시한다.
    #[serde(default)]
    pub cascade_depth: u32,
}
```

**`DomainEvent::new` 등 기존 생성자는 변경하지 않는다.** 기본값으로 `parent_event_id = None`, `cascade_depth = 0`이 들어간다 (initial 이벤트는 그대로 정상). dispatcher가 commit 단계에서 자식 이벤트의 metadata를 갱신한다.

### 5.2 작업 2 — `EventStore` trait에 단건 조회 메서드 추가

**파일:** `src/application/event_store.rs`

**변경:**

```rust
pub trait EventStore: Send + Sync {
    // ... 기존 메서드 ...

    /// id로 단일 이벤트 조회. 트리 거슬러 가기(parent chain 탐색)에 사용.
    fn get_event_by_id(&self, id: EventId) -> Option<DomainEvent>;
}
```

`InMemoryEventStore` 구현:

```rust
fn get_event_by_id(&self, id: EventId) -> Option<DomainEvent> {
    let store = self.events.read().unwrap();
    store.iter().find(|e| e.id == id).cloned()
}
```

### 5.3 작업 3 — Dispatcher BFS에서 부모 인덱스 추적

**파일:** `src/application/command/dispatcher.rs`

**변경:** BFS 루프의 큐 element를 `(depth, event, parent_staging_idx)` 트리플로 확장한다.

```rust
pub async fn dispatch_v2(&self, cmd: Command) -> Result<DispatchV2Output, DispatchV2Error>
where
    R: Send + Sync,
{
    let cid = self.command_seq.fetch_add(1, Ordering::SeqCst);
    let initial_event = self.build_initial_event(&cmd)?;
    let aggregate_key = initial_event.aggregate_key();

    let mut repo_guard = self.repository.lock().expect("repository mutex poisoned");
    let mut shared = HandlerShared::default();
    let mut prior_events: Vec<DomainEvent> = Vec::new();
    
    // ★ 큐 element 확장: (depth, event, parent_staging_idx)
    //   parent_staging_idx == None → initial 이벤트
    //   parent_staging_idx == Some(i) → staging_buffer[i]가 부모
    let mut event_queue: VecDeque<(u32, DomainEvent, Option<usize>)> = VecDeque::new();
    let mut staging_buffer: Vec<DomainEvent> = Vec::new();
    
    // ★ staging 인덱스별 부모 인덱스 (commit 단계에서 사용)
    let mut parent_indices: Vec<Option<usize>> = Vec::new();
    let mut depths: Vec<u32> = Vec::new();

    event_queue.push_back((0, initial_event, None));

    while let Some((depth, event, parent_idx)) = event_queue.pop_front() {
        if depth > MAX_CASCADE_DEPTH {
            return Err(DispatchV2Error::CascadeTooDeep { depth });
        }
        if staging_buffer.len() >= MAX_EVENTS_PER_COMMAND {
            return Err(DispatchV2Error::EventBudgetExceeded);
        }

        // ★ 이벤트가 staging에 들어가는 인덱스 — 자식들이 이걸 가리킬 것
        let my_idx = staging_buffer.len();

        for handler in self.transactional_handlers.iter() {
            if !handler.interest().matches(&event) {
                continue;
            }
            let DeliveryMode::Transactional { can_emit_follow_up, .. } = handler.mode() else {
                continue;
            };

            let mut ctx = EventHandlerContext { /* 기존 그대로 */ };

            let result = handler
                .handle(&event, &mut ctx)
                .map_err(|source| DispatchV2Error::HandlerFailed {
                    handler: handler.name(),
                    source,
                })?;

            if can_emit_follow_up {
                for follow_up in result.follow_up_events {
                    // ★ 자식의 parent_staging_idx = my_idx
                    event_queue.push_back((depth + 1, follow_up, Some(my_idx)));
                }
            } else {
                debug_assert!(result.follow_up_events.is_empty(), /* ... */);
            }
        }

        staging_buffer.push(event.clone());
        parent_indices.push(parent_idx);
        depths.push(depth);
        prior_events.push(event);
    }

    Self::apply_shared_to_repository(&mut *repo_guard, &aggregate_key, &shared);

    // ★ commit 시 parent_indices/depths를 함께 전달
    let committed = self.commit_staging_buffer(
        &aggregate_key,
        staging_buffer,
        cid,
        parent_indices,
        depths,
    );

    // ... 기존 inline phase / fanout 그대로 ...

    Ok(DispatchV2Output { events: committed, shared })
}
```

### 5.4 작업 4 — `commit_staging_buffer` 2-pass 구조

**파일:** `src/application/command/dispatcher.rs`

**변경:** 부모 id가 자식 metadata에 들어가야 하므로 단일 패스로는 불가. 2-pass:

```rust
fn commit_staging_buffer(
    &self,
    _command_key: &AggregateKey,
    staging: Vec<DomainEvent>,
    cid: u64,
    parent_indices: Vec<Option<usize>>,
    depths: Vec<u32>,
) -> Vec<DomainEvent> {
    debug_assert_eq!(staging.len(), parent_indices.len());
    debug_assert_eq!(staging.len(), depths.len());

    let mut committed: Vec<DomainEvent> = Vec::with_capacity(staging.len());
    
    // Pass 1: id, sequence, cid, depth 할당. parent_event_id는 임시로 None.
    //         BFS 처리 순서가 곧 staging 순서이므로, 부모는 항상 자식보다 먼저 commit된다.
    for (idx, event) in staging.into_iter().enumerate() {
        let per_event_id = event.aggregate_key().npc_id_hint().to_string();
        let id = self.event_store.next_id();
        let seq = self.event_store.next_sequence(&per_event_id);
        
        let mut e = DomainEvent::new(id, per_event_id, seq, event.payload);
        e = e.with_correlation(cid);
        e.metadata.cascade_depth = depths[idx];
        
        // Pass 1에서는 parent_event_id 미설정 — Pass 2에서 채움
        committed.push(e);
    }
    
    // Pass 2: parent_indices를 사용해 부모 id를 자식 metadata에 채움.
    //         committed[idx]의 부모가 committed[parent_indices[idx]]에 있으므로 안전.
    for idx in 0..committed.len() {
        if let Some(parent_idx) = parent_indices[idx] {
            committed[idx].metadata.parent_event_id = Some(committed[parent_idx].id);
        }
        // initial 이벤트(parent_indices[idx] == None)는 parent_event_id 그대로 None.
    }
    
    // Pass 3: EventStore에 append.
    //         Pass 1·2가 끝난 후에 append하는 이유는, append가 끝나면 이벤트가 외부에
    //         노출되므로 metadata가 완전한 상태에서 노출되어야 하기 때문이다.
    self.event_store.append(&committed);
    
    committed
}
```

**중요한 변경**: 기존 코드는 **각 이벤트마다 즉시 `event_store.append(&[e.clone()])`** 했지만, 이제는 **모든 이벤트를 committed에 모은 뒤 한 번에 append**한다. 이로써 metadata가 완전히 채워진 상태로 EventStore에 들어간다.

### 5.5 작업 5 — Mind Studio trace 응답에 트리 구조 추가

**파일:** `src/bin/mind-studio/handlers/query.rs`

**변경:** `TraceView`에 트리 표현 추가.

```rust
#[derive(Serialize)]
pub struct TraceNode {
    pub event: DomainEvent,
    pub children: Vec<TraceNode>,
}

#[derive(Serialize)]
pub struct TraceView {
    pub correlation_id: u64,
    pub event_count: usize,
    /// flat list (id 오름차순)
    pub events: Vec<DomainEvent>,
    /// 트리 구조 (root는 parent_event_id == None)
    pub tree: Option<TraceNode>,
}

pub async fn get_trace(
    State(state): State<AppState>,
    Path(correlation_id): Path<u64>,
) -> Result<Json<TraceView>, AppError> {
    let store = state.shared_dispatcher.event_store();
    let events = store.get_events_by_correlation(correlation_id);
    
    let tree = build_tree(&events);
    
    Ok(Json(TraceView {
        correlation_id,
        event_count: events.len(),
        events: events.clone(),
        tree,
    }))
}

fn build_tree(events: &[DomainEvent]) -> Option<TraceNode> {
    // root 찾기: parent_event_id == None
    let root = events.iter().find(|e| e.metadata.parent_event_id.is_none())?;
    Some(build_node(root, events))
}

fn build_node(parent: &DomainEvent, all: &[DomainEvent]) -> TraceNode {
    let children: Vec<TraceNode> = all
        .iter()
        .filter(|e| e.metadata.parent_event_id == Some(parent.id))
        .map(|child| build_node(child, all))
        .collect();
    TraceNode {
        event: parent.clone(),
        children,
    }
}
```

---

## 6. 테스트 요구사항

### 6.1 cascade_depth 검증 (필수)

```rust
#[tokio::test]
async fn cascade_depth_increases_along_follow_up_chain() {
    let dispatcher = build_test_dispatcher().await;
    
    // ApplyStimulus는 follow-up cascade가 깊은 cmd
    let result = dispatcher.dispatch_v2(Command::ApplyStimulus { /* ... */ }).await.unwrap();
    
    let initial = &result.events[0];
    assert_eq!(initial.metadata.cascade_depth, 0, "initial event must have depth 0");
    
    // 적어도 하나는 depth > 0이어야 (cmd가 follow-up을 만든다면)
    let max_depth = result.events.iter()
        .map(|e| e.metadata.cascade_depth)
        .max()
        .unwrap();
    assert!(max_depth > 0, "expected at least one follow-up event");
}
```

### 6.2 parent_event_id 일관성 검증 (필수)

```rust
#[tokio::test]
async fn parent_event_id_forms_valid_tree() {
    let dispatcher = build_test_dispatcher().await;
    let result = dispatcher.dispatch_v2(Command::ApplyStimulus { /* ... */ }).await.unwrap();
    
    let event_ids: HashSet<_> = result.events.iter().map(|e| e.id).collect();
    
    // 모든 parent_event_id는 같은 묶음의 다른 이벤트를 가리킨다
    for ev in &result.events {
        if let Some(parent_id) = ev.metadata.parent_event_id {
            assert!(
                event_ids.contains(&parent_id),
                "parent_event_id {} not found in same correlation bundle", parent_id
            );
        }
    }
    
    // 정확히 하나의 root (parent_event_id == None)
    let roots: Vec<_> = result.events.iter()
        .filter(|e| e.metadata.parent_event_id.is_none())
        .collect();
    assert_eq!(roots.len(), 1, "exactly one root event expected");
    assert_eq!(roots[0].metadata.cascade_depth, 0, "root must have depth 0");
}
```

### 6.3 부모-자식 depth 관계 검증 (필수)

```rust
#[tokio::test]
async fn child_depth_is_parent_plus_one() {
    let dispatcher = build_test_dispatcher().await;
    let result = dispatcher.dispatch_v2(Command::ApplyStimulus { /* ... */ }).await.unwrap();
    
    let by_id: HashMap<EventId, &DomainEvent> = result.events.iter()
        .map(|e| (e.id, e))
        .collect();
    
    for ev in &result.events {
        if let Some(parent_id) = ev.metadata.parent_event_id {
            let parent = by_id.get(&parent_id).expect("parent must exist");
            assert_eq!(
                ev.metadata.cascade_depth,
                parent.metadata.cascade_depth + 1,
                "child depth must be parent depth + 1"
            );
        }
    }
}
```

### 6.4 get_event_by_id 검증 (필수)

```rust
#[tokio::test]
async fn event_store_returns_event_by_id() {
    let dispatcher = build_test_dispatcher().await;
    let result = dispatcher.dispatch_v2(Command::Appraise { /* ... */ }).await.unwrap();
    
    let target = &result.events[0];
    let fetched = dispatcher.event_store().get_event_by_id(target.id);
    
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().id, target.id);
    
    // 존재하지 않는 id는 None
    let missing = dispatcher.event_store().get_event_by_id(99999999);
    assert!(missing.is_none());
}
```

### 6.5 Trace 트리 빌드 smoke test (권장)

`/api/projection/trace/:cid` 엔드포인트 호출 후 `tree.children` 구조가 비어 있지 않음을 확인.

### 6.6 회귀 확인 (필수)

- correlation-id-activation의 모든 테스트 통과
- read-side-activation의 drift 감지 테스트 통과
- `cargo test --workspace --all-features` 전체 통과

---

## 7. 점진적 도입 순서 (권장)

### 1단계 — 메타데이터 필드 + 단일 이벤트 검증
- 작업 1 (`EventMetadata` 확장)
- `DomainEvent::new`로 만든 이벤트가 `parent_event_id = None`, `cascade_depth = 0`을 갖는지 unit test
- 이 시점엔 dispatcher가 아직 새 필드를 채우지 않으므로 모든 이벤트가 default 값

### 2단계 — Dispatcher BFS 추적
- 작업 3 (BFS 큐 트리플 확장 + parent_indices/depths 수집)
- 작업 4 (commit 2-pass)
- 테스트 6.1, 6.2, 6.3 통과 확인

### 3단계 — EventStore 단건 조회
- 작업 2 (`get_event_by_id`)
- 테스트 6.4 통과 확인

### 4단계 — Mind Studio 트리 응답
- 작업 5 (TraceNode + build_tree)
- 테스트 6.5 통과 확인

### 5단계 — 문서화
- README 또는 system-overview.md에 "한 cmd의 인과 트리 추적 가능" 명시
- 시뮬레이션 파일(`sim/dispatch_v2_sim.py`, `sim/dispatch_v2_sim2.py`)이 만들었던 그림이 이제 실제 데이터로 만들어짐을 본문에 적시

---

## 8. 체크리스트 (PR 올리기 전)

### Library core
- [ ] `EventMetadata`에 `parent_event_id: Option<EventId>` 추가 (`#[serde(default)]`)
- [ ] `EventMetadata`에 `cascade_depth: u32` 추가 (`#[serde(default)]`)
- [ ] `EventStore` trait에 `get_event_by_id` 추가
- [ ] `InMemoryEventStore`에 `get_event_by_id` 구현
- [ ] `dispatch_v2` BFS 큐를 트리플로 확장
- [ ] `parent_indices`, `depths` 벡터로 staging 메타 추적
- [ ] `commit_staging_buffer` 2-pass 구조로 변경
- [ ] EventStore append를 마지막에 한 번만 호출하도록 변경

### Mind Studio
- [ ] `TraceNode` 구조체 추가
- [ ] `TraceView`에 `tree: Option<TraceNode>` 추가
- [ ] `build_tree`/`build_node` 헬퍼 구현

### 테스트
- [ ] `cascade_depth_increases_along_follow_up_chain` 통과
- [ ] `parent_event_id_forms_valid_tree` 통과
- [ ] `child_depth_is_parent_plus_one` 통과
- [ ] `event_store_returns_event_by_id` 통과
- [ ] Trace 트리 빌드 smoke test (선택)
- [ ] `cargo test --workspace --all-features` 전체 통과
- [ ] `cargo test --workspace --no-default-features` 통과
- [ ] `cargo clippy --workspace --all-features -- -D warnings` 경고 없음

### 수동 smoke test
- [ ] Mind Studio 기동 → Appraise/ApplyStimulus 실행
- [ ] `curl http://localhost:PORT/api/projection/trace/<cid>` 로 응답 받기
- [ ] 응답의 `tree.children`이 비어 있지 않고, 자식의 `cascade_depth`가 부모 +1임을 확인

---

## 9. 관련 파일 (작업 시 참조 경로)

| 역할 | 경로 | 변경 여부 |
|---|---|---|
| EventMetadata 정의 | `src/domain/event.rs` | 수정 (필드 추가) |
| Dispatcher BFS + commit | `src/application/command/dispatcher.rs` | 수정 (큐 구조, 2-pass commit) |
| EventStore trait + InMemory | `src/application/event_store.rs` | 수정 (get_event_by_id 추가) |
| Mind Studio query 핸들러 | `src/bin/mind-studio/handlers/query.rs` | 수정 (트리 응답) |
| 시뮬레이션 (참고용) | `sim/dispatch_v2_sim2.py` | 읽기 전용 — 동작의 시각화 참고 |

---

## 10. Out of Scope / 후속 작업

본 태스크에서 **하지 않는다**:

- **Cmd 사이의 인과(`causation_id`).** 다른 cmd 사이의 인과 추적은 별도 태스크. 호출자(외부 코드)가 명시적으로 "이 cmd는 cid=42 cmd의 결과로 발동된다"고 선언하는 API가 필요한데, 이는 dispatcher 인터페이스 확장이 따른다.
- **Actor / Intent / Trigger 메타데이터.** "누가 어떤 의도로 이 cmd를 발동했는가"는 별도 태스크.
- **Random seed 보존.** 결정적 재생산을 위한 seed 메타는 별도 태스크.
- **EventStore 영속화.** SQLite 영속화 시 `parent_event_id` 인덱스 등은 영속화 태스크에서 다룬다.
- **Trace tree의 SVG/Mermaid 렌더링.** 본 태스크는 JSON 트리 응답까지. 시각화 UI는 별도.

---

## 11. 위험 요소

### 11.1 2-pass commit의 부분 실패
Pass 1·2가 메모리에서만 일어나고 Pass 3 append 직전에 패닉이 나면, 일부 이벤트만 EventStore에 들어가는 부분 실패가 발생할 수 있다. 현재 `InMemoryEventStore::append`는 단일 lock 안에서 `extend`하므로 원자적이다. SQLite 영속화 후엔 트랜잭션 처리가 필요하다 — 그건 영속화 태스크에서.

### 11.2 Initial 이벤트의 cascade_depth 역할
Initial 이벤트가 `cascade_depth = 0`을 갖는 건 명시적이다. 그러나 `EventMetadata::default()`도 `cascade_depth = 0`이므로, "default로 만든 이벤트"와 "정상적인 initial 이벤트"가 메타만 보면 구분되지 않는다. **이는 의도된 동작**이다 — initial 이벤트는 항상 root이고, default 메타는 곧 initial 메타와 같다고 본다.

### 11.3 staging 인덱스의 안정성
BFS 중 `staging_buffer.push`가 일어나기 전에 자식이 큐에 들어가므로, 부모의 `my_idx`가 자식의 `parent_idx`로 정확히 전달된다. **단, BFS pop 후 handler 실행 도중 staging_buffer가 다른 곳에서 변경되면 인덱스가 어긋난다**. 현재 구조는 단일 스레드 BFS이므로 안전하지만, 미래에 핸들러 병렬화를 도입하면 이 불변량이 깨질 수 있다. 명시적으로 코드 주석에 적어둔다.

### 11.4 트리 구조 응답의 재귀 깊이
`build_node`가 재귀라 cascade depth가 매우 깊으면 스택 오버플로우 위험이 있다. 현재 `MAX_CASCADE_DEPTH = 4`로 가드되어 있어 안전하지만, 이 가드가 변경되면 재귀 → 반복으로 리팩터링이 필요할 수 있다.
