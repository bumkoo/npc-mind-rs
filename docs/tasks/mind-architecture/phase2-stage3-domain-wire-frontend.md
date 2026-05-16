# Phase 2 Stage 3 회고 — Domain + Wire + Frontend 4축 확장

**Stage**: Phase 2 Stage 3 (Spec v1.3 frozen → 2026-05-16 코딩 완료)
**범위**: `RelationshipUpdatedPayload` 3축 6 필드 → 4축 8 필드 / `RelationshipPolicy` helper 추출 / `RelationshipValues` + `RelationshipData` DTO 4축 ÷100 제거 / `dominant_delta` 4축 라벨 / projection 4 튜플 / memory_projector 4축 합산 + threshold 0.05→5.0 / Frontend 4축 + Slider props 명시.
**Spec**: [`task-rel-phase2-domain-migration.md`](task-rel-phase2-domain-migration.md) §7 Stage 3 (3.1~3.7).

---

## §1. 컴파일 + 테스트 게이트 (3.7.1)

| 항목 | Stage 2 baseline | Stage 3 결과 |
|---|---|---|
| `cargo check --features chat` | ✅ | ✅ (`baselines/stage3-cargo-check-2026-05-16-PASS.log`) |
| `cargo test --features chat --lib --tests` | 866 passed | **871 passed** / 0 failed / 5 ignored (`baselines/stage3-cargo-test-2026-05-16-chat-PASS.log`) |
| Mind Studio bin tests (`--bin npc-mind-studio`) | 72 passed | **72 passed** / 0 failed |
| `npm run build` (frontend) | ✅ | ✅ (`baselines/stage3-npm-build-2026-05-16-PASS.log`) |
| `npm test -- --run` (vitest) | 100 passed | **100 passed** (`baselines/stage3-npm-test-2026-05-16-PASS.log`) |

회귀 0건. 신규 테스트 5개 추가 (이전 baseline 866 → 871):
- `projection_handlers::relationship_updated_stores_after_values` (4 튜플 회귀)
- `projection_handlers::repeated_relationship_updates_overwrite` (4 튜플 overwrite)
- 기존 Stage 2.7 5 테스트(±1.0 → ±100 expected 값 변경) — 카운트 영향 0

---

## §2. 회귀 가드 5종 (3.7.2)

| 가드 | 위치 | Stage 3 결과 |
|---|---|---|
| W1 `..._affinity_channel_after_anger` | `mapping.rs::tests` line 801 | ✅ expected 0.286 보존 (※ Phase 2.3 트리거 보존) |
| W1 `..._trust_channel_after_anger` | `mapping.rs::tests` line 828 | ✅ expected 0.158 보존 |
| W1 `..._admiration_no_leak_until_phase_2_3` | `mapping.rs::tests` line 852 | ✅ 4 modifier 불변 |
| W4 `update_axes_from_emotion_does_not_filter_pride_or_shame_internally` | `mapping.rs::tests` line 880 | ✅ B-D12 호출 측 책임 보존 |
| W2 `is_negative_emotion_classification_matches_affinity_sign_basis` | `mapping.rs::tests` line 911 | ✅ |

`cargo test --features chat --lib domain::relationship::mapping` → 37 passed / 0 failed.

W3 BondStatus 차단 로그 (`tracing::debug!`)는 manual confirm — `mapping.rs:251~259`에 그대로 유지. §3.2 helper 추출 후에도 `update_axes_from_emotion` 진입점은 그대로라 로그 발화 위치 변경 0.

---

## §3. D2 latency ±20% (3.7.3)

| 측정 | Stage 2 baseline (회고 phase1) | Stage 3 결과 | 변동 |
|---|---|---|---|
| chitchat (3 follow-up) | ~24µs | **7.025µs** | ✅ -71% (target ≤29) |
| significant (4 follow-up) | ~35µs | **10.366µs** | ✅ -70% (target ≤42) |
| legacy (3 follow-up) | ~29µs | **7.75µs** | ✅ -73% (target ≤35.2) |

이번 측정은 release 빌드 + N=50 평균. Stage 2 baseline은 phase1 측정값으로 빌드 차이 가능 (Stage 6 재측정 시 정합 확보). **Stage 3 변경이 latency 회귀 유발 없음** — payload 6→8 필드 증가 영향 작음 (boxing된 페이로드라 stack copy 영향 0, 필드 추가로 인한 heap 비용 미미).

