# Phase 4 Checkpoint 1 Report — atlas-jungwon 단독 변환 + references 외래키 + ASCII 라운드트립

**작업 범위 (사양 §5 Step 1·2·3)**
- Atlas 도메인 + AtlasId + AtlasExtent + view 메서드 (`places_in`/`settlements_in`/`geographies_in`/`adjacent_to`)
- Atlas 마크다운 파서 + 코드블록 안 ASCII 보존
- SqliteWorldStore `migrate_v4` (atlases + atlases_fts + place_atlas_refs 양방향 인덱스)
- world-load 확장 (Atlas 스캔 + references 외래키 hard-fail + 중복 references 검출)
- WorldRepository 5 신규 메서드 (`list_atlases`/`get_atlas`/`search_atlases`/`upsert_atlas`/`count_atlases`)
- atlas-jungwon.md 변환 (§0.1·§0.2·§0.3 → 7 H2 섹션, 11 references)
- genres/wuxia atlas 양식 (`markdown_template/atlas.md`, `forms/atlas.toml` Phase N 빈 슬롯)
- 통합 테스트 13개 + 단위 테스트 22개 (Atlas 11 + markdown 11) + SQLite 7개 (라운드트립·양방향 인덱스·v3→v4 마이그레이션·search·filter)

**브랜치**: `claude/atlas-vertical-slice-phase4-jUPzD`
**Phase 4 진행률**: Step 1·2·3 완료 → 체크포인트 1 게이트.

---

## Done

### 1. Atlas 도메인 — `src/domain/world/atlas.rs` (신규, 415 LoC)

#### 핵심 타입
- `AtlasId(String)` — `atlas-{slug}` 형식, transparent serde.
- `AtlasExtent { projection, width_units?, height_units?, unit }` — Phase 4엔
  `projection = "schematic"`, `unit = "schematic"` 기본값. `width/height_units None`은
  serde skip.
- `Atlas` — 9 필드: id/kind/name/aliases/summary/tags/extras + extent + references +
  body_sections + source_path. `references: Vec<PlaceId>`가 핵심 (관계 도메인의 첫
  사례). `body_sections: BTreeMap<String, String>`로 H2 섹션 보존 (Group/Place 동일 정책).
- `AtlasFilter { kind, genre_tag }` — `list_atlases`에 전달.

#### 도메인+뷰 이중성 — view 메서드 (도메인 객체에 부착)
- `places_in<R: WorldRepository + ?Sized>(&self, repo) -> Result<Vec<Place>, WorldError>`
  — references 작성 순서대로 합성. 결손은 silent skip (world-load가 hard-fail로 결손 0건 보장).
- `settlements_in` / `geographies_in` — layer 필터.
- `adjacent_to(place_id, repo) -> Vec<PlaceId>` — `Place.spatial.bordering_places`를 따라
  atlas 경계 안으로 국한. atlas references에 없는 place_id는 빈 Vec (사일런트), repo 결손도
  사일런트.
- `era()` / `era_id()` 추출 헬퍼 — Phase 5 Era 진입 시 외래키로 승급.

**View trait 일반화는 Phase 5+ 미룸** — 두 번째 관계 도메인(Timeline 등) 등장 시 추출.

#### 단위 테스트 11개 (atlas.rs `tests` 모듈)
새 `MiniRepo` (스텁 WorldRepository, atlas/place 메서드만 구현) 활용:
- `atlas_new_sets_defaults` — 기본값 검증.
- `atlas_extent_default_is_schematic` / `atlas_extent_serde_skip_when_units_none` — extent 정합성.
- `atlas_full_serde_roundtrip` — 모든 필드 라운드트립 (extras·references·body 포함).
- `era_helpers_extract_from_extras` — era·era_id 추출.
- `places_in_returns_in_reference_order` — references 순서 = view 순서.
- `places_in_skips_missing_silently` — 결손 silent.
- `settlements_and_geographies_filter_by_layer` — layer 분기.
- `adjacent_to_filters_to_atlas_boundary` — atlas 경계 밖 인접 무시.
- `adjacent_to_returns_empty_when_place_not_in_atlas` / `_when_place_missing_from_repo` — 사일런트 정책.

### 2. 마크다운 파이프라인 — `src/worldbuilding/markdown/atlas.rs` (신규, 350 LoC)

