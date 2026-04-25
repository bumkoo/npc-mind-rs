# 변경 기록 (Change Log)

라이브러리 동작이나 공개 API에 영향을 주는 변경을 시간순으로 기록한다.
각 항목은 별도 파일이며 파일명은 `YYYY-MM-DD-<slug>.md` 형식.

## 작성 기준

- **포함**: 공개 API/포트/이벤트 스키마/HTTP 엔드포인트 변경, 행동 변화
  (행동이 같더라도 동시성 모델이나 보장 수준이 바뀐 경우 포함),
  deprecation 결정, breaking change.
- **제외**: 내부 리팩터링, 주석/문서 수정, 테스트 추가만 있는 변경,
  의존성 minor bump.

## 파일 구조

각 항목은 다음 섹션을 권장한다 (없으면 생략):

- **요약** — 한 문단 컨텍스트
- **Added / Changed / Deprecated / Removed / Fixed** — 카테고리별 항목
- **Notes / Out of Scope** — 후속 task로 미룬 부분, 알려진 한계
- **검증** — 추가된 테스트, 회귀 확인 방법

## 인덱스

| 날짜 | 변경 | 파일 |
|---|---|---|
| 2026-04-25 | `correlation_id` 활성화 — dispatch 단위 인과 사슬 추적 | [`2026-04-25-correlation-id-activation.md`](2026-04-25-correlation-id-activation.md) |
| 2026-04-25 | `parent_event_id` / `cascade_depth` 활성화 — cmd 안의 인과 트리 보존 | [`2026-04-25-parent-event-id-activation.md`](2026-04-25-parent-event-id-activation.md) |

## 관련

- Task 명세는 [`docs/tasks/`](../tasks/)에 별도로 존재한다. 본 디렉토리는
  task 완료 후 **무엇이 어떻게 바뀌었는지** 기록하는 용도.
- README나 system-overview.md 같은 살아있는 문서가 본 변경에 따라 갱신될
  때, 어느 커밋에서 어떤 의도로 갱신했는지 본 디렉토리에서 추적할 수 있다.
