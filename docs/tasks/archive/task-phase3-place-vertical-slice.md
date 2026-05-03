# Phase 3: Place 카테고리 Vertical Slice (정적 시드 마지막 인스턴스 도메인)

> **For Claude Code.** 이 문서는 자급 자족이며 외부 링크 없이도 작업 시작 가능.
> 결정 사항을 임의 변경하지 말 것. 변경이 필요하면 보고서에서 디렉터 승인 요청.
> **선행 조건**: Phase 0 (Lore RAG) + Phase 1 (Group) + Phase 2 (Person + Player + Runtime sync) 모두 종결.
> **체크포인트 분리 게이트**: Phase 1 미준수 후속으로 Phase 2부터 강제 적용 중. **Phase 3에서도 강제 — 1회 통합 commit 금지.**

---

## 1. 목표

장르 중립 Worldbuilding 도구의 **세 번째 인스턴스 도메인 = Place**를 끝까지 한 사이클. **정적 시드(Phase 0~3)의 마지막 인스턴스 카테고리** — Phase 4 Atlas는 별도 분리, Phase 5+는 두 결의 다리(gameplay 통합).

이번 Phase가 다루는 추상은 **세 결**:
1. **Place 도메인** — Settlement·Geography 두 layer + spatial(parent_place·bordering·geography_refs) + aliases
2. **Phase 1·2 외래키 활성** — `Group.headquarters`·`Person.birthplace`·`Person.current_location` 검증을 텍스트 → 정식 외래키로 승급
3. **Group의 sect kind ↔ Place의 sect kind 이중 등록 패턴** — 무협 sect는 Place(공간 인스턴스) + Group(조직체) 양쪽에 등록, 외래키로 연결

**Atlas는 Phase 4로 분리**. Phase 3 종결 후 별도 `task-phase4-atlas-vertical-slice.md` 작성. Atlas는 도메인+뷰의 이중성을 가진 관계 도메인이라 Phase 3와 결이 다름.

**검증 게이트**: `wuxia-core/docs/world/seven-nations.md`(칠국 v1.1, 1076줄) → 우리 형식 마크다운으로 변환:
- 7국 정치체 → `place-*.md` × 7 (layer = settlement)
- 자연 지형 1-2 → `place-*.md` (layer = geography)
- world-load 후 **Phase 1·2 외래키 활성** — 모든 headquarters·birthplace·current_location ID가 places에 존재해야 통과 (현재 13건+α 텍스트 보존 상태)
- 정성 평가: `list_places(layer=settlement)` → 7건 / `list_places(layer=geography)` → 1-2건 / `search_places("산악")` → 매칭

## 2. 연관 컨텍스트

- `CLAUDE.md` 프로젝트 루트
- `docs/tasks/00-roadmap.md` — 전체 Phase 흐름·작업 순서·결정 로그
- `docs/tasks/task-phase1-group-vertical-slice.md` + `phase1-implementation-report.md`
- `docs/tasks/task-phase2-person-vertical-slice.md` + `phase2-checkpoint{1,2}-report.md` + 두 follow-up 보고서
- `docs/tasks/task-phase3-place-vertical-slice.archive.md` — 원래 Phase 1로 작성된 보존본. **참고용** (양식·SQLite·외래키 패턴 거의 그대로 활용 가능. Atlas 부분은 Phase 4로 분리되니 본 TASK엔 미포함)
- 메모리(Cowork 세션 보유): 도구 추상화 두 결, 9 인스턴스 도메인 + 1 관계 도메인(Atlas), Place layer 분화, 작업 순서, Score VO 재사용 패턴
- 입력 자료:
  - `wuxia-core/docs/world/seven-nations.md` — 칠국 v1.1 (1076줄)
  - 같은 디렉토리 `history.md`·`history-characters.md`·`character-naming.md`는 Phase 5+ 입력 (이번엔 손대지 않음)
  - Phase 1·2 산출 — `projects/chilguk-chunchu/world/{group,person}/*.md` 안의 `headquarters`·`birthplace`·`current_location` 텍스트 ID 참조

## 3. 제약

### 3.1 장르 중립 vs 의존

