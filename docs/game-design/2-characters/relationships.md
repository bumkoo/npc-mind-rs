# 관계 시스템 (Relationships)

> Version: 0.7 — 2026-05-09
> 위치: `docs/game-design/2-characters/relationships.md`
> 의존: `_schema.md` v0.6, `00-pillars.md` v0.1
> 동반 시스템: `action_triggers.md` v0.1 (★ v0.6 신설 — 본 문서에서 분류한 관계가 실제 행동 emit으로 이어지는 평가 시스템)
> 참조: `npc-mind-rs` OCC/PAD 엔진

## 0. 설계 원칙

이 문서는 Pillar 3 ("관계가 곧 시스템")의 *본체*다. 관계의 *구조와 분류*를 담당.
*행동 emit*은 별도 문서 `action_triggers.md`로 분리됨 (v0.6 신설).

명제:

1. **호감/적대 이분법은 거짓이다.** 한 사람을 신뢰하면서도 두려워할 수 있고, 존경하면서도 함께 있고 싶지 않을 수 있다.
2. **관계는 *상태*가 아니라 *형태*다.** "끊어졌다"는 없다. 모든 만남은 어떤 *형태*로든 존재한다.
3. **관계는 NPC가 *스스로* 갱신한다.** 디자이너 스크립트가 아닌 OCC 감정에서 유도.
4. **관계는 *세 차원*에서 동시에 존재한다.** 정서·기능적 분류(BondKind), 활동 상태(BondStatus), 형식적 동반(Partnership). 셋은 *직교*.
5. **★ v0.6 추가: 분류와 실행은 분리된다.** 본 문서는 *분류*까지. *실행 가능성·행동 emit*은 `action_triggers.md`.
6. **★ v0.7 추가: LLM과 Engine은 다른 일을 한다.** 서사적 의미·선언·요약은 LLM이, 정량 격동도·시간 누적·외부 사건 전파는 Engine이 평가. 자세한 책임 매트릭스는 §6.5.

---

## 1. 4축 — Trust · Affinity · Respect · Wariness

### 1.1 각 축의 정의

| 축 | 측정 대상 | 한 줄 질문 | 범위 |
|---|---|---|---|
| **trust** | 말과 행동의 일치도 | "등을 보일 수 있는가?" | -100 ~ +100 |
| **affinity** | 정서적 거리 | "혼자 있을 때 그리운가?" | -100 ~ +100 |
| **respect** | 판단·능력·인격 평가 | "이 사람의 의견을 따를 만한가?" | -100 ~ +100 |
| **wariness** | 위협 인식 | "무방비로 마주할 수 있는가?" | 0 ~ +100 |

### 1.2 직교성

4축은 *직교*. 한 축이 다른 축을 자동 결정하지 않음.

흥미로운 패턴 예: trust↑+wariness↑ (예측 가능하지만 위험), respect↑+affinity↓ (위대하나 거리감), affinity↑+trust↓ (그리우나 의지 못함), respect↑+affinity↓+wariness↑ (적이지만 인정 — 숙적).

### 1.3 음수의 의미

음수는 *적극적 반대 인식*.
- trust = -50: *확신을 가지고* 의심.
- affinity 음수: 혐오.
- respect 음수: 경멸.
- wariness는 음수 없음 (0이 완전 무방비의 극값).

### 1.4 갱신 빈도와 점착성

- **즉각 갱신**: 한 사건당 ±1~30 (OCC 감정 기반).
- **주기 감쇠**: 큰 변화는 시간 흐르며 평균값으로 수렴. transformation_event 기록 변화는 감쇠 없음.
- **양극의 점착성**: ±100 도달 시 추가 입력에도 머무름.
- **사망/결판 후 점착**: `bond_status: Deceased`/`Resolved` 관계는 axes freeze. OCC 입력 차단. 회상 OCC만 별도 처리(§4.5).

### 1.5 ★ v0.6 신설: compass 변화 후 axes 자연 누적 룰

NPC의 compass(`inner_compass.compass`)가 큰 사건으로 변화할 때, *모든 key_bond의 axes를 일괄 재평가*해야 하는가?

**v0.6 결론: 자연 누적으로 충분. 일괄 재평가 *불필요*.**

근거:
- 같은 사건이 compass를 바꿨다면, 그 사건의 OCC 감정이 *이미* 관련 key_bonds의 axes를 갱신했음 (§4 흐름).
- compass 변화 자체는 *자기 가치관의 재정립*이지 *타인에 대한 인식의 즉각 재조정*이 아님.
- 시간이 흐르며 새 compass 하에서 누적되는 OCC가 axes를 *서서히* 그 방향으로 끌어감 → 자연 누적.

검증:
- 임충 산신묘 사건: compass "체제 순응 → 체제 저항"으로 변화. 그러나 *기존 key_bonds*의 axes는 산신묘 OCC만으로 갱신 (육겸 -100 등). 다른 인물에 대한 *재평가*는 자연 누적으로 처리.
- 수련 li_mubai_death: compass 변화. 그러나 옥교룡·맹사조 axes 재평가 없이 자연 누적으로 충분히 표현.

**예외 (디자이너 명시 시)**: compass 변화가 *과거의 인식 자체를 재해석*하게 만드는 드문 경우. 예: "그 사람이 옳았다"는 깨달음으로 과거 적의 axes가 재평가될 수도. 이건 transition_point의 `impact`에 *명시적 axes 재평가 사건*으로 기록 — 자동이 아닌 *수동* 트리거.

---

## 2. 관계의 형태 — type과 type_history

### 2.1 type — 자유 텍스트 한 줄

`type`은 *현재* 관계의 형태를 한 줄로 기술. 4축 수치와 별개.

### 2.2 type_history

```yaml
type_history:
  - { since: "age 7~10",            type: "은인" }
  - { since: "age 10~24",           type: "주인·아버지" }
  - { since: "tp_3_master_falls",   type: "주인·동지" }
```

`since`는 자유 텍스트.

### 2.3 transformation_events

```yaml
transformation_events:
  - { event_id: "tp_4_liangshan", new_type: "주인·동지" }
```

### 2.4 dormant_bonds

만난 적은 있으나 *한 번도 활성화된 적 없는* 잠재 관계. 한 번 활성이었다가 비활성된 관계는 `key_bonds[].bond_status: Dormant/Reactivating`로 처리.

```yaml
dormant_bonds:
  - target: "어린 시절의 누군가 (구체 미정)"
    last_contact: "age 5~7"
    fragment: "안개 속 누군가의 손길..."
    note: "기연 후보."
```

