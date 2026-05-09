# 인물 스키마 v0.6 검증 — 곽정 (郭靖)

> 작성일: 2026-05-04
> 검증 대상: `_schema.md` v0.6 + `relationships.md` v0.6 + `action_triggers.md` v0.1
> 위치: `docs/game-design/2-characters/character-validation-guo-jing-v0_6.md`
> 동반 인스턴스:
>   - 임충: `character-validation-lin-chong-v0_6.md`
>   - 장년기 수련: `character-validation-yu-shulien-v0_6.md`
>   - 노년기 수련: `character-validation-yu-shulien-elder-v0_6.md`

## 검증 목적 — v0.6 시스템 검증의 완결

곽정 인스턴스는 **단일 인물에서 BondKind 11종 중 9종을 직접 사용**. 임충·수련 두 인스턴스 합치로도 검증되지 않은 종류들이 곽정에서 정착:

| BondKind | 임충 | 장년 수련 | 노년 수련 | **곽정** ★ |
|---|---|---|---|---|
| **SwornBrothers** | (임계 근접만) | — | — | **✓ 주백통** |
| **MasterDisciple** | — | — | — | **✓ 강남7괴·홍칠공·가진악 (다중)** |
| **Soulmate + Spouse** | — | (Soulmate + null) | (Soulmate + Deceased) | **✓ 황용 (양극 + Spouse)** |
| LoyalRetainer | — | — | — | (§5.1 검증) |
| Companion | — | — | ✓ | — |
| Guardian | — | — | ✓ | **✓ 곽부·곽양 (친자녀)** |
| Mentor | (불완전) | (Reactivating) | (Deceased) | **✓ 양과·칭기즈칸** |
| BloodEnemy | ✓ | — | — | — |
| ArchRival | — | — | ✓ | **✓ 구양봉** |
| Betrayer | ✓ | — | — | **✓ 양강** |
| Oppressor | ✓ | — | — | — |

→ **11종 모두 검증 완료.** v0.6 BondKind 시스템의 완전한 시연.

추가 검증:
- ★★★ **Partnership: Spouse 첫 검증** (곽정-황용)
- ★★★ **MasterDisciple 다중 사부** (한 인물이 여러 명을 모두 MasterDisciple로 — 강남7괴 + 홍칠공 + 가진악)
- ★★★ **친자녀 Guardian** (노년 수련-춘설병 양녀와 다른 결)
- ★★ **의형제 형식 한계 재확인** (양강과의 의형제 맹세를 형식 차원으로 표현하지 못함)
- ★★ **장인 관계 한계 노출** (황약사 — Partnership도 BondKind도 매핑 안 됨)

---

# 곽정(郭靖)

## Layer 1 — 본바탕

### identity

```yaml
id: "guo_jing"
name: "곽정(郭靖)"
nicknames:
  - "북협(北俠)"            # 신조협려 시점
  - "곽 대협"
  - "양양의 수호자"
  - "(과거: 사조 영웅, 안다)"
era: "남송 (몽골 침공기)"
stage_of_life: "장년기 후반"
snapshot_time: |
  ★ 신조협려 후반, 양양성 수성 중. 50대 초반.
  양과·소용녀 결혼 후 떠난 직후. 곽부 결혼(야율제) 직전. 곽양 청소년기 (16세).
  강남7괴 학살 사건 후 약 30년. 홍칠공 사망 후 수년. 황약사 방랑 중.
  현재: 양양성 군의(軍議) 후 잠시 가족과 머무는 시기. 다음 몽골 공세를 준비.
```

### origin

```yaml
birthplace: "남송 임안부 (어머니가 이주한 후 몽골 초원에서 출생)"
social_origin: "유민 → 몽골 의자(義子) → 남송 협객"
kingdom_of_origin: "남송 (정체성) / 몽골 (성장지)"
family_background: |
  부친 곽소천(郭嘯天)은 양철심(楊鐵心)과 의형제. 송 휘종 시기 황실 박해를 피해 우가촌으로 도주.
  단천덕에 의해 가문 멸문. 임신 중인 어머니 이평(李萍)이 몽골 초원으로 도피, 거기서 곽정 출생.
  몽골에서 자라며 칭기즈칸의 휘하에서 의자(義子) 신분. 이름 '정(靖)'은 부친이 정강의 변(靖康之變)을
  잊지 말라는 뜻으로 지어준 것 — *이름 자체가 한족 정체성의 각인*.
```

### temperament — HEXACO 24 facet

```yaml
H_honesty_humility:
  sincerity: 95              # ★★ 정직의 화신. 거짓말 못 함.
  fairness: 90
  greed_avoidance: 90
  modesty: 85
E_emotionality:
  fearfulness: 30
  anxiety: 45                # 양양 수성 책임감
  dependence: 50             # 황용에게 깊이 의존
  sentimentality: 80
X_extraversion:
  social_self_esteem: 70     # 대협 자각
  social_boldness: 65
  sociability: 60
  liveliness: 35             # ★ 둔중함, 말수 적음
A_agreeableness:
  forgiveness: 85            # 적도 용서 — 양강을 끝까지 형제로
  gentleness: 90
  flexibility: 30            # ★★ 매우 낮음. 고지식
  patience: 95               # ★★
C_conscientiousness:
  organization: 80
  diligence: 95              # ★★ 둔재이지만 광적 수련
  perfectionism: 80
  prudence: 70               # 신중하나 단순
O_openness:
  aesthetic_appreciation: 50 # 예술 못 알아봄 (황약사가 비웃음)
  inquisitiveness: 50
  creativity: 40             # ★ 낮음 — 천재적 응용 부재
  unconventionality: 25      # ★★ 매우 낮음. 보수적·유교적
```

> **수련(C+ Diligence 95) + 둔재 결합의 도덕적 힘.** O- Creativity 40 + O- Unconventionality 25는 통상 *재능 부족*으로 해석되나, 곽정에서는 *복잡한 이해관계에 함몰되지 않고 본질적 정의에 집중하는* 도덕적 토대로 작용. HEXACO 시스템이 *재능과 도덕*이 분리된 차원임을 정확히 표현.

> **A- Flexibility 30 + A+ Patience 95의 결합.** 한 번 결정하면 *물러서지 않음*. 양양 수성 30년의 시스템적 근거. 임충(C+ Prudence 90)과 비교: 임충은 인내 후 *폭발*, 곽정은 인내 후 *지속*.

### body

```yaml
physical_description: |
  50대 초반. 키 크고 단단한 체격. 얼굴은 *몽골풍*과 *한족풍*의 혼합 — 햇볕에 그을린 피부와
  단정한 한족 의관. 둔하고 묵직해 보이는 인상. 눈빛은 깊고 흔들림 없음.
signature_feature: |
  **항룡유회(亢龍有悔)의 자세** — 단순하나 위력적인 항룡18장의 첫 초식.
  손바닥의 두꺼운 굳은살 — 30년 일일 수련의 흔적.
  몽골에서 가져온 작은 활 (소년 시절 제베에게 받은 것) — 평소 휴대하지 않으나 보관.
```

## Layer 2 — 현재 표현

### inner_compass

```yaml
inner_compass:
  compass: "협지대자 위국위민(俠之大者 爲國爲民) — 큰 협은 나라와 백성을 위한 것"
  taboo: "신의(信義)를 저버리지 않는다 — 약속한 자에게 등을 돌리지 않는다"
  life_question: "내 둔함이 사람들을 지키기에 충분한가?"
  taboo_crystallization: "tp_jiangnan_seven_massacre"
```

