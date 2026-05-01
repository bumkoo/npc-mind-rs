# Worldbuilding Tool — Phase Roadmap

> 전체 Phase 흐름·진행 상태·핵심 결정의 한 곳 진실.
> 새 Phase 시작·결정 변경 시 이 문서와 메모리를 같이 갱신.

## 1. 도구 정체성

**NPC Mind Studio = 장르 중립 Worldbuilding 도구.** 무협(칠국춘추)은 첫 프로젝트일 뿐, SF·판타지 등 다른 장르도 같은 도구로 만들 수 있다.

**산출물 흐름**:
```
사용자 인터뷰/마크다운(SoT)
   ↓ AI 협업 (Lore RAG)
DDD 도메인 (Rust struct)
   ↓ 빌드 산출물
인프라 (SQLite + EventBus + MCP)
   ↓
Mind Studio UI / 게임 엔진
```

**3 계층 분리** (절대 섞지 말 것):
- `src/` = 도구 (장르 영원히 모름)
- `genres/<name>/` = 장르 패키지 (도구 위에 입히는 옷)
- `projects/<name>/` = 사용자 산출물 (마크다운 SoT + 빌드 SQLite)

## 2. 추상 분류 — 10 도메인

```
9 인스턴스 도메인 (실체)              1 관계 도메인 (현재)
─────────────────────                 ─────────────────────
Place · Person · Group · Item        Atlas
Skill · Knowledge · Lore             — 도메인이면서 뷰의 이중성
Event · Era                          — 고유 상태 (좌표·projection·격자)
                                     — 고유 로직 (거리·세력권·인접)
                                     — view 인터페이스 (합성 결과 노출)
```

**미래 관계 도메인 패밀리 후보** (Phase 5+ 자연 추가):
Timeline (Event × Era) · OrgChart (Group × Group) · FamilyTree (Person × Group) · SkillTree (Skill 집합).

## 3. 작업 순서·이유

```
Phase 0: Lore RAG MCP                   ✅ 완료 (2026-04-29)
Phase 1: Group                          🔄 시작 대기 — 작전 완성됨
Phase 2: Person                         ⏳ 예정
Phase 3: Place                          ⏳ 예정 (구 Phase 1 — 보존)
Phase 4: Atlas (Place의 view 도메인)    ⏳ 예정
Phase 5: Event + Era 결합              ⏳ 예정
Phase 6+: Skill · Item · Knowledge · Lore
Phase N: 폼 시스템 · AI 협업 빈칸 · UI 패널
```

**왜 Group 먼저** (2026-04-30 결정 — 원래 Place였음):
1. **추상 누수 방지** — Place 먼저 짜면 Group 책임(통치체·강호 결사·세가)이 Place의 kind·extras로 흘러들어옴. 나중 Group 진입 시 Place 양식 재손질 필요.
2. **게임 서사 비중** — 사용자 비전 "액션보다 인물·갈등 중심". 게임 동력은 Group·Person 관계망.
3. **시드 자료 풍부** — 칠국 통치 집단 + 구파일방 + 천마신교 + 십상시 등 wuxia-core/docs에 풍부.

**의존 관계**:
- Group의 `members` (Person ID) → Phase 2에서 외래키 활성
- Group의 `headquarters` (Place ID) → Phase 3에서 외래키 활성
- Place의 `controlling_group` (Group ID) → Phase 1 정의 후 Phase 3에서 활성
- Atlas의 `references` (Place ID) → Phase 3 후 Phase 4에서 활성

## 4. Phase 한 페이지 요약

