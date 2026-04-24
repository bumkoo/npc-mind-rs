# NPC Tool & Memory 시스템 설계

> **Status**: Superseded — 2026-04-11 (original draft)  
> **현행 설계**: [`system-design-eventbus-cqrs.md` §9 RAG 설계](system-design-eventbus-cqrs.md#9-rag-설계-게임-내-히스토리)
>
> 이 문서는 초기 LanceDB 기반 설계안이다. 실제 구현은 단일 SQLite 파일에 FTS5(trigram) + sqlite-vec vec0 가상 테이블을 함께 두는 방식으로 결정되었다. LanceDB async-only 제약 회피 + sqlite-vec가 순수 C 확장이라 tokio 런타임 전이 없음이 결정 근거. 아래 내용은 역사적 참고용으로만 보존한다.
>
> **Scope (원본)**: NPC에게 "도구(Tool)"를 부여하고, 첫 번째 도구로 "기억(Memory)"을 구현하는 시스템 설계

---

## 1. 요구사항 정리

### 기능 요구사항

| # | 요구사항 | 비고 |
|---|---------|------|
| F1 | NPC가 대화 중 **도구를 호출**할 수 있다 | LLM function calling |
| F2 | 첫 번째 도구는 **기억 조회(recall)** | 벡터 유사도 검색 |
| F3 | **세계관 기억** — 공유 설정, 배경 지식 | 읽기 전용, 시나리오에서 로드 |
| F4 | **장기 기억** — 대화 후 요약·저장되는 에피소드 기억 | NPC별, 관계별 |
| F5 | 기억 종류별 **조회 권한·메타데이터** | 누가 알고 있는가 |
| F6 | **대화 요약 핸들러** — 대화 종료 후 기억 생성 | LLM 프롬프트 기반 |

### 비기능 요구사항

| # | 요구사항 | 비고 |
|---|---------|------|
| N1 | 헥사고날 아키텍처 유지 | 기존 패턴 준수 |
| N2 | feature flag로 점진적 활성화 | `--features memory`, `--features tool` |
| N3 | LanceDB 사용 (벡터 저장소) | 로컬 임베디드 DB, 서버 불필요 |
| N4 | 기존 embed 인프라(bge-m3) 재활용 | `TextEmbedder` 포트 |

---

## 2. 핵심 설계 결정

### 2-1. Tool은 도메인인가, 인프라인가?

**결론: 도구 정의(ToolDefinition)는 도메인, 도구 실행(ToolExecution)은 인프라**

```
┌─────────────────────────────────────────────────────┐
│ Domain                                               │
│                                                      │
│  ToolDefinition          ToolCapability              │
│  ├─ id: "recall"         ├─ npc가 어떤 도구를 쓸 수 있는가 │
│  ├─ description          ├─ 언제 사용할 수 있는가         │
│  └─ parameters schema    └─ 비용/제약 조건               │
│                                                      │
├──────────────────────────────────────────────────────┤
│ Application                                          │
│                                                      │
│  ToolOrchestrator                                    │
│  ├─ LLM의 tool_call → 도메인 검증 → 실행 위임           │
│  └─ 결과를 대화 컨텍스트에 주입                          │
│                                                      │
├──────────────────────────────────────────────────────┤
│ Adapter (Infrastructure)                             │
│                                                      │
│  rig-core Tool trait      LanceDB adapter            │
│  ├─ function calling 프로토콜  ├─ 벡터 저장/검색         │
│  └─ JSON schema 변환         └─ TextEmbedder 재활용    │
│                                                      │
└─────────────────────────────────────────────────────┘
```

**근거:**
- NPC가 어떤 능력을 가지고 있는지(기억 조회, 물건 줍기 등)는 **캐릭터의 본질적 속성** → 도메인
- 그 능력을 LLM function calling으로 실행하는 방식은 **기술적 구현** → 인프라
- 기존 패턴과 일치: `ActingGuide`(도메인)를 `ConversationPort`(인프라)가 LLM에 전달하는 것과 동일한 구조

### 2-2. rig.rs는 어떤 계층인가?

**결론: rig.rs는 인프라(Adapter) 계층**

rig-core는 LLM 통신 라이브러리이며, 현재 `RigChatAdapter`가 `ConversationPort` 포트를 구현하는 어댑터로 존재합니다. 도구 실행도 같은 패턴을 따릅니다:

```
Domain Port          →  Adapter (rig-core)
────────────────────    ─────────────────────
ConversationPort    →  RigChatAdapter          (현재)
ToolExecutor Port   →  RigToolAdapter          (새로 추가)
```

rig 0.33의 `MultiTurnStreamItem::ToolCall`을 처리하는 코드가 어댑터에 추가되고, 도메인은 `ToolExecutor` 포트를 통해서만 상호작용합니다.

### 2-3. Memory는 도메인인가?

**결론: Memory는 독립 도메인 모듈 (`domain::memory`)**

```
현재 domain/
├── personality    ← HEXACO 성격
├── emotion        ← OCC 감정
├── relationship   ← 관계
├── guide          ← 연기 가이드
├── pad            ← PAD 모델
└── (신규)
    ├── memory     ← 기억 도메인        ★
    └── tool       ← 도구 정의 도메인    ★
```

**근거:**
- 기억은 NPC의 행동을 결정하는 핵심 요소 — "무엇을 기억하는가"가 대사와 태도를 바꿈
- 기억의 종류, 접근 권한, 감쇠(decay)는 비즈니스 규칙
- 저장(LanceDB)은 인프라이지만, "어떤 기억이 관련성이 높은가"의 판단 로직은 도메인

---

## 3. Memory 도메인 설계

### 3-1. 기억의 분류 체계

```
Memory
├── WorldKnowledge (세계관 기억)
│   ├── Lore         — 역사, 전설, 규칙
│   ├── Geography    — 장소, 지역 설명
│   └── Character    — 인물 배경, 공개된 사실
│
├── EpisodicMemory (에피소드 기억 / 장기 기억)
│   ├── DialogueSummary  — 대화 요약
│   ├── KeyEvent         — 핵심 사건 (감정 강도 기반)
│   └── Decision         — NPC의 판단/결정 기록
│
└── SharedKnowledge (공유 지식)
    └── Rumor / News     — NPC 간 전파 가능한 정보
```

### 3-2. Memory 도메인 모델

```rust
// src/domain/memory/mod.rs

/// 기억 한 조각
pub struct Memory {
    pub id: MemoryId,
    pub kind: MemoryKind,
    pub content: String,              // 기억 내용 (텍스트)
    pub source: MemorySource,         // 누가/어디서 생성했나
    pub access: AccessControl,        // 누가 열람할 수 있는가
    pub created_at: SceneTimestamp,    // 게임 시간 기준
    pub emotional_valence: Option<f32>, // 감정가 (-1.0 ~ 1.0)
    pub importance: f32,              // 중요도 (0.0 ~ 1.0)
    pub tags: Vec<String>,           // 검색용 태그
}

/// 기억 유형
pub enum MemoryKind {
    /// 세계관 — 시나리오 로드 시 일괄 주입, 불변
    WorldKnowledge { category: WorldCategory },
    /// 에피소드 — 대화 후 생성, 시간에 따라 감쇠 가능
    Episodic { episode_type: EpisodeType },
    /// 공유 — NPC 간 전파 가능
    Shared { spread_range: SpreadRange },
}

pub enum WorldCategory { Lore, Geography, Character, Rule }
pub enum EpisodeType { DialogueSummary, KeyEvent, Decision }
pub enum SpreadRange { Local, Faction, Global }

/// 기억 출처
pub struct MemorySource {
    pub creator_npc_id: Option<String>,  // None이면 시스템(세계관) 생성
    pub scene_id: Option<String>,
    pub partner_id: Option<String>,
}

/// 접근 권한 — 누가 이 기억을 조회할 수 있는가
pub enum AccessControl {
    /// 모든 NPC가 접근 가능 (세계관 공통 지식)
    Public,
    /// 특정 NPC만 접근 가능 (개인 경험)
    Private { owner_ids: Vec<String> },
    /// 특정 관계의 NPC만 접근 (비밀 공유 등)
    Shared { owner_ids: Vec<String>, shared_with: Vec<String> },
    /// 조건부 접근 (신뢰도 임계값 등)
    Conditional { condition: AccessCondition },
}

pub enum AccessCondition {
    /// 신뢰도가 threshold 이상인 NPC만 열람
    TrustAbove { threshold: f32 },
    /// 친밀도가 threshold 이상인 NPC만 열람
    ClosenessAbove { threshold: f32 },
    /// 특정 faction/group 소속만 열람
    FactionMember { faction_id: String },
}
```

### 3-3. Memory 조회 결과 (Recall)

```rust
/// 기억 조회 결과 — 도구(recall)가 반환하는 값
pub struct RecallResult {
    pub memories: Vec<RecalledMemory>,
    pub query: String,
}

pub struct RecalledMemory {
    pub memory: Memory,
    pub relevance: f32,     // 벡터 유사도 (0.0 ~ 1.0)
    pub recency_boost: f32, // 최신성 보너스
    pub importance_boost: f32, // 중요도 보너스
    pub final_score: f32,   // 최종 스코어 (가중합)
}
```

### 3-4. 스코어링 공식

```
final_score = w_relevance × relevance
            + w_recency × recency_boost
            + w_importance × importance
            + w_emotion × |emotional_valence|

where:
  relevance     = cosine_similarity(query_embedding, memory_embedding)
  recency_boost = exp(-decay_rate × time_elapsed)
  w_relevance   = 0.5  (튜닝 상수, tuning.rs에 정의)
  w_recency     = 0.2
  w_importance  = 0.2
  w_emotion     = 0.1
```

이 가중치는 `tuning.rs`에서 관리하여 기존 감정 튜닝 상수와 동일한 패턴을 따릅니다.

---

## 4. Tool 도메인 설계

### 4-1. Tool 정의 모델

```rust
// src/domain/tool/mod.rs

/// NPC가 사용할 수 있는 도구 정의
pub struct ToolDefinition {
    pub id: String,
    pub name: String,
    pub description: String,        // LLM에 전달될 도구 설명
    pub parameters: ToolParameters, // JSON Schema
    pub constraints: ToolConstraints,
}

/// 도구 사용 제약
pub struct ToolConstraints {
    /// 대화 턴당 최대 호출 횟수
    pub max_calls_per_turn: Option<u32>,
    /// 이 도구를 사용할 수 있는 감정 조건 (예: 공포 상태에서만 "도주" 가능)
    pub emotion_conditions: Vec<EmotionCondition>,
    /// 비용 (게임 리소스 소모 등, 향후 확장)
    pub cost: Option<f32>,
}

/// NPC별 도구 능력 — 어떤 NPC가 어떤 도구를 갖는가
pub struct ToolCapability {
    pub npc_id: String,
    pub tools: Vec<ToolDefinition>,
}
```

### 4-2. Tool 실행 흐름

```
Player 대사 입력
    │
    ▼
ConversationPort.send_message()
    │
    ├─→ LLM 응답: 일반 텍스트 → 그대로 반환
    │
    └─→ LLM 응답: tool_call(recall, {query: "..."})
         │
         ▼
    ToolOrchestrator (Application 계층)
         │
         ├─ 1. 도메인 검증: NPC가 이 도구를 사용할 수 있는가?
         │     └─ ToolCapability 확인 + ToolConstraints 평가
         │
         ├─ 2. 실행 위임: ToolExecutor 포트 → 어댑터
         │     └─ recall → MemoryStore.search()
         │
         ├─ 3. 결과를 대화 컨텍스트에 주입
         │     └─ tool_response → ConversationPort
         │
         └─ 4. LLM이 tool 결과를 참고하여 최종 응답 생성
              └─ "아, 그때 그 일 말이냐..." (기억 기반 대사)
```

---

## 5. 계층별 구조 설계

### 5-1. 전체 아키텍처 (확장 후)

```
┌──────────────────────────────────────────────────────────┐
│                    Presentation / API                      │
│   Mind Studio REST + MCP (handlers/)                      │
└────────────────────────┬─────────────────────────────────┘
                         │
┌────────────────────────┴─────────────────────────────────┐
│                    Application                             │
│                                                            │
│  MindService          ToolOrchestrator    MemoryService    │
│  (감정+가이드)          (도구 실행 조율)     (기억 관리)      │
│                                                            │
│  DialogueTestService  SummarizationService                │
│  (대화 테스트)          (대화 → 기억 변환)                   │
│                                                            │
└────┬──────────────┬──────────────┬────────────────────────┘
     │              │              │
┌────┴────┐   ┌────┴────┐   ┌────┴──────────────┐
│ Domain  │   │ Domain  │   │ Domain            │
│         │   │         │   │                   │
│ emotion │   │ tool    │   │ memory            │
│ guide   │   │ ToolDef │   │ Memory, MemoryKind│
│ person- │   │ ToolCap │   │ AccessControl     │
│  ality  │   │ Constr. │   │ RecallResult      │
│ relat.  │   │         │   │ scoring rules     │
│ scene   │   │         │   │                   │
└────┬────┘   └─────────┘   └────┬──────────────┘
     │                           │
┌────┴───────────────────────────┴──────────────────────────┐
│                     Ports                                   │
│                                                             │
│  MindRepository (기존)                                      │
│  ConversationPort (기존) ─ tool_call 처리 확장               │
│  ToolExecutor (신규) ─ 도구 실행 추상화                      │
│  MemoryStore (신규) ─ 기억 저장·검색 추상화                   │
│  Summarizer (신규) ─ 대화 → 기억 요약 추상화                  │
│  TextEmbedder (기존) ─ 재활용                                │
│                                                             │
└────┬───────────────────────────┬──────────────────────────┘
     │                           │
┌────┴───────────────────────────┴──────────────────────────┐
│                     Adapters                                │
│                                                             │
│  RigChatAdapter (확장) ─ ToolCall 스트림 처리                 │
│  RigToolAdapter (신규) ─ rig tool trait 구현                  │
│  LanceDbMemoryStore (신규) ─ LanceDB 벡터 검색               │
│  LlmSummarizer (신규) ─ LLM 프롬프트로 요약 생성              │
│  InMemoryRepository (기존)                                   │
│  OrtEmbedder (기존)                                          │
│                                                             │
└──────────────────────────────────────────────────────────┘
```

### 5-2. 신규 포트 정의

```rust
// ports.rs 에 추가

// ---------------------------------------------------------------------------
// 기억 저장소 포트 (memory feature)
// ---------------------------------------------------------------------------

/// 기억 저장소 포트 — 기억의 저장·검색·삭제를 추상화
///
/// LanceDB, SQLite+벡터, 인메모리 등 구체적 저장소는 어댑터가 결정한다.
/// 검색은 벡터 유사도 + 메타데이터 필터링 조합.
#[cfg(feature = "memory")]
#[async_trait::async_trait]
pub trait MemoryStore: Send + Sync {
    /// 기억 저장 (임베딩은 내부에서 자동 생성)
    async fn store(&self, memory: Memory) -> Result<MemoryId, MemoryError>;

    /// 기억 일괄 저장 (세계관 초기 로드 등)
    async fn store_batch(&self, memories: Vec<Memory>) -> Result<Vec<MemoryId>, MemoryError>;

    /// 벡터 유사도 검색 + 접근 권한 필터링
    async fn search(
        &self,
        query: &str,
        accessor_npc_id: &str,
        filter: &MemoryFilter,
        limit: usize,
    ) -> Result<Vec<RecalledMemory>, MemoryError>;

    /// 특정 NPC의 기억 삭제 (리셋용)
    async fn clear_npc_memories(&self, npc_id: &str) -> Result<(), MemoryError>;
}

/// 검색 필터
#[cfg(feature = "memory")]
pub struct MemoryFilter {
    pub kinds: Option<Vec<MemoryKind>>,  // 특정 종류만
    pub min_importance: Option<f32>,      // 중요도 하한
    pub time_range: Option<TimeRange>,    // 시간 범위
}

// ---------------------------------------------------------------------------
// 도구 실행 포트 (tool feature)
// ---------------------------------------------------------------------------

/// 도구 실행 포트 — NPC의 도구 호출을 처리
///
/// LLM이 tool_call을 발생시키면 Application 계층(ToolOrchestrator)이
/// 도메인 검증 후 이 포트를 통해 실제 실행을 위임한다.
#[cfg(feature = "tool")]
#[async_trait::async_trait]
pub trait ToolExecutor: Send + Sync {
    /// 도구 실행 — tool_id와 인자를 받아 결과 반환
    async fn execute(
        &self,
        tool_id: &str,
        args: serde_json::Value,
        context: &ToolContext,
    ) -> Result<serde_json::Value, ToolError>;
}

/// 도구 실행 컨텍스트 — 누가, 어떤 상황에서 호출했는가
#[cfg(feature = "tool")]
pub struct ToolContext {
    pub npc_id: String,
    pub partner_id: String,
    pub current_emotion: Option<EmotionState>,
}

// ---------------------------------------------------------------------------
// 요약 포트 (memory feature)
// ---------------------------------------------------------------------------

/// 대화 요약 포트 — 대화 이력을 기억으로 변환
///
/// LLM 기반 요약, 규칙 기반 추출 등 구현을 교체할 수 있다.
#[cfg(feature = "memory")]
#[async_trait::async_trait]
pub trait Summarizer: Send + Sync {
    /// 대화 이력 → 에피소드 기억 목록
    async fn summarize(
        &self,
        dialogue: &[DialogueTurn],
        context: &SummarizationContext,
    ) -> Result<Vec<Memory>, SummarizationError>;
}

#[cfg(feature = "memory")]
pub struct SummarizationContext {
    pub npc_id: String,
    pub partner_id: String,
    pub npc_name: String,
    pub partner_name: String,
    pub scene_description: Option<String>,
    pub emotional_arc: Vec<(EmotionType, f32)>, // 감정 변화 궤적
}
```

### 5-3. Feature Flag 설계

```toml
# Cargo.toml
[features]
default = []
embed  = ["dep:bge-m3-onnx-rust"]
chat   = ["dep:rig-core", "dep:async-trait", ...]
memory = ["embed", "dep:lancedb", "dep:async-trait", "dep:tokio"]
tool   = ["chat", "memory"]  # tool은 chat(LLM) + memory(첫 도구)에 의존
mind-studio = ["dep:axum", ...]
```

`memory`가 `embed`에 의존하는 이유: 기억 저장 시 임베딩 벡터를 생성해야 하므로 `TextEmbedder`가 필요합니다. `tool`이 `chat`에 의존하는 이유: LLM의 function calling을 처리하려면 대화 세션이 필요합니다.

---

## 6. ConversationPort 확장 — Tool Call 처리

### 6-1. 현재 vs 확장

현재 `RigChatAdapter`는 `MultiTurnStreamItem::ToolCall`을 무시합니다. Tool 시스템 도입 시 이를 처리해야 합니다.

**선택지 두 가지:**

| 방식 | 설명 | 장점 | 단점 |
|------|------|------|------|
| A. ConversationPort 확장 | `send_message()`가 `ToolCall`도 반환 | 기존 포트 확장, 단순 | 포트가 복잡해짐 |
| B. 별도 AgentPort 신설 | Tool-aware 대화를 별도 포트로 | 기존 포트 불변 | 중복 가능성 |

**결론: 방식 A — ConversationPort 확장 (응답 타입 확장)**

```rust
/// 대화 응답 — 텍스트 또는 도구 호출
#[cfg(feature = "chat")]
pub enum ChatAction {
    /// 일반 텍스트 응답
    Text(ChatResponse),
    /// 도구 호출 요청 — Application 계층이 실행 후 결과를 돌려줘야 함
    ToolCall {
        call_id: String,
        tool_name: String,
        arguments: serde_json::Value,
    },
}

// ConversationPort에 메서드 추가
#[cfg(feature = "tool")]
async fn send_message_with_tools(
    &self,
    session_id: &str,
    user_message: &str,
    available_tools: &[ToolDefinition],
) -> Result<ChatAction, ConversationError>;

#[cfg(feature = "tool")]
async fn submit_tool_result(
    &self,
    session_id: &str,
    call_id: &str,
    result: serde_json::Value,
) -> Result<ChatAction, ConversationError>;
```

### 6-2. Tool-Aware 대화 루프

```
appraise → start_session(prompt + tool_definitions)
  │
  loop {
    ├─ send_message_with_tools(상대 대사, available_tools)
    │
    ├─→ ChatAction::Text(response)
    │     → apply_stimulus(PAD) → [Beat 전환 시 update_system_prompt]
    │     → 루프 계속
    │
    └─→ ChatAction::ToolCall { tool_name: "recall", args: {query: "..."} }
          │
          ├─ ToolOrchestrator.validate(npc, tool_name)
          ├─ ToolExecutor.execute("recall", args, context)
          │     └─ MemoryStore.search(query, npc_id, filter, limit)
          │     └─ → RecallResult { memories: [...] }
          │
          ├─ submit_tool_result(call_id, recall_result_json)
          │     → LLM이 기억을 참고하여 최종 텍스트 생성
          │
          └─ ChatAction::Text(response) → apply_stimulus → 루프 계속
  }
  │
  end_session → after_dialogue
             → SummarizationService.summarize(history) → MemoryStore.store()
```

---

## 7. 대화 요약 핸들러 설계

### 7-1. 요약 파이프라인

```
대화 종료 (after_dialogue)
    │
    ▼
┌─────────────────────────────────────────┐
│ SummarizationService (Application)       │
│                                          │
│  1. 대화 이력 수집                        │
│  2. 감정 궤적 추출 (emotion arc)          │
│  3. Summarizer 포트 호출                  │
│  4. 결과 검증 (길이, 필수 필드)            │
│  5. MemoryStore에 저장                    │
│                                          │
└───────────┬─────────────────────────────┘
            │
            ▼
┌─────────────────────────────────────────┐
│ LlmSummarizer (Adapter)                 │
│                                          │
│  프롬프트 구조:                            │
│  ┌─────────────────────────────────────┐ │
│  │ System: 당신은 기억 정리 담당입니다.     │ │
│  │ NPC '{name}'의 시점에서 대화를 요약하세요.│ │
│  │                                     │ │
│  │ 출력 형식:                            │ │
│  │ - summary: 전체 요약 (2-3문장)        │ │
│  │ - key_events: 핵심 사건 목록          │ │
│  │ - emotional_shift: 감정 변화 요약     │ │
│  │ - decisions: NPC가 내린 판단          │ │
│  │ - importance: 0.0 ~ 1.0             │ │
│  └─────────────────────────────────────┘ │
│                                          │
│  입력: 전체 대화 이력 + 감정 궤적           │
│  출력: Memory[] (DialogueSummary + KeyEvent)│
│                                          │
└─────────────────────────────────────────┘
```

### 7-2. 요약 프롬프트 예시

```
당신은 무협 세계관의 '{npc_name}'입니다.
방금 '{partner_name}'과의 대화가 끝났습니다.

이 대화에서 기억해야 할 것들을 정리하세요:

[대화 이력]
{dialogue_history}

[감정 변화]
시작: {start_emotions}
종료: {end_emotions}

다음 JSON 형식으로 응답하세요:
{
  "summary": "대화의 핵심을 2-3문장으로",
  "key_events": [
    {"description": "...", "importance": 0.8, "emotional_valence": 0.5}
  ],
  "decisions": [
    {"description": "...", "reason": "..."}
  ],
  "relationship_impression": "상대에 대한 인상 변화"
}
```

### 7-3. 비동기 요약 (fire-and-forget)

대화 요약은 **대화 종료 후 비동기로 실행**합니다. Player에게 대기 시간을 부과하지 않기 위함입니다.

```rust
// after_dialogue 내부
pub async fn after_dialogue_with_memory(&self, req: AfterDialogueRequest) -> AfterDialogueResponse {
    // 1. 기존 로직: 관계 갱신 + 감정 클리어
    let response = self.mind_service.after_dialogue(req).await;

    // 2. 비동기 요약 시작 (fire-and-forget)
    let history = self.conversation_port.end_session(&session_id).await?;
    tokio::spawn(async move {
        if let Err(e) = summarization_service.summarize_and_store(history, context).await {
            tracing::warn!("기억 생성 실패: {e}");
        }
    });

    response
}
```

---

## 8. Event Sourcing / CQRS 검토

### 8-1. 현재 상태

현재 시스템은 **상태 기반(State-based)** 모델:
- `EmotionState`는 매 턴 덮어쓰기
- `Relationship`은 누적 변경
- 대화 이력은 세션 종료 시 반환 후 소멸

### 8-2. Event Sourcing 적용 검토

| 구성 요소 | Event Sourcing 적합도 | 이유 |
|-----------|---------------------|------|
| 감정 변화 | ⭐⭐ 중간 | 매 턴 변화를 이벤트로 기록하면 감정 궤적 재현 가능. 단, 현재는 `EmotionState` 덮어쓰기로 충분 |
| 관계 변화 | ⭐⭐ 중간 | 장기적 관계 이력 추적에 유용하지만, 현재 규모에서는 과도 |
| **기억 생성** | ⭐⭐⭐ **높음** | 대화 이벤트 → 요약 → 기억 파이프라인이 자연스러운 이벤트 소싱 |
| 도구 호출 | ⭐⭐ 중간 | 감사(audit) 목적으로 유용하지만 필수 아님 |

### 8-3. CQRS 적용 — Memory에 한정하여 도입

**기억 시스템만 CQRS 패턴을 채택합니다:**

```
                    ┌─────────────────────────┐
                    │     Write Model          │
                    │  (Command Side)          │
                    │                          │
  대화 종료 ────────→│  DialogueEvent {         │
  도구 호출 결과 ──→│    turns, emotions,       │
  관계 변화 ───────→│    relationship_delta     │
                    │  }                       │
                    │         │                │
                    │         ▼                │
                    │  Summarizer (LLM)        │
                    │         │                │
                    │         ▼                │
                    │  MemoryStore.store()     │
                    └─────────────────────────┘

                    ┌─────────────────────────┐
                    │     Read Model           │
                    │  (Query Side)            │
                    │                          │
  recall(query) ──→│  MemoryStore.search()    │
                    │  ├─ 벡터 유사도           │
                    │  ├─ 접근 권한 필터링       │
                    │  └─ 스코어링              │
                    │         │                │
                    │         ▼                │
                    │  RecallResult            │
                    └─────────────────────────┘
```

**Write / Read 분리의 이점:**
- **Write**: 대화 이벤트 원본은 전체 보존 (이벤트 로그). 요약은 비동기로 생성. 나중에 요약 프롬프트를 개선하면 기존 이벤트를 재처리할 수 있음
- **Read**: 벡터 검색에 최적화된 인덱스 (LanceDB). 접근 권한은 쿼리 시점에 평가

### 8-4. 이벤트 로그 설계

```rust
/// 대화 이벤트 — 기억 생성의 원본 데이터
pub struct DialogueEvent {
    pub id: String,
    pub npc_id: String,
    pub partner_id: String,
    pub scene_id: Option<String>,
    pub timestamp: SceneTimestamp,
    pub turns: Vec<DialogueTurn>,
    pub emotion_arc: Vec<EmotionSnapshot>,   // 턴별 감정 스냅샷
    pub relationship_before: Relationship,
    pub relationship_after: Relationship,
    pub beat_transitions: Vec<BeatTransition>, // Beat 전환 기록
}

/// Beat 전환 기록
pub struct BeatTransition {
    pub turn_index: usize,
    pub from_focus_id: Option<String>,
    pub to_focus_id: String,
    pub trigger_emotion: EmotionType,
}
```

**전체 Event Sourcing(감정, 관계 포함)은 현 단계에서 과도합니다.** 1인 개발 규모에서 기억 시스템의 Write/Read 분리만으로 충분한 확장성을 확보할 수 있으며, 이벤트 로그를 보존해두면 나중에 필요 시 전체 ES로 확장할 수 있는 여지를 남깁니다.

---

## 9. LanceDB 어댑터 설계

### 9-1. 왜 LanceDB인가

| 항목 | LanceDB | SQLite + Extension | Qdrant |
|------|---------|-------------------|--------|
| 배포 복잡도 | 임베디드 (서버 불필요) ✅ | 임베디드 ✅ | 별도 서버 필요 ❌ |
| 벡터 검색 | 네이티브 ANN ✅ | 외부 확장 필요 | 네이티브 ✅ |
| Rust 지원 | lancedb crate ✅ | rusqlite ✅ | qdrant-client ✅ |
| 메타데이터 필터링 | SQL-like 필터 ✅ | SQL ✅ | Qdrant 필터 ✅ |
| 1인 개발 적합성 | **높음** | 중간 | 낮음 (운영 부담) |

### 9-2. LanceDB 스키마

```
Table: memories
┌─────────────┬─────────────┬──────────────────────────────┐
│ Column       │ Type        │ Description                  │
├─────────────┼─────────────┼──────────────────────────────┤
│ id           │ String      │ UUID                         │
│ kind         │ String      │ "world_knowledge" / "episodic" │
│ content      │ String      │ 기억 내용 텍스트              │
│ embedding    │ Vector[1024]│ bge-m3 임베딩                │
│ npc_id       │ String      │ 소유 NPC (NULL = 공유)       │
│ partner_id   │ String?     │ 관련 상대 NPC               │
│ access_type  │ String      │ "public" / "private" / ...   │
│ access_ids   │ String[]    │ 접근 가능한 NPC ID 목록      │
│ importance   │ Float       │ 0.0 ~ 1.0                   │
│ emotional_v  │ Float       │ 감정가 -1.0 ~ 1.0           │
│ created_at   │ Int64       │ 게임 타임스탬프              │
│ scene_id     │ String?     │ 출처 Scene                   │
│ tags         │ String[]    │ 검색 태그                    │
│ episode_type │ String?     │ "summary" / "key_event" / ...│
│ scenario_id  │ String      │ 시나리오 구분 (멀티 시나리오) │
└─────────────┴─────────────┴──────────────────────────────┘

Table: dialogue_events (이벤트 로그)
┌─────────────┬─────────────┬──────────────────────────────┐
│ id           │ String      │ UUID                         │
│ npc_id       │ String      │                              │
│ partner_id   │ String      │                              │
│ scene_id     │ String?     │                              │
│ timestamp    │ Int64       │                              │
│ event_json   │ String      │ DialogueEvent 전체 JSON      │
│ processed    │ Boolean     │ 요약 처리 완료 여부           │
│ scenario_id  │ String      │                              │
└─────────────┴─────────────┴──────────────────────────────┘
```

### 9-3. LanceDbMemoryStore 어댑터

```rust
// src/adapter/lancedb_memory.rs

pub struct LanceDbMemoryStore {
    db: lancedb::Database,
    embedder: Arc<Mutex<dyn TextEmbedder>>, // bge-m3 재활용
    memory_table: String,
    event_table: String,
}

#[async_trait]
impl MemoryStore for LanceDbMemoryStore {
    async fn store(&self, memory: Memory) -> Result<MemoryId, MemoryError> {
        // 1. content → embedding (TextEmbedder)
        // 2. Memory → LanceDB row
        // 3. table.add() → MemoryId
    }

    async fn search(
        &self,
        query: &str,
        accessor_npc_id: &str,
        filter: &MemoryFilter,
        limit: usize,
    ) -> Result<Vec<RecalledMemory>, MemoryError> {
        // 1. query → embedding
        // 2. vector_search(embedding, limit * 2)  // 필터링 마진
        //      .filter(access_control_sql(accessor_npc_id))
        //      .filter(kind_filter)
        //      .execute()
        // 3. 결과에 recency_boost, importance_boost 적용
        // 4. final_score로 재정렬 → top-limit 반환
    }
}
```

---

## 10. 기억과 ActingGuide 통합

기억 조회 결과는 LLM의 tool 응답으로 주입되지만, **시스템 프롬프트에도 핵심 기억을 포함**하는 것이 효과적입니다.

### 10-1. ActingGuide 확장

```rust
// domain/guide/mod.rs 확장

pub struct ActingGuide {
    // ... 기존 필드 ...
    pub npc_name: String,
    pub personality: PersonalitySnapshot,
    pub emotion: EmotionSnapshot,
    pub directive: ActingDirective,

    // ★ 신규: 기억 컨텍스트
    pub memory_context: Option<MemoryContext>,
}

pub struct MemoryContext {
    /// Scene 시작 시 자동 조회된 관련 기억 요약
    pub relevant_memories: Vec<MemorySummary>,
    /// 사용 가능한 도구 목록 (LLM에 전달)
    pub available_tools: Vec<ToolSummary>,
}

pub struct MemorySummary {
    pub content: String,
    pub source_hint: String,  // "이전 대화에서", "세계관 지식" 등
}
```

### 10-2. 프롬프트 포맷 예시

```
[성격] 당신은 소림사의 무승 혜광입니다. ...
[감정] 현재 경계(Fear 0.6)하고 있으며 ...
[연기 지시] ...

[기억]
- (세계관) 이 객잔은 혈마맹의 세력권에 있다
- (이전 대화) 지난번 장무기와 만났을 때 그가 의심스러운 행동을 했다
- (핵심 사건) 사부가 "혈마맹과 거래하지 말라"고 당부했다

[도구]
당신은 다음 도구를 사용할 수 있습니다:
- recall(query): 기억을 떠올립니다. 대화 중 과거 일이 떠오르면 사용하세요.
```

---

## 11. 구현 로드맵

### Phase 1: Memory 도메인 + 저장소 (2주)

```
[x] domain/memory/ — Memory, MemoryKind, AccessControl, RecallResult
[x] ports.rs — MemoryStore, MemoryFilter
[x] adapter/lancedb_memory.rs — LanceDbMemoryStore
[x] tuning.rs — 기억 스코어링 가중치 상수
[x] Cargo.toml — memory feature flag + lancedb 의존성
[ ] 통합 테스트: 저장 → 검색 → 접근 권한 필터링
```

### Phase 2: Tool 도메인 + ConversationPort 확장 (2주)

```
[ ] domain/tool/ — ToolDefinition, ToolCapability, ToolConstraints
[ ] ports.rs — ToolExecutor, ToolContext, ChatAction
[ ] adapter/rig_chat.rs — ToolCall 스트림 처리 확장
[ ] application/tool_orchestrator.rs — 도구 실행 조율
[ ] 통합 테스트: LLM tool_call → recall → 응답
```

### Phase 3: 대화 요약 핸들러 (1주)

```
[ ] ports.rs — Summarizer, SummarizationContext
[ ] adapter/llm_summarizer.rs — LLM 프롬프트 기반 요약
[ ] application/summarization_service.rs — 요약 오케스트레이터
[ ] dialogue_events 이벤트 로그 저장
[ ] after_dialogue 확장: 비동기 요약 + 기억 저장
```

### Phase 4: ActingGuide 통합 + Mind Studio UI (1주)

```
[ ] domain/guide — MemoryContext 추가
[ ] presentation/ — 기억 포맷 로케일 지원
[ ] mind-studio handlers — 기억 CRUD REST API
[ ] mind-studio-ui — 기억 뷰어 컴포넌트
```

---

## 12. Trade-off 분석

| 결정 | 선택 | 대안 | 트레이드오프 |
|------|------|------|------------|
| Memory를 도메인으로 | ✅ 도메인 | 인프라 전용 | 복잡도↑ but 기억 규칙이 비즈니스 로직이므로 올바른 배치 |
| LanceDB | ✅ 채택 | SQLite+ext, Qdrant | 서버 불필요, Rust 지원 우수. 대규모에서 Qdrant가 나을 수 있음 |
| 부분 CQRS (기억만) | ✅ 채택 | 전체 ES, 상태 기반 | 실용적 타협. 이벤트 로그 보존으로 나중에 전체 ES 확장 가능 |
| ConversationPort 확장 | ✅ 채택 | 별도 AgentPort | 기존 포트 활용, 중복 방지. 다만 포트가 커질 수 있음 |
| Fire-and-forget 요약 | ✅ 채택 | 동기 요약 | UX 우선 (대기 없음). 요약 실패 시 이벤트 로그에서 재시도 가능 |
| bge-m3 임베딩 재활용 | ✅ 채택 | 별도 모델 | 배포 단순화. 다국어(ko/en) 지원되는 bge-m3가 기억 검색에도 적합 |

---

## 13. 향후 재검토 항목

시스템 성장에 따라 재검토가 필요한 사항:

1. **기억 감쇠(Decay)**: 현재는 `recency_boost`로 자연 감쇠. 명시적 삭제/보관 정책이 필요할 수 있음
2. **멀티 NPC 기억 전파**: SharedKnowledge의 전파 메커니즘 (소문, 뉴스)
3. **기억 충돌 해소**: 같은 사건에 대해 NPC별로 다른 기억이 있을 때
4. **컨텍스트 윈도우 관리**: 기억이 많아지면 system prompt 토큰 제한 관리 필요
5. **임베딩 모델 교체**: bge-m3보다 나은 모델 등장 시 마이그레이션 전략
6. **전체 Event Sourcing**: 감정/관계 변화까지 이벤트로 관리할 필요가 생기면