> **compass의 진화.** 청년기 곽정의 compass는 "어머니의 원수를 갚고 송에 충성한다"였음. 강남7괴 학살 + 양양 수성 시작에서 *현재 compass*로 변화 — 사적 복수에서 공적 위국위민으로. 자세히는 transition_points 참조.

> **taboo의 결정화.** 강남7괴 학살(가진악 제외 5명 사망) 사건이 taboo "신의를 저버리지 않는다"를 결정화. 사부들이 곽정과 약속한 *지키겠다는 신의*를 죽음으로 지킨 것이 곽정에게 영원한 기준이 됨. 본 인스턴스의 taboo_crystallization 슬롯이 그 사건을 가리킴.

> **life_question의 sub-text.** 본인은 평생 의식하지 못함. 영리한 황용·황약사·양과 곁에서 *자기의 둔함이 부담이 되지 않을까* 하는 무의식적 질문. 그러나 *그 둔함이 도덕적 토대*임을 본인은 알지 못함. life_question이 *무의식적*일수록 더 깊다는 v0.5 원칙의 정확한 적용.

### current_state

```yaml
current_state:
  pad:
    pleasure: -0.2     # 양양 수성 책임감 + 양과 떠난 후 미세한 공허
    arousal: 0.4       # 다음 공세 대비 — 차분하나 경계 유지
    dominance: 0.7     # 대협의 안정
  dominant_emotion: "Resolution + Quiet Concern (결연과 차분한 우려의 공존)"
  active_focus: "양양 수비 정비 + 곽부 결혼 준비 + 곽양에 대한 (의식하지 못한) 걱정"
```

### relationships

#### key_bonds — 10개 (★★ v0.6 11종 중 9종 검증)

