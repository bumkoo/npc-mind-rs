# 인물 스키마 v0.6 검증 — 임충 (林沖)

> 작성일: 2026-05-04
> 검증 대상: `_schema.md` v0.6 + `relationships.md` v0.6 + `action_triggers.md` v0.1
> 위치: `docs/game-design/2-characters/character-validation-lin-chong-v0_6.md`
> 이전 버전: v0.5 (`character-validation-lin-chong-v0_5.md` 폐기)

## v0.6 변경 요약

임충 인스턴스는 v0.6에서 *큰 변경 없음*. 변경 위치:

| 영역 | v0.5 | v0.6 |
|---|---|---|
| 노지심 분류 | null (SwornBrothers 임계 근접, 30일 미달) | **null 유지** + Companion 후보도 검토하나 *형제 결*로 SwornBrothers 후보 채택 |
| ActionTrigger | "양산박 합류" 직관적 서술 | 고구·고아내에 대한 5차원 feasibility 평가 *시스템 도출* |
| compass 자연 누적 룰 | 명시 부재 | departure_xiushu 등의 compass_change 없음을 *명시적*으로 정합 |

핵심 변경은 *분류*가 아닌 *행동 도출의 시스템화*. ActionTriggerEvaluator의 적용으로 임충의 양산박 합류·고아내 처단 보류가 *직관*에서 *5차원 평가의 자연 결과*로.

---

# 임충(林沖)

## Layer 1 — 본바탕

### identity

```yaml
id: "lin_chong"
name: "임충(林沖)"
nicknames: ["표자두(豹子頭)", "천웅성(天雄星)"]
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
  하급 무관 가문 출신. 부친 일찍 사망. 본인은 무예로 80만 금군교두(禁軍敎頭) 자리까지.
  유가적 충(忠)·의(義) 교육을 받은 *체제 안의 엘리트*. 현재 그 체제에서 박탈됨.
```

### temperament — HEXACO (변화 없음, v0.5와 동일)

```yaml
H_honesty_humility: { sincerity: 80, fairness: 85, greed_avoidance: 75, modesty: 75 }
E_emotionality:     { fearfulness: 30, anxiety: 60, dependence: 50, sentimentality: 80 }
X_extraversion:     { social_self_esteem: 70, social_boldness: 70, sociability: 50, liveliness: 45 }
A_agreeableness:    { forgiveness: 55, gentleness: 60, flexibility: 50, patience: 95 }
C_conscientiousness:{ organization: 85, diligence: 90, perfectionism: 80, prudence: 90 }
O_openness:         { aesthetic_appreciation: 50, inquisitiveness: 50, creativity: 55, unconventionality: 55 }
```

### body

