# Phase 1: Group 카테고리 Vertical Slice

> **For Claude Code.** 이 문서는 자급 자족이며 외부 링크 없이도 작업 시작 가능.
> 결정 사항을 임의 변경하지 말 것. 변경이 필요하면 보고서에서 디렉터 승인 요청.

## 1. 목표

장르 중립 Worldbuilding 도구의 **첫 인스턴스 도메인 = Group**을 끝까지 한 사이클 완성.

`인터뷰/마크다운(SoT) → DDD 도메인 → 인프라 → MCP 노출` 파이프라인이 Group 한정으로 돌아가는지를 검증. 다른 도메인(Person·Place·Item·Skill·Knowledge·Lore·Event·Era·Atlas)은 Phase 2+.

**왜 Group이 첫 카테고리인가** (이전 Phase 1=Place에서 변경, 2026-04-30 결정):
1. **추상 누수 방지** — Place 먼저 짜면 Group 책임(통치체·강호 결사·세가)이 Place의 kind·extras로 흘러들어옴
2. **게임 서사 비중** — 사용자 비전 "인물·집단 중심"에 부합. 무협의 동력은 Group·Person 관계망
3. **시드 자료 풍부** — 칠국 통치 집단 + 구파일방 + 천마신교 + 십상시 등 wuxia-core/docs에 풍부

**검증 게이트**: `wuxia-core/docs/world/` + `characters/character-roster.md` + `npc-01~11.md`에서 Group 5-6개 변환 → SQLite 적재 → MCP `list_groups`·`get_group` 호출 결과로 모두 정확히 반환. 핵심 추상 검증: 시간성(founded/dissolved/status), 멤버십(텍스트 보존), 외래키(parent_group·headquarters 텍스트), 양식 분기(통치 vs 결사).

## 2. 연관 컨텍스트

- `CLAUDE.md` 프로젝트 루트
- `docs/tasks/task-phase0-lore-rag-bootstrap.md` — Phase 0 종료, Lore RAG MCP 가동 중
- `docs/tasks/task-phase3-place-vertical-slice.md` — 원래 Phase 1, Phase 3로 연기 (참고용)
- 메모리(Cowork 세션 보유): 9 인스턴스 도메인 + 1 관계 도메인(Atlas), 작업 순서 Group → Person → Place → Atlas, SoT는 마크다운
- 입력 자료:
  - `wuxia-core/docs/world/seven-nations.md` — 칠국 v1.1 (대진 §1 등에 통치 구조)
  - `wuxia-core/docs/world/history-characters.md` — 역사 인물·문파 배치 v1.2
  - `wuxia-core/docs/characters/character-roster.md` — 인물 총람 v1.1 (소속·문파)
  - `wuxia-core/docs/characters/npc-01~11.md` — NPC 11명 (소속 정보)

## 3. 제약

### 3.1 장르 중립 vs 의존

| 위치 | 책임 |
|---|---|
| `src/domain/world/group.rs` | **장르 영원히 모름** — id·name·aliases·kind(String)·summary·tags·extras·body_sections·temporal·members(텍스트)·headquarters(텍스트)·parent_group·**allied_groups**·**rival_groups** |
| `src/worldbuilding/markdown/group.rs` | 장르 중립 frontmatter+섹션 파서 |
| `src/adapter/sqlite_world.rs` | `groups` 테이블 + FTS5 |
| `genres/wuxia/forms/group.toml` | 무협 kind 옵션 (Phase 2 폼 미사용, 자리만) |
| `genres/wuxia/markdown_template/group.md` | 무협 .md 작성 템플릿 |
| `projects/chilguk-chunchu/world/group/*.md` | 칠국춘추 프로젝트 인스턴스 |

**`src/`에 wuxia 단어가 들어가면 안 됨.** 황실·문파·강호·결사·세가·십상시 같은 어휘는 모두 `genres/wuxia/`·`projects/chilguk-chunchu/`에만.

