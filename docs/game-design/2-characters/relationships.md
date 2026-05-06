# 관계 시스템 (Relationships)

> Version: 0.4 — 2026-05-04
> 위치: `docs/game-design/2-characters/relationships.md`
> 의존: `_schema.md` v0.4, `00-pillars.md` v0.1
> 참조: `npc-mind-rs` OCC/PAD 엔진

## 0. 설계 원칙

이 문서는 Pillar 3 ("관계가 곧 시스템")의 *본체*다.

세 명제가 모든 디자인 결정의 시금석:

1. **호감/적대 이분법은 거짓이다.** 한 사람을 신뢰하면서도 두려워할 수 있고, 존경하면서도 함께 있고 싶지 않을 수 있다.
2. **관계는 *상태*가 아니라 *형태*다.** "끊어졌다"는 없다. 모든 만남은 어떤 *형태*로든 존재한다 — 적, 미련, 잊혀진 자, 묻힌 형제까지.
3. **관계는 NPC가 *스스로* 갱신한다.** 디자이너가 스크립트로 박지 않는다. NPC의 OCC 감정이 관계를 만들고, 관계가 다시 다음 감정의 강도를 결정한다.

---

## 1. 4축 — Trust · Affinity · Respect · Wariness

### 1.1 각 축의 정의

| 축 | 측정 대상 | 한 줄 질문 | 범위 |
|---|---|---|---|
| **trust** (신뢰) | 이 사람의 *말과 행동의 일치도* — 약속·예측가능성 | "등을 보일 수 있는가?" | -100 ~ +100 |
| **affinity** (친밀) | 이 사람과 함께 있을 때의 *정서적 거리* | "혼자 있을 때 그리운가?" | -100 ~ +100 |
| **respect** (존경) | 이 사람의 *판단·능력·인격*에 대한 상하 평가 | "이 사람의 의견을 따를 만한가?" | -100 ~ +100 |
| **wariness** (경계) | 이 사람이 *가할 수 있는 위협*의 인식 | "무방비로 마주할 수 있는가?" | 0 ~ +100 |

각 축의 한 줄 질문은 **인물 작성자의 멘탈 모델**이다. 새 인물 인스턴스의 axes를 채울 때, 디자이너는 4개의 질문에 답한 뒤 수치로 변환한다.

### 1.2 직교성 — 핵심 디자인 결정

**4축은 *직교*한다.** 한 축이 다른 축을 자동으로 결정하지 않는다.

직교성이 만드는 *흥미로운 패턴*들:

| 패턴 | 의미 | 무협·문학 사례 |
|---|---|---|
| trust↑ + wariness↑ | "예측 가능하지만 위험한 사람" — 합리적 적, 야심가 | 연청에게 송강 (귀순 정책은 따르나, 토사구팽 예감) |
| respect↑ + affinity↓ | "위대하지만 가까이하기 어려움" — 외로운 거인 | 와호장룡 이모백을 보는 옥교룡 (초기) |
| affinity↑ + trust↓ | "그리우나 의지할 수 없음" — 미련의 형태 | 펑쩌에 대한 손유탕 |
| trust↑ + affinity↑ + respect↓ | "내 사람이지만 의지할 수 없음" — 보호 본능 | 자식을 보는 부모 |
| **respect↑ + affinity↓ + wariness↑** | "적이지만 인정함" — 숙적 | 와호장룡 푸른여우 ↔ 수련 |
| trust↑ + respect↑ + affinity↑ + wariness↑(낮지 않음) | "지기이나 그도 인간이다" — 한계 인식 | **연청에게 노준의 (95/90/90/30)** |

직교성은 *이분법의 거짓*을 시스템적으로 구현한다. 단일 축의 "호감도"로는 위 패턴 중 어느 것도 표현되지 않는다.

### 1.3 음수의 의미

음수는 단순히 "낮음"이 아니라 *적극적 반대 인식*이다.

