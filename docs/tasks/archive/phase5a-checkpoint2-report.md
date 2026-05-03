# Phase 5a 체크포인트 2 보고서 — 6 Event 변환 + MCP 도구 + 정성 평가

> **상태**: ✅ Phase 5a 종결 — Phase 5b TASK 작성 진입 가능.
> **작업 브랜치**: `claude/event-vertical-slice-phase5a-FBIN2`
> **사양**: `docs/tasks/task-phase5a-event-vertical-slice.md`
> **작성일**: 2026-05-02

## Done

- [x] **6 Event 변환** (5 신규 + bloody-night related_events 양방향 추가)
  - `event-empire-founding` (-270, 시대 시작점)
  - `event-bloody-cult-rebellion-2nd` (-30, 황실 권위 손상 분기점)
  - `event-blood-disappearance` (-12, 사전 정지 작업)
  - `event-bloody-night` (-10, 체크포인트 1 + related_events 양방향)
  - `event-hwasan-fall` (-10, bloody-night 양방향 시연)
  - `event-six-states-independence` (-7, 칠국 형성)
- [x] **외래키 결손 0건** (Phase 1·2·3·4·5a 매트릭스 — `fk errors (활성) = 0`)
- [x] **MCP 도구 3개** — `list_events` · `get_event` · `search_events` (`mcp_server.rs`, embed feature gated)
- [x] **REST 엔드포인트 3개** — `/api/world/events` · `/api/world/events/{id}` · `/api/world/events/search` (`handlers/world_events.rs`)
- [x] `world_events.rs` REST + `mcp_server.rs` MCP 일치 패턴 (`world_atlases.rs` 미러)
- [x] 정성 평가 6 search 쿼리 모두 의도대로 매칭
- [x] `list_events(participants_person="npc-02")` → 조고 관여 3 사건 (blood-disappearance + bloody-night + hwasan-fall)
- [x] `list_events(participants_person="npc-01")` → 명경 관여 2 사건 (30년 전 혈교 침공 + bloody-night)
- [x] `list_events(year_relative_min=-30, year_relative_max=0)` → 5 사건 (empire-founding 제외)
- [x] `related_events` 양방향 시연 — bloody-night ⊃ hwasan-fall ∧ hwasan-fall ⊃ bloody-night
- [x] **사이드 픽스**: `state.rs`의 사라진 trait 이름 `LlamaServerMonitor` → `InferenceServerMonitor`로 정정 (이름 변경 후 미정리된 2줄. mind-studio 빌드 차단 해제용. Phase 5a 작업 외 — 검증을 위한 prerequisite)
- [x] `cargo build --features mind-studio,chat,embed --bin npc-mind-studio` 통과
- [x] `cargo test --features embed --lib` → 470 passed (Phase 5a 회귀 0)
- [x] `cargo test --features embed --test world_chilguk_chunchu_*` (Phase 1·2·3·4 e2e 39 tests) 모두 통과

## Diff (체크포인트 1 → 체크포인트 2 누적)

```
 .../world/event/event-bloody-night.md              |   5 +-
 src/bin/mind-studio/handlers/mod.rs                |   3 +
 src/bin/mind-studio/main.rs                        |  10 +-
 src/bin/mind-studio/mcp_server.rs                  | 135 ++++++++++++++++++
 src/bin/mind-studio/state.rs                       |   4 +-
 5 files changed, 153 insertions(+), 4 deletions(-)
```

신규 파일 (체크포인트 2):
- `src/bin/mind-studio/handlers/world_events.rs` (94줄)
- `projects/chilguk-chunchu/world/event/event-empire-founding.md`
- `projects/chilguk-chunchu/world/event/event-bloody-cult-rebellion-2nd.md`
- `projects/chilguk-chunchu/world/event/event-blood-disappearance.md`
- `projects/chilguk-chunchu/world/event/event-hwasan-fall.md`
- `projects/chilguk-chunchu/world/event/event-six-states-independence.md`
- `examples/phase5a_eval.rs` — 진단용 정성 평가 CLI

## 데모 명령

