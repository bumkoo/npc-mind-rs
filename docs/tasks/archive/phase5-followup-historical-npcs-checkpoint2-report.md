# Phase 5c.1 체크포인트 2 보고서 — 7 historical/active NPC 정밀 매핑 + Phase 5a 외래키 매트릭스 완전 활성

> **상태**: ✅ 체크포인트 2 통과 — Phase 5c.1 종결 (디렉터 리뷰 후 Phase 5c.2 mid-era events 또는 Phase 6+ 진입).
> **작업 브랜치**: `claude/historical-npcs-phase5c-oR0fL`
> **사양**: `docs/tasks/task-phase5-followup-historical-npcs.md` v1.1
> **작성일**: 2026-05-02
> **선행**: `phase5-followup-historical-npcs-checkpoint1-report.md` (체크포인트 1 + follow-up 리뷰 반영)

## Done

- [x] **7 NPC 변환 완료** (사양 §3.3 권장 7건 채택, 선택 추가 4건은 Phase 6+ 이관):
  - **필수 4 active**:
    - [x] `npc-08.md` 바투 (북원 늑대왕 / 설화 부친) — `heritage_doc_pending: true`, `hexaco_confidence: precise`
    - [x] `npc-09.md` 진대인 (동해 진씨 상방 당주) — `heritage_doc_pending: true`, `hexaco_confidence: precise`
    - [x] `npc-10.md` 3대 천마 (천마신교 교주 / 설화 사부) — `heritage_doc_pending: true`, `hexaco_confidence: precise`
    - [x] `npc-11.md` 소풍자 stub 승급 — `legacy source_status` 제거 + `heritage_doc_pending: true` + `hexaco_confidence: precise`
  - **핵심 historical 3**:
    - [x] `npc-chuyangjinin.md` 추양진인 (화산 마지막 장문인) — `heritage_doc_pending: true`, `hexaco_confidence: pending`
    - [x] `npc-jincheonmyeong.md` 진천명 (대진 태조) — `heritage_doc_pending: true`, `hexaco_confidence: pending`
    - [x] `npc-danun.md` 단운 (태무제) — **`heritage_doc_pending: false`, `hexaco_confidence: precise`** ★ 디렉터 사양과 다름 (보고서 §결정 1 명시)
- [x] **Phase 5a Event 외래키 매트릭스 완전 활성** (5 Event 갱신):
  - [x] `event-bloody-cult-rebellion-2nd` — `npc-danun` + `npc-chuyangjinin` + `npc-im-seoun` 추가 (boundary 해소, 디렉터 옵션 b)
  - [x] `event-bloody-night` — `npc-chuyangjinin` 추가 (`npc-im-seoun` 이미 있음)
  - [x] `event-hwasan-fall` — `npc-chuyangjinin` 추가 (`npc-im-seoun` 이미 있음)
  - [x] `event-blood-disappearance` — `npc-chuyangjinin` 추가 (디렉터 history 정합 검토 결과 추가)
  - [x] `event-empire-founding` — `npc-jincheonmyeong` 추가 (대진 태조)
- [x] 산문 `(npc 미등록) 추양진인/태조 진천명/단운` 마커 정정 (5 Event 모두)
- [x] `cargo build --features embed` 통과
- [x] `cargo test --features embed --lib` → **560 passed (회귀 0건)**
- [x] world-load Phase 1·2·3·4·5a·5b·5c.1·5c.2 통합 ingest 통과:
  - `persons indexed = 16` (Phase 5b 9 + 임서운 1 [Phase 5c.1] + 6 [Phase 5c.2] = 16)
  - `mind eligible = 12` (12 active + player → historical 4명 제외, 정책 정합)
  - `fk errors (활성) = 0`
- [x] Phase 1·2·3·4 e2e 회귀 (54 tests 합산: 14 + 11 + 12 + 7 + 13 + 9 + 7) 통과

## Diff (Phase 5c.1 체크포인트 2 누적)

