# Phase 5 Follow-up — Historical NPC 시드 확장 (D2 처리)

> **For Claude Code.** Phase 5a Q&A의 D2 결정 후속 작업. 짧은 follow-up TASK.
> **선행 조건**: Phase 5a·5b 모두 종결.
> **체크포인트 분리 게이트 강제 적용** — 1회 통합 commit 금지.

---

## 1. 목표

Phase 5a/5b 본문에 `(npc 미등록)` 텍스트로 보존된 historical NPC들을 정식 시드로 등록하고 외래키 활성. character-roster v1.1의 ★★★★+ 우선순위 인물 + 5a/5b body에 등장한 메인 서사 인물.

**스코프**:
- character-roster ★★★★+ 미작성 인물 풍부화: **npc-08 바투** · **npc-09 진대인** · **npc-10 3대 천마** + npc-11 소풍자 풍부화(stub → 정식)
- Phase 5a/5b body 핵심 인물 신규 등록: **임서운**(player 부친) · **추양진인**(화산 장문인) · **진천명**(대진 태조) · **단운**(태무제)
- 선택 추가: 풍만리(개방 초대)·설무한(천마신교 초대)·자양진인(화산 조사)·천리안(개방 정보)

**검증 게이트**:
- 체크포인트 1: **임서운** 단독 변환 (player 부친·메인 비밀의 핵심)
- 체크포인트 2: 5-7 historical npc 추가 + Phase 5a/5b body 외래키 활성

## 2. 연관 컨텍스트

- `docs/tasks/00-roadmap.md` — D2 결정 로그 (2026-05-02)
- `docs/tasks/task-phase2-person-vertical-slice.md` + Phase 2 보고서들 — Person 도메인·HEXACO 시작값 패턴
- `docs/tasks/phase5a-checkpoint{1,2}-report.md` — Phase 5a body의 `(npc 미등록)` 인물 일람
- 메모리: Person.kind="historical", Person.status=dead/missing/unknown 두 축 (Phase 2 Q3·C)
- 입력 자료:
  - `wuxia-core/docs/world/history-characters.md` v1.2 — 역사 인물 매핑 (★ 핵심 입력)
  - `wuxia-core/docs/characters/character-roster.md` v1.1 — npc-08·09·10·11 우선순위
  - Phase 5a 6 Event + Phase 5b 5 Era body — 참조 인물 일람
  - Phase 2 산출 9 Person — 양식·매핑 패턴

## 3. 제약

### 3.1 도메인 변경 X — 데이터 추가만

Phase 2 Person 도메인 그대로. 코드 변경 0. **데이터 + 테스트만 추가**:
- `projects/chilguk-chunchu/world/person/{npc-08,09,10,임서운id,...}.md` 추가
- `tests/world_chilguk_chunchu_followup_historical_npcs.rs` 신규
- world-load 결과: persons indexed 9 → 14-17

### 3.2 kind = "historical" 또는 "active"

| 인물 | kind | status | 근거 |
|---|---|---|---|
| npc-08 바투 | active | alive | 게임 시작 시 생존, 직접 만남 (character-roster) |
| npc-09 진대인 | active | alive | 동해 상방의회 의장, 생존 |
| npc-10 3대 천마 | active | alive | 게임 시작 시 생존 (천마신교 교주) |
| npc-11 소풍자 | active | alive | 이미 stub, 풍부화 (개방 장로) |
| 임서운 | historical | missing | 10년 전 행방불명 — player 메인 비밀 |
| 추양진인 | historical | dead | 10년 전 화산 멸문 시 사망(추정) |
| 진천명 | historical | dead | 270년 전 대진 태조, 캐논 사망 |
| 단운(태무제) | historical | dead | 30년 전 행방불명/사망 |
| (선택) 풍만리·설무한·자양진인·천리안 | historical | dead | 100년+ 전 인물, 캐논 사망 |

**npc-08·09·10·11은 kind=active**라 mind eligible 등록 (Phase 2 NPC mind 통합).
**나머지 historical은 mind 등록 안 함** (Phase 2 §3.5 정책 — `kind ∈ {active, player}`만 upsert).

### 3.3 ID 명명

character-roster의 `npc-XX` 명명 따름. 추가 historical npc는:
- `npc-imseowoon` (임서운, player 부친) — character-roster 미명명이라 한국어 발음 슬러그
- `npc-chuyangjinin` (추양진인) — 동일 패턴
- `npc-jincheonmyeong` (진천명, 대진 태조) — 동일 패턴
- `npc-danwoon` (단운, 태무제) — 동일 패턴

