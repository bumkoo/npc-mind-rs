# 행동 트리거 시스템 (Action Triggers)

> Version: 0.1 — 2026-05-04 (★ v0.6에서 신설)
> 위치: `docs/game-design/2-characters/action_triggers.md`
> 의존: `relationships.md` v0.7, `_schema.md` v0.6, `00-pillars.md` v0.2
> 참조: `npc-mind-rs` PAD 엔진

## 0. 왜 이 문서가 필요한가

`relationships.md`는 관계의 *분류*를 한다. BondKind: Betrayer = 처단 대상, BloodEnemy = 즉결 살해 의무, SwornBrothers = 동귀어진 후보, etc.

그러나 *분류 ≠ 실제 행동*. v0.5 인스턴스 검증에서 명확히 노출된 한계:

- **임충-고아내**: BloodEnemy 임계 충족, 그러나 *처단 행동 보류*. 정치 권력 보호막 때문.
- **임충-고구**: Oppressor 임계 충족, 직접 처단 *현실적 불가*. 양산박 합류로 *변형된 표출*.
- **수련-옥교룡 (재회 시)**: 본래라면 Mentor 가르침 트리거이나, 수련은 *떠나옴*. 옥교룡이 가족을 이뤘기 때문.

이 셋은 모두 *분류는 명확하나 행동이 그대로 emit되지 않는* 사례. 분류와 실행 사이에 *평가 단계*가 필요하다.

**핵심 명제**: BondKind는 *동기 부여*까지. 실행은 *별도 평가*.

이게 v0.6에서 분리된 이유. relationships.md는 *NPC가 누구를 어떻게 인식하는가*, action_triggers.md는 *그 인식이 어떤 행동으로 나타나는가*.

---

## 1. 시스템 개요

### 1.1 입력 / 출력

```rust
pub struct ActionTriggerEvaluator;

impl ActionTriggerEvaluator {
    pub fn evaluate(
        npc: &Npc,                          // 행동 주체
        relationship: &Relationship,        // 대상 관계
        scene_context: &SceneContext,       // 현재 환경
    ) -> Vec<ActionCandidate>;
}

pub struct ActionCandidate {
    pub action_kind: ActionKind,
    pub target: NpcId,
    pub feasibility: f32,           // 실행 가능성 (0.0 ~ 1.0)
    pub urgency: f32,               // 시급성 (0.0 ~ 1.0)
    pub blocked_by: Vec<BlockReason>,  // 차단 사유 (있으면)
    pub deferred_form: Option<DeferredAction>,  // 차단 시 *변형된* 행동
}
```

### 1.2 시스템의 위치

```
RelationshipUpdater [domain]
  → BondKind/Status/Partnership 평가, axes 갱신
  ↓
ActionTriggerEvaluator [domain]   ★ 본 문서 책임
  → 동기(BondKind) × 가능성(scene_context) 평가
  ↓
ActionCandidates emit
  ↓
NPC AI Layer [application]
  → 후보 중 선택, 실제 행동 실행
```

본 문서는 *후보 산출*까지. 후보 중 *어느 하나를 실제 실행*하는 결정은 NPC AI Layer (LLM 또는 결정론 룰).

> **v0.7 정합 노트**: 위 흐름의 *상단* (RelationshipUpdater의 입력)이 `relationships.md` v0.7에서 명시화됨. Inner Loop(대화 턴)는 axes를 갱신하지 않으며, ActionTriggerEvaluator도 Inner Loop에서 호출되지 않는다. 모든 평가는 Outer Loop (`after_dialogue` 후 Reflection 통과 시)에서만 동작. 자세한 분리는 `relationships.md` v0.7 §6.1.
>
> **구현 phasing**: ActionTriggerEvaluator는 Phase 3c 작업. Channel 2 (Temporal, Phase 3a) + Channel 3 (External, Phase 3b)의 출력 — `BondKindEntered`/`BondStatusChanged`/`NpcLearnedAbout` — 이 모두 평가 입력. 자세한 phasing은 `docs/tasks/mind-architecture/00-roadmap.md`.

### 1.3 ActionKind 분류

