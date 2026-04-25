# parent_event_id / cascade_depth 활성화 — cmd 안의 인과 트리

**날짜:** 2026-04-25
**관련 task:** [`docs/tasks/parent-event-id-activation.md`](../tasks/parent-event-id-activation.md)
**관련 커밋:** `f55df93` (Phase A 1단계) · `5ae0e44` (Phase A 리뷰 정리) · `2d89e6e` (Phase B 2단계) · `28ab3c2` (Phase B 리뷰 정리)

## 요약

`correlation-id-activation` 으로 한 dispatch가 만든 이벤트 묶음(cid)은 추적 가능해졌으나, 묶음 내부의 **부모-자식 관계**와 **cascade 깊이**는 EventStore에 보존되지 않았다. dispatcher BFS 루프는 `(depth, event)` 큐로 이 정보를 임시로 알고 있지만 commit 시 버리고 있었다. 본 변경은 두 정보를 `EventMetadata`에 영구 보존하고, BFS 루프에서 부모 인덱스를 추적해 commit 단계에서 채워 한 cmd 안의 인과 트리를 정확히 시각화·평가할 수 있게 한다.

본 태스크는 `docs/tasks/parent-event-id-activation.md` §7 의 1·2단계만 다룬다 — 3단계(`EventStore::get_event_by_id`) 와 4단계(Mind Studio `/api/projection/trace/:cid` 응답에 트리 구조 표현) 는 후속 task 로 분리.

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

## Notes / Out of Scope

- **`EventStore::get_event_by_id(id)`** (task §5.2)
  단건 조회 메서드 추가는 본 변경 범위 밖. 인과 트리 응답 작성 시 부모를 follow-up 으로 거슬러 갈 때 필요해질 수 있다 — 후속 task.
- **Mind Studio `/api/projection/trace/:cid` 트리 구조 응답** (task §5.5)
  현재 trace 엔드포인트는 flat list 만 반환한다. `parent_event_id` 가 영구 저장됐으므로 클라이언트 측 트리 빌드도 가능하지만, 서버에서 `TraceNode { event, children }` 로 미리 빌드해 주는 형태로 노출하는 것은 후속 task.
- **Cmd 사이의 인과 (`causation_id`)**
  본 변경은 한 cmd 안의 인과만 다룬다. 서로 다른 cmd 사이의 인과 추적은 호출자가 명시적으로 선언해야 하는 별도 설계 결정 (`docs/tasks/parent-event-id-activation.md` §10).
- **Actor / Intent / Trigger 메타데이터**
  "누가 어떤 의도로 이 cmd 를 발동했는가" 는 별도 task.
- **트리 구조 응답의 재귀 깊이**
  현재 `MAX_CASCADE_DEPTH = 4` 가드 하에서 재귀로도 안전하지만, 가드 변경 시 반복으로 리팩터링 필요할 수 있다.

## 검증

- `tests/dispatch_v2_test.rs` 신규 3 건 + 1 시각화 헬퍼
  - `cascade_depth_increases_along_follow_up_chain` (§6.1)
  - `parent_event_id_forms_valid_tree` (§6.2)
  - `child_depth_is_parent_plus_one` (§6.3)
  - `print_causal_tree_for_stimulus` (#[ignore], 수동 시각화 — `sim/dispatch_v2_sim2.py` 가설 재현)
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
- 회귀: `tests/dispatch_v2_test.rs` 18 passed (1 ignored = 시각화 도우미) · `projection_drift::*` 7 건 · 기본 features 전체 523 passed.
- 사전 환경 의존(ONNX 모델 부재 16 건 + sqlite-vec 1 건)은 stash baseline 시점에서 동일하게 실패함 — 본 변경과 무관.