선택 추가 인물(풍만리·설무한 등)도 같은 패턴.

### 3.4 HEXACO 잠정 매핑 — heritage-pending 또는 정밀

historical npc는 열전 미작성이라 HEXACO 잠정 매핑. 두 옵션:
- (a) **heritage-pending 마커** (npc-07 천순제·npc-11 소풍자 패턴) — `extras.source_status: heritage-pending` + 잠정 6 dim
- (b) **정밀 매핑** — character-roster + history-characters에서 정성 추론 후 신뢰도 표기 (Phase 2 npc-06 패턴)

**권장**: 임서운(체크포인트 1)·npc-08·09·10은 (b) 정밀 매핑, 나머지 historical은 (a) heritage-pending. 이유:
- 임서운은 player 메인 비밀 → 정밀 가치 큼
- npc-08·09·10은 character-roster ★★★★+ → 게임 캐논
- 나머지 historical은 NPC 대사·서적·비문에만 등장 → 잠정 충분

체크포인트 1 보고서에서 디렉터 검토.

### 3.5 외래키 활성

Phase 5a/5b body의 텍스트 ID들이 실제 person 시드 등록 후 활성:
- Phase 5a 6 Event participants.people 검증 (이미 활성, 추가 인물 추가 시 검증 통과)
- Phase 5b 5 Era body 핵심 인물 (텍스트만, 외래키 활성 X — 본 follow-up 외)
- Phase 1 Group members.person_id (이미 활성)

### 3.6 체크포인트 분리 게이트

1. **체크포인트 1**: 임서운 단독 변환 + Phase 5a 6 Event participants에 임서운 추가 → commit pause
2. **체크포인트 2**: 5-7 추가 historical npc + 외래키 통과 → commit pause → Phase 5 follow-up 종결

## 4. Done Criteria

- [ ] 임서운 단독 변환 (체크포인트 1) + HEXACO 정밀 매핑 + status=missing
- [ ] 5-7 historical npc 추가 (체크포인트 2)
- [ ] character-roster ★★★★+ 미작성 인물 (npc-08·09·10) 풍부화 + npc-11 stub → 정식
- [ ] Phase 5a 6 Event body의 `(npc 미등록)` → 정식 person_id 외래키 활성 (가능한 인물만)
- [ ] world-load: persons indexed 9 → 14-17 (선택 추가 인물 수에 따라)
- [ ] mind eligible 9 → 12-13 (npc-08·09·10·11 + player가 active)
- [ ] e2e 테스트 — historical npc 시드 라운드트립 + Phase 5a Event participants 외래키 활성 검증
- [ ] `cargo build` + `cargo test --features embed` + 기존 e2e 회귀 통과

## 5. 단계별 작업

### Step 1 — 임서운 단독 변환 ★체크포인트 1★

대상: 임서운 (player 부친). Phase 2 player.md `## 비밀` + Phase 5a body 다수 참조.

작업:
1. `wuxia-core/docs/world/history-characters.md` v1.2 임서운 항목 통독
2. Phase 2 player.md + Phase 5a 6 Event body의 임서운 언급 종합
3. `projects/chilguk-chunchu/world/person/npc-imseowoon.md` 작성
4. **HEXACO 정밀 매핑** (정파·검학·player 보호자·10년 전 행방불명 결로):
   - H: +0.7 (화산파 정파적 정직성·player 보호의 자비)
   - E: +0.5 (player에 대한 부성애·신경증)
   - X: 0.0 (수도자 평균)
   - A: +0.7 (자비·player에 대한 헌신)
   - C: +0.6 (화산파 수련 규율)
   - O: +0.4 (전통 수도자, 약간 보수적)
5. `status = missing` (10년 전 행방불명)
6. `kind = historical`
7. `extras.player_relevance: ★★★★★`
8. `extras.secret: player 부친 + 추양진인 수제자 + 혈매화검 보유자` (player 메인 비밀의 핵심)
9. Phase 5a 6 Event body의 임서운 언급을 정식 person_id로 갱신:
   - bloody-night participants.people에 npc-imseowoon 추가
   - hwasan-fall participants.people에 npc-imseowoon 추가
   - blood-disappearance participants.people에 npc-imseowoon 추가
10. world-load 통과 — persons=10, fk errors=0

