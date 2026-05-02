# Phase 5 Follow-up: Historical NPCs (Phase 5c.1)

> **For Claude Code.** 이 문서는 자급 자족이며 외부 링크 없이도 작업 시작 가능.
> 결정 사항을 임의 변경하지 말 것. 변경이 필요하면 보고서에서 디렉터 승인 요청.
> **선행 조건**: Phase 0·1·2(+2.1·2.2)·3·4·5a·5b 모두 종결.
> **체크포인트 분리 게이트 강제 적용** — Phase 1 미준수 후속, Phase 2·3·4·5a·5b 정상 준수.

---

## 1. 목표

Phase 5b §6 Decision Log (2026-05-02 D2) 후속:

> historical npc 시드 확장(임서운·추양진인·바투·진대인·천마 등) = Phase 5b 종결 후 follow-up TASK (D2) | Cowork

**Phase 5c.1 = "historical NPCs follow-up"** — Phase 5a Event 산문에서 `(npc 미등록)`으로
우회 보존된 인물들을 정식 `Person` 등록으로 승급하고, 영향 받는 Phase 5a Event participants
외래키를 활성화한다.

세 결 통합:

1. **historical NPC 시드 확장** — `kind=historical` Person 7-11명 신규 변환. mind 시스템
   업서트 대상 아님(`is_mind_eligible()` = false), 외래키 매트릭스에만 등록.
2. **heritage-pending NPC 4명 정식 변환** — npc-08 바투·npc-09 진대인·npc-10 3대 천마·
   npc-11 소풍자(현재 stub) 정밀 매핑.
3. **Phase 5a Event 외래키 활성** — 6 Event 중 historical NPC 등장 사건의 `participants.people`
   배열에 신규 ID 추가. `(npc 미등록)` 산문 마커 제거 또는 정정.

**검증 게이트**:
- 체크포인트 1: 임서운 단독 변환 + 임서운 등장 Event 외래키 활성 (3-4건)
- 체크포인트 2: 7-11 추가 historical npc + Phase 5a 6 Event 외래키 모두 활성

## 2. 연관 컨텍스트

- `CLAUDE.md` 프로젝트 루트
- `docs/tasks/00-roadmap.md` — 전체 흐름·결정 로그
- `docs/tasks/task-phase5a-event-vertical-slice.md` + `phase5a-checkpoint{1,2}-report.md` — Phase 5a 결과
- `docs/tasks/task-phase2-person-vertical-slice.md` + `phase2-checkpoint{1,2}-report.md` — Person 도메인·HEXACO 매핑 패턴 (npc-06 정밀 / npc-07 heritage-pending 기준선)
- 입력 자료:
  - `wuxia-core/docs/characters/character-roster.md` — H1-H29 historical 인물 + N1-N11 active 인물
  - `wuxia-core/docs/world/history-characters.md` — 시기별 인물 배치 (§9·§10·§11·§13)
  - `wuxia-core/docs/characters/npc-{08,09,10}-*.md` — 열전 미작성, character-roster 보강 필요
  - 기존 Phase 5a 6 Event 산문 — `(npc 미등록)` 마커 제거·정정 대상

## 3. 제약

### 3.1 장르 중립 vs 의존

Phase 5c.1은 **새 도메인을 추가하지 않음**. 기존 `Person` 도메인(`src/domain/world/person.rs`)
+ Phase 5a Event 도메인의 인스턴스 데이터만 추가. `src/`·`genres/wuxia/` 변경 금지.

| 위치 | 책임 |
|---|---|
| `src/domain/world/person.rs` | **변경 금지** — 도메인 정의 그대로 |
| `src/worldbuilding/markdown/person.rs` | **변경 금지** |
| `src/bin/world_load.rs` | **변경 금지** — Phase 5a FK 검증 그대로 활용 |
| `projects/chilguk-chunchu/world/person/{npc-im-seoun,...}.md` | 신규 historical Person 마크다운 |
| `projects/chilguk-chunchu/world/event/event-*.md` | participants.people 배열 + 산문 정정 |

**`src/`에 wuxia 단어 X.** 작업 결과물은 `projects/chilguk-chunchu/`에만 집중.

