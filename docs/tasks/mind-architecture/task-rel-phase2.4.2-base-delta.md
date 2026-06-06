# Phase 2.4.2 — base_delta 4셀 ([MOD-1·2·3] 일괄)

> **상태**: 🟢 FROZEN + 실행 완료 (2026-06-07, 확인① 승인). 확인② 검증 대기.
> **선행**: Phase 2.4.1 종결 (PR #95 `842e0c7`). 4축 도메인 안정.
> **그룹**: §4.2 (base_delta 48셀 lookup) — 단일 책임, 회귀 신호 1회 격리.

## 1. 범위

S1~S2 narrative 검토에서 작가(Bekay)가 도출한 수정입력 [MOD-1·2·3] 일괄 적용.
정확히 **4셀** 변경. HEXACO 보정·intensity·OCC 분류 불변.

### 1.1 입력 디자인 문서 freeze

- `narrative-review-mod-log.md` — [MOD-1·2·3] 확정값 (S1~S4 종합 §"확정 수정입력")
- `00-roadmap.md` v0.10 §Phase 2.4.2
- `relationships.md` §4.2 (정본 48셀 표, v0.7)

## 2. 변경 본체 — 4셀

`src/domain/relationship/mapping.rs::base_delta()`:

| 감정 | 축 | 현재 → 신 | MOD | 근거 |
|---|---|---|---|---|
| Gratitude | trust | 20.0 → **15.0** | MOD-1 | trust:affinity 2:1 과함 → 1.5:1 (15:10) |
| Reproach | wariness | 10.0 → **15.0** | MOD-2 | 배신 후 경계심 과소 |
| Hate | wariness | 15.0 → **20.0** | MOD-2 | 동상 |
| Anger | respect | 0.0 → **−10.0** | MOD-3 | 분노에 인격 격하 동반 |

**원복**: Anger.wariness는 25.0 그대로 (MOD-2에서 +30 시도 → +25 원복, 분노 전역 경계심 부작용 회피).
나머지 44셀 불변.

## 3. Blast radius — 박제값 재측정 대상 (실측 확정)

### 3.1 `src/domain/relationship/mapping.rs` (변경 본체 + 단위테스트)

base_delta 4셀 외 깨지는 테스트 (단정값 + 손계산 주석 동반 갱신):

| 테스트 | 변경 |
|---|---|
| `base_delta_gratitude` | trust 20.0 → 15.0 |
| `base_delta_anger` | respect 0.0 → −10.0 |
| `base_delta_reproach` | wariness 10.0 → 15.0 |
| `base_delta_hate` | wariness 15.0 → 20.0 |
| `base_delta_sum_anger_hate_reproach_matches_s2` | respect −30.0 → **−40.0**, wariness 50.0 → **60.0** (주석 합산값 동반) |
| `update_axes_neutral_hexaco_uses_base_delta_and_intensity` | Gratitude trust 20 → 15 (단정 + 주석) |
| `update_axes_s2_lin_chong_anger_alone` | respect 30.0 → **18.6** (Anger respect −10×0.95×1.2=−11.4; 주석 "respect 0 * ... = 0" → "−10×0.95×1.2=−11.4") |

**불변 (편집 0)**: W1 가드 `beat_rel_modifiers_affinity_channel_after_anger`·`beat_rel_modifiers_trust_channel_after_anger` — Anger의 trust(−25)·affinity(−10) 셀 불변 → 15.8/28.6 유지 → 통과.

**⚠ Stage 0 누락 → 실행 중 발견·정정**: `src/application/command/policies/relationship_policy.rs` `handler_v2_tests::dialogue_end_applies_4_axes_with_gratitude` — Gratitude 1.0, trust 20→40 박제. signature 숫자(64.4 등)가 아니라 grep 미포착. 신값: after_trust 40→35 (Gratitude trust 15 반영, neutral npc modifier 1.0). 동 모듈 `dialogue_end_skips_pride_but_applies_anger` 주석 `respect: 0`→`-10` stale 동반 정정(단정 미포함 — 통과 유지).

### 3.2 `tests/phase2_narrative_test.rs` (S1·S2·S3)

expected 튜플 + 손계산 주석 갱신:

| 케이스 | 현재 → 신 | 바뀌는 축 |
|---|---|---|
| S1 | (64.4, 46.0, 32.0, 0.0) → **(60.8, 46.0, 32.0, 0.0)** | trust |
| S2 | (3.8, 3.0, −4.0, 42.5) → **(3.8, 3.0, −13.0, 50.5)** | respect, wariness |
| S3 | (25.216, 26.64, −1.76, 32.32) → **(25.216, 26.64, −5.12, 34.28)** | respect, wariness |

S3 trust/affinity는 25.216/26.64 정밀도 그대로 (불변 축).

### 3.3 `data/scenarios/appraise-validation/{S1,S2,S3}.json` (박제 필드)

| 파일 | 필드 | 변경 |
|---|---|---|
| S1 | `_expected_axes_delta.trust` | "+14.4 (… × +20 × 1.2)" → "+10.8 (… × +15 × 1.2)" |
| S1 | `_expected_final_axes.trust` | 64.4 → 60.8 |
| S2 | `_expected_axes_delta.respect` | "−24 (Reproach −20 + Hate −4 + Anger 0)" → "−33 (… + Anger −9)" |
| S2 | `_expected_axes_delta.wariness` | "+42.5 (Reproach +8 + Hate +12 + Anger +22.5)" → "+50.5 (Reproach +12 + Hate +16 + Anger +22.5)" |
| S2 | `_expected_final_axes` | respect −4.0 → −13.0, wariness 42.5 → 50.5 |
| S2 | `_pd_c1_anchor_case` | `delta_above_threshold_axes` respect(−24)→(−33)·wariness(+42.5)→(+50.5); `current_dominant_label_only` "[trust Δ=-46.20]" → **"[wariness Δ=+50.50]"** (dominant 뒤집힘 ↓3.4); `discussion` 라벨 예시·수치 동반 |
| S3 | `_expected_axes_delta.respect` | "−11.76 (… Anger 0)" → "−15.12 (… Anger −3.36)" |
| S3 | `_expected_axes_delta.wariness` | "+12.32 (… Reproach +3.92 …)" → "+14.28 (… Reproach +5.88 …)" |
| S3 | `_expected_final_axes` | respect −1.76 → −5.12, wariness 32.32 → 34.28 |

### 3.4 정본 `docs/game-design/2-characters/relationships.md` §4.2 표

(열 순서 = 감정 | trust | affinity | respect | wariness)

- L344 `| Gratitude | +20 |` → `+15`
- L345 `| Anger | … | 0 | +25 |` → respect `−10`
- L347 `| Reproach | … | +10 |` → wariness `+15`
- L355 `| Hate | … | +15 |` → wariness `+20`

### 3.5 실행 후 doc (확인② 이후)

- `narrative-review-mod-log.md`: [MOD-1·2·3] 상태 🟡→🟢, S1~S3 박제 예정값 → 확정 표기
- `00-roadmap.md`: §Phase 2.4.2 종결 마커 + §6.5 §4.2 행 갱신

## 4. 영향 없음 분석 (회귀 아님 확인)

- **PAD 벤치 20케이스**: base_delta는 OCC→4축 경로. utterance→PAD(BGE-M3 임베딩) 경로와 구조 독립 (2.4.1 G3와 동일). **기대값 변경 불요, 보존 확인만**.
- **D3 3밴드 significance** (0.000/0.461/0.980): `compute_significance`는 turn signal 기반, base_delta 무관 → 불변. (mod-log "재측정" 표기는 과보수 — 실측상 무관.)
- **dominant_delta 뒤집힘 = 테스트 미박제**: `memory_relationship_cause_test.rs:377`은 `content.contains("Δ=")` 라벨 *존재*만 검증 (어느 축인지 안 박음) → 깨지지 않음. 단 게임 내 S2 관계기억 텍스트가 trust→wariness 라벨로 *바뀜* (의도된 행동 변화, JSON `_pd_c1_anchor_case`에 반영).

## 5. 실행 순서 (Claude Code 핸드오프)

1. `mapping.rs` base_delta 4셀 수정 (§2)
2. `mapping.rs` 단위테스트 7건 + 주석 갱신 (§3.1)
3. `cargo test --lib` → 신값 산출 확인, mod-log 손계산과 일치 검증
4. `tests/phase2_narrative_test.rs` 3건 + 주석 갱신 (§3.2)
5. `data/scenarios/appraise-validation/{S1,S2,S3}.json` 갱신 (§3.3)
6. `relationships.md` §4.2 4셀 (§3.4)
7. 전체 회귀 + PAD 벤치 보존 확인 (§6)
8. 의도 파일만 stage → commit (push 보류, 명시 지시 시만)

## 6. 검증 게이트

1. `cargo test --lib` 회귀 0 (554P 유지 + 변경 테스트 신값 통과)
2. `cargo test --test phase2_narrative_test` S1~S3 신 박제값 통과 (tol ±0.5)
3. PAD 벤치 20케이스 = `pad-anchor-score-matrix.md` 잠긴 기대값 보존 (deviation 0)
4. (통합) `cargo test --lib --tests --bins` 0F

## 7. 신 박제값 (mod-log 산정 = 손계산 재검증 완료)

손계산 (확정):
- S1 trust: 50 + (0.6×15×1.2)=10.8 → **60.8**
- S2 respect: 20 + (0.8×−25 + 0.8×−5 + 0.9×−10)=−33 → **−13.0**; wariness: 0 + (0.8×15 + 0.8×20 + 0.9×25)=50.5 → **50.5**
- S3 respect: 10 + (0.7×−5 + 0.7×−25 + 0.6×−10)×0.56=−15.12 → **−5.12**; wariness: 20 + (0.7×15 + 0.6×25)×0.56=14.28 → **34.28**

narrative 테스트 원칙 = "코드 산출 = ground truth" → step 3에서 cargo 신값이 위 손계산과 일치 확인 후 박제.

## 변경 이력

| 버전 | 일자 | 변경 |
|---|---|---|
| v0.1 | 2026-06-07 | 초안. Stage 0 사실조사 후 작성. 확인① 동결 대기. |
| v0.2 | 2026-06-07 | 🟢 확인① 승인 → 실행 완료. 4셀 + 박제 5종 갱신. 게이트: `cargo test --lib` **554P/0F** + narrative 3P + mapping 37P. Stage 0 누락 1건(relationship_policy gratitude 테스트) 실행 중 발견·정정(§3.1). |
