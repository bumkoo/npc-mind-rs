# Phase 3 Checkpoint 1 Report — Place 두 layer 변환 + 외래키 활성

**작업 범위 (사양 §5 Step 1·2·3)**
- Place 도메인 (`PlaceLayer`/`Spatial`/`PlaceFilter` + cycle 검증) 채움
- 마크다운 파서 + SQLite `migrate_v3` + `places`/`places_fts` 테이블
- world-load 외래키 활성 (Phase 1·2의 텍스트 보존 → 에러 승급)
- **`projects/chilguk-chunchu/world/place/`** 두 layer 1쌍 변환:
  - `place-daejin.md` — settlement/nation
  - `place-western-mountains.md` — geography/mountain-range (신규 1차 작성)
- 통합 테스트 7건 (E2E 라운드트립 + layer 분기 검증)
- 장르 템플릿 `genres/wuxia/markdown_template/place-{settlement,geography}.md` + `forms/place.toml`

**브랜치**: `claude/place-vertical-slice-phase3-6bp5c`
**Phase 3 진행률**: Step 1·2·3 완료. Step 4·5는 체크포인트 1 통과 후 착수.

---

## Done

### 1. 도메인 + 포트
- `src/domain/world/place.rs` — `Place`/`PlaceId`/`PlaceLayer`(serde lowercase)/`Spatial`/`PlaceFilter` + `detect_parent_place_cycle` (group의 동일 알고리즘 재사용, canonical rotation, dangling parent 무시)
- `src/domain/world/mod.rs` — re-export 추가
- `src/worldbuilding/repository.rs` — `WorldRepository`에 `list_places`/`get_place`/`search_places`/`upsert_place`/`count_places` 추가
- `WorldError::ParentCycle` variant 그대로 재사용 (path string으로 group/place 구분)

### 2. 마크다운 파서
- `src/worldbuilding/markdown/place.rs` — Settlement·Geography 두 양식을 같은 파서로 처리. layer enum, spatial mapping, extras (sequence·string·nested map 모두) 보존. Group/Person 파서와 동일한 `serde_yaml` + `parse_h2_sections` 패턴.
- `PlaceMarkdownError` — `MissingField`/`TypeMismatch`/`InvalidLayer`(value 포함) — 진단 친화적
- 9개 단위 테스트 (settlement/geography fixture, layer-missing, invalid layer, null/empty spatial, parent_place 파싱, sect 이중 등록 시연)

### 3. SQLite 스키마
- `migrate_v3` (v2→v3): `places` 테이블 + `places_fts` (FTS5 trigram) + `parent_place` 캐시 컬럼 + 4 인덱스(layer/kind/parent/project). `place_atlas_refs` 자리는 Phase 4(Atlas)로 미루고 미생성.
- `SCHEMA_VERSION = 3` (단일 row 유지 검증 테스트도 v3로 갱신)
- `WorldRepository` impl — upsert·list(layer/kind/parent_place/genre_tag 필터)·get·search(FTS5+LIKE fallback)·count
- 8개 SQLite 단위 테스트 + 2개 마이그레이션 테스트 (v1→v2, v2→v3)

### 4. world-load 외래키 활성
- `world/place/*.md` 스캔 → places upsert (검증 통과 시)
- **활성된 외래키 (모두 hard-fail, partial commit 방지)**:
  - `Group.headquarters` ↔ `places.id`
  - `Person.birthplace`/`current_location` ↔ `places.id`
  - `Place.spatial.parent_place` cycle (DFS, canonical rotation)
  - `Place.spatial.bordering_places` ↔ `places.id`
  - `Place.spatial.geography_refs` ↔ `places.id` + **target layer가 `Geography`이어야** (settlement→geography 의미 보장)
  - `Place.extras.controlling_group` ↔ `groups.id` (sect kind만 hard-fail; 다른 kind에서 명시되면 검증은 하되 fatal 아님)
- `cycle_errors_total` 추가 — fatal_cycle 분기로 hard-fail
- 결과 블록에 `places parsed`/`places indexed`/`group cycles`/`place cycles` 필드 추가
- 통합 테스트 결과: **24 FK 결손 → DB 미수정** (의도된 상태 — 사양 §5 Step 3 예측치)

