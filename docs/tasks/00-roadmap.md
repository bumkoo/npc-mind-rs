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
Phase 0:   Lore RAG MCP                   ✅ 완료 (2026-04-29)
Phase 1:   Group                          ✅ 완료 (2026-04-30)
Phase 2:   Person + NPC mind 통합         ✅ 완료 (2026-05-01)
Phase 2.1: Player follow-up               ✅ 완료 (2026-05-01)
Phase 2.2: Runtime sync follow-up         ✅ 완료 (2026-05-01)
Phase 3:   Place + Phase 1·2 FK 활성      ✅ 완료 (2026-05-01)
Phase 4:   Atlas (도메인+뷰 이중성)       ✅ 완료 (2026-05-01)
Phase 5a:  Event (인스턴스)               ✅ 완료 (2026-05-02)
Phase 5b:  Era + Timeline + Atlas overlay (View trait 보류)   ✅ 완료 (2026-05-02)
Phase 5c.1: Historical NPCs follow-up (D2 처리)   ✅ 완료 (2026-05-02)
Phase 5c.2: Mid-era Events follow-up               🔄 작전 작성 완료 (Claude Code 대기, 2026-05-03)
Phase 6+:  Skill · Item · Knowledge · Lore   ⏳
Phase N:   폼 시스템 · AI 협업 빈칸 · UI 패널 ⏳
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
| 2 | Person | Phase 1 | 7 Person + Group 외래키 활성 + **NPC mind 자동 등록** | ✅ 완료 (2026-05-01) | `task-phase2-person-vertical-slice.md` + `phase2-checkpoint{1,2}-report.md` |
| 2.1 | Player follow-up | Phase 2 | id="player" + HEXACO 시작값 + mind eligible = 8 | ✅ 완료 | `task-phase2-followup-player-character.md` + `phase2-followup-player-report.md` |
| 2.2 | Runtime sync follow-up | Phase 2 | POST /api/world/persons/sync + emotion 보존 검증 | ✅ 완료 | `task-phase2-followup-runtime-sync.md` + `phase2-followup-runtime-sync-report.md` |
| 3 | Place | Phase 2 | 11 Place(8 settlement+3 geography) + 외래키 0건 + sect/geography_refs 양방향 | ✅ 완료 (2026-05-01) | `task-phase3-place-vertical-slice.md` + `phase3-checkpoint{1,2}-report.md` |
| 4 | Atlas | Phase 3 | atlas-jungwon + references 11 Place + ASCII 4단계 byte-exact + view 메서드 (도메인+뷰 이중성) | ✅ 완료 (2026-05-01) | `task-phase4-atlas-vertical-slice.md` + `phase4-checkpoint{1,2}-report.md` |
| 5a | Event | Phase 4 | 6 Event + participants 외래키 0건 + related_events 양방향 + alias 패턴 일관 | ✅ 완료 (2026-05-02) | `task-phase5a-event-vertical-slice.md` + `phase5a-checkpoint{1,2}-report.md` |
| 5b | Era + Timeline + Atlas overlay | Phase 5a | 5 Era + 1 Timeline + view 메서드 4종 + Atlas overlay 양방향 + 외래키 0건 | ✅ 완료 (2026-05-02) | `task-phase5b-era-timeline-vertical-slice.md` + `phase5b-checkpoint{1,2}-report.md` |
| 5c.1 | Historical NPCs follow-up | Phase 5b | 임서운 + 7 historical/active npc + Phase 5a Event 외래키 갱신 (핵심 분기 0건) | ✅ 완료 (2026-05-02) | `task-phase5-followup-historical-npcs.md` + `phase5-followup-historical-npcs-checkpoint{1,2}-report.md` |
| 5c.2 | Mid-era Events follow-up | 5c.1 | 6 mid-era event (founding 1·prosperity 1·turning 3·decline 1) + era key_events 4종 갱신 + Phase 5a 6 event related_events 역방향 정합 + 5c.1 npc 외래키 활성 | 🔄 작전 작성 완료 (2026-05-03) | `task-phase5-followup-mid-era-events.md` |
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

