# Phase 2 Follow-up — 런타임 World→Mind 재동기화 endpoint

> **For Claude Code.** Phase 2 본 task와 분리된 짧은 후속 task. 작가 워크플로우 개선용.
> 결정 사항을 임의 변경하지 말 것. 변경이 필요하면 디렉터 승인 요청.
> **선행 조건**: Phase 2 체크포인트 2 통과.

## 1. 목표

`world-load --reload`로 SQLite를 갱신한 후 mind-studio를 **재시작하지 않고도** 변경된 Person을 mind repository에 다시 동기화할 수 있게 한다.

작가가 .md 편집 중 즉시 dialogue_start로 변경된 HEXACO를 시연하고 싶은 워크플로우. 현재는 mind-studio 재시작이 필요해 LLM 캐시·Scene 진행 상태가 초기화됨 — 이 task로 그 부담 제거.

## 2. 연관 컨텍스트

- `task-phase2-person-vertical-slice.md` §3.5 / §6.8 — mind upsert idempotent 정책 (기존 emotion_state·scene·memory 보존)
- `phase2-checkpoint1-report.md` §5.5 — "런타임 sync는 별도 task" 디렉터 결정
- `src/bin/mind-studio/state.rs` — `AppState::sync_world_persons_into_repo` 메서드 (이미 구현됨, 시작 시점만 호출)
- `src/bin/mind-studio/handlers/world_persons.rs` — 추가될 endpoint 위치

## 3. 제약

### 3.1 idempotent 정책 유지

- 같은 NpcId가 이미 있으면 personality·name·description만 갱신. emotion_state·scene·memory는 절대 손대지 않음.
- 이미 `AppState::sync_world_persons_into_repo`가 `inner.npcs.insert`로 덮어쓰고 `rebuild_repo_from_inner()`를 호출하는 구조 — `rebuild_repo_from_inner`이 emotion·scene을 보존하는지 코드 레벨 검토 필요.

### 3.2 단일 endpoint

- POST `/api/world/persons/sync` — body 없음, 응답 `{ "synced": <count> }`
- 같은 path는 mind-studio MCP에 추가하지 않음 (REST만). MCP는 read-only 도구로 유지하는 기존 정책.

### 3.3 인증·동시성

- mind-studio가 단일 작가용이라 인증 미고려.
- 동시 호출 방지는 `inner` RwLock·`shared_dispatcher.repository_guard()` Mutex 자연 직렬화로 충분.

## 4. Done Criteria

- [ ] POST `/api/world/persons/sync` endpoint 추가 (`src/bin/mind-studio/handlers/world_persons.rs`)
- [ ] `AppState::sync_world_persons_into_repo`는 그대로 재사용 (기존 코드 변경 없음)
- [ ] 단위·integration 테스트 1-2건 — 호출 후 inner.npcs 갱신 확인 + emotion_state 보존 확인
- [ ] CLAUDE.md "Mind Studio" 섹션에 endpoint 1줄 추가
- [ ] `cargo build --features mind-studio,chat,embed` 통과

추정 변경량: ~30 LOC (handler 1 + route 등록 1 + test 1).

## 5. 단계별 작업

### Step 1 — Handler 추가

```rust
// src/bin/mind-studio/handlers/world_persons.rs

#[derive(serde::Serialize)]
pub struct SyncResponse {
    pub synced: usize,
}

pub async fn sync_persons(
    State(state): State<AppState>,
) -> Result<Json<SyncResponse>, AppError> {
    if state.world_store.is_none() {
        return Err(AppError::NotImplemented(
            "world index 미구성 — NPC_MIND_WORLD_DB + world-load 실행 필요".into(),
        ));
    }
    let n = state
        .sync_world_persons_into_repo()
        .await
        .map_err(AppError::from)?;
    Ok(Json(SyncResponse { synced: n }))
}
```

### Step 2 — Route 등록

```rust
// src/bin/mind-studio/main.rs (Phase 2 Person 라우트 블록 안)
.route("/api/world/persons/sync", post(handlers::world_persons::sync_persons))
```

순서 주의: search·{id} 다음 등록 — axum 매칭 우선순위.

### Step 3 — 테스트

`tests/world_persons_runtime_sync_test.rs` (또는 기존 통합 테스트에 추가):
1. SqliteWorldStore에 npc-02 등록 → mind-studio 부팅 → inner.npcs 검증 → 1명
2. world store에 npc-03 추가 (직접 upsert) → POST /api/world/persons/sync → 응답 `{synced: 2}`
3. inner.npcs 검증 → 2명
4. emotion_state 보존 검증 — sync 전 npc-02 감정 설정 → sync 후에도 유지

### Step 4 — 보고서

`docs/tasks/phase2-followup-runtime-sync-report.md` 약 80 라인.

## 6. Out of Scope

- 단일 인물 sync (POST `/api/world/persons/{id}/sync`) — 본 task에선 일괄 sync만. 부분 sync는 Phase N+
- 자동 watch (파일 변경 감지) — 작가가 명시적으로 호출하는 흐름 유지
- 인증·rate limit — dev tool 가정
- world-load CLI 후 자동 호출 — CLI와 mind-studio가 분리 프로세스라 비현실적. 작가가 명시적으로 호출

## 7. 시작 체크리스트

1. `src/bin/mind-studio/state.rs::sync_world_persons_into_repo` 코드 재확인
2. `rebuild_repo_from_inner`이 emotion_state 보존하는지 코드 레벨 검증 (필요 시 emotion_state도 별도 보존 로직 추가)
3. handler + route + test 추가
4. CLAUDE.md 1줄 + 보고서

## 8. 추정 작업량

- Claude Code 작업 시간: 1 시간 미만
- 코드 변경 줄 수: ~30