```rust
pub enum ActionKind {
    // 자기희생·헌신 계열 (지기·동반·양육)
    SelfSacrifice { for_target: NpcId },          // SwornBrothers 동귀어진
    BequestLegacy { to_target: NpcId },           // MasterDisciple 비급 전수
    SilentDeparture { from_target: NpcId },       // Soulmate 침묵의 결단
    LoyalAct { for_target: NpcId },               // LoyalRetainer 충성
    GuardianProtect { for_target: NpcId },        // Guardian 양육·보호
    CompanionSupport { for_target: NpcId },       // Companion 우정의 도움

    // 가르침 계열 (멘토)
    OfferGuidance { to_target: NpcId },           // Mentor 충고
    WatchOver { target: NpcId },                  // Mentor 지켜봄 (가르치지 않고)

    // 처단·복수 계열 (원수)
    DirectKill { target: NpcId },                 // BloodEnemy 즉결 처단
    FormalDuel { target: NpcId },                 // ArchRival 결투
    PrivateExposure { target: NpcId },            // Betrayer 폭로
    SystemicResistance { target: NpcId },         // Oppressor 봉기·반체제

    // 추모·회상 계열 (Deceased/Resolved)
    VisitGrave { for_deceased: NpcId },
    VisitMeaningfulPlace { for_deceased: NpcId },
    HandleHeirloom { for_deceased: NpcId },
    SilentMonologue { for_deceased: NpcId },
    SpeakOfThemToOthers { for_deceased: NpcId },

    // 회피·보류 계열 (자기 보호)
    AvoidContact { with_target: NpcId },
    DeferAction { against_target: NpcId, reason: String },
}
```

29 variants. BondKind 11종에 대응되는 행동 + 회상 5종 + 회피·보류 2종.

---

## 2. 평가 흐름

```
1. BondKind에서 *기본 후보* 도출 (§3)
   → 각 BondKind는 표준 ActionKind 후보 1~3개 emit
2. BondStatus 필터 (§4)
   → Resolved/Deceased는 추모 후보로 변환
   → Dormant는 후보 약화
3. 가능성 평가 (§5)
   → scene_context의 변수들로 feasibility 계산
   → 차단 사유(BlockReason) 식별
4. 차단 시 변형(deferred form) 산출 (§6)
   → 직접 행동 불가 시 *간접 행동* 후보로
5. 시급성 평가 (§7)
   → urgency = PAD intensity × bond_kind 본질 점수 × 시간 압박
6. 최종 ActionCandidate 목록 산출
```

---

## 3. BondKind → 기본 ActionKind 후보

각 BondKind의 *전형적 후보*. 디자이너가 *기본값*으로 사용, NPC 특성·상황에 따라 변형.

```rust
fn base_candidates(bond_kind: BondKind) -> Vec<ActionKind> {
    match bond_kind {
        // 지기·동반
        SwornBrothers   => vec![SelfSacrifice, CompanionSupport],
        MasterDisciple  => vec![BequestLegacy, OfferGuidance],
        Soulmate        => vec![SilentDeparture, SilentMonologue, CompanionSupport],
        LoyalRetainer   => vec![LoyalAct, SilentDeparture],
        Companion       => vec![CompanionSupport],
        Guardian        => vec![GuardianProtect, BequestLegacy],
        // 멘토
        Mentor          => vec![OfferGuidance, WatchOver],
        // 원수
        BloodEnemy      => vec![DirectKill],
        ArchRival       => vec![FormalDuel],
        Betrayer        => vec![DirectKill, PrivateExposure, AvoidContact],
        Oppressor       => vec![SystemicResistance, AvoidContact],
    }
}
```

복수 후보의 의미: NPC의 HEXACO·current_state·scene_context에 따라 *어느 후보가 우세할지* 결정. 예: Betrayer에 대해 H+ Sincerity 높은 NPC는 PrivateExposure(공개 폭로) 선호, A- Forgiveness 낮은 NPC는 DirectKill 선호.

---

## 4. BondStatus 필터

```rust
fn status_filter(candidates: Vec<ActionKind>, status: BondStatus, target: NpcId) -> Vec<ActionKind> {
    match status {
        Active => candidates,                // 그대로
        Resolved { .. } => to_recollection(candidates, target),  // 추모 후보로 변환
        Deceased => to_recollection(candidates, target),
        Dormant => candidates.iter().map(|c| c.weakened()).collect(),
        Reactivating { .. } => candidates,    // Active와 동일하게 평가
    }
}
```