### Phase 2 — Person Vertical Slice + NPC Mind 통합 ✅

**완료 (2026-05-01)** — 보고서 `phase2-checkpoint{1,2}-report.md`

목표: 두 번째 인스턴스 도메인 + worldbuilding ↔ npc-mind 첫 다리.

세 결 통합:
1. **Person 도메인** — id·kind(active/historical/legendary/player) · status × kind 두 축 · HEXACO 6 dim 일급 · aliases · affiliation · birthplace · current_location · temporal · extras
2. **Phase 1 Group 외래키 활성** — `Group.members.person_id`·`Person.affiliation` 검증 텍스트 → 에러 승급
3. **NPC Mind 자동 등록** — `world-load`가 `NpcRepository::upsert` 자동 호출. HEXACO·name 갱신, emotion_state·scene·memory 보존(idempotent)

산출:
- 7 Person 변환 (체크포인트 1: npc-02 조고 / 체크포인트 2: npc-01·03·04·05·06·07)
- `src/domain/world/person.rs` + `src/worldbuilding/markdown/person.rs` + `src/worldbuilding/mind_sync.rs` + `SqliteWorldStore` migrate_v2 + `bin/world_load` 확장
- MCP 도구 3개: `list_persons` · `get_person` · `search_persons`
- 41 e2e (12 batch + 11 npc-02 + 18 group 회귀) + 342 lib

핵심 결정 (5개 사용자 confirm):
- Q1·B: HEXACO 6 dim 일급
- Q2·B: Player = Person.kind="player" sub-kind
- Q3·C: status × kind 두 축
- Q4·A: 첫 변환 = npc-02 조고
- Q5·A: NPC mind 자동 upsert (Phase 2에)

추가 결정:
- HEXACO 범위 = -1.0~+1.0 + Score VO 재사용 (사용자 코드 정보 + 외부 리뷰)
- 십상시 분리 + parent_group=대진 황실
- Big5 → HEXACO 변환 표 7인 (npc-07만 신뢰도 "낮음", `source_status: heritage-pending`)
- npc-04 빈 affiliation + `extras.pending_groups` 메타 (서량/당가 그룹 부재)

체크포인트 분리 게이트 **정상 준수** (Phase 1 미준수 후속 회복).

#### Phase 2.1 — Player Follow-up ✅

**완료 (2026-05-01)** — 보고서 `phase2-followup-player-report.md`

Q2·B 정책 단독 검증. `player.md` 1개 (110 라인) — id="player" + HEXACO baseline (+0.5/+0.3/0/+0.4/+0.5/+0.5) + 17세 화산파 유일 생존자 배경 + 4 비밀 (혈매화검·임서운 정체 오인·혈교 약물·임서운 생존 가능성).

코드 변경 0 — Phase 2 본문 일반화가 player를 흡수. **Q2·B 정책 정합성 입증**. mind eligible = 8.

#### Phase 2.2 — Runtime Sync Follow-up ✅

**완료 (2026-05-01)** — 보고서 `phase2-followup-runtime-sync-report.md`

POST `/api/world/persons/sync` endpoint 추가 (~35 LOC). 작가가 `world-load --reload` 후 mind-studio 재시작 없이 변경된 HEXACO 반영.

**핵심 발견**: `rebuild_repo_from_inner`이 `inner.emotions`를 명시 재적용 — Phase 2 본문 §3.5 "idempotent + 동적 상태 보존" 보장이 정합. **신규 보존 로직 0 LOC** (사양 §7 추정 50-80 LOC 최악 시나리오 회피).

회귀 가드 e2e: `sync_preserves_emotion_state_across_reloads` — sync 전 감정 설정 → sync 후 감정 유지 자동 검증.

#### npc-11 heritage-pending Stub (사용자 직접 작성)

