# AppraisalEngine 설계 문서

## 개요

AppraisalEngine은 NPC 심리 엔진의 핵심이다.
HEXACO 성격과 Situation을 입력받아 OCC 감정(EmotionState)을 생성한다.

두 가지 공개 메서드를 제공한다:

| 메서드 | 용도 | 입력 |
|--------|------|------|
| `appraise()` | 1회성 평가 (맥락 없음) | personality + situation |
| `appraise_with_context()` | 대화 중 감정 누적 | personality + situation + current_state |

`appraise()`는 내부적으로 `appraise_with_context()`를 빈 상태로 호출한 것이다.
즉 `appraise_with_context()`가 실제 엔진이고, `appraise()`는 편의 메서드이다.

---

## appraise()

### 시그니처

```rust
pub fn appraise(
    personality: &HexacoProfile,
    situation: &Situation,
) -> EmotionState
```

### 역할

이전 맥락 없이 **단일 상황에 대한 감정을 생성**한다.
내부에서 빈 EmotionState를 만들어 appraise_with_context()에 전달한다.

```rust
pub fn appraise(personality: &HexacoProfile, situation: &Situation) -> EmotionState {
    Self::appraise_with_context(personality, situation, &EmotionState::new())
}
```

### 사용 시점

- NPC가 처음 등장할 때
- 단발성 이벤트 반응 (전투 시작, 아이템 발견 등)
- 이전 대화 맥락이 없는 첫 반응

### 예시

```rust
let state = AppraisalEngine::appraise(&교룡.personality, &Situation {
    description: "라이벌이 무림맹주에 추대됨".into(),
    focus: SituationFocus::Event {
        desirability_for_self: 0.0,
        desirability_for_other: Some(0.8),
        is_prospective: false,
        prior_expectation: None,
    },
});
// → EmotionState { Resentment(0.56) }
// EmotionalMomentum은 전부 0.0 → 순수하게 HEXACO만으로 계산
```

---

## appraise_with_context()

### 시그니처

```rust
pub fn appraise_with_context(
    personality: &HexacoProfile,
    situation: &Situation,
    current_state: &EmotionState,
) -> EmotionState
```

### 역할

**이전 감정 상태 위에 새 감정을 누적**한다.
현재 감정이 새 평가의 가중치로 작용하여, 대화가 진행될수록 감정이 변화한다.

### 입력

| 파라미터 | 타입 | 설명 |
|----------|------|------|
| personality | &HexacoProfile | NPC의 고정 성격 (대화 중 변하지 않음) |
| situation | &Situation | 이번 턴에 일어난 상황 |
| current_state | &EmotionState | 이전 턴까지 누적된 감정 상태 |

### 출력

EmotionState — 이전 감정 + 새 감정이 합산된 업데이트된 상태

### 사용 시점

- NPC와 여러 턴에 걸친 대화
- 연속된 사건에 대한 반응 (전투 중 상황 변화 등)
- 이전 감정이 다음 반응에 영향을 줘야 하는 모든 경우

### 실행 순서

```
1. EmotionalMomentum 산출
   current_state에서 4가지 영향 계수를 뽑음

2. 기존 감정 복제
   state = current_state.clone()
   (빈 상태가 아니라 이전 감정을 그대로 가져옴)

3. SituationFocus 분기
   Event / Action / Object 중 하나로 라우팅
   각 내부 함수에 momentum 전달

4. HEXACO 가중치 + momentum 가산
   성격 기본값에 감정 관성이 더해져 최종 가중치 결정

5. OCC 규칙에 따라 감정 생성
   새 감정이 state에 add() (같은 유형이면 강도 합산)

6. 업데이트된 EmotionState 반환
```

---

## EmotionalMomentum (감정 관성)

current_state에서 산출되는 4가지 영향 계수.
HEXACO 가중치에 가산되어 감정 강도를 변화시킨다.

### 산출 방법

```rust
EmotionalMomentum::from_state(current_state)
```

| 계수 | 산출 | 범위 | 의미 |
|------|------|------|------|
| negative_bias | \|overall_valence의 음수 부분\| × 0.5 | 0.0~0.5 | 이미 기분이 나쁘면 새 부정감정 증폭 |
| positive_bias | overall_valence의 양수 부분 × 0.3 | 0.0~0.3 | 이미 기분이 좋으면 새 긍정감정 증폭 |
| anger_erosion | anger_intensity × 0.5 | 0.0~0.5 | 기존 분노가 patience 브레이크를 갉아먹음 |
| sensitivity_boost | (fear + distress) / 2 × 0.3 | 0.0~0.3 | 기존 공포/고통이 감정 민감도를 높임 |

### 작용 지점

각 계수가 어느 가중치 공식에 가산되는지:

```
Event 분기:
  emotional_amp  = 1.0 + |E| × 0.3  + sensitivity_boost
  positive_amp   = 1.0 + X × 0.3    + positive_bias
  anger_mod      = (1.0 - A × 0.4)  + anger_erosion + negative_bias

Action 분기:
  reproach_amp   = (1.0 - gentleness × 0.3) + negative_bias
  anger_amp      = (1.0 - patience × 0.4)   + anger_erosion + negative_bias

Object 분기:
  현재 momentum 미적용 (향후 확장 가능)
```

---

## 내부 함수 3개

AppraisalEngine의 공개 메서드가 SituationFocus에 따라 호출하는 비공개 함수:

### appraise_event()

```
입력: personality, state, momentum, desirability_self, desirability_other,
      is_prospective, prior_expectation
```

