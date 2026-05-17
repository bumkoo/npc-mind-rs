# Phase 2.3 KICKOFF — Appraise Tuning + ±100 Native 전환

**Stage**: Phase 2 **종결 ✅ (Stage 0~6 완료, 2026-05-16)** → Phase 2.3 진입.
**전제**: Phase 2 전체 종결 — [`phase2-checkpoint-report.md`](phase2-checkpoint-report.md) (Stage 1~6 종합) + [`phase2-stage6-bench-handoff.md`](phase2-stage6-bench-handoff.md) (회고). Stage 3~6 회고가 인계 사항의 정본 소스.
**Phase 2.3 spec**: 본 문서는 KICKOFF (인계). 정식 spec은 [`task-rel-phase2.3-appraise-tuning.md`](task-rel-phase2.3-appraise-tuning.md)로 분리 작성 (Stage 6에서 초안 신설).

> ⚠️ **정정 포인터 (Phase 2.3 확인①, 2026-05-17, P-D-1)**: 본 KICKOFF §1-C(§C "4축 합산")·§1-E(§E "Stage 4 미처리 커스텀 Deserialize 잔존") 서술은 Stage 0 사실조사 실측으로 **정정됨**. 정본 = FROZEN spec [`task-rel-phase2.3-appraise-tuning.md`](task-rel-phase2.3-appraise-tuning.md) **§0.5**. 본 KICKOFF 원문은 이력 보존 위해 미수정 — 정정 내용은 FROZEN spec §0.5 우선.

---

## §1. Phase 2.3 범위 (상위 골격)

### A) ±100 native 전환 — 잔존 ÷100 layer 청소

Stage 3는 wire boundary 4겹 (domain → application → adapter → frontend)에서 ÷100 layer를 제거했으나, *도메인 내부* 2 사이트 + 1 uncatalogued 사이트가 ±1.0 가정으로 잔존:

> ★ **Stage 6 추가 플래그 (S6-D5)** — spec L508 "closeness/power production 0건" vs **실측 12 파일 / 69 매치**.
> Stage 6에서는 *정리 안 함* (W1 회귀 가드 = Phase 2.3 진입 트리거 조기 발파 위험). Phase 2.3 §A 진입 시 **정확 위치 재카탈로그 필수**.
>
> 분포 힌트 (정확한 분류는 Phase 2.3에서):
> - **튜닝 파라미터 명칭** (logical 이름 유지): `domain/tuning.rs` (`closeness_update_rate`, `rel_closeness_*` 가중치) — 의미상 affinity 가중치이므로 *이름만* `affinity_*`로 rename 후보. 값/동작 변경 0
> - **modifiers 내부 ÷100** (이 문서 §1-A 본문 2): `domain/relationship/mod.rs`, `domain/relationship/mapping.rs` — modifier 가중치 표 재조정 핵심 작업
> - **RelationshipLevel from_score ÷100** (이 문서 §1-A 본문 3): `domain/guide/snapshot.rs`
> - **UI/locale presentation** (3축 라벨 잔존): `presentation/{locale,formatter}.rs`, `bin/mind-studio/{state,domain_sync,handler_tests}.rs` — `closeness_level`/`power_level` 라벨/슬롯. B-D-D 한글 라벨은 Stage 3에서 frontend에 추가됐으나 *backend presentation 슬롯은 3축 그대로*. Phase 2.3에서 4축 라벨로 확장 (closeness_level → affinity_level + respect_level 등). `power_level` 폐기 (B-D4)
> - **adapter test doc string**: `adapter/memory_repository.rs:634-636` — 더미 v0.6 JSON 예시 (테스트용), 의미 영향 0
> - **telling_ingestion ÷100** (이 문서 §1-A 본문 1): `application/command/telling_ingestion_handler.rs:80`
> - **emotion situation**: `domain/emotion/situation.rs` — 분류 필요 (별 의미 가능성: closeness가 NPC↔partner 평가 함수 인자명)
> - **worldbuilding/markdown**: 미발견 (별 의미 = 장소 인접성)이라 *별도 도메인* 가능성 — Phase 2.3에서 분해 확인
>
> Phase 2.3 §A 첫 작업 = **12 파일 69 매치 정확 분류 + 변경 카테고리화** (rename / 값 변경 / 폐기 / 별 의미 분리).

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

