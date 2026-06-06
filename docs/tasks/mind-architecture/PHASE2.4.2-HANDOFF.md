# Phase 2.4.2 전달 사항 (새 세션 진입용)

> 작성 2026-06-06. 직전 세션(2.4.0·2.4.1 종결)에서 다음 세션(2.4.2)으로 인계.
> **먼저 읽을 것**: [00-roadmap.md](00-roadmap.md) Phase 2.4 절 — 2.4.2 스코프·전체 sub-stage 상태.

## 1. 현재 상태 (2026-06-06 기준)

- **2.4.0 종결** — HEXACO 이중개입=의도된 누적 확정. 정본 [phase2.4.0-hexaco-double-application-review.md](phase2.4.0-hexaco-double-application-review.md).
- **2.4.1 종결** — intensity weight 튜닝. 정본 [task-rel-phase2.4.1-intensity-weight.md](task-rel-phase2.4.1-intensity-weight.md). PR #95 main 머지(`842e0c7`). `cargo test --lib` 554P/0F, 회귀 0.
- **main 최신 commit**: `56a9b7b`. **push 보류** — Bekay 지시 시 push.
- 남은 sub-stage: **2.4.2(다음)** → 2.4.3(최고 침습) → 2.4.4.

## 2. 2.4.2 스코프 (base_delta 4셀, 상수 변경)

출처: [narrative-review-mod-log.md](narrative-review-mod-log.md) [MOD-1·2·3]. `base_delta` 테이블(감정→4축 기본 변동)에서:

| 감정.축 | 현재 → 변경 |
|---|---|
| Gratitude.trust | 20 → **15** |
| Reproach.wariness | 10 → **15** |
| Hate.wariness | 15 → **20** |
| Anger.respect | 0 → **−10** (Anger.wariness는 25 **원복**) |

위치: 관계 4축 변환 테이블 — `mapping.rs`(Stage 0에서 정확 경로·현재값 grep 확인 필수). 침습도 낮음(상수).

## 3. ★ 2.4.1과 결정적으로 다른 점 — S1~S4 영향 있음

- 2.4.1은 **①강도 산출**(personality weight)이라 S1~S4(강도 preset)가 **무관**했음.
- **2.4.2는 ②변환**(`4축변동 = base_delta × 강도 × hexaco_modifier`). **S1~S4 appraise-validation은 바로 이 ②변환을 검증**(intensity preset + `_expected_axes_delta` 박제). → **2.4.2는 S1~S4 `_expected_axes_delta` 박제값을 반드시 재측정·갱신해야 함.** (이번엔 갱신이 스코프 내 정상 작업.)
- **PAD 벤치**(`pad-anchor-score-matrix.md`)는 임베딩(utterance→PAD) 경로라 base_delta와 **여전히 독립 → 무영향**.

## 4. Baseline (D) — 2.4.2 진입 시

- `cargo test --lib` **554P/0F** (2.4.1이 추가/수정한 테스트는 `tests/` 통합이라 --lib 수 불변).
- 유지해야 할 가드: `tests/phase241_intensity_weight_test.rs`(7P), `tests/personality_test.rs`(18P).
- 재측정 대상: `data/scenarios/appraise-validation/S1~S4.json`의 `_expected_axes_delta` + 관련 narrative 테스트.

## 5. 협업 프로토콜 (Phase 2.3+ 계승)

- 단계: Stage 0 사실조사(grep/cargo) → B-D 결정 → C 위험 → D baseline → FROZEN spec(.md) → Claude Code 핸드오프(KICKOFF.md) → 확인② 검증.
- Bekay 체크인 3회: ⓪ Stage 0 후 방향 / ① FROZEN 직전 동결·인계 / ② 실행 후 검증. 그 사이 자율 진행.
- **한 턴에 한 개념**, 진행 전 확인.
- 보고/확인 → HTML, 정본(FROZEN·결론·KICKOFF) → .md.
- **Git: Claude 직접 커밋**, 의도 파일만 스테이징. **push는 명시 지시 시만.**
- 검증은 커밋 메시지 아닌 **실제 코드 diff + cargo test**로.

## 6. 도구·주의

- 파일: Filesystem MCP / Desktop Commander. 셸/grep: Windows-MCP PowerShell + `Select-String`, **`target\` 제외**.
- 엔진 검증: npc-mind-studio MCP `appraise`(trace 필드로 weight·delta 분리 관찰). **단 코드 변경분은 MCP 서버 재빌드·재시작 후 반영** — 미재빌드 서버는 구 바이너리.
- 통제군 fixture: `data/appraise-test/prudence-intensity-fixtures/scenario.json`(`load_scenario`로 재사용).
- 약자엔 **(설명) 병기** (예: X(외향성), E(정서성), OCC, PAD). Korean은 tool 호출 시 리터럴 UTF-8.

## 7. 진입 첫 액션

1. `mapping.rs` base_delta 테이블 grep → 4셀 현재값 실측 확인 (Stage 0).
2. S1~S4 `_expected_axes_delta` 현 박제값 확보 (재측정 baseline).
3. 방향 ⓪ 보고 → FROZEN spec → Claude Code.