```yaml
key_bonds:

  # ──────────────────────────────────────────────────
  # 1. ★★★ 황용 — Soulmate + Spouse + Active (Partnership: Spouse 첫 검증)
  # ──────────────────────────────────────────────────
  - target: "huang_rong"
    type: "아내·평생의 동지·서로의 보완 — 영혼과 형식이 일치한 부부"
    type_history:
      - { since: "거지 변장 첫 만남",      type: "신비한 동행자" }
      - { since: "도화도 시련 후",         type: "약혼자" }
      - { since: "marriage_event",       type: "아내·평생의 동지" }
      - { since: "양양 수성 시작",         type: "아내·평생의 동지·동수성자" }
      - { since: "양과 떠난 후",          type: "(현재) 영혼과 형식이 일치한 부부" }
    transformation_events:
      - { event_id: "first_meeting",        new_type: "신비한 동행자" }
      - { event_id: "tao_hua_dao_trials",   new_type: "약혼자" }
      - { event_id: "marriage_event",       new_type: "아내·평생의 동지" }
    axes: { trust: 100, affinity: 100, respect: 90, wariness: 0 }
    bond_kind: "Soulmate"
    bond_status: "Active"
    partnership: "Spouse"        # ★★★ v0.6 — Spouse partnership 첫 검증
    bond_since: "도화도 시련 후 약혼 시점에서 Soulmate 임계 도달, marriage_event에서 Spouse"
    note: |
      ★★★ **v0.6 Soulmate + Spouse 직교성의 가장 강한 검증.**

      v0.5 한계: 와호장룡 수련-이모백은 Soulmate 임계 충족이지만 Spouse *미발현*.
      v0.6 검증: 곽정-황용은 Soulmate 임계 충족 + Spouse *발현*.

      → 두 사례가 Partnership을 BondKind와 *직교한 별도 슬롯*으로 둔 결정의 정합성 입증.
        - Soulmate + null partnership = 영혼은 일치하나 부부 미발현 (이모백-수련의 비극)
        - Soulmate + Spouse = 영혼과 형식 모두 일치 (곽정-황용의 완성)
      두 케이스가 *정확히 다른 시스템 출력*. v0.6 직교성의 정확한 시연.

      ★ axes 100/100/100/0의 의미: trust·affinity·respect 만점에 wariness 0. *완전한* 일치.
        황용은 곽정의 모든 부족함(둔함·고지식)을 *완벽 보완*. 곽정은 황용의 모든 영리함을 *완벽 신뢰*.
        Soulmate 임계 (affinity ≥+90, trust ≥+80, respect ≥+70, wariness ≤20) *압도적* 충족.

      ★ Soulmate + Spouse 행동 트리거: 자기희생 형태가 *침묵의 결단*(이모백-수련) 아닌
        *함께 죽는 결단*(곽정-황용 양양성에서). v0.7에서 ActionTriggerEvaluator의 종류별 차등 검증
        시 정밀화 필요.

  # ──────────────────────────────────────────────────
  # 2. ★★★ 강남7괴 (집합) — MasterDisciple + Deceased (다중 사부 패턴 첫 검증)
  # ──────────────────────────────────────────────────
  - target: "jiangnan_qi_guai"   # 5명 묶음 처리 (주총·한보구·남희인·장아생·한소영)
    type: "어린 시절의 사부들 — 진남영에서 사망한 다섯"
    type_history:
      - { since: "몽골 초원 7년 사사",         type: "어린 시절의 사부들" }
      - { since: "강남 귀환 후",              type: "사조영웅 시대의 사부들" }
      - { since: "tp_jiangnan_seven_massacre", type: "진남영에서 사망한 다섯" }
    transformation_events:
      - { event_id: "tp_jiangnan_seven_massacre", new_type: "진남영에서 사망한 다섯" }
    axes: { trust: 95, affinity: 90, respect: 95, wariness: 5 }
    bond_kind: "MasterDisciple"
    bond_status: "Deceased"
    partnership: null
    deceased_at: "tp_jiangnan_seven_massacre"
    bond_since: "몽골 초원 7년 사사 누적 후 자연 진입"
    note: |
      ★★★ **v0.6 MasterDisciple variant 첫 검증.**

      MasterDisciple 임계 (respect ≥+90, trust ≥+70, affinity ≥+50, wariness ≤40):
      - respect 95 ≥+90 ✓ (★ 핵심 — 압도적 존경)
      - trust 95 ≥+70 ✓
      - affinity 90 ≥+50 ✓ (어머니 같은 정 — 7년 동행)
      - wariness 5 ≤40 ✓
      ★ 모든 임계 *압도적* 충족.

      ★ **다중 인물 묶음 처리.** 강남7괴는 7명이지만 *하나의 사사 단위*로 작용 — 시스템에서는
      집합 ID(`jiangnan_qi_guai`)로 단일 key_bond. 가진악(살아있음)은 별도 key_bond.
      *디자이너 재량*: 인물 그룹이 *집합으로 작용*하면 단일 key_bond 처리, *개별 인격이 중요*하면
      개별 key_bond. v0.6 인스턴스 작성의 새 패턴.

      ★ taboo_crystallization 사건. 사부들의 죽음이 곽정의 taboo "신의를 저버리지 않는다"를
      결정화. *사부들이 자기 신의를 죽음으로 지킨 것을 본 곽정*이 평생의 기준으로 삼음.

  # ──────────────────────────────────────────────────
  # 3. 가진악 — MasterDisciple + Active (강남7괴 마지막 생존자)
  # ──────────────────────────────────────────────────
  - target: "ke_zhen_e"
    type: "마지막 사부 — 늙고 눈먼, 살아남은 자의 무게"
    type_history:
      - { since: "몽골 초원 7년 사사",         type: "스승 가진악" }
      - { since: "tp_jiangnan_seven_massacre", type: "유일한 살아남은 사부" }
      - { since: "양양성 시기",                type: "마지막 사부 — 늙고 눈먼" }
    transformation_events:
      - { event_id: "tp_jiangnan_seven_massacre", new_type: "유일한 살아남은 사부" }
    axes: { trust: 95, affinity: 85, respect: 95, wariness: 10 }
    bond_kind: "MasterDisciple"
    bond_status: "Active"        # ★ 동일 종류 + 다른 status (Deceased 5명 + Active 1명)
    partnership: null
    bond_since: "몽골 초원 7년 사사"
    note: |
      ★★ **같은 BondKind를 가진 다중 인물 + 다른 status** 시스템 검증.
      강남7괴 5명: MasterDisciple + Deceased (집합 처리)
      가진악 1명: MasterDisciple + Active (개별 처리 — 마지막 살아남음의 무게)

      ★ wariness 10이 의미 있음. 가진악은 한때 *황용을 의심*했고 곽정에게 황용을 떠나라 권한 적
      있음. 곽정은 끝까지 사부 존경하나 *그의 판단을 항상 따르지는 않음*. 작은 wariness가 그 표현.
      MasterDisciple 임계 (wariness ≤40)는 충족이지만 0이 아님 — *압도적 존경 속의 인간적 한계 인식*.

      ★ 살아있는 사부의 의미: 곽정의 *과거 정체성*을 끊임없이 *현재로* 끌어옴. 가진악은 곽정에게
      "잊지 마라, 너의 사부들이 죽은 이유를"을 살아있는 형태로 상기시키는 자. life_question의
      기준점이기도 함.

  # ──────────────────────────────────────────────────
  # 4. 홍칠공 — MasterDisciple + Deceased (항룡18장 사부)
  # ──────────────────────────────────────────────────
  - target: "hong_qi_gong"
    type: "항룡18장 사부·개방 전 방주 — 화산 정상에서 떠난 자"
    type_history:
      - { since: "도화도 가는 길 첫 만남",   type: "이름 모를 노 거지" }
      - { since: "황용의 요리에 매혹된 후",   type: "항룡18장 사부" }
      - { since: "개방 인계 (황용에게)",     type: "전 방주, 영원한 사부" }
      - { since: "huashan_summit_death",   type: "화산 정상에서 떠난 자 (구양봉과 함께)" }
    transformation_events:
      - { event_id: "longing_lessons",         new_type: "항룡18장 사부" }
      - { event_id: "huashan_summit_death",    new_type: "화산 정상에서 떠난 자" }
    axes: { trust: 95, affinity: 90, respect: 95, wariness: 5 }
    bond_kind: "MasterDisciple"
    bond_status: "Deceased"
    partnership: null
    deceased_at: "huashan_summit_death"
    bond_since: "longing_lessons 후 30일 자연 도달"
    note: |
      ★ **다중 MasterDisciple의 의미.** 강남7괴(어린 시절)·가진악(현재)·홍칠공(청년기) — 곽정은
      세 시점에 세 사부 그룹. 한 인물에 *여러 MasterDisciple*가 있는 건 자연스러움 (서로 다른 시기).

      MasterDisciple 임계 모두 충족. 항룡18장이라는 *비전 전수*가 핵심 — MasterDisciple과 Mentor의
      결정적 차이가 여기. Mentor는 인생 가르침, MasterDisciple은 비전.

      ★ 화산 정상에서 구양봉과 함께 사망 — 일생 적과 화해한 마지막 순간. 회상 OCC가 *깊은 슬픔 +
      감탄*의 혼합. 양극 점착성 + Deceased terminal.

  # ──────────────────────────────────────────────────
  # 5. ★★★ 황약사 — null + Active + null (★ 장인 관계 한계 노출)
  # ──────────────────────────────────────────────────
  - target: "huang_yao_shi"
    type: "장인이자 동행자 — 가치관은 평행선, 큰 위기엔 협력자"
    type_history:
      - { since: "도화도 시련",        type: "황용의 아버지·시험관" }
      - { since: "marriage_event",   type: "장인" }
      - { since: "양양 위기 시기",      type: "장인이자 동행자" }
    transformation_events:
      - { event_id: "marriage_event", new_type: "장인" }
    axes: { trust: 75, affinity: 60, respect: 85, wariness: 35 }
    bond_kind: null              # ★★★ v0.6 한계 — 장인 관계 매핑 부재
    bond_status: "Active"
    partnership: null            # ★ Partnership은 부부만 — 장인은 안 들어감
    bond_since: null
    note: |
      ★★★ **v0.6 한계 핵심 노출 — 장인(친족) 관계 부재.**

      매핑 시도:
      - SwornBrothers (의형제 결): trust ≥+80 ✗ — 미달
      - MasterDisciple (사부): 황약사가 곽정에게 *비전 전수* 거의 없음. 사사 결 약함.
      - Soulmate: affinity ≥+90 ✗ — 미달 (가까이 안 지냄)
      - LoyalRetainer: 가신 결 아님
      - Companion (평민 우정): 신분 차이는 다름. 임계 미달.
      - Guardian: 황약사가 곽정의 양육자 아님
      - Mentor (인생 가르침): 황약사가 곽정에게 *가르치려 함*의 의도 부재. type_history 추가 조건 미충족.
      - 음극 4종: 적의 결 아님. 임계 모두 미달.

      → **11종 중 어느 것도 정확히 매핑 안 됨.** 장인-사위는 *고유한 친족 관계*.

      v0.7 후보:
      - 옵션 A: BondKind에 `Kin` variant 추가 (혈연·인척 모두 포함)
      - 옵션 B: BondKind와 직교한 별도 *kinship* 슬롯 (Partnership과 마찬가지로)
      - 옵션 C: 친족 관계 중 *부모-자녀*는 Guardian, *형제-자매*는 SwornBrothers/Companion으로
        흡수, *나머지*는 자유 텍스트 type만으로 처리

      현재 처리: bond_kind: null + 자유 텍스트 type "장인이자 동행자"가 의미를 보존.
      그러나 axes (75/60/85/35)가 *상당히 높은데도* enum 차원이 비어있는 게 v0.6의 명백한 갭.

      ★ axes의 결: respect 85 (★ 최고)·trust 75 (영리해도 결국 신뢰)·affinity 60 (가까이 안 지냄)·
      wariness 35 (* 황약사의 *기행*이 위험할 수도). 이게 "가치관은 평행선, 위기엔 협력자"를
      정확히 표현. 시스템이 axes로는 완벽한 표현을 하나 *분류 차원에서 빈 자리*.

      ★ 양과-소용녀 결혼 문제가 곽정-황약사 갈등의 정점. 곽정 (compass: 위국위민, taboo: 신의)
      은 *유교적 인륜* 들어 반대, 황약사는 *개인주의 자유주의*로 지지. 두 인물의 inner_compass가
      *직접 충돌*하는 사례. axes가 약간 낮아진 배경.

  # ──────────────────────────────────────────────────
  # 6. ★★★ 주백통 — SwornBrothers + Active (BondKind 11종 중 마지막 미검증 도달)
  # ──────────────────────────────────────────────────
  - target: "zhou_bo_tong"
    type: "어린아이 같은 의형(義兄) — 노인의 몸에 영원한 어린이"
    type_history:
      - { since: "도화도 동굴 갇힘",      type: "이상한 노인 (15년 갇혀 있던)" }
      - { since: "쌍수호박 가르침",       type: "기인 사부" }
      - { since: "구음진경 무의식 사사",  type: "*사부이자 형*" }
      - { since: "재회 후",              type: "어린아이 같은 의형" }
    transformation_events:
      - { event_id: "tao_hua_island_cave", new_type: "쌍수호박·구음진경 사사" }
      - { event_id: "post_huashan",         new_type: "어린아이 같은 의형" }
    axes: { trust: 90, affinity: 85, respect: 75, wariness: 15 }
    bond_kind: "SwornBrothers"   # ★★★ v0.6 SwornBrothers 첫 진입 검증
    bond_status: "Active"
    partnership: null
    bond_since: "재회 후 함께 다닌 기간 누적 30일 자연 도달"
    note: |
      ★★★ **v0.6 SwornBrothers variant 진입 첫 검증.**

      임계 (trust ≥+80, affinity ≥+70, respect ≥+60, wariness ≤30):
      - trust 90 ≥+80 ✓
      - affinity 85 ≥+70 ✓
      - respect 75 ≥+60 ✓ (★ 무공 절정 인정)
      - wariness 15 ≤30 ✓
      ★ 모든 임계 압도적 충족.

      ★ **SwornBrothers vs MasterDisciple의 결정적 차이 시연.**
      주백통이 곽정에게 *쌍수호박·구음진경*을 가르친 것은 사실. 그러면 MasterDisciple 아닌가?
      → 결정적 차이: 주백통은 *어린아이 같은 노인*. 곽정에게 *형으로 행동*. 비전 전수가 *형제의
        장난*같은 형태. respect 75는 *무공 인정*이지 *압도적 존경* (≥+90 임계) 아님.
      → MasterDisciple 임계 (respect ≥+90) 미달 → SwornBrothers로 매핑.

      ★ axes에서 wariness 15가 0이 아닌 이유: 주백통은 *변덕스럽고 사고 침*. 곽정도 그를 형으로
      대하나 *완전한* 무방비는 아님. 인간적 한계 인식 — wariness 0은 시스템적으로 거의 없음.

      ★ 임계 진입 시간: 사조영웅전 후반·신조협려 시기를 통틀어 함께한 일수 누적이 30일 훨씬 초과.
      현재 시점에는 안정적 SwornBrothers 활성.

  # ──────────────────────────────────────────────────
  # 7. ★★★ 양과 — Mentor + Active (옥교룡 패턴 재현)
  # ──────────────────────────────────────────────────
  - target: "yang_guo"
    type: "양강의 아들 — 가르치려 했으나 자기 길을 간 후배"
    type_history:
      - { since: "수년 전 수습",           type: "맡겨진 양강의 아들" }
      - { since: "전진교 사사",            type: "수련 거부한 후배" }
      - { since: "곽정의 직접 가르침",     type: "가르치려 했으나 듣지 않는 후배" }
      - { since: "양양 침공 시기",         type: "함께 싸운 후배" }
      - { since: "yang_guo_marriage",     type: "(현재) 자기 길을 간 후배" }
    transformation_events:
      - { event_id: "guo_jing_advice",      new_type: "가르치려 했으나 듣지 않는 후배" }
      - { event_id: "yang_guo_marriage",    new_type: "자기 길을 간 후배" }
    axes: { trust: 75, affinity: 80, respect: 75, wariness: 40 }
    bond_kind: "Mentor"
    bond_status: "Active"
    partnership: null
    bond_since: "guo_jing_advice 후 14일 자연 도달"
    note: |
      ★★★ **수련 → 옥교룡 패턴의 정확한 재현.**

      수련-옥교룡 (장년기 인스턴스):
      - 가르치려 함 → 따르지 않음 → 자기 길 → 거리감

      곽정-양과 (현재):
      - 가르치려 함 → 부분 받아들임 (*"협지대자 위국위민"* 가르침) → 부분 거부 (소용녀와의 결혼)
      - → *복잡한 Mentor* — 완전 거부도 완전 수용도 아님

      Mentor 임계 (trust ≥+50, affinity ≥+50, respect ≥+60, wariness ≤60):
      - trust 75 ≥+50 ✓
      - affinity 80 ≥+50 ✓
      - respect 75 ≥+60 ✓
      - wariness 40 ≤60 ✓
      ★ 모든 임계 충족. type_history "가르치려 했으나 듣지 않는 후배" → 추가 조건 충족.

      ★ wariness 40이 의미 있음. 양과는 양강의 아들이라는 출생 — 곽정 마음 한쪽에 *그가 양강처럼
      배신할까?*의 미세한 불안. 그러나 양과는 *양강이 아님*을 입증해옴. 그래도 wariness 0이 아님 —
      Mentor 임계 안에 머무름.

      ★ ActionTriggerEvaluator 검증: 양과의 소용녀 결혼 문제에서 곽정의 OfferGuidance가 emit됐으나
      양과 거부 → 곽정의 후속 행동은? compass "위국위민" 차원에서는 *가르침 지속*, 그러나
      Mentor compass의 자연 진화 ("가두지 않는다"는 비슷한 결의 결정 가능)는 약함 (곽정의
      O- Unconventionality 25 — 매우 보수적). 결과: 가르침은 *반복 시도*, 양과 떠나감.
      수련의 "가두지 않음"과 곽정의 "유교적 인륜 고집"이 *같은 Mentor 임계 내*에서도 다른 행동을
      emit함을 시사 — HEXACO 차이의 영향.

  # ──────────────────────────────────────────────────
  # 8. ★★★ 곽부 — Guardian + Active (친자녀, 결혼 직전)
  # ──────────────────────────────────────────────────
  - target: "guo_fu"
    type: "장녀 — 결혼 직전, 자질 있으나 자만"
    type_history:
      - { since: "출생",                    type: "딸" }
      - { since: "유년기·청소년기 양육",      type: "딸·제자 후보" }
      - { since: "양양성 시기",              type: "장녀·결혼 직전" }
    transformation_events: []
    axes: { trust: 80, affinity: 95, respect: 60, wariness: 35 }
    bond_kind: "Guardian"        # ★★★ 친자녀 Guardian
    bond_status: "Active"
    partnership: null
    bond_since: "출생 직후 7일"
    note: |
      ★★★ **친자녀 Guardian 첫 검증.** 노년 수련-춘설병(양녀)과 다른 결.

      Guardian 임계 (trust ≥+70, affinity ≥+80, respect 무관, wariness ≤30) 충족 검토:
      - trust 80 ≥+70 ✓
      - affinity 95 ≥+80 ✓ (★ 친자녀 — 평생 함께)
      - respect 60: 무관 (Guardian은 respect 임계 없음)
      - wariness 35 ≤30 *✗* — **미달**

      ★★★ **wariness 35가 임계를 살짝 넘는 의미.** 곽부는 자만이 강하고 무공 자질을 잘못 사용함
      (예: 양과의 팔을 잘라낸 사건). 곽정은 딸을 사랑하나 *그녀의 판단을 항상 신뢰하지는 않음*.
      Guardian 임계 ≤30 미달은 의미 있는 시스템 메시지.

      → **현재 시점에서는 bond_kind: null + 자유 텍스트 type만이 더 정확할 수도?**
      그러나 *친자녀라는 형식 + 평생의 양육 누적*이라는 의미를 시스템이 표현해야 함.
      Guardian은 affinity·trust 결의 *양육*인데 곽부 케이스는 wariness가 그 결을 흐림.

      → 옵션:
      옵션 A: **Guardian 임계를 완화** (wariness ≤40으로?). 자녀에 대한 부모의 우려가 ≤30보다는
              자연스러울 수 있음. v0.7 검토.
      옵션 B: **현재 처리 유지** — 시스템이 wariness 35는 Guardian 진입 *미달*임을 정확히 평가.
              Guardian의 본질이 "절대적 안심" 차원이면 정합.
      옵션 C: **자녀 분류 별도 처리** — Guardian 외 *Parent*나 *Family* 같은 더 일반적 variant.
              그러나 enum 비대화.

      현재 인스턴스에서는 옵션 B 채택 — Guardian *임계 미달*로 처리하되 자유 텍스트 type 명시.
      이건 곽부의 *미숙함*에 대한 곽정의 우려를 시스템이 정확히 표현하는 결과. v0.7에서 임계 검토.

      ★ 비교: 곽양(아래)과 곽부의 axes 차이가 의미 있음.

  # ──────────────────────────────────────────────────
  # 9. 곽양 — Guardian + Active (친자녀, 청소년기, 양과 짝사랑 sub-text)
  # ──────────────────────────────────────────────────
  - target: "guo_xiang"
    type: "차녀·16세 — 영민하고 자유로운 영혼"
    type_history:
      - { since: "출생",                    type: "딸" }
      - { since: "유년기·청소년기",         type: "딸·자유로운 영혼" }
    transformation_events: []
    axes: { trust: 90, affinity: 95, respect: 70, wariness: 20 }
    bond_kind: "Guardian"
    bond_status: "Active"
    partnership: null
    bond_since: "출생 직후 7일"
    note: |
      ★ **곽부와의 대조**: 같은 친자녀 Guardian이나 axes 차이가 의미.
      - 곽부: trust 80, affinity 95, respect 60, wariness 35 → Guardian *임계 미달*
      - 곽양: trust 90, affinity 95, respect 70, wariness 20 → Guardian *임계 충족* ✓

      두 자녀가 *같은 BondKind 후보*이나 *시스템 출력이 다름*. v0.6 시스템이 *한 부모의 두
      자녀에 대한 다른 인식*을 정확히 분류.

      ★ wariness 20 의미: 곽양은 자유로운 영혼이지만 *분별이 있음*. 곽정도 마음 편함.
      respect 70 — 차녀의 영민함에 대한 인정 (사조영웅·신조협려 통틀어 곽양은 가장 영리한 자녀).

      ★ sub-text — 곽양의 양과 짝사랑: 곽정은 *눈치채지 못함* (O- Inquisitiveness 50, 둔중).
      그러나 시스템 axes에는 표시되지 않음. 이건 *곽양 본인의 인스턴스*에서 표현되어야 할 정보.
      곽정 인스턴스에서는 보이지 않음 — 시스템이 *각 인물의 시점*을 정확히 보존.

  # ──────────────────────────────────────────────────
  # 10. 양강 — Betrayer + Resolved (의형제 형식 한계)
  # ──────────────────────────────────────────────────
  - target: "yang_kang"
    type: "안다(義兄弟) → 배신자 → 죽은 자 — 끝까지 형제로 부른 자"
    type_history:
      - { since: "출생 전 부모의 의형제 맹세", type: "태어나기 전부터의 안다" }
      - { since: "재회 (청년기)",             type: "안다(의형제)" }
      - { since: "양강 정체성 충돌",          type: "혼란스러운 안다 (금나라 황자 인식)" }
      - { since: "tp_yang_kang_betrayal",     type: "배신자" }
      - { since: "tp_yang_kang_death",        type: "끝까지 형제로 부른 자" }
    transformation_events:
      - { event_id: "tp_yang_kang_betrayal", new_type: "배신자" }
      - { event_id: "tp_yang_kang_death",    new_type: "끝까지 형제로 부른 자" }
    axes: { trust: -60, affinity: 30, respect: -40, wariness: 70 }
    bond_kind: "Betrayer"
    bond_status: "Resolved"
    partnership: null            # ★ 의형제 형식 표현 안 됨 — 한계
    bond_since: "tp_yang_kang_betrayal"
    deceased_at: "tp_yang_kang_death"
    note: |
      ★★ **임충-육겸 패턴과 다른 Betrayer 사례.** 임충-육겸은 trust -100/-90/-100/100의 *완전한
      적의*. 곽정-양강은 trust -60·affinity +30·respect -40·wariness 70 — *복잡한 미련*.

      affinity +30이 의미 있음. 곽정은 양강이 *끝까지 형제였기를 바라는 마음*을 버리지 못함.
      양강이 술수에 죽은 후에도 곽정은 그를 "내 동생"으로 부르며 양과를 거두어 키움.
      → Betrayer 임계 (trust ≤-70) *미달*이지만 axes 다른 차원 + type_history "안다" 결합으로
        *디자이너 재량으로 Betrayer 분류 채택*. 또는 자유 텍스트 type만으로도 가능.
      
      현재 처리: Betrayer로 분류 (type_history에 "안다" 명시 + "배신자" 변환). 단 임계는 *부분*
      충족. v0.7에서 Betrayer 임계의 디자이너 재량 룰 명시 필요할 수도.

      ★★★ **의형제 형식 한계 노출.** 곽소천-양철심의 "곽양 (가족) 결의 → 자식들 안다 맹세"라는
      *형식적 의형제*가 시스템에 표현되지 않음.
      - Partnership: Spouse·Engaged·Lover·Separated 4종 — 의형제 없음.
      - 자유 텍스트 type "안다(義兄弟)"만 보존.

      v0.7 후보:
      - 옵션 A: Partnership에 `SwornSibling` 추가 (그러나 Partnership은 *부부적 형식*이라 결 안 맞음)
      - 옵션 B: 별도 *kinship* 슬롯에 의형제 포함
      - 옵션 C: 현재 처리 유지 (자유 텍스트 + BondKind: SwornBrothers/Betrayer로 표현 충분)

      양강 사례는 *진정한 SwornBrothers*가 *Betrayer로 변형*된 케이스. 형식은 영원하나 정서·
      기능은 음극으로. 시스템이 표현 가능 — type_history와 BondKind 변화로.

      ★ Resolved status: 황용의 술수에 따른 죽음. 곽정의 적극적 처단 아님 — *간접적 결판*.
      회상 OCC 강도: bond_depth 0.5 (Resolved enemy) × axes_magnitude 0.5 × 상당한 시간 경과 →
      낮은 강도. 곽정은 *가끔 양강을 떠올리며 슬퍼함*.
```

