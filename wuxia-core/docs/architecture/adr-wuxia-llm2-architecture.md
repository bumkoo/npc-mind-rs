# ADR: wuxia-llm2 아키텍처 — AI 미들웨어 재설계

**버전:** v1.0.0
**작성일:** 2026-03-12T21:00:00+09:00
**상태:** 설계 확정, 구현 전

**관련 문서:**
- `architecture-decision.md` — 전체 crate 구조
- `dependency-principles.md` — 4가지 의존성 원칙
- `domain-analysis.md` — 12개 도메인 분석
- `memory-domain-analysis.md` — 기억 도메인 상세
- `adr-async-sync-decision.md` — MemoryRepository Async/Sync 전략

---

## 1. 문제

현재 AI 관련 crate 구조에 3가지 근본적 문제가 있다.

### 1.1 비즈니스 로직과 인프라의 혼재

```
  wuxia-llm (현재)
  ├── conversation/session.rs    ← 비즈니스 로직 (세션 관리)
  ├── conversation/context.rs    ← 비즈니스 로직 (컨텍스트 공급)
  ├── conversation/manager.rs    ← 비즈니스 로직 (대화 압축)
  ├── prompt/template.rs         ← 비즈니스 로직 (프롬프트 조립)
  ├── sentiment/pipeline.rs      ← 비즈니스 로직 (감정 판정)
  ├── parser/response.rs         ← 비즈니스 로직 (응답 파싱)
  ├── adapter/llama_cpp.rs       ← 인프라 (유일한 진짜 어댑터)
  └── mock.rs                    ← 테스트용
```

"어댑터" crate인데 90%가 비즈니스 로직이다. LlamaCppAdapter 하나만이 진짜 인프라 코드이다.

### 1.2 core가 AI 구현 세부사항을 알아야 함

```
  현재 core에 정의된 저수준 Port:
  - LlmPort            → "텍스트 생성해줘"
  - MemoryRepository    → "벡터 검색해줘"
  - EmbeddingPort       → "임베딩 변환해줘"

  core의 도메인 서비스가 이 3개 Port를 직접 조합:
  recall_memories(repo, query, embedder, ...) → Vec<RankedMemory>

  → core가 LLM, 벡터DB, 임베딩의 존재를 알고 있다
  → 새 AI 기능 추가 시 core에 새 Port를 추가해야 한다
```

### 1.3 인프라 간 격리 부재

```
  wuxia-llm:     llama-cpp-2 의존 (추론용)
  wuxia-memory:  llama-cpp-2 의존 (임베딩용) + lancedb 의존

  같은 llama-cpp-2 라이브러리를 2개 crate에서 각각 초기화.
  인프라 교체 시 여러 crate를 동시 수정해야 한다.
```

---

## 2. 결정

**wuxia-llm2를 AI 미들웨어로 신규 설계한다.** core는 고수준 업무 인터페이스로만 AI와 소통하며, AI 내부 구현(프롬프트 조립, 기억 검색, LLM 호출, 임베딩, 벡터DB)은 llm2와 독립 infra crate들의 책임이다.

### 2.1 핵심 원칙

```
  ① core는 "소연과 대화해줘" 수준의 고수준 요청만 한다
  ② 프롬프트 조립, 세션 관리, 기억 검색은 llm2의 비즈니스 로직(task)이다
  ③ 추론/임베딩/벡터DB는 각각 독립 crate로 분리된 인프라이다
  ④ llm2 자체는 외부 라이브러리 의존이 없는 순수 비즈니스 crate이다
  ⑤ 모든 인프라는 복수 task를 동시 서빙한다
```

---

## 3. 설계 결정 상세

### 3.1 결정 1 — 고수준 trait 위치: wuxia-core

**선택:** 방안 A — core에 trait 정의 (헥사고널 원칙 유지)

```
  wuxia-core
  └── pub trait ConversationAi { ... }    ← core가 인터페이스 소유
  └── pub trait MemoryAi { ... }
  └── pub trait PsychologyAi { ... }

  wuxia-llm2
  └── impl ConversationAi for Llm2Service  ← llm2가 구현

  의존 방향: llm2 → core (정상)
```

