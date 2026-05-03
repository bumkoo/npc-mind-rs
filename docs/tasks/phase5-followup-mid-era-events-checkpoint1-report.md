# Phase 5c.2 체크포인트 1 보고서 — event-taemuje-enthronement 단독 변환

> **대상 task**: [`docs/tasks/task-phase5-followup-mid-era-events.md`](task-phase5-followup-mid-era-events.md) v1.0 §5 Step 1 + §6
> **선행 컨텍스트**: Phase 5c.1 종결 (npc-danun·npc-chuyangjinin·npc-jincheonmyeong·npc-im-seoun + npc-08·09·10·11 등록 완료)
> **체크포인트 분리 게이트**: 본 보고서 후 **commit pause 유지** — 디렉터 통과 신호 받고 체크포인트 2 진행

---

## Done

체크포인트 1 스코프 전부 처리. 사료 정합 검토에서 task §5 Step 1 가설 1건 기각 (Q4 — 조고 즉위 시점 미관여), 이외는 task 권장값 그대로 채택.

### 데이터 변경

| # | 파일 | 변경 |
|---|------|------|
| 1 | `projects/chilguk-chunchu/world/event/event-taemuje-enthronement.md` | **신규 작성** — Phase 5a 패턴 정합 (frontmatter + 산문 §개요·발단·전개·결과·핵심 인물·게임에서의 역할) |
| 2 | `projects/chilguk-chunchu/world/era/era-decline.md` | `key_events: []` → `[event-taemuje-enthronement]` + §핵심 인물 마커 정합 (단운·추양진인) |
| 3 | `projects/chilguk-chunchu/world/event/event-bloody-cult-rebellion-2nd.md` | `related_events`에 `event-taemuje-enthronement` 추가 (역방향 — +3년) |
| 4 | `projects/chilguk-chunchu/world/event/event-blood-disappearance.md` | `related_events`에 `event-taemuje-enthronement` 추가 (역방향 — +21년) |
| 5 | `projects/chilguk-chunchu/world/event/event-bloody-night.md` | `related_events`에 `event-taemuje-enthronement` 추가 (역방향 — +23년) |

코드 변경 0건. 도메인 enum 그대로.

### event-taemuje-enthronement 핵심 frontmatter

```yaml
id: event-taemuje-enthronement
kind: founding                       # ★ Q1 결정 — empire-founding과 같은 권력 등극 카테고리, 신규 enthronement 미도입
category: historical
name: 태무제 즉위
aliases: [237년차 즉위, 단운 등극]
temporal:
  year_relative: -33                 # task §3.2 표 그대로
era_id: era-decline                  # -33 ∈ [-70, -30) 안쪽
participants:
  people: [npc-danun]                # ★ Q4 결정 — 조고 미포함 (사료 정합)
  groups: [group-daejin-court]       # 십상시 미포함 (245년차 결성, 즉위 시점 미존재)
  places: [place-daejin]
related_events:
  - event-bloody-cult-rebellion-2nd  # +3년 — 즉위 후 혈교 비술 거래 시작이 침공 직접 인과
  - event-blood-disappearance        # +21년 — 태무제 시대 권력 균열 누적 결과
  - event-bloody-night               # +23년 — 태무제 시대 황실 권위 손상의 붕괴 트리거
extras:
  player_relevance: 3
```

### 외래키 활성 결과

| 활성 대상 | 등록 시점 | 위치 |
|---|---|---|
| **npc-danun** (단운/태무제) | Phase 5c.1 산출 | event-taemuje-enthronement.participants.people · era-decline.md §핵심 인물 |
| **npc-chuyangjinin** (추양진인) | Phase 5c.1 산출 | era-decline.md §핵심 인물 (event participants 미포함 — 즉위에 직접 관여 사료 없음) |

### era-decline 산문 정합 (디렉터 사양 §6.6)

`(npc 미등록) 단운(태무제)` → `npc-danun 단운(태무제)`
`(npc 미등록) 추양진인` → `npc-chuyangjinin 추양진인`

