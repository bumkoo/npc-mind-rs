# Narrative 검토 수정입력 누적 로그 (S1~S4)

> **목적**: Phase 2.3 종결 후, S1~S4 narrative 박제값을 작가(Bekay)가 검토하며
> 발견한 *엔진 튜닝 수정입력*을 누적 기록. 발견 시마다 개별 task 띄우지 않고
> 여기 모은 뒤, S1~S4 전수 검토 완료 후 **한 task로 일괄 처리**
> (정본 §4.2/§4.3 다회 수정·다회 회귀측정 방지 — 회귀 신호 1회 격리).
>
> **상태**: 🟡 누적 중 (S1 완료 / S2~S4 미검토)
> **선행**: Phase 2.3 종결 (git `676185c`). 본 로그는 Phase 2.3 비스코프 — 후속 별도 task 입력.
> **주의**: 본 문서는 *수정입력 수집*만. 실제 코드/정본 변경은 별도 task의 확인①
> 동결 후 Claude Code 실행 단계에서. 여기 기록 = 미적용 설계 의도.

---

## 분리 원칙 (회귀 신호 격리 — Phase 2.3 교훈 계승)

수정입력은 변경 대상 정본 절로 분류. 서로 합치지 않음 (한 번에 한 종류):

- **그룹 §4.2** — base_delta 48셀 lookup 테이블 (감정별 4축 기본 변동)
- **그룹 §4.3** — HEXACO 보정자 6룰 (계단/비례 등)
- **그룹 기타** — OCC 분류, intensity, 시나리오 입력값 등

각 그룹은 독립 task로 처리 가능 (정본 절이 다르고 회귀 측정 격리 필요).

---

## 누적 수정입력

### [MOD-1] §4.2 — base_delta(Gratitude) trust:affinity 비 완화

| 항목 | 내용 |
|---|---|
| **발견** | S1 (임충→노지심, Admiration+Gratitude) 박제값 검토 |
| **그룹** | §4.2 (base_delta 테이블) |
| **현재** | `base_delta(Gratitude)` = trust **+20** / affinity **+10** / respect 0 / wariness −10 → trust:affinity = **2:1** |
| **작가 판정** | 개인차(HEXACO 보정) 제외 시 trust:affinity 2:1은 과함. "목숨 빚 진 의형제"인데 친밀감 대비 신뢰가 2배 과증가. **1.5:1**이 적합. |
| **확정 방식** | **(가) trust 하향**: trust 20 → **15** (affinity 10 유지) → 15:10 = 1.5:1. *신뢰 증가폭 자체를 억제* (방식 (나) affinity 상향은 기각 — 신뢰 상승폭이 과했다는 판정이므로). |
| **S1 영향** | trust Δ +14.4 → **+10.8** (sincerity ×1.2 적용 후: 15×0.6×1.2). 최종 trust 64.4 → **60.8**. affinity/respect/wariness 불변. |
| **전역 영향** | `mapping.rs` base_delta(Gratitude) 셀 + `relationships.md §4.2` 정본 48셀 표 1셀 수정. Gratitude 등장 **모든 시나리오** trust Δ 변동. W1 가드/D3 3밴드 재측정·재조정 동반(의도된 변경, 회귀 아님). |
| **상태** | 🟡 수정입력 박제 (미적용). 별도 §4.2 task 확인① 동결 후 적용. |

### [MOD-2] §4.2 — base_delta wariness 상향 (Reproach/Hate/Anger)

