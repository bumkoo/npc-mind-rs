# C 차원 (Conscientiousness) 완전 가이드

## 개요

C = **Conscientiousness (성실성)**

HEXACO 성격 모델의 6개 차원 중 다섯 번째 차원이다.
사람이 얼마나 조직적이고 근면하며, 자기 행동에 높은 기준을 적용하고 충동을 억제하는지를 측정한다.

| C 수준 | NPC 성향 |
|--------|---------|
| **높음 (+)** | 조직적이고 근면하며, 완벽주의적이고 신중함. 자기 행동에 높은 기준을 적용 |
| **낮음 (-)** | 무질서하고 게으르며, 충동적이고 무모함. 자기 기준이 느슨함 |

무협 예시:
- C 높음: 엄격한 수련을 반복하는 무인, 철저한 계획을 세우는 군사(軍師), 규율 잡힌 문파의 장
- C 낮음: 규칙을 무시하는 방랑자, 즉흥적인 협객, 훈련보다 본능에 의존하는 싸움꾼

---

## 4개 Facet 상세

C 차원은 **4개 Facet(하위 성격 요소)**으로 구성된다.
각 Facet은 독립적인 수치를 가지며 범위는 **-1.0 ~ +1.0**이다.

---

### Facet 1. Organization (조직력)

> **"이 NPC는 정리정돈을 즐기고 체계적으로 행동하는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 계획을 세우고 따름. 정리된 환경을 선호. 순서와 체계를 중시 |
| 0.0 (중간) | 필요할 때만 체계적으로 행동 |
| -1.0 (낮음) | 어수선하고 즉흥적. 계획 없이 행동. 정리정돈에 무관심 |

**연결 감정**: 직접 연결된 단독 감정 없음
- avg.c(C 평균)에 1/4 기여 → `standards_amp` 경유로 Action 브랜치 감정 전반에 간접 기여

---

### Facet 2. Diligence (근면)

> **"이 NPC는 맡은 일에 성실하고 끈기 있게 임하는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 목표를 향해 꾸준히 노력함. 어렵고 지루한 일도 포기하지 않음. 책임감이 강함 |
| 0.0 (중간) | 흥미 있는 일에는 열심히, 아닌 일에는 적당히 |
| -1.0 (낮음) | 금방 포기함. 어려운 일을 회피함. 최소한의 노력만 함 |

**연결 감정**: 직접 연결된 단독 감정 없음
- avg.c(C 평균)에 1/4 기여

---

### Facet 3. Perfectionism (완벽주의)

> **"이 NPC는 높은 기준을 세우고 완벽을 추구하는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 세부 사항에 집착함. 실수를 매우 싫어함. 일이 완벽하게 될 때까지 만족하지 못함 |
| 0.0 (중간) | 적당한 완성도에 만족 |
| -1.0 (낮음) | "대충"이 익숙함. 세부 사항에 무관심. 완성도보다 완료를 우선시 |

**연결 감정**: 직접 연결된 단독 감정 없음
- avg.c(C 평균)에 1/4 기여

---

### Facet 4. Prudence (신중함)

> **"이 NPC는 충동적인 행동을 자제하고 결과를 고려하는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 행동 전에 신중히 생각함. 충동을 억제함. 리스크를 회피함 |
| 0.0 (중간) | 상황에 따라 즉흥적이기도, 신중하기도 함 |
| -1.0 (낮음) | 생각 없이 행동함. 충동적. 결과를 고려하지 않음 |

**연결 감정 (Event 브랜치)**: `Distress (고통)` 충동 억제
- 공식: `impulse_mod = 1.0 - prudence.max(0.0) × 0.3`
- 신중한 NPC는 불쾌한 사건에도 즉각적인 감정 폭발이 억제됨
- Organization, Diligence, Perfectionism은 impulse_mod에 관여하지 않음

---

## avg.c — C 차원 수치 평균

```
avg.c = (organization + diligence + perfectionism + prudence) / 4
범위: -1.0 ~ +1.0
```

4개 Facet이 **모두 동등하게** C 평균에 기여한다.
이 평균값은 `|avg.c|` (절댓값)로 사용된다 — **C가 높든 낮든 자기 행동에 대한 감정이 강해진다.**

```
standards_amp = 1.0 + |avg.c| × 0.3
```

예시:
```
NPC 규율: organization=0.8, diligence=0.9, perfectionism=0.7, prudence=0.8
          → avg.c = 0.80  → standards_amp = 1.24 (자기 행동 감정 24% 증폭)

NPC 무뢰: organization=-0.7, diligence=-0.8, perfectionism=-0.6, prudence=-0.9
          → avg.c = -0.75  → |avg.c| = 0.75  → standards_amp = 1.225 (동일 증폭)
```

