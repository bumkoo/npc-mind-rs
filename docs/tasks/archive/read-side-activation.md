# Task — Read Side 활성화 (Projection 실사용)

> **목적.** 현재 본 리포지토리는 `CommandDispatcher`·`EventStore`·`Projection` 까지 CQRS + Event Sourcing 인프라를 모두 갖추고 있으나, Mind Studio UI가 `StateInner`를 직접 조회하는 구조라 **Read Side의 실질 소비자가 없다**. 이 태스크는 **library core를 건드리지 않고** Mind Studio에 Projection 기반 조회 엔드포인트 3개를 추가하여 CQRS의 "Read Model이 외부를 섬긴다"는 약속을 실제로 작동시킨다.
>
> **범위.** `src/bin/mind-studio/` 이하만. `src/application/`, `src/domain/`, `src/adapter/`, `src/ports.rs`, `src/presentation/`, `src/lib.rs` **수정 금지**.
>
> **소요 예상.** 실 코드 약 120 LoC + 테스트 1~2개. 단계 도입 시 1단계만 먼저 진행 가능.

---

## 1. 배경 — 현재 상태 진단

검증된 사실:
- `EmotionProjection` / `RelationshipProjection` / `SceneProjection` 은 `src/application/projection.rs` 에 정의되어 있고, `EmotionProjectionHandler::from_shared(inner)` 등 **공유 Arc 생성자**가 이미 `src/application/command/projection_handlers.rs` 에 공개되어 있다.
- `dispatch_v2` Inline phase에서 위 핸들러들이 매번 실행되어 Projection이 최신 상태로 유지된다.
- 그러나 `EmotionProjection`을 참조하는 프로덕션 코드는 `src/application/` 내부(정의·등록)와 `tests/dispatch_v2_test.rs`뿐이며, **Mind Studio 어떤 경로에서도 조회되지 않는다**.
- Mind Studio UI 조회는 전부 `AppState.inner: StateInner`의 HashMap을 직접 읽는다.

결론: 인프라는 완성, **쓰기-읽기를 잇는 Query 엔드포인트만 빠져 있다**.

---

## 2. 목표

1. `AppState`가 3개 Projection의 공유 Arc 핸들을 보유한다.
2. `shared_dispatcher` 빌드 시 Projection 핸들러를 **AppState가 제공한 Arc로** 주입한다(from_shared 패턴).
3. HTTP GET 엔드포인트 3개를 `/api/projection/*` prefix로 신설한다.
4. "Write 경로 → Projection에 반영 → Read 경로가 같은 값을 돌려줌"을 보증하는 drift 감지 테스트 1개 이상 추가한다.

---

## 3. 완료 기준 (Definition of Done)

- [ ] `GET /api/projection/emotion/:npc_id` 가 해당 NPC의 mood/dominant/snapshot을 반환한다.
- [ ] `GET /api/projection/relationship/:owner/:target` 가 closeness/trust/power를 반환한다.
- [ ] `GET /api/projection/scene` 이 현재 활성 Scene의 active_focus_id와 is_active를 반환한다.
- [ ] `cargo test --workspace` 모든 기존 테스트 통과.
- [ ] 신규 drift 감지 테스트 1개 이상 추가·통과.
- [ ] `cargo clippy --workspace -- -D warnings` 경고 없음.
- [ ] 기존 Mind Studio UI의 CRUD·scenario load·appraise·stimulus·chat 경로 동작 변화 없음 (수동 smoke test 또는 기존 통합 테스트로 확인).


---

## 4. 전제 및 주의사항

### 4.1 절대 금지
- `src/application/`, `src/domain/`, `src/adapter/`, `src/ports.rs`, `src/presentation/`, `src/lib.rs` **어느 파일도 수정하지 않는다**.
- `StateInner` 제거·구조 변경 금지. 기존 UI CRUD 경로 그대로 둔다.
- `rebuild_repo_from_inner` 로직 건드리지 않는다.
- Projection을 UI가 쓰는 SSOT로 승격하지 않는다 — 어디까지나 **병렬 읽기 경로**다.

