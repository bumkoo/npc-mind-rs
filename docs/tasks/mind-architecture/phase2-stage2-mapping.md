# Phase 2 Stage 2 회고 — OCC → 4축 매핑

**일자**: 2026-05-15
**Spec**: `task-rel-phase2-domain-migration.md` §7 Stage 2 (v1.2 spec frozen)
**브랜치**: `claude/relaxed-lichterman-80e860` (worktree)
**선행**: Stage 1 (`phase2-stage1` v1.1 spec) — `feat(phase2-stage1): Relationship 도메인 4축 마이그레이션` (0506213) + ultrareview fix (d1e645c)
**산출**: 4축 자동 변동 *최초 활성*. 정책 3 위치의 임시 no-op (`// TODO(Stage 2)`) 제거 + `update_axes_from_emotion` 진입점 박힘.

## 작업 내역 (spec 2.1~2.7 1:1 대응)

### 2.1 — `mapping.rs` 신설 + `AxisModifier` 박기

| 변경 | 파일 | 라인 |
|---|---|---|
| 신설 모듈 | [src/domain/relationship/mapping.rs](../../../src/domain/relationship/mapping.rs) | — |
| `AxisModifier` 구조체 + `Default` | [src/domain/relationship/axis.rs](../../../src/domain/relationship/axis.rs) | 137~191 |
| `mod mapping;` + `pub use AxisModifier` + `pub use update_axes_from_emotion` | [src/domain/relationship/mod.rs](../../../src/domain/relationship/mod.rs) | 20~27 |

가시성: `update_axes_from_emotion` → `pub` (Stage 3 정책 진입점), `base_delta` / `hexaco_modifier` → `pub(crate)` (내부 헬퍼), `AxisModifier` → `pub` (axis.rs 데이터 모듈 일관).

### 2.2 — `base_delta(EmotionType) -> AxisDelta` 48셀 lookup

[mapping.rs:32~115](../../../src/domain/relationship/mapping.rs:32) — `match` 표현식으로 12 OCC explicit + `_ => AxisDelta::default()` fallback. v0.7 §4.2 표값 그대로 박힘:

| OCC | trust | affinity | respect | wariness |
|---|---:|---:|---:|---:|
| Gratitude | +20 | +10 | 0 | −10 |
| Anger | −25 | −10 | 0 | +25 |
| Admiration | 0 | 0 | +20 | 0 |
| Reproach | −10 | −10 | −25 | +10 |
| HappyFor | +5 | +10 | 0 | 0 |
| Resentment | 0 | −10 | −5 | +15 |
| Pity | 0 | +10 | −5 | 0 |
| Gloating | −10 | −20 | −10 | 0 |
| Pride | 0 | +5 | +10 | 0 |
| Shame | −5 | −10 | −10 | +5 |
| Love | +5 | +20 | +5 | −5 |
| Hate | −10 | −25 | −5 | +15 |

10 OCC 의도된 누락 (B-D14): Joy / Distress / Hope / Fear / Satisfaction / Disappointment / Relief / FearsConfirmed / Remorse / Gratification.

### 2.3 — `AxisModifier` 메서드 + `hexaco_modifier` 6 보정 룰

`AxisModifier` 메서드 (axis.rs):
- `combine_uniform(self, factor: f32) -> Self` — 전역 곱셈 (Patience/Prudence/Forgiveness 룰)
- `scale_axis(mut self, kind: AxisKind, factor: f32) -> Self` — 축별 곱셈 (Sincerity → trust / Anxiety → wariness)

`hexaco_modifier` (mapping.rs:117~167):
- HIGH_THRESHOLD = 0.5 / LOW_THRESHOLD = −0.5
- 5 룰 활성: H+ Sincerity / A+ Patience / A− Forgiveness (negative emotion only) / E+ Anxiety / C+ Prudence
- O+ Unconventionality는 placeholder (적용 0) — Phase 2.3 또는 3+ 정밀화 예정
- `is_negative_emotion` 헬퍼 (11 emotion: Anger / Reproach / Resentment / Gloating / Hate / Distress / Fear / Disappointment / FearsConfirmed / Shame / Remorse)

### 2.4 — `update_axes_from_emotion` 단일 함수

[mapping.rs:175~199](../../../src/domain/relationship/mapping.rs:175):

```rust
pub fn update_axes_from_emotion(rel, emotion, intensity, hexaco) {
    if !rel.bond_status().accepts_live_input() { return; }
    let base = base_delta(emotion);
    let modulator = hexaco_modifier(emotion, hexaco);
    let delta = AxisDelta { trust: base.trust * intensity * modulator.trust, ... };
    rel.apply_delta(&delta);
}
```