| # | 카테고리 | 시작 조건 | 검증 게이트 | 상태 | TASK 파일 |
|---|---|---|---|---|---|
| 0 | Lore RAG | — | 22(공인 PD 3) 자료 인덱싱 + MCP 도구 3 | ✅ 완료 | `task-phase0-lore-rag-bootstrap.md` |
| 1 | Group | Phase 0 | 6 Group + temporal·parent·allied/rival 검증 | ✅ 완료 (2026-04-30) | `task-phase1-group-vertical-slice.md` + `phase1-implementation-report.md` |
| 2 | Person | Phase 1 | 5-6 Person + Group 외래키 활성 + **NPC mind 자동 등록** | 🔄 작전 완성 | `task-phase2-person-vertical-slice.md` |
| 3 | Place | Phase 2 | 7국 + 자연 1-2 + Atlas는 분리 | ⏳ | `task-phase3-place-vertical-slice.md` (보존) |
| 4 | Atlas | Phase 3 | atlas-jungwon + 좌표·로직 + Place 합성 view | ⏳ | (Phase 3 TASK에서 분리 예정) |
| 5 | Event + Era | Phase 4 | 270년 28사건 + Era 결합 | ⏳ | — |
| 6 | Skill | Phase 5 | 무공 5종 + 사문 외래키 | ⏳ | — |
| 7 | Item | Phase 5 | 보물·신검 + Person 외래키 | ⏳ | — |
| 8 | Knowledge·Lore | — | 학문·예술 / 짐승·영물 | ⏳ | — |
| N | 폼 시스템 + AI 협업 + UI | Phase 1+ | 인터뷰 폼 → AI 빈칸 → 마크다운 | ⏳ | — |

## 5. 각 Phase 상세

### Phase 0 — Lore RAG MCP 부트스트랩 ✅

**완료 (2026-04-29)**

목표: 장르 원전을 임베딩+RAG+MCP로 인덱싱. 이후 모든 Phase에서 작전·검증 시 RAG 호출 (부트스트랩 패턴 — 메타 도구·실제 도구 양쪽 사용).

산출:
- `src/lore/` 모듈 + `bin/lore-ingest` CLI + `bin/mind-studio` MCP 통합
- 3 PD 원전 인덱싱: 水滸傳(張啟疆)·江湖奇俠傳·蜀山劍俠傳 — 17,213 청크, ~100MB SQLite
- MCP 도구 3개: `search_lore` · `list_corpora` · `get_chunk`

검증: Cowork-side 직접 평가 — 10 쿼리 (KO 5 + ZH 5) + shuihuzhuan 단독 + get_chunk 문맥 확장. 한국어/중국어 cross-lingual 양호, D2 마크업 잔여 없음, chunk_id 결정성·overlap 작동 확인.

cleanup 후속: 노이즈 청크 필터(Cover·封面·目錄·짧은 청크) Claude Code에 위임 → 반영 완료.

### Phase 1 — Group Vertical Slice ✅

**완료 (2026-04-30)** — 보고서 `phase1-implementation-report.md`

목표: 첫 인스턴스 도메인 = Group을 끝까지 한 사이클. 시간성·멤버십·외래키(parent/allied/rival)·진영(wuxia extras) 검증.

산출:
- 6 Group 변환 (대진 황실·십상시·남궁가·무림맹·천마신교·개방)
- `src/domain/world/group.rs` + `src/worldbuilding/markdown/{frontmatter,group}.rs` + `src/adapter/sqlite_world.rs` + `src/bin/world_load.rs`
- MCP 도구 3개: `list_groups` · `get_group` · `search_groups`
- 14 e2e 테스트 (라운드트립·필터·외래키·검색 자동 검증)
- 인프라 결정 9개 (sync trait·FTS+LIKE fallback·alignment 캐시 컬럼 등)

검증: 빌드·테스트 278+298+14 pass. 변환 결정 16개 + 인프라 결정 9개 합리적. 데이터 손실 없음.

알려진 follow-up:
- **체크포인트 분리 게이트 미준수** — 1회 commit 통합. Phase 2부터 강제 준수
- `serde_yaml` 0.9 deprecated — `serde_yml` 마이그레이션 (선택)
- Mind Studio MCP 직접 호출 평가는 `.mcp.json` env 설정 후 가능 (Phase와 별도)

검증 게이트: 5-6 Group 변환
- `group-daejin-court` (dynasty-court) ★ 체크포인트 1
- `group-namgung` (clan)
- `group-mulim-mang` (alliance — 270년)
- `group-shipsangsi` (covert-band, parent=대진 황실 — 수직 시연)
- `group-cheonma-shingyo` (sect-religious — 사파)
- `group-gaebang` (mendicant-order — 옵션)

도메인 핵심: `temporal`(founded·dissolved·status·notes) · `members`(Person ID 텍스트) · `headquarters`(Place ID 텍스트) · `parent_group` · `allied_groups` · `rival_groups` · `aliases`. 진영·무협 특화 관계는 wuxia 패키지 책임.

