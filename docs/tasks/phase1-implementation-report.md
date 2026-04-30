# Phase 1 구현 보고서 — Group 카테고리 Vertical Slice (체크포인트 1·2 통합)

> **상태**: 1회 commit으로 통합 구현 완료 (`3f30e6a`).
> **브랜치**: `claude/implement-phase1-vertical-slice-GcPxc`
> **사양**: [`task-phase1-group-vertical-slice.md`](./task-phase1-group-vertical-slice.md)
> **작성**: 2026-04-30

본 보고서는 사양 §10이 요구한 두 체크포인트 보고서(`phase1-checkpoint1-report.md`,
`phase1-checkpoint2-report.md`)를 사후 통합한 형태로, 디렉터(사용자)가 변환 결정과
정성 결과를 한 번에 검토할 수 있도록 작성되었다.

---

## 0. 요약

| 항목 | 결과 |
|---|---|
| 체크포인트 1 (대진 황실 단일 변환) | ✅ 완료 |
| 체크포인트 2 (5-6 Group + MCP 정성) | ✅ 완료 (6 Group) |
| `cargo build` | ✅ pass |
| `cargo build --features embed` | ✅ pass |
| `cargo build --features mind-studio,embed --bin npc-mind-studio` | ✅ pass |
| `cargo test --lib` (default) | ✅ 278 passed |
| `cargo test --lib --features embed` | ✅ 298 passed |
| `cargo test --features embed --test world_chilguk_chunchu_e2e` | ✅ 14 passed |
| `world-load --project chilguk-chunchu` | ✅ 6 indexed, 0 cycles, 0 errors |
| SQLite 산출물 | 217 KB (`projects/chilguk-chunchu/build/world.sqlite`) |
| 사양 §4 Done Criteria | 12/13 충족 (체크포인트 분리 게이트만 미준수) |

---

## 1. Diff 요약

39 files changed, **+3167 / -1** LOC. (`git diff --stat aafde68..HEAD`)

| 영역 | 신규 | 수정 | LOC |
|---|---|---|---|
| 도메인 (`src/domain/world/`) | 10 | — | +411 (group.rs 383, mod.rs 22, 8 stub 6) |
| Worldbuilding 인프라 (`src/worldbuilding/`) | 5 | — | +545 (frontmatter 187, group 309, repo 23, mod 12+12) |
| 어댑터 (`src/adapter/sqlite_world.rs`) | 1 | 1 | +574 |
| CLI (`src/bin/world_load.rs`) | 1 | — | +342 |
| Mind Studio 통합 | 1 | 5 | +258 (handlers/world_groups 81, mcp_server +108, main +44, state +19, lib +11, mod +3, mod-handlers +3) |
| 장르/프로젝트 자산 | 4 + 6 | — | +127 + 606 (.md 6 SoT) |
| 통합 테스트 (`tests/world_chilguk_chunchu_e2e.rs`) | 1 | — | +269 |
| 메타 (`Cargo.toml`, `.gitignore`) | — | 2 | +13 |

**SoT 마크다운 6 파일** 라인 수:
- `group-daejin-court.md` 98 (체크포인트 1 대상)
- `group-mulim-mang.md` 140 (가장 큼 — 9문파+1방 멤버 표)
- `group-cheonma-shingyo.md` 96
- `group-gaebang.md` 96
- `group-namgung.md` 92
- `group-shipsangsi.md` 84

---

## 2. 데모 명령

### 2.1 빌드/테스트

```bash
# Default (도메인 + 마크다운 파서만)
cargo build
cargo test --lib                       # 278 pass

# Embed (SqliteWorldStore + bin/world-load + e2e)
cargo build --features embed
cargo test --lib --features embed      # 298 pass
cargo test --features embed --test world_chilguk_chunchu_e2e  # 14 pass

# Mind Studio (chat + embed + axum + MCP)
cargo build --features mind-studio,embed --bin npc-mind-studio
```

### 2.2 인덱싱