BondStatus 차단 진입 가드 + 인라인 곱셈 (별도 helper 없음). B-D12 (Pride/Shame agent_id=None) 가드는 *호출 측* 책임 (2.6).

### 2.5 — `RelationshipModifiers`: 변경 0

spec 2.5의 *작업 면적 0* 결정 그대로. Stage 0 A2 가정 (`closeness_*` 4 필드)이 *옛 분석 자료에 의한 오해*였고, 실제 Stage 1 코드의 `RelationshipModifiers`는 이미 추상 이름 (intensity/trust/empathy/hostility)으로 박혀있었음. Stage 1에서 *입력 swap*만 적용 (closeness_norm → affinity_norm)으로 시그니처 보존. 5곳 사용처 시맨틱 자동 흡수.

Phase 2.3로 이관: `RelationshipModifiers` 필드 정밀화 (respect_modifier / wariness_modifier 신설 검토) + tuning profile rename (`rel_closeness_*_weight` → `rel_affinity_*_weight`) + `profile.toml` 마이그레이션.

### 2.6 — 정책 3 위치 표준 호출 + B-D12 가드

| 위치 | 변경 |
|---|---|
| [relationship_policy.rs:130~150](../../../src/application/command/policies/relationship_policy.rs:130) (`handle_relationship_update_with_cause`) | `iter_active()` 루프 + B-D12 가드 + `update_axes_from_emotion` 호출 + TODO(Stage 2) 주석 제거 |
| [relationship_policy.rs:222~235](../../../src/application/command/policies/relationship_policy.rs:222) (`handle_dialogue_end` outer-loop branch) | 동일 패턴. `ctx.get_npc(npc_id)?` 추가 |
| [stimulus_policy.rs:71~84](../../../src/application/command/policies/stimulus_policy.rs:71) (`process_beat_transition`) | `stimulated.iter_active()` 루프 + B-D12 가드. `beat_rel.modifiers()` 사용 보존 |

`use` 추가:
- relationship_policy.rs: `crate::domain::emotion::EmotionType` + `crate::domain::relationship::update_axes_from_emotion`
- stimulus_policy.rs: `EmotionType`, `update_axes_from_emotion`

Stage 1 ±1.0 contract (affinity/100, trust/100) 보존 — Stage 3 payload 6→8 확장 시 정리.

### 2.7 — 단위 테스트 ~42개 추가

| 파일 | 케이스 | 추가 |
|---|---|---|
| `axis.rs` (Stage 1.9 tests에 추가) | AxisModifier default/combine_uniform/scale_axis/chained | +4 |
| `mapping.rs` (신설 tests) | base_delta 12 OCC + 10 default + S2 합산 + hexaco_modifier 7 (single + composite + neutral + is_negative) + update_axes_from_emotion 10 (정상 / intensity 0 / S2 / clamp / BondStatus 4 variants / unmapped) | +33 |
| `relationship_policy.rs` (기존 tests에 추가) | Gratitude 정상 / B-D12 Pride+Shame skip / 혼합 (Pride skip + Anger 적용) / Deceased 차단 / empty emotion | +5 |
| `stimulus_policy.rs` (기존 tests에 추가) | Pride-only Beat 전환 (B-D12) / Gratitude Beat 전환 정상 | +2 |
| **합계** | | **+44** |

Spec 추정 ~42에 근접 (+44 박음).

## Stage 0 §3.6 S2 임충 수치 정합 검증

[mapping.rs:617~649](../../../src/domain/relationship/mapping.rs:617) `update_axes_s2_lin_chong_anger_alone`:

| 축 | before | delta | after | 기대 | 결과 |
|---|---:|---:|---:|---:|---|
| trust | 50.0 | −34.2 | 15.8 | ≈ 16 (±0.1) | ✅ |
| affinity | 40.0 | −11.4 | 28.6 | ≈ 29 (±0.1) | ✅ |
| respect | 30.0 | 0.0 | 30.0 | 30 (±0.1) | ✅ |
| wariness | 5.0 | +28.5 | 33.5 | ≈ 34 (±0.1) | ✅ |

