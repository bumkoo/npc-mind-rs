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
- 공식: `gratitude_amp = 1.0 + sincerity.max(0.0) × 0.3`
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
- 공식: `pride_mod = 1.0 - modesty.max(0.0) × 0.3`
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

예시:
```
NPC 무백: sincerity=0.7, fairness=0.6, greed=0.6, modesty=0.7
          → avg.h = 0.65  (공감형, HappyFor 발생 가능)

NPC 교룡: sincerity=-0.6, fairness=-0.4, greed=-0.8, modesty=-0.5
          → avg.h = -0.575  (질투형, Resentment 발동)
```

---

## 핵심 변수 해설

### `h` / `avg.h`

코드 내에서 `let h = avg.h` 형태로 쓰이는 H 평균의 단순 축약 변수명이다.
H 차원 전체를 대표하는 단일 수치로, Event 브랜치의 조건문에서 사용된다.

### `a` / `avg.a`

A 차원(Agreeableness, 원만성)의 Facet 평균값이다.
`p.agreeableness.avg4()` 로 산출하며 H와 함께 타인에 대한 감정 판정에 사용된다.

### `desir_other` (desirability_other)

**"타인 입장에서 이 사건이 얼마나 바람직한가"**를 나타내는 Situation 입력값이다.

```
+1.0 → 타인에게 매우 좋은 일 (예: 승진, 보상, 사랑받음)
-1.0 → 타인에게 매우 나쁜 일 (예: 패배, 손해, 굴욕)
 0.0 → 타인과 무관한 사건
```

자신에게 어떤지(`desirability_self`)와 별개로 입력받아,
NPC가 타인의 상황에 공감할지, 질투할지, 쾌재를 부를지 계산한다.

### `anger_mod` (Anger Modulation, 분노 조절 계수)

A(원만성)가 낮을수록 분노·적대 감정을 증폭시키는 배율이다.

```
anger_mod = (1.0 - avg.a × 0.4)
            + m.anger_erosion    // 이전 대화 맥락 없으면 0.0
            + m.negative_bias    // 이전 대화 맥락 없으면 0.0
```

| avg.a 값 | anger_mod (맥락 없을 때) | 의미 |
|----------|------------------------|------|
| +1.0 (매우 원만) | 0.6 | Resentment 40% 억제 |
| 0.0 (중간) | 1.0 | 기준값 |
| -1.0 (매우 비우호적) | 1.4 | Resentment 40% 증폭 |

### `modesty` (코드 내 단독 변수)

H 차원 4개 Facet 중 **겸손(Modesty) 하나의 단독 수치**다.
`p.honesty_humility.modesty.value()` 로 참조하며 Action 브랜치에서만 단독 사용된다.
avg.h와 구별하여 주의해야 한다.

### `pride_mod` (Pride Modifier, 자부심 조절 계수)

modesty가 높을수록 Pride 감정을 억제하는 배율이다.

```
pride_mod = 1.0 - modesty.max(0.0) × 0.3
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
| **발동 조건** | avg.h > 0.0, desir_other > 0 |
| **강도 공식** | `desir_other × (0.5 + (avg.h + avg.a) / 2 × 0.5)` |
| **해석** | H와 A 모두 높을수록 타인의 행운에 더 진심으로 기뻐함 |

예시: 무백(avg.h=+0.65, avg.a=+0.58), 라이벌 승진(desir_other=0.8)
```
강도 = 0.8 × (0.5 + (0.65 + 0.58) / 2 × 0.5) = 0.8 × 0.8075 ≈ 0.65
```

---

#### Resentment (타인의 행운에 질투/시기)

**발동 전제: `avg.h < -0.2`**
이 조건을 충족하지 않으면 desir_other가 아무리 커도 Resentment는 발생하지 않는다.

| 항목 | 내용 |
|------|------|
| **발동 조건** | avg.h < -0.2 AND desir_other > 0 |
| **강도 공식** | `desir_other × \|avg.h\| × anger_mod` |
| **`\|avg.h\|` 의미** | avg.h는 발동 시 항상 음수 → 절댓값으로 "H가 얼마나 낮은가"를 강도에 반영 |

| avg.h | \|avg.h\| | 의미 |
|-------|----------|------|
| -0.25 (간신히 발동) | 0.25 | Resentment 약함 |
| -0.70 | 0.70 | Resentment 강함 |
| -1.00 (최저) | 1.00 | Resentment 최대 |

흐름:
```
[1] avg.h < -0.2?  →  NO  →  Resentment 없음
                   →  YES ↓
