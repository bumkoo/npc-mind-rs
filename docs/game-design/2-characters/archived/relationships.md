# 관계 시스템 (Relationships)

> Version: 0.5 — 2026-05-04
> 위치: `docs/game-design/2-characters/relationships.md`
> 의존: `_schema.md` v0.5, `00-pillars.md` v0.1
> 참조: `npc-mind-rs` OCC/PAD 엔진

## 0. 설계 원칙

이 문서는 Pillar 3 ("관계가 곧 시스템")의 *본체*다.

세 명제가 모든 디자인 결정의 시금석:

1. **호감/적대 이분법은 거짓이다.** 한 사람을 신뢰하면서도 두려워할 수 있고, 존경하면서도 함께 있고 싶지 않을 수 있다.
2. **관계는 *상태*가 아니라 *형태*다.** "끊어졌다"는 없다. 모든 만남은 어떤 *형태*로든 존재한다 — 적, 미련, 잊혀진 자, 묻힌 형제까지.
3. **관계는 NPC가 *스스로* 갱신한다.** 디자이너가 스크립트로 박지 않는다. NPC의 OCC 감정이 관계를 만들고, 관계가 다시 다음 감정의 강도를 결정한다.

v0.5의 핵심 추가 명제:

4. **관계는 *세 차원*에서 동시에 존재한다.** 정서·기능적 분류(BondKind), 활동 상태(BondStatus), 형식적 동반(Partnership). 셋은 *직교*하며, 한 차원의 변화가 다른 차원을 자동 결정하지 않는다.

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
| trust↑ + wariness↑ | "예측 가능하지만 위험한 사람" — 합리적 적, 야심가 | 연청에게 송강 |
| respect↑ + affinity↓ | "위대하지만 가까이하기 어려움" | 와호장룡 이모백을 보는 옥교룡 (초기) |
| affinity↑ + trust↓ | "그리우나 의지할 수 없음" | 펑쩌에 대한 손유탕 |
| trust↑ + affinity↑ + respect↓ | "내 사람이지만 의지할 수 없음" — 보호 본능 | 자식을 보는 부모 |
| respect↑ + affinity↓ + wariness↑ | "적이지만 인정함" — 숙적 | 와호장룡 푸른여우 ↔ 수련 |
| trust↑ + respect↑ + affinity↑ + wariness↑(낮지 않음) | "지기이나 그도 인간이다" — 한계 인식 | 연청에게 노준의 (95/90/90/30) |

직교성은 *이분법의 거짓*을 시스템적으로 구현한다. 단일 축의 "호감도"로는 위 패턴 중 어느 것도 표현되지 않는다.

### 1.3 음수의 의미

음수는 단순히 "낮음"이 아니라 *적극적 반대 인식*이다.

- **trust = 0**: 이 사람을 모름. 예측 불가.
- **trust = -50**: *확신을 가지고* 의심함. "이 사람의 말은 거짓이다."
- **trust = -100**: 모든 말이 함정이라 학습됨.

마찬가지로:
- **affinity 음수** = 혐오. 함께 있는 게 고통.
- **respect 음수** = 경멸. 이 사람을 *아래*로 봄.
- **wariness는 음수 없음.** 0이 이미 "완전 무방비"의 극값.

### 1.4 갱신 빈도와 점착성

- **즉각 갱신**: 한 사건당 ±1~30. OCC 감정 기반 (§4).
- **주기 감쇠**: 큰 변화는 시간이 흐르며 평균값으로 *수렴*. 단 `transformation_event`로 기록된 변화는 감쇠하지 않음.
- **양극의 점착성**: ±100에 도달하면 추가 입력에도 머무름. 한 번 *완전한* 신뢰/불신/혐오/경멸에 도달한 관계는 다시 일상으로 돌아가지 않는다.
- **사망 후 점착**: `bond_status: Deceased`로 전환된 관계의 axes는 freeze된다 — OCC 입력이 없으므로 자연 변화 없음. 회상 OCC는 별도 처리(§4.5).

---

## 2. 관계의 형태 — type과 type_history