**기각된 대안:**
- 방안 B (llm2에 trait 정의): core → llm2 의존이 발생하여 헥사고널 원칙 위반
- 방안 C (별도 port crate): 1인 개발에서 crate 관리 부담 과다

**근거:** trait의 메서드 시그니처가 도메인 언어로 표현되므로 ("대화해줘", "기억시켜줘") core에 있는 것이 자연스럽다. AI 구현 세부사항(temperature, top_k 등)은 노출하지 않는다.

### 3.2 결정 2 — 업무 단위 분리 trait (ISP 적용)

**선택:** 방식 2 — 업무 단위로 trait 분리

```
  pub trait ConversationAi: Send + Sync {
      fn start_chat(&self, npc_id: CharacterId, ctx: ChatContext) -> SessionId;
      fn chat(&self, session: SessionId, input: &str) -> ChatResult;
      fn end_chat(&self, session: SessionId) -> EndChatResult;
  }

  pub trait MemoryAi: Send + Sync {
      fn memorize(&self, npc_id: CharacterId, exp: Experience) -> ();
      fn recall(&self, npc_id: CharacterId, query: &str) -> Vec<Memory>;
  }

  pub trait PsychologyAi: Send + Sync {
      fn judge_emotion(&self, npc_id: CharacterId, ctx: EmotionContext) -> Judgment;
      fn reflect(&self, npc_id: CharacterId) -> Vec<Reflection>;
  }
```

**기각된 대안:**
- 방식 1 (단일 AiService trait): 메서드 증가 시 비대화, 기억만 필요한 곳에서 전체 trait 주입 필요

**근거:** 인터페이스 분리 원칙(ISP). 각 도메인이 필요한 trait만 의존. 기존 저수준 Port 3개와 동일한 수이지만 추상화 수준이 완전히 다르다.

```
  기존 Port (저수준):                   신규 trait (고수준):
  LlmPort ("텍스트 생성해줘")          ConversationAi ("대화해줘")
  MemoryRepository ("벡터 검색해줘")   MemoryAi ("기억시켜줘")
  EmbeddingPort ("임베딩 변환해줘")    PsychologyAi ("감정 판정해줘")
```

### 3.3 결정 3 — 세션 기반 ConversationAi

**선택:** 방안 B — 세션 기반 (start_chat → chat → end_chat)

```
  core 서사 시스템                     wuxia-llm2
  ───────────────                     ──────────────

  ① ChatContext 조립 (NPC 정보 뭉치)
     → start_chat(npc_id, ctx)       → 세션 생성, ctx 캐시
                                      ← SessionId 반환
  ② 플레이어 발화
     → chat(session_id, input)       → 기억 검색 → 프롬프트 → LLM → 파싱
                                      ← ChatResult { npc_response, affinity_delta, ... }
  ③ 대화 종료
     → end_chat(session_id)          → 요약 생성, 기억 저장
                                      ← EndChatResult { summary, memories_created, ... }
```

**core가 start_chat에 넘기는 ChatContext:**

```
  pub struct ChatContext {
      pub character: CharacterSnapshot,   // 이름, 나이, 역할, 소속
      pub personality: PersonalityView,   // HEXACO, 가치관 (읽기 전용)
      pub relationship: RelationshipView, // 호감도, 신뢰도, 레벨
      pub speech_rules: SpeechRules,      // 말투, 금기어, 대사 예시
      pub locale: Locale,                 // 언어 설정
  }
```

**기각된 대안:**
- 방안 A (매번 전부 넘김): 파라미터 폭발, "고수준 인터페이스"의 취지에 반함
- 방안 C (llm2가 NPC 데이터 직접 조회): llm2 → core 역방향 의존, 순환 위험

### 3.4 결정 4 — 저수준 Port를 llm2 내부로 이동 (시나리오 C)

**선택:** 시나리오 C — 계층 분리