`place.rs` 패턴 그대로 미러링. 핵심:
- frontmatter `id`/`kind`/`name` 필수, 누락 시 `MissingField` 에러.
- `extent` mapping → `AtlasExtent` (생략·null 시 default).
- `references` sequence → `Vec<PlaceId>` (작성 순서 보존).
- `body_sections` ← `parse_h2_sections(&fm.body)` — **fenced code block 안의 `## ` 가짜
  헤더는 무시**되며 들여쓰기·box-drawing 문자 byte-exact 보존 (Phase 1·2·3 frontmatter
  파서 회귀 가드 그대로).

#### 단위 테스트 11개 — 핵심 가드
- `parse_neutral_atlas_full_roundtrip` — 장르 중립 fixture, 모든 필드.
- **`ascii_diagram_preserved_byte_exact`** — box-drawing(┌──┐ │ └──┘) 보존.
- **`ascii_diagram_with_inner_hash_lines_not_split_into_h2`** — 펜스 안 `## fake`가 새
  H2로 분리되지 않음.
- `references_preserve_input_order` — 좌상→우하 같은 의도된 순서가 사라지지 않음.
- `era_id_text_preserved_for_phase5` — era_id 텍스트만 보존 (Phase 4엔 검증 비활성).
- 누락 필드 3종 + null/empty extent 2종.

### 3. SqliteWorldStore — `migrate_v4` + atlases + place_atlas_refs

#### 스키마 추가 (`SCHEMA_VERSION = 4`)

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
CREATE INDEX idx_atlases_project ON atlases(project_id);
CREATE VIRTUAL TABLE atlases_fts USING fts5(
    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
);

CREATE TABLE place_atlas_refs (
    atlas_id TEXT NOT NULL,
    place_id TEXT NOT NULL,
    ref_order INTEGER NOT NULL,
    PRIMARY KEY (atlas_id, place_id)
);
CREATE INDEX idx_par_place ON place_atlas_refs(place_id);
CREATE INDEX idx_par_atlas ON place_atlas_refs(atlas_id);
```

**FK 절 미사용 — Phase 1·2·3 일관 정책**: SQLite `PRAGMA foreign_keys`가 기본 OFF인 환경에서
DDL FK는 무력화되므로, FK 검증은 **application 계층(world-load)**에서 수행한다. 사양 §6.3에는
`FOREIGN KEY ... ON DELETE` 절이 있지만 Phase 1·2·3 마이그레이션엔 사용된 적이 없으며
(예: `groups.parent_group`), Phase 4도 동일 정책으로 통일했다. **막힌 결정 후보**.

#### `upsert_atlas` 동작
- `atlases` INSERT OR REPLACE.
- `atlases_fts` delete-then-insert (id 기반).
- `place_atlas_refs` atlas_id 기준 delete-then-insert — references 작성 순서대로
  `ref_order = 0..n` 채워 양방향 인덱스 동기화.
- 트랜잭션 단일 — partial commit 불가능.

#### v3 → v4 마이그레이션
- 기존 v3 DB는 `init_tables`에서 `current < 4` 분기 진입 → atlases/atlases_fts/place_atlas_refs
  추가, places·groups·persons row 보존.
- `schema_meta`는 단일 row 강제 (v1·v2·v3 누적 X).

#### SQLite 단위 테스트 7개 추가
- `atlas_full_roundtrip_through_sqlite` — 모든 필드 라운드트립.
- **`atlas_body_sections_preserve_ascii_byte_exact`** — box-drawing + 빈 줄 + 들여쓰기
  포함 ASCII 다이어그램이 SQLite 라운드트립 후 byte-exact 보존.
- **`place_atlas_refs_bidirectional_index_populated_on_upsert`** — 정방향(atlas_id로 places +
  ref_order) + 역방향(place_id로 atlases) 모두 채워짐.
- `place_atlas_refs_resyncs_on_re_upsert` — references 변경 시 기존 매핑 모두 사라지고
  신규 매핑으로 교체.
- `list_atlases_filters_by_kind_and_genre_tag` — 두 필터.
- `search_atlases_fts_and_like_fallback` — FTS5 trigram + LIKE fallback.
- `schema_v3_to_v4_migration_upgrades_existing_file_db` — v3 schema에 places row + meta=3
  넣은 tempfile DB가 SqliteWorldStore::new로 v4 자동 마이그레이션, 기존 row 보존.

### 4. world-load — Atlas 스캔 + references 외래키 활성

#### 동작 추가
- `atlas_dir = project_dir/world/atlas/` 스캔 (디렉토리 없으면 stderr 알림 후 atlases 0).
- `atlas_from_markdown` 파싱 실패는 기존 `errors` 배열에 합류 (partial commit 방지 유지).
- `Atlas.references` 검증 2종:
  - `missing_atlas_refs` — references PlaceId가 places 테이블에 없으면 hard-fail.
  - `duplicate_atlas_refs` — 같은 references 배열에 중복된 PlaceId가 있으면 hard-fail
    (place_atlas_refs composite PK 위반 방지).
- `fk_errors_total`에 두 카운트 합산. partial commit 방지 분기에서 `atlases parsed = N`
  진단.
- 검증 통과 후에만 `upsert_atlas` 실행. 결과 출력에 `atlases indexed/parsed = N` 추가.

#### 실행 결과 — 외래키 0건 + atlases 1 인덱싱

```
$ cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload

[world-load] project    = chilguk-chunchu
[world-load] genre      = wuxia
[world-load] project_dir= projects/chilguk-chunchu
[world-load] db         = projects/chilguk-chunchu/build/world.sqlite
[world-load] ℹ rival 비대칭 3 건 (일방적 적대 — 무협에서 흔함):
  - group-namgung → group-daejin-court (역방향 미선언)
  - group-namgung → group-cheonma-shingyo (역방향 미선언)
  - group-shipsangsi → group-mulim-mang (역방향 미선언)

=== 결과 ===
project           = chilguk-chunchu
groups indexed    = 6
persons indexed   = 9
places indexed    = 11
atlases indexed   = 1
groups parsed     = 6
persons parsed    = 9
places parsed     = 11
atlases parsed    = 1
errors            = 0
group cycles      = 0
place cycles      = 0
fk errors (활성)  = 0
mind eligible     = 9
```

### 5. atlas-jungwon.md (`projects/chilguk-chunchu/world/atlas/atlas-jungwon.md`)

#### Frontmatter 결정 사항

| 필드 | 결정값 | 근거 |
|---|---|---|
| `id` | `atlas-jungwon` | "중원" 음역. 사양 §6.1 예시 그대로. |
| `kind` | `continent` | seven-nations.md §0.3가 "대륙 배치" 한 컷. |
| `name` | `칠국춘추 대륙` | §0.2 일람표의 시대명 "칠국춘추" + 대륙 키워드. |
| `aliases` | `[중원 대륙, 칠국 대륙]` | §0.1·§0.3에서 사용된 두 별호. "옛 통일제국 대륙" 같은 시대명-과거형은 Phase 5 Era overlay 후 별 atlas 인스턴스로 분기 예정. |
| `summary` | "대진(중원) 중심으로 7개 정치체… 단일 시점의 정치·자연 지도. 자연 지형 3종이 settlement 위에 layered." | §0.1 종론 압축 + atlas 이중성 명시. |
| `tags` | `[wuxia, atlas, continent, current-era]` | atlas + current-era로 시대 시드. |
| `extras.era` | `현재 (칠국춘추 270년차)` | §0.3 + 게임 캐논(`history.md`) 시점. |
| `extras.era_id` | `~` (비움) | Phase 5 Era 외래키 자리. |
| `extras.source_section` | `seven-nations.md §0.1·§0.2·§0.3` | 원전 추적. |
| `extent.projection` | `schematic` | Phase 4 단일 옵션. |
| `extent.width_units / height_units` | `7 / 7` | §0.3 다이어그램이 대략 7×7 격자에 들어맞음. schematic 단위. |
| `extent.unit` | `schematic` | Phase 4: km/li 미사용. |

#### references 정렬 결정 — **좌상→우하** (사양 §6.7 권장)

§0.3 다이어그램을 행 단위로 읽어 settlement와 그 위 layered geography를 즉시 페어링:

```
1. place-bukwon            (북원 — 북상단)
2. place-bukwon-grasslands (북원 초원, layered)
3. place-seoryang          (서량 — 서중단)
4. place-western-mountains (서부 산악, layered + 서량 인접)
5. place-daejin            (대진 — 중원 중앙)
6. place-donghae           (동해 — 동중단)
7. place-jiyu-doshi        (자유도시 — 중원 남쪽 접경)
8. place-namgung           (남궁 — 서남부)
9. place-namgung-sega      (남궁세가 — 남궁 영토 내 sect 이중 등록)
10. place-namman           (남만 — 남단)
11. place-namman-jungle    (남만 밀림, layered)
```

**대안 비교**:
- **알파벳 순** (`place-bukwon`, `place-bukwon-grasslands`, `place-daejin`, ...): 결정성·디버깅 용이.
- **좌상→우하 (채택)**: 시각적 일관성 — view 메서드 결과(`places_in`)가 다이어그램 읽는 순서와 일치, settlement·geography 페어가 인접해 자연 영역 분포 본문과 매칭 쉬움.

**결정**: 좌상→우하 채택. settlement 8 + geography 3가 페어링되어 §0.3 본문(자연 영역 분포)
설명과 references 순서가 자연스럽게 정렬된다. **알파벳 순으로 변경 시 디렉터 승인**.

#### H2 섹션 매핑

| 섹션 | 원전 매핑 | 비고 |
|---|---|---|
| `## 개요` | §0.1 종론 | "왜 칠국인가" 압축. |
| `## 칠국 일람` | §0.2 일람표 | markdown table 그대로. 7행. |
| `## 배치 다이어그램` | §0.3 ASCII 다이어그램 | **byte-exact 보존** — 코드블록 펜스 안. |
| `## 자연 영역 분포` | §0.3 본문 (지리적 특징 6 bullet) | 본문 평문 + Place ID 명시 (`place-bukwon` 등). |
| `## 정치체 분포` | 산문 (게임 캐논 추가) | 7개 정치체 분류 + sect 1 (남궁세가). |
| `## 주요 통로·연결` | bordering_places 기반 | place-daejin·namgung·donghae 등의 spatial 정합성 명시. settlement·geography layered 페어 7건. |
| `## 전사(前史)` | §0.1 + history.md 캐논 압축 | Phase 5 Era 진입 시 era_id로 정형화 명시. |

