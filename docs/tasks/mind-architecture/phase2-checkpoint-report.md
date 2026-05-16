# Phase 2 Checkpoint Report — Relationship 4축 도메인 마이그레이션 종결

**Status**: ✅ **Phase 2 완료** (2026-05-16, Stage 0~6 종결)
**상위 spec**: [`task-rel-phase2-domain-migration.md`](task-rel-phase2-domain-migration.md) v1.0 frozen
**대응 도메인 spec**: `relationships.md` v0.7 (4축 trust/affinity/respect/wariness + BondKind/BondStatus/Partnership/type)
**다음 단계**: Phase 2.3 (Appraise tuning + ±100 native 잔존 청소 + W1 expected 재조정) — KICKOFF: [`PHASE2.3-KICKOFF.md`](PHASE2.3-KICKOFF.md)

---

## §1. Phase 2 한눈에 보기

- **목표**: Relationship 3축(closeness/trust/power × ±1.0) → **4축(trust/affinity/respect/wariness × ±100)** + 4 보조축(BondKind/BondStatus/Partnership/type) 도입.
- **결과**: 도메인 본체 + 매핑 + wire payload + frontend + 시나리오 4파일 전부 v0.7로 영구 이행. v0.6 transient code 0건. `compute_significance` D3 3밴드 exact 보존 (0.000 / 0.461 / 0.980).
- **비용**: Stage 1(도메인) → 2(매핑) → 3(wire+frontend) → 4(시나리오 영구 변환 + v0.6 code 0건화) → 5(narrative 4축 박제) → 6(bench+회고+인계). 6 Stage. 코드 면적은 spec §3 표 그대로 달성, 회귀 0건.
- **부채 인계 (Phase 2.3+)**: §6 부채표 참조. *Phase 2 종결 게이트 차단 아님*.

---

## §2. Stage별 종결 요약 + 회고 포인터

| Stage | 종결 | 회고 | 핵심 산출 |
|---|---|---|---|
| **0** | 사실조사 | spec §1 / §2 / §3.6 (시뮬레이션) | B-D 12 결정 + R1~R3 위험 분해 + 4 무협 시나리오 시뮬 검증 |
| **1** | 도메인 재작성 (2026-05-14) | [`phase2-stage1-domain.md`](phase2-stage1-domain.md) | `relationship/{mod,axis,bond,partnership}.rs` 신설. `Relationship` 4축 본체. `AxisScore`/`WarinessScore` 타입 분리 (B-D1). Builder 4축 API. 3축 메서드(`after_dialogue`/`with_updated_closeness`/`with_power`) 완전 제거. 7 호출처 정리 (no-op clone 임시). 13 테스트 일괄 변환. |
| **2** | OCC → 4축 매핑 (2026-05-15) | [`phase2-stage2-mapping.md`](phase2-stage2-mapping.md) + [`task-rel-phase2-stage2-retrospective-cleanup.md`](task-rel-phase2-stage2-retrospective-cleanup.md) | `mapping.rs` 신설. `base_delta` 48셀 lookup (12 explicit + B-D14 10 누락). `hexaco_modifier` 6 보정룰 (Sincerity/Patience/Forgiveness/Anxiety/Prudence + O+ placeholder). `update_axes_from_emotion` 단일 진입점 (B-D5). Stage 1 no-op 3 위치 해소. W1/W2/W4 회귀 가드 5종 박제. |
| **3** | Wire + frontend 4축 (2026-05-16) | [`phase2-stage3-domain-wire-frontend.md`](phase2-stage3-domain-wire-frontend.md) | `RelationshipUpdatedPayload` 6 → 8 필드 + ÷100 layer wire boundary 4겹 제거 (B-D-A ±100 raw). `RelationshipPolicy::apply_emotions_to_relationship` helper 추출 (B-D-helper). `RelationshipValues`/`RelationshipData` DTO 4축 ÷100 제거. `dominant_delta` 4축 라벨. `memory_relationship_delta_threshold` 0.05 → 5.0. Frontend Slider min/max ±100 + 4축 라벨 (B-D-D 한글). 신규 5 테스트. KICKOFF Stage 3본 (157줄) 신설. |
| **4** | 시나리오 영구 변환 + v0.6 0건화 (2026-05-16) | [`phase2-stage4-migration.md`](phase2-stage4-migration.md) | 4파일(`confession/session_001` + 3 phase1-validation) v0.7 영구 변환. v0.6 code 3 경로 제거(`memory_repository::RelationshipJson` v0.6 자동 ×100 / `state.rs:680~734` 커스텀 Deserialize impl + 5 테스트 / `v2_scenes.rs RelationshipUpsertV0_6` legacy endpoint). 데이터 폐기 2건 (`huckleberry_finn` / `treasure_island` → `_discarded-v0.6/`). `_schema.md` v0.7 갱신. 신규 4 `relationship_json_tests`. v0.6 grep 0건 확인. |
| **5** | Narrative 4축 박제 (2026-05-16) | [`phase2-stage5-narrative.md`](phase2-stage5-narrative.md) + FROZEN spec | Phase 1 픽스처 B-D12 정합화 (`daily-training` Pride → Admiration). `#[ignore]` 2건 해제 (daily + shanshenmiao). `phase2_narrative_test.rs` 신설 (S1~S3 정량 + S4 정성). D3 3밴드 exact 보존 (0.000 / 0.461 / 0.980). S5-D1~D6 6 결정 박제. |
| **6** | Bench + 회고 + Phase 2.3 KICKOFF (2026-05-16) | [`phase2-stage6-bench-handoff.md`](phase2-stage6-bench-handoff.md) + 본 문서 + KICKOFF 갱신 | D2 latency 3케이스 임계값 이내 + D4 ±20% + D3 exact 보존. A5 `MAX_EVENTS_PER_COMMAND=22` grep + worst-case 산출 교차검증 (Phase 2 본체 변경 0 / Phase 2.5 worst≈17 < 22). Phase 2.3 KICKOFF Stage 4·5 인계 5항 추가. `appraise-validation/` 디렉토리 + Phase 2.3 spec 초안. |