| 위치 | 책임 |
|---|---|
| `src/domain/world/place.rs` | **장르 영원히 모름** — id·name·**aliases**·layer(Settlement\|Geography 일급 enum)·kind(String)·summary·tags·extras·body_sections·**spatial(parent_place·bordering_places·geography_refs)** |
| `src/worldbuilding/markdown/place.rs` | 장르 중립 frontmatter+섹션 파서 |
| `src/adapter/sqlite_world.rs` (확장) | `places` 테이블 + FTS5 + `migrate_v3` |
| `genres/wuxia/forms/place.toml` | 무협 kind 옵션·확장 필드 (Phase N 폼 활성 시 사용) |
| `genres/wuxia/markdown_template/place-{settlement,geography}.md` | 무협 .md 작성 템플릿 (두 layer별) |
| `projects/chilguk-chunchu/world/place/*.md` | 칠국춘추 인스턴스 |

**`src/`에 wuxia 단어 X.** 정체(政體)·기·내공·문파·강호 같은 어휘는 `genres/wuxia/`·`projects/chilguk-chunchu/`에만.

### 3.2 PlaceLayer — 일급 enum

```rust
// src/domain/world/place.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaceLayer {
    /// 공동체 관리 공간 — 국가·도시·문파·자치령. 인간 시간 단위로 변동.
    Settlement,
    /// 자연 지형 — 산악·해안·밀림·초원·사막. 지질학적 시간 단위, 거의 안 변함.
    Geography,
}
```

같은 좌표 위에 두 결이 포개짐. `Settlement`가 자기 위치한 `Geography`를 `spatial.geography_refs`로 참조. Era overlay(Phase 5+)의 기반.

### 3.3 Phase 1·2 외래키 활성 — Phase 3의 핵심 검증 게이트

| 외래키 | Phase 1·2 상태 | Phase 3 활성 후 |
|---|---|---|
| `Group.headquarters` (Place ID) | 텍스트 보존, 경고만 | **Place ID 존재 검증 — 결손 시 에러** |
| `Person.birthplace` (Place ID) | 텍스트 보존, 경고만 | **활성 — 에러** |
| `Person.current_location` (Place ID) | 텍스트 보존, 경고만 | **활성 — 에러** |
| `Place.spatial.bordering_places` | (Phase 3 신규) | 활성 — 같은 도메인 내라 검증 가능 |
| `Place.spatial.geography_refs` (settlement → geography) | (Phase 3 신규) | 활성 |
| `Place.spatial.parent_place` | (Phase 3 신규) | 활성 + **cycle 검증** |
| `Place.controlling_group` (sect kind, optional) | (Phase 3 신규) | 활성 — Phase 1 Group ID 검증 |

**현재 Phase 1·2 시드 데이터의 Place ID 텍스트 참조** (Phase 3 진입 시 검증 활성될 ID들):
- `place-daejin-luoyang` — group-daejin-court·npc-02·npc-07 등 다수 참조
- `place-namgung-geomseong` — group-namgung·npc-03 참조
- `place-free-cities` (자유도시) — npc-05·player 참조
- `place-east-coast` — player.birthplace
- `place-free-cities-back-alleys` — player.starting_location
- 기타 birthplace·current_location 미상(`~`) 8건

**검증 결과 예측**:
- 7국 settlement만 변환 시 일부 결손 — 자유도시 세부(`back-alleys`)·동해 연안 같은 지역 단위는 Phase 5+
- 결손 처리: Phase 3 양식에 충분한 sub-place 만들거나 Phase 1·2 시드 ID를 더 추상화된 ID로 단순화 (예: `place-east-coast` → `place-donghae`)

이 영향이 Phase 3 진입 시점의 첫 큰 결정 — 사용자 결정 필요. 체크포인트 1 보고서에 발견 사항 명시 + 디렉터 승인 절차 거침.

### 3.4 sect kind 이중 등록 — Place + Group

무협 sect는 두 카테고리에 동시 등장 (Phase 1 결정 — 메모리 박힘):
- `Place / kind=sect` — 영토·건물 (공간 인스턴스). `parent_place`로 영토 소속, `extras.controlling_group`으로 Group 외래키
- `Group / kind=clan|sect-religious` — 사람들의 결속 (조직). `headquarters`로 Place 참조

Phase 3 시드 데이터엔 sect Place 1-2개만 시연:
- `place-namgung-sega` (남궁세가 본부 — 검성 외곽). `parent_place: place-namgung`(국가). `extras.controlling_group: group-namgung`
- 또는 `place-aimi-pa` (아미파 본거지). `parent_place: place-mt-aimi`(자연 지형)

체크포인트 2에서 1-2개 시연 — Phase 5+ Group과의 관계 메커니즘 검증.

### 3.5 SoT = 마크다운

기존 흐름 동일. SQLite는 빌드 산출물(.gitignore).

