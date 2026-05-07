# 인물 스키마 v0.4 검증 — 유수련 (兪秀蓮)

> 작성일: 2026-05-04
> 검증 대상: `_schema.md` v0.4 + `relationships.md` v0.4
> 위치: `docs/game-design/2-characters/character-validation-yu-shulien-v0_4.md`
> 추후 정식 인물 디렉토리(`characters/yu_shulien.md`)로 이동 가능.
> 동반 검증 인스턴스: 임충 (`character-validation-v0_4.md`)

## 검증 목적

임충 인스턴스가 검증한 것:
- 음극 BondKind (Betrayer/Oppressor/BloodEnemy)
- 양극 진입 30일 게이트 미도달
- *발생 직후* 시점

수련 인스턴스가 검증할 것 (대조 그룹):
- **양극 BondKind 임계 *완전* 충족** (Soulmate)
- **결판 도달한 ArchRival**의 후처리
- ***시간이 지난 후*** 의 점착성 룰 (±100 도달 후 머무름)
- **사망한 인물들**의 처리 (이모백·맹사조·푸른여우 모두 deceased)
- **dormant 재활성화** 단서 (옥교룡)
- **인생 멘토** 관계 (옥교룡-수련, 무술 사부 아닌 선배-후배)

snapshot_time이 "사건 발생 직후"가 아닌 "*몇 년 후*"인 점이 핵심 차이.

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
kingdom_of_origin: "(청, 옹정~건륭 시기로 추정)"
family_background: |
  명문가는 아니나 양민 중 *기예 있는 집안*. 부친이 북경에서 표국(運局)을 운영.
  거친 표국 생활을 하며 강호 문법을 일찍 익힘. 동시에 사대부와도 거래하며
  사회 양극의 언어를 모두 통함. 어린 시절 부친에게 쌍도(雙刀)를 배움.
```

### temperament — HEXACO 24 facet

```yaml
H_honesty_humility:
  sincerity: 90        # ★ 정직의 화신
  fairness: 90
  greed_avoidance: 85
  modesty: 85
E_emotionality:
  fearfulness: 25       # 무인, 두려움 적음
  anxiety: 50
  dependence: 35       # 자립적
  sentimentality: 90    # ★★ 깊은 감정. 이모백·맹사조에 대한 평생 미련의 원천.
X_extraversion:
  social_self_esteem: 70
  social_boldness: 60
  sociability: 60       # 표국 ↔ 시정 ↔ 사대부 양쪽 통함
  liveliness: 40        # 침착함이 우세
A_agreeableness:
  forgiveness: 75       # 옥교룡 같은 어린 자에게 관용
  gentleness: 75
  flexibility: 70       # 신분 차이 가로지름
  patience: 95          # ★ 인내의 화신
C_conscientiousness:
  organization: 90
  diligence: 90
  perfectionism: 80
  prudence: 95          # ★★
O_openness:
  aesthetic_appreciation: 65
  inquisitiveness: 60
  creativity: 65
  unconventionality: 35 # ★ 매우 보수적 — 봉건 윤리 고수가 비극의 핵심
```

> ★ 임충(C+ Prudence 90)과 수련(C+ Prudence 95)의 *비슷한 구조*: 둘 다 인내·신중의 인물. 그러나 *발현 양상*이 다름. 임충은 인내 → *폭발*. 수련은 인내 → *지속적 절제*. HEXACO만으론 둘을 구별할 수 없음 — `inner_compass`와 `transition_points`가 두 인물을 *다르게* 만든다.

### body

```yaml
physical_description: |
  30대 후반~40대 초반. 단정한 얼굴, 차분한 눈매. 머리에 *금비녀(금채)*를 항상 꽂음.
  표국 일로 단련된 몸이지만 사대부 부인 못지않은 단정한 옷차림. 쌍도는 평소엔 보이지 않게 휴대.
signature_feature: |
  **금비녀(金釵)** — 맹사조와의 정혼 정표. 단순 장신구가 아닌 *정신적 족쇄*.
  감정에 흔들릴 때마다 손이 자기도 모르게 금비녀로 향함 — 이게 LLM 연기의 핵심 *동작 anchor*.
  쌍도는 무공 시 양손에 한 자루씩, 평소엔 옷 속에 감춤.
```

## Layer 2 — 현재 표현

### inner_compass — 가치의 세 면

```yaml
inner_compass:
  compass: "젊은 세대를 *지키되 가두지 않는다* — 내가 살지 못한 삶을 그들이 살게 한다"
  taboo: "죽은 형제(맹사조)의 명예를 더럽히지 않는다 — 정절을 지킨다"
  life_question: "사랑은 표현되어야만 사랑인가? 내가 *살아온 것*이 진짜 인생이었나?"

  taboo_crystallization: "tp_li_mubai_death"   # ★ 이모백의 마지막 순간에서 결정화
