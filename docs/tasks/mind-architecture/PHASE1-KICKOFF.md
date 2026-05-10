# Phase 1 Kickoff — Claude Code 인계 가이드

> 본 문서는 Claude Code (또는 다른 구현자)가 Phase 1 작업을 *시작*할 때 읽는 진입점.
> spec 본문은 `task-rel-phase1-reflection.md` (1928줄). 본 문서는 *그것을 읽기 전 메타*.

## 작업 시작 순서

1. **다음 4개 문서를 *순서대로* 읽기**:
   1. `CLAUDE.md` — 프로젝트 전체 컨텍스트 (3-layer 구조, dispatch_v2, ports/, policies/, UoW, 빌드 매트릭스)
   2. `docs/game-design/2-characters/relationships.md` v0.7 §6 — Reflection 설계 의도 + 게이트 가드레일 + LLM↔Engine 분업
   3. `docs/tasks/mind-architecture/00-roadmap.md` v0.2 — Phase 1/2/3a/3b/3c 전체 phasing + Gap analysis + Concept→Code 매핑
   4. `docs/tasks/mind-architecture/task-rel-phase1-reflection.md` v0.1 — **Phase 1 spec 본문** (1928줄, 6 stage)

2. **Stage 0 진입 *전에* 확인**:
   - `cargo test --workspace --all-features` 실행 → 통과 카운트 박제 (회귀 baseline)
   - dispatch_v2 시간 측정 → latency baseline 박제
   - 두 baseline을 *spec 문서*의 §"Stage 0 Findings" 섹션에 추가

3. **Stage 0 실행** — spec §5 Stage 0 그대로:
   - 8개 grep audit 패턴 실행 → 결과를 spec §"Stage 0 Findings"에 캡처
   - 4개 spot-read 파일 (rig_chat.rs, relationship_policy.rs, dialogue_orchestrator.rs, uow.rs) → 핵심 패턴 spec에 박제
   - 5개 결정 항목 (가)~(마) 확정 → spec §4.4에 추가
   - 추가 위험 발견 시 spec §11에 추가
   - Impact Map 표 완성

4. **Stage 0 완료 후 Bekay 검토 받음**. 결정 (가)~(마)이 spec 작성 시점의 *권장*과 다를 수 있음. 합의 후 Stage 1 진입.

5. **Stage 1~5 순차 실행**. 각 stage 게이트 통과 후 다음 진입.

## 핵심 원칙

### 코드 변경 전 *반드시* 직접 읽기 (Tier B 규율)

본 spec은 *spec 작성자의 가정* 위에 있음. 일부는 검증됐고 일부는 추론. 구현 전에 *해당 파일을 직접 읽고*:

| 검증 안 된 가정 | 직접 확인할 파일 |
|---|---|
| RigChatAdapter 다중 세션 동시 처리 가능 | `src/adapter/rig_chat.rs` |
| RelationshipPolicy 현재 follow-up 발행 패턴 | `src/application/command/policies/relationship_policy.rs` |
| DialogueOrchestrator 세션→NPC 매핑 | `src/application/dialogue_orchestrator.rs` |
| UoW.add_event 호출 패턴 | `src/application/command/uow.rs` |
| AfterDialogueResponse 위치 (scene.rs vs 별도?) | `src/application/dto/` 디렉토리 |
| Npc::compass_short_label() 메서드 존재 여부 | `src/domain/npc.rs` 또는 동등 |
| uuid crate 의존성 존재 여부 | `Cargo.toml` |

→ Stage 0의 *spot-read*가 정확히 이 일. 결과가 *spec 가정과 다르면* spec을 *그 결과에 맞춰 수정*.

### OCP 위반 검사 — PR 전 자동 grep

```
findstr /S /I "ConversationBackedReflectionPort" src\application\
# 결과: 0건이어야 함

findstr /S /I "use crate::adapter" src\application\reflection_service.rs
# 결과: 0건이어야 함
```

### 빌드 매트릭스 — 모두 통과해야 함

```
cargo check --all-features
cargo build --features chat
cargo build --no-default-features
cargo build --features mind-studio,chat,embed --bin npc-mind-studio
cargo test --workspace --all-features
cargo clippy --workspace --all-features -- -D warnings
```

특히 `--no-default-features` 빌드는 Phase 1 후 *절대 깨지지 말아야 함*. ReflectionService는 chat feature gated, RelationshipPolicy의 게이트도 reflection: None 호환 분기 필요.

## 의사결정 권한 (Tier 정책)

| 영역 | 권한 |
|---|---|
| spec과 다른 *코드 패턴* 발견 시 | spec 수정 → Bekay 알림 |
| spec의 *결정 (가)~(마)* 확정 | Stage 0 결과 보고 → Bekay 결정 |
| spec과 일치하는 *상세 구현* | Claude Code 자율 |
| 기존 테스트 깨짐 | 즉시 중단 → Bekay 알림 |
| 새 dependency 추가 (uuid 외) | Bekay 사전 합의 |
| 가중치/임계값 (0.3, 0.40/0.30/0.15/0.15) 조정 | Stage 4 narrative validation 결과 기반, Bekay 합의 |

## 체크포인트 보고

각 Stage 완료 시 다음 형식으로 보고:

```
## Stage X — 완료

### 산출물
- 파일 N개 추가, M개 수정
- 테스트 N개 추가
- ...

### 게이트 통과
- [✓] cargo check
- [✓] cargo test (~543개 → ~548개)
- ...

### 발견 사항
- (예상 외 catch, spec과 다른 패턴 등)

### 다음 stage 진입 가능 여부
- [✓ / ⚠ / ✗]
```

전체 phase 완료 시 `phase1-checkpoint-report.md` 별도 작성 (`docs/tasks/mind-architecture/`).

## 막힐 때

- spec이 *모호*하면 → 본 spec의 §4.4 11개 핵심 결정으로 돌아가서 *어느 결정에 위배되는가* 자문
- 코드 패턴이 *spec과 다름*이면 → spec 수정 우선. 코드를 spec에 맞추려 fight하지 말 것
- 빌드가 *반복적으로 깨짐*이면 → Stage 분리 잘못. 큰 변경을 작은 stage로 분할
- *결정 항목 (가)~(마) 외*에 새 결정 필요하면 → spec §4 또는 §11 (위험)에 박제 후 Bekay에게

## 관련 인접 문서 (필요 시 참조)

| 영역 | 경로 |
|---|---|
| 무협 어휘 / 톤 | `docs/game-design/00-pillars.md` v0.2 (Pillar 6 LLM↔Engine 분업) |
| 행동 트리거 (Phase 3c 사전 학습용) | `docs/game-design/2-characters/action_triggers.md` |
| 인물 검증 사례 | `docs/character-validation/` |
| 이벤트 카탈로그 | `docs/architecture/event-handler-catalog.md` |
| dispatch_v2 internals | `docs/architecture/dispatch-v2-internals.md` |
| Frontend 구조 (Phase 1.5 follow-up 시) | `docs/architecture/frontend-architecture.md` |

## 변경 이력

| 버전 | 일자 | 변경 |
|---|---|---|
| v0.1 | 2026-05-10 | Claude Code 인계용 kickoff 가이드 작성. spec v0.1 + 00-roadmap.md v0.2 동반. |