```yaml
physical_description: |
  30대 후반. 표범의 머리에 둥근 눈. 얼굴에 *낙인(刺字)* — 창주 유배 시 죄인 표식.
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

#### key_bonds — 5개 (v0.5와 분류 동일, v0.6에서 ActionTrigger note 추가)

```yaml
key_bonds:

  # ──────────────────────────────────────────────────
  # 1. 육겸 — Betrayer + Resolved (v0.6에서 변화 없음)
  # ──────────────────────────────────────────────────
  - target: "lu_qian"
    type: "죽마고우 → 적·처단 대상 → 처단됨"
    type_history:
      - { since: "유년기",                  type: "죽마고우" }
      - { since: "고구 매수 후",             type: "은밀한 배신자 (임충은 모름)" }
      - { since: "shanshenmiao_event",     type: "적·처단 대상" }
      - { since: "shanshenmiao_event",     type: "처단됨" }
    transformation_events:
      - { event_id: "shanshenmiao_event", new_type: "처단됨" }
    axes: { trust: -100, affinity: -90, respect: -100, wariness: 100 }
    bond_kind: "Betrayer"
    bond_status: { Resolved: { reason: "산신묘에서 임충이 직접 처단" } }
    partnership: null
    bond_since: "shanshenmiao_event"
    note: |
      v0.6 회상 OCC 작동 가능. 산신묘를 떠올릴 때 Sadness + Resolution 혼합 emit.
      회상 강도 계산:
      - bond_depth: 0.5 (Resolved 적)
      - axes_magnitude: (100+90+100+100)/4/100 = 0.975
      - time_decay: 1.0 (사건 직후)
      - sentimentality: 0.8
      - 최종 강도 약 0.5 → SilentMonologue 후보 등록 가능 (죽마고우를 죽인 무게)
      추모 행동은 emit되지 않음 (적이라 기일 의식 없음).

  # ──────────────────────────────────────────────────
  # 2. 고구 — Oppressor + Active (★ v0.6 ActionTrigger 검증)
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
    bond_status: "Active"
    partnership: null
    bond_since: "baihu_jietang_event"
    note: |
      ★★★ v0.6 ActionTriggerEvaluator 검증의 핵심 사례.

      입력: bond_kind: Oppressor → 기본 후보 [SystemicResistance, AvoidContact]
      
      SystemicResistance 평가:
      - physical_access: 0.05 (고구는 황궁, 임충은 양산박행)
      - power_balance: 0.10 (정치 권력 압도적 차이)
      - social_permission: 0.55 (반체제는 의로움 인정도 부분적)
      - self_capability: 0.75 (HEXACO·PAD 적합)
      - moral_alignment: 0.95 (compass와 정합)
      - combined: ~0.55 (외부 차단 강하나 양산박 합류 형태로 가능)
      
      → SystemicResistance(고구) feasibility 0.55 urgency 0.4
      → 양산박 합류 자연 도출
      
      v0.5에서 "양산박 합류는 적의의 변형된 표출" 직관적 서술이
      v0.6에서 *5차원 평가의 자연 결과*로 환원.

  # ──────────────────────────────────────────────────
  # 3. 고아내 — BloodEnemy + Active (★ v0.6 ActionTrigger 차단·변형 검증)
  # ──────────────────────────────────────────────────
  - target: "gao_yanei"
    type: "아내를 노린 자, 직접 가해의 발단"
    type_history:
      - { since: "초기 만남",   type: "권력자의 양아들" }
      - { since: "흑심 발각", type: "아내를 노린 자" }
    transformation_events:
      - { event_id: "wife_violation_attempt", new_type: "아내를 노린 자" }
    axes: { trust: -90, affinity: -90, respect: -70, wariness: 90 }
    bond_kind: "BloodEnemy"
    bond_status: "Active"
    partnership: null
    bond_since: "wife_violation_attempt"
    note: |
      ★★★ v0.6 ActionTriggerEvaluator의 *차단·변형* 검증 사례.

      입력: bond_kind: BloodEnemy → 기본 후보 [DirectKill]
      
      DirectKill 평가:
      - physical_access: 0.4 (개봉 같은 도시이나 호위병 다수)
      - power_balance: 0.25 (정치 보호막)
      - social_permission: 0.6 (의로움이 공감 받으나 권력자 양아들 살해는 위험)
      - self_capability: 0.85 (무력 충분, 의지 강함)
      - moral_alignment: 0.9 (compass와 정합)
      - combined: 0.18 → blocked
      - blocked_by: [PhysicallyUnreachable, OverwhelminglyPowerful]
      - deferred → SystemicResistance
      
      → DirectKill(고아내) blocked → SystemicResistance(고아내, 양산박 활동 일환) urgency 0.6
      
      v0.5에서 "권력 보호막으로 처단 보류" 직관적 처리가
      v0.6에서 *physical_access × power_balance 곱이 임계 미달*로 자동 평가.
      고아내 처단 욕구가 *Oppressor 행동(양산박 활동)에 흡수*되어 표출.

  # ──────────────────────────────────────────────────
  # 4. 노지심 — null (★ v0.6에서 Companion 후보 검토했으나 SwornBrothers 결로)
  # ──────────────────────────────────────────────────
  - target: "lu_zhishen"
    type: "구명의 은인 → 형제 후보"
    type_history:
      - { since: "초기 우정 (개봉 시절)", type: "음주의 벗" }
      - { since: "yezhulin_rescue",   type: "구명의 은인 → 형제 후보" }
    transformation_events:
      - { event_id: "yezhulin_rescue", new_type: "구명의 은인 → 형제 후보" }
    axes: { trust: 80, affinity: 75, respect: 70, wariness: 25 }
    bond_kind: null
    bond_status: "Active"
    partnership: null
    bond_since: null
    note: |
      ★ v0.6에서 *두 양극 양극 variant 후보 검토*:

      1. **SwornBrothers** (진입 30일):
         - 임계: trust ≥+80 ✓, affinity ≥+70 ✓, respect ≥+60 ✓, wariness ≤30 ✓
         - 30일 카운트는 야저림 후 며칠뿐 — 미달
         - 본질: *형제 결*. 수호전 원작도 노지심-임충은 의형제 결.

      2. **Companion** (진입 30일):
         - 임계: trust ≥+75 ✓, affinity ≥+65 ✓, respect ≥+50 ✓, wariness ≤30 ✓
         - 30일 미달은 동일.
         - 본질: *친구 결*. 신분 차이 가로지르는 평민 우정.

      ★ 두 임계 *모두 충족*하지만 결이 다름:
      - 노지심-임충은 *함께 적과 싸울 형제* 결 (수호지 양산박 합류 후 동귀어진 후보).
      - SwornBrothers의 *동귀어진 트리거*가 적합.
      - Companion은 *생사를 함께 하지 않는* 평민 우정 — 노지심에 안 맞음.

      → SwornBrothers 후보로 채택. 30일 미달이라 현재 bond_kind: null.
      산신묘 후 헤어지면 카운터 리셋. 양산박 재회 시 처음부터 다시 카운트.

  # ──────────────────────────────────────────────────
  # 5. 장씨 (아내) — null + Active + Separated (v0.6에서도 동일)
  # ──────────────────────────────────────────────────
  - target: "zhang_shi"
    type: "아내 → 휴서 후 별거 → 다시 만날 수 없는 사람"
    type_history:
      - { since: "결혼 후",                 type: "아내" }
      - { since: "baihu_jietang_event",   type: "지키지 못한 아내" }
      - { since: "departure_xiushu",      type: "휴서 후 별거" }
    transformation_events:
      - { event_id: "marriage_event",   new_type: "아내" }
      - { event_id: "departure_xiushu", new_type: "휴서 후 별거" }
    axes: { trust: 95, affinity: 90, respect: 70, wariness: 5 }
    bond_kind: null
    bond_status: "Active"
    partnership: "Separated"
    bond_since: null
    note: |
      ★★★ v0.6에서도 *부부형 동반의 BondKind 매핑 부재* 한계 그대로.
      
      v0.6 11종 변검토:
      - SwornBrothers: 형제 결 — 부부 아님
      - Soulmate: 영혼 일치 — 가능하나 *부부의 동반 본질*이 결이 다름
      - LoyalRetainer: 가신 — 부부 아님
      - Companion: 친구 — 부부 아님
      - Guardian: 부모-자녀 — 부부 아님
      
      → 11종 어디에도 *부부형 동반*이 없음. v0.7 후보 명시.
      
      현재 처리:
      - bond_kind: null (자유 텍스트 type만)
      - Partnership: Separated가 *결혼이 있었음 + 현재 별거*의 형식 보존
      - axes 깊은 양수가 정서적 깊이 보존
      
      세 차원 직교가 *부분적으로* 표현 — Partnership으로 형식, axes로 깊이, type으로 의미.
      그러나 *bond_kind 차원이 비어있음*은 v0.6의 명백한 갭.