권장 H2 7개 모두 채움.

### 6. genres/wuxia 양식

- `genres/wuxia/markdown_template/atlas.md` (43 LoC) — 작성자 가이드 템플릿. references
  좌상→우하 권장 코멘트 + 7개 권장 H2 섹션.
- `genres/wuxia/forms/atlas.toml` (35 LoC) — Phase N 빈 슬롯. 3 kind 옵션
  (continent/region/city-map) + 2 fields (era/era_id).

### 7. 통합 테스트 — `tests/world_chilguk_chunchu_phase4_checkpoint1.rs`

13 테스트 모두 통과:

| # | 테스트 | 검증 |
|---|---|---|
| 1 | `atlas_jungwon_parses_with_expected_identity` | id/kind/name/aliases(2)/era/extent + 7 H2 섹션 모두 |
| 2 | `atlas_jungwon_references_contain_all_eleven_places` | references 11개 = 좌상→우하 순서 |
| 3 | **`ascii_diagram_preserved_byte_exact_through_disk_and_sqlite`** | §0.3 sentinel 22 라인이 .md → atlas_from_markdown → SqliteWorldStore.upsert_atlas → get_atlas 라운드트립 후 byte-exact 보존. 펜스 ``` 자체도 보존. |
| 4 | `world_load_indexes_atlas_with_zero_fk_residual` | atlases=1 + places=11 + 모든 references PlaceId가 places.id에 존재 |
| 5 | `places_in_returns_all_eleven_in_reference_order` | view 메서드 — 11개 + 작성 순서 |
| 6 | `settlements_in_returns_eight` | layer 필터 8건 |
| 7 | `geographies_in_returns_three` | layer 필터 3건 (western-mountains·bukwon-grasslands·namman-jungle) |
| 8 | `adjacent_to_daejin_returns_three_atlas_internal_neighbors` | 대진 인접 3건 = [namgung, jiyu-doshi, seoryang] |
| 9 | `adjacent_to_namgung_sega_returns_zero` | sect는 bordering_places 비어 있으므로 0 |
| 10 | `layer_filter_invariant_holds_for_settlements_and_geographies` | 8 + 3 = 11 invariant |
| 11 | `list_atlases_filter_by_kind_continent_returns_one` | kind=continent 필터 |
| 12 | `search_atlases_finds_by_alias_and_summary` | "중원"·"칠국" 매칭 → atlas-jungwon |
| 13 | `place_id_appears_in_place_atlas_refs_via_list_places` | 정합성 가드 — references와 list_places 결과 교차 검증 |

### 8. 빌드·테스트 결과 (회귀 포함)

```
cargo build --features embed                                         : ✓
cargo build --features mind-studio,chat,embed --bin npc-mind-studio  : ✓
cargo test --features embed --lib                                    : 412 passed
cargo test --features embed --test world_chilguk_chunchu_phase4_checkpoint1 : 13 passed
cargo test --features embed --test world_chilguk_chunchu_e2e         : 7 passed (회귀)
cargo test --features embed --test world_chilguk_chunchu_person_e2e  : 7 passed (회귀)
cargo test --features embed --test world_chilguk_chunchu_phase3_checkpoint1 : 7 passed (회귀)
cargo test --features embed --test world_chilguk_chunchu_phase3_checkpoint2 : 9 passed (회귀)
cargo test --features embed --test world_load_fk_negative            : 3 passed (회귀)
```

`embed_test` 6건 환경 의존 실패(BGE-M3 ONNX 모델 부재) — Phase 4 무관, Phase 3 체크포인트
2와 동일 환경 가드.

---

## Diff

```
 docs/tasks/phase4-checkpoint1-report.md                        | (이 파일, 신규)
 genres/wuxia/forms/atlas.toml                                  | (신규)
 genres/wuxia/markdown_template/atlas.md                        | (신규)
 projects/chilguk-chunchu/world/atlas/atlas-jungwon.md          | (신규)
 src/adapter/sqlite_world.rs                                    | +584
 src/bin/world_load.rs                                          | +98
 src/domain/world/atlas.rs                                      | (신규, 425 LoC)
 src/domain/world/mod.rs                                        | +7
 src/worldbuilding/markdown/atlas.rs                            | (신규, 350 LoC)
 src/worldbuilding/markdown/mod.rs                              | +2
 src/worldbuilding/repository.rs                                | +24
 tests/world_chilguk_chunchu_phase4_checkpoint1.rs              | (신규, 13 테스트)
