# NPC 심리 엔진 아키텍처 v2

## 개요

NPC 심리 엔진은 **성격(HEXACO)**이 **상황(Situation)**을 해석하여 **감정(OCC)**을 생성하고,
이를 LLM이 연기할 수 있도록 **가이드(ActingGuide)**를 출력하는 시스템이다.

v2에서는 대화 중 감정 변동, NPC 간/플레이어 간 관계, 감정 분류 도구를 통합하여
4레이어 아키텍처로 재설계한다.

### 참조 이론

| 이론 | 역할 | 출처 |
|------|------|------|
| HEXACO | 성격 모델 (6차원 × 4 facet) | Ashton & Lee, 2007 |
| OCC | 감정 구조 (22개 감정, 3분기) | Ortony, Clore, Collins, 1988 |
| PAD | 감정 공간 (3축 연속 좌표) | Mehrabian & Russell, 1974 |
| ALMA | OCC↔PAD 매핑 | Gebhard, 2005 |

### 관련 문서

- [HEXACO 연구](hexaco-research.md)
- [OCC 감정 모델](occ-emotion-model.md)
- [Situation 구조](situation-structure.md)
- [AppraisalEngine 설계](appraisal-engine.md)
- 차원별 가이드: [H](h-dimension-guide.md) · [E](e-dimension-guide.md) · [X](x-dimension-guide.md) · [A](a-dimension-guide.md) · [C](c-dimension-guide.md) · [O](o-dimension-guide.md)

---

## 4레이어 아키텍처

### 전체 흐름

```
┌─────────────────────────────────────────────────────────┐
│  레이어1: Situation (세계관 기준, 객관, 고정)               │
│  "밀고는 나쁜 행위다" (praiseworthiness: -0.7)            │
│                                                           │
│  레이어2: HEXACO (성격, 고정)                              │
│  "나는 참을성 있는 사람이다" (patience: +0.8)               │
│                                                           │
│  레이어3: Relationship (상대별 관계, 대화 중 고정)           │
│  "이 사람은 나의 의형제였다" (closeness: 0.9, trust: 0.8)  │
│                                                           │
│  → AppraisalEngine.appraise() → 초기 EmotionState         │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│  레이어4: PAD 자극 (대화 중, 매 턴 변동)                    │
│                                                           │
│  대사 텍스트 → PAD 앵커 임베딩 → PAD(P, A, D)             │
│  × Receptivity (현재 감정과 자극의 공명)                    │
│  × HEXACO (자극 수용 방식 조절)                            │
│  → apply_stimulus() → 갱신된 EmotionState                 │
│  → ActingGuide (+Relationship) → LLM 대사 생성            │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│  대화 종료 후                                              │
│  최종 EmotionState + Situation → Relationship 갱신         │
└─────────────────────────────────────────────────────────┘
```

### 레이어별 책임

| 레이어 | 담당 | 입력 | 변하는가 |
|--------|------|------|----------|
| Situation | 이 세계에서 무슨 일이 일어났는가 | 게임 시스템 | 고정 (상황 전환 시 새로 생성) |
| HEXACO | 이 NPC는 어떤 사람인가 | NPC 정의 | 고정 |
| Relationship | 이 NPC와 상대는 어떤 사이인가 | 상대별 관계 데이터 | 대화 중 고정, 대화 후 갱신 |
| PAD 자극 | 지금 이 대화에서 무슨 말이 오가는가 | 대사 텍스트 | 매 턴 변동 |

### 상황 전환

대화 중 상황이 바뀌는 경우(예: "대화 중 갑자기 적이 습격"):
- 새 Situation을 생성하여 `appraise()` 재호출
- 새로운 대화 세션으로 취급
- 이전 EmotionState는 초기화 (새 상황의 감정으로 대체)

### 이번 개발 범위 (v2 스코프)

대화 세션 안에서 Situation은 **고정**으로 본다.
상황 전환은 일어날 수 있지만, 이번 개발에서는 구현하지 않는다.

```
이번 개발에서 고정:
  Situation    = 고정 (대화 중 변경 없음)
  HEXACO       = 고정
  Relationship = 고정

이번 개발에서 변동:
  EmotionState = 기존 감정 강도만 변동 (새 감정 생성 없음)
  유일한 변수   = PAD 자극 (매 턴)

향후 확장:
  대화 중 상황 전환 → 새 Situation → appraise() 재호출 → 새 감정 생성
  플레이어 행동 선택에 의한 서사 전환
```

**핵심 원칙:**
- **감정 강도 변동** → `apply_stimulus()` (PAD 자극)
- **새 감정 생성** → `appraise()` (Situation 경유, 항상)
- 대사는 대사 수준 (감정 강도 변동), 행동은 서사 수준 (상황 전환)

---

## 레이어1: Situation (변경 없음)

### 설계 원칙

Situation의 f32 값들은 **이 세계(무림)에서의 전형적 해석**이다.
NPC 개인의 주관적 해석이 아니라, 세계관 기준의 객관적 사건 강도.

```
"동료가 적에게 아군 위치를 밀고했다"
  → 무림 세계에서 이건 praiseworthiness: -0.7 짜리 행위
  → 무백이든 교룡이든 같은 Situation을 받음
  → 차이를 만드는 건 HEXACO와 Relationship
```

### 현재 구조 (유지)

```rust
pub struct Situation {
    pub description: String,      // 텍스트 설명 (LLM 가이드용)
    pub focus: SituationFocus,    // 감정 계산의 실제 입력
}

pub enum SituationFocus {
    Event {
        desirability_for_self: f32,
        desirability_for_other: Option<f32>,
        is_prospective: bool,
        prior_expectation: Option<PriorExpectation>,
    },
    Action {
        is_self_agent: bool,
        praiseworthiness: f32,
        outcome_for_self: Option<f32>,
    },
    Object {
        appealingness: f32,
    },
}
```

---

## 레이어2: HEXACO (변경 없음)

6차원 × 4 facet = 24개 facet으로 NPC의 성격을 정의.
각 값은 -1.0 ~ 1.0 범위, 0.0이 평균적 성격.
AppraisalEngine에서 감정 강도의 가중치로 작용.