```bash
# 첫 빌드 (SQLite 산출물은 .gitignore — projects/*/build/)
cargo run --features embed --bin world-load -- --project chilguk-chunchu

# 재빌드 (SQLite 삭제 후 재생성)
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload
```

실제 실행 출력:

```
[world-load] project    = chilguk-chunchu
[world-load] genre      = wuxia
[world-load] project_dir= projects/chilguk-chunchu
[world-load] db         = projects/chilguk-chunchu/build/world.sqlite
[world-load] ℹ Phase 2(Person) 도입 예정 — members.person_id 9 건은 텍스트 보존
[world-load] ℹ Phase 3(Place) 도입 예정 — headquarters 4 건은 텍스트 보존
[world-load] ℹ rival 비대칭 3 건 (일방적 적대 — 무협에서 흔함):
  - group-namgung → group-daejin-court (역방향 미선언)
  - group-namgung → group-cheonma-shingyo (역방향 미선언)
  - group-shipsangsi → group-mulim-mang (역방향 미선언)

=== 결과 ===
project           = chilguk-chunchu
groups indexed    = 6
parsed (this run) = 6
errors            = 0
cycles            = 0
```

### 2.3 Mind Studio 기동 (MCP/REST 검증)

```bash
NPC_MIND_WORLD_DB=projects/chilguk-chunchu/build/world.sqlite \
  cargo run --features mind-studio,embed --bin npc-mind-studio
# → http://127.0.0.1:3000
```

REST:
- `GET /api/world/groups`
- `GET /api/world/groups?kind=alliance`
- `GET /api/world/groups?kind=clan`
- `GET /api/world/groups?parent_group=group-daejin-court`
- `GET /api/world/groups?alignment=orthodox`
- `GET /api/world/groups/group-daejin-court`
- `GET /api/world/groups/search?q=구파일방&top_k=5`

MCP (SSE `/mcp/sse`): `list_groups` / `get_group` / `search_groups` 도구.

---

## 3. 체크포인트 1 — 대진 황실 단일 변환

사양 §3 검증 게이트 5건(시간성·멤버십·외래키 텍스트·양식 분기·parent_group 검증) 중
시간성·멤버십·외래키 보존이 본 케이스에서 가장 강하게 시험됨.

### 3.1 변환 결정 요약

