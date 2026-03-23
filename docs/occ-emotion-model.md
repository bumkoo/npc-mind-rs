# OCC 감정 모델 & HEXACO→OCC 매핑 보고서

## 개요

OCC 모델은 Ortony, Clore, Collins가 1988년에 제안한 인지적 감정 구조 이론이다.
감정을 "상황에 대한 평가(appraisal)의 결과"로 보며, 22개 감정 유형을 체계적으로 분류한다.

NPC 심리 엔진에서 OCC는 HEXACO 성격 모델과 결합하여 작동한다:
성격(HEXACO)이 평가의 가중치가 되어, 같은 상황에서도 NPC마다 다른 감정을 생성한다.

### 참고문헌

- Ortony, A., Clore, G.L., Collins, A. (1988). *The Cognitive Structure of Emotions*. Cambridge University Press.
- Ortony, A., Clore, G.L., Collins, A. (2022). *The Cognitive Structure of Emotions* (2nd ed.). Cambridge University Press.
- Steunebrink, B.R. et al. (2009). "The OCC Model Revisited." *KI 2009*.
- Bartneck, C. (2002). "Integrating the OCC Model of Emotions in Embodied Characters."

---

## OCC 핵심 원리

### 감정의 정의

OCC에서 감정은 **상황에 대한 평가된 반응(valenced reaction)**이다.
같은 객관적 사건이라도 개인이 그 상황을 어떻게 해석(appraise)하느냐에 따라
다른 감정, 다른 강도가 생성된다.

### 3대 분기 (Branch)

세상을 바라보는 세 가지 초점에 따라 감정이 분기된다:

| 분기 | 초점 | 핵심 평가 기준 | 기본 반응 |
|------|------|----------------|-----------|
| **Event** | 사건의 결과 | 바람직함(desirability) | pleased / displeased |
| **Action** | 행위자의 행동 | 칭찬받을만함(praiseworthiness) | approving / disapproving |
| **Object** | 대상의 속성 | 매력(appealingness) | liking / disliking |

---

## 22개 감정 유형 전체 분류

### Event-based (사건의 결과)

#### Well-being (자기 복지)

| 감정 | 영문 | Valence | 조건 |
|------|------|---------|------|
| 기쁨 | Joy | + | 자신에게 바람직한 사건 발생 |
| 고통 | Distress | - | 자신에게 바람직하지 않은 사건 발생 |

#### Fortune-of-others (타인의 운)

| 감정 | 영문 | Valence | 조건 |
|------|------|---------|------|
| 대리기쁨 | HappyFor | + | 타인에게 좋은 일 + 나도 기쁨 |
| 동정 | Pity | - | 타인에게 나쁜 일 + 나도 안타까움 |
| 고소함 | Gloating | +/- | 타인에게 나쁜 일 + 내가 기쁨 |
| 시기/원망 | Resentment | - | 타인에게 좋은 일 + 내가 불쾌 |

#### Prospect-based (전망)

| 감정 | 영문 | Valence | 조건 |
|------|------|---------|------|
| 희망 | Hope | + | 바람직한 사건이 일어날 가능성 |
| 두려움 | Fear | - | 바람직하지 않은 사건이 일어날 가능성 |
| 만족 | Satisfaction | + | 바랐던 일이 실현됨 (Hope → confirmed) |
| 실망 | Disappointment | - | 바랐던 일이 실현되지 않음 (Hope → disconfirmed) |
| 안도 | Relief | + | 두려워했던 일이 일어나지 않음 (Fear → disconfirmed) |
| 공포확인 | FearsConfirmed | - | 두려워했던 일이 실현됨 (Fear → confirmed) |

### Action-based (행위자의 행동)

#### Attribution (귀인)

| 감정 | 영문 | Valence | 조건 |
|------|------|---------|------|
| 자부심 | Pride | + | 자기 행동을 긍정 평가 |
| 수치심 | Shame | - | 자기 행동을 부정 평가 |
| 감탄 | Admiration | + | 타인 행동을 긍정 평가 |
| 비난 | Reproach | - | 타인 행동을 부정 평가 |

#### Compound: Well-being + Attribution (복합 감정)

Event(사건 결과) + Action(행동 평가)이 결합된 감정:

| 감정 | 영문 | Valence | 조건 |
|------|------|---------|------|
| 감사 | Gratitude | + | 타인의 좋은 행동 + 나에게 좋은 결과 (Admiration + Joy) |
| 분노 | Anger | - | 타인의 나쁜 행동 + 나에게 나쁜 결과 (Reproach + Distress) |
| 만족감 | Gratification | + | 내 좋은 행동 + 좋은 결과 (Pride + Joy) |
| 후회 | Remorse | - | 내 나쁜 행동 + 나쁜 결과 (Shame + Distress) |

### Object-based (대상의 속성)

| 감정 | 영문 | Valence | 조건 |
|------|------|---------|------|
| 좋아함 | Love | + | 매력적인 대상 |
| 싫어함 | Hate | - | 비매력적인 대상 |

---

## 감정 강도 변수 (Intensity Variables)

OCC 모델에서 감정은 발생 여부뿐 아니라 **강도(intensity)**가 핵심이다.
같은 감정이라도 강도에 따라 NPC의 행동이 달라진다.

OCC가 제시한 강도 영향 변수:

| 변수 | 설명 | NPC 엔진에서의 대응 |
|------|------|---------------------|
| desirability | 사건이 얼마나 바람직한가 | Situation 입력값 |
| praiseworthiness | 행동이 얼마나 칭찬/비난받을만한가 | Situation 입력값 |
| appealingness | 대상이 얼마나 매력적인가 | Situation 입력값 |
| likelihood | 전망 사건의 가능성 | Situation.is_prospective |
| unexpectedness | 예상치 못한 정도 | 향후 확장 예정 |
| **sense of reality** | 현실감/몰입도 | **HEXACO 성격이 이 역할 수행** |

핵심 통찰: HEXACO 성격이 OCC의 "sense of reality"와 유사한 역할을 한다.
성격은 상황을 얼마나 심각하게/가볍게 받아들이는지를 결정하는 개인차 변수다.

---

## HEXACO → OCC 매핑 (AppraisalEngine 설계)

### 파이프라인

```
Situation(상황)
    ↓
    ├─ Event / Action / Object 분기 판별
    ↓
AppraisalEngine.appraise(personality, situation)
    ↓
    ├─ HEXACO 성격으로 감정 강도 가중치 계산
    ├─ OCC 규칙에 따라 감정 유형 결정
    ├─ 가중치 적용하여 감정 강도 산출
    ↓
EmotionState (감정 유형 + 강도의 조합)
```

### HEXACO 6차원별 감정 영향

각 HEXACO 차원이 OCC 감정의 어떤 측면에 영향을 미치는지 정리:

#### H: 정직-겸손성 → Fortune-of-others 분기 핵심

| H 점수 | 감정 영향 | 수식 |
|--------|-----------|------|
| H↑ (양수) | HappyFor 증폭 — 타인의 행운에 진심으로 기뻐함 | empathy = (H + A) / 2 |
| H↓ (음수) | Resentment 발생 — 타인의 행운에 질투/시기 | H < -0.2 이면 발동 |
| H↓ + A↓ | Gloating 발생 — 타인의 불행에 고소함 | cruelty = (\|H\| + \|A\|) / 2 |
| modesty↑ | Pride 억제 — 겸손하면 자부심이 줄어듦 | pride_mod = 1.0 - modesty × 0.3 |
| sincerity↑ | Gratitude 증폭 — 진실한 성격은 감사를 더 강하게 느낌 | gratitude_amp = 1.0 + sincerity × 0.3 |

무협 예시:
- 무백(H=+0.65): 라이벌 승진에 HappyFor, Resentment 없음
- 교룡(H=-0.55): 라이벌 승진에 Resentment 발생, HappyFor 없음

#### E: 정서성 → 전반적 감정 반응 증폭

| E 요소 | 감정 영향 | 수식 |
|--------|-----------|------|
| E 전체 | 감정 반응의 전반적 증폭/억제 | emotional_amp = 1.0 + \|E\| × 0.3 |
| fearfulness↑ | Fear 직접 증폭 | fear_amp = 1.0 + fearfulness × 0.5 |
| fearfulness↓ | Fear 증폭 미발생 (대담) | fearfulness < 0 이면 증폭 없음 |
| sentimentality↑ | Pity 증폭 — 감상적이면 동정심 강함 | compassion에 가산 |

E는 **감정의 볼륨 노브** 역할이다.
E↑인 NPC는 모든 감정을 더 강하게 느끼고, E↓인 NPC는 담담하게 반응한다.