계산 경로:
- base_delta(Anger): `{trust −25, affinity −10, respect 0, wariness +25}`
- hexaco_modifier(Anger, 임충): Sincerity 0.7 → trust ×1.2 → Forgiveness −0.7 + negative → 전역 ×1.5 → Prudence 0.8 → 전역 ×0.8 → 최종 `{trust 1.44, affinity 1.2, respect 1.2, wariness 1.2}`
- delta = base × 0.95 × modulator = `{−34.2, −11.4, 0, +28.5}`
- apply_delta + clamp ±100 / 0..100 → `{15.8, 28.6, 30.0, 33.5}` ✅

## Spec 가정 정정 사례

| Spec 가정 | 현실 | 어디서 잡힘 |
|---|---|---|
| 1.4 `BondStatus::Reactivating { trigger: EventId }` — EventId가 `pub type EventId = u64` (domain/event.rs) | `bond.rs`가 use하는 EventId는 `domain/world/event.rs::EventId(pub String)` newtype | Stage 2.7 mapping.rs 테스트 컴파일 에러 |
| 2.5 `RelationshipModifiers` 4 필드 = `closeness_*` (Stage 0 A2) | Phase 1 1.5 즈음에 이미 추상 이름 (intensity/empathy/hostility)으로 리네임됨 | Stage 1 코딩 + 2.5 spec freeze 시 검증 |
| 2.2~2.4 `OccEmotion` enum (Stage 0 작성 시 명명) | 실재 코드 `EmotionType` 22 variants (의미는 동일) | Stage 2.6 spec freeze 직전 grep |

**교훈**: Stage 0 A 카테고리 (코드 사실 조사)는 *Phase 진행 중 변경된 부분에 대해 재확인 필요*. 본 Stage 2에서는 spec 작성 시 `Score::new(0.7)` API가 실제 `Score::new(v, &field)`인 점도 *구현 시점*에 발견.

## 컴파일 + 테스트 게이트

| # | 게이트 | 결과 |
|---|---|---|
| 1 | `cargo check --all-features` | ✅ PASS (1 warning — pre-existing `reflection_service.rs:30` unused imports) |
| 2 | `cargo test --lib` (default features) | ✅ 545 passed / 0 failed |
| 3 | `cargo test --features chat --lib --tests` | ✅ **866 passed / 0 failed / 5 ignored** — `baselines/stage2-cargo-test-2026-05-15-chat-PASS.log` 박제 |
| 4 | `cargo test --all-features` | ⚠️ Windows CRT 충돌 (embed + ort 정적 링크 — CLAUDE.md 박힌 환경 이슈, `CFLAGS=/MD` 셸 설정 + `cargo clean` 필요). examples 일부도 embed 요구로 컴파일 실패. Stage 1 baseline 1220은 embed 포함 환경 기준 — 본 worktree에서는 `--lib --tests`로 우회 |

## Stage 3 진입 전제 (확인됨)

1. ✅ `update_axes_from_emotion` 외부 진입점 활성. base_delta × intensity × hexaco_modifier × BondStatus 가드 일체 동작.
2. ✅ B-D12 가드 *호출 측*에서 박힘 (Pride/Shame skip) — 3 정책 위치.
3. ✅ BondStatus Deceased/Resolved/Dormant 차단 + Active/Reactivating 통과 — 단위 테스트로 검증.
4. ✅ S2 임충 케이스 수치 정합 (Anger 단독: trust 50→16, affinity 40→29, wariness 5→34 ±0.1).
5. ⚠️ Stage 1 ±1.0 contract (`affinity/100`, `trust/100`) 보존 — RelationshipUpdatedPayload schema가 ±1.0이라 정규화 layer 남음. Stage 3에서 payload 6→8 확장 시 정리.

## Stage 3에서 다룰 항목 (본 stage 비포함)

- `RelationshipUpdatedPayload` 6 → 8 필드 (`closeness_*`/`power_*` → `affinity_*`/`respect_*`/`wariness_*`)
- DTO 변환 (`dialogue_orchestrator.rs`, `domain_sync.rs`, `dialogue_test_service.rs`)
- SSE bridge (`event_bridge.rs`) 매핑 갱신
- Mind Studio frontend 4축 표시
- ±1.0 contract 정규화 layer 제거

## 발견 사항

1. **`relationship_policy.rs` 기존 5 테스트 영향**: `handle_relationship_update_with_cause` + `handle_dialogue_end`에 `ctx.get_npc(npc_id)?` 추가로 인해 5개 기존 테스트가 NpcNotFound로 실패. `with_npc(neutral_npc("alice"))` 추가로 복구. 모듈 최상단에 `neutral_npc(id)` 헬퍼 함수 박음.

