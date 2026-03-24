# X 차원 (Extraversion) 완전 가이드

## 개요

X = **Extraversion (외향성)**

HEXACO 성격 모델의 6개 차원 중 세 번째 차원이다.
사람이 사회적 환경에서 얼마나 자신감 있고, 활발하며, 긍정적인 감정을 표현하는지를 측정한다.

| X 수준 | NPC 성향 |
|--------|---------|
| **높음 (+)** | 사교적이고 활발하며, 자기 표현이 강하고, 긍정적 감정을 더 강하게 느끼는 성향 |
| **낮음 (-)** | 내향적이고 조용하며, 혼자 있기를 선호. 긍정 감정 증폭 없음 |

무협 예시:
- X 높음: 의협심 넘치는 주인공형 협객, 호방한 무림 맹주, 활기찬 상단 행수
- X 낮음: 은둔 수련자, 말 없는 자객, 조용한 도사

---

## 4개 Facet 상세

X 차원은 **4개 Facet(하위 성격 요소)**으로 구성된다.
각 Facet은 독립적인 수치를 가지며 범위는 **-1.0 ~ +1.0**이다.

---

### Facet 1. Social Self-Esteem (사회적 자존감)

> **"이 NPC는 사회적 관계에서 자신이 가치 있다고 느끼는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 자신이 타인에게 호감을 준다고 믿음. 거절에 크게 상처받지 않음. 자신감 있게 교류 |
| 0.0 (중간) | 상황에 따라 자신감이 있기도, 위축되기도 함 |
| -1.0 (낮음) | 자신이 타인에게 별로라고 느낌. 사회적 상황에서 위축됨. 비판에 쉽게 상처받음 |

**연결 감정**: 직접 연결된 단독 감정 없음
- avg.x(X 평균)에 1/4 기여 → `positive_amp` 경유로 긍정 감정 전반에 간접 기여

---

### Facet 2. Social Boldness (사회적 대담성)

> **"이 NPC는 사람들 앞에서 당당하게 행동하는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 낯선 사람에게 먼저 말을 걺. 공개적인 자리에서도 주눅들지 않음. 리더 역할을 즐김 |
| 0.0 (중간) | 익숙한 상황에서는 당당하지만 낯선 상황에서는 조심스러움 |
| -1.0 (낮음) | 낯선 상황에 불안을 느낌. 사람들 앞에서 말하기 어려워함. 관심을 피하려 함 |

**연결 감정**: 직접 연결된 단독 감정 없음
- avg.x(X 평균)에 1/4 기여

---

### Facet 3. Sociability (사교성)

> **"이 NPC는 타인과 어울리는 것을 즐기는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 사람들과 교류하는 것을 즐김. 파티나 모임을 선호. 혼자 있으면 심심해함 |
| 0.0 (중간) | 사교와 독거 모두 적당히 즐김 |
| -1.0 (낮음) | 혼자 있는 것을 선호. 사교 모임을 에너지 소모로 느낌. 깊은 1:1 교류를 선호 |

**연결 감정**: 직접 연결된 단독 감정 없음
- avg.x(X 평균)에 1/4 기여

---

### Facet 4. Liveliness (활력)

> **"이 NPC는 활기차고 열정적인가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 에너지가 넘침. 유머가 있음. 주변을 밝게 만듦. 긍정적 에너지를 발산 |
| 0.0 (중간) | 상황에 따라 활기차기도, 차분하기도 함 |
| -1.0 (낮음) | 조용하고 차분함. 에너지 소비를 아낌. 진지하고 무거운 분위기 |

**연결 감정**: 직접 연결된 단독 감정 없음
- avg.x(X 평균)에 1/4 기여

---

## avg.x — X 차원 수치 평균

```
avg.x = (social_self_esteem + social_boldness + sociability + liveliness) / 4
범위: -1.0 ~ +1.0
```

4개 Facet이 **모두 동등하게** X 평균에 기여한다.
이 평균값은 `avg.x.max(0.0)` — **양수일 때만** 긍정 감정을 증폭시킨다.

```
positive_amp = 1.0 + avg.x.max(0.0) × 0.3 + m.positive_bias
```

