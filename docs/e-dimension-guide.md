# E 차원 (Emotionality) 완전 가이드

## 개요

E = **Emotionality (정서성)**

HEXACO 성격 모델의 6개 차원 중 두 번째 차원이다.
사람이 감정적으로 얼마나 민감하게 반응하고, 불안 및 의존 성향을 얼마나 갖는지를 측정한다.

| E 수준 | NPC 성향 |
|--------|---------|
| **높음 (+)** | 감정 기복이 크고, 두려움과 불안을 잘 느끼며, 타인에게 공감하고 의지하는 성향 |
| **낮음 (-)** | 감정적으로 안정적이고, 위험에 덤덤하며, 독립적인 성향 |

무협 예시:
- E 높음: 감수성 예민한 시인 검객, 상처받기 쉬운 귀족 자제, 타인의 고통에 눈물 흘리는 치료사
- E 낮음: 냉정한 암살자, 두려움 없는 전사, 감정 없는 노련한 첩자

---

## 4개 Facet 상세

E 차원은 **4개 Facet(하위 성격 요소)**으로 구성된다.
각 Facet은 독립적인 수치를 가지며 범위는 **-1.0 ~ +1.0**이다.

---

### Facet 1. Fearfulness (두려움)

> **"이 NPC는 위험 상황에서 공포를 느끼는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 위협에 쉽게 겁먹음. 위험을 미리 감지하고 회피하려 함. 타인의 위협에 매우 민감 |
| 0.0 (중간) | 상황에 따라 두려움을 느끼기도, 무시하기도 함 |
| -1.0 (낮음) | 위험에도 덤덤함. 두려움이 거의 없음. 무모한 행동을 서슴지 않음 |

**연결 감정 (Event 브랜치)**: `Fear (공포)` 추가 증폭
- 공식: `fear_amp = 1.0 + fearfulness.max(0.0) × 0.5`
- 두려움 Facet이 높을수록 Fear 감정이 추가로 강해짐
- Anxiety, Dependence, Sentimentality는 Fear 전용 증폭에 관여하지 않음
- `emotional_amp` 경유의 간접 증폭과 별개로 Fear에 한해 추가 배율이 곱해짐

---

### Facet 2. Anxiety (불안)

> **"이 NPC는 미래의 위협이나 불확실성에 대해 걱정하는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 걱정이 많음. 최악의 상황을 예상함. 작은 일에도 스트레스를 받음 |
| 0.0 (중간) | 적당히 걱정하되 크게 흔들리지 않음 |
| -1.0 (낮음) | 결과에 대해 걱정하지 않음. 낙천적. 위기에도 침착함 |

**연결 감정**: 직접 연결된 단독 감정 없음
- avg.e(E 평균)에 1/4 기여 → `emotional_amp` 경유로 모든 감정 강도에 간접 기여

---

### Facet 3. Dependence (의존성)

> **"이 NPC는 타인의 도움이나 정서적 지지를 필요로 하는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 타인의 도움을 원함. 혼자 결정하는 것을 두려워함. 지지가 없으면 불안해함 |
| 0.0 (중간) | 상황에 따라 의존하기도, 독립적으로 행동하기도 함 |
| -1.0 (낮음) | 자립적. 타인의 지지 없이 행동. 감정적으로 독립적 |

**연결 감정**: 직접 연결된 단독 감정 없음
- avg.e(E 평균)에 1/4 기여 → `emotional_amp` 경유로 모든 감정 강도에 간접 기여

---

### Facet 4. Sentimentality (감상성)

> **"이 NPC는 타인의 감정에 공감하고 감정적으로 연결되는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 타인의 고통에 쉽게 감동받음. 연민이 깊음. 공감 능력이 뛰어남 |
| 0.0 (중간) | 상황에 따라 공감하기도, 무감각하기도 함 |
| -1.0 (낮음) | 타인의 감정에 무뎜. 연민을 잘 느끼지 않음. 냉정하게 판단 |

**연결 감정 (Event 브랜치)**: `Pity (동정)` 발동 조건
- 조건: `avg.a > 0.0 || sentimentality > 0.0`
- avg.a(원만성)가 낮더라도 sentimentality만 높으면 Pity가 발동됨
- Fearfulness, Anxiety, Dependence는 Pity 조건에 관여하지 않음

---

## avg.e — E 차원 수치 평균

```
avg.e = (fearfulness + anxiety + dependence + sentimentality) / 4
범위: -1.0 ~ +1.0
```

4개 Facet이 **모두 동등하게** E 평균에 기여한다.
이 평균값은 `|avg.e|` (절댓값)로 사용된다 — **E가 높든 낮든 감정 반응이 강해진다.**

