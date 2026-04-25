# Memory Step A — Foundation

**상태**: ✅ 2026-04 · **커밋**: `ebc3a8a` (구현) + `f0964f6` (리뷰 대응) + `01a6dab` (문서)
**브랜치**: `claude/reference-memory-docs-UA06H`
**설계 정본**: [`../memory/03-implementation-design.md`](../memory/03-implementation-design.md) §13 Step A

## 핵심

행동 변화 없이 스키마·VO·Ranker만 준비. 기존 LLM 프롬프트엔 영향 없음 (그건 Step B).

- `MemoryScope` (5변종, Relationship 대칭 정규화) · `MemorySource` (4단계) · `Provenance` (Seeded/Runtime) · `MemoryLayer` (A/B) VO
- `MemoryEntry` 13 필드 확장 + `::personal()` 호환 생성자 + `npc_id` grand-father
- `MemoryRanker` 2단계 (Source 우선 필터 + 5요소 점수) + `DecayTauTable` 3축 룩업
- SQLite v1→v2 자동 마이그레이션 (15 컬럼 + 6 인덱스 + vec0 partition_key 재구성, 트랜잭션)
- `MemoryStore` 7 신규 메서드 + `MemoryQuery`/`MemoryScopeFilter`
- `RelationshipUpdated.cause` hook (Step A는 `Unspecified` 고정)

## 사용자 승인 결정 (3차 문서 §17)

- **§17.1**: `MemoryEntry.npc_id`는 `scope.owner_a()` 투영으로 grand-father + `#[deprecated]`
- **§17.4**: ID 포맷은 `mem-{06d}` 순번 유지

## 변경 파일

```
src/domain/memory.rs              # 확장 (≈600줄)
src/domain/memory/ranker.rs       # 신규
src/domain/tuning.rs              # 메모리 상수 추가
src/domain/event.rs               # RelationshipChangeCause + cause 필드
src/ports.rs                      # MemoryStore 7 신규 메서드 + MemoryQuery/Filter
src/adapter/sqlite_memory.rs      # 스키마 v2 + 신규 메서드 (트랜잭션 원자성)
src/application/memory_agent.rs   # MemoryEntry::personal 호출로 갱신
src/application/command/agents/relationship_agent.rs  # cause: Unspecified
src/application/command/projection_handlers.rs       # 테스트 cause 필드
src/bin/mind-studio/domain_sync.rs # 패턴 매치 .. 추가
tests/common/in_memory_store.rs   # 신규 7 메서드 brute-force 구현
tests/memory_test.rs              # MemoryEntry::personal로 이행
tests/sqlite_memory_test.rs       # v1→v2 마이그레이션 테스트 추가
```

## 검증

- 신규 단위·통합 테스트 15건 (scope 대칭 / source priority / canonical / serde alias / retention / v1→v2 / supersede / record_recall / MemoryQuery 필터 등)
- 전체 362+ 테스트 green (default · listener_perspective)
- 샌드박스 embed 빌드 차단 (ort 바이너리 CDN 503) → CI 검증

## 리뷰 후속 수정 (`f0964f6`)

- `NpcAllowed` 동적 SQL 바인딩을 `?N` 반복 → 3개 독립 placeholder + push 3회 (안전)
- `migrate_v2` 전체를 `unchecked_transaction`으로 감싸 vec0 DROP/CREATE/INSERT 원자성 보장
- `partition_key` 포맷에 ID `:` 금지 제약 docstring 추가
- `from_origin_chain` 힌트 동작 (Heard/Rumor 무시) 명시화

## 잔여 / 알려진 제약

- `MemoryRanker` 호출 production 경로 없음 → Step B에서 `DialogueAgent` 주입 시 연결
- `RumorStore`·Rumor 애그리거트 → Step C
- `RelationshipUpdated.cause`는 모든 발행 지점이 `Unspecified` → Step C/D에서 채움
- `Faction`/`Family` Scope의 NpcAllowed Join → Step C `NpcWorld` 도입 시 확장
- Rumor 테이블은 빈 상태로 선제 생성만 (스키마 안정성 목적)