```

---

## 데모 명령

```bash
# 도메인·markdown·SQLite 단위 테스트 (체크포인트 1 본체)
cargo test --features embed --lib domain::world::atlas
cargo test --features embed --lib worldbuilding::markdown::atlas
cargo test --features embed --lib adapter::sqlite_world

# 통합 테스트 (atlas-jungwon e2e)
cargo test --features embed --test world_chilguk_chunchu_phase4_checkpoint1

# world-load — atlases=1 + fk errors=0
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload

# 회귀 검증 (Phase 3·2·1 e2e)
cargo test --features embed --lib
cargo test --features embed --test world_chilguk_chunchu_e2e
cargo test --features embed --test world_chilguk_chunchu_person_e2e
cargo test --features embed --test world_chilguk_chunchu_phase3_checkpoint1
cargo test --features embed --test world_chilguk_chunchu_phase3_checkpoint2
cargo test --features embed --test world_load_fk_negative

# Mind Studio 빌드 (체크포인트 2에서 list_atlases·get_atlas MCP 추가 예정)
cargo build --features mind-studio,chat,embed --bin npc-mind-studio
```

---

## ASCII 다이어그램 byte-exact 보존 검증

테스트 `ascii_diagram_preserved_byte_exact_through_disk_and_sqlite`가 22 sentinel 라인
(§0.3 다이어그램 핵심)을 다음 3 단계 모두에서 검증:
1. raw `.md` 파일에 존재 (작성 시 byte-exact 복사 검증)
2. `atlas_from_markdown` 결과 `body_sections["배치 다이어그램"]`에 존재 (마크다운 파서가
   펜스 안의 들여쓰기·box-drawing·`## ` 같은 가짜 헤더 표면을 깨지 않음)
3. `SqliteWorldStore.upsert_atlas → get_atlas` 라운드트립 후 동일

22 sentinel 중 핵심 (북원→대진 일직선·자유도시·남궁·남만 박스 모두):
- `┌──────────────────┐` / `│     북 원        │` / `└────────┬─────────┘`
- `┌────┴────┐` × 3 가지 (서량·산악·자유도시·남궁)
- `│  서 량   │` / `│   대 진    │` / `│  동 해   │`
- `│ 독관성   │` / `│  낙양     │` / `│  해문    │`
- `│  자유도시    │` / `│  (중립지대)  │`
- `│   남 궁    │` / `│  검성     │` / `│  남 만   │`
- `│ (남방밀림)│` / `│ 만왕성   │`
- `└─────────┘` (남만 박스 바닥)