```bash
# Ingest
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload
# → events indexed = 6 · fk errors (활성) = 0 · 회귀 0건

# 정성 평가 — 한 번에 전부
cargo run --features embed --example phase5a_eval

# Mind Studio 빌드 (REST + MCP)
cargo build --features mind-studio,chat,embed --bin npc-mind-studio
```

## 6 Event 일람 (id ASC)

| id | kind | year_relative | related_events |
|---|---|---|---|
| `event-blood-disappearance` | betrayal | -12 | bloody-cult-2nd · bloody-night · hwasan-fall |
| `event-bloody-cult-rebellion-2nd` | war | -30 | empire-founding · blood-disappearance |
| `event-bloody-night` | betrayal | -10 | blood-disappearance · hwasan-fall · six-states-independence |
| `event-empire-founding` | founding | -270 | bloody-cult-rebellion-2nd |
| `event-hwasan-fall` | disaster | -10 | bloody-night · blood-disappearance · bloody-cult-rebellion-2nd |
| `event-six-states-independence` | founding | -7 | bloody-night |

(category=historical, era_id=~, 모두 Phase 5b에서 외래키 활성 예정)

## 인과 사슬 시연

```
empire-founding (-270)
  ├─ bloody-cult-rebellion-2nd (-30) ──┐
  │                                    │
  └← (역참조) ← ← ←                    │
                                       ↓
                       blood-disappearance (-12) ─┐
                                                  │
                                   bloody-night (-10)  ←→  hwasan-fall (-10)
                                       │       │ (양방향 시연 — 디렉터 핵심 검증)
                                       ↓       ↓
                       six-states-independence (-7)
```

related_events는 free-form Vec<EventId>이라 작성 순서 = 인과 의도. 양방향 cycle 가드는 Phase 5+에서 검토 (사양 §3.2 — Phase 5a엔 단순 결손만 검증).

## 변환 결정 — 체크포인트 1 패턴 일관 적용

| 사건 | kind 결정 | aliases 패턴 |
|---|---|---|
| empire-founding | **founding** (war보다 포괄적 — 무림맹·구파일방·대진·맹약 모두 결성) | "창세전쟁" + "원년의 약속" — 사건 본질 별호. "270년 전 건국"은 시간 화법 |
| bloody-cult-rebellion-2nd | **war** (대규모 군사 침공·격퇴) | "혈교 부활" + "240년차 혈교 전쟁" — 부활은 본질, 240년차는 역사 화법 |
| blood-disappearance | **betrayal** (조고의 정치 숙청, history.md §0.3 ⑤ "악행 연쇄"에 포함) | "12년 전 실종" + "화산 정보망 절단" — 본질·시간 두 결 |
| bloody-night | **betrayal** (체크포인트 1 결정 그대로) | "붉은 밤" + "10년 전 변란" |
| hwasan-fall | **disaster** (한 문파의 절멸 — disaster가 결과형 표현, betrayal보다 정확) | "260년 화산파 절멸" + "화산 잔존자" — player 관점 화법 |
| six-states-independence | **founding** (6 정치체 신규 결성이 5년 흐름의 본질) | "칠국 형성" + "263-265년 분열" |

**alias 결정 패턴 일관 검증**: 결과형 표현("칠국춘추의 시작", "6국 독립의 시작" 등)은 모두 `extras.outcome`으로 이관, alias엔 사건 본질 별호 + 시간/관점 화법만. 시간 표기는 `temporal.year` 자유 텍스트로 분리.

## 정성 평가

### list_events(participants_person="npc-02") — 조고 관여 사건

```
event-blood-disappearance      year_rel=-12   (12년 전 피의 실종 — 배후)
event-bloody-night             year_rel=-10   (붉은 밤 — 기획자/실행자)
event-hwasan-fall              year_rel=-10   (화산파 멸문 — 기획자 의심)
```

3 사건 모두 history.md §0.3 ⑤ "조고의 악행 연쇄"(피의실종→화산파→붉은 밤)에 매칭. **history-characters §0.3·§10·§11 정합성 확인.**

### list_events(participants_person="npc-01") — 명경 관여 사건

