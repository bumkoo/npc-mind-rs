# ADR-001: rig 라이브러리를 활용한 프롬프트 품질 테스트 Agent 통합

**Status:** Accepted (구현 완료)
**Date:** 2026-04-03
**Deciders:** Bekay

## Context

NPC Mind Engine은 HEXACO 성격 → OCC 감정 → LLM 연기 가이드(프롬프트)를 생성하는 파이프라인을 갖추고 있다. 현재 Mind Studio에서 `FormattedMindService`가 생성한 `prompt`는 사람이 읽고 평가하거나, MCP 서버를 통해 외부 LLM에 수동으로 전달하는 방식으로 검증한다.

**문제:** 프롬프트 품질을 체계적으로 검증하려면 생성된 프롬프트를 실제 LLM에 system prompt로 주입하고, 다턴 대화를 통해 NPC가 감정·성격에 맞게 연기하는지 확인하는 **자동화된 대화 루프**가 필요하다.

**제약 조건:**
- 로컬 추론 엔진 서버 (`http://127.0.0.1:8081/v1`, OpenAI-compatible API)를 사용
- 기존 DDD + 헥사고날 아키텍처를 유지
- Rust 기반 솔루션 선호
- Mind Studio 웹 UI와 통합하여 대화 과정을 시각화

## Decision