### 3.6 검색 범위

FTS5 trigram + 2-char LIKE fallback (Phase 1 D5 패턴 그대로). 의미 검색은 Phase 5+.

### 3.7 체크포인트 분리 게이트 — 강제 적용

Phase 2 흐름 그대로:
1. **체크포인트 1**: 대진(settlement) + 서부 산악(geography) 두 layer 변환 → world-load 외래키 활성 검증 → commit pause → 보고서 #1 → Cowork 리뷰 → 통과
2. **체크포인트 2**: 6 settlement 추가 + 자연 1-2 + sect 1-2 + MCP 정성 → commit pause → 보고서 #2 → Cowork 리뷰 → Phase 3 종결

**1회 통합 commit 금지.**

## 4. Done Criteria

- [ ] 디렉토리 골격: `src/domain/world/place.rs` (기존 stub 채움), `src/worldbuilding/markdown/place.rs`
- [ ] `Place` 애그리거트 + `PlaceLayer` enum + `Spatial` + `WorldRepository` 트레잇 확장 + 단위 테스트
- [ ] 마크다운 frontmatter+섹션 파서 (Settlement·Geography 두 양식) + 단위 테스트
- [ ] `genres/wuxia/markdown_template/{place-settlement,place-geography}.md` 템플릿
- [ ] `genres/wuxia/forms/place.toml` 자리 (Phase N 빈 슬롯)
- [ ] `SqliteWorldStore` 확장 — `places` 테이블 + `places_fts` + `place_atlas_refs` 자리(Phase 4) + `migrate_v3`
- [ ] `bin/world-load` 확장 — `world/place/*.md` 스캔 + Phase 1·2 외래키 활성 (에러 승급)
- [ ] `bin/mind-studio` MCP 도구 3개: `list_places` · `get_place` · `search_places` (filter 포함)
- [ ] **체크포인트 1**: 대진(settlement) + 서부 산악(geography) 두 layer 라운드트립 + Phase 1·2 외래키 active 시연
- [ ] **체크포인트 2**: 6 settlement 추가 + 자연 1-2 + sect 1-2 + MCP 정성 평가 + 외래키 결손 0건
- [ ] `cargo build` + `cargo test --features embed` + 기존 e2e 회귀 통과
- [ ] 정성 검증: `list_places(layer="settlement")` → 7-8건 / `list_places(layer="geography")` → 1-2건 / `search_places("산악")` → 매칭

## 5. 단계별 작업

### Step 1 — 디렉토리 골격 + 도메인

```
src/domain/world/
├── place.rs        # Place + PlaceId + PlaceLayer + Spatial (기존 stub 채움)
├── group.rs, person.rs   # Phase 1·2 그대로
└── ...

src/worldbuilding/
├── markdown/
│   ├── place.rs    # Place .md → 도메인 (신규)
│   └── group.rs, person.rs, frontmatter.rs   # 재사용
├── repository.rs   # WorldRepository — list_places/get_place/search_places 추가

src/adapter/
└── sqlite_world.rs # places 테이블 + FTS + migrate_v3

src/bin/
└── world_load.rs   # world/place/* 스캔 + Phase 1·2 외래키 활성

genres/wuxia/
├── forms/place.toml          # Phase N 빈 슬롯
└── markdown_template/
    ├── place-settlement.md   # 정치체용 양식
    └── place-geography.md    # 자연 지형용 양식

projects/chilguk-chunchu/
└── world/
    └── place/                # 8-10 .md (Step 3·4)
```

#### `Place` 애그리거트

```rust
// src/domain/world/place.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlaceId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaceLayer {
    Settlement,
    Geography,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Spatial {
    /// 수직 포함 (영토상 1:1). 도시→국가, 문파→산맥, 광역 영역.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_place: Option<PlaceId>,
    /// schematic 위치 ("south-west" 등) — Phase 4 Atlas에서 활용.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_position: Option<String>,
    /// 수평 인접 Place들.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bordering_places: Vec<PlaceId>,
    /// (settlement만) 어느 자연 지형 위에 layered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub geography_refs: Vec<PlaceId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    pub id: PlaceId,
    pub layer: PlaceLayer,                   // 일급
    pub kind: String,                         // 장르가 채움. wuxia: nation|autonomous-zone|city|sect | mountain-range|coast|jungle|grassland|desert|forest|river|lake|landmark
    pub name: String,
    pub aliases: Vec<String>,                 // 별호·옛 이름·자(字)
    pub summary: String,
    pub tags: Vec<String>,
    pub extras: serde_json::Map<String, Value>,    // ki_concentration·climate·hazards·controlling_group 등
    pub body_sections: BTreeMap<String, String>,
    pub spatial: Spatial,
    pub source_path: Option<String>,
}
```

