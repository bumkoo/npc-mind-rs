# Phase 2: Person 카테고리 Vertical Slice (+ NPC Mind 통합)

> **For Claude Code.** 이 문서는 자급 자족이며 외부 링크 없이도 작업 시작 가능.
> 결정 사항을 임의 변경하지 말 것. 변경이 필요하면 보고서에서 디렉터 승인 요청.

## 1. 목표

장르 중립 Worldbuilding 도구의 **두 번째 인스턴스 도메인 = Person**을 끝까지 한 사이클. 동시에 **worldbuilding 도구 ↔ 기존 npc-mind 엔진의 첫 다리**를 놓는다.

이번 Phase에서 다루는 추상은 **세 결**:
1. **Person 도메인** — 사람 인스턴스 (id·name·aliases·kind·status·hexaco·affiliation·birthplace 등)
2. **Phase 1 Group 외래키 활성** — `MemberRef.person_id` 검증을 텍스트 → 정식 외래키로
3. **NPC Mind 통합** — `Person.id` ↔ `npc_mind::Npc.id`. world-load가 mind engine `NpcRepository`에 자동 upsert.

`인터뷰/마크다운(SoT) → DDD 도메인 → 인프라 → MCP 노출 + npc-mind 등록` 파이프라인이 Person 한정으로 돌아가는지를 검증.

**검증 게이트**: `wuxia-core/docs/characters/character-roster.md` v1.1 + 개별 npc-01~11 열전을 입력으로:
- 5-6 Person 변환 (체크포인트 1: 조고 단독 → 체크포인트 2: 5인 일괄)
- world-load 후 mind-studio에서 `dialogue_start("npc-02")` 호출 → 조고가 HEXACO 기반 가이드 생성 가능

## 2. 연관 컨텍스트

- `CLAUDE.md` 프로젝트 루트 — 기존 NPC mind·Scene·EventBus·CQRS·HEXACO 기반 OCC appraisal·PAD 분석기 구조
- `docs/tasks/00-roadmap.md` — 전체 Phase 흐름
- `docs/tasks/task-phase1-group-vertical-slice.md` — Phase 1 사양
- `docs/tasks/phase1-implementation-report.md` — Phase 1 결과 (보고서 §7 "Phase 2 진입 가능성" 참고)
- 메모리(Cowork 세션 보유): 9 인스턴스 도메인 + 1 관계 도메인, 작업 순서, Group 핵심 필드, 청강만리/칠국춘추 구분
- 입력 자료:
  - `wuxia-core/docs/characters/character-roster.md` v1.1 — 인물 총람 (12명 + 플레이어)
  - `wuxia-core/docs/characters/npc-01.md ~ npc-11.md` — 개별 열전 (있는 것만)
  - `wuxia-core/docs/world/history-characters.md` — 역사 인물 (이미 사망)
  - `wuxia-core/docs/world/character-naming.md` — 명명 규칙 v1.1
  - Phase 1 산출 `projects/chilguk-chunchu/world/group/*.md` — 조고는 group-daejin-court·group-shipsangsi의 멤버로 이미 등록됨

## 3. 제약

### 3.1 장르 중립 vs 의존

| 위치 | 책임 |
|---|---|
| `src/domain/world/person.rs` | **장르 영원히 모름** — id·name·aliases·kind(String)·status·**hexaco**·temporal·affiliation·birthplace·current_location·summary·tags·extras·body_sections |
| `src/worldbuilding/markdown/person.rs` | 장르 중립 frontmatter+섹션 파서 |
| `src/adapter/sqlite_world.rs` (확장) | `persons` 테이블 + FTS5 (Phase 1 패턴 미러) |
| `genres/wuxia/forms/person.toml` | 무협 kind 옵션·확장 필드 (Phase 2 폼 미사용, 자리만) |
| `genres/wuxia/markdown_template/person.md` | 무협 .md 작성 템플릿 |
| `projects/chilguk-chunchu/world/person/*.md` | 칠국춘추 인스턴스 |

**`src/`에 wuxia 단어 X.** 사문·무공·강호·문파 같은 어휘는 `genres/wuxia/`·`projects/chilguk-chunchu/`에만.

### 3.2 HEXACO 표현 — Q1·B 결정