```

#### dormant_bonds

```yaml
dormant_bonds:
  - target: "어린 시절 첫 무술 사부 (이름 미상)"
    last_contact: "age 12~13"
    fragment: |
      "사모는 사람을 *지키는* 것이지 *위협하는* 것이 아니다."
    note: |
      기연 후보. 양산박에서 비슷한 가르침을 주는 노승·은자 만남이 기연 트리거 가능.
```

### voice — v0.5와 동일 (생략하지 않고 보존)

```yaml
voice:
  speech_register: "정중함 ↔ 냉혹함 (양극 공존, 현재는 후자 우세)"
  vocabulary_level: "사대부 + 무관 용어 (현재 강호 어휘 섞임 시작)"
  tics:
    - "체제 안에서: 상대를 존칭으로 ('태위', '대인', '현처')"
    - "현재: '네 놈', '간신적자' 같은 도덕적 비난 어휘"
    - "전투 호령은 짧고 단정적"
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

### past — transition_points (v0.5와 동일, compass_change 위치만 명시)

```yaml
transition_points:

  - id: "marriage_event"
    age: "20대 후반"
    event: "장씨와 결혼 — Partnership: null → Spouse 전환"
    impact:
      hexaco_shifts: [ "E+ Sentimentality: 70 → 80" ]
      compass_change: null   # ★ Partnership만 변화, compass는 그대로
    inner_resolution: "이 사람과 평생을 함께한다."

  - id: "tp_baihu_jietang"
    age: "30대 중반"
    event: "백호절당 함정 → 유배 결정"
    impact:
      hexaco_shifts:
        - "C+ Prudence: 80 → 90"
        - "E+ Anxiety: 30 → 50"
      compass_change: null   # ★ 첫 충격이지만 compass 아직 그대로 ("법 안에서 결백 증명")
    inner_resolution: "체제는 나를 보호하지 않는다. 그러나 *법 안에서* 결백을 증명하리라."

  - id: "wife_violation_attempt"
    age: "30대 중반"
    event: "고아내가 장씨에게 흑심 — 미수에 그침"
    impact:
      hexaco_shifts:
        - "E+ Anxiety: 30 → 35"
        - "A- Forgiveness: 60 → 55"
      compass_change: null
    inner_resolution: "내 가족을 노리는 자는 결코 용서하지 않는다."

  - id: "departure_xiushu"
    age: "30대 중반"
    event: "장씨에게 휴서 — Partnership: Spouse → Separated 전환"
    impact:
      hexaco_shifts: [ "E+ Sentimentality: 70 → 80" ]
      compass_change: null   # ★ Partnership 변화이지만 compass는 그대로 ("법 안에서")
    inner_resolution: "내가 지키지 못한다. 차라리 자유롭게."

  - id: "tp_yezhulin"
    age: "30대 중반"
    event: "야저림 호송관 살려줌 (taboo 결정화)"
    impact:
      hexaco_shifts: [ "X+ Social Boldness: 65 → 70" ]
      compass_change: null
    inner_resolution: "내 손에 떨어진 무고한 자는 죽이지 않는다."
    significance: "★ taboo_crystallization 지점."

  - id: "yezhulin_rescue"
    age: "30대 중반"
    event: "야저림 — 노지심이 임충 생명 구함"
    impact:
      hexaco_shifts: [ "E- Dependence: 50 → 45" ]
      compass_change: null
    inner_resolution: "이 자는 거짓이 없다."

  - id: "tp_shanshenmiao"
    age: "30대 중반"
    event: "산신묘 — 육겸 등 직접 처단. 육겸 status: Active → Resolved 전환."
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
      ★★★ 최대 전환점. 동시에 5가지 작동:
        1. 육겸 BondKind 진입 (Betrayer, 즉시)
        2. 육겸 BondStatus 전환 (Active → Resolved)
        3. compass_change
        4. life_question 발생
        5. v0.6 §1.5 자연 누적 룰 — 다른 key_bond axes는 *재평가하지 않음*
            (고구·고아내·장씨 axes는 사건의 OCC만으로 자연 갱신)
```