**체크포인트 1 보고서** (`docs/tasks/phase5-followup-historical-npcs-checkpoint1-report.md`):
- npc-imseowoon.md 전문
- HEXACO 결정 근거 (정밀 매핑 6 dim)
- Phase 5a 6 Event body 외래키 갱신 결과
- world-load 통과 (persons=10, fk=0)
- mind eligible 변화 (9 → 9, historical은 mind 등록 X)
- 막힌 결정 (예: 임서운의 group 소속 — group-mulim-mang? 또는 group-hwasan(미등록)?)

→ Cowork 리뷰 → **commit pause 유지** → 통과 신호 받고 Step 2.

### Step 2 — 5-7 historical npc 추가 ★체크포인트 2★

후보 (디렉터 결정 받기):

**필수 4건** (★★★★+):
- npc-08 바투 (북원 늑대왕, kind=active) — character-roster 우선순위
- npc-09 진대인 (동해 상방의회 의장, active) — Phase 1 group-donghae 멤버
- npc-10 3대 천마 (천마신교 교주, active) — character-roster ★★★★★
- npc-11 소풍자 풍부화 (stub → 정식, active)

**Phase 5a body 핵심 historical 3건**:
- 추양진인 (화산 장문인, dead, 10년 전 멸문 시) — Phase 5a hwasan-fall body
- 진천명 (대진 태조, dead, 270년 전) — Phase 5a empire-founding body
- 단운 (태무제, dead/missing, 30년 전) — Phase 5a bloody-cult-rebellion-2nd body

**선택 추가 4건** (history-characters.md 우선순위 ★★★ 이하):
- 풍만리 (개방 초대) — era-founding body
- 설무한 (천마신교 초대) — era-prosperity·era-turning 시드
- 자양진인 (화산 조사) — era-founding body
- 천리안 (개방 정보망 초대) — era-founding body

**권장**: **필수 4 + Phase 5a 핵심 3 = 7건**. 선택 4건은 별도 follow-up 또는 Phase 6+. 단 디렉터 결정.

**필수 4건 (active)**:
- HEXACO 정밀 매핑 권장 (character-roster + history-characters 시드 풍부)
- mind eligible 활성

**핵심 historical 3건 (dead)**:
- HEXACO heritage-pending 마커 권장 (잠정 6 dim + `extras.source_status: heritage-pending`)
- mind 등록 X

**체크포인트 2 보고서**:
- 7-11건 historical npc 일람 (id·kind·status·HEXACO 신뢰도)
- Phase 5a 6 Event participants 외래키 활성 결과
- world-load: persons indexed 9 → 16-20
- mind eligible 9 → 13 (player + 9 active = 13: 1·2·3·4·5·6·7·8·9·10·11·player + 1 = 13)
- 외래키 결손 0건
- search 정성 평가 — "임서운"·"바투"·"진대인"·"3대 천마"·"태무제"·"진천명"
- Phase 5+ follow-up 진입 가능 여부 (mid-era-events)

→ Cowork 리뷰 → 통과 시 Phase 5 historical-npcs follow-up 종결.

## 6. 결정 사항 (변경 시 디렉터 승인)

### 6.1 ID 명명 규칙

- character-roster `npc-XX`: npc-08·09·10·11 그대로
- 추가 historical: `npc-{한국어슬러그}` (예: `npc-imseowoon`·`npc-chuyangjinin`·`npc-jincheonmyeong`·`npc-danwoon`)

### 6.2 HEXACO 매핑 정밀도

- 임서운·npc-08·09·10·11: 정밀 매핑 (Phase 2 npc-06 패턴, 신뢰도 "보통" 이상)
- Phase 5a 핵심 historical (추양진인·진천명·단운): heritage-pending 마커 (npc-07 패턴)
- 선택 추가 historical: heritage-pending

### 6.3 status × kind 매트릭스

| 인물 그룹 | kind | status | mind eligible |
|---|---|---|---|
| npc-08·09·10·11 (active) | active | alive | ⭕ |
| 임서운 | historical | missing | ❌ |
| 추양진인·진천명·단운 | historical | dead | ❌ |
| 선택 추가 | historical | dead/legendary | ❌ |

### 6.4 group_id 매핑