분기 순서:
1. prior_expectation이 Some → Satisfaction / Disappointment / Relief / FearsConfirmed (단독 처리, return)
2. is_prospective = true → Hope / Fear (단독 처리, return)
3. desirability_self → Joy / Distress
4. desirability_other가 Some → HappyFor / Pity / Gloating / Resentment (3번과 병행)

HEXACO 영향:
- E → emotional_amp (전반적 감정 증폭)
- X → positive_amp (긍정 감정 증폭)
- A → anger_mod (부정 감정 완화/증폭)
- C.prudence → impulse_mod (즉각 반응 억제)
- E.fearfulness → fear_amp (Fear 직접 증폭)
- E.sentimentality → Pity의 compassion에 가산
- H, A → Fortune-of-others 분기 (HappyFor vs Resentment vs Gloating)

### appraise_action()

```
입력: personality, state, momentum, is_self_agent, praiseworthiness, outcome_for_self
```

분기 순서:
1. is_self_agent + praiseworthiness 방향 → Pride / Shame
2. !is_self_agent + praiseworthiness 방향 → Admiration / Reproach
3. outcome_for_self가 Some → Compound 감정 (Gratification / Remorse / Gratitude / Anger)

HEXACO 영향:
- C → standards_amp (자기 기준의 높이 → Pride/Shame 증폭)
- H.modesty → pride_mod (겸손하면 Pride 억제)
- A.gentleness → reproach_amp (온화하면 Reproach 억제)
- A.patience → anger_amp (인내심이 Anger 직접 조절)
- H.sincerity → gratitude_amp (진실하면 Gratitude 증폭)

### appraise_object()

```
입력: personality, state, momentum, appealingness
```

분기:
- appealingness > 0 → Love
- appealingness < 0 → Hate

HEXACO 영향:
- O.aesthetic_appreciation → aesthetic_amp (미적 감수성이 Love/Hate 증폭)

---

## 사용 예시: 교룡 3턴 대화

```rust
let yu = make_교룡();  // patience=-0.7, gentleness=-0.5

// 턴1: 1회성 (appraise 사용)
let turn1 = Situation {
    description: "그 검을 돌려주시오".into(),
    focus: SituationFocus::Event {
        desirability_for_self: -0.3, ..
    },
};
let state1 = AppraisalEngine::appraise(&yu.personality, &turn1);
// momentum = 전부 0 (빈 상태)
// → state1 = { Distress(0.3) }

// 턴2: 맥락 누적 (appraise_with_context 사용)
let turn2 = Situation {
    description: "그건 내 사부의 유품이오".into(),
    focus: SituationFocus::Action {
        is_self_agent: true, praiseworthiness: -0.4, ..
    },
};
let state2 = AppraisalEngine::appraise_with_context(
    &yu.personality, &turn2, &state1
);
// momentum = { negative_bias=0.15, ... } ← state1의 Distress에서 산출
// → state2 = { Distress(0.3), Shame(0.2) }

// 턴3: 분노 폭발
let turn3 = Situation {
    description: "도둑질이라 부를 수밖에".into(),
    focus: SituationFocus::Action {
        is_self_agent: false, praiseworthiness: -0.6,
        outcome_for_self: Some(-0.5),
    },
};
let state3 = AppraisalEngine::appraise_with_context(
    &yu.personality, &turn3, &state2
);
// momentum = { negative_bias=0.25, sensitivity_boost=0.045, ... }
// anger_amp = (1.0 - (-0.7)×0.4) + 0.0 + 0.25 = 1.53
// → state3 = { Distress, Shame, Anger(폭발), Reproach(증폭) }
```

핵심: 턴3의 Anger는 `appraise()`로 계산한 것보다 `appraise_with_context()`로
계산한 것이 더 강하다. negative_bias가 anger_amp에 가산되기 때문이다.

---

## appraise() vs appraise_with_context() 비교

| 항목 | appraise() | appraise_with_context() |
|------|------------|-------------------------|
| 입력 | personality + situation | personality + situation + current_state |
| 초기 상태 | 빈 EmotionState | current_state를 clone |
| momentum | 전부 0.0 | current_state에서 산출 |
| 감정 누적 | 없음 (매번 새로 시작) | 이전 감정 위에 누적 |
| 사용 시점 | 단발성 반응, 첫 턴 | 대화 중, 연속 사건 |
| 내부 구현 | appraise_with_context(p, s, &empty)를 호출 | 실제 엔진 |

---

## 설계 판단

### appraise()를 왜 남겨두는가

대부분의 단발성 사용에서 빈 EmotionState를 만들어 넘기는 보일러플레이트를
제거하기 위한 편의 메서드이다. 기존 테스트와의 하위 호환도 유지한다.

### momentum 계수가 왜 작은 값인가 (0.3, 0.5)

감정 관성이 성격보다 강해지면 HEXACO의 의미가 퇴색된다.
momentum은 성격 효과를 **미세 조정**하는 역할이지, 대체하는 역할이 아니다.
교룡(patience=-0.7)과 무백(patience=+0.8)의 차이가 항상 지배적이어야 한다.

### EmotionState를 clone하는 이유

입력 current_state를 변경하지 않고 새 상태를 반환한다.
호출자가 이전 상태를 보존할 수 있어서, 분기 시나리오("만약 다른 말을 했다면?")에
대한 비교가 가능하다. 테스트에서도 이 특성을 활용한다.

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|-----------|
| 0.1.0 | 2026-03-23 | 초기 작성. appraise(), appraise_with_context(), EmotionalMomentum, 내부 함수 3개, 설계 판단 정리 |
