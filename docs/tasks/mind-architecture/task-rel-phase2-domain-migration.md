# Phase 2 — Relationship 도메인 마이그레이션 (4축 + BondKind/BondStatus/Partnership + type)

**Status**: `ready` — **Stage 0 종결 (2026-05-13), v1.0 spec freeze**. Stage 1 진입 대기.
**Owner**: Bekay + Claude
**Parent**: `docs/tasks/mind-architecture/00-roadmap.md` §5 Phase 2 (분할 후 본 phase)
**Sibling**: `task-rel-phase2.5-channel1.md` (별도 phase로 분리, 추후 작성)
**Prerequisite**: Phase 1/1.5/1.6 ✅ 완료 (`phase1-checkpoint-report.md` v0.2)

---

## §1 Scope

**포함**:
- `Relationship` 도메인: 3축 (closeness/trust/power, ±1.0) → 4축 (trust/affinity/respect/wariness, ±100)
- `BondKind` 11종 enum 도입 (relationships.md v0.7 §3.1)
- `BondStatus` 5종 enum 도입 (§3.5)
- `Partnership` 4종 enum 도입 (§3.6)
- `type: String` + `type_history: Vec<TypeChange>` 자유 텍스트 필드 (§2)
- 시나리오 JSON schema v0.6 → v0.7 갱신 (Relationship 필드 한정)
- 검증 시나리오 ~45 페어 데이터 마이그레이션
- 1095+ tests 회귀 갱신

**비포함** (Phase 2.3 별도 — 신설):
- appraise 로직 정비 (시뮬레이션 기반 튜닝)
- 누락 OCC 식별 + 자동 보완/경고 (I1)
- Compound 감정 식별 확장
- `RelationshipModifiers` 정밀화 (4축 환경에서 누락 modifier 검증)
- HEXACO 보정자 정량 미세조정
- base_delta 48셀 시나리오 기반 미세조정

**비포함** (Phase 2.5 별도):
- Channel 1 Declarative 활성화 (`declarative_events` / `partnership_event` placeholder는 *enum/필드 정의만* 박고 LLM emit/검증/적용은 Phase 2.5)
- 사회적 일관성 검증 5 카테고리 (A~E)
- 4-tier 적용 모드
- **★ `axis_modulation` (LLM 미세조정 3지선다)** — Reflection 결과 schema 확장

**비포함** (Phase 3 별도):
- Channel 2 Temporal (BondKind 시간 게이트 자동 진입)
- Channel 3 External (세계 사건 overlay)
- `RecollectionAction` 5종 추모 행동
- `ActionTriggerEvaluator` 5-dim feasibility

---

## §2 Inputs

| 문서 | 입력 버전 | 출력 버전 |
|---|---|---|
| `relationships.md` | v0.7 (현행, 변경 없음) | v0.7 (참조만) |
| `_schema.md` | v0.6 (현행) | **v0.7** (Phase 2가 갱신) |
| `00-roadmap.md` | v0.5 (Phase 1.5/1.6 반영) | v0.6 (Phase 2 진입 표기) |

**참조 baseline**:
- Phase 1 spec stages (6 stage 패턴)
- Phase 1 checkpoint report v0.2 (전제 조건 §7)

---

## §3 Findings (Stage 0)

Phase 1 Stage 0 패턴(F1~F12)을 본 phase에 적용. A 카테고리 5개 — *현재 코드 사실 조사*. 각 항목은 변경 전 *현재 상태*를 grep으로 확정하고, Phase 2 변경 면적을 산정한다.


### A1 — `Relationship` 도메인 + `Score` 타입 + 사용처

**현재 도메인** (`src/domain/relationship.rs`):
```rust
pub struct Relationship {
    owner: NpcId, target: NpcId,
    closeness: Score, trust: Score, power: Score,
}
pub struct RelationshipModifiers {
    pub closeness_modifier: f32, pub closeness_squared: f32,
    pub closeness_abs: f32, pub trust_modifier: f32,
}
```

**`Score` 타입**: `src/domain/personality.rs` 정의 — *HEXACO 24 facet과 공유*. `Score(f32)` 범위 ±1.0. Phase 2가 ±100 범위로 가면 *공유 타입 분리 필요*.

**사용처 134 매치** (wuxia-core 4매치는 제외, 폐기 예정):

| 영역 | 매치 | 책임 | Phase 2 변경 |
|---|---|---|---|
| `domain/relationship.rs` | 13 | 정의 + 내부 테스트 | 재작성 본체 |
| `Relationship::neutral()` 호출 | 16 | Policy fallback (값 무관) | 자동 흡수 (헬퍼 시그니처 보존) |
| `Relationship::new` / `RelationshipBuilder` | 4 매치 | 시나리오 JSON 진입점 1 + UI CRUD 1 + 테스트 2 | 시그니처 변경 |
| `.modifiers()` 통과 | 5곳 | emotion/stimulus/scene policy + memory_repository | **변경 0** (인터페이스 보존, 내부 매핑만 재작성) |
| `.closeness()`/`.trust()`/`.power()` 직접 호출 | 6곳 | snapshot + orchestrator + relationship_policy + memory_repository + telling_ingestion + domain_sync | 명시 변경 (4축 메서드명) |
| 테스트 호출 | ~100 | Builder 패턴 + neutral 호출 | **회귀 면적 큼** — 자동 마이그레이션 스크립트 검토 |

**핵심 발견**:
1. `modifiers()` 추상화 경계가 *OCC 감정 엔진 5곳을 자동 흡수*. 인터페이스 면적 작음.
2. 시나리오 JSON↔도메인 진입점은 `memory_repository.rs:195` 단 1곳. 3축→4축 변환 룰 1곳 집중 가능.
3. 회귀 면적의 본질은 *테스트 ~100 호출의 Builder 시그니처 변경*. 자동 변환 스크립트가 비용 절감 가능 — Phase 2 Stage 1에서 검토.
4. `Score` 타입이 HEXACO와 공유라 *분리 결정* 필요 (→ B-D1).


