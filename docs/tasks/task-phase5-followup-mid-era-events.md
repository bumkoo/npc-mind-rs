# Phase 5 Follow-up — Mid-era Events 시드 확장 (D2 처리, 5c.2)

> **For Claude Code.** Phase 5a Q&A의 D2 결정의 두 번째 후속. 짧은 follow-up TASK.
> **선행 조건**: Phase 5c.1 (Historical NPCs follow-up) 종결 — npc-danun(태무제)·
> npc-jincheonmyeong(진천명)·npc-chuyangjinin(추양진인)·npc-im-seoun(임서운)·
> npc-08·09·10·11(stub→정식) 모두 등록됨.
> **체크포인트 분리 게이트 강제 적용** — 1회 통합 commit 금지.

---

## 1. 목표

Phase 5b body의 era별 `key_events: []` 빈 슬롯을 **mid-era 6 event**로 채우고, 각 era의
산문 §핵심 인물에 텍스트로만 보존된 "(npc 미등록)"이 5c.1 종결로 등록된 historical npc로
**외래키 활성** 가능해진 부분을 정합한다. Phase 5a 6 event(border-era)는 그대로 두고,
**전성기·변곡기·쇠퇴기**의 빈 시간 구획을 메운다.

**스코프**:
- Phase 5b 5 era 중 mid-era 3 era(prosperity·turning·decline) `key_events` 채움
- 신규 6 event 작성 — era 분포: prosperity 1 / turning 3 / decline 1 / founding 1
- Phase 5c.1 산출 npc 외래키 활성 (event participants에 npc-danun·npc-jincheonmyeong·
  npc-chuyangjinin 등 합류 — 가능한 인물만)

**검증 게이트**:
- 체크포인트 1: **태무제 즉위** 단독 변환 (event-taemuje-enthronement, -33, era-decline)
  — 5c.1 산출 npc-danun 직접 활용 + era-decline `key_events` 첫 항목
- 체크포인트 2: 나머지 5 mid-era event + era 본문 외래키 활성

## 2. 연관 컨텍스트

- `docs/tasks/00-roadmap.md` — D2 결정 로그 (2026-05-02) + Phase 5c.1 진입·종결
- `docs/tasks/task-phase5a-event-vertical-slice.md` + Phase 5a 보고서들 — Event 도메인·
  년도/기간/era_id/related_events 패턴
- `docs/tasks/task-phase5b-era-timeline-vertical-slice.md` + 보고서들 — Era 도메인·
  boundary 정책(start inclusive · end exclusive)·`key_events` 슬롯
- `docs/tasks/task-phase5-followup-historical-npcs.md` v1.2 + 보고서 — historical npc
  분류 정책 정착·외래키 활성 결과
- 메모리: Event.kind catalog (war/disaster/founding/betrayal 등 Phase 5a 정형) ·
  Era.kind catalog (founding/prosperity/turning/decline/fall-of-empire)
- 입력 자료:
  - `wuxia-core/docs/world/history.md` — 30년차·100년차·130년차·160년차·190년차·237년차
    명시 사건 (★ 핵심 입력)
  - `wuxia-core/docs/world/history-characters.md` v1.2 — 각 사건의 인물 매핑
  - 각 era .md 본문의 §핵심 트리거·§핵심 인물 — "(npc 미등록)" 텍스트가 정식 외래키 후보
  - Phase 5a 6 event — era_id·related_events 패턴

## 3. 제약

### 3.1 도메인 변경 X — 데이터 추가만

Phase 5a Event 도메인 그대로. 코드 변경 0. **데이터 + 테스트만 추가**:
- `projects/chilguk-chunchu/world/event/event-{slug}.md` 6개 추가
- `projects/chilguk-chunchu/world/era/era-{prosperity,turning,decline,founding}.md`
  `key_events` 슬롯 갱신 (event_id 추가)
- `tests/world_chilguk_chunchu_followup_mid_era_events.rs` 신규
- world-load 결과: events indexed 6 → 12

### 3.2 6 mid-era event 후보 (디렉터 결정 받기)