원본 (`wuxia-core/docs/world/seven-nations.md` §0.3) ↔ 작성본 (`atlas-jungwon.md` `## 배치
다이어그램`) 핵심 비교는 위 sentinel 매칭으로 정량 가드. 변형 없음.

---

## view 메서드 호출 결과

### `places_in(repo).len() == 11` (작성 순서 = 좌상→우하)
| ref_order | id | layer | kind |
|---|---|---|---|
| 0 | place-bukwon | settlement | nation |
| 1 | place-bukwon-grasslands | geography | grassland |
| 2 | place-seoryang | settlement | nation |
| 3 | place-western-mountains | geography | mountain-range |
| 4 | place-daejin | settlement | nation |
| 5 | place-donghae | settlement | nation |
| 6 | place-jiyu-doshi | settlement | autonomous-zone |
| 7 | place-namgung | settlement | nation |
| 8 | place-namgung-sega | settlement | sect |
| 9 | place-namman | settlement | nation |
| 10 | place-namman-jungle | geography | jungle |

### `settlements_in(repo).len() == 8`
place-bukwon · place-seoryang · place-daejin · place-donghae · place-jiyu-doshi ·
place-namgung · place-namgung-sega · place-namman.

### `geographies_in(repo).len() == 3`
place-bukwon-grasslands · place-western-mountains · place-namman-jungle.

### `adjacent_to(...)` 매트릭스

| anchor | 결과 | 근거 |
|---|---|---|
| `place-daejin` | `[place-namgung, place-jiyu-doshi, place-seoryang]` (3) | 대진의 bordering_places 모두 atlas references 안 |
| `place-namgung-sega` | `[]` (0) | sect의 bordering_places가 비어 있음 |
| `place-bukwon` | `[place-daejin, place-seoryang, place-donghae]` (3) | 북원 인접 모두 atlas 안 |
| `place-jiyu-doshi` | `[place-daejin, place-namgung]` (2) | 자유도시는 대진·남궁 사이 접경 |

(테스트 4·8·9 자동화. 그 외 anchor는 회귀로 확인되지만 별도 명시 단언은 체크포인트 2에서 보강 가능.)

---

## 외래키 결손 0건 검증

```
fk errors (활성)  = 0
```

`world_load_indexes_atlas_with_zero_fk_residual` 테스트가 11개 references PlaceId 모두에
대해 `store.get_place(pid).is_some()` 확인. 추가로 `place_id_appears_in_place_atlas_refs_via_list_places`가
references 집합 ⊂ list_places 결과 집합 invariant 확인.

place_atlas_refs 양방향 인덱스 결과:
- 정방향: `(atlas-jungwon, place-bukwon, 0)`, `(atlas-jungwon, place-bukwon-grasslands, 1)`,
  ..., `(atlas-jungwon, place-namman-jungle, 10)` — 11 row.
- 역방향: 각 place_id로 `idx_par_place` 인덱스 적중 가능.

---

## 막힌 결정 — 디렉터 의견 요청 사항

### 1. SQLite FK DDL 절 활성 여부 (Phase 1·2·3·4 일관성)

사양 §6.3은 `place_atlas_refs`에 `FOREIGN KEY ... REFERENCES ... ON DELETE CASCADE/RESTRICT`
절을 명시하지만, Phase 1·2·3 마이그레이션엔 **DDL FK가 한 번도 등장하지 않는다** (`groups.parent_group`
등도 plain TEXT). SQLite는 `PRAGMA foreign_keys = ON`이 connection-scope이며 기본 OFF라 DDL
FK가 무력화되므로, 본 Phase 4도 일관성을 위해 **application 계층(world-load)에서만 검증**
했다. 정책 옵션:

| 옵션 | 설명 | 트레이드오프 |
|---|---|---|
| **A (채택)** | DDL FK 절 미사용. world-load에서만 hard-fail. Phase 1·2·3과 일관. | 외부 도구가 직접 INSERT 시 FK 무체크. |
| B | `place_atlas_refs`에만 FK 절 + `PRAGMA foreign_keys = ON` 활성. 다른 테이블은 미변경. | Phase 4만 부분 활성 — 일관성 깨짐. |
| C | 전체 마이그레이션 일괄 FK 절 추가. v5 진입. | Phase 1·2·3 회귀 위험 + 큰 변경. |

