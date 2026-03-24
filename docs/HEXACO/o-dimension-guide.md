# O 차원 (Openness to Experience) 완전 가이드

## 개요

O = **Openness to Experience (경험에 대한 개방성)**

HEXACO 성격 모델의 6개 차원 중 여섯 번째 차원이다.
사람이 얼마나 새로운 경험과 예술적 아름다움에 열려 있으며, 창의적이고 호기심이 많은지를 측정한다.

| O 수준 | NPC 성향 |
|--------|---------|
| **높음 (+)** | 새로운 경험에 열려 있고, 예술적 감수성이 뛰어나며, 창의적이고 호기심이 많은 성향 |
| **낮음 (-)** | 전통적이고 관습적이며, 실용적. 예술보다 익숙하고 검증된 것을 선호하는 성향 |

무협 예시:
- O 높음: 시서화(詩書畵)에 능한 풍류 검객, 창의적인 무공 개발자, 다양한 문화를 섭렵한 여행자
- O 낮음: 전통 기법만 고집하는 장인, 새로운 무공을 거부하는 보수 문파, 실용만 추구하는 용병

---

## 4개 Facet 상세

O 차원은 **4개 Facet(하위 성격 요소)**으로 구성된다.
각 Facet은 독립적인 수치를 가지며 범위는 **-1.0 ~ +1.0**이다.

---

### Facet 1. Aesthetic Appreciation (미적 감상)

> **"이 NPC는 예술, 자연, 아름다움에 강하게 반응하는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 음악·미술·자연에 깊이 감동받음. 아름다운 것에 강하게 이끌림. 예술 작품에 집착 |
| 0.0 (중간) | 좋은 것은 알지만 특별히 깊이 반응하지 않음 |
| -1.0 (낮음) | 예술적 아름다움에 무감각함. 실용성 위주로 판단. 미적 가치보다 기능을 중시 |

**연결 감정 (Object 브랜치)**: `Love (호감)` / `Hate (혐오)` 증폭
- 공식: `aesthetic_amp = 1.0 + aesthetic_appreciation.abs() × 0.3`
- 미적 감상 수치가 극단적일수록(높든 낮든) 대상에 대한 감정 반응이 강해짐
- Inquisitiveness, Creativity, Unconventionality는 Love/Hate에 관여하지 않음

---

### Facet 2. Inquisitiveness (탐구심)

> **"이 NPC는 지식과 새로운 정보를 열정적으로 추구하는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 새로운 지식을 탐구함. 여러 분야에 호기심이 많음. 알려지지 않은 것을 조사함 |
| 0.0 (중간) | 관심 분야에만 호기심을 가짐 |
| -1.0 (낮음) | 이미 아는 것에 만족. 새로운 정보를 찾으려 하지 않음. 실용적인 것만 추구 |

**연결 감정**: 직접 연결된 단독 감정 없음
- 감정 계산에 관여하지 않음

---

### Facet 3. Creativity (창의성)

> **"이 NPC는 새로운 아이디어와 독창적 해결책을 추구하는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 독창적인 방법을 즐김. 기존 방식을 넘어서는 아이디어를 추구. 상상력이 풍부 |
| 0.0 (중간) | 전통과 혁신을 적절히 혼합 |
| -1.0 (낮음) | 검증된 방식을 선호. 독창성보다 안정성. 새로운 시도를 꺼림 |

**연결 감정**: 직접 연결된 단독 감정 없음
- 감정 계산에 관여하지 않음

---

### Facet 4. Unconventionality (비관습성)

> **"이 NPC는 관습과 전통에 얽매이지 않고 독자적으로 행동하는가?"**

| 수치 | 행동 경향 |
|------|---------|
| +1.0 (높음) | 관습을 무시함. 사회 규범보다 자신의 기준을 따름. 기이하거나 독특한 취향을 가짐 |
| 0.0 (중간) | 대체로 관습을 따르지만 가끔 벗어나기도 함 |
| -1.0 (낮음) | 전통을 중시. 관습적인 행동을 선호. 틀을 벗어나는 것에 불편함을 느낌 |

**연결 감정**: 직접 연결된 단독 감정 없음
- 감정 계산에 관여하지 않음

---

## avg.o — O 차원 수치 평균