### A2 — OCC → 3축 매핑 함수 + 갱신 책임자

**현재 OCC → axes 매핑**:
```rust
// Relationship::after_dialogue (단일 함수, closeness 1축만 자동)
pub fn after_dialogue(&self, final_state: &EmotionState, significance: f32) -> Self {
    self.with_updated_closeness(final_state.overall_valence(), significance)
    // trust: 변경 없음 (향후 LLM 평가)
    // power: 변경 없음 (서사 이벤트만)
}
```

**공식**: `new_closeness = clamp(old + valence × 0.05 × (1 + sig × 3.0), ±1.0)`

**갱신 책임자**: `application/command/policies/relationship_policy.rs` — 호출 위치 *2 곳 중복* (`handle_dialogue_end` + `handle_relationship_update_with_cause`). Phase 2 재작성 시 helper 추출 권장.

**`outer_loop_entry()` 게이트** — Phase 2/3 진입 자리 *예약됨*:
```rust
match reflection {
    Some(refl) => {
        refl.significance_score >= 0.3
            || !refl.is_chitchat
            || !refl.declarative_events.is_empty()     // ← Phase 2.5 활성화 위치
            || refl.partnership_event.is_some()        // ← Phase 2.5 활성화 위치
        // || temporal_signals (Phase 3a)
        // || external_events (Phase 3b)
    }
    None => legacy_significance.is_some(),
}
```
현재는 declarative_events/partnership_event가 항상 빈/None이라 조건이 작동 안 함. Phase 2가 *enum/필드 정의*만 박으면 Phase 2.5에서 *진짜 데이터*가 흘러들어와 게이트 즉시 동작.

**`RelationshipUpdatedPayload`** — 외부 schema 영향:
```rust
// 현재: 6 필드 (3축 × 2 = before/after)
closeness_before, trust_before, power_before,
closeness_after,  trust_after,  power_after,
// Phase 2 후: 8 필드 (4축 × 2) + cause 그대로
trust_before, affinity_before, respect_before, wariness_before,
trust_after,  affinity_after,  respect_after,  wariness_after,
// power 폐기 (B-D4 확정)
```

**외부 구독자**: `relationship_memory_handler`, SSE bridge (`event_bridge`), Mind Studio frontend — schema 변경 영향.

**핵심 발견**:
1. 현재 *자동 갱신은 closeness 1축뿐*. Phase 2가 *4축 자동 갱신 룰*을 어디까지 박을지가 작업 크기 결정 (→ B-D5, B-D6).
2. `RelationshipUpdatedPayload` 6→8 필드 변경 — frontend `domain_sync.rs` + SSE event_bridge 매핑 갱신.
3. `outer_loop_entry()` 게이트는 *Phase 2 변경 0*. Phase 2.5에서 데이터만 흐름.


### A3 — `RelationshipChangeCause` enum 5 variants + 사용처

**정의** (`src/domain/event.rs:138-150`):
```rust
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RelationshipChangeCause {
    SceneInteraction { scene_id: SceneId },
    InformationTold { origin_chain: Vec<String> },
    WorldEventOverlay { topic: Option<String> },
    Rumor { rumor_id: String },
    #[default] Unspecified,
}
```

**사용처 22 매치 분류**:

| 위치 | 종류 | 패턴 |
|---|---|---|
| `relationship_policy.rs:88, 117, 241` | **emit (3 위치)** | `SceneInteraction` (BeatTransitioned) + `Unspecified` × 2 |
| `event.rs:901, 1094` | tests | 단위 테스트 |
| `projection_handlers.rs:311, 336, 347` | tests | RelationshipProjection 회귀 |
| `relationship_memory_handler.rs:87~120` | **consume** | 5 variant 분기 — MemorySource/topic/content 매핑 |
| `relationship_memory_handler.rs:188-191` | **consume** | origin_chain 추출 (InformationTold/Rumor) |
| `relationship_memory_handler.rs:376~531` | tests | 분기별 회귀 테스트 |

**핵심 발견**:
1. **emit은 SceneInteraction + Unspecified 2 variant만 실제 작동**. `InformationTold`/`WorldEventOverlay`/`Rumor`는 enum 정의만 있고 *어디서도 emit되지 않음* (Step C/D 설계자가 미리 박은 forward-compat 자리).
2. **consume 측 완전 구현**. 새 variant 추가 시 `relationship_memory_handler` 분기에 추가만 하면 됨 (기존 패턴 mirror).
3. **Phase 2 본체 영향 0**. cause enum과 *변경 면적*은 직교. Phase 2.5에서 declarative_events 활성화 시 새 variant 후보 (`BondKindFormed` 등) — *Phase 2 결정 사항 아님*.
4. `RelationshipUpdatedPayload`의 `cause` 필드는 그대로 (4축 변경과 무관).


### A4 — 시나리오 JSON 3축 데이터 분포

**규모**: 267 매치 (closeness/trust/power 각각) = ~89 Relationship instance = **~45 페어** (대부분 a↔b 쌍방향). 위치 — 시나리오 JSON 안 + `session_*_result.json` 테스트 결과 파일.

**별도 `wuxia_world/assets/relationships/` 디렉토리 비어 있음** — 관계는 시나리오 JSON 안에 직접 박힘. 향후 Phase 2.x 분리 작업 후보지만 *Phase 2 범위 아님*.

**검증 케이스 2개**:

| 페어 | closeness | trust | power | 의미 |
|---|---|---|---|---|
| 임충 → 육겸 | +0.4 | +0.5 | 0.0 | "옛 친구" 인식 (배신 전) |
| 육겸 → 임충 | -0.2 | -0.3 | -0.4 | "제거 대상" (배신 의도) |
| 수련 → 무백 | +0.7 | +0.8 | -0.1 | 의형제+절제된 사모 |
| 무백 → 수련 | +0.7 | +0.8 | +0.1 | 동일 |

