# 인물 스키마 (Character Schema)

> Version: 0.6 — 2026-05-04
> 모든 인물 인스턴스가 따르는 공통 구조.
> 위치: `docs/game-design/2-characters/_schema.md`
> 동반 시스템:
>   - `relationships.md` v0.6 — 관계 분류
>   - `action_triggers.md` v0.1 — 행동 emit 평가

## 설계 원칙

3-Layer 구조:
- **Layer 1 — 본바탕 (Base)**: HEXACO, 출신, 신체.
- **Layer 2 — 현재 표현 (Expression)**: 내적 나침반, 현재 감정, 관계, 화법.
- **Layer 3 — 시간축 (Arc)**: 과거 전환점, 현재 갈등, 미래 궤적.

핵심 원칙:
1. HEXACO가 base.
2. 가치 표현은 inner_compass 한 곳.
3. Tier로 부담 분배.
4. **★ v0.6: 분류와 행동은 분리.** _schema의 key_bonds는 *분류*. 행동 평가는 ActionTriggerEvaluator (`action_triggers.md`).

---

## Tier 시스템

| Tier | 대상 | 필수 필드 |
|------|------|-----------|
| Tier 1 | 단역 | HEXACO 6 factor + `compass` 한 줄 |
| Tier 2 | 조연 | + HEXACO 24 facet + 전체 inner_compass + transition_point 1개 + voice 기본 + snapshot_time |
| Tier 3 | 주연·동료·주적 | 풀 스키마. life_question 필수. voice_anchors 3개+ |

---

## Layer 1 — 본바탕

### identity

- `id`, `name`, `nicknames`, `era`, `stage_of_life`
- `snapshot_time`: 이 인스턴스가 *언제의* 인물인가. 자유 텍스트.

### origin

- `birthplace`, `social_origin`, `kingdom_of_origin`, `family_background`

### temperament — HEXACO

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

## Layer 2 — 현재 표현

### inner_compass

```yaml
inner_compass:
  compass: "..."
  taboo:   "..."
  life_question: "..."
  taboo_crystallization: "<transition_point id 또는 null>"
```

- compass: *움직이게* 함. 시간 따라 바뀔 수 있음.
- taboo: *멈추게* 함.
- life_question: *흔들리게* 함. 평생 의문.

#### life_question 작성 원칙
1. 인물 본인은 의식 못할 수 있다.
2. 직접 대사에 그대로 나오지 않는다.
3. 디자이너·LLM 메타 정보로만 사용.
4. transition_points에서 *드러날 수 있다*.

#### 엔진과의 연결
- PAD 자극의 비대칭 증폭기.
- 기연 트리거 단서.
- acting directive 깊이.

#### `taboo_crystallization`
- transition_points[].id 참조 (단일 source).
- 모든 taboo가 결정화 사건을 가질 필요 없음 (null 허용).

### current_state

- `pad`: { pleasure, arousal, dominance }
- `dominant_emotion`: OCC 우세 감정
- `active_focus`: 현재 가장 강한 동기

### relationships
> 본체: `relationships.md` v0.6.
> 행동 평가: `action_triggers.md` v0.1.

#### key_bonds

```yaml
key_bonds:
  - target: <인물 id>
    type: <자유 텍스트>
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
    bond_kind:    <BondKind | null>     # 11 variants 또는 null
    bond_status:  <BondStatus>          # 5 variants
    partnership:  <Partnership | null>  # 4 variants 또는 null
    bond_since:   <자유 텍스트>
    deceased_at:  <event_id>            # Deceased status일 때만
    note: <자유 텍스트>

dormant_bonds:
  - target: <인물 id 또는 "(구체 미정)">
    last_contact: <자유 텍스트>
    fragment:     <자유 텍스트>
    note:         <자유 텍스트>
```

> **dormant_bonds vs key_bonds[Dormant]의 차이**:
> - dormant_bonds: *한 번도 활성화된 적 없는* 잠재 관계. 영향력만 활성화 가능.
> - key_bonds[bond_status: Dormant]: *예전에 활성*이었던 관계의 휴면.

#### `bond_kind` enum (★ v0.6 갱신: 9 → 11)

```rust
pub enum BondKind {
    // 지기·동반 — 양극 임계
    SwornBrothers,    // 의형제·동지 (진입 30일)
    MasterDisciple,   // 사부-제자 (무술 비전 전수)
    Soulmate,         // 영혼의 동반자
    LoyalRetainer,    // 가신·은인
    Companion,        // 평생의 우인 ★ v0.6 신설 (진입 30일)
    Guardian,         // 부모-자녀형 ★ v0.6 신설 (진입 7일)
    // 멘토 — 중간극 임계
    Mentor,           // 인생 선배·후배 (진입 14일)
    // 원수 — 음극 임계 (진입 즉시)
    BloodEnemy,
    ArchRival,
    Betrayer,
    Oppressor,
}
```

각 종류의 임계값·행동 트리거는 `relationships.md` §3 참조.

#### `bond_status` enum

```rust
pub enum BondStatus {
    Active,
    Resolved { reason: String },
    Deceased,
    Dormant,
    Reactivating { trigger: EventId },
}
```

- 모든 key_bonds[]는 `bond_status` 명시 (Active 디폴트라도).
- `Resolved`/`Deceased`는 *terminal*.

#### `partnership` enum

```rust
pub enum Partnership {
    Spouse, Engaged, Lover, Separated,
}
```

- BondKind와 *완전 직교*.
- axes와 직접 연동되지 않음.
- 변화 동력은 *공식 사건*.

#### 핵심 약속 (인스턴스 작성자 준수 사항)