- **trust = 0**: 이 사람을 모름. 예측 불가.
- **trust = -50**: *확신을 가지고* 의심함. "이 사람의 말은 거짓이다."
- **trust = -100**: 모든 말이 함정이라 학습됨. 임충에게 산신묘 후의 육겸.

마찬가지로:
- **affinity 음수** = 혐오. 함께 있는 게 고통.
- **respect 음수** = 경멸. 이 사람을 *아래*로 봄. 임충이 왕륜을 본 시각.
- **wariness는 음수 없음.** 0이 이미 "완전 무방비"의 극값. 위협 인식의 *역방향*은 정의되지 않음.

### 1.4 갱신 빈도와 점착성

- **즉각 갱신**: 한 사건당 ±1~30. OCC 감정 기반 (§4).
- **주기 감쇠**: 큰 변화는 시간이 흐르며 평균값으로 *수렴*. 단 `transformation_event`로 기록된 변화는 감쇠하지 않음.
- **양극의 점착성**: ±100에 도달하면 추가 입력에도 머무름. 한 번 *완전한* 신뢰/불신/혐오/경멸에 도달한 관계는 다시 일상으로 돌아가지 않는다.

---

## 2. 관계의 형태 — type과 type_history

수치(4축)는 *강도*를 측정한다. 그러나 관계에는 *의미*도 필요하다. "trust 95 / affinity 90 / respect 90 / wariness 30"인 두 관계가 모두 같지 않다. 양아버지일 수도, 동지일 수도, 옛 연인일 수도 있다. 이 의미를 담는 게 `type`이다.

### 2.1 type — 자유 텍스트 한 줄

`type`은 한 줄 자유 텍스트로 *현재* 관계의 형태를 기술. 4축 수치와 별개의 정보.

예시:
- "양아버지·주인·지기"
- "죽마고우 → 적"
- "아내 → 이별 후 재회 가능한 자"
- "스승 → 떠나간 제자"
- "묻힌 형제들"
- "권력의 정점에서 나를 짓밟은 자"

→ 4축은 *얼마나*를 측정. type은 *무엇*을 명명.

### 2.2 type_history — 관계의 변형 이력

관계의 형태는 *변형*된다. 종료가 아니라 변형. 그래서 *이력*을 기록한다.

```yaml
type_history:
  - { since: "age 7~10",            type: "은인" }
  - { since: "age 10~24",           type: "주인·아버지" }
  - { since: "tp_3_master_falls",   type: "주인·동지" }
  - { since: "tp_6_master_refuses", type: "양아버지·주인·지기 → 떠나는 자" }
```

`since` 키는 자유 텍스트 — `age`, 사건 ID, 인생 단계 등 무엇이든 *언제 그 type이 시작됐는지*를 표현하면 됨.

연청-노준의의 30년이 4번의 변형으로 표현된다. 단순 "양아버지" 한 줄로는 잃어버리는 *시간의 두께*가 보존된다.

### 2.3 transformation_events — type을 바꾼 사건

type을 *바꾼* 사건들. `transition_points`와 cross-reference.

```yaml
transformation_events:
  - { event_id: "tp_4_liangshan",     new_type: "주인·동지" }
  - { event_id: "tp_6_master_refuses", new_type: "떠나는 자" }
```

이게 Pillar 4 ("시간이 의미를 만든다")의 시스템적 구현이다. 10년 전 사건이 *지금의 type*을 만들었음을, 시스템이 명시적으로 추적한다. 재회 시 LLM이 *그 사건*을 sub-text로 활용 가능.

### 2.4 dormant_bonds — 잠재 관계

만난 적은 있으나 활성화되지 않은 연결. 4축이 모두 0 근처인 *약한* 관계지만, "*아예 모름*"과는 다르다.

```yaml
dormant_bonds:
  - target: "어린 시절의 누군가 (구체 미정)"
    last_contact: "age 5~7"
    fragment: "안개 속 누군가의 손길. 얼굴은 기억나지 않음."
    note: "기연 후보 — 게임 진행 중 채워질 빈 슬롯."
```