**3축 → 4축+type 변환 룰**:

| 3축 | 4축 대응 | 변환 가능성 |
|---|---|---|
| `trust` ±1.0 | `trust` ±100 | **자동** — 의미 동일 보존, `× 100` |
| `closeness` ±1.0 | `affinity` ±100 | **반자동** — 의미 부분 겹침 (closeness ⊃ affinity), `× 100` 후 디자이너 검토 |
| (없음) | `respect` ±100 | **수동** — 디자이너 보충 (B-D10) |
| (없음) | `wariness` 0~100 | **수동** — 디자이너 보충 (B-D10) |
| `power` ±1.0 | (폐기, type 흡수) | **수동** — 디자이너가 `type` 한 줄 작성 (B-D4 확정) |

**핵심 발견**:
1. **자동 변환 가능 비율 ~50%** (trust + closeness만). respect/wariness/type은 *디자이너 손 작업 필수*.
2. **`session_*_result.json` 결과 파일도 3축 박힘** — Phase 2 완료 후 일괄 폐기 + 재실행 권장 (B-D9).
3. **데이터 마이그레이션 워크플로우는 별도 Stage**가 될 가능성 큼 — ~45 페어 × 4축 + type = ~225 값 디자이너 검토.
4. `power` 데이터 활용도가 *미미했음* (대부분 ±0.0~±0.4 범위, ActingGuide 라벨용 디스플레이) — B-D4 확정 근거.

### A5 — `MAX_EVENTS_PER_COMMAND` 재산정

**현재 22** (`dispatcher.rs:35-41`): Phase 1 worst-case 8~9 + 안전 마진 2.5배.

**Phase 2 본체 영향**: 변경 0.
- 3축 → 4축: payload 필드 크기만 6 → 8, 이벤트 *수* 영향 없음.
- BondKind/BondStatus/Partnership/type/type_history 도입: Relationship 필드 추가, 이벤트 추가 아님.
- `power` 폐기: payload 필드 감소.

**Phase 2.5 worst-case 예상** (참고):
```
DialogueEndRequested 1 + DialogueReflected 1 + RelationshipUpdated (4축) 1
+ declarative_events fan-out N (현실 상한 ≈ 5)
+ 사회적 일관성 검증 reject 최대 5 (5 카테고리 A~E)
+ EmotionCleared 1 + SceneEnded 1 + Inline projection 3
= 12 + N ≈ 17
```

**결론**: Phase 2 / Phase 2.5 모두 22 안전. **인상 불필요**.

---

## §3 종합 — Phase 2 영향 면적

| 변경 면적 | 크기 | 비고 |
|---|---|---|
| 도메인 본체 (relationship.rs + Score) | 큼 | 재작성 |
| RelationshipPolicy 매핑 | 중 | 2 위치 helper 추출 + 재작성 |
| Payload schema (6→8 필드) | 작음 | 필드 이름 변경 + 추가 |
| cause enum | **0** | 직교 |
| consume 측 (memory_handler) | **0** | 인터페이스 보존 |
| 시나리오 JSON 진입점 | 중 | 변환 룰 1곳 집중 |
| Mind Studio CRUD | 작음 | 단순 필드 변경 |
| 테스트 회귀 | 큼 | ~100 호출 자동 마이그레이션 검토 |
| 시나리오 데이터 (디자이너 손) | 큼 | ~45 페어 × 5필드 = ~225 값 |
| `MAX_EVENTS_PER_COMMAND` | **0** | 22 안전 |

---

## §3.6 시뮬레이션 검증 (S1~S4) — Stage 0 추가 발견

B 카테고리 결정 항목 (B-D6, B-D12, B-D13, B-D14)의 근거 확보 + Phase 2 통로 A 디자인 검증을 위해 무협지 시나리오 4 케이스에 v0.7 §4 디자인 적용.

### S1 — 임충 → 노지심 (Gratitude 단순)

- 디자이너 박는 Beat focus: `event(desirability +0.7) + action(agent_id="lu_zhishen", praiseworthiness +0.8)`
- appraise 자동 생성: Joy + **Admiration** + **Gratitude** (compound)
- 결과: trust +13, affinity +6, respect 0 → *Admiration 자동 생성*으로 **respect 0 문제 자체가 발생 안 함**
- ✅ base_delta 표 + ActionFocus 박기로 *방향성 자연*

### S2 — 임충 → 육겸 (산신묘 대형 사건)

- 디자이너 박는 focus: `event(desirability -0.95, prospect=FearConfirmed) + action(agent_id="lu_qian", praiseworthiness -0.95) + object(appealingness -0.95)`
- appraise 자동 생성: Distress + FearsConfirmed + **Reproach** + **Hate** + **Anger** (compound)
- 결과 (Anger + Hate + Reproach 합산 + HEXACO ×1.2 + axis_modulation "high"): trust -49, affinity -43, respect -30, wariness +53
- 시나리오 박힌 *후 상태* (trust -30, affinity -20, wariness 매우 높음)와 비교: **affinity/wariness는 align**, trust만 *추가 -31 변동* 필요
- ✅ Phase 2 통로 A (점진적) + Phase 2.5 통로 B (declarative_events 큰 도약) **분담 작동** 입증

### S3 — 수련 → 옥교룡 (상충 감정)

- 디자이너 박는 focus: `event(desirability_for_self -0.4, desirability_for_other=옥교룡 -0.7) + action(agent_id="yu_xiaolong", praiseworthiness -0.6)`
- appraise 자동 생성: Distress + **Pity** + **Reproach** + **Anger** (compound)
- 결과 (Pity + Reproach + Anger 합산 + HEXACO ×0.56 + axis_modulation 혼합): trust -9, affinity -3, respect -15, wariness +14
- ★ **affinity 거의 변화 없음** (Pity +5와 Anger/Reproach -10 상쇄) — *상충 감정의 정확한 시뮬레이션* (와호장룡 수련의 *옥교룡을 비난하면서도 안타까워함* 패턴 포착)
- ✅ base_delta 표가 *상충 감정* 균형을 자동 처리

