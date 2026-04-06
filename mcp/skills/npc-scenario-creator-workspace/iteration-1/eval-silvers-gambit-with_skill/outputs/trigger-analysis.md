# Trigger 분석 상세 보고서

## 개요

Silver의 도박 시나리오에서 Beat 전환 메커니즘의 상세 분석. 특히 SKILL.md의 "4가지 체크리스트"를 중심으로 각 트리거의 타당성을 검증한다.

---

## Beat 1 → Beat 2 전환 분석

### 현재 Trigger 정의

```json
"trigger": [
  [
    {"below": 0.4, "emotion": "Distress"},
    {"above": 0.2, "emotion": "Hope"}
  ]
]
```

**의미**: Distress < 0.4 AND Hope > 0.2일 때 Beat 2로 전환

---

### Skill의 4가지 체크리스트

#### 1️⃣ **이 Beat에서 태어나야 할 감정은 무엇인가?**

Beat 2 ("Impressed")에서 Silver가 경험할 감정들:

| 감정 | 강도 | 근거 |
|------|------|------|
| **Admiration** | 높음 | Jim의 용감하고 영리한 자백 (praiseworthiness=0.85) |
| **Joy** | 중간 | Event desirability 0.75 → 긍정 사건 |
| **Pride** | 높음 | Jim을 길러낸 자신의 영향력 재확인 |
| **Relief** | 중간~높음 | 배를 잃었지만 새로운 동맹자 획득 |
| **Gratification** | 높음 | Pride + Joy의 복합 감정 |

**핵심 감정**: **Admiration** + **Pride** + **Joy**

---

#### 2️⃣ **그 감정이 피어나려면 이전 Beat의 어떤 감정이 어떻게 변해야 하는가?**

**Beat 1의 상태**:
```
Pride: 0.609 (높음)
Joy: 0.332 (중간)
Gratification: 0.470 (중간~높음)
Distress: 0.0 (없음) ⚠️
Hope: ? (초기값 불명)
```

**Beat 2 진입을 위해 필요한 변화**:

1. **Distress 증가 → 감소**
   - 배를 잃은 좌절감(Distress)이 Jim의 대담함으로 인해 완화
   - 현재 문제: Beat 1에서 Distress가 생성되지 않음 → 감소시킬 것이 없음

2. **Hope 증가**
   - Jim의 용감한 자백 = "새로운 가능성"의 신호
   - Hope 감정이 0에서 0.2 이상으로 상승

3. **Admiration 신규 생성**
   - Beat 1: 없음
   - Beat 2: Jim의 행동 (agent_id=jim, praiseworthiness=0.85) → Admiration 신규 생성

**요약**:
```
Beat 1: Distress ↑, Hope ↓, Pride/Joy ↑
                  ↓
Stimulus (Jim의 대담한 고백)
                  ↓
Beat 2: Distress ↓, Hope ↑, Admiration ↑, Pride/Joy 유지
```

---

#### 3️⃣ **그 변화가 stimulus(PAD 자극)만으로 도달 가능한가?**

### Stimulus 메커니즘 이해

**정의**: `apply_stimulus()`는 기존 감정의 **강도만 조절**할 수 있고, **새로운 감정을 생성하지 못함**

**공식** (src/domain/tuning.rs 참조):
```
inertia = max(1.0 - intensity, STIMULUS_MIN_INERTIA)
delta = pad_dot × absorb_rate × STIMULUS_IMPACT_RATE × inertia
result_intensity = clamp(intensity + delta)
```

**파라미터**:
- `STIMULUS_IMPACT_RATE = 0.5` — 자극 영향도
- `STIMULUS_MIN_INERTIA = 0.30` — 최소 관성
- 강한 감정(intensity↑) → inertia↓ → 변동 커짐
- 약한 감정(intensity↓) → inertia↑ → 변동 작음

### 적용 분석

**Beat 1 → Beat 2 Stimulus 시나리오**:

Jim의 대담한 고백:
```
Utterance: "나는 사과통에서 음모를 들었고, 스쿠너를 훔쳤으며,
           Black Dog를 알고 있다. 죽이든 살리든 마음대로 하라."

PAD 분석 (예상):
P(Pleasure): 0.6 → Admiration 신호
A(Arousal): 0.8 → 높은 자극
D(Dominance): 0.5 → 상대 존중

Silver의 현재 감정:
Pride(0.609) — high intensity → low inertia → 변동 민감
Hope(0.0) — zero intensity → 기존값이 없으므로 stimulus 불가 ❌

Joy(0.332) — moderate intensity → moderate inertia → 약간의 변동

Distress(0.0) — 존재하지 않음 → stimulus 불가 ❌
```

**결론**:
- ✅ **Admiration은 신규 생성 가능** (appraise에서 event/action 평가로 생성)
- ✅ **Pride 변동은 가능** (existing, stimulus로 조절 가능)
- ✅ **Joy 변동은 가능** (existing, stimulus로 조절 가능)
- ❌ **Distress 변동은 불가능** (Beat 1에서 생성되지 않음)
- ❌ **Hope 증가는 불가능** (Beat 1에서 존재하지 않음)

**문제점**: 트리거에서 "Distress < 0.4" 조건이 무의미함. 0은 < 0.4를 만족하므로 즉시 전환될 가능성.

---

#### 4️⃣ **이전 Beat의 appraise에서 참조 대상 감정이 실제로 생성되는가?**

### Beat 1 appraise 검증

실제 호출 결과:
```json
{
  "emotions": [
    {"emotion_type": "Joy", "intensity": 0.3315},
    {"emotion_type": "Pride", "intensity": 0.6087},
    {"emotion_type": "Gratification", "intensity": 0.4701}
  ]
}
```

**트리거가 참조하는 감정**:
- ✅ **Distress**: 생성 안 됨 (0 수치로 처리)
- ❓ **Hope**: 생성 안 됨 (0 수치로 처리)

### 문제 진단

**상황 1: 즉시 전환 위험**
```
Beat 1의 Distress = 0
Trigger: Distress < 0.4?
결과: true → 즉시 Beat 2로 전환 가능성 ⚠️
```

Jim의 고백이 없어도 Beat 2로 넘어갈 수 있다는 뜻.

**상황 2: Hope 참조 불가**
```
Beat 1의 Hope = 생성 안 됨 (undefined)
Trigger: Hope > 0.2?
결과: false → 첫 turn에서 거의 전환 안 됨
```

Hope가 생성되는 event가 Beat 1에 없으므로, Jim의 stimulus에서도 Hope 생성이 어려움.

---

## 개선 방안

### 방안 A: Beat 1 Event 재설계 (추천)

**문제의 근본 원인**: Event desirability 0.3이 너무 긍정적

**개선**:
```json
// Before
"event": {
  "desirability_for_self": 0.3,
  "description": "..."
}

// After
"event": {
  "desirability_for_self": -0.2,  // 배 상실의 좌절 강조
  "description": "배를 완전히 잃었다. 요새와 식량을 확보했지만,
                  대규모 원정의 꿈은 무너졌다. Jim의 출현은 기회일 수도,
                  시간 낭비일 수도 있다. 아직 결정되지 않은 상황."
}
```

**예상 결과**:
```
new appraise:
- Joy(↓) : 0.3 → 약 0.1 (배 상실 강조)
- Distress(↑) : new → 약 0.4~0.5 (좌절감 생성)
- Pride(유지) : 약 0.6 (여전히 통제 중)

Trigger "Distress < 0.4": 이제 의미 있음
- Beat 초기: Distress = 0.4~0.5 (조건 거짓)
- Stimulus 후: Distress = 0.1~0.2 (조건 참) → Beat 2 전환
```

### 방안 B: Trigger 조건 단순화

