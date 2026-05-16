# Appraise Validation Scenarios (Phase 2.3+ 신설 자리)

**Status**: 디렉토리 신설만 (Phase 2 Stage 6 작업 6, 2026-05-16).
**용도**: Phase 2.3 narrative 시뮬레이션 (Stage 0 §3.6 S1~S4 + 추가) 검증 시나리오 JSON 박제.

---

## 배경

Phase 2 Stage 5에서 `tests/phase2_narrative_test.rs`로 S1~S3 정량 (S4 정성) 박제했다.
박제값은 코드 실행 출력 = ground truth (S5-D3). 시나리오 JSON은 박지 않고 **인라인 NPC + Relationship 구성**으로 처리 — fixture 신설 0건.

Phase 2.3 진입 시점에서 `axis_modulation` 활성화 + S1~S3 narrative 타당성 검토 + S4 정량화 시도가 진행되면, 다음 산출이 본 디렉토리에 누적된다:

- 시나리오 JSON (situation/focus/intensity/HEXACO/관계 초기값) — S1~S4 각 1파일
- `_expected_axes_delta` prose — 디자이너 narrative 의도 박제
- 결과 파일 (`session_*_result.json`) — Phase 2.3/3에서 자동 dump CLI 신설 후 (B-D9 인프라 부재 → §5.1)

---

## 디렉토리 구조 (Phase 2.3 진입 시 작성 예정)

```
data/scenarios/appraise-validation/
  S1-lin-chong-lu-zhishen.json       임충→노지심 (Admiration + Gratitude)
  S2-lin-chong-lu-qian.json          임충→육겸 (Reproach + Hate + Anger)
  S3-yu-shulien-yujiaolong.json      수련→옥교룡 (Pity + Reproach + Anger)
  S4-lin-chong-gao-qiu.json          임충→고구 (정성, 시간 분산 + 권력 거리)
  README.md                           본 파일
  results/
    S1-result-<git-hash>.json         narrative-dump CLI 출력 (B-D9 인프라 신설 후)
    ...
```

---

## Phase 2 Stage 5 박제값 (참고)

[`tests/phase2_narrative_test.rs`](../../../tests/phase2_narrative_test.rs) 에서 박힌 S1~S3 EXPECTED:

| 케이스 | 초기 (trust/aff/resp/war) | HEXACO modifier | sum delta | EXPECTED |
|---|---|---|---|---|
| S1 임충→노지심 | 50 / 40 / 20 / 0 | trust×1.2 (sincerity 0.6) | +14.4 / +6 / +12 / -6 (war 0 clamp) | (64.4, 46.0, 32.0, 0.0) |
| S2 임충→육겸 | 50 / 40 / 20 / 0 | forgiveness=0.0 A−Forg 미발동 | -46.2 / -37 / -24 / +42.5 | (3.8, 3.0, -4.0, 42.5) |
| S3 수련→옥교룡 | 40 / 30 / 10 / 20 | trust×0.672 / 기타×0.56 | -14.784 / -3.36 / -11.76 / +12.32 | (25.216, 26.64, -1.76, 32.32) |

기여 감정 (base_delta 비-0만):
- S1: Admiration 0.6 + Gratitude 0.6
- S2: Reproach 0.8 + Hate 0.8 + Anger 0.9
- S3: Pity 0.7 + Reproach 0.7 + Anger 0.6

S4는 정량 *제외* (정성, 게이트3 흡수 — S5-D2).

---

## Phase 2.3 진입 시 작업

1. [`PHASE2.3-KICKOFF.md`](../../../docs/tasks/mind-architecture/PHASE2.3-KICKOFF.md) §5.3 narrative 검토 항목 → 디자이너가 어색 판정 시 입력 intensity/HEXACO 조정 → 새 EXPECTED 산출 → 본 디렉토리에 JSON 박제
2. S4 정량화 시도: 시간 분산 모델 + `axis_modulation` 활성화 후 박제
3. `narrative-dump` CLI 신설 (B-D9 인프라): 시나리오 JSON 입력 → result.json dump → `results/` 디렉토리에 박제
