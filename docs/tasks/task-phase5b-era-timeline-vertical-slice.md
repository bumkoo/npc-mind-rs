# Phase 5b: Era + Timeline view + Atlas overlay (관계 도메인 두 번째 + Phase 5a era_id 활성)

> **For Claude Code.** 이 문서는 자급 자족이며 외부 링크 없이도 작업 시작 가능.
> 결정 사항을 임의 변경하지 말 것. 변경이 필요하면 보고서에서 디렉터 승인 요청.
> **선행 조건**: Phase 0·1·2(+2.1·2.2)·3·4·5a 모두 종결.
> **체크포인트 분리 게이트 강제 적용** — Phase 1 미준수 후속, Phase 2·3·4·5a에서 정상.

---

## 1. 목표

Phase 5b는 **세 결의 통합 검증**:

1. **Era 인스턴스 도메인** — 5 시대(history.md §0.2: 건국기·전성기·변곡기·쇠퇴기·붕괴기). Phase 5a `Event.era_id` 텍스트 → 정식 외래키 활성.
2. **Timeline 관계 도메인** — Atlas와 같은 결의 도메인+뷰 이중성. Era × Event 합성 view.
3. **Atlas overlay 활성** — `atlas.era_id` 외래키 (Q3·a 결정). atlas-jungwon이 era-fall-of-empire 시점임을 명시.

**핵심 결정 — View trait 일반화 보류 (Q2 결정)**: Atlas와 Timeline 각자 view 메서드를 자체 구현. trait 추출은 두 사례를 충분히 사용한 뒤 Phase 5+ 또는 별도 작업.

**검증 게이트**:
- 체크포인트 1: 5 Era 변환 + Phase 5a 6 Event era_id 외래키 활성
- 체크포인트 2: 1 Timeline + Atlas overlay + MCP 도구 + 정성 평가

## 2. 연관 컨텍스트

- `CLAUDE.md` 프로젝트 루트
- `docs/tasks/00-roadmap.md` — 전체 흐름·결정 로그
- `docs/tasks/task-phase5a-event-vertical-slice.md` + `phase5a-checkpoint{1,2}-report.md` — Phase 5a 결과
- `docs/tasks/task-phase4-atlas-vertical-slice.md` — Atlas의 도메인+뷰 이중성 패턴 (Timeline이 미러링)
- 메모리(Cowork 세션 보유): 9 인스턴스 + 1 관계 도메인 → **Phase 5b로 1 관계 도메인 추가 (Atlas, Timeline)**, Phase 5b Q1·Q2·Q3 결정
- 입력 자료:
  - `wuxia-core/docs/world/history.md` — 270년 연표 (5 시대 + 28 사건). **Phase 5b 핵심 입력**
  - Phase 5a 산출 — `projects/chilguk-chunchu/world/event/*.md` × 6 (era_id 외래키 활성 대상)
  - Phase 4 산출 — `atlas-jungwon` (era_id 외래키 활성 대상)

## 3. 제약

### 3.1 장르 중립 vs 의존

| 위치 | 책임 |
|---|---|
| `src/domain/world/era.rs` | **장르 영원히 모름** — id·name·aliases·kind(String)·summary·tags·extras·temporal(year_relative_start/end·label·duration)·key_events(Vec<EventId>)·body_sections |
| `src/domain/world/timeline.rs` | **장르 영원히 모름** — id·name·aliases·kind·summary·extras·extent(year_relative_min/max·projection)·references(Vec<EraId>)·body_sections + view 메서드 (`eras_in`·`events_in`·`events_during`·`causal_chain`) |
| `src/worldbuilding/markdown/{era,timeline}.rs` | 장르 중립 frontmatter+섹션 파서 |
| `src/adapter/sqlite_world.rs` (확장) | `eras` + `timelines` + FTS5 + `event_era_refs` (양방향) + `migrate_v6` + atlases.era_id 컬럼 추가 |
| `genres/wuxia/forms/{era,timeline}.toml` | Phase N 빈 슬롯 |
| `genres/wuxia/markdown_template/{era,timeline}.md` | 무협 양식 |
| `projects/chilguk-chunchu/world/{era,timeline}/*.md` | 칠국춘추 인스턴스 |

