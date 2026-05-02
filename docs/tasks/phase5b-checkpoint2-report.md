# Phase 5b 체크포인트 2 보고서 — Timeline + view 메서드 4종 + Atlas overlay

> **상태**: ✅ Phase 5b 종결 — 디렉터 리뷰 대기. follow-up TASK 작성 진입 가능.
> **작업 브랜치**: `claude/event-vertical-slice-phase5a-FBIN2`
> **사양**: `docs/tasks/task-phase5b-era-timeline-vertical-slice.md`
> **작성일**: 2026-05-02

## Done

- [x] **사양 §3.1 동기화** — 디렉터 변경 `Timeline.references: Vec<EventId>` → `Vec<EraId>` + `timeline_event_refs` → `timeline_era_refs`. timeline=era 묶음 + era=event 묶음 두 단계 합성으로 정형화.
- [x] `src/domain/world/timeline.rs` — Timeline 애그리거트 + TimelineId + TimelineFilter + view 메서드 4종 (eras_in/events_in/events_during/causal_chain) + 18 단위 테스트 (BFS cycle 방지·timeline 경계 외 사일런트 포함)
- [x] `src/worldbuilding/markdown/timeline.rs` — Timeline 마크다운 파서 + 8 단위 테스트 (R4 strict typing 일관)
- [x] `genres/wuxia/markdown_template/timeline.md` 템플릿 + `genres/wuxia/forms/timeline.toml`
- [x] `SqliteWorldStore::migrate_v7` — `timelines` + `timelines_fts` (trigram) + `timeline_era_refs` 양방향 인덱스 (composite PK + idx_ter_era·idx_ter_timeline) + 8 SQLite 테스트
- [x] `WorldRepository`: `list_timelines`/`get_timeline`/`search_timelines`/`upsert_timeline`/`count_timelines` + `get_eras_batch`/`get_events_batch` 기본 구현 (Phase 5a R1·R2 패턴 — Integer bind + destructure)
- [x] `bin/world-load` 확장 — `world/timeline/*.md` 스캔 + Timeline.references 외래키 활성 + 카테고리 내 중복 금지
- [x] `bin/mind-studio` REST 3개 (`/api/world/timelines`·`/{id}`·`/search`) + MCP 도구 3개
- [x] `tests/world_load_fk_negative_timeline.rs` (4 tests) — Phase 5a N1 패턴 미러
- [x] **timeline-jungwon-history 변환** — `references = 5 era` (작성 순서 = 시간순) + body §개요/§Era 변천/§핵심 인과 사슬/§게임 시점에서의 활용
- [x] **view 메서드 4종 e2e** — `eras_in=5` / `events_in=6` / `events_during(era-fall-of-empire)=5` / `causal_chain(event-bloody-night)=6` (timeline 전체)
- [x] **Atlas overlay 양방향 시연** — atlas-jungwon.era_id ↔ era-fall-of-empire 양방향 검증
- [x] world-load 통과 — `timelines indexed=1`, fk errors=0
- [x] `cargo test --features embed --lib` → 554 passed (회귀 0건)
- [x] Phase 4 checkpoint2 e2e 1건 stale assertion 정정 (atlas era_id is_none → era-fall-of-empire)

## Diff (Phase 5b 체크포인트 1 → 체크포인트 2 누적)

```
 .../task-phase5b-era-timeline-vertical-slice.md    |  12 +-
 src/adapter/sqlite_world.rs                        | 593 ++++++++++-
 src/bin/mind-studio/handlers/mod.rs                |   3 +
 src/bin/mind-studio/main.rs                        |  15 +-
 src/bin/mind-studio/mcp_server.rs                  | 102 ++++
 src/bin/world_load.rs                              |  93 +++-
 src/domain/world/atlas.rs                          |  29 +
 src/domain/world/mod.rs                            |   2 +
 src/worldbuilding/markdown/mod.rs                  |   2 +
 src/worldbuilding/repository.rs                    |  54 +-
 tests/world_chilguk_chunchu_phase4_checkpoint2.rs  |   8 +-
 11 files changed, 896 insertions(+), 17 deletions(-)
```

