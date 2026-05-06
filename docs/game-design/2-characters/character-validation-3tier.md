# 인물 스키마 검증 — 3 Tier 인스턴스

> Schema v0.2 검증용
> 작성일: 2026-05-04
> 목적: Tier 1·2·3 각 한 명씩 채워, 스키마가 모든 층에서 작동함을 검증.
> 인물: Tier 3 = 연청, Tier 2 = 객점 주인 노가(老柯), Tier 1 = 성문 위병 갑(甲)

---

# Tier 3 — 연청(燕青) v1.1

> 시점(snapshot): **방랍 토벌 후, 노준의에게 작별 직전.** 28~30세.
> 결정 확정: taboo = B, life_question = 후보 #2.

## Layer 1 — 본바탕

### identity

```yaml
id: "yan_qing"
name: "연청 (燕青)"
nicknames:
  - "낭자(浪子)"
  - "천교성(天巧星)"
era: "북송 휘종 치세"
stage_of_life: "장년기"
snapshot_time: "방랍 토벌 직후, 노준의 작별 직전"   # v0.3 신설 필드
```

### origin

```yaml
birthplace: "북경(대명부)"
social_origin: "고아 → 가신(家臣)"
kingdom_of_origin: "송"
family_background: |
  대명부의 평민 출신, 어려서 부모를 여읜 고아.
  당대 거부(巨富) 노준의(玉麒麟)에게 거두어져 양육됨.
  실질적 가족은 노준의 한 사람뿐. 친혈연의 기억은 희미.
```

### temperament — HEXACO 24 facet

```yaml
H_honesty_humility:
  sincerity: 70
  fairness: 75
  greed_avoidance: 95
  modesty: 80
E_emotionality:
  fearfulness: 30
  anxiety: 40
  dependence: 65
  sentimentality: 75
X_extraversion:
  social_self_esteem: 80
  social_boldness: 90
  sociability: 70
  liveliness: 75
A_agreeableness:
  forgiveness: 60
  gentleness: 65
  flexibility: 85
  patience: 80
C_conscientiousness:
  organization: 80
  diligence: 90
  perfectionism: 90
  prudence: 95
O_openness:
  aesthetic_appreciation: 90
  inquisitiveness: 75
  creativity: 85
  unconventionality: 85
```

### body

```yaml
physical_description: |
  28~30세이나 24~25세로 보이는 동안. 신장 약 165cm.
  관옥처럼 흰 피부, 별처럼 빛나는 눈, 연지 바른 듯 붉은 입술.
  당대 천하제일의 미남자. 늠름하나 우아한 비율.
signature_feature: |
  전신을 뒤덮은 정교한 문신(花繡).
  등·가슴엔 봉황과 운룡(雲龍)이 비단처럼 어우러진 형상.
  태안주 최고 장인이 새긴 "천하무이의 보물".
```

## Layer 2 — 현재 표현

### inner_compass — 가치의 세 면 ★ 확정

```yaml
inner_compass:
  compass: "공성명수신퇴(功成名遂身退) — 공을 이루었으니 물러난다"
  taboo: "내 몸(미모·문신)을 정치 도구로 쓰지 않는다"
  life_question: "내가 받은 무조건의 은혜를, 평생 갚을 수 있을까?"

  taboo_crystallization: "tp_5_li_shishi"  # 이사사 사건에서 결정화. v0.3 신설 필드.
```

### current_state

```yaml
current_state:
  pad:
    pleasure: -0.2
    arousal: 0.3
    dominance: 0.6
  dominant_emotion: "Resignation (체념적 결연)"
  active_focus: "노준의에게 마지막 충고 + 자기 길로 떠남"
```

### relationships