X가 음수(내향적)이면 positive_amp에 영향 없다. 내향성은 긍정 감정을 억제하지 않으며,
단지 증폭이 없을 뿐이다.

예시:
```
NPC 강호: social_self_esteem=0.7, social_boldness=0.8, sociability=0.6, liveliness=0.9
          → avg.x = 0.75  → positive_amp = 1.225 (긍정 감정 22.5% 증폭)

NPC 은거: social_self_esteem=-0.5, social_boldness=-0.7, sociability=-0.6, liveliness=-0.4
          → avg.x = -0.55  → .max(0.0) = 0.0  → positive_amp = 1.00 (증폭 없음)
```

---

## 핵심 변수 해설

### `avg.x` / `positive_amp`

X 차원 전체 평균을 기반으로 계산되는 긍정 감정 배율이다.

```
positive_amp = 1.0 + avg.x.max(0.0) × 0.3 + m.positive_bias
```

| avg.x 값 | positive_amp (맥락 없을 때) | 의미 |
|----------|---------------------------|------|
| +1.0 (매우 외향적) | 1.30 | 긍정 감정 30% 증폭 |
| +0.5 | 1.15 | 긍정 감정 15% 증폭 |
| 0.0 (중간) | 1.00 | 기준값 |
| -1.0 (내향적) | 1.00 | 증폭 없음 (`.max(0.0)` 처리) |

### Facet 단독 사용 없음

X 차원의 4개 Facet은 모두 avg.x에만 기여하며, 어느 Facet도 감정 함수에서 단독으로 참조되지 않는다.
H.modesty, H.sincerity, A.gentleness, A.patience처럼 단독으로 쓰이는 Facet이 없다.

---

## X 4개 Facet × OCC 3 브랜치 연결 — 감정별 상세

### Event 브랜치 — `avg.x` 긍정 감정 증폭

자신에게 일어난 사건 중 **긍정 사건**(Joy, Hope, Satisfaction, Relief)에만 X가 관여한다.
부정 사건(Distress, Fear, Disappointment, FearsConfirmed)에는 X 관여 없다.
타인에게 일어난 사건(HappyFor, Resentment, Pity, Gloating)에도 X 관여 없다.

---

#### Joy (기쁨)

| 항목 | 내용 |
|------|------|
| **발동 조건** | desirability_self > 0 |
| **강도 공식** | `desirability_self × positive_amp` |
| **X 역할** | positive_amp에 avg.x.max(0.0) × 0.3 기여 |

---

#### Hope (희망)

| 항목 | 내용 |
|------|------|
| **발동 조건** | desirability_self > 0, is_prospective = true |
| **강도 공식** | `base × positive_amp` |
| **X 역할** | positive_amp에 기여 |

---

#### Satisfaction (만족)

| 항목 | 내용 |
|------|------|
| **발동 조건** | 이전에 Hope → 사건이 예상대로 긍정 결과 |
| **강도 공식** | `base × positive_amp` |
| **X 역할** | positive_amp에 기여 |

---

#### Relief (안도)

| 항목 | 내용 |
|------|------|
| **발동 조건** | 이전에 Fear → 사건이 예상보다 좋게 해결 |
| **강도 공식** | `base × positive_amp` |
| **X 역할** | positive_amp에 기여 |

---

#### 부정 Event 감정 — X 관여 없음

| 감정 | 담당 |
|------|------|
| Distress | E.emotional_amp, C.impulse_mod |
| Fear | E.emotional_amp, E.fearfulness |
| Disappointment | E.emotional_amp |
| FearsConfirmed | E.emotional_amp |

---

### Action 브랜치 — X 관여 없음

| 감정 | 담당 |
|------|------|
| Pride, Shame | C.standards_amp / H.modesty |
| Admiration, Reproach | C.standards_amp / A.gentleness |
| Gratitude, Anger | H.sincerity / A.patience |

X 4개 Facet 모두 Action 브랜치에 영향 없다.

---

### Object 브랜치 — X 관여 없음

| 감정 | 담당 |
|------|------|
| Love | O.aesthetic_appreciation |
| Hate | O.aesthetic_appreciation |

X 4개 Facet 모두 Object 브랜치에 영향 없다.

---

## Facet별 담당 감정 한눈에 보기

