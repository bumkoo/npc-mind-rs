# Phase 2.3 KICKOFF — Appraise Tuning + ±100 Native 전환

**Stage**: Phase 2 종결 → Phase 2.3 진입 (Stage 4-6 사이 또는 별도 트랙)
**전제**: Phase 2 Stage 3 종결 ([phase2-stage3-domain-wire-frontend.md](phase2-stage3-domain-wire-frontend.md)). Stage 4 (시나리오 마이그레이션) 완료 가정.
**Phase 2.3 spec**: 본 문서는 KICKOFF (인계). 정식 spec은 `task-rel-phase2.3-appraise-tuning.md`로 분리 작성.

---

## §1. Phase 2.3 범위 (상위 골격)

### A) ±100 native 전환 — 잔존 ÷100 layer 청소

Stage 3는 wire boundary 4겹 (domain → application → adapter → frontend)에서 ÷100 layer를 제거했으나, *도메인 내부* 2 사이트 + 1 uncatalogued 사이트가 ±1.0 가정으로 잔존:

1. **`src/application/command/telling_ingestion_handler.rs:80`**
   ```rust
   .map(|r| (r.trust().value() / 100.0 + 1.0) / 2.0)
   ```
   confidence 정규화 공식 `(t + 1) / 2 ∈ [0,1]`이 ±1.0 가정. ±100 native로 갱신:
   ```rust
   .map(|r| (r.trust().value() + 100.0) / 200.0)
   ```
   (수학적 동치, 정규화 결과 동일.)

2. **`src/domain/relationship/mod.rs:172-173`** — `modifiers()` 내부 정규화
   ```rust
   let affinity_norm = self.affinity.value() / 100.0; // -1.0..1.0
   let trust_norm = self.trust.value() / 100.0;
   ```
   `RelationshipModifiers` 가중치 (`rel_closeness_intensity_weight` 등)가 ±1.0 정규화된 값 가정. ±100 native 전환 시 가중치 자체를 1/100 스케일로 재조정해야 결과 동일. **modifier 가중치 표 전수 재조정 (Phase 2.3 의 ÷100 제거 핵심 작업)**.

3. **`src/domain/guide/snapshot.rs:316-317`** — `RelationshipLevel::from_score()` 호출
   ```rust
   closeness_level: RelationshipLevel::from_score(rel.affinity().value() / 100.0),
   trust_level: RelationshipLevel::from_score(rel.trust().value() / 100.0),
   ```
   `RelationshipLevel` enum의 `from_score()` 시그니처가 ±1.0 가정. 4 옵션:
   - (a) `from_score()` 시그니처 ±100 변경 (tuning const `level_very_high_threshold` 등 재조정 수반)
   - (b) `RelationshipLevel` enum을 4축 raw 값에서 직접 매핑 (Phase 2.3 spec 결정)
   - (c) Phase 2.3에서 `closeness_level`/`trust_level`을 4축 (trust/affinity/respect/wariness) 다축 라벨로 확장 (B-D-D 결정 영향)
   - (d) presentation layer를 `power_level` 폐기 + `affinity_level`/`respect_level`/`wariness_level` 등 추가

### B) Appraise 입력 정밀화 (S1~S4 시뮬레이션 결과 반영)

Stage 0 §3.6 simulation 검증에서 발견된 *appraise 입력 의존성* 정밀화. S2(임충→육겸 산신묘) 등 narrative 시뮬에서 base_delta + HEXACO 보정만으로 narrative 의도와 어긋난 케이스 보정.

- `axis_modulation` 인자 검토 (Phase 2.5 영역 — 일부 Phase 2.3 선행 가능)
- HEXACO 6 → 24 facet spread 재조정 (`worldbuilding::mind_sync::person_to_npc`)
- Reflection 게이트 (significance) 자체 조정 가능성 검토

### C) memory_relationship_delta_threshold 4축 합산 sensitivity 정밀화

Stage 3 → threshold 0.05 → 5.0. 4축 합산은 `|Δtrust| + |Δaffinity| + |Δrespect| + |Δwariness|`. 4축 동시 변동 시 *over-trigger* 가능성. 옵션:
- (a) 최대값 1개 기준 (`max(|Δtrust|, |Δaffinity|, |Δrespect|, |Δwariness|)`)
- (b) 가중 합 (예: trust×0.4 + affinity×0.4 + respect×0.1 + wariness×0.1)
- (c) 1축이라도 threshold 5.0 넘으면 통과 (OR 조건)

Phase 2.3 narrative 시뮬 결과 보고 결정.

### D) W1 회귀 가드 expected 값 재조정 (Phase 2.3 트리거 보존)

`src/domain/relationship/mapping.rs::tests`:
- `beat_rel_modifiers_affinity_channel_after_anger` — expected `0.286 = affinity 28.6/100`
- `beat_rel_modifiers_trust_channel_after_anger` — expected `0.158 = trust 15.8/100`
- `beat_rel_modifiers_admiration_no_leak_until_phase_2_3` — 4 modifier 불변

`modifiers()` ±100 native 전환 시 이 expected 값이 깨지는 게 정상 (±100 raw 스케일로 재조정 필요). Phase 2.3 시작 신호 트리거 — 이 테스트가 PASS인 동안은 modifier API 변경 *안 함*. 깨지는 순간 Phase 2.3 modifier 정밀화 진입.

#### W1 expected 재조정 표 (Phase 2.3 작업 시 적용)