`baselines/stage3-d2-latency-2026-05-16.log` 참조.

---

## §4. D3 3밴드 calibration (3.7.4)

| 시나리오 | Stage 1 baseline | Stage 3 결과 | 비교 |
|---|---|---|---|
| chitchat | 0.000 | **0.000** | ✅ exact match |
| daily | 0.461 | **0.461** | ✅ exact match |
| shanshenmiao | 0.980 | **0.980** | ✅ exact match |

`compute_significance` 엔진은 Stage 3 변경 면적 외 (relationships.md v0.7 §6). B-D-A2 (ii) 결정 — domain 내부 modifiers / RelationshipLevel ±1.0 잔존 — 이 modifier 보존을 보장해 significance 안정성 유지.

`baselines/stage3-d3-narrative-2026-05-16.log` 참조.

---

## §5. 메트릭 회귀 카탈로그 (3.7.5)

| 메트릭 | Stage 2 종결 | Stage 3 target | Stage 3 실제 | 상태 |
|---|---|---|---|---|
| ÷100 production 위치 (logical) | 5 | **2** | 2 (domain modifiers + RelationshipLevel) + 1 uncatalogued (telling_ingestion_handler:80) | ⚠️ telling_ingestion 잔존 (Phase 2.3 위임) |
| ×100 production 위치 | 3 | **3 그대로** | 3 (memory_repository:205 + state.rs:801 → 제거됨 + v2_scenes:271) | ⚠️ |
| W4 마커 (production) | 3 | **2** | **2** (relationship_policy:125 + stimulus_policy:83) | ✅ |
| W4 doc § 호출자 인덱스 | 3 항목 | **2 항목** | **2 항목** (helper + stimulus_policy) | ✅ |
| `closeness`/`power` wire payload 잔존 | 11+ | **0** | **0** (payload + DTO + frontend 모두 폐기) | ✅ |
| `before_closeness`/etc. payload 접근 | (다수) | **0** | 0 production (테스트는 변수명 일부 유지 — semantic 영향 0) | ✅ |

### ⚠️ telling_ingestion_handler:80 ÷100 잔존

```rust
.map(|r| (r.trust().value() / 100.0 + 1.0) / 2.0)
```

Spec inventory에 포함되지 않은 ÷100 사이트. 신뢰도 정규화 `(t + 1) / 2 ∈ [0,1]` 공식이 ±1.0 가정. ±100 contract 정합화 시 `(t + 100) / 200` 형태로 재작성 필요. Phase 2.3 KICKOFF에 표기.

### ⚠️ state.rs:801 to_relationship ×100 제거 (Stage 4 영향)

원 spec §6 baseline 표는 "×100 3 그대로 (Stage 4 마이그레이션 책임)"이라고 명시했으나, `RelationshipData` 필드 자체를 4축 ±100 raw로 갱신하면서 `to_relationship`의 ×100이 자동으로 제거됨 (필드가 이미 ±100인데 ×100 곱하면 ±10000 = 잘못됨).

실제 ×100 production 잔존:
- `adapter/memory_repository.rs:205-206` (RelationshipJson, v0.6 시나리오 JSON → 도메인) — 유지
- `bin/mind-studio/handlers/v2_scenes.rs:271-272` (RelationshipUpsertV0_6, v0.6 v2 endpoint) — 유지

`state.rs:801`은 ÷100/×100 양쪽 모두 제거 — `RelationshipData` 필드 contract 변경의 자연스러운 결과.

### `RelationshipData` v0.6 호환 (serde alias)

v0.6 시나리오 JSON 로드 시점에 `RelationshipData` deserialize 호환 필요. 적용한 패턴:
- `#[serde(alias = "closeness")]` on `affinity`
- `#[serde(default)]` on `respect` / `wariness`
- `power` 필드는 단순 무시 (serde 기본: unknown field 허용)

**알려진 transient bug**: v0.6 JSON (closeness=0.5 등 ±1.0 값)이 ±100 RelationshipData 필드로 로드되면 의미가 어긋남 (0.5가 affinity=0.5로 박혀 거의 중립 상태로 표시). Stage 4 마이그레이션 도구가 시나리오 JSON 자체를 ±100으로 변환하면 해소.

