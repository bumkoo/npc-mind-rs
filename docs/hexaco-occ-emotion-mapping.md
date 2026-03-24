# HEXACO × OCC 22감정 매핑 보고서

## 개요

OCC 22개 감정은 HEXACO 6차원이 **가중치 변수**로 작용하여 강도와 발생 여부가 결정된다.
같은 상황에서도 NPC마다 다른 감정이 생성되는 이유는 이 매핑 때문이다.

이 문서는:
1. **22개 감정 × 6차원 매핑 전체 표** — 빠른 참조
2. **OCC 분기별 HEXACO 영향 상세** — 공식·역할·무협 예시
3. **차원별 전용 가이드 링크** — 각 차원의 Facet 상세

관련 문서:
- [OCC 감정 모델 & 파이프라인](occ-emotion-model.md)
- 차원별 가이드: [H](h-dimension-guide.md) · [E](e-dimension-guide.md) · [X](x-dimension-guide.md) · [A](a-dimension-guide.md) · [C](c-dimension-guide.md) · [O](o-dimension-guide.md)

---

## 22감정 × HEXACO 매핑 전체 표

`●` = 직접 영향, `○` = 간접/보조 영향, `—` = 해당 없음

| OCC 분기 | 감정 (EN) | H | E | X | A | C | O |
|----------|-----------|---|---|---|---|---|---|
| Well-being | Joy | — | ●증폭 | ●증폭 | — | — | — |
| Well-being | Distress | — | ●증폭 | — | — | ○억제(prudence) | — |
| Fortune-of-others | HappyFor | ●발동조건 | ○증폭 | — | ●공감계수 | — | — |
| Fortune-of-others | Pity | ○ | ●(sentimentality) | — | ●공감계수 | — | — |
| Fortune-of-others | Gloating | ●발동조건(H↓) | — | — | ●잔인함계수(A↓) | — | — |
| Fortune-of-others | Resentment | ●발동조건(H↓) | — | — | ○(forgiveness↑억제) | — | — |
| Prospect | Hope | — | ○증폭 | ●증폭 | — | — | — |
| Prospect | Fear | — | ●(fearfulness) | — | — | — | — |
| Prospect | Satisfaction | — | ○증폭 | ●증폭 | — | — | — |
| Prospect | Disappointment | — | ●증폭 | — | — | — | — |
| Prospect | Relief | — | ○증폭 | ●증폭 | — | — | — |
| Prospect | FearsConfirmed | — | ●증폭 | — | — | — | — |
| Attribution | Pride | ●억제(modesty) | — | — | — | ●증폭(standards) | — |
| Attribution | Shame | — | — | — | — | ●증폭(standards) | — |
| Attribution | Admiration | — | — | — | — | ●증폭(standards) | — |
| Attribution | Reproach | — | — | — | ●억제(gentleness) | ●증폭(standards) | — |
| Compound | Gratitude | ●증폭(sincerity) | ○증폭 | — | — | — | — |
| Compound | Anger | ○(cruelty) | — | — | ●브레이크(patience) | ○억제(prudence) | — |
| Compound | Gratification | — | ○증폭 | ●증폭 | — | ●증폭(standards) | — |
| Compound | Remorse | — | ○증폭 | — | — | ●증폭(standards) | — |
| Object | Love | — | — | — | — | — | ●증폭(aesthetic) |
| Object | Hate | — | — | — | — | — | ●증폭(aesthetic) |

---

## OCC 분기별 HEXACO 영향 상세

### Event-based: Well-being

**Joy / Distress** — 자신에게 직접 일어난 사건의 결과

| 차원 | 역할 | 수식 |
|------|------|------|
| E 전체 | 전반적 감정 증폭(볼륨 노브) | `emotional_amp = 1.0 + |E| × 0.3` |
| X 전체 | Joy 등 긍정 감정 증폭 | `positive_amp = 1.0 + X × 0.3` (X > 0일 때) |
| C.prudence | Distress 즉각 반응 억제 | `impulse_mod = 1.0 - prudence × 0.3` |

