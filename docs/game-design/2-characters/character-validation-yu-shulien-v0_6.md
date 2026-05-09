# 인물 스키마 v0.6 검증 — 장년기 유수련 (兪秀蓮)

> 작성일: 2026-05-04
> 검증 대상: `_schema.md` v0.6 + `relationships.md` v0.6 + `action_triggers.md` v0.1
> 위치: `docs/game-design/2-characters/character-validation-yu-shulien-v0_6.md`
> 이전 버전: v0.5 (`character-validation-yu-shulien-v0_5.md` 폐기)
> 동반 인스턴스: 노년기 수련 (`character-validation-yu-shulien-elder-v0_6.md`)

## v0.6 변경 요약

장년기 수련 인스턴스는 v0.6에서 *변화 거의 없음*. 변경 위치만 정리:

| 영역 | v0.5 | v0.6 |
|---|---|---|
| 5개 bond_kind 분류 | 모두 v0.6 11 variants에서도 동일 결정 | **변화 없음** |
| 유태보 — Companion 가능성 검토 | (해당 없음) | **장년기에는 affinity 미달**로 진입 불가 명시 |
| ActionTrigger 검증 | (해당 없음) | 옥교룡 변경 재회 *전 시점*이라 직접 사례 없음 |

장년기 수련은 *Soulmate (이모백)*와 *Mentor (옥교룡 Reactivating)*의 핵심 검증 인스턴스. v0.6에서 분류 변화 없이 *기존 시스템 검증의 정합성만 재확인*.

---

# 유수련(兪秀蓮) — 장년기 인스턴스

## Layer 1 — 본바탕

### identity

```yaml
id: "yu_shulien"
name: "유수련(兪秀蓮)"
nicknames: ["쌍도여협(雙刀女俠)", "표국의 여주인"]
era: "청대 (왕도려 학철오부곡 세계관)"
stage_of_life: "장년기 → 노년기 진입 직전"
snapshot_time: |
  이모백 사망 후 약 3~5년. 옥교룡이 변경에서 살아있다는 떠도는 소문을 막 들은 직후.
  표국 운영은 후배에게 부분 위임 시작. 아직 양녀(춘설병)는 만나지 못함.
```

### origin

```yaml
birthplace: "북경"
social_origin: "양민 → 표국주(運局主)"
kingdom_of_origin: "청"
family_background: |
  명문가는 아니나 양민 중 *기예 있는 집안*. 부친이 북경에서 표국 운영.
```

### temperament — HEXACO (v0.5와 동일)

```yaml
H_honesty_humility: { sincerity: 90, fairness: 90, greed_avoidance: 85, modesty: 85 }
E_emotionality:     { fearfulness: 25, anxiety: 50, dependence: 35, sentimentality: 90 }
X_extraversion:     { social_self_esteem: 70, social_boldness: 60, sociability: 60, liveliness: 40 }
A_agreeableness:    { forgiveness: 75, gentleness: 75, flexibility: 70, patience: 95 }
C_conscientiousness:{ organization: 90, diligence: 90, perfectionism: 80, prudence: 95 }
O_openness:         { aesthetic_appreciation: 65, inquisitiveness: 60, creativity: 65, unconventionality: 35 }
```

### body

```yaml
physical_description: |
  30대 후반~40대 초반. 단정한 얼굴, 차분한 눈매. 머리에 *금비녀(금채)*.
signature_feature: |
  **금비녀(金釵)** — 맹사조와의 정혼 정표. *정신적 족쇄*. 감정 흔들릴 때 손이 향함.
```

## Layer 2 — 현재 표현

### inner_compass

```yaml
inner_compass:
  compass: "젊은 세대를 *지키되 가두지 않는다* — 내가 살지 못한 삶을 그들이 살게 한다"
  taboo: "죽은 형제(맹사조)의 명예를 더럽히지 않는다 — 정절을 지킨다"
  life_question: "사랑은 표현되어야만 사랑인가? 내가 *살아온 것*이 진짜 인생이었나?"
  taboo_crystallization: "tp_li_mubai_death"
```