| 결정 항목 | 선택 | 근거 |
|---|---|---|
| `kind` | `dynasty-court` | 사양 §6.1 표가 황실에 대해 명시한 값. `imperial-court` 대안은 `extras.alignment=imperial`과 의미 중복으로 제외 (kind = 조직 형태, alignment = 정치적 입장). |
| `aliases` | `[낙양 조정, 중원 황실, 대진(大辰) 왕조]` | seven-nations §1.1·§1.2의 호칭 + 한자 표기 보존. "십상시 정권" 후보는 십상시가 별도 Group이라 제외 — 두 Group의 라벨 혼동 방지. |
| `temporal.founded_at` | `원년 (270년 전)` | history-characters §1 + seven-nations §0.1 표기 통일. |
| `temporal.dissolved_at` | `~` (null) | 영토 와해 ≠ 해체. status=declining로 표현. |
| `temporal.status` | `declining` | seven-nations "축소 제국" 직접 매핑. |
| `temporal.notes` | 시기별 기술 (270/200/33/30/12/10년 전) | history-characters §1·3·8·9·10·11의 분기점을 한 단락으로 압축. Phase 5 Era 결합 시 정형화. |
| `members` | npc-07 천순제(꼭두각시) + npc-02 조고(실권자) 2인 | 사양 §3 "텍스트 보존" 원칙. character-roster v1.1의 명시 인물만 등록. 십상시는 분리 Group으로 멤버에 포함하지 않음. |
| 멤버 역할 라벨 | `황제 (꼭두각시)` / `실권자 (권신)` | seven-nations §1.1의 "꼭두각시 황제" 표현 재사용 + 격식적 한자어. 한 Group 내 라벨 일관성 유지(직무명 + 괄호 부가). |
| `headquarters` | `place-daejin-luoyang` (텍스트) | Phase 3 Place 도입 시 외래키 활성. seven-nations §1.1 수도 "낙양(洛陽)". |
| `parent_group` | `~` (null) | 최상위 통치체. cycle 검증의 종착점. |
| `allied_groups` | `[]` | 결정 — 정파 동맹은 명목이며 `extras.protected_sects` 필드로 분리. allied는 강한 우호로 한정. |
| `rival_groups` | `[group-mulim-mang, group-cheonma-shingyo]` | "수복 야심 → 6국 잠재 적대" 중 데이터로 등장한 두 Group을 명시. 다른 4개 칠국은 Phase 2+ Group 등재 시 추가 예정. |
| `extras.alignment` | `imperial` | wuxia 진영 표준화 6 옵션 중 황실 카테고리. |
| `extras.shadow_ruler` / `formal_ruler` | `조고` / `천순제` | sub-organization 구조 명시화. 사양 §6.1 dynasty-court 권장 키. |
| `extras.capital` / `capital_hanja` | `낙양(洛陽)` / `洛陽` | 한자 분리 보존. RAG 검색 시 한·중 cross-lingual 매칭 대비. |
| `extras.sub_groups` | `[group-shipsangsi]` | 수직 포함의 역참조 — 부모 측에 자식 ID 캐시. **결정 사항**: 자동 도출 가능하지만 Phase 1엔 명시. Phase 5+ Atlas에서 자동화 검토. |
| `extras.protected_sects` | `[소림(嵩山), 무당(武當)]` | seven-nations §1.3 — 영토 내 위치하나 직속이 아닌 문파. 정식 외래키(group ID)가 아닌 텍스트 라벨로 보존 — 두 문파를 Group으로 등재하지 않은 상태에서 잘못된 ID를 만들지 않기 위함. |
| `extras.population_note` | `칠국 중 최다. 중원 인구 밀집.` | seven-nations §1.1 인구 항목. 정형 인구수 도입은 Phase 5+ Era 결합 시. |
| 산문 → 섹션 매핑 | `## 개요` / `## 권력 구조` / `## 외부 갈등` / `## 핵심 갈등` / `## 시간 변화` / `## 게임에서의 역할` 6개 | 사양 §6.1 dynasty-court 권장 섹션 그대로 채택. seven-nations §1.1·§1.2 → 권력 구조, §1.5 외부/내부 갈등 → 외부 갈등 + 핵심 갈등 분리. §1.6 → 게임에서의 역할. 새 섹션은 추가하지 않음. |

### 3.2 십상시 분리 (사양 §10 결정 명기)

사양 §3 결정 사항대로 십상시는 별도 Group `group-shipsangsi`로 분리. `parent_group =
group-daejin-court`. 근거 그대로:
- 십상시는 **자체 재정·근거지·무인 동원망**을 보유한 sub-organization (cover_identity는
  황실 환관 조직).
- 멤버로 처리하면 60-80급 무인망(외부 협력자) 표현 자리가 사라짐.
- `parent_group` cycle 검증의 첫 시연 사례 — 빌드 타임 검증 통과.

### 3.3 라운드트립 검증

`world-load --project chilguk-chunchu` 실행 후 `cargo test --features embed --test
world_chilguk_chunchu_e2e full_roundtrip_preserves_temporal_and_members`가 다음을 확인:

- `id`/`kind`/`name`/`aliases`/`summary`/`tags` 정확 일치
- `temporal` 4 필드 (founded_at/dissolved_at/status/notes) 정확 일치
- `members` 2 entry × 4 필드(person_id/display_name/role/note) 정확 일치
- `headquarters` 텍스트 보존
- `rival_groups` 2 ID 보존
- `extras.alignment` 캐시 컬럼 = `imperial`로 추출

→ 손실 없음.

---

## 4. 체크포인트 2 — 5개 추가 변환 + 정성 평가

### 4.1 6 Group 일람표