### 4.2 지켜야 할 원칙
- **자기 라벨링.** URL은 반드시 `/api/projection/*` prefix. 나중에 코드 리뷰어가 경로만 보고 "이건 Read Side" 라고 판별 가능해야 한다.
- **Arc 일치.** Dispatcher가 업데이트하는 Projection과 Query 핸들러가 읽는 Projection은 **같은 `Arc<Mutex<T>>`의 복제**여야 한다. 서로 다른 인스턴스면 읽기 값이 항상 비어 있다.
- **Mutex poisoning은 Infrastructure 에러로 매핑**. 기존 `EmotionProjectionHandler::handle`의 패턴을 그대로 따른다.

### 4.3 사전 확인 사항 (작업 시작 전)
다음 파일을 먼저 읽고 **현재 구현 세부를 파악**한 뒤 작업을 시작한다:

1. `src/application/command/dispatcher.rs` — `CommandDispatcher::with_default_handlers()`가 어떻게 Projection Handler를 등록하는가. `with_handler(Arc<dyn EventHandler>)` 같은 제네릭 확장 빌더가 이미 존재하는가.
2. `src/application/command/projection_handlers.rs` — `from_shared(inner)` 생성자의 정확한 시그니처.
3. `src/bin/mind-studio/state.rs` — `AppState::new`의 `shared_dispatcher` 빌드 블록 (embed feature 유무에 따라 분기가 두 벌임에 주의).
4. `src/bin/mind-studio/main.rs` 또는 router 구성 파일 — `.route(...)` 등록 위치.
5. `src/bin/mind-studio/handlers/mod.rs` — 핸들러 모듈 등록 패턴.

**만약** `CommandDispatcher`에 외부 Projection Handler를 주입할 수 있는 공개 빌더가 **없다면**, 이 태스크의 범위를 **벗어난다**. 이 경우 작업을 중단하고 본 문서 끝의 "Out of Scope 처리" 섹션을 참고해 별도 결정을 받는다.

---

## 5. 작업 명세

### 5.1 작업 1 — `AppState`에 Projection 공유 핸들 추가

**파일:** `src/bin/mind-studio/state.rs`

**변경:** `AppState` 구조체에 3개 필드 추가.

```rust
use std::sync::Mutex as StdMutex; // tokio::Mutex와 구분 필요 시 별칭
use npc_mind::application::projection::{
    EmotionProjection, RelationshipProjection, SceneProjection,
};

pub struct AppState {
    // ... 기존 필드 유지 ...
    
    /// Read Side — Projection 공유 핸들.
    /// `shared_dispatcher`의 Inline Projection Handler와 동일한 Arc를 공유한다.
    /// Query 핸들러(/api/projection/*)가 이 Arc를 lock하여 읽는다.
    pub emotion_projection: Arc<StdMutex<EmotionProjection>>,
    pub relationship_projection: Arc<StdMutex<RelationshipProjection>>,
    pub scene_projection: Arc<StdMutex<SceneProjection>>,
}
```

**주의:**
- `Mutex` 종류: 기존 `EmotionProjectionHandler::new()` 내부에서 사용하는 Mutex 타입에 맞춘다 (std의 `Mutex` 유력). `projection_handlers.rs`를 먼저 읽고 맞춰라.
- `lib.rs`에서 `EmotionProjection` 등이 이미 `pub use`로 노출되어 있는지 확인. 없다면 `npc_mind::application::projection::EmotionProjection`처럼 fully qualified path 사용.

---

### 5.2 작업 2 — Dispatcher 빌드 시 Projection 주입

**파일:** `src/bin/mind-studio/state.rs` — `AppState::new`

**변경:** 현재 `shared_dispatcher`를 만드는 두 블록(embed on/off)에서, **Projection Arc를 먼저 생성**하고 이를 **Dispatcher Builder에 주입**한 뒤, **AppState 필드에도 같은 Arc를 저장**한다.

