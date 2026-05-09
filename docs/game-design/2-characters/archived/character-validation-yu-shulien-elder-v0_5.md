# 인물 스키마 v0.5 검증 — 노년기 유수련 (兪秀蓮)

> 작성일: 2026-05-04
> 검증 대상: `_schema.md` v0.5 + `relationships.md` v0.5
> 위치: `docs/game-design/2-characters/character-validation-yu-shulien-elder-v0_5.md`
> 동반 인스턴스:
>   - 장년기 수련: `character-validation-yu-shulien-v0_5.md` (40대 초반)
>   - 임충: `character-validation-lin-chong-v0_5.md`

## 검증 목적 — snapshot_time 시스템의 본격 검증

이 인스턴스는 **장년기 수련의 시간적 후속**이다. 같은 인물 (`id: yu_shulien`)의 다른 시점.

v0.3에서 도입한 `snapshot_time` 슬롯이 *진정으로 검증*되는 것은 같은 인물의 두 인스턴스가 *명확히 다른 상태*를 가지면서도 *같은 정체성의 연속*임을 시스템이 표현할 때다. 이전까지는 단일 인스턴스만 작성했으므로 검증 불완전.

핵심 검증 항목:

| 검증 | 어떻게 |
|---|---|
| **snapshot_time이 정체성을 분기시키는가** | 같은 id, 같은 origin·HEXACO 큰 골격, 다른 상태 변수 |
| **compass_change 일관성** | 장년기 ↔ 노년기 compass가 다르면 그 사이 transition_point에 compass_change 필수 |
| **BondKind 일생 변화** | 5개 기존 bonds의 status·axes 변화 |
| **점착성 + Deceased terminal 룰** | 이모백·맹사조 axes 그대로 유지되는가 |
| **Guardian variant 부재** | 춘설병 관계를 v0.5로 어떻게 표현할 것인가 — 명확한 시스템 갭 노출 |
| **PAD 변화 vs axes 점착성의 분리** | 같은 axes에서 PAD는 일상으로 변화 |

---

# 유수련(兪秀蓮) — 노년기 인스턴스

## Layer 1 — 본바탕

### identity

```yaml
id: "yu_shulien"          # ★ 장년기 인스턴스와 *동일* — 같은 인물
name: "유수련(兪秀蓮)"
nicknames:
  - "쌍도여협(雙刀女俠)"          # 유지
  - "표국의 옛 여주인"            # ★ "옛" 추가 — 표국 운영 위임
  - "춘설병의 양모"               # ★ 신규
  - "노 협객"                    # ★ 신규
era: "청대 (왕도려 학철오부곡 세계관, 철기은병 시기)"
stage_of_life: "노년기 진입"
snapshot_time: |
  ★★★ 수련 50대 초반~중반. 변경에서 옥교룡과 짧은 재회 후 약 5~6년 경과.
  옥교룡 사망 소식 후 약 3~4년. 나소호 사망 후 약 1년. 춘설병을 양녀로 들이고 1~2년.
  현재: 춘설병에게 첫 쌍도술을 가르치기 시작한 시점.
```

> **장년기 인스턴스의 snapshot_time과의 거리:**
> - 장년기: "이모백 사망 후 약 3~5년. 옥교룡이 변경에 살아있다는 소문을 막 들은 직후."
> - 노년기: "그로부터 *약 10~12년 경과*."
> 일생의 *마지막 큰 호*가 이 사이에 펼쳐졌음.

### origin

```yaml
# ★ 장년기와 동일 — origin은 인물의 본바탕이라 변화 없음
birthplace: "북경"
social_origin: "양민 → 표국주(運局主) → 노년 위임"
kingdom_of_origin: "청"
family_background: |
  명문가는 아니나 양민 중 *기예 있는 집안*. 부친이 북경에서 표국 운영.
  거친 표국 생활을 하며 강호 문법을 일찍 익힘.
```

### temperament — HEXACO 24 facet

> 노년기에 따른 *미세 변화*만. HEXACO는 base이므로 큰 변화 없음. 단 일생의 사건이 누적되어 일부 facet이 천천히 이동.

```yaml
H_honesty_humility:
  sincerity: 90              # 그대로
  fairness: 90
  greed_avoidance: 90        # ↑ (85 → 90, 노년 더 비물질적)
  modesty: 85
E_emotionality:
  fearfulness: 25
  anxiety: 40                # ↓ (50 → 40, 큰 비극을 모두 겪고 통과한 후의 평정)
  dependence: 35
  sentimentality: 90         # 그대로 — 일생 미련의 핵심
X_extraversion:
  social_self_esteem: 75     # ↑ (70 → 75, 노년의 자기 안정)
  social_boldness: 60
  sociability: 55            # ↓ (60 → 55, 더 차분)
  liveliness: 35             # ↓ (40 → 35)
A_agreeableness:
  forgiveness: 80            # ↑ (75 → 80, 노년의 관용)
  gentleness: 80             # ↑ (75 → 80)
  flexibility: 70
  patience: 95               # 그대로
C_conscientiousness:
  organization: 90
  diligence: 85              # ↓ (90 → 85, 표국 위임으로 약화)
  perfectionism: 75          # ↓ (80 → 75)
  prudence: 95               # 그대로
O_openness:
  aesthetic_appreciation: 70 # ↑ (65 → 70, 노년의 자연·시 감상)
  inquisitiveness: 55        # ↓ (60 → 55)
  creativity: 65
  unconventionality: 40      # ↑ (35 → 40, ★ 양녀 양육이라는 비전통적 결정)
```

> **변화 패턴**: A+ Forgiveness·Gentleness 상승, E+ Anxiety 하강, O+ Unconventionality 상승. 노년의 *수용·관용·온화* 이행이 시스템적으로 표현.
> **유지 패턴**: H+ Sincerity 90, A+ Patience 95, C+ Prudence 95, E+ Sentimentality 90. *인물의 본질*은 변하지 않음.

### body