Phase 2 종결 후 외래키 활성화 시 `npc-11` (소풍자, 개방 장로) FK 결손 2건 발견 (group-gaebang·group-mulim-mang 양쪽 참조). 사용자가 npc-07 패턴 그대로 stub `npc-11.md` 직접 작성 — kind=active, `source_status: heritage-pending`, 잠정 HEXACO. **persons indexed = 9** (8 + npc-11 stub) 도달.

**최종 상태**: Phase 1 6 Group + Phase 2 9 Person (heritage-pending 2명: npc-07·11) + Player 1명. mind eligible = 9.

알려진 한계:
- dialogue_start REST 경로는 `/api/chat/start` (dialogue_start는 MCP 도구 이름) — Phase 2 종결 시점에 명료화
- Mind Studio가 NPC mind 통합 시 Scene·관계·Beat 없이는 의미 있는 dialogue 시연 어려움 → Phase 5+ "두 결의 다리"에서 해결 예정

### Phase 3 — Place Vertical Slice ✅

**완료 (2026-05-01)** — 보고서 `phase3-checkpoint{1,2}-report.md`

세 결 통합 검증:
1. **Place 도메인** — Settlement·Geography 두 layer + spatial(parent_place·bordering·geography_refs) + aliases
2. **Phase 1·2 외래키 활성** — `Group.headquarters`·`Person.birthplace`·`Person.current_location` 텍스트 → 에러 승급. 24 결손 → **0** 도달.
3. **sect kind 이중 등록 양방향** — `place-namgung-sega.controlling_group` ↔ `group-namgung.headquarters`

산출:
- 11 Place 변환 (8 settlement + 3 geography). 6 distinct kind: nation·autonomous-zone·sect·mountain-range·grassland·jungle
- `src/domain/world/place.rs` + `src/worldbuilding/markdown/place.rs` + `SqliteWorldStore` migrate_v3 + `bin/world_load` 외래키 활성
- MCP 도구 3개: `list_places` · `get_place` · `search_places`
- 9 신규 e2e + 7 체크포인트 1 e2e + 회귀 통과 (Phase 1·2 + dispatch + dialogue + director 모두)

핵심 결정 (디렉터 명시 5 + 묵시 14):
- 자유도시·정암·동해 연안·낙양·독관성 모두 city-level 단순화 (Phase 5+ Atlas/Scene 정밀도 복원 가능)
- 자연 지형 시연 = bukwon-grasslands + namman-jungle 둘 다 (distinct kind)
- sect 이중 등록 = place-namgung-sega 1개 (양방향 외래키 시연)
- group-namgung.headquarters → place-namgung-sega 갱신 (sect 양방향)

양방향 외래키 자동 e2e 가드:
- `sect_double_registration_bidirectional`
- `geography_refs_bidirectional_with_bukwon`
- `geography_refs_layer_constraint_holds` (settlement.geography_refs target은 모두 Geography invariant)
- `fk_zero_phase1_phase2_seeds_all_resolve` (회귀 가드)

알려진 한계:
- city-level sub-place(낙양·독관성·검성·뒷골목) 정밀도는 Phase 5+ Atlas/Scene 통합 시 복원 검토
- Atlas는 Phase 4로 분리 (도메인+뷰 이중성, 다음 Phase)

### Phase 4 — Atlas Vertical Slice ✅

**완료 (2026-05-01)** — 보고서 `phase4-checkpoint{1,2}-report.md`

목표: 첫 관계 도메인. 도메인+뷰 이중성 검증. 미래 관계 도메인 패밀리(Timeline·OrgChart·FamilyTree·SkillTree)의 첫 사례.

세 결:
1. **Atlas 도메인** — id·name·aliases·kind(continent/region/city-map)·extent(projection·units)·**references(Vec<PlaceId>)**·body_sections(다이어그램 ASCII 보존)
2. **도메인+뷰 이중성** — 자기 데이터 소유(좌표·projection) + view 메서드(`places_in`·`settlements_in`·`geographies_in`·`adjacent_to`) + ASCII 다이어그램 노출
3. **관계 도메인 패밀리 패턴 정착** — Phase 5+ Timeline·OrgChart 등 일반화의 시드. View trait 일반화는 두 번째 view 등장 시(Phase 5 Timeline)