```yaml
key_bonds:
  - target: "노준의(盧俊義)"
    type: "양아버지·주인·지기"
    axes: { trust: 95, affinity: 90, respect: 90, wariness: 30 }
    note: "현재 충고 거절당함 — 관계의 *변형* 진행 중. 종료가 아님."
  - target: "송강(宋江)"
    type: "두령·정치적 거리"
    axes: { trust: 50, affinity: 60, respect: 70, wariness: 75 }
    note: "그의 귀순 정책은 따랐으나, 토사구팽 예감은 송강의 야심에서 옴."
  - target: "이사사(李師師)"
    type: "의자매·정치적 동맹"
    axes: { trust: 70, affinity: 65, respect: 60, wariness: 50 }
    note: "유혹을 의남매로 변환. 진심의 비율은 *끝까지 모호*."

dormant_bonds:
  - target: "어린 시절의 누군가 (구체 미정)"
    note: "기연 후보 — 게임 진행 중 채워질 빈 슬롯."
```

### voice

```yaml
voice:
  speech_register: "다층적 (격조 / 풍류 / 비장 / 거친)"
  vocabulary_level: "사대부 + 강호 + 양민 (3중)"
  tics:
    - "주인 앞에선 자신을 '소인(小人)'이라 칭함"
    - "절박할 땐 '주인님!'으로 시작하는 짧은 호소"
    - "외부에선 한자성어·고사 인용"
    - "결정적 순간엔 시(詩)로 답함"
  voice_anchors:
    - context: "충정의 절박함 (노준의 위기)"
      utterance: "주인님, 소인 연청이 여기 있습니다! 어서 정신을 차리십시오!"
    - context: "무인의 자신감 (씨름 시합 전)"
      utterance: "씨름이란 힘이 있으면 힘을 쓰고 힘이 없으면 지혜를 쓴다는 말이 있습니다. 연청이 입만 살아 있는 것이 아닙니다."
    - context: "유혹 거절 (이사사 앞)"
      utterance: "낭자께서 저를 과분하게 아껴주시니, 누님으로 모시겠습니다."
    - context: "비장한 호소 (황제 휘종 앞)"
      utterance: "신은 천리에 어긋나는 죄를 지어 감히 아뢸 수가 없습니다. 다만 형님과 형제들은 폐하의 성은을 입어 나라에 보답하기만을 고대해 왔습니다."
    - context: "조용한 결별 (노준의에게 마지막 절)"
      utterance: "주인님의 뜻이 그러하시다면 어쩔 수 없습니다. 저는 오늘로써 인연을 정리하고 떠나고자 합니다. 부디 몸조심하십시오. 이것이 저의 마지막 절입니다."
```

### titles

```yaml
titles:
  - "낭자(浪子)"
  - "천교성(天巧星)"
  - "양산박 보군두령"   # 현재 *해체 직전*
```

## Layer 3 — 시간축

### past — transition_points

```yaml
transition_points:
  - id: "tp_1_orphaned"
    age: "5~7"
    event: "부모 사망"
    impact:
      hexaco_shifts:
        - "E+ Sentimentality: 50 → 75"
        - "E+ Dependence: 50 → 70"
        - "A+ Patience: 60 → 80"
    inner_resolution: "혼자서는 살 수 없다 — 누군가에게 의지해야 한다"

  - id: "tp_2_taken_in"
    age: "7~10"
    event: "노준의에게 거두어짐"
    impact:
      hexaco_shifts:
        - "X+ Social Self-Esteem: 60 → 80"
        - "C+ Diligence: 70 → 90"
    inner_resolution: "이 은혜는 평생 갚는다"
    significance: "★ life_question의 *발생 지점*."

  - id: "tp_3_master_falls"
    age: "24"
    event: "노준의 가산 몰수, 거지 신세 — 연청도 함께 구걸"
    impact:
      hexaco_shifts:
        - "C+ Prudence: 70 → 90"
        - "H+ Modesty: 70 → 80"
        - "X+ Social Boldness: 80 → 90"
    inner_resolution: "주인의 영광이 무너져도 나는 따른다 — 그것이 진짜 충성"

  - id: "tp_4_liangshan"
    age: "25"
    event: "노준의 구출 → 양산박 가입"
    impact:
      hexaco_shifts:
        - "O+ Unconventionality: 70 → 85"
        - "X+ Social Boldness: 90 → 92"
    inner_resolution: "법 밖의 의(義)도 의이다"

  - id: "tp_5_li_shishi"
    age: "27"
    event: "이사사 의남매 결연 — 미모를 정치 도구로 쓸 *순간에 거부*"
    impact:
      hexaco_shifts:
        - "H+ Sincerity: 65 → 70"
      taboo_crystallization: true   # v0.3 신설 슬롯
    inner_resolution: "내 몸은 도구가 아니다. 나는 사람이다."

  - id: "tp_6_master_refuses"
    age: "28~30"
    event: "방랍 평정 후, 노준의가 동반 은퇴 권유를 *거절*"
    impact:
      hexaco_shifts:
        - "E- Dependence: 70 → 40"
        - "O+ Unconventionality: 85 → 90"
      compass_change:                # v0.3 신설 슬롯
        from: "주인 노준의를 보호한다"
        to: "공성명수신퇴 — 공을 이루었으니 물러난다"
    inner_resolution: "이제 내 길로 간다. 충성은 따름이지 함께 죽음이 아니다."
    significance: "★★ 가장 큰 전환점. compass 변화. life_question이 처음 *의식되는* 순간."
```