```
emotional_amp = 1.0 + |avg.e| × 0.3 + m.sensitivity_boost
```

예시:
```
NPC 설화: fearfulness=0.7, anxiety=0.6, dependence=0.5, sentimentality=0.8
          → avg.e = 0.65  → emotional_amp = 1.195 (기본 대비 19.5% 증폭)

NPC 냉검: fearfulness=-0.6, anxiety=-0.7, dependence=-0.8, sentimentality=-0.5
          → avg.e = -0.65  → |avg.e| = 0.65  → emotional_amp = 1.195 (동일 증폭)
```

> **주의**: avg.e는 부호와 무관하게 절댓값으로 감정 강도를 증폭시킨다.
> "감정이 풍부해서 강함"과 "감정이 메말라서 강함"은 방향이 다르지만, 반응 크기는 같다.

---

## 핵심 변수 해설

### `emotional_amp` (Emotional Amplifier, 감정 민감도 배율)

E 차원 전체 평균의 절댓값이 모든 Event 감정의 강도를 증폭시키는 전역 배율이다.

```
emotional_amp = 1.0 + |avg.e| × 0.3 + m.sensitivity_boost
```

| avg.e 값 | |avg.e| | emotional_amp (맥락 없을 때) | 의미 |
|----------|--------|------------------------------|------|
| +1.0 또는 -1.0 | 1.0 | 1.30 | 모든 감정 30% 증폭 |
| +0.5 또는 -0.5 | 0.5 | 1.15 | 모든 감정 15% 증폭 |
| 0.0 | 0.0 | 1.00 | 기준값 |

### `fear_amp` (Fear Amplifier, 공포 전용 배율)

E.fearfulness(두려움)만 단독으로 관여하는 Fear 전용 배율이다.

```
fear_amp = 1.0 + fearfulness.max(0.0) × 0.5
```

| fearfulness | fear_amp | 결과 |
|------------|---------|------|
| +1.0 (매우 두려움) | 1.50 | Fear 50% 추가 증폭 |
| +0.5 | 1.25 | Fear 25% 추가 증폭 |
| 0.0 | 1.00 | 추가 증폭 없음 |
| -1.0 (담력 있음) | 1.00 | 추가 증폭 없음 (`max(0.0)`으로 음수 차단) |

Fear 최종 강도: `desirability_self.abs() × fear_amp × emotional_amp`
→ fearfulness가 높으면 두 배율이 중첩된다.

### `sentimentality` (코드 내 단독 변수)

E 차원 4개 Facet 중 **감상성(Sentimentality) 하나의 단독 수치**다.
`p.emotionality.sentimentality.value()`로 참조하며 Pity 발동 조건에서만 단독 사용된다.
avg.e와 구별하여 주의해야 한다.

---

## E 4개 Facet × OCC 3 브랜치 연결 — 감정별 상세

### Event 브랜치 — `avg.e` 전역 증폭 + Facet 단독 개입

---

#### emotional_amp 적용 감정 (avg.e 전체 평균)

`emotional_amp`는 아래 감정들의 강도에 일괄 곱해진다:

| 감정 | 공식 (일부) |
|------|-----------|
| **Joy** | `desirability_self × positive_amp` (positive_amp는 별도) |
| **Distress** | `desirability_self.abs() × emotional_amp × impulse_mod` |
| **Hope** | `base × positive_amp` |
| **Fear** | `desirability_self.abs() × fear_amp × emotional_amp` |
| **Satisfaction** | `base × positive_amp` |
| **Disappointment** | `base × emotional_amp` |
| **Relief** | `base × positive_amp` |
| **FearsConfirmed** | `base × emotional_amp` |

---

#### Fear (공포) — E.fearfulness 추가 관여

| 항목 | 내용 |
|------|------|
| **발동 조건** | desirability_self < 0, is_prospective = true |
| **강도 공식** | `desirability_self.abs() × fear_amp × emotional_amp` |
| **fear_amp** | `1.0 + fearfulness.max(0.0) × 0.5` |
| **해석** | emotional_amp (avg.e 기반)와 fear_amp (fearfulness 단독)가 함께 곱해짐 |

---

#### Pity (동정) — E.sentimentality 단독 관여

| 항목 | 내용 |
|------|------|
| **발동 조건** | desir_other < 0, `avg.a > 0.0 OR sentimentality > 0.0` |
| **해석** | 원만성이 낮더라도 감상성이 높으면 타인의 불행에 동정이 발동됨 |
| **Fearfulness, Anxiety, Dependence** | Pity에 관여하지 않음 |

---

### Action 브랜치 — E 관여 없음

| 감정 | 담당 |
|------|------|
| Pride, Shame | C.standards_amp / H.modesty |
| Admiration, Reproach | C.standards_amp / A.gentleness |
| Gratitude, Anger | H.sincerity / A.patience |
| Gratification, Remorse | C.standards_amp |