상세: [HEXACO 연구](hexaco-research.md)

---

## 레이어3: Relationship (신규)

### 설계 원칙

- NPC와 NPC, NPC와 플레이어 사이의 관계를 모델링
- **상대마다 다름**: 교룡→무백 관계와 교룡→소호 관계는 별개
- **대화 중 고정**: 한 대화 세션 안에서는 변하지 않음
- **대화 후/이벤트 시 갱신**: 대화 결과 또는 서사 이벤트로 변경

### 3축 정의

```rust
pub struct Relationship {
    /// 친밀도 (-1.0=적대, 0.0=무관, 1.0=절친)
    pub closeness: f32,
    /// 신뢰도 (-1.0=불신, 0.0=중립, 1.0=전적 신뢰)
    pub trust: f32,
    /// 상하 관계 (-1.0=하위, 0.0=대등, 1.0=상위)
    pub power: f32,
}
```

### 축별 역할

#### closeness — 감정 반응의 전반적 배율

가까운 사이일수록 감정 반응이 강해진다.
Fortune-of-others 분기에서 방향도 결정.

```
사용처: AppraisalEngine + ActingGuide

AppraisalEngine에서:
  closeness 높음 + 타인에게 좋은 일 → HappyFor 증폭
  closeness 높음 + 타인에게 나쁜 일 → Pity 증폭
  closeness 낮음/적대 + 타인에게 좋은 일 → Resentment 쪽으로
  closeness 낮음/적대 + 타인에게 나쁜 일 → Gloating 쪽으로

ActingGuide에서:
  closeness → 어휘 친밀도 결정
```

**HEXACO H(정직-겸손성)와의 관계:**
- H = "나는 원래 타인의 행운을 시기하는 사람인가" (성격, 상대 불문)
- closeness = "이 사람의 행운을 시기하는가" (관계, 상대별)
- 둘 다 Fortune-of-others 분기에 영향하지만 역할이 다름

#### trust — 기대 위반/부합에 따른 감정 조절

신뢰와 행동의 불일치가 감정 강도를 증폭/완화한다.
OCC의 unexpectedness(예상치 못한 정도) 변수 역할을 수행.

```
사용처: AppraisalEngine + ActingGuide

AppraisalEngine에서:
  trust 높았는데 배신 (trust 0.8 + praiseworthiness -0.7)
    → 기대 위반 극대 → Anger/Disappointment 증폭

  trust 낮았는데 배신 (trust -0.5 + praiseworthiness -0.7)
    → 기대 부합 → "역시나" → Anger 감소, FearsConfirmed 쪽으로

  trust 낮았는데 도움 (trust -0.5 + praiseworthiness 0.7)
    → 기대 위반 → Gratitude/Satisfaction 증폭

기대 위반도 공식:
  expectation_violation = |trust - praiseworthiness의 부호 방향|
  위반 시 → 감정 증폭
  부합 시 → 감정 완화

ActingGuide에서:
  trust → 경계/개방 정도
```

#### power — 대사 생성 톤 결정 (감정 엔진 영향 최소)

```
사용처: ActingGuide 중심

ActingGuide에서:
  같은 Anger 0.6이라도:
    상위자에게: "사부... 어찌 그러십니까" (공손하지만 분노)
    대등자에게: "도대체 왜 그런 거요" (직접적 분노)
    하위자에게: "감히!" (권위적 분노)

  → 존댓말/반말, 권위적/공손한 톤 결정

AppraisalEngine에서:
  제한적 영향 (Pride/Shame 약간)
  상위자에게 칭찬받음 → Pride 증폭
  상위자에게 비난받음 → Shame 증폭
```

### 변경 시점과 트리거

| 축 | 변경 트리거 | 변경 시점 | 변경 속도 |
|---|---|---|---|
| power | 게임 서사 이벤트 (승급, 내공 상실 등) | 이벤트 발생 시 | 급격 (즉시) |
| trust | Action의 praiseworthiness 결과 | 대화 후 / 이벤트 시 | 중간 (점진적, 배신은 급락) |
| closeness | 대화 감정 결과 + 이벤트 | 대화 후 / 이벤트 시 | 느림 (매우 점진적) |

### 대화 후 갱신 메커니즘

```rust
/// 대화 종료 후 관계 갱신
pub fn update_after_dialogue(
    relationship: &mut Relationship,
    final_state: &EmotionState,
    situation: &Situation,
) {
    // trust: Action 분기의 praiseworthiness 기반
    if let SituationFocus::Action { praiseworthiness, .. } = &situation.focus {
        relationship.trust += praiseworthiness * TRUST_UPDATE_RATE;
        relationship.trust = relationship.trust.clamp(-1.0, 1.0);
    }

    // closeness: 대화의 전체 감정 결과 기반
    let valence = final_state.overall_valence();
    relationship.closeness += valence * CLOSENESS_UPDATE_RATE;
    relationship.closeness = relationship.closeness.clamp(-1.0, 1.0);
}
```

### 피드백 루프 방지

대화 중에 Relationship이 실시간으로 변하면 감정 계산이 불안정해진다.
따라서 **대화 세션 안에서는 Relationship을 고정**하고,
대화 종료 후 최종 결과를 기반으로 갱신한다.

```
대화 세션 안:
  Situation    = 고정
  HEXACO       = 고정
  Relationship = 고정  ← 이게 핵심
  PAD 자극만   = 매 턴 변동

대화 세션 후:
  EmotionState의 최종 결과를 보고
  Relationship 갱신 판단
```

---

## 레이어4: PAD 자극 (신규)

### 설계 원칙

대화 중 대사가 NPC의 감정을 변동시키는 유일한 경로.
Situation을 재평가하는 것이 아니라, **대사의 감정적 자극이 기존 감정 상태를 흔드는 것**.

### PAD 모델

```
P (Pleasure)   = 쾌 ────── 불쾌     (-1.0 ~ 1.0)
A (Arousal)    = 각성 ──── 이완     (-1.0 ~ 1.0)
D (Dominance)  = 지배 ──── 복종     (-1.0 ~ 1.0)
```

