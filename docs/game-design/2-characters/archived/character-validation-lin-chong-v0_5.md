# 인물 스키마 v0.5 검증 — 임충 (林沖)

> 작성일: 2026-05-04
> 검증 대상: `_schema.md` v0.5 + `relationships.md` v0.5
> 위치: `docs/game-design/2-characters/character-validation-lin-chong-v0_5.md`
> 이전 버전: v0.4 (`character-validation-v0_4.md` 폐기됨)

## v0.5 적용 요약

이전 v0.4 인스턴스에서 발견된 한계가 v0.5의 세 차원 직교화로 해소:

| v0.4 한계 | v0.5 해소 |
|---|---|
| Dyadic romantic bond (장씨) — axes 깊으나 enum 부재 | `bond_kind: null` + `partnership: Separated`로 *형식*과 *깊이* 분리 표현 |
| 죽마고우 처단 후 axes 영구화 (육겸) | `bond_status: Resolved`로 *결판 도달* 명시. 행동 트리거 불활성. |
| 향후 노지심 SwornBrothers 진입 모호 | `bond_status: Active` + Mentor variant 검토 (현재 인스턴스에선 SwornBrothers 임계 근접 유지) |

세 차원 모두 인스턴스에서 의미 있게 사용됨.

---

# 임충(林沖)

## Layer 1 — 본바탕

### identity

```yaml
id: "lin_chong"
name: "임충(林沖)"
nicknames:
  - "표자두(豹子頭)"
  - "천웅성(天雄星)"
era: "북송 휘종 치세"
stage_of_life: "장년기"
snapshot_time: "산신묘 사건 직후, 양산박 가입 *전*"   # 30대 후반
```

### origin

```yaml
birthplace: "동경(東京, 개봉)"
social_origin: "양민 → 무관(武官)"
kingdom_of_origin: "송"
family_background: |
  하급 무관 가문 출신. 부친도 무관이었으나 일찍 사망.
  본인은 무예로 입신하여 80만 금군교두(禁軍敎頭) 자리까지 오름.
  유가적 충(忠)·의(義) 교육을 받고 자란 *체제 안의 엘리트*.
  현재 (산신묘 후) 그 체제로부터 박탈됨.
```

### temperament — HEXACO 24 facet

```yaml
H_honesty_humility:
  sincerity: 80
  fairness: 85
  greed_avoidance: 75
  modesty: 75
E_emotionality:
  fearfulness: 30
  anxiety: 60       # ↑ 산신묘 후 트라우마성 불안
  dependence: 50
  sentimentality: 80
X_extraversion:
  social_self_esteem: 70
  social_boldness: 70
  sociability: 50
  liveliness: 45
A_agreeableness:
  forgiveness: 55
  gentleness: 60
  flexibility: 50
  patience: 95      # ★
C_conscientiousness:
  organization: 85
  diligence: 90
  perfectionism: 80
  prudence: 90      # ★
O_openness:
  aesthetic_appreciation: 50
  inquisitiveness: 50
  creativity: 55
  unconventionality: 55  # ↑ 산신묘 후 체제 밖으로
```

### body

```yaml
physical_description: |
  30대 후반. 표범의 머리에 둥근 눈, 호랑이 같은 수염.
  현재 얼굴에 *낙인(刺字)* — 창주 유배 시 새겨진 죄인의 표식.
signature_feature: |
  일장팔척 장사모(丈蛇矛). 현재는 산신묘에서 빼앗은 적의 무기 임시 사용.
```

## Layer 2 — 현재 표현

### inner_compass

```yaml
inner_compass:
  compass: "내 손으로 의(義)를 행한다 — 부패한 권력의 칼이 되지 않는다"
  taboo: "무고한 자에게 칼을 휘두르지 않는다"
  life_question: "나는 다시 충성할 가치가 있는 무엇을 만날 수 있을까?"
  taboo_crystallization: "tp_yezhulin"
```

### current_state

```yaml
current_state:
  pad: { pleasure: -0.6, arousal: 0.4, dominance: 0.5 }
  dominant_emotion: "Resentment + Resolution (한과 결연의 공존)"
  active_focus: "양산박을 향해 도주 + 다음 행동 결정"
```

### relationships

#### key_bonds — v0.5 적용 (5개 모두)