수치(4축)는 *강도*를 측정한다. 그러나 관계에는 *의미*도 필요하다. 이 의미를 담는 게 `type`이다.

### 2.1 type — 자유 텍스트 한 줄

`type`은 한 줄 자유 텍스트로 *현재* 관계의 형태를 기술. 4축 수치와 별개의 정보.

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

`since` 키는 자유 텍스트 — `age`, 사건 ID, 인생 단계 등 무엇이든.

### 2.3 transformation_events — type을 바꾼 사건

```yaml
transformation_events:
  - { event_id: "tp_4_liangshan",     new_type: "주인·동지" }
  - { event_id: "tp_6_master_refuses", new_type: "떠나는 자" }
```

이게 Pillar 4 ("시간이 의미를 만든다")의 시스템적 구현.

### 2.4 dormant_bonds — 잠재 관계

만난 적은 있으나 활성화되지 않은 연결. **"한 번도 활성화된 적 없는" 잠재 관계.** 한 번 활성이었다가 비활성이 된 관계는 dormant_bonds가 아니라 `key_bonds[].bond_status: Dormant` 또는 `Reactivating`으로 처리 (§3.5).

```yaml
dormant_bonds:
  - target: "어린 시절의 누군가 (구체 미정)"
    last_contact: "age 5~7"
    fragment: "안개 속 누군가의 손길. 얼굴은 기억나지 않음."
    note: "기연 후보 — 게임 진행 중 채워질 빈 슬롯."
```

dormant_bond는 **기연(Pillar 5) 트리거의 핵심 슬롯**.

---

## 3. 관계의 세 차원 — BondKind · BondStatus · Partnership

v0.5 핵심 변경. 관계를 단일 enum으로 표현하던 v0.4에서, **세 차원의 직교 분류**로 확장.

```rust
pub struct Relationship {
    // ... axes, type_history, transformation_events ...
    pub bond_kind:   Option<BondKind>,     // 정서·기능적 분류 (지기/원수/멘토)
    pub bond_status: BondStatus,           // 활동 상태 (Active/Deceased/Resolved/Dormant/Reactivating)
    pub partnership: Option<Partnership>,  // 형식적 동반 (Spouse/Engaged/Lover/Separated)
}
```

세 차원은 *직교*. 한 관계가 동시에 *Soulmate + Deceased + Spouse*일 수 있고 (사망한 부부), *null + Active + Lover*일 수 있다 (연인이지만 BondKind 어느 분류에도 안 맞음).

### 3.0 차원의 구분 원칙

| 차원 | 무엇을 표현하는가 | 변화의 동력 |
|---|---|---|
| **BondKind** | *정서적·기능적 분류*. 이 관계가 어떤 종류인가 | OCC 감정 누적 → axes 변화 → 임계 도달/이탈 |
| **BondStatus** | *현재 활동 상태*. 이 관계가 지금 어떻게 작동하는가 | 사건 (사망·결판·재발견) |
| **Partnership** | *형식적 동반*. 이 관계의 사회적 형식 | 결혼·정혼·이혼 같은 *공식 사건* |

같은 BondKind라도 status에 따라 행동 트리거가 다르다 (활성 ArchRival은 결투 추구 / Resolved ArchRival은 추모). 같은 Partnership이라도 BondKind가 다르면 의미가 다르다 (Soulmate Spouse는 영혼+형식 일치 / null Spouse는 정략결혼).

### 3.1 BondKind — 9 variants (지기 4 + 원수 4 + 멘토 1)

