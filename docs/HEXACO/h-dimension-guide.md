# H 차원 (Honesty-Humility) 완전 가이드

## 개요

H = **Honesty-Humility (정직-겸손성)**

HEXACO 성격 모델의 6개 차원 중 첫 번째 차원이다.
사람이 도덕적·사회적 관계에서 얼마나 정직하고 겸손하게 행동하는가를 측정한다.

| H 수준 | NPC 성향 |
|--------|---------|
| **높음 (+)** | 진실하고 공정하며, 욕심 없고, 자기를 낮추는 성향 |
| **낮음 (-)** | 교활하고 불공정하며, 물질 집착적이고, 자기 과시를 즐기는 성향 |

무협 예시:
- H 높음: 의리 있는 협객, 정직한 무사, 청빈한 도사
- H 낮음: 배후에서 조종하는 흑막, 탐욕적 상인, 교활한 악당

---

## 4개 Facet 상세

H 차원은 **4개 Facet(하위 성격 요소)**으로 구성된다.
각 Facet은 독립적인 수치를 가지며 범위는 **-1.0 ~ +1.0**이다.

---

### Facet 1. Sincerity (진실성)

> **"이 NPC는 자기 감정과 의도를 솔직하게 드러내는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 상대에게 솔직하게 말함. 속마음과 겉말이 일치. 아첨하지 않음 |
| 0.0 (중간) | 상황에 따라 솔직하기도, 감추기도 함 |
| -1.0 (낮음) | 위선적. 속마음을 숨기고 겉으로만 좋게 보이려 함. 아첨에 능함 |

**연결 감정 (Action 브랜치)**: `Gratitude (감사)` 증폭
- 공식: `gratitude_amp = 1.0 + sincerity.max(0.0) × W`
- W = 0.3 (PERSONALITY_WEIGHT 상수)
- 진실한 NPC일수록 타인의 선의를 진심으로 받아들여 더 강하게 감사함
- Fairness, Greed Avoidance, Modesty는 Gratitude에 관여하지 않음

---

### Facet 2. Fairness (공정성)

> **"이 NPC는 규칙을 지키고 불공정한 이득을 거부하는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 원칙을 중시. 부정한 이득 거부. 공정한 판단 추구 |
| 0.0 (중간) | 상황에 따라 타협 가능 |
| -1.0 (낮음) | 규칙을 어겨서라도 이득을 추구. 사기, 부정직에 거리낌 없음 |

**연결 감정**: 직접 연결된 단독 감정 없음
- avg.h(H 평균)에 1/4 기여 → Event 브랜치의 HappyFor, Resentment, Gloating에 간접 기여

---

### Facet 3. Greed Avoidance (탐욕 회피)

> **"이 NPC는 부와 사치, 사회적 지위에 집착하지 않는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 물질적 욕망이 적음. 화려한 것에 흥미 없음. 검소하고 소박함 |
| 0.0 (중간) | 적당한 욕심 |
| -1.0 (낮음) | 부, 권력, 사치를 강하게 추구. 허세를 부림. 타인의 성공을 시기함 |

**연결 감정**: 직접 연결된 단독 감정 없음
- avg.h(H 평균)에 1/4 기여 → Greed Avoidance가 낮을수록 avg.h를 끌어내려
  Resentment 발동 임계값(-0.2) 도달 가능성이 높아짐

---

### Facet 4. Modesty (겸손)

> **"이 NPC는 자신의 능력과 업적을 과장하지 않고 낮추는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 자기 자랑을 꺼림. 칭찬받아도 쑥스러워함. 자기 성취를 과소평가 |
| 0.0 (중간) | 적절한 자기 평가 |
| -1.0 (낮음) | 자기 과시를 즐김. 잘난 척이 심함. 자신의 우월함을 드러내고 싶어함 |

**연결 감정 (Action 브랜치)**: `Pride (자부심)` 억제
- 공식: `pride_mod = 1.0 - modesty.max(0.0) × W`
- W = 0.3 (PERSONALITY_WEIGHT 상수)
- 겸손한 NPC는 칭찬받을 행동을 해도 Pride가 약하게 발생
- Sincerity, Fairness, Greed Avoidance는 Pride에 관여하지 않음

---

