# 인물 스키마 v0.4 검증 — 임충 (林沖)

> 작성일: 2026-05-04
> 검증 대상: `_schema.md` v0.4 + `relationships.md` v0.4
> 위치: `docs/game-design/2-characters/character-validation-v0_4.md`
> 추후 정식 인물 디렉토리(`characters/lin_chong.md`)로 이동 가능.

## 검증 목적

v0.4에서 신설·갱신된 슬롯이 실제 인물 인스턴스에서 *자연스럽게* 작동하는가를 검증한다.
임충 한 명의 풍부한 관계망으로 다음을 모두 시연:

| v0.4 슬롯 | 검증 위치 |
|---|---|
| `snapshot_time` | "산신묘 사건 직후, 양산박 가입 *전*" |
| `taboo_crystallization` | `tp_yezhulin` (야저림에서 호송관 살려준 사건) |
| `compass_change` | `tp_shanshenmiao` (체제 순응 → 체제 저항) |
| **음수 axes** | 육겸 (-100/-90/-100/100), 고구·고아내 다수 |
| `type_history` | 5개 key_bond 모두 |
| `transformation_events` | 4개 사건 cross-reference |
| `bond_kind: Betrayer` | 육겸 — 음극 즉시 진입 + type_history 의존 |
| `bond_kind: Oppressor` | 고구 — 직접 만나지 못한 거대 권력 |
| `bond_kind: BloodEnemy` | 고아내 — 가족(아내)을 노린 자 |
| `bond_kind: null` (임계 *근접*) | 노지심 — **양극 30일 게이트 검증** |
| `bond_kind: null` (펑쩌 패턴) | 장씨 — **시스템 한계 발견** (부부형 bond 부재) |
| `dormant_bonds` | 어린 시절 첫 사부 (기연 후보) |

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
snapshot_time: "산신묘 사건 직후, 양산박 가입 *전*"   # ★ 30대 후반
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
  sincerity: 80         # 정직한 무관
  fairness: 85
  greed_avoidance: 75
  modesty: 75
E_emotionality:
  fearfulness: 30       # 무인, 두려움 적음
  anxiety: 60           # ↑ 산신묘 후 트라우마성 불안
  dependence: 50
  sentimentality: 80    # 깊은 감정 (아내·은인)
X_extraversion:
  social_self_esteem: 70
  social_boldness: 70   # 폭발 시 강함
  sociability: 50       # 무뚝뚝한 편
  liveliness: 45
A_agreeableness:
  forgiveness: 55       # 무고에게는 ↑, 가해자에게는 ↓ (분리)
  gentleness: 60
  flexibility: 50
  patience: 95          # ★ 인내의 화신
C_conscientiousness:
  organization: 85
  diligence: 90
  perfectionism: 80
  prudence: 90          # ★ "인내 → 폭발" 패턴의 시스템적 근거
O_openness:
  aesthetic_appreciation: 50
  inquisitiveness: 50
  creativity: 55        # 전술적 창의
  unconventionality: 55 # ↑ 산신묘 후 체제 밖으로 (35→55)
```

### body

```yaml
physical_description: |
  30대 후반. 표범의 머리에 둥근 눈, 호랑이 같은 수염.
  날카로운 눈매, 단단한 체격. 현재 얼굴에 *낙인(刺字)* — 창주 유배 시 새겨진 죄인의 표식.
signature_feature: |
  일장팔척(一丈八尺)의 장사모(丈蛇矛). 뱀처럼 굽이치는 날.
  사모를 휘두를 때 "뱀이 혀를 놀리듯" 빠르고 변화무쌍.
  현재는 산신묘에서 빼앗은 적의 무기를 임시로 갖고 있음 (자기 사모는 백호절당에서 압수됨).
```

## Layer 2 — 현재 표현

### inner_compass — 가치의 세 면

```yaml
inner_compass:
  compass: "내 손으로 의(義)를 행한다 — 부패한 권력의 칼이 되지 않는다"
  taboo: "무고한 자에게 칼을 휘두르지 않는다"
  life_question: "나는 다시 충성할 가치가 있는 무엇을 만날 수 있을까?"

  taboo_crystallization: "tp_yezhulin"   # ★ 야저림 사건에서 결정화
