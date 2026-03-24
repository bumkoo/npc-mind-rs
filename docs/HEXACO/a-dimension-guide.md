# A 차원 (Agreeableness) 완전 가이드

## 개요

A = **Agreeableness (원만성)**

HEXACO 성격 모델의 6개 차원 중 네 번째 차원이다.
사람이 타인과의 갈등에서 얼마나 온화하고 인내하며 용서하는 성향을 갖는지를 측정한다.

| A 수준 | NPC 성향 |
|--------|---------|
| **높음 (+)** | 용서하고 온화하며, 유연하고 인내심이 강함. 분노와 적대감이 적음 |
| **낮음 (-)** | 까다롭고 공격적이며, 완고하고 분노를 잘 표출함 |

무협 예시:
- A 높음: 자비로운 고승, 온화한 의원, 포용력 있는 문파 장로
- A 낮음: 격렬한 복수자, 냉혹한 무관, 불같은 성격의 투사

---

## 4개 Facet 상세

A 차원은 **4개 Facet(하위 성격 요소)**으로 구성된다.
각 Facet은 독립적인 수치를 가지며 범위는 **-1.0 ~ +1.0**이다.

---

### Facet 1. Forgiveness (용서)

> **"이 NPC는 자신에게 해를 끼친 자를 용서하는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 원한을 오래 품지 않음. 과거의 잘못을 쉽게 잊음. 화해를 먼저 청함 |
| 0.0 (중간) | 상황에 따라 용서하기도, 기억하기도 함 |
| -1.0 (낮음) | 원한을 오래 기억함. 복수심이 강함. 화해를 쉽게 받아들이지 않음 |

**연결 감정**: 직접 연결된 단독 감정 없음
- avg.a(A 평균)에 1/4 기여 → `anger_mod` 경유로 Resentment에 간접 기여

---

### Facet 2. Gentleness (온화함)

> **"이 NPC는 타인을 비판하거나 판단할 때 부드럽게 하는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 타인의 실수를 관대하게 봄. 부드럽게 비판함. 상대방의 입장을 먼저 헤아림 |
| 0.0 (중간) | 상황에 따라 관대하기도, 비판적이기도 함 |
| -1.0 (낮음) | 타인의 잘못을 날카롭게 지적함. 비판이 가혹함. 상대의 잘못을 쉽게 용납하지 않음 |

**연결 감정 (Action 브랜치)**: `Reproach (비난)` 증폭
- 공식: `reproach_amp = (1.0 - gentleness × 0.3) + m.negative_bias`
- 온화함이 낮을수록 타인의 비난받을 행동에 더 강하게 반응
- Forgiveness, Flexibility, Patience는 Reproach에 관여하지 않음

---

### Facet 3. Flexibility (유연성)

> **"이 NPC는 자기 의견을 양보하고 타협하는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 의견 충돌에서 먼저 양보함. 타협을 선호함. 고집을 부리지 않음 |
| 0.0 (중간) | 상황에 따라 타협하기도, 주장을 굽히지 않기도 함 |
| -1.0 (낮음) | 자기 의견을 고집함. 논쟁에서 쉽게 물러서지 않음. 완고함 |

**연결 감정**: 직접 연결된 단독 감정 없음
- avg.a(A 평균)에 1/4 기여

---

### Facet 4. Patience (인내)

> **"이 NPC는 좌절이나 도발에도 침착함을 유지하는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 도발에도 침착함을 유지함. 분노를 잘 드러내지 않음. 참을성이 강함 |
| 0.0 (중간) | 한계를 넘으면 화를 내지만 평소에는 참음 |
| -1.0 (낮음) | 작은 자극에도 분노가 폭발함. 참을성이 없음. 쉽게 흥분함 |

**연결 감정 (Action 브랜치)**: `Anger (분노)` 증폭
- 공식: `anger_amp = (1.0 - patience × 0.4) + m.anger_erosion + m.negative_bias`
- 인내심이 낮을수록 타인의 비난받을 행동에 더 강하게 분노함
- Forgiveness, Gentleness, Flexibility는 Anger에 관여하지 않음

---

## avg.a — A 차원 수치 평균

```
avg.a = (forgiveness + gentleness + flexibility + patience) / 4
범위: -1.0 ~ +1.0
```

4개 Facet이 **모두 동등하게** A 평균에 기여한다.
이 평균값은 Event 브랜치와 Action 브랜치 모두에서 사용된다.

```
// Event 브랜치 — anger_mod
anger_mod = (1.0 - avg.a × 0.4) + m.anger_erosion + m.negative_bias

// Event 브랜치 — HappyFor 공감 배율
empathy = (avg.h.max(0.0) + avg.a.max(0.0)) / 2.0

// Event 브랜치 — Gloating 발동 조건
avg.a < -0.2 AND avg.h < -0.2
```

예시:
```
NPC 자비: forgiveness=0.8, gentleness=0.7, flexibility=0.6, patience=0.9
          → avg.a = 0.75  → anger_mod = 0.70  (분노 30% 억제)

NPC 격노: forgiveness=-0.5, gentleness=-0.6, flexibility=-0.4, patience=-0.9
          → avg.a = -0.60  → anger_mod = 1.24  (분노 24% 증폭)
```