```rust
pub enum BondKind {
    // 지기 — 양극 임계 (진입 천천히 / 이탈 즉시)
    SwornBrothers,    // 의형제·동지형
    MasterDisciple,   // 사부-제자형 (무술 비전 전수)
    Soulmate,         // 영혼의 동반자형
    LoyalRetainer,    // 가신·은인형
    // 원수 — 음극 임계 (진입 즉시 / 이탈 천천히)
    BloodEnemy,       // 혈적 — 가족·은인을 해친 자
    ArchRival,        // 숙적 — 평생의 결판 대상
    Betrayer,         // 배신자 — 한때 가까웠으나 등을 돌린 자
    Oppressor,        // 압제자 — 권력으로 짓밟은 자
    // 멘토 — 중간극 임계 (v0.5 신설)
    Mentor,           // 인생 선배·후배 (무술 사부 아닌 인생 가르침)
}

impl BondKind {
    pub fn is_zhiji(&self) -> bool { /* SwornBrothers..LoyalRetainer */ }
    pub fn is_enemy(&self) -> bool { /* BloodEnemy..Oppressor */ }
    pub fn is_mentor(&self) -> bool { /* Mentor */ }
}
```

#### 지기 4종류 (변화 없음)

##### SwornBrothers — 의형제·동지형
```yaml
임계값: { trust ≥+80, affinity ≥+70, respect ≥+60, wariness ≤30 }
자기희생: "함께 싸우고 함께 죽음." 동귀어진(同歸於盡) 트리거.
```

##### MasterDisciple — 사부-제자형
```yaml
임계값: { respect ≥+90, trust ≥+70, affinity ≥+50, wariness ≤40 }
자기희생: "비급·심법 전수, 명예를 넘김." 후계자 지정 트리거.
특이점: ★ Mentor와 차이는 *무술 비전 전수*가 핵심. 비급이 없으면 Mentor.
```

##### Soulmate — 영혼의 동반자형
```yaml
임계값: { affinity ≥+90, trust ≥+80, respect ≥+70, wariness ≤20 }
자기희생: "침묵 속의 결단. 상대를 위해 자기 길을 *바꿈*." 미완의 사랑 트리거.
특이점: Partnership 슬롯과 *직교*. 부부일 수도, 미발현일 수도 있음.
```

##### LoyalRetainer — 가신·은인형
```yaml
임계값: { trust ≥+90, respect ≥+85, affinity ≥+80, wariness 임계 없음 }
자기희생: "주인의 명예를 위해 자기 신분·미래·생명을 도구로 씀."
```

#### 원수 4종류 (변화 없음)

##### BloodEnemy — 혈적
```yaml
임계값: { trust ≤-80, affinity ≤-80, respect 무관, wariness ≥+70 }
행동 트리거: "추적·매복·즉결 처단."
```

##### ArchRival — 숙적
```yaml
임계값: { trust 무관, affinity ≤-50, respect ≥+60, wariness ≥+60 }
행동 트리거: "공정한 결투·자존심 건 시합·결판."
특이점: respect 높음이 BloodEnemy와의 결정적 차이.
```

##### Betrayer — 배신자
```yaml
임계값: { trust ≤-70, affinity ≤-50, respect ≤-40, wariness ≥+70 }
+ 추가 조건: type_history에 *이전의 가까운 type*이 존재해야 함
행동 트리거: "폭로·사적 처단·또는 영원한 회피."
```

##### Oppressor — 압제자
```yaml
임계값: { trust ≤-40, affinity ≤-50, respect -20~+30, wariness ≥+80 }
행동 트리거: "체제 자체에 저항. 봉기·반체제 결사."
```

#### Mentor — 인생 선배·후배 (★ v0.5 신설)

> 와호장룡 수련-옥교룡, 수호전 노지심-임충 (잠재). 청강만리에서 노년 수련-춘설병.
> *가르치되 가두지 않는*, 또는 *가르치려 했으나 따르지 않은*.

```yaml
임계값:
  trust:    ≥ +50         # 진심을 알아봄
  affinity: ≥ +50         # 정서적 연결
  respect:  ≥ +60         # 자질·재능 인정 (멘티에 대한)
  wariness: ≤ 60          # ★ 경계가 *낮지 않을 수도* (멘티가 길을 잘못 들 위험 인식)
  + 추가 조건: type_history에 "가르치려 함" 또는 "조언함" 의미의 type 존재

자기희생 형태: "자기 시간·평판·미래를 *후배의 길*에 투자."
대표 행동: 갈림길에서 충고, 후배를 위한 위험 감수, 후배가 따르지 않아도 *지켜봄*.

특이점:
  - MasterDisciple과 차이:
    * 무술 비전 전수 *없음* (비급 슬롯 없음)
    * respect 임계 낮음 (90 → 60)
    * wariness 임계 *높음* (40 → 60). 멘티가 어긋날 위험을 *이미 인식*.
  - SwornBrothers와 차이: *비대칭*. 멘토와 멘티는 동등한 형제 아님.
  - 양방향 가능: A → B Mentor / B → A Mentee. 한 관계의 양쪽이 다르게 분류됨.
```