| 인물 | affiliation 후보 |
|---|---|
| npc-08 바투 | group-bukwon-tribes(미등록) — 텍스트만, Phase 6+ |
| npc-09 진대인 | group-donghae-merchant-council(미등록) — 텍스트만 |
| npc-10 3대 천마 | group-cheonma-shingyo (Phase 1 등록) — 외래키 활성 |
| npc-11 소풍자 | group-mulim-mang + group-gaebang(미등록) — 부분 활성 |
| 임서운 | group-mulim-mang? 또는 group-hwasan-pa(미등록)? — 디렉터 결정 |
| 추양진인 | 동일 — group-hwasan-pa(미등록) |
| 진천명 | group-daejin-court(Phase 1 등록 — 단 270년 전 시점이라 era 결합 시 분리 필요?) |
| 단운 | group-daejin-court (Phase 1 등록) |

**미등록 group은 affiliation에서 누락 처리** (Phase 5a D1 정책 그대로). 본문에 텍스트 명시.

### 6.5 Phase 5a 6 Event body 외래키 갱신

본 follow-up 진행 시 다음 갱신:
- bloody-night.participants.people: 임서운·추양진인 추가 (둘 다 historical)
- hwasan-fall.participants.people: 임서운·추양진인 추가
- blood-disappearance.participants.people: 임서운 추가
- bloody-cult-rebellion-2nd.participants.people: 단운 추가
- empire-founding.participants.people: 진천명·풍만리(선택)·설무한(선택)·자양진인(선택)·천리안(선택) 추가

이 갱신이 외래키 활성 검증의 핵심 — historical npc 등록 후 6 Event body가 정식 외래키로 정합.

### 6.6 SQLite·MCP·기타 — 변경 없음

Phase 2 Person 패턴 그대로. 코드 변경 X.

## 7. Out of Scope

- mid-era-events follow-up (별도 TASK — 본 follow-up 종결 후)
- 시기별 atlas 분기 (별도 follow-up 또는 Phase 6+)
- 미등록 group 추가 (group-hwasan-pa·group-bukwon-tribes·group-donghae-merchant-council 등) — Phase 6+ legendary/historical group 카테고리 등장 시
- player 메인 퀘스트 시드 (예: 임서운 추적 quest) — Phase 6+ gameplay 다리
- HEXACO 24 facet 정형 — Phase 4+ 영구 보류 (Phase 5+ 작전과 일관)
- npc-mind 시스템에서 historical npc 활용 (예: 죽은 인물의 dialogue·기억) — Phase 6+

## 8. 코드 위치 가이드

| 위치 | 무엇을 볼지 |
|---|---|
| `projects/chilguk-chunchu/world/person/npc-02.md` (Phase 2) | 정밀 매핑 패턴 (정밀 6 dim + biography_short + game_role) |
| `projects/chilguk-chunchu/world/person/npc-07.md` (Phase 2) | heritage-pending 마커 패턴 |
| `projects/chilguk-chunchu/world/person/npc-11.md` (사용자 작성 stub) | 사용자 직접 stub 패턴 — 본 follow-up이 풍부화 |
| `projects/chilguk-chunchu/world/person/player.md` (Phase 2.1) | player.md 비밀 4종에 임서운 직접 등장 — 임서운 작성 시 정합 검증 |
| `wuxia-core/docs/world/history-characters.md` v1.2 | ★ 핵심 입력. character-roster + history 시드 |

## 9. 시작 체크리스트

1. `task-phase2-person-vertical-slice.md` + Phase 2 보고서 빠르게 훑기
2. `wuxia-core/docs/world/history-characters.md` v1.2 통독 — 본 follow-up 핵심 입력
3. `wuxia-core/docs/characters/character-roster.md` v1.1 통독 — npc-08·09·10·11 우선순위
4. Phase 5a 6 Event body의 `(npc 미등록)` 인물 일람 추출
5. **임서운 단독 변환** → ★체크포인트 1★ 보고 → **commit pause**
6. 7-11 historical npc 추가 → 체크포인트 2

## 10. 리뷰 채널

체크포인트 1·2 보고서를 디렉터(사용자)가 Cowork 세션에 복붙. 형식:
- **Done** · **Diff** · **데모 명령** · **결정** · **막힌 것** · **다음 의견**
- HEXACO 매핑 결정 근거 (정밀 vs heritage-pending) + group affiliation 처리 + Phase 5a Event body 외래키 갱신 결과 명시

보고서 파일명:
- 체크포인트 1: `docs/tasks/phase5-followup-historical-npcs-checkpoint1-report.md`
- 체크포인트 2: `docs/tasks/phase5-followup-historical-npcs-checkpoint2-report.md`