## avg.h — H 차원 수치 평균

```
avg.h = (sincerity + fairness + greed_avoidance + modesty) / 4
범위: -1.0 ~ +1.0
```

4개 Facet이 **모두 동등하게** H 평균에 기여한다.
이 평균값이 Event 브랜치에서 타인에 대한 감정 판정의 기준이 된다.

예시 (`tests/common/mod.rs` 기준):
```
NPC 무백: sincerity=0.8, fairness=0.7, greed_avoidance=0.6, modesty=0.5
          → avg.h = (0.8+0.7+0.6+0.5)/4 = 0.65  (공감형, HappyFor 발생 가능)

NPC 교룡: sincerity=-0.4, fairness=-0.5, greed_avoidance=-0.6, modesty=-0.7
          → avg.h = (-0.4-0.5-0.6-0.7)/4 = -0.55  (질투형, Resentment 발동)
```

---

## 핵심 상수

### AppraisalEngine 상수 (engine.rs)

| 상수명 | 코드 | 값 | 용도 |
|--------|------|-----|------|
| `W` | `AppraisalEngine::W` | 0.3 | 성격 facet이 감정 강도에 미치는 범용 계수 |
| `EMPATHY_BASE` | `AppraisalEngine::EMPATHY_BASE` | 0.5 | Fortune-of-others 기본 공감 강도 |
| `FORTUNE_THRESHOLD` | `AppraisalEngine::FORTUNE_THRESHOLD` | -0.2 | Resentment/Gloating 발동 기준 (H↓, A↓ 판정) |

### Guide 레이어 상수 (guide/mod.rs)

| 상수명 | 값 | 용도 |
|--------|-----|------|
| `TRAIT_THRESHOLD` | 0.3 | 성격 특성 추출 및 어조 판단 임계값 |
| `HONESTY_RESTRICTION_THRESHOLD` | 0.5 | H↑일 때 거짓말 금지 제약 발동 |

---

## 핵심 변수 해설

### `h` / `avg.h`

코드 내에서 `let h = avg.h` 형태로 쓰이는 H 평균의 단순 축약 변수명이다.
H 차원 전체를 대표하는 단일 수치로, Event 브랜치의 조건문에서 사용된다.
`p.dimension_averages().h` 로 산출한다.

### `a` / `avg.a`

A 차원(Agreeableness, 원만성)의 Facet 평균값이다.
`p.dimension_averages().a` 로 산출하며 H와 함께 타인에 대한 감정 판정에 사용된다.

### `rel_mul` (Relationship Intensity Multiplier)

**관계의 친밀도/적대도에 따른 감정 배율**이다.
모든 감정 공식에 최종 배율로 곱해진다.

```rust
// relationship.rs
pub fn emotion_intensity_multiplier(&self) -> f32 {
    1.0 + self.closeness.intensity() * 0.5
}
```

| closeness | rel_mul | 의미 |
|-----------|---------|------|
| 0.0 (무관) | 1.0 | 기준값 |
| ±0.5 (보통) | 1.25 | 25% 증폭 |
| ±1.0 (절친/적대) | 1.5 | 50% 증폭 |

### `trust_mod` (Trust Emotion Modifier)

**신뢰와 행동의 불일치에 따른 감정 배율**이다.
Action 브랜치에서 Reproach, Gratitude, Anger에 곱해진다.

```rust
// relationship.rs
pub fn trust_emotion_modifier(&self, praiseworthiness: f32) -> f32 {
    let violation = self.expectation_violation(praiseworthiness);
    0.5 + violation * 0.5  // 범위: 0.5 ~ 1.5
}
```

- trust 높은데 배신 → 기대 위반 → 감정 증폭 (최대 1.5)
- trust 낮은데 배신 → 기대 부합 → 감정 약화 (최소 0.5)

### `desir_other` (desirability_other)

**"타인 입장에서 이 사건이 얼마나 바람직한가"**를 나타내는 Situation 입력값이다.

```
+1.0 → 타인에게 매우 좋은 일 (예: 승진, 보상, 사랑받음)
-1.0 → 타인에게 매우 나쁜 일 (예: 패배, 손해, 굴욕)
 0.0 → 타인과 무관한 사건
```

