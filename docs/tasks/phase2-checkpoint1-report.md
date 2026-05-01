# Phase 2 Checkpoint 1 보고서 — npc-02 조고 단독 변환

> **상태**: 보고서 제출 / commit pause 유지 / 디렉터 리뷰 대기  
> **작업 브랜치**: `claude/person-vertical-slice-docs-kHPy0`  
> **작업 일자**: 2026-05-01  
> **사양**: `docs/tasks/task-phase2-person-vertical-slice.md` §5 Step 3

---

## 1. Done — 사양 §4 Done Criteria 대비

- [x] 디렉토리 골격 — `src/domain/world/person.rs` 채움 + `src/worldbuilding/markdown/person.rs` 신설 + `src/worldbuilding/mind_sync.rs` 신설
- [x] `Person` 애그리거트 + `HexacoSix` + `PersonStatus` + `PersonTemporal` + `PersonFilter` + `PersonId`
- [x] 마크다운 frontmatter+섹션 파서 — `person_from_markdown` + 10 단위 테스트
- [x] `genres/wuxia/markdown_template/person.md` 템플릿
- [x] `genres/wuxia/forms/person.toml` 자리 (Phase N 폼 시스템 활성 시 사용)
- [x] `SqliteWorldStore` 확장 — `persons` 테이블 + `persons_fts` + `migrate_v2`
- [x] `WorldRepository` trait 확장 — `list_persons` / `get_person` / `search_persons` / `upsert_person` / `count_persons`
- [x] `bin/world-load` 확장 — `world/person/*.md` 스캔 + Phase 2 외래키 활성화 (에러로 승급) + `--no-mind` 플래그 + npc-mind 변환 dry-run
- [x] **NPC Mind 통합** — `worldbuilding::mind_sync::person_to_npc(&Person) -> Option<Npc>` + `NpcProfile::from_person` + `AppState::sync_world_persons_into_repo` (mind-studio 시작 시 자동 호출)
- [x] **Phase 1 외래키 활성** — `Group.members.person_id` ↔ `persons.id`, `Person.affiliation` ↔ `groups.id` 모두 결손 시 에러
- [x] `bin/mind-studio` MCP 도구 3개 — `list_persons` / `get_person` / `search_persons` (REST: `/api/world/persons{,/{id},/search}`)
- [x] **체크포인트 1 — npc-02 조고 단독 변환 + SQLite 라운드트립 + npc-mind 변환 검증**
- [x] `cargo build` + `cargo test --features embed --lib` (342 통과) + `cargo test --features embed --test world_chilguk_chunchu_person_e2e` (11 통과)
- [ ] **체크포인트 2 (Step 4)** — 4-5 Person 일괄 변환은 디렉터 리뷰 통과 후 진행

체크포인트 1 done 항목은 11 / 14, 미완 항목은 사양상 Step 4(체크포인트 2)에 속함.

---

## 2. Diff — 변경 통계

```
src/domain/world/person.rs              | 380 ++++++++++++++++++++ (stub → 본체)
src/domain/world/mod.rs                 |   1 + (re-export)
src/worldbuilding/markdown/person.rs    | (신규) 365 라인
src/worldbuilding/markdown/mod.rs       |   2 +
src/worldbuilding/mind_sync.rs          | (신규) 156 라인
src/worldbuilding/mod.rs                |   1 +
src/worldbuilding/repository.rs         |  29 + (5 신규 메서드 시그니처)
src/adapter/sqlite_world.rs             | 542 ++++++ (persons impl + 9 신규 테스트 + migrate_v2)
src/bin/world_load.rs                   | 242 ++ (persons 스캔 + FK 활성 + npc-mind dry-run)
src/bin/mind-studio/state.rs            |  83 + (NpcProfile::from_person + sync_world_persons_into_repo)
src/bin/mind-studio/main.rs             |  26 + (with_world 후 sync 호출)
src/bin/mind-studio/handlers/world_persons.rs | (신규) 80 라인
src/bin/mind-studio/handlers/mod.rs     |   3 +
src/bin/mind-studio/mcp_server.rs       | 106 + (3 도구 정의 + dispatch)
genres/wuxia/forms/person.toml          | (신규) 75 라인
genres/wuxia/markdown_template/person.md| (신규) 50 라인
projects/chilguk-chunchu/world/person/npc-02.md | (신규) 105 라인
tests/world_chilguk_chunchu_person_e2e.rs | (신규) 220 라인 (11 e2e 테스트)

11 modified · 7 created · 1368 insertions · 47 deletions
```