미등록 인물(풍만리·설무한·2대 천마)은 텍스트 보존 (Phase 5a D1 정책 그대로).

---

## Diff

```text
M projects/chilguk-chunchu/world/era/era-decline.md
M projects/chilguk-chunchu/world/event/event-blood-disappearance.md
M projects/chilguk-chunchu/world/event/event-bloody-cult-rebellion-2nd.md
M projects/chilguk-chunchu/world/event/event-bloody-night.md
+ projects/chilguk-chunchu/world/event/event-taemuje-enthronement.md   (신규)
```

총 5 파일 (1 신규 + 4 수정). 산출물 lines: event 신규 ~110줄 + 역방향 1줄×3 + era key_events 2줄 + era 핵심 인물 마커 2줄.

---

## 데모 명령

```bash
# 1. world-load — events 6 → 7, fk errors=0 확인
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload

# 결과 (실측):
#   events indexed = 7
#   eras indexed   = 5
#   fk errors (활성) = 0
#   group cycles   = 0
#   place cycles   = 0
#   mind eligible  = 12

# 2. world e2e 회귀 — 13 binaries 통과
cargo test --features embed --test world_chilguk_chunchu_e2e          # 14 passed
cargo test --features embed --test world_chilguk_chunchu_phase5c_e2e  # 20 passed
cargo test --features embed --test world_load_fk_negative_event       # 7 passed
# (총 13 world test binary, 121 testcases, 0 failed)

# 3. event 라운드트립 수동 검증
sqlite3 projects/chilguk-chunchu/build/world.sqlite \
  "SELECT id, kind, year_relative, era_id FROM events WHERE id='event-taemuje-enthronement';"
# 출력: event-taemuje-enthronement|founding|-33|era-decline

# 4. era key_events 검증
sqlite3 projects/chilguk-chunchu/build/world.sqlite \
  "SELECT key FROM era_key_events WHERE era_id='era-decline' ORDER BY ordinal;"
# 출력: event-taemuje-enthronement
```

---

## 결정

### Q1. Event.kind = `founding` (재사용) ★ task §3.3·§6.2 권장값 채택

**결정**: `founding` 재사용. 신규 `enthronement` 미도입.

**근거**:
- empire-founding(원년)과 같은 권력 등극 본질. founding의 의미 = "권력 단위 출범".
- 신규 `enthronement`는 founding의 부분집합 (등극 사건 = 권력 단위 출범의 한 형태).
- task §3.3 "founding 재사용 권장 — empire-founding과 같은 카테고리" 채택.
- 카탈로그 비대화 회피. Event.kind는 string이라 자유롭게 추가 가능하지만 의미 중복은 분류 가치 약화.
- **체크포인트 2 6 event 후보 중 5건은 신규 kind 도입 권장** (reform-fail·convention·schism·political-movement·discovery — task §3.3 표): 본 결정과 모순 없음. 해당 5건은 founding으로 환원되지 않는 분리 의미가 있음.

### Q4. participants에 npc-02(조고) **미포함** ★ task §5 Step 1 가설 기각

**결정**: `participants.people = [npc-danun]` 단독. `participants.groups = [group-daejin-court]` 단독 (group-shipsangsi 미포함).

**근거** (사료 정합 검토 — wuxia-core 출처 직접 확인):
- `wuxia-core/docs/characters/npc-02-jogo.md` §상세 연표 (line 224-234):
  - 235년차 (35년 전, 20세): 입궁
  - **237년차 (즉위 시점, 22세): 하급 관리** — 즉위의 직접 행위자가 아님
  - 240년차 (30년 전, 25세): 2차 혈교 침공 시 태무제 측근 진입
  - 245년차 (25년 전, 30세): 십상시 결성
  - 255년차 (15년 전, 40세): 영주 숙청 (조고의 첫 대규모 숙청)