무협 예시: "해독약 구하기 실패"
- 무백 (E↓): Disappointment·Distress 존재하나 `emotional_amp` 낮음 → 담담
- 수련 (E.sentimentality↑): 감정 증폭 → 깊고 억눌린 고통

---

### Event-based: Fortune-of-others

타인에게 좋은·나쁜 일이 생겼을 때 자신이 어떻게 반응하는가를 결정한다.
**H(정직-겸손성)** 가 이 분기의 발동 방향을 결정하는 핵심 변수다.

#### 발동 조건

```
H ≥ 0  →  HappyFor (타인의 행운에 기쁨), Pity (타인의 불행에 동정)
H < -0.2 → Resentment (타인의 행운에 시기)
H < 0 AND A < 0 → Gloating (타인의 불행에 고소함)
```

#### 강도 공식

| 감정 | 계수 | 수식 |
|------|------|------|
| HappyFor | 공감력 | `empathy = (H + A) / 2` |
| Pity | 공감력 + sentimentality | `empathy` + E.sentimentality 가산 |
| Resentment | — | H < -0.2 조건 충족 시 발동, `forgiveness`↑이면 억제 |
| Gloating | 잔인함 | `cruelty = (|H| + |A|) / 2` (H < 0, A < 0일 때) |

무협 예시: "라이벌이 무림맹주에 추대됨"
- 무백 (H=+0.65, A↑): `empathy` 높음 → HappyFor, Resentment 미발생
- 교룡 (H=-0.55, A↓): H < -0.2 & A < 0 → Resentment 발생, Gloating 가능

---

### Event-based: Prospect

미래 사건의 **가능성 예측**에서 발생하는 감정.

| 감정 | HEXACO 영향 |
|------|-------------|
| Hope | X↑ → `positive_amp`로 Hope 강도 증폭 |
| Fear | E.fearfulness↑ → `fear_amp = 1.0 + fearfulness × 0.5` 직접 증폭 |
| Satisfaction | X↑ → `positive_amp` 적용 |
| Disappointment | E↑ → `emotional_amp` 적용 |
| Relief | X↑ → `positive_amp` 적용 |
| FearsConfirmed | E↑ → `emotional_amp` 적용 |

무협 예시: "적의 대군이 다가옴"
- 무백 (E.fearfulness=-0.6): `fear_amp` 증폭 없음 → Fear 존재하나 약함
- 소호 (E.fearfulness=-0.7): 극도로 대담 → Fear 최소화

---

### Action-based: Attribution

행위자의 행동을 **칭찬/비난**하는 감정 분기.
**C(성실성)** 의 높은 자기 기준이 이 분기의 강도를 결정한다.

| 감정 | 차원 | 역할 | 수식 |
|------|------|------|------|
| Pride | H.modesty | 겸손하면 자부심 억제 | `pride_mod = 1.0 - modesty × 0.3` |
| Pride | C 전체 | 높은 기준 → Pride 증폭 | `standards_amp = 1.0 + |C| × 0.3` |
| Shame | C 전체 | 높은 기준 → 기준 위반 시 수치심 증폭 | `standards_amp` |
| Admiration | C 전체 | 높은 기준 → 타인 행동 평가 증폭 | `standards_amp` |
| Reproach | A.gentleness | 온화하면 비난 억제 | `reproach_amp = 1.0 - gentleness × 0.3` |
| Reproach | C 전체 | 높은 기준 → 비난 강도 증폭 | `standards_amp` |

C의 이중 역할:
1. **prudence** → 즉각 감정 반응 억제 (감정 표현을 줄임)
2. **높은 자기 기준** → Pride/Shame 강도 증폭 (기준에 더 민감)

무협 예시:
- 수련 (C=+0.70, prudence=+0.9): 자기 행동 실수 시 강한 Shame, 표현은 억제
- 소호 (C=-0.33, prudence=-0.5): 낮은 기준, 충동적 반응