```
  core에서 제거:
    ❌ LlmPort
    ❌ MemoryRepository
    ❌ EmbeddingPort
    ❌ recall_memories() (오케스트레이션 함수)

  core에 유지:
    ✅ MemoryEntry, ScoredMemory, RankedMemory 등 데이터 타입
    ✅ retrieval_score() — 4축 검색 점수 (순수 수학, 게임 디자이너 규칙)
    ✅ rank_memories() — 기억 랭킹 (순수 정렬, 게임 디자이너 규칙)
    ✅ MemoryEvent 도메인 이벤트

  llm2 내부에 자체 Port:
    InferenceLlm trait   (구 LlmPort)
    Embedder trait        (구 EmbeddingPort)
    VectorStore trait     (구 MemoryRepository)

  llm2 task로 이동:
    recall_memories()    (Port 의존 오케스트레이션)
```

**기각된 대안:**
- 시나리오 A (전면 교체): 빅뱅 마이그레이션 위험
- 시나리오 B (공존): 두 경로 유지 부담, 과도기 복잡도

**순수 도메인 함수 귀속 기준:**

```
  "이 로직이 AI 인프라 없이도 의미가 있는가?"

  retrieval_score():  ✅ 순수 수학 공식, 게임 디자이너 조정 → core
  rank_memories():    ✅ 정렬 + 필터, 게임 디자이너 조정 → core
  recall_memories():  ❌ embed + search 필요, Port 없이 동작 불가 → llm2 task

  요약: 도메인 규칙(What)은 core, 실행 방법(How)은 llm2
```

### 3.5 결정 5 — Infra 각각 독립 crate

**선택:** 각 인프라를 독립 crate로 분리

```
  wuxia-inference    추론 LLM    llama-cpp-2 의존
  wuxia-embedder     임베딩      llama-cpp-2/bge 의존
  wuxia-vectordb     벡터 DB     lancedb 의존
```

**장점:**
- 인프라 교체가 crate 단위로 깔끔 (lancedb → qdrant = wuxia-vectordb만 교체)
- 컴파일 격리 (lancedb 빌드 오류가 inference에 영향 없음)
- feature flag 불필요 (crate 자체를 넣거나 빼면 됨)
- llm2 crate가 외부 라이브러리 의존 없는 순수 비즈니스 crate가 됨
- "모든 인프라는 복수 task를 동시 서빙" 원칙이 자연스러움

**단점:**
- crate 수 증가 (5 → 7개)
- 1인 개발자 관리 부담 (수용 가능한 수준으로 판단)
- trait 변경 시 여러 crate 동시 수정 (infra port는 안정적이므로 빈도 낮음)

### 3.6 결정 6 — 비즈니스 레이어 명칭: task

**선택:** `task`

```
  "대화 task", "성찰 task"
  → AI에게 시키는 작업 단위라는 의미
  → 짧고 직관적
  → 향후: "감정판정 task", "기억저장 task"
```

**기각된 대안:**
- `biz`: 게임 도메인에 안 어울림
- `pipeline`: 적절하나 task보다 길고 데이터 흐름에 치우친 느낌
- `workflow`: 엔터프라이즈 느낌
- `domain`: 기존 core domain과 혼동

### 3.7 결정 7 — 초기 scope: conversation + reflection

```
  task/conversation/   ← NPC 대화 (11섹션 프롬프트, 세션, 응답 파싱, 감정 판정)
  task/reflection/     ← 성찰 + 요약 (성찰 프롬프트, 기억 저장)

  감정 판정: conversation task 내부에 포함 (나중에 별도 task로 승격 가능)
  기억 검색: 두 task가 공유 (각 task가 port를 직접 사용)
```

### 3.8 결정 8 — facade 역할: 얇은 위임

**선택:** 관점 A — task가 port를 직접 보유, facade는 얇은 위임

```
  ConversationTask {
      inference: Arc<dyn InferenceLlm>,
      embedder: Arc<dyn Embedder>,
      store: Arc<dyn VectorStore>,
      sessions: HashMap<SessionId, Session>,
  }

  Llm2Service (facade) {
      conversation: ConversationTask,
      reflection: ReflectionTask,
  }

  impl ConversationAi for Llm2Service {
      fn chat(&self, s, i) { self.conversation.chat(s, i) }  // 위임
  }
```