```
event-bloody-cult-rebellion-2nd year_rel=-30   (30년 전 19세 종군, 단운 부상 치료)
event-bloody-night              year_rel=-10   (49세, 정파 정보망으로 인지)
```

2 사건 모두 history-characters §9.1 + §11.1에서 명시된 명경의 핵심 기억. **30년 시점 차이가 명경 NPC의 단운에 대한 복잡한 감정의 기원으로 이어짐 (게임 내 캐릭터 깊이의 데이터 근거).**

### list_events(year_relative_min=-30, year_relative_max=0) — 30년 전 ~ 현재

```
5 사건 (empire-founding -270 제외)
- bloody-cult-rebellion-2nd  (-30)
- blood-disappearance        (-12)
- bloody-night               (-10)
- hwasan-fall                (-10)
- six-states-independence    (-7)
```

쇠퇴기·붕괴기 분기점이 5 사건으로 정형화되어 270년차 게임 시점에서 NPC들이 직접 기억할 수 있는 사건 일람으로 사용 가능.

### list_events(year_relative_min=-300, year_relative_max=-100)

```
1 사건 — empire-founding (-270)
```

변곡기 이전 100년 (170~270년차) 시드 데이터의 일관성 검증. **추가 사건 미시드 — Phase 5b·6+에서 보강 예정** (예: 30년차 병권 회수, 100년차 무림대회, 130년차 사파 형성 등 현재 미변환).

### search_events 6쿼리

| query | hits | 의도 매칭 |
|---|---|---|
| "혈교" | 5 | empire-founding · bloody-cult-2nd · blood-disappearance · bloody-night · hwasan-fall — 모두 혈교 인과 사슬에 위치. 정확. |
| "붉은 밤" | 4 | bloody-night + 그 related_events 사건들 (bloody-night을 본문/related에서 언급하는 사건). 정확. |
| "임서운" | 3 | bloody-night · hwasan-fall · blood-disappearance — 모두 임서운 언급. 정확. |
| "화산" | 5 | 화산파 관련 5 사건 모두. 정확. |
| "건국" | 1 | **empire-founding 단독** — 270년 전 건국이라는 본질 정확 매칭. (six-states-independence는 "건국" 미사용, "신규 결성" 어휘) |
| "독립" | 2 | six-states-independence + bloody-night — 후자는 outcome에 "6 지역 독립 운동" 명시. 인과 사슬상 정확. |

FTS5 trigram 토크나이저가 한국어·한자·영어 모두 정합. **6 쿼리 모두 의도된 사건 정확 매칭.**

## 외래키 결손 검증 — Phase 1·2·3·4·5a 매트릭스

```
=== 결과 ===
project           = chilguk-chunchu
groups indexed    = 6
persons indexed   = 9
places indexed    = 11
atlases indexed   = 1
events indexed    = 6
groups parsed     = 6
persons parsed    = 9
places parsed     = 11
atlases parsed    = 1
events parsed     = 6
errors            = 0
group cycles      = 0
place cycles      = 0
fk errors (활성)  = 0
mind eligible     = 9
```

**Phase 5a 활성 외래키 (Event)**:
- `participants.people` ↔ `persons.id`: 6 사건 × 평균 2 인물 = 약 11 매칭 — 결손 0
- `participants.groups` ↔ `groups.id`: 6 사건 × 평균 2.5 그룹 = 약 15 매칭 — 결손 0
- `participants.places` ↔ `places.id`: 6 사건 × 평균 2 장소 = 약 13 매칭 — 결손 0
- `related_events` ↔ `events.id`: 6 사건 × 평균 1.7 = 약 10 매칭 — 결손 0
- `era_id` (텍스트만, 검증 비활성): 모두 `~` 비움 — Phase 5b 활성

## 막힌 결정 (체크포인트 1 → 체크포인트 2 처리)

