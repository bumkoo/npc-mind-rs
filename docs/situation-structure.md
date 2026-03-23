# Situation 구조 설계 문서

## 개요

Situation은 **AppraisalEngine의 입력**이다.
"NPC가 처한 상황"을 구조화하여 OCC 감정 엔진에 전달하는 역할을 한다.

게임/대화 시스템과 감정 엔진 사이의 **인터페이스**이며,
4사이클에서 fastembed(bge-m3)를 도입하면 자연어 텍스트를 자동으로
이 구조체로 변환하는 것이 목표이다.

---

## 전체 구조

```
Situation
├── description: String              ← 텍스트 설명 (LLM 가이드 생성용)
└── focus: SituationFocus            ← 감정 계산의 실제 입력
        ├── Event { ... }            ← 12개 감정 결정
        ├── Action { ... }           ←  8개 감정 결정
        └── Object { ... }           ←  2개 감정 결정
                                        총 22개 OCC 감정
```

Rust enum이므로 **셋 중 하나만** 선택된다.
하나의 Situation은 반드시 Event이거나 Action이거나 Object이다.

---

## Situation 필드

| 필드 | 타입 | 역할 |
|------|------|------|
| description | String | 상황의 텍스트 설명. 현재 엔진 연산에는 미사용. 3사이클 LLM 가이드 생성 시 활용 |
| focus | SituationFocus | 감정 계산의 실제 입력. OCC 3대 분기 중 하나 |

---

## SituationFocus: 3가지 분기

OCC 모델의 3대 분기와 1:1 대응한다.

### Event — "무슨 일이 일어났다"

사건의 결과에 대한 반응. 12개 감정을 결정한다.

```rust
Event {
    desirability_for_self: f32,
    desirability_for_other: Option<f32>,
    is_prospective: bool,
    prior_expectation: Option<PriorExpectation>,
}
```

#### 필드 설명

| 필드 | 타입 | 범위 | 설명 |
|------|------|------|------|
| desirability_for_self | f32 | -1.0 ~ 1.0 | 사건이 나에게 얼마나 좋은/나쁜 일인지 |
| desirability_for_other | Option | -1.0 ~ 1.0 또는 None | 사건이 타인에게 미치는 영향. None이면 타인 무관 |
| is_prospective | bool | true/false | 아직 안 일어난 미래 사건인지 |
| prior_expectation | Option | enum 또는 None | 이전에 기대했던 사건의 실현 여부 |

#### 필드 조합 → 감정 매핑

| desirability_self | desirability_other | is_prospective | prior_expectation | 결과 감정 |
|---|---|---|---|---|
| +양수 | - | false | None | **Joy** (기쁨) |
| -음수 | - | false | None | **Distress** (고통) |
| - | +양수 (H↑,A↑) | false | None | **HappyFor** (대리기쁨) |
| - | +양수 (H↓) | false | None | **Resentment** (시기) |
| - | -음수 (A↑) | false | None | **Pity** (동정) |
| - | -음수 (H↓,A↓) | false | None | **Gloating** (고소함) |
| +양수 | - | **true** | None | **Hope** (희망) |
| -음수 | - | **true** | None | **Fear** (두려움) |
| - | - | - | HopeFulfilled | **Satisfaction** (만족) |
| - | - | - | HopeUnfulfilled | **Disappointment** (실망) |
| - | - | - | FearUnrealized | **Relief** (안도) |
| - | - | - | FearConfirmed | **FearsConfirmed** (공포확인) |

#### Event 예시

적의 대군이 다가온다 (미래 위협):
```rust
Event {
    desirability_for_self: -0.7,     // 나에게 나쁜 일
    desirability_for_other: None,    // 타인 관점 해당 없음
    is_prospective: true,            // 아직 안 일어남 → Fear 경로
    prior_expectation: None,         // 새 사건
}
```

라이벌이 무림맹주에 추대됨 (타인의 운):
```rust
Event {
    desirability_for_self: 0.0,      // 나에게 직접 영향 없음
    desirability_for_other: Some(0.8),// 타인에게 매우 좋은 일
    is_prospective: false,           // 이미 일어남
    prior_expectation: None,         // 새 사건
}
// → HEXACO H↑이면 HappyFor, H↓이면 Resentment
```

해독약 구하기 실패 (희망의 미실현):
```rust
Event {
    desirability_for_self: -0.8,
    desirability_for_other: None,
    is_prospective: false,
    prior_expectation: Some(PriorExpectation::HopeUnfulfilled),
}
// → Disappointment
```

---

### Action — "누군가가 뭘 했다"

행위자의 행동에 대한 반응. 8개 감정을 결정한다.

```rust
Action {
    is_self_agent: bool,
    praiseworthiness: f32,
    outcome_for_self: Option<f32>,
}
```

#### 필드 설명

| 필드 | 타입 | 범위 | 설명 |
|------|------|------|------|
| is_self_agent | bool | true/false | 행위자가 나 자신인지 타인인지 |
| praiseworthiness | f32 | -1.0 ~ 1.0 | 행동이 칭찬받을만한 정도. 양수=칭찬, 음수=비난 |
| outcome_for_self | Option | -1.0 ~ 1.0 또는 None | 행동의 결과가 나에게 미친 영향. None이면 결과 무관 |

#### 필드 조합 → 감정 매핑

