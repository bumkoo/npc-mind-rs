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

각 차원의 Facet 상세, 핵심 변수 공식, OCC 브랜치별 감정 연결, 이론적 근거는
차원별 전용 가이드를 참조한다.

| 차원 | 역할 | 주요 감정 | 상세 가이드 |
|------|------|---------|-----------|
| H 정직-겸손성 | Fortune-of-others 분기 핵심 | HappyFor, Resentment, Gloating, Pride, Gratitude | [H 차원 가이드](h-dimension-guide.md) |
| E 정서성 | 감정 볼륨 노브 | 전체 감정 증폭, Fear 직접 | [E 차원 가이드](e-dimension-guide.md) |
| X 외향성 | 긍정 감정 증폭기 | Joy, Hope, Satisfaction, Relief | [X 차원 가이드](x-dimension-guide.md) |
| A 원만성 | 분노 브레이크 | Anger, Reproach, Resentment, Gloating | [A 차원 가이드](a-dimension-guide.md) |
| C 성실성 | 충동 억제 + 자기 기준 | Distress(억제), Pride/Shame/Admiration/Reproach | [C 차원 가이드](c-dimension-guide.md) |
| O 개방성 | 미적 감수성 | Love, Hate | [O 차원 가이드](o-dimension-guide.md) |

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

## 대화 중 감정 변화 (Emotional Momentum)

### 문제

1사이클 구현은 1회성이었다: 상황 하나 → 감정 하나 → 끝.
하지만 실제 NPC 대화는 여러 턴에 걸쳐 감정이 누적된다.

```
무백: "그 검을 돌려주시오"    → 교룡: (짜증, Distress 0.3)
무백: "그건 내 사부의 유품이오" → 교룡: (죄책감 추가, Shame 0.2 + Distress 0.3)
무백: "도둑질이라 부를 수밖에"  → 교룡: (분노 폭발, Anger 0.8 — 기존 짜증이 증폭)
```

### 해결: appraise_with_context

기존 `appraise(personality, situation)`에 추가로
`appraise_with_context(personality, situation, current_state)`를 제공한다.

```
appraise()              → 1회성 평가 (내부에서 빈 상태로 호출)
appraise_with_context() → 현재 감정 위에 새 감정을 누적
```

### EmotionalMomentum (감정 관성)

현재 EmotionState에서 4가지 영향 계수를 산출한다:

| 계수 | 역할 | 산출 방법 | 범위 |
|------|------|-----------|------|
| negative_bias | 기존 부정감정 → 새 부정감정 증폭 | overall_valence 음수 부분 × 0.5 | 0.0~0.5 |
| positive_bias | 기존 긍정감정 → 새 긍정감정 증폭 | overall_valence 양수 부분 × 0.3 | 0.0~0.3 |
| anger_erosion | 기존 Anger → patience 브레이크 약화 | anger_intensity × 0.5 | 0.0~0.5 |
| sensitivity_boost | 기존 Fear/Distress → 감정 민감도 상승 | (fear + distress) / 2 × 0.3 | 0.0~0.3 |

### 각 계수가 작용하는 지점

```
emotional_amp  += sensitivity_boost    (E의 볼륨 노브에 가산)
positive_amp   += positive_bias        (X의 긍정 증폭기에 가산)
anger_mod      += anger_erosion        (A의 브레이크에 마모 추가)
anger_mod      += negative_bias        (전반적 부정 바이어스)
reproach_amp   += negative_bias        (비난 감정에도 부정 바이어스)
action anger   += anger_erosion        (Action 분기 Anger에도 마모)
action anger   += negative_bias        (Action 분기에도 부정 바이어스)
```

### HEXACO와의 상호작용

momentum은 성격의 효과를 **감쇠시키거나 증폭**시킨다:

- 무백(patience=+0.8): patience 브레이크가 강하지만, 이미 Anger가 높으면
  anger_erosion이 브레이크를 약화 → 그래도 교룡보다는 절제됨
- 교룡(patience=-0.7): 원래 브레이크가 없는데 anger_erosion까지 가산
  → 대화가 길어질수록 분노가 기하급수적으로 폭발

### 시나리오: 교룡 3턴 대화

| 턴 | 상황 | 새 감정 | 누적 상태 | momentum 효과 |
|----|------|---------|-----------|---------------|
| 1 | "검을 돌려주시오" | Distress 0.3 | Distress 0.3 | 없음 (첫 턴) |
| 2 | "사부의 유품이오" | Shame 0.2 | Distress 0.3 + Shame 0.2 | negative_bias 발생 |
| 3 | "도둑질이라 부를 수밖에" | Anger ↑↑ | Distress + Shame + Anger 폭발 | negative_bias + sensitivity_boost → 분노 증폭 |

**핵심 검증**: 턴3의 Anger는 맥락 없이(appraise) 평가한 것보다
맥락 있게(appraise_with_context) 평가한 것이 더 강하다.

### 시나리오: 무백 vs 교룡 2턴 누적

같은 부정적 대화 2턴 후:
- 무백: patience↑이 anger_erosion을 흡수 → 여전히 절제됨
- 교룡: patience↓에 anger_erosion 추가 → 폭발적 증폭

### 시나리오: 긍정 감정 누적

좋은 소식이 연속될 때:
- 턴1 Joy → positive_bias 발생 → 턴2 Joy가 단독보다 더 강해짐
- X↑인 NPC는 이 증폭이 더 큼

### 설계 판단

- **장기기억/시간 도메인 없이** 현재 EmotionState만으로 구현
- 기존 `appraise()`와 하위 호환 유지 (내부에서 빈 상태로 호출)
- 감정 감쇠(decay)와 감정 기억(memory)은 별도 도메인(시간, 장기기억)이 필요하므로 추후 검토

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
  - AppraisalEngine::appraise_with_context: 대화 중 감정 누적 지원
  - EmotionalMomentum: 현재 감정이 새 평가에 미치는 4가지 영향 계수

- 테스트 14개 통과:
  - 배신 시나리오 (무백/교룡/수련 비교)
  - 적 대군 시나리오 (Fear 강도)
  - 라이벌 승진 시나리오 (HappyFor vs Resentment)
  - 해독약 실패 시나리오 (Disappointment)
  - EmotionState 기능 (dominant, valence, significant)
  - 대화 중 감정 변화: 교룡 3턴 누적 (맥락 있을 때 분노가 더 강함)
  - 대화 중 감정 변화: 무백 vs 교룡 2턴 누적 (성격에 따른 증폭 차이)
  - 대화 중 감정 변화: 긍정 감정 연속 누적

### 향후 확장 예정

- 3사이클: EmotionState → LLM 프롬프트 가이드 생성
- 4사이클: fastembed(bge-m3) 기반 상황 텍스트 → Situation 자동 변환
- unexpectedness, familiarity 등 추가 강도 변수
- 감정 감쇠(decay) 시스템: 시간이 지나면 감정 강도가 줄어듦 → 별도 도메인(시간) 필요, 추후 검토
- 감정 기억: 과거 감정 이력이 현재 appraisal에 영향 → 별도 도메인(장기기억) 필요, 추후 검토

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|-----------|
| 0.1.0 | 2026-03-23 | 초기 작성. OCC 22개 감정 분류, HEXACO→OCC 매핑 6차원 전체 정리 |
| 0.1.1 | 2026-03-23 | 4인 캐릭터 시나리오별 감정 비교, 구현 상태, 향후 확장 정리 |
| 0.2.0 | 2026-03-23 | 대화 중 감정 변화(EmotionalMomentum) 추가. appraise_with_context, 4가지 영향 계수, 3개 시나리오 테스트 |