##### Mentor 진입·이탈 룰
- **진입**: 양극이지만 SwornBrothers보다 *짧음* — **연속 14일 유지**. 실제로 인생 멘토 관계는 *한 번의 충고와 그 후의 짧은 동행*으로도 형성 가능.
- **이탈**: 즉시 (양극 일반 룰).

이 14일 vs 30일 차등이 v0.5의 *진입 시간 차등*의 첫 사례. v0.4는 균일 30일이었음. 차등이 합리적인지는 추가 인스턴스 검증 필요.

### 3.2 진입·이탈 룰 (v0.4 유지 + Mentor 추가)

| 종류 | 진입 | 이탈 |
|---|---|---|
| 지기 (SwornBrothers/MasterDisciple/Soulmate/LoyalRetainer) | 4축 임계 *연속 30일 유지* | 즉시 (한 축이라도 임계 미달) |
| 멘토 (Mentor) | 4축 임계 *연속 14일 유지* | 즉시 |
| 원수 (BloodEnemy/ArchRival/Betrayer/Oppressor) | 즉시 (4축 임계 도달) | 4축이 임계 위로 회복 후 *연속 30일 유지* |

**카운터 리셋 룰** (양극·중간극·음극 동일): 카운트 흐름 중 한 축이라도 임계에서 벗어나면 카운터 즉시 `null`로 리셋. 다시 임계 도달 시 처음부터 카운트.

### 3.3 다중 BondKind

한 NPC는 여러 명의 BondKind 보유 가능. 단 *종류는 다른 게 자연스러움*. 두 명의 LoyalRetainer-주인은 모순. 같은 종류의 *지기* 복수는 *내적 갈등*의 씨앗 — 두 의형제가 서로 적이 되는 사건처럼.

### 3.4 BondKind 비대칭

A → B의 bond_kind와 B → A의 bond_kind는 *다를 수 있다.* Mentor 관계가 가장 명확:
- 수련 → 옥교룡: Mentor (가르치려 함)
- 옥교룡 → 수련: null 또는 Mentee 형태 (배우려는 자는 아님 — 따르지 않음)

각 NPC의 인스턴스 파일에서 *자기 관점*의 bond_kind만 기록. 시스템이 자동 대칭화하지 않음.

### 3.5 BondStatus — 5 variants (★ v0.5 신설)

```rust
pub enum BondStatus {
    Active,                              // 활성 관계, 상호작용 가능
    Resolved { reason: String },         // 결판 도달 (ArchRival 결판, 화해, 깨끗한 이별)
    Deceased,                            // 상대 사망
    Dormant,                             // 비활성 (오래 멈춘 활성 관계)
    Reactivating { trigger: EventId },   // 재활성화 단서 들어옴
}
```

#### 각 status의 의미

##### Active
일반 활성 관계. 4축 갱신·OCC 입력·BondKind 임계 평가 모두 정상 작동.

##### Resolved { reason }
관계의 *주된 동력이 끝남*. ArchRival의 결판, 가족 갈등의 화해, 옛 연인의 깨끗한 이별 등.

```yaml
bond_status: { Resolved: { reason: "결판 도달 — 푸른여우 처단됨" } }
```

- axes는 freeze (자연 변화 없음).
- BondKind는 그대로 유지 (수련의 정체성 형성에 미친 영향은 영구).
- 행동 트리거 *불활성* — 결판 도달한 ArchRival에게 새 결투 신청 트리거 안 함.
- 회상 OCC는 처리 가능 (§4.5).