### 3.2 외래키는 텍스트 보존만

Person·Place 도메인이 없으니(Phase 2·3) 다음 외래키들은 **텍스트로만 보존**:
- `members` (Person ID 배열) — 예: `["npc-02", "npc-07"]`
- `headquarters` (Place ID) — 예: `"place-daejin-luoyang"` (아직 정의 안 된 ID)
- `parent_group` (Group ID) — 예: 십상시의 parent_group = `"group-daejin-court"`

**검증은 빌드 타임 경고만**(에러 아님). Phase 2(Person)·Phase 3(Place)에서 정식 외래키 활성. 단 **`parent_group` cycle 검증**은 Phase 1부터 — 같은 도메인 내 cycle은 즉시 검출 가능.

### 3.3 시간성 (Temporal) 일급

Group의 핵심 차별점은 시간성 — 형성·전성기·해체. Phase 1엔 자유 텍스트로 보존, Phase 5(Era 결합) 정형화:

```rust
pub struct Temporal {
    pub founded_at: Option<String>,      // "270년 전" "현재 황조 즉위 시" "원년" 등
    pub dissolved_at: Option<String>,    // 해체된 경우
    pub status: GroupStatus,             // Active | Declining | Dissolved | Dormant
    pub notes: Option<String>,           // 시기별 변동 자유 메모
}
```

### 3.4 SoT = 마크다운

기존 흐름과 동일. `projects/chilguk-chunchu/world/group/*.md` SoT, SQLite는 빌드 산출물(.gitignore).

### 3.5 검색 범위

FTS5 trigram만. 의미 검색 임베딩은 Phase 2+.

## 4. Done Criteria

- [ ] 디렉토리 골격: `src/domain/world/`, `src/worldbuilding/`, `genres/wuxia/`, `projects/chilguk-chunchu/`
- [ ] `Group` 애그리거트 + `Temporal` + `MemberRef` + `WorldRepository` 트레잇 + 단위 테스트
- [ ] 마크다운 frontmatter+섹션 파서 + 단위 테스트
- [ ] `genres/wuxia/markdown_template/group.md` 템플릿 (§6.1)
- [ ] `genres/wuxia/forms/group.toml` 자리 (Phase 2 빈 슬롯)
- [ ] `SqliteWorldStore` (`groups` 테이블 + FTS5) + 라운드트립 테스트
- [ ] `bin/world-load` CLI: 마크다운 일괄 로드 → SQLite 빌드
- [ ] `bin/mind-studio` MCP 도구 3개: `list_groups`, `get_group`, `search_groups`
- [ ] **체크포인트 1**: 대진 황실(`group-daejin-court`) 단일 변환 라운드트립
- [ ] **체크포인트 2**: 5-6개 Group 변환 + MCP 정성 평가
- [ ] `cargo build` + `cargo test --features embed` 통과
- [ ] 정성 검증: `list_groups(kind="alliance")` → 무림맹 등장. parent_group 검증 → 십상시 → 대진 황실 cycle 없음.

## 5. 단계별 작업

### Step 1 — 디렉토리 골격 + 도메인

```
src/domain/world/
├── mod.rs          # pub use, 9 인스턴스 도메인 자리 + Atlas는 Phase 4 별도
├── group.rs        # Group + GroupId + Temporal + MemberRef + GroupStatus
├── place.rs, person.rs, item.rs, skill.rs, knowledge.rs, lore.rs, event.rs, era.rs   # 빈

src/worldbuilding/
├── mod.rs
├── markdown/
│   ├── mod.rs
│   ├── frontmatter.rs    # YAML frontmatter 파서
│   └── group.rs          # Group .md → 도메인
├── repository.rs         # WorldRepository 트레잇 (groups 메서드 한정)
└── builder.rs            # 빈 (Phase 2: 폼 시스템 진입점)

src/adapter/
└── sqlite_world.rs       # SqliteWorldStore — groups + FTS5

src/bin/
└── world_load.rs         # CLI

genres/wuxia/
├── genre.toml
├── forms/group.toml      # Phase 2 자리만
└── markdown_template/
    └── group.md

projects/chilguk-chunchu/
├── project.toml          # genre = "wuxia", title, description
└── world/
    └── group/            # 5-6개 .md (Step 3·4)
```