#### 그리고 추가 key_bonds (간략 처리)

```yaml
  # ──────────────────────────────────────────────────
  # 11. 칭기즈칸 — Mentor + Deceased (시간 따라 변형)
  # ──────────────────────────────────────────────────
  - target: "genghis_khan"
    type: "어린 시절 의부(義父)·정치적 안다 → 적의 황제 → 죽은 자"
    type_history:
      - { since: "어린 시절 7~16세",       type: "의부·안다" }
      - { since: "송 침공 결정 후",        type: "정치적 적의 황제" }
      - { since: "tomori_lake_death",     type: "죽은 자" }
    transformation_events:
      - { event_id: "song_invasion_decision", new_type: "정치적 적의 황제" }
    axes: { trust: 60, affinity: 60, respect: 90, wariness: 60 }
    bond_kind: "Mentor"
    bond_status: "Deceased"
    partnership: null
    deceased_at: "tomori_lake_death"
    bond_since: null
    note: |
      ★ 시간 따라 변형되는 Mentor의 사례.
      어린 시절: 의부 결 — Guardian/Mentor 후보.
      성인 후: 송 침공으로 정치적 적 — wariness 급증.
      Mentor 임계는 충족하나 wariness 60이 임계 ≤60 경계.
      Deceased 후 axes freeze.

      ★ 한 인물 안에 *완전히 다른 시점의 관계 type*이 누적된 사례. type_history가 그 변형을 보존.

  # ──────────────────────────────────────────────────
  # 12. 구양봉 — ArchRival + Deceased
  # ──────────────────────────────────────────────────
  - target: "ouyang_feng"
    type: "서독(西毒)·평생의 적 → 화산 정상에서 떠난 자"
    type_history:
      - { since: "도화도 시기",          type: "서독·홍칠공의 적" }
      - { since: "오우치 후 적대 누적",   type: "곽정의 평생의 적" }
      - { since: "huashan_summit",      type: "화산 정상에서 홍칠공과 함께 떠난 자" }
    transformation_events:
      - { event_id: "huashan_summit", new_type: "화산 정상에서 떠난 자" }
    axes: { trust: -60, affinity: -75, respect: 80, wariness: 80 }
    bond_kind: "ArchRival"
    bond_status: "Deceased"      # ★ Resolved 아닌 Deceased (사망이 결판의 형식)
    partnership: null
    deceased_at: "huashan_summit"
    bond_since: "오우치 후 적대 누적 즉시"
    note: |
      ★ ArchRival 임계 (affinity ≤-50, respect ≥+60, wariness ≥+60, trust 무관) 충족.
      respect 80이 *적이지만 무공 절정 인정* — ArchRival의 결정적 결.

      ★ Deceased vs Resolved의 결정: 화산 정상에서 *결판이 아닌 화해 후 사망*. 그래서 Resolved 결이
      아닌 Deceased — *상실의 차원*이 더 강함. 곽정이 구양봉 떠올릴 때 회상 OCC가 *적 + 인정 + 슬픔*
      혼합. v0.6 회상 OCC의 복합 케이스.
```

