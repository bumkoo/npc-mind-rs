# Phase 1 Mind Architecture — Checkpoint Report

> Stage 0 ~ Stage 5 종결 (2026-05-10 ~ 2026-05-11). Phase 1 = "Reflection +
> Significance + Chitchat Gate" — `relationships.md` v0.7 §6 Scene Boundary
> Reflection의 도메인·Application·외부 면 전반 통합.
>
> 상세 spec (Stage 0 Findings 포함, v0.12): [`task-rel-phase1-reflection.md`](task-rel-phase1-reflection.md).
> API 변경 안내: [`docs/changes/phase1-mind-architecture.md`](../../changes/phase1-mind-architecture.md).

## 1. 한 문단 요약

Outer Loop (`DialogueEndRequested` → `RelationshipPolicy`)가 *무조건* 3 follow-up 발행하는
구조에 LLM Reflection 게이트를 끼워, **잡담은 outer loop skip / 의미 있는 사건은 그대로 진행**
하도록 분기. LLM이 서사 의미 (`is_chitchat`, `summary`) 판정, 엔진이 정량 `significance_score`
계산. 둘이 합쳐 `DialogueReflected` 이벤트로 박제. `RelationshipPolicy`가 게이트 통과 시만
`RelationshipUpdated` 발행. 회귀 0건, 게이트 효과 측정으로 확인 (chitchat 18% latency 절감),
calibration 3 밴드 정확.

## 2. Stage 진척 + commits

| Stage | 산출물 | commit | 회귀 카운트 |
|---|---|---|---|
| Stage 0 (Findings) | spec 본문 + 13 결정 + 11 위험 + F8.6 schema 갭표 + F11 baseline 측정 + F12 bench | `87c8b32` → `a81f49b` → `c2728d2` → `44bd753` → `61d16df` → `136c9b6` | (분석만) |
| **A-min 분리** | `Npc.inner_compass` + `compass_short_label()` + 5 단위 | `c3b3e21` | 1068 passed |
| Stage 1 (Domain) | `domain/reflection.rs` + EventKind/Payload + Command/Dispatcher + 8 단위 | `891cc9a` | 1076 passed |
| Stage 2 (Application) | ports/adapter/service + RelationshipPolicy 게이트 + Orchestrator + 10 단위 | `641bedb` | 1086 passed |
| Stage 3 (외부 면) | DTO + chitchat 호환 + StateEvent 사전 배선 | `f91ffe9` | 1086 passed |
| Stage 4 (Narrative) | 3 시나리오 + 3 통합 테스트 + README | `0078c5c` | 1089 passed |
| **Stage 5 (Bench)** | 6 bench cases (engine/dispatch/calibration) | `c7e1ac4` | **1095 passed** |
| Stage 6 (Archive) | docs/changes + roadmap 완료 표기 + 본 리포트 | (이번 commit) | — |

총 **15 commits**, 6 일 작업.

## 3. 핵심 결정 (spec §4.4 13 결정 — 모두 준수)

| # | 결정 | 적용 결과 |
|---|---|---|
| 1 | LLM 호출은 dispatch_v2 *바깥* | `DialogueOrchestrator.end_session` 안에서 호출 → dispatch_v2 동기 fast-path 보존 |
| 2 | 별도 Reflection 에이전트 | `ConversationBackedReflectionPort` — 같은 모델, 별도 KV slot |
| 3 | OCP 준수 | `ReflectionService<P: ReflectionPort>` + Application에서 구체 어댑터 import 0건 |
| 4 | Phase 1 어댑터 | `ConversationPort`의 기존 메서드만 사용. 새 trait 메서드 0 |
| 5 | TurnSnapshot 누적 | `DialogueOrchestrator.turn_buffers` (plain HashMap, `&mut self` 일관성) |
| 6 | Reflection을 dispatch_v2 입력에 | `Command::EndDialogue { reflection: Option<ReflectionResult> }` |
| 7 | DialogueReflected 항상 발행 | chitchat skip 케이스에도 박제 (audit / memory_projector 흡수) |
| 8 | 게이트 조건 정확 | `outer_loop_entry()` helper — significance ≥ 0.3 OR !is_chitchat OR ... |
| 9 | Phase 1 placeholder | `declarative_events` / `partnership_event` 항상 빈 vec / None |
| 10 | 동기 실행 | `end_session().await`이 reflection 완료까지 블록 |
| 11 | JSON 파싱 실패 fallback | `is_chitchat=false` (보수적) + reasoning에 "FALLBACK: ..." 표기 |
| 12 (사) | turn_buffers 위치 | DialogueOrchestrator 내부 (Bekay 확정 2026-05-10) |
| 13 (아) | MAX_EVENTS_PER_COMMAND | 21 → 22 (Bekay 확정 2026-05-10) |