dormant_bond의 *영향력*은 활성화될 수 있다 (사건에서 *떠올라* 새 key_bond에 전달). 그러나 dormant_bond *자체*는 새 key_bond를 자동 생성하지 않음 (수련 노년기의 무명 여검객 사례).

---

## 3. 관계의 세 차원 — BondKind · BondStatus · Partnership

```rust
pub struct Relationship {
    pub bond_kind:   Option<BondKind>,     // 정서·기능적 분류 (지기/원수/멘토/양육자/동반자)
    pub bond_status: BondStatus,           // 활동 상태
    pub partnership: Option<Partnership>,  // 형식적 동반
}
```

### 3.0 차원의 구분 원칙

| 차원 | 무엇을 표현하는가 | 변화의 동력 |
|---|---|---|
| **BondKind** | 정서·기능적 분류 | OCC 감정 누적 → axes 변화 → 임계 도달/이탈 |
| **BondStatus** | 현재 활동 상태 | 사건 (사망·결판·재발견) |
| **Partnership** | 형식적 동반 | 공식 사건 (결혼식·이혼 등) |

### 3.1 BondKind — 11 variants (★ v0.6: Guardian + Companion 추가)

```rust
pub enum BondKind {
    // 지기·동반 — 양극 임계
    SwornBrothers,    // 의형제·동지형
    MasterDisciple,   // 사부-제자형 (무술 비전 전수)
    Soulmate,         // 영혼의 동반자형
    LoyalRetainer,    // 가신·은인형
    Companion,        // 평생의 우인 (★ v0.6 신설)
    Guardian,         // 부모-자녀형 (★ v0.6 신설)
    // 멘토 — 중간극 임계
    Mentor,           // 인생 선배·후배
    // 원수 — 음극 임계
    BloodEnemy,       // 혈적
    ArchRival,        // 숙적
    Betrayer,         // 배신자
    Oppressor,        // 압제자
}

impl BondKind {
    pub fn is_zhiji(&self) -> bool { /* SwornBrothers..LoyalRetainer */ }
    pub fn is_companion_class(&self) -> bool { /* Companion */ }
    pub fn is_guardian(&self) -> bool { /* Guardian */ }
    pub fn is_mentor(&self) -> bool { /* Mentor */ }
    pub fn is_enemy(&self) -> bool { /* BloodEnemy..Oppressor */ }
}
```

#### 지기 4종류 (v0.5 유지)

##### SwornBrothers — 의형제·동지형
```yaml
임계값: { trust ≥+80, affinity ≥+70, respect ≥+60, wariness ≤30 }
자기희생: "함께 싸우고 함께 죽음." 동귀어진(同歸於盡).
```

##### MasterDisciple — 사부-제자형
```yaml
임계값: { respect ≥+90, trust ≥+70, affinity ≥+50, wariness ≤40 }
자기희생: "비급·심법 전수, 명예를 넘김." 후계자 지정.
특이점: ★ 무술 비전 전수가 핵심. 비급 없으면 Mentor 또는 Guardian.
```

##### Soulmate — 영혼의 동반자형
```yaml
임계값: { affinity ≥+90, trust ≥+80, respect ≥+70, wariness ≤20 }
자기희생: "침묵 속의 결단. 상대를 위해 자기 길을 바꿈."
특이점: Partnership과 *직교*. 부부일 수도, 미발현일 수도.
```

##### LoyalRetainer — 가신·은인형
```yaml
임계값: { trust ≥+90, respect ≥+85, affinity ≥+80, wariness 임계 없음 }
자기희생: "주인의 명예를 위해 자기 신분·미래·생명을 도구로 씀."
```

#### Companion — 평생의 우인 (★ v0.6 신설)

> 노년기 수련-유태보. 신분 차이를 가로지르는 *깊은 우정*.

```yaml
임계값:
  trust:    ≥ +75
  affinity: ≥ +65
  respect:  ≥ +50
  wariness: ≤ 30
자기희생 형태: "*신뢰*로 함께 가나 *생사*는 따로." 깊은 우정의 자기희생은 *위로·증언·기억*.
대표 행동:
  - 곤란 시 *반드시* 도우러 옴 (단 자기 목숨 걸지는 않음 — 이게 SwornBrothers와 차이)
  - 죽음 후 그를 *기억하고 증언함* (장년기 후 노년기까지 우정이 이어지는 핵심)
  - 신분·계층 차이를 *가로지름* (사대부 ↔ 평민 같은)
특이점:
  - SwornBrothers와 차이: 동귀어진 *없음*. wariness 임계 더 관대 (30 vs 30 동일하나 자기희생 결이 다름).
  - SwornBrothers는 *형제*, Companion은 *친구*. 핏줄 의식 없음.
  - 임계 도달 자동 진입 가능하나 *디자이너 재량*도 인정 — 같은 axes에서 자유 텍스트 type 선택 가능.
진입: 연속 30일 (SwornBrothers와 동일).
이탈: 즉시.
```

#### Guardian — 부모-자녀형 (★ v0.6 신설)

> 노년기 수련-춘설병. 양육 + 보호 + 가르침. 친·양 무관.

```yaml
임계값:
  trust:    ≥ +70
  affinity: ≥ +80   # ★ 핵심 — 모성·부성의 정서적 깊이
  respect:  무관    # 어린 자녀에 대한 압도적 존경 부재. 자질 인정만으로 충분
  wariness: ≤ 30
자기희생 형태: "자녀를 위한 모든 희생. 자기 미래·생명까지." 가족 결단.
대표 행동:
  - 자녀 위한 위험 감수 (자기 안전 후순위)
  - 자녀가 잘못된 길에 갈 위험 *시*에는 가르침을 시도. 듣지 않으면 *지켜봄* (compass의 "가두지 않는다"와 결합 가능)
  - 자녀의 *마지막을 지킴*. 또는 자녀가 자기 마지막을 지킴.
특이점:
  - MasterDisciple과 차이:
    * 비급 전수 *없음* (필수 아님 — 무술 가르침이 부수적이지 핵심 아님)
    * respect 임계 무관 (자녀에 대한 존경은 자질 인정 수준이지 압도적이지 않음)
    * 양육·보호 본질
  - SwornBrothers와 차이: 비대칭. 양육자가 위.
  - Mentor와 차이: 가족 형식 (Mentor는 가족 무관, 비대칭이지만 더 거리감).
  - Companion과 차이: 가족 결합 + 자기희생 강도 ↑.
진입: ★ **연속 7일** (가족 형성은 빠름. 양녀화 결정 후 며칠이면 부모 정체성 형성).
이탈: 즉시.
변형:
  - 친자녀 vs 양자녀 vs 후견 자녀 — 모두 Guardian. 차이는 type 자유 텍스트로.
  - 부모 → 자녀 방향: Guardian.
  - 자녀 → 부모 방향: 별도 분류 필요할 수도 있으나 v0.6에서는 Mentor 또는 LoyalRetainer로 흡수 가능 (정서적 깊이 + 존경/가신 결).
```