dormant_bond는 **기연(Pillar 5) 트리거의 핵심 슬롯**이다. NPC의 `life_question`에 *닿는* 사건과 함께 dormant 상대가 등장하면 PAD 비대칭 증폭이 일어나며 활성 관계로 전환.

---

## 3. BondKind — 양극·음극 임계값 시스템

> "지기는 게임 메커닉의 *임계값*이다." — Pillar 3
> 이 임계값은 *양방향*이다. 양극(지기)과 음극(원수) 모두.

### 3.0 왜 양·음 모두 카테고리인가

지기만 카테고리화하고 원수는 자유 텍스트 type으로만 두는 건 시스템 비대칭이다. 무협 세계관에서 원수·숙적·배신자·압제자는 지기 못지않게 *행동을 결정짓는 메커닉*이다 — 임충이 고구·고아내·육겸을 *각각 다른 방식*으로 대하는 이유, 무송이 형의 살해범을 *즉결 처단*하는 이유, 와호장룡 이모백이 푸른여우와 *수십 년 결판 못 낸* 이유.

각각이 *다른 종류의 적*이고 *다른 행동 트리거*를 가진다. 그래서 v0.4에서 통합 enum으로 둔다:

```rust
pub enum BondKind {
    // 지기 — 양극 임계
    SwornBrothers,    // 의형제·동지형
    MasterDisciple,   // 사부-제자형
    Soulmate,         // 영혼의 동반자형
    LoyalRetainer,    // 가신·은인형
    // 원수 — 음극 임계
    BloodEnemy,       // 혈적 — 가족·은인을 해친 자
    ArchRival,        // 숙적 — 평생의 결판 대상
    Betrayer,         // 배신자 — 한때 가까웠으나 등을 돌린 자
    Oppressor,        // 압제자 — 권력으로 짓밟은 자
}

impl BondKind {
    pub fn is_zhiji(&self) -> bool { /* SwornBrothers..LoyalRetainer */ }
    pub fn is_enemy(&self) -> bool { /* BloodEnemy..Oppressor */ }
}
```

한 관계는 한 종류의 BondKind만 가짐 (또는 `null`). 지기와 원수는 상호 배타.

### 3.1 지기 4종류

#### SwornBrothers — 의형제·동지형
> 수호전 노지심-임충, 무송-시은. *함께 죽을 수 있는 형제.*

```yaml
임계값:
  trust:    ≥ +80
  affinity: ≥ +70
  respect:  ≥ +60   # 친구로 인정 가능한 수준
  wariness: ≤ 30
자기희생 형태: "함께 싸우고 함께 죽음." 동귀어진(同歸於盡) 트리거.
대표 행동: 적진 단신 돌입, 형제의 시신 회수, 형제의 원수 평생 추적.
```

#### MasterDisciple — 사부-제자형
> 사조영웅 황약사-곽정. *비급을 전수하는 비대칭 관계.*

```yaml
임계값:
  respect:  ≥ +90   # 압도적 존경이 핵심
  trust:    ≥ +70
  affinity: ≥ +50   # 사부와의 거리감은 자연스러움
  wariness: ≤ 40
자기희생 형태: "비급·심법 전수, 명예를 넘김." 후계자 지정 트리거.
대표 행동: 마지막 비급 전수 후 소멸, 제자 대신 누명 짊어짐, 제자의 사문 입회 보증.
```

#### Soulmate — 영혼의 동반자형
> 와호장룡 이모백-수련. *말하지 않아도 통하는 미완의 관계.*

```yaml
임계값:
  affinity: ≥ +90   # 정서적 일치가 핵심
  trust:    ≥ +80
  respect:  ≥ +70
  wariness: ≤ 20
자기희생 형태: "침묵 속의 결단. 상대를 위해 자기 길을 *바꿈*." 미완의 사랑 트리거.
대표 행동: 고백 없이 떠남, 상대를 위해 강호 은퇴, 임종에 곁을 지킴.
```

#### LoyalRetainer — 가신·은인형
> 연청-노준의. *비대칭적 충성. 받은 은혜를 갚는 관계.*