| event_id | 한글명 | year_relative | era_id | kind | 출처 |
|---|---|---|---|---|---|
| `event-taemuje-enthronement` | 태무제 즉위 | **-33** | era-decline | founding | history.md §1.4·era-decline §개요 (237년차) |
| `event-byeongkwon-recall` | 30년차 병권 회수 시도 | **-240** | era-founding | reform-fail | history.md §1.1·era-founding §결과 (30년차) |
| `event-mulim-conference-1st` | 1차 무림대회 | **-170** | era-prosperity | convention | history.md §1.2·era-prosperity §핵심 트리거 (100년차) |
| `event-sapa-formation` | 사파 3파벌 형성 | **-140** | era-turning | schism | history.md §1.3·era-turning §핵심 트리거 (130년차) |
| `event-jachi-movement` | 변경 자치 운동 | **-110** | era-turning | political-movement | history.md §1.3·era-turning §핵심 트리거 (160년차) |
| `event-cult-remnant-discovery` | 화산파 혈교 잔당 발견 | **-80** | era-turning | discovery | history.md §1.3·era-turning §핵심 트리거 (190년차) |

**era 분포 검증** (boundary 정책 — start inclusive · end exclusive):
- era-founding [-270, -220): empire-founding(-270) **+ byeongkwon-recall(-240)** = 2건
- era-prosperity [-220, -150): mulim-conference-1st(-170) = 1건 (이전 0건)
- era-turning [-150, -70): sapa-formation(-140)·jachi-movement(-110)·
  cult-remnant-discovery(-80) = 3건 (이전 0건)
- era-decline [-70, -30): taemuje-enthronement(-33) = 1건 (이전 0건)
- era-fall-of-empire [-30, 0): bloody-cult-rebellion-2nd(-30, boundary)·
  bloody-night·hwasan-fall·blood-disappearance·six-states-independence = 5건

**3 mid-era 모두 ≥1 event 확보** — Phase 5b의 "현재 80년+(turning)·40년(decline)·
70년(prosperity)이 비어있어 timeline 한가운데가 공백" 문제 해소.

### 3.3 Event.kind 신규 vs 재사용

| event | kind | 이유 |
|---|---|---|
| taemuje-enthronement | `founding` | 권력 등극 사건 — empire-founding과 같은 카테고리 |
| byeongkwon-recall | `reform-fail` (신규) 또는 `political` | 30년차 황실의 무림 병권 회수 시도가 **실패**해 무림 독립 관행 확립의 분기점. Phase 5a kind 카탈로그에 없음 — **`reform-fail` 신규 권장** |
| mulim-conference-1st | `convention` (신규) 또는 `political` | 무림대회는 정파 합의의 정형 — `convention` 신규 권장 |
| sapa-formation | `schism` (신규) | 정사 분리의 시작 — `schism` 신규 권장 |
| jachi-movement | `political-movement` (신규) | 정치적 흐름 — Phase 5a `political` 보다 동적 의미 분리 |
| cult-remnant-discovery | `discovery` (신규) | 80년 후 혈교 부활의 인과 시작점 — `discovery` 신규 권장 |

**kind 정책**: Phase 5a처럼 신규 kind는 자유롭게 추가 (도메인 enum이 아니라 string).
체크포인트 보고서에 신규 kind 일람 명시 → 디렉터 검토.

### 3.4 외래키 활성 후보

**event participants.people**:
- taemuje-enthronement → npc-danun(태무제, 5c.1 산출)
- byeongkwon-recall → npc-jincheonmyeong(대진 태조, 5c.1 산출 — 30년차에 노년)
- mulim-conference-1st → (선택) 혜통대사·적하검 등 — 5c.1 미등록 시 텍스트만
- sapa-formation → (선택) 초대 천마(설무한 또는 별도) — 5c.1 미등록 시 텍스트만
- jachi-movement → (선택) 진해(진대인 고조부)·아골타·곽천풍 — 5c.1 미등록 시 텍스트만
- cult-remnant-discovery → npc-chuyangjinin(추양진인) 본인 미등장 시점이나 화산파
  세대(자양진인 후예) — 텍스트만

**era body**: 각 era의 §핵심 인물에 텍스트만 있던 인물이 5c.1으로 정식 등록됐다면
event participants 또는 era 본문에 정식 ID로 갱신.

**조건부 활성**: 5c.1 종결 후 등록 NPC 셋만 외래키 활성. 그 외는 텍스트 보존 (Phase 5a
D1 정책 그대로 — "미등록 인물은 텍스트만").

### 3.5 related_events 인과 사슬

**6 event 간 인과**:
- byeongkwon-recall(-240) → mulim-conference-1st(-170): "병권 회수 실패"가
  "100년차 무림 자율 운영 정형(무림맹주 추대)"의 70년 누적 결과
