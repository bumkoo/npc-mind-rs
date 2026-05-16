# Phase 2 Stage 5 회고 — Narrative 시뮬레이션 검증 (4축 박제)

**Stage**: Phase 2 Stage 5 (FROZEN spec → 2026-05-16 실행 완료)
**범위**: Phase 1 narrative 픽스처 4축 정합화 + S1~S3 narrative 박제 신설 + D3 3밴드 회귀 확인 + 결과 파일 정책 정리.
**Spec (정본)**: [`task-rel-phase2-stage5-narrative-FROZEN.md`](task-rel-phase2-stage5-narrative-FROZEN.md)
**상위 spec**: [`task-rel-phase2-domain-migration.md`](task-rel-phase2-domain-migration.md) §7 Stage 5

---

## §1. 확정 결정 박제 (S5-D1 ~ S5-D6)

본 6개 결정은 *변경 금지*. 향후 Stage·Phase에서 인용 시 본 표를 정본으로 한다.

| ID | 결정 | 근거 |
|---|---|---|
| **S5-D1** | Stage 5 ground truth = `base_delta × intensity × hexaco_modifier` **default 산출값 정본**. spec §3.6 수치(S2 -49 등)는 *디자인 타당성 추정치*로 격하. `axis_modulation`은 Phase 2.5 reflection LLM 출력 필드 → Stage 5엔 입력 부재 = default ±0 (코드에 ±5 가산 항 자체 없음). | mapping.rs L263~272 / S2 코드 -45 vs §3.6 -49 / `src/domain/reflection.rs::ReflectionResult` 7 필드에 `axis_modulation` 없음 |
| **S5-D2** | S4(임충→고구)는 §3.6 focus 수치 *의도적 부재* = **정성, 게이트3 흡수**. 게이트2 = "**S1~S3 정량 ±N + S4 정성**". | §3.6 S4 "3 layer separation" 정성 규정 |
| **S5-D3** | tolerance N = **±0.5**. ground truth = 진입 시점 *코드 실행 산출 4축값* **회귀 가드**. intensity는 weight(HEXACO·튜닝)에만 존재 → 코드가 유일 정본. | helpers.rs intensity=`|base|×weight×mod`, compound `(c1+c2)/2` |
| **S5-D4** | 게이트 분업: **2=기계적 회귀**(박제 ±0.5) / **3·4=서사 타당성**(디자이너→adjustment→JSON). 어색 시 게이트2 완화 *금지*, 게이트3에서 입력 JSON 조정→박제값 갱신. **조정은 입력, 가드는 출력 재현.** | S5-D1~D3 정합 / W1 가드 동형 |
| **S5-D5** | 진입 baseline 정본 = **(failed=0 ∧ Stage 4 대비 회귀 0 ∧ D3 3밴드 보존)**. passed 절대수는 측정명령·feature flag·캡처 의존 → baseline 부적합. | D재측정: S4 875 vs `--lib --tests --bins` 838 = 미실행 차(회귀 아님) |
| **S5-D6** | 게이트1 = bench 3함수 **밴드 assert 자동**(`<0.3`/`[0.3,0.7)`/`≥0.7`) + exact(0/0.461/0.980) **로그 박제 + Stage2 baseline 수동대조**(편차 시 재독, tol ±0.01). exact 자동 assert *금지*(D4 간섭). D2/D4 동형. | phase1_bench_test L242~336 assert는 밴드만 |

---

## §2. 작업 성격 변경 4건 (Stage 0 사실조사 → FROZEN spec 도출)

상위 spec §7 Stage 5 (line 2021~2034)에 *추가/재해석된* 사항들. Stage 5 본 회고에 박제.