### current_state

```yaml
current_state:
  pad: { pleasure: -0.3, arousal: 0.3, dominance: 0.6 }
  dominant_emotion: "Acceptance + Dormant Longing (수용된 미련)"
  active_focus: "옥교룡 단서의 진위 확인 — 행동할지, 보낼지 결정"
```

### relationships

#### key_bonds — 5개 (모두 v0.5 분류 유지, v0.6 검토 결과 기록)

```yaml
key_bonds:

  # ──────────────────────────────────────────────────
  # 1. 이모백 — Soulmate + Deceased + null (v0.6에서도 동일)
  # ──────────────────────────────────────────────────
  - target: "li_mubai"
    type: "영원히 미완의 사랑 — 죽음으로 비로소 받게 된 고백"
    type_history:
      - { since: "맹사조 사망 전",          type: "약혼자의 의형제" }
      - { since: "맹사조 사망 후",          type: "지기 + 잠재 연인" }
      - { since: "qingming_jian_stolen",  type: "함께 싸우는 동지" }
      - { since: "li_mubai_death",        type: "영원히 미완의 사랑" }
    transformation_events:
      - { event_id: "li_mubai_death", new_type: "영원히 미완의 사랑" }
    axes: { trust: 95, affinity: 95, respect: 95, wariness: 5 }
    bond_kind: "Soulmate"
    bond_status: "Deceased"
    partnership: null
    deceased_at: "li_mubai_death"
    bond_since: "맹사조 사망 후 약 5년"
    note: |
      ★ 세 차원 직교의 정확한 표현. Soulmate + Deceased + null = 영혼 일치 + 사망 + 부부 미발현.
      v0.6에서도 *동일 분류*. 시스템이 와호장룡 비극의 본질을 정확히 보존.

  # ──────────────────────────────────────────────────
  # 2. 푸른여우 — ArchRival + Resolved + null (v0.5 동일)
  # ──────────────────────────────────────────────────
  - target: "bi_yan_huli"
    type: "이모백의 사부의 원수 → 결판된 적 (사망)"
    type_history:
      - { since: "이모백 사부 살해 사건",     type: "이모백의 사부의 원수" }
      - { since: "li_mubai_death",        type: "이모백을 죽인 직접 가해자 → 결판된 적" }
    transformation_events:
      - { event_id: "li_mubai_death",  new_type: "결판된 적" }
    axes: { trust: -70, affinity: -90, respect: 70, wariness: 90 }
    bond_kind: "ArchRival"
    bond_status: { Resolved: { reason: "이모백의 복수로 처단" } }
    partnership: null
    bond_since: "이모백 사부 살해 사건"

  # ──────────────────────────────────────────────────
  # 3. 옥교룡 — Mentor + Reactivating + null (v0.5 동일)
  # ──────────────────────────────────────────────────
  - target: "yu_jiaolong"
    type: "가르치려 했으나 따르지 않은 후배 → 변경에 살아있다는 단서"
    type_history:
      - { since: "북경 첫 만남",            type: "표국 손님 (가짜 신분)" }
      - { since: "qingming_jian_stolen",  type: "청명검 도둑·적대" }
      - { since: "수련의 진심 어린 충고",   type: "가르치려 했으나 듣지 않는 후배" }
      - { since: "wudang_mountain_fall",  type: "행방불명" }
      - { since: "current_rumor",         type: "변경에 살아있다는 단서" }
    transformation_events:
      - { event_id: "qingming_jian_stolen", new_type: "청명검 도둑·적대" }
      - { event_id: "shulien_advice",       new_type: "가르치려 했으나 듣지 않는 후배" }
      - { event_id: "wudang_mountain_fall", new_type: "행방불명" }
      - { event_id: "current_rumor",        new_type: "변경에 살아있다는 단서" }
    axes: { trust: 60, affinity: 75, respect: 80, wariness: 50 }
    bond_kind: "Mentor"
    bond_status: { Reactivating: { trigger: "current_rumor" } }
    partnership: null
    bond_since: "shulien_advice 후 14일 유지된 시점"
    note: |
      Mentor variant + Reactivating status 동시 사용 — v0.5에서 검증된 핵심 사례.
      v0.6에서도 동일. Reactivating 단계이므로 ActionTrigger는 *현재 시점*에는 평가 안 됨
      (상대 미접촉). 향후 bian_jing_meeting에서 OfferGuidance → WatchOver 변형이 도출될 예정 —
      이건 노년기 인스턴스 §검증 9.4에서 시연.

  # ──────────────────────────────────────────────────
  # 4. ★ 유태보 — null (v0.6 Companion 후보 검토했으나 임계 미달)
  # ──────────────────────────────────────────────────
  - target: "liu_taibao"
    type: "북경 시정의 의리 있는 친구 — 신분을 가로지른 평민 동지"
    type_history:
      - { since: "와호장룡 시기 (청명검 추적)", type: "정보원 + 동행자" }
      - { since: "이모백 사후",                type: "북경 시정의 의리 있는 친구" }
    transformation_events:
      - { event_id: "qingming_jian_stolen", new_type: "정보원 + 동행자" }
    axes: { trust: 75, affinity: 60, respect: 50, wariness: 30 }
    bond_kind: null   # ★ v0.6 Companion 임계 미달
    bond_status: "Active"
    partnership: null
    bond_since: null
    note: |
      ★ v0.6 Companion variant 임계 검토:
      - Companion 임계: trust ≥+75 ✓, affinity ≥+65 *✗*, respect ≥+50 ✓, wariness ≤30 ✓
      - **affinity 60 < 65 — 미달.** 장년기에는 우정이 충분히 깊지 않음.
      - SwornBrothers 임계: trust ≥+80 *✗*, ... — 더 미달.
      
      → 장년기 시점에는 *어느 양극 variant도* 임계 미달. 자유 텍스트 type만으로 처리.
      
      ★ 노년기 인스턴스에서 axes (80/70/60/20)로 누적되어 Companion 임계 *모두 충족* — 약 10년의
      일상 우정이 자연 누적의 결과. 두 인스턴스 합치 검증의 핵심 사례.

  # ──────────────────────────────────────────────────
  # 5. 맹사조 — null + Deceased + Engaged (v0.5 동일)
  # ──────────────────────────────────────────────────
  - target: "meng_sizhao"
    type: "죽은 약혼자 — 평생 정절의 정표 (금비녀 = 그의 흔적)"
    type_history:
      - { since: "정혼 무렵",     type: "약혼자 (만난 적 적음)" }
      - { since: "정혼 ~ 사망",   type: "약혼자 (단기간)" }
      - { since: "사망 후",       type: "죽은 약혼자 — 평생 정절의 정표" }
    transformation_events:
      - { event_id: "engagement_event",  new_type: "약혼자" }
      - { event_id: "meng_sizhao_death", new_type: "죽은 약혼자" }
    axes: { trust: 80, affinity: 70, respect: 75, wariness: 0 }
    bond_kind: null
    bond_status: "Deceased"
    partnership: "Engaged"
    deceased_at: "meng_sizhao_death"
    bond_since: null
    note: |
      Partnership: Engaged + bond_status: Deceased 조합. 정혼은 깨지지 않음.
      bond_kind null이지만 *현재 정체성에 가장 큰 영향*인 핵심 사례.
```