---

### Compound: Well-being + Attribution

Event 결과와 Action 평가가 **결합**된 복합 감정.

#### Gratitude (감사)

타인의 좋은 행동 + 나에게 좋은 결과 → Admiration + Joy

| 차원 | 역할 | 수식 |
|------|------|------|
| H.sincerity | 진실한 성격 → 감사 증폭 | `gratitude_amp = 1.0 + sincerity × 0.3` |
| E | 전반적 감정 증폭 | `emotional_amp` |

#### Anger (분노)

타인의 나쁜 행동 + 나에게 나쁜 결과 → Reproach + Distress

| 차원 | 역할 | 수식 |
|------|------|------|
| A 전체 | 분노 브레이크 | `anger_mod = 1.0 - A × 0.4` |
| A.patience | patience↑ → Anger 억제 | `anger_amp = 1.0 - patience × 0.4` |
| A.patience | patience↓ → Anger 증폭 | patience < 0이면 anger 증폭 |
| C.prudence | 즉각 반응 억제 | `impulse_mod` |

무협 예시: "동료의 배신"
- 무백 (patience=+0.8): Anger 발생하나 강도 억제 → 절제된 분노
- 교룡 (patience=-0.7): `anger_amp` 증폭 → 폭발적 분노
- 수련 (patience=+0.9, prudence=+0.9): Anger 극도 억제 + 즉각 반응 없음 → 계획적 대응

#### Gratification (만족감)

내 좋은 행동 + 좋은 결과 → Pride + Joy

| 차원 | 역할 |
|------|------|
| X | 긍정 감정 증폭 |
| C | standards_amp로 Pride 부분 증폭 |

#### Remorse (후회)

내 나쁜 행동 + 나쁜 결과 → Shame + Distress

| 차원 | 역할 |
|------|------|
| C | standards_amp로 Shame 부분 증폭 |
| E | emotional_amp로 Distress 부분 증폭 |

---

### Object-based

대상(사물, 사람, 개념)의 **속성**에 대한 반응.

| 감정 | 차원 | 역할 | 수식 |
|------|------|------|------|
| Love | O.aesthetic_appreciation | 미적 감수성 → 반응 증폭 | `aesthetic_amp = 1.0 + |aesthetic| × 0.3` |
| Hate | O.aesthetic_appreciation | 미적 감수성 → 혐오 증폭 | `aesthetic_amp` |

O↑ NPC: 아름다운 것에 더 깊이 감동, 추한 것에 더 강하게 혐오.
O↓ NPC: 미적·정서적 자극에 둔감, Love/Hate 모두 약함.

---

## HEXACO 차원별 영향 요약

| 차원 | 역할 비유 | 핵심 공식 변수 | 주요 감정 |
|------|-----------|---------------|---------|
| **H** 정직-겸손 | 도덕 필터 | empathy, cruelty, pride_mod, gratitude_amp | HappyFor, Resentment, Gloating, Pride, Gratitude |
| **E** 정서성 | 볼륨 노브 | emotional_amp, fear_amp | 전체 감정 증폭, Fear |
| **X** 외향성 | 긍정 증폭기 | positive_amp | Joy, Hope, Satisfaction, Relief, Gratification |
| **A** 원만성 | 분노 브레이크 | anger_mod, anger_amp, reproach_amp | Anger, Reproach, (Resentment/Gloating 간접) |
| **C** 성실성 | 충동 억제 + 자기 기준 | impulse_mod, standards_amp | Distress(억제), Pride, Shame, Admiration, Reproach |
| **O** 개방성 | 미적 감수성 | aesthetic_amp | Love, Hate |

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|-----------|
| 0.1.0 | 2026-03-24 | occ-emotion-model.md에서 HEXACO 6차원 상세 섹션 분리 독립. 22감정 × HEXACO 매핑 전체 표 추가 |