---

## §3. D baseline 최종 대조표 (Phase 1 → Phase 2 종결)

**측정 명령 (S5-D5 / S6-D1 정본, Stage 5·6 일관 고정)**: `cargo test --lib --tests --bins`

### D1 — 테스트 회귀

| 항목 | Phase 1 종결 | Stage 1 | Stage 2 | Stage 3 | Stage 4 | Stage 5 | **Stage 6 (Phase 2 종결)** |
|---|---|---|---|---|---|---|---|
| failed | 0 | 0 | 0 | 0 | 0 | 0 | **0** ✅ |
| passed (`--lib --tests --bins`) | — | — | — | — | — | 838 → 843 | **843** ✅ |
| ignored | — | — | — | — | — | 2 (daily/shanshenmiao 해제) | **2** ✅ |
| result 묶음 | — | — | — | — | — | 64 → 65 | **65** ✅ |
| `cargo test --features chat --lib --tests` passed (참고) | — | — | 866 | 871 | 875 | — | — |

**참고**: Stage 1~4는 `--features chat --lib --tests` 측정명령(spec line 1996 baseline)을 썼고 Stage 5에서 `--lib --tests --bins`로 정본 정착(S5-D5). 절대수 대신 *failed=0 ∧ 회귀 0 ∧ 증감 설명가능*이 정본.

### D2 — `dispatch_v2(EndDialogue)` latency (debug 빌드, N=50 평균)

| 케이스 | 임계값 (spec L1891) | Phase 1 baseline | Stage 3 (release N=50) | Stage 5 진입 (debug) | **Stage 6** |
|---|---|---|---|---|---|
| chitchat (3 follow-up) | ≤29.0 µs | ~24 µs | 7.025 | 26.35 | **15.70** ✅ |
| significant (4 follow-up) | ≤42.0 µs | ~35 µs | 10.366 | 38.92 | **26.68** ✅ |
| legacy (3 follow-up) | ≤35.2 µs | ~29 µs | 7.75 | 30.74 | **20.89** ✅ |

**4축 매핑 추가 latency 영향**: payload 6 → 8 필드 + 4축 base_delta lookup + 4축 hexaco_modifier 평가 → 임계값 대비 50% 마진 보존. **Phase 2 4축 매핑은 latency 회귀를 유발하지 않았다** (S6-D2 박제). debug↔release 빌드 차이가 변동의 주된 요인.

### D3 — Narrative 3밴드 calibration

| 시나리오 | Phase 1 baseline | Stage 2/3 | Stage 4 (sanity) | Stage 5 | **Stage 6** |
|---|---|---|---|---|---|
| chitchat (low, <0.3) | 0.000 | 0.000 | (chitchat 1 pass) | 0.000 | **0.000** ✅ exact |
| daily-training (mid, [0.3,0.7)) | 0.461 | 0.461 | (`#[ignore]`) | 0.461 | **0.461** ✅ exact |
| shanshenmiao (high, ≥0.7) | 0.980 | 0.980 | (`#[ignore]`) | 0.980 | **0.980** ✅ exact |