#### dormant_bonds

```yaml
dormant_bonds:
  - target: "어린 시절 표국에 잠시 머물렀던 무명의 여검객"
    last_contact: "age 10~12"
    fragment: |
      "도(刀)는 사람을 *베는* 것이 아니라 *지키는* 것이다."
    note: "기연 후보. 노년기 first_lesson에서 영향력 활성화됨."
```

### voice — v0.5와 동일

```yaml
voice:
  speech_register: "정중·절제 (강호 어투 + 표국 실용 언어 혼합)"
  vocabulary_level: "사대부와 평민 양쪽 통하는 중간 어휘"
  tics:
    - "'강호 사람은…' 같은 일반화된 가르침 자주"
    - "이모백 직접 호명 회피 — '이 형(李兄)' 또는 '이 검객'"
    - "옥교룡에 대해 *과거형* 사용 — '그 아이는…'"
    - "격렬한 감정에서도 *목소리를 낮춤*"
    - "감정 흔들릴 때 손이 *금비녀로 향함* — 무의식적 동작"
  voice_anchors:
    - context: "옥교룡에게 강호 충고 (와호장룡 시기)"
      utterance: |
        "강호는 자유를 주는 곳이 아니라 *책임과 고통이 따르는 곳*이오.
         그대가 보고 있는 것은 강호가 아니라 강호 *환상*이오."
    - context: "유태보에게 정보 부탁"
      utterance: |
        "유 형, 어렵게 부탁드립니다. 이번 청명검 일은 강호 외부에서 들어온 손이라
         우리 표국의 길로는 답이 안 나옵니다. 그대의 길을 빌리고자 하오."
    - context: "이모백 사망 직후, 절제된 슬픔 (taboo 작동 중)"
      utterance: |
        "(금비녀에 손이 갔다 다시 내려놓으며) 이 검객은 바람처럼 가셨소.
         산 사람은 산 사람의 길을 가야 하니… 청명검은 그가 있어야 할 곳에 보내드리리다."
    - context: "수년 후, 옥교룡 단서를 들음 (현재 snapshot_time)"
      utterance: |
        "변경이라… 그 아이가 거기까지 갔다는 건 살았다는 뜻이오. (잠시 침묵)
         강호는 사람을 잃되 잊지 않는 곳. 가야겠소.
         단 이번엔 *데려오기 위해서*가 아니라 *얼굴 한 번 보기 위해서*."
    - context: "노년기 양녀 양육 시점 (미래)"
      utterance: |
        "춘설아, 도를 익히는 것은 누군가를 베기 위해서가 아니다.
         네가 이 도를 들 때마다, 먼저 *지킬 사람*의 얼굴을 떠올리거라."
```