```
 docs/tasks/phase5-followup-historical-npcs-checkpoint2-report.md       (신규, 본 문서)
 projects/chilguk-chunchu/world/person/npc-08.md                        (신규, ~110줄, 바투)
 projects/chilguk-chunchu/world/person/npc-09.md                        (신규, ~120줄, 진대인)
 projects/chilguk-chunchu/world/person/npc-10.md                        (신규, ~125줄, 3대 천마)
 projects/chilguk-chunchu/world/person/npc-11.md                        (stub 승급, 70 → ~125줄, 소풍자)
 projects/chilguk-chunchu/world/person/npc-chuyangjinin.md              (신규, ~140줄, 추양진인)
 projects/chilguk-chunchu/world/person/npc-jincheonmyeong.md            (신규, ~135줄, 진천명)
 projects/chilguk-chunchu/world/person/npc-danun.md                     (신규, ~165줄, 단운)
 projects/chilguk-chunchu/world/event/event-bloody-cult-rebellion-2nd.md (3 hunks: participants + director_decisions + 핵심 인물 + 전개)
 projects/chilguk-chunchu/world/event/event-bloody-night.md             (3 hunks: participants + director_decisions + 핵심 인물)
 projects/chilguk-chunchu/world/event/event-hwasan-fall.md              (3 hunks: participants + director_decisions + 핵심 인물)
 projects/chilguk-chunchu/world/event/event-blood-disappearance.md      (3 hunks: participants + director_decisions + 핵심 인물)
 projects/chilguk-chunchu/world/event/event-empire-founding.md          (3 hunks: participants + director_decisions + 발단 + 핵심 인물)
```

src 코드 변경 0줄 (체크포인트 1과 동일) — Phase 2 Person 도메인 + Phase 5a Event 도메인
+ Phase 5b world-load FK 검증 인프라 그대로 작동.

## 데모 명령

```bash
cargo build --features embed
cargo test --features embed --lib                                    # 560 passed
cargo test --features embed --test world_chilguk_chunchu_e2e         # 14 passed
cargo test --features embed --test world_chilguk_chunchu_person_e2e  # 11 passed
cargo test --features embed --test world_chilguk_chunchu_persons_batch_e2e  # 12 passed (EXPECTED_PERSON_IDS 7인 한정)
cargo test --features embed --test world_chilguk_chunchu_player_e2e  # 7 passed
cargo test --features embed --test world_chilguk_chunchu_phase4_checkpoint{1,2}  # 13 + 9 passed
cargo test --features embed --test world_load_fk_negative_event      # 7 passed

cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload --db /tmp/world-phase5c2.sqlite
```

## 결과

```
=== 결과 ===
project           = chilguk-chunchu
groups indexed    = 6
persons indexed   = 16                  ← 10 → 16 (+npc-08·09·10·chuyangjinin·jincheonmyeong·danun)
places indexed    = 11
atlases indexed   = 1
events indexed    = 6
eras indexed      = 5
timelines indexed = 1
errors            = 0
group cycles      = 0
place cycles      = 0
fk errors (활성)  = 0                   ← 새 외래키 9건 추가에도 결손 0
mind eligible     = 12                  ← 9 → 12 (+npc-08·09·10 active. historical 4명은 제외, 정책 정합)
```

## 사용자 결정 처리 결과

### 1. boundary 케이스 (event-bloody-cult-rebellion-2nd) — 옵션 (b) 채택 ✅

디렉터 결정대로 추양진인 + 임서운 + 단운 동시 추가. 산문 §전개에 다음 명시:

> 추양진인(npc-chuyangjinin, 화산 장문인)의 80년 혈교 감시 정보·지식이 토벌의 핵심 —
> 이후 10년간 화산파를 위험 인물로 만든다. 추양진인은 본 사건 참전·정보 제공 시
> 수제자 임서운(npc-im-seoun, 당시 청년기)을 동행시켜 화산파의 혈교감시단 기록 보존에
> 직접 관여하게 함 (Phase 5c.2 외래키 활성).

사용자 발화 "4건"과 출처 정합성 양쪽 만족. 임서운의 240년차 활동은 추양진인 수제자
신분상의 합리적 추정 (산문에 명시).

### 2. legacy npc-07·11 마이그레이션 — 비강제 채택 ✅

- **npc-11 소풍자**: stub 승급 시점에 자연 정리 — `source_status: heritage-pending` 키
  제거 + `heritage_doc_pending: true` + `hexaco_confidence: precise` 분리. npc-05-soyeon.md
  §사부 소풍자·§12년 전·§8년 전 파견 다중 묘사로 신뢰도 정밀 등급 도달.
- **npc-07 천순제**: `source_status: heritage-pending` 그대로 유지. Phase 6+ 정밀 패스
  시점에 마이그레이션 예정.

### 3. 인물 범위 — 권장 7건 채택 ✅