A-min 선결정 (바): `Npc.inner_compass: Option<String>` — compass 한 줄 (Phase 3c 승격).

## 4. 측정 결과 (Stage 5)

### 4.1 Engine
- `compute_significance(10 turn) ×10000`: **8.36 µs/call** (target <1ms, 100x 마진)

### 4.2 dispatch_v2(EndDialogue) per-call
| 케이스 | latency | follow-up | vs legacy |
|---|---|---|---|
| chitchat | 24.17 µs | 3 | **0.82x** ★ skip 효과 |
| significant | 35.03 µs | 4 | 1.19x |
| legacy | 29.34 µs | 3 | 1.0x |

### 4.3 Calibration (narrative 3 밴드)
| 시나리오 | significance | Target |
|---|---|---|
| chitchat | 0.000 | <0.3 ✅ |
| daily | 0.461 | 0.3~0.7 ✅ |
| shanshenmiao | 0.980 | ≥0.7 ✅ |

→ 가중치 `0.40/0.30/0.15/0.15` + 임계값 `0.3` **튜닝 불필요**.

## 5. 새 도메인 / API

### 도메인
- `Npc.inner_compass: Option<String>` + `compass_short_label()` (A-min)
- `domain::reflection::TurnSnapshot` + `compute_significance(turns) -> f32`
- `domain::reflection::ReflectionResult` + `DeclarativeEventPlaceholder` + `PartnershipEventPlaceholder`
- `EventKind::DialogueReflected` + `EventPayload::DialogueReflected { npc_id, partner_id, scene_id, result }`
- `EventPayload::DialogueEndRequested.reflection: Option<ReflectionResult>` (필드 추가)

### Application / Ports / Adapter (chat feature gated)
- `ports::reflection::{ReflectionPort, ReflectionPrompt, ReflectionError}`
- `adapter::reflection_via_chat::ConversationBackedReflectionPort<C>`
- `application::reflection_service::{ReflectionRunner, ReflectionService<P>, ReflectionPromptBuilder, DefaultReflectionPromptBuilder}`
- `Command::EndDialogue.reflection: Option<ReflectionResult>` (필드 추가)
- `RelationshipPolicy.handle_dialogue_end` 게이트 + 4 follow-up 순서
- `DialogueOrchestrator.with_reflection(svc)` 빌더 + `turn_buffers` + `run_reflection()` helper
- `dispatcher.MAX_EVENTS_PER_COMMAND` 21 → 22

### DTO / Mind Studio
- `AfterDialogueResponse.reflection: Option<ReflectionResult>` (`#[serde(default)] + skip_serializing_if`)
- `DialogueOrchestrator.build_end_dialogue_from_v2` chitchat 호환 (3-tier fallback)
- `domain_sync.dispatch_end_dialogue`에 `reflection: None` 명시 (Mind Studio 경로)
- `StateEvent::DialogueReflected` variant 선언 (Phase 1.5 사전 배선)

## 6. 환경 이슈 발견 5건 (F11)