```yaml
key_bonds:

  # ──────────────────────────────────────────────────
  # 1. 육겸 — Betrayer + Resolved (★ v0.5 결판 도달 표시)
  # ──────────────────────────────────────────────────
  - target: "lu_qian"
    type: "죽마고우 → 적·처단 대상 → 처단됨"
    type_history:
      - { since: "유년기",                  type: "죽마고우" }
      - { since: "고구 매수 후",             type: "은밀한 배신자 (임충은 모름)" }
      - { since: "shanshenmiao_event",     type: "적·처단 대상" }
      - { since: "shanshenmiao_event",     type: "처단됨" }     # ★ v0.5: status 변화 명시
    transformation_events:
      - { event_id: "shanshenmiao_event", new_type: "적·처단 대상 → 처단됨" }
    axes: { trust: -100, affinity: -90, respect: -100, wariness: 100 }
    bond_kind: "Betrayer"
    bond_status: { Resolved: { reason: "산신묘에서 임충이 직접 처단" } }   # ★ v0.5
    partnership: null
    bond_since: "shanshenmiao_event"
    note: |
      v0.4 한계 해소: 처단 후 axes는 freeze, 행동 트리거 불활성. 그러나 BondKind는 그대로 유지 —
      *죽마고우를 직접 죽인 사실*의 그림자가 임충의 정체성에 영원히 남음. 회상 OCC가 가능
      (산신묘를 떠올릴 때 Sadness + Resolution 혼합 emit).

  # ──────────────────────────────────────────────────
  # 2. 고구 — Oppressor + Active (직접 처단 불가 상태로 활성)
  # ──────────────────────────────────────────────────
  - target: "gao_qiu"
    type: "권력의 정점에서 나를 짓밟은 자"
    type_history:
      - { since: "복무 시절",             type: "최고 상관 (태위)" }
      - { since: "baihu_jietang_event", type: "권력의 정점에서 나를 짓밟은 자" }
    transformation_events:
      - { event_id: "baihu_jietang_event", new_type: "권력의 정점에서 나를 짓밟은 자" }
    axes: { trust: -70, affinity: -80, respect: -50, wariness: 95 }
    bond_kind: "Oppressor"
    bond_status: "Active"     # ★ v0.5: 결판 미도달, 양산박 합류로 *변형된 표출*
    partnership: null
    bond_since: "baihu_jietang_event"
    note: |
      Oppressor는 직접 처단 어려우므로 *체제 자체에 저항*하는 행동으로 emit. 양산박 합류가 그 표출.
      bond_status: Active이지만 행동 트리거의 *형태*가 BloodEnemy와 다름 — 직접 vs 체제적.

  # ──────────────────────────────────────────────────
  # 3. 고아내 — BloodEnemy + Active (★ v0.5 직교성 검증: 트리거 충족하나 보류)
  # ──────────────────────────────────────────────────
  - target: "gao_yanei"
    type: "아내를 노린 자, 직접 가해의 발단"
    type_history:
      - { since: "초기 만남",   type: "권력자의 양아들" }
      - { since: "흑심 발각", type: "아내를 노린 자, 직접 가해의 발단" }
    transformation_events:
      - { event_id: "wife_violation_attempt", new_type: "아내를 노린 자" }
    axes: { trust: -90, affinity: -90, respect: -70, wariness: 90 }
    bond_kind: "BloodEnemy"
    bond_status: "Active"
    partnership: null
    bond_since: "wife_violation_attempt"
    note: |
      ★ 시스템 검증: BondKind 임계 충족 + bond_status Active → BondKind 차원에서는 *처단 트리거 emit*.
      그러나 *실행 가능성* (정치권력·물리거리 변수)이 별도 평가에서 *보류* 결정. 
      v0.5는 *분류 차원*까지 표현. 실행 가능성 평가는 ActionTriggerEvaluator(미설계) 영역.

  # ──────────────────────────────────────────────────
  # 4. 노지심 — SwornBrothers 임계 근접 + Active
  # ──────────────────────────────────────────────────
  - target: "lu_zhishen"
    type: "구명의 은인 → 형제 후보"
    type_history:
      - { since: "초기 우정 (개봉 시절)", type: "음주의 벗" }
      - { since: "yezhulin_rescue",   type: "구명의 은인 → 형제 후보" }
    transformation_events:
      - { event_id: "yezhulin_rescue", new_type: "구명의 은인 → 형제 후보" }
    axes: { trust: 80, affinity: 75, respect: 70, wariness: 25 }
    bond_kind: null              # ★ SwornBrothers 임계 충족하나 *연속 30일* 미달
    bond_status: "Active"
    partnership: null
    bond_since: null
    note: |
      ★★ 양극 진입 시간 게이트 검증. SwornBrothers 임계 (trust ≥80, affinity ≥70, respect ≥60,
      wariness ≤30) *모두 충족*이지만 연속 30일 카운터는 야저림 사건 후 며칠뿐. 곧 헤어지면 리셋.

      ★ Mentor variant 후보로도 평가했으나, 노지심-임충은 *동등한 형제* 결이지 비대칭 멘토가 아님.
      respect 임계는 충족하지만 *가르치려 함*의 type_history 부재 → Mentor 추가 조건 미충족.
      따라서 SwornBrothers 분류만 유효. 분류 시도가 v0.5에서 정확히 작동.

  # ──────────────────────────────────────────────────
  # 5. 장씨 (아내) — null + Active + Separated (★ v0.5 직교성 핵심 검증)
  # ──────────────────────────────────────────────────
  - target: "zhang_shi"
    type: "아내 → 휴서 후 별거 → 다시 만날 수 없는 사람"
    type_history:
      - { since: "결혼 후",                 type: "아내" }
      - { since: "baihu_jietang_event",   type: "지키지 못한 아내 (유배 결정 후)" }
      - { since: "departure_xiushu",      type: "아내 → 휴서 후 별거 → 다시 만날 수 없는 사람" }
    transformation_events:
      - { event_id: "marriage_event",   new_type: "아내" }                  # ★ v0.5: Spouse 진입 사건
      - { event_id: "departure_xiushu", new_type: "휴서 후 별거" }          # ★ v0.5: Separated 진입 사건
    axes: { trust: 95, affinity: 90, respect: 70, wariness: 5 }
    bond_kind: null              # ★ Soulmate 임계 충족하나 의미 결 다름 (영혼 일치보다 부부 동반)
    bond_status: "Active"        # ★ v0.5: 장씨 생존 (현재 시점 기준)
    partnership: "Separated"     # ★★★ v0.5 핵심 — 결혼 후 휴서
    bond_since: null
    note: |
      ★★★ v0.5 직교성의 가장 강한 검증.
      - axes는 깊은 양수 (95/90/70/5) — Soulmate·SwornBrothers·LoyalRetainer 임계 충족.
      - 그러나 BondKind 어느 것도 *부부의 의미 결*과 정확히 맞지 않아 null.
      - Partnership: Separated가 *결혼이 있었음 + 현재 별거*의 형식을 정확 표현.
      - Status: Active이므로 행동 가능성 살아있음 — 향후 재회·구원 트리거 가능.

      v0.4에서 "axes 깊지만 enum 매핑 부재"로 노출된 한계가, v0.5에서 *세 차원 직교*로 정확 해소.
      enum 강제 없이도 시스템이 관계의 *형식 + 깊이 + 활동성*을 모두 보존.
```