**`src/`에 wuxia 단어 X.** 건국기·붕괴기·칠국춘추 같은 어휘는 `genres/wuxia/`·`projects/`에만.

### 3.2 5 Era — `history.md` §0.2 정확 매핑

5 시대 boundary는 history.md §0.2를 정확히 따름. boundary 케이스(예: bloody-cult-rebellion-2nd가 -30년이라 쇠퇴기 끝 vs 붕괴기 시작 어느 era에 속할지)는 **체크포인트 1 보고서에서 디렉터 결정 사항**으로 명시.

추정 매핑 (history.md §0.2 확인 후 정정):

| Era ID | 시대명 | year_relative 범위 | Phase 5a Event 매핑 (잠정) |
|---|---|---|---|
| `era-empire-founding` | 건국기 | -270 ~ -200 | event-empire-founding |
| `era-prosperity` | 전성기 | -200 ~ -140 | (미시드) |
| `era-turning` | 변곡기 | -140 ~ -100 | (미시드) |
| `era-decline` | 쇠퇴기 | -100 ~ -30 | (boundary 케이스) |
| `era-fall-of-empire` | 붕괴기 | -30 ~ 0 | bloody-cult-2nd · blood-disappearance · bloody-night · hwasan-fall · six-states-independence (5건) |

**boundary 케이스**: `event-bloody-cult-rebellion-2nd`(-30)이 쇠퇴기 끝(-30)인지 붕괴기 시작(-30)인지. history.md §0.2 정확 boundary 표기 따름. 5b는 inclusive/exclusive boundary 정책도 결정.

### 3.3 Phase 5a Event era_id 외래키 활성

| 검증 (Phase 5b 활성) | 정책 |
|---|---|
| `Event.era_id` → `eras.id` 존재 | **에러** (활성, Phase 5a 텍스트 → 승급) |
| `Atlas.era_id` → `eras.id` 존재 | **에러** (활성, Phase 4 텍스트 → 승급) |
| `Timeline.references` 각 EraId → `eras.id` 존재 | **에러** (5b 신규) |
| `Era.key_events` 각 EventId → `events.id` 존재 | **에러** (자체 도메인) |

Phase 5a era_id가 비워진 6 Event 모두 Phase 5b 진입 시 매핑 필요. 매핑 안 한 era_id=null은 허용 (선택적 외래키).

### 3.4 Atlas overlay 활성 — Q3·a 결정

`Atlas.era_id: Option<EraId>` 외래키 활성. atlas-jungwon = `era-fall-of-empire` (현재 시점). 시기별 atlas 분기(atlas-daejin-empire = era-empire-founding 등)는 **Phase 5b 미포함, Phase 5b 종결 후 follow-up 또는 Phase 6+**.

### 3.5 View trait 일반화 보류 — Q2 결정

Atlas의 view 메서드(`places_in`·`settlements_in`·`adjacent_to`)와 Timeline의 view 메서드(`eras_in`·`events_in`·`events_during`·`causal_chain`)는 각자 자체 구현. **trait 추출은 Phase 5b 종결 후 두 사례 충분히 사용한 뒤 결정.** Phase 5b는 trait 도입 X.

### 3.6 Timeline 관계 도메인 — Atlas 패턴 미러

Timeline은 Atlas와 같은 결의 도메인+뷰 이중성:

```rust
pub struct TimelineExtent {
    pub year_relative_min: i32,
    pub year_relative_max: i32,
    pub projection: String,    // "linear" (Phase 5b 단일 옵션). Phase N+ "tree"·"branching" 가능
}

pub struct Timeline {
    pub id: TimelineId,
    pub kind: String,            // "main-history" | "character-arc" | "war-chronicle"
    pub name: String,
    pub aliases: Vec<String>,
    pub summary: String,
    pub tags: Vec<String>,
    pub extras: serde_json::Map<String, Value>,
    pub extent: TimelineExtent,
    pub references: Vec<EraId>,  // 핵심 — timeline에 등장하는 era들
    pub body_sections: BTreeMap<String, String>,
    pub source_path: Option<String>,
}

impl Timeline {
    pub fn eras_in<R: WorldRepository>(&self, repo: &R) -> Result<Vec<Era>, WorldError>;
    pub fn events_in<R: WorldRepository>(&self, repo: &R) -> Result<Vec<Event>, WorldError>;
    pub fn events_during<R: WorldRepository>(&self, era_id: &EraId, repo: &R) -> Result<Vec<Event>, WorldError>;
    pub fn causal_chain<R: WorldRepository>(&self, event_id: &EventId, repo: &R) -> Result<Vec<EventId>, WorldError>;
}
```