```
avg.o = (aesthetic_appreciation + inquisitiveness + creativity + unconventionality) / 4
범위: -1.0 ~ +1.0
```

> **중요**: 현재 감정 엔진에서 `avg.o`는 **사용되지 않는다.**
> O 차원에서 감정 계산에 실제로 사용되는 것은 `aesthetic_appreciation` **단독 수치**뿐이다.

```
aesthetic_amp = 1.0 + aesthetic_appreciation.abs() × 0.3
```

예시:
```
NPC 풍류: aesthetic_appreciation=0.9
          → aesthetic_amp = 1.27  (Love/Hate 27% 증폭)

NPC 무감: aesthetic_appreciation=-0.8
          → |aesthetic_appreciation| = 0.8  → aesthetic_amp = 1.24  (동일 방향 증폭)
```

> **주의**: `.abs()` 사용 — 미적 감상이 높든 낮든 모두 Love/Hate 강도를 증폭시킨다.
> 높은 aesthetic_appreciation: "아름다운 것에 깊이 매혹/혐오"
> 낮은 aesthetic_appreciation: "미적 관심 자체가 강한 특성값"

---

## 핵심 변수 해설

### `aesthetic_appreciation` (코드 내 단독 변수)

O 차원 4개 Facet 중 **미적 감상(Aesthetic Appreciation) 하나의 단독 수치**다.
`p.openness.aesthetic_appreciation.value()`로 참조하며 Object 브랜치에서만 사용된다.
avg.o와 별개로 직접 참조된다.

### `aesthetic_amp` (Aesthetic Amplifier, 미적 감정 배율)

미적 감상 수치의 절댓값이 Love/Hate 감정 강도를 증폭시키는 배율이다.

```
aesthetic_amp = 1.0 + aesthetic_appreciation.abs() × 0.3
```

| aesthetic_appreciation | aesthetic_amp | 의미 |
|----------------------|--------------|------|
| +1.0 (매우 높음) | 1.30 | Love/Hate 30% 증폭 |
| +0.5 | 1.15 | Love/Hate 15% 증폭 |
| 0.0 | 1.00 | 기준값 |
| -0.5 | 1.15 | 동일 증폭 (절댓값) |
| -1.0 (매우 낮음) | 1.30 | Love/Hate 30% 증폭 |

### avg.o 미사용 이유

inquisitiveness, creativity, unconventionality는 OCC 모델의 22가지 감정 중
직접 대응되는 감정이 없다. 이 세 Facet은 NPC의 인지·탐구 성향을 측정하지만,
감정 평가(appraise)의 결과물로 연결되는 OCC 감정 타입이 현재 구현에 없다.

---

## O 4개 Facet × OCC 3 브랜치 연결 — 감정별 상세

### Event 브랜치 — O 관여 없음

| 감정 | 담당 |
|------|------|
| Joy/Distress | X.positive_amp, E.emotional_amp |
| Fear/Hope | E.fearfulness, E.emotional_amp |
| HappyFor/Resentment | H.avg, A.avg |
| Pity | A.avg, E.sentimentality |
| Gloating | H.avg, A.avg |

O 4개 Facet 모두 Event 브랜치에 영향 없다.

---

### Action 브랜치 — O 관여 없음

| 감정 | 담당 |
|------|------|
| Pride, Shame | C.standards_amp / H.modesty |
| Admiration, Reproach | C.standards_amp / A.gentleness |
| Gratitude, Anger | H.sincerity / A.patience |
| Gratification, Remorse | C.standards_amp |

O 4개 Facet 모두 Action 브랜치에 영향 없다.

---

### Object 브랜치 — `aesthetic_appreciation` 단독 사용

대상에 대한 매력·혐오 반응에만 O가 관여하며, aesthetic_appreciation 하나만 사용된다.

---

#### Love (호감/매혹)

| 항목 | 내용 |
|------|------|
| **발동 조건** | appealingness > 0 |
| **강도 공식** | `appealingness × aesthetic_amp` |
| **aesthetic_amp** | `1.0 + aesthetic_appreciation.abs() × 0.3` |
| **해석** | 미적 감상이 극단적일수록 매력적인 대상에 더 강하게 이끌림 |

---

#### Hate (혐오/반감)

