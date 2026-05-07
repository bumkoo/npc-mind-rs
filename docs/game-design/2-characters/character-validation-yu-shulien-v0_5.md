# 인물 스키마 v0.5 검증 — 유수련 (兪秀蓮)

> 작성일: 2026-05-04
> 검증 대상: `_schema.md` v0.5 + `relationships.md` v0.5
> 위치: `docs/game-design/2-characters/character-validation-yu-shulien-v0_5.md`
> 이전 버전: v0.4 (`character-validation-yu-shulien-v0_4.md` 폐기됨)
> 동반 검증 인스턴스: 임충 (`character-validation-lin-chong-v0_5.md`)

## v0.5 적용 요약

수련 인스턴스가 v0.5의 *모든 신설 슬롯*을 직접 사용:

| v0.5 신설 | 수련 인스턴스에서 |
|---|---|
| **bond_status: Active** | 옥교룡 시점 전·유태보 |
| **bond_status: Deceased** | 이모백·맹사조 |
| **bond_status: Resolved** | 푸른여우 (결판 도달) |
| **bond_status: Reactivating** | 옥교룡 (현재 — 단서 막 들어옴) |
| **bond_status: Dormant** | (해당 없음. 향후 양녀 양육 전 옥교룡이 거기로 갈 수도) |
| **partnership: Spouse** | (해당 없음. 수련은 결혼 안 함) |
| **partnership: Engaged** | 맹사조 — 정혼 중 사망 |
| **partnership: null + Soulmate** | 이모백 — 영혼 일치하나 부부 미발현 |
| **BondKind: Mentor** | 옥교룡 — v0.5 신설 variant 첫 적용 |

5개 status 중 4개 사용 (Dormant 미사용), 4개 partnership 중 1개 사용 (Engaged), Mentor variant 1회 사용.
임충 인스턴스와 합치면 status 5종 모두 + partnership 2종(Engaged/Separated) 검증 완료.

---

# 유수련(兪秀蓮)

## Layer 1 — 본바탕

### identity

```yaml
id: "yu_shulien"
name: "유수련(兪秀蓮)"
nicknames:
  - "쌍도여협(雙刀女俠)"
  - "표국의 여주인"
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
  거친 표국 생활을 하며 강호 문법을 일찍 익힘. 동시에 사대부와도 거래.
  어린 시절 부친에게 쌍도(雙刀)를 배움.
```

### temperament — HEXACO 24 facet

```yaml
H_honesty_humility:
  sincerity: 90      # ★
  fairness: 90
  greed_avoidance: 85
  modesty: 85
E_emotionality:
  fearfulness: 25
  anxiety: 50
  dependence: 35
  sentimentality: 90  # ★★ 평생 미련의 원천
X_extraversion:
  social_self_esteem: 70
  social_boldness: 60
  sociability: 60
  liveliness: 40
A_agreeableness:
  forgiveness: 75
  gentleness: 75
  flexibility: 70
  patience: 95       # ★
C_conscientiousness:
  organization: 90
  diligence: 90
  perfectionism: 80
  prudence: 95       # ★★
O_openness:
  aesthetic_appreciation: 65
  inquisitiveness: 60
  creativity: 65
  unconventionality: 35  # ★ 보수적 — 비극의 핵심
```

### body