```rust
impl AppState {
    pub fn new(
        collector: AppraisalCollector,
        analyzer: Option<impl UtteranceAnalyzer + Send + 'static>,
    ) -> Self {
        let (event_tx, _) = tokio::sync::broadcast::channel(64);

        // director_v2 빌드 (기존 그대로 유지)
        let director_v2 = { /* 기존 코드 */ };

        // ★ Projection Arc 선행 생성 — 이것들을 dispatcher와 AppState가 공유
        let emotion_projection = Arc::new(StdMutex::new(EmotionProjection::new()));
        let relationship_projection = Arc::new(StdMutex::new(RelationshipProjection::new()));
        let scene_projection = Arc::new(StdMutex::new(SceneProjection::new()));

        // ★ shared_dispatcher 빌드 시 from_shared 주입
        //   기존 with_default_handlers() 대신, projection만 수동 주입하고
        //   나머지 policy/handler는 기존 패턴 유지.
        //   정확한 빌더 API는 dispatcher.rs를 읽고 맞춤.
        #[cfg(feature = "embed")]
        let (shared_dispatcher, memory_store, rumor_store) = {
            // ... mem / rum 생성 (기존 그대로) ...
            
            let repo = InMemoryRepository::new();
            let store = Arc::new(InMemoryEventStore::new());
            let bus = Arc::new(EventBus::new());
            let dispatcher = Arc::new(
                CommandDispatcher::new(repo, store, bus)
                    .with_default_policies_only()  // ← 이름 예시. 실제 API는 dispatcher.rs 확인
                    .with_handler(Arc::new(
                        EmotionProjectionHandler::from_shared(emotion_projection.clone())
                    ))
                    .with_handler(Arc::new(
                        RelationshipProjectionHandler::from_shared(relationship_projection.clone())
                    ))
                    .with_handler(Arc::new(
                        SceneProjectionHandler::from_shared(scene_projection.clone())
                    ))
                    .with_memory_full(mem.clone())
                    .with_rumor(mem.clone(), rum.clone()),
            );
            (dispatcher, mem, rum)
        };

        #[cfg(not(feature = "embed"))]
        let shared_dispatcher = {
            // 동일한 패턴을 embed 비활성 분기에도 적용
            // ...
        };

        Self {
            // ... 기존 필드 ...
            emotion_projection,
            relationship_projection,
            scene_projection,
        }
    }
}
```

**중요 판단 포인트:**
- `with_default_handlers()`가 내부에서 Projection Handler를 **직접 생성**한다면, 그 메서드를 그대로 쓰되 우리 Arc는 **별도로** 전달하면 **중복 등록**이 된다. 이 경우:
  - (a) `with_default_handlers()` 대신 `with_default_policies_only()` 처럼 **policy만 등록**하는 변형 메서드를 `CommandDispatcher`에 추가해야 한다. 이는 library core 수정이므로 **본 태스크 범위 밖**.
  - (b) 또는 Mind Studio가 policy들을 하나씩 수동으로 with_handler 호출. 이 역시 dispatcher.rs의 공개 API에 달려 있음.
- **dispatcher.rs 를 먼저 읽고 판단**한다. 공개 빌더가 부족하면 본 태스크 중단 후 별도 결정.

---

### 5.3 작업 3 — Query 전용 핸들러 신설

**신규 파일:** `src/bin/mind-studio/handlers/query.rs`

```rust
//! Projection 기반 Read Side 엔드포인트.
//! 
//! `StateInner` 직접 조회 경로와 **병렬로** 존재하며, CQRS Read Model이
//! 실제로 외부 소비자를 가진다는 것을 입증하는 경로.
//! 
//! 모든 핸들러는 `AppState`가 보관한 Projection Arc에서 read-only lock을 획득한다.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;

use crate::handlers::AppError;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Emotion
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct EmotionView {
    pub npc_id: String,
    pub mood: Option<f32>,
    pub dominant: Option<(String, f32)>,
    pub snapshot: Option<Vec<(String, f32)>>,
}

pub async fn get_emotion(
    State(state): State<AppState>,
    Path(npc_id): Path<String>,
) -> Result<Json<EmotionView>, AppError> {
    let proj = state
        .emotion_projection
        .lock()
        .map_err(|_| AppError::Internal("emotion projection mutex poisoned".into()))?;

    Ok(Json(EmotionView {
        mood: proj.get_mood(&npc_id),
        dominant: proj.get_dominant(&npc_id).cloned(),
        snapshot: proj.get_snapshot(&npc_id).cloned(),
        npc_id,
    }))
}

// ---------------------------------------------------------------------------
// Relationship
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct RelationshipView {
    pub owner: String,
    pub target: String,
    pub closeness: Option<f32>,
    pub trust: Option<f32>,
    pub power: Option<f32>,
}

pub async fn get_relationship(
    State(state): State<AppState>,
    Path((owner, target)): Path<(String, String)>,
) -> Result<Json<RelationshipView>, AppError> {
    let proj = state
        .relationship_projection
        .lock()
        .map_err(|_| AppError::Internal("relationship projection mutex poisoned".into()))?;

    let values = proj.get_values(&owner, &target);
    Ok(Json(RelationshipView {
        owner,
        target,
        closeness: values.map(|v| v.0),
        trust: values.map(|v| v.1),
        power: values.map(|v| v.2),
    }))
}

// ---------------------------------------------------------------------------
// Scene
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct SceneView {
    pub is_active: bool,
    pub active_focus_id: Option<String>,
}

pub async fn get_scene(
    State(state): State<AppState>,
) -> Result<Json<SceneView>, AppError> {
    let proj = state
        .scene_projection
        .lock()
        .map_err(|_| AppError::Internal("scene projection mutex poisoned".into()))?;

    Ok(Json(SceneView {
        is_active: proj.is_active(),
        active_focus_id: proj.active_focus_id().map(String::from),
    }))
}
```

