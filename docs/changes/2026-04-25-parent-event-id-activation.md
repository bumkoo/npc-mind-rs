# parent_event_id / cascade_depth 활성화 — cmd 안의 인과 트리

**날짜:** 2026-04-25
**관련 task:** [`docs/tasks/parent-event-id-activation.md`](../tasks/parent-event-id-activation.md)
**관련 커밋:** `f55df93` (Phase A 1단계) · `5ae0e44` (Phase A 리뷰 정리) · `2d89e6e` (Phase B 2단계) · `28ab3c2` (Phase B 리뷰 정리) · `a601529` (문서 현행화) · 본 커밋 (3·4단계)

## 요약

`correlation-id-activation` 으로 한 dispatch가 만든 이벤트 묶음(cid)은 추적 가능해졌으나, 묶음 내부의 **부모-자식 관계**와 **cascade 깊이**는 EventStore에 보존되지 않았다. dispatcher BFS 루프는 `(depth, event)` 큐로 이 정보를 임시로 알고 있지만 commit 시 버리고 있었다. 본 변경은 두 정보를 `EventMetadata`에 영구 보존하고, BFS 루프에서 부모 인덱스를 추적해 commit 단계에서 채워 한 cmd 안의 인과 트리를 정확히 시각화·평가할 수 있게 한다.

`docs/tasks/parent-event-id-activation.md` §7 의 1·2·3·4·5 단계 전체 완료. 후속 task 로 분리하지 않음.

## Added

- **`EventMetadata.parent_event_id: Option<EventId>`**
  - 이 이벤트를 발생시킨 부모 이벤트의 id. `None` 이면 초기 커맨드 이벤트(트리의 root).
  - 같은 cid 안에서 이 필드를 따라 거슬러 올라가면 root 에 도달한다.
  - `#[serde(default)]` — 기존 직렬화 데이터(필드 없음)는 `None` 으로 deserialize.
- **`EventMetadata.cascade_depth: u32`**
  - BFS cascade 깊이. 초기 커맨드 이벤트가 0, 그 follow-up 이 1, ...
  - `#[serde(default)]` — 기존 데이터는 0 으로 deserialize. initial 이벤트의 정상값과 일치하므로 의미 충돌 없음.
- **`DomainEvent` 빌더 메서드 2 종**
  - `with_cascade_depth(u32) -> Self`
  - `with_parent(EventId) -> Self`
  - 기존 `with_correlation(u64)` 와 같은 builder 패턴.
- **`EventStore::get_event_by_id(id: EventId) -> Option<DomainEvent>`** (3단계)
  - 단건 이벤트 조회 — `parent_event_id` 사슬을 따라 트리를 거슬러 갈 때 사용.
  - 기본 구현은 `get_all_events()` 스캔으로 O(N). `InMemoryEventStore` 는 동일하게 O(N) 구현. 영속 백엔드는 PK 인덱스로 O(1) override 권장.
- **Mind Studio `/api/projection/trace/{cid}` 응답에 `tree` 필드 추가** (4단계)
  - `TraceView { correlation_id, event_count, events, tree: Option<TraceNode> }`
  - `TraceNode { event, children: Vec<TraceNode> }` — 재귀 구조.
  - root 는 `parent_event_id == None`. 이벤트 묶음이 비어 있으면 `tree == null`.
  - `build_tree`/`build_node` 헬퍼는 `MAX_CASCADE_DEPTH = 4` 가드 하에서 재귀 구현 (가드 변경 시 반복형 리팩터링 필요).

## Changed

- **BFS 큐 element 트리플 확장**
  - `VecDeque<(u32, DomainEvent)>` → `VecDeque<(u32, DomainEvent, Option<usize>)>`.
  - 세 번째 요소는 staging_buffer 내 부모 위치 인덱스. `None` 이면 root.
  - 부모 EventStore id 는 commit 전이라 미정이므로 인덱스로 가리키고, commit 시 id 매핑.
- **`commit_staging_buffer` 시그니처 + 구조 변경**
  - `parent_indices: Vec<Option<usize>>`, `depths: Vec<u32>` 두 인자 추가.
  - **이벤트마다 `event_store.append(&[e.clone()])` 하던 루프를 단일 `event_store.append(&committed)` 로 통합.**
    metadata 가 완전히 채워진 뒤에만 EventStore 에 노출되도록.
    `InMemoryEventStore::append` 는 단일 lock 안의 `extend` 라 원자적이라 안전.
    SQLite 영속화 도입 시점엔 트랜잭션 처리 필요.
  - 빌더 체인 `DomainEvent::new(...).with_correlation(cid).with_cascade_depth(depths[idx])` + 부모 있으면 `.with_parent(...)` — 직접 필드 할당 제거.
