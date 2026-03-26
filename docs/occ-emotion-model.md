# OCC 감정 모델 & HEXACO→OCC 매핑 보고서 (현행화)

## 개요

OCC 모델은 Ortony, Clore, Collins가 1988년에 제안한 인지적 감정 구조 이론이다.
감정을 "상황에 대한 평가(appraisal)의 결과"로 보며, 22개 감정 유형을 체계적으로 분류한다.

NPC 심리 엔진에서 OCC는 HEXACO 성격 모델과 결합하여 작동한다:
성격(HEXACO)이 평가의 가중치가 되어, 같은 상황에서도 NPC마다 다른 감정을 생성한다.

---

## OCC 핵심 원리

### 감정의 정의

OCC에서 감정은 **상황에 대한 평가된 반응(valenced reaction)**이다.
같은 객관적 사건이라도 개인이 그 상황을 어떻게 해석(appraise)하느냐에 따라
다른 감정, 다른 강도가 생성된다.

### 3대 분기 (Branch)

| 분기 | 초점 | 핵심 평가 기준 | 기본 반응 |
|------|------|----------------|-----------|
| **Event** | 사건의 결과 | 바람직함(desirability) | pleased / displeased |
| **Action** | 행위자의 행동 | 칭찬받을만함(praiseworthiness) | approving / disapproving |
| **Object** | 대상의 속성 | 매력(appealingness) | liking / disliking |

---

## 22개 감정 유형 및 구현 방식

### EmotionState 관리
- **고정 크기 배열**: 성능 최적화를 위해 `[f32; 22]` 배열을 사용하여 22종의 OCC 감정 강도를 관리한다.
- **Valence**: 각 감정 유형은 고유의 기본 Valence(-1.0 ~ 1.0)를 가지며, 전체 상태의 톤(`overall_valence`)을 계산하는 데 사용된다.

### 22개 감정 분류 및 특수 규칙
1. **Event-based**: Joy, Distress, HappyFor, Pity, Gloating, Resentment, Hope, Fear, Satisfaction, Disappointment, Relief, FearsConfirmed
2. **Action-based**: Pride, Shame, Admiration, Reproach, Gratification, Remorse, Gratitude, Anger
3. **Object-based**: Love, Hate

#### 감정별 특수 Valence (EmotionType::base_valence)
일반적인 감정은 1.0(긍정) 또는 -1.0(부정)의 값을 가지나, 일부 감정은 복합적인 성격을 반영하여 완화된 수치를 사용한다.
- **Gloating (고소함)**: **0.5** (타인의 불행을 기뻐하는 어두운 기쁨)
- **Resentment (시기/원망)**: **-0.5** (타인의 행운에 대한 부정적 감정)

#### 타인의 운(Fortune-of-others) 세부 규칙
- **EMPATHY_BASE (0.5)**: `HappyFor`와 `Pity`는 성격 가중치가 0이더라도 기본적으로 0.5의 강도를 기반으로 생성된다. (인간의 기본 공감능력 반영)
- **FORTUNE_THRESHOLD (-0.2)**: `Resentment`와 `Gloating`은 성격 점수(H, A)가 이 임계값보다 낮을 때만 발생한다. 즉, 단순히 부정적인 것보다 "충분히 악의적일 때" 발동한다.

#### 전망 감정(Prospect) 시퀀스
사건의 기대(`Anticipation`)와 확인(`Confirmation`)은 다음과 같이 연결되어 감정이 전이된다.
- **Hope (희망)** → 실현: **Satisfaction** (만족) / 미실현: **Disappointment** (실망)
- **Fear (두려움)** → 실현: **FearsConfirmed** (공포확인) / 미실현: **Relief** (안도)

---

## HEXACO → OCC 매핑 (AppraisalEngine)

### 가중치 공식
모든 성격 가중치는 범용 계수 **W(0.3)**를 사용하여 다음과 같은 패턴으로 적용된다:
- **증폭**: `1.0 + (Score * W)`
- **억제**: `1.0 - (max(0, Score) * W)`

