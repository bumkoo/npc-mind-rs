# Phase 2 Stage 4 회고 — 시나리오 v0.7 영구 변환 + v0.6 코드 0건화

**Stage**: Phase 2 Stage 4 (Spec v1.4 frozen → 2026-05-16 코딩 완료)
**범위**: 시나리오 4파일 v0.7 영구 변환 + v0.6 코드 3경로 (memory_repository RelationshipJson + state.rs 커스텀 Deserialize + v2_scenes legacy endpoint) 제거 + 데이터 폐기 2건 (huckleberry_finn / treasure_island) + `_schema.md` v0.7 갱신.
**Spec**: [`task-rel-phase2-domain-migration.md`](task-rel-phase2-domain-migration.md) §7 Stage 4 (4.1~4.6).

---

## §1. 컴파일 + 테스트 게이트 (4.6 종결 게이트 #1)

| 항목 | Stage 3 baseline | Stage 4 결과 |
|---|---|---|
| `cargo check --features chat` | ✅ | ✅ |
| `cargo test --features chat --lib --tests` | 871 passed | **875 passed** / 0 failed (`baselines/stage4-cargo-test-2026-05-16-PASS.log`) |
| Mind Studio bin tests (`--bin npc-mind-studio`) | 95 passed (※) | **90 passed** / 0 failed (`baselines/stage4-mindstudio-2026-05-16-PASS.log`) |

(※) Stage 4 진입 직전 측정 — `stage4-prep-mindstudio-2026-05-16-PASS.log`. Spec v1.4 §6 D1은 "Mind Studio 77→72+N_v07"로 적었으나 실측 baseline은 95 (스펙 작성 시점 이후 다른 테스트 추가가 누적된 것으로 보임 — 게이트 본질은 *회귀 0*이므로 영향 없음).

**회귀 0건**:
- lib+tests: 871 → 875 (+4 신규 `relationship_json_tests`, 즉 N_v07 = 4 ≥3 spec 요구사항 충족)
- Mind Studio bin: 95 → 90 (−5 책임 B `relationship_data_tests` H1 부산물 제거 = *정상, 회귀 아님* spec line 1996)

---

## §2. D3 narrative 3밴드 sanity (4.6 종결 게이트 #2)

**구조적 보장 (C-3 동치 사슬)**: spec §7 Stage 4 line 1970 검증대로 `Relationship::modifiers()`는
`affinity.value()/100` 만을 참조. 4파일 변환 시 산술 hard 불변식 `affinity = closeness × 100`을
*정확히* 지켰으므로 `modifiers()` 출력 불변 → `compute_significance` 불변 → D3 3밴드 exact 보존.
`respect`/`wariness` 신규 2축은 `modifiers()` 비참조 — significance 불활성 (B-D14 정합).

**직접 측정 보류**: Phase 1 narrative validation 시나리오 중 `daily-training` / `lin-chong-shanshenmiao`는
Stage 2 후 재활성화 예정 `#[ignore]` 상태이므로 cargo test로 자동 검증 불가. `chitchat-passerby`만
실행되며 1 passed (`baselines/stage4-d3-sanity-2026-05-16.log`). 정식 D3 0.000/0.461/0.980 exact 재실행은
**Stage 5 narrative 검증**으로 위임 (spec line 2010 단서: "Stage 5 전 sanity").

산술 변환 표 (디자이너 검토용):

| 파일 | 페어 | v0.6 (closeness/trust/power) | v0.7 (trust/affinity/respect/wariness) |
|---|---|---|---|
| `confession/session_001` | `shu_lien→mu_baek` | 0.7 / 0.8 / -0.1 | 80 / 70 / 35 / 0 |
| `confession/session_001` | `mu_baek→shu_lien` | 0.7 / 0.8 / 0.1 | 80 / 70 / 35 / 0 |
| `chitchat-passerby` | `lin_chong→passerby` | 0.0 / 0.0 / 0.0 | 0 / 0 / 0 / 0 |
| `daily-training` | `yu_shulien→chunxueping` | 0.4 / 0.5 / 0.7 | 50 / 40 / 20 / 0 |
| `daily-training` | `chunxueping→yu_shulien` | 0.6 / 0.8 / -0.7 | 80 / 60 / 30 / 0 |
| `shanshenmiao` | `lin_chong→lu_qian` | 0.4 / 0.5 / 0.0 | 50 / 40 / 20 / 0 |
| `shanshenmiao` | `lu_qian→lin_chong` | -0.2 / -0.3 / -0.4 | -30 / -20 / -10 / 15 |