검증 게이트: timeline-jungwon-history (270년 칠국 역사) 1건 변환 + view 메서드 e2e.

### 3.7 SoT = 마크다운, 검색 = FTS5 + LIKE fallback

기존 흐름.

### 3.8 체크포인트 분리 게이트

1. **체크포인트 1**: 5 Era 변환 + Phase 5a 6 Event era_id 외래키 활성 + Atlas era_id 외래키 활성 → commit pause → `phase5b-checkpoint1-report.md` → Cowork 리뷰
2. **체크포인트 2**: 1 Timeline 변환 + view 메서드 e2e + MCP 도구 + 정성 평가 → commit pause → `phase5b-checkpoint2-report.md` → Phase 5b 종결

**1회 통합 commit 금지.**

## 4. Done Criteria

- [ ] 디렉토리 골격: `src/domain/world/{era,timeline}.rs` (stub 채움), `src/worldbuilding/markdown/{era,timeline}.rs`
- [ ] `Era` 애그리거트 + `EraId` + `EraTemporal` + 단위 테스트
- [ ] `Timeline` 애그리거트 + `TimelineId` + `TimelineExtent` + view 메서드(`eras_in`·`events_in`·`events_during`·`causal_chain`) + 단위 테스트
- [ ] 마크다운 frontmatter+섹션 파서 (era·timeline 두 양식) + 단위 테스트
- [ ] `genres/wuxia/markdown_template/{era,timeline}.md` 템플릿
- [ ] `genres/wuxia/forms/{era,timeline}.toml` 자리 (Phase N 빈 슬롯)
- [ ] `SqliteWorldStore` 확장 — `eras` + `timelines` + `event_era_refs` (양방향) + `atlases.era_id` 컬럼 + `migrate_v6`
- [ ] `bin/world-load` 확장 — `world/{era,timeline}/*.md` 스캔 + Event/Atlas era_id 외래키 활성
- [ ] `bin/mind-studio` MCP 도구 6개: `list_eras` · `get_era` · `search_eras` · `list_timelines` · `get_timeline` · `search_timelines`
- [ ] **체크포인트 1**: 5 Era 변환 + 6 Event era_id 매핑 통과 + atlas-jungwon.era_id 활성
- [ ] **체크포인트 2**: 1 Timeline 변환 + view 메서드 e2e + MCP 정성 + 외래키 결손 0건
- [ ] `cargo build` + `cargo test --features embed` + 기존 e2e 회귀 통과
- [ ] 정성 검증: `list_eras()` → 5건 / `get_timeline("timeline-jungwon-history").events_during("era-fall-of-empire", repo)` → 5건 (붕괴기 사건들)

## 5. 단계별 작업

### Step 1 — Era 도메인 + 마크다운 파이프라인

#### `Era` 애그리거트

```rust
// src/domain/world/era.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EraId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EraTemporal {
    pub year_relative_start: i32,    // 270년차 기준. era-empire-founding은 -270
    pub year_relative_end: i32,       // era-fall-of-empire는 0 (현재)
    pub year_label: String,            // "270~200년 전" 자유 텍스트
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_years: Option<i32>,   // 캐시 (end - start)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Era {
    pub id: EraId,
    pub kind: String,                  // "founding" | "prosperity" | "turning" | "decline" | "fall" — 장르가 채움
    pub name: String,
    pub aliases: Vec<String>,
    pub summary: String,
    pub tags: Vec<String>,
    pub extras: serde_json::Map<String, Value>,
    pub temporal: EraTemporal,
    pub key_events: Vec<EventId>,      // 자체 도메인 외래키 (events.id)
    pub body_sections: BTreeMap<String, String>,
    pub source_path: Option<String>,
}

pub struct EraFilter {
    pub kind: Option<String>,
    pub year_relative_min: Option<i32>,
    pub year_relative_max: Option<i32>,
    pub genre_tag: Option<String>,
}
```

