# Phase 3 Checkpoint 2 Report — 11 Place 등록 + MCP 도구 + 외래키 0건

**작업 범위 (사양 §5 Step 4·5)**
- 시드 외래키 갱신 19건 (디렉터 5 명시 + city-level 단순화 묵시 14건)
- 9 신규 Place .md 작성 (6 settlement + 2 geography + 1 sect)
- world-load 외래키 0건 달성 — 11 Place 모두 인덱싱
- MCP 도구 3종(`list_places`/`get_place`/`search_places`) + REST 엔드포인트
- 통합 테스트 9건 (sect 양방향, geography_refs 양방향, 6 search 쿼리, 외래키 0건 가드)

**브랜치**: `claude/place-vertical-slice-phase3-6bp5c`
**선행 commit**: `2987680` (체크포인트 1)
**Phase 3 진행률**: Step 1·2·3·4·5 완료 → Phase 3 종결 가능.

---

## Done

### 1. 시드 외래키 갱신 (19건, 디렉터 5 + city-level 단순화 14)

디렉터 명시 (5):
| # | 변경 | 파일 |
|---|---|---|
| 1 | `headquarters: place-free-city` → `place-jiyu-doshi` | group-gaebang.md |
| 2 | `current_location: place-free-cities` → `place-jiyu-doshi` (×2) | npc-05.md, player.md |
| 3 | `birthplace + current_location: place-namgung-jeongam` → `place-namgung` | npc-03.md |
| 4 | `birthplace: place-east-coast` → `place-donghae` (×2) | npc-05.md, player.md |
| 5 | `birthplace + current_location: place-aimi-shan` → `place-seoryang` | npc-01.md |

city-level 단순화 묵시 (14, "fk errors = 0" + 11 Place 한도 달성용 패턴 일관 적용):
| # | 변경 | 파일 |
|---|---|---|
| 6 | `headquarters: place-daejin-luoyang` → `place-daejin` (×2) | group-daejin-court.md, group-shipsangsi.md |
| 7 | `headquarters: place-namgung-geomseong` → `place-namgung-sega` (sect 이중 등록 양방향) | group-namgung.md |
| 8 | `birthplace + current_location: place-daejin-luoyang` → `place-daejin` (×3 인물 = 6) | npc-02.md, npc-07.md, npc-06.md(current만) |
| 9 | `birthplace + current_location: place-seoryang-dokgwanseong` → `place-seoryang` | npc-04.md |

`player.extras.starting_location: place-free-cities-back-alleys` — 그대로 유지 (Phase 3 외래키 매트릭스 §3.3에 없는 `extras.*` 필드. Phase 5+ Atlas/Scene에서 정밀도 부활 결정).

### 2. 9 신규 Place .md (`projects/chilguk-chunchu/world/place/`)

| 파일 | layer | kind | 핵심 |
|---|---|---|---|
| `place-namgung.md` | settlement | nation | 검왕 남궁혁의 가문 왕국. controlling_group=group-namgung. bordering=[daejin, jiyu-doshi, namman]. |
| `place-seoryang.md` | settlement | nation | 독왕의 나라. **geography_refs=[place-western-mountains]** — settlement→geography layered 시연. 본문 "독관성(毒關城)" 명시. |
| `place-bukwon.md` | settlement | nation | 늑대왕 바투의 유목 왕국. **geography_refs=[place-bukwon-grasslands]** — 디렉터 요청 양방향 시연. |
| `place-namman.md` | settlement | nation | 부족연합. **geography_refs=[place-namman-jungle]**. |
| `place-donghae.md` | settlement | nation | 상인 공화국. 영토 동해 연안 + 섬. |
| `place-jiyu-doshi.md` | settlement | autonomous-zone | 강호자치령 + 옛 영주 번왕국터. 빈민가/뒷골목 본문 보존. |
| `place-namgung-sega.md` | settlement | sect | **parent_place=place-namgung** + **controlling_group=group-namgung** sect 이중 등록 시연. |
| `place-bukwon-grasslands.md` | geography | grassland | 북원 초원. bordering=[place-bukwon]. 본문에 "초원" 다수 등장. |
| `place-namman-jungle.md` | geography | jungle | 남만 밀림. bordering=[place-namman]. 본문에 "밀림" 다수 등장. |

distinct kind 수: settlement {nation, autonomous-zone, sect} + geography {mountain-range, grassland, jungle} = 6종. layer·kind 다양성 검증 폭 확보.