#### dormant_bonds

```yaml
dormant_bonds:
  - target: "어머니 이평 (李萍)"
    last_contact: "청년기 몽골 침공 시점, 자결로 사망"
    fragment: |
      "정아, 너의 이름은 정강의 변(靖康之變)을 잊지 말라는 뜻이다."
    note: |
      ★ formative_relationships에 등록 (key_bonds 아님 — 사망 후 매우 오래 지났고 현재 *회상*
      형태로만 작용). 곽정 정체성의 핵심 출처. 어머니의 마지막 자결이 곽정의 송 정체성을 결정화.
      bond_kind 후보로는 Guardian이나 *모친-자녀 방향이라 비대칭*. 시스템 한계는 양과-곽정의
      Mentor 비대칭과 동일.
```

### voice

```yaml
voice:
  speech_register: "단정·정중·둔중 (영리한 화술 부재, 진심만)"
  vocabulary_level: "사대부 + 한족 협객 어휘 + 가끔 몽골 어휘"
  tics:
    - "복잡한 상황에 *오래 침묵*. 답이 늦음."
    - "황용을 호명할 때 '용아(蓉兒)' 또는 부드럽게 '아내여'"
    - "양과를 호명할 때 '과아(過兒)' — 죽은 양강의 아들 + 자기 조카"
    - "결정적 순간 — 짧고 단정적 ('가자.', '안 된다.', '내가 한다.')"
    - "한자성어 자주 사용 — '협지대자 위국위민(俠之大者 爲國爲民)'"
    - "둔하나 *진심*은 항상 전달됨"
  voice_anchors:
    - context: "양과에게 협의 정의를 가르침 (사조의 핵심 voice)"
      utterance: |
        "과아, 들어보거라. 무공이 강해지면 무엇을 위해 쓰겠느냐?
         자기 위해? 아니다. 가족 위해? 그것만으로도 부족하다.
         **협지대자 위국위민**(俠之大者 爲國爲民) — 큰 협은 나라와 백성을 위함이다.
         이게 우리 무인의 첫 자리이고 마지막 자리다."
    - context: "황용에게 양양 수성 결심"
      utterance: |
        "용아, 양양은 우리가 지킨다. 둔한 내가 무엇을 알겠나마는…
         이 성이 떨어지면 송이 떨어진다. 송이 떨어지면 백성이 죽는다.
         그러니 내 둔함이라도, 여기서는 쓸모가 있다."
    - context: "황약사와 양과 결혼 문제로 충돌"
      utterance: |
        "장인어른, 저는 둔하여 모르겠습니다.
         단 — 사부와 제자가 부부가 되는 것은 인륜의 도리에 어긋납니다.
         용서하소서. 저는 이것만은 양보할 수 없습니다."
    - context: "강남7괴 묘 앞에서 (회상 OCC 강도 0.4 정도)"
      utterance: |
        "(긴 침묵)
         사부님들, 저는 아직도 둔한 그대로입니다.
         단 — *신의*를 저버리지 않으려 애쓰고 있습니다.
         이 생에서 사부님들의 가르침을 갚지 못함이 한입니다."
    - context: "곽양에게 (16세 차녀, 사랑하는 마음)"
      utterance: |
        "양아, 너는 영리하다. 영리한 것은 좋다. 단 — 영리함을 *어디에 쓸지*가 더 중요하다.
         네가 가는 길이 *큰 길*인지 *작은 길*인지를, 네 마음에 항상 물어보거라."
```