### 차원별 역할
| 차원 | 주요 영향 로직 |
|------|--------------|
| **H 정직-겸손성** | `Modesty`가 높으면 자부심(`Pride`) 억제, `Sincerity`가 높으면 감사(`Gratitude`) 증폭 |
| **E 정서성** | `Emotionality` 평균이 전반적 감정 폭(`emotional_amp`) 결정, `Fearfulness`가 두려움(`Fear`) 직접 증폭 |
| **X 외향성** | `Extraversion` 평균이 긍정적 사건에 대한 기쁨(`Joy`) 및 희망(`Hope`) 증폭 |
| **A 원만성** | `Patience`가 높으면 분노(`Anger`) 억제, `Gentleness`가 높으면 비난(`Reproach`) 억제 |
| **C 성실성** | `Prudence`가 높으면 고통(`Distress`) 및 충동적 감정 억제, 성실성 평균이 도덕적 기준(`Standards`)으로 작용 |
| **O 개방성** | `Aesthetic Appreciation`이 높을수록 대상에 대한 호불호(`Love`/`Hate`) 명확화 |

---

## 대화 중 감정 변화 (Stimulus Processing)

현재 엔진은 대화 턴마다 상황을 재평가하는 대신, **대사 자극(Stimulus)**을 통해 기존 감정 강도를 동적으로 변동시킨다.

### StimulusEngine 로직
1. **PAD 매핑**: 입력된 대사 자극(PAD)과 기존 감정의 PAD 방향을 비교(내적, Dot Product)한다.
2. **방향성 적용**:
   - 자극과 일치하는 방향의 감정 → 강도 증가
   - 자극과 반대되는 방향의 감정 → 강도 감소
3. **자연 소멸(Fade)**: 강도가 **0.05(FADE_THRESHOLD)** 미만으로 떨어지면 해당 감정은 소멸된 것으로 간주하여 제거한다.
4. **IMPACT_RATE(0.1)**: 한 턴의 대사가 미치는 최대 영향력을 제한하여 감정의 급격한 널뛰기를 방지한다.

### 성격별 자극 수용도 (Absorb Rate)
- **E(정서성)**: 높을수록 모든 자극을 더 크게 받아들임.
- **A.patience(인내심)**: 높을수록 부정적 자극(Pleasure < 0)에 대한 저항력 가짐.
- **C.prudence(신중함)**: 높을수록 감정의 변동 폭 자체를 억제.

---

## 구현 상태 및 검증

### 완료된 기능
- [x] OCC 22개 감정 유형 정의 및 인덱스 기반 관리 (`types.rs`)
- [x] HEXACO 성격 × 관계(Relationship) 기반 감정 평가 엔진 (`engine.rs`)
- [x] 복합 감정(Compound: Anger, Gratitude 등) 자동 생성 로직
- [x] PAD 기반 대사 자극 처리 엔진 (`stimulus.rs`)
- [x] 20개 이상의 시나리오 기반 단위/통합 테스트 통과

### 검증된 시나리오 (tests/)
- **의형제 배신**: 관계(Trust/Closeness)가 높을수록 배신 시 더 큰 분노 발생.
- **라이벌 승진**: 성격(H, A)에 따라 `HappyFor`(대리기쁨)와 `Resentment`(시기) 분기 확인.
- **두려움과 실망**: 미래 전망(`Prospect`)에 따른 `Fear` 및 결과 확인(`Confirmation`) 테스트.
- **대화 누적**: 여러 턴의 도발 자극이 감정을 점진적으로 폭발시키거나 소멸시키는 과정 검증.

---

## 향후 확장 계획
- **감정 감쇠(Decay)**: 대화가 없는 시간 경과에 따른 자연스러운 감정 감소 시스템 개발.
- **상황 텍스트 자동 변환**: LLM 또는 임베딩 모델을 사용하여 텍스트로부터 `Situation` 입력을 자동 추출하는 기능.
