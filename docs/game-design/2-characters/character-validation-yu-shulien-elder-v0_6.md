# 인물 스키마 v0.6 검증 — 노년기 유수련 (兪秀蓮)

> 작성일: 2026-05-04
> 검증 대상: `_schema.md` v0.6 + `relationships.md` v0.6 + `action_triggers.md` v0.1
> 위치: `docs/game-design/2-characters/character-validation-yu-shulien-elder-v0_6.md`
> 이전 버전: v0.5 (`character-validation-yu-shulien-elder-v0_5.md` 폐기)
> 동반 인스턴스:
>   - 장년기 수련: `character-validation-yu-shulien-v0_6.md`
>   - 임충: `character-validation-lin-chong-v0_6.md`

## v0.6 핵심 변경

이 인스턴스는 v0.6의 *모든 신설 항목*을 직접 사용:

| v0.6 신설 | v0.5 처리 | v0.6 갱신 |
|---|---|---|
| **Companion variant** | 유태보 — 자유 텍스트 type, 임계 도달했으나 enum 부재 | **`bond_kind: Companion`** ★ 정식 분류 |
| **Guardian variant** | 춘설병 — MasterDisciple 임시 처방 (respect 임계 미달) | **`bond_kind: Guardian`** ★ 정식 분류 |
| **회상 OCC 구체화** | voice anchor에 동작 묘사만 ("금비녀에 손이 갔다") | 회상 트리거·강도·PAD 영향 *시스템적 도출* |
| **ActionTriggerEvaluator** | "compass대로 떠나옴" 직관적 서술 | feasibility 5차원 평가의 *시스템 도출* |
| **compass 자연 누적 룰** | 명시 안됨 | tp_chunxue_adoption 등이 compass_change 없음을 *명시적*으로 정합 |

이 인스턴스는 v0.6 시스템의 *완전한 검증*. 임시 처방·직관 서술이 모두 *시스템 메커니즘*으로 환원.

---

# 유수련(兪秀蓮) — 노년기 인스턴스

## Layer 1 — 본바탕

### identity

```yaml
id: "yu_shulien"          # 장년기 인스턴스와 동일
name: "유수련(兪秀蓮)"
nicknames:
  - "쌍도여협(雙刀女俠)"
  - "표국의 옛 여주인"
  - "춘설병의 양모"
  - "노 협객"
era: "청대 (왕도려 학철오부곡 세계관, 철기은병 시기)"
stage_of_life: "노년기 진입"
snapshot_time: |
  수련 50대 초반~중반. 변경에서 옥교룡과 짧은 재회 후 약 5~6년.
  옥교룡 사망 후 약 3~4년. 나소호 사망 후 약 1년. 춘설병을 양녀로 들이고 1~2년.
  현재: 춘설병에게 첫 쌍도술을 가르치기 시작한 시점.
```

### origin

```yaml
# 장년기와 동일
birthplace: "북경"
social_origin: "양민 → 표국주(運局主) → 노년 위임"
kingdom_of_origin: "청"
family_background: |
  명문가는 아니나 양민 중 *기예 있는 집안*. 부친이 북경에서 표국 운영.
```

### temperament — HEXACO 24 facet

> 노년기 미세 변화. 변화 위치 표시.

```yaml
H_honesty_humility:
  sincerity: 90
  fairness: 90
  greed_avoidance: 90       # ↑ 85 → 90
  modesty: 85
E_emotionality:
  fearfulness: 25
  anxiety: 40               # ↓ 50 → 40
  dependence: 35
  sentimentality: 90
X_extraversion:
  social_self_esteem: 75    # ↑ 70 → 75
  social_boldness: 60
  sociability: 55           # ↓ 60 → 55
  liveliness: 35            # ↓ 40 → 35
A_agreeableness:
  forgiveness: 80           # ↑ 75 → 80
  gentleness: 80            # ↑ 75 → 80
  flexibility: 70
  patience: 95
C_conscientiousness:
  organization: 90
  diligence: 85             # ↓ 90 → 85
  perfectionism: 75         # ↓ 80 → 75
  prudence: 95
O_openness:
  aesthetic_appreciation: 70 # ↑ 65 → 70
  inquisitiveness: 55        # ↓ 60 → 55
  creativity: 65
  unconventionality: 40      # ↑ 35 → 40 (양녀 양육이라는 비전통적 결정)
```

