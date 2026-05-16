# Phase 2 Stage 5 — Narrative 시뮬레이션 검증 (FROZEN SPEC)

> **상태**: FROZEN (설계 종결, Claude Code 인계용 실행 spec)
> **상위 spec**: `task-rel-phase2-domain-migration.md` §7 Stage 5 (line 2021~2034) — 본 문서는 그 골격의 *구체화*이며 재정의가 아님
> **선행**: Stage 0~4 완료. git HEAD `c686240` (PR #91 머지)
> **회고 산출물(작업 후)**: `phase2-stage5-narrative.md` (별도)
> **설계 근거**: 모든 결정(S5-D1~D6)은 Stage 0 사실조사(코드 grep + cargo 실측)로 검증됨

---

## §0. 본 Stage의 성격

- spec에 범위·게이트가 *이미 정의*됨 → 무거운 B-D 재정의 없음. **검증·튜닝 phase**.
- Stage 0 사실조사가 spec 대비 **작업 성격 변경 4건 + 신규 결정 6건** 도출 → 본 FROZEN spec이 정본.
- 코드 변경면: 작음 (테스트 픽스처 정합화 + 신규 테스트 파일 + 회고). 도메인/application/adapter 로직 변경 0.

---

## §1. 확정 결정 (S5-D1 ~ S5-D6) — 변경 금지

| ID | 결정 | 근거 (Stage 0 출처) |
|---|---|---|
| **S5-D1** | Stage 5 ground truth = `base_delta × intensity × hexaco_modifier` **default 산출값 정본**. spec §3.6 수치(S2 -49 등)는 *디자인 타당성 추정치*로 격하. axis_modulation은 Phase 2.5 reflection LLM 출력 필드 → Stage 5엔 입력 부재 = default ±0 (코드에 ±5 가산 항 자체 없음). | 묶음①: mapping.rs L263~272 / S2 코드 -45 vs §3.6 -49 |
| **S5-D2** | S4(임충→고구)는 §3.6 focus 수치 *의도적 부재* = **정성, 게이트 3 흡수**. 게이트 2 = "**S1~S3 정량 ±N + S4 정성**". | 묶음③ / §3.6 S4 "3 layer separation" 정성 규정 |
| **S5-D3** | tolerance N = **±0.5**. ground truth = 진입 시점 *코드 실행 산출 4축값 박제* **회귀 가드**. intensity는 weight(HEXACO·튜닝)에만 존재 → 코드가 유일 정본. | 묶음②: helpers.rs intensity=`|base|×weight×mod`, compound `(c1+c2)/2` |
| **S5-D4** | 게이트 분업: **2=기계적 회귀**(박제±0.5) / **3·4=서사 타당성**(디자이너→adjustment→JSON). 어색 시 게이트2 완화 *금지*, 게이트3에서 입력 JSON 조정→박제값 갱신. **조정은 입력, 가드는 출력 재현.** | S5-D1~D3 정합 / W1 가드 동형 |
| **S5-D5** | 진입 baseline 정본 = **(failed=0 ∧ S4 대비 회귀 0 ∧ D3 3밴드 보존)**. passed 절대수는 측정명령·feature flag·캡처 의존 → baseline 부적합. | D재측정: S4 875 vs `--lib --tests --bins` 838 = 미실행 차(회귀 아님) |
| **S5-D6** | 게이트1 = bench 3함수 **밴드 assert 자동**(`<0.3`/`[0.3,0.7)`/`≥0.7`) + exact(0/0.461/0.980) **로그 박제+Stage2 baseline 수동대조**(편차 시 재독, tol±0.01). exact 자동 assert 금지(D4 간섭). D2/D4 동형. | D3경로: phase1_bench_test L242~336 assert는 밴드만 |

---

## §2. 작업 항목 (작업 1 ~ 6) — Claude Code 실행 단위

각 작업은 *compile → 관련 테스트 → 다음 작업* 게이트. 한 작업 실패 시 다음 진행 금지.

### 작업 1 — daily 픽스처 B-D12 정합화 ★ Stage 0 도출 (spec 미예상)

**문제(실측)**: `tests/phase1_daily_training_test.rs` L52-58 `set_intensity(Pride,0.4)` 단독. Pride는 B-D12로 helper(`relationship_policy.rs` L127-129) `continue` 스킵 → `update_axes_from_emotion` 0회 → 4축 변동 0 → L105 `(after-initial).abs()>EPSILON` **영구 실패**. 코드 버그 아님 = B-D12 확정 *이전* mock 결함.

**수정(Admiration 단독 — 확정)**:
1. `set_intensity(Pride,0.4)` → `set_intensity(EmotionType::Admiration, 0.4)` 단독. (0.4 = 게이트3 디자이너 검토 대상 — 회고에 명시.)
2. L105 assert `.affinity().value()` → `.respect().value()`. 변수 `initial_closeness`/`after_closeness` → `initial_respect`/`after_respect`. 메시지 → "★ respect 변화 (제자 성장 인정 — Admiration)".
3. `daily-training.json` `_expected_axes_delta` → `"미세 (respect ↑ — 제자 성장 인정/Admiration)"`.
4. 테스트 doc 주석 L9 → "respect 미세 ↑ (Admiration)".
5. `#[ignore="..."]` 제거.

**근거**: Admiration="어제보다 안정됐다" 사부 인정 / Gratitude 방향 어색 배제 / Love·복합 변동폭 과대(high-band)라 mid 부적합. base_delta Adm=respect+20 단독. **D3와 무관**(D3=phase1_bench 하드코딩, JSON·픽스처 안 읽음).

**게이트**: `cargo test --test phase1_daily_training_test` → 1 passed.

### 작업 2 — shanshenmiao #[ignore] 해제

`--ignored` 강제 green 실측 완료. **`tests/phase1_shanshenmiao_test.rs` L38 `#[ignore="..."]` 제거만.**

**게이트**: `cargo test --test phase1_shanshenmiao_test` → 1 passed.

### 작업 3 — `tests/phase2_narrative_test.rs` 신설 (S1~S3 정량 + S4 정성)

미존재 확정. 기존 `phase1_*_test.rs` 패턴(mock ReflectionResult + set_intensity + dispatch_v2 + repo 4축 readback) 재사용.

**기여 감정 mock (묶음③ 확정 — base_delta 비-0만)**:
- S1 임충→노지심: Admiration + Gratitude (Joy=0)
- S2 임충→육겸: Reproach + Hate + Anger (Distress·FC=0). mapping.rs L471/L564 인프라 조립.
- S3 수련→옥교룡: Pity + Reproach + Anger (Distress=0)
- S4: 정량 *제외*. 정성 검증 주석 + 게이트3 핸드오프 노트만 (함수 없음).

**박제 절차(S5-D3)**: ① 각 케이스 실제 실행 → 산출 4축 4값 획득 ② `assert!((v-EXPECTED).abs()<0.5)` (EXPECTED=실행 산출값) ③ 회고에 박제표 ("진입 시점 코드 산출=ground truth") ④ axis_modulation 부재 주석.

**게이트**: `cargo test --test phase2_narrative_test` → S1~S3 3함수 passed.

### 작업 4 — D3 게이트1 (S5-D6)

본체 = `phase1_bench_test.rs` `bench_narrative_calibration_*` 3함수 (변경 0).
1. 실행 → stdout `... significance: X.XXX` → `baselines/stage5-d3-narrative-<date>.log` 박제.
2. exact를 Stage2 baseline(0/0.461/0.980) 수동 대조. 편차 >±0.01 → 재독 트리거(중단·조사).
3. 밴드 assert 자동 통과 확인.

**assert 강화(exact 박제) 금지** — S5-D6 (D4 간섭 회피).

### 작업 5 — result.json 신규 생성 (B-D9)

재생성 대상 0건(=신규). backup은 `_discarded-v0.6` 통합.
**착수 전 grep 선조사**: result.json write 경로(`serde_json::to_writer`+session) 메커니즘 특정.
- 자동 dump 존재 → 3 narrative 실행 생성.
- 부재 → B-D9 "출력 재현가능" 근거로 *스킵+Phase 2.3/3 위임 회고 명시* (위험 하, 강제 아님).

### 작업 6 — 회고 `phase2-stage5-narrative.md` + 외부 문서

**박제 필수**: S5-D1~D6+근거 / 작업성격변경 4건(게이트2재해석·작업1·게이트1밴드+수동·result신규) / 진입baseline(S5-D5형태)+로그목록 / S1~S3 박제표 / examples 부채(phase5b,무관,1줄) / listener_perspective default-ON 발견 / 작업1 intensity 디자이너 잔여 / S4 정성 결과.
**외부**: spec §7 Stage5에 "→ FROZEN: 본 문서" 포인터 1줄. (00-roadmap/CLAUDE.md 완료표기는 Stage6 책임 — 범위 외.)

---

## §3. 진입 baseline 박제값 (S5-D5)

**측정 명령 (고정)**: `cargo test --lib --tests --bins` (examples 제외 = Stage 4 동일 범위)

| 항목 | 진입값 | 비고 |
|---|---|---|
| exit code | 0 | 빌드·실행 성공 |
| failed | **0** | ✅ 진입 green (정본) |
| Stage 4 대비 회귀(green→red) | **0건** | ✅ 정본 |
| `--lib --tests --bins` passed | 838 (참고치, baseline 아님) | 측정명령·feature 의존 (S5-D5) |
| Stage 4 `cargo test` 로그 합산 | 875 (참고치) | 범위 다름(doc/bin 포함) |
| domain/src `#[ignore]` | 0건 | Stage4 이후 신규 비활성화 없음 |
| Stage 5 대상 ignored | daily + shanshenmiao = 2 (작업1·2 해제) | 실측 |
| 기타 ignored | dispatch_v2 외 — Stage 5 무관 기존 | 실측 |
| D3 3밴드 | chitchat 0.000 / daily 0.461 / shanshenmiao 0.980 | Stage2/3 baseline (작업4 수동 대조 기준) |
| git HEAD | c686240 / 작업트리 코드 변경 0 | 진입 직전 확인 |

**진입 직전 재측정 산출 로그**: `baselines/stage5-prep-RAW.txt` (이미 생성됨, `cargo test --lib --tests --bins` exit=0).

**기존 부채 (진입 차단 아님 — 회고 §위험 박제)**:
- examples `phase5b_checkpoint2_eval` 빌드 실패 (`sqlite_world` import — 예제 코드 결함, lib 정상). Phase 5 산출물, Stage 5 무관.
- `listener_perspective` = `default = ["listener_perspective"]` (Phase 7 Step 5 default-ON). 과거 "default=[]" 기록 폐기.

---

## §4. 실행 순서 + 종결 게이트

**순서 (의존)**: 작업1 → 작업2 → 작업3 → 작업4 → 작업5 → 작업6.
- 작업1·2 = #[ignore] 해제 (진입 baseline을 877+로). 작업3가 작업1·2의 helper 동작에 의존하므로 선행.
- 작업3 박제값은 작업1·2 완료 후 *코드 상태*에서 산출 (S5-D3 "진입 시점").
- 작업4는 독립(D3는 별 경로) — 순서 무관하나 회귀 조기 발견 위해 작업3 후 권장.

**Stage 5 종결 게이트 (spec §7 게이트 1~4 + S5 재정의)**:
1. ☐ **게이트1 (S5-D6)**: bench 3함수 밴드 assert green + exact 로그 박제 + Stage2 수동대조 ±0.01 이내.
2. ☐ **게이트2 (S5-D2·D3)**: phase2_narrative_test S1~S3 박제값 ±0.5 재현 green. S4 = 정성(게이트3).
3. ☐ **게이트3 (S5-D4)**: 4축 변동이 시나리오 의도와 정합 — 디자이너 narrative 검토 통과 (S4 포함). 작업1 intensity 확정.
4. ☐ **게이트4 (S5-D4)**: 어색 케이스 0 (또는 게이트3에서 입력 JSON 조정 완료 → 게이트2 박제값 갱신).
5. ☐ **회귀 게이트 (S5-D5)**: `cargo test --lib --tests --bins` failed=0 ∧ Stage4 대비 회귀 0 ∧ D3 3밴드 보존.

**전체 종결 시 산출**:
- `tests/phase2_narrative_test.rs` (신규)
- `tests/phase1_daily_training_test.rs` / `phase1_shanshenmiao_test.rs` (#[ignore] 해제 + 작업1 정합화)
- `data/scenarios/phase1-validation/daily-training.json` (_expected_axes_delta 정합)
- `baselines/stage5-*.log` (진입·D3·종결)
- result.json (작업5 — 메커니즘 존재 시)
- `phase2-stage5-narrative.md` (회고)
- spec §7 Stage5 포인터 1줄

**Claude Code 인계 시 주의**:
- S5-D1~D6은 **변경 금지** 결정. 실행 중 의문 발생 시 *중단·디자이너 확인*, 임의 재해석 금지.
- 작업1 intensity 0.4는 *잠정* — 게이트3 디자이너 확정 전까지 fix 아님 (회고 명시).
- 디자이너 git 직접 수행 — Claude Code는 commit *메시지 텍스트만* 제공, git 명령 실행 금지.
- 한 작업 게이트 실패 시 다음 작업 진행 금지, 원인 보고.