```yaml
physical_description: |
  50대 초반~중반. 단정한 얼굴에 깊은 주름. 차분한 눈매에 *수용된 슬픔*이 어림.
  머리에 여전히 *금비녀(금채)*. 머리는 반백.
signature_feature: |
  **금비녀(金釵)** — 변하지 않음. 평생의 정신적 족쇄이자 정체성의 표지.
  단 노년에는 손이 금비녀로 향하는 동작이 *덜 빈번*. 감정 흔들림이 줄어든 결과.
  쌍도는 평소 가르침용 한 자루만 휴대. 다른 한 자루는 춘설병에게 증여.   # ★ 신규
```

## Layer 2 — 현재 표현

### inner_compass — 가치의 세 면

```yaml
inner_compass:
  compass: "젊은 세대를 *지키되 가두지 않는다* — 내가 살지 못한 삶을 그들이 살게 한다"
  taboo: "죽은 형제(맹사조)의 명예를 더럽히지 않는다 — 정절을 지킨다"
  life_question: "사랑은 표현되어야만 사랑인가? 내가 *살아온 것*이 진짜 인생이었나?"
  taboo_crystallization: "tp_li_mubai_death"
```

> **장년기와 비교**: compass·taboo·life_question·taboo_crystallization 모두 *동일*. 즉 *변화 없음*.

> **★ 변화 없음의 의미**: compass는 장년기 li_mubai_death에서 *바뀐 후 안정*. 노년기 양녀 양육은 그 compass의 *직접 실행*이지 새 변화가 아님. snapshot_time이 다르지만 compass는 같으므로 그 사이에 `compass_change`가 있는 transition_point가 *없어야 함* — 일관성 검증 ✓.

> **life_question의 답을 찾는 중**: 양녀 양육이 life_question에 *부분적 답*. "내가 살지 못한 삶을 그녀가 살게 함"이 답의 일부. 그러나 *완전한 답은 없음*. 의문 자체는 그대로 두되, current_state에 답을 찾는 *과정*이 반영됨.

### current_state — ★★ 핵심 검증: axes 점착성 vs PAD 변화

```yaml
current_state:
  pad:
    pleasure:  0.1     # ★ 장년기 -0.3 → 노년기 0.1. 양녀와의 일상에서 작은 기쁨.
    arousal:   0.2     # ★ 장년기 0.3 → 0.2. 더 평온.
    dominance: 0.7     # ★ 장년기 0.6 → 0.7. 어른으로서의 안정.
  dominant_emotion: "Maternal Affection + Lingering Sorrow (모성과 수용된 슬픔의 공존)"
  active_focus: "춘설병의 첫 쌍도술 — 어떻게 가르칠 것인가"
```

> ★★★ **시스템 검증의 가장 강한 사례**: 같은 인물의 같은 axes (이모백 95/95/95/5는 그대로) — 그러나 *현재 PAD*는 변화. 점착성 룰은 *axes에만 적용*, *PAD는 일상 사건에 따라 자유롭게 변동*. 노년의 일상(양녀와의 시간)이 PAD를 양수로 끌어올림. 슬픔은 *깊이 있되 동요는 줄어듦* — 이게 점착성과 PAD 분리의 정확한 작동.

### relationships

#### key_bonds — 7개 (5개 기존 + 2개 신규)