신규 파일:
- `src/domain/world/timeline.rs` (~640줄, 18 단위 테스트 — view 메서드 4종 e2e + BFS cycle·경계 가드)
- `src/worldbuilding/markdown/timeline.rs` (~280줄, 8 테스트)
- `src/bin/mind-studio/handlers/world_timelines.rs` (~80줄)
- `genres/wuxia/forms/timeline.toml`
- `genres/wuxia/markdown_template/timeline.md`
- `projects/chilguk-chunchu/world/timeline/timeline-jungwon-history.md`
- `tests/world_load_fk_negative_timeline.rs` (~190줄, 4 tests)
- `examples/phase5b_checkpoint2_eval.rs` (진단용 — view 메서드 4종 + atlas overlay 시연)

## 데모 명령

```bash
# 빌드 + 테스트
cargo build --features mind-studio,chat,embed --bin npc-mind-studio
cargo test --features embed --lib                                              # 554 passed
cargo test --features embed --test world_load_fk_negative_timeline             # 4 passed
cargo test --features embed --test world_chilguk_chunchu_phase4_checkpoint{1,2} # 22 passed (회귀)

# Ingest
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload

# 정성 평가 — view 메서드 4종 + atlas overlay
cargo run --features embed --example phase5b_checkpoint2_eval
```

## 결과

```
project           = chilguk-chunchu
groups indexed    = 6
persons indexed   = 9
places indexed    = 11
atlases indexed   = 1
events indexed    = 6
eras indexed      = 5
timelines indexed = 1
errors            = 0
fk errors (활성)  = 0
```

## view 메서드 4종 e2e — `phase5b_checkpoint2_eval` 출력

### 1. `eras_in(repo)` → 5 era (작성 순서)

```
era-founding              kind=founding    [-270, -220)  key_events=1
era-prosperity            kind=prosperity  [-220, -150)  key_events=0
era-turning               kind=turning     [-150,  -70)  key_events=0
era-decline               kind=decline     [ -70,  -30)  key_events=0
era-fall-of-empire        kind=fall        [ -30,   +0)  key_events=5
```

### 2. `events_in(repo)` → 6 사건 (era.key_events 평면화 — era 순서 + era 내 작성 순서)

```
event-empire-founding               year_rel=-270  era=era-founding
event-bloody-cult-rebellion-2nd     year_rel= -30  era=era-fall-of-empire
event-blood-disappearance           year_rel= -12  era=era-fall-of-empire
event-bloody-night                  year_rel= -10  era=era-fall-of-empire
event-hwasan-fall                   year_rel= -10  era=era-fall-of-empire
event-six-states-independence       year_rel=  -7  era=era-fall-of-empire
```

1 era-founding 사건 + 5 era-fall-of-empire 사건 = 6 사건. era-prosperity·turning·decline은 Phase 5a 시드 미변환이라 평면화 결과에 사건 없음 (Phase 5b 종결 후 follow-up `task-phase5-followup-mid-era-events.md`에서 보강 예정).

### 3. `events_during(era-fall-of-empire, repo)` → 5 사건

```
event-bloody-cult-rebellion-2nd     year_rel=-30
event-blood-disappearance           year_rel=-12
event-bloody-night                  year_rel=-10
event-hwasan-fall                   year_rel=-10
event-six-states-independence       year_rel= -7
```

era-fall-of-empire의 key_events(5건)을 직접 합성. boundary 정책 §3.3 — bloody-cult-rebellion-2nd(year_relative=-30)가 era-fall-of-empire 시작 트리거로 매핑된 결과 그대로.

### 4. `causal_chain(event-bloody-night, repo)` → BFS 결과 6건