### E) v0.6 시나리오 JSON 로드 — 커스텀 Deserialize (Stage 4 미처리 확정, Phase 2.3 인계 유지)

> ★ **Stage 6 갱신 (S6-D5)** — Stage 4 회고 §4-C3 "state.rs:680~734 커스텀 Deserialize impl + 5 테스트 *제거*"가 산문으로 박혔으나 **실측 코드 확인 결과 잔존**: `src/bin/mind-studio/state.rs:666~671` 부근. Stage 4 책임 2번 항목은 *미처리 확정*. Stage 6에서는 *건드리지 않음* (Phase 2.3 진입 트리거 조기 발파 위험). Phase 2.3 §E 본문이 정본 위치.

Stage 3 리뷰 H1 (save-roundtrip data loss) 반영 → `RelationshipData`에 **커스텀 `Deserialize` impl** 도입 (`state.rs:680~`). serde alias가 아니라 스키마 감지 + 값 의미 보존:

```rust
// state.rs:680~ 실제 구현:
impl<'de> Deserialize<'de> for RelationshipData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> { ... }
}
// :708  let is_v06 = raw.closeness.is_some() || raw.power.is_some();
// :714  tracing::warn!("RelationshipData v0.6 schema detected ...")
// :719  let affinity = raw.closeness.unwrap_or(0.0) * 100.0;  // 값 의미 보존
```

- v0.6 키(`closeness`/`power`) 감지 시 `trust × 100`, `closeness × 100 → affinity` **자동 변환** (closeness=0.5 → affinity=50, 값 의미 보존 — KICKOFF v1.0 초안의 "값 의미 어긋남"은 H1 반영 전 상태, 정정됨)
- `power`는 폐기 (B-D4) — 값 무시
- `tracing::warn!`로 자동 변환 표시 + Stage 4 마이그레이션 권장 로그
- v0.7 입력 (trust/affinity 키 + ±100)은 그대로 통과 (자동 변환 없음)
- 5 신규 단위 테스트 (`state.rs:927 mod relationship_data_tests`): `v06_schema_auto_multiplies_by_100` / `v07_schema_passes_through_without_scaling` / `v06_power_alone_triggers_migration` / `save_roundtrip_v06_to_v07_preserves_semantic` / `v07_missing_optional_fields_defaults_to_zero`

**결과**: v0.6 시나리오 JSON 그대로 로드해도 ±100 raw로 자동 마이그레이션 + 값 의미 보존. save-roundtrip data loss 위험 해소.

**Stage 4 책임 (회고 §7-E와 일치)**:
1. 시나리오 JSON 파일 자체를 v0.7 4-필드 스키마로 **영구 변환** (원 B-D8 책임)
2. **`RelationshipData` 커스텀 `Deserialize` impl (`state.rs:680~`) + 5 테스트 제거** (신규 책임 — H1이 만든 것). ★ 제거 대상은 *serde alias가 아니라 커스텀 Deserialize impl 전체* — v1.0 초안의 `#[serde(alias = "closeness")]` 제거 서술은 코드와 불일치 (그런 alias 없음), 본 항목으로 정정됨
3. Stage 4 완료 게이트에 *v0.6 자동 변환 코드 완전 제거 검증* 추가 (dead code 영구 잔존 방지)

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

## §3. Phase 2 종결 → Phase 2.3 인계 메트릭 baseline

**측정 명령 (S5-D5 / S6-D1 정본, Stage 5·6 일관 고정)**: `cargo test --lib --tests --bins`

`docs/tasks/mind-architecture/baselines/stage{3,4,5,6}-*-2026-05-16.log` 참조.

### Phase 2 종결 시점 메트릭 표 (Stage 6 박제 — 본 KICKOFF 정본 baseline)