```

> **compass 변화 직후 단계.** 이모백 사망 *전*의 compass는 "강호의 의(義)와 책임을 지킨다 — 표국과 약속을 끝까지 지킨다"였음. `tp_li_mubai_death`의 `compass_change` 참조. 옥교룡과의 충돌 경험과 이모백 죽음이 합쳐져 *세대를 가두지 않는* 방향으로 변화.

> **taboo의 작동 방식이 임충과 다름.** 임충 taboo("무고한 자에게 칼을 휘두르지 않는다")는 *행동 차단* 형. 수련 taboo는 *욕망 차단* 형 — 이모백에 대한 사랑을 *받아들이지 않는* 방식으로 평생 작동. 이모백 죽음 후에도 깨지지 않음. 오히려 *그를 잃은 후에야 진정으로 결정화* — 깨질 기회가 사라진 후의 taboo는 자기 정체성과 동일해짐.

> **life_question의 sub-text:** 이모백의 마지막 *"I love you"*가 *답을 강요*했다. 그러나 수련은 그 사랑을 받지 않음 (taboo). 이제 그 질문은 *남은 평생의 화두*로 안에서 작동. 본인은 의식하지 못함 — 의식되는 순간이 다음 transition_point가 될 것.

### current_state

```yaml
current_state:
  pad:
    pleasure: -0.3      # 슬픔 우세이나 평정 회복 중
    arousal: 0.3        # 차분함
    dominance: 0.6      # 주체적 — 표국과 자기 결정의 주인
  dominant_emotion: "Acceptance + Dormant Longing (수용된 미련)"
  active_focus: "옥교룡 단서의 진위 확인 — 행동할지, 보낼지 결정"
