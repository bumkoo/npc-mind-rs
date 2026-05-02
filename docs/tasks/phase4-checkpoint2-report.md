# Phase 4 Checkpoint 2 Report — view 메서드 e2e + MCP 도구 + 정성 평가 + Phase 4 종결

**작업 범위 (사양 §5 Step 4)**
- mind-studio REST 엔드포인트 3종 (`GET /api/world/atlases{,/search,/{id}}`)
- MCP 도구 3종 (`list_atlases` / `get_atlas` / `search_atlases`)
- 통합 테스트 9개 — adjacent_to 매트릭스 (모든 11 anchor) + search 3 쿼리 + 외래키 가드 + filter
- Mind Studio 라이브 정성 평가 — REST 3 엔드포인트 모두 atlas-jungwon 정확 응답
- 회귀 가드 — 체크포인트 1 13개 + Phase 1·2·3 e2e + dispatch_v2 + director 모두 통과

**브랜치**: `claude/atlas-vertical-slice-phase4-jUPzD`
**선행 commit**: `a12a5d5` (체크포인트 1)
**Phase 4 진행률**: Step 1·2·3·4 완료 → Phase 4 종결 가능.

**디렉터 결정 4건 처리 완료 (체크포인트 1 통과)**:
1. SQLite FK DDL 절: 옵션 A 채택 (application-layer 검증). 변경 없음.
2. references 정렬: 좌상→우하 채택. 변경 없음.
3. aliases 2개 `["중원 대륙", "칠국 대륙"]` 승인.
4. extent 7×7 schematic 승인.

---

## Done

### 1. Mind Studio REST 엔드포인트 — `src/bin/mind-studio/handlers/world_atlases.rs` (신규)

Phase 1·2·3 `world_groups`/`world_persons`/`world_places` 패턴 그대로 미러링:

| 엔드포인트 | 기능 | 필터 |
|---|---|---|
| `GET /api/world/atlases` | 리스트 | `kind` (continent/region/city-map) · `genre_tag` |
| `GET /api/world/atlases/{id}` | 단건 detail | references + body_sections + extras 전체 |
| `GET /api/world/atlases/search?q=&top_k=` | FTS5 trigram | LIKE fallback |

`world_store` 미부착 시 `AppError::NotImplemented`, 미존재 id에 `AppError::NotFound`. 라우트
등록은 `main.rs`에서 path param `{id}`보다 `/search`를 먼저 등록 (axum 매칭 우선순위).

### 2. MCP 도구 3종 — `src/bin/mind-studio/mcp_server.rs`

`list_tools()` 정의 + `call_tool()` 분기 추가:

```jsonc
{ "name": "list_atlases", "inputSchema": { "kind?", "genre_tag?" } }
{ "name": "get_atlas",    "inputSchema": { "atlas_id" } }
{ "name": "search_atlases","inputSchema": { "query", "top_k?" } }
```

description에 **Atlas는 첫 관계 도메인이며 references(Vec<PlaceId>)로 다른 Place들을 합성**
이라는 결을 명시. `get_atlas` description에 view 메서드 4종(`places_in`/`settlements_in`/
`geographies_in`/`adjacent_to`) 사용법 안내 (클라이언트가 references와 list_places/get_place로
별도 호출).

embed feature 미활성 시 `--features embed 필요` 에러 — Phase 1·2·3 패턴 그대로.

### 3. 통합 테스트 — `tests/world_chilguk_chunchu_phase4_checkpoint2.rs` (9개, 모두 통과)

| # | 테스트 | 검증 |
|---|---|---|
| 1 | `adjacent_to_matrix_for_all_atlas_anchors` | **11 anchor 모두**의 atlas-internal 인접 — 체크포인트 1의 daejin·namgung-sega 보강 |
| 2 | `settlements_in_layer_invariant_eight_settlement_layer` | 8건 + 모두 `layer=Settlement` |
| 3 | `geographies_in_layer_invariant_three_geography_layer` | 3건 + 모두 `layer=Geography` |
| 4 | `places_in_partition_invariant_holds` | 8 + 3 = 11 = `references.len` invariant |
| 5 | `search_atlases_three_queries_match_atlas_jungwon` | **디렉터 명시 3 쿼리**: "칠국"·"중원"·"대륙" 모두 atlas-jungwon top hit |
| 6 | `references_zero_fk_residual_against_loaded_places` | 11 references PlaceId 모두 places 테이블 존재 |
| 7 | `place_atlas_refs_row_count_matches_references_length` | references 11 = list_atlases 결과의 references.len |
| 8 | `list_atlases_filter_by_kind_and_genre_tag` | kind=continent → 1, kind=region → 0, genre_tag=wuxia → 1 |
| 9 | `get_atlas_detail_contains_references_and_body_sections_and_extras` | 11 refs + 7 H2 섹션 + era 텍스트 |