자신에게 어떤지(`desirability_self`)와 별개로 입력받아,
NPC가 타인의 상황에 공감할지, 질투할지, 쾌재를 부를지 계산한다.

### `negative_mod` (부정 감정 조절 계수)

Event 브랜치 전용. A(원만성) 평균이 높을수록 부정 감정을 억제하는 배율이다.
Distress와 Resentment에 적용된다.

```rust
// engine.rs — appraise_event()
let negative_mod = 1.0 - avg.a.max(0.0) * w;
```

| avg.a 값 | negative_mod | 의미 |
|----------|-------------|------|
| +1.0 (매우 원만) | 0.70 | 부정 감정 30% 억제 |
| 0.0 (중간) | 1.00 | 기준값 |
| -1.0 (비우호적) | 1.00 | 억제 없음 (`.max(0.0)`으로 음수 차단) |

**주의**: 이전 문서에서 사용한 `anger_mod` 변수와 다르다.
`anger_mod`는 Action 브랜치에서만 사용되며 `A.patience` 개별 facet 기반이다.

### `standards_amp` (자기 기준 증폭 계수)

Action 브랜치 전용. C(성실성) 차원이 높을수록 자기 행동에 대한 평가가 엄격해진다.
Pride, Shame, Admiration, Reproach 등 모든 Action 감정에 적용된다.

```rust
// engine.rs — appraise_action()
let standards_amp = 1.0 + avg.c.abs() * w;
```

### `modesty` (코드 내 단독 변수)

H 차원 4개 Facet 중 **겸손(Modesty) 하나의 단독 수치**다.
`p.honesty_humility.modesty.value()` 로 참조하며 Action 브랜치에서만 단독 사용된다.
avg.h와 구별하여 주의해야 한다.

### `pride_mod` (Pride Modifier, 자부심 조절 계수)

modesty가 높을수록 Pride 감정을 억제하는 배율이다.

```
pride_mod = 1.0 - modesty.max(0.0) × W
```

| modesty 값 | pride_mod | 결과 |
|-----------|----------|------|
| +1.0 (매우 겸손) | 0.70 | Pride 30% 억제 |
| +0.5 | 0.85 | Pride 15% 억제 |
| 0.0 | 1.00 | 억제 없음 |
| -1.0 (오만) | 1.00 | 억제 없음 (`max(0.0)`으로 음수 차단) |

---

## H 4개 Facet × OCC 3 브랜치 연결 — 감정별 상세

### Event 브랜치 — `avg.h` 전체 평균 사용

자신에게 일어난 사건(Joy/Distress)은 H와 **무관**하다.
**타인에게 일어난 사건**(Fortune-of-others)에만 H가 관여한다.

---

#### HappyFor (타인의 행운에 대리기쁨)

| 항목 | 내용 |
|------|------|
| **발동 조건** | `(h > 0.0 \|\| a > 0.0)` AND `desir_other > 0` |
| **empathy** | `(h.max(0.0) + a.max(0.0)) / 2.0` |
| **강도 공식** | `desir_other × (EMPATHY_BASE + empathy × EMPATHY_BASE) × rel_mul` |
| **해석** | H와 A 중 양수인 부분만 공감 강도에 기여. 음수 값은 `.max(0.0)`으로 차단 |

코드 (`engine.rs`):
```rust
if h > 0.0 || a > 0.0 {
    let empathy = (h.max(0.0) + a.max(0.0)) / 2.0;
    state.add(Emotion::new(EmotionType::HappyFor,
        desir_other * (Self::EMPATHY_BASE + empathy * Self::EMPATHY_BASE) * rel_mul));
}
```

상세 흐름:
```
[1] h > 0.0 || a > 0.0?  →  NO  →  HappyFor 없음
                          →  YES ↓
[2] empathy = (h.max(0.0) + a.max(0.0)) / 2.0
[3] 강도 = desir_other × (0.5 + empathy × 0.5) × rel_mul
```

예시: 무백(avg.h=0.65, avg.a=0.575), 라이벌 승진(desir_other=0.8), 중립 관계(rel_mul=1.0)
```
empathy = (0.65 + 0.575) / 2.0 = 0.6125
강도 = 0.8 × (0.5 + 0.6125 × 0.5) × 1.0 = 0.8 × 0.80625 ≈ 0.645
```