### titles

```yaml
titles:
  - "쌍도여협(雙刀女俠)"
  - "표국주(運局主)"
```

## Layer 3 — 시간축

### past — transition_points

```yaml
transition_points:

  - id: "tp_first_master_lesson"
    age: "10~12"
    event: "어린 시절 여검객의 가르침 — '도는 지키는 것'"
    impact:
      hexaco_shifts:
        - "H+ Sincerity: 80 → 90"
        - "O+ Aesthetic Appreciation: 50 → 60"
      compass_change: null
    inner_resolution: "여인도 도를 들 수 있다. 단 지키기 위해."
    significance: "compass의 원형. dormant_bonds로 보존."

  - id: "engagement_event"
    age: "20대 초반"
    event: "맹사조와 정혼 — Partnership: null → Engaged 전환"
    impact:
      hexaco_shifts: []
      compass_change: null
    inner_resolution: "이 사람과 평생을 함께한다."
    significance: "Partnership 진입. 후일 정절의 시작점."

  - id: "meng_sizhao_death"
    age: "20대 초반"
    event: "맹사조 사망 — Partnership: Engaged 유지 + bond_status: null → Deceased"
    impact:
      hexaco_shifts:
        - "E+ Sentimentality: 75 → 85"
        - "O- Unconventionality: 40 → 35"
      compass_change: null
    inner_resolution: "그의 명예를 더럽히지 않는다."
    significance: "★ taboo의 *최초 형성*. 후일 이모백과의 진전을 막는 모든 결정의 출처."

  - id: "meet_li_mubai"
    age: "20대 중반"
    event: "이모백과 깊이 만남. 서로의 마음 알면서 *침묵*."
    impact:
      hexaco_shifts: [ "E+ Sentimentality: 85 → 90" ]
      compass_change: null
    inner_resolution: "내 마음은 안다. 그러나 입에 담지 않는다."

  - id: "qingming_jian_stolen"
    age: "30대 중반"
    event: "청명검 도난 — 옥교룡 + 푸른여우 사건 시작"
    impact:
      hexaco_shifts:
        - "X+ Social Boldness: 55 → 60"
        - "A+ Patience: 90 → 95"
      compass_change: null
    inner_resolution: "이 아이는 재능이 있다. 잘못된 길에서 끌어내야 한다."

  - id: "shulien_advice"
    age: "30대 중반"
    event: "수련이 옥교룡에게 강호 본질 충고 — Mentor 진입 14일 카운트 시작"
    impact:
      hexaco_shifts: []
      compass_change: null
    inner_resolution: "이 아이가 듣지 않더라도, 누군가는 말해야 한다."
    significance: "★ Mentor BondKind 진입 트리거."

  - id: "wudang_mountain_fall"
    age: "30대 중반"
    event: "옥교룡 무당산에서 떨어짐 — bond_status: Active → Dormant"
    impact:
      hexaco_shifts: [ "E+ Sentimentality: 90 → 90 (이미 만점)" ]
      compass_change: null
    inner_resolution: "내가 잘못 가르쳤는가…"

  - id: "li_mubai_death"
    age: "30대 후반"
    event: |
      이모백, 푸른여우 독침에 사망. 마지막 *I love you*. 수련은 *받지 않음*.
      직후 푸른여우 처단.
    impact:
      hexaco_shifts:
        - "C+ Prudence: 90 → 95"
        - "A+ Forgiveness: 70 → 75"
      compass_change:
        from: "강호의 의(義)와 책임을 지킨다 — 표국과 약속을 끝까지 지킨다"
        to:   "젊은 세대를 *지키되 가두지 않는다* — 내가 살지 못한 삶을 그들이 살게 한다"
    inner_resolution: |
      "내가 사랑을 받지 않은 것은 약함이 아니라 약속이었다."
    significance: |
      ★★★ 최대 전환점. *동시에 5가지 시스템 슬롯 작동*:
        1. 이모백 BondStatus 전환 (Active → Deceased)
        2. 푸른여우 BondStatus 전환 (Active → Resolved)
        3. compass_change
        4. taboo_crystallization
        5. life_question 발생
      ★ v0.6 §1.5 자연 누적 룰 — 다른 key_bond axes는 *재평가 없이* 자연 누적.

  - id: "current_rumor"
    age: "40대 초반 (snapshot_time)"
    event: "옥교룡 변경 살아있다는 소문 — bond_status: Dormant → Reactivating"
    impact:
      hexaco_shifts: []
      compass_change: null
    inner_resolution: "확인하러 가야겠다."
```

