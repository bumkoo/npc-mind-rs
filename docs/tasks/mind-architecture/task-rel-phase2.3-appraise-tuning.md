# Phase 2.3 — Appraise Tuning + ±100 Native 전환 (Spec Draft)

**Status**: 🟡 **DRAFT** — 헤더 + 범위 골격만. 본체는 Phase 2.3 진입 시 작성.
**작성일**: 2026-05-16 (Phase 2 Stage 6 작업 6)
**선행**: Phase 2 종결 ([`phase2-checkpoint-report.md`](phase2-checkpoint-report.md)) + KICKOFF ([`PHASE2.3-KICKOFF.md`](PHASE2.3-KICKOFF.md) v1.2)
**진입 트리거**: 디자이너 결정 — Phase 2.3을 별도 트랙으로 진행할지, Phase 3 (story / tool / world-knowledge) 진입 전 선행할지.

---

## §0. 본 문서의 성격

본 문서는 Phase 2 Stage 6에서 신설된 **초안**이다. 본체 (`§1. 범위`, `§2. 결정사항`, `§3. 작업 분해`, `§4. 종결 게이트`) 는 Phase 2.3 진입 시점에 KICKOFF 인계 사항을 정식 spec으로 승격하면서 작성한다.

본 초안 단계의 목적:
- Phase 2.3 진입 *전에* 본 spec 파일이 존재해야 KICKOFF가 "정식 spec은 본 문서로 분리 작성" 포인터로 가리킬 수 있음 (인계 게이트 4).
- 초안 단계에서 범위 골격만 박아 두면, 디자이너 또는 Phase 2.3 인계자가 *어떤 결정을 해야 하는지* 한눈에 파악 가능.

---

## §1. 범위 골격 (KICKOFF §1 인용)

KICKOFF §1을 정본 그대로 인계한다. 본 spec 본체에서 결정 단위로 분해 예정.

### A) ±100 native 전환 — 잔존 ÷100 layer 청소
- KICKOFF §1-A 본문 3 사이트 + Stage 6 추가 플래그 (12 파일 / 69 매치 분포 힌트)
- 첫 작업 = **12 파일 69 매치 정확 분류 + 변경 카테고리화** (rename / 값 변경 / 폐기 / 별 의미 분리)

### B) Appraise 입력 정밀화
- KICKOFF §1-B (`axis_modulation` / HEXACO 6→24 facet spread / Reflection 게이트 검토)
- `axis_modulation` 활성화 = reflection LLM 출력 필드 신설 (S5-D1 박제 — `ReflectionResult` 7 필드 현재 부재)
- Phase 2.5와 일부 중첩 — 어느 부분이 Phase 2.3 / Phase 2.5인지 spec 본체에서 분리

### C) `memory_relationship_delta_threshold` 4축 합산 sensitivity
- KICKOFF §1-C 3 옵션 (max / 가중합 / OR)
- Phase 2.3 narrative 시뮬 결과 보고 결정

### D) W1 회귀 가드 expected 값 재조정
- KICKOFF §1-D (`mapping.rs::tests` 3 가드)
- Phase 2.3 시작 신호 트리거: W1 PASS인 동안 modifier API 변경 *안 함*. 깨지는 순간 진입

### E) v0.6 시나리오 JSON 로드 — 커스텀 Deserialize 제거
- KICKOFF §1-E (Stage 4 미처리 확정 → 본 Phase 2.3에서 처리)
- `src/bin/mind-studio/state.rs:666~671` 커스텀 Deserialize impl + 5 테스트 제거

---

## §2. Stage 4·5·6 인계 5항 (KICKOFF §5 인용)

KICKOFF §5 1:1 인용. 본 spec 본체에서 *어느 항목을 Phase 2.3 본체에 포함시킬지* 분리.

| 항목 | KICKOFF § | 본 spec 처리 (Phase 2.3 진입 시 결정) |
|---|---|---|
| `session_*_result.json` 자동 dump 인프라 부재 (B-D9) | §5.1 | Phase 2.3 vs Phase 3 분리 결정 |
| 작업 1 intensity 0.4 잠정 확정 | §5.2 | 디자이너 검토 결과 박제 |
| S1~S3 narrative 타당성 검토 | §5.3 | 디자이너 검토 결과 박제 |
| S4 임충→고구 정성 검증 + 시간 분산 | §5.4 | Phase 2.5 위임 검토 (axis_modulation 활성화 후 정량 가능) |
| Stage 4·5 메트릭 baseline | §5.5 | 본 spec 종결 게이트 기준값 |

---

## §3. narrative validation 디렉토리

[`data/scenarios/appraise-validation/`](../../../data/scenarios/appraise-validation/) — Stage 6에서 신설. README 박힘. S1~S4 시나리오 JSON은 Phase 2.3 진입 시 박제.

---

## §4. 본 spec 작성자에게 인계 (Phase 2.3 진입 첫 작업)

1. 본 spec §1·§2 골격을 정식 결정 단위로 분해. KICKOFF §1·§5 1:1 매핑 보존.
2. 결정사항 (P-D1 / P-D2 ... 같은 ID 부여) — 각 결정의 옵션 + 확정 + 근거.
3. 작업 분해 (Stage 단위 — Phase 2.3는 단일 Stage일지 다중 Stage일지 결정).
4. 종결 게이트 정의 (KICKOFF §3 재측정 권장 항목 + W1 expected 갱신 + 12 파일 재카탈로그 + narrative 검증 결과 박제).
5. 본 문서 status `🟡 DRAFT` → `🟢 ACTIVE` 갱신.

---

## §5. 변경 이력

| 날짜 | 변경 |
|---|---|
| 2026-05-16 | Phase 2 Stage 6 작업 6 — 초안 신설. 헤더 + 범위 골격만. 본체는 Phase 2.3 진입 시 작성. KICKOFF v1.2 §1·§5 1:1 인계 포인터. |