### body

```yaml
physical_description: |
  50대 초반~중반. 단정한 얼굴에 깊은 주름. 차분한 눈매에 *수용된 슬픔*이 어림. 머리는 반백.
signature_feature: |
  **금비녀(金釵)** — 변하지 않음. 평생의 정신적 족쇄.
  단 노년에는 손이 금비녀로 향하는 동작이 *덜 빈번*. 감정 흔들림 줄어든 결과.
  쌍도는 평소 가르침용 한 자루만 휴대. 다른 한 자루는 춘설병에게 증여.
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

> **장년기와 동일.** 이는 v0.6 §1.5 *compass 자연 누적 룰*의 검증: 노년기 사이에 5개 transition_points 추가됨에도 compass_change 없음 — compass는 li_mubai_death에서 정착됐고 이후 사건들은 *compass의 실행*이지 변화가 아님. 일관성 ✓.

### current_state

```yaml
current_state:
  pad:
    pleasure:  0.1     # 장년기 -0.3 → 0.1
    arousal:   0.2     # 장년기 0.3 → 0.2
    dominance: 0.7     # 장년기 0.6 → 0.7
  dominant_emotion: "Maternal Affection + Lingering Sorrow"
  active_focus: "춘설병의 첫 쌍도술 — 어떻게 가르칠 것인가"