- **단일 패스 commit (Phase B 리뷰 결과)**
  - 초기 구현은 Pass 1(id/cid/depth 부착) → Pass 2(부모 id 링크) 두 패스였으나, BFS 처리 순서가 부모→자식이라 자식 차례엔 `committed[parent_idx]` 가 이미 id 를 받은 상태 → 단일 패스로 충분. 2N → N 순회.
- **Breaking — `EventStore` trait 메서드 추가** (3단계)
  - `fn get_event_by_id(&self, id: EventId) -> Option<DomainEvent>` — default 구현 제공이라 외부 구현체가 깨지진 않으나, 영속 백엔드는 인덱스 활용으로 override 권장.

## Notes / Out of Scope

- **Cmd 사이의 인과 (`causation_id`)**
  본 변경은 한 cmd 안의 인과만 다룬다. 서로 다른 cmd 사이의 인과 추적은 호출자가 명시적으로 선언해야 하는 별도 설계 결정 (`docs/tasks/parent-event-id-activation.md` §10).
- **Actor / Intent / Trigger 메타데이터**
  "누가 어떤 의도로 이 cmd 를 발동했는가" 는 별도 task.
- **트리 구조 응답의 재귀 깊이**
  현재 `MAX_CASCADE_DEPTH = 4` 가드 하에서 재귀로도 안전하지만, 가드 변경 시 반복으로 리팩터링 필요.
- **Director_v2 trace 노출**
  `/api/v2/*` Director 경로는 별도 dispatcher 인스턴스를 쓰며, 그쪽 EventStore 의 trace 노출은 `correlation-id-activation` 과 동일하게 별도 task.
- **영속 EventStore 의 인덱스**
  현재 `InMemoryEventStore` 의 `get_event_by_id` 와 `get_events_by_correlation` 는 모두 O(N) scan. SQLite 등 영속 백엔드 도입 시 PK·correlation_id 인덱스를 활용한 O(log N) override 필요.

## 검증

- `tests/dispatch_v2_test.rs` 신규 4 건 + 1 시각화 헬퍼
  - `cascade_depth_increases_along_follow_up_chain` (§6.1)
  - `parent_event_id_forms_valid_tree` (§6.2)
  - `child_depth_is_parent_plus_one` (§6.3)
  - `event_store_returns_event_by_id` (§6.4)
  - `print_causal_tree_for_stimulus` (#[ignore], 수동 시각화 — `sim/dispatch_v2_sim2.py` 가설 재현)
- `src/bin/mind-studio/handler_tests.rs` 신규 1 건
  - `projection_drift::trace_endpoint_returns_causal_tree` (§6.5)
- `src/domain/event.rs` 신규 4 건
  - `default_metadata_has_no_parent_and_zero_depth`
  - `new_domain_event_inherits_default_metadata`
  - `legacy_metadata_json_deserializes_with_defaults`
  - `empty_metadata_json_deserializes`
- 수동 인과 트리 재현 결과 (`stimulus_cmd` dispatch):
  ```
  --- correlation_id = 2 (3 events) ---
  #4 StimulusApplyRequested (depth=0)
    #5 StimulusApplied (depth=1)
      #6 GuideGenerated (depth=2)
  ```
- 회귀: `tests/dispatch_v2_test.rs` 19 passed (1 ignored = 시각화 도우미) · `projection_drift::*` 8 건 · 기본 features 전체 524 passed.
- 사전 환경 의존(ONNX 모델 부재 16 건 + sqlite-vec 1 건 + memory_endpoints 시나리오 로드 5 건)은 stash baseline 시점에서 동일하게 실패함 — 본 변경과 무관.

### 수동 smoke test

`cargo run --features mind-studio,chat,embed --bin npc-mind-studio` 후 NPC seed → `/api/appraise` 호출 → `curl /api/projection/trace/1` 응답:

```json
{
  "correlation_id": 1,
  "event_count": 3,
  "events": [...],
  "tree": {
    "event": { "id": 1, ... "metadata": { "parent_event_id": null, "cascade_depth": 0 } },
    "children": [{
      "event": { "id": 2, ... "metadata": { "parent_event_id": 1, "cascade_depth": 1 } },
      "children": [{
        "event": { "id": 3, ... "metadata": { "parent_event_id": 2, "cascade_depth": 2 } },
        "children": []
      }]
    }]
  }
}
```

cascade `AppraiseRequested → EmotionAppraised → GuideGenerated` 가 트리로 정확히 재현됨.