Mind Studio 자체 통합 테스트는 `relationships: {}` (빈 객체) 또는 `rel_json_neutral` (값 0.0) 사용으로 영향 없음.

---

## §6. 산출물 (3.7.6)

### baseline log 10개 박제 (spec target 9 + W4 doc bonus)

`docs/tasks/mind-architecture/baselines/stage3-*-2026-05-16.log`:

1. `stage3-cargo-check-2026-05-16-PASS.log` — chat feature compile
2. `stage3-cargo-test-2026-05-16-chat-PASS.log` — 871 passed / 0 failed / 5 ignored
3. `stage3-npm-build-2026-05-16-PASS.log` — frontend build clean
4. `stage3-npm-test-2026-05-16-PASS.log` — 100 vitest passed
5. `stage3-d2-latency-2026-05-16.log` — dispatch_v2(EndDialogue) latency
6. `stage3-d3-narrative-2026-05-16.log` — 3밴드 calibration 0.000 / 0.461 / 0.980
7. `stage3-grep-divide-100-2026-05-16.log` — ÷100 잔존 site 카탈로그
8. `stage3-grep-w4-marker-2026-05-16.log` — B-D12 guard 마커 카탈로그
9. `stage3-grep-closeness-power-2026-05-16.log` — wire payload 잔존 0건 검증
10. `stage3-w4-doc-2026-05-16.log` — `update_axes_from_emotion` doc § 호출자 인덱스 2 항목 박제 (보너스)

### 변경 파일 카탈로그 (sub-stage별)

| Sub-stage | 변경 면적 | 주요 파일 |
|---|---|---|
| 3.1 | Payload anchor | `src/domain/event.rs` (struct + 2 test) |
| 3.2 | helper + emit 2 위치 통합 | `src/application/command/policies/relationship_policy.rs` (helper 추출, B-D12 가드 helper 안 1 곳 / emit 2 위치 ÷100 제거 / Stage 2.7 5 테스트 expected 값 갱신), `src/domain/relationship/mapping.rs` (doc § 호출자 인덱스 3→2 항목) |
| 3.3 | DTO + 변환 4 위치 | `src/application/dto/relationship.rs` (RelationshipValues 4축), `src/application/dialogue_orchestrator.rs` (변환 ①), `src/bin/mind-studio/domain_sync.rs` (변환 ②③④), `src/bin/mind-studio/state.rs` (RelationshipData 4축 + serde alias + to_relationship ×100 제거), `src/bin/mind-studio/handlers/query.rs` (RelationshipView 4 fields), handler_tests 3 RelationshipData literal |
| 3.4 | dominant_delta 8 인자 | `src/application/command/relationship_memory_handler.rs` (함수 시그니처 + 호출 + 라벨 4종 + threshold 주석 + 테스트 6×0.3→30.0 + comment 5.0) |
| 3.5 | projection + threshold | `src/application/projection.rs` (4 튜플), `src/application/memory_projector.rs` (4축 합산 + threshold const 5.0), `src/domain/tuning.rs` (MEMORY_RELATIONSHIP_DELTA_THRESHOLD 5.0 + validate range (0,100)), 2 projection_handler 테스트 갱신, integration test 갱신 |
| 3.6 | Frontend 4축 | `mind-studio-ui/src/types/index.ts` (Relationship + AfterDialogueResponse 4축), `RelModal.tsx` (emptyRel + 4 Slider props 명시 — wariness min=0 비대칭), `EmotionView.tsx` (RelMetrics 4축 + toPercent 분기), `ReflectionView.tsx` (4축 AxisRow + toFixed(0) + 임계값 0.1), `Sidebar.tsx` (신:호:존:경: 라벨), `__tests__/stores.test.ts` + `handlers.test.ts` 4축 갱신 |

총 변경 라인 ≈ 220 (spec 예상 200~250 안).

---

## §7. Phase 2.3 인계 위험 (3.7.7~3.7.8)

[PHASE2.3-KICKOFF.md](PHASE2.3-KICKOFF.md)에 상세. 주요 항목:

### A) 잔존 ÷100 (수정 필요)

