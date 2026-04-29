# 아키텍처 결정 기록: MemoryRepository Async/Sync 전략

> **문서 버전**: v1.0.0  
> **작성일**: 2026-02-20 22:30:00  
> **상태**: 확정  
> **관련 문서**: step3.2-embedding-strategy.md, architecture-decision.md  
> **스프린트**: Sprint 3 — Step 3.2.1 (LanceDB 연결 + 스키마 생성)  

---

## 1. 문제

LanceDB Rust SDK는 전체 API가 async이다(`connect().execute().await`, `query().execute().await`). 반면 wuxia-core의 `MemoryRepository` 트레이트는 sync이다. 이 불일치를 어떻게 해결할 것인가.

```
  wuxia-core (sync)              LanceDB SDK (async)
  ─────────────────              ──────────────────
  trait MemoryRepository {       db.connect(uri)
    fn save(&mut self, ...)        .execute().await
    fn search(&self, ...)        table.query()
    fn count(&self, ...)           .nearest_to(vec)
  }                                .execute().await
  
  ↑ 동기 인터페이스               ↑ 비동기 인터페이스
  
  이 둘을 어떻게 연결할 것인가?
```

---

## 2. 결정

**B안 — sync 유지 + block_on 방식을 채택한다.**

`MemoryRepository` 트레이트는 sync로 유지하고, `LanceDbRepository` 구현체 내부에서 전용 tokio Runtime을 생성하여 `block_on()`으로 LanceDB async API를 호출한다.

```rust
pub struct LanceDbRepository {
    runtime: tokio::runtime::Runtime,   // 전용 tokio 런타임
    db: lancedb::Database,
    table: lancedb::Table,
    embedder: Box<dyn EmbeddingPort>,
}

impl MemoryRepository for LanceDbRepository {
    fn save(&mut self, entry: MemoryEntry) -> Result<(), String> {
        self.runtime.block_on(async {
            // LanceDB async API 호출
            self.table.add(records).execute().await
        })
    }
    
    fn search(&self, ...) -> Vec<ScoredMemory> {
        self.runtime.block_on(async {
            self.table.query()
                .nearest_to(&query_vector)
                .execute().await
        })
    }
}
```

---

## 3. 검토한 대안

### A안: MemoryRepository를 async로 전환

트레이트의 모든 메서드를 `async fn`으로 변경하고, `async-trait` 크레이트를 사용하여 dyn dispatch를 지원하는 방안.

```rust
#[async_trait]
trait MemoryRepository {
    async fn save(&mut self, entry: MemoryEntry) -> Result<(), String>;
    async fn search(&self, ...) -> Vec<ScoredMemory>;
}
```

**기각 사유**: Bevy의 공식 비동기 패턴을 조사한 결과, async 전환이 불필요함을 확인했다(후술).

### B+ 안: 전용 워커 스레드 + 채널

LanceDbRepository가 내부적으로 별도 스레드를 생성하고, sync 메서드는 채널로 요청을 보낸 뒤 응답을 기다리는 방안.

```
  호출자 → channel.send(요청) → [워커 스레드: tokio 런타임] → channel.recv(응답)
```

**기각 사유**: block_on이 안전한 상황(아래 분석)에서 채널 추상화는 불필요한 복잡도 추가.

### C안: 별도 AsyncMemoryRepository 트레이트

sync와 async 트레이트를 분리하여 이중 관리하는 방안.

**기각 사유**: 트레이트 2개를 유지보수하는 부담이 이점 대비 과도함.

---

## 4. 결정 근거

### 4.1 Bevy 공식 비동기 패턴 — "sync를 태스크 풀에 감싸라"

Bevy Cheatbook의 공식 예제가 다음 패턴을 보여준다.

