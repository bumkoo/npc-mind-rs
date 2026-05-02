# Phase 5c.1 체크포인트 1 보고서 — 임서운 단독 변환 + 임서운 등장 Event 외래키 활성

> **상태**: ✅ 체크포인트 1 통과 — 디렉터 리뷰 대기. **commit pause 유지**.
> **작업 브랜치**: `claude/historical-npcs-phase5c-oR0fL`
> **사양**: `docs/tasks/task-phase5-followup-historical-npcs.md`
> **작성일**: 2026-05-02

## Done

- [x] `docs/tasks/task-phase5-followup-historical-npcs.md` (사양 문서, ~250줄, 9 섹션)
- [x] `projects/chilguk-chunchu/world/person/npc-im-seoun.md` (임서운 단독 변환, kind=historical, status=missing)
  - HEXACO 6 dim 정밀 매핑 (정파·검학·player 보호자·10년 전 행방불명)
  - `extras.priority: "★★★★★"` (player 메인 비밀의 핵심)
  - `extras.secret`: 4건 명시 (추양진인 수제자 오인식 / 혈매화검 진짜 주인 / 생존 가능성 / 12년 전 위탁 경위)
  - affiliation: empty + `extras.pending_groups: [group-hwasan-pa]` (npc-04 패턴)
  - `extras.player_relevance: 5`, 7 H2 섹션 (개요·배경·동기·비밀·HEXACO 분석·관계·게임에서의 역할)
- [x] **3 Event participants.people 외래키 활성** (임서운 명시 등장):
  - `event-bloody-night.md` — `npc-im-seoun` 추가 (셋째 밤 player 도주)
  - `event-hwasan-fall.md` — `npc-im-seoun` 추가 (추양진인 수제자, 마지막 밤 player 도주)
  - `event-blood-disappearance.md` — `npc-im-seoun` 추가 (12년 전 player 위탁 시작)
- [x] **산문 정정** — `(npc 미등록) 임서운: player 부친` → `npc-im-seoun 임서운: player 보호자(친부 아님)` (3 Event)
  - player.md §배경 + 캐릭터 시트 v1.2 §2 기준 사실 정합 (player 친부모 미상, 임서운은 5세 위탁 양육자).
- [x] **director_decisions Phase 5c.1 절 추가** — 3 Event 모두 변환 결정 추적성 확보
- [x] `cargo build --features embed` 통과
- [x] `cargo test --features embed --lib` → 560 passed (회귀 0건)
- [x] world-load Phase 1·2·3·4·5a·5b·5c.1 통합 ingest 통과
  - `persons indexed = 10` (Phase 5b 9 + npc-im-seoun)
  - `mind eligible = 9` (변동 없음 — historical kind는 mind upsert 제외, 정책 정합)
  - `fk errors (활성) = 0`
- [x] Phase 1·2·3·4 e2e 회귀 (mind eligible 8 + Phase 4 13 + Phase 5b 호환) 통과
- [x] world-load FK negative e2e 통과 (event 7 + era 5 + timeline 4 + base 5)

## Diff (Phase 5c.1 체크포인트 1 누적)

```
 docs/tasks/phase5-followup-historical-npcs-checkpoint1-report.md       (신규, ~본 문서)
 docs/tasks/task-phase5-followup-historical-npcs.md                     (신규, ~250줄, 사양)
 projects/chilguk-chunchu/world/person/npc-im-seoun.md                  (신규, ~150줄, 임서운 단독)
 projects/chilguk-chunchu/world/event/event-blood-disappearance.md      (3 hunks: participants + director_decisions + 핵심 인물 + 게임 역할)
 projects/chilguk-chunchu/world/event/event-bloody-night.md             (3 hunks: participants + director_decisions + 결과 + 핵심 인물)
 projects/chilguk-chunchu/world/event/event-hwasan-fall.md              (2 hunks: participants + director_decisions + 핵심 인물)
```

src 코드 변경 0줄 — Phase 2 Person 도메인 + Phase 5a Event 도메인 + Phase 5b world-load FK 검증
인프라가 그대로 작동.

## 데모 명령

