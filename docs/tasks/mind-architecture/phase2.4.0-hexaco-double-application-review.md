# Phase 2.4.0 — HEXACO 이중 개입 검토 (결론)

> 선행 진단 · 게이트. **코드 변경 0.** 2.4.1~2.4.3 튜닝 방향을 확정하는 검토 산출물.
> 검토일 2026-06-06. 실측 도구: npc-mind-studio MCP `appraise` (trace 필드).
> 입력 근거: [05-hexaco.html](../../emotion/05-hexaco.html) · [06-relationship.html](../../emotion/06-relationship.html) · [narrative-review-mod-log.md](narrative-review-mod-log.md) 말미 [HEXACO 이중 개입].

## 0. 결론 요약

- HEXACO는 한 사건에서 **두 번 곱셈**으로 작용: ① 감정 강도 산출 + ② 관계 4축 변환. 사슬 연결(② 입력 = ① 출력).
- **판정: 이중 적용은 중복(버그) 아님 — 의도된 누적.** ①과 ②가 서로 다른 심리 기능이라 정당. → 둘 다 유지.
- 단, ① 내부에서 **결함 1건** 발견: `praiseworthiness_weight`가 성실성 *평균*을 써서 prudence가 Reproach를 부당 증폭. → **2.4.1에서 facet 분해로 해소** (2.4.0 범위 밖, 위임).
- 천장 포화는 calibration 관심사 — 버그 아님. 강도 1.0 천장 클러스터링(곱 누적)은 구조 문제로 **Phase 2.6 deferred** 분리.

## 1. 두 개입 지점

| | 위치 | 역할 |
|---|---|---|
| ① 강도 | `personality.rs` weight 함수군 (`desirability_self_weight` · `desirability_prospect_weight` · `praiseworthiness_weight:608` 등) | 같은 사건을 성격이 얼마나 강하게 *느끼게* 하는가 |
| ② 변환 | `mapping.rs:138 hexaco_modifier` 6 보정룰 | 그 감정이 관계 4축을 얼마나 *바꾸는가* |

사슬: `4축변동 = base_delta(감정) × 강도(①출력) × hexaco_modifier(②)`. → HEXACO가 한 사건에 두 번 곱해짐.

## 2. 역할 경계 규칙 (판정 기준)

**규칙**: ①과 ②의 곱이 정당하려면 **서로 다른 심리 기능**이어야 한다.

- ① = "얼마나 강하게 *느끼는가*" (감정 발생 강도)
- ② = "느낀 것을 관계에 얼마나 *반영하는가*" (관계 갱신 민감도)
- **정당(누적 유지)**: 같은 facet이라도 두 기능이 독립이면 곱 허용. 예 — 신중한 인물이 분노는 크게 느끼되(①↑) 관계는 천천히 끊는다(②↓).
- **중복(한쪽 제거)**: 같은 facet이 같은 방향으로 *동일 기능*을 두 번 세면 정리 대상.

## 3. 판정

1. **② 변환 누적 = 의도 확정** (Bekay, 2026-06-06). `hexaco_modifier` 현행 유지. 신중함이 강도(①)도 변환(②)도 누르는 것은 "둔감한 기질"의 정당한 복합 표현 — 중복 아님.
2. **① 강도 = 정당하나 결함 1건**: `praiseworthiness_weight` 공통항이 성실성 *평균*(`(org+dil+perf+prud)/4 × 0.3`)이라, prudence가 Reproach(타인 비난) weight를 끌어올림. "신중함 → 비난 민감"은 심리 근거가 약한 부수효과. → **2.4.1에서 `diligence×0.10 + perfectionism 비대칭`으로 facet 분해**(org·prud 제외)하여 해소.

## 4. 실측 증거

동일 배신 상황(`desirability_for_self` / `praiseworthiness` = −0.7), 나머지 facet 0, prudence만 변화. MCP `appraise` trace:

| prudence | Distress weight → 강도 | Reproach weight → 강도 |
|---|---|---|
| +0.8 | 0.76 → 0.532 | 1.06 → 0.742 |
| 0 | 1.00 → 0.700 | 1.00 → 0.700 |
| −0.8 | 1.24 → 0.868 | 0.94 → 0.658 |

- **Distress**: `−신중함×0.3` 정확 작동 = 의도 (신중 → 고통 둔감). ① 정상.
- **Reproach**: prudence↑ → 성실성평균↑ → weight↑ (0.658 → 0.742) = **부수효과**. ← 2.4.1 대상.
- 통제군 fixture 영속화: `data/appraise-test/prudence-intensity-fixtures/scenario.json` (NPC 4 + 관계 3). `load_scenario`로 재사용.

## 5. 포화 관찰 (calibration, not bug)

weight clamp `[0.5, 1.5]` + 강도 clamp `[0, 1]` (`types.rs:289·303`) 2겹. 독립 facet이 같은 방향으로 누적되면 천장 도달 → 변별 손실. 2.4.1 계수는 **예산보존**(분해 총량 ≈ 원 0.3)으로 잡아 회피(겸손+완벽주의 0.7/0.7 Shame = 1.42, 천장 미만 확인). 강도 1.0 천장에 *드라마틱 장면이 곱 누적으로 몰리는* 구조 문제는 별건 → **Phase 2.6 (deferred)**.

## 6. 후속

- **2.4.1** — 정서성 weight 재검토 + praiseworthiness facet 분해.
- **2.4.3** — `hexaco_modifier`는 ②로서 유지. 별개인 `RelationshipModifiers`(관계 *상태* 기반 modifier) 재설계는 그대로 진행.
- **Phase 2.6 (deferred)** — 강도 soft-saturation + 표현 밴드 세분화 + floor 대칭 재검토.