```yaml
key_bonds:

  # ──────────────────────────────────────────────────
  # 1. 이모백 — Soulmate + Deceased + null (★ 변화 없음 — Deceased terminal)
  # ──────────────────────────────────────────────────
  - target: "li_mubai"
    type: "영원히 미완의 사랑 — 오랜 사별의 인연"   # ★ "오랜" 추가, 시간 두께 표시
    type_history:
      - { since: "맹사조 사망 전",          type: "약혼자의 의형제" }
      - { since: "맹사조 사망 후",          type: "지기 + 잠재 연인" }
      - { since: "qingming_jian_stolen",  type: "함께 싸우는 동지" }
      - { since: "li_mubai_death",        type: "영원히 미완의 사랑" }
      - { since: "장년기 ~ 노년기 사이",     type: "오랜 사별의 인연" }   # ★ 신규
    transformation_events:
      - { event_id: "li_mubai_death", new_type: "영원히 미완의 사랑" }
    axes: { trust: 95, affinity: 95, respect: 95, wariness: 5 }   # ★ 변화 없음 — Deceased freeze
    bond_kind: "Soulmate"
    bond_status: "Deceased"
    partnership: null
    deceased_at: "li_mubai_death"
    bond_since: "맹사조 사망 후 약 5년"
    note: |
      ★★★ **Deceased terminal + 점착성** 검증의 핵심 사례.
      장년기 인스턴스 (이모백 사후 3~5년) → 노년기 인스턴스 (사후 약 13~17년).
      약 10년이 더 지났으나 axes는 *완전히 동일* (95/95/95/5).
      Deceased status로 axes freeze + 양극 점착성 룰의 결합 작동.

      ★ 그러나 *PAD 동요는 줄어듦*. 회상 OCC가 발생해도 강도가 더 낮음.
      이게 v0.5 §4.5 "axes는 정체성, PAD는 일상" 원칙의 정확한 작동.

      type에 "오랜 사별의 인연" 추가 — 자유 텍스트로 *시간 두께*만 보존.

  # ──────────────────────────────────────────────────
  # 2. 푸른여우 — ArchRival + Resolved + null (★ 변화 없음 + 잊혀가는 적)
  # ──────────────────────────────────────────────────
  - target: "bi_yan_huli"
    type: "결판된 적 — 거의 잊혀진 자"     # ★ "거의 잊혀진" 추가
    type_history:
      - { since: "이모백 사부 살해 사건",     type: "이모백의 사부의 원수" }
      - { since: "li_mubai_death",        type: "이모백을 죽인 직접 가해자 → 결판된 적" }
      - { since: "장년기 ~ 노년기 사이",     type: "결판된 적 — 거의 잊혀진 자" }   # ★ 신규
    transformation_events:
      - { event_id: "li_mubai_death", new_type: "이모백을 죽인 직접 가해자 → 결판된 적" }
    axes: { trust: -70, affinity: -90, respect: 70, wariness: 90 }   # ★ 변화 없음 — Resolved freeze
    bond_kind: "ArchRival"
    bond_status: { Resolved: { reason: "이모백의 복수로 처단" } }
    partnership: null
    bond_since: "이모백 사부 살해 사건"
    note: |
      Resolved status도 Deceased와 마찬가지로 axes freeze. 노년기에도 그대로 보존.
      차이: Deceased는 *추모* 색채, Resolved는 *결판 후의 거리감*.
      회상 OCC 강도가 매우 낮음 — 거의 떠올리지 않음. type의 "거의 잊혀진"이 그 표현.

      v0.5 검증 포인트: BondKind와 Status가 *영구 보존*되어 인물 정체성에 누적되는 *역사 층*을
      형성. 정체성은 *현재의 axes·PAD*가 아닌 *지나온 모든 BondKind*의 합.

  # ──────────────────────────────────────────────────
  # 3. 옥교룡 — Mentor + Deceased + null (★★★ status 전환 흐름 완결 검증)
  # ──────────────────────────────────────────────────
  - target: "yu_jiaolong"
    type: "가르치려 했으나 자기 길을 간 후배 — 변경에서 *살다 간* 자"
    type_history:
      - { since: "북경 첫 만남",             type: "표국 손님 (가짜 신분)" }
      - { since: "qingming_jian_stolen",   type: "청명검 도둑·적대" }
      - { since: "shulien_advice",         type: "가르치려 했으나 듣지 않는 후배" }
      - { since: "wudang_mountain_fall",   type: "행방불명" }
      - { since: "current_rumor",          type: "변경에 살아있다는 단서" }
      - { since: "bian_jing_meeting",      type: "변경에서 살아있는 후배 (재회)" }     # ★ 신규
      - { since: "yu_jiaolong_death",      type: "변경에서 살다 간 자" }              # ★ 신규
    transformation_events:
      - { event_id: "shulien_advice",       new_type: "가르치려 했으나 듣지 않는 후배" }
      - { event_id: "wudang_mountain_fall", new_type: "행방불명" }
      - { event_id: "current_rumor",        new_type: "변경에 살아있다는 단서" }
      - { event_id: "bian_jing_meeting",    new_type: "변경에서 살아있는 후배 (재회)" }
      - { event_id: "yu_jiaolong_death",    new_type: "변경에서 살다 간 자" }
    axes: { trust: 65, affinity: 80, respect: 80, wariness: 35 }   # ★ 변경 — 만남으로 갱신 후 freeze
    bond_kind: "Mentor"
    bond_status: "Deceased"     # ★★★ Reactivating → Active(짧게) → Deceased 흐름 완결
    partnership: null
    deceased_at: "yu_jiaolong_death"
    bond_since: "shulien_advice 후 14일 (장년기 인스턴스 시점)"
    note: |
      ★★★ **status 전환 흐름의 완결 검증.** 장년기 → 노년기 사이에 status가 3번 전환:
        1. (장년기 시점) Reactivating { trigger: current_rumor }
        2. (장년기 + 약 5~6년) bian_jing_meeting → Active 짧게 복귀
            - 변경에서 만난 옥교룡은 나소호와 *가족을 이룬 자유로운 자*. 수련은 *떠남*.
            - 짧은 만남에서 OCC: HappyFor (그녀가 행복함) + Pride (그녀가 자기 길을 찾음)
            - axes 부분 갱신: trust 60→65, affinity 75→80, wariness 50→35
            - "더 이상 가르칠 자가 아님" — Mentor 역할은 *완료된 형태로*  남음
        3. (장년기 + 약 8~10년) yu_jiaolong_death → Deceased
            - 옥교룡이 변경에서 자연사. 평온한 죽음.
            - axes freeze. respect 80 유지 — *그녀가 자기 삶을 완성*했다는 인정.

      ★ axes의 *가벼운 갱신*이 의미 있음. 사망 *전*에 만남에서 갱신 → 그 갱신값으로 freeze.
      "더 이상 가르칠 자가 아니나, 그녀의 삶이 옳았다"는 마지막 인식이 axes에 보존.

      ★ Mentor BondKind는 그대로 유지 — *역할의 형태*는 영구. 단 행동 트리거 (가르치려 함)는 불활성.
      compass의 "지키되 가두지 않는다"가 *옥교룡에게 행동으로 입증*된 사례 — 변경에서 *떠나옴*.

  # ──────────────────────────────────────────────────
  # 4. 유태보 — null + Active + null (★ 시간이 흐른 우정의 변화)
  # ──────────────────────────────────────────────────
  - target: "liu_taibao"
    type: "북경 시정의 의리 있는 친구 — 평생의 우인"   # ★ "평생의" 추가
    type_history:
      - { since: "와호장룡 시기",     type: "정보원 + 동행자" }
      - { since: "이모백 사후",       type: "북경 시정의 의리 있는 친구" }
      - { since: "노년기",           type: "평생의 우인" }                    # ★ 신규
    transformation_events:
      - { event_id: "qingming_jian_stolen", new_type: "정보원 + 동행자" }
    axes: { trust: 80, affinity: 70, respect: 60, wariness: 20 }   # ★ 누적 갱신
    bond_kind: null     # ★ SwornBrothers 임계 (trust ≥80, affinity ≥70, respect ≥60, wariness ≤30) 도달!
    bond_status: "Active"
    partnership: null
    bond_since: null
    note: |
      ★★ **양극 진입 시간 게이트의 *반대* 검증.** 장년기 (75/60/50/30) → 노년기 (80/70/60/20).
      약 10년의 *일상 우정*이 axes를 천천히 누적시켜 SwornBrothers 임계 *바로 도달*.

      그러나 bond_kind: null인 이유:
      1. 임계 도달 후 *연속 30일 유지* 룰. 이제 막 도달했으므로 카운트 시작.
      2. 신분 차이를 가로지르는 평민 동지의 본질이 *형제 결*과 다름. SwornBrothers의 동귀어진 트리거가
         적합한가? 우정의 결은 더 차분 — 자기희생까진 아닌 *깊은 신뢰*.

      → 해석 옵션 두 가지:
      - 옵션 A: SwornBrothers 카운트 진행, 30일 후 진입. 이게 v0.5 룰의 자연 작동.
      - 옵션 B: enum 강제 회피. type "평생의 우인"이 SwornBrothers와 결이 달라 null 유지.

      현재 인스턴스에서는 옵션 B 채택 — 자유 텍스트 type이 더 정확. 시스템적으로는 옵션 A도 가능
      (디자이너 선택). 이게 *임계 도달이 자동 진입을 강제하지 않는* v0.5의 *재량 영역*. 추후 Companion
      또는 Friend 같은 *덜 무거운 양극 variant*가 검토될 수 있음 (v0.6 후보).

  # ──────────────────────────────────────────────────
  # 5. 맹사조 — null + Deceased + Engaged (★ 변화 없음, 시간이 흐를수록 *멀어진 기억*)
  # ──────────────────────────────────────────────────
  - target: "meng_sizhao"
    type: "죽은 약혼자 — 평생 정절의 정표 (시간이 흘러 흐려진 얼굴)"   # ★ "흐려진" 추가
    type_history:
      - { since: "정혼 무렵",     type: "약혼자" }
      - { since: "사망 후",       type: "죽은 약혼자 — 평생 정절의 정표" }
      - { since: "노년기",        type: "흐려진 얼굴이지만 살아있는 정표" }    # ★ 신규
    transformation_events:
      - { event_id: "engagement_event",  new_type: "약혼자" }
      - { event_id: "meng_sizhao_death", new_type: "죽은 약혼자" }
    axes: { trust: 80, affinity: 70, respect: 75, wariness: 0 }   # ★ 변화 없음 — Deceased freeze
    bond_kind: null
    bond_status: "Deceased"
    partnership: "Engaged"      # ★ 영구 보존 — 정혼은 깨지지 않음
    deceased_at: "meng_sizhao_death"
    bond_since: null
    note: |
      ★★ **Partnership: Engaged의 영구성 검증.** 약 30년 전의 정혼이 노년에도 그대로 보존.
      금비녀가 여전히 머리에 — *형식의 영구성*이 정체성의 핵심.

      type의 "흐려진 얼굴" — 시간이 흘러 *기억의 선명도*는 약해짐. 그러나 *효과*는 영구.
      axes가 freeze이므로 시스템적으로는 동일하나, 자유 텍스트가 *시간 두께*를 더함.

      ★ 비교: 이모백("오랜 사별의 인연") vs 맹사조("흐려진 얼굴이지만 살아있는 정표").
      둘 다 Deceased이나 *기억의 질감*이 다름. 이모백은 *깊은 추모*, 맹사조는 *정체성의 정표*.
      이게 BondKind 차이의 결과 — Soulmate(이모백) vs null + Engaged(맹사조).
      후자는 BondKind 정서 차원이 비어있고 Partnership 형식 차원만 영구.

  # ──────────────────────────────────────────────────
  # 6. ★ 춘설병 — MasterDisciple + Active + null (★★★ Guardian variant 한계 핵심)
  # ──────────────────────────────────────────────────
  - target: "chun_xue_bing"
    type: "양녀이자 후계자 — 옥교룡의 핏줄, 내가 가르치는 다음 세대"
    type_history:
      - { since: "nasoho_death",        type: "맡겨진 옥교룡의 딸" }
      - { since: "chunxue_adoption",    type: "양녀" }
      - { since: "first_lesson",        type: "양녀이자 후계자 — 옥교룡의 핏줄, 내가 가르치는 다음 세대" }
    transformation_events:
      - { event_id: "chunxue_adoption", new_type: "양녀" }
      - { event_id: "first_lesson",     new_type: "양녀이자 후계자" }
    axes: { trust: 75, affinity: 90, respect: 60, wariness: 25 }   # ★ 빠른 양극 도달 — 양육의 OCC 누적 강함
    bond_kind: "MasterDisciple"   # ★★★ 임시 처방 — 무술 사사 결로 매핑
    bond_status: "Active"
    partnership: null
    bond_since: "first_lesson 후 7일 시점 (양극 진입 14~30일 룰 카운트 중)"   # ★ 진입 카운트 진행
    note: |
      ★★★ **v0.5 시스템 한계 핵심 노출.**

      수련-춘설병의 본질:
      1. 양모-양녀 (모성)
      2. 무술 사부-제자 (비전 전수)
      3. 일생 마지막의 핵심 관계 (life_question에 대한 *답*)

      위 3가지가 모두 핵심이나, BondKind 9 variants 중 *어느 것도 1번을 표현 못함*.

      **임시 처방 (현재 인스턴스):** MasterDisciple variant를 사용하되 type/note에서 양모성을 강조.
      MasterDisciple 임계 (respect ≥+90, trust ≥+70, affinity ≥+50, wariness ≤40) 검증:
        - respect 60 < 90: ★ 미달. *어린 양녀에 대한 respect*는 자질 인정이지 압도적 존경 아님.
        - 결국 MasterDisciple도 *완벽한* 매핑은 아님.
        - 임시 처방으로 두되 v0.6에서 Guardian/Parent variant 정식 도입 필요.

      **v0.6 후보 — Guardian variant 제안:**
      ```rust
      Guardian,    // 부모-자녀형 (양육 + 보호 + 가르침. 친·양 무관)
      // 임계: trust ≥+70, affinity ≥+80, respect 무관, wariness ≤30
      // 자기희생 형태: "자녀를 위한 모든 희생. 자기 미래·생명까지."
      // 진입: 7일 (가족 형성은 *빠름*)
      ```

      ★ **임시 처방으로도 시스템이 작동함**: MasterDisciple로 분류하면 *후계자 지정 트리거*가
      활성화 — 수련의 마지막 행동으로 춘설병에게 *비급(=쌍도술)* 전수가 자연. 즉 *임시 매핑이
      틀린 행동을 emit하지는 않음*. 모성 차원이 시스템 표현에서 빠질 뿐.

      ★ *axes 75/90/60/25*의 의미: affinity 90이 핵심 — *모성의 정서*는 affinity로 표현됨.
      respect 60은 *어린 자에 대한 자질 인정*, trust 75는 *순수한 자녀에 대한 신뢰*, wariness 25는
      *자녀를 잃을 수 있다는 부모 본능의 약한 경계*.

      → axes는 정확히 표현됨. enum이 부족할 뿐. 이게 v0.5 한계의 정확한 진단.

  # ──────────────────────────────────────────────────
  # 7. ★ 나소호 — null + Deceased + null (짧게 알았던 자, 양녀 양육의 단서)
  # ──────────────────────────────────────────────────
  - target: "luo_xiao_hu"
    type: "옥교룡의 남편이자 춘설병의 부친 — 짧게 알았던 자"
    type_history:
      - { since: "bian_jing_meeting",   type: "옥교룡의 남편 (한 번의 만남)" }
      - { since: "yu_jiaolong_death",   type: "옥교룡 사후 어린 춘설병의 부친" }
      - { since: "nasoho_death",        type: "춘설병을 남기고 떠난 자" }
    transformation_events:
      - { event_id: "bian_jing_meeting", new_type: "옥교룡의 남편 (한 번의 만남)" }
      - { event_id: "yu_jiaolong_death", new_type: "옥교룡 사후 어린 춘설병의 부친" }
      - { event_id: "nasoho_death",      new_type: "춘설병을 남기고 떠난 자" }
    axes: { trust: 60, affinity: 30, respect: 50, wariness: 10 }
    bond_kind: null
    bond_status: "Deceased"
    partnership: null
    deceased_at: "nasoho_death"
    bond_since: null
    note: |
      ★ 짧게 알았으나 *춘설병 양녀화의 직접 사연*. axes 깊지 않으나 형식적으로 중요.
      respect 50 — *옥교룡을 사랑한 자*에 대한 약한 인정.
      affinity 30 — 가까이는 아니나 *그녀를 행복하게 한 자*에 대한 따뜻함.
      bond_kind: null + Deceased 조합. v0.5에서는 자유 텍스트 type만으로 충분.
```

