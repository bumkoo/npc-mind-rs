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

## H 기반 EmotionalMomentum 구성

### 배경: EmotionalMomentum이란?

`EmotionalMomentum`은 **이전 감정 상태가 새 평가에 미치는 심리적 관성**을 수치화한 구조체다.
`appraise_with_context()`를 통해 대화 맥락을 유지할 때만 활성화된다.

현재 4개 필드로 구성된다:

```
negative_bias     — 부정 valence → 새 부정 감정 증폭 (0.0 ~ 0.5)
positive_bias     — 긍정 valence → 새 긍정 감정 증폭 (0.0 ~ 0.3)
anger_erosion     — 기존 Anger → patience 브레이크 약화 (0.0 ~ 0.5)
sensitivity_boost — 기존 Fear/Distress → 감정 민감도 상승 (0.0 ~ 0.3)
```

"이미 화가 난 상태에서 자극을 받으면 더 쉽게 폭발한다"는 심리적 관성을 모델링한다.

---

### H 기반 모멘텀의 필요성

현재 모멘텀은 감정 상태(EmotionState)만 참고하며 **H 차원·Facet이 빠져 있다**.
그 결과 H가 직접 관여하는 감정들(HappyFor, Resentment, Gloating, Gratitude, Pride)은
`appraise`와 `appraise_with_context`의 결과가 동일하다.

H/Facet 기반 모멘텀이 필요한 이유:

| 상황 | 기대되는 심리 효과 |
|------|----------------|
| H 높음 + 이미 HappyFor 상태에서 새 공감 자극 | 공감이 공명하여 더 강하게 기뻐함 |
| H 낮음 + 이미 Resentment 상태에서 새 질투 자극 | 시기심이 침식처럼 누적됨 (anger_erosion 원리) |
| H·A 낮음 + 이미 Gloating 상태에서 새 고소 자극 | 쾌재가 겹쳐 더 강해짐 |
| sincerity 높음 + 이미 Gratitude 상태에서 새 호의 | 감사가 공명하여 더 깊어짐 |
| modesty 높음 + 이미 Pride 상태에서 새 칭찬 | 겸손 억제가 추가로 누적됨 |

---

### 신규 5개 필드 설계

| 필드 | 기반 | 범위 | 심리 효과 |
|------|------|------|---------|
| `h_empathy_boost` | avg.h + 기존 HappyFor | 0.0 ~ 0.3 | 공감 공명 — H 높은 NPC가 이미 대리기쁨 상태면 다음 기쁨도 더 강하게 |
| `h_resentment_erosion` | avg.h + 기존 Resentment | 0.0 ~ 0.5 | 시기 침식 — H 낮은 NPC가 이미 질투 중이면 anger_erosion처럼 다음 질투 증폭 |
| `h_gloating_bias` | avg.h·avg.a + 기존 Gloating | 0.0 ~ 0.3 | 쾌재 누적 — H·A 낮음의 복합 악의가 Gloating을 겹쳐 쌓음 |
| `sincerity_gratitude_boost` | sincerity + 기존 Gratitude | 0.0 ~ 0.3 | 감사 공명 — 진실한 NPC가 이미 감사 중이면 새 호의에 더 깊이 감사 |
| `modesty_pride_suppression` | modesty + 기존 Pride | 0.0 ~ 0.3 | 겸손 억제 누적 — 겸손한 NPC가 이미 자부심을 느끼면 추가 칭찬에 더 억제 |

---

### 계산 공식

`from_state(state, p)` — `&EmotionState`와 `&HexacoProfile`을 함께 받아 구성:

```
h_empathy_boost         = happyfor_intensity  × avg.h.max(0.0)  × 0.3
h_resentment_erosion    = resentment_intensity × |avg.h|         × 0.5
h_gloating_bias         = gloating_intensity  × (|avg.h| + |avg.a|) / 2 × 0.3
sincerity_gratitude_boost = gratitude_intensity × sincerity.max(0.0) × 0.3
modesty_pride_suppression = pride_intensity     × modesty.max(0.0)   × 0.3
```

**avg.h/avg.a 처리 원칙:**

| 필드 | avg.h 처리 | 이유 |
|------|-----------|------|
| `h_empathy_boost` | `.max(0.0)` | H 높을 때만 공감 공명 — 음수이면 효과 없음 |
| `h_resentment_erosion` | `.abs()` | H가 얼마나 낮은지가 강도 — 음수값의 크기를 씀 |
| `h_gloating_bias` | `.abs()` | 동일 — 낮을수록 강해짐 |

**Facet 처리 원칙:**