#### dormant_bonds

```yaml
dormant_bonds:
  - target: "어린 시절 첫 무술 사부 (이름 미상)"
    last_contact: "age 12~13"
    fragment: |
      처음 사모를 잡았을 때, 사부가 손 위에 손을 얹어주던 무게.
      "사모는 사람을 *지키는* 것이지 *위협하는* 것이 아니다"라는 한 마디만 또렷이.
    note: |
      기연 후보. 양산박에서 비슷한 가르침을 주는 노승·은자 만남이 기연 트리거 가능.
      *한 번도 활성화된 적 없는* 잠재 관계 — dormant_bonds 정의에 부합.
```

### voice

```yaml
voice:
  speech_register: "정중함 ↔ 냉혹함 (양극 공존, 현재는 후자 우세)"
  vocabulary_level: "사대부 + 무관 용어 (현재 강호 어휘 섞임 시작)"
  tics:
    - "체제 안에서: 상대를 존칭으로 ('태위', '대인', '현처')"
    - "현재: '네 놈', '간신적자' 같은 도덕적 비난 어휘"
    - "전투 호령은 짧고 단정적 ('에잇!', '기다려라!')"
    - "결정적 순간 — 자기 호명 ('양산박의 표자두 임충이!')"
  voice_anchors:
    - context: "정중한 무관 (체제 안 시절, 아내에게)"
      utterance: "현처(賢妻), 내 말을 들어보오. 나는 운이 사나워 노 태위의 모함을 받았소."
    - context: "도덕적 호소 (휴서 작성, 장인에게)"
      utterance: "장인어른, 제가 아내와 헤어지려는 것은 사랑하지 않아서가 아닙니다."
    - context: "각성 후 냉혹한 처단 (산신묘, 육겸에게)"
      utterance: "너 같은 놈은 살려둘 가치도 없다. 네 놈의 심장을 꺼내 내 억울함을 씻으리라!"
    - context: "양산박 시대 호령 (미래 시점)"
      utterance: "양산박의 표자두 임충이 여기 있다! 비겁하게 숨지 말고 내 사모를 받아라!"
    - context: "겸손한 의리 (조개·송강에 대한 충성, 미래)"
      utterance: "형님께서 가시는 길이라면 이 임충, 말 앞의 졸개라도 되어 끝까지 따르겠나이다."
```