#### Mentor — 인생 선배·후배 (v0.5 유지)

```yaml
임계값: { trust ≥+50, affinity ≥+50, respect ≥+60, wariness ≤60 }
+ 추가 조건: type_history에 "가르치려 함" 의미 존재.
자기희생: "자기 시간·평판·미래를 후배의 길에 투자."
진입: 연속 14일.
```

#### 원수 4종류 (v0.5 유지)

##### BloodEnemy
```yaml
임계값: { trust ≤-80, affinity ≤-80, respect 무관, wariness ≥+70 }
행동 트리거: "추적·매복·즉결 처단."
```

##### ArchRival
```yaml
임계값: { trust 무관, affinity ≤-50, respect ≥+60, wariness ≥+60 }
행동 트리거: "공정 결투·결판."
```

##### Betrayer
```yaml
임계값: { trust ≤-70, affinity ≤-50, respect ≤-40, wariness ≥+70 }
+ type_history에 *이전의 가까운 type* 필수.
```

##### Oppressor
```yaml
임계값: { trust ≤-40, affinity ≤-50, respect -20~+30, wariness ≥+80 }
행동 트리거: "체제 자체에 저항."
```

### 3.2 진입·이탈 룰 (v0.6 갱신)

| 종류 | 진입 | 이탈 |
|---|---|---|
| 지기 (SwornBrothers/MasterDisciple/Soulmate/LoyalRetainer) | 연속 30일 | 즉시 |
| **Companion** ★ | 연속 30일 | 즉시 |
| **Guardian** ★ | **연속 7일** | 즉시 |
| 멘토 (Mentor) | 연속 14일 | 즉시 |
| 원수 (BloodEnemy/ArchRival/Betrayer/Oppressor) | 즉시 | 임계 위 회복 후 연속 30일 |

진입 시간 차등의 의미:
- **Guardian 7일** = 가족 형성은 빠름 (양녀 결정 = 며칠이면 부모 됨)
- **Mentor 14일** = 인생 가르침은 짧은 동행으로도 형성
- **지기·Companion 30일** = 깊은 신뢰는 일상의 검증을 요구
- **원수 즉시** = 적의는 사건이 만든다

### 3.3 다중 BondKind

한 NPC는 여러 BondKind 보유 가능. 종류는 다른 게 자연. 같은 종류 복수는 *내적 갈등*의 씨앗.

### 3.4 BondKind 비대칭

A → B와 B → A의 bond_kind가 다를 수 있다 (Mentor 사례 — 가르치려 한 자와 거부한 자).

### 3.5 BondStatus — 5 variants (v0.5 유지)

```rust
pub enum BondStatus {
    Active,
    Resolved { reason: String },
    Deceased,
    Dormant,
    Reactivating { trigger: EventId },
}
```

- `Resolved`/`Deceased`는 *terminal*. axes freeze.
- `Dormant`는 복귀 가능 (Reactivating 거쳐 Active로).
- 자세한 의미는 v0.5 §3.5와 동일.

### 3.6 Partnership — 4 variants (v0.5 유지)

```rust
pub enum Partnership {
    Spouse, Engaged, Lover, Separated,
}
```

- BondKind와 *완전 직교*.
- axes와 직접 연동되지 않음 (정략결혼 = trust 0 + Spouse 가능).
- 변화 동력은 *공식 사건*.

---

## 4. OCC 감정 → 4축 변화 매핑

### 4.1 변화 함수 (v0.5 유지)

```rust
pub fn update_axes_from_emotion(rel: &mut Relationship, emotion: OccEmotion, intensity: f32, npc_hexaco: &Hexaco) {
    if !rel.bond_status.accepts_live_input() { return; }   // Deceased/Resolved/Dormant 차단
    let base = base_delta(emotion);
    let modulator = hexaco_modifier(emotion, npc_hexaco);
    let delta = base * intensity * modulator;
    rel.trust    = (rel.trust    + delta.trust   ).clamp(-100.0, 100.0);
    rel.affinity = (rel.affinity + delta.affinity).clamp(-100.0, 100.0);
    rel.respect  = (rel.respect  + delta.respect ).clamp(-100.0, 100.0);
    rel.wariness = (rel.wariness + delta.wariness).clamp(   0.0, 100.0);
}
```

### 4.2 base_delta 표 (v0.5 유지)

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

### 4.3 HEXACO 보정자 (v0.5 유지)

| 특성 | 보정 |
|---|---|
| H+ Sincerity 높음 | trust 변화 ×1.2 |
| A+ Patience 높음 | 모든 변화 ×0.7 |
| A- Forgiveness 낮음 | 부정 감정 변화 ×1.5 |
| E+ Anxiety 높음 | wariness 변화 ×1.3 |
| C+ Prudence 높음 | 큰 변화 시 ×0.8, 시간 분산 |
| O+ Unconventionality 높음 | 양극 도달 더 쉬움 |

### 4.4 통합 흐름 — npc-mind-rs 연결

사건은 *시간 스케일*이 다른 두 종류의 처리를 통과한다. **Inner Loop**는 *대화 턴마다*, **Outer Loop**는 *장면 종료 시* 동작.

> **v0.7 정정 노트**: v0.6까지 본 절은 한 줄 흐름으로 표현됐으나 부정확했음. turn 단위 처리(appraise/apply_stimulus)와 scene 단위 처리(axes/BondKind/BondStatus)를 한 흐름에 섞은 그림이었음. base_delta 표(§4.2)의 ±10~25 값을 매 턴 적용하면 5턴이면 양극 도달 — 무협 서사의 시간감과 어긋남. v0.7에서 두 루프 분리. 분리 근거는 §6.1.

#### Inner Loop (대화 턴 단위)

```
대화 턴 발화 (Turn Event)
  ↓
appraise()              [domain]   → OccEmotion + intensity (다수)
  ↓
[listener_pad_convert]  [domain]   → 청자 시점 PAD (Phase 7)
  ↓
apply_stimulus()        [domain]   → PAD 갱신
  ↓
ActingGuide::build()    [domain]   → LLM 연기 가이드
  ↓
[LLM 발화 → 다음 turn]
```

