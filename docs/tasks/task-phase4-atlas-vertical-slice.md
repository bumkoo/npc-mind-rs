# Phase 4: Atlas Vertical Slice (첫 관계 도메인, 도메인+뷰 이중성)

> **For Claude Code.** 이 문서는 자급 자족이며 외부 링크 없이도 작업 시작 가능.
> 결정 사항을 임의 변경하지 말 것. 변경이 필요하면 보고서에서 디렉터 승인 요청.
> **선행 조건**: Phase 0·1·2(+2.1·2.2)·3 모두 종결.
> **체크포인트 분리 게이트 강제 적용** — Phase 1 미준수 후속, Phase 2·3에서 정상 회복. 1회 통합 commit 금지.

---

## 1. 목표

장르 중립 Worldbuilding 도구의 **첫 관계 도메인 = Atlas**를 끝까지 한 사이클. **9 인스턴스 도메인과 결이 다른 새 추상 검증** — 도메인+뷰 이중성.

이번 Phase가 다루는 추상은 **세 결**:
1. **Atlas 도메인** — id·name·aliases·kind·extent(projection·units)·references(Vec<PlaceId>) + body_sections(다이어그램 ASCII 보존)
2. **도메인+뷰 이중성** — Atlas는 자기 고유 상태(좌표·projection·격자) + 고유 로직(거리·인접·세력권 합성) + view 인터페이스(다른 도메인 합성 노출)
3. **관계 도메인 패밀리의 첫 사례** — 미래 Timeline(Event×Era)·OrgChart(Group×Group)·FamilyTree(Person×Group)·SkillTree(Skill 집합)의 패턴 정착

**검증 게이트**: `wuxia-core/docs/world/seven-nations.md §0.3` ASCII 다이어그램을 시드로 `atlas-jungwon` 1개 변환:
- `references` = Phase 3의 11 Place 모두 (8 settlement + 3 geography)
- `## 배치 다이어그램` 섹션에 §0.3 ASCII 그대로 보존
- view 메서드 (place 합성·인접·layer 필터) 자동 e2e
- MCP 도구 2개 (`list_atlases`·`get_atlas`)
- 외래키 결손 0건 (references 모두 places 테이블에 존재)

## 2. 연관 컨텍스트

- `CLAUDE.md` 프로젝트 루트
- `docs/tasks/00-roadmap.md` — 전체 흐름·결정 로그
- `docs/tasks/task-phase3-place-vertical-slice.md` + `phase3-checkpoint{1,2}-report.md` — Phase 3 결과
- `docs/tasks/task-phase3-place-vertical-slice.archive.md` — 원래 Phase 1로 작성된 보존본. **Atlas 부분이 거기에 들어 있어 본 TASK 작성 시 양식·SQLite·외래키 패턴 재활용 가능**
- 메모리(Cowork 세션 보유): 도구 추상화 두 결, 9 인스턴스 + 1 관계 도메인, Atlas 이중성, 미래 관계 도메인 패밀리 후보
- 입력 자료:
  - `wuxia-core/docs/world/seven-nations.md §0.1·§0.2·§0.3` — 칠국 종론 + 일람표 + 대륙 배치 ASCII 다이어그램 (Phase 4 핵심 입력)
  - Phase 3 산출 — `projects/chilguk-chunchu/world/place/*.md` × 11 (atlas references의 입력 후보)

## 3. 제약

### 3.1 장르 중립 vs 의존

| 위치 | 책임 |
|---|---|
| `src/domain/world/atlas.rs` | **장르 영원히 모름** — id·name·aliases·kind(String)·summary·tags·extras·**extent(projection·units)**·**references(Vec<PlaceId>)**·body_sections |
| `src/worldbuilding/markdown/atlas.rs` | 장르 중립 frontmatter+섹션 파서 (Phase 1·2·3 패턴 미러) |
| `src/worldbuilding/views/atlas.rs` | (선택) 도메인+뷰의 view 메서드 분리 자리 — Phase 5 Timeline 등장 시 일반화. Phase 4엔 같은 파일에 포함 가능 |
| `src/adapter/sqlite_world.rs` (확장) | `atlases` 테이블 + `atlases_fts` + `place_atlas_refs` 정식 활성 + `migrate_v4` |
| `genres/wuxia/forms/atlas.toml` | Phase N 빈 슬롯 |
| `genres/wuxia/markdown_template/atlas.md` | 무협 atlas 양식 |
| `projects/chilguk-chunchu/world/atlas/atlas-jungwon.md` | 칠국 대륙 인스턴스 |

