# Phase 2.4.1 — intensity weight 튜닝 (FROZEN spec)

> 🟢 **FROZEN (2026-06-06)**. emotion 도메인 weight 함수 정량 정비.
> 입력 freeze: [05-hexaco.html](../../emotion/05-hexaco.html) + [phase2.4.0-hexaco-double-application-review.md](phase2.4.0-hexaco-double-application-review.md).
> 변경 표면: `src/domain/personality.rs` weight 함수 4개 + `src/ports/personality.rs` trait `AppraisalWeights` 시그니처 1 + 호출부(`src/domain/emotion/appraisal/event.rs`) 1.
> **코드 변경 후 PAD 벤치 잠금 재확정 — Bekay 승인 없이 기대값 변경 금지.**

## 1. 배경

두 결함을 한 묶음(2.4.1)으로 정비. 둘 다 *감정 강도 산출(① 단계)* 의 성격 weight 문제로, PAD 벤치를 공유.

**(A) 정서성(E, Emotionality) 과증폭** — `desirability_self_weight` · `desirability_prospect_weight` · `desirability_confirmation_weight`가 `avg.e.effect(W_STANDARD)`(E 평균 ×0.3)를 *부호 분기 밖에서 무조건* 가산. HEXACO상 E(정서성 = fearfulness/anxiety/dependence/sentimentality)는 *부정·취약성* 반응 차원이고 긍정 정서는 X(외향성)가 주도하므로, Joy·Hope·Satisfaction에 E를 더하는 것은 "불안한 사람이 더 기뻐한다"는 구성 모순.

**(B) praiseworthiness 성실성 평균 경유 prudence 오염** — `praiseworthiness_weight`가 공통항으로 `avg.c.effect(W_STANDARD)`(성실성 4 facet 평균 ×0.3)를 써서, prudence(신중)가 Reproach(타인 비난) 강도를 끌어올림. 2.4.0 실측 확인(prudence +0.8 → Reproach weight 0.700→0.742). "신중함 → 비난 민감"은 심리 근거 약한 부수효과. (2.4.0 위임분.)

## 2. 핵심 원칙

- **E(정서성)는 fear-lifecycle에만**: Fear · Relief · FearsConfirmed. 공포를 강하게 느끼는 인물은 그 *해소(안도)* 와 *실현* 도 강하게 느낌(안도감 통찰, Bekay 2026-06-06).
- **X(외향성)는 hope/긍정-lifecycle**: Joy · Hope · Satisfaction · Disappointment. 낙관·기대·몰입 강도.
- **성실성 → praiseworthiness는 facet 차등**: diligence(근면) 균일 + perfectionism(완벽주의) 비대칭. org(조직성)·prudence(신중)는 도덕 평가 민감도와 링크 약해 제외.

## 3. 변경 명세 (코드 레벨)

### 3.1 `desirability_self_weight` (Joy / Distress)

```rust
// BEFORE
let mut e = avg.e.effect(W_STANDARD);
e += if desirability >= 0.0 {
    avg.x.effect(W_STANDARD)
} else {
    -avg.a.effect(W_STANDARD) - self.conscientiousness.prudence.effect(W_STANDARD)
};

// AFTER — E를 음수(Distress) 분기로 이동
let e = if desirability >= 0.0 {
    avg.x.effect(W_STANDARD)                                   // Joy: X
} else {
    avg.e.effect(W_STANDARD)
        - avg.a.effect(W_STANDARD)
        - self.conscientiousness.prudence.effect(W_STANDARD)  // Distress: +E −A −Pru
};
```
**효과**: Joy는 E 손실(X만), Distress는 불변(이미 E 포함).

### 3.2 `desirability_prospect_weight` (Hope / Fear)

```rust
// BEFORE
let mut e = avg.e.effect(W_STANDARD);
e += if desirability >= 0.0 {
    avg.x.effect(W_STANDARD) - self.conscientiousness.prudence.effect(W_MILD)
} else {
    self.emotionality.fearfulness.effect(W_STANDARD)
};

// AFTER — E를 음수(Fear) 분기로 이동
let e = if desirability >= 0.0 {
    avg.x.effect(W_STANDARD) - self.conscientiousness.prudence.effect(W_MILD)   // Hope: X −Pru
} else {
    avg.e.effect(W_STANDARD) + self.emotionality.fearfulness.effect(W_STANDARD) // Fear: +E +fear
};
```
**효과**: Hope는 E 손실, Fear는 불변(E + fearfulness 그대로).

