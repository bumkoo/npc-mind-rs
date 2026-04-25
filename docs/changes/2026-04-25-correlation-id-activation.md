# correlation_id 활성화 — dispatch 단위 인과 사슬 추적

**날짜:** 2026-04-25
**관련 task:** [`docs/tasks/correlation-id-activation.md`](../tasks/correlation-id-activation.md)
**관련 커밋:** `504f6d2` (1단계) · `755585e` (2·3단계) · `d05513d` (smoke test)

## 요약

`DomainEvent.metadata.correlation_id`는 필드와 부착 코드가 모두 갖춰져
있었으나 발급 트리거(`set_correlation_id`)를 호출하는 코드가 0개라 모든
저장 이벤트의 cid가 `None` 상태였다. 본 변경은 발급을 자동화하고, 묶음
조회 API를 추가하며, mind-studio에 trace 엔드포인트를 노출한다.

## Added

- **`dispatch_v2` 호출 단위 `correlation_id` 자동 발급**
  - 함수 진입 시 `command_seq.fetch_add(1, SeqCst)`로 cid 생성
  - cid는 `dispatch_v2` 함수 로컬 변수 — 인스턴스 필드에 저장하지 않음
    (per-call 격리, 동시 호출에 안전)
  - 1부터 시작하며 0은 "미설정" sentinel로 예약
- **`EventStore::get_events_by_correlation(cid) -> Vec<DomainEvent>`**
  - 같은 cid를 공유하는 이벤트 묶음 조회
  - `InMemoryEventStore` 구현 포함
  - EventStore에 추가된 순서를 보존 (정렬은 호출자 책임)
- **`GET /api/projection/trace/{correlation_id}`** (mind-studio)
  - 한 dispatch의 전체 인과 사슬 반환 (`{ correlation_id, event_count, events }`)
  - `shared_dispatcher`의 EventStore만 본다 — `/api/v2/*` Director 경로는
    별도 dispatcher 인스턴스라 본 엔드포인트 범위 밖
  - 평가 파이프라인(DeepEval Phase 1) trace 단위 입력으로 사용 가능

## Changed

- **Breaking — `EventStore` trait에 메서드 추가**
  - `fn get_events_by_correlation(&self, correlation_id: u64) -> Vec<DomainEvent>;`
  - 외부 구현체가 있다면 추가 구현 필요. 현재 리포지토리 내 구현체는
    `InMemoryEventStore` 1개라 영향 없음. `SqliteEventStore` 등 미래
    구현체는 본 메서드를 함께 구현해야 한다.
- **`CommandDispatcher.correlation_id: Arc<AtomicU64>` 글로벌 슬롯 제거**
  - cid가 인스턴스 공유 슬롯에 있던 시기에는 `set` 후 `current_correlation_id()`
    사이에 다른 dispatch 호출이 들어오면 cid가 섞일 위험이 있었다.
    repository mutex가 우연히 직렬화해주고 있었을 뿐 명시적 보증은 아니었다.
  - cid가 함수 로컬 변수가 되어 동시 dispatch에 안전.
- **`commit_staging_buffer` 시그니처 확장**
  - `cid: u64` 인자 추가, 항상 `with_correlation` 부착
  - cid 부착 위치가 본 함수 단 한 군데로 단일화 (§4.3 원칙)

## Deprecated

- **`CommandDispatcher::set_correlation_id`**
  - `dispatch_v2`가 cid를 자동 발급하므로 외부 주입 수단이 불필요
  - 현재는 no-op + `#[deprecated]` 마킹
  - 다음 마이너 버전(예: v0.5)에서 완전 제거 예정

## Notes / Out of Scope

- **영속화 시 카운터 복원** (task §12.3 — 영속화 task로 이관)
  프로세스 재시작 시 `command_seq`가 1로 리셋되므로 같은 cid가 다른
  세션에서 재사용될 수 있다. SQLite 등 영속 EventStore 도입 시점에
  시작 시 `SELECT MAX(correlation_id) FROM events`로 카운터를 복원하는
  로직이 필요하다. `dispatcher.rs::CommandDispatcher::command_seq` doc
  주석에 이 요구사항을 명시해두었다.
- **외부 cid 주입** (task §10)
  HTTP 헤더(`X-Request-Id` 등)로 들어온 cid를 dispatch에 전달하는 기능은
  별도 task. 현재는 dispatcher가 자체 발급만 한다.
- **Director_v2 trace 노출** (task §10)
  `/api/v2/*` 경로는 별도 dispatcher 인스턴스를 쓰며 그쪽 EventStore의
  trace 노출은 별도 task.
- **`set_correlation_id` 완전 제거**
  본 변경은 deprecation까지. 실제 제거는 다음 마이너 버전.

## 검증

- `tests/dispatch_v2_test.rs` 신규 3건
  - `dispatch_v2_attaches_correlation_id_to_all_events` (6.1)
  - `distinct_dispatch_calls_get_distinct_correlation_ids` (6.2)
  - `event_store_returns_correct_correlation_bundle` (6.3)
- `src/bin/mind-studio/handler_tests.rs` 신규 1건
  - `projection_drift::trace_endpoint_returns_correlation_bundle` (6.4)
- 회귀: `tests/dispatch_v2_test.rs` 13건 + `projection_drift` 6건 모두 통과
- 라이브러리 전체 466건 + mind-studio bin 50건 통과
