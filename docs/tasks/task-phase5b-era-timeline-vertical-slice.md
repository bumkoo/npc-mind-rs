# Phase 5b: Era + Timeline Vertical Slice (Phase 5 분리 후반부)

> **For Claude Code.** 이 문서는 자급 자족이며 외부 링크 없이도 작업 시작 가능.
> 결정 사항을 임의 변경하지 말 것. 변경이 필요하면 보고서에서 디렉터 승인 요청.
> **선행 조건**: Phase 5a 종결 ✅
> **체크포인트 분리 게이트 강제 적용** — Phase 2·3·4·5a 패턴 그대로. 1회 통합 commit 금지.

---

## 1. 목표

Phase 5의 후반부 — **Era 인스턴스 도메인** + **Timeline 관계 도메인** + Phase 4·5a의
era_id 외래키 활성. Phase 5a에서 모든 사건이 `era_id=~`로 비워진 상태였던 것을
정형 시간 축으로 승급한다.

**Phase 5b의 책임**:
1. **Era 도메인** — 5 시대(`history.md` §0.2 그대로) 정형. id·kind·name·aliases·summary·
   tags·extras·temporal(start/end_year_relative)·key_events·body_sections.
2. **Timeline 도메인** — 첫 두 번째 관계 도메인. Atlas와 결이 같은 도메인+뷰
   이중성. references(Vec<EventId>) + view 메서드 (events_in/events_during/causal_chain).
3. **Event.era_id 외래키 활성** — Phase 5a 6 사건의 era_id를 era 인스턴스로 매핑.
4. **Atlas.era_id 외래키 활성** — Phase 4 atlas-jungwon의 era_id 텍스트를 외래키로 승급.
5. **Atlas overlay** — Q3 (a) 결정에 따라 atlas.era_id 외래키 단일 매핑. 시기별
   atlas 분기는 Phase 6+ follow-up TASK.
6. **검증 게이트**:
   - 체크포인트 1: 5 era 변환 + Phase 5a 6 Event era_id 활성 + atlas-jungwon era_id 활성
   - 체크포인트 2: 1 Timeline + view 메서드 4종 e2e + MCP 도구 6개 정성 평가

**Phase 5b의 책임 외**:
- View trait 일반화 — Q2 결정에 따라 보류. Atlas + Timeline 각자 view 메서드 자체 구현.
- 시기별 atlas 분기 (atlas-daejin-empire 등) — `task-phase5-followup-era-atlases.md`
- Historical NPC 시드 확장 — `task-phase5-followup-historical-npcs.md`
- Era 외래키 매트릭스의 다른 도메인 확장 (예: Person 활동 시기·Group 존속 시기) — Phase 6+

## 2. 사용자 결정 3건 반영

| ID | 질문 | 결정 |
|---|---|---|
| Q1 | Era 개수·분할 | **5 era** — `history.md` §0.2 그대로 (founding/prosperity/turning/decline/fall) |
| Q2 | View trait 일반화 | **보류** — Atlas + Timeline 각자 view 메서드 자체 구현. 두 사례로는 정형 추출 시기상조 (Phase 6+ 세 번째 관계 도메인 등장 시 재검토) |
| Q3 | Atlas overlay | **(a) atlas.era_id 외래키** — 시기별 atlas 분기는 별 atlas 인스턴스 (Phase 6+ follow-up) |

## 3. 제약

### 3.1 장르 중립 vs 의존