```

> **compass 변화 직후.** 산신묘 사건 *전*의 compass는 "법과 군의 명을 따르며 가족을 지킨다"였음. `tp_shanshenmiao`의 `compass_change` 참조.

> **life_question의 sub-text:** 충성형 인간이 충성의 대상을 잃었을 때의 깊은 질문. 본인은 의식하지 못함. 양산박이 답이 될 수 있을지가 향후 화두 — 송강과의 관계가 이 질문에 *부분적 답*을 줄지, *반복된 배신*이 될지가 인물의 운명.

### current_state

```yaml
current_state:
  pad:
    pleasure: -0.6      # 깊은 슬픔·고립
    arousal: 0.4        # 여전히 경계 상태 (적이 다 죽지 않음)
    dominance: 0.5      # 직접 처단으로 자기 효능감 부분 회복
  dominant_emotion: "Resentment + Resolution (한과 결연의 공존)"
  active_focus: "양산박을 향해 도주 + 다음 행동 결정"
```

### relationships

#### key_bonds

```yaml
key_bonds:

  # ──────────────────────────────────────────────────
  # 1. 육겸 — Betrayer (음극 즉시 진입)
  # ──────────────────────────────────────────────────
  - target: "lu_qian"
    type: "죽마고우 → 적·처단 대상 (이미 처단됨)"
    type_history:
      - { since: "유년기",                  type: "죽마고우" }
      - { since: "고구 매수 후",             type: "은밀한 배신자 (임충은 모름)" }
      - { since: "shanshenmiao_event",     type: "적·처단 대상" }
    transformation_events:
      - { event_id: "shanshenmiao_event", new_type: "적·처단 대상" }
    axes: { trust: -100, affinity: -90, respect: -100, wariness: 100 }
    bond_kind: "Betrayer"
    bond_since: "shanshenmiao_event"
    note: |
      이미 산신묘에서 임충이 *직접 처단*. 시신은 남았으나 관계 type은 영원히 type_history에 보존.
      *죽마고우를 직접 죽인 사실*의 무게가 임충의 dominant_emotion에 그림자를 드리움.
      Betrayer 추가 조건 (type_history에 가까운 type 존재) 충족 — 유년기 "죽마고우"가 핵심.

  # ──────────────────────────────────────────────────
  # 2. 고구 — Oppressor (직접 만나지 못한 거대 권력)
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
    bond_since: "baihu_jietang_event"
    note: |
      직접 만난 적은 백호절당 한 번. 그 후 모든 가해는 *대리인을 통해*.
      임충은 고구를 *체제 자체*로 인식. respect=-50 — 권력자로서의 격은 인정하지 않으나, 그 *권력의 구조*는 인정 (Oppressor의 respect 범위 -20~+30 약간 벗어남, 그러나 ㅁ방향 정합).
      직접 처단은 현실적으로 불가 — 양산박 합류는 이 적의의 *변형된 표출*.

  # ──────────────────────────────────────────────────
  # 3. 고아내 — BloodEnemy (가족=아내를 노린 자)
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
    bond_since: "wife_violation_attempt"
    note: |
      혈적 임계 충족 (trust ≤-80, affinity ≤-80, wariness ≥70). 가족(아내)을 직접 노렸다는 점에서 BloodEnemy.
      단 — 고구의 양아들이라는 *권력 보호막* 때문에 처단 행동이 즉시 emit되지 않고 *보류 상태*.
      ★ 시스템 노출: BondKind는 *분류*까지, 실제 *행동 가능성*은 별도 평가 필요. (정치권력·물리거리·NPC 자기보호 본능 등의 변수.)

  # ──────────────────────────────────────────────────
  # 4. 노지심 — SwornBrothers 임계 *근접*, 30일 게이트 검증
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
    bond_since: null
    note: |
      ★★ SwornBrothers 임계값 (trust ≥80, affinity ≥70, respect ≥60, wariness ≤30) *모두 충족*.
      그러나 *연속 30일 유지*는 미달 — 야저림 사건 후 함께 도주한 며칠뿐.
      카운터 흐름 중. 이게 **양극 진입의 시간 게이트** 검증 사례.

      산신묘 후 도망치며 노지심과 헤어졌으므로 (수호전 원작: 노지심 동경 잔류, 임충 혼자 양산박행) —
      *카운터는 곧 리셋될 가능성이 더 높음.* 미래의 양산박 재회에서 카운트가 *처음부터* 다시 시작됨.

      → 시스템적 함의: "야저림 한 사건"으로는 SwornBrothers가 되지 않음. 그 후의 *일상의 시간*이
      필요한데, 임충의 운명은 그 일상을 *허락하지 않음*. 이게 임충-노지심 관계의 비극성.

  # ──────────────────────────────────────────────────
  # 5. 장씨 (아내) — bond_kind null, 펑쩌 패턴 + 시스템 한계 노출
  # ──────────────────────────────────────────────────
  - target: "zhang_shi"
    type: "아내 → 휴서 후 별거 → 다시 만날 수 없는 사람"
    type_history:
      - { since: "결혼 후",                 type: "아내" }
      - { since: "baihu_jietang_event",   type: "지키지 못한 아내 (유배 결정 후)" }
      - { since: "departure_xiushu",      type: "아내 → 휴서 후 별거 → 다시 만날 수 없는 사람" }
    transformation_events:
      - { event_id: "departure_xiushu", new_type: "휴서 후 별거" }
    axes: { trust: 95, affinity: 90, respect: 70, wariness: 5 }
    bond_kind: null
    bond_since: null
    note: |
      ★★ axes는 *모두 양수의 깊은 사랑*. 그러나 type 자체가 "함께할 수 없음".
      펑쩌-손유탕 패턴의 변종 — 거기는 -50/+40/-10/60 (배신 후 잔존), 여기는 95/90/70/5 (이별 후 잔존).
      4축 시스템이 *모순된 감정의 공존*을 표현.

      ★★★ 시스템 한계 노출:
      trust 95·affinity 90이면 SwornBrothers·LoyalRetainer 임계는 충족. 그러나 *type이 부부*라
      현재 enum 8종(SwornBrothers/MasterDisciple/Soulmate/LoyalRetainer + 4 enemies) 어느 것에도
      *완벽히* 맞지 않음. Soulmate가 가장 가깝지만 affinity 90 임계 *경계*이고, 부부의 본질은
      "함께 산다 + 자녀 + 사회적 결속"이라 Soulmate(영혼의 동반자)와 의미 결이 다름.

      → v0.5 확장 후보: `LifePartner` 또는 `Beloved` variant. 또는 BondKind 외 별도 슬롯
      (`romantic_bond`?) 검토. 일단 v0.4에서는 자유 텍스트 type으로 보존.