```rust
// Bevy Cheatbook 공식 예제
fn async_compute_system(
    mut commands: Commands,
    task_pool: Res<AsyncComputeTaskPool>
) {
    let handle = task_pool.spawn(async move {
        // sync 블로킹을 async 블록 안에서 직접 사용
        std::thread::sleep(std::time::Duration::from_secs(2));
        42
    });
    commands.insert_resource(handle);
}
```

Bevy의 의도된 패턴은 "sync 코드를 `AsyncComputeTaskPool.spawn(async { ... })`으로 감싸서 배경 워커에서 실행"하는 것이다. `MemoryRepository`가 async일 필요가 없다.

### 4.2 LLM 추론은 CPU/GPU-bound — async로 해결 불가

async가 효율적인 것은 I/O-bound 작업(네트워크 대기, 디스크 대기)이다. CPU가 "기다리는" 시간에 다른 작업을 끼워넣는 것이 async의 본질이기 때문이다.

```
  I/O-bound (async가 효과적):
  CPU:  [일]..........대기...........[일]
        0.01ms        수십~수백ms      0.01ms
        → 대기 시간에 다른 작업 가능
  
  CPU/GPU-bound (async가 무의미):
  GPU:  [토큰1][토큰2][토큰3]...[토큰200]
  CPU:  [대기][받기][대기][받기]...[대기][받기]
        ────────── 1500ms 연속 계산 ──────────
        → "빈 시간"이 없으므로 양보할 수 없음
```

gemma3:12b 추론은 500~3000ms 동안 GPU가 쉬지 않고 계산하는 작업이다. llama-cpp-rs API가 sync 블로킹(`generate(prompt) → String`)이므로, async로 감싸도 1.5초 동안 해당 스레드는 묶인다.

### 4.3 LanceDB 작업은 1~5ms — async 이득 없음

LanceDB의 벡터 검색, 삽입 작업은 1~5ms이다. 이 짧은 시간을 async로 양보해도 실질적 이득(다른 태스크에 CPU를 넘기는 효과)이 거의 없다.

### 4.4 block_on 런타임 중첩 패닉 — 발생하지 않음

초기 우려: Bevy의 async 태스크 풀 안에서 tokio `block_on`을 호출하면 "Cannot start a runtime from within a runtime" 패닉이 발생하지 않는가.

분석 결과: **발생하지 않는다.**

```
  ❌ 패닉하는 경우 — tokio 안에서 tokio
  tokio::Runtime::block_on(async {
      another_tokio_runtime.block_on(async { ... })  // 💥 PANIC
  });
  → tokio가 스레드-로컬 상태로 자기 자신의 중첩을 감지
  
  ✅ 안전한 경우 — Bevy 안에서 tokio
  bevy::AsyncComputeTaskPool.spawn(async {
      tokio_runtime.block_on(async {
          lancedb.query().await                       // ✅ 정상
      });
  });
  → Bevy는 async-executor 크레이트를 사용
  → tokio와 스레드-로컬 상태를 공유하지 않음
  → tokio의 중첩 감지가 Bevy 실행기를 인식하지 못함
```

### 4.5 스레드 풀 비용 — 실질적으로 제로

Bevy의 `AsyncComputeTaskPool`은 앱 시작 시 워커 스레드를 미리 생성(CPU 코어 수만큼)하고 앱 종료까지 유지한다. 게임 중에 새 스레드를 만들지 않으므로, "스레드 생성 비용"은 발생하지 않는다.

```
  앱 시작 시: 워커 4개 생성 (1회)
  게임 중:    태스크를 기존 워커에 배정 (비용 ≈ 0)
```

---

## 5. Phase 5~6(Bevy 통합) 시 적용 방법

MemoryRepository 트레이트 변경 없이, Bevy 시스템에서 태스크 풀로 감싸기만 하면 된다.