#### `Group` 애그리거트

```rust
// src/domain/world/group.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GroupId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupStatus {
    Active,
    Declining,
    Dissolved,
    Dormant,        // 잠적·잠복
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Temporal {
    pub founded_at: Option<String>,        // 자유 텍스트, Phase 5 Era 결합 시 정형
    pub dissolved_at: Option<String>,
    pub status: GroupStatus,                // 기본 Active
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberRef {
    pub person_id: Option<String>,          // Phase 1엔 텍스트("npc-02"), Phase 2 외래키
    pub display_name: Option<String>,       // person_id 없을 때 표시명
    pub role: String,                       // "수장"·"이인자"·"문도"·"외부 협력자"
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: GroupId,
    pub kind: String,                       // 장르가 채움. Phase 1 wuxia: dynasty-court | clan | sect-religious | mendicant-order | alliance | covert-band | tribe-confederacy | merchants-council
    pub name: String,
    pub aliases: Vec<String>,               // 별호·옛 이름 (예: 무림맹 ↔ 구파일방)
    pub summary: String,
    pub tags: Vec<String>,
    pub extras: serde_json::Map<String, Value>,
    pub body_sections: BTreeMap<String, String>,
    pub temporal: Temporal,
    pub members: Vec<MemberRef>,
    pub headquarters: Option<String>,       // Place ID 텍스트 (Phase 3 외래키)
    pub parent_group: Option<GroupId>,      // 수직 포함 (예: 십상시 → 대진 황실)
    pub allied_groups: Vec<GroupId>,        // 수평 — 공인된 우호·동맹
    pub rival_groups: Vec<GroupId>,         // 수평 — 공인된 적대·경쟁
    pub source_path: Option<String>,
}
```

#### `WorldRepository` 트레잇 (Phase 1엔 groups만)

```rust
#[async_trait]
pub trait WorldRepository: Send + Sync {
    async fn list_groups(&self, filter: GroupFilter) -> Result<Vec<Group>, WorldError>;
    async fn get_group(&self, id: &GroupId) -> Result<Option<Group>, WorldError>;
    async fn search_groups(&self, query: &str, top_k: u32) -> Result<Vec<Group>, WorldError>;
}

#[derive(Debug, Clone, Default)]
pub struct GroupFilter {
    pub kind: Option<String>,
    pub status: Option<GroupStatus>,
    pub parent_group: Option<GroupId>,
    pub genre_tag: Option<String>,
}
```

산출물 검증: `cargo build` 통과. 단위 테스트 — Group 인스턴스 생성, Temporal 직렬화/역직렬화, parent_group cycle 검출 함수.

### Step 2 — 마크다운 파이프라인

#### Frontmatter 양식 (§6.1 참조)

`serde_yaml` 추가(이미 있으면 재사용). gray_matter 같은 외부 크레이트 안 씀(Phase 0 D2 원칙).

#### CLI: `world-load`

```
cargo run --features embed --bin world-load -- --project chilguk-chunchu [--reload]
```

동작:
1. `projects/chilguk-chunchu/project.toml` 로드 → genre 확인
2. `world/group/*.md` 순회 → `Group` 객체 → `groups` 테이블 upsert
3. **`parent_group` cycle 검증** — DFS로 자기 도달 시 경고
4. `members.person_id`·`headquarters`·`parent_group` 외래키 존재 검증 (Phase 1엔 경고만 — 외부 도메인 ID는 자연스럽게 결손)
5. 진행률·결과 stdout