```

#### dormant_bonds

```yaml
dormant_bonds:
  - target: "어린 시절 첫 무술 사부 (이름 미상)"
    last_contact: "age 12~13"
    fragment: |
      처음 사모를 잡았을 때, 사부가 손 위에 손을 얹어주던 무게.
      사부의 얼굴은 흐릿하나, "사모는 사람을 *지키는* 것이지 *위협하는* 것이 아니다"라는
      한 마디만 또렷이 남음.
    note: |
      기연 후보. 임충이 산신묘 후 *처음으로 처단을 직접 행함*에 마음이 흔들릴 때
      이 기억이 떠오를 가능성. 양산박에서 비슷한 가르침을 주는 노승·은자와의 만남이
      기연 트리거가 될 수 있음. life_question("충성할 가치가 있는 무엇")에 부분적 답을
      *암시*하는 만남으로 발현 가능.
```

### voice

```yaml
voice:
  speech_register: "정중함 ↔ 냉혹함 (양극 공존, 현재는 후자 우세)"
  vocabulary_level: "사대부 + 무관 용어 (현재 강호 어휘 섞임 시작)"
  tics:
    - "체제 안에서: 상대를 존칭으로 대함 ('태위', '대인', '현처' 등)"
    - "현재: '네 놈', '간신적자' 같은 도덕적 비난 어휘"
    - "전투 호령은 짧고 단정적 ('에잇!', '기다려라!')"
    - "결정적 순간 — 자기 호명 ('양산박의 표자두 임충이 여기 있다!')"
  voice_anchors:
    - context: "정중한 무관 (체제 안 시절, 아내에게)"
      utterance: "현처(賢妻), 내 말을 들어보오. 나는 운이 사나워 노 태위의 모함을 받았소."
    - context: "도덕적 호소 (휴서 작성, 장인에게)"
      utterance: "장인어른, 제가 아내와 헤어지려는 것은 사랑하지 않아서가 아닙니다."
    - context: "각성 후 냉혹한 처단 (산신묘, 육겸에게)"
      utterance: "너 같은 놈은 살려둘 가치도 없다. 네 놈의 심장을 꺼내 내 억울함을 씻으리라!"
    - context: "양산박 시대의 호령 (미래 시점, sub-text 참고)"
      utterance: "양산박의 표자두 임충이 여기 있다! 비겁하게 숨지 말고 내 사모를 받아라!"
    - context: "겸손한 의리 (조개·송강에 대한 충성, 미래 시점)"
      utterance: "형님께서 가시는 길이라면 이 임충, 말 앞의 졸개라도 되어 끝까지 따르겠나이다."