```
X 차원 (Extraversion)
├── Social Self-Esteem (사회적 자존감)
│   └── 직접 감정 없음
│       avg.x에 1/4 기여
│
├── Social Boldness (사회적 대담성)
│   └── 직접 감정 없음
│       avg.x에 1/4 기여
│
├── Sociability (사교성)
│   └── 직접 감정 없음
│       avg.x에 1/4 기여
│
└── Liveliness (활력)
    └── 직접 감정 없음
        avg.x에 1/4 기여

avg.x (4 Facet 평균, 양수만 사용)
  > 0.0 → Event: Joy/Hope/Satisfaction/Relief 긍정 감정 증폭 (positive_amp)
  ≤ 0.0 → 효과 없음 (.max(0.0) 처리)
```

---

## 이론적 근거 분석

각 설계 결정에 대한 학술적 근거 유무를 정리한다.

### ✅ 근거 있음

**avg.x가 긍정 감정만 증폭시킴**

HEXACO 및 Big Five 연구에서 외향성은 일관되게 긍정 정동(positive affect)과 강하게 연관된다
(Ashton & Lee, 2001; DeNeve & Cooper, 1998).
외향적인 사람은 같은 사건에서 더 강한 기쁨·희망을 경험하는 경향이 있다.

**내향성(음수 avg.x)이 부정 감정을 증폭시키지 않음**

외향성과 부정 정동(negative affect)의 관계는 독립적이다. 내향성은 긍정 정동의 결여를 의미하지,
부정 정동의 증가를 의미하지 않는다. 부정 정동은 신경증적 성향(HEXACO에서 E 차원)이 담당한다.
`.max(0.0)` 처리로 이 관계를 정확히 반영했다.

**Facet 단독 사용 없이 avg.x만 사용**

X 차원의 4개 Facet(사회적 자존감, 대담성, 사교성, 활력)은 모두 "긍정 정동의 풍부함"이라는
단일 구인을 다른 측면에서 측정한다. OCC 모델에서 이에 1:1 대응되는 개별 감정이 없어
각 Facet을 분리하는 것이 심리적 근거를 갖기 어렵다.

참고문헌:
- Ashton, M. C., & Lee, K. (2001). A theoretical basis for the major dimensions of personality. *European Journal of Personality*, 15(5), 327-353.

### ❌ 엔지니어링 설계값 (학술 근거 없음)

| 값 | 설명 | 튜닝 방향 |
|----|------|---------|
| **0.3 계수** (positive_amp) | avg.x가 긍정 감정에 미치는 비율 | 높이면 외향성의 영향력 증가 |

---

## X 차원과 EmotionalMomentum

### EmotionalMomentum 현재 구성

`EmotionalMomentum`은 **이전 감정 상태가 새 평가에 미치는 심리적 관성**을 수치화한 구조체다.
`appraise_with_context()`를 통해 대화 맥락을 유지할 때만 활성화된다.

**계산 공식**:
```
positive_bias = valence.max(0.0) × 0.3
```

### X 관련 감정에서 Momentum이 개입하는 경로

X가 관여하는 감정들 중 `EmotionalMomentum`이 개입하는 경우는 **positive_amp 하나**이며,
`positive_bias`를 통한 간접 개입이다.

```
positive_amp = 1.0 + avg.x.max(0.0) × 0.3 + m.positive_bias
```

- 이전 대화에서 긍정 감정이 쌓였을 때 → `positive_bias` 상승
- 다음 평가에서 `positive_amp` 증가 → Joy/Hope/Satisfaction/Relief가 더 강해짐

| X 관련 감정 | Momentum 개입 | 개입 필드 |
|------------|-------------|---------|
| Joy | 간접 (positive_amp 경유) | `positive_bias` |
| Hope | 간접 (positive_amp 경유) | `positive_bias` |
| Satisfaction | 간접 (positive_amp 경유) | `positive_bias` |
| Relief | 간접 (positive_amp 경유) | `positive_bias` |

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|---------|
| 0.1.0 | 2026-03-24 | 초기 작성. X 4 Facet 상세, avg.x, 핵심 변수 해설, OCC 브랜치별 감정 공식, 이론적 근거 분석, EmotionalMomentum 관계 정리 |