### 3. world-load 외래키 0건

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
groups parsed     = 6
persons parsed    = 9
places parsed     = 11
errors            = 0
group cycles      = 0
place cycles      = 0
fk errors (활성)  = 0
mind eligible     = 9
```

체크포인트 1: 24건 결손 → 체크포인트 2: **0건**. 사양 §5 Step 4 목표 달성.

### 4. MCP 도구 + REST 엔드포인트

`src/bin/mind-studio/handlers/world_places.rs` (Phase 1·2 패턴 그대로):
- `GET /api/world/places` (filter: layer/kind/parent_place/genre_tag)
- `GET /api/world/places/search?q=...&top_k=`
- `GET /api/world/places/{id}`

`src/bin/mind-studio/mcp_server.rs` (3 tool):
- `list_places(layer?, kind?, parent_place?, genre_tag?)` → `Vec<Place>`
- `get_place(place_id)` → `Place` (없으면 에러)
- `search_places(query, top_k=5)` → `Vec<Place>`

embed feature 미활성 또는 `world_store` 미부착 시 적절한 에러 반환 — Phase 1·2 동일 정책. `cargo build --features mind-studio,embed --bin npc-mind-studio` 통과.

### 5. 통합 테스트 (`tests/world_chilguk_chunchu_phase3_checkpoint2.rs`)

9개 테스트, 모두 통과:

| # | 테스트 | 검증 |
|---|---|---|
| 1 | `places_indexed_with_expected_counts` | 11 places · 8 settlements · 3 geographies · 6 nations · 1 autonomous-zone · 1 sect |
| 2 | `list_places_filter_by_parent_place_returns_namgung_sega` | parent_place 필터: place-namgung → namgung-sega |
| 3 | `sect_double_registration_bidirectional` | place-namgung-sega ↔ group-namgung 양방향 외래키 |
| 4 | `geography_refs_bidirectional_with_bukwon` | place-bukwon ↔ place-bukwon-grasslands 양방향 인접 |
| 5 | `geography_refs_layer_constraint_holds` | 모든 settlement.geography_refs target이 layer=Geography (런타임 invariant) |
| 6 | `fk_zero_phase1_phase2_seeds_all_resolve` | Phase 1·2 시드의 모든 hq/birthplace/current_location ID가 places에 존재 — 0 결손 가드 |
| 7 | `search_places_six_queries_match_expected_targets` | 6쿼리: 검성·독관성·낙양·산악·초원·밀림 |
| 8 | `parent_place_cycle_is_zero_for_full_dataset` | 전체 11 Place 데이터셋에서 cycle 0건 |
| 9 | `place_daejin_borders_resolve_after_step4` | 체크포인트 1엔 의도적 결손이었던 place-daejin.bordering_places가 모두 해소 |

### 6. 빌드·테스트 결과 (회귀 포함)

```
cargo build --features embed                                : ✓
cargo build --features mind-studio,embed --bin npc-mind-studio : ✓
cargo test --features embed --lib                           : 379 passed
cargo test --features embed --test world_chilguk_chunchu_phase3_checkpoint1 : 7 passed (1 ignored — dump 보조)
cargo test --features embed --test world_chilguk_chunchu_phase3_checkpoint2 : 9 passed
cargo test --features embed --test world_chilguk_chunchu_e2e               : 7 passed (회귀)
cargo test --features embed --test world_chilguk_chunchu_person_e2e        : 7 passed
cargo test --features embed --test dispatch_v2_test                        : 36 passed
cargo test --features embed --test dialogue_test                           : 14 passed
cargo test --features embed --test director_test                           : 30 passed (1 ignored)
```

`embed_test` 6건 환경 의존 실패 (BGE-M3 ONNX 모델 부재) — Phase 3 무관, 체크포인트 1과 동일.

---

## Diff

```
 docs/tasks/phase3-checkpoint2-report.md            | (이 파일)
 projects/chilguk-chunchu/world/place/place-bukwon-grasslands.md            | (신규)
 projects/chilguk-chunchu/world/place/place-bukwon.md                       | (신규)
 projects/chilguk-chunchu/world/place/place-donghae.md                      | (신규)
 projects/chilguk-chunchu/world/place/place-jiyu-doshi.md                   | (신규)
 projects/chilguk-chunchu/world/place/place-namgung-sega.md                 | (신규)
 projects/chilguk-chunchu/world/place/place-namgung.md                      | (신규)
 projects/chilguk-chunchu/world/place/place-namman-jungle.md                | (신규)
 projects/chilguk-chunchu/world/place/place-namman.md                       | (신규)
 projects/chilguk-chunchu/world/place/place-seoryang.md                     | (신규)
 projects/chilguk-chunchu/world/group/group-daejin-court.md                 | (1)
 projects/chilguk-chunchu/world/group/group-gaebang.md                      | (1)
 projects/chilguk-chunchu/world/group/group-namgung.md                      | (1)
 projects/chilguk-chunchu/world/group/group-shipsangsi.md                   | (1)
 projects/chilguk-chunchu/world/person/npc-01.md                            | (2)
 projects/chilguk-chunchu/world/person/npc-02.md                            | (2)
 projects/chilguk-chunchu/world/person/npc-03.md                            | (2)
 projects/chilguk-chunchu/world/person/npc-04.md                            | (2)
 projects/chilguk-chunchu/world/person/npc-05.md                            | (2)
 projects/chilguk-chunchu/world/person/npc-06.md                            | (1)
 projects/chilguk-chunchu/world/person/npc-07.md                            | (2)
 projects/chilguk-chunchu/world/person/player.md                            | (2)
 src/bin/mind-studio/handlers/mod.rs                                        | (3)
 src/bin/mind-studio/handlers/world_places.rs                               | (신규)
 src/bin/mind-studio/main.rs                                                | (7)
 src/bin/mind-studio/mcp_server.rs                                          | (~120)
 tests/world_chilguk_chunchu_phase3_checkpoint2.rs                          | (신규)