#### `WorldRepository` 확장

```rust
pub trait WorldRepository: Send + Sync {
    // Phase 1·2 (그대로)
    fn list_groups(...)?;
    fn list_persons(...)?;

    // Phase 3 — Place
    fn list_places(&self, filter: PlaceFilter) -> Result<Vec<Place>, WorldError>;
    fn get_place(&self, id: &PlaceId) -> Result<Option<Place>, WorldError>;
    fn search_places(&self, query: &str, top_k: u32) -> Result<Vec<Place>, WorldError>;
}

pub struct PlaceFilter {
    pub layer: Option<PlaceLayer>,
    pub kind: Option<String>,
    pub parent_place: Option<PlaceId>,
    pub genre_tag: Option<String>,
}
```

산출물 검증: `cargo build` 통과. 단위 테스트 — Place 두 layer 인스턴스 + Spatial parent cycle 검출.

### Step 2 — 마크다운 파이프라인 + 외래키 활성

#### Frontmatter 양식 (§6.1 참조)

`serde_yaml` 재사용. line-based H2 파싱 (Phase 1 D2·D3).

#### `world-load` 확장

```
cargo run --features embed --bin world-load -- --project chilguk-chunchu [--reload]
```

동작:
1. Phase 1·2 동작 — group·person 로드
2. Phase 3 동작 — `world/place/*.md` 로드 → places 테이블 upsert
3. **외래키 검증 활성**:
   - `groups.headquarters` Place ID 존재 — 결손 시 **에러**
   - `persons.birthplace`·`current_location` Place ID 존재 — 결손 시 **에러**
   - `places.spatial.parent_place`·`bordering_places`·`geography_refs` 존재 + cycle 검증 — 결손 시 에러
   - `places.extras.controlling_group` Group ID 존재 (sect kind만) — 결손 시 에러
4. **결손 시 partial commit** — Phase 1·2의 strict rollback 정책 그대로. 결손 발견 시 DB 미수정.

산출물 검증: 빈 .md 라운드트립 + 외래키 결손/통과 양쪽 케이스 단위 테스트.

### Step 3 — 두 layer 1쌍 변환 시연 ★체크포인트 1★

**체크포인트 1 = layer 분기 + 외래키 활성 동시 검증**.

대상:
- **대진(大辰)** — Place / Settlement / kind=nation. seven-nations §1 + Phase 1 group-daejin-court 메모.
- **서부 산악지대** — Place / Geography / kind=mountain-range. seven-nations §0.3 다이어그램 + 서량 §3 산악 언급. **신규 1차 작성**.

작업:
1. `seven-nations.md` §1 통독 + group-daejin-court·person npc-02·07 산문 정합성
2. `projects/chilguk-chunchu/world/place/place-daejin.md` 작성 (settlement 양식)
3. 서부 산악지대 신규 작성 (geography 양식, §6.7 가이드 따름)
4. `world-load --reload` 실행

**예상 결과 (외래키 활성 후)**:
```
=== 결과 ===
groups indexed    = 6
persons indexed   = 9
places indexed    = 2     ← 대진 + 서부 산악
fk errors         = N (대진·서부 산악만으론 미해결 ID 다수 잔여 — 의도된 상태)
world-load 실패: N 외래키 결손 — Phase 3 활성. ...
```

`fk errors = N`은 정상. Phase 1·2 시드의 모든 Place ID 참조(낙양 등)가 아직 등록 안 됐기 때문. 체크포인트 1엔 대진·서부 산악만으로 검증하고 Step 4에서 나머지 Place 추가해 0건으로.

5. SQLite 라운드트립 검증 — 두 Place 모든 필드 보존
6. parent_place cycle 검출 단위 테스트 통과
7. layer 분기 검증 — 두 Place가 각자 다른 섹션 마커·extras 사용