### past — formative_relationships

```yaml
formative_relationships:
  - id: "father"
    type: "부친 (일찍 사망한 무관)"
    legacy: "무관 정체성의 원형."

  - id: "first_master"
    type: "어린 시절 첫 무술 사부 (이름 미상)"
    legacy: "사모를 처음 가르친 자. taboo의 *씨앗*. dormant_bond에도 등록."

  - id: "lu_qian_past"
    type: "유년기 죽마고우 (이미 처단됨)"
    legacy: |
      *신뢰의 원형이자 그 파괴의 원형*. key_bonds[Resolved]에도 동시 등록.
      v0.6 명확화: 현재 정체성 영향 강하므로 key_bonds 위치 정합. formative는 *과거 의미 깊이* 표시용.
```

### present — unresolved_tension

```yaml
unresolved_tension:
  - id: "ut_1_wife_fate"
    category: "관계적·죄책감"
    description: |
      장씨 안위 미확인. life_question에 가장 직접 닿는 미해결.
      Partnership: Separated + Status: Active이므로 재회 가능성 살아있음.

  - id: "ut_2_gao_unreachable"
    category: "외부적·구조적"
    description: |
      고구·고아내는 체제 정점. v0.6 ActionTrigger에서 BloodEnemy 처단 *blocked*.
      양산박 합류로 SystemicResistance 변형 표출.

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
    고아내가 장씨에게 다시 손을 뻗고 자결로 정조 지킴. 임충은 *몇 달 후* 소식.
    장씨 bond_status: Active → Deceased + Partnership: Separated 유지.
  trigger_condition: "ut_1_wife_fate 답을 찾을 때."

joyful_seed:
  description: |
    양산박에서 어린 시절 첫 사부의 가르침을 *다른 형태로* 전하는 노승 만남.
    dormant_bonds[0] 활성화 가능성.
  trigger_condition: "노승과의 만남 + 산신묘 처단의 *손의 무게*에 대한 새 해석."
```