`to_recollection`은 BondKind 본질에 따라 추모 행동 선택:
- Soulmate Deceased → SilentMonologue, HandleHeirloom 우세
- SwornBrothers Deceased → SpeakOfThemToOthers, VisitGrave 우세
- Guardian Deceased → VisitGrave, HandleHeirloom 우세
- ArchRival Resolved → 후보 거의 없음 (가끔 SpeakOfThemToOthers 정도, 강도 낮음)

`weakened`(Dormant)는 feasibility를 0.3 곱함. 행동은 *가능*하나 *우선순위 낮음*.

---

## 5. 가능성(Feasibility) 평가

행동의 *실행 가능성*을 5개 차원으로 평가:

```rust
pub struct FeasibilityScore {
    pub physical_access:    f32,   // 0.0 ~ 1.0
    pub power_balance:      f32,
    pub social_permission:  f32,
    pub self_capability:    f32,
    pub moral_alignment:    f32,
    pub combined:           f32,   // 5개 차원의 조합 (보통 곱)
}
```

### 5.1 Physical Access — 물리적 접근 가능성

*상대에게 물리적으로 닿을 수 있는가?*

```rust
fn physical_access(npc: &Npc, target: &Npc, scene: &SceneContext) -> f32 {
    let same_region = scene.region == target.current_region;
    let target_reachable = !target.is_hidden && !target.is_protected_location;
    match (same_region, target_reachable) {
        (true, true)   => 1.0,
        (true, false)  => 0.4,    // 같은 지역이나 보호된 장소
        (false, true)  => 0.2,    // 다른 지역, 추적 가능
        (false, false) => 0.05,
    }
}
```

검증 — 임충-고구: 고구는 동경 황궁. 임충은 양산박. 같은 지역 아님 + 황궁 = 보호된 장소. → **physical_access ≈ 0.05**.

### 5.2 Power Balance — 권력·무력 균형

*상대를 제압할 수 있는가?*

```rust
fn power_balance(npc: &Npc, target: &Npc, scene: &SceneContext) -> f32 {
    let martial = (npc.martial_power - target.martial_power) / 100.0 + 0.5;  // -0.5 ~ +1.5
    let political = (npc.political_power - target.political_power) / 100.0 + 0.5;
    let allies = npc.ally_count_at_scene as f32 / (npc.ally_count_at_scene + target.ally_count_at_scene + 1) as f32;
    (martial * 0.4 + political * 0.4 + allies * 0.2).clamp(0.0, 1.0)
}
```

검증 — 임충-고아내: 고아내 본인 무력은 약함, 그러나 *고구의 양아들*이라 정치 권력 ↑ + 개봉의 호위병 다수. 임충 단신·죄인 신분. → **power_balance ≈ 0.25**.

### 5.3 Social Permission — 사회적 허용

*그 행동이 사회적으로 허용되는가?*

```rust
fn social_permission(action: ActionKind, npc: &Npc, target: &Npc, scene: &SceneContext) -> f32 {
    // Action의 사회적 평가 + NPC 평판·신분에 따른 가중
    let base = action.social_acceptance_base();   // 결투는 0.7, 즉결 처단은 0.2 등
    let npc_reputation_modifier = npc.reputation_for_action(action);
    let scene_modifier = scene.social_eyes_modifier();   // 공개된 장소는 ↓
    (base * npc_reputation_modifier * scene_modifier).clamp(0.0, 1.0)
}
```

검증 — 무송-반금련 (BloodEnemy 즉결 처단): 형 무대 살해라는 *명백한 죄과*. 무송이 *증거를 모으고 공개적으로* 처단했다. → social_permission ≈ 0.6 (불법이지만 의로움 인정).

### 5.4 Self Capability — 자기 능력

*NPC 본인이 그 행동을 *수행할 수 있는가*?*