산출물 검증: 빈 .md 라운드트립 테스트.

### Step 3 — 대진 황실 단일 변환 시연 ★체크포인트 1★

대상: **대진 황실** (`group-daejin-court`). 시드 자료가 가장 풍부 — `seven-nations.md` §1.2 통치 구조 + `character-roster.md` 천순제·조고·십상시 정보 + `history-characters.md` 황조 계보.

작업:
1. `seven-nations.md` §1 + `character-roster.md` 02·07 항목 + `history-characters.md` 대진 부분 종합 → `projects/chilguk-chunchu/world/group/group-daejin-court.md`
2. `cargo run --features embed --bin world-load -- --project chilguk-chunchu`
3. `Group` 도메인 객체 라운드트립 검증 — 모든 필드 보존
4. `members`에 `npc-07(천순제, 명목 황제)`·`npc-02(조고, 실권자)` 텍스트 보존
5. `headquarters: "place-daejin-luoyang"` 텍스트만 (place 없음)
6. `temporal.founded_at: "원년 (270년 전)"`, `status: Declining` (축소 제국)

**체크포인트 1 보고서**:
- `git diff --stat`
- `group-daejin-court.md` 전문 (변환 결과 첨부)
- 로드 후 `Group` 도메인 객체의 모든 필드 dump (JSON)
- SQLite 사이즈 + groups 행 수
- **변환 시 결정한 것**:
  - 산문→섹션 마커 매핑 (§1.2 통치 구조·§1.5 핵심 갈등 같은 부분이 어느 섹션으로?)
  - frontmatter `extras` 키 선택 (실권자·명목 원수·세금 정책 등)
  - `kind` 선택 — `dynasty-court`이 적절한지 다른 후보(예: `imperial-court`)와 비교
  - **aliases** 후보 (예: "낙양 조정"·"중원 황실"·"십상시 정권")
  - **temporal.notes**에 270년간 변동 어떻게 기술했는지
  - members 역할 라벨 — "수장"·"이인자"·"꼭두각시 황제" 같은 표현 일관성
- **십상시 분리는 확정** (2026-04-30 리뷰): 별도 Group `group-shipsangsi`, `parent_group = group-daejin-court`. 멤버로 처리 X. 근거 — 십상시는 독자 조직망·재정·근거지를 가진 sub-organization. 체크포인트 1 보고엔 분리 결정 의문 X.
- 단 **대진 황실의 `allied_groups`·`rival_groups`** 후보를 보고서에 명시 (예: rival에 군림 시도 세력? allied에 명목상 우호 문파?)
- Step 4·5 진행 가능 여부 의견

→ Cowork 리뷰 → 통과 시 다음 단계.

### Step 4 — 5-6개 Group 일괄 변환

체크포인트 1 통과 후 나머지 4-5개:

| Group ID | kind | 시드 자료 출처 | 비고 |
|---|---|---|---|
| `group-namgung` | clan | seven-nations §2 + npc-03 (남궁혁) | 가문 |
| `group-mulim-mang` | alliance | history-characters §1·구파일방 | 무림맹 (270년 역사) |
| `group-shipsangsi` | covert-band | character-roster §3 + npc-02 조고 | 십상시. **parent_group = group-daejin-court** (수직 포함 시연) |
| `group-cheonma-shingyo` | sect-religious | character-roster 10번 + npc-06 (야율설화) | 천마신교, 사파 |
| `group-gaebang` | mendicant-order | npc-05 (소연) + npc-11 (소풍자) | 개방. 옵션 — 6개로 만들 때 |

`headquarters`·`members`는 모두 텍스트 보존. parent_group의 cycle 없는지 빌드 타임 검증 ✓.