**체크포인트 1 보고서** (`docs/tasks/phase3-checkpoint1-report.md`):
- `git diff --stat`
- 두 .md 전문 (변환 결과)
- 로드 후 Place 도메인 객체 dump (JSON, 모든 필드)
- SQLite places 행 수 + place_fts 인덱스 확인
- **변환 시 결정한 것**:
  - 대진: 산문→섹션 매핑, frontmatter `extras` 키 (capital·ki_concentration 등)
  - **대진 aliases** (예: "낙양"·"중원"·"옛 황도")
  - **`spatial.parent_place`**: null (최상위) vs 다른 ID
  - **`spatial.geography_refs`**: 어떤 자연 지형 위에 — `place-western-mountains` 또는 `place-jungwon-plain` 등
  - **`spatial.bordering_places`**: 인접 정치체 (남궁·자유도시·...)
  - 서부 산악지대: 신규 작성이라 추론한 모든 항목 (terrain_type·climate·hazards·signature_features·aliases·parent_place)
  - layer 분기: 두 layer가 다른 섹션 마커·extras 사용했는지
  - **외래키 결손 분석**: world-load 출력의 N건 중 어떤 ID가 (a) Phase 4·5+에서 정의될 것 vs (b) Step 4에서 해소될 것 vs (c) Phase 1·2 시드 텍스트 ID를 단순화해야 할 것
- **막힌 결정**: 디렉터 결정 필요 사항 (특히 sub-place 정밀도 — 자유도시 `back-alleys` 같은 세부 ID는 어떻게 처리할지)
- Step 4 진행 가능 여부 의견

→ Cowork 리뷰 → **commit pause 유지** → 통과 신호 받고 다음 단계.

### Step 4 — 6 settlement 추가 + 자연 1-2 + sect 1-2

체크포인트 1 통과 후:
1. 나머지 6국 settlement: 남궁·서량·북원·남만·동해·자유도시
2. 자연 지형 추가 1-2: 남쪽 밀림(남만), 또는 동해 연안. **Step 4 시작 시 어느 자연 지형 추가할지 디렉터 결정**
3. sect 1-2개: place-namgung-sega 또는 place-aimi-pa (§3.4 이중 등록 시연)
4. **외래키 결손 0건 목표** — 모든 Phase 1·2 시드의 Place ID 참조가 places에 등록됨

**필수 시연**:
- world-load 결과: `fk errors = 0`
- Phase 1·2 시드의 모든 headquarters·birthplace·current_location 매핑 통과
- `spatial.geography_refs` 시연 — 정치체가 자연 지형 위에 layered
- sect 이중 등록 시연 — `place-namgung-sega.controlling_group = group-namgung`

자유도시·동해 같은 sub-place 처리는 Step 4 시작 시 결정:
- (a) Phase 1·2 시드 ID 단순화 — `place-free-cities-back-alleys` → `place-free-cities` (정밀도 떨어짐)
- (b) Phase 3에서 sub-place 추가 — 자유도시 본체 + back-alleys 별도 Place
- (c) Phase 5+로 미루기 — Phase 1·2 시드를 임시로 단순 ID로 갱신

내 권장은 **(a)** 또는 **(c)** — Phase 3 분량 절제. 단 디렉터 결정 사항.

### Step 5 — MCP 도구 3개 + 정성 검증 ★체크포인트 2★

```
list_places(filter: PlaceFilter) -> Vec<PlaceSummary>
  PlaceFilter { layer?, kind?, parent_place?, genre_tag? }
  PlaceSummary { id, name, layer, kind, summary_one_line, tags }

get_place(place_id: String) -> Option<PlaceDetail>
  PlaceDetail = full Place

search_places(query: String, top_k: u32 = 5) -> Vec<PlaceSummary>
  FTS5 trigram (name + aliases + summary + body)
```

**체크포인트 2 보고서** (`docs/tasks/phase3-checkpoint2-report.md`):
- `list_places(layer="settlement")` 결과 (7-8건 — 7국 + sect 1-2)
- `list_places(layer="geography")` 결과 (1-2 자연 지형)
- `list_places(parent_place="place-daejin")` → 대진 영토 내 sub-place
- `get_place("place-daejin")` 전체 detail
- `search_places` 6쿼리:
  - "검성" → place-namgung
  - "독관성" → place-seoryang
  - "낙양" → place-daejin (alias 매칭)
  - "산악" → 서부 산악
  - "남만" → 남만 + 남만 밀림 두 매칭
  - "검왕" → place-namgung-sega (Phase 1 group-namgung body 매칭)
- 외래키 결손 0건 검증
- sect 이중 등록 검증 — Phase 1 group-namgung.headquarters ↔ place-namgung-sega 양방향
- 정성 평가: layer·spatial·외래키 활성이 손실 없이 보존됐는가
- Phase 4 (Atlas) 진입 가능 여부 의견