### 5. 통합 테스트 (`tests/world_chilguk_chunchu_phase3_checkpoint1.rs`)
- daejin·western-mountains 각각 정체성 검증
- **layer 분기**: 두 Place가 distinct extras key set 사용 (settlement → capital/polity/controlling_group; geography → terrain_type/climate/hazards/signature_features)
- SQLite 라운드트립 — 모든 필드 (extras nested map/array 포함) 보존
- `list_places(layer=Settlement)` → 1건 / `list_places(layer=Geography)` → 1건
- search_places: alias("중원 황도")·body("산악"·"낙양") 매칭 모두 성공
- `parent_place` cycle 검출: 두 Place 모두 parent 없음 → 0건
- `dump_places_json` (#[ignore]) — 보고서용 JSON 재생산용

### 6. 빌드·테스트 결과
- `cargo build --features embed`: ✓ 성공
- `cargo test --features embed --lib`: **379 passed, 0 failed**
- `cargo test --features embed --tests` (Phase 3 신규 + 회귀):
  - `world_chilguk_chunchu_phase3_checkpoint1`: **7 passed**
  - `world_chilguk_chunchu_e2e`: 7 passed
  - `world_chilguk_chunchu_person_e2e`: 7 passed
  - `dispatch_v2_test`: 36 passed / `dialogue_*`: 14 passed / `director_test`: 30 passed
  - **사전 환경 의존 실패: `embed_test` 6건** — BGE-M3 ONNX 모델 파일(`../models/bge-m3`) 부재로 `OrtEmbedder::new`가 실패. Phase 3 변경 이전 stash 상태에서도 동일 실패 재현됨 (사전 존재 환경 의존, Phase 3 무관).

---

## Diff

```
 genres/wuxia/forms/place.toml                      | 104 ++++
 genres/wuxia/markdown_template/place-geography.md  |  38 ++
 genres/wuxia/markdown_template/place-settlement.md |  40 ++
 projects/chilguk-chunchu/world/place/place-daejin.md            |  65 +++
 projects/chilguk-chunchu/world/place/place-western-mountains.md |  63 +++
 src/adapter/sqlite_world.rs                        | 590 ++++++++++++++++++++-
 src/bin/world_load.rs                              | 267 ++++++++--
 src/domain/world/mod.rs                            |   3 +
 src/domain/world/place.rs                          | 403 +++++++++++++-
 src/worldbuilding/markdown/mod.rs                  |   2 +
 src/worldbuilding/markdown/place.rs                | 338 ++++++++++++
 src/worldbuilding/repository.rs                    |  22 +-
 tests/world_chilguk_chunchu_phase3_checkpoint1.rs  | 255 +++++++++
 13 files changed, 2143 insertions(+), 47 deletions(-)
```

---

## 데모 명령

```bash
# 단위 테스트
cargo test --features embed --lib place
cargo test --features embed --lib schema_v

# 통합 테스트 (체크포인트 1 본체)
cargo test --features embed --test world_chilguk_chunchu_phase3_checkpoint1

# JSON dump 재생산
cargo test --features embed --test world_chilguk_chunchu_phase3_checkpoint1 \
  dump_places_json -- --ignored --nocapture

# world-load (의도적 fail — 외래키 결손 24건 stderr로 출력 후 DB 미수정)
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload
```

---

## 변환된 두 .md 전문

### `projects/chilguk-chunchu/world/place/place-daejin.md`

```markdown
---
id: place-daejin
layer: settlement
kind: nation
name: 대진(大辰)
aliases:
  - 중원 황도
  - 옛 통일제국
  - 축소 제국
summary: |
  270년 전 통일제국 대진이 차지했던 영토. 현재는 중원 일부로 축소됐으나 정통성 명분과
  최다 인구를 보유한 정치체. 천순제는 꼭두각시이며 실권은 환관 조고가 쥐고 있다.
tags: [wuxia, place, settlement, nation, central-plain]
extras:
  capital: 낙양(洛陽)
  capital_hanja: 洛陽
  polity: 왕조 (축소 제국)
  population_note: 칠국 중 최다. 중원 인구 밀집.
  ki_concentration: 보통
  controlling_group: group-daejin-court
spatial:
  parent_place: ~
  relative_position: center
  bordering_places:
    - place-namgung
    - place-jiyu-doshi
    - place-seoryang
  geography_refs: []
---

## 개요
대진(大辰)은 270년 전 무림 연합의 지원으로 태조 진천명이 세운 통일제국의 후예 영토다.
한때 칠국 모든 지역의 명목 종주였으나, 10년 전 "붉은 밤의 변"으로 6개 지역이
독립·자치를 선언하며 영토는 중원 일부로 축소됐다. …

## 통치
명목 원수는 천순제(`npc-07`)이지만 옥좌에 앉되 말은 하지 못한다. 실권자는 환관
조고(`npc-02`)이며, 사설 첩보·암살 조직 십상시(`group-shipsangsi`)를 통해 …

## 핵심 NPC
- 천순제 (`npc-07`) — 명목 황제, 꼭두각시
- 조고 (`npc-02`) — 실권자, 십상시 수장
- 야율설화 (`npc-06`) — 북원에서 보낸 볼모. 자유도시에 파견 중

## 핵심 갈등
- 잃어버린 영토 수복 야심 — 6국 모두에 잠재 적대 …
- 천순제 vs 조고 — 꼭두각시가 깨어날 것인가 …
- 소림·무당 vs 조정 — 정파 문파가 독자 행동할 것인가 …
- 천마신교(사파) — 혈교와의 묵계 의혹 + 사파 적대 …
- 혈교 잔당 — 낙양 지하에 거점 가능성 …

## 플레이어가 방문할 이유
- 메인 퀘스트: 피의 실종 사건의 현장이 낙양 지하
- 조고 대면: 메인 적대자의 근거지
- 소림·무당 방문: 정파 핵심 문파 접촉
- 혈교 단서: 낙양 지하 탐색
- 야율설화 관련: 설화의 볼모 거처

## 전사(前史)
원년에 태조 진천명이 무림 연합과 함께 통일제국 건국. … 10년 전 "붉은 밤의 변"으로
영토 와해. 현재 천순제·조고 체제.
```

### `projects/chilguk-chunchu/world/place/place-western-mountains.md` (신규 1차 작성)

```markdown
---
id: place-western-mountains
layer: geography
kind: mountain-range
name: 서부 산악지대
aliases:
  - 서령산맥
  - 만년설봉
summary: |
  대륙 서쪽 변경의 거대 산맥. 만년설 봉우리들과 깊은 협곡으로 외부 세계와의
  유일한 통로이자 천연 방벽. 서량의 등을 받치며 점창·아미를 품는 자연 지형.
tags: [wuxia, place, geography, mountain-range, western-frontier]
extras:
  terrain_type: mountain-range
  climate: 고산 한랭. 겨울 폭설로 협곡 차단 빈번.
  hazards:
    - 눈사태
    - 협곡 안개
    - 산적
    - 마수
  signature_features:
    - 망주봉(望主峰)
    - 십리협(十里峽)
    - 천녀폭(天女瀑)
spatial:
  parent_place: ~
  relative_position: west
  bordering_places:
    - place-seoryang
  # geography_refs는 settlement에서만 의미 — geography는 비움
---

## 개요
서부 산악지대는 대륙 서쪽 끝, 서량(西涼) 영토의 등을 받치는 거대 산맥이다.
만년설로 덮인 봉우리들과 깊은 협곡이 외부 세계와의 자연스러운 단절을 만들고,
동시에 서역 교역로의 유일한 통로 역할을 한다. 점창산·아미산이 이 영역의 일부로 흡수된다.

## 지형·기후
고산 한랭 기후. 여름은 짧고 겨울은 길어 폭설이 협곡을 자주 차단한다. …

## 위험·서식 생물
- 눈사태와 협곡 안개로 외지인은 토박이 안내 없이 통행 곤란
- 산적 무리가 협곡을 거점으로 활동. 서량 군사가 정기적으로 토벌
- 무협 결의 마수(예: 설표·고산 독사 변종) 출몰
- 당가 일족의 약재 채집과 서량 군사의 정찰이 동시에 이루어지는 활동권

## 인접 정치체
- 서량(`place-seoryang`) — 산맥의 동쪽 변경. 독관성·점창·아미를 품으며 산맥을
  자연 방벽으로 삼는다. 산맥 너머의 서역과의 교역은 모두 서량을 통과.

## 자원·산물
- 약초·광물: 당가 일족의 독약/해독약/비약 산업의 원자재 다수 출처
- 빙하수·고산 광물: 일부 비급의 수련에 필요한 환경 조건
- 서역 교역로의 통행세 — 서량 국가 경제의 한 축
- 산적·마수 도살 부산물 — 무인의 보조 수입

## 플레이어가 방문할 이유
- 외부 세계로 나가는 통로: 서역 너머의 단서 추적
- 비급·기연: 깊은 협곡·빙하 동굴에 은거하는 고수와의 조우
- 산적 토벌·마수 사냥 등 사이드 퀘스트의 본거지
- 점창·아미 등 산악에 위치한 정파 문파 방문 (Step 4·이후 등록 예정)
```

---

## 도메인 객체 dump (JSON, 라운드트립 후)

`dump_places_json` 테스트 출력의 핵심 부분:

### `place-daejin`

```json
{
  "id": "place-daejin",
  "layer": "settlement",
  "kind": "nation",
  "name": "대진(大辰)",
  "aliases": ["중원 황도", "옛 통일제국", "축소 제국"],
  "tags": ["wuxia","place","settlement","nation","central-plain"],
  "extras": {
    "capital": "낙양(洛陽)",
    "capital_hanja": "洛陽",
    "controlling_group": "group-daejin-court",
    "ki_concentration": "보통",
    "polity": "왕조 (축소 제국)",
    "population_note": "칠국 중 최다. 중원 인구 밀집."
  },
  "body_sections": {
    "개요": "…",
    "통치": "…",
    "핵심 NPC": "…",
    "핵심 갈등": "…",
    "플레이어가 방문할 이유": "…",
    "전사(前史)": "…"
  },
  "spatial": {
    "relative_position": "center",
    "bordering_places": ["place-namgung","place-jiyu-doshi","place-seoryang"]
  }
}
```

`spatial.parent_place`/`geography_refs`는 default(없음·빈)이라 직렬화에서 생략됨 — `Spatial`의 `skip_serializing_if` 정책. 라운드트립은 같은 default로 복원되므로 원본 동치.

### `place-western-mountains`

```json
{
  "id": "place-western-mountains",
  "layer": "geography",
  "kind": "mountain-range",
  "name": "서부 산악지대",
  "aliases": ["서령산맥","만년설봉"],
  "tags": ["wuxia","place","geography","mountain-range","western-frontier"],
  "extras": {
    "climate": "고산 한랭. 겨울 폭설로 협곡 차단 빈번.",
    "hazards": ["눈사태","협곡 안개","산적","마수"],
    "signature_features": ["망주봉(望主峰)","십리협(十里峽)","천녀폭(天女瀑)"],
    "terrain_type": "mountain-range"
  },
  "body_sections": {
    "개요": "…",
    "지형·기후": "…",
    "위험·서식 생물": "…",
    "인접 정치체": "…",
    "자원·산물": "…",
    "플레이어가 방문할 이유": "…"
  },
  "spatial": {
    "relative_position": "west",
    "bordering_places": ["place-seoryang"]
  }
}
```

---

## SQLite 라운드트립 + FTS 인덱스 검증

`sqlite_roundtrip_preserves_all_fields_for_both_layers` 테스트 (in-memory SQLite, 파일 DB 동일 코드 경로):

- `count_places("chilguk-chunchu")` = **2**
- `get_place(place-daejin)` → 원본과 동치 (모든 필드 보존)
- `get_place(place-western-mountains)` → 원본과 동치
- 두 layer 분기: settlement·geography가 각자 다른 H2 섹션 + 다른 extras 키 사용 (`layer_branching_two_places_use_distinct_extras_keys`)

`places_fts` 인덱스 검증 (`search_places_matches_alias_and_body`):
- "중원 황도" → place-daejin (alias, FTS5 trigram)
- "산악" → place-western-mountains (body 매칭)
- "낙양" → place-daejin (extras·body)

`parent_place` cycle 검출: 두 Place 모두 parent 없음 → cycles 0건 (`parent_place_cycle_detection_passes_for_two_top_level_places`).

> world-load CLI는 Phase 1·2 시드의 미해결 Place ID 참조 24건 때문에 의도적으로 fail
> 후 DB 미수정 상태로 남아 있다. 따라서 SQLite 영속화 검증은 통합 테스트의 in-memory
> 라운드트립 (`SqliteWorldStore::in_memory()`)으로 대체. v2→v3 마이그레이션은 별도
> 테스트(`schema_v2_to_v3_migration_upgrades_existing_file_db`)로 file-DB 경로까지 검증.

---

## 변환 시 결정한 것

### 대진 (settlement, nation)

| 결정 항목 | 값 | 근거 |
|---|---|---|
| `id` | `place-daejin` | 사양 §6.1 example. nation 단위로 단순 ID. |
| `aliases` | `["중원 황도","옛 통일제국","축소 제국"]` | "낙양"은 alias 대신 `extras.capital`로 분리 (낙양은 city, 대진은 nation — granularity가 다름). "축소 제국"은 §1 제목에서. |
| `extras.capital` | `낙양(洛陽)` + `capital_hanja: 洛陽` | seven-nations.md §1.1 |
| `extras.polity` | `왕조 (축소 제국)` | §1.1 정체란 |
| `extras.ki_concentration` | `보통` | 무협 게임 결, 중원 평균치로 가정 |
| `extras.controlling_group` | `group-daejin-court` | Phase 1 Group 외래키 — 황실이 nation을 제어. sect 이중 등록과 동일 패턴, nation에서도 controlling_group 사용 가능. |
| `spatial.parent_place` | `~` (null) | 최상위 정치체 — atlas 안에 직접 자리잡음 |
| `spatial.relative_position` | `center` | §0.3 "대진이 중원(중앙)을 차지" |
| `spatial.bordering_places` | `[place-namgung, place-jiyu-doshi, place-seoryang]` | §0.3 다이어그램 + 사양 §6.1 예시. 추가 인접(북원·동해)은 제외 — diagrams상 중간자(자유도시) 또는 distance 모호. |
| `spatial.geography_refs` | `[]` (빈) | 중원 평원(`place-jungwon-plain`)이 아직 정의되지 않아 의도적으로 비움. Step 4 또는 Phase 4에서 정의되면 추가. |
| H2 섹션 | 개요·통치·핵심 NPC·핵심 갈등·플레이어가 방문할 이유·전사(前史) | settlement 양식 권장 6 + 전사 |

### 서부 산악지대 (geography, mountain-range — 신규 1차 작성)

seven-nations.md엔 직접 시트 없음. §0.3 다이어그램의 서량 좌측 빈칸 + §3 본문의 산악 언급에서 추론·창작.

| 결정 항목 | 값 | 추론 근거 |
|---|---|---|
| `id` | `place-western-mountains` | 사양 §6.1 example |
| `name` | `서부 산악지대` | 사양 §6.7 권장 |
| `aliases` | `["서령산맥","만년설봉"]` | 사양 §6.7 후보(서령산맥·서변 영봉·만년설봉) 중 2개 채택. "서변 영봉"은 너무 verbose라 제외. |
| `kind` | `mountain-range` | 사양 §6.7 |
| `extras.terrain_type` | `mountain-range` | 사양 §6.7 |
| `extras.climate` | `고산 한랭. 겨울 폭설로 협곡 차단 빈번.` | 사양 §6.7 + §3.3 곽천웅 30년 관문 방어 묘사 |
| `extras.hazards` | `[눈사태, 협곡 안개, 산적, 마수]` | 사양 §6.7 권장 + 무협 결의 "마수" 추가 |
| `extras.signature_features` | `[망주봉(望主峰), 십리협(十里峽), 천녀폭(天女瀑)]` | 사양 §6.1 example 그대로 채택 (작가가 향후 변경 가능) |
| `spatial.parent_place` | `~` | 광역 자연 영역 미정의 — 사양 §6.7 옵션 |
| `spatial.relative_position` | `west` | §0.3 다이어그램 |
| `spatial.bordering_places` | `[place-seoryang]` | 사양 §6.7 — 산맥 너머 외부 세계는 Phase 5+. |
| `spatial.geography_refs` | (생략) | geography는 geography_refs 비움 — 본 layer가 자기 자신이 자연 지형이라 layered 의미 없음. Spatial의 default로 직렬화 시 생략됨. |
| 본문 산문 | 6 H2 섹션 | geography 양식 권장 6: 개요·지형·기후·위험·서식 생물·인접 정치체·자원·산물·플레이어가 방문할 이유 |

### Layer 분기 검증 (자동 테스트로 보강)

두 Place가 layer별 distinct extras key set + distinct H2 set 사용:

| settlement (`place-daejin`)에만 | geography (`place-western-mountains`)에만 |
|---|---|
| `capital`, `capital_hanja`, `polity`, `population_note`, `ki_concentration`, `controlling_group` | `terrain_type`, `climate`, `hazards`, `signature_features` |
| H2: `통치`, `핵심 NPC`, `전사(前史)` | H2: `지형·기후`, `위험·서식 생물`, `자원·산물` |
| 공통 H2: `개요`, `핵심 갈등` (settlement만 양쪽 보유), `플레이어가 방문할 이유` | 공통 H2: `개요`, `인접 정치체`(geography만), `플레이어가 방문할 이유` |

`layer_branching_two_places_use_distinct_extras_keys` 테스트가 위 매트릭스 자동화.

---

## 외래키 결손 분석

`world-load --reload` 실행 결과 (Phase 3 활성 후): **24건 결손, DB 미수정 종료**.

```
groups parsed     = 6
persons parsed    = 9
places parsed     = 2
fk errors (활성)  = 24
```

원시 입력 시드의 **distinct Place ID 9개** (참조 횟수 합 24):

| # | 시드 ID | 참조 위치 | 분류 | 처리 방안 |
|---|---|---|---|---|
| 1 | `place-daejin-luoyang` | group-daejin-court (hq), group-shipsangsi (hq), npc-02 (b+c), npc-06 (c), npc-07 (b+c) — 7건 | (b) Step 4에서 sub-place로 추가 OR (c) 시드 단순화 | **디렉터 결정 필요 — 권장 (b)** (낙양은 게임 메인 무대) |
| 2 | `place-namgung-geomseong` | group-namgung (hq) — 1건 | (b) Step 4 sub-place | **권장 (b)** (남궁세가 본거지 + sect 이중 등록 시연 기회) |
| 3 | `place-namgung-jeongam` | npc-03 (b+c) — 2건 | (b) Step 4 sub-place OR (c) 시드 ID를 `place-namgung`으로 단순화 | **디렉터 결정 — 권장 (c)** (정암은 가상 지명. 출신지 정밀도 낮춰도 무해) |
| 4 | `place-free-city` | group-gaebang (hq) — 1건 | **typo 수정 필요** — 다른 시드는 `place-free-cities`(복수형) 사용 | **Step 4 시드 수정**: 단수→복수 통일 (`place-free-cities`) |
| 5 | `place-free-cities` | npc-05 (c), player (c) — 2건 | (b) Step 4 자유도시 settlement 추가 | **권장 (b)** |
| 6 | `place-east-coast` | npc-05 (b), player (b) — 2건 | (b) Step 4 동해(`place-donghae`)로 단순화 OR 그대로 유지 + 동해 settlement 추가 | **디렉터 결정 — 권장 (c)** 시드를 `place-donghae`로 갱신 (동해 nation = 현재 영토, 옛 이름 충돌 없음) |
| 7 | `place-aimi-shan` | npc-01 (b+c) — 2건 | (a) Step 4 sect 1-2개 시연에서 `place-aimi-pa`(아미파)로 등록. (b) 산악은 별도 sub-geography? | **권장 — sect 시연을 아미파로**: 시드를 `place-aimi-pa`로 갱신, parent=`place-seoryang`, geography_refs=`place-western-mountains` |
| 8 | `place-bukwon-grasslands` | npc-06 (b) — 1건 | (b) Step 4 자연 지형 1-2개 시연에서 북원 초원으로 등록 OR (c) `place-bukwon`(국가)로 단순화 | **디렉터 결정 — 권장 (b)** (자연 지형 layer 시연을 위해 `place-bukwon-grasslands`를 grassland kind로 신규 작성) |
| 9 | `place-free-cities-back-alleys` | player.extras.starting_location — 1건 (검증 외) | (a) 사양 외래키 매트릭스 §3.3에 없음 — `extras` 안의 텍스트라 Phase 3 검증 미적용 | **유지** (Phase 5+ Atlas/Scene 통합 시점 정밀도 결정) |

**분류 요약**:
- (a) Step 4에서 자연스레 해소: ID 4(typo 수정), 7(아미파 sect 등록 시연), 9(extras라 검증 외)
- (b) Step 4에서 sub-place 추가로 해소: ID 1, 2, 5, 8
- (c) Phase 1·2 시드 ID 단순화로 해소 (디렉터 의견 필요): ID 3, 6 (권장)

**Step 4 진입 시 외래키 결손 0건 목표 달성 가능 — 모든 9 ID에 대한 처리 방안 식별됨.**

### 자유도시 sub-place 정밀도 — 디렉터 결정 사항

사양 §3.3 "자유도시 세부(`back-alleys`)·동해 연안 같은 지역 단위"의 처리 방안을 디렉터 결정 사항으로 명시 요청:

**옵션 A — sub-place 적극 도입 (정밀도 유지)**
- `place-jiyu-doshi` (자유도시 본체, settlement/autonomous-zone)
- `place-jiyu-doshi-back-alleys` (sub-place, parent=`place-jiyu-doshi`, kind=city 또는 새 kind="district")
- player.extras.starting_location은 그대로 `place-free-cities-back-alleys` 유지 → typo 수정만 (`place-free-cities-back-alleys` → `place-jiyu-doshi-back-alleys`)
- **장점**: 게임 시작 지점의 정밀도 보존
- **단점**: Phase 3 분량 증가, kind 분류 결정 추가 필요 (district는 신규 kind?)

**옵션 B — 정밀도 단순화 (사양 §3.3 권장 (c))**
- `place-jiyu-doshi` (자유도시 본체) 1개만 등록
- 시드 갱신: `place-free-cities-back-alleys` → `place-jiyu-doshi`
- player.extras.starting_location 검증은 어차피 Phase 3 외래키 매트릭스에 없으니 typo 수정만 (`place-free-cities` → `place-jiyu-doshi`)
- **장점**: Phase 3 분량 절제. Phase 5+ Scene/Atlas에서 정밀도 부활.
- **단점**: 게임 시작 지점("뒷골목") 정밀도 손실 — 단 `extras.starting_location` 텍스트 + body_sections 산문에 "뒷골목" 어휘 보존

**옵션 C — Phase 5+ 미루기**
- player.extras.starting_location은 그대로 두고, 자유도시 본체조차 Phase 5+로 미룸
- player.current_location은 `place-jiyu-doshi`로 가리키되 자유도시 등록은 Phase 5+
- **단점**: Step 4 외래키 0건 목표 미달성 (`place-jiyu-doshi` 결손 잔여)

**Claude 권장**: **옵션 B** (정밀도 단순화). 사양 §3.3에서도 "Phase 3 분량 절제"가 (a)/(c) 권장이라 명시. 디렉터 의견 요청.

### `place-namgung-jeongam` 정밀도 — 디렉터 결정 사항

`place-namgung-jeongam`(정암)은 npc-03(남궁린, 비공식 무력)의 출신지·현재 위치 기록. 정암은 seven-nations.md에 직접 등장하지 않고 Phase 2 시드에서 자체 작명한 sub-place. 두 옵션:

**옵션 A**: Step 4에서 `place-namgung-jeongam`을 city/district로 등록 (parent=`place-namgung`)
**옵션 B**: 시드를 `place-namgung`(국가 단위)로 단순화 — 남궁가 출신만 알면 충분

**Claude 권장**: **옵션 B** (정밀도 단순화). 정암은 게임 메인 무대 아님, 인물의 affinity 표지 충분.

### 동해 `place-east-coast` 정밀도 — 디렉터 결정 사항

`place-east-coast`(동해 연안)은 npc-05·player의 birthplace. 동해는 §6에서 nation으로 등장(`place-donghae`로 등록 가능). "연안" sub-place는 §0.3 다이어그램에 직접 시트 없음.

**옵션 A**: `place-donghae`(nation, settlement) + `place-donghae-coast`(geography, coast) 둘 다 등록 → birthplace를 `place-donghae-coast`로 갱신
**옵션 B**: 시드를 `place-donghae`로 단순화 (정밀도 낮춤). 출신이 동해 사람이면 충분.

**Claude 권장**: 사양 §3.4에서 sect 1-2개 시연 외에 자연 지형 1-2개 시연도 별도 명시되어 있어, 동해 연안을 자연 지형 시연으로 활용하면 자연스러움. 그러나 본 보고서의 분류 요약과 일관성을 위해 **권장은 옵션 B (단순화)**. 자연 지형 시연은 `place-bukwon-grasslands` 또는 남만 밀림으로.

---

## 막힌 결정 — 디렉터 의견 요청 사항

요약: 사양 §3.3·§3.4의 sub-place 정밀도 결정을 Step 4 진입 전 확정 필요.

1. **자유도시 sub-place 정밀도** (옵션 A/B/C 중) — 권장 B
2. **`place-namgung-jeongam` 등록 vs 시드 단순화** — 권장 단순화
3. **`place-east-coast` 등록 vs `place-donghae` 단순화** — 권장 단순화
4. **자연 지형 시연 1-2개의 선택** — 사양 §5 Step 4 "남쪽 밀림(남만), 또는 동해 연안" 양자 중 하나. **Claude 권장: `place-bukwon-grasslands`(이미 시드에 참조됨) + 남만 밀림 1개 추가** — 시드 외래키 해소도 동시에.
5. **sect 이중 등록 시연 1-2개의 선택** — 사양 §3.4 후보: `place-namgung-sega`(parent=`place-namgung`) 또는 `place-aimi-pa`(parent=`place-seoryang`). **Claude 권장: 둘 다** — namgung-sega는 nation 영토 sub-place 시연, aimi-pa는 geography_refs 시연(아미산은 서부 산악 안). 추가 시드 매핑: npc-01(아미파 마지막 의녀)의 birthplace를 `place-aimi-shan` → `place-aimi-pa`로 갱신.
6. **체크포인트 1 통과 신호 후 Step 4 진입 가능 여부**

---

## 다음 의견

체크포인트 1의 핵심 추상 검증(layer 분기·spatial·외래키 활성·sect 이중 등록 패턴)이 모두 동작함을 자동 테스트로 보강했고, 사양 §5 Step 3의 예측치(fk errors = N, 의도된 partial-state)도 일치한다.

Step 4 진입 시 다음 순서를 권장:

1. **시드 ID 정합성 정리** (typo 수정 + 단순화 결정 반영)
   - `place-free-city` → `place-free-cities` (group-gaebang.md typo 수정, 양식 통일)
   - `place-east-coast` → `place-donghae` (디렉터 승인 시)
   - `place-namgung-jeongam` → `place-namgung` (디렉터 승인 시)
   - `place-aimi-shan` → `place-aimi-pa` (sect 시연 채택 시)

2. **6 settlement 추가**: 남궁·서량·북원·남만·동해·자유도시 (각 1 .md)
3. **자연 1-2 추가**: `place-bukwon-grasslands`(필요) + 남만 밀림 1개
4. **sect 1-2 추가**: `place-namgung-sega` + `place-aimi-pa`
5. **`place-daejin.spatial.geography_refs`**에 `place-jungwon-plain` 추가 여부 — Phase 4 Atlas로 미룰지 결정 (Phase 4 진입 시 정의 가능, 우선 빈 값 유지 권장)

Step 5(MCP 도구 + 체크포인트 2)는 외래키 0건 달성 후 단일 commit으로 묶기 적절.

**commit pause 유지** — Cowork 리뷰 통과 신호 후 Step 4·5 착수.