| 변경 | 변경 전 (상위 spec 추정) | 변경 후 (FROZEN spec 본문) | 근거 |
|---|---|---|---|
| **게이트2 재해석** | "S1~S4 ground truth ±N 이내" — 4 케이스 정량 | "**S1~S3 정량 ±0.5 + S4 정성(게이트3)**" (S5-D2/D3) | §3.6 S4 focus 수치 의도적 부재 / "3 layer separation" 정성 규정 |
| **작업1 신규** (spec 미예상) | (없음) | `tests/phase1_daily_training_test.rs` Pride→Admiration 픽스처 정합화. Pride는 B-D12 helper에서 `continue` 스킵 → 4축 변동 0 → 영구 실패. 코드 버그 아님 = B-D12 확정 *이전* mock 결함. | relationship_policy.rs L127-129 (B-D12 guard) / mapping.rs base_delta(Pride) 가드 |
| **게이트1 = 밴드 assert + 수동대조** | "3밴드 calibration 보존 ± tolerance" — exact assert 가능성 | "bench 3함수 **밴드 자동** + exact **수동대조**(편차 ±0.01 시 재독)". exact 자동 assert 강화 *금지*. (S5-D6) | phase1_bench_test L242~336 — assert는 밴드만, exact는 println |
| **result.json 신규 생성** | "session_*_result.json 일괄 재생성 (B-D9)" — 재생성 | "재생성 대상 0건 = **신규**". backup은 `_discarded-v0.6` 통합. 자동 dump 메커니즘 부재 확인 → B-D9 "출력 재현가능" 근거로 *스킵 + Phase 2.3/3 위임* (§5 작업5 결과 참조). | grep 결과: `result.json` write는 `state.rs::save_to_file(as_scenario=false)` 만 — Mind Studio 인터랙티브 save 전용, 테스트 자동 dump 부재 |

---

## §3. 진입 baseline (S5-D5)

**측정 명령 (고정)**: `cargo test --lib --tests --bins` (examples 제외 = Stage 4 동일 범위)

| 항목 | 진입값 | 비고 |
|---|---|---|
| exit code | 0 | 빌드·실행 성공 |
| **failed** | **0** | ✅ 진입 green (정본) |
| Stage 4 대비 회귀(green→red) | **0건** | ✅ 정본 |
| `--lib --tests --bins` passed | 838 (참고치, baseline 아님) | 측정명령·feature 의존 (S5-D5) |
| domain/src `#[ignore]` | 0건 | Stage 4 이후 신규 비활성화 없음 |
| Stage 5 대상 ignored | daily + shanshenmiao = 2 (작업1·2 해제 대상) | 실측 |
| D3 3밴드 | chitchat 0.000 / daily 0.461 / shanshenmiao 0.980 | Stage 2/3 baseline (작업 4 수동 대조 기준) |
| git HEAD | c686240 / 작업트리 코드 변경 0 | 진입 직전 |

**로그 파일**:
- `baselines/stage5-d3-narrative-2026-05-16.log` — 작업 4 D3 박제 + Stage2 수동대조 표
- `baselines/stage5-exit-2026-05-16.log` — 종결 시점 `--lib --tests --bins` 측정 (843 passed / 0 failed / 2 ignored)

**기존 부채 (진입 차단 아님 — §위험 박제)**:
- examples `phase5b_checkpoint2_eval` 빌드 실패 (`sqlite_world` import — 예제 코드 결함, lib 정상). Phase 5 산출물, Stage 5 무관.
- `listener_perspective` = `default = ["listener_perspective"]` (Phase 7 Step 5 default-ON). 과거 "default=[]" 기록 폐기.

---

## §4. 작업별 결과

### 작업 1 — daily 픽스처 B-D12 정합화 ★ Stage 0 도출 (spec 미예상)

**문제**: `tests/phase1_daily_training_test.rs` L52-58 `set_intensity(Pride, 0.4)` 단독. Pride는 B-D12로 helper(`relationship_policy.rs::apply_emotions_to_relationship` L127-129) `continue` 스킵 → `update_axes_from_emotion` 0회 → 4축 변동 0 → L105 `(after-initial).abs()>EPSILON` **영구 실패**. 코드 버그 아님 = B-D12 확정 *이전* mock 결함.