체크포인트 1: 대진 황실 단일 변환 — 십상시 분리 확정, allied/rival 후보 결정, alignment=imperial 시연.
체크포인트 2: 5-6 Group + MCP 정성 평가 — rival 대칭(무림맹↔천마신교)·alignment 필터·parent_group 수직 시연.

### Phase 2 — Person Vertical Slice + NPC Mind 통합 🔄

**작전 완성 (2026-04-30)** — TASK `task-phase2-person-vertical-slice.md`

목표: 두 번째 인스턴스 도메인 + worldbuilding ↔ npc-mind 첫 다리.

세 결: (1) Person 도메인 (2) Phase 1 Group 외래키 활성 (members.person_id·affiliation) (3) NPC Mind 자동 등록 (world-load → NpcRepository upsert).

핵심 결정 (5개 모두 사용자 confirm):
- **Q1·B**: HEXACO 6 dim frontmatter 일급, 24 facet은 extras·본문
- **Q2·B**: Player Character는 `Person.kind="player"` sub-kind (별도 카테고리 X)
- **Q3·C**: status(alive/dead/missing/unknown) × kind(historical/active/legendary/player) 두 축
- **Q4·A**: 첫 변환 = `npc-02 조고` (체크포인트 1)
- **Q5·A**: NPC mind 통합을 Phase 2에 활성화. world-load가 `NpcRepository::upsert` 자동 호출. HEXACO·name 갱신, emotion_state·scene·memory는 보존(idempotent)

검증 게이트: 조고 단독 변환 후 mind-studio에서 `dialogue_start("npc-02")` 동작. 체크포인트 1·2 분리 게이트 **강제 준수** (Phase 1 미준수 후속).

### Phase 3 — Place Vertical Slice ⏳

**구 Phase 1, 2026-04-30 연기.** TASK 보존: `task-phase3-place-vertical-slice.md`.

목표: 공간 도메인. Settlement·Geography 두 layer. 칠국 7개 + 서부 산악지대 1개 시범.

진입 시 추가 손질:
- sect kind에 `controlling_group_id` 외래키 활성화 (Group 정의된 후라)
- Group·Person 외래키 검증 추가
- Atlas 부분은 Phase 4로 분리

도메인 핵심: layer(Settlement|Geography) · kind · aliases · summary · spatial(parent_place·atlas·bordering·geography_refs) · extras · body_sections.

### Phase 4 — Atlas (Place의 view 도메인) ⏳

목표: 첫 관계 도메인. 도메인+뷰 이중성 검증. 좌표계·projection·distance·세력권 로직.

예정: `atlas-jungwon` (칠국 대륙) — seven-nations.md §0.3 ASCII 다이어그램 시드.

핵심 결정 예상:
- schematic projection만 Phase 4, 좌표·SVG는 Phase N
- Era overlay (시기별 정치 지도) 분리 시점 — Phase 5 Era 결합 시
- View trait 일반화 (두 번째 view 등장 시 = Phase 5 Timeline)

### Phase 5 — Event + Era 결합 ⏳

목표: 시간 축 + 사건. 270년 역사 28사건 + 5 시대 (history.md).

핵심: Era는 인스턴스 도메인이지만 Timeline view(Event × Era)도 함께 등장 — 두 번째 관계 도메인.

### Phase 6+ — Skill · Item · Knowledge · Lore ⏳

각 Phase별 vertical slice 패턴 반복. Phase 1-3에서 굳은 frontmatter·SQLite·MCP 패턴을 미러링.

### Phase N — 폼 시스템 · AI 협업 빈칸 · UI 패널 ⏳

목표: **인터뷰형 worldbuilding agent**. 사용자가 폼 답 → AI가 Lore RAG로 빈칸 채움 제안 → 정형 마크다운 산출.

진입 조건: 카테고리 3-5개에서 양식이 안정된 후. Phase 1-3 끝난 시점.

산출: Mind Studio worldbuilding 패널 (Vite+React+Zustand). 카테고리별 폼 + AI 빈칸 채움 + 마크다운 미리보기 + 일관성 검증.

## 6. 누적 핵심 결정 (Decision Log)

