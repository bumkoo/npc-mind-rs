# Phase 3: Place 카테고리 Vertical Slice (Atlas는 Phase 4로 분리 예정)

> **연기 안내 (2026-04-30)**: 원래 Phase 1로 작성됐으나, 작업 순서를 **Group → Person → Place → Atlas**
> 로 재조정. 이유: (1) 추상 누수 방지 — Place 먼저 짜면 Group 책임(통치체·강호 결사·세가)이
> Place의 kind·extras로 흘러들어옴. (2) 게임 서사 비중 — 사용자 비전 "인물·집단 중심"에 부합.
> 본 문서는 Phase 3 진입 시점에 재활성화. 그때까지 **참고용 보존**.
>
> **Phase 3 진입 시 추가 손질 항목**:
> - sect kind에서 `controlling_group_id` 외래키 활성화 (Phase 1 Group이 정의된 뒤니까)
> - sect Place는 영토·건물만 책임, 권력 관계는 Group이 책임 (이중 등록 패턴)
> - Group·Person 외래키 검증 추가
>
> **Atlas 분리 안내**: Atlas는 *도메인이면서 뷰*의 이중성을 가진 **관계 도메인**(고유 좌표·로직 +
> 합성 인터페이스). 본 문서의 Atlas 섹션은 Phase 4 진입 시점에 별도
> `task-phase4-atlas-vertical-slice.md`로 추출 예정.

---

> **For Claude Code.** 이 문서는 자급 자족이며 외부 링크 없이도 작업 시작 가능.
> 결정 사항을 임의 변경하지 말 것. 변경이 필요하면 보고서에서 디렉터 승인 요청.

## 1. 목표

장르 중립 Worldbuilding 도구의 **공간 추상**을 끝까지 한 사이클 완성.

이번 Phase에서 다루는 추상은 **세 결**:
1. **Place / Settlement layer** — 인문 층(국가·자치령·도시·문파 등 공동체 관리 공간)
2. **Place / Geography layer** — 자연 층(산악·해안·밀림·초원·사막 등 자연 지형)
3. **Atlas** — 메타 카테고리, 위 두 layer를 묶는 통합 뷰(좌표계·인접·전체 다이어그램)

`인터뷰/마크다운(SoT) → DDD 도메인 → 인프라 → MCP 노출` 파이프라인이 위 셋에 한정해 돌아가는지를 검증. 다른 8 카테고리(Person·Group·Item·Skill·Knowledge·Lore·Event·Era)는 Phase 2+.

**검증 게이트**: `wuxia-core/docs/world/seven-nations.md`(작가가 v1.1까지 다듬은 칠국 시트, 1076줄)를 다음으로 변환:
- 7국 정치체 → `place-*.md` × 7 (layer = settlement)
- 서부 산악지대 → `place-western-mountains.md` × 1 (layer = geography)
- 칠국 대륙 → `atlas-jungwon.md` × 1 (§0.3 ASCII 다이어그램 그대로 시드)

→ SQLite 적재 → MCP `list_places`·`get_atlas` 호출 결과로 모두 정확히 반환.

## 2. 연관 컨텍스트

- `CLAUDE.md` 프로젝트 루트
- `docs/tasks/task-phase0-lore-rag-bootstrap.md` — Phase 0 종료, Lore RAG MCP 가동 중
- `docs/tasks/phase0-lore-rag-bootstrap-report1.md` — 인프라 보고
- 메모리(Cowork 세션 보유): 장르 중립 도구·**10 추상 카테고리**(Place·Person·Group·Item·Skill·Knowledge·Lore·Event·Era + **Atlas**)·`genres`/`projects` 분리·SoT는 마크다운
- 입력 자료: `wuxia-core/docs/world/seven-nations.md` (칠국 v1.1, 1076줄). 같은 디렉토리 `history.md`·`history-characters.md`·`character-naming.md`는 Phase 2+ 입력.

## 3. 제약

### 3.1 장르 중립 vs 장르 의존 — 절대 섞지 말 것