```bash
# 빌드 + 테스트
cargo build --features embed
cargo test --features embed --lib                                    # 560 passed (회귀 0건)
cargo test --features embed --test world_chilguk_chunchu_e2e         # Phase 1 회귀
cargo test --features embed --test world_chilguk_chunchu_persons_batch_e2e   # Phase 2 회귀 (12 passed)
cargo test --features embed --test world_chilguk_chunchu_player_e2e          # Phase 2.1 회귀 (7 passed)
cargo test --features embed --test world_chilguk_chunchu_phase4_checkpoint2  # Phase 4 회귀
cargo test --features embed --test world_load_fk_negative_event              # Phase 5a 회귀

# Ingest
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload --db /tmp/world-phase5c.sqlite
```

## 결과

```
=== 결과 ===
project           = chilguk-chunchu
groups indexed    = 6
persons indexed   = 10                  ← 9 → 10 (+npc-im-seoun)
places indexed    = 11
atlases indexed   = 1
events indexed    = 6
eras indexed      = 5
timelines indexed = 1
groups parsed     = 6
persons parsed    = 10                  ← 9 → 10
places parsed     = 11
atlases parsed    = 1
events parsed     = 6
eras parsed       = 5
timelines parsed  = 1
errors            = 0
group cycles      = 0
place cycles      = 0
fk errors (활성)  = 0                   ← 새 외래키 추가에도 결손 0
mind eligible     = 9                   ← 변동 없음 (historical kind는 mind 제외, 정책 정합)
```

## 임서운(林書雲) 핵심 결정 (체크포인트 1 입력 처리)

### 1. ID 결정 — `npc-im-seoun`

| 옵션 | 결과 |
|---|---|
| `npc-im-seoun` (descriptive Romanized, npc- prefix) | ✅ 채택 |
| `npc-h29` (character-roster H29 코드) | 미채택 (npc-XX 숫자 ID는 active N1-N11 1:1 매핑 보존) |
| `H29` (test 패턴) | 미채택 (project 일관성 npc- prefix 유지) |

**근거**: 11 active/heritage-pending Person이 `npc-{01..11}`로 character-roster N1-N11과 1:1
매핑되어 있어, historical Person에 숫자 ID 사용 시 이름공간 충돌. `npc-im-seoun`은
descriptive Romanized로 검색·디버깅 친숙. 향후 historical 7-11명도 같은 패턴
(`npc-chuyangjinin`, `npc-jincheonmyeong`, `npc-danun` 등) — 사양 §3.2 명시.

### 2. kind / status — `historical` / `missing`

| 필드 | 값 | 근거 |
|---|---|---|
| `kind` | `historical` | 게임 시작 시점(270년차) 행방불명 10년 — 직접 대화 불가, mind 등록 X |
| `status` | `missing` | character-roster H29 "👻 행방불명" + history-characters §11.1 "행방불명 (생사 불명)" + player.md §비밀 ④ "생존 가능성" 명시 |
| `is_mind_eligible()` | `false` | `kind=historical` → mind upsert 제외 (Phase 2 정책 정합) |

`PersonStatus::Missing` 사용 — `Dead`로 강제하지 않음. history-characters H29 "생사 불명"
표기 + 메인 퀘스트 후반 "생존 가능성 분기"가 사양상 보존되어야 함.

### 3. affiliation — empty + pending_groups

```yaml
affiliation: []
extras:
  pending_groups: [group-hwasan-pa]
```

**근거 (npc-04 패턴 미러)**:
- 화산파(group-hwasan-pa)는 Phase 1 시드 6 group(daejin-court·shipsangsi·namgung·mulim-mang·
  cheonma-shingyo·gaebang)에 미등록.
- group-mulim-mang proxy 채택 검토했으나 **불채택** — 화산파는 무림맹 alliance 멤버 문파였으나
  개별 화산 제자가 무림맹 직접 멤버로 등록되지 않은 게 적절. 제자→문파→연합 hierarchy
  보존이 사양상 더 정확.
- npc-04 당무괴(서량/당가 그룹 미등록 → empty + pending_groups)에서 정착한 패턴 그대로 적용.
- Phase 6+ "historical/legendary group" 카테고리에서 group-hwasan-pa 등록 시 affiliation 배열
  로 승격 (별도 follow-up).

### 4. HEXACO 매핑 — 정밀 등급 (Phase 2 npc-06 패턴)

다중 출처 일관 묘사로 매핑 신뢰도는 정밀 등급:

| 출처 | 명시 |
|---|---|
| character-roster.md H29 | "화산파 수제자, 플레이어(7세) 구출자, 기록자, 행방불명" |
| history-characters.md §11.1 | "추양진인 수제자, '구름에 글을 쓰다' — 기록을 남기고 사라짐" |
| history-characters.md §13 | 260년차 핵심 인물 표 명시 |
| player.md (Phase 2.1) | §배경 + §비밀 ①·②·④ + §관계에 임서운 4건 명시 |
| 칠국춘추_플레이어_캐릭터_시트.md v1.2 | §2 "설정 의도(플레이어 오인식)" + §관계망 + §가치관 |
| npc-02-jogo.md / npc-03-namgunghyuk.md / npc-11-taemuje.md | 각자 회상에서 임서운 도주 묘사 |

| dim | 값 | 핵심 근거 |
|---|---|---|
| H (정직-겸손) | **+0.7** | 화산파 정파 정직성 + 추양진인 수제자 신분을 player에게 숨기고 "말단 제자"로 자처 (modesty 매우 양수) |
| E (정서성) | **+0.5** | player 보호 자기희생 + 행방불명 결단의 무게 (sentimentality 양수) |
| X (외향성) | **-0.2** | 검학 + 기록자(書雲) + 사문 일에 집중, 사교 적음 |
| A (원만성) | **+0.5** | 정파 의리 + player 무조건 보호 + 추양진인 충실 |
| C (성실성) | **+0.7** | 화산파 수제자 = 검학 정점 + 270년 전통 보존자 |
| O (개방성) | **+0.4** | 검학 + 기록·문서화 + 비공식 player 양육 결단 (unconventionality) |

**중요 — `extras.source_status: heritage-pending` 유지**: 매핑 신뢰도는 정밀 등급(다중 출처
일관) 이지만 직접 열전(`character/npc-29-imseoun.md`) 부재라 source_status 플래그는
heritage-pending 보존. Phase 6+ 단독 .md 작성 시 재검토 가드.

가치관 5축 (player 캐릭터 시트의 임서운 묘사 측면 그대로 채택):
- chung 0.7 / eui 0.8 (게임 내 최고 등급) / hyo 0.6 / bok 0.4 / yah 0.2

### 5. extras.secret — 4건 명시

player 메인 비밀과의 1:1 매핑:

| 비밀 | 출처 |
|---|---|
| 1. 추양진인 수제자 — player "말단 제자" 오인식 | player 캐릭터 시트 v1.2 §2 |
| 2. 혈매화검 진짜 주인 — 화산파 보물 아닌 본인 검 | player.md §비밀 ① |
| 3. 생존 가능성 — history-characters H29 "행방불명·생사 불명" | player.md §비밀 ④ |
| 4. 12년 전 위탁 경위 — 우연 vs 사전 결정 | 캐릭터 시트 v1.2 §2 |

`extras.secret` 필드는 본 Person이 처음 도입. 향후 historical NPC 변환 시 동일 패턴 — 산문
`## 비밀` 섹션과 frontmatter `extras.secret` 양쪽 보존 (검색·SQL 필터·산문 가독성 모두 충족).

### 6. Event 외래키 활성 결정 — 3건 + 1 boundary 보고

사양 §5.1.3 + 본 절차:

| Event | year_relative | participants.people 추가 | 산문 등장 | 결정 |
|---|---|---|---|---|
| `event-blood-disappearance` | -12 | ✅ `npc-im-seoun` | ✅ player.md + 캐릭터 시트 v1.2 명시 | 추가 |
| `event-bloody-night` | -10 | ✅ `npc-im-seoun` | ✅ 셋째 밤 player 도주 명시 | 추가 |
| `event-hwasan-fall` | -10 | ✅ `npc-im-seoun` | ✅ 추양진인 수제자, 마지막 밤 player 도주 명시 | 추가 |
| `event-bloody-cult-rebellion-2nd` | -30 | ❌ (체크포인트 1 보류) | 산문에 직접 등장 X. 추양진인 등장만 명시 | **boundary 보고** |

**boundary 케이스 — `event-bloody-cult-rebellion-2nd` 결정**:
- 산문에 임서운이 직접 명시되지 않음 (history-characters §13에 240년차 핵심 인물로 단운·명경·
  풍만리·설무한·2대 천마만 등재. 임서운 없음).
- 추양진인은 240년차 화산 장문인으로 등장하나 임서운은 그의 수제자라는 사실 외 240년차 시점
  활동 명시 부재. 임서운이 240년차에 화산 일반 제자였을 가능성은 합리적이나 출처 보수성 유지
  필요.