— Inner Loop 동안 *axes는 동요하지 않음*. PAD만 갱신. (왜? §6.1)

═══ Scene Boundary (after_dialogue) ═══

##### Reflection 단계 (§6.2)

LLM이 대화의 *서사적 의미*를 평가, 엔진이 *정량 significance*를 결정론으로 계산.
LLM이 "잡담"으로 판단하면 Outer Loop *진입 안 함* — 대화는 메모리에 요약만 저장하고 종료.

#### Outer Loop (장면 단위, 조건부)

```
누적 OCC 응축 (top-K peak intensity)        → 장면 대표 감정 묶음
  ↓
RelationshipUpdater     [domain]   → bond_status 검사 후 4축 갱신
  ↓
type_history 갱신                  → ① Channel 1 Declarative (LLM 식별)
                                     ② Channel 3 External   (event bus)
  ↓
Partnership 갱신                   → Channel 1 우선 (의례·선언)
  ↓
BondKind 임계 + 시간 게이트         → Channel 2 Temporal (카운터 read model)
  → BondKindEntered/Exited 도메인 이벤트 emit
  ↓
BondStatus 자동 전환               → Channel 3 우선 (사망·결판)
  → BondStatusChanged 도메인 이벤트 emit
  ↓
★ ActionTriggerEvaluator [domain]  → action_triggers.md §5의 룰 적용
  → 실행 가능성 평가 후 ActionTriggered 이벤트 emit
```

본 문서는 *Outer Loop의 ActionTriggerEvaluator 진입 직전*까지. ActionTriggerEvaluator 본체는 `action_triggers.md` 책임.

3-channel transformation/partnership trigger의 자세한 정의는 §6.4. Reflection LLM 입출력 schema는 §6.2.

### 4.5 회상 OCC — Deceased/Resolved 관계 (★ v0.6 구체화)

#### 4.5.1 회상 OCC의 정의

`bond_status: Deceased` 또는 `Resolved`인 관계에서, NPC가 *상대를 떠올리는* 사건이 발생할 때 emit되는 OCC. 새 입력이 *없는데도* 감정이 발생.

핵심 원칙:
- **axes는 변경하지 않음**. 관계는 freeze.
- **PAD에는 일시적 영향**. 며칠간 슬픔·기쁨 등.
- **강한 회상은 *행동 트리거* 가능** — 추모 의식, 옛 장소 방문, 묘비 손질.

#### 4.5.2 회상 트리거 — 어떤 사건·환경이 회상 OCC를 발생시키는가

5가지 회상 트리거 분류:

```rust
pub enum RecollectionTrigger {
    /// 1. 환경 단서: 옛 장소·물건·계절·시간을 마주침
    EnvironmentalCue { cue: String, similarity: f32 },
    /// 2. 비슷한 인물: 새 만남이 deceased 상대와 닮음 (외모/말투/행동)
    SimilarPerson { target_id: NpcId, similarity_axis: String, score: f32 },
    /// 3. 중요 일자: 사망일·기일·결혼기념일 등
    SignificantDate { kind: String, days_since_event: i32 },
    /// 4. 꿈·무의식: 잠자리·명상 시 자발 발생
    Spontaneous { dream: bool },
    /// 5. 외부 호출: 다른 NPC가 그 인물을 언급
    ExternalMention { mentioner: NpcId },
}
```

각 트리거의 강도 차이:
- EnvironmentalCue: similarity 비례 (0.0~1.0). 옛 장소 *그곳*은 1.0, 비슷한 풍경은 0.5.
- SimilarPerson: score 비례. 외모/말투/행동의 유사도.
- SignificantDate: 일자 정확도 비례. 정확한 기일 1.0, 그 달 0.5.
- Spontaneous: 무작위 0.3~0.7 사이. NPC E+ Sentimentality에 비례.
- ExternalMention: 0.4~0.8. mentioner와의 관계에 비례.

#### 4.5.3 회상 OCC 강도 계산

```rust
pub fn compute_recollection_intensity(
    rel: &Relationship,
    trigger: &RecollectionTrigger,
    npc: &Npc,
    time_since_event: Days,
) -> f32 {
    let base_strength = trigger.base_strength();              // 0.0~1.0
    let bond_depth = rel.bond_kind.depth_score();             // BondKind별 깊이 점수
    let axes_magnitude = rel.axes.magnitude_at_freeze();      // axes 절대값 평균
    let time_decay = (1.0 / (1.0 + time_since_event.years() * 0.1)).max(0.3);
    let sentimentality = npc.hexaco.E_emotionality.sentimentality / 100.0;

    base_strength * bond_depth * axes_magnitude * time_decay * (0.5 + sentimentality * 0.5)
}
```

요소별 의미:
- `bond_depth`: SwornBrothers/Soulmate = 1.0, Companion/Guardian = 0.8, MasterDisciple/Mentor = 0.7, Resolved 적 = 0.5
- `axes_magnitude`: |trust| + |affinity| + |respect| + wariness 평균. 깊은 관계일수록 강한 회상.
- `time_decay`: 시간 흐를수록 약화. 단 *바닥은 0.3* (점착성 룰 — 영원히 사라지지 않음).
- `sentimentality`: NPC 감수성 — 같은 트리거라도 인물에 따라 강도 다름.

검증 예시 (수련 노년기, 이모백 사후 13~17년):
- bond_depth: Soulmate = 1.0
- axes_magnitude: (95 + 95 + 95 + 5) / 4 / 100 = 0.725
- time_decay: 1.0 / (1.0 + 15 * 0.1) = 0.4
- sentimentality: 90 / 100 = 0.9
- 최종 (옛 장소 풍경 만남, base 0.5): 0.5 × 1.0 × 0.725 × 0.4 × (0.5 + 0.9*0.5) = 약 0.14

→ 강하지 않은 OCC. 며칠간 가벼운 슬픔. *깊지만 동요 작은* 정확한 표현.

#### 4.5.4 PAD 영향과 지속

회상 OCC가 발생하면:

```rust
pub struct RecollectionEffect {
    pub pad_delta: PadVector,
    pub duration_days: u32,
    pub triggers_action: Option<RecollectionAction>,
}
```

- **pad_delta**: 회상 OCC가 *미러링*하는 원래 OCC의 PAD 영향. 다만 강도는 위 식으로 약화.
- **duration_days**: 강도 비례. 0.1 미만 → 1일, 0.3 미만 → 3일, 0.5 미만 → 7일, 그 이상 → 14~30일.
- **triggers_action**: 강도 0.5 이상에서 *추모 행동* 후보 emit.