##### Deceased
상대 사망. 가장 흔한 status 변경 사유.

```yaml
bond_status: Deceased
deceased_at: "li_mubai_death"   # ★ EventId 보존
```

- axes freeze.
- BondKind 그대로 유지 (사후에도 정체성 영향 영구).
- partnership도 그대로 유지 (DeceasedSpouse 같은 별도 variant 불필요 — Partnership: Spouse + Status: Deceased로 표현).
- 회상 OCC, 추모 행동 트리거 활성.

##### Dormant
한 번 활성이었던 관계가 *오래 멈춤*. 옛 친구와 연락 끊긴 상태, 떠난 가족 등.

```yaml
bond_status: Dormant
last_active: "20대 후반"
```

- axes freeze.
- 시간이 흐르면 *서서히 감쇠 가능* (Dormant 전용 룰 — 점착성과 다름).
- 재활성화 트리거 시 status: Reactivating으로 전환.

##### Reactivating { trigger }
Dormant이던 관계에 활성화 단서가 들어옴. 옥교룡 사례.

```yaml
bond_status: { Reactivating: { trigger: "current_rumor" } }
```

- axes 부분 unfreeze. 새 OCC 입력 받기 시작.
- 단서 확인 후 Active 또는 Dormant 복귀 결정.
- *짧은 transitional 상태*. 보통 하나의 시나리오 호 안에서 해소됨.

#### Status 전환 다이어그램

```
                ┌──────────────────────┐
                ▼                      │
              Active ←─────────── Reactivating
              │  │                      ▲
              │  └──→ Resolved          │
              │                         │
              ├──→ Deceased             │
              │                         │
              └──→ Dormant ─────────────┘
                       (단서 들어옴)
```

`Resolved`와 `Deceased`는 *terminal*. 다시 Active로 돌아오지 않음 (상대 사망은 회복 불가, 결판 도달은 재결판 불가 — 새 적이라면 다른 BondKind).
`Dormant`는 *복귀 가능*. Reactivating을 거쳐 Active로.

### 3.6 Partnership — 4 variants (★ v0.5 신설)

```rust
pub enum Partnership {
    Spouse,      // 정식 결혼한 부부
    Engaged,     // 정혼
    Lover,       // 연인 (결혼 전, 비정혼)
    Separated,   // 휴서·이혼·별거 (결혼은 *있었던*)
}
```

Partnership은 BondKind와 *완전히 직교*. 같은 BondKind: Soulmate가 partnership: Spouse일 수도 (영혼+형식 일치 부부), null일 수도 (와호장룡 이모백-수련 — 영혼 일치하나 부부 미발현).

#### 각 variant의 의미

##### Spouse
정식 결혼. 사회적·법적 공식 관계. 결혼 후 어느 쪽이 사망해도 partnership: Spouse 유지 (bond_status: Deceased로 *상태*만 변경).

##### Engaged
정혼 상태. 결혼 전. 무협 세계관에서 정혼은 거의 결혼만큼의 무게 — 정혼자 사망 시 partnership: Engaged + bond_status: Deceased로 보존되며, 이게 평생 정절의 근거가 될 수 있음 (수련-맹사조 케이스).

##### Lover
정혼·결혼 *없는* 연인. 사적 결합. 무협에서 자주 비극의 형태 (사회적 인정 없음).

##### Separated
결혼 후 별거·이혼·휴서. 결혼이 *있었던* 사실 자체는 보존. 임충-장씨 케이스 (휴서 발급).

#### Partnership과 axes의 관계

Partnership은 axes와 *직접 연동되지 않는다*. 부부라는 사실이 자동 trust↑가 되지 않음. 정략결혼은 trust 0 + Partnership: Spouse도 가능. 반대로 trust 95 + Partnership: null도 가능 (수련-이모백).

이게 Partnership을 *별도 슬롯*으로 둔 이유 — axes·BondKind가 이미 정서·기능 차원을 표현하므로, 형식 차원만 Partnership에 분리.

#### Partnership 변화의 동력