| 위치 | 책임 |
|---|---|
| `src/domain/world/era.rs` | **장르 영원히 모름** — id·kind(String)·name·aliases·summary·tags·extras·temporal·key_events(Vec<EventId>)·body_sections |
| `src/domain/world/timeline.rs` | **장르 영원히 모름** — id·kind·name·aliases·summary·tags·extras·references(Vec<EventId>)·body_sections + view 메서드 |
| `src/worldbuilding/markdown/{era,timeline}.rs` | 장르 중립 frontmatter+섹션 파서 |
| `src/adapter/sqlite_world.rs` (확장) | `eras` 테이블 + FTS5 (체크포인트 1, `migrate_v6`) + `timelines` 테이블 + `timeline_event_refs` 양방향 인덱스 (체크포인트 2, `migrate_v7`) |
| `genres/wuxia/forms/{era,timeline}.toml` | Phase N 빈 슬롯 + kind 옵션 (founding/prosperity/turning/decline/fall) |
| `genres/wuxia/markdown_template/{era,timeline}.md` | 무협 era/timeline 양식 |
| `projects/chilguk-chunchu/world/{era,timeline}/*.md` | 칠국춘추 era·timeline 인스턴스 |

**`src/`에 wuxia 단어 X.** "붕괴기"·"칠국춘추" 등은 모두 `genres/wuxia/`·`projects/`에만.

### 3.2 외래키 매트릭스 확장 (Phase 5b 활성)

| 검증 | Phase 5a까지 | Phase 5b |
|---|---|---|
| `Era.key_events` ↔ `events.id` | — | **에러** (활성, 자체 도메인 외래키 X — Era→Event) |
| `Event.era_id` ↔ `eras.id` | 텍스트만 (Phase 5a) | **에러** (활성) |
| `Atlas.era_id` ↔ `eras.id` | 텍스트만 (Phase 4) | **에러** (활성) — Atlas 도메인 모델 변경 없이 `extras["era_id"]` 헬퍼로 추출 후 검증 |
| `Timeline.references` ↔ `events.id` | — | **에러** (체크포인트 2) |
| `Timeline.references` 중복 금지 | — | **에러** (composite PK 보호) |

partial commit 방지 — 검증 → upsert 순서. Phase 1·2·3·4·5a 패턴 그대로.

### 3.3 Era boundary 정책 — `start_year_relative` inclusive · `end_year_relative` exclusive

```
era-founding         start=-270, end=-220   → -270 ≤ year < -220
era-prosperity       start=-220, end=-150   → -220 ≤ year < -150
era-turning          start=-150, end=-70    → -150 ≤ year < -70
era-decline          start=-70,  end=-30    → -70  ≤ year < -30
era-fall-of-empire   start=-30,  end=0      → -30  ≤ year < 0  (현재 270년차 = 0)
```

**boundary 케이스 처리**:
- `event-bloody-cult-rebellion-2nd` (year_relative=-30) → `era-fall-of-empire` (start inclusive)
  - history.md §0.2의 "쇠퇴기 200~240" 표기는 inclusive-exclusive로 해석.
  - 디렉터 권장: 240년차(=−30)는 붕괴기 시작 트리거.
- `event-empire-founding` (year_relative=-270) → `era-founding` (start inclusive)
- 270년차(현재, year_relative=0)은 어느 era에도 속하지 않음 (모든 era end exclusive).
  → 현재 시점 사건은 별도 era 추가 시까지 era_id 비울 것 (Phase 5b 6 Event 모두 -7 이하라 영향 없음).

이 정책은 view 메서드 `events_during(era_id)`에 인코딩 — `e.year_relative >= era.start AND e.year_relative < era.end`.

### 3.4 Atlas 도메인 모델 — extras["era_id"] 그대로 유지

Atlas의 era_id를 top-level 필드로 승격하는 breaking change는 Phase 6+로 미룬다. Phase 5b엔
**최소 변경 원칙**:
- Atlas 도메인 모델: 그대로 (`Atlas::era_id() -> Option<&str>` 헬퍼만).
- world-load CLI: `atlas.era_id()` 호출 → era_id_set에 없으면 hard-fail.
- atlas-jungwon.md: `extras.era_id`를 `era-fall-of-empire`로 변경 (텍스트 → 외래키 승급).

Event는 이미 top-level `era_id` 필드라 양식 변경 없음.

### 3.5 SoT (Source of Truth)

기존 흐름. 마크다운 = SoT, SQLite는 빌드 산출물(.gitignore).

### 3.6 검색 범위

FTS5 trigram + LIKE fallback (Phase 1·2·3·4·5a 패턴 그대로).