#### 4.5.5 추모 행동 트리거

```rust
pub enum RecollectionAction {
    VisitGrave,                    // 묘소 방문
    VisitMeaningfulPlace,          // 옛 장소 방문
    HandleHeirloom,                // 유품·정표 만짐
    SilentMonologue,               // 침묵의 혼잣말 (수련의 금비녀)
    SpeakOfThemToOthers,           // 다른 NPC에게 그 인물 이야기
}
```

선택 기준은 BondKind + 강도 + 환경:
- Soulmate + 강도 0.7+ → SilentMonologue 또는 HandleHeirloom 선호
- SwornBrothers + 강도 0.7+ → SpeakOfThemToOthers 선호 (형제는 *기억하고 증언함*)
- Guardian + 강도 0.7+ → VisitGrave + HandleHeirloom

추모 행동의 *실제 emit*은 ActionTriggerEvaluator (`action_triggers.md`) 책임. 본 문서는 *후보*까지.

#### 4.5.6 회상 OCC와 axes의 관계 — 비대칭

회상 OCC는 axes를 변경하지 않으나, 매우 강한 회상이 *반복적으로* 일어나면 PAD 누적이 NPC 일상 상태에 영향. 이게 "잊지 못함"의 시스템적 표현.

단 axes 자체는 영구 freeze — 죽은 자에 대한 신뢰가 *늘어나거나 줄어들지 않음*. 신뢰는 *살아있는 사람 사이*의 변수.

---

## 5. 검증 사례 — v0.6 적용

### 5.1 노년기 수련 → 춘설병 (Guardian + Active)

```yaml
target: "chun_xue_bing"
type: "양녀이자 후계자"
axes: { trust: 75, affinity: 90, respect: 60, wariness: 25 }
bond_kind: "Guardian"        # ★ v0.6 — 임시 처방(MasterDisciple) 해소
bond_status: "Active"
partnership: null
bond_since: "first_lesson 후 7일 도달 시점"   # ★ Guardian 진입 7일 게이트
```

> v0.5에서 MasterDisciple 임시 처방 + respect 임계 미달 한계가 *완전 해소*.
> Guardian 임계 충족: trust 75 ≥+70 ✓, affinity 90 ≥+80 ✓, respect 무관 ✓, wariness 25 ≤30 ✓.
> 자기희생 형태가 *후계자 지정* + *양육 본질* 결합.

### 5.2 노년기 수련 → 유태보 (Companion + Active)

```yaml
target: "liu_taibao"
type: "북경 시정의 의리 있는 친구 — 평생의 우인"
axes: { trust: 80, affinity: 70, respect: 60, wariness: 20 }
bond_kind: "Companion"       # ★ v0.6 — 자유 텍스트 type 보류 해소
bond_status: "Active"
partnership: null
bond_since: "약 30년 일상 우정 누적 — 노년기 시점에 자연 진입"
```

> v0.5에서 SwornBrothers 임계 *근접*하나 *형제 결*과 다른 평민 우정으로 null + 자유 텍스트 처리한 한계가 *해소*.
> Companion 임계 충족: trust 80 ≥+75 ✓, affinity 70 ≥+65 ✓, respect 60 ≥+50 ✓, wariness 20 ≤30 ✓.
> 자기희생 형태가 *동귀어진 없이* 깊은 신뢰 — 정확한 결.

### 5.3 회상 OCC 작동 — 노년기 수련의 이모백 회상

수련이 노년에 옛 객점 (이모백과 마지막으로 함께 갔던 곳)을 지날 때:

```yaml
trigger: { EnvironmentalCue: { cue: "옛 객점", similarity: 0.7 } }
계산:
  - base_strength: 0.7 (similarity 비례)
  - bond_depth: 1.0 (Soulmate)
  - axes_magnitude: 0.725 (95+95+95+5 평균)
  - time_decay: 0.4 (15년 경과)
  - sentimentality: 0.9 (E+ Sentimentality 90)
  - 최종 강도: 0.7 × 1.0 × 0.725 × 0.4 × (0.5 + 0.9*0.5) = 0.193

결과:
  - PAD 영향: 일시적 pleasure -0.2, arousal -0.1, dominance -0.1 (가벼운 슬픔)
  - duration_days: 3일 (0.3 미만이므로 3일)
  - triggers_action: None (0.5 미달)
```

axes 변화 없음. 며칠간 PAD 가벼운 슬픔. 추모 행동 emit 안 함. *깊지만 동요 작은* 정확한 표현.

기일 (li_mubai_death의 정확한 일자) 만남 시:
- trigger: SignificantDate { kind: "기일", days_since_event: 0 } → base 1.0
- 강도: 1.0 × 1.0 × 0.725 × 0.4 × 0.95 = 0.275
- duration: 7일
- triggers_action: None (0.5 미달이지만 강도가 임계 가까움 — HandleHeirloom 후보 등록)

수련이 *기일에 금비녀를 손에 쥔다* — 자연 행동이 시스템에서 도출됨.

---

## 6. 장면 경계 리플렉션 (★ v0.7 신설)

### 6.0 왜 별도 절인가

§4까지 정의한 *4축 갱신* 흐름은 한 가지 가정 위에 서 있다 — *사건이 발생하면 axes가 갱신된다*. 그러나 npc-mind-rs는 대화 턴마다 OCC를 emit한다. base_delta 표(§4.2)의 값을 *매 턴* 적용하면 양극 도달이 너무 빠르다. 무협의 시간감과 어긋남.

이 어긋남은 v0.6까지 §4.4 흐름도가 *암묵적으로 가정한* scope 분리를 명시화하지 않은 데서 왔다. v0.7은 이 분리를 *문서의 1급 시민*으로 격상한다.

### 6.1 Inner/Outer 루프 분리 원칙

| | Inner Loop | Outer Loop |
|---|---|---|
| **시간 단위** | 대화 턴 (1 발화) | 장면 (after_dialogue) |
| **건드리는 것** | PAD, ActingGuide | axes, BondKind, BondStatus, Partnership, type/type_history |
| **빈도** | 분당 ~10회 | 시간당 ~1회 |
| **목적** | 자연스러운 *연기* | 서사적 *결산* |
| **LLM 호출** | ActingGuide 생성용 | Reflection 판정용 |

핵심 명제:

1. **Inner Loop은 axes를 건드리지 않는다.** PAD가 격렬하게 진동해도 관계 평가는 안정. NPC가 한 장면 안에서 자연스럽게 흔들리되 *상대를 어떻게 보는지*는 흔들리지 않는다.
2. **Outer Loop은 장면 단위로 결산한다.** 누적된 OCC를 응축, axes에 한 번 반영. 이때 *서사적 사건*(transformation/partnership/사망 등)이 함께 처리됨.
3. **두 루프의 데이터는 한 방향으로 흐른다.** Inner → Outer (turn OCC 누적 → 응축). Outer → Inner는 *다음 장면 시작* 시점에만 (새 axes/BondKind 상태가 다음 ActingGuide의 입력).

### 6.2 Scene Boundary와 Reflection 단계

Inner Loop의 마지막 turn 후 `after_dialogue` 호출 시점이 Scene Boundary. 이 시점에 **Reflection 단계**가 동작:

```
after_dialogue 호출
  ↓
─── Reflection (LLM + Engine 협업) ───
LLM 입력:
  - turn-level OCC 누적 리스트
  - 대화 transcript
  - NPC: compass / taboo / life_question / 현재 PAD
  - 대상 NPC 정보 / 현재 BondKind / axes

LLM 출력 (structured JSON):
  - is_chitchat: bool
  - summary: "1~2문장"
  - declarative_events: [{ kind, target, text, reasoning }]
  - partnership_event:  Option<{ kind, reason }>

엔진 계산 (LLM과 무관, 결정론):
  - significance_score (§6.3)
  - external_events    (event bus 조회)
  - temporal_signals   (BondKind 카운터 read model)
─────────────────────────────────────

분기:
  is_chitchat && significance_score < 0.3
       → 메모리에만 요약 저장, Outer Loop skip
  그 외
       → Outer Loop 진입
```

Reflection 결과는 `DialogueReflected` 도메인 이벤트로 박제 — replay 시 LLM 재호출 없이 저장된 판단을 사용. ES/CQRS의 결정성 보장.

### 6.3 Engine-computed Significance

Significance는 *대화의 객관적 격동도*를 측정하는 점수. **LLM이 아니라 엔진이 계산**한다. 이유: LLM이 자기 점수를 자기가 매기면 *transformation_event 검증의 가드레일*로 쓸 수 없다 (순환 논리).

엔진은 turn-level OCC/PAD 신호 *4가지*를 가중 합산:

| 신호 | 가중치 | 의미 |
|---|---|---|
| `peak_occ_intensity` | 0.40 | 가장 격렬했던 한 순간의 OCC 강도. 짧지만 깊은 격발이 평생을 바꾼다. |
| `pad_trajectory_magnitude` | 0.30 | 대화 동안 PAD가 출렁인 누적 진폭. 잔잔함 vs 계속 흔들림. |
| `occ_diversity` | 0.15 | 등장한 distinct OCC type 개수. 단색 vs 복합. |
| `beat_signal` | 0.15 | Beat 전환 발생 여부. 디자이너가 시나리오에서 의도한 굴곡. |

```rust
fn compute_significance(turns: &[TurnSnapshot]) -> f32 {
    let peak_occ = turns.iter()
        .flat_map(|t| t.occ_emotions.iter().map(|e| e.intensity))
        .fold(0.0f32, f32::max);
    let pad_magnitude = (turns.windows(2)
        .map(|w| (w[1].pad - w[0].pad).magnitude())
        .sum::<f32>()
        .min(2.0)) / 2.0;
    let diversity = (turns.iter()
        .flat_map(|t| t.occ_emotions.iter().map(|e| e.kind))
        .collect::<HashSet<_>>().len() as f32 / 5.0).min(1.0);
    let beat_signal = if turns.iter().any(|t| t.beat_changed) { 1.0 } else { 0.0 };

    (peak_occ * 0.40
       + pad_magnitude * 0.30
       + diversity * 0.15
       + beat_signal * 0.15).clamp(0.0, 1.0)
}
```

가중치는 *디자인 파라미터* — 검증 사례로 tuning. 핵심은 모든 입력이 turn 버퍼에서 오는 결정론적 값이라는 것. 같은 대화 replay 시 동일 점수.

### 6.4 3-channel Transformation/Partnership Trigger

`transformation_event`(type 변화)와 `partnership_event`(Partnership 변화)는 *감정 격동의 함수가 아니다*. 더 큰 맥락의 함수다. 무협 서사의 transformation 사례를 보면:

| 사건 | 어떻게 일어나는가 |
|---|---|
| 이모백-수련 → Soulmate | 30년 함께 무공 닦음. *시간 누적*의 결과 |
| 임충-노지심 → SwornBrothers | 야저림 구출 자리에서 "형 동생" *호칭 선언* |
| 곽정-황용 → Spouse | 결혼식 의례 (연애결혼). *형식적 의례*가 사건 발생 시점 |
| 무송 → 반금련 BloodEnemy | 형 시신 발견. *외부 사건* 트리거 |

→ 격동만 측정해서는 위 사례 중 어느 것도 못 잡거나 일부만 잡는다. v0.7은 transformation/partnership 트리거를 **3개 독립 채널**로 분리:

#### Channel 1: Declarative (선언·의례)

- **언제 발화되나**: 대화 안에서 *형식적 사건*이 일어남. 결혼식, 의형제 결연, 사부 입문, 봉작, 원수 선언.
- **누가 식별하나**: LLM. 대화 텍스트에서 declarative speech act 추출. Reflection의 `declarative_events` 출력 슬롯.
- **감정 격동 무관**: 정략혼은 PAD 격동 거의 없어도 Spouse 발생.
- **게이트**: 사회적 일관성 검증 + 적용 모드 정책.

##### 사회적 일관성 검증 — 5 카테고리

LLM이 `declarative_events`/`partnership_event`를 emit했을 때 엔진이 통과시키기 전 5가지 검증을 수행:

| 카테고리 | 무엇을 보는가 | 한 줄 예 |
|---|---|---|
| **A. Structural** | 이미 그 상태? 동시 공존 불가? | 이미 Spouse인데 또 Spouse → reject |
| **B. Precondition** | 현재 상태에서 그 상태로 *전이 가능*? | None → Separated → reject (없는 관계 분리 불가) |
| **C. BondStatus Block** | 활동 상태가 변화 *차단*? | Deceased → 새 Partnership 형성 불가 |
| **D. Mutuality** | A→B 적용 시 B→A에도 적용? | Partnership은 양방향, BondKind는 보통 단방향 |
| **E. Domain Knowledge** | type 변환이 서사적으로 자연스러운가? | "양녀" → "스승" 부자연 (자유 텍스트라 무한 조합) |