### S4 — 임충 → 고구 (맥락 의존, 표 한계 시험)

- *같은 외형 사건* (고구의 자비)이 NPC 시각에 따라 *전혀 다른 감정*. base_delta 표는 *맥락 무시* — 한계?
- 검증 결과: **한계 아님**. *3 layer separation*이 흡수:
  - Layer 1 (Beat focus 설계): 디자이너가 *NPC 시각으로* event/action/object 박음 → 옥교룡 자비를 *위협*으로 박으면 `desirability_for_self -0.3`이 됨
  - Layer 1.5 (Relationship modifiers): 기존 임충→고구 적대 관계의 `trust_modifier`/`hostility_modifier`가 *감정 강도 자동 조정*
  - Layer 2/3 (appraise + base_delta): *입력이 박힌 후* 결정론적 매핑
- 디자이너 실수 (옵션 A 박음)에도 *Relationship modifiers가 자연 보정* — 임팩트 약화
- ✅ base_delta 표의 *맥락 무시*는 *진짜 한계 아님*. Layer 3 only의 책임

### 시뮬레이션 검증 종합

| 케이스 | 검증 결과 |
|---|---|
| S1 (Gratitude 단순) | ✅ Admiration 자동 식별 |
| S2 (산신묘 대형 사건) | ✅ Phase 2/2.5 분담 작동 |
| S3 (상충 감정) | ✅ affinity 정체 (자동 균형) |
| S4 (맥락 의존) | ✅ 3 layer separation |

→ **base_delta 48셀 표 + HEXACO 보정자 + axis_modulation 결합이 Phase 2 통로 A의 *합당한 작동*을 입증**. v0.7 §4 디자인 그대로 박는 게 적절.

### ★ 핵심 발견 — appraise 입력 의존성

S1~S4에서 *공통 패턴*: **appraise는 *디자이너 박은 Beat focus 완전성에 의존***. ActionFocus 안 박으면 Admiration/Reproach 자동 생성 0. EventFocus 안 박으면 Joy/Distress/HappyFor/Pity 등 자동 생성 0.

- 시나리오 디자이너가 *12+ OCC 완전 식별*은 부담 큼
- *상식적 추론* (예: "도움받음 → 칭찬할 행위자 있음") 자동화 안 됨

→ **Phase 2.3 (appraise 정비) 신설 결정**. Phase 2 본체와 Phase 2.5 사이에 *얇은 phase*로 분리. 시뮬레이션 시나리오 set 공식화 + 누락 OCC 검증/경고 (I1) + Compound 식별 확장 + modifiers 정밀화 + HEXACO/base_delta 미세조정.

---

## §4 Decisions (Stage 0 — ✅ Phase 2 본체 결정 완료)

B 카테고리 14개 항목. **Phase 2 본체 12개 전부 확정 ✅**. B-D7/B-D11은 Phase 2.5 시점 결정.