| 일자 | 결정 | 출처 |
|---|---|---|
| 2026-04-29 | npc-mind-rs를 장르 중립 worldbuilding 도구로 확장 | 사용자 |
| 2026-04-29 | wuxia-core src 폐기, docs만 활용 | 사용자 |
| 2026-04-29 | SoT = 마크다운, JSON+SQLite는 빌드 산출물 | 사용자 |
| 2026-04-29 | 큰 작업: docs/tasks/*.md → Claude Code → 보고서 → Cowork 리뷰 | 사용자 |
| 2026-04-29 | Lore RAG = 부트스트랩 (메타·실제 양쪽) | Cowork |
| 2026-04-29 | 첫 9 추상 카테고리 (15→9 매핑) | Cowork |
| 2026-04-30 | Atlas 추가 → 10 도메인. 후 *도메인+뷰 이중성*으로 재정리 | Cowork ↔ 사용자 |
| 2026-04-30 | Place layer 분화 (Settlement vs Geography) | Cowork |
| 2026-04-30 | aliases·parent_place core 도메인 편입 | 외부 리뷰 |
| 2026-04-30 | 작업 순서 Group → Person → Place → Atlas로 재조정 | 사용자 |
| 2026-04-30 | Group의 allied/rival·alignment(wuxia)·enmity/fellowship(wuxia) | 외부 리뷰 |
| 2026-04-30 | 십상시 별도 Group + parent_group=대진 황실 | 외부 리뷰 |
| 2026-04-30 | Group은 인스턴스 도메인, Atlas는 관계 도메인 (다른 결) | 사용자 ↔ 외부 리뷰 |
| 2026-04-30 | Phase 1 완료 — 6 Group + MCP + e2e 14 pass | Claude Code |
| 2026-04-30 | 체크포인트 분리 게이트 Phase 2부터 강제 준수 (Phase 1 미준수 후 결정) | Cowork |
| 2026-04-30 | Phase 2 결정 5개 — HEXACO 6 dim 일급 / Player=Person sub-kind / status×kind 두 축 / 첫 변환=조고 / NPC mind 자동 upsert | 사용자 |

## 7. 청강만리 vs 칠국춘추 (혼동 주의)

- **청강만리(青江萬里)** = 사용자 영감 받은 기존 무협지(와호장룡 관련). **프로젝트 이름 아님.**
- **칠국춘추(七國春秋)** = 사용자 직접 작성 중인 오리지널 세계관 = 프로젝트 = `projects/chilguk-chunchu/`

슬러그는 한국어 발음(`chilguk-chunchu`). 중국어 병음·영문 의미역 사용 안 함.

## 8. 입력 자료 위치

- 무협 원전: `wuxia-core/docs/Chinese-Literature/` (PD 3권만 인덱싱, 나머지는 라이선스 검증 후)
- 중국 정사: `wuxia-core/docs/Chinese-History/` (사기·한서, 추후 인덱싱)
- 칠국춘추 시드:
  - `wuxia-core/docs/world/seven-nations.md` — 칠국 v1.1 (1076줄)
  - `wuxia-core/docs/world/history.md` — 270년 연표
  - `wuxia-core/docs/world/history-characters.md` — 역사 인물·문파 배치
  - `wuxia-core/docs/characters/character-roster.md` — 인물 총람 v1.1
  - `wuxia-core/docs/characters/npc-01~11.md` — NPC 11명 열전

## 9. 진행 가이드

**새 Phase 진입 시**:
1. 이전 Phase의 done criteria 충족 확인
2. 본 로드맵에서 다음 Phase 항목 확인
3. `docs/tasks/task-phase{N}-{slug}.md` 작성 (자급 자족 형식, 10 섹션)
4. Claude Code에 위임 → 체크포인트 보고서 → Cowork 리뷰

**핵심 결정 변경 시**:
1. §6 Decision Log에 한 줄 추가
2. 영향 받는 Phase 항목 갱신
3. 메모리 (`memory/project_worldbuilding_tool.md`) 동기화

**Phase 상태 갱신**:
- ✅ 완료 (체크포인트 2 통과)
- 🔄 진행 중 (체크포인트 1·2 사이)
- ⏳ 예정 (TASK 작성 전)
- 작전 완성 (TASK 작성 됐으나 Claude Code 위임 전)