**수정** (Admiration 단독):
1. `set_intensity(EmotionType::Pride, 0.4)` → `set_intensity(EmotionType::Admiration, 0.4)` 단독 (0.4 = **게이트3 디자이너 검토 잠정**, fix 아님).
2. L105 assert `.affinity().value()` → `.respect().value()`. 변수 `initial_closeness`/`after_closeness` → `initial_respect`/`after_respect`. 메시지 → "★ respect 변화 (제자 성장 인정 — Admiration)".
3. `daily-training.json` `_expected_axes_delta` → `"미세 (respect ↑ — 제자 성장 인정/Admiration)"`.
4. 테스트 doc 주석 → "respect 미세 ↑ (Admiration)".
5. `#[ignore="..."]` 제거.

**근거**: Admiration="어제보다 안정됐다" 사부 인정. Gratitude 방향 어색 배제 / Love·복합 변동폭 high-band 부적합. base_delta Adm = respect+20 단독. **D3와 무관** (D3 = phase1_bench 하드코딩, JSON·픽스처 안 읽음).

**게이트**: `cargo test --test phase1_daily_training_test` → ✅ **1 passed**.

### 작업 2 — shanshenmiao #[ignore] 해제

`--ignored` 강제 green 실측 완료. **`tests/phase1_shanshenmiao_test.rs` L38 `#[ignore="..."]` 제거만.**

**게이트**: `cargo test --test phase1_shanshenmiao_test` → ✅ **1 passed**.

### 작업 3 — `tests/phase2_narrative_test.rs` 신설 (S1~S3 정량 + S4 정성)

미존재 확정 → 신규 작성. 기존 `phase1_*_test.rs` 패턴 (mock ReflectionResult + set_intensity + dispatch_v2 + repo 4축 readback) 재사용. NPC/관계는 `InMemoryRepository::new()` + `NpcBuilder` + `RelationshipBuilder`로 인라인 구성 — fixture JSON 신설 0 (S5-D3 박제 절차는 코드 출력 기반이므로 fixture 불필요).

**기여 감정 (spec §2 작업3 묶음③ — base_delta 비-0만)**:
- S1 임충→노지심: Admiration 0.6 + Gratitude 0.6 (Joy=0)
- S2 임충→육겸: Reproach 0.8 + Hate 0.8 + Anger 0.9 (Distress·FC=0)
- S3 수련→옥교룡: Pity 0.7 + Reproach 0.7 + Anger 0.6 (Distress=0)
- S4: 정량 *제외*. 정성 검증 주석 + 게이트3 핸드오프 노트만 (함수 없음, 파일 말미 주석).

**박제표 (S5-D3, 진입 시점 코드 산출 = ground truth, tol ±0.5)**:

| 케이스 | 초기 (trust/aff/resp/war) | HEXACO modifier | sum delta | 박제 EXPECTED |
|---|---|---|---|---|
| S1 임충→노지심 | 50 / 40 / 20 / 0 | trust×1.2 (sincerity 0.6) | +14.4 / +6 / +12 / -6 (war 0으로 clamp) | **(64.4, 46.0, 32.0, 0.0)** |
| S2 임충→육겸 | 50 / 40 / 20 / 0 | 동일 (forgiveness=0.0, A−Forg 미발동) | -46.2 / -37 / -24 / +42.5 | **(3.8, 3.0, -4.0, 42.5)** |
| S3 수련→옥교룡 | 40 / 30 / 10 / 20 | trust×0.672 / 기타×0.56 (sin 0.7 × pat 0.9 × pru 0.8; forgiveness=0.6 A−Forg 미발동) | -14.784 / -3.36 / -11.76 / +12.32 | **(25.216, 26.64, -1.76, 32.32)** |

EXPECTED는 디자이너 산정값이 *아님*. 본 commit 시점의 코드 출력 = ground truth (S5-D3). 어색 시 게이트2 완화 금지 (S5-D4), 게이트3에서 입력 조정 → EXPECTED 갱신.

**axis_modulation 부재 주석**: 본 테스트 파일 상단 doc-comment 박제. `ReflectionResult` 7 필드 정합 (S5-D1).

**게이트**: `cargo test --test phase2_narrative_test` → ✅ **3 passed**.

### 작업 4 — D3 게이트1 (S5-D6)