검증 게이트: `atlas-jungwon` 1개 변환 (seven-nations.md §0.3 ASCII 다이어그램 시드) + references 11 Place 외래키 0건 + ASCII 다이어그램 byte-exact 보존 + view 메서드 e2e (places_in 11·settlements_in 8·geographies_in 3·adjacent_to 정합) + MCP 도구 2개.

핵심 결정 (사양 §6 권장값):
- extent.projection = schematic만 (좌표·SVG는 Phase N+)
- references = 11 Place 모두 (Phase 3 산출과 1:1)
- Era overlay = Phase 5 분리 (extras.era_id 텍스트만 보존)
- View trait 일반화 = Phase 5 Timeline 등장 시
- ASCII 다이어그램 byte-exact 보존 (코드블록 안에)

체크포인트 분리 게이트 강제 적용 (Phase 1 미준수 후속).

### Phase 5 — 5a Event + 5b (Era + Timeline view) 분리 ⏳

**Phase 5는 두 결로 분리** (2026-05-01 결정):

#### Phase 5a — Event Vertical Slice ✅

**완료 (2026-05-02)** — 보고서 `phase5a-checkpoint{1,2}-report.md`

목표: 두 번째 인스턴스 도메인. 270년 28사건 중 핵심 5-10건 변환 + Phase 1·2·3 외래키 매트릭스 확장 (Event.participants.{people,groups,places}).

핵심 결정:
- 첫 변환 = `event-bloody-night` (붉은 밤의 변, 10년 전, player·조고 직결)
- `EventCategory` 일급 enum (Historical·Scheduled·Legendary)
- `EventTemporal.year_relative` 캐시 컬럼 (Era 결합 전 임시 정렬)
- `era_id` 텍스트만 (5b에서 외래키 활성)
- `ParticipantsRefs` (people·groups·places) 모두 외래키 hard-fail

검증 게이트: 5-10 Event + 외래키 0건 + MCP 도구 3 (`list_events`·`get_event`·`search_events`).

#### Phase 5b — Era + Timeline + Atlas overlay ✅

**완료 (2026-05-02)** — 보고서 `phase5b-checkpoint{1,2}-report.md`

산출:
- 5 Era 변환 (founding/prosperity/turning/decline/fall, history.md §0.2 정확 매핑, 50+70+80+40+30=270 정합)
- 1 Timeline (timeline-jungwon-history) — references=Vec<EraId> 두 단계 합성
- view 메서드 4종 e2e (eras_in 5 / events_in 6 / events_during 5 / causal_chain 6 BFS)
- Atlas overlay 양방향 (atlas-jungwon ↔ era-fall-of-empire)
- migrate_v6·v7 (eras·timelines·timeline_era_refs)
- MCP 도구 6개 + REST 6개

핵심 발견:
- **두 단계 합성 패턴** — timeline=era 묶음, era=event 묶음. 미래 관계 도메인의 시드.
- **causal_chain BFS = 6** — bloody-night에서 시작한 인과 사슬이 timeline 전체 6 사건을 도달. Phase 5a related_events 양방향 시드의 자연 결과.
- **boundary 정책 정확** — start inclusive · end exclusive. 270년차(year_relative=0)는 어느 era에도 무소속(미래 era 시드).

세 결 통합:
1. **Era 인스턴스 도메인** — 5 시대(history.md §0.2: 건국기·전성기·변곡기·쇠퇴기·붕괴기). Phase 5a Event era_id 외래키 활성.
2. **Timeline 관계 도메인** — Atlas와 같은 결의 도메인+뷰. references=Vec<EraId> + view 메서드(eras_in·events_in·events_during·causal_chain).
3. **Atlas overlay** — `atlas.era_id` 외래키 활성. atlas-jungwon = era-fall-of-empire (현재 시점).