| is_self | praiseworthiness | outcome_for_self | 결과 감정 |
|---------|------------------|------------------|-----------|
| true | +양수 | None | **Pride** (자부심) |
| true | -음수 | None | **Shame** (수치심) |
| false | +양수 | None | **Admiration** (감탄) |
| false | -음수 | None | **Reproach** (비난) |
| true | +양수 | Some(+) | **Gratification** (Pride + Joy) |
| true | -음수 | Some(-) | **Remorse** (Shame + Distress) |
| false | +양수 | Some(+) | **Gratitude** (Admiration + Joy) |
| false | -음수 | Some(-) | **Anger** (Reproach + Distress) |

#### 복합 감정 (Compound)의 핵심

Anger와 Gratitude는 Action + Event가 결합된 **복합 감정**이다.
Action 분기 안에서 `outcome_for_self` 필드가 Event적 요소를 담당한다.

OCC 원본에서도 이 구조를 따른다:
- **Anger** = Reproach(타인 행동 비난) + Distress(나에게 나쁜 결과)
- **Gratitude** = Admiration(타인 행동 칭찬) + Joy(나에게 좋은 결과)

#### Action 예시

동료의 배신 (타인의 비난받을 행동 + 나에게 나쁜 결과):
```rust
Action {
    is_self_agent: false,            // 타인의 행동
    praiseworthiness: -0.7,          // 매우 비난받을 행동 → Reproach
    outcome_for_self: Some(-0.6),    // 나에게 나쁜 결과 → + Anger 복합
}
```

교룡이 검을 훔친 자신의 행동을 돌아봄:
```rust
Action {
    is_self_agent: true,             // 자기 자신의 행동
    praiseworthiness: -0.4,          // 스스로도 약간 나쁘다고 느낌 → Shame
    outcome_for_self: None,          // 결과보다 행동 자체에 대한 평가
}
```

---

### Object — "무언가를 접했다"

대상의 속성에 대한 반응. 2개 감정을 결정한다.

```rust
Object {
    appealingness: f32,
}
```

#### 필드 설명

| 필드 | 타입 | 범위 | 설명 |
|------|------|------|------|
| appealingness | f32 | -1.0 ~ 1.0 | 대상의 매력도. 양수=매력, 음수=혐오 |

#### 필드 → 감정 매핑

| appealingness | 결과 감정 |
|---------------|-----------|
| +양수 | **Love** (좋아함) |
| -음수 | **Hate** (싫어함) |

HEXACO O(개방성)의 aesthetic_appreciation이 Love/Hate 강도를 증폭한다.

#### Object 예시

명검을 보았다:
```rust
Object { appealingness: 0.8 }   // → Love (O↑이면 더 강하게)
```

독이 묻은 암기를 보았다:
```rust
Object { appealingness: -0.6 }  // → Hate
```

---

## PriorExpectation: 전망 확인 감정

Event 분기의 `prior_expectation` 필드에 사용된다.
이전에 Hope 또는 Fear를 느낀 사건의 결과가 확인될 때 발생하는 감정이다.

```rust
pub enum PriorExpectation {
    HopeFulfilled,    // 바랐던 일이 실현됨 → Satisfaction
    HopeUnfulfilled,  // 바랐던 일이 실현되지 않음 → Disappointment
    FearUnrealized,   // 두려워했던 일이 안 일어남 → Relief
    FearConfirmed,    // 두려워했던 일이 실현됨 → FearsConfirmed
}
```

OCC의 전망(prospect) 감정 흐름:

```
Hope(희망) ──실현──→ Satisfaction(만족)
           ──미실현→ Disappointment(실망)

Fear(두려움) ──실현──→ FearsConfirmed(공포확인)
             ──미실현→ Relief(안도)
```

---

## 데이터 흐름

```
게임/대화 시스템
    │
    │ 상황 발생 (텍스트 + 수치 파라미터)
    ▼
Situation { description, focus }
    │
    │ AppraisalEngine.appraise(personality, situation)
    │   또는
    │ AppraisalEngine.appraise_with_context(personality, situation, current_state)
    ▼
EmotionState { emotions: Vec<Emotion> }
    │
    │ 3사이클: LLM 프롬프트 가이드 생성
    ▼
LLM 연기 지시문
```

---

## 설계 판단

### 왜 SituationFocus가 enum인가

하나의 상황에서 "사건이 일어났고, 동시에 누군가의 행동이고, 대상도 있다"는
경우가 있을 수 있다. 하지만 현재는 단일 focus로 설계한 이유:

1. **OCC 원본 구조와 일치**: OCC도 22개 감정을 3개 분기로 배타적 분류
2. **복합 상황은 Action의 outcome_for_self로 해결**: Anger, Gratitude 등 복합 감정이 이미 Action 내부에서 Event를 포함
3. **복잡한 상황은 여러 Situation으로 분해 가능**: `appraise_with_context`를 연속 호출하면 감정이 누적됨

### 향후 확장 가능성

- `focus: Vec<SituationFocus>`로 변경하여 한 상황에서 여러 초점 동시 평가
- 4사이클: 자연어 텍스트 → Situation 자동 변환 (fastembed + bge-m3)
- description 필드에 화자, 대상자, 장소 등 메타 정보 추가

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|-----------|
| 0.1.0 | 2026-03-23 | 초기 작성. Situation/SituationFocus/PriorExpectation 전체 구조, 필드→감정 매핑표, 예시, 설계 판단 |