`HexacoSix` 6 dimension을 **일급 필드**. 24 facet은 별도 자리:
- 6 dim: frontmatter `hexaco:` 블록에 정형 (필수). 0.0~1.0 또는 -1.0~+1.0(기존 npc-mind 컨벤션 따름)
- 24 facet: `extras.hexaco_facets` 정형 JSON (선택). 없으면 character-roster·열전 본문 산문에서 점진 채움
- 본문 `## HEXACO 분석` 섹션은 자유 산문 (작가 메모용)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HexacoSix {
    pub honesty_humility: f32,
    pub emotionality: f32,
    pub extraversion: f32,
    pub agreeableness: f32,
    pub conscientiousness: f32,
    pub openness: f32,
}
```

값 범위·default·기존 npc-mind와의 정합성은 §6.7 결정 사항 참고.

### 3.3 Status × Kind 두 축 — Q3·C 결정

```rust
pub enum PersonStatus {
    Alive,      // 게임 시작 시 생존
    Dead,       // 사망 (역사 인물 또는 게임 도중 사망)
    Missing,    // 생사 불명 (퀘스트 대상)
    Unknown,    // 정보 없음
}

pub enum PersonKind {
    // 도구 인지하는 추상 enum 또는 String?
    // 결정: String — Group과 일관성 (장르 확장)
}
```

`kind`는 free-form String. wuxia 패키지가 옵션 채움:
- `historical` — 이미 사망한 역사 인물 (NPC 대사·서적·비문 등장)
- `active` — 게임 시작 시 생존, 직접 만남
- `legendary` — 실존 여부 불확실 (퀘스트 단서·비급 저자)
- `player` — 플레이어 캐릭터 (Q2·B 결정 — 단일 카테고리 sub-kind)

**Q2·B 적용**: 별도 PlayerCharacter 도메인 만들지 않음. Person.kind="player" 인스턴스 1개만 등록 (칠국춘추엔 17세 화산파 유일 생존자).

### 3.4 Phase 1 Group 외래키 활성 — Q5 A의 기반

`affiliation: Vec<GroupId>` 일급 필드. Phase 1에선 텍스트만 보존됐으나 Phase 2부터 **빌드 타임 검증 활성**:
- world-load 시 affiliation 각 GroupId가 `groups` 테이블에 존재하는지 확인 — 결손 시 **에러** (경고가 아닌)
- Phase 1 Group의 `members.person_id` 검증도 같은 시점 활성 — 결손 시 에러

`birthplace`·`current_location`은 Phase 3 Place까지 텍스트만 보존(경고).

### 3.5 NPC Mind 통합 — Q5·A 결정 (이번 Phase의 핵심)

Person 도메인이 정의되면 **world-load가 npc-mind 엔진의 NpcRepository에 자동 upsert**.

```
projects/chilguk-chunchu/world/person/npc-02.md
  ↓ world-load --project chilguk-chunchu
  ├─ persons 테이블 upsert (worldbuilding SQLite)
  ├─ FTS5 인덱스 갱신
  └─ npc-mind NpcRepository upsert ← 신규
       Person.hexaco → npc_mind::Npc.personality
       Person.id → npc_mind::NpcId (같은 ID 직접 사용)
       Person.name → npc_mind::Npc.name
       (selected fields)