### past — formative_relationships

```yaml
formative_relationships:
  - id: "parents"
    type: "원형적 가족 (이미 사라짐)"
    legacy: "기억은 희미하나 첫 상실의 모형. 모든 이별에 그림자를 드리움."
  - id: "lu_junyi"
    type: "양아버지·주인·평생의 인연"
    legacy: "정체성의 기둥. 잃기엔 너무 큰 존재. 떠남으로써 비로소 *자기*가 됨."
  - id: "li_shishi"
    type: "정치적 자매·미답의 가능성"
    legacy: "유혹을 거부함으로써 자신을 정의."
```

### present — unresolved_tension

```yaml
unresolved_tension:
  - id: "ut_1_master_fate"
    category: "관계적·예감적"
    description: "노준의가 자기 충고를 거절했다. 그가 죽으면, 내 떠남이 옳았던 건가, 비겁한 건가?"
  - id: "ut_2_brothers_left"
    category: "관계적·죄책감"
    description: "양산박 동료 대부분 비극을 향함. 그들을 두고 떠나는 것이 옳은가? 함께 죽는 것이 의(義)인가?"
  - id: "ut_3_self_alone"
    category: "내부적·정체성"
    description: "평생 처음으로 *주인 없이* 사는 삶. 나는 누구인가?"
```

### future hooks

```yaml
tragic_seed:
  description: "노준의의 죽음 소식. 자기 예견이 *옳았음*을 확인. 옳음의 비통."
  trigger_condition: "노준의 사망 이벤트 시 PAD 비대칭 증폭 — life_question에 직접 닿음."

joyful_seed:
  description: "강호 어딘가에서 음악을 연주하던 밤, 한 여행자가 곡조에 멈춰 섬. 그 사람의 눈이 *어린 시절 잃은 누군가*를 닮았음."
  trigger_condition: "dormant_bond #1 활성화 시. Pillar 5 기연 트리거."
```

---

# Tier 2 — 객점 주인 노가(老柯)

> 가상 인물. 7국 시대 어느 작은 마을의 객점 주인. 명명된 NPC, 단발성 등장이지만 인상적인 만남.
> 검증 목적: Tier 2 *최소 필수 필드*가 인물을 실제로 살아있게 만드는가?

## Layer 1 — 본바탕

### identity

```yaml
id: "old_ke"
name: "노가(老柯)"   # 손님들이 그렇게 부름. 본명은 잊혀진 지 오래.
nicknames: []
era: "7국 시대 (구체 미정)"
stage_of_life: "노년기"
snapshot_time: "현재"
```

### origin

```yaml
birthplace: "변경의 작은 마을 (이름 없음)"
social_origin: "양민"
kingdom_of_origin: "(미정)"
family_background: "전직 군졸. 30년 전 전쟁에서 다리를 잃고 고향에 객점을 차림."
```

