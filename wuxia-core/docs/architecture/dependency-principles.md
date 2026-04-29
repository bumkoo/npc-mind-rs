# 의존성 원칙 — 칠국춘추 프로젝트 설계 기준

**버전:** v1.1.0
**수정일:** 2026-02-23 03:15:00

---

## 1. 이 문서의 목적

코드를 작성하거나 리뷰할 때 "이 의존성이 올바른가?"를 판단하는 기준.
4가지 독립된 원칙이 함께 작동하여 프로젝트의 유연성과 단방향 의존성을 보장한다.

**관련 문서:**
- `architecture-decision.md` — 아키텍처 결정 기록 (crate 구조, 기술 스택)

---

## 2. 4가지 원칙

### 원칙 1: Port & Adapter (헥사고날 아키텍처)

**출처:** Alistair Cockburn, "Hexagonal Architecture" (2005)
**핵심 질문:** "도메인과 외부를 어떻게 분리하는가?"

도메인(wuxia-core)은 외부 기술을 직접 참조하지 않는다.
대신 trait(Port)으로 "이런 기능이 필요하다"만 선언한다.
외부 기술은 Adapter crate에서 그 trait을 구현한다.

```
  wuxia-core (도메인)
  ├── LlmPort (trait)                ← Port: "텍스트 생성이 필요하다"
  ├── MemoryRepository (trait)       ← Port: "기억 저장/검색이 필요하다"
  ├── RelationshipRepository (trait) ← Port: "관계 저장/조회가 필요하다"
  │
  ├── Relationship (struct)          ← 도메인 데이터
  ├── MemoryEntry (struct)           ← 도메인 데이터
  └── GameTime (struct)              ← 도메인 데이터

  wuxia-llm (Adapter)
  └── LlamaCppAdapter                ← impl LlmPort

  wuxia-memory (Adapter)
  ├── InMemoryRepository             ← impl MemoryRepository
  └── LanceDbRepository              ← impl MemoryRepository
```

**이 원칙이 방지하는 것:**
- core → adapter 방향의 의존 (도메인이 인프라를 아는 것)

**위반 예시:**
```rust
// ❌ wuxia-core 안에서 LanceDB를 직접 참조
use lancedb::Connection;

// ✅ wuxia-core는 trait만 정의
pub trait MemoryRepository: Send + Sync {
    fn search(&self, ...) -> Vec<ScoredMemory>;
}
```

---

### 원칙 2: 의존성 주입 (Dependency Injection)

**출처:** Martin Fowler, "Inversion of Control Containers and the Dependency Injection pattern" (2004)
**핵심 질문:** "의존성을 어떻게 넘기는가?"

UseCase(ChatSession 등)는 구현체를 직접 생성하지 않는다.
생성자 인자로 외부에서 받는다.

```
  ChatSession이 아는 것:              ChatSession이 모르는 것:
  ═════════════════════              ═════════════════════
  dyn LlmPort (trait)                LlamaCppAdapter
  dyn MemoryRepository (trait)       LanceDbRepository
  Relationship (struct)              InMemoryRepository
```

**이 원칙이 방지하는 것:**
- UseCase가 구현체를 직접 생성하여 특정 기술에 결합되는 것

**위반 예시:**
```rust
// ❌ UseCase 안에서 구현체를 직접 생성
impl ChatSession {
    fn new() -> Self {
        let repo = LanceDbRepository::new("./data");
    }
}

// ✅ 외부에서 주입받음
impl ChatSession {
    fn new(
        llm: Box<dyn LlmPort>,
        memory_repo: Box<dyn MemoryRepository>,
        relationship: Relationship,
    ) -> Self { ... }
}
```

---

### 원칙 3: Composition Root (조립 지점)

**출처:** Mark Seemann, "Dependency Injection in .NET" (2011)
**핵심 질문:** "구현체를 어디서 조립하는가?"

구현체 선택과 연결은 프로그램 진입점(main) 한 곳에서만 한다.
이 지점에서만 구체적인 Adapter 이름이 등장한다.