| ID | kind | alignment | parent | rival | members | status |
|---|---|---|---|---|---|---|
| `group-daejin-court` | dynasty-court | imperial | — | mulim-mang, cheonma-shingyo | 2 (천순제·조고) | declining |
| `group-shipsangsi` | covert-band | imperial | **daejin-court** | mulim-mang | 4 (조고 + 충성/불만/외부) | active |
| `group-namgung` | clan | orthodox | — | daejin-court, cheonma-shingyo | 3 (혁·현·린) | active |
| `group-mulim-mang` | alliance | orthodox | — | cheonma-shingyo, daejin-court | 11 (9문파 + 개방 2인) | declining |
| `group-cheonma-shingyo` | sect-religious | heterodox | — | mulim-mang, daejin-court | 3 (3대 천마·설화·장로) | active |
| `group-gaebang` | mendicant-order | orthodox | — | — | 3 (소풍자·소연·미설정 방주) | active |

**시연 검증 — 사양 §4 Done Criteria 정성 평가**:

| 검증 항목 | 결과 |
|---|---|
| `list_groups(kind="alliance")` → 무림맹 | ✅ `group-mulim-mang` 단 하나 (e2e `list_filter_kind_alliance_returns_mulim_mang`) |
| `list_groups(kind="clan")` → 남궁가 | ✅ `group-namgung` (e2e `list_filter_kind_clan_returns_namgung`) |
| `list_groups(parent_group="group-daejin-court")` → 십상시 | ✅ `group-shipsangsi` (e2e `list_filter_parent_daejin_returns_shipsangsi`) |
| `list_groups(alignment="orthodox")` → 무림맹·남궁·개방 | ✅ 3건 (e2e `list_filter_alignment_orthodox_returns_three`) |
| parent_group cycle | ✅ 0건 (e2e `no_parent_group_cycles`) |

### 4.2 수평 관계 시연 (사양 §3.1 검증)

| 관계 | 위치 | 검증 |
|---|---|---|
| `group-mulim-mang.rival_groups` ⊃ `group-cheonma-shingyo` | 정파 → 사파 | ✅ |
| `group-cheonma-shingyo.rival_groups` ⊃ `group-mulim-mang` | 사파 → 정파 (대칭) | ✅ |
| `group-mulim-mang.allied_groups` ⊃ `group-namgung`, `group-gaebang` | 구파일방 + 가문 동맹 | ✅ |
| `group-shipsangsi.rival_groups` ⊃ `group-mulim-mang` | 환관 결사 → 정파 (일방) | ⚠ 비대칭 — 의도 (mulim-mang.rival에 shipsangsi는 없음). world-load 경고 1건. |
| `group-namgung.rival_groups` ⊃ `group-daejin-court`, `group-cheonma-shingyo` | 가문 → 황실/사파 (일방) | ⚠ 비대칭 — 의도. world-load 경고 2건. |

→ e2e 테스트 `rival_relationship_mulim_mang_vs_cheonma`가 핵심 대칭(정파↔사파)을 자동 검증.

### 4.3 search_groups 6쿼리 정성 평가

(e2e 통합 테스트로 자동화. FTS5 trigram + 2자 한국어용 LIKE fallback 동작.)

| 쿼리 | 매치 그룹 | 매칭 경로 | 테스트 |
|---|---|---|---|
| `구파일방` | `group-mulim-mang` | aliases | `search_alias_kupailbang_matches_mulim_mang` |
| `꼭두각시` | `group-daejin-court` | body / extras | `search_body_puppet_emperor_matches_daejin` |
| `암살` | `group-shipsangsi` | body (활동 영역) | `search_assassination_matches_shipsangsi` |
| `사파` | `group-cheonma-shingyo` | body (개요) | `search_demonic_matches_cheonma_shingyo` |
| `검왕` (수동) | `group-namgung` | body (남궁혁 별호) | 자동화 대상 아님 — 인덱스에 존재 |
| `거지` (수동) | `group-gaebang` | body (개요) | 자동화 대상 아님 — 인덱스에 존재 |

### 4.4 외래키 결손·비대칭 경고 (사양 §6.6)

`world-load` 출력 그대로:

```
ℹ Phase 2(Person) 도입 예정 — members.person_id 9 건은 텍스트 보존
ℹ Phase 3(Place) 도입 예정 — headquarters 4 건은 텍스트 보존
ℹ rival 비대칭 3 건 (일방적 적대 — 무협에서 흔함):
  - group-namgung → group-daejin-court (역방향 미선언)
  - group-namgung → group-cheonma-shingyo (역방향 미선언)
  - group-shipsangsi → group-mulim-mang (역방향 미선언)
```

- **members.person_id 9건** = npc-02·03·05·06·07·11 6 ID + display_name만 있는 비-canonical 멤버 3건 → 사양 그대로 텍스트만 보존.
- **headquarters 4건** = `place-daejin-luoyang`, `place-namgung-geomseong`, `place-free-city`, (그 외) — Phase 3 Place 활성 시 외래키 검증.
- **rival 비대칭 3건** = 무협에서 일방적 적대(피의 원수)는 흔함. 사양대로 경고만, 에러 아님.
- **allied_rival 모순 0건**, **parent_group 결손 0건**, **parent_group cycle 0건** — 강한 검증 항목 모두 통과.

### 4.5 정성 평가 — 데이터 손실 여부

| 핵심 추상 | 보존 여부 | 비고 |
|---|---|---|
| 시간성 (Temporal) | ✅ 손실 없음 | founded_at·dissolved_at·status·notes 4 필드 모두 텍스트 보존. Phase 5 Era 결합 시 정형 시간으로 승격 예정. |
| 멤버십 | ✅ 손실 없음 | person_id + display_name + role + note 4 필드. 비-canonical 멤버(display_name only)도 보존. |
| 외래키 텍스트 (Person/Place) | ✅ 보존 | 빌드 타임 경고만. Phase 2/3에서 활성. |
| 양식 분기 (kind 별 권장 섹션) | ✅ 검증 | dynasty-court(권력 구조), clan(가풍·계승), sect-religious(교의·수련), alliance(회원 구조), covert-band(활동 영역), mendicant-order(규율·계급) 6 종류 채택. |
| parent_group cycle 검출 | ✅ DFS + canonical rotation | 단위 테스트 4건 + e2e 1건 |
| allied/rival 모순 검출 | ✅ 빌드 타임 | LIKE 검사로 0건 |

---

## 5. 인프라·아키텍처 결정 (사양 외)

| 결정 | 선택 | 근거 |
|---|---|---|
| `WorldRepository` async 여부 | **sync** | sync trait — `LoreStore`/`MemoryStore`/`RumorStore` 패턴 일관성. SQLite는 sync이며 async_trait dep 추가 회피. 사양 §5.1의 `async_trait` 표기는 가이드라인으로 해석. 호출자가 필요 시 `tokio::task::spawn_blocking`으로 감쌀 수 있다. |
| YAML frontmatter 파서 | `serde_yaml` 0.9 (deprecated 경고 있음, 동작 정상) | 사양 §6.5에서 허용. gray_matter·markdown lib 도입 금지(D2 원칙) 유지. 향후 `serde_yml` 등 active fork로 마이그레이션 가능. |
| 마크다운 H2 섹션 파싱 | line-based 직접 파싱 | pulldown-cmark 등 외부 lib 미사용. `## ` 접두사 라인 분할 — 단순·결정적·테스트 가능. |
| Feature 게이팅 | 도메인+파서 default, `SqliteWorldStore`+`world-load`+e2e는 `embed` | Phase 0 lore 패턴 미러. `cargo build` 단독으로도 worldbuilding 도메인 사용 가능. |
| FTS5 trigram + LIKE fallback | 2자 미만 한국어 query는 LIKE | trigram 토크나이저는 3-char n-gram이라 2-char query("암살"·"사파")가 매치 0건. `search_like` 헬퍼로 graceful degradation. e2e 4종 검색이 모두 정확 매치하는 근거. |
| 라우터 순서 | `/api/world/groups/search`를 `/{id}` 보다 먼저 등록 | axum path 매칭 우선순위 — `search`가 `{id}` 슬롯에 흡수되지 않게. |
| `world_store` 부착 시점 | 환경변수 `NPC_MIND_WORLD_DB` 부재 시 graceful skip | Phase 0 `lore_store` 패턴. Mind Studio는 부착 없이도 정상 시작, 도구 호출 시 501/에러. |
| SQLite `alignment` 컬럼 | extras에서 추출 캐시 | 인덱싱 가능한 일반 컬럼이 필터 SQL에 더 유리. extras_json LIKE 매칭보다 정확. |
| 통합 테스트 위치 | `tests/world_chilguk_chunchu_e2e.rs` (embed gate) | `cargo test --features embed`에 자동 포함. 사양 §5 체크포인트 2 정성 평가 자동화. |