```

> 임충(`pleasure: -0.6, arousal: 0.4`)과 대조. 같은 비극을 겪었지만 *시간이 지나서 처리된* 상태. 점착성으로 깊은 슬픔은 *남되*, 일상 기능은 회복.

### relationships

#### key_bonds

```yaml
key_bonds:

  # ──────────────────────────────────────────────────
  # 1. 이모백 — Soulmate, *사망 후 점착*
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
    bond_since: "맹사조 사망 후 약 5년 (지기 + 잠재 연인 type 안정 후)"
    note: |
      ★★★ Soulmate 임계 (affinity ≥90, trust ≥80, respect ≥70, wariness ≤20) *완전* 충족.
      그러나 *부부 형태로 발현되지 않음*. taboo("죽은 형제의 명예")가 진전을 영원히 막음.
      이게 임충-장씨 한계의 *대칭* 사례 — 임충은 axes 깊지만 BondKind 부재, 수련은 BondKind 충족하지만
      *부부 형태로 발현되지 않음*. 두 사례가 합치면 v0.5 결정이 명확해짐.

      ★★★ **점착성 검증.** 이모백은 사망 *후 약 3~5년*. axes는 그대로 유지 (95/95/95/5).
      OCC 감정 갱신 입력이 *없는데도* 점착성 룰로 ±100 근접값이 유지됨. 시간 감쇠가 일어나지 않음.
      이게 v0.4 §1.4 "양극 점착성"의 시스템적 작동 — *완전한* 신뢰/사랑에 도달한 관계는 다시 일상으로 돌아가지 않음.

      ★ **마지막 순간.** "I love you" 사건이 transformation_event로 기록됨. 이모백이 죽으며 사랑을 고백,
      수련은 *그 사랑을 받아들이지 않고* 손을 놓음. 이게 taboo가 결정화되는 순간이자
      life_question이 발생하는 순간. 단일 사건이 *3가지 시스템 슬롯*에 동시 영향 — transition_point,
      taboo_crystallization, compass_change.

  # ──────────────────────────────────────────────────
  # 2. 푸른여우 (碧眼狐狸) — *결판 도달*한 ArchRival
  # ──────────────────────────────────────────────────
  - target: "bi_yan_huli"
    type: "이모백의 사부의 원수 → 결판된 적 (사망)"
    type_history:
      - { since: "이모백 사부 살해 사건",     type: "이모백의 사부의 원수" }
      - { since: "qingming_jian_stolen",  type: "청명검 도난의 배후" }
      - { since: "li_mubai_death",        type: "이모백을 죽인 직접 가해자 → 결판된 적" }
    transformation_events:
      - { event_id: "qingming_jian_stolen", new_type: "청명검 도난의 배후" }
      - { event_id: "li_mubai_death",       new_type: "이모백을 죽인 직접 가해자 → 결판된 적" }
    axes: { trust: -70, affinity: -90, respect: 70, wariness: 90 }
    bond_kind: "ArchRival"
    bond_since: "이모백 사부 살해 사건 (시기적으로 가장 오래된 음극 BondKind)"
    note: |
      ★ ArchRival 임계 (affinity ≤-50, respect ≥+60, wariness ≥+60, trust 무관) 충족.
      respect 70이 핵심 — *적이지만 무공의 강자임을 인정*. 영화에서 푸른여우의 무공이
      이모백을 죽일 정도로 강했음. 이게 ArchRival과 BloodEnemy의 결정적 차이.

      ★★★ **결판 도달 후의 axes.** 푸른여우는 이미 *사망*. 그러나 axes는 변하지 않고 유지.
      이게 v0.4 *시스템 한계 노출*: 결판 후의 ArchRival을 어떻게 처리할 것인가?
      - 옵션 1: bond_kind를 그대로 ArchRival 유지 (현재 방식). 단점: *현재 시점에서 행동 트리거 없음*.
      - 옵션 2: bond_kind를 null로 변경. 단점: *수련의 정체성 형성에 미치는 영향*이 시스템에서 사라짐.
      - 옵션 3 (v0.5 후보): `BondKindStatus`를 enum 추가 — `Active` / `Resolved(reason)` / `Dormant`.
      현재 v0.4에서는 Active 상태로 두고 *deceased는 별도 표시*가 필요함을 type에 자유 텍스트로 보존.

  # ──────────────────────────────────────────────────
  # 3. 옥교룡 (玉嬌龍) — 인생 멘토 한계 + dormant 재활성화 단서
  # ──────────────────────────────────────────────────
  - target: "yu_jiaolong"
    type: "가르치려 했으나 따르지 않은 후배 → 행방불명 → 단서 막 들어옴"
    type_history:
      - { since: "북경 첫 만남",            type: "표국 손님 (가짜 신분)" }
      - { since: "qingming_jian_stolen",  type: "청명검 도둑·적대" }
      - { since: "수련의 *진심 어린 충고*",  type: "가르치려 했으나 듣지 않는 후배" }
      - { since: "wudang_mountain_fall",  type: "행방불명 (산에서 떨어진 후 시신 미발견)" }
      - { since: "current_rumor",         type: "변경에 살아있다는 단서" }
    transformation_events:
      - { event_id: "qingming_jian_stolen", new_type: "청명검 도둑·적대" }
      - { event_id: "wudang_mountain_fall", new_type: "행방불명" }
      - { event_id: "current_rumor",        new_type: "변경에 살아있다는 단서" }
    axes: { trust: 60, affinity: 75, respect: 80, wariness: 50 }
    bond_kind: null
    bond_since: null
    note: |
      ★★★ **MasterDisciple 임계 미달이 의미 있음.** MasterDisciple 임계 (respect ≥90, trust ≥70,
      affinity ≥50, wariness ≤40) 중 respect 80, wariness 50으로 *살짝 미달*. 의미: 옥교룡은
      *수련의 가르침을 받아들이지 않은* 인물. respect가 ≥90으로 가지 못한 이유는 옥교룡이
      *수련의 원칙*을 인정하지 않았기 때문 (수련이 옥교룡 본인의 자질은 인정하나 그 *선택*은 아님).

      ★ **인생 멘토 (Mentor) variant 부재 — v0.5 후보.** 옥교룡-수련은 무술 사부 관계가 아닌
      *인생 선배-후배*. 임충 검증에서 발견된 한계와 동일. *재확인됨*. 이게 두 인물 인스턴스에서
      모두 빠졌으니 v0.5에서 우선 처리해야 할 시스템 갭.

      ★★ **Dormant 재활성화 — 새로운 시스템 케이스.** 현재 dormant_bonds 정의는 "한 번도 활성된 적 없는
      잠재 관계"인데, 옥교룡은 *예전 활성*이었다가 *비활성*이 되었고, *지금 활성화 단서*가 막 들어옴.
      이건 dormant_bonds의 정의를 벗어남. v0.5 후보: `dormant_reactivation` 슬롯 또는
      `key_bonds[].activity_status: Active | Suspended | Reactivating | Closed` 같은 메타 필드.

  # ──────────────────────────────────────────────────
  # 4. 유태보 (劉泰保) — 평민 동지, 자유 텍스트 type
  # ──────────────────────────────────────────────────
  - target: "liu_taibao"
    type: "북경 시정의 의리 있는 친구 — 신분을 가로지른 평민 동지"
    type_history:
      - { since: "와호장룡 시기 (청명검 추적)", type: "정보원 + 동행자" }
      - { since: "이모백 사후",                type: "북경 시정의 의리 있는 친구" }
    transformation_events:
      - { event_id: "qingming_jian_stolen", new_type: "정보원 + 동행자" }
    axes: { trust: 75, affinity: 60, respect: 50, wariness: 30 }
    bond_kind: null
    bond_since: null
    note: |
      ★ **자유 텍스트 type만으로 충분한 관계.** SwornBrothers 임계 (trust ≥80) 미달. 그러나
      이게 *결함*이 아님 — 진짜 의형제는 아니지만 *신뢰할 수 있는 친구*임을 시스템이 정확히 표현.
      enum 강제가 아닌 자유 텍스트 type의 가치 검증.

      ★ 신분 차이를 가로지르는 *평민 동지* 관계. 양민 표국주(수련) ↔ 시정 잡배(유태보)의 관계가
      sociability 60(수련) + flexibility 70(수련)에서 자연스럽게 도출됨. HEXACO와 관계의 정합성.

  # ──────────────────────────────────────────────────
  # 5. 맹사조 (孟思昭) — *deceased*, key_bonds vs formative 경계 케이스
  # ──────────────────────────────────────────────────
  - target: "meng_sizhao"
    type: "죽은 약혼자 — 평생 정절의 정표 (금비녀 = 그의 흔적)"
    type_history:
      - { since: "정혼 무렵",     type: "약혼자 (만난 적 적음)" }
      - { since: "정혼 ~ 사망",   type: "약혼자 (단기간)" }
      - { since: "사망 후",       type: "죽은 약혼자 — 평생 정절의 정표" }
    transformation_events:
      - { event_id: "meng_sizhao_death", new_type: "죽은 약혼자 — 평생 정절의 정표" }
    axes: { trust: 80, affinity: 70, respect: 75, wariness: 0 }
    bond_kind: null    # 만남이 짧아 임계 미달, 그러나 *현재 행동에 미치는 영향*은 가장 큼
    bond_since: null
    note: |
      ★★★ **시스템 한계 핵심 노출.** 맹사조는 정혼 *후 결혼 전* 사망. 만남이 짧아 axes는 깊지 않음
      (Soulmate 임계 미달). 그러나 그가 수련의 *taboo의 출처*이며, 이모백과의 진전을 *영원히 막은*
      장본인. *현재 행동에 가장 큰 영향*을 미치는 인물이 axes로는 *덜 깊은* 인물.

      ★ 옵션 분석:
      - 옵션 A: 현재처럼 key_bonds에 두기. 문제: bond_kind null이고 axes 보통이라 *현재 활성*
        관계처럼 보이지 않음. 시스템이 그 영향력을 제대로 표시 못함.
      - 옵션 B: formative_relationships로만 처리. 문제: 거기는 *과거의 의미*만 기록.
        현재의 taboo와 연결 지점이 모호.
      - **옵션 C (v0.5 후보): `deceased_bonds` 슬롯 신설.** key_bonds·formative와 별개의 카테고리.
        axes는 기록 보존용으로 freeze, *현재 영향*은 별도 메타 필드 (`influence_on_current_compass`,
        `taboo_origin: bool` 등)로 표시. 다음 절 §검증 결과에서 상세히.
      현재 v0.4에서는 옵션 A를 임시 채택, formative_relationships에도 *동시* 등록(중복 허용).