```rust
fn self_capability(action: ActionKind, npc: &Npc) -> f32 {
    let hexaco_fit = action.hexaco_fitness(&npc.hexaco);   // HEXACO 적합도
    let pad_state = action.pad_readiness(&npc.current_pad);  // 현재 PAD 상태에서 가능한가
    let physical_state = npc.physical_health;
    (hexaco_fit * 0.4 + pad_state * 0.4 + physical_state * 0.2).clamp(0.0, 1.0)
}
```

검증 — 노년기 수련의 SilentDeparture: HEXACO에 자연스러움 (A+ 높음, O- 낮음 = 절제 정착). PAD 안정. → self_capability ≈ 0.85.

### 5.5 Moral Alignment — 도덕적 정렬

*행동이 NPC의 inner_compass와 충돌하지 않는가?*

```rust
fn moral_alignment(action: ActionKind, npc: &Npc, target: &Npc) -> f32 {
    let compass_alignment = action.compass_fit(&npc.inner_compass.compass);
    let taboo_violation = action.violates_taboo(&npc.inner_compass.taboo);
    if taboo_violation { return 0.0; }   // taboo 위반은 *완전 차단*
    let life_question_resonance = action.resonates_with_question(&npc.inner_compass.life_question);
    (compass_alignment * 0.6 + life_question_resonance * 0.4).clamp(0.0, 1.0)
}
```

검증:
- 임충 야저림: 호송관 살해 = taboo "무고한 자에게 칼을 휘두르지 않는다" *위반* → moral_alignment = 0.0 → 행동 차단.
- 수련 SilentDeparture (옥교룡 변경 재회): compass "젊은 세대를 가두지 않는다"와 *완벽 정렬* → moral_alignment ≈ 0.95.

### 5.6 Combined Feasibility

```rust
fn combined(scores: &FeasibilityScore) -> f32 {
    if scores.moral_alignment < 0.1 { return 0.0; }   // taboo 위반은 *전체 차단*
    let positive = scores.physical_access * scores.power_balance * scores.social_permission;
    let qualifier = scores.self_capability * scores.moral_alignment;
    positive.powf(0.6) * qualifier.powf(0.4)   // 비대칭 가중 — 외부 차단이 더 강함
}
```

`powf(0.6)` 비대칭의 의미: *외부 변수* (physical/power/social) 중 하나가 매우 낮으면 전체가 낮아짐. *내부 변수* (capability/moral)는 그보다 약하게 영향. NPC가 *능력과 의지*가 있어도 *상황이 안 되면* 행동 못함의 정확한 표현.

---

## 6. 차단 시 변형 (Deferred Form)

feasibility < 0.3이면 *차단됨*. 그러나 단순 포기가 아니라 *변형된 행동*을 후보로 emit.

```rust
pub enum BlockReason {
    PhysicallyUnreachable,
    OverwhelminglyPowerful,
    SocialTaboo,
    SelfIncapable,
    MoralConflict,
}

pub struct DeferredAction {
    pub original: ActionKind,
    pub blocked_by: Vec<BlockReason>,
    pub variant: ActionKind,        // 변형된 행동
}
```

### 변형 룰

```rust
fn defer(original: ActionKind, blocks: &[BlockReason]) -> Option<ActionKind> {
    match (original, blocks.contains(&PhysicallyUnreachable), blocks.contains(&OverwhelminglyPowerful)) {
        // 직접 처단 차단 → 체제 저항으로
        (DirectKill, true, _) | (DirectKill, _, true) => Some(SystemicResistance),
        // 결투 차단 → 후일 기약 (변형 없이 보류)
        (FormalDuel, _, _) => Some(DeferAction { reason: "결투 가능성 미도달" }),
        // 자기희생 차단 → 다른 형태의 헌신
        (SelfSacrifice, true, _) => Some(LoyalAct),    // 멀리 떨어진 형제를 위한 작은 헌신
        // Soulmate 침묵의 결단 차단 → SilentMonologue로
        (SilentDeparture, _, _) => Some(SilentMonologue),
        // 가르침 차단 → 지켜봄으로
        (OfferGuidance, _, _) => Some(WatchOver),
        // 양육 보호 차단 → 멀리서 지켜봄
        (GuardianProtect, true, _) => Some(WatchOver),
        // 폭로 차단 → 회피로
        (PrivateExposure, _, _) => Some(AvoidContact),
        _ => None,
    }
}
```