E 4개 Facet 모두 Action 브랜치에 영향 없다.

---

### Object 브랜치 — E 관여 없음

| 감정 | 담당 |
|------|------|
| Love | O.aesthetic_appreciation |
| Hate | O.aesthetic_appreciation |

E 4개 Facet 모두 Object 브랜치에 영향 없다.

---

## Facet별 담당 감정 한눈에 보기

```
E 차원 (Emotionality)
├── Fearfulness (두려움)
│   └── Event 브랜치: Fear 추가 증폭 (+최대 50%)
│       avg.e에 1/4 기여
│
├── Anxiety (불안)
│   └── 직접 감정 없음
│       avg.e에 1/4 기여
│
├── Dependence (의존성)
│   └── 직접 감정 없음
│       avg.e에 1/4 기여
│
└── Sentimentality (감상성)
    └── Event 브랜치: Pity 발동 조건
        avg.e에 1/4 기여

avg.e (4 Facet 평균, 절댓값 사용)
  |avg.e| 높음 → Event: Joy/Distress/Fear/Hope/Satisfaction/Disappointment/Relief/FearsConfirmed 전체 증폭
```

---

## 이론적 근거 분석

각 설계 결정에 대한 학술적 근거 유무를 정리한다.

### ✅ 근거 있음

**Fearfulness가 Fear 전용으로 단독 사용됨**

HEXACO 연구에서 fearfulness는 "물리적 위험에 대한 공포 경험 및 신체적 해악을 회피하는 성향"을
직접 측정하도록 설계되었다. OCC의 Fear(예상되는 불쾌한 사건에 대한 두려움)와 직접 대응된다.

**Sentimentality가 Pity 발동 조건에 관여함**

HEXACO 연구에서 sentimentality는 "타인과 감정적으로 연결되고 공감을 느끼는 성향"을 측정한다.
타인의 불행에 반응하는 Pity와 직접 대응된다.

**avg.e.abs() 사용 — 양방향 모두 증폭**

외향성(X)이 긍정 감정만 증폭시키는 것과 달리, E는 정서성의 극단적 방향 자체(높든 낮든)가
감정 반응의 강도를 결정하는 요소다.
매우 감정적인 NPC와 매우 냉정한 NPC 모두 자신의 방식으로 강하게 반응하지만,
반응의 방향(긍정/부정)은 상황과 다른 차원(H, A, X 등)이 결정한다.

참고문헌:
- Ashton, M. C., & Lee, K. (2007). Empirical, theoretical, and practical advantages of the HEXACO model. *Personality and Social Psychology Review*, 11(2), 150-166.

### ❌ 엔지니어링 설계값 (학술 근거 없음)

| 값 | 설명 | 튜닝 방향 |
|----|------|---------|
| **0.3 계수** (emotional_amp) | avg.e가 전체 감정에 미치는 비율 | 높이면 E의 영향력 증가 |
| **0.5 계수** (fear_amp) | fearfulness가 Fear에 미치는 비율 — 0.3보다 강하게 설정 | 낮추면 Fear 전용 증폭 약화 |

---

## E 차원과 EmotionalMomentum

### EmotionalMomentum 현재 구성

`EmotionalMomentum`은 **이전 감정 상태가 새 평가에 미치는 심리적 관성**을 수치화한 구조체다.
`appraise_with_context()`를 통해 대화 맥락을 유지할 때만 활성화된다.

**계산 공식**:
```
sensitivity_boost = (fear_intensity + distress_intensity) / 2.0 × 0.3
```

### E 관련 감정에서 Momentum이 개입하는 경로

E가 관여하는 감정들 중 `EmotionalMomentum`이 개입하는 경우는 **emotional_amp 하나**이며,
`sensitivity_boost`를 통한 간접 개입이다.

```
emotional_amp = 1.0 + |avg.e| × 0.3 + m.sensitivity_boost
```

- 이전 대화에서 Fear/Distress가 쌓였을 때 → `sensitivity_boost` 상승
- 다음 평가에서 `emotional_amp` 증가 → 모든 감정 반응이 더 강해짐

| E 관련 감정 | Momentum 개입 | 개입 필드 |
|------------|-------------|---------|
| Joy/Distress/Hope/Fear 등 | 간접 (emotional_amp 경유) | `sensitivity_boost` |
| Pity | 없음 | — |

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|---------|
| 0.1.0 | 2026-03-24 | 초기 작성. E 4 Facet 상세, avg.e, 핵심 변수 해설, OCC 브랜치별 감정 공식, 이론적 근거 분석, EmotionalMomentum 관계 정리 |