```

> ★★ **axes 점착성 vs PAD 변화의 정확한 분리 검증.** 같은 인물의 같은 axes (이모백 95/95/95/5는 그대로) — 그러나 PAD는 *양수로 회복*. 양녀와의 일상이 PAD를 끌어올림.

### relationships

#### key_bonds — 7개 (★ 2개 v0.6 변경)

```yaml
key_bonds:

  # ──────────────────────────────────────────────────
  # 1. 이모백 — Soulmate + Deceased + null (변화 없음)
  # ──────────────────────────────────────────────────
  - target: "li_mubai"
    type: "영원히 미완의 사랑 — 오랜 사별의 인연"
    type_history:
      - { since: "맹사조 사망 전",          type: "약혼자의 의형제" }
      - { since: "맹사조 사망 후",          type: "지기 + 잠재 연인" }
      - { since: "qingming_jian_stolen",  type: "함께 싸우는 동지" }
      - { since: "li_mubai_death",        type: "영원히 미완의 사랑" }
      - { since: "장년기 ~ 노년기 사이",     type: "오랜 사별의 인연" }
    transformation_events:
      - { event_id: "li_mubai_death", new_type: "영원히 미완의 사랑" }
    axes: { trust: 95, affinity: 95, respect: 95, wariness: 5 }
    bond_kind: "Soulmate"
    bond_status: "Deceased"
    partnership: null
    deceased_at: "li_mubai_death"
    bond_since: "맹사조 사망 후 약 5년"
    note: |
      ★ Deceased terminal + 양극 점착성 결합 검증. 사후 약 13~17년 axes 동일 유지.
      v0.6 §4.5 회상 OCC 적용 검증 — §검증 결과의 9.1 사례 참조.

  # ──────────────────────────────────────────────────
  # 2. 푸른여우 — ArchRival + Resolved + null (변화 없음)
  # ──────────────────────────────────────────────────
  - target: "bi_yan_huli"
    type: "결판된 적 — 거의 잊혀진 자"
    type_history:
      - { since: "이모백 사부 살해 사건",     type: "이모백의 사부의 원수" }
      - { since: "li_mubai_death",        type: "이모백을 죽인 직접 가해자 → 결판된 적" }
      - { since: "장년기 ~ 노년기 사이",     type: "결판된 적 — 거의 잊혀진 자" }
    transformation_events:
      - { event_id: "li_mubai_death", new_type: "이모백을 죽인 직접 가해자 → 결판된 적" }
    axes: { trust: -70, affinity: -90, respect: 70, wariness: 90 }
    bond_kind: "ArchRival"
    bond_status: { Resolved: { reason: "이모백의 복수로 처단" } }
    partnership: null
    bond_since: "이모백 사부 살해 사건"
    note: |
      Resolved terminal + 점착성. 회상 OCC 강도 매우 낮음 — 거의 떠올리지 않음.

  # ──────────────────────────────────────────────────
  # 3. 옥교룡 — Mentor + Deceased + null (변화 없음)
  # ──────────────────────────────────────────────────
  - target: "yu_jiaolong"
    type: "가르치려 했으나 자기 길을 간 후배 — 변경에서 *살다 간* 자"
    type_history:
      - { since: "북경 첫 만남",             type: "표국 손님 (가짜 신분)" }
      - { since: "qingming_jian_stolen",   type: "청명검 도둑·적대" }
      - { since: "shulien_advice",         type: "가르치려 했으나 듣지 않는 후배" }
      - { since: "wudang_mountain_fall",   type: "행방불명" }
      - { since: "current_rumor",          type: "변경에 살아있다는 단서" }
      - { since: "bian_jing_meeting",      type: "변경에서 살아있는 후배 (재회)" }
      - { since: "yu_jiaolong_death",      type: "변경에서 살다 간 자" }
    transformation_events:
      - { event_id: "shulien_advice",       new_type: "가르치려 했으나 듣지 않는 후배" }
      - { event_id: "wudang_mountain_fall", new_type: "행방불명" }
      - { event_id: "current_rumor",        new_type: "변경에 살아있다는 단서" }
      - { event_id: "bian_jing_meeting",    new_type: "변경에서 살아있는 후배 (재회)" }
      - { event_id: "yu_jiaolong_death",    new_type: "변경에서 살다 간 자" }
    axes: { trust: 65, affinity: 80, respect: 80, wariness: 35 }
    bond_kind: "Mentor"
    bond_status: "Deceased"
    partnership: null
    deceased_at: "yu_jiaolong_death"
    bond_since: "shulien_advice 후 14일"
    note: |
      ★ status 3-stage 전환 완결: Reactivating → Active(짧게) → Deceased.
      v0.6 ActionTriggerEvaluator 검증 — bian_jing_meeting 시점에 OfferGuidance가 차단되어
      WatchOver(떠나옴)으로 변형됨. §검증 결과의 9.4 사례 참조.

  # ──────────────────────────────────────────────────
  # 4. ★ 유태보 — null → Companion ★ v0.6 정식 분류
  # ──────────────────────────────────────────────────
  - target: "liu_taibao"
    type: "북경 시정의 의리 있는 친구 — 평생의 우인"
    type_history:
      - { since: "와호장룡 시기",     type: "정보원 + 동행자" }
      - { since: "이모백 사후",       type: "북경 시정의 의리 있는 친구" }
      - { since: "노년기",           type: "평생의 우인" }
    transformation_events:
      - { event_id: "qingming_jian_stolen", new_type: "정보원 + 동행자" }
    axes: { trust: 80, affinity: 70, respect: 60, wariness: 20 }
    bond_kind: "Companion"        # ★★★ v0.6 — 정식 분류
    bond_status: "Active"
    partnership: null
    bond_since: "약 30년 일상 우정 누적, 노년기 시점에 자연 진입"
    note: |
      ★★★ **v0.6 Companion variant 첫 정식 적용.**
      v0.5에서 SwornBrothers 임계 *근접*하나 *형제 결*과 다른 평민 우정으로 자유 텍스트 type만 채택했음.
      v0.6 Companion 임계 (trust ≥+75, affinity ≥+65, respect ≥+50, wariness ≤30) 모두 충족 ✓.

      **SwornBrothers와의 결정적 차이:**
      - 자기희생 형태가 *동귀어진 없음*. 깊은 신뢰는 있되 *생사를 함께*는 아님.
      - 신분 차이를 가로지르는 평민 우정의 본질에 정합.
      - 임계 충족 + 30일 카운트는 약 30년 누적으로 자연스럽게.

      **v0.5 → v0.6 시스템 발전의 명확한 사례.** 직관으로 "Companion이 필요하다"고 한 것이 v0.6에서
      enum으로 정식 분류됨.

  # ──────────────────────────────────────────────────
  # 5. 맹사조 — null + Deceased + Engaged (변화 없음)
  # ──────────────────────────────────────────────────
  - target: "meng_sizhao"
    type: "죽은 약혼자 — 평생 정절의 정표 (시간이 흘러 흐려진 얼굴)"
    type_history:
      - { since: "정혼 무렵",     type: "약혼자" }
      - { since: "사망 후",       type: "죽은 약혼자 — 평생 정절의 정표" }
      - { since: "노년기",        type: "흐려진 얼굴이지만 살아있는 정표" }
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
      Partnership: Engaged의 영구성 검증 — 약 30년 전 정혼이 노년에도 그대로 보존.
      금비녀가 정체성의 핵심. 회상 OCC 강도 약함 (시간 깊어 PAD 동요 작음).

  # ──────────────────────────────────────────────────
  # 6. ★ 춘설병 — MasterDisciple → Guardian ★ v0.6 정식 분류
  # ──────────────────────────────────────────────────
  - target: "chun_xue_bing"
    type: "양녀이자 후계자 — 옥교룡의 핏줄, 내가 가르치는 다음 세대"
    type_history:
      - { since: "nasoho_death",        type: "맡겨진 옥교룡의 딸" }
      - { since: "chunxue_adoption",    type: "양녀" }
      - { since: "first_lesson",        type: "양녀이자 후계자" }
    transformation_events:
      - { event_id: "chunxue_adoption", new_type: "양녀" }
      - { event_id: "first_lesson",     new_type: "양녀이자 후계자" }
    axes: { trust: 75, affinity: 90, respect: 60, wariness: 25 }
    bond_kind: "Guardian"            # ★★★ v0.6 — 정식 분류
    bond_status: "Active"
    partnership: null
    bond_since: "chunxue_adoption 후 7일 도달 시점"   # ★ Guardian 진입 7일 게이트
    note: |
      ★★★ **v0.6 Guardian variant 첫 정식 적용.**
      v0.5에서 MasterDisciple 임시 처방으로 처리. respect 임계 (≥+90) *미달*이 명백한 한계였음.

      v0.6 Guardian 임계 검증:
      - trust 75 ≥+70 ✓
      - affinity 90 ≥+80 ✓ (★ 핵심 — 모성·부성의 정서 깊이)
      - respect 무관 ✓ (어린 자녀에 대한 압도적 존경 부재 자연)
      - wariness 25 ≤30 ✓
      - 진입 7일 게이트: chunxue_adoption 후 7일 — 가족 형성의 빠른 시간

      **MasterDisciple과의 결정적 차이:**
      - 비급 전수가 *부수적*. 양육이 본질.
      - respect 무관 — 자녀 자질 인정으로 충분.
      - 자기희생 형태가 *비급 전수*가 아닌 *자녀를 위한 모든 희생*.

      **임시 처방의 *대부분 옳음*도 검증.** v0.5에서 MasterDisciple로 처리해도 *후계자 지정 트리거*는
      활성화되어 쌍도술 전수 행동이 자연스럽게 도출됐음. v0.6은 *모성 차원*까지 표현 — 시스템 정교화.

  # ──────────────────────────────────────────────────
  # 7. 나소호 — null + Deceased + null (변화 없음)
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
      짧게 알았으나 *춘설병 양녀화의 직접 사연*. 자유 텍스트 type만으로 충분한 케이스.