`phase1_bench_test.rs` `bench_narrative_calibration_*` 3함수 (변경 0):
1. 실행 → stdout 박제 → `baselines/stage5-d3-narrative-2026-05-16.log`.
2. exact 값 Stage2 baseline(0 / 0.461 / 0.980) 수동 대조. 편차 ±0.01 이내.
3. 밴드 assert 자동 통과 확인.

**박제 결과**:

| 케이스 | exact | Stage 2 baseline | 편차 | 밴드 assert |
|---|---|---|---|---|
| chitchat (low) | 0.000 | 0.000 | 0.000 | ✅ `< 0.3` |
| daily (mid) | 0.461 | 0.461 | 0.000 | ✅ `[0.3, 0.7)` |
| shanshenmiao (high) | 0.980 | 0.980 | 0.000 | ✅ `≥ 0.7` |

편차 전부 0.000 ≪ ±0.01 → 재독 트리거 없음.
**exact 자동 assert 강화 *적용 안 함*** (S5-D6 / D4 간섭 회피).

### 작업 5 — result.json 신규 생성 (B-D9) → **스킵 + Phase 2.3/3 위임**

**grep 선조사 결과**: result.json write 경로 메커니즘 = **자동 dump 부재**.
- `src/bin/mind-studio/state.rs::save_to_file(path, as_scenario=false)` — 유일한 `result.json` writer. Mind Studio REST `/api/save` 핸들러를 통한 *인터랙티브 사용자 액션* 전용.
- 테스트(`tests/*.rs`), `dispatch_v2`, narrative 시나리오 어디에도 `serde_json::to_writer` + `result.json` 자동 dump 패턴 없음.
- 기존 `data/_discarded-v0.6/treasure_island/.../session_*_result.json` 3건 = 과거 인터랙티브 세션 산출. 재생성 대상 0건.

**결정**: B-D9 ("출력 재현가능 — 결과 파일은 입력 아님") 근거로 본 Stage에서는 **스킵**. Phase 2.3/3에서 narrative 시뮬레이션 자동 dump 인프라(예: `cargo run --bin narrative-dump -- --scenario S1`) 신설 후 결과 재생성. 위험 *하* (regression guard는 §4 작업 3·4가 이미 보장).

### 작업 6 — 회고 + 외부 문서 (본 문서)

- ✅ `phase2-stage5-narrative.md` (본 문서) 작성.
- ✅ 상위 spec §7 Stage 5에 "→ FROZEN: 본 문서" 포인터 1줄.
- 00-roadmap.md / CLAUDE.md Stage 5 완료 표기는 **Stage 6 책임** — 본 회고 범위 외 (FROZEN spec §2 작업 6 명시).

---

## §5. 종결 게이트 (FROZEN spec §4)

| 게이트 | 정의 | 결과 |
|---|---|---|
| **1 (S5-D6)** | bench 3함수 밴드 assert green + exact 로그 박제 + Stage 2 수동 대조 ±0.01 이내 | ✅ 통과 (작업 4) |
| **2 (S5-D2·D3)** | phase2_narrative_test S1~S3 박제값 ±0.5 재현 green | ✅ 통과 (작업 3) |
| **3 (S5-D4)** | 4축 변동이 시나리오 의도와 정합 — 디자이너 narrative 검토 통과 (S4 포함). 작업 1 intensity 확정. | ⏳ **디자이너 잔여** (§6 핸드오프 노트) |
| **4 (S5-D4)** | 어색 케이스 0 (또는 게이트 3에서 입력 JSON 조정 완료 → 게이트 2 박제값 갱신) | ⏳ **디자이너 잔여** (§6 핸드오프 노트) |
| **회귀 게이트 (S5-D5)** | `cargo test --lib --tests --bins` failed=0 ∧ Stage 4 대비 회귀 0 ∧ D3 3밴드 보존 | ✅ 통과 — 843 passed / 0 failed / 2 ignored. Stage 4 대비 회귀 0건 (passed +5 = 작업 1·2 unignore +2, S1~S3 +3). D3 3밴드 보존 (작업 4). |