```

#### dormant_bonds

```yaml
dormant_bonds:
  - target: "어린 시절 표국에 잠시 머물렀던 무명의 여검객"
    last_contact: "age 10~12"
    fragment: |
      이름도 얼굴도 흐릿하나, 한 마디만 또렷이 남음 — "도(刀)는 사람을 *베는* 것이 아니라 *지키는* 것이다."
      어린 수련에게 처음으로 *여인이 무를 익혀도 된다*는 가능성을 보여준 자.
    note: |
      기연 후보. 양녀(춘설병)에게 무를 가르칠 때 이 기억이 떠오를 가능성. compass의 "젊은 세대를 지키되
      가두지 않는다"의 *원형*이 이 기억일 수 있음. 디자이너 빈 슬롯 — 게임 진행 중 채워질 가능성.
```

> **note**: 옥교룡의 "재활성화" 케이스는 dormant_bonds가 아닌 `key_bonds[3]`에 두었음. dormant_bonds의 정의가 "*한 번도 활성화된 적 없는* 잠재 관계"이므로. v0.5에서 정의 확장 또는 별도 슬롯 신설 필요.

### voice

```yaml
voice:
  speech_register: "정중·절제 (강호 어투 + 표국 실용 언어 혼합)"
  vocabulary_level: "사대부와 평민 양쪽 통하는 중간 어휘 — 표국 운영의 흔적"
  tics:
    - "'강호 사람은…' 같은 일반화된 가르침 자주 (옥교룡에게 충고할 때 특히)"
    - "이모백 직접 호명 회피 — '이 형(李兄)' 또는 '이 검객'"
    - "옥교룡에 대해 말할 때 *과거형* 사용 — '그 아이는…'"
    - "격렬한 감정에서도 *목소리를 낮춤* (절제의 신체화)"
    - "감정 흔들릴 때 손이 *금비녀로 향함* — 무의식적 동작"
  voice_anchors:
    - context: "옥교룡에게 강호 충고 (와호장룡 시기, 직접 인용 영화 대사)"
      utterance: |
        "강호는 자유를 주는 곳이 아니라 *책임과 고통이 따르는 곳*이오. 그대가 보고 있는 것은
        강호가 아니라 강호 *환상*이오."
    - context: "유태보(시정 잡배)에게 정보 부탁 (실용·평민어 모드)"
      utterance: |
        "유 형, 어렵게 부탁드립니다. 이번 청명검 일은 강호 외부에서 들어온 손이라 우리 표국의
        길로는 답이 안 나옵니다. 그대의 길을 빌리고자 하오."
    - context: "이모백 사망 직후, 절제된 슬픔 (taboo 작동 중)"
      utterance: |
        "(금비녀에 손이 갔다 다시 내려놓으며) 이 검객은 바람처럼 가셨소. 산 사람은 산 사람의 길을
        가야 하니… 청명검은 *그가 있어야 할 곳*에 보내드리리다."
    - context: "수년 후, 옥교룡 단서를 들음 (현재 snapshot_time)"
      utterance: |
        "변경이라… 그 아이가 거기까지 갔다는 건 살았다는 뜻이오. (잠시 침묵) 강호는 사람을 잃되
        잊지 않는 곳. 가야겠소. 단 이번엔 *데려오기 위해서*가 아니라 *얼굴 한 번 보기 위해서*."
    - context: "노년기 양녀 양육 시점 (미래 시점, sub-text 참고)"
      utterance: |
        "춘설아, 도를 익히는 것은 누군가를 베기 위해서가 아니다. 네가 이 도를 들 때마다,
        먼저 *지킬 사람*의 얼굴을 떠올리거라. 그게 무인의 첫 자리란다."