### 3.7 체크포인트 분리 게이트 — 강제 적용

1. **체크포인트 1**: 5 Era 변환 + Phase 5a 6 Event era_id 외래키 활성 + atlas-jungwon era_id 외래키 활성 → commit pause → `phase5b-checkpoint1-report.md` → 디렉터 리뷰
2. **체크포인트 2**: 1 Timeline 변환 + view 메서드 4종 e2e + MCP 도구 6개 정성 평가 → commit pause → `phase5b-checkpoint2-report.md` → Phase 5b 종결

**1회 통합 commit 금지.**

## 4. Done Criteria

### 체크포인트 1
- [ ] `src/domain/world/era.rs` — Era 애그리거트 + EraId + EraTemporal + EraFilter + 단위 테스트
- [ ] `src/worldbuilding/markdown/era.rs` — Era 마크다운 파서 + 단위 테스트
- [ ] `genres/wuxia/markdown_template/era.md` 템플릿
- [ ] `genres/wuxia/forms/era.toml` 자리 (Phase N) + 5 kind 옵션
- [ ] `SqliteWorldStore::migrate_v6` — `eras` + `eras_fts` (Era는 단순 인스턴스 도메인 — Atlas의 place_atlas_refs 같은 양방향 인덱스 불필요)
- [ ] `WorldRepository`: `list_eras`/`get_era`/`search_eras`/`upsert_era`/`count_eras`
- [ ] `bin/world-load` 확장 — `world/era/*.md` 스캔 + Era.key_events 외래키 + Event.era_id 외래키 + Atlas.era_id 외래키 (셋 모두 활성, 결손 시 hard-fail)
- [ ] `bin/mind-studio` REST + MCP 도구 3개: `list_eras` / `get_era` / `search_eras`
- [ ] `tests/world_load_fk_negative_era.rs` — era 결손 e2e (N1 패턴 미러)
- [ ] **5 Era 변환** (founding/prosperity/turning/decline/fall) — `projects/chilguk-chunchu/world/era/*.md`
- [ ] **6 Event 업데이트** — era_id 활성 (1 founding + 5 fall-of-empire, boundary 정책 §3.3 적용)
- [ ] **atlas-jungwon 업데이트** — `extras.era_id = era-fall-of-empire` 활성
- [ ] world-load 통과 — `eras indexed = 5`, `events indexed = 6`, `atlases indexed = 1`, fk errors = 0

### 체크포인트 2
- [ ] `src/domain/world/timeline.rs` — Timeline 애그리거트 + TimelineId + TimelineFilter + view 메서드 (events_in / events_during / causal_chain) + 단위 테스트
- [ ] `src/worldbuilding/markdown/timeline.rs` — Timeline 마크다운 파서 + 단위 테스트
- [ ] `genres/wuxia/markdown_template/timeline.md` 템플릿
- [ ] `genres/wuxia/forms/timeline.toml` 자리
- [ ] `SqliteWorldStore::migrate_v7` — `timelines` + `timelines_fts` + `timeline_event_refs` 양방향 인덱스 (composite PK)
- [ ] `WorldRepository`: `list_timelines`/`get_timeline`/`search_timelines`/`upsert_timeline`/`count_timelines`
- [ ] `bin/world-load` 확장 — `world/timeline/*.md` 스캔 + Timeline.references 외래키 + 중복 금지
- [ ] `bin/mind-studio` REST + MCP 도구 3개: `list_timelines` / `get_timeline` / `search_timelines`
- [ ] `tests/world_load_fk_negative_timeline.rs` — timeline 결손 e2e
- [ ] **1 Timeline 변환** — `timeline-jungwon-history`
- [ ] view 메서드 4종 e2e — `eras_in` / `events_in` / `events_during(era-fall-of-empire)` / `causal_chain`
- [ ] MCP 도구 6개 정성 평가
- [ ] cargo build + cargo test --features embed --lib 통과

## 5. 단계별 작업

### Step 1 — Era 도메인 + 마크다운 파서 (체크포인트 1)