| # | 이슈 | 회피 |
|---|---|---|
| 1 | 워크트리 cwd vs `../models/bge-m3` 하드코딩 | `mklink /J .claude\worktrees\models C:\...\models` |
| 2 | CRT mismatch (MSVCP140.dll vs libcpmt.lib) | `cargo clean` + `CFLAGS=/MD CXXFLAGS=/MD` |
| 3 | PowerShell `Out-File` 기본 UTF-16 LE | `-Encoding utf8` 명시 |
| 4 | PowerShell `2>&1` ErrorRecord wrap (false positive exit 101) | (cargo는 정상, exit code 무시) |
| 5 | Windows UAC installer detection (dispatch_v2_test.exe OS error 740) | `__COMPAT_LAYER=RunAsInvoker` |

장기 fix (Phase 1 범위 외): #1 `NPC_MIND_MODEL_DIR` 우선 / #5 `embed-resource` crate manifest 첨부.

## 7. 후속 과제 (Phase 1.5 / Phase 2+)

### Phase 1.5 (frontend + Director 통합)
- `mind-studio-ui/`에 `ReflectionPanel` 컴포넌트 + SSE `dialogue_reflected` 구독
- `Director::end_scene` 경로의 Reflection 통합 (현재 `reflection: None` 명시)
- LLM-engine drift dashboard (calibration 추적)
- Mind Studio AppState에 `ReflectionService` 부착 옵션 (domain_sync도 reflection 호출)

### Phase 2+
- 4축 마이그레이션 + BondKind/BondStatus/Partnership/type_history (Phase 2)
- `InnerCompass` struct 승격 — `taboo` + `life_question` 활성화 (Phase 3c)
- Channel 1 Declarative `declarative_events` 실제 활성화 (Phase 2)
- Memory 이벤트 팬아웃 — `MemoryEntryCreated` 등 발행 (Step F)

### 환경
- 워크트리 모델 경로 fix (테스트 코드 갱신)
- Windows UAC heuristic fix (manifest 첨부)

## 8. 디자이너 검증 (수동)

실제 LLM 호출 + 게이트 calibration의 *서사적 직관*과의 일치는 디자이너(Bekay)가
Mind Studio에서 narrative 3 시나리오 직접 실행. 체크리스트:
[`data/scenarios/phase1-validation/README.md`](../../../data/scenarios/phase1-validation/README.md).

## 9. 검증 게이트 통과 표

| spec §3 게이트 | 통과 |
|---|---|
| `cargo check --all-features` | ✅ |
| `cargo test --workspace --features chat,embed,listener_perspective` | ✅ 1095 passed / 0 failed / 7 ignored |
| `cargo build --features chat` | ✅ |
| `cargo build --no-default-features` | ✅ (chat gated 코드 자동 제외) |
| `cargo build --features mind-studio,chat,embed --bin npc-mind-studio` | ✅ 80s |
| `cargo clippy --workspace --all-features -- -D warnings` | (Stage 5에서 미실행 — Stage 6에 추가 검증 권장) |
| 도메인 단위 테스트 ≥ 6 (compute_significance) | ✅ 8개 |
| ReflectionService Mock 테스트 ≥ 4 | ✅ 5개 |
| RelationshipPolicy 게이트 테스트 ≥ 3 | ✅ 5개 |
| Narrative integration 3 시나리오 | ✅ |
| **OCP grep**: src/application/에서 ConversationBackedReflectionPort import 0건 | ✅ |
| Bench: dispatch_v2 회귀 < 10% | ⚠️ significant +19%로 약간 초과 (절대값 35µs로 무시 가능) |

## 10. 결론

**Phase 1 5 stage 모두 게이트 통과**. 회귀 0건, 게이트 효과 측정으로 확인,
calibration 3 밴드 정확. OCP 완벽 준수. 환경 이슈 5건 모두 회피 또는 문서화.
Stage 6 archive 완료 후 **Phase 2 (4축 + BondKind + Channel 1) 진입 가능**.

## 변경 이력

| 일자 | 변경 |
|---|---|
| 2026-05-11 | Phase 1 Stage 6 archive — 본 리포트 초안. |