#### 마크다운 양식 — era-fall-of-empire 예시

```yaml
---
id: era-fall-of-empire
kind: fall
name: 붕괴기
aliases: [말기, 말세, 종말기]
summary: |
  단운(태무제) 즉위 후 30년간의 황실 권위 붕괴와 6국 독립 운동. 칠국춘추 270년차의
  직전 시기. 모든 메인 서사 트리거 사건들이 이 시기에 집중.
tags: [wuxia, era, fall-period, current-era]
temporal:
  year_relative_start: -30
  year_relative_end: 0
  year_label: 30~0년 전 (240~270년차)
  duration_years: 30
key_events:
  - event-bloody-cult-rebellion-2nd
  - event-blood-disappearance
  - event-bloody-night
  - event-hwasan-fall
  - event-six-states-independence
extras:
  notable_npcs: [npc-02, npc-07, npc-01]
  status: 현재 진행 중
  source_section: history.md §0.2 붕괴기
---

## 개요
산문 — 시대 핵심 묘사.

## 시대 트리거
산문 — 이전 era에서 어떻게 전환됐는가.

## 주요 사건 흐름
산문 — key_events의 인과 흐름.

## 핵심 인물
- 조고(npc-02): 시기 권력 장악
- 천순제(npc-07): 즉위
- 명경(npc-01): 정파 정보망

## 게임 시점에서의 의미
산문 — 270년차에 NPC들이 이 시기를 어떻게 기억하는가.
```

권장 H2 섹션: `## 개요` · `## 시대 트리거` · `## 주요 사건 흐름` · `## 핵심 인물` · `## 게임 시점에서의 의미`.

### Step 2 — Timeline 도메인 + 마크다운 파이프라인

#### `Timeline` 애그리거트 (§3.6 참조)

view 메서드 4종 구현:
- `eras_in<R>(&self, repo) -> Vec<Era>` — references 따라 Era 정합 list
- `events_in<R>(&self, repo) -> Vec<Event>` — 모든 era들에 속한 events 합성 (era별 events.era_id 매칭)
- `events_during<R>(&self, era_id, repo) -> Vec<Event>` — 특정 era의 events
- `causal_chain<R>(&self, event_id, repo) -> Vec<EventId>` — related_events traversal (timeline 안의 events만 필터)

#### 마크다운 양식 — timeline-jungwon-history 예시

```yaml
---
id: timeline-jungwon-history
kind: main-history
name: 칠국춘추 270년사
aliases: [중원사, 칠국 역사, 대륙사]
summary: |
  대륙 단일 정치체 = 대진제국에서 7국 분열 = 칠국춘추까지 270년의 흐름.
  5 시대 + 6 핵심 사건 + 인과 사슬을 합성하는 main timeline.
tags: [wuxia, timeline, main-history]
extent:
  year_relative_min: -270
  year_relative_max: 0
  projection: linear                 # Phase 5b 단일 옵션
references:
  - era-empire-founding
  - era-prosperity
  - era-turning
  - era-decline
  - era-fall-of-empire
extras:
  source_section: history.md §0.1·§0.2
  era_id: ~                          # timeline 자체는 era에 안 속함
---

## 개요
대륙 270년 흐름의 종론.

## Era 변천
산문 — 5 era 차례로 어떻게 이어지는가.

## 핵심 인과 사슬
산문 — 6 Event의 인과 표현 (related_events 기반). bloody-cult-2nd → blood-disappearance →
bloody-night ↔ hwasan-fall → six-states-independence.

## 게임 시점에서의 활용
산문 — NPC 대사·서적·기억 시드로 어떻게 활용되는가.
```

### Step 3 — 5 Era 변환 + Phase 5a Event era_id 활성 ★체크포인트 1★