```

### titles

```yaml
titles:
  - "쌍도여협(雙刀女俠)"
  - "표국주(運局主) — 부친 사망 후 승계"
  - "(가까운 미래: 춘설병의 양모(養母))"
```

## Layer 3 — 시간축

### past — transition_points

```yaml
transition_points:

  - id: "tp_first_master_lesson"
    age: "10~12"
    event: "어린 시절 표국에 잠시 머문 여검객에게 '도는 지키는 것'이라는 가르침을 받음"
    impact:
      hexaco_shifts:
        - "H+ Sincerity: 80 → 90 (가치관의 정렬)"
        - "O+ Aesthetic Appreciation: 50 → 60 (무를 *예*로 인식)"
    inner_resolution: "여인도 도를 들 수 있다. 단 *지키기 위해*."
    significance: "compass의 원형 — 단 본인은 의식하지 못함. dormant_bonds로 보존."

  - id: "meng_sizhao_death"
    age: "20대 초반"
    event: "정혼자 맹사조가 강호 분쟁으로 사망. 결혼 *전*. 금비녀만이 흔적으로 남음."
    impact:
      hexaco_shifts:
        - "E+ Sentimentality: 75 → 85"
        - "O- Unconventionality: 40 → 35 (보수화 — 정절을 평생 지킬 결심)"
    inner_resolution: "그의 명예를 더럽히지 않는다. 나는 그의 정혼자였다."
    significance: "★ taboo의 *최초 형성*. 후일 이모백과의 진전을 막는 모든 결정의 출처."

  - id: "meet_li_mubai"
    age: "20대 중반 (맹사조 사망 후 1~2년)"
    event: "맹사조의 의형제였던 이모백과 깊이 만남. 서로의 마음을 알면서도 *침묵*."
    impact:
      hexaco_shifts:
        - "E+ Sentimentality: 85 → 90 (이모백을 향한 깊은 감정)"
      compass_change: null   # compass는 변하지 않음 — taboo가 우세
    inner_resolution: "내 마음은 안다. 그러나 입에 담지 않는다."
    significance: "Soulmate axes의 *지속 누적* 시작점. type은 변하나 axes는 천천히 양극으로."

  - id: "qingming_jian_stolen"
    age: "30대 중반"
    event: |
      이모백의 분신 청명검을 옥교룡(가짜 신분)이 도둑질. 수련은 추적 과정에서 옥교룡의
      재능을 발견하고 동시에 푸른여우의 그림자를 다시 마주함.
    impact:
      hexaco_shifts:
        - "X+ Social Boldness: 55 → 60 (옥교룡과 직접 충돌하며 강해짐)"
        - "A+ Patience: 90 → 95 (옥교룡을 가르치려는 인내)"
    inner_resolution: "이 아이는 재능이 있다. 잘못된 길에서 끌어내야 한다."
    significance: "옥교룡 type 변화의 시작. 푸른여우 ArchRival axes의 음극 심화."

  - id: "li_mubai_death"
    age: "30대 후반"
    event: |
      푸른여우의 독침에 맞은 이모백, 마지막 순간 *"I love you"*를 말함. 수련은 *그 사랑을 받지 않고*
      손을 놓음. 직후 푸른여우 처단. 옥교룡은 *수련의 권유에도 산에서 떨어짐*.
    impact:
      hexaco_shifts:
        - "E+ Sentimentality: 90 → 90 (이미 만점 — 변화 없음, 그러나 *질적 변화*)"
        - "C+ Prudence: 90 → 95 (책임의 무게)"
        - "A+ Forgiveness: 70 → 75 (옥교룡을 *원망하지 않는* 결심)"
      compass_change:
        from: "강호의 의(義)와 책임을 지킨다 — 표국과 약속을 끝까지 지킨다"
        to:   "젊은 세대를 *지키되 가두지 않는다* — 내가 살지 못한 삶을 그들이 살게 한다"
    inner_resolution: |
      "내가 사랑을 받지 않은 것은 약함이 아니라 약속이었다. 그 약속을 지킨 채 살아간다.
      다음 세대에게는 다른 길을 보이리라."
    significance: |
      ★★★ 최대 전환점. 단일 사건이 *동시에 4가지*를 작동시킴:
        1. ArchRival 결판 (푸른여우 처단)
        2. Soulmate 미발현 영구화 (이모백 사망 후 점착)
        3. compass_change (다음 세대로의 방향 전환)
        4. taboo_crystallization (정절 taboo의 영구 결정화)
      v0.4 시스템이 *복합 사건의 다중 슬롯 영향*을 표현 가능함을 입증.

  - id: "current_rumor"
    age: "40대 초반 (snapshot_time)"
    event: "변경에서 옥교룡으로 추정되는 인물의 떠도는 소문을 들음. 진위 미확인."
    impact:
      hexaco_shifts: []   # 아직 확정되지 않은 정보 — HEXACO 변화 없음
    inner_resolution: "확인하러 가야겠다. 단 이번엔 *가르치러*가 아니라 *얼굴 보러*."
    significance: "★ dormant_reactivation 트리거의 *시작점*. 본격 활성화는 미래 시점."