| ID | 결정 | Phase 5a 처리 |
|---|---|---|
| D1 — 혈교 잔당 그룹 미등록 | 영구 누락 + 산문 명시 | 6 사건 모두 `participants.groups`에 혈교 누락. body_sections에 "혈교 잔당의 침투" 등 산문 명시. Phase 6+ legendary group 카테고리 등장 시 자연스럽게 처리. |
| D2 — 임서운·추양진인 npc 미등록 | Phase 5b 종결 후 follow-up TASK | 6 사건 모두 `participants.people`에 미등록 인물 누락. body_sections에 `(npc 미등록)` 텍스트로 명시. `task-phase5-followup-historical-npcs.md`에서 시드 확장. |
| D3 — `era_id` 식별자 | `era-fall-of-empire` 채택 (Phase 5b 정식 외래키) | 6 사건 모두 `era_id=~` 비움. Phase 5b Era 도메인 진입 시 정식 매핑 (250-270년차 사건은 era-fall-of-empire 후보). |

## Phase 5a 회귀 가드

- `cargo build --features embed`: 통과
- `cargo build --features mind-studio,chat,embed --bin npc-mind-studio`: 통과 (state.rs 사이드 픽스 후)
- `cargo test --features embed --lib`: 470 passed (Phase 5a 단위 테스트 16+23+9 = 48 신규 + 회귀 0)
- `cargo test --features embed --test world_chilguk_chunchu_*`: 39 e2e 모두 통과
- world-load:
  - `events indexed = 6` · `fk errors (활성) = 0`
  - 외래키 활성 시연 (체크포인트 1): npc-99 주입 → 빌드 실패(DB 미수정) → 복구 → 정상

## Phase 5a 종결 후 진입

### 즉시 진입 — Phase 5b TASK 작성

작업 범위:
- **Era 도메인** — 인스턴스 도메인 (id·name·temporal_range·boundaries)
- **Timeline view** — Event × Era (관계 도메인). Atlas와 결이 같은 도메인+뷰 이중성.
- **View trait 일반화** — Atlas + Timeline 두 사례에서 공통 패턴 추출. `View<DomainItem>` trait 정형.
- **Atlas overlay 활성** — 시기별 atlas 분기 (예: `atlas-daejin-empire` Era=founding-era, `atlas-jungwon` Era=current).
- Event.era_id 외래키 활성 → 6 사건 매핑 (era-fall-of-empire 4건, era-decline 1건, era-founding 1건 후보).

### Phase 5b 종결 후 — Follow-up TASK

`docs/tasks/task-phase5-followup-historical-npcs.md`:
- 임서운·추양진인 + 기타 핵심 historical NPC (단운/태무제·풍만리·설무한·자양진인·천리안 등) 시드 확장
- D1 재검토 — 혈교 잔당이 historical group으로 처리될지 결정
- 6 Event 본문의 `(npc 미등록)` 텍스트를 정식 외래키로 승급

## 회귀 가드 결과 요약

| 검증 | 결과 |
|---|---|
| `cargo build` (default) | ✅ |
| `cargo build --features embed` | ✅ |
| `cargo build --features mind-studio,chat,embed --bin npc-mind-studio` | ✅ (state.rs 사이드 픽스 후) |
| `cargo test --features embed --lib` | ✅ 470 passed |
| world-load Phase 1·2·3·4·5a 통합 ingest | ✅ events indexed=6, fk errors=0 |
| 정성 평가 (search 6쿼리 + 4 list 필터 + 양방향 시연) | ✅ 모두 의도대로 매칭 |
| Phase 1·2·3·4 e2e 회귀 (39 tests) | ✅ 모두 통과 |

## 다음 의견

Phase 5a 종결 사양 충족. 디렉터 통과 시 Phase 5b TASK 작성 진입. Era 도메인 + Timeline view + View trait 일반화의 사양 작성을 위해 다음 항목 사전 확인 필요:

1. Era boundary 정형 — `history.md` §0.2 5 시대(건국기/전성기/변곡기/쇠퇴기/붕괴기) 그대로 5 era? 또는 더 세분?
2. View trait 시그니처 — Atlas의 `places_in/settlements_in/adjacent_to` + Timeline의 `events_in_era/events_at_year/causal_chain` 공통 추출 형태.
3. Atlas overlay 활성 — 시기별 atlas 분기를 `atlas.era_id`로? 또는 별도 `atlas_overlay` 관계 테이블?

위 3건은 Phase 5b TASK 작성 시 디렉터와 1차 결정.