무협 대사 예시:

```
"도둑질이라 부를 수밖에"        → P:-0.6  A:+0.7  D:+0.5
  불쾌, 고각성, 지배적 = 비난/분노 영역

"대협의 은혜를 잊지 않겠소"      → P:+0.7  A:+0.3  D:-0.3
  쾌, 중각성, 복종적 = 감사/경의 영역

"어찌할 바를 모르겠소..."       → P:-0.5  A:-0.2  D:-0.7
  불쾌, 저각성, 무력 = 슬픔/절망 영역

"이 검은 내 것이오. 물러서시오"  → P:-0.3  A:+0.4  D:+0.8
  약간 불쾌, 중각성, 강한 지배 = 거부/강경 영역

"두고 보자... 반드시 갚아주마"   → P:-0.8  A:+0.3  D:-0.2
  강한 불쾌, 중각성, 약간 복종 = 굴욕적 원한 영역
```

### PAD 추출: 앵커 임베딩 방식

3축이 곧 3개의 양극단 앵커 쌍.
각 축마다 양극단 앵커 텍스트를 bge-m3로 임베딩하고,
입력 대사와의 코사인 유사도 차이로 -1.0~1.0 스칼라를 추출.

```
P축 앵커:
  (+) "기쁘고 흐뭇하다" / (-) "괴롭고 불쾌하다"

A축 앵커:
  (+) "격앙되어 흥분한다" / (-) "차분하고 담담하다"

D축 앵커:
  (+) "내가 주도한다, 물러서라" / (-) "어찌할 바를 모르겠다"
```

추출 공식:

```
P = sim(대사, P+앵커) - sim(대사, P-앵커)  → 정규화하여 -1.0 ~ 1.0
A = sim(대사, A+앵커) - sim(대사, A-앵커)  → 정규화하여 -1.0 ~ 1.0
D = sim(대사, D+앵커) - sim(대사, D-앵커)  → 정규화하여 -1.0 ~ 1.0
```

앵커 안정화: 각 앵커는 **여러 변형 표현의 평균 벡터**로 구성.

```rust
pub struct PadAnchor {
    /// 이 앵커를 대표하는 여러 표현
    pub variants: Vec<String>,
    /// 사전 계산된 임베딩 (variants의 평균 벡터)
    pub embedding: Vec<f32>,
    /// 이 앵커의 수치 값 (+1.0 또는 -1.0)
    pub value: f32,
}

pub struct PadAnchorSet {
    pub pleasure_positive: PadAnchor,    // P+
    pub pleasure_negative: PadAnchor,    // P-
    pub arousal_positive: PadAnchor,     // A+
    pub arousal_negative: PadAnchor,     // A-
    pub dominance_positive: PadAnchor,   // D+
    pub dominance_negative: PadAnchor,   // D-
}
```

PAD 앵커는 **Situation focus에 무관하게 범용**.
모든 분기에서 같은 앵커 세트를 사용.

### PAD 추출 파이프라인

```
대사 텍스트
  ├─ Aho-Corasick: 감정 키워드 신호 빠른 탐지 (힌트)
  ├─ bge-m3: 대사 임베딩 → 6개 앵커와 유사도 계산
  └─ → PAD(P, A, D) 출력
```

bge-reranker는 PAD 추출에는 불필요 (앵커가 6개뿐이라 top-k 재순위가 필요 없음).
Aho-Corasick은 보조 역할 (힌트 제공, 앵커 유사도에 가산점).

### 감정 분류 도구 적용 대상

| 대사 출처 | 분석 필요 여부 | 이유 |
|-----------|---------------|------|
| 플레이어 자유 입력 | 필요 | 구조화되지 않은 자연어 |
| LLM이 생성한 NPC 대사 | 불필요 | 감정이 먼저 결정되고 대사가 나옴 |
| 스크립트된 대사 | 불필요 | 작가가 태깅 |

### apply_stimulus 설계

`appraise_with_context()`를 대체하는 새 메서드.
**기존 감정의 강도만 변동시키며, 새 감정을 생성하지 않는다.**

설계 원칙: **곱셈과 덧셈만으로 구현. 벡터 정규화, 별도 크기 계산 없음.**

#### OCC→PAD 매핑 테이블

22개 감정은 각각 PAD 좌표를 갖는다 (Gebhard 2005, ALMA 모델 참고).
`EmotionType::to_pad()`로 접근. 대표값이며 플레이테스트로 튜닝 대상.

```
Anger         → P:-0.51, A:+0.59, D:+0.25
Reproach      → P:-0.30, A:+0.20, D:+0.40
Distress      → P:-0.40, A:+0.20, D:-0.50
Fear          → P:-0.64, A:+0.60, D:-0.43
Disappointment→ P:-0.30, A:-0.40, D:-0.40
FearsConfirmed→ P:-0.50, A:+0.30, D:-0.60
Shame         → P:-0.30, A:+0.10, D:-0.60
Remorse       → P:-0.30, A:+0.10, D:-0.60
Resentment    → P:-0.20, A:+0.30, D:-0.20
Hate          → P:-0.60, A:+0.60, D:+0.30
Pity          → P:-0.40, A:-0.20, D:-0.50
Joy           → P:+0.40, A:+0.20, D:+0.10
Hope          → P:+0.20, A:+0.20, D:-0.10
Satisfaction  → P:+0.30, A:-0.20, D:+0.40
Relief        → P:+0.20, A:-0.30, D:+0.20
Pride         → P:+0.40, A:+0.30, D:+0.30
Admiration    → P:+0.50, A:+0.30, D:-0.20
Gratitude     → P:+0.40, A:+0.20, D:-0.30
Gratification → P:+0.50, A:+0.40, D:+0.40
HappyFor      → P:+0.40, A:+0.20, D:+0.20
Gloating      → P:+0.30, A:+0.30, D:+0.30
Love          → P:+0.30, A:+0.10, D:+0.20
```

#### 핵심 로직: 함수 3개, 30줄 미만