| 위치 | 책임 |
|---|---|
| `src/domain/world/place.rs` | **장르 영원히 모름** — id·name·**aliases**·layer·kind(String)·summary·tags·extras·body_sections·spatial(parent_place 포함) |
| `src/domain/world/atlas.rs` | **장르 영원히 모름** — id·name·kind(String)·extent·references·body_sections |
| `src/worldbuilding/markdown/{place,atlas}.rs` | 장르 중립 frontmatter+섹션 파서 |
| `src/adapter/sqlite_world.rs` | 장르 중립 SQLite 스키마 (`places`·`atlases`·`place_atlas_refs`) |
| `genres/wuxia/forms/{place,atlas}.toml` | 무협 kind 옵션·확장 필드 (Phase 1엔 폼 미사용, 자리만) |
| `genres/wuxia/markdown_template/*.md` | 무협 .md 작성 템플릿 |
| `projects/chilguk-chunchu/world/{place,atlas}/*.md` | 칠국춘추 프로젝트의 인스턴스 |

**`src/`에 wuxia 단어가 들어가면 안 됨.** 정체(政體)·기·내공·문파 같은 무협 어휘는 모두 `genres/wuxia/`·`projects/chilguk-chunchu/`에만.

### 3.2 PlaceLayer — 일급 enum

```rust
// src/domain/world/place.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaceLayer {
    /// 공동체 관리 공간 — 국가·도시·문파·자치령. 인간 시간 단위로 변함.
    Settlement,
    /// 자연 지형 — 산악·해안·밀림·초원·사막. 지질학적 시간 단위로 거의 안 변함.
    Geography,
}
```

층위 분리 이유: 같은 좌표 위에 두 결이 포개진다. 정치체는 자연 위에 얹혀 있고 시기마다 바뀌지만 자연은 그대로. `Settlement`는 자기 위치한 `Geography`를 `spatial.geography_refs`로 참조.

### 3.3 Source of Truth = 마크다운

- `projects/chilguk-chunchu/world/place/*.md` + `world/atlas/*.md`가 SoT
- SQLite는 빌드 산출물(.gitignore) — `world-load` CLI로 재생성
- 도메인 객체·SQLite를 직접 편집하는 API 없음 (Phase 1엔 read-only)
- 마크다운 편집 → `cargo run --features embed --bin world-load -- --project chilguk-chunchu`

### 3.4 .gitignore 추가

```
# Worldbuilding builds
projects/*/build/
projects/*/build/world.sqlite
```

`projects/<name>/world/**/*.md`(SoT)는 git에 들어감.

### 3.5 임베딩·검색 범위

Place·Atlas 검색은 **FTS5(trigram) 텍스트 검색만.** 의미 검색(임베딩)은 Phase 2+. 정형 데이터라 list/get/keyword search로 충분.

## 4. Done Criteria

- [ ] 디렉토리 골격: `src/domain/world/`, `src/worldbuilding/`, `genres/wuxia/`, `projects/chilguk-chunchu/`
- [ ] 추상 `Place` (layer + aliases + spatial.parent_place 일급 필드) + `Atlas` 애그리거트 + `WorldRepository` 트레잇 + 단위 테스트
- [ ] 마크다운 frontmatter+섹션 파서 (place·atlas 두 양식) + 단위 테스트
- [ ] `genres/wuxia/markdown_template/{place-settlement,place-geography,atlas}.md` 템플릿 (§6.1)
- [ ] `genres/wuxia/forms/{place,atlas}.toml` 자리 (Phase 2 폼 시스템 빈 슬롯)
- [ ] `SqliteWorldStore` (places·atlases·place_atlas_refs 테이블 + FTS5) + 라운드트립 테스트
- [ ] **체크포인트 1**: 대진(settlement) + 서부 산악지대(geography) 변환 → 두 layer 라운드트립 검증
- [ ] **체크포인트 2**: 나머지 6국 + atlas-jungwon 변환 → MCP 정성 평가
- [ ] `bin/world-load` CLI: 마크다운 일괄 로드 → SQLite 빌드
- [ ] `bin/mind-studio` MCP 도구 4개: `list_places`, `get_place`, `search_places`, `get_atlas`
- [ ] `cargo build` + `cargo test --features embed` 통과
- [ ] 정성 검증: `list_places(layer="geography")` → 서부 산악지대 등장. `list_places(layer="settlement")` → 7국. `get_atlas("atlas-jungwon")` → 다이어그램+references 모두 반환.