```yaml
임계값:
  trust:    ≥ +90
  respect:  ≥ +85
  affinity: ≥ +80
  wariness: 임계 없음 (0~50 자연스러움. 주인의 한계를 *아는* 충성이 더 깊음)
자기희생 형태: "주인의 명예를 위해 *자기 신분·미래·생명*을 도구로 씀." 가신 결단 트리거.
대표 행동: 주인 대신 위험 감수, 주인의 정적 회유, 주인 사후 은퇴, 명예를 위해 떠남.
```

### 3.2 원수 4종류

#### BloodEnemy — 혈적 (가족·은인을 해친 자)
> 무송 → 반금련·서문경. *형 무대를 죽인 자에 대한 즉결 처단 의무.*

```yaml
임계값:
  trust:    ≤ -80
  affinity: ≤ -80
  respect:  무관           # 강하든 약하든 상관없음
  wariness: ≥ +70
행동 트리거: "추적·매복·즉결 처단."
대표 행동: 도망 못 하게 막음, 죽음으로만 결판, 일상을 모두 미루고 추적.
```

#### ArchRival — 숙적 (평생의 결판 대상)
> 와호장룡 푸른여우 ↔ 수련. *수십 년 결판 못 낸 결투.*

```yaml
임계값:
  trust:    무관           # 결투 약속은 지킬 거라 인정 가능
  affinity: ≤ -50
  respect:  ≥ +60          # ★ 핵심 — 실력·인격을 *인정*함
  wariness: ≥ +60
행동 트리거: "공정한 결투 신청·자존심 건 시합·결정적 대결로 결판."
대표 행동: 정정당당한 대결 신청, 일대일 결투를 위해 동료를 거리둠, 대결 *전*까지는 임시 휴전 가능.
특이점: respect 높음이 BloodEnemy와의 결정적 차이. *적이지만 인정함*.
```

#### Betrayer — 배신자 (한때 가까웠으나 등을 돌린 자)
> 임충 → 육겸. *죽마고우의 산신묘 배신.*

```yaml
임계값:
  trust:    ≤ -70
  affinity: ≤ -50
  respect:  ≤ -40
  wariness: ≥ +70
  + 추가 조건: type_history에 *이전의 가까운 type*이 존재해야 함
행동 트리거: "폭로·사적 처단·또는 영원한 회피."
대표 행동: 은밀한 추적, 공개적 폭로, 직접 처단 (임충형) 또는 영원한 회피 (펑쩌-손유탕형).
특이점: type_history 의존이 핵심. 단순한 적과 *배신자*의 차이는 "한때 가까웠음"의 흔적.
```

#### Oppressor — 압제자 (권력으로 짓밟은 자)
> 임충 → 고구. *직접 만나지 못하는 거대 권력.*

```yaml
임계값:
  trust:    ≤ -40           # 무조건적 적의이지만 직접 검증 불가
  affinity: ≤ -50
  respect:  -20 ~ +30       # 권력은 인정할 수도, 부정할 수도
  wariness: ≥ +80           # 권력 위협이 핵심
행동 트리거: "체제 자체에 저항. 봉기·암살 모의·반체제 결사 형성."
대표 행동: 양산박 합류, 동조자 결집, 직접 처단보다 *체제 자체와의 싸움*.
특이점: 거리감이 있음. 매일 마주하지 않음. 직접 만난 적 *없을 수도* 있음.
```

### 3.3 진입·이탈의 비대칭 — 양극과 음극이 다르다

이게 v0.4의 핵심 시스템 결정이다.

| 종류 | 진입 | 이탈 |
|---|---|---|
| **지기** (양극) | *천천히* — 4축 임계가 **연속 30일 유지** | *즉시* — 한 축이라도 임계 미달 시 |
| **원수** (음극) | *즉시* — 4축 임계 도달 즉시 | *천천히* — 4축이 임계 위로 회복 후 **연속 30일 유지** |

이게 본질에 부합한다:

- **신뢰는 천천히 쌓이고 즉시 깨진다** → 지기 진입은 일상의 검증을 요구, 이탈은 한 번의 배신으로.
- **적의는 한 사건으로 만들어지고 시간으로 풀린다** → 원수 진입은 사건 자체로, 화해는 일상의 검증을 요구.

산신묘 직후의 임충이 정확히 이 패턴을 보여준다:
- 30년 죽마고우 육겸과의 trust=85가 산신묘 *한 사건*으로 -100 → **즉시 Betrayer 진입**.
- 만약 (있을 리 없지만) 육겸이 살아남아 임충에게 진심으로 사죄한다면, 4축이 점차 회복되어도 *연속 30일 임계 위 유지* 후에야 Betrayer에서 벗어남.

#### 카운터 리셋 룰 (양극·음극 동일)

진입(또는 이탈)을 향한 카운트가 흐르는 도중 *한 축이라도 임계에서 벗어나면* 카운터는 즉시 `null`로 리셋. 다시 임계 도달 시 카운트는 **처음부터** 다시 시작.

```
[T=0]  지기 임계 도달, 카운터 시작
[T=15] 다툼으로 trust 임계 미달 → 카운터 리셋 (15일 무효)
[T=18] 화해, 임계 다시 도달 → 처음부터 카운트
[T=48] 새 카운터 기준 30일 도달 → 진입
```

이게 신뢰의 *연속성*을 강제한다 — "한 번 깨진 후 다시 쌓는 것"은 처음부터의 시간을 요구한다.

#### 30일이라는 숫자

디자인 선택이지 자연 법칙은 아니다. 너무 짧으면(3~7일) 단일 사건으로 진입 가능 → 무게 가벼움. 너무 길면(180일+) 시나리오에서 도달 불가. **30일이 "한 사건 → 일상 한 달 → 사건 한 번 더" 정도의 감정 점검 사이클과 부합.** 게임 진행하며 보정 가능.

종류별 차등도 가능 (예: SwornBrothers 30일 / MasterDisciple 90일 / Soulmate 180일). v0.4에서는 **균일 30일**로 두고 검증 후 보정.

### 3.4 다중 BondKind

한 NPC는 여러 명의 BondKind 보유 가능. 단 *종류는 다른 게 자연스러움*. 두 명의 LoyalRetainer-주인을 동시에 가지는 건 모순. 두 명의 BloodEnemy를 가지는 건 자연 (무송에게 반금련·서문경 둘 다 BloodEnemy).

같은 종류의 *지기* 복수가 있다면 NPC가 *내적 갈등*을 겪어야 한다 — 두 의형제가 서로 적이 되는 사건처럼. 이게 비극의 씨앗.

---

## 4. OCC 감정 → 4축 변화 매핑

이게 Pillar 2 (NPC 자율성)의 핵심 — **관계는 디자이너 스크립트가 아니라 NPC의 감정에서 유도된다**.

### 4.1 변화 함수

```rust
pub fn update_axes_from_emotion(
    rel: &mut Relationship,
    emotion: OccEmotion,
    intensity: f32,         // 0.0 ~ 1.0 (npc-mind-rs OCC intensity)
    npc_hexaco: &Hexaco,
) {
    let base = base_delta(emotion);
    let modulator = hexaco_modifier(emotion, npc_hexaco);
    let delta = base * intensity * modulator;

    rel.trust    = (rel.trust    + delta.trust   ).clamp(-100.0, 100.0);
    rel.affinity = (rel.affinity + delta.affinity).clamp(-100.0, 100.0);
    rel.respect  = (rel.respect  + delta.respect ).clamp(-100.0, 100.0);
    rel.wariness = (rel.wariness + delta.wariness).clamp(   0.0, 100.0);
}
```

### 4.2 base_delta 표 (intensity = 1.0 기준 만점 변화량)