**`src/`에 wuxia 단어 X.** 칠국·중원·황도 같은 어휘는 `genres/wuxia/`·`projects/chilguk-chunchu/`에만.

### 3.2 도메인+뷰 이중성 — Atlas의 본질

Atlas는 **단순 view가 아님**. 자기 고유 상태(좌표·projection·격자) + 고유 로직(거리·인접 그래프 traversal) + 합성 인터페이스 셋 모두를 가짐.

```rust
// 도메인 측면 — 자기 데이터 소유
pub struct Atlas {
    pub id: AtlasId,
    pub extent: AtlasExtent,           // projection·units 등 자기 좌표계
    pub references: Vec<PlaceId>,      // 어느 Place들이 이 view에 등장
    pub body_sections: BTreeMap<String, String>,  // ## 배치 다이어그램 ASCII
    // ...
}

// 뷰 측면 — 다른 도메인 합성 메서드 (도메인 객체에 메서드로 부착)
impl Atlas {
    /// references 따라 Place 정보 합성
    pub fn places_in<R: WorldRepository>(&self, repo: &R) -> Vec<Place>;

    /// settlement만 / geography만 필터링
    pub fn settlements_in<R>(&self, repo: &R) -> Vec<Place>;
    pub fn geographies_in<R>(&self, repo: &R) -> Vec<Place>;

    /// 특정 Place의 인접 그래프 (Phase 3 spatial.bordering_places 따라)
    pub fn adjacent_to<R>(&self, place_id: &PlaceId, repo: &R) -> Vec<PlaceId>;
}
```

**View trait 일반화는 Phase 5+ 미룸** — 두 번째 관계 도메인(Timeline·OrgChart 등)이 등장할 때 공통 패턴 추출. Phase 4엔 Atlas 단독 처리.

### 3.3 Phase 3 외래키 활성 패턴 일관

`Atlas.references` ID는 모두 `places` 테이블에 존재해야 (Phase 3 hard-fail 패턴 그대로):
- world-load 시 `references` 각 PlaceId 검증
- 결손 시 에러 + DB 미수정 (partial commit 방지)
- `place_atlas_refs` 테이블에 양방향 인덱스 (atlas → place 정방향, place → atlas 역참조 동시 lookup)

### 3.4 Era overlay는 Phase 5 분리

**시기별 정치 지도(통일제국 시대 vs 칠국춘추 시대)는 Phase 5 Era 결합 시 활성**. Phase 4엔 단일 시점(현재 칠국춘추) atlas만. Era overlay 메커니즘 자리만 박아두기:
- `Atlas.extras.era`: 잠정 텍스트 ("현재" 또는 "270년차")
- `Atlas.extras.era_id`: Phase 5 Era 외래키 텍스트 보존만

### 3.5 SoT = 마크다운

기존 흐름 동일. SQLite는 빌드 산출물(.gitignore).

### 3.6 검색 범위

FTS5 trigram + LIKE fallback (Phase 1 D5 패턴).

### 3.7 체크포인트 분리 게이트 — 강제 적용

1. **체크포인트 1**: `atlas-jungwon` 단독 변환 + references 11 Place 외래키 검증 + body_sections ASCII 보존 라운드트립 → commit pause → `phase4-checkpoint1-report.md` → Cowork 리뷰
2. **체크포인트 2**: view 메서드 + MCP 도구 + 정성 평가 → commit pause → `phase4-checkpoint2-report.md` → Phase 4 종결

**1회 통합 commit 금지.**

## 4. Done Criteria