**수평 관계 시연 (필수 박음)** — 5-6 Group 변환 시 다음 관계가 데이터로 등장:
- `group-mulim-mang.rival_groups` ← `group-cheonma-shingyo` (정파 vs 사파)
- `group-cheonma-shingyo.rival_groups` ← `group-mulim-mang` (대칭)
- `group-mulim-mang.allied_groups` ← `group-namgung`·`group-gaebang` (구파일방 + 보조)
- `group-shipsangsi.rival_groups` ← (대진 외부의 적대 세력 후보)

**진영(alignment) extras 시연** — wuxia 표준화 채택. 각 Group의 extras에:
- 무림맹·남궁·개방 → `alignment: orthodox` (정파)
- 천마신교 → `alignment: heterodox` 또는 `demonic` (사파/마교)
- 대진 황실·십상시 → `alignment: imperial` (황실)
- 북원 등 → `alignment: outland` (새외)

### Step 5 — MCP 도구 3개 + 정성 검증 ★체크포인트 2★

```
list_groups(filter: GroupFilter) -> Vec<GroupSummary>
  GroupFilter { kind?, status?, parent_group?, genre_tag? }
  GroupSummary { id, name, kind, status, summary_one_line, tags }

get_group(group_id: String) -> Option<GroupDetail>
  GroupDetail = full Group

search_groups(query: String, top_k: u32 = 5) -> Vec<GroupSummary>
  FTS5 trigram (name + aliases + summary + body)
```

`AppState`에 embed-gated `world_store: Option<Arc<dyn WorldRepository>>` 추가. `NPC_MIND_WORLD_DB` 부재 시 graceful skip(Phase 0 lore_store 패턴).

**체크포인트 2 보고서**:
- `list_groups()` 결과 (5-6 Group — id·name·kind·status·1줄 요약)
- `list_groups(kind="alliance")` → 무림맹
- `list_groups(kind="clan")` → 남궁가
- `list_groups(parent_group="group-daejin-court")` → 십상시 (수직 포함 시연)
- `list_groups(alignment="orthodox")` → 무림맹·남궁·개방 (alignment extras 필터 시연 — wuxia 장르 패키지 룩업)
- `get_group("group-daejin-court")` 전체 detail
- `get_group("group-shipsangsi")` 전체 detail (parent_group 외래키 검증)
- `get_group("group-mulim-mang")` — `rival_groups`에 `group-cheonma-shingyo` 들어 있는지 (수평 관계 시연)
- `search_groups` 6쿼리:
  - "검왕" → 남궁가 (멤버 별호 매칭 — body에 들어 있음)
  - "꼭두각시" → 대진 황실 (extras·body 매칭)
  - "구파일방" → 무림맹 (alias 매칭)
  - "사파" → 천마신교
  - "거지" → 개방 (옵션)
  - "암살" 또는 "첩보" → 십상시
- 정성 평가: 시간성·멤버십·parent_group 분기가 손실 없이 보존됐는가
- `world-load`의 외래키 결손 경고 출력 — 어떤 ID들이 미정의 상태로 보고됐는가
- Phase 2(Person) 진입 가능 여부 의견

→ Cowork 리뷰 → 통과 시 Phase 1 종료.

## 6. 결정 사항 (변경 시 디렉터 승인)

### 6.1 Frontmatter·섹션 약속 (장르 중립)

#### 공통

필수: `id`, `kind`, `name`. 권장: `aliases`, `summary`, `tags`, `temporal`. 선택: `extras`, `members`, `headquarters`, `parent_group`, `body_sections`.

`id` 형식: `group-{slug}`. slug = ASCII 소문자·숫자·하이픈. 한자는 한국어 발음 음역(예: `group-daejin-court`).

#### 양식 예시 — 대진 황실 (체크포인트 1 후보)