핵심 결정 (사용자 confirm):
- **Q1 = 5 era** (history.md §0.2 그대로)
- **Q2 = View trait 일반화 보류** (Atlas + Timeline 각자 view 메서드, trait 추출은 Phase 5+ 또는 별도 작업)
- **Q3 = (a) atlas.era_id 외래키** (시기별 atlas 분기는 별 atlas 인스턴스, overlay 관계 테이블은 Phase 6+)

검증 게이트:
- 체크포인트 1: 5 Era 변환 + Phase 5a 6 Event era_id 활성 + atlas-jungwon.era_id 활성
- 체크포인트 2: 1 Timeline + view 메서드 e2e + MCP 정성

알려진 boundary 케이스: bloody-cult-rebellion-2nd(-30)이 era-decline 끝 vs era-fall-of-empire 시작. 디렉터 결정 (체크포인트 1 보고서).

진입 조건: Phase 5a 종결 (✅ 완료).

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
| 2026-05-01 | HEXACO 범위 = -1.0~+1.0 확정 + Score VO 재사용 (외부 리뷰 + 사용자 코드 정보) | 외부 리뷰 + 사용자 |
| 2026-05-01 | Phase 2 종결 — 7 Person + 41 e2e + commit pause 게이트 정상 회복 | Claude Code |
| 2026-05-01 | Player Character ID = "player" 채택 (단일 플레이어 가정) | Cowork |
| 2026-05-01 | Phase 2 본문 §3.5 무결성 검증 통과 — `rebuild_repo_from_inner`이 emotion_state 명시 재적용. 신규 보존 로직 0 LOC | Claude Code (Track C) |
| 2026-05-01 | Q2·B 정책 단독 검증 통과 — player 추가가 src/ 코드 변경 0으로 흡수 | Claude Code (Track B) |
| 2026-05-01 | **도구 추상화 두 결 명시** — Phase 0~4=worldbuilding(정적·작가), Phase 5+=gameplay(동적·런타임). 두 결 잇는 다리(Scenario Builder·Relationship 자동 시드·Memory 자동 시드·Object→Item)가 Phase 5+ 핵심 결정 | 사용자 |
| 2026-05-01 | Phase 3 작전 완성 — Place 도메인 + Phase 1·2 외래키 활성(headquarters·birthplace·current_location 검증 승급) + sect 이중 등록 + Atlas 분리 (Phase 4) | Cowork |
| 2026-05-01 | npc-11 heritage-pending stub 사용자 직접 작성 — FK 결손 2건 해소, persons indexed = 9 도달 | 사용자 |
| 2026-05-01 | dialogue REST 경로 = `/api/chat/start` 명료화 (`dialogue_start`는 MCP 도구 이름) | Cowork |
| 2026-05-01 | Phase 3 종결 — 11 Place + 외래키 0건 + sect/geography_refs 양방향 자동 e2e | Claude Code |
| 2026-05-01 | city-level 단순화 일관 적용 — 낙양·독관성·검성·정암·동해 연안·자유도시 뒷골목 모두 nation/sect 단위로 통합. Phase 5+ Atlas/Scene 정밀도 복원 가능 | Cowork ↔ Claude Code |
| 2026-05-01 | Phase 4 작전 완성 — Atlas 도메인+뷰 이중성, references 11 Place, schematic projection, ASCII byte-exact 보존, View trait 일반화는 Phase 5+ | Cowork |
| 2026-05-01 | Phase 4 종결 — atlas-jungwon + references 11 + ASCII 4단계 byte-exact + view 메서드 e2e + REST/MCP 라이브 정성 통과 | Claude Code |
| 2026-05-01 | SQLite FK DDL 절 정책 = application-layer 검증 (옵션 A 채택, Phase 5+ 동일 정책) | 사용자 ↔ Claude Code |
| 2026-05-01 | **Phase 5 분리 결정** — 5a Event 인스턴스 + 5b Era + Timeline view + View trait 일반화 + Atlas overlay (Q1) | 사용자 |
| 2026-05-01 | View trait 일반화 시점 = Phase 5b (Atlas + Timeline 두 사례로 추출, Q2) | 사용자 |
| 2026-05-01 | 첫 Event = "붉은 밤의 변" (10년 전, player·조고 직결, Q4) | 사용자 |
| 2026-05-01 | gameplay 다리 (Scenario·Scene·Beat·Memory 통합) = Phase 6+ (Q5) | 사용자 |
| 2026-05-02 | Phase 5a 종결 — 6 Event + 외래키 0건 + related_events 양방향 + alias 패턴 일관 (본질 vs 결과형 분리) | Claude Code |
| 2026-05-02 | alias 결정 패턴 — 본질 가리키는 별호만 alias, 결과형 표현은 `extras.outcome`, 시간 표기는 `temporal.year`. 모든 미래 인스턴스 카테고리에 적용 권장 | Claude Code |
| 2026-05-02 | 혈교 잔당 그룹 = 영구 누락 + 산문 명시 (D1, Phase 6+ historical/legendary group 카테고리에서 자연 처리) | Cowork |
| 2026-05-02 | historical npc 시드 확장(임서운·추양진인·바투·진대인·천마 등) = Phase 5b 종결 후 follow-up TASK (D2) | Cowork |
| 2026-05-02 | Phase 5a 사이드 픽스 — `LlamaServerMonitor` → `InferenceServerMonitor` 정정 (mind-studio 빌드 차단 해제) | Claude Code |
| 2026-05-02 | Phase 5b 사전 결정 3건 — 5 era 채택 / View trait 보류 / atlas.era_id 외래키 (Q1·Q2·Q3) | 사용자 |
| 2026-05-02 | Phase 5b 작전 완성 — Era 5 + Timeline + Atlas overlay + Phase 5a Event era_id 활성. View trait 일반화는 Phase 5+ 두 사례 사용 후 결정 | Cowork |
| 2026-05-02 | Phase 5b 종결 — 5 Era + 1 Timeline + 두 단계 합성 + causal_chain BFS = timeline 전체 6 사건 도달 + Atlas overlay 양방향 | Claude Code |
| 2026-05-02 | boundary 정책 = start inclusive · end exclusive. bloody-cult-rebellion-2nd(-30) → era-fall-of-empire (붕괴기 시작 트리거) | Claude Code |
| 2026-05-02 | Atlas 모델 비변경 결정 — `extras["era_id"]` + 헬퍼 그대로. top-level 필드 승격은 Phase 6+ breaking change로 미룸 | Claude Code |
| 2026-05-02 | Phase 5+ follow-up 흐름 = (C) historical-npcs + mid-era-events 동시 → Phase 6 진입. 단 의존성으로 5c.1 → 5c.2 순서 진행 | 사용자 |
| 2026-05-02 | Phase 5c.1 작전 완성 — 임서운(체크포인트 1) + 7-11 historical npc + HEXACO 정밀(active 4)/heritage-pending(historical 3-7) | Cowork |
| 2026-05-02 | Phase 5c.1 종결 — 7건 변환 + Phase 5a Event 핵심 분기 사건(bloody-night·hwasan-fall) 외래키 0건 + npc-11 stub 승급 | Claude Code |
| 2026-05-02 | 직교 플래그 컨벤션 (heritage_doc_pending + hexaco_confidence) + extras.secret 컨벤션 정착. 사양 §3.3·§3.4b 정형화 | Claude Code |
| 2026-05-02 | npc-09-jinyarim.md ≠ npc-09 진대인 — 별개 인물 (진야림 = 영주 왕가, 진대인 = 동해 상방). 디렉터 매트릭스 사실 오류 정정 | Claude Code |
| 2026-05-02 | npc-danun = `heritage_doc_pending: false` + `hexaco_confidence: precise` 채택 (wuxia-core 본기 존재). 사양 §3.3 일률 분류 → 인물별 결정으로 갱신 | Cowork (Claude Code 권장 채택) |