```rust
// Phase 5~6: Bevy 시스템 코드 (예상)
fn npc_think_system(
    task_pool: Res<AsyncComputeTaskPool>,
    query: Query<(Entity, &NpcBrain)>,
    repo: Res<Arc<RwLock<dyn MemoryRepository>>>,
    llm: Res<Arc<dyn LlmPort>>,
) {
    for (entity, brain) in query.iter() {
        let repo = repo.clone();
        let llm = llm.clone();
        
        let task = task_pool.spawn(async move {
            // 전부 sync — 워커 스레드에서 실행
            let memories = repo.read().unwrap().search(...);
            let prompt = build_prompt(&memories);
            let response = llm.generate(prompt);
            repo.write().unwrap().save(new_memory);
            response
        });
        
        commands.entity(entity).insert(PendingNpcReply(task));
    }
}

fn check_npc_replies_system(
    mut query: Query<(Entity, &mut PendingNpcReply)>,
) {
    for (entity, mut pending) in query.iter_mut() {
        if let Some(reply) = pending.0.try_recv() {
            show_dialogue(entity, &reply);
        }
    }
}
```

변경 범위 예측: MemoryRepository 트레이트 0건, LanceDbRepository 내부 0건, 새로 작성하는 Bevy 시스템 코드에서 `task_pool.spawn(async move { ... })` 감싸기만 추가.

---

## 6. 결정 요약

| 항목 | 결정 |
|------|------|
| MemoryRepository 트레이트 | sync 유지 |
| LanceDbRepository 내부 | 전용 tokio Runtime + block_on |
| InMemoryRepository | 변경 없음 |
| Bevy 통합 시 | AsyncComputeTaskPool.spawn()으로 감싸기 |
| async 전환 필요성 | 없음 (LLM은 CPU/GPU-bound, DB는 1~5ms) |
| 런타임 중첩 위험 | 없음 (Bevy async-executor ≠ tokio) |

---

## 7. 함께 확정된 결정 사항

Step 3.2.1 설계 논의에서 함께 확정된 두 가지 결정을 함께 기록한다.

### 7.1 Decision 2: 벡터 임베딩 책임 — LanceDbRepository가 EmbeddingPort 소유

LanceDbRepository가 내부에 `Box<dyn EmbeddingPort>`를 보유하고, `save()` 시 자동으로 content를 임베딩하여 벡터와 함께 저장한다. 호출자(ConversationService 등)는 임베딩의 존재를 모른다.

```
  호출자: repo.save(entry)
  내부:   ① entry.content → embedder.embed() → vector
          ② entry + vector → LanceDB 테이블에 저장
```

**근거**: 헥사고날 아키텍처에서 어댑터가 외부 인프라(임베딩 + DB)를 캡슐화하는 것이 원칙에 부합.

### 7.2 Decision 3: MVP 스키마 범위 — 현재 MemoryEntry 필드만

전략 문서의 13개 필드 중 현재 MemoryEntry에 존재하는 필드만으로 MVP 스키마를 구성한다.

```
  MVP 스키마 (Step 3.2.1):
  ─────────────────────────
  id              : UInt64          ← MemoryEntry.id
  character_id    : UInt64          ← MemoryEntry.character_id
  content         : Utf8            ← MemoryEntry.content
  vector          : Float32[1024]   ← EmbeddingPort로 자동 생성
  importance      : Float32         ← MemoryEntry.importance
  memory_type     : Utf8            ← MemoryEntry.memory_type (enum → string)
  game_year       : UInt32          ← GameTime.year
  game_month      : UInt32          ← GameTime.month
  game_day        : UInt32          ← GameTime.day
  keywords_json   : Utf8            ← serde_json::to_string(keywords)
```

emotional_intensity, participants, location 등은 향후 Phase에서 도메인 모델 확장 시 추가.

---

## 8. 변경 이력

| 버전 | 변경일시 | 변경 내역 |
|------|----------|-----------|
| v1.0.0 | 2026-02-20 22:30:00 | 최초 작성. Decision 1(sync + block_on), Decision 2(EmbeddingPort 소유), Decision 3(MVP 스키마) 확정 기록 |