```rust
const IMPACT_RATE: f32 = 0.1;
const FADE_THRESHOLD: f32 = 0.05;

pub fn apply_stimulus(
    personality: &HexacoProfile,
    current_state: &EmotionState,
    stimulus: &PAD,
) -> EmotionState {
    let absorb = stimulus_absorb_rate(personality, stimulus);
    let mut new_state = current_state.clone();

    for emotion in current_state.emotions() {
        let alignment = pad_dot(&emotion.emotion_type().to_pad(), stimulus);
        let delta = alignment * absorb * IMPACT_RATE;
        let new_intensity = (emotion.intensity() + delta).clamp(0.0, 1.0);

        if new_intensity < FADE_THRESHOLD {
            new_state.remove(emotion.emotion_type());
        } else {
            new_state.set_intensity(emotion.emotion_type(), new_intensity);
        }
    }
    new_state
}

/// PAD 단순 내적 — 같은 방향이면 양수, 반대면 음수.
/// 자극이 강할수록 내적 절대값이 커서 자극 크기도 자동 반영.
fn pad_dot(a: &PAD, b: &PAD) -> f32 {
    a.pleasure * b.pleasure + a.arousal * b.arousal + a.dominance * b.dominance
}

/// HEXACO 기반 자극 수용도 — 성격에 따라 자극을 걸러내거나 증폭.
fn stimulus_absorb_rate(p: &HexacoProfile, stimulus: &PAD) -> f32 {
    let avg = p.dimension_averages();
    let mut rate = 1.0;
    rate += avg.e.abs() * 0.3;                                      // E: 전반적 민감도
    if stimulus.pleasure < 0.0 {
        rate -= p.agreeableness.patience.value().max(0.0) * 0.4;    // A: 부정 자극 완충
    }
    rate -= p.conscientiousness.prudence.value().max(0.0) * 0.3;    // C: 급변 억제
    rate.max(0.1)  // 완전 무시 방지
}
```

#### 동작 원리

**pad_dot (공명 + 자극 크기 동시 처리):**

내적이 하는 일: 방향이 같으면 양수(증폭), 반대면 음수(감소).
자극 PAD의 크기가 클수록 내적 절대값이 커서 별도 magnitude 계산 불필요.

```
Anger PAD(-0.51, +0.59, +0.25) · 도발(-0.6, +0.7, +0.5)
= 0.306 + 0.413 + 0.125 = +0.844  → 같은 방향, 증폭

Anger PAD(-0.51, +0.59, +0.25) · 사과(+0.5, -0.3, -0.4)
= -0.255 - 0.177 - 0.100 = -0.532  → 반대 방향, 감소
```

**stimulus_absorb_rate (HEXACO 자극 수용도):**

기존 AppraisalEngine의 가중치 상수를 그대로 사용. 곱셈 체인 대신 덧셈/뺄셈.

X(외향성) 긍정 증폭을 삭제한 이유: `appraise()`에서 이미 X가 Joy/Hope 등의
초기 강도를 높여놨음. apply_stimulus에서 또 적용하면 이중 적용.

**기존 EmotionalMomentum 4가지 효과가 pad_dot 하나로 자연 통합:**

| 기존 Momentum | pad_dot으로의 통합 |
|---|---|
| negative_bias (부정→부정 증폭) | 부정 감정 PAD · 부정 자극 PAD → 양수 내적 → 증폭 |
| positive_bias (긍정→긍정 증폭) | 긍정 감정 PAD · 긍정 자극 PAD → 양수 내적 → 증폭 |
| anger_erosion (분노→분노 증폭) | Anger PAD · 분노 자극 PAD → 큰 양수 내적 → 강한 증폭 |
| sensitivity_boost (공포→민감) | Fear PAD · 부정 자극 PAD → P축 공명 |

공명(receptivity)이 별도 단계 없이 pad_dot 안에서 자연스럽게 발생한다.
감정별로 각각 pad_dot을 하니까, Anger가 쌓여있으면 Anger와 같은 방향의
자극에 더 크게 반응하는 효과가 자동으로 나온다.

#### 예시: 교룡 vs 무백

같은 자극: "도둑질이라 부를 수밖에" → PAD(-0.6, +0.7, +0.5)

```
교룡 (E=0, patience=-0.7, prudence=0):
  absorb = 1.0 + 0 - 0 - 0 = 1.0
  Anger alignment = pad_dot((-0.51,+0.59,+0.25), (-0.6,+0.7,+0.5)) = +0.844
  delta = 0.844 × 1.0 × 0.1 = +0.084
  Anger: 0.6 → 0.684

무백 (E=0.38, patience=0.8, prudence=0.8):
  absorb = 1.0 + 0.114 - 0.32 - 0.24 = 0.554
  Anger alignment = +0.844 (같은 값)
  delta = 0.844 × 0.554 × 0.1 = +0.047
  Anger: 0.3 → 0.347

교룡 delta(0.084) > 무백 delta(0.047) ✓
성격 차이 유지 ✓
```

반대 방향 자극 (사과):

```
자극: "대협, 제가 잘못했소" → PAD(+0.5, -0.3, -0.4)

교룡:
  alignment = pad_dot((-0.51,+0.59,+0.25), (+0.5,-0.3,-0.4)) = -0.532
  delta = -0.532 × 1.0 × 0.1 = -0.053
  Anger: 0.6 → 0.547 (감소)

무백:
  alignment = -0.532
  absorb: 부정 자극이 아니므로 patience 완충 미적용
  absorb = 1.0 + 0.114 - 0.24 = 0.874
  delta = -0.532 × 0.874 × 0.1 = -0.047
  Anger: 0.3 → 0.253 (감소)
```

#### 핵심 제약

- 새 감정을 생성하지 않음 — 기존 감정의 강도만 변동
- 감정이 0.05 이하로 떨어지면 제거 (자연 소멸)
- 새 감정이 필요한 상황 = 상황 전환 → `appraise()` 재호출
- `IMPACT_RATE`로 한 턴의 변동량 제어 (성격이 항상 지배적)

#### 엔지니어링 상수 (학술 근거 없음, 플레이테스트로 튜닝)