A~D는 *enum/상태 비교*로 결정론 검증 가능. E는 *자유 텍스트 의미*라 결정론 불가 — 적용 모드(아래)로 통제. 자세한 룰은 Phase 2 구현 spec.

##### 적용 모드 — 점진적 디자이너 제어

라이브러리는 *이벤트 단위*로 디자이너가 적용 모드를 *옵트인*할 수 있다. 원칙: **글이 없으면 LLM 자유**. 디자이너는 통제하고 싶은 부분만 쓴다.

4단계 점진 통제:

| Tier | 작성 분량 | LLM 자유도 | 용도 |
|---|---|---|---|
| **0. 무설정** | 0줄 | 100% (default) | 일상 대화·보조 사건. 시나리오 default policy(보통 `audit`)에 따라 emit + 로그 |
| **1. 모드만** | `mode: ...` 한 줄 | 모드별 | 꿈·환상 장면(`reject`) 같은 의지 표명 |
| **2. Alternatives** | mode + 대안 셋 | 셋 안에서 선택 | 작가가 plot 분기를 명시 |
| **3. + Hints** | 위 + `reasoning_hint` | guided 선택 | 핵심 plot — 매칭 정확도 ↑ |

세 모드 (allowlist/audit/reject)와 alternatives 안의 대안은 *모두 옵션*. 디자이너는 플롯 핵심에만 깊이 쓰고 그 외는 무설정으로 둔다.

**Default policy** — 시나리오의 `default_transformation_policy`로 미설정 이벤트 처리 결정. 명시 안 하면 `audit` (적용 + 로그). 가장 보수적으로 운영하려면 `reject`, LLM 자유를 최대로 하려면 `audit`.

각 alternative는 `new_type` 텍스트와 함께 `bond_kind_shift`(선택)를 동반해 type 변화와 BondKind 변화를 한 묶음으로 일관 적용한다. LLM이 alternatives 외 텍스트 emit 시 `fallback`이 안전판.

`reasoning_hint`(Tier 3)는 *왜 이 결말이 가능한가*를 디자이너가 명시한 텍스트로, prompt에 alternatives와 함께 주입돼 LLM 매칭 정확도를 높인다. 작성 부담 큰 만큼 *정말 중요한 plot point*에만 권장.

예 — 임충 시나리오:

```yaml
# Tier 0: 일상 대화는 무설정 → LLM 자유, default_transformation_policy 적용

# Tier 1: 꿈 장면
transformation_events:
  - event_id: "lin_chong_dream_01"
    mode: reject

# Tier 3: 핵심 plot point
  - event_id: "tp_4_liangshan"
    npc: "lin_chong"
    target: "self"
    mode: allowlist
    alternatives:
      - new_type: "양산박 호걸"
        reasoning_hint: "체제에 등 돌리고 의적 합류 — 적극적 결단."
        bond_kind_shift: SwornBrothers
      - new_type: "낭인"
        reasoning_hint: "양산박 합류 거부, 도망자로 떠돎 — 회피적 대응."
        bond_kind_shift: null
      - new_type: "은둔자"
        reasoning_hint: "산속에 숨어 세상 등짐 — 단절."
        bond_kind_shift: null
    fallback: "낭인"
```

> **mind-studio 모드 연동**: 위 production 모드와 무관하게, mind-studio 저작 환경에서는 모든 emit이 *suggested* 상태로 추가 박제. 디자이너가 mind-studio에서 audit 로그를 보며 *반복 발생하는 LLM emit 패턴*을 발견 → 해당 이벤트를 Tier 1/2/3로 *승격*. 시간이 지날수록 시나리오가 자연 정착화되는 워크플로우.

#### Channel 2: Temporal (시간 누적)

- **언제 발화되나**: 카운터 read model이 BondKind 진입 게이트 임계 도달 알림.
- **누가 식별하나**: 엔진. 매 outer loop 처리 시 카운터 점검.
- **감정 격동 무관**: 매일 작은 도움이 30일 → Companion 진입. 어떤 한 대화도 결정적이지 않음.
- **게이트**: axes가 BondKind threshold 위 + 연속 N일 유지 (§3.2 시간 게이트).

#### Channel 3: External (외부 사건 cross-reference)

- **언제 발화되나**: *다른 NPC*에서 발생한 도메인 이벤트가 이 관계에 파급. 무대 사망 → 무송-반금련 BloodEnemy.
- **누가 식별하나**: 엔진의 EventPropagator (application layer). PropagationRule + 인지 정책에 따라 cross-NPC 이벤트 발행.
- **감정 격동 무관**: transformation 자체는 외부 사실의 산물. 격동은 *그 사실을 인지한 순간*에 일어나지만 transformation 결정과 별개.
- **게이트**: 외부 사건의 신뢰성 + NPC 인지 여부.

EventPropagator의 자세한 메커니즘은 implementation roadmap 문서 참조 (Phase 3 작업).

#### Channel 4 (보조): Emotional (감정 격동)

대화 중 감정 누적이 자연스럽게 새 type을 만드는 경우. 예: 적이라 생각했던 자에게 도움받고 *깨달음*. 분명히 존재하지만 위 3 채널보다 *드문* 케이스. **본류가 아닌 보조**.

#### 가드레일

각 채널의 적용 조건:

```
Outer Loop 진입:
  significance >= 0.3
  OR  declarative_events 비어 있지 않음
  OR  external_events 비어 있지 않음
  OR  temporal_signals 비어 있지 않음

transformation_event 적용 (Channel 1):
  LLM emit  AND  significance >= 0.5  AND  peak_occ_intensity >= 0.7
  AND 사회적 일관성 검증 A~D 통과
  AND 적용 모드 정책 통과

partnership_event 적용 (Channel 1):
  LLM emit  AND  significance >= 0.4
  AND 사회적 일관성 검증 A~D 통과
  AND 적용 모드 정책 통과
  (Partnership은 의례 — *명시성*이 *감정 강도*보다 중요. 정략혼은 격동 없을 수 있음)

Channel 2/3는 별도 가드레일 (§3.2 시간 게이트, EventPropagator 룰)
```

LLM ↔ engine 불일치(예: LLM이 transformation emit했으나 significance 미달)는 *별도 로그*에 남김. LLM 판단의 calibration drift 감지용.

### 6.5 LLM ↔ Engine 책임 분리