### past — formative_relationships

```yaml
formative_relationships:
  - id: "father"
    type: "표국 운영자, 부친"
    legacy: "쌍도술 사사. 표국 운영의 모든 기초."

  - id: "first_master_unnamed"
    type: "어린 시절 무명 여검객"
    legacy: "compass의 직접 출처. dormant_bond에도 등록."
```

### present — unresolved_tension

```yaml
unresolved_tension:
  - id: "ut_1_unspoken_love"
    category: "내부적·죄책감"
    description: "이모백의 마지막 사랑을 *받지 않은* 자신에 대한 평생 자문."

  - id: "ut_2_yu_jiaolong_fate"
    category: "관계적·책임감"
    description: |
      옥교룡 행방 미확인. 살아있다는 단서. 가야 하는가?
      ★ bond_status: Reactivating의 직접 표현.

  - id: "ut_3_qingming_jian"
    category: "외부적·상징적"
    description: "청명검 행방. 이모백의 분신을 *어떤 형태로* 보존할 것인가."
```

### future hooks

```yaml
joyful_seed:
  description: |
    옥교룡-나소호의 딸 춘설병을 양녀로. *모성애 승화*.
    이모백·맹사조의 미완을 다음 세대에서 완성.
  trigger_condition: |
    `ut_2_yu_jiaolong_fate` 추적 결과 옥교룡은 사망 또는 만남 거부, 그러나 자녀 발견.
    `dormant_bonds[0]` 활성화.

tragic_seed:
  description: |
    옥교룡 단서가 거짓이거나, 만나도 더 이상 가르침을 받을 자가 아닐 가능성.
  trigger_condition: |
    옥교룡 bond_status: Reactivating → Resolved 또는 Deceased 전환.
```