---

## 핵심 변수 해설

### `avg.a` / `anger_mod` (Anger Modulation, 분노 조절 계수)

A 차원 전체 평균이 분노·적대 감정을 조절하는 배율이다.

```
anger_mod = (1.0 - avg.a × 0.4) + m.anger_erosion + m.negative_bias
```

| avg.a 값 | anger_mod (맥락 없을 때) | 의미 |
|----------|------------------------|------|
| +1.0 (매우 원만) | 0.60 | 분노 계열 감정 40% 억제 |
| 0.0 (중간) | 1.00 | 기준값 |
| -1.0 (매우 비우호적) | 1.40 | 분노 계열 감정 40% 증폭 |

`anger_mod`는 Event 브랜치의 **Resentment**에 사용된다.

### `gentleness` (코드 내 단독 변수)

A 차원 4개 Facet 중 **온화함(Gentleness) 하나의 단독 수치**다.
`p.agreeableness.gentleness.value()`로 참조하며 Action 브랜치 Reproach에서만 단독 사용된다.

```
reproach_amp = (1.0 - gentleness × 0.3) + m.negative_bias
```

| gentleness | reproach_amp (맥락 없을 때) | 의미 |
|-----------|--------------------------|------|
| +1.0 (매우 온화) | 0.70 | Reproach 30% 억제 |
| 0.0 (중간) | 1.00 | 기준값 |
| -1.0 (가혹함) | 1.30 | Reproach 30% 증폭 |

### `patience` (코드 내 단독 변수)

A 차원 4개 Facet 중 **인내(Patience) 하나의 단독 수치**다.
`p.agreeableness.patience.value()`로 참조하며 Action 브랜치 Anger에서만 단독 사용된다.

```
anger_amp = (1.0 - patience × 0.4) + m.anger_erosion + m.negative_bias
```

| patience | anger_amp (맥락 없을 때) | 의미 |
|---------|------------------------|------|
| +1.0 (매우 인내) | 0.60 | Anger 40% 억제 |
| 0.0 (중간) | 1.00 | 기준값 |
| -1.0 (참을성 없음) | 1.40 | Anger 40% 증폭 |

---

## A 4개 Facet × OCC 3 브랜치 연결 — 감정별 상세

### Event 브랜치 — `avg.a` 전체 평균 사용

---

#### HappyFor (타인의 행운에 대리기쁨) — avg.a 관여

| 항목 | 내용 |
|------|------|
| **발동 조건** | avg.h > 0.0 OR avg.a > 0.0, desir_other > 0 |
| **강도 공식** | `desir_other × (0.5 + (avg.h.max(0) + avg.a.max(0)) / 2 × 0.5)` |
| **A 역할** | 공감 배율(empathy)에 avg.a.max(0.0)가 기여 — A가 높을수록 타인의 행운을 더 기뻐함 |

---

#### Resentment (타인의 행운에 질투) — avg.a 관여 (anger_mod 경유)

| 항목 | 내용 |
|------|------|
| **발동 조건** | avg.h < -0.2 AND desir_other > 0 |
| **강도 공식** | `desir_other × \|avg.h\| × anger_mod` |
| **A 역할** | anger_mod = `(1.0 - avg.a × 0.4) + m.anger_erosion + m.negative_bias` — A가 낮을수록 Resentment 증폭 |

---

#### Gloating (타인의 불행에 쾌재) — avg.a 발동 조건 및 강도

| 항목 | 내용 |
|------|------|
| **발동 조건** | avg.h < -0.2 **AND** avg.a < -0.2 (A만 낮아선 부족, H와 복합 조건) |
| **강도 공식** | `\|desir_other\| × (\|avg.h\| + \|avg.a\|) / 2` |
| **해석** | H↓(이기심) + A↓(공격성)의 복합 조건. A가 낮을수록 Gloating 강도 증가 |

---

#### Pity (타인의 불행에 동정) — avg.a 발동 조건

| 항목 | 내용 |
|------|------|
| **발동 조건** | desir_other < 0, `avg.a > 0.0 OR sentimentality > 0.0` |
| **A 역할** | avg.a가 양수이면 Pity 발동 조건 충족 (E.sentimentality와 OR 관계) |

---

### Action 브랜치 — Facet 개별값 직접 사용

Event 브랜치와 달리 avg.a가 아닌 **특정 Facet 하나**만 단독으로 관여한다.

---

#### Reproach (비난) — A.gentleness 관여

| 항목 | 내용 |
|------|------|
| **발동 조건** | is_self_agent = false, praiseworthiness < 0 |
| **조절 공식** | `reproach_amp = (1.0 - gentleness × 0.3) + m.negative_bias` |
| **Forgiveness, Flexibility, Patience** | Reproach에 관여하지 않음 |

---

#### Anger (분노) — A.patience 관여

| 항목 | 내용 |
|------|------|
| **발동 조건** | is_self_agent = false, praiseworthiness < 0, outcome_for_self < 0 |
| **조절 공식** | `anger_amp = (1.0 - patience × 0.4) + m.anger_erosion + m.negative_bias` |
| **Forgiveness, Gentleness, Flexibility** | Anger에 관여하지 않음 |