```
  soyeon_chat.rs (Composition Root)
  ═════════════════════════════════
  fn main() {
      // 여기서만 구현체 이름이 등장
      let llm = LlamaCppAdapter::new(...);
      let memory = LanceDbRepository::new(...);
      let relationship = Relationship::new(...);

      // UseCase에 주입
      let session = ChatSession::new(
          Box::new(llm),
          Box::new(memory),
          relationship,
      );
  }
```

**이 원칙이 방지하는 것:**
- 조립 코드가 여기저기 흩어져서 "어떤 구현체를 쓰는지" 추적 불가능한 상태

**위반 예시:**
```rust
// ❌ UseCase 계층에서 구현체를 import
// wuxia-app/src/chat_session.rs
use wuxia_memory::LanceDbRepository;

// ✅ UseCase는 trait만 import, 조립은 main()에서
// wuxia-app/src/chat_session.rs
use wuxia_core::memory::MemoryRepository;
```

---

### 원칙 4: 의존성 규칙 (Dependency Rule)

**출처:** Robert C. Martin, "The Clean Architecture" (2012)
**핵심 질문:** "의존 방향이 올바른가?"

소스코드 의존성은 항상 안쪽(상위 정책, 더 안정적인 계층)을 향해야 한다.
같은 계층의 crate끼리도 서로 참조할 수 없다.

```
  ┌──────────────────────────────────────────────┐
  │  바깥: soyeon_chat (binary, Composition Root) │
  │  ┌──────────────────────────────────────┐    │
  │  │  wuxia-llm, wuxia-memory (Adapter)    │    │
  │  │  ┌──────────────────────────────┐    │    │
  │  │  │  wuxia-app (UseCase)          │    │    │
  │  │  │  ┌──────────────────────┐    │    │    │
  │  │  │  │  wuxia-core (도메인)  │    │    │    │
  │  │  │  └──────────────────────┘    │    │    │
  │  │  └──────────────────────────────┘    │    │
  │  └──────────────────────────────────────┘    │
  └──────────────────────────────────────────────┘

  의존성 방향: 항상 안쪽으로만 →
```

**이 원칙이 방지하는 것 (원칙 1~3이 커버하지 못하는 빈 틈):**
- 같은 계층(어댑터끼리)의 상호 참조
- 역방향 의존 (안쪽 → 바깥)

```
  원칙 1~3이 방지하는 것:          원칙 4가 추가로 방지하는 것:
  ═══════════════════════         ═══════════════════════════
  core → adapter ❌               adapter ↔ adapter ❌
  app → adapter ❌                안쪽 → 바깥 전부 ❌
  UseCase 내 구현체 생성 ❌
  조립 코드 분산 ❌
```

**위반 예시:**
```rust
// ❌ 어댑터가 다른 어댑터를 참조
// wuxia-llm/Cargo.toml
[dependencies]
wuxia-memory = { path = "../wuxia-memory" }  # 어댑터 → 어댑터

// ❌ UseCase가 어댑터를 참조
// wuxia-app/Cargo.toml
[dependencies]
wuxia-llm = { path = "../wuxia-llm" }  # UseCase → 어댑터

// ✅ 각 crate는 wuxia-core만 참조
// wuxia-llm/Cargo.toml
[dependencies]
wuxia-core = { path = "../wuxia-core" }

// wuxia-app/Cargo.toml
[dependencies]
wuxia-core = { path = "../wuxia-core" }
```

---

## 3. 원칙 간 관계

```
  원칙 1 (Port & Adapter)     → "인터페이스를 분리해라"
       │
       ▼
  원칙 2 (DI)                → "그 인터페이스를 생성자로 받아라"
       │
       ▼
  원칙 3 (Composition Root)  → "실제 연결은 main()에서 해라"
       │
       ▼
  원칙 4 (Dependency Rule)   → "의존은 항상 안쪽으로만, 같은 계층도 금지"
```

4개가 함께 작동해야 단방향 의존성이 보장된다.

```
  Port만 있고 DI가 없으면:
  → UseCase가 구현체를 직접 만들어서 Port 의미 없음

  DI만 있고 Composition Root가 없으면:
  → 조립 코드가 여기저기 흩어져서 관리 불가

  1~3만 있고 Dependency Rule이 없으면:
  → 어댑터끼리 참조해도 막을 근거가 없음
```