### temperament — HEXACO 24 facet (Tier 2도 풀로 채움)

```yaml
H_honesty_humility:
  sincerity: 65
  fairness: 80          # 손님 차별 없음
  greed_avoidance: 70
  modesty: 75
E_emotionality:
  fearfulness: 50       # 군 출신, 무서움 적음
  anxiety: 60
  dependence: 30
  sentimentality: 80    # 군 시절 동료들 기억 많음
X_extraversion:
  social_self_esteem: 60
  social_boldness: 50
  sociability: 75       # 손님 듣는 직업
  liveliness: 55
A_agreeableness:
  forgiveness: 70
  gentleness: 65
  flexibility: 60
  patience: 85         # 객점 주인 필수 덕목
C_conscientiousness:
  organization: 80
  diligence: 80
  perfectionism: 50
  prudence: 75
O_openness:
  aesthetic_appreciation: 50
  inquisitiveness: 70   # 손님들 사연을 듣는 호기심
  creativity: 40
  unconventionality: 35
```

### body

```yaml
physical_description: "60대 후반. 마르고 키 작음. 왼쪽 다리가 의족(나무)."
signature_feature: "걸을 때 의족이 나무 바닥을 두드리는 *딱-딱* 소리. 손님들이 그 소리로 그가 다가옴을 안다."
```

## Layer 2 — 현재 표현

### inner_compass (Tier 2: compass + taboo 필수, life_question 선택)

```yaml
inner_compass:
  compass: "오는 손님 누구든 따뜻한 한 끼와 잠자리를 준다"
  taboo: "전쟁 이야기는 *내가 먼저* 꺼내지 않는다"  # 자기 트라우마 다루기
  # life_question: 생략 (Tier 2)
```

### current_state

```yaml
current_state:
  pad: { pleasure: 0.1, arousal: 0.0, dominance: 0.4 }
  dominant_emotion: "Contentment (잔잔한 만족)"
  active_focus: "오늘 저녁 손님맞이"
```

### relationships

```yaml
key_bonds:
  - target: "(고인이 된 군 동료들)"
    type: "묻힌 형제들"
    axes: { trust: 100, affinity: 100, respect: 100, wariness: 0 }
    note: "이미 죽은 자들과의 관계. 마음 안에서만 작동."
dormant_bonds: []
```

### voice (Tier 2: 기본만)

```yaml
voice:
  speech_register: "소박"
  vocabulary_level: "양민"
  tics:
    - "손님 자리 안내할 때 '편히 드시오, 편히' 두 번 반복"
    - "맥주 따를 때 한쪽 눈을 살짝 감음 (옛 군 습관)"
  # voice_anchors: 생략 (Tier 2 선택)
```

### titles

```yaml
titles: []
```

## Layer 3 — 시간축 (Tier 2: transition_point 1개)

### past

```yaml
transition_points:
  - id: "tp_lost_leg"
    age: "37"
    event: "변경 전쟁에서 다리를 잃고 의병 제대"
    impact:
      hexaco_shifts:
        - "E+ Sentimentality: 60 → 80"
        - "X- Liveliness: 70 → 55"
    inner_resolution: "남은 시간은 살아남은 자로서 살자"
```

### present

```yaml
unresolved_tension:
  - id: "ut_1_old_uniform"
    description: "벽장 깊숙이 둔 옛 군복을 30년째 버리지 못함. 왜 못 버리는지 본인도 모름."
```

### future hooks (선택 — 비워둠)

```yaml
tragic_seed: null
joyful_seed: null
```

---

# Tier 1 — 성문 위병 갑(甲)

> 가상 인물. 어느 도시 성문의 무명 위병. 플레이어가 한 번 스쳐 지나갈 NPC.
> 검증 목적: Tier 1의 *최소 필드*가 캐릭터를 *조금이라도* 살아있게 만드는가?

## Layer 1 — 본바탕 (최소)

### identity