| 메트릭 | Stage 3 종결 (참고) | Stage 4 종결 (참고) | **Stage 5 종결 = Stage 6 진입 = Phase 2 종결** |
|---|---|---|---|
| failed | 0 | 0 | **0** |
| passed (`--lib --tests --bins`) | 871 (`--features chat --lib --tests`) | 875 (동일 측정 명령) | **843** (S5-D5 정본 명령) |
| ignored | 5 | — | **2** (daily/shanshenmiao 해제) |
| result 묶음 | — | — | **65** |
| ÷100 production 위치 (logical) | 2 + 1 uncatalogued | 동일 | 동일 (telling_ingestion + modifiers + RelationshipLevel) + **closeness/power src 12 파일 / 69 매치 (S6-D5 신규 카탈로그)** |
| ×100 production 위치 | 3 | **0** (Stage 4 v0.6 code 0건화) | 0 유지 |
| W4 마커 (production) | 2 | — | 2 (relationship_policy helper + stimulus_policy beat) |
| `closeness`/`power` wire payload 잔존 | 0 | 0 | 0 (payload + DTO + frontend) |
| D2 latency chitchat / significant / legacy (debug 빌드) | 7 / 10 / 8 µs (release N=50) | — | **15.70 / 26.68 / 20.89 µs** (Stage 6, 임계값 29/42/35.2 전부 이내) |
| D4 10turn×10000 avg | — | — | **9.77 µs/call** (임계값 ~10 µs ±20% 이내) |
| D3 3밴드 calibration | 0.000 / 0.461 / 0.980 (exact) | (chitchat 1 pass, 나머지 ignored) | **0.000 / 0.461 / 0.980** (exact, Stage 1 baseline 동치 사슬 C-3 보존) |
| git HEAD | — | — | **Phase 2 종결 = `9339909` (PR #93 머지)**. 참고: Stage 6 진입/FROZEN = `e3df875`, Stage 5 종결 = `5b2b798` (PR #92) |

### Phase 2.3 진입 시 재측정 권장 항목

1. `cargo test --lib --tests --bins` — 843 → ? (modifier ±100 native 전환 후 회귀 확인). S5-D5 정본 정의 *(failed=0 ∧ 회귀 0 ∧ 증감 설명가능 ∧ D3 3밴드 보존)* 적용.
2. D2 latency 재측정 (modifier 가중치 재조정으로 ÷100 부담 제거 시 -5% 추정 — Stage 6 진입 실측 대비)
3. D3 narrative band — modifier 정밀화로 변동 가능 (Phase 2.3 narrative 검증 필수)
4. Phase 2.3 narrative 시뮬: [`data/scenarios/appraise-validation/`](../../../data/scenarios/appraise-validation/) 디렉토리 (Stage 6에서 신설) + S1~S4 case 박제 (`tests/phase2_3_narrative_test.rs` 또는 동등)
5. W1 회귀 가드 expected 값 재조정 (§1-D 표)

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

## §5. Stage 4·5 → Phase 2.3 인계 5항 (Stage 6 신규 — S6-D3)

Stage 4·5 회고 §부채/잔여로 박힌 5항. Phase 2.3 정식 spec 작성 전 *전수 검토* 권장.

### 5.1. **`session_*_result.json` 자동 dump 인프라 부재** (B-D9, Stage 5 §5 작업5 + §7)

- **현황**: `state.rs::save_to_file(path, as_scenario=false)` 만 result.json writer. Mind Studio REST `/api/save` 핸들러를 통한 *인터랙티브 사용자 액션* 전용. 테스트(`tests/*.rs`), `dispatch_v2`, narrative 시나리오 어디에도 `serde_json::to_writer` + `result.json` 자동 dump 패턴 없음. 기존 `data/_discarded-v0.6/treasure_island/.../session_*_result.json` 3건 = 과거 인터랙티브 세션 산출. 재생성 대상 0건.
- **Phase 2.3 위임**: narrative 시뮬레이션 자동 dump CLI (`cargo run --bin narrative-dump -- --scenario S1` 등) 신설 후 결과 일괄 재생성. 위험 *하* (regression guard는 Stage 5 §4 작업 3·4가 이미 보장).

### 5.2. **작업 1 intensity 0.4 잠정 확정** (Stage 5 §6.1)

- **현황**: `tests/phase1_daily_training_test.rs::set_intensity(EmotionType::Admiration, 0.4)` 는 *잠정값*. 디자이너 검토 시 1택:
  - **0.4 유지** — 일상 가르침의 "어제보다 안정됐다" 톤
  - **상향** (0.5~0.6) — mid 밴드 axes 변동이 미세→중강 필요 시
  - **하향** (0.3) — mid 밴드 axes 변동 *너무 큼* 판정 시
- **조정 시 동기**: `daily-training.json` `_expected_axes_delta` 문구 + Stage 5 회고 §4 작업 1 박제값 (S5-D4)

### 5.3. **S1~S3 narrative 타당성 검토** (Stage 5 §6.2)

Stage 5 §4 작업 3 표 EXPECTED는 "현재 코드의 출력"이지 "디자이너 의도와 정합" 보증 아님. 게이트 3 디자이너 검토 항목:

| 케이스 | 박제값 (trust/aff/resp/war) | 검토 포인트 |
|---|---|---|
| S1 임충→노지심 | (64.4, 46.0, 32.0, 0.0) | respect +12, trust +14.4. 의리·은혜 갚음. Admiration+Gratitude 효과 합치 적절? |
| S2 임충→육겸 | (3.8, 3.0, -4.0, 42.5) | trust 50→3.8 (거의 0). 옛 친구의 처단 → 완전 단절. wariness +42.5 (50% 도달) — 과한지/부족한지 |
| S3 수련→옥교룡 | (25.216, 26.64, -1.76, 32.32) | trust -14.8 / respect -11.8 / wariness +12.3. 안타까움+책망+분노. 사부의 *체념과 한* 표현 적절? |

어색 시: 입력 emotion intensity 조정 → 코드 재실행 → 새 EXPECTED 박제 → Stage 5 §4 작업 3 표 갱신. 게이트 2 tolerance 완화 *금지* (S5-D4).

### 5.4. **S4 임충→고구 정성 검증 + 시간 분산** (Stage 5 §6.3)

- §3.6 focus 수치 *의도적 부재* (S5-D2). 임충 對고구 감정 = *체제 정점에 대한 누적 분노* — 단일 시점 EmotionState mock으로 박제 어려움 (시간 분산 + 권력 거리).
- "3 layer separation" (서사·인지·정서)이 4축 변동에 자연스럽게 반영되는지 *서사 직관*과 정합 검증.
- **Phase 2.3+ 정량화 진입점**: 시간 분산 모델 + `axis_modulation` 활성화 (B-D6) 시 정량 가능. `axis_modulation` 자체가 reflection LLM 출력 필드로 신설되어야 함 (S5-D1 박제 — `ReflectionResult` 7 필드 현재 부재).

### 5.5. **Stage 4·5 메트릭 baseline** (S6-D3 통합 표시)

- Stage 3 종결 시점 baseline은 본 KICKOFF v1.0/1.1 §3 표 (참고치).
- Stage 4 종결: 875 passed (`--features chat --lib --tests`) + Mind Studio 90 passed. v0.6 grep 0건. (회고 §1·§3)
- Stage 5 종결: **843 passed / 0 failed / 2 ignored / 65 묶음** (`--lib --tests --bins`, S5-D5 정본). D3 3밴드 exact 보존.
- Stage 6 진입 = Stage 5 종결 동등. Stage 6 코드 변경 0. (S6-D1)
- Phase 2.3 진입 시 본 baseline 대비 회귀 0 확인 필수.

---

## §6. 역대조 게이트 (C6-2 — Stage 6 신규)

Stage 4 회고 §5 + Stage 5 회고 §6·§7 잔여표 ↔ 본 KICKOFF 항목 1:1 매핑 확인. **누락 0건**.

| Source 회고 | 항목 | KICKOFF 위치 |
|---|---|---|
| Stage 4 회고 §5-1 | D3 sanity 직접 측정 (daily/shanshenmiao `#[ignore]` 해제 후) | Stage 5 작업 1·2에서 해소 (게이트 표시 — Stage 6에서 D3 0.000/0.461/0.980 exact 재확인) |
| Stage 4 회고 §5-2 | 휴리스틱 신규 2축 narrative 검토 (respect = closeness × 50 / wariness = max(0, -trust × 50)) | §5.3 (S1~S3 narrative 타당성) — Phase 2.3 디자이너 검토 |
| Stage 4 회고 §5-3 | bond_kind 명시 진입 (전 페어 미지정 → MasterDisciple/Betrayer 등) | §5.3 (narrative 검토 결과 따라 진입) — 도메인 `modifiers()` 비참조이므로 D3 영향 없음 |
| Stage 4 회고 §5-4 | `session_*_result.json` 재생성 (B-D9) | §5.1 (자동 dump 인프라 부재 — Phase 2.3/3 위임) |
| Stage 4 회고 §5-5 | `_discarded-v0.6/` 영구 폐기 결정 | (Phase 2.3 본체 진입 시 디자이너 결정 — 본 KICKOFF *아래* 비스코프) |
| Stage 5 회고 §6.1 | 작업 1 intensity 0.4 잠정 확정 | **§5.2** |
| Stage 5 회고 §6.2 | S1~S3 박제값 narrative 타당성 | **§5.3** |
| Stage 5 회고 §6.3 | S4 임충→고구 정성 검증 | **§5.4** |
| Stage 5 회고 §7-1 | listener_perspective default-ON 발견 (사실 정정) | (본 KICKOFF 비스코프 — Phase 2 종합 보고서 §6.2 박제) |
| Stage 5 회고 §7-2 | examples `phase5b_checkpoint2_eval` 빌드 실패 | (본 KICKOFF 비스코프 — Phase 5 후속 처리) |
| Stage 5 회고 §7-3 | result.json 자동 dump 인프라 부재 | **§5.1** |
| Stage 5 회고 §7-4 | 작업 1 intensity 0.4 잠정 | **§5.2** |
| Stage 5 회고 §7-5 | S4 정성 검증 | **§5.4** |
| Stage 6 회고 (신규) | closeness/power src 12 파일 / 69 매치 재카탈로그 | **§1-A 본문 박스 + §3 메트릭 표** |
| Stage 6 회고 (신규) | state.rs:666~671 커스텀 Deserialize 잔존 | **§1-E 본문 박스** |

---

## §7. 변경 이력

| 버전 | 날짜 | 변경 |
|---|---|---|
| 1.0 | 2026-05-16 | Stage 3 종결 시점 작성. 잔존 ÷100 3 위치 / W1 깨지는 트리거 / W1 expected 재조정 표 / R-3b memory 혼재 / R-3g threshold 정밀화 / v0.6 transient bug 청소 / Phase 2.3 작업 순서 권장. |
| 1.1 | 2026-05-16 | §1-E 정정: 실제 코드는 `#[serde(alias)]`가 아니라 커스텀 `Deserialize` impl (state.rs:680~) — 값 의미 보존 (closeness 0.5 → affinity 50). 회고 §5/§7-E와 일치. Stage 4 제거 대상 = 커스텀 Deserialize impl 전체 + 5 테스트 (존재하지 않는 serde alias 아님). 코드 검증으로 확정. |
| **1.2** | **2026-05-16** | **Stage 6 갱신 (S6-D3/D5/C6-2)**: §1-A 플래그 박스 추가 (spec L508 "production 0" vs 실측 12 파일 / 69 매치 분포 힌트) + §1-E 갱신 박스 (Stage 4 미처리 확정 → Phase 2.3 인계 유지) + §3 baseline Phase 2 종결값으로 갱신 + **§5 신규 5항 인계 섹션** (result.json dump 부재 / intensity 0.4 / S1~S3 narrative / S4 정성·시간분산 / Stage 4·5 메트릭) + **§6 역대조 게이트** (Stage 4·5 잔여 ↔ KICKOFF 1:1 매핑, 누락 0). 기존 157줄 보존. |