**모듈 등록:** `src/bin/mind-studio/handlers/mod.rs` 에 `pub mod query;` 한 줄 추가.

---

### 5.4 작업 4 — 라우터 등록

**파일:** `src/bin/mind-studio/main.rs` (또는 라우터 구성 파일)

**변경:** Router에 3개 route 추가.

```rust
use crate::handlers::query;

Router::new()
    // ... 기존 routes ...
    .route(
        "/api/projection/emotion/:npc_id",
        get(query::get_emotion),
    )
    .route(
        "/api/projection/relationship/:owner/:target",
        get(query::get_relationship),
    )
    .route(
        "/api/projection/scene",
        get(query::get_scene),
    )
    .with_state(state)
```

기존 `/api/*` 경로와 prefix가 다르므로 충돌 없음.

---

## 6. 테스트 요구사항

### 6.1 Drift 감지 테스트 (필수 — 최소 1개)

**파일:** `src/bin/mind-studio/` 내부 기존 테스트 파일 또는 신규 `handler_tests.rs`의 테스트 추가 (이미 존재). Mind Studio bin 전용이므로 `#[cfg(test)]` 모듈 안.

목적: **Write 경로로 상태를 바꾼 뒤, Read 경로(Projection)가 같은 값을 돌려준다**는 것을 보증.

테스트 시나리오 예시 (의사 코드):

```rust
#[tokio::test]
async fn projection_reflects_appraise_result() {
    // 1. AppState 및 테스트용 NPC/상황 준비
    let state = build_test_app_state().await;
    load_test_scenario(&state).await;

    // 2. Command 실행 (Write Side)
    let appraise_req = AppraiseRequest { /* ... */ };
    let response = StudioService::perform_appraise(&state, appraise_req).await.unwrap();
    let expected_mood = response.mood;
    let npc_id = response.npc_id.clone();

    // 3. Projection 조회 (Read Side) — 같은 npc_id, 같은 값이어야 함
    let proj = state.emotion_projection.lock().unwrap();
    let projection_mood = proj.get_mood(&npc_id).expect("projection must have mood after appraise");

    assert!((expected_mood - projection_mood).abs() < f32::EPSILON,
        "Write path reported mood={}, Projection returned mood={} — drift detected",
        expected_mood, projection_mood);
}
```

### 6.2 엔드포인트 smoke test (권장)

`axum::Router`를 직접 빌드해 `axum::test` 또는 `tower::ServiceExt::oneshot`으로 호출하여 상태코드·JSON 형태 검증.

### 6.3 회귀 확인 (필수)

`cargo test --workspace` 전체 통과. 특히:
- `tests/dispatch_v2_test.rs` — 기존 Projection 동작 검증
- `src/bin/mind-studio/handler_tests.rs` — 기존 Mind Studio 핸들러 테스트

---

## 7. 점진적 도입 순서 (권장)

한 번에 3개 엔드포인트를 모두 추가하지 말고 단계적으로:

### 1단계 — Emotion만 (최소 PR)
- 작업 1의 `emotion_projection` 필드만 추가
- 작업 2에서 `EmotionProjectionHandler::from_shared` 주입만 적용
- 작업 3에서 `get_emotion` 만 구현
- 작업 4에서 `/api/projection/emotion/:npc_id` 만 등록
- Drift 감지 테스트 1개 통과 확인