```yaml
id: "guard_jia_001"
name: "위병 갑(甲)"   # 이름 없음. 시스템 식별용.
era: "7국 시대"
stage_of_life: "장년기"
```

### origin

```yaml
social_origin: "양민"
# birthplace, kingdom_of_origin, family_background: 생략
```

### temperament — HEXACO 6 factor만 (Tier 1)

```yaml
# 24 facet 생략. 6 factor 평균값만.
H: 50
E: 60
X: 40
A: 55
C: 70   # 군 규율
O: 35   # 변화 싫어함
```

### body

```yaml
# 생략 가능. 단 한 줄로:
physical_description: "흔한 체격. 갑옷에 가려 외모 인상 약함."
```

## Layer 2 — 현재 표현 (최소)

### inner_compass (Tier 1: compass 한 줄만)

```yaml
inner_compass:
  compass: "교대 시간까지 무사히 — 그러고 집에 가서 자식 얼굴 본다"
```

### current_state, relationships, voice, titles
> Tier 1: 생략. 필요시 자유 추가.

## Layer 3 — 시간축
> Tier 1: 생략.

---

# 검증 결과 — 스키마 v0.2 → v0.3 보정 사항

세 인물을 채우면서 발견된 스키마의 약점:

## 발견 1: snapshot_time 필드 필수

연청처럼 시간축이 긴 인물은 *언제의 인물인가*를 명시해야 같은 인물의 다른 인스턴스를 구분 가능. **v0.3에서 Tier 2·3에 `snapshot_time` 필드 신설.** Tier 1은 보통 "현재" 고정이라 생략.

## 발견 2: transition_points에 compass_change 슬롯

단순히 transition_point가 일어났다는 사실만 적으면 *그것이 인물을 어떻게 바꿨는가*가 약함. **v0.3에서 transition_points 항목에 `impact.compass_change: { from, to }` 슬롯 신설.** 모든 점에 필수는 아님 — *compass가 실제로 변한 점*에만 사용.

## 발견 3: transition_points에 taboo_crystallization 슬롯

taboo도 단순히 "있다"가 아니라 *어디서 결정화되었는가*가 깊이를 만듦. **v0.3에서 `impact.taboo_crystallization: true|false` 슬롯 신설.** 인물의 *taboo가 처음 명확해진 사건*에 표시.

## 발견 4: Tier 1·2의 *비어있음*도 의미가 됨

Tier 1 위병 갑의 compass "자식 얼굴 본다"가 한 줄이지만, 그게 LLM 연기에 충분한 안내가 됨. Tier 1을 *비워두는 것*이 아니라 *최소한*으로 채우는 패턴이 작동함을 검증.

## 발견 5: 인스턴스 검증의 비대칭

Tier 1 위병 갑은 채우는 데 5분, Tier 2 노가는 15분, Tier 3 연청은 60분 분량. **이게 정확히 의도한 비대칭**이고, 너의 1인 개발에서 NPC 100명 만들 때 이 비율이 시간을 살린다.

---

## 다음 단계 제안

스키마는 v0.3으로 가볍게 보정하면 안정. 이제 *인물 카테고리* 다음의 무엇을 할지 결정할 차례:

1. **인물 카테고리의 sub-문서 작성** — `relationships.md` (관계 다축 시스템), `reputation.md` (평판·칭호 시스템), `psychology-engine.md` (npc-mind-rs 연결)
2. **2번 버킷 다른 카테고리** — `skills.md` (무공 분류 체계)
3. **3번 버킷으로 이동** — `narrative-patterns.md` (이미 정리된 SP/WK 패턴 재배치)
4. **1번 버킷으로 이동** — `factions.md` (문파) 또는 `geography.md`

내 추천은 **1번 → relationships.md**. 인물 스키마에서 자꾸 참조하던 "관계 다축 시스템"의 본체를 정의해야 인물 인스턴스가 *완전*해짐. 게다가 Pillar 3(관계가 곧 시스템)이 게임의 중심 메커니즘이라 다른 모든 카테고리에 앞서야 함.

어디로 갈까?