```
1. event-bloody-night                  (related: [blood-disappearance, hwasan-fall, six-states-independence])
2. event-blood-disappearance           (related: [bloody-cult-rebellion-2nd, bloody-night, hwasan-fall])
3. event-hwasan-fall                   (related: [bloody-night, blood-disappearance, bloody-cult-rebellion-2nd])
4. event-six-states-independence       (related: [bloody-night])
5. event-bloody-cult-rebellion-2nd     (related: [empire-founding, blood-disappearance])
6. event-empire-founding               (related: [bloody-cult-rebellion-2nd])
```

bloody-night → bloody-night의 related 3건(blood-disappearance·hwasan-fall·six-states-independence) → blood-disappearance의 related(bloody-cult-rebellion-2nd) → bloody-cult-rebellion-2nd의 related(empire-founding). BFS는 `visited` set으로 cycle 방지 — bloody-night ↔ hwasan-fall 양방향이 무한 루프를 만들지 않음.

**timeline 전체(6 사건)가 인과 사슬로 강하게 연결되어 있음을 시연** — Phase 5a related_events 양방향 시드의 자연스러운 결과. timeline 경계 안 transitive closure가 모든 사건을 포함.

## search_timelines 3 쿼리

| query | hits |
|---|---|
| "270년사" | timeline-jungwon-history (1건 — name 매칭) |
| "main-history" | timeline-jungwon-history (1건 — alias 매칭) |
| "칠국 역사" | (0건) |

**"칠국 역사" 0 hit 관찰**: timeline name="칠국춘추 270년사", alias=["중원사", "main-history", "270년 연표"]·body·summary 어디에도 "칠국 역사" 정확 매칭 없음. 사양 §5 Step 4 권장 쿼리지만 실제 데이터에 정확 매칭 데이터 부재 — Phase 6+에서 더 풍부한 timeline·body 추가 시 자연스럽게 매칭. **본 결과는 검색 엔진의 정확한 동작 — 데이터 부재일 뿐 동작 결함 X**. summary나 alias 보강은 follow-up 사항으로 보고서에서 명시.

## Atlas overlay 양방향 시연 (사양 §5 Step 4 핵심 검증)

```
atlas-jungwon.era_id = "era-fall-of-empire"
↳ resolved era: 붕괴기 (kind=fall, key_events=5)
↳ era-fall-of-empire를 era_id로 가지는 atlas 일람: ["atlas-jungwon"]
```

**양방향 검증 통과**:
- 정방향 (atlas → era): atlas-jungwon이 era-fall-of-empire를 외래키로 참조
- 역방향 (era → atlas): era-fall-of-empire를 era_id로 가지는 atlas 일람이 atlas-jungwon 단독

Q3 (a) 결정 그대로 — 시기별 atlas 분기는 별 atlas 인스턴스 (Phase 6+ follow-up `task-phase5-followup-era-atlases.md`).

## 외래키 결손 0건 — Phase 5b 체크포인트 2 매트릭스

```
Timeline.references ↔ eras.id: 1 timeline × 5 references = 5 매칭, 결손 0
Timeline.references 카테고리 내 중복 금지: 0 위반
```

**Phase 1·2·3·4·5a·5b 통합 검증**: world-load 결과 fk errors (활성) = 0 (모든 도메인 외래키 정합).

## FK negative e2e — `tests/world_load_fk_negative_timeline.rs` (4 tests)

체크포인트 2 manual demo의 영구 자동화:
- `rejects_timeline_references_pointing_at_missing_era` — era-99 주입 → hard-fail + DB 미수정
- `rejects_duplicate_references` — 같은 era 중복 시 차단 (composite PK 위반 사전 방지)
- `accepts_timeline_with_all_five_era_references` — Phase 5b 5 era 정합 시드 통과
- `recovers_after_fixing_missing_reference` — fail → DB 미수정 → 정정 → 통과 (체크포인트 2 demo 정확 자동화)

## 디렉터 변경 적용 — references=Vec<EraId> + 두 단계 합성

체크포인트 2 진입 시 디렉터가 사양 §3.1 변경:
- 원래: `Timeline.references: Vec<EventId>` (사건 직접 합성)
- 변경: `Timeline.references: Vec<EraId>` (era 묶음 + era=event 묶음의 두 단계 합성)
- 결과: `events_in` view가 era.key_events 평면화 — 사건 합성은 간접