## 7. 청강만리 vs 칠국춘추 (혼동 주의)

- **청강만리(青江萬里)** = 사용자 영감 받은 기존 무협지(와호장룡 관련). **프로젝트 이름 아님.**
- **칠국춘추(七國春秋)** = 사용자 직접 작성 중인 오리지널 세계관 = 프로젝트 = `projects/chilguk-chunchu/`

슬러그는 한국어 발음(`chilguk-chunchu`). 중국어 병음·영문 의미역 사용 안 함.

## 8. 입력 자료 + 산출물

### 입력 자료 (wuxia-core/docs/)

- 무협 원전: `Chinese-Literature/` (Phase 0 PD 3권 인덱싱 — 水滸傳·江湖奇俠傳·蜀山劍俠傳. 나머지는 라이선스 검증 후)
- 중국 정사: `Chinese-History/` (사기·한서, Phase 5+ 인덱싱)
- 칠국춘추 시드:
  - `world/seven-nations.md` — 칠국 v1.1 (1076줄) — Phase 1·3 입력
  - `world/history.md` — 270년 연표 — Phase 5 입력
  - `world/history-characters.md` — 역사 인물·문파 배치 — Phase 2·5 입력
  - `characters/character-roster.md` — 인물 총람 v1.1 — Phase 2 입력
  - `characters/npc-01·02·03·04·05·06·11.md` — 열전 완성 7명 (★ 우선순위)
  - `characters/npc-07·08·09·10` — 열전 미작성 (heritage-pending 잠정 매핑 또는 Phase 5+ 풍부화)
  - `characters/칠국춘추_플레이어_캐릭터_시트.md` — 플레이어 시트 (Phase 2.1 입력)