---

## 4. 프로젝트 적용 — 계층별 규칙

### 4.1 의존성 허용 매트릭스

| from ＼ to | wuxia-core | wuxia-app | wuxia-llm | wuxia-memory | wuxia-data |
|------------|:----------:|:---------:|:---------:|:------------:|:----------:|
| **wuxia-core** | - | ❌ | ❌ | ❌ | ❌ |
| **wuxia-app** (UseCase) | ✅ | - | ❌ | ❌ | ❌ |
| **wuxia-llm** (Adapter) | ✅ | ❌ | - | ❌ | ❌ |
| **wuxia-memory** (Adapter) | ✅ | ❌ | ❌ | - | ❌ |
| **wuxia-data** (Adapter) | ✅ | ❌ | ❌ | ❌ | - |
| **wuxia-game** (Bevy) | ✅ | ✅ | ❌ | ❌ | ✅ |
| **binary** (Comp. Root) | ✅ | ✅ | ✅ | ✅ | ✅ |

핵심 규칙:
- 어댑터끼리는 서로 모른다 (같은 행에서 다른 어댑터 열은 모두 ❌)
- UseCase(app)는 core만 안다
- binary(Composition Root)만 모든 crate를 참조할 수 있다

### 4.2 각 계층이 할 수 있는 것 / 할 수 없는 것

```
  wuxia-core (도메인):
    ✅ struct, enum, trait 정의
    ✅ 순수 함수 (테스트에 외부 의존 없음)
    ✅ Application Service (UseCase) — 도메인 내 서비스 조합
    ❌ use lancedb, use llama_cpp, use bevy

  wuxia-llm / wuxia-memory / wuxia-data (어댑터):
    ✅ wuxia-core의 trait 구현 (impl LlmPort, impl MemoryRepository)
    ✅ 외부 라이브러리 사용 (llama-cpp-2, lancedb, toml)
    ✅ 테스트용 Mock 구현 (MockLlm, InMemoryRepository)
    ❌ 다른 어댑터 crate 참조
    ❌ UseCase 로직 포함 (여러 Port를 조합하는 흐름)

  wuxia-app (UseCase):
    ✅ wuxia-core의 trait과 struct 사용
    ✅ 여러 Port를 조합하여 비즈니스 흐름 구성
    ❌ 구현체(LanceDb, LlamaCpp) 직접 참조
    ❌ 외부 라이브러리 직접 사용

  wuxia-game (Bevy 엔진 계층):
    ✅ wuxia-core, wuxia-app 참조
    ✅ Bevy Plugin/Component/System 정의
    ✅ wuxia-data 참조 (에셋 로딩)
    ❌ 어댑터(llm, memory) 직접 참조 — Resource로 주입받음

  soyeon_chat / main.rs (Composition Root):
    ✅ 모든 crate 참조 가능
    ✅ 구현체 생성 + UseCase에 주입
    ❌ 비즈니스 로직 직접 구현
```

### 4.3 의존성 방향 그래프

```
                    wuxia-core
                   (순수 도메인)
                   ⊘ 외부 의존
                        │
          ┌─────────────┼───────────────┐
          │             │               │
          ▼             ▼               ▼
    wuxia-llm     wuxia-memory     wuxia-data
    (llama-cpp)   (LanceDB)        (toml/json)
    ⊘ 서로 모름   ⊘ 서로 모름      ⊘ 서로 모름
          │             │               │
          │       wuxia-app             │
          │      (UseCase)              │
          │     core만 참조             │
          │        │                    │
          └────────┼────────────────────┘
                   │
              wuxia-game
             (Bevy Plugin)
                   │
                   ▼
           soyeon_chat / main
          (Composition Root)
          (모든 crate 조립)
```

화살표는 항상 위에서 아래로만. 역방향 의존 금지.
같은 높이의 crate끼리 가로 화살표 금지.

---

## 5. 설계 판단 기록