**인프라 공유 방식:**

```
  // Composition Root (wuxia-app)
  let inference = Arc::new(LlamaCppInference::new(...));
  let embedder = Arc::new(BgeM3Embedder::new(...));
  let store = Arc::new(LanceDbStore::new(...));

  let conversation = ConversationTask::new(
      inference.clone(), embedder.clone(), store.clone(),
  );
  let reflection = ReflectionTask::new(
      inference.clone(), embedder.clone(), store.clone(),
  );
  let llm2 = Llm2Service::new(conversation, reflection);

  → 인프라 인스턴스 1개씩, 모든 task가 Arc로 공유
```

**나중에 감정 판정 분리 시:**
- ConversationTask 내부의 감정판정 로직을 SentimentChecker 구조체로 추출
- ConversationTask가 SentimentChecker를 소유 (facade 변경 없음)
- 또는 별도 SentimentTask로 승격 (facade에 추가)

### 3.9 결정 9 — 기존 crate 처리: 삭제 후 코드 이동

```
  wuxia-llm (삭제):
  ├── conversation/*     → wuxia-llm2/task/conversation/
  ├── prompt/*           → wuxia-llm2/task/conversation/ + common/
  ├── sentiment/*        → wuxia-llm2/task/conversation/ (내부)
  ├── parser/*           → wuxia-llm2/common/
  ├── adapter/llama_cpp  → wuxia-inference/
  └── mock.rs            → wuxia-llm2/port/ (Mock)

  wuxia-memory (삭제):
  ├── lancedb/           → wuxia-vectordb/
  ├── embedding/llama_cpp→ wuxia-embedder/
  ├── embedding/mock     → wuxia-llm2/port/ (Mock)
  ├── in_memory          → wuxia-vectordb/ 또는 llm2 port/ (Mock)
  └── config             → wuxia-embedder/ 또는 llm2/config
```

---

## 4. 전체 아키텍처

### 4.1 crate 구성 (7개)

```
  기존 (5 crate):              신규 (7 crate):
  ─────────────                ─────────────
  wuxia-core                   wuxia-core          (유지, trait 교체)
  wuxia-llm        ──삭제──→   wuxia-llm2          (신규, task+port+common)
  wuxia-memory     ──삭제──→   wuxia-inference      (신규, 추론 LLM)
  wuxia-data                   wuxia-embedder       (신규, 임베딩)
  wuxia-app                    wuxia-vectordb       (신규, 벡터DB)
                               wuxia-data           (유지)
                               wuxia-app            (유지, 조립 단순화)
```

### 4.2 의존성 방향 그래프

```
                     wuxia-core
                    (도메인 순수)
                    ⊘ 외부 의존
                         │
              ┌──────────┤
              │          │
              ▼          ▼
         wuxia-llm2   wuxia-data
         (task+port)  (toml/json)
         ⊘ 외부 의존  ⊘ 외부 의존
              │
     ┌────────┼─────────┐
     │        │         │
     ▼        ▼         ▼
  wuxia-     wuxia-    wuxia-
  inference  embedder  vectordb
  (llama-cpp)(llama-cpp)(lancedb)
  ⊘ 서로모름 ⊘ 서로모름 ⊘ 서로모름
     │        │         │
     └────────┼─────────┘
              │
         wuxia-game
        (Bevy Plugin)
              │
              ▼
          wuxia-app
       (Composition Root)
```

### 4.3 의존성 허용 매트릭스

| from ＼ to | core | llm2 | inference | embedder | vectordb | data | game | app |
|------------|:----:|:----:|:---------:|:--------:|:--------:|:----:|:----:|:---:|
| **core** | - | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **llm2** | ✅ | - | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **inference** | ❌ | ✅(port) | - | ❌ | ❌ | ❌ | ❌ | ❌ |
| **embedder** | ❌ | ✅(port) | ❌ | - | ❌ | ❌ | ❌ | ❌ |
| **vectordb** | ❌ | ✅(port) | ❌ | ❌ | - | ❌ | ❌ | ❌ |
| **data** | ✅ | ❌ | ❌ | ❌ | ❌ | - | ❌ | ❌ |
| **game** | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ | - | ❌ |
| **app** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | - |

핵심 규칙:
- infra crate들은 llm2의 port trait만 참조 (core를 직접 참조하지 않음)
- infra crate끼리는 서로 모름
- llm2는 core만 참조 (infra를 직접 참조하지 않음)
- app(Composition Root)만 모든 crate를 참조

### 4.4 wuxia-llm2 내부 모듈 구조

```
  wuxia-llm2/
  │
  ├── src/
  │   ├── lib.rs                    ← pub 모듈 선언 + Llm2Service re-export
  │   │
  │   ├── facade.rs                 ← Llm2Service (진입점)
  │   │                                impl ConversationAi (위임)
  │   │                                impl MemoryAi (위임)
  │   │
  │   ├── common/                   ← task 공통
  │   │   ├── mod.rs
  │   │   ├── message.rs            ← ChatMessage, Role (OpenAI 형식 통일)
  │   │   ├── prompt_builder.rs     ← 프롬프트 조립 유틸리티
  │   │   └── parser.rs             ← 공통 응답 파싱 유틸리티
  │   │
  │   ├── task/                     ← Task Layer
  │   │   ├── mod.rs
  │   │   │
  │   │   ├── conversation/         ← 대화 task
  │   │   │   ├── mod.rs
  │   │   │   ├── task.rs           ← ConversationTask (세션 관리, 전체 흐름)
  │   │   │   ├── prompt.rs         ← 11섹션 프롬프트 조립
  │   │   │   ├── response.rs       ← NPC 응답 파싱
  │   │   │   ├── compress.rs       ← 대화 압축/요약
  │   │   │   └── sentiment.rs      ← 감정 판정 (극단앵커 + 정기판정)
  │   │   │
  │   │   └── reflection/           ← 성찰 task
  │   │       ├── mod.rs
  │   │       ├── task.rs           ← ReflectionTask (성찰 전체 흐름)
  │   │       ├── prompt.rs         ← 성찰 프롬프트 조립
  │   │       └── recall.rs         ← recall_memories (core에서 이동)
  │   │
  │   ├── port/                     ← llm2 내부 Port (trait 정의)
  │   │   ├── mod.rs
  │   │   ├── inference.rs          ← InferenceLlm trait
  │   │   ├── embedder.rs           ← Embedder trait
  │   │   └── vector_store.rs       ← VectorStore trait
  │   │
  │   └── config.rs                 ← Llm2Config (전체 설정)
  │
  └── Cargo.toml                    ← 의존: wuxia-core만
```

### 4.5 llm2 내부 의존 방향

```
  facade.rs
     │ 위임
     ▼
  task/ (conversation, reflection)
     │         │
     │         ▼
     │      common/ (ChatMessage, PromptBuilder)
     │
     ▼
  port/ (InferenceLlm, Embedder, VectorStore trait)
     ▲
     │
  [외부 infra crate들이 구현]

  + task/ ──import──→ wuxia-core (retrieval_score, MemoryEntry 등)

  규칙: 항상 위에서 아래로만
```

### 4.6 ChatMessage 형식 통일

```
  llm2 내부 메시지 프로토콜 (OpenAI 호환)
  ═══════════════════════════════════════

  enum Role { System, User, Assistant }

  struct ChatMessage {
      role: Role,
      content: String,
  }

  모든 task는 이 형식으로 프롬프트를 만든다.
  모든 infra는 이 형식을 입력으로 받는다.

  대화 task:  [System(11섹션), User(player_input)]  → InferenceLlm
  감정 판정:  [System(판정지시), User(대화이력)]      → InferenceLlm
  성찰 task:  [System(성찰지시), User(기억목록)]      → InferenceLlm

  → Infra는 "누가 보냈는지" 몰라도 됨
  → task 추가 시 Infra 수정 불필요
```