- mulim-conference-1st(-170) → sapa-formation(-140): "정도 기준 정형화"가
  "30년 후 파문→사파 결성"의 직접 트리거
- jachi-movement(-110) → six-states-independence(-7, Phase 5a): 110년 전 시작된
  자치 운동이 7년 전 칠국 독립의 직접 인과
- cult-remnant-discovery(-80) → bloody-cult-rebellion-2nd(-30, Phase 5a): 80년 전
  발견이 50년 후 재침공의 인과 시작
- taemuje-enthronement(-33) → bloody-cult-rebellion-2nd(-30): 즉위 3년 후 격퇴
- taemuje-enthronement(-33) → blood-disappearance(-12): 21년 후 조고의 권력 장악
  사전 작업
- taemuje-enthronement(-33) → bloody-night(-10): 23년 후 황실 멸망의 직접 트리거

각 event의 `related_events`에 양방향 명시 (Phase 5a 패턴). **Phase 5a 6 event도
역방향 갱신 필요** — 예: bloody-cult-rebellion-2nd.related_events에
event-cult-remnant-discovery·event-taemuje-enthronement 추가.

### 3.6 era body §핵심 트리거 → key_events 정합

각 era .md의 `key_events: []`를 신규 event_id로 채움:
- era-founding.key_events: `[event-empire-founding, event-byeongkwon-recall]`
- era-prosperity.key_events: `[event-mulim-conference-1st]`
- era-turning.key_events: `[event-sapa-formation, event-jachi-movement,
  event-cult-remnant-discovery]`
- era-decline.key_events: `[event-taemuje-enthronement]`
- era-fall-of-empire.key_events: 그대로 (Phase 5b 시점 5건 모두 채워짐)

**era body §핵심 트리거 산문도 정합** — "30년차 병권 회수"·"100년차 무림대회"
같은 텍스트가 정식 event ID로 referenced 가능해짐. 산문 정형은 Phase 5a 정합 패턴
(텍스트 + 외래키 동시 명시).

### 3.7 체크포인트 분리 게이트

1. **체크포인트 1**: event-taemuje-enthronement 단독 변환 + npc-danun 외래키 활성 +
   era-decline.key_events 갱신 → commit pause
2. **체크포인트 2**: 나머지 5 mid-era event + era body §핵심 트리거 정합 +
   Phase 5a 6 event related_events 역방향 갱신 → commit pause → Phase 5 follow-up 종결

## 4. Done Criteria

- [ ] event-taemuje-enthronement 단독 변환 (체크포인트 1) + npc-danun 외래키 활성 +
      era-decline.key_events 갱신
- [ ] 5 mid-era event 추가 (체크포인트 2): byeongkwon-recall · mulim-conference-1st ·
      sapa-formation · jachi-movement · cult-remnant-discovery
- [ ] era 4종 (founding·prosperity·turning·decline) `key_events` 슬롯 채움
- [ ] Phase 5a 6 event `related_events` 역방향 갱신 (bloody-cult-rebellion-2nd ·
      blood-disappearance · bloody-night)
- [ ] 5c.1 산출 npc 외래키 활성 (가능한 인물만)
- [ ] world-load: events indexed 6 → 12, fk errors=0
- [ ] e2e 테스트 — mid-era event 시드 라운드트립 + era key_events 정합 + related_events
      역참조 검증
- [ ] `cargo build` + `cargo test --features embed` + 기존 e2e 회귀 통과

## 5. 단계별 작업

### Step 1 — event-taemuje-enthronement 단독 변환 ★체크포인트 1★

대상: 태무제 즉위 (event-taemuje-enthronement, -33). Phase 5c.1 npc-danun 활용.

작업:
1. `wuxia-core/docs/world/history.md` §1.4 "237년차 태무제 즉위" 통독
2. `wuxia-core/docs/characters/npc-11-taemuje.md` 본기 — 단운의 즉위 시점 시각·동기 통독
3. `projects/chilguk-chunchu/world/event/event-taemuje-enthronement.md` 작성:
   - id: event-taemuje-enthronement
   - kind: founding (또는 enthronement 신규)
   - aliases: ["태무제 즉위", "237년차 즉위"]
   - temporal.year_relative: -33
   - era_id: era-decline
   - participants.people: [npc-danun, npc-02 (조고 — 즉위 정치 배후 의혹)]
   - participants.groups: [group-daejin-court, group-shipsangsi]
   - related_events: [event-bloody-cult-rebellion-2nd (3년 후), event-blood-disappearance
     (21년 후), event-bloody-night (23년 후)]
   - extras.player_relevance: 3 (조고 추적 단서로 직결)
   - extras.director_decisions: kind 결정·5c.1 npc-danun 직접 활용 명기