→ Cowork 리뷰 → 통과 시 Phase 3 종결 → Phase 4 (Atlas) 작전 작성 진입.

## 6. 결정 사항 (변경 시 디렉터 승인)

### 6.1 Frontmatter 양식 — Settlement·Geography 두 결

#### 공통

필수: `id`, `layer`, `kind`, `name`. 권장: `aliases`, `summary`, `tags`, `spatial`. 선택: `extras`, `body_sections`.

`id` 형식: `place-{slug}`. slug = ASCII 소문자·숫자·하이픈. 한자는 한국어 발음 음역.
`tags` 첫 항목 = 장르 (`wuxia`).

#### Settlement layer 양식 — 대진(大辰) 예시

```yaml
---
id: place-daejin
layer: settlement
kind: nation                                # nation | autonomous-zone | city | sect
name: 대진(大辰)
aliases: [낙양, 중원 황도, 옛 통일제국]
summary: |
  270년 전 통일제국 대진의 영토. 현재는 중원 일부로 축소됐으나 정통성 명분과
  최다 인구를 보유한 정치체.
tags: [wuxia, place, settlement, nation, central-plain]
extras:
  capital: 낙양(洛陽)                       # 수도 명
  capital_hanja: 洛陽
  polity: 왕조 (축소 제국)
  population_note: 칠국 중 최다
  ki_concentration: 보통                    # wuxia 특화 — 기 농도
  controlling_group: group-daejin-court     # Phase 1 Group 외래키 (sect도 같은 패턴)
spatial:
  parent_place: ~                           # 최상위 정치체
  relative_position: center
  bordering_places: [place-namgung, place-jiyu-doshi, place-seoryang]
  geography_refs: [place-jungwon-plain]     # 중원 평원 위에 layered
---

## 개요
산문 1-2 단락 — 칠국 중 위상.

## 통치
산문 — 권력 구조. group-daejin-court·group-shipsangsi 참조.

## 핵심 NPC
- 천순제 (npc-07) — 명목 황제
- 조고 (npc-02) — 실권자

## 핵심 갈등
산문 — 영토 수복 야심·정파 동맹·천마신교 적대 등.

## 플레이어가 방문할 이유
산문 — 메인 퀘스트 핵심 무대.

## 전사(前史)
산문 — 옵션. 270년 역사·붉은 밤의 변.
```

권장 H2 섹션: `## 개요` · `## 통치` · `## 핵심 NPC` · `## 핵심 갈등` · `## 플레이어가 방문할 이유` · `## 전사(前史)`.

#### Geography layer 양식 — 서부 산악지대 예시

```yaml
---
id: place-western-mountains
layer: geography
kind: mountain-range                        # mountain-range | coast | jungle | grassland | desert | forest | river | lake | landmark
name: 서부 산악지대
aliases: [서령산맥, 서변 영봉, 만년설봉]
summary: |
  대륙 서쪽 변경. 만년설 봉우리들과 깊은 협곡. 서량 너머로 외부 세계와의
  유일한 통로이자 천연 방벽.
tags: [wuxia, place, geography, terrain, mountain-range, western-frontier]
extras:
  terrain_type: mountain-range
  climate: 고산 한랭, 겨울 폭설
  hazards: [눈사태, 협곡 안개, 산적, 마수]
  signature_features: [망주봉(望主峰), 십리협(十里峽), 천녀폭(天女瀑)]
spatial:
  parent_place: ~                           # 광역 자연 영역 미정의
  relative_position: west
  bordering_places: [place-seoryang]
  # geography_refs는 settlement에서만 — geography는 비움
---

## 개요
산문 1-2 단락 — 지형의 의미·외부와 단절.

## 지형·기후
산문.

## 위험·서식 생물
산문 — 마수 등 무협 결.

## 인접 정치체
- 서량 (place-seoryang) — 산악 동쪽 변경

## 자원·산물
산문 — 약초·광물 등.

## 플레이어가 방문할 이유
산문 — 외부 세계로의 통로·비급 발견 가능성.
```

권장 H2 섹션: `## 개요` · `## 지형·기후` · `## 위험·서식 생물` · `## 인접 정치체` · `## 자원·산물` · `## 플레이어가 방문할 이유`.

#### 공통 — 두 layer 양쪽

`## 개요` · `## 플레이어가 방문할 이유` 두 섹션은 모든 Place에 공통(추상이 보장하는 최소 약속).

#### sect kind 양식 — 이중 등록 시연