### 3.2 historical NPC ID 네이밍 정책

- 11 active/heritage-pending Person은 `npc-{01..11}` 숫자 ID 보존 (Phase 2 결정).
- historical Person은 **`npc-{descriptive-romanized-name}`** 패턴 (예: `npc-im-seoun`,
  `npc-chuyangjinin`, `npc-jincheonmyeong`, `npc-danun`).
- 이유: 숫자 ID는 character-roster N1-N11과 1:1 매핑 의미가 있어 historical(H##) 분리 필요.
  descriptive ID가 검색·디버깅 친숙.

### 3.3 HEXACO 매핑 정밀도 — 두 등급

Phase 2의 두 등급 정책 그대로:

| 등급 | 출처 | 신뢰도 표기 |
|---|---|---|
| **정밀** | 열전 또는 character-roster + history-characters 다중 출처 + 명시적 가치관 | 신뢰도 보통+ |
| **heritage-pending** | 열전 미작성, character-roster + history 단편만 | `extras.source_status: heritage-pending` |

Phase 5c.1 매핑 (체크포인트 2 보고서에 명시):

| Person ID | 등급 | 사유 |
|---|---|---|
| `npc-im-seoun` (임서운) | **정밀** | character-roster H29 + history-characters §11.1 + player.md 다중 명시 + player 메인 비밀 4종 출처 |
| `npc-08` 바투 | 정밀 | character-roster N8 + history.md 명시 |
| `npc-09` 진대인 | 정밀 | character-roster N9 + npc-09-jinyarim 부친 |
| `npc-10` 3대 천마 | 정밀 | character-roster N10 + npc-06 사부 |
| `npc-11` 소풍자 | 정밀 (stub 승급) | 사용자 작성 stub → 정식 매핑 |
| `npc-chuyangjinin` (추양진인) | heritage-pending | history-characters §9·§11.1 단편 |
| `npc-jincheonmyeong` (진천명) | heritage-pending | history-characters §1·§13 단편 |
| `npc-danun` (단운, 태무제) | heritage-pending | history-characters §10·§11 단편 + character-roster H1 |

**선택 추가 4건** (디렉터 결정):
- `npc-pungmanri` (풍만리) — heritage-pending
- `npc-seolmuhan` (설무한) — heritage-pending
- `npc-jayangjinin` (자양진인) — heritage-pending
- `npc-cheonrian` (천리안) — heritage-pending

권장 7건 = 필수 4(npc-08·09·10·11) + 핵심 historical 3(추양진인·진천명·단운).
선택 추가 4건은 별도 follow-up 또는 Phase 6+로 이관 후보. 체크포인트 2 시작 시 디렉터 결정.

### 3.4 affiliation 정책 — pending_groups 패턴 (npc-04 미러)

화산파(group-hwasan-pa)·청성파·아미파·해남파 등은 Phase 1 미등록. 등록될 때까지:

```yaml
affiliation: []                  # 정식 외래키 없음
extras:
  pending_groups: [group-hwasan-pa]   # Phase 6+ historical/legendary group 카테고리에서 등록 예정
```

이 패턴은 npc-04 당무괴(서량/당가 그룹 미등록)에서 정착. 외래키 hard-fail 회피 + 정보 보존.

### 3.5 Phase 5a Event 외래키 활성

| 검증 | 정책 |
|---|---|
| `Event.participants.people` → `persons.id` 존재 | **에러** (Phase 5a 활성, 그대로 유지) |

신규 historical Person이 등록되면, 해당 인물이 산문에 명시된 Event의 `participants.people`
배열에 ID를 추가한다. 산문의 `(npc 미등록) 인물명` 마커는 제거하거나 ID 참조로 정정.

**boundary 케이스**: 산문에 직접 등장하지 않으나 history-characters의 시기별 배치
(§13 인물·문파 배치 요약표)를 통해 추론되는 인물의 외래키 추가 여부는 디렉터 결정.
임의 추가 금지 — 출처 보수성 유지.

### 3.6 mind eligible 변화 검증 (체크포인트 2)

- 현재(Phase 5b 종결): mind eligible = 9 (npc-01·02·03·04·05·06·07·11 + player)
- 체크포인트 2 후: **mind eligible = 13** (player + npc-01·02·03·04·05·06·07·08·09·10·11 = 12 active + player = 13)
- historical npc는 mind 등록 X (`is_mind_eligible()` = false 가드).

회귀 가드: `world-load` 후 `npc_repository.list_active_persons()` 카운트 13 검증.

### 3.7 SoT = 마크다운, 검색 = FTS5 + LIKE fallback

기존 흐름.

### 3.8 체크포인트 분리 게이트

1. **체크포인트 1**: 임서운 단독 변환 + 임서운 등장 Phase 5a Event 외래키 활성 (3-4건) →
   commit pause → `phase5-followup-historical-npcs-checkpoint1-report.md` → Cowork 리뷰
2. **체크포인트 2**: 7-11 추가 historical/heritage-pending Person + Phase 5a 6 Event 외래키
   매트릭스 모두 활성 + mind eligible 13 검증 → commit pause →
   `phase5-followup-historical-npcs-checkpoint2-report.md` → Phase 5c.1 종결

**1회 통합 commit 금지.**

## 4. Done Criteria

### 체크포인트 1 (임서운 단독)

- [ ] `projects/chilguk-chunchu/world/person/npc-im-seoun.md` 작성 (kind=historical, status=missing)
- [ ] HEXACO 6 dim 정밀 매핑 (정파·검학·player 보호자·10년 전 행방불명 패턴)
- [ ] `extras.priority: "★★★★★"` (player 메인 비밀의 핵심)
- [ ] `extras.secret: "..."` 명시 (3-4 비밀: 추양진인 수제자 오인식 / 혈매화검 진짜 주인 / 생존 가능성)
- [ ] affiliation 결정 — empty + `extras.pending_groups: [group-hwasan-pa]` (npc-04 패턴)
- [ ] 임서운 등장 Phase 5a Event 외래키 활성:
  - [ ] `event-bloody-night.md` participants.people에 `npc-im-seoun` 추가
  - [ ] `event-hwasan-fall.md` participants.people에 `npc-im-seoun` 추가
  - [ ] `event-blood-disappearance.md` participants.people에 `npc-im-seoun` 추가
  - [ ] (boundary) `event-bloody-cult-rebellion-2nd.md` — 산문에 임서운 미등장 시 외래키
        추가 보류, 보고서에 명시. 추양진인 등록 시 함께 처리(체크포인트 2 후보).
- [ ] 산문 `(npc 미등록) 임서운` 마커 제거·정정 (player 부친 → player 보호자 사실 정정 포함)
- [ ] `cargo build` + `cargo test --features embed` 통과 (회귀 가드)
- [ ] world-load FK 검증 통과 (persons indexed = 10, eligible = 9 변동 없음)

### 체크포인트 2 (7-11 추가)

- [ ] 권장 7건 또는 11건 — 디렉터 결정
- [ ] HEXACO 매핑 신뢰도 표기 (정밀 vs heritage-pending)
- [ ] Phase 5a 6 Event 외래키 매트릭스 모두 활성 (산문 (npc 미등록) 마커 0건)
- [ ] mind eligible = 13 검증 (npc-08·09·10·11 mind 등록 시점)
- [ ] `cargo build` + `cargo test --features embed` 통과
- [ ] 회귀 가드: 기존 9 active person mind upsert 영향 없음(idempotent)

## 5. 단계별 작업

### Step 1 — 체크포인트 1: 임서운 단독 변환

#### 5.1.1 `npc-im-seoun.md` frontmatter 골격

```yaml
---
id: npc-im-seoun
kind: historical
name: 임서운(林書雲)
aliases:
  - 화산파 수제자
  - 구름에 글을 쓰다
  - 그 사람 (player 화법)
status: missing
hexaco:
  honesty_humility: 0.7    # 정파 정직 + 추양진인 수제자 신분 숨김(modesty 강함)
  emotionality: 0.5         # player 보호 자기희생 + 행방불명 결심(sentimentality·anxiety 양수)
  extraversion: -0.2        # 검학 + 기록자 ("구름에 글을 쓰다") + 사문 일에 집중
  agreeableness: 0.5        # 정파 의리 + player 무조건 보호 + 추양진인 충실
  conscientiousness: 0.7    # 화산파 수제자 = 검학 정점 + 270년 전통 보존자
  openness: 0.4             # 검학 + 기록 + 비공식 player 양육 결단(unconventionality 양수)
temporal:
  birth_year: 미상 (220년대 후반 추정)
  death_year: ~                 # missing — sahcong 미확인
  age_at_game_start: ~          # 행방불명 — 추정 40~50대
  notes: |
    추양진인 수제자라는 직위를 보면 260년차 화산파 멸문 시점 30대 후반~50대 초반
    추정. character-roster H29 + history-characters §11.1 출처. 정확한 출생 연도 불명.
    10년 전 행방불명, 본 Phase 등록 시점 기준 status=missing. 메인 퀘스트 후반에서
    생존 가능성 분기 트리거.
affiliation: []                  # 화산파(group-hwasan-pa) Phase 1 미등록
birthplace: ~                     # 미상
current_location: ~               # 행방불명
summary: |
  ...
tags:
  - wuxia
  - person
  - historical
  - hwasan-disciple
  - player-protector
  - missing
extras:
  signature_skill: 화산파 검학(추양진인 직계 비전) + 기록·문서화 + 혈매화검(개인검)
  biography_short: 화산파 추양진인 수제자. player 보호자. 10년 전 멸문 직후 행방불명.
  game_role: 메인 서사 비밀 축 — player의 정체·혈매화검 출처·생사 진실 모두 본 인물에 수렴
  priority: "★★★★★"
  combat_style: 화산파 검학 + 기록자 성향. 정면 전투력보다 보호·도주 우선.
  story_role: player의 "그 사람". 사망 인식이지만 행방불명. 메인 퀘스트 후반 반전.
  pending_groups: [group-hwasan-pa]
  big_five_legacy: {}
  values:
    chung: 0.7
    eui: 0.8
    hyo: 0.6
    bok: 0.4
    yah: 0.2
  hexaco_facets: {}
  source_status: heritage-pending
  secret: |
    1. 추양진인 수제자 — player는 "말단 제자"로 오인식.
    2. 혈매화검의 진짜 주인 — 화산파 보물이 아닌 본인 개인 검.
    3. 생존 가능성 — history-characters H29 "행방불명·생사 불명".
  player_relevance: 5
---
```

#### 5.1.2 산문 섹션 — npc-06/07 패턴 미러

7개 H2 섹션:
- `## 개요`
- `## 배경`
- `## 동기`
- `## 비밀`
- `## HEXACO 분석`
- `## 관계`
- `## 게임에서의 역할`

#### 5.1.3 Phase 5a Event 외래키 활성 (3건 + 1 boundary)

3 명시 사건:
- `event-bloody-night.md` participants.people: `npc-im-seoun` 추가 (셋째 밤 player 도주)
- `event-hwasan-fall.md` participants.people: `npc-im-seoun` 추가 (추양진인 수제자, player 구출)
- `event-blood-disappearance.md` participants.people: `npc-im-seoun` 추가 (개방으로부터 player(5세) 위탁)

1 boundary:
- `event-bloody-cult-rebellion-2nd.md`: 산문에 임서운 미등장. 추양진인이 등장하지만 임서운은
  240년차 시점 화산 일반 제자였을 추정만 가능. 출처 보수성 유지 — **체크포인트 1에선 추가
  보류**. 추양진인 등록 시(체크포인트 2) 함께 결정.

산문 정정:
- `(npc 미등록) 임서운: player 부친` → `(npc-im-seoun) 임서운: player 보호자`
  (player.md 기준 player의 친부모 미상, 임서운은 5세 위탁 후 비공식 양육자).
- `(npc 미등록) 임서운` 마커 제거 또는 ID 참조로 정정.

### Step 2 — 체크포인트 2: 7-11 추가

권장 7건 = 필수 4 + 핵심 historical 3.

#### 5.2.1 필수 4 (heritage-pending 승급)

- `npc-08` 바투 (북원 늑대왕 / 야율설화 부친) — character-roster N8 + npc-06 부친 묘사
- `npc-09` 진대인 (동해 진씨 상방 당주) — character-roster N9 + npc-09-jinyarim 부친
- `npc-10` 3대 천마 (천마신교 교주) — character-roster N10 + npc-06 사부
- `npc-11` 소풍자 (개방 장로) — 기존 stub 정밀 매핑 승급

#### 5.2.2 핵심 historical 3

- `npc-chuyangjinin` 추양진인 (화산 장문인, 260년차 멸문 시 전사) — heritage-pending
- `npc-jincheonmyeong` 진천명 (태조, 270년 전 건국) — heritage-pending
- `npc-danun` 단운 (태무제, 30년 전 직접 참전) — heritage-pending

#### 5.2.3 선택 추가 4 — 디렉터 결정

- `npc-pungmanri` 풍만리 / `npc-seolmuhan` 설무한 / `npc-jayangjinin` 자양진인 / `npc-cheonrian` 천리안.

#### 5.2.4 Phase 5a 6 Event 외래키 매트릭스 활성

각 Event participants.people 배열에서 `(npc 미등록)` 마커 0건 도달.

#### 5.2.5 mind eligible 검증

- 현재: 9 → 체크포인트 2 후: **13** (player + npc-01..11 = 12 active + 1 player)
- npc-08·09·10·11 정식 변환 시 `is_mind_eligible()` = true → mind upsert 자동 등록
- historical 7-11명은 `is_mind_eligible()` = false → mind 등록 X

회귀 가드: `world-load` 후 SQLite `mind_eligible_persons` 뷰 또는 직접 카운트 13 검증.

## 6. 핵심 결정 (체크포인트 1 입력)

| 결정 | 옵션 | 권장 | 비고 |
|---|---|---|---|
| 임서운 ID | `npc-im-seoun` / `npc-h29` / `H29` | `npc-im-seoun` | descriptive Romanized, npc- prefix 일관 |
| 임서운 affiliation | empty + pending_groups / group-mulim-mang proxy | empty + pending_groups | npc-04 정착 패턴 |
| 임서운 status | alive / dead / missing | **missing** | history-characters H29 명시 "행방불명·생사 불명" |
| HEXACO 등급 | 정밀 / heritage-pending | **정밀** | character-roster + history-characters + player.md 다중 출처. extras.source_status: heritage-pending는 "열전 미작성" 표기로만 |
| Event 외래키 추가 | 3건(명시) / 4건(boundary 포함) | **3건 + 1 boundary 보고** | bloody-cult-rebellion-2nd는 산문 미등장. 출처 보수성 유지 |
| 산문 정정 | "player 부친" → "player 보호자" | **정정 적용** | player.md 기준 사실 일치 |

## 7. 알려진 한계 / Phase 6+ 후보

- **화산파 group 등록**: Phase 1 group 카테고리는 active 6 group만. 멸문된 화산파(historical)는
  Phase 6+ "historical/legendary group" 카테고리에서 처리. 임서운·추양진인의 affiliation은
  그때까지 empty + pending_groups.
- **혈교 잔당 group**: Phase 5a Decision Log D1 "영구 누락 + 산문 명시". Phase 6+ 후보 그대로.
- **historical Person mind 등록**: 현재 정책으로 대화 불가. Phase 6+에서 "기억의 인물"로
  플레이어 회상 시 등장하는 흐름 추가 검토 가능 (Memory 시스템 통합).
- **age_at_game_start**: historical NPCs는 추정값만 가능. character-roster·history-characters
  추정 텍스트 그대로 보존.

## 8. 후속

Phase 5c.1 종결 후:
- Cowork에서 `task-phase5-followup-mid-era-events.md` 작성 — 본 Phase 산출 historical npc를
  mid-era(전성기·변곡기·쇠퇴기) 사건의 participants로 활용. history.md §0.2의 미시드 14사건
  중 핵심 5-10건 변환.
- 그 후 Phase 6+ (Skill·Item·Knowledge·Lore) 진입.

## 9. 변경 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|-----------|
| v1.0 | 2026-05-02 | 초안 작성 — Phase 5b D2 follow-up 정형. 체크포인트 1·2 분리. 권장 7건 / 선택 11건. |