```yaml
physical_description: |
  30대 후반~40대 초반. 단정한 얼굴, 차분한 눈매. 머리에 *금비녀(금채)*를 항상 꽂음.
signature_feature: |
  **금비녀(金釵)** — 맹사조와의 정혼 정표. 단순 장신구가 아닌 *정신적 족쇄*.
  감정에 흔들릴 때마다 손이 자기도 모르게 금비녀로 향함 — LLM 연기의 핵심 *동작 anchor*.
  쌍도는 무공 시 양손 한 자루씩, 평소엔 옷 속 감춤.
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

#### key_bonds — v0.5 적용 (5개 모두)

```yaml
key_bonds:

  # ──────────────────────────────────────────────────
  # 1. 이모백 — Soulmate + Deceased (★★ partnership 미발현 핵심)
  # ──────────────────────────────────────────────────
  - target: "li_mubai"
    type: "영원히 미완의 사랑 — 죽음으로 비로소 받게 된 고백"
    type_history:
      - { since: "맹사조 사망 전",          type: "약혼자의 의형제" }
      - { since: "맹사조 사망 후",          type: "지기 + 잠재 연인 (서로 알면서 침묵)" }
      - { since: "qingming_jian_stolen",  type: "함께 싸우는 동지" }
      - { since: "li_mubai_death",        type: "영원히 미완의 사랑" }
    transformation_events:
      - { event_id: "qingming_jian_stolen", new_type: "함께 싸우는 동지" }
      - { event_id: "li_mubai_death",       new_type: "영원히 미완의 사랑" }
    axes: { trust: 95, affinity: 95, respect: 95, wariness: 5 }
    bond_kind: "Soulmate"
    bond_status: "Deceased"        # ★★★ v0.5 핵심
    partnership: null              # ★★★ v0.5 직교성 — Soulmate인데 부부 미발현
    deceased_at: "li_mubai_death"  # ★ v0.5 신설 슬롯
    bond_since: "맹사조 사망 후 약 5년"
    note: |
      ★★★ v0.5의 가장 강한 검증 사례. 세 차원 *모두* 의미 있게 사용:
      - BondKind: Soulmate (영혼의 일치)
      - Status: Deceased (사망 후 axes freeze, 회상 OCC만 가능)
      - Partnership: null (부부 미발현 — 비극의 정확한 시스템적 표현)

      v0.4에서 "Soulmate인데 부부 발현 안 됨"이 한계였으나, v0.5에서 Partnership을 *별도 슬롯*으로
      두자 자연스러운 표현. Soulmate + null 조합이 와호장룡 비극의 본질 — *영혼은 일치하나 형식은
      발현되지 않은*. v0.4의 "enum이 부부형 매핑 강제"가 사라짐.

      ★ axes 95/95/95/5는 *3~5년간 유지*. Status: Deceased로 freeze. v0.4 §1.4 점착성 룰의 작동 +
      v0.5의 명시적 freeze 룰의 결합. 새 OCC 입력은 차단되되, 회상 OCC는 PAD에 영향 가능 (§4.5).

  # ──────────────────────────────────────────────────
  # 2. 푸른여우 — ArchRival + Resolved (★ 결판 도달 후처리)
  # ──────────────────────────────────────────────────
  - target: "bi_yan_huli"
    type: "이모백의 사부의 원수 → 결판된 적 (사망)"
    type_history:
      - { since: "이모백 사부 살해 사건",     type: "이모백의 사부의 원수" }
      - { since: "qingming_jian_stolen",  type: "청명검 도난의 배후" }
      - { since: "li_mubai_death",        type: "이모백을 죽인 직접 가해자 → 결판된 적" }
    transformation_events:
      - { event_id: "li_mubai_death",  new_type: "이모백을 죽인 직접 가해자 → 결판된 적" }
    axes: { trust: -70, affinity: -90, respect: 70, wariness: 90 }
    bond_kind: "ArchRival"
    bond_status: { Resolved: { reason: "이모백의 복수로 처단" } }   # ★ v0.5 Resolved 첫 사례
    partnership: null
    deceased_at: null              # ★ Resolved지만 deceased_at도 채울 수 있음 (옵션)
    bond_since: "이모백 사부 살해 사건"
    note: |
      ★★ v0.5 Resolved status 검증.
      - BondKind: ArchRival 그대로 유지 (수련 정체성 형성에 영구 영향)
      - Status: Resolved {결판 도달} → 행동 트리거 *불활성*. 새 결투 신청 emit 안 함.
      - axes freeze. 회상 OCC만 가능.

      v0.4에서 "결판 후 처리 부재"가 한계였으나, v0.5에서 Resolved status로 명확히 해결.
      reason 필드로 *어떤 결판인가*까지 시스템에 기록 — 추후 회상 OCC가 어떤 색채인가 결정 가능
      ("처단해 통쾌" vs "이미 끝났으나 슬픔").

      ★ 푸른여우는 *사망*이지만 status는 Resolved (Deceased 아님). 이유: 수련 시점에서 푸른여우의
      본질은 *적이고, 결판났음*. 사망은 결판의 *수단*. Status는 *관계의 의미*를 표현, 사망 사실 자체는
      type에 자유 텍스트로 보존. 이게 두 status의 의미 구분 — Deceased는 *상실의 슬픔* 차원,
      Resolved는 *결판의 종료* 차원.

  # ──────────────────────────────────────────────────
  # 3. 옥교룡 — Mentor + Reactivating (★★★ v0.5 신설 variant + status 동시)
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
    bond_kind: "Mentor"           # ★★★ v0.5 신설 variant 첫 적용
    bond_status: { Reactivating: { trigger: "current_rumor" } }  # ★★★ v0.5 신설 status
    partnership: null
    bond_since: "shulien_advice 후 14일 유지된 시점"   # ★ Mentor 진입 14일 게이트
    note: |
      ★★★ v0.5의 *두 가지 신설*을 동시 사용. 이 인스턴스의 시스템 검증 핵심.

      **Mentor variant 검증:**
      - 임계: trust ≥+50 (60 ✓), affinity ≥+50 (75 ✓), respect ≥+60 (80 ✓), wariness ≤60 (50 ✓).
      - 추가 조건: type_history에 "가르치려 함" 의미 존재 — `shulien_advice` 사건 ✓.
      - 진입 14일 게이트: 청명검 추적 + 충고 기간이 14일 이상 유지 ✓.
      v0.4에서 분류 불가했던 관계가 v0.5에서 *비로소* Mentor로 정확 분류.

      **Reactivating status 검증:**
      - wudang_mountain_fall 후 옥교룡 행방불명 → status: Dormant (또는 Active이되 휴면)
      - current_rumor 사건 → status: Reactivating { trigger: current_rumor }
      - axes 부분 unfreeze. 새 OCC 입력 받기 시작 (소문 듣고 발생한 Hope/Anxiety 등).
      - 단서 확인 시점에서 Active 또는 Dormant 복귀 결정.
      v0.4의 "dormant_bonds vs Dormant 정의 모호"가 v0.5에서 status로 명확 해소.

      ★ wariness=50이 의미 있음. *멘티가 또 어긋날 위험을 이미 인식*. Mentor 임계가 ≤60으로
      높은 이유 — MasterDisciple(≤40)보다 *경계가 자연스러움*. 옥교룡이 정확히 Mentor 임계
      범위에 들어옴.

  # ──────────────────────────────────────────────────
  # 4. 유태보 — null + Active (자유 텍스트 type만으로 충분)
  # ──────────────────────────────────────────────────
  - target: "liu_taibao"
    type: "북경 시정의 의리 있는 친구 — 신분을 가로지른 평민 동지"
    type_history:
      - { since: "와호장룡 시기 (청명검 추적)", type: "정보원 + 동행자" }
      - { since: "이모백 사후",                type: "북경 시정의 의리 있는 친구" }
    transformation_events:
      - { event_id: "qingming_jian_stolen", new_type: "정보원 + 동행자" }
    axes: { trust: 75, affinity: 60, respect: 50, wariness: 30 }
    bond_kind: null                # SwornBrothers 임계 trust ≥80 미달, Mentor도 미달
    bond_status: "Active"
    partnership: null
    bond_since: null
    note: |
      ★ 자유 텍스트 type만으로 충분한 관계의 가치 검증. enum 강제가 *결함*이 아닌 *정확*.
      신분 차이를 가로지르는 *평민 동지*의 자연스러운 표현. v0.5에서도 이런 관계는 enum 없이
      type + axes + status로 충분.

  # ──────────────────────────────────────────────────
  # 5. 맹사조 — null + Deceased + Engaged (★★★ partnership 핵심 검증)
  # ──────────────────────────────────────────────────
  - target: "meng_sizhao"
    type: "죽은 약혼자 — 평생 정절의 정표 (금비녀 = 그의 흔적)"
    type_history:
      - { since: "정혼 무렵",     type: "약혼자 (만난 적 적음)" }
      - { since: "정혼 ~ 사망",   type: "약혼자 (단기간)" }
      - { since: "사망 후",       type: "죽은 약혼자 — 평생 정절의 정표" }
    transformation_events:
      - { event_id: "engagement_event",  new_type: "약혼자" }                 # ★ Partnership 진입
      - { event_id: "meng_sizhao_death", new_type: "죽은 약혼자" }
    axes: { trust: 80, affinity: 70, respect: 75, wariness: 0 }
    bond_kind: null                # 만남 짧아 BondKind 임계 미달
    bond_status: "Deceased"        # ★ v0.5
    partnership: "Engaged"         # ★★★ v0.5 — 정혼 상태로 사망
    deceased_at: "meng_sizhao_death"
    bond_since: null
    note: |
      ★★★ v0.5의 가장 큰 한계 해소 사례. v0.4에서 "key_bonds vs formative 어디 둘까?"의 한계가
      가장 강하게 노출된 인물.

      **v0.5 해결 방식:**
      - axes 깊지 않음 (만남 짧아 BondKind 임계 미달) → bond_kind: null.
      - 그러나 *현재 정체성에 가장 큰 영향* (taboo의 출처) → key_bonds 위치가 정합.
      - Status: Deceased로 *현재 상호작용 불가*만 명시.
      - **Partnership: Engaged**가 *수련 정체성의 핵심*을 보존.
        정혼자였다는 사실 = 평생 정절 taboo의 근거. 시스템이 이를 *형식 차원*에서 명확히 표시.

      v0.4에서는 "axes 보통이고 enum 없으니 key_bonds에 둘 의미가 모호"했으나, v0.5에서는
      Partnership: Engaged가 *현재 영향력의 출처*를 명확히 표시. 시스템이 자기 설명적이 됨.

      ★ formative_relationships에는 *중복 등록 안 함* (v0.4에서는 중복했음).
      v0.5 명확화 룰: "현재 정체성·행동에 강한 영향이면 key_bonds[Deceased]에만". formative는
      *과거 의미만 남은* 인물 전용. 맹사조는 현재 영향이 결정적이므로 key_bonds 단독.