작업:
1. `wuxia-core/docs/world/history.md` §0.2 통독 → 5 era boundary 정확 매핑
2. 5 era .md 작성 (`projects/chilguk-chunchu/world/era/era-{empire-founding,prosperity,turning,decline,fall-of-empire}.md`)
3. Phase 5a 6 Event의 `era_id` 매핑 갱신:
   - `event-empire-founding.era_id = "era-empire-founding"`
   - `event-bloody-cult-rebellion-2nd.era_id` = boundary 결정 (era-decline 끝 vs era-fall-of-empire 시작)
   - `event-blood-disappearance.era_id = "era-fall-of-empire"`
   - `event-bloody-night.era_id = "era-fall-of-empire"`
   - `event-hwasan-fall.era_id = "era-fall-of-empire"`
   - `event-six-states-independence.era_id = "era-fall-of-empire"`
4. atlas-jungwon.era_id = "era-fall-of-empire" 갱신
5. world-load 실행 — fk errors = 0 도달

**체크포인트 1 보고서** (`docs/tasks/phase5b-checkpoint1-report.md`):
- `git diff --stat`
- 5 era .md 핵심 부분 (frontmatter + §개요 1단락)
- 6 Event era_id 매핑 결과
- atlas-jungwon.era_id 활성 결과
- world-load 출력 (fk errors = 0)
- **변환 시 결정한 것**:
  - 5 era boundary 정확 매핑 (history.md §0.2)
  - bloody-cult-rebellion-2nd boundary 케이스 (-30 → era-decline 또는 era-fall-of-empire)
  - kind 결정 (founding·prosperity·turning·decline·fall)
  - aliases (각 era 2-3개)
  - key_events 정렬 순서 (시간순 권장)
- **막힌 결정**: 디렉터 결정 필요 사항 (boundary 정책·kind 명명 등)
- Step 4 진행 가능 여부 의견

→ Cowork 리뷰 → **commit pause 유지** → 통과 신호 받고 다음 단계.

### Step 4 — Timeline 도메인 + atlas overlay 활성

체크포인트 1 통과 후:
1. `Timeline` 도메인 구현 + view 메서드 4종 + 단위 테스트
2. `timeline-jungwon-history.md` 작성 (§Step 2 양식)
3. SqliteWorldStore migrate (timelines 테이블 + atlases.era_id 컬럼)
4. world-load 통합 — atlas-jungwon.era_id 활성 시연
5. view 메서드 자동 e2e:
   - `events_during("era-fall-of-empire", repo)` → 5 (또는 boundary 결정에 따라 6)
   - `causal_chain("event-bloody-night", repo)` → bloody-night의 related_events traversal 결과

### Step 5 — MCP 도구 6개 + 정성 평가 ★체크포인트 2★

```
list_eras(filter) -> Vec<Era>
get_era(era_id) -> Option<Era>
search_eras(query, top_k) -> Vec<Era>

list_timelines(filter) -> Vec<Timeline>
get_timeline(timeline_id) -> Option<Timeline>
search_timelines(query, top_k) -> Vec<Timeline>
```

**체크포인트 2 보고서** (`docs/tasks/phase5b-checkpoint2-report.md`):
- `list_eras()` 결과 (5건 — 시간순)
- `get_era("era-fall-of-empire")` 전체 detail + key_events 5건
- `get_timeline("timeline-jungwon-history")` 전체 detail + references 5 era
- view 메서드 호출 결과 (eras_in·events_in·events_during·causal_chain)
- search 6쿼리 — "건국기"·"붕괴기"·"칠국춘추 270년사"·"붕괴 시대" 등
- 외래키 결손 0건 검증 (Phase 5a Event era_id + Atlas era_id 모두 활성)
- Phase 6+ 진입 가능 여부 의견

→ Cowork 리뷰 → 통과 시 Phase 5b 종결.

## 6. 결정 사항 (변경 시 디렉터 승인)

### 6.1 Era frontmatter 양식 — §Step 1 예시 그대로

권장 H2 섹션: `## 개요` · `## 시대 트리거` · `## 주요 사건 흐름` · `## 핵심 인물` · `## 게임 시점에서의 의미`.

### 6.2 Timeline frontmatter 양식 — §Step 2 예시 그대로

