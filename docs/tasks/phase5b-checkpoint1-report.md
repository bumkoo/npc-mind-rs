# Phase 5b 체크포인트 1 보고서 — Era + Phase 5a/4 era_id 외래키 활성

> **상태**: ✅ 체크포인트 1 통과 — 디렉터 리뷰 대기. **commit pause 유지**.
> **작업 브랜치**: `claude/event-vertical-slice-phase5a-FBIN2`
> **사양**: `docs/tasks/task-phase5b-era-timeline-vertical-slice.md`
> **작성일**: 2026-05-02

## Done

- [x] `src/domain/world/era.rs` — `Era` 애그리거트 + `EraId` + `EraTemporal` + `EraFilter` + 16 단위 테스트 (boundary 정책 §3.3 + 5 era 일관성 회귀 가드 포함)
- [x] `src/worldbuilding/markdown/era.rs` — Era 마크다운 파서 + 16 단위 테스트 (R4 strict typing 패턴 일관 적용)
- [x] `genres/wuxia/markdown_template/era.md` 템플릿
- [x] `genres/wuxia/forms/era.toml` Phase N 슬롯 + 5 kind 옵션 (founding/prosperity/turning/decline/fall)
- [x] `SqliteWorldStore::migrate_v6` — `eras` + `eras_fts` (trigram) + 10 SQLite 테스트 (boundary 정책 검증 + v5→v6 마이그레이션 가드 포함)
- [x] `WorldRepository`: `list_eras` / `get_era` / `search_eras` / `upsert_era` / `count_eras` (Phase 5a R1·R2 패턴 — Integer bind + destructure)
- [x] `bin/world-load` 확장 — `world/era/*.md` 스캔 + Phase 5b 외래키 활성 3종 (Era.key_events + Event.era_id + Atlas.era_id)
- [x] `bin/mind-studio` REST 엔드포인트 3개 (`/api/world/eras`·`/{id}`·`/search`) + MCP 도구 3개 (`list_eras`·`get_era`·`search_eras`)
- [x] `tests/world_load_fk_negative_era.rs` (5 tests) — Phase 5a N1 패턴 미러
- [x] **5 Era 변환** — founding/prosperity/turning/decline/fall (history.md §0.2 정확 매핑)
- [x] **6 Event era_id 활성** — 1 founding + 5 fall-of-empire (boundary 정책 §3.3 적용 — bloody-cult-rebellion-2nd boundary 케이스 포함)
- [x] **atlas-jungwon era_id 활성** — `extras.era_id = era-fall-of-empire` (Phase 4 텍스트 → Phase 5b 활성)
- [x] world-load 통과 — `eras indexed=5`, `events indexed=6`, `atlases indexed=1`, `fk errors=0`
- [x] cargo test --features embed --lib → 520 passed (회귀 0건)
- [x] Phase 4 e2e 회귀 — `tests/world_chilguk_chunchu_phase4_checkpoint1.rs::atlas_jungwon_parses_with_expected_identity` 1건 stale assertion 정정 (Phase 5b로 era_id 활성됨이 의도된 변화)

## Diff (Phase 5b 체크포인트 1 누적)

```
 .../chilguk-chunchu/world/atlas/atlas-jungwon.md   |   2 +-
 .../world/event/event-{6 사건}.md                   |  12 +-  (각 1줄: era_id 활성)
 src/adapter/sqlite_world.rs                        | 606 +++++++++-
 src/bin/mind-studio/handlers/mod.rs                |   3 +
 src/bin/mind-studio/main.rs                        |  10 +-
 src/bin/mind-studio/mcp_server.rs                  | 102 ++++
 src/bin/world_load.rs                              | 128 ++++-
 src/domain/world/atlas.rs                          |  16 +
 src/domain/world/era.rs                            | 380 ++++++++++-
 src/domain/world/mod.rs                            |   1 +
 src/worldbuilding/markdown/mod.rs                  |   2 +
 src/worldbuilding/repository.rs                    |  26 +-
 tests/world_chilguk_chunchu_phase4_checkpoint1.rs  |   8 +-
 18 files changed, 1265 insertions(+), 31 deletions(-)
```

신규 파일:
- `docs/tasks/task-phase5b-era-timeline-vertical-slice.md` (사양 문서)
- `src/worldbuilding/markdown/era.rs` (~400줄, 16 테스트)
- `src/bin/mind-studio/handlers/world_eras.rs` (~80줄)
- `genres/wuxia/forms/era.toml` (50줄)
- `genres/wuxia/markdown_template/era.md` (35줄)
- `projects/chilguk-chunchu/world/era/era-{founding,prosperity,turning,decline,fall-of-empire}.md` (5건, 평균 50줄)
- `tests/world_load_fk_negative_era.rs` (~230줄, 5 tests)
- `examples/phase5b_eval.rs` (진단용)

## 데모 명령