이는 timeline=era 묶음의 의미를 명확히 함. timeline이 시간 컨테이너이고, era가 사건 컨테이너이며, view 메서드가 두 단계 합성을 수행. **사양 §3.1 + §5 + §6의 timeline_era_refs 일관 업데이트 완료**.

## 회귀 가드

- `cargo build --features mind-studio,chat,embed --bin npc-mind-studio` 통과
- `cargo test --features embed --lib` → 554 passed (Phase 5b 체크포인트 2 신규 — 18 timeline domain + 8 timeline markdown + 8 sqlite timeline = 34건 추가, 회귀 0)
- Phase 1·2·3·4·5a 회귀: world e2e + FK negative 모두 통과 (Phase 4 checkpoint2 stale assertion 1건 정정)
- world-load: timelines=1, fk errors=0
- `phase5b_checkpoint2_eval`: view 메서드 4종 + atlas overlay 양방향 + search 3쿼리 (실제 매칭 0건 1쿼리는 데이터 부재로 명시적 보고)

## Phase 5b 종결 후 follow-up TASK

Phase 5b 종결 사양 충족. Phase 5b 종결 후 두 follow-up TASK 작성 진입:

### 1. `task-phase5-followup-historical-npcs.md` (D2 후속)

historical npc 시드 확장 — 임서운·추양진인·바투·진대인·천마 등. Phase 5a/5b의 `(npc 미등록)` 텍스트를 정식 외래키로 승급. 6 Event + 5 Era body의 핵심 인물들이 npc 시드로 들어가게 하기.

### 2. `task-phase5-followup-mid-era-events.md` (선택)

era-prosperity·era-turning·era-decline 사건 6건 추가 변환 — 현재 key_events=0 상태 해소. 후보 사건 (history.md §0.3·§2~§4):
- 30년차 병권 회수 (era-founding 후반부 또는 era-prosperity 시작)
- 70년차 교역로 완성·상방 탄생 (era-prosperity)
- 100년차 제1차 무림대회 분쟁 (era-prosperity)
- 130년차 사파 형성기 (era-turning)
- 160년차 지방 자치 운동 시작 (era-turning)
- 190년차 혈교 잔당 첫 발견 (era-turning)
- 237년차 태무제 즉위 (era-decline)

이 follow-up이 진행되면 timeline-jungwon-history.events_in 결과가 6→13건으로 풍부해지며, era별 events_during 결과도 자연스럽게 균형이 맞춰짐.

### 3. `task-phase5-followup-era-atlases.md` (선택, Q3 ext)

시기별 atlas 분기 — 원래 Q3 (a)에서 미룬 사항. atlas-daejin-empire(통일제국 시기, era-prosperity~era-turning) + atlas-fragmentation(분열기 atlas, era-fall-of-empire) 등. 현재 atlas-jungwon은 era-fall-of-empire 시점의 단일 정치 지도이므로 다른 시점은 별도 atlas로.

## 회귀 가드 결과 요약

| 검증 | 결과 |
|---|---|
| `cargo build --features embed` | ✅ |
| `cargo build --features mind-studio,chat,embed --bin npc-mind-studio` | ✅ |
| `cargo test --features embed --lib` | ✅ 554 passed |
| `cargo test --features embed --test world_load_fk_negative_timeline` | ✅ 4 passed |
| world-load Phase 1~5b 통합 ingest | ✅ timelines=1, fk errors=0 |
| Phase 1·2·3·4·5a e2e 회귀 | ✅ 모두 통과 (Phase 4 stale 1건 정정) |
| view 메서드 4종 e2e (eras_in 5 / events_in 6 / events_during 5 / causal_chain 6 BFS) | ✅ |
| Atlas overlay 양방향 시연 | ✅ |
| search_timelines 3쿼리 | ✅ 2/3 정확 매칭 + 1쿼리 데이터 부재 명시 |