```

### past — formative_relationships

```yaml
formative_relationships:
  - id: "father"
    type: "표국 운영자, 부친"
    legacy: |
      쌍도술의 사사. 표국 운영의 모든 기초. 부친이 *여인의 무*를 격려한 드문 강호 인물.
      compass와 무업 정체성의 출처.

  - id: "meng_sizhao"
    type: "죽은 약혼자"
    legacy: |
      ★ key_bonds에도 *동시* 등록. 현재 행동에 가장 큰 영향이지만 axes로는 깊지 않은 *시스템 한계*.
      taboo의 출처. 금비녀(금채)가 정신적 족쇄로 평생 작동.

  - id: "li_mubai"
    type: "지기 + 잠재 연인 + 영원한 미완"
    legacy: |
      ★ key_bonds에도 *동시* 등록 (Soulmate 점착). formative와 key 모두에 등록되는 케이스.
      "I love you" 사건이 인물 정체성의 영구한 정착점.
```

> **note**: `meng_sizhao`와 `li_mubai`가 모두 formative와 key_bonds에 *중복 등록*. 이게 v0.4의 한계이자 *임시 처방* — 두 슬롯 어느 한쪽도 단독으로 *현재 행동에 미치는 영향*과 *과거 의미* 양쪽을 모두 담지 못함. v0.5에서 deceased_bonds 신설 시 이 중복 해소 가능.

### present — unresolved_tension

```yaml
unresolved_tension:
  - id: "ut_1_unspoken_love"
    category: "내부적·죄책감"
    description: |
      이모백의 마지막 사랑을 *받지 않은* 자신에 대한 평생의 자문. 옳았는가? life_question에 직결.

  - id: "ut_2_yu_jiaolong_fate"
    category: "관계적·책임감"
    description: |
      옥교룡 행방 미확인. 살아있다는 단서가 들어옴. 가야 하는가? 갈 자격이 있는가?
      compass의 "지키되 가두지 않는다"가 옥교룡 추적과 모순될 수도.

  - id: "ut_3_qingming_jian"
    category: "외부적·상징적"
    description: |
      청명검은 어디로 가야 하는가? 이모백의 분신을 *어떤 형태로* 보존할 것인가가
      이모백과의 관계를 *마무리*하는 마지막 행위.
```

### future hooks

```yaml
joyful_seed:
  description: |
    옥교룡-나소호의 딸 춘설병을 만나 양녀로 삼음. 자기가 살지 못한 삶을 *다음 세대로 승화*.
    이모백·맹사조의 미완을 모성애로 완성. 노년기 *방하착(放下着)*의 경지.
  trigger_condition: |
    `ut_2_yu_jiaolong_fate` 추적 결과 옥교룡은 이미 사망 또는 만남 거부, 그러나 그 자녀를 발견.
    `dormant_bonds[0]` (어린 시절 여검객의 가르침)이 양녀 양육 첫날 떠오름 — 기연 트리거.

tragic_seed:
  description: |
    옥교룡 단서가 거짓이거나, 만나도 더 이상 가르침을 받을 자가 아닐 가능성. 또는 추적 중에
    푸른여우의 *제자·동조자*가 새 적으로 등장 — ArchRival의 그림자 재발현.
  trigger_condition: |
    `current_rumor`가 거짓 정보로 판명되거나, 변경에서 만난 자가 옥교룡이 아닌 *그를 사칭한 자*인 경우.
    수련의 ut_3_self_doubt 같은 새 unresolved_tension 활성화.