> **주의**: avg.c는 부호와 무관하게 절댓값으로 자기 행동 감정을 증폭시킨다.
> "엄격한 기준을 가졌기 때문에 강함"과 "반규범적이기 때문에 강함" 모두 반응 크기는 같다.

---

## 핵심 변수 해설

### `standards_amp` (Standards Amplifier, 자기 기준 배율)

C 차원 전체 평균의 절댓값이 자기 행동·타인 행동 평가 감정의 강도를 증폭시키는 배율이다.

```
standards_amp = 1.0 + |avg.c| × 0.3
```

| avg.c 값 | |avg.c| | standards_amp | 의미 |
|----------|--------|---------------|------|
| +1.0 또는 -1.0 | 1.0 | 1.30 | Pride/Shame 등 30% 증폭 |
| +0.5 또는 -0.5 | 0.5 | 1.15 | 15% 증폭 |
| 0.0 | 0.0 | 1.00 | 기준값 |

`standards_amp`가 적용되는 감정: Pride, Shame, Admiration, Reproach, Gratification, Remorse

### `impulse_mod` (Impulse Modifier, 충동 억제 계수)

C.prudence(신중함)만 단독으로 관여하는 Distress 충동 억제 계수다.

```
impulse_mod = 1.0 - prudence.max(0.0) × 0.3
```

| prudence | impulse_mod | 결과 |
|---------|-------------|------|
| +1.0 (매우 신중) | 0.70 | Distress 30% 억제 |
| +0.5 | 0.85 | Distress 15% 억제 |
| 0.0 | 1.00 | 억제 없음 |
| -1.0 (충동적) | 1.00 | 억제 없음 (`max(0.0)`으로 음수 차단) |

`impulse_mod`는 Distress 강도 계산에만 곱해진다:
```
Distress = desirability_self.abs() × emotional_amp × impulse_mod
```

---

## C 4개 Facet × OCC 3 브랜치 연결 — 감정별 상세

### Event 브랜치 — C.prudence 단독 개입

자신의 사건·타인의 사건 감정 대부분은 C와 **무관**하다.
**Distress 충동 반응**에만 C.prudence가 관여한다.

---

#### Distress (고통) — C.prudence 관여

| 항목 | 내용 |
|------|------|
| **발동 조건** | desirability_self < 0 |
| **강도 공식** | `desirability_self.abs() × emotional_amp × impulse_mod` |
| **impulse_mod** | `1.0 - prudence.max(0.0) × 0.3` |
| **해석** | 신중한 NPC는 불쾌한 상황에서도 즉각적인 감정 폭발 대신 절제된 반응을 보임 |

---

#### 그 외 Event 감정 — C 관여 없음

| 감정 | 담당 |
|------|------|
| Joy | X.positive_amp |
| Fear | E.emotional_amp, E.fearfulness |
| Hope | X.positive_amp |
| HappyFor/Resentment/Pity/Gloating | H, A, E.sentimentality |

---

### Action 브랜치 — `avg.c` 전체 평균 사용

자기 행동 또는 타인 행동을 평가하는 모든 Action 감정에 `standards_amp`가 곱해진다.

---

#### Pride (자부심)

| 항목 | 내용 |
|------|------|
| **발동 조건** | is_self_agent = true, praiseworthiness > 0 |
| **강도 공식** | `praiseworthiness × standards_amp × pride_mod` |
| **C 역할** | standards_amp — C가 극단적일수록 자신의 칭찬받을 행동에 더 강하게 반응 |

---

#### Shame (수치심)

| 항목 | 내용 |
|------|------|
| **발동 조건** | is_self_agent = true, praiseworthiness < 0 |
| **강도 공식** | `praiseworthiness.abs() × standards_amp` |
| **C 역할** | standards_amp — 완벽주의 NPC는 자신의 실수에 더 강한 수치심을 느낌 |

---

#### Admiration (감탄)

| 항목 | 내용 |
|------|------|
| **발동 조건** | is_self_agent = false, praiseworthiness > 0 |
| **강도 공식** | `praiseworthiness × standards_amp` |
| **C 역할** | standards_amp — C가 높을수록 타인의 훌륭한 행동을 더 강하게 감탄 |

---

#### Reproach (비난)