## 5. 단계별 작업

### Step 1 — 디렉토리 골격 + 도메인

```
src/domain/world/
├── mod.rs          # pub use, 10 카테고리 자리
├── place.rs        # Place + PlaceId + PlaceLayer + Spatial
├── atlas.rs        # Atlas + AtlasId
├── person.rs, group.rs, item.rs, skill.rs, knowledge.rs, lore.rs, event.rs, era.rs   # 빈

src/worldbuilding/
├── mod.rs
├── markdown/
│   ├── mod.rs
│   ├── frontmatter.rs    # YAML frontmatter 파서
│   ├── place.rs          # Place .md → 도메인
│   └── atlas.rs          # Atlas .md → 도메인
├── repository.rs         # WorldRepository 트레잇 (places + atlases)
└── builder.rs            # 빈 (Phase 2: 폼 시스템 진입점)

src/adapter/
└── sqlite_world.rs       # SqliteWorldStore — places + atlases + FTS5

src/bin/
└── world_load.rs         # CLI

genres/wuxia/
├── genre.toml
├── forms/{place,atlas}.toml          # Phase 1엔 자리만
└── markdown_template/
    ├── place-settlement.md
    ├── place-geography.md
    └── atlas.md

projects/chilguk-chunchu/
├── project.toml          # genre = "wuxia", title, description
└── world/
    ├── place/            # 8개 .md (Step 3·4)
    └── atlas/            # 1개 .md (Step 4)
```

#### `Place` 애그리거트

```rust
// src/domain/world/place.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlaceId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Spatial {
    pub atlas: Option<AtlasId>,                 // 어느 atlas에 속하나
    pub parent_place: Option<PlaceId>,          // 수직 포함 (영토상 1:1). 도시→국가, 문파→산맥 등
    pub relative_position: Option<String>,      // schematic 위치 ("south-west" 등)
    pub bordering_places: Vec<PlaceId>,         // 수평 인접 Place들
    pub geography_refs: Vec<PlaceId>,           // (settlement만) 어느 자연 지형 위에 layered
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    pub id: PlaceId,
    pub layer: PlaceLayer,                      // Settlement | Geography (일급)
    pub kind: String,                           // "nation"/"sect"/"mountain-range" 등 — 장르가 채움
    pub name: String,
    pub aliases: Vec<String>,                   // 이명·별호·옛 이름 (예: 자금성 ↔ 황궁, Erebor ↔ Lonely Mountain)
    pub summary: String,
    pub tags: Vec<String>,
    pub extras: serde_json::Map<String, Value>,
    pub body_sections: BTreeMap<String, String>,
    pub spatial: Spatial,
    pub source_path: Option<String>,
}
```

#### `Atlas` 애그리거트

```rust
// src/domain/world/atlas.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AtlasId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasExtent {
    pub projection: String,                     // "schematic" | "cartesian" | "hex-grid" 등 — Phase 1엔 schematic만
    pub width_units: Option<u32>,
    pub height_units: Option<u32>,
    pub unit: String,                           // "schematic" | "km" | "li" 등
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atlas {
    pub id: AtlasId,
    pub kind: String,                           // "continent" | "region" | "city-map" — 장르가 채움
    pub name: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub extent: AtlasExtent,
    pub references: Vec<PlaceId>,               // 이 atlas에 속한 Place들 (정·자연 둘 다)
    pub body_sections: BTreeMap<String, String>,// `## 배치 다이어그램` 등 — 산문/ASCII 보존
    pub source_path: Option<String>,
}
```

#### `WorldRepository` 트레잇

```rust
#[async_trait]
pub trait WorldRepository: Send + Sync {
    // Place
    async fn list_places(&self, filter: PlaceFilter) -> Result<Vec<Place>, WorldError>;
    async fn get_place(&self, id: &PlaceId) -> Result<Option<Place>, WorldError>;
    async fn search_places(&self, query: &str, top_k: u32) -> Result<Vec<Place>, WorldError>;
    // Atlas
    async fn get_atlas(&self, id: &AtlasId) -> Result<Option<Atlas>, WorldError>;
    async fn list_atlases(&self) -> Result<Vec<Atlas>, WorldError>;
}