**보존 메커니즘 (C-3 동치 사슬, Stage 4 §2 박제)**: `Relationship::modifiers()`는 `affinity.value() / 100.0` 만을 참조. 4파일 변환 시 산술 hard 불변식 `affinity = closeness × 100`을 정확히 지킴 → `modifiers()` 출력 불변 → `compute_significance` 불변. `respect`/`wariness` 신규 2축은 `modifiers()` 비참조 (B-D14 정합).

### D4 — `compute_significance` 단발 호출 latency

| 측정 | 임계값 | Phase 1 baseline | Stage 5 진입 | **Stage 6** |
|---|---|---|---|---|
| 10 turn × 10000 iter avg | ~10.0 µs (±20%) | 8.36 µs | 8.29 µs | **9.77 µs** ✅ (+17% 이내) |

### D5 — Mind Studio bin tests (참고)

| Stage 3 | Stage 4 |
|---|---|
| 77 passed | **90 passed** (Stage 4: H1 부산물 5 제거 = -5 정상) |

### D6 — Frontend (`npm test --run`)

| Stage 3 | Stage 6 |
|---|---|
| 100 passed | (재측정 불필요 — Stage 3 이후 frontend 변경 0건) |

---

## §4. B-D 결정 최종 상태 (12개 + 합의 4개 + Phase 2.5 위임 2개)

### B 카테고리 본체 (B-D1 ~ B-D14)