| OCC Emotion | trust | affinity | respect | wariness |
|---|---|---|---|---|
| **Gratitude** (그가 나를 도왔다) | +20 | +10 | 0 | -10 |
| **Anger** (그가 나를 해쳤다) | -25 | -10 | 0 | +25 |
| **Admiration** (그가 훌륭하다) | 0 | 0 | +20 | 0 |
| **Reproach** (그가 비열하다) | -10 | -10 | -25 | +10 |
| **HappyFor** (그의 기쁨이 기쁘다) | +5 | +10 | 0 | 0 |
| **Resentment** (그의 기쁨이 *위협*) | 0 | -10 | -5 | +15 |
| **Pity** (그가 측은하다) | 0 | +10 | -5 | 0 |
| **Gloating** (그의 불행이 통쾌) | -10 | -20 | -10 | 0 |
| **Pride** (그를 통해 자랑스럽다) | 0 | +5 | +10 | 0 |
| **Shame** (그를 통해 부끄럽다) | -5 | -10 | -10 | +5 |
| **Love** (그를 좋아함) | +5 | +20 | +5 | -5 |
| **Hate** (그를 싫어함) | -10 | -25 | -5 | +15 |

### 4.3 HEXACO 보정자

같은 Gratitude라도 인물마다 강도가 다르다. 보정자의 예:

| HEXACO 특성 | 보정 | 의미 |
|---|---|---|
| H+ Sincerity 높음 | trust 변화 ×1.2 | 정직한 인물은 신뢰를 *진심*으로 갱신 |
| A+ Patience 높음 | 모든 변화 ×0.7 | 참을성 있는 인물은 천천히 |
| A- Forgiveness 낮음 | 부정 감정 변화 ×1.5 | 용서 못 함 |
| E+ Anxiety 높음 | wariness 변화 ×1.3 | 불안한 인물은 경계가 빨리 쌓임 |
| C+ Prudence 높음 | 큰 변화 시 ×0.8, 시간 분산 | 신중한 인물은 즉각 결론짓지 않음 |
| O+ Unconventionality 높음 | 양극 도달 더 쉬움 | 극단으로 치닫는 경향 |

→ 임충(C+ Prudence 높음)은 같은 배신 사건에도 천천히 wariness가 쌓이지만, 임계점 폭발 시 -100으로 *즉시* 도달한다. 이게 그의 "인내 → 폭발" 패턴의 시스템적 근거.

### 4.4 통합 흐름 — npc-mind-rs 연결

```
사건 발생 (Event)
  ↓
appraise()              [npc-mind-rs / domain]
  → OccEmotion + intensity
  ↓
apply_stimulus()        [npc-mind-rs / domain]
  → PAD 갱신
  ↓
RelationshipUpdater     [NEW — 본 문서 정의]
  → 4축 갱신 (위 함수)
  ↓
type 변화 체크
  → 사건이 transformation_event 자격이면 type_history 갱신
  ↓
BondKind 임계값 체크
  → 양극(지기) 진입: 연속 30일 카운트 흐름
  → 음극(원수) 진입: 즉시
  → BondKindEntered / BondKindExited 도메인 이벤트 emit (Event Sourcing)
```

**`RelationshipUpdater`는 `src/domain/relationship/`에 위치하는 도메인 서비스**. tokio import 없음. PAD/OCC와 동일한 runtime-agnostic 원칙을 따른다.

BondKind 진입·이탈은 **도메인 이벤트로 emit**되어 EventBus를 통해 다른 시스템(자기희생 트리거, 처단 트리거, 기연 트리거, 평판 시스템, 동행 시스템)이 구독 가능하다.

---

## 5. 검증 사례

세 인물의 핵심 관계가 본 시스템에서 어떻게 표현되는지 검증한다.

### 5.1 연청 → 노준의 (LoyalRetainer 지기)