- 필수 4 active: npc-08·09·10·11 (모두 정밀 등급)
- 핵심 historical 3: npc-chuyangjinin·npc-jincheonmyeong·npc-danun
- 선택 추가 4건 (풍만리·설무한·자양진인·천리안): Phase 6+ 또는 별도 follow-up. 본
  Phase 스코프 외 — 산문에 (npc 미등록) 마커 유지 + Phase 5c.1 §3.3 "선택 추가 4건"
  주석 명시.

### 4. Event 외래키 매트릭스 갱신 — 전부 활성 ✅

| Event | year_relative | 추가 | 누적 participants.people |
|---|---|---|---|
| `event-empire-founding` | -270 | +`npc-jincheonmyeong` | npc-jincheonmyeong (1건) |
| `event-bloody-cult-rebellion-2nd` | -30 | +`npc-danun` +`npc-chuyangjinin` +`npc-im-seoun` (boundary) | npc-01, npc-danun, npc-chuyangjinin, npc-im-seoun (4건) |
| `event-blood-disappearance` | -12 | +`npc-chuyangjinin` (history 정합 검토 결과) | npc-02, player, npc-im-seoun, npc-chuyangjinin (4건) |
| `event-bloody-night` | -10 | +`npc-chuyangjinin` | npc-02, npc-07, npc-01, npc-03, npc-im-seoun, npc-chuyangjinin (6건) |
| `event-hwasan-fall` | -10 | +`npc-chuyangjinin` | npc-02, npc-03, player, npc-im-seoun, npc-chuyangjinin (5건) |
| `event-six-states-independence` | -7 | (변경 없음) | npc-03, npc-04 (2건) |

총 6 Event 중 5 Event 갱신 + 1 Event 변경 없음 (six-states-independence는 본 Phase
historical NPC 미등장).

## 디렉터 사양 갱신 — npc-danun 분류 (보고할 결정)

### 결정 1: npc-danun을 `heritage_doc_pending: false` + `hexaco_confidence: precise`로 분류

**디렉터 사양**: "핵심 historical 3 (모두 heritage-pending + hexaco_confidence: pending)" —
npc-danun도 pending 분류 의도.

**본 보고 결정**: `heritage_doc_pending: false` + `hexaco_confidence: precise` 적용.

**근거**: `wuxia-core/docs/characters/npc-11-taemuje.md`가 단독 본기(本紀) 형식 + Big Five
+ 가치관 + 성격 변질 곡선 + 7시각 회상을 명시 보유. 사양 §3.3 직교 플래그 정의:

> `extras.heritage_doc_pending` (`bool`): **열전 단독 .md 부재** —
> `wuxia-core/docs/characters/npc-{XX}-{name}.md` 같은 단독 캐릭터 시트가 없음.

→ npc-danun은 본기 .md가 wuxia-core에 있으므로 `false`. hexaco_confidence는 본기에
명시된 Big Five 기반이므로 `precise`.

**디렉터 사양과 다른 분류이므로 별도 보고**:
- 디렉터 사양 §3.3 "핵심 historical 3"의 hexaco_confidence 분류는 추양진인·진천명·
  단운 모두 pending. 본 보고는 단운만 precise로 분류.
- 디렉터가 (a) 본 결정 유지 (b) 사양 일관성을 위해 단운도 pending으로 변경 (c) 다른
  처리 — 셋 중 결정 후보.
- 권장: 본 결정 유지 (출처 정합성). 디렉터 사양은 "단독 wuxia-core 본기 .md가 있는
  historical 인물은 precise 가능"으로 §3.3 갱신.

### 결정 2: npc-09 진대인 출처 — npc-09-jinyarim.md ≠ npc-05-soyeon.md

**디렉터 사양**: "npc-09 진대인 ([npc-09-jinyarim.md] 깊은 묘사 → hexaco_confidence: precise 가능)"

**본 보고 발견**: `wuxia-core/docs/characters/npc-09-jinyarim.md`는 **진야림(陳野林,
영주 왕가 막내 왕자)**의 문서로 본 진대인(陳大人, 동해 진씨 상방 당주)과 **별개 인물**.
character-roster H20 진해의 후예 라인이 진대인이며, 진야림은 별도 historical figure
(255년차 영주 숙청 생존자, 본 Phase 미등록).

**실제 깊은 출처**: `wuxia-core/docs/characters/npc-05-soyeon.md` §사부 소풍자·§12년 전
사건·§8년 전 파견·§소연이 본 사부의 다중 묘사. 본 인물 진대인은 소연(npc-05) 시점에서
"진대인이 보는 소연" 관점으로 깊이 묘사됨.