---

#### Resentment (타인의 행운에 질투/시기)

**발동 전제: `h < FORTUNE_THRESHOLD(-0.2)`**
이 조건을 충족하지 않으면 desir_other가 아무리 커도 Resentment는 발생하지 않는다.

| 항목 | 내용 |
|------|------|
| **발동 조건** | `h < -0.2` AND `desir_other > 0` |
| **강도 공식** | `desir_other × h.abs() × negative_mod × rel_mul` |
| **negative_mod** | `1.0 - avg.a.max(0.0) × W` |
| **`h.abs()` 의미** | h는 발동 시 항상 음수 → 절댓값으로 "H가 얼마나 낮은가"를 강도에 반영 |

코드 (`engine.rs`):
```rust
if h < t {
    state.add(Emotion::new(EmotionType::Resentment,
        desir_other * h.abs() * negative_mod * rel_mul));
}
```

| avg.h | h.abs() | 의미 |
|-------|---------|------|
| -0.25 (간신히 발동) | 0.25 | Resentment 약함 |
| -0.70 | 0.70 | Resentment 강함 |
| -1.00 (최저) | 1.00 | Resentment 최대 |

흐름:
```
[1] h < -0.2?  →  NO  →  Resentment 없음
               →  YES ↓
[2] negative_mod = 1.0 - avg.a.max(0.0) × 0.3
[3] 강도 = desir_other × |h| × negative_mod × rel_mul
```

예시: 교룡(avg.h=-0.55, avg.a=-0.55), 라이벌 승진(desir_other=0.8), 중립 관계(rel_mul=1.0)
```
negative_mod = 1.0 - (-0.55).max(0.0) × 0.3 = 1.0 - 0.0 = 1.0
강도 = 0.8 × 0.55 × 1.0 × 1.0 = 0.44
```

**참고**: 교룡의 avg.a가 음수(-0.55)이므로 `.max(0.0)` 에 의해 negative_mod=1.0이 된다.
A가 높은 NPC라면 negative_mod가 1.0 미만이 되어 Resentment가 억제된다.

---

#### Gloating (타인의 불행에 쾌재)

| 항목 | 내용 |
|------|------|
| **발동 조건** | `h < -0.2` **AND** `a < -0.2` AND `desir_other < 0` |
| **cruelty** | `(h.abs() + a.abs()) / 2.0` |
| **강도 공식** | `\|desir_other\| × cruelty × rel_mul` |
| **해석** | H↓(이기심) + A↓(공격성)의 조합이 타인의 불행을 즐기는 심리를 만듦 |

코드 (`engine.rs`):
```rust
if h < t && a < t {
    let cruelty = (h.abs() + a.abs()) / 2.0;
    state.add(Emotion::new(EmotionType::Gloating,
        abs * cruelty * rel_mul));
}
```

예시: 교룡(avg.h=-0.55, avg.a=-0.55), 적의 패배(desir_other=-0.7), 중립 관계(rel_mul=1.0)
```
cruelty = (0.55 + 0.55) / 2.0 = 0.55
강도 = 0.7 × 0.55 × 1.0 = 0.385
```

---

#### Pity (타인의 불행에 동정)

| 항목 | 내용 |
|------|------|
| **발동 조건** | `desir_other < 0` AND (`a > 0.0` OR `sentimentality > 0.0`) |
| **compassion** | `(a.max(0.0) + sentimentality.max(0.0)) / 2.0` |
| **강도 공식** | `\|desir_other\| × (EMPATHY_BASE + compassion × EMPATHY_BASE) × rel_mul` |
| **H 관여** | 없음 — A(원만성)와 E.sentimentality(감상성)가 담당 |

코드 (`engine.rs`):
```rust
if a > 0.0 || p.emotionality.sentimentality.value() > 0.0 {
    let compassion = (a.max(0.0)
        + p.emotionality.sentimentality.value().max(0.0)) / 2.0;
    state.add(Emotion::new(EmotionType::Pity,
        abs * (Self::EMPATHY_BASE + compassion * Self::EMPATHY_BASE) * rel_mul));
}
```

---

### Action 브랜치 — Facet 개별값 직접 사용