```
src/domain/world/
├── era.rs                # Era + EraId + EraTemporal + EraFilter (Phase 5a stub 채움)
└── ...

src/worldbuilding/
├── markdown/
│   ├── era.rs            # Era .md → 도메인 (신규)
│   └── ...
├── repository.rs         # WorldRepository — list_eras/get_era/search_eras/upsert_era/count_eras 추가
```

**`Era` 애그리거트**:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EraId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct EraTemporal {
    /// 270년차 기준 절대 연도 (inclusive). 예: era-founding = -270.
    pub start_year_relative: Option<i32>,
    /// 270년차 기준 절대 연도 (exclusive). 예: era-founding = -220 → era-prosperity 시작.
    pub end_year_relative: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Era {
    pub id: EraId,
    pub kind: String,                          // "founding"|"prosperity"|"turning"|"decline"|"fall"
    pub name: String,
    pub aliases: Vec<String>,
    pub summary: String,
    pub tags: Vec<String>,
    pub extras: Map<String, Value>,
    pub temporal: EraTemporal,
    pub key_events: Vec<EventId>,              // Phase 5a Event 외래키 활성 (Era → Event 단방향)
    pub body_sections: BTreeMap<String, String>,
    pub source_path: Option<String>,
}

impl Era {
    /// 본 era의 시간 범위 안에 year_relative가 속하는지 (start inclusive, end exclusive).
    pub fn contains_year(&self, year_relative: i32) -> bool { ... }

    /// 본 era의 길이 (연단위). start/end 모두 있어야 Some.
    pub fn duration_years(&self) -> Option<u32> { ... }
}
```

**`EraFilter`**:

```rust
pub struct EraFilter {
    pub kind: Option<String>,
    /// 본 era가 포함하는 year_relative (start_year_relative <= ? AND end_year_relative > ?).
    pub contains_year: Option<i32>,
    pub genre_tag: Option<String>,
}
```

산출물 검증: `cargo build` 통과. 단위 테스트 — Era 인스턴스 생성, contains_year boundary, EraTemporal 직렬화, key_events 빈 배열 default.

### Step 2 — SqliteWorldStore migrate_v6 + world-load 확장 (체크포인트 1)

#### SQLite 스키마 (체크포인트 1)

```sql
CREATE TABLE eras (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    aliases_json TEXT NOT NULL DEFAULT '[]',
    summary TEXT NOT NULL DEFAULT '',
    tags_json TEXT NOT NULL DEFAULT '[]',
    extras_json TEXT NOT NULL DEFAULT '{}',
    temporal_json TEXT NOT NULL DEFAULT '{}',
    start_year_relative INTEGER,                -- 캐시 컬럼 (inclusive, 정렬·필터용)
    end_year_relative INTEGER,                  -- 캐시 컬럼 (exclusive)
    key_events_json TEXT NOT NULL DEFAULT '[]',
    body_sections_json TEXT NOT NULL DEFAULT '{}',
    source_path TEXT,
    updated_at INTEGER NOT NULL
);
CREATE INDEX idx_eras_kind ON eras(kind);
CREATE INDEX idx_eras_start_year ON eras(start_year_relative);
CREATE INDEX idx_eras_end_year ON eras(end_year_relative);
CREATE INDEX idx_eras_project ON eras(project_id);
CREATE VIRTUAL TABLE eras_fts USING fts5(
    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
);
```

`schema_meta.version = 6` 마이그레이션. v5 DB는 자동 ALTER + eras·eras_fts 추가.

**Era는 인스턴스 도메인이라 Atlas의 place_atlas_refs 같은 양방향 인덱스 불필요.**
key_events는 events.id 외래키지만 역방향 lookup("이 사건이 어느 era의 key_events에
포함됐나")이 흔하지 않다 — 필요 시 Phase 6+에서 추가.

#### world-load CLI 확장 (체크포인트 1)

동작:
1. Phase 1·2·3·4·5a 동작 — group/person/place/atlas/event 로드
2. Phase 5b 동작 — `world/era/*.md` 로드 → eras 테이블 upsert
3. **외래키 검증 활성**:
   - `Era.key_events` 각 ID → events 테이블 존재 — **에러**
   - `Event.era_id` (있으면) → eras 테이블 존재 — **에러** (Phase 5a 텍스트 → Phase 5b 활성)
   - `Atlas.era_id` (있으면) → eras 테이블 존재 — **에러** (Phase 4 텍스트 → Phase 5b 활성)
4. partial commit 방지 (Phase 1·2·3·4·5a 패턴 그대로)

산출물 검증: `tests/world_load_fk_negative_era.rs` — N1 패턴 미러 (era-99 주입 → 실패 → 복구 → 통과).

### Step 3 — 5 Era 변환 + Phase 5a Event/atlas-jungwon era_id 활성 (체크포인트 1)

5 Era 매핑 (history.md §0.2 정확):

| id | kind | start | end | aliases | key_events |
|---|---|---|---|---|---|
| era-founding | founding | -270 | -220 | [건국기, 원년대] | [event-empire-founding] |
| era-prosperity | prosperity | -220 | -150 | [전성기, 태평성세] | [] (Phase 5a 시드 없음) |
| era-turning | turning | -150 | -70 | [변곡기, 균열기] | [] (Phase 5a 시드 없음) |
| era-decline | decline | -70 | -30 | [쇠퇴기, 태무제 시기] | [] (Phase 5a 시드 없음) |
| era-fall-of-empire | fall | -30 | 0 | [붕괴기, 6국 분열기] | [event-bloody-cult-rebellion-2nd, event-blood-disappearance, event-bloody-night, event-hwasan-fall, event-six-states-independence] |

6 Event era_id 매핑:
- `event-empire-founding` (-270) → `era-founding`
- `event-bloody-cult-rebellion-2nd` (-30) → `era-fall-of-empire` (boundary 정책 §3.3 — 240년차는 붕괴기 시작 트리거)
- `event-blood-disappearance` (-12) → `era-fall-of-empire`
- `event-bloody-night` (-10) → `era-fall-of-empire`
- `event-hwasan-fall` (-10) → `era-fall-of-empire`
- `event-six-states-independence` (-7) → `era-fall-of-empire`

atlas-jungwon era_id:
- `extras.era_id` = `"era-fall-of-empire"` (현재 270년차의 정치 지도)

작업:
1. `cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload`
2. SQLite eras 5행 + Event 6 row의 era_id + Atlas 1 row의 era_id 모두 외래키 통과 검증
3. **외래키 활성 시연** — era-99 같은 미존재 ID를 Event.era_id에 주입했다가 빌드 실패 확인 → 복구 → 빌드 성공

**체크포인트 1 보고서** (`docs/tasks/phase5b-checkpoint1-report.md`):
- diff stat
- 5 Era 일람 (id·kind·start/end·key_events 카운트)
- 6 Event era_id 매핑 결과 + atlas-jungwon era_id
- world-load 결과 (`eras indexed = 5`, fk errors = 0)
- 외래키 활성 시연 (의도적 결손 + 복구)
- 변환 결정:
  - 5 era kind 결정
  - aliases 2-3개씩
  - boundary 정책 §3.3 적용 결과 (특히 bloody-cult-rebellion-2nd)
  - key_events 정렬 (시간순)
- Step 4 (체크포인트 2) 진행 가능 여부

→ commit pause → 디렉터 리뷰 → 통과 후 체크포인트 2 진입.

### Step 4 — Timeline 도메인 + view 메서드 + MCP 도구 (체크포인트 2)

```
src/domain/world/
├── timeline.rs          # Timeline + TimelineId + TimelineFilter + view 메서드 (신규)

src/worldbuilding/
├── markdown/
│   └── timeline.rs      # Timeline .md → 도메인 (신규)
└── repository.rs        # list_timelines/get_timeline/search_timelines/upsert_timeline/count_timelines 추가
```

**`Timeline` 애그리거트** (Atlas와 결이 같은 도메인+뷰 이중성, Q2 결정에 따라 자체 view 메서드):

```rust
pub struct TimelineId(pub String);

pub struct Timeline {
    pub id: TimelineId,
    pub kind: String,                          // 장르가 채움 (wuxia: "history" | "biographical" 등)
    pub name: String,
    pub aliases: Vec<String>,
    pub summary: String,
    pub tags: Vec<String>,
    pub extras: Map<String, Value>,
    pub references: Vec<EventId>,              // 본 timeline에 포함된 사건들 (작성 순서 = 시간 순)
    pub body_sections: BTreeMap<String, String>,
    pub source_path: Option<String>,
}

impl Timeline {
    /// 본 timeline의 references에 등장하는 모든 era 조회 (event.era_id를 모아 era_id 셋 반환).
    /// 합성: WorldRepository를 통해 references → events → era_id 조회.
    pub fn eras_in<R: WorldRepository + ?Sized>(&self, repo: &R) -> Result<Vec<EraId>, WorldError>;

    /// 본 timeline의 references를 Event 객체로 합성 (작성 순서 보존).
    pub fn events_in<R: WorldRepository + ?Sized>(&self, repo: &R) -> Result<Vec<Event>, WorldError>;

    /// 특정 era에 속하는 본 timeline의 사건들 (year_relative inclusive-exclusive 정책).
    pub fn events_during<R: WorldRepository + ?Sized>(
        &self,
        era_id: &EraId,
        repo: &R,
    ) -> Result<Vec<Event>, WorldError>;

    /// 특정 사건의 인과 사슬 — references 안에서 related_events를 따라 BFS 합성.
    /// timeline 경계 안에 머무르며 (timeline-국한 transitive closure), 결과는 BFS 순서.
    pub fn causal_chain<R: WorldRepository + ?Sized>(
        &self,
        seed: &EventId,
        repo: &R,
    ) -> Result<Vec<Event>, WorldError>;
}
```

**`TimelineFilter`**:

```rust
pub struct TimelineFilter {
    pub kind: Option<String>,
    pub references_event: Option<EventId>,     // 특정 사건을 포함하는 timeline 검색
    pub genre_tag: Option<String>,
}
```

#### SQLite 스키마 (체크포인트 2)

```sql
CREATE TABLE timelines (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    aliases_json TEXT NOT NULL DEFAULT '[]',
    summary TEXT NOT NULL DEFAULT '',
    tags_json TEXT NOT NULL DEFAULT '[]',
    extras_json TEXT NOT NULL DEFAULT '{}',
    references_json TEXT NOT NULL DEFAULT '[]',  -- 단일 권위 (Atlas 패턴)
    body_sections_json TEXT NOT NULL DEFAULT '{}',
    source_path TEXT,
    updated_at INTEGER NOT NULL
);
CREATE INDEX idx_timelines_kind ON timelines(kind);
CREATE INDEX idx_timelines_project ON timelines(project_id);
CREATE VIRTUAL TABLE timelines_fts USING fts5(
    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
);
CREATE TABLE timeline_event_refs (
    timeline_id TEXT NOT NULL,
    event_id TEXT NOT NULL,
    ref_order INTEGER NOT NULL,
    PRIMARY KEY (timeline_id, event_id)         -- composite PK 보호
);
CREATE INDEX idx_ter_event ON timeline_event_refs(event_id);
CREATE INDEX idx_ter_timeline ON timeline_event_refs(timeline_id);
```

`schema_meta.version = 7` 마이그레이션.

#### `timeline-jungwon-history` 변환 + view 메서드 e2e

`projects/chilguk-chunchu/world/timeline/timeline-jungwon-history.md`:

```yaml
---
id: timeline-jungwon-history
kind: history
name: 칠국춘추 270년사
aliases: [중원사, 270년 연표]
summary: 원년부터 현재(270년차)까지의 핵심 분기점 6 사건.
tags: [wuxia, timeline, history]
references:
  - event-empire-founding
  - event-bloody-cult-rebellion-2nd
  - event-blood-disappearance
  - event-bloody-night
  - event-hwasan-fall
  - event-six-states-independence
---

## 개요
...
```

view 메서드 e2e:
- `eras_in(repo)` → `[era-founding, era-fall-of-empire]` (2 era)
- `events_in(repo)` → 6 Event (작성 순서 = 시간 순)
- `events_during(era-fall-of-empire, repo)` → 5 Event (boundary 정책 §3.3)
- `causal_chain(event-bloody-night, repo)` → BFS 결과 (related_events 따라 timeline 경계 안 transitive closure)

#### MCP 도구 6개

체크포인트 2까지 누적:
1. `list_eras(filter)` / 2. `get_era(era_id)` / 3. `search_eras(query)` (체크포인트 1)
4. `list_timelines(filter)` / 5. `get_timeline(timeline_id)` / 6. `search_timelines(query)` (체크포인트 2)

Atlas 패턴 그대로.

**체크포인트 2 보고서** (`docs/tasks/phase5b-checkpoint2-report.md`):
- diff stat (체크포인트 1 → 체크포인트 2)
- 1 Timeline 변환 결과
- view 메서드 4종 e2e 출력
- MCP 도구 6개 정성 평가
- Atlas overlay 시연 (atlas-jungwon.era_id = era-fall-of-empire)
- Phase 5b 종결 후 follow-up TASK 작성 진입 의견

→ commit pause → 디렉터 리뷰 → 통과 시 Phase 5b 종결 → follow-up TASK 작성.

## 6. 결정 사항 (변경 시 디렉터 승인)

### 6.1 Frontmatter 양식 — era 예시 (era-fall-of-empire)

```yaml
---
id: era-fall-of-empire
kind: fall
name: 붕괴기
aliases:
  - 6국 분열기
  - 240-270년차
summary: |
  240~270년차의 30년. 통일제국 대진의 영토 와해와 칠국 형성이 일어난 시기.
  이 시기에 일어난 분기점 5 사건이 게임 시작 시점(270년차)의 정치 지도를 만듦.
tags: [wuxia, era, historical, fall-of-empire]
temporal:
  start_year_relative: -30
  end_year_relative: 0
  notes: |
    Phase 5b §3.3 boundary 정책: start inclusive · end exclusive.
    270년차(=0)는 본 era 외 — 현재 시점 사건은 별도 era 추가 시까지 era_id 비움.
key_events:
  - event-bloody-cult-rebellion-2nd
  - event-blood-disappearance
  - event-bloody-night
  - event-hwasan-fall
  - event-six-states-independence
extras:
  game_role: 게임 시작 시점의 정치 지도가 본 시대에서 형성됨
  player_relevance: 5
---

## 개요
산문 — 본 시대의 핵심 흐름.

## 핵심 트리거
산문 — 직전 시대(쇠퇴기)에서 본 시대로 넘어가는 트리거.

## 결과
산문 — 본 시대가 만든 칠국춘추의 정치 지도.

## 게임에서의 역할
- 메인 서사 분기점들의 시간 컨테이너
- player의 출생·트라우마·현재 시점 모두 본 시대 안
```

### 6.2 `genres/wuxia/forms/era.toml` (Phase N 빈 슬롯)

```toml
extends = "era"

[[fields.kind.options]]
value = "founding"; label = "건국기"
[[fields.kind.options]]
value = "prosperity"; label = "전성기"
[[fields.kind.options]]
value = "turning"; label = "변곡기"
[[fields.kind.options]]
value = "decline"; label = "쇠퇴기"
[[fields.kind.options]]
value = "fall"; label = "붕괴기"
```

### 6.3 환경변수

`NPC_MIND_WORLD_DB` 그대로.

### 6.4 라이브러리

기존 — Phase 0 D2·D3 의존성 회피 원칙 계승.

### 6.5 외래키 매트릭스 (Phase 5b 활성)

| 검증 | Phase 1 | 2 | 3 | 4 | 5a | 5b |
|---|---|---|---|---|---|---|
| (이전 Phase 외래키 — 그대로) | ... | ... | ... | ... | ... | 그대로 |
| **`Era.key_events`** | — | — | — | — | — | **에러** (활성) |
| **`Event.era_id`** | — | — | — | — | 텍스트 | **에러** (활성) |
| **`Atlas.era_id`** | — | — | — | 텍스트 | (그대로) | **에러** (활성) |
| **`Timeline.references`** | — | — | — | — | — | **에러** (체크포인트 2) |
| **`Timeline.references` 중복 금지** | — | — | — | — | — | **에러** (체크포인트 2) |

## 7. Out of Scope (Phase 5b)

- View trait 일반화 — Q2 보류
- 시기별 atlas 분기 (atlas-daejin-empire 등) — Phase 6+ follow-up
- Era 외래키의 다른 도메인 확장 (Person 활동 시기·Group 존속 시기) — Phase 6+
- Timeline 다중 (timeline-hwasan-fall-only·timeline-blood-cult-arc 등) — 체크포인트 2엔 1건만, 다중은 follow-up
- Historical NPC 시드 확장 — `task-phase5-followup-historical-npcs.md`
- Timeline 자동 정렬 (year_relative 오름차순 자동) — Phase 5b엔 작성 순서 보존만

## 8. 코드 위치 가이드

작업 시작 5분에 읽을 곳:

| 위치 | 무엇을 볼지 |
|---|---|
| `src/domain/world/event.rs` (Phase 5a) | EventId·EventTemporal·Event·EventFilter — Era·Timeline의 도메인 패턴 미러 |
| `src/domain/world/atlas.rs` (Phase 4) | 관계 도메인 + 도메인+뷰 이중성 — Timeline 미러 |
| `src/worldbuilding/markdown/{event,atlas}.rs` | 마크다운 파서 패턴 |
| `src/adapter/sqlite_world.rs` migrate_v5 (Phase 5a) | 인스턴스 도메인 마이그레이션 패턴 — Era 미러 |
| `src/adapter/sqlite_world.rs` migrate_v4 (Phase 4) | 관계 도메인 양방향 인덱스 패턴 — Timeline 미러 |
| `src/bin/world_load.rs` Phase 5a section | 외래키 검증 흐름 + N1 자동화 패턴 |
| `tests/world_load_fk_negative_event.rs` | FK negative e2e 패턴 — era·timeline 미러 |

## 9. 시작 체크리스트

1. `CLAUDE.md` + Phase 5a 보고서 빠르게 훑기
2. **`wuxia-core/docs/world/history.md` §0.1·§0.2** 통독 — 5 era boundary 정확 매핑 입력
3. Phase 5a 6 Event (`projects/chilguk-chunchu/world/event/*.md`) 통독 — era_id 매핑 입력
4. Phase 4 atlas-jungwon (`projects/chilguk-chunchu/world/atlas/atlas-jungwon.md`) 확인 — era_id 매핑 입력
5. Era 도메인 + EraTemporal + EraFilter + 마크다운 파서 + 단위 테스트 (Step 1·2)
6. SqliteWorldStore migrate_v6 + eras + 라운드트립 테스트
7. world-load 확장 — Era.key_events + Event.era_id + Atlas.era_id 외래키 활성
8. **5 Era 변환 + 6 Event era_id 활성 + atlas-jungwon era_id 활성** → ★체크포인트 1★ 보고 → **commit pause**

## 10. 리뷰 채널

체크포인트 1·2 보고서를 디렉터(사용자)가 Cowork 세션에 복붙. 형식:
- **Done** · **Diff** · **데모 명령** · **결정** · **막힌 것** · **다음 의견**
- 변환 시 모든 추론·결정 (특히 boundary 정책 적용 결과·5 era kind 결정·view 메서드 e2e)을 본문에 상세히 명시

보고서 파일명:
- 체크포인트 1: `docs/tasks/phase5b-checkpoint1-report.md`
- 체크포인트 2: `docs/tasks/phase5b-checkpoint2-report.md`