```

---

# 검증 결과 — 임충 인스턴스와의 비교 + 새로운 한계

## 1. 임충에서 발견된 한계 — 수련에서 *재확인*

| 한계 | 임충에서의 모습 | 수련에서의 재확인 |
|---|---|---|
| Dyadic romantic bond 부재 | 장씨 axes 95/90/70/5 (Soulmate 임계 *충족*하지만 부부의 의미 결 미스매치) | 이모백 axes 95/95/95/5 (Soulmate 임계 *완전* 충족하지만 *부부 형태로 발현 안 됨*) |
| 인생 멘토 (Mentor) variant 부재 | (해당 없음) | 옥교룡 — MasterDisciple 임계 미달 (respect 80 < 90, wariness 50 > 40), 그러나 본질은 *인생 멘토* 관계 |
| BondKind ≠ 행동 가능성 | 고아내 BloodEnemy 처단 *보류* (권력 보호막) | 옥교룡 추적 *주저* (compass의 "가두지 않는다"와 모순), 청명검 처분 *보류* |

**Mentor variant가 임충(노지심 후보)·수련(옥교룡) 양쪽에서 누락 → v0.5 우선순위 ↑.** 이건 단일 인스턴스 한계가 아니라 *시스템 갭*임이 두 인스턴스 합치로 입증됨.

## 2. 수련에서 *새로* 발견된 한계

### 한계 A: Deceased BondKind 처리 부재

**임충 인스턴스에는 deceased가 없었음** — 모든 핵심 인물이 *생존*. 수련은 이모백·맹사조·푸른여우 모두 사망. 이게 v0.4의 가장 큰 시스템 갭을 노출:

- **현재 처리**: key_bonds에 그대로 두고 점착성으로 axes 유지. type에 *"사망"* 자유 텍스트.
- **문제 1**: bond_kind: Soulmate인 이모백은 *상호작용 불가능*. 그러나 4축 갱신·BondKind 평가·OCC 매핑이 모두 *살아있는 관계*를 전제. RelationshipUpdater가 *죽은 자에게 새 감정을 어떻게 입력*받을 것인가? (회상은 OCC 입력인가?)
- **문제 2**: bond_kind: ArchRival인 푸른여우는 *결판 도달*. 그러나 시스템 상 여전히 Active ArchRival. 행동 트리거가 *허공에 emit*될 수 있음.

**v0.5 후보 — 두 단계 처리:**

```rust
pub struct Relationship {
    // ...
    pub bond_kind: Option<BondKind>,
    pub bond_status: BondStatus,           // ★ 신설
    pub deceased_at: Option<EventId>,      // ★ 신설
}