#### dormant_bonds

```yaml
dormant_bonds:
  - target: "어린 시절 표국에 잠시 머물렀던 무명의 여검객"
    last_contact: "age 10~12"
    fragment: |
      "도(刀)는 사람을 *베는* 것이 아니라 *지키는* 것이다."
    note: |
      ★★ **활성화됨 (영향력만)** — first_lesson 사건에서 이 가르침이 *떠올라*
      춘설병에게 그대로 전달됨. compass의 직접 출처임을 *수련 본인이 의식한* 첫 순간.

      ★ v0.5 검증: dormant_bond는 *관계 자체가 활성화*되지 않음 (여검객은 사망/실종 추정).
      대신 *영향력이 활성화*되어 새 관계(춘설병)에 *전달*됨. dormant_bond는 *기억의 영향*과
      *기연 단서*로서 작동, 새 key_bond를 자동 생성하지 않음. 이게 v0.5의 dormant 정의 확립.

      → dormant_bond는 *현재 시점의 인스턴스에서도 dormant로 유지됨*. 다만 노년기 인스턴스의
      first_lesson transition_point에 그 영향이 명시적으로 기록됨.
```

### voice — 노년기 voice_anchors

```yaml
voice:
  speech_register: "더 침착해진 절제 — 가르치는 톤이 우세"
  vocabulary_level: "사대부와 평민 양쪽 통하는 중간 어휘 + 노년의 평이함"
  tics:
    - "'강호 사람은…' 자주, 그러나 *가르치는 어조*로"
    - "이모백 호명 회피는 더 약해짐 — 수십 년 후엔 가끔 '이 형'으로 자연스럽게"
    - "옥교룡에 대해 *과거형 + 인정의 어조* — '그 아이는 자기 길을 갔다'"
    - "춘설병에게 *짧고 분명한 가르침* — 길게 설명하지 않음"
    - "감정 흔들림은 거의 사라짐. 금비녀로 손이 가는 동작도 *드물어짐*"
  voice_anchors:
    - context: "춘설병에게 첫 쌍도술 가르침"
      utterance: |
        "춘설아, 도를 익히는 것은 누군가를 *베기* 위해서가 아니다.
         네가 이 도를 들 때마다, 먼저 *지킬 사람*의 얼굴을 떠올리거라.
         그게 무인의 첫 자리란다."
    - context: "춘설병이 어머니(옥교룡) 이야기를 물을 때"
      utterance: |
        "네 어머니는 *자기 길을 간 자유로운 사람*이었다.
         네가 그 자유를 닮되, 더 멀리 가거라.
         가두려는 자가 있다면 — 나든 누구든 — 그를 떠나도 좋다."
    - context: "유태보(평생의 우인)와 노년의 회상"
      utterance: |
        "유 형, 이제 와 보니… 우리가 함께 다닌 길이 가장 긴 길이었소.
         이 형도, 그 아이도 모두 짧게 다녀갔는데… 그대만 변함없이 옆에."
    - context: "표국 후배에게 운영 위임"
      utterance: |
        "표국은 이제 그대의 손에 맡기오. 단 하나만 — *짐을 받을 때*마다 *그 짐의 무게*가
         의뢰인의 인생 무게라는 걸 잊지 마시오. 그게 표국주의 첫 자리요."
    - context: "이모백을 회상하는 혼잣말 (춘설병 잠든 후)"
      utterance: |
        "(금비녀에 손이 살짝 갔다 내려놓으며) 이 형… 그대가 마지막에 한 말이 옳았는지
         이제는 모르겠소. 단 그대가 갔던 길은… 끝까지 옳았소."
```