| 항목 | 내용 |
|---|---|
| **발견** | S2 (임충→육겸, 산신묘 배신) 박제값 검토 |
| **그룹** | §4.2 (base_delta 테이블) |
| **현재** | wariness base: Reproach **+10** / Hate **+15** / Anger **+25** → S2 wariness Δ = +42.5 (Rep 8 + Hate 12 + Anger 22.5), 최종 wariness 42.5 (≈43%) |
| **작가 판정** | 본인 암살 음모 확인 후 경계심 ≈43%는 너무 작음. 이상적으로 75%. 단 75%는 base만으로 강제 시 Anger 등장 *전 시나리오* wariness 과다 위험 → S2 단건 위해 정본 base를 75 맞춤은 보류. |
| **확정 변경** | Reproach +10 → **+15**, Hate +15 → **+20**, **Anger +25 → +25 (원복 — 변경 없음)**. (Bekay 최종 수정: Anger wariness는 현행 25 유지. Anger 전역 영향 회피 — Reproach·Hate만 상향.) |
| **S2 영향** | wariness Δ = Rep(15×0.8=12) + Hate(20×0.8=16) + Anger(25×0.9=22.5) = **+50.5**. 최종 wariness 42.5 → **50.5** (≈50%). E+ Anxiety 미발동(anxiety 0.4<0.5) 확인. 75 미달이나 Anger 원복으로 전역 영향 최소화 우선. |
| **전역 영향** | `mapping.rs` base_delta **2셀(Reproach·Hate wariness)** + `relationships.md §4.2` 정본 표 2셀. Anger wariness는 불변이라 분노 시나리오 wariness 영향 없음 (Anger 원복 효과). Reproach/Hate 등장 시나리오만 wariness Δ 상승. |
| **Anger 원복 메모** | Anger wariness +30 시도 → +25 원복(Bekay). 분노가 전 시나리오 경계심을 키우는 부작용 회피. S2 wariness는 Rep·Hate 상향만으로 50.5(현 42.5 → +8). 75 목표는 미달 — base 절제 우선, S2 극단성은 별도(intensity/Phase 2.5) 처리 영역으로 잔류. |
| **상태** | 🟡 수정입력 박제 (미적용). §4.2 task 확인① 동결 후 적용. [MOD-1]과 동일 §4.2 그룹 — 같은 task에서 일괄 처리. |

### [MOD-3] §4.2 — base_delta(Anger) respect 격하 추가 (−10 확정)

| 항목 | 내용 |
|---|---|
| **발견** | S2 박제값 검토 — respect 20→−4.0 (배신인데 인격 경멸 −4밖에 안 됨) |
| **그룹** | §4.2 (base_delta 테이블) |
| **분해** | S2 respect Δ −24 = Reproach(−25×0.8=−20) + Hate(−5×0.8=−4) + **Anger(0×0.9=0)**. respect를 깎는 건 사실상 Reproach 혼자, **Anger respect base = 0** (분노가 인격을 안 깎음). |
| **작가 판정** | 방식 **(가) 채택** — "분노에는 *그 인간 자체에 대한 격하*가 동반돼야 한다". 현 Anger가 trust·wariness만 건드리고 respect=0인 건 "분노했으나 경멸은 안 함"이라는 어색한 결과. (방식 (나) Reproach respect 강화는 기각.) |
| **확정 변경** | `base_delta(Anger)` respect: 0 → **−10** (Bekay 확정, S2·S3 양측 검토 후). S2(원수) 최종 respect −13.0(강한 경멸 OK) / S3(사제) 최종 respect −5.1(약한 경멸 OK) — 양 시나리오 동시 타당 확인. |
| **S2 영향** | respect Δ −24 → −33 (Anger −10×0.9=−9 추가). 최종 respect −4.0 → **−13.0**. |
| **S3 영향** | respect Δ −11.76 → −15.12 (Anger −10×0.6×0.56=−3.36 추가). 최종 respect −1.76 → **−5.12**. |
| **전역 영향** | Anger 등장 모든 시나리오 respect Δ 동반 하락. [MOD-2]와 같은 §4.2 Anger 셀 — 동일 task에서 함께 변경. |
| **상태** | 🟢 값 확정 (−10). §4.2 task에서 [MOD-1·2]와 일괄 적용. |

---

## S1 검토 종결 메모

S1 판정 = **(B) 어색 → 수정입력 [MOD-1] 도출**. S1 박제값
`(64.4, 46.0, 32.0, 0.0)`은 **확정 아님 — [MOD-1] 적용 후 재박제 예정**
(예상 `(60.8, 46.0, 32.0, 0.0)`).

## S2 검토 종결 메모