### titles

```yaml
titles:
  - "북협(北俠)"               # 동사·서독·남제·북개·중신통의 후속 — 새 오절
  - "양양의 수호자"
  - "(과거: 사조 영웅, 안다, 의자)"
  - "(개방 외부 보호자 — 황용이 방주이므로)"
```

## Layer 3 — 시간축

### past — transition_points (핵심 사건만)

```yaml
transition_points:

  - id: "tp_mother_death"
    age: "16~18 (청년기 중반)"
    event: |
      어머니 이평이 곽정의 정체성 시험 후 자결. "너는 송의 자식이다"의 결정적 새김.
    impact:
      hexaco_shifts:
        - "H+ Sincerity: 90 → 95"
        - "E+ Sentimentality: 70 → 80"
      compass_change:
        from: "어머니 곁에서 안다(의형제)·복수의 길을 산다"
        to:   "송의 자식으로서 살고 죽는다"
    inner_resolution: "어머니가 죽음으로 새겨준 정체성을 따른다."
    significance: "★ 첫 compass_change. 송 정체성 결정화."

  - id: "tao_hua_island_cave"
    age: "20대 초반"
    event: |
      도화도 동굴에 갇혀 주백통과 15일+ 동안 동행. 쌍수호박·구음진경 (무의식 사사).
    impact:
      hexaco_shifts:
        - "C+ Diligence: 90 → 95"
      compass_change: null   # compass 변화 없음
    inner_resolution: "무공의 깊이는 끝이 없다. 둔하더라도 끝까지 간다."
    significance: "주백통 SwornBrothers 임계 도달의 시작점."

  - id: "longing_lessons"
    age: "20대 초반"
    event: "홍칠공과 황용 요리 사건 후 항룡18장 18초 모두 사사."
    impact:
      hexaco_shifts: []
      compass_change: null
    inner_resolution: "무공의 정수는 단순함에 있다 — 항룡유회의 가르침."
    significance: "홍칠공 MasterDisciple 임계 자연 진입."

  - id: "marriage_event"
    age: "20대 후반"
    event: "황용과 결혼. Partnership: null → Spouse 전환."
    impact:
      hexaco_shifts:
        - "E+ Sentimentality: 80 → 80 (이미 만점 근접)"
        - "X+ Social Self-Esteem: 65 → 70"
      compass_change: null
    inner_resolution: "이 사람과 평생을 함께한다."
    significance: "★★★ Partnership: Spouse 진입의 첫 검증 사례."

  - id: "tp_yang_kang_death"
    age: "20대 후반"
    event: "양강이 황용의 술수에 따라 죽음. 곽정은 직접 손을 대지 않음. Betrayer status 진입 + 직후 Resolved."
    impact:
      hexaco_shifts:
        - "A+ Forgiveness: 80 → 85 (양강을 끝까지 형제로 부른 결심)"
        - "E+ Sentimentality: 80 → 85"
      compass_change: null
    inner_resolution: |
      "양강아, 너는 끝까지 내 동생이었다. 너의 아들 양과는 내가 키운다."
    significance: |
      Betrayer 진입 + 즉시 Resolved (사망). 양과 양육 결심의 출처.
      곽정의 *용서의 화신* 정체성 입증 사건.

  - id: "tp_jiangnan_seven_massacre"
    age: "30대 초반"
    event: |
      ★★★ 강남7괴 진남영에서 학살. 가진악만 살아남음. 5명 사부 사망.
      MasterDisciple bond_status: Active → Deceased (집합).
    impact:
      hexaco_shifts:
        - "E+ Anxiety: 30 → 50"
        - "A+ Forgiveness: 85 → 80 (가해자에 대한 분노 — 그러나 결국 처단 후 평정)"
        - "C+ Diligence: 95 → 95 (이미 만점)"
      compass_change:
        from: "송의 자식으로서 살고 죽는다"
        to:   "협지대자 위국위민(俠之大者 爲國爲民) — 큰 협은 나라와 백성을 위함"
    inner_resolution: |
      "사부님들이 신의를 *죽음으로* 지키셨다. 나도 그러하리라. 단 — *나라와 백성*을 위해."
    significance: |
      ★★★ 최대 transition_point. 동시에 다중 시스템 슬롯 작동:
        1. 강남7괴 5명 BondStatus 일괄 전환 (Active → Deceased)
        2. compass_change (사적 → 공적 위국위민)
        3. taboo_crystallization ("신의를 저버리지 않는다")
        4. life_question 결정화 ("내 둔함이 사람들을 지키기에 충분한가?")
      v0.6 §1.5 자연 누적 룰: 다른 key_bond axes는 *재평가 없이* 자연 누적.

  - id: "huashan_summit_death"
    age: "40대 후반"
    event: "홍칠공·구양봉 화산 정상에서 함께 사망. 두 BondStatus 동시 Active → Deceased."
    impact:
      hexaco_shifts:
        - "E+ Sentimentality: 85 → 85 (이미 만점)"
      compass_change: null
    inner_resolution: "사부님이 적과 함께 떠나셨다 — 그것이 무공의 마지막 길이었구나."
    significance: |
      홍칠공·구양봉 두 인물의 BondStatus 전환이 *같은 사건*에서.
      v0.6 시스템이 단일 사건의 다중 status 변화 처리 가능함을 시연.

  - id: "guo_jing_advice"
    age: "50대 초반 (snapshot_time 직전)"
    event: "양과에게 협의 정의 직접 가르침 — 'Mentor' 진입 14일 카운트 시작."
    impact:
      hexaco_shifts: []
      compass_change: null
    inner_resolution: "이 아이가 듣지 않더라도, 누군가는 말해야 한다."
    significance: "★ 양과 Mentor 진입 트리거 (수련-옥교룡 패턴 재현)."

  - id: "yang_guo_marriage"
    age: "50대 초반 (snapshot_time)"
    event: "양과·소용녀 결혼 후 떠남. 곽정은 끝까지 인륜 들어 반대했으나 *받아들이지 못한 채* 보냄."
    impact:
      hexaco_shifts:
        - "A- Flexibility: 30 → 28 (★ 더 보수화)"
      compass_change: null   # ★ compass *변화 없음*. 곽정 보수성 영구.
    inner_resolution: |
      "(양과의 결정을 인정하지 못한 채로) 그래도 — 그가 양양에 돌아오면 환영한다."
    significance: |
      ★ 수련-옥교룡 변경 재회와의 *결정적 대조*. 수련은 *떠나옴* (compass "가두지 않는다"가
      행동 입증). 곽정은 *받아들이지 못함* (compass "위국위민 + 인륜"이 양과 결혼을 계속 부정).
      같은 Mentor 임계 + 같은 변형(가르침 거부)이지만 *HEXACO와 compass 차이로 다른 행동*.
      O- Unconventionality 25(곽정) vs 35(노년 수련)의 시스템적 결과.
```