```yaml
target: "lu_zhonyi"
type: "양아버지·주인·지기 → 떠나는 자"
type_history:
  - { since: "age 7~10",             type: "은인" }
  - { since: "age 10~24",            type: "주인·아버지" }
  - { since: "tp_3_master_falls",    type: "주인·동지" }
  - { since: "tp_6_master_refuses",  type: "양아버지·주인·지기 → 떠나는 자" }
axes: { trust: 95, affinity: 90, respect: 90, wariness: 30 }
bond_kind: "LoyalRetainer"
bond_since: "tp_3_master_falls"   # 가산 몰수 후 함께 거지 신세 → 안정적 임계 진입
transformation_events:
  - { event_id: "tp_2_taken_in",       new_type: "은인" }
  - { event_id: "tp_3_master_falls",   new_type: "주인·동지" }
  - { event_id: "tp_6_master_refuses", new_type: "떠나는 자" }
```

**검증 포인트:**
- ✅ **wariness=30이 의미를 가짐.** "노준의도 인간이고 한계가 있다"는 *작지만 0이 아닌* 인식. 이게 충고를 *하는* 동력이고, 거절당했을 때 *떠나는* 동력이다. wariness=0이면 그저 따라 죽음 — 이야기가 사라진다.
- ✅ **LoyalRetainer 임계값 충족** (trust 95 ≥ 90, respect 90 ≥ 85, affinity 90 ≥ 80, wariness 임계 없음).
- ✅ **자기희생은 *함께 죽음*이 아닌 *주인의 명예를 위해 떠남*** — LoyalRetainer 종류의 정확한 발현.
- ✅ **type_history에 30년이 4단계로 압축**되어 LLM이 *어느 시점의 연청인가*를 정확히 연기 가능.

### 5.2 임충 → 육겸 (Betrayer 원수, *즉시* 진입)

산신묘 사건 직전 → 직후의 axes 변화로 *변형*과 *음극 즉시 진입*을 동시에 검증한다.

```yaml
# 산신묘 사건 *전* (배신 발각 전, 죽마고우 인식 유지)
target: "lu_qian"
type: "죽마고우 (어린 시절부터)"
axes: { trust: 85, affinity: 70, respect: 60, wariness: 15 }
bond_kind: null                   # 양극 임계 미달 (respect 60 < 80)

# 산신묘 사건 *후* (배신 확인 → 처단)
target: "lu_qian"
type: "죽마고우 → 적·처단 대상"
type_history:
  - { since: "유년기",              type: "죽마고우" }
  - { since: "shanshenmiao_event",  type: "적·처단 대상" }
axes: { trust: -100, affinity: -90, respect: -100, wariness: 100 }
bond_kind: "Betrayer"
bond_since: "shanshenmiao_event"  # ★ 즉시 진입 — 음극 임계는 사건 자체로 활성화
transformation_events:
  - { event_id: "shanshenmiao_event", new_type: "적·처단 대상" }
```

**검증 포인트:**
- ✅ **"끊어졌다"가 아니라 *적*으로 변형.** 단순 0 변경이면 "이제 모르는 사이"가 되어 살해 동기가 약하다. -100/-100/-100/+100은 *복수의 화신*이 되는 시스템적 근거.
- ✅ **Betrayer 임계 + type_history 조건 모두 충족.** 유년기 "죽마고우"가 type_history에 남아있어야 *단순 적*과 다른 Betrayer 자격 충족.
- ✅ **음극 즉시 진입.** 산신묘 사건 한 번으로 `bond_kind: Betrayer` 활성화. 30일 대기 없음. 이게 "적의는 사건이 만든다"의 시스템적 표현.
- ✅ **C+ Prudence 보정 효과.** 임충은 보통 천천히 변화하지만, 산신묘에서 들은 대화의 OCC intensity = 1.0 + Anger 만점이 한 번에 -100으로 끌어내림. "인내 → 폭발" 패턴.

### 5.3 펑쩌 → 손유탕 (배신 후 잔존하는 affinity, *임계 미달* 사례)

> 무협 세계관 외 사례지만, 4축 시스템이 *모순된 감정의 공존*을 표현하는지 검증.