#### adjacent_to 11 anchor 매트릭스 (테스트 #1 자동화)

| anchor | 결과 | 주석 |
|---|---|---|
| `place-bukwon` | `[place-daejin, place-seoryang, place-donghae]` (3) | 북원 인접 모두 atlas 안 |
| `place-seoryang` | `[place-daejin, place-bukwon, place-western-mountains]` (3) | settlement→geography 인접 포함 |
| `place-daejin` | `[place-namgung, place-jiyu-doshi, place-seoryang]` (3) | 중원 — 체크포인트 1 #8 |
| `place-donghae` | `[place-daejin, place-bukwon]` (2) | 동방 해안 |
| `place-jiyu-doshi` | `[place-daejin, place-namgung]` (2) | 자유도시 — 대진·남궁 사이 접경 |
| `place-namgung` | `[place-daejin, place-jiyu-doshi, place-namman]` (3) | 서남부 |
| `place-namgung-sega` | `[]` (0) | sect — 체크포인트 1 #9 |
| `place-namman` | `[place-namgung]` (1) | 남단 |
| `place-bukwon-grasslands` | `[place-bukwon]` (1) | geography → settlement (역방향 인접) |
| `place-western-mountains` | `[place-seoryang]` (1) | geography → settlement |
| `place-namman-jungle` | `[place-namman]` (1) | geography → settlement |

invariant: 모든 결과 PlaceId가 atlas references 안 (atlas 경계 밖 인접 자동 제외).

### 4. Mind Studio 라이브 정성 평가

`NPC_MIND_WORLD_DB=projects/chilguk-chunchu/build/world.sqlite` + `MIND_STUDIO_PORT=3457`로
서버 실행 + `world-load --reload`로 atlas-jungwon 인덱싱 후 REST 3 엔드포인트 호출:

```bash
$ curl -s http://127.0.0.1:3457/api/world/atlases | jq .
count: 1
id: atlas-jungwon
kind: continent
name: 칠국춘추 대륙
refs: 11

$ curl -s http://127.0.0.1:3457/api/world/atlases/atlas-jungwon | jq .
id: atlas-jungwon
kind: continent
aliases: ['중원 대륙', '칠국 대륙']
extent: { projection: schematic, width_units: 7, height_units: 7, unit: schematic }
refs (좌상→우하):
 - place-bukwon
 - place-bukwon-grasslands
 - place-seoryang
 - place-western-mountains
 - place-daejin
 - place-donghae
 - place-jiyu-doshi
 - place-namgung
 - place-namgung-sega
 - place-namman
 - place-namman-jungle
body sections (BTreeMap 알파벳 순): [개요, 배치 다이어그램, 자연 영역 분포, 전사(前史),
                                    정치체 분포, 주요 통로·연결, 칠국 일람]
era: 현재 (칠국춘추 270년차)

## 배치 다이어그램 첫 5줄:
  '```'
  '                    ┌──────────────────┐'
  '                    │     북 원        │'
  '                    │   (초원/유목)     │'
  '                    │   왕정(오르두)    │'

$ for q in 칠국 중원 대륙; do
    curl -s "http://127.0.0.1:3457/api/world/atlases/search?q=$q&top_k=3"
done
=== search?q=칠국 ===  hits: 1 / ids: ['atlas-jungwon']
=== search?q=중원 ===  hits: 1 / ids: ['atlas-jungwon']
=== search?q=대륙 ===  hits: 1 / ids: ['atlas-jungwon']
```

**정성 검증 결과**:
- `list_atlases` — 1건 정확 반환, references count 11.
- `get_atlas("atlas-jungwon")` — id/kind/name/aliases(2)/extent(7×7 schematic)/refs(11, 좌상→우하)/
  7 H2 섹션/extras.era 모두 정확.
- `## 배치 다이어그램` HTTP 응답에서 box-drawing(┌──────────────────┐) byte-exact 보존
  (UTF-8 JSON 인코딩 후에도 깨짐 없음).
- search_atlases 3 쿼리 모두 atlas-jungwon top hit.

MCP `tools/call`은 SSE 채널 비동기 응답 패턴(직접 POST는 `{status: sent}`만 반환)이라 라이브
e2e는 생략하나, `call_tool` 분기는 동일한 `SqliteWorldStore.{list,get,search}_atlases` 메서드를
호출하며 그 메서드들은 lib 단위 테스트(7개) + e2e(9+13=22개)로 가드된다.

### 5. 빌드·테스트 결과 (회귀 포함)