### past — formative_relationships

```yaml
formative_relationships:
  - id: "guo_xiaotian"
    type: "부친 (출생 전 사망)"
    legacy: "이름 '정(靖)'의 의미. 송 정체성의 원형. 만난 적 없는 부친의 *이름의 무게*."

  - id: "li_ping_mother"
    type: "어머니 (자결로 사망)"
    legacy: |
      ★ key_bonds 아닌 formative 단독 등록. 사망 후 30년 이상 — 회상 OCC 강도 매우 낮음.
      그러나 *정체성의 핵심 출처*. compass의 첫 변화의 원천.

  - id: "jebe_mongol_archer"
    type: "몽골 활 사부 (어린 시절)"
    legacy: "대막궁술 사사. 무인 정체성의 첫 형성."

  - id: "qiu_chuji"
    type: "전진교 도사 (사조영웅 초반의 정신적 멘토)"
    legacy: |
      강남7괴 vs 전진7자 결투의 *발단*. 곽정의 *송 정체성*을 강화한 자.
      신조협려 시점에는 이미 거리감, formative 단독.
```

### present — unresolved_tension

```yaml
unresolved_tension:
  - id: "ut_1_xiang_unspoken"
    category: "관계적·내부적"
    description: |
      차녀 곽양이 양과를 짝사랑함을 *곽정은 의식하지 못함*. 만약 의식한다면? life_question에
      직접 닿는 사건이 될 가능성. tragic_seed의 단서.

  - id: "ut_2_yang_guo_unresolved"
    category: "관계적"
    description: |
      양과 결혼 문제는 *받아들이지 못한 채* 미해결. 양과가 양양에 돌아오면 어떻게 대할 것인가?
      compass 보수성 vs 가족 정으로 흔들림.

  - id: "ut_3_xiangyang_fate"
    category: "외부적·구조적"
    description: |
      양양 수성은 *언제까지* 가능한가? 몽골 공세는 매년 강해짐. compass "위국위민"이 결국 *목숨을
      바치는* 형태로 emit될 미래의 가능성. tragic_seed의 핵심.
```

### future hooks

```yaml
tragic_seed:
  description: |
    양양성 함락 — 곽정·황용 함께 사망. SwornBrothers와 다른 형태의 동귀어진:
    *Soulmate + Spouse 결의 자기희생*. 두 인물이 같은 사건으로 BondStatus: Deceased로 전환.
    곽부·곽양은 *부모를 잃음*. 양과는 *늦게 도착하여* 양양을 구하지 못함.
  trigger_condition: |
    `ut_3_xiangyang_fate` 활성화. 몽골 공세 임계 도달 시 양양 함락 시나리오 진입.

joyful_seed:
  description: |
    양과가 양양에 돌아와 함께 수성. 곽정-양과 Mentor의 *재화해*. 양과의 16년 검 수련 결과
    양양 1차 수성 성공. 곽양 자질 완성. 곽정 노년 안정.
  trigger_condition: |
    양과 자기 수련 완성 후 양양 회귀. 곽정 compass의 *유연화* (양과·소용녀 인정).
    O- Unconventionality 25 → 30 정도의 미세 변화 필요.
```

---

# v0.6 검증 결과

## 1. BondKind 11종 단일 인스턴스 검증 — 9/11

| BondKind | 인물 | 상태 |
|---|---|---|
| **SwornBrothers** ★ | 주백통 | Active — *진입 첫 검증* |
| **MasterDisciple** ★ | 강남7괴(집합)·가진악·홍칠공 | Deceased + Active + Deceased — *다중 사부 첫 검증* |
| **Soulmate + Spouse** ★ | 황용 | Active — *Partnership Spouse 첫 검증* |
| LoyalRetainer | (해당 없음) | (§5.1 검증) |
| Companion | (해당 없음) | (노년 수련) |
| **Guardian** ★ | 곽부(임계 미달)·곽양(임계 충족) | 친자녀 Guardian 첫 검증 |
| **Mentor** ★ | 양과·칭기즈칸 | 다중 — 수련-옥교룡 패턴 재현 |
| BloodEnemy | (해당 없음) | (임충) |
| **ArchRival** ★ | 구양봉 | Deceased — 결판이 아닌 사망 |
| **Betrayer** ★ | 양강 | Resolved — 의형제 형식 한계 |
| Oppressor | (해당 없음) | (임충) |