| 상수 | 값 | 역할 | 튜닝 방향 |
|------|---|------|---------|
| IMPACT_RATE | 0.1 | 턴당 변동량 제한 | 높이면 감정 변동 빨라짐 |
| FADE_THRESHOLD | 0.05 | 자연 소멸 기준 | 높이면 감정 빨리 사라짐 |
| E 민감도 계수 | 0.3 | E가 수용도에 미치는 비율 | 기존 EMOTIONALITY_AMP_FACTOR와 동일 |
| A 완충 계수 | 0.4 | patience가 부정 자극 완충에 미치는 비율 | 기존 PATIENCE_ANGER_FACTOR와 동일 |
| C 억제 계수 | 0.3 | prudence가 급변 억제에 미치는 비율 | 기존 PRUDENCE_IMPULSE_FACTOR와 동일 |
| 최소 수용도 | 0.1 | 완전 무시 방지 하한 | — |

**기존 대비 단순화 요약:**

| 기존 (v2 초안) | 단순화 (v2 확정) |
|---|---|
| emotion_state_to_pad() 전체 PAD 변환 | 삭제 — 감정별 직접 내적 |
| compute_receptivity() 별도 공명 계산 | 삭제 — pad_dot에 통합 |
| 코사인 유사도 (벡터 정규화) | 단순 내적 (크기 포함이 오히려 자연스러움) |
| stimulus_magnitude 별도 계산 | 삭제 — 내적에 이미 반영 |
| StimulusModifier 4필드 구조체 | 함수 하나, 덧셈/뺄셈 |
| 곱셈 체인 (result × a × b × c × d) | 덧셈/뺄셈 (rate += ... , rate -= ...) |
| X(외향성) 긍정 증폭 | 삭제 — appraise()에서 이미 적용 |
| 함수 5~6개, 100줄+ | 함수 3개, 30줄 미만 |

---

## AppraisalEngine 변경 사항

### 변경 전 (현재)

```rust
pub trait Appraiser {
    fn appraise(&self, personality: &HexacoProfile, situation: &Situation) -> EmotionState;
    fn appraise_with_context(&self, personality: &HexacoProfile, situation: &Situation,
                             current_state: &EmotionState) -> EmotionState;
}
```

- 상수 12개 (0.3, 0.4, 0.5 혼재 — 차이의 근거 없음)
- `appraise_with_context`가 매 턴 새 Situation을 받아 재평가
- EmotionalMomentum 4가지가 ad-hoc으로 감정 관성 처리
- 대사 자극과 Momentum이 이중 적용 위험
- 함수마다 다른 가중치 공식

### 변경 후 (v2)

#### 포트 정의

```rust
/// 상황 평가 포트 — 상황 진입 시 1회 사용
pub trait Appraiser {
    fn appraise(
        &self,
        personality: &HexacoProfile,
        situation: &Situation,
        relationship: &Relationship,
    ) -> EmotionState;
}

/// 대사 자극 처리 포트 — 대화 매 턴 사용
pub trait StimulusProcessor {
    fn apply_stimulus(
        &self,
        personality: &HexacoProfile,
        current_state: &EmotionState,
        stimulus: &PAD,
    ) -> EmotionState;
}

/// 대사 감정 분석 포트 — 플레이어 자유 입력 분석
pub trait UtteranceAnalyzer {
    fn analyze(&self, utterance: &str) -> PAD;
}
```

#### 상수 단순화: 12개 → 3개

```rust
impl AppraisalEngine {
    /// 성격이 감정 강도에 미치는 범용 계수
    const PERSONALITY_WEIGHT: f32 = 0.3;
    /// Fortune-of-others 기본 공감 강도
    const EMPATHY_BASE: f32 = 0.5;
    /// Fortune-of-others 발동 임계값 (H↓, A↓ 판정)
    const FORTUNE_THRESHOLD: f32 = -0.2;
}
```

기존 12개 상수가 대부분 0.3이었고, 0.4와 0.5의 차이에 학술적 근거가 없었음.
개별 facet의 값(-1.0~1.0) 자체가 이미 캐릭터 간 차이를 만들어서
상수까지 다를 필요 없음. 전부 플레이테스트로 튜닝할 값.

#### 가중치 패턴 통일

모든 가중치 계산이 하나의 패턴으로 통일:

```
증폭: 1.0 + facet_value × PERSONALITY_WEIGHT
억제: 1.0 - facet_value.max(0.0) × PERSONALITY_WEIGHT
```

#### appraise_event (단순화)

```rust
fn appraise_event(p: &HexacoProfile, state: &mut EmotionState,
    desirability_self: f32, desirability_other: Option<f32>,
    is_prospective: bool, prior: Option<PriorExpectation>,
) {
    let avg = p.dimension_averages();
    let w = Self::PERSONALITY_WEIGHT;

    // 공통 가중치 — 전부 같은 패턴
    let emotional_amp = 1.0 + avg.e.abs() * w;
    let positive_amp  = 1.0 + avg.x.max(0.0) * w;
    let negative_mod  = 1.0 - avg.a.max(0.0) * w;
    let impulse_mod   = 1.0 - p.conscientiousness.prudence.value().max(0.0) * w;

    // 1. prior_expectation → Satisfaction / Disappointment / Relief / FearsConfirmed
    if let Some(prior) = prior {
        let intensity = desirability_self.abs() * emotional_amp;
        match prior {
            HopeFulfilled  => state.add(Emotion::new(Satisfaction, intensity)),
            HopeUnfulfilled => state.add(Emotion::new(Disappointment, intensity)),
            FearUnrealized => state.add(Emotion::new(Relief, intensity)),
            FearConfirmed  => state.add(Emotion::new(FearsConfirmed, intensity)),
        }
        return;
    }

    // 2. prospect → Hope / Fear
    if is_prospective {
        if desirability_self > 0.0 {
            state.add(Emotion::new(Hope, desirability_self * positive_amp));
        } else {
            let fear_amp = 1.0 + p.emotionality.fearfulness.value().abs() * w;
            state.add(Emotion::new(Fear,
                desirability_self.abs() * emotional_amp * fear_amp));
        }
        return;
    }

    // 3. well-being → Joy / Distress
    if desirability_self > 0.0 {
        state.add(Emotion::new(Joy,
            desirability_self * emotional_amp * positive_amp));
    } else if desirability_self < 0.0 {
        state.add(Emotion::new(Distress,
            desirability_self.abs() * emotional_amp * negative_mod * impulse_mod));
    }

    // 4. fortune-of-others → HappyFor / Pity / Gloating / Resentment
    if let Some(other) = desirability_other {
        let t = Self::FORTUNE_THRESHOLD;
        let h = avg.h;
        let a = avg.a;
        if other > 0.0 {
            if h > 0.0 || a > 0.0 {
                let empathy = (h.max(0.0) + a.max(0.0)) / 2.0;
                state.add(Emotion::new(HappyFor,
                    other * (Self::EMPATHY_BASE + empathy * Self::EMPATHY_BASE)));
            }
            if h < t {
                state.add(Emotion::new(Resentment, other * h.abs() * negative_mod));
            }
        } else {
            let abs = other.abs();
            if a > 0.0 || p.emotionality.sentimentality.value() > 0.0 {
                let compassion = (a.max(0.0)
                    + p.emotionality.sentimentality.value().max(0.0)) / 2.0;
                state.add(Emotion::new(Pity,
                    abs * (Self::EMPATHY_BASE + compassion * Self::EMPATHY_BASE)));
            }
            if h < t && a < t {
                state.add(Emotion::new(Gloating, abs * (h.abs() + a.abs()) / 2.0));
            }
        }
    }
}
```

