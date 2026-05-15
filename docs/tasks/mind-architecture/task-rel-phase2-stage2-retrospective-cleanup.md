# Phase 2 Stage 2 회고 정리 — W1~W4 통합

**파일**: `docs/tasks/mind-architecture/task-rel-phase2-stage2-retrospective-cleanup.md`
**v1.0 — 2026-05-16** (frozen)
**선행**:
  - `phase2-stage2-mapping.md` (회고)
  - Stage 2 commit (claude/relaxed-lichterman-80e860 worktree, 0506213 + d1e645c)
**브랜치 전략**: 별도 worktree 또는 Stage 2 commit 직후 같은 브랜치 — Bekay 결정.

---

## 1. 목표

회고 §"알려진 위험" W1~W4 위험 4개를 *단일 task*로 정리. 각 Stage 독립 commit + 통합 검증 게이트. 의미 변경 없음 — *방어 + 가시성 + 문서화* 위주.

---

## 2. 산출 일람

| Stage | 카테고리 | 산출 | LOC | 영향 파일 |
|---|---|---|---:|---|
| W1 | 회귀 가드 | Beat aspirational modifier 회귀 가드 3개 | ~50 | `relationship/mapping.rs` (tests) |
| W2 | 분류 문서화 | `is_negative_emotion` 분류 기준 박제 | ~16 + 문서 1단락 | `relationship/mapping.rs` + `relationships.md` v0.7 §4.3 |
| W3 | 디버깅 가시성 | BondStatus silent return tracing | ~7 | `relationship/mapping.rs:225~227` |
| W4 | 방어적 마커 | B-D12 호출자 인덱스 + 마커 + 가드 | ~34 | `relationship/mapping.rs` doc + 3 정책 위치 + tests |

**총**: ~107 LOC + ~70 LOC tests + 문서 1~2단락.

---

## 3. Stage 진입 의존

모두 *상호 독립*. 선행 = Stage 2 commit 1개.

**권장 순서**: **W1 → W4 → W2 → W3** (위험 무게 + 디버깅 인프라 우선순위 기준).

```
Stage 2 commit  ──┬── Stage W1 (회귀 가드 — Stage 3 진입 전 필수)
                  ├── Stage W4 (방어 마커 — 4번째 호출자 등장 전 보험)
                  ├── Stage W2 (분류 박제 — 6개월 후 코드 재해석 가드)
                  └── Stage W3 (debug 가시성 — Phase 7 진입 전 권장)
```

상호 독립이라 **어떤 순서로도 진행 가능** — 권장 순서는 의도된 우선순위일 뿐.

---

## 4. Stage W1 — Beat Aspirational Modifier 회귀 가드

### 4.1 배경