[rig](https://github.com/0xPlaygrounds/rig) (`rig-core`) 라이브러리를 **새로운 어댑터 계층**으로 추가하여, Mind Engine이 생성한 프롬프트로 LLM Agent를 구성하고 대화 테스트를 수행한다.

## 아키텍처 설계

### 계층 배치 (헥사고날 경계 준수)

```
┌─────────────────────────────────────────────────────────┐
│                    Presentation                          │
│  Mind Studio (Axum)    MCP Server (Python)               │
│  ┌─────────────┐       ┌─────────────────┐              │
│  │ 대화 테스트  │       │ chat/dialogue   │              │
│  │ UI 패널     │       │ MCP tools       │              │
│  └──────┬──────┘       └────────┬────────┘              │
├─────────┼───────────────────────┼───────────────────────┤
│         │      Application      │                        │
│         ▼                       ▼                        │
│  ┌──────────────────────────────────────┐               │
│  │         DialogueTestService          │  ← 새로운 서비스│
│  │  (MindService + ConversationAgent)   │               │
│  └──────────┬──────────────┬────────────┘               │
│             │              │                             │
├─────────────┼──────────────┼────────────────────────────┤
│             │    Ports     │                             │
│             ▼              ▼                             │
│  ┌──────────────┐  ┌─────────────────┐  ┌────────────────────┐│
│  │MindRepository│  │ConversationPort │  │LlamaServerMonitor  ││
│  │(기존)        │  │                 │  │(health/slots/      ││
│  └──────────────┘  └────────┬────────┘  │ metrics)           ││
│                             │           └─────────┬──────────┘│
├─────────────────────────────┼─────────────────────┼──────────┤
│                   Adapters  │                     │           │
│                             ▼                     │           │
│                   ┌─────────────────┐             │           │
│                   │  RigChatAdapter │ ────────────┘           │
│                   │  (rig-core)     │ ← 4개 포트 구현         │
│                   └────────┬────────┘                         │
│                            │ (공유 reqwest::Client)           │
│                            ▼                                  │
│                   http://127.0.0.1:8081                       │
│                   ├─ /v1/chat/completions (rig)               │
│                   ├─ /v1/models (모델 감지)                    │
│                   ├─ /health, /slots, /metrics (모니터링)      │
│                   (로컬 추론 엔진)                             │
└─────────────────────────────────────────────────────────────┘
```

### 1. 새로운 포트: `ConversationPort`

```rust
// src/ports.rs 에 추가

/// 대화 에이전트 포트 — LLM과의 대화 세션을 추상화
///
/// Mind Engine이 생성한 프롬프트를 system prompt로 사용하여
/// LLM과 다턴 대화를 수행한다. rig 외 다른 LLM 클라이언트로
/// 교체 가능하도록 인터페이스를 추상화한다.
#[async_trait]
pub trait ConversationPort: Send + Sync {
    /// 새 대화 세션을 시작한다.
    /// system_prompt: MindEngine이 생성한 ActingGuide 프롬프트
    /// 반환: 세션 식별자
    async fn start_session(
        &self,
        session_id: &str,
        system_prompt: &str,
    ) -> Result<(), ConversationError>;

    /// 사용자/상대 NPC의 대사를 전달하고 응답을 받는다.
    async fn send_message(
        &self,
        session_id: &str,
        user_message: &str,
    ) -> Result<String, ConversationError>;

    /// system_prompt를 갱신한다 (Beat 전환 시).
    async fn update_system_prompt(
        &self,
        session_id: &str,
        new_prompt: &str,
    ) -> Result<(), ConversationError>;

    /// 세션을 종료하고 대화 이력을 반환한다.
    async fn end_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<DialogueTurn>, ConversationError>;
}

#[derive(Debug)]
pub struct DialogueTurn {
    pub role: DialogueRole,
    pub content: String,
}

#[derive(Debug)]
pub enum DialogueRole {
    System,
    User,      // 대화 상대 (Player 또는 상대 NPC)
    Assistant,  // 이 NPC의 응답
}

#[derive(Debug, thiserror::Error)]
pub enum ConversationError {
    #[error("LLM connection failed: {0}")]
    ConnectionError(String),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("LLM inference error: {0}")]
    InferenceError(String),
}
```

**설계 근거:**
- `async_trait` 사용: LLM 호출은 본질적으로 비동기 I/O
- 세션 기반: 다턴 대화의 컨텍스트(히스토리)를 유지
- `update_system_prompt`: Beat 전환 시 감정 변화를 반영한 새 프롬프트 주입 — **핵심 차별점**
- rig에 의존하지 않는 순수 인터페이스: 도메인/어플리케이션 계층은 rig를 모름

### 2. 새로운 어댑터: `RigChatAdapter`

```rust
// src/adapter/rig_chat.rs (실제 구현)

use rig::client::CompletionClient;
use rig::completion::{Chat, Message};
use rig::providers::openai;
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct RigChatAdapter {
    client: openai::CompletionsClient,  // completions_api()가 반환하는 타입
    model_name: String,
    sessions: RwLock<HashMap<String, ChatSession>>,
}

struct ChatSession {
    system_prompt: String,
    rig_history: Vec<Message>,           // rig Message (LLM API 전달용)
    dialogue_history: Vec<DialogueTurn>, // 도메인 이력 (반환용)
}

impl RigChatAdapter {
    pub fn new(base_url: &str, model_name: &str) -> Self {
        // rig 0.33: 기본 Responses API → completions_api()로 Chat Completions 전환
        let client = openai::Client::builder()
            .api_key("no-key-needed")
            .base_url(base_url)
            .build()
            .expect("OpenAI 호환 클라이언트 생성 실패")
            .completions_api();
        Self { client, model_name: model_name.to_string(), sessions: RwLock::new(HashMap::new()) }
    }

    async fn chat_with_agent(&self, system_prompt: &str, user_message: &str, history: Vec<Message>)
        -> Result<String, ConversationError>
    {
        let agent = self.client.agent(&self.model_name).preamble(system_prompt).build();
        let response: String = Chat::chat(&agent, user_message, history)
            .await.map_err(|e: rig::completion::PromptError| ConversationError::InferenceError(e.to_string()))?;
        Ok(response)
    }
}

// ConversationPort 구현: start_session, send_message, update_system_prompt, end_session
// send_message: read lock → chat_with_agent → write lock → 이력 업데이트
// update_system_prompt: system_prompt만 교체, rig_history 유지 (Beat 전환)
```

**Feature 게이트:**
```toml
# Cargo.toml
[features]
chat = ["dep:rig-core", "dep:async-trait", "dep:tokio"]

[dependencies]
# Windows MSVC: default-features = false + reqwest-native-tls (aws-lc-sys 회피)
rig-core = { version = "0.33", optional = true, default-features = false, features = ["reqwest-native-tls"] }
async-trait = { version = "0.1", optional = true }
```

**rig 0.33 핵심 타입 정리:**
- `openai::Client::builder().build()` → `openai::Client` (기본: Responses API)
- `.completions_api()` → `openai::CompletionsClient` = `client::Client<OpenAICompletionsExt>`
- `.agent(model).preamble(prompt).build()` 호출 시 `use rig::client::CompletionClient` 필요
- `Chat::chat(&agent, msg, history)` 호출 시 `use rig::completion::Chat` 필요 (UFCS로 타입 추론 해결)

### 3. 새로운 어플리케이션 서비스: `DialogueTestService`

```rust
// src/application/dialogue_test_service.rs

/// 대화 테스트 오케스트레이터
///
/// MindService의 프롬프트 생성 + ConversationPort의 LLM 대화를
/// 하나의 루프로 결합한다.
pub struct DialogueTestService<R, A, S, C>
where
    R: MindRepository,
    A: Appraiser,
    S: StimulusProcessor,
    C: ConversationPort,
{
    mind: FormattedMindService<R, A, S>,
    chat: C,
    analyzer: Option<Box<dyn UtteranceAnalyzer>>,  // embed 시 PAD 자동 분석
}
```

**핵심 대화 루프:**

```
┌──────────┐     ① appraise()      ┌──────────────┐
│  Mind     │ ──── prompt ────────▶ │  Conversation │
│  Service  │                       │  Port (rig)   │
│           │     ③ stimulus()      │               │
│           │ ◀── LLM 응답 ──────  │               │
│           │  (PAD 자동 분석)      │               │
│           │                       │               │
│           │  ④ beat 전환 시:      │               │
│           │  update_system_prompt │               │
│           │ ────  new prompt ───▶ │               │
│           │                       │               │
│           │     ⑤ 반복...         │               │
└──────────┘                       └──────────────┘
```

```rust
impl<R, A, S, C> DialogueTestService<R, A, S, C> {
    /// 한 턴의 대화를 처리한다.
    pub async fn process_turn(
        &self,
        session_id: &str,
        user_utterance: &str,  // Player 또는 상대 NPC의 대사
        npc_id: &str,
        partner_id: &str,
    ) -> Result<DialogueTurnResult, DialogueTestError> {
        // 1. LLM에 상대 대사 전달 → NPC 응답 받기
        let npc_response = self.chat
            .send_message(session_id, user_utterance).await?;

        // 2. 상대 대사의 PAD 분석 (embed feature 시 자동)
        let pad = if let Some(analyzer) = &self.analyzer {
            Some(analyzer.analyze(user_utterance)?)
        } else {
            None  // 수동 PAD 입력 필요
        };

        // 3. stimulus 적용 → 감정 변화 + Beat 전환 체크
        if let Some(pad) = pad {
            let stimulus_result = self.mind.apply_stimulus(
                StimulusRequest { npc_id, partner_id, pad, .. }
            )?;

            // 4. Beat 전환 시 → system_prompt 갱신
            if stimulus_result.beat_changed {
                self.chat.update_system_prompt(
                    session_id,
                    &stimulus_result.prompt,
                ).await?;
            }
        }

        Ok(DialogueTurnResult {
            npc_response,
            emotions: /* current emotion state */,
            beat_changed: /* ... */,
        })
    }
}
```

### 4. Mind Studio 통합

**새로운 API 엔드포인트:**

| Endpoint | 기능 |
|----------|------|
| `POST /api/chat/start` | 대화 세션 시작 (appraise → prompt → rig agent 생성) |
| `POST /api/chat/message` | 대사 전송 → NPC 응답 + 감정 변화 |
| `POST /api/chat/end` | 세션 종료 + 전체 대화 이력 + 관계 갱신 |
| `GET /api/chat/history` | 현재 세션 대화 이력 |
| `GET /api/llm/status` | 통합 서버 상태 (health + model + slots + metrics) |
| `GET /api/llm/health` | llama-server 헬스 체크 |
| `GET /api/llm/slots` | llama-server 슬롯 상태 |
| `GET /api/llm/metrics` | Prometheus 메트릭 (파싱 + 원문) |

**UI 패널 (`mind-studio-ui/` — Vite + React + Zustand):**
- 채팅 인터페이스: Player 대사 입력 → NPC 응답 표시
- 실시간 감정 변화 그래프 (기존 턴 히스토리 활용)
- Beat 전환 알림 배너
- system_prompt 변경 이력 표시 (프롬프트 품질 디버깅)

### 5. 파일 구조 변경

```
src/
  adapter/
    rig_chat.rs            ← ConversationPort + LlamaServerMonitor 구현 (rig-core)
    llama_timings.rs       ← TimingsCapturingClient (rig HttpClientExt 래퍼)
  application/
    dialogue_test_service.rs  ← 대화 테스트 오케스트레이터
  ports.rs                 ← ConversationPort + LlamaServerMonitor 포트 정의
  bin/mind-studio/
    handlers/chat.rs       ← /api/chat/* 엔드포인트
    handlers/llm.rs        ← /api/llm/* 모니터링 엔드포인트
    state.rs               ← AppState (chat, llm_info, llm_detector, llm_monitor)
    static/                ← mind-studio-ui/ 빌드 출력 (Vite)
tests/
  llm_monitor_test.rs      ← LlamaServerMonitor mock 서버 테스트
  llama_timings_test.rs    ← TimingsCapturingClient mock 서버 테스트
```

## Options Considered

### Option A: rig-core를 헥사고날 어댑터로 통합 (선택)

| Dimension | Assessment |
|-----------|------------|
| 복잡도 | Medium — 새 포트 1개, 어댑터 1개, 서비스 1개 |
| 아키텍처 정합성 | High — 기존 DDD/헥사고날 패턴 완벽 준수 |
| 유연성 | High — ConversationPort로 rig 외 클라이언트 교체 가능 |
| 팀 친숙도 | Medium — Rust 기반이라 기존 코드와 동질적 |
| 테스트 용이성 | High — MockConversationPort로 LLM 없이 테스트 가능 |

**Pros:**
- 도메인/어플리케이션 계층이 rig에 직접 의존하지 않음 (포트 추상화)
- Beat 전환 시 system_prompt 동적 갱신 — Mind Engine의 핵심 가치 증명
- feature gate(`chat`)로 선택적 포함, 기존 빌드에 영향 없음
- Mind Studio UI에서 대화 과정 시각화 가능
- MCP 서버에도 자연스럽게 확장 (Claude Agent가 대화 테스트 자동화)

**Cons:**
- rig의 OpenAI-compatible 클라이언트가 로컬 서버와 호환성 검증 필요
- async 런타임 의존성 추가 (tokio — mind-studio feature에 이미 있음)
- rig 라이브러리의 안정성/버전 변경 리스크

### Option B: reqwest로 직접 OpenAI API 호출

| Dimension | Assessment |
|-----------|------------|
| 복잡도 | Low — HTTP 클라이언트만 사용 |
| 아키텍처 정합성 | High — 마찬가지로 어댑터로 래핑 가능 |
| 유연성 | Medium — 대화 이력 관리를 직접 구현해야 함 |
| 팀 친숙도 | High — reqwest는 Rust 생태계 표준 |
| 기능 확장성 | Low — Tool calling, RAG 등 직접 구현 필요 |

**Pros:** 의존성 최소, 직접 제어 가능
**Cons:** 대화 이력/세션 관리, 스트리밍, tool calling 직접 구현 → 낮은 생산성

### Option C: Python 스크립트로 분리 (MCP 서버 확장)

| Dimension | Assessment |
|-----------|------------|
| 복잡도 | Low — Python openai 라이브러리 사용 |
| 아키텍처 정합성 | Low — Rust 프로젝트 외부에 별도 프로세스 |
| 유연성 | Medium — HTTP 통신으로 연결 |
| 통합도 | Low — Mind Studio UI와 별도 |

**Pros:** 빠른 프로토타이핑, Python LLM 생태계 활용
**Cons:** 아키텍처 일관성 저하, IPC 오버헤드, 타입 안전성 부재

## Trade-off Analysis

**Option A (rig)를 선택하는 핵심 이유:**

1. **Beat 전환 + system_prompt 갱신 루프**가 이 프로젝트의 핵심 가치다. rig의 Agent/Chat 추상화가 이 루프를 자연스럽게 지원한다. reqwest로 직접 구현하면 동일한 추상화를 처음부터 만들어야 한다.

2. **향후 확장성**: rig의 Tool calling 기능을 활용하면, NPC가 게임 월드와 상호작용하는 시나리오(물건 줍기, 장소 이동 등)를 Agent tool로 표현할 수 있다. 이는 "AI Agent를 활용한 NPC 자율 행동"이라는 장기 비전과 일치한다.

3. **ConversationPort 추상화**로 rig 의존성을 격리했으므로, 나중에 다른 프레임워크(예: llm-chain, kalosm)로 교체하더라도 어댑터만 바꾸면 된다.

**주의할 트레이드오프:**
- ~~rig의 OpenAI-compatible 모드에서 로컬 서버 연결 시 인증 우회(빈 API 키)가 정상 작동하는지 먼저 검증 필요~~ → ✅ 검증 완료: `api_key("no-key-needed")` + llama.cpp 정상 동작
- rig 버전 업데이트 시 API 변경 가능성 — `ConversationPort`가 이를 흡수하는 방파제 역할
- rig 0.33의 Responses API 기본 전환이 실제 문제를 일으킴 → `.completions_api()` 필수

## Consequences

**더 쉬워지는 것:**
- 프롬프트 품질 평가: 생성 → 대화 → 감정 변화 → 프롬프트 갱신의 전체 루프를 한 곳에서 테스트
- NPC 간 대화 시뮬레이션: 두 NPC 각각에 Agent를 생성하여 자동 대화 가능
- MCP를 통한 자동화: Claude Agent가 시나리오 로드 → 대화 실행 → 결과 평가까지 자동 수행
- 시나리오 기반 회귀 테스트: `data/huckleberry_finn/` 시나리오로 대화 품질 자동 검증

**더 어려워지는 것:**
- 빌드 의존성 증가: rig-core + tokio(이미 있음) + async-trait
- 로컬 추론 서버가 반드시 실행 중이어야 대화 테스트 가능
- 비동기 코드가 도메인 경계에 근접 (ConversationPort가 async)

**재검토가 필요한 시점:**
- rig 라이브러리가 major 버전 변경 시 어댑터 업데이트
- NPC 간 자율 대화(Agent↔Agent) 시나리오 구체화 시 아키텍처 확장
- 프롬프트 품질 평가 메트릭(자동 채점) 도입 시 `DialogueTestService` 확장

## Action Items

1. [x] rig-core + 로컬 서버 연결 PoC — `openai::CompletionsClient` + `completions_api()` 동작 확인
2. [x] `ConversationPort` 트레이트 + `ConversationError` → `ports.rs`에 추가
3. [x] `RigChatAdapter` 구현 (`src/adapter/rig_chat.rs`) + feature gate `chat`
4. [x] `DialogueTestService` 구현 (`src/application/dialogue_test_service.rs`)
5. [x] Mind Studio `/api/chat/*` 엔드포인트 추가 (`chat/start`, `chat/message`, `chat/end`)
6. [x] 채팅 UI 패널 구현 (`mind-studio-ui/` — Vite + React + Zustand)
7. [ ] MockConversationPort로 대화 루프 단위 테스트
8. [ ] `data/huckleberry_finn/` 시나리오로 E2E 대화 테스트
9. [ ] MCP 서버에 `start_chat`, `send_chat_message`, `end_chat` 도구 추가
10. [x] `LlamaServerMonitor` 포트 + `RigChatAdapter` 구현 — `/health`, `/slots`, `/metrics`
11. [x] `TimingsCapturingClient::with_client()` — 공유 `reqwest::Client` 커넥션 풀 통합
12. [x] Mind Studio `/api/llm/*` 모니터링 엔드포인트 추가 (status, health, slots, metrics)

### 구현 중 발견된 기술 이슈 및 해결

| 이슈 | 원인 | 해결 |
|------|------|------|
| Windows MSVC 링크 에러 (`__builtin_bswap`) | rig-core 기본 rustls → aws-lc-sys가 GCC intrinsic 사용 | `default-features = false, features = ["reqwest-native-tls"]` |
| LLM 404 Not Found | rig 0.33 기본 Responses API (`/v1/responses`) | `.completions_api()` → Chat Completions API 전환 |
| `CompletionsExt` 타입 미발견 | 문서화 부족, 실제 타입명 다름 | `openai::CompletionsClient` (= `Client<OpenAICompletionsExt>`) |
| `agent()` 메서드 미발견 | trait 미import | `use rig::client::CompletionClient` 추가 |
| `chat()` 타입 추론 실패 | 메서드 호출에서 제네릭 추론 불가 | UFCS `Chat::chat(&agent, ...)` + `PromptError` 명시 |
| 커넥션 풀 미공유 | `TimingsCapturingClient`와 `reqwest::get()` 별도 클라이언트 | `with_client()` 생성자 + `RigChatAdapter.http_client` 단일 풀 |
| `/metrics` 404에서 파싱 성공 | text 응답이 빈 문자열이어도 `parse()` 성공 | `error_for_status()` 추가하여 HTTP 오류 명시 처리 |
| llama-server 관리 API URL | `/slots`, `/metrics`는 `/v1` 없이 root 경로 | `server_url` 필드에서 `/v1` suffix 자동 제거 |