---

# v0.6 검증 결과

## 1. 5개 key_bonds 분류 (v0.6 기준)

| 인물 | bond_kind | bond_status | partnership | v0.5 → v0.6 |
|---|---|---|---|---|
| 육겸 | Betrayer | Resolved | null | 변화 없음 |
| 고구 | Oppressor | Active | null | 변화 없음 |
| 고아내 | BloodEnemy | Active | null | 변화 없음 |
| 노지심 | null (SwornBrothers/Companion 후보) | Active | null | v0.6 검토에서 SwornBrothers 결 채택 |
| 장씨 | null | Active | Separated | **v0.6에서도 매핑 부재 — v0.7 후보** |

## 2. v0.6 ActionTriggerEvaluator 검증 — 핵심 변경

| 사례 | v0.5 처리 | v0.6 시스템 도출 |
|---|---|---|
| 양산박 합류 | "Oppressor 행동의 변형된 표출" 직관 | SystemicResistance feasibility 0.55 — 5차원 평가 결과 |
| 고아내 처단 보류 | "권력 보호막으로 보류" 직관 | DirectKill blocked (combined 0.18) → SystemicResistance 변형 |

★ 두 사례 모두 *직관 → 시스템 메커니즘* 환원. Bekay의 게임에서 임충의 양산박 합류는 *디자이너 스크립트 아닌 시스템 도출 결과*가 됨.

## 3. compass 자연 누적 룰 검증

임충은 *큰 compass 변화* 1회 (`tp_shanshenmiao`). 그러나 그 사건의 OCC가 *육겸 axes만* 직접 갱신 — 고구·고아내·장씨 axes는 *재평가 없이* 자연 누적. 모순 없음.

다른 transition_points의 `compass_change: null` 명시:
- marriage_event, tp_baihu_jietang, wife_violation_attempt, departure_xiushu, tp_yezhulin, yezhulin_rescue
- 모두 compass_change null이 정합 — 큰 사건들이지만 compass는 그대로 유지하다가 산신묘에서 *최종 변화*.

## 4. v0.6에서 임충 인스턴스가 노출하는 한계

### 한계 — 부부형 BondKind 부재

장씨에 대한 임충의 관계가 v0.6 11종 어디에도 정확 매핑 안됨. axes가 깊고(95/90/70/5) Partnership: Separated가 형식 보존하지만, *bond_kind 차원이 빈* 채로 남음.

v0.7 후보:
- 옵션 A: `Beloved` 또는 `LifePartner` variant 추가
- 옵션 B: BondKind와 직교한 별도 *romantic_bond* 슬롯 (Partnership 외)
- 옵션 C: 현재 처리 유지 (자유 텍스트 + Partnership으로 충분)

옵션 결정은 추가 인스턴스 검증 (예: 곽정-황용 Spouse 케이스) 후 권장.

---

## 변경 이력

| 버전 | 일자 | 변경 |
|------|------|------|
| v1.0 (v0.4) | 2026-05-04 | 초안 |
| v2.0 (v0.5) | 2026-05-04 | 세 차원 직교화 적용 |
| v3.0 (v0.6) | 2026-05-04 | 노지심 Companion vs SwornBrothers 후보 비교 (SwornBrothers 결 채택). 고구·고아내 ActionTrigger 5차원 feasibility 검증. compass_change null 명시적 정합. 부부형 BondKind 부재가 v0.6에서도 한계로 남음 (v0.7 후보). |