```

---

## 데모 명령

```bash
# Step 4 — 외래키 0건 검증
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload
# 기대: places indexed = 11, fk errors (활성) = 0

# Step 5 — 통합 테스트 (체크포인트 2 본체)
cargo test --features embed --test world_chilguk_chunchu_phase3_checkpoint2

# 회귀 검증 (체크포인트 1 + Phase 1·2 e2e + dispatch/dialogue/director)
cargo test --features embed --lib
cargo test --features embed --test world_chilguk_chunchu_phase3_checkpoint1
cargo test --features embed --test world_chilguk_chunchu_e2e
cargo test --features embed --test world_chilguk_chunchu_person_e2e

# Mind Studio 부착 시 MCP 도구
cargo run --features mind-studio,embed --bin npc-mind-studio
# REST: GET /api/world/places?layer=settlement
# REST: GET /api/world/places/search?q=낙양
# REST: GET /api/world/places/place-namgung-sega
# MCP:  list_places(layer="settlement") · get_place("place-daejin") · search_places(query="검성")
```

---

## 정성 검증 — 사양 §5 Step 5 매트릭스 자동화 결과

### `list_places(layer="settlement")` → 8건

(통합 테스트 `places_indexed_with_expected_counts` 자동화)

| id | name | kind |
|---|---|---|
| `place-bukwon` | 북원(北原) | nation |
| `place-daejin` | 대진(大辰) | nation |
| `place-donghae` | 동해(東海) | nation |
| `place-jiyu-doshi` | 자유도시 | autonomous-zone |
| `place-namgung` | 남궁(南宮) | nation |
| `place-namgung-sega` | 남궁세가(南宮世家) | sect |
| `place-namman` | 남만(南蠻) | nation |
| `place-seoryang` | 서량(西涼) | nation |

### `list_places(layer="geography")` → 3건

| id | name | kind |
|---|---|---|
| `place-bukwon-grasslands` | 북원 초원 | grassland |
| `place-namman-jungle` | 남만 밀림 | jungle |
| `place-western-mountains` | 서부 산악지대 | mountain-range |

### `list_places(parent_place="place-namgung")` → 1건

| id | name | parent_place |
|---|---|---|
| `place-namgung-sega` | 남궁세가(南宮世家) | place-namgung |

### `get_place("place-namgung-sega")` — sect 이중 등록 양방향

```json
{
  "id": "place-namgung-sega",
  "layer": "settlement",
  "kind": "sect",
  "name": "남궁세가(南宮世家)",
  "extras": {
    "controlling_group": "group-namgung",   ← Place → Group
    ...
  },
  "spatial": {
    "parent_place": "place-namgung"
  }
}
```

```
get_group("group-namgung").headquarters = "place-namgung-sega"   ← Group → Place
```

테스트 `sect_double_registration_bidirectional` 자동화.

### `search_places` 6쿼리 (디렉터 명시)

| query | 매치 | 매칭 경로 |
|---|---|---|
| "검성" | place-namgung | body·extras (`capital: 검성(劍城)`) |
| "독관성" | place-seoryang | body·extras (`capital: 독관성(毒關城)`) |
| "낙양" | place-daejin | body·extras (`capital: 낙양(洛陽)`) |
| "산악" | place-western-mountains | name·body (자연 지형) |
| "초원" | place-bukwon-grasslands | body 다수 + alias `대초원/북녘 초원` |
| "밀림" | place-namman-jungle | name·body 다수 |

테스트 `search_places_six_queries_match_expected_targets` 자동화.

### `place-bukwon.spatial.geography_refs ↔ place-bukwon-grasslands` 양방향

- `place-bukwon.spatial.geography_refs = [place-bukwon-grasslands]` (settlement → geography layered)
- `place-bukwon-grasslands.spatial.bordering_places = [place-bukwon]` (geography → settlement 인접)

테스트 `geography_refs_bidirectional_with_bukwon` 자동화. 추가로 `geography_refs_layer_constraint_holds` 가 모든 settlement에 대해 target layer=Geography invariant 확인.

### 외래키 결손 0건 가드

테스트 `fk_zero_phase1_phase2_seeds_all_resolve`이 모든 (group.headquarters · person.birthplace · person.current_location) → places.id 사상 검증. 결손 발견 시 어떤 ID가 누락됐는지 진단 메시지 출력.

---

## 디렉터 결정 5건 처리 결과

| # | 결정 | 처리 |
|---|---|---|
| 1 | 자유도시 sub-place 정밀도 = 옵션 B (단순화) | `place-free-city`/`place-free-cities` → `place-jiyu-doshi` 통일. `extras.starting_location`는 그대로 유지 (Phase 5+). |
| 2 | `place-namgung-jeongam` 단순화 | npc-03 birthplace + current_location → `place-namgung` |
| 3 | `place-east-coast` → `place-donghae` 단순화 | npc-05·player birthplace → `place-donghae` |
| 4 | 자연 지형 시연 = `place-bukwon-grasslands` + `place-namman-jungle` 양쪽 | 둘 다 작성. distinct kind(grassland·jungle). place-bukwon·place-namman geography_refs 양방향 시연. |
| 5 | sect 이중 등록 = `place-namgung-sega` 1개 | 작성. parent=`place-namgung`, controlling_group=`group-namgung`. group-namgung.headquarters를 `place-namgung-geomseong` → `place-namgung-sega`로 갱신해 양방향 외래키 시연. |

추가 묵시 적용 (외래키 0건 + 11 Place 한도 충족):
- city-level 단순화: `place-daejin-luoyang` → `place-daejin` (group-daejin-court·shipsangsi·npc-02·06·07), `place-seoryang-dokgwanseong` → `place-seoryang` (npc-04), `place-namgung-geomseong` → `place-namgung-sega` (group-namgung)
- 디렉터 권장 일관 패턴 — Phase 5+ Atlas/Scene 도입 시 정밀도 복원 가능.

---

## 막힌 결정 — 없음

체크포인트 1 보고서의 디렉터 의견 요청 6건이 모두 처리되어 Phase 3 종결까지의 모든 결정이 이미 합의됨. 추가 의견 요청 없이 회귀·테스트 통과 → Phase 4(Atlas) 진입 가능 상태.

---

## Phase 4 (Atlas) 진입 가능 여부 의견

**진입 가능**.

체크포인트 2 산출물이 Phase 4의 전제를 모두 충족:
1. **Place 도메인 완성** — Atlas의 `references: Vec<PlaceId>`가 직접 가리킬 11 Place 인스턴스 + spatial.relative_position 라벨 모두 정착. Atlas의 `## 배치 다이어그램` 산문 + `references` 외래키만 추가하면 됨.
2. **외래키 매트릭스 0건** — Atlas의 references 도입 시 layer-cross 검증(Atlas → Place)이 Phase 1·2·3과 같은 하드 페일 패턴으로 매끄럽게 확장.
3. **MCP 패턴 정착** — `list_*`/`get_*`/`search_*` 3종이 Place까지 일관 정착. Atlas도 `get_atlas`/`list_atlases` 같은 패턴으로 즉시 추가 가능.
4. **schema_meta v3 + place_atlas_refs 자리** — Phase 4 진입 시 `migrate_v4`로 atlases·atlases_fts·place_atlas_refs 추가가 Phase 1·2·3 마이그레이션 패턴 그대로.

다음 권장 입력: `wuxia-core/docs/world/seven-nations.md §0.3` 다이어그램을 atlas-jungwon으로 직접 변환 + 칠국 references 매핑.

> 사양 §3.7 게이트 그대로 — Phase 4는 별도 TASK(`task-phase4-atlas-vertical-slice.md`)로 분리. 본 보고서 통과 후 Phase 4 작전 작성 진입.

---

## 다음 의견

체크포인트 2의 산출물 11 Place + sect 양방향 + geography_refs 양방향 + 외래키 0건이 모두 사양 §5 Step 4·5의 검증 매트릭스를 자동 테스트로 보강. 회귀(체크포인트 1 + Phase 1·2 e2e + dispatch/dialogue/director)도 모두 통과.

체크포인트 분리 게이트(§3.7) 그대로 — 본 보고서가 Cowork 리뷰 통과 시 **Phase 3 종결**. Phase 4(Atlas) 별도 TASK로 분리해 진입.

이번 commit 범위: Step 4·5 단일 commit으로 묶기 적절(체크포인트 1 보고서 §다음 의견의 권장 그대로).