검증 사례:

#### 임충 → 고구 (DirectKill 차단)
- physical_access 0.05 + power_balance 매우 낮음 → blocks = [PhysicallyUnreachable, OverwhelminglyPowerful]
- defer 룰: DirectKill → SystemicResistance
- 변형 행동: *양산박 합류, 봉기에 가담*
- → 임충의 양산박 합류가 *시스템적으로 도출*됨. 단순 망명이 아닌 Oppressor에 대한 변형된 처단.

#### 수련 → 옥교룡 (변경 재회 시 OfferGuidance 차단)
- 옥교룡이 가족을 이루고 행복함. 수련 compass "가두지 않는다"와 *충돌 가능성* (가르침이 *가두는 것*이 됨)
- moral_alignment 평가에서 0.3 정도로 낮음 (가르치는 것이 compass와 미세 충돌)
- defer 룰: OfferGuidance → WatchOver
- 변형 행동: *짧은 만남 후 떠나옴, 멀리서 지켜봄*
- → 수련이 *떠나옴*이 시스템적으로 도출.

#### 임충 → 고아내 (DirectKill 차단)
- physical_access 0.7 (개봉 같은 도시), power_balance 0.25 (정치 보호)
- blocks = [OverwhelminglyPowerful]
- defer 룰: DirectKill → SystemicResistance
- 변형 행동: *고아내 처단을 양산박 활동의 *일환*으로 후일 기약*
- → 임충의 고아내 처단 욕구가 *Oppressor 행동에 흡수*되어 표출.

---

## 7. 시급성 (Urgency) 평가

같은 feasibility라도 *얼마나 급하게* 행동할 것인가가 다름.

```rust
fn urgency(npc: &Npc, action: ActionKind, scene: &SceneContext) -> f32 {
    let pad_intensity = npc.current_pad.magnitude();   // 현재 감정 강도
    let bond_essence = action.bond_essence_score();    // 행동의 본질적 시급성
    let temporal_pressure = scene.time_pressure;       // 환경 압박 (예: 적 도주 중)
    (pad_intensity * 0.4 + bond_essence * 0.4 + temporal_pressure * 0.2).clamp(0.0, 1.0)
}
```

`bond_essence_score`:
- BloodEnemy DirectKill: 0.9 (즉결 본질)
- Soulmate SilentMonologue: 0.2 (천천히)
- Guardian GuardianProtect: 0.7 (긴박할 수도)
- Mentor OfferGuidance: 0.4 (인내심)

검증 — 무송 vs 임충의 BloodEnemy 시급성:
- 무송 → 반금련: 발견 즉시 처단. PAD 강 + bond_essence 0.9 + 도주 가능 0.8 → urgency ≈ 0.85.
- 임충 → 고아내: 차단됨 (feasibility 낮음), 변형 행동의 urgency는 낮음 (long-term).

---

## 8. 최종 ActionCandidate 산출 흐름

```rust
fn evaluate(npc: &Npc, rel: &Relationship, scene: &SceneContext) -> Vec<ActionCandidate> {
    // 1. BondKind 기본 후보
    let mut candidates: Vec<ActionKind> = if let Some(kind) = rel.bond_kind {
        base_candidates(kind)
    } else {
        vec![]   // bond_kind null이면 자유 텍스트 type 기반 별도 평가 (생략)
    };

    // 2. BondStatus 필터
    candidates = status_filter(candidates, rel.bond_status, rel.target);

    // 3-7. 각 후보별 평가
    candidates.into_iter().map(|action| {
        let feasibility_scores = evaluate_feasibility(npc, action, &rel.target, scene);
        let combined = feasibility_scores.combined();
        let blocked = identify_blocks(&feasibility_scores);
        let urgency = urgency(npc, action, scene);

        let deferred = if combined < 0.3 {
            defer(action, &blocked).map(|variant| DeferredAction {
                original: action, blocked_by: blocked.clone(), variant
            })
        } else { None };

        ActionCandidate {
            action_kind: action,
            target: rel.target,
            feasibility: combined,
            urgency,
            blocked_by: blocked,
            deferred_form: deferred,
        }
    }).collect()
}
```

---