```

#### dormant_bonds

```yaml
dormant_bonds:
  - target: "어린 시절 표국에 잠시 머물렀던 무명의 여검객"
    last_contact: "age 10~12"
    fragment: |
      "도(刀)는 사람을 *베는* 것이 아니라 *지키는* 것이다."
    note: |
      ★ first_lesson 사건에서 *영향력 활성화* (관계 자체는 dormant 유지).
      compass의 직접 출처임을 *수련 본인이 의식한* 첫 순간.
```

### voice — 노년기

```yaml
voice:
  speech_register: "더 침착해진 절제 — 가르치는 톤이 우세"
  vocabulary_level: "사대부와 평민 양쪽 통하는 중간 어휘 + 노년의 평이함"
  tics:
    - "'강호 사람은…' 자주, *가르치는 어조*로"
    - "이모백 호명 회피 약해짐 — 가끔 '이 형'으로 자연스럽게"
    - "옥교룡에 대해 *과거형 + 인정의 어조*"
    - "춘설병에게 *짧고 분명한 가르침*"
    - "감정 흔들림 거의 사라짐. 금비녀로 손이 가는 동작도 *드물어짐*"
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
    - context: "이모백 기일, 금비녀를 손에 쥠 (v0.6 회상 OCC 작동)"
      utterance: |
        "(기일 아침 — 금비녀를 잠시 손에 쥐고) 이 형… 그대가 마지막에 한 말이 옳았는지
         이제는 모르겠소. 단 그대가 갔던 길은… 끝까지 옳았소."
    - context: "표국 후배에게 운영 위임"
      utterance: |
        "표국은 이제 그대의 손에 맡기오. 단 하나만 — *짐을 받을 때*마다 *그 짐의 무게*가
         의뢰인의 인생 무게라는 걸 잊지 마시오."
