# Phase 5a: Event Vertical Slice (두 번째 인스턴스 도메인 — Phase 5 분리 첫 부분)

> **For Claude Code.** 이 문서는 자급 자족이며 외부 링크 없이도 작업 시작 가능.
> 결정 사항을 임의 변경하지 말 것. 변경이 필요하면 보고서에서 디렉터 승인 요청.
> **선행 조건**: Phase 0~4 모두 종결.
> **체크포인트 분리 게이트 강제 적용** — Phase 1 미준수 후속, Phase 2·3·4에서 정상. 1회 통합 commit 금지.

---

## 1. 목표

장르 중립 Worldbuilding 도구의 **두 번째 인스턴스 도메인 = Event**를 끝까지 한 사이클.

Phase 5는 **두 결로 분리**:
- **Phase 5a (본 TASK)** — Event 인스턴스 도메인 단독. era_id 외래키는 텍스트만 보존.
- **Phase 5b (5a 종결 후)** — Era 인스턴스 도메인 + Timeline view (Event×Era) + **View trait 일반화** (Atlas + Timeline 두 사례로 패턴 추출) + Atlas overlay 활성 (era_id 외래키 승급, atlas-daejin-empire 같은 시기별 atlas 분기).

**5a의 책임**:
1. **Event 도메인** — id·name·aliases·kind·category·summary·tags·extras·temporal·era_id(텍스트)·participants(people·groups·places 외래키)·body_sections
2. **Phase 1·2·3 외래키 활성 확장** — `Event.participants.people/groups/places` 검증 활성 (에러). 첫 외래키 매트릭스가 Phase 4 atlas와 다른 도메인 방향으로 확장.
3. **270년 28사건의 첫 변환** — "붉은 밤의 변"(10년 전, 메인 서사 분기점) 단독 + 4-9건 추가

**5a의 책임 외**:
- View trait 일반화 — Phase 5b
- Era 도메인 + Timeline view — Phase 5b
- Atlas overlay (시기별 atlas 분기) — Phase 5b
- gameplay 다리 (Scenario·Scene·Beat·Memory 통합) — Phase 6+

**검증 게이트**: `wuxia-core/docs/world/history.md`(270년 연표) + `history-characters.md`(역사 인물·문파 배치)에서 5-10 Event 변환:
- 체크포인트 1: **붉은 밤의 변** 단독 변환 + Phase 1·2·3 외래키 활성 시연
- 체크포인트 2: 4-9 Event 추가 (붉은 밤 외 핵심 사건) + MCP 도구 + 정성 평가

## 2. 연관 컨텍스트

- `CLAUDE.md` 프로젝트 루트
- `docs/tasks/00-roadmap.md` — 전체 흐름·결정 로그
- 이전 Phase 보고서들 — `phase{0,1,2,3,4}-*-report.md`
- 메모리(Cowork 세션 보유): 9 인스턴스 + 1 관계 도메인, 작업 순서, Phase 5a/5b 분리 결정, 첫 Event = "붉은 밤의 변"
- 입력 자료:
  - `wuxia-core/docs/world/history.md` — 270년 연표 (5 시대 + 28 사건)
  - `wuxia-core/docs/world/history-characters.md` v1.2 — 역사 인물·문파 배치 (사건별 관여자)
  - Phase 1~4 산출 — `projects/chilguk-chunchu/world/{group,person,place,atlas}/*.md`

## 3. 제약

### 3.1 장르 중립 vs 의존

| 위치 | 책임 |
|---|---|
| `src/domain/world/event.rs` | **장르 영원히 모름** — id·name·aliases·kind(String)·category·summary·tags·extras·temporal·era_id(Option<String>)·participants(ParticipantsRefs)·body_sections |
| `src/worldbuilding/markdown/event.rs` | 장르 중립 frontmatter+섹션 파서 |
| `src/adapter/sqlite_world.rs` (확장) | `events` 테이블 + FTS5 + `event_participants_refs` 테이블 + `migrate_v5` |
| `genres/wuxia/forms/event.toml` | Phase N 빈 슬롯 |
| `genres/wuxia/markdown_template/event.md` | 무협 event 양식 |
| `projects/chilguk-chunchu/world/event/*.md` | 칠국춘추 사건 인스턴스 |