- **체크포인트 1에선 추가 보류**. 사양 §3.5 "임의 추가 금지 — 출처 보수성 유지" 정책 적용.
- 체크포인트 2 추양진인 등록 시 함께 결정: (a) 추양진인만 추가 (b) 추양진인 + 임서운 동시 추가
  (산문에 "추양진인의 수제자로 임서운이 동행" 명시 후) — 디렉터 결정 후보.

**디렉터 사용자 발화의 "4건"**: 사용자가 "Phase 5a Event 외래키 갱신 4건 (bloody-night·
hwasan-fall·blood-disappearance·bloody-cult-rebellion-2nd 중 임서운 등장)"이라 명시. 본
체크포인트는 출처 보수성으로 3건 + 1 boundary로 처리 — 디렉터 검토 시 (a) 본 결정 유지
또는 (b) 4번째 추가(추양진인 함께) 둘 중 결정 가능.

### 7. 산문 정정 — "player 부친" → "player 보호자"

3 Event 산문에서 임서운을 "player 부친"으로 표기한 부분이 player.md §배경과 모순:

> 5세(12년 전) 피의 실종 사건 납치 피해아. 개방 구출 직후 화산파 임서운에게 위탁.

player의 친부모는 미상이며 임서운은 5세 위탁 후 비공식 양육자(보호자). 캐릭터 시트 v1.2 §2의
"설정 의도"에 따르면 임서운은 player 시점에선 "그 사람"으로 기억되며 부친 아님. 본 정정으로
Phase 5a 산문의 사실 오류 1건 해소.

| Event | before | after |
|---|---|---|
| `event-bloody-night.md` 결과·핵심 인물 | "player의 부친 임서운" | "player 보호자 임서운" |
| `event-hwasan-fall.md` 핵심 인물 | "player 부친" | "player 보호자(친부 아님)" |
| `event-blood-disappearance.md` 게임 역할 | "player의 부친 임서운" | "player 보호자 임서운" |

## mind eligible 변화 검증 — 9 유지

| 시점 | mind eligible | 비고 |
|---|---|---|
| Phase 5b 종결 | 9 | npc-01·02·03·04·05·06·07·11 + player |
| **Phase 5c.1 체크포인트 1** | **9** | npc-im-seoun = `kind=historical` → mind 미등록 (정책 정합) |
| Phase 5c.1 체크포인트 2 (예상) | 13 | npc-08·09·10·11(stub 승급) 정식 변환 시점 |

회귀 가드: `cargo run --bin world-load --features embed -- --project chilguk-chunchu --reload`
출력의 `mind eligible = 9`. `is_mind_eligible_only_active_or_player` 단위 테스트는 변경 없이
historical 케이스 false 반환 검증 통과.

## 7 H2 섹션 — npc-06/07 패턴 미러

`npc-im-seoun.md`는 npc-06(정밀)의 7 H2 + npc-07(heritage-pending)의 source_status 마커
조합:

- `## 개요` — 신원·직위·별호·역사적 위치 한 단락
- `## 배경` — 출생 추정 → 240년차 boundary 명시 → 258년차 player 위탁 → 260년차 멸문 마지막 밤
- `## 동기` — 표층(화산파 270년 전통 보존)·심층(신뢰받은 자의 의리)·두려움 3가지
- `## 비밀` — 4건 (추양진인 수제자 / 혈매화검 진짜 주인 / 생존 가능성 / 12년 전 위탁 경위)
- `## HEXACO 분석` — 6 dim 각자 근거 + heritage-pending 가드 명시
- `## 관계` — affiliation 빈 사유 + 추양진인·player·npc-02·npc-03·npc-01·개방 거지 6노드
- `## 게임에서의 역할` — 메인 퀘스트 3 단계 (초반/중반/후반) + Phase 6+ Memory 시스템 통합 후보

## 회귀 가드 결과 요약

