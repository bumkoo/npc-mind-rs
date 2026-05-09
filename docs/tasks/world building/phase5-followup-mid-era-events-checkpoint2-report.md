# Phase 5c.2 체크포인트 2 보고서 — mid-era 5 event 추가 + era 정합 + e2e 테스트

> **대상 task**: [`docs/tasks/task-phase5-followup-mid-era-events.md`](task-phase5-followup-mid-era-events.md) v1.0 §5 Step 2 + §6
> **선행 컨텍스트**: 체크포인트 1 디렉터 통과 (PR #76 머지 가능 상태). Q1·Q4 권장값 채택 + Q2·Q3·신규 kind 5종 권장값 채택.
> **체크포인트 분리 게이트**: 본 보고서 후 **commit pause 유지** — 디렉터 통과 신호 받고 Phase 5 시리즈 종결.

---

## Done

체크포인트 2 스코프 전부 처리. 디렉터 통과 신호 받은 권장값 4건(Q1·Q2·Q3·Q4) + 신규 kind 5종 도입 그대로 적용. 코드 변경 0 — 데이터 + 테스트만.

### 데이터 변경

| # | 파일 | 변경 |
|---|------|------|
| 1 | `projects/chilguk-chunchu/world/event/event-byeongkwon-recall.md` | **신규** — 30년차 병권 회수 (kind=reform-fail, era-founding) |
| 2 | `projects/chilguk-chunchu/world/event/event-mulim-conference-1st.md` | **신규** — 100년차 1차 무림대회 (kind=convention, era-prosperity) |
| 3 | `projects/chilguk-chunchu/world/event/event-sapa-formation.md` | **신규** — 130년차 사파 3파벌 형성 (kind=schism, era-turning) |
| 4 | `projects/chilguk-chunchu/world/event/event-jachi-movement.md` | **신규** — 160년차 변경 자치 운동 (kind=political-movement, era-turning) |
| 5 | `projects/chilguk-chunchu/world/event/event-cult-remnant-discovery.md` | **신규** — 190년차 화산파 혈교 잔당 발견 (kind=discovery, era-turning) |
| 6 | `projects/chilguk-chunchu/world/era/era-founding.md` | `key_events`에 `event-byeongkwon-recall` 추가 + §핵심 인물에 npc-jincheonmyeong 외래키 활성 + 30년차 인물 4명 텍스트 추가 |
| 7 | `projects/chilguk-chunchu/world/era/era-prosperity.md` | `key_events: []` → `[event-mulim-conference-1st]` + game_role 갱신 |
| 8 | `projects/chilguk-chunchu/world/era/era-turning.md` | `key_events: []` → 3 신규 event 일괄 |
| 9 | `projects/chilguk-chunchu/world/event/event-empire-founding.md` | `related_events`에 `event-byeongkwon-recall` 추가 (역방향) |
| 10 | `projects/chilguk-chunchu/world/event/event-six-states-independence.md` | `related_events`에 `event-jachi-movement` 추가 (역방향) |
| 11 | `projects/chilguk-chunchu/world/event/event-bloody-cult-rebellion-2nd.md` | `related_events`에 `event-cult-remnant-discovery` 추가 (역방향) |
| 12 | `tests/world_chilguk_chunchu_followup_mid_era_events.rs` | **신규** — 14 testcase (라운드트립 + bidirectional + boundary + kind + FK + 회귀 가드) |

### 5 신규 event 일람 (시간순)

| event_id | year | era | kind | participants | related_events forward |
|---|---|---|---|---|---|
| `event-byeongkwon-recall` | -240 | era-founding | **reform-fail** (신규) | groups=[daejin-court, mulim-mang], places=[daejin] | empire-founding(-30) + mulim-conference-1st(+70) |
| `event-mulim-conference-1st` | -170 | era-prosperity | **convention** (신규) | groups=[mulim-mang, gaebang], places=[daejin] | byeongkwon-recall(-70) + sapa-formation(+30) |
| `event-sapa-formation` | -140 | era-turning | **schism** (신규) | groups=[cheonma-shingyo, mulim-mang], places=[daejin] | mulim-conference-1st(-30) + cult-remnant-discovery(+60) |
| `event-jachi-movement` | -110 | era-turning | **political-movement** (신규) | places=[donghae, namman, seoryang, daejin], groups=[] | six-states-independence(+103) |
| `event-cult-remnant-discovery` | -80 | era-turning | **discovery** (신규) | groups=[mulim-mang, daejin-court], places=[daejin] | sapa-formation(-60) + bloody-cult-rebellion-2nd(+50) |

**5 event 모두 participants.people = [] (Q3 정책 — 본 사건 인물 전부 5c.1 미등록)**.

### 신규 kind 5종 (Q1 정책 정합)

| kind | 의미 | event |
|---|---|---|
| `reform-fail` | 실패한 개혁 — 황실 시도가 도리어 분기점이 된 경우 | byeongkwon-recall |
| `convention` | 합의의 정형 — 무림 합의 구조의 정형화 | mulim-conference-1st |
| `schism` | 분리 — 단일 단위가 둘로 갈라지는 분기 | sapa-formation |
| `political-movement` | 정치적 흐름(장기) — 한 시점이 아닌 다년 운동 | jachi-movement |
| `discovery` | 발견 — 후속 사건의 인과 시작점 | cult-remnant-discovery |

`founding`은 재사용 (taemuje-enthronement, 체크포인트 1 결정). Phase 5a `war`/`betrayal`/
`disaster`/`founding`과 합쳐 Event.kind 카탈로그가 Phase 5c.2 종결 시점 9종.

### era 4종 key_events 슬롯 채움

| era | start | end | key_events | 변화 |
|---|---|---|---|---|
| era-founding | -270 | -220 | [empire-founding, **byeongkwon-recall**] | 1 → 2 |
| era-prosperity | -220 | -150 | [**mulim-conference-1st**] | 0 → 1 |
| era-turning | -150 | -70 | [**sapa-formation, jachi-movement, cult-remnant-discovery**] | 0 → 3 |
| era-decline | -70 | -30 | [taemuje-enthronement] (체크포인트 1 채움) | 0 → 1 |
| era-fall-of-empire | -30 | 0 | [bloody-cult-rebellion-2nd, blood-disappearance, bloody-night, hwasan-fall, six-states-independence] | 5 (Phase 5b 그대로) |

**전체 12 event 모두 era 1개에 정형 매칭** (boundary 정책 §3.2 — start inclusive · end exclusive).

### related_events 양방향 정합 (task §3.5 인과 사슬)

체크포인트 2에서 추가된 forward + reverse 매트릭스 (체크포인트 1 enthronement 3건은 별도):

```
empire-founding(-270) ↔ byeongkwon-recall(-240)         (체크포인트 2 신설)
byeongkwon-recall(-240) ↔ mulim-conference-1st(-170)    (체크포인트 2 신설, 양방향 자체 신규)
mulim-conference-1st(-170) ↔ sapa-formation(-140)       (체크포인트 2 신설, 양방향 자체 신규)
sapa-formation(-140) ↔ cult-remnant-discovery(-80)      (체크포인트 2 신설, 양방향 자체 신규)
cult-remnant-discovery(-80) ↔ bloody-cult-rebellion-2nd(-30)  (체크포인트 2 신설)
jachi-movement(-110) ↔ six-states-independence(-7)      (체크포인트 2 신설)
empire-founding(-270) ↔ bloody-cult-rebellion-2nd(-30)  (Phase 5a 기존)
taemuje-enthronement(-33) ↔ bloody-cult-rebellion-2nd(-30) (체크포인트 1)
taemuje-enthronement(-33) ↔ blood-disappearance(-12)    (체크포인트 1)
taemuje-enthronement(-33) ↔ bloody-night(-10)           (체크포인트 1)
blood-disappearance(-12) ↔ bloody-cult-rebellion-2nd(-30) (Phase 5a 기존)
blood-disappearance(-12) ↔ bloody-night(-10)            (Phase 5a 기존)
blood-disappearance(-12) ↔ hwasan-fall(-10)             (Phase 5a 기존)
bloody-night(-10) ↔ hwasan-fall(-10)                    (Phase 5a 기존)
bloody-night(-10) ↔ six-states-independence(-7)         (Phase 5a 기존)
```

**총 15개 양방향 인과 사슬 — e2e 테스트 `related_events_bidirectional_integrity`로 자동 검증** (BIDIRECTIONAL_LINKS 11개는 task §3.5 명시 + Phase 5a 기존 일부; 나머지는 Phase 5a/5b 기존 link로 별도 관리).

### Q2·Q3 결정 결과 (디렉터 통과 신호 채택)

- **Q2**: `event-byeongkwon-recall` 슬러그 한자음+영문 혼용 채택. `military-rights-recall` 단독은 한자어 고유성 손실이라 선택 안 함.
- **Q3**: `event-mulim-conference-1st.participants.people = []` 채택. 혜통 대사·적하검·일소옹 모두 5c.1 미등록 + 본 follow-up 스코프 외라 산문 §핵심 인물에 텍스트로만 명시. 같은 정책을 sapa-formation·jachi-movement·cult-remnant-discovery에도 일관 적용.

---

## Diff

```text
+ projects/chilguk-chunchu/world/event/event-byeongkwon-recall.md       (신규, ~140줄)
+ projects/chilguk-chunchu/world/event/event-mulim-conference-1st.md    (신규, ~140줄)
+ projects/chilguk-chunchu/world/event/event-sapa-formation.md          (신규, ~150줄)
+ projects/chilguk-chunchu/world/event/event-jachi-movement.md          (신규, ~150줄)
+ projects/chilguk-chunchu/world/event/event-cult-remnant-discovery.md  (신규, ~150줄)
M projects/chilguk-chunchu/world/era/era-founding.md         (key_events +1, 핵심 인물 +5 / 1 활성)
M projects/chilguk-chunchu/world/era/era-prosperity.md       (key_events 0→1, game_role 갱신)
M projects/chilguk-chunchu/world/era/era-turning.md          (key_events 0→3)
M projects/chilguk-chunchu/world/event/event-empire-founding.md         (related_events +1)
M projects/chilguk-chunchu/world/event/event-six-states-independence.md (related_events +1)
M projects/chilguk-chunchu/world/event/event-bloody-cult-rebellion-2nd.md (related_events +1)
+ tests/world_chilguk_chunchu_followup_mid_era_events.rs    (신규, 14 testcase)
```

총 12 파일 (6 신규 + 6 수정). 신규 라인 ~870줄.

---

## 데모 명령

```bash
# 1. world-load — events 7 → 12, fk errors=0 확인
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload

# 결과 (실측):
#   events indexed = 12  (← 7에서 +5)
#   eras indexed   = 5
#   fk errors (활성) = 0
#   group cycles   = 0
#   place cycles   = 0
#   mind eligible  = 12

# 2. 신규 e2e 테스트 — 14 cases
cargo test --features embed --test world_chilguk_chunchu_followup_mid_era_events
# 결과: 14 passed; 0 failed; 0 ignored

# 3. 기존 world 테스트 회귀
cargo test --features embed --test world_chilguk_chunchu_e2e            # 14 passed
cargo test --features embed --test world_chilguk_chunchu_phase5c_e2e    # 20 passed
cargo test --features embed --test world_load_fk_negative_event         # 7 passed

# 4. event 검색 정성 평가 — sqlite로 5 신규 event 직접 조회
sqlite3 projects/chilguk-chunchu/build/world.sqlite \
  "SELECT id, kind, year_relative FROM events WHERE year_relative BETWEEN -240 AND -80 ORDER BY year_relative;"
# 출력 (5 신규):
# event-byeongkwon-recall|reform-fail|-240
# event-mulim-conference-1st|convention|-170
# event-sapa-formation|schism|-140
# event-jachi-movement|political-movement|-110
# event-cult-remnant-discovery|discovery|-80

# 5. era key_events 일괄 검증
sqlite3 projects/chilguk-chunchu/build/world.sqlite \
  "SELECT era_id, key, ordinal FROM era_key_events ORDER BY era_id, ordinal;"
# 출력:
# era-decline|event-taemuje-enthronement|0
# era-fall-of-empire|event-bloody-cult-rebellion-2nd|0
# ... (Phase 5b 5건)
# era-founding|event-empire-founding|0
# era-founding|event-byeongkwon-recall|1
# era-prosperity|event-mulim-conference-1st|0
# era-turning|event-sapa-formation|0
# era-turning|event-jachi-movement|1
# era-turning|event-cult-remnant-discovery|2
```

---

## 결정

### Q1 (체크포인트 1 통과 + 신규 kind 5종 적용)

체크포인트 1에서 founding 재사용이 결정됐고, Q1 정책 "Event.kind는 string 자유 추가"가 본 체크포인트 2의 신규 kind 5종 도입을 그대로 정당화. 카탈로그 비대화 회피 + 의미 분리의 균형을 디렉터 권장 그대로 채택.

신규 5 kind는 모두 founding(권력 단위 출범)·war(군사 충돌)·betrayal(배신)·disaster(재해)·discovery(발견)·ritual(의례) Phase 5a 카탈로그에 환원되지 않는 분리 의미를 가짐:
- **reform-fail** vs founding/political: 권력 시도의 실패가 본질 (founding은 성공한 출범, political은 단일 사건)
- **convention** vs founding: 합의 구조의 정형 (founding은 단위 출범)
- **schism** vs betrayal: 분리(단위 분기) vs 배신(개인적 배신)
- **political-movement** vs political: 장기 운동(다년) vs 단일 정치 사건
- **discovery** (Phase 5a 미사용 — 카탈로그에 명목상 있으나 시드 미사용): 본 사건이 첫 사용

Phase 5c.2 종결 시점 Event.kind 카탈로그 (실 사용 9종):
```
founding · war · betrayal · disaster · convention · schism ·
political-movement · reform-fail · discovery
```

### Q2 (디렉터 권장 채택)

- `event-byeongkwon-recall`: 한국어-한자음(byeongkwon) + 영문 의미어(recall) 혼용 유지. task §6.1 정합.

### Q3 (디렉터 권장 채택)

- 5 신규 event 모두 `participants.people = []`. 미등록 인물(혜통 대사·적하검·일소옹·진해·아골타·곽천풍·진여·각원 대사·수월진인·진무양·벽운자·태허진인·혜안 대사·정심 대사·초대 천마·녹림왕·독고선·진태광 18인)은 산문 §핵심 인물에 텍스트로 보존.
- Phase 5a D1 정책 그대로. 추가 historical npc 등록 follow-up은 Phase 6+ 또는 별도 task.

### Q4 (체크포인트 1 통과 적용)

체크포인트 1에서 npc-02(조고)·group-shipsangsi 미포함 결정이 통과됐고 본 체크포인트 2에 영향 없음. 5 신규 event 모두 사료 정합 우선의 같은 원칙으로 작성됨.

### 추가 결정 (체크포인트 2 신설)

- **event-jachi-movement.participants.groups = []**: 동해 상방·남만 부족·서량 군벌은 모두 group 미등록 + 자치 운동 1세대 group은 형성 전 단계. group 정형 자체가 부적합 → 비움. participants.places 4개로 흐름의 광역 정형.
- **event-cult-remnant-discovery에 npc-chuyangjinin 직접 미포함**: 본 사건은 진여(1세대) 발견이며 추양진인은 80년 후 5세대 계승자. participants 미포함, 산문 §핵심 인물에 80년 정보 라인 종착자로 명시. 화산파 group 미등록이라 mulim-mang으로 추상화.
- **event-empire-founding 역방향 갱신 시 byeongkwon-recall 1건만**: empire-founding의 related_events는 본 follow-up 종결 시점 [bloody-cult-rebellion-2nd, byeongkwon-recall] 2건. taemuje-enthronement과 직접 연결은 240년 거리상 약함 (간접 인과는 산문에).

---

## 막힌 것

**없음.** 체크포인트 1·2 모두 디렉터 권장값 정합 완료.

다만 본 follow-up 종결 후 검토 항목 (Phase 5 시리즈 종결 검토 시 디렉터 결정):
- **event-cult-remnant-discovery에 group-cheonma-shingyo 추가 검토**: 천마신교의 혈교 무공 연구 라인이 본 사건과 은밀한 정보 교환 가능성(history-characters §7.2)이라 group 추가도 가능. 현재는 산문에만 명시. 디렉터가 명시적 외래키 활성을 선호하면 추가 — 현재 미포함은 history-characters의 "가능성"이라는 확정 미흡.
- **event-jachi-movement → bloody-night 인과 사슬 부재**: 본 사건이 110년 누적 정치 운동인데 직접 종착이 six-states-independence(-7) 1건만 forward로 명시. 263-265년차 분열의 토양이지만 본 사건 → bloody-night(-10) 직접 인과는 약함이라 미포함. 디렉터 검토 가능.
- **e2e 테스트 BIDIRECTIONAL_LINKS 11개는 task §3.5 명시 + Phase 5a 기존 일부 — 나머지 (blood-disappearance ↔ hwasan-fall · bloody-night ↔ hwasan-fall 등)는 Phase 5a 기존 link로 별도**: 회귀 가드는 본 follow-up 신설 + 체크포인트 1 sliced된 부분에 한정. Phase 5a 6 event 간 양방향은 Phase 5a 보고서 정합 (별도 작업).

---

## 검증 결과

### world-load (events 7 → 12, fk=0)
```
project           = chilguk-chunchu
groups indexed    = 6
persons indexed   = 16
places indexed    = 11
atlases indexed   = 1
events indexed    = 12   (← 7에서 +5)
eras indexed      = 5
errors            = 0
group cycles      = 0
place cycles      = 0
fk errors (활성)  = 0
mind eligible     = 12
```

### cargo test --features embed (worldbuilding 전수)

신규 e2e + 13 기존 world test binary = **14 binaries 합쳐 135 cases · 0 failed · 1 ignored**:

| binary | 결과 |
|---|---|
| **world_chilguk_chunchu_followup_mid_era_events** (신규) | **14 passed** |
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

### 14 신규 testcase 검증 항목

1. `six_mid_era_events_parse_with_expected_meta` — 6 event id/kind/year_relative/era_id 정합 + summary/body 비어있지 않음
2. `five_new_kinds_introduced_per_q1_policy` — 신규 5 kind 도입 + founding 재사용 1건 검증
3. `era_boundary_consistency` — 6 event year_relative ∈ era boundary 안쪽 (start inclusive · end exclusive)
4. `era_key_events_slots_filled` — era 4종 key_events 순서 정확히 일치
5. `related_events_bidirectional_integrity` — 11 양방향 인과 forward + reverse 모두 자동 검증
6. `related_events_targets_all_resolve` — 12 event 전체 + 모든 related_events 타겟 해소 (events=12 검증 포함)
7. `fk_activations_for_5c1_npcs` — taemuje-enthronement.npc-danun 활성 + Q4 (조고/십상시 미포함) + Q3 (5 신규 participants.people 비움)
8. `cult_remnant_discovery_chains_to_taemuje_via_bloody_cult` — 인과 사슬 매개 정합 (직접 link 아닌 간접 link)
9. `jachi_movement_directly_chains_to_six_states` — 110년 누적 직접 인과 + 4 places 정합
10. `taemuje_enthronement_summary_corrected` — PR #76 review Finding #1 회귀 가드 (30년 후 → 23년 후)
11. `event_id_namespace_consistency` — 6 event id 명명 규칙 (event-{slug}, ASCII)
12. `participants_groups_use_only_registered_groups` — 등록된 6 group 외 참조 0
13. `participants_places_use_only_registered_places` — 등록된 11 place 외 참조 0
14. `event_id_roundtrip_preserved_for_all_events` — EventId String 라운드트립

### 환경 의존 실패 (본 follow-up 무관)

ONNX 모델(`../models/bge-m3/`) 미배치로 12개 PAD analyzer 의존 테스트 실패 (체크포인트 1과 동일). 본 follow-up은 데이터 + 테스트 추가만이라 무관.

---

## 다음 의견

**Phase 5 시리즈 종결 검토 권장 항목**:

### 1. Phase 5 시리즈 종결 가능 (5a + 5b + 5c.1 + 5c.2 모두 완료)

- **Phase 5a** (Event 도메인 + 6 사건 시드): ✅
- **Phase 5b** (Era 도메인 + 5 시대 + Timeline): ✅
- **Phase 5c.1** (Historical NPCs follow-up — 4 historical + 4 stub→정식 등록): ✅
- **Phase 5c.2** (Mid-era Events follow-up — 본 follow-up): ✅ 체크포인트 1·2 통과

전체 12 event · 5 era · 16 person · 6 group · 11 place · 1 atlas · 1 timeline 정형 완료.

### 2. Phase 6+ 진입 가능 (4 도메인 후보)

task §7 Out of Scope의 Phase 6+ 도메인:
- **Skill** — 무공·기술·기예 (HEXACO·Place·Person 외래키 후보)
- **Item** — 무기·보물·기록 단편 (혈매화검·문서 단편 등)
- **Knowledge** — 정적 세계관 지식 (270년 약속·270년 전통)
- **Lore** — 전설·구전·소문 (이미 lore.sqlite Phase 0 인프라 있음)

본 follow-up 종결 후 Phase 6 도메인 선택은 디렉터 결정. Roadmap [`docs/tasks/00-roadmap.md`](00-roadmap.md) 참조.

### 3. 추가 historical npc 등록 follow-up (선택)

5 mid-era event 산문에 텍스트로만 명시된 18인이 Phase 6+ 또는 별도 follow-up 후보:
- **건국기**: 진태광(3대 태광제), 진무양(화산), 벽운자(청성), 태허진인(무당), 혜안(소림)
- **전성기**: 혜통(소림), 적하검(화산), 일소옹(개방), 진승(진씨 상방 시조), 융성제(7대), 육해천(해남)
- **변곡기**: 초대 천마, 정심(소림), 녹림왕, 독고선, 진해(진씨 상방 5대), 아골타(남만), 곽천풍(서량), 진여·수월진인·각원(혈교 잔당 발견)

위 18인 중 진해(진대인 고조부)·아골타(남만 왕가 시조)·곽천풍(npc-04 당무괴 시조)·초대 천마(3대 천마 시조) 4인이 현역 NPC 정체성의 직계 시조라 우선 등록 후보.

### 4. 이번 follow-up 데이터 변경의 캐논 안정성

5 event 산문 + frontmatter는 history.md·history-characters.md v1.2 사료 직접 인용 + npc-02-jogo.md §상세 연표 정합 검토 결과를 반영. 후일 사료 갱신(v1.3+) 시 본 follow-up 데이터도 정합 검토 필요할 수 있으나 현재 시점에는 사료 캐논 그대로.

### 5. e2e 테스트 회귀 가드의 미래 확장성

`tests/world_chilguk_chunchu_followup_mid_era_events.rs`의 14 cases는 본 follow-up + Phase 5c.2 체크포인트 1 + Phase 5a 일부의 양방향 정합을 자동 검증. Phase 6+ event 추가 시 BIDIRECTIONAL_LINKS·MID_ERA_EVENTS·ERA_KEY_EVENTS_EXPECTED 상수만 갱신하면 동일 패턴으로 회귀 가드 확장 가능.

**Phase 5 시리즈 종결 디렉터 통과 신호 받기 전 commit pause 유지**. 통과 시 Phase 5 시리즈 전체 종결 + Phase 6+ 진입 검토.