**결정**: hexaco_confidence: precise 등급은 그대로 유지 (출처가 npc-09-jinyarim.md가
아닌 npc-05-soyeon.md임을 산문 §출처 노트에 명시). 디렉터 사양의 파일명 참조는 단순
혼동으로 추정.

### 결정 3: 추가 디렉터 결정 후보 — npc-11 소풍자 in event-blood-disappearance

`npc-05-soyeon.md` §12년 전 사건에 명시:

> 사건: 혈교가 황제(태무제) 수명연장 실험을 위해 아이들을 납치. 소연은 납치된 아이들 중 하나.
> ...
> 개방 거지들이 아이 납치의 진실을 탐지. 소풍자가 이끄는 개방 15명이 낙양 지하 침입.

→ 소풍자(npc-11)가 본 사건의 "개방 15명 구출조 인솔자"로 직접 명시. 추가 정합성 매우 강함.

**본 보고 처리**: 디렉터 매트릭스에 명시 없음 + 디렉터 명시 항목 외 임의 추가 금지
(사양 §3.5 "출처 보수성 유지") 정책 적용. 추가 보류, 산문에 디렉터 결정 후보로 명시.

**디렉터 결정 후보**: (a) 추가 (b) 보류 — 디렉터 검토 시 결정. 추가 시 별도 commit으로
분리 가능.

## NPC별 핵심 결정 요약

### npc-08 바투 (active)
- HEXACO: H 0.0 / E +0.4 / X +0.4 / A -0.2 / C +0.6 / O +0.3 (정밀)
- 가치관: 충 0.7, 의 0.6, 효 0.4, 복 0.6, 야 0.5
- 출처: character-roster N8 + npc-06-yayulseolhwa.md §관계·§3대 천마와의 관계·§가치관 다중

### npc-09 진대인 (active)
- HEXACO: H -0.3 / E -0.2 / X +0.3 / A 0.0 / C +0.7 / O +0.4 (정밀)
- 가치관: 충 0.3, 의 0.3, 효 0.7, 복 0.2, 야 0.7
- 출처: character-roster N9 + npc-05-soyeon.md §사부 소풍자·§8년 전 파견 다중 (디렉터
  파일명 참조 정정 — 결정 2 참조)

### npc-10 3대 천마 (active)
- HEXACO: H -0.4 / E -0.3 / X 0.0 / A -0.5 / C +0.5 / O +0.7 (정밀)
- 가치관: 충 0.5, 의 0.4, 효 0.5, 복 0.3, 야 0.7
- 출처: character-roster N10 + npc-06-yayulseolhwa.md §관계·§3대 천마와의 관계·§사상 인용 다중

### npc-11 소풍자 stub 승급 (active)
- HEXACO: H +0.6 / E +0.3 / X +0.4 / A +0.5 / C +0.4 / O +0.5 (정밀, stub → 정밀)
- 가치관: 충 0.6, 의 0.7, 효 0.5, 복 0.2, 야 0.4
- 출처: character-roster N11 + npc-05-soyeon.md §사부 소풍자·§12년 전 사건·§8년 전 파견 다중
- **legacy `source_status: heritage-pending` 제거 + `heritage_doc_pending: true` +
  `hexaco_confidence: precise` 분리** (사양 §3.3 직교 플래그 마이그레이션 사례 1번)

### npc-chuyangjinin 추양진인 (historical)
- status: dead (260년차 멸문 시 전사)
- HEXACO: H +0.6 / E +0.4 / X 0.0 / A +0.3 / C +0.7 / O +0.3 (pending, 잠정)
- 가치관: 충 0.7, 의 0.7, 효 0.7, 복 0.4, 야 0.2
- 출처: character-roster H27 + history-characters §9.1·§11.1·§11.2 단편

### npc-jincheonmyeong 진천명 (historical)
- status: dead (30년차 즈음 추정)
- HEXACO: H +0.5 / E 0.0 / X +0.4 / A +0.4 / C +0.7 / O +0.5 (pending, 잠정)
- 가치관: 충 0.7, 의 0.6, 효 0.5, 복 0.3, 야 0.5
- 출처: character-roster H01 + history.md §1.1 + history-characters §1·§13 단편