S2 판정 = **(B) 어색 → [MOD-2](wariness) + [MOD-3](Anger respect=−10 확정) 도출**.
부분 타당 확인: ① `base_delta(Reproach)` trust −10 기여 = 타당(작가 확인, MOD 없음).
② respect 음수 진입(−4.0) = clamp 무관 정상(AxisScore ±100), 단 *강도가 약함* → [MOD-3].
S2 박제값 `(3.8, 3.0, −4.0, 42.5)`은 **확정 아님 — [MOD-2·3] 적용 후 재박제 예정**:
[MOD] 전부 반영 시 S2 ≈ `(3.8, 3.0, −13.0, 50.5)` (trust/affinity는 [MOD] 무관 불변;
wariness는 Anger 원복(25)으로 50.5 — Rep 12 + Hate 16 + Anger 22.5).

## S3 검토 종결 메모

S3 판정 = **(A) [MOD 반영값] 그럴듯 → 확정** (별도 신규 MOD 없음).
메커니즘 3중 교차검증 정합(base_delta 3셀 · HEXACO 3룰 곱셈누적 · S3.json _modifier_compound).
[MOD-2·3] 반영 재계산 (Anger wariness 25 원복 반영): trust/affinity는 [MOD] 무관 불변.
- trust 25.22 (불변) / affinity 26.64 (불변)
- wariness 32.32 → **34.28** ([MOD-2] Reproach wariness 10→15만 반영, Anger 25 원복.
  계산: [Rep 15×0.7 + Anger 25×0.6]×0.56 = 25.5×0.56 = +14.28, 초기 20)
- respect −1.76 → **−5.12** ([MOD-3] Anger respect −10 반영분)
특기: ① Pity(동정) affinity +10이 Reproach/Anger 감소를 상쇄 → affinity 거의 유지(−3.36)
= "제자가 미워도 정은 남는 사부의 한" — 의도된 표현으로 작가 수용. ② 수련 절제 보정
(patience 0.9 × prudence 0.8 → ×0.56, trust는 ×0.672)이 모든 변동을 절반으로 누름 =
"절제의 화신" 정확한 구현으로 작가 수용 (과하게 눌리지 않음 판정).
S3 박제값 = [MOD] 반영 후 `(25.22, 26.64, −5.12, 34.28)`로 재박제 예정 (별도 어색 없음).
다음: S4 (마지막 — 임충→고구 누적 분노 / 시간분산).

## S4 검토 종결 메모

S4 판정 = **정성 케이스 — 정량 박제 보류 (판정 대상 아님)**.
코드 실측 확정 사실:
- 사건별 intensity 도출 = 현 엔진 가능. `appraise()` → `action::appraise`(Reproach
  = |praiseworthiness| × HEXACO weight × 관계 modifier) + `event::appraise`(Distress)
  → `compound::appraise` → **Anger = (Reproach + Distress) / 2** (helpers.rs:60-63,
  단 val1>0 && val2>0 조건). praiseworthiness는 `ActionInput` DTO 외부 입력
  (시나리오/MCP API, 엔진이 도덕판단 안 함 — 작성자/LLM이 제공).
- 4축 누적 = 현 엔진 가능 (AxisScore.add_delta, clamp ±100). S1~S3이 그 증거.
- 즉 "사건마다 감정 도출 → Δ 4축 누적" = **현 엔진 작동** (작가 직관 정확).
- S4 검증 BLOCKED 진짜 사유: ⑴ `phase2_narrative_test.rs` 단일 시점 set_intensity
  주입 형식 — 다사건 praiseworthiness 시퀀스 입력란 없음 (실게임 dispatch_v2는
  사건마다 호출되니 가능, **검증 형식만의 한계**) ⑵ axis_modulation 부재
  (누적상태 의존 Δ 변조 — 9번 쌓인 위 10번째 폭발/체념) ⑶ 시간/매개(권력거리) 모델 부재.
- **로드맵 실측 정정**: S4가 요구하는 "다사건 시퀀스 → 시간축 누적 곡선"은
  Phase 2.5(axis_modulation = 단일 reflection ±5 미세조정)·Phase 3a(Channel 2
  Temporal = BondKind 경과일 카운터) **어느 쪽에도 그 형태로 미설계**. S4.json
  "Phase 2.5 이관" 표기는 간극 있음.