#[derive(Debug, Clone, Default)]
pub struct PlaceFilter {
    pub layer: Option<PlaceLayer>,
    pub atlas_id: Option<AtlasId>,
    pub kind: Option<String>,
    pub genre_tag: Option<String>,
}
```

산출물 검증: `cargo build` 통과. 단위 테스트 — Place 두 layer 인스턴스 + Atlas 인스턴스 생성, 직렬화/역직렬화 라운드트립.

### Step 2 — 마크다운 파이프라인

#### 양식 약속 — 자세한 내용은 §6.1 참조

3종 템플릿:
- `place-settlement.md` — 정치체용 (frontmatter + 통치/핵심 NPC 섹션)
- `place-geography.md` — 자연 지형용 (frontmatter + 지형/위험 섹션)
- `atlas.md` — 통합 뷰 (frontmatter + ASCII 다이어그램/배치 섹션)

#### Frontmatter·섹션 파서

`serde_yaml` 추가(이미 있으면 재사용). gray_matter·markdown 추가 크레이트 도입 안 함 — Phase 0 D2 의존성 회피 원칙 계승.

`layer` 필드 결손 시 어떻게 추론할지: 기본 `settlement`으로 폴백 + 빌드 타임 경고. 대신 `kind`가 자연 지형 명시(mountain-range·jungle 등)이면 `geography`로 자동 추론 + 경고. 명시가 권장.

#### CLI: `world-load`

```
cargo run --features embed --bin world-load -- --project chilguk-chunchu [--reload]
```

동작:
1. `projects/chilguk-chunchu/project.toml` 로드 → genre 확인
2. `world/place/*.md` 순회 → `Place` 객체 → `places` 테이블 upsert
3. `world/atlas/*.md` 순회 → `Atlas` 객체 → `atlases` 테이블 upsert
4. `Place.spatial.atlas` 외래키 → `place_atlas_refs` 테이블 채움 (양방향 lookup용)
5. `Place.spatial.bordering_places` ID들이 실제 존재하는지 빌드 타임 경고 (검증은 Phase 4+)
6. 진행률·결과 stdout

산출물 검증: 빈 .md 3종(settlement·geography·atlas) 라운드트립 단위 테스트.

### Step 3 — 두 layer 1쌍 변환 시연 ★체크포인트 1★

**체크포인트 1은 layer 분기 검증의 핵심.** 한 변환만으론 layer 추상이 검증되지 않음.

대상:
- **대진(大辰)** — Place / Settlement / kind=nation
- **서부 산악지대** — Place / Geography / kind=mountain-range — seven-nations.md §0.3 다이어그램의 서량 인접 산악 영역. 본문에 직접 시트는 없으니 **신규 1차 작성** (안내 §6.7)

작업:
1. `seven-nations.md`에서 대진 섹션 → `projects/chilguk-chunchu/world/place/place-daejin.md`
2. `seven-nations.md` §0.3 다이어그램 + 서량 §3 본문 참고하여 → `place/place-western-mountains.md` 신규 작성
3. `cargo run --features embed --bin world-load -- --project chilguk-chunchu`
4. 두 Place의 라운드트립 검증 — 각 layer가 자기 그룹의 frontmatter·섹션을 정확히 보존했는지
5. `place-daejin.spatial.geography_refs`에 `place-western-mountains` 들어가는지 (서량과 산악 사이 관계 — 대진은 서량을 통해 산악과 간접 연결, 직접 인접은 서량) — **이건 대진엔 안 들어감**, 서량 변환 시 들어감. 대진은 spatial 비워둠 또는 인접 정치체 일부만.

**체크포인트 1 보고서**:
- `git diff --stat`
- `place-daejin.md` + `place-western-mountains.md` 전문
- 로드 후 두 Place 도메인 객체의 모든 필드 dump (JSON)
- SQLite 사이즈 + places 행 수
- **변환 시 결정한 것**:
  - 대진: 산문→섹션 마커 매핑, frontmatter extras 키 선택
  - **대진 aliases**: 어떤 별호를 채택했는지 (예: "중원 황실"·"낙양 조정")
  - **대진 parent_place**: null인지(최상위), 다른 ID인지
  - 서부 산악지대: 신규 작성이라 어떤 정보를 추론·창작했는지 명시 (terrain_type·climate·hazards·signature_features·aliases·parent_place)
  - layer 분기: 두 layer가 각자 다른 섹션 마커·extras를 사용했는지
  - **외래키 검증**: world-load 실행 시 cycle·결손 경고가 나오는지 (Phase 1엔 경고만)
- **막힌 결정**: 디렉터 결정 필요 사항
- Step 4·5 진행 가능 여부 의견

→ Cowork 리뷰 → 통과 시 다음 단계.

### Step 4 — 나머지 변환 + atlas

체크포인트 1 통과 후:
1. 나머지 6국(남궁·서량·북원·남만·동해·자유도시) settlement 변환
2. **`atlas-jungwon`** 변환 — seven-nations.md §0.3 ASCII 다이어그램 + §0.1·0.2 종론을 atlas.md에 그대로 가져옴. 다이어그램은 `## 배치 다이어그램` 섹션에 코드블록으로 보존 (도구는 텍스트로만 다룸, 렌더링 X).
3. atlas의 `references`에 8 Place(7 settlement + 1 geography) 모두 등록
4. 각 Place의 `spatial.atlas = atlas-jungwon`, `relative_position` 채움 (south-west·north 등 schematic)
5. 서량의 `spatial.geography_refs = [place-western-mountains]` (정치체-자연 연결 시연)
6. `world-load --reload` → SQLite places=8, atlases=1, place_atlas_refs=8

전사(前史) 처리: 자유도시의 v1.1 영주 번왕국 전사는 자유도시 .md의 `## 전사(前史)` 섹션 안에. 영주는 별도 Place 분리 안 함(Phase 1 OoS).

### Step 5 — MCP 도구 4개 + 정성 검증 ★체크포인트 2★

```
list_places(filter: PlaceFilter) -> Vec<PlaceSummary>
  PlaceFilter { layer?, atlas_id?, kind?, genre_tag? }
  PlaceSummary { id, name, layer, kind, summary_one_line, tags }

get_place(place_id: String) -> Option<PlaceDetail>
  PlaceDetail = full Place

search_places(query: String, top_k: u32 = 5) -> Vec<PlaceSummary>
  FTS5 trigram

get_atlas(atlas_id: String) -> Option<Atlas>
  body_sections 포함 (`## 배치 다이어그램` ASCII가 그대로 반환)
```

`AppState`에 embed-gated `world_store: Option<Arc<dyn WorldRepository>>` 추가. `NPC_MIND_WORLD_DB` 부재 시 graceful skip(Phase 0 lore_store 패턴).

**체크포인트 2 보고서**:
- `list_places(layer="settlement")` 결과 (7국 — id·name·kind·1줄 요약)
- `list_places(layer="geography")` 결과 (서부 산악지대)
- `list_places(atlas_id="atlas-jungwon")` 결과 (8개 — settlement 7 + geography 1)
- `get_place("place-daejin")` 전체 detail
- `get_place("place-western-mountains")` 전체 detail
- `get_atlas("atlas-jungwon")` — `## 배치 다이어그램`이 ASCII 그대로 보존됐는지 확인
- `search_places` 6쿼리:
  - "검왕" → 남궁
  - "독" → 서량
  - "의회" → 동해
  - "유목" → 북원
  - "산악" → 서부 산악지대 (geography 매칭 시연)
  - **"황궁" 또는 "낙양 조정" → 대진** (aliases 매칭 시연 — name엔 안 나오지만 aliases엔 들어감)
- 정성 평가: layer 분기·atlas 통합 뷰가 손실 없이 보존됐는가
- Phase 2 진행 가능 여부 의견

→ Cowork 리뷰 → 통과 시 Phase 1 종료.

## 6. 결정 사항 (변경 시 디렉터 승인)

### 6.1 Frontmatter·섹션 약속 (장르 중립)

#### 공통 (모든 Place layer)
필수: `id`, `layer`, `kind`, `name`. 권장: `summary`, `tags`. 선택: `extras`, `spatial`, `body_sections`.

`id` 형식: `place-{slug}`. slug = ASCII 소문자·숫자·하이픈. 한자는 한국어 발음으로 음역(예: `place-daejin`).
`tags` 첫 항목 = 장르(`wuxia`).

#### Settlement layer 양식

```yaml
layer: settlement
kind: nation                                # nation | autonomous-zone | city | sect
aliases: [중원 황실, 낙양 조정]              # 옛 이름·별호 — FTS5 검색 대상에 포함
extras:
  capital: 낙양
  ruler_name: 천순제
  ruler_kind: 꼭두각시 황제
  shadow_ruler: 조고
  polity: 왕조 (축소 제국)
  independence_year: ~                      # null
spatial:
  atlas: atlas-jungwon
  parent_place: ~                           # 최상위 정치체 — null. 도시·문파라면 부모 정치체·산맥 ID
  relative_position: center
  bordering_places: [place-namgung, place-jiyu-doshi]
  geography_refs: []                        # 정치체가 어느 자연 지형 위에 layered
```

권장 H2 섹션: `## 개요` · `## 통치` · `## 핵심 NPC` · `## 핵심 갈등` · `## 플레이어가 방문할 이유` · `## 전사(前史)`

#### Geography layer 양식

```yaml
layer: geography
kind: mountain-range                        # mountain-range | coast | jungle | grassland | desert | forest | river | lake | landmark
aliases: [서령산맥, 서변 영봉]               # 자연 지형도 별칭 흔함
extras:
  terrain_type: mountain-range
  climate: 고산 한랭, 겨울 폭설
  hazards: [눈사태, 협곡 안개, 산적]
  signature_features: [망주봉, 십리협, 천녀폭]
spatial:
  atlas: atlas-jungwon
  parent_place: ~                           # 광역 자연 영역의 일부면 부모 ID
  relative_position: west
  bordering_places: [place-seoryang]
  # geography_refs는 settlement에서만 — geography는 비움
```

권장 H2 섹션: `## 개요` · `## 지형·기후` · `## 위험·서식 생물` · `## 인접 정치체` · `## 자원·산물` · `## 플레이어가 방문할 이유`

#### 공통 — 두 layer 양쪽

`## 개요` · `## 플레이어가 방문할 이유` 두 섹션은 모든 Place에 공통(추상이 보장하는 최소 약속).

#### Atlas 양식

```yaml
---
id: atlas-jungwon
kind: continent                             # continent | region | city-map
name: 중원 대륙
summary: |
  ...
tags: [wuxia, atlas, continent]
extent:
  projection: schematic
  width_units: 7
  height_units: 7
  unit: schematic
references:
  - place-daejin
  - place-namgung
  - place-seoryang
  - place-bukwon
  - place-namman
  - place-donghae
  - place-jiyu-doshi
  - place-western-mountains
---

## 개요
산문 — seven-nations.md §0.1 종론 옮김.

## 칠국 일람
산문 — §0.2 일람표 옮김.

## 배치 다이어그램
\`\`\`
                    ┌──────────────────┐
                    │     북 원        │
                    │   (초원/유목)    │
                    │   왕정(오르두)    │
                    └────────┬─────────┘
                    ...
\`\`\`

## 자연 영역 분포
산문 — §0.3 본문.

## 정치체 분포
산문.

## 주요 통로·연결
- 대진 ↔ 남궁: 중원 평원 직통
- 서량 ↔ 자유도시: 서부 산악 협곡 통로
- 동해 ↔ 자유도시: 해상 항로
```

### 6.2 `genres/wuxia/forms/*.toml` (Phase 2 빈 슬롯)

```toml
# place.toml
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
value = "mountain-range"; label = "산악"; layer = "geography"
[[fields.kind.options]]
value = "jungle"; label = "밀림"; layer = "geography"
# ...
```

### 6.3 SQLite 스키마

```sql
CREATE TABLE places (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    layer TEXT NOT NULL CHECK(layer IN ('settlement','geography')),
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    aliases_json TEXT NOT NULL DEFAULT '[]',     -- 이명·별호 배열
    parent_place TEXT,                            -- spatial.parent_place 캐시 컬럼 (조회·검증용)
    summary TEXT NOT NULL DEFAULT '',
    tags_json TEXT NOT NULL DEFAULT '[]',
    extras_json TEXT NOT NULL DEFAULT '{}',
    body_sections_json TEXT NOT NULL DEFAULT '{}',
    spatial_json TEXT NOT NULL DEFAULT '{}',
    source_path TEXT,
    updated_at INTEGER NOT NULL
);
CREATE INDEX idx_places_layer ON places(layer);
CREATE INDEX idx_places_kind ON places(kind);
CREATE INDEX idx_places_parent ON places(parent_place);

CREATE TABLE atlases (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    summary TEXT NOT NULL DEFAULT '',
    tags_json TEXT NOT NULL DEFAULT '[]',
    extent_json TEXT NOT NULL DEFAULT '{}',
    references_json TEXT NOT NULL DEFAULT '[]',
    body_sections_json TEXT NOT NULL DEFAULT '{}',
    source_path TEXT,
    updated_at INTEGER NOT NULL
);

CREATE TABLE place_atlas_refs (
    place_id TEXT NOT NULL,
    atlas_id TEXT NOT NULL,
    relative_position TEXT,
    PRIMARY KEY (place_id, atlas_id)
);
CREATE INDEX idx_par_atlas ON place_atlas_refs(atlas_id);

CREATE VIRTUAL TABLE places_fts USING fts5(
    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
);

CREATE VIRTUAL TABLE atlases_fts USING fts5(
    id UNINDEXED, name, summary, body, tokenize='trigram'
);
```

`schema_meta` 테이블로 마이그레이션 버전 관리 (Phase 0 패턴).

### 6.4 환경변수

- `NPC_MIND_WORLD_DB` = `projects/chilguk-chunchu/build/world.sqlite` (없으면 graceful skip)
- 다중 프로젝트 관리는 Phase 2+

### 6.5 라이브러리

- TOML: 기존 재사용
- YAML(frontmatter): `serde_yaml` (없으면 추가)
- 추가 마크다운 크레이트(`gray_matter`·`pulldown-cmark`) 도입 금지 — Phase 0 D2 원칙

### 6.6 외래키

Phase 1엔 다음 외래키들의 ID 존재 검증을 **빌드 타임 경고**만(에러 아님). `world-load` 실행 시 stdout에 경고 출력:

- `spatial.bordering_places` 각 ID 존재
- `spatial.geography_refs` 각 ID 존재
- `spatial.parent_place` ID 존재
- `atlas.references` 각 ID 존재
- **`parent_place` cycle 검증** — DFS로 자기 도달 시 경고 (A→B→A 등)

검증·해상도(에러 승급)는 Phase 4(인물 통합)에서 다른 외래키와 함께 정식 도입.

### 6.7 서부 산악지대 신규 작성 가이드

seven-nations.md엔 직접 시트 없음. §0.3 다이어그램에서 서량이 "(서부변경)"으로 표기되고 §3 서량 본문에 산악 언급이 있음. 다음 정보를 작성·추론:

- `name`: "서부 산악지대" (또는 작가 결정)
- `terrain_type`: mountain-range
- `climate`: 고산 한랭, 겨울 폭설 (서량 위치·당무괴 독관성 언급에서 추론)
- `hazards`: 눈사태, 협곡 안개, 산적 (게임 무대로 합리적 추론)
- `signature_features`: 1-3개 — 명칭은 작가 창작 또는 추후 결정
- `aliases`: 1-2개 — 작가 결정 또는 보고서에서 후보 제시 (예: "서령산맥"·"서변 영봉")
- `parent_place`: null (광역 자연 영역이 따로 정의되지 않으면) 또는 `place-western-frontier` 같은 더 큰 영역
- `bordering_places`: [place-seoryang, (이 산악 너머 외부 세계는 Phase 1 범위 외)]
- 본문 산문은 1-2 단락 — Lore RAG로 무협지 산악 묘사 검색해 톤 참고 가능 (창작 보조)

이 신규 작성 시 결정한 모든 항목을 체크포인트 1 보고서에 명시해 디렉터가 검토.

## 7. Out of Scope (Phase 1)

- 폼 시스템 (Phase 2)
- AI 협업 빈칸 채움 — Lore RAG 자동 호출 (Phase 2)
- Place·Atlas 외 8 카테고리 도메인 모델
- Place ↔ Person/Skill 외래키 검증
- 의미 검색(임베딩) — Phase 1엔 FTS5만
- Mind Studio worldbuilding UI 패널 (Phase 3+)
- WorldEvent 이벤트 발행 (Phase 5+)
- 다중 프로젝트 관리 (Phase 2+)
- **SVG 맵·좌표계·hex grid·distance matrix** (Phase 5+)
- **Era overlay — 시기별 정치 지도** (Phase 5+, Era 카테고리 통합 시)
- 영주 번왕국을 별도 Place로 분리 (Phase 2+)
- 서부 산악지대 외 자연 지형(밀림·초원·해안·사막) — Phase 2 초반 확장 후보
- **강호 동맹·무림 권력 네트워크 등 비국가 조직체** — Phase 2+ `Group` 카테고리. Place의 sect kind는 공간 인스턴스(영토 소속·건물)만 다루고, 동맹·맹주·결사 같은 권력 관계는 Group이 책임. sect는 Place + Group 양쪽에 동시 등록되며 외래키로 연결(Phase 2+).
- aliases·parent_place의 다국어 i18n (Phase 2+)

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

1. `CLAUDE.md` 통독 + Phase 0 산출 (`src/lore/`·`bin/lore_ingest.rs`·`mind-studio/mcp_server.rs`) 빠르게 훑기
2. `wuxia-core/docs/world/seven-nations.md` 1076줄 통독 — Phase 1 입력의 형태 파악
3. 디렉토리 골격 신설 (Step 1) — 8 카테고리는 빈 스켈레톤만
4. `Place`(layer 일급) + `Atlas` + `WorldRepository` + 마크다운 파서 + 단위 테스트
5. SqliteWorldStore + 라운드트립 테스트
6. **대진 + 서부 산악지대 변환** → ★체크포인트 1★ 보고

## 10. 리뷰 채널

체크포인트 1·2 보고서를 디렉터(사용자)가 Cowork 세션에 복붙. 형식:
- **Done** · **Diff** · **데모 명령** · **결정** · **막힌 것** · **다음 의견**
- 신규 작성한 자연 지형(서부 산악지대)의 모든 추론 항목을 본문에 상세히 명시

보고서 파일명:
- 체크포인트 1: `docs/tasks/phase1-checkpoint1-report.md`
- 체크포인트 2: `docs/tasks/phase1-checkpoint2-report.md`