Event 브랜치와 달리 avg.h가 아닌 **특정 Facet 하나**만 단독으로 관여한다.

Action 브랜치 공통 변수:
- `standards_amp = 1.0 + avg.c.abs() × W` — 자기 기준의 엄격함
- `trust_mod` — Relationship의 기대 위반도 배율 (0.5~1.5, 타인 행동 평가 시)
- `rel_mul` — 관계 친밀도 배율 (1.0~1.5)

---

#### Pride (자부심) — H.modesty 관여

| 항목 | 내용 |
|------|------|
| **발동 조건** | `is_self_agent = true`, `praiseworthiness > 0` |
| **pride_mod** | `1.0 - modesty.max(0.0) × W` |
| **강도 공식** | `praiseworthiness × standards_amp × pride_mod × rel_mul` |
| **Sincerity, Fairness, Greed Avoidance** | Pride에 관여하지 않음 |

코드 (`engine.rs`):
```rust
let pride_mod = 1.0 - p.honesty_humility.modesty.value().max(0.0) * w;
state.add(Emotion::new(EmotionType::Pride,
    praiseworthiness * standards_amp * pride_mod * rel_mul));
```

---

#### Shame (수치심) — H 관여 없음

| 항목 | 내용 |
|------|------|
| **발동 조건** | `is_self_agent = true`, `praiseworthiness < 0` |
| **강도 공식** | `\|praiseworthiness\| × standards_amp × rel_mul` |
| **H 관여** | 없음 — C.standards_amp(자기 기준)가 담당 |

---

#### Admiration (감탄) — H 관여 없음 (참고)

| 항목 | 내용 |
|------|------|
| **발동 조건** | `is_self_agent = false`, `praiseworthiness > 0` |
| **강도 공식** | `praiseworthiness × standards_amp × rel_mul` |
| **H 관여** | 없음 |

---

#### Reproach (비난) — H 관여 없음 (참고)

| 항목 | 내용 |
|------|------|
| **발동 조건** | `is_self_agent = false`, `praiseworthiness < 0` |
| **reproach_mod** | `1.0 - gentleness.max(0.0) × W` (A.gentleness 기반) |
| **강도 공식** | `\|praiseworthiness\| × standards_amp × reproach_mod × trust_mod × rel_mul` |
| **H 관여** | 없음 — A.gentleness와 trust_mod가 담당 |

---

#### Gratitude (감사) — H.sincerity 관여

| 항목 | 내용 |
|------|------|
| **발동 조건** | `is_self_agent = false`, `praiseworthiness > 0`, `outcome_for_self > 0` |
| **gratitude_amp** | `1.0 + sincerity.max(0.0) × W` |
| **강도 공식** | `(praiseworthiness + outcome) / 2.0 × gratitude_amp × trust_mod × rel_mul` |
| **Fairness, Greed Avoidance, Modesty** | Gratitude에 관여하지 않음 |

코드 (`engine.rs`):
```rust
let gratitude_amp = 1.0 + p.honesty_humility.sincerity.value().max(0.0) * w;
state.add(Emotion::new(EmotionType::Gratitude,
    (praiseworthiness + outcome) / 2.0 * gratitude_amp * trust_mod * rel_mul));
```

---

#### Anger (분노) — H 관여 없음

| 항목 | 내용 |
|------|------|
| **발동 조건** | `is_self_agent = false`, `praiseworthiness < 0`, `outcome_for_self < 0` |
| **anger_mod** | `1.0 - patience.value() × W` (A.patience 기반) |
| **강도 공식** | `(\|praiseworthiness\| + \|outcome\|) / 2.0 × anger_mod × trust_mod × rel_mul` |
| **H 관여** | 없음 — A.patience(인내심)가 담당 |

코드 (`engine.rs`):
```rust
let anger_mod = 1.0 - p.agreeableness.patience.value() * w;
state.add(Emotion::new(EmotionType::Anger,
    (praiseworthiness.abs() + outcome.abs()) / 2.0 * anger_mod * trust_mod * rel_mul));
```