```

### titles

```yaml
titles:
  - "쌍도여협(雙刀女俠)"
  - "노 협객"
  - "춘설병의 양모"
  - "(전직: 표국주 — 후배에게 위임)"
```

## Layer 3 — 시간축

### past — transition_points (장년기 인스턴스의 9개 + 노년기 신규 5개)

> 장년기 9개 transition_points는 그대로 유지 (생략 표시).

```yaml
transition_points:
  - id: "(... 장년기 인스턴스의 9개 transition_points 모두 ...)"

  # ★ 노년기 신규 5개
  - id: "bian_jing_meeting"
    age: "40대 초~중반"
    event: |
      변경에서 옥교룡과 짧은 재회. 옥교룡은 나소호와 가족을 이루어 행복하게 살고 있음.
      수련은 *떠나옴*. 옥교룡 status: Reactivating → Active → (다시 거리감).
    impact:
      hexaco_shifts:
        - "A+ Forgiveness: 75 → 78"
      compass_change: null   # ★ compass는 이미 정착, 변화 없음 (자연 누적 룰)
    inner_resolution: "그녀가 살아있고 행복하다. 그것이 가장 좋은 결말이다."
    significance: |
      compass의 *행동 입증*. v0.6 ActionTrigger §9.4 — OfferGuidance → WatchOver 변형.

  - id: "yu_jiaolong_death"
    age: "40대 후반"
    event: "옥교룡 변경에서 자연사. 옥교룡 status: Active → Deceased."
    impact:
      hexaco_shifts:
        - "E+ Anxiety: 50 → 45"
      compass_change: null
    inner_resolution: "그녀가 자기 삶을 완성했다."
    significance: "옥교룡 status 전환 흐름 완결."

  - id: "nasoho_death"
    age: "50대 초반"
    event: "나소호도 사망. 어린 춘설병 후견 요청 들어옴."
    impact:
      hexaco_shifts:
        - "A+ Gentleness: 75 → 78"
      compass_change: null
    inner_resolution: "옥교룡의 핏줄이다. 내가 맡는다."
    significance: "춘설병 양녀화의 결정점."

  - id: "chunxue_adoption"
    age: "50대 초반"
    event: "춘설병을 정식 양녀로 들임. 새 key_bond 생성. 7일 후 Guardian 진입."
    impact:
      hexaco_shifts:
        - "O+ Unconventionality: 35 → 40"
        - "A+ Gentleness: 78 → 80"
      compass_change: null   # ★ compass의 *실행*이지 변화 아님
    inner_resolution: |
      "내가 살지 못한 삶을 그녀가 살게 한다. 단 *내 방식*을 강요하지 않는다."
    significance: |
      ★★ compass의 *직접 실행 사건*. v0.6 Guardian variant 진입.
      compass_change null이 v0.6 §1.5 자연 누적 룰의 정합 입증.

  - id: "first_lesson"
    age: "50대 초반~중반 (snapshot_time)"
    event: |
      춘설병에게 첫 쌍도술 가르침. dormant_bonds[0]의 어린 시절 가르침이 *떠올라* 그대로 전달.
    impact:
      hexaco_shifts: []
      compass_change: null
    inner_resolution: "내가 받은 것을 내가 전한다. 강호의 시간이란 이런 것이다."
    significance: |
      ★★★ dormant_bond 영향력 활성화 사례. 무명 여검객의 가르침이 *수련 본인이 의식한* 첫 순간.
      life_question의 *부분적 답*: "받은 것을 전했다. 그것이 인생이다."