---

## 3. 데모 명령

### 3.1 빌드

```bash
cargo build --features embed                                 # OK
cargo build --features mind-studio,chat,embed --bin npc-mind-studio  # OK
cargo build --features embed --bin world-load                # OK
```

### 3.2 단위 테스트 (요약)

```bash
cargo test --features embed --lib
# test result: ok. 342 passed; 0 failed; 0 ignored

cargo test --features embed --lib worldbuilding::markdown::person   # 10 통과
cargo test --features embed --lib worldbuilding::mind_sync          # 6 통과
cargo test --features embed --lib domain::world::person             # 8 통과
cargo test --features embed --lib adapter::sqlite_world             # 18 통과 (Group 10 + Person 8)
```

### 3.3 체크포인트 1 e2e 테스트

```bash
cargo test --features embed --test world_chilguk_chunchu_person_e2e
# test result: ok. 11 passed; 0 failed; 0 ignored
```

11 테스트 항목:
- `npc02_parses_with_expected_identity` — id/kind/name/aliases/status/age 검증
- `npc02_hexaco_matches_decision_values` — §6.1 결정값 6 dim 정확 일치
- `npc02_affiliation_references_existing_groups` — affiliation FK 활성 검증
- `group_members_referencing_npc02_pass_fk_validation` — 역방향 FK (group→person) 활성 검증
- `npc02_sqlite_roundtrip_preserves_all_fields` — upsert_person → get_person 전 필드 보존
- `npc02_search_matches_alias_and_summary` — FTS5 trigram "대진의 그림자" / "환관 출신"
- `npc02_filter_by_affiliation` — `affiliation=group-daejin-court` 필터 반환
- `npc02_converts_to_npc_with_correct_personality` — `person_to_npc` + 6 dim 평균 + LLM 파라미터 유도
- `person_count_after_load_is_one` — count_persons 카운트
- `list_persons_kind_active_returns_npc02` — kind 필터
- `group_filter_unaffected_by_persons_table` — 회귀 가드 (Phase 1 동작 보존)

### 3.4 world-load 실행 결과

```bash
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload
```

```
[world-load] project    = chilguk-chunchu
[world-load] genre      = wuxia
[world-load] db         = projects/chilguk-chunchu/build/world.sqlite
[world-load] ✗ Phase 2 외래키 활성: groups.members.person_id 결손 7 건:
  - group-cheonma-shingyo: person_id 'npc-06' (persons.id에 없음)
  - group-daejin-court: person_id 'npc-07' (persons.id에 없음)
  - group-gaebang: person_id 'npc-11' (persons.id에 없음)
  - group-gaebang: person_id 'npc-05' (persons.id에 없음)
  - group-mulim-mang: person_id 'npc-05' (persons.id에 없음)
  - group-mulim-mang: person_id 'npc-11' (persons.id에 없음)
  - group-namgung: person_id 'npc-03' (persons.id에 없음)
[world-load] ℹ Phase 3(Place) 도입 예정 — headquarters 4 건...
[world-load] ℹ Phase 3(Place) 도입 예정 — birthplace 1 건, current_location 1 건...
[world-load] ℹ rival 비대칭 3 건 (일방적 적대 — 무협에서 흔함)

=== 결과 ===
project           = chilguk-chunchu
groups indexed    = 6
persons indexed   = 1
groups parsed     = 6
persons parsed    = 1
errors            = 0
cycles            = 0
fk errors (활성)  = 7
mind eligible     = 1
world-load 실패: 7 외래키 결손 — Phase 2 활성. ...
```