- [ ] 디렉토리 골격: `src/domain/world/atlas.rs`(기존 stub 채움), `src/worldbuilding/markdown/atlas.rs`
- [ ] `Atlas` 애그리거트 + `AtlasId` + `AtlasExtent` + view 메서드(`places_in`·`settlements_in`·`geographies_in`·`adjacent_to`) + 단위 테스트
- [ ] 마크다운 frontmatter+섹션 파서 + 단위 테스트 (특히 `## 배치 다이어그램` ASCII 보존 검증)
- [ ] `genres/wuxia/markdown_template/atlas.md` 템플릿
- [ ] `genres/wuxia/forms/atlas.toml` 자리 (Phase N 빈 슬롯)
- [ ] `SqliteWorldStore` 확장 — `atlases` + `atlases_fts` + `place_atlas_refs` 정식 활성 + `migrate_v4`
- [ ] `bin/world-load` 확장 — `world/atlas/*.md` 스캔 + references 외래키 활성 (에러 승급)
- [ ] `bin/mind-studio` MCP 도구 2개: `list_atlases` · `get_atlas`
- [ ] **체크포인트 1**: atlas-jungwon 단독 변환 + references 11 Place 외래키 통과 + ASCII 다이어그램 라운드트립
- [ ] **체크포인트 2**: view 메서드 자동 e2e + MCP 정성 평가 + 외래키 결손 0건
- [ ] `cargo build` + `cargo test --features embed` + 기존 e2e 회귀 통과
- [ ] 정성 검증: `get_atlas("atlas-jungwon").places_in(repo)` → 11 Place / `settlements_in` → 8 / `geographies_in` → 3 / `adjacent_to(place-daejin, repo)` → 인접 3개

## 5. 단계별 작업

### Step 1 — 디렉토리 골격 + 도메인

```
src/domain/world/
├── atlas.rs            # Atlas + AtlasId + AtlasExtent + view 메서드 (기존 stub 채움)
├── group.rs, person.rs, place.rs   # Phase 1·2·3 그대로
└── ...

src/worldbuilding/
├── markdown/
│   ├── atlas.rs        # Atlas .md → 도메인 (신규)
│   └── ...
├── repository.rs       # WorldRepository — list_atlases/get_atlas 추가

src/adapter/
└── sqlite_world.rs     # atlases 테이블 + FTS + place_atlas_refs 활성 + migrate_v4

src/bin/
└── world_load.rs       # world/atlas/* 스캔 + references 외래키 활성

genres/wuxia/
├── forms/atlas.toml          # Phase N 빈 슬롯
└── markdown_template/atlas.md

projects/chilguk-chunchu/
└── world/
    └── atlas/                # atlas-jungwon.md (Step 3)
```

#### `Atlas` 애그리거트

```rust
// src/domain/world/atlas.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AtlasId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AtlasExtent {
    /// "schematic" (Phase 4) | "cartesian" | "hex-grid" (Phase N+)
    pub projection: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_units: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height_units: Option<u32>,
    /// "schematic" (Phase 4 단위 의미 없음) | "km" | "li" (Phase N+)
    #[serde(default = "default_unit")]
    pub unit: String,
}

fn default_unit() -> String { "schematic".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atlas {
    pub id: AtlasId,
    pub kind: String,                              // "continent" | "region" | "city-map" — 장르가 채움
    pub name: String,
    pub aliases: Vec<String>,
    pub summary: String,
    pub tags: Vec<String>,
    pub extras: serde_json::Map<String, Value>,    // era·era_id (Phase 5 시드)·기타
    pub extent: AtlasExtent,
    pub references: Vec<PlaceId>,                  // 핵심 — atlas에 등장하는 Place들
    pub body_sections: BTreeMap<String, String>,   // ## 배치 다이어그램 등 ASCII/산문 보존
    pub source_path: Option<String>,
}

// view 메서드 — 도메인 객체에 부착 (Phase 5+ trait으로 일반화)
impl Atlas {
    /// 본 atlas의 references 따라 Place 정보 합성.
    pub fn places_in<R: WorldRepository>(&self, repo: &R) -> Result<Vec<Place>, WorldError> {
        self.references.iter()
            .filter_map(|id| repo.get_place(id).transpose())
            .collect()
    }

    /// settlement layer만.
    pub fn settlements_in<R: WorldRepository>(&self, repo: &R) -> Result<Vec<Place>, WorldError> {
        Ok(self.places_in(repo)?
            .into_iter()
            .filter(|p| p.layer == PlaceLayer::Settlement)
            .collect())
    }

    /// geography layer만.
    pub fn geographies_in<R: WorldRepository>(&self, repo: &R) -> Result<Vec<Place>, WorldError> {
        Ok(self.places_in(repo)?
            .into_iter()
            .filter(|p| p.layer == PlaceLayer::Geography)
            .collect())
    }

    /// 특정 Place의 인접 그래프 (Place.spatial.bordering_places 따라).
    /// Atlas의 references 안에 있는 인접만 반환 (atlas 경계 밖은 무시).
    pub fn adjacent_to<R: WorldRepository>(
        &self,
        place_id: &PlaceId,
        repo: &R,
    ) -> Result<Vec<PlaceId>, WorldError> {
        let place = repo.get_place(place_id)?
            .ok_or_else(|| WorldError::PlaceNotFound(place_id.clone()))?;
        let in_atlas: HashSet<&PlaceId> = self.references.iter().collect();
        Ok(place.spatial.bordering_places.into_iter()
            .filter(|id| in_atlas.contains(id))
            .collect())
    }
}
```