### titles

```yaml
titles:
  - "표자두(豹子頭)"
  - "천웅성(天雄星)"
  - "(실효: 80만 금군교두 — 박탈됨)"
```

## Layer 3 — 시간축

### past — transition_points

```yaml
transition_points:

  - id: "marriage_event"
    age: "20대 후반"
    event: "장씨와 결혼 — Partnership: null → Spouse 전환"
    impact:
      hexaco_shifts:
        - "E+ Sentimentality: 70 → 80"
    inner_resolution: "이 사람과 평생을 함께한다."
    significance: "Partnership: Spouse 진입 사건. v0.5 partnership 일관성 검증용."

  - id: "tp_baihu_jietang"
    age: "30대 중반"
    event: "백호절당 함정 → 유배 결정"
    impact:
      hexaco_shifts:
        - "C+ Prudence: 80 → 90"
        - "E+ Anxiety: 30 → 50"
    inner_resolution: "체제는 나를 보호하지 않는다."
    significance: "첫 충격. 아직 체제 안에 머물려 함."

  - id: "wife_violation_attempt"
    age: "30대 중반"
    event: "고아내가 장씨에게 흑심 — 미수에 그침"
    impact:
      hexaco_shifts:
        - "E+ Anxiety: 30 → 35"
        - "A- Forgiveness: 60 → 55"
    inner_resolution: "내 가족을 노리는 자는 결코 용서하지 않는다."
    significance: "고아내 → BloodEnemy 진입 시작점."

  - id: "departure_xiushu"
    age: "30대 중반"
    event: "장씨에게 휴서를 써줌 — Partnership: Spouse → Separated 전환"
    impact:
      hexaco_shifts:
        - "E+ Sentimentality: 70 → 80"
    inner_resolution: "내가 지키지 못한다. 차라리 자유롭게."
    significance: "★ Partnership: Separated 진입. 사랑은 살아있으나 형식은 깨짐."

  - id: "tp_yezhulin"
    age: "30대 중반"
    event: "야저림에서 호송관 살해 시도 → 노지심 구출 → 호송관 *살려줌*"
    impact:
      hexaco_shifts:
        - "X+ Social Boldness: 65 → 70"
    inner_resolution: "내 손에 떨어진 무고한 자는 죽이지 않는다."
    significance: "★ taboo_crystallization 지점."

  - id: "yezhulin_rescue"
    age: "30대 중반"
    event: "야저림 — 노지심이 임충의 생명 구함"
    impact:
      hexaco_shifts:
        - "E- Dependence: 50 → 45"
    inner_resolution: "이 자는 거짓이 없다."
    significance: "노지심 axes 즉시 갱신. SwornBrothers 임계 도달의 첫걸음."

  - id: "tp_shanshenmiao"
    age: "30대 중반"
    event: |
      산신묘 — 육겸 등의 자기 살해 자랑 엿들음 → *직접 처단*. 육겸 bond_status: Active → Resolved 전환.
    impact:
      hexaco_shifts:
        - "O+ Unconventionality: 35 → 55"
        - "X+ Social Boldness: 70 → 80"
        - "E+ Anxiety: 50 → 60"
      compass_change:
        from: "법과 군의 명을 따르며 가족을 지킨다"
        to:   "내 손으로 의(義)를 행한다 — 부패한 권력의 칼이 되지 않는다"
    inner_resolution: "더 이상 위선의 규칙에 얽매이지 않는다."
    significance: |
      ★★★ 최대 전환점. *동시에 4가지 시스템 슬롯 작동*:
        1. 육겸 BondKind 진입 (Betrayer, 즉시)
        2. 육겸 BondStatus 전환 (Active → Resolved)
        3. compass_change
        4. life_question 발생 ("충성할 가치가 있는 무엇")
```

### past — formative_relationships

```yaml
formative_relationships:
  - id: "father"
    type: "부친 (일찍 사망한 무관)"
    legacy: |
      무관 정체성의 원형. 아버지를 일찍 잃었기에 *체제(군)*가 부친 역할을 대신함.

  - id: "first_master"
    type: "어린 시절 첫 무술 사부 (이름 미상)"
    legacy: "사모를 처음 가르친 자. taboo의 *씨앗*."

  - id: "lu_qian_past"
    type: "유년기 죽마고우 (이미 처단됨)"
    legacy: |
      *신뢰의 원형이자 그 파괴의 원형*. key_bonds에도 동시 등록 (현재 영향 + 과거 의미 모두 큼).
      v0.5 명확화: bond_status: Resolved이므로 key_bonds 위치가 정합 (현재 정체성 영향).
      formative에도 등록은 *과거 의미의 깊이* 추가 표시용.
```