**주의**: Action 브랜치의 `anger_mod`와 Event 브랜치의 `negative_mod`는 별개 변수다.
- `anger_mod` = `1.0 - patience.value() × W` (patience 개별 facet, `.max(0.0)` 없음)
- `negative_mod` = `1.0 - avg.a.max(0.0) × W` (A 평균, `.max(0.0)` 있음)

---

### Object 브랜치 — H 전혀 관여 안 함

| 감정 | 담당 |
|------|------|
| Love | O.aesthetic_appreciation |
| Hate | O.aesthetic_appreciation |

H 4개 Facet 모두 Object 브랜치에 영향 없다.

---

## Guide 레이어에서의 H 연동

AppraisalEngine(감정 생성) 외에, Guide 레이어에서도 avg.h가 직접 사용된다.

### ActingDirective — 어조(Tone) 결정

dominant 감정이 Pride일 때, avg.h에 따라 어조가 분기된다 (`directive.rs`):

```rust
Some(EmotionType::Pride) => {
    if avg.h > t { Tone::QuietConfidence }
    else { Tone::ProudArrogant }
}
```

| avg.h | 결과 | 의미 |
|-------|------|------|
| > 0.3 (TRAIT_THRESHOLD) | `QuietConfidence` | 조용한 자신감이 묻어나는 어조 |
| ≤ 0.3 | `ProudArrogant` | 자랑스럽고 거만한 어조 |

겸손한 성품(H 높음)의 NPC는 자부심을 느껴도 조용히 드러내고,
H가 낮은 NPC는 노골적으로 거만해진다.

### ActingDirective — 금지 사항(Restriction)

```rust
if avg.h > HONESTY_RESTRICTION_THRESHOLD {
    restrictions.push(Restriction::NoLyingOrExaggeration);
}
```

| avg.h | 결과 |
|-------|------|
| > 0.5 | `NoLyingOrExaggeration` — 거짓말이나 과장 금지 |
| ≤ 0.5 | 제약 없음 |

H가 높은 NPC는 **감정 상태와 무관하게** 거짓말이나 과장을 하지 못한다.
이 제약은 성격 기반이므로 분노나 두려움 상태에서도 유지된다.

### PersonalitySnapshot — 성격 특성·말투 (`snapshot.rs`)

avg.h가 TRAIT_THRESHOLD(±0.3)를 넘으면 LLM 가이드에 성격 특성과 말투가 포함된다:

```rust
if avg.h > t {
    traits.push(PersonalityTrait::HonestAndModest);
    styles.push(SpeechStyle::FrankAndUnadorned);
} else if avg.h < -t {
    traits.push(PersonalityTrait::CunningAndAmbitious);
    styles.push(SpeechStyle::HidesInnerThoughts);
}
```

| avg.h | PersonalityTrait | SpeechStyle |
|-------|-----------------|-------------|
| > 0.3 | `HonestAndModest` (진실되고 겸손) | `FrankAndUnadorned` (솔직하고 꾸밈없음) |
| < -0.3 | `CunningAndAmbitious` (교활하고 야심적) | `HidesInnerThoughts` (속내를 감춤) |
| -0.3 ~ 0.3 | H 관련 특성 없음 | H 관련 말투 없음 |

---

## Facet별 담당 감정 한눈에 보기

```
H 차원 (Honesty-Humility)
├── Sincerity (진실성)
│   └── Action: Gratitude 증폭 (+최대 30%)
│       avg.h에 1/4 기여
│
├── Fairness (공정성)
│   └── 직접 감정 없음
│       avg.h에 1/4 기여
│
├── Greed Avoidance (탐욕 회피)
│   └── 직접 감정 없음
│       avg.h에 1/4 기여
│
└── Modesty (겸손)
    └── Action: Pride 억제 (-최대 30%)
        avg.h에 1/4 기여

avg.h (4 Facet 평균)
  ── AppraisalEngine (감정 생성) ──
  h>0 || a>0      →  Event: HappyFor (타인 행운에 공감)
  h < -0.2        →  Event: Resentment (타인 행운에 질투)
  h < -0.2
  + a < -0.2      →  Event: Gloating (타인 불행에 쾌재)

  ── Guide 레이어 (연기 지시) ──
  > 0.3           →  Tone: QuietConfidence (Pride 시 조용한 자신감)
  > 0.5           →  Restriction: NoLyingOrExaggeration (거짓말 금지)
  > 0.3           →  Trait: HonestAndModest + FrankAndUnadorned
  < -0.3          →  Trait: CunningAndAmbitious + HidesInnerThoughts
```