### 3.3 `desirability_confirmation_weight` (Satisfaction/Disappointment/Relief/FearsConfirmed) — **시그니처 변경**

```rust
// BEFORE
fn desirability_confirmation_weight(&self, _desirability: f32) -> f32 {
    let avg = self.dimension_averages();
    let e = avg.e.effect(W_STANDARD) - self.conscientiousness.prudence.effect(W_MILD);
    finalize_weight(BASE_SELF, e, CLAMP_STANDARD)
}

// AFTER — fear축/hope축 분기. is_fear_axis 인자 신설.
fn desirability_confirmation_weight(&self, is_fear_axis: bool) -> f32 {
    let avg = self.dimension_averages();
    let driver = if is_fear_axis {
        avg.e.effect(W_STANDARD)   // Relief / FearsConfirmed: E (fear-lifecycle)
    } else {
        avg.x.effect(W_STANDARD)   // Satisfaction / Disappointment: X (hope-lifecycle)
    };
    let e = driver - self.conscientiousness.prudence.effect(W_MILD);
    finalize_weight(BASE_SELF, e, CLAMP_STANDARD)
}
```
**효과**: Fear축(Relief/FearsConfirmed)은 `E −Pru` 현행과 **동일(불변)**. Hope축(Satisfaction/Disappointment)만 `E→X` 교체.

### 3.4 `praiseworthiness_weight` (Pride/Shame/Admiration/Reproach) — facet 분해

```rust
// BEFORE: 공통항 = avg.c.effect(W_STANDARD) (성실성 4 facet 평균)
// AFTER: diligence 균일 + perfectionism 비대칭 (org·prud 제외). sign은 effect 밖 적용(기존 관례).
fn praiseworthiness_weight(&self, is_self: bool, praiseworthiness: f32) -> f32 {
    let c = &self.conscientiousness;

    // 성실성 기여 — diligence 균일(0.10)
    let dil = c.diligence.effect(0.10);

    // perfectionism 비대칭
    let perf = if is_self {
        if praiseworthiness > 0.0 { -c.perfectionism.effect(0.10) }  // Pride  −0.10
        else                      {  c.perfectionism.effect(0.20) }  // Shame  +0.20
    } else {
        if praiseworthiness > 0.0 {  c.perfectionism.effect(0.15) }  // Admiration +0.15
        else                      {  c.perfectionism.effect(0.20) }  // Reproach   +0.20
    };

    // 분기항(기존 유지) — 자기=modesty, 타인=gentleness
    let branch = if is_self {
        if praiseworthiness > 0.0 { -self.honesty_humility.modesty.effect(W_STANDARD) }
        else                      {  self.honesty_humility.modesty.effect(W_STANDARD) }
    } else {
        if praiseworthiness < 0.0 { -self.agreeableness.gentleness.effect(W_STANDARD) }
        else                      {  self.agreeableness.gentleness.effect(W_STANDARD) }
    };

    finalize_weight(BASE_SELF, dil + perf + branch, CLAMP_STANDARD)
}
```
**계수 근거**: 예산보존(분해 총량 ≈ 원 0.3) → 겸손+완벽주의 인물 Shame 포화 회피(검증 0.7/0.7 = 1.42 < 1.5). Pride는 perf −0.10 + modesty −로 자긍심 억제, Reproach는 perf +0.20 vs gentleness −로 상쇄.

> `effect(w)`는 선형 `facet_value × w` (2.4.0 MCP trace로 검증: prudence 0.8 → avg.c 0.2 → effect(0.3)=0.06 → weight 1.06). 음수 계수는 effect 밖에서 부호 적용(기존 modesty/gentleness 관례 일치).

## 4. 호출부 변경 (`appraisal/event.rs`)