```
cargo build --features embed                                                  : ✓
cargo build --features mind-studio,chat,embed --bin npc-mind-studio           : ✓
cargo test  --features embed --lib                                            : 412 passed
cargo test  --features embed --test world_chilguk_chunchu_phase4_checkpoint2  : 9 passed
cargo test  --features embed --test world_chilguk_chunchu_phase4_checkpoint1  : 13 passed (회귀)
cargo test  --features embed --test world_chilguk_chunchu_phase3_checkpoint1  : 7 passed (회귀)
cargo test  --features embed --test world_chilguk_chunchu_phase3_checkpoint2  : 9 passed (회귀)
cargo test  --features embed --test world_chilguk_chunchu_e2e                 : 7 passed (회귀)
cargo test  --features embed --test world_chilguk_chunchu_person_e2e          : 7 passed (회귀)
cargo test  --features embed --test world_load_fk_negative                    : 3 passed (회귀)
cargo test  --features embed --test dispatch_v2_test                          : 30 passed (회귀)
cargo test  --features embed --test director_test                             : 14 passed (회귀)
```

**회귀 0건**. `embed_test` 6건 환경 의존 실패 (BGE-M3 ONNX 모델 부재) — Phase 4 무관, 지속.

---

## Diff (체크포인트 2)

```
 docs/tasks/phase4-checkpoint2-report.md          | (이 파일, 신규)
 src/bin/mind-studio/handlers/mod.rs              | +3
 src/bin/mind-studio/handlers/world_atlases.rs    | (신규, 80 LoC)
 src/bin/mind-studio/main.rs                      | +9
 src/bin/mind-studio/mcp_server.rs                | +96 (3 tool 정의 + 3 call_tool 분기)
 tests/world_chilguk_chunchu_phase4_checkpoint2.rs | (신규, 9 테스트)
```

---

## 데모 명령

```bash
# 체크포인트 2 본체
cargo test --features embed --test world_chilguk_chunchu_phase4_checkpoint2

# 회귀
cargo test --features embed --lib
cargo test --features embed --test world_chilguk_chunchu_phase4_checkpoint1
cargo test --features embed --test world_chilguk_chunchu_phase3_checkpoint1
cargo test --features embed --test world_chilguk_chunchu_phase3_checkpoint2
cargo test --features embed --test world_chilguk_chunchu_e2e
cargo test --features embed --test world_chilguk_chunchu_person_e2e
cargo test --features embed --test world_load_fk_negative
cargo test --features embed --test dispatch_v2_test
cargo test --features embed --test director_test

# world-load
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload
# 기대: atlases indexed = 1, fk errors = 0

# Mind Studio 라이브
NPC_MIND_WORLD_DB=projects/chilguk-chunchu/build/world.sqlite \
  cargo run --features mind-studio,chat,embed --bin npc-mind-studio
# REST: GET /api/world/atlases
# REST: GET /api/world/atlases/atlas-jungwon
# REST: GET /api/world/atlases/search?q=칠국
# MCP:  list_atlases() · get_atlas("atlas-jungwon") · search_atlases("중원")
```

---

## ASCII 다이어그램 byte-exact 보존 — 4단계 가드 (체크포인트 1 + 2 통합)

| 단계 | 매체 | 검증 테스트 |
|---|---|---|
| 1 | raw `.md` 파일 | `ascii_diagram_preserved_byte_exact_through_disk_and_sqlite` (CP1) — 22 sentinel 라인 |
| 2 | `atlas_from_markdown` 결과 | 위 테스트 — 펜스 안 가짜 헤더·들여쓰기 보존 |
| 3 | `SqliteWorldStore.upsert_atlas → get_atlas` 라운드트립 | 위 테스트 — string equality |
| 4 | **HTTP JSON 응답** (REST `/api/world/atlases/atlas-jungwon`) | 정성 평가 (라이브) — 박스 그림 깨짐 없음 |

원본 `seven-nations.md §0.3` ↔ 작성본 `atlas-jungwon.md ## 배치 다이어그램` ↔ HTTP 응답
모두 byte-exact 정합.

---

## Phase 4 종결 — Done Criteria 매트릭스 (사양 §4)