---

## 이론적 근거 분석

각 설계 결정에 대한 학술적 근거 유무를 정리한다.

### ✅ 근거 있음

**Action 브랜치에서 Facet 개별값 사용**

HEXACO 연구에서 각 Facet은 서로 다른 심리 구인을 독립적으로 측정하도록 설계되었다:
- Modesty = "자신의 능력·성취를 과장하지 않는다" → Pride(자기 행동 평가)와 직접 대응
- Sincerity = "감정·의도를 솔직하게 드러낸다" → Gratitude(타인 선의 수용)와 직접 대응

avg.h를 사용하면 Fairness, Greed Avoidance가 Pride나 Gratitude를 희석시켜
심리적으로 맞지 않는 결과가 된다.

**Event 브랜치에서 avg.h 사용**

HEXACO 연구에서 H 차원 전체는 "타인을 착취하거나 공정하게 대하는 도덕적 성향" 전체를
측정하도록 설계되었다 (Ashton & Lee, 2007):

> "낮은 H: 착취할 기회가 있을 때 이기적으로 행동"

타인의 행운에 질투하거나 기뻐하는 반응은 Sincerity 하나만의 문제가 아니라
H의 4개 Facet이 복합적으로 작용하는 도덕적 인격 전체의 문제다.

참고문헌:
- Ashton, M. C., & Lee, K. (2007). Empirical, theoretical, and practical advantages of the HEXACO model. *Personality and Social Psychology Review*, 11(2), 150-166.
- Ortony, A., Clore, G.L., Collins, A. (1988). *The Cognitive Structure of Emotions*. Cambridge University Press.

### ❌ 엔지니어링 설계값 (학술 근거 없음)

| 값 | 설명 | 튜닝 방향 |
|----|------|---------|
| **W = 0.3** | 성격이 감정 강도에 미치는 범용 계수 | 높이면 성격 영향 증가, 낮추면 감소 |
| **-0.2 임계값** | Resentment/Gloating 발동 기준 | 더 낮추면 발동 범위 좁아짐, 높이면 넓어짐 |
| **0.3 TRAIT_THRESHOLD** | Guide 레이어 성격 특성 추출 기준 | 낮추면 더 약한 성격도 가이드에 포함 |
| **0.5 HONESTY_RESTRICTION_THRESHOLD** | 거짓말 금지 발동 기준 | 낮추면 더 많은 NPC가 거짓말 금지 |

이 수치들은 가중치 패턴 통일(`1.0 ± facet × W`) 원칙 하에 선택된 엔지니어링 파라미터다.
실제 게임/시나리오 테스트를 통해 조정해야 한다.

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|---------|
| 0.1.0 | 2025-03-24 | 초기 작성. H 4 Facet 상세, avg.h, 핵심 변수 해설, OCC 브랜치별 감정 공식, 이론적 근거 분석 정리 |
| 0.2.0 | 2025-03-24 | H 차원과 EmotionalMomentum 구성 섹션 추가 |
| 0.3.0 | 2025-03-25 | 현행 코드 기준 전면 현행화. 주요 변경: (1) EmotionalMomentum 섹션 삭제 (코드에 미존재), (2) anger_mod→negative_mod 변수 교체 및 공식 수정 (계수 0.4→W=0.3, .max(0.0) 추가), (3) HappyFor 발동 조건 수정 (h>0→h>0∥a>0, 공식에 .max(0.0) 클램핑 반영), (4) 모든 감정 공식에 rel_mul(관계 배율) 추가, (5) Action 브랜치에 trust_mod·standards_amp 추가, (6) Gratitude·Anger 강도 공식 정밀 반영 (compound 감정 평균 패턴), (7) Guide 레이어 H 연동 섹션 신설 (Tone 분기, Restriction, PersonalitySnapshot), (8) 테스트 캐릭터 facet 수치를 tests/common/mod.rs 기준으로 수정 (무백 sincerity 0.7→0.8 등), (9) Admiration·Reproach 참고 항목 추가, (10) Pity 공식 상세화 |