| 역할 | 엔진 (도메인) | LLM (application) |
|---|---|---|
| `significance_score` | ✅ 결정론 계산 | ❌ 출력하지 않음 |
| `is_chitchat` | ❌ | ✅ 직관 판정 |
| `summary` | ❌ | ✅ 1~2문장 요약 |
| `declarative_events` 제안 (Ch.1) | ❌ | ✅ 텍스트에서 추출 |
| `partnership_event` 제안 (Ch.1) | ❌ | ✅ 의례 식별 |
| `temporal_signals` (Ch.2) | ✅ 카운터 read model | ❌ |
| `external_events` (Ch.3) | ✅ event bus 조회 | ❌ |
| 사회적 일관성 검증 A~D | ✅ 결정론 | ❌ |
| 적용 모드 알림 (allowlist/audit/reject) | ✅ scenario config 조회 | ❌ |
| Alternative 매칭 (Ch.1 allowlist) | ❌ | ✅ 대안 중 선택 |
| 제안 적용 여부 결정 | ✅ 게이트 통과 시만 | ❌ |
| OCC 응축 (top-K) | ✅ 결정론 | ❌ |
| axes 갱신 | ✅ 결정론 (HEXACO 보정자) | ❌ |

**원칙**: *LLM은 작가, Engine은 편집자*. 작가는 "이 장면이 캐릭터에게 큰 의미"라 주장하나, 편집자는 *원고에 실제로 격동의 흔적이 있는지*를 정량으로 확인하고 통과 여부를 결정. 두 역할이 같은 텍스트를 다른 각도에서 본다.

### 6.6 검증 — 무협 사례 재해석

| 사건 | 어느 채널이 잡는가 | Outer Loop 효과 |
|---|---|---|
| 이모백-수련 Soulmate | Channel 2 (Temporal) — 매일 작은 동행이 30년 누적 | bond_kind: Soulmate 진입 |
| 임충-노지심 의형제 | Channel 1 (Declarative) — "형 동생" 호칭 선언 | type_history 갱신, bond_kind: SwornBrothers 진입 |
| 곽정-황용 Spouse | Channel 1 (Declarative) — 결혼식 의례 + Channel 4 감정 누적 (연애결혼) | partnership: Spouse |
| 무송 → 반금련 BloodEnemy | Channel 3 (External) — 무대 사망 이벤트 cross-reference | bond_kind: BloodEnemy 진입, ActionTrigger emit |
| 와호장룡 옥교룡 → 노소호 | Channel 1 (Declarative) emit 후 사회적 일관성 검증 D(양방향 동의)에서 reject | Partnership 적용 안 됨, 도주 사건 발생 — *검증이 작동한* 사례 |

→ 5사례 모두 잡힘. 감정 채널 4가 본류가 아니어도 무협의 거의 모든 transformation이 잡힌다. 마지막 와호장룡 사례는 *사회적 일관성 검증*이 LLM emit을 reject하는 시나리오로, 엔진의 가드레일이 서사적으로 정확히 작동하는 demonstration.

### 6.7 단계별 도입

§6의 전체 시스템은 한 번에 구현되지 않는다. 단계 분리:

| Phase | 내용 | 영향 범위 |
|---|---|---|
| **v0.7 (Phase 1)** | Reflection 단계 + Engine significance + is_chitchat 게이트 | Relationship 모델 *그대로* (현 코드의 3축 유지). 작은 변경. |
| **v0.8 (Phase 2)** | 4축 마이그레이션 + Channel 1 (Declarative) + 사회적 일관성 검증 + 적용 모드 | Relationship 도메인 재작성. _schema.md 갱신. |
| **v0.9 (Phase 3)** | Channel 2 (Temporal 카운터) + Channel 3 (External Propagator) + ActionTrigger | BondKind 카운터 read model + EventPropagator + ActionTriggerEvaluator 신설. |

자세한 phasing과 현 코드 매핑은 `docs/tasks/mind-architecture/00-roadmap.md` 참조.

---

## 7. 다음 단계

본 문서가 정의하지 않는 것:

1. **ActionTriggerEvaluator의 룰** — `action_triggers.md` v0.1 참조.
2. **동행(同行) 시스템** — `companions.md` (가칭, 미작성).
3. **평판(評判) 시스템** — `reputation.md` (가칭).
4. **인연(因緣)·기연(奇緣) 트리거** — Pillar 5.

본 문서가 발생시키는 **스키마 v0.6 보정 사항**:
- BondKind enum 9 → 11 (Companion, Guardian 추가)
- 검증 체크리스트에 Guardian·Companion 임계 일관성 항목 추가

---

## 변경 이력

| 버전 | 일자 | 변경 |
|------|------|------|
| v0.3 | 2026-05-04 | 4축(직교+음수) + type/type_history + 4종류 지기 + OCC 매핑 |
| v0.4 | 2026-05-04 | BondKind 통합 (지기 4 + 원수 4 = 8). 진입·이탈 비대칭. |
| v0.5 | 2026-05-04 | 세 차원 직교화 (BondKind 9 + BondStatus 5 + Partnership 4). 회상 OCC 골격. |
| v0.6 | 2026-05-04 | **BondKind 11**: Companion·Guardian 신설 (노년기 수련 한계 해소). **회상 OCC 구체화**: 5종 트리거·강도 계산식·PAD 영향·추모 행동 5종. **compass 변화 후 axes 자연 누적 룰 명시** (§1.5). **ActionTriggerEvaluator 분리** — 별도 문서 `action_triggers.md`로 *행동 emit* 책임 이동. relationships.md는 *분류*까지. Guardian 진입 7일·Mentor 14일·SwornBrothers/Companion 30일 — 진입 시간 차등 정착. |
| v0.7 | 2026-05-09 | **Inner/Outer 루프 명시 분리** (§4.4 흐름도 수정 + §6.1). **Scene Boundary Reflection 단계 신설** (§6.2): LLM이 서사적 의미 평가, 엔진이 정량 significance 계산. **3-channel transformation/partnership trigger** (§6.4): Declarative (LLM) + Temporal (엔진 카운터) + External (EventPropagator) + 보조 Emotional. **Channel 1 적용 모드** (§6.4): 4-tier 점진적 디자이너 제어 — 무설정/모드만/Alternatives/+Hints. mind-studio 저작 워크플로우 연동. **사회적 일관성 검증 5 카테고리** (§6.4): A. Structural / B. Precondition / C. BondStatus Block / D. Mutuality / E. Domain Knowledge. **LLM ↔ Engine 책임 분리 원칙** (§6.5, 작가-편집자 비유). **단계별 도입** (§6.7): Phase 1/2/3 분리. *§1~§5 (4축/BondKind/BondStatus/Partnership) 변경 없음 — Phase 2 작업으로 이연.* |