| 필드 | Facet 처리 | 이유 |
|------|-----------|------|
| `sincerity_gratitude_boost` | `.max(0.0)` | 진실성 높을 때만 감사 공명 — 위선적이면 효과 없음 |
| `modesty_pride_suppression` | `.max(0.0)` | 겸손 높을 때만 억제 추가 — 오만하면 억제 없음 |

---

### 감정 계산 적용 위치

각 H 기반 모멘텀 필드가 기존 감정 공식에 어떻게 더해지는지:

#### HappyFor
```
// 기존
desir_other × (0.5 + empathy × 0.5)

// H 모멘텀 적용
desir_other × (0.5 + empathy × 0.5) + m.h_empathy_boost
```
- `m.h_empathy_boost`는 감정 강도에 직접 가산 (배율이 아닌 절대값 추가)

#### Resentment
```
// 기존
desir_other × |avg.h| × anger_mod

// H 모멘텀 적용
desir_other × |avg.h| × (anger_mod + m.h_resentment_erosion)
```
- `anger_erosion`과 동일한 위치에 가산 — patience 브레이크 약화 효과와 동일 구조

#### Gloating
```
// 기존
|desir_other| × cruelty

// H 모멘텀 적용
|desir_other| × cruelty + m.h_gloating_bias
```
- 직접 가산 (HappyFor와 동일 패턴)

#### Gratitude
```
// 기존
base × gratitude_amp
  gratitude_amp = 1.0 + sincerity.max(0.0) × 0.3

// H 모멘텀 적용
base × (gratitude_amp + m.sincerity_gratitude_boost)
```
- `gratitude_amp`에 모멘텀 항을 더해 배율을 상승시킴

#### Pride
```
// 기존
base × standards_amp × pride_mod
  pride_mod = 1.0 - modesty.max(0.0) × 0.3

// H 모멘텀 적용
base × standards_amp × (pride_mod - m.modesty_pride_suppression).max(0.0)
```
- `pride_mod`에서 추가 억제 항을 빼고 음수 방어용 `.max(0.0)` 적용

---

### from_state 시그니처 변경

```rust
// 변경 전 — 감정 상태만 참조
fn from_state(state: &EmotionState) -> Self

// 변경 후 — 성격 프로파일도 함께 받음
fn from_state(state: &EmotionState, p: &HexacoProfile) -> Self
```

`evaluate()`에서의 호출:
```rust
// 변경 전
let momentum = EmotionalMomentum::from_state(current_state);

// 변경 후
let momentum = EmotionalMomentum::from_state(current_state, personality);
```

---

### 심리 원리 요약

**공명 효과 (Resonance)**: 같은 방향의 감정이 이미 존재할 때 새 자극을 받으면 더 크게 진동한다.
H 높은 NPC의 HappyFor, sincerity 높은 NPC의 Gratitude가 이에 해당한다.

**침식 효과 (Erosion)**: 부정 감정이 지속되면서 억제 기제를 조금씩 갉아먹는다.
`anger_erosion`(분노가 인내를 약화)과 동일 원리로, H 낮은 NPC의 Resentment·Gloating이 이에 해당한다.

**억제 누적 (Suppression Compounding)**: 겸손(modesty)이 이미 Pride를 억제하고 있는 상황에서
누적된 Pride가 있으면 다음 칭찬에서 억제가 추가로 작동한다.

---

### 기존 모멘텀과 신규 H 기반 모멘텀 비교

```
EmotionalMomentum
│
├── 기존 (감정 상태 기반)
│   ├── negative_bias      — 전체 부정 valence → 부정 감정 전반 증폭
│   ├── positive_bias      — 전체 긍정 valence → 긍정 감정 전반 증폭
│   ├── anger_erosion      — 기존 Anger → patience 약화
│   └── sensitivity_boost  — 기존 Fear/Distress → 민감도 상승
│
└── 신규 (H 차원·Facet 기반)
    ├── h_empathy_boost         — avg.h↑ + HappyFor  → 공감 공명
    ├── h_resentment_erosion    — avg.h↓ + Resentment → 시기 침식
    ├── h_gloating_bias         — avg.h↓·avg.a↓ + Gloating → 쾌재 누적
    ├── sincerity_gratitude_boost — sincerity↑ + Gratitude → 감사 공명
    └── modesty_pride_suppression — modesty↑ + Pride → 겸손 억제 누적
```

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|---------|
| 0.1.0 | 2026-03-24 | 초기 작성. H 4 Facet 상세, avg.h, 핵심 변수 해설, OCC 브랜치별 감정 공식, 이론적 근거 분석 정리 |
| 0.2.0 | 2026-03-24 | H 기반 EmotionalMomentum 구성 섹션 추가. 신규 5개 필드 설계·공식·적용 위치·심리 원리 문서화 |
