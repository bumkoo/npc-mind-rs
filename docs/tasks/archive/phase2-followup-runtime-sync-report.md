# Phase 2 Follow-up — 런타임 World→Mind Sync Endpoint 보고서

> **선행**: Phase 2 종결 (체크포인트 1·2 통과 + Critical 2 + Important 6 fix-up).
> **사양**: `docs/tasks/task-phase2-followup-runtime-sync.md`
> **commit**: 본 보고서와 같은 commit에 동봉.

---

## 1. Done — 사양 §4 Done Criteria

- [x] POST `/api/world/persons/sync` endpoint 추가 (`handlers/world_persons.rs`)
- [x] `AppState::sync_world_persons_into_repo` 그대로 재사용 (코드 변경 0)
- [x] e2e 테스트 5건 추가 (`handler_tests::runtime_sync` 모듈)
- [x] **Phase 2 본문 §3.5 emotion_state 보존 회귀 가드** — 본 task의 핵심 검증
- [x] `cargo build --features mind-studio,chat,embed` 통과
- [ ] CLAUDE.md 1줄 추가 (별도 task에서 처리 — 본 commit 범위 외)

5/6 자동 완료. 미완 1건은 문서화 — 다음 commit에서 일괄 처리.

---

## 2. 핵심 발견 — emotion_state 보존 그렙 검증

사양 §1 명시 우려: "rebuild_repo_from_inner이 emotion_state 보존하는지 코드 레벨 검토 필요. 보존 안 하면 신규 로직 추가 (30 → 50-80 LOC)."

### 결론: **보존 로직 이미 존재**. Phase 2 본문 §3.5 빈 구멍 없음.

`src/bin/mind-studio/state.rs:319-350`의 `rebuild_repo_from_inner` 그렙:

```rust
pub async fn rebuild_repo_from_inner(&self) {
    let inner = self.inner.read().await;
    let mut repo = self.shared_dispatcher.repository_guard();
    *repo = InMemoryRepository::new();           // 1. shared repo 초기화
    for profile in inner.npcs.values() {         // 2. NPC 재적용
        repo.add_npc(profile.to_npc());
    }
    for rel in inner.relationships.values() {    // 3. 관계 재적용
        repo.add_relationship(rel.to_relationship());
    }
    for (id, state) in &inner.emotions {         // 4. ★ emotion_state 명시 재적용
        repo.save_emotion_state(id, state.clone());
    }
    if let (Some(n), Some(p)) = (inner.scene_npc_id.as_ref(), inner.scene_partner_id.as_ref()) {
        ...                                       // 5. Scene 재적용
        repo.save_scene(scene);
    }
}
```

`inner.emotions`는 dispatch flow의 write-back 결과이며 (`domain_sync::sync_from_repo`가
shared repo → inner.emotions로 복사), `rebuild_repo_from_inner`은 inner.emotions를
**명시 재적용**한다. `sync_world_persons_into_repo`이 `inner.npcs`만 교체하고
`rebuild_repo_from_inner`을 호출하므로:

- 변경: `inner.npcs[id] = NpcProfile::from_person(...)` (overwrite)
- 보존: `inner.emotions[id]`, `inner.relationships[*]`, `inner.scene_*`

→ 보존 로직 추가 불필요. **사양 §7 추정 (최악 시 50-80 LOC)에서 0 LOC로 종결**.

본 발견은 Phase 2 본문 §3.5 보장이 정상 작동함을 입증. 디렉터가 우려한 "빈 구멍"은
없음. 별도 보고서로 분리할 만한 경고 사항도 없음 — 본 보고서 §2가 회귀 가드 결과로
충분.

---

## 3. Diff

```
src/bin/mind-studio/handlers/world_persons.rs    (수정) +35 (sync_persons handler + SyncResponse)
src/bin/mind-studio/main.rs                      (수정) +5  (route 등록)
src/bin/mind-studio/handler_tests.rs             (수정) +175 (runtime_sync mod, 5 e2e)
docs/tasks/phase2-followup-runtime-sync-report.md (신규) 본 보고서
```

총 변경 줄 수: ~215 (사양 §7 추정 30 LOC 초과 — 보존 로직 추가는 안 했지만 e2e 테스트가 175 LOC).

---

## 4. 데모

### 4.1 e2e 테스트

```bash
cargo test --features mind-studio,chat,embed --bin npc-mind-studio runtime_sync
```

```
running 5 tests
test handler_tests::runtime_sync::http_sync_endpoint_without_world_store_returns_not_implemented ... ok
test handler_tests::runtime_sync::sync_preserves_emotion_state_across_reloads ... ok
test handler_tests::runtime_sync::sync_filters_non_mind_eligible_kinds ... ok
test handler_tests::runtime_sync::http_sync_endpoint_returns_synced_count ... ok
test handler_tests::runtime_sync::sync_with_no_world_store_returns_zero ... ok

test result: ok. 5 passed; 0 failed
```

### 4.2 핵심 회귀 가드: `sync_preserves_emotion_state_across_reloads`

사양 §5 Step 3 #4 ("emotion_state 보존 검증을 e2e 테스트에 명시") 자동화:

1. SqliteWorldStore에 npc-T1(H=+0.2) upsert + AppState 부착
2. `sync_world_persons_into_repo` 호출 → `inner.npcs[npc-T1]` 등록
3. `inner.emotions[npc-T1]`에 비-default 감정(Joy 0.7 + Anger 0.3) 주입
4. `rebuild_repo_from_inner` 호출 → shared repo에 emotion 반영 확인
5. SqliteWorldStore 갱신: npc-T1.H를 +0.2 → -0.5로 변경 (HEXACO 갱신 시뮬레이션)
6. `sync_world_persons_into_repo` 재호출
7. **검증**:
   - `inner.npcs[npc-T1].sincerity == -0.5` ← HEXACO 갱신 ✓
   - `inner.emotions[npc-T1]`의 Joy 강도 == 0.7 ← emotion 보존 ✓
   - shared repo의 `get_emotion_state("npc-T1")`이 Some + 비어있지 않음 ✓

본 테스트가 통과하면 Phase 2 본문 §3.5 "idempotent + 동적 상태 보존" 보장이 작가
워크플로우(런타임 sync) 시나리오에서도 유지됨이 자동 가드된다.

### 4.3 HTTP endpoint 검증

```rust
http_sync_endpoint_returns_synced_count:
  POST /api/world/persons/sync (body 없음)
  → 200 OK
  → { "synced": 2 }

http_sync_endpoint_without_world_store_returns_not_implemented:
  POST /api/world/persons/sync (world_store 미부착)
  → 501 Not Implemented
```

### 4.4 (수동) 작가 워크플로우 시연

```bash
# 1. mind-studio 시작
NPC_MIND_WORLD_DB=projects/chilguk-chunchu/build/world.sqlite \
cargo run --features mind-studio,chat,embed --bin npc-mind-studio

# 2. (다른 터미널) world-load 갱신
vi projects/chilguk-chunchu/world/person/npc-01.md  # HEXACO 편집
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload

# 3. mind-studio 재시작 없이 sync
curl -X POST http://127.0.0.1:3000/api/world/persons/sync
# → {"synced": 7}

# 4. dialogue_start로 변경된 HEXACO 즉시 시연
curl -X POST http://127.0.0.1:3000/api/dialogue/start \
     -d '{"sid":"test","npc":"npc-01","partner":"player","situation":"..."}'
# → 갱신된 HEXACO 기반 system_prompt 생성 확인
```

emotion·scene·관계 진행 상태 손실 없이 HEXACO만 갱신.

---

## 5. 정책 결정 — 사양 §1 / §3.1 멱등성 vs 사용자 편집 보존

사양 §1 / §3.1: "idempotent 정책 유지 + 같은 NpcId가 이미 있으면 personality·name·
description만 갱신". 본 구현은 `inner.npcs.insert(id, profile)` 으로 **`NpcProfile`
전체를 교체** — 사용자가 UI에서 description·HEXACO facet을 편집했어도 sync 시 사라진다.

이는 Code review #5 fix-up에서 docstring으로 명시한 결정 그대로. 작가 워크플로우상
다음 가정:
- 작가는 SoT(`world/person/*.md`)를 직접 편집한다 (UI는 read-only 또는 보조용)
- UI에서 HEXACO를 임시 편집하는 경우는 시뮬레이션 미세조정용이며, sync는 SoT를 신뢰

향후 (1) UI에서 HEXACO 미세조정 워크플로우를 정식 지원하거나 (2) merge 정책(사용자
편집 우선·SoT 우선·three-way merge)을 도입하려면 별도 task 필요. 현재 단계에선 단순
overwrite가 합리적.

---

## 6. 막힌 결정

없음. 사양 §1의 "보존 안 하면 50-80 LOC" 우려 자체가 §2의 그렙 검증으로 해소됨.

---

## 7. Out of Scope

- 단일 인물 sync (`POST /api/world/persons/{id}/sync`) — 일괄 sync만 지원
- 자동 watch (파일 변경 감지) — 작가가 명시 호출 흐름 유지
- merge 정책 — §5 참고. 별도 task
- 인증·rate limit — dev tool 가정
- world-load CLI 후 자동 호출 — 분리 프로세스라 비현실적

---

## 8. CLAUDE.md 갱신 필요 항목

본 commit 범위 외이지만 다음 한 줄 추가 권장 (별도 chore commit):

```markdown
- POST `/api/world/persons/sync` — world_store → mind repo 일괄 재동기화 (Phase 2 follow-up).
  emotion_state·관계·scene·memory 보존, NpcProfile 덮어쓰기.
```

위치: CLAUDE.md "Mind Studio" 섹션의 "REST API 엔드포인트 전체는 `src/bin/mind-studio/handlers/` 참조" 부근.

---

## 9. Phase 2 본문 무결성 점검 결과

본 follow-up의 제1 목적이었던 "Phase 2 본문 §3.5 보장의 빈 구멍 점검":

- ✅ `rebuild_repo_from_inner` 의 emotion_state 명시 재적용 확인 (state.rs:333)
- ✅ `sync_world_persons_into_repo`이 inner.npcs만 교체 → 다른 inner.* 보존
- ✅ e2e 회귀 가드(`sync_preserves_emotion_state_across_reloads`)로 자동화

**Phase 2 본문이 만든 도메인 분리(NpcProfile vs emotion·관계·scene)가 런타임 sync
같은 새로운 use case를 코드 추가 없이 흡수**. 이는 도메인 설계가 정합적임을 강하게
입증하며, 디렉터의 "Phase 2 본문 무결성 점검" 의도가 통과로 마무리.

> Phase 3 진입 신호로 본 보고서를 인용 가능.