| 항목 | 내용 |
|------|------|
| **발동 조건** | is_self_agent = false, praiseworthiness < 0 |
| **강도 공식** | `praiseworthiness.abs() × standards_amp × reproach_amp` |
| **C 역할** | standards_amp — A.gentleness와 함께 작용 |

---

#### Gratification (자기만족) / Remorse (후회)

| 항목 | 내용 |
|------|------|
| **발동 조건** | 자기 행동 + 자신에 대한 결과가 둘 다 긍정/부정 |
| **강도 공식** | `(praiseworthiness + outcome) / 2.0 × standards_amp` |
| **C 역할** | standards_amp |

---

### Object 브랜치 — C 관여 없음

| 감정 | 담당 |
|------|------|
| Love | O.aesthetic_appreciation |
| Hate | O.aesthetic_appreciation |

C 4개 Facet 모두 Object 브랜치에 영향 없다.

---

## Facet별 담당 감정 한눈에 보기

```
C 차원 (Conscientiousness)
├── Organization (조직력)
│   └── 직접 감정 없음
│       avg.c에 1/4 기여
│
├── Diligence (근면)
│   └── 직접 감정 없음
│       avg.c에 1/4 기여
│
├── Perfectionism (완벽주의)
│   └── 직접 감정 없음
│       avg.c에 1/4 기여
│
└── Prudence (신중함)
    └── Event 브랜치: Distress 충동 억제 (-최대 30%)
        avg.c에 1/4 기여

avg.c (4 Facet 평균, 절댓값 사용)
  |avg.c| 높음 → Action: Pride/Shame/Admiration/Reproach/Gratification/Remorse 전체 증폭
```

---

## 이론적 근거 분석

각 설계 결정에 대한 학술적 근거 유무를 정리한다.

### ✅ 근거 있음

**Prudence가 Distress 충동 억제에 단독 사용됨**

HEXACO 연구에서 prudence는 "충동을 억제하고 결과를 고려하는 성질"을 측정한다.
충동적인 감정 폭발을 억제하는 것은 prudence가 직접 담당하며,
organization, diligence, perfectionism과는 구분되는 독립적인 심리 구인이다.

**avg.c.abs() 사용 — 양방향 모두 증폭**

성실성이 높은 NPC(높은 기준)와 낮은 NPC(반규범적 성향) 모두 자기 행동에 대한 감정이 강하다.
완벽주의자는 잘못에 강한 수치심을 느끼고, 규범을 무시하는 NPC는 오히려 그 반대의 자기 정당화 감정이 강하다.

참고문헌:
- Ashton, M. C., & Lee, K. (2007). Empirical, theoretical, and practical advantages of the HEXACO model. *Personality and Social Psychology Review*, 11(2), 150-166.

### ❌ 엔지니어링 설계값 (학술 근거 없음)

| 값 | 설명 | 튜닝 방향 |
|----|------|---------|
| **0.3 계수** (standards_amp) | avg.c가 자기 행동 감정에 미치는 비율 | 높이면 C의 영향력 증가 |
| **0.3 계수** (impulse_mod) | prudence가 Distress 억제에 미치는 비율 | 높이면 신중함의 억제 효과 강화 |

---

## C 차원과 EmotionalMomentum

### EmotionalMomentum 현재 구성

`EmotionalMomentum`은 **이전 감정 상태가 새 평가에 미치는 심리적 관성**을 수치화한 구조체다.
`appraise_with_context()`를 통해 대화 맥락을 유지할 때만 활성화된다.

### C 관련 감정에서 Momentum이 개입하는 경로

C가 관여하는 감정들 중 `EmotionalMomentum`이 직접 개입하는 경로는 **없다.**

| C 관련 감정 | Momentum 개입 | 개입 필드 |
|------------|-------------|---------|
| Distress | 없음 (impulse_mod는 Momentum과 독립) | — |
| Pride, Shame | 없음 | — |
| Admiration, Reproach | Reproach에 negative_bias 개입 (단, standards_amp와는 독립) | `negative_bias` |
| Gratification, Remorse | 없음 | — |

> **참고**: Reproach는 `reproach_amp = (1.0 - gentleness × 0.3) + m.negative_bias`로 계산되므로
> Momentum(negative_bias)이 개입하지만, 이는 A.gentleness 경로이며 C의 standards_amp와는 독립이다.

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|---------|
| 0.1.0 | 2026-03-24 | 초기 작성. C 4 Facet 상세, avg.c, 핵심 변수 해설, OCC 브랜치별 감정 공식, 이론적 근거 분석, EmotionalMomentum 관계 정리 |