```yaml
---
id: group-daejin-court
kind: dynasty-court                          # 장르가 채움
name: 대진 황실
aliases: [낙양 조정, 중원 황실, 십상시 정권]
summary: |
  270년 전 통일제국 대진의 후예. 천순제는 꼭두각시이며 실권은
  환관 조고가 잡고 있다. 영토는 중원으로 축소됐으나 정통성 명분을 쥐고 있다.
tags: [wuxia, group, dynasty, declining-empire]
temporal:
  founded_at: 원년 (270년 전)
  dissolved_at: ~
  status: declining                          # Active | Declining | Dissolved | Dormant
  notes: |
    270년 전 통일제국으로 출발. 10년 전 "붉은 밤의 변" 이후 영토 와해.
    현재 천순제 즉위 후 조고 실권 장악.
members:
  - person_id: npc-07
    display_name: 천순제
    role: 황제 (꼭두각시)
    note: 명목 원수
  - person_id: npc-02
    display_name: 조고
    role: 실권자
    note: 환관, 십상시 수장
headquarters: place-daejin-luoyang            # Place ID 텍스트 — Phase 3 외래키
parent_group: ~                               # 최상위 통치체
allied_groups: []                             # 수평 우호 (빈 배열도 허용)
rival_groups: []                              # 수평 적대 — 대진 황실의 적대 후보를 보고에서 결정
extras:
  alignment: imperial                         # wuxia 진영 표준화 (orthodox|heterodox|demonic|outland|imperial|neutral)
  shadow_ruler: 조고
  formal_ruler: 천순제
  capital: 낙양(洛陽)
  sub_groups: [group-shipsangsi]              # 자식 group ID 텍스트 (역참조 자동 도출 가능)
  # 무협 특화 관계 (장르 extras — core 도메인은 모름):
  enmity_groups: []                           # 피의 원수 (rival보다 강한 무협 어휘)
  fellowship_groups: []                       # 사문·동문·결사 형제
---

## 개요
산문 — 황실의 현 상황 1-2 단락.

## 권력 구조
산문 — 천순제 ↔ 조고 ↔ 십상시 ↔ 황실 관료의 관계.

## 외부 갈등
산문 — 다른 6국·소림·무당·무림 세력과의 관계.

## 핵심 갈등
산문 — Group 자체의 내부 결단·운명적 모순.

## 시간 변화
산문 — temporal.notes를 더 자세히. 시기별 변동.

## 게임에서의 역할
산문 — Group이 게임 진행에 어떻게 등장하는가.
```

권장 H2 섹션: `## 개요` · `## 권력 구조` · `## 외부 갈등` · `## 핵심 갈등` · `## 시간 변화` · `## 게임에서의 역할`. 두 섹션(`## 개요` · `## 게임에서의 역할`)은 모든 Group 공통.

#### kind 별 양식 미세 조정

| kind | 권장 추가 섹션 | extras 권장 키 |
|---|---|---|
| `dynasty-court` | `## 권력 구조` | shadow_ruler·formal_ruler·capital |
| `clan` | `## 가풍·계승` | clan_head·lineage_rule·signature_skill |
| `sect-religious` | `## 교의·수련` | sect_master·doctrine·schools |
| `alliance` | `## 회원 구조` | leader_role·founding_members·voting_rule |
| `covert-band` | `## 활동 영역` | leader·specialty·cover_identity |
| `mendicant-order` | `## 규율·계급` | head_role·grades·signature_practice |
| `tribe-confederacy` | `## 부족 구성` | great_chief·tribes·council_rule |
| `merchants-council` | `## 의회 구조` | president·seat_count·voting_rule |

### 6.2 `genres/wuxia/forms/group.toml` (Phase 2 빈 슬롯)