**대안**:
```json
// Before (현재)
"trigger": [
  [
    {"below": 0.4, "emotion": "Distress"},
    {"above": 0.2, "emotion": "Hope"}
  ]
]

// After (단순화)
"trigger": [
  [
    {"above": 0.5, "emotion": "Admiration"}
  ]
]
```

**장점**: Jim의 행동(praiseworthiness=0.85)에서 Admiration이 자동 생성 → 조건 단순
**단점**: Distress 감소 과정을 의식적으로 표현하지 못함

---

## Beat 2 → Beat 3 전환 분석

### 현재 Trigger 정의

```json
"trigger": [
  [
    {"above": 0.6, "emotion": "Admiration"},
    {"below": 0.3, "emotion": "Fear"}
  ],
  [
    {"above": 0.5, "emotion": "Joy"},
    {"above": 0.4, "emotion": "Pride"}
  ]
]
```

**의미**:
- **경로 1**: Admiration > 0.6 AND Fear < 0.3
- **경로 2**: Joy > 0.5 AND Pride > 0.4

---

### 4가지 체크리스트 검증

#### 1️⃣ **이 Beat에서 태어나야 할 감정은 무엇인가?**

Beat 3에서 Silver가 경험할 감정:

| 감정 | 강도 | 근거 |
|------|------|------|
| **Anger** | 매우 높음 | Morgan의 도전 (praiseworthiness=-0.95) |
| **Reproach** | 매우 높음 | 명령 불복종, 반역 행위 |
| **Pride** | 높음 | 리더십 권위 재확립 ("누가 감히") |
| **Fear** | 낮음~없음 | Silver의 fearfulness=-0.5, 대담함 유지 |

**핵심 감정**: **Anger** + **Reproach** + **Pride 강화**

---

#### 2️⃣ **이전 Beat의 어떤 감정이 변해야 하는가?**

**Beat 2의 최종 상태**:
```
Admiration: 높음 (Jim의 praiseworthiness=0.85)
Joy: 중간~높음 (Event desirability=0.75)
Pride: 높음~매우 높음 (자신의 영향력 확인)
Fear: 낮음 (Silver는 원래 fearfulness=-0.5)
```

**Beat 3 진입 필요 변화**:

1. **Admiration 또는 Pride 높은 상태 유지**
   - Morgan의 도전 자체가 Silver의 자부심을 건드림
   - 높은 감정 상태에서 도전 → 분노로 전환

2. **Fear 낮음 유지**
   - Silver의 성격상 당연히 낮음
   - 도전해도 두려워하지 않음

3. **Anger 신규 생성**
   - Morgan의 행동 (agent_id=morgan, praiseworthiness=-0.95)
   - Anger/Reproach 신규 생성

**요약**:
```
Beat 2 말기: Admiration↑, Joy↑, Pride↑, Fear↓
              ↓
Stimulus (Morgan의 칼과 반역)
              ↓
Beat 3: Anger↑, Reproach↑, Pride↑(권위 강조), Fear↓(유지)
```

---

#### 3️⃣ **그 변화가 stimulus만으로 가능한가?**

**Morgan의 칼 행동 시뮬레이션**:

```
Utterance: "Then here goes! [칼을 뽑는다]"

PAD 분석 (예상):
P(Pleasure): -0.8 → 불쾌, 위협
A(Arousal): 0.95 → 매우 높은 자극
D(Dominance): -0.8 → Silver의 우월성 도전

Silver의 현재 감정:
Admiration(높음) — high intensity → low inertia
   + PAD: P음수, D음수 → Admiration ↓, Anger ↑
   + Inertia 작으므로 급변 가능

Pride(높음) — high intensity → low inertia
   + PAD: P음수, D음수 → Pride ↓ 또는 유지 (자존심으로 방어)
   + Inertia 작으므로 급변 가능

Joy(중간) — moderate intensity
   + PAD: P음수 → Joy 급감

Fear(낮음) — low intensity → high inertia
   + Silver는 fearfulness=-0.5 (낮음)
   + 높은 inertia로 인해 자극 후에도 변동 미미
   + Fear 낮음 유지 가능
```