```

### past — formative_relationships

```yaml
formative_relationships:
  - id: "father"
    type: "표국 운영자, 부친"
    legacy: "쌍도술 사사. 표국 운영의 모든 기초."

  - id: "first_master_unnamed"
    type: "어린 시절 무명 여검객 (이름 없음)"
    legacy: |
      compass의 직접 출처. dormant_bond에도 등록.
      first_lesson에서 영향력 활성화. 자기 인식의 결정적 사건.
```

> v0.5의 li_mubai_past 항목 *제거 유지* (key_bonds[Deceased]에 충분히 영향 표현).

### present — unresolved_tension

```yaml
unresolved_tension:
  - id: "ut_1_chunxue_future"
    category: "관계적·책임감"
    description: |
      춘설병이 자라서 *옥교룡처럼* 자기 길을 가려 하면 어떻게 할 것인가?
      compass의 "가두지 않는다"가 시험받을 미래.

  - id: "ut_2_unfinished_question"
    category: "내부적·정체성"
    description: |
      "내가 살아온 것이 진짜 인생이었나"의 *마지막 차원* — 양녀를 통한 답이 *충분한가*.

  - id: "ut_3_legacy"
    category: "외부적·책임"
    description: |
      표국 후계 + 쌍도술 후계 + 정체성 후계. 세 가지가 같은 인물(춘설병)에 집중되면
      가두는 게 아닌가? compass와의 미세 갈등.
```

### future hooks

```yaml
joyful_seed:
  description: |
    춘설병이 수련의 가르침을 *자기 방식으로* 발전시켜 새 세대 협객이 됨.
    수련은 *춘설병이 떠나는 것*을 평온히 보냄. compass의 최종 완성.
  trigger_condition: "ut_1_chunxue_future가 *긍정적 답*을 찾을 때."

tragic_seed:
  description: |
    춘설병이 옥교룡과 닮아 *체제 밖으로* 나감. 그러나 옥교룡과 달리 *길을 잃음*.
    수련의 마지막 호흡은 양녀를 *찾으러 가는* 길.
  trigger_condition: "ut_1_chunxue_future가 *부정적*으로 발현."
```

---

# v0.6 검증 결과

## 1. BondKind 11종 매핑 (v0.6 기준)

| 인물 | bond_kind | bond_status | partnership | v0.5 → v0.6 변경 |
|---|---|---|---|---|
| 이모백 | Soulmate | Deceased | null | 변화 없음 |
| 푸른여우 | ArchRival | Resolved | null | 변화 없음 |
| 옥교룡 | Mentor | Deceased | null | 변화 없음 |
| **유태보** | **Companion** ★ | Active | null | **null → Companion** |
| 맹사조 | null | Deceased | Engaged | 변화 없음 |
| **춘설병** | **Guardian** ★ | Active | null | **MasterDisciple(임시) → Guardian** |
| 나소호 | null | Deceased | null | 변화 없음 |

★ = v0.6 신설 variant 정식 적용.

## 2. v0.6 회상 OCC 메커니즘 검증

### 9.1 이모백 평일 회상 — 옛 객점 풍경

```yaml
trigger: { EnvironmentalCue: { cue: "옛 객점", similarity: 0.7 } }
계산:
  - base_strength: 0.7
  - bond_depth: 1.0 (Soulmate)
  - axes_magnitude: (95+95+95+5)/4/100 = 0.725
  - time_decay: 1.0 / (1.0 + 15*0.1) = 0.4
  - sentimentality: 0.9
  - 최종: 0.7 × 1.0 × 0.725 × 0.4 × (0.5 + 0.9*0.5) = 0.193
결과:
  - PAD 영향: pleasure -0.2, arousal -0.1, dominance -0.1
  - duration_days: 3일
  - triggers_action: None (0.5 미달, HandleHeirloom 약한 후보로만 등록)
```

### 9.2 이모백 기일 회상

```yaml
trigger: { SignificantDate: { kind: "기일", days_since_event: 0 } }
계산:
  - base_strength: 1.0 (정확한 기일)
  - 다른 요소 동일
  - 최종: 1.0 × 1.0 × 0.725 × 0.4 × 0.95 = 0.275
