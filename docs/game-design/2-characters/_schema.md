# 인물 스키마 (Character Schema)

> Version: 0.5 — 2026-05-04
> 모든 인물 인스턴스가 따르는 공통 구조.
> 위치: `docs/game-design/2-characters/_schema.md`

## 설계 원칙

이 스키마는 **3개 층(Layer)**으로 구성된다:

- **Layer 1 — 본바탕 (Base)**: HEXACO 기질, 출신, 신체.
- **Layer 2 — 현재 표현 (Expression)**: 내적 나침반, 현재 감정, 관계, 화법.
- **Layer 3 — 시간축 (Arc)**: 과거 전환점, 현재 갈등, 미래 궤적.

핵심 원칙:
1. HEXACO가 base, 모두에게 공통.
2. 가치 표현은 inner_compass 한 곳에서 끝낸다.
3. 인물의 게임 내 비중에 따라 Tier로 부담 분배.

---

## Tier 시스템

| Tier | 대상 | 필수 필드 |
|------|------|-----------|
| **Tier 1** | 단역 | HEXACO 6 factor + `compass` 한 줄 |
| **Tier 2** | 조연 | + HEXACO 24 facet + 전체 `inner_compass` + transition_point 1개 + voice 기본 + `snapshot_time` |
| **Tier 3** | 주연·동료·주적 | 풀 스키마. `life_question` 필수. `voice_anchors` 3개 이상. |

---

## Layer 1 — 본바탕 (Base)

### identity
- `id`: 시스템 식별자
- `name`: 본명
- `nicknames`: 별명 목록
- `era`: 어느 시대 인물인가
- `stage_of_life`: 청년기 / 장년기 / 노년기
- `snapshot_time`: 이 인스턴스가 *언제의* 인물인가. 자유 텍스트.

### origin
- `birthplace`, `social_origin`, `kingdom_of_origin`, `family_background`

### temperament — HEXACO
> Tier 1: 6 factor 점수만. Tier 2 이상: 24 facet 모두.

```yaml
H_honesty_humility: { sincerity, fairness, greed_avoidance, modesty }
E_emotionality:     { fearfulness, anxiety, dependence, sentimentality }
X_extraversion:     { social_self_esteem, social_boldness, sociability, liveliness }
A_agreeableness:    { forgiveness, gentleness, flexibility, patience }
C_conscientiousness:{ organization, diligence, perfectionism, prudence }
O_openness:         { aesthetic_appreciation, inquisitiveness, creativity, unconventionality }
```

### body
- `physical_description`, `signature_feature`

---

## Layer 2 — 현재 표현 (Expression)

### inner_compass — 가치의 세 면

```yaml
inner_compass:
  compass: "..."
  taboo:   "..."
  life_question: "..."
  taboo_crystallization: "<transition_point id 또는 null>"
```

- **compass**: 인물을 *움직이게* 한다. 시간 따라 바뀔 수 있음.
- **taboo**: 인물을 *멈추게* 한다.
- **life_question**: 인물을 *흔들리게* 한다. 평생의 의문.

#### life_question 작성 원칙
1. 인물 본인은 의식하지 못할 수 있다.
2. 인물의 직접 대사에 그대로 나오면 안 된다.
3. 디자이너·LLM 메타 정보로만 사용.
4. transition_points에서 *드러날 수 있다*.

#### 엔진과의 연결
- PAD 자극의 비대칭 증폭기.
- 기연 트리거 단서 (Pillar 5).
- acting directive 깊이.

#### `taboo_crystallization`
- `transition_points[].id` 중 하나를 참조 (단일 source of truth).
- 모든 taboo가 결정화 사건을 가질 필요는 없음 (`null` 허용).

### current_state
- `pad`: { pleasure, arousal, dominance }
- `dominant_emotion`: OCC 우세 감정
- `active_focus`: 현재 가장 강한 동기

### relationships
> 본체는 별도 카테고리: `2-characters/relationships.md` (v0.5).

#### key_bonds — 활성/비활성 모두

```yaml
key_bonds:
  - target: <인물 id>
    type: <자유 텍스트, 현재 관계의 형태>
    type_history:
      - { since: <자유 텍스트>, type: <자유 텍스트> }
      ...
    transformation_events:
      - { event_id: <transition_point id>, new_type: <자유 텍스트> }
      ...
    axes:
      trust:    <-100 ~ +100>
      affinity: <-100 ~ +100>
      respect:  <-100 ~ +100>
      wariness: <0 ~ +100>
    bond_kind:    <BondKind | null>                # 9 variants 또는 null
    bond_status:  <BondStatus>                     # ★ v0.5 신설 — 5 variants
    partnership:  <Partnership | null>             # ★ v0.5 신설 — 4 variants 또는 null
    bond_since:   <자유 텍스트>                     # bond_kind 진입 시점
    deceased_at:  <event_id>                       # ★ v0.5 신설 — Deceased status일 때만
    note: <자유 텍스트>

dormant_bonds:
  - target: <인물 id 또는 "(구체 미정)">
    last_contact: <자유 텍스트>
    fragment:     <자유 텍스트>                     # 단편적 기억
    note:         <자유 텍스트>
```