#### `WorldRepository` 확장

```rust
pub trait WorldRepository: Send + Sync {
    // Phase 1·2·3 (그대로)
    fn list_groups(...)?;
    fn list_persons(...)?;
    fn list_places(...)?;
    fn get_place(...)?;

    // Phase 4 — Atlas
    fn list_atlases(&self, filter: AtlasFilter) -> Result<Vec<Atlas>, WorldError>;
    fn get_atlas(&self, id: &AtlasId) -> Result<Option<Atlas>, WorldError>;
    fn search_atlases(&self, query: &str, top_k: u32) -> Result<Vec<Atlas>, WorldError>;
}

pub struct AtlasFilter {
    pub kind: Option<String>,
    pub genre_tag: Option<String>,
}
```

산출물 검증: `cargo build` 통과. 단위 테스트 — Atlas 인스턴스 생성, view 메서드(places_in·settlements_in·geographies_in·adjacent_to) 작동.

### Step 2 — 마크다운 파이프라인 + references 외래키 활성

#### Frontmatter 양식 (§6.1 참조)

`serde_yaml` 재사용. line-based H2 파싱 — 단 `## 배치 다이어그램` 안의 ASCII 코드블록(```...```)이 그대로 보존되도록 주의 (FTS 인덱스에 들어가도 무방하나 마크다운 파서가 코드블록 처리 안 망가뜨릴 것).

#### `world-load` 확장

```
cargo run --features embed --bin world-load -- --project chilguk-chunchu [--reload]
```

동작:
1. Phase 1·2·3 동작 — group·person·place 로드
2. Phase 4 동작 — `world/atlas/*.md` 로드 → atlases 테이블 upsert
3. **외래키 검증 활성**:
   - `Atlas.references` 각 PlaceId 존재 — 결손 시 **에러**
   - `place_atlas_refs` 테이블에 양방향 매핑 채움
4. `Atlas.extras.era_id`(있으면) Phase 5 Era 외래키 텍스트 보존, 검증 비활성 (Phase 5에서 활성)
5. partial commit 방지 (Phase 3 패턴 그대로)

산출물 검증: 빈 atlas .md 라운드트립 + references 결손/통과 양쪽 케이스 단위 테스트.

### Step 3 — atlas-jungwon 단독 변환 시연 ★체크포인트 1★

대상: `atlas-jungwon` (칠국 대륙). 시드 입력:
- `wuxia-core/docs/world/seven-nations.md §0.1` — 종론
- `seven-nations.md §0.2` — 칠국 일람표
- `seven-nations.md §0.3` — 대륙 배치 ASCII 다이어그램 + 자연 영역 분포 본문

작업:
1. `seven-nations.md §0.1·§0.2·§0.3` 통독
2. `projects/chilguk-chunchu/world/atlas/atlas-jungwon.md` 작성:
   - frontmatter: id·kind=continent·name·aliases·summary·tags·extent(projection=schematic, units 미정 또는 7×7 schematic)·**references = 11 Place 모두**
   - `## 개요` — §0.1 종론
   - `## 칠국 일람` — §0.2 표
   - `## 배치 다이어그램` — §0.3 ASCII **그대로 보존** (코드블록 안에)
   - `## 자연 영역 분포` — §0.3 본문
   - `## 정치체 분포` — 산문
   - `## 주요 통로·연결` — 산문 (대진↔남궁·서량↔자유도시 등)
3. `cargo run --features embed --bin world-load -- --project chilguk-chunchu`
4. SQLite atlases 1행 검증 + references 11개 외래키 통과
5. 라운드트립 — `## 배치 다이어그램` ASCII가 손실 없이 보존됐는지 (특히 `┌──┐` 같은 box-drawing 문자)
6. view 메서드 검증 — `atlas.places_in(repo).len() == 11` / `atlas.settlements_in(repo).len() == 8` / `atlas.geographies_in(repo).len() == 3`