```yaml
---
id: place-namgung-sega
layer: settlement
kind: sect                                   # sect는 settlement layer
name: 남궁세가
aliases: [남궁가 본가, 검성 남궁세가]
extras:
  controlling_group: group-namgung           # Phase 1 Group 외래키 — sect 이중 등록 핵심
  capital: 검성(劍城) 외곽 산자락
  ki_concentration: 농후
spatial:
  parent_place: place-namgung                # 영토상 남궁국 안
  geography_refs: [place-western-mountains]  # (예시) 산기슭에 위치
---
```

`extras.controlling_group`을 통해 Place(공간) ↔ Group(조직) 외래키 연결. Group의 `headquarters: place-namgung-sega` 와 양방향.

### 6.2 `genres/wuxia/forms/place.toml` (Phase N 빈 슬롯)

```toml
extends = "place"

[[fields.layer.options]]
value = "settlement"; label = "공동체"
[[fields.layer.options]]
value = "geography"; label = "자연 지형"

[[fields.kind.options]]
value = "nation"; label = "국가"; layer = "settlement"
[[fields.kind.options]]
value = "autonomous-zone"; label = "자치령"; layer = "settlement"
[[fields.kind.options]]
value = "city"; label = "도시"; layer = "settlement"
[[fields.kind.options]]
value = "sect"; label = "문파/세가"; layer = "settlement"
[[fields.kind.options]]
value = "mountain-range"; label = "산악"; layer = "geography"
[[fields.kind.options]]
value = "jungle"; label = "밀림"; layer = "geography"
[[fields.kind.options]]
value = "coast"; label = "해안"; layer = "geography"
# ... 등
```

### 6.3 SQLite 스키마 — places 테이블

```sql
CREATE TABLE places (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    layer TEXT NOT NULL CHECK(layer IN ('settlement','geography')),
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    aliases_json TEXT NOT NULL DEFAULT '[]',
    summary TEXT NOT NULL DEFAULT '',
    tags_json TEXT NOT NULL DEFAULT '[]',
    extras_json TEXT NOT NULL DEFAULT '{}',
    body_sections_json TEXT NOT NULL DEFAULT '{}',
    spatial_json TEXT NOT NULL DEFAULT '{}',
    parent_place TEXT,                         -- spatial.parent_place 캐시 컬럼
    source_path TEXT,
    updated_at INTEGER NOT NULL
);
CREATE INDEX idx_places_layer ON places(layer);
CREATE INDEX idx_places_kind ON places(kind);
CREATE INDEX idx_places_parent ON places(parent_place);

CREATE VIRTUAL TABLE places_fts USING fts5(
    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
);
```

`schema_meta.version = 3` 마이그레이션. Phase 1·2 v2 DB는 자동 ALTER + places·places_fts 추가.

### 6.4 환경변수

`NPC_MIND_WORLD_DB` 그대로 (Phase 1·2 동일).

### 6.5 라이브러리

기존 — Phase 0 D2·D3 의존성 회피 원칙 계승.

### 6.6 외래키 (Phase 3 활성 매트릭스)

| 검증 | Phase 1 | Phase 2 | Phase 3 |
|---|---|---|---|
| `Group.parent_group` cycle | 에러 | 에러 | 에러 |
| `Group.allied/rival_groups` 존재 | 경고 | 경고 | 경고 |
| `Group.members.person_id` 존재 | 경고 | **에러** | 에러 |
| `Group.headquarters` 존재 | 경고 | 경고 | **에러 (활성)** |
| `Person.affiliation` 존재 | (신규) | **에러** | 에러 |
| `Person.birthplace`·`current_location` 존재 | (신규) | 경고 | **에러 (활성)** |
| `Place.spatial.parent_place` cycle | (신규) | (신규) | **에러** |
| `Place.spatial.bordering_places` 존재 | (신규) | (신규) | **에러** |
| `Place.spatial.geography_refs` 존재 + layer 일치 | (신규) | (신규) | **에러** |
| `Place.extras.controlling_group` (sect) 존재 | (신규) | (신규) | **에러** |

### 6.7 서부 산악지대 신규 작성 가이드

seven-nations.md엔 직접 시트 없음. §0.3 다이어그램 + §3 서량 본문에 산악 언급. 다음 정보를 작성·추론:

- `name`: "서부 산악지대" (또는 작가 결정)
- `aliases`: 1-2개 후보 — "서령산맥"·"서변 영봉"·"만년설봉"
- `terrain_type`: mountain-range
- `climate`: 고산 한랭, 겨울 폭설 (서량 위치·당무괴 독관성 언급에서 추론)
- `hazards`: 눈사태·협곡 안개·산적·마수 (게임 무대 합리적 추론)
- `signature_features`: 1-3개 — 명칭은 작가 창작 또는 후속 결정
- `parent_place`: null (광역 자연 영역 미정의) 또는 `place-western-frontier`
- `bordering_places`: [place-seoryang, (산악 너머 외부 세계는 Phase 5+ 범위 외)]
- `relative_position`: "west"
- 본문 산문 1-2 단락 — Lore RAG로 蜀山劍俠傳·江湖奇俠傳의 산악 묘사 검색해 톤 참고 가능

체크포인트 1 보고서에 신규 작성 시 결정한 모든 항목 명시 → 디렉터 검토.

## 7. Out of Scope (Phase 3)

- **Atlas 도메인** — Phase 4 별도 TASK
- 좌표·SVG 맵·hex grid·distance matrix — Phase 4+
- Era overlay (시기별 정치 지도) — Phase 5+
- 영주 번왕국 별도 Place — Phase 5+
- 자유도시 sub-place(`back-alleys` 등) — 디렉터 결정 따라 (a) 정밀도 단순화 (b) Phase 5+
- 폼 시스템·AI 협업 빈칸 — Phase N
- AI 생성 마크다운 — Phase N
- Mind Studio worldbuilding UI 패널 — Phase N+
- Place ↔ Item·Skill·Knowledge 외래키 — Phase 5+ (해당 도메인 등장 시)
- 시나리오·Scene·Beat·Memory 통합 (gameplay 다리) — Phase 5+

## 8. 코드 위치 가이드

작업 시작 5분에 읽을 곳:

| 위치 | 무엇을 볼지 |
|---|---|
| `Cargo.toml` `[features]` | feature 게이팅 |
| `src/domain/world/group.rs` (Phase 1) | aliases·spatial·외래키 패턴 |
| `src/domain/world/person.rs` (Phase 2) | layer-like 분화·HEXACO Score VO 재사용 |
| `src/worldbuilding/markdown/{group,person}.rs` | 마크다운 파서 패턴 |
| `src/adapter/sqlite_world.rs` | migrate_v2·외래키 검증 패턴 |
| `src/bin/world_load.rs` | Phase 1·2 외래키 검증 흐름 — Phase 3 활성 시 같은 패턴 |
| `src/bin/mind-studio/handlers/world_persons.rs` | MCP 도구 등록 패턴 |
| `task-phase3-place-vertical-slice.archive.md` | 보존본 — 양식·SQLite·외래키 패턴 거의 그대로 활용 (Atlas 부분만 분리) |

## 9. 시작 체크리스트

1. `CLAUDE.md` + Phase 0~2 산출 (`src/lore/`·`src/domain/world/`·`src/worldbuilding/`·`src/adapter/sqlite_world.rs`) 빠르게 훑기
2. Phase 1·2 보고서 (`phase1-implementation-report.md` + `phase2-checkpoint{1,2}-report.md` + 두 follow-up 보고서) + `task-phase3-place-vertical-slice.archive.md` 통독
3. **`wuxia-core/docs/world/seven-nations.md` §1 (대진) + §0.3 (대륙 배치) 통독**
4. **현재 Phase 1·2 시드의 모든 Place ID 참조 grep** — `headquarters`·`birthplace`·`current_location`로 등장하는 모든 ID 목록화 (Step 3 외래키 결손 분석에 필요)
5. Place 도메인 + 마크다운 파서 + 단위 테스트 (Step 1·2)
6. SqliteWorldStore migrate_v3 + places + 라운드트립 테스트
7. world-load 확장 — 외래키 활성 (에러 승급)
8. **대진 + 서부 산악지대 변환** → ★체크포인트 1★ 보고 → **commit pause**

## 10. 리뷰 채널

체크포인트 1·2 보고서를 디렉터(사용자)가 Cowork 세션에 복붙. 형식:
- **Done** · **Diff** · **데모 명령** · **결정** · **막힌 것** · **다음 의견**
- 신규 작성 자연 지형의 모든 추론 항목 + 외래키 결손 분석을 본문에 상세히 명시
- Phase 1·2 시드의 sub-place 정밀도 결정 (단순화 vs 분리 vs 미루기)을 디렉터 의견 요청

보고서 파일명:
- 체크포인트 1: `docs/tasks/phase3-checkpoint1-report.md`
- 체크포인트 2: `docs/tasks/phase3-checkpoint2-report.md`