---

## §6. 디자이너 핸드오프 노트 (게이트 3·4 잔여)

게이트 3·4는 *서사 타당성* 판단 — 본 회고 시점에 처리 불가. Bekay가 Mind Studio에서 수동 검증 시 본 표 참조.

### 6.1. 작업 1 intensity 0.4 잠정 확정

`tests/phase1_daily_training_test.rs::set_intensity(EmotionType::Admiration, 0.4)`는 *잠정값* (FROZEN spec §2 작업 1 명시). 게이트 3 디자이너 검토에서 다음 중 1택:
1. **0.4 유지** — 일상 가르침의 "어제보다 안정됐다" 톤에 맞으면 그대로 박제.
2. **상향** (예: 0.5~0.6) — Admiration 강도가 너무 약하면 mid 밴드 axes 변동이 미세를 넘어 미세-중강이 되는지 확인.
3. **하향** (예: 0.3) — mid 밴드 axes 변동이 *너무 큼* 판정 시.

조정 시 `daily-training.json` `_expected_axes_delta` 문구 + 본 회고 §4 작업 1 박제값도 동기 갱신 (S5-D4).

### 6.2. S1~S3 박제값 narrative 타당성

§4 작업 3 표 EXPECTED는 "현재 코드의 출력"이지 "디자이너 의도와 정합"의 보증 아님. 게이트 3에서:
- **S1 임충→노지심**: respect +12, trust +14.4. 의리·은혜의 갚음. Admiration+Gratitude 효과 *합치 적절한가*?
- **S2 임충→육겸**: trust 50→3.8 (거의 0). 옛 친구의 처단 → 완전 단절 합당. wariness +42.5 (50% 도달) — *과한지/부족한지*.
- **S3 수련→옥교룡**: trust -14.8 / respect -11.8 / wariness +12.3. 안타까움+책망+분노 (제자의 폭주). 사부의 *체념과 한* 표현 적절한가?

어색 시 (S5-D4):
- 입력 emotion intensity 조정 → 코드 재실행 → 새 EXPECTED 박제 → 본 회고 §4 작업 3 표 갱신.
- 게이트 2 tolerance 완화 *금지*.

### 6.3. S4 임충→고구 정성 검증

§3.6 focus 수치 *의도적 부재* (S5-D2). 디자이너 narrative 직관으로 확인:
- 임충의 對고구 감정은 *체제 정점에 대한 누적 분노* — 단일 시점 EmotionState mock으로 박제 어려움 (시간 분산 + 권력 거리).
- "3 layer separation" (서사·인지·정서)이 4축 변동에 자연스럽게 반영되는지 *서사 직관*과 정합 검증.
- 향후 Phase 2.3+에서 시간 분산 + `axis_modulation` 활성화 시 정량 가능.

---

## §7. 위험 / 부채 박제

| 항목 | 분류 | 후속 처리 |
|---|---|---|
| `listener_perspective` default-ON 발견 | 사실 정정 | Phase 7 Step 5 정합. 과거 "default=[]" 기록 폐기. 본 회고에 박제만. |
| examples `phase5b_checkpoint2_eval` 빌드 실패 (`sqlite_world` import) | Phase 5 산출물 결함 | Stage 5 무관. Phase 5 후속에서 처리 — 본 회고 1줄 명기. |
| result.json 자동 dump 인프라 부재 | B-D9 잔여 | Phase 2.3/3 위임 — narrative 시뮬레이션 자동 dump CLI 신설 시 결과 일괄 재생성. |
| 작업 1 intensity 0.4 잠정 | 디자이너 확정 잔여 | 게이트 3 검토 시 §6.1 절차로 확정. |
| S4 정성 검증 | 디자이너 잔여 | §6.3. |

---

## §8. 변경 이력

| 일자 | 변경 |
|---|---|
| 2026-05-16 | Phase 2 Stage 5 종결. 작업 1~6 모두 종결 또는 디자이너 핸드오프. 게이트 1·2·회귀 통과, 게이트 3·4는 디자이너 검토 잔여 (§6). |