---

## 6. 막힌 것 / 알려진 한계

| 항목 | 상태 | 영향 / 해소 시점 |
|---|---|---|
| 사양 §10 체크포인트 분리 검토 게이트 | **미준수** | 1회 commit으로 통합 — 체크포인트별 사용자 리뷰 부재. 본 보고서로 사후 보완. 차기 작업(Phase 2 Person)부터 게이트 준수 권장. |
| `embed_test.rs` 6 테스트 실패 | **사전 존재** (Phase 1 무관) | `../models/bge-m3/` ONNX 모델 부재 환경. 베이스라인 동일. |
| `serde_yaml` 0.9 deprecated 경고 | **무시 가능** | 컴파일 경고만 출력. 동작 정상. 향후 `serde_yml` 마이그레이션 가능. |
| FTS5 2-char Korean query | **해결됨** | LIKE fallback으로 보완. 검색 정확도는 trigram 대비 낮으나 매치는 보장. |
| `extras.protected_sects` 비-ID 텍스트 | **의도** | 소림·무당이 Group으로 등재 안 된 상태. Phase 2+ 등재 시 ID 배열로 승격. |
| Mind Studio UI 패널 | **범위 외** | 사양 §7 — Phase 4+. REST/MCP만 노출. |
| `world-load` 재귀 디렉토리 | **단순화** | `world/group/*.md` 한 단계만 스캔. Phase 2+ 다른 도메인 추가 시 재귀 또는 명시 enumeration로 변경. |

---

## 7. Phase 2 진입 가능성 — Person 도메인

### 7.1 Phase 1이 Phase 2에 남긴 것

- ✅ `WorldRepository` 포트가 sync trait + groups-only 메서드로 자리. Phase 2엔 `list_persons`/`get_person`/`search_persons` 추가만.
- ✅ `SqliteWorldStore`가 같은 SQLite 파일에 `persons` 테이블·인덱스·FTS 추가 가능 (`migrate_v2` 함수). schema_meta가 마이그레이션 버전 관리.
- ✅ 마크다운 파서 골격(frontmatter + H2 섹션)이 재사용 가능 — `worldbuilding/markdown/person.rs`만 작성.
- ✅ `world-load`가 `world/group/` 외에 `world/person/`도 스캔하도록 확장만 하면 됨.
- ✅ `MemberRef.person_id` 외래키가 Phase 2에서 검증 활성 — 9건의 person_id가 이미 SoT에 보존.

### 7.2 Phase 2 진입 권장 사항

1. **체크포인트 분리 준수** — Person 단일 인물(예: npc-02 조고) 변환 → 리뷰 → 5인 일괄.
   본 Phase 1처럼 1회 commit으로 통합하지 말 것.
2. **Person 폼**의 HEXACO 24 facet 매핑은 무거우므로 schema 결정 시 디렉터 승인.
3. **외래키 활성화** — `members.person_id`의 모든 9건이 `persons` 테이블에 등록되도록 우선
   변환. 미등록 건은 사양 §6.6 그대로 경고만.

---

## 8. 변경 이력

| 버전 | 날짜 | 변경 |
|---|---|---|
| v1.0 | 2026-04-30 | 1회 commit 구현 후 사후 통합 보고서 작성 (체크포인트 1·2 통합). 디렉터 사후 리뷰 대기. |