Partnership은 OCC 감정 누적이 아닌 **공식 사건**으로 변화:
- 결혼식 → null → Spouse
- 이혼·휴서 → Spouse → Separated
- 정혼 파기 → Engaged → null

이 사건들은 transformation_events에 등록되며, axes는 그 사건의 OCC 감정으로 *별도* 변화한다 (사건 = 단일 source가 두 슬롯에 영향).

---

## 4. OCC 감정 → 4축 변화 매핑

### 4.1 변화 함수 (v0.4 유지)

```rust
pub fn update_axes_from_emotion(
    rel: &mut Relationship,
    emotion: OccEmotion,
    intensity: f32,
    npc_hexaco: &Hexaco,
) {
    // bond_status 검사 — Deceased / Resolved / Dormant는 자연 변화 없음
    if !rel.bond_status.accepts_live_input() {
        return;  // 회상 OCC는 별도 함수 (§4.5)
    }

    let base = base_delta(emotion);
    let modulator = hexaco_modifier(emotion, npc_hexaco);
    let delta = base * intensity * modulator;

    rel.trust    = (rel.trust    + delta.trust   ).clamp(-100.0, 100.0);
    rel.affinity = (rel.affinity + delta.affinity).clamp(-100.0, 100.0);
    rel.respect  = (rel.respect  + delta.respect ).clamp(-100.0, 100.0);
    rel.wariness = (rel.wariness + delta.wariness).clamp(   0.0, 100.0);
}
```

### 4.2 base_delta 표 (v0.4 유지)

| OCC Emotion | trust | affinity | respect | wariness |
|---|---|---|---|---|
| Gratitude | +20 | +10 | 0 | -10 |
| Anger | -25 | -10 | 0 | +25 |
| Admiration | 0 | 0 | +20 | 0 |
| Reproach | -10 | -10 | -25 | +10 |
| HappyFor | +5 | +10 | 0 | 0 |
| Resentment | 0 | -10 | -5 | +15 |
| Pity | 0 | +10 | -5 | 0 |
| Gloating | -10 | -20 | -10 | 0 |
| Pride | 0 | +5 | +10 | 0 |
| Shame | -5 | -10 | -10 | +5 |
| Love | +5 | +20 | +5 | -5 |
| Hate | -10 | -25 | -5 | +15 |

### 4.3 HEXACO 보정자 (v0.4 유지)

| HEXACO 특성 | 보정 |
|---|---|
| H+ Sincerity 높음 | trust 변화 ×1.2 |
| A+ Patience 높음 | 모든 변화 ×0.7 |
| A- Forgiveness 낮음 | 부정 감정 변화 ×1.5 |
| E+ Anxiety 높음 | wariness 변화 ×1.3 |
| C+ Prudence 높음 | 큰 변화 시 ×0.8, 시간 분산 |
| O+ Unconventionality 높음 | 양극 도달 더 쉬움 |

### 4.4 통합 흐름 — npc-mind-rs 연결

```
사건 발생 (Event)
  ↓
appraise()              [domain]
  → OccEmotion + intensity
  ↓
apply_stimulus()        [domain]
  → PAD 갱신
  ↓
RelationshipUpdater     [domain]
  → bond_status 검사
  → 4축 갱신 (Active만)
  ↓
type 변화 체크
  → transformation_event 등록 시 type_history 갱신
  ↓
Partnership 변화 체크
  → 공식 사건이면 Partnership 갱신
  ↓
BondKind 임계값 체크
  → 양극(지기) 진입: 연속 30일 카운트
  → 멘토 진입: 연속 14일 카운트
  → 음극(원수) 진입: 즉시
  → BondKindEntered / BondKindExited 도메인 이벤트 emit
  ↓
BondStatus 자동 전환 검사
  → 사망 사건이면 → Deceased
  → 결판 사건이면 → Resolved
  → BondStatusChanged 도메인 이벤트 emit
```

### 4.5 회상 OCC — Deceased/Resolved 관계 (★ v0.5 신설)