### npc-danun 단운 / 태무제 (historical)
- status: missing (255년차, 향년 48세 — 사망/행방불명)
- HEXACO: H -0.4 / E +0.4 / X 0.0 / A -0.2 / C +0.6 / O +0.8 (**precise**)
- 가치관: 충 0.9, 의 0.4, 효 0.4, 복 0.5, 야 0.9
- 출처: **wuxia-core/docs/characters/npc-11-taemuje.md 본기 §1·§2(Big Five)·§3(가치관)·
  §관계 7시각 정밀 출처**
- **`heritage_doc_pending: false`** (본기 .md 존재) + **`hexaco_confidence: precise`**
  → 디렉터 사양과 다른 분류 (결정 1 참조)

## mind eligible 변화 검증 — 9 → 12

| 시점 | mind eligible | 변동 |
|---|---|---|
| Phase 5b 종결 | 9 | 8 active + 1 player |
| Phase 5c.1 체크포인트 1 | 9 | npc-im-seoun (historical) 추가 — mind 미등록 |
| **Phase 5c.1 체크포인트 2** | **12** | npc-08·09·10 active 추가 (+3) → 11 active + 1 player = 12 |

(디렉터 사양은 "12 active + player = 13"으로 표기됐으나 실제 npc-01~11 = 11 active.
12 active 시점은 player를 active로 셀 때만 — 정의상 player kind="player"는 active와
별개 카운트라 11 + 1 = 12. 사양 표기는 단순 카운트 오류, 실제 의도와 일치.)

회귀 가드 단위 테스트 `is_mind_eligible_only_active_or_player`:
- `kind="active"` → true ✅ (npc-08·09·10·11)
- `kind="player"` → true ✅
- `kind="historical"` → false ✅ (npc-im-seoun·chuyangjinin·jincheonmyeong·danun)
- `kind="legendary"` → false (Phase 6+ 후보)

## Phase 5a Event 외래키 매트릭스 — `(npc 미등록)` 마커 잔존 분석

| Event | (npc 미등록) 잔존 | 사유 |
|---|---|---|
| `event-empire-founding` | 4건 | 현무진인·원혜대사·자양진인·적마존·천리안 — 선택 추가 4건 후보 또는 Phase 6+ |
| `event-bloody-cult-rebellion-2nd` | 3건 | 풍만리·설무한·2대 천마 — 선택 추가 4건 후보 또는 Phase 6+ |
| `event-blood-disappearance` | 2건 | 다수의 무림 고수·개방 거지 — 익명/집단 |
| `event-bloody-night` | 0건 ✅ | 본 Phase에서 모두 해소 |
| `event-hwasan-fall` | 0건 ✅ | 본 Phase에서 모두 해소 |
| `event-six-states-independence` | 0건 (현역 NPC만) | 본 Phase historical NPC 미등장 |

**완전 해소(0건)**: 2 Event (bloody-night, hwasan-fall) — Phase 5c.1 핵심 분기 사건.
**부분 해소**: 3 Event (empire-founding, bloody-cult-rebellion-2nd, blood-disappearance) —
선택 추가 4건 + 익명 인물 후보. Phase 6+ historical/legendary group 카테고리에서 자연
처리.

사양 §4 Done Criteria의 "산문 (npc 미등록) 마커 0건"은 **권장 7건 범위 내에서 0건
도달** (추양진인·임서운·단운·진천명 모두 해소). 선택 추가 4건은 별도 작업이라 본
Phase 종결 기준 정합.

## 7 H2 섹션 일관 — npc-06/07/im-seoun 패턴 미러

7 신규 person 모두 7 H2 섹션 + frontmatter `extras.secret` (사양 §3.8) + 직교 플래그
(사양 §3.3) 적용:

| Person | 섹션 | extras.secret | heritage_doc_pending | hexaco_confidence |
|---|---|---|---|---|
| npc-08 | 7 | 3건 | true | precise |
| npc-09 | 7 | 3건 | true | precise |
| npc-10 | 7 | 3건 | true | precise |
| npc-11 (승급) | 7 | 3건 | true | precise |
| npc-chuyangjinin | 7 | 3건 | true | pending |
| npc-jincheonmyeong | 7 | 3건 | true | pending |
| npc-danun | 7 | 3건 | **false** | **precise** ★ |

## 회귀 가드 결과 요약