권장 H2 섹션: `## 개요` · `## Era 변천` · `## 핵심 인과 사슬` · `## 게임 시점에서의 활용`.

### 6.3 SQLite 스키마

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
    year_relative_start INTEGER NOT NULL,    -- 캐시 (정렬·범위 필터용)
    year_relative_end INTEGER NOT NULL,
    key_events_json TEXT NOT NULL DEFAULT '[]',
    body_sections_json TEXT NOT NULL DEFAULT '{}',
    source_path TEXT,
    updated_at INTEGER NOT NULL
);
CREATE INDEX idx_eras_kind ON eras(kind);
CREATE INDEX idx_eras_year_range ON eras(year_relative_start, year_relative_end);

CREATE VIRTUAL TABLE eras_fts USING fts5(
    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
);

CREATE TABLE timelines (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    aliases_json TEXT NOT NULL DEFAULT '[]',
    summary TEXT NOT NULL DEFAULT '',
    tags_json TEXT NOT NULL DEFAULT '[]',
    extras_json TEXT NOT NULL DEFAULT '{}',
    extent_json TEXT NOT NULL DEFAULT '{}',
    references_json TEXT NOT NULL DEFAULT '[]',
    body_sections_json TEXT NOT NULL DEFAULT '{}',
    source_path TEXT,
    updated_at INTEGER NOT NULL
);
CREATE INDEX idx_timelines_kind ON timelines(kind);

CREATE VIRTUAL TABLE timelines_fts USING fts5(
    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
);

-- Era ↔ Timeline 양방향 인덱스 (atlas ↔ place 패턴 미러)
CREATE TABLE timeline_era_refs (
    timeline_id TEXT NOT NULL,
    era_id TEXT NOT NULL,
    ref_order INTEGER NOT NULL,
    PRIMARY KEY (timeline_id, era_id)
);
CREATE INDEX idx_ter_era ON timeline_era_refs(era_id);
CREATE INDEX idx_ter_timeline ON timeline_era_refs(timeline_id);

-- atlases.era_id 컬럼 추가 (Phase 4 atlases 테이블 ALTER)
ALTER TABLE atlases ADD COLUMN era_id TEXT;
CREATE INDEX idx_atlases_era ON atlases(era_id);