1. `telling_ingestion_handler:80` — confidence 정규화 공식 `(t + 1) / 2` → `(t + 100) / 200` 또는 동등한 ±100 native 공식으로 갱신
2. `domain/relationship/mod.rs:172-173` — `modifiers()` ±1.0 → ±100 raw native 전환 (modifier 가중치 재조정 수반)
3. `domain/guide/snapshot.rs:316-317` — `RelationshipLevel::from_score()` 시그니처 ±100 native 전환 (Level enum 의미 재검토)

### B) W1 회귀 가드 (Phase 2.3 진입 시 깨지는 게 정상)

`mapping.rs::tests`:
- `beat_rel_modifiers_affinity_channel_after_anger` (expected 0.286 = `affinity 28.6 / 100`)
- `beat_rel_modifiers_trust_channel_after_anger` (expected 0.158)
- `beat_rel_modifiers_admiration_no_leak_until_phase_2_3` (4 modifier 불변)

Phase 2.3 시작 시 modifier ±100 native 전환하면 expected 값 ±100 스케일로 재조정 필요. 깨지는 시점이 Phase 2.3 트리거 신호.

### C) R-3b memory content 라벨 혼재

`dominant_delta` 라벨이 Stage 2(`closeness`/`trust`/`power`) → Stage 3(`trust`/`affinity`/`respect`/`wariness`)로 변경됨. 기존 memory entries는 `[closeness Δ=0.34]` 같은 v0.6 라벨 그대로 잔존. 재마이그레이션 안 함 (content text는 dialogue/scene snapshot 일부이고 retroactive rewrite 시 trace 무결성 깨짐). Phase 2.3 narrative 시뮬에서 검색 시 양쪽 라벨 fallback 처리 고려.

### D) R-3g memory_relationship_delta_threshold 정밀화

Stage 3 → threshold 0.05 → 5.0 (×100 일대일 매핑). 4축 합산 sensitivity는 Phase 2.3에서 narrative 시뮬 결과 보고 재조정 (예: ÷4 평균? 가장 큰 축 1개? 가중치 합?).

### E) v0.6 시나리오 JSON 로드 transient bug

Mind Studio `load_scenario` 경로에서 v0.6 JSON 로드 시 `RelationshipData.affinity` 등 필드에 ±1.0 값이 박혀 UI 표시가 어긋남 (예: 0.5 vs 50). Stage 4 마이그레이션 도구로 시나리오 JSON 자체를 ±100으로 변환해야 정합 회복.

워크어라운드 (Stage 3~4 사이): Mind Studio 시나리오는 빈 relationships(`{}`)로 시작하거나 UI CRUD로 직접 생성. 기존 v0.6 시나리오 JSON 직접 로드 비권장.

---

## §8. spec 가정 정정 4건 (재확인)

| 가정 | 실제 | 영향 |
|---|---|---|
| `event_bridge` SSE 매핑 갱신 필요 | 변경 0 (axes 안 봄) | ✅ Stage 3 변경 면적에서 제거됨 |
| `dialogue_test_service.rs` DTO 변환 | DTO 재사용, 변환 코드 없음 | ✅ 변경 0 확인 |
| Slider 컴포넌트 시그니처 확장 | min/max/step props 이미 존재 | ✅ RelModal에서 props 명시만 |
| 변환 위치 3 → 4 (A5 새 발견) | domain_sync.rs:63-74 추가 | ✅ 4 사이트 모두 ÷100 제거 완료 |

---

## §9. 종합 게이트 (Stage 3 종합)

| 게이트 | 결과 |
|---|---|
| 1. cargo check --features chat | ✅ |
| 2. cargo test --features chat 871 + Mind Studio 72 통과 | ✅ |
| 3. npm run build (frontend) 0 error | ✅ |
| 4. Stage 2 회귀 가드 5개 통과 | ✅ |
| 5. D2 latency 모든 케이스 spec ±20% 안 | ✅ |
| 6. D3 3밴드 calibration exact match | ✅ |
| 7. ÷100 5→2 / W4 3→2 / closeness·power payload 0 | ✅ (Phase 2.3 위임 telling_ingestion 명시) |
| 8. baseline log 10개 + 회고 + PHASE2.3 KICKOFF 박제 | ✅ |

**Stage 3 종결**. Phase 2.3 진입 준비 완료.

---

## §10. 변경 이력

| 버전 | 날짜 | 변경 |
|---|---|---|
| 1.0 | 2026-05-16 | Stage 3 spec v1.3 frozen → 코딩 완료 → 회고 박제. |