- 즉위 시점 조고 = 22세 하급 관리. 즉위 정치 배후 의혹은 사료에 없음.
- `group-shipsangsi`도 245년차 결성이라 즉위 시점 미존재 → groups에서 제외.
- task §5 Step 1 "npc-02(조고 — 즉위 정치 배후 의혹)"는 가설로 제시됐고 디렉터가 Q4에서 검토 요청 — 사료 정합 결과 **기각**.
- history-characters §10.1는 258년차 피의 실종 사건 시점이라 즉위 시점과 무관. 그 시점 십상시 핵심으로 조고가 명시되지만 즉위 시점에는 십상시 자체가 미존재.
- **대안 채택 가능성**: 조고가 즉위 후 점진적으로 측근화된 흐름은 산문 §결과에 텍스트로 기록 ("21년 후 피의 실종 — 태무제 시대 권력 균열의 누적 결과"). 정식 외래키 활성은 조고가 직접 행위자인 240년차 이후 사건들에 한정.

**참여자 최종**:
| field | 값 | 근거 |
|---|---|---|
| people | `[npc-danun]` | wuxia-core npc-11-taemuje.md 본기 §1·§4 |
| groups | `[group-daejin-court]` | 즉위 무대 |
| places | `[place-daejin]` | 등극 무대 (낙양 황궁은 place 미등록) |

### Q2·Q3 (체크포인트 1 외 — 체크포인트 2에서 결정 필요)

본 체크포인트 1은 event-taemuje-enthronement 단독 작업이라 Q2(byeongkwon 슬러그)·Q3(혜통대사·적하검 외래키)는 체크포인트 2 진입 시 디렉터 결정 필요. 디렉터 권장값 사전 노트만 명시:

- **Q2 권장**: `event-byeongkwon-recall` (한국어-한자음 + 영문 의미어 혼용) 유지. task §3.2 표·§6.1의 명명 규칙 정합 — 영문 의미어 단독(`military-rights-recall`)은 한자어 고유성 손실. 단 디렉터 결정 우선.
- **Q3 권장**: 혜통대사·적하검은 5c.1 미등록이므로 `event-mulim-conference-1st.participants.people` 비움 + 산문 텍스트만 명시 (Phase 5a D1 정책). Phase 6+ 또는 별도 follow-up에서 추가 historical npc 등록 시 외래키 활성.

---

## 막힌 것

**없음.** 결정 슬롯 4건 중 2건(Q1·Q4)은 본 체크포인트 1 스코프, 둘 다 디렉터 통과 검토 후 체크포인트 2 진행 시 동일 정책 유지 가정.

다만 Q4 결정이 task §5 Step 1 가설과 다르므로 **디렉터 명시적 통과 신호** 필요. 만약 디렉터가 "조고를 정치 배후로 포함"을 선호하면 다음 옵션:

| 옵션 | 결과 | 근거 |
|---|---|---|
| **a** (현 보고서 채택) | participants에 조고·십상시 미포함 | 사료(npc-02 §상세 연표) 정합 우선 |
| b | participants.people에 npc-02 추가, groups에는 미포함 | 조고 22세 시점에도 "정치 배후" 해석 채택, 십상시는 8년 후 결성이라 그룹은 명백 미포함 |
| c | 둘 다 포함 + extras.director_decisions에 "사료 비정합 의도적 채택" 명시 | 게임 내러티브 우선 |

본 보고서는 **옵션 a**로 작성 완료. 디렉터가 b·c를 선호하면 보고서 §결정·event 파일 frontmatter·산문을 갱신.

---

## 검증 결과

### world-load (events 6 → 7, fk=0)
```
project           = chilguk-chunchu
events indexed    = 7    (← 6에서 +1)
eras indexed      = 5
errors            = 0
group cycles      = 0
place cycles      = 0
fk errors (활성)  = 0
mind eligible     = 12
```

### cargo test --features embed (worldbuilding 한정)

13 world test binary 전부 통과 (121 testcases · 0 failed):