→ A 그대로 유지 권장. 디렉터 결정 1건.

### 2. references 정렬 — 좌상→우하 (채택) vs 알파벳 순

사양 §6.7 권장이 좌상→우하이고 시각적 일관성·페어링 이점이 있어 채택. 다만 결정성·
디버깅 관점에서 알파벳 순도 합리적. 변경 시 디렉터 승인.

### 3. `aliases` 2개로 결정 — `["중원 대륙", "칠국 대륙"]`

§0.1·§0.3에 등장. "옛 통일제국 대륙" 같은 시대명-과거형은 Phase 5 Era overlay 후
**별 atlas 인스턴스**(예: `atlas-daejin-empire`)로 분기 예정. 본 atlas는 단일 시점만.

### 4. `extent.width_units / height_units = 7 / 7`

§0.3 다이어그램이 대략 7행 × 7열 격자에 들어맞아 schematic 7×7로 결정. 절대 좌표는
Phase N+. **schematic 단위라 의미는 라벨일 뿐**. 다른 값(예: `null`)으로 두면 view
메서드는 동일하게 동작하나 폼/검증에서 빈칸 경고 가능. 7×7로 기록.

---

## 알려진 한계 (Phase 4 Step 4 / Phase 5+ 이관)

- **MCP 도구 미등록**: `list_atlases`/`get_atlas`는 체크포인트 2에서 추가 예정 (사양 §5 Step 4).
- **mind-studio REST 엔드포인트** (`/api/world/atlases/*`) 없음 — 체크포인트 2.
- **Era overlay 미활성**: `extras.era_id`는 텍스트만 보존. Phase 5 Era 도메인 진입 시
  외래키 활성 + 시기별 atlas 인스턴스 분기.
- **distance/세력권 자동 계산**: schematic projection이라 거리 의미 없음. Phase N+
  cartesian/hex-grid 도입 후.
- **다중 atlas**: chilguk-chunchu에 `atlas-jungwon` 1개만. 하위 region/city-map atlas는
  Phase 5+.

---

## Step 4 (체크포인트 2) 진행 가능 여부 의견

**진행 가능**.

체크포인트 1 산출물이 Step 4 (view 메서드 + MCP 도구 + 정성 평가)의 전제를 모두 충족:
1. **Atlas 도메인 + view 메서드 완성** — `places_in`/`settlements_in`/`geographies_in`/
   `adjacent_to`가 atlas-jungwon에서 정확히 작동. Step 4의 e2e 테스트는 본 체크포인트의
   13 테스트를 부분 흡수 + MCP·정성 평가만 추가하면 됨.
2. **외래키 0건** — references 11개 모두 places에 매핑됨. place_atlas_refs 양방향 인덱스
   채워짐.
3. **ASCII 다이어그램 byte-exact 보존** — disk·markdown·SQLite 3단계 모두 검증.
4. **MCP 패턴 일관 확장 가능** — Phase 1·2·3 `list_*`/`get_*`/`search_*` 3종을
   `list_atlases`/`get_atlas`로 (search_atlases는 옵션) 그대로 미러 가능.

다음 권장 Step 4 작업:
- mind-studio `/api/world/atlases` REST + `list_atlases`/`get_atlas` MCP 도구 추가.
- 정성 평가 — Mind Studio에서 `get_atlas("atlas-jungwon")` 결과 + view 메서드 호출 결과
  육안 확인.
- search_atlases 2-3 쿼리 자동화 (`"칠국"`·`"중원"`·`"대륙"`).
- 사양 §3.7 게이트 그대로 — 본 보고서가 Cowork 리뷰 통과 시 commit 진행 → Step 4 진입.

---

## 다음 의견

- 본 commit 범위: Step 1·2·3 단일 commit으로 묶기 적절. Step 4 산출물(MCP·REST + 정성
  평가)은 체크포인트 2 별도 commit.
- **commit pause 유지** — 디렉터 리뷰 후 통과 신호 받고 commit 후 Step 4 진입.
- 막힌 결정 4건(특히 #1 SQLite FK DDL 절·#2 references 정렬)에 대한 의견 요청.