사망·결판 도달 관계는 새 입력을 받지 않지만, NPC가 *회상*할 때 OCC 감정이 발생할 수 있다. 수련이 이모백을 떠올릴 때 Sadness·Love·Pride 등.

```rust
pub fn process_recollection_occ(
    rel: &Relationship,
    emotion: OccEmotion,
    intensity: f32,
) -> RecollectionEffect {
    // axes는 *변경하지 않음* (관계 자체는 freeze)
    // 그러나 NPC의 PAD에는 영향 — 일시적 감정 변화
    RecollectionEffect {
        pad_delta: emotion.to_pad_delta(intensity),
        triggers_action: emotion.is_strong_enough(intensity),
    }
}
```

회상 OCC는:
- axes를 변경하지 않음 (관계는 freeze).
- NPC PAD에는 일시적 영향 (며칠간 슬픔 등).
- 강한 회상은 *행동* 트리거 가능 (추모 의식, 옛 장소 방문).

이게 "사망 후에도 관계는 정체성에 영향"의 시스템적 표현. axes 변화 없이도 NPC 행동에 반영.

---

## 5. 검증 사례 — v0.5 적용

### 5.1 연청 → 노준의 (LoyalRetainer 지기, Active)

```yaml
target: "lu_zhonyi"
type: "양아버지·주인·지기 → 떠나는 자"
axes: { trust: 95, affinity: 90, respect: 90, wariness: 30 }
bond_kind: "LoyalRetainer"
bond_status: "Active"
partnership: null
bond_since: "tp_3_master_falls"
```

> v0.4와 동일. Active 상태이므로 status 명시적 표기만 추가.

### 5.2 임충 → 육겸 (Betrayer, *결판 도달* → Resolved)

```yaml
target: "lu_qian"
type: "죽마고우 → 적·처단 대상 → 처단됨"
axes: { trust: -100, affinity: -90, respect: -100, wariness: 100 }
bond_kind: "Betrayer"
bond_status: { Resolved: { reason: "산신묘에서 직접 처단" } }
partnership: null
bond_since: "shanshenmiao_event"
```

> v0.5의 핵심 변화: 임충이 *직접 처단*했으니 Resolved로 전환. axes는 freeze. 회상 OCC만 작동 — 죽마고우를 직접 죽인 사실의 그림자.

### 5.3 수련 → 이모백 (Soulmate, Deceased, Partnership 미발현)

```yaml
target: "li_mubai"
type: "영원히 미완의 사랑"
axes: { trust: 95, affinity: 95, respect: 95, wariness: 5 }
bond_kind: "Soulmate"
bond_status: "Deceased"
partnership: null         # ★ Soulmate + Spouse가 가능했으나 발현 안 됨
deceased_at: "li_mubai_death"
```

> v0.5의 직교성 검증: BondKind: Soulmate + Partnership: null. 영혼의 일치는 있으나 부부 형식은 발현되지 않은 *비극의 정확한 표현*.

### 5.4 임충 → 장씨 (Partnership: Separated, BondKind 미매핑)

```yaml
target: "zhang_shi"
type: "아내 → 휴서 후 별거 → 다시 만날 수 없는 사람"
axes: { trust: 95, affinity: 90, respect: 70, wariness: 5 }
bond_kind: null           # ★ 어떤 enum도 정확히 매핑 안 됨
bond_status: "Active"     # 장씨 생존
partnership: "Separated"  # ★ 결혼 후 휴서
```

> v0.5의 직교성 검증: BondKind: null + Partnership: Separated. axes가 깊으나 Soulmate 결이 아닌 *부부의 비극*. enum 강제 매핑 없이 자유 텍스트 type + Partnership으로 정확 표현.

### 5.5 수련 → 옥교룡 (Mentor, Reactivating)

```yaml
target: "yu_jiaolong"
type: "가르치려 했으나 따르지 않은 후배 → 변경에 살아있다는 단서"
axes: { trust: 60, affinity: 75, respect: 80, wariness: 50 }
bond_kind: "Mentor"        # ★ v0.5 신설 variant 첫 적용
bond_status: { Reactivating: { trigger: "current_rumor" } }
partnership: null
bond_since: "qingming_jian_chase 후 약 14일 유지된 시점"
```