-- events.era_id 컬럼은 Phase 5a에 이미 있음. 인덱스만 추가/유지 확인.
```

`schema_meta.version = 6` 마이그레이션. Phase 5a v5 DB는 자동 ALTER + eras·timelines·timeline_era_refs 추가 + atlases.era_id 컬럼.

### 6.4 환경변수

`NPC_MIND_WORLD_DB` 그대로.

### 6.5 라이브러리

기존 — Phase 0 D2·D3 의존성 회피 원칙 계승.

### 6.6 외래키 매트릭스 (Phase 5b 활성)

| 검증 | Phase 5a | Phase 5b |
|---|---|---|
| Group / Person / Place / Atlas / Event (이전 Phase) | (Phase 별) | 그대로 |
| **`Event.era_id` → `eras.id`** | 텍스트만 | **에러** (활성) |
| **`Atlas.era_id` → `eras.id`** | 텍스트만 (Phase 4) | **에러** (활성) |
| **`Timeline.references` → `eras.id`** | (5b 신규) | **에러** |
| **`Era.key_events` → `events.id`** | (5b 신규) | **에러** |

### 6.7 Phase 5a 6 Event era_id 매핑 가이드

체크포인트 1 보고서에서 디렉터 검토:

| Event | year_relative | 권장 era_id | 비고 |
|---|---|---|---|
| event-empire-founding | -270 | era-empire-founding | 명확 |
| event-bloody-cult-rebellion-2nd | -30 | era-decline 끝 vs era-fall-of-empire 시작 | **boundary 케이스 — 디렉터 결정** |
| event-blood-disappearance | -12 | era-fall-of-empire | 명확 |
| event-bloody-night | -10 | era-fall-of-empire | 명확 |
| event-hwasan-fall | -10 | era-fall-of-empire | 명확 |
| event-six-states-independence | -7 | era-fall-of-empire | 명확 |

boundary 정책 옵션:
- (a) inclusive end — `era-decline.year_relative_end = -30`이면 -30 사건은 era-decline에 포함
- (b) exclusive end — `era-decline.year_relative_end = -31`이면 -30 사건은 era-fall-of-empire
- (c) bloody-cult-2nd는 사건 본질이 era-fall-of-empire 시작 트리거라 후자에 포함

내 권장: **(c)** — bloody-cult-rebellion-2nd가 붕괴기 시작 트리거라 era-fall-of-empire에 매핑. 단 디렉터 결정.

## 7. Out of Scope (Phase 5b)

- **View trait 일반화** — Q2 결정. Phase 5b 종결 후 두 사례 충분 사용 후 결정
- **시기별 atlas 분기** (atlas-daejin-empire 등) — Phase 5b 종결 후 follow-up 또는 Phase 6+
- **다중 Timeline** (character-arc·war-chronicle 등) — Phase 6+
- **`atlas_overlay` 관계 테이블** (한 atlas가 여러 era에 등장) — Phase 6+ (Q3·a 단순 결정 채택)
- **Era cycle 검증** (era 그래프 cycle) — Phase 5+ (era는 보통 선형이라 단순 결손 검증만)
- **Era hierarchy** (era → sub-era) — Phase 6+
- **gameplay 다리** (Scenario·Scene·Beat·Memory 통합) — Phase 6+
- **historical NPC 시드 확장** (임서운·추양진인 등) — Phase 5b 종결 후 follow-up TASK

## 8. 코드 위치 가이드

작업 시작 5분에 읽을 곳:

| 위치 | 무엇을 볼지 |
|---|---|
| `Cargo.toml` `[features]` | feature 게이팅 |
| `src/domain/world/atlas.rs` (Phase 4) | **Timeline의 도메인+뷰 패턴 그대로 미러링** — view 메서드 구현 참고 |
| `src/domain/world/event.rs` (Phase 5a) | era_id 텍스트 보존 패턴, related_events 자체 외래키 |
| `src/worldbuilding/markdown/{atlas,event}.rs` | 마크다운 파서 패턴 |
| `src/adapter/sqlite_world.rs` (Phase 5a) | migrate_v5·event_participants_refs 양방향 — Phase 5b `migrate_v6` + eras·timelines·timeline_era_refs 미러 |
| `src/bin/world_load.rs` (Phase 5a) | 외래키 활성 흐름 |
| `src/bin/mind-studio/handlers/{world_atlases,world_events}.rs` | MCP·REST 패턴 — eras·timelines 동일 패턴 |

## 9. 시작 체크리스트

1. `CLAUDE.md` + Phase 0~5a 산출 빠르게 훑기
2. **`wuxia-core/docs/world/history.md` §0.1·§0.2** 통독 — 5 era boundary 정확 확인
3. **Phase 5a 6 Event** (`projects/chilguk-chunchu/world/event/*.md`) 통독 — era_id 매핑 입력
4. Phase 4 atlas-jungwon 확인 — era_id 매핑 입력
5. Era 도메인 + 마크다운 파서 + 단위 테스트 (Step 1)
6. SqliteWorldStore migrate_v6 + eras + 라운드트립 테스트
7. world-load 확장 — Event/Atlas era_id 외래키 활성
8. **5 Era 변환 + Phase 5a 6 Event era_id 매핑** → ★체크포인트 1★ 보고 → **commit pause**
9. Timeline 도메인 + view 메서드 + Atlas overlay 활성 → 체크포인트 2

## 10. 리뷰 채널

체크포인트 1·2 보고서를 디렉터(사용자)가 Cowork 세션에 복붙. 형식:
- **Done** · **Diff** · **데모 명령** · **결정** · **막힌 것** · **다음 의견**
- 5 era boundary 정확 매핑 + bloody-cult-rebellion-2nd boundary 결정을 본문에 상세히 명시
- view 메서드 호출 결과 (eras_in·events_in·events_during·causal_chain) 표 형식

보고서 파일명:
- 체크포인트 1: `docs/tasks/phase5b-checkpoint1-report.md`
- 체크포인트 2: `docs/tasks/phase5b-checkpoint2-report.md`
