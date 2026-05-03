# Phase 5a 체크포인트 1 보고서 — Event Vertical Slice

> **상태**: ✅ 체크포인트 1 통과 — 디렉터 리뷰 대기. **commit pause 유지**.
> **작업 브랜치**: `claude/event-vertical-slice-phase5a-FBIN2`
> **사양**: `docs/tasks/task-phase5a-event-vertical-slice.md`
> **작성일**: 2026-05-02

## Done

- [x] Event 애그리거트 — `EventId` · `EventCategory` · `EventTemporal` · `ParticipantsRefs` · `Event` · `EventFilter` (`src/domain/world/event.rs`)
- [x] Event 마크다운 파서 (`src/worldbuilding/markdown/event.rs`) + 23 단위 테스트 + 한국어 wuxia 라운드트립
- [x] `WorldRepository` 확장 — `list_events` / `get_event` / `search_events` / `upsert_event` / `count_events`
- [x] `SqliteWorldStore.migrate_v5` — `events` + `events_fts` (trigram) + `event_participants_refs` 양방향 인덱스 (composite PK + 카테고리별 부분 인덱스 3종)
- [x] `bin/world-load` 확장 — `world/event/*.md` 스캔 + Phase 5a 외래키 활성 (people·groups·places·related_events + 카테고리 내 중복 금지)
- [x] `genres/wuxia/markdown_template/event.md` 템플릿
- [x] `genres/wuxia/forms/event.toml` Phase N 폼 자리 (kind 6종 + category 3종 + era_id + year_relative)
- [x] `projects/chilguk-chunchu/world/event/event-bloody-night.md` 변환
- [x] **외래키 활성 시연** — 의도적 `npc-99` 주입 → 빌드 실패(DB 미수정) → 복구 → 빌드 성공
- [x] `cargo build` (default + embed) + `cargo test --features embed --lib` (470 passed, 0 failed) + `cargo test --features embed --tests` (회귀 0 — 6 embed_test 실패는 Phase 5a 무관, ONNX 모델 미배치)

## Diff (Phase 5a Step 1·2·3 누적)

```
 src/adapter/sqlite_world.rs       | 853 ++++++++++++++++++++++++++++++++++
 src/bin/world_load.rs             | 179 +++++++-
 src/domain/world/atlas.rs         |  16 +
 src/domain/world/event.rs         | 468 ++++++++++++++++++++-
 src/domain/world/mod.rs           |   3 +
 src/worldbuilding/markdown/mod.rs |   2 +
 src/worldbuilding/repository.rs   |  31 +-
 7 files changed, 1535 insertions(+), 17 deletions(-)
```

신규 파일:
- `src/worldbuilding/markdown/event.rs` (468줄, 23 테스트)
- `genres/wuxia/forms/event.toml` (54줄)
- `genres/wuxia/markdown_template/event.md` (47줄)
- `projects/chilguk-chunchu/world/event/event-bloody-night.md` (107줄)
- `examples/dump_bloody_night.rs` (진단용 — JSON dump + 필터 시연)

## 데모 명령

```bash
# 빌드 + 테스트
cargo build --features embed
cargo test --features embed --lib                 # 470 passed (16 event domain + 23 event markdown + 9 sqlite event)
cargo test --features embed --test world_chilguk_chunchu_e2e \
                              --test world_chilguk_chunchu_phase4_checkpoint1 \
                              --test world_chilguk_chunchu_phase4_checkpoint2 \
                              --test world_load_fk_negative   # 모두 통과 (Phase 1·2·3·4 회귀 0)

# Ingest
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload

# 진단 — get_event / count_events / list_events(participants_person="npc-02")
cargo run --features embed --example dump_bloody_night
```

세 결과:
1. ingest: `events indexed = 1` · `fk errors (활성) = 0` · 기존 4 도메인 회귀 0건
2. `count_events(project="chilguk-chunchu") = 1`
3. `list_events(participants_person="npc-02")` → `event-bloody-night (붉은 밤의 변, year_relative=Some(-10))`

## 외래키 활성 시연 (Phase 1·2·3·5a)