---

#### Gratitude (감사) — A 관여 없음

| 항목 | 내용 |
|------|------|
| **발동 조건** | is_self_agent = false, praiseworthiness > 0, outcome_for_self > 0 |
| **A 관여** | 없음 — H.sincerity(진실성)가 담당 |

---

### Object 브랜치 — A 관여 없음

| 감정 | 담당 |
|------|------|
| Love | O.aesthetic_appreciation |
| Hate | O.aesthetic_appreciation |

A 4개 Facet 모두 Object 브랜치에 영향 없다.

---

## Facet별 담당 감정 한눈에 보기

```
A 차원 (Agreeableness)
├── Forgiveness (용서)
│   └── 직접 감정 없음
│       avg.a에 1/4 기여
│
├── Gentleness (온화함)
│   └── Action 브랜치: Reproach 증폭 (+최대 30%)
│       avg.a에 1/4 기여
│
├── Flexibility (유연성)
│   └── 직접 감정 없음
│       avg.a에 1/4 기여
│
└── Patience (인내)
    └── Action 브랜치: Anger 증폭 (+최대 40%)
        avg.a에 1/4 기여

avg.a (4 Facet 평균)
  > 0.0         → Event: HappyFor 공감 배율 기여
                  Event: Pity 발동 조건
  < -0.2
  + avg.h < -0.2 → Event: Gloating (H와 복합 조건)
  전체 범위      → Event: Resentment의 anger_mod 조절
```

---

## 이론적 근거 분석

각 설계 결정에 대한 학술적 근거 유무를 정리한다.

### ✅ 근거 있음

**Action 브랜치에서 Facet 개별값 사용**

HEXACO 연구에서 각 Facet은 서로 다른 심리 구인을 독립적으로 측정한다:
- Patience = "좌절에도 화내지 않는 성질" → Anger(타인의 나쁜 행동에 대한 분노)와 직접 대응
- Gentleness = "타인을 가혹하게 판단하지 않는 성질" → Reproach(타인 행동 비난)와 직접 대응

avg.a를 사용하면 Forgiveness, Flexibility가 Anger/Reproach를 희석시켜 심리적으로 맞지 않는 결과가 된다.

**Event 브랜치에서 avg.a 사용**

타인에 대한 감정(HappyFor, Resentment, Gloating)은 개별 Facet이 아닌 A 차원 전체의 "적대성/친화성"이 결정한다.

참고문헌:
- Ashton, M. C., & Lee, K. (2007). Empirical, theoretical, and practical advantages of the HEXACO model. *Personality and Social Psychology Review*, 11(2), 150-166.

### ❌ 엔지니어링 설계값 (학술 근거 없음)

| 값 | 설명 | 튜닝 방향 |
|----|------|---------|
| **-0.2 임계값** | Gloating 발동 기준 | 낮추면 발동 범위 좁아짐 |
| **0.4 계수** (anger_mod, anger_amp) | A/patience가 분노에 미치는 비율 | 높이면 A의 영향력 증가 |
| **0.3 계수** (reproach_amp) | gentleness가 Reproach에 미치는 비율 | 플레이테스트로 검증 필요 |

---

## A 차원과 EmotionalMomentum

### EmotionalMomentum 현재 구성

`EmotionalMomentum`은 **이전 감정 상태가 새 평가에 미치는 심리적 관성**을 수치화한 구조체다.
`appraise_with_context()`를 통해 대화 맥락을 유지할 때만 활성화된다.

**관련 필드**:
```
anger_erosion  = anger_intensity × 0.5
negative_bias  = valence.min(0.0).abs() × 0.5
```

### A 관련 감정에서 Momentum이 개입하는 경로

A가 관여하는 감정들 중 `EmotionalMomentum`이 개입하는 경우는 **anger_mod, reproach_amp, anger_amp** 세 곳이며,
모두 `anger_erosion`과 `negative_bias`를 통한 간접 개입이다.

```
anger_mod    = (1.0 - avg.a × 0.4)    + m.anger_erosion + m.negative_bias
reproach_amp = (1.0 - gentleness × 0.3) + m.negative_bias
anger_amp    = (1.0 - patience × 0.4)  + m.anger_erosion + m.negative_bias
```

| A 관련 감정 | Momentum 개입 | 개입 필드 |
|------------|-------------|---------|
| HappyFor | 없음 | — |
| Resentment | 간접 (anger_mod 경유) | `anger_erosion`, `negative_bias` |
| Gloating | 없음 | — |
| Pity | 없음 | — |
| Reproach | 간접 (reproach_amp 경유) | `negative_bias` |
| Anger | 간접 (anger_amp 경유) | `anger_erosion`, `negative_bias` |

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|---------|
| 0.1.0 | 2026-03-24 | 초기 작성. A 4 Facet 상세, avg.a, 핵심 변수 해설, OCC 브랜치별 감정 공식, 이론적 근거 분석, EmotionalMomentum 관계 정리 |