Stage 2가 `stimulus_policy::process_beat_transition`에 *aspirational view* 박음 — `beat_rel`에 `update_axes_from_emotion` 적용 후 `beat_rel.modifiers()`가 *변동된* affinity/trust로 산정되어 `appraise()`에 흐름. Two-phase relationship semantics ([stimulus_policy.rs:69~76](../../../src/application/command/policies/stimulus_policy.rs#L69) 주석)는 의도된 디자인이지만, *변화량이 spec 예측 범위 안*인지 보장하는 회귀 가드 부재.

회고 §S2가 *4축 수치*(trust 50→15.8 등)는 검증했으나, *그 4축 → `modifiers()` → `RelationshipModifiers` 4 필드*의 1 hop까지는 미검증. 본 Stage가 그 hop 추가.

### 4.2 결정적 단서 — `modifiers()`는 4축 중 2축만 사용

[`relationship/mod.rs:170~178`](../../../src/domain/relationship/mod.rs#L170) 구현:
- **affinity 채널** → `intensity_multiplier` / `empathy_multiplier` / `hostility_multiplier`
- **trust 채널** → `trust_modifier`
- `respect` / `wariness`는 미사용 (Phase 2.3 정밀화 대기)

→ 회귀 가드 표면: **affinity 채널 1 + trust 채널 1 + 누출 차단 1 = 테스트 3개**.

### 4.3 정량 임계값 결정

테스트 1·2의 정량 비교 임계값은 **`1e-3`** (절댓값 차).

채택 근거:
1. 회고 §S2가 ±0.1 정밀도로 4축 수치 박제했으므로 modifier 1 hop의 임계값을 *더 느슨하게* 풀면 §S2 정밀도 희석.
2. modifier 값 범위 [0.0, ~1.5] + 단일 곱+합 산식 → floating point noise ≤ 1e-6, 1e-3는 *noise보다 1000배 위*.
3. `expected` 계산이 *`profile()` weight를 직접 참조*하므로 weight 튜닝과 임계값 무관 — 1e-3 정밀도는 *base_delta 표 또는 hexaco_modifier 곱* 변화만 잡음.

### 4.4 산출 — 테스트 3개

위치: `src/domain/relationship/mapping.rs` tests 모듈.

#### 4.4.1 헬퍼 추가

```rust
// 진입 시 기존 tests의 헬퍼와 grep 후 *재사용* 우선. 부재 시 신규 박을 것.
fn lin_chong_hexaco() -> HexacoProfile {
    // 회고 §S2: Sincerity 0.7 / Forgiveness -0.7 / Prudence 0.8, 그 외 NEUTRAL
    // (정확한 builder API는 Stage 2 기존 tests의 패턴 follow)
    todo!("Stage W1 진입 시 fill in")
}

fn lin_chong_relationship_baseline() -> Relationship {
    RelationshipBuilder::new("lin_chong", "gao_qiu")
        .trust(AxisScore::new(50.0).unwrap())
        .affinity(AxisScore::new(40.0).unwrap())
        .respect(AxisScore::new(30.0).unwrap())
        .wariness(WarinessScore::new(5.0).unwrap())
        .build()
}
```

#### 4.4.2 Test 1 — Affinity 채널 (방향 + 정량)

```rust
#[test]
fn beat_rel_modifiers_affinity_channel_after_anger() {
    let hexaco = lin_chong_hexaco();
    let rel    = lin_chong_relationship_baseline();
    let mut beat_rel = rel.clone();

    update_axes_from_emotion(&mut beat_rel, EmotionType::Anger, 0.95, &hexaco);

    let before = rel.modifiers();
    let after  = beat_rel.modifiers();

    // (1) 방향 회귀
    assert!(after.intensity_multiplier < before.intensity_multiplier);
    assert!(after.empathy_multiplier   < before.empathy_multiplier);
    assert!(after.hostility_multiplier > before.hostility_multiplier);

    // (2) 정량 회귀 — 회고 §S2 affinity 28.6 / 100 = 0.286
    let p = profile();
    let expected = (1.0 + 0.286 * p.rel_closeness_intensity_weight).max(0.0);
    assert!((after.intensity_multiplier - expected).abs() < 1e-3,
        "drift: got {}, expected {}", after.intensity_multiplier, expected);
}
```

#### 4.4.3 Test 2 — Trust 채널 (정량)

```rust
#[test]
fn beat_rel_modifiers_trust_channel_after_anger() {
    let hexaco = lin_chong_hexaco();
    let rel    = lin_chong_relationship_baseline();
    let mut beat_rel = rel.clone();

    update_axes_from_emotion(&mut beat_rel, EmotionType::Anger, 0.95, &hexaco);

    let before = rel.modifiers();
    let after  = beat_rel.modifiers();

    assert!(after.trust_modifier < before.trust_modifier);

    // 회고 §S2 trust 15.8 / 100 = 0.158
    let p = profile();
    let expected = 1.0 + 0.158 * p.rel_trust_emotion_weight;
    assert!((after.trust_modifier - expected).abs() < 1e-3,
        "drift: got {}, expected {}", after.trust_modifier, expected);
}
```

#### 4.4.4 Test 3 — Living spec: respect/wariness 누출 차단

```rust
#[test]
fn beat_rel_modifiers_admiration_no_leak_until_phase_2_3() {
    // Admiration base_delta = { trust 0, affinity 0, respect +20, wariness 0 }
    // → modifier 4 필드 *완전 불변* 이어야 함.
    // Phase 2.3에서 respect를 modifier에 연결하면 *이 테스트가 깨지는 게 정상* —
    // "Phase 2.3 시작 시 spec 재확인" 신호.
    let hexaco = neutral_hexaco();
    let rel    = lin_chong_relationship_baseline();
    let mut beat_rel = rel.clone();

    update_axes_from_emotion(&mut beat_rel, EmotionType::Admiration, 0.7, &hexaco);

    let before = rel.modifiers();
    let after  = beat_rel.modifiers();

    assert_eq!(after.intensity_multiplier, before.intensity_multiplier);
    assert_eq!(after.trust_modifier,        before.trust_modifier);
    assert_eq!(after.empathy_multiplier,    before.empathy_multiplier);
    assert_eq!(after.hostility_multiplier,  before.hostility_multiplier);
}
```

### 4.5 게이트

- `cargo check --all-features` ✅
- `cargo test --lib --tests beat_rel_modifiers` → 3 PASS
- `cargo test --lib` 전체 → +3
- Manual recompute: Test 1·2 expected 수치가 회고 §S2 4축 값과 modifier 산식의 *기계적 곱* 결과와 일치 (Bekay 1회 손 검산)
- `cargo test --features chat --lib --tests` → +3 (chat 모드 회귀 없음)

### 4.6 완료 조건

- [ ] 3 테스트 PASS
- [ ] 헬퍼 2개 박힘 (또는 기존 헬퍼 재사용)
- [ ] `cargo test --lib` +3 / `cargo test --features chat --lib --tests` +3
- [ ] commit message 회고 §W1 참조 + Test 3의 "Phase 2.3 진입 시 깨질 예정" 의도 명시

### 4.7 알려진 작은 위험 (Stage W1 자체)

- **헬퍼 중복**: `lin_chong_hexaco`가 Stage 2 기존 tests에 *유사 헬퍼* 있을 가능성 — 진입 시 grep으로 *재사용* 우선.
- **`profile()` 의존**: tuning weight 값이 *런타임 dependent*. test profile이 다르면 expected 수치 어긋날 수 있음. 대안: weight를 *직접 inject*하는 helper로 우회 — Stage 2 기존 패턴 follow.

---

## 5. Stage W2 — `is_negative_emotion` 분류 박제

### 5.1 배경

`mapping.rs::is_negative_emotion` (11 OCC 감정 enumeration)이 *OCC valence*가 아닌 ***4축 base_delta의 affinity 부호***를 기준으로 분류. 결정 자체는 *서사 직관과 일치* (회고 §W2 분석). 그러나:

- 결정 *기준*이 코드에 *암묵적으로 박힘* — 함수 body는 enumeration만, 기준은 회고 §W2에만 존재
- spec §4.3 본문이 "부정 감정" 정의를 *명시하지 않음*
- 미래 변경 시 누군가 OCC valence 기준으로 Pity를 추가하면 *서사 의도 정반대* (Forgiveness 낮은 NPC의 동정심이 ×1.5 증폭)

W2 위험은 *동작*이 아니라 *해석 표면*. 가드는 (a) 코드 doc, (b) living spec test, (c) 문서 본문 — 3중.

### 5.2 결정적 단서 — 공감 군 4감정의 비대칭

`base_delta` 표에서 affinity 부호로 봤을 때:

| 공감군 감정 | affinity | 부정 분류 | 서사 직관 ✅ |
|---|---:|:---:|---|
| HappyFor | +10 | 제외 | 인색한 사람의 *남의 행복 기뻐함* 증폭 안 됨 |
| Pity | +10 | **제외** ← 핵심 | 인색한 사람의 *동정심* 증폭 안 됨 |
| Gloating | −20 | 포함 | 인색한 사람의 *고소함* 증폭 |
| Resentment | −10 | 포함 | 인색한 사람의 *원망* 증폭 |

→ Pity 분류가 *전체 시스템의 의도된 비대칭*의 한 부분.

### 5.3 산출

#### 5.3.1 함수 doc 보강 ([`mapping.rs:179~`](../../../src/domain/relationship/mapping.rs#L179) 부근)

```rust
/// 부정 감정 판정 (A− Forgiveness 룰 적용 대상).
///
/// **분류 기준**: 본 함수는 *4축 base_delta의 affinity 부호*를 기준으로 한다.
/// OCC valence(사건-반응의 호/오)와 *다를 수 있음*에 유의:
/// - Pity는 OCC valence상 *부정*(남의 불운에 대한 반응)이지만
///   `base_delta(Pity).affinity = +10` 이므로 *제외*된다.
///
/// **결정 근거**: A− Forgiveness 룰의 ×1.5 증폭이 *관계 충격(4축 affinity 감소)이
/// 큰 감정*에만 적용되어야 서사 직관과 일치. 예) 인색한 사람의 동정심 증폭은
/// "인색하지만 더 깊은 동정"이라는 *반대 모순* 발생.
///
/// 회고 §W2 + spec §4.3 참조.
///
/// **Phase 2.3 narrative 검증 항목**: 공감 군 4감정(HappyFor / Pity / Gloating /
/// Resentment)에 A− Forgiveness 적용 결과가 서사 직관과 일치하는지 시뮬.
fn is_negative_emotion(e: EmotionType) -> bool { ... }
```

#### 5.3.2 Living spec 테스트

위치: `mapping.rs` tests 모듈.

```rust
#[test]
fn is_negative_emotion_classification_matches_affinity_sign_basis() {
    // "affinity 부호 기준" 결정을 living spec으로 박제.
    // OCC valence 기준으로 분류 변경 시 즉시 깨짐 → 회고 §W2 + doc + §4.3 재독.

    // affinity + 공감 감정 — 부정 분류 *제외*
    assert!(!is_negative_emotion(EmotionType::HappyFor));
    assert!(!is_negative_emotion(EmotionType::Pity),
        "Pity: OCC valence 부정이지만 affinity +10이라 제외 (회고 §W2)");

    // affinity − 공감 감정 — 부정 분류 *포함*
    assert!(is_negative_emotion(EmotionType::Gloating));
    assert!(is_negative_emotion(EmotionType::Resentment));

    // 11 enumeration sanity (변경 검출용)
    assert!(is_negative_emotion(EmotionType::Anger));
    assert!(is_negative_emotion(EmotionType::Hate));
    assert!(is_negative_emotion(EmotionType::Distress));
}
```

#### 5.3.3 spec §4.3 본문 정정

위치: `docs/game-design/2-characters/relationships.md` v0.7 §4.3 (또는 frozen이면 후속 spec).

추가 단락:

> **"부정 감정"의 정확한 정의**: A− Forgiveness 룰이 적용되는 *부정 감정*은
> **4축 base_delta의 affinity 부호가 음(−)인 감정**으로 정의한다. OCC valence
> 와 다를 수 있으며, 특히 Pity는 OCC valence상 부정이지만 affinity +10이라
> *제외*된다.
>
> 채택 근거: A− Forgiveness 룰의 ×1.5 증폭이 *관계 충격이 큰* 감정에만 적용
> 되어야 "인색한 NPC의 동정심이 더 강함" 같은 *서사 반대 효과*를 막을 수 있다.
>
> 현재 부정 감정 enumeration (11개): Anger / Reproach / Resentment / Gloating /
> Hate / Distress / Fear / Disappointment / FearsConfirmed / Shame / Remorse.
>
> 공감 군 4감정의 적용 결과 검증: Phase 2.3 narrative 시뮬에서 확인.

### 5.4 게이트

- `cargo check --all-features` ✅
- `cargo test --lib --tests is_negative_emotion_classification` → 1 PASS
- `cargo test --lib` 전체 → +1
- `relationships.md` markdown lint ✅

### 5.5 완료 조건

- [ ] 함수 doc 4단락 박힘
- [ ] Living spec 테스트 PASS
- [ ] spec §4.3 단락 박힘
- [ ] commit message 회고 §W2 참조 + Phase 2.3 narrative 검증 항목 명시

### 5.6 알려진 작은 위험

- **§4.3 박는 위치**: `relationships.md` v0.7이 *frozen* 상태라면 본문 직접 수정 vs 후속 spec(v0.8) — Bekay 결정. v0.7 frozen이면 v0.8 또는 별도 *분류 정의 부록* 문서가 깔끔.

---

## 6. Stage W3 — Silent Return Tracing

### 6.1 배경

[`mapping.rs:225~227`](../../../src/domain/relationship/mapping.rs#L225) BondStatus 차단 silent return — 의도된 동작이지만 *디버깅 단서 0*. 차단 빈도는 production에서 낮음 (Dormant/Deceased/Resolved 모두 드뭄). 그러나 *디버깅 필요 시점의 통증은 큼*. 비용 ≈ 0의 `tracing::debug!` 1줄로 해결.

### 6.2 결정적 단서 — 호출 측 책임 분리는 유지

현재 호출 측이 skip 발생을 *알아야 할 시나리오 없음*. 옵션 B(시그니처 변경) 보류, 옵션 A(debug 로그)로 충분. Phase 7 또는 stream-migration 중 *호출 측이 skip 결과를 활용해야 할* 필요 등장 시 옵션 B로 escalate.

### 6.3 산출

#### 6.3.1 `mapping.rs:225~227` 변경

```rust
// ── 가드: BondStatus 차단 ─────
if !rel.bond_status().accepts_live_input() {
    tracing::debug!(
        owner = %rel.owner_id(),
        target = %rel.target_id(),
        emotion = ?emotion,
        bond_status = ?rel.bond_status(),
        intensity = intensity,
        "update_axes_from_emotion skipped: bond_status blocks live input"
    );
    return;
}
```

#### 6.3.2 회귀 가드 — 기존 테스트 grep 후 보강

회고 §2.7에 "BondStatus 4 variants" 테스트 박힘. 진입 시 grep으로 확인:

```bash
rg -n 'BondStatus::Dormant|BondStatus::Deceased|BondStatus::Resolved' \
    src/domain/relationship/mapping.rs
# 예상: tests 모듈에 3+ hits
```

- **존재**: 추가 작업 0
- **부재**: 4 variants(Active / Reactivating / Dormant / Deceased / Resolved 해당) 각각 silent return 회귀 테스트 추가

#### 6.3.3 수동 verify 1회

```bash
RUST_LOG=npc_mind_rs::domain::relationship::mapping=debug \
    cargo test --lib <적절한 dormant 테스트 이름> -- --nocapture
```

(test name은 §6.3.2 grep 결과 또는 신규 박은 회귀 가드 이름으로 대체)

예상 출력 (대략):
```
DEBUG ... update_axes_from_emotion skipped: bond_status blocks live input
      owner=lin_chong target=gao_qiu emotion=Anger
      bond_status=Dormant intensity=0.95
```

### 6.4 게이트

- `cargo check --all-features` ✅
- `cargo test --lib --tests` 전체 → 변동 0 또는 +N(보강 시)
- 수동 verify: RUST_LOG 1회 실행 후 Bekay check

### 6.5 완료 조건

- [ ] `tracing::debug!` 1줄 박힘
- [ ] (조건부) BondStatus 4 variants 회귀 가드 grep 확인 후 부재 시 보강
- [ ] 수동 RUST_LOG verify 1회
- [ ] commit message 회고 §W3 참조 + 옵션 B escalate 트리거(호출 측 skip 활용 필요) 미정 명시

### 6.6 알려진 작은 위험

- **로그 폭주**: 시나리오에 *Dormant 다수 + 매 Beat 전환 = 전 NPC 루프*면 debug 출력 폭주 가능. 그러나 debug는 production 기본 비활성이므로 영향 0. 강등 필요 시 DEBUG → TRACE — Bekay 결정.
- **호출 빈도 측정 부재**: 차단이 *비정상적으로 자주* 일어나는지 production metric으로 잡으려면 추가 hook 필요 — 작업 면적 밖. Phase 2.3 metrics 작업 시 검토.

---

## 7. Stage W4 — B-D12 호출자 마커

### 7.1 배경

B-D12 (Pride/Shame agent_id=None) 가드가 *호출 측 명시 패턴*으로 3 위치에 분산. spec §4 결정 — 의식적 선택, 함수 책임 경계 보존. 그러나 *4번째 호출자 추가 시 누락 위험*. Phase 3a/3b의 Channel 2 Temporal / Channel 3 External 등 새 입력 채널이 등장하면 새 `iter_active()` 루프가 복사되며 가드 라인을 *잊을 가능성*.

옵션 A(예방 마커)로 *지금* 누락 위험만 낮춤. 4번째 호출자 등장 시 옵션 B(`Relationship::accepts_axis_update_for(emotion)` 헬퍼)로 escalate.

### 7.2 결정적 단서

- 현재 호출자 *정확히 3*: 
  - [`relationship_policy::handle_relationship_update_with_cause:143`](../../../src/application/command/policies/relationship_policy.rs#L143)
  - [`relationship_policy::handle_dialogue_end:238`](../../../src/application/command/policies/relationship_policy.rs#L238)
  - [`stimulus_policy::process_beat_transition:78`](../../../src/application/command/policies/stimulus_policy.rs#L78)
- 새 호출자 추가 흐름은 보통 *기존 위치 grep → 복사 → 수정*. 이 흐름에 *마커가 따라오면* 누락 차단.

### 7.3 산출

#### 7.3.1 `update_axes_from_emotion` doc 호출자 인덱스 ([`mapping.rs:217~218`](../../../src/domain/relationship/mapping.rs#L217))

기존 doc 끝에 추가:

```rust
/// ## 호출자 인덱스 (B-D12 가드 *필수* 위치)
///
/// 본 함수를 *새 위치에서* 호출할 때는 반드시 다음 패턴을 함께 박을 것:
/// ```rust,ignore
/// // B-D12 guard: Pride/Shame are self-emotions, no target-relationship semantics.
/// // If this loop is duplicated to a new caller, this guard MUST be copied.
/// if matches!(emotion_type, EmotionType::Pride | EmotionType::Shame) {
///     continue;
/// }
/// update_axes_from_emotion(&mut rel, emotion_type, intensity, hexaco);
/// ```
///
/// 현재 호출자 (4번째 추가 시 본 리스트 갱신 + §7.3.2 마커 복사):
/// - `application::command::policies::relationship_policy::handle_relationship_update_with_cause`
/// - `application::command::policies::relationship_policy::handle_dialogue_end`
/// - `application::command::policies::stimulus_policy::process_beat_transition`
///
/// 회고 §W4 + spec §7 참조.
pub fn update_axes_from_emotion(...) { ... }
```

#### 7.3.2 3 호출 측 마커 주석

`relationship_policy.rs:143` / `relationship_policy.rs:238` / `stimulus_policy.rs:78` 각각 *동일 문구*:

```rust
// B-D12 guard: Pride/Shame are self-emotions, no target-relationship semantics.
// If this loop is duplicated to a new caller, this guard MUST be copied.
// See mapping.rs::update_axes_from_emotion doc § "호출자 인덱스".
if matches!(emotion_type, EmotionType::Pride | EmotionType::Shame) {
    continue;
}
update_axes_from_emotion(&mut beat_rel, emotion_type, intensity, hexaco);
```

3 위치 *동일 문구* 강제 — 새 위치 복사 시 *주석 그대로 따라옴* → 가드 누락 차단.

#### 7.3.3 Living spec 회귀 가드

```rust
#[test]
fn update_axes_from_emotion_does_not_filter_pride_or_shame_internally() {
    // B-D12 가드는 *호출 측 책임* (spec §4) — 본 함수는 Pride/Shame이 직접
    // 전달되면 *base_delta 그대로 4축을 변동*해야 한다.
    // 누군가 함수 안에 `matches!(Pride|Shame) return;` 박으면 이 테스트가
    // 깨지며 spec §4 + 회고 §W4 재독 후 결정 재확인 강제.
    let hexaco = neutral_hexaco();
    let mut rel = RelationshipBuilder::new("a", "b")
        .trust(AxisScore::new(50.0).unwrap())
        .affinity(AxisScore::new(40.0).unwrap())
        .build();
    let affinity_before = rel.affinity();

    update_axes_from_emotion(&mut rel, EmotionType::Pride, 0.8, &hexaco);

    // base_delta(Pride) = { trust 0, affinity +5, respect +10, wariness 0 }
    // → affinity 변동 발생해야 함 (함수 안에서 차단 안 됨)
    assert_ne!(rel.affinity(), affinity_before,
        "함수 자체는 Pride/Shame 차단하지 *않음* (spec §4 결정)");
}
```

### 7.4 게이트

- `cargo check --all-features` ✅
- `cargo test --lib --tests update_axes_from_emotion_does_not_filter` → 1 PASS
- `cargo test --lib` 전체 → +1
- grep verify:
  ```bash
  rg -n 'B-D12 guard' src/application/command/policies/
  # 예상: 정확히 3 hits

  rg -n '호출자 인덱스' src/domain/relationship/mapping.rs
  # 예상: 1 hit (함수 doc)
  ```

### 7.5 완료 조건

- [ ] doc 호출자 인덱스 추가
- [ ] 3 호출 측 마커 주석 박힘 (grep 3 hits)
- [ ] Living spec 회귀 테스트 PASS
- [ ] commit message 회고 §W4 참조 + "4번째 호출자 등장 시 옵션 B(`accepts_axis_update_for`) escalate" 미래 트리거 명시

### 7.6 알려진 작은 위험

- **마커 텍스트의 일관성**: 3 위치 *동일 문구* 강제 — 한 위치에서 *살짝* 수정되면 grep가 해당 위치만 누락. 대안: Rust attribute 매크로 또는 정확한 문자열 lint — 작업 면적 밖. 4번째 호출자 등장 시 옵션 B로 escalate되며 자동 해결.

---

## 8. 누적 검증 게이트

각 Stage 후 + 최종 통합 검증.

**Stage 누적별 테스트 카운트 (Stage 2 baseline 866 기준, 권장 순서 W1 → W4 → W2 → W3)**:

| Stage 진입 후 | `cargo test --lib` | 비고 |
|---|---|---|
| W1 후 | 866 → 869 (+3) | regression guard 3개 |
| W1 + W4 후 | 869 → 870 (+1) | living spec guard 1개 |
| W1 + W4 + W2 후 | 870 → 871 (+1) | living spec guard 1개 |
| W1 + W4 + W2 + W3 후 | 871 → 871 (+0~N) | W3 회귀 가드 grep 후 부재 시 보강 +N |

**최종 통합 게이트**:

| # | 게이트 | 목표 |
|---|---|---|
| 1 | `cargo check --all-features` | ✅ PASS |
| 2 | `cargo test --lib --tests` | 866 → 871 (최소) |
| 3 | `cargo test --features chat --lib --tests` | 866 → 871 (최소, chat 모드 동일) |
| 4 | Bench regression | ±5% 이내 (`benches/` 전체 평균) |
| 5 | grep verify | §9 전체 통과 |

---

## 9. Spec 가정 검증 요구사항 (grep)

**Stage W1 진입 전**:
```bash
# beat_rel.modifiers() 호출 지점 1곳 확인
rg -n 'beat_rel\.modifiers\(\)' src/application/command/policies/
# 예상: 1 hit @ stimulus_policy.rs:~84

# fn modifiers() — 4축 중 trust/affinity만 사용 재확인
rg -n 'fn modifiers' src/domain/relationship/mod.rs
# 예상: 1 hit, body에 affinity_norm + trust_norm만
```

**Stage W2 진입 전**:
```bash
# is_negative_emotion 함수 위치 + 호출자 확인
rg -n 'is_negative_emotion' src/domain/relationship/mapping.rs
# 예상: 1 fn def + 1 caller (hexaco_modifier 내)
```

**Stage W3 진입 전**:
```bash
# 기존 BondStatus 4 variants 회귀 가드 존재 여부
rg -n 'BondStatus::Dormant|BondStatus::Deceased|BondStatus::Resolved' \
    src/domain/relationship/mapping.rs
# 예상: tests 모듈에 3+ hits — 존재 시 추가 작업 0
```

**Stage W4 진입 전**:
```bash
# B-D12 가드 3 hits 일치 확인
rg -n 'matches!\(.*Pride.*Shame.*\)' src/application/command/policies/
# 예상: 정확히 3 hits — relationship_policy.rs:143, 238 + stimulus_policy.rs:78
```

**Stage W4 완료 후**:
```bash
# 마커 주석 3 hits 확인
rg -n 'B-D12 guard' src/application/command/policies/
# 예상: 정확히 3 hits

# 함수 doc 호출자 인덱스 박힘 확인
rg -n '호출자 인덱스' src/domain/relationship/mapping.rs
# 예상: 1 hit (함수 doc)
```

---

## 10. Push 단위 + Commit Message 템플릿

4 commit 분리 (회고 §W1~§W4와 1:1). Bekay가 직접 commit 실행 — Claude는 message text만 제공.

### Stage W1 commit message 초안

```
test(phase2-stage2): W1 회귀 가드 — Beat aspirational modifier 변화량 박제

회고 §W1 추적. Stage 2의 `beat_rel` aspirational view가 만드는
`modifiers()` 변화가 spec 예측 범위 안인지 회귀 가드 박제.

3 테스트 추가:
- Test 1 — affinity 채널 (intensity/empathy/hostility) 방향+정량 (±1e-3)
- Test 2 — trust 채널 정량 (±1e-3)
- Test 3 — Living spec: respect/wariness 누출 차단 (Phase 2.3 시작 시 깨짐 예정)

S2 임충 베이스라인 (trust=50, affinity=40, Anger 0.95) 사용.

cargo test --lib 866 → 869 PASS
```

### Stage W4 commit message 초안

```
chore(phase2-stage2): W4 — B-D12 호출자 마커 + living spec guard

회고 §W4 추적. 호출 측 분산 패턴(spec §4 결정) 유지 + 4번째 호출자
누락 방지 마커 박기:
- `update_axes_from_emotion` doc에 "호출자 인덱스" 박음
- 3 호출 측에 동일 마커 주석 — grep 'B-D12 guard' 3 hits
- Living spec: 함수가 Pride/Shame 차단 *안 함* 회귀 가드

4번째 호출자 등장 시 옵션 B (Relationship::accepts_axis_update_for)
escalate 트리거 명시.

cargo test --lib +1 PASS
```

### Stage W2 commit message 초안

```
docs+test(phase2-stage2): W2 — is_negative_emotion 분류 기준 박제

회고 §W2 추적. "부정 감정" 정의가 *4축 base_delta affinity 부호 기반*
임을 (a) 코드 doc + (b) living spec test + (c) spec §4.3 본문에 박제.

Pity는 OCC valence 부정이지만 affinity +10이라 *제외*된다는
결정 근거 박힘 — Phase 2.3 narrative 검증 시 공감 군 4감정 확인.

cargo test --lib +1 PASS
```

### Stage W3 commit message 초안

```
chore(phase2-stage2): W3 — BondStatus silent return tracing

회고 §W3 추적. `update_axes_from_emotion`의 BondStatus 차단 silent return에
`tracing::debug!` 1줄 추가. 옵션 B(시그니처 변경) 보류 — 호출 측이 skip을
활용해야 할 시나리오 등장 시 escalate.

RUST_LOG=...=debug 수동 verify 1회 완료.
```

---

## 11. 진입 전 / 후 체크리스트

**전체 task 진입 전**:
- [ ] 회고 `phase2-stage2-mapping.md` §"알려진 위험" 재독
- [ ] Stage 2 commit 박힌 상태 (default + chat features 모두 866 PASS)
- [ ] 작업 worktree 결정 (별도 vs 동일)

**각 Stage 진입 전**:
- [ ] §9 해당 stage grep 결과 일치
- [ ] 회고 §해당 W번호 단락 재독
- [ ] 본 spec §해당 stage 본문 재독

**각 Stage 완료 후**:
- [ ] 해당 §"게이트" 통과
- [ ] 해당 §"완료 조건" 모두 ✅
- [ ] commit message에 §10 템플릿 사용 + 회고 §W번호 참조

**전체 task 완료 후**:
- [ ] §8 누적 게이트 5개 모두 통과
- [ ] §9 grep 검증 전체 통과
- [ ] 본 spec frozen → push 후 `phase2-stage2-mapping.md`의 §"알려진 위험" W1~W4 *해결 박제* 후속 회고 1단락 추가 (또는 본 spec의 *완료 회고* 별도 파일)