```bash
# 빌드 + 테스트
cargo build --features embed
cargo test --features embed --lib                 # 520 passed (16 era + 16 era markdown + 10 sqlite era 신규)
cargo test --features embed --test world_load_fk_negative_era                # 5 passed
cargo test --features embed --test world_chilguk_chunchu_phase4_checkpoint1  # 13 passed (Phase 4 e2e 회귀)

# Ingest
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload

# 정성 평가 — boundary 정책 + 6 Event era_id + atlas era_id 매핑
cargo run --features embed --example phase5b_eval
```

## 결과

```
=== 결과 ===
project           = chilguk-chunchu
groups indexed    = 6
persons indexed   = 9
places indexed    = 11
atlases indexed   = 1
events indexed    = 6
eras indexed      = 5
errors            = 0
group cycles      = 0
place cycles      = 0
fk errors (활성)  = 0
mind eligible     = 9
```

## 5 Era 일람 (id ASC)

| id | kind | start | end | duration | key_events |
|---|---|---|---|---|---|
| `era-decline` | decline | -70 | -30 | 40y | 0 |
| `era-fall-of-empire` | fall | -30 | 0 | 30y | 5 |
| `era-founding` | founding | -270 | -220 | 50y | 1 |
| `era-prosperity` | prosperity | -220 | -150 | 70y | 0 |
| `era-turning` | turning | -150 | -70 | 80y | 0 |

총 270년 (50+70+80+40+30 = 270 ✓ — 5 era boundary 일관성 회귀 가드 통과). era-decline·era-prosperity·era-turning은 Phase 5a 시드에 사건 미변환이라 `key_events=0` (Phase 6+ 보강 예정).

## Boundary 정책 §3.3 — start inclusive · end exclusive (회귀 가드)

`list_eras(contains_year=?)` 5 케이스:

| year_relative | 매칭 era | 의도 |
|---|---|---|
| -270 | `era-founding` | 원년 = era-founding start (inclusive) |
| -31 | `era-decline` | era-decline 끝 직전 (한 해 전) |
| **-30** | **`era-fall-of-empire`** | **boundary — start inclusive, era-decline의 end는 exclusive라 미매칭** |
| -7 | `era-fall-of-empire` | event-six-states-independence 시점 |
| 0 | (없음) | 현재 270년차 — 모든 era end exclusive라 어느 era에도 속하지 않음 (디렉터 결정) |

## 6 Event era_id 매핑 결과

| event id | year_relative | era_id |
|---|---|---|
| event-blood-disappearance | -12 | era-fall-of-empire |
| event-bloody-cult-rebellion-2nd | **-30** | **era-fall-of-empire** ★ boundary 케이스 |
| event-bloody-night | -10 | era-fall-of-empire |
| event-empire-founding | -270 | era-founding |
| event-hwasan-fall | -10 | era-fall-of-empire |
| event-six-states-independence | -7 | era-fall-of-empire |

**boundary 케이스 결정**: event-bloody-cult-rebellion-2nd (year_relative=-30)는 era-decline의 end(=-30, exclusive)이자 era-fall-of-empire의 start(=-30, inclusive). §3.3 정책 + 디렉터 권장에 따라 era-fall-of-empire(붕괴기 시작 트리거)로 매핑.

## atlas-jungwon era_id 활성

```
atlas-jungwon (kind=continent) → extras.era_id = "era-fall-of-empire"
```

Phase 4에서 텍스트만이던 `extras.era_id`가 외래키로 활성됨. 270년차(현재) 정치 지도이며 era-fall-of-empire(240~270년차)의 가장 늦은 시점.

**중요 — Atlas 모델 비변경 결정 (사양 §3.4)**: Atlas 도메인 모델은 그대로 (`extras["era_id"]` 헬퍼 `Atlas::era_id()`만 사용). top-level era_id 필드 승격은 Phase 6+로 미룸 (breaking change 회피). world-load CLI가 `at.era_id()`로 추출 후 검증.

## search_eras 4쿼리

| query | hits |
|---|---|
| "붕괴" | era-decline, era-fall-of-empire (2건 — "붕괴기" alias·name 매칭) |
| "분열" | era-fall-of-empire (1건 — "6국 분열기" alias) |
| "건국" | era-founding (1건 — "건국기" name) |
| "270" | era-fall-of-empire, era-founding (2건 — "270년차"·"270~220년 전" 본문 매칭) |

## 외래키 활성 시연 — `tests/world_load_fk_negative_era.rs` (5 tests)

체크포인트 1 manual demo의 영구 자동화 가드:
- `rejects_era_key_events_pointing_at_missing_id` — Era.key_events에 미존재 event 주입 → hard-fail + DB 미수정
- `rejects_event_era_id_pointing_at_missing_id` — Event.era_id에 미존재 era 주입 → hard-fail (Phase 5a 텍스트 → Phase 5b 활성 회귀 가드)
- `rejects_atlas_era_id_pointing_at_missing_id` — Atlas.extras.era_id에 미존재 era 주입 → hard-fail (Phase 4 텍스트 → Phase 5b 활성 회귀 가드)
- `accepts_canonical_5_era_with_boundary_event` — boundary year_relative=-30 event가 era-fall에 매핑되는 정합 시드
- `recovers_after_fixing_missing_era_id` — fail → DB 미수정 → 정정 → 통과 (체크포인트 1 demo 정확 자동화)