2. **HEXACO API 정합**: `hexaco.honesty_humility.sincerity.value()` 형태 spec 가정이 *실제 API와 정합*. `domain/personality.rs:215~222` HexacoProfile 6 dimension + 각 4 facet 구조 그대로.

3. **`EmotionState::iter_active` API 정합**: spec 가정 `impl Iterator<Item = (EmotionType, f32, Option<&str>)>` *실제와 정합*. `set_intensity(emotion_type, intensity)`도 도메인에서 직접 활용 가능.

4. **Worktree에 baselines/ 폴더 없음**: spec D1 baseline 1220은 main 브랜치 또는 별도 환경 기준. 본 Stage에서는 default + chat features 기준 baseline 별도 박제.

## 알려진 위험 (push 후 Stage 3에서 정리)

### W1 — Beat 후 appraise modifier에 *간접 의미 변경* 발생 (회귀 가드 부재)

[stimulus_policy.rs:69~95](../../../src/application/command/policies/stimulus_policy.rs:69)의 `process_beat_transition`에서:
- Stage 1: `beat_rel = relationship.clone()` (변동 0) → `beat_rel.modifiers()`은 *원본 affinity/trust*로 산정 → appraise에 전달
- Stage 2: `beat_rel`에 `update_axes_from_emotion`로 *stimulated emotion 기준 4축 변동* 적용 → `beat_rel.modifiers()`은 *변동된 affinity/trust*로 산정 → appraise에 전달

즉 *Beat 시점 new_state 감정 강도*가 Stage 1 vs Stage 2 사이에 **측정 가능하게 달라짐**. *의도된 활성화* (4축 변동의 효과가 modifier 산정에 *기대 반영*되는 것이 spec §4.1 본질)이지만, **회귀 가드 단위 테스트 부재** — *unintended drift*와 *intended activation* 구분 못함.

- spec §7 종결 게이트 7개 모두 *단위* 수치만 검증 (S2 임충 Anger 단독 trust 50→15.8 등). *integrated Beat appraise* 결과 정합 검증 없음.
- cargo test 866 PASS 결과는 *기존 단위 테스트가 변경에 robust*했음을 보여줄 뿐 *정량 동등성*을 의미하지 않음.
- **Stage 3 진입 직전 또는 Phase 2.3 narrative 시뮬에서**: Beat 후 modifier *변화량* 측정 단위 테스트 1~2개 박을 것. 예: `relationship trust 50 / affinity 40` 상태 + Anger 0.95 → `intensity_multiplier` 변화 정량 / 동일 input의 Stage 1 baseline 대비 차이.

### W2 — Pity의 부정 감정 분류 — spec §4.3 본문 모호

`is_negative_emotion` 헬퍼 (mapping.rs:179~)에서 *Pity 제외* 박았으나, spec §4.3 본문이 *"부정 감정"의 정확한 enumeration*을 명시하지 않음. *OCC valence 부정*으로 본다면 Pity 포함되어야 하고, *4축 base_delta가 음 우세*로 본다면 Pity는 affinity +10이라 제외. 본 구현은 **affinity 부호 기준** (4축 효과 우선).

- `is_negative_emotion` 함수 doc에 정의 박제 (commit 시점).
- Phase 2.3 narrative 검증에서 *공감 군 4 (HappyFor/Pity/Gloating/Resentment)의 A− Forgiveness 룰 적용 여부* 시뮬로 재확인 예정.

### W3 — `update_axes_from_emotion` BondStatus 차단 silent return

[mapping.rs:217~219](../../../src/domain/relationship/mapping.rs:217). Deceased/Resolved/Dormant 시 *조용히 return* — 이벤트/로그 부재. 의도된 동작이지만 *디버깅 추적 단서 없음*. Phase 2.3 진입 시 `tracing::debug!` 1줄 추가 검토 (비용 0).

### W4 — B-D12 가드의 호출 측 분산 (3 위치 동일 패턴)

[relationship_policy.rs:143, 238](../../../src/application/command/policies/relationship_policy.rs:143) + [stimulus_policy.rs:78](../../../src/application/command/policies/stimulus_policy.rs:78) — *`matches!(Pride|Shame) continue`* 3 위치 박힘. spec §4 결정대로 *"호출 측 책임"* 일관이지만 DRY 위반.

- **4번째 호출자** (Phase 3a/3b에서 `Channel 2 Temporal` / `Channel 3 External` 추가 시) 누락 위험.
- Phase 2.3 또는 3a 진입 시 `Relationship::accepts_axis_update_for(emotion_type)` 같은 헬퍼로 통합 검토 (현재 호출 측 명시 패턴 유지).