```

매핑 정책:
- Person.id == NpcId 직접 사용 (예: `npc-02` ↔ `npc-02`)
- HEXACO 6 dim 직접 매핑 — **둘 다 -1.0 ~ +1.0 범위로 확정** (§6.7). `Score` VO 재사용 가능
- `kind="active"` 또는 `kind="player"`만 mind upsert. `historical`·`legendary`는 mind에 등록 안 함 (대화 안 함, NPC mind 슬롯 절약)
- mind upsert는 **idempotent** — 같은 .md 두 번 로드해도 안전
- mind 시스템에 이미 같은 NpcId 존재 시: HEXACO·name 갱신, 기존 emotion_state·scene·memory는 보존
- **PAD 초기 무드는 Phase 2 미포함** (§7 OoS) — mind 시스템 default(0,0,0) 또는 HEXACO 기반 자동 추론에 위임

검증 게이트: world-load 후 mind-studio에서 `dialogue_start("npc-02")` 호출이 정상 동작 — 조고가 자기 HEXACO 기반 system_prompt 받고 답변 생성 가능.

### 3.6 SoT = 마크다운

기존 흐름 동일. SQLite는 빌드 산출물(.gitignore).

### 3.7 검색 범위

FTS5 trigram + 2-char LIKE fallback (Phase 1 D5 패턴 그대로).

### 3.8 체크포인트 분리 게이트 — Phase 1 미준수 후 강제 적용

**Phase 1에서 1회 통합 commit으로 게이트 미준수.** Phase 2부터 강제:

1. **체크포인트 1 진입**: 조고 단독 변환 + 라운드트립 + npc-mind 자동 등록 검증 → commit pause → 보고서 #1 (`phase2-checkpoint1-report.md`) → Cowork 리뷰 → 통과 신호 받기
2. **통과 후 체크포인트 2**: 4-5 인물 일괄 + MCP 정성 평가 → commit pause → 보고서 #2 (`phase2-checkpoint2-report.md`) → Cowork 리뷰

**1회 commit으로 통합 금지.** Phase 1 보고서 §6에 자체 시인된 항목.

## 4. Done Criteria

- [ ] 디렉토리 골격: `src/domain/world/person.rs` (기존 stub 채움), `src/worldbuilding/markdown/person.rs`
- [ ] `Person` 애그리거트 + `HexacoSix` + `PersonStatus` + `WorldRepository` 트레잇 확장 + 단위 테스트
- [ ] 마크다운 frontmatter+섹션 파서 + 단위 테스트
- [ ] `genres/wuxia/markdown_template/person.md` 템플릿 (§6.1)
- [ ] `genres/wuxia/forms/person.toml` 자리 (Phase 2 빈 슬롯)
- [ ] `SqliteWorldStore` 확장 — `persons` 테이블 + FTS5 + `migrate_v2` 마이그레이션
- [ ] `bin/world-load` 확장 — `world/person/*.md` 스캔 추가
- [ ] **NPC Mind 통합** — world-load가 `npc_mind::NpcRepository`에 자동 upsert (active·player만)
- [ ] **Phase 1 외래키 활성** — `Group.members.person_id`·`Person.affiliation` 검증 에러로 승급
- [ ] `bin/mind-studio` MCP 도구 3개: `list_persons` · `get_person` · `search_persons`
- [ ] **체크포인트 1**: npc-02 조고 단독 변환 + mind upsert + dialogue_start 검증
- [ ] **체크포인트 2**: 5-6 Person 변환 + MCP 정성 평가 + 외래키 라운드트립
- [ ] `cargo build` + `cargo test --features embed` + `cargo test --features mind-studio,embed,chat` 통과
- [ ] 정성 검증: `list_persons(kind="active")` → 4-5 / `list_persons(status="dead")` → 0-1 / `dialogue_start("npc-02")` 동작

## 5. 단계별 작업

### Step 1 — 디렉토리 골격 + 도메인

```
src/domain/world/
├── person.rs       # Person + PersonId + PersonStatus + HexacoSix (기존 stub 채움)
├── group.rs        # Phase 1 그대로
├── ...

src/worldbuilding/
├── markdown/
│   ├── person.rs   # Person .md → 도메인 (신규)
│   └── group.rs, frontmatter.rs   # Phase 1 재사용
├── repository.rs   # WorldRepository 확장 — list_persons/get_person/search_persons

src/adapter/
└── sqlite_world.rs # persons 테이블 + FTS + migrate_v2

src/bin/
└── world_load.rs   # world/person/* 스캔 추가 + npc-mind upsert

genres/wuxia/
├── forms/person.toml          # Phase 2 빈 슬롯
└── markdown_template/person.md

projects/chilguk-chunchu/
└── world/
    └── person/                # 5-6 .md (Step 3·4)
```

#### `Person` 애그리거트

```rust
// src/domain/world/person.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PersonId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PersonStatus {
    #[default]
    Alive,
    Dead,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HexacoSix {
    #[serde(default)] pub honesty_humility: f32,
    #[serde(default)] pub emotionality: f32,
    #[serde(default)] pub extraversion: f32,
    #[serde(default)] pub agreeableness: f32,
    #[serde(default)] pub conscientiousness: f32,
    #[serde(default)] pub openness: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PersonTemporal {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birth_year: Option<String>,         // "원년" "270년 전" "AD 1980" 등 자유 텍스트
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub death_year: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age_at_game_start: Option<u32>,     // character-roster의 "나이" 컬럼 매핑
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub id: PersonId,
    pub kind: String,                       // "active"|"historical"|"legendary"|"player" (장르가 채움)
    pub name: String,
    pub aliases: Vec<String>,               // 별호·자(字)·호(號)·옛 이름
    pub status: PersonStatus,
    pub hexaco: HexacoSix,                  // 6 dim 일급
    pub temporal: PersonTemporal,
    pub affiliation: Vec<GroupId>,          // Phase 1 Group ID — Phase 2부터 정식 외래키
    pub birthplace: Option<String>,         // Place ID 텍스트 (Phase 3)
    pub current_location: Option<String>,   // Place ID 텍스트 (Phase 3)
    pub summary: String,
    pub tags: Vec<String>,
    pub extras: serde_json::Map<String, Value>,    // hexaco_facets·signature_skill·biography_short·level 등
    pub body_sections: BTreeMap<String, String>,
    pub source_path: Option<String>,
}
```

#### `WorldRepository` 확장

```rust
#[async_trait_or_sync]   // Phase 1 D1 결정 — sync trait 유지
pub trait WorldRepository: Send + Sync {
    // Phase 1 — Group
    fn list_groups(&self, filter: GroupFilter) -> Result<Vec<Group>, WorldError>;
    fn get_group(&self, id: &GroupId) -> Result<Option<Group>, WorldError>;
    fn search_groups(&self, query: &str, top_k: u32) -> Result<Vec<Group>, WorldError>;
    // Phase 2 — Person 추가
    fn list_persons(&self, filter: PersonFilter) -> Result<Vec<Person>, WorldError>;
    fn get_person(&self, id: &PersonId) -> Result<Option<Person>, WorldError>;
    fn search_persons(&self, query: &str, top_k: u32) -> Result<Vec<Person>, WorldError>;
}

pub struct PersonFilter {
    pub kind: Option<String>,
    pub status: Option<PersonStatus>,
    pub affiliation: Option<GroupId>,        // 특정 Group의 멤버 필터
    pub genre_tag: Option<String>,
}
```

산출물 검증: `cargo build` 통과. 단위 테스트 — Person 인스턴스 생성, HEXACO 직렬화/역직렬화, 라운드트립.

### Step 2 — 마크다운 파이프라인 + npc-mind 어댑터

#### Frontmatter 양식 (§6.1 참조)

`serde_yaml` 재사용 (Phase 1 D2). 마크다운 H2 파싱 line-based (Phase 1 D3).

#### `world-load` 확장

```
cargo run --features embed,chat,mind-studio --bin world-load -- --project chilguk-chunchu [--reload] [--no-mind]
```

동작:
1. Phase 1 동작 — `world/group/*.md` 로드
2. Phase 2 동작 — `world/person/*.md` 로드 → persons 테이블 upsert
3. **NPC mind upsert** (default ON, `--no-mind`로 끔)
   - `kind in {"active", "player"}` 필터
   - `Person.id` → `npc_mind::NpcId` (같은 ID)
   - `Person.hexaco` → `npc_mind::Personality` 변환
   - `npc_mind::NpcRepository::upsert()` 호출 (idempotent)
   - 기존 emotion_state·scene·memory는 **보존** (덮어쓰지 않음)
4. **외래키 검증 (Phase 1 + Phase 2 활성)**:
   - Group.members.person_id 각 ID가 persons에 존재 — 결손 시 에러
   - Person.affiliation 각 ID가 groups에 존재 — 결손 시 에러
   - `birthplace`·`current_location` Place ID는 텍스트만 (Phase 3 활성)

산출물 검증: `--no-mind`로 빌드 → mind upsert 분리 검증.

### Step 3 — 조고(npc-02) 단독 변환 시연 ★체크포인트 1★

대상: **조고** (`npc-02`). character-roster v1.1 §2 + 열전 npc-02 + Phase 1 group-daejin-court·group-shipsangsi 멤버 등록 정보.

작업:
1. `character-roster.md` + `npc-02.md` 통독 (열전 완성도 ★★★ 인물)
2. `projects/chilguk-chunchu/world/person/npc-02.md` 작성
3. `cargo run --features embed --bin world-load -- --project chilguk-chunchu`
4. SQLite persons 테이블에 npc-02 1행 검증
5. Phase 1 외래키 활성 — group-daejin-court.members.npc-02 / group-shipsangsi.members.npc-02 검증 통과
6. **NPC mind upsert 검증** — mind-studio 띄우고:
   ```bash
   curl -X POST http://127.0.0.1:3000/api/dialogue/start \
        -d '{"sid":"test","npc":"npc-02","partner":"player","situation":"..."}'
   ```
   조고가 HEXACO 기반 system_prompt 받고 답변 생성

**체크포인트 1 보고서** (`docs/tasks/phase2-checkpoint1-report.md`):
- `git diff --stat`
- `npc-02.md` 전문 (변환 결과)
- 로드 후 `Person` 도메인 객체 dump (JSON, 모든 필드)
- SQLite persons 1행 검증 + groups의 외래키 검증 통과
- mind-studio dialogue_start 호출 결과 (조고의 첫 답변 발췌)
- **변환 시 결정한 것**:
  - 산문 → 섹션 마커 매핑
  - HEXACO 6 dim 값 결정 근거 (열전·character-roster의 어떤 단서에서 0.x 추출했는지)
  - 24 facet 포함 여부 (extras에 정형 vs 본문 산문)
  - aliases 후보 (대진의 그림자·환관 조고·조 십상시 수장 등)
  - kind/status — active/alive 자명하나 명시
  - affiliation — [group-daejin-court, group-shipsangsi] 정렬 순서·역할 메타
- **막힌 결정**: HEXACO 값 범위 정합성·NPC mind upsert 시 기존 emotion_state 처리·birthplace 표기 등
- Step 4 진행 가능 여부 의견

→ Cowork 리뷰 → **commit pause 유지** → 통과 신호 받고 다음 단계.

### Step 4 — 4-5 Person 일괄 변환 + 외래키 검증 (체크포인트 2)

체크포인트 1 통과 후 다음 인물:

| Person ID | kind | status | affiliation | 시드 출처 |
|---|---|---|---|---|
| `npc-01` | active | alive | aimi-pa(미정) | npc-01 (명경 사태, 정파 양심) |
| `npc-03` | active | alive | group-namgung | npc-03 (남궁혁 검왕) |
| `npc-04` | active | alive | seoryang-dang(미정) | npc-04 (당무괴 독왕) |
| `npc-05` | active | alive | group-gaebang | npc-05 (소연) |
| `npc-06` | active | alive | group-cheonma-shingyo | npc-06 (야율설화) |

추가 후보 (5인 또는 6인):
- `npc-07` 천순제 (열전 미작성, character-roster §3) — Phase 1 group-daejin-court 멤버
- 또는 player character 1명 (kind=player) — character-roster §1

**필수 시연**:
- 외래키 활성 검증 — npc-03의 affiliation에 `group-namgung` 존재 → 검증 통과
- mind upsert 5-6명 — `cargo run world-load`로 일괄 등록
- mind-studio에서 다중 인물 dialogue 동작

**체크포인트 2 보고서** (`docs/tasks/phase2-checkpoint2-report.md`):
- `list_persons(kind="active")` 결과 (5-6명)
- `list_persons(status="alive")` 결과
- `list_persons(affiliation="group-namgung")` → npc-03 (외래키 시연)
- `get_person("npc-03")` 전체 detail
- `search_persons` 5쿼리 — "검왕"·"독왕"·"천이"·"환관"·"늑대왕" 같은 별호 매칭
- mind 통합 검증 — 5-6 NPC가 mind-studio에서 모두 dialogue_start 가능
- 외래키 결손 발생 여부 (Group의 affiliation·person_id 모두 활성)
- Phase 3(Place) 진입 가능 여부 의견

→ Cowork 리뷰 → 통과 시 Phase 2 종료.

### Step 5 — MCP 도구 3개 노출 (Step 4 통합)

```
list_persons(filter: PersonFilter) -> Vec<PersonSummary>
get_person(person_id: String) -> Option<PersonDetail>
search_persons(query: String, top_k: u32 = 5) -> Vec<PersonSummary>
```

`AppState`에 이미 부착된 `world_store`(Phase 1) 그대로 활용. 환경변수 변경 없음.

## 6. 결정 사항 (변경 시 디렉터 승인)

### 6.1 Frontmatter 양식 — npc-02 조고 예시

```yaml
---
id: npc-02
kind: active                                 # active | historical | legendary | player
name: 조고(曹高)
aliases: [대진의 그림자, 환관 조고, 십상시 수장]
status: alive                                # alive | dead | missing | unknown
hexaco:                                      # 6 dim 일급. 범위는 §6.7
  honesty_humility: -0.8                     # 권모술수 — 매우 낮음
  emotionality: -0.3                         # 감정 억제
  extraversion: -0.2                         # 음지 활동 선호
  agreeableness: -0.7                        # 비협조·복수심
  conscientiousness: 0.7                     # 치밀함
  openness: 0.5                              # 새로운 책략 수용
temporal:
  birth_year: 215년 전(추정)
  death_year: ~                              # 생존
  age_at_game_start: 55
  notes: 화산파 멸문 의혹의 배경 인물.
affiliation:
  - group-daejin-court                       # Phase 1 Group 외래키 (정식 활성)
  - group-shipsangsi
birthplace: place-daejin-luoyang             # Phase 3 외래키 텍스트
current_location: place-daejin-luoyang
summary: |
  대진 황실의 그림자. 환관 출신으로 천순제를 꼭두각시 삼고 십상시를
  통해 황실을 실질 통치한다. 메인 적대자.
tags: [wuxia, person, antagonist, eunuch, declining-empire]
extras:
  hexaco_facets:                             # 24 facet 정형 (선택)
    H_sincerity: -0.9
    H_fairness: -0.7
    # ... 나머지 22 facet 또는 본문 산문으로 보충
  signature_skill: 권모술수·정보 조작
  biography_short: 환관으로 입조 후 30년에 걸쳐 황실 권력 장악
  game_role: 메인 적대자 (Main Antagonist)
  priority: ★★★
---

## 개요
산문 1-2 단락 — 인물 핵심 묘사.

## 배경
산문 — 출신·성장·입조 과정.

## 동기
산문 — 무엇을 원하는가, 무엇을 두려워하는가.

## 비밀
산문 — 다른 사람은 모르는 것 (화산파 멸문 의혹·혈교 묵계 등).

## HEXACO 분석
산문 — 6 dim 결정 근거. 24 facet 보충 가능.

## 관계
- group-daejin-court 안에서: 실권자
- group-shipsangsi 안에서: 수장
- npc-01 명경: 적대 (정파의 양심)
- npc-07 천순제: 꼭두각시로 부림

## 게임에서의 역할
산문 — 메인 퀘스트·서사 역할·플레이어 첫 조우 시점.
```

권장 H2 섹션: `## 개요` · `## 배경` · `## 동기` · `## 비밀` · `## HEXACO 분석` · `## 관계` · `## 게임에서의 역할` 7개. 두 섹션(`## 개요` · `## 게임에서의 역할`)은 모든 Person 공통.

#### kind 별 양식 미세 조정

| kind | 권장 추가 섹션 | extras 권장 키 |
|---|---|---|
| `active` | `## 비밀` · `## 동기` | game_role · priority · signature_skill |
| `historical` | `## 사후 평가` · `## 후대 영향` | death_circumstance · legacy |
| `legendary` | `## 전설 유래` · `## 유산` | reality_status · mentioned_in |
| `player` | `## 시작 조건` · `## 빌드 가능성` | starting_level · starting_skills · class |

### 6.2 `genres/wuxia/forms/person.toml` (Phase 2 빈 슬롯)

```toml
extends = "person"

[[fields.kind.options]]
value = "active"; label = "현역"
[[fields.kind.options]]
value = "historical"; label = "역사 인물"
[[fields.kind.options]]
value = "legendary"; label = "전설"
[[fields.kind.options]]
value = "player"; label = "플레이어"

[[fields.status.options]]
value = "alive"; label = "생존"
[[fields.status.options]]
value = "dead"; label = "사망"
[[fields.status.options]]
value = "missing"; label = "행방불명"
[[fields.status.options]]
value = "unknown"; label = "미상"

# 무협 특화 — 사문(師門) — affiliation의 한 부분이지만 무협엔 강한 결
[[fields]]
key = "sect_master"
label = "사문 스승"
type = "person_id"

[[fields]]
key = "signature_skill"
label = "성명절기"
type = "string"
```

### 6.3 SQLite 스키마 — persons 테이블

```sql
CREATE TABLE persons (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    aliases_json TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'alive' CHECK(status IN ('alive','dead','missing','unknown')),
    hexaco_json TEXT NOT NULL DEFAULT '{}',          -- HexacoSix 직렬화
    temporal_json TEXT NOT NULL DEFAULT '{}',
    affiliation_json TEXT NOT NULL DEFAULT '[]',     -- Group ID 배열
    birthplace TEXT,                                  -- Phase 3 외래키 (텍스트)
    current_location TEXT,                            -- Phase 3 외래키 (텍스트)
    summary TEXT NOT NULL DEFAULT '',
    tags_json TEXT NOT NULL DEFAULT '[]',
    extras_json TEXT NOT NULL DEFAULT '{}',
    body_sections_json TEXT NOT NULL DEFAULT '{}',
    source_path TEXT,
    updated_at INTEGER NOT NULL
);
CREATE INDEX idx_persons_kind ON persons(kind);
CREATE INDEX idx_persons_status ON persons(status);

CREATE VIRTUAL TABLE persons_fts USING fts5(
    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
);

-- 외래키 활성 검증을 위한 join 쿼리 인덱스
CREATE INDEX idx_persons_affiliation ON persons(affiliation_json);
```

`schema_meta.version = 2` 마이그레이션. Phase 1 v1 DB는 자동 ALTER TABLE + persons·persons_fts 추가.

### 6.4 환경변수

`NPC_MIND_WORLD_DB` 그대로 (Phase 1 그대로).
- 추가 변수 없음 — npc-mind는 같은 프로세스 안에서 작동

### 6.5 라이브러리

기존 — Phase 1 D2·D3 원칙 계승.

### 6.6 외래키 활성 (Phase 1 → Phase 2 승급)

| 검증 | Phase 1 | Phase 2 |
|---|---|---|
| `Group.members.person_id` 존재 | 경고 | **에러** (활성) |
| `Group.parent_group` 존재 + cycle | 에러 (이미 활성) | 그대로 |
| `Group.allied_groups`·`rival_groups` 존재 | 경고 | 경고 (그대로 — 외부 도메인 아님) |
| `Person.affiliation` 존재 | (Phase 2 신규) | **에러** (활성) |
| `Person.birthplace`·`current_location` 존재 | (Phase 2 신규) | 경고 (Phase 3 활성) |
| `Group.headquarters` 존재 | 경고 | 경고 (Phase 3 활성) |

빌드 타임 검증이 강해지므로 Phase 1 시드 데이터에서 결손 발견 가능. 결손 시:
- Group 양식 수정 (members.person_id를 미정의 ID 만들지 말 것)
- 또는 Person 시드 추가

### 6.7 HEXACO 값 범위 — -1.0 ~ +1.0 확정 (기존 코드 정합)

**확정: -1.0 ~ +1.0 (양극)** — 사용자 확인(2026-05-01).

기존 `npc_mind` 구조 (사용자 명시):
- `src/domain/personality.rs` — `Score` Value Object가 -1.0 ~ +1.0 범위 강제. `1.0 + (Score * 0.3)` 같은 공식으로 감정 가중치 계산.
- `src/domain/pad.rs` — PAD 3 차원도 -1.0 ~ +1.0 (P 긍정·A 각성·D 지배). Phase 2 스코프 밖.
- OCC 감정 강도는 0.0 ~ 1.0, 1.0 클램핑. Phase 2 스코프 밖.

근거:
- 외부 리뷰(2026-05-01) 권장과 기존 코드가 일치 → 마이그레이션 불필요
- 양극 표현이 character 묘사에 자연스러움 (조고 "-0.8 권모술수")
- HEXACO 학술 표기(-2 ~ +2 표준화 점수)와 정합
- 본 TASK §6.1 npc-02 양식 예시 음수 값 그대로 유효

**Score VO 재사용 권장** (§3.5와 연동):
- `Person.hexaco` 6 필드를 `f32`로 두지 말고 `npc_mind::Score` VO 재사용 검토
- Personality·Score는 장르 중립 추상(HEXACO 학술 표준)이라 도메인 누수 아님
- 정합성 100%, 변환 함수 불필요, 범위 강제 자동
- 단 worldbuilding 모듈이 npc-mind에 의존하는 게 됨 — Cargo dependency 방향 한 번 검토 (`worldbuilding` 모듈은 같은 크레이트 안이라 자연스러움)
- 결정은 Claude Code Step 1에서 grep 후 — `Score::new` 인터페이스 무거우면 `f32` 직접 + 변환 함수도 OK

### 6.8 npc-mind 자동 등록 — idempotent + 보존

```rust
// pseudocode
fn upsert_to_mind(person: &Person, repo: &impl NpcRepository) -> Result<()> {
    if !matches!(person.kind.as_str(), "active" | "player") { return Ok(()); }

    let npc_id = NpcId::from(&person.id.0);
    let personality = HexacoSix → Personality 변환;

    if repo.exists(&npc_id) {
        // 갱신 — emotion_state·scene·memory 보존
        repo.update_personality(&npc_id, personality)?;
        repo.update_name(&npc_id, &person.name)?;
    } else {
        // 신규 — empty emotion_state, no scene
        repo.create(npc_id, person.name.clone(), personality)?;
    }
    Ok(())
}
```

핵심: **HEXACO·name만 갱신, 동적 상태(emotion·scene·memory)는 보존**. 게임 진행 중에 .md 편집·재로드해도 진행 상태 안 잃음.

## 7. Out of Scope (Phase 2)

- Place 도메인 — Phase 3
- Atlas 도메인 — Phase 4
- Item·Skill·Knowledge·Lore — Phase 5+
- Person 간 관계 그래프 (FamilyTree·MentorChain) — Phase 5+ 관계 도메인
- HEXACO 24 facet 정형 검증 — Phase 1엔 6 dim만, 24 facet은 자유 형식 보존
- Player Character 빌드 시스템 (skill·class·level) — Phase N(게임 플레이)
- 폼 시스템·AI 협업 빈칸 채움 — Phase N+
- aliases·hexaco_facets 다국어 i18n — Phase 5+
- mind upsert의 Event 발행 — Phase 1 EventBus 패턴 가능하나 Phase 2엔 직접 호출만
- 화산파 멸문 진실·혈교 묵계 같은 비밀 정보의 권한 제어 (특정 NPC만 알도록) — Phase 5+
- **Person.frontmatter의 default PAD 무드** — Phase 2 미포함. mind 시스템이 default(0,0,0) 또는 HEXACO 기반 자동 추론. 작가가 시작 무드를 명시하고 싶은 케이스(예: 조고 "차가운 계산")는 Phase 5+ Era 결합 시점에 자연 추가
- **Person.frontmatter의 default 감정 (Joy·Anger 등 OCC)** — 같은 이유로 Phase 2 미포함. 동적 상태는 mind 시스템 책임

## 8. 코드 위치 가이드

작업 시작 5분에 읽을 곳:

| 위치 | 무엇을 볼지 |
|---|---|
| `Cargo.toml` `[features]` | feature 게이팅 |
| `src/domain/world/group.rs` (Phase 1) | Person 미러링할 핵심 패턴 |
| `src/worldbuilding/markdown/group.rs` (Phase 1) | 마크다운 파서 패턴 |
| `src/adapter/sqlite_world.rs` (Phase 1) | SQLite + FTS + migrate_v2 자리 |
| `src/bin/world_load.rs` (Phase 1) | CLI 확장 패턴 |
| `src/bin/mind-studio/handlers/world_groups.rs` (Phase 1) | MCP 도구 등록 |
| `src/domain/character/...` (기존 npc-mind) | `Personality` / `Npc` 구조 — HEXACO 매핑 결정 |
| `src/ports.rs` | `NpcRepository` — Person upsert 인터페이스 |
| `src/application/...` | 기존 mind 서비스 — Person upsert 호출 위치 |

## 9. 시작 체크리스트

1. `CLAUDE.md` + Phase 0·1 산출 (`src/lore/`·`src/domain/world/`·`src/worldbuilding/`) 빠르게 훑기
2. `task-phase1-group-vertical-slice.md` + `phase1-implementation-report.md` 통독
3. **`wuxia-core/docs/characters/character-roster.md` v1.1 + `npc-02.md` 통독** — Phase 2 입력의 형태 파악
4. **기존 `npc_mind::Personality`·`Npc` 구조 확인** — HEXACO 매핑·NpcRepository upsert 인터페이스 파악
5. Person 도메인 + 마크다운 파서 + 단위 테스트
6. SqliteWorldStore migrate_v2 + persons 테이블 + 라운드트립 테스트
7. world-load 확장 — `world/person/*.md` 스캔 + npc-mind upsert
8. **조고 단독 변환 + dialogue_start 검증** → ★체크포인트 1★ 보고 → **commit pause**

## 10. 리뷰 채널 — 게이트 강제 적용

체크포인트 1 후 **반드시 commit 중단**, 보고서 → Cowork 복붙 → 리뷰 → 통과 신호 받고 Step 4 진행.

보고서 형식:
- **Done** · **Diff** · **데모 명령** · **결정** · **막힌 것** · **다음 의견**
- HEXACO 값 결정 근거·NPC mind upsert 시 발견한 정합성 이슈를 본문에 상세히 명시

보고서 파일명:
- 체크포인트 1: `docs/tasks/phase2-checkpoint1-report.md`
- 체크포인트 2: `docs/tasks/phase2-checkpoint2-report.md`