산술 검증:
- `affinity = closeness × 100`: ∀ 7페어 ✓ (정확, 반올림 없음)
- `trust_v07 = trust_v06 × 100`: ∀ 7페어 ✓
- `power` 폐기: ∀ 7페어 ✓ (B-D4 — 위계 정보는 `type` 자유 텍스트로 흡수)
- `respect = closeness × 50` (휴리스틱 B-D10 baseline, bond_kind 미지정): ∀ 7페어 ✓
- `wariness = max(0, -trust × 50)`: 6/7 페어 0, 1페어 (lu_qian→lin_chong) 15 ✓

`bond_kind` 전 페어 미지정 — 보수적 결정. ground truth 사람-검토 narrative (shu_lien↔mu_baek "의형제+절제된 사모" / lin_chong↔lu_qian "옛 친구→배신")는 `type` 자유 텍스트로 박음.

---

## §3. v0.6 코드 0건 grep 게이트 (4.6 종결 게이트 #3·#4)

baseline log: `baselines/stage4-grep-v06-zero-2026-05-16.log`.

| 검색어 | production 결과 |
|---|---|
| `RelationshipUpsertV0_6` (struct) | 0건 ✓ |
| `upsert_relationship_v2` (fn) | 0건 ✓ |
| `is_v06` / `auto-multiplying` / `RawRelationship` | 0건 ✓ |
| `"closeness"` / `"power"` (production string) | 0건 ✓ (남은 매치 2건 = 신규 unit test `legacy_v06_keys_are_silently_ignored`의 *입력 문자열* — production 동작 아님) |
| `* 100.0` (memory_repository.rs + v2_scenes.rs) | 0건 ✓ (메트릭 2→0 충족) |
| `impl<'de> Deserialize<'de> for RelationshipData` | 제거 ✓ |
| 4파일 `closeness` / `power` 키 | 0건 ✓ |
| 4파일 prose `_expected_*` v0.6 축명 | 0건 ✓ (`closeness/trust 약간 ↑` → `affinity/trust 약간 ↑` 등) |

---

## §4. 산출 변경 카탈로그

### C1 — 4.1 데이터 폐기

- `data/huckleberry_finn/` → `data/_discarded-v0.6/huckleberry_finn/` (git mv, 하드 삭제 아님 안전장치)
- `data/treasure_island/` → `data/_discarded-v0.6/treasure_island/` (git mv)
- `src/bin/mind-studio/mcp_server.rs:242` description 예시 경로 cosmetic 교체 (`treasure_island/ch01/...` → `wuxia_world/confession/...`)
- C-1 검증 결과: src/ tests/ benches/ Cargo.toml 어디에도 load 코드 0건 — 폐기 무해 확인

### C2 — 4.2 + 4.3 (atomic, ★ 절대 분리 금지)

생존 4파일 v0.7 영구 변환:
- `data/wuxia_world/confession/session_001/scenario.json` (1페어 양방향)
- `data/scenarios/phase1-validation/chitchat-passerby.json` (1페어 단방향)
- `data/scenarios/phase1-validation/daily-training.json` (1페어 양방향)
- `data/scenarios/phase1-validation/lin-chong-shanshenmiao.json` (1페어 양방향)
- 백업: `data/scenarios.backup-v0.6/`

`src/adapter/memory_repository.rs::RelationshipJson` 순수 v0.7 재작성:
- v0.6 자동 ×100 사슬 *제거* (4축 ±100 raw 직독)
- 필드 `closeness`/`trust`/`power` → `trust`/`affinity`/`respect`/`wariness`
- 신규 `relationship_json_tests` 4 단위 테스트 (≥3 spec 요구사항):
  - `v07_four_axes_round_trip_preserves_values`
  - `missing_optional_axes_default_to_zero`
  - `legacy_v06_keys_are_silently_ignored` (명시 동작: v0.6 키 무시)
  - `neutral_all_zeros_round_trip`

### C3 — 4.4 책임 B (state.rs 커스텀 Deserialize + 5 테스트 제거)

- `src/bin/mind-studio/state.rs:680~734` `impl<'de> Deserialize<'de> for RelationshipData` *제거*
- struct `RelationshipData` 선언을 `#[derive(Clone, Serialize, Deserialize)]`로 환원
- 4축 필드에 `#[serde(default)]` 부가 — missing field가 422 reject되지 않도록 보호 (custom impl이 했던 0 default 동작 보존)
- `src/bin/mind-studio/state.rs:925~1009` `mod relationship_data_tests` (5 테스트) *제거*:
  - `v06_schema_auto_multiplies_by_100` / `v07_schema_passes_through_without_scaling` / `v06_power_alone_triggers_migration` / `save_roundtrip_v06_to_v07_preserves_semantic` / `v07_missing_optional_fields_defaults_to_zero`

### C4 — 4.5 v2_scenes legacy endpoint 제거

