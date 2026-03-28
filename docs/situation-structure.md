# Situation 구조 설계 문서 (현행화)

## 개요

Situation은 **AppraisalEngine의 입력**이다. NPC가 처한 상황을 구조화하여 OCC 감정 엔진에 전달하는 역할을 한다.

핵심은 **다중 초점(Multi-focus)** 지원이다. 하나의 상황이 사건(Event), 행동(Action), 대상(Object)의 성격을 동시에 가질 수 있으며, 엔진은 이를 조합하여 복합 감정을 생성한다.

---

## 전체 구조

```rust
pub struct Situation {
    pub description: String,              // 전체 상황 설명 (Compound 감정의 context)
    pub event: Option<EventFocus>,        // Well-being, Prospect 등 12개 감정
    pub action: Option<ActionFocus>,      // Attribution 등 8개 감정
    pub object: Option<ObjectFocus>,      // Love/Hate 2개 감정
}
```

- **Option 기반**: 3개 Focus 중 최소 1개는 필수 (`Situation::new()` 스마트 생성자).
- **복합 감정 자동 생성**: Action과 Event가 동시에 존재하면 엔진이 Anger/Gratitude 등을 자동 생성.
- **description**: 각 Focus에도 개별 description이 있고, Situation 전체에도 description이 있음.
  Compound 감정의 context는 `situation.description`을 사용.

---

## Focus 상세

### 1. EventFocus (사건 발생)

```rust
pub struct EventFocus {
    pub description: String,                           // "밀고로 인한 추방 위기"
    pub desirability_for_self: f32,                     // -1.0 ~ 1.0
    pub desirability_for_other: Option<DesirabilityForOther>,
    pub prospect: Option<Prospect>,
}

pub struct DesirabilityForOther {
    pub target_id: String,
    pub desirability: f32,
    pub relationship: Relationship,  // NPC → 대상 관계
}
```

- **description**: 감정의 context로 사용됨 (Joy, Distress, Hope, Fear 등)
- **desirability_for_self**: 자신에게 바람직한 정도
- **desirability_for_other**: 제3자에게 미치는 영향 (HappyFor, Pity, Resentment, Gloating)
- **prospect**: Anticipation(미래) 또는 Confirmation(결과 확인)

#### Prospect 및 결과 확인
- `Anticipation` → Hope(긍정) 또는 Fear(부정)
- `Confirmation(HopeFulfilled)` → Satisfaction
- `Confirmation(HopeUnfulfilled)` → Disappointment
- `Confirmation(FearUnrealized)` → Relief
- `Confirmation(FearConfirmed)` → FearsConfirmed

---

### 2. ActionFocus (행동 평가) — 3분기 구조

```rust
pub struct ActionFocus {
    pub description: String,                  // "교룡의 밀고 행위"
    pub agent_id: Option<String>,             // None=자기, Some(id)=타인
    pub praiseworthiness: f32,                // -1.0 ~ 1.0
    pub relationship: Option<Relationship>,   // 제3자면 관계 포함
}
```

#### 3분기 로직

| agent_id | relationship | 의미 | 생성 감정 | rel_mul/trust_mod 출처 |
|---|---|---|---|---|
| `None` | `_` | 자기 행동 | Pride/Shame | 없음 |
| `Some(_)` | `None` | 대화 상대 행동 | Admiration/Reproach | appraise 파라미터 |
| `Some(_)` | `Some(rel)` | 제3자 행동 | Admiration/Reproach | 제3자 relationship |

---

### 3. ObjectFocus (대상 인식)

```rust
pub struct ObjectFocus {
    pub target_id: String,            // 게임 시스템 참조용 ("천잠사검")
    pub target_description: String,   // Love/Hate의 context ("천잠사로 만든 명검")
    pub appealingness: f32,           // -1.0 ~ 1.0
}
```

---

## Context 매핑 규칙

각 감정에 `context: Option<String>`이 부착되며, Focus의 description에서 복사됨.

| 감정 | context 출처 |
|---|---|
| Joy, Distress, Hope, Fear, 확인4종 | `event.description` |
| HappyFor, Pity, Resentment, Gloating | `"{event.description} (대상: {target_id})"` |
| Pride, Shame, Admiration, Reproach | `action.description` |
| Love, Hate | `object.target_description` |
| Compound (Anger, Gratitude 등) | `situation.description` |

---

## 상황 구성 예시

### "동료의 배신" (Action + Event 결합)
```rust
Situation::new(
    "동료 무사가 적에게 아군의 위치를 밀고했다".into(),
    Some(EventFocus {
        description: "밀고로 인한 추방 위기".into(),
        desirability_for_self: -0.7,
        desirability_for_other: None,
        prospect: None,
    }),
    Some(ActionFocus {
        description: "교룡의 밀고 행위".into(),
        agent_id: Some("gyo_ryong".into()),  // 대화 상대
        praiseworthiness: -0.8,
        relationship: None,
    }),
    None,  // object 없음
)
// → Reproach + Distress → Compound Anger 자동 생성
```
