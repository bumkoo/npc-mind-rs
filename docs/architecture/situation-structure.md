# Situation 구조 설계 문서 (현행화)

## 개요

Situation은 **AppraisalEngine의 입력**으로, NPC가 처한 상황을 구조화하여 OCC 감정 엔진에 전달하는 Value Object이다.

v2 리팩토링을 통해 **다중 초점(Multi-focus)** 지원이 강화되었으며, 외부 입력을 도메인 모델로 안전하게 변환하기 위한 **Application 계층의 Mapping 패턴**이 적용되었다.

---

## 데이터 흐름: DTO → Domain

라이브러리 외부에서 들어오는 데이터(`SituationInput`)는 Application 계층에서 도메인 모델(`Situation`)로 변환된다.

1.  **`SituationInput` (DTO)**: 네트워크나 파일을 통해 들어온 가공되지 않은 요청 데이터.
2.  **`to_domain(&self, repo, ...)`**: DTO에 구현된 변환 메서드.
    -   `MindRepository`를 사용하여 필요한 관계(Relationship)나 객체 정보(Object Description)를 조회한다.
    -   `Situation::new()` 스마트 생성자를 호출하여 유효성을 검증한다.
3.  **`Situation` (Domain)**: 감정 평가 엔진이 직접 다루는 순수 비즈니스 모델.

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

-   **최소 1개 Focus 필수**: `event`, `action`, `object` 중 하나 이상은 반드시 `Some`이어야 한다. (`SituationError::NoFocus` 검증)
-   **복합 감정 자동 생성**: 엔진이 `action`과 `event`의 동시 존재를 감지하여 Anger, Gratitude 등을 자동 도출한다.
-   **Context 전파**: 각 Focus의 `description`은 생성된 감정의 `context` 필드로 전파되어 향후 LLM 프롬프트 생성에 활용된다.

---

## Focus 상세 설계

### 1. EventFocus (사건 중심 평가)

```rust
pub struct EventFocus {
    pub description: String,
    pub desirability_for_self: f32,                     // -1.0 ~ 1.0 (Joy/Distress)
    pub desirability_for_other: Option<DesirabilityForOther>,
    pub prospect: Option<Prospect>,
}
```

-   **Well-being**: `desirability_for_self` 수치에 따라 기쁨 혹은 고통을 생성한다.
-   **Fortune-of-others**: `desirability_for_other`가 존재할 경우, 대상에 대한 관계(`Relationship`)를 고려하여 `HappyFor`, `Pity`, `Resentment`, `Gloating` 중 하나를 생성한다.
-   **Prospect**: `Anticipation`(미래)인 경우 `Hope/Fear`를, `Confirmation`(결과 확인)인 경우 `Satisfaction` 등 4종 감정을 생성한다.

---

### 2. ActionFocus (행동 중심 평가)

```rust
pub struct ActionFocus {
    pub description: String,
    pub agent_id: Option<String>,             // None=자기, Some(id)=타인
    pub praiseworthiness: f32,                // -1.0 ~ 1.0
    pub relationship: Option<Relationship>,   // 제3자 행동인 경우 제공
}
```

-   **Attribution 분기**: 행위자가 누구인지, 그리고 행위자와의 관계가 어떠한지에 따라 감정의 종류와 강도가 달라진다.
    -   **Self**: `Pride`(자부심) 또는 `Shame`(수치심)
    -   **Other**: `Admiration`(찬사) 또는 `Reproach`(비난)
-   **관계 보정**: 타인 행동 평가 시 `trust_mod`와 `rel_mul`이 강도에 반영된다.

---

### 3. ObjectFocus (대상 중심 평가)

```rust
pub struct ObjectFocus {
    pub target_id: String,            // 식별자
    pub target_description: String,   // 대상 설명 (감정 context로 사용)
    pub appealingness: f32,           // -1.0 ~ 1.0 (Love/Hate)
}
```

-   **Attraction**: 대상의 존재 자체에 대한 호불호를 평가한다. `appealingness`가 양수이면 `Love`(호감), 음수이면 `Hate`(반감)를 생성한다.

---

## 상황 구성 예시: "친구의 배신"

```rust
// 1. SituationInput DTO 생성 (JSON 역직렬화 결과라고 가정)
let input = SituationInput {
    description: "친구가 적에게 내 위치를 알렸다".into(),
    event: Some(EventInput { desirability_for_self: -0.7, ... }),
    action: Some(ActionInput { agent_id: Some("friend_id"), praiseworthiness: -0.8, ... }),
    object: None,
};

// 2. to_domain()을 통해 도메인 모델로 변환 (MindService 계층)
let situation = input.to_domain(&repository, "my_id", "friend_id")?;

// 3. 감정 평가 실행
let state = AppraisalEngine::appraise(&personality, &situation, &relationship);
// 결과: Reproach(비난) + Distress(고통) → Compound Anger(분노) 발생
```


---

## SceneFocus — Beat 전환을 위한 Focus 옵션 (v2.0 추가)

`SceneFocus`는 Situation의 확장으로, 장면(Scene) 내에서 **자동 Beat 전환**을 지원한다.
게임이 Scene 시작 시 여러 Focus 옵션을 제공하면, 엔진이 stimulus 처리 중 감정 조건을 평가하여 Beat 전환을 판단한다.

```rust
pub struct SceneFocus {
    pub id: String,                    // Focus 식별자
    pub description: String,           // LLM 가이드의 [상황] 섹션에 사용
    pub trigger: FocusTrigger,         // 전환 조건
    pub event: Option<EventFocus>,     // Situation과 동일한 구조
    pub action: Option<ActionFocus>,
    pub object: Option<ObjectFocus>,
}

pub enum FocusTrigger {
    Initial,                           // Scene 시작 시 바로 적용
    Conditions(Vec<Vec<EmotionCondition>>),  // OR [ AND[...], AND[...] ]
}

pub struct EmotionCondition {
    pub emotion: EmotionType,          // 대상 감정
    pub threshold: ConditionThreshold, // Below(f32) | Above(f32) | Absent
}
```

### 전환 흐름
1. stimulus 호출 → 감정 강도 조정 (관성 적용)
2. 대기 중 Focus의 trigger 조건 체크 (목록 순서 = 우선순위)
3. 조건 충족 시 → after_beat (관계 갱신) → 새 Focus로 appraise → merge_from_beat