| 검증 | 결과 |
|---|---|
| `cargo build --bin world-load --features embed` | ✅ |
| `cargo test --features embed --lib` | ✅ 560 passed |
| `cargo test --features embed --test world_chilguk_chunchu_e2e` | ✅ |
| `cargo test --features embed --test world_chilguk_chunchu_person_e2e` | ✅ 11 passed |
| `cargo test --features embed --test world_chilguk_chunchu_persons_batch_e2e` | ✅ 12 passed |
| `cargo test --features embed --test world_chilguk_chunchu_player_e2e` | ✅ 7 passed |
| `cargo test --features embed --test world_chilguk_chunchu_phase3_checkpoint{1,2}` | ✅ 통과 |
| `cargo test --features embed --test world_chilguk_chunchu_phase4_checkpoint{1,2}` | ✅ 13 + 9 passed |
| `cargo test --features embed --test world_load_fk_negative` | ✅ 5 passed |
| `cargo test --features embed --test world_load_fk_negative_event` | ✅ 7 passed |
| `cargo test --features embed --test world_load_fk_negative_era` | ✅ 5 passed |
| `cargo test --features embed --test world_load_fk_negative_timeline` | ✅ 4 passed |
| world-load Phase 1·2·3·4·5a·5b·5c.1 통합 ingest | ✅ persons=10, mind eligible=9, fk errors=0 |
| Phase 5a 6 Event 외래키 매트릭스 (3 활성 + 3 미변경) | ✅ |

**환경 의존성 사유 미회귀**: `tests/embed_test.rs` 6 PAD 분석 테스트는 ONNX 모델
(`../models/bge-m3/`) 부재로 실패 — 본 Phase 5c.1 변경과 무관 (사전 환경 이슈, Phase 5a
Pad 벤치 등도 같은 이유로 환경 의존).

## 막힌 것

없음 — 사양 §6 핵심 결정 6건 모두 명확하게 처리:
- ID, kind/status, affiliation, HEXACO 등급, Event 외래키 범위, 산문 정정 모두 결정·적용

**디렉터 검토 후보** (정보용):
- **boundary 케이스 추가 결정**: `event-bloody-cult-rebellion-2nd`에 임서운을 추가할지 보류할지.
  체크포인트 2 추양진인 등록 시 (a) 추양진인만 (b) 추양진인 + 임서운 동시 — 둘 중 디렉터
  결정 후보. 본 체크포인트는 출처 보수성으로 보류.
- **사용자 발화 "4건"**: 본 체크포인트는 명시 등장 3건 + 1 boundary로 처리. 디렉터 의도가
  "추양진인 수제자라는 정황상 임서운도 240년차 활동" 추정이라면 체크포인트 2에서 함께
  해소(추양진인 + 임서운 동시 추가). 디렉터 명시 4건 유지 의향이면 본 체크포인트에서 1건
  추가 가능 — 별도 commit으로 분리.

## 다음 의견 — 체크포인트 2 진행 가능

체크포인트 1 사양 충족. 디렉터 통과 시 체크포인트 2 진입:

### 체크포인트 2 작업 범위 (사양 §5.2 + 사용자 결정 후보)

**권장 7건** (사양 §3.3 + §5.2):
- 필수 4 (heritage-pending 정밀 승급): npc-08 바투 / npc-09 진대인 / npc-10 3대 천마 / npc-11 소풍자(stub 승급)
- 핵심 historical 3 (heritage-pending): npc-chuyangjinin 추양진인 / npc-jincheonmyeong 진천명 / npc-danun 단운(태무제)

**선택 추가 4** (디렉터 결정):
- npc-pungmanri 풍만리 / npc-seolmuhan 설무한 / npc-jayangjinin 자양진인 / npc-cheonrian 천리안

### 체크포인트 2 검증 게이트 (예상)

- 7-11 historical/heritage-pending Person 변환 (디렉터 7 vs 11 결정)
- Phase 5a 6 Event 외래키 매트릭스 모두 활성 (`(npc 미등록)` 마커 0건 도달)
- mind eligible: 9 → **13** (npc-08·09·10·11 정식 변환 시점)
- 회귀 가드: 기존 9 active person mind upsert 영향 없음(idempotent)

### 디렉터 결정 필요 사항 (체크포인트 2 시작 전)

1. **권장 7 vs 선택 11** — 사양 §3.3 권장은 7건. 선택 추가 4건은 별도 follow-up 또는 Phase 6+ 후보.
2. **boundary 케이스 결과 처리** — `event-bloody-cult-rebellion-2nd` 임서운 추가 여부 (본 보고 §결정 6 참조).
3. **HEXACO 등급 정책** — 추양진인·진천명·단운·바투 등은 출처 단편 → heritage-pending. npc-09
   진대인은 npc-09-jinyarim.md에 부친으로 묘사가 깊어 정밀 가능 후보.

체크포인트 2 진입 후 본 보고서를 베이스라인으로 사용 — Phase 5c.1 종결 후 Cowork에서
`task-phase5-followup-mid-era-events.md` 작성으로 자연 이행.