무협 예시:
- 소호(E=-0.40): 대담하여 두려움 증폭이 적고, 전반적으로 담담
- 수련(E=+0.03): 복합적 — sentimentality↑이지만 fearfulness↓

#### X: 외향성 → 긍정 감정 증폭

| X 점수 | 감정 영향 | 수식 |
|--------|-----------|------|
| X↑ (양수) | Joy, Hope, Satisfaction 등 긍정 감정 증폭 | positive_amp = 1.0 + X × 0.3 (X>0일 때) |
| X↓ (음수) | 긍정 감정 증폭 없음 (내성적) | X<0이면 positive_amp = 1.0 |

X는 **긍정 감정의 증폭기**다. 부정 감정에는 직접 영향을 주지 않는다.

#### A: 원만성 → 부정 감정 조절의 핵심

| A 요소 | 감정 영향 | 수식 |
|--------|-----------|------|
| A 전체 | 부정 감정 완화/증폭 | anger_mod = 1.0 - A × 0.4 |
| patience↑ | Anger 직접 억제 | anger_amp = 1.0 - patience × 0.4 |
| patience↓ | Anger 직접 증폭 | patience < 0 이면 anger 증폭 |
| gentleness↑ | Reproach 억제 | reproach_amp = 1.0 - gentleness × 0.3 |
| forgiveness↑ | Resentment 억제 | Fortune-of-others 분기에서 작용 |

A는 **분노/공격성의 브레이크** 역할이다.
A↑이면 화가 나도 참고, A↓이면 즉각 폭발한다.

무협 예시: "동료의 배신" 상황
- 무백(patience=+0.8): Anger 발생하지만 강도 억제 → 절제된 분노
- 교룡(patience=-0.7): Anger 강도 증폭 → 폭발적 분노
- 수련(patience=+0.9): Anger 극도로 억제 → 억눌린 고통

#### C: 성실성 → 충동 억제 + 자기 기준

| C 요소 | 감정 영향 | 수식 |
|--------|-----------|------|
| prudence↑ | 즉각 감정 반응 억제 | impulse_mod = 1.0 - prudence × 0.3 |
| C 전체 | Pride/Shame 증폭 (높은 자기 기준) | standards_amp = 1.0 + \|C\| × 0.3 |
| C↑ | Shame 증폭 — 자기 기준 위반 시 더 강한 수치심 | standards_amp으로 증폭 |
| C↓ | 충동적 반응 — prudence 억제 없이 즉각 행동 | impulse_mod ≈ 1.0 |

C는 **이중 역할**을 한다:
1. prudence가 Distress 등 즉각 반응을 억제 (감정 표현 자체를 줄임)
2. 높은 자기 기준이 Pride/Shame 강도를 증폭 (기준 충족/위반에 더 민감)

무협 예시:
- 수련(C=+0.70, prudence=+0.9): 감정을 억누르되, 기준 위반 시 강한 수치심
- 소호(C=-0.33, prudence=-0.5): 충동적으로 반응, 자기 기준에 무심

#### O: 개방성 → 대상 반응(Love/Hate) 증폭

| O 요소 | 감정 영향 | 수식 |
|--------|-----------|------|
| aesthetic_appreciation↑ | Love/Hate 증폭 — 미적 감수성 | aesthetic_amp = 1.0 + \|aesthetic\| × 0.3 |
| O↑ | 대상에 대한 반응이 더 강렬 | 아름다운 것에 더 감동, 추한 것에 더 혐오 |
| O↓ | 대상에 대한 반응이 무덤덤 | 미적/정서적 자극에 둔감 |

O는 현재 Object-based 분기에만 작용하지만, 향후 창의적 문제 해결이나
비관습적 감정 반응에도 확장 가능하다.

---

## HEXACO→OCC 매핑 요약표

| HEXACO | 역할 비유 | 영향 대상 감정 | 방향 |
|--------|-----------|----------------|------|
| H 정직-겸손 | 도덕 필터 | HappyFor, Resentment, Gloating, Pride, Gratitude | H↑=공감, H↓=질투/고소 |
| E 정서성 | 볼륨 노브 | 전체 감정, Fear 직접 | E↑=증폭, E↓=담담 |
| X 외향성 | 긍정 증폭기 | Joy, Hope, Satisfaction, Relief | X↑=긍정 강화 |
| A 원만성 | 분노 브레이크 | Anger, Reproach, Pity | A↑=억제, A↓=폭발 |
| C 성실성 | 충동 억제 + 자기 기준 | Distress 억제, Pride/Shame 증폭 | C↑=절제+높은 기준 |
| O 개방성 | 미적 감수성 | Love, Hate | O↑=반응 증폭 |