```toml
extends = "group"

[[fields.kind.options]]
value = "dynasty-court"; label = "황실/조정"
[[fields.kind.options]]
value = "clan"; label = "세가/가문"
[[fields.kind.options]]
value = "sect-religious"; label = "신교/사파"
[[fields.kind.options]]
value = "alliance"; label = "동맹/연합체"
[[fields.kind.options]]
value = "covert-band"; label = "비밀 결사/사병"
[[fields.kind.options]]
value = "mendicant-order"; label = "개방/탁발 결사"
[[fields.kind.options]]
value = "tribe-confederacy"; label = "부족 연합"
[[fields.kind.options]]
value = "merchants-council"; label = "상방 의회"

# 무협 진영 표준화 — wuxia 장르 절대 좌표계
[[fields]]
key = "alignment"                             # extras에 들어가는 키
label = "진영"
required = true
type = "enum"
options = [
  { value = "orthodox",   label = "정파(正)" },
  { value = "heterodox",  label = "사파(邪)" },
  { value = "demonic",    label = "마교(魔)" },
  { value = "outland",    label = "새외(塞外)" },
  { value = "imperial",   label = "황실" },
  { value = "neutral",    label = "중립" },
]

# 무협 특화 수평 관계 (rival·allied 외 추가 결)
[[fields]]
key = "enmity_groups"                         # 피의 원수 (rival보다 강도 높음)
label = "원수"
type = "id_array"
target = "group"

[[fields]]
key = "fellowship_groups"                     # 사문·동문·결사 형제
label = "사문·동문"
type = "id_array"
target = "group"
```

### 6.3 SQLite 스키마

```sql
CREATE TABLE groups (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    aliases_json TEXT NOT NULL DEFAULT '[]',
    parent_group TEXT,                                -- 캐시 컬럼
    allied_groups_json TEXT NOT NULL DEFAULT '[]',    -- 수평 우호 ID 배열
    rival_groups_json TEXT NOT NULL DEFAULT '[]',     -- 수평 적대 ID 배열
    headquarters TEXT,                                -- Place ID 텍스트 — Phase 3 외래키
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','declining','dissolved','dormant')),
    alignment TEXT,                                   -- wuxia extras에서 추출한 캐시 (장르 미설정 시 NULL)
    summary TEXT NOT NULL DEFAULT '',
    tags_json TEXT NOT NULL DEFAULT '[]',
    extras_json TEXT NOT NULL DEFAULT '{}',
    body_sections_json TEXT NOT NULL DEFAULT '{}',
    temporal_json TEXT NOT NULL DEFAULT '{}',
    members_json TEXT NOT NULL DEFAULT '[]',
    source_path TEXT,
    updated_at INTEGER NOT NULL
);
CREATE INDEX idx_groups_kind ON groups(kind);
CREATE INDEX idx_groups_parent ON groups(parent_group);
CREATE INDEX idx_groups_status ON groups(status);
CREATE INDEX idx_groups_alignment ON groups(alignment);

CREATE VIRTUAL TABLE groups_fts USING fts5(
    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
);
```

`schema_meta` 마이그레이션 버전 관리 (Phase 0 패턴).

### 6.4 환경변수

`NPC_MIND_WORLD_DB` = `projects/chilguk-chunchu/build/world.sqlite` (없으면 graceful skip).

### 6.5 라이브러리

- TOML: 기존 재사용
- YAML: `serde_yaml` (없으면 추가)
- gray_matter·markdown 도입 금지 (Phase 0 D2 원칙)

### 6.6 외래키

Phase 1엔 다음 검증을 빌드 타임 경고만:
- `members.person_id` 존재 검증 (Phase 2 Person 도입 시 활성)
- `headquarters` Place ID 존재 검증 (Phase 3 Place 도입 시 활성)
- `parent_group` Group ID 존재 검증 (Phase 1부터 가능 — 같은 도메인 내)
- **`parent_group` cycle 검증** — DFS로 자기 도달 시 경고 (Phase 1부터 가능)
- `allied_groups` / `rival_groups` 각 ID 존재 검증 (Phase 1부터 가능 — 같은 도메인 내)
- `allied_groups` ↔ `rival_groups` **모순 검증** — 같은 ID가 둘 다에 등장 시 경고
- **rival 대칭성 경고** — A의 rival에 B가 있으면 B의 rival에도 A가 있는 게 자연스러움. 비대칭 시 경고만(에러 X — 일방적 적대도 무협엔 흔함)
- 무협 extras (`enmity_groups`·`fellowship_groups`) 검증은 wuxia 패키지가 책임 (선택)