#### appraise_action (단순화)

```rust
fn appraise_action(p: &HexacoProfile, state: &mut EmotionState,
    is_self_agent: bool, praiseworthiness: f32, outcome_for_self: Option<f32>,
) {
    let avg = p.dimension_averages();
    let w = Self::PERSONALITY_WEIGHT;
    let standards_amp = 1.0 + avg.c.abs() * w;

    if is_self_agent {
        if praiseworthiness > 0.0 {
            let pride_mod = 1.0 - p.honesty_humility.modesty.value().max(0.0) * w;
            state.add(Emotion::new(Pride,
                praiseworthiness * standards_amp * pride_mod));
        } else {
            state.add(Emotion::new(Shame,
                praiseworthiness.abs() * standards_amp));
        }
    } else {
        if praiseworthiness > 0.0 {
            state.add(Emotion::new(Admiration,
                praiseworthiness * standards_amp));
        } else {
            let reproach_mod = 1.0 - p.agreeableness.gentleness.value().max(0.0) * w;
            state.add(Emotion::new(Reproach,
                praiseworthiness.abs() * standards_amp * reproach_mod));
        }
    }

    // compound 감정
    if let Some(outcome) = outcome_for_self {
        if is_self_agent {
            if praiseworthiness > 0.0 && outcome > 0.0 {
                state.add(Emotion::new(Gratification,
                    (praiseworthiness + outcome) / 2.0 * standards_amp));
            } else if praiseworthiness < 0.0 && outcome < 0.0 {
                state.add(Emotion::new(Remorse,
                    (praiseworthiness.abs() + outcome.abs()) / 2.0 * standards_amp));
            }
        } else {
            if praiseworthiness > 0.0 && outcome > 0.0 {
                let gratitude_amp = 1.0
                    + p.honesty_humility.sincerity.value().max(0.0) * w;
                state.add(Emotion::new(Gratitude,
                    (praiseworthiness + outcome) / 2.0 * gratitude_amp));
            } else if praiseworthiness < 0.0 && outcome < 0.0 {
                let anger_mod = 1.0 - p.agreeableness.patience.value() * w;
                state.add(Emotion::new(Anger,
                    (praiseworthiness.abs() + outcome.abs()) / 2.0 * anger_mod));
            }
        }
    }
}
```

#### appraise_object (변경 없음)

```rust
fn appraise_object(p: &HexacoProfile, state: &mut EmotionState,
    appealingness: f32,
) {
    let w = Self::PERSONALITY_WEIGHT;
    let aesthetic_amp = 1.0 + p.openness.aesthetic_appreciation.value().abs() * w;

    if appealingness > 0.0 {
        state.add(Emotion::new(Love, appealingness * aesthetic_amp));
    } else if appealingness < 0.0 {
        state.add(Emotion::new(Hate, appealingness.abs() * aesthetic_amp));
    }
}
```

### 삭제 대상

| 항목 | 이유 |
|------|------|
| `appraise_with_context()` | `apply_stimulus()`로 대체 |
| `EmotionalMomentum` 구조체 | `pad_dot()` 단순 내적으로 통합 |
| `EmotionalMomentum::from_state()` | 삭제 — 별도 공명 계산 불필요 |
| 4가지 momentum 계수 | `pad_dot()` 하나로 자연 통합 |
| 12개 가중치 상수 | `PERSONALITY_WEIGHT` 1개로 통합 |
| Momentum 파라미터 전달 | 내부 함수에서 `m` 파라미터 제거 |

### 유지 대상

| 항목 | 이유 |
|------|------|
| `appraise()` | 상황 진입 시 1회 평가. Relationship 파라미터 추가 |
| `appraise_event/action/object` 내부 함수 | OCC 분기 로직 유지, Momentum 제거 + 상수 통합 |
| OCC 22개 감정 분기 규칙 | 변경 없음 |

### appraise()에 Relationship 추가

```
변경 전:
  감정 강도 = Situation 값 × HEXACO 가중치

변경 후:
  감정 강도 = Situation 값 × HEXACO 가중치 × Relationship 가중치

Relationship 가중치:
  closeness → 전반적 감정 배율
  trust     → 기대 위반도에 따른 증폭/완화
  power     → Pride/Shame에 제한적 영향
```

### 단순화 요약