> **dormant_bonds vs key_bonds[bond_status: Dormant]의 차이** (v0.5 명확화):
> - `dormant_bonds`: *한 번도 활성화된 적 없는* 잠재 관계. 기연 트리거의 빈 슬롯.
> - `key_bonds[].bond_status: Dormant`: *예전에 활성*이었으나 오래 멈춘 관계.
> 이 둘은 정의가 다르며 시스템 동작도 다르다 (전자는 활성화 *이벤트* 트리거, 후자는 status 전환).

#### `bond_kind` enum (★ v0.5 갱신: 8 → 9)

```rust
pub enum BondKind {
    // 지기 — 양극 임계 (진입 30일 / 이탈 즉시)
    SwornBrothers,
    MasterDisciple,
    Soulmate,
    LoyalRetainer,
    // 원수 — 음극 임계 (진입 즉시 / 이탈 30일)
    BloodEnemy,
    ArchRival,
    Betrayer,
    Oppressor,
    // 멘토 — 중간극 임계 (진입 14일 / 이탈 즉시)  ★ v0.5 신설
    Mentor,
}
```

각 종류의 임계값·행동 트리거는 `relationships.md` §3.1 참조.

#### `bond_status` enum (★ v0.5 신설)

```rust
pub enum BondStatus {
    Active,                              // 활성 관계
    Resolved { reason: String },         // 결판 도달 (ArchRival 결판, 화해, 이별)
    Deceased,                            // 상대 사망
    Dormant,                             // 비활성 (오래 멈춘 활성 관계)
    Reactivating { trigger: EventId },   // 재활성화 단서 들어옴
}
```

- 모든 `key_bonds[]`는 `bond_status` 필드를 *반드시* 가짐 (`Active`가 기본값).
- `Resolved`와 `Deceased`는 *terminal* — 다시 Active로 돌아오지 않음.
- 자세한 의미·전환은 `relationships.md` §3.5 참조.

#### `partnership` enum (★ v0.5 신설)

```rust
pub enum Partnership {
    Spouse,      // 정식 결혼
    Engaged,     // 정혼
    Lover,       // 연인 (결혼 전, 비정혼)
    Separated,   // 휴서·이혼·별거
}
```

- BondKind와 *완전히 직교*. Soulmate + Spouse 가능, null + Spouse 가능.
- Partnership과 axes도 직접 연동되지 않음 (정략결혼은 trust 0 + Spouse 가능).
- 변화 동력은 OCC 누적이 아닌 *공식 사건* (결혼식, 이혼).
- 자세한 의미는 `relationships.md` §3.6 참조.

#### 핵심 약속 (인스턴스 작성자가 반드시 지킬 것)

- **4축은 직교**.
- **음수는 *적극적 반대 인식***. 0과 다름.
- **wariness는 단방향**. 음수 없음.
- **type_history는 항상 누적**.
- **bond_kind는 임계값 만족 시에만 채움**. 임계 미달이면 `null`.
- **Betrayer 추가 조건**: type_history에 *이전의 가까운 type*이 존재해야 함.
- **Mentor 추가 조건**: type_history에 "가르치려 함" 또는 "조언함" 의미의 type 존재.
- **bond_status는 항상 명시**. 디폴트 `Active`라도 명시.
- **partnership은 형식만 표시**. 정서적 깊이는 BondKind와 axes에서.
- **Deceased status는 deceased_at 필수**.
- **세 차원의 직교**: BondKind / BondStatus / Partnership. 한 차원이 다른 차원을 자동 결정하지 않음.

### voice — LLM 연기용
- `speech_register`, `vocabulary_level`, `tics`, `voice_anchors` (Tier 3 필수, 3~5개)

### titles
- `titles`: 보유 칭호 목록

---

## Layer 3 — 시간축 (Arc)

### past

#### transition_points

```yaml
transition_points:
  - id: <자유 텍스트, 인스턴스 내 고유>
    age: <자유 텍스트>
    event: <한 줄>
    impact:
      hexaco_shifts:
        - "<특성+/- 이름: from → to>"
        ...
      compass_change:                              # compass가 *실제로 변한* 점에만
        from: <기존 compass>
        to:   <새 compass>
    inner_resolution: <한 줄>
    significance: <한 줄, 선택>
```

#### formative_relationships

```yaml
formative_relationships:
  - id: <인물 id>
    type: <자유 텍스트>
    legacy: <한 줄>
```

> v0.5 명확화: `formative_relationships`와 `key_bonds[bond_status: Deceased]`의 차이는 *현재 행동 영향*. 사망했으나 *현재 정체성·행동에 강한 영향*인 인물은 `key_bonds`에 Deceased status로. 사망했고 *과거 의미만 남은* 인물은 `formative_relationships`에. 같은 인물이 둘 다에 등록될 수도 있음 (현재 영향 + 과거 의미가 모두 큰 경우).