| 검증 | 결과 |
|---|---|
| `cargo build --bin world-load --features embed` | ✅ |
| `cargo test --features embed --lib` | ✅ 560 passed |
| `cargo test --features embed --test world_chilguk_chunchu_e2e` | ✅ 14 passed |
| `cargo test --features embed --test world_chilguk_chunchu_person_e2e` | ✅ 11 passed |
| `cargo test --features embed --test world_chilguk_chunchu_persons_batch_e2e` | ✅ 12 passed (EXPECTED 7인 한정) |
| `cargo test --features embed --test world_chilguk_chunchu_player_e2e` | ✅ 7 passed |
| `cargo test --features embed --test world_chilguk_chunchu_phase4_checkpoint{1,2}` | ✅ 13 + 9 passed |
| `cargo test --features embed --test world_load_fk_negative_event` | ✅ 7 passed |
| world-load Phase 1·2·3·4·5a·5b·5c.1·5c.2 통합 ingest | ✅ persons=16, mind eligible=12, fk errors=0 |
| Phase 5a 6 Event 외래키 매트릭스 (5 Event 갱신 + 1 Event 변경 없음) | ✅ |

**환경 의존성 사유 미회귀**: `tests/embed_test.rs` 6 PAD 분석 테스트는 ONNX 모델
(`../models/bge-m3/`) 부재로 실패 — 본 Phase 변경과 무관 (사전 환경 이슈).

## 막힌 것

없음 — 디렉터 명시 결정 7개(boundary·legacy·범위·외래키 매트릭스·검증 4건) 모두
명확하게 처리.

**디렉터 보고 사항** (정보용, 본 보고서 §결정 참조):
- **결정 1 (npc-danun precise 분류)**: 디렉터 사양 "핵심 historical 3 = pending"과
  다른 분류. wuxia-core 본기 .md 존재로 source-fidelity 우선. 사양 §3.3 갱신 권장.
- **결정 2 (npc-09 출처 정정)**: 디렉터 사양 [npc-09-jinyarim.md]는 진야림 ≠ 진대인.
  실제 출처는 npc-05-soyeon.md. 산문 §출처 노트 명시.
- **결정 3 (npc-11 in blood-disappearance)**: 추가 정합성 매우 강하나 디렉터 매트릭스
  외 항목이라 보류. 추가 시 별도 commit으로 분리 가능.

## Phase 5c.1 종결 판정

✅ **체크포인트 1·2 모두 통과**:
- 체크포인트 1 (commit `2d6c683` + follow-up `2c19f04`): 임서운 단독 + 3 Event FK + 산문 정정 + 사양 v1.1 갱신
- 체크포인트 2 (본 commit): 7 NPC + 5 Event FK + 사양 §3.3 직교 플래그 적용 사례

**Phase 5c.1 종결**. 디렉터 통과 시 Cowork에서 다음 작업 진입:

### 다음 단계 (Phase 5c.2 또는 Phase 6+)

1. **Phase 5c.2 mid-era events** (Cowork 작업): `task-phase5-followup-mid-era-events.md`
   작성 — 본 Phase 산출 historical NPC(추양진인·진천명·단운)를 history.md §0.2의 미시드
   14사건(전성기·변곡기·쇠퇴기 사건들) 중 핵심 5-10건의 participants로 활용.
   - 30년차 병권 회수 (태광제 H07 + 화산 진무양 H09 + 무당 태허진인 H11)
   - 100년차 무림대회 (소림 혜통 H13 + 화산 적하검 H14 + 개방 일소옹 H15)
   - 130년차 사파 형성 (초대 천마 H16 + 정심 대사 H19)
   - 160년차 자치 운동 (진해 H20 + 아골타 H21 + 곽천풍 H22)
   - 190년차 혈교 잔당 발견 (진여 H23 + 수월진인 H25)
   - 240년차 추가: 풍만리 H30 + 설무한 H31 (선택 추가 4건의 핵심)

2. **Phase 6+ Skill·Item·Knowledge·Lore** (이전 로드맵): 인스턴스 도메인 4종 추가.
   본 Phase 산출은 모두 호환.

### 권장 디렉터 결정 (Phase 5c.2 진입 전)

1. **npc-danun precise 분류 확정 또는 변경** (본 보고 §결정 1) — 사양 §3.3 갱신 여부.
2. **npc-11 in blood-disappearance 추가 여부** (본 보고 §결정 3) — 별도 commit 분리.
3. **Phase 5c.2 범위 확정** — mid-era events 5건 vs 10건. 선택 추가 4건 historical NPC
   포함 여부.
4. **Phase 6+ historical group 등록 시점** — 화산파·천마신교 외 historical group 추가
   (group-bukwon-tribe·group-jin-merchant-house·group-donghae-federation) 시점.