| 항목 | 기존 | v2 |
|------|------|-----|
| 상수 | 12개 (0.3, 0.4, 0.5 혼재) | 3개 (PERSONALITY_WEIGHT, EMPATHY_BASE, FORTUNE_THRESHOLD) |
| 가중치 패턴 | 함수마다 다른 공식 | `1.0 ± facet × 0.3` 통일 |
| Momentum | 4계수, 모든 함수에 전달 | 삭제 |
| 내부 함수 시그니처 | (personality, state, momentum, ...) | (personality, state, ...) |

---

## ActingGuide 변경 사항

ActingGuide에 Relationship 정보를 포함하여 LLM이 관계에 맞는 대사를 생성할 수 있도록 한다.

```rust
pub struct ActingGuide {
    pub npc_name: String,
    pub npc_description: String,
    pub personality: PersonalitySnapshot,
    pub emotion: EmotionSnapshot,
    pub directive: ActingDirective,
    pub situation_description: Option<String>,
    pub relationship: Option<RelationshipSnapshot>,  // 신규
}

pub struct RelationshipSnapshot {
    /// 상대방 이름/ID
    pub target_name: String,
    /// 친밀도 라벨 ("의형제", "모르는 사람", "숙적" 등)
    pub closeness_label: String,
    /// 신뢰도 라벨 ("전적으로 믿음", "경계함" 등)
    pub trust_label: String,
    /// 상하 관계 라벨 ("사부", "대등한 동료", "제자" 등)
    pub power_label: String,
}
```

---

## 감정 분류 도구 구성

### 도구별 역할

| 도구 | 역할 | 적용 대상 |
|------|------|----------|
| fastembed-rs (bge-m3) | PAD 앵커 임베딩 유사도 계산 | 플레이어 자유 입력 |
| bge-reranker-v2-m3 | (PAD 추출에는 미사용, 향후 확장용) | — |
| ahocorasick_rs | 감정 키워드 빠른 탐지 (힌트) | 모든 텍스트 입력 |

### 파이프라인

```
플레이어 대사
  │
  ├─ [1단] Aho-Corasick: 키워드 신호 탐지
  │   "도둑질" → 비난 힌트
  │   "은혜" → 감사 힌트
  │
  ├─ [2단] bge-m3: 임베딩 → 6개 앵커와 유사도
  │   sim(대사, P+) = 0.2, sim(대사, P-) = 0.7 → P ≈ -0.6
  │   sim(대사, A+) = 0.6, sim(대사, A-) = 0.2 → A ≈ +0.5
  │   sim(대사, D+) = 0.6, sim(대사, D-) = 0.2 → D ≈ +0.5
  │
  └─ → PAD(-0.6, +0.5, +0.5)
```

### 사전 준비

- PAD 앵커 텍스트 세트 정의 (3축 × 2극 × N개 변형)
- bge-m3로 앵커 임베딩 사전 계산 및 캐싱
- Aho-Corasick 키워드 사전 구축 (무협 도메인 특화)

---

## 데이터 흐름 요약

### 상황 진입 시

```
게임 시스템
  ├─ Situation 생성 (세계관 기준)
  ├─ HEXACO 조회 (NPC 고정 성격)
  └─ Relationship 조회 (상대별 관계)
        │
        ▼
  AppraisalEngine.appraise(personality, situation, relationship)
        │
        ▼
  초기 EmotionState
        │
        ▼
  ActingGuide 생성 → LLM → NPC 첫 반응 대사
```

### 대화 중 (매 턴)

```
상대방 대사 (플레이어 자유 입력)
        │
        ▼
  UtteranceAnalyzer.analyze(대사) → PAD
        │
        ▼
  StimulusProcessor.apply_stimulus(personality, current_state, PAD)
        │
        ▼
  갱신된 EmotionState
        │
        ▼
  ActingGuide 생성 (+Relationship) → LLM → NPC 응답 대사
```

### 대화 종료 후

```
  최종 EmotionState + Situation
        │
        ▼
  Relationship 갱신
    trust: Action의 praiseworthiness 기반 (점진적)
    closeness: 대화 감정 결과 기반 (매우 점진적)
    power: 변경 없음 (서사 이벤트에서만 변경)
```

---

## 테스트 전략 변경

### 기존 테스트 (유지, 수정 필요)

| 테스트 | 변경 |
|--------|------|
| 배신 시나리오 (무백/교룡 비교) | Relationship 파라미터 추가 |
| 적 대군 시나리오 (Fear 강도) | 변경 없음 |
| 라이벌 승진 시나리오 | closeness에 따른 HappyFor/Resentment 분기 테스트 추가 |
| 해독약 실패 시나리오 | 변경 없음 |

### 기존 테스트 (삭제/대체)

| 테스트 | 이유 |
|--------|------|
| 대화_교룡_3턴_감정_누적 | appraise_with_context 삭제에 따라 apply_stimulus 기반으로 재작성 |
| 대화_무백은_누적되어도_절제 | 위와 동일 |
| 대화_긍정_감정_누적 | 위와 동일 |

### 신규 테스트

| 테스트 | 검증 내용 |
|--------|----------|
| Relationship closeness 배율 | 같은 상황, closeness 차이 → 감정 강도 차이 |
| Relationship trust 기대 위반 | trust 높은데 배신 → 감정 증폭 |
| PAD 앵커 추출 정확도 | 프로토타입 대사 → PAD 값 검증 |
| Receptivity 공명 | 같은 방향 PAD 자극 → 감정 증폭 |
| Receptivity 반대 방향 | 반대 PAD 자극 → 감정 완화 |
| HEXACO StimulusModifier | 무백(patience↑) vs 교룡(patience↓) → 같은 자극 다른 변동량 |
| emotion_state_to_pad 변환 | OCC 감정 조합 → PAD 가중 평균 정확성 |
| 감정 자연 소멸 | 반대 자극 반복 → 감정 강도 0.05 이하 → 제거 |
| 대화 후 Relationship 갱신 | 부정 대화 → closeness/trust 하락 |
| 상황 전환 (향후) | 새 Situation → 새 EmotionState — 이번 개발 범위 밖 |

---

## 구현 순서 (사이클)

### 완료