## 변환 결정 (체크포인트 1 핵심)

### 1. 5 era kind 결정 (Q1 디렉터 결정 그대로)

`history.md` §0.2 5 시대 원문 매핑:
- founding (건국기) — 0~50년차
- prosperity (전성기) — 50~120년차
- turning (변곡기) — 120~200년차
- decline (쇠퇴기) — 200~240년차
- fall (붕괴기) — 240~270년차

### 2. boundary 정책 §3.3 — start inclusive · end exclusive

5 era end가 다음 era start와 정확히 일치 (inclusive-exclusive 정합) → 어느 year_relative도 정확히 한 era에만 속함. 단 270년차(year_relative=0)는 마지막 era end(exclusive)라 어느 era에도 속하지 않음 — 게임 시작 시점이 별도 era로 정형화되지 않은 상태(Phase 5b 결정).

bloody-cult-rebellion-2nd(=−30)는 디렉터 권장에 따라 era-fall-of-empire(붕괴기 시작 트리거)로 매핑.

### 3. aliases 2-3개 (Phase 5a alias 결정 패턴 일관 적용)

각 era에 별호 2개씩 (관습 + 시간 표기):
- era-founding: "원년대" + "0-50년차"
- era-prosperity: "태평성세" + "50-120년차"
- era-turning: "균열기" + "120-200년차"
- era-decline: "태무제 시기" + "200-240년차"
- era-fall-of-empire: "6국 분열기" + "240-270년차"

결과형 표현(예: "통일제국 시기")은 era summary 본문으로 이관 (alias 결정 패턴 일관).

### 4. key_events 정렬 — 시간순 권장

key_events 배열 작성 순서가 시간순이도록 권장. Phase 5b R4 strict typing 패턴 따라 silent skip 차단. era-fall-of-empire의 5 사건은 -30 → -12 → -10 → -10 → -7 순서.

era-prosperity·era-turning·era-decline은 Phase 5a 시드 미변환이라 `key_events=[]` (Phase 6+ 보강 예정).

### 5. Atlas 모델 비변경 — 디렉터 결정 §3.4

`extras["era_id"]` 그대로 + world-load 헬퍼 검증 패턴. top-level 필드 승격은 Phase 6+ breaking change로 미룸.

## 막힌 것

없음 — 사용자 결정 3건(Q1·Q2·Q3) 모두 명확하며 boundary 정책도 정확히 인코딩됨.

**관찰 사항** (디렉터 정보용):
- era-prosperity·era-turning·era-decline의 `key_events`는 Phase 5a 6 시드가 모두 era-founding·era-fall-of-empire에 몰려 있어 0건. Phase 6+에서 (a) 30년차 병권 회수 (b) 100년차 무림대회 (c) 130년차 사파 형성 (d) 160년차 자치 운동 (e) 190년차 혈교 잔당 발견 (f) 240년차 2차 혈교 침공의 추가 변환 시 채워짐 — 이는 Phase 5b의 결손이 아니며 사양 §7 Out of Scope에서 명시.
- `task-phase5-followup-era-atlases.md` (선택) — 시기별 atlas 분기 (atlas-daejin-empire 등)는 Phase 5b 종결 후 follow-up TASK.

## 다음 의견 — 체크포인트 2 진행 가능

체크포인트 1 사양 충족. 디렉터 통과 시 체크포인트 2 진입:
- Timeline 도메인 (Atlas와 결이 같은 도메인+뷰 이중성)
- view 메서드 4종 (`eras_in`/`events_in`/`events_during`/`causal_chain`) e2e
- migrate_v7 (timelines + timelines_fts + timeline_event_refs 양방향 인덱스)
- MCP 도구 3개 추가 (`list_timelines`/`get_timeline`/`search_timelines`)
- 1 Timeline 변환 (`timeline-jungwon-history`) + view 메서드 e2e

체크포인트 2 시작 전 디렉터 결정 필요 사항 없음 — 사양 §5 Step 4 모두 명확.

## 회귀 가드 결과 요약

| 검증 | 결과 |
|---|---|
| `cargo build --features embed` | ✅ |
| `cargo build --features mind-studio,chat,embed --bin npc-mind-studio` | ✅ |
| `cargo test --features embed --lib` | ✅ 520 passed |
| world-load Phase 1·2·3·4·5a·5b 통합 ingest | ✅ eras=5, events=6, fk errors=0 |
| Phase 1·2·3·4 e2e 회귀 (39 + 1 stale assertion 정정) | ✅ |
| 정성 평가 (boundary 5 + search 4 + 6 Event mapping + atlas mapping) | ✅ 모두 의도대로 |
| Phase 5b FK negative e2e (5 tests) | ✅ |