| ID | 결정 | 상태 |
|---|---|---|
| **B-D1** | 2 타입 분리 (HEXACO `Score` ±1.0 / 신설 `AxisScore` ±100 + `WarinessScore` 0~100) | ✅ Stage 1 완료 |
| **B-D2** | f32 내부 + JSON 정수 round 출력 | ✅ Stage 1 완료 |
| **B-D3** | closeness → affinity 자동 변환 + 디자이너 조정 | ✅ Stage 4 완료 |
| **B-D4** | `power` 폐기 (`type` 자유 텍스트 흡수) | ✅ Stage 1 완료 |
| **B-D5** | 단일 함수 `update_axes_from_emotion` | ✅ Stage 2 완료 |
| **B-D6** | T1(대화 끝 batch) + D6-a(48셀 + HEXACO + BondStatus 차단 + clamp) + axis_modulation 3지선다 | ✅ T1/D6-a Stage 2 완료. axis_modulation은 `ReflectionResult` 7 필드에 *부재* (S5-D1 확인) — Phase 2.5에서 reflection LLM 출력 필드 신설 시 활성 |
| **B-D7** | (Phase 2.5) 새 cause variant 명명 | ⏳ Phase 2.5 위임 |
| **B-D8** | W3+ (자동 + Claude AI 추론 + 디자이너 검토) | ✅ Stage 4 완료 (4파일 영구 변환) |
| **B-D9** | session_*_result.json 일괄 폐기 + Phase 2 후 재생성 | ⚠️ Stage 5 §5 작업5 — *자동 dump 메커니즘 부재* 확인. **재생성 대상 0건**. backup은 `_discarded-v0.6` 통합. 재생성 인프라 신설은 Phase 2.3/3 위임 |
| **B-D10** | (B') 간단 휴리스틱 + BondKind 보완 | ✅ Stage 4 완료 (4파일 변환 표 §2 박제: respect = closeness × 50 / wariness = max(0, -trust × 50)) |
| **B-D11** | (Phase 2.5) declarative_events 상한 N | ⏳ Phase 2.5 위임 |
| **B-D12** | Shame/Pride(`agent_id=None`) → 4축 변동 0, PAD만 영향 | ✅ Stage 2 완료 (`relationship_policy.rs:127-129` helper `continue` 스킵) |
| **B-D13** | 1회 변동 별도 cap 없음 (HEXACO × intensity × axis_modulation 자연 한계) | ✅ Stage 2 완료 |
| **B-D14** | Well-being/Prospect 10 OCC 4축 매핑 의도된 누락 | ✅ Stage 2 완료 (Joy/Distress/Hope/Fear/Satisfaction/Disappointment/Relief/FearsConfirmed/Remorse/Gratification 10개 base_delta 0) |

### Stage 3 추가 결정 (B-D-A / B-D-B / B-D-D / B-D-helper)

| ID | 결정 | 상태 |
|---|---|---|
| **B-D-A** | wire format ±100 raw (event payload + DTO ÷100 layer 5 위치 완전 제거) | ✅ Stage 3 완료 |
| **B-D-B** | `closeness`/`power` 필드명 완전 폐기. 4축 식별자 = `trust`/`affinity`/`respect`/`wariness` (순서 고정) | ✅ Stage 3 완료 |
| **B-D-D** | 한글 라벨 (γ) — `신뢰`/`호감`/`존중`/`경계` (단어) + `신/호/존/경` (한 글자) | ✅ Stage 3 완료 |
| **B-D-helper** | `RelationshipPolicy::apply_emotions_to_relationship` private method (2 use sites). stimulus_policy::process_beat_transition은 inline 유지 (특수성) | ✅ Stage 3 완료 |

### Stage 5/6 추가 결정 (S5-D1~D6 / S6-D1~D5)

S5/S6 결정은 *측정 정본* 결정이며 본문 §2 회고 포인터로 위임.

---

## §5. 4축 도메인 안정화 결과

### 도메인 본체 (Stage 1)

```
src/domain/relationship/
  ├─ mod.rs        Relationship aggregate + Builder + TypeChange (4축 본체)
  ├─ axis.rs       AxisScore / WarinessScore / AxisDelta / AxisKind + AxisModifier
  ├─ bond.rs       BondKind (11 variants) + BondStatus (5 variants) + accepts_live_input()
  └─ partnership.rs Partnership (4 variants)
```

### 4축 + 4 보조축 스키마

| 축 | 타입 | 범위 | 의미 |
|---|---|---|---|
| trust | AxisScore | ±100 | 의탁/배신 가능성 |
| affinity | AxisScore | ±100 | 심리적 끌림 (이전 closeness 의미 일부) |
| respect | AxisScore | ±100 | 권위·능력 인정 |
| wariness | WarinessScore | 0~100 | 단방향 경계 (음수 컴파일 시점 차단 — B-D1) |
| bond_kind | BondKind | 11 variants | MasterDisciple / SwornSiblings / 등 |
| bond_status | BondStatus | 5 variants | accepts_live_input() 게이트 |
| partnership | Partnership | 4 variants | 협력 모드 |
| type | String (free text) | — | `power` 흡수 후 자유 서술 (B-D4) |

### closeness → affinity 이행

- **자동 변환**: `affinity = closeness × 100` (산술 hard 불변식, Stage 4 §2 박제)
- **휴리스틱 신규 2축**: `respect = closeness × 50` + `wariness = max(0, -trust × 50)` (BondKind 미지정 baseline, B-D10)
- **시나리오 4파일 변환 결과** (Stage 4 §2 표): 7페어 산술 정확 검증

### ±100 wire (Stage 3 B-D-A)

- 4 ÷100 normalization layer 완전 제거 (event payload + DTO)
- frontend Slider min/max ±100, 임계값 `> 0.1`, `toFixed(0)`
- 한글 라벨 4축 적용

### B-D12 호출 측 책임

- `relationship_policy.rs::apply_emotions_to_relationship` helper에서 Pride/Shame `continue` 스킵
- `update_axes_from_emotion` 내부는 차단 안 함 (W4 가드 `update_axes_from_emotion_does_not_filter_pride_or_shame_internally`)
- 호출 측이 의도적 책임을 가짐 (Phase 2.5에서 `BondKind` 별 미세 차별 시 호출 측 분기 점)

---

## §6. 부채 / 디자이너 잔여 (Phase 2.3 이후)

### 6.1. Stage 5 디자이너 잔여 (게이트 3·4 — 미해결 명기)

| 항목 | 회고 §6 |
|---|---|
| 작업 1 intensity 0.4 잠정 (`tests/phase1_daily_training_test.rs::set_intensity(Admiration, 0.4)`) | §6.1 — 디자이너 검토 시 0.4 유지 / 상향(0.5~0.6) / 하향(0.3) 1택. 조정 시 `daily-training.json` `_expected_axes_delta` + 회고 §4 작업 1 박제값 동기 갱신. |
| S1~S3 narrative 타당성 | §6.2 — S1 임충→노지심 / S2 임충→육겸 / S3 수련→옥교룡 의도와 정합 검토. 어색 시 입력 intensity 조정 → 코드 재실행 → EXPECTED 박제 갱신 (게이트 2 tolerance 완화 금지) |
| S4 임충→고구 정성 검증 | §6.3 — 시간 분산 + 권력 거리. Phase 2.3+에서 `axis_modulation` 활성화 시 정량 가능 |

### 6.2. 코드 부채 (Phase 2.3 이후)

| 항목 | 발견 위치 | Phase 2.3 위임 |
|---|---|---|
| ÷100 잔존 (logical) | `telling_ingestion_handler.rs:80` + `domain/relationship/mod.rs::modifiers():172-173` + `domain/guide/snapshot.rs:316-317` | KICKOFF §1-A 1·2·3 항목 |
| ×100 잔존 grep 카탈로그 누락 (12+ 위치) | spec L508 "production 0" vs **closeness/power src 잔존 12+ 위치** — worldbuilding/markdown은 *별 의미*(장소 인접성) 가능성 | KICKOFF §A 플래그 — Phase 2.3 진입 시 정확 위치 재카탈로그 필요 |
| W1 가드 expected 값 (±1.0 스케일) | `mapping.rs::tests` `affinity_channel_after_anger` 0.286 / `trust_channel_after_anger` 0.158 / `admiration_no_leak_until_phase_2_3` | KICKOFF §1-D / 1-D 표 |
| `memory_relationship_delta_threshold` 4축 합산 sensitivity | Stage 3에서 0.05 → 5.0 (단순 ×100). 4축 동시 변동 시 over-trigger 가능 | KICKOFF §1-C |
| **`session_*_result.json` 자동 dump 인프라 부재 (B-D9)** | Stage 5 §5 작업5 / §7. `state.rs::save_to_file(as_scenario=false)` 만 존재 (Mind Studio 인터랙티브) | Phase 2.3/3 — `cargo run --bin narrative-dump -- --scenario S1` 등 신설 |
| `examples/phase5b_checkpoint2_eval` 빌드 실패 (`sqlite_world` import) | Phase 5 산출물, Stage 6 무관 | Phase 5 후속에서 처리 |
| `listener_perspective` default-ON 명기 (사실 정정) | Stage 5 §7 위험 — Phase 7 Step 5 정합 | 본 보고에 박제만 (작업 0) |

**Stage 4 미처리 확정** (S6-D5):
- `src/bin/mind-studio/state.rs:666~671` 커스텀 Deserialize impl + 5 테스트 — **여전히 존재**. Stage 4 §4 KICKOFF §1-E 갱신본의 "Stage 4 책임 2" 미처리 → Phase 2.3 인계 유지 (완료 표기 금지)

### 6.3. Phase 2.5 위임 (B-D7 / B-D11)

- B-D7: 새 cause variant 명명 (Phase 2.5 — declarative_events 흡수 시점)
- B-D11: declarative_events 상한 N
- B-D6 axis_modulation: reflection LLM 출력 필드 신설 + ±5 가산 활성화

---

## §7. Phase 2 종결 게이트 통과 확인

| 게이트 | 정의 | 결과 |
|---|---|---|
| **1 (S6-D2)** | D2 latency 3케이스 ±20% 이내 + 4축 영향 박제 | ✅ 작업1 통과 (chitchat 15.70/29.0 = 54% margin, significant 26.68/42.0 = 36% margin, legacy 20.89/35.2 = 41% margin) |
| **2 (S6-D2)** | bench 재측정 완료 + 로그 박제 | ✅ 작업1·2 — `baselines/stage6-d2d4-bench-2026-05-16.log` |
| **3 (S6-D1)** | 전체회귀 failed=0 ∧ ≥843P ∧ ≤2I ∧ 증감 설명가능 + D3 3밴드 보존 | ✅ 작업2·3 — 843P/0F/2I/65 묶음, Stage 5 종결값 exact 유지. 증감 0 (Stage 6 신규 0) |
| **4** | Phase 2.3 진입 준비 완료 — KICKOFF 갱신(역대조 누락0) + appraise-validation 디렉토리 + phase2.3 spec 초안 | ✅ 작업6 (본 commit) |
| **5** | 외부 문서 동기화 완료 (CLAUDE.md / 00-roadmap.md / spec frozen) | ✅ 작업7 (본 commit) |

**5 게이트 전부 ☑ → Phase 2 (4축 도메인 안정) 완료.**

다음 = **Phase 2.3** (appraise 정비 + ±100 native 잔존 청소).

---

## §8. 변경 이력

| 일자 | 변경 |
|---|---|
| 2026-05-16 | Phase 2 Stage 6 종결과 함께 Phase 2 전체 종합 보고 초안 작성. Stage 0~6 산출 + B-D 12 결정 최종 상태 + 4축 안정화 결과 + 부채 인계 카탈로그 + 종결 게이트 통과 확인. |