### 5.1 ChatSession 위치 — wuxia-llm 유지 (위반 아님)

```
  위치: wuxia-llm/src/conversation/session.rs

  ChatSession은 LlmPort + MemoryRepository + Relationship을 조합한다.
  → 일견 UseCase(응용 서비스) 역할처럼 보인다.
  → 그러나 ChatSession이 참조하는 것은 모두 wuxia-core의 trait/struct이다:

     use wuxia_core::llm::LlmPort;              ← trait
     use wuxia_core::memory::MemoryRepository;   ← trait
     use wuxia_core::relationship::Relationship; ← struct

  → LlamaCppAdapter, LanceDbRepository 등 구현체를 직접 참조하지 않는다.
  → wuxia-llm → wuxia-core 의존만 존재. 다른 어댑터 crate를 참조하지 않음.
  → 원칙 1~4 모두 충족. 이동 불필요.

  만약 이동한다면:
    wuxia-llm에 LlamaCppAdapter 1개만 남아 crate 존재 의미가 사라진다.
    wuxia-app이 conversation + prompt + parser + mock을 모두 포함하게 되어
    과도한 분리(over-engineering)에 해당한다.

  결론: ChatSession은 wuxia-llm에 유지한다.
  binary(Composition Root)에서 구현체를 생성하여 ChatSession 생성자에 주입한다.
```

```
  wuxia-llm 의존성 검증:
  ═══════════════════════
  wuxia-llm → wuxia-core ✅  (trait/struct만 사용)
  wuxia-llm → wuxia-memory ❌ (참조 없음)
  wuxia-llm → wuxia-app ❌   (역방향 없음)

  원칙 1 (Port & Adapter):  ChatSession은 trait만 사용 ✅
  원칙 2 (DI):              MemoryRepository, Relationship을 생성자로 주입 ✅
  원칙 3 (Composition Root): soyeon_chat.rs에서 구현체 조립 ✅
  원칙 4 (Dependency Rule):  wuxia-llm → wuxia-core 단방향만 ✅
```

---

## 6. 체크리스트 — 코드 작성/리뷰 시

새 코드를 작성하거나 리뷰할 때 아래 질문에 모두 "아니오"여야 한다:

```
  원칙 1 (Port & Adapter):
    □ 도메인 crate(wuxia-core)가 외부 라이브러리를 import하고 있는가?
    □ trait 없이 구현체를 직접 참조하고 있는가?

  원칙 2 (DI):
    □ UseCase가 구현체를 직접 생성(new)하고 있는가?
    □ 생성자 대신 내부에서 의존성을 만들고 있는가?

  원칙 3 (Composition Root):
    □ main() 바깥에서 Adapter를 선택하는 코드가 있는가?
    □ 조립 로직이 여러 파일에 분산되어 있는가?

  원칙 4 (Dependency Rule):
    □ 어댑터 crate가 다른 어댑터 crate를 import하고 있는가?
    □ 안쪽 계층이 바깥 계층을 참조하고 있는가?
    □ Cargo.toml의 [dependencies]에 같은 계층 crate가 있는가?
```

하나라도 "예"이면 원칙 위반이다. 설계를 재검토해야 한다.

---

## 변경 이력

| 버전 | 변경일시 | 변경 내역 |
|------|----------|-----------|
| v1.0.0 | 2026-02-23 02:30:00 | 초기 작성. 4가지 원칙 정의 — (1) Port & Adapter (Cockburn 2005), (2) 의존성 주입 (Fowler 2004), (3) Composition Root (Seemann 2011), (4) 의존성 규칙 (Martin 2012). 의존성 허용 매트릭스. 계층별 허용/금지 규칙. 의존성 방향 그래프. ChatSession 위치 위반 기록. 코드 작성/리뷰 체크리스트. |
| v1.1.0 | 2026-02-23 03:15:00 | 섹션 5 수정: ChatSession 위치 판단 변경. wuxia-app 이동 계획 철회 → wuxia-llm 유지. ChatSession이 wuxia-core trait/struct만 참조하므로 원칙 위반 아님 확인. 과도한 분리(over-engineering) 방지. |