`event-bloody-night.md`의 `participants.people`에 의도적으로 `npc-99`(미존재) 주입 → world-load 재실행:

```
[world-load] ✗ Phase 5a 외래키 활성: events.participants.people 결손 1 건:
  - event-bloody-night: participants.people 'npc-99' (persons.id에 없음)

=== 결과 (DB 미수정) ===
project           = chilguk-chunchu
events parsed     = 1
fk errors (활성)  = 1
world-load 실패: 1 외래키 결손 — Phase 2·3·4·5a 활성. DB 미수정. .md 수정 후 재실행하세요.
```

`npc-99`를 제거하고 재실행 → `events indexed = 1`·`fk errors = 0` 정상 회복. **Phase 4 패턴 그대로 — partial commit 방지 검증됨.**

## 변환 시 결정한 것 (체크포인트 1 핵심)

### 1. `kind = "betrayal"` (war/disaster 후보 중)

근거:
- `history.md` §0.3 역추적 갈등 씨앗 매트릭스에서 ⑤ "조고의 악행 연쇄"(영주→피의 실종→화산파)
  로 분류 — 핵심 본질이 권력 투쟁의 배반.
- `history-characters.md` §11.1는 조고를 "붉은 밤의 기획자/실행자"로 명시.
- `war`(2차 혈교 침공)는 240년차로 별도 사건. `disaster`(재해)는 자연재해 내포. `betrayal`이 가장 부합.

### 2. `aliases` 2건 (3건 후보 중)

- `"붉은 밤"` (축약형 — NPC 일상 화법)
- `"10년 전 변란"` (현재 시점 NPC가 시간을 들먹일 때 쓰는 표현)
- `"6국 독립의 시작"`은 결과형 표현이라 `extras.outcome`으로 이관 (사건 본질의 alias 아님).

### 3. `temporal.year_relative = -10` + `year = "10년 전 (260년차)"`

- 270년차 기준 절대 연도 -10 — `history.md` §6 연표 요약표 그대로.
- `year` 자유 텍스트는 NPC 화법 톤 보존.
- `duration = "사흘 밤"` — `history.md` §5.1는 `[기존 초안 유지]` 표기로 명시값 없으나, 게임 캐논 결정값으로 채택. notes에 명시.

### 4. `participants.people` (4명 — 권장 3명에서 1명 추가)

- `npc-02 조고` — 사건 기획자 (history-characters §11.1)
- `npc-07 천순제` — 즉위 직후 꼭두각시 황제 (§11.1)
- `npc-01 명경` — 아미파 정보망. 인지자 (§11.1)
- `npc-03 남궁혁` — 30대 초반, 화산파 멸문 관여 의혹 (§11.1) — 사양 §6.7 "권장 3명"에 추가 1명

미등록 인물 (의도적 누락):
- `임서운` (player 부친) — Phase 2 npc 미등록. body_sections `## 핵심 인물`에 `(npc 미등록)`로 표기.
- `추양진인` (화산 장문인) — Phase 2 npc 미등록. 동일 표기.

### 5. `participants.groups` (3개)

- `group-daejin-court` — 영토 와해의 주체 황실
- `group-shipsangsi` — 조고의 권력 결사
- `group-mulim-mang` — 사건 후 정파 동맹 흔들림

**막힌 결정**: `혈교 잔당`. Phase 1 그룹 미등록 — `participants.groups`에 누락 처리. 디렉터 검토 필요.

### 6. `participants.places` (3곳)

- `place-daejin` — 사건 무대 (황실 와해)
- `place-namgung` — 6 지역 중 첫 독립
- `place-jiyu-doshi` — 5년 후 옛 영주 위 자연 형성 (사건의 후속 인과)

### 7. `era_id = ~` (잠정 비움)

Phase 5a엔 텍스트만 보존되며 검증 비활성. Phase 5b Era 도메인 진입 시 `"era-fall-of-empire"` 또는 `"era-decline"` 결정 예정. notes 필드에 마이그레이션 의도 명시.

### 8. `related_events = []`