| # | 항목 | 상태 |
|---|---|---|
| B-D1 | `Score` 타입 운명 (HEXACO와 분리/유지/일반화) | ✅ **확정 — A (분리) + 2 타입**: HEXACO `Score(f32)` ±1.0 그대로 / Relationship 4축 신설 `AxisScore(f32)` ±100 (trust/affinity/respect) + `WarinessScore(f32)` 0~100 별 타입. wariness 음수 박는 실수 *컴파일 시점 차단*. HEXACO 사용처 변경 0. |
| B-D2 | ±1.0 → ±100 변환 방식 (내부 float? 정수?) | ✅ **확정 — f32 내부 표현 + JSON 정수 round 출력**. v0.7 §4.1 코드 그대로 호환. base_delta × intensity × HEXACO 곱셈 *정밀도 유지* (예: -25 × 0.95 × 1.2 = -28.5 정확). 시나리오 JSON엔 `"trust": 75` 정수 표기, 내부 75.0. |
| B-D3 | closeness → affinity 변환 룰 (의미 다름) | ✅ **확정 — (c) 혼합**: 자동 변환 baseline `affinity = closeness × 100` + 디자이너 선택적 조정. closeness("함께 있을 때 친근감") ⊃ affinity("혼자일 때 그리움") 의미 부분 겹침. 자동 변환을 *초기값*으로 박고 narrative 검증/Phase 2.3 시뮬레이션 중 어색한 케이스만 조정. 원수 케이스(임충→고구 등) 음수 보충은 B-D10 (초기값 룰)에서 흡수. |
| B-D4 | `power` 운명 | ✅ **확정 — 폐기, `type` 자유 텍스트 흡수** |
| B-D5 | 4축 각각 별도 매핑 함수? 단일 함수? | ✅ **확정 — 단일 함수 (v0.7 §4.1 그대로)** `update_axes_from_emotion(rel, emotion, intensity, hexaco)`. 한 OCC 감정 입력에 4축 동시 갱신. `base_delta(emotion) -> AxisDelta` 48셀 lookup + `hexaco_modifier(emotion, hexaco) -> AxisModifier` + clamp. 4축별 분리 함수는 *코드 중복 + 비효율*이라 비채택. 구체 구현 (lookup 자료구조 / 메서드 vs 자유 함수)은 Stage 1. |
| B-D6 | 4축 자동 갱신 룰 + 시점 + 가드레일 | ✅ **확정 — T1 (대화 끝 batch) + D6-a (v0.7 §4.1~4.3 그대로: base_delta 48셀 + HEXACO 보정자 + BondStatus 차단 + clamp) + axis_modulation 3지선다 (low/default/high → ±5/0/+5, reflection LLM 출력 필드 신설, 추가 LLM 호출 0)** |
| B-D7 | (Phase 2.5) 새 cause variant 명명 | Phase 2.5 |
| B-D8 | 시나리오 데이터 마이그레이션 — 반자동 스크립트 + 디자이너 수동? | ✅ **확정 — W3+ (자동 + Claude AI 추론 + 디자이너 검토)**. 6 단계 워크플로우: ① Rust binary 마이그레이션 도구 작성 (`tools/migrate_relationships/`) → ② Claude AI 추론으로 BondKind/type 채움 + 디자이너 검토 → ③ Rust binary 실행 (자동 산술 변환: trust×100, closeness×100→affinity, BondKind 기반 respect/wariness baseline) → ④ 컴파일 + 기존 테스트 → ⑤ narrative 시뮬레이션 검증 → ⑥ Claude AI 추론으로 어색 케이스 조정 + 디자이너 검토. **Claude prompt template 박음** (`docs/migration/claude-prompts/`: bond-kind-inference.md, type-text-inference.md, adjustment-suggestion.md). 안전장치: 원본 백업 (`data/scenarios.backup-v0.6/`) + 드라이런 모드 + diff 출력. |
| B-D9 | `session_*_result.json` 폐기 정책 | ✅ **확정 — (a) 일괄 폐기 + Phase 2 후 재생성**. 결과 파일은 *입력 아닌 출력* — 재현 가능. 백업 `data/sessions.backup-v0.6/` 이동 후 폐기. Phase 2 종결 시점 narrative 시뮬레이션 (Stage 5)에서 4축 시스템으로 일괄 재생성. 재생성된 결과가 v0.7 검증 데이터. |
| B-D10 | respect/wariness 초기값 룰 (0 시작? closeness 부호로 추정?) | ✅ **확정 — (B') 간단 휴리스틱 + BondKind 보완**. 마이그레이션 시 디자이너가 *BondKind 먼저 박음* (없는 페어 None). 자동 변환: BondKind 원수 4종 → respect -60 / wariness +80, BondKind Guardian/Mentor → respect +60 / wariness +5, BondKind 지기 4 + Companion/LoyalRetainer → respect closeness×70 / wariness +5, BondKind None → respect closeness×50 / wariness max(0, -trust×50). 디자이너 narrative 검증에서 조정. B-D8 워크플로우와 결합. |
| B-D11 | (Phase 2.5) declarative_events 상한 N | Phase 2.5 |
| B-D12 | Shame/Pride (`agent_id=None`) 처리 | ✅ **확정 — 4축 변동 0, PAD만 영향** (v0.7 §4.2 표의 Shame/Pride 행은 4축 자동 갱신에서 무시) |
| B-D13 | 1회 변동 상한 | ✅ **확정 — 별도 cap 없음** (HEXACO 보정자 + intensity 곱 + axis_modulation ±5가 자연 한계 형성) |
| B-D14 | Well-being/Prospect 10 OCC 4축 매핑 누락 | ✅ **확정 — 의도된 누락 채택** (Joy/Distress/Hope/Fear/Satisfaction/Disappointment/Relief/FearsConfirmed/Remorse/Gratification 10개는 4축 변동 0, PAD만 영향. Compound 감정(Anger/Gratitude)이 간접 흡수) |

### ★ Phase 2.3 신설 결정

§3.6 시뮬레이션 검증에서 발견된 *appraise 입력 의존성* 문제 해결을 위해 Phase 2 본체와 Phase 2.5 사이에 **Phase 2.3 — appraise 정비** 신설:

- Phase 2 (4축 도메인 안정) → **Phase 2.3 (appraise 정비, 시뮬레이션 기반)** → Phase 2.5 (LLM 통합)
- 작업 후보: 시뮬레이션 시나리오 set 공식화 (S1~S4 + 신규 케이스 ~15개) / 누락 OCC 검증/경고 (I1) / Compound 식별 확장 / `RelationshipModifiers` 정밀화 / HEXACO·base_delta 미세조정
- 별도 spec `task-rel-phase2.3-appraise-tuning.md` (Phase 2 종결 후 작성)
- `00-roadmap.md` §5에 Phase 2.3 행 신설 필요

---

## §5 Risks (C 카테고리 — Stage 0 진행 중)

### R1 — 회귀 면적 큼

- 테스트 ~100 호출이 `Relationship` 3축 시그니처에 의존 (Builder 패턴 + `Relationship::new`)
- 완화: Stage 1에서 자동 마이그레이션 스크립트 검토 (B-D8 결정 후)

### R2 — 시나리오 데이터 디자이너 손 작업 ✅ **대폭 완화 (B-D8 확정 2026-05-13)**

- 기존 우려: ~45 페어 × 4축 + type = ~225 값 디자이너 검토 필요
- **완화**: B-D8 W3+ 채택. *디자이너 손 작업* → *Claude AI 추론 + 디자이너 검토*. 디자이너는 *작성*하지 않고 *검토*만. Claude prompt template (`docs/migration/claude-prompts/`)로 외부 사용자도 동일 워크플로우 적용 가능.
- 잔존 위험: Claude 추론 *문학적 정확성* — 무협 원전 맥락 (수호지/와호장룡/사조영웅전)에 대한 LLM 지식 범위 한계. 검증 부담은 narrative 시뮬레이션 (Stage 5)으로 흡수.

### R3 — `Score` 타입 HEXACO와 공유 ✅ **해소 (B-D1 확정 2026-05-13)**

- 기존 우려: `Score(f32)` ±1.0 현재 HEXACO 24 facet과 *공유 Value Object*. 4축 ±100 도입 시 충돌 가능.
- **해소**: B-D1 A (분리) + 2 타입 결정으로 *HEXACO `Score` 사용처 변경 0*. 새 `AxisScore`/`WarinessScore` 타입 신설로 격리.

### R4 — `RelationshipUpdatedPayload` 6→8 필드 schema breaking

- 외부 구독자: `relationship_memory_handler`, SSE bridge (`event_bridge`), Mind Studio frontend
- 완화: Stage 1에서 schema 갱신 + Phase 1.6의 event_bridge 패턴 활용 (수동 emit 0)

### R5 — appraise 입력 의존성 (S1~S4 검증에서 발견)

- appraise는 *디자이너 박은 Beat focus 완전성*에 의존. 누락 시 4축 변동 누락
- 디자이너가 *12+ OCC 정확 식별* 부담 큼, *상식적 추론* 자동화 안 됨
- 완화: **Phase 2.3 (appraise 정비)에서 시뮬레이션 기반 검증/경고 (I1) + Compound 식별 확장**
- Phase 2 본체에는 영향 없음 (도메인 마이그레이션과 직교)

### R6 — base_delta 48셀 시나리오 검증 부담

- 표 값 *방향성*은 S1~S4 검증 통과. *정량값* 미세조정 가능성 존재.
- 완화: Phase 2.3에서 시나리오 set 기반 정량 미세조정 (Phase 2 본체에선 v0.7 §4.2 표 그대로 박음)

---

## §6 Baseline (D 카테고리)

Phase 2 회귀 검증의 기준점. Phase 1 종결 시점 (2026-05-11 baseline) 인용 + Stage 1 진입 직전 재측정.

### D1 — cargo test 통과 카운트

| 항목 | Phase 1 종결 시점 | 출처 |
|---|---|---|
| `cargo test --all-features --workspace` | **1095 passed**, 0 failed | `phase1-checkpoint-report.md:35-36, 308` |
| `cargo check --all-features` | ✅ | 동일 |
| `cargo build --features chat` | ✅ | 동일 |

**게이트**: Phase 2 마이그레이션 완료 후 *동일 카운트 ± 신규 테스트 수* 통과. 회귀 0건.

### D2 — `dispatch_v2(EndDialogue)` latency

| 케이스 | Phase 1 latency | follow-up |
|---|---|---|
| chitchat | **24.17 µs** | 3 |
| significant | **35.03 µs** | 4 |
| legacy | **29.34 µs** | 3 |

**게이트**: Phase 2 후 *±20% 이내*. 4축 매핑 추가로 약간 증가 예상 (예: ~30/42/35 µs). axis_modulation는 reflection LLM에서 추가되므로 별 영향 없음.

### D3 — Narrative 3밴드 calibration

| 시나리오 | significance | Target |
|---|---|---|
| chitchat-passerby | **0.000** | <0.3 ✅ |
| daily-training | **0.461** | 0.3~0.7 ✅ |
| lin-chong-shanshenmiao | **0.980** | ≥0.7 ✅ |

**게이트**: Phase 2 마이그레이션 후 동일 시나리오의 *3밴드 위치 보존*. 가중치 `0.40/0.30/0.15/0.15` + 임계값 `0.3` 유지.

### D4 — `compute_significance` 엔진 성능

| 항목 | Phase 1 baseline |
|---|---|
| `compute_significance(10 turn) ×10000` | **8.36 µs/call** (target <1ms, 100x 마진) |

**게이트**: Phase 2 후 ±20% 이내.

### D5 — `MAX_EVENTS_PER_COMMAND`

| 항목 | 현재 | A5 결론 |
|---|---|---|
| 상수 값 | **22** | Phase 2 본체 변경 0, Phase 2.5 worst-case 17, 인상 불필요 |

### D6 — 코드 메트릭

| 항목 | Phase 1 종결 | 출처 |
|---|---|---|
| domain/ tokio 참조 | 0 | userMemories |
| ports.rs tokio 참조 | 1 (`send_message_stream` — 별도 migration) | userMemories |
| application/ tokio 참조 | 5 (event_bus + memory_projector + director/) | userMemories |
| EventKind variant 수 | 31개 | `00-roadmap.md` §2 |

### Stage 1 진입 직전 재측정 작업

Stage 1 시작 첫 작업: 위 수치 *재측정*하여 `baselines/cargo-test-2026-MM-DD-PASS.log` 패턴으로 박제. Phase 2 진행 중 비교 기준.

---

## §7 Stages

Phase 1 6 stage 패턴 따라 분할. 직선 의존 (Stage N → Stage N+1). 각 stage 종결 시 grep 게이트 + 통과 카운트 검증.

### Stage 1 — Type 신설 + Domain 재작성

**범위**:
- `AxisScore(f32)` + `WarinessScore(f32)` 신설 (B-D1/D2) — `src/domain/relationship/axis.rs` 또는 `relationship.rs` 내부 모듈
- `BondKind` 11 variants enum (`SwornBrothers`/`MasterDisciple`/`Soulmate`/`LoyalRetainer`/`Companion`/`Guardian`/`Mentor`/`BloodEnemy`/`ArchRival`/`Betrayer`/`Oppressor`)
- `BondStatus` 5 variants enum (`Active`/`Resolved`/`Deceased`/`Dormant`/`Reactivating`) + `accepts_live_input()` helper
- `Partnership` 4 variants enum (`Spouse`/`Engaged`/`Lover`/`Separated`)
- `Relationship` 본체 재작성: 4축 + bond_kind + bond_status + partnership + type + type_history (B-D4: `power` 폐기)
- `RelationshipBuilder` 4축 API (`.trust(AxisScore)` / `.affinity(AxisScore)` / `.respect(AxisScore)` / `.wariness(WarinessScore)`)
- `Relationship::neutral()` 시그니처 보존 (16곳 자동 흡수)
- 단위 테스트: clamp 범위, 양극 점착, BondStatus 차단 등

**게이트**:
1. `cargo check --all-features` 통과
2. `Relationship::neutral()` 호출 16곳 자동 흡수 (변경 0)
3. `WarinessScore::new(-50.0)` 컴파일 차단 확인 (불변식 강제)
4. 단위 테스트 통과

**산출 commit**: `phase2-stage1-domain.md` 회고 +  `baselines/cargo-test-2026-MM-DD-PASS.log` (Stage 1 진입 직전 재측정)

---

### Stage 2 — OCC → 4축 매핑 (base_delta + HEXACO + modifiers)

**범위**:
- `AxisDelta { trust: f32, affinity: f32, respect: f32, wariness: f32 }` Value Object
- `base_delta(emotion: OccEmotion) -> AxisDelta` 48셀 lookup (v0.7 §4.2, B-D6 D6-a)
  - 12 OCC × 4축 (Well-being/Prospect 10 OCC는 4축 변동 0 — B-D14)
  - `match` 표현식 또는 `const` 배열 (구현 선택)
- `hexaco_modifier(emotion: OccEmotion, hexaco: &Hexaco) -> AxisModifier` 6 보정 룰 (v0.7 §4.3, B-D6 D6-a)
  - H+ Sincerity ×1.2 trust / A+ Patience ×0.7 전역 / A- Forgiveness ×1.5 부정 / E+ Anxiety ×1.3 wariness / C+ Prudence ×0.8 큰 변화 / O+ Unconventionality 양극 가속
- `update_axes_from_emotion(rel, emotion, intensity, hexaco)` 단일 함수 (B-D5)
- BondStatus 차단 (`if !rel.bond_status.accepts_live_input() { return; }`)
- Shame/Pride (`agent_id=None`) 4축 변동 0 (B-D12)
- `RelationshipModifiers` 갱신 (4축 환경 modifier — Phase 2.3에서 정밀화 예정, Phase 2는 기존 시그니처 유지)
- 단위 테스트: S1~S4 ground truth 비교, Compound 감정 (Anger/Gratitude 등) 검증

**게이트**:
1. 단위 테스트 통과 (S1~S4 ground truth ±N 이내)
2. base_delta 48셀 lookup 결정론 (같은 입력 → 같은 출력)
3. BondStatus Deceased/Resolved/Dormant 차단 확인
4. Shame/Pride 4축 변동 0 확인

**산출 commit**: `phase2-stage2-mapping.md` 회고

---

### Stage 3 — RelationshipUpdater (대화 끝 batch 갱신)

**범위**:
- `relationship_policy.rs` 재작성:
  - 기존 closeness 1축 자동 갱신 코드 제거
  - 4축 자동 갱신 코드 신설 (`update_axes_from_emotion` 호출)
  - T1 시점 유지 (대화 끝 + 게이트 통과 시 1번)
  - 매핑 호출 2곳 중복 → helper 추출
- `RelationshipUpdatedPayload` 6→8 필드:
  - `closeness_before/after, trust_before/after, power_before/after` 제거
  - `trust_before/after, affinity_before/after, respect_before/after, wariness_before/after` 추가
  - `cause` 그대로 (변경 0)
- `RelationshipChangeCause` 변경 0 (A3 결론)
- `event_bridge` SSE 매핑 갱신 — `RelationshipUpdated` 페이로드 8 필드 반영
- Mind Studio frontend `domain_sync.rs` 4축 DTO 변환 갱신
- Reflection schema에 `axis_modulation` 필드 *placeholder* 추가 (Phase 2.5 활성화 대기 — 일단 모두 "default" 박힘 가정)
- `outer_loop_entry()` 게이트 변경 0 (Phase 2/2.5/3 진입 자리 그대로)
- 통합 테스트

**게이트**:
1. **1095+ tests 통과** (시그니처 변경 ~100 호출 모두 갱신)
2. `dispatch_v2(EndDialogue)` latency ±20% 이내 (D2 baseline 비교)
3. `event_bridge` SSE 발행 8 필드 확인
4. Mind Studio frontend 4축 표시 동작 확인
5. `outer_loop_entry()` 게이트 작동 (chitchat skip / significant 진입)

**산출 commit**: `phase2-stage3-updater.md` 회고

---

### Stage 4 — 마이그레이션 도구 + 시나리오 데이터

**범위**:
- Rust binary 작성: `tools/migrate_relationships/` (B-D8)
  - 입력: v0.6 시나리오 JSON 디렉토리
  - 출력: v0.7 시나리오 JSON
  - 자동 변환: `trust × 100` → trust / `closeness × 100` → affinity / BondKind 기반 respect/wariness baseline (B-D10) / `power` 필드 삭제 / default 필드 채움
  - 옵션: `--dry-run` / `--diff` / `--backup-dir`
- Claude prompt template 3 파일 (`docs/migration/claude-prompts/`):
  - `bond-kind-inference.md` — 시나리오 페어별 BondKind 추론
  - `type-text-inference.md` — type 자유 텍스트 추론
  - `adjustment-suggestion.md` — narrative 결과 기반 조정 제안 (Stage 5에서 사용)
- 백업 디렉토리 이동: `data/scenarios.backup-v0.6/` + `data/sessions.backup-v0.6/`
- 디자이너 + Claude 협업: ~45 페어 BondKind/type 박기
- 마이그레이션 도구 실행 → v0.7 JSON 생성
- `_schema.md` v0.6 → v0.7 갱신 (Relationship 섹션 한정)

**게이트**:
1. 마이그레이션 도구 자체 단위 테스트 통과
2. 모든 시나리오 JSON v0.7 schema 통과 (serde 역직렬화 + 도메인 validation)
3. 마이그레이션 후 컴파일 + 1095+ tests 통과
4. 디자이너 BondKind/type 검토 통과 (Claude 추론 결과 합당성 확인)

**산출 commit**: `phase2-stage4-migration.md` 회고

---

### Stage 5 — Narrative 시뮬레이션 검증

**범위**:
- Phase 1 narrative 3 시나리오 (chitchat-passerby/daily-training/lin-chong-shanshenmiao) 4축 시스템에서 재실행
- S1~S4 시뮬레이션 케이스를 *Phase 2 narrative test*로 박음 (`tests/phase2_narrative_test.rs` 신설)
  - 각 케이스 ground truth (기대 4축 변동) 명시
  - base_delta + HEXACO + (axis_modulation Phase 2.5 전이므로 모두 "default") 적용 결과 확인
- session_*_result.json 일괄 재생성 (B-D9)
- 어색 케이스 식별 → Claude AI 추론으로 조정 제안 (`adjustment-suggestion.md` 사용) → 디자이너 검토 → JSON 미세조정
- 시나리오별 narrative report

**게이트**:
1. 3밴드 calibration 보존 (chitchat 0.000 / daily 0.461 / shanshenmiao 0.980 ± tolerance — D3 baseline 비교)
2. S1~S4 ground truth ±N 이내
3. 4축 변동이 시나리오 의도와 정합 (디자이너 narrative 검토 통과)
4. 어색 케이스 0건 (또는 모두 디자이너 조정 완료)

**산출 commit**: `phase2-stage5-narrative.md` 회고

---

### Stage 6 — Bench + 회고 + Phase 2.3 KICKOFF

**범위**:
- `dispatch_v2(EndDialogue)` 재측정 (chitchat/significant/legacy) — D2 baseline 비교
- `compute_significance` 재측정 — D4 baseline 비교
- `MAX_EVENTS_PER_COMMAND = 22` 안전성 재확인 (A5 worst-case 산출 검증)
- 4축 매핑 추가의 latency 영향 측정 (Stage 2의 base_delta lookup + HEXACO 보정 비용)
- Phase 2 checkpoint report 작성 (`phase2-checkpoint-report.md`)
- **Phase 2.3 KICKOFF 작성** (`PHASE2.3-KICKOFF.md`) — 다음 phase 인계
  - Phase 2.3 spec 작성 준비 (`task-rel-phase2.3-appraise-tuning.md`)
  - 시뮬레이션 시나리오 set 신설 디렉토리 (`data/scenarios/appraise-validation/`)
- 외부 문서 인덱스 갱신:
  - `CLAUDE.md` Mind Architecture Phase 2 ✅ 표기
  - `00-roadmap.md` §5 Phase 2 완료 표기 + §6.5 §1~§3 진척 갱신
- spec `task-rel-phase2-domain-migration.md` v1.0 frozen 표기

**게이트**:
1. Latency ±20% 이내 (D2 baseline 비교, 4축 매핑 추가 영향 측정값 박제)
2. Bench 재측정 완료
3. 회귀 0건 (1095+ tests 통과 + narrative 3밴드 보존)
4. Phase 2.3 진입 준비 완료 (KICKOFF + spec 디렉토리 초안)
5. 외부 문서 인덱스 동기화 완료

**산출 commit**: `phase2-stage6-bench-handoff.md` 회고 + `PHASE2.3-KICKOFF.md`

---

## Stage 0 종결

본 spec의 §0~§7 모든 절 작성 완료. Phase 2 본체 결정 12개 박힘. Stage 1 진입 준비 완료.

**다음 작업** (Stage 1 진입):
1. `baselines/cargo-test-2026-MM-DD-PASS.log` 재측정 박제
2. Stage 1 `feat/phase2-stage1-domain` 브랜치 작성
3. `src/domain/relationship/axis.rs` 신설부터 시작

---

## 변경 이력

| 버전 | 날짜 | 변경 |
|---|---|---|
| 0.1 | 2026-05-13 | 초안. A 카테고리 Findings 5개 종결, B 카테고리 진행 중 (B-D4 확정). |
| 0.2 | 2026-05-13 | §3.6 시뮬레이션 검증 (S1~S4) 추가. B-D6/D12/D13/D14 확정. **Phase 2.3 신설 결정**. §5 Risks 작성 (R1~R6). §1 Scope에 Phase 2.3/2.5 비포함 항목 + axis_modulation 표기 추가. |
| 0.3 | 2026-05-13 | B-D1 (Score 타입 분리) + B-D2 (f32 내부 표현) 확정. `AxisScore` + `WarinessScore` 2 타입 신설 결정. R3 해소 표기. |
| 0.4 | 2026-05-13 | B-D3 (closeness → affinity 혼합 변환) 확정. 자동 변환 baseline + 디자이너 선택적 조정. |
| 0.5 | 2026-05-13 | B-D10 (respect/wariness 초기값 룰) 확정. 간단 휴리스틱 + BondKind 보완 (원수/Guardian/Mentor/지기 차등 baseline). |
| 0.6 | 2026-05-13 | B-D5 (4축 매핑 함수 구조) 확정. v0.7 §4.1 단일 함수 그대로 (4축 동시 갱신). |
| 0.7 | 2026-05-13 | B-D8 (시나리오 마이그레이션 워크플로우) 확정. W3+ — Rust binary 자동 변환 + Claude AI 추론 (BondKind/type/조정) + 디자이너 검토. Claude prompt template 박음 (`docs/migration/claude-prompts/`). R2 대폭 완화 표기. |
| 0.8 | 2026-05-13 | **★ B 카테고리 Phase 2 본체 12개 결정 완료**. B-D9 (session_*_result.json 폐기) 확정 — Phase 2 후 일괄 재생성. B-D7/B-D11은 Phase 2.5 시점 결정. §4 헤더 종결 표기. |
| 0.9 | 2026-05-13 | **★ §6 Baseline (D 카테고리) 작성**. Phase 1 종결 시점 baseline 인용: 1095 tests passed / dispatch_v2 latency 24/35/29µs / narrative 3밴드 0.000/0.461/0.980 / compute_significance 8.36µs / EventKind 31개. D1~D6 항목. Stage 1 진입 직전 재측정 작업 명시. |
| 1.0 | 2026-05-13 | **★ Stage 0 종결**. §7 Stages 작성 — 6 stage 분할 (Stage 1 Type/Domain → Stage 2 Mapping → Stage 3 Updater → Stage 4 Migration → Stage 5 Narrative → Stage 6 Bench/Handoff). 각 stage 범위·게이트·산출 commit 명시. Phase 2 본체 spec 작성 완료, Stage 1 진입 준비. |