## 9. v0.5 인스턴스에서의 검증

### 9.1 임충 → 고구 (Oppressor)

```yaml
입력:
  bond_kind: Oppressor, bond_status: Active
  scene: 임충 산신묘 직후, 양산박행 도주 중
출력 후보:
  1. SystemicResistance(고구) feasibility 0.55 urgency 0.4 (양산박 합류로 가능)
  2. AvoidContact(고구) feasibility 0.95 urgency 0.2 (즉시 가능)
실제 선택: SystemicResistance (양산박 합류) — feasibility×urgency 곱이 더 큼.
```

→ 시스템이 *양산박 합류*를 자연스럽게 도출.

### 9.2 임충 → 고아내 (BloodEnemy)

```yaml
입력:
  bond_kind: BloodEnemy, bond_status: Active
  scene: 임충 산신묘 직후
출력 후보:
  1. DirectKill(고아내) feasibility 0.18 urgency 0.7 → blocked, deferred to SystemicResistance
  2. (deferred) SystemicResistance(고아내) feasibility 0.5 urgency 0.6 (양산박 활동 일환)
실제 선택: SystemicResistance (양산박 합류와 흡수)
```

→ 고아내 처단 욕구가 *체제 저항으로 흡수*되어 표출.

### 9.3 수련 노년 → 이모백 (Soulmate Deceased)

```yaml
입력:
  bond_kind: Soulmate, bond_status: Deceased
  scene: 옛 객점을 지남
status_filter: Active 후보 → 추모 후보로 변환 (SilentMonologue, HandleHeirloom)
회상 OCC 강도: 0.193 (relationships.md §4.5.3)
출력 후보:
  1. HandleHeirloom feasibility 0.95 urgency 0.193 (금비녀)
  2. SilentMonologue feasibility 0.98 urgency 0.193
실제 선택: 약한 강도, NPC가 *손이 금비녀로 갔다 내려놓음* — 가장 약한 형태의 HandleHeirloom.
```

→ 자연스러운 *작은 추모 동작*이 시스템에서 도출. voice_anchors의 "(금비녀에 손이 갔다 내려놓으며)"가 정확히 이 메커니즘.

### 9.4 노년기 수련 → 옥교룡 변경 재회 (Mentor Active 짧게)

```yaml
입력:
  bond_kind: Mentor, bond_status: Active (재회 직후)
  scene: 변경 옥교룡의 가정. 옥교룡 행복 상태.
출력 후보:
  1. OfferGuidance(옥교룡) moral_alignment 0.3 (compass "가두지 않는다"와 충돌) → 차단
     → deferred to WatchOver
  2. (deferred) WatchOver(옥교룡) feasibility 0.85 urgency 0.4
실제 선택: WatchOver — 떠나옴.
```

→ 수련의 *떠나옴*이 정확히 도출. compass의 행동 입증이 시스템적으로.

---

## 10. 다음 단계

본 문서가 정의하지 않는 것:

1. **NPC AI Layer의 후보 선택 룰** — 여러 ActionCandidate 중 *어느 하나*를 실제 실행할지. LLM 또는 결정론 룰. 별도 시스템.
2. **scene_context의 상세 변수 목록** — physical_access·power_balance 계산에 필요. game scene 시스템과 연동.
3. **bond_kind null인 관계의 행동 평가** — 자유 텍스트 type 기반 후보 도출. v0.7 후보.
4. **회상 OCC와 ActionTrigger의 정밀 통합** — 현재는 §9.3에서 단순 적용. 회상 강도 0.5+ 시 자동 후보 emit 룰 정밀화 필요.
5. **여러 NPC가 *같은 대상*에 대해 *동시에* ActionTrigger emit하는 케이스** — 협공, 집단 봉기. 다중 NPC 조정 필요.

---

## 변경 이력

| 버전 | 일자 | 변경 |
|------|------|------|
| v0.1 | 2026-05-04 | 초안. relationships.md v0.6에서 분리. ActionKind 29 variants, 5차원 feasibility 평가, 차단 시 변형 룰. v0.5 인스턴스 4개 검증 사례 (임충-고구, 임충-고아내, 수련-이모백 추모, 수련-옥교룡 변경 재회). |