4. era-decline.md `key_events: []` → `[event-taemuje-enthronement]`
5. era-decline.md 산문 §핵심 트리거 "237년차 태무제 즉위" 텍스트는 보존, 산문 §핵심
   인물 "(npc 미등록) 단운(태무제)" → "npc-danun 단운(태무제)"로 정식 ID 정합
6. Phase 5a event 3건 related_events 역방향 갱신:
   - event-bloody-cult-rebellion-2nd.related_events에 event-taemuje-enthronement 추가
   - event-blood-disappearance.related_events에 event-taemuje-enthronement 추가
   - event-bloody-night.related_events에 event-taemuje-enthronement 추가
7. world-load 통과 — events=7, fk errors=0

**체크포인트 1 보고서** (`docs/tasks/phase5-followup-mid-era-events-checkpoint1-report.md`):
- event-taemuje-enthronement.md 전문
- kind 결정 근거 (founding 재사용 vs enthronement 신규 — 디렉터 결정 권장)
- 5c.1 npc-danun 외래키 활성 결과
- era-decline.key_events 갱신 결과
- Phase 5a 3 event related_events 역방향 갱신 결과
- world-load 통과 (events=7, fk=0)
- 막힌 결정 (예: 조고가 즉위 시점 십상시 핵심? 또는 즉위 후 합류? — history-characters
  §10.1 참조)

→ Cowork 리뷰 → **commit pause 유지** → 통과 신호 받고 Step 2.

### Step 2 — 5 mid-era event 추가 ★체크포인트 2★

대상 (디렉터 결정 받기, 6 후보 중 5):

**필수 5건** (era 분포 보장 — turning에 3건):
- event-byeongkwon-recall (-240, era-founding) — 30년차 병권 회수 실패
- event-mulim-conference-1st (-170, era-prosperity) — 100년차 무림맹주 정형
- event-sapa-formation (-140, era-turning) — 130년차 사파 3파벌 형성
- event-jachi-movement (-110, era-turning) — 160년차 자치 운동 시작
- event-cult-remnant-discovery (-80, era-turning) — 190년차 화산파 혈교 잔당 발견

작업:
1. `wuxia-core/docs/world/history.md` §1.1·1.2·1.3 통독 — 30년차·100년차·130년차·
   160년차·190년차 사건 일람
2. `wuxia-core/docs/world/history-characters.md` v1.2 — 각 사건 인물 매핑
3. 5 event .md 작성 (각 §3.2 표·§3.3 kind·§3.5 related_events 정합)
4. era 본문 `key_events` 갱신:
   - era-founding.key_events: [event-empire-founding, event-byeongkwon-recall]
   - era-prosperity.key_events: [event-mulim-conference-1st]
   - era-turning.key_events: [event-sapa-formation, event-jachi-movement,
     event-cult-remnant-discovery]
5. era 본문 §핵심 인물 "(npc 미등록)" 텍스트 정합 — 5c.1 산출 npc 또는
   기존 등록 npc(npc-jincheonmyeong·npc-chuyangjinin 등)로 정식 ID 갱신 가능 부분만
6. Phase 5a 6 event related_events 역방향 갱신:
   - bloody-cult-rebellion-2nd.related_events에 event-cult-remnant-discovery·
     event-taemuje-enthronement 추가
   - six-states-independence.related_events에 event-jachi-movement 추가
   - empire-founding.related_events에 event-byeongkwon-recall 추가
7. world-load 통과 — events=12, fk errors=0
8. 신규 e2e 테스트:
   - 6 event 시드 라운드트립 (id·kind·temporal·era_id·related_events 검증)
   - era 4종 key_events 슬롯 채움 검증
   - related_events 양방향 정합 (forward + reverse 모두 검증)
   - 외래키 활성 검증 (npc-danun·npc-jincheonmyeong 등 등록 npc 참조 0 결손)