```

### titles

```yaml
titles:
  - "표자두(豹子頭)"
  - "천웅성(天雄星)"
  - "(실효: 80만 금군교두 — 백호절당 사건 후 *박탈됨*)"
```

## Layer 3 — 시간축

### past — transition_points

```yaml
transition_points:

  - id: "tp_baihu_jietang"
    age: "30대 중반"
    event: "백호절당 함정 — 보검 구경 핑계로 군사 기밀 구역 유인 → 암살범으로 몰림 → 유배 결정"
    impact:
      hexaco_shifts:
        - "C+ Prudence: 80 → 90 (체제에 대한 신뢰 무너지며 자기 신중성 ↑)"
        - "E+ Anxiety: 30 → 50 (트라우마성 불안 시작)"
    inner_resolution: "체제는 나를 보호하지 않는다. 그러나 아직 *법 안에서* 결백을 증명하리라."
    significance: "★ 첫 충격. 아직 체제 안에 머물려 함."

  - id: "wife_violation_attempt"
    age: "30대 중반 (백호절당 직전)"
    event: "고아내가 장씨에게 흑심을 품고 접근 시도 — 임충 도착으로 미수에 그침"
    impact:
      hexaco_shifts:
        - "E+ Anxiety: 30 → 35"
        - "A- Forgiveness: 60 → 55 (가해 시도자에 대한 비용서)"
    inner_resolution: "내 가족을 노리는 자는 결코 용서하지 않는다."
    significance: "고아내 → BloodEnemy 진입의 시작점."

  - id: "departure_xiushu"
    age: "30대 중반 (유배 길 직전)"
    event: "장씨에게 휴서를 써줌 — 자기 부재 중 그녀가 자유롭게 살 수 있도록"
    impact:
      hexaco_shifts:
        - "E+ Sentimentality: 70 → 80 (이별의 깊이)"
    inner_resolution: "내가 그녀를 지키지 못한다. 차라리 그녀를 자유롭게."
    significance: "장씨 type 변화의 결정적 사건. *사랑은 살아있으나 함께 갈 수 없는* 형태."

  - id: "tp_yezhulin"
    age: "30대 중반"
    event: "야저림에서 호송관 동초·설패의 살해 시도 → 노지심에게 구출 → 노지심의 권유에도 호송관 *살려줌*"
    impact:
      hexaco_shifts:
        - "A+ Forgiveness 양면화: 무고한(=명령 받은) 자에 ↑, 가해자에 ↓"
        - "X+ Social Boldness: 65 → 70"
    inner_resolution: "내 손에 떨어진 무고한 자는 죽이지 않는다. 진짜 적은 *위에* 있다."
    significance: "★★ taboo_crystallization 지점. '무고한 자에게 칼을 휘두르지 않는다'가 처음 명확."

  - id: "yezhulin_rescue"
    age: "30대 중반"
    event: "야저림 — 노지심이 나무 뒤에서 나타나 호송관 제압, 임충의 생명 구함"
    impact:
      hexaco_shifts:
        - "E- Dependence: 50 → 45 (예상치 못한 도움이 *의존*보다 *연대* 인식으로)"
    inner_resolution: "이 자는 거짓이 없다. 진심으로 나를 위해 칼을 든 사람."
    significance: "노지심에 대한 axes 즉시 갱신 (Gratitude + Admiration). SwornBrothers 임계 도달의 첫걸음."

  - id: "tp_shanshenmiao"
    age: "30대 중반"
    event: "창주 초료장 방화 → 산신묘 대피 → 우연히 육겸·부안 등의 자기 살해 자랑 엿들음 → 죽마고우 배신 확인 → *직접 처단*"
    impact:
      hexaco_shifts:
        - "O+ Unconventionality: 35 → 55 (체제 밖으로 나가는 결단)"
        - "X+ Social Boldness: 70 → 80"
        - "E+ Anxiety: 50 → 60 (새로운 트라우마)"
      compass_change:
        from: "법과 군의 명을 따르며 가족을 지킨다"
        to:   "내 손으로 의(義)를 행한다 — 부패한 권력의 칼이 되지 않는다"
    inner_resolution: "더 이상 위선의 규칙에 얽매이지 않는다. 진짜 의(義)는 내가 정한다."
    significance: |
      ★★★ 최대 전환점. compass 변화. *체제 순응 → 체제 저항*.
      육겸 → Betrayer 즉시 진입. 죽마고우를 직접 죽인 사실은 *영원한 그림자*.