### present — unresolved_tension

```yaml
unresolved_tension:
  - id: "ut_1_wife_fate"
    category: "관계적·죄책감"
    description: |
      장씨의 안위 미확인. life_question에 가장 직접 닿는 미해결.
      Partnership: Separated + Status: Active이므로 재회·재결합 트리거 가능성 살아있음.

  - id: "ut_2_gao_unreachable"
    category: "외부적·구조적"
    description: |
      고구·고아내는 체제 정점. BondKind 처단 트리거 *보류 상태*.

  - id: "ut_3_self_doubt"
    category: "내부적·정체성"
    description: |
      30년 신뢰한 죽마고우가 가짜였다면, 누구를 신뢰할 수 있는가?
      → 노지심 SwornBrothers 카운터가 30일을 채우지 못하는 *내적 이유*이기도 함.
```

### future hooks

```yaml
tragic_seed:
  description: |
    고아내가 장씨에게 다시 손을 뻗고 자결로 정조 지킴. 임충은 *몇 달 후* 소식 들음.
    장씨 bond_status: Active → Deceased 전환 트리거.
  trigger_condition: |
    `ut_1_wife_fate` 답을 찾을 때. compass의 "의(義)"가 *실패한 의*로 재정의됨.

joyful_seed:
  description: |
    양산박에서 어린 시절 첫 사부의 가르침을 *다른 형태로* 전하는 노승 만남.
    `dormant_bonds[0]` 활성화 가능성.
  trigger_condition: |
    노승과의 만남 + 산신묘 처단의 *손의 무게*에 대한 새 해석.
```

---

# v0.5 적용 검증 결과

## 세 차원 활용 분포 (5개 key_bonds)

| 인물 | bond_kind | bond_status | partnership |
|---|---|---|---|
| 육겸 | Betrayer | Resolved | null |
| 고구 | Oppressor | Active | null |
| 고아내 | BloodEnemy | Active | null |
| 노지심 | null | Active | null |
| 장씨 | null | Active | **Separated** |

5개 BondKind variants (Betrayer/Oppressor/BloodEnemy/null/null), 2개 Status (Active 4 / Resolved 1), 1개 Partnership (Separated). 

## v0.4 한계 해소 검증

| v0.4 한계 | v0.5 해소 방식 | 임충 인스턴스에서 |
|---|---|---|
| Romantic bond 부재 | Partnership 별도 슬롯 | 장씨 — Partnership: Separated |
| 결판 도달 후 처리 부재 | bond_status: Resolved | 육겸 — Resolved + reason 명시 |
| 사망 처리 부재 | bond_status: Deceased + deceased_at | 임충에는 사망자 없음 (수련 인스턴스에서 검증) |
| Mentor variant 부재 | BondKind에 Mentor 추가 | 노지심 평가 — Mentor 추가 조건(가르침) 미충족 → SwornBrothers 후보 유지. *분류 시도가 정확히 작동*. |

## v0.5의 한계 — 다음 검증 필요

1. **ActionTriggerEvaluator** — 고아내 BloodEnemy의 *분류는 충족, 실행은 보류* 케이스. v0.5는 분류만, 실행은 별도 시스템 필요.
2. **회상 OCC 메커니즘 구체화** — §4.5 골격만 있음. 육겸을 떠올릴 때 어떤 OCC가 어느 정도 강도로 발생하는가?
3. **compass 변화 후 axes 재평가** — 임충의 compass가 산신묘에서 변화. 그러나 axes는 사건 누적값. compass 변화 시 *모든 key_bond 재평가* 필요한 케이스 추적 필요.

이 3개는 v0.6 후보. 현재 v0.5 시스템은 임충 인스턴스를 *왜곡 없이* 표현 가능.

---

## 변경 이력

| 버전 | 일자 | 변경 |
|------|------|------|
| v1.0 (v0.4 스키마) | 2026-05-04 | 초안. v0.4 검증. dyadic romantic / Mentor / Action 한계 노출. |
| v2.0 (v0.5 스키마) | 2026-05-04 | **v0.5 적용**. bond_status (Active/Resolved) 5개 key_bonds 모두 명시. partnership: Separated 장씨에 적용. 노지심 Mentor 평가 결과 SwornBrothers 후보 유지. 검증 결과 v0.5가 v0.4 한계를 해소함을 입증. |