**체크포인트 1 보고서** (`docs/tasks/phase4-checkpoint1-report.md`):
- `git diff --stat`
- `atlas-jungwon.md` 전문 (변환 결과)
- 로드 후 Atlas 도메인 객체 dump (JSON)
- ASCII 다이어그램 라운드트립 검증 결과 (원본·복원 둘 다 표시)
- world-load 결과 (atlases indexed = 1, references 결손 0)
- view 메서드 호출 결과 (places_in·settlements_in·geographies_in·adjacent_to)
- **변환 시 결정한 것**:
  - aliases (예: "중원 대륙"·"칠국춘추 대륙")
  - kind = continent
  - extent.projection = schematic
  - extent.width_units·height_units (또는 미정 둘 다)
  - extras.era 잠정값 (예: "현재" 또는 "270년차")
  - extras.era_id 비움 (Phase 5 외래키)
  - 산문 → 섹션 마커 매핑 (§0.1·0.2·0.3 → 어느 H2)
- **막힌 결정**: 디렉터 결정 필요 사항 (특히 ASCII 다이어그램 안의 빈칸·정렬 보존 여부, references 정렬 순서 등)
- Step 4 진행 가능 여부 의견

→ Cowork 리뷰 → **commit pause 유지** → 통과 신호 받고 다음 단계.

### Step 4 — view 메서드 + MCP 도구 ★체크포인트 2★

```
list_atlases(filter: AtlasFilter) -> Vec<AtlasSummary>
  AtlasFilter { kind?, genre_tag? }
  AtlasSummary { id, name, kind, summary_one_line, tags, places_count }

get_atlas(atlas_id: String) -> Option<AtlasDetail>
  AtlasDetail = full Atlas (extent + references + body_sections 포함)
```

view 메서드 e2e 자동화:
- `atlas_places_in_returns_eleven`
- `atlas_settlements_in_returns_eight`
- `atlas_geographies_in_returns_three`
- `atlas_adjacent_to_daejin_returns_three` (place-namgung·place-jiyu-doshi·place-seoryang)
- `atlas_adjacent_to_namgung_sega_returns_zero_or_one` (sect는 보통 인접 없음, parent·controlling만)
- `atlas_layer_filter_invariant_holds` — settlements_in의 모든 결과가 layer=Settlement
- `ascii_diagram_preserved_byte_exact` — body_sections의 `## 배치 다이어그램`이 byte-exact 보존
- `references_zero_fk_residual` — 모든 references PlaceId가 places에 존재

**체크포인트 2 보고서** (`docs/tasks/phase4-checkpoint2-report.md`):
- `list_atlases()` 결과 — atlas-jungwon 1건
- `get_atlas("atlas-jungwon")` 전체 detail (references 11 Place + ASCII 다이어그램)
- view 메서드 호출 결과 표 (places_in·settlements_in·geographies_in·adjacent_to)
- search_atlases 2-3쿼리 — "칠국"·"중원"·"대륙"
- ASCII 다이어그램 byte-exact 보존 확인
- 외래키 결손 0건 검증
- Phase 5 (Event + Era + Timeline view) 진입 가능 여부 의견

→ Cowork 리뷰 → 통과 시 Phase 4 종결.

## 6. 결정 사항 (변경 시 디렉터 승인)

### 6.1 Frontmatter 양식 — atlas-jungwon 예시