체크포인트 1 단독 사건이라 비움. Step 4(체크포인트 2)에서 `event-hwasan-fall`·`event-six-states-independence`·`event-blood-disappearance` 등 추가 시 양방향 채울 예정.

## 막힌 것 — 디렉터 결정 필요

### D1. 혈교 잔당 그룹 미등록 처리

Phase 1 그룹 시드(`projects/chilguk-chunchu/world/group/`)에 혈교(血敎)가 없음. 본 사건의 핵심 인과 중 "혈교 잔당의 침투"가 빠지는 셈. 세 옵션:

| 옵션 | 장점 | 단점 |
|---|---|---|
| (a) Phase 1 group `group-blood-cult-remnants` 추가 | participants.groups 정합 | Phase 1 시드 확장 — 별도 변환 단계 추가 + 사후 리뷰 부담 |
| (b) Phase 6+ legendary group으로 미루기 | Phase 1 시드 손대지 않음 | 본 사건의 인과 사슬에 빈자리 |
| (c) 영구 누락 (현재) | 진행 중단 없음 | "혈교가 사건의 트리거 일부" 정보가 데이터에 없음 |

**현재 (c) 채택**, body_sections `## 핵심 인물`엔 텍스트 명시. 결정 요청.

### D2. 임서운·추양진인 npc 미등록

Phase 2 npc 시드에 두 인물 부재(`npc-08~10` 비어있음). 본 사건의 핵심 인물이라 후속 변환 시 npc 추가가 필요할 수 있다 — 특히 임서운은 player의 부친이자 Phase 2 시드에서 player.md가 직접 언급. 후속 phase 시드 확장 결정 요청.

### D3. Phase 5b era_id 식별자

본 사건의 `era_id`는 `era-fall-of-empire`(붕괴기 240~270년차) 또는 `era-decline`(쇠퇴기 200~240년차) 중 어느 쪽? `history.md` §0.2는 240년부터 붕괴기로 잡으나 본 사건은 260년차라 명확히 붕괴기. Phase 5b 진입 시 디렉터 1차 결정.

## 다음 의견 — Step 4 (체크포인트 2) 진행 가능

Step 1·2·3 완료. 체크포인트 1의 디렉터 시그널 받으면 Step 4 진입 가능:
- 4-9 Event 추가 (사양 §5 Step 4 후보 7건 중 5-7건):
  - `event-empire-founding` (270년 전)
  - `event-bloody-cult-rebellion-2nd` (30년 전, 2차 혈교 침공)
  - `event-blood-disappearance` (12년 전, 피의 실종)
  - `event-hwasan-fall` (10년 전, 화산파 멸문 — 붉은 밤 직후, related_events 양방향)
  - `event-six-states-independence` (5-7년 전, 칠국 형성)
- MCP 도구 3개: `list_events` · `get_event` · `search_events`
- 정성 평가: 6 쿼리 — "혈교"·"붉은 밤"·"임서운"·"화산"·"건국"·"독립"

체크포인트 2 권장 = 5-7건. 디렉터의 후보 우선순위·범위 시그널 받기.

## 회귀 가드

- Phase 1·2·3·4 e2e 통과: `world_chilguk_chunchu_e2e` 9 · `phase4_checkpoint1` 7 · `phase4_checkpoint2` 7 · `world_load_fk_negative` 3 — 모두 통과
- Phase 5a 신규 단위 테스트:
  - 16 event domain (`domain::world::event::tests`)
  - 23 event markdown (`worldbuilding::markdown::event::tests`, 한국어 wuxia 라운드트립 포함)
  - 9 sqlite event (`adapter::sqlite_world::tests` — 라운드트립 · 양방향 인덱스 · v4→v5 마이그레이션 · participants 필터 · year_relative 범위 · FTS · stale row 교체 · count·project 필터)
- `migrate_v5` `IF NOT EXISTS`라 v5 신규 DB·v4 file → v5 file 모두 안전. v4 파일 마이그레이션 가드 테스트 추가됨 (`schema_v4_to_v5_migration_upgrades_existing_file_db`).