| 항목 | 내용 |
|------|------|
| **발동 조건** | appealingness < 0 |
| **강도 공식** | `appealingness.abs() × aesthetic_amp` |
| **aesthetic_amp** | `1.0 + aesthetic_appreciation.abs() × 0.3` |
| **해석** | 미적 감상이 극단적일수록 혐오스러운 대상에 더 강하게 반발 |

---

## Facet별 담당 감정 한눈에 보기

```
O 차원 (Openness to Experience)
├── Aesthetic Appreciation (미적 감상)
│   └── Object 브랜치: Love/Hate 증폭 (±최대 30%)
│       avg.o에 1/4 기여 (단, avg.o는 감정 계산에 미사용)
│
├── Inquisitiveness (탐구심)
│   └── 감정 계산에 관여하지 않음
│
├── Creativity (창의성)
│   └── 감정 계산에 관여하지 않음
│
└── Unconventionality (비관습성)
    └── 감정 계산에 관여하지 않음

aesthetic_appreciation (단독 수치, 절댓값 사용)
  높든 낮든 → Object: Love/Hate 증폭 (aesthetic_amp)
```

---

## 이론적 근거 분석

각 설계 결정에 대한 학술적 근거 유무를 정리한다.

### ✅ 근거 있음

**aesthetic_appreciation이 Love/Hate 담당**

OCC 이론에서 Love/Hate는 대상의 매력/혐오에 반응하는 감정으로 정의된다
(Ortony, Clore, Collins, 1988). HEXACO에서 aesthetic_appreciation은 "예술적·자연적·감각적
아름다움에 강하게 반응하는 성질"을 측정하며, Love/Hate의 원동력과 직접 대응된다.

**aesthetic_appreciation.abs() 사용**

미적 감상이 극단적으로 높은 NPC는 아름다운 것에 깊이 반하고, 극단적으로 낮은 NPC는
감각 자체에 강한 반응 편차를 가질 수 있다. 양방향 극단 모두 대상에 대한 강한 감정 반응으로 이어진다.

참고문헌:
- Ortony, A., Clore, G.L., Collins, A. (1988). *The Cognitive Structure of Emotions*. Cambridge University Press.
- Ashton, M. C., & Lee, K. (2007). Empirical, theoretical, and practical advantages of the HEXACO model. *Personality and Social Psychology Review*, 11(2), 150-166.

### ❌ 엔지니어링 설계값 (학술 근거 없음)

| 값 | 설명 | 튜닝 방향 |
|----|------|---------|
| **0.3 계수** (aesthetic_amp) | aesthetic_appreciation이 Love/Hate에 미치는 비율 | 높이면 O의 영향력 증가 |

### ℹ️ 미구현 (현재 연결 없음)

| Facet | 이유 |
|-------|------|
| **Inquisitiveness** | 대응되는 OCC 감정 타입 없음 |
| **Creativity** | 대응되는 OCC 감정 타입 없음 |
| **Unconventionality** | 대응되는 OCC 감정 타입 없음 |

---

## O 차원과 EmotionalMomentum

### EmotionalMomentum 현재 구성

`EmotionalMomentum`은 **이전 감정 상태가 새 평가에 미치는 심리적 관성**을 수치화한 구조체다.
`appraise_with_context()`를 통해 대화 맥락을 유지할 때만 활성화된다.

### O 관련 감정에서 Momentum이 개입하는 경로

O가 관여하는 감정들 중 `EmotionalMomentum`이 개입하는 경로는 **없다.**

```
// appraise_object 내부
fn appraise_object(personality, situation, _m: &EmotionalMomentum) -> EmotionState
```

`appraise_object`는 `EmotionalMomentum`을 파라미터로 전달받지만 실제로 사용하지 않는다
(`_m`으로 언더스코어 처리).

| O 관련 감정 | Momentum 개입 | 개입 필드 |
|------------|-------------|---------|
| Love | 없음 | — |
| Hate | 없음 | — |

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|---------|
| 0.1.0 | 2026-03-24 | 초기 작성. O 4 Facet 상세, aesthetic_appreciation 단독 사용, 핵심 변수 해설, OCC 브랜치별 감정 공식, 이론적 근거 분석, EmotionalMomentum 관계 정리 |