### 산출물 (projects/chilguk-chunchu/world/)

- **Phase 1 산출** — `group/*.md` × 6:
  - group-daejin-court (dynasty-court, imperial)
  - group-shipsangsi (covert-band, parent=daejin-court)
  - group-namgung (clan, orthodox)
  - group-mulim-mang (alliance, orthodox)
  - group-cheonma-shingyo (sect-religious, heterodox)
  - group-gaebang (mendicant-order, orthodox)
- **Phase 2 산출** — `person/*.md` × 9:
  - npc-01 명경 / npc-02 조고 / npc-03 남궁혁 / npc-04 당무괴 / npc-05 소연 / npc-06 야율설화 (열전 풍부)
  - npc-07 천순제 (heritage-pending 잠정)
  - npc-11 소풍자 (heritage-pending 잠정 — 사용자 직접 작성)
  - player (Q2·B sub-kind)
- **Phase 3 산출** — `place/*.md` × 11:
  - 8 settlement: place-daejin·namgung·seoryang·bukwon·namman·donghae·jiyu-doshi·namgung-sega(sect)
  - 3 geography: place-western-mountains·bukwon-grasslands·namman-jungle
  - 6 distinct kind: nation·autonomous-zone·sect·mountain-range·grassland·jungle
- **Phase 4 산출** — `atlas/atlas-jungwon.md` × 1 (kind=continent, references 좌상→우하 11, extent 7×7 schematic, era_id=era-fall-of-empire 활성)
- **Phase 5a 산출** — `event/*.md` × 6 (1 era-founding + 5 era-fall-of-empire, related_events 양방향 인과 사슬)
- **Phase 5b 산출** — `era/*.md` × 5 (founding·prosperity·turning·decline·fall, 270년 정합) + `timeline/timeline-jungwon-history.md` × 1 (Vec<EraId> 두 단계 합성)
- **빌드 산출물 (gitignore)** — `build/world.sqlite` (Phase 3 후 +α)

### 미작성 인물 (Phase 5+ 풍부화 후보)

- npc-08 바투 (북원 늑대왕 / 야율설화 부친) ★★★★
- npc-09 진대인 (동해 진씨 상방 당주) ★★★
- npc-10 3대 천마 (천마신교 교주) ★★★★★

이 셋은 character-roster ★★★★+ 우선순위지만 열전 미작성 — Phase 5+에서 character-roster + history 자료로 풍부화 + heritage-pending 마커 해제.

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