**체크포인트 2 보고서** (`docs/tasks/phase5-followup-mid-era-events-checkpoint2-report.md`):
- 5 event 일람 (id·kind·year_relative·era_id·participants 요약)
- era 4종 key_events 갱신 결과
- Phase 5a 6 event related_events 역방향 갱신 일람
- world-load: events indexed 6 → 12
- 외래키 결손 0건 (5c.1 산출 npc + 기존 등록 npc만 활성, 미등록 텍스트는 보존)
- 신규 kind 일람 (reform-fail·convention·schism·political-movement·discovery)
- search 정성 평가 — "태무제"·"무림대회"·"사파"·"자치"·"혈교 잔당"
- timeline view 검증 (atlas-jungwon overlay 정합 — Phase 5b 산출에 mid-era event 추가
  반영 검증)
- Phase 6+ 진입 가능 여부 (Skill·Item·Knowledge·Lore)

→ Cowork 리뷰 → 통과 시 Phase 5 mid-era-events follow-up 종결 → Phase 5 시리즈 전체 종결.

## 6. 결정 사항 (변경 시 디렉터 승인)

### 6.1 ID 명명 규칙

- year-marker 사건: `event-{topic}-enthronement` 또는 `event-{topic}-discovery` 같은
  의미 슬러그
- 정치 흐름: `event-{topic}-movement` 또는 `event-{topic}-formation`
- 회의: `event-{topic}-conference-{ordinal}` (1차 → `-1st`)
- 한국어 슬러그가 너무 모호한 경우 영문 의미어 우선 (`byeongkwon-recall`은 한국어
  한자 음 + 영문 의미어 혼용 — 디렉터 검토)

### 6.2 Event.kind 신규 vs 재사용 (§3.3 정합)

| event | kind 권장 | 신규 여부 | 이유 |
|---|---|---|---|
| taemuje-enthronement | founding | 재사용 | empire-founding과 같은 권력 등극 |
| byeongkwon-recall | reform-fail | 신규 | "실패한 개혁"이 본질 — 분기점 |
| mulim-conference-1st | convention | 신규 | 무림 합의의 정형 |
| sapa-formation | schism | 신규 | 정사 분리의 시작 |
| jachi-movement | political-movement | 신규 | 정치적 흐름 (장기) |
| cult-remnant-discovery | discovery | 신규 | 인과 시작점 |

**대안**: kind를 4-5종으로 압축 (예: `political`·`military`·`cultural`·`disaster`).
디렉터 결정.

### 6.3 boundary 케이스 — -240 / -80

- event-byeongkwon-recall: year_relative=-240. era-founding [-270, -220) ∋ -240 ✓
- event-cult-remnant-discovery: year_relative=-80. era-turning [-150, -70) ∋ -80 ✓

둘 다 명확히 안쪽이라 boundary 정책 검토 불필요. 단 -70(era-turning end exclusive,
era-decline start inclusive) 같은 경계 사건은 본 follow-up 스코프 외.

### 6.4 participants.people 활성 정책

5c.1 산출 npc 또는 기존 등록 npc만 정식 ID로 활성. 미등록 인물(혜통대사·적하검·진해·
아골타·곽천풍 등)은 **텍스트 보존** (Phase 5a D1 정책 그대로). 본 follow-up 스코프
외에서 추가 historical npc 등록 follow-up은 Phase 6+ 또는 별도 task.

### 6.5 related_events 양방향 정합

신규 6 event 도입으로 Phase 5a 6 event도 역방향 갱신 필요. **양방향 정합**을 e2e
테스트로 검증 (forward + reverse 모두 통과). 산문 §발단·§결과 텍스트 보존하되
frontmatter `related_events` 정형 갱신.

### 6.6 era body §핵심 인물 정합

5c.1 종결로 정식 등록된 npc는 era 본문 §핵심 인물의 "(npc 미등록)" 텍스트를 정식 ID로
갱신:
- era-founding §핵심 인물 "(npc 미등록) 태조 진천명" → "npc-jincheonmyeong 진천명"
- era-decline §핵심 인물 "(npc 미등록) 단운(태무제)" → "npc-danun 단운(태무제)"
- era-decline §핵심 인물 "(npc 미등록) 추양진인" → "npc-chuyangjinin 추양진인"

미등록 인물은 텍스트 보존 (예: "(npc 미등록) 풍만리"·"설무한"·"2대 천마" 등 — Phase 6+).

### 6.7 SQLite·MCP·기타 — 변경 없음

Phase 5a Event 패턴 그대로. 코드 변경 X.