```yaml
---
id: atlas-jungwon
kind: continent                              # continent | region | city-map
name: 칠국춘추 대륙
aliases: [중원 대륙, 칠국 대륙]
summary: |
  대진(중원) 중심으로 7개 정치체가 자리한 대륙. 270년 전 통일제국 대진에서
  10년 전 붉은 밤의 변을 거쳐 현재 칠국춘추 시대로.
tags: [wuxia, atlas, continent, current-era]
extent:
  projection: schematic                      # Phase 4: schematic만. cartesian·hex-grid는 Phase N+
  width_units: 7
  height_units: 7
  unit: schematic                            # Phase 4: 의미 없는 단위. Phase N+ km·li
references:                                  # 11 Place 모두 — Phase 3 산출과 1:1
  - place-daejin
  - place-namgung
  - place-seoryang
  - place-bukwon
  - place-namman
  - place-donghae
  - place-jiyu-doshi
  - place-namgung-sega
  - place-western-mountains
  - place-bukwon-grasslands
  - place-namman-jungle
extras:
  era: 현재 (칠국춘추 270년차)               # Phase 5 Era 결합 시 정형
  era_id: ~                                  # Phase 5 외래키 자리 (현재 비움)
  source_section: seven-nations.md §0.1·§0.2·§0.3
---

## 개요
대륙 종론 — §0.1 옮김.

## 칠국 일람
§0.2 일람표 옮김 (markdown table 형식 OK).

## 배치 다이어그램
\`\`\`
                    ┌──────────────────┐
                    │     북 원        │
                    │   (초원/유목)    │
                    │   왕정(오르두)    │
                    └────────┬─────────┘
                             │
         ┌───────────────────┼──────────────────┐
        ...
\`\`\`
(seven-nations.md §0.3 ASCII 다이어그램 그대로 byte-exact 보존)

## 자연 영역 분포
산문 — §0.3 본문 옮김.

## 정치체 분포
산문 — 어느 정치체가 어디에 자리. 표 또는 산문 자유.

## 주요 통로·연결
- 대진 ↔ 남궁: 중원 평원 직통
- 서량 ↔ 자유도시: 서부 산악 협곡 통로 (place-western-mountains 경유)
- 동해 ↔ 자유도시: 해상 항로
- 북원 ↔ 대진: 초원-중원 경계
```

권장 H2 섹션: `## 개요` · `## 칠국 일람` · `## 배치 다이어그램` · `## 자연 영역 분포` · `## 정치체 분포` · `## 주요 통로·연결`. **`## 배치 다이어그램`은 모든 atlas 공통 — view의 핵심 출력**.

### 6.2 `genres/wuxia/forms/atlas.toml` (Phase N 빈 슬롯)

```toml
extends = "atlas"

[[fields.kind.options]]
value = "continent"; label = "대륙"
[[fields.kind.options]]
value = "region"; label = "광역"
[[fields.kind.options]]
value = "city-map"; label = "도시 지도"

[[fields]]
key = "era"
label = "시대"
type = "string"

[[fields]]
key = "era_id"
label = "Era 외래키 (Phase 5)"
type = "era_id"
```

### 6.3 SQLite 스키마

```sql
CREATE TABLE atlases (
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
CREATE INDEX idx_atlases_kind ON atlases(kind);

CREATE VIRTUAL TABLE atlases_fts USING fts5(
    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
);

-- 양방향 atlas ↔ place 인덱스 (Phase 3 자리만 잡았던 테이블 정식 활성)
CREATE TABLE place_atlas_refs (
    atlas_id TEXT NOT NULL,
    place_id TEXT NOT NULL,
    ref_order INTEGER NOT NULL,                -- references 배열 내 위치 보존
    PRIMARY KEY (atlas_id, place_id),
    FOREIGN KEY (atlas_id) REFERENCES atlases(id) ON DELETE CASCADE,
    FOREIGN KEY (place_id) REFERENCES places(id) ON DELETE RESTRICT
);
CREATE INDEX idx_par_place ON place_atlas_refs(place_id);    -- place → atlas 역참조 빠르게
CREATE INDEX idx_par_atlas ON place_atlas_refs(atlas_id);
```

`schema_meta.version = 4` 마이그레이션. Phase 3 v3 DB는 자동 ALTER + atlases·atlases_fts·place_atlas_refs 추가.

### 6.4 환경변수

`NPC_MIND_WORLD_DB` 그대로.

### 6.5 라이브러리

기존 — Phase 0 D2·D3 의존성 회피 원칙 계승.

### 6.6 외래키 매트릭스 (Phase 4 활성)

| 검증 | Phase 1 | Phase 2 | Phase 3 | Phase 4 |
|---|---|---|---|---|
| Group / Person 검증들 | (Phase 별) | (Phase 별) | (Phase 별) | 그대로 |
| Place 검증들 | — | — | (Phase 3) | 그대로 |
| **`Atlas.references` 존재** | — | — | — | **에러** (활성) |
| `Atlas.extras.era_id` 존재 | — | — | — | 텍스트만 (Phase 5 활성) |

### 6.7 atlas-jungwon 작성 가이드

§6.1 양식 그대로 채택. 추가 결정 사항:

**ASCII 다이어그램 보존 정책**:
- §0.3 다이어그램을 코드블록(```...```) 안에 byte-exact 보존
- 마크다운 파서가 코드블록 그대로 body_sections에 저장
- `ascii_diagram_preserved_byte_exact` 단위·통합 테스트로 가드

**references 정렬**:
- Phase 3 산출 11 Place의 ID 알파벳 순 OR §0.3 다이어그램의 좌상→우하 순. **권장: 좌상→우하**(시각적 일관성). 단 결정 시 보고서에 명시.

**extras.era 정책**:
- Phase 4엔 잠정 텍스트만. Phase 5 Era 진입 시 era_id 외래키 활성.
- atlas-jungwon은 "현재 (칠국춘추 270년차)" 또는 "270년차"

## 7. Out of Scope (Phase 4)

- 절대 좌표·SVG·hex grid — Phase N+
- Era overlay (시기별 정치 지도) — Phase 5 Era 결합 시
- View trait 일반화 — Phase 5 Timeline 등장 시 (두 번째 view 패턴 추출)
- distance matrix·세력권 자동 계산 — Phase N+ (좌표 도입 후)
- atlas 간 hierarchy (continent → region → city-map drilldown) — Phase 5+
- 다중 atlas (예: 칠국 대륙 + 동해 군도 별도 atlas) — Phase 5+
- AI 자동 다이어그램 생성 — Phase N+
- Mind Studio worldbuilding UI 패널 — Phase N+
- gameplay 다리 (Scene·Beat·관계 시드) — Phase 5+

## 8. 코드 위치 가이드

작업 시작 5분에 읽을 곳:

| 위치 | 무엇을 볼지 |
|---|---|
| `Cargo.toml` `[features]` | feature 게이팅 |
| `src/domain/world/place.rs` (Phase 3) | aliases·spatial·외래키 패턴 — Atlas의 references와 유사 |
| `src/worldbuilding/markdown/place.rs` (Phase 3) | 마크다운 파서 패턴. **코드블록(```...```) 처리 주의** |
| `src/adapter/sqlite_world.rs` (Phase 3) | migrate_v3 + place_atlas_refs 자리 — Phase 4에서 정식 활성 |
| `src/bin/world_load.rs` (Phase 3) | 외래키 검증 흐름 — Atlas.references도 같은 패턴 |
| `src/bin/mind-studio/handlers/world_places.rs` (Phase 3) | MCP 도구 등록 — atlases도 같은 패턴 |
| `task-phase3-place-vertical-slice.archive.md` | 보존본 안의 Atlas 양식·SQLite·외래키 패턴 — 본 TASK 작성 시 직접 참고 |

## 9. 시작 체크리스트

1. `CLAUDE.md` + Phase 0~3 산출 빠르게 훑기
2. **`task-phase3-place-vertical-slice.archive.md`의 Atlas 부분** 통독 — 양식·SQLite·외래키 패턴 거의 그대로 활용 가능
3. **`wuxia-core/docs/world/seven-nations.md` §0.1·§0.2·§0.3** 통독 — Phase 4 핵심 입력
4. Phase 3 산출 (`projects/chilguk-chunchu/world/place/*.md` × 11) ID 목록 확인 — references에 들어갈 것
5. Atlas 도메인 + AtlasExtent + view 메서드 + 마크다운 파서 + 단위 테스트 (Step 1·2)
6. SqliteWorldStore migrate_v4 + atlases·place_atlas_refs + 라운드트립 테스트
7. world-load 확장 — Atlas.references 외래키 활성
8. **atlas-jungwon 변환** (§0.3 다이어그램 보존 핵심) → ★체크포인트 1★ 보고 → **commit pause**

## 10. 리뷰 채널

체크포인트 1·2 보고서를 디렉터(사용자)가 Cowork 세션에 복붙. 형식:
- **Done** · **Diff** · **데모 명령** · **결정** · **막힌 것** · **다음 의견**
- **ASCII 다이어그램 보존 검증**을 본문에 명시 (원본·복원 비교)
- view 메서드 호출 결과 (places_in·settlements_in·geographies_in·adjacent_to)를 표 형식으로

보고서 파일명:
- 체크포인트 1: `docs/tasks/phase4-checkpoint1-report.md`
- 체크포인트 2: `docs/tasks/phase4-checkpoint2-report.md`