```yaml
target: "sun_yutang"
type: "전 남편 → 다시 만난 옛 인연 → 침묵의 헤어짐"
type_history:
  - { since: "결혼 전",              type: "지적 동지" }
  - { since: "결혼",                 type: "남편" }
  - { since: "guo_moruo_affair",     type: "이혼한 전 남편" }
  - { since: "beijing_reunion",      type: "다시 만난 옛 인연" }
  - { since: "confession_night",     type: "침묵의 헤어짐" }
axes: { trust: -50, affinity: 40, respect: -10, wariness: 60 }
bond_kind: null    # ★ Betrayer 임계 미달 (affinity +40, 임계는 ≤ -50)
transformation_events:
  - { event_id: "guo_moruo_affair",  new_type: "이혼한 전 남편" }
  - { event_id: "beijing_reunion",   new_type: "다시 만난 옛 인연" }
  - { event_id: "confession_night",  new_type: "침묵의 헤어짐" }
```

**검증 포인트:**
- ✅ **trust=-50, affinity=+40의 *비대칭*.** 손유탕의 회고 ("처음부터 완전히 의지할 수 없는 사람이었다, 그러나 항상 아름답다고 생각했다")를 시스템적으로 표현.
- ✅ **bond_kind = null 처리가 의미 있음.** Betrayer 임계 (`trust ≤ -70 AND affinity ≤ -50 AND respect ≤ -40`)에서 affinity가 +40이라 *임계 미달*. 시스템이 "배신은 있었으나 *영원한 적은 아닌* 관계"를 정확히 표현.
- ✅ **임충 사례와의 대조.** 같은 "배신"이지만 임충은 모든 음극 임계 충족 → Betrayer 즉시 진입. 펑쩌는 임계 미달 → bond_kind 없이 자유 텍스트 type만으로 처리. **시스템이 두 종류의 배신을 *다르게* 분류함을 입증.**
- ✅ **type_history 5단계.** 관계의 *서사적 깊이*가 시스템에 보존됨.

---

## 6. 다음 단계

본 문서가 *정의하지 않는* 것 — 향후 별도 문서:

1. **동행(同行) 시스템** — 관계가 깊어지면 두 인물의 *HEXACO가 미세하게 변한다*. "누구와 함께 길을 갔느냐가 그 사람을 만든다." 별도: `companions.md` (가칭).
2. **평판(評判) 시스템** — 한 사람의 4축이 *사회 전체*에 미치는 효과. 별도: `reputation.md`.
3. **인연(因緣)·기연(奇緣) 트리거 룰** — `dormant_bonds` → `key_bonds` 활성화의 정확한 조건. Pillar 5 직결.
4. **자기희생/처단 행동 트리거** — BondKind별로 어떤 행동 패턴을 emit할지 구체 룰. 본 문서는 *임계값과 분류*까지, 행동 emit은 별도 시스템.

본 문서가 발생시키는 **스키마 v0.4 보정 사항**:

- `key_bonds[]`의 `zhiji_kind` → **`bond_kind`** 로 이름 변경. enum variants 8개 (지기 4 + 원수 4).
- `key_bonds[]`의 `zhiji_since` → **`bond_since`** 로 이름 변경.
- 검증 체크리스트의 "zhiji_kind ↔ axes 임계 일관성" → "bond_kind ↔ axes 임계 일관성 (양극·음극 모두)" 로 갱신.

---

## 변경 이력

| 버전 | 일자 | 변경 |
|------|------|------|
| v0.3 | 2026-05-04 | 초안. 4축 정의(직교 + 음수) + type/type_history + 4종류 지기 + OCC 매핑 + 3 검증 사례 |
| v0.4 | 2026-05-04 | **BondKind 통합 (지기 4종 + 원수 4종 = 8 variants).** 진입·이탈 비대칭 룰 명시 (지기: 진입 천천히/이탈 즉시, 원수: 진입 즉시/이탈 천천히). `zhiji_kind` → `bond_kind`, `zhiji_since` → `bond_since` 이름 변경. §5 검증 사례에 bond_kind 표기 추가. 펑쩌 사례를 *임계 미달*로 재해석하여 시스템의 분류 정확도 입증. |