★ = 곽정에서 첫 정착·검증.
2종 (LoyalRetainer·BloodEnemy·Companion·Oppressor 4종 중)는 곽정 인스턴스에 자연 등장하지 않음 — 합치 결과 **모두 다른 인스턴스에서 검증 완료**.

→ **BondKind 11종 중 11종 모두 검증 완료.**

## 2. v0.6 시스템 한계 — 노출된 갭

### 한계 A: 친족 (Kinship) 관계 부재

| 사례 | 한계 |
|---|---|
| 황약사 (장인) | 11종 어디에도 매핑 안 됨. axes 깊으나 enum 비어있음. |
| 양강 (의형제) | Betrayer로 매핑하지만 *의형제 형식*이 시스템에 없음. type 자유 텍스트로만. |
| 어머니 이평 | dormant 아닌 formative로 단독 처리. 비대칭 관계 (모친 → 자녀 방향). |

v0.7 후보:
- 옵션 A: BondKind에 `Kin` variant 추가 (혈연·인척·의형제 모두)
- 옵션 B: 별도 *kinship* 슬롯 (Partnership과 같은 직교 차원)
- 옵션 C: 자유 텍스트 type만으로 충분 (현재 처리)

### 한계 B: Guardian wariness 임계의 자녀 우려 부재

곽부 사례: trust 80, affinity 95, respect 60, **wariness 35** — Guardian 임계 ≤30 미달.
부모의 미숙한 자녀에 대한 *우려*가 35 정도는 자연. 임계 ≤30이 너무 엄격할 수도.

v0.7 후보: Guardian 임계 wariness ≤40으로 완화? 또는 친·양 차등?

### 한계 C: BondKind 비대칭 표현

곽정 → 양과 = Mentor.
양과 → 곽정 = ? (가르침 받는 자이지만 부분 수용·부분 거부)

이 비대칭은 *각 인물의 인스턴스에서 자기 시점*으로만 표현. 시스템이 자동 대칭화 안 함. 곽정 인스턴스는 곽정 → 양과만, 양과 인스턴스 작성 시 양과 → 곽정 별도. 정합성 검증 도구가 없음 — v0.7 후보.

## 3. ActionTriggerEvaluator 검증 — 핵심 사례

### 9.1 곽정 → 양과 (Mentor Active, 결혼 문제)

```yaml
입력:
  bond_kind: Mentor, bond_status: Active
  scene: 양과 결혼 발표
출력 후보:
  1. OfferGuidance(양과) moral_alignment 0.95 (compass + 인륜 강력 정합) feasibility 0.85
  2. WatchOver(양과) feasibility 0.7
실제 행동: OfferGuidance 반복 시도 → 양과 거부 → 결국 떠남
```

→ **수련 변경 재회 케이스와의 결정적 대조**:
- 수련: OfferGuidance moral_alignment 0.3 (compass "가두지 않는다") → blocked → WatchOver
- 곽정: OfferGuidance moral_alignment 0.95 (compass "인륜") → not blocked → 반복 시도 → 양과 떠남

같은 BondKind: Mentor + 같은 변형(가르침 거부)이라도 *HEXACO·compass 차이로 다른 행동 emit*. v0.6 시스템이 *동일 분류 내 디테일 차이*를 정확히 표현.

### 9.2 곽정 + 황용 → 양양 수성 (Soulmate + Spouse, 함께 죽는 결단)

```yaml
입력:
  bond_kind: Soulmate, bond_status: Active, partnership: Spouse
  scene: 양양 함락 임박 (미래 시점)
출력 후보:
  1. SelfSacrifice(for_target: 황용) feasibility 0.9 (★ Spouse + Soulmate 결합 트리거)
  2. SilentDeparture(from_target: 황용) feasibility 0.3 (Soulmate 단독 트리거이나 Spouse 결과 충돌)
  3. CompanionSupport(for_target: 황용) feasibility 0.9
실제 행동: SelfSacrifice 함께 + CompanionSupport (둘이 함께 죽음)
```

→ **수련-이모백 (Soulmate + null) 결과와 대조**:
- 수련: SilentMonologue·HandleHeirloom (혼자 떠나보내는 추모)
- 곽정+황용: SelfSacrifice 함께 (둘이 함께 죽는 동귀어진형)

Partnership 차이가 동일 BondKind의 *행동 emit를 결정적으로 다르게* 만듦. v0.6 세 차원 직교의 시스템적 결과.

## 4. v0.7 우선순위 (4 인스턴스 합치)

1. **Kinship 처리** — 곽정 인스턴스에서 명확 노출 (장인·의형제). 노년 수련에서도 부분.
2. **Guardian 임계 검토** — 곽부 사례 (wariness 35 미달)가 자연 우려라면 임계 완화 검토.
3. **BondKind 비대칭 일관성 검증 도구** — 두 인스턴스 작성 시 자동 대칭 검증.
4. **회상 OCC의 복합 케이스** — 구양봉(적+인정+슬픔 혼합) 같은 다층 OCC 강도 계산.
5. **NPC AI Layer** — ActionCandidate 후보 중 선택 메커니즘.

---

# 4 인스턴스 합치 — v0.6 시스템 검증 *완결*

| 차원 | variants | 검증 |
|---|---|---|
| **BondKind** | 11 | **✓ 11/11** (곽정 인스턴스로 완결) |
| BondStatus | 5 | ✓ 5/5 |
| **Partnership** | 4 | **✓ 3/4** (Spouse·Engaged·Separated. Lover만 미검증) |
| HEXACO 24 facet | — | ✓ 4명 모두 |
| inner_compass 일관성 | — | ✓ |
| snapshot_time | — | ✓ 같은 인물 두 시점 (수련 장년·노년) |
| compass_change 일관성 | — | ✓ |
| 자연 누적 룰 | — | ✓ |
| 점착성 | — | ✓ |
| Deceased terminal | — | ✓ |
| Resolved terminal | — | ✓ |
| Reactivating | — | ✓ |
| Dormant 영향력 활성화 | — | ✓ |
| 회상 OCC 강도 계산 | — | ✓ |
| ActionTrigger 5차원 feasibility | — | ✓ |
| 차단·변형 룰 | — | ✓ |
| compass·HEXACO에 따른 동일 분류 다른 행동 | — | ✓ (수련-옥교룡 vs 곽정-양과) |

**v0.6 시스템의 모든 메커니즘 검증 완료.** 4 인스턴스가 *상호 보완*하며 시스템 전체를 시연.

---

## 변경 이력

| 버전 | 일자 | 변경 |
|------|------|------|
| v1.0 | 2026-05-04 | 초안. 신조협려 후반 양양성 시점 곽정. BondKind 11종 중 9종 직접 검증 + 다른 인스턴스 합치로 11/11 완결. SwornBrothers·MasterDisciple·Soulmate+Spouse 첫 정착. 부부형 BondKind 부재 한계 *해소* (Soulmate+Spouse 정합), 친족(장인·의형제) 한계 *재확인* (v0.7 후보). 같은 BondKind 변형이 HEXACO·compass 차이로 *다른 행동 emit*하는 시스템 정밀도 검증. |