### titles

```yaml
titles:
  - "쌍도여협(雙刀女俠)"           # 그대로
  - "노 협객"                     # ★ 신규
  - "춘설병의 양모"                # ★ 신규
  - "(전직: 표국주 — 후배에게 위임)"  # ★ 변화
```

## Layer 3 — 시간축

### past — transition_points (장년기에 추가되는 새 사건들)

> 장년기 인스턴스의 transition_points는 *그대로 유지*. 노년기에는 그 *후속 사건들*이 추가됨.

```yaml
transition_points:

  # (장년기 인스턴스의 transition_points 모두 유지 — 생략 표시)
  - id: "(... 장년기 인스턴스의 9개 transition_points 모두 ...)"

  # ★ 노년기 신규 사건들
  - id: "bian_jing_meeting"
    age: "40대 초~중반"
    event: |
      변경에서 옥교룡과 짧은 재회. 옥교룡은 나소호와 가족을 이루어 행복하게 살고 있음.
      수련은 *떠나옴* — 데려오지도, 가두지도 않음.
      옥교룡 status: Reactivating → Active (짧게).
    impact:
      hexaco_shifts:
        - "A+ Forgiveness: 75 → 78 (옥교룡의 선택을 인정)"
        - "C+ Prudence: 95 → 95 (변화 없음)"
      compass_change: null   # ★ compass는 이미 li_mubai_death에서 정착 — 변화 없음
    inner_resolution: "그녀가 살아있고 행복하다. 그것이 가장 좋은 결말이다."
    significance: |
      compass의 *행동 입증* — "젊은 세대를 가두지 않는다"가 *실제 행동*으로 드러남.
      Mentor 역할이 이 사건으로 *완료됨* — 더 이상 가르칠 자가 아니나 정체성은 보존.

  - id: "yu_jiaolong_death"
    age: "40대 후반"
    event: |
      옥교룡 변경에서 자연사 소식. 평온한 죽음.
      옥교룡 status: Active → Deceased.
    impact:
      hexaco_shifts:
        - "E+ Anxiety: 50 → 45 (큰 미해결 하나 종결)"
        - "E+ Sentimentality: 90 → 90 (이미 만점)"
      compass_change: null
    inner_resolution: "그녀가 자기 삶을 완성했다. 나는 다음을 본다."
    significance: |
      옥교룡의 status 전환 흐름 완결 — Reactivating → Active → Deceased.
      "다음"이 무엇인지는 아직 모름. 곧 답이 옴 (춘설병).

  - id: "nasoho_death"
    age: "50대 초반"
    event: |
      나소호도 사망 (병으로 추정). 어린 춘설병이 친척에게 맡겨질 위기.
      수련에게 후견 요청 들어옴.
    impact:
      hexaco_shifts:
        - "A+ Gentleness: 75 → 78 (어린 자녀에 대한 보호 본능)"
      compass_change: null
    inner_resolution: "옥교룡의 핏줄이다. 내가 맡는다."
    significance: "춘설병 양녀화의 직접 결정점."

  - id: "chunxue_adoption"
    age: "50대 초반"
    event: "춘설병을 정식 양녀로 들임. 새 key_bond 생성."
    impact:
      hexaco_shifts:
        - "O+ Unconventionality: 35 → 40 (양녀 양육이라는 비전통적 선택)"
        - "A+ Gentleness: 78 → 80"
      compass_change: null   # ★ compass는 *실행*이지 변화 아님
    inner_resolution: |
      "내가 살지 못한 삶을 그녀가 살게 한다. 단 *내 방식*을 강요하지 않는다."
    significance: |
      ★★ compass의 *직접 실행 사건*. 새 key_bond 생성 — 처음으로 BondKind: MasterDisciple
      (임시 처방) 진입. axes는 양육의 OCC 누적으로 빠르게 양극으로.

  - id: "first_lesson"
    age: "50대 초반~중반 (snapshot_time)"
    event: |
      춘설병에게 첫 쌍도술 가르침. 그 순간 어린 시절 무명 여검객의 가르침
      ("도는 지키는 것")이 *떠올라* 그대로 전달.
    impact:
      hexaco_shifts: []   # 큰 HEXACO 변화 없음
      compass_change: null
    inner_resolution: |
      "내가 받은 것을 내가 전한다. 강호의 시간이란 이런 것이다."
    significance: |
      ★★★ **dormant_bond 활성화 사례 (영향력만)**. 어린 시절 여검객의 가르침이 *수련 본인이
      의식한* 첫 순간. dormant_bonds[0]은 *관계로서는* 그대로 dormant 유지되나, *영향력*은
      first_lesson에서 활성화되어 춘설병에게 전달.

      ★ life_question("내가 살아온 것이 진짜 인생이었나")의 *부분적 답*: "받은 것을 전했다.
      그것이 인생이다." 단 *완전한* 답은 아님 — 의문은 여전히 가끔 떠오름.
```