| 테스트 | Stage 3 expected (±1.0) | Phase 2.3 target (±100) | 비고 |
|---|---|---|---|
| `affinity_channel_after_anger` | `0.286` | `28.6` | 단위만 변경 (100배) |
| `trust_channel_after_anger` | `0.158` | `15.8` | 단위만 변경 |
| `admiration_no_leak_until_phase_2_3` | 4 modifier 불변 | — | Admiration이 더 이상 *no-leak* 이 아닌 경우 (modifier 변경) 시 *제거 가능 정상* |

### E) v0.6 시나리오 JSON 로드 transient bug 청소

Stage 3에서 `RelationshipData`에 `#[serde(alias = "closeness")]` 추가로 v0.6 JSON 호환 *deserialize*만 가능. 값 의미는 어긋남 (closeness=0.5가 affinity=0.5로 박힘). Stage 4 migration 후 Phase 2.3 진입 시 시나리오 JSON이 ±100 schema이므로 이 alias 제거 검토:

```rust
// Stage 4 migration 완료 후 Phase 2.3에서 제거 검토:
#[serde(alias = "closeness")]
pub affinity: f32,
```

`#[serde(default)]` on respect/wariness는 v0.7 JSON에도 *backward compat* 보장 차원에서 유지 권장 (디자이너가 일부 axes 생략한 경우).

---

## §2. R-3b memory content 라벨 혼재 — 인지 + 미해소

Stage 3 후의 `RelationshipMemoryHandler::dominant_delta` 라벨:
- Stage 2 (이전): `closeness` / `trust` / `power`
- Stage 3 (현재): `trust` / `affinity` / `respect` / `wariness`

기존 memory entries (Stage 2 이전에 생성)는 `[closeness Δ=0.34]` 같은 v0.6 라벨 그대로 잔존. 재마이그레이션 *안 함* — content text는 dialogue/scene snapshot의 일부이고 retroactive rewrite는 trace 무결성을 깨뜨림.

**Phase 2.3 narrative 시뮬에서 검색 시**:
- 양쪽 라벨 fallback 처리 권장 (예: `closeness` → `affinity` regex 매핑)
- Phase 2.3 narrative 시뮬 시점부터의 entries는 모두 v0.7 라벨이므로 신규 검색 결과는 일관

---

## §3. Stage 3 → Phase 2.3 인계 메트릭 baseline

`docs/tasks/mind-architecture/baselines/stage3-*-2026-05-16.log` 모든 파일 참조.

### Stage 3 종결 시점 메트릭 표

| 메트릭 | Stage 3 종결 |
|---|---|
| ÷100 production 위치 (logical) | 2 (domain modifiers + RelationshipLevel) + 1 uncatalogued (telling_ingestion) |
| ×100 production 위치 | 2 (memory_repository v0.6 deserializer + v2_scenes v0.6 endpoint) |
| W4 마커 (production) | 2 (relationship_policy helper + stimulus_policy beat) |
| `closeness`/`power` wire payload 잔존 | 0 (payload + DTO + frontend) |
| `cargo test --features chat` 통과 | 871 (lib + tests) + 72 (mind-studio bin) = 943 |
| D2 latency (chitchat / significant / legacy) | 7 / 10 / 8 µs |
| D3 3밴드 calibration | 0.000 / 0.461 / 0.980 (Stage 1 baseline exact match) |

### Phase 2.3 진입 시 재측정 권장 항목

1. `cargo test --features chat` 871 → ? (modifier ±100 native 전환 후 회귀 확인)
2. D2 latency 재측정 (modifier 가중치 재조정으로 ±1.0 정규화 부담 제거 시 -5% 추정)
3. D3 narrative band — modifier 정밀화로 변동 가능 (Phase 2.3 narrative 검증 필수)
4. Phase 2.3 narrative 시뮬: `data/scenarios/appraise-validation/` 디렉토리 신설 + S1~S4 case 박제 (`tests/phase2_3_narrative_test.rs` 또는 동등)

---

## §4. 작업 순서 권장

1. **Stage 4 시나리오 마이그레이션 도구 작성** ([task-rel-phase2-domain-migration.md](task-rel-phase2-domain-migration.md) §7 Stage 4) — 시나리오 JSON `closeness/trust/power × ±1.0` → `trust/affinity/respect/wariness × ±100` 자동 변환
2. **Stage 5 narrative 시뮬레이션** — 3밴드 calibration + S1~S4 ground truth 재검증
3. **Stage 6 Bench + 회고** — Phase 2 종결
4. **Phase 2.3 진입**:
   - `task-rel-phase2.3-appraise-tuning.md` spec 작성 (본 KICKOFF 기반)
   - `data/scenarios/appraise-validation/` 디렉토리 신설
   - ±100 native 전환 (telling_ingestion → modifiers → snapshot 순)
   - W1 expected 값 ±100 스케일로 재조정
   - threshold 4축 합산 sensitivity 결정
   - Appraise 정밀화

---

## §5. 변경 이력

| 버전 | 날짜 | 변경 |
|---|---|---|
| 1.0 | 2026-05-16 | Stage 3 종결 시점 작성. 잔존 ÷100 3 위치 / W1 깨지는 트리거 / W1 expected 재조정 표 / R-3b memory 혼재 / R-3g threshold 정밀화 / v0.6 transient bug 청소 / Phase 2.3 작업 순서 권장. |