```

#### dormant_bonds

```yaml
dormant_bonds:
  - target: "어린 시절 표국에 잠시 머물렀던 무명의 여검객"
    last_contact: "age 10~12"
    fragment: |
      이름도 얼굴도 흐릿하나, 한 마디만 또렷이 — "도(刀)는 사람을 *베는* 것이 아니라 *지키는* 것이다."
      어린 수련에게 처음으로 *여인이 무를 익혀도 된다*는 가능성을 보여준 자.
    note: |
      *한 번도 활성화된 적 없는* 잠재 관계 — dormant_bonds 정의에 부합 (v0.5 명확화).
      옥교룡의 Reactivating 케이스와 구별: 옥교룡은 *예전 활성*이었으므로 key_bonds에.
      이 무명 여검객은 *진정한 잠재* — 활성화 트리거 시 새 key_bond 생성.
```

### voice

```yaml
voice:
  speech_register: "정중·절제 (강호 어투 + 표국 실용 언어 혼합)"
  vocabulary_level: "사대부와 평민 양쪽 통하는 중간 어휘"
  tics:
    - "'강호 사람은…' 같은 일반화된 가르침 자주"
    - "이모백 직접 호명 회피 — '이 형(李兄)' 또는 '이 검객'"
    - "옥교룡에 대해 *과거형* 사용 — '그 아이는…'"
    - "격렬한 감정에서도 *목소리를 낮춤* (절제의 신체화)"
    - "감정 흔들릴 때 손이 *금비녀로 향함* — 무의식적 동작"
  voice_anchors:
    - context: "옥교룡에게 강호 충고 (와호장룡 시기)"
      utterance: |
        "강호는 자유를 주는 곳이 아니라 *책임과 고통이 따르는 곳*이오.
         그대가 보고 있는 것은 강호가 아니라 강호 *환상*이오."
    - context: "유태보(시정 잡배)에게 정보 부탁"
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
  - "(가까운 미래: 춘설병의 양모)"
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
    inner_resolution: "여인도 도를 들 수 있다. 단 지키기 위해."
    significance: "compass의 원형. dormant_bonds로 보존."

  - id: "engagement_event"
    age: "20대 초반"
    event: "맹사조와 정혼 — Partnership: null → Engaged 전환"
    impact:
      hexaco_shifts: []      # 정혼 자체는 큰 HEXACO 변화 없음
    inner_resolution: "이 사람과 평생을 함께한다."
    significance: "Partnership: Engaged 진입. 후일 평생 정절의 시작점."

  - id: "meng_sizhao_death"
    age: "20대 초반"
    event: "정혼자 맹사조 사망 — Partnership: Engaged 유지 + bond_status: null → Deceased 전환"
    impact:
      hexaco_shifts:
        - "E+ Sentimentality: 75 → 85"
        - "O- Unconventionality: 40 → 35 (보수화)"
    inner_resolution: "그의 명예를 더럽히지 않는다. 나는 그의 정혼자였다."
    significance: |
      ★ taboo의 *최초 형성*. 후일 이모백과의 진전을 막는 모든 결정의 출처.
      Partnership: Engaged 유지 — 정혼은 *깨지지 않음*. 사회적·정신적으로 영원한 약혼자.

  - id: "meet_li_mubai"
    age: "20대 중반"
    event: "맹사조의 의형제 이모백과 깊이 만남. 서로 마음 알면서도 *침묵*."
    impact:
      hexaco_shifts:
        - "E+ Sentimentality: 85 → 90"
      compass_change: null   # compass 변화 없음 — taboo가 우세
    inner_resolution: "내 마음은 안다. 그러나 입에 담지 않는다."
    significance: "Soulmate axes의 *지속 누적* 시작점."

  - id: "qingming_jian_stolen"
    age: "30대 중반"
    event: "청명검 도난 — 옥교룡 + 푸른여우 사건 시작"
    impact:
      hexaco_shifts:
        - "X+ Social Boldness: 55 → 60"
        - "A+ Patience: 90 → 95"
    inner_resolution: "이 아이는 재능이 있다. 잘못된 길에서 끌어내야 한다."
    significance: "옥교룡 type 변화 시작. 푸른여우 ArchRival axes 음극 심화."

  - id: "shulien_advice"
    age: "30대 중반"
    event: "수련이 옥교룡에게 강호 본질을 직접 충고 — Mentor 진입 14일 카운트 시작"
    impact:
      hexaco_shifts: []
    inner_resolution: "이 아이가 듣지 않더라도, 누군가는 말해야 한다."
    significance: "★ Mentor BondKind 진입 트리거. 14일 후 Mentor 활성화."

  - id: "wudang_mountain_fall"
    age: "30대 중반"
    event: "옥교룡이 무당산에서 떨어짐. bond_status: Active → Dormant 전환"
    impact:
      hexaco_shifts:
        - "E+ Sentimentality: 90 → 90 (이미 만점)"
    inner_resolution: "내가 잘못 가르쳤는가… 잘못 보았는가…"
    significance: "옥교룡 status Dormant 진입. 자책의 시작."

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
      "내가 사랑을 받지 않은 것은 약함이 아니라 약속이었다. 다음 세대에게는 다른 길을 보이리라."
    significance: |
      ★★★ 최대 전환점. *동시에 5가지 시스템 슬롯 작동* (v0.5 확장):
        1. 이모백 BondStatus 전환 (Active → Deceased)
        2. 푸른여우 BondStatus 전환 (Active → Resolved)
        3. compass_change
        4. taboo_crystallization
        5. life_question 발생
      v0.5 시스템이 *복합 비극의 다중 슬롯 영향*을 정확히 표현 가능함을 입증.

  - id: "current_rumor"
    age: "40대 초반 (snapshot_time)"
    event: "옥교룡이 변경에 살아있다는 소문 — bond_status: Dormant → Reactivating 전환"
    impact:
      hexaco_shifts: []
    inner_resolution: "확인하러 가야겠다."
    significance: "★ v0.5 Reactivating status 진입. 본격 활성화는 미래 시점."