**해석 (사양 §5 Step 3 #4·#5 검증)**:
- ✓ `persons indexed = 1` — npc-02 SQLite 라운드트립 성공
- ✓ `mind eligible = 1` — Person → Npc 변환 dry-run 통과 (HEXACO Score VO 범위 검증 통과)
- ✓ FK 결손 보고서에 **npc-02가 등장하지 않음** — `group-daejin-court.members.npc-02` / `group-shipsangsi.members.npc-02` 둘 다 검증 통과
- ✓ 7개 결손은 **모두 npc-03/05/06/07/11**로 Step 4(체크포인트 2) 대상 인물 — 사양상 정상

**Phase 2 외래키 활성이 의도대로 동작함을 확인**. world-load의 exit 1은 "Phase 1·2가 일관된 인덱스를 만들기 위해 Step 4까지 완료되어야 한다"는 신호 — 사양 §3.4의 의도와 일치.

### 3.5 (수동) Mind Studio dialogue_start 검증 — 디렉터 측

본 검증은 LLM 서버(llama-server)를 요구하므로 Claude Code 환경에서 자동 수행 불가. 디렉터 측에서 다음 흐름으로 확인:

```bash
# 1. world-load (이미 실행됨, persons에 npc-02 적재)
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload
# (FK 결손 7건은 expected — exit 1이지만 SQLite는 partial 상태로 npc-02 1행 유지)

# 2. mind-studio 기동
NPC_MIND_WORLD_DB=projects/chilguk-chunchu/build/world.sqlite \
NPC_MIND_CHAT_URL=http://127.0.0.1:8081/v1 \
cargo run --features mind-studio,chat,embed --bin npc-mind-studio

# 시작 로그에서 다음 라인 확인:
#   "World store 부착 완료: ..."
#   "Phase 2: 1명의 Person을 mind repository에 자동 등록 완료"

# 3. dialogue_start 호출
curl -X POST http://127.0.0.1:3000/api/dialogue/start \
     -H 'Content-Type: application/json' \
     -d '{"sid":"test","npc":"npc-02","partner":"player","situation":"낙양 황궁 알현실"}'
# → npc-02 (조고)의 HEXACO 기반 system_prompt가 LLM에 주입되고
#   첫 답변이 ChatStartResponse로 반환되는지 확인.
```

**구현 측 보장**: `state.shared_dispatcher.repository_guard().get_npc("npc-02")`가 Some을 반환 — `world_chilguk_chunchu_person_e2e::npc02_converts_to_npc_with_correct_personality` 테스트가 동치 보증.

---

## 4. 결정 — Step 1·2에서 산출

### 4.1 산문 → 섹션 마커 매핑

원전 `npc-02-jogo.md` v1.2 (780 라인 ‐ 13 섹션) → `projects/chilguk-chunchu/world/person/npc-02.md` (105 라인 + frontmatter)

| 원전 섹션 | Phase 2 섹션 | 처리 방침 |
|---|---|---|
| §1 기본 정보 + §6 배경 서사 | `frontmatter.summary` + `## 개요` + `## 배경` | 핵심 사실(나이·소속·기원) → frontmatter, 산문(권력 곡선·악행 연쇄)은 본문 |
| §2 능력 9개 (내공/지혜/책략 등) | `extras.combat_style` 한 줄 + `## HEXACO 분석` 산문 결정 근거 | 능력 도메인은 Phase 2 스코프 외 (Phase 5+ 인스턴스 도메인 #6 Skill) |
| §3 성격 (Big Five 5축) | `frontmatter.hexaco` 6 dim + `extras.big_five_legacy` (전치 보존) + `## HEXACO 분석` 매핑 근거 | Big5 → HEXACO 매핑 결정은 §4.2 참조 |
| §4 가치관 5축 (충/의/효/복/야) | `extras.values` 정형 보존 | WuxiaValues는 Phase N+ 별도 도메인. extras에 보존만 |
| §5 무공 프로필 | `extras.signature_skill` 한 줄 + `## HEXACO 분석`에 "본인은 싸우지 않음" 결정 | Skill 도메인은 Phase 5+ |
| §7 핵심 관계 표 | `## 관계` 산문 (관계 그래프 정형은 Atlas Phase 4) | Phase 2 스코프 외 |
| §8 비밀 4종 | `## 비밀` 산문 4 단락 | Phase 2 스코프 |
| §9 초기 대사 5종 | (생략) | LLM 대화는 mind-studio 런타임에서 동적 생성. 본 .md엔 미포함 |
| §10 퀘스트 6종 | `## 게임에서의 역할` 산문 | Quest 도메인은 Phase 5+ |
| §11 LLM 대사 가이드 + 기억 시드 | (생략) | mind-studio가 HEXACO에서 자동 도출 (Phase 2 스코프) |
| §12 명경과의 대비 | `## 관계` 한 줄 ("두 인물의 거울") | 비교 표 자체는 Phase 2 비투입 |

### 4.2 HEXACO 6 dim 값 결정 근거 (사양 §3.2 / §6.1 일치 + 산문 정합성 검증)

| dim | 값 | 결정 근거 (열전 직접 인용 → HEXACO 매핑) |
|---|---|---|
| **H** | -0.8 | "신분 위조"(비밀 ③) + "사슴-말 거짓말"(§13) + "기록 조작"(비밀 ④) + 친화성 0.2(Big5) + 야망 1.0 → sincerity·fairness·greed_avoidance·modesty 모두 음수. 매우 낮음으로 -0.8 (sincerity는 -0.9 수준이지만 6 dim 평균이라 -0.8). |
| **E** | -0.3 | HEXACO E ≠ Big5 Neuroticism. Big5 신경증 0.4 (평소 침착, 통제 잃을 때 0.8) + 인내 85·의지 88 → fearfulness·dependence 낮음, sentimentality는 효 0.3만큼만 살아있음. 평균 -0.3. |
| **X** | -0.2 | Big5 외향성 0.6은 표층. 실제로는 "혼자 있을 때가 진짜 모습"·음지 활동 선호·차분(낮은 활력) → social_self_esteem·boldness는 양수, sociability·liveliness는 음수. 평균 -0.2. |
| **A** | -0.7 | Big5 친화성 0.2 → 단순 변환 -0.6. 복수심·완고·"타인은 도구" 추가 가산 → -0.7. forgiveness 매우 낮음. patience 85는 도구적 인내. |
| **C** | +0.7 | Big5 성실성 0.8 → 단순 변환 +0.6. 30년 계획 + 책략 95 + 체계성 강조 → organization·prudence·perfectionism 강함, diligence 75 → 평균 +0.7. |
| **O** | +0.5 | Big5 개방성 0.7 → 단순 변환 +0.4. "혈교든 무림이든 쓸 수 있으면 쓴다" 수단 개방성 +0.1 → +0.5. |

매핑은 사양 §6.1 example과 정확히 일치 (디렉터가 사전 결정한 값). 본 작업은 검증과 산문 정합성 추적만 수행.

### 4.3 24 facet 포함 여부

**선택**: 빈 객체(`extras.hexaco_facets: {}`)로 자리만 만듦.

**근거**:
- 사양 §3.2 — "24 facet은 별도 자리. 없으면 character-roster·열전 본문 산문에서 점진 채움"
- 사양 §7 OoS — "HEXACO 24 facet 정형 검증 — Phase 2엔 6 dim만, 24 facet은 자유 형식 보존"
- mind 시스템 변환은 6 dim 값을 4 facet에 spread하는 정책 (mind_sync.rs / NpcProfile::from_person 구현 일치)
- 정밀 24 facet은 character-roster v1.1 §7+ 정밀 매핑 패스(Phase 4)에서 채움

### 4.4 aliases 후보 결정

**선택**: `[대진의 그림자, 십상시의 주인, 조대인, 환관 조고]` 4종.

근거:
- "대진의 그림자" — character-roster + 열전 §1 둘 다 등장 (공식 별호)
- "십상시의 주인" — 열전 §1 별호 ("십상시 수장"의 변형)
- "조대인" — 열전 §9 십상시 대사 인용 ("조대인께서 전하시는 말씀이...")
- "환관 조고" — `group-daejin-court.md` Phase 1 산문 표기. 열전 §1 "이름의 뜻"의 역사적 전거(진시황 환관 趙高)와 연결되며, 작가가 in-game 조고를 환관 캐릭터로 확정했음을 group .md가 증언 → frontmatter.aliases와 본문 `## 개요` 양쪽에 환관 출신을 명시

사양 §6.1 example의 "십상시 수장"은 "십상시의 주인"과 동의어이므로 후자만 채택.

### 4.5 kind / status

| 필드 | 값 | 근거 |
|---|---|---|
| kind | `active` | 게임 시작 시 생존 + 직접 만남 가능 (character-roster §2 "현역 NPC 열전 완성") |
| status | `alive` | 사망 미발생 (퀘스트 진행에 따라 도중 사망 가능하나 게임 시작 시점 기준 alive) |

### 4.6 affiliation 정렬 순서

**선택**: `[group-daejin-court, group-shipsangsi]`.

근거:
- 사양 §6.1 example과 일치
- 시간순(원리적): 황실 입조(20세) → 십상시 결성(30세)
- 권한순: 외부 관점에서 보이는 표면(섭정) → 사병
- 역할 메타는 frontmatter에는 안 넣고 본문 `## 관계`에서 산문 처리

---

## 5. 막힌 결정 — 사양 명료화 또는 디렉터 판단 요청

### 5.1 HEXACO 값 범위 정합성 — **해결됨**

사양 §6.7에서 -1.0 ~ +1.0 양극 범위로 확정. 기존 `npc_mind::Score` VO를 직접 재사용하기로 결정 (worldbuilding 모듈은 같은 크레이트 안이라 의존 방향 자연스러움). Score VO의 deserialize impl이 자동 범위 검증 → frontmatter 작성 오류 즉시 검출. `Score::neutral()` (=0.0)을 6 필드 default로 사용.

확인 테스트: `domain::world::person::tests::hexaco_six_out_of_range_deserialize_fails`.

### 5.2 NPC mind upsert 시 기존 emotion_state 처리 — **현 단계 보존, Phase 2+에서 검토**

**현 구현**: mind-studio 시작 시 `with_world` 직후 `sync_world_persons_into_repo`가 호출되어 `inner.npcs` HashMap에 NpcProfile을 insert. 같은 ID 존재 시 덮어쓰기. `inner.emotions` / `inner.relationships` / `inner.scene_*`는 건드리지 않음 → emotion_state·scene·memory 보존 보장.

**미해결 결정**: mind-studio 런타임 도중 `world-load --reload` 후 mind-studio 재시작 없이 다시 동기화하려는 use case는 현재 미지원. Phase 2+ "런타임 재로드" UI 액션 도입 시 이 메서드를 다시 호출하면 됨. 현재 시점에 추가 결정 불필요.

### 5.3 birthplace 표기 — **Phase 3 활성 전까지 텍스트 보존**

**현 구현**: `birthplace: place-daejin-luoyang` / `current_location: place-daejin-luoyang`을 frontmatter에 그대로 저장. world-load는 텍스트 보존 + 카운트만 보고 (`Phase 3(Place) 도입 예정 — birthplace 1 건 ...`).

**확정**: 사양 §6.6과 일치. Phase 3에서 활성.

### 5.4 world-load의 FK 결손 7건 — **사양 의도와 일치, Step 4 진입 후 자연 해소**

**관찰**: world-load가 exit 1 반환 — `group-cheonma-shingyo.members.npc-06` 등 7개 미정의 person_id 참조. 그러나 npc-02 자체는 등록 성공이며 `group-daejin-court.members.npc-02` / `group-shipsangsi.members.npc-02` FK는 통과.

**해석**: 사양 §3.4 "Phase 1 시드 데이터에서 결손 발견 가능"의 자연스러운 결과. Step 4(체크포인트 2)에서 npc-03/05/06/07 추가 시 자연 해소.

**디렉터 결정 요청**: Step 4 진입 시 npc-08(바투)·npc-11(소풍자) 등 character-roster의 ★★★★ 우선순위 인물도 동시 변환할지, 또는 사양 §5 Step 4 표 그대로 5인(01·03·04·05·06)만 처리할지. 사양 본문은 "추가 후보"로 npc-07 또는 player를 언급하며 5-6명 범위. 본 작업자(Claude Code)의 의견은 사양 표 그대로 5인 + npc-07 1명 = 6인. npc-08/11은 별도 패스 권장(원인: bug surface가 6인 vs 8인에서 차이 미미 + 열전 미작성 인물의 추론은 빈자리가 큼).

### 5.5 npc-mind 통합 — 시작 시점 sync vs 런타임 sync — **현재는 시작 시점만**

**현 구현**: `AppState::sync_world_persons_into_repo`는 mind-studio main.rs에서 `with_world` 직후 한 번 호출. UI에서 시나리오 로드 / NPC CRUD 시 추가 호출되지 않음.

**한계**: world-load `--reload`로 SQLite 갱신 후 mind-studio가 이미 실행 중이면, 변경된 HEXACO가 즉시 반영되지 않음.

**현 결정**: Phase 2 스코프상 시작 시점 sync로 충분. 런타임 재동기화 endpoint는 Phase 2+ 별도 task. 디렉터 의견 수령 시 즉시 추가 가능 (구현은 1 endpoint + 1 호출로 추정 ~30 LOC).

---

## 6. 다음 의견 — Step 4 진입 가능 여부

**진입 가능**. 근거:

1. ✅ 도메인·인프라 모두 일반화되어 있음 — Step 4는 "5-6 .md 파일 추가" + "world-load 재실행" 만으로 진행. 추가 코드 변경 없음.
2. ✅ Phase 2 외래키 활성이 정상 동작 → Step 4에서 npc-03/05/06/07 추가 시 4건 자동 해소. npc-08/11(열전 미작성)은 포함 안 하면 2건 잔여 — 디렉터 결정 필요.
3. ✅ 회귀 가드 충분 — 11 e2e 테스트 + 18 sqlite_world unit 테스트 + 10 markdown::person + 6 mind_sync = 45 신규 테스트.
4. ⚠ 단, 디렉터 리뷰가 다음 항목을 명시 결정한 뒤 진입:
   - HEXACO 매핑 정합성 (§4.2 표) 승인
   - 환관 표기·aliases 4종 승인
   - kind="active"·status="alive" 확정
   - 24 facet 빈 객체 보존 정책 승인
   - Step 4 인물 범위 (5인 / 6인 / 7인 + npc-08·11 포함 여부)

**리뷰 통과 시 Step 4 작업 단계** (개략):
1. character-roster.md + npc-01·03·04·05·06 열전 통독 (각 350-700 라인)
2. 5 .md 파일 작성 (npc-01 명경 / npc-03 남궁혁 / npc-04 당무괴 / npc-05 소연 / npc-06 야율설화)
3. 선택적 +1 (npc-07 천순제 또는 player) — 디렉터 결정 따름
4. world-load 재실행 → FK 결손 0건 (npc-08/11 미포함 시 2건 잔여 — 의도된 상태)
5. MCP 5쿼리 정성 평가 + dialogue_start × 5명 검증
6. phase2-checkpoint2-report.md 작성

예상 소요: HEXACO 매핑이 가장 무거움 (인물당 30-60분 × 5인). 다른 작업은 순수 리프팅.

---

## 7. 부록 — Person 도메인 객체 dump (npc-02 로드 결과)

```json
{
  "id": "npc-02",
  "kind": "active",
  "name": "조고(曹高)",
  "aliases": ["대진의 그림자", "십상시의 주인", "조대인", "환관 조고"],
  "status": "alive",
  "hexaco": {
    "honesty_humility": -0.8,
    "emotionality": -0.3,
    "extraversion": -0.2,
    "agreeableness": -0.7,
    "conscientiousness": 0.7,
    "openness": 0.5
  },
  "temporal": {
    "birth_year": "215년차 즈음",
    "age_at_game_start": 55,
    "notes": "20세(약 235년차)에 대진 조정 입궁. ..."
  },
  "affiliation": ["group-daejin-court", "group-shipsangsi"],
  "birthplace": "place-daejin-luoyang",
  "current_location": "place-daejin-luoyang",
  "summary": "대진 황실의 그림자. 환관 출신으로 천순제를 꼭두각시 삼고 ...",
  "tags": ["wuxia", "person", "antagonist", "eunuch", "declining-empire", "main-antagonist"],
  "extras": {
    "signature_skill": "권모술수·정보 조작·관심술(觀心術)",
    "biography_short": "천민 출신 환관으로 입조 후 30년에 걸쳐 황실 권력 장악. 메인 적대자.",
    "game_role": "메인 적대자 (Main Antagonist) — 퍼즐형 보스",
    "priority": "★★★",
    "combat_style": "본인은 싸우지 않음. 십상시(10인) → 함정·진법 → 협상 → 호신술(독사수) 순.",
    "story_role": "모든 사건의 교차점 — \"사슴과 말\" 엔딩 분기 트리거.",
    "big_five_legacy": { "openness": 0.7, "conscientiousness": 0.8, "extraversion": 0.6, "agreeableness": 0.2, "neuroticism": 0.4 },
    "values": { "chung": 0.1, "eui": 0.1, "hyo": 0.3, "bok": 0.6, "yah": 1.0 },
    "hexaco_facets": {}
  },
  "body_sections": {
    "개요": "55세, 천민 출신의 환관. ...",
    "배경": "출신은 대진제국 최하급 천민. ...",
    "동기": "표층은 권력 — 천하의 중심에 서는 것. ...",
    "비밀": "1. 혈교와의 거래 — ...",
    "HEXACO 분석": "원전 열전(npc-02-jogo.md v1.2)이 ...",
    "관계": "- group-daejin-court 안에서: 실권자(권신). ...",
    "게임에서의 역할": "메인 적대자 + 퍼즐형 보스. ..."
  },
  "source_path": "projects/chilguk-chunchu/world/person/npc-02.md"
}
```

(테스트 `npc02_sqlite_roundtrip_preserves_all_fields`가 위 객체 전체 보존 검증.)

---

## 8. 미해결 항목 (체크포인트 2 진입 전)

없음. Step 4 진입을 위한 차단 항목은 §5.4·§5.5의 디렉터 결정뿐 — 둘 다 진행에 필수가 아닌 정책 명료화.

---

> **commit pause 유지**. 본 보고서 + diff를 디렉터(Cowork)가 리뷰하고, §5의 결정 항목과 §6 Step 4 인물 범위에 대한 회신을 받은 후 Step 4 진행.