> v0.5의 두 가지 신설 동시 검증: Mentor variant + Reactivating status.
> Mentor 임계 (trust ≥50, affinity ≥50, respect ≥60, wariness ≤60) 모두 충족.
> 수련이 옥교룡을 *가르치려 한* type_history 존재 — 추가 조건 충족.
> v0.4에서 분류 불가했던 관계가 v0.5에서 *비로소* 정확 분류.

### 5.6 수련 → 푸른여우 (ArchRival, Resolved)

```yaml
target: "bi_yan_huli"
type: "이모백의 사부의 원수 → 결판된 적 (사망)"
axes: { trust: -70, affinity: -90, respect: 70, wariness: 90 }
bond_kind: "ArchRival"
bond_status: { Resolved: { reason: "이모백의 복수로 처단" } }
partnership: null
bond_since: "이모백 사부 살해 사건"
```

> v0.5 검증: BondKind 그대로 유지 (정체성 영향 영구) + Status로 *현재 행동 트리거 불활성* 표시.

### 5.7 수련 → 맹사조 (정혼자 사망 — Partnership: Engaged + Deceased)

```yaml
target: "meng_sizhao"
type: "죽은 약혼자 — 평생 정절의 정표 (금비녀 = 그의 흔적)"
axes: { trust: 80, affinity: 70, respect: 75, wariness: 0 }
bond_kind: null           # 만남 짧아 임계 미달
bond_status: "Deceased"
partnership: "Engaged"    # ★ 정혼 상태로 사망
deceased_at: "meng_sizhao_death"
```

> v0.5 검증의 가장 강한 사례: BondKind는 null이지만 *Partnership + Status가 인물 정체성의 핵심*을 정확히 보존. v0.4에서는 "key_bonds vs formative 어디 둘까?" 한계가 있었으나, v0.5에서는 *현재 활성 슬롯이지만 Deceased status*로 명확히 표현.

---

## 6. 다음 단계

본 문서가 *정의하지 않는* 것 — 향후 별도 문서:

1. **동행(同行) 시스템** — 별도: `companions.md`.
2. **평판(評判) 시스템** — 별도: `reputation.md`.
3. **인연(因緣)·기연(奇緣) 트리거 룰** — Pillar 5 직결.
4. **자기희생/처단 행동 트리거** — BondKind × BondStatus별 행동 emit 룰.
5. **회상 OCC의 구체 메커니즘** — §4.5 골격만 있음. 어떤 사건·환경이 회상 트리거인가, PAD 영향의 강도와 지속, 추모 행동 트리거 조건 등.

본 문서가 발생시키는 **스키마 v0.5 보정 사항**:
- `key_bonds[]`에 `bond_status` 필드 추가 (5 variants enum)
- `key_bonds[]`에 `partnership` 필드 추가 (4 variants enum, Optional)
- `key_bonds[]`에 `deceased_at` 필드 추가 (Deceased status일 때만 사용)
- `BondKind`에 `Mentor` variant 추가 (8 → 9)
- 검증 체크리스트에 status·partnership 일관성 항목 추가

---

## 변경 이력

| 버전 | 일자 | 변경 |
|------|------|------|
| v0.3 | 2026-05-04 | 초안. 4축(직교+음수) + type/type_history + 4종류 지기 + OCC 매핑 + 검증 사례 |
| v0.4 | 2026-05-04 | BondKind 통합 (지기 4 + 원수 4 = 8). 진입·이탈 비대칭. zhiji_kind → bond_kind. |
| v0.5 | 2026-05-04 | **세 차원 직교화**: BondKind (9 — Mentor 추가) + BondStatus (5, 신설) + Partnership (4, 신설). 회상 OCC §4.5 신설. 임충·수련 검증의 모든 시스템 한계 해소: deceased 처리, 결판 후 처리, dormant 재활성화, romantic bond 분리, 인생 멘토 분류. Mentor 진입 14일 게이트 — v0.5의 진입 시간 차등 첫 사례. |