---

## 5. 기존 코드 대비 변화 요약

### 5.1 core 변경

| 항목 | 현재 | 변경 후 |
|------|------|---------|
| LlmPort trait | core/llm/ | **제거** (llm2 내부 InferenceLlm으로) |
| MemoryRepository trait | core/memory/port.rs | **제거** (llm2 내부 VectorStore로) |
| EmbeddingPort trait | core/memory/embedding.rs | **제거** (llm2 내부 Embedder로) |
| recall_memories() | core/memory/recall.rs | **llm2 task로 이동** |
| retrieval_score() | core/memory/retrieval.rs | **유지** (순수 도메인 규칙) |
| rank_memories() | core/memory/retrieval.rs | **유지** (순수 도메인 규칙) |
| MemoryEntry 등 타입 | core/memory/types.rs | **유지** |
| ConversationAi trait | (없음) | **신규 추가** |
| MemoryAi trait | (없음) | **신규 추가** |
| PsychologyAi trait | (없음) | **향후 추가** |
| ChatContext 등 타입 | (없음) | **신규 추가** |

### 5.2 crate 생멸

| crate | 상태 | 비고 |
|-------|------|------|
| wuxia-core | 유지 | 저수준 Port 제거, 고수준 trait 추가 |
| wuxia-llm | **삭제** | 코드를 llm2 + inference로 이동 |
| wuxia-memory | **삭제** | 코드를 llm2 + embedder + vectordb로 이동 |
| wuxia-llm2 | **신규** | task + port + common (순수 비즈니스) |
| wuxia-inference | **신규** | 추론 LLM (llama-cpp-2) |
| wuxia-embedder | **신규** | 임베딩 (llama-cpp-2/bge) |
| wuxia-vectordb | **신규** | 벡터DB (lancedb) |
| wuxia-data | 유지 | 변경 없음 |
| wuxia-app | 유지 | 조립 단순화 |

---

## 6. 미결 사항

| # | 항목 | 상태 | 비고 |
|---|------|------|------|
| 1 | 마이그레이션 전략 | 미정 | 기존 820+ 테스트 이동 방법 |
| 2 | PsychologyAi trait 상세 | 미정 | 심리 도메인 구현 시 설계 |
| 3 | MemoryAi trait 상세 메서드 | 미정 | memorize/recall 시그니처 확정 필요 |
| 4 | ChatContext 필드 확정 | 미정 | PersonalityView, SpeechRules 등 타입 설계 |
| 5 | InferenceLlm/Embedder/VectorStore trait 시그니처 | 미정 | 기존 Port 기반 + ChatMessage 통합 |
| 6 | wuxia-inference와 wuxia-embedder 통합 여부 | 미정 | 둘 다 llama-cpp-2 의존, 합칠지 검토 |
| 7 | Async/Sync 전략 계승 | 미정 | 기존 block_on 방식 유지 또는 재검토 |

---

## 변경 이력

| 버전 | 변경일시 | 변경 내역 |
|------|----------|-----------|
| v1.0.0 | 2026-03-12T21:00:00+09:00 | 초기 작성. 9개 설계 결정 확정: (1) 고수준 trait 위치 = core, (2) 업무 단위 분리 trait (ConversationAi/MemoryAi/PsychologyAi), (3) 세션 기반 ConversationAi (start_chat→chat→end_chat), (4) 저수준 Port를 llm2 내부로 이동 (시나리오 C), (4.1) 순수 함수 귀속: retrieval_score/rank = core 유지, recall = llm2 이동, (5) Infra 각각 독립 crate (inference/embedder/vectordb), (6) 비즈니스 레이어 명칭 = task, (7) 초기 scope = conversation + reflection, (8) facade 역할 = 얇은 위임 (task가 port 직접 보유), (9) 기존 wuxia-llm/wuxia-memory 삭제 후 코드 이동. 전체 crate 구조 (7개), 의존성 그래프, 허용 매트릭스, llm2 내부 모듈 구조, ChatMessage 통일 설계. 미결 사항 7건 기록. |