```

### past — formative_relationships

```yaml
formative_relationships:
  - id: "father"
    type: "부친 (일찍 사망한 무관)"
    legacy: |
      무관 정체성의 원형. "무예는 사람을 지키는 것"이라는 가르침의 출처.
      아버지를 일찍 잃었기에 *체제(군)*가 부친 역할을 대신함 — 그 체제가 자기를 짓밟았을 때
      충격이 더 큼.

  - id: "first_master"
    type: "어린 시절 첫 무술 사부 (이름 미상)"
    legacy: |
      사모를 처음 가르친 자. taboo의 *씨앗*을 심음. 현재 dormant_bond에도 등록.
      formative와 dormant 양쪽에 등록 — 과거 의미 + 미래 활성화 가능성 동시.

  - id: "lu_qian_past"
    type: "유년기 죽마고우 (이미 처단됨)"
    legacy: |
      *신뢰의 원형이자 그 파괴의 원형*.
      향후 임충이 누구를 깊이 신뢰하는 데 시간이 더 걸리게 만들 *각인*.
      현재 key_bonds에도 -100/-90/-100/100으로 활성. formative와 key 양쪽 등록.
```

### present — unresolved_tension

```yaml
unresolved_tension:
  - id: "ut_1_wife_fate"
    category: "관계적·죄책감"
    description: |
      아내 장씨를 *지키지 못했고*, 휴서까지 써서 *떠나보냈다*. 그녀가 살아있는지,
      고아내가 다시 노리는지, 알 길이 없다. life_question에 가장 직접적으로 닿는 미해결.

  - id: "ut_2_gao_unreachable"
    category: "외부적·구조적"
    description: |
      고구·고아내는 *체제의 정점*에 있어 직접 처단 불가. 양산박 합류는 이 적의의 *간접 표출*.
      진짜 결판은 가능한가? BloodEnemy(고아내) 처단 트리거가 *보류 상태*인 시스템적 표현.

  - id: "ut_3_self_doubt"
    category: "내부적·정체성"
    description: |
      30년 신뢰한 죽마고우(육겸)가 가짜였다면, 내가 *지금 신뢰할* 수 있는 사람은 누구인가?
      노지심은 진심인가? 내가 또 잘못 보는 것은 아닌가?
      → 노지심 SwornBrothers 카운터가 30일을 채우지 못하는 *내적 이유*이기도 함.
```

### future hooks

```yaml
tragic_seed:
  description: |
    고아내가 임충 부재 중 장씨에게 다시 손을 뻗고, 장씨가 자결로 정조를 지킴.
    임충은 *몇 달 후에야* 그 소식을 들음.
  trigger_condition: |
    `ut_1_wife_fate`가 *답을 찾을 때*. PAD 비대칭 증폭 — life_question에 직접 닿음.
    compass의 "의(義)"가 *실패한 의*로 재정의됨. taboo가 흔들릴 위기
    (무고한 자=아내를 *결과적으로* 못 지킴).

joyful_seed:
  description: |
    양산박에서 어린 시절 첫 사부의 가르침을 *다른 형태로* 전하는 노승을 만남
    (노지심? 또는 별도 인물). 사모를 *어떻게* 다룰지에 대한 새로운 의미 발견.
  trigger_condition: |
    `dormant_bonds[0]` 활성화. 산신묘에서 처단을 직접 행한 *손의 무게*가
    노승의 가르침으로 *재해석*되며 부분적 평안. life_question에 부분적 답.