| binary | 결과 |
|---|---|
| world_chilguk_chunchu_e2e | 14 passed |
| world_chilguk_chunchu_person_e2e | 11 passed |
| world_chilguk_chunchu_persons_batch_e2e | 12 passed |
| world_chilguk_chunchu_phase3_checkpoint1 | 7 passed (1 ignored) |
| world_chilguk_chunchu_phase3_checkpoint2 | 9 passed |
| world_chilguk_chunchu_phase4_checkpoint1 | 13 passed |
| world_chilguk_chunchu_phase4_checkpoint2 | 9 passed |
| world_chilguk_chunchu_phase5c_e2e | 20 passed |
| world_chilguk_chunchu_player_e2e | 7 passed |
| world_load_fk_negative | 3 passed |
| world_load_fk_negative_era | 5 passed |
| world_load_fk_negative_event | 7 passed |
| world_load_fk_negative_timeline | 4 passed |

**합계**: 121 passed · 0 failed · 1 ignored.

`cargo build --features embed` 통과.

### 환경 의존 실패 (본 follow-up 무관)

ONNX 모델(`../models/bge-m3/`) 미배치로 12개 PAD analyzer 의존 테스트 실패:
- embed_test (6) · dialogue_converter_integration (1) · listener_perspective_integration_bench (1) · magnitude_bench (1) · magnitude_classifier_bench (1) · pad_anchor_count_bench (1) · pad_benchmark_test (1) · pad_colbert_bench (1) · pad_gemini_bench (1) · pad_individual_scores (1) · sign_classifier_bench (1) · sparse_spike (1)

전부 `OrtEmbedder` 초기화 시 모델 디렉토리 부재로 panic. 본 follow-up은 데이터만 수정하므로 무관. CI/local 환경에서 모델 배치 시 해소.

---

## 다음 의견

**체크포인트 2 진입 후 작업 순서 권장**:

1. **6 후보 event 5건 작성** (turning에 3건 — task §3.2 표):
   - event-byeongkwon-recall (-240, era-founding, kind=reform-fail 권장)
   - event-mulim-conference-1st (-170, era-prosperity, kind=convention 권장)
   - event-sapa-formation (-140, era-turning, kind=schism 권장)
   - event-jachi-movement (-110, era-turning, kind=political-movement 권장)
   - event-cult-remnant-discovery (-80, era-turning, kind=discovery 권장)

2. **era 4종 key_events 갱신** (era-decline은 본 체크포인트에서 완료, 나머지 3종):
   - era-founding: `[event-empire-founding, event-byeongkwon-recall]`
   - era-prosperity: `[event-mulim-conference-1st]`
   - era-turning: `[event-sapa-formation, event-jachi-movement, event-cult-remnant-discovery]`

3. **Phase 5a 6 event related_events 추가 역방향 갱신** (체크포인트 1에서 enthronement 3건 완료, 체크포인트 2에서 추가):
   - event-empire-founding.related_events에 event-byeongkwon-recall 추가
   - event-six-states-independence.related_events에 event-jachi-movement 추가
   - event-bloody-cult-rebellion-2nd.related_events에 event-cult-remnant-discovery 추가

4. **e2e 테스트 신규** (task §5 Step 2):
   - `tests/world_chilguk_chunchu_followup_mid_era_events.rs` 신설
   - 6 event 라운드트립 (id·kind·temporal·era_id·related_events 검증)
   - era 4종 key_events 슬롯 채움 검증
   - related_events 양방향 정합 (forward + reverse)
   - 외래키 결손 0건 검증

5. **신규 kind 5종 일람** 보고서 §결정에 명시 (Q1 정책 — Phase 5a처럼 자유 추가)

**환경 의존 12개 ONNX 테스트는 본 follow-up 종결 후에도 실패 유지** — 별도 환경 설정 issue. Phase 5c.2 종결 시점 회귀 검증은 world_chilguk_chunchu_* + world_load_fk_negative_* 13 binary 기준.

**Q4 결정 통과 신호 받기 전 commit pause 유지**. 디렉터 답변 후 체크포인트 2 진입.