### past — formative_relationships

```yaml
formative_relationships:
  - id: "father"
    type: "표국 운영자, 부친"
    legacy: "쌍도술 사사. 표국 운영의 모든 기초."

  # ★ 장년기 인스턴스의 li_mubai_past 항목은 *제거*
  # 이유: 이모백은 key_bonds[Deceased]에 *충분히* 영향이 표현됨. 노년기에는 중복 회피 명확화.

  - id: "first_master_unnamed"     # ★ 신규
    type: "어린 시절 무명 여검객 (이름 없음)"
    legacy: |
      compass의 *직접 출처*. dormant_bond에도 등록되어 있음 — *이중 등록*.
      그러나 v0.5 룰: dormant_bonds는 *기연 단서*, formative는 *과거 의미*. 둘이 같은
      인물에 대해 *다른 차원*을 표현하므로 중복 등록 정합.
      first_lesson에서 영향력 활성화됨. 자기 인식의 결정적 사건.
```

### present — unresolved_tension

```yaml
unresolved_tension:
  - id: "ut_1_chunxue_future"
    category: "관계적·책임감"
    description: |
      ★ 신규. 춘설병이 자라서 *옥교룡처럼* 자기 길을 가려 하면 어떻게 할 것인가?
      compass의 "가두지 않는다"가 시험받을 미래의 사건. tragic_seed의 직접 연결.

  - id: "ut_2_unfinished_question"
    category: "내부적·정체성"
    description: |
      ★ 변형. 장년기의 ut_1_unspoken_love가 노년에는 *부분적으로 해소*. 이제 의문은
      "내가 살아온 것이 진짜 인생이었나"의 *마지막 차원* — 양녀를 통한 답이 *충분한가*.

  - id: "ut_3_legacy"
    category: "외부적·책임"
    description: |
      ★ 신규. 표국 후계 + 쌍도술 후계 + 자기 정체성("쌍도여협") 후계.
      세 가지가 같은 인물(춘설병)에게 집중되면 가두는 게 아닌가? compass와의 미세 갈등.
```

