# Situation 구조 설계 문서 (현행화)

## 개요

Situation은 **AppraisalEngine의 입력**이다. NPC가 처한 상황을 구조화하여 OCC 감정 엔진에 전달하는 역할을 한다.

현재 구현(v2)의 핵심은 **다중 초점(Multi-focus)** 지원이다. 하나의 상황이 사건(Event), 행동(Action), 대상(Object)의 성격을 동시에 가질 수 있으며, 엔진은 이를 조합하여 복합 감정을 생성한다.

---

## 전체 구조

```
Situation
├── description: String              ← 텍스트 설명 (LLM 가이드 생성용)
└── focuses: Vec<SituationFocus>     ← 감정 계산의 실제 입력 (다중 초점)
        ├── Event(EventFocus)        ← Well-being, Prospect 등 12개 감정
        ├── Action(ActionFocus)      ← Attribution 등 8개 감정
        └── Object(ObjectFocus)      ← Love/Hate 2개 감정
```

- **다중 초점**: `Vec`을 사용하여 한 번에 여러 초점을 전달할 수 있다.
- **복합 감정 자동 생성**: `focuses`에 `Action`과 `Event`가 동시에 존재하면 엔진이 이를 감지하여 **분노(Anger)**나 **감사(Gratitude)** 등을 자동으로 생성한다.

---

## SituationFocus: 3가지 분기 상세

### 1. Event (사건 발생)
누군가에게 무슨 일이 일어났을 때의 반응이다.

```rust
pub struct EventFocus {
    pub desirability_for_self: f32,
    pub desirability_for_other: Option<DesirabilityForOther>,
    pub prospect: Option<Prospect>,
}
```

- **desirability_for_self**: 자신에게 바람직한 정도 (-1.0 ~ 1.0).
- **desirability_for_other**: 타인에게 미치는 영향. **Relationship** 정보를 포함하는 `DesirabilityForOther` 구조체를 사용한다.
- **prospect**: 전망 정보. `Anticipation`(미래) 또는 `Confirmation`(결과 확인)을 나타낸다.

#### Prospect 및 결과 확인 (ProspectResult)
- **Anticipation**: 미래 사건 예측 → `Hope`(희망) 또는 `Fear`(두려움) 생성.
- **Confirmation**: 이전 전망의 결과 확인.
  - `HopeFulfilled` → **Satisfaction** (만족)
  - `HopeUnfulfilled` → **Disappointment** (실망)
  - `FearUnrealized` → **Relief** (안도)
  - `FearConfirmed` → **FearsConfirmed** (공포확인)

---

### 2. Action (행동 평가)
행위자의 행동이 칭찬/비난받을 만한지에 대한 반응이다.

```rust
pub struct ActionFocus {
    pub is_self_agent: bool,
    pub praiseworthiness: f32,
}
```

- **is_self_agent**: 행위자가 나 자신인지 여부.
- **praiseworthiness**: 행동의 칭찬/비난 정도 (-1.0 ~ 1.0).
- **v2 변경점**: `outcome_for_self` 필드가 제거되었다. 행동의 결과는 별도의 `EventFocus`를 `focuses` 벡터에 함께 담아 전달함으로써 복합 감정을 생성한다.

---

### 3. Object (대상 인식)
대상의 매력도에 대한 반응이다.

```rust
pub struct ObjectFocus {
    pub appealingness: f32,
}
```

- **appealingness**: 대상의 매력도 (-1.0 ~ 1.0). 양수면 `Love`, 음수면 `Hate`를 생성한다.

---

## 상황 구성 예시 (다중 초점 활용)

### "동료의 배신" (Action + Event 결합)
```rust
Situation {
    description: "동료 무사가 적에게 아군의 위치를 밀고했다".into(),
    focuses: vec![
        SituationFocus::Action(ActionFocus {
            is_self_agent: false,
            praiseworthiness: -0.7, // 비난받을 행동
        }),
        SituationFocus::Event(EventFocus {
            desirability_for_self: -0.6, // 나에게 나쁜 결과
            desirability_for_other: None,
            prospect: None,
        }),
    ],
}
// → 엔진은 Reproach(비난)와 Distress(고통)를 기반으로 Anger(분노)를 생성한다.
```

---

## 설계 판단 (Design Decisions)

1. **왜 Vec<SituationFocus>인가?**: 현실의 상황은 단편적이지 않다. "친구가 나를 도와주려다(Action) 실수로 내 소중한 물건을 깼다(Event)"와 같은 상황을 하나의 `Situation` 객체에 온전히 담기 위함이다.
2. **Value Object 원칙**: `Situation`은 고유 ID가 없는 순수 데이터 객체이다. 상황의 추적과 맥락 연결은 엔진 외부(게임 시스템)의 책임이다.
3. **확장성**: 향후 LLM이 상황을 분석하여 이 구조체로 변환할 때, 여러 관점을 배열 형태로 유연하게 담을 수 있도록 설계되었다.
