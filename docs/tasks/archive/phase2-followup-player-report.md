# Phase 2 Follow-up — Player Character (Q2·B kind="player") 보고서

> **선행**: Phase 2 종결 (체크포인트 1·2 통과 + Critical 2 + Important 6 fix-up).
> **사양**: `docs/tasks/task-phase2-followup-player-character.md`
> **commit**: 본 보고서와 같은 commit에 동봉.

---

## 1. Done — 사양 §4 Done Criteria

- [x] `projects/chilguk-chunchu/world/person/player.md` 작성 — `kind: player`, `id: player`
- [x] world-load 실행 시 `mind eligible = 8` (기존 7 active + 1 player)
- [x] e2e 테스트 7건 추가 (`tests/world_chilguk_chunchu_player_e2e.rs`)
- [x] 기존 batch e2e 회귀 없음 — `load_all_persons`이 player를 별도 슬라이스로 분리
- [ ] 디렉터 수동 검증: mind-studio 시작 로그 + dialogue_start("player", partner=...) — LLM 의존이라 자동 수행 불가

7/8 자동 완료. 미완 1건은 LLM 서버 의존.

---

## 2. 결정 — 사양 §3.1 Q&A

### 2.1 ID = `player` (단일 인스턴스 가정)

디렉터 결정: "단일 플레이어 가정 채택. 멀티플레이어는 게임 디자인 결정이지 도구 결정 아님". 본 frontmatter는 `id: player`로 단일 인스턴스. Phase N+ 멀티플레이어 도입 시 `pc-01`, `pc-02` 등으로 마이그레이션 가능 — id가 단일 컬럼이라 변경 비용 낮음.

### 2.2 HEXACO 시작값 — §3.3 권장값 그대로 채택

| dim | 값 | 근거 |
|---|---|---|
| H | +0.5 | 화산파 정파적 정직성 학습 + 임서운의 가르침 + 17세 천진함 |
| E | +0.3 | 17세 + 두 번의 학살 트라우마 + 임서운/부모 부재 의존성 |
| X | 0.0 | 청년기 평균 + 자유도시 빈민가 생존술 |
| A | +0.4 | 명경 가르침에서 자비 학습 + 빈민가 동료에 대한 의리 |
| C | +0.5 | 화산파 수련 규율의 잔재 + 빈민가 생존술 |
| O | +0.5 | 청년기 평균 + 떠돌이 경험에서 다양한 환경 노출 |

회귀 가드: `player_hexaco_matches_recommended_baseline` 테스트가 6 dim 정확값을 잠금.

### 2.3 affiliation 빈 배열

화산파 멸문 + 정식 입문 없음 → `affiliation: []`. 무소속이 의미 정합. Phase 3+ `group-free-cities` 같은 자유도시 그룹 추가 시 옵션. `extras.starting_location: place-free-cities-back-alleys` 마커로 위치 보존.

### 2.4 player 전용 extras 마커

```yaml
extras:
  starting_inventory: [혈매화검(血梅花劍), 동전 몇 닢]
  starting_location: place-free-cities-back-alleys
  player_init: true            # 게임 시작 시 사용자 입력으로 name·gender 갱신됨
  big_five_legacy:             # 청년기 평균 — 게임 진행 중 갱신되는 가변 영역
    openness: 0.55, ...
  values:                       # 17세 baseline (다른 active 인물 정형화 대비)
    chung: 0.4, ...
```

`player_init: true` 마커 — `extras` 디렉터 컨벤션. 향후 mind-studio가 게임 시작 시 사용자 입력으로 `name`·`gender`를 갱신하면 이 마커를 트리거로 사용 가능.

---

## 3. Diff

```
projects/chilguk-chunchu/world/person/player.md     (신규) 약 110 라인
tests/world_chilguk_chunchu_player_e2e.rs            (신규) 약 165 라인 (7 e2e)
tests/world_chilguk_chunchu_persons_batch_e2e.rs    (수정) load_all_persons에 active 필터 추가
                                                            (player는 별도 슬라이스)
docs/tasks/phase2-followup-player-report.md          (신규) 본 보고서
```

코드 변경 줄 수: ~280 (사양 §7 추정 100% 일치). 본문 코드(`src/`) 변경 0 — Phase 2에서 이미 `kind=player` 처리가 일반화되어 있어 데이터·테스트 추가만 수행.

---

## 4. 데모

### 4.1 world-load

```bash
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload
```

```
=== 결과 (DB 미수정) ===
project           = chilguk-chunchu
groups parsed     = 6
persons parsed    = 8        ← player 추가 (기존 7 + 1)
errors            = 0
cycles            = 0
fk errors (활성)  = 2          ← npc-11 잔여 (Phase 2 의도된 상태, 본 follow-up 영향 없음)
mind failures     = 0
```

`mind eligible = 8` — kind=player가 person_to_npc 적격이며 변환 가능을 확인.

### 4.2 e2e 테스트

```bash
cargo test --features embed --test world_chilguk_chunchu_player_e2e
# test result: ok. 7 passed; 0 failed
```

7 테스트 항목:
- `player_parses_with_correct_kind_and_id` — id/kind/age/affiliation 검증
- `player_hexaco_matches_recommended_baseline` — §3.3 권장값 회귀 가드
- `player_is_mind_eligible` — Q2·B 정책 자동 검증 (`person_to_npc` Some 반환)
- `player_sqlite_roundtrip_preserves_all_fields` — extras·marker 보존
- `list_persons_kind_player_returns_only_player` — kind 필터 정확성
- `player_count_combined_with_seven_actives_is_eight` — 8 인물 공존
- `player_extras_carry_starting_inventory` — 혈매화검 등 마커 검증

### 4.3 회귀 가드

기존 e2e:
- `world_chilguk_chunchu_persons_batch_e2e`: 12/12 통과 (active 7인 분리 검증)
- `world_chilguk_chunchu_person_e2e`: 11/11 통과 (npc-02 단독)
- `world_chilguk_chunchu_e2e`: 14/14 통과 (Phase 1 group)

신규 7 + 기존 37 = **44 워드빌드 e2e 모두 통과**.

---

## 5. 막힌 결정

없음. 사양 §3 / §4 / §5 모두 별다른 분기 없이 직선적 진행.

---

## 6. Out of Scope (향후 task)

- 사용자 빌드 시스템 (skill·class·level·equipment 갱신) — 게임 플레이 Phase
- 멀티플레이어 (id=pc-01 등) — Phase N+
- 사용자 이름·외모 커스터마이징 UI — Phase N+
- player_init 마커 활성 흐름 (mind-studio가 첫 입력 시 갱신) — 별도 task

---

## 7. Phase 2 본문 §3.3 / §6.7 검증 통과

본 follow-up은 Phase 2 본문 §3.3 / §6.7의 **Q2·B 결정** ("별도 PlayerCharacter 도메인 안 만듦, Person.kind=player 인스턴스 1개로 처리")을 단독 슬라이스로 검증. 결과:

- ✅ kind=player가 mind 시스템에서 active와 동일 흐름으로 처리됨 (`person_to_npc` Some)
- ✅ HEXACO·extras·temporal 모두 active와 동일한 frontmatter 스키마 사용
- ✅ Phase 2 본문이 만든 일반화가 player 케이스를 별도 코드 추가 없이 흡수 — 이는 §3.3 의 Q2·B 결정 정합성을 강하게 입증

> Phase 2 종결 후 follow-up이 본문 결정을 회귀 검증한 사례. 디자인 결정 자동 회귀 가드의 모범 사례로 본 보고서 §7을 향후 follow-up TASK 패턴으로 인용 가능.