**결론**:
- ✅ **Anger는 신규 생성 가능** (Morgan의 행동 평가에서)
- ✅ **Admiration/Pride는 stimulus로 조절 가능** (높은 강도 → 변동성 높음)
- ✅ **Fear는 낮음 유지 가능** (원래 낮고, inertia 높음)

**타당성**: ✅ stimulus만으로 충분히 가능

---

#### 4️⃣ **참조 감정이 실제로 생성되는가?**

**Beat 2 appraise 예상 결과**:
```json
{
  "emotions": [
    {"emotion_type": "Admiration", "intensity": 0.7+},
    {"emotion_type": "Joy", "intensity": 0.5+},
    {"emotion_type": "Pride", "intensity": 0.7+}
  ]
}
```

**Trigger 경로 1 확인**:
```
Condition: Admiration > 0.6 AND Fear < 0.3
Beat 2 예상: Admiration = 0.7+, Fear = 0~ (원래 낮음)
결과: ✅ 조건 충족 가능
```

**Trigger 경로 2 확인**:
```
Condition: Joy > 0.5 AND Pride > 0.4
Beat 2 예상: Joy = 0.5+, Pride = 0.7+
결과: ✅ 조건 충족 가능
```

---

## 결론: Trigger 타당성 평가

### Beat 1→2 평가: ⚠️ 부분적 개선 필요

| 항목 | 평가 | 비고 |
|------|------|------|
| 감정 종류 | ✅ 적절 | Admiration, Joy, Pride |
| 이전 Beat 변화 | ⚠️ 문제 | Distress 미생성 (Beat 1 설정 개선 필요) |
| Stimulus 가능성 | ⚠️ 부분적 | Distress/Hope 기준값 부재 → trigger 무의미화 |
| 참조 감정 생성 | ❌ 실패 | Distress, Hope 미생성 |

**개선 우선순위**:
1. **높음**: Beat 1 Event desirability를 -0.2로 조정 → Distress 생성 강화
2. **중간**: Hope 생성 메커니즘 재검토 (별도 action 추가 또는 event 수정)
3. **낮음**: Trigger 조건 단순화 (Admiration만으로도 충분할 수 있음)

### Beat 2→3 평가: ✅ 우수

| 항목 | 평가 | 비고 |
|------|------|------|
| 감정 종류 | ✅ 적절 | Anger, Reproach, Pride |
| 이전 Beat 변화 | ✅ 적절 | Admiration/Pride 고조 후 stimulus 수신 |
| Stimulus 가능성 | ✅ 가능 | 높은 강도 감정 → 민감한 반응 |
| 참조 감정 생성 | ✅ 확인 | Admiration, Joy, Pride 모두 예상됨 |
| 다중 경로 | ✅ 우수 | 두 가지 심리 메커니즘 모두 타당 |

---

## 최종 권고

### 즉시 실행

```json
// src/data/treasure_island/ch28_silvers_gambit/실버의도박.json 수정
"focuses": [
  {
    "id": "calculating",
    "event": {
      "desirability_for_self": -0.2,  // 0.3에서 -0.2로 수정
      "description": "배를 완전히 잃었다. 요새와 식량을 확보했지만,
                      대규모 원정의 꿈은 무너졌다. Jim의 출현은 새로운
                      변수다 — 기회인지 방해인지 아직 모른다."
    },
    ...
  }
]
```

### 검증 절차

1. **첫 번째 turn**: Jim의 "용감한 고백" → `apply_stimulus()`
   - 예상 결과: Distress ↓, Admiration ↑ → Beat 2 전환 확인

2. **두 번째 turn**: Morgan의 "칼 반란" → `apply_stimulus()`
   - 예상 결과: Anger ↑, Admiration/Pride 조정 → Beat 3 전환 확인

3. **각 Beat의 guide 품질**: `generate_guide()` 확인
   - Silver의 프롬프트가 원작 장면과 부합하는지 검증