| 사이클 | 내용 | 상태 |
|--------|------|------|
| 1 | HEXACO 성격 모델 | ✅ 완료 |
| 2 | OCC 감정 모델 + AppraisalEngine | ✅ 완료 → 사이클 5에서 상수 단순화 |
| 2.5 | 대화 중 감정 변화 (EmotionalMomentum) | ✅ 완료 → 사이클 7에서 삭제 |
| 3 | LLM 연기 가이드 (ActingGuide, 다국어 포맷터) | ✅ 완료 |

### 예정

| 사이클 | 내용 | 산출물 | 의존성 |
|--------|------|--------|--------|
| 4 | Relationship 도메인 모델 | Relationship 구조체, 기본 테스트 | 없음 |
| 5 | AppraisalEngine 단순화 + Relationship 통합 | 상수 12→3, Momentum 제거, appraise()에 relationship 추가, 기존 테스트 수정 | 사이클 4 |
| 6 | PAD 도메인 모델 | PAD 구조체, OCC→PAD 매핑 테이블 (EmotionType::to_pad()) | 없음 |
| 7 | apply_stimulus 구현 + 2.5 삭제 | pad_dot, stimulus_absorb_rate, appraise_with_context 삭제, 기존 감정 누적 테스트 3개 재작성 | 사이클 6 |
| 8 | ActingGuide에 Relationship 포함 | RelationshipSnapshot, power 기반 톤 결정 | 사이클 4 |
| 9 | 대화 후 Relationship 갱신 메커니즘 | update_after_dialogue() | 사이클 4, 7 |
| 10 | PAD 앵커 임베딩 파이프라인 (fastembed-rs + bge-m3) | UtteranceAnalyzer 구현, 앵커 세트 | 사이클 6 |
| 11 | Aho-Corasick 키워드 사전 + PAD 힌트 통합 | 무협 도메인 키워드 사전, 힌트 가산 | 사이클 10 |

### 사이클 5 상세: AppraisalEngine 단순화 + Relationship 통합

기존 코드의 복잡성을 줄이면서 Relationship을 추가하는 사이클.

**변경:**
- 상수 12개 → `PERSONALITY_WEIGHT(0.3)`, `EMPATHY_BASE(0.5)`, `FORTUNE_THRESHOLD(-0.2)` 3개
- 가중치 패턴 통일: `1.0 ± facet × PERSONALITY_WEIGHT`
- 내부 함수에서 `&EmotionalMomentum` 파라미터 제거
- `appraise()` 시그니처에 `&Relationship` 추가

**기존 테스트 영향:**
- 상수 변경으로 일부 감정 강도 값이 소폭 달라짐 (fearfulness 0.5→0.3 등)
- assert 값 조정 필요하나 **비교 관계는 유지** (교룡 > 무백 등)

### 사이클 7 상세: apply_stimulus 구현 + 2.5 삭제

사이클 2.5에서 만든 EmotionalMomentum을 pad_dot으로 교체하는 사이클.

**추가:**
- `StimulusProcessor` 트레이트 + 구현체
- `pad_dot()` — 단순 내적, 3줄
- `stimulus_absorb_rate()` — HEXACO 수용도, 7줄
- `apply_stimulus()` — 루프 + delta 적용, 15줄

**삭제:**
- `EmotionalMomentum` 구조체
- `EmotionalMomentum::from_state()`
- `AppraisalEngine::appraise_with_context()`
- `Appraiser` 트레이트의 `appraise_with_context()` 메서드

**테스트 재작성:**

```rust
// 기존 (삭제): 매 턴 새 Situation
let state2 = AppraisalEngine::appraise_with_context(
    yu.personality(), &turn2, &state1);

// 신규: 고정 Situation, PAD 자극
let state = AppraisalEngine::appraise(
    yu.personality(), &situation, &relationship);
let stimulus = PAD { pleasure: -0.6, arousal: 0.7, dominance: 0.5 };
let state1 = processor.apply_stimulus(
    yu.personality(), &state, &stimulus);
```

| 기존 테스트 | 재작성 | 검증 포인트 (동일 유지) |
|------------|--------|----------------------|
| 대화_교룡_3턴_감정_누적 | PAD 자극 3회 연속 | 부정 자극 반복 → pad_dot 공명 → 분노 증폭 |
| 대화_무백은_누적되어도_절제 | 무백 vs 교룡 PAD 자극 비교 | patience↑ → absorb_rate 낮음 → 변동 작음 |
| 대화_긍정_감정_누적 | 긍정 PAD 자극 반복 | 긍정 감정 PAD · 긍정 자극 PAD → 양수 내적 → Joy 증폭 |

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|-----------|
| 0.1.0 | 2026-03-24 | 초기 작성. 4레이어 아키텍처, Relationship 3축, PAD 자극 모델, Momentum→Receptivity 통합, 감정 분류 도구 파이프라인, 구현 순서 |
| 0.2.0 | 2026-03-24 | apply_stimulus 5단계 상세 설계 추가. OCC→PAD 매핑 테이블, StimulusModifier(HEXACO 자극 수용 조절), 감정별 alignment 변동량 공식, 무백/교룡 계산 예시. 개발 범위 확정: 대화 중 Situation 고정, 기존 감정 강도 변동만 (새 감정 생성 없음) |
| 0.3.0 | 2026-03-24 | 구현 순서 재편. 사이클 9(Momentum 리팩터링)를 사이클 7(apply_stimulus)에 통합. 사이클 7 상세 계획 추가: 삭제 대상, 테스트 재작성 방법, 검증 포인트 |
| 0.4.0 | 2026-03-24 | apply_stimulus 단순화. emotion_state_to_pad/compute_receptivity/코사인유사도/StimulusModifier 구조체/X긍정증폭 삭제. pad_dot(단순 내적)+stimulus_absorb_rate(덧셈뺄셈) 2함수로 통합. 함수 3개 30줄 미만 |
| 0.5.0 | 2026-03-24 | AppraisalEngine 단순화. 상수 12개→3개(PERSONALITY_WEIGHT, EMPATHY_BASE, FORTUNE_THRESHOLD). 가중치 패턴 통일(1.0 ± facet × 0.3). Momentum 파라미터 제거. appraise_event/action/object 코드 수준 설계 포함 |