**`src/`에 wuxia 단어 X.** 혈교·붉은 밤·구파일방 같은 어휘는 모두 `genres/wuxia/`·`projects/`에만.

### 3.2 Phase 1·2·3 외래키 매트릭스 확장

| 검증 (Phase 5a 활성) | 정책 |
|---|---|
| `Event.participants.people` 각 ID → `persons.id` 존재 | **에러** (활성) |
| `Event.participants.groups` 각 ID → `groups.id` 존재 | **에러** (활성) |
| `Event.participants.places` 각 ID → `places.id` 존재 | **에러** (활성) |
| `Event.era_id` (있으면) | **텍스트만** (Phase 5b에서 활성) |
| `Event.related_events` (있으면) | 같은 도메인 내라 활성. cycle 검증은 Phase 5+ (대부분 사건은 비순환이라 단순 결손만) |

Phase 1·2·3 패턴 그대로. partial commit 방지.

### 3.3 Event 카테고리 — `EventCategory` 일급 enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventCategory {
    /// 이미 일어남, 캐논. 시드 자료의 28 사건 대부분.
    Historical,
    /// Phase 6+ 게임 도중 발생할 예정 사건. Phase 5a엔 미사용.
    Scheduled,
    /// 진위 불확실 (전설·구전).
    Legendary,
}
```

Phase 5a에선 `Historical` 위주, `Scheduled`는 Phase 6+ gameplay 다리에서 본격 활용.

### 3.4 Temporal — Era 결합 전 자유 텍스트 + relative

```rust
pub struct EventTemporal {
    /// 자유 텍스트 — "10년 전", "270년차", "원년"
    pub year: Option<String>,
    /// 270년차 기준 절대 연도 — Phase 5b Era 결합 시 정형 시간으로 승급. 정렬 가능.
    pub year_relative: Option<i32>,
    /// 사건 지속 — "사흘 밤", "수년" 등 자유 텍스트
    pub duration: Option<String>,
    /// 자유 메모
    pub notes: Option<String>,
}
```

Phase 5a엔 `year_relative`로 정렬·필터 (Phase 5b Era 정형 전 임시 시간 축). Phase 5b에서 `era_id` 외래키 활성 후 절대 연도 계산은 Era 도메인이 책임.

### 3.5 ParticipantsRefs — 외래키 셋 모음

```rust
pub struct ParticipantsRefs {
    pub people: Vec<String>,    // PersonId 텍스트 — Phase 2 외래키 검증
    pub groups: Vec<String>,    // GroupId 텍스트 — Phase 1 외래키 검증
    pub places: Vec<String>,    // PlaceId 텍스트 — Phase 3 외래키 검증
}
```

세 카테고리 다 활성. 결손 시 에러. cycle 없음(서로 다른 도메인).

### 3.6 SoT = 마크다운

기존 흐름. SQLite는 빌드 산출물(.gitignore).

### 3.7 검색 범위

FTS5 trigram + LIKE fallback (Phase 1 D5 패턴).

### 3.8 체크포인트 분리 게이트 — 강제 적용

1. **체크포인트 1**: `event-bloody-night` (붉은 밤의 변) 단독 변환 + Phase 1·2·3 외래키 활성 시연 → commit pause → `phase5a-checkpoint1-report.md` → Cowork 리뷰
2. **체크포인트 2**: 4-9 Event 추가 + MCP 도구 + 정성 평가 → commit pause → `phase5a-checkpoint2-report.md` → Phase 5a 종결

**1회 통합 commit 금지.**

## 4. Done Criteria

- [ ] 디렉토리 골격: `src/domain/world/event.rs`(stub 채움), `src/worldbuilding/markdown/event.rs`
- [ ] `Event` 애그리거트 + `EventId` + `EventCategory` + `EventTemporal` + `ParticipantsRefs` + 단위 테스트
- [ ] 마크다운 frontmatter+섹션 파서 + 단위 테스트
- [ ] `genres/wuxia/markdown_template/event.md` 템플릿
- [ ] `genres/wuxia/forms/event.toml` 자리 (Phase N)
- [ ] `SqliteWorldStore` 확장 — `events` + `events_fts` + `event_participants_refs` + `migrate_v5`
- [ ] `bin/world-load` 확장 — `world/event/*.md` 스캔 + participants 외래키 활성 (에러 승급)
- [ ] `bin/mind-studio` MCP 도구 3개: `list_events` · `get_event` · `search_events`
- [ ] **체크포인트 1**: 붉은 밤의 변 단독 변환 + 외래키 활성 라운드트립
- [ ] **체크포인트 2**: 4-9 Event + MCP 정성 평가 + 외래키 결손 0건
- [ ] `cargo build` + `cargo test --features embed` + 기존 e2e 회귀 통과
- [ ] 정성 검증: `list_events(category="historical")` → 5-10건 / `list_events(participants_person="npc-02")` → 조고 관여 사건 / `search_events("혈교")` → 매칭

## 5. 단계별 작업

### Step 1 — 디렉토리 골격 + 도메인

```
src/domain/world/
├── event.rs            # Event + EventId + EventCategory + EventTemporal + ParticipantsRefs + EventFilter
├── group.rs, person.rs, place.rs, atlas.rs   # Phase 1·2·3·4 그대로
└── ...

src/worldbuilding/
├── markdown/
│   ├── event.rs        # Event .md → 도메인 (신규)
│   └── ...
├── repository.rs       # WorldRepository — list_events/get_event/search_events 추가

src/adapter/
└── sqlite_world.rs     # events + FTS + event_participants_refs + migrate_v5

src/bin/
└── world_load.rs       # world/event/* 스캔 + participants 외래키 활성

genres/wuxia/
├── forms/event.toml          # Phase N 빈 슬롯
└── markdown_template/event.md

projects/chilguk-chunchu/
└── world/
    └── event/                # 5-10 .md (Step 3·4)
```

#### `Event` 애그리거트

```rust
// src/domain/world/event.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EventCategory {
    #[default]
    Historical,
    Scheduled,
    Legendary,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct EventTemporal {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    /// 270년차 기준 절대 연도 (정렬용). Phase 5b Era 결합 시 정형.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year_relative: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParticipantsRefs {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub people: Vec<String>,    // PersonId 텍스트
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<String>,    // GroupId 텍스트
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub places: Vec<String>,    // PlaceId 텍스트
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub kind: String,                           // "war"|"betrayal"|"founding"|"disaster"|"ritual" 등 — 장르가 채움
    #[serde(default)]
    pub category: EventCategory,                // historical|scheduled|legendary
    pub name: String,
    pub aliases: Vec<String>,
    pub summary: String,
    pub tags: Vec<String>,
    pub extras: serde_json::Map<String, Value>,
    pub temporal: EventTemporal,
    /// 텍스트만 (Phase 5b에서 외래키 활성)
    pub era_id: Option<String>,
    pub participants: ParticipantsRefs,
    pub body_sections: BTreeMap<String, String>,
    /// 같은 도메인 내 외래키 — 다른 사건 참조 (예: 사건의 결과로 일어난 후속 사건)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_events: Vec<EventId>,
    pub source_path: Option<String>,
}
```

#### `WorldRepository` 확장

```rust
pub trait WorldRepository: Send + Sync {
    // Phase 1·2·3·4 (그대로)
    fn list_groups(...)?;
    fn list_persons(...)?;
    fn list_places(...)?;
    fn list_atlases(...)?;

    // Phase 5a — Event
    fn list_events(&self, filter: EventFilter) -> Result<Vec<Event>, WorldError>;
    fn get_event(&self, id: &EventId) -> Result<Option<Event>, WorldError>;
    fn search_events(&self, query: &str, top_k: u32) -> Result<Vec<Event>, WorldError>;
}

pub struct EventFilter {
    pub category: Option<EventCategory>,
    pub kind: Option<String>,
    pub participants_person: Option<String>,    // 특정 인물 관여
    pub participants_group: Option<String>,
    pub participants_place: Option<String>,
    pub year_relative_min: Option<i32>,         // Era 결합 전 임시 정렬 필터
    pub year_relative_max: Option<i32>,
    pub genre_tag: Option<String>,
}
```

산출물 검증: `cargo build` 통과. 단위 테스트 — Event 인스턴스 생성, EventTemporal year_relative 직렬화, ParticipantsRefs 누락 필드 default 처리.

### Step 2 — 마크다운 파이프라인 + participants 외래키 활성

#### Frontmatter 양식 (§6.1 참조)

`serde_yaml` 재사용. line-based H2 파싱.

#### `world-load` 확장

```
cargo run --features embed --bin world-load -- --project chilguk-chunchu [--reload]
```

동작:
1. Phase 1·2·3·4 동작 — group·person·place·atlas 로드
2. Phase 5a 동작 — `world/event/*.md` 로드 → events 테이블 upsert
3. **외래키 검증 활성**:
   - `Event.participants.people` 각 ID → persons 테이블 존재 — 결손 시 **에러**
   - `Event.participants.groups` 각 ID → groups 테이블 존재 — **에러**
   - `Event.participants.places` 각 ID → places 테이블 존재 — **에러**
   - `Event.related_events` 각 ID → events 테이블 (자체 도메인) — **에러**
   - `Event.era_id` (있으면) — 텍스트만 (Phase 5b에서 활성)
4. `event_participants_refs` 테이블에 양방향 인덱스 정방향 (event → participant) + 역방향 인덱스 (`idx_epr_person`·`idx_epr_group`·`idx_epr_place`)
5. partial commit 방지 (Phase 1·2·3·4 패턴 그대로)

산출물 검증: 빈 event .md 라운드트립 + 외래키 결손/통과 양쪽 케이스 단위 테스트.

### Step 3 — 붉은 밤의 변 단독 변환 시연 ★체크포인트 1★

대상: `event-bloody-night` (붉은 밤의 변, 10년 전). 시드 입력:
- `wuxia-core/docs/world/history.md` 의 "붉은 밤의 변" 항목
- `wuxia-core/docs/world/history-characters.md` 의 H29·H30 등 관련 항목
- Phase 2 시드 — npc-02 조고·npc-07 천순제·npc-01 명경 등의 본문에서 "붉은 밤의 변"·"10년 전" 언급
- Phase 4 atlas-jungwon 의 `## 전사(前史)` "10년 전 붉은 밤의 변" 명시

작업:
1. history.md + history-characters.md 통독 → 붉은 밤의 변의 (a) 발생 시점 (b) 발단 (c) 핵심 인물 (d) 결과 정리
2. `projects/chilguk-chunchu/world/event/event-bloody-night.md` 작성 (양식 §6.1)
3. `cargo run --features embed --bin world-load -- --project chilguk-chunchu`
4. SQLite events 1행 검증 + participants 외래키 통과
5. 라운드트립 — 모든 필드 보존
6. **외래키 활성 시연** — 의도적으로 npc-99 같은 미존재 ID를 participants에 추가했다가 빌드 실패 확인 → 정상 ID로 복귀 → 빌드 성공

**체크포인트 1 보고서** (`docs/tasks/phase5a-checkpoint1-report.md`):
- `git diff --stat`
- `event-bloody-night.md` 전문
- 로드 후 Event 도메인 객체 dump (JSON)
- world-load 결과 (events indexed = 1, fk errors = 0)
- 외래키 활성 시연 (의도적 결손 + 복구)
- **변환 시 결정한 것**:
  - `kind` (예: "betrayal"·"war"·"disaster" 중)
  - `aliases` (예: "붉은 밤"·"10년 전 변란")
  - `temporal.year` 자유 텍스트 + `year_relative = 260` (270년차 기준 -10년)
  - `temporal.duration` (예: "사흘 밤" 또는 "수일")
  - `participants.people` (npc-02·07·01 등)
  - `participants.groups` (group-daejin-court·group-mulim-mang·혈교 잔당 etc)
  - `participants.places` (place-daejin·place-namgung 등)
  - `era_id` 잠정 텍스트 (예: "era-fall-of-empire"·"era-current") — Phase 5b 결정
  - `related_events` (있으면 — 다른 사건과의 관계)
- **막힌 결정**: 디렉터 결정 필요 사항 (특히 혈교 그룹 미정 — Phase 1 group 미등록 — 처리 방안)
- Step 4 진행 가능 여부 의견

→ Cowork 리뷰 → **commit pause 유지** → 통과 신호 받고 다음 단계.

### Step 4 — 4-9 Event 추가 + MCP 도구

체크포인트 1 통과 후 4-9 Event 추가 (28 사건 중 핵심 일부).

**Step 4 시작 시 디렉터 결정 — 추가 Event 후보**:

후보 리스트 (history.md 기반, 우선순위):
1. `event-empire-founding` (270년 전, 통일제국 대진 건국) — 시대 시작점, ★★★★★
2. `event-bloody-cult-rebellion-1st` (200년 전쯤, 1차 혈교 반란) — 혈교 첫 등장, ★★★★
3. `event-bloody-cult-rebellion-2nd` (30년 전, 2차 혈교 침공) — 황실 권위 손상, ★★★★
4. `event-blood-disappearance` (12년 전, 피의 실종 사건) — player·소연 시드, ★★★★★
5. `event-hwasan-fall` (10년 전, 화산파 멸문) — player 직접 연관, ★★★★★
6. `event-six-states-independence` (5-7년 전, 6국 독립) — 칠국춘추 형성, ★★★
7. `event-yiseowoon-disappearance` (10년 전, 임서운 행방불명) — player 메인 비밀, ★★★

체크포인트 2 권장 = 5-7건 (붉은 밤 + 위 4-6건). 디렉터 의견 받음.

5-7건 Event 다 변환 후:
- world-load 외래키 0건 (Phase 1·2·3·4 시드 + Phase 5a Event 모두 정합)
- MCP 도구 3종 정성 평가
- search 6쿼리 — "혈교"·"붉은 밤"·"임서운"·"화산"·"건국"·"독립"

### Step 5 — MCP 도구 3개 노출 (Step 4 통합)

```
list_events(filter: EventFilter) -> Vec<EventSummary>
  EventSummary { id, name, kind, category, year_text, summary_one_line, tags }

get_event(event_id: String) -> Option<EventDetail>
  EventDetail = full Event

search_events(query: String, top_k: u32 = 5) -> Vec<EventSummary>
  FTS5 trigram (name + aliases + summary + body)
```

`AppState`에 이미 부착된 `world_store` 그대로 활용. 환경변수 변경 없음.

**체크포인트 2 보고서** (`docs/tasks/phase5a-checkpoint2-report.md`):
- `list_events()` 결과 (5-10 Event — 붉은 밤 포함)
- `list_events(category="historical")` 결과
- `list_events(participants_person="npc-02")` → 조고 관여 사건 (붉은 밤·2차 혈교 침공·피의 실종 등 다수)
- `list_events(participants_place="place-daejin")` → 대진 관여 사건
- `get_event("event-bloody-night")` 전체 detail
- search_events 6쿼리
- 외래키 결손 0건 검증
- year_relative 정렬 시연 — `list_events(year_relative_min=-30, year_relative_max=0)` 등
- Phase 5b (Era + Timeline view + View trait 일반화) 진입 가능 여부 의견

→ Cowork 리뷰 → 통과 시 Phase 5a 종결 → Phase 5b 작전 작성.

## 6. 결정 사항 (변경 시 디렉터 승인)

### 6.1 Frontmatter 양식 — event-bloody-night 예시

```yaml
---
id: event-bloody-night
kind: betrayal                                # betrayal | war | founding | disaster | ritual | discovery
category: historical
name: 붉은 밤의 변
aliases:
  - 붉은 밤
  - 10년 전 변란
  - 6국 독립의 시작
summary: |
  10년 전(260년차) 통일제국 대진의 영토 와해를 가져온 결정적 사건. 대진 황실 내부 권력
  투쟁 + 혈교 잔당의 침투 + 6 지역 독립 운동이 한 밤에 폭발. player의 부친 임서운이
  행방불명된 사건이며, 화산파 멸문의 직전 사건.
tags: [wuxia, event, historical, fall-of-empire, current-era-trigger]
temporal:
  year: 10년 전 (260년차)
  year_relative: -10                          # 270년차 기준 -10
  duration: 사흘 밤
  notes: |
    Phase 5b Era 결합 시 era_id="era-fall-of-empire"로 정형 시간 승급 예정.
era_id: ~                                     # Phase 5b 외래키 자리
participants:
  people:
    - npc-02                                  # 조고 — 권력 투쟁의 한 축
    - npc-07                                  # 천순제 — 즉위 전후 권력 공백
    - npc-01                                  # 명경 — 정파 정보망 일원
  groups:
    - group-daejin-court                      # 대진 황실 — 영토 와해의 주체
    - group-shipsangsi                        # 십상시 — 조고가 권력 장악
    - group-mulim-mang                        # 무림맹 — 사건 후 정파 동맹 흔들림
  places:
    - place-daejin                            # 대진 영토 — 사건 무대
    - place-namgung                           # 남궁 — 6 지역 중 첫 독립
    - place-jiyu-doshi                        # 자유도시 — 5년 전 자치령 선언의 원인
related_events:
  - event-blood-disappearance                 # 12년 전 피의 실종 (전조)
  - event-hwasan-fall                         # 10년 전 화산파 멸문 (직후)
  - event-six-states-independence             # 5-7년 전 6국 독립 (결과)
extras:
  trigger: 천순제 즉위 직후 권력 공백 + 혈교 잔당 침투 + 지방 영주 자립 운동 동시 폭발
  outcome: 6 지역 독립 → 칠국춘추 시대 시작
  game_role: 메인 서사 분기점 1 — player의 부친 임서운 행방불명 + 메인 적대자 조고의 권력 장악
  player_relevance: ★★★★★
---

## 개요
산문 1-2 단락 — 사건 핵심 묘사.

## 발단
산문 — 사건 직전 상황 + 트리거.

## 전개
산문 — 사흘 밤 동안의 전개. 핵심 인물·장소 명시.

## 결과
산문 — 영토 와해·6국 독립·임서운 행방불명·player 어머니 사망 등.

## 핵심 인물
- npc-02 조고: 십상시 통한 권력 장악
- npc-07 천순제: 꼭두각시 황제 즉위
- npc-01 명경: 정파 정보망 일원으로 사건 인지
- (npc 미등록) 임서운: player 부친, 행방불명

## 게임에서의 역할
- 메인 서사 분기점 1
- player 시작 시점의 직전 사건 (player가 어릴 때 직접 겪음)
- 메인 적대자 조고의 권력 기반
- 모든 칠국 정치체의 현재 정체성 트리거
```

권장 H2 섹션: `## 개요` · `## 발단` · `## 전개` · `## 결과` · `## 핵심 인물` · `## 게임에서의 역할`.

### 6.2 `genres/wuxia/forms/event.toml` (Phase N 빈 슬롯)

```toml
extends = "event"

[[fields.kind.options]]
value = "betrayal"; label = "변란/배신"
[[fields.kind.options]]
value = "war"; label = "전쟁"
[[fields.kind.options]]
value = "founding"; label = "건국"
[[fields.kind.options]]
value = "disaster"; label = "재해"
[[fields.kind.options]]
value = "ritual"; label = "의례"
[[fields.kind.options]]
value = "discovery"; label = "발견"

[[fields.category.options]]
value = "historical"; label = "역사 사건"
[[fields.category.options]]
value = "scheduled"; label = "예정 사건 (Phase 6+)"
[[fields.category.options]]
value = "legendary"; label = "전설"
```

### 6.3 SQLite 스키마

```sql
CREATE TABLE events (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'historical' CHECK(category IN ('historical','scheduled','legendary')),
    name TEXT NOT NULL,
    aliases_json TEXT NOT NULL DEFAULT '[]',
    summary TEXT NOT NULL DEFAULT '',
    tags_json TEXT NOT NULL DEFAULT '[]',
    extras_json TEXT NOT NULL DEFAULT '{}',
    temporal_json TEXT NOT NULL DEFAULT '{}',
    year_relative INTEGER,                    -- 캐시 컬럼 (정렬용)
    era_id TEXT,                              -- 텍스트 보존, Phase 5b 외래키 활성
    participants_json TEXT NOT NULL DEFAULT '{}',
    body_sections_json TEXT NOT NULL DEFAULT '{}',
    related_events_json TEXT NOT NULL DEFAULT '[]',
    source_path TEXT,
    updated_at INTEGER NOT NULL
);
CREATE INDEX idx_events_kind ON events(kind);
CREATE INDEX idx_events_category ON events(category);
CREATE INDEX idx_events_year_relative ON events(year_relative);
CREATE INDEX idx_events_era_id ON events(era_id);

CREATE VIRTUAL TABLE events_fts USING fts5(
    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
);

CREATE TABLE event_participants_refs (
    event_id TEXT NOT NULL,
    ref_kind TEXT NOT NULL CHECK(ref_kind IN ('person','group','place')),
    ref_id TEXT NOT NULL,
    ref_order INTEGER NOT NULL,
    PRIMARY KEY (event_id, ref_kind, ref_id)
);
CREATE INDEX idx_epr_person ON event_participants_refs(ref_id) WHERE ref_kind = 'person';
CREATE INDEX idx_epr_group ON event_participants_refs(ref_id) WHERE ref_kind = 'group';
CREATE INDEX idx_epr_place ON event_participants_refs(ref_id) WHERE ref_kind = 'place';
CREATE INDEX idx_epr_event ON event_participants_refs(event_id);
```

`schema_meta.version = 5` 마이그레이션. Phase 4 v4 DB는 자동 ALTER + events·events_fts·event_participants_refs 추가.

### 6.4 환경변수

`NPC_MIND_WORLD_DB` 그대로.

### 6.5 라이브러리

기존 — Phase 0 D2·D3 의존성 회피 원칙 계승.

### 6.6 외래키 매트릭스 (Phase 5a 활성)

| 검증 | Phase 1 | Phase 2 | Phase 3 | Phase 4 | Phase 5a |
|---|---|---|---|---|---|
| Group / Person / Place / Atlas (이전 Phase) | (Phase 별) | ... | ... | ... | 그대로 |
| **`Event.participants.people`** | — | — | — | — | **에러** (활성) |
| **`Event.participants.groups`** | — | — | — | — | **에러** |
| **`Event.participants.places`** | — | — | — | — | **에러** |
| `Event.related_events` 존재 | — | — | — | — | **에러** (자체 도메인) |
| `Event.era_id` 존재 | — | — | — | — | 텍스트만 (Phase 5b 활성) |

### 6.7 붉은 밤의 변 작성 가이드

`history.md`엔 직접 시트 없을 가능성 — 항목별 산문일 가능성. `history-characters.md` v1.2의
"H29·H30" 같은 사건 번호와 관련 인물 매핑 활용.

권장 결정값 (디렉터 검토):
- `id` = `event-bloody-night`
- `kind` = `betrayal` (배신·변란이 본질) 또는 `war` (대규모 충돌이라면). 디렉터 결정.
- `aliases` 2-3개 — `["붉은 밤", "10년 전 변란"]` 권장
- `temporal.year_relative = -10`
- `temporal.duration` = "사흘 밤" (게임 캐논 결정 또는 history.md 참조)
- `participants.people` = npc-01·02·07 (Phase 2 시드의 인물들. npc-08~11 추가 가능)
- `participants.groups` = group-daejin-court·shipsangsi·mulim-mang (혈교 잔당은 Phase 1 미등록 — 디렉터 결정)
- `participants.places` = place-daejin·namgung·jiyu-doshi (혈교 거점은 미등록)
- `era_id` = 잠정 텍스트 ("era-fall-of-empire" 또는 "era-current") — Phase 5b 결정
- `related_events` = peer event 미등록이라 비움 또는 위 §5 Step 4 후보 IDs 미리 텍스트로

**막힌 결정 후보**: 혈교 그룹 미등록 처리 — Phase 1 group으로 추가 vs Phase 6+ legendary group으로 미루기 vs participants.groups에서 누락. 디렉터 결정 필요.

## 7. Out of Scope (Phase 5a)

- **Era 도메인** — Phase 5b
- **Timeline view (Event × Era)** — Phase 5b
- **View trait 일반화** (Atlas + Timeline 두 사례로 추출) — Phase 5b
- **Atlas overlay 활성** (시기별 atlas 분기) — Phase 5b
- **gameplay 다리** (Scenario·Scene·Beat·Memory 통합) — Phase 6+
- absolute year 계산 (Era 결합 후) — Phase 5b
- Event hierarchy (사건 → 하위 사건) — Phase 6+
- Event branching (선택지 분기) — Phase N (gameplay)
- AI 자동 사건 생성 — Phase N+

## 8. 코드 위치 가이드

작업 시작 5분에 읽을 곳:

| 위치 | 무엇을 볼지 |
|---|---|
| `Cargo.toml` `[features]` | feature 게이팅 |
| `src/domain/world/group.rs`·`person.rs`·`place.rs` | aliases·spatial·외래키 패턴 미러 |
| `src/worldbuilding/markdown/{group,person,place}.rs` | 마크다운 파서 패턴 |
| `src/adapter/sqlite_world.rs` (Phase 4) | migrate_v4·atlases·place_atlas_refs — Phase 5a `migrate_v5`+events+event_participants_refs 미러 |
| `src/bin/world_load.rs` (Phase 4) | 외래키 검증 흐름 |
| `src/bin/mind-studio/handlers/world_atlases.rs` (Phase 4) | MCP·REST 패턴 |

## 9. 시작 체크리스트

1. `CLAUDE.md` + Phase 0~4 산출 빠르게 훑기
2. **`wuxia-core/docs/world/history.md`** + **`history-characters.md` v1.2** 통독 — Phase 5a 핵심 입력
3. Phase 1·2·3·4 산출 (`projects/chilguk-chunchu/world/{group,person,place,atlas}/*.md`) ID 목록 확인 — participants 외래키에 들어갈 것
4. Event 도메인 + EventCategory + EventTemporal + ParticipantsRefs + 마크다운 파서 + 단위 테스트 (Step 1·2)
5. SqliteWorldStore migrate_v5 + events·event_participants_refs + 라운드트립 테스트
6. world-load 확장 — participants 외래키 활성
7. **붉은 밤의 변 변환** → ★체크포인트 1★ 보고 → **commit pause**

## 10. 리뷰 채널

체크포인트 1·2 보고서를 디렉터(사용자)가 Cowork 세션에 복붙. 형식:
- **Done** · **Diff** · **데모 명령** · **결정** · **막힌 것** · **다음 의견**
- 변환 시 모든 추론·결정 (특히 혈교 그룹 미등록 처리·temporal year_relative·era_id 잠정 텍스트)을 본문에 상세히 명시

보고서 파일명:
- 체크포인트 1: `docs/tasks/phase5a-checkpoint1-report.md`
- 체크포인트 2: `docs/tasks/phase5a-checkpoint2-report.md`