### future hooks

```yaml
joyful_seed:
  description: |
    춘설병이 수련의 가르침을 *자기 방식으로* 발전시켜 새 세대의 협객이 됨.
    수련은 *춘설병이 떠나는 것*을 평온히 보냄. compass의 최종 완성.
  trigger_condition: |
    ut_1_chunxue_future가 *긍정적 답*을 찾을 때.
    수련의 마지막 voice_anchor — "잘 가거라" — 가 발화되는 시점이 인스턴스의 다음 snapshot_time.

tragic_seed:
  description: |
    춘설병이 옥교룡과 닮아 *체제 밖으로* 나감. 그러나 옥교룡과 달리 *길을 잃음*.
    수련의 마지막 호흡은 양녀를 *찾으러 가는* 길.
  trigger_condition: |
    ut_1_chunxue_future가 *부정적*으로 발현. 두 번째 옥교룡의 비극.
    수련의 일생이 *완성되지 못한 채* 끝나는 가능성.
```

---

# v0.5 적용 검증 결과 — 노년기 인스턴스가 시스템에 추가한 것

## 1. snapshot_time 시스템의 본격 검증

### 같은 인물의 두 인스턴스 비교

| 슬롯 | 장년기 (40대 초) | 노년기 (50대 초~중) | 변화 의미 |
|---|---|---|---|
| id | yu_shulien | yu_shulien | ★ 동일 — 같은 인물 |
| HEXACO 24 facet | 기준값 | 일부 미세 변화 (A↑, E-Anxiety↓, O-Unconv↑) | 노년의 *수용·관용* 이행 |
| inner_compass | 3 필드 + crystallization | 모두 동일 | ★ compass_change *없음* — 일관성 ✓ |
| current_state.PAD | { -0.3, 0.3, 0.6 } | { 0.1, 0.2, 0.7 } | ★★ axes 그대로 + PAD 일상 변화 |
| key_bonds 수 | 5개 | 7개 (5 기존 + 2 신규) | 일생이 추가한 관계 |
| voice_anchors | 5개 | 5개 (대부분 신규) | 같은 voice_register이지만 *발화 맥락 변화* |

**검증 결과**: `id`와 `compass`는 *변하지 않으면 안 됨*. *진짜 같은 인물의 같은 가치관*. HEXACO와 PAD와 axes는 *변할 수 있음*. snapshot_time이 정확히 이 분리를 표현.

### compass_change 일관성 검증 ✓

장년기 → 노년기 사이에 5개 transition_points 추가 (bian_jing_meeting, yu_jiaolong_death, nasoho_death, chunxue_adoption, first_lesson). 그러나 모두 `compass_change: null` — compass는 이미 li_mubai_death에서 정착됐으므로 이 사이에 변화 없음. 두 인스턴스의 compass가 동일하므로 *일관성 ✓*.

만약 노년기 compass가 달랐다면 위 5개 사건 중 *어느 하나*에 `compass_change`가 *반드시* 있어야 했을 것. v0.3 일관성 룰의 정확한 작동.

## 2. BondKind 일생 변화 검증

| 인물 | 장년기 | 노년기 | 변화 의미 |
|---|---|---|---|
| 이모백 | Soulmate + Deceased | 동일 | ★ Deceased terminal + 점착성 영구 |
| 푸른여우 | ArchRival + Resolved | 동일 | Resolved terminal + 점착성 영구 |
| **옥교룡** | Mentor + Reactivating | **Mentor + Deceased** | ★★★ status 3-stage 전환 완결 |
| 유태보 | null + Active | null + Active | axes 누적 (75/60/50/30 → 80/70/60/20) |
| 맹사조 | null + Deceased + Engaged | 동일 | Deceased + Engaged 영구 보존 |
| **춘설병** | (없음) | **MasterDisciple + Active** ★ 신규 | 일생 마지막의 핵심 관계 |
| **나소호** | (없음) | **null + Deceased** ★ 신규 | 짧게 알았던 자, 춘설병의 단서 |

★ **옥교룡 status 전환 흐름**: Reactivating { current_rumor } → Active(bian_jing_meeting) → Deceased(yu_jiaolong_death). v0.5 status enum의 진정한 검증 — 한 관계가 *세 status*를 거쳐가는 일생.

## 3. Guardian variant 한계의 명확한 노출

춘설병 관계를 v0.5의 9 BondKind variants 중 *정확히* 매핑 못함. MasterDisciple로 임시 처리했으나 respect 임계 (≥+90) 미달 — 본질적으로 양육은 압도적 존경 위에 서지 않음.

**v0.6 후보 — Guardian variant 정식 제안:**
```rust
Guardian,    // 부모-자녀형 (양육 + 보호 + 가르침. 친·양 무관)
// 임계: trust ≥+70, affinity ≥+80, respect 무관, wariness ≤30
// 자기희생 형태: "자녀를 위한 모든 희생. 자기 미래·생명까지."
// 진입: 7일 (가족 형성은 *빠름*)
// MasterDisciple과 차이: 비급 전수 *없음*, respect 임계 무관, 양육 본질
// SwornBrothers와 차이: 비대칭 (양육자가 위)
// Mentor와 차이: 가족 형식 (Mentor는 가족 무관)
```

★ 검증의 흥미로운 발견: **임시 처방으로도 시스템이 *작동함***. MasterDisciple로 두면 후계자 지정 트리거가 활성화 — 수련의 마지막 행동(쌍도술 비전 전수)이 자연. 즉 *임시 매핑이 잘못된 행동을 emit하지 않음*. 모성 차원만 시스템 표현에서 빠질 뿐. v0.5는 *부분적으로 정확*, v0.6에서 *완전*.

## 4. 양극 진입 시간 게이트의 양면 검증