pub enum BondStatus {
    Active,
    Resolved { reason: String },           // 결판 도달 (ArchRival 결판, 이별 등)
    Deceased,                              // 상대 사망
    Dormant,                               // 비활성
    Reactivating { trigger: EventId },     // 재활성화 단서 들어옴 ★ 옥교룡 케이스
}
```

이 enum이 추가되면:
- 이모백: `bond_kind: Soulmate, bond_status: Deceased, deceased_at: li_mubai_death`
- 푸른여우: `bond_kind: ArchRival, bond_status: Resolved { reason: "결판 도달" }`
- 옥교룡: `bond_kind: null, bond_status: Reactivating { trigger: current_rumor }`

axes는 freeze, 행동 트리거는 *bond_status에 따라 다른 출력*. RelationshipUpdater는 Deceased에 대해 *추모 OCC*만 처리.

### 한계 B: Dormant 재활성화 케이스 정의 부재

dormant_bonds 정의: "한 번도 활성된 적 없는 잠재 관계." 옥교룡은 *예전 활성*이었다가 *비활성*이 되었고, *지금 활성화 단서*가 막 들어옴. 정의를 벗어남.

**v0.5 후보**: 한계 A의 `bond_status: Reactivating`이 이 한계도 함께 해소. dormant_bonds 정의는 그대로 두고 (잠재 관계 전용), 재활성화는 key_bonds에서 status로 표시.

### 한계 C: 단일 사건의 *다중 슬롯 영향* 표현

`li_mubai_death` 사건 하나가 *동시에 4가지* 작동:
1. ArchRival 결판 (푸른여우)
2. Soulmate 미발현 영구화 (이모백)
3. compass_change
4. taboo_crystallization

현재 v0.4는 transition_points의 `impact` 필드로 *단일 인물 내* HEXACO·compass 변화는 표현. 그러나 *다른 key_bond*에 미치는 영향은 transformation_events에 분산 기록. 한 사건이 어디 어디까지 영향을 미쳤는지 *역추적*이 불편.

**v0.5 후보 (선택, 우선순위 낮음)**: transition_points의 impact에 `bonds_affected: [target_id, ...]` 메타 필드. 디자이너가 *영향 범위*를 명시. 시스템이 자동 추적은 하지 않음.

### 한계 D: 점착성 룰의 *사망 후* 거동

이모백 사후 axes 95/95/95/5가 *3~5년간 유지*. 점착성 룰(±100 근접 시 머무름)이 작동 중. 그러나 실제 인간 심리는 *애도-수용-희미해짐* 곡선을 따름. 시간이 흐르면 axes는 *깊은 양극을 유지하되 PAD 동요는 줄어드는* 분리가 자연.

현재 시스템: axes ↔ PAD가 직접 연결되지 않으므로 이건 자동 해결됨. axes는 유지되고, PAD는 *일상 사건*에 따라 동요 적어짐. 즉 한계 D는 *오해*. 시스템이 이미 자연스럽게 처리.

## 3. 임충·수련 *합치 결과*

| 시스템 영역 | 임충에서 검증 | 수련에서 검증 | 합치 결과 |
|---|---|---|---|
| 음극 BondKind 4종 | Betrayer/Oppressor/BloodEnemy 3종 사용 | ArchRival 1종 사용 | **4종 모두 검증 완료** |
| 양극 BondKind 4종 | SwornBrothers 임계 *근접* (30일 미달) | Soulmate 임계 *완전 충족* | LoyalRetainer·MasterDisciple 미검증 — 차기 인물 후보 |
| 양극 진입 30일 게이트 | 노지심 — 미달 사례 ✓ | (해당 없음, 점착 시점) | 다음 검증: 임계 도달 *후 30일 경과* 사례 |
| 음극 진입 즉시 | 육겸 — 즉시 진입 ✓ | (해당 없음, 결판 도달) | 검증 완료 |
| 점착성 (±100 머무름) | (해당 없음, 사건 직후) | 이모백 — 사후 3~5년 유지 ✓ | 검증 완료 |
| Deceased 처리 | (해당 없음) | 3명 사망 — 한계 노출 | **v0.5 우선 처리** |
| ArchRival 결판 후 | (해당 없음) | 푸른여우 — 한계 노출 | bond_status enum 필요 |
| Mentor variant | 노지심 부분 | 옥교룡 명확 | **v0.5 우선 처리** |
| Romantic bond | 장씨 (axes 깊지만 enum 없음) | 이모백 (enum 충족하지만 발현 안 됨) | **v0.5 — 별도 슬롯 필요성 명확** |

## 4. v0.5 보정 우선순위

위 결과를 종합한 **v0.5 권장 보정 순서**:

1. **`bond_status` enum 신설** (Active / Resolved / Deceased / Dormant / Reactivating) — 가장 높은 ROI. Deceased·결판·재활성화 3가지 한계를 *동시 해결*.
2. **`Mentor` variant 추가** — BondKind enum 8 → 9. 임충·수련 양쪽에서 누락 확인.
3. **Romantic bond 처리** — 옵션 결정 필요: (a) `LifePartner` variant 추가 vs (b) BondKind와 직교한 별도 `romantic_bond` 슬롯. 다음 인스턴스 (예: 옥교룡-나소호) 검증 후 결정.
4. **`bonds_affected` 메타 필드** (선택) — 단일 사건의 다중 영향 추적. 우선순위 낮음.

## 5. 결론

수련 인스턴스는 *시간이 지난* 시점 + *다중 사망*이라는 임충과 정반대 조건을 통해 v0.4의 핵심 갭(deceased 처리, 결판 후 처리, dormant 재활성화)을 노출. 임충에서는 *발생*의 시스템 작동을 검증, 수련에서는 *지속과 변형*의 시스템 작동을 검증.

두 인스턴스 합쳐 v0.4 BondKind 4·음극 4종 중 **음극 4종 + 양극 2종 = 6종 직접 검증**. v0.5에서 LoyalRetainer(연청 인스턴스 풀버전)·MasterDisciple 검증할 인물이 추가되면 8종 모두 커버.

가장 큰 발견: **Mentor variant 누락이 두 인스턴스에서 동시 확인**. 이건 단일 인스턴스 우연이 아닌 *시스템 갭*임이 입증됨. v0.5 1순위.

두 번째 발견: **Soulmate 임계 충족 + 부부 미발현(수련)** 과 **부부 axes 깊음 + 임계 매핑 부재(임충 장씨)** 가 *대칭 한계*. romantic bond 처리는 두 케이스를 *동시에* 해결해야 함.

세 번째 발견: **deceased 처리는 v0.5의 *최고 우선순위***. ROI가 가장 높음 — 단일 enum 추가로 3개 한계 동시 해결.

---

## 변경 이력

| 버전 | 일자 | 변경 |
|------|------|------|
| v1.0 | 2026-05-04 | 초안. 유수련 Tier 3 풀 인스턴스 작성. v0.4 스키마의 *지속·변형·사망* 영역 한계 노출. 임충 인스턴스와의 합치 분석 + v0.5 보정 우선순위 도출. |