### present
- `unresolved_tension`: 1~3개

### future hooks (선택)
- `tragic_seed`, `joyful_seed`

---

## 검증 체크리스트

### Tier 1 (단역)
1. HEXACO 6 factor 점수가 채워져 있나?
2. `compass` 한 줄이 있나?

### Tier 2 (조연)
1. HEXACO 24 facet 점수가 모두 채워져 있나?
2. `inner_compass` 세 필드 중 *최소 compass + taboo* 가 있나?
3. `transition_points` 최소 1개?
4. `voice` 기본 정보가 있나?
5. `identity.snapshot_time` 필드 채워져 있나?

### Tier 3 (주연·주요)
1. HEXACO 24 facet 모두?
2. `inner_compass` 세 필드 모두 (life_question 필수)?
3. `transition_points` 최소 2개?
4. `voice_anchors` 최소 3개?
5. `unresolved_tension` 최소 1개?
6. `tragic_seed` 또는 `joyful_seed` 중 하나?
7. `identity.snapshot_time` 필드 채워져 있나?
8. `taboo_crystallization` 적절히 처리됐나?

### 일관성 검증 (모든 Tier)
- HEXACO ↔ taboo 일관성.
- HEXACO ↔ compass 일관성.
- HEXACO ↔ life_question 일관성.
- axes 직교성 일관성.
- snapshot_time ↔ compass 일관성: 같은 인물의 두 snapshot_time 인스턴스가 다른 compass를 가지면, 둘 사이의 transition_point에 `compass_change`가 *반드시* 있어야 함.
- **bond_kind ↔ axes 임계 일관성** *(양극·중간극·음극 모두)*: relationships.md §3.1의 임계값을 만족하는가?
  - 양극(지기 4종): 임계 미만인데 bond_kind가 SwornBrothers 등이면 모순.
  - 중간극(Mentor): 임계 미만인데 bond_kind가 Mentor면 모순. 추가 조건(type_history에 "가르치려 함")도 필수.
  - 음극(원수 4종): 임계 미만인데 bond_kind가 BloodEnemy 등이면 모순.
  - **Betrayer 특수**: 임계 충족 + `type_history`에 *이전의 가까운 type*이 *반드시* 존재.
- **(★ v0.5 신설) bond_status 일관성**:
  - 모든 key_bonds[]에 bond_status 필드 존재 (Active 디폴트라도 명시).
  - `Deceased` status 시 `deceased_at` 필수.
  - `Resolved` status 시 `reason` 필수.
  - `Reactivating` status 시 `trigger` (EventId) 필수.
  - `Resolved`·`Deceased`인 관계의 axes는 freeze (마지막 활성 시점의 값으로 보존). 인스턴스 작성 시 *그 시점*을 기준으로 채워야 함.
- **(★ v0.5 신설) partnership 일관성**:
  - Partnership: Spouse 또는 Separated인 관계는 결혼 사건이 type_history 또는 transformation_events에 *반드시* 등록되어 있어야 함.
  - Partnership: Engaged + Deceased 조합이면 정혼 사건과 사망 사건 모두 기록.
  - Partnership 변화는 transformation_events에 (axes 변화 사건이 아닌 *공식 사건*으로) 등록 권장.
- **(★ v0.5 신설) 세 차원 직교 검증**:
  - BondKind: Soulmate + Partnership: null이 자연스러운가? (영혼 일치 + 부부 미발현 — 가능)
  - BondKind: null + Partnership: Spouse가 자연스러운가? (정략결혼 — 가능)
  - BondKind와 Partnership이 자동 연동되지 않음을 인스턴스 작성자가 *의식*해야 함.
- taboo_crystallization ↔ transition_point 일관성: 가리키는 event_id가 transition_points[]에 *실제 존재*하는가?

---

## 변경 이력

| 버전 | 일자 | 변경 |
|------|------|------|
| v0.1 | 2026-05-04 | 초안 |
| v0.2 | 2026-05-04 | inner_compass + Tier 시스템 |
| v0.3 | 2026-05-04 | snapshot_time, taboo_crystallization, compass_change, type_history, transformation_events, axes 음수 허용 |
| v0.4 | 2026-05-04 | bond_kind enum 8 variants (지기 4 + 원수 4) |
| v0.5 | 2026-05-04 | **세 차원 직교화**: bond_kind 9 variants (Mentor 추가), bond_status 5 variants (신설), partnership 4 variants (신설). deceased_at 필드 신설. dormant_bonds vs Dormant status 정의 명확화. formative_relationships vs Deceased key_bonds 차이 명확화. 검증 체크리스트에 status·partnership·세 차원 직교 항목 추가. relationships.md v0.5와 동기화. |