## 7. Out of Scope (Phase 1)

- Person·Place·Item 등 다른 도메인 (각 Phase로)
- 외래키 정식 검증 (현 단계는 텍스트 보존 + 경고)
- 의미 검색(임베딩) — Phase 1엔 FTS5만
- 폼 시스템 (Phase 2)
- AI 협업 빈칸 채움 — Lore RAG 자동 호출 (Phase 2)
- Mind Studio worldbuilding UI 패널 (Phase 4+)
- WorldEvent 이벤트 발행 (Phase 5+)
- 다중 프로젝트 관리 (Phase 2+)
- aliases 다국어 i18n (Phase 2+)
- Group 간 정형 관계 모델 — `OrgChart` 관계 도메인. **단순 ID 참조(`allied_groups`·`rival_groups`)는 Phase 1에 들어감**. 강도·시작 시기·근거 사건 같은 정밀 관계는 Phase 5+ OrgChart로.
- temporal의 정형 시간 (Era 결합) — Phase 5+

## 8. 코드 위치 가이드

작업 시작 5분에 읽을 곳:

| 위치 | 무엇을 볼지 |
|---|---|
| `Cargo.toml` `[features]` | feature 게이팅 패턴 |
| `src/domain/{character, scene, memory}` | 기존 도메인 애그리거트 패턴 |
| `src/ports.rs` | `MindRepository` — `WorldRepository` 만들 때 참고 |
| `src/adapter/sqlite_*` | `SqliteMemoryStore`·`SqliteLoreStore` (Phase 0) — FTS5 trigram + 마이그레이션 |
| `src/lore/store.rs` | 가장 최근 패턴, 그대로 미러링 |
| `src/bin/mind-studio/handlers/` | MCP 도구 등록 패턴 |
| `src/bin/mind-studio/state.rs` | AppState — `lore_store` 옆 `world_store` 추가 |
| `src/bin/mind-studio/main.rs` | `NPC_MIND_LORE_DB` 부착 패턴 미러 |

## 9. 시작 체크리스트

1. `CLAUDE.md` + Phase 0 산출 (`src/lore/`·`bin/lore_ingest.rs`·`mind-studio/mcp_server.rs`) 빠르게 훑기
2. `task-phase3-place-vertical-slice.md` 한 번 훑기 — Phase 3로 미뤄진 Place의 양식이 Group 양식을 짤 때 참고가 됨 (특히 §6.1 frontmatter·`spatial`·외래키 패턴)
3. `wuxia-core/docs/world/seven-nations.md` §1 (대진), `world/history-characters.md` 통독
4. `wuxia-core/docs/characters/character-roster.md` v1.1 통독
5. 디렉토리 골격 신설 (Step 1) — 8 도메인은 빈 스켈레톤만, Atlas는 Phase 4
6. `Group` + `Temporal` + `MemberRef` + `WorldRepository` + 마크다운 파서 + 단위 테스트
7. SqliteWorldStore + 라운드트립 테스트
8. **대진 황실 변환** → ★체크포인트 1★ 보고

## 10. 리뷰 채널

체크포인트 1·2 보고서를 디렉터(사용자)가 Cowork 세션에 복붙. 형식:
- **Done** · **Diff** · **데모 명령** · **결정** · **막힌 것** · **다음 의견**
- 변환 시 모든 추론·결정(특히 십상시 분리·kind 명명·temporal 표현)을 본문에 상세히 명시

보고서 파일명:
- 체크포인트 1: `docs/tasks/phase1-checkpoint1-report.md`
- 체크포인트 2: `docs/tasks/phase1-checkpoint2-report.md`