---

# v0.6 검증 결과

## 1. 5개 key_bonds 분류 (v0.6 기준)

| 인물 | bond_kind | bond_status | partnership | v0.5 → v0.6 |
|---|---|---|---|---|
| 이모백 | Soulmate | Deceased | null | 변화 없음 |
| 푸른여우 | ArchRival | Resolved | null | 변화 없음 |
| 옥교룡 | Mentor | Reactivating | null | 변화 없음 |
| **유태보** | **null** (Companion 임계 미달) | Active | null | v0.6 검토에서 affinity 미달 명시 |
| 맹사조 | null | Deceased | Engaged | 변화 없음 |

## 2. 노년기 인스턴스와의 합치 — 시간 누적의 검증

핵심 비교 (장년기 → 노년기, 약 10~12년 경과):

| 인물 | 장년기 axes | 노년기 axes | bond_kind 변화 |
|---|---|---|---|
| 이모백 | 95/95/95/5 | **동일** (Deceased freeze) | 동일 |
| 푸른여우 | -70/-90/70/90 | **동일** (Resolved freeze) | 동일 |
| 옥교룡 | 60/75/80/50 | 65/80/80/35 (사후 갱신 후 freeze) | Reactivating → Deceased |
| **유태보** | **75/60/50/30** (Companion 임계 미달) | **80/70/60/20** (Companion 임계 충족) | **null → Companion** ★ |
| 맹사조 | 80/70/75/0 | **동일** (Deceased freeze) | 동일 |

★ **유태보의 변화가 가장 흥미.** 약 10년의 일상 우정이 axes를 75/60/50/30 → 80/70/60/20으로 누적시켜 Companion 임계 자연 도달. 이게 v0.6의 *시간 누적과 양극 진입* 룰의 가장 명확한 시연.

## 3. v0.6에서도 이 인스턴스의 한계는 *없음*

장년기 수련 인스턴스는 v0.5에서 이미 *세 차원 직교*의 핵심 검증을 모두 시연. v0.6 신설 항목 중 직접 적용되는 것이 *없음* — 이미 분류가 완전.

이게 시스템 안정성의 신호. v0.5 → v0.6 변경이 *기존 인스턴스를 깨지 않음*. 후방 호환 ✓.

## 4. 핵심 발견 — 인스턴스 *일생* 추적의 가치

장년기 + 노년기 두 인스턴스 합치가 보여주는 것:
- **변하지 않는 것**: id, compass, taboo, life_question, *Deceased/Resolved 관계의 axes*
- **변하는 것**: PAD, HEXACO 미세, *Active 관계의 axes*, bond_kind 진입
- **점진적 진입**: 유태보 사례 — *일상의 시간*이 양극 진입을 만듦

v0.6 시스템이 *시간을 정확히 표현*. Pillar 4 ("시간이 의미를 만든다")의 시스템적 구현.

---

## 변경 이력

| 버전 | 일자 | 변경 |
|------|------|------|
| v1.0 (v0.4) | 2026-05-04 | 초안 |
| v2.0 (v0.5) | 2026-05-04 | 세 차원 직교화 적용 |
| v3.0 (v0.6) | 2026-05-04 | v0.6 11 variants 검토 결과: 5개 분류 *모두 동일* 유지. 유태보 Companion 임계 미달 (affinity 60 < 65) 명시 — 노년기 인스턴스에서 자연 도달의 *대조 시작점*. v0.6 후방 호환 ✓. |