```

### past — formative_relationships

```yaml
formative_relationships:
  - id: "father"
    type: "표국 운영자, 부친"
    legacy: |
      쌍도술 사사. 표국 운영의 모든 기초. compass와 무업 정체성의 출처.

  - id: "li_mubai_past"
    type: "지기 + 잠재 연인 (생전)"
    legacy: |
      v0.5 명확화: 이모백은 key_bonds[Deceased]에 *현재 정체성 영향*으로 등록되어 있음.
      formative에는 *추가 등록하지 않음* (중복 회피). 단, 그의 *생전 시기*는 인생에 결정적.
      이건 별도 항목으로 남기는 의미가 있어 short legacy로 보존.
```

> **v0.5 변경**: 이전 v0.4에서 이모백·맹사조를 formative와 key_bonds 양쪽 *중복* 등록했으나, v0.5에서는 *현재 영향 강하면 key_bonds[Deceased] 단독*. formative는 *짧은 legacy 메모*만 남기거나 *완전 제거*. 일관성 ↑.

### present — unresolved_tension

```yaml
unresolved_tension:
  - id: "ut_1_unspoken_love"
    category: "내부적·죄책감"
    description: |
      이모백의 마지막 사랑을 *받지 않은* 자신에 대한 평생 자문. life_question에 직결.

  - id: "ut_2_yu_jiaolong_fate"
    category: "관계적·책임감"
    description: |
      옥교룡 행방 미확인. 살아있다는 단서. 가야 하는가? 갈 자격이 있는가?
      ★ bond_status: Reactivating의 직접 표현.

  - id: "ut_3_qingming_jian"
    category: "외부적·상징적"
    description: |
      청명검 행방. 이모백의 분신을 *어떤 형태로* 보존할 것인가.