이 단계에서 **Dispatcher 빌더 API의 제약이 있는지 검증**된다. 막히면 여기서 멈추고 상의.

### 2단계 — Drift 감지 테스트 안착
- 1단계가 통과하면 drift 감지 테스트가 CI에서 안정적으로 도는지 확인
- 이게 CQRS 입증의 하이라이트

### 3단계 — Relationship, Scene 확장
- 같은 패턴 반복. 위험 없음.

### 4단계 — 문서 업데이트
- `docs/architecture/system-overview.md` §3 레이어 개관 박스에 `/api/projection/*` 엔드포인트 추가
- `CLAUDE.md` 구현 현황 표에 "Read Side Activation" 항목 추가
- README에 `/api/projection/*` 설명 한 문단

---

## 8. 체크리스트 (PR 올리기 전)

- [ ] `src/application/`, `src/domain/`, `src/adapter/`, `src/ports.rs`, `src/presentation/`, `src/lib.rs` 무변경 확인 (`git diff` 검사)
- [ ] `AppState`에 3개 Projection 필드 추가
- [ ] `shared_dispatcher` 빌드 시 Projection을 from_shared로 주입 (embed on/off 두 분기 모두)
- [ ] `handlers/query.rs` 신설 + `handlers/mod.rs`에 등록
- [ ] 라우터에 3개 `/api/projection/*` 엔드포인트 등록
- [ ] Drift 감지 테스트 1개 이상 통과
- [ ] `cargo test --workspace --all-features` 모두 통과
- [ ] `cargo test --workspace --no-default-features` 통과 (embed 비활성 시에도 동작)
- [ ] `cargo clippy --workspace --all-features -- -D warnings` 경고 없음
- [ ] 수동 smoke test: Mind Studio UI 기동 → NPC 생성 → 시나리오 로드 → Appraise 실행 → `curl http://localhost:PORT/api/projection/emotion/<npc_id>` 로 정상 JSON 수신
- [ ] 기존 Mind Studio UI CRUD·scenario load·appraise·chat 동작 변화 없음

---

## 9. 관련 파일 (작업 시 참조 경로)

| 역할 | 경로 |
|---|---|
| Projection 타입 정의 | `src/application/projection.rs` |
| Projection EventHandler 어댑터 | `src/application/command/projection_handlers.rs` |
| Dispatcher 빌더 | `src/application/command/dispatcher.rs` |
| Mind Studio AppState | `src/bin/mind-studio/state.rs` |
| 기존 핸들러 패턴 | `src/bin/mind-studio/handlers/npc.rs`, `handlers/mod.rs` |
| 라우터 구성 | `src/bin/mind-studio/main.rs` |
| 라이브러리 공개 export | `src/lib.rs` (읽기 전용 — 수정 금지) |

---

## 10. Out of Scope / 후속 작업

본 태스크에서 **하지 않는다**:

- `StateInner` 제거 또는 구조 변경.
- UI CRUD 경로를 Command enum 기반으로 승격.
- `MemoryStore`·`RumorStore` 기반 조회 엔드포인트 (별도 태스크: Step E의 Memory/Rumor UI 확장).
- SSE 기반 Projection 실시간 push (지금은 polling 전용).
- `director_v2` 경로의 Projection 노출 (v2는 shadow 상태 — 별도 태스크).

**만약 작업 2에서 Dispatcher 빌더 API가 부족해 library core 수정이 필요하다는 결론이 나오면**, 본 태스크를 중단하고 다음 별도 태스크로 분리:

- `CommandDispatcher`에 `with_handler(Arc<dyn EventHandler>)` 공개 빌더 추가
- `with_default_handlers()` 를 `with_default_policies()` + `with_default_projections()` 로 분할
- 이 변경은 library core 수정이므로 별도 ADR 또는 설계 검토 필요

---

## 11. 완료 후 후속 선언

본 태스크가 완료되면 `docs/architecture/system-overview.md` §3 레이어 개관의 "Application" 박스에 다음 줄을 추가한다:

```
Query Endpoints       /api/projection/emotion, /relationship, /scene
                      (Read Side — Projection 기반 조회, StateInner와 공존)
```

그리고 README에 **"CQRS + Event Sourcing + EventBus"** 문구가 단순 설계 의도가 아닌 **실행 증거를 가진 선언**이 된다.