## 7. Out of Scope

- Skill·Item·Knowledge·Lore 도메인 (Phase 6+)
- 추가 historical npc 등록 (혜통대사·적하검·진해·아골타·곽천풍·풍만리·설무한 등) —
  Phase 6+ 또는 별도 follow-up
- atlas-jungwon era overlay 갱신 (Phase 5b 산출이 mid-era event 추가에 자동 반영되는지
  검증만 — 갱신 자체는 본 follow-up 스코프 외)
- player 메인 퀘스트 시드 (예: 태무제 행방 추적 quest) — Phase 6+ gameplay 다리
- HEXACO 24 facet 정형 — Phase 6+ 영구 보류
- npc-mind 시스템에서 historical event 활용 (예: NPC 대화에서 "100년 전 무림대회"
  참조) — Phase 6+

## 8. 코드 위치 가이드

| 위치 | 무엇을 볼지 |
|---|---|
| `projects/chilguk-chunchu/world/event/event-empire-founding.md` (Phase 5a) | founding kind 패턴 + era_id 정합 |
| `projects/chilguk-chunchu/world/event/event-bloody-cult-rebellion-2nd.md` (Phase 5a) | boundary 케이스 + related_events 다중 (3건+) |
| `projects/chilguk-chunchu/world/event/event-blood-disappearance.md` (Phase 5a + 5c.1 chore) | participants.people 다중 (npc-02·player·임서운·추양진인·소풍자) + extras.director_decisions 진화 패턴 |
| `projects/chilguk-chunchu/world/era/era-fall-of-empire.md` (Phase 5b) | key_events 5건 채워진 정합 패턴 |
| `projects/chilguk-chunchu/world/era/era-decline.md` (Phase 5b) | key_events 비어있는 상태 — 본 follow-up이 채움 |
| `projects/chilguk-chunchu/world/person/npc-danun.md` (Phase 5c.1) | 본 follow-up 체크포인트 1의 외래키 대상 |
| `tests/world_chilguk_chunchu_phase5a_event.rs` (Phase 5a) | event 시드 라운드트립 패턴 |
| `tests/world_chilguk_chunchu_phase5b_era_timeline.rs` (Phase 5b) | era key_events 검증 패턴 |
| `wuxia-core/docs/world/history.md` §1.1·1.2·1.3·1.4 | ★ 핵심 입력. 30년차·100년차·130년차·160년차·190년차·237년차 명시 |
| `wuxia-core/docs/world/history-characters.md` v1.2 §10·§11 | 사건별 인물 매핑 |

## 9. 시작 체크리스트

1. `task-phase5a-event-vertical-slice.md` + Phase 5a 보고서 빠르게 훑기
2. `task-phase5b-era-timeline-vertical-slice.md` + Phase 5b 보고서 빠르게 훑기
3. `task-phase5-followup-historical-npcs.md` v1.2 + 보고서 — 5c.1 산출 npc 일람
4. `wuxia-core/docs/world/history.md` §1.1·1.2·1.3·1.4 통독 — 본 follow-up 핵심 입력
5. `wuxia-core/docs/characters/npc-11-taemuje.md` 본기 통독 — 체크포인트 1 입력
6. **event-taemuje-enthronement 단독 변환** → ★체크포인트 1★ 보고 → **commit pause**
7. 5 mid-era event 추가 + era body 정합 → 체크포인트 2

## 10. 리뷰 채널

체크포인트 1·2 보고서를 디렉터(사용자)가 Cowork 세션에 복붙. 형식:
- **Done** · **Diff** · **데모 명령** · **결정** · **막힌 것** · **다음 의견**
- kind 신규 vs 재사용 결정 근거 + related_events 양방향 정합 결과 + era key_events
  갱신 결과 명시

보고서 파일명:
- 체크포인트 1: `docs/tasks/phase5-followup-mid-era-events-checkpoint1-report.md`
- 체크포인트 2: `docs/tasks/phase5-followup-mid-era-events-checkpoint2-report.md`

## 11. 변경 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|-----------|
| v1.0 | 2026-05-03 | 초안 — Phase 5c.1 종결 후속 두 번째 follow-up. 6 mid-era event 후보 정형 (체크포인트 1=태무제 즉위 / 체크포인트 2=나머지 5건). era 분포 검증 (founding 2 / prosperity 1 / turning 3 / decline 1 / fall-of-empire 5 = 12 total). |
