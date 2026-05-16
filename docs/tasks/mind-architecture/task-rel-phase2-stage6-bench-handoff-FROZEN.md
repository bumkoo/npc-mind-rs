# Phase 2 Stage 6 — Bench + 회고 + Phase 2.3 KICKOFF (FROZEN SPEC)

> **상태**: FROZEN (설계 종결, Claude Code 인계용 실행 spec)
> **상위 spec**: `task-rel-phase2-domain-migration.md` §7 Stage 6 (line 2044~2066) — 골격 구체화 (재정의 아님)
> **선행**: Stage 0~5 완료. git HEAD `5b2b798` (PR #92 머지, Stage 5 종결)
> **본 Stage = Phase 2 *마지막* Stage** (Phase 2는 Stage 0~6 = 6 작업 Stage 구조, Stage 6 종결 = Phase 2 완료)
> **설계 근거**: 모든 결정(S6-D1~D5)은 Stage 6 사실조사(spec/회고 grep + cargo/bench 실측)로 검증됨

---

## §0. 본 Stage의 성격

- **측정 + 인계 phase** — 코드 변경 거의 0 (bench 재실행 + 문서 작성/갱신 위주).
- spec §7 Stage 6 범위 8항에 *코드 수정 항목 0개*. Stage 5(픽스처 정합)와 달리 로직 변경 없음.
- Stage 6 신규 *발명* 결정 사실상 0 — Stage 5 baseline 철학(S5-D5/D6)을 Phase 2 종결 측정에 *일관 승계*.
- Stage 0 사실조사가 spec 대비 **작업 성격 변경 4건 + 신규 결정 5건** 도출 → 본 FROZEN spec이 정본.

---

## §1. 확정 결정 (S6-D1 ~ S6-D5) — 변경 금지

| ID | 결정 | 근거 (Stage 6 사실조사) |
|---|---|---|
| **S6-D1** | 게이트3 "1095+ tests"를 **S5-D5 승계 재해석**: 정본 = `cargo test --lib --tests --bins` 기준 **failed=0 ∧ Stage 5 종결 대비 회귀 0 ∧ narrative 3밴드 보존 ∧ 증감분 전부 설명 가능(미스터리 증감=회귀 트리거)**. 절대수(545/866/1095/1220)는 측정명령·feature 의존 참고치로 격하. 측정명령 Stage 5와 동일 고정. | ⑥-B: 동일 프로젝트가 spec 내 4개 절대수로 표기 — S5-D5 병리 재출현 |
| **S6-D2** | 게이트1·2(D2/D4 latency) = **S5-D6 패턴 D2/D4 확장**: `phase1_bench_test.rs` bench 함수 자동 실행 → spec 박제 임계값 대조 → 로그 박제. 임계값 신규 산정 0 (spec L1891 기존재: D2 chitchat≤29/significant≤42/legacy≤35.2µs, D4 ±20%). exact 자동 assert 강화 *금지*. | ⑥-A: D2/D4 측정경로(phase1_bench_test L30/70/139)·임계값 spec 기존재 |
| **S6-D3** | KICKOFF 산출 = spec "작성"이 아니라 **기존 `PHASE2.3-KICKOFF.md`(Stage 3본, A~E) 갱신**. Stage 4·5 인계 5항 추가: ① result.json dump 부재 ② intensity 0.4 잠정 ③ S1~S3 narrative 타당성 ④ S4 정성·시간분산 ⑤ Stage 4·5 메트릭 baseline. §3 baseline에 Stage 5 종결값 반영. | ⑥-C: KICKOFF 기존재(커밋 e8accff=Stage 3본), Stage 4·5 미반영 |
| **S6-D4** | A5(MAX_EVENTS=22) = **코드변경 0, grep+산출 교차검증만**. `dispatcher.rs:35-41` 상수=22 grep + spec L221~230 worst-case(Phase2 변경0 / Phase2.5 worst≈17<22) 교차대조. 인상 불필요 결론 박제. Phase 2.5분은 *산출 가정 명시*(실측 아님). | ⑥-A: spec L212~230 이미 "22 안전" 결론 박음 |
| **S6-D5** | KICKOFF §E(v0.6 커스텀 Deserialize) = **Stage 4 미처리, 코드 존재 확정 → Phase 2.3 인계 *유지*** (완료 표기 금지). closeness/power 잔존 12+ = **Stage 6 정리 안 함**, KICKOFF §A에 "spec L508 'production 0' vs 실측 12+ 재카탈로그 필요" 플래그만 기록 → Phase 2.3 §A 위임 (지금 고치면 W1 회귀가드=Phase 2.3 진입 트리거 조기 발파). | ⑥-C/C6-5 실측: state.rs:666~671 커스텀 Deserialize 존재 / closeness·power src 12+ 위치 |

---

## §2. 작업 항목 (작업 1 ~ 7) — Claude Code 실행 단위

코드 변경 0 (작업 1~4 = 측정·검증, 5~7 = 문서). 각 작업 게이트 실패 시 다음 진행 금지·보고.

### 작업 1 — D2/D4 latency 재측정 + 박제 (S6-D2)

**측정**: `cargo test --test phase1_bench_test -- --nocapture` → stdout 캡처 → `baselines/stage6-d2d4-bench-<date>.log` 박제.
- D2 함수: `bench_dispatch_v2_end_dialogue_chitchat_vs_significant` (L139)
- D4 함수: `bench_compute_significance_single_call_under_microsecond_avg` (L30), `..._empty_short_long_scaling` (L70)

**대조 (spec 박제 임계값 — 신규 산정 0)**:
| 케이스 | 임계값 | Stage 6 진입 실측(참고) |
|---|---|---|
| D2 chitchat | ≤29.0 µs | 26.35 ✅ |
| D2 significant | ≤42.0 µs | 38.92 ✅ |
| D2 legacy | ≤35.2 µs | 30.74 ✅ |
| D4 10turn×10000 | ≤~10.0 µs (±20%) | 8.29 ✅ |

**게이트**: bench 6 passed / 0 failed. D2 3케이스 ±20% 이내, D4 ±20% 이내. 초과 시 *중단·보고* (회귀 vs 예상 4축비용 분해 필요 — 단 진입 실측 전부 통과라 red 가능성 낮음).
**박제**: 회고에 "4축 매핑 추가 latency 영향" 표 (Phase 1 baseline vs Stage 6 실측 vs 임계값).

### 작업 2 — D3 narrative 3밴드 회귀 재확인 (S6-D1)

작업 1과 동시 측정(같은 bench 실행). `bench_narrative_calibration_*` 3함수 → 0.000/0.461/0.980 Stage 5 종결값 exact 일치 확인. 편차 ±0.01 초과 시 재독 트리거.
**게이트**: 3밴드 자동 assert green + exact 로그 박제 + Stage 5 종결값 대조 일치.

### 작업 3 — 전체 회귀 검증 (S6-D1)

`cargo test --lib --tests --bins 2>&1 > baselines/stage6-final-<date>.log`.
**기준선 (C6-3 박제 — Stage 6 진입값)**: 843 passed / 0 failed / 2 ignored / 65 묶음.
**게이트**: failed=0 ∧ passed ≥ 843 ∧ ignored ≤ 2 ∧ 증감분 전부 설명 가능(Stage 6 신규≈0이므로 843 유지 예상). 미스터리 증감 = 회귀 의심·보고.

### 작업 4 — A5 교차검증 (S6-D4, 코드변경 0)

1. `dispatcher.rs:35-41` grep → `MAX_EVENTS_PER_COMMAND = 22` 확인.
2. spec L221~230 worst-case 산출 재대조: Phase 2 본체 변경 0 (3축→4축 = payload 필드만, 이벤트 수 불변) / Phase 2.5 worst ≈ 17 < 22.
3. 회고에 "A5 = 22 안전, 인상 불필요. Phase 2.5분은 산출 가정(실측 아님) 명시" 박제.
**게이트**: 상수 22 확인 + 산출 교차대조 완료.

### 작업 5 — Phase 2 checkpoint report (`phase2-checkpoint-report.md` 신규)

Phase 2 전체(Stage 0~6) 종합. 박제 필수:
- Stage별 종결 요약 (1~6) + 각 회고 포인터
- D1~D6 baseline 최종 대조표 (Phase 1 → Phase 2 종결)
- B-D 12개 결정 최종 상태 + Phase 2.3 위임된 것 표시
- 4축 도메인 안정화 결과 (closeness→affinity 이행, ±100 wire, B-D12 등)
- Stage 5 디자이너 잔여 (게이트 3·4 = intensity/S1~S3/S4) 미해결 상태 명기
- 부채: examples phase5b 빌드 / closeness·power 잔존 12+ / 커스텀 Deserialize → 전부 Phase 2.3 이후

### 작업 6 — KICKOFF 갱신 (S6-D3·D5, `PHASE2.3-KICKOFF.md` *갱신*)

기존 157줄(Stage 3본 A~E) **보존하며 추가**:
- §3 baseline: Stage 3 메트릭 → **Stage 5 종결값으로 갱신** (843P/0F/2I, D2 26.35/38.92/30.74, D4 8.29, D3 3밴드)
- **신규 인계 5항 섹션 추가** (S6-D3): result.json dump 부재 / intensity 0.4 잠정 / S1~S3 narrative 타당성(Stage5 §6.2) / S4 정성·시간분산(Stage5 §6.3) / Stage 4·5 메트릭
- §A 플래그 추가 (S6-D5): "spec L508 'closeness/power production 0' vs 실측 12+ — Phase 2.3 §A 착수 시 정확 위치 재카탈로그 필요. worldbuilding/markdown은 별 의미(장소 인접성) 가능성 분해 필요"
- §E 갱신 (S6-D5): "Stage 4 미처리 확정(state.rs:666~671 커스텀 Deserialize 존재) → Phase 2.3 인계 유지" (완료 표기 금지)
- **역대조 게이트 (C6-2)**: Stage 4 회고 §5 + Stage 5 회고 §6·§7 잔여표 ↔ KICKOFF 항목 1:1 매핑 확인 (누락 0).

**추가 산출 (spec §범위6)**: `data/scenarios/appraise-validation/` 디렉토리 신설 + README 초안 (Phase 2.3 시뮬 시나리오 set 자리). `task-rel-phase2.3-appraise-tuning.md` spec *초안*(헤더+범위 골격만 — 본체는 Phase 2.3 진입 시).

### 작업 7 — 외부 문서 인덱스 동기화 + 회고

- `CLAUDE.md`: Mind Architecture Phase 2 ✅ 표기 (Stage 5에서 "Stage 6 책임"으로 위임됐던 것 — 여기서 처리)
- `00-roadmap.md`: §5 Phase 2 완료 표기 + Phase 2.3 행 신설 + §6.5 진척 갱신
- spec `task-rel-phase2-domain-migration.md`: v1.0 frozen 표기 + §7 Stage 6에 "→ FROZEN: 본 문서" 포인터
- 회고 `phase2-stage6-bench-handoff.md`: S6-D1~D5 + 작업성격변경 4건 + D baseline 최종 + ±100 vs ±1.0 boundary diagram (spec L399 요구) + 부채 박제

---

## §3. 진입 baseline 박제값 (Stage 6 진입 = Stage 5 종결, 실측 완료)

**측정 명령 (S5-D5 승계, 고정)**: `cargo test --lib --tests --bins`

| 항목 | 진입값 | 비고 |
|---|---|---|
| exit / failed | 0 / **0** | ✅ green (S6-D1 정본) |
| passed | 843 | Stage 5 진입 838 → +5 (S1~S3 3 + daily·shanshenmiao 해제 2) |
| ignored | 2 | Stage 5 진입 4 → -2 (작업 1·2 #[ignore] 해제) |
| result 묶음 | 65 | +1 (phase2_narrative_test.rs) |
| 미스터리 증감 | 0 | 모든 증감 Stage 5 신규로 설명됨 |
| D2 chitchat/significant/legacy | 26.35 / 38.92 / 30.74 µs | 임계 29/42/35.2 전부 이내 |
| D4 10turn×10000 | 8.29 µs/call | Phase 1 8.36 대비 -0.8% |
| D3 3밴드 | 0.000 / 0.461 / 0.980 | Stage 5 종결값 exact |
| git HEAD | 5b2b798 (PR #92) | 작업트리 코드 변경 0 |
| raw 로그 | `baselines/stage6-prep-RAW.txt` | 진입 직전 재측정 (이미 생성) |

**기존 부채 (진입 차단 아님 — Phase 2.3 이후 / 회고 §부채 박제)**:
- examples `phase5b_checkpoint2_eval` 빌드 실패 (Phase 5 산출물, Stage 6 무관 — `--lib --tests --bins`로 회피, exit=0)
- closeness/power src 잔존 12+ (spec L508 "0" vs 실측 불일치 → KICKOFF §A 재카탈로그 → Phase 2.3 §A)
- state.rs:666~671 커스텀 Deserialize impl + 5 테스트 (Stage 4 미제거 → Phase 2.3 §E)
- Stage 5 디자이너 잔여: intensity 0.4 / S1~S3 narrative 타당성 / S4 정성 (게이트 3·4 — Phase 2.3+ 디자이너)

---

## §4. 실행 순서 + 종결 게이트

**순서**: 작업1·2 (동시 — 같은 bench 실행) → 작업3 (전체회귀) → 작업4 (A5) → 작업5 (checkpoint) → 작업6 (KICKOFF) → 작업7 (외부문서+회고).
- 작업 1~4 = 측정·검증 (코드변경 0). 작업 5~7 = 문서.
- 작업 6 KICKOFF 갱신은 작업 1~3 실측값을 §3 baseline에 반영하므로 측정 후 진행.

**Stage 6 종결 게이트 (spec §7 게이트 1~5 + S6 재해석)**:
1. ☐ **게이트1 (S6-D2)**: D2 latency 3케이스 ±20% 이내 + 4축 영향 박제 (작업1).
2. ☐ **게이트2 (S6-D2)**: bench 재측정 완료 + 로그 박제 (작업1·2).
3. ☐ **게이트3 (S6-D1)**: 전체회귀 failed=0 ∧ ≥843P ∧ ≤2I ∧ 증감 설명가능 + D3 3밴드 보존 (작업2·3).
4. ☐ **게이트4**: Phase 2.3 진입 준비 완료 — KICKOFF 갱신(역대조 누락0) + appraise-validation 디렉토리 + phase2.3 spec 초안 (작업6).
5. ☐ **게이트5**: 외부 문서 동기화 완료 (CLAUDE.md / 00-roadmap.md / spec frozen) (작업7).

**Phase 2 종결 선언 조건**: 게이트 1~5 전부 ☑ → Phase 2 (4축 도메인 안정) 완료. 다음 = Phase 2.3 (appraise 정비).

**전체 종결 시 산출**:
- `baselines/stage6-{d2d4-bench,final}-<date>.log`
- `phase2-checkpoint-report.md` (신규)
- `PHASE2.3-KICKOFF.md` (갱신 — Stage 4·5 인계 추가)
- `data/scenarios/appraise-validation/` + README (신규)
- `task-rel-phase2.3-appraise-tuning.md` (초안)
- `phase2-stage6-bench-handoff.md` (회고)
- CLAUDE.md / 00-roadmap.md / spec v1.0 frozen (갱신)

**Claude Code 인계 주의**:
- S6-D1~D5 **변경 금지**. 의문 시 중단·확인, 임의 재해석 금지.
- 코드 변경 0 Stage — closeness/power·Deserialize·examples 부채 *건드리지 말 것* (S6-D5: Phase 2.3 트리거 조기 발파 위험).
- D2 초과 시: 회귀 vs 예상 4축비용 분해 후 보고 (단 진입 실측 전부 통과라 red 낮음).
- 디자이너 git 직접 — Claude Code는 commit 메시지 텍스트만, git 명령 실행 금지.
- 작업 게이트 실패 시 다음 진행 금지·원인 보고.