---

## 시나리오별 4인 캐릭터 감정 비교

### "동료의 배신" (Action: 타인의 비난받을 행동 + 나에게 나쁜 결과)

| 캐릭터 | 핵심 성격 | 주요 감정 | 강도 | 행동 예측 |
|--------|-----------|-----------|------|-----------|
| 무백 | A↑ patience=0.8 | Anger (억제됨) + Reproach | 낮음 | 차분히 대응, 해결책 모색 |
| 교룡 | A↓ patience=-0.7 | Anger (폭발) + Reproach (강함) | 높음 | 즉각 복수, 공격적 반응 |
| 수련 | A↑ prudence=0.9 | Anger (극도 억제) + Reproach | 매우 낮음 | 감정 숨기고 계획 세움 |
| 소호 | C↓ prudence=-0.5 | Anger (중간) + Reproach | 중간 | 계획 없이 즉각 행동 |

### "라이벌이 무림맹주에 추대됨" (Event: 타인에게 좋은 일)

| 캐릭터 | 핵심 성격 | 주요 감정 | 이유 |
|--------|-----------|-----------|------|
| 무백 | H↑ A↑ | HappyFor | 높은 공감력, Resentment 미발생 |
| 교룡 | H↓ A↓ | Resentment | 교활하고 탐욕적, 타인의 행운에 시기 |

### "적의 대군이 다가옴" (Event: 전망, 부정)

| 캐릭터 | 핵심 성격 | Fear 강도 | 이유 |
|--------|-----------|-----------|------|
| 무백 | E↓ fearfulness=-0.6 | 존재하나 약함 | 대담, emotional_amp 낮음 |
| 소호 | E↓ fearfulness=-0.7 | 약함 | 극도로 대담, fear 증폭 미발생 |

### "해독약 구하기 실패" (Event: 희망 미실현)

| 캐릭터 | 핵심 성격 | Disappointment 강도 | 이유 |
|--------|-----------|---------------------|------|
| 무백 | E↓ | 깊지만 담담 | emotional_amp 낮음 |
| 수련 | E 복합 (sentimentality↑) | 깊고 억눌림 | emotional_amp 약간 높음 |

---

## 구현 상태

### 완료 (2사이클)

- `src/domain/emotion.rs`
  - EmotionType: OCC 22개 감정 전체 enum
  - EmotionBranch: Event / Action / Object 분기
  - Emotion: 감정 유형 + 강도(0.0~1.0)
  - EmotionState: 감정 조합 관리 (add, dominant, significant, overall_valence)
  - Situation / SituationFocus: 상황 입력 모델
  - PriorExpectation: 전망 확인 감정용 (Satisfaction, Disappointment, Relief, FearsConfirmed)
  - AppraisalEngine: HEXACO × OCC → 감정 생성 핵심 엔진

- 테스트 11개 통과:
  - 배신 시나리오 (무백/교룡/수련 비교)
  - 적 대군 시나리오 (Fear 강도)
  - 라이벌 승진 시나리오 (HappyFor vs Resentment)
  - 해독약 실패 시나리오 (Disappointment)
  - EmotionState 기능 (dominant, valence, significant)

### 향후 확장 예정

- 3사이클: EmotionState → LLM 프롬프트 가이드 생성
- 4사이클: fastembed(bge-m3) 기반 상황 텍스트 → Situation 자동 변환
- unexpectedness, familiarity 등 추가 강도 변수
- 감정 감쇠(decay) 시스템: 시간이 지나면 감정 강도가 줄어듦
- 감정 기억: 과거 감정 이력이 현재 appraisal에 영향

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|-----------|
| 0.1.0 | 2026-03-23 | 초기 작성. OCC 22개 감정 분류, HEXACO→OCC 매핑 6차원 전체 정리 |
| 0.1.1 | 2026-03-23 | 4인 캐릭터 시나리오별 감정 비교, 구현 상태, 향후 확장 정리 |