[2] desir_other × |avg.h| × anger_mod  →  Resentment 강도 결정
```

예시: 교룡(avg.h=-0.575, avg.a=-0.55), 라이벌 승진(desir_other=0.8)
```
anger_mod = 1.0 - (-0.55) × 0.4 = 1.22
강도 = 0.8 × 0.575 × 1.22 ≈ 0.56
```

---

#### Gloating (타인의 불행에 쾌재)

| 항목 | 내용 |
|------|------|
| **발동 조건** | avg.h < -0.2 **AND** avg.a < -0.2 (H만 낮아선 부족) |
| **강도 공식** | `\|desir_other\| × (\|avg.h\| + \|avg.a\|) / 2` |
| **해석** | H↓(이기심) + A↓(공격성)의 조합이 타인의 불행을 즐기는 심리를 만듦 |

---

#### Pity (타인의 불행에 동정)

| 항목 | 내용 |
|------|------|
| **발동 조건** | desir_other < 0 |
| **H 관여** | 없음 — E.sentimentality(감상성)가 담당 |

---

### Action 브랜치 — Facet 개별값 직접 사용

Event 브랜치와 달리 avg.h가 아닌 **특정 Facet 하나**만 단독으로 관여한다.

---

#### Pride (자부심) — H.modesty 관여

| 항목 | 내용 |
|------|------|
| **발동 조건** | is_self_agent = true, praiseworthiness > 0 |
| **조절 공식** | `pride_mod = 1.0 - modesty.max(0.0) × 0.3` |
| **Sincerity, Fairness, Greed Avoidance** | Pride에 관여하지 않음 |

---

#### Shame (수치심) — H 관여 없음

| 항목 | 내용 |
|------|------|
| **발동 조건** | is_self_agent = true, praiseworthiness < 0 |
| **H 관여** | 없음 — C.standards_amp(자기 기준)가 담당 |

---

#### Gratitude (감사) — H.sincerity 관여

| 항목 | 내용 |
|------|------|
| **발동 조건** | is_self_agent = false, praiseworthiness > 0, outcome_for_self > 0 |
| **증폭 공식** | `gratitude_amp = 1.0 + sincerity.max(0.0) × 0.3` |
| **Fairness, Greed Avoidance, Modesty** | Gratitude에 관여하지 않음 |

---

#### Anger (분노) — H 관여 없음

| 항목 | 내용 |
|------|------|
| **발동 조건** | is_self_agent = false, praiseworthiness < 0, outcome_for_self < 0 |
| **H 관여** | 없음 — A.patience(인내심)가 담당 |

---

### Object 브랜치 — H 전혀 관여 안 함

| 감정 | 담당 |
|------|------|
| Love | O.aesthetic_appreciation |
| Hate | O.aesthetic_appreciation |

H 4개 Facet 모두 Object 브랜치에 영향 없다.

---

## Facet별 담당 감정 한눈에 보기

```
H 차원 (Honesty-Humility)
├── Sincerity (진실성)
│   └── Action 브랜치: Gratitude 증폭 (+최대 30%)
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
    └── Action 브랜치: Pride 억제 (-최대 30%)
        avg.h에 1/4 기여

avg.h (4 Facet 평균)
  > 0.0         →  Event: HappyFor (타인 행운에 공감)
  < -0.2        →  Event: Resentment (타인 행운에 질투)
  < -0.2
  + avg.a < -0.2 →  Event: Gloating (타인 불행에 쾌재)
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
| **-0.2 임계값** | Resentment 발동 기준 | 더 낮추면 발동 범위 좁아짐, 높이면 넓어짐 |
| **0.4 계수** (anger_mod) | A가 anger_mod에 미치는 비율 | 높이면 A의 영향력 증가 |
| **0.3 계수** (pride_mod, gratitude_amp) | Modesty/Sincerity가 감정에 미치는 비율 | 플레이테스트로 검증 필요 |

이 수치들은 "성격(HEXACO)이 감정 관성(EmotionalMomentum)보다 지배적이어야 한다"는
설계 원칙 하에 임의로 선택된 파라미터다. 실제 게임/시나리오 테스트를 통해 조정해야 한다.

---

## H 차원과 EmotionalMomentum

### EmotionalMomentum 현재 구성

`EmotionalMomentum`은 **이전 감정 상태가 새 평가에 미치는 심리적 관성**을 수치화한 구조체다.
`appraise_with_context()`를 통해 대화 맥락을 유지할 때만 활성화된다.

**구조체 정의** (`engine.rs`):

```rust
struct EmotionalMomentum {
    /// 기존 부정 감정이 새 부정 감정을 증폭 (0.0 ~ 0.5)
    negative_bias: f32,
    /// 기존 긍정 감정이 새 긍정 감정을 증폭 (0.0 ~ 0.3)
    positive_bias: f32,
    /// 기존 Anger가 patience 브레이크를 약화 (0.0 ~ 0.5)
    anger_erosion: f32,
    /// 기존 Fear/Distress가 감정 민감도를 높임 (0.0 ~ 0.3)
    sensitivity_boost: f32,
}
```

**`from_state` 시그니처**:

```rust
fn from_state(state: &EmotionState) -> Self
```

`&EmotionState` 하나만 받는다. `HexacoProfile`은 받지 않는다.

**계산 공식**:

```
negative_bias     = valence.min(0.0).abs() × 0.5
positive_bias     = valence.max(0.0) × 0.3
anger_erosion     = anger_intensity × 0.5
sensitivity_boost = (fear_intensity + distress_intensity) / 2.0 × 0.3
```

---

### H 관련 감정에서 Momentum이 개입하는 경로

H가 관여하는 감정들 중 `EmotionalMomentum`이 실제로 개입하는 경우는 **Resentment 하나**뿐이며,
그것도 `anger_mod`를 통한 간접 개입이다.

```
anger_mod = (1.0 - avg.a × 0.4) + m.anger_erosion + m.negative_bias
```

- `avg.h`는 `anger_mod`에 들어가지 않는다
- `m.anger_erosion`, `m.negative_bias`만 anger_mod에 가산

| H 관련 감정 | Momentum 개입 | 개입 필드 |
|------------|-------------|---------|
| HappyFor | 없음 | — |
| Resentment | 간접 (anger_mod 경유) | `anger_erosion`, `negative_bias` |
| Gloating | 없음 | — |
| Gratitude | 없음 | — |
| Pride | 없음 | — |

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|---------|
| 0.1.0 | 2026-03-24 | 초기 작성. H 4 Facet 상세, avg.h, 핵심 변수 해설, OCC 브랜치별 감정 공식, 이론적 근거 분석 정리 |
| 0.2.0 | 2026-03-24 | H 차원과 EmotionalMomentum 구성 섹션 추가. 현재 구현 상태 기준으로 구조체 필드·계산 공식·H 차원과의 관계·감정별 개입 경로 정리 |