| 사례 | 어떤 검증 |
|---|---|
| 춘설병 | first_lesson 후 *7일 카운트 진행 중* (MasterDisciple 30일 룰) |
| 유태보 | 약 10년의 일상 우정으로 SwornBrothers 임계 *바로 도달*, 그러나 *진입 보류* (자유 텍스트 type 채택) |

유태보 사례가 흥미로움: 임계 도달 자동 진입을 *디자이너가 강제하지 않을 수 있음*. type의 결과 BondKind 결이 다르면 enum 강제 회피가 정합. v0.5의 *재량 영역*. 이게 v0.6에서 *Companion* 같은 *덜 무거운 양극 variant* 검토 단서 (SwornBrothers의 동귀어진은 신분 차이 평민 우정에 안 맞음).

## 5. dormant_bonds 정의 확립

어린 시절 무명 여검객의 가르침이 first_lesson에서 *떠올라* 춘설병에게 전달. 그러나:

- *관계 자체*는 활성화되지 않음 (여검객 사망/실종 추정).
- *영향력*만 활성화. 새 key_bond 생성하지 않음.
- dormant_bond는 *기연 단서이자 기억의 영향력*이지 *재활성 가능 관계*가 아님.

→ **v0.5 dormant_bonds 정의 확립**: 옥교룡 같은 "예전 활성 → 휴면 → 재활성" 케이스는 `key_bonds[bond_status: Dormant/Reactivating]`. 무명 여검객 같은 "한 번도 활성된 적 없는 영향력"은 `dormant_bonds`. 둘은 시스템 동작이 *명확히 다름*.

## 6. PAD vs axes 분리의 명확한 시연

| 인물 | 장년기 axes | 노년기 axes | PAD 변화 |
|---|---|---|---|
| 이모백 | 95/95/95/5 | **동일** | 회상 OCC 강도 약화 |
| 맹사조 | 80/70/75/0 | **동일** | 회상 빈도 ↓ |
| 푸른여우 | -70/-90/70/90 | **동일** | 거의 떠올리지 않음 |

axes는 *정체성*이라 freeze. PAD는 *현재 일상*이라 변화. 같은 axes가 다른 PAD로 표현됨. v0.5 §1.4 (axes 점착성) + §4.5 (회상 OCC) 결합 작동.

## 7. v0.5 시스템의 노년기 표현력 — 결론

노년기 수련 인스턴스가 *왜곡 없이* 표현됨:

- **시간의 누적** → BondKind 영구 보존, status 전환 흐름, axes 점착성으로 표현
- **관계의 마지막** → Deceased + Resolved의 terminal 룰로 *닫힌 챕터*들이 정체성 층 형성
- **새로운 시작** → Active 신규 key_bond (춘설병)가 일생 마지막 핵심 관계
- **부분적으로 풀린 의문** → unresolved_tension 변형 + life_question 안정

가장 강한 단일 검증: **이모백 axes (95/95/95/5)가 약 13~17년간 그대로 유지**되면서 PAD는 변하고 type에 "오랜 사별의 인연" 추가됨. 점착성·terminal·자유 텍스트가 *시간의 깊이*를 시스템에 새기는 메커니즘.

가장 명확한 한계: **Guardian variant 부재**. 춘설병 관계가 일생 마지막의 핵심인데도 enum이 그것을 정확히 표현 못함. v0.6 1순위.

## 8. v0.6 후보 우선순위 (임충 + 장년기 수련 + 노년기 수련 합치)

1. **★ Guardian variant** — 노년기 수련에서 명확 노출. *모든 게임에 자녀 양육이 등장*하므로 보편적 필요.
2. **ActionTriggerEvaluator** — 임충에서 노출 (BondKind 분류 vs 실행 가능성).
3. **회상 OCC 메커니즘 구체화** — 장년기·노년기 수련에서 골격만 사용 (§4.5). 강도 계산·트리거 조건 미설계.
4. **Companion variant (또는 Friend)** — 노년기 유태보 사례. SwornBrothers보다 *덜 무거운* 양극.
5. **Mentee variant** — BondKind 비대칭. 옥교룡 → 수련 방향 (장년기 인스턴스에서 noted).
6. **compass 변화 후 axes 재평가** — 여전히 명시 룰 부재. 다음 인스턴스에서 모순 발생 시 명시.

---

# 임충 + 장년기 수련 + 노년기 수련 — 3 인스턴스 합치 커버리지

| 차원 | variants | 커버 |
|---|---|---|
| BondKind | 9 | **8/9** (MasterDisciple만 미검증 — 황약사·곽정 같은 *전형적* 사부-제자 후보) |
| BondStatus | 5 | **5/5 ✓** |
| Partnership | 4 | **2/4** (Engaged·Separated. Spouse·Lover 미검증) |
| snapshot_time | 분리 | **2 시점 같은 인물** (장년기 ↔ 노년기 수련) ★ |
| compass_change | 일관성 | ✓ (장년기 → 노년기 변화 없음 = 일관) |
| axes 음수 | 검증 | ✓ (육겸 -100, 푸른여우 등) |
| 점착성 (±100) | 검증 | ✓ (이모백 95/95/95/5 13~17년 유지) |
| Deceased terminal | 검증 | ✓ (이모백·맹사조·옥교룡) |
| Resolved terminal | 검증 | ✓ (육겸·푸른여우) |
| Reactivating → Active → Deceased | 검증 | ✓ (옥교룡 일생) |
| dormant 영향력 활성화 | 검증 | ✓ (무명 여검객 → first_lesson) |

**3 인스턴스로 v0.5 시스템의 거의 모든 메커니즘 검증 완료.** v0.6 보정 시점이 도래.

---

## 변경 이력

| 버전 | 일자 | 변경 |
|------|------|------|
| v1.0 | 2026-05-04 | 초안. 노년기 수련 (50대 초~중) 인스턴스. snapshot_time 시스템의 본격 검증 — 같은 인물 두 시점 인스턴스 합치. BondKind 일생 변화 (옥교룡 status 3-stage 전환), 신규 key_bond (춘설병·나소호), Guardian variant 한계 명확 노출, dormant 영향력 활성화 메커니즘 확립. v0.6 보정 우선순위 도출. |