- `src/bin/mind-studio/handlers/v2_scenes.rs` `RelationshipUpsertV0_6` struct + `upsert_relationship_v2` async fn *제거*
- `src/bin/mind-studio/main.rs:317-319` axum 라우터 `POST /api/v2/relationships` 등록 *제거*
- 모듈 doc + state.rs `director_v2` 필드 doc 갱신 (`POST /api/v2/relationships` 언급 제거)
- `src/bin/mind-studio/handler_tests.rs` 5 호출처 정리:
  - `rel_json_neutral()` v0.6-shape helper *제거* → 새 `seed_neutral_relationship_v2(state, owner, target)` helper로 교체 (director_v2 dispatcher의 `repository_guard()`로 Director 내부 repo에 직접 `save_relationship(...)` 호출)
  - 4 setup 호출처를 helper 호출로 교체 (Scene 시작 전 관계 등록 setup 보존)
- frontend (`mind-studio-ui/`) 호출처 0건 — `npm run build` 영향 없음 (spec 게이트 #4.5)

### C4 부수 — handler_tests.rs `relationship_data()` helper v0.7 갱신

- 기존: v0.6 shape `{closeness: -0.3, trust: -0.5, power: 0.4}` → POST `/api/relationships`로 보내고 custom Deserialize의 ×100 사슬에 의존
- 갱신: v0.7 raw 표기 `{trust: -50, affinity: -30, respect: -15, wariness: 25}` (산술 변환 결과)
- 사유: C3로 custom Deserialize 제거 후 derive(Deserialize) + `#[serde(default)]`만으로는 closeness/power 키를 silent ignore하지만 trust 값을 `×100` 안 함 → 의도 차이. helper를 v0.7로 정합화 + RelationshipData에 `#[serde(default)]` 추가의 *조합*으로 회귀 16건 해소.

### C5 — 4.6 `_schema.md` v0.7 갱신

- `docs/game-design/2-characters/_schema.md:101` Relationship 섹션 본체 참조 `relationships.md v0.6` → `v0.7` + 마이그레이션 노트 4축 표기 명시
- 변경 이력 v0.7 (2026-05-16) 항목 추가

---

## §5. 사후 (Stage 5 narrative 인계 사항)

1. **D3 sanity 직접 측정**: Phase 1 narrative validation 시나리오의 `daily-training` / `lin-chong-shanshenmiao` `#[ignore]` 해제 시점에 `compute_significance` 0.000 / 0.461 / 0.980 exact 재확인 권장 (`tests/phase1_*` 세 케이스 묶음). C-3 구조적 보장에 따라 회귀 가능성 매우 낮음.
2. **휴리스틱 신규 2축 narrative 검토**: `respect = closeness × 50` / `wariness = max(0, -trust × 50)` baseline이 Stage 5 narrative에서 어색하면 사람-검토로 조정 (spec B-D10).
3. **bond_kind 명시 진입 (Stage 5+)**: 본 Stage 4는 보수적으로 *전 페어 미지정*. Stage 5 narrative 검증 후 `BondKind` 명시 (예: `MasterDisciple`·`Betrayer`)가 의미 추가 필요하다고 판단되면 인스턴스 작성자 + Claude 추론 + 디자이너 검토 페어로 박음. 도메인 `modifiers()`는 bond_kind 비참조이므로 D3 영향 없음.
4. **session_*_result.json 재생성**: Phase 1 narrative 결과 파일들은 4축 시스템에서 재실행되어 갱신 필요 (spec B-D9, Stage 5).
5. **`_discarded-v0.6/` 영구 폐기**: Stage 5 narrative + Stage 6 handoff 완료 후 디자이너 결정에 따라 `data/_discarded-v0.6/` 완전 삭제 또는 보존 결정. 본 회고 시점 안전장치로 보존.

---

## §6. baseline 박제

| 파일 | 내용 |
|---|---|
| `baselines/stage4-prep-cargo-test-2026-05-16-PASS.log` | 진입 직전 lib+tests baseline (871 passed) |
| `baselines/stage4-prep-mindstudio-2026-05-16-PASS.log` | 진입 직전 Mind Studio bin baseline (95 passed) |
| `baselines/stage4-cargo-test-2026-05-16-PASS.log` | 종결 lib+tests (875 passed, 회귀 0) |
| `baselines/stage4-mindstudio-2026-05-16-PASS.log` | 종결 Mind Studio bin (90 passed, 회귀 0) |
| `baselines/stage4-grep-v06-zero-2026-05-16.log` | v0.6 0건 grep 결과 5종 |
| `baselines/stage4-d3-sanity-2026-05-16.log` | D3 sanity (chitchat 1 PASS, 나머지 2 ignored Stage 2 후) |

---

## §7. 변경 이력

| 버전 | 일자 | 변경 |
|---|---|---|
| 1.0 | 2026-05-16 | 초안 작성 (C1~C5 commit 종결 후). |
