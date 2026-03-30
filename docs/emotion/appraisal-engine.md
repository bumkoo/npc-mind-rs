# AppraisalEngine 설계 문서 (현행화)

## 개요

AppraisalEngine은 NPC 심리 엔진의 핵심 도메인 서비스이다.
**성격 가중치(AppraisalWeights)**, **상황(Situation)**, 그리고 대상과의 **관계 modifier(RelationshipModifiers)**를 입력받아 OCC 감정(EmotionState)을 생성한다.

현재 엔진은 **"정적 평가(Appraisal) + 동적 자극(Stimulus)"** 아키텍처를 채택하고 있다:
- `AppraisalEngine`: 상황 진입 시 1회 호출되어 초기 감정 상태를 결정한다.
- `StimulusEngine`: 대화 진행 중 대사 자극에 따라 감정 강도를 실시간으로 변동시킨다.

---

## appraise()

### 시그니처

```rust
pub trait Appraiser {
    fn appraise<P: AppraisalWeights>(
        &self,
        personality: &P,
        situation: &Situation,
        dialogue_modifiers: &RelationshipModifiers,
    ) -> EmotionState;
}
```

- `personality`: 성격 가중치 포트 (`HexacoProfile`이 기본 구현)
- `dialogue_modifiers`: 대화 상대와의 관계에서 사전 계산된 감정 배율 (intensity, trust, empathy, hostility)

### 역할

상황(Situation) 내의 각 포커스(Focus)를 분석하고, 성격과 관계 modifier를 가중치로 적용하여 감정을 생성한다.

1. **Focus 순회**: `Situation`에 포함된 `Option<EventFocus>`, `Option<ActionFocus>`, `Option<ObjectFocus>`를 각각 독립적으로 평가한다.
2. **복합 감정 감지**: Action과 Event가 동시에 존재할 경우, 이를 결합하여 **Compound 감정**(분노, 감사 등)을 자동으로 생성한다.
3. **관계 가중치 적용**: Admiration/Reproach에만 `rel_mul`과 `trust_mod`를 적용한다. 나머지 감정은 rel_mul 없음.
4. **Emotion context**: 각 감정에 Focus의 `description`이 부착되어 LLM 프롬프트에 포함된다.

---

## 가중치 시스템 (Weighting System)

### 1. 성격 가중치 (HEXACO)
범용 계수 **W(0.3)**를 사용하여 성격 점수(-1.0 ~ 1.0)를 강도 배율로 변환한다.
- `1.0 + (Score * W)`: 점수가 높을수록 감정 증폭
- `1.0 - (max(0, Score) * W)`: 점수가 높을수록 감정 억제 (예: 인내심에 의한 분노 억제)

### 2. 타인 복지 감정 보정 (Fortune-of-others)
사건의 대상이 타인인 경우(`DesirabilityForOther`), **해당 타인과의 관계에서 사전 계산된 `RelationshipModifiers`**가 공감/적대 배율로 개입한다.
- **`modifiers.empathy_modifier`**: `(1.0 + closeness × 0.3).max(0.0)`
  - **용도**: `HappyFor`(대리기쁨)와 `Pity`(동정)에 적용. 친할수록 타인의 행운에 더 기뻐하고 불행에 더 슬퍼한다.
- **`modifiers.hostility_modifier`**: `(1.0 - closeness × 0.3).max(0.0)`
  - **용도**: `Resentment`(시기)와 `Gloating`(고소함)에 적용. 친할수록 시기심이 억제되고, 사이가 나쁠수록 타인의 불행을 더 고소해한다.

**주의**: `intensity_multiplier`와 `trust_modifier`는 **Admiration/Reproach에만** 적용된다. Event 감정이나 Fortune-of-others에는 적용하지 않는다.

---

## 내부 평가 로직 및 HEXACO 매핑 (Internal Logic)

각 감정 분기에서 HEXACO 성격 차원은 감정의 발생 여부와 강도를 결정하는 핵심 가중치로 작용한다.

### 1. appraise_event() (사건 기반)
사건이 자신과 타인에게 미치는 바람직함(`desirability`)을 평가한다.
- **E (정서성)**: `Emotionality` 평균이 전반적 감정 폭(`emotional_amp`)을 결정하여 모든 사건 반응의 기본 크기를 조절한다.
- **X (외향성)**: `Extraversion` 평균이 `Joy`(기쁨)와 `Hope`(희망)를 추가 증폭한다.
- **A (원만성)**: `Agreeableness` 평균이 `Distress`(고통)를 억제하는 브레이크 역할을 한다.
- **C (성실성)**: `Prudence`(신중함)가 높을수록 부정적 사건에 대한 즉각적인 `Distress` 반응이 억제된다.
- **E.fearfulness**: 두려움(`Fear`) 감정의 강도를 직접 결정한다.
- **타인의 운 (Fortune-of-others)**:
  - **H (정직-겸손)** & **A (원만성)**: 두 수치가 높을수록 `HappyFor`(대리기쁨)와 `Pity`(동정)가 활성화되고, 낮을수록 `Resentment`(시기)와 `Gloating`(쾌재)이 발생한다.
  - **E.sentimentality**: 동정(`Pity`)의 정서적 깊이를 조절한다.