| Done 항목 | 상태 | 검증 |
|---|---|---|
| 디렉토리 골격 (atlas.rs, markdown/atlas.rs) | ✅ | CP1 |
| Atlas 애그리거트 + AtlasId + AtlasExtent + view 메서드 + 단위 테스트 | ✅ | 도메인 11 단위 (CP1) |
| 마크다운 frontmatter+섹션 파서 + 단위 테스트 (배치 다이어그램 ASCII 보존) | ✅ | 11 단위 (CP1) |
| `genres/wuxia/markdown_template/atlas.md` | ✅ | CP1 |
| `genres/wuxia/forms/atlas.toml` (Phase N 빈 슬롯) | ✅ | CP1 |
| SqliteWorldStore (atlases + atlases_fts + place_atlas_refs + migrate_v4) | ✅ | 7 SQLite 단위 + v3→v4 마이그레이션 (CP1) |
| `bin/world-load` Atlas 외래키 활성 | ✅ | CP1 — atlases=1, fk=0 |
| `bin/mind-studio` MCP 도구 (list_atlases·get_atlas) | ✅ | **CP2** + search_atlases 추가 |
| 체크포인트 1: atlas-jungwon 단독 변환 + 외래키 + ASCII 라운드트립 | ✅ | CP1 — 13 e2e |
| **체크포인트 2: view 메서드 e2e + MCP 정성 평가 + 외래키 결손 0건** | ✅ | **CP2 — 9 e2e + REST 라이브 검증** |
| `cargo build` + `cargo test --features embed` + 기존 e2e 회귀 통과 | ✅ | 회귀 0건 |
| 정성 검증: places_in=11 / settlements_in=8 / geographies_in=3 / adjacent_to(daejin)=3 | ✅ | 자동 + 라이브 |

**모든 Done Criteria 충족**.

---

## Out of Scope (Phase 4) — Phase 5+ 이관 항목 (사양 §7)

명시적으로 미구현 + Phase 5+에서 다룰 항목 (재확인):
- 절대 좌표·SVG·hex grid (Phase N+)
- **Era overlay** (시기별 정치 지도) — Phase 5 Era 결합 시 `extras.era_id` 외래키 활성 +
  통일제국 시대(`atlas-daejin-empire` 등) 별 atlas 인스턴스로 분기.
- **View trait 일반화** — Phase 5 Timeline 등장 시 두 번째 view 패턴과 함께 추출.
- distance matrix·세력권 자동 계산 (Phase N+)
- Atlas 간 hierarchy (continent → region → city-map drilldown) — Phase 5+
- 다중 atlas (예: 동해 군도 별도 atlas) — Phase 5+
- AI 자동 다이어그램 생성 (Phase N+)
- Mind Studio worldbuilding UI 패널 (Phase N+)
- gameplay 다리 (Scene·Beat·관계 시드) — Phase 5+

---

## Phase 5 (Event + Era + Timeline view) 진입 가능 여부 의견

**진입 가능**.

체크포인트 2 산출물이 Phase 5의 전제를 모두 충족:
1. **Atlas 도메인 + view 메서드 패턴 정착** — `places_in`/`settlements_in`/`geographies_in`/
   `adjacent_to` 4 view가 atlas-jungwon에서 정확히 작동. Timeline은 `events_in`/
   `eras_in`/`events_during(era)` 같은 view를 같은 트레이트로 일반화 가능.
2. **외래키 매트릭스 활성** — Phase 4 `atlases.references` hard-fail 패턴이 Phase 5
   `events.era_id`·`atlases.extras.era_id` 활성에 그대로 확장.
3. **MCP `list_*`/`get_*`/`search_*` 3종 패턴** — 4 도메인 모두 일관 정착. Event/Era도
   동일 패턴.
4. **`schema_meta v4` + 마이그레이션 패턴** — Phase 5 진입 시 `migrate_v5` (events·eras·
   event_era_refs 등)가 Phase 1·2·3·4 마이그레이션 패턴 그대로.
5. **Atlas+Era overlay 자리 명시** — `Atlas.extras.era`·`era_id` 텍스트 보존 + 사양에
   "Phase 5 외래키 활성" 명기. atlas-jungwon이 단일 시점인 점도 명확히 문서화.

**Phase 5 권장 입력**:
- `wuxia-core/docs/world/history.md` 등 시계열 캐논.
- atlas-jungwon에 era_id 외래키 활성 후 `atlas-daejin-empire` 등 시기별 분기.
- View trait 일반화 — Atlas의 `places_in` + 새로 추가될 Timeline의 `events_in`을 공통
  `View<Item>` 트레이트로 추출.

> 사양 §3.7 게이트 그대로 — Phase 5는 별도 TASK(`task-phase5-...md`)로 분리. 본 보고서
> 통과 후 Phase 5 작전 작성 진입.

---

## 다음 의견

- 본 commit 범위: Step 4 (REST + MCP + e2e + 정성 평가) 단일 commit. 체크포인트 1
  commit(`a12a5d5`)과 분리 — "1회 통합 commit 금지" 게이트 준수.
- Phase 4 종결 후 Phase 5 (Event + Era + Timeline view) 작전 작성 권장.
- 막힌 결정 없음 — 디렉터 4건 결정이 체크포인트 1 통과 후 모두 처리됨.