결과:
  - PAD 영향: 더 강함
  - duration_days: 7일
  - triggers_action: HandleHeirloom 등록 (0.5 가까움 — voice anchor의 "금비녀를 손에 쥠"이 정확히 이 결과)
```

> ★★★ **voice anchor가 시스템에서 도출됨.** v0.5에서 *디자이너가 작성*한 "(금비녀에 손이 갔다 내려놓으며)" 묘사가 v0.6에서 *시스템 평가의 자연 결과*로 환원. 디자인 직관 → 시스템 메커니즘.

## 3. v0.6 ActionTriggerEvaluator 검증

### 9.3 노년 수련 → 춘설병 (Guardian Active)

```yaml
입력:
  bond_kind: Guardian, bond_status: Active
  scene: 첫 쌍도술 가르침 시점
출력 후보:
  1. GuardianProtect(춘설병) feasibility 0.92 urgency 0.5
  2. BequestLegacy(춘설병) feasibility 0.95 urgency 0.4 (쌍도술 비전 전수)
실제 행동: BequestLegacy + GuardianProtect 병행 (가르침 + 보호 본능)
```

### 9.4 노년 수련 → 옥교룡 (변경 재회 시점, Mentor Active)

```yaml
입력:
  bond_kind: Mentor, bond_status: Active (재회 직후)
  scene: 변경 옥교룡의 가정. 행복 상태.
출력 후보:
  1. OfferGuidance(옥교룡) moral_alignment 0.3 (compass "가두지 않는다"와 충돌) → blocked
     → deferred to WatchOver
  2. (deferred) WatchOver(옥교룡) feasibility 0.85 urgency 0.4
실제 행동: WatchOver — 떠나옴
```

> ★★★ 수련의 *떠나옴*이 시스템에서 도출. v0.5에서는 디자이너 직관 ("compass대로 떠난다")이었던 것이 v0.6에서 5차원 feasibility 평가의 *자연 결과*.

## 4. v0.6 시스템 발전의 누적 검증

| 디자인 영역 | v0.5 처리 | v0.6 시스템 표현 |
|---|---|---|
| 평민 우정 (유태보) | 자유 텍스트 type | Companion enum |
| 양육 관계 (춘설병) | MasterDisciple 임시 + respect 미달 | Guardian enum + 임계 정합 |
| 회상 동작 (금비녀) | voice anchor 묘사 | 회상 OCC 강도 계산 + HandleHeirloom |
| 떠나옴 (옥교룡 변경) | inner_resolution 서술 | OfferGuidance blocked → WatchOver 변형 |
| compass 변화 후 axes | 명시 부재 | 자연 누적 룰 §1.5 |

이전엔 *디자이너가 직관적으로 작성*했던 인물의 행동·동작·결정이 *시스템 메커니즘에서 자연 도출*되는 단계로 발전. **인스턴스 작성이 점점 *디자이너 작업 ↓ + 시스템 자동화 ↑*** 방향으로 이동.

## 5. v0.7 후보 (남은 한계)

1. **Mentee variant** — 옥교룡 → 수련 방향. 가르침을 거부한 자도 멘티 카테고리?
2. **Beloved/LifePartner variant** — 임충-장씨, 부부형 동반의 정식 분류 (수련-이모백은 Soulmate로 충분).
3. **friendship vs romantic distinction** — Companion이 너무 광범위. 친구·동지·우인 등 세분화 필요?
4. **NPC AI Layer** — ActionTriggerEvaluator가 *후보*를 emit, *선택*은 누가? LLM? 결정론?
5. **다중 NPC 협공·집단 봉기** — 같은 대상에 여러 NPC가 동시 ActionTrigger.

---

## 변경 이력

| 버전 | 일자 | 변경 |
|------|------|------|
| v1.0 (v0.4 스키마) | 2026-05-04 | 초안 |
| v2.0 (v0.5 스키마) | 2026-05-04 | bond_status 5종 모두 사용. Mentor + Reactivating. |
| v3.0 (v0.6 스키마) | 2026-05-04 | **Companion·Guardian 정식 적용** (유태보·춘설병). 회상 OCC 강도 계산 + ActionTrigger 5차원 feasibility 검증. v0.6 시스템 발전의 *완전한* 누적 검증. |