### 2. appraise_action() (행동 기반)
행위자의 행동이 얼마나 찬양/비난받을 만한지(`praiseworthiness`) 평가한다.
**3분기 구조**: `agent_id`와 `modifiers`에 따라 분기한다.
- `agent_id: None` → 자기 행동 → **Pride/Shame** (modifier 없음)
- `agent_id: Some(_)`, `modifiers: None` → 대화 상대 → **Admiration/Reproach** (appraise 파라미터의 dialogue_modifiers 사용)
- `agent_id: Some(_)`, `modifiers: Some(mods)` → 제3자 → **Admiration/Reproach** (제3자 modifiers 사용)

HEXACO 관여:
- **C (성실성)**: 성실성 평균이 **도덕적/사회적 기준(`standards_amp`)**으로 작용하여, 기준이 높을수록 모든 행동 감정의 강도가 강해진다.
- **H.modesty (겸손)**: 높을수록 자신의 선행에 대한 자부심(`Pride`)이 절제된다.
- **A.gentleness (온화함)**: 높을수록 타인의 잘못에 대한 비난(`Reproach`) 감정이 억제된다.
- **intensity_multiplier**: Admiration/Reproach에만 적용. `(1.0 + closeness × 0.5).max(0.0)`.
- **trust_modifier**: Admiration/Reproach에만 적용. `1.0 + trust × 0.3`.

### 3. appraise_compound() (복합 감정)
사건의 결과(`Event`)와 원인이 되는 행동(`Action`)이 결합될 때 발생한다.
- **분노 (Anger)**: 타인의 나쁜 행동 + 나쁜 결과. **A.patience(인내심)**가 높을수록 분노가 억제된다.
- **감사 (Gratitude)**: 타인의 좋은 행동 + 좋은 결과. **H.sincerity(진실성)**가 높을수록 감사의 진정성과 강도가 증폭된다.
- **만족감/후회**: 자신의 행동 결과에 따른 복합 반응에서도 `C(성실성)` 기반의 기준(`standards_amp`)이 강도를 조절한다.

### 4. appraise_object() (대상 기반)
대상의 매력(`appealingness`)을 평가한다.
- **O (개방성)**: `Aesthetic Appreciation`(미적 감수성)이 높을수록 대상에 대한 호불호(`Love`, `Hate`)가 더욱 명확하고 강하게 나타난다.

---

## 설계 판단 (Design Decisions)

### 1. 왜 RelationshipModifiers를 인자로 받는가?
동일한 배신이라도 "지나가는 행인"과 "의형제"의 배신은 NPC가 느끼는 감정의 무게가 완전히 다르기 때문이다. `Relationship` Aggregate의 내부 구조를 감정 도메인에 노출하지 않기 위해, 필요한 배율만 사전 계산한 `RelationshipModifiers` VO를 사용한다.

### 2. 정적 평가와 동적 자극의 분리
`AppraisalEngine`은 상황의 '사실 관계'를 기반으로 첫 감정을 잡는 데 집중하고, 이후 대화의 '뉘앙스'에 따른 변화는 `StimulusEngine`에 위임함으로써 책임 소재를 명확히 분리했다.

### 3. 복합 감정 자동 생성의 이점
`Situation` 데이터에 `Action`과 `Event`만 넣어주면 엔진이 자동으로 "이건 감사할 일이다" 혹은 "이건 화낼 일이다"라고 판단하므로, 상황 데이터를 생성하는 외부 시스템(LLM 등)의 부담을 줄여준다.

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|-----------|
| 0.1.0 | 2026-03-23 | 초기 설계안 작성 (Momentum 포함) |
| 0.2.0 | 2026-03-26 | **현행화**: 실제 구현된 Relationship 기반 시스템으로 전면 수정. appraise_with_context 삭제 및 appraise_compound 추가. |
| 0.3.0 | 2026-03-28 | Action 3분기(agent_id+relationship), rel_mul Admiration/Reproach 한정, Emotion context, empathy/hostility_rel_modifier 명칭 통일 |
| 0.4.0 | 2026-03-30 | Appraiser 시그니처 변경: &Relationship → &RelationshipModifiers, &HexacoProfile → &P: AppraisalWeights. Action 3분기 relationship → modifiers |