`desirability_confirmation_weight` 호출 지점에서 `ProspectResult` → `is_fear_axis: bool` 매핑 전달:

```rust
let is_fear_axis = matches!(result,
    ProspectResult::FearUnrealized | ProspectResult::FearConfirmed);
let weight = profile.desirability_confirmation_weight(is_fear_axis);
```
- `FearUnrealized`(→Relief) / `FearConfirmed`(→FearsConfirmed) → `true`
- `HopeFulfilled`(→Satisfaction) / `HopeUnfulfilled`(→Disappointment) → `false`

trait `AppraisalWeights` (`src/ports/personality.rs`)의 메서드 시그니처도 `_desirability: f32` → `is_fear_axis: bool`로 변경. 모든 impl(HexacoProfile + 테스트 mock) 동반.

## 5. 변경/불변 케이스 (PAD 벤치 영향 범위)

| 감정 | 변경 | weight |
|---|---|---|
| Joy | ✅ 변경 (E 제거) | `base + X` |
| Hope | ✅ 변경 (E 제거) | `base + X − Pru` |
| Satisfaction · Disappointment | ✅ 변경 (E→X) | `base + X − Pru` |
| Pride/Shame/Admiration/Reproach | ✅ 변경 (facet 분해) | `Dil×0.10 + Perf 비대칭 + 분기` |
| **Distress** | ⬜ 불변 | `base + E − A − Pru` |
| **Fear** | ⬜ 불변 | `base + E + fear` |
| **Relief · FearsConfirmed** | ⬜ 불변 | `base + E − Pru` |
| empathy / hostility / appealingness / stimulus | ⬜ 불변 | — |

→ PAD 벤치는 **변경 케이스만** 재측정. 불변 케이스 편차 발생 시 = 버그 신호.

## 6. 검증 게이트

1. `cargo check` + `cargo test --lib` — baseline 554P/0F 유지(+ weight 직접 검증 unit 기대값 갱신분).
2. appraise-validation **S1~S4** ground truth 재측정 — praiseworthiness 변경으로 Admiration/Reproach intensity 이동분 박제 갱신.
3. **PAD 벤치 20케이스** (`pad-anchor-score-matrix.md`) 변경 케이스 재측정 → **잠금 재확정(Bekay 승인)**. 불변 케이스 편차 0 확인.
4. 통제군 fixture(`appraise-test/prudence-intensity-fixtures`)로 prudence→Reproach 평탄화(셋 다 동일) + 정서성 인물 Joy 하락 확인.
5. clamp 하한 점검 — 저-X(외향성) 인물의 Hope/Satisfaction = `base + X − Pru`가 0.5 floor 접촉 여부.

## 7. 위험 (C)

- 시그니처 변경(confirmation): trait + impl + mock + 호출부 4지점 동반. 컴파일러가 누락 강제.
- praiseworthiness 변경 → S1~S4 + PAD 벤치 Pride/Shame/Admir/Reproach 케이스 전부 재측정.
- 무한 튜닝 위험 — 본 spec 계수 FROZEN. 변경은 Bekay 재확인 후.

## 8. Baseline (D)

- 메트릭: Phase 2.3 종결 `cargo test --lib` 554P/0F.
- 잠금: `pad-anchor-score-matrix.md` 20케이스 + S1~S4 박제값(commit 시점).
- 표면: personality.rs weight 4 + ports/personality.rs trait 1 + appraisal/event.rs 호출 1.

## 9. 비스코프

- 정서성 facet 분해(sentimentality만 양수 잔존 등 안 B) — 채택 안 함(A안: E축 통째 이동).
- 강도 1.0 천장 soft-saturation → **Phase 2.6 (deferred)**.
- `RelationshipModifiers` 재설계 → **2.4.3**.
- base_delta 4셀 → **2.4.2**.

## 10. 인계 (Claude Code)

순서: ① self/prospect(3.1·3.2, 본문만) → ② confirmation 시그니처+분기(3.3·4) → ③ praiseworthiness 분해(3.4) → 각 단계 후 `cargo test --lib` + 게이트 2~4. 단계별 박제 갱신으로 회귀 신호 격리. push는 Bekay 지시 시.