- 4축은 직교.
- 음수는 *적극적 반대 인식*. 0과 다름.
- wariness는 단방향.
- type_history는 항상 누적.
- bond_kind는 임계값 만족 시에만 채움. 임계 미달이면 null.
- **Betrayer 추가 조건**: type_history에 *이전의 가까운 type*.
- **Mentor 추가 조건**: type_history에 "가르치려 함" 또는 "조언함" 의미의 type.
- **Companion 재량**: 임계 도달이라도 *형제 결*과 다른 평민 우정이면 자유 텍스트 type 선택 가능.
- **Guardian 진입 7일**: 가족 형성은 빠름.
- **bond_status 항상 명시** (Active 기본도).
- **Deceased status는 deceased_at 필수**.
- **세 차원 직교**: BondKind / BondStatus / Partnership.

### voice
- speech_register, vocabulary_level, tics, voice_anchors (Tier 3 필수, 3~5개)

### titles
- 보유 칭호 목록.

---

## Layer 3 — 시간축

### past — transition_points

```yaml
transition_points:
  - id: <인스턴스 내 고유>
    age: <자유 텍스트>
    event: <한 줄>
    impact:
      hexaco_shifts:
        - "<특성+/- 이름: from → to>"
      compass_change:                      # compass가 *실제로 변한* 점에만
        from: <기존>
        to:   <새>
    inner_resolution: <한 줄>
    significance: <한 줄, 선택>
```

### past — formative_relationships

```yaml
formative_relationships:
  - id: <인물 id>
    type: <자유 텍스트>
    legacy: <한 줄>
```

> **formative_relationships vs key_bonds[Deceased]의 차이** (v0.5 명확화 유지):
> - 현재 정체성·행동에 강한 영향이면 → key_bonds[Deceased].
> - 과거 의미만 남으면 → formative_relationships.
> - 같은 인물이 양쪽 등록되는 경우는 *드물게만*.

### present — unresolved_tension

```yaml
unresolved_tension:
  - id: <자유 텍스트>
    category: <외부적 / 내부적 / 관계적>
    description: <한 줄>
```

### future hooks (선택)

```yaml
tragic_seed: { description, trigger_condition }
joyful_seed: { description, trigger_condition }
```

---

## 검증 체크리스트

### Tier 1
1. HEXACO 6 factor 채워져 있나?
2. compass 한 줄?

### Tier 2
1. HEXACO 24 facet?
2. inner_compass 중 *최소 compass + taboo*?
3. transition_points 최소 1개?
4. voice 기본 정보?
5. identity.snapshot_time?

### Tier 3
1. HEXACO 24 facet 모두?
2. inner_compass 세 필드 모두 (life_question 필수)?
3. transition_points 최소 2개?
4. voice_anchors 최소 3개?
5. unresolved_tension 최소 1개?
6. tragic_seed 또는 joyful_seed 중 하나?
7. identity.snapshot_time?
8. taboo_crystallization 적절히 처리?

### 일관성 검증 (모든 Tier)

- HEXACO ↔ taboo / compass / life_question 일관성.
- axes 직교성.
- snapshot_time ↔ compass 일관성: 두 시점이 다른 compass면 사이 transition_point에 compass_change 필수.
- **bond_kind ↔ axes 임계 일관성** (모든 종류):
  - 양극 6종 (SwornBrothers/MasterDisciple/Soulmate/LoyalRetainer/Companion/Guardian)의 임계 검증.
  - 중간극 1종 (Mentor)의 임계 + type_history 추가 조건.
  - 음극 4종 (BloodEnemy/ArchRival/Betrayer/Oppressor)의 임계.
  - **Betrayer 특수**: type_history에 이전 가까운 type *반드시*.
  - **Mentor 특수**: type_history에 "가르치려 함" *반드시*.
  - **Guardian 진입 7일** vs SwornBrothers/Companion 30일 vs Mentor 14일 — 진입 게이트 차등 적용.
- bond_status 일관성:
  - 모든 key_bonds[]에 명시.
  - Deceased 시 deceased_at 필수.
  - Resolved 시 reason 필수.
  - Reactivating 시 trigger 필수.
  - Resolved/Deceased 관계의 axes는 freeze (마지막 활성 시점 값).
- partnership 일관성:
  - Spouse/Separated이면 결혼 사건이 type_history 또는 transformation_events에 등록.
  - Engaged + Deceased이면 정혼 + 사망 사건 모두 기록.
- 세 차원 직교 검증:
  - BondKind와 Partnership이 자동 연동되지 않음 의식.
  - Soulmate + null partnership / null + Spouse 자연스러움.
- taboo_crystallization ↔ transition_point 일관성.
- **★ v0.6 신설: compass 변화 후 자연 누적 룰 의식**:
  - 큰 compass 변화 시 axes 일괄 재평가는 *불필요*. 자연 누적으로 충분.
  - 명시적 axes 재평가가 필요한 드문 경우는 transition_point의 impact에 *수동* 트리거로 기록.

---

## 변경 이력

| 버전 | 일자 | 변경 |
|------|------|------|
| v0.1 | 2026-05-04 | 초안 |
| v0.2 | 2026-05-04 | inner_compass + Tier |
| v0.3 | 2026-05-04 | snapshot_time, taboo_crystallization, compass_change, type_history, transformation_events, axes 음수 |
| v0.4 | 2026-05-04 | bond_kind enum 8 variants |
| v0.5 | 2026-05-04 | 세 차원 직교화 (BondKind 9 + BondStatus 5 + Partnership 4) |
| v0.6 | 2026-05-04 | **BondKind 11**: Companion·Guardian 신설. action_triggers.md 참조 추가 (행동 평가 분리). 검증 체크리스트에 Guardian/Companion 임계 + 진입 게이트 차등 + compass 자연 누적 룰 추가. relationships.md v0.6과 동기화. |