```

---

# 검증 결과 — v0.4 시스템이 작동하는가?

## 검증된 슬롯 (성공)

5개 key_bond에서 BondKind 8 variants 중 4개 직접 사용 + 2가지 의미 있는 null 케이스:

| key_bond | bond_kind | 검증되는 시스템 동작 |
|---|---|---|
| 육겸 | **Betrayer** | ✅ 음극 즉시 진입 + type_history 의존 (Betrayer 추가 조건) |
| 고구 | **Oppressor** | ✅ 직접 만나지 못한 거대 권력. respect 약간 음수 (Oppressor 범위 -20~+30 경계). |
| 고아내 | **BloodEnemy** | ✅ 가족(아내)을 노린 자. 임계 충족하지만 처단 행동 *보류* (잠재 트리거). |
| 노지심 | **null (임계 *근접*)** | ✅ **양극 30일 게이트 검증.** 4축 임계 충족이지만 시간 미달 → bond_kind null. |
| 장씨 | **null (펑쩌 변종)** | ✅ axes 모두 양수의 깊은 사랑이지만 type이 "함께할 수 없음". |

기타 슬롯도 모두 자연스럽게 사용됨 — `snapshot_time`, `taboo_crystallization`, `compass_change`, `type_history` (5개 모두), `transformation_events` (4개), `dormant_bonds`.

## 발견된 시스템 한계 (v0.5 후보)

### 한계 1: dyadic romantic bond 부재

장씨에 대한 임충의 관계는 axes (95/90/70/5)가 SwornBrothers·LoyalRetainer 임계를 *충족*하나, 의미 결이 다름. 부부·연인 관계는 현재 enum 어디에도 정확히 매핑되지 않음.

**v0.5 후보:**
- 옵션 A: `BondKind`에 `LifePartner` 또는 `Beloved` variant 추가.
- 옵션 B: BondKind 외 별도 슬롯 `romantic_bond` 도입 (BondKind와 직교 가능 — 영혼의 동반자이면서 부부일 수 있음).

옵션 B가 더 표현력 높지만 슬롯 추가. 옵션 A가 더 단순하지만 enum 늘어남. 선택은 다음 검증 인스턴스(예: 와호장룡 수련-이모백) 후 결정 권장.

### 한계 2: BondKind는 *분류*까지, *행동 가능성*은 별도 평가

고아내(BloodEnemy)는 임계를 충족하지만 *권력 보호막*으로 처단 행동이 emit되지 않음. 시스템이 BondKind를 분류한다고 *바로 행동*이 트리거되는 건 아님. 다음 변수들이 별도 평가되어야 함:

- 정치·물리적 거리
- NPC 자기보호 본능
- 동행 NPC의 만류
- 현재 PAD 상태 (intensity 충분한가)

→ 별도 시스템 필요: **Action Trigger Evaluator** (가칭). BondKind는 *동기 부여*까지, *실행 가능성* 평가는 별개. v0.5에서 설계.

### 한계 3: compass 변화 *직후*의 axes 일관성

임충의 compass가 산신묘에서 막 변함. 그러나 axes는 그 이전 사건들(백호절당, 야저림)에서 누적된 값. compass 변화 *순간*에 axes 전체를 재평가해야 하는가, 아니면 자연 누적으로 충분한가?

현재 v0.4: 자연 누적. 문제 없어 보이나, 큰 compass 변화 시 *모든 key_bond의 재평가*가 필요한 케이스가 있을지 추적 필요.

## 결론

v0.4 스키마는 임충의 풍부한 관계망을 *왜곡 없이* 표현. 핵심 시스템 결정(직교 4축, 음수, BondKind 8종, 양극·음극 비대칭 진입)이 모두 의미 있게 작동. 

v0.5에서 우선 다룰 항목: dyadic romantic bond, Action Trigger Evaluator. 이 두 가지는 임충 인스턴스 작성 *과정 자체*에서 발견된 한계로, 다음 인물 인스턴스(예: 와호장룡 수련, 또는 무송) 작성 시 더 구체화 가능.

---

## 변경 이력

| 버전 | 일자 | 변경 |
|------|------|------|
| v1.0 | 2026-05-04 | 초안. 임충 Tier 3 풀 인스턴스 작성. v0.4 스키마 검증 결과 + 발견된 3가지 시스템 한계 정리. |