- **Bekay 결정**: 다사건 시간축 누적 모델 = **버전 1.0 이후로 보류 (당장 불필요,
  본 검토에 미반영)**. S4는 "정성 케이스, 정량 보류" 그대로 종결. 추가 설계 안 함.

---

## S1~S4 narrative 검토 — 종합 (세션 종결)

| 케이스 | 판정 | 도출 |
|---|---|---|
| S1 임충→노지심 (보은) | (B) 어색 | [MOD-1] Gratitude trust 20→15 (2:1→1.5:1) |
| S2 임충→육겸 (배신) | (B) 어색 | [MOD-2] Reproach·Hate wariness ↑(Anger 원복) + [MOD-3] Anger respect 0→−10 |
| S3 수련→옥교룡 (사부의 한) | (A) 수용 | 신규 MOD 없음 ([MOD-2·3] 자연 반영, 작가 수용) |
| S4 임충→고구 (누적분노) | 정성 보류 | Phase 2.5 이관(간극 명시), 1.0 이후 |

**확정 수정입력 = [MOD-1·2·3], 전부 §4.2 base_delta 그룹. 변경 = 정확히 4셀**:
Gratitude.trust 20→15 / Reproach.wariness 10→15 / Hate.wariness 15→20 /
Anger.respect 0→−10. (Anger.wariness는 +30 시도 후 +25 원복 — 변경 없음.)

**[MOD] 반영 후 박제 예정값**:
- S1 `(64.4,46.0,32.0,0.0)` → `(60.8, 46.0, 32.0, 0.0)`
- S2 `(3.8,3.0,−4.0,42.5)` → `(3.8, 3.0, −13.0, 50.5)`
- S3 `(25.22,26.64,−1.76,32.32)` → `(25.22, 26.64, −5.12, 34.28)`
- S4 = 박제값 없음 (정성)

→ 본 narrative 검토 세션 **종결**. [MOD-1·2·3]은 미적용 설계의도 — 별도 §4.2
task(Stage 0 → 확인① → Claude Code → 확인②)에서 일괄 적용 + 전 시나리오
회귀 재측정 + W1/D3 가드 재조정. **본 세션은 수정입력 도출까지가 범위.**

---

## [관찰: HEXACO 이중 개입] — 신규 검토 task 필요 (별도 세션)

**발견 (코드 확정)**: HEXACO가 한 사건 처리에서 **두 번** 개입:
- **개입 1 (intensity 산출)**: `personality.rs:608 praiseworthiness_weight`
  = conscientiousness 평균 + gentleness(A facet) → 감정 *강도*에 곱
- **개입 2 (4축 변환)**: `mapping.rs:138 hexaco_modifier` 6룰
  (sincerity→trust×1.2 등) → 관계 *변화량*에 곱
- 같은 성격이 ①감정 생성 강도 + ②관계 변환 양쪽에 **곱셈으로 이중 적용**.
  S3(수련)이 극단 예시 — patience가 intensity(gentleness 경유)·4축(all×0.7)
  두 단계 모두 감쇠 → 의도보다 과하게 눌릴 가능성.

**우려**: 이중 감쇠/증폭, 개념 중복(같은 심리기제 2회 카운트 의심).
지금까지 narrative 검토는 intensity 직접 주입이라 **개입1이 사각지대**였음.

**→ 별도 task: `OCC 정의 → 감정(강도) → HEXACO 관계변환` 전 경로 검증.**
- 범위: 검토 only + 필요 시 설계 문서(도메인 규칙 수준)까지.
  ① 현재 구현 코드 정리 (도메인 규칙 수준, **엔진 시뮬레이션으로 확인 필수**)
  ② 수정 설계 문서 (도메인 규칙 수준) — 필요 시
- 케이스: 기존 3 (S1~S3, 단 intensity도 엔진 도출 방식 재구성) + **신규 3**
  (동일 상황·성격만 다른 대비군 — 이중 개입을 드러내는 설계, 작가 입력 필요)
- Phase 2.5 **진입 전** 수행 (appraise 안정 baseline 검증 목적).
- 본 세션에서 task 착수 안 함 — 다음 세션에서 시작 (착수 방안은 본 세션 말미 별도 정리).