```

### future hooks

```yaml
joyful_seed:
  description: |
    옥교룡-나소호의 딸 춘설병을 만나 양녀로 삼음. *모성애로 승화*.
    이모백·맹사조의 미완을 다음 세대에서 완성. 노년기 *방하착*.
  trigger_condition: |
    `ut_2_yu_jiaolong_fate` 추적 결과 옥교룡은 사망 또는 만남 거부, 그러나 자녀 발견.
    `dormant_bonds[0]` 활성화 (어린 시절 여검객의 가르침이 양녀 양육 첫날 떠오름).

tragic_seed:
  description: |
    옥교룡 단서가 거짓이거나, 만나도 더 이상 가르침을 받을 자가 아닐 가능성.
    또는 푸른여우의 *제자·동조자*가 새 적으로 등장 — 새 ArchRival 진입.
  trigger_condition: |
    `current_rumor`가 거짓 정보로 판명되거나, 변경에서 만난 자가 옥교룡 사칭자.
    옥교룡 bond_status: Reactivating → Resolved { reason: "사망 확정" } 전환.
```

---

# v0.5 적용 검증 결과

## 세 차원 활용 분포 (5개 key_bonds)

| 인물 | bond_kind | bond_status | partnership |
|---|---|---|---|
| 이모백 | **Soulmate** | **Deceased** | null |
| 푸른여우 | **ArchRival** | **Resolved** | null |
| 옥교룡 | **Mentor** ★ | **Reactivating** ★ | null |
| 유태보 | null | Active | null |
| 맹사조 | null | **Deceased** | **Engaged** ★ |

★ = v0.5 신설 슬롯의 *고유 사용 사례*.

## 임충 + 수련 합치 — v0.5 슬롯 커버리지

| v0.5 슬롯 | 검증 | 어느 인스턴스 |
|---|---|---|
| BondKind: SwornBrothers | (임계 근접) | 임충-노지심 |
| BondKind: MasterDisciple | 미검증 | (다음 인물 후보) |
| BondKind: Soulmate | ✓ | 수련-이모백 |
| BondKind: LoyalRetainer | (연청 별도) | 검증 사례 §5.1 |
| BondKind: BloodEnemy | ✓ | 임충-고아내 |
| BondKind: ArchRival | ✓ | 수련-푸른여우 |
| BondKind: Betrayer | ✓ | 임충-육겸 |
| BondKind: Oppressor | ✓ | 임충-고구 |
| **BondKind: Mentor** | ✓ | 수련-옥교룡 |
| BondStatus: Active | ✓ | 다수 |
| **BondStatus: Resolved** | ✓ | 임충-육겸, 수련-푸른여우 |
| **BondStatus: Deceased** | ✓ | 수련-이모백, 수련-맹사조 |
| **BondStatus: Dormant** | ✓ | 옥교룡 (wudang fall ~ rumor 사이) |
| **BondStatus: Reactivating** | ✓ | 수련-옥교룡 (현재) |
| Partnership: Spouse | 미검증 | (다음 인물 후보) |
| **Partnership: Engaged** | ✓ | 수련-맹사조 |
| Partnership: Lover | 미검증 | (다음 인물 후보) |
| **Partnership: Separated** | ✓ | 임충-장씨 |

**커버리지 요약:**
- BondKind 9종 중 8종 검증 (MasterDisciple만 미검증, 차기 인스턴스로)
- BondStatus 5종 모두 검증 ✓
- Partnership 4종 중 2종 검증 (Spouse·Lover 미검증)

두 인스턴스만으로 v0.5 시스템의 *대부분* 검증 완료.

## v0.5 시스템 한계 — 향후 검증 필요

### 한계 1: Action Trigger Evaluator 미설계
임충-고아내 BloodEnemy 임계 충족 + 처단 *보류* 케이스. 시스템은 *분류*까지, *실행 가능성*은 별도 평가 필요. v0.6 후보.

### 한계 2: 회상 OCC 메커니즘 골격만 있음
§4.5에 함수 시그니처만. 어떤 상황이 회상 트리거인지, 강도 계산은 어떻게, 추모 행동 트리거 조건은 무엇인지 미설계. v0.6 후보.

### 한계 3: compass 변화 후 axes 재평가
큰 compass 변화(임충 산신묘, 수련 li_mubai_death) 시 *모든 key_bond 재평가*가 필요한가? 현재 v0.5는 자연 누적. 다음 인스턴스에서 모순 발생하면 명시 룰 필요.

### 한계 4: BondKind 비대칭 — Mentee variant?
수련 → 옥교룡 = Mentor. 옥교룡 → 수련 = ? 옥교룡은 가르침을 *거부*하지만, 그래도 수련은 그녀의 인생에 영향을 미친 자. *Mentee*나 *Influenced* variant가 필요할 수도. 옥교룡 인스턴스 작성 시 결정 권장.

### 한계 5: noyaer/시간 차등 게이트
v0.5는 SwornBrothers 30일 / Mentor 14일. MasterDisciple·Soulmate·LoyalRetainer는 모두 30일. 종류별 시간 차등이 더 자연스러울 수 있으나 (예: Soulmate 90일?), 현재는 균일 30일 + Mentor 14일 예외. 추가 검증 필요.

## 결론

v0.5 시스템은 임충·수련 두 비극적 인스턴스를 *왜곡 없이* 표현. 세 차원 직교화가 v0.4의 모든 핵심 한계 해소:

1. **Romantic bond**: Partnership 슬롯 직교화로 해결 — 임충-장씨, 수련-이모백 양쪽 패턴 표현 가능.
2. **Deceased**: BondStatus enum으로 해결 — 이모백·맹사조 정확 분류.
3. **Resolved (결판)**: BondStatus enum으로 해결 — 육겸 처단, 푸른여우 결판 분류.
4. **Reactivating**: BondStatus enum으로 해결 — 옥교룡 현재 상태 정확 표현.
5. **Mentor**: BondKind에 추가 — 수련-옥교룡 분류 가능.

가장 강한 검증: **수련 → 이모백** = `Soulmate + Deceased + null`. 한 줄로 *영혼의 동반자 + 사망 + 부부 미발현*이라는 와호장룡 비극의 본질이 시스템에 정확히 보존됨. v0.4에서는 이 표현이 불가능했음.

---

## 변경 이력

| 버전 | 일자 | 변경 |
|------|------|------|
| v1.0 (v0.4 스키마) | 2026-05-04 | 초안. v0.4 검증. deceased / Resolved / Reactivating / Mentor 한계 노출. |
| v2.0 (v0.5 스키마) | 2026-05-04 | **v0.5 적용**. 5개 key_bonds 모두에 bond_status 명시. 이모백 = Soulmate + Deceased + null. 푸른여우 = ArchRival + Resolved. 옥교룡 = Mentor + Reactivating (★ 신설 variant·status 동시 사용). 맹사조 = null + Deceased + Engaged. formative 중복 등록 제거 (v0.4 잔여 문제 해결). 임충+수련 합치 검증 결과 — v0.5 슬롯 대부분 커버. |
